#!/usr/bin/env bash
# Generates the examples in examples/ (described in examples/README.md). For each example:
# NAME.pdf (LilyPond), NAME.ly (its source), NAME.mid, NAME.musicxml, NAME.mscz (MuseScore,
# converted from the MusicXML file) and NAME.txt (what fux printed: plan, entries, cost, voices).
# The script gives the .ly and .musicxml files the example's title (in MusicXML the work's title
# and the title on the first page); otherwise they are as fux writes them. The mass (11) is a
# book instead of a score, NAME.ly and NAME.pdf, with a MusicXML, MIDI and MuseScore file per
# movement (NAME-1-kyrie.musicxml … NAME-6-agnus.musicxml) and NAME.txt; it keeps the titles fux
# gives it, and its NAME.txt leaves out the search times. The seeds are fixed, so every run
# writes the same settings; .ly, .mid, .musicxml and .txt come out byte for byte the same, while
# PDF and .mscz also hold time stamps and random identifiers (Ghostscript, MuseScore) that change
# from run to run.
#
#   tools/make_examples.sh                     all examples
#   tools/make_examples.sh 03 7                only the examples with these numbers
#   tools/make_examples.sh 11                  only the mass
#   tools/make_examples.sh --nur-notenbild 11  only the mass, from its parts, without a search
#
# fux writes the mass to target/beispiele/, beside NAME.teile/, where it keeps each part as soon
# as it is set, and the script copies the book, the movements and NAME.txt into examples/ (it
# removes those of an earlier run there first, so that it copies only what fux writes now). The
# parts stay there for --nur-notenbild. Like all of target/ they are not in the repository, and
# cargo clean deletes them with target/ (unless CARGO_TARGET_DIR points elsewhere). The mass is
# set with --jobs 4, four parts at once, on any machine: line 2 of its NAME.txt gives the native
# estimate for that many parts at once (K6), which would otherwise follow the threads of the
# machine; the notes are the same for any --jobs.
#
# --nur-notenbild (anywhere among the numbers) sets the mass from these parts with fux mass --resume
# instead of searching it, for a change to the notation alone (LilyPond, MusicXML, MIDI): book and
# movements anew in seconds instead of minutes. Without kept parts the script stops before it builds
# fux or sets any example; the first run is one without the switch. --resume refuses parts set with
# another plan, seed, rule profile or version of the generator (generator= in fux-part, the constant
# fux_core::mass::GENERATOR, which every change after which the same request sets other notes raises
# by hand), and its message ends the script. A change that sets other notes but leaves the version
# as it was goes unnoticed: --resume takes the old parts, and their report still matches
# examples/NAME.txt. So after a change to the generator (rules, tables, search) that may set other
# notes, set the mass without the switch. A resumed run names the folder for each part instead of
# its search time and counts the parts taken from there in its total. The script leaves that out of
# the report, as it does the search times, and goes on only where the report is then
# examples/NAME.txt byte for byte, which stays as it is; otherwise it stops before it changes a file
# of the mass in examples/. Parts the folder lacks, after an interrupted run, fux searches, as
# --resume does. The examples 01-10 have no parts and are set as without the switch.
#
# Needs cargo (builds fux when needed), LilyPond 2.24 or later (another path with FUX_LILYPOND)
# and MuseScore 4 (`mscore`; another command with MSCORE=...). On an idle machine with eight cores
# (5.11.2026) the five-voice Agnus Dei took 13 s, examples 01-10 together 45 s, the mass 128 s,
# from its parts 10 s (fux under a second, the rest LilyPond and MuseScore).
#
# Each seed was chosen from several candidates by the program's cost, its warnings (none), its
# plan (a clear subject, entries at the fifth or octave that overlap, natural underlay, clear
# cadences) and by looking at the score; the comments name the candidates.
set -euo pipefail
cd "$(dirname "$0")/.."

# The numbers asked for, as two digits (3 and 03 both select 03-...), and the switch.
only=()
notation_only=0
for arg in "$@"; do
  if [[ $arg == --nur-notenbild ]]; then
    notation_only=1
  elif [[ $arg =~ ^[0-9]+$ ]]; then
    only+=("$(printf '%02d' "$((10#$arg))")")
  else
    echo "Usage: tools/make_examples.sh [--nur-notenbild] [NUMBER...]," \
      "e.g. tools/make_examples.sh 03 7" >&2
    exit 1
  fi
done

out=examples
kept=target/beispiele
lilypond=${FUX_LILYPOND:-lilypond}
mscore=${MSCORE:-mscore}
fux=${CARGO_TARGET_DIR:-target}/release/fux
# The mass, example 11. With --nur-notenbild it is set from its kept parts; without them the
# script stops here, before it builds fux or sets any example.
mass=11-missa-brevis-ionisch-transponiert-4stimmig
if ((notation_only)) && { ((${#only[@]} == 0)) || [[ " ${only[*]} " == *" ${mass%%-*} "* ]]; } &&
  ! compgen -G "$kept/$mass.teile/*.musicxml" >/dev/null; then
  echo "No parts of $mass in $kept/$mass.teile/: set it once without --nur-notenbild," \
    "which searches it and keeps its parts there." >&2
  exit 1
fi

for tool in cargo "$lilypond" "$mscore"; do
  if ! command -v "$tool" >/dev/null; then
    echo "Missing $tool (LilyPond: FUX_LILYPOND=..., MuseScore: MSCORE=...)." >&2
    exit 1
  fi
done

cargo build --release -p fux-cli
mkdir -p "$out"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
made=()

# example NAME TITLE FUX-ARGUMENTS...: one example; NAME starts with its number.
example() {
  local name=$1 title=$2
  shift 2
  if ((${#only[@]})) && [[ " ${only[*]} " != *" ${name%%-*} "* ]]; then
    return
  fi
  if [[ $title == *[\|\&\\\"\<\>]* ]]; then
    echo "The title of $name must not contain | & \\ \" < or >." >&2
    exit 1
  fi
  local start
  start=$(date +%s%N)
  "$fux" "$@" -f ly,midi,musicxml -o "$out/$name" >"$tmp/fux.txt"
  grep -v '^geschrieben: ' "$tmp/fux.txt" >"$out/$name.txt"
  # fux titles a score after its mode and text in LilyPond ("dorisch – Kyrie I" for 01, 04 and
  # 08 alike) and "Freier Satz 1" or "Kontrapunkt 1" in MusicXML, as the work's title and as the
  # title on its first page, which MuseScore shows.
  sed -i "s|^  title = \".*\"\$|  title = \"$title\"|" "$out/$name.ly"
  sed -i -E -e "s|<work-title>[^<]*</work-title>|<work-title>$title</work-title>|" \
    -e "s|(<credit-type>title</credit-type><credit-words [^>]*>)[^<]*(</credit-words>)|\1$title\2|" \
    "$out/$name.musicxml"
  if ! grep -qxF "  title = \"$title\"" "$out/$name.ly" ||
    ! grep -qF "<work-title>$title</work-title>" "$out/$name.musicxml" ||
    [ "$(grep -cF "<credit-type>title</credit-type>" "$out/$name.musicxml")" != 1 ] ||
    ! grep -qF ">$title</credit-words>" "$out/$name.musicxml"; then
    echo "Could not set the title of $out/$name.ly and $out/$name.musicxml." >&2
    exit 1
  fi
  # As fux renders its PDF.
  if ! "$lilypond" -dno-point-and-click --pdf -o "$out/$name" "$out/$name.ly" \
    >"$tmp/lilypond.log" 2>&1; then
    cat "$tmp/lilypond.log" >&2
    echo "LilyPond could not set $out/$name.ly." >&2
    exit 1
  fi
  if ! QT_QPA_PLATFORM=offscreen "$mscore" -o "$out/$name.mscz" "$out/$name.musicxml" \
    >"$tmp/mscore.log" 2>&1; then
    cat "$tmp/mscore.log" >&2
    echo "MuseScore could not convert $out/$name.musicxml." >&2
    exit 1
  fi
  local ext
  for ext in ly pdf mid musicxml mscz txt; do
    if [ ! -s "$out/$name.$ext" ]; then
      echo "Missing or empty: $out/$name.$ext" >&2
      exit 1
    fi
  done
  local ms=$((($(date +%s%N) - start) / 1000000))
  local cost
  cost=$(grep -m1 -oE 'Kosten [0-9]+' "$out/$name.txt" || true)
  printf '%-50s %s, %s, %d.%d s\n' "$name" "$(head -n1 "$out/$name.txt")" "$cost" \
    $((ms / 1000)) $((ms % 1000 / 100))
  made+=("${name%%-*}")
}

# mass_example NAME FUX-ARGUMENTS...: a whole mass (fux mass); NAME starts with its number. fux
# writes it to $kept, beside NAME.teile/, where it keeps every part as soon as it is set; that
# folder is not part of the example, and a mass fux cannot finish changes nothing in examples/.
# With --nur-notenbild fux takes the parts from there (--resume) instead of searching them.
mass_example() {
  local name=$1
  shift
  if ((${#only[@]})) && [[ " ${only[*]} " != *" ${name%%-*} "* ]]; then
    return
  fi
  local start base=$kept/$name resume=()
  start=$(date +%s%N)
  if ((notation_only)); then
    resume=(--resume)
  fi
  # $kept outlives the run: the book, NAME.txt and the movements of an earlier run go, so that
  # only what fux writes now goes into examples/. NAME.teile/ stays.
  rm -f -- "$base.ly" "$base.txt" "$base"-[1-6]-*.musicxml "$base"-[1-6]-*.mid
  # Each part as it is set, or taken from its file, goes to stderr, since a search takes minutes.
  if ! "$fux" "$@" "${resume[@]}" -f ly,midi,musicxml -o "$base" >"$tmp/fux.txt"; then
    cat "$tmp/fux.txt" >&2
    if ((notation_only)); then
      echo "fux could not set the mass $name from the parts in $base.teile/ (its message" \
        "above); without --nur-notenbild the script searches it." >&2
    else
      echo "fux could not set the whole mass $name." >&2
    fi
    exit 1
  fi
  # fux writes all it printed to NAME.txt, with the seconds each part's search took and the
  # total search time, which change from run to run; with --resume each part taken from its file
  # names the folder instead of its seconds, and the total counts the parts taken from there.
  local folder=${base//./\\.}\\.teile
  sed -E -e 's/^(\[ *[0-9]+\/[0-9]+\] .*: [0-9]+ Takte, Kosten [0-9]+), [0-9]+,[0-9] s$/\1/' \
    -e "s|^(\[ *[0-9]+/[0-9]+\] .*: [0-9]+ Takte, Kosten [0-9]+), aus $folder\$|\1|" \
    -e "s|^(Gesamt: [0-9]+ von [0-9]+ Teil(en)?) \([0-9]+ aus $folder\),|\1,|" \
    -e 's/^(Gesamt: .*), Suchzeit [0-9:]+,/\1,/' "$base.txt" >"$tmp/$name.txt"
  if grep -qE '^\[.* [0-9]+,[0-9] s$|^Gesamt: .*Suchzeit' "$tmp/$name.txt" ||
    grep -qF " aus $base.teile" "$tmp/$name.txt"; then
    echo "Could not leave the search times and the folder of the parts out of $base.txt." >&2
    exit 1
  fi
  if ((notation_only)); then
    # The parts are the example's where fux reports the mass from them as examples/NAME.txt does.
    if ! diff -u "$out/$name.txt" "$tmp/$name.txt" >"$tmp/txt.diff" 2>&1; then
      head -n 40 "$tmp/txt.diff" >&2
      echo "The mass from the parts in $base.teile/ is reported otherwise than in" \
        "$out/$name.txt (above): they are not the example's parts, or fux reports a mass" \
        "otherwise now. The files of the mass in $out/ are as they were; without" \
        "--nur-notenbild the script searches it." >&2
      exit 1
    fi
  else
    cp "$tmp/$name.txt" "$out/$name.txt"
  fi
  cp "$base.ly" "$out/$name.ly"
  # The movements, NAME-1-kyrie … NAME-6-agnus, as many as the mass has; MuseScore converts them
  # in one job.
  local movements=() file job=""
  for file in "$base"-[1-6]-*.musicxml; do
    file=${file##*/}
    file=${file%.musicxml}
    movements+=("$file")
    cp "$kept/$file.musicxml" "$kept/$file.mid" "$out/"
    job+="${job:+, }{\"in\": \"$out/$file.musicxml\", \"out\": \"$out/$file.mscz\"}"
  done
  echo "[$job]" >"$tmp/mscore.json"
  if ! "$lilypond" -dno-point-and-click --pdf -o "$out/$name" "$out/$name.ly" \
    >"$tmp/lilypond.log" 2>&1; then
    cat "$tmp/lilypond.log" >&2
    echo "LilyPond could not set $out/$name.ly." >&2
    exit 1
  fi
  if ! QT_QPA_PLATFORM=offscreen "$mscore" -j "$tmp/mscore.json" >"$tmp/mscore.log" 2>&1; then
    cat "$tmp/mscore.log" >&2
    echo "MuseScore could not convert the movements of $out/$name." >&2
    exit 1
  fi
  local ext
  for file in "$name.ly" "$name.pdf" "$name.txt"; do
    if [ ! -s "$out/$file" ]; then
      echo "Missing or empty: $out/$file" >&2
      exit 1
    fi
  done
  for file in "${movements[@]}"; do
    for ext in musicxml mid mscz; do
      if [ ! -s "$out/$file.$ext" ]; then
        echo "Missing or empty: $out/$file.$ext" >&2
        exit 1
      fi
    done
  done
  local ms=$((($(date +%s%N) - start) / 1000000)) how=""
  if ((notation_only)); then
    how=", from its parts"
  fi
  printf '%-50s %s, %s, %d.%d s%s\n' "$name" "$(head -n1 "$out/$name.txt" | cut -d: -f1)" \
    "$(grep -m1 '^Gesamt: ' "$out/$name.txt" | cut -d' ' -f2-)" $((ms / 1000)) \
    $((ms % 1000 / 100)) "$how"
  made+=("${name%%-*}")
}

# Free counterpoint with Latin text and imitation (Phases 11-13).

# Seeds 1-10 (costs 84, 60, 45, 48, 47, 57, 70, 49, 52, 57); 5 (47) has a clear subject D-F-G-A
# whose first three entries overlap; 3 costs less (45), but its voices enter four and eight bars
# apart.
example 01-kyrie-imitation-dorisch-4stimmig "dorisch – Kyrie I" \
  free --mode dorisch --voices 4 --text kyrie --seed 5
# Seeds 1-10 (costs 30, 26, 69, 35, 24, 26, 35, 26, 35, 25); 10 (25) has the most phrygian
# subject, E-G-F-E-F-E-C around the final with the semitone E-F, enters on E, A and E and ends
# with the major third G sharp; 5 costs one less, but its subject leaps from E to C and its last
# chord has no third.
example 02-christe-imitation-phrygisch-3stimmig "phrygisch – Christe" \
  free --mode phrygisch --voices 3 --text christe --seed 10
# Hypomixolydian, not mixolydian: the soprano's mixolydian octave G4-G5 (FREE-AMBITUS) reaches
# past the top of its system, D5 (VOI-RANGE). In all mixolydian candidates (seeds 1-16, costs
# 306, 248, 182, 283, 235, 220, 379, 227, 185, 214, 199, 195, 279, 194, 224, 215) it sank to B3,
# in 11 of them rose no higher than A4 and in 13 crossed under a lower voice at times (in 8 under
# the alto). Of the hypomixolydian seeds 1-10 (costs 173, 183, 149, 199, 126, 170, 114, 179, 131,
# 155), 5 (126) opens with D-D-E-D-B in the soprano, enters three times on D and twice on G and
# ends on a full chord; 7 costs less (114), but its subject holds G for three bars and moves only
# to F and back, and its voices enter up to eight bars apart; 9 (131) enters four times on D.
example 03-agnus-dei-imitation-hypomixolydisch-5stimmig "hypomixolydisch – Agnus Dei I und II" \
  free --mode hypomixolydisch --voices 5 --text agnus --seed 5
# Subjects from the chant of the Kyriale (Kyrie IX "Cum jubilo"), in the chant's first mode.
# Seeds 1-20 (costs 127, 51, 71, 83, 53, 55, 80, 57, 85, 63, 62, 57, 53, 48, 96, 64, 63, 54, 108,
# 153); 14 (48) costs least: its alto sings the chant's first phrase D-F-G-A in whole notes,
# soprano and tenor answer on A, its second point sets the eleison phrase C-D-F-G-F, answered a
# fourth lower, and it ends without a fifth, with the major third F sharp in the alto and the
# soprano on D5. 2 (51) and 5 (53) open with two entries on the same D, 13 (53) repeats the first
# phrase instead of the eleison phrase.
example 04-kyrie-cum-jubilo-choral-4stimmig "dorisch – Kyrie I über Kyrie IX „Cum jubilo“" \
  free --voices 4 --text kyrie --subject chant --seed 14
# Aeolian seeds 1-8 (costs 70, 49, 77, 82, 72, 97, 84, 77) and hypoaeolian seeds 1-6 (97, 64, 47,
# 92, 31, 47); hypoaeolian 5 costs least.
example 05-benedictus-imitation-hypoaeolisch-4stimmig "hypoäolisch – Benedictus" \
  free --mode hypoäolisch --voices 4 --text benedictus --seed 5
# Seeds 1-10 (costs 11, 6, 6, 8, 6, 7, 8, 15, 8, 6); 6 (7) declaims "Ad te le-vá-" syllable by
# syllable up to the accent and answers on A and D; 2, 3, 5 and 10 cost one less, but 3 enters in
# syncopation half a bar apart, 10 repeats the F of its subject, and 2 and 5 bring their third
# voice six bars later, 2 on the A it began with.
example 06-ad-te-levavi-imitation-dorisch-3stimmig "dorisch – Ad te levavi" \
  free --mode dorisch --voices 3 --text "Ad te le-vá-vi á-ni-mam me-am." --seed 6
# Seeds 1-6 (costs 75, 100, 114, 100, 86, 52); 6 costs least by far (52). Its first point enters
# on F and C, and its first cadence (measure 17) comes in the soprano on "vo-" of "voluntatis";
# the second and the last end sense units.
example 07-gloria-imitation-ionisch-4stimmig "ionisch – Gloria: Et in terra … Glorificamus te" \
  free --mode ionisch --voices 4 --text gloria:1-5 --seed 6
# For comparison with 01: the same text without imitation, as in Phase 12. Seeds 1-10, as for 01
# (costs 44, 34, 54, 33, 95, 47, 33, 29, 53, 45); 4 (33) begins with long notes; 8 costs less (29),
# but its soprano begins "Ký-" with quarter notes in unison with the alto, and its tenor cadences
# in the middle of "eleison"; 7, which costs as much as 4, sings the text only once in the soprano,
# in long melismas.
example 08-kyrie-ohne-imitation-dorisch-4stimmig "dorisch – Kyrie I ohne Imitation" \
  free --mode dorisch --voices 4 --text kyrie --imitation aus --seed 4

# Species counterpoint after Fux on his cantus firmi.

# Seeds 1-10 all cost 4; 4 mixes the elements of the fifth species most.
example 09-fux-gattung5-dorisch-2stimmig "dorisch – 5. Gattung" \
  cp --cf fux:dorian --species 5 --seed 4
# The six cantus with seeds 1-5, the Aeolian one with seeds 6-12 too. In the tenor the Dorian,
# Ionian and Phrygian cantus sound an octave lower than written (the Phrygian down to A2); of the
# others, Aeolian seed 4 costs least (36; Lydian 40-42, Mixolydian 48-50).
example 10-fux-gattung5-cantus-im-tenor-aeolisch-4stimmig "äolisch – 5. Gattung, Cantus im Tenor" \
  cp --cf fux:aeolian --voices 4 --cf-voice T --species 5 --voice S --seed 4

# A whole mass (Phase 14).

# A Missa brevis in the transposed Ionian mode, on F with B flat, as Palestrina's of 1570. Seeds
# 1-5 all set 26 of 26 parts without a warning (costs 3133, 3149, 3139, 3249, 2876); 2 has the
# most varied cadences: from its second part on, the Credo's parts end on F and C in turn, and only
# the end of the Sanctus cadences more than three times in a row on one degree (F). 5 costs least,
# but cadences on C eight times in nine in the middle of its Credo; 1 and 3 open the Kyrie on B
# flat; 4 costs most.
mass_example "$mass" \
  mass --mode ionisch-transponiert --seed 2 --details --jobs 4

for number in "${only[@]}"; do
  if [[ " ${made[*]} " != *" $number "* ]]; then
    echo "No example $number." >&2
    exit 1
  fi
done
