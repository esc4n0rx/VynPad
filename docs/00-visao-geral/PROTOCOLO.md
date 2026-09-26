# Protocolo PadGaming v1 (rascunho)

> Documento vivo. A T-101 finaliza e congela a v1. Toda mudança incompatível incrementa `proto_ver`.

## 1. Portas padrão

| Porta | Protocolo | Uso |
|---|---|---|
| 47800 | TCP + TLS 1.3 | Canal de controle (pareamento, handshake, configuração) |
| 47801 | UDP | Input (client → server), rumble e pong (server → client) |
| 47802 | UDP broadcast | Descoberta de fallback |
| 5353 | mDNS | Serviço `_padgaming._tcp.local.` |

## 2. Descoberta

**mDNS**: o server anuncia `_padgaming._tcp` com TXT:
`id=<uuid>`, `name=<nome do PC>`, `ver=1`, `udp=47801`, `os=windows|linux`, `free=<slots livres>`, `fp=<8 primeiros bytes do fingerprint, hex>`.

O client anuncia `_padgaming-client._tcp` (TXT `id`, `name`, `model`) enquanto está na tela de conexão, para aparecer em "Dispositivos próximos" no server.

**Broadcast (fallback)**: o client envia `PGDISC1` para `255.255.255.255:47802`; o server responde em unicast com um JSON contendo os mesmos campos do TXT.

## 3. Pareamento (QR / PIN)

O QR code contém uma URI:

```
padgaming://pair?h=192.168.0.10&p=47800&fp=<sha256 do cert, base64url>&t=<pairing_token>&n=<nome>
```

- `pairing_token`: 128 bits aleatórios, uso único, expira em 5 min.
- O certificado TLS é autoassinado, gerado no primeiro boot (`rcgen`) e guardado no diretório de dados. O client **fixa (pinning)** o fingerprint.
- **PIN**: 6 dígitos exibidos no server. O client conecta via descoberta (TOFU no fingerprint, que o usuário pode conferir na tela) e envia o PIN. Após 5 tentativas erradas, bloqueio de 60 s.
- Com o pareamento concluído, o server emite um `device_token` (256 bits). No SQLite fica só o **hash** (SHA-256) dele.

## 4. Canal de controle (TCP/TLS)

Framing: `u32 LE length` + payload **JSON UTF-8** (baixa frequência e fácil de depurar; o Kotlin usa `kotlinx.serialization`).

| Mensagem | Direção | Campos |
|---|---|---|
| `hello` | C→S | `proto_ver`, `device_id`, `name`, `model`, `os_ver`, `caps` (`gyro`, `rumble`), `auth`: `{pairing_token}` \| `{pin}` \| `{device_token}` |
| `welcome` | S→C | `session_id` (u32), `udp_port`, `udp_key` (32 B, base64), `slot` (0–3), `device_token?`, `server_name`, `send_rate_hz` |
| `reject` | S→C | `reason`: `bad_auth` \| `no_slot` \| `version` \| `revoked` \| `locked` |
| `config` | S→C | `gyro_enabled`, `send_rate_hz`, `haptics` (dicas para o client) |
| `bye` | ambos | `reason` |
| `ping` / `pong` | ambos | keepalive TLS a cada 5 s |

## 5. Pacotes UDP

Todos os campos em **little-endian**. O cabeçalho comum (12 bytes) é autenticado como AAD:

| Offset | Tipo | Campo |
|---|---|---|
| 0 | u16 | `magic` = `0x4750` ("PG") |
| 2 | u8 | `ver` = 1 |
| 3 | u8 | `type` |
| 4 | u32 | `session_id` |
| 8 | u32 | `seq` (monotônico por direção) |

Depois vem o **payload cifrado** com ChaCha20-Poly1305 (`udp_key`; nonce de 12 bytes = `direction (u32) ‖ seq (u64)`) + **tag de 16 bytes**.

### 0x01 — InputState (C→S), payload de 24 bytes

| Offset | Tipo | Campo |
|---|---|---|
| 0 | u64 | `t_client_us` (relógio monotônico do client) |
| 8 | u16 | `buttons` (bitmask XInput: DPAD_UP = 0x0001 … Y = 0x8000; 0x0400 = Guide) |
| 10 | u8 | `lt` |
| 11 | u8 | `rt` |
| 12 | i16 | `lx` |
| 14 | i16 | `ly` |
| 16 | i16 | `rx` |
| 18 | i16 | `ry` |
| 20 | u16 | `rumble_ack_seq` |
| 22 | u8 | `flags` (bit0 = motion presente) |
| 23 | u8 | `battery` (0–100; 255 = desconhecido) |

Pacote total: 12 + 24 + 16 = **52 bytes**.

### 0x02 — InputState + Motion (C→S), payload de 36 bytes

InputState + `gx, gy, gz` (i16, rad/s × 1000) + `ax, ay, az` (i16, m/s² × 100).

### 0x10 — Ping (C→S) / 0x11 — Pong (S→C)

Payload: `t_client_us` (u64) + `t_server_rx_us` (u64, só no pong) + `server_proc_us` (u32, só no pong).

### 0x20 — Rumble (S→C), payload de 3 bytes

`large` (u8), `small` (u8), `flags` (u8). Enviado quando muda e repetido a cada 100 ms enquanto for ≠ 0, até o client confirmar via `rumble_ack_seq`.

## 6. Regras

- O server descarta pacotes com `seq` ≤ último aceito (anti-replay e desordem), com tratamento de wrap-around do u32.
- Cada pacote carrega o **estado completo**: perder um pacote não perde o input.
- Taxa de envio: imediato quando o estado muda (limite de 250 Hz) + keepalive a 10 Hz quando ocioso.
- Timeouts: 250 ms sem pacote → estado neutro; 5 s → sessão "suspensa"; 30 s → slot liberado.
- Pacotes com tag inválida são descartados em silêncio (e contados nas métricas).

## 7. Test vectors

`protocol/test-vectors/*.json`: `{name, key, header, payload_plain, packet_hex}`. Obrigatórios nos testes em Rust (T-102) e Kotlin (T-902).
