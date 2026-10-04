import { useRef, type KeyboardEvent, type PointerEvent } from "react";
import { useTranslation } from "react-i18next";
import type { Curve } from "@/lib/bindings";
import { evalCurve, isCustom, movePoint, pointsOf, type CurvePoints } from "./profileModel";

const SIZE = 180;
const PAD = 12;
const SPAN = SIZE - 2 * PAD;
const STEP = 0.02;

const px = (x: number) => PAD + x * SPAN;
const py = (y: number) => SIZE - PAD - y * SPAN;

interface CurveEditorProps {
  curve: Curve;
  /** Só a curva personalizada tem pontos para arrastar. */
  onChange: (curve: Curve) => void;
}

/** A resposta do analógico (entrada → saída). Na curva personalizada, os 4 pontos se arrastam. */
export function CurveEditor({ curve, onChange }: CurveEditorProps) {
  const { t } = useTranslation("mapping");
  const svg = useRef<SVGSVGElement>(null);
  const dragging = useRef<number | null>(null);
  const custom = isCustom(curve);
  const points = pointsOf(curve);

  const path = Array.from({ length: 65 }, (_, i) => {
    const x = i / 64;
    return `${i === 0 ? "M" : "L"}${px(x).toFixed(1)} ${py(evalCurve(curve, x)).toFixed(1)}`;
  }).join(" ");

  const update = (next: CurvePoints) => {
    onChange({ type: "custom", points: next });
  };

  const onMove = (event: PointerEvent) => {
    const index = dragging.current;
    const rect = svg.current?.getBoundingClientRect();
    if (index === null || !rect) return;
    const scale = SIZE / rect.width;
    const x = ((event.clientX - rect.left) * scale - PAD) / SPAN;
    const y = 1 - ((event.clientY - rect.top) * scale - PAD) / SPAN;
    update(movePoint(points, index, x, y));
  };

  const onKey = (index: number) => (event: KeyboardEvent) => {
    const [x, y] = points[index] ?? [0, 0];
    const delta: Record<string, [number, number]> = {
      ArrowLeft: [-STEP, 0],
      ArrowRight: [STEP, 0],
      ArrowUp: [0, STEP],
      ArrowDown: [0, -STEP],
    };
    const move = delta[event.key];
    if (!move) return;
    event.preventDefault();
    update(movePoint(points, index, x + move[0], y + move[1]));
  };

  return (
    <svg
      ref={svg}
      viewBox={`0 0 ${String(SIZE)} ${String(SIZE)}`}
      role="group"
      aria-label={t("curve.label")}
      onPointerMove={onMove}
      onPointerUp={() => {
        dragging.current = null;
      }}
      onPointerCancel={() => {
        dragging.current = null;
      }}
      className="size-44 shrink-0 touch-none select-none"
    >
      <rect x={PAD} y={PAD} width={SPAN} height={SPAN} className="fill-control stroke-separator" />
      {[0.25, 0.5, 0.75].map((g) => (
        <g key={g} className="stroke-separator" strokeWidth={0.5}>
          <line x1={px(g)} y1={PAD} x2={px(g)} y2={SIZE - PAD} />
          <line x1={PAD} y1={py(g)} x2={SIZE - PAD} y2={py(g)} />
        </g>
      ))}
      <line
        x1={px(0)}
        y1={py(0)}
        x2={px(1)}
        y2={py(1)}
        className="stroke-fg-3"
        strokeDasharray="3 3"
        strokeWidth={0.75}
      />
      <path d={path} className="fill-none stroke-accent" strokeWidth={2} />
      {custom &&
        points.map(([x, y], index) => (
          <circle
            key={index}
            cx={px(x)}
            cy={py(y)}
            r={6}
            tabIndex={0}
            role="slider"
            aria-label={t("curve.point", { n: index + 1 })}
            aria-valuemin={0}
            aria-valuemax={100}
            aria-valuenow={Math.round(y * 100)}
            aria-valuetext={t("curve.pointValue", {
              x: Math.round(x * 100),
              y: Math.round(y * 100),
            })}
            onKeyDown={onKey(index)}
            onPointerDown={(event) => {
              dragging.current = index;
              event.currentTarget.setPointerCapture(event.pointerId);
              event.preventDefault();
            }}
            className="cursor-grab fill-knob stroke-accent focus-ring outline-none active:cursor-grabbing"
            strokeWidth={2}
            data-testid={`curve-point-${String(index)}`}
          />
        ))}
    </svg>
  );
}
