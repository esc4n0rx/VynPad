import { describe, expect, it } from "vitest";
import type { SlotMetricsDto } from "@/lib/bindings";
import { summarize, tipsFor } from "./insights";

const metrics = (patch: Partial<SlotMetricsDto> = {}): SlotMetricsDto => ({
  slot: 0,
  session_id: 1,
  rtt_ms: 20,
  rtt_percentiles_ms: { p50: 18, p95: 25, p99: 30 },
  jitter_ms: 2,
  loss_pct: 0.5,
  out_of_order: 0,
  rate_hz: 250,
  processing_percentiles_ms: { p50: 0.02, p95: 0.05, p99: 0.1 },
  battery: null,
  quality: "good",
  ...patch,
});

describe("tipsFor", () => {
  it("conexão boa não tem dicas", () => {
    expect(tipsFor(metrics())).toEqual([]);
  });

  it("perda acima de 2% sugere chegar perto do roteador", () => {
    expect(tipsFor(metrics({ loss_pct: 2 }))).toEqual([]);
    expect(tipsFor(metrics({ loss_pct: 2.1 }))).toEqual(["loss"]);
  });

  it("jitter e RTT altos geram as suas dicas, na ordem de urgência", () => {
    expect(tipsFor(metrics({ jitter_ms: 9 }))).toEqual(["jitter"]);
    expect(
      tipsFor(
        metrics({
          loss_pct: 5,
          jitter_ms: 20,
          rtt_percentiles_ms: { p50: 70, p95: 90, p99: 120 },
        }),
      ),
    ).toEqual(["loss", "jitter", "rtt"]);
  });

  it("valores ausentes (NaN que virou null) não geram dica", () => {
    expect(tipsFor(metrics({ loss_pct: null, jitter_ms: null }))).toEqual([]);
  });
});

describe("summarize", () => {
  const labels = {
    slot: (n: number) => `P${String(n + 1)}`,
    quality: (m: SlotMetricsDto) => m.quality ?? "?",
    line: (v: Record<string, string>) =>
      `rtt=${v.rtt ?? ""} p95=${v.p95 ?? ""} jitter=${v.jitter ?? ""} loss=${v.loss ?? ""} rate=${v.rate ?? ""} proc=${v.processing ?? ""}`,
  };

  it("uma linha por slot, em ordem, com os números formatados", () => {
    const text = summarize([metrics({ slot: 2 }), metrics({ slot: 0 })], labels, "en-US");
    expect(text).toBe(
      [
        "P1: rtt=20.0 p95=25.0 jitter=2.0 loss=0.5 rate=250 proc=0.05: good",
        "P3: rtt=20.0 p95=25.0 jitter=2.0 loss=0.5 rate=250 proc=0.05: good",
      ].join("\n"),
    );
  });

  it("sem RTT ainda, mostra um traço", () => {
    expect(summarize([metrics({ rtt_ms: null })], labels, "en-US")).toContain("rtt=—");
  });
});
