import { QueryClientProvider, QueryClient } from "@tanstack/react-query";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { createMemoryRouter, RouterProvider } from "react-router";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { TooltipProvider } from "@/components/ui/tooltip";
import { commands } from "@/test/bindings-mock";
import { NAV_ITEMS, pathForShortcut } from "./navigation";
import { routes } from "./router";

vi.mock("@/lib/bindings", async () => (await import("@/test/bindings-mock")).bindingsMock);
vi.mock("@/lib/log", () => ({ log: { warn: vi.fn(), error: vi.fn(), info: vi.fn() } }));
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    close: vi.fn(),
    minimize: vi.fn(),
    toggleMaximize: vi.fn(),
    isMaximized: () => Promise.resolve(false),
    onResized: () => Promise.resolve(() => undefined),
    onFocusChanged: () => Promise.resolve(() => undefined),
  }),
}));

function matchMedia(matches: boolean) {
  vi.stubGlobal("matchMedia", (query: string) => ({
    matches,
    media: query,
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
  }));
}

function renderAt(path: string) {
  const router = createMemoryRouter(routes, { initialEntries: [path] });
  render(
    <QueryClientProvider
      client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}
    >
      <TooltipProvider>
        <RouterProvider router={router} />
      </TooltipProvider>
    </QueryClientProvider>,
  );
  return router;
}

afterEach(cleanup);

beforeEach(() => {
  matchMedia(false);
  commands.getServerInfo.mockResolvedValue({
    name: "GAMING",
    fingerprint_short: "ab12",
    control_port: 47800,
    input_port: 47801,
    discovery_port: 47802,
    ports_pending_restart: null,
    addresses: [{ interface: "Wi-Fi", ip: "192.168.0.10" }],
    free_slots: 4,
  });
  commands.driverStatus.mockResolvedValue({ state: "ready" });
  commands.setUiVisible.mockResolvedValue(undefined);
});

describe("atalhos", () => {
  it("Ctrl+1..5 seguem a ordem da sidebar e Ctrl+, abre as configurações", () => {
    NAV_ITEMS.forEach((item, index) => {
      expect(pathForShortcut(String(index + 1))).toBe(item.path);
    });
    expect(pathForShortcut(",")).toBe("/settings");
    expect(pathForShortcut("6")).toBeUndefined();
    expect(pathForShortcut("a")).toBeUndefined();
  });
});

describe("shell", () => {
  it("mostra a sidebar com as seções e o status do server", async () => {
    renderAt("/devices");
    const nav = await screen.findByRole("navigation", { name: "Seções" });
    for (const item of NAV_ITEMS) {
      expect(nav.querySelector(`a[href="${item.path}"]`)).toBeTruthy();
    }
    expect(await screen.findByText("Online")).toBeTruthy();
    expect(screen.getByText("Driver OK")).toBeTruthy();
    expect(screen.getByText("192.168.0.10")).toBeTruthy();
    expect(nav.querySelector('a[href="/devices"]')?.getAttribute("aria-current")).toBe("page");
  });

  it("navega pelo teclado com Ctrl+<número> e Ctrl+,", async () => {
    const router = renderAt("/");
    await screen.findByRole("navigation", { name: "Seções" });

    fireEvent.keyDown(window, { key: "3", ctrlKey: true });
    await waitFor(() => {
      expect(router.state.location.pathname).toBe("/mapping");
    });
    expect(await screen.findByRole("complementary", { name: "Lista de perfis" })).toBeTruthy();

    fireEvent.keyDown(window, { key: ",", ctrlKey: true });
    await waitFor(() => {
      expect(router.state.location.pathname).toBe("/settings");
    });
    expect(
      await screen.findByRole("navigation", { name: "Seções das configurações" }),
    ).toBeTruthy();
  });

  it("recolhe a sidebar para só ícones em janelas estreitas", async () => {
    matchMedia(true);
    renderAt("/");
    const nav = await screen.findByRole("navigation", { name: "Seções" });
    expect(nav.className).toContain("w-14");
    expect(screen.getByRole("link", { name: "Dispositivos" })).toBeTruthy();
    expect(screen.queryByText("Online")).toBeNull();
  });

  it("o onboarding é tela cheia, sem sidebar", async () => {
    renderAt("/onboarding");
    await screen.findByText("Esta tela chega na T-601.");
    expect(screen.queryByRole("navigation", { name: "Seções" })).toBeNull();
  });
});
