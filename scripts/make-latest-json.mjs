// Monta o `latest.json` que o `tauri-plugin-updater` lê (T-704).
//
// Uso:
//   node scripts/make-latest-json.mjs --tag v0.2.0 --repo dono/repo \
//     --dir target/release/bundle/nsis --notes-file notes.md --out latest.json
//
// Procura em --dir o instalador NSIS (`*-setup.exe`) e a assinatura minisign (`.sig`) que o
// `tauri build` gera com `createUpdaterArtifacts`. O manifesto fica no release, ao lado do
// instalador, e o app o busca em `releases/latest/download/latest.json`.
import { existsSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const args = Object.fromEntries(
  process.argv
    .slice(2)
    .reduce((pairs, arg, i, all) => (arg.startsWith("--") ? [...pairs, [arg.slice(2), all[i + 1]]] : pairs), []),
);

const need = (name) => {
  if (!args[name]) {
    console.error(`falta --${name}`);
    process.exit(2);
  }
  return args[name];
};

const tag = need("tag");
const repo = need("repo");
const dir = need("dir");
const out = need("out");
const version = tag.replace(/^v/, "");

const installers = readdirSync(dir).filter((f) => f.endsWith("-setup.exe"));
if (installers.length !== 1) {
  console.error(`esperava 1 instalador *-setup.exe em ${dir}, achei ${installers.length}`);
  process.exit(1);
}
const installer = installers[0];
const sigPath = join(dir, `${installer}.sig`);
if (!existsSync(sigPath)) {
  console.error(`falta a assinatura ${sigPath}: o build rodou com createUpdaterArtifacts e a chave?`);
  process.exit(1);
}

const notes = args["notes-file"] && existsSync(args["notes-file"])
  ? readFileSync(args["notes-file"], "utf8").trim()
  : "";

const manifest = {
  version,
  notes,
  pub_date: new Date().toISOString(),
  platforms: {
    "windows-x86_64": {
      signature: readFileSync(sigPath, "utf8").trim(),
      url: `https://github.com/${repo}/releases/download/${tag}/${encodeURIComponent(installer)}`,
    },
  },
};

writeFileSync(out, `${JSON.stringify(manifest, null, 2)}\n`);
console.log(`${out}: versão ${version}, ${installer}`);
