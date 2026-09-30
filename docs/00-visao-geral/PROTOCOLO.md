# Protocolo PadGaming v1

> **Status: v1 congelado** (T-101, 2026-09-27). Toda mudança segue a [política de versionamento](#10-versionamento).
> Contrato entre o server (Rust, T-102) e o client Android (Kotlin, T-902). Os [test vectors](#11-test-vectors) em `protocol/test-vectors/` são normativos: em caso de divergência entre este texto e os vectors, abra uma issue — nenhum dos dois "ganha" sozinho.

Palavras **DEVE**, **NÃO DEVE** e **PODE** têm o sentido da RFC 2119.

## 1. Convenções

- **Endianness:** todo inteiro binário (UDP e framing do canal de controle) é **little-endian**.
- **Tipos:** `u8/u16/u32` sem sinal, `i16/i64` com sinal (complemento de dois). Não existe `u64` no protocolo.
- **Faixas pensadas para Kotlin:** campos que chegam ao JSON ou que o Kotlin guarda em `Int` têm faixa ≤ 2³¹−1 (ex.: `session_id`). Ver [§12](#12-notas-para-a-implementação-kotlin).
- **Binário em texto** (JSON, QR, TXT): **base64url sem padding** (RFC 4648 §5), exceto onde se diz "hex" (minúsculo).
- **Relógios:** `*_us` são microssegundos de um relógio **monotônico** local (Android: `SystemClock.elapsedRealtimeNanos() / 1000`; server: `Instant` desde o boot do processo). Os relógios de client e server não são comparáveis entre si.
- **Bits reservados:** quem envia DEVE zerar; quem recebe DEVE ignorar (mascarar).

## 2. Portas padrão

| Porta | Protocolo | Uso |
|---|---|---|
| 47800 | TCP + TLS 1.3 | Canal de controle (pareamento, handshake, configuração) |
| 47801 | UDP | Input e ping (client → server), rumble e pong (server → client) |
| 47802 | UDP broadcast | Descoberta de fallback |
| 5353 | mDNS | Serviço `_padgaming._tcp.local.` |

As portas 47800/47801 podem mudar nas configurações do server; o client sempre usa as portas anunciadas (mDNS/broadcast/QR) e a `udp_port` do `welcome`.

## 3. Descoberta

### 3.1 mDNS / DNS-SD

O server anuncia `_padgaming._tcp.local.` na porta TCP de controle, com TXT (todos os valores são texto UTF-8):

| Chave | Exemplo | Significado |
|---|---|---|
| `id` | `5b0c…` (UUID) | Identificador estável do server |
| `name` | `PC-Sala` | Nome exibido (≤ 63 bytes) |
| `ver` | `1` | `proto_ver` suportado |
| `udp` | `47801` | Porta UDP de input |
| `os` | `windows` \| `linux` | SO do server |
| `free` | `3` | Slots livres (0–4) |
| `fp` | `3fa1…` (16 hex) | 8 primeiros bytes do fingerprint do certificado, em hex |

O client anuncia `_padgaming-client._tcp.local.` enquanto está na tela de conexão, com TXT `id` (device_id), `name`, `model`. A porta anunciada é a da socket UDP local do client e é **só informativa** (o server nunca se conecta a ela).

> Nota: o nome `_padgaming-client` passa dos 15 caracteres que a RFC 6763 recomenda para o tipo de serviço. O server (`mdns-sd`) é configurado para aceitá-lo; o client Android deve conferir se o `NsdManager` também aceita (T-903).

### 3.2 Broadcast (fallback)

- O client envia os 7 bytes ASCII `PGDISC1` para `255.255.255.255:47802`.
- O server responde em **unicast** para a origem com um JSON UTF-8 (sem framing) que segue `protocol/schemas/discovery-response.schema.json`:

```json
{"id":"5b0c8a1e-2f4d-4c7a-9e1b-0d3f6a7b8c9d","name":"PC-Sala","ver":1,"tcp":47800,"udp":47801,"os":"windows","free":3,"fp":"3fa1c2d4e5f60718"}
```

Datagramas diferentes de `PGDISC1` são ignorados. A resposta é só uma dica: a confiança vem do TLS com pinning (§4).

## 4. Pareamento e TLS

### 4.1 Certificado

- Gerado no primeiro boot (`rcgen`, ECDSA P-256) e guardado no diretório de dados. **TLS 1.3 apenas**.
- **Fingerprint** = SHA-256 do certificado em DER (32 bytes).
- Depois de parear, o client **fixa (pinning)** o fingerprint e recusa qualquer outro. Se o server regenerar o certificado, é preciso parear de novo.

### 4.2 URI do QR code

```
padgaming://pair?v=1&h=192.168.0.10,10.0.0.5&p=47800&fp=<32 B base64url>&t=<16 B base64url>&n=PC%20Sala
```

| Parâmetro | Obrigatório | Regra |
|---|---|---|
| `v` | sim | Versão da URI = `1`. Outro inteiro → erro `unsupported_version`. |
| `h` | sim | Um ou mais IPv4 separados por vírgula, na ordem de preferência (o PC pode ter várias interfaces). O client tenta um por vez. |
| `p` | sim | Porta TCP de controle (1–65535). |
| `fp` | sim | Fingerprint do certificado (43 caracteres → 32 bytes). |
| `t` | sim | `pairing_token` (22 caracteres → 16 bytes). |
| `n` | sim | Nome do server, 1–64 caracteres, codificado com percent-encoding UTF-8. |

- O encoder DEVE usar `%20` para espaço e NÃO DEVE usar `+` (o `Uri` do Android não converte `+` em espaço); o decoder NÃO DEVE converter `+`.
- Parâmetros desconhecidos são ignorados; parâmetros repetidos → `bad_param`.
- Erros de parsing: `bad_scheme` (esquema ≠ `padgaming` ou ação ≠ `pair`), `missing_param`, `bad_param`, `unsupported_version`.

### 4.3 Credenciais

| Credencial | Formato | Regras |
|---|---|---|
| `pairing_token` | 128 bits aleatórios | Uso único, expira em 5 min. Transportado no QR. |
| `pin` | 6 dígitos (texto, preserva zeros à esquerda) | Exibido no server, expira em 2 min, uso único. Após **5 erros** (contagem global do server e também por IP), bloqueio de **60 s** (`reject` `locked`). |
| `device_token` | 256 bits aleatórios | Emitido no `welcome` do primeiro pareamento. O server guarda só o **SHA-256**. O client guarda junto com o fingerprint fixado. |

**Fluxo por PIN:** o client conecta via descoberta confiando no primeiro certificado (TOFU); as duas telas mostram os 8 primeiros bytes do fingerprint (`fp` do mDNS) para o usuário conferir antes de digitar o PIN.

## 5. Canal de controle (TCP/TLS)

### 5.1 Framing

Cada mensagem é `length (u32 LE)` + `length` bytes de **JSON UTF-8** contendo um objeto.

- `length` DEVE estar entre 1 e **16384**. Fora disso: erro `bad_frame` e a conexão é fechada.
- JSON inválido, que não é objeto ou que viola o schema: erro `bad_message`. Antes do `hello` aceito o server responde `reject` `bad_request` e fecha; depois, só registra e descarta a mensagem.
- Os encoders podem usar qualquer ordem de chaves e espaçamento; o `frame_hex` dos vectors é só um exemplo de encoding válido.

### 5.2 Mensagens

Toda mensagem tem o discriminador `"type"`. O schema normativo é `protocol/schemas/control-message.schema.json`.

- Campos **desconhecidos** DEVEM ser ignorados.
- Mensagens de `type` **desconhecido** DEVEM ser ignoradas (resultado `ignore` nos vectors), exceto como primeira mensagem do client.
- Valores de enum desconhecidos em `reject.reason` e `bye.reason` DEVEM ser tratados como um erro genérico (não como falha de parsing).

| `type` | Direção | Campos (obrigatórios em negrito) |
|---|---|---|
| `hello` | C→S | **`proto_ver`**, **`device_id`** (UUID), **`name`** (1–64), **`model`** (≤ 64), **`os_ver`** (≤ 32), `app_ver` (≤ 32), **`caps`** `{gyro, rumble}` (bool), **`auth`** (ver abaixo) |
| `welcome` | S→C | **`proto_ver`**, **`session_id`** (1–2147483647), **`udp_port`**, **`udp_key`** (32 B), `device_token` (32 B, só no primeiro pareamento), **`server_id`** (UUID), **`server_name`**, **`config`** (objeto `config` sem `type`) |
| `reject` | S→C | **`reason`**: `bad_request` \| `bad_auth` \| `locked` \| `revoked` \| `no_slot` \| `version`; `retry_after_s` (com `locked`); `supported_versions` (com `version`); `message` (depuração, ≤ 256) |
| `config` | S→C | **`slot`** (0–3), **`send_rate_hz`** (10–500), **`gyro_enabled`**, **`rumble_enabled`**, **`touch_haptics`** — sempre o estado completo |
| `bye` | ambos | **`reason`**: `user` \| `shutdown` \| `revoked` \| `kicked` \| `timeout` \| `seq_exhausted` \| `error`; `message` (≤ 256) |
| `ping` / `pong` | ambos | — |

`auth` tem o discriminador `"method"`:

```json
{"method":"pairing_token","token":"<16 B base64url>"}
{"method":"pin","pin":"042917"}
{"method":"device_token","token":"<32 B base64url>"}
```

`config` é uma dica para o client: `send_rate_hz` é a taxa **máxima** de envio de input; `gyro_enabled` diz se o server quer pacotes `0x02`; `rumble_enabled` diz se o server vai mandar `0x20`; `touch_haptics` sugere vibração local ao tocar em botões. `slot` é o índice do controle (para o client exibir "Jogador N").

### 5.3 Sequência e regras

1. Client abre TLS e DEVE mandar `hello` em até **10 s**; senão o server fecha.
2. Server responde `welcome` (sessão criada) ou `reject` e fecha a conexão.
   - `version`: `proto_ver` não suportado; `supported_versions` lista os aceitos.
   - `revoked`: o `device_id` é conhecido e foi revogado. `bad_auth`: credencial inválida ou expirada.
   - `no_slot`: os 4 slots estão ocupados.
3. Cada `welcome` cria uma **sessão nova**: `session_id` e `udp_key` novos (aleatórios); os contadores `seq` recomeçam (§6.3).
4. Keepalive: quem não enviou nada por **5 s** manda `ping`; quem recebe `ping` responde `pong`. Sem receber nada por **15 s**, fecha a conexão.
5. Depois do `welcome`, o server PODE mandar `config` a qualquer momento (ex.: o usuário ligou o gyro).
6. `bye` precede um fechamento limpo. O server manda `revoked` quando o usuário revoga o dispositivo e `kicked` quando o desconecta pela UI.
7. **Reconexão:** se o TLS cair, o client reconecta com `device_token` com backoff 0,5 s → 1 s → 2 s → 4 s → 5 s (teto). O server reserva o slot do `device_id` por **30 s** após a queda do TLS; uma reconexão nesse prazo volta ao mesmo slot (com uma sessão nova).

## 6. Pacotes UDP

### 6.1 Formato geral

```
+----------------- 12 bytes -----------------+---- N bytes ----+-- 16 bytes --+
| magic u16 | ver u8 | type u8 | session_id u32 | seq u32 | payload cifrado |     tag      |
+--------------------------------------------+-----------------+--------------+
            cabeçalho (em claro, autenticado como AAD)
```

| Offset | Tipo | Campo | Valor |
|---|---|---|---|
| 0 | u16 | `magic` | `0x4750` (bytes `50 47` = ASCII "PG") |
| 2 | u8 | `ver` | `1` |
| 3 | u8 | `type` | ver tabela abaixo |
| 4 | u32 | `session_id` | do `welcome` |
| 8 | u32 | `seq` | contador por direção (§6.3) |

| `type` | Nome | Direção | Payload | Pacote total |
|---|---|---|---|---|
| `0x01` | InputState | C→S | 24 B | **52 B** |
| `0x02` | InputState + Motion | C→S | 36 B | **64 B** |
| `0x10` | Ping | C→S | 8 B | **36 B** |
| `0x11` | Pong | S→C | 20 B | **48 B** |
| `0x20` | Rumble | S→C | 4 B | **32 B** |

O tamanho de cada tipo é **exato**: não existe padding nem extensão no fim do payload.

### 6.2 Criptografia

- AEAD **ChaCha20-Poly1305** (RFC 8439), chave = `udp_key` (32 bytes, nova a cada sessão).
- **AAD** = os 12 bytes do cabeçalho exatamente como transmitidos.
- **Nonce** (12 bytes) = `direction (u32 LE) ‖ seq (u32 LE) ‖ 00 00 00 00`, com `direction` = `0` para C→S e `1` para S→C.
- O pacote é `cabeçalho ‖ ciphertext ‖ tag`. O ciphertext tem o mesmo tamanho do payload.

Como a chave vale uma sessão só e o nonce combina direção e `seq`, **um nonce nunca se repete** — desde que o `seq` nunca volte (§6.3).

### 6.3 `seq`, anti-replay e wrap-around

- Cada lado mantém **um** contador por sessão para os pacotes que envia (compartilhado entre todos os tipos daquela direção). O primeiro pacote usa `seq = 0`, e cada pacote enviado — inclusive retransmissões — incrementa 1.
- **Não há wrap-around.** O `seq` NÃO DEVE voltar a 0 dentro da sessão (isso repetiria o nonce). O último `seq` utilizável é `0xFFFFFFFF`; depois de enviá-lo, o lado DEVE parar de enviar UDP e encerrar a sessão (`bye` `seq_exhausted`), e o client reconecta e recebe uma chave nova. A 250 Hz isso só acontece depois de ~198 dias contínuos.
- Quem recebe guarda `last_seq` (o maior `seq` aceito; começa "vazio") e aceita um pacote só se `seq > last_seq` (comparação **sem sinal** de 32 bits, sem aritmética modular). Lacunas são permitidas (perda); `seq` igual ou menor é descartado (replay ou fora de ordem).
- `last_seq` só é atualizado **depois** que o pacote passa por todas as validações (§6.5).
- O server grava o endereço de origem do último pacote autenticado da sessão e manda os pacotes S→C para lá (cobre troca de IP/porta no roaming do Wi-Fi). Antes do primeiro pacote C→S válido o server não envia UDP.

### 6.4 Payloads

#### `0x01` — InputState (C→S), 24 bytes

| Offset | Tipo | Campo | Regras |
|---|---|---|---|
| 0 | i64 | `t_client_us` | Relógio monotônico do client, ≥ 0 |
| 8 | u16 | `buttons` | Bitmask XInput (§7). Bit `0x0800` reservado |
| 10 | u8 | `lt` | Gatilho esquerdo 0–255 |
| 11 | u8 | `rt` | Gatilho direito 0–255 |
| 12 | i16 | `lx` | Analógico esquerdo X (−32768 = esquerda, 32767 = direita) |
| 14 | i16 | `ly` | Analógico esquerdo Y (−32768 = baixo, 32767 = **cima**, convenção XInput) |
| 16 | i16 | `rx` | Analógico direito X |
| 18 | i16 | `ry` | Analógico direito Y |
| 20 | u16 | `rumble_ack_id` | `rumble_id` do último Rumble aplicado; `0` = nenhum |
| 22 | u8 | `flags` | bit 0 = motion presente (DEVE ser 1 em `0x02` e 0 em `0x01`); bits 1–7 reservados |
| 23 | u8 | `battery` | 0–100 (%); `255` = desconhecido; 101–254 são tratados como desconhecido |

Os analógicos vão **brutos** (sem deadzone nem curva): o mapeamento é aplicado no server (D-08).

#### `0x02` — InputState + Motion (C→S), 36 bytes

Os 24 bytes do InputState (com `flags` bit 0 = 1) seguidos de:

| Offset | Tipo | Campo | Unidade |
|---|---|---|---|
| 24 | i16 | `gx` | rad/s × 1000 |
| 26 | i16 | `gy` | rad/s × 1000 |
| 28 | i16 | `gz` | rad/s × 1000 |
| 30 | i16 | `ax` | m/s² × 100 (inclui a gravidade) |
| 32 | i16 | `ay` | m/s² × 100 |
| 34 | i16 | `az` | m/s² × 100 |

- **Referencial do controle:** X para a direita, Y para cima, Z saindo da tela em direção ao jogador, com o aparelho na orientação em que está sendo usado (o client aplica a rotação da tela aos eixos do sensor). Rotação positiva = anti-horária pela regra da mão direita (mesma convenção do Android).
- Valores fora da faixa são **saturados** em −32768/32767.
- O client manda `0x02` (em vez de `0x01`) enquanto `config.gyro_enabled` for `true` e `caps.gyro` for `true`.

#### `0x10` — Ping (C→S), 8 bytes

| Offset | Tipo | Campo |
|---|---|---|
| 0 | i64 | `t_client_us` |

#### `0x11` — Pong (S→C), 20 bytes

| Offset | Tipo | Campo | Regras |
|---|---|---|---|
| 0 | i64 | `t_client_us` | Eco do Ping |
| 8 | i64 | `t_server_rx_us` | Relógio do server na recepção do Ping |
| 16 | u32 | `server_proc_us` | Tempo entre a recepção do Ping e o envio do Pong |

RTT medido pelo client = `agora − t_client_us − server_proc_us`.

#### `0x20` — Rumble (S→C), 4 bytes

| Offset | Tipo | Campo | Regras |
|---|---|---|---|
| 0 | u16 | `rumble_id` | Incrementa a cada **mudança** de estado (não a cada retransmissão). Começa em 1; depois de `0xFFFF` volta a **1** (0 é reservado para "nenhum") |
| 2 | u8 | `large` | Motor grande (baixa frequência) 0–255 |
| 3 | u8 | `small` | Motor pequeno (alta frequência) 0–255 |

### 6.5 Ordem de validação e erros

Quem recebe DEVE validar nesta ordem e descartar o pacote no primeiro erro, em silêncio (sem resposta), contando o erro nas métricas. A ordem é normativa porque os vectors esperam um erro específico.

| # | Checagem | Erro |
|---|---|---|
| 1 | Tamanho ≥ 12 | `too_short` |
| 2 | `magic == 0x4750` | `bad_magic` |
| 3 | `ver == 1` | `bad_version` |
| 4 | `type` conhecido | `unknown_type` |
| 5 | `type` é da direção que este lado recebe | `wrong_direction` |
| 6 | Tamanho == tamanho exato do `type` | `bad_length` |
| 7 | `session_id` é de uma sessão ativa | `unknown_session` |
| 8 | `seq > last_seq` | `stale_seq` |
| 9 | Tag AEAD válida | `bad_tag` |
| 10 | Semântica do payload (bit de motion coerente com o `type`) | `bad_payload` |

A checagem 8 vem antes da 9 para barrar replays sem gastar CPU com AEAD; é segura porque `last_seq` só avança depois da 10.

Os decoders DEVEM entregar `buttons` mascarado com `0xF7FF` e `flags` mascarado com `0x01`.

## 7. Botões (bitmask XInput)

| Bit | Nome XInput | Nome no PadGaming |
|---|---|---|
| `0x0001` | `XINPUT_GAMEPAD_DPAD_UP` | `dpad_up` |
| `0x0002` | `XINPUT_GAMEPAD_DPAD_DOWN` | `dpad_down` |
| `0x0004` | `XINPUT_GAMEPAD_DPAD_LEFT` | `dpad_left` |
| `0x0008` | `XINPUT_GAMEPAD_DPAD_RIGHT` | `dpad_right` |
| `0x0010` | `XINPUT_GAMEPAD_START` | `start` |
| `0x0020` | `XINPUT_GAMEPAD_BACK` | `back` |
| `0x0040` | `XINPUT_GAMEPAD_LEFT_THUMB` | `left_thumb` (L3) |
| `0x0080` | `XINPUT_GAMEPAD_RIGHT_THUMB` | `right_thumb` (R3) |
| `0x0100` | `XINPUT_GAMEPAD_LEFT_SHOULDER` | `left_shoulder` (LB) |
| `0x0200` | `XINPUT_GAMEPAD_RIGHT_SHOULDER` | `right_shoulder` (RB) |
| `0x0400` | Guide (`XUSB_GAMEPAD_GUIDE` no ViGEm; não documentado no XInput) | `guide` |
| `0x0800` | — | **reservado** |
| `0x1000` | `XINPUT_GAMEPAD_A` | `a` |
| `0x2000` | `XINPUT_GAMEPAD_B` | `b` |
| `0x4000` | `XINPUT_GAMEPAD_X` | `x` |
| `0x8000` | `XINPUT_GAMEPAD_Y` | `y` |

Máscara de bits válidos: `0xF7FF`. Os nomes da terceira coluna são usados nos perfis de mapeamento e nos vectors.

## 8. Taxas de envio

| Quem | O quê | Regra |
|---|---|---|
| Client | InputState / Motion | Imediato quando o estado muda, respeitando o intervalo mínimo `1 / send_rate_hz` (padrão 250 Hz = 4 ms); mudanças dentro do intervalo são agrupadas e o estado mais recente sai no fim dele. |
| Client | Keepalive | Reenvia o estado atual a **10 Hz** (100 ms) quando nada muda. |
| Client | Ping `0x10` | A cada **250 ms**. |
| Server | Pong `0x11` | Um para cada Ping válido, imediatamente. |
| Server | Rumble `0x20` | Quando o estado muda (novo `rumble_id`) e retransmite o estado atual a cada **100 ms** até receber `rumble_ack_id` igual ao `rumble_id` atual. |

O client aplica todo Rumble válido que receber (a ordem já é garantida pelo `seq`), inclusive `large = small = 0`, que desliga os motores.

## 9. Timeouts

| Lado | Condição | Ação |
|---|---|---|
| Server | 250 ms sem pacote C→S válido | Controle virtual vai para o **estado neutro** (solta tudo; RF-07). |
| Server | 5 s sem pacote C→S válido | Sessão marcada como **suspensa** na UI (o slot continua reservado). |
| Server | TLS fechado ou 15 s sem mensagem de controle | Sessão encerrada (`udp_key` descartada, `session_id` inválido); slot reservado por **30 s** para o mesmo `device_id`. |
| Client | 2 s sem pacote S→C válido | Para a vibração. |
| Ambos | `hello` não chega em 10 s | Server fecha a conexão. |
| Server | PIN errado 5× | `reject` `locked` por 60 s. |

## 10. Versionamento

- `proto_ver` (no `hello`/`welcome`), `ver` do cabeçalho UDP, `ver` do mDNS/broadcast e `v` da URI andam juntos: hoje todos valem **1**.
- **Mudança compatível (não incrementa):** campos JSON opcionais novos; novos `type` de mensagem de controle; novos valores em `reject.reason`/`bye.reason`; novos parâmetros opcionais na URI e chaves no TXT; novos `type` de pacote UDP (quem não conhece descarta com `unknown_type`); dar significado a um bit reservado **desde que** ignorá-lo mantenha o comportamento correto.
- **Mudança incompatível (incrementa `proto_ver`):** alterar layout, tamanho ou semântica de pacote UDP existente; mudar criptografia, nonce, AAD ou framing; tornar obrigatório um campo que era opcional; remover campo, mensagem ou valor de enum; mudar unidades ou faixas.
- O server pode aceitar várias versões e responde `reject` `version` com `supported_versions` quando não aceita a do client. O client sempre manda a maior versão que conhece.
- Mudanças no protocolo exigem atualizar este documento, o schema, o gerador de vectors e um novo registro em [DECISOES.md](DECISOES.md).

## 11. Test vectors

Ficam em `protocol/test-vectors/` e são **gerados** por `protocol/tools/gen.mjs` (implementação de referência em Node, usando o ChaCha20-Poly1305 do OpenSSL — independente da RustCrypto e do `javax.crypto`). `pnpm --filter @padgaming/protocol check` confere que os arquivos commitados batem com o gerador, validam nos schemas e que o decoder de referência produz o resultado esperado. Não edite os JSON à mão.

| Arquivo | Conteúdo | Schema |
|---|---|---|
| `udp.json` | Pacotes UDP válidos e inválidos | `schemas/udp-vectors.schema.json` |
| `control.json` | Frames do canal de controle | `schemas/control-vectors.schema.json` |
| `pairing-uri.json` | URIs de pareamento | `schemas/pairing-uri-vectors.schema.json` |

**UDP** — cada vector tem o **contexto do receptor** (`role` = `server`\|`client`, `session_id`, `key_hex`, `last_seq` ou `null`), o `packet_hex` e o resultado esperado:

- `"expect": {"result": "ok", "header": {...}, "payload": {...}, "payload_plain_hex": "..."}` — o decoder DEVE produzir exatamente esse cabeçalho/payload, e o encoder, a partir deles (com `key_hex` e a direção do `type`), DEVE produzir exatamente `packet_hex`. Vectors com `"decode_only": true` (bits reservados ligados no fio) só valem para o decoder.
- `"expect": {"result": "error", "error": "<código da §6.5>"}`.

**Controle** — `frame_hex` + `expect` com `result` = `ok` (com o `message` decodificado), `ignore`, `bad_frame` ou `bad_message`.

**URI** — `uri` + `expect` com `result` = `ok` (com os parâmetros decodificados; binários em hex) ou `error` (com o código da §4.2).

Inteiros nos vectors são números JSON menores que 2⁵³ (seguros em qualquer parser); os bytes exatos estão sempre no `*_hex`.

## 12. Notas para a implementação Kotlin

- `ByteBuffer.wrap(buf).order(ByteOrder.LITTLE_ENDIAN)`, reutilizando o buffer (sem alocar por pacote).
- Leitura sem sinal: `u8` → `get().toInt() and 0xFF`; `u16` → `getShort().toInt() and 0xFFFF`; `u32` → `getInt().toLong() and 0xFFFFFFFFL`.
- `seq` e `server_proc_us` cabem em `Long` (u32); `session_id` cabe em `Int` (≤ 2³¹−1, também no JSON); `i64` são sempre ≥ 0 e cabem em `Long` direto.
- A comparação `seq > last_seq` DEVE ser sem sinal (`Long` depois do `and 0xFFFFFFFFL`, ou `Integer.compareUnsigned`).
- `kotlinx.serialization`: `classDiscriminator = "type"` nas mensagens e `@JsonClassDiscriminator("method")` em `auth`; `ignoreUnknownKeys = true`.
- ChaCha20-Poly1305: `Cipher.getInstance("ChaCha20-Poly1305")` com `IvParameterSpec(nonce)` e `updateAAD(header)` (API 28+); Tink/BouncyCastle na API 26–27 (T-902).
