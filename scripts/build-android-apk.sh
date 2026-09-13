#!/usr/bin/env bash
# Construit l'APK Android de RivaldSend (téléphone ARM64).
# Usage : ./scripts/build-android-apk.sh [--release|--debug]  (défaut : release)
# - release : petite taille (ProGuard + symboles retirés), signée avec un
#   keystore local créé automatiquement (~/.config/rivaldsend-release-key.jks).
# - debug : grosse (~270 Mo), signature debug auto, utile pour logcat.
# Durée : 10 à 20 min la première fois (Gradle + compilation release).
set -euo pipefail

MODE="${1:---release}"
RACINE="$(cd "$(dirname "$0")/.." && pwd)"

# Localise le SDK (ici : ~/.Android/Sdk, pas ~/Android/Sdk).
if [ -d "$HOME/.Android/Sdk" ]; then SDK="$HOME/.Android/Sdk";
elif [ -d "$HOME/Android/Sdk" ]; then SDK="$HOME/Android/Sdk";
else echo "SDK Android introuvable." >&2; exit 1; fi

export ANDROID_HOME="$SDK"
export ANDROID_SDK_ROOT="$SDK"
export NDK_HOME="$SDK/ndk/27.1.12297006"
export ANDROID_NDK_HOME="$SDK/ndk/27.1.12297006"
export JAVA_HOME=/usr/lib/jvm/java-17-openjdk-amd64
LLVM="$SDK/ndk/27.1.12297006/toolchains/llvm/prebuilt/linux-x86_64/bin"
export CC_aarch64_linux_android="$LLVM/aarch64-linux-android24-clang"
export AR_aarch64_linux_android="$LLVM/llvm-ar"

ARGS=(-t aarch64 --apk --ci)
if [ "$MODE" = "--debug" ]; then
  ARGS+=(-d)
  echo "Mode debug (APK volumineuse, signature auto)."
else
  # Keystore local pour la release (créé une seule fois, jamais commité).
  MAGASIN="$HOME/.config/rivaldsend-release-key.jks"
  if [ ! -f "$MAGASIN" ]; then
    echo "Création du keystore local $MAGASIN…"
    keytool -genkeypair -keystore "$MAGASIN" -alias rivaldsend \
      -keyalg RSA -keysize 2048 -validity 10000 \
      -storepass android -keypass android \
      -dname "CN=RivaldSend Test, OU=Test, O=Test, C=FR"
  fi
  export TAURI_ANDROID_KEYSTORE_PATH="$MAGASIN"
  export TAURI_ANDROID_KEYSTORE_PASSWORD=android
  export TAURI_ANDROID_KEY_ALIAS=rivaldsend
  export TAURI_ANDROID_KEY_PASSWORD=android
  ARGS+=(--split-per-abi)
  echo "Mode release (APK par ABI, taille minimale)."
fi

cd "$RACINE/crates/rivaldsend-app"
# Le chemin des assets web est fixé dans tauri.conf.json (../../dist).
"$RACINE/node_modules/.bin/tauri" android build "${ARGS[@]}"

if [ "$MODE" != "--debug" ]; then
  # Les variables TAURI_ANDROID_* ne sont pas reprises par Gradle : l'APK sort
  # *-unsigned et Android refuse de l'installer. On signe donc à la main.
  OUT="$RACINE/crates/rivaldsend-app/gen/android/app/build/outputs/apk/arm64/release"
  NON_SIGNE="$OUT/app-arm64-release-unsigned.apk"
  SIGNE="$OUT/app-arm64-release.apk"
  BT="$SDK/build-tools/36.0.0"
  "$BT/zipalign" -f 4 "$NON_SIGNE" "$OUT/app-arm64-release-aligned.apk"
  "$BT/apksigner" sign --ks "$MAGASIN" --ks-pass pass:android \
    --key-pass pass:android --out "$SIGNE" "$OUT/app-arm64-release-aligned.apk"
  "$BT/apksigner" verify "$SIGNE" && echo "APK signée et vérifiée : $SIGNE"
  rm -f "$OUT/app-arm64-release-aligned.apk"
fi
