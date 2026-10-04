// Gate de cobertura por crate (T-701).
//
// Uso: node scripts/coverage-gate.mjs <cov.json>
//
// `cov.json` vem de `cargo llvm-cov --workspace --json --summary-only --output-path cov.json`.
// Soma as linhas de `crates/<nome>/src` e exige o mínimo nos crates de **domínio**. Ficam de fora,
// de propósito: `vynpad-driver` (a parte do ViGEmBus só roda com o driver instalado, nos testes
// `#[ignore]`), `vynpad-sim` e `vynpad-dbprep` (ferramentas) e o desktop (coberto por Vitest e
// pelos testes E2E). A tabela vai para o resumo do job quando `GITHUB_STEP_SUMMARY` existe.
import { appendFileSync, readFileSync } from "node:fs";

const MIN_PERCENT = 80;
const DOMAIN = [
  "vynpad-protocol",
  "vynpad-mapping",
  "vynpad-core",
  "vynpad-storage",
  "vynpad-net",
  "vynpad-server",
];

const path = process.argv[2];
if (!path) {
  console.error("uso: node scripts/coverage-gate.mjs <cov.json>");
  process.exit(2);
}

const report = JSON.parse(readFileSync(path, "utf8"));
const crates = new Map();
for (const file of report.data[0].files) {
  const match = /crates[\\/]([^\\/]+)[\\/]src[\\/]/.exec(file.filename);
  if (!match) continue;
  const total = crates.get(match[1]) ?? { covered: 0, count: 0 };
  total.covered += file.summary.lines.covered;
  total.count += file.summary.lines.count;
  crates.set(match[1], total);
}

const rows = [];
const failures = [];
for (const [name, { covered, count }] of [...crates].sort(([a], [b]) => a.localeCompare(b))) {
  const percent = count === 0 ? 100 : (100 * covered) / count;
  const gated = DOMAIN.includes(name);
  const ok = !gated || percent >= MIN_PERCENT;
  rows.push({ name, covered, count, percent, gated, ok });
  if (!ok) failures.push(`${name}: ${percent.toFixed(1)}% (mínimo ${MIN_PERCENT}%)`);
}
for (const name of DOMAIN) {
  if (!crates.has(name)) failures.push(`${name}: sem dados de cobertura`);
}

const line = (r) =>
  `${r.name.padEnd(18)} ${r.percent.toFixed(1).padStart(5)}%  ${String(r.covered).padStart(5)}/${String(r.count).padEnd(5)}  ${r.gated ? (r.ok ? "ok" : "ABAIXO") : "(fora do gate)"}`;
console.log(rows.map(line).join("\n"));

if (process.env.GITHUB_STEP_SUMMARY) {
  const table = [
    `### Cobertura de linhas (mínimo ${MIN_PERCENT}% nos crates de domínio)`,
    "",
    "| Crate | Cobertura | Linhas | Gate |",
    "|---|---:|---:|---|",
    ...rows.map(
      (r) =>
        `| ${r.name} | ${r.percent.toFixed(1)}% | ${r.covered}/${r.count} | ${r.gated ? (r.ok ? "✅" : "❌") : "—"} |`,
    ),
    "",
  ].join("\n");
  appendFileSync(process.env.GITHUB_STEP_SUMMARY, table);
}

if (failures.length > 0) {
  for (const failure of failures) console.error(`FALHOU: ${failure}`);
  process.exit(1);
}
console.log(`ok: todos os crates de domínio com pelo menos ${MIN_PERCENT}%`);
