import type { Locale } from "./config";
import { en } from "./en";
import { ptBR, type Dictionary } from "./pt-BR";

export type { Dictionary };

const dictionaries: Record<Locale, Dictionary> = { "pt-BR": ptBR, en };

export function getDictionary(locale: Locale): Dictionary {
  return dictionaries[locale];
}

/** O código de idioma de datas e números (`Intl`). */
export function intlLocale(locale: Locale): string {
  return locale === "pt-BR" ? "pt-BR" : "en-US";
}
