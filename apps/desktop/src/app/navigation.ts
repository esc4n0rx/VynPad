import type { ParseKeys } from "i18next";
import {
  Activity,
  Gamepad2,
  Settings,
  SlidersHorizontal,
  Smartphone,
  type LucideIcon,
} from "lucide-react";

export interface NavItem {
  path: string;
  /** Chave do rótulo no namespace `common`. */
  labelKey: ParseKeys<"common">;
  icon: LucideIcon;
}

/**
 * Seções do app, na ordem da sidebar. A posição vale também para o atalho `Ctrl+<n>`
 * (Configurações é a 5ª; `Ctrl+,` também abre).
 */
export const NAV_ITEMS: readonly NavItem[] = [
  { path: "/", labelKey: "nav.controls", icon: Gamepad2 },
  { path: "/devices", labelKey: "nav.devices", icon: Smartphone },
  { path: "/mapping", labelKey: "nav.mapping", icon: SlidersHorizontal },
  { path: "/latency", labelKey: "nav.latency", icon: Activity },
  { path: "/settings", labelKey: "nav.settings", icon: Settings },
];

export const SETTINGS_PATH = "/settings";

/** Abaixo desta largura (px) a sidebar mostra só os ícones. */
export const COLLAPSE_BELOW_PX = 1000;

/** Rota que o atalho `Ctrl+<tecla>` abre, ou `undefined` se a tecla não é um atalho. */
export function pathForShortcut(key: string): string | undefined {
  if (key === ",") return SETTINGS_PATH;
  if (!/^[1-9]$/.test(key)) return undefined;
  return NAV_ITEMS[Number(key) - 1]?.path;
}
