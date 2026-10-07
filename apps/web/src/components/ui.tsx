import type { AnchorHTMLAttributes, ReactNode } from "react";
import { cn } from "@/lib/cn";

type ButtonVariant = "primary" | "secondary" | "ghost";
type ButtonSize = "md" | "lg";

const base =
  "focus-ring inline-flex shrink-0 items-center justify-center gap-2 rounded-full font-semibold whitespace-nowrap select-none transition-[transform,background-color,box-shadow] duration-200 ease-out active:scale-[0.97]";

const variants: Record<ButtonVariant, string> = {
  primary:
    "bg-accent text-on-accent hover:bg-accent-hover shadow-[0_1px_0_rgb(255_255_255/0.28)_inset,0_6px_18px_-6px_var(--accent)]",
  secondary: "bg-control text-fg hover:bg-control-hover",
  ghost: "text-fg-2 hover:text-fg",
};

const sizes: Record<ButtonSize, string> = {
  md: "h-10 px-4 text-sm",
  lg: "h-12 px-6 text-base",
};

export function buttonClass(
  variant: ButtonVariant = "primary",
  size: ButtonSize = "lg",
  extra?: string,
) {
  return cn(base, variants[variant], sizes[size], extra);
}

interface ButtonLinkProps extends AnchorHTMLAttributes<HTMLAnchorElement> {
  variant?: ButtonVariant;
  size?: ButtonSize;
}

/** Link com cara de botão. `<a>` simples: serve para âncoras e para a rota de download. */
export function ButtonLink({ variant, size, className, children, ...rest }: ButtonLinkProps) {
  return (
    <a className={buttonClass(variant, size, className)} {...rest}>
      {children}
    </a>
  );
}

interface SectionHeadingProps {
  eyebrow: string;
  title: string;
  lead?: string;
  id?: string;
  align?: "left" | "center";
  className?: string;
}

export function SectionHeading({
  eyebrow,
  title,
  lead,
  id,
  align = "left",
  className,
}: SectionHeadingProps) {
  return (
    <div className={cn("max-w-2xl", align === "center" && "mx-auto text-center", className)}>
      <p className="mb-3 text-sm font-semibold tracking-wide text-accent">{eyebrow}</p>
      <h2 id={id} className="text-h2 font-semibold text-balance">
        {title}
      </h2>
      {lead && <p className="mt-4 text-lead text-pretty text-fg-2">{lead}</p>}
    </div>
  );
}

/** Largura e respiro padrão de cada seção. */
export function Container({ children, className }: { children: ReactNode; className?: string }) {
  return (
    <div className={cn("mx-auto w-full max-w-[72rem] px-5 sm:px-8", className)}>{children}</div>
  );
}
