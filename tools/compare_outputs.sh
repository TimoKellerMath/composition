#!/usr/bin/env bash
# Checks that a change leaves the outputs of fux as they were: sets a fixed set of requests with
# two builds and compares every file they write and every report byte for byte, apart from the
# seconds a search took. The requests: free settings with and without text and imitation, cantus
# firmi, all species, `fux check` of every setting written, render, book and text, examples 01-10
# (set anew, and the committed ones checked and rendered), plans of `fux mass`, two small masses
# (one of them resumed) and a Benedictus, and the answers of the WebAssembly module
# (tools/web_answers.mjs, with Deno or else Node.js).
#
#   tools/compare_outputs.sh base [REV]   builds REV (default HEAD) from git, without the working
#                                         tree's changes, and sets the requests into compare/REV
#   tools/compare_outputs.sh check REV    builds the working tree, sets the requests into
#                                         compare/work and compares them with compare/REV
#
# compare/ lies outside target/, so cargo clean keeps it; it is not part of the repository
# (another place with FUX_COMPARE=...). Each side also writes times.txt with the wall time of
# every request and, where perf can count them, its instructions, which vary far less with the
# load of the machine; times.txt is not compared. Needs cargo, the target wasm32-unknown-unknown
# and Deno or Node.js; a side takes about 15 minutes on eight cores.
set -euo pipefail
cd "$(dirname "$0")/.."
repo=$PWD
compare=$(realpath -m "${FUX_COMPARE:-$repo/compare}")
wasm=wasm32-unknown-unknown
target=$(realpath -m "${CARGO_TARGET_DIR:-target}")

usage() {
  echo "Usage: tools/compare_outputs.sh base [REV] | check REV" >&2
  exit 1
}

# build_rev REV DIR: fux, fux_web.wasm and fux.js of REV into DIR/bin, built from an export of
# REV in DIR/src with a target directory of their own; both are removed afterwards. The module is
# built as tools/build_web.sh of REV builds it: with the profile wasm where REV's Cargo.toml has
# one, with release in older revisions.
build_rev() {
  local rev=$1 dir=$2
  rm -rf "$dir/src" "$dir/build"
  mkdir -p "$dir/src" "$dir/bin"
  git archive "$rev" | tar -x -C "$dir/src"
  local build=(cargo build --locked --manifest-path "$dir/src/Cargo.toml" --target-dir "$dir/build")
  local profile=release
  if grep -q '^\[profile\.wasm\]' "$dir/src/Cargo.toml"; then
    profile=wasm
  fi
  "${build[@]}" --release -p fux-cli
  "${build[@]}" --profile "$profile" -p fux-web --target "$wasm"
  cp "$dir/build/release/fux" "$dir/build/$wasm/$profile/fux_web.wasm" "$dir/src/web/fux.js" \
    "$dir/bin/"
  rm -rf "$dir/src" "$dir/build"
}

# build_work DIR: the same of the working tree (the module as tools/build_web.sh builds it).
build_work() {
  local dir=$1
  mkdir -p "$dir/bin"
  cargo build --release -p fux-cli --target-dir "$target"
  cargo build --profile wasm -p fux-web --target "$wasm" --target-dir target/web-build
  cp "$target/release/fux" "target/web-build/$wasm/wasm/fux_web.wasm" web/fux.js "$dir/bin/"
}

# set_requests DIR: sets every request with the build in DIR/bin into DIR/out, with relative
# output paths, so that the reports' „geschrieben:“ lines agree between the sides.
set_requests() {
  local dir=$1
  local fux=$dir/bin/fux out=$dir/out
  rm -rf "$out"
  mkdir -p "$out"
  printf '%-10s %8s %16s  %s\n' request seconds instructions arguments >"$dir/times.txt"
  local perf=0
  if command -v perf >/dev/null && perf stat -x, -e instructions:u -o /dev/null -- true \
    2>/dev/null; then
    perf=1
  fi

  # run NAME ARGS...: fux ARGS in $out, its report in NAME.stdout and NAME.stderr, its exit code
  # at the end of NAME.stdout.
  run() {
    local name=$1
    shift
    local start status=0 instructions=-
    start=$(date +%s%N)
    if ((perf)); then
      # perf returns 0 for a program a signal ended; the shell in between keeps its 128 + signal.
      (cd "$out" && perf stat -x, -e instructions:u -o "$dir/perf.csv" -- \
        bash -c '"$@"; echo $? >"$0"' "$dir/status" "$fux" "$@" >"$name.stdout" 2>"$name.stderr") ||
        true
      status=$(cat "$dir/status")
      instructions=$(grep -m1 instructions "$dir/perf.csv" | cut -d, -f1 || true)
    else
      (cd "$out" && "$fux" "$@" >"$name.stdout" 2>"$name.stderr") || status=$?
    fi
    echo "exit $status" >>"$out/$name.stdout"
    local ms=$((($(date +%s%N) - start) / 1000000))
    printf '%-10s %4d.%03d %16s  %s\n' "$name" $((ms / 1000)) $((ms % 1000)) "$instructions" \
      "$*" >>"$dir/times.txt"
  }

  # setting NAME ARGS...: a setting with its details, written as LilyPond, MusicXML and MIDI,
  # then checked again from its MusicXML file.
  setting() {
    local name=$1
    shift
    run "$name" "$@" --details -f ly,musicxml,midi -o "$name"
    if [ -f "$out/$name.musicxml" ]; then
      run "$name-check" check --musicxml "$name.musicxml" --mark -f ly,musicxml -o "$name-check"
    fi
  }

  setting s01 free --text none --voices 2 --mode dorian --seed 1
  setting s02 free --text none --voices 3 --mode phrygian --seed 2
  setting s03 free --text none --voices 4 --mode mixolydian --seed 3 --length 14
  setting s04 free --text kyrie --voices 2 --seed 1
  setting s05 free --text kyrie --voices 3 --imitation aus --seed 1 --mode aeolian
  setting s06 free --text kyrie --voices 3 --seed 1
  setting s07 free --text kyrie --voices 4 --seed 2 --mode ionian-transposed
  setting s08 free --text agnus --voices 4 --seed 1 --mode hypodorian
  setting s09 free --text christe --voices 4 --subject chant --seed 1
  setting s10 free --text "Ad te le-vá-vi á-ni-mam me-am." --voices 3 --seed 3 --mode lydian
  setting s11 free --text gloria:1-5 --voices 4 --imitation aus --seed 1
  setting s12 free --text sanctus --voices 3 --seed 1 --mode mixolydian
  setting s13 free --text benedictus --voices 4 --seed 4 --mode phrygian
  setting s14 free --text kyrie --voices 3 --cadences 6:A --imitation aus --seed 1
  setting s15 free --text kyrie --voices 5 --seed 1
  setting s16 free --text credo:1-4 --voices 4 --seed 2 --mode dorian-transposed
  setting s17 free --text kyrie --voices 3 --seed 5 --repeat frei
  setting s18 free --text gloria:1-5 --voices 3 --seed 2 --repeat keine --imitation aus
  setting s19 free --text kyrie --voices 3 --imitation aus --seed 1 --rule IMI-HELD=V
  setting s20 free --text kyrie --voices 4 --seed 3 --rule IMI-HELD.struck=P2
  setting s21 --lang en free --text sanctus --voices 4 --seed 3 --mode ionian
  setting s22 free --text credo:8-14 --subject chant --seed 3 --beam 40
  setting s23 free --text kyrie --subject chant --seed 2 --beam 40
  setting s24 free --text gloria:1-5 --seed 5 --beam 40
  setting s25 free --text none --voices 3 --seed 4
  # The loads the speed of the search is measured on (w1 and w6; w2 is the Benedictus below).
  setting w1 free --text gloria:1-5 --voices 4 --seed 1
  setting w6 free --text kyrie --voices 3 --seed 2
  setting c01 cp --mode dorian --species 3 --seed 1
  setting c02 cp --mode phrygian --species 5 --seed 2
  setting c03 cp --mode mixolydian --species 4 --voices 3 --seed 1
  setting c04 cp --cf fux:aeolian --voices 4 --cf-voice T --species 5 --voice S --seed 4
  setting c05 cp --mode aeolian --species 2 --seed 3
  setting c06 cp --mode dorian --species 0 --voices 3 --text kyrie --seed 1
  setting c07 cp --cf "$repo/tests/fixtures/chant/pange-lingua.gabc" --species 1 --voices 3 --seed 1
  setting c08 cp --cf fux:dorian --species 1 --seed 1
  setting c09 cp --cf fux:dorian --species 2 --seed 1
  setting c10 cp --cf fux:dorian --species 4 --seed 1
  run f01 cf --mode dorian --seed 1 -f ly,musicxml,midi -o f01
  run f02 cf --seed 4 --count 2 --details -f ly,midi,musicxml -o f02
  run f03 cf --mode lydian --length 10 --count 3 --seed 4 -f ly,musicxml,midi -o f03

  run x01 render --example 5 -f ly,musicxml,midi -o x01
  run x02 check --example 5 --mark -f ly -o x02
  run x03 book --modes dorisch --species 1,2 --exercises --seed 1 -f ly,musicxml,midi -o x03
  run x04 free --text kyrie --voices 3 --seed 1 --rule IMI-HELD=aus --rule IMI-SILENCE=aus \
    --details -f ly,musicxml,midi -o x04
  run x05 check --musicxml s06.musicxml --rule IMI-HELD=aus --rule IMI-SILENCE=aus
  run x06 --lang en check --musicxml s20.musicxml --rule IMI-HELD.struck=P2
  run t01 text kyrie
  run t02 text credo:1-4
  run t03 text --list
  run t04 --lang en text gloria:1-5 --eleison 3

  # Examples 01-10 as tools/make_examples.sh sets them (without its titles), and the committed
  # ones checked and rendered.
  run e01 free --mode dorisch --voices 4 --text kyrie --seed 5 -f ly,midi,musicxml -o e01
  run e02 free --mode phrygisch --voices 3 --text christe --seed 10 -f ly,midi,musicxml -o e02
  run e03 free --mode hypomixolydisch --voices 5 --text agnus --seed 5 -f ly,midi,musicxml -o e03
  run e04 free --voices 4 --text kyrie --subject chant --seed 14 -f ly,midi,musicxml -o e04
  run e05 free --mode hypoäolisch --voices 4 --text benedictus --seed 5 -f ly,midi,musicxml -o e05
  run e06 free --mode dorisch --voices 3 --text "Ad te le-vá-vi á-ni-mam me-am." --seed 6 \
    -f ly,midi,musicxml -o e06
  run e07 free --mode ionisch --voices 4 --text gloria:1-5 --seed 6 -f ly,midi,musicxml -o e07
  run e08 free --mode dorisch --voices 4 --text kyrie --imitation aus --seed 4 \
    -f ly,midi,musicxml -o e08
  run e09 cp --cf fux:dorian --species 5 --seed 4 -f ly,midi,musicxml -o e09
  run e10 cp --cf fux:aeolian --voices 4 --cf-voice T --species 5 --voice S --seed 4 \
    -f ly,midi,musicxml -o e10
  local file name
  for file in "$repo"/examples/{01,02,03,04,05,06,07,08,09,10}-*.musicxml; do
    name=k${file##*/examples/}
    name=${name%%-*}
    run "$name-check" check --musicxml "$file"
    run "$name-check-en" --lang en check --musicxml "$file" --imitation erkennen
    run "$name-mark" check --musicxml "$file" --mark -f ly,musicxml,midi -o "$name-mark"
    run "$name-render" render --musicxml "$file" -f ly,musicxml,midi -o "$name-render"
    run "$name-orig" render --musicxml "$file" --clefs original -f ly,musicxml -o "$name-orig"
  done

  # Masses: plans, two small masses, the first one resumed from the parts it kept, and the
  # Benedictus of a Missa brevis (w2). Each runs with --jobs 4 on any machine, as
  # tools/make_examples.sh sets example 11: line 2 of a report of fux mass gives the native
  # estimate for that many parts at once (K6), which would otherwise follow the threads of the
  # machine. A fux before K6, the base of an older revision, has no --jobs and gets none.
  local jobs=()
  if [[ $("$fux" mass --help 2>/dev/null) == *--jobs* ]]; then
    jobs=(--jobs 4)
  fi
  run m01 mass --plan --seed 1 "${jobs[@]}"
  run m02 mass --plan --details --seed 7 --preset normal --voices 5 "${jobs[@]}"
  run m03 mass --plan --seed 3 --mode phrygisch --movements kyrie,credo "${jobs[@]}"
  run m04 --lang en mass --plan --seed 1 --motif an "${jobs[@]}"
  mkdir -p "$out/m05" "$out/m06" "$out/w2"
  run m05/kyrie mass --movements kyrie --voices 3 --beam 40 --seed 1 --details \
    -f ly,musicxml,midi -o m05/messe "${jobs[@]}"
  mkdir -p "$out/m05/first"
  cp -r "$out"/m05/messe* "$out/m05/first/"
  run m05/resume mass --movements kyrie --voices 3 --beam 40 --seed 1 --details --resume \
    -f ly,musicxml,midi -o m05/messe "${jobs[@]}"
  run m06/agnus mass --movements agnus --voices 3 --beam 40 --seed 2 --lengths natürlich \
    -f musicxml,midi -o m06/messe "${jobs[@]}"
  run m07 check --musicxml m05/messe.teile/1-02-christe.musicxml
  run m08 check --musicxml m05/messe-1-kyrie.musicxml
  run w2/benedictus mass --movements benedictus --seed 1 -f ly,midi,musicxml -o w2/messe \
    "${jobs[@]}"

  # The answers of the WebAssembly module.
  local web=(deno run --allow-read --allow-write)
  if ! command -v deno >/dev/null; then
    web=(node)
  fi
  local start
  start=$(date +%s%N)
  DENO_DIR=${DENO_DIR:-$compare/.deno} "${web[@]}" tools/web_answers.mjs "$dir/bin/fux_web.wasm" \
    "$dir/bin/fux.js" "$out/web"
  local ms=$((($(date +%s%N) - start) / 1000000))
  printf '%-10s %4d.%03d %16s  %s\n' web $((ms / 1000)) $((ms % 1000)) - \
    "tools/web_answers.mjs" >>"$dir/times.txt"

  # The seconds of a search and what is left of a mass change from run to run.
  find "$out" -type f \( -name '*.stdout' -o -name '*.stderr' -o -name '*.txt' \) -exec \
    sed -E -i -e 's/[0-9]+,[0-9] s\b/T s/g' -e 's/[0-9]+\.[0-9] s\b/T s/g' \
    -e 's/(Suchzeit|search time) [0-9:]+/\1 T/g' -e 's/(noch etwa|about) [0-9]+:[0-9]{2}/\1 T/g' {} +
  rm -f "$dir/perf.csv" "$dir/status"
}

case ${1:-} in
  base)
    (($# <= 2)) || usage
    rev=$(git rev-parse --short "${2:-HEAD}^{commit}")
    dir=$compare/$rev
    build_rev "$rev" "$dir"
    set_requests "$dir"
    echo "Base $rev: $(find "$dir/out" -type f | wc -l) files in $dir/out"
    ;;
  check)
    (($# == 2)) || usage
    rev=$(git rev-parse --short "$2^{commit}")
    base=$compare/$rev
    if [ ! -d "$base/out" ]; then
      echo "No base for $rev: tools/compare_outputs.sh base $rev" >&2
      exit 1
    fi
    dir=$compare/work
    build_work "$dir"
    set_requests "$dir"
    files=$(find "$base/out" -type f | wc -l)
    # PDF files and MuseScore's hold time stamps; no request writes them.
    if diff -r -x '*.pdf' -x '*.mscz' "$base/out" "$dir/out" >"$dir/diff.txt"; then
      echo "All $files files as in $rev."
    else
      echo "Differences from $rev ($files files there), in $dir/diff.txt:"
      diff -rq -x '*.pdf' -x '*.mscz' "$base/out" "$dir/out" | sed "s|$compare/||g"
      exit 1
    fi
    ;;
  *)
    usage
    ;;
esac
