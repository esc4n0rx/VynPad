# T-306 — Regras de firewall do Windows

| Campo | Valor |
|---|---|
| Fase | F3 — Rede |
| Prioridade | P0 |
| Estimativa | P |
| Depende de | T-301 |
| Status | 🟨 Backend pronto; falta validar o `fix` (UAC) com o simulador em outro PC e a UI (T-601) |

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

## Situação

Pronto, em `vynpad_net::firewall`:
- `check(exe, ports)`: lê as regras de entrada pelo COM (`INetFwPolicy2`, sem admin, numa thread própria) e diz, porta a porta (TCP 47800, UDP 47801, UDP 47802, UDP 5353), se há regra habilitada de entrada/permitir para este executável cobrindo **todos os perfis ativos**. Uma regra só para "Privado" não vale numa rede "Pública". Firewall desligado nos perfis ativos = nada bloqueado. Aceita regras por programa ou só por porta, listas e intervalos (`47000-48000`), `*` e protocolo "qualquer".
- Aviso de rede pública: `FirewallStatus::public_network_warning()` + `NETWORK_SETTINGS_URI` (`ms-settings:network-status`) para o link.
- `fix(exe, ports)`: um único pedido de UAC (`ShellExecuteExW` com `runas`) que roda `cmd /c` com os `netsh advfirewall`: apaga as regras antigas pelo nome e cria uma regra de entrada por porta, restrita ao executável e aos perfis Privado e Domínio (nunca Público). Recusar o UAC vira `FirewallError::Cancelled`. Caminho com `"`, `%` ou quebra de linha é recusado (o `cmd` expandiria). Devolve o status depois da correção.
- `self_test(lan_ip, ports)`: sonda o canal de controle (TCP) e o `VPDISC1` pelo IP da LAN. Limite: tráfego para o próprio IP não passa pelo filtro de entrada, então isto prova que os serviços estão de pé e ligados à LAN, não que o firewall deixaria um celular entrar. Quem prova isso é o simulador em outro PC.
- Em outros sistemas `check` devolve "não se aplica" e `fix` devolve `Unsupported` (Linux: T-802).
- Exemplo: `cargo run -p vynpad-net --example firewall_check [-- --exe caminho] [--fix]`.

Testes: 14 unitários da avaliação e do comando (rodam em qualquer SO) e um que lê as regras reais no Windows. Na máquina de desenvolvimento a leitura funcionou e mostrou Wi-Fi em rede **Pública** e nenhuma regra do VynPad.

Pendências:
- `--fix` foi rodado na máquina de desenvolvimento (UAC aceito, código 0): as 4 regras foram criadas (entrada, permitir, habilitadas, perfis Domínio e Particular, restritas ao executável do exemplo; conferido com `netsh advfirewall firewall show rule`). O `check` seguiu reportando bloqueado porque a rede do Wi-Fi é Pública, como previsto. **Falta** conferir que o simulador em outro PC conecta (com a rede em Privada). Como as regras são só para Privado/Domínio, numa rede Pública elas não valem até o usuário trocar o perfil: o `check` continua reportando bloqueado e o aviso aparece, como pedido.
- O command `fix_firewall` e o onboarding (T-506/T-601) só precisam chamar `check`/`fix`. O instalador (T-703) deve criar as mesmas regras (`RULE_PREFIX`).
- A compilação para Linux só é conferida no CI (o alvo não está instalado localmente).
