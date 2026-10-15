#!/usr/bin/env python3
"""The vendored frontend libraries against upstream, and against the OSV database.

cargo audit and Dependabot read Cargo.lock and the workflows, never web/vendor/,
so this is the only thing that looks there. Each file is compared, by SHA-256,
with the pin written here for the file of the upstream release it was taken
from; a file in the folder this script does not list fails; the version named
here has to be the one THIRD_PARTY_NOTICES.md names and, for CodeMirror, the
one codemirror.js declares. Online, the pins themselves are compared with the
npm tarball of that version, so a pin taken on a copied file, or a version
bumped without its files, fails too: the tarball is the only place a hash
is taken from, never the file under web/vendor/. The pins came from the
tarballs of CodeMirror 5.65.21, @xterm/xterm 5.5.0 and @xterm/addon-fit
0.10.0, and from the javascript/ build of diff-match-patch, which has no
release and is checked against nothing upstream (0085).

Then each library is asked about twice, as an npm package and as a tag of its
git repository, because advisories are filed either way: CVE-2025-6493 is
recorded against CodeMirror's commits, and an npm query never finds it. A clean
run means OSV maps nothing to these versions, not that there is nothing. OSV
lists CVE-2025-6493 up to CodeMirror 5.65.20, yet 5.65.21 still carries it: its
markdown.js is byte-identical. SECURITY.md is where accepted issues live.

Standard library only. Exit 0 when clean, 1 on an advisory not accepted, a
version that disagrees, a file or a pin that is not the upstream one or a file
not listed here, 2 when the files are in order and neither OSV nor the npm
registry could be reached.
"""

import hashlib
import http.client
import io
import json
import re
import sys
import tarfile
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
VENDOR = "web/vendor"

# What web/vendor/ holds. Updating a library means replacing its files, its
# version here and in THIRD_PARTY_NOTICES.md, and each file's hash, together:
# the checks below fail when any of them disagree. A file maps to the path it
# has in the upstream package and to its SHA-256 there, taken from the npm
# tarball of the version (`npm pack`, or the registry URL `upstream_hashes`
# builds), never on the copied file.
LIBRARIES = [
    {
        "name": "CodeMirror",
        "version": "5.65.21",
        "npm": "codemirror",
        "git": "https://github.com/codemirror/codemirror5",
        # The one vendored file that states its own version: ties the files
        # to the version the notices name and OSV is asked about.
        "declared_in": ("codemirror.js", 'CodeMirror.version = "{}"'),
        "files": {
            "codemirror.js": ("lib/codemirror.js", "e98aac5ffa07bae58acd4ff07c4293059f8921c0ae0eba506929d8c6f41c9288"),
            "codemirror.css": ("lib/codemirror.css", "eb494ea972d2661ef86f7f6ac656dd6786d721e49c9c1b46e1eb967e4b6f9bf3"),
            "cm-mode-sql.js": ("mode/sql/sql.js", "d5edb6bd39caa6155cab6bcdeb380a29229264a8a26e5fddd21ca12f0ebf359f"),
            "cm-mode-yaml.js": ("mode/yaml/yaml.js", "7de73109e5bfb6951d53764f5210f00f7859b57525811fab6b6f843980a7726e"),
            "cm-mode-markdown.js": ("mode/markdown/markdown.js", "8fe7b11848f79cc9119b2a239f38a00944f6f2bb764a00bc837ecf8d34cf7690"),
            "cm-mode-jinja2.js": ("mode/jinja2/jinja2.js", "037860e24dfa4c5a27ccc3d4a0383ac500332955aa336947f4133cffa57e4e13"),
            "cm-searchcursor.js": ("addon/search/searchcursor.js", "2c0b8cd8fe7ca5ca2253f4aacc914b390bff5b3cac3475df66c43a9ea8c8ccbe"),
            "cm-merge.js": ("addon/merge/merge.js", "e6bee9d4720f751038126809938c66199c7a5a4679b695b0db60302ef9dfbff2"),
            "cm-merge.css": ("addon/merge/merge.css", "059381d10112184efb5a90067c396fb24e257a97f0f1190de740a4eaebbfe2a3"),
        },
    },
    {
        "name": "xterm.js",
        "version": "5.5.0",
        "npm": "@xterm/xterm",
        "git": "https://github.com/xtermjs/xterm.js",
        "files": {
            "xterm.js": ("lib/xterm.js", "1f991ac3b4b283ebf96e60ae23a00a52765dd3a2e46fa6fdda9f1aab032f7495"),
            "xterm.css": ("css/xterm.css", "ba8e6985669488981ccf40c0cefe3aba80722cb6c92de7ad628b0bd717faf2b6"),
        },
    },
    {
        "name": "@xterm/addon-fit",
        "version": "0.10.0",
        "npm": "@xterm/addon-fit",
        "files": {
            "xterm-addon-fit.js": ("lib/addon-fit.js", "bdaefa370b1bfc42ee88d46fe6072400902a4d4b2d45cd93438dda9b23c97089"),
        },
    },
    {
        # Copied from the upstream javascript/ build, which has no release: every
        # advisory ever filed against the repository is reported.
        "name": "diff-match-patch",
        "version": None,
        "git": "https://github.com/google/diff-match-patch",
        "files": {
            "diff_match_patch.js": ("javascript/diff_match_patch_uncompressed.js", "9a79cf031ac7c2e366416181051acb3e6d2cacf79c5354148f4c71ea20c7e4a3"),
        },
    },
]

# Advisories judged acceptable, each explained in SECURITY.md under Known issues.
ACCEPTED = {"CVE-2025-6493"}


def osv(query):
    req = urllib.request.Request(
        "https://api.osv.dev/v1/query",
        data=json.dumps(query).encode(),
        headers={"Content-Type": "application/json"},
    )
    with urllib.request.urlopen(req, timeout=30) as res:
        return json.load(res).get("vulns", [])


def queries(lib):
    version = {"version": lib["version"]} if lib["version"] else {}
    if lib.get("npm"):
        yield f"npm {lib['npm']}", {**version, "package": {"name": lib["npm"], "ecosystem": "npm"}}
    if lib.get("git"):
        yield f"git {lib['git']}", {**version, "package": {"name": lib["git"], "ecosystem": "GIT"}}


def label(lib):
    return f"{lib['name']} {lib['version'] or '(unversioned)'}"


def named_with(notices, name, version):
    """Whether the notices' line naming the library carries this version as a
    whole: `5.5.0` must not pass on `15.5.0`, nor on another library's row."""
    return any(
        name in line and re.search(rf"(?<![\d.]){re.escape(version)}(?![\d.])", line)
        for line in notices.splitlines()
    )


def local_problems(lib, notices):
    """What is wrong with the library as it sits on disk: no network needed."""
    problems = []
    if lib["version"] and not named_with(notices, lib["name"], lib["version"]):
        problems.append("THIRD_PARTY_NOTICES.md names another version")
    if "declared_in" in lib:
        name, needle = lib["declared_in"]
        path = ROOT / VENDOR / name
        if path.is_file() and needle.format(lib["version"]) not in path.read_text(errors="replace"):
            problems.append(f"{VENDOR}/{name} does not declare this version")
    for name, (upstream, digest) in lib["files"].items():
        path = ROOT / VENDOR / name
        if not path.is_file():
            problems.append(f"{VENDOR}/{name} is missing")
        elif hashlib.sha256(path.read_bytes()).hexdigest() != digest:
            problems.append(f"{VENDOR}/{name} is not upstream's {upstream}, byte for byte")
    return problems


def upstream_hashes(lib):
    """The SHA-256 of each pinned file as it is in the npm tarball of this
    version, read from the registry and never written to disk."""
    package = lib["npm"]
    url = f"https://registry.npmjs.org/{package}/-/{package.rsplit('/', 1)[-1]}-{lib['version']}.tgz"
    with urllib.request.urlopen(url, timeout=60) as res:
        data = res.read()
    wanted = {f"package/{upstream}": name for name, (upstream, _) in lib["files"].items()}
    out = {}
    with tarfile.open(fileobj=io.BytesIO(data), mode="r:gz") as tar:
        for member in tar:
            if member.isfile() and member.name in wanted:
                out[wanted[member.name]] = hashlib.sha256(tar.extractfile(member).read()).hexdigest()
    return out


def pin_problems(lib):
    """The pins against upstream: a pin taken on a copied file, or a version
    bumped without its files, is caught here and nowhere else. Nothing for a
    library with no release to fetch."""
    if not lib.get("npm"):
        return []
    upstream = upstream_hashes(lib)
    problems = []
    for name, (path, digest) in lib["files"].items():
        if name not in upstream:
            problems.append(f"the npm tarball of {lib['version']} has no {path}")
        elif upstream[name] != digest:
            problems.append(f"the pin for {VENDOR}/{name} is not the SHA-256 of {path} in the npm tarball of {lib['version']}")
    return problems


def report(lib, found, unsound, complete):
    """Prints what OSV said about one library, and returns how many advisories
    are not accepted. `ok` only once both queries answered and the files are
    in order."""
    bad = 0
    for f in found:
        names = ", ".join(sorted(f["names"]))
        if f["names"] & ACCEPTED:
            print(f"known   {label(lib)}: {names}, accepted in SECURITY.md")
        else:
            print(f"FAILED  {label(lib)}: {names} via {f['source']}: {f['summary']}")
            bad += 1
    if complete and not found and lib["name"] not in unsound:
        print(f"ok      {label(lib)}")
    return bad


def main():
    notices = (ROOT / "THIRD_PARTY_NOTICES.md").read_text()
    security = (ROOT / "SECURITY.md").read_text()
    failed = 0

    for cve in sorted(ACCEPTED):
        if cve not in security:
            print(f"FAILED  {cve} is accepted here but not explained in SECURITY.md")
            failed += 1

    # The files first, printed once, so that an offline run still says whether
    # web/vendor/ holds what it should.
    unsound = set()
    for lib in LIBRARIES:
        for problem in local_problems(lib, notices):
            print(f"FAILED  {label(lib)}: {problem}")
            failed += 1
            unsound.add(lib["name"])
    listed = {name for lib in LIBRARIES for name in lib["files"]}
    for stray in sorted(p.name for p in (ROOT / VENDOR).iterdir()):
        # A hidden file is Finder's .DS_Store or an editor's swap file, never a
        # library: transient, ignored by git, and absent from a CI checkout.
        if stray.startswith(".") or stray in listed:
            continue
        print(f"FAILED  {VENDOR}/{stray} is not listed in this script, so nothing audits it")
        failed += 1

    for lib in LIBRARIES:
        # The same advisory can come back from both queries under two ids, a CVE
        # from one and its GHSA from the other: merged when their names overlap.
        found = []
        try:
            for problem in pin_problems(lib):
                print(f"FAILED  {label(lib)}: {problem}")
                failed += 1
                unsound.add(lib["name"])
            for source, query in queries(lib):
                for vuln in osv(query):
                    names = {vuln["id"], *vuln.get("aliases", [])}
                    same = next((f for f in found if f["names"] & names), None)
                    if same:
                        same["names"] |= names
                    else:
                        summary = (vuln.get("summary") or vuln.get("details") or "").strip()
                        first = (summary.splitlines() or [""])[0][:100]
                        found.append({"names": names, "source": source, "summary": first})
        except (OSError, http.client.HTTPException, json.JSONDecodeError, tarfile.TarError) as e:
            # OSError is a refused or reset connection, a timeout and a TLS
            # failure as well as URLError; HTTPException a server that hung up
            # mid-answer; TarError a tarball that is not one. Whatever was
            # found before this stands, the npm answer of a library whose git
            # query failed included.
            failed += report(lib, found, unsound, complete=False)
            print(f"cannot reach the npm registry or OSV ({e}): the files were checked against their pins, "
                  "the pins against upstream and the advisories were not")
            return 1 if failed else 2

        failed += report(lib, found, unsound, complete=True)

    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
