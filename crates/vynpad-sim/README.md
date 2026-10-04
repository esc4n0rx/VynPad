# vynpad-sim

Client simulador em CLI: testa o server sem o app Android.

```text
# 1) server de desenvolvimento (debug; sem autenticação!)
cargo run -p vynpad-net --example dev_server            # controles virtuais reais (ViGEmBus)
cargo run -p vynpad-net --example dev_server -- --mock  # só imprime

# 2) simulador
cargo run -p vynpad-sim -- connect --insecure-dev --pattern circle
cargo run -p vynpad-sim -- connect --insecure-dev --script exemplos/mira.toml
cargo run -p vynpad-sim -- connect --insecure-dev --pattern stress --loss 10% --delay 20ms --jitter 5ms --reorder
# pareamento de verdade (o PIN aparece em Dispositivos > Parear novo); --hold mantém o controle conectado
cargo run -p vynpad-sim -- pair --pin 123456 --host 127.0.0.1 --name "Meu celular" --hold 60s
cargo run -p vynpad-sim -- pair --uri "vynpad://pair?..." --save pareado.json

cargo run -p vynpad-sim -- bench --insecure-dev --clients 4 --rate 250 --duration 60s --json bench.json
cargo run -p vynpad-sim --features gamepad -- connect --insecure-dev --gamepad   # controle físico local

# descoberta (com `cargo run -p vynpad-net --example discovery_server` em outro terminal)
cargo run -p vynpad-sim -- discover
cargo run -p vynpad-sim -- announce --name "Pixel do Paulo" --duration 60s

# 3) conferir o controle virtual sem abrir o joy.cpl
cargo run -p vynpad-driver --example xinput_watch
```

- `discover` (mDNS + broadcast) e `announce` (client simulado em "dispositivos próximos") já existem;
  `pair` ainda não está exposto na CLI, então a autenticação por linha de comando é só a sessão de
  desenvolvimento (`--insecure-dev`). O canal TLS está em `vynpad_sim::control`.
- `connect --interactive` (teclado → botões) ainda não existe.
- As falhas simuladas valem para o sentido client → server. Reordenar (ou jitter alto) faz o
  server descartar pacotes como `replay`, que é o comportamento esperado do anti-replay.
- Script TOML: veja a documentação de `vynpad_sim::source::Script`.
