#!/usr/bin/env bash
# Regenera os metadados de `.sqlx/` que as macros `query!` do sqlx leem no modo offline
# (SQLX_OFFLINE=true, ver .cargo/config.toml). Rode depois de mudar uma query ou uma migration
# e versione o resultado. Não precisa do sqlx-cli: as macros gravam os metadados sozinhas quando
# compilam contra um banco de verdade e `SQLX_OFFLINE_DIR` está definido.
set -euo pipefail
cd "$(dirname "$0")/.."

ROOT="$(pwd -W 2>/dev/null || pwd)" # no Git Bash, o caminho no formato do Windows
DIR="target/sqlx-prepare"

cargo run -q -p vynpad-dbprep -- "$DIR"
rm -rf .sqlx
mkdir .sqlx

export DATABASE_URL="sqlite:$ROOT/$DIR/vynpad.db"
export SQLX_OFFLINE=false
export SQLX_OFFLINE_DIR="$ROOT/.sqlx"
# Força a expansão das macros de novo.
cargo clean -q -p vynpad-storage
cargo check -q -p vynpad-storage --all-targets

echo "metadados gerados: $(ls .sqlx | wc -l) queries em .sqlx/"
