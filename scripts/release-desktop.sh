#!/usr/bin/env bash
# Gera, assina e sobe o release do VynPad para Windows (o mesmo que o workflow release.yml faz no CI).
#
# Uso (Git Bash, na raiz do repositório, com a árvore limpa e na main):  scripts/release-desktop.sh
#
# O que faz: confere versão e chave, compila o NSIS e o MSI, assina (código e atualização), monta o
# latest.json e as notas e cria o GitHub Release como RASCUNHO. Nada vira público até você abrir o
# rascunho no GitHub e clicar em "Publish release".
#
# Chaves e senhas (nunca vão para arquivo nem para o histórico):
#   Atualizador (obrigatório)  TAURI_SIGNING_PRIVATE_KEY_PATH  (padrão ~/.vynpad/updater-v1.key)
#                              TAURI_SIGNING_PRIVATE_KEY_PASSWORD (pedida na hora se não definida)
#   Assinatura de código (opcional, Azure Trusted Signing): AZURE_TENANT_ID, AZURE_CLIENT_ID,
#     AZURE_CLIENT_SECRET, TRUSTED_SIGNING_ENDPOINT, TRUSTED_SIGNING_ACCOUNT, TRUSTED_SIGNING_PROFILE.
#     Sem elas o instalador sai SEM assinatura e o SmartScreen avisa "editor desconhecido".
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"
repo="${GITHUB_REPOSITORY:-esc4n0rx/VynPad}"
conf="apps/desktop/src-tauri/tauri.conf.json"
out="$root/target/release-local"

version="$(node -p "require('./$conf').version")"
tag="v$version"

echo "== VynPad $tag =="

# ---- Conferências
if [ -n "$(git status --porcelain)" ]; then
  echo "A árvore tem mudanças não commitadas. Commite (ou guarde) antes de gerar um release." >&2
  exit 1
fi
if [ "$(git rev-parse --abbrev-ref HEAD)" != "main" ]; then
  echo "Gere releases a partir da main." >&2
  exit 1
fi
git fetch --quiet origin main
if [ "$(git rev-parse HEAD)" != "$(git rev-parse origin/main)" ]; then
  echo "A main local não é a mesma do GitHub (faça push ou pull antes): o release sairia de um commit que não está lá." >&2
  exit 1
fi
if gh release view "$tag" --repo "$repo" >/dev/null 2>&1; then
  echo "O release $tag já existe no GitHub. Aumente a versão em $conf (e nos Cargo.toml)." >&2
  exit 1
fi

# ---- Chave do atualizador
key_path="${TAURI_SIGNING_PRIVATE_KEY_PATH:-$HOME/.vynpad/updater-v1.key}"
if [ ! -f "$key_path" ]; then
  echo "Chave do atualizador não encontrada: $key_path" >&2
  exit 1
fi
if [ -f "$key_path.pub" ]; then
  conf_pub="$(node -p "require('./$conf').plugins.updater.pubkey")"
  if [ "$(tr -d '\r\n' < "$key_path.pub")" != "$conf_pub" ]; then
    echo "A chave pública de $key_path.pub não é a 'pubkey' do $conf: os apps não aceitariam a atualização." >&2
    exit 1
  fi
fi
export TAURI_SIGNING_PRIVATE_KEY
TAURI_SIGNING_PRIVATE_KEY="$(cat "$key_path")"
if [ -z "${TAURI_SIGNING_PRIVATE_KEY_PASSWORD+x}" ]; then
  read -r -s -p "Senha da chave do atualizador (Enter se não tem): " TAURI_SIGNING_PRIVATE_KEY_PASSWORD
  echo
fi
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD

# ---- Assinatura de código (opcional)
mkdir -p "$out"
signed=false
if [ -n "${AZURE_CLIENT_ID:-}" ] && [ -n "${TRUSTED_SIGNING_ACCOUNT:-}" ]; then
  command -v trusted-signing-cli >/dev/null 2>&1 || cargo install trusted-signing-cli --locked
  OUT_JSON="$out/signing.json" node -e '
    const cmd = `trusted-signing-cli -e ${process.env.TRUSTED_SIGNING_ENDPOINT} -a ${process.env.TRUSTED_SIGNING_ACCOUNT} -c ${process.env.TRUSTED_SIGNING_PROFILE} -d VynPad %1`;
    require("fs").writeFileSync(process.env.OUT_JSON, JSON.stringify({ bundle: { createUpdaterArtifacts: true, windows: { signCommand: cmd } } }));
  '
  signed=true
else
  echo
  echo "AVISO: sem os segredos do Azure Trusted Signing o instalador sai SEM assinatura de código"
  echo "       (o Windows SmartScreen vai alertar quem baixar)."
  read -r -p "Continuar mesmo assim? [s/N] " answer
  [ "$answer" = "s" ] || [ "$answer" = "S" ] || exit 1
  echo '{"bundle":{"createUpdaterArtifacts":true}}' > "$out/signing.json"
fi
config="$out/signing.json"
if command -v cygpath >/dev/null 2>&1; then config="$(cygpath -m "$config")"; fi

# ---- Build
pnpm install --frozen-lockfile
pnpm --filter @vynpad/desktop tauri build --bundles nsis,msi --config "$config"

nsis="target/release/bundle/nsis"
msi="target/release/bundle/msi"
if [ "$signed" = true ]; then
  powershell.exe -NoProfile -Command "\$bad = Get-ChildItem target/release/bundle -Recurse -Include *.exe,*.msi | Where-Object { (Get-AuthenticodeSignature \$_.FullName).Status -ne 'Valid' }; if (\$bad) { \$bad.FullName; throw 'Arquivos sem assinatura válida' }"
fi

# ---- Notas, manifesto do atualizador e release (rascunho)
node scripts/changelog.mjs --tag "$tag" --repo "$repo" --out "$out/notes.md"
node scripts/make-latest-json.mjs --tag "$tag" --repo "$repo" --dir "$nsis" --notes-file "$out/notes.md" --out "$out/latest.json"

gh release create "$tag" --repo "$repo" --draft --target main \
  --notes-file "$out/notes.md" --title "VynPad $tag" \
  "$nsis"/*.exe "$nsis"/*.exe.sig "$msi"/*.msi "$out/latest.json"

echo
echo "Rascunho criado. Revise e publique em:  https://github.com/$repo/releases"
echo "(Publicar cria a tag $tag; só então o site e o atualizador enxergam a versão.)"
