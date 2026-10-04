// Implementação de referência do protocolo VynPad v1 (docs/00-visao-geral/PROTOCOLO.md).
// Usada só para gerar e conferir os test vectors; o server usa o crate vynpad-protocol
// e o client Android o módulo core:protocol.

import { createCipheriv, createDecipheriv } from "node:crypto";

export const MAGIC = 0x5056;
export const VERSION = 1;
export const HEADER_LEN = 12;
export const TAG_LEN = 16;
export const MAX_FRAME_LEN = 16384;

export const DIR_C2S = 0;
export const DIR_S2C = 1;

export const BUTTONS_MASK = 0xf7ff;
export const FLAG_MOTION = 0x01;

export const PacketType = Object.freeze({
  INPUT: 0x01,
  INPUT_MOTION: 0x02,
  PING: 0x10,
  PONG: 0x11,
  RUMBLE: 0x20,
});

/** Direção e tamanho do payload de cada tipo (§6.1). */
export const PACKET_SPECS = Object.freeze({
  [PacketType.INPUT]: { name: "input", dir: DIR_C2S, payloadLen: 24 },
  [PacketType.INPUT_MOTION]: { name: "input_motion", dir: DIR_C2S, payloadLen: 36 },
  [PacketType.PING]: { name: "ping", dir: DIR_C2S, payloadLen: 8 },
  [PacketType.PONG]: { name: "pong", dir: DIR_S2C, payloadLen: 20 },
  [PacketType.RUMBLE]: { name: "rumble", dir: DIR_S2C, payloadLen: 4 },
});

/** Botões XInput (§7), na ordem dos bits. */
export const BUTTONS = Object.freeze({
  dpad_up: 0x0001,
  dpad_down: 0x0002,
  dpad_left: 0x0004,
  dpad_right: 0x0008,
  start: 0x0010,
  back: 0x0020,
  left_thumb: 0x0040,
  right_thumb: 0x0080,
  left_shoulder: 0x0100,
  right_shoulder: 0x0200,
  guide: 0x0400,
  a: 0x1000,
  b: 0x2000,
  x: 0x4000,
  y: 0x8000,
});

export function buttonsFromNames(names) {
  return names.reduce((acc, n) => {
    if (!(n in BUTTONS)) throw new Error(`botão desconhecido: ${n}`);
    return acc | BUTTONS[n];
  }, 0);
}

export function buttonNames(bits) {
  return Object.entries(BUTTONS)
    .filter(([, bit]) => (bits & bit) !== 0)
    .map(([name]) => name);
}

// ---------------------------------------------------------------- UDP

export function encodeHeader({ magic = MAGIC, ver = VERSION, type, session_id, seq }) {
  const h = Buffer.alloc(HEADER_LEN);
  h.writeUInt16LE(magic, 0);
  h.writeUInt8(ver, 2);
  h.writeUInt8(type, 3);
  h.writeUInt32LE(session_id, 4);
  h.writeUInt32LE(seq, 8);
  return h;
}

export function decodeHeader(buf) {
  return {
    magic: buf.readUInt16LE(0),
    ver: buf.readUInt8(2),
    type: buf.readUInt8(3),
    session_id: buf.readUInt32LE(4),
    seq: buf.readUInt32LE(8),
  };
}

/** Nonce de 12 bytes: direction (u32 LE) ‖ seq (u32 LE) ‖ 00 00 00 00 (§6.2). */
export function nonce(dir, seq) {
  const n = Buffer.alloc(12);
  n.writeUInt32LE(dir, 0);
  n.writeUInt32LE(seq, 4);
  return n;
}

export function seal(key, dir, header, plain) {
  const c = createCipheriv("chacha20-poly1305", key, nonce(dir, header.readUInt32LE(8)), {
    authTagLength: TAG_LEN,
  });
  c.setAAD(header, { plaintextLength: plain.length });
  const ct = Buffer.concat([c.update(plain), c.final()]);
  return Buffer.concat([header, ct, c.getAuthTag()]);
}

/** Retorna o payload em claro ou `null` se a tag não confere. */
export function open(key, dir, packet) {
  const header = packet.subarray(0, HEADER_LEN);
  const ct = packet.subarray(HEADER_LEN, packet.length - TAG_LEN);
  const tag = packet.subarray(packet.length - TAG_LEN);
  const d = createDecipheriv("chacha20-poly1305", key, nonce(dir, header.readUInt32LE(8)), {
    authTagLength: TAG_LEN,
  });
  d.setAAD(header, { plaintextLength: ct.length });
  d.setAuthTag(tag);
  try {
    return Buffer.concat([d.update(ct), d.final()]);
  } catch {
    return null;
  }
}

function writeInput(buf, p) {
  buf.writeBigInt64LE(BigInt(p.t_client_us), 0);
  buf.writeUInt16LE(p.buttons, 8);
  buf.writeUInt8(p.lt, 10);
  buf.writeUInt8(p.rt, 11);
  buf.writeInt16LE(p.lx, 12);
  buf.writeInt16LE(p.ly, 14);
  buf.writeInt16LE(p.rx, 16);
  buf.writeInt16LE(p.ry, 18);
  buf.writeUInt16LE(p.rumble_ack_id, 20);
  buf.writeUInt8(p.flags, 22);
  buf.writeUInt8(p.battery, 23);
}

function readInput(buf) {
  return {
    t_client_us: Number(buf.readBigInt64LE(0)),
    buttons: buf.readUInt16LE(8) & BUTTONS_MASK,
    lt: buf.readUInt8(10),
    rt: buf.readUInt8(11),
    lx: buf.readInt16LE(12),
    ly: buf.readInt16LE(14),
    rx: buf.readInt16LE(16),
    ry: buf.readInt16LE(18),
    rumble_ack_id: buf.readUInt16LE(20),
    flags: buf.readUInt8(22) & FLAG_MOTION,
    battery: buf.readUInt8(23),
  };
}

const MOTION_FIELDS = ["gx", "gy", "gz", "ax", "ay", "az"];

/** Serializa o payload em claro. Não valida nada: os vectors inválidos precisam montar bytes errados. */
export function encodePayload(type, p) {
  const buf = Buffer.alloc(PACKET_SPECS[type].payloadLen);
  switch (type) {
    case PacketType.INPUT:
      writeInput(buf, p);
      break;
    case PacketType.INPUT_MOTION:
      writeInput(buf, p);
      MOTION_FIELDS.forEach((f, i) => buf.writeInt16LE(p[f], 24 + i * 2));
      break;
    case PacketType.PING:
      buf.writeBigInt64LE(BigInt(p.t_client_us), 0);
      break;
    case PacketType.PONG:
      buf.writeBigInt64LE(BigInt(p.t_client_us), 0);
      buf.writeBigInt64LE(BigInt(p.t_server_rx_us), 8);
      buf.writeUInt32LE(p.server_proc_us, 16);
      break;
    case PacketType.RUMBLE:
      buf.writeUInt16LE(p.rumble_id, 0);
      buf.writeUInt8(p.large, 2);
      buf.writeUInt8(p.small, 3);
      break;
    default:
      throw new Error(`tipo sem payload conhecido: ${type}`);
  }
  return buf;
}

export function decodePayload(type, buf) {
  switch (type) {
    case PacketType.INPUT:
      return readInput(buf);
    case PacketType.INPUT_MOTION: {
      const p = readInput(buf);
      MOTION_FIELDS.forEach((f, i) => (p[f] = buf.readInt16LE(24 + i * 2)));
      return p;
    }
    case PacketType.PING:
      return { t_client_us: Number(buf.readBigInt64LE(0)) };
    case PacketType.PONG:
      return {
        t_client_us: Number(buf.readBigInt64LE(0)),
        t_server_rx_us: Number(buf.readBigInt64LE(8)),
        server_proc_us: buf.readUInt32LE(16),
      };
    case PacketType.RUMBLE:
      return { rumble_id: buf.readUInt16LE(0), large: buf.readUInt8(2), small: buf.readUInt8(3) };
    default:
      throw new Error(`tipo sem payload conhecido: ${type}`);
  }
}

/**
 * Decodifica um datagrama seguindo a ordem normativa da §6.5.
 * `ctx` = { role: "server" | "client", session_id, key, last_seq: number | null }.
 */
export function decodePacket(packet, ctx) {
  const err = (error) => ({ result: "error", error });

  if (packet.length < HEADER_LEN) return err("too_short");
  const header = decodeHeader(packet);
  if (header.magic !== MAGIC) return err("bad_magic");
  if (header.ver !== VERSION) return err("bad_version");
  const spec = PACKET_SPECS[header.type];
  if (!spec) return err("unknown_type");
  const recvDir = ctx.role === "server" ? DIR_C2S : DIR_S2C;
  if (spec.dir !== recvDir) return err("wrong_direction");
  if (packet.length !== HEADER_LEN + spec.payloadLen + TAG_LEN) return err("bad_length");
  if (header.session_id !== ctx.session_id) return err("unknown_session");
  if (ctx.last_seq !== null && header.seq <= ctx.last_seq) return err("stale_seq");
  const plain = open(ctx.key, spec.dir, packet);
  if (plain === null) return err("bad_tag");

  if (header.type === PacketType.INPUT || header.type === PacketType.INPUT_MOTION) {
    const motion = (plain.readUInt8(22) & FLAG_MOTION) !== 0;
    if (motion !== (header.type === PacketType.INPUT_MOTION)) return err("bad_payload");
  }
  return {
    result: "ok",
    header,
    payload: decodePayload(header.type, plain),
    payload_plain_hex: plain.toString("hex"),
  };
}

// ---------------------------------------------------------------- Canal de controle

export const CONTROL_TYPES = Object.freeze([
  "hello",
  "welcome",
  "reject",
  "config",
  "bye",
  "ping",
  "pong",
]);

export function encodeFrame(json) {
  const body = Buffer.from(json, "utf8");
  const len = Buffer.alloc(4);
  len.writeUInt32LE(body.length, 0);
  return Buffer.concat([len, body]);
}

/**
 * Decodifica um frame completo (§5.1). `validate` é a função do schema control-message
 * (ajv); mensagens de `type` desconhecido não passam por ela e resultam em `ignore`.
 */
export function decodeFrame(frame, validate) {
  if (frame.length < 4) return { result: "bad_frame" };
  const len = frame.readUInt32LE(0);
  if (len < 1 || len > MAX_FRAME_LEN) return { result: "bad_frame" };
  if (frame.length !== 4 + len) throw new Error("vector de frame incompleto");

  let message;
  try {
    message = JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(frame.subarray(4)));
  } catch {
    return { result: "bad_message" };
  }
  if (message === null || typeof message !== "object" || Array.isArray(message)) {
    return { result: "bad_message" };
  }
  if (typeof message.type !== "string") return { result: "bad_message" };
  if (!CONTROL_TYPES.includes(message.type)) return { result: "ignore" };
  if (!validate(message)) return { result: "bad_message" };
  return { result: "ok", message };
}

// ---------------------------------------------------------------- URI de pareamento

const B64URL = /^[A-Za-z0-9_-]+$/;
const IPV4 = /^(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}$/;

function b64urlBytes(s, bytes) {
  // Sem padding: ceil(bytes * 4 / 3) caracteres.
  if (!B64URL.test(s) || s.length !== Math.ceil((bytes * 4) / 3)) return null;
  const buf = Buffer.from(s, "base64url");
  if (buf.length !== bytes || buf.toString("base64url") !== s) return null;
  return buf;
}

/** Faz o parsing da URI do QR code (§4.2). */
export function parsePairingUri(uri) {
  const err = (error) => ({ result: "error", error });
  const prefix = "vynpad://pair?";
  if (!uri.startsWith(prefix)) return err("bad_scheme");

  const params = new Map();
  for (const part of uri.slice(prefix.length).split("&")) {
    const eq = part.indexOf("=");
    if (eq <= 0) return err("bad_param");
    let key, value;
    try {
      key = decodeURIComponent(part.slice(0, eq));
      value = decodeURIComponent(part.slice(eq + 1));
    } catch {
      return err("bad_param");
    }
    if (params.has(key)) return err("bad_param");
    params.set(key, value);
  }
  for (const k of ["v", "h", "p", "fp", "t", "n"]) {
    if (!params.has(k)) return err("missing_param");
  }

  if (!/^[1-9]\d{0,8}$/.test(params.get("v"))) return err("bad_param");
  if (Number(params.get("v")) !== 1) return err("unsupported_version");

  const hosts = params.get("h").split(",");
  if (!hosts.every((h) => IPV4.test(h))) return err("bad_param");

  const portStr = params.get("p");
  const port = Number(portStr);
  if (!/^[1-9]\d{0,4}$/.test(portStr) || port > 65535) return err("bad_param");

  const fp = b64urlBytes(params.get("fp"), 32);
  const token = b64urlBytes(params.get("t"), 16);
  if (!fp || !token) return err("bad_param");

  const name = params.get("n");
  const nameLen = [...name].length;
  if (nameLen < 1 || nameLen > 64) return err("bad_param");

  return {
    result: "ok",
    params: {
      version: 1,
      hosts,
      port,
      fingerprint_hex: fp.toString("hex"),
      pairing_token_hex: token.toString("hex"),
      name,
    },
  };
}
