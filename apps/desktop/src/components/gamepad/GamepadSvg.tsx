import { memo, useId, type KeyboardEvent, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import type { GamepadElement } from "./elements";

/** Letras gravadas nos botões (não são texto traduzível). */
const GLYPH = { a: "A", b: "B", x: "X", y: "Y", lb: "LB", rb: "RB", lt: "LT", rt: "RT" } as const;

interface Interaction {
  selected: GamepadElement | undefined;
  onSelect: (element: GamepadElement) => void;
}

interface ElProps {
  id: GamepadElement;
  interaction: Interaction | undefined;
  children: ReactNode;
}

/**
 * Um elemento nomeado do controle (`data-el`). No modo interativo vira um botão focável, com o
 * nome como tooltip nativo (`<title>`); no modo ao vivo é só desenho.
 */
function El({ id, interaction, children }: ElProps) {
  const { t } = useTranslation("mapping");
  if (!interaction) return <g data-el={id}>{children}</g>;

  const label = t(`elements.${id}`);
  const select = (event: { stopPropagation: () => void }) => {
    // O cap do analógico fica dentro do grupo do analógico: o clique vale só para o mais interno.
    event.stopPropagation();
    interaction.onSelect(id);
  };
  const onKeyDown = (event: KeyboardEvent) => {
    if (event.key !== "Enter" && event.key !== " ") return;
    event.preventDefault();
    select(event);
  };

  return (
    <g
      data-el={id}
      data-selected={interaction.selected === id}
      role="button"
      tabIndex={0}
      aria-label={label}
      aria-pressed={interaction.selected === id}
      onClick={select}
      onKeyDown={onKeyDown}
    >
      <title>{label}</title>
      {children}
    </g>
  );
}

function Stick({
  side,
  cx,
  cy,
  interaction,
}: {
  side: "left" | "right";
  cx: number;
  cy: number;
  interaction: Interaction | undefined;
}) {
  const click = side === "left" ? "ls-click" : "rs-click";
  return (
    <El id={side === "left" ? "stick-left" : "stick-right"} interaction={interaction}>
      <circle className="pad-well" cx={cx} cy={cy} r={30} />
      <g data-cap={side}>
        <El id={click} interaction={interaction}>
          <circle className="pad-shape" cx={cx} cy={cy} r={21} />
          <circle className="pad-ring" cx={cx} cy={cy} r={12} />
        </El>
      </g>
    </El>
  );
}

function Face({
  id,
  glyph,
  cx,
  cy,
  interaction,
}: {
  id: "btn-a" | "btn-b" | "btn-x" | "btn-y";
  glyph: string;
  cx: number;
  cy: number;
  interaction: Interaction | undefined;
}) {
  return (
    <El id={id} interaction={interaction}>
      <circle className="pad-shape" cx={cx} cy={cy} r={12} />
      <text className="pad-glyph" x={cx} y={cy} dy="0.35em" textAnchor="middle" data-detail>
        {glyph}
      </text>
    </El>
  );
}

function Trigger({
  id,
  x,
  glyph,
  clip,
  interaction,
}: {
  id: "lt" | "rt";
  x: number;
  glyph: string;
  clip: string;
  interaction: Interaction | undefined;
}) {
  return (
    <El id={id} interaction={interaction}>
      <clipPath id={clip}>
        <rect x={x} y={18} width={60} height={32} rx={11} />
      </clipPath>
      <rect className="pad-shape" x={x} y={18} width={60} height={32} rx={11} />
      <rect
        className="pad-fill"
        x={x}
        y={18}
        width={60}
        height={32}
        clipPath={`url(#${clip})`}
        pointerEvents="none"
      />
      <text className="pad-glyph" x={x + 30} y={34} dy="0.35em" textAnchor="middle" data-detail>
        {glyph}
      </text>
    </El>
  );
}

function Bumper({
  id,
  x,
  glyph,
  interaction,
}: {
  id: "lb" | "rb";
  x: number;
  glyph: string;
  interaction: Interaction | undefined;
}) {
  return (
    <El id={id} interaction={interaction}>
      <rect className="pad-shape" x={x} y={55} width={84} height={17} rx={8.5} />
      <text className="pad-glyph" x={x + 42} y={63.5} dy="0.35em" textAnchor="middle" data-detail>
        {glyph}
      </text>
    </El>
  );
}

function Arm({
  id,
  x,
  y,
  width,
  height,
  interaction,
}: {
  id: "dpad-up" | "dpad-down" | "dpad-left" | "dpad-right";
  x: number;
  y: number;
  width: number;
  height: number;
  interaction: Interaction | undefined;
}) {
  return (
    <El id={id} interaction={interaction}>
      <rect className="pad-shape" x={x} y={y} width={width} height={height} rx={3} />
    </El>
  );
}

interface GamepadSvgProps {
  /** Quando presente, cada elemento vira um botão selecionável. */
  interaction?: Interaction;
  className?: string;
}

/**
 * O controle X360 em "flat glass": contorno suave, gradientes sutis e um grupo nomeado por
 * elemento (`data-el`: `btn-a`, `lb`, `lt`, `dpad-up`, `stick-left`, `ls-click`...). Só desenha;
 * quem o anima é o `createGamepadRenderer`, mexendo nos atributos sem re-renderizar.
 *
 * Ids de gradiente/clip levam um sufixo único: vários controles convivem na mesma tela.
 */
export const GamepadSvg = memo(function GamepadSvg({ interaction, className }: GamepadSvgProps) {
  const uid = useId().replace(/:/g, "");
  const body = `${uid}-body`;
  const shine = `${uid}-shine`;

  return (
    <svg
      viewBox="0 0 480 300"
      className={className}
      xmlns="http://www.w3.org/2000/svg"
      aria-hidden={interaction ? undefined : true}
    >
      <defs>
        <linearGradient id={body} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" className="pad-stop-top" />
          <stop offset="1" className="pad-stop-bottom" />
        </linearGradient>
        <linearGradient id={shine} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" className="pad-stop-shine" />
          <stop offset="1" className="pad-stop-clear" />
        </linearGradient>
      </defs>

      <Trigger id="lt" x={112} glyph={GLYPH.lt} clip={`${uid}-lt`} interaction={interaction} />
      <Trigger id="rt" x={308} glyph={GLYPH.rt} clip={`${uid}-rt`} interaction={interaction} />
      <Bumper id="lb" x={96} glyph={GLYPH.lb} interaction={interaction} />
      <Bumper id="rb" x={300} glyph={GLYPH.rb} interaction={interaction} />

      <path
        className="pad-body"
        fill={`url(#${body})`}
        d="M240 78C200 78 170 70 140 74C96 80 70 120 52 190C40 238 44 276 70 282C96 288 112 262 132 232C148 210 170 208 190 208L290 208C310 208 332 210 348 232C368 262 384 288 410 282C436 276 440 238 428 190C410 120 384 80 340 74C310 70 280 78 240 78Z"
      />
      <path
        className="pad-shine"
        fill={`url(#${shine})`}
        d="M240 82C204 82 174 75 142 79C108 84 84 118 68 176C112 120 170 112 240 112C310 112 368 120 412 176C396 118 372 84 338 79C306 75 276 82 240 82Z"
        pointerEvents="none"
      />

      <Stick side="left" cx={138} cy={124} interaction={interaction} />
      <Stick side="right" cx={292} cy={172} interaction={interaction} />

      <g transform="translate(188 172)">
        <path
          className="pad-well"
          d="M-10 -28h20v18h18v20h-18v18h-20v-18h-18v-20h18z"
          pointerEvents="none"
        />
        <Arm id="dpad-up" x={-10} y={-28} width={20} height={22} interaction={interaction} />
        <Arm id="dpad-down" x={-10} y={6} width={20} height={22} interaction={interaction} />
        <Arm id="dpad-left" x={-28} y={-10} width={22} height={20} interaction={interaction} />
        <Arm id="dpad-right" x={6} y={-10} width={22} height={20} interaction={interaction} />
      </g>

      <Face id="btn-y" glyph={GLYPH.y} cx={342} cy={98} interaction={interaction} />
      <Face id="btn-a" glyph={GLYPH.a} cx={342} cy={150} interaction={interaction} />
      <Face id="btn-x" glyph={GLYPH.x} cx={316} cy={124} interaction={interaction} />
      <Face id="btn-b" glyph={GLYPH.b} cx={368} cy={124} interaction={interaction} />

      <El id="guide" interaction={interaction}>
        <circle className="pad-shape" cx={240} cy={102} r={14} />
        <circle className="pad-ring" cx={240} cy={102} r={7} data-detail />
      </El>
      <El id="back" interaction={interaction}>
        <circle className="pad-shape" cx={210} cy={126} r={7} />
      </El>
      <El id="start" interaction={interaction}>
        <circle className="pad-shape" cx={270} cy={126} r={7} />
      </El>
    </svg>
  );
});
