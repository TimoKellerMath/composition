#!/usr/bin/env python3
"""Download chant melodies in GABC from GregoBase: the hymns in tests/fixtures/chant/ and the
chants of the Kyriale that fux-core embeds (crates/fux-core/src/imitation/kyriale/).

Usage (from anywhere):
    python3 tools/fetch_chant.py [OUT_ROOT]

Each file goes to its directory below OUT_ROOT, by default the repository root. A download into
an empty directory can be compared with the files in the repository before it replaces them.

GregoBase (https://gregobase.selapa.net) releases all its transcriptions under CC0. A chant that
GregoBase marks as still under copyright is skipped, as is one without a GABC transcription. The
script takes the GABC that the page of each chant offers for download, without further verses;
a hymn whose strophes are all in one transcription keeps its first strophe. The line endings
are normalized, and the text is checked to be GABC. The README.md beside the files names their
sources. Uses only the standard library.
"""

import pathlib
import re
import sys
import urllib.request

FIXTURES = "tests/fixtures/chant"
KYRIALE = "crates/fux-core/src/imitation/kyriale"

# Directory, file name, GregoBase id.
CHANTS = [
    (FIXTURES, "ave-maris-stella.gabc", 9733),
    (FIXTURES, "veni-creator.gabc", 3241),
    (FIXTURES, "pange-lingua.gabc", 3855),
    (FIXTURES, "iste-confessor.gabc", 2334),
    (KYRIALE, "kyrie-ix.gabc", 2976),
    (KYRIALE, "gloria-ix.gabc", 2771),
    (KYRIALE, "sanctus-ix.gabc", 587),
    (KYRIALE, "agnus-ix.gabc", 707),
    (KYRIALE, "credo-i.gabc", 344),
    (KYRIALE, "kyrie-iv.gabc", 1061),
]
SITE = "https://gregobase.selapa.net/"
PAGE = SITE + "chant.php?id={id}"
# The page of a chant shows this instead of the score, and offers no download.
COPYRIGHTED = "This tune is still under copyright."
# The GABC downloads that a page offers: the whole chant, or its elements one by one.
DOWNLOAD = re.compile(r'href="(download\.php\?id=(\d+)&amp;format=gabc(?:&amp;elem=(\d+))?)"')
# A line that begins the second strophe of a hymn.
SECOND_STROPHE = re.compile(r"^\s*2\.\s", re.MULTILINE)


def fetch(url: str) -> str:
    with urllib.request.urlopen(url, timeout=60) as response:
        return response.read().decode("utf-8")


def downloads(chant_id: int, page: str) -> list[str]:
    """The URLs of the chant's GABC, its elements in order; empty without a transcription."""
    links = []
    for href, link_id, element in DOWNLOAD.findall(page):
        if int(link_id) == chant_id:
            links.append((int(element or 0), SITE + href.replace("&amp;", "&") + "&1verse=1"))
    return [url for _, url in sorted(set(links))]


def first_strophe(body: str) -> str:
    """The body up to the second strophe, if the transcription numbers its strophes."""
    second = SECOND_STROPHE.search(body)
    return body if second is None else body[: second.start()].rstrip() + "\n"


def check(text: str, chant_id: int) -> None:
    """Stops unless the text looks like GABC: a header of `key:value;` lines, `%%`, a body."""
    header, separator, body = text.partition("%%\n")
    if not separator or "(" not in body:
        sys.exit(f"GregoBase {chant_id}: no GABC body")
    for line in header.splitlines():
        if line and not re.fullmatch(r"[\w-]+:.*;", line):
            sys.exit(f"GregoBase {chant_id}: unexpected header line {line!r}")
    if any(not c.isprintable() and c not in "\n\t" for c in text):
        sys.exit(f"GregoBase {chant_id}: control characters in the GABC")


def gabc(chant_id: int, urls: list[str]) -> str:
    """The chant's GABC: the header of the first element and the bodies of all of them."""
    header, bodies = "", []
    for url in urls:
        text = fetch(url).replace("\r\n", "\n").replace("\r", "\n")
        head, separator, body = text.partition("\n%%\n")
        if not separator:
            sys.exit(f"GregoBase {chant_id}: no GABC in {url}")
        header = header or head + "\n%%\n"
        bodies.append(body if body.endswith("\n") else body + "\n")
    text = header + first_strophe("".join(bodies))
    check(text, chant_id)
    return text


def main() -> None:
    repository = pathlib.Path(__file__).resolve().parents[1]
    root = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else repository
    for directory, name, chant_id in CHANTS:
        page = fetch(PAGE.format(id=chant_id))
        if COPYRIGHTED in page:
            print(f"{name}: GregoBase {chant_id} is under copyright, skipped")
            continue
        urls = downloads(chant_id, page)
        if not urls:
            print(f"{name}: GregoBase {chant_id} has no GABC, skipped")
            continue
        out = root / directory
        out.mkdir(parents=True, exist_ok=True)
        (out / name).write_text(gabc(chant_id, urls), encoding="utf-8")
        print(f"{name}: GregoBase {chant_id}")


if __name__ == "__main__":
    main()
