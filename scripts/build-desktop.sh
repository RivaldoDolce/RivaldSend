#!/usr/bin/env bash
# Construit l'app desktop RivaldSend en release (.deb/.AppImage/.rpm).
# Usage : ./scripts/build-desktop.sh
# Durée : ~10 min la première fois (compilation release complète).
# Prérequis (Pop!_OS/Ubuntu 22.04) : webkit2gtk-4.1-dev, gtk-3, ayatana, rsvg…
set -euo pipefail

RACINE="$(cd "$(dirname "$0")/.." && pwd)"

command -v pnpm >/dev/null || { echo "pnpm introuvable." >&2; exit 1; }
pkg-config --exists webkit2gtk-4.1 2>/dev/null \
  || echo "Astuce : s'il manque WebKit : sudo apt install libwebkit2gtk-4.1-dev"

cd "$RACINE"
pnpm tauri build

PAQUETS="$RACINE/target/release/bundle"
echo ""
echo "Paquets générés :"
find "$PAQUETS" \( -name "*.deb" -o -name "*.AppImage" -o -name "*.rpm" \) 2>/dev/null || true
echo ""
echo "Installation (exemple .deb) :"
echo "  sudo apt install $PAQUETS/deb/rivaldsend_*_amd64.deb $PAQUETS/deb/RivaldSend_*_amd64.deb"
echo "Si la fenêtre reste noire au lancement, voir scripts/run-desktop.sh"
echo "puis scripts/fix-desktop-launcher.sh pour un correctif permanent."
