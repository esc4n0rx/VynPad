import { Slider as SliderPrimitive } from "radix-ui";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/utils";

interface SliderProps {
  value: number;
  onValueChange: (value: number) => void;
  min: number;
  max: number;
  step?: number;
  /** Valor padrão: ganha uma marca na trilha e o duplo clique no valor volta a ele. */
  defaultValue?: number;
  /** Nome acessível (ex.: "Zona morta"). */
  label: string;
  /** Formata o número mostrado ao lado (ex.: `(v) => `${v}%``). */
  format?: (value: number) => string;
  disabled?: boolean;
  className?: string;
}

/** Slider com o valor atual visível e a marcação do valor padrão. */
export function Slider({
  value,
  onValueChange,
  min,
  max,
  step = 1,
  defaultValue,
  label,
  format = String,
  disabled,
  className,
}: SliderProps) {
  const { t } = useTranslation();
  const markPercent =
    defaultValue === undefined ? undefined : ((defaultValue - min) / (max - min)) * 100;

  return (
    <div className={cn("flex items-center gap-3", className)}>
      <SliderPrimitive.Root
        value={[value]}
        onValueChange={([v]) => {
          if (v !== undefined) onValueChange(v);
        }}
        min={min}
        max={max}
        step={step}
        disabled={disabled}
        className="relative flex h-5 flex-1 touch-none items-center select-none data-[disabled]:opacity-40"
      >
        <SliderPrimitive.Track className="relative h-1 grow rounded-full bg-control">
          <SliderPrimitive.Range className="absolute h-full rounded-full bg-accent" />
          {markPercent !== undefined && (
            <span
              aria-hidden
              data-testid="slider-default-mark"
              className="absolute top-1/2 h-2.5 w-0.5 -translate-x-1/2 -translate-y-1/2 rounded-full bg-fg-3"
              style={{ left: `${markPercent}%` }}
            />
          )}
        </SliderPrimitive.Track>
        <SliderPrimitive.Thumb
          aria-label={label}
          className="block size-4 rounded-full bg-knob shadow-card focus-ring transition-transform hover:scale-110"
        />
      </SliderPrimitive.Root>
      <output
        title={defaultValue === undefined ? undefined : t("ui.restoreDefault")}
        onDoubleClick={() => {
          if (defaultValue !== undefined && !disabled) onValueChange(defaultValue);
        }}
        className="w-12 shrink-0 text-right text-body text-fg-2 tabular-nums"
      >
        {format(value)}
      </output>
    </div>
  );
}
