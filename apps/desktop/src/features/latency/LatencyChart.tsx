import { useEffect, useRef } from "react";
import uPlot from "uplot";
import "uplot/dist/uPlot.min.css";
import { useTranslation } from "react-i18next";
import { history, SLOT_COUNT, useHistoryVersion } from "@/lib/api/latencyHistory";

export type WindowSeconds = 30 | 120 | 600;

interface LatencyChartProps {
  /** Quanto tempo o eixo X mostra, terminando na amostra mais recente. */
  windowSec: WindowSeconds;
  /** Congelado: o gráfico para de seguir os dados (o histórico continua crescendo). */
  frozen: boolean;
  /** Slots escondidos pela legenda. */
  hidden: ReadonlySet<number>;
}

const SLOT_VARS = ["--slot-1", "--slot-2", "--slot-3", "--slot-4"] as const;

const css = (name: string) =>
  getComputedStyle(document.documentElement).getPropertyValue(name).trim() || "gray";

function data(): uPlot.AlignedData {
  return [history.times, ...history.p50, ...history.p95];
}

/** Índice da série no uPlot: 0 é o eixo X, depois os p50 dos 4 slots, depois os p95. */
const seriesIndex = (slot: number, kind: "p50" | "p95") =>
  1 + slot + (kind === "p95" ? SLOT_COUNT : 0);

function options(
  width: number,
  height: number,
  view: { windowSec: number },
  labels: { p50: string; p95: string; slot: (n: number) => string },
): uPlot.Options {
  const slotSeries = (kind: "p50" | "p95") =>
    Array.from({ length: SLOT_COUNT }, (_, slot): uPlot.Series => ({
      label: `${labels.slot(slot + 1)} ${kind === "p50" ? labels.p50 : labels.p95}`,
      stroke: () => css(SLOT_VARS[slot] ?? "--accent"),
      width: kind === "p50" ? 2 : 1.25,
      dash: kind === "p95" ? [5, 4] : undefined,
      points: { show: false },
      spanGaps: false,
    }));
  const axis = (): uPlot.Axis => ({
    stroke: () => css("--text-secondary"),
    grid: { stroke: () => css("--separator"), width: 1 },
    ticks: { stroke: () => css("--separator"), width: 1 },
    font: "11px Inter Variable, system-ui, sans-serif",
  });
  return {
    width,
    height,
    padding: [10, 12, 0, 0],
    legend: { show: false },
    cursor: { drag: { x: false, y: false }, points: { show: false } },
    scales: {
      x: {
        time: true,
        range: (u) => {
          const end = u.data[0]?.[u.data[0].length - 1] ?? Date.now() / 1000;
          return [end - view.windowSec, end];
        },
      },
      y: { range: (_u, _min, max) => [0, Math.max(20, Math.ceil((max || 0) * 1.25))] },
    },
    axes: [
      axis(),
      { ...axis(), size: 52, values: (_u, ticks) => ticks.map((v) => `${String(v)} ms`) },
    ],
    series: [{}, ...slotSeries("p50"), ...slotSeries("p95")],
  };
}

/** RTT p50 (linha cheia) e p95 (tracejada) de cada controle, nas cores dos slots. */
export function LatencyChart({ windowSec, frozen, hidden }: LatencyChartProps) {
  const { t } = useTranslation("latency");
  const host = useRef<HTMLDivElement>(null);
  const chart = useRef<uPlot | null>(null);
  const view = useRef({ windowSec });
  const frozenRef = useRef(frozen);

  useEffect(() => {
    const element = host.current;
    if (!element) return;
    const plot = new uPlot(
      options(element.clientWidth || 600, element.clientHeight || 280, view.current, {
        p50: t("chart.p50"),
        p95: t("chart.p95"),
        slot: (n) => `P${String(n)}`,
      }),
      data(),
      element,
    );
    chart.current = plot;

    // Segue o histórico sem re-renderizar o React (4 Hz).
    const unsubscribe = useHistoryVersion.subscribe(() => {
      if (!frozenRef.current) plot.setData(data(), true);
    });
    const resize = new ResizeObserver(() => {
      plot.setSize({ width: element.clientWidth, height: element.clientHeight });
    });
    resize.observe(element);
    // Trocar o tema muda as cores dos tokens: repinta o canvas.
    const theme = new MutationObserver(() => {
      plot.redraw(false);
    });
    theme.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });

    return () => {
      unsubscribe();
      resize.disconnect();
      theme.disconnect();
      plot.destroy();
      chart.current = null;
    };
    // `t` só entra nos rótulos das séries (a legenda do app é a nossa).
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    view.current.windowSec = windowSec;
    chart.current?.setData(data(), true);
  }, [windowSec]);

  useEffect(() => {
    frozenRef.current = frozen;
    // Ao descongelar, volta a acompanhar os dados de uma vez.
    if (!frozen) chart.current?.setData(data(), true);
  }, [frozen]);

  useEffect(() => {
    const plot = chart.current;
    if (!plot) return;
    for (let slot = 0; slot < SLOT_COUNT; slot++) {
      const show = !hidden.has(slot);
      plot.setSeries(seriesIndex(slot, "p50"), { show }, false);
      plot.setSeries(seriesIndex(slot, "p95"), { show }, false);
    }
  }, [hidden]);

  return (
    <div
      ref={host}
      role="img"
      aria-label={t("chart.label")}
      className="h-72 w-full min-w-0 [&_.u-legend]:hidden"
    />
  );
}
