import { StrictMode } from "react";
import { renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { commands, resetBindingsMock } from "@/test/bindings-mock";
import { resetLiveSubscriptions, useLiveInput, useLiveReport } from "./live";

vi.mock("@/lib/bindings", async () => (await import("@/test/bindings-mock")).bindingsMock);

const report = { buttons: 0, lt: 0, rt: 0, lx: 5, ly: 0, rx: 0, ry: 0 };

beforeEach(() => {
  resetBindingsMock();
  resetLiveSubscriptions();
});

describe("useLiveReport", () => {
  it("sem nenhum componente usando, o backend nunca é acionado", () => {
    expect(commands.subscribeLiveInput).not.toHaveBeenCalled();
    renderHook(() => useLiveReport(null));
    expect(commands.subscribeLiveInput).not.toHaveBeenCalled();
  });

  it("assina ao montar e cancela ao desmontar, e mostra o estado do slot", () => {
    const view = renderHook(() => useLiveReport(1));
    expect(commands.subscribeLiveInput).toHaveBeenCalledWith(1);
    expect(view.result.current).toBeUndefined();

    useLiveInput.getState().set(1, report);
    view.rerender();
    expect(view.result.current).toEqual(report);

    view.unmount();
    expect(commands.unsubscribeLiveInput).toHaveBeenCalledWith(1);
    // O último estado não fica para trás.
    expect(useLiveInput.getState().reports[1]).toBeUndefined();
  });

  it("várias telas no mesmo slot: o backend só vê a primeira e a última", () => {
    const a = renderHook(() => useLiveReport(0));
    const b = renderHook(() => useLiveReport(0));
    expect(commands.subscribeLiveInput).toHaveBeenCalledTimes(1);
    a.unmount();
    expect(commands.unsubscribeLiveInput).not.toHaveBeenCalled();
    b.unmount();
    expect(commands.unsubscribeLiveInput).toHaveBeenCalledTimes(1);
  });

  it("trocar de slot cancela o antigo e assina o novo", () => {
    const view = renderHook(({ slot }) => useLiveReport(slot), { initialProps: { slot: 0 } });
    view.rerender({ slot: 2 });
    expect(commands.unsubscribeLiveInput).toHaveBeenCalledWith(0);
    expect(commands.subscribeLiveInput).toHaveBeenLastCalledWith(2);
    view.unmount();
    expect(commands.unsubscribeLiveInput).toHaveBeenLastCalledWith(2);
  });

  it("o StrictMode (monta, desmonta e monta de novo) termina com uma assinatura ativa", () => {
    const view = renderHook(() => useLiveReport(3), { wrapper: StrictMode });
    const subscribes = commands.subscribeLiveInput.mock.calls.length;
    const unsubscribes = commands.unsubscribeLiveInput.mock.calls.length;
    expect(subscribes - unsubscribes).toBe(1);
    view.unmount();
    expect(commands.unsubscribeLiveInput).toHaveBeenCalledTimes(subscribes);
  });

  it("uma falha do backend ao assinar não quebra a tela", async () => {
    commands.subscribeLiveInput.mockRejectedValue({ code: "invalid_input", message_key: "x" });
    const view = renderHook(() => useLiveReport(0));
    await Promise.resolve();
    expect(view.result.current).toBeUndefined();
  });
});
