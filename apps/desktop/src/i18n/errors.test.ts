import { describe, expect, it } from "vitest";
import { isAppError } from "@/lib/errors";
import { notifyError, useToasts } from "@/lib/toast";
import { messageForError } from "./errors";

const base = { message_key: "errors.internal", details: null, in_use: null } as const;

describe("isAppError", () => {
  it("reconhece o formato que o backend devolve", () => {
    expect(isAppError({ code: "internal", message_key: "errors.internal" })).toBe(true);
  });

  it("recusa qualquer outra coisa", () => {
    for (const value of [null, undefined, "erro", 3, {}, { code: "x" }, new Error("x")]) {
      expect(isAppError(value)).toBe(false);
    }
  });
});

describe("messageForError", () => {
  it("traduz cada código conhecido", () => {
    for (const code of [
      "internal",
      "invalid_input",
      "unsupported",
      "not_found",
      "builtin_profile",
      "profile_in_use",
      "import_invalid",
      "no_network",
    ] as const) {
      const text = messageForError({ ...base, code, message_key: `errors.${code}` });
      expect(text.length).toBeGreaterThan(10);
      expect(text).not.toContain("errors.");
    }
  });

  it("só as validações mostram o detalhe do backend", () => {
    const invalid = {
      ...base,
      code: "invalid_input",
      message_key: "errors.invalid_input",
    } as const;
    expect(
      messageForError({ ...invalid, details: "as portas precisam ser 1024 ou mais" }),
    ).toContain("as portas precisam ser 1024 ou mais");
    const internal = { ...base, code: "internal", details: "SqliteError: disk I/O" } as const;
    expect(messageForError(internal)).not.toContain("Sqlite");
  });

  it("uma chave que o app ainda não conhece cai na mensagem genérica", () => {
    expect(messageForError({ ...base, code: "internal", message_key: "errors.nova" })).toContain(
      "Algo deu errado",
    );
  });

  it("algo que não é AppError (ex.: falha do próprio Tauri) também vira mensagem", () => {
    expect(messageForError("command não encontrado")).toContain("Algo deu errado");
    expect(messageForError(undefined)).toContain("Algo deu errado");
  });
});

describe("toasts", () => {
  it("notifyError empilha o aviso e mantém só os mais recentes", () => {
    useToasts.setState({ toasts: [] });
    for (let i = 0; i < 6; i++) notifyError({ ...base, code: "internal" });
    expect(useToasts.getState().toasts).toHaveLength(4);
    const [first] = useToasts.getState().toasts;
    useToasts.getState().dismiss(first?.id ?? -1);
    expect(useToasts.getState().toasts).toHaveLength(3);
  });
});
