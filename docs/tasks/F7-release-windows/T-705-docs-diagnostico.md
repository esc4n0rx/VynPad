# T-705 — Docs do usuário e pacote de diagnóstico

| Campo | Valor |
|---|---|
| Fase | F7 — Qualidade e release Windows |
| Prioridade | P1 |
| Estimativa | M |
| Depende de | T-004 |
| Status | 🟨 Pronta; falta abrir o diagnóstico exportado e ler a ajuda no app, no Windows |

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

## Fechamento
- **Documentação do usuário** em `docs/usuario/pt-BR` e `docs/usuario/en`: primeiros passos, parear um celular, mapeamento e perfis, gyro, latência e **solução de problemas** (não encontra o PC, firewall, rede pública, ViGEmBus, jogo que não reconhece o controle, Steam Input duplicando, input atrasado e uma seção para cada erro conhecido da UI). Os dois idiomas têm os mesmos ids de título (`## Título {#id}`), conferidos por teste.
- **Ajuda dentro do app** (`features/help`): a tela `/help` renderiza esses mesmos arquivos (empacotados com `?raw`, sem rede) no idioma do app, com um renderizador de markdown mínimo e seguro (`lib/markdown.ts`: sem HTML, links só `#id` e https). Abre por **F1**, por Configurações → Sobre → "Abrir a ajuda" e pelo botão **"Ver ajuda"** que todo toast de erro conhecido ganha (`notifyError` leva ao título do erro, `ERROR_HELP`). O tipo `Record<ErrorCode, string>` faz o compilador exigir uma seção para cada código novo, e um teste confere que ela existe nos dois idiomas.
- **Exportar diagnóstico** (`vynpad_server::diagnostics` + command `export_diagnostics`): `.zip` na pasta Downloads (ou Área de Trabalho), mostrado no Explorer, com `info.txt` (versões, SO, driver, portas, impressão digital pública, firewall), `settings.json`, `network.txt`, `metrics.json`, `devices.json` (sem token nem hash) e os logs dos últimos 3 dias. Opção de mascarar os IPs (`192.168.x.x`).
- **Sem segredos, testado**: `redact` tira o valor de campos como `token`, `device_token`, `udp_key`, `pin`, `authorization`, e qualquer palavra que pareça token (base64 longo) ou hash (hex longo), preservando UUIDs e o fingerprint curto. `no_secret_survives_in_the_zip` monta um pacote com logs cheios de tokens, chaves e PINs falsos e procura cada um em todos os arquivos do `.zip`.
- Não precisou do plugin de diálogo do Tauri: o arquivo vai direto para uma pasta conhecida e o app o mostra.
