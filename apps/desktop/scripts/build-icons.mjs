// Gera os ícones da bandeja (T-607) em várias resoluções (`tray/*.ico`).
//
// O ícone do app NÃO sai daqui: `source/app-icon.png` (1024 px, fundo transparente) é a arte
// final feita à mão (o original gerado está em `source/app-icon.original.png`). Para refazer
// os .ico/.icns/PNGs a partir dela: `pnpm tauri icon src-tauri/icons/source/app-icon.png`.
// Uso deste script: `pnpm icons`.
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

// Bandeja: o próprio ícone do app (`source/app-icon.png`, o quadrado azul com o gamepad branco),
// para a bandeja e o app terem a mesma identidade; o estado vai num selo no canto. Os tamanhos
// cobrem 100% a 200%+ de escala (16/24/32 px) e o 256 do ICO.
const appIcon = readFileSync(join(source, "app-icon.png")).toString("base64");

function trayIcon(badge) {
  const dot = badge
    ? `<circle cx="780" cy="780" r="230" fill="${badge}" stroke="#1C1C1E" stroke-width="50"/>`
    : "";
  return `<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 1024 1024">
    <image width="1024" height="1024" xlink:href="data:image/png;base64,${appIcon}"/>${dot}</svg>`;
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
