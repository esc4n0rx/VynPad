import { getCurrentWindow } from "@tauri-apps/api/window";

const appWindow = getCurrentWindow();

const buttons = [
  { label: "Fechar", color: "bg-traffic-close", action: () => void appWindow.close() },
  { label: "Minimizar", color: "bg-traffic-minimize", action: () => void appWindow.minimize() },
  {
    label: "Maximizar",
    color: "bg-traffic-maximize",
    action: () => void appWindow.toggleMaximize(),
  },
];

/**
 * Barra de título provisória: permite arrastar, fechar, minimizar e maximizar
 * a janela sem decorações. O visual final (semáforo com hover, estado inativo) é da T-502.
 */
export function TitleBar() {
  return (
    <header
      data-tauri-drag-region
      onDoubleClick={() => void appWindow.toggleMaximize()}
      className="flex h-11 shrink-0 items-center gap-2 px-4"
    >
      {buttons.map((b) => (
        <button
          key={b.label}
          type="button"
          aria-label={b.label}
          title={b.label}
          onClick={b.action}
          onDoubleClick={(e) => e.stopPropagation()}
          className={`size-3 rounded-full ${b.color}`}
        />
      ))}
    </header>
  );
}
