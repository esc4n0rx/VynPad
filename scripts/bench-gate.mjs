// Gate de regressão do benchmark do server (T-702).
//
// Uso: node scripts/bench-gate.mjs <atual.json> <base.json>
//
// Compara o p99 do processamento no server (`server_proc_us`) com a base versionada. Falha se
// passar de 1 ms (RNF-01) ou se crescer mais de 20% sobre a base. Como o valor é de dezenas de
// µs e runners compartilhados oscilam, a base usada na comparação tem piso de 100 µs.
import { readFileSync } from "node:fs";

const TOLERANCE = 1.2;
const FLOOR_US = 100;
const LIMIT_US = 1000;

const [currentPath, basePath] = process.argv.slice(2);
if (!currentPath || !basePath) {
  console.error("uso: node scripts/bench-gate.mjs <atual.json> <base.json>");
  process.exit(2);
}

const load = (path) => JSON.parse(readFileSync(path, "utf8")).total;
const current = load(currentPath);
const base = load(basePath);

const p99 = current.server_proc_us.p99;
const allowed = Math.max(base.server_proc_us.p99, FLOOR_US) * TOLERANCE;
console.log(`p99 do server: ${p99} µs (base ${base.server_proc_us.p99} µs, limite ${allowed} µs)`);

const failures = [];
if (current.pongs === 0) failures.push("nenhum Pong chegou");
if (current.ping_loss_percent > 1) failures.push(`${current.ping_loss_percent}% de pings sem resposta`);
if (p99 > LIMIT_US) failures.push(`p99 acima de ${LIMIT_US} µs (RNF-01)`);
if (p99 > allowed) failures.push("regressão de mais de 20% sobre a base");

if (failures.length > 0) {
  for (const failure of failures) console.error(`FALHOU: ${failure}`);
  process.exit(1);
}
console.log("ok");
