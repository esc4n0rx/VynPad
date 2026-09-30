import { isAppError } from "@/lib/errors";

/**
 * Mensagens dos erros do backend (chaves `errors.*`). Provisório: a T-504 (i18n PT-BR/EN) move
 * isto para os arquivos de idioma e passa a traduzir de verdade.
 */
const PT_BR: Record<string, string> = {
  "errors.internal": "Algo deu errado. Tente de novo; se continuar, envie o diagnóstico.",
  "errors.invalid_input": "Um valor informado não é válido.",
  "errors.unsupported": "Este recurso não está disponível neste sistema.",
  "errors.not_found": "Não encontrei o item pedido. Ele pode ter sido removido.",
  "errors.builtin_profile":
    "O perfil Padrão é do app e não pode ser alterado nem apagado. Duplique-o para personalizar.",
  "errors.profile_in_use": "Este perfil está em uso.",
  "errors.import_invalid": "O arquivo não é um perfil válido do PadGaming.",
  "errors.no_network": "Nenhuma rede local encontrada. Conecte o PC ao Wi-Fi ou à rede cabeada.",
};

/** Texto para o usuário: a mensagem traduzida e, quando houver, o detalhe do backend. */
export function messageForError(error: unknown): string {
  if (!isAppError(error)) {
    return PT_BR["errors.internal"] ?? "Algo deu errado.";
  }
  const base = PT_BR[error.message_key] ?? PT_BR["errors.internal"] ?? "Algo deu errado.";
  // Só as validações trazem um detalhe pensado para o usuário; nos outros ele é diagnóstico.
  return error.code === "invalid_input" && error.details ? `${base} ${error.details}` : base;
}
