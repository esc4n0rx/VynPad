import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { createMemoryRouter, RouterProvider } from "react-router";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Settings } from "@/lib/api/settings";
import type { AppError, FirewallStatusDto } from "@/lib/bindings";
import { useToasts } from "@/lib/toast";
import { commands, resetBindingsMock } from "@/test/bindings-mock";
import { SettingsPage } from "./SettingsPage";

vi.mock("@/lib/bindings", async () => (await import("@/test/bindings-mock")).bindingsMock);
vi.mock("@/lib/log", () => ({ log: { warn: vi.fn(), error: vi.fn(), info: vi.fn() } }));

const defaults: Settings = {
  language: "pt-BR",
  theme: "dark",
  accent: "#0A84FF",
  ports: { control: 47800, input: 47801, discovery: 47802 },
  autostart: false,
  start_minimized: false,
  onboarding_completed: true,
  require_approval: false,
  server_name: "",
  close_to_tray: true,
  default_profile_id: null,
  send_rate_hz: 250,
  reservation_secs: 30,
  transparency: true,
  reduce_motion: false,
};

const firewall = (patch: Partial<FirewallStatusDto> = {}): FirewallStatusDto => ({
  applicable: true,
  firewall_on: true,
  ok: true,
  public_network: false,
  ports: [
    { purpose: "Canal de controle", port: 47800, protocol: "tcp", allowed: true },
    { purpose: "Input", port: 47801, protocol: "udp", allowed: true },
  ],
  ...patch,
});

function setup(settings: Partial<Settings> = {}, initial = "/settings") {
  const current = { ...defaults, ...settings };
  commands.getSettings.mockResolvedValue(current);
  commands.updateSettings.mockImplementation((next: Settings) => Promise.resolve(next));
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
  commands.listNetworkInterfaces.mockResolvedValue([
    { name: "Wi-Fi", ip: "192.168.0.10", auto: true, virtual_like: false },
    { name: "vEthernet (WSL)", ip: "172.20.0.1", auto: false, virtual_like: true },
  ]);
  commands.firewallStatus.mockResolvedValue(firewall());
  commands.driverStatus.mockResolvedValue({ state: "ready" });
  commands.listProfiles.mockResolvedValue([
    {
      id: "padrao",
      name: "Padrão",
      description: null,
      icon: null,
      is_builtin: true,
      schema_version: 1,
      created_at: 0,
      updated_at: 0,
    },
    {
      id: "fps",
      name: "FPS",
      description: null,
      icon: null,
      is_builtin: false,
      schema_version: 1,
      created_at: 0,
      updated_at: 0,
    },
  ]);
  commands.appInfo.mockResolvedValue({ app_version: "0.1.0", core_version: "0.1.0" });
  commands.logsDir.mockResolvedValue("C:\\Users\\ana\\AppData\\Local\\PadGaming\\logs");
  const router = createMemoryRouter(
    [
      { path: "/settings", element: <SettingsPage /> },
      { path: "/onboarding", element: <p>tela de introdução</p> },
    ],
    { initialEntries: [initial] },
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

const lastSaved = () => {
  const calls = commands.updateSettings.mock.calls as [Settings][];
  return calls[calls.length - 1]?.[0];
};

const goTo = (section: string) => {
  fireEvent.click(
    within(screen.getByRole("navigation", { name: "Seções das configurações" })).getByRole(
      "button",
      { name: section },
    ),
  );
};

beforeEach(() => {
  resetBindingsMock();
  useToasts.setState({ toasts: [] });
  Element.prototype.hasPointerCapture = () => false;
  Element.prototype.releasePointerCapture = () => undefined;
  Element.prototype.scrollIntoView = () => undefined;
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

describe("navegação e busca", () => {
  it("abre na seção Geral e troca de seção pela lateral", async () => {
    setup();
    expect(await screen.findByRole("switch", { name: "Iniciar com o Windows" })).toBeTruthy();
    expect(screen.queryByText("Tema")).toBeNull();
    goTo("Aparência");
    expect(await screen.findByRole("radio", { name: "Claro" })).toBeTruthy();
    expect(screen.queryByRole("switch", { name: "Iniciar com o Windows" })).toBeNull();
    expect(
      within(screen.getByRole("navigation", { name: "Seções das configurações" }))
        .getByRole("button", { name: "Aparência" })
        .getAttribute("aria-current"),
    ).toBe("page");
  });

  it("a busca atravessa as seções, ignora acentos e esconde o resto", async () => {
    setup();
    await screen.findByRole("switch", { name: "Iniciar com o Windows" });
    fireEvent.change(screen.getByRole("searchbox", { name: "Buscar nas configurações" }), {
      target: { value: "conexao" },
    });
    // "Testar conexão" (Rede) e outras que citam conexão.
    expect(await screen.findByText("Testar conexão")).toBeTruthy();
    expect(screen.queryByRole("switch", { name: "Iniciar com o Windows" })).toBeNull();
    // As seções sem resultado somem.
    expect(screen.queryByRole("heading", { name: "Aparência" })).toBeNull();
  });

  it("uma busca sem resultado avisa", async () => {
    setup();
    await screen.findByRole("switch", { name: "Iniciar com o Windows" });
    fireEvent.change(screen.getByRole("searchbox", { name: "Buscar nas configurações" }), {
      target: { value: "zzzz" },
    });
    expect(await screen.findByText("Nada encontrado para “zzzz”.")).toBeTruthy();
  });
});

describe("geral", () => {
  it("cada interruptor grava a configuração e mantém as outras", async () => {
    setup({ server_name: "Sala" });
    fireEvent.click(await screen.findByRole("switch", { name: "Iniciar com o Windows" }));
    await waitFor(() => {
      expect(lastSaved()).toEqual({ ...defaults, server_name: "Sala", autostart: true });
    });
    fireEvent.click(
      screen.getByRole("switch", { name: "Pedir aprovação para novos dispositivos" }),
    );
    await waitFor(() => {
      expect(lastSaved()?.require_approval).toBe(true);
    });
    expect(
      screen
        .getByRole("switch", { name: "Fechar vai para a bandeja" })
        .getAttribute("aria-checked"),
    ).toBe("true");
  });

  it("o nome do server grava ao sair do campo, sem espaços nas pontas", async () => {
    setup();
    const input = await screen.findByRole("textbox", { name: "Nome do server" });
    fireEvent.change(input, { target: { value: "  PC da sala  " } });
    expect(commands.updateSettings).not.toHaveBeenCalled();
    fireEvent.blur(input);
    await waitFor(() => {
      expect(lastSaved()?.server_name).toBe("PC da sala");
    });
  });

  it("refazer a introdução zera o onboarding e abre a tela", async () => {
    const router = setup();
    fireEvent.click(await screen.findByRole("button", { name: "Refazer" }));
    await waitFor(() => {
      expect(router.state.location.pathname).toBe("/onboarding");
    });
    expect(lastSaved()?.onboarding_completed).toBe(false);
  });
});

describe("rede", () => {
  it("valida as portas antes de gravar e só grava quando estão certas", async () => {
    setup();
    goTo("Rede");
    const control = await screen.findByLabelText("Controle (TCP)");
    const apply = screen.getByRole("button", { name: "Aplicar portas" });
    expect((apply as HTMLButtonElement).disabled).toBe(true);

    fireEvent.change(control, { target: { value: "80" } });
    expect(await screen.findByText("Use portas de 1024 a 65535.")).toBeTruthy();
    expect((apply as HTMLButtonElement).disabled).toBe(true);

    fireEvent.change(control, { target: { value: "47801" } });
    expect(await screen.findByText("As três portas precisam ser diferentes.")).toBeTruthy();

    fireEvent.change(control, { target: { value: "50000" } });
    await waitFor(() => {
      expect((apply as HTMLButtonElement).disabled).toBe(false);
    });
    fireEvent.click(apply);
    await waitFor(() => {
      expect(lastSaved()?.ports).toEqual({ control: 50000, input: 47801, discovery: 47802 });
    });
  });

  it("avisa que as portas novas esperam o reinício e oferece reiniciar", async () => {
    setup();
    commands.getServerInfo.mockResolvedValue({
      name: "PC",
      fingerprint_short: "AB12",
      control_port: 47800,
      input_port: 47801,
      discovery_port: 47802,
      ports_pending_restart: { control: 50000, input: 50001, discovery: 50002 },
      addresses: [],
      free_slots: 4,
    });
    commands.restartApp.mockResolvedValue(null);
    goTo("Rede");
    expect(
      await screen.findByText("As portas novas valem depois de reiniciar o app."),
    ).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Reiniciar agora" }));
    await waitFor(() => {
      expect(commands.restartApp).toHaveBeenCalledTimes(1);
    });
  });

  it("lista as interfaces com os selos e copia o IP", async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    vi.stubGlobal("navigator", { clipboard: { writeText } });
    setup();
    goTo("Rede");
    expect(await screen.findByText("vEthernet (WSL)")).toBeTruthy();
    expect(screen.getByText("Em uso")).toBeTruthy();
    expect(screen.getByText("Virtual")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Copiar 192.168.0.10" }));
    await waitFor(() => {
      expect(writeText).toHaveBeenCalledWith("192.168.0.10");
    });
    vi.unstubAllGlobals();
  });

  it("testar conexão mostra o resultado de cada sonda", async () => {
    commands.testConnection.mockResolvedValue([
      { what: "Canal de controle (TCP 47800)", ok: true },
      { what: "Descoberta (UDP 47802)", ok: false },
    ]);
    setup();
    goTo("Rede");
    fireEvent.click(await screen.findByRole("button", { name: "Testar" }));
    expect(await screen.findByText(/Canal de controle \(TCP 47800\)/)).toBeTruthy();
    expect(screen.getByText("Respondeu")).toBeTruthy();
    expect(screen.getByText("Sem resposta")).toBeTruthy();
  });

  it("firewall liberado só informa o estado das portas", async () => {
    setup();
    goTo("Rede");
    expect(await screen.findByText("O firewall não bloqueia o PadGaming.")).toBeTruthy();
    expect(screen.getByText(/TCP 47800 · liberada/)).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Corrigir" })).toBeNull();
  });

  it("firewall bloqueando oferece corrigir e mostra as regras criadas", async () => {
    setup();
    commands.firewallStatus.mockResolvedValue(
      firewall({
        ok: false,
        ports: [{ purpose: "Canal de controle", port: 47800, protocol: "tcp", allowed: false }],
      }),
    );
    commands.fixFirewall.mockResolvedValue(firewall());
    goTo("Rede");
    expect(await screen.findByText("Algumas portas estão bloqueadas.")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Corrigir" }));
    await waitFor(() => {
      expect(commands.fixFirewall).toHaveBeenCalledTimes(1);
    });
    expect(await screen.findByText("O firewall não bloqueia o PadGaming.")).toBeTruthy();
    expect(useToasts.getState().toasts.at(-1)?.message).toBe("Regras criadas.");
  });

  it("recusar o UAC avisa sem tratar como falha do app", async () => {
    const cancelled: AppError = {
      code: "cancelled",
      message_key: "errors.cancelled",
      details: null,
      in_use: null,
    };
    setup();
    commands.firewallStatus.mockResolvedValue(firewall({ ok: false }));
    commands.fixFirewall.mockRejectedValue(cancelled);
    goTo("Rede");
    fireEvent.click(await screen.findByRole("button", { name: "Corrigir" }));
    await waitFor(() => {
      expect(useToasts.getState().toasts.at(-1)?.message).toBe(
        "A permissão de administrador foi recusada.",
      );
    });
  });

  it("rede pública avisa e abre as configurações de rede", async () => {
    commands.openNetworkSettings.mockResolvedValue(null);
    setup();
    commands.firewallStatus.mockResolvedValue(firewall({ public_network: true }));
    goTo("Rede");
    fireEvent.click(await screen.findByRole("button", { name: "Abrir configurações de rede" }));
    await waitFor(() => {
      expect(commands.openNetworkSettings).toHaveBeenCalledTimes(1);
    });
  });
});

describe("controles", () => {
  it("mostra o driver e leva ao download", async () => {
    commands.openVigemDownload.mockResolvedValue(null);
    setup();
    goTo("Controles");
    expect(await screen.findByText("Driver OK")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Reinstalar" }));
    await waitFor(() => {
      expect(commands.openVigemDownload).toHaveBeenCalledTimes(1);
    });
  });

  it("escolhe o perfil padrão, a taxa de envio", async () => {
    setup();
    goTo("Controles");
    const select = await screen.findByRole("combobox", {
      name: "Perfil padrão para novos dispositivos",
    });
    fireEvent.keyDown(select, { key: "Enter" });
    fireEvent.click(await screen.findByRole("option", { name: "FPS" }));
    await waitFor(() => {
      expect(lastSaved()?.default_profile_id).toBe("fps");
    });

    fireEvent.click(screen.getByRole("radio", { name: "125 Hz" }));
    await waitFor(() => {
      expect(lastSaved()?.send_rate_hz).toBe(125);
    });
  });

  it("o tempo de reserva só grava quando o slider para", async () => {
    setup();
    goTo("Controles");
    const slider = await screen.findByRole("slider", { name: "Tempo de reserva do slot" });
    for (let i = 0; i < 4; i++) fireEvent.keyDown(slider, { key: "ArrowRight" });
    expect(commands.updateSettings).not.toHaveBeenCalled();
    await waitFor(
      () => {
        expect(commands.updateSettings).toHaveBeenCalledTimes(1);
      },
      { timeout: 1500 },
    );
    expect(lastSaved()?.reservation_secs).toBe(50);
  });
});

describe("aparência e idioma", () => {
  it("troca o tema, a cor e os interruptores", async () => {
    setup();
    goTo("Aparência");
    fireEvent.click(await screen.findByRole("radio", { name: "Claro" }));
    await waitFor(() => {
      expect(lastSaved()?.theme).toBe("light");
    });
    fireEvent.click(screen.getByRole("radio", { name: "Verde" }));
    await waitFor(() => {
      expect(lastSaved()?.accent).toBe("#30D158");
    });
    fireEvent.click(screen.getByRole("switch", { name: "Transparência" }));
    await waitFor(() => {
      expect(lastSaved()?.transparency).toBe(false);
    });
    fireEvent.click(screen.getByRole("switch", { name: "Reduzir movimento" }));
    await waitFor(() => {
      expect(lastSaved()?.reduce_motion).toBe(true);
    });
  });

  it("marca a cor atual e aceita uma cor personalizada", async () => {
    setup({ accent: "#BF5AF2" });
    goTo("Aparência");
    expect((await screen.findByRole("radio", { name: "Roxo" })).getAttribute("aria-checked")).toBe(
      "true",
    );
    expect(screen.getByRole("radio", { name: "Azul" }).getAttribute("aria-checked")).toBe("false");
    fireEvent.change(screen.getByLabelText("Cor personalizada"), { target: { value: "#123456" } });
    await waitFor(() => {
      expect(lastSaved()?.accent).toBe("#123456");
    });
  });

  it("troca o idioma", async () => {
    setup();
    goTo("Idioma");
    fireEvent.click(await screen.findByRole("radio", { name: "English" }));
    await waitFor(() => {
      expect(lastSaved()?.language).toBe("en");
    });
  });
});

describe("privacidade", () => {
  it("limpar o histórico confirma e diz quantos registros saíram", async () => {
    commands.clearHistory.mockResolvedValue(12);
    setup();
    goTo("Privacidade e segurança");
    fireEvent.click(await screen.findByRole("button", { name: "Limpar histórico" }));
    expect(commands.clearHistory).not.toHaveBeenCalled();
    const dialog = await screen.findByRole("dialog");
    fireEvent.click(within(dialog).getByRole("button", { name: "Limpar" }));
    await waitFor(() => {
      expect(commands.clearHistory).toHaveBeenCalledTimes(1);
    });
    await waitFor(() => {
      expect(useToasts.getState().toasts.at(-1)?.message).toBe("Histórico limpo (12 registros).");
    });
  });

  it("regenerar a identidade confirma, executa e oferece reiniciar", async () => {
    commands.regenerateIdentity.mockResolvedValue(null);
    commands.restartApp.mockResolvedValue(null);
    setup();
    goTo("Privacidade e segurança");
    fireEvent.click(await screen.findByRole("button", { name: "Regenerar identidade" }));
    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText(/precisam parear de novo/)).toBeTruthy();
    expect(commands.regenerateIdentity).not.toHaveBeenCalled();
    fireEvent.click(within(dialog).getByRole("button", { name: "Regenerar" }));
    await waitFor(() => {
      expect(commands.regenerateIdentity).toHaveBeenCalledTimes(1);
    });
    const restart = await screen.findByText("Reiniciar o app?");
    expect(restart).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Reiniciar agora" }));
    await waitFor(() => {
      expect(commands.restartApp).toHaveBeenCalledTimes(1);
    });
  });

  it("dá para cancelar sem apagar nada", async () => {
    setup();
    goTo("Privacidade e segurança");
    fireEvent.click(await screen.findByRole("button", { name: "Regenerar identidade" }));
    const dialog = await screen.findByRole("dialog");
    fireEvent.click(within(dialog).getByRole("button", { name: "Cancelar" }));
    expect(commands.regenerateIdentity).not.toHaveBeenCalled();
  });
});

describe("sobre", () => {
  it("mostra a versão, abre a pasta de logs e lista as licenças", async () => {
    commands.openLogsDir.mockResolvedValue(null);
    setup();
    goTo("Sobre");
    expect(await screen.findByText("App 0.1.0 · Núcleo 0.1.0")).toBeTruthy();
    expect(await screen.findByText(/Pasta: C:\\Users\\ana/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Abrir pasta de logs" }));
    await waitFor(() => {
      expect(commands.openLogsDir).toHaveBeenCalledTimes(1);
    });

    // Atualizações e diagnóstico ainda não existem.
    expect(screen.getAllByRole("button", { name: "Em breve" })).toHaveLength(2);
    for (const button of screen.getAllByRole("button", { name: "Em breve" })) {
      expect((button as HTMLButtonElement).disabled).toBe(true);
    }

    fireEvent.click(screen.getByRole("button", { name: "Mostrar licenças" }));
    const licenses = screen.getByRole<HTMLTextAreaElement>("textbox", {
      name: "Licenças de terceiros",
    });
    expect(licenses.value).toContain("ViGEmBus");
    expect(licenses.value).toContain("uPlot");
    act(() => {
      fireEvent.click(screen.getByRole("button", { name: "Esconder licenças" }));
    });
    expect(screen.queryByRole("textbox", { name: "Licenças de terceiros" })).toBeNull();
  });
});
