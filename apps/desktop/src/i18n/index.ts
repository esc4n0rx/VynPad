import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import type { AppSettings, Language } from "@/lib/bindings";
import en from "./en.json";
import ptBR from "./pt-BR.json";

export const LANGUAGES: readonly Language[] = ["pt-BR", "en"];
export const DEFAULT_LANGUAGE: Language = "pt-BR";

function isLanguage(value: unknown): value is Language {
  return LANGUAGES.includes(value as Language);
}

/**
 * Idioma inicial: o que o usuário salvou, senão o do sistema (se for inglês), senão PT-BR.
 *
 * O backend semeia `language` com "pt-BR", então "ainda não escolhido" não existe como valor:
 * enquanto o onboarding não termina, a escolha salva é tratada como o padrão e vale o idioma do
 * sistema. A T-601 grava o idioma no passo de boas-vindas.
 */
export function pickLanguage(
  settings: Pick<AppSettings, "language" | "onboarding_completed"> | undefined,
  systemLanguage: string = navigator.language,
): Language {
  if (settings?.onboarding_completed && isLanguage(settings.language)) return settings.language;
  return systemLanguage.toLowerCase().startsWith("en") ? "en" : DEFAULT_LANGUAGE;
}

// Os textos vão dentro do app: a inicialização é síncrona e nada pisca em branco.
void i18n.use(initReactI18next).init({
  resources: { "pt-BR": ptBR, en },
  lng: DEFAULT_LANGUAGE,
  fallbackLng: DEFAULT_LANGUAGE,
  defaultNS: "common",
  ns: Object.keys(ptBR),
  interpolation: { escapeValue: false },
  initAsync: false,
});

/** Troca o idioma da interface e o `lang` do documento. */
export async function setLanguage(language: Language): Promise<void> {
  if (i18n.language !== language) await i18n.changeLanguage(language);
  document.documentElement.lang = language;
}

export { i18n };
