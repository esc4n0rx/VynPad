# T-705 — Docs do usuário e pacote de diagnóstico

| Campo | Valor |
|---|---|
| Fase | F7 — Qualidade e release Windows |
| Prioridade | P1 |
| Estimativa | M |
| Depende de | T-004 |
| Status | ⬜ A fazer |

## Objetivo
Reduzir o suporte: o usuário resolve sozinho, ou envia tudo o que precisamos em um arquivo.

## Escopo
- `docs/usuario/` (PT-BR + EN): primeiros passos, parear, mapeamento, gyro, dicas de latência, **solução de problemas** (não encontra o PC, firewall, rede pública, ViGEmBus, jogo não reconhece o controle, Steam Input duplicando controles, input atrasado).
- Botão **"Exportar diagnóstico"**: `.zip` com os logs dos últimos 3 dias, versão do app e do SO, status do driver, interfaces de rede (IPs mascarados opcionalmente), métricas recentes e configurações (sem tokens/segredos).
- Link "Ajuda" na UI abrindo a página correspondente.

## Critérios de aceite
- [ ] O zip de diagnóstico não contém nenhum segredo (teste automatizado procurando padrões de token).
- [ ] Cada erro conhecido da UI tem link para a seção de troubleshooting.

## Fora de escopo
- Site do produto.
