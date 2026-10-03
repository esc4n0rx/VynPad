import { useId, type ReactNode } from "react";
import { motion } from "motion/react";
import { RadioGroup } from "radix-ui";
import { cn } from "@/lib/utils";

export interface SegmentedOption<T extends string> {
  value: T;
  label: ReactNode;
}

interface SegmentedControlProps<T extends string> {
  options: readonly SegmentedOption<T>[];
  value: T;
  onValueChange: (value: T) => void;
  /** Nome acessível do grupo (ex.: "Tema"). */
  label: string;
  className?: string;
}

/** Escolha única entre poucas opções, com a pílula selecionada deslizando (spring). */
export function SegmentedControl<T extends string>({
  options,
  value,
  onValueChange,
  label,
  className,
}: SegmentedControlProps<T>) {
  const pillId = useId();

  return (
    <RadioGroup.Root
      aria-label={label}
      value={value}
      onValueChange={(v) => {
        onValueChange(v as T);
      }}
      orientation="horizontal"
      className={cn("inline-flex rounded-md bg-control p-0.5", className)}
    >
      {options.map((option) => {
        const selected = option.value === value;
        return (
          <RadioGroup.Item
            key={option.value}
            value={option.value}
            className={cn(
              "relative h-7 rounded-[8px] px-3 text-body font-medium focus-ring transition-colors",
              selected ? "text-fg" : "text-fg-2 hover:text-fg",
            )}
          >
            {selected && (
              <motion.span
                layoutId={pillId}
                transition={{ type: "spring", stiffness: 380, damping: 32 }}
                className="absolute inset-0 rounded-[8px] bg-elevated shadow-card"
              />
            )}
            <span className="relative">{option.label}</span>
          </RadioGroup.Item>
        );
      })}
    </RadioGroup.Root>
  );
}
