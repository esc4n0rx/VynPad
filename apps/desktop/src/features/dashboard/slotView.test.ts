import { describe, expect, it } from "vitest";
import type { SlotDto, XInputSlotDto } from "@/lib/bindings";
import { secondsLeft, slotCardKind } from "./slotView";

const slot = (patch: Partial<SlotDto>): SlotDto => ({
  slot: 0,
  state: "free",
  expires_in_ms: null,
  device_id: null,
  session: null,
  paused: false,
  user_index: null,
  ...patch,
});

describe("slotCardKind", () => {
  it("livre e sem nada no XInput é vazio", () => {
    expect(slotCardKind(slot({}), undefined, undefined)).toBe("empty");
    const free: XInputSlotDto[] = [{ index: 0, occupant: "free" }];
    expect(slotCardKind(slot({}), undefined, free)).toBe("empty");
  });

  it("controle físico no índice do slot vira o card informativo", () => {
    const xinput: XInputSlotDto[] = [
      { index: 0, occupant: "virtual" },
      { index: 1, occupant: "physical" },
    ];
    expect(slotCardKind(slot({ slot: 1 }), undefined, xinput)).toBe("physical");
    expect(slotCardKind(slot({ slot: 0 }), undefined, xinput)).toBe("empty");
  });

  it("um celular conectado é `connected`, mesmo reservado e sem input ainda", () => {
    expect(slotCardKind(slot({ state: "active" }), "live", undefined)).toBe("connected");
    expect(slotCardKind(slot({ state: "reserved" }), undefined, undefined)).toBe("connected");
    expect(slotCardKind(slot({ state: "active" }), "neutral", undefined)).toBe("connected");
  });

  it("sem sinal por 5 s ou conexão caída é `reconnecting`", () => {
    expect(slotCardKind(slot({ state: "active" }), "stalled", undefined)).toBe("reconnecting");
    expect(slotCardKind(slot({ state: "suspended", expires_in_ms: 20_000 }), undefined, [])).toBe(
      "reconnecting",
    );
  });

  it("o controle físico só importa em slot livre", () => {
    const xinput: XInputSlotDto[] = [{ index: 0, occupant: "physical" }];
    expect(slotCardKind(slot({ state: "active" }), "live", xinput)).toBe("connected");
  });
});

describe("secondsLeft", () => {
  it("arredonda para cima e nunca fica negativo", () => {
    expect(secondsLeft(10_000, 0)).toBe(10);
    expect(secondsLeft(10_000, 9_001)).toBe(1);
    expect(secondsLeft(10_000, 10_000)).toBe(0);
    expect(secondsLeft(10_000, 15_000)).toBe(0);
  });
});
