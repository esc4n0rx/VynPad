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
