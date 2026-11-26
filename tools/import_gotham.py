#!/usr/bin/env python3
"""Convert Mark Gotham's Fux corpus (Humdrum **kern + JSON metadata) into the
TOML test fixtures under tests/fixtures/fux/.

Usage (from the repository root):
    python3 tools/import_gotham.py [DATASET_DIR] [OUT_DIR]

Defaults: references/datasets/gotham-species and tests/fixtures/fux.
Uses only the standard library. Voices are written in LilyPond absolute
notation with Dutch note names (c' = C4) and explicit durations on every note.
Each example is checked for complete measures; its cantus firmus is matched
against tests/fixtures/fux/cantus.toml up to octave transposition.
"""

import json
import pathlib
import re
import sys
import tomllib

TOKEN = re.compile(
    r"^(?P<tie_start>\[)?(?P<dur>\d+)(?P<dot>\.)?(?P<pitch>[a-gA-G]+|r)"
    r"(?P<acc>[#n-])?X?[LJ]*(?P<tie_end>\])?[LJ]*$"
)
TICKS_PER_WHOLE = 8  # one eighth note = 1 tick, one measure (2/2) = 8 ticks
LILY_ACCIDENTAL = {None: "", "n": "", "#": "is", "-": "es"}
SCI_ACCIDENTAL = {None: "", "n": "", "#": "#", "-": "b"}


def kern_octave(letters: str) -> int:
    """Octave in scientific pitch notation: c = C4, cc = C5, C = C3, CC = C2."""
    return 3 + len(letters) if letters.islower() else 4 - len(letters)


def lily_pitch(step: str, acc: str | None, octave: int) -> str:
    name = step + LILY_ACCIDENTAL[acc]
    if name in ("ees", "aes"):  # LilyPond's Dutch spelling of E-flat and A-flat
        name = name[0] + "s"
    marks = "'" * (octave - 3) if octave >= 3 else "," * (3 - octave)
    return name + marks


def ticks(dur: int, dotted: bool) -> int:
    t = TICKS_PER_WHOLE // dur
    return t + t // 2 if dotted else t


def parse_kern(path: pathlib.Path):
    """Return {part_number: (lily_measures, sci_notes)} for one **kern file.

    lily_measures is a list of measures, each a list of LilyPond tokens.
    sci_notes is the list of sounding pitches (scientific notation), ties merged.
    """
    spine_parts: list[int] = []
    parts: dict[int, list[list[str]]] = {}
    sci: dict[int, list[str]] = {}
    tied: dict[int, bool] = {}
    measure_ticks: dict[int, int] = {}

    def close_measure(part: int, label: str) -> None:
        if parts[part][-1]:
            if measure_ticks[part] != TICKS_PER_WHOLE:
                sys.exit(f"{path}: part {part}, measure {label}: {measure_ticks[part]} ticks")
            parts[part].append([])
            measure_ticks[part] = 0

    for line in path.read_text(encoding="utf-8").splitlines():
        if not line or line.startswith("!"):
            continue
        cols = line.split("\t")
        if cols[0].startswith("*part"):
            spine_parts = [int(c.removeprefix("*part")) for c in cols]
            for p in spine_parts:
                parts[p], sci[p], tied[p], measure_ticks[p] = [[]], [], False, 0
            continue
        if line.startswith("*"):
            continue
        if line.startswith("="):
            for p in spine_parts:
                close_measure(p, cols[0])
            continue
        for p, tok in zip(spine_parts, cols):
            if tok == ".":
                continue
            m = TOKEN.match(tok)
            if not m:
                sys.exit(f"{path}: unknown kern token {tok!r}")
            dur, dotted = int(m["dur"]), bool(m["dot"])
            measure_ticks[p] += ticks(dur, dotted)
            lily_dur = f"{dur}{'.' if dotted else ''}"
            if m["pitch"] == "r":
                parts[p][-1].append(f"r{lily_dur}")
                continue
            step, octave = m["pitch"][0].lower(), kern_octave(m["pitch"])
            tie = "~" if m["tie_start"] else ""
            parts[p][-1].append(lily_pitch(step, m["acc"], octave) + lily_dur + tie)
            if not tied[p]:
                sci[p].append(f"{step.upper()}{SCI_ACCIDENTAL[m['acc']]}{octave}")
            tied[p] = bool(m["tie_start"])
    for p in parts:
        if not parts[p][-1]:
            parts[p].pop()
    return {p: (parts[p], sci[p]) for p in parts}


def pitch_classes(notes: list[str]) -> list[str]:
    return [re.sub(r"-?\d+$", "", n) for n in notes]


def load_metadata(path: pathlib.Path) -> dict:
    text = path.read_text(encoding="utf-8")
    # gap_201.json contains a trailing comma; tolerate that instead of editing the source.
    return json.loads(re.sub(r",(\s*[}\]])", r"\1", text))


def toml_str(s: str) -> str:
    return json.dumps(s, ensure_ascii=False)


def main() -> None:
    dataset = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "references/datasets/gotham-species")
    out_dir = pathlib.Path(sys.argv[2] if len(sys.argv) > 2 else "tests/fixtures/fux")
    with open(out_dir / "cantus.toml", "rb") as f:
        cantus = tomllib.load(f)["cantus"]
    known = {c["id"]: pitch_classes(c["notes"].split()) for c in cantus}
    commit = re.search(r"Commit: (\w+)", (dataset / "SOURCE.txt").read_text()).group(1)

    by_voices: dict[int, list[str]] = {2: [], 3: [], 4: []}
    unmatched = []
    for krn in sorted(dataset.glob("*/sp*/*.krn")):
        meta = load_metadata(krn.with_suffix(".json"))
        parsed = parse_kern(krn)
        voices = meta["voices"]
        if sorted(parsed) != list(range(1, voices + 1)):
            sys.exit(f"{krn}: parts {sorted(parsed)} do not match {voices} voices")
        cf_part = meta["cantus_firmus"]["part"]
        cf_notes = parsed[cf_part][1]
        matches = [cid for cid, pcs in known.items() if pcs == pitch_classes(cf_notes)]
        if not matches:
            unmatched.append((krn.stem, " ".join(cf_notes)))
        fig = meta["figure"]
        source = meta.get("source") or {}
        lines = [
            "[[example]]",
            f"id = {toml_str(meta['id'])}",
            f"figure = {toml_str(str(fig['number']) + (fig.get('suffix') or ''))}",
            f"note = {toml_str(fig.get('note') or '')}",
            f"final = {toml_str(meta['modal_final'].upper())}",
            f"species = {json.dumps(meta['species'])}",
            f"cf_part = {cf_part}",
            f"cantus = {toml_str(matches[0] if matches else '')}",
            f"clefs = {json.dumps(meta['clefs'])}",
            f"page = {meta.get('page') or 0}",
            f"source_pdf = {toml_str(source.get('pdf_file') or '')}",
            "parts = [",
        ]
        for p in range(1, voices + 1):
            music = " | ".join(" ".join(m) for m in parsed[p][0])
            lines.append(f"  {toml_str(music)},")
        lines.append("]")
        by_voices[voices].append("\n".join(lines))

    for voices, examples in by_voices.items():
        header = (
            f"# Fux, Gradus ad Parnassum (1725): all {voices}-part examples.\n"
            "# Generated by tools/import_gotham.py from references/datasets/gotham-species\n"
            f"# (github.com/MarkGotham/species, commit {commit[:7]}, scores CC0). Do not edit by hand.\n"
            "#\n"
            "# parts: top to bottom, LilyPond absolute notation (c' = C4), '|' = barline.\n"
            "# cf_part: 1-based index of the cantus firmus part, counted from the top.\n"
            "# cantus: id of the matching cantus firmus in cantus.toml.\n"
            "# clefs: original clefs, top to bottom. page: page in the 1725 print.\n"
        )
        (out_dir / f"gradus-{voices}st.toml").write_text(
            header + "\n" + "\n\n".join(examples) + "\n", encoding="utf-8"
        )
        print(f"gradus-{voices}st.toml: {len(examples)} examples")
    for stem, notes in unmatched:
        print(f"no cantus.toml entry for {stem}: {notes}")


if __name__ == "__main__":
    main()
