import { commands } from "@/lib/bindings";
import { log } from "@/lib/log";
import { pickLanguage, setLanguage } from "./index";

/** Define o idioma antes do primeiro render, a partir das configurações salvas. */
export async function bootLanguage(): Promise<void> {
  try {
    const settings = await commands.getSettings();
    await setLanguage(pickLanguage(settings));
  } catch (error) {
    log.warn(`não consegui ler o idioma salvo: ${JSON.stringify(error)}`);
    await setLanguage(pickLanguage(undefined));
  }
}
