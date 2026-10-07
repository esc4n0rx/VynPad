import "server-only";

/**
 * Configuração do site, lida do ambiente a cada chamada (nunca vai para o navegador: este
 * módulo é `server-only`). Valores inválidos são ignorados, não repassados.
 *
 * | Variável         | Para quê                                                                |
 * | ---------------- | ----------------------------------------------------------------------- |
 * | `GITHUB_REPO`    | `dono/repositório` dos releases (padrão `esc4n0rx/VynPad`)               |
 * | `GITHUB_TOKEN`   | token **somente leitura** (fine-grained, `Contents: read`); obrigatório se o repositório for privado |
 * | `PLAY_STORE_URL` | link da Google Play; quando existe, o Android passa a ser "disponível"  |
 * | `SITE_URL`       | endereço público do site (canonical, sitemap, Open Graph)               |
 * | `GITHUB_URL`     | link público do repositório, mostrado no rodapé só se for informado     |
 */
export interface SiteConfig {
  repo: string;
  token: string | undefined;
  playStoreUrl: string | undefined;
  siteUrl: string;
  githubUrl: string | undefined;
}

const DEFAULT_REPO = "esc4n0rx/VynPad";
const REPO_PATTERN = /^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/;

function clean(value: string | undefined): string | undefined {
  const trimmed = value?.trim();
  return trimmed ? trimmed : undefined;
}

/** `https://` e o host esperado; qualquer outra coisa vira `undefined`. */
function httpsUrl(value: string | undefined, hosts?: readonly string[]): string | undefined {
  const raw = clean(value);
  if (!raw) return undefined;
  try {
    const url = new URL(raw);
    if (url.protocol !== "https:") return undefined;
    if (hosts && !hosts.includes(url.hostname)) return undefined;
    return url.toString();
  } catch {
    return undefined;
  }
}

export function readConfig(env: Record<string, string | undefined> = process.env): SiteConfig {
  const repo = clean(env.GITHUB_REPO);
  const site = clean(env.SITE_URL);
  return {
    repo: repo && REPO_PATTERN.test(repo) ? repo : DEFAULT_REPO,
    token: clean(env.GITHUB_TOKEN),
    playStoreUrl: httpsUrl(env.PLAY_STORE_URL, ["play.google.com"]),
    siteUrl: (
      httpsUrl(site) ??
      (site?.startsWith("http://localhost") ? site : undefined) ??
      "http://localhost:3000"
    ).replace(/\/$/, ""),
    githubUrl: httpsUrl(env.GITHUB_URL, ["github.com"]),
  };
}
