import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";

export interface WindowState {
  focused: boolean;
  maximized: boolean;
}

/**
 * Foco e maximização da janela. Também grava `data-maximized` no `<html>`, para o CSS tirar o
 * arredondamento quando a janela ocupa a tela toda.
 */
export function useWindowState(): WindowState {
  const [state, setState] = useState<WindowState>({ focused: true, maximized: false });

  useEffect(() => {
    const appWindow = getCurrentWindow();
    let disposed = false;
    const unlisten: (() => void)[] = [];

    const syncMaximized = () => {
      appWindow
        .isMaximized()
        .then((maximized) => {
          if (disposed) return;
          setState((s) => ({ ...s, maximized }));
        })
        .catch(() => undefined);
    };

    const subscribe = (pending: Promise<() => void>) => {
      pending
        .then((stop) => {
          if (disposed) stop();
          else unlisten.push(stop);
        })
        .catch(() => undefined);
    };

    syncMaximized();
    subscribe(appWindow.onResized(syncMaximized));
    subscribe(
      appWindow.onFocusChanged(({ payload: focused }) => {
        setState((s) => ({ ...s, focused }));
      }),
    );

    return () => {
      disposed = true;
      unlisten.forEach((stop) => {
        stop();
      });
    };
  }, []);

  useEffect(() => {
    document.documentElement.dataset.maximized = String(state.maximized);
  }, [state.maximized]);

  return state;
}
