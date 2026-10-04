import { useCallback, useEffect, useRef, useState } from "react";
import { useSaveProfile } from "@/lib/api/profiles";
import type { Profile, ProfileDto } from "@/lib/bindings";

/** Espera depois da última alteração antes de gravar (T-604). */
export const SAVE_DEBOUNCE_MS = 300;
/** Alterações em sequência (arrastar um slider) valem um passo só do desfazer. */
export const UNDO_COALESCE_MS = 600;
const UNDO_LIMIT = 100;

export type SaveState = "idle" | "pending" | "saving" | "saved" | "error";

interface Draft {
  name: string;
  description: string | null;
  icon: string | null;
  profile: Profile;
}

/**
 * A sessão de edição de um perfil: o rascunho local, o salvamento automático (debounce de
 * 300 ms, aplicado na hora nos controles que usam o perfil) e o desfazer (Ctrl+Z).
 *
 * O perfil embutido é só de leitura: o backend recusa gravá-lo, então a tela oferece duplicar.
 */
export function useProfileEditor(dto: ProfileDto) {
  const { id } = dto.summary;
  const readOnly = dto.summary.is_builtin;
  const [draft, setDraft] = useState<Draft>({
    name: dto.summary.name,
    description: dto.summary.description,
    icon: dto.summary.icon,
    profile: dto.profile,
  });
  const [saveState, setSaveState] = useState<SaveState>("idle");
  const [canUndo, setCanUndo] = useState(false);
  const latest = useRef(draft);
  const past = useRef<Draft[]>([]);
  const lastEdit = useRef(0);
  const timer = useRef<number | null>(null);
  const { mutate } = useSaveProfile();

  const flush = useCallback(() => {
    if (timer.current === null) return;
    window.clearTimeout(timer.current);
    timer.current = null;
    setSaveState("saving");
    mutate(
      { id, input: latest.current },
      {
        onSuccess: () => {
          setSaveState(timer.current === null ? "saved" : "pending");
        },
        onError: () => {
          setSaveState("error");
        },
      },
    );
  }, [id, mutate]);

  const commit = useCallback(
    (next: Draft) => {
      latest.current = next;
      setDraft(next);
      setSaveState("pending");
      if (timer.current !== null) window.clearTimeout(timer.current);
      timer.current = window.setTimeout(flush, SAVE_DEBOUNCE_MS);
    },
    [flush],
  );

  const remember = useCallback(() => {
    const now = Date.now();
    if (now - lastEdit.current > UNDO_COALESCE_MS) {
      past.current.push(latest.current);
      if (past.current.length > UNDO_LIMIT) past.current.shift();
      setCanUndo(true);
    }
    lastEdit.current = now;
  }, []);

  /** Altera o perfil de mapeamento (botões, analógicos, gatilhos, gyro). */
  const edit = useCallback(
    (change: (profile: Profile) => Profile) => {
      if (readOnly) return;
      remember();
      commit({ ...latest.current, profile: change(latest.current.profile) });
    },
    [commit, readOnly, remember],
  );

  /** Altera nome, descrição ou ícone. */
  const editMeta = useCallback(
    (patch: Partial<Pick<Draft, "name" | "description" | "icon">>) => {
      if (readOnly) return;
      remember();
      commit({ ...latest.current, ...patch });
    },
    [commit, readOnly, remember],
  );

  const undo = useCallback(() => {
    const previous = past.current.pop();
    if (!previous) return;
    setCanUndo(past.current.length > 0);
    // O próximo gesto começa um passo novo.
    lastEdit.current = 0;
    commit(previous);
  }, [commit]);

  // Trocar de perfil ou sair da tela grava o que ainda estava esperando o debounce.
  useEffect(() => flush, [flush]);

  return { draft, readOnly, saveState, canUndo, edit, editMeta, undo, flush };
}
