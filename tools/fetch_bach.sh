#!/bin/sh
# Loads the data of the Bach feasibility study into references/datasets/bach-fugen
# (see the README there for sources and rights). Run from the root of the repository.
set -eu
D=references/datasets/bach-fugen
mkdir -p "$D/wtc" "$D/wir" "$D/algomus"

for b in 1 2; do
  for n in $(seq -w 1 24); do
    curl -fsS -m 60 -o "$D/wtc/wtc${b}f$n.krn" \
      "https://raw.githubusercontent.com/humdrum-tools/bach-wtc-fugues/master/kern/wtc${b}f$n.krn"
  done
done

for p in I/19 I/22 II/07 II/11 II/16 II/23 II/24; do
  curl -fsS -m 60 -o "$D/wir/${p%/*}_${p#*/}_fugue.txt" \
    "https://raw.githubusercontent.com/MarkGotham/When-in-Rome/master/Corpus/Keyboard_Other/Bach,_Johann_Sebastian/The_Well-Tempered_Clavier_${p%/*}/${p#*/}_fugue/analysis.txt"
done

mkdir -p "$D/kdf" "$D/opfer"
for n in 001 002 003 004 005 006 007 008 009 010 011 012 013 014 015 016a 016b 018a 018b 019; do
  curl -fsS -m 60 -o "$D/kdf/artfugue-$n.krn" \
    "https://raw.githubusercontent.com/craigsapp/art-of-the-fugue/main/kern/artfugue-$n.krn"
done
for n in 001 002 013a 013b 013c 013d; do
  curl -fsS -m 60 -o "$D/opfer/offering-$n.krn" \
    "https://raw.githubusercontent.com/craigsapp/bach-musical-offering/main/kern/offering-$n.krn"
done

# There is no file for I/11 (BWV 856).
for n in $(seq 1 24); do
  f=$(printf '%02d-bwv%d-ref.dez' "$n" $((845 + n)))
  curl -fsS -m 60 -o "$D/algomus/$f" \
    "https://gitlab.com/algomus.fr/algomus-data/-/raw/master/fugues/bach-wtc-i/$f" 2>/dev/null ||
    rm -f "$D/algomus/$f"
done

# Phase B2: the analyses of the 371 chorales and of the 24 preludes of WTC I (When in Rome).
mkdir -p "$D/wir/choraele" "$D/wir/praeludien"
W=https://raw.githubusercontent.com/MarkGotham/When-in-Rome/master/Corpus
for n in $(seq -w 1 371); do
  [ -s "$D/wir/choraele/$n.txt" ] ||
    curl -fsS -m 60 -o "$D/wir/choraele/$n.txt" \
      "$W/Early_Choral/Bach,_Johann_Sebastian/Chorales/$n/analysis.txt"
done
for n in $(seq -w 1 24); do
  [ -s "$D/wir/praeludien/$n.txt" ] ||
    curl -fsS -m 60 -o "$D/wir/praeludien/$n.txt" \
      "$W/Keyboard_Other/Bach,_Johann_Sebastian/The_Well-Tempered_Clavier_I/$n/analysis.txt"
done
