import { useEffect } from "react";
import { useNavigate } from "react-router";
import { commands, events } from "@/lib/bindings";
import { log } from "@/lib/log";

/**
 * A bandeja pode pedir uma tela ("Parear novo dispositivo", "Configurações"). A janela talvez nem
 * existisse quando o menu foi clicado, então o pedido fica guardado no backend: a UI o lê ao abrir
 * e de novo a cada aviso (`tray:navigate`).
 */
export function useTrayNavigation(): void {
  const navigate = useNavigate();

  useEffect(() => {
    let active = true;
    const go = () => {
      commands
        .takePendingNavigation()
        .then((path) => {
          if (active && path) void navigate(path);
        })
        .catch((error: unknown) => {
          log.warn(`não consegui ler a navegação pedida pela bandeja: ${JSON.stringify(error)}`);
        });
    };
    go();
    const unlisten = events.trayNavigate.listen(go);
    return () => {
      active = false;
      void unlisten.then((stop) => {
        stop();
      });
    };
  }, [navigate]);
}
