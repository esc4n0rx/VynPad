// Gera protocol/test-vectors/*.json. Uso: pnpm --filter @vynpad/protocol gen

import { writeFileSync } from "node:fs";
import { join } from "node:path";

import { loadAjv, serialize, VECTORS_DIR } from "./common.mjs";
import { buildAll } from "./vectors.mjs";

const { validator } = loadAjv();
const files = buildAll(validator("control-message.schema.json"));
for (const [file, content] of Object.entries(files)) {
  writeFileSync(join(VECTORS_DIR, file), serialize(content));
  console.log(`${file}: ${content.vectors.length} vectors`);
}
