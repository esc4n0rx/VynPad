/** Erro padronizado devolvido pelos commands (espelha `AppError` em `src-tauri/src/error.rs`). */
export type ErrorCode = "internal" | "invalid_input";

export interface AppError {
  code: ErrorCode;
  /** Chave do i18n com a mensagem para o usuário. */
  message_key: string;
  /** Só para diagnóstico; não exibir diretamente ao usuário. */
  details?: string;
}

export function isAppError(value: unknown): value is AppError {
  return (
    typeof value === "object" &&
    value !== null &&
    typeof (value as AppError).code === "string" &&
    typeof (value as AppError).message_key === "string"
  );
}
