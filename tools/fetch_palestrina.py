#!/usr/bin/env python3
"""Download Palestrina's masses in Humdrum **kern into references/datasets/palestrina-masses/.

Usage (from the repository root):
    python3 tools/fetch_palestrina.py [OUT_DIR]

The files come from the corpus of music21 (github.com/cuthbertLab/music21, directory
music21/corpus/palestrina) at a fixed commit, so that every download gives the same files. John
Miller encoded the masses (copyright 2000) after the complete edition; Bret Aarden converted
them to Humdrum. music21 distributes them with the permission of the encoders; see the README
in the output directory. Uses only the standard library.
"""

import concurrent.futures
import json
import pathlib
import sys
import urllib.request

REPOSITORY = "cuthbertLab/music21"
COMMIT = "1e68f5fc08970e8cee1e25627039dc0b64275367"
DIRECTORY = "music21/corpus/palestrina/"
TREE = f"https://api.github.com/repos/{REPOSITORY}/git/trees/{COMMIT}?recursive=1"
RAW = f"https://raw.githubusercontent.com/{REPOSITORY}/{COMMIT}/{DIRECTORY}{{name}}"


def fetch(url: str) -> bytes:
    with urllib.request.urlopen(url, timeout=60) as response:
        return response.read()


def main() -> None:
    out = pathlib.Path(
        sys.argv[1] if len(sys.argv) > 1 else "references/datasets/palestrina-masses"
    )
    (out / "kern").mkdir(parents=True, exist_ok=True)
    tree = json.loads(fetch(TREE))
    if tree.get("truncated"):
        sys.exit("the listing of the repository is truncated")
    names = sorted(
        entry["path"][len(DIRECTORY):]
        for entry in tree["tree"]
        if entry["type"] == "blob"
        and entry["path"].startswith(DIRECTORY)
        and entry["path"].endswith(".krn")
    )

    def download(name: str) -> str:
        text = fetch(RAW.format(name=name)).decode("utf-8", errors="replace")
        text = text.replace("\r\n", "\n").replace("\r", "\n")
        if not text.endswith("\n"):
            text += "\n"
        (out / "kern" / name).write_text(text, encoding="utf-8")
        return name

    with concurrent.futures.ThreadPoolExecutor(max_workers=16) as pool:
        done = list(pool.map(download, names))
    print(f"{len(done)} files from {REPOSITORY}@{COMMIT[:10]} in {out / 'kern'}")


if __name__ == "__main__":
    main()
