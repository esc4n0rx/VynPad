import type { ButtonElement } from "@/components/gamepad/elements";
import type {
  Button,
  Curve,
  DeadzoneShape,
  GyroAxis,
  GyroMode,
  Profile,
  StickSide,
  Turbo,
} from "@/lib/bindings";

/** O analógico com todos os campos preenchidos (o tipo gerado admite ausente e nulo). */
export interface ResolvedStick {
  deadzone_inner: number;
  deadzone_outer: number;
  deadzone_shape: DeadzoneShape;
  anti_deadzone: number;
  curve: Curve;
  sensitivity: number;
  invert_x: boolean;
  invert_y: boolean;
}

export interface ResolvedTrigger {
  deadzone: number;
  full_point: number;
  digital: boolean;
}

export interface ResolvedGyro {
  enabled: boolean;
  mode: GyroMode;
  target: StickSide;
  axis_x: GyroAxis;
  axis_y: GyroAxis;
  sensitivity_x: number;
  sensitivity_y: number;
  invert_x: boolean;
  invert_y: boolean;
  deadzone: number;
  smoothing: { enabled: boolean; min_cutoff: number; beta: number };
  recenter_button: Button | null;
  mix_with_stick: boolean;
}

/** Os valores padrão do backend (`padgaming-mapping`): campo ausente no perfil vale isto. */
export const STICK_DEFAULTS = {
  deadzone_inner: 0,
  deadzone_outer: 0,
  deadzone_shape: "radial",
  anti_deadzone: 0,
  curve: { type: "linear" },
  sensitivity: 1,
  invert_x: false,
  invert_y: false,
} as const satisfies ResolvedStick;

export const TRIGGER_DEFAULTS = {
  deadzone: 0,
  full_point: 1,
  digital: false,
} as const satisfies ResolvedTrigger;

export const GYRO_DEFAULTS = {
  enabled: false,
  mode: { type: "always" },
  target: "right",
  axis_x: "yaw",
  axis_y: "pitch",
  sensitivity_x: 1,
  sensitivity_y: 1,
  invert_x: false,
  invert_y: false,
  deadzone: 0.02,
  smoothing: { enabled: true, min_cutoff: 1, beta: 0.05 },
  recenter_button: null,
  mix_with_stick: true,
} as const satisfies ResolvedGyro;

export const RUMBLE_DEFAULTS = { enabled: true, intensity: 1 } as const;
/** Intensidade máxima da vibração (200%). */
export const RUMBLE_MAX = 2;

export const TURBO_DEFAULT_HZ = 10;
export const TURBO_MIN_HZ = 1;
export const TURBO_MAX_HZ = 30;

/** Duas deadzones juntas não podem passar daqui (validação do backend). */
export const DEADZONE_SUM_LIMIT = 0.95;

/** Pontos de uma curva personalizada nova: quase linear, para o usuário puxar daí. */
export const DEFAULT_CURVE_POINTS: CurvePoints = [
  [0.25, 0.25],
  [0.5, 0.5],
  [0.75, 0.75],
  [0.9, 0.9],
];

type Point = [number, number];
export type CurvePoints = [Point, Point, Point, Point];

/** O elemento do desenho (`data-el`) ↔ o botão do protocolo. */
export const ELEMENT_BUTTON: Record<ButtonElement, Button> = {
  "btn-a": "a",
  "btn-b": "b",
  "btn-x": "x",
  "btn-y": "y",
  lb: "left_shoulder",
  rb: "right_shoulder",
  "dpad-up": "dpad_up",
  "dpad-down": "dpad_down",
  "dpad-left": "dpad_left",
  "dpad-right": "dpad_right",
  start: "start",
  back: "back",
  guide: "guide",
  "ls-click": "left_thumb",
  "rs-click": "right_thumb",
};

export const BUTTON_ELEMENT = Object.fromEntries(
  Object.entries(ELEMENT_BUTTON).map(([element, button]) => [button, element]),
) as Record<Button, ButtonElement>;

export const ALL_BUTTONS = Object.values(ELEMENT_BUTTON);

export const clamp = (value: number, min: number, max: number) =>
  Math.min(max, Math.max(min, value));

/** Arredonda ao passo (evita 0.30000000000000004 no JSON do perfil). */
export const snap = (value: number, step = 0.01) => Math.round(value / step) * step;

// ------------------------------------------------------------------ leitura com padrões

/** O analógico com todos os campos preenchidos (os ausentes ou nulos viram o padrão). */
export function stickOf(profile: Profile, side: StickSide): ResolvedStick {
  const s = profile.sticks?.[side] ?? {};
  return {
    deadzone_inner: s.deadzone_inner ?? STICK_DEFAULTS.deadzone_inner,
    deadzone_outer: s.deadzone_outer ?? STICK_DEFAULTS.deadzone_outer,
    deadzone_shape: s.deadzone_shape ?? STICK_DEFAULTS.deadzone_shape,
    anti_deadzone: s.anti_deadzone ?? STICK_DEFAULTS.anti_deadzone,
    curve: s.curve ?? STICK_DEFAULTS.curve,
    sensitivity: s.sensitivity ?? STICK_DEFAULTS.sensitivity,
    invert_x: s.invert_x ?? STICK_DEFAULTS.invert_x,
    invert_y: s.invert_y ?? STICK_DEFAULTS.invert_y,
  };
}

export function triggerOf(profile: Profile, side: StickSide): ResolvedTrigger {
  const t = profile.triggers?.[side] ?? {};
  return {
    deadzone: t.deadzone ?? TRIGGER_DEFAULTS.deadzone,
    full_point: t.full_point ?? TRIGGER_DEFAULTS.full_point,
    digital: t.digital ?? TRIGGER_DEFAULTS.digital,
  };
}

export function gyroOf(profile: Profile): ResolvedGyro {
  const g = profile.gyro ?? {};
  const smoothing = g.smoothing ?? {};
  return {
    enabled: g.enabled ?? GYRO_DEFAULTS.enabled,
    mode: g.mode ?? GYRO_DEFAULTS.mode,
    target: g.target ?? GYRO_DEFAULTS.target,
    axis_x: g.axis_x ?? GYRO_DEFAULTS.axis_x,
    axis_y: g.axis_y ?? GYRO_DEFAULTS.axis_y,
    sensitivity_x: g.sensitivity_x ?? GYRO_DEFAULTS.sensitivity_x,
    sensitivity_y: g.sensitivity_y ?? GYRO_DEFAULTS.sensitivity_y,
    invert_x: g.invert_x ?? GYRO_DEFAULTS.invert_x,
    invert_y: g.invert_y ?? GYRO_DEFAULTS.invert_y,
    deadzone: g.deadzone ?? GYRO_DEFAULTS.deadzone,
    smoothing: {
      enabled: smoothing.enabled ?? GYRO_DEFAULTS.smoothing.enabled,
      min_cutoff: smoothing.min_cutoff ?? GYRO_DEFAULTS.smoothing.min_cutoff,
      beta: smoothing.beta ?? GYRO_DEFAULTS.smoothing.beta,
    },
    recenter_button: g.recenter_button ?? GYRO_DEFAULTS.recenter_button,
    mix_with_stick: g.mix_with_stick ?? GYRO_DEFAULTS.mix_with_stick,
  };
}

export function rumbleOf(profile: Profile): { enabled: boolean; intensity: number } {
  return {
    enabled: profile.rumble?.enabled ?? RUMBLE_DEFAULTS.enabled,
    intensity: profile.rumble?.intensity ?? RUMBLE_DEFAULTS.intensity,
  };
}

// ------------------------------------------------------------------ escrita (imutável)

export function setStick(
  profile: Profile,
  side: StickSide,
  patch: Partial<ResolvedStick>,
): Profile {
  let next = { ...stickOf(profile, side), ...patch };
  // A soma das deadzones tem teto no backend: quem acabou de mudar cede o excesso.
  if (next.deadzone_inner + next.deadzone_outer >= DEADZONE_SUM_LIMIT) {
    const room = snap(DEADZONE_SUM_LIMIT - 0.01);
    next =
      patch.deadzone_outer !== undefined
        ? { ...next, deadzone_outer: snap(room - next.deadzone_inner) }
        : { ...next, deadzone_inner: snap(room - next.deadzone_outer) };
  }
  return { ...profile, sticks: { ...profile.sticks, [side]: next } };
}

export function setSwapSticks(profile: Profile, swap: boolean): Profile {
  return { ...profile, sticks: { ...profile.sticks, swap } };
}

export function setTrigger(
  profile: Profile,
  side: StickSide,
  patch: Partial<ResolvedTrigger>,
): Profile {
  const next = { ...triggerOf(profile, side), ...patch };
  // O ponto de acionamento total precisa ficar acima da deadzone.
  if (next.full_point <= next.deadzone) {
    if (patch.full_point !== undefined) next.deadzone = snap(Math.max(0, next.full_point - 0.05));
    else next.full_point = snap(Math.min(1, next.deadzone + 0.05));
  }
  return { ...profile, triggers: { ...profile.triggers, [side]: next } };
}

export function setRumble(
  profile: Profile,
  patch: Partial<{ enabled: boolean; intensity: number }>,
): Profile {
  const next = { ...rumbleOf(profile), ...patch };
  return { ...profile, rumble: { ...next, intensity: clamp(snap(next.intensity), 0, RUMBLE_MAX) } };
}

export function setGyro(profile: Profile, patch: Partial<ResolvedGyro>): Profile {
  return { ...profile, gyro: { ...gyroOf(profile), ...patch } };
}

/** Para onde o botão vai (`undefined` = para ele mesmo, que não precisa constar no perfil). */
export function remapOf(profile: Profile, button: Button): string | undefined {
  return profile.buttons?.remap?.[button];
}

export function setRemap(profile: Profile, button: Button, target: string): Profile {
  const remap = { ...profile.buttons?.remap };
  if (target === button) delete remap[button];
  else remap[button] = target;
  return { ...profile, buttons: { ...profile.buttons, remap } };
}

export function turboOf(profile: Profile, button: Button): Turbo | undefined {
  return profile.buttons?.turbo?.find((t) => t.button === button);
}

/** Liga (`hz`) ou desliga (`null`) o turbo do botão. */
export function setTurbo(profile: Profile, button: Button, hz: number | null): Profile {
  const others = (profile.buttons?.turbo ?? []).filter((t) => t.button !== button);
  const turbo =
    hz === null ? others : [...others, { button, hz: clamp(hz, TURBO_MIN_HZ, TURBO_MAX_HZ) }];
  return { ...profile, buttons: { ...profile.buttons, turbo } };
}

// ------------------------------------------------------------------ curvas

export const isCustom = (curve: Curve): curve is Extract<Curve, { type: "custom" }> =>
  curve.type === "custom";

/** Os 4 pontos de uma curva personalizada, sem os `null` que o tipo gerado admite. */
export function pointsOf(curve: Curve): CurvePoints {
  if (!isCustom(curve)) return DEFAULT_CURVE_POINTS;
  const p = curve.points.map(([x, y], i) => [
    x ?? DEFAULT_CURVE_POINTS[i]?.[0] ?? 0,
    y ?? DEFAULT_CURVE_POINTS[i]?.[1] ?? 0,
  ]) as CurvePoints;
  return p;
}

/** Muda um ponto mantendo `x` estritamente crescente dentro de (0, 1) e `y` em [0, 1]. */
export function movePoint(points: CurvePoints, index: number, x: number, y: number): CurvePoints {
  const GAP = 0.02;
  const lower = index === 0 ? GAP : (points[index - 1]?.[0] ?? 0) + GAP;
  const upper = index === 3 ? 1 - GAP : (points[index + 1]?.[0] ?? 1) - GAP;
  const next = points.map((p) => [...p]) as CurvePoints;
  next[index] = [snap(clamp(x, lower, upper)), snap(clamp(y, 0, 1))];
  return next;
}

/**
 * Valor da curva em `x` (0–1), para desenhar. Espelha o backend: a linear é a identidade, a
 * exponencial `x^e`, e a personalizada passa pelos 4 pontos e por (0,0) e (1,1) (segmentos de reta).
 */
export function evalCurve(curve: Curve, x: number): number {
  const t = clamp(x, 0, 1);
  switch (curve.type) {
    case "linear":
      return t;
    case "exponential":
      return t ** (curve.exponent ?? 1);
    case "custom": {
      const pts: Point[] = [[0, 0], ...pointsOf(curve), [1, 1]];
      for (let i = 1; i < pts.length; i++) {
        const [x1, y1] = pts[i - 1] ?? [0, 0];
        const [x2, y2] = pts[i] ?? [1, 1];
        if (t <= x2) return x2 === x1 ? y2 : y1 + ((y2 - y1) * (t - x1)) / (x2 - x1);
      }
      return 1;
    }
  }
}

/** Troca o tipo de curva preservando o que dá (o expoente e os pontos voltam aos padrões). */
export function curveOfType(type: Curve["type"], previous: Curve): Curve {
  switch (type) {
    case "linear":
      return { type: "linear" };
    case "exponential":
      return {
        type: "exponential",
        exponent: previous.type === "exponential" ? previous.exponent : 2,
      };
    case "custom":
      return { type: "custom", points: pointsOf(previous) };
  }
}
