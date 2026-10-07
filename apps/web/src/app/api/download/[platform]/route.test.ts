import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { GET } from "./route";

const release = {
  tag_name: "v1.0.0",
  name: "VynPad v1.0.0",
  html_url: "https://github.com/acme/pad/releases/tag/v1.0.0",
  draft: false,
  prerelease: false,
  assets: [
    {
      id: 1,
      name: "VynPad_1.0.0_x64-setup.exe",
      size: 1000,
      browser_download_url:
        "https://github.com/acme/pad/releases/download/v1.0.0/VynPad_1.0.0_x64-setup.exe",
    },
  ],
};

const call = (platform: string, accept = "application/json") =>
  GET(new Request(`https://vynpad.example/api/download/${platform}`, { headers: { accept } }), {
    params: Promise.resolve({ platform }),
  });

beforeEach(() => {
  vi.stubEnv("GITHUB_REPO", "acme/pad");
  vi.stubEnv("GITHUB_TOKEN", "");
  vi.stubEnv("PLAY_STORE_URL", "");
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.unstubAllEnvs();
});

describe("GET /api/download/[platform]", () => {
  it("redireciona (302) para o arquivo do release e nunca guarda em cache", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(Response.json(release)));
    const response = await call("windows");
    expect(response.status).toBe(302);
    expect(response.headers.get("location")).toContain("VynPad_1.0.0_x64-setup.exe");
    expect(response.headers.get("cache-control")).toBe("no-store");
    expect(response.headers.get("x-robots-tag")).toBe("noindex");
  });

  it("plataforma desconhecida: 404 em JSON, sem consultar o GitHub", async () => {
    const fetchMock = vi.fn();
    vi.stubGlobal("fetch", fetchMock);
    const response = await call("..%2F..%2Fetc");
    expect(response.status).toBe(404);
    expect(await response.json()).toEqual({ error: "unknown_platform" });
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it("para uma pessoa (Accept: text/html) o erro volta à seção de download com o aviso", async () => {
    const response = await call("android", "text/html,application/xhtml+xml");
    expect(response.status).toBe(303);
    const location = new URL(response.headers.get("location") ?? "");
    expect(location.pathname).toBe("/");
    expect(location.searchParams.get("download")).toBe("unavailable");
    expect(location.hash).toBe("#download");
  });

  it("GitHub fora do ar: 503", async () => {
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error("rede")));
    const response = await call("windows");
    expect(response.status).toBe(503);
    expect(await response.json()).toEqual({ error: "upstream" });
  });

  it("Android vai à Google Play quando o link está configurado", async () => {
    vi.stubEnv(
      "PLAY_STORE_URL",
      "https://play.google.com/store/apps/details?id=app.vynpad.android",
    );
    const response = await call("android");
    expect(response.status).toBe(302);
    expect(response.headers.get("location")).toContain("play.google.com");
  });
});
