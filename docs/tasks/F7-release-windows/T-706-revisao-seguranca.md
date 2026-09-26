# T-706 — Revisão de segurança

| Campo | Valor |
|---|---|
| Fase | F7 — Qualidade e release Windows |
| Prioridade | P1 |
| Estimativa | M |
| Depende de | T-103, T-302 |
| Status | ⬜ A fazer |

## Objetivo
Um app que injeta input no PC precisa ser seguro por padrão.

## Escopo
- Modelo de ameaças (STRIDE) documentado: atacante na mesma LAN, replay, MITM no pareamento, DoS por flood UDP/TCP, arquivo de perfil malicioso.
- Checklist:
  - [ ] Nenhum input aceito sem sessão autenticada.
  - [ ] Anti-replay verificado; tags inválidas não geram trabalho caro.
  - [ ] Flood UDP de fontes desconhecidas não degrada as sessões legítimas (descarte barato antes da criptografia, via lookup de `session_id`).
  - [ ] Rate limit no TCP (handshakes por IP).
  - [ ] `import_profile` valida o tamanho e o schema (sem desserialização insegura).
  - [ ] Capabilities do Tauri mínimas; CSP sem `unsafe-eval`.
  - [ ] Segredos com permissões de arquivo restritas; `Secret<T>` nos logs.
  - [ ] `cargo audit` / `cargo deny` limpos.
- Correções dos achados ou tasks abertas para cada um.

## Critérios de aceite
- [ ] Documento `docs/SEGURANCA.md` com o modelo de ameaças e o checklist concluído.

## Fora de escopo
- Pentest externo.
