import { useEffect } from "react";
import { create } from "zustand";
import { commands, type MetricsDto, type SlotMetricsDto } from "@/lib/bindings";
import { log } from "@/lib/log";

interface MetricsState {
  /** Métricas de cada slot com sessão (`metrics:tick`); o slot sem sessão não aparece. */
  bySlot: Partial<Record<number, SlotMetricsDto>>;
  set: (metrics: MetricsDto) => void;
  clear: () => void;
}

/** Latência, jitter, perda, taxa e bateria de cada slot, a 4 Hz com a janela à vista. */
export const useMetrics = create<MetricsState>((set) => ({
  bySlot: {},
  set: (metrics) => {
    set({ bySlot: Object.fromEntries(metrics.slots.map((m) => [m.slot, m])) });
  },
  clear: () => {
    set({ bySlot: {} });
  },
}));

/** As métricas do `slot` (`undefined` sem sessão ou antes do primeiro tick). */
export function useSlotMetrics(slot: number): SlotMetricsDto | undefined {
  return useMetrics((s) => s.bySlot[slot]);
}

/**
 * Avisa o backend quando a janela some ou volta (minimizada, em outra aba, atrás de um jogo em
 * tela cheia): escondida, as métricas saem a 0,2 Hz, só para a bandeja.
 */
export function useReportVisibility(): void {
  useEffect(() => {
    const report = () => {
      commands.setUiVisible(document.visibilityState === "visible").catch((error: unknown) => {
        log.warn(`não consegui informar a visibilidade da janela: ${JSON.stringify(error)}`);
      });
    };
    document.addEventListener("visibilitychange", report);
    report();
    return () => {
      document.removeEventListener("visibilitychange", report);
    };
  }, []);
}
