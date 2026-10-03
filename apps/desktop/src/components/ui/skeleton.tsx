import type { ComponentProps } from "react";
import { cn } from "@/lib/utils";

/** Placeholder de carregamento; o pulso para com `prefers-reduced-motion` (regra global do CSS). */
export function Skeleton({ className, ...props }: ComponentProps<"div">) {
  return (
    <div aria-hidden className={cn("animate-pulse rounded-md bg-control", className)} {...props} />
  );
}
