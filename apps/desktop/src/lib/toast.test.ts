import { afterEach, beforeEach, describe, expect, it } from "vitest";
import type { AppError } from "@/lib/bindings";
import { notifyError, useToasts } from "./toast";

const error = (code: AppError["code"]): AppError => ({
  code,
  message_key: `errors.${code}`,
  details: null,
  in_use: null,
});

beforeEach(() => {
  useToasts.setState({ toasts: [] });
  window.location.hash = "";
});

afterEach(() => {
  useToasts.setState({ toasts: [] });
  window.location.hash = "";
});

describe("notifyError", () => {
  it("todo erro conhecido vem com a ação que abre a ajuda dele", () => {
    notifyError(error("no_network"));
    const [toast] = useToasts.getState().toasts;
    expect(toast?.kind).toBe("error");
    expect(toast?.message).toContain("Nenhuma rede local");
    expect(toast?.action?.label).toBe("Ver ajuda");
    toast?.action?.onClick();
    expect(window.location.hash).toBe("#/help?topic=err-no-network");
  });

  it("um erro que não é do app não tem ação (não há o que explicar)", () => {
    notifyError(new Error("falhou"));
    const [toast] = useToasts.getState().toasts;
    expect(toast?.message).toContain("Algo deu errado");
    expect(toast?.action).toBeUndefined();
  });
});
