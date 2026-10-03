import { Check } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/utils";

interface StepperProps {
  /** Rótulos dos passos, em ordem. */
  steps: readonly string[];
  /** Índice (a partir de 0) do passo atual. */
  current: number;
  className?: string;
}

/** Indicador de progresso do onboarding: passos concluídos, atual e pendentes. */
export function Stepper({ steps, current, className }: StepperProps) {
  const { t } = useTranslation();
  return (
    <ol aria-label={t("ui.progress")} className={cn("flex items-center gap-2", className)}>
      {steps.map((label, index) => {
        const done = index < current;
        const active = index === current;
        return (
          <li
            key={label}
            aria-current={active ? "step" : undefined}
            className="flex items-center gap-2"
          >
            <span
              className={cn(
                "flex size-6 items-center justify-center rounded-full text-caption font-semibold tabular-nums transition-colors",
                done && "bg-accent text-on-accent",
                active && "bg-accent-soft text-accent ring-2 ring-accent",
                !done && !active && "bg-control text-fg-3",
              )}
            >
              {done ? <Check className="size-3.5" strokeWidth={2.5} /> : index + 1}
            </span>
            <span className={cn("text-body", active ? "font-medium text-fg" : "text-fg-2")}>
              {label}
            </span>
            {index < steps.length - 1 && (
              <span
                aria-hidden
                className={cn("h-px w-6", index < current ? "bg-accent" : "bg-separator")}
              />
            )}
          </li>
        );
      })}
    </ol>
  );
}
