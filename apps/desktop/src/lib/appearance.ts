import { useEffect } from "react";
import { useSettings, type Settings } from "@/lib/api/settings";
import { applyAccent, applyTheme, type Theme } from "@/lib/theme";

type Appearance = Pick<Settings, "theme" | "accent" | "reduce_motion">;

const DARK_QUERY = "(prefers-color-scheme: dark)";

/** O tema "system" segue o do sistema; os outros valem como estão. */
export function resolveTheme(theme: Settings["theme"], systemIsDark: boolean): Theme {
  if (theme === "system") return systemIsDark ? "dark" : "light";
  return theme;
}

const systemIsDark = () =>
  typeof window.matchMedia === "function" ? window.matchMedia(DARK_QUERY).matches : true;

/** Aplica tema, cor de destaque e "reduzir movimento" no `<html>`. */
export function applyAppearance(
  appearance: Appearance,
  root: HTMLElement = document.documentElement,
): void {
  applyTheme(resolveTheme(appearance.theme, systemIsDark()), root);
  applyAccent(appearance.accent, root);
  root.dataset.reduceMotion = String(appearance.reduce_motion);
}

/** Mantém a aparência em dia com as configurações (e com o tema do sistema, no modo "Sistema"). */
export function useAppearanceSync(): void {
  const { data: settings } = useSettings();

  useEffect(() => {
    if (!settings) return;
    applyAppearance(settings);
    if (settings.theme !== "system" || typeof window.matchMedia !== "function") return;
    const list = window.matchMedia(DARK_QUERY);
    const onChange = () => {
      applyAppearance(settings);
    };
    list.addEventListener("change", onChange);
    return () => {
      list.removeEventListener("change", onChange);
    };
  }, [settings]);
}
