import type { AppError } from "@/lib/bindings";

export type { AppError, ErrorCode } from "@/lib/bindings";

/** Erro padronizado devolvido pelos commands (espelha `AppError` em `src-tauri/src/error.rs`). */
export function isAppError(value: unknown): value is AppError {
  return (
    typeof value === "object" &&
    value !== null &&
    typeof (value as AppError).code === "string" &&
    typeof (value as AppError).message_key === "string"
  );
}
