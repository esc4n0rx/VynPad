import type { ComponentProps } from "react";
import { X } from "lucide-react";
import { Dialog as DialogPrimitive } from "radix-ui";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "@/lib/utils";
import { IconButton } from "./button";

export const Dialog = DialogPrimitive.Root;
export const DialogTrigger = DialogPrimitive.Trigger;
export const DialogClose = DialogPrimitive.Close;

const contentVariants = cva(
  "glass fixed z-50 flex flex-col gap-3 shadow-popover outline-none data-[state=closed]:animate-out data-[state=open]:animate-in",
  {
    variants: {
      // Modal centralizado ou sheet encostado na direita.
      placement: {
        center:
          "top-1/2 left-1/2 w-[min(28rem,calc(100vw-2rem))] -translate-x-1/2 -translate-y-1/2 rounded-xl p-5 data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0",
        sheet:
          "inset-y-0 right-0 w-[min(24rem,100vw)] rounded-l-xl p-5 data-[state=closed]:slide-out-to-right data-[state=open]:slide-in-from-right",
      },
    },
    defaultVariants: { placement: "center" },
  },
);

type DialogContentProps = ComponentProps<typeof DialogPrimitive.Content> &
  VariantProps<typeof contentVariants> & { closeLabel?: string };

/** Modal (`placement="center"`) ou sheet (`placement="sheet"`) com backdrop em blur. */
export function DialogContent({
  className,
  placement,
  closeLabel = "Fechar",
  children,
  ...props
}: DialogContentProps) {
  return (
    <DialogPrimitive.Portal>
      <DialogPrimitive.Overlay className="fixed inset-0 z-50 bg-overlay backdrop-blur-sm data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:animate-in data-[state=open]:fade-in-0" />
      <DialogPrimitive.Content className={cn(contentVariants({ placement }), className)} {...props}>
        {children}
        <DialogPrimitive.Close asChild>
          <IconButton label={closeLabel} size="sm" className="absolute top-3 right-3">
            <X strokeWidth={1.75} />
          </IconButton>
        </DialogPrimitive.Close>
      </DialogPrimitive.Content>
    </DialogPrimitive.Portal>
  );
}

export function DialogTitle({ className, ...props }: ComponentProps<typeof DialogPrimitive.Title>) {
  return (
    <DialogPrimitive.Title
      className={cn("pr-8 text-title font-semibold text-fg", className)}
      {...props}
    />
  );
}

export function DialogDescription({
  className,
  ...props
}: ComponentProps<typeof DialogPrimitive.Description>) {
  return (
    <DialogPrimitive.Description className={cn("text-body text-fg-2", className)} {...props} />
  );
}

export function DialogFooter({ className, ...props }: ComponentProps<"div">) {
  return <div className={cn("mt-2 flex justify-end gap-2", className)} {...props} />;
}
