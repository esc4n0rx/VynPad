import type { ComponentProps } from "react";
import { cn } from "@/lib/utils";

/** Card de vidro: fundo elevado translúcido com blur e sombra suave. */
export function Card({ className, ...props }: ComponentProps<"div">) {
  return <div className={cn("rounded-lg p-4 glass", className)} {...props} />;
}

export function CardTitle({ className, children, ...props }: ComponentProps<"h3">) {
  return (
    <h3 className={cn("text-subtitle font-semibold text-fg", className)} {...props}>
      {children}
    </h3>
  );
}

export function CardDescription({ className, ...props }: ComponentProps<"p">) {
  return <p className={cn("text-body text-fg-2", className)} {...props} />;
}
