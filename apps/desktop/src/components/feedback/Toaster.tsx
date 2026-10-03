import { Toaster as Sonner } from "sonner";

/** Região de avisos (sonner), com o visual de vidro do design system. */
export function Toaster() {
  return (
    <Sonner
      position="bottom-right"
      visibleToasts={4}
      toastOptions={{
        unstyled: true,
        classNames: {
          toast:
            "glass flex w-80 items-start gap-2 rounded-lg px-4 py-3 text-body text-fg shadow-popover",
          error: "text-danger",
          success: "text-success",
          info: "text-info",
          title: "text-fg",
        },
      }}
    />
  );
}
