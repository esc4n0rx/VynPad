import { useTranslation } from "react-i18next";
import { Maximize2, Minimize2, Minus, X } from "lucide-react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useWindowState } from "@/lib/useWindowState";
import { cn } from "@/lib/utils";

interface TitleBarProps {
  /** Título centralizado (opcional). */
  title?: string;
}

/**
 * Barra de título de 44 px: arrasta a janela, duplo clique maximiza/restaura e o semáforo fica à
 * esquerda, com os ícones aparecendo no hover e cinza quando a janela perde o foco.
 *
 * O Snap Layouts do Windows 11 (hover no botão de maximizar) não é viável aqui: ele só aparece
 * para botões que o sistema reconhece como área de legenda (`HTMLMAXBUTTON`), e o Tauri não
 * expõe isso a uma webview sem moldura.
 */
export function TitleBar({ title }: TitleBarProps) {
  const { t } = useTranslation();
  const { focused, maximized } = useWindowState();
  const appWindow = getCurrentWindow();

  const lights = [
    {
      label: t("window.close"),
      color: "bg-traffic-close",
      // Inativa fica cinza; o hover no grupo devolve a cor (classe literal para o Tailwind achar).
      hover: "group-hover:bg-traffic-close",
      icon: <X />,
      // Com a bandeja ativa (T-607) fechar passa a esconder a janela em vez de encerrar.
      action: () => void appWindow.close(),
    },
    {
      label: t("window.minimize"),
      color: "bg-traffic-minimize",
      // Inativa fica cinza; o hover no grupo devolve a cor (classe literal para o Tailwind achar).
      hover: "group-hover:bg-traffic-minimize",
      icon: <Minus />,
      action: () => void appWindow.minimize(),
    },
    {
      label: maximized ? t("window.restore") : t("window.maximize"),
      color: "bg-traffic-maximize",
      // Inativa fica cinza; o hover no grupo devolve a cor (classe literal para o Tailwind achar).
      hover: "group-hover:bg-traffic-maximize",
      icon: maximized ? <Minimize2 /> : <Maximize2 />,
      action: () => void appWindow.toggleMaximize(),
    },
  ];

  return (
    <header
      data-tauri-drag-region
      onDoubleClick={() => void appWindow.toggleMaximize()}
      className="relative flex h-11 shrink-0 items-center px-4"
    >
      <div className="group flex items-center gap-2">
        {lights.map((light) => (
          <button
            key={light.label}
            type="button"
            aria-label={light.label}
            title={light.label}
            onClick={light.action}
            onDoubleClick={(e) => {
              e.stopPropagation();
            }}
            className={cn(
              "flex size-3 items-center justify-center rounded-full text-traffic-glyph focus-ring transition-colors [&_svg]:size-2 [&_svg]:stroke-[3] [&_svg]:opacity-0 group-hover:[&_svg]:opacity-100",
              focused ? light.color : cn("bg-traffic-inactive", light.hover),
            )}
          >
            {light.icon}
          </button>
        ))}
      </div>
      {title && (
        <span
          data-tauri-drag-region
          className="pointer-events-none absolute inset-x-0 text-center text-body font-medium text-fg-2"
        >
          {title}
        </span>
      )}
    </header>
  );
}
