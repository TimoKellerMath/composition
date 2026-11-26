#!/usr/bin/env bash
# Der Rückweg über die Dateien: zu den zwanzig Themen, die mit fux-fuge kommen, je eine drei- und
# eine vierstimmige Fuge (Seed 1) schreiben, aus MusicXML und Akkordzeilen wieder lesen und mit
# `fux-fuge check` prüfen (mit dem Profil fuge, mit dem der Komponist setzt). Je Fuge eine Zeile:
# der Bericht des Komponisten und der der Prüfung.
# Die Prüfung kennt weder die Rollen der Töne noch die Orgelpunkte des Plans (sie schätzt sie),
# die Kosten dürfen darum leicht abweichen; ein Verbot darf nicht brechen.
# Danach die Schlüsse dieser Fugen und, zum Vergleich, die der Fugen Bachs: Schlussakkord und Weg
# der Oberstimme in ihn (tools/bach_schlussakkorde.py).
#
#   tools/bach_fugen_dateien_pruefen.sh > docs/design/bach-fugen/b6/laeufe/dateien-zurueckgelesen.txt
#
# Rückgabewert 1, wenn eine zurückgelesene Fuge ein Verbot bricht.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build --release -p fux-bach --bin fux-fuge 2> /dev/null
fuge="$PWD/target/release/fux-fuge"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
mapfile -t themes < <("$fuge" themes | awk '/^[a-z]/ && $1 != "Pläne" { print $1 }')
one() {
    local stem="$work/$1-$2st"
    "$fuge" fugue --theme "$1" --voices "$2" --seed 1 --out "$stem" > "$stem.bericht" 2> /dev/null
    local status=0
    "$fuge" check "$stem.musicxml" --chords "$stem.akkorde.txt" --profile fuge > "$stem.pruefung" 2>&1 || status=$?
    echo "$1-$2st: Komponist: $(grep -m1 'Verbot(e)' "$stem.bericht") Datei: $(grep -m1 'Verbot(e)' "$stem.pruefung") [Rückgabewert $status]"
}
export -f one
export fuge work
for theme in "${themes[@]}"; do
    for voices in 3 4; do echo "$theme $voices"; done
done | xargs -P "$(nproc)" -L 1 bash -c 'one "$0" "$1"' | sort > "$work/zeilen"
cat "$work/zeilen"
clean=$(grep -c "Datei: 0 Verbot(e)" "$work/zeilen" || true)
same=$(awk -F'Komponist: | Datei: | \\[' '$2 == $3' "$work/zeilen" | wc -l)
echo "Ohne Verbot zurückgelesen: $clean von $(wc -l < "$work/zeilen"); Bericht Wort für Wort gleich: $same."
echo
echo "Die Schlüsse dieser Fugen."
python3 tools/bach_schlussakkorde.py "$work"/*.musicxml
echo
echo "Die Schlüsse der Fugen des Wohltemperierten Klaviers (47: die Datei von I/20 bricht vor dem Schluss ab)."
mapfile -t bach < <(ls references/datasets/bach-fugen/wtc/*.krn | grep -v wtc1f20)
python3 tools/bach_schlussakkorde.py "${bach[@]}"
[ "$clean" = "$(wc -l < "$work/zeilen")" ]
