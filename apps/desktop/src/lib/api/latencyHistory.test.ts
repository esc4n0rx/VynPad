import { beforeEach, describe, expect, it } from "vitest";
import type { MetricsDto, SlotMetricsDto } from "@/lib/bindings";
import { appendMetrics, clearHistory, history, HISTORY_SECONDS } from "./latencyHistory";

const slotMetrics = (
  slot: number,
  p50: number,
  p95: number,
  rtt: number | null = p50,
): SlotMetricsDto => ({
  slot,
  session_id: 1,
  rtt_ms: rtt,
  rtt_percentiles_ms: { p50, p95, p99: p95 },
  jitter_ms: 0,
  loss_pct: 0,
  out_of_order: 0,
  rate_hz: 250,
  processing_percentiles_ms: { p50: 0, p95: 0, p99: 0 },
  battery: null,
  quality: null,
});

const tick = (...slots: SlotMetricsDto[]): MetricsDto => ({ slots });

beforeEach(clearHistory);

describe("histórico de latência", () => {
  it("guarda p50 e p95 de cada slot e deixa nulo quem não tem sessão", () => {
    appendMetrics(tick(slotMetrics(0, 10, 14), slotMetrics(2, 30, 45)), 1_000_000);
    appendMetrics(tick(slotMetrics(0, 11, 15)), 1_000_250);
    expect(history.times).toEqual([1000, 1000.25]);
    expect(history.p50[0]).toEqual([10, 11]);
    expect(history.p95[0]).toEqual([14, 15]);
    expect(history.p50[2]).toEqual([30, null]);
    expect(history.p50[1]).toEqual([null, null]);
    expect(history.p50[3]).toEqual([null, null]);
  });

  it("sem RTT medido ainda, não desenha nada em vez de um zero", () => {
    appendMetrics(tick(slotMetrics(0, 0, 0, null)), 5_000);
    expect(history.p50[0]).toEqual([null]);
    expect(history.p95[0]).toEqual([null]);
  });

  it("apara o que passou de 10 minutos, em lotes, sem desalinhar as séries", () => {
    const start = 1_000_000;
    // 11 minutos a 4 Hz.
    for (let i = 0; i < 11 * 60 * 4; i++) {
      appendMetrics(tick(slotMetrics(0, i, i)), start + i * 250);
    }
    const span = (history.times.at(-1) ?? 0) - (history.times[0] ?? 0);
    expect(span).toBeLessThanOrEqual(HISTORY_SECONDS + 11);
    expect(span).toBeGreaterThanOrEqual(HISTORY_SECONDS - 1);
    for (const series of [...history.p50, ...history.p95]) {
      expect(series).toHaveLength(history.times.length);
    }
    // O primeiro valor guardado é o que corresponde ao primeiro instante guardado.
    const firstIndex = Math.round(((history.times[0] ?? 0) * 1000 - start) / 250);
    expect(history.p50[0]?.[0]).toBe(firstIndex);
  });
});
