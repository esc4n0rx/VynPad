import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useLiveInput } from "@/lib/api/live";
import { useSlots } from "@/lib/api/slots";
import type { AppError, Profile, ProfileDto, ProfileSummaryDto } from "@/lib/bindings";
import { useToasts } from "@/lib/toast";
import { commands, resetBindingsMock } from "@/test/bindings-mock";
import { MappingPage } from "./MappingPage";
import { SAVE_DEBOUNCE_MS } from "./useProfileEditor";

vi.mock("@/lib/bindings", async () => (await import("@/test/bindings-mock")).bindingsMock);
vi.mock("@/lib/log", () => ({ log: { warn: vi.fn(), error: vi.fn(), info: vi.fn() } }));

const summary = (id: string, name: string, builtin = false): ProfileSummaryDto => ({
  id,
  name,
  description: null,
  icon: null,
  is_builtin: builtin,
  schema_version: 1,
  created_at: 0,
  updated_at: 0,
});

const store: Record<string, ProfileDto> = {};
const add = (id: string, name: string, profile: Profile = {}, builtin = false) => {
  store[id] = { summary: summary(id, name, builtin), profile };
};

beforeEach(() => {
  resetBindingsMock();
  for (const key of Object.keys(store)) delete store[key];
  add("padrao", "Padrão", {}, true);
  add("fps", "FPS", { sticks: { left: { sensitivity: 1.5 } } });
  commands.listProfiles.mockImplementation(() =>
    Promise.resolve(Object.values(store).map((d) => d.summary)),
  );
  commands.getProfile.mockImplementation((id: string) => Promise.resolve(store[id]));
  commands.saveProfile.mockImplementation(
    (id: string | null, input: { name: string; profile: Profile }) => {
      const key = id ?? "novo";
      store[key] = { summary: summary(key, input.name), profile: input.profile };
      return Promise.resolve(store[key]);
    },
  );
  commands.subscribeLiveInput.mockResolvedValue(null);
  commands.unsubscribeLiveInput.mockResolvedValue(undefined);
  useSlots.setState({ slots: null, health: {} });
  useToasts.setState({ toasts: [] });
  // O Radix usa APIs de ponteiro que o jsdom não tem.
  Element.prototype.hasPointerCapture = () => false;
  Element.prototype.releasePointerCapture = () => undefined;
  Element.prototype.scrollIntoView = () => undefined;
});

afterEach(() => {
  cleanup();
  useLiveInput.setState({ reports: {} });
});

function setup() {
  return render(
    <QueryClientProvider
      client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}
    >
      <MappingPage />
    </QueryClientProvider>,
  );
}

function pick(container: HTMLElement, element: string) {
  const target = container.querySelector(`[data-el="${element}"]`);
  if (!target) throw new Error(`sem elemento ${element}`);
  fireEvent.click(target);
}

/** Aperta a seta no slider com o nome dado (o Radix trata o teclado no thumb). */
function nudge(name: string, key: string, times = 1) {
  const thumb = screen.getByRole("slider", { name });
  for (let i = 0; i < times; i++) fireEvent.keyDown(thumb, { key });
}

describe("lista de perfis", () => {
  it("mostra os perfis e abre o primeiro, o embutido, só para leitura", async () => {
    setup();
    expect(await screen.findByRole("button", { name: /^Padrão/ })).toBeTruthy();
    expect(screen.getByRole("button", { name: /^FPS/ })).toBeTruthy();
    expect(
      (await screen.findByRole("textbox", { name: "Nome do perfil" })).hasAttribute("disabled"),
    ).toBe(true);
    expect(screen.getByText("Perfil embutido: duplique para editar.")).toBeTruthy();
  });

  it("escolher outro perfil carrega as propriedades dele", async () => {
    const view = setup();
    fireEvent.click(await screen.findByRole("button", { name: /^FPS/ }));
    const name = await screen.findByDisplayValue("FPS");
    expect(name.hasAttribute("disabled")).toBe(false);
    pick(view.container, "stick-left");
    expect(await screen.findByRole("slider", { name: "Sensibilidade" })).toBeTruthy();
    expect(screen.getByText("1,50×")).toBeTruthy();
  });

  it("criar um perfil grava e abre o novo", async () => {
    setup();
    await screen.findByRole("button", { name: /^FPS/ });
    fireEvent.click(screen.getByRole("button", { name: "Novo perfil" }));
    await waitFor(() => {
      expect(commands.saveProfile).toHaveBeenCalledWith(null, {
        name: "Novo perfil",
        description: null,
        icon: null,
        profile: {},
      });
    });
    expect(await screen.findByDisplayValue("Novo perfil")).toBeTruthy();
  });

  it("duplicar usa o nome com o sufixo de cópia", async () => {
    commands.duplicateProfile.mockImplementation((id: string, name: string) => {
      add("copia", name, store[id]?.profile);
      return Promise.resolve(store.copia);
    });
    setup();
    await screen.findByRole("button", { name: /^FPS/ });
    const menu = screen.getByRole("button", { name: "Ações do perfil FPS" });
    act(() => {
      fireEvent.keyDown(menu, { key: "Enter" });
    });
    fireEvent.click(await screen.findByRole("menuitem", { name: "Duplicar" }));
    await waitFor(() => {
      expect(commands.duplicateProfile).toHaveBeenCalledWith("fps", "FPS (cópia)");
    });
    expect(await screen.findByDisplayValue("FPS (cópia)")).toBeTruthy();
  });

  it("excluir um perfil em uso pede confirmação com os números e depois exclui", async () => {
    const inUse: AppError = {
      code: "profile_in_use",
      message_key: "errors.profile_in_use",
      details: null,
      in_use: { devices: 2, slots: 1 },
    };
    commands.deleteProfile.mockRejectedValueOnce(inUse).mockImplementation((id: string) => {
      delete store[id];
      return Promise.resolve(null);
    });
    setup();
    await screen.findByRole("button", { name: /^FPS/ });
    const menu = screen.getByRole("button", { name: "Ações do perfil FPS" });
    act(() => {
      fireEvent.keyDown(menu, { key: "Enter" });
    });
    fireEvent.click(await screen.findByRole("menuitem", { name: "Excluir" }));

    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText(/2 dispositivo\(s\)/)).toBeTruthy();
    expect(within(dialog).getByText(/1 controle\(s\)/)).toBeTruthy();
    expect(commands.deleteProfile).toHaveBeenCalledTimes(1);
    fireEvent.click(within(dialog).getByRole("button", { name: "Excluir" }));
    await waitFor(() => {
      expect(commands.deleteProfile).toHaveBeenLastCalledWith("fps", true);
    });
  });

  it("o perfil embutido não pode ser excluído", async () => {
    setup();
    await screen.findByRole("button", { name: /^Padrão/ });
    const menu = screen.getByRole("button", { name: "Ações do perfil Padrão" });
    act(() => {
      fireEvent.keyDown(menu, { key: "Enter" });
    });
    const item = await screen.findByRole("menuitem", { name: "Excluir" });
    expect(item.getAttribute("aria-disabled")).toBe("true");
  });

  it("importa um arquivo .padprofile e abre o perfil importado", async () => {
    commands.importProfile.mockImplementation((text: string) => {
      const parsed = JSON.parse(text) as { name: string };
      add("importado", parsed.name);
      return Promise.resolve(store.importado);
    });
    setup();
    await screen.findByRole("button", { name: /^FPS/ });
    const file = new File([JSON.stringify({ name: "Do amigo" })], "amigo.padprofile", {
      type: "application/json",
    });
    // O `File` do jsdom ainda não tem `text()`.
    Object.defineProperty(file, "text", { value: () => Promise.resolve('{"name":"Do amigo"}') });
    fireEvent.change(screen.getByLabelText("Importar", { selector: "input" }), {
      target: { files: [file] },
    });
    await waitFor(() => {
      expect(commands.importProfile).toHaveBeenCalledWith('{"name":"Do amigo"}');
    });
    expect(await screen.findByDisplayValue("Do amigo")).toBeTruthy();
  });

  it("exportar baixa o texto do perfil como .padprofile", async () => {
    commands.exportProfile.mockResolvedValue('{"name":"FPS"}');
    const created: string[] = [];
    vi.stubGlobal("URL", {
      createObjectURL: (blob: Blob) => {
        created.push(String(blob.size));
        return "blob:fake";
      },
      revokeObjectURL: vi.fn(),
    });
    const click = vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(function (
      this: HTMLAnchorElement,
    ) {
      expect(this.download).toBe("FPS.padprofile");
    });
    setup();
    await screen.findByRole("button", { name: /^FPS/ });
    const menu = screen.getByRole("button", { name: "Ações do perfil FPS" });
    act(() => {
      fireEvent.keyDown(menu, { key: "Enter" });
    });
    fireEvent.click(await screen.findByRole("menuitem", { name: "Exportar" }));
    await waitFor(() => {
      expect(click).toHaveBeenCalledTimes(1);
    });
    expect(commands.exportProfile).toHaveBeenCalledWith("fps");
    click.mockRestore();
    vi.unstubAllGlobals();
  });

  it("restaurar o padrão pede confirmação", async () => {
    commands.restoreDefaultProfile.mockResolvedValue(null);
    setup();
    await screen.findByRole("button", { name: /^FPS/ });
    fireEvent.click(screen.getByRole("button", { name: "Restaurar padrão" }));
    expect(commands.restoreDefaultProfile).not.toHaveBeenCalled();
    const dialog = await screen.findByRole("dialog");
    fireEvent.click(within(dialog).getByRole("button", { name: "Restaurar" }));
    await waitFor(() => {
      expect(commands.restoreDefaultProfile).toHaveBeenCalledTimes(1);
    });
  });
});

describe("propriedades", () => {
  async function openFps() {
    const view = setup();
    fireEvent.click(await screen.findByRole("button", { name: /^FPS/ }));
    await screen.findByDisplayValue("FPS");
    return view;
  }

  it("sem elemento escolhido, orienta a escolher um", async () => {
    setup();
    expect(await screen.findByText(/Selecione um botão, gatilho ou analógico/)).toBeTruthy();
  });

  it("mexer na deadzone grava depois do debounce, sem reconectar", async () => {
    const view = await openFps();
    pick(view.container, "stick-left");
    nudge("Deadzone interna", "ArrowRight", 5);
    expect(commands.saveProfile).not.toHaveBeenCalled();
    await waitFor(
      () => {
        expect(commands.saveProfile).toHaveBeenCalledTimes(1);
      },
      { timeout: SAVE_DEBOUNCE_MS * 4 },
    );
    const [id, input] = commands.saveProfile.mock.calls[0] as [string, { profile: Profile }];
    expect(id).toBe("fps");
    expect(input.profile.sticks?.left?.deadzone_inner).toBeCloseTo(0.05);
    // O que já estava no perfil continua lá.
    expect(input.profile.sticks?.left?.sensitivity).toBe(1.5);
    expect(await screen.findByText("Salvo")).toBeTruthy();
  });

  it("Ctrl+Z desfaz a última alteração", async () => {
    const view = await openFps();
    pick(view.container, "stick-left");
    nudge("Sensibilidade", "ArrowRight", 3);
    expect(screen.getByText("1,65×")).toBeTruthy();
    fireEvent.keyDown(window, { key: "z", ctrlKey: true });
    expect(await screen.findByText("1,50×")).toBeTruthy();
  });

  it("Ctrl+Z dentro de um campo de texto fica com o campo", async () => {
    const view = await openFps();
    pick(view.container, "stick-left");
    nudge("Sensibilidade", "ArrowRight", 3);
    fireEvent.keyDown(screen.getByRole("textbox", { name: "Nome do perfil" }), {
      key: "z",
      ctrlKey: true,
    });
    expect(screen.getByText("1,65×")).toBeTruthy();
  });

  it("o botão pode ser mapeado para outro e ganhar turbo", async () => {
    const view = await openFps();
    pick(view.container, "btn-a");
    expect(await screen.findByText("Botão A")).toBeTruthy();

    const select = screen.getByRole("combobox", { name: "Mapear para" });
    fireEvent.keyDown(select, { key: "Enter" });
    fireEvent.click(await screen.findByRole("option", { name: "B" }));

    fireEvent.click(screen.getByRole("switch", { name: "Turbo" }));
    await waitFor(
      () => {
        expect(commands.saveProfile).toHaveBeenCalled();
      },
      { timeout: SAVE_DEBOUNCE_MS * 4 },
    );
    const calls = commands.saveProfile.mock.calls as [string, { profile: Profile }][];
    const last = calls[calls.length - 1]?.[1].profile;
    expect(last?.buttons?.remap?.a).toBe("b");
    expect(last?.buttons?.turbo).toEqual([{ button: "a", hz: 10 }]);
    expect(screen.getByRole("slider", { name: "Velocidade do turbo" })).toBeTruthy();
  });

  it("o gatilho tem deadzone, ponto máximo e modo digital", async () => {
    const view = await openFps();
    pick(view.container, "rt");
    expect((await screen.findAllByText("RT (gatilho direito)")).length).toBeGreaterThan(0);
    expect(screen.getByRole("slider", { name: "Deadzone" })).toBeTruthy();
    expect(screen.getByRole("slider", { name: "Ponto máximo" })).toBeTruthy();
    fireEvent.click(screen.getByRole("switch", { name: "Modo digital" }));
    await waitFor(
      () => {
        expect(commands.saveProfile).toHaveBeenCalled();
      },
      { timeout: SAVE_DEBOUNCE_MS * 4 },
    );
    const [, input] = commands.saveProfile.mock.calls[0] as [string, { profile: Profile }];
    expect(input.profile.triggers?.right?.digital).toBe(true);
  });

  it("a curva personalizada tem 4 pontos que se movem pelo teclado", async () => {
    const view = await openFps();
    pick(view.container, "stick-right");
    fireEvent.click(await screen.findByRole("radio", { name: "Personalizada" }));
    const point = await screen.findByRole("slider", { name: "Ponto 2 da curva" });
    expect(screen.getAllByRole("slider", { name: /Ponto \d da curva/ })).toHaveLength(4);
    fireEvent.keyDown(point, { key: "ArrowUp" });
    await waitFor(
      () => {
        expect(commands.saveProfile).toHaveBeenCalled();
      },
      { timeout: SAVE_DEBOUNCE_MS * 4 },
    );
    const calls = commands.saveProfile.mock.calls as [string, { profile: Profile }][];
    const curve = calls[calls.length - 1]?.[1].profile.sticks?.right?.curve;
    expect(curve?.type).toBe("custom");
    if (curve?.type === "custom") expect(curve.points[1]?.[1]).toBeCloseTo(0.52);
  });

  it("a aba do gyro liga o gyro e mostra as opções", async () => {
    await openFps();
    fireEvent.click(screen.getByRole("radio", { name: "Gyro" }));
    fireEvent.click(await screen.findByRole("switch", { name: "Ativar o gyro" }));
    expect(screen.getByRole("slider", { name: "Sensibilidade X" })).toBeTruthy();
    fireEvent.click(screen.getByRole("radio", { name: "Segurando" }));
    expect(await screen.findByRole("combobox", { name: "Botão do modo" })).toBeTruthy();
    await waitFor(
      () => {
        expect(commands.saveProfile).toHaveBeenCalled();
      },
      { timeout: SAVE_DEBOUNCE_MS * 4 },
    );
    const calls = commands.saveProfile.mock.calls as [string, { profile: Profile }][];
    const gyro = calls[calls.length - 1]?.[1].profile.gyro;
    expect(gyro?.enabled).toBe(true);
    expect(gyro?.mode).toEqual({ type: "while_held", button: "left_shoulder" });
  });

  it("no perfil embutido todos os controles ficam desabilitados", async () => {
    const view = setup();
    await screen.findByText("Perfil embutido: duplique para editar.");
    pick(view.container, "stick-left");
    const slider = await screen.findByRole("slider", { name: "Sensibilidade" });
    expect(slider.getAttribute("data-disabled")).not.toBeNull();
    expect(screen.getByRole("switch", { name: "Inverter X" }).hasAttribute("disabled")).toBe(true);
  });
});

describe("pré-visualização", () => {
  it("segue o slot escolhido: assina o input ao vivo dele", async () => {
    setup();
    await screen.findByText("Perfil embutido: duplique para editar.");
    expect(commands.subscribeLiveInput).not.toHaveBeenCalled();
    const select = screen.getByRole("combobox", { name: "Pré-visualizar com" });
    fireEvent.keyDown(select, { key: "Enter" });
    fireEvent.click(await screen.findByRole("option", { name: "P3" }));
    await waitFor(() => {
      expect(commands.subscribeLiveInput).toHaveBeenCalledWith(2);
    });
  });
});
