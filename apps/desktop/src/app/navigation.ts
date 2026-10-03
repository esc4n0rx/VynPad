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
  label: string;
  icon: LucideIcon;
}

/**
 * Seções do app, na ordem da sidebar. A posição vale também para o atalho `Ctrl+<n>`
 * (Configurações é a 5ª; `Ctrl+,` também abre).
 * Os rótulos migram para os arquivos de idioma na T-504.
 */
export const NAV_ITEMS: readonly NavItem[] = [
  { path: "/", label: "Controles", icon: Gamepad2 },
  { path: "/devices", label: "Dispositivos", icon: Smartphone },
  { path: "/mapping", label: "Mapeamento", icon: SlidersHorizontal },
  { path: "/latency", label: "Latência", icon: Activity },
  { path: "/settings", label: "Configurações", icon: Settings },
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
