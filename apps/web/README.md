# VynPad: site oficial

Next.js 16 (App Router) + Tailwind 4 + Motion. Segue o design system do produto
(`docs/00-visao-geral/DESIGN-SYSTEM.md`): mesmos tokens, escuro por padrão, claro opcional, glass
e molas. Português (padrão) e inglês, tudo em `src/i18n`.

```bash
pnpm install
pnpm --filter @vynpad/web dev      # http://localhost:3000
pnpm --filter @vynpad/web build    # build de produção
pnpm --filter @vynpad/web test     # vitest
```

## Como os downloads funcionam

O site mostra a versão mais recente do GitHub e leva cada botão a uma rota própria,
`/api/download/<plataforma>`, que **redireciona** para o arquivo. O visitante nunca fala com a API
do GitHub e nenhum segredo vai ao navegador.

| Plataforma       | Origem                                                   |
| ---------------- | -------------------------------------------------------- |
| `windows`        | asset `*setup*.exe` do release mais recente              |
| `windows-msi`    | asset `*.msi`                                            |
| `linux-appimage` | asset `*.AppImage` (quando houver)                       |
| `linux-deb`      | asset `*.deb` (quando houver)                            |
| `android`        | Google Play, **só** se `PLAY_STORE_URL` estiver definido |

Regras de segurança (cobertas por testes em `src/lib/github.test.ts` e `route.test.ts`):

- A plataforma vem de uma **lista fixa**; o nome do arquivo e o id do asset vêm do release que o
  **servidor** leu, nunca do pedido.
- **Repositório privado** (o caso hoje): com `GITHUB_TOKEN`, o servidor pede ao GitHub o endereço
  **assinado e temporário** do asset (`redirect: "manual"`) e manda o visitante para ele. O token
  fica no servidor e o arquivo não passa pelo site. O destino só é aceito se for `github.com` ou
  `*.githubusercontent.com`.
- **Repositório público**: sem token, usa o link público do asset.
- A resposta do GitHub é lida de forma defensiva (campos validados, links só `https://github.com`,
  rascunhos ignorados) e guardada por 5 minutos (`revalidate`), para não estourar o limite da API
  (60/h sem token). Falha de rede ou limite vira um aviso na página, nunca uma página quebrada.
- O SHA-256 mostrado vem do campo `digest` que o GitHub calcula para cada asset.
- As notas de versão (`/changelog`) passam por um renderizador de Markdown próprio: HTML cru nunca é
  interpretado e só links `http(s)` viram link.
- Cabeçalhos de segurança (CSP restrito à própria origem, HSTS, `frame-ancestors 'none'`...) em
  `next.config.ts`. Sem scripts, fontes ou imagens de terceiros.

## Variáveis de ambiente

| Variável         | Obrigatória         | O quê                                                                                                                                         |
| ---------------- | ------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| `GITHUB_TOKEN`   | se o repo é privado | Token **fine-grained**, só leitura (`Contents: Read-only`) **neste repositório**. Nunca `NEXT_PUBLIC_`.                                       |
| `GITHUB_REPO`    | não                 | `dono/repositório` (padrão `esc4n0rx/VynPad`)                                                                                                 |
| `PLAY_STORE_URL` | não                 | Link da Google Play (`https://play.google.com/...`). Quando existe, o Android vira "disponível" e `/api/download/android` redireciona para lá |
| `SITE_URL`       | em produção         | Endereço público (`https://...`): canonical, sitemap, robots e Open Graph                                                                     |
| `GITHUB_URL`     | não                 | Link público do repositório; o rodapé e as novidades só o mostram se for informado                                                            |

Criar o token: GitHub → Settings → Developer settings → Fine-grained tokens → repositório
`esc4n0rx/VynPad` → permissões: **Contents: Read-only** (e nada mais). Defina uma validade e anote
para renová-lo.

## Publicar (Vercel)

1. Importe o repositório e use **Root Directory** `apps/web`.
2. Em Settings → Environment Variables, cadastre `GITHUB_TOKEN`, `SITE_URL` e, quando existir,
   `PLAY_STORE_URL` (Production e Preview).
3. Para o site refletir um release novo na hora, chame
   `revalidateTag("releases")`/redeploy; sem isso ele atualiza sozinho em até 5 minutos.

## Estrutura

```
src/app/[locale]/        páginas (home, privacy, changelog), layout e 404
src/app/api/download/    rota de download (redirecionamento)
src/components/          interface (hero, seções, cartões de download, tema, idioma)
src/lib/                 config, GitHub, download, markdown seguro, detecção de sistema
src/i18n/                dicionários pt-BR e en (o tipo garante que têm as mesmas chaves)
src/proxy.ts             redireciona `/` para o idioma da pessoa (cookie, depois Accept-Language)
```
