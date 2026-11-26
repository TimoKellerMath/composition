#!/usr/bin/env python3
"""Die Schlüsse von Stücken: der Schlussakkord und der Weg der Oberstimme in ihn.

    tools/bach_schlussakkorde.py references/datasets/bach-fugen/wtc/*.krn
    tools/bach_schlussakkorde.py out/*.musicxml

Liest kern (Akkorde in einer Stimme zählen mit allen Tönen; die Oberstimme ist die letzte Spalte)
und MusicXML, wie fux-fuge es schreibt (der Schlussakkord füllt den letzten Takt, Stimme 1 ist die
Oberstimme). Gezählt wird, was beim letzten Anschlag klingt, als Töne über dem tiefsten: je Art
eine Zeile mit Anzahl und Namen. Dann der Ton, auf dem die Oberstimme endet, das Intervall, mit
dem sie ihn erreicht, und die Arten nach Stimmenzahl (bei kern die Zahl der Spalten am Anfang),
getrennt danach, ob der Schlussakkord mehr Töne hat als das Stück Stimmen. Bei MusicXML dazu, in
wie vielen Stücken alle Stimmen im Schlussakkord klingen.
"""
import collections
import re
import sys
import xml.etree.ElementTree as ET

STEP = {"c": 0, "d": 2, "e": 4, "f": 5, "g": 7, "a": 9, "b": 11}
KINDS = {
    (0,): "Einklang oder Oktave",
    (0, 4): "Grundton und Terz",
    (0, 3): "Grundton und Terz",
    (0, 7): "leere Quinte",
    (0, 4, 7): "Dreiklang",
    (0, 3, 7): "Dreiklang",
}
ORDER = ["Dreiklang", "Grundton und Terz", "Einklang oder Oktave", "leere Quinte"]
TOPS = {0: "Grundton", 3: "Terz", 4: "Terz", 7: "Quinte"}


def kern_pitches(token):
    out = []
    for sub in token.split():
        match = re.search(r"([A-Ga-g]+)([#\-n]*)", sub)
        if not match or "r" in sub:
            continue
        letters, marks = match.group(1), match.group(2)
        octave = 3 + len(letters) if letters[0].islower() else 4 - len(letters)
        step = STEP[letters[0].lower()]
        out.append(12 * (octave + 1) + step + marks.count("#") - marks.count("-"))
    return out


def kern(path):
    """Was beim letzten Anschlag klingt (je Spalte ihr letztes Zeichen), und die Oberstimme:
    die Töne der letzten Spalte der Reihe nach, None für eine Pause."""
    current, final, top, voices = [], None, [], 0
    for line in open(path, encoding="utf-8", errors="replace"):
        line = line.rstrip("\n")
        if not line or line.startswith("!") or line.startswith("="):
            continue
        fields = line.split("\t")
        if line.startswith("**"):
            voices = fields.count("**kern")
        if line.startswith("*"):
            if any(f in ("*^", "*v", "*-", "*+", "*x") for f in fields):
                new, k = [], 0
                while k < len(fields):
                    cur = current[k] if k < len(current) else "."
                    if fields[k] == "*^":
                        new += [cur, cur]
                    elif fields[k] == "*v":
                        j = k
                        while j + 1 < len(fields) and fields[j + 1] == "*v":
                            j += 1
                        merged = " ".join(c for c in current[k : j + 1] if c not in (".", ""))
                        new.append(merged or ".")
                        k = j
                    elif fields[k] != "*-":
                        new.append(cur)
                    k += 1
                current = new
            continue
        current = (current + ["."] * len(fields))[: len(fields)]
        onset = False
        for k, field in enumerate(fields):
            if field == ".":
                continue
            current[k] = field
            # Ein gebundener Ton ist kein Anschlag.
            struck = "]" not in field and "_" not in field
            if kern_pitches(field) and struck:
                onset = True
            if k == len(fields) - 1 and struck:
                tones = kern_pitches(field)
                top.append(max(tones) if tones else None)
        if onset:
            final = [p for c in current for p in kern_pitches(c)]
    return final, top, voices, None


def musicxml(path):
    """Die letzten Töne der Stimmen im letzten Takt mit Tönen, die Oberstimme (Stimme 1), und
    ob alle Stimmen im Schlussakkord klingen."""
    root = ET.parse(path).getroot()
    voices, last, top = set(), {}, []

    def midi(note):
        pitch = note.find("pitch")
        alter = int(float(pitch.findtext("alter") or 0))
        return 12 * (int(pitch.findtext("octave")) + 1) + STEP[pitch.findtext("step").lower()] + alter

    for part in root.iter("part"):
        name = part.get("id")
        measures = part.findall("measure")
        for measure in measures:
            for note in measure.findall("note"):
                pitched = note.find("pitch") is not None
                if pitched:
                    voices.add((name, note.findtext("voice")))
                if note.findtext("voice") == "1":
                    if not pitched:
                        top.append(None)
                    elif not any(t.get("type") == "stop" for t in note.findall("tie")):
                        top.append(midi(note))
        for measure in reversed(measures):
            notes = [n for n in measure.findall("note") if n.find("pitch") is not None]
            if not notes:
                continue
            for note in notes:
                last[(name, note.findtext("voice"))] = midi(note)
            break
    return list(last.values()), top, len(voices), len(last) == len(voices)


def arrival(top):
    """Wie die Oberstimme ihren letzten Ton erreicht, und ob vom letzten anderen Ton her
    schrittweise."""
    while top and top[-1] is None:
        top = top[:-1]
    last = top[-1]
    before = top[-2] if len(top) > 1 else None
    if before is None:
        kind = "nach einer Pause"
    elif before == last:
        kind = "Tonwiederholung"
    elif abs(last - before) <= 2:
        kind = "Schritt aufwärts" if last > before else "Schritt abwärts"
    else:
        kind = "Sprung"
    other = next((t for t in reversed(top[:-1]) if t is not None and t != last), None)
    return kind, other is not None and abs(last - other) <= 2


def main(paths):
    kinds, whole, known = {}, 0, 0
    tops, arrivals, stepwise, count = collections.Counter(), collections.Counter(), 0, 0
    by_voices = collections.Counter()
    for path in paths:
        read = musicxml if path.endswith(".musicxml") else kern
        tones, top, voices, all_voices = read(path)
        name = re.sub(r"\.(krn|musicxml)$", "", path.split("/")[-1])
        if not tones:
            kinds.setdefault("ohne Töne", []).append(name)
            continue
        count += 1
        low = min(tones)
        classes = tuple(sorted({(t - low) % 12 for t in tones}))
        kind = KINDS.get(classes, "anderes " + str(list(classes)))
        kinds.setdefault(kind, []).append(name)
        by_voices[(voices, len(tones) > voices, kind)] += 1
        tops[TOPS.get((max(tones) - low) % 12, "anderer Ton")] += 1
        how, by_step = arrival(top)
        arrivals[how] += 1
        stepwise += by_step
        if all_voices is not None:
            known += 1
            whole += all_voices
    print("Schlussakkord:")
    for kind in ORDER + sorted(k for k in kinds if k not in ORDER):
        names = kinds.get(kind, [])
        print(f"{len(names):3}  {kind}{':  ' + ' '.join(names) if names else ''}")
    print("Die Oberstimme endet auf: " + ", ".join(
        f"{name} {tops[name]}" for name in ["Grundton", "Terz", "Quinte", "anderer Ton"] if tops[name] or name != "anderer Ton"
    ) + ".")
    print("Die Oberstimme erreicht ihren Schlusston: " + ", ".join(
        f"{name} {arrivals[name]}"
        for name in ["Schritt aufwärts", "Schritt abwärts", "Tonwiederholung", "nach einer Pause", "Sprung"]
    ) + f"; vom letzten anderen Ton her schrittweise: {stepwise} von {count}.")
    print("Nach Stimmenzahl:")
    for voices in sorted({key[0] for key in by_voices}):
        for more in (False, True):
            found = [(kind, by_voices[(voices, more, kind)]) for kind in ORDER]
            found = [f"{kind} {n}" for kind, n in found if n]
            if found:
                tones = "mehr Töne als Stimmen" if more else "höchstens ein Ton je Stimme"
                print(f"  {voices} Stimmen, {tones}: " + ", ".join(found))
    if known:
        print(f"Alle Stimmen im Schlussakkord: {whole} von {known}.")


if __name__ == "__main__":
    main(sys.argv[1:])
