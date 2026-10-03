import type { ComponentProps } from "react";
import { cn } from "@/lib/utils";

export function Input({ className, type = "text", ...props }: ComponentProps<"input">) {
  return (
    <input
      type={type}
      className={cn(
        "h-8 w-full rounded-md bg-control px-3 text-body text-fg focus-ring transition-colors outline-none select-text placeholder:text-fg-3 hover:bg-control-hover disabled:opacity-40 aria-[invalid=true]:ring-1 aria-[invalid=true]:ring-danger",
        className,
      )}
      {...props}
    />
  );
}
