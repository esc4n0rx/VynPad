import { useEffect } from "react";
import { useNavigate } from "react-router";
import { pathForShortcut } from "@/app/navigation";

/** `Ctrl+1..5` abre as seções e `Ctrl+,` as Configurações (`Cmd` vale como `Ctrl`). */
export function useNavigationShortcuts(): void {
  const navigate = useNavigate();

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
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
