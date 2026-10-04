import { applyAppearance } from "@/lib/appearance";
import { commands } from "@/lib/bindings";
import { log } from "@/lib/log";
import { DEFAULT_ACCENT } from "@/lib/theme";
import { pickLanguage, setLanguage } from "./index";

/** Define o idioma antes do primeiro render, a partir das configurações salvas. */
export async function bootLanguage(): Promise<void> {
  try {
    const settings = await commands.getSettings();
    // Tema e cor antes do primeiro render, para nada piscar.
    applyAppearance({
      theme: settings.theme ?? "dark",
      accent: settings.accent ?? DEFAULT_ACCENT,
      reduce_motion: settings.reduce_motion ?? false,
    });
    await setLanguage(pickLanguage(settings));
  } catch (error) {
    log.warn(`não consegui ler o idioma salvo: ${JSON.stringify(error)}`);
    await setLanguage(pickLanguage(undefined));
  }
}
