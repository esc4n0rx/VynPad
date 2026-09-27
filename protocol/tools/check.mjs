// Confere os test vectors commitados. Uso: pnpm --filter @padgaming/protocol check
//
// 1. O ChaCha20-Poly1305 do Node bate com a RFC 8439 §2.8.2.
// 2. Os arquivos em test-vectors/ são idênticos ao que o gerador produz.
// 3. Cada arquivo valida no seu schema.
// 4. Relendo os arquivos, o decoder de referência produz o `expect` de cada vector e o
//    encoder reproduz o `packet_hex` dos vectors UDP válidos.

import { createCipheriv } from "node:crypto";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { isDeepStrictEqual } from "node:util";

import { loadAjv, serialize, VECTORS_DIR } from "./common.mjs";
import {
  PACKET_SPECS,
  decodeFrame,
  decodePacket,
  encodeHeader,
  encodePayload,
  parsePairingUri,
  seal,
} from "./reference.mjs";
import { buildAll } from "./vectors.mjs";

const failures = [];
const fail = (msg) => failures.push(msg);

// 1. Sanidade do AEAD (RFC 8439 §2.8.2).
{
  const key = Buffer.from(
    "808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9f",
    "hex",
  );
  const nonce = Buffer.from("070000004041424344454647", "hex");
  const aad = Buffer.from("50515253c0c1c2c3c4c5c6c7", "hex");
  const plain = Buffer.from(
    "Ladies and Gentlemen of the class of '99: If I could offer you only one tip for the future, sunscreen would be it.",
  );
  const c = createCipheriv("chacha20-poly1305", key, nonce, { authTagLength: 16 });
  c.setAAD(aad, { plaintextLength: plain.length });
  const ct = Buffer.concat([c.update(plain), c.final()]);
  if (!ct.subarray(0, 8).equals(Buffer.from("d31a8d34648e60db", "hex")))
    fail("RFC 8439: ciphertext diverge");
  if (!c.getAuthTag().equals(Buffer.from("1ae10b594f09e26a7e902ecbd0600691", "hex")))
    fail("RFC 8439: tag diverge");
}

const { ajv, validator } = loadAjv();
const validateMessage = validator("control-message.schema.json");
const expected = buildAll(validateMessage);
const schemaOf = {
  "udp.json": "udp-vectors.schema.json",
  "control.json": "control-vectors.schema.json",
  "pairing-uri.json": "pairing-uri-vectors.schema.json",
};

let total = 0;
for (const [file, content] of Object.entries(expected)) {
  const path = join(VECTORS_DIR, file);
  let text;
  try {
    text = readFileSync(path, "utf8");
  } catch {
    fail(`${file}: não existe (rode \`pnpm --filter @padgaming/protocol gen\`)`);
    continue;
  }

  // 2. Arquivo em dia com o gerador.
  if (text !== serialize(content)) {
    fail(
      `${file}: desatualizado em relação ao gerador (rode \`pnpm --filter @padgaming/protocol gen\`)`,
    );
  }

  // 3. Schema do arquivo.
  const data = JSON.parse(text);
  const validate = validator(schemaOf[file]);
  if (!validate(data)) fail(`${file}: não valida no schema: ${ajv.errorsText(validate.errors)}`);

  // 4. Decoder/encoder de referência contra o conteúdo do arquivo.
  const names = new Set();
  for (const v of data.vectors) {
    const id = `${file}#${v.name}`;
    if (names.has(v.name)) fail(`${id}: nome duplicado`);
    names.add(v.name);

    let got;
    if (file === "udp.json") {
      const ctx = { ...v.receiver, key: Buffer.from(v.receiver.key_hex, "hex") };
      got = decodePacket(Buffer.from(v.packet_hex, "hex"), ctx);
      if (v.expect.result === "ok" && !v.decode_only) {
        const { header, payload } = v.expect;
        const packet = seal(
          ctx.key,
          PACKET_SPECS[header.type].dir,
          encodeHeader(header),
          encodePayload(header.type, payload),
        );
        if (packet.toString("hex") !== v.packet_hex)
          fail(`${id}: o encoder não reproduz packet_hex`);
      }
    } else if (file === "control.json") {
      got = decodeFrame(Buffer.from(v.frame_hex, "hex"), validateMessage);
    } else {
      got = parsePairingUri(v.uri);
    }
    if (!isDeepStrictEqual(got, v.expect)) fail(`${id}: o decoder de referência diverge do expect`);
  }

  const valid = data.vectors.filter((v) => v.expect.result === "ok").length;
  console.log(
    `${file}: ${data.vectors.length} vectors (${valid} válidos, ${data.vectors.length - valid} inválidos/ignorados)`,
  );
  total += data.vectors.length;
}

if (failures.length) {
  console.error(`\n${failures.length} falha(s):`);
  for (const f of failures) console.error(`  - ${f}`);
  process.exit(1);
}
console.log(`OK: ${total} vectors conferidos.`);
