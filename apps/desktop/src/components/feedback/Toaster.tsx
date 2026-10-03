import { Toaster as Sonner } from "sonner";
import { useTranslation } from "react-i18next";

const TOAST_CLASS_NAMES = {
  toast: "glass flex w-80 items-start gap-2 rounded-lg px-4 py-3 text-body text-fg shadow-popover",
  error: "text-danger",
  success: "text-success",
  info: "text-info",
  title: "text-fg",
};

/** Região de avisos (sonner), com o visual de vidro do design system. */
export function Toaster() {
  const { t } = useTranslation();

  return (
    <Sonner
      position="bottom-right"
      visibleToasts={4}
      containerAriaLabel={t("toast.region")}
      toastOptions={{ unstyled: true, classNames: TOAST_CLASS_NAMES }}
    />
  );
}
