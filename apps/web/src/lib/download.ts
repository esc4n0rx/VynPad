import "server-only";
import { readConfig, type SiteConfig } from "./config";
import { getLatestRelease, githubHeaders, isPlatform } from "./github";

export type DownloadFailure = "unknown_platform" | "not_available" | "upstream";

export type DownloadOutcome =
  { ok: true; url: string } | { ok: false; status: 404 | 502 | 503; code: DownloadFailure };

const API = "https://api.github.com";

/** Para onde o GitHub pode mandar o download. Qualquer outro destino é recusado. */
export function isAllowedDownloadHost(url: URL): boolean {
  return (
    url.protocol === "https:" &&
    (url.hostname === "github.com" || url.hostname.endsWith(".githubusercontent.com"))
  );
}

/**
 * Descobre para onde mandar quem clicou em "Baixar".
 *
 * - O nome do arquivo e o id do asset vêm **do release que o servidor leu**, nunca do pedido:
 *   o visitante só escolhe uma plataforma de uma lista fixa.
 * - Repositório público (sem token): o link público do asset.
 * - Repositório privado (com token): o servidor pede ao GitHub o link assinado e temporário do
 *   arquivo e manda o visitante para ele. O token não sai daqui e o arquivo não passa pelo site.
 * - Android: a Google Play, quando o link está configurado.
 */
export async function resolveDownload(
  platform: string,
  config: SiteConfig = readConfig(),
): Promise<DownloadOutcome> {
  if (!isPlatform(platform)) return { ok: false, status: 404, code: "unknown_platform" };

  if (platform === "android") {
    return config.playStoreUrl
      ? { ok: true, url: config.playStoreUrl }
      : { ok: false, status: 404, code: "not_available" };
  }

  const result = await getLatestRelease(config);
  if (result.kind === "error") return { ok: false, status: 503, code: "upstream" };
  if (result.kind === "empty") return { ok: false, status: 404, code: "not_available" };
  const artifact = result.release.artifacts[platform];
  if (!artifact) return { ok: false, status: 404, code: "not_available" };

  if (!config.token) return { ok: true, url: artifact.browserUrl };

  try {
    const response = await fetch(
      `${API}/repos/${config.repo}/releases/assets/${String(artifact.id)}`,
      {
        headers: githubHeaders(config, "application/octet-stream"),
        // Não seguimos o redirecionamento: queremos o endereço assinado, não o arquivo.
        redirect: "manual",
        cache: "no-store",
        signal: AbortSignal.timeout(8_000),
      },
    );
    const location = response.headers.get("location");
    if (response.status !== 302 || !location) return { ok: false, status: 502, code: "upstream" };
    const target = new URL(location);
    if (!isAllowedDownloadHost(target)) return { ok: false, status: 502, code: "upstream" };
    return { ok: true, url: target.toString() };
  } catch {
    return { ok: false, status: 502, code: "upstream" };
  }
}
