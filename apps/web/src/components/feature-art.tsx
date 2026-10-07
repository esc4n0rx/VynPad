import { CloudOff, Lock, Monitor, Smartphone, Wifi } from "lucide-react";
import type { CSSProperties } from "react";
import type { Dictionary } from "@/i18n";
import { fmt } from "@/i18n/config";
import styles from "./feature-art.module.css";

const cssVars = (vars: Record<string, string>) => vars as CSSProperties;

// ---------------------------------------------------------------------------- latência

const CHART_W = 400;
const CHART_H = 130;

/** Duas voltas da mesma onda (a segunda emenda na primeira), para o gráfico rolar sem salto. */
function latencyPath(): { line: string; area: string } {
  const points: string[] = [];
  for (let x = 0; x <= CHART_W * 2; x += 5) {
    const t = (x / CHART_W) * Math.PI * 2;
    let y = 96 + 6 * Math.sin(t * 2) + 3.5 * Math.sin(t * 5 + 1) + 2 * Math.sin(t * 11);
    // Dois soluços por volta, para o gráfico não parecer uma senoide.
    const phase = x % CHART_W;
    if (phase >= 150 && phase <= 175) y -= 16 * Math.sin(((phase - 150) / 25) * Math.PI);
    if (phase >= 305 && phase <= 325) y -= 9 * Math.sin(((phase - 305) / 20) * Math.PI);
    points.push(`${x === 0 ? "M" : "L"} ${String(x)} ${y.toFixed(1)}`);
  }
  const line = points.join(" ");
  return {
    line,
    area: `${line} L ${String(CHART_W * 2)} ${String(CHART_H)} L 0 ${String(CHART_H)} Z`,
  };
}

const LATENCY = latencyPath();

export function LatencyArt({ label }: { label: string }) {
  return (
    <div className={styles.art} role="img" aria-label={label}>
      <svg
        className={styles.chart}
        viewBox={`0 0 ${String(CHART_W)} ${String(CHART_H)}`}
        preserveAspectRatio="none"
      >
        <defs>
          <linearGradient id="latency-fill" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0" stopColor="var(--accent)" stopOpacity="0.32" />
            <stop offset="1" stopColor="var(--accent)" stopOpacity="0" />
          </linearGradient>
        </defs>
        <line className={styles.goal} x1="0" x2={CHART_W} y1="26" y2="26" />
        <text className={styles.goalLabel} x="6" y="19">
          10 ms
        </text>
        <g className={`${styles.chartScroll ?? ""} motion-scroll`}>
          <path className={styles.chartArea} d={LATENCY.area} />
          <path className={styles.chartLine} d={LATENCY.line} vectorEffect="non-scaling-stroke" />
        </g>
      </svg>
    </div>
  );
}

// ---------------------------------------------------------------------------- slots

export function SlotsArt({ dict }: { dict: Pick<Dictionary, "hero"> }) {
  const delays = ["0s", "1.6s", "3.2s", "4.8s"];
  return (
    <div className={styles.slots} aria-hidden>
      {[1, 2, 3, 4].map((n, i) => (
        <div
          key={n}
          className={styles.slotPill}
          style={cssVars({ "--c": `var(--slot-${String(n)})`, "--d": delays[i] ?? "0s" })}
        >
          <span className={styles.slotDot} />
          <span className={styles.slotName}>P{n}</span>
          <span className="truncate text-xs font-medium text-fg-3">
            {fmt(dict.hero.scene.player, { n })}
          </span>
        </div>
      ))}
    </div>
  );
}

// ---------------------------------------------------------------------------- gyro

export function GyroArt({ labels }: { labels: { modeTilt: string } }) {
  return (
    <div className={styles.gyro} aria-hidden>
      <div className={styles.gyroStage}>
        <svg className={styles.gyroArc} viewBox="0 0 120 60">
          <path d="M 6 56 A 54 54 0 0 1 114 56" />
        </svg>
        <div className={styles.gyroPhone} />
        <div className={styles.gyroTrack}>
          <span className={styles.gyroDot} />
        </div>
      </div>
      <span className="sr-only">{labels.modeTilt}</span>
    </div>
  );
}

// ---------------------------------------------------------------------------- vibração

export function RumbleArt() {
  return (
    <div className={styles.rumble} aria-hidden>
      <div className={styles.rumbleStage}>
        <span
          className={`${styles.wave ?? ""} ${styles.waveL ?? ""}`}
          style={cssVars({ "--d": "0s" })}
        />
        <span
          className={`${styles.wave ?? ""} ${styles.waveL ?? ""}`}
          style={cssVars({ "--d": "0.8s" })}
        />
        <div className={styles.rumblePhone} />
        <span
          className={`${styles.wave ?? ""} ${styles.waveR ?? ""}`}
          style={cssVars({ "--d": "0s" })}
        />
        <span
          className={`${styles.wave ?? ""} ${styles.waveR ?? ""}`}
          style={cssVars({ "--d": "0.8s" })}
        />
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------- layouts

interface Dot {
  l: number;
  t: number;
  s: number;
}

const LAYOUTS: { color: string; dots: Dot[] }[] = [
  // Corrida: um analógico grande e poucos botões.
  {
    color: "--slot-3",
    dots: [
      { l: 12, t: 38, s: 30 },
      { l: 74, t: 30, s: 14 },
      { l: 84, t: 52, s: 14 },
      { l: 8, t: 8, s: 12 },
      { l: 80, t: 8, s: 12 },
    ],
  },
  // Tiro: dois analógicos e o conjunto ABXY.
  {
    color: "--slot-1",
    dots: [
      { l: 10, t: 44, s: 24 },
      { l: 52, t: 58, s: 20 },
      { l: 78, t: 24, s: 11 },
      { l: 88, t: 36, s: 11 },
      { l: 78, t: 48, s: 11 },
      { l: 68, t: 36, s: 11 },
    ],
  },
  // Retrô: direcional e dois botões grandes.
  {
    color: "--slot-4",
    dots: [
      { l: 12, t: 30, s: 14 },
      { l: 24, t: 42, s: 14 },
      { l: 12, t: 54, s: 14 },
      { l: 0, t: 42, s: 14 },
      { l: 72, t: 44, s: 20 },
      { l: 86, t: 30, s: 20 },
    ],
  },
];

export function LayoutsArt({ names }: { names: string[] }) {
  const delays = ["0s", "3s", "6s"];
  return (
    <div className={styles.layouts} aria-hidden>
      {LAYOUTS.map((layout, i) => (
        <div key={layout.color}>
          <div
            className={styles.layoutCard}
            style={cssVars({ "--c": `var(${layout.color})`, "--d": delays[i] ?? "0s" })}
          >
            {layout.dots.map((dot) => (
              <i
                key={`${String(dot.l)}-${String(dot.t)}`}
                style={{
                  left: `${String(dot.l)}%`,
                  top: `${String(dot.t)}%`,
                  width: `${String(dot.s * 0.9)}%`,
                  aspectRatio: "1",
                }}
              />
            ))}
          </div>
          <p className={styles.layoutName}>{names[i]}</p>
        </div>
      ))}
    </div>
  );
}

// ---------------------------------------------------------------------------- privacidade

export function PrivacyArt() {
  return (
    <div className="flex flex-col items-center gap-5" aria-hidden>
      <div className={`${styles.node ?? ""} ${styles.cloud ?? ""} relative`}>
        <CloudOff className="size-6" strokeWidth={1.6} />
      </div>
      <div className={styles.nodes}>
        <div className={styles.node}>
          <Smartphone className="size-6" strokeWidth={1.6} />
        </div>
        <div className={styles.wire} />
        <div className={styles.node}>
          <Wifi className="size-6" strokeWidth={1.6} />
        </div>
        <div className={styles.wire} />
        <div className={styles.node}>
          <Monitor className="size-6" strokeWidth={1.6} />
        </div>
      </div>
      <Lock className="-mt-2 size-4 text-accent" strokeWidth={2} />
    </div>
  );
}

// ---------------------------------------------------------------------------- como funciona

export function WizardArt({ steps }: { steps: string[] }) {
  return (
    <ol className={styles.wizard} aria-label={steps.join(", ")}>
      {steps.map((step, i) => (
        <li
          key={step}
          className={styles.wizardRow}
          style={cssVars({ "--d": `${String(i * 1.5)}s` })}
        >
          <span className={styles.wizardDot} />
          <span className={styles.wizardLabel}>{step}</span>
        </li>
      ))}
    </ol>
  );
}

/** Módulos de um QR de enfeite (finder patterns nos três cantos, o resto sorteado de forma fixa). */
function qrModules(): { x: number; y: number }[] {
  const size = 21;
  const cells: { x: number; y: number }[] = [];
  let seed = 7;
  const next = () => {
    seed = (seed * 1103515245 + 12345) % 2147483648;
    return seed / 2147483648;
  };
  const inFinder = (x: number, y: number) =>
    (x < 8 && y < 8) || (x >= size - 8 && y < 8) || (x < 8 && y >= size - 8);
  for (let y = 0; y < size; y++) {
    for (let x = 0; x < size; x++) {
      if (!inFinder(x, y) && next() > 0.52) cells.push({ x, y });
    }
  }
  return cells;
}

const QR = qrModules();

export function QrArt({ label }: { label: string }) {
  const finder = (x: number, y: number) => (
    <g key={`${String(x)}-${String(y)}`}>
      <rect x={x} y={y} width="7" height="7" />
      <rect x={x + 1} y={y + 1} width="5" height="5" style={{ fill: "var(--text-primary)" }} />
      <rect x={x + 2} y={y + 2} width="3" height="3" />
    </g>
  );
  return (
    <div className="relative inline-block" role="img" aria-label={label}>
      <svg className={styles.qr} viewBox="0 0 21 21" shapeRendering="crispEdges">
        {finder(0, 0)}
        {finder(14, 0)}
        {finder(0, 14)}
        {QR.map((c) => (
          <rect key={`${String(c.x)}-${String(c.y)}`} x={c.x} y={c.y} width="1" height="1" />
        ))}
      </svg>
      <span className={`${styles.scan ?? ""} motion-only`} />
    </div>
  );
}
