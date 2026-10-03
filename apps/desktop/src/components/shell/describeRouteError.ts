import { isRouteErrorResponse } from "react-router";

/** Texto com o que deu errado, para o usuário copiar e mandar no suporte. */
export function describeRouteError(error: unknown): string {
  if (isRouteErrorResponse(error)) return `${String(error.status)} ${error.statusText}`;
  if (error instanceof Error) return error.stack ?? `${error.name}: ${error.message}`;
  try {
    return JSON.stringify(error);
  } catch {
    return String(error);
  }
}
