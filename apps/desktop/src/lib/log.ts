import { commands, type FrontendLogLevel } from "@/lib/bindings";

/** O backend recusa mensagens acima de 8 KB; cortamos antes com folga para o sufixo. */
const MAX_LENGTH = 8000;

function send(level: FrontendLogLevel, message: string) {
  const text = message.length > MAX_LENGTH ? `${message.slice(0, MAX_LENGTH)}… [cortado]` : message;
  // Falha ao logar não deve quebrar a UI; o console ainda mostra em dev.
  commands.logFromFrontend(level, text).catch((err: unknown) => {
    console.warn("falha ao enviar log ao backend", err);
  });
}

/** Logs do React gravados no mesmo arquivo do backend (target `frontend`). */
export const log = {
  error: (message: string) => send("error", message),
  warn: (message: string) => send("warn", message),
  info: (message: string) => send("info", message),
  debug: (message: string) => send("debug", message),
};

/** Envia ao log os erros não tratados da webview. */
export function installGlobalErrorLogging() {
  window.addEventListener("error", (event) => {
    log.error(`${event.message} (${event.filename}:${event.lineno}:${event.colno})`);
  });
  window.addEventListener("unhandledrejection", (event) => {
    const reason: unknown = event.reason;
    log.error(
      `promise rejeitada sem tratamento: ${reason instanceof Error ? (reason.stack ?? reason.message) : JSON.stringify(reason)}`,
    );
  });
}
