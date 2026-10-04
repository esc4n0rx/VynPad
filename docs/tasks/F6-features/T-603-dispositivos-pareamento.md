# T-603 — Dispositivos: próximos, pareados, parear

| Campo | Valor |
|---|---|
| Fase | F6 — Features |
| Prioridade | P0 |
| Estimativa | G |
| Depende de | T-103, T-303, T-403 |
| Status | 🟨 Pronta; falta conferir no app com o simulador (3 s para aparecer, QR colado e PIN, revogar derrubando a sessão) |

## Objetivo
Gerenciar os celulares, com auto detect na rede e pareamento sem fricção.

## Escopo
- **Próximos na rede** (auto detect, T-303): cards com ícone de celular + gamepad, nome, modelo, IP, sinal "visto agora", estado (novo / pareado / conectado). Um card "novo" tem o botão "Parear" (abre o modal com o QR, destacando esse dispositivo).
- **Pareados**: lista com ícone de status, nome (editável inline), modelo, último acesso, slot preferido, perfil padrão, menu (Renomear, Definir slot, Definir perfil, Revogar, Esquecer).
- **Modal "Parear novo dispositivo"**: abas QR code | PIN; o QR/PIN se renova ao expirar, com contador; fingerprint curto visível para conferência; fecha sozinho com animação de sucesso quando o pareamento conclui.
- **Aprovação** (se `require_approval`): diálogo "Galaxy S24 quer se conectar" com Permitir/Negar e timeout.
- Histórico de conexões (últimos 50 eventos do `connection_log`) em um painel recolhível.
- Estado vazio com ilustração e instruções.

## Critérios de aceite
- [ ] O simulador anunciando aparece em "Próximos" em até 3 s.
- [ ] Parear pelo QR (colando a URI no simulador) e pelo PIN funciona.
- [ ] Revogar derruba a sessão ativa na hora.

## Fora de escopo
- Lado Android (T-903/T-904).

## Fechamento
- `features/devices`: `DevicesPage` (próximos, pareados, histórico, estado vazio), `NearbySection`, `PairedSection` (nome editável, menu com slot e perfil padrão, revogar e esquecer com confirmação), `PairDialog` (abas QR/PIN, renova ao vencer, fecha com sucesso ao receber `devices:paired`, invalida o código ao fechar) e `HistoryPanel` (50 eventos, só consulta ao abrir).
- `ApprovalDialog` fica na raiz (`RootLayout`): o pedido aparece em qualquer tela, com contagem regressiva (`timeout_s`, novo no `ApprovalPromptDto`); fechar sem responder nega.
- Componentes novos de uso geral: `ActionMenu` (menu com submenus) e `ConfirmDialog`.
- Os eventos `devices:paired` alimentam `useLastPaired` (`lib/api/server.ts`), que o modal observa.
