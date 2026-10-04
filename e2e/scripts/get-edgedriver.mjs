// Baixa o msedgedriver na versão do WebView2 instalado (T-701).
//
// O tauri-driver só repassa os comandos ao driver nativo, e no Windows o app roda no WebView2: o
// msedgedriver precisa ter a MESMA versão principal dele, senão a sessão não abre.
// Uso: pnpm --filter @vynpad/e2e drivers   (grava em e2e/.drivers/msedgedriver.exe)
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

if (process.platform !== "win32") {
  console.error("O E2E com tauri-driver roda no Windows (WebView2).");
  process.exit(1);
}

const here = dirname(fileURLToPath(import.meta.url));
const out = join(here, "..", ".drivers");
const target = join(out, "msedgedriver.exe");

// WebView2 Runtime e Edge, nesta ordem; o primeiro que existir define a versão.
const CLIENTS = [
  "{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
  "{56EB18F8-B008-4CBD-B6D2-8C97FE7E9062}",
];
const HIVES = [
  "HKLM\\SOFTWARE\\WOW6432Node\\Microsoft\\EdgeUpdate\\Clients",
  "HKCU\\Software\\Microsoft\\EdgeUpdate\\Clients",
  "HKLM\\SOFTWARE\\Microsoft\\EdgeUpdate\\Clients",
];

function installedVersion() {
  for (const [index, client] of CLIENTS.entries()) {
    for (const hive of HIVES) {
      try {
        const text = execFileSync("reg", ["query", `${hive}\\${client}`, "/v", "pv"], {
          encoding: "utf8",
          stdio: ["ignore", "pipe", "ignore"],
        });
        const match = /pv\s+REG_SZ\s+(\d+\.\d+\.\d+\.\d+)/.exec(text);
        if (match) return { version: match[1], source: index === 0 ? "WebView2 Runtime" : "Edge" };
      } catch {
        // chave ausente: tenta a próxima
      }
    }
  }
  throw new Error("não achei o WebView2 nem o Edge instalados");
}

async function download(version) {
  const response = await fetch(`https://msedgedriver.microsoft.com/${version}/edgedriver_win64.zip`);
  return response.ok ? Buffer.from(await response.arrayBuffer()) : null;
}

/** Quando a versão exata não existe, usa a mais nova da mesma versão principal. */
async function latestForMajor(major) {
  const response = await fetch(`https://msedgedriver.microsoft.com/LATEST_RELEASE_${major}_WINDOWS`);
  if (!response.ok) return null;
  // O arquivo vem em UTF-16.
  return Buffer.from(await response.arrayBuffer()).toString("utf16le").replace(/^﻿/, "").trim();
}

const { version, source } = installedVersion();
console.log(`${source} ${version}`);
if (source !== "WebView2 Runtime") {
  // O app roda no WebView2, não no Edge; se as versões diferirem a sessão não abre.
  console.warn("aviso: o WebView2 Runtime não foi achado no registro; usando a versão do Edge");
}
let zip = await download(version);
if (!zip) {
  const fallback = await latestForMajor(version.split(".")[0]);
  console.log(`sem driver exato; usando ${fallback}`);
  zip = fallback ? await download(fallback) : null;
}
if (!zip) {
  console.error("não consegui baixar o msedgedriver");
  process.exit(1);
}

mkdirSync(out, { recursive: true });
const archive = join(out, "edgedriver.zip");
writeFileSync(archive, zip);
execFileSync(
  "powershell",
  ["-NoProfile", "-Command", `Expand-Archive -Force -LiteralPath '${archive}' -DestinationPath '${out}'`],
  { stdio: "inherit" },
);
rmSync(archive);
if (!existsSync(target)) {
  console.error(`o zip não trouxe ${target}`);
  process.exit(1);
}
console.log(execFileSync(target, ["--version"], { encoding: "utf8" }).trim());
