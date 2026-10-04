// WebdriverIO + tauri-driver (T-701): abre o VynPad de verdade e usa a interface.
//
// Pré-requisitos (Windows):
//   - o app compilado com o frontend embutido:
//       pnpm --filter @vynpad/desktop build
//       cargo build --release -p vynpad-desktop --features tauri/custom-protocol
//   - `cargo install tauri-driver --locked`
//   - o msedgedriver da versão do WebView2: `pnpm --filter @vynpad/e2e drivers`
//   - o simulador (`cargo build -p vynpad-sim`), que faz o papel do celular
//   - nenhum VynPad aberto (o app aceita uma instância só)
//
// O app roda com `VYNPAD_DATA_DIR` numa pasta temporária e `VYNPAD_E2E` (portas 47900-47902, sem
// mDNS e sem procurar atualização), então não toca nos dados nem na rede do usuário. Os arquivos
// em `specs/` rodam em ordem e **compartilham** a pasta de dados: o primeiro conclui a introdução
// e os seguintes já começam no dashboard.
import { spawn } from "node:child_process";
import { browser } from "@wdio/globals";
import { existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { homedir, tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "..");
const exe = process.platform === "win32" ? ".exe" : "";

const application =
  process.env.VYNPAD_E2E_APP ?? join(root, "target", "release", `vynpad-desktop${exe}`);
const nativeDriver =
  process.env.EDGE_DRIVER ?? join(here, ".drivers", `msedgedriver${exe}`);
const tauriDriver =
  process.env.TAURI_DRIVER ?? join(homedir(), ".cargo", "bin", `tauri-driver${exe}`);

// `E2E_DRIVER_LOG=<arquivo>` liga o log detalhado do msedgedriver (para diagnosticar uma sessão
// que não abre). O tauri-driver só repassa `--native-driver`, então um wrapper põe as opções.
function nativeDriverCommand() {
  const log = process.env.E2E_DRIVER_LOG;
  if (!log || process.platform !== "win32") return nativeDriver;
  const wrapper = join(here, ".drivers", "msedgedriver-verbose.cmd");
  writeFileSync(wrapper, `@echo off
"${nativeDriver}" %* --verbose --log-path="${log}"
`);
  return wrapper;
}

// O lançador carrega este arquivo primeiro e define a pasta; os workers herdam o ambiente.
const ownsDataDir = !process.env.VYNPAD_DATA_DIR;
process.env.VYNPAD_DATA_DIR ??= mkdtempSync(join(tmpdir(), "vynpad-e2e-"));
process.env.VYNPAD_E2E = "1";

let driver;

export const config = {
  runner: "local",
  logLevel: "warn",
  hostname: "127.0.0.1",
  port: 4444,
  path: "/",
  specs: ["./specs/**/*.e2e.mjs"],
  maxInstances: 1,
  capabilities: [{ maxInstances: 1, "tauri:options": { application } }],
  framework: "mocha",
  reporters: ["spec"],
  mochaOpts: { ui: "bdd", timeout: 120_000 },
  waitforTimeout: 15_000,
  connectionRetryTimeout: 60_000,
  connectionRetryCount: 2,

  onPrepare() {
    for (const [what, file] of [
      ["o app (compile com --features tauri/custom-protocol)", application],
      ["o tauri-driver (cargo install tauri-driver --locked)", tauriDriver],
      ["o msedgedriver (pnpm --filter @vynpad/e2e drivers)", nativeDriver],
    ]) {
      if (!existsSync(file)) throw new Error(`falta ${what}: ${file}`);
    }
  },

  beforeSession() {
    driver = spawn(tauriDriver, ["--native-driver", nativeDriverCommand()], {
      stdio: [null, process.stdout, process.stderr],
    });
    // O tauri-driver demora um instante para abrir a porta 4444.
    return new Promise((done) => setTimeout(done, 1500));
  },

  afterSession() {
    driver?.kill();
  },

  // Uma captura da tela do app quando um teste falha, para o CI anexar ao resultado.
  async afterTest(test, _context, { passed }) {
    if (passed) return;
    const dir = join(here, "logs");
    mkdirSync(dir, { recursive: true });
    const name = `${test.parent} ${test.title}`.replace(/[^\p{L}\p{N}]+/gu, "-");
    await browser.saveScreenshot(join(dir, `${name}.png`));
  },

  onComplete() {
    if (ownsDataDir) rmSync(process.env.VYNPAD_DATA_DIR, { recursive: true, force: true });
  },
};
