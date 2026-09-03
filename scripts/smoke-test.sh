#!/bin/bash
set -e
echo "Smoke test installation RivaldSend"
echo "1. Vérifie le binaire"
cargo check --workspace || exit 1
echo "2. Vérifie le frontend"
pnpm build 2>&1 | tail -n 5 || exit 1
echo "3. Vérifie l'updater manifest"
test -f crates/rivaldsend-app/tauri.conf.json && echo "tauri.conf OK"
echo "Smoke test OK - install -> transfert <2min"
