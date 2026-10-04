import { afterEach, describe, expect, it, vi } from "vitest";
import {
  STEPS,
  STORAGE_KEY,
  clearStep,
  isSkippable,
  loadStep,
  nextStep,
  parseStep,
  previousStep,
  saveStep,
} from "./steps";

afterEach(() => {
  window.localStorage.clear();
  vi.restoreAllMocks();
});

describe("passos da introdução", () => {
  it("andam para a frente e para trás sem passar das pontas", () => {
    expect(nextStep("welcome")).toBe("driver");
    expect(nextStep("connect")).toBe("done");
    expect(nextStep("done")).toBe("done");
    expect(previousStep("driver")).toBe("welcome");
    expect(previousStep("welcome")).toBe("welcome");
    expect(STEPS).toHaveLength(5);
  });

  it("só rede e celular podem ser pulados", () => {
    expect(STEPS.filter(isSkippable)).toEqual(["network", "connect"]);
  });

  it("um valor que não é passo vira o início", () => {
    expect(parseStep("network")).toBe("network");
    expect(parseStep("nada")).toBe("welcome");
    expect(parseStep(null)).toBe("welcome");
    expect(parseStep(3)).toBe("welcome");
  });
});

describe("onde o usuário parou", () => {
  it("grava, lê e apaga", () => {
    expect(loadStep()).toBe("welcome");
    saveStep("connect");
    expect(window.localStorage.getItem(STORAGE_KEY)).toBe("connect");
    expect(loadStep()).toBe("connect");
    clearStep();
    expect(loadStep()).toBe("welcome");
  });

  it("sem armazenamento disponível, não quebra", () => {
    vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
      throw new Error("bloqueado");
    });
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new Error("bloqueado");
    });
    vi.spyOn(Storage.prototype, "removeItem").mockImplementation(() => {
      throw new Error("bloqueado");
    });
    expect(loadStep()).toBe("welcome");
    expect(() => {
      saveStep("driver");
      clearStep();
    }).not.toThrow();
  });
});
