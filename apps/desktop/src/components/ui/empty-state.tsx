import type { ComponentProps, ReactNode } from "react";
import { cn } from "@/lib/utils";

interface EmptyStateProps extends Omit<ComponentProps<"div">, "title"> {
  /** Ilustração ou ícone grande (decorativo). */
  icon?: ReactNode;
  title: ReactNode;
  description?: ReactNode;
  /** Ação principal (ex.: um `Button`). */
  action?: ReactNode;
}

export function EmptyState({
  icon,
  title,
  description,
  action,
  className,
  ...props
}: EmptyStateProps) {
  return (
    <div
      className={cn("flex flex-col items-center gap-2 px-6 py-10 text-center", className)}
      {...props}
    >
      {icon && (
        <div
          aria-hidden
          className="mb-2 flex size-14 items-center justify-center rounded-xl bg-control text-fg-2 [&_svg]:size-7"
        >
          {icon}
        </div>
      )}
      <p className="text-subtitle font-semibold text-fg">{title}</p>
      {description && <p className="max-w-sm text-body text-fg-2">{description}</p>}
      {action && <div className="mt-3">{action}</div>}
    </div>
  );
}
