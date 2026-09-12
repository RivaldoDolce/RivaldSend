#!/usr/bin/env bash
# Construit l'APK debug Android de RivaldSend (téléphone ARM64).
# Usage : ./scripts/build-android-apk.sh
# Durée : 10 à 20 min la première fois (téléchargement Gradle + dépendances).
set -euo pipefail

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

cd "$RACINE/crates/rivaldsend-app"
exec "$RACINE/node_modules/.bin/tauri" android build -d -t aarch64 --apk --ci \
  -c "{\"build\": {\"frontendDist\": \"$RACINE/dist\"}}"
