export type Os = "windows" | "android" | "ios" | "macos" | "linux" | "other";

/**
 * Descobre o sistema de quem visita, só para destacar o download certo. Usa a plataforma que o
 * navegador informa (quando existe) e cai no `userAgent`. Não é segurança nem métrica: errar
 * apenas mostra o cartão errado em destaque, e todos continuam à mostra.
 */
export function detectOs(userAgent: string, platformHint?: string): Os {
  const ua = userAgent.toLowerCase();
  const hint = platformHint?.toLowerCase() ?? "";
  // Android antes de Linux: o userAgent do Android também diz "Linux".
  if (hint === "android" || ua.includes("android")) return "android";
  if (/iphone|ipad|ipod/.test(ua) || hint === "ios") return "ios";
  if (hint === "windows" || ua.includes("windows")) return "windows";
  if (hint === "macos" || ua.includes("mac os x") || ua.includes("macintosh")) return "macos";
  if (hint === "linux" || ua.includes("linux") || ua.includes("x11")) return "linux";
  return "other";
}

export const OS_NAMES: Record<Os, string> = {
  windows: "Windows",
  android: "Android",
  ios: "iOS",
  macos: "macOS",
  linux: "Linux",
  other: "",
};
