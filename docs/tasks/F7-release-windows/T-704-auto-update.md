# T-704 — Auto-update

| Campo | Valor |
|---|---|
| Fase | F7 — Qualidade e release Windows |
| Prioridade | P1 |
| Estimativa | M |
| Depende de | T-703 |
| Status | ⬜ A fazer |

## Objetivo
Manter os usuários na versão mais recente sem esforço e sem interromper uma partida.

## Escopo
- `tauri-plugin-updater` com assinatura minisign; o manifesto `latest.json` fica no GitHub Releases.
- Verificação no boot + a cada 6 h; canal `stable` (e `beta` opcional nas Configurações).
- UI: badge em Configurações → Sobre + toast "Atualização disponível" com notas da versão (markdown).
- **Nunca atualizar com controles conectados**: adiar até ficar sem sessões ou até o usuário confirmar.
- Compatibilidade de protocolo: o server avisa quando um client usa um `proto_ver` antigo (mensagem para atualizar o app Android).

## Critérios de aceite
- [ ] Atualizar da versão N para N+1 preserva o banco, os perfis e os pareamentos.
- [ ] Com um controle conectado, a atualização não é aplicada sem confirmação.

## Fora de escopo
- Delta updates.
