import { create } from "zustand";
import type { MetricsDto } from "@/lib/bindings";

/** Quanto tempo o histórico guarda (a maior janela do gráfico). */
export const HISTORY_SECONDS = 600;
/** Só apara quando há pelo menos este tanto de amostras vencidas (10 s a 4 Hz). */
const TRIM_BATCH = 40;
export const SLOT_COUNT = 4;

/** Uma série por slot; `null` onde o slot não tinha sessão ou ainda não havia medida. */
export type Series = (number | null)[];

export interface LatencyHistory {
  /** Instante de cada amostra, em segundos desde a época Unix (o eixo X do uPlot). */
  times: number[];
  /** RTT p50 de cada slot (ms), na janela de 60 s do backend. */
  p50: Series[];
  /** RTT p95 de cada slot (ms). */
  p95: Series[];
}

const empty = (): LatencyHistory => ({
  times: [],
  p50: Array.from({ length: SLOT_COUNT }, () => []),
  p95: Array.from({ length: SLOT_COUNT }, () => []),
});

/** Os arrays mudam no lugar (4 amostras por segundo × 8 séries); `version` avisa quem desenha. */
export const history: LatencyHistory = empty();

interface VersionState {
  version: number;
  bump: () => void;
}

export const useHistoryVersion = create<VersionState>((set) => ({
  version: 0,
  bump: () => {
    set((s) => ({ version: s.version + 1 }));
  },
}));

/** Acrescenta um `metrics:tick` ao histórico e apara o que passou de 10 minutos. */
export function appendMetrics(metrics: MetricsDto, nowMs: number = Date.now()): void {
  const now = nowMs / 1000;
  history.times.push(now);
  for (let slot = 0; slot < SLOT_COUNT; slot++) {
    const m = metrics.slots.find((s) => s.slot === slot);
    // Sem medida de RTT ainda: o gráfico não desenha nada, em vez de um zero enganoso.
    const hasRtt = m !== undefined && m.rtt_ms !== null;
    history.p50[slot]?.push(hasRtt ? m.rtt_percentiles_ms.p50 : null);
    history.p95[slot]?.push(hasRtt ? m.rtt_percentiles_ms.p95 : null);
  }
  // Apara em lotes: só quando já passou bastante da janela, para não deslocar os arrays a cada tick.
  const limit = now - HISTORY_SECONDS;
  let cut = 0;
  while (cut < history.times.length && (history.times[cut] ?? now) < limit) cut++;
  if (cut >= TRIM_BATCH) {
    history.times.splice(0, cut);
    for (const series of [...history.p50, ...history.p95]) series.splice(0, cut);
  }
  useHistoryVersion.getState().bump();
}

/** Esvazia o histórico (testes e troca de sessão do app). */
export function clearHistory(): void {
  Object.assign(history, empty());
  useHistoryVersion.getState().bump();
}
