#!/usr/bin/env bash
# Gera, assina e sobe um release completo do VynPad (Android + Windows) de uma vez.
#
# Uso (Git Bash, na raiz do repositório, com a árvore limpa e na main):
#   scripts/release.sh [X.Y.Z] [opções]
#
#   X.Y.Z            versão nova (sobe a versão em todos os arquivos, commita e dá push). Sem ela,
#                    usa a versão atual do tauri.conf.json (primeiro release, ou repetir uma tentativa).
#   --publish        publica o release no GitHub na hora (padrão: rascunho, para você revisar antes)
#   --with-apk       anexa o APK assinado ao release (para baixar fora da Play; veja o aviso abaixo)
#   --skip-checks    pula as verificações de qualidade (fmt, clippy, testes, lint) antes do build
#   --skip-android   não gera o Android
#   --skip-desktop   não gera o Windows nem cria o release no GitHub
#   --dry-run        só confere tudo e mostra o plano, sem alterar nada
#
# O que acontece, em ordem:
#   1. confere árvore limpa, branch main e sincronia com o GitHub
#   2. verificações de qualidade (as mesmas do CI)
#   3. se há versão nova: troca a versão (Cargo, Tauri, Android), commita e dá push
#   4. Android: testes, lint, AAB e APK assinados com a chave de upload
#   5. Windows: instalador NSIS e MSI, assinatura do atualizador, latest.json, notas e GitHub Release
#
# Senhas: a do keystore do Android e a da chave do atualizador são pedidas UMA vez, aqui, e passadas
# aos outros scripts; nunca vão para arquivo nem histórico. Para não perguntar, defina
# VYNPAD_KEYSTORE_PASSWORD e TAURI_SIGNING_PRIVATE_KEY_PASSWORD no ambiente.
# Sem os segredos do Azure Trusted Signing o instalador sai SEM assinatura de código (o SmartScreen
# avisa); este script segue assim de propósito. Para assinar, defina as variáveis AZURE_* e
# TRUSTED_SIGNING_* (veja scripts/release-desktop.sh).
#
# Aviso do APK: um APK assinado com a chave de upload e instalado fora da Play não é atualizado pela
# Play (a assinatura final é a da Google); quem o instalou precisa desinstalar antes.
#
# O AAB para a Play Console fica em target/release-local/VynPad-X.Y.Z.aab; o envio à Play é manual.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

version=""
publish=0
with_apk=0
checks=1
android=1
desktop=1
dry=0
for arg in "$@"; do
  case "$arg" in
    --publish) publish=1 ;;
    --with-apk) with_apk=1 ;;
    --skip-checks) checks=0 ;;
    --skip-android) android=0 ;;
    --skip-desktop) desktop=0 ;;
    --dry-run) dry=1 ;;
    -h | --help) sed -n '2,32p' "${BASH_SOURCE[0]}"; exit 0 ;;
    -*) echo "Opção desconhecida: $arg (use --help)" >&2; exit 2 ;;
    *) version="$arg" ;;
  esac
done

conf="apps/desktop/src-tauri/tauri.conf.json"
current="$(node -p "require('./$conf').version")"
version="${version:-$current}"
if ! [[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "Versão inválida: $version (use X.Y.Z)" >&2
  exit 2
fi
tag="v$version"
repo="${GITHUB_REPOSITORY:-esc4n0rx/VynPad}"
out="$root/target/release-local"

step() { echo; echo "==> $*"; }

# ---- 1. Conferências
step "Conferindo o repositório"
if [ -n "$(git status --porcelain)" ]; then
  echo "A árvore tem mudanças não commitadas. Commite (ou guarde) antes." >&2
  exit 1
fi
if [ "$(git rev-parse --abbrev-ref HEAD)" != "main" ]; then
  echo "Gere releases a partir da main." >&2
  exit 1
fi
git fetch --quiet origin main
if [ "$(git rev-parse HEAD)" != "$(git rev-parse origin/main)" ]; then
  echo "A main local não é a mesma do GitHub (faça push ou pull antes)." >&2
  exit 1
fi
if gh release view "$tag" --repo "$repo" >/dev/null 2>&1; then
  echo "O release $tag já existe no GitHub. Use uma versão nova: scripts/release.sh X.Y.Z" >&2
  exit 1
fi
if [ "$version" != "$current" ] && git rev-parse -q --verify "refs/tags/$tag" >/dev/null; then
  echo "A tag $tag já existe." >&2
  exit 1
fi

echo "Versão: $version (atual: $current)  |  tag: $tag  |  repo: $repo"
echo "Android: $([ $android = 1 ] && echo sim || echo não)  |  Windows: $([ $desktop = 1 ] && echo sim || echo não)  |  publicar na hora: $([ $publish = 1 ] && echo sim || echo "não (rascunho)")  |  APK no release: $([ $with_apk = 1 ] && echo sim || echo não)"
if [ $dry = 1 ]; then
  echo
  echo "--dry-run: nada foi alterado."
  exit 0
fi

# ---- Senhas (uma vez só)
if [ $android = 1 ] && [ -z "${VYNPAD_KEYSTORE_PASSWORD:-}" ]; then
  read -r -s -p "Senha do keystore do Android: " VYNPAD_KEYSTORE_PASSWORD
  echo
fi
export VYNPAD_KEYSTORE_PASSWORD="${VYNPAD_KEYSTORE_PASSWORD:-}"
if [ $desktop = 1 ] && [ -z "${TAURI_SIGNING_PRIVATE_KEY_PASSWORD+x}" ]; then
  read -r -s -p "Senha da chave do atualizador (Enter se não tem): " TAURI_SIGNING_PRIVATE_KEY_PASSWORD
  echo
fi
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="${TAURI_SIGNING_PRIVATE_KEY_PASSWORD-}"
if [ -z "${AZURE_CLIENT_ID:-}" ]; then
  export VYNPAD_ALLOW_UNSIGNED=1
  echo "(Sem assinatura de código do Windows: o instalador vai gerar o aviso do SmartScreen.)"
fi
[ $publish = 1 ] && export VYNPAD_PUBLISH=1

# ---- 2. Qualidade
if [ $checks = 1 ]; then
  step "Verificações de qualidade"
  cargo fmt --all --check
  pnpm format:check
  pnpm lint
  pnpm typecheck
  pnpm test
  cargo clippy --workspace --all-targets --locked -- -D warnings
  cargo test --workspace --exclude vynpad-desktop --locked
else
  echo "(verificações de qualidade puladas)"
fi

# ---- 3. Versão nova
if [ "$version" != "$current" ]; then
  step "Subindo a versão para $version"
  code="$(node scripts/bump-version.mjs "$version")"
  cargo update --workspace --quiet
  git add Cargo.toml Cargo.lock apps/desktop/package.json "$conf" apps/android/androidApp/build.gradle.kts
  git commit -q -m "chore(release): $tag"
  git push origin main
  echo "versionCode do Android: $code"
fi

mkdir -p "$out"
extra=""

# ---- 4. Android
if [ $android = 1 ]; then
  step "Android (AAB e APK assinados)"
  scripts/release-android.sh
  cp apps/android/androidApp/build/outputs/bundle/release/androidApp-release.aab "$out/VynPad-$version.aab"
  cp apps/android/androidApp/build/outputs/apk/release/androidApp-release.apk "$out/VynPad-$version.apk"
  [ $with_apk = 1 ] && extra="$out/VynPad-$version.apk"
fi

# ---- 5. Windows e GitHub Release
if [ $desktop = 1 ]; then
  step "Windows (instalador, assinatura do atualizador e GitHub Release)"
  VYNPAD_EXTRA_ASSETS="$extra" scripts/release-desktop.sh
fi

step "Pronto"
[ $android = 1 ] && echo "AAB para a Play Console: $out/VynPad-$version.aab   (envie em Teste interno)"
[ $android = 1 ] && echo "APK assinado (para testar):  $out/VynPad-$version.apk"
[ $desktop = 1 ] && echo "GitHub: https://github.com/$repo/releases"
[ $desktop = 1 ] && [ $publish = 0 ] && echo "O release está como RASCUNHO: revise e clique em \"Publish release\"."
echo "O site mostra a versão nova em até 5 minutos depois da publicação."
