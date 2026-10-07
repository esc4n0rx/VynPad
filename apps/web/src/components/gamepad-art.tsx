import type { CSSProperties } from "react";
import styles from "./gamepad-art.module.css";

interface GamepadArtProps {
  /** Variável CSS da cor do slot (`--slot-1`...): o que acende ao apertar. */
  color: string;
  /** Segundos de defasagem na animação, para dois controles não apertarem juntos. */
  offset?: number;
  /** Mostra as letras A B X Y (no celular grande; no card pequeno ficam de fora). */
  labels?: boolean;
  /** Sem animação (controle ocioso). */
  still?: boolean;
  className?: string;
}

/**
 * O controle do app, desenhado em SVG: dois analógicos, direcional, ABXY, ombros e gatilhos.
 * Os botões acendem em sequência (loop de 8 s) só com CSS, então o mesmo desenho serve ao celular
 * e ao card do desktop, em sincronia, sem JavaScript.
 */
export function GamepadArt({
  color,
  offset = 0,
  labels = false,
  still = false,
  className,
}: GamepadArtProps) {
  const style = { "--c": `var(${color})`, "--o": `${String(offset)}s` } as CSSProperties;
  const cls = (name: string) => `${styles[name] ?? ""} ${still ? "" : (styles.live ?? "")}`;
  return (
    <svg
      viewBox="0 0 200 100"
      className={`${styles.pad ?? ""} ${className ?? ""}`}
      style={style}
      aria-hidden
      focusable="false"
    >
      {/* Gatilhos e ombros */}
      <rect className={styles.shell} x="6" y="3" width="30" height="11" rx="5" />
      <rect className={styles.shell} x="40" y="3" width="28" height="11" rx="5" />
      <rect className={styles.shell} x="132" y="3" width="28" height="11" rx="5" />
      <rect
        className={`${styles.shell ?? ""} ${cls("rb")}`}
        x="132"
        y="3"
        width="28"
        height="11"
        rx="5"
      />
      <rect className={styles.shell} x="164" y="3" width="30" height="11" rx="5" />
      <rect className={cls("trigger2")} x="164" y="3" width="30" height="11" rx="5" />

      {/* Analógico esquerdo */}
      <circle className={styles.well} cx="36" cy="46" r="17" />
      <g className={cls("knobL")}>
        <circle className={styles.cap} cx="36" cy="46" r="9" />
        <circle className={styles.capRing} cx="36" cy="46" r="5" />
      </g>

      {/* Direcional */}
      <g className={styles.dpad}>
        <rect className={styles.shell} x="57" y="60" width="8" height="12" rx="2.5" />
        <rect className={styles.shell} x="57" y="80" width="8" height="12" rx="2.5" />
        <rect className={styles.shell} x="45" y="72" width="12" height="8" rx="2.5" />
        <rect className={styles.shell} x="65" y="72" width="12" height="8" rx="2.5" />
        <rect className={cls("dpadRight")} x="65" y="72" width="12" height="8" rx="2.5" />
      </g>

      {/* Botões centrais */}
      <circle className={styles.shell} cx="88" cy="40" r="3.6" />
      <circle className={styles.shell} cx="100" cy="36" r="5" />
      <circle className={styles.shell} cx="112" cy="40" r="3.6" />

      {/* Analógico direito */}
      <circle className={styles.well} cx="128" cy="74" r="13" />
      <g className={cls("knobR")}>
        <circle className={styles.cap} cx="128" cy="74" r="7" />
        <circle className={styles.capRing} cx="128" cy="74" r="3.8" />
      </g>

      {/* ABXY */}
      <circle className={styles.well} cx="166" cy="46" r="26" />
      <circle className={`${styles.shell ?? ""} ${cls("y")}`} cx="166" cy="29" r="8.5" />
      <circle className={styles.shell} cx="149" cy="46" r="8.5" />
      <circle className={`${styles.shell ?? ""} ${cls("b")}`} cx="183" cy="46" r="8.5" />
      <circle className={`${styles.shell ?? ""} ${cls("a")}`} cx="166" cy="63" r="8.5" />
      {labels && (
        <g className={styles.glyph}>
          <text x="166" y="32" textAnchor="middle">
            Y
          </text>
          <text x="149" y="49" textAnchor="middle">
            X
          </text>
          <text x="183" y="49" textAnchor="middle">
            B
          </text>
          <text x="166" y="66" textAnchor="middle">
            A
          </text>
        </g>
      )}
    </svg>
  );
}
