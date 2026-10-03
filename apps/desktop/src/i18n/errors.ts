import { isAppError } from "@/lib/errors";
import { i18n } from "./index";
import type ptBR from "./pt-BR.json";

type ErrorKey = keyof typeof ptBR.errors;

/** `errors.not_found` (a chave que o backend manda) → `not_found` no namespace `errors`. */
function errorKey(messageKey: string): string {
  return messageKey.replace(/^errors\./, "");
}

function isErrorKey(key: string): key is ErrorKey {
  return i18n.exists(key, { ns: "errors" });
}

/** Texto para o usuário: a mensagem traduzida e, quando houver, o detalhe do backend. */
export function messageForError(error: unknown): string {
  const generic = i18n.t("errors:internal");
  if (!isAppError(error)) return generic;

  const key = errorKey(error.message_key);
  // O backend pode mandar uma chave que esta versão da UI ainda não conhece.
  const base = isErrorKey(key) ? i18n.t(key, { ns: "errors" }) : generic;
  // Só as validações trazem um detalhe pensado para o usuário; nos outros ele é diagnóstico.
  return error.code === "invalid_input" && error.details ? `${base} ${error.details}` : base;
}
