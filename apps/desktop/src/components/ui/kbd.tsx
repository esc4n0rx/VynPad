import type { ComponentProps } from "react";
import { cn } from "@/lib/utils";

/** Tecla de atalho (ex.: `Ctrl` `K`). */
export function Kbd({ className, ...props }: ComponentProps<"kbd">) {
  return (
    <kbd
      className={cn(
        "inline-flex h-5 min-w-5 items-center justify-center rounded-sm bg-control px-1.5 font-sans text-caption font-medium text-fg-2",
        className,
      )}
      {...props}
    />
  );
}
