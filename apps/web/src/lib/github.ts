import "server-only";
import { readConfig, type SiteConfig } from "./config";

/** As plataformas que o site sabe servir. A rota de download só aceita estas chaves. */
export const PLATFORMS = [
  "windows",
  "windows-msi",
  "linux-appimage",
  "linux-deb",
  "android",
] as const;
export type Platform = (typeof PLATFORMS)[number];

export function isPlatform(value: unknown): value is Platform {
  return typeof value === "string" && (PLATFORMS as readonly string[]).includes(value);
}

export interface Artifact {
  /** Id do asset no GitHub (só o servidor o usa, para pedir o link assinado). */
  id: number;
  name: string;
  size: number;
  /** SHA-256 em hexadecimal, quando o GitHub informa (campo `digest`). */
  sha256: string | undefined;
  /** Link público do asset (só serve se o repositório for público). */
  browserUrl: string;
}

export interface ReleaseInfo {
  tag: string;
  /** A tag sem o `v` inicial. */
  version: string;
  name: string;
  publishedAt: string | undefined;
  notes: string;
  htmlUrl: string;
  prerelease: boolean;
  artifacts: Partial<Record<Platform, Artifact>>;
}

export type ReleaseResult =
  { kind: "ok"; release: ReleaseInfo } | { kind: "empty" } | { kind: "error" };

const API = "https://api.github.com";
/** Quanto tempo o servidor reaproveita a resposta do GitHub (e o limite de requisições dura mais). */
export const REVALIDATE_SECONDS = 300;

// ------------------------------------------------------------------------ leitura defensiva

type Json = Record<string, unknown>;

const isObject = (v: unknown): v is Json =>
  typeof v === "object" && v !== null && !Array.isArray(v);
const str = (v: unknown): string | undefined => (typeof v === "string" ? v : undefined);
const num = (v: unknown): number | undefined =>
  typeof v === "number" && Number.isFinite(v) ? v : undefined;

/** Aceita só links `https://github.com/...` (o que vem do GitHub, mas não confiamos às cegas). */
function githubUrl(v: unknown): string | undefined {
  const raw = str(v);
  if (!raw) return undefined;
  try {
    const url = new URL(raw);
    return url.protocol === "https:" && url.hostname === "github.com" ? url.toString() : undefined;
  } catch {
    return undefined;
  }
}

function parseArtifact(raw: unknown): Artifact | undefined {
  if (!isObject(raw)) return undefined;
  const id = num(raw.id);
  const name = str(raw.name);
  const size = num(raw.size);
  const browserUrl = githubUrl(raw.browser_download_url);
  if (id === undefined || !name || size === undefined || !browserUrl) return undefined;
  const digest = str(raw.digest);
  const sha = digest?.match(/^sha256:([0-9a-f]{64})$/i)?.[1]?.toLowerCase();
  return { id, name, size, sha256: sha, browserUrl };
}

/** Qual arquivo do release serve a cada plataforma (pelo nome; `.sig` e afins ficam de fora). */
export function platformOf(name: string): Platform | undefined {
  const lower = name.toLowerCase();
  if (lower.endsWith(".sig") || lower.endsWith(".json") || lower.endsWith(".sha256"))
    return undefined;
  if (lower.endsWith(".exe") && lower.includes("setup")) return "windows";
  if (lower.endsWith(".msi")) return "windows-msi";
  if (lower.endsWith(".appimage")) return "linux-appimage";
  if (lower.endsWith(".deb")) return "linux-deb";
  return undefined;
}

export function parseRelease(raw: unknown): ReleaseInfo | undefined {
  if (!isObject(raw) || raw.draft === true) return undefined;
  const tag = str(raw.tag_name);
  const htmlUrl = githubUrl(raw.html_url);
  if (!tag || !htmlUrl) return undefined;
  const artifacts: Partial<Record<Platform, Artifact>> = {};
  if (Array.isArray(raw.assets)) {
    for (const item of raw.assets) {
      const artifact = parseArtifact(item);
      const platform = artifact && platformOf(artifact.name);
      // O primeiro arquivo de cada plataforma vale; duplicados são ignorados.
      if (artifact && platform && !artifacts[platform]) artifacts[platform] = artifact;
    }
  }
  return {
    tag,
    version: tag.replace(/^v/i, ""),
    name: str(raw.name) || tag,
    publishedAt: str(raw.published_at),
    notes: str(raw.body) ?? "",
    htmlUrl,
    prerelease: raw.prerelease === true,
    artifacts,
  };
}

// ------------------------------------------------------------------------ requisições

export function githubHeaders(
  config: SiteConfig,
  accept = "application/vnd.github+json",
): HeadersInit {
  const headers: Record<string, string> = {
    Accept: accept,
    "X-GitHub-Api-Version": "2022-11-28",
    "User-Agent": "vynpad-web",
  };
  // O token só vai ao api.github.com, nunca ao navegador nem a outro host.
  if (config.token) headers.Authorization = `Bearer ${config.token}`;
  return headers;
}

async function getJson(
  url: string,
  config: SiteConfig,
): Promise<{ status: number; body: unknown }> {
  const response = await fetch(url, {
    headers: githubHeaders(config),
    next: { revalidate: REVALIDATE_SECONDS, tags: ["releases"] },
    signal: AbortSignal.timeout(8_000),
  });
  if (!response.ok) return { status: response.status, body: undefined };
  return { status: response.status, body: (await response.json()) as unknown };
}

/** O release mais recente publicado (sem rascunhos). Nunca lança: falha vira `error`. */
export async function getLatestRelease(config: SiteConfig = readConfig()): Promise<ReleaseResult> {
  try {
    const latest = await getJson(`${API}/repos/${config.repo}/releases/latest`, config);
    if (latest.status === 200) {
      const release = parseRelease(latest.body);
      return release ? { kind: "ok", release } : { kind: "error" };
    }
    // `latest` ignora pré-lançamentos: se só existem eles (ou nada), olha a lista.
    if (latest.status === 404) {
      const list = await getJson(`${API}/repos/${config.repo}/releases?per_page=10`, config);
      if (list.status === 200 && Array.isArray(list.body)) {
        for (const item of list.body) {
          const release = parseRelease(item);
          if (release) return { kind: "ok", release };
        }
        return { kind: "empty" };
      }
      // Repositório privado sem token (ou sem acesso) responde 404 também: não sabemos o que há.
      return list.status === 404 ? { kind: "empty" } : { kind: "error" };
    }
    return { kind: "error" };
  } catch {
    return { kind: "error" };
  }
}

/** Os últimos releases publicados, para a página de novidades. */
export async function getReleases(
  limit = 10,
  config: SiteConfig = readConfig(),
): Promise<ReleaseInfo[]> {
  try {
    const list = await getJson(
      `${API}/repos/${config.repo}/releases?per_page=${String(limit)}`,
      config,
    );
    if (list.status !== 200 || !Array.isArray(list.body)) return [];
    return list.body.map(parseRelease).filter((r): r is ReleaseInfo => r !== undefined);
  } catch {
    return [];
  }
}
