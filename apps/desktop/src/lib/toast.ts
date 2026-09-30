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

/** Avisos na tela. Provisório: a T-608 traz o design final e as notificações do sistema. */
export const useToasts = create<ToastState>((set) => ({
  toasts: [],
  push: (kind, message) => {
    const id = nextId++;
    set((s) => ({ toasts: [...s.toasts, { id, kind, message }].slice(-MAX_TOASTS) }));
    return id;
  },
  dismiss: (id) => {
    set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) }));
  },
}));

/** Mostra o erro de um command como aviso, com a mensagem traduzida. */
export function notifyError(error: unknown): void {
  useToasts.getState().push("error", messageForError(error));
}
