import { commands, type WindowChrome } from "@/lib/bindings";
import { log } from "@/lib/log";

/** Antes de saber como a janela foi pintada: fundo sólido e cantos pelo CSS. */
export const FALLBACK_CHROME: WindowChrome = { material: "solid", corners: "css" };

/**
 * Grava o material e os cantos no `<html>` (`data-material` / `data-corners`); `index.css` troca
 * o fundo do app e o arredondamento a partir deles.
 */
export function applyWindowChrome(
  chrome: WindowChrome,
  root: HTMLElement = document.documentElement,
): void {
  root.dataset.material = chrome.material;
  root.dataset.corners = chrome.corners;
}

/** Pede ao backend como a janela foi pintada, antes do primeiro render (evita piscar). */
export async function bootWindowChrome(): Promise<WindowChrome> {
  let chrome = FALLBACK_CHROME;
  try {
    chrome = await commands.windowChrome();
  } catch (error) {
    log.warn(`não consegui ler o material da janela: ${JSON.stringify(error)}`);
  }
  applyWindowChrome(chrome);
  return chrome;
}
