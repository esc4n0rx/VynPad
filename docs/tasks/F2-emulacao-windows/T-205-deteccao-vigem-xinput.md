# T-205 — Detecção/instalação do ViGEmBus e slots XInput

| Campo | Valor |
|---|---|
| Fase | F2 — Emulação Windows |
| Prioridade | P0 |
| Estimativa | M |
| Depende de | T-202 |
| Status | ⬜ A fazer |

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
