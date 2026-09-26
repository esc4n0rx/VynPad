# T-306 — Regras de firewall do Windows

| Campo | Valor |
|---|---|
| Fase | F3 — Rede |
| Prioridade | P0 |
| Estimativa | P |
| Depende de | T-301 |
| Status | ⬜ A fazer |

## Objetivo
Evitar a causa nº 1 de "não conecta": o firewall bloqueando UDP/mDNS.

## Escopo
- Detectar se existem regras de entrada para o executável (INetFwPolicy2 via COM, crate `windows`).
- Detectar o perfil da rede atual (Pública/Privada) e avisar quando for "Pública".
- Command `fix_firewall`: cria regras de entrada (TCP 47800, UDP 47801/47802, UDP 5353) restritas ao executável e aos perfis Privado/Domínio, via processo elevado (UAC).
- O instalador também cria as regras (T-703); o onboarding só revalida.
- Autodiagnóstico: "Testar conexão" envia um pacote para si mesmo pela interface LAN.

## Critérios de aceite
- [ ] Numa máquina sem regras, o onboarding corrige com um clique e o simulador em outro PC conecta.
- [ ] O aviso de rede pública aparece com o link para as configurações do Windows.

## Fora de escopo
- Firewall no Linux (documentado na T-802).
