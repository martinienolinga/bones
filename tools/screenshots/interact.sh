#!/usr/bin/env bash
# Session scriptée : lance l'app sur un écran virtuel (Xvfb + lavapipe), puis
# exécute les actions lues sur l'entrée standard, une par ligne :
#
#   wait <s>                       attendre
#   click <x> <y>                  clic gauche (un toucher)
#   drag <x1> <y1> <x2> <y2>       glisser du bouton gauche (un doigt)
#   rdrag <x1> <y1> <x2> <y2>      glisser du bouton droit
#   wheel up|down <x> <y> <n>      molette (zoom)
#   key <touche>                   touche xdotool (ex. ctrl+z, Delete)
#   shot <nom>                     capture dans $OUT_DIR/<nom>.png
#
# Exemple :
#   printf 'wait 2\nclick 600 400\nshot gallery\n' | tools/screenshots/interact.sh
#
# Variables : BIN, OUT_DIR, BONES_DOCUMENTS, SIZE, DISPLAY_NUM (défaut : 98),
# STARTUP (secondes avant la première action, défaut : 10) — voir shot.sh.
# Le multitouch ne se simule pas sous X11 : les gestes à plusieurs doigts sont
# couverts par les tests unitaires de crates/bones-app/src/input/tests.rs.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT_DIR="${OUT_DIR:-$ROOT/target/screenshots}"
export BONES_DOCUMENTS="${BONES_DOCUMENTS:-$OUT_DIR/docs}"
mkdir -p "$OUT_DIR" "$BONES_DOCUMENTS"

export DISPLAY=":${DISPLAY_NUM:-98}"
Xvfb "$DISPLAY" -screen 0 "${SIZE:-1366x1024}x24" >/dev/null 2>&1 &
XPID=$!
APID=""
trap 'kill $APID $XPID 2>/dev/null || true; wait 2>/dev/null || true' EXIT
sleep 1

WINIT_UNIX_BACKEND=x11 "${BIN:-$ROOT/target/debug/bones}" >"$OUT_DIR/interact.log" 2>&1 &
APID=$!
sleep "${STARTUP:-10}"

# Glissement en 8 pas, pour que l'app voie un vrai mouvement.
glide() {
    local button=$1 x1=$2 y1=$3 x2=$4 y2=$5
    xdotool mousemove "$x1" "$y1" mousedown "$button"
    for i in 1 2 3 4 5 6 7 8; do
        xdotool mousemove $((x1 + (x2 - x1) * i / 8)) $((y1 + (y2 - y1) * i / 8))
        sleep 0.08
    done
    xdotool mouseup "$button"
    sleep 1
}

while read -r cmd a b c d; do
    case "$cmd" in
        wait) sleep "$a" ;;
        click) xdotool mousemove "$a" "$b" click 1; sleep 1 ;;
        drag) glide 1 "$a" "$b" "$c" "$d" ;;
        rdrag) glide 3 "$a" "$b" "$c" "$d" ;;
        wheel)
            xdotool mousemove "$b" "$c"
            for _ in $(seq 1 "$d"); do
                xdotool click "$([ "$a" = up ] && echo 4 || echo 5)"
                sleep 0.15
            done
            sleep 1
            ;;
        key) xdotool key "$a"; sleep 1 ;;
        shot) import -window root "$OUT_DIR/$a.png"; echo "Capture : $OUT_DIR/$a.png" ;;
        "" | \#*) ;;
        *) echo "Action inconnue : $cmd" >&2 ;;
    esac
done

grep -i "panick\|error" "$OUT_DIR/interact.log" | head -5 || true
