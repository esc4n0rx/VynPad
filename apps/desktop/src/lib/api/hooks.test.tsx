import type { ReactNode } from "react";
import { QueryClientProvider } from "@tanstack/react-query";
import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { createQueryClient } from "@/app/queryClient";
import type { AppError } from "@/lib/bindings";
import { useToasts } from "@/lib/toast";
import { commands, resetBindingsMock } from "@/test/bindings-mock";
import { useDeleteProfile, useProfiles, useSaveProfile } from "./profiles";
import { useSettings, useUpdateSettings, type Settings } from "./settings";

vi.mock("@/lib/bindings", async () => (await import("@/test/bindings-mock")).bindingsMock);

const defaults: Settings = {
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
};

function error(code: AppError["code"], extra: Partial<AppError> = {}): AppError {
  return { code, message_key: `errors.${code}`, details: null, in_use: null, ...extra };
}

function setup() {
  const queryClient = createQueryClient();
  // Sem tentar de novo: o teste de erro não espera o `retry` do padrão.
  queryClient.setDefaultOptions({ queries: { retry: false } });
  const wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );
  return { queryClient, wrapper };
}

beforeEach(() => {
  resetBindingsMock();
  useToasts.setState({ toasts: [] });
});

describe("configurações", () => {
  it("carrega do backend e a gravação atualiza o cache", async () => {
    commands.getSettings.mockResolvedValue(defaults);
    const { wrapper, queryClient } = setup();
    const view = renderHook(() => useSettings(), { wrapper });
    await waitFor(() => expect(view.result.current.data).toEqual(defaults));

    const update = renderHook(() => useUpdateSettings(), { wrapper });
    const saved = { ...defaults, theme: "light" as const };
    commands.updateSettings.mockResolvedValue(saved);
    await act(async () => {
      await update.result.current.mutateAsync(saved);
    });
    expect(commands.updateSettings).toHaveBeenCalledWith(saved);
    expect(queryClient.getQueryData(["settings"])).toEqual(saved);
  });

  it("um erro de validação vira aviso com a mensagem e o detalhe", async () => {
    commands.updateSettings.mockRejectedValue(
      error("invalid_input", { details: "a cor de destaque precisa estar no formato #RRGGBB" }),
    );
    const { wrapper } = setup();
    const update = renderHook(() => useUpdateSettings(), { wrapper });
    await act(async () => {
      await update.result.current.mutateAsync(defaults).catch(() => undefined);
    });
    const [toast] = useToasts.getState().toasts;
    expect(toast?.kind).toBe("error");
    expect(toast?.message).toContain("Um valor informado não é válido.");
    expect(toast?.message).toContain("#RRGGBB");
  });
});

describe("perfis", () => {
  it("uma consulta que falha mostra o aviso traduzido", async () => {
    commands.listProfiles.mockRejectedValue(error("internal"));
    const { wrapper } = setup();
    renderHook(() => useProfiles(), { wrapper });
    await waitFor(() => expect(useToasts.getState().toasts).toHaveLength(1));
    expect(useToasts.getState().toasts[0]?.message).toContain("Algo deu errado");
  });

  it("apagar um perfil em uso não mostra aviso: a tela pede confirmação com os números", async () => {
    commands.deleteProfile.mockRejectedValueOnce(
      error("profile_in_use", { in_use: { devices: 2, slots: 1 } }),
    );
    const { wrapper } = setup();
    const del = renderHook(() => useDeleteProfile(), { wrapper });

    const failure = await act(async () =>
      del.result.current.mutateAsync({ id: "p", confirm: false }).then(
        () => null,
        (e: AppError) => e,
      ),
    );
    expect(failure?.code).toBe("profile_in_use");
    expect(failure?.in_use).toEqual({ devices: 2, slots: 1 });
    expect(useToasts.getState().toasts).toHaveLength(0);

    // Confirmado, apaga.
    await act(async () => {
      await del.result.current.mutateAsync({ id: "p", confirm: true });
    });
    expect(commands.deleteProfile).toHaveBeenLastCalledWith("p", true);
  });

  it("salvar um perfil refaz a lista", async () => {
    commands.listProfiles.mockResolvedValue([]);
    const saved = { summary: { id: "p1", name: "x" }, profile: {} };
    commands.saveProfile.mockResolvedValue(saved);
    const { wrapper, queryClient } = setup();
    const list = renderHook(() => useProfiles(), { wrapper });
    await waitFor(() => expect(commands.listProfiles).toHaveBeenCalledTimes(1));

    const save = renderHook(() => useSaveProfile(), { wrapper });
    await act(async () => {
      await save.result.current.mutateAsync({
        id: null,
        input: { name: "x", description: null, icon: null, profile: {} },
      });
    });
    await waitFor(() => expect(commands.listProfiles).toHaveBeenCalledTimes(2));
    expect(list.result.current.isError).toBe(false);
    // O perfil gravado fica no cache para o editor reabrir sem ler de novo.
    expect(queryClient.getQueryData(["profiles", "p1"])).toEqual(saved);
  });
});
