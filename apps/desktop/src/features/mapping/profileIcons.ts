import {
  Car,
  Crosshair,
  Footprints,
  Gamepad2,
  Ghost,
  Plane,
  Puzzle,
  Swords,
  Trophy,
  Zap,
  type LucideIcon,
} from "lucide-react";

/** Os ícones que um perfil pode ter (o nome é o que fica gravado no perfil). */
export const PROFILE_ICONS = {
  gamepad: Gamepad2,
  swords: Swords,
  crosshair: Crosshair,
  car: Car,
  plane: Plane,
  footprints: Footprints,
  puzzle: Puzzle,
  trophy: Trophy,
  ghost: Ghost,
  zap: Zap,
} as const satisfies Record<string, LucideIcon>;

export type ProfileIconName = keyof typeof PROFILE_ICONS;

export const PROFILE_ICON_NAMES = Object.keys(PROFILE_ICONS) as ProfileIconName[];

export const isKnownIcon = (name: string | null): name is ProfileIconName =>
  name !== null && name in PROFILE_ICONS;
