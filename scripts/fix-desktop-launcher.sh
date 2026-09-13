#!/usr/bin/env bash
# Rend permanent le contournement WebKit pour l'app installée :
# copie le .desktop système vers ~/.local (prioritaire) en forçant
# WEBKIT_DISABLE_DMABUF_RENDERER=1 dans la ligne Exec, sans toucher
# au reste de la ligne (arguments %F/%U conservés).
# Usage : ./scripts/fix-desktop-launcher.sh
set -euo pipefail

SOURCE=""
for candidat in /usr/share/applications/RivaldSend.desktop \
                /usr/share/applications/rivaldsend.desktop \
                /usr/local/share/applications/RivaldSend.desktop \
                /usr/local/share/applications/rivaldsend.desktop; do
  if [ -f "$candidat" ]; then SOURCE="$candidat"; break; fi
done
if [ -z "$SOURCE" ]; then
  echo ".desktop introuvable : installe d'abord le .deb (voir scripts/build-desktop.sh)." >&2
  exit 1
fi

DEST="$HOME/.local/share/applications/$(basename "$SOURCE")"
mkdir -p "$(dirname "$DEST")"
# Préfixe env ... uniquement si pas déjà présent (script ré-exécutable).
if grep -qE "^Exec=env WEBKIT_DISABLE_DMABUF_RENDERER=1 " "$SOURCE"; then
  cp "$SOURCE" "$DEST"
else
  sed -E 's|^Exec=(.*)$|Exec=env WEBKIT_DISABLE_DMABUF_RENDERER=1 \1|' \
    "$SOURCE" > "$DEST"
fi
update-desktop-database "$(dirname "$DEST")" 2>/dev/null || true
echo "Correctif appliqué : $DEST"
grep -E "^Exec=" "$DEST"
echo "Relance l'app depuis le dock. Si l'écran reste noir :"
echo "  ./scripts/run-desktop.sh --no-composite"
