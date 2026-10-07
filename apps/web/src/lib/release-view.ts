import "server-only";
import type { SiteConfig } from "./config";
import { PLATFORMS, type Platform, type ReleaseResult } from "./github";

/**
 * O que a interface precisa saber dos downloads, já "achatado" e serializável: o servidor lê o
 * GitHub, e só isto desce para o navegador (nada de id de asset, token ou link assinado).
 */
export interface DownloadFile {
  platform: Platform;
  name: string;
  size: number;
  sha256: string | undefined;
}

export interface DownloadView {
  status: "ok" | "empty" | "error";
  version: string | undefined;
  tag: string | undefined;
  publishedAt: string | undefined;
  prerelease: boolean;
  files: DownloadFile[];
  playAvailable: boolean;
}

export function toDownloadView(result: ReleaseResult, config: SiteConfig): DownloadView {
  const playAvailable = Boolean(config.playStoreUrl);
  if (result.kind !== "ok") {
    return {
      status: result.kind,
      version: undefined,
      tag: undefined,
      publishedAt: undefined,
      prerelease: false,
      files: [],
      playAvailable,
    };
  }
  const { release } = result;
  const files: DownloadFile[] = [];
  for (const platform of PLATFORMS) {
    const artifact = release.artifacts[platform];
    if (artifact)
      files.push({ platform, name: artifact.name, size: artifact.size, sha256: artifact.sha256 });
  }
  return {
    status: "ok",
    version: release.version,
    tag: release.tag,
    publishedAt: release.publishedAt,
    prerelease: release.prerelease,
    files,
    playAvailable,
  };
}
