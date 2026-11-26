#!/usr/bin/env bash
# Builds the web version into target/web: the WebAssembly module (crates/fux-web, with the profile
# wasm of Cargo.toml) and its gzip copy fux_web.wasm.gz, the page (web/) and Verovio, which
# engraves the notation (target/web/verovio). Serve the directory with any web server, e.g.
# `python3 -m http.server -d target/web`; an upload takes all of it.
#
# Needs the target wasm32-unknown-unknown (rustup target add wasm32-unknown-unknown; on Arch
# Linux the package rust-wasm). Without it, but with the sources of the standard library
# (rust-src), the standard library is built from them. The first build downloads Verovio with curl
# into target/verovio-<version>; Python 3 checks the page's Content-Security-Policy.
set -euo pipefail
cd "$(dirname "$0")/.."

# Browsers run the page's inline script only if its Content-Security-Policy names the script's hash.
python3 - web/index.html <<'EOF'
import base64, hashlib, re, sys

page = open(sys.argv[1], encoding="utf-8").read()
policy = re.search(r'http-equiv="Content-Security-Policy"\s+content="([^"]*)"', page)
for script in re.findall(r"<script>(.*?)</script>", page, re.S):
    digest = base64.b64encode(hashlib.sha256(script.encode()).digest()).decode()
    if not policy or f"'sha256-{digest}'" not in policy[1]:
        sys.exit(f"{sys.argv[1]}: its Content-Security-Policy lacks 'sha256-{digest}', "
                 "the hash of an inline script.")
EOF

target=wasm32-unknown-unknown
sysroot=$(rustc --print sysroot)
build=(cargo build -p fux-web --profile wasm --target "$target" --target-dir target/web-build)
if [ -d "$sysroot/lib/rustlib/$target" ]; then
  "${build[@]}"
elif [ -d "$sysroot/lib/rustlib/src/rust/library" ]; then
  echo "No standard library for $target: building it from rust-src." >&2
  RUSTC_BOOTSTRAP=1 "${build[@]}" -Zbuild-std=std,panic_abort
else
  echo "Missing the target $target: rustup target add $target (Arch Linux: pacman -S rust-wasm)." >&2
  exit 1
fi

# Verovio is free software (LGPL-3.0-or-later): the toolkit from its npm package, the licences from
# its repository at the same version, each checked against its SHA-256.
verovio=6.3.0
cache=target/verovio-$verovio
fetch() { # url, file name, SHA-256
  local file=$cache/$2 sum
  if [ ! -f "$file" ]; then
    mkdir -p "$cache"
    curl -fsSL --retry 3 -o "$file.part" "$1"
    mv "$file.part" "$file"
  fi
  sum=$( (sha256sum "$file" 2>/dev/null || shasum -a 256 "$file") | cut -d ' ' -f 1)
  if [ "$sum" != "$3" ]; then
    rm "$file"
    echo "$1: SHA-256 $sum instead of $3; the download is removed." >&2
    exit 1
  fi
}
fetch "https://cdn.jsdelivr.net/npm/verovio@$verovio/dist/verovio-toolkit-wasm.js" \
  verovio-toolkit-wasm.js d794119cd5ea83a3e835849945f0289c91ac2bac427f4aa312c7010a800b46bd
fetch "https://raw.githubusercontent.com/rism-digital/verovio/version-$verovio/COPYING" \
  COPYING 8b1ba204bb69a0ade2bfcf65ef294a920f6bb361b317dba43c7ef29d96332b9b
fetch "https://raw.githubusercontent.com/rism-digital/verovio/version-$verovio/COPYING.LESSER" \
  COPYING.LESSER a5681bf9b05db14d86776930017c647ad9e6e56ff6bbcfdf21e5848288dfaf1b

mkdir -p target/web/verovio
cp web/index.html web/style.css web/app.js web/worker.js web/fux.js web/pool.js target/web/
cp "target/web-build/$target/wasm/fux_web.wasm" target/web/
# The site's server sends fux_web.wasm uncompressed, so the page fetches this copy and decompresses
# it itself (web/fux.js). Without name and time (-n) the same module gives the same copy.
gzip -9 -n -c target/web/fux_web.wasm >target/web/fux_web.wasm.gz
cp "$cache"/{verovio-toolkit-wasm.js,COPYING,COPYING.LESSER} target/web/verovio/
cat > target/web/verovio/README.txt <<EOF
Verovio $verovio (https://www.verovio.org) engraves the notation of this page.

verovio-toolkit-wasm.js is the unchanged file dist/verovio-toolkit-wasm.js of the npm package
verovio@$verovio. Verovio is free software under the GNU Lesser General Public License, version 3
or later (COPYING.LESSER, which supplements the GNU General Public License in COPYING).
Its source code at this version: https://github.com/rism-digital/verovio/tree/version-$verovio
EOF
# Uploads keep the modes: readable for the web server whatever the umask.
find target/web -type d -exec chmod 755 {} + -o -type f -exec chmod 644 {} +
echo "Ready: python3 -m http.server -d target/web, then http://localhost:8000"
