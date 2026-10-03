import { toast, type ExternalToast } from "sonner";
import { create } from "zustand";
import { messageForError } from "@/i18n/errors";

export type ToastKind = "error" | "info" | "success";

export interface Toast {
  id: number;
  kind: ToastKind;
  message: string;
}

interface ToastState {
  toasts: Toast[];
  push: (kind: ToastKind, message: string) => number;
  dismiss: (id: number) => void;
}

/** Quanto tempo um aviso fica na tela. */
export const TOAST_MS = 6000;
/** No máximo estes avisos ao mesmo tempo (os mais antigos saem). */
const MAX_TOASTS = 4;

let nextId = 1;

type Show = (message: string, data: ExternalToast) => string | number;

const show: Record<ToastKind, Show> = {
  error: (m, d) => toast.error(m, d),
  info: (m, d) => toast.info(m, d),
  success: (m, d) => toast.success(m, d),
};

/**
 * Avisos na tela. O desenho e as animações são do sonner (`components/feedback/Toaster`); este
 * store guarda os avisos ativos para o resto do app e os testes. A T-608 acrescenta as
 * notificações do sistema.
 */
export const useToasts = create<ToastState>((set, get) => ({
  toasts: [],
  push: (kind, message) => {
    const id = nextId++;
    set((s) => {
      const toasts = [...s.toasts, { id, kind, message }];
      // Os que passam do limite saem da tela junto com o store.
      toasts.slice(0, -MAX_TOASTS).forEach((old) => toast.dismiss(old.id));
      return { toasts: toasts.slice(-MAX_TOASTS) };
    });
    show[kind](message, {
      id,
      duration: TOAST_MS,
      // Fechado pelo usuário ou por tempo: tira do store também.
      onDismiss: () => get().dismiss(id),
      onAutoClose: () => get().dismiss(id),
    });
    return id;
  },
  dismiss: (id) => {
    set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) }));
    toast.dismiss(id);
  },
}));

/** Mostra o erro de um command como aviso, com a mensagem traduzida. */
export function notifyError(error: unknown): void {
  useToasts.getState().push("error", messageForError(error));
}
