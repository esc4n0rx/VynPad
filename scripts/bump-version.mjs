// Troca a versão do VynPad em todos os lugares onde ela mora (usado por scripts/release.sh).
//
// Uso: node scripts/bump-version.mjs 0.2.0
//
// Atualiza: Cargo.toml (workspace), tauri.conf.json, apps/desktop/package.json e o Android
// (`versionName` e `versionCode`, que sobe de 1 em 1: a Play exige um código maior a cada envio).
// Imprime o novo `versionCode`. Depois disso, rode `cargo update --workspace` para o Cargo.lock.
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const version = process.argv[2];
if (!version || !/^\d+\.\d+\.\d+$/.test(version)) {
  console.error("uso: node scripts/bump-version.mjs X.Y.Z");
  process.exit(2);
}

/** Troca a primeira ocorrência de `pattern` (com um grupo para o valor) e confere que achou. */
function edit(file, pattern, replacement) {
  const path = join(root, file);
  const text = readFileSync(path, "utf8");
  if (!pattern.test(text)) {
    console.error(`não achei a versão em ${file}`);
    process.exit(1);
  }
  writeFileSync(path, text.replace(pattern, replacement));
}

edit("Cargo.toml", /^version = "[^"]*"/m, `version = "${version}"`);
edit("apps/desktop/src-tauri/tauri.conf.json", /"version":\s*"[^"]*"/, `"version": "${version}"`);
edit("apps/desktop/package.json", /"version":\s*"[^"]*"/, `"version": "${version}"`);

const gradle = "apps/android/androidApp/build.gradle.kts";
const code = Number(/versionCode = (\d+)/.exec(readFileSync(join(root, gradle), "utf8"))?.[1]);
if (!Number.isInteger(code)) {
  console.error(`não achei o versionCode em ${gradle}`);
  process.exit(1);
}
edit(gradle, /versionName = "[^"]*"/, `versionName = "${version}"`);
edit(gradle, /versionCode = \d+/, `versionCode = ${code + 1}`);

console.log(code + 1);
