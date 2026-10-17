#!/usr/bin/env bash
# Construit, installe et lance Bones dans un simulateur iPad.
#
# Usage : ios/run-sim.sh [debug|release]
#
# Si aucun simulateur n'est démarré, le premier iPad disponible est lancé
# (variable BONES_SIM_DEVICE pour forcer un modèle, ex. "iPad Pro 13-inch (M4)").
set -euo pipefail

CONFIG="${1:-debug}"
BUNDLE_ID="com.paintsharp.bones"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

"$ROOT/ios/build.sh" sim "$CONFIG"

TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
APP_DIR="$TARGET_DIR/ios/$CONFIG/Bones.app"

booted_udid() {
    xcrun simctl list devices booted | sed -nE 's/.*\(([0-9A-F-]{36})\) \(Booted\).*/\1/p' | head -n 1
}

UDID="$(booted_udid)"
if [[ -z "$UDID" ]]; then
    if [[ -n "${BONES_SIM_DEVICE:-}" ]]; then
        PATTERN="$BONES_SIM_DEVICE"
    else
        PATTERN="iPad"
    fi
    UDID="$(xcrun simctl list devices available \
        | grep -F "$PATTERN" \
        | sed -nE 's/.*\(([0-9A-F-]{36})\) \((Shutdown|Booted)\).*/\1/p' \
        | head -n 1)"
    if [[ -z "$UDID" ]]; then
        echo "Aucun simulateur iPad disponible. Installez-en un depuis Xcode > Settings > Platforms." >&2
        exit 1
    fi
    echo "==> démarrage du simulateur $UDID"
    xcrun simctl boot "$UDID"
    open -a Simulator
fi

echo "==> installation sur $UDID"
xcrun simctl install "$UDID" "$APP_DIR"
echo "==> lancement"
xcrun simctl launch --console-pty "$UDID" "$BUNDLE_ID"
