# padgaming-net

Rede do server: loop UDP de input, canal de controle TLS e descoberta mDNS.

- `input`: loop UDP de input (T-301).
- `control`: servidor de controle TLS: handshake, sessões, keepalive e confirmação de novos dispositivos (T-302).
- `identity`, `pairing`, `qr`: certificado, fingerprint, tokens/PIN e QR do pareamento (T-103).
- `discovery`: anúncio mDNS, respondedor de broadcast `PGDISC1` e dispositivos próximos (T-303).
