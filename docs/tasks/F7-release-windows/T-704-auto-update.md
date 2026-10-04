# T-704 — Auto-update

| Campo | Valor |
|---|---|
| Fase | F7 — Qualidade e release Windows |
| Prioridade | P1 |
| Estimativa | M |
| Depende de | T-703 |
| Status | 🟨 Em andamento |

## Objetivo
Manter os usuários na versão mais recente sem esforço e sem interromper uma partida.

## Escopo
- `tauri-plugin-updater` com assinatura minisign; o manifesto `latest.json` fica no GitHub Releases.
- Verificação no boot + a cada 6 h; canal `stable` (e `beta` opcional nas Configurações).
- UI: badge em Configurações → Sobre + toast "Atualização disponível" com notas da versão (markdown).
- **Nunca atualizar com controles conectados**: adiar até ficar sem sessões ou até o usuário confirmar.
- Compatibilidade de protocolo: o server avisa quando um client usa um `proto_ver` antigo (mensagem para atualizar o app Android).

## Critérios de aceite
- [ ] Atualizar da versão N para N+1 preserva o banco, os perfis e os pareamentos. O instalador não mexe em `%APPDATA%\VynPad` (os dados só saem se o usuário marcar a opção ao desinstalar), mas a atualização de ponta a ponta ainda não foi exercitada: falta publicar dois releases reais.
- [x] Com um controle conectado, a atualização não é aplicada sem confirmação (`install_policy` em `updates.rs`, com testes, e o diálogo em Configurações → Sobre, com testes).

## Implementação
- `tauri-plugin-updater` com o endpoint `releases/latest/download/latest.json` e a chave pública em `tauri.conf.json`. Instalação `passive`.
- `updates.rs`: procura ao abrir (30 s depois) e a cada 6 h, se `check_updates` estiver ligado; comandos `check_for_update`, `pending_update` e `install_update(confirmed)`; evento `update:available`. Baixa primeiro, confere as sessões de novo, avisa os clients (`core.stop()`) e só então instala, já que no Windows o instalador encerra o processo.
- UI: Configurações → Sobre → Atualizações (procurar, selo da versão nova, novidades em markdown, instalar com confirmação) e a opção "Procurar atualizações sozinho"; toast "Atualização disponível" no evento.
- Versão de protocolo antiga: o server já recusava (`RejectReason::Version`); o aviso agora diz "versão incompatível, atualize o app no celular".
- Release: `scripts/make-latest-json.mjs` monta o manifesto a partir do `.sig`, e o `release.yml` o publica com o instalador. `createUpdaterArtifacts` só liga no release (o build comum não precisa da chave).

## Decisões
- **Nunca instala sozinho.** O escopo falava em adiar até ficar sem sessões; preferi só oferecer, porque um app de bandeja que fecha e reabre sem o usuário pedir surpreende. A UI avisa e o usuário decide.
- **Sem canal `beta` por enquanto** (era opcional): exigiria uma segunda tag de manifesto e uma opção a mais. Fica para quando houver um beta de verdade.
- Chave de assinatura gerada sem senha, fora do repositório (`~/.vynpad/updater.key`); ver `docs/SEGURANCA.md` (G).

## Para publicar o primeiro release
1. Guardar `~/.vynpad/updater.key` em local seguro e cadastrar o segredo `TAURI_SIGNING_PRIVATE_KEY` (o conteúdo do arquivo) no GitHub. Se a chave ganhar senha, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.
2. Se o repositório mudar de nome, atualizar o endpoint em `tauri.conf.json`.
3. Marcar a tag `vX.Y.Z` igual à versão do `tauri.conf.json`.

## Fora de escopo
- Delta updates.
