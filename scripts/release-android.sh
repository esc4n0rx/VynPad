#!/usr/bin/env bash
# Gera o App Bundle (.aab) e o APK de release do Android, assinados com a chave de upload.
#
# Uso (Git Bash, na raiz do repositório):  scripts/release-android.sh
#
# A senha é pedida na hora (não aparece na tela nem fica em arquivo ou histórico). Para rodar sem
# perguntar, defina VYNPAD_KEYSTORE_PASSWORD (e, se for diferente, VYNPAD_KEY_PASSWORD) antes.
# Variáveis opcionais: VYNPAD_KEYSTORE (padrão ~/.vynpad/vynpad-upload.jks), VYNPAD_KEY_ALIAS
# (padrão vynpad-upload), JAVA_HOME e ANDROID_HOME (os padrões do Windows são detectados).
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export VYNPAD_KEYSTORE="${VYNPAD_KEYSTORE:-$HOME/.vynpad/vynpad-upload.jks}"
export VYNPAD_KEY_ALIAS="${VYNPAD_KEY_ALIAS:-vynpad-upload}"

if [ ! -f "$VYNPAD_KEYSTORE" ]; then
  echo "Keystore não encontrado: $VYNPAD_KEYSTORE" >&2
  exit 1
fi

if [ -z "${VYNPAD_KEYSTORE_PASSWORD:-}" ]; then
  read -r -s -p "Senha do keystore: " VYNPAD_KEYSTORE_PASSWORD
  echo
fi
export VYNPAD_KEYSTORE_PASSWORD
export VYNPAD_KEY_PASSWORD="${VYNPAD_KEY_PASSWORD:-$VYNPAD_KEYSTORE_PASSWORD}"

# O caminho do keystore precisa estar no formato do sistema para o Gradle (no Windows, C:/...).
if command -v cygpath >/dev/null 2>&1; then
  VYNPAD_KEYSTORE="$(cygpath -m "$VYNPAD_KEYSTORE")"
  export VYNPAD_KEYSTORE
fi

if [ -z "${JAVA_HOME:-}" ] && [ -d "/c/Program Files/Eclipse Adoptium" ]; then
  export JAVA_HOME="/c/Program Files/Eclipse Adoptium/$(ls '/c/Program Files/Eclipse Adoptium' | grep '^jdk-17' | head -1)"
fi
if [ -z "${ANDROID_HOME:-}" ] && [ -n "${LOCALAPPDATA:-}" ]; then
  export ANDROID_HOME="$LOCALAPPDATA/Android/Sdk"
fi

cd "$root/apps/android"
# Mesmos passos do CI, mais o release assinado.
./gradlew :shared:testAndroid :androidApp:lintRelease :androidApp:bundleRelease :androidApp:assembleRelease

aab="androidApp/build/outputs/bundle/release/androidApp-release.aab"
apk="androidApp/build/outputs/apk/release/androidApp-release.apk"

echo
echo "Confere a assinatura do AAB:"
"$JAVA_HOME/bin/jarsigner" -verify "$aab" | head -3
echo
echo "Impressão digital SHA-256 da chave de upload (informe-a ao Play Console se pedir):"
"$JAVA_HOME/bin/keytool" -list -v -keystore "$VYNPAD_KEYSTORE" -alias "$VYNPAD_KEY_ALIAS" -storepass "$VYNPAD_KEYSTORE_PASSWORD" 2>/dev/null | grep -E "SHA256" | head -1
echo
echo "Prontos:"
ls -la "$aab" "$apk" 2>/dev/null
echo
echo "AAB: envie ao Play Console (Teste interno). O APK assinado serve para testar no celular."
