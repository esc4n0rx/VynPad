# Segurança do VynPad

Revisão de segurança da T-706. O VynPad faz uma coisa delicada: aceita input de outro aparelho da rede e o injeta no PC como se fosse um controle. Por isso o servidor parte do princípio de que **ninguém é confiável até pareado**, e de que **tudo o que vem da rede ou de um arquivo é hostil até ser validado**.

Este documento diz o que protegemos, de quem, como, **onde cada defesa é verificada** e o que ainda fica de risco. Dá para refazer a revisão rodando os testes citados.

## 1. O que protegemos

| Bem | Por que importa |
|---|---|
| **Injeção de input no PC** | Quem controla um slot controla teclas e cliques indiretos (jogos, atalhos de overlay). |
| **Token do dispositivo** | É a credencial de reconexão de um celular pareado. |
| **Chave privada do server** (`server.key`) | Quem a tem finge ser este PC. |
| **Perfis de mapeamento** | Um perfil malicioso importado pode tentar quebrar o app ou configurar algo inesperado. |
| **Banco local** (SQLite) | Pareamentos, perfis, histórico. |

## 2. Quem ataca (modelo de ameaças, STRIDE)

Os atacantes considerados: **A)** alguém na mesma rede local (Wi-Fi de casa, de um café, de um evento), **B)** um aparelho já pareado que se comporta mal, **C)** um arquivo `.padprofile` vindo de outra pessoa, **D)** outro programa rodando no mesmo PC do usuário (fora do escopo, ver §5).

| STRIDE | Ameaça | Defesa | Verificação |
|---|---|---|---|
| **S**poofing | Um aparelho qualquer da rede finge ser um celular pareado e manda input. | Todo pacote UDP é autenticado (ChaCha20-Poly1305) com a chave da **sessão**, entregue só pelo canal TLS depois de o dispositivo provar quem é (token). Sem sessão, o pacote é descartado. | `crates/vynpad-net/tests/input_udp.rs` (`a_flood_of_garbage_does_not_starve_the_legitimate_session` conta os descartes por tag inválida e por sessão desconhecida); `protocol/test-vectors` (validação na ordem normativa do PROTOCOLO §6.5). |
| Spoofing | Um PC falso finge ser o server para roubar o token do celular. | O QR/URI de pareamento carrega o **fingerprint do certificado** do server; o celular faz *pinning* e recusa outro certificado. O token só viaja dentro do TLS 1.3 já pinado. | `crates/vynpad-sim/tests/control_handshake.rs` (`a_pinned_client_refuses_another_certificate`); `crates/vynpad-server/tests/core.rs` (`regenerating_the_identity_forgets_every_device_and_ends_their_sessions`: identidade nova invalida os pareamentos antigos). |
| **T**ampering | Alterar um pacote UDP em trânsito. | A tag AEAD cobre o cabeçalho (como AAD) e o conteúdo: qualquer alteração derruba o pacote. | Test vectors do protocolo (casos de tag inválida e de cabeçalho adulterado). |
| Tampering | Importar um `.padprofile` que corrompa o app. | O texto é limitado a 256 KB, o JSON tem profundidade limitada pelo parser, o formato e a versão são conferidos, e **todos** os valores são validados antes de gravar. Nada é executado: o perfil é só dado. | `crates/vynpad-storage/tests/repos.rs` (`hostile_padprofile_files_are_refused_without_storing_anything`: arquivo gigante, aninhado, formato errado, versão futura, valores fora da faixa, nome vazio ou enorme, texto que não é JSON). |
| **R**epudiation | Não saber quem conectou. | Cada conexão, desconexão, recusa e pareamento vai para o histórico (`connection_log`) com o dispositivo e o horário. | `crates/vynpad-server/tests/core.rs` (`connections_disconnections_and_rejections_go_to_the_history`). |
| **I**nformation disclosure | Ler o tráfego de controle ou de input. | O canal de controle é TLS 1.3; o input UDP é cifrado. A descoberta (mDNS e broadcast) revela só o que é público de propósito: nome do server, portas e os 8 primeiros bytes do fingerprint. | PROTOCOLO §3 e §4. |
| Information disclosure | Vazar token ou chave em logs. | Segredos usam `Secret<T>` (`Debug` e `Display` imprimem `Secret(***)`). O diagnóstico exportado não inclui token nem chave (ver T-705). | `crates/vynpad-protocol/src/secret.rs` (testes do tipo). |
| **D**enial of service | Inundar a porta UDP de input com lixo para atrapalhar um jogo em andamento. | O descarte é barato e **antes da criptografia cara**: cabeçalho inválido, sessão desconhecida e `seq` repetido saem sem tocar no AEAD (PROTOCOLO §6.5). Só um pacote com sessão válida e `seq` novo paga a verificação da tag. | `a_flood_of_garbage_does_not_starve_the_legitimate_session`: ~37 mil pacotes de lixo por segundo (sessão desconhecida, sessão conhecida com tag errada e datagramas malformados) e a sessão legítima recebe 250 de 250 inputs, com p99 de processamento de ~0,1 ms. |
| Denial of service | Encher as conexões TCP pendentes para ninguém conseguir parear. | Limite global (16) **e por endereço** (4) de conexões ainda sem `hello` autenticado, prazo de 10 s para o TLS e o `hello`, e bloqueio de 60 s depois de 5 PINs errados. | `crates/vynpad-sim/tests/control_handshake.rs` (`pending_connections_are_limited`, `one_address_cannot_take_every_pending_slot`; o bloqueio por PIN e o uso único do PIN e do token têm testes em `crates/vynpad-net/src/pairing.rs`). |
| Denial of service | Um dispositivo pareado manda lixo ou fica mudo. | O watchdog solta o controle em 250 ms sem pacotes, a sessão cai em 15 s sem nada no canal de controle e a reserva do slot vence. | `crates/vynpad-sim/tests/control_handshake.rs` e `crates/vynpad-server/tests/core.rs` (ciclo de vida da sessão). |
| **E**levation of privilege | Um pareamento novo entrar sem o dono saber. | Com "Pedir aprovação" ligado, o primeiro pareamento espera a resposta do usuário (30 s), e o PIN/QR vale por poucos minutos e só um uso. | `crates/vynpad-server/tests/core.rs` (`new_devices_wait_for_the_users_answer_when_confirmation_is_on`, `an_unanswered_confirmation_expires_and_closes_the_popup`). |
| Elevation of privilege | A UI (webview) ganhar poder demais. | Permissões do Tauri mínimas (`capabilities/default.json`, só o que a janela usa: arrastar, fechar, minimizar, maximizar, estado da janela e eventos). CSP de produção sem `unsafe-eval` e sem scripts inline (`script-src 'self'`). A webview não acessa arquivos nem o shell; tudo passa por *commands* tipados que validam a entrada. | `apps/desktop/src-tauri/tauri.conf.json` e `capabilities/default.json`; os testes de commands em `apps/desktop/src-tauri/src/lib.rs` conferem que índices e valores inválidos são recusados. |

## 3. Checklist da T-706

- [x] **Nenhum input aceito sem sessão autenticada.** O motor só entrega ao driver pacotes que passaram por sessão conhecida, `seq` novo e tag válida. A sessão fixa de desenvolvimento (`--insecure-dev`, sem autenticação) só existe em builds de debug (`#[cfg(debug_assertions)]`) e nunca é registrada pelo app.
- [x] **Anti-replay verificado.** Janela de `seq` por sessão, checada **antes** do AEAD; `seq` de 32 bits sem wrap-around (D-20), então o nonce nunca repete com a mesma chave.
- [x] **Tags inválidas não geram trabalho caro** e **flood UDP não degrada as sessões legítimas.** Ver a linha de DoS acima e o teste de flood. Limite honesto: o buffer de recepção do socket é compartilhado, e uma inundação muito acima do que a máquina processa pode encher o buffer do sistema e derrubar também pacotes legítimos. A mitigação é o descarte barato e o buffer grande (`recv_buffer_bytes`); a regra de firewall do app só vale em redes Privada e Domínio, então uma rede pública (onde o atacante é mais provável) já nem chega à porta.
- [x] **Rate limit no TCP.** Pendentes por endereço e no total, prazos curtos e bloqueio por PIN errado (corrigido nesta revisão: antes só havia o limite total, que um único aparelho podia esgotar).
- [x] **`import_profile` valida tamanho e schema.** Ver a linha de Tampering acima.
- [x] **Capabilities do Tauri mínimas; CSP sem `unsafe-eval`.** O `opener:default` foi removido das capabilities (a UI não abre links por JavaScript; o app abre o que precisa pelo Rust).
- [x] **Segredos com permissões restritas e `Secret<T>` nos logs.** A chave do server é criada com permissão `0600` em Unix. No Windows ela fica em `%APPDATA%\VynPad`, pasta que o Windows já restringe ao próprio usuário (herdada do perfil); o app não aperta mais que isso. Ver o risco D abaixo.
- [x] **`cargo deny` limpo** (advisories, licenças, bans e fontes). `cargo audit` usa o mesmo banco de avisos (RustSec) que o `cargo deny check advisories`, que roda no CI.

## 4. Achados e o que foi feito

| # | Achado | Gravidade | Ação |
|---|---|---|---|
| 1 | Um único endereço da LAN podia ocupar todas as 16 vagas de conexão pendente e impedir novos pareamentos. | Média | **Corrigido:** limite por endereço (`max_pending_per_ip`, padrão 4) e teste. |
| 2 | A capability `opener:default` dava à webview o poder de abrir URLs e caminhos, sem a UI usar. | Baixa | **Corrigido:** removida. |
| 3 | Faltava teste dos arquivos de perfil hostis e do flood de UDP. | Baixa | **Corrigido:** os dois testes acima. |
| 4 | A chave privada no Windows depende da ACL padrão do perfil do usuário. | Baixa | Aceito e documentado (risco D). Se o perfil do usuário for compartilhado, qualquer processo do mesmo usuário já teria acesso ao app inteiro de qualquer forma. |

## 5. Riscos que ficam (e por quê)

- **A) Atacante com a rede inteira sob controle** (roteador malicioso) pode **derrubar** a conexão (DoS), mas não injetar input nem ler o tráfego: sem a chave da sessão, nada passa na autenticação.
- **B) Celular pareado malicioso** pode mandar input arbitrário **no próprio slot**; é o que um controle faz. Revogar o dispositivo (Dispositivos → Revogar) encerra a sessão na hora e invalida o token.
- **C) PIN de 6 dígitos:** vale 2 minutos, 5 tentativas erradas por endereço travam 60 s, e o PIN só é aceito dentro do TLS pinado. Um atacante na LAN precisaria adivinhar 1 em 1.000.000 com ~5 tentativas por minuto, dentro da janela em que o usuário mostrou o PIN. É o mesmo nível de risco do pareamento por PIN do Bluetooth, e o QR (token de 128 bits) não tem esse limite.
- **D) Outro programa do mesmo usuário no mesmo PC** pode ler `%APPDATA%\VynPad`. Não é um modelo de ameaça deste app: esse programa já poderia injetar input diretamente no Windows.
- **E) O ViGEmBus** é um driver de terceiros (arquivado, ver D-03) com privilégios de kernel. O app o isola atrás da trait `VirtualPadDriver` e só entrega a ele relatórios de controle já validados e mapeados.
- **F) Pentest externo** está fora de escopo desta task.

## 6. Como refazer a revisão

```text
cargo test -p vynpad-net --test input_udp          # flood, replay, tags, sessões
cargo test -p vynpad-sim --test control_handshake  # handshake, pinning, limites TCP, PIN
cargo test -p vynpad-storage --test repos          # importação de perfis hostis
cargo test -p vynpad-server --test core            # aprovação, revogação, identidade
cargo deny check                                      # advisories, licenças, bans, fontes
```
