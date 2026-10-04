import type { ErrorCode } from "@/lib/errors";
import { headingIds } from "./markdown";

/** Os documentos da ajuda, na ordem do menu. O nome é o do arquivo em `docs/usuario/<idioma>/`. */
export const HELP_DOCS = [
  "primeiros-passos",
  "parear-um-celular",
  "mapeamento-e-perfis",
  "gyro",
  "latencia",
  "solucao-de-problemas",
] as const;
export type HelpDoc = (typeof HELP_DOCS)[number];

export type HelpLanguage = "pt-BR" | "en";

/** O documento onde cada erro conhecido é explicado e o id do título dele. */
export const ERROR_HELP: Record<ErrorCode, string> = {
  internal: "err-internal",
  invalid_input: "err-invalid-input",
  unsupported: "err-unsupported",
  not_found: "err-not-found",
  builtin_profile: "err-builtin-profile",
  profile_in_use: "err-profile-in-use",
  import_invalid: "err-import-invalid",
  no_network: "err-no-network",
  cancelled: "err-cancelled",
};

const SOURCES = import.meta.glob("../../../../docs/usuario/*/*.md", {
  query: "?raw",
  import: "default",
  eager: true,
});

/** O texto de um documento no idioma pedido (cai no PT-BR se faltar). */
export function helpSource(doc: HelpDoc, language: HelpLanguage): string {
  const find = (lang: string) =>
    Object.entries(SOURCES).find(([path]) => path.endsWith(`/${lang}/${doc}.md`))?.[1];
  return find(language) ?? find("pt-BR") ?? "";
}

/** Em que documento está o título `id` (para os links entre páginas). */
export function docWithHeading(id: string, language: HelpLanguage): HelpDoc | null {
  return HELP_DOCS.find((doc) => headingIds(helpSource(doc, language)).includes(id)) ?? null;
}

/** O endereço (do roteador por hash) da ajuda do erro `code`. */
export function helpHashForError(code: ErrorCode): string {
  return `#/help?topic=${ERROR_HELP[code]}`;
}

/** Abre a ajuda no tópico do erro, de qualquer lugar do app. */
export function openHelpForError(code: ErrorCode): void {
  window.location.hash = helpHashForError(code);
}
