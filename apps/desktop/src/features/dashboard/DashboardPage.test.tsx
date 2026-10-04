import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { createMemoryRouter, RouterProvider } from "react-router";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useLiveInput } from "@/lib/api/live";
import { useMetrics } from "@/lib/api/metrics";
import { useSlots } from "@/lib/api/slots";
import type { DeviceDto, ProfileSummaryDto, SlotDto, SlotsDto } from "@/lib/bindings";
import { commands, resetBindingsMock } from "@/test/bindings-mock";
import { DashboardPage } from "./DashboardPage";

vi.mock("@/lib/bindings", async () => (await import("@/test/bindings-mock")).bindingsMock);
vi.mock("@/lib/log", () => ({ log: { warn: vi.fn(), error: vi.fn(), info: vi.fn() } }));

// O Radix usa APIs de ponteiro que o jsdom não tem.
beforeEach(() => {
  Element.prototype.hasPointerCapture = () => false;
  Element.prototype.releasePointerCapture = () => undefined;
  Element.prototype.scrollIntoView = () => undefined;
});

const slot = (index: number, patch: Partial<SlotDto> = {}): SlotDto => ({
  slot: index,
  state: "free",
  expires_in_ms: null,
  device_id: null,
  session: null,
  paused: false,
  user_index: null,
  ...patch,
});

const dto = (slots: SlotDto[]): SlotsDto => ({
  slots,
  free_count: slots.filter((s) => s.state === "free").length,
});

const phone: DeviceDto = {
  id: "dev-1",
  name: "Galaxy da Ana",
  model: "SM-S918B",
  os_version: null,
  paired_at: 0,
  last_seen_at: null,
  last_ip: null,
  revoked: false,
  preferred_slot: null,
  default_profile_id: null,
  color: null,
  current_slot: 0,
};

const summary = (id: string, name: string): ProfileSummaryDto => ({
  id,
  name,
  description: null,
  icon: null,
  is_builtin: id === "padrao",
  schema_version: 1,
  created_at: 0,
  updated_at: 0,
});

interface Options {
  profiles?: (string | null)[];
  xinput?: { index: number; occupant: "free" | "virtual" | "physical" }[];
}

function setup(slots: SlotDto[], options: Options = {}) {
  commands.getServerInfo.mockResolvedValue({
    name: "PC",
    fingerprint_short: "AB12",
    control_port: 47800,
    input_port: 47801,
    discovery_port: 47802,
    ports_pending_restart: null,
    addresses: [{ interface: "Wi-Fi", ip: "192.168.0.10" }],
    free_slots: 4,
  });
  commands.listDevices.mockResolvedValue([phone]);
  commands.listProfiles.mockResolvedValue([summary("padrao", "Padrão"), summary("fps", "FPS")]);
  commands.getProfile.mockImplementation((id: string) =>
    Promise.resolve({
      summary: summary(id, id),
      profile: { gyro: { enabled: id === "fps" } },
    }),
  );
  commands.getSlotProfiles.mockResolvedValue(options.profiles ?? ["padrao", null, null, null]);
  commands.xinputSlots.mockResolvedValue(options.xinput ?? []);
  commands.subscribeLiveInput.mockResolvedValue(null);
  commands.unsubscribeLiveInput.mockResolvedValue(undefined);
  commands.swapSlots.mockResolvedValue(null);
  commands.setSlotPaused.mockResolvedValue(null);
  commands.disconnectSlot.mockResolvedValue(null);
  commands.setSlotProfile.mockResolvedValue("fps");
  useSlots.setState({ slots: dto(slots), health: {} });

  const router = createMemoryRouter(
    [
      { path: "/", element: <DashboardPage /> },
      { path: "/devices", element: <p>tela de dispositivos</p> },
      { path: "/latency", element: <p>tela de latência</p> },
    ],
    { initialEntries: ["/"] },
  );
  render(
    <QueryClientProvider
      client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}
    >
      <RouterProvider router={router} />
    </QueryClientProvider>,
  );
  return router;
}

const connected = (index: number, patch: Partial<SlotDto> = {}) =>
  slot(index, { state: "active", device_id: "dev-1", session: 7 + index, ...patch });

beforeEach(() => {
  resetBindingsMock();
});

afterEach(() => {
  cleanup();
  useSlots.setState({ slots: null, health: {} });
  useLiveInput.setState({ reports: {} });
  useMetrics.setState({ bySlot: {} });
});

const card = (index: number) => screen.getByTestId(`slot-${String(index)}`);

describe("dashboard", () => {
  it("mostra os 4 slots vazios com o convite para parear", async () => {
    const router = setup([0, 1, 2, 3].map((i) => slot(i)));
    expect(screen.getAllByText("Aguardando conexão")).toHaveLength(4);
    expect(await screen.findByText("192.168.0.10:47800")).toBeTruthy();

    fireEvent.click(within(card(1)).getByRole("button", { name: "Parear dispositivo" }));
    await waitFor(() => {
      expect(router.state.location.pathname).toBe("/devices");
    });
  });

  it("o botão do cabeçalho abre a tela de dispositivos", async () => {
    const router = setup([0, 1, 2, 3].map((i) => slot(i)));
    fireEvent.click(screen.getByRole("button", { name: "Parear novo" }));
    await waitFor(() => {
      expect(router.state.location.pathname).toBe("/devices");
    });
  });

  it("um card conectado mostra o aparelho, o perfil e o selo de gyro", async () => {
    setup([connected(0), slot(1), slot(2), slot(3)]);
    expect(await screen.findByText("Galaxy da Ana")).toBeTruthy();
    expect(screen.getByText("SM-S918B")).toBeTruthy();
    expect(card(0).dataset.kind).toBe("connected");
    expect(card(1).dataset.kind).toBe("empty");
    // O perfil Padrão (sem gyro): sem selo.
    await waitFor(() => {
      expect(commands.getProfile).toHaveBeenCalledWith("padrao");
    });
    expect(within(card(0)).queryByText("Gyro")).toBeNull();
    // Cada controle conectado assina o input ao vivo.
    expect(commands.subscribeLiveInput).toHaveBeenCalledWith(0);
  });

  it("o selo de gyro aparece quando o perfil do slot usa gyro", async () => {
    setup([connected(0), slot(1), slot(2), slot(3)], { profiles: ["fps", null, null, null] });
    await within(card(0)).findByText("Gyro");
  });

  it("mostra bateria, latência e a qualidade da conexão do slot", async () => {
    setup([connected(0), slot(1), slot(2), slot(3)]);
    act(() => {
      useMetrics.getState().set({
        slots: [
          {
            slot: 0,
            session_id: 7,
            rtt_ms: 21.4,
            rtt_percentiles_ms: { p50: 20, p95: 25, p99: 30 },
            jitter_ms: 2,
            loss_pct: 0,
            out_of_order: 0,
            rate_hz: 250,
            processing_percentiles_ms: { p50: 0.02, p95: 0.05, p99: 0.1 },
            battery: 87,
            quality: "good",
          },
        ],
      });
    });
    const metrics = within(card(0));
    expect(await metrics.findByText("21 ms")).toBeTruthy();
    expect(metrics.getByText("87%")).toBeTruthy();
    expect(metrics.getByText("Boa")).toBeTruthy();
    // O slot sem sessão não mostra nada disso.
    expect(within(card(1)).queryByText("21 ms")).toBeNull();
  });

  it("sem medida ainda, a latência aparece como traço e a qualidade fica de fora", () => {
    setup([connected(0), slot(1), slot(2), slot(3)]);
    act(() => {
      useMetrics.getState().set({
        slots: [
          {
            slot: 0,
            session_id: 7,
            rtt_ms: null,
            rtt_percentiles_ms: { p50: 0, p95: 0, p99: 0 },
            jitter_ms: 0,
            loss_pct: 0,
            out_of_order: 0,
            rate_hz: 0,
            processing_percentiles_ms: { p50: 0, p95: 0, p99: 0 },
            battery: null,
            quality: null,
          },
        ],
      });
    });
    expect(within(card(0)).getByText("— ms")).toBeTruthy();
    expect(within(card(0)).queryByText("Excelente")).toBeNull();
  });

  it("sem sinal vira reconectando, com o contador da reserva", () => {
    setup([connected(0, { state: "suspended", expires_in_ms: 20_000 }), slot(1), slot(2), slot(3)]);
    expect(card(0).dataset.kind).toBe("reconnecting");
    expect(within(card(0)).getByText(/Reconectando…/)).toBeTruthy();
    expect(within(card(0)).getByText(/Reserva termina em 20 s/)).toBeTruthy();
  });

  it("um controle físico no índice do slot mostra o card informativo", async () => {
    setup(
      [0, 1, 2, 3].map((i) => slot(i)),
      { xinput: [{ index: 2, occupant: "physical" }] },
    );
    await within(card(2)).findByText("Controle físico");
    expect(card(2).dataset.kind).toBe("physical");
  });

  it("trocar o perfil no card aplica na hora", async () => {
    setup([connected(0), slot(1), slot(2), slot(3)]);
    const trigger = await within(card(0)).findByRole("combobox", { name: "Perfil de P1" });
    fireEvent.keyDown(trigger, { key: "Enter" });
    fireEvent.click(await screen.findByRole("option", { name: "FPS" }));
    await waitFor(() => {
      expect(commands.setSlotProfile).toHaveBeenCalledWith(0, "fps");
    });
  });

  it("arrastar o P1 para o P3 troca os slots", async () => {
    setup([connected(0), slot(1), slot(2), slot(3)]);
    const data = new Map<string, string>();
    const dataTransfer = {
      setData: (type: string, value: string) => data.set(type, value),
      getData: (type: string) => data.get(type) ?? "",
      get types() {
        return [...data.keys()];
      },
      effectAllowed: "",
      dropEffect: "",
    };
    expect(card(0).getAttribute("draggable")).toBe("true");
    expect(card(2).getAttribute("draggable")).toBe("false");

    fireEvent.dragStart(card(0), { dataTransfer });
    fireEvent.dragOver(card(2), { dataTransfer });
    fireEvent.drop(card(2), { dataTransfer });

    await waitFor(() => {
      expect(commands.swapSlots).toHaveBeenCalledTimes(1);
    });
    const [a, b] = commands.swapSlots.mock.calls[0] as [number, number];
    expect(new Set([a, b])).toEqual(new Set([0, 2]));
  });

  it("soltar o card em cima dele mesmo não troca nada", () => {
    setup([connected(0), slot(1), slot(2), slot(3)]);
    const data = new Map<string, string>([["application/x-padgaming-slot", "0"]]);
    const dataTransfer = {
      getData: (type: string) => data.get(type) ?? "",
      get types() {
        return [...data.keys()];
      },
      dropEffect: "",
    };
    fireEvent.drop(card(0), { dataTransfer });
    expect(commands.swapSlots).not.toHaveBeenCalled();
  });

  function openMenu(index: number) {
    const button = within(card(index)).getByRole("button", {
      name: `Ações de P${String(index + 1)}`,
    });
    act(() => {
      fireEvent.keyDown(button, { key: "Enter" });
    });
  }

  it("o menu pausa, desconecta e abre a latência", async () => {
    const router = setup([connected(0), slot(1), slot(2), slot(3)]);

    openMenu(0);
    fireEvent.click(await screen.findByRole("menuitem", { name: "Pausar" }));
    await waitFor(() => {
      expect(commands.setSlotPaused).toHaveBeenCalledWith(0, true);
    });

    openMenu(0);
    fireEvent.click(await screen.findByRole("menuitem", { name: "Desconectar" }));
    await waitFor(() => {
      expect(commands.disconnectSlot).toHaveBeenCalledWith(0);
    });

    openMenu(0);
    fireEvent.click(await screen.findByRole("menuitem", { name: "Ver latência" }));
    await waitFor(() => {
      expect(router.state.location.pathname).toBe("/latency");
    });
  });

  it("o menu testa a vibração do celular do slot", async () => {
    commands.testRumble.mockResolvedValue(null);
    setup([connected(0), slot(1), slot(2), slot(3)]);
    openMenu(0);
    fireEvent.click(await screen.findByRole("menuitem", { name: "Testar vibração" }));
    await waitFor(() => {
      expect(commands.testRumble).toHaveBeenCalledWith(0);
    });
  });

  it("um slot pausado oferece retomar e mostra o selo", async () => {
    setup([connected(0, { paused: true }), slot(1), slot(2), slot(3)]);
    expect(within(card(0)).getByText("Pausado")).toBeTruthy();
    openMenu(0);
    fireEvent.click(await screen.findByRole("menuitem", { name: "Retomar" }));
    await waitFor(() => {
      expect(commands.setSlotPaused).toHaveBeenCalledWith(0, false);
    });
  });

  it("o submenu troca de slot sem arrastar", async () => {
    setup([connected(0), slot(1), slot(2), slot(3)]);
    openMenu(0);
    const swap = await screen.findByRole("menuitem", { name: /Trocar de slot/ });
    act(() => {
      fireEvent.keyDown(swap, { key: "ArrowRight" });
    });
    fireEvent.click(await screen.findByRole("menuitem", { name: "Trocar com P3" }));
    await waitFor(() => {
      expect(commands.swapSlots).toHaveBeenCalledWith(0, 2);
    });
  });
});
