#!/usr/bin/env python3
"""An exported lineage with the Halloween look on, driven in headless Chrome on Jaffle Shop.

The look draws art on the lineage's boxes and cuts their names to the room it
leaves (0107). Neither may reach a file or a picture pasted in a ticket, which
`Lineage.snapshot()` draws, so this checks, for each of the three box styles:

  - the art, the dewdrops on the edges and the web on the selected box are on
    the canvas, so the check is not made against nothing;
  - what the snapshot writes is the same, character for character, as what it
    writes of the same canvas with the look off: no art, no `halloween`, no
    names cut short or squeezed to the look's fonts;
  - the page keeps its look afterwards, the body's classes and the art;
  - the look switched off with a box selected leaves no web, art or dewdrop on
    the canvas, and on again grows one web.

Two models whose names the styles cut, and the plain look does not, are added
to the copy of Jaffle Shop.

It runs as scripts/walk_tour.py does, whose copy of Jaffle Shop, server and
DevTools client it borrows: it needs dbt to parse the copy, and Chrome, and
skips itself without either, exit 2.

Exit 0 when the exports match, 1 when they do not, 2 when skipped.
"""

import os
import shutil
import sys
import tempfile
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import walk_tour as W  # noqa: E402  skips, exit 2, without dbt or Chrome

STYLES = ["tombstone", "coffin", "grimoire"]
# Both short of the 27 characters the plain look keeps and past what each style
# keeps: the first by the coffin and the grimoire, the second by all three.
LONG = "int_payments_by_method"
LONGER = "int_payments_by_method_ab"
SNAPSHOT = "Lineage.snapshot({ css: 'x' }).svg"
LOOK_ON = "document.body.classList.contains('halloween')"


class Walk:
    def __init__(self, page):
        self.page = page
        self.failed = 0

    def check(self, label, ok, detail=""):
        print(("PASS  " if ok else "FAIL  ") + label + ("" if ok else f"\n        {detail}"))
        self.failed += not ok

    def settle(self, expr, what):
        self.page.until(expr, what, timeout=10)
        time.sleep(0.3)

    def look(self, on):
        if self.page.js(LOOK_ON) != on:
            self.page.js("document.querySelector('#look-btn').click()")
        self.settle(f"{LOOK_ON} === {str(on).lower()}", "the look")

    def style(self, name):
        page = self.page
        page.js("document.querySelector('#settings-btn').click()")
        page.until("!!document.querySelector('#settings-nav')", "the settings")
        page.js("[...document.querySelectorAll('#settings-nav button')].find((b) => b.textContent.startsWith('Theme')).click()")
        page.until(f"!!document.querySelector('[data-key=look-boxes-{name}]')", "the Theme section")
        page.js(f"document.querySelector('[data-key=look-boxes-{name}]').click()")
        time.sleep(0.4)
        page.js("document.querySelector('#settings-btn').click()")
        self.settle(f"document.body.classList.contains('hw-boxes-{name}')", f"the {name}")

    def run(self, url):
        page = self.page
        page.call("Page.navigate", url=url)
        page.until("document.querySelectorAll('#tree .row').length > 0", "the page", 30)
        page.js("document.querySelectorAll('.tour-offer').forEach((o) => o.remove())")
        page.js("[...document.querySelectorAll('#tree .row')].find((r) => r.dataset.path === 'models').click()")
        page.until(f"!!document.querySelector('#tree .row[data-path=\"models/{LONG}.sql\"]')", "the long model")
        page.js(f"document.querySelector('#tree .row[data-path=\"models/{LONG}.sql\"]')"
                ".dispatchEvent(new MouseEvent('dblclick', { bubbles: true }))")
        page.until(f"[...document.querySelectorAll('#graph .nd')].some((n) => n.dataset.id.endsWith('{LONG}'))", "its lineage", 20)
        time.sleep(1)

        print("--- the same file with the look off ---")
        self.look(False)
        plain = page.js(SNAPSHOT)
        self.check("a plain canvas exports, with no art in it", bool(plain) and "hw-art" not in plain)
        self.check("and the long names are there whole", LONG in plain and LONGER in plain)
        self.look(True)

        for name in STYLES:
            print(f"--- {name} ---")
            self.style(name)
            self.check("the art is on the canvas", page.js("document.querySelectorAll('.hw-art').length") > 0)
            self.check("and the dewdrops on its edges", page.js("document.querySelectorAll('#graph .hw-bead').length") > 0)
            self.check("and the web on the selected box, one", page.js("document.querySelectorAll('#graph .nd.sel > .hw-web').length") == 1)
            cut = page.js("[...document.querySelectorAll('#graph .nd:not(.rider) .t1')].some((t) => t.textContent.endsWith('…'))")
            self.check("and cuts the long name, which is what the export must undo", cut)
            themed = page.js(SNAPSHOT)
            first = next((i for i, (a, b) in enumerate(zip(themed, plain)) if a != b), None)
            if first is None and len(themed) != len(plain):
                first = min(len(themed), len(plain))
            where = "" if first is None else f"first difference at {first}: {themed[first - 40:first + 60]!r} against {plain[first - 40:first + 60]!r}"
            self.check("the export is the plain canvas's, character for character", themed == plain, where)
            self.check("nothing of the look is in it", all(w not in themed for w in ("hw-art", "hw-on", "halloween", "hw-shape", "hw-bead", "hw-web")))
            kept = page.js("document.body.classList.contains('halloween')"
                           f" && document.body.classList.contains('hw-boxes-{name}') && document.querySelectorAll('.hw-art').length > 0")
            self.check("the page keeps its look after it", kept)

        print("--- the look off and on again, a box selected ---")
        self.look(False)
        left = page.js("document.querySelectorAll('#graph .hw-web, #graph .hw-art, #graph .hw-bead').length")
        self.check("with the look off, no web, art or dewdrop is left on the canvas", left == 0, f"{left} left")
        self.look(True)
        self.check("and on again, the selected box grows one web", page.js("document.querySelectorAll('#graph .nd.sel > .hw-web').length") == 1)
        return self.failed


def main():
    os.environ.setdefault("DBT_SEND_ANONYMOUS_USAGE_STATS", "False")
    os.environ.setdefault("DO_NOT_TRACK", "1")
    binary = W.edith_binary()
    with tempfile.TemporaryDirectory(prefix="dbt-edith-export-") as tmp:
        work = Path(tmp)
        project = work / "jaffle_shop"
        shutil.copytree(W.JAFFLE, project)
        (project / "models" / f"{LONG}.sql").write_text("select * from {{ ref('stg_payments') }}\n")
        (project / "models" / f"{LONGER}.sql").write_text(f"select * from {{{{ ref('{LONG}') }}}}\n")
        W.parse(project)
        (work / "home").mkdir()
        edith = W.Edith(binary, project, work)
        page = None
        try:
            page = W.Page(work, 1)
            failed = Walk(page).run(edith.url)
        finally:
            if page:
                page.close()
            edith.stop()
    if failed:
        print(f"{failed} check(s) failed")
        return 1
    print("an exported lineage is the same with the look on")
    return 0


if __name__ == "__main__":
    sys.exit(main())
