#!/usr/bin/env python3
"""The lineage's up and down boxes, driven in headless Chrome on Jaffle Shop.

The depth is how far a walk on the canvas has looked around one model. This
checks the one rule focusNode keeps about it, against the requests the page
makes for the lineage:

  - the boxes start at 1 and 1;
  - a box double-clicked on the canvas keeps the depth, and a +N badge
    raises it and keeps it raised;
  - a model reached any other way, from Cmd/Ctrl + K, a tab, a ref() in the
    editor or a name in the Catalog's lists, is drawn at 1 and 1 again;
  - the Catalog's Upstream models and Downstream models set the box they name
    and leave the other, on a box clicked on the canvas too;
  - Back and Forward to another tab bring back the depth it was left with,
    whether the canvas moves or stays on the same model;
  - the same model drawn again, from Custom selection and back, keeps it.

It runs as scripts/walk_tour.py does, whose copy of Jaffle Shop, server and
DevTools client it borrows: it needs dbt to parse the copy, and Chrome, and
skips itself without either, exit 2.

Exit 0 when the boxes behave, 1 when they do not, 2 when skipped.
"""

import os
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import walk_tour as W  # noqa: E402  skips, exit 2, without dbt or Chrome

# The model and the depth of the last lineage the page asked for.
LINEAGE = W.LINEAGE
DEPTH = "document.querySelector('#up').value + '/' + document.querySelector('#down').value"


class Walk:
    def __init__(self, page):
        self.page = page
        self.failed = 0

    def check(self, label, ok, detail=""):
        print(("PASS  " if ok else "FAIL  ") + label + ("" if ok else f"\n        {detail}"))
        self.failed += not ok

    def drawn(self, label, model, depth):
        """The lineage drawn on `model` at `depth`, in the boxes and in the
        request, once the page has asked for it."""
        want = f"model.jaffle_shop.{model} {depth}"
        try:
            self.page.until(f"({LINEAGE}) === '{want}' && ({DEPTH}) === '{depth}'", label, timeout=10)
        except AssertionError:
            pass
        got = f"{self.page.js(LINEAGE)}, boxes {self.page.js(DEPTH)}"
        self.check(label, got == f"{want}, boxes {depth}", f"want {want}, got {got}")
        time.sleep(0.3)

    def depth(self, up, down):
        # Both boxes, then one change: two would race two requests.
        self.page.js(f"(d => {{ document.querySelector('#up').value = '{up}'; d.value = '{down}';"
                     " d.dispatchEvent(new Event('change')); })(document.querySelector('#down'))")
        try:
            self.page.until(f"({LINEAGE}).endsWith(' {up}/{down}')", f"the lineage at {up}/{down}")
        except AssertionError as e:
            raise AssertionError(f"{e}: {self.page.js(LINEAGE)}, boxes {self.page.js(DEPTH)}") from None

    def palette(self, name):
        page = self.page
        page.js("document.querySelector('#palette-btn').click()")
        page.js(f"(i => {{ i.value = '{name}'; i.dispatchEvent(new Event('input')); }})(document.querySelector('#palette-input'))")
        row = f"[...document.querySelectorAll('#palette-results .pres')].find((r) => r.querySelector('.p1').textContent === '{name}')"
        page.until(f"!!{row}", f"{name} in the palette")
        page.js(f"{row}.click()")

    def box(self, model, event):
        self.page.js(f"document.querySelector('#graph .nd[data-id=\"model.jaffle_shop.{model}\"]')"
                     f".dispatchEvent(new MouseEvent('{event}', {{ bubbles: true }}))")

    def nav(self, which):
        self.page.js(f"document.querySelector('.crumb.nav[aria-label^={which}]').click()")

    def run(self, url):
        page = self.page
        page.call("Page.navigate", url=url)
        page.until("document.querySelectorAll('#tree .row').length > 0", "the page", 30)
        page.js("performance.setResourceTimingBufferSize(100000)")
        page.js("document.querySelectorAll('.tour-offer').forEach((o) => o.remove())")

        print("--- the canvas keeps it ---")
        self.check("the boxes start at 1 and 1", page.js(DEPTH) == "1/1", page.js(DEPTH))
        self.palette("customers")
        self.drawn("customers from Cmd+K, at 1 and 1", "customers", "1/1")
        self.depth(3, 2)
        self.box("stg_orders", "dblclick")
        self.drawn("stg_orders double-clicked on the canvas, still at 3 and 2", "stg_orders", "3/2")

        print("--- anything else starts again ---")
        self.palette("orders")
        self.drawn("orders from Cmd+K, back to 1 and 1", "orders", "1/1")
        page.js("document.querySelector('#graph .more-badge[data-dir=up]').dispatchEvent(new MouseEvent('click', { bubbles: true }))")
        self.drawn("an up +N badge on orders, 2 and 1", "orders", "2/1")

        print("--- back and forward ---")
        self.nav("Back")
        self.drawn("Back to stg_orders, at the 3 and 2 it was left with", "stg_orders", "3/2")
        self.nav("Back")
        self.drawn("Back to customers, at 3 and 2", "customers", "3/2")
        self.nav("Forward")
        self.drawn("Forward to stg_orders, at 3 and 2", "stg_orders", "3/2")
        self.nav("Forward")
        self.drawn("Forward to orders, at the 2 and 1 it was left with", "orders", "2/1")

        print("--- a tab, a link, the Catalog ---")
        page.js("[...document.querySelectorAll('#tabbar .ftab')].find((t) => t.textContent.startsWith('customers.sql')).click()")
        self.drawn("customers from its tab, at 1 and 1", "customers", "1/1")
        self.depth(3, 3)
        link = "[...document.querySelectorAll('.CodeMirror .cm-reflink')].find((s) => s.textContent.includes('stg_payments'))"
        page.until(f"!!{link}", "the stg_payments link in customers.sql")
        page.js(f"(s => {{ const r = s.getBoundingClientRect(); s.dispatchEvent(new MouseEvent('mousedown', "
                f"{{ bubbles: true, button: 0, clientX: r.left + r.width / 2, clientY: r.top + r.height / 2 }})); }})({link})")
        self.drawn("stg_payments from a ref() in customers.sql, at 1 and 1", "stg_payments", "1/1")
        self.nav("Back")
        self.drawn("Back to customers, at 3 and 3", "customers", "3/3")
        # A file with no node leaves the canvas on customers, at a depth
        # changed there: Back to customers.sql draws it at the one it left.
        self.palette("dbt_project.yml")
        page.until("document.querySelector('#tabbar .ftab.active')?.textContent.startsWith('dbt_project.yml')", "dbt_project.yml")
        self.depth(1, 1)
        self.nav("Back")
        self.drawn("Back to customers.sql, the canvas still on customers, at the 3 and 3 left there", "customers", "3/3")
        page.js("document.querySelector('[data-dock=catalog]').click()")
        catalog_link = "[...document.querySelectorAll('#catalog .links .link')].find((a) => a.textContent.includes('stg_orders'))"
        page.until(f"!!{catalog_link}", "the Catalog's upstream list")
        page.js(f"{catalog_link}.click()")
        self.drawn("stg_orders from the Catalog's upstream list, at 1 and 1", "stg_orders", "1/1")

        print("--- the Catalog's counts ---")
        page.js("document.querySelector('[data-dock=lineage]').click()")
        self.depth(3, 2)
        self.box("orders", "click")
        page.until("(document.querySelector('.cat-title')?.textContent || '').includes('orders')"
                   " && !(document.querySelector('.cat-title')?.textContent || '').includes('stg_')", "orders in the Catalog")
        page.js("document.querySelector('[data-dock=catalog]').click()")
        stat = "[...document.querySelectorAll('#catalog .stat')].find((s) => s.querySelector('.k').textContent === 'Upstream models')?.querySelector('.v')"
        page.until(f"!!{stat}", "the Upstream models count")
        page.js(f"{stat}.click()")
        self.drawn("Upstream models of a box clicked on the canvas: 1 up, the 2 down kept", "orders", "1/2")

        print("--- the same model ---")
        self.depth(2, 3)
        page.js("document.querySelector('[data-mode=select]').click()")
        page.until("!document.querySelector('#select-bar').classList.contains('hidden')", "Custom selection")
        # In its panel over the dock (0092): its close button is the way back.
        page.js("document.querySelector('#selpanel-close').click()")
        page.until("document.querySelector('#select-bar').classList.contains('hidden')", "Models")
        self.drawn("Custom selection and back on orders, still 2 and 3", "orders", "2/3")

        self.check("no error in the page", not page.errors, "; ".join(page.errors))
        return self.failed


def main():
    os.environ.setdefault("DBT_SEND_ANONYMOUS_USAGE_STATS", "False")
    os.environ.setdefault("DO_NOT_TRACK", "1")
    binary = os.environ.get("DBT_EDITH")
    if not binary:
        subprocess.run(["cargo", "build", "--quiet"], cwd=W.ROOT, check=True)
        binary = W.ROOT / "target" / "debug" / ("dbt-edith.exe" if os.name == "nt" else "dbt-edith")
    with tempfile.TemporaryDirectory(prefix="dbt-edith-depth-") as tmp:
        work = Path(tmp)
        project = work / "jaffle_shop"
        shutil.copytree(W.JAFFLE, project)
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
    print("the depth boxes behave")
    return 0


if __name__ == "__main__":
    sys.exit(main())
