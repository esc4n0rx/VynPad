// Gera as notas de um release a partir dos Conventional Commits (T-704, release.yml).
//
// Uso: node scripts/changelog.mjs --tag v0.2.0 [--repo dono/repo] [--out notes.md]
//
// Pega os commits entre a tag anterior e `--tag` (ou HEAD, se a tag ainda não existir) e agrupa
// por tipo. O gerador de notas do GitHub só lista pull requests, e aqui se trabalha com commits
// direto na `main`, então ele sairia quase vazio.
import { execFileSync } from "node:child_process";
import { writeFileSync } from "node:fs";

const args = Object.fromEntries(
  process.argv
    .slice(2)
    .reduce((pairs, arg, i, all) => (arg.startsWith("--") ? [...pairs, [arg.slice(2), all[i + 1]]] : pairs), []),
);
const tag = args.tag;
if (!tag) {
  console.error("uso: node scripts/changelog.mjs --tag vX.Y.Z [--repo dono/repo] [--out arquivo]");
  process.exit(2);
}

const git = (...a) =>
  execFileSync("git", a, { encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] }).trim();
const exists = (ref) => {
  try {
    git("rev-parse", "--verify", "--quiet", `${ref}^{commit}`);
    return true;
  } catch {
    return false;
  }
};

const head = exists(tag) ? tag : "HEAD";
let previous = "";
try {
  previous = git("describe", "--tags", "--abbrev=0", "--match", "v*", `${head}^`);
} catch {
  // primeiro release: vai desde o primeiro commit
}
const range = previous ? `${previous}..${head}` : head;

const SECTIONS = [
  ["breaking", "⚠️ Mudanças incompatíveis"],
  ["feat", "✨ Novidades"],
  ["fix", "🐛 Correções"],
  ["perf", "⚡ Desempenho"],
  ["refactor", "♻️ Refatoração"],
  ["test", "🧪 Testes"],
  ["docs", "📚 Documentação"],
  ["maintenance", "🔧 Manutenção"],
];
const MAINTENANCE = new Set(["ci", "build", "chore", "style", "revert"]);

const groups = new Map(SECTIONS.map(([key]) => [key, []]));
const log = git("log", range, "--no-merges", "--pretty=format:%h%x09%s");
for (const line of log ? log.split("\n") : []) {
  const [hash, subject] = line.split("\t");
  const match = /^(\w+)(?:\(([^)]+)\))?(!)?:\s*(.+)$/.exec(subject);
  if (!match) {
    groups.get("maintenance").push({ hash, text: subject });
    continue;
  }
  const [, type, scope, bang, text] = match;
  const key = bang ? "breaking" : MAINTENANCE.has(type) ? "maintenance" : groups.has(type) ? type : "maintenance";
  groups.get(key).push({ hash, scope, text });
}

const lines = [];
for (const [key, title] of SECTIONS) {
  const items = groups.get(key);
  if (items.length === 0) continue;
  lines.push(`### ${title}`, "");
  for (const { hash, scope, text } of items) {
    lines.push(`- ${scope ? `**${scope}:** ` : ""}${text} (${hash})`);
  }
  lines.push("");
}
if (lines.length === 0) lines.push("Sem mudanças registradas.", "");
if (args.repo && previous) {
  lines.push(`**Comparar:** https://github.com/${args.repo}/compare/${previous}...${tag}`, "");
}

const notes = lines.join("\n");
if (args.out) writeFileSync(args.out, notes);
else process.stdout.write(notes);
