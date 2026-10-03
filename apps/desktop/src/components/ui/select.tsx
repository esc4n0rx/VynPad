import type { ReactNode } from "react";
import { Check, ChevronDown } from "lucide-react";
import { Select as SelectPrimitive } from "radix-ui";
import { cn } from "@/lib/utils";

export interface SelectOption {
  value: string;
  label: ReactNode;
}

interface SelectProps {
  options: readonly SelectOption[];
  value: string;
  onValueChange: (value: string) => void;
  /** Nome acessível (ex.: "Idioma"). */
  label: string;
  placeholder?: string;
  disabled?: boolean;
  className?: string;
}

export function Select({
  options,
  value,
  onValueChange,
  label,
  placeholder,
  disabled,
  className,
}: SelectProps) {
  return (
    <SelectPrimitive.Root value={value} onValueChange={onValueChange} disabled={disabled}>
      <SelectPrimitive.Trigger
        aria-label={label}
        className={cn(
          "inline-flex h-8 min-w-36 items-center justify-between gap-2 rounded-md bg-control px-3 text-body text-fg focus-ring transition-colors hover:bg-control-hover disabled:opacity-40",
          className,
        )}
      >
        <SelectPrimitive.Value placeholder={placeholder} />
        <SelectPrimitive.Icon>
          <ChevronDown className="size-4 text-fg-2" strokeWidth={1.75} />
        </SelectPrimitive.Icon>
      </SelectPrimitive.Trigger>
      <SelectPrimitive.Portal>
        <SelectPrimitive.Content
          position="popper"
          sideOffset={6}
          className="z-50 min-w-(--radix-select-trigger-width) overflow-hidden rounded-lg p-1 shadow-popover glass"
        >
          <SelectPrimitive.Viewport>
            {options.map((option) => (
              <SelectPrimitive.Item
                key={option.value}
                value={option.value}
                className="relative flex h-7 cursor-default items-center rounded-sm pr-2 pl-7 text-body text-fg outline-none select-none data-[highlighted]:bg-accent data-[highlighted]:text-on-accent"
              >
                <SelectPrimitive.ItemIndicator className="absolute left-2">
                  <Check className="size-3.5" strokeWidth={2} />
                </SelectPrimitive.ItemIndicator>
                <SelectPrimitive.ItemText>{option.label}</SelectPrimitive.ItemText>
              </SelectPrimitive.Item>
            ))}
          </SelectPrimitive.Viewport>
        </SelectPrimitive.Content>
      </SelectPrimitive.Portal>
    </SelectPrimitive.Root>
  );
}
