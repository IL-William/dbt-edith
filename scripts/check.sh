#!/usr/bin/env bash
# Everything that has to pass before a change is done: Rust tests, the release
# notes, both audits, the Snowflake script's tests, the selection engine against
# dbt itself, the guided tour walked in Chrome, the browser harnesses, and a
# syntax check on the frontend the harnesses cannot catch.
#
# JavaScriptCore ships with macOS. Elsewhere, set JSC to a JavaScript shell
# (jsc, d8, node) or accept that the browser half is skipped.
set -uo pipefail
cd "$(dirname "$0")/.."

JSC=${JSC:-/System/Library/Frameworks/JavaScriptCore.framework/Versions/A/Helpers/jsc}
failed=0
pass=0
# A part that cannot run here skips itself and the run still passes, so the
# last line names each one: "all good" alone read as if everything had run.
skipped=""
skip() { skipped="${skipped:+$skipped, }$1"; }

echo "== cargo test =="
# --locked: the lockfile is what every install builds (0085), so a test that
# would have rewritten it fails here instead of at the next install.
if cargo test --quiet --locked; then
  echo "rust ok"
else
  echo "RUST FAILED"
  failed=$((failed + 1))
fi

echo
echo "== release notes =="
# Merging a version bump releases it with its CHANGELOG.md section (0045), so
# the changelog has to be at the version Cargo.toml is at.
if scripts/release_notes.sh > /dev/null; then
  echo "release notes ok"
else
  echo "RELEASE NOTES FAILED"
  failed=$((failed + 1))
fi

echo
echo "== cargo audit =="
# The lockfile against the RustSec advisory database. Optional here, like jsc,
# because it needs a crate installed and the network; CI runs it every time.
# A yanked version is refused, not warned about: --locked still builds it,
# where an unlocked resolve would have moved on (0085).
if cargo audit --version > /dev/null 2>&1; then
  if cargo audit --quiet --deny yanked; then
    echo "audit ok"
  else
    echo "AUDIT FAILED"
    failed=$((failed + 1))
  fi
else
  echo "SKIPPED: no cargo-audit. Install it with: cargo install cargo-audit --locked"
  skip "cargo audit"
fi

echo
echo "== vendored libraries =="
# web/vendor/ against its upstream releases by hash, then against OSV, which
# neither cargo audit nor Dependabot reads. Needs python3; without the network
# the hashes are still checked and only the advisories are skipped.
if ! command -v python3 > /dev/null 2>&1; then
  echo "SKIPPED: no python3"
  skip "vendored libraries"
else
  python3 scripts/audit_vendored.py
  case $? in
    0) echo "vendored ok" ;;
    2) echo "vendored files ok; SKIPPED the advisories: OSV unreachable"; skip "OSV advisories" ;;
    *) echo "VENDORED FAILED"; failed=$((failed + 1)) ;;
  esac
fi

echo
echo "== snowflake script =="
# tools/sf_lineage.py against a fake connector and a fake PyYAML, so it needs no
# warehouse and nothing installed, only a Python recent enough for the script.
if ! python3 -c 'import sys; sys.exit(sys.version_info < (3, 10))' > /dev/null 2>&1; then
  echo "SKIPPED: no python3 at 3.10 or later"
  skip "snowflake script"
else
  out=$(python3 tools/test_sf_lineage.py 2>&1)
  if [ $? -eq 0 ]; then
    echo "snowflake script ok ($(printf '%s\n' "$out" | sed -n 's/^Ran \([0-9]*\) tests.*/\1/p') tests)"
  else
    printf '%s\n' "$out" | tail -30
    echo "SNOWFLAKE SCRIPT FAILED"
    failed=$((failed + 1))
  fi
fi

echo
echo "== dbt itself =="
# The Custom selection box and the lineage against `dbt ls`, on tests/fixtures/jaffle_shop
# and on Fivetran's Shopify, cloned at a pinned tag (0033, 0056). It needs dbt-core and
# dbt-duckdb, so it skips itself without them, and Shopify without the network; CI runs
# it on every model for two versions of dbt-core, here every eighth model of Shopify.
# DBT_PYTHON names a Python that has them, and DBT a dbt executable to run instead,
# dbt Fusion included.
py=${DBT_PYTHON:-python3}
if ! command -v "$py" > /dev/null 2>&1; then
  echo "SKIPPED: no $py"
  skip "dbt itself"
else
  out=$("$py" scripts/compare_with_dbt.py 2>&1)
  case $? in
    0) echo "dbt ok ($(printf '%s\n' "$out" | head -1 | cut -d, -f1): $(printf '%s\n' "$out" | tail -1))" ;;
    # Exit 2 also means Shopify alone was skipped, Jaffle Shop having agreed.
    2) printf '%s\n' "$out" | tail -1
       case $(printf '%s\n' "$out" | tail -1) in
         "SKIPPED: Shopify"*) skip "Shopify against dbt" ;;
         *) skip "dbt itself" ;;
       esac ;;
    *) printf '%s\n' "$out" | grep -v '^PASS' | tail -30; echo "DBT COMPARISON FAILED"; failed=$((failed + 1)) ;;
  esac
fi

echo
echo "== the tour in Chrome =="
# Every step of the guided tour, walked in headless Chrome on a parsed copy of
# Jaffle Shop (0074). It needs the same dbt as the step above to parse it, and
# Chrome, and skips itself without either.
if ! command -v "$py" > /dev/null 2>&1; then
  echo "SKIPPED: no $py"
  skip "the tour"
else
  out=$("$py" scripts/walk_tour.py 2>&1)
  case $? in
    0) echo "tour ok ($(printf '%s\n' "$out" | grep -c '^PASS') checks)" ;;
    2) printf '%s\n' "$out" | tail -1; skip "the tour" ;;
    *) printf '%s\n' "$out" | grep -v '^PASS' | tail -30; echo "TOUR FAILED"; failed=$((failed + 1)) ;;
  esac
fi

echo
echo "== the depth boxes in Chrome =="
# What keeps the lineage's depth and what starts it again, driven the same way
# on the same Jaffle Shop, and skipped the same way.
if ! command -v "$py" > /dev/null 2>&1; then
  echo "SKIPPED: no $py"
  skip "the depth boxes"
else
  out=$("$py" scripts/walk_depth.py 2>&1)
  case $? in
    0) echo "depth ok ($(printf '%s\n' "$out" | grep -c '^PASS') checks)" ;;
    2) printf '%s\n' "$out" | tail -1; skip "the depth boxes" ;;
    *) printf '%s\n' "$out" | grep -v '^PASS' | tail -30; echo "DEPTH FAILED"; failed=$((failed + 1)) ;;
  esac
fi

echo
echo "== Edith's menu in Chrome =="
# The menu the name at the top left opens under the pointer (0086), driven by
# a real pointer the same way, and skipped the same way.
if ! command -v "$py" > /dev/null 2>&1; then
  echo "SKIPPED: no $py"
  skip "Edith's menu"
else
  out=$("$py" scripts/walk_menu.py 2>&1)
  case $? in
    # A NOTE is a check the machine was too slow to make, which a count alone would hide.
    0) echo "menu ok ($(printf '%s\n' "$out" | grep -c '^PASS') checks)"
       if printf '%s\n' "$out" | grep '^NOTE'; then skip "a timing check of Edith's menu"; fi ;;
    2) printf '%s\n' "$out" | tail -1; skip "Edith's menu" ;;
    *) printf '%s\n' "$out" | grep -v '^PASS' | tail -30; echo "MENU FAILED"; failed=$((failed + 1)) ;;
  esac
fi

echo
echo "== an export with the Halloween look on, in Chrome =="
# What Lineage.snapshot() writes of a canvas drawn with each box style, against
# the same canvas without the look (0107), driven the same way, and skipped the
# same way.
if ! command -v "$py" > /dev/null 2>&1; then
  echo "SKIPPED: no $py"
  skip "the export with the look on"
else
  out=$("$py" scripts/walk_export.py 2>&1)
  case $? in
    0) echo "export ok ($(printf '%s\n' "$out" | grep -c '^PASS') checks)" ;;
    2) printf '%s\n' "$out" | tail -1; skip "the export with the look on" ;;
    *) printf '%s\n' "$out" | grep -v '^PASS' | tail -30; echo "EXPORT FAILED"; failed=$((failed + 1)) ;;
  esac
fi

echo
echo "== browser tests =="
if [ ! -x "$JSC" ] && ! command -v "$JSC" > /dev/null 2>&1; then
  echo "SKIPPED: no JavaScript shell at $JSC. Set JSC to one to run these."
  skip "browser tests and the frontend parse"
else
  # A harness slices web/app.js between two function names. A rename that breaks
  # a slice shows up here as an error, not as a quietly empty test file.
  for t in web/tests/*.js; do
    # A harness that throws stops where it stands, prints no FAIL and exits
    # non-zero: without the exit code, the PASS lines before it would vouch
    # for the whole file. A rejected promise does neither, so tabs.js, the one
    # harness built on promises, prints its own FAIL.
    out=$("$JSC" "$t" 2>&1)
    code=$?
    n=$(printf '%s\n' "$out" | grep -c '^PASS' || true)
    bad=$(printf '%s\n' "$out" | grep -c 'FAIL' || true)
    if [ "$code" -ne 0 ] || [ "$bad" -gt 0 ] || [ "$n" -eq 0 ]; then
      echo "FAILED  $t"
      printf '%s\n' "$out" | grep -v '^PASS' | head -20
      failed=$((failed + 1))
    else
      echo "ok      $t ($n)"
      pass=$((pass + n))
    fi
  done

  echo
  echo "== the frontend parses =="
  # Whole files, because a harness reads only slices of them: a syntax error
  # outside every slice would pass them all.
  syntax=$(mktemp)
  for f in web/app.js web/lineage.js; do
    printf "try { new Function(read('%s')); print('ok'); }\ncatch (e) { print('SYNTAX ERROR ' + e); }\n" "$f" > "$syntax"
    if [ "$("$JSC" "$syntax")" = "ok" ]; then
      echo "ok      $f"
    else
      "$JSC" "$syntax"
      failed=$((failed + 1))
    fi
  done
  rm -f "$syntax"
fi

echo
if [ "$failed" -eq 0 ] && [ -z "$skipped" ]; then
  echo "all good: $pass browser assertions, plus the Rust suite"
elif [ "$failed" -eq 0 ]; then
  echo "passed, but skipped: $skipped ($pass browser assertions, plus the Rust suite)"
else
  echo "$failed check(s) failed${skipped:+, and skipped: $skipped}"
fi
exit $((failed > 0))
