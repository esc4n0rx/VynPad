import { cn } from "@/lib/utils";

const COLORS = ["bg-slot-1", "bg-slot-2", "bg-slot-3", "bg-slot-4"] as const;
const SIZES = {
  sm: "size-5 text-[10px]",
  md: "size-7 text-caption",
  lg: "size-9 text-body",
} as const;

interface SlotBadgeProps {
  /** Índice do slot, de 0 a 3 (mostra P1–P4). */
  slot: number;
  size?: keyof typeof SIZES;
  /** Slot sem controle: contorno na cor do slot em vez de preenchido. */
  dim?: boolean;
  className?: string;
}

/** Indicador P1–P4 na cor do slot. */
export function SlotBadge({ slot, size = "md", dim, className }: SlotBadgeProps) {
  const index = ((slot % 4) + 4) % 4;
  return (
    <span
      className={cn(
        "inline-flex shrink-0 items-center justify-center rounded-full font-semibold tabular-nums",
        SIZES[size],
        dim ? "bg-control text-fg-3" : cn(COLORS[index], "text-on-accent"),
        className,
      )}
    >
      {`P${String(index + 1)}`}
    </span>
  );
}
