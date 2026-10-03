import type { ComponentProps } from "react";
import { Switch } from "radix-ui";
import { cn } from "@/lib/utils";

/** Interruptor estilo macOS (`role="switch"`). */
export function Toggle({ className, ...props }: ComponentProps<typeof Switch.Root>) {
  return (
    <Switch.Root
      className={cn(
        "inline-flex h-[22px] w-[38px] shrink-0 items-center rounded-full bg-control p-0.5 focus-ring transition-colors disabled:opacity-40 data-[state=checked]:bg-success",
        className,
      )}
      {...props}
    >
      <Switch.Thumb className="block size-[18px] rounded-full bg-knob shadow-sm transition-transform data-[state=checked]:translate-x-4" />
    </Switch.Root>
  );
}
