// Definição dos test vectors (PROTOCOLO.md §11). Os casos válidos são montados com a
// implementação de referência; cada caso inválido declara o erro esperado e o gerador
// confere que o decoder de referência chega exatamente nele.

import { createHash } from "node:crypto";
import { isDeepStrictEqual } from "node:util";

import {
  BUTTONS_MASK,
  DIR_C2S,
  DIR_S2C,
  FLAG_MOTION,
  PACKET_SPECS,
  PacketType,
  buttonNames,
  buttonsFromNames,
  decodeFrame,
  decodePacket,
  encodeFrame,
  encodeHeader,
  encodePayload,
  parsePairingUri,
  seal,
} from "./reference.mjs";

const KEY = Buffer.from("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f", "hex");
const OTHER_KEY = Buffer.from(
  "a0a1a2a3a4a5a6a7a8a9aaabacadaeafb0b1b2b3b4b5b6b7b8b9babbbcbdbebf",
  "hex",
);
const SESSION_ID = 0x12345678;

// ---------------------------------------------------------------- UDP

const NEUTRAL = {
  t_client_us: 1_000_000,
  buttons: 0,
  lt: 0,
  rt: 0,
  lx: 0,
  ly: 0,
  rx: 0,
  ry: 0,
  rumble_ack_id: 0,
  flags: 0,
  battery: 255,
};

/** Monta o pacote a partir de valores "brutos" (podem ser inválidos de propósito). */
function build({
  type,
  payload,
  seq,
  session_id = SESSION_ID,
  key = KEY,
  dir = PACKET_SPECS[type]?.dir ?? DIR_C2S,
  magic,
  ver,
}) {
  const header = encodeHeader({ magic, ver, type, session_id, seq });
  const plainType = PACKET_SPECS[type] ? type : PacketType.INPUT;
  return seal(key, dir, header, encodePayload(plainType, payload));
}

function receiver(role, last_seq, session_id = SESSION_ID) {
  return { role, session_id, key_hex: KEY.toString("hex"), last_seq };
}

function ctxOf(r) {
  return { ...r, key: Buffer.from(r.key_hex, "hex") };
}

function describeButtons(bits) {
  const names = buttonNames(bits);
  return names.length ? ` Botões: ${names.join(", ")}.` : "";
}

function udpValid(name, description, { role, last_seq = null, decode_only = false, ...pkt }) {
  const packet = build(pkt);
  const r = receiver(role, last_seq, pkt.session_id);
  const got = decodePacket(packet, ctxOf(r));
  if (got.result !== "ok") throw new Error(`${name}: esperado ok, veio ${got.error}`);

  const intended = { ...pkt.payload };
  if ("buttons" in intended) {
    intended.buttons &= BUTTONS_MASK;
    intended.flags &= FLAG_MOTION;
  }
  if (!isDeepStrictEqual(got.payload, intended)) {
    throw new Error(`${name}: payload decodificado diverge do pretendido`);
  }
  const masked = !isDeepStrictEqual(got.payload, pkt.payload);
  if (masked !== decode_only) throw new Error(`${name}: decode_only incoerente`);

  const bits = "buttons" in got.payload ? describeButtons(got.payload.buttons) : "";
  return {
    name,
    description: description + bits,
    ...(decode_only ? { decode_only: true } : {}),
    receiver: r,
    packet_hex: packet.toString("hex"),
    expect: got,
  };
}

function udpInvalid(name, description, error, { role = "server", last_seq = null, packet }) {
  const r = receiver(role, last_seq);
  const got = decodePacket(packet, ctxOf(r));
  if (got.result !== "error" || got.error !== error) {
    throw new Error(`${name}: esperado ${error}, veio ${got.error ?? got.result}`);
  }
  return { name, description, receiver: r, packet_hex: packet.toString("hex"), expect: got };
}

function flipByte(buf, index, mask) {
  const out = Buffer.from(buf);
  out[index] ^= mask;
  return out;
}

function udpVectors() {
  const typical = {
    t_client_us: 5_123_456_789,
    buttons: buttonsFromNames(["a", "right_shoulder", "dpad_left"]),
    lt: 0,
    rt: 200,
    lx: 12000,
    ly: -8000,
    rx: -1,
    ry: 32767,
    rumble_ack_id: 3,
    flags: 0,
    battery: 87,
  };
  const typicalPacket = (seq, extra = {}) =>
    build({ type: PacketType.INPUT, payload: typical, seq, ...extra });

  const valid = [
    udpValid("input_neutral", "InputState em repouso, primeiro pacote da sessão (seq 0).", {
      role: "server",
      type: PacketType.INPUT,
      seq: 0,
      payload: NEUTRAL,
    }),
    udpValid("input_typical", "InputState típico, logo depois do seq 41.", {
      role: "server",
      type: PacketType.INPUT,
      seq: 42,
      last_seq: 41,
      payload: typical,
    }),
    udpValid(
      "input_extremes",
      "Todos os botões válidos, gatilhos no máximo e analógicos nos limites de i16; lacuna de seq (perda) é aceita.",
      {
        role: "server",
        type: PacketType.INPUT,
        seq: 1000,
        last_seq: 998,
        payload: {
          t_client_us: 7_777_777,
          buttons: BUTTONS_MASK,
          lt: 255,
          rt: 255,
          lx: -32768,
          ly: 32767,
          rx: 32767,
          ry: -32768,
          rumble_ack_id: 0xffff,
          flags: 0,
          battery: 100,
        },
      },
    ),
    udpValid(
      "input_large_values",
      "session_id máximo (2^31-1), seq perto do limite de u32 e t_client_us = 2^53-1: exige leitura sem sinal.",
      {
        role: "server",
        type: PacketType.INPUT,
        session_id: 0x7fffffff,
        seq: 0xfffffffe,
        last_seq: 0xfffffffd,
        payload: { ...NEUTRAL, t_client_us: 9_007_199_254_740_991 },
      },
    ),
    udpValid(
      "input_reserved_bits_ignored",
      "Bit reservado 0x0800 em buttons e bit 7 em flags ligados no fio: o decoder mascara (buttons & 0xF7FF, flags & 0x01).",
      {
        role: "server",
        type: PacketType.INPUT,
        seq: 7,
        last_seq: 6,
        decode_only: true,
        payload: { ...NEUTRAL, buttons: 0x0800 | 0x1000, flags: 0x80 },
      },
    ),
    udpValid("input_motion", "InputState + Motion com o aparelho quase parado (gravidade em Y).", {
      role: "server",
      type: PacketType.INPUT_MOTION,
      seq: 10,
      last_seq: 9,
      payload: {
        ...NEUTRAL,
        t_client_us: 2_000_000,
        buttons: buttonsFromNames(["b"]),
        flags: FLAG_MOTION,
        battery: 50,
        gx: 1571,
        gy: -250,
        gz: 0,
        ax: 12,
        ay: 981,
        az: -35,
      },
    }),
    udpValid("input_motion_saturated", "Motion com todos os eixos saturados nos limites de i16.", {
      role: "server",
      type: PacketType.INPUT_MOTION,
      seq: 11,
      last_seq: 10,
      payload: {
        ...NEUTRAL,
        flags: FLAG_MOTION,
        gx: 32767,
        gy: -32768,
        gz: 32767,
        ax: -32768,
        ay: 32767,
        az: -32768,
      },
    }),
    udpValid("ping", "Ping do client.", {
      role: "server",
      type: PacketType.PING,
      seq: 43,
      last_seq: 42,
      payload: { t_client_us: 123_456_789_012 },
    }),
    udpValid("pong", "Pong do server, primeiro pacote S→C da sessão (direção 1 no nonce).", {
      role: "client",
      type: PacketType.PONG,
      seq: 0,
      payload: { t_client_us: 123_456_789_012, t_server_rx_us: 98_765_432_101, server_proc_us: 37 },
    }),
    udpValid("rumble_on", "Rumble ligado (rumble_id 1).", {
      role: "client",
      type: PacketType.RUMBLE,
      seq: 5,
      last_seq: 4,
      payload: { rumble_id: 1, large: 255, small: 128 },
    }),
    udpValid(
      "rumble_off",
      "Rumble desligado (large = small = 0); o client aplica e para os motores.",
      {
        role: "client",
        type: PacketType.RUMBLE,
        seq: 6,
        last_seq: 5,
        payload: { rumble_id: 2, large: 0, small: 0 },
      },
    ),
  ];

  const input42 = typicalPacket(42);
  const invalid = [
    udpInvalid("too_short_empty", "Datagrama vazio.", "too_short", {
      packet: Buffer.alloc(0),
    }),
    udpInvalid("too_short_header", "Só 11 bytes (cabeçalho incompleto).", "too_short", {
      packet: input42.subarray(0, 11),
    }),
    udpInvalid(
      "bad_magic_big_endian",
      "magic gravado em big-endian (bytes 50 56): erro clássico de endianness.",
      "bad_magic",
      { packet: build({ type: PacketType.INPUT, payload: typical, seq: 42, magic: 0x5650 }) },
    ),
    udpInvalid("bad_version", "ver = 2 (pacote bem formado e selado).", "bad_version", {
      packet: build({ type: PacketType.INPUT, payload: typical, seq: 42, ver: 2 }),
    }),
    udpInvalid("unknown_type", "type 0x03 não existe na v1.", "unknown_type", {
      packet: build({ type: 0x03, payload: typical, seq: 42 }),
    }),
    udpInvalid(
      "wrong_direction_at_server",
      "Server recebendo um Rumble (tipo S→C).",
      "wrong_direction",
      {
        packet: build({
          type: PacketType.RUMBLE,
          payload: { rumble_id: 1, large: 1, small: 1 },
          seq: 1,
        }),
      },
    ),
    udpInvalid(
      "wrong_direction_at_client",
      "Client recebendo um InputState (tipo C→S).",
      "wrong_direction",
      { role: "client", packet: input42 },
    ),
    udpInvalid("bad_length_truncated", "InputState sem o último byte (51 bytes).", "bad_length", {
      packet: input42.subarray(0, 51),
    }),
    udpInvalid(
      "bad_length_trailing_byte",
      "InputState com 1 byte extra no fim (53 bytes).",
      "bad_length",
      {
        packet: Buffer.concat([input42, Buffer.from([0])]),
      },
    ),
    udpInvalid(
      "bad_length_motion_type_input_size",
      "type 0x02 com o tamanho de um InputState (52 bytes).",
      "bad_length",
      {
        packet: Buffer.concat([
          flipByte(input42.subarray(0, 12), 3, 0x01 ^ 0x02),
          input42.subarray(12),
        ]),
      },
    ),
    udpInvalid("unknown_session", "session_id diferente do da sessão ativa.", "unknown_session", {
      packet: typicalPacket(42, { session_id: SESSION_ID + 1 }),
    }),
    udpInvalid("stale_seq_replay", "Mesmo seq do último aceito (replay).", "stale_seq", {
      last_seq: 42,
      packet: input42,
    }),
    udpInvalid("stale_seq_older", "seq menor que o último aceito (fora de ordem).", "stale_seq", {
      last_seq: 43,
      packet: input42,
    }),
    udpInvalid(
      "stale_seq_no_wraparound",
      "seq 0 depois de 0xFFFFFFFF: não existe wrap-around, o pacote é descartado.",
      "stale_seq",
      { last_seq: 0xffffffff, packet: typicalPacket(0) },
    ),
    udpInvalid("bad_tag_flipped_tag", "Último bit da tag invertido.", "bad_tag", {
      last_seq: 41,
      packet: flipByte(input42, input42.length - 1, 0x01),
    }),
    udpInvalid("bad_tag_flipped_ciphertext", "Primeiro byte do ciphertext adulterado.", "bad_tag", {
      last_seq: 41,
      packet: flipByte(input42, 12, 0x80),
    }),
    udpInvalid(
      "bad_tag_tampered_header",
      "seq do cabeçalho trocado de 42 para 43 depois de selar (AAD e nonce mudam).",
      "bad_tag",
      { last_seq: 41, packet: flipByte(input42, 8, 42 ^ 43) },
    ),
    udpInvalid("bad_tag_wrong_key", "Selado com outra chave de sessão.", "bad_tag", {
      last_seq: 41,
      packet: typicalPacket(42, { key: OTHER_KEY }),
    }),
    udpInvalid(
      "bad_tag_wrong_nonce_direction",
      "InputState (C→S) selado com direction = 1 no nonce.",
      "bad_tag",
      { last_seq: 41, packet: typicalPacket(42, { dir: DIR_S2C }) },
    ),
    udpInvalid(
      "bad_payload_input_with_motion_flag",
      "type 0x01 com o bit de motion ligado em flags.",
      "bad_payload",
      {
        packet: build({
          type: PacketType.INPUT,
          payload: { ...typical, flags: FLAG_MOTION },
          seq: 42,
        }),
      },
    ),
    udpInvalid(
      "bad_payload_motion_without_flag",
      "type 0x02 com o bit de motion desligado em flags.",
      "bad_payload",
      {
        packet: build({
          type: PacketType.INPUT_MOTION,
          payload: { ...typical, flags: 0, gx: 0, gy: 0, gz: 0, ax: 0, ay: 981, az: 0 },
          seq: 42,
        }),
      },
    ),
  ];

  return {
    $schema: "../schemas/udp-vectors.schema.json",
    format: 1,
    description:
      "Pacotes UDP do protocolo VynPad v1 (PROTOCOLO.md §6). Gerado por protocol/tools/gen.mjs — não edite à mão.",
    vectors: [...valid, ...invalid],
  };
}

// ---------------------------------------------------------------- Canal de controle

const DEVICE_ID = "3f2b8c1a-9d4e-4b7f-8a6c-1e2d3c4b5a69";
const SERVER_ID = "5b0c8a1e-2f4d-4c7a-9e1b-0d3f6a7b8c9d";
const PAIRING_TOKEN = Buffer.from("101112131415161718191a1b1c1d1e1f", "hex");
const DEVICE_TOKEN = createHash("sha256").update("vynpad device token").digest();
const UDP_KEY = KEY;
const CONFIG = {
  slot: 1,
  send_rate_hz: 250,
  gyro_enabled: false,
  rumble_enabled: true,
  touch_haptics: true,
};

const HELLO = {
  type: "hello",
  proto_ver: 1,
  device_id: DEVICE_ID,
  name: "Galaxy do Paulo",
  model: "SM-S921B",
  os_ver: "Android 14",
  caps: { gyro: true, rumble: true },
  auth: { method: "pairing_token", token: PAIRING_TOKEN.toString("base64url") },
};

function control(validate, name, description, frame, result) {
  const got = decodeFrame(frame, validate);
  if (got.result !== result) throw new Error(`${name}: esperado ${result}, veio ${got.result}`);
  return { name, description, frame_hex: frame.toString("hex"), expect: got };
}

function controlVectors(validate) {
  const ok = (name, description, message) =>
    control(validate, name, description, encodeFrame(JSON.stringify(message)), "ok");
  const bad = (name, description, message, result = "bad_message") =>
    control(validate, name, description, encodeFrame(JSON.stringify(message)), result);
  const raw = (name, description, frame, result) =>
    control(validate, name, description, frame, result);

  return {
    $schema: "../schemas/control-vectors.schema.json",
    format: 1,
    description:
      "Frames do canal de controle VynPad v1 (PROTOCOLO.md §5). Gerado por protocol/tools/gen.mjs — não edite à mão.",
    vectors: [
      ok("hello_pairing_token", "hello do primeiro pareamento via QR code.", HELLO),
      ok("hello_pin", "hello do pareamento por PIN (zero à esquerda preservado).", {
        ...HELLO,
        auth: { method: "pin", pin: "042917" },
      }),
      ok("hello_device_token", "hello de reconexão de um dispositivo já pareado, sem gyro.", {
        ...HELLO,
        app_ver: "1.0.0",
        caps: { gyro: false, rumble: true },
        auth: { method: "device_token", token: DEVICE_TOKEN.toString("base64url") },
      }),
      ok(
        "hello_unknown_fields_ignored",
        "Campos desconhecidos (inclusive aninhados) são ignorados.",
        {
          ...HELLO,
          future_field: { anything: [1, 2, 3] },
          caps: { gyro: true, rumble: true, future_cap: true },
        },
      ),
      ok("welcome_first_pairing", "welcome com device_token (só no primeiro pareamento).", {
        type: "welcome",
        proto_ver: 1,
        session_id: SESSION_ID,
        udp_port: 47801,
        udp_key: UDP_KEY.toString("base64url"),
        device_token: DEVICE_TOKEN.toString("base64url"),
        server_id: SERVER_ID,
        server_name: "PC-Sala",
        config: CONFIG,
      }),
      ok(
        "welcome_reconnect",
        "welcome de reconexão (sem device_token), sessão com session_id máximo.",
        {
          type: "welcome",
          proto_ver: 1,
          session_id: 0x7fffffff,
          udp_port: 47801,
          udp_key: OTHER_KEY.toString("base64url"),
          server_id: SERVER_ID,
          server_name: "PC-Sala",
          config: { ...CONFIG, slot: 0, gyro_enabled: true },
        },
      ),
      ok("reject_bad_auth", "reject por credencial inválida.", {
        type: "reject",
        reason: "bad_auth",
      }),
      ok("reject_locked", "reject por excesso de PINs errados.", {
        type: "reject",
        reason: "locked",
        retry_after_s: 60,
      }),
      ok("reject_version", "reject por proto_ver não suportado.", {
        type: "reject",
        reason: "version",
        supported_versions: [1],
        message: "proto_ver 2 não suportado",
      }),
      ok("reject_unknown_reason", "reason desconhecido é válido (tratado como erro genérico).", {
        type: "reject",
        reason: "maintenance",
      }),
      ok("config", "config com o estado completo (gyro ligado pela UI).", {
        type: "config",
        ...CONFIG,
        gyro_enabled: true,
        send_rate_hz: 120,
      }),
      ok("bye_user", "bye iniciado pelo usuário.", { type: "bye", reason: "user" }),
      ok("bye_seq_exhausted", "bye por esgotamento do seq (§6.3).", {
        type: "bye",
        reason: "seq_exhausted",
      }),
      ok("ping", "Keepalive do canal de controle.", { type: "ping" }),
      ok("pong", "Resposta ao keepalive.", { type: "pong" }),
      bad(
        "unknown_type_ignored",
        "type desconhecido é ignorado.",
        { type: "telemetry", x: 1 },
        "ignore",
      ),
      raw("frame_zero_length", "length = 0.", Buffer.from("00000000", "hex"), "bad_frame"),
      raw(
        "frame_too_large",
        "length = 16385 (acima do máximo de 16384); o erro sai só pelo cabeçalho do frame.",
        Buffer.from("01400000", "hex"),
        "bad_frame",
      ),
      raw("invalid_json", "JSON truncado.", encodeFrame('{"type":"ping"'), "bad_message"),
      raw(
        "invalid_utf8",
        "Byte 0xFF no meio do JSON (UTF-8 inválido).",
        (() => {
          const f = encodeFrame('{"type":"bye","reason":"user"}');
          f[f.length - 4] = 0xff;
          return f;
        })(),
        "bad_message",
      ),
      bad("not_an_object", "O JSON precisa ser um objeto.", [{ type: "ping" }]),
      bad("missing_type", "Objeto sem o discriminador type.", { reason: "user" }),
      bad("hello_missing_auth", "hello sem auth.", (({ auth: _auth, ...rest }) => rest)(HELLO)),
      bad("hello_pin_five_digits", "PIN com 5 dígitos.", {
        ...HELLO,
        auth: { method: "pin", pin: "42917" },
      }),
      bad("hello_unknown_auth_method", "auth.method desconhecido.", {
        ...HELLO,
        auth: { method: "password", password: "hunter2" },
      }),
      bad("hello_pairing_token_wrong_length", "pairing_token com 32 bytes em vez de 16.", {
        ...HELLO,
        auth: { method: "pairing_token", token: DEVICE_TOKEN.toString("base64url") },
      }),
      bad(
        "hello_token_with_padding",
        "Token em base64 com padding (=) em vez de base64url sem padding.",
        {
          ...HELLO,
          auth: { method: "pairing_token", token: PAIRING_TOKEN.toString("base64") },
        },
      ),
      bad("welcome_session_id_zero", "session_id 0 é reservado.", {
        type: "welcome",
        proto_ver: 1,
        session_id: 0,
        udp_port: 47801,
        udp_key: UDP_KEY.toString("base64url"),
        server_id: SERVER_ID,
        server_name: "PC-Sala",
        config: CONFIG,
      }),
      bad("welcome_slot_out_of_range", "slot 4 (só existem 0–3).", {
        type: "welcome",
        proto_ver: 1,
        session_id: SESSION_ID,
        udp_port: 47801,
        udp_key: UDP_KEY.toString("base64url"),
        server_id: SERVER_ID,
        server_name: "PC-Sala",
        config: { ...CONFIG, slot: 4 },
      }),
      bad("config_incomplete", "config sem touch_haptics (o estado é sempre completo).", {
        type: "config",
        slot: 1,
        send_rate_hz: 250,
        gyro_enabled: false,
        rumble_enabled: true,
      }),
      bad("config_send_rate_too_high", "send_rate_hz acima de 500.", {
        type: "config",
        ...CONFIG,
        send_rate_hz: 1000,
      }),
    ],
  };
}

// ---------------------------------------------------------------- URI de pareamento

const FINGERPRINT = createHash("sha256").update("vynpad test certificate").digest();
const FP = FINGERPRINT.toString("base64url");
const TK = PAIRING_TOKEN.toString("base64url");

function uri(name, description, value, expected) {
  const got = parsePairingUri(value);
  const result = got.result === "ok" ? "ok" : got.error;
  if (result !== expected) throw new Error(`${name}: esperado ${expected}, veio ${result}`);
  return { name, description, uri: value, expect: got };
}

function pairingUriVectors() {
  const base = `vynpad://pair?v=1&h=192.168.0.10&p=47800&fp=${FP}&t=${TK}&n=PC-Sala`;
  return {
    $schema: "../schemas/pairing-uri-vectors.schema.json",
    format: 1,
    description:
      "URIs de pareamento do QR code (PROTOCOLO.md §4.2). Gerado por protocol/tools/gen.mjs — não edite à mão.",
    vectors: [
      uri("single_host", "URI mínima com um IPv4.", base, "ok"),
      uri(
        "multiple_hosts_utf8_name",
        "Vários IPv4 em ordem de preferência e nome UTF-8 com espaços (%20).",
        `vynpad://pair?v=1&h=192.168.0.10,10.0.0.5,172.16.1.2&p=47800&fp=${FP}&t=${TK}&n=Sala%20de%20Jogos%20%E2%80%94%20Jo%C3%A3o`,
        "ok",
      ),
      uri(
        "any_param_order_and_unknown_param",
        "Ordem dos parâmetros é livre e parâmetros desconhecidos são ignorados.",
        `vynpad://pair?n=PC-Sala&x=future&t=${TK}&fp=${FP}&p=50000&h=10.0.0.1&v=1`,
        "ok",
      ),
      uri(
        "plus_is_not_space",
        "'+' não é convertido em espaço: o nome é literalmente 'PC+Sala'.",
        base.replace("n=PC-Sala", "n=PC+Sala"),
        "ok",
      ),
      uri(
        "wrong_scheme",
        "Esquema https em vez de vynpad.",
        base.replace("vynpad://", "https://"),
        "bad_scheme",
      ),
      uri(
        "wrong_action",
        "Ação connect em vez de pair.",
        base.replace("//pair?", "//connect?"),
        "bad_scheme",
      ),
      uri(
        "missing_fingerprint",
        "Sem o parâmetro fp.",
        base.replace(`&fp=${FP}`, ""),
        "missing_param",
      ),
      uri("missing_version", "Sem o parâmetro v.", base.replace("v=1&", ""), "missing_param"),
      uri("unsupported_version", "v=2.", base.replace("v=1", "v=2"), "unsupported_version"),
      uri(
        "fingerprint_wrong_length",
        "fp com 31 bytes.",
        base.replace(FP, FINGERPRINT.subarray(0, 31).toString("base64url")),
        "bad_param",
      ),
      uri(
        "token_with_padding",
        "t em base64 com padding (=).",
        base.replace(TK, encodeURIComponent(PAIRING_TOKEN.toString("base64"))),
        "bad_param",
      ),
      uri(
        "host_not_ipv4",
        "h com hostname em vez de IPv4.",
        base.replace("192.168.0.10", "meu-pc.local"),
        "bad_param",
      ),
      uri(
        "host_octet_out_of_range",
        "h com octeto 256.",
        base.replace("192.168.0.10", "192.168.0.256"),
        "bad_param",
      ),
      uri("port_out_of_range", "p = 70000.", base.replace("p=47800", "p=70000"), "bad_param"),
      uri("duplicated_param", "Parâmetro p repetido.", `${base}&p=47801`, "bad_param"),
      uri(
        "name_too_long",
        "n com 65 caracteres.",
        base.replace("n=PC-Sala", `n=${"a".repeat(65)}`),
        "bad_param",
      ),
    ],
  };
}

/** Todos os arquivos de vectors, indexados pelo nome do arquivo. */
export function buildAll(validateControl) {
  return {
    "udp.json": udpVectors(),
    "control.json": controlVectors(validateControl),
    "pairing-uri.json": pairingUriVectors(),
  };
}
