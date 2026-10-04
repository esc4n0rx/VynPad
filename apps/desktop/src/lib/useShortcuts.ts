import { useEffect } from "react";
import { useNavigate } from "react-router";
import { pathForShortcut } from "@/app/navigation";

/** `Ctrl+1..5` abre as seções, `Ctrl+,` as Configurações (`Cmd` vale como `Ctrl`) e `F1` a ajuda. */
export function useNavigationShortcuts(): void {
  const navigate = useNavigate();

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      // F1 abre a ajuda.
      if (event.key === "F1" && !event.ctrlKey && !event.metaKey && !event.altKey) {
        event.preventDefault();
        void navigate("/help");
        return;
      }
      if (!(event.ctrlKey || event.metaKey) || event.altKey || event.shiftKey) return;
      const path = pathForShortcut(event.key);
      if (path === undefined) return;
      event.preventDefault();
      void navigate(path);
    };
    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [navigate]);
}
