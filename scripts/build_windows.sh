#!/usr/bin/env bash
# Cross-compiles the Windows .exe and writes its SHA-256 beside it.
#
# The .exe reaches the VM by hand: a share, a key, a remote desktop. Nothing on
# the way says the file that arrives is the one built here, so the hash is
# written next to it, to be read out on the VM with a tool Windows already has:
#
#   certutil -hashfile dbt-edith.exe SHA256
#
# The two must match before the file is run. Needs the mingw-w64 linker and
# windres (docs/windows.md).
set -euo pipefail
cd "$(dirname "$0")/.."

target=x86_64-pc-windows-gnu
cargo build --release --locked --target "$target"
# Where cargo put it, which a CARGO_TARGET_DIR moves: hashing target/ then
# would vouch for an older file.
exe="${CARGO_TARGET_DIR:-target}/$target/release/dbt-edith.exe"

# build.rs builds an .exe without its version resource rather than fail (0093),
# but this is the one taken to the VM, whose antivirus reads those properties.
windres=x86_64-w64-mingw32-windres
props=$("$windres" -J coff -O rc -i "$exe" 2>/dev/null || true)
field() { printf '%s\n' "$props" | sed -n "s/^ *VALUE \"$1\", \"\(.*\)\"\$/\1/p"; }
company=$(field CompanyName)
product=$(field ProductName)
version=$(field ProductVersion)
if [ -z "$company" ] || [ -z "$product" ] || [ -z "$version" ]; then
  rm -f "$exe.sha256"
  echo "$exe names no publisher or version in its properties: is $windres on PATH? (docs/windows.md)" >&2
  exit 1
fi

if command -v sha256sum >/dev/null; then
  hash=$(sha256sum "$exe" | cut -d' ' -f1)
else
  hash=$(shasum -a 256 "$exe" | cut -d' ' -f1)
fi
# The format `sha256sum -c` reads, with the bare file name, so the pair checks
# itself wherever it is copied together.
printf '%s  %s\n' "$hash" "dbt-edith.exe" > "$exe.sha256"

echo "built   $exe"
echo "props   $company, $product $version"
echo "sha256  $hash"
echo "        (also in $exe.sha256; on the VM: certutil -hashfile dbt-edith.exe SHA256)"
