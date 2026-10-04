import { useRef, type PointerEvent } from "react";
import { clamp, DEADZONE_SUM_LIMIT, snap } from "./profileModel";

const SIZE = 160;
const CENTER = SIZE / 2;
const RADIUS = 70;

interface DeadzoneDiagramProps {
  inner: number;
  outer: number;
  shape: "radial" | "axial";
  onChange: (patch: { deadzone_inner?: number; deadzone_outer?: number }) => void;
  /** Descrição para leitores de tela (os sliders ao lado fazem o mesmo, com teclado). */
  label: string;
}

/**
 * O curso do analógico com as duas deadzones: o miolo apagado (interna) e a borda (externa),
 * cada uma com uma alça para arrastar. Os sliders do painel controlam os mesmos valores.
 */
export function DeadzoneDiagram({ inner, outer, shape, onChange, label }: DeadzoneDiagramProps) {
  const svg = useRef<SVGSVGElement>(null);
  const dragging = useRef<"inner" | "outer" | null>(null);

  const distance = (event: PointerEvent) => {
    const rect = svg.current?.getBoundingClientRect();
    if (!rect) return 0;
    const scale = SIZE / rect.width;
    const x = (event.clientX - rect.left) * scale - CENTER;
    const y = (event.clientY - rect.top) * scale - CENTER;
    return shape === "axial" ? Math.max(Math.abs(x), Math.abs(y)) : Math.hypot(x, y);
  };

  const move = (event: PointerEvent) => {
    if (!dragging.current) return;
    const fraction = distance(event) / RADIUS;
    if (dragging.current === "inner") {
      const max = DEADZONE_SUM_LIMIT - 0.01 - outer;
      onChange({ deadzone_inner: snap(clamp(fraction, 0, Math.min(0.5, max))) });
    } else {
      const max = DEADZONE_SUM_LIMIT - 0.01 - inner;
      onChange({ deadzone_outer: snap(clamp(1 - fraction, 0, Math.min(0.5, max))) });
    }
  };

  const startDrag = (which: "inner" | "outer", event: PointerEvent) => {
    dragging.current = which;
    event.currentTarget.setPointerCapture(event.pointerId);
    event.preventDefault();
  };
  const release = () => {
    dragging.current = null;
  };

  const innerR = inner * RADIUS;
  const outerR = (1 - outer) * RADIUS;
  const area = (r: number) =>
    shape === "axial" ? (
      <rect x={CENTER - r} y={CENTER - r} width={2 * r} height={2 * r} />
    ) : (
      <circle cx={CENTER} cy={CENTER} r={r} />
    );

  return (
    <svg
      ref={svg}
      viewBox={`0 0 ${String(SIZE)} ${String(SIZE)}`}
      role="img"
      aria-label={label}
      onPointerMove={move}
      onPointerUp={release}
      onPointerCancel={release}
      className="size-40 shrink-0 touch-none select-none"
    >
      {/* O curso inteiro. */}
      <g className="fill-control stroke-separator" strokeWidth={1}>
        {area(RADIUS)}
      </g>
      {/* Até onde o analógico vale (o que passa da externa vira o máximo). */}
      <g className="fill-accent-soft stroke-accent" strokeWidth={1.5}>
        {area(Math.max(outerR, 1))}
      </g>
      {/* A deadzone interna: tudo aqui dentro é zero. */}
      <g className="fill-window stroke-fg-3" strokeWidth={1.5} strokeDasharray="4 3">
        {area(Math.max(innerR, 0.5))}
      </g>
      <line x1={CENTER} y1={6} x2={CENTER} y2={SIZE - 6} className="stroke-separator" />
      <line x1={6} y1={CENTER} x2={SIZE - 6} y2={CENTER} className="stroke-separator" />
      <circle
        cx={CENTER + innerR}
        cy={CENTER}
        r={6}
        onPointerDown={(event) => {
          startDrag("inner", event);
        }}
        className="cursor-ew-resize fill-knob stroke-fg-3"
        strokeWidth={1.5}
        data-testid="deadzone-inner-handle"
      />
      <circle
        cx={CENTER + outerR}
        cy={CENTER}
        r={6}
        onPointerDown={(event) => {
          startDrag("outer", event);
        }}
        className="cursor-ew-resize fill-accent stroke-on-accent"
        strokeWidth={1.5}
        data-testid="deadzone-outer-handle"
      />
    </svg>
  );
}
