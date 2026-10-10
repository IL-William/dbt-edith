#!/usr/bin/env bash
# Prints the notes of the version Cargo.toml names: its section of CHANGELOG.md,
# which is what its GitHub release says (0045).
#
# Fails unless the newest dated section of CHANGELOG.md is that version's and
# has something in it, so a bump that left Unreleased where it was, or a
# changelog moved on to a version Cargo.toml does not carry, stops at CI rather
# than at the release.
set -euo pipefail
cd "$(dirname "$0")/.."

# The first `version =` at the start of a line is [package]'s: that table comes
# first, and every dependency's version sits inline after its name.
version=$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -n 1)
if [ -z "$version" ]; then
  echo "no version in Cargo.toml" >&2
  exit 1
fi

# A dated heading is `## 0.9.0 - 2026-10-05`. Unreleased has no date, so it is
# skipped; the first dated one has to be this version, and the lines down to
# the next heading are its notes, blank lines trimmed at both ends.
notes=$(awk -v want="$version" '
  function dated(line,   rest) {
    if (substr(line, 1, 3) != "## ") return ""
    rest = substr(line, 4)
    if (rest !~ / - [0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]$/) return ""
    return substr(rest, 1, length(rest) - 13)
  }
  !found && dated($0) != "" {
    if (dated($0) != want) {
      print "CHANGELOG.md is at " dated($0) " and Cargo.toml at " want > "/dev/stderr"
      bad = 1
      exit
    }
    found = 1
    next
  }
  found && substr($0, 1, 3) == "## " { exit }
  found { lines[++n] = $0 }
  END {
    if (bad) exit 1
    if (!found) {
      print "CHANGELOG.md has no \"## " want " - <date>\": a version bump turns Unreleased into it" > "/dev/stderr"
      exit 1
    }
    first = 1
    while (first <= n && lines[first] ~ /^[ \t]*$/) first++
    while (n >= first && lines[n] ~ /^[ \t]*$/) n--
    for (i = first; i <= n; i++) print lines[i]
  }
' CHANGELOG.md)

if [ -z "$notes" ]; then
  echo "the $version section of CHANGELOG.md is empty" >&2
  exit 1
fi
printf '%s\n' "$notes"
