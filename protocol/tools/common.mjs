// Utilidades compartilhadas pelo gerador e pelo check: caminhos, schemas (Ajv, draft 2020-12) e serialização.

import { readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import Ajv2020 from "ajv/dist/2020.js";

export const PROTOCOL_DIR = join(dirname(fileURLToPath(import.meta.url)), "..");
export const SCHEMAS_DIR = join(PROTOCOL_DIR, "schemas");
export const VECTORS_DIR = join(PROTOCOL_DIR, "test-vectors");

const BASE_ID = "https://vynpad.local/schemas/";

export function loadAjv() {
  const ajv = new Ajv2020({ allErrors: true, strict: true });
  for (const file of readdirSync(SCHEMAS_DIR).filter((f) => f.endsWith(".schema.json"))) {
    ajv.addSchema(JSON.parse(readFileSync(join(SCHEMAS_DIR, file), "utf8")));
  }
  return {
    ajv,
    validator: (file) => ajv.getSchema(BASE_ID + file),
  };
}

/** Formato canônico dos arquivos de vectors (2 espaços, LF final). */
export function serialize(obj) {
  return `${JSON.stringify(obj, null, 2)}\n`;
}
