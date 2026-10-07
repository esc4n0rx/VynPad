import { afterEach, describe, expect, it, vi } from "vitest";
import type { SiteConfig } from "./config";
import { readConfig } from "./config";
import { resolveDownload } from "./download";
import { getLatestRelease, parseRelease, platformOf } from "./github";

const config = (over: Partial<SiteConfig> = {}): SiteConfig => ({
  repo: "acme/pad",
  token: undefined,
  playStoreUrl: undefined,
  siteUrl: "https://vynpad.example",
  githubUrl: undefined,
  ...over,
});

const asset = (id: number, name: string, extra: Record<string, unknown> = {}) => ({
  id,
  name,
  size: 5_000_000,
  browser_download_url: `https://github.com/acme/pad/releases/download/v1.2.0/${name}`,
  ...extra,
});

const release = (over: Record<string, unknown> = {}) => ({
  tag_name: "v1.2.0",
  name: "VynPad v1.2.0",
  html_url: "https://github.com/acme/pad/releases/tag/v1.2.0",
  published_at: "2026-10-06T12:00:00Z",
  body: "- novidade",
  draft: false,
  prerelease: false,
  assets: [
    asset(1, "VynPad_1.2.0_x64-setup.exe", { digest: `sha256:${"a".repeat(64)}` }),
    asset(2, "VynPad_1.2.0_x64-setup.exe.sig"),
    asset(3, "VynPad_1.2.0_x64_en-US.msi"),
    asset(4, "latest.json"),
  ],
  ...over,
});

const json = (body: unknown, status = 200) =>
  new Response(JSON.stringify(body), { status, headers: { "content-type": "application/json" } });

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("configuração", () => {
  it("usa os padrões quando o ambiente está vazio", () => {
    const c = readConfig({});
    expect(c.repo).toBe("esc4n0rx/VynPad");
    expect(c.token).toBeUndefined();
    expect(c.playStoreUrl).toBeUndefined();
    expect(c.siteUrl).toBe("http://localhost:3000");
  });

  it("recusa repositório malformado, Play Store de outro host e links sem https", () => {
    const c = readConfig({
      GITHUB_REPO: "../../etc/passwd",
      PLAY_STORE_URL: "https://evil.example/store?id=app.vynpad.android",
      GITHUB_URL: "http://github.com/x/y",
    });
    expect(c.repo).toBe("esc4n0rx/VynPad");
    expect(c.playStoreUrl).toBeUndefined();
    expect(c.githubUrl).toBeUndefined();
  });

  it("aceita a Play Store e normaliza o endereço do site", () => {
    const c = readConfig({
      PLAY_STORE_URL: "https://play.google.com/store/apps/details?id=app.vynpad.android",
      SITE_URL: "https://vynpad.app/",
      GITHUB_TOKEN: "  ghp_x  ",
    });
    expect(c.playStoreUrl).toContain("play.google.com");
    expect(c.siteUrl).toBe("https://vynpad.app");
    expect(c.token).toBe("ghp_x");
  });
});

describe("leitura do release", () => {
  it("separa os arquivos por plataforma e ignora assinatura e latest.json", () => {
    const parsed = parseRelease(release());
    expect(parsed?.version).toBe("1.2.0");
    expect(parsed?.artifacts.windows?.name).toBe("VynPad_1.2.0_x64-setup.exe");
    expect(parsed?.artifacts["windows-msi"]?.id).toBe(3);
    expect(Object.keys(parsed?.artifacts ?? {}).sort()).toEqual(["windows", "windows-msi"]);
  });

  it("lê o SHA-256 só quando o formato é o esperado", () => {
    const ok = parseRelease(release());
    expect(ok?.artifacts.windows?.sha256).toBe("a".repeat(64));
    const bad = parseRelease(release({ assets: [asset(1, "x-setup.exe", { digest: "md5:abc" })] }));
    expect(bad?.artifacts.windows?.sha256).toBeUndefined();
  });

  it("recusa rascunho, dado malformado e links que não são do github.com", () => {
    expect(parseRelease(release({ draft: true }))).toBeUndefined();
    expect(parseRelease({ tag_name: 3 })).toBeUndefined();
    expect(parseRelease(null)).toBeUndefined();
    expect(parseRelease(release({ html_url: "https://evil.example/x" }))).toBeUndefined();
    const sneaky = parseRelease(
      release({
        assets: [
          asset(9, "x-setup.exe", { browser_download_url: "https://evil.example/x-setup.exe" }),
        ],
      }),
    );
    expect(sneaky?.artifacts.windows).toBeUndefined();
  });

  it("reconhece o nome dos instaladores", () => {
    expect(platformOf("VynPad_1.0.0_x64-setup.exe")).toBe("windows");
    expect(platformOf("VynPad.msi")).toBe("windows-msi");
    expect(platformOf("VynPad.AppImage")).toBe("linux-appimage");
    expect(platformOf("vynpad.deb")).toBe("linux-deb");
    expect(platformOf("VynPad_1.0.0_x64-setup.exe.sig")).toBeUndefined();
    expect(platformOf("random.exe")).toBeUndefined();
  });
});

describe("busca do release mais recente", () => {
  it("manda o token só no cabeçalho de autorização da API", async () => {
    const fetchMock = vi.fn().mockResolvedValue(json(release()));
    vi.stubGlobal("fetch", fetchMock);
    const result = await getLatestRelease(config({ token: "secret" }));
    expect(result.kind).toBe("ok");
    const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect(url).toBe("https://api.github.com/repos/acme/pad/releases/latest");
    expect((init.headers as Record<string, string>).Authorization).toBe("Bearer secret");
    expect(url).not.toContain("secret");
  });

  it("sem token não manda autorização", async () => {
    const fetchMock = vi.fn().mockResolvedValue(json(release()));
    vi.stubGlobal("fetch", fetchMock);
    await getLatestRelease(config());
    const init = fetchMock.mock.calls[0]?.[1] as RequestInit;
    expect((init.headers as Record<string, string>).Authorization).toBeUndefined();
  });

  it("sem release publicado vira `empty` (e só pré-lançamento vem da lista)", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValueOnce(json({}, 404)).mockResolvedValueOnce(json([])),
    );
    expect((await getLatestRelease(config())).kind).toBe("empty");

    vi.stubGlobal(
      "fetch",
      vi
        .fn()
        .mockResolvedValueOnce(json({}, 404))
        .mockResolvedValueOnce(json([release({ prerelease: true, tag_name: "v0.1.0-beta" })])),
    );
    const beta = await getLatestRelease(config());
    expect(beta.kind === "ok" && beta.release.prerelease).toBe(true);
  });

  it("erro de rede ou 5xx vira `error`, sem lançar", async () => {
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error("rede")));
    expect((await getLatestRelease(config())).kind).toBe("error");
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(json({}, 500)));
    expect((await getLatestRelease(config())).kind).toBe("error");
  });
});

describe("resolução do download", () => {
  const stubGithub = (assetResponse?: Response) => {
    const fetchMock = vi.fn((url: string, _init?: RequestInit) =>
      Promise.resolve(
        url.includes("/releases/assets/")
          ? (assetResponse ?? new Response(null, { status: 404 }))
          : json(release()),
      ),
    );
    vi.stubGlobal("fetch", fetchMock);
    return fetchMock;
  };

  it("só aceita plataformas da lista fixa", async () => {
    const outcome = await resolveDownload("../../etc/passwd", config());
    expect(outcome).toEqual({ ok: false, status: 404, code: "unknown_platform" });
    expect((await resolveDownload("macos", config())).ok).toBe(false);
  });

  it("repositório público: manda para o link público do arquivo", async () => {
    stubGithub();
    const outcome = await resolveDownload("windows", config());
    expect(outcome).toEqual({
      ok: true,
      url: "https://github.com/acme/pad/releases/download/v1.2.0/VynPad_1.2.0_x64-setup.exe",
    });
  });

  it("repositório privado: pede o link assinado, sem seguir o redirecionamento", async () => {
    const signed =
      "https://release-assets.githubusercontent.com/github-production-release-asset/abc?sig=1";
    const fetchMock = stubGithub(
      new Response(null, { status: 302, headers: { location: signed } }),
    );
    const outcome = await resolveDownload("windows", config({ token: "secret" }));
    expect(outcome).toEqual({ ok: true, url: signed });
    const call = fetchMock.mock.calls.find(([u]) => u.includes("/releases/assets/1"));
    const init = call?.[1] as unknown as RequestInit;
    expect(init.redirect).toBe("manual");
    expect((init.headers as Record<string, string>).Accept).toBe("application/octet-stream");
  });

  it("recusa um redirecionamento para fora do GitHub", async () => {
    stubGithub(
      new Response(null, {
        status: 302,
        headers: { location: "https://evil.example/malware.exe" },
      }),
    );
    const outcome = await resolveDownload("windows", config({ token: "secret" }));
    expect(outcome).toEqual({ ok: false, status: 502, code: "upstream" });
  });

  it("recusa resposta inesperada do GitHub no arquivo", async () => {
    stubGithub(new Response("nope", { status: 200 }));
    expect((await resolveDownload("windows", config({ token: "secret" }))).ok).toBe(false);
  });

  it("plataforma sem arquivo no release não está disponível", async () => {
    stubGithub();
    expect(await resolveDownload("linux-deb", config())).toEqual({
      ok: false,
      status: 404,
      code: "not_available",
    });
  });

  it("GitHub fora do ar vira 503", async () => {
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error("rede")));
    expect(await resolveDownload("windows", config())).toEqual({
      ok: false,
      status: 503,
      code: "upstream",
    });
  });

  it("Android vai para a Google Play quando configurada e some quando não", async () => {
    const play = "https://play.google.com/store/apps/details?id=app.vynpad.android";
    expect(await resolveDownload("android", config({ playStoreUrl: play }))).toEqual({
      ok: true,
      url: play,
    });
    expect(await resolveDownload("android", config())).toEqual({
      ok: false,
      status: 404,
      code: "not_available",
    });
  });
});
