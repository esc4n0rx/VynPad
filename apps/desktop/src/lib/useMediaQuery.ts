import { useSyncExternalStore } from "react";

/** Acompanha uma media query (ex.: `(max-width: 999px)`) e re-renderiza quando ela muda. */
export function useMediaQuery(query: string): boolean {
  return useSyncExternalStore(
    (onChange) => {
      const list = window.matchMedia(query);
      list.addEventListener("change", onChange);
      return () => {
        list.removeEventListener("change", onChange);
      };
    },
    () => window.matchMedia(query).matches,
  );
}
