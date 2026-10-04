# T-205 — Detecção/instalação do ViGEmBus e slots XInput

| Campo | Valor |
|---|---|
| Fase | F2 — Emulação Windows |
| Prioridade | P0 |
| Estimativa | M |
| Depende de | T-202 |
| Status | 🟨 Backend pronto; falta validar o fluxo numa VM limpa e a tela (T-601) |

## Objetivo
O usuário nunca deve ficar travado por falta do driver nem confuso com controles físicos ocupando slots.

## Escopo
- **Detecção**: tentar `Client::connect`; complementar com consulta ao serviço `ViGEmBus` (SCM) e à versão do driver (registro/`SetupDi`).
- Status exposto: `Ready`, `NotInstalled`, `Outdated(versão)`, `ServiceStopped`, `Error`.
- **Instalação guiada**: o setup oficial do ViGEmBus vai em `resources/` do bundle; a UI chama o command `install_vigem`, que executa o setup com elevação (UAC) e acompanha a conclusão; em seguida, revalidar.
- Checar a licença de redistribuição do ViGEmBus e incluir o aviso em "Sobre".
- **Slots XInput ocupados**: consultar `XInputGetState` (0..3) periodicamente (1 Hz, só com a UI aberta) para identificar controles físicos, e mostrar "P2 ocupado por um controle físico".
- Command `driver_status` + evento `driver:changed`.

## Critérios de aceite
- [ ] Numa VM limpa, o onboarding instala o driver e o status vira `Ready` sem reiniciar o app.
- [ ] Parar o serviço com o app aberto → a UI mostra o erro em até 2 s.
- [ ] Um controle físico conectado aparece como ocupando um slot.

## Fora de escopo
- A tela de onboarding (T-601).

## Situação

Pronto e testado (com o ViGEmBus 1.22.0 real):
- Status `Ready`/`NotInstalled`/`Outdated{found,required}`/`ServiceStopped`/`Error` (`vynpad_driver::DriverStatus`, `setup::classify_bus_error`). O serviço é consultado no SCM e a versão no registro.
- `VigemDriver::status()` detecta a queda do serviço com o app aberto e volta a `Ready` sozinho quando o serviço sobe (reconecta ao barramento).
- Índice XInput real: medido no `plug` por diferença de ocupação (`VigemDriver::user_index`), corrigindo o `get_user_index` do driver, que devolvia o índice errado.
- Sem o driver (`NotInstalled` ou `Outdated`), o status traz `download_url` (https://vigembusdriver.com/download/). O command `open_vigem_download` abre a página no navegador quando o usuário clica no aviso; o app não abre o navegador sozinho. Decisão: **não embutimos o instalador** no bundle; o usuário baixa e instala pelo site oficial.
- Checker de linha de comando: `cargo run -p vynpad-driver --example check_driver` (`-- --simulate missing|stopped|outdated`, `-- --open`).
- Ocupação do XInput: `xinput::XInputProbe` + `vynpad_core::classify_xinput` (livre / virtual nosso / físico).
- Tauri: commands `driver_status`, `open_vigem_download`, `xinput_slots` e evento `driver:changed` (thread que confere a cada 1 s).
- Licença: BSD 3-Clause conferida no driver instalado; aviso em `docs/THIRD-PARTY-NOTICES.md`.

Pendente:
- Validação numa VM limpa (instalar pelo site e ver o status virar `Ready` sem reiniciar o app) e o teste manual de parar o serviço (precisa de administrador).
- Ponte tipada no frontend (T-506) e a tela (T-601/T-603).
