import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { createMemoryRouter, RouterProvider } from "react-router";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useLastPaired } from "@/lib/api/server";
import type { Settings } from "@/lib/api/settings";
import type { DriverStatusDto, FirewallStatusDto, NearbyDto } from "@/lib/bindings";
import { useToasts } from "@/lib/toast";
import { commands, resetBindingsMock } from "@/test/bindings-mock";
import { OnboardingPage } from "./OnboardingPage";
import { STORAGE_KEY } from "./steps";

vi.mock("@/lib/bindings", async () => (await import("@/test/bindings-mock")).bindingsMock);
vi.mock("@/lib/log", () => ({ log: { warn: vi.fn(), error: vi.fn(), info: vi.fn() } }));

const settings: Settings = {
  language: "pt-BR",
  theme: "dark",
  accent: "#0A84FF",
  ports: { control: 47800, input: 47801, discovery: 47802 },
  autostart: false,
  start_minimized: false,
  onboarding_completed: false,
  require_approval: false,
  server_name: "",
  close_to_tray: true,
  default_profile_id: null,
  send_rate_hz: 250,
  reservation_secs: 30,
  transparency: true,
  reduce_motion: false,
  tray_hint_shown: false,
  notify_connections: true,
  notify_pairing: true,
  notify_driver: true,
  notify_skip_fullscreen: true,
};

const firewall = (patch: Partial<FirewallStatusDto> = {}): FirewallStatusDto => ({
  applicable: true,
  firewall_on: true,
  ok: true,
  public_network: false,
  ports: [],
  ...patch,
});

interface Options {
  driver?: DriverStatusDto;
  firewall?: FirewallStatusDto;
  nearby?: NearbyDto[];
}

function setup(options: Options = {}) {
  commands.getSettings.mockResolvedValue(settings);
  commands.updateSettings.mockImplementation((next: Settings) => Promise.resolve(next));
  commands.driverStatus.mockResolvedValue(options.driver ?? { state: "ready" });
  commands.firewallStatus.mockResolvedValue(options.firewall ?? firewall());
  commands.getServerInfo.mockResolvedValue({
    name: "PC da sala",
    fingerprint_short: "AB12",
    control_port: 47800,
    input_port: 47801,
    discovery_port: 47802,
    ports_pending_restart: null,
    addresses: [{ interface: "Wi-Fi", ip: "192.168.0.10" }],
    free_slots: 4,
  });
  commands.getNearbyDevices.mockResolvedValue(options.nearby ?? []);
  commands.listDevices.mockResolvedValue([]);
  commands.startPairing.mockResolvedValue({
    uri: "vynpad://pair?x=1",
    qr_svg: '<svg viewBox="0 0 2 2"><rect width="2" height="2"/></svg>',
    pin: "482913",
    token_ttl_s: 120,
    pin_ttl_s: 120,
    fingerprint_short: "AB12CD34",
    hosts: ["192.168.0.10"],
    port: 47800,
  });
  commands.cancelPairing.mockResolvedValue(undefined);
  commands.subscribeLiveInput.mockResolvedValue(null);
  commands.unsubscribeLiveInput.mockResolvedValue(undefined);
  const router = createMemoryRouter(
    [
      { path: "/onboarding", element: <OnboardingPage /> },
      { path: "/", element: <p>painel</p> },
    ],
    { initialEntries: ["/onboarding"] },
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

const next = () => fireEvent.click(screen.getByRole("button", { name: "Continuar" }));
const stepIs = (name: string) => {
  expect(screen.getByText(new RegExp(`Etapa [0-9] de 5: ${name}`))).toBeTruthy();
};

beforeEach(() => {
  resetBindingsMock();
  window.localStorage.clear();
  useToasts.setState({ toasts: [] });
  useLastPaired.setState({ seq: 0, last: null });
});

afterEach(() => {
  cleanup();
});

describe("introdução", () => {
  it("começa nas boas-vindas, com o idioma para escolher", async () => {
    setup();
    expect(await screen.findByText("Seu celular é o controle")).toBeTruthy();
    stepIs("Boas-vindas");
    fireEvent.click(await screen.findByRole("radio", { name: "English" }));
    await waitFor(() => {
      expect((commands.updateSettings.mock.calls.at(-1) as [Settings])[0].language).toBe("en");
    });
  });

  it("o passo do driver, com o driver pronto, deixa continuar", async () => {
    setup();
    await screen.findByText("Seu celular é o controle");
    next();
    expect(await screen.findByText("Driver instalado e funcionando.")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Continuar" })).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Continuar sem driver" })).toBeNull();
  });

  it("sem o driver, oferece o download e só deixa seguir 'sem driver'", async () => {
    commands.openVigemDownload.mockResolvedValue(null);
    setup({ driver: { state: "not_installed", download_url: "https://example.test" } });
    await screen.findByText("Seu celular é o controle");
    next();
    expect(await screen.findByText("O driver ainda não está instalado.")).toBeTruthy();
    expect(screen.getByText(/Sem o driver, o VynPad não cria controles/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Baixar o ViGEmBus" }));
    await waitFor(() => {
      expect(commands.openVigemDownload).toHaveBeenCalledTimes(1);
    });
    expect(screen.queryByRole("button", { name: "Continuar" })).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Continuar sem driver" }));
    expect(await screen.findByText("Endereço")).toBeTruthy();
  });

  it("um driver antigo mostra as versões", async () => {
    setup({
      driver: {
        state: "outdated",
        found: "1.10",
        required: "1.16",
        download_url: "https://example.test",
      },
    });
    await screen.findByText("Seu celular é o controle");
    next();
    expect(
      await screen.findByText(
        "O driver instalado é antigo (1.10). Precisa da versão 1.16 ou mais nova.",
      ),
    ).toBeTruthy();
  });

  it("o passo da rede mostra o nome e o endereço e corrige o firewall", async () => {
    commands.fixFirewall.mockResolvedValue(firewall());
    setup({ firewall: firewall({ ok: false }) });
    await screen.findByText("Seu celular é o controle");
    next();
    await screen.findByText("Driver instalado e funcionando.");
    next();
    expect(await screen.findByText("PC da sala")).toBeTruthy();
    expect(screen.getByText("192.168.0.10:47800")).toBeTruthy();
    expect(await screen.findByText("O firewall do Windows está bloqueando o VynPad.")).toBeTruthy();
    // Enquanto não está certo, dá para pular.
    expect(screen.getByRole("button", { name: "Pular por enquanto" })).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Permitir conexões" }));
    await waitFor(() => {
      expect(commands.fixFirewall).toHaveBeenCalledTimes(1);
    });
    await waitFor(() => {
      expect(useToasts.getState().toasts.at(-1)?.message).toBe("Pronto, conexões permitidas.");
    });
  });

  it("recusar o UAC explica e deixa tentar de novo", async () => {
    commands.fixFirewall.mockRejectedValue({
      code: "cancelled",
      message_key: "errors.cancelled",
      details: null,
      in_use: null,
    });
    setup({ firewall: firewall({ ok: false }) });
    await screen.findByText("Seu celular é o controle");
    next();
    await screen.findByText("Driver instalado e funcionando.");
    next();
    fireEvent.click(await screen.findByRole("button", { name: "Permitir conexões" }));
    await waitFor(() => {
      expect(useToasts.getState().toasts.at(-1)?.message).toBe(
        "A permissão de administrador foi recusada. Dá para tentar de novo.",
      );
    });
    expect(screen.getByRole("button", { name: "Permitir conexões" })).toBeTruthy();
  });

  it("com o firewall liberado, o passo da rede não oferece pular", async () => {
    setup();
    await screen.findByText("Seu celular é o controle");
    next();
    await screen.findByText("Driver instalado e funcionando.");
    next();
    expect(await screen.findByText("O firewall deixa o celular conectar.")).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Pular por enquanto" })).toBeNull();
  });

  it("o passo do celular mostra o QR e, ao parear, acende o controle e libera o Continuar", async () => {
    setup();
    await screen.findByText("Seu celular é o controle");
    next();
    await screen.findByText("Driver instalado e funcionando.");
    next();
    await screen.findByText("O firewall deixa o celular conectar.");
    next();
    expect(await screen.findByRole("img", { name: "QR code para parear" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Pular por enquanto" })).toBeTruthy();
    expect(screen.getByText(/Nenhum celular com o VynPad aberto por perto/)).toBeTruthy();

    act(() => {
      useLastPaired.getState().record({ device_id: "d1", name: "Galaxy da Ana", model: "S24" });
    });
    expect(await screen.findByText("Galaxy da Ana pareado!")).toBeTruthy();
    expect(screen.getByText(/O controle P1 já está respondendo/)).toBeTruthy();
    // O controle de P1 passa a seguir o input ao vivo.
    await waitFor(() => {
      expect(commands.subscribeLiveInput).toHaveBeenCalledWith(0);
    });
    expect(screen.queryByRole("button", { name: "Pular por enquanto" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Concluir" })).toBeNull();
  });

  it("os celulares por perto aparecem na lista", async () => {
    setup({
      nearby: [
        {
          id: "n1",
          name: "Pixel do Bruno",
          model: "Pixel 8",
          ip: "192.168.0.50",
          port: 47800,
          seen_ago_ms: 0,
          paired: false,
        },
      ],
    });
    await screen.findByText("Seu celular é o controle");
    next();
    await screen.findByText("Driver instalado e funcionando.");
    next();
    await screen.findByText("O firewall deixa o celular conectar.");
    next();
    const nearby = await screen.findByRole("region", { name: "Celulares por perto" });
    expect(await within(nearby).findByText("Pixel do Bruno")).toBeTruthy();
    expect(within(nearby).getByText("Novo")).toBeTruthy();
  });

  it("pular os passos opcionais chega ao resumo, que conclui a introdução", async () => {
    const router = setup({
      driver: { state: "service_stopped" },
      firewall: firewall({ ok: false }),
    });
    await screen.findByText("Seu celular é o controle");
    next();
    await screen.findByText(/O driver está instalado, mas parado/);
    fireEvent.click(screen.getByRole("button", { name: "Continuar sem driver" }));
    fireEvent.click(await screen.findByRole("button", { name: "Pular por enquanto" }));
    fireEvent.click(await screen.findByRole("button", { name: "Pular por enquanto" }));

    expect(await screen.findByText("Tudo pronto")).toBeTruthy();
    const summary = screen.getByRole("list", { name: "Resumo" });
    expect(within(summary).getAllByText("pendente")).toHaveLength(2);

    fireEvent.click(screen.getByRole("switch", { name: "Iniciar com o Windows" }));
    await waitFor(() => {
      expect((commands.updateSettings.mock.calls.at(-1) as [Settings])[0].autostart).toBe(true);
    });

    fireEvent.click(screen.getByRole("button", { name: "Começar" }));
    await waitFor(() => {
      expect(router.state.location.pathname).toBe("/");
    });
    expect((commands.updateSettings.mock.calls.at(-1) as [Settings])[0].onboarding_completed).toBe(
      true,
    );
    expect(window.localStorage.getItem(STORAGE_KEY)).toBeNull();
  });

  it("Voltar volta um passo", async () => {
    setup();
    await screen.findByText("Seu celular é o controle");
    expect(screen.queryByRole("button", { name: "Voltar" })).toBeNull();
    next();
    await screen.findByText("Driver instalado e funcionando.");
    fireEvent.click(screen.getByRole("button", { name: "Voltar" }));
    expect(await screen.findByText("Seu celular é o controle")).toBeTruthy();
  });

  it("retoma do passo em que o app foi fechado", async () => {
    window.localStorage.setItem(STORAGE_KEY, "network");
    setup();
    expect(await screen.findByText("Endereço")).toBeTruthy();
    stepIs("Rede");
  });

  it("um valor estranho guardado recomeça do início", async () => {
    window.localStorage.setItem(STORAGE_KEY, "voltar-para-o-futuro");
    setup();
    expect(await screen.findByText("Seu celular é o controle")).toBeTruthy();
  });

  it("o passo atual fica marcado no indicador de etapas", async () => {
    setup();
    await screen.findByText("Seu celular é o controle");
    const stepper = screen.getByRole("navigation", { name: "Etapas da introdução" });
    const items = within(stepper).getAllByRole("listitem");
    expect(items).toHaveLength(5);
    expect(items[0]?.getAttribute("aria-current")).toBe("step");
    next();
    await screen.findByText("Driver instalado e funcionando.");
    expect(items[1]?.getAttribute("aria-current")).toBe("step");
  });
});
