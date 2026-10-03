import type { InputReportDto } from "@/lib/bindings";
import { BUTTON_ELEMENTS, BUTTON_MASKS, NEUTRAL_REPORT } from "./elements";

/** Quanto o cap do analógico anda (unidades do SVG) com o eixo no máximo. */
export const STICK_TRAVEL = 9;
const AXIS_MAX = 32768;

export interface GamepadRenderer {
  /** Pinta `report` no SVG; `undefined` solta tudo. Barato quando nada mudou. */
  apply: (report: InputReportDto | undefined) => void;
}

/** Posição do cap, em unidades do SVG, para um par de eixos `XInput` (Y positivo = para cima). */
export function stickOffset(x: number, y: number): { dx: number; dy: number } {
  let nx = x / AXIS_MAX;
  let ny = -y / AXIS_MAX;
  // O curso é circular: a diagonal não passa do raio.
  const length = Math.hypot(nx, ny);
  if (length > 1) {
    nx /= length;
    ny /= length;
  }
  return { dx: nx * STICK_TRAVEL, dy: ny * STICK_TRAVEL };
}

/**
 * Liga um relatório de input ao desenho do controle, mexendo direto nos atributos do SVG (sem
 * passar pelo React). Guarda o que já aplicou e só toca no DOM o que mudou.
 */
export function createGamepadRenderer(root: Element): GamepadRenderer {
  const find = (selector: string) => root.querySelector(selector);
  const buttons = BUTTON_ELEMENTS.map((name) => ({
    name,
    mask: BUTTON_MASKS[name],
    el: find(`[data-el="${name}"]`),
    pressed: undefined as boolean | undefined,
  }));
  const triggers = (["lt", "rt"] as const).map((name) => ({
    name,
    el: find(`[data-el="${name}"]`) as SVGElement | null,
    value: -1,
  }));
  const sticks = [
    { cap: find('[data-cap="left"]'), x: "lx", y: "ly", at: "" },
    { cap: find('[data-cap="right"]'), x: "rx", y: "ry", at: "" },
  ] as const;
  const stickAt = sticks.map(() => "");
  let last: InputReportDto | undefined;
  let applied = false;

  return {
    apply(report) {
      const next = report ?? NEUTRAL_REPORT;
      if (applied && (report === last || (report === undefined && last === undefined))) return;
      applied = true;
      last = report;

      for (const button of buttons) {
        const pressed = (next.buttons & button.mask) !== 0;
        if (pressed === button.pressed) continue;
        button.pressed = pressed;
        button.el?.setAttribute("data-pressed", String(pressed));
      }

      for (const trigger of triggers) {
        const value = Math.round((next[trigger.name] / 255) * 1000) / 1000;
        if (value === trigger.value) continue;
        trigger.value = value;
        trigger.el?.style.setProperty("--v", String(value));
        trigger.el?.setAttribute("data-pressed", String(value > 0));
      }

      sticks.forEach((stick, index) => {
        const { dx, dy } = stickOffset(next[stick.x], next[stick.y]);
        const at = `translate(${dx.toFixed(2)} ${dy.toFixed(2)})`;
        if (at === stickAt[index]) return;
        stickAt[index] = at;
        stick.cap?.setAttribute("transform", at);
      });
    },
  };
}
