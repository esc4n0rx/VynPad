// Gera os ícones do app a partir dos SVGs de `src-tauri/icons/source`:
//   - `source/app-icon.png` (1024 px) — entrada do `pnpm tauri icon`, que produz .ico/.icns/PNGs;
//   - `tray/*.ico` — ícones da bandeja (T-607) em várias resoluções.
// Uso: `pnpm icons` (e depois `pnpm tauri icon src-tauri/icons/source/app-icon.png`).
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { Resvg } from "@resvg/resvg-js";
import pngToIco from "png-to-ico";

const icons = join(dirname(fileURLToPath(import.meta.url)), "..", "src-tauri", "icons");
const source = join(icons, "source");
const tray = join(icons, "tray");
mkdirSync(tray, { recursive: true });

/** Renderiza um SVG em PNG com `size` px de lado. */
function rasterize(svg, size) {
  return new Resvg(svg, { fitTo: { mode: "width", value: size } }).render().asPng();
}

// Ícone do app.
writeFileSync(
  join(source, "app-icon.png"),
  rasterize(readFileSync(join(source, "app-icon.svg"), "utf8"), 1024),
);

// Bandeja: o gamepad em branco (fica bem sobre a barra escura do Windows), sem fundo; o estado
// vai num selo no canto. Os tamanhos cobrem 100% a 200%+ de escala (16/24/32 px) e o 256 do ICO.
const GAMEPAD =
  "M240 78C200 78 170 70 140 74C96 80 70 120 52 190C40 238 44 276 70 282C96 288 112 262 132 232C148 210 170 208 190 208L290 208C310 208 332 210 348 232C368 262 384 288 410 282C436 276 440 238 428 190C410 120 384 80 340 74C310 70 280 78 240 78Z";
const HOLES =
  '<circle cx="138" cy="124" r="22"/><circle cx="292" cy="172" r="22"/><path d="M178 148h20v18h18v20h-18v18h-20v-18h-18v-20h18z" transform="translate(-2 -4)"/><circle cx="342" cy="98" r="11"/><circle cx="342" cy="150" r="11"/><circle cx="316" cy="124" r="11"/><circle cx="368" cy="124" r="11"/>';

function trayIcon(badge) {
  const dot = badge
    ? `<circle cx="392" cy="268" r="60" fill="${badge}" stroke="#1C1C1E" stroke-width="14"/>`
    : "";
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="30 -29 420 420">
    <path d="${GAMEPAD}" fill="#FFFFFF"/>
    <g fill="#1C1C1E" fill-opacity="0.9">${HOLES}</g>${dot}</svg>`;
}

const variants = { normal: undefined, active: "#30D158", error: "#FF453A" };
const sizes = [16, 20, 24, 32, 48, 64, 256];

for (const [name, badge] of Object.entries(variants)) {
  const svg = trayIcon(badge);
  const ico = await pngToIco(sizes.map((size) => rasterize(svg, size)));
  writeFileSync(join(tray, `${name}.ico`), ico);
  writeFileSync(join(source, `tray-${name}.png`), rasterize(svg, 256));
}

console.log("ícones gerados em", icons);
