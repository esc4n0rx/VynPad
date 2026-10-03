import type { ComponentProps } from "react";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "@/lib/utils";

const badgeVariants = cva(
  "inline-flex items-center gap-1.5 rounded-full px-2 py-0.5 text-caption font-medium",
  {
    variants: {
      status: {
        neutral: "bg-control text-fg-2",
        success: "bg-success/15 text-success",
        warning: "bg-warning/15 text-warning",
        danger: "bg-danger/15 text-danger",
        info: "bg-info/15 text-info",
      },
    },
    defaultVariants: { status: "neutral" },
  },
);

const dotVariants: Record<NonNullable<VariantProps<typeof badgeVariants>["status"]>, string> = {
  neutral: "bg-fg-3",
  success: "bg-success",
  warning: "bg-warning",
  danger: "bg-danger",
  info: "bg-info",
};

export type BadgeProps = ComponentProps<"span"> & VariantProps<typeof badgeVariants>;

/** Selo de estado (conectado, aviso, erro...). O texto carrega o sentido; a cor só reforça. */
export function Badge({ className, status, children, ...props }: BadgeProps) {
  return (
    <span className={cn(badgeVariants({ status }), className)} {...props}>
      <span aria-hidden className={cn("size-1.5 rounded-full", dotVariants[status ?? "neutral"])} />
      {children}
    </span>
  );
}
