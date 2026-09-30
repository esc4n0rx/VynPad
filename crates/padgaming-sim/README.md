# padgaming-sim

Client simulador em CLI: testa o server sem o app Android.

```text
# 1) server de desenvolvimento (debug; sem autenticação!)
cargo run -p padgaming-net --example dev_server            # controles virtuais reais (ViGEmBus)
cargo run -p padgaming-net --example dev_server -- --mock  # só imprime

# 2) simulador
cargo run -p padgaming-sim -- connect --insecure-dev --pattern circle
cargo run -p padgaming-sim -- connect --insecure-dev --script exemplos/mira.toml
cargo run -p padgaming-sim -- connect --insecure-dev --pattern stress --loss 10% --delay 20ms --jitter 5ms --reorder
cargo run -p padgaming-sim -- bench --insecure-dev --clients 4 --rate 250 --duration 60s --json bench.json
cargo run -p padgaming-sim --features gamepad -- connect --insecure-dev --gamepad   # controle físico local

# descoberta (com `cargo run -p padgaming-net --example discovery_server` em outro terminal)
cargo run -p padgaming-sim -- discover
cargo run -p padgaming-sim -- announce --name "Pixel do Paulo" --duration 60s

# 3) conferir o controle virtual sem abrir o joy.cpl
cargo run -p padgaming-driver --example xinput_watch
```

- `discover` (mDNS + broadcast) e `announce` (client simulado em "dispositivos próximos") já existem;
  `pair` ainda não está exposto na CLI, então a autenticação por linha de comando é só a sessão de
  desenvolvimento (`--insecure-dev`). O canal TLS está em `padgaming_sim::control`.
- `connect --interactive` (teclado → botões) ainda não existe.
- As falhas simuladas valem para o sentido client → server. Reordenar (ou jitter alto) faz o
  server descartar pacotes como `replay`, que é o comportamento esperado do anti-replay.
- Script TOML: veja a documentação de `padgaming_sim::source::Script`.
