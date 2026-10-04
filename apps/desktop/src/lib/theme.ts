export type Theme = "dark" | "light";

export const DEFAULT_THEME: Theme = "dark";
export const DEFAULT_ACCENT = "#0A84FF";

/** Aplica o tema no `<html>` (`data-theme`); os tokens de cor trocam sozinhos. */
export function applyTheme(theme: Theme, root: HTMLElement = document.documentElement): void {
  root.dataset.theme = theme;
}

/**
 * Aplica o accent escolhido (`#RRGGBB`). Hover e fundo suave derivam de `--accent` no CSS.
 * Valor inválido volta ao azul padrão. Devolve o accent efetivamente aplicado.
 */
export function applyAccent(color: string, root: HTMLElement = document.documentElement): string {
  const valid = /^#[0-9a-f]{6}$/i.test(color);
  const accent = valid ? color : DEFAULT_ACCENT;
  root.style.setProperty("--accent", accent);
  return accent;
}

/** A paleta de destaque do macOS (valores do tema escuro): `id` é a chave do nome no i18n. */
export const ACCENT_PALETTE = [
  { id: "blue", color: DEFAULT_ACCENT },
  { id: "purple", color: "#BF5AF2" },
  { id: "pink", color: "#FF375F" },
  { id: "red", color: "#FF453A" },
  { id: "orange", color: "#FF9F0A" },
  { id: "yellow", color: "#FFD60A" },
  { id: "green", color: "#30D158" },
  { id: "graphite", color: "#8E8E93" },
] as const;
