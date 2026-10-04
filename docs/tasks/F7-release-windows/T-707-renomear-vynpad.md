# T-707 — Renomear o projeto para VynPad

| Campo | Valor |
|---|---|
| Fase | F7 — Qualidade e release Windows |
| Prioridade | P0 |
| Estimativa | M |
| Depende de | — |
| Status | 🟨 Em andamento |

## Objetivo
O nome PadGaming já está registrado, então o produto passa a se chamar **VynPad**. Nada do nome antigo pode sobrar no código, na documentação, nos identificadores e no que vai no fio. O projeto ainda não foi lançado, então não há migração de dados nem compatibilidade com o nome antigo.

## Mapa do nome
| Antes | Depois | Onde |
|---|---|---|
| `PadGaming` | `VynPad` | texto, UI, docs, nome do produto, pasta de dados |
| `padgaming` | `vynpad` | crates (`vynpad-*`), módulos Rust, pacotes `@vynpad/*`, `vynpad.db`, esquema de URL `vynpad://`, serviço mDNS `_vynpad._tcp`, arquivos de log, `com.vynpad.server` |
| `PADGAMING` | `VYNPAD` | variáveis de ambiente (`VYNPAD_STRESS_SECS` e `VYNPAD_STRESS_SEED`) |
| magic `PG` (`50 47`, `0x4750`) | `VP` (`56 50`, `0x5056`) | cabeçalho do pacote UDP |
| `PGDISC1` | `VPDISC1` | pedido de descoberta por broadcast |
| `padgaming.local` | `vynpad.local` | `$id` dos JSON Schemas |

Regras de firewall passam a se chamar `VynPad *`, e o instalador remove essas, não as antigas.

## Feito
- Pastas das crates com `git mv` (histórico preservado) e substituição em todos os arquivos de texto rastreados.
- Test vectors regenerados pelo gerador de referência (`pnpm --filter @vynpad/protocol gen`), porque o magic faz parte do AAD e muda todos os pacotes cifrados.
- `Cargo.lock` e `pnpm-lock.yaml` atualizados; `cargo fmt` reordenou os `use`, já que a ordem alfabética mudou.
- Validado como o CI: fmt, clippy `-D warnings`, testes do workspace e do desktop, `cargo deny`, prettier, eslint, tsc, vitest e `check` dos vectors.

## Pendências
- **Ícones e arte**: os PNG/ICO/ICNS não foram alterados. Se tiverem o nome ou o logotipo antigo, precisam ser refeitos.
- **Repositório**: os links nos docs já apontam para `github.com/esc4n0rx/VynPad`, mas o `origin` ainda é o repositório antigo. A migração para o repo novo fica para depois.
- **Instalação antiga**: quem tem o app instalado como PadGaming mantém essa instalação (outra chave de desinstalação, outra pasta de dados); desinstalar à mão.
- Recomendar um domínio e conta de assinatura já com o nome VynPad (T-703).

## Critérios de aceite
- [x] `git grep -i padgaming` só acha as menções históricas (D-25 e esta task).
- [x] CI local verde (comandos do `ci.yml`).
- [ ] Ícones e identidade visual com o novo nome.
- [ ] Repositório novo criado e histórico migrado.
