import type { SlotMetricsDto } from "@/lib/bindings";

/** Perda acima disto (em %) sugere sinal fraco. */
export const LOSS_TIP_PCT = 2;
/** Jitter acima disto (em ms) sugere economia de energia do Wi-Fi no celular. */
export const JITTER_TIP_MS = 8;
/** RTT p95 acima disto (em ms) sugere rede congestionada. */
export const RTT_TIP_MS = 60;

export type TipKey = "loss" | "jitter" | "rtt";

/** Dicas para a conexão de um slot, da mais urgente para a menos. */
export function tipsFor(metrics: SlotMetricsDto): TipKey[] {
  const tips: TipKey[] = [];
  if ((metrics.loss_pct ?? 0) > LOSS_TIP_PCT) tips.push("loss");
  if ((metrics.jitter_ms ?? 0) > JITTER_TIP_MS) tips.push("jitter");
  if ((metrics.rtt_percentiles_ms.p95 ?? 0) > RTT_TIP_MS) tips.push("rtt");
  return tips;
}

export interface SummaryLabels {
  slot: (slot: number) => string;
  quality: (metrics: SlotMetricsDto) => string;
  line: (values: {
    rtt: string;
    p95: string;
    jitter: string;
    loss: string;
    rate: string;
    processing: string;
  }) => string;
}

const fixed = (value: number | null, digits: number, locale?: string) =>
  (value ?? 0).toLocaleString(locale, {
    minimumFractionDigits: digits,
    maximumFractionDigits: digits,
  });

/** O resumo em texto (uma linha por slot) que o botão "Copiar" põe na área de transferência. */
export function summarize(
  slots: readonly SlotMetricsDto[],
  labels: SummaryLabels,
  locale?: string,
): string {
  return [...slots]
    .sort((a, b) => a.slot - b.slot)
    .map((m) =>
      [
        labels.slot(m.slot),
        labels.line({
          rtt: m.rtt_ms === null ? "—" : fixed(m.rtt_ms, 1, locale),
          p95: fixed(m.rtt_percentiles_ms.p95, 1, locale),
          jitter: fixed(m.jitter_ms, 1, locale),
          loss: fixed(m.loss_pct, 1, locale),
          rate: fixed(m.rate_hz, 0, locale),
          processing: fixed(m.processing_percentiles_ms.p95, 2, locale),
        }),
        labels.quality(m),
      ].join(": "),
    )
    .join("\n");
}
