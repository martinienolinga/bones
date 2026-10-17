#!/usr/bin/env bash
# Construit le paquet iPad Bones.app à partir du binaire Rust.
#
# Usage : ios/build.sh [sim|device] [debug|release]
#
#   sim     → simulateur iPad sur Mac Apple Silicon (aarch64-apple-ios-sim),
#             signature ad hoc.
#   device  → iPad réel (aarch64-apple-ios). Variables requises :
#             BONES_SIGN_IDENTITY        identité de signature (ex. "Apple Development: …")
#             BONES_PROVISIONING_PROFILE chemin du profil .mobileprovision
#
# Résultat : target/ios/<config>/Bones.app
set -euo pipefail

DESTINATION="${1:-sim}"
CONFIG="${2:-debug}"
BUNDLE_ID="com.paintsharp.bones"
BIN_NAME="bones"
APP_NAME="Bones"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

case "$DESTINATION" in
    sim) TARGET="aarch64-apple-ios-sim" ;;
    device) TARGET="aarch64-apple-ios" ;;
    *)
        echo "Destination inconnue : $DESTINATION (attendu : sim ou device)" >&2
        exit 64
        ;;
esac

case "$CONFIG" in
    debug) CARGO_FLAGS=() ;;
    release) CARGO_FLAGS=(--release) ;;
    *)
        echo "Configuration inconnue : $CONFIG (attendu : debug ou release)" >&2
        exit 64
        ;;
esac

if [[ "$(uname -s)" != "Darwin" ]]; then
    echo "La construction iOS nécessite macOS avec Xcode 15 ou plus récent." >&2
    exit 1
fi

if [[ "$DESTINATION" == "device" ]]; then
    : "${BONES_SIGN_IDENTITY:?définissez BONES_SIGN_IDENTITY (security find-identity -v -p codesigning)}"
    : "${BONES_PROVISIONING_PROFILE:?définissez BONES_PROVISIONING_PROFILE (fichier .mobileprovision)}"
    if [[ ! -f "$BONES_PROVISIONING_PROFILE" ]]; then
        echo "Profil introuvable : $BONES_PROVISIONING_PROFILE" >&2
        exit 1
    fi
fi

export IPHONEOS_DEPLOYMENT_TARGET="16.0"

echo "==> cargo build ($TARGET, $CONFIG)"
cargo build -p bones-app --bin "$BIN_NAME" --target "$TARGET" ${CARGO_FLAGS[@]+"${CARGO_FLAGS[@]}"}

TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
BINARY="$TARGET_DIR/$TARGET/$CONFIG/$BIN_NAME"
APP_DIR="$TARGET_DIR/ios/$CONFIG/$APP_NAME.app"

if [[ ! -f "$BINARY" ]]; then
    echo "Binaire introuvable : $BINARY" >&2
    exit 1
fi

echo "==> assemblage de $APP_DIR"
rm -rf "$APP_DIR"
mkdir -p "$APP_DIR"
cp "$BINARY" "$APP_DIR/$BIN_NAME"
cp "$ROOT/ios/Info.plist" "$APP_DIR/Info.plist"
# Ressources optionnelles (icônes, catalogues compilés…).
if [[ -d "$ROOT/ios/Assets" ]]; then
    cp -R "$ROOT/ios/Assets/." "$APP_DIR/"
fi
plutil -lint "$APP_DIR/Info.plist" >/dev/null

echo "==> signature"
if [[ "$DESTINATION" == "sim" ]]; then
    codesign --force --sign - --timestamp=none "$APP_DIR"
else
    cp "$BONES_PROVISIONING_PROFILE" "$APP_DIR/embedded.mobileprovision"
    WORK_DIR="$(mktemp -d)"
    trap 'rm -rf "$WORK_DIR"' EXIT
    security cms -D -i "$BONES_PROVISIONING_PROFILE" >"$WORK_DIR/profile.plist"
    plutil -extract Entitlements xml1 -o "$WORK_DIR/entitlements.plist" "$WORK_DIR/profile.plist"
    codesign --force --sign "$BONES_SIGN_IDENTITY" \
        --entitlements "$WORK_DIR/entitlements.plist" \
        --generate-entitlement-der \
        "$APP_DIR"
fi
codesign --verify --verbose=1 "$APP_DIR"

echo
echo "Paquet prêt : $APP_DIR"
echo
if [[ "$DESTINATION" == "sim" ]]; then
    echo "Étapes suivantes (simulateur) :"
    echo "  xcrun simctl install booted \"$APP_DIR\""
    echo "  xcrun simctl launch booted $BUNDLE_ID"
    echo "ou simplement : ios/run-sim.sh $CONFIG"
else
    echo "Étapes suivantes (iPad connecté) :"
    echo "  xcrun devicectl list devices"
    echo "  xcrun devicectl device install app --device <UDID> \"$APP_DIR\""
    echo "  xcrun devicectl device process launch --device <UDID> $BUNDLE_ID"
fi
