#!/usr/bin/env bash
# Die Fugen in examples/fugen: zu den zwanzig Themen, die mit fux-fuge kommen, je eine drei- und
# eine vierstimmige Fuge (Seed 1), als PDF, MIDI und MuseScore, dazu der Bericht des Programms
# als Text. Mit ALLES=1 bleiben auch MusicXML und LilyPond liegen.
#
#   tools/make_fugues.sh [Zielverzeichnis] [Themen …]
#
# Braucht LilyPond für die PDF und MuseScore (mscore) für die .mscz-Dateien.
set -euo pipefail
cd "$(dirname "$0")/.."
out="${1:-examples/fugen}"
shift || true
mkdir -p "$out"
cargo build --release -p fux-bach --bin fux-fuge
fuge=target/release/fux-fuge
if [ "$#" -gt 0 ]; then
    themes=("$@")
else
    mapfile -t themes < <("$fuge" themes | awk '/^[a-z]/ && $1 != "Pläne" { print $1 }')
fi
for theme in "${themes[@]}"; do
    for voices in 3 4; do
        stem="$out/$theme-${voices}st"
        QT_QPA_PLATFORM=offscreen "$fuge" fugue --theme "$theme" --voices "$voices" --seed 1 \
            --out "$stem" --pdf --mscz 2> /dev/null | grep -v "^$out/" > "$stem.txt"
        [ -n "${ALLES:-}" ] || rm -f "$stem.musicxml" "$stem.ly" "$stem.akkorde.txt"
        echo "$stem: $(sed -n 2p "$stem.txt")"
    done
done
if [ "$out" = "examples/fugen" ] && [ "$#" -eq 0 ]; then
    python3 tools/fugen_readme.py
fi
