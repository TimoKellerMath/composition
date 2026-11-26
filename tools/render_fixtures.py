#!/usr/bin/env python3
"""Typeset all Fux examples from tests/fixtures/fux/gradus-*st.toml with LilyPond.

Usage (from the repository root):
    python3 tools/render_fixtures.py [--modern-clefs] [--no-pdf] [FIXTURE_DIR] [OUT_LY]

Defaults: tests/fixtures/fux and references/fux/gradus-beispiele.ly.
Writes one \\score per example (original clefs unless --modern-clefs) and, unless
--no-pdf is given, runs `lilypond` to produce the PDF next to the .ly file.
LilyPond's bar checks double as a check that every measure is complete.
"""

import pathlib
import subprocess
import sys
import tomllib

ORIGINAL_CLEFS = {"C1": "soprano", "C3": "alto", "C4": "tenor", "F4": "bass", "G2": "treble"}
MODERN_CLEFS = {"C1": "treble", "C3": "treble", "C4": "treble_8", "F4": "bass", "G2": "treble"}
MODES = {"D": "dorisch", "E": "phrygisch", "F": "lydisch", "G": "mixolydisch", "A": "äolisch", "C": "ionisch"}


def score(ex: dict, clefs: dict[str, str]) -> str:
    species = "+".join(str(s) for s in ex["species"])
    title = f"Fig. {ex['figure']} – {MODES[ex['final']]} – Gattung {species}"
    if ex["note"]:
        title += f" ({ex['note']})"
    staves = []
    for i, (clef, music) in enumerate(zip(ex["clefs"], ex["parts"]), 1):
        name = '\\with { instrumentName = "C. f." } ' if i == ex["cf_part"] else ""
        staves.append(f'    \\new Staff {name}{{ \\time 2/2 \\clef "{clefs[clef]}" {music} \\bar "|." }}')
    return (
        "\\score {\n"
        f'  \\header {{ piece = "{title}" opus = "{ex["id"]}, S. {ex["page"]}" }}\n'
        "  \\new StaffGroup <<\n" + "\n".join(staves) + "\n  >>\n"
        "  \\layout { }\n}"
    )


def main() -> None:
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    clefs = MODERN_CLEFS if "--modern-clefs" in sys.argv else ORIGINAL_CLEFS
    fixtures = pathlib.Path(args[0] if args else "tests/fixtures/fux")
    out = pathlib.Path(args[1] if len(args) > 1 else "references/fux/gradus-beispiele.ly")

    blocks = [
        '\\version "2.26.0"',
        "\\header {\n"
        '  title = "J. J. Fux, Gradus ad Parnassum (1725)"\n'
        '  subtitle = "Alle Notenbeispiele"\n'
        '  subsubtitle = "Daten: github.com/MarkGotham/species (CC0), erzeugt mit tools/render_fixtures.py"\n'
        "  tagline = ##f\n}",
    ]
    for voices in (2, 3, 4):
        examples = tomllib.loads((fixtures / f"gradus-{voices}st.toml").read_text(encoding="utf-8"))["example"]
        blocks.append(f'\\markup \\fill-line {{ \\large \\bold "{voices}-stimmig" }}')
        blocks.extend(score(ex, clefs) for ex in examples)
    out.write_text("\n\n".join(blocks) + "\n", encoding="utf-8")
    print(f"wrote {out}")

    if "--no-pdf" not in sys.argv:
        result = subprocess.run(
            ["lilypond", "-dno-point-and-click", "-o", str(out.with_suffix("")), str(out)],
            capture_output=True,
            text=True,
        )
        problems = [line for line in result.stderr.splitlines() if "warning" in line or "error" in line]
        if result.returncode != 0 or problems:
            sys.exit("lilypond reported problems:\n" + "\n".join(problems or [result.stderr]))
        print(f"wrote {out.with_suffix('.pdf')}")


if __name__ == "__main__":
    main()
