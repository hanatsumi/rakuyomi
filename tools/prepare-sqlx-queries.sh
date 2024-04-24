#!/usr/bin/env bash
set -ev

export BACKEND_CLI_DIR="${PROJECT_DIR}/backend/cli"
export DATABASE_URL="sqlite:/tmp/rakuyomi.db"

cd "$BACKEND_CLI_DIR"

cargo sqlx db create
cargo sqlx migrate run
cargo sqlx prepare

echo "sqlx queries prepared successfully!"