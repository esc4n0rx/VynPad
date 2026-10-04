import { useEffect, useRef, useState } from "react";

/**
 * Um valor que o usuário arrasta (slider) e que só deve ir ao backend quando ele para: devolve o
 * valor local e uma função para mudá-lo; `commit` roda `ms` depois da última mudança.
 */
export function useDebouncedValue<T>(
  saved: T,
  commit: (value: T) => void,
  ms = 300,
): [T, (value: T) => void] {
  const [local, setLocal] = useState(saved);
  const timer = useRef<number | null>(null);
  const latest = useRef(commit);
  useEffect(() => {
    latest.current = commit;
  });
  // Mudou por fora (outra janela, ou o backend corrigiu): acompanha, se não está arrastando.
  const [lastSaved, setLastSaved] = useState(saved);
  if (saved !== lastSaved) {
    setLastSaved(saved);
    if (timer.current === null) setLocal(saved);
  }
  useEffect(
    () => () => {
      if (timer.current !== null) window.clearTimeout(timer.current);
    },
    [],
  );

  const change = (value: T) => {
    setLocal(value);
    if (timer.current !== null) window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => {
      timer.current = null;
      latest.current(value);
    }, ms);
  };
  return [local, change];
}
