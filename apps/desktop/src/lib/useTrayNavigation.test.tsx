import { cleanup, render, waitFor } from "@testing-library/react";
import { createMemoryRouter, RouterProvider } from "react-router";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { commands, emit, resetBindingsMock } from "@/test/bindings-mock";
import { useTrayNavigation } from "./useTrayNavigation";

vi.mock("@/lib/bindings", async () => (await import("@/test/bindings-mock")).bindingsMock);
vi.mock("@/lib/log", () => ({ log: { warn: vi.fn(), error: vi.fn(), info: vi.fn() } }));

function Probe() {
  useTrayNavigation();
  return null;
}

function setup() {
  const router = createMemoryRouter(
    [
      { path: "/", element: <Probe /> },
      { path: "/devices", element: <p>dispositivos</p> },
      { path: "/settings", element: <p>configurações</p> },
    ],
    { initialEntries: ["/"] },
  );
  render(<RouterProvider router={router} />);
  return router;
}

beforeEach(() => {
  resetBindingsMock();
});

afterEach(cleanup);

describe("useTrayNavigation", () => {
  it("ao abrir, vai para a tela que a bandeja pediu enquanto a janela não existia", async () => {
    commands.takePendingNavigation.mockResolvedValue("/devices?pair=1");
    const router = setup();
    await waitFor(() => {
      expect(router.state.location.pathname).toBe("/devices");
    });
    expect(router.state.location.search).toBe("?pair=1");
  });

  it("sem pedido, fica onde está", async () => {
    commands.takePendingNavigation.mockResolvedValue(null);
    const router = setup();
    await waitFor(() => {
      expect(commands.takePendingNavigation).toHaveBeenCalledTimes(1);
    });
    expect(router.state.location.pathname).toBe("/");
  });

  it("com a janela aberta, o aviso da bandeja leva à tela pedida", async () => {
    commands.takePendingNavigation.mockResolvedValueOnce(null).mockResolvedValueOnce("/settings");
    const router = setup();
    await waitFor(() => {
      expect(commands.takePendingNavigation).toHaveBeenCalledTimes(1);
    });
    emit("trayNavigate");
    await waitFor(() => {
      expect(router.state.location.pathname).toBe("/settings");
    });
  });

  it("um erro ao ler o pedido não derruba a tela", async () => {
    commands.takePendingNavigation.mockRejectedValue({ code: "internal" });
    const router = setup();
    await waitFor(() => {
      expect(commands.takePendingNavigation).toHaveBeenCalled();
    });
    expect(router.state.location.pathname).toBe("/");
  });
});
