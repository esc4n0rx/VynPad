// O simulador (`vynpad-sim`) no papel do celular, para os testes E2E.
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
const exe = process.platform === "win32" ? ".exe" : "";

/** Porta de controle do app em modo E2E (ver `E2E_PORTS` em `src-tauri/src/lib.rs`). */
export const CONTROL = "127.0.0.1:47900";

function simPath() {
  const candidates = [
    process.env.VYNPAD_SIM,
    join(root, "target", "debug", `vynpad-sim${exe}`),
    join(root, "target", "release", `vynpad-sim${exe}`),
  ].filter(Boolean);
  const found = candidates.find((p) => existsSync(p));
  if (!found) throw new Error("falta o simulador: cargo build -p vynpad-sim");
  return found;
}

/**
 * Pareia pelo PIN e segura a sessão por `holdSeconds` (o controle fica conectado num slot).
 * Devolve `{ done }`: uma promessa com a saída do simulador, que falha se ele terminar com erro.
 */
export function pairByPin({ pin, name, holdSeconds = 0 }) {
  const args = ["pair", "--pin", pin, "--host", CONTROL, "--name", name];
  if (holdSeconds > 0) args.push("--hold", `${holdSeconds}s`);
  const child = spawn(simPath(), args, { stdio: ["ignore", "pipe", "pipe"] });
  let output = "";
  child.stdout.on("data", (chunk) => (output += chunk));
  child.stderr.on("data", (chunk) => (output += chunk));
  const done = new Promise((resolveDone, reject) => {
    child.on("error", reject);
    child.on("close", (code) =>
      code === 0 ? resolveDone(output) : reject(new Error(`vynpad-sim saiu com ${code}:\n${output}`)),
    );
  });
  return { done, stop: () => child.kill() };
}
