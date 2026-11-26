#!/usr/bin/env python3
"""Statistics over Fux's examples in tests/fixtures/fux/gradus-*st.toml.

Usage (from the repository root):
    python3 tools/corpus_stats.py [FIXTURE_DIR]

Prints the facts the rule catalogue (docs/regeln.md) cites as "Korpus":
accidentals by mode and position, how second-species counterpoints begin,
leaps of a sixth or more, the measure rhythms of the fifth species, dissonance
figures, ranges per clef, and the texture of all examples (parallels, hidden
parallels, motion, sonorities, spacing, voice crossing, first, penultimate and
final sonority). Melodic counts cover
counterpoint voices only; texture counts cover all voices.
"""

import collections
import itertools
import pathlib
import re
import sys
import tomllib

STEPS = "cdefgab"
SEMITONES = dict(c=0, d=2, e=4, f=5, g=7, a=9, b=11)
TOKEN = re.compile(r"(?:(?P<step>[a-g])(?P<acc>is|es|s)?(?P<oct>[',]*)|r)(?P<dur>\d+)(?P<dot>\.)?(?P<tie>~)?")
MODES = {"D": "dorisch", "E": "phrygisch", "F": "lydisch", "G": "mixolydisch", "A": "äolisch", "C": "ionisch"}


def measures(music: str) -> list[list[dict]]:
    """Split a LilyPond part into measures of note dicts (rests have step None)."""
    result = []
    for bar in music.split("|"):
        notes = []
        for tok in bar.split():
            m = TOKEN.fullmatch(tok)
            if not m:
                sys.exit(f"unknown token {tok!r}")
            note = {"step": m["step"], "dur": int(m["dur"]), "dot": bool(m["dot"]), "tie": bool(m["tie"])}
            if m["step"]:
                octave = 3 + m["oct"].count("'") - m["oct"].count(",")
                alter = {"is": 1, "es": -1, "s": -1, None: 0}[m["acc"]]
                note["acc"] = m["acc"]
                note["midi"] = 12 * (octave + 1) + SEMITONES[m["step"]] + alter
                note["diatonic"] = 7 * octave + STEPS.index(m["step"])
            notes.append(note)
        result.append(notes)
    return result


def rhythm(bar: list[dict], tied_in: bool) -> str:
    """Rhythm of one measure, e.g. '⁀2 4 4' (tied in) or '4 4 2⁀' (tied over)."""
    names = []
    for i, n in enumerate(bar):
        name = ("r" if n["step"] is None else "") + str(n["dur"]) + ("." if n["dot"] else "")
        if i == 0 and tied_in:
            name = "⁀" + name
        if n["tie"]:
            name += "⁀"
        names.append(name)
    return " ".join(names)


def main() -> None:
    fixtures = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "tests/fixtures/fux")
    accidentals = collections.Counter()
    starts = collections.Counter()
    leaps = collections.Counter()
    patterns = collections.Counter()
    ranges: dict[str, list[int]] = collections.defaultdict(list)
    for voices in (2, 3, 4):
        for ex in tomllib.loads((fixtures / f"gradus-{voices}st.toml").read_text(encoding="utf-8"))["example"]:
            for index, part in enumerate(ex["parts"], 1):
                bars = measures(part)
                clef = ex["clefs"][index - 1]
                ranges[clef] += [n["diatonic"] for bar in bars for n in bar if n["step"]]
                if index == ex["cf_part"]:
                    continue
                last = len(bars) - 1
                onsets, tied_in = [], False
                for b, bar in enumerate(bars):
                    if voices == 2 and ex["species"] == [5] and 0 < b < last:
                        patterns[rhythm(bar, tied_in)] += 1
                    for n in bar:
                        if n["step"] and n["acc"]:
                            where = {last: "Schlusstakt", last - 1: "vorletzter Takt"}.get(b, "innen")
                            name = n["step"].upper() + {"is": "♯", "es": "♭", "s": "♭"}[n["acc"]]
                            accidentals[(voices, MODES[ex["final"]], name, where)] += 1
                        if n["step"] and not tied_in:
                            onsets.append(n)
                        tied_in = n["tie"]
                if voices == 2 and ex["species"] == [2]:
                    starts["mit halber Pause" if bars[0][0]["step"] is None else "ohne Pause"] += 1
                for a, b in zip(onsets, onsets[1:]):
                    steps = abs(b["diatonic"] - a["diatonic"])
                    if steps >= 5:
                        name = {5: "Sexte", 6: "Septime", 7: "Oktave"}.get(steps, f"{steps + 1}. Stufe")
                        semis = abs(b["midi"] - a["midi"])
                        direction = "auf" if b["midi"] > a["midi"] else "ab"
                        leaps[(voices, "+".join(map(str, ex["species"])), f"{name} ({semis} HT) {direction}")] += 1

    print("## Akzidentien in Gegenstimmen (Stimmen, Modus, Ton, Position): Anzahl")
    for key, count in sorted(accidentals.items()):
        print(f"{key}: {count}")
    print("\n## Beginn der 2. Gattung, zweistimmig")
    for key, count in starts.items():
        print(f"{key}: {count}")
    print("\n## Sprünge ab der Sexte in Gegenstimmen (Stimmen, Gattung, Intervall): Anzahl")
    for key, count in sorted(leaps.items()):
        print(f"{key}: {count}")
    print("\n## Taktrhythmen der 5. Gattung, zweistimmig, ohne ersten und letzten Takt (⁀ = Bindung)")
    for key, count in patterns.most_common():
        print(f"{count:3d}  {key}")
    print("\n## 5. Gattung (G5-RHYTHM, G5-PASSING, G5-EIGHTHS, G5-ORNAMENTED-RES, G5-VARIETY, MEL-VARIETY)")
    for key, count in sorted(fifth_species_stats(fixtures).items(), key=lambda kv: tuple(map(str, kv[0]))):
        print(f"{count:3d}  {key}")
    figures, suspensions = two_voice_dissonances(fixtures)
    print("\n## Dissonanzen der 3. Gattung, zweistimmig, gegen den CF (Viertel 2–4)")
    for key, count in figures.most_common():
        print(f"{count:3d}  {key}")
    print("\n## Vorhalte der 4. und 5. Gattung, zweistimmig (Lage der Gegenstimme, Dissonanz–Auflösung)")
    print("(Intervalle oktavreduziert; Auflösung = nächster Ton im Takt, bei Verzierung ggf. ein Zwischenton)")
    for key, count in sorted(suspensions.items()):
        print(f"{count:3d}  {key}")
    print("\n## Sekunden und Septimen zwischen Oberstimmen auf der Thesis (Stimmen, Gattung, Intervall)")
    for key, figs in sorted(upper_voice_dissonances(fixtures).items()):
        print(f"{len(figs):3d}  {key}: {', '.join(figs[:6])}{' …' if len(figs) > 6 else ''}")
    print("\n## Satzbild aller Stimmen (Stimmen, Merkmal, …): Anzahl")
    print("(G1 = Beispiele der 1. Gattung; Bewegungen jeweils zwischen aufeinanderfolgenden Anschlägen)")
    for key, count in sorted(texture_stats(fixtures).items(), key=lambda kv: tuple(map(str, kv[0]))):
        print(f"{key}: {count}")
    print("\n## Melodische Verbote in allen Stimmen; Wiederholungen und Wechselnoten in der Gattungsstimme")
    for key, figs in sorted(melodic_checks(fixtures).items()):
        print(f"{len(figs):3d}  {key}: {', '.join(sorted(set(figs))[:8])}")
    print("\n## Wörtlich wiederholte Tonfolgen in Gegenstimmen (MEL-MOTIF; Stimmen, Gattung, Länge)")
    print("(Wiederholung mindestens zwei Töne versetzt; Haltebögen zählen als ein Ton)")
    for key, figs in sorted(motif_repeats(fixtures).items()):
        print(f"{len(figs):3d}  {key}: {', '.join(sorted(set(figs))[:8])}")
    print("\n## Lage je Originalschlüssel: Anteil der Töne nach Stufen über (+) oder unter (−) der Mittellinie")
    print("(±4 = oberste/unterste Linie, ±5 = eine Stufe außerhalb des Systems; Gegenstimmen und Cantus)")
    for key, shares in register_stats(fixtures).items():
        print(f"{key}: " + " ".join(f"{d:+d}:{share:.1f}" for d, share in shares))
    print("\n## Tonumfang je Originalschlüssel, alle Stimmen (Stufen relativ zum Liniensystem)")
    for clef, steps in sorted(ranges.items()):
        bottom = STAFF_BOTTOM_LINE[clef]
        lo, hi = min(steps), max(steps)
        print(
            f"{clef}: {sci(lo)}–{sci(hi)}; Liniensystem {sci(bottom)}–{sci(bottom + 8)}; "
            f"unten {bottom - lo:+d}, oben {hi - bottom - 8:+d} Stufen über das System hinaus"
        )


def register_stats(fixtures: pathlib.Path) -> dict[str, list[tuple[int, float]]]:
    """Per clef and for all counterpoints: percent of notes by steps from the staff's middle line.

    VOI-RANGE cites these shares (a step below the staff is rarer than a step above).
    """
    counts: dict[str, collections.Counter] = collections.defaultdict(collections.Counter)
    for voices in (2, 3, 4):
        for ex in tomllib.loads((fixtures / f"gradus-{voices}st.toml").read_text(encoding="utf-8"))["example"]:
            for k, part in enumerate(ex["parts"]):
                clef = ex["clefs"][k]
                if clef not in STAFF_BOTTOM_LINE:
                    continue
                middle = STAFF_BOTTOM_LINE[clef] + 4
                role = "Cantus" if k + 1 == ex["cf_part"] else "Gegenstimmen"
                for bar in measures(part):
                    for n in bar:
                        if n["step"]:
                            d = max(-6, min(6, n["diatonic"] - middle))
                            counts[clef][d] += 1
                            counts[role][d] += 1
    result = {}
    for key in sorted(counts):
        total = sum(counts[key].values())
        result[key] = [(d, 100 * counts[key][d] / total) for d in range(-6, 7)]
    return result


CONSONANT = {(0, 0), (2, 3), (2, 4), (4, 7), (5, 8), (5, 9)}  # (steps mod 7, semitones mod 12)


def soundings(part: str) -> list[dict]:
    """Notes with start and end in eighths, tied notes merged."""
    out, t, tied = [], 0, False
    for bar in measures(part):
        for n in bar:
            d = int(8 // n["dur"] * (1.5 if n["dot"] else 1))
            if n["step"] is None:
                tied = False
            elif tied and out and out[-1]["midi"] == n["midi"]:
                out[-1]["end"] = t + d
            else:
                out.append(dict(n, start=t, end=t + d))
            if n["step"]:
                tied = n["tie"]
            t += d
    return out


def fifth_species_stats(fixtures: pathlib.Path) -> collections.Counter:
    """Measure rhythms by place, dissonances, suspensions and variety of the fifth species.
    Dissonances count against the cantus firmus, in two voices only (except the rhythms)."""
    c = collections.Counter()
    for voices in (2, 3, 4):
        for ex in tomllib.loads((fixtures / f"gradus-{voices}st.toml").read_text(encoding="utf-8"))["example"]:
            if 5 not in ex["species"]:
                continue
            cf = soundings(ex["parts"][ex["cf_part"] - 1])
            for index, part in enumerate(ex["parts"], 1):
                bars = measures(part)
                if index == ex["cf_part"] or not any(n["dur"] in (4, 8) for b in bars for n in b):
                    continue
                # Rhythms by place.
                tied, last, entry = False, len(bars) - 1, next(i for i, b in enumerate(bars) if any(n["step"] for n in b))
                rhythms = []
                for i, bar in enumerate(bars):
                    r = rhythm(bar, tied)
                    tied = bar[-1]["tie"]
                    rhythms.append(r)
                    place = "Schluss" if i == last else "Anfang" if i == entry else "vorletzter" if i == last - 1 else "innen"
                    c[("Rhythmus", voices, place, r)] += 1
                if voices != 2:
                    continue
                inner = rhythms[1:-2]
                for a, b in zip(inner, inner[1:]):
                    c[("gleicher Rhythmus wie der Vortakt", "ja" if a == b else "nein")] += 1
                line = soundings(part)
                at = lambda t: next(m for m in cf if m["start"] <= t < m["end"])
                for i, n in enumerate(line):
                    prev = line[i - 1] if i and line[i - 1]["end"] == n["start"] else None
                    nxt = line[i + 1] if i + 1 < len(line) and line[i + 1]["start"] == n["end"] else None
                    if n["end"] - n["start"] == 2 and prev and i >= 2:
                        before = line[i - 2]
                        xyx = before["midi"] == n["midi"] and prev["midi"] != n["midi"]
                        c[("Viertel kehrt zum Ton zwei Noten davor zurück", "ja" if xyx else "nein")] += 1
                    if prev and prev["end"] - prev["start"] == 2 and n["end"] - n["start"] >= 4:
                        c[("Viertel, dann Halbe", "gleicher Ton" if prev["midi"] == n["midi"] else "anderer Ton")] += 1
                    if n["start"] % 8 and n["start"] < n["end"] and not is_consonant(n, at(n["start"])):
                        step = lambda a, b: b["diatonic"] - a["diatonic"]
                        if prev and nxt and abs(step(prev, n)) == 1 and abs(step(n, nxt)) == 1:
                            fig = "Durchgang" if step(prev, n) == step(n, nxt) else ("untere Wechselnote" if step(prev, n) < 0 else "obere Wechselnote")
                        elif prev and nxt and step(prev, n) == -1 and step(n, nxt) == -2:
                            fig = "Cambiata"
                        else:
                            fig = "anderes"
                        value = {1: "Achtel", 2: "Viertel", 4: "Halbe", 6: "punktierte Halbe"}.get(n["end"] - n["start"], "Synkope")
                        beat = "auf dem Viertelschlag" if n["start"] % 2 == 0 else "zwischen zwei Vierteln"
                        c[("dissonant angeschlagen", value, beat, fig)] += 1
                    downbeat = (n["start"] // 8 + 1) * 8
                    if n["start"] % 8 and n["end"] > downbeat and not is_consonant(n, at(downbeat)):
                        target = downbeat + 4
                        held = "Viertel" if n["end"] == downbeat + 2 else "Halbe"
                        how = "nicht aufgelöst"
                        for m in line[i + 1:]:
                            if m["start"] > target:
                                break
                            if m["diatonic"] - n["diatonic"] == -1 and is_consonant(m, at(m["start"])) and m["start"] in (n["end"], target):
                                how = "direkt" if m["start"] == n["end"] else "verziert auf dem 3. Viertel"
                                break
                        c[("Vorhalt gehalten als", held, how)] += 1
    return c


def motif_repeats(fixtures: pathlib.Path) -> dict[tuple, list[str]]:
    """Literal repetitions of 3 to 5 notes in counterpoint voices, as MEL-MOTIF counts them: the
    repeated figure starts at least two notes after an earlier occurrence."""
    found: dict[tuple, list[str]] = collections.defaultdict(list)
    for voices in (2, 3, 4):
        for ex in tomllib.loads((fixtures / f"gradus-{voices}st.toml").read_text(encoding="utf-8"))["example"]:
            species = "+".join(map(str, ex["species"]))
            for index, part in enumerate(ex["parts"], 1):
                if index == ex["cf_part"]:
                    continue
                pitches, tied = [], False
                for bar in measures(part):
                    for n in bar:
                        if n["step"] is None:
                            tied = False
                            continue
                        if not (tied and pitches and pitches[-1] == n["midi"]):
                            pitches.append(n["midi"])
                        tied = n["tie"]
                for length in (3, 4, 5):
                    for start in range(2, len(pitches) - length + 1):
                        figure = pitches[start:start + length]
                        if any(pitches[i:i + length] == figure for i in range(start - 1)):
                            found[(voices, species, length)].append(ex["figure"])
    return found


def is_consonant(a: dict, b: dict) -> bool:
    low, high = sorted((a, b), key=lambda n: (n["diatonic"], n["midi"]))
    return ((high["diatonic"] - low["diatonic"]) % 7, (high["midi"] - low["midi"]) % 12) in CONSONANT


def generic(a: dict, b: dict) -> int:
    """Generic interval number between two notes, octaves reduced (unison = 1, ninth = 2)."""
    return abs(a["diatonic"] - b["diatonic"]) % 7 + 1


def two_voice_dissonances(fixtures: pathlib.Path):
    """Classify third-species dissonances and fourth/fifth-species suspensions in the two-part examples."""
    figures, suspensions = collections.Counter(), collections.Counter()
    for ex in tomllib.loads((fixtures / "gradus-2st.toml").read_text(encoding="utf-8"))["example"]:
        cf = [bar[0] for bar in measures(ex["parts"][ex["cf_part"] - 1])]
        cp_index = 2 if ex["cf_part"] == 1 else 1
        position = "unten" if cp_index == 2 else "oben"
        bars = measures(ex["parts"][cp_index - 1])
        if ex["species"] == [3]:
            line = [(b, i, n) for b, bar in enumerate(bars) for i, n in enumerate(bar) if n["step"]]
            for k in range(1, len(line) - 1):
                b, i, n = line[k]
                if i == 0 or is_consonant(n, cf[b]):
                    continue
                before, after = line[k - 1][2], line[k + 1][2]
                step_in, step_out = n["diatonic"] - before["diatonic"], after["diatonic"] - n["diatonic"]
                if abs(step_in) == 1 and abs(step_out) == 1:
                    kind = "Durchgang" if step_in == step_out else "Wechselnote"
                elif step_in == -1 and step_out == -2:
                    kind = f"Cambiata (Terz abwärts, Fig. {ex['figure']})"
                else:
                    kind = f"sonstige (hinein {step_in:+d}, heraus {step_out:+d} Stufen)"
                figures[f"{kind}, {i + 1}. Viertel"] += 1
        if ex["species"] in ([4], [5]):
            for b in range(1, len(bars)):
                first = bars[b][0] if bars[b] else None
                previous = bars[b - 1][-1] if bars[b - 1] else None
                if not (first and first["step"] and previous and previous["tie"]):
                    continue
                if is_consonant(first, cf[b]):
                    continue
                # Resolution: the next note that starts in this measure on beat 3 or earlier.
                resolution = next((n for n in bars[b][1:] if n["step"]), None)
                res = f"{generic(resolution, cf[b])}" if resolution else "?"
                key = f"G{ex['species'][0]}, Gegenstimme {position}: {generic(first, cf[b])}–{res}"
                suspensions[key] += 1
    return figures, suspensions


def upper_voice_dissonances(fixtures: pathlib.Path) -> dict[str, list[str]]:
    """Seconds and sevenths between two upper voices sounding at a downbeat (3 and 4 parts)."""
    found: dict[str, list[str]] = collections.defaultdict(list)
    for voices in (3, 4):
        for ex in tomllib.loads((fixtures / f"gradus-{voices}st.toml").read_text(encoding="utf-8"))["example"]:
            parts = [measures(p) for p in ex["parts"]]
            for b in range(len(parts[0])):
                # A note tied over the barline still sounds at the downbeat; rests do not.
                sounding = [bars[b][0] for bars in parts if bars[b] and bars[b][0]["step"]]
                if len(sounding) < voices:
                    continue
                bass = min(sounding, key=lambda n: n["midi"])
                uppers = [n for n in sounding if n is not bass]
                for i, a in enumerate(uppers):
                    for c in uppers[i + 1 :]:
                        low, high = sorted((a, c), key=lambda n: n["midi"])
                        steps = (high["diatonic"] - low["diatonic"]) % 7
                        if steps in (1, 6):
                            species = "+".join(map(str, ex["species"]))
                            key = f"{voices}st, G{species}, {'Sekunde' if steps == 1 else 'Septime'}"
                            found[key].append(f"Fig. {ex['figure']} T{b + 1}")
    return found


TICKS = {1: 8, 2: 4, 4: 2, 8: 1}  # eighth note = 1 tick, measure = 8 ticks


def timeline(music: str) -> list[dict]:
    """Sounding notes of one part with start/end ticks; tied notes are merged."""
    notes, t = [], 0
    for bar in measures(music):
        for n in bar:
            d = TICKS[n["dur"]] * 3 // 2 if n["dot"] else TICKS[n["dur"]]
            if n["step"] is not None:
                if notes and notes[-1]["open"] and notes[-1]["end"] == t:
                    notes[-1]["end"], notes[-1]["open"] = t + d, n["tie"]
                else:
                    notes.append({**n, "start": t, "end": t + d, "open": n["tie"]})
            t += d
    return notes


def sounding(notes: list[dict], t: int) -> dict | None:
    return next((n for n in notes if n["start"] <= t < n["end"]), None)


def slices(ex: dict) -> tuple[list[list[dict]], list[tuple[int, list]]]:
    """All onsets of an example with the note sounding in every part (None = rest)."""
    lines = [timeline(p) for p in ex["parts"]]
    onsets = sorted({n["start"] for line in lines for n in line})
    return lines, [(t, [sounding(line, t) for line in lines]) for t in onsets]


def interval(a: dict, b: dict) -> tuple[int, int]:
    """(diatonic steps, semitones) from the lower to the higher note."""
    low, high = sorted((a, b), key=lambda n: (n["diatonic"], n["midi"]))
    return high["diatonic"] - low["diatonic"], high["midi"] - low["midi"]


def perfect_class(a: dict, b: dict) -> str | None:
    steps, semis = interval(a, b)
    return {(0, 0): "8", (4, 7): "5"}.get((steps % 7, semis % 12))


def motion(a1: dict, a2: dict, b1: dict, b2: dict) -> str:
    da, db = a2["midi"] - a1["midi"], b2["midi"] - b1["midi"]
    if da == 0 and db == 0:
        return "keine"
    if da == 0 or db == 0:
        return "Seitenbewegung"
    return "gerade" if (da > 0) == (db > 0) else "Gegenbewegung"


CHORD_KINDS = (
    "5/3 vollständig",
    "6/3",
    "Terz + Oktave",
    "ohne Terz (Quinte/Oktave)",
    "leere Sexte (Sexte + Oktave)",
    "dissonant",
)


def chord_kind(above: set[int]) -> str:
    """Sonority type from the generic intervals above the bass (0 = octave, 2 = third, ...).

    The classes are those of TEX-TRIAD (`kind` in crates/fux-core/src/rules/texture.rs): a
    second, fourth or seventh above the bass, or a fifth with a sixth, is dissonant; without a
    third, fifths and octaves (`no_third`) are told from the bare sixth, a sixth with octaves only,
    which only free counterpoint rates (`free_bare_sixth`).
    """
    if not above <= {0, 2, 4, 5} or {4, 5} <= above:
        return "dissonant"
    if {2, 4} <= above:
        return "5/3 vollständig"
    if {2, 5} <= above:
        return "6/3"
    if 2 in above:
        return "Terz + Oktave"
    if 5 in above:
        return "leere Sexte (Sexte + Oktave)"
    return "ohne Terz (Quinte/Oktave)"


def texture_stats(fixtures: pathlib.Path) -> collections.Counter:
    c = collections.Counter()
    for voices in (2, 3, 4):
        for ex in tomllib.loads((fixtures / f"gradus-{voices}st.toml").read_text(encoding="utf-8"))["example"]:
            g1 = ex["species"] == [1]
            lines, sl = slices(ex)
            full = [(t, s) for t, s in sl if None not in s]
            last = sl[-1][0]
            # Parallel and hidden fifths/octaves, runs of parallel thirds/sixths.
            for i, j in itertools.combinations(range(voices), 2):
                outer = "Außenstimmen" if (i, j) == (0, voices - 1) else "andere Stimmpaare"
                run = 0
                for (t1, s1), (t2, s2) in zip(sl, sl[1:]):
                    a1, a2, b1, b2 = s1[i], s2[i], s1[j], s2[j]
                    if None in (a1, a2, b1, b2):
                        run = 0
                        continue
                    mot, target = motion(a1, a2, b1, b2), perfect_class(a2, b2)
                    if mot == "gerade" and target:
                        kind = "parallel" if perfect_class(a1, b1) == target else "verdeckt"
                        where = "in den Schlussklang" if t2 == last else "innen"
                        c[(voices, f"{kind}e Quinte/Oktave", outer, where)] += 1
                    if mot == "Gegenbewegung" and target and perfect_class(a1, b1) == target:
                        c[(voices, "Gegenparallele (gleiche vollkommene Konsonanz in Gegenbewegung)")] += 1
                    if g1:
                        cls1, cls2 = interval(a1, b1)[0] % 7, interval(a2, b2)[0] % 7
                        if mot == "gerade" and cls1 == cls2 and cls2 in (2, 5):
                            run = run + 1 if run else 2
                        else:
                            if run >= 3:
                                c[(voices, "G1 Terzen/Sexten parallel in Folge", run)] += 1
                            run = 0
                if run >= 3:
                    c[(voices, "G1 Terzen/Sexten parallel in Folge", run)] += 1
            # Voice crossing and spacing.
            for t, s in full:
                c[(voices, "Klänge gesamt")] += 1
                if any(s[k]["midi"] < s[k + 1]["midi"] for k in range(voices - 1)):
                    c[(voices, "Klänge mit Stimmkreuzung")] += 1
                if t % 8:
                    continue
                pitches = sorted(n["diatonic"] for n in s)
                gaps = [b - a for a, b in zip(pitches, pitches[1:])]
                if voices == 2:
                    label = "bis Dezime" if gaps[0] <= 9 else "Undezime/Duodezime" if gaps[0] <= 11 else "über Duodezime"
                    c[(2, "Abstand auf der Thesis", label)] += 1
                else:
                    c[(voices, "Oberstimmen-Abstände auf der Thesis")] += len(gaps) - 1
                    c[(voices, "Oberstimmen-Abstand über Oktave")] += sum(g > 7 for g in gaps[1:])
                    c[(voices, "Bass zur nächsten Stimme über Duodezime")] += gaps[0] > 11
            if voices == 2:
                accent_parallels(ex, lines, sl, c)
                two_voice_details(ex, lines, sl, c)
                continue
            # Sonorities against the bass (three and four parts); every class of the inner
            # first-species sonorities is listed, also where Fux never writes it.
            if g1:
                for kind in CHORD_KINDS:
                    c[(voices, "G1 innerer Klang", kind)] += 0
            final_pc = SEMITONES[ex["final"].lower()]
            for idx, (t, s) in enumerate(full):
                bass = min(s, key=lambda n: n["midi"])
                above = {interval(bass, n)[0] % 7 for n in s if n is not bass}
                third = next((n for n in s if n is not bass and interval(bass, n)[0] % 7 == 2), None)
                if idx == 0:
                    c[(voices, "erster Klang", "mit Terz" if third else "ohne Terz")] += 1
                    on_final = (bass["midi"] - final_pc) % 12 == 0
                    c[(voices, "erster Klang, Bass", "Finalis" if on_final else "andere Stufe")] += 1
                    c[(voices, "erster Klang mit Sexte über dem Bass")] += 5 in above
                if t == last:
                    quality = "ohne Terz"
                    if third:
                        quality = "große Terz" if interval(bass, third)[1] % 12 == 4 else "kleine Terz"
                        if interval(bass, third)[0] >= 9:
                            quality += " als Dezime oder höher"
                    c[(voices, "Schlussklang", quality)] += 1
                elif g1 and idx > 0:
                    kind = chord_kind(above)
                    c[(voices, "G1 innerer Klang", kind)] += 1
                    if any(a["midi"] == b["midi"] for a, b in itertools.combinations(s, 2)):
                        c[(voices, "G1 innerer Klang mit Einklang")] += 1
                    if voices == 4 and kind == "5/3 vollständig":
                        classes = [interval(bass, n)[0] % 7 for n in s if n is not bass]
                        doubled = {0: "Grundton", 2: "Terz", 4: "Quinte"}[max({0, 2, 4}, key=classes.count)]
                        c[(4, "G1 vollständiger Dreiklang, verdoppelt", doubled)] += 1
                        steps = sorted(interval(bass, n)[0] for n in s if n is not bass)
                        if steps == [4, 7, 9]:
                            c[(4, "G1 vollständiger Dreiklang in natürlicher Lage 5-8-10")] += 1
            cadence(ex, voices, lines, sl, c)
            if g1:
                for (t1, s1), (t2, s2) in zip(full, full[1:]):
                    c[(voices, "G1 Klangwechsel")] += 1
                    moves = [b["midi"] - a["midi"] for a, b in zip(s1, s2)]
                    if all(m > 0 for m in moves) or all(m < 0 for m in moves):
                        c[(voices, "G1 alle Stimmen in dieselbe Richtung")] += 1
                    bass1, bass2 = min(s1, key=lambda n: n["midi"]), min(s2, key=lambda n: n["midi"])
                    for x1, x2 in zip(s1, s2):
                        if x1 is bass1 or x2 is bass2:
                            continue
                        sixths = interval(bass1, x1)[0] % 7 == 5 and interval(bass2, x2)[0] % 7 == 5
                        if sixths and x2["midi"] > x1["midi"] and bass2["midi"] > bass1["midi"]:
                            c[(voices, "G1 steigende Sexten zum Bass in Folge")] += 1
    return c


def cadence(ex: dict, voices: int, lines: list[list[dict]], sl: list, c: collections.Counter) -> None:
    """The penultimate sonority (the last before the final measure): bass, leading tone, dissonance.

    TEX-PENULT and TEX-CLAUSULA cite these counts.
    """
    final_pc = SEMITONES[ex["final"].lower()]
    phrygian = ex["final"] == "E"
    # The leading tone; in the Phrygian mode the semitone above the final.
    leading = (final_pc + (1 if phrygian else -1)) % 12
    last = sl[-1][0]
    penultimate = last - TICKS[1]
    t, s = [x for x in sl if x[0] < last][-1]
    bass = min(s, key=lambda n: n["midi"])
    degree = (bass["midi"] - final_pc) % 12
    if phrygian:
        label = {10: "phrygisch D", 1: "phrygisch F"}.get(degree, f"phrygisch, {degree} HT über der Finalis")
    elif degree == 11:
        label = "verzögerter Leitton" if bass["start"] > penultimate else "Leitton auf der Thesis"
    else:
        label = {7: "5. Stufe", 2: "2. Stufe"}.get(degree, f"{degree} HT über der Finalis")
    c[(voices, "vorletzter Klang, Bass", label)] += 1
    down = next(x for x in sl if x[0] >= penultimate)[1]
    down_bass = min((n for n in down if n), key=lambda n: n["midi"])
    held = (down_bass["midi"] - final_pc) % 12 == 0 and down_bass["start"] < penultimate
    if held:
        c[(voices, "vorletzter Takt, Bass auf der Thesis", "Finalis über den Taktstrich gehalten")] += 1
    leads = [k for k, n in enumerate(s) if n["midi"] % 12 == leading]
    c[(voices, "vorletzter Klang, Stimmen mit dem Leitton", len(leads))] += 1
    for k in leads:
        after = next((n for n in lines[k] if n["start"] >= last), None)
        step = after["midi"] - s[k]["midi"] if after else None
        c[(voices, "Leitton geht in die Finalis", step in (-1, 1) and after["midi"] % 12 == final_pc)] += 1
    if any(n is not bass and not is_consonant(bass, n) and interval(bass, n)[0] % 7 != 3 for n in s):
        c[(voices, "vorletzter Klang mit Dissonanz zum Bass (ohne Quarte)", ex["figure"])] += 1
    if any(n is not bass and interval(bass, n)[0] % 7 == 3 for n in s):
        c[(voices, "vorletzter Klang mit Quarte zum Bass", ex["figure"])] += 1


# Semitone sizes of the major, minor and perfect intervals, by generic step (mod 7).
REGULAR_SEMITONES = {0: {0}, 1: {1, 2}, 2: {3, 4}, 3: {5}, 4: {7}, 5: {8, 9}, 6: {10, 11}}
NAMES = ["Einklang", "Sekunde", "Terz", "Quarte", "Quinte", "Sexte", "Septime"]


def melodic_checks(fixtures: pathlib.Path) -> dict[str, list[str]]:
    """Augmented/diminished melodic intervals in all parts; note repetitions and dissonant
    neighbour notes in the voice that carries the species (single-species examples only)."""
    found: dict[str, list[str]] = collections.defaultdict(list)
    for voices in (2, 3, 4):
        for ex in tomllib.loads((fixtures / f"gradus-{voices}st.toml").read_text(encoding="utf-8"))["example"]:
            fig = f"Fig. {ex['figure']}"
            lines, _ = slices(ex)
            for p, notes in enumerate(lines, 1):
                role = "CF" if p == ex["cf_part"] else "Gegenstimme"
                for a, b in zip(notes, notes[1:]):
                    steps, semis = abs(b["diatonic"] - a["diatonic"]), abs(b["midi"] - a["midi"])
                    if b["start"] == a["end"] and semis - 12 * (steps // 7) not in REGULAR_SEMITONES[steps % 7]:
                        name = "chromatischer Halbton" if steps == 0 else f"übermäßige/verminderte {NAMES[steps % 7]}"
                        found[f"{voices}st, {role}: {name}"].append(fig)
            species = ex["species"]
            if len(species) != 1 or species[0] not in (2, 3):
                continue
            shortest = {2: 4, 3: 2}[species[0]]  # ticks of a half / quarter note
            for p, notes in enumerate(lines):
                if p + 1 == ex["cf_part"] or not any(n["end"] - n["start"] == shortest for n in notes):
                    continue
                for a, b in zip(notes, notes[1:]):
                    if a["midi"] == b["midi"] and b["start"] == a["end"]:
                        found[f"{voices}st, G{species[0]}: Tonwiederholung"].append(fig)
                if species[0] != 3:
                    continue
                others = [line for q, line in enumerate(lines) if q != p]
                for a, b, d in zip(notes, notes[1:], notes[2:]):
                    if b["start"] % 8 == 0 or d["diatonic"] != a["diatonic"] or abs(b["diatonic"] - a["diatonic"]) != 1:
                        continue
                    sounding_now = [sounding(line, b["start"]) for line in others]
                    bass = min([n for n in sounding_now if n] + [b], key=lambda n: n["midi"])
                    for o in filter(None, sounding_now):
                        fourth_between_uppers = interval(b, o)[0] % 7 == 3 and bass is not b and bass is not o
                        if not is_consonant(b, o) and not fourth_between_uppers:
                            found[f"{voices}st, G3: dissonante Wechselnote"].append(
                                f"{fig} T{b['start'] // 8 + 1}"
                            )
                            break
    return found


def two_voice_details(ex: dict, lines: list[list[dict]], sl: list, c: collections.Counter) -> None:
    """Unisons on inner downbeats and the context of octave leaps in the second species."""
    cf, cp = ex["cf_part"] - 1, 2 - ex["cf_part"]
    species = ex["species"][0]
    last = sl[-1][0]
    for t, s in sl:
        if t % 8 == 0 and 0 < t < last and None not in s and s[0]["midi"] == s[1]["midi"]:
            c[(2, f"G{species} Einklang auf innerer Thesis")] += 1
    if species != 2:
        return
    notes = lines[cp]
    for a, b in zip(notes, notes[1:]):
        if abs(b["diatonic"] - a["diatonic"]) != 7:
            continue
        position = "Thesis→Arsis" if a["start"] % 8 == 0 else "Arsis→Thesis"
        other = sounding(lines[cf], a["start"])
        close = other is not None and abs(other["diatonic"] - a["diatonic"]) <= 2
        c[(2, "G2 Oktavsprung", position, "Stimmen höchstens eine Terz entfernt" if close else "Stimmen weiter entfernt")] += 1


def accent_parallels(ex: dict, lines: list[list[dict]], sl: list, c: collections.Counter) -> None:
    """Same perfect interval on consecutive downbeats, reached in similar motion (two parts)."""
    if ex["species"] not in ([2], [3], [5]):
        return
    cf, cp = ex["cf_part"] - 1, 2 - ex["cf_part"]
    downbeats = {t: s for t, s in sl if t % 8 == 0}
    for t, s1 in downbeats.items():
        s2 = downbeats.get(t + 8)
        if not s2 or None in (s1[cf], s1[cp], s2[cf], s2[cp]):
            continue
        target = perfect_class(s2[cf], s2[cp])
        if not target or perfect_class(s1[cf], s1[cp]) != target:
            continue
        if motion(s1[cp], s2[cp], s1[cf], s2[cf]) != "gerade":
            continue
        detail = ""
        if ex["species"] == [2]:
            arsis = sounding(lines[cp], t + 4)
            leap = abs(arsis["diatonic"] - s1[cp]["diatonic"]) if arsis else 0
            detail = ", Arsis-Sprung bis Terz" if leap <= 2 else ", Arsis-Sprung ab Quarte"
        label = f"G{ex['species'][0]} Quinten/Oktaven auf aufeinanderfolgenden Thesen{detail}"
        c[(2, label, f"Fig. {ex['figure']} T{t // 8 + 1}")] += 1


# Diatonic index (7 * octave + step) of the bottom staff line for each original clef.
STAFF_BOTTOM_LINE = {"C1": 7 * 4 + 0, "C3": 7 * 3 + 3, "C4": 7 * 3 + 1, "F4": 7 * 2 + 4, "G2": 7 * 4 + 2}


def sci(diatonic: int) -> str:
    return STEPS[diatonic % 7].upper() + str(diatonic // 7)


if __name__ == "__main__":
    main()
