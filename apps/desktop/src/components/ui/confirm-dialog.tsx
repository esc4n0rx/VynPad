import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "./button";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogTitle,
} from "./dialog";

interface ConfirmDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: ReactNode;
  description: ReactNode;
  confirmLabel: string;
  cancelLabel: string;
  /** Ação irreversível: o botão de confirmar fica vermelho. */
  destructive?: boolean;
  onConfirm: () => void;
}

/** Confirmação antes de uma ação sem volta (revogar, esquecer, apagar). */
export function ConfirmDialog({
  open,
  onOpenChange,
  title,
  description,
  confirmLabel,
  cancelLabel,
  destructive,
  onConfirm,
}: ConfirmDialogProps) {
  const { t } = useTranslation();
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent closeLabel={t("ui.close")}>
        <DialogTitle>{title}</DialogTitle>
        <DialogDescription>{description}</DialogDescription>
        <DialogFooter>
          <DialogClose asChild>
            <Button variant="secondary">{cancelLabel}</Button>
          </DialogClose>
          <Button
            variant={destructive ? "destructive" : "primary"}
            onClick={() => {
              onConfirm();
              onOpenChange(false);
            }}
          >
            {confirmLabel}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
