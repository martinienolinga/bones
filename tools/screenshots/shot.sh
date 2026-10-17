#!/usr/bin/env bash
# Lance l'app sur un écran virtuel (Xvfb + lavapipe) et prend une capture.
#
# Usage : tools/screenshots/shot.sh [sortie.png]
#
# Variables :
#   BIN              binaire de l'app (défaut : target/debug/bones)
#   OUT_DIR          dossier des captures et du journal (défaut : target/screenshots)
#   BONES_DOCUMENTS  dossier Documents de l'app (défaut : $OUT_DIR/docs)
#   WAIT             secondes avant la capture (défaut : 8)
#   SIZE             taille de l'écran (défaut : 1366x1024, iPad 12,9" en points)
#   DISPLAY_NUM      numéro d'écran Xvfb (défaut : 99)
#
# Prérequis (Linux) : Xvfb, ImageMagick (import), pilote Vulkan lavapipe.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT_DIR="${OUT_DIR:-$ROOT/target/screenshots}"
OUT="${1:-$OUT_DIR/app.png}"
export BONES_DOCUMENTS="${BONES_DOCUMENTS:-$OUT_DIR/docs}"
mkdir -p "$OUT_DIR" "$BONES_DOCUMENTS" "$(dirname "$OUT")"

DISPLAY_NUM="${DISPLAY_NUM:-99}"
Xvfb ":$DISPLAY_NUM" -screen 0 "${SIZE:-1366x1024}x24" >/dev/null 2>&1 &
XPID=$!
APID=""
trap 'kill $APID $XPID 2>/dev/null || true; wait 2>/dev/null || true' EXIT
sleep 1

DISPLAY=":$DISPLAY_NUM" WINIT_UNIX_BACKEND=x11 RUST_LOG=warn \
    "${BIN:-$ROOT/target/debug/bones}" >"$OUT_DIR/app.log" 2>&1 &
APID=$!
sleep "${WAIT:-8}"

DISPLAY=":$DISPLAY_NUM" import -window root "$OUT"
echo "Capture : $OUT"
tail -5 "$OUT_DIR/app.log"
