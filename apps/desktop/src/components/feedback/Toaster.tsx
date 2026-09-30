import { useEffect } from "react";
import { TOAST_MS, useToasts } from "@/lib/toast";

/** Lista de avisos (região `aria-live`, para leitores de tela). */
export function Toaster() {
  const toasts = useToasts((s) => s.toasts);
  const dismiss = useToasts((s) => s.dismiss);

  useEffect(() => {
    const timers = toasts.map((t) => setTimeout(() => dismiss(t.id), TOAST_MS));
    return () => {
      timers.forEach(clearTimeout);
    };
  }, [toasts, dismiss]);

  return (
    <div
      role="status"
      aria-live="polite"
      className="pointer-events-none fixed right-4 bottom-4 z-50 flex w-80 flex-col gap-2"
    >
      {toasts.map((t) => (
        <div
          key={t.id}
          className="pointer-events-auto rounded-xl border border-white/10 bg-neutral-900/90 px-4 py-3 text-sm text-white shadow-lg backdrop-blur"
        >
          <p className={t.kind === "error" ? "text-red-300" : undefined}>{t.message}</p>
        </div>
      ))}
    </div>
  );
}
