import type { ReactNode } from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ProfileDto } from "@/lib/bindings";
import { commands, resetBindingsMock } from "@/test/bindings-mock";
import { setStick, stickOf } from "./profileModel";
import { SAVE_DEBOUNCE_MS, UNDO_COALESCE_MS, useProfileEditor } from "./useProfileEditor";

vi.mock("@/lib/bindings", async () => (await import("@/test/bindings-mock")).bindingsMock);

const dto = (patch: Partial<ProfileDto["summary"]> = {}): ProfileDto => ({
  summary: {
    id: "p1",
    name: "FPS",
    description: null,
    icon: null,
    is_builtin: false,
    schema_version: 1,
    created_at: 0,
    updated_at: 0,
    ...patch,
  },
  profile: {},
});

function setup(profile: ProfileDto = dto()) {
  const wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider
      client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}
    >
      {children}
    </QueryClientProvider>
  );
  return renderHook(() => useProfileEditor(profile), { wrapper });
}

beforeEach(() => {
  vi.useFakeTimers();
  resetBindingsMock();
  commands.saveProfile.mockImplementation((_id: string, input: { name: string }) =>
    Promise.resolve({ summary: { ...dto().summary, name: input.name }, profile: {} }),
  );
  commands.listProfiles.mockResolvedValue([]);
});

afterEach(() => {
  vi.useRealTimers();
});

const sensitivity = (view: ReturnType<typeof setup>) =>
  stickOf(view.result.current.draft.profile, "left").sensitivity;

describe("salvamento automático", () => {
  it("grava 300 ms depois da última alteração, uma vez só", async () => {
    const view = setup();
    act(() => {
      view.result.current.edit((p) => setStick(p, "left", { sensitivity: 1.5 }));
    });
    expect(view.result.current.saveState).toBe("pending");
    act(() => {
      vi.advanceTimersByTime(SAVE_DEBOUNCE_MS - 50);
    });
    act(() => {
      view.result.current.edit((p) => setStick(p, "left", { sensitivity: 2 }));
    });
    act(() => {
      vi.advanceTimersByTime(SAVE_DEBOUNCE_MS - 50);
    });
    expect(commands.saveProfile).not.toHaveBeenCalled();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(60);
    });
    expect(commands.saveProfile).toHaveBeenCalledTimes(1);
    const [id, input] = commands.saveProfile.mock.calls[0] as [string, { profile: unknown }];
    expect(id).toBe("p1");
    expect(stickOf(input.profile as never, "left").sensitivity).toBe(2);
    expect(view.result.current.saveState).toBe("saved");
  });

  it("sair da tela grava o que ainda esperava o debounce", async () => {
    const view = setup();
    act(() => {
      view.result.current.edit((p) => setStick(p, "left", { invert_x: true }));
    });
    expect(commands.saveProfile).not.toHaveBeenCalled();
    view.unmount();
    await act(async () => {
      await Promise.resolve();
    });
    expect(commands.saveProfile).toHaveBeenCalledTimes(1);
  });

  it("renomear também grava, com o nome novo", async () => {
    const view = setup();
    act(() => {
      view.result.current.editMeta({ name: "Corrida" });
    });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(SAVE_DEBOUNCE_MS + 10);
    });
    expect(commands.saveProfile).toHaveBeenCalledWith(
      "p1",
      expect.objectContaining({ name: "Corrida" }),
    );
  });

  it("um erro ao gravar aparece no estado", async () => {
    commands.saveProfile.mockRejectedValue({ code: "invalid_input", message_key: "x" });
    const view = setup();
    act(() => {
      view.result.current.edit((p) => setStick(p, "left", { sensitivity: 3 }));
    });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(SAVE_DEBOUNCE_MS + 10);
    });
    expect(view.result.current.saveState).toBe("error");
  });
});

describe("desfazer", () => {
  it("volta ao estado anterior e grava de novo", async () => {
    const view = setup();
    act(() => {
      view.result.current.edit((p) => setStick(p, "left", { sensitivity: 2 }));
    });
    expect(view.result.current.canUndo).toBe(true);
    act(() => {
      view.result.current.undo();
    });
    expect(sensitivity(view)).toBe(1);
    expect(view.result.current.canUndo).toBe(false);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(SAVE_DEBOUNCE_MS + 10);
    });
    const [, input] = commands.saveProfile.mock.calls[0] as [string, { profile: never }];
    expect(stickOf(input.profile, "left").sensitivity).toBe(1);
  });

  it("alterações em sequência (arrastar um slider) valem um passo só", () => {
    const view = setup();
    for (const value of [1.1, 1.2, 1.3, 1.4]) {
      act(() => {
        view.result.current.edit((p) => setStick(p, "left", { sensitivity: value }));
        vi.advanceTimersByTime(100);
      });
    }
    act(() => {
      view.result.current.undo();
    });
    expect(sensitivity(view)).toBe(1);
    expect(view.result.current.canUndo).toBe(false);
  });

  it("gestos separados por uma pausa são passos separados", () => {
    const view = setup();
    act(() => {
      view.result.current.edit((p) => setStick(p, "left", { sensitivity: 2 }));
      vi.advanceTimersByTime(UNDO_COALESCE_MS + 100);
      view.result.current.edit((p) => setStick(p, "left", { sensitivity: 3 }));
    });
    act(() => {
      view.result.current.undo();
    });
    expect(sensitivity(view)).toBe(2);
    act(() => {
      view.result.current.undo();
    });
    expect(sensitivity(view)).toBe(1);
  });

  it("desfazer sem histórico não faz nada", () => {
    const view = setup();
    act(() => {
      view.result.current.undo();
    });
    expect(commands.saveProfile).not.toHaveBeenCalled();
    expect(view.result.current.saveState).toBe("idle");
  });
});

describe("perfil embutido", () => {
  it("é só de leitura: nenhuma alteração entra nem grava", async () => {
    const view = setup(dto({ is_builtin: true, name: "Padrão" }));
    expect(view.result.current.readOnly).toBe(true);
    act(() => {
      view.result.current.edit((p) => setStick(p, "left", { sensitivity: 3 }));
      view.result.current.editMeta({ name: "Outro" });
    });
    expect(sensitivity(view)).toBe(1);
    expect(view.result.current.draft.name).toBe("Padrão");
    await act(async () => {
      await vi.advanceTimersByTimeAsync(SAVE_DEBOUNCE_MS + 10);
    });
    expect(commands.saveProfile).not.toHaveBeenCalled();
  });
});
