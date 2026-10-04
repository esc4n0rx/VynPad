import { describe, expect, it } from "vitest";
import type { Profile } from "@/lib/bindings";
import {
  BUTTON_ELEMENT,
  DEFAULT_CURVE_POINTS,
  ELEMENT_BUTTON,
  curveOfType,
  evalCurve,
  gyroOf,
  movePoint,
  pointsOf,
  remapOf,
  rumbleOf,
  setGyro,
  setRumble,
  setRemap,
  setStick,
  setSwapSticks,
  setTrigger,
  setTurbo,
  stickOf,
  triggerOf,
  turboOf,
} from "./profileModel";

describe("valores padrão", () => {
  it("um perfil vazio resolve para os padrões do backend", () => {
    const profile: Profile = {};
    expect(stickOf(profile, "left")).toEqual({
      deadzone_inner: 0,
      deadzone_outer: 0,
      deadzone_shape: "radial",
      anti_deadzone: 0,
      curve: { type: "linear" },
      sensitivity: 1,
      invert_x: false,
      invert_y: false,
    });
    expect(triggerOf(profile, "right")).toEqual({ deadzone: 0, full_point: 1, digital: false });
    const gyro = gyroOf(profile);
    expect(gyro.enabled).toBe(false);
    expect(gyro.mode).toEqual({ type: "always" });
    expect(gyro.target).toBe("right");
    expect(gyro.deadzone).toBe(0.02);
    expect(gyro.smoothing).toEqual({ enabled: true, min_cutoff: 1, beta: 0.05 });
  });

  it("nulos do tipo gerado também viram o padrão", () => {
    const profile: Profile = { sticks: { left: { sensitivity: null, deadzone_inner: null } } };
    expect(stickOf(profile, "left").sensitivity).toBe(1);
    expect(stickOf(profile, "left").deadzone_inner).toBe(0);
  });

  it("os elementos do desenho e os botões do protocolo se correspondem nos dois sentidos", () => {
    expect(Object.keys(ELEMENT_BUTTON)).toHaveLength(15);
    for (const [element, button] of Object.entries(ELEMENT_BUTTON)) {
      expect(BUTTON_ELEMENT[button]).toBe(element);
    }
  });
});

describe("analógicos", () => {
  it("altera só o analógico pedido e não muda o perfil original", () => {
    const original: Profile = { sticks: { right: { sensitivity: 2 } } };
    const next = setStick(original, "left", { sensitivity: 1.5, invert_x: true });
    expect(next.sticks?.left?.sensitivity).toBe(1.5);
    expect(next.sticks?.left?.invert_x).toBe(true);
    expect(next.sticks?.right?.sensitivity).toBe(2);
    expect(original.sticks?.left).toBeUndefined();
  });

  it("deadzone interna + externa nunca chega a 0,95: quem mexeu cede o excesso", () => {
    const base = setStick({}, "left", { deadzone_inner: 0.4 });
    const outer = setStick(base, "left", { deadzone_outer: 0.5 });
    const o = stickOf(outer, "left");
    expect(o.deadzone_inner).toBe(0.4);
    expect(o.deadzone_inner + o.deadzone_outer).toBeLessThan(0.95);

    const inner = setStick(setStick({}, "left", { deadzone_outer: 0.5 }), "left", {
      deadzone_inner: 0.5,
    });
    const i = stickOf(inner, "left");
    expect(i.deadzone_outer).toBe(0.5);
    expect(i.deadzone_inner + i.deadzone_outer).toBeLessThan(0.95);
  });

  it("trocar esquerdo e direito fica em `sticks.swap`", () => {
    expect(setSwapSticks({}, true).sticks?.swap).toBe(true);
    expect(setSwapSticks({ sticks: { swap: true } }, false).sticks?.swap).toBe(false);
  });
});

describe("gatilhos", () => {
  it("o ponto máximo fica sempre acima da deadzone", () => {
    const a = setTrigger({}, "left", { deadzone: 0.5 });
    expect(triggerOf(a, "left").full_point).toBe(1);
    const b = setTrigger({}, "left", { full_point: 0.3 });
    expect(triggerOf(b, "left").full_point).toBe(0.3);
    // Subir a deadzone além do ponto máximo empurra o ponto máximo.
    const c = setTrigger(b, "left", { deadzone: 0.6 });
    expect(triggerOf(c, "left").full_point).toBeGreaterThan(triggerOf(c, "left").deadzone);
    // Descer o ponto máximo abaixo da deadzone empurra a deadzone.
    const d = setTrigger(setTrigger({}, "right", { deadzone: 0.5 }), "right", { full_point: 0.4 });
    expect(triggerOf(d, "right").deadzone).toBeLessThan(0.4);
  });

  it("o modo digital é independente", () => {
    expect(triggerOf(setTrigger({}, "right", { digital: true }), "right").digital).toBe(true);
    expect(triggerOf(setTrigger({}, "right", { digital: true }), "left").digital).toBe(false);
  });
});

describe("botões", () => {
  it("mapear para outro botão grava no remap, e voltar para si mesmo apaga a entrada", () => {
    const mapped = setRemap({}, "a", "b");
    expect(remapOf(mapped, "a")).toBe("b");
    expect(remapOf(mapped, "b")).toBeUndefined();
    const back = setRemap(mapped, "a", "a");
    expect(remapOf(back, "a")).toBeUndefined();
    expect(back.buttons?.remap).toEqual({});
  });

  it("aceita os destinos especiais", () => {
    expect(remapOf(setRemap({}, "x", "left_trigger"), "x")).toBe("left_trigger");
    expect(remapOf(setRemap({}, "x", "none"), "x")).toBe("none");
  });

  it("turbo liga com a taxa, troca a taxa e desliga", () => {
    const on = setTurbo({}, "a", 12);
    expect(turboOf(on, "a")).toEqual({ button: "a", hz: 12 });
    const faster = setTurbo(on, "a", 20);
    expect(faster.buttons?.turbo).toEqual([{ button: "a", hz: 20 }]);
    const two = setTurbo(faster, "b", 5);
    expect(two.buttons?.turbo).toHaveLength(2);
    expect(setTurbo(two, "a", null).buttons?.turbo).toEqual([{ button: "b", hz: 5 }]);
  });

  it("a taxa do turbo fica entre 1 e 30", () => {
    expect(turboOf(setTurbo({}, "a", 99), "a")?.hz).toBe(30);
    expect(turboOf(setTurbo({}, "a", 0), "a")?.hz).toBe(1);
  });
});

describe("gyro", () => {
  it("altera um campo sem perder os outros", () => {
    const profile = setGyro(setGyro({}, { enabled: true }), { sensitivity_x: 3 });
    const gyro = gyroOf(profile);
    expect(gyro.enabled).toBe(true);
    expect(gyro.sensitivity_x).toBe(3);
    expect(gyro.sensitivity_y).toBe(1);
  });

  it("os modos que dependem de botão levam o botão junto", () => {
    const profile = setGyro({}, { mode: { type: "while_held", button: "left_shoulder" } });
    expect(gyroOf(profile).mode).toEqual({ type: "while_held", button: "left_shoulder" });
  });
});

describe("vibração", () => {
  it("um perfil sem a seção usa o padrão: ligada, a 100%", () => {
    expect(rumbleOf({})).toEqual({ enabled: true, intensity: 1 });
  });

  it("liga, desliga e muda a intensidade sem perder o resto", () => {
    const half = setRumble({}, { intensity: 0.5 });
    expect(rumbleOf(half)).toEqual({ enabled: true, intensity: 0.5 });
    const off = setRumble(half, { enabled: false });
    expect(rumbleOf(off)).toEqual({ enabled: false, intensity: 0.5 });
  });

  it("a intensidade fica entre 0 e 200%", () => {
    expect(rumbleOf(setRumble({}, { intensity: 3 })).intensity).toBe(2);
    expect(rumbleOf(setRumble({}, { intensity: -1 })).intensity).toBe(0);
    expect(rumbleOf(setRumble({}, { intensity: 0.3333 })).intensity).toBe(0.33);
  });
});

describe("curvas", () => {
  it("a linear é a identidade", () => {
    for (const x of [0, 0.3, 1]) expect(evalCurve({ type: "linear" }, x)).toBe(x);
  });

  it("a exponencial é x elevado ao expoente", () => {
    expect(evalCurve({ type: "exponential", exponent: 2 }, 0.5)).toBeCloseTo(0.25);
    expect(evalCurve({ type: "exponential", exponent: 0.5 }, 0.25)).toBeCloseTo(0.5);
    expect(evalCurve({ type: "exponential", exponent: null }, 0.5)).toBeCloseTo(0.5);
  });

  it("a personalizada passa pelos pontos e liga (0,0) e (1,1) com retas", () => {
    const curve = {
      type: "custom" as const,
      points: [
        [0.25, 0.1],
        [0.5, 0.2],
        [0.75, 0.6],
        [0.9, 0.9],
      ] as [[number, number], [number, number], [number, number], [number, number]],
    };
    expect(evalCurve(curve, 0)).toBe(0);
    expect(evalCurve(curve, 0.25)).toBeCloseTo(0.1);
    expect(evalCurve(curve, 0.5)).toBeCloseTo(0.2);
    expect(evalCurve(curve, 0.375)).toBeCloseTo(0.15);
    expect(evalCurve(curve, 1)).toBe(1);
    expect(evalCurve(curve, 0.95)).toBeCloseTo(0.95);
  });

  it("trocar o tipo preserva o que dá e usa padrões no resto", () => {
    const expo = curveOfType("exponential", { type: "linear" });
    expect(expo).toEqual({ type: "exponential", exponent: 2 });
    expect(curveOfType("exponential", { type: "exponential", exponent: 3 })).toEqual({
      type: "exponential",
      exponent: 3,
    });
    expect(curveOfType("linear", expo)).toEqual({ type: "linear" });
    expect(curveOfType("custom", expo)).toEqual({ type: "custom", points: DEFAULT_CURVE_POINTS });
    const custom = { type: "custom" as const, points: DEFAULT_CURVE_POINTS };
    expect(pointsOf(custom)).toEqual(DEFAULT_CURVE_POINTS);
  });

  it("mover um ponto mantém x estritamente crescente dentro de (0, 1) e y em [0, 1]", () => {
    const moved = movePoint(DEFAULT_CURVE_POINTS, 1, 0.9, 2);
    // O ponto 1 não passa do ponto 2 (x = 0,75).
    expect(moved[1][0]).toBeLessThan(moved[2][0]);
    expect(moved[1][1]).toBe(1);
    const xs = moved.map((p) => p[0]);
    expect([...xs].sort((a, b) => a - b)).toEqual(xs);
    expect(new Set(xs).size).toBe(4);

    const first = movePoint(DEFAULT_CURVE_POINTS, 0, -1, -1);
    expect(first[0][0]).toBeGreaterThan(0);
    expect(first[0][1]).toBe(0);
    const last = movePoint(DEFAULT_CURVE_POINTS, 3, 5, 0.5);
    expect(last[3][0]).toBeLessThan(1);
    // O original não é alterado.
    expect(DEFAULT_CURVE_POINTS[1]).toEqual([0.5, 0.5]);
  });
});
