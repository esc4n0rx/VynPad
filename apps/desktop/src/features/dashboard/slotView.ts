import type { SessionHealthDto, SlotDto, XInputSlotDto } from "@/lib/bindings";

/** Como o card de um slot se apresenta. */
export type SlotCardKind =
  /** Livre e sem nada no índice XInput. */
  | "empty"
  /** Um controle físico está no índice do slot (T-205): o card só informa. */
  | "physical"
  /** Um celular está conectado. */
  | "connected"
  /** Sem sinal (5 s) ou conexão caída com a reserva correndo. */
  | "reconnecting";

/** Decide o card a partir do slot, do estado de saúde do input e da ocupação do XInput. */
export function slotCardKind(
  slot: SlotDto,
  health: SessionHealthDto | undefined,
  xinput: readonly XInputSlotDto[] | undefined,
): SlotCardKind {
  if (slot.state === "suspended" || (slot.state !== "free" && health === "stalled")) {
    return "reconnecting";
  }
  if (slot.state === "active" || slot.state === "reserved") return "connected";
  const occupant = xinput?.find((x) => x.index === slot.slot)?.occupant;
  return occupant === "physical" ? "physical" : "empty";
}

/** Segundos inteiros que faltam, arredondando para cima (nunca negativo). */
export function secondsLeft(deadlineMs: number, nowMs: number): number {
  return Math.max(0, Math.ceil((deadlineMs - nowMs) / 1000));
}
