#!/usr/bin/env bash
set -euo pipefail

# Optimize assets for runtime use
# Requires: cwebp (webp), optionally optipng for fallback
# Run once: bash scripts/optimize-assets.sh

SRC="assets/Images"
OUT="public/assets"

mkdir -p "$OUT"

echo "=== Optimizing onboarding background ==="
cwebp -q 75 -resize 1600 0 "$SRC/onboarding-bg.png" -o "$OUT/onboarding-bg.webp" 2>/dev/null && \
  echo "  -> onboarding-bg.webp ($(du -h "$OUT/onboarding-bg.webp" | cut -f1))" || \
  echo "  SKIP (cwebp not installed)"

echo "=== Optimizing concept transfer watermark ==="
cwebp -q 70 -resize 800 0 "$SRC/concept-transfer.png" -o "$OUT/concept-transfer.webp" 2>/dev/null && \
  echo "  -> concept-transfer.webp ($(du -h "$OUT/concept-transfer.webp" | cut -f1))" || \
  echo "  SKIP (cwebp not installed)"

echo "=== Optimizing empty states ==="
for f in empty-no-peers empty-no-history empty-error empty-empty-transfer; do
  if [ -f "$SRC/$f.png" ]; then
    cwebp -q 80 "$SRC/$f.png" -o "$OUT/$f.webp" 2>/dev/null && \
      echo "  -> $f.webp ($(du -h "$OUT/$f.webp" | cut -f1))" || \
      echo "  SKIP $f (cwebp not installed)"
  fi
done

echo "=== Copying platform icons ==="
for f in icon-desktop icon-mobile; do
  if [ -f "$SRC/$f.png" ]; then
    cwebp -q 80 "$SRC/$f.png" -o "$OUT/$f.webp" 2>/dev/null && \
      echo "  -> $f.webp ($(du -h "$OUT/$f.webp" | cut -f1))" || \
      echo "  SKIP $f (cwebp not installed)"
  fi
done

echo ""
echo "Done. Total runtime assets:"
du -sh "$OUT" 2>/dev/null || echo "  (empty)"
echo ""
echo "Note: Design/*.png are reference-only, never bundled."
echo "      social-cover.png excluded (marketing only)."
