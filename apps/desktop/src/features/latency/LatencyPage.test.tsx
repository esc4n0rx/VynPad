import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { appendMetrics, clearHistory, history } from "@/lib/api/latencyHistory";
import { useMetrics } from "@/lib/api/metrics";
import type { SlotMetricsDto } from "@/lib/bindings";
import { LatencyPage } from "./LatencyPage";

// O uPlot desenha em canvas, que o jsdom não tem: o dublê guarda as chamadas.
const plots = vi.hoisted(() => [] as FakePlot[]);
interface FakePlot {
  setData: ReturnType<typeof vi.fn>;
  setSeries: ReturnType<typeof vi.fn>;
  setSize: ReturnType<typeof vi.fn>;
  redraw: ReturnType<typeof vi.fn>;
  destroy: ReturnType<typeof vi.fn>;
}
vi.mock("uplot", () => ({
  default: class {
    setData = vi.fn();
    setSeries = vi.fn();
    setSize = vi.fn();
    redraw = vi.fn();
    destroy = vi.fn();
    constructor() {
      plots.push(this);
    }
  },
}));
vi.mock("uplot/dist/uPlot.min.css", () => ({}));

const metric = (slot: number, patch: Partial<SlotMetricsDto> = {}): SlotMetricsDto => ({
  slot,
  session_id: 10 + slot,
  rtt_ms: 21.4,
  rtt_percentiles_ms: { p50: 20, p95: 25, p99: 30 },
  jitter_ms: 2,
  loss_pct: 0,
  out_of_order: 0,
  rate_hz: 250,
  processing_percentiles_ms: { p50: 0.02, p95: 0.05, p99: 0.1 },
  battery: 80,
  quality: "excellent",
  ...patch,
});

function tick(...slots: SlotMetricsDto[]) {
  act(() => {
    useMetrics.getState().set({ slots });
    appendMetrics({ slots });
  });
}

beforeEach(() => {
  plots.length = 0;
  clearHistory();
  useMetrics.setState({ bySlot: {} });
  vi.stubGlobal(
    "ResizeObserver",
    class {
      observe = vi.fn();
      disconnect = vi.fn();
    },
  );
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe("monitor de latência", () => {
  it("sem controles mostra o estado vazio e não deixa copiar", () => {
    render(<LatencyPage />);
    expect(screen.getByText("Nenhum controle conectado")).toBeTruthy();
    expect(screen.getByRole<HTMLButtonElement>("button", { name: "Copiar resumo" }).disabled).toBe(
      true,
    );
  });

  it("mostra os tiles de cada controle conectado, com a qualidade", () => {
    render(<LatencyPage />);
    tick(metric(0), metric(2, { quality: "unstable", loss_pct: 1.5, jitter_ms: 4.25 }));
    const p1 = within(screen.getByRole("group", { name: "Controle P1" }));
    expect(p1.getByText("25,0 ms")).toBeTruthy(); // RTT p95
    expect(p1.getByText("250/s")).toBeTruthy();
    expect(p1.getByText("0,05 ms")).toBeTruthy(); // processamento no server
    expect(p1.getByText("Excelente")).toBeTruthy();
    const p3 = within(screen.getByRole("group", { name: "Controle P3" }));
    expect(p3.getByText("Instável")).toBeTruthy();
    expect(p3.getByText("1,5%")).toBeTruthy();
    expect(screen.queryByRole("group", { name: "Controle P2" })).toBeNull();
  });

  it("sem medida de RTT ainda, o tile mostra traço e a qualidade fica em medição", () => {
    render(<LatencyPage />);
    tick(metric(0, { rtt_ms: null, quality: null }));
    const p1 = within(screen.getByRole("group", { name: "Controle P1" }));
    expect(p1.getByText("—")).toBeTruthy();
    expect(p1.getByText("Medindo…")).toBeTruthy();
  });

  it("dá dicas quando há perda ou jitter altos", () => {
    render(<LatencyPage />);
    tick(metric(0, { loss_pct: 3.2 }), metric(1, { jitter_ms: 12 }));
    expect(screen.getByText(/P1: aproxime-se do roteador/)).toBeTruthy();
    expect(screen.getByText(/P2: desative a economia de bateria do Wi-Fi/)).toBeTruthy();
    expect(screen.queryByText(/P1: desative/)).toBeNull();
  });

  it("não mostra dicas com a conexão boa", () => {
    render(<LatencyPage />);
    tick(metric(0));
    expect(screen.queryByText("Dicas")).toBeNull();
  });

  it("o gráfico segue o histórico e a janela pode ser trocada", () => {
    render(<LatencyPage />);
    expect(plots).toHaveLength(1);
    const plot = plots[0];
    plot.setData.mockClear();
    tick(metric(0));
    expect(plot.setData).toHaveBeenCalledTimes(1);
    expect(history.p95[0]).toEqual([25]);

    plot.setData.mockClear();
    fireEvent.click(screen.getByRole("radio", { name: "30 s" }));
    expect(plot.setData).toHaveBeenCalled();
  });

  it("pausar congela o gráfico e retomar volta a seguir os dados", () => {
    render(<LatencyPage />);
    const plot = plots[0];
    tick(metric(0));
    fireEvent.click(screen.getByRole("button", { name: "Pausar gráfico" }));
    expect(screen.getByText("Gráfico pausado")).toBeTruthy();

    plot.setData.mockClear();
    tick(metric(0));
    tick(metric(0));
    expect(plot.setData).not.toHaveBeenCalled();
    // O histórico continuou crescendo por baixo.
    expect(history.times).toHaveLength(3);

    fireEvent.click(screen.getByRole("button", { name: "Retomar gráfico" }));
    expect(plot.setData).toHaveBeenCalled();
  });

  it("a legenda esconde e mostra as duas linhas do slot", () => {
    render(<LatencyPage />);
    const plot = plots[0];
    const toggle = screen.getByRole("button", { name: "Mostrar ou esconder o P2" });
    expect(toggle.getAttribute("aria-pressed")).toBe("true");

    plot.setSeries.mockClear();
    fireEvent.click(toggle);
    expect(toggle.getAttribute("aria-pressed")).toBe("false");
    // As séries do P2: p50 no índice 2 e p95 no 6 (0 é o eixo X).
    expect(plot.setSeries).toHaveBeenCalledWith(2, { show: false }, false);
    expect(plot.setSeries).toHaveBeenCalledWith(6, { show: false }, false);

    plot.setSeries.mockClear();
    fireEvent.click(toggle);
    expect(plot.setSeries).toHaveBeenCalledWith(2, { show: true }, false);
    expect(plot.setSeries).toHaveBeenCalledWith(6, { show: true }, false);
  });

  it("copia o resumo em texto", async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    vi.stubGlobal("navigator", { clipboard: { writeText } });
    render(<LatencyPage />);
    tick(metric(0), metric(1, { quality: "good", rtt_ms: 38.2 }));
    fireEvent.click(screen.getByRole("button", { name: "Copiar resumo" }));
    await waitFor(() => {
      expect(writeText).toHaveBeenCalledTimes(1);
    });
    const text = writeText.mock.calls[0]?.[0] as string;
    expect(text.split("\n")).toHaveLength(2);
    expect(text).toContain(
      "P1: RTT 21,4 ms (p95 25,0 ms), jitter 2,0 ms, perda 0,0%, 250 pacotes/s",
    );
    expect(text).toContain("P2: RTT 38,2 ms");
    expect(text).toContain("Boa");
    expect(await screen.findByRole("button", { name: "Resumo copiado" })).toBeTruthy();
  });

  it("destrói o gráfico ao sair da tela", () => {
    const view = render(<LatencyPage />);
    const plot = plots[0];
    view.unmount();
    expect(plot.destroy).toHaveBeenCalledTimes(1);
  });
});
