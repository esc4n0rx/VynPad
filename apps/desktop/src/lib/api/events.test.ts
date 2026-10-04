import { QueryClient } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { SlotsDto } from "@/lib/bindings";
import { commands, emit, listenerCount, resetBindingsMock, stopped } from "@/test/bindings-mock";
import { registerEventListeners } from "./events";
import { keys } from "./keys";
import { useLiveInput } from "./live";
import { useMetrics } from "./metrics";
import { useToasts } from "@/lib/toast";
import { useApprovals } from "./server";
import { useSlots } from "./slots";

vi.mock("@/lib/bindings", async () => (await import("@/test/bindings-mock")).bindingsMock);

const EVENT_COUNT = 19;

const slots: SlotsDto = {
  free_count: 3,
  slots: [
    {
      slot: 0,
      state: "active",
      expires_in_ms: null,
      device_id: "d1",
      session: 7,
      paused: false,
      user_index: 0,
    },
    ...[1, 2, 3].map((slot) => ({
      slot,
      state: "free" as const,
      expires_in_ms: null,
      device_id: null,
      session: null,
      paused: false,
      user_index: null,
    })),
  ],
};

let queryClient: QueryClient;
let stop: () => void;

beforeEach(async () => {
  resetBindingsMock();
  commands.getSlots.mockResolvedValue(slots);
  commands.getMetrics.mockResolvedValue({ slots: [] });
  useMetrics.setState({ bySlot: {} });
  useSlots.setState({ slots: null, health: {} });
  useLiveInput.setState({ reports: {} });
  useApprovals.setState({ pending: [] });
  queryClient = new QueryClient();
  stop = await registerEventListeners(queryClient);
});

afterEach(() => {
  stop();
});

describe("registerEventListeners", () => {
  it("liga todos os eventos uma única vez, mesmo chamado de novo (StrictMode)", async () => {
    expect(listenerCount()).toBe(EVENT_COUNT);
    const again = await registerEventListeners(queryClient);
    expect(again).toBe(stop);
    expect(listenerCount()).toBe(EVENT_COUNT);
  });

  it("zera o input ao vivo e lê os slots para a tela nascer completa", () => {
    expect(commands.resetLiveInput).toHaveBeenCalledTimes(1);
    expect(commands.getSlots).toHaveBeenCalledTimes(1);
    expect(useSlots.getState().slots).toEqual(slots);
  });

  it("o tick de métricas substitui as métricas de cada slot com sessão", () => {
    const metric = (slot: number) => ({
      slot,
      session_id: 7 + slot,
      rtt_ms: 12.5,
      rtt_percentiles_ms: { p50: 12, p95: 15, p99: 20 },
      jitter_ms: 1,
      loss_pct: 0,
      out_of_order: 0,
      rate_hz: 250,
      processing_percentiles_ms: { p50: 0.02, p95: 0.05, p99: 0.1 },
      battery: 80,
      quality: "excellent",
    });
    emit("metricsTick", { metrics: { slots: [metric(0), metric(2)] } });
    expect(Object.keys(useMetrics.getState().bySlot)).toEqual(["0", "2"]);
    expect(useMetrics.getState().bySlot[2]?.rtt_ms).toBe(12.5);
    // O slot que saiu do tick some do store.
    emit("metricsTick", { metrics: { slots: [metric(2)] } });
    expect(useMetrics.getState().bySlot[0]).toBeUndefined();
  });

  it("o aviso da bandeja vira um toast", () => {
    useToasts.setState({ toasts: [] });
    emit("trayHint");
    expect(useToasts.getState().toasts.at(-1)?.message).toBe(
      "O VynPad continua rodando na bandeja.",
    );
  });

  it("os avisos do backend viram toasts com o tom certo", () => {
    useToasts.setState({ toasts: [] });
    emit("noticeEvent", { level: "success", message: "Galaxy conectado como P1" });
    emit("noticeEvent", { level: "error", message: "O driver parou" });
    const [first, second] = useToasts.getState().toasts;
    expect(first).toMatchObject({ kind: "success", message: "Galaxy conectado como P1" });
    expect(second).toMatchObject({ kind: "error", message: "O driver parou" });
  });

  it("uma atualização disponível entra no cache e avisa com um toast", () => {
    useToasts.setState({ toasts: [] });
    const update = { version: "0.2.0", current_version: "0.1.0", notes: null, date: null };
    emit("updateAvailable", { update });
    expect(queryClient.getQueryData(keys.pendingUpdate)).toEqual(update);
    expect(useToasts.getState().toasts[0]).toMatchObject({
      kind: "info",
      message: "Nova versão 0.2.0 disponível. Veja em Configurações → Sobre.",
    });
  });

  it("desfaz tudo e permite registrar de novo", async () => {
    stop();
    expect(stopped.size).toBe(EVENT_COUNT);
    expect(listenerCount()).toBe(0);
    stop = await registerEventListeners(queryClient);
    expect(listenerCount()).toBe(EVENT_COUNT);
  });
});

describe("eventos do backend", () => {
  it("settingsChanged atualiza o cache das configurações", () => {
    emit("settingsChanged", { settings: { theme: "light" } });
    expect(queryClient.getQueryData(keys.settings)).toEqual({ theme: "light" });
  });

  it("slotsChanged atualiza o store e o slot que ficou livre perde o estado de saúde", () => {
    useSlots.getState().setHealth(1, "stalled");
    useSlots.getState().setHealth(0, "neutral");
    emit("slotsChanged", { state: slots });
    expect(useSlots.getState().slots).toEqual(slots);
    expect(useSlots.getState().health).toEqual({ 0: "neutral" });
  });

  it("sessionState e sessionEnded mantêm a saúde do slot", () => {
    emit("sessionState", { session_id: 7, slot: 0, health: "stalled" });
    expect(useSlots.getState().health[0]).toBe("stalled");
    emit("sessionEnded", { session_id: 7, slot: 0, device_id: "d1", reason: "dropped" });
    expect(useSlots.getState().health[0]).toBeUndefined();
  });

  it("mudanças de dispositivos e de sessão invalidam as consultas certas", () => {
    const invalidate = vi.spyOn(queryClient, "invalidateQueries");
    emit("devicesChanged");
    expect(invalidate).toHaveBeenLastCalledWith({ queryKey: keys.devices });
    emit("devicesPaired", { device_id: "d1", name: "Pixel", model: "8" });
    expect(invalidate).toHaveBeenCalledWith({ queryKey: keys.connectionLog });
    emit("connectionRejected", { peer: "1.2.3.4", device_id: null, reason: "bad_auth" });
    expect(invalidate).toHaveBeenLastCalledWith({ queryKey: keys.connectionLog });
    emit("sessionStarted", { session_id: 1, slot: 0, device_id: "d1", peer: "1.2.3.4" });
    expect(invalidate).toHaveBeenLastCalledWith({ queryKey: keys.connectionLog });
    emit("reservationExpired", { slot: 0 });
    expect(invalidate).toHaveBeenLastCalledWith({ queryKey: keys.devices });
  });

  it("devicesDiscovered e driverChanged escrevem direto no cache", () => {
    const devices = [
      { id: "x", name: "Pixel", model: "8", ip: "1.2.3.4", port: 1, seen_ago_ms: 0, paired: false },
    ];
    emit("devicesDiscovered", { devices });
    expect(queryClient.getQueryData(keys.nearby)).toEqual(devices);
    emit("driverChanged", { status: { state: "service_stopped" } });
    expect(queryClient.getQueryData(keys.driver)).toEqual({ state: "service_stopped" });
  });

  it("os pedidos de confirmação entram e saem da lista", () => {
    const prompt = { request_id: 3, device_id: "d", name: "Pixel", model: "8", peer: "1.2.3.4" };
    emit("approvalRequested", { prompt });
    expect(useApprovals.getState().pending).toEqual([prompt]);
    // O mesmo pedido de novo não duplica.
    emit("approvalRequested", { prompt });
    expect(useApprovals.getState().pending).toHaveLength(1);
    emit("approvalResolved", { request_id: 3, accepted: true });
    expect(useApprovals.getState().pending).toEqual([]);
  });

  it("inputLive guarda o último estado de cada slot", () => {
    const report = { buttons: 1, lt: 0, rt: 0, lx: 10, ly: 0, rx: 0, ry: 0 };
    emit("inputLive", { slot: 2, report });
    expect(useLiveInput.getState().reports[2]).toEqual(report);
  });
});
