import { useId, type ReactNode } from "react";
import { cn } from "@/lib/utils";

interface FieldProps {
  label: string;
  /** Recebe o `id` do rótulo (para `aria-labelledby`, quando o controle não tem rótulo próprio). */
  children: (labelId: string) => ReactNode;
  className?: string;
}

/** Uma linha do painel: rótulo à esquerda e o controle à direita (empilha em painéis estreitos). */
export function Field({ label, children, className }: FieldProps) {
  const labelId = useId();
  return (
    <div className={cn("flex flex-col gap-1.5", className)}>
      <span id={labelId} className="text-caption font-medium text-fg-2">
        {label}
      </span>
      {children(labelId)}
    </div>
  );
}

/** Rótulo e interruptor lado a lado. */
export function ToggleRow({
  label,
  children,
}: {
  label: string;
  children: (labelId: string) => ReactNode;
}) {
  const labelId = useId();
  return (
    <div className="flex items-center justify-between gap-3">
      <span id={labelId} className="text-body text-fg">
        {label}
      </span>
      {children(labelId)}
    </div>
  );
}
