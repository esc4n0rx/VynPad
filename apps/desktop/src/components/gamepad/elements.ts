import type { InputReportDto } from "@/lib/bindings";

/** Máscara de botões `XInput` de cada elemento do desenho (espelha `Buttons` em vynpad-protocol). */
export const BUTTON_MASKS = {
  "btn-a": 0x1000,
  "btn-b": 0x2000,
  "btn-x": 0x4000,
  "btn-y": 0x8000,
  lb: 0x0100,
  rb: 0x0200,
  "dpad-up": 0x0001,
  "dpad-down": 0x0002,
  "dpad-left": 0x0004,
  "dpad-right": 0x0008,
  start: 0x0010,
  back: 0x0020,
  guide: 0x0400,
  "ls-click": 0x0040,
  "rs-click": 0x0080,
} as const;

export type ButtonElement = keyof typeof BUTTON_MASKS;
export type TriggerElement = "lt" | "rt";
export type StickElement = "stick-left" | "stick-right";

/** Tudo o que se pode acender, mover ou selecionar no controle (`data-el` no SVG). */
export type GamepadElement = ButtonElement | TriggerElement | StickElement;

export const BUTTON_ELEMENTS = Object.keys(BUTTON_MASKS) as ButtonElement[];
export const GAMEPAD_ELEMENTS: readonly GamepadElement[] = [
  ...BUTTON_ELEMENTS,
  "lt",
  "rt",
  "stick-left",
  "stick-right",
];

/** Controle solto: nada apertado, gatilhos e analógicos no centro. */
export const NEUTRAL_REPORT: InputReportDto = {
  buttons: 0,
  lt: 0,
  rt: 0,
  lx: 0,
  ly: 0,
  rx: 0,
  ry: 0,
};

/** Cor do slot (P1–P4, índice 0–3) como valor CSS, vinda dos tokens do design system. */
export function slotColor(slot: number): string {
  return `var(--slot-${String((((slot % 4) + 4) % 4) + 1)})`;
}
