import type { ComponentProps } from "react";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "@/lib/utils";

const buttonVariants = cva(
  "focus-ring inline-flex shrink-0 items-center justify-center gap-1.5 rounded-md font-medium whitespace-nowrap transition-colors select-none disabled:pointer-events-none disabled:opacity-40 [&_svg]:size-4 [&_svg]:shrink-0",
  {
    variants: {
      variant: {
        primary: "bg-accent text-on-accent hover:bg-accent-hover",
        secondary: "bg-control text-fg hover:bg-control-hover",
        ghost: "text-fg-2 hover:bg-control hover:text-fg",
        destructive: "bg-danger/15 text-danger hover:bg-danger/25",
      },
      size: {
        sm: "h-7 px-2.5 text-caption",
        md: "h-8 px-3.5 text-body",
        lg: "h-10 px-5 text-subtitle",
      },
    },
    defaultVariants: { variant: "secondary", size: "md" },
  },
);

export type ButtonProps = ComponentProps<"button"> & VariantProps<typeof buttonVariants>;

export function Button({ className, variant, size, type = "button", ...props }: ButtonProps) {
  return (
    <button type={type} className={cn(buttonVariants({ variant, size }), className)} {...props} />
  );
}

const iconButtonSizes = { sm: "size-7", md: "size-8", lg: "size-10" } as const;

export type IconButtonProps = Omit<ButtonProps, "children" | "aria-label"> & {
  /** Obrigatório: botão só de ícone não tem texto para o leitor de tela. */
  label: string;
  children: ComponentProps<"button">["children"];
};

/** Botão quadrado só com ícone; o `label` vira `aria-label` e tooltip nativo. */
export function IconButton({
  label,
  className,
  variant = "ghost",
  size = "md",
  children,
  ...props
}: IconButtonProps) {
  return (
    <Button
      aria-label={label}
      title={label}
      variant={variant}
      size={size}
      className={cn("px-0", iconButtonSizes[size ?? "md"], className)}
      {...props}
    >
      {children}
    </Button>
  );
}
