# T-603 — Dispositivos: próximos, pareados, parear

| Campo | Valor |
|---|---|
| Fase | F6 — Features |
| Prioridade | P0 |
| Estimativa | G |
| Depende de | T-103, T-303, T-403 |
| Status | ⬜ A fazer |

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
