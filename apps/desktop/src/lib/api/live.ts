import { useEffect } from "react";
import { create } from "zustand";
import { commands, type InputReportDto } from "@/lib/bindings";
import { log } from "@/lib/log";

interface LiveState {
  /** Último estado de cada controle assinado. */
  reports: Partial<Record<number, InputReportDto>>;
  set: (slot: number, report: InputReportDto) => void;
  clear: (slot: number) => void;
}

/** Estado ao vivo dos controles (`input:live`), só dos slots que alguma tela assina. */
export const useLiveInput = create<LiveState>((set) => ({
  reports: {},
  set: (slot, report) => {
    set((s) => ({ reports: { ...s.reports, [slot]: report } }));
  },
  clear: (slot) => {
    set((s) => {
      const reports = { ...s.reports };
      delete reports[slot];
      return { reports };
    });
  },
}));

/** Quantos componentes desta janela assinam cada slot. O backend recebe só 0→1 e 1→0. */
const holders = new Map<number, number>();

function acquire(slot: number): void {
  const count = (holders.get(slot) ?? 0) + 1;
  holders.set(slot, count);
  if (count === 1) {
    commands.subscribeLiveInput(slot).catch((error: unknown) => {
      log.warn(`não consegui assinar o input ao vivo do slot ${slot}: ${JSON.stringify(error)}`);
    });
  }
}

function release(slot: number): void {
  const count = (holders.get(slot) ?? 0) - 1;
  if (count > 0) {
    holders.set(slot, count);
    return;
  }
  holders.delete(slot);
  useLiveInput.getState().clear(slot);
  commands.unsubscribeLiveInput(slot).catch((error: unknown) => {
    log.warn(`não consegui cancelar o input ao vivo do slot ${slot}: ${JSON.stringify(error)}`);
  });
}

/** Esquece as assinaturas locais (ao recarregar a janela; o backend é zerado por `reset`). */
export function resetLiveSubscriptions(): void {
  holders.clear();
  useLiveInput.setState({ reports: {} });
}

/**
 * Mantém o backend emitindo o input do `slot` enquanto o componente estiver montado, sem
 * re-renderizar a cada relatório (quem desenha lê com `getLiveReport` num laço de animação).
 */
export function useLiveSubscription(slot: number | null): void {
  useEffect(() => {
    if (slot === null) return;
    acquire(slot);
    return () => {
      release(slot);
    };
  }, [slot]);
}

/** O último relatório do `slot`, lido do store sem assinar (não re-renderiza ninguém). */
export function getLiveReport(slot: number): InputReportDto | undefined {
  return useLiveInput.getState().reports[slot];
}

/**
 * O estado ao vivo do controle do `slot`, enquanto o componente estiver montado. Sem nenhum
 * componente usando, o backend não amostra nem emite nada. Re-renderiza a cada relatório: para
 * desenhar a 60 Hz use `useLiveSubscription` + `getLiveReport` (como o `Gamepad`).
 */
export function useLiveReport(slot: number | null): InputReportDto | undefined {
  useLiveSubscription(slot);
  return useLiveInput((s) => (slot === null ? undefined : s.reports[slot]));
}
