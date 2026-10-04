import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { MemoryRouter } from "react-router";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useApprovals, useLastPaired } from "@/lib/api/server";
import type {
  DeviceDto,
  LogEntryDto,
  NearbyDto,
  PairingOfferDto,
  ProfileSummaryDto,
} from "@/lib/bindings";
import { commands, resetBindingsMock } from "@/test/bindings-mock";
import { ApprovalDialog } from "./ApprovalDialog";
import { DevicesPage } from "./DevicesPage";

vi.mock("@/lib/bindings", async () => (await import("@/test/bindings-mock")).bindingsMock);
vi.mock("@/lib/log", () => ({ log: { warn: vi.fn(), error: vi.fn(), info: vi.fn() } }));

beforeEach(() => {
  resetBindingsMock();
  // O Radix usa APIs de ponteiro que o jsdom não tem.
  Element.prototype.hasPointerCapture = () => false;
  Element.prototype.releasePointerCapture = () => undefined;
  Element.prototype.scrollIntoView = () => undefined;
});

afterEach(() => {
  cleanup();
  useApprovals.setState({ pending: [] });
  useLastPaired.setState({ seq: 0, last: null });
});

const device = (patch: Partial<DeviceDto> = {}): DeviceDto => ({
  id: "dev-1",
  name: "Galaxy da Ana",
  model: "SM-S918B",
  os_version: "Android 15",
  paired_at: 0,
  last_seen_at: null,
  last_ip: null,
  revoked: false,
  preferred_slot: null,
  default_profile_id: null,
  color: null,
  current_slot: null,
  ...patch,
});

const nearby = (patch: Partial<NearbyDto> = {}): NearbyDto => ({
  id: "dev-9",
  name: "Pixel do Bruno",
  model: "Pixel 8",
  ip: "192.168.0.50",
  port: 47800,
  seen_ago_ms: 500,
  paired: false,
  ...patch,
});

const offer = (patch: Partial<PairingOfferDto> = {}): PairingOfferDto => ({
  uri: "padgaming://pair?x=1",
  qr_svg: '<svg viewBox="0 0 2 2"><rect width="2" height="2"/></svg>',
  pin: "482913",
  token_ttl_s: 120,
  pin_ttl_s: 120,
  fingerprint_short: "AB12CD34",
  hosts: ["192.168.0.10"],
  port: 47800,
  ...patch,
});

const profile = (id: string, name: string): ProfileSummaryDto => ({
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
  devices?: DeviceDto[];
  nearby?: NearbyDto[];
  offer?: PairingOfferDto;
  log?: LogEntryDto[];
}

function setup(options: Options = {}, route = "/") {
  commands.listDevices.mockResolvedValue(options.devices ?? []);
  commands.getNearbyDevices.mockResolvedValue(options.nearby ?? []);
  commands.listProfiles.mockResolvedValue([profile("padrao", "Padrão"), profile("fps", "FPS")]);
  commands.startPairing.mockResolvedValue(options.offer ?? offer());
  commands.cancelPairing.mockResolvedValue(undefined);
  commands.getConnectionLog.mockResolvedValue(options.log ?? []);
  render(
    <QueryClientProvider
      client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}
    >
      <MemoryRouter initialEntries={[route]}>
        <DevicesPage />
        <ApprovalDialog />
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

function openMenu(name: string) {
  const button = screen.getByRole("button", { name: `Ações de ${name}` });
  act(() => {
    fireEvent.keyDown(button, { key: "Enter" });
  });
}

describe("próximos na rede", () => {
  it("mostra o novo com o botão de parear e destaca o aparelho no modal", async () => {
    setup({ nearby: [nearby()] });
    expect(await screen.findByText("Pixel do Bruno")).toBeTruthy();
    expect(screen.getByText("Novo")).toBeTruthy();
    expect(screen.getByText("visto agora")).toBeTruthy();
    expect(screen.getByText("192.168.0.50")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Parear" }));
    expect(await screen.findByText("Pareando com Pixel do Bruno")).toBeTruthy();
  });

  it("um aparelho já pareado não oferece parear e mostra se está conectado", async () => {
    setup({
      devices: [device({ id: "dev-9", current_slot: 1 })],
      nearby: [nearby({ paired: true, seen_ago_ms: 7000 })],
    });
    expect(await screen.findByText("visto há 7 s")).toBeTruthy();
    // O selo "Conectado" aparece no card de próximos e, por "Conectado em P2", no da lista.
    expect((await screen.findAllByText("Conectado")).length).toBeGreaterThan(0);
    expect(screen.queryByRole("button", { name: "Parear" })).toBeNull();
  });

  it("sem nada por perto nem pareado, mostra o estado vazio", async () => {
    setup();
    expect(await screen.findByText("Nenhum dispositivo ainda")).toBeTruthy();
    expect(screen.getByText("Nenhum celular com o PadGaming aberto por perto.")).toBeTruthy();
  });
});

describe("aberto pela bandeja", () => {
  it("com ?pair=1 já mostra o código de pareamento", async () => {
    setup({}, "/devices?pair=1");
    expect(await screen.findByRole("img", { name: "QR code para parear" })).toBeTruthy();
    expect(commands.startPairing).toHaveBeenCalledTimes(1);
    fireEvent.click(screen.getByRole("button", { name: "Fechar" }));
    await waitFor(() => {
      expect(screen.queryByRole("img", { name: "QR code para parear" })).toBeNull();
    });
  });
});

describe("modal de parear", () => {
  it("gera o código ao abrir, mostra QR e PIN e invalida ao fechar", async () => {
    setup();
    fireEvent.click(await screen.findByRole("button", { name: "Parear novo" }));

    const qr = await screen.findByRole("img", { name: "QR code para parear" });
    expect(qr.querySelector("svg")).not.toBeNull();
    expect(screen.getByText(/AB12CD34/)).toBeTruthy();
    expect(screen.getByText(/192\.168\.0\.10:47800/)).toBeTruthy();
    expect(commands.startPairing).toHaveBeenCalledTimes(1);

    fireEvent.click(screen.getByRole("radio", { name: "PIN" }));
    expect(screen.getByLabelText("PIN de pareamento").textContent).toBe("482913");

    fireEvent.click(screen.getByRole("button", { name: "Fechar" }));
    await waitFor(() => {
      expect(commands.cancelPairing).toHaveBeenCalled();
    });
  });

  it("renova o código sozinho quando ele vence", async () => {
    setup({ offer: offer({ token_ttl_s: 1, pin_ttl_s: 1 }) });
    fireEvent.click(await screen.findByRole("button", { name: "Parear novo" }));
    await screen.findByRole("img", { name: "QR code para parear" });
    await waitFor(
      () => {
        expect(commands.startPairing.mock.calls.length).toBeGreaterThanOrEqual(2);
      },
      { timeout: 4000 },
    );
  });

  it("fecha sozinho com a animação de sucesso quando o pareamento conclui", async () => {
    setup();
    fireEvent.click(await screen.findByRole("button", { name: "Parear novo" }));
    await screen.findByRole("img", { name: "QR code para parear" });

    act(() => {
      useLastPaired.getState().record({ device_id: "dev-1", name: "Galaxy da Ana", model: "S24" });
    });
    expect(await screen.findByText("Galaxy da Ana pareado!")).toBeTruthy();
    await waitFor(
      () => {
        expect(screen.queryByText("Galaxy da Ana pareado!")).toBeNull();
      },
      { timeout: 4000 },
    );
  });

  it("um erro ao gerar o código oferece tentar de novo", async () => {
    commands.startPairing.mockRejectedValueOnce({ code: "internal" });
    setup();
    fireEvent.click(await screen.findByRole("button", { name: "Parear novo" }));
    fireEvent.click(await screen.findByRole("button", { name: "Tentar de novo" }));
    await screen.findByRole("img", { name: "QR code para parear" });
  });
});

describe("pareados", () => {
  it("renomeia pelo menu e salva com Enter", async () => {
    commands.renameDevice.mockResolvedValue("Celular da Ana");
    setup({ devices: [device()] });
    await screen.findByText("Galaxy da Ana");

    openMenu("Galaxy da Ana");
    fireEvent.click(await screen.findByRole("menuitem", { name: "Renomear" }));
    const input = await screen.findByRole("textbox", { name: "Nome do dispositivo" });
    fireEvent.change(input, { target: { value: "Celular da Ana" } });
    fireEvent.keyDown(input, { key: "Enter" });
    await waitFor(() => {
      expect(commands.renameDevice).toHaveBeenCalledWith("dev-1", "Celular da Ana");
    });
  });

  it("Escape cancela a edição sem salvar", async () => {
    setup({ devices: [device()] });
    await screen.findByText("Galaxy da Ana");
    openMenu("Galaxy da Ana");
    fireEvent.click(await screen.findByRole("menuitem", { name: "Renomear" }));
    const input = await screen.findByRole("textbox", { name: "Nome do dispositivo" });
    fireEvent.change(input, { target: { value: "Outro" } });
    fireEvent.keyDown(input, { key: "Escape" });
    expect(screen.queryByRole("textbox", { name: "Nome do dispositivo" })).toBeNull();
    expect(commands.renameDevice).not.toHaveBeenCalled();
  });

  it("revogar pede confirmação e derruba o dispositivo", async () => {
    commands.revokeDevice.mockResolvedValue(null);
    setup({ devices: [device({ current_slot: 0 })] });
    await screen.findByText("Galaxy da Ana");

    openMenu("Galaxy da Ana");
    fireEvent.click(await screen.findByRole("menuitem", { name: "Revogar" }));
    expect(commands.revokeDevice).not.toHaveBeenCalled();
    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText("Revogar Galaxy da Ana?")).toBeTruthy();
    fireEvent.click(within(dialog).getByRole("button", { name: "Revogar" }));
    await waitFor(() => {
      expect(commands.revokeDevice).toHaveBeenCalledWith("dev-1");
    });
  });

  it("esquecer também confirma antes", async () => {
    commands.forgetDevice.mockResolvedValue(null);
    setup({ devices: [device()] });
    await screen.findByText("Galaxy da Ana");
    openMenu("Galaxy da Ana");
    fireEvent.click(await screen.findByRole("menuitem", { name: "Esquecer" }));
    const dialog = await screen.findByRole("dialog");
    fireEvent.click(within(dialog).getByRole("button", { name: "Esquecer" }));
    await waitFor(() => {
      expect(commands.forgetDevice).toHaveBeenCalledWith("dev-1");
    });
  });

  it("define o slot preferido pelo menu", async () => {
    commands.setDeviceDefaults.mockResolvedValue(null);
    setup({ devices: [device({ color: "#ff0000" })] });
    await screen.findByText("Galaxy da Ana");

    openMenu("Galaxy da Ana");
    const slotItem = await screen.findByRole("menuitem", { name: "Definir slot" });
    act(() => {
      fireEvent.keyDown(slotItem, { key: "ArrowRight" });
    });
    fireEvent.click(await screen.findByRole("menuitem", { name: "P3" }));
    await waitFor(() => {
      expect(commands.setDeviceDefaults).toHaveBeenCalledWith("dev-1", {
        preferred_slot: 2,
        default_profile_id: null,
        color: "#ff0000",
      });
    });
  });

  it("um dispositivo revogado mostra o selo e não pode ser revogado de novo", async () => {
    setup({ devices: [device({ revoked: true })] });
    expect(await screen.findByText("Revogado")).toBeTruthy();
    openMenu("Galaxy da Ana");
    const item = await screen.findByRole("menuitem", { name: "Revogar" });
    expect(item.getAttribute("aria-disabled")).toBe("true");
  });
});

describe("histórico", () => {
  it("fica recolhido e só consulta o banco quando abre", async () => {
    const log: LogEntryDto[] = [
      {
        id: 2,
        device_id: "dev-1",
        event: "connected",
        slot: 0,
        detail: null,
        at: 1_700_000_000_000,
      },
      {
        id: 1,
        device_id: "ghost",
        event: "disconnected",
        slot: null,
        detail: "timeout",
        at: 1_699_999_000_000,
      },
    ];
    setup({ devices: [device()], log });
    await screen.findByText("Galaxy da Ana");
    expect(commands.getConnectionLog).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "Histórico de conexões" }));
    expect(await screen.findByText("Conectou")).toBeTruthy();
    expect(screen.getByText("Desconectou")).toBeTruthy();
    expect(screen.getByText(/Dispositivo removido/)).toBeTruthy();
    expect(commands.getConnectionLog).toHaveBeenCalledWith({
      device_id: null,
      before_id: null,
      limit: 50,
    });
  });
});

describe("aprovação", () => {
  const prompt = {
    request_id: 7,
    device_id: "dev-9",
    name: "Galaxy S24",
    model: "SM-S921",
    peer: "192.168.0.50",
    timeout_s: 30,
  };

  it("pergunta se pode conectar e responde Permitir", async () => {
    commands.answerApproval.mockResolvedValue(true);
    setup();
    act(() => {
      useApprovals.getState().add(prompt);
    });
    expect(await screen.findByText("Galaxy S24 quer se conectar")).toBeTruthy();
    expect(screen.getByText("Expira em 30 s")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Permitir" }));
    await waitFor(() => {
      expect(commands.answerApproval).toHaveBeenCalledWith(7, true);
    });
  });

  it("fechar sem responder equivale a negar", async () => {
    commands.answerApproval.mockResolvedValue(true);
    setup();
    act(() => {
      useApprovals.getState().add(prompt);
    });
    const dialog = await screen.findByRole("dialog");
    fireEvent.keyDown(dialog, { key: "Escape" });
    await waitFor(() => {
      expect(commands.answerApproval).toHaveBeenCalledWith(7, false);
    });
  });

  it("some quando o backend resolve o pedido", async () => {
    setup();
    act(() => {
      useApprovals.getState().add(prompt);
    });
    await screen.findByText("Galaxy S24 quer se conectar");
    act(() => {
      useApprovals.getState().remove(7);
    });
    await waitFor(() => {
      expect(screen.queryByText("Galaxy S24 quer se conectar")).toBeNull();
    });
  });
});
