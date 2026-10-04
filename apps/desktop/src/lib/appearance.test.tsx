import type { ReactNode } from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { applyAppearance, resolveTheme, useAppearanceSync } from "./appearance";
import { keys } from "./api/keys";
import type { Settings } from "./api/settings";

const base = { theme: "dark", accent: "#FF453A", reduce_motion: false } as const;

describe("resolveTheme", () => {
  it("o sistema decide só no modo Sistema", () => {
    expect(resolveTheme("system", true)).toBe("dark");
    expect(resolveTheme("system", false)).toBe("light");
    expect(resolveTheme("dark", false)).toBe("dark");
    expect(resolveTheme("light", true)).toBe("light");
  });
});

describe("applyAppearance", () => {
  beforeEach(() => {
    vi.stubGlobal("matchMedia", () => ({ matches: false }));
  });
  afterEach(() => {
    vi.unstubAllGlobals();
    delete document.documentElement.dataset.theme;
    delete document.documentElement.dataset.reduceMotion;
    document.documentElement.style.removeProperty("--accent");
  });

  it("grava tema, cor e reduzir movimento no <html>", () => {
    applyAppearance({ ...base, theme: "light", reduce_motion: true });
    const root = document.documentElement;
    expect(root.dataset.theme).toBe("light");
    expect(root.dataset.reduceMotion).toBe("true");
    expect(root.style.getPropertyValue("--accent")).toBe("#FF453A");
  });

  it("o tema Sistema segue o do sistema", () => {
    applyAppearance({ ...base, theme: "system" });
    expect(document.documentElement.dataset.theme).toBe("light");
  });

  it("uma cor inválida volta ao azul padrão", () => {
    applyAppearance({ ...base, accent: "azul" });
    expect(document.documentElement.style.getPropertyValue("--accent")).toBe("#0A84FF");
  });
});

describe("useAppearanceSync", () => {
  let listeners: (() => void)[] = [];
  let dark = true;

  beforeEach(() => {
    listeners = [];
    dark = true;
    vi.stubGlobal("matchMedia", () => ({
      get matches() {
        return dark;
      },
      addEventListener: (_: string, fn: () => void) => listeners.push(fn),
      removeEventListener: (_: string, fn: () => void) => {
        listeners = listeners.filter((l) => l !== fn);
      },
    }));
  });
  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
  });

  function mount(settings: Partial<Settings>) {
    const client = new QueryClient();
    client.setQueryData(keys.settings, { ...base, ...settings });
    const wrapper = ({ children }: { children: ReactNode }) => (
      <QueryClientProvider client={client}>{children}</QueryClientProvider>
    );
    return renderHook(() => useAppearanceSync(), { wrapper });
  }

  it("aplica as configurações e acompanha o sistema quando o tema é Sistema", () => {
    const view = mount({ theme: "system" });
    expect(document.documentElement.dataset.theme).toBe("dark");
    dark = false;
    listeners.forEach((l) => {
      l();
    });
    expect(document.documentElement.dataset.theme).toBe("light");
    view.unmount();
    expect(listeners).toHaveLength(0);
  });

  it("com tema fixo, não escuta o sistema", () => {
    mount({ theme: "light" });
    expect(document.documentElement.dataset.theme).toBe("light");
    expect(listeners).toHaveLength(0);
  });
});
