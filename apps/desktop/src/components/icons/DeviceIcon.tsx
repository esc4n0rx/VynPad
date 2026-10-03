import { useTranslation } from "react-i18next";
import { cn } from "@/lib/utils";

export type DeviceStatus = "connected" | "idle" | "pairing" | "offline";

interface DeviceIconProps {
  status: DeviceStatus;
  /** Esconde o ícone dos leitores de tela quando o estado já aparece em texto ao lado. */
  decorative?: boolean;
  className?: string;
}

const BADGE_FILL: Record<DeviceStatus, string> = {
  connected: "fill-success",
  idle: "fill-fg-3",
  pairing: "fill-info",
  offline: "fill-danger",
};

/**
 * Celular com um mini-gamepad na tela, em quatro estados: conectado (selo verde), ocioso,
 * pareando (ondas pulsando) e offline (corte diagonal e desbotado).
 */
export function DeviceIcon({ status, decorative, className }: DeviceIconProps) {
  const { t } = useTranslation();
  const label = t(`deviceStatus.${status}`);

  return (
    <svg
      viewBox="0 0 48 48"
      role={decorative ? undefined : "img"}
      aria-label={decorative ? undefined : label}
      aria-hidden={decorative ? true : undefined}
      data-status={status}
      className={cn("size-12", status === "offline" && "opacity-60", className)}
    >
      {status === "pairing" && (
        <g className="fill-none stroke-info" strokeWidth={1.5} strokeLinecap="round">
          <path className="animate-pulse" d="M6 16a14 14 0 0 0 0 16" />
          <path className="animate-pulse [animation-delay:250ms]" d="M2 12a20 20 0 0 0 0 24" />
          <path className="animate-pulse" d="M42 16a14 14 0 0 1 0 16" />
          <path className="animate-pulse [animation-delay:250ms]" d="M46 12a20 20 0 0 1 0 24" />
        </g>
      )}
      <rect
        x={13}
        y={4}
        width={22}
        height={40}
        rx={5}
        className="fill-elevated stroke-fg-2"
        strokeWidth={1.75}
      />
      <rect x={16} y={9} width={16} height={28} rx={2} className="fill-control" />
      {/* mini-gamepad na tela */}
      <path
        className="fill-fg-2"
        d="M19.5 20.5h9a3 3 0 0 1 2.9 2.3l1 4a1.8 1.8 0 0 1-3 1.7l-1.7-1.8h-6.4l-1.7 1.8a1.8 1.8 0 0 1-3-1.7l1-4a3 3 0 0 1 2.9-2.3z"
      />
      <circle cx={21.5} cy={23.8} r={0.9} className="fill-control" />
      <circle cx={26.8} cy={23.2} r={0.7} className="fill-control" />
      <circle cx={28.6} cy={24.6} r={0.7} className="fill-control" />
      <rect x={20} y={40} width={8} height={1.5} rx={0.75} className="fill-fg-3" />
      {status !== "pairing" && (
        <g>
          <circle cx={37} cy={37} r={6} className={BADGE_FILL[status]} />
          {status === "connected" && (
            <path
              d="M34 37l2.2 2.2L40.2 35"
              className="fill-none stroke-on-accent"
              strokeWidth={1.75}
              strokeLinecap="round"
              strokeLinejoin="round"
            />
          )}
          {status === "offline" && (
            <path
              d="M34.5 34.5l5 5M39.5 34.5l-5 5"
              className="stroke-on-accent"
              strokeWidth={1.75}
              strokeLinecap="round"
            />
          )}
        </g>
      )}
    </svg>
  );
}
