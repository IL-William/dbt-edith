#!/usr/bin/env bash
# Cross-compiles the Windows .exe and writes its SHA-256 beside it.
#
# The .exe reaches the VM by hand: a share, a key, a remote desktop. Nothing on
# the way says the file that arrives is the one built here, so the hash is
# written next to it, to be read out on the VM with a tool Windows already has:
#
#   certutil -hashfile dbt-edith.exe SHA256
#
# The two must match before the file is run. Needs the mingw-w64 linker
# (README, "For a Windows machine").
set -euo pipefail
cd "$(dirname "$0")/.."

target=x86_64-pc-windows-gnu
cargo build --release --target "$target"
exe="target/$target/release/dbt-edith.exe"

if command -v sha256sum >/dev/null; then
  hash=$(sha256sum "$exe" | cut -d' ' -f1)
else
  hash=$(shasum -a 256 "$exe" | cut -d' ' -f1)
fi
# The format `sha256sum -c` reads, with the bare file name, so the pair checks
# itself wherever it is copied together.
printf '%s  %s\n' "$hash" "dbt-edith.exe" > "$exe.sha256"

echo "built   $exe"
echo "sha256  $hash"
echo "        (also in $exe.sha256; on the VM: certutil -hashfile dbt-edith.exe SHA256)"
