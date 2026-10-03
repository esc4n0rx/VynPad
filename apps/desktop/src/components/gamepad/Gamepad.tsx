import { useEffect, useMemo, useRef, type CSSProperties } from "react";
import { useTranslation } from "react-i18next";
import { getLiveReport, useLiveSubscription } from "@/lib/api/live";
import type { InputReportDto } from "@/lib/bindings";
import { cn } from "@/lib/utils";
import type { GamepadElement } from "./elements";
import { GamepadSvg } from "./GamepadSvg";
import { createGamepadRenderer } from "./renderer";
import { onFrame } from "./ticker";

export type GamepadMode = "live" | "interactive" | "mini";

interface GamepadProps {
  /**
   * - `live`: acende os botões, desloca os analógicos e enche os gatilhos com o input.
   * - `interactive`: cada elemento é selecionável (editor de mapeamento).
   * - `mini`: versão compacta (cards, listas), sem as letras e detalhes.
   */
  mode?: GamepadMode;
  /** Slot (0–3) cujo input ao vivo desenhar. Assina o backend enquanto estiver montado. */
  slot?: number;
  /** Estado fixo (sobrepõe o `slot`): pré-visualização, testes e listas sem assinatura. */
  report?: InputReportDto;
  /** Cor do slot como valor CSS (ex.: `slotColor(0)`); padrão: a cor de destaque. */
  color?: string;
  /** Elemento selecionado (modo `interactive`). */
  selected?: GamepadElement;
  onSelect?: (element: GamepadElement) => void;
  className?: string;
}

/**
 * O controle X360. A atualização acontece por `requestAnimationFrame`, lendo o estado de um store
 * fora do React: o componente não re-renderiza a 60 Hz.
 */
export function Gamepad({
  mode = "live",
  slot,
  report,
  color,
  selected,
  onSelect,
  className,
}: GamepadProps) {
  const { t } = useTranslation();
  const wrapperRef = useRef<HTMLDivElement>(null);

  // Só assina o backend quando o desenho vai de fato seguir o slot.
  useLiveSubscription(report === undefined && slot !== undefined ? slot : null);

  useEffect(() => {
    const svg = wrapperRef.current?.querySelector("svg");
    if (!svg) return;
    const renderer = createGamepadRenderer(svg);
    if (report !== undefined || slot === undefined) {
      renderer.apply(report);
      return;
    }
    renderer.apply(getLiveReport(slot));
    return onFrame(() => {
      renderer.apply(getLiveReport(slot));
    });
  }, [slot, report]);

  const interaction = useMemo(
    () => (mode === "interactive" && onSelect ? { selected, onSelect } : undefined),
    [mode, selected, onSelect],
  );

  const style = color ? ({ "--pad-color": color } as CSSProperties) : undefined;

  return (
    <div
      ref={wrapperRef}
      role={interaction ? "group" : "img"}
      aria-label={t("gamepad.label")}
      data-mode={mode}
      style={style}
      className={cn("gamepad", className)}
    >
      <GamepadSvg interaction={interaction} className="h-auto w-full" />
    </div>
  );
}
