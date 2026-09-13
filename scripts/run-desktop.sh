#!/usr/bin/env bash
# Lance l'app desktop installée avec les contournements WebKit nécessaires
# sur les pilotes qui font écran noir (EGL/DRI2, cas vu sur Pop!_OS).
# Usage : ./scripts/run-desktop.sh [--no-composite]
# Si l'app s'affiche avec ce script mais pas depuis le dock, lancez
# scripts/fix-desktop-launcher.sh pour rendre le correctif permanent.
set -euo pipefail

BINAIRE=""
for candidat in rivaldsend-app rivaldsend; do
  if command -v "$candidat" >/dev/null 2>&1; then BINAIRE="$candidat"; break; fi
done
if [ -z "$BINAIRE" ]; then
  for candidat in /usr/bin/rivaldsend-app /usr/bin/rivaldsend /usr/local/bin/rivaldsend-app; do
    if [ -x "$candidat" ]; then BINAIRE="$candidat"; break; fi
  done
fi
if [ -z "$BINAIRE" ]; then
  echo "Binaire introuvable : installe d'abord le .deb (voir scripts/build-desktop.sh)." >&2
  exit 1
fi

export WEBKIT_DISABLE_DMABUF_RENDERER=1
if [ "${1:-}" = "--no-composite" ]; then
  export WEBKIT_DISABLE_COMPOSITING_MODE=1
  shift
  echo "Contournement renforcé (DMABUF + compositing désactivés)."
fi
echo "Lancement de $BINAIRE…"
echo "(Si l'écran reste noir, colle-moi les logs affichés ci-dessous.)"
exec "$BINAIRE" "$@"
