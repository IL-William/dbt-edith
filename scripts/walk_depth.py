#!/usr/bin/env python3
"""The lineage's depth control, driven in headless Chrome on Jaffle Shop.

The depth is how far a walk on the canvas has looked around one model, written
as dbt writes a selector, `1+ customers +0`, in two hidden number boxes
(#up, #down) that a button and a menu of levels on each side of the name set
(0158). This checks the one rule focusNode keeps about it, against the
requests the page makes for the lineage, and the control itself:

  - the boxes start at 1 and 1;
  - a box double-clicked on the canvas keeps the depth, and a +N badge
    raises it and keeps it raised;
  - a model reached any other way, from Cmd/Ctrl + K, a tab, a ref() in the
    editor or a name in the Catalog's lists, is drawn at 1 and 1 again;
  - the Catalog's Upstream models and Downstream models set the box they name
    and leave the other, on a box clicked on the canvas too;
  - Back and Forward to another tab bring back the depth it was left with,
    whether the canvas moves or stays on the same model;
  - the same model drawn again, from the Lineage space and back, keeps it;
  - the top bar folds the search field first, then the project's name, then
    its tabs, and just above the width where each of the last two comes still
    fits and folds the same with the settings open and back on Code;
  - the control is driven by the keyboard alone: Enter on a number opens its
    menu on the level drawn, an arrow and Enter draw another (one request),
    Escape gives the focus back to the button, and Enter on the copy button
    writes the selector as shown;
  - a model with no children shows +0 and a disabled button while #down still
    holds 1, and says why; a model with children lists its levels;
  - a 40-character model name is cut in the middle in the control, whole in
    its title, and its menu's rows fit (a model added to this walk's own copy
    of Jaffle Shop, never to the fixture).

It runs as scripts/walk_tour.py does, whose copy of Jaffle Shop, server and
DevTools client it borrows: it needs dbt to parse the copy, and Chrome, and
skips itself without either, exit 2.

Exit 0 when the boxes behave, 1 when they do not, 2 when skipped.
"""

import os
import shutil
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

    def press(self, key):
        """A real key on whatever has the focus."""
        keys = {"Enter": ("Enter", "Enter", 13, "\r"), "Escape": ("Escape", "Escape", 27, None),
                "ArrowDown": ("ArrowDown", "ArrowDown", 40, None)}
        self.page.key(*keys[key])
        time.sleep(0.15)

    def menu_rows(self):
        return self.page.js("[...document.querySelectorAll('.depthmenu .lvrow')].map((r) => r.querySelector('.s').textContent)")

    def palette(self, name):
        page = self.page
        page.js("document.querySelector('#palette-btn').click()")
        page.js(f"(i => {{ i.value = '{name}'; i.dispatchEvent(new Event('input')); }})(document.querySelector('#palette-input'))")
        row = f"[...document.querySelectorAll('#palette-results .pal-row')].find((r) => r.querySelector('.pal-nm').textContent === '{name}')"
        page.until(f"!!{row}", f"{name} in the palette")
        page.js(f"{row}.click()")

    def box(self, model, event):
        self.page.js(f"document.querySelector('#graph .nd[data-id=\"model.jaffle_shop.{model}\"]')"
                     f".dispatchEvent(new MouseEvent('{event}', {{ bubbles: true }}))")

    def nav(self, which):
        self.page.js(f"document.querySelector('.crumb.nav[aria-label^={which}]').click()")

    def fold(self):
        """The top bar's fold (0140, 0166): the search field shrinks with the
        window down to 200px while the project's name keeps its width; then
        `nosearch` takes its words away; then the name is cut, never under
        120px; then `folded` keeps the tabs not lit to their glyph. Just above
        the width where each of the last two comes, opening the settings and
        coming back to Code leaves the bar inside the window and folded as it
        was."""
        page = self.page
        # Fits, classes, the manifest badge (whose age repaints and so refolds
        # the bar), the name's room, whether the name is cut, the search
        # field's width and whether its words show.
        state = """(b => [b.scrollWidth <= b.clientWidth, b.className,
          document.querySelector('#fresh-badge').textContent,
          document.querySelector('#project').clientWidth,
          document.querySelector('#project').scrollWidth > document.querySelector('#project').clientWidth,
          document.querySelector('#palette-btn').offsetWidth,
          getComputedStyle(document.querySelector('#palette-btn .ph')).display !== 'none'])(document.querySelector('#topbar'))"""

        def at(width):
            page.call("Emulation.setDeviceMetricsOverride", width=width, height=800, deviceScaleFactor=1, mobile=False)
            time.sleep(0.4)
            return page.js(state)

        wide = at(1400)
        self.check("wide, nothing folded and the search field shows its words", wide[1] == "" and wide[6], str(wide))

        def round_trip(width, what):
            """The settings opened and left for Code at `width`: the bar fits and
            keeps its classes the whole way. Read again when the badge's age
            repainted, and so refolded, between two reads."""
            for _ in range(3):
                before = at(width)
                page.js("document.querySelector('#settings-btn').click()")
                time.sleep(0.6)
                during = page.js(state)
                page.js("document.querySelector('#space-code').click()")
                time.sleep(0.6)
                after = page.js(state)
                if before[2] == during[2] == after[2]:
                    break
            self.check(f"{what}, at {width}px, the bar fits", before[0], str(before))
            self.check("  with the settings open it still fits, and folds as before",
                       during[0] and during[1] == before[1], f"{before}, then {during}")
            self.check("  and back on Code the same", after[0] and after[1] == before[1], f"{before}, then {after}")

        def first_below(cls, from_width):
            width = from_width
            while width > 600 and cls not in at(width)[1]:
                width -= 10
            self.check(f"the bar gets {cls} somewhere above 600px", width > 600, f"still without it at {width}")
            return width

        # Past each fold by 20px, so a badge repainting between two reads is the
        # only thing that can move it.
        round_trip(first_below("nosearch", 1400) + 20, "just above the search field losing its words")
        round_trip(first_below("folded", 1400) + 20, "just above the tabs folding")

        name = "a_long_invented_project_name_for_the_jaffle_shop_staging_and_marts_models"
        page.js(f"document.querySelector('#project').textContent = '{name}'")
        search_first, name_kept, search_min, name_cut, name_floor = True, True, True, False, True
        full = at(1400)[3]
        seen_nosearch = False
        for width in range(1400, 700, -10):
            now = at(width)
            if "nosearch" not in now[1]:
                # The field gives way alone: its words stay and it never goes under
                # 200px, and the name keeps the width it had in the wide bar.
                search_first = search_first and now[6]
                search_min = search_min and now[5] >= 200
                name_kept = name_kept and now[3] == full
            elif "folded" not in now[1]:
                seen_nosearch = True
                name_cut = name_cut or now[3] < full
                # Never cut under 120px while the tabs are still named.
                name_floor = name_floor and now[3] >= 120
        self.check("the search field shrinks to 200px, words and all, with the name as wide as ever", search_first and search_min and name_kept)
        self.check("then its words go (nosearch), and only then is a long name cut", seen_nosearch and name_cut)
        self.check("  down to 120px, never less, before the tabs fold", name_floor)
        page.js("document.querySelector('#project').textContent = 'jaffle_shop'")
        page.call("Emulation.clearDeviceMetricsOverride")

    def control(self):
        """The depth control, by the keyboard, on a model with no children, one
        with children and a name past 24 characters."""
        page = self.page
        q = "document.querySelector"
        long = "customer_orders_by_payment_methods_daily"
        cut = long[:12] + "\u2026" + long[-11:]
        # The clipboard is stubbed: a headless page may refuse the real one, and
        # what matters here is the text the button writes.
        page.js("window.__copied = ''; navigator.clipboard.writeText = (t) => { window.__copied = t; return Promise.resolve(); }")

        print("--- the control: a model with no children ---")
        self.palette("customers")
        self.drawn("customers at 1 and 1", "customers", "1/1")
        page.until(f"{q}('#up-btn b').textContent === '1' && {q}('#down-btn b').textContent === '0'", "the numbers shown")
        self.check("customers shows 1+ and +0 while #down still holds 1",
                   page.js(f"{q}('#up-btn b').textContent + '/' + {q}('#down-btn b').textContent + '/' + {q}('#down').value") == "1/0/1")
        self.check("the name sits between the buttons", page.js(f"{q}('#depth-name').textContent") == "customers")
        self.check("the empty side is aria-disabled, named none, and says why", page.js(
            f"(b => b.getAttribute('aria-disabled') + '|' + b.getAttribute('aria-label') + '|' + b.title)({q}('#down-btn'))")
            == "true|Levels downstream: none|customers has no children")
        self.check("the inputs are hidden, not gone", page.js(f"{q}('#up').hidden && {q}('#down').hidden"))
        page.js(f"{q}('#down-btn').click()")
        time.sleep(0.2)
        self.check("a click on the empty side opens nothing", page.js(f"!{q}('.depthmenu')"))

        print("--- the control: by the keyboard ---")
        page.js(f"{q}('#up-btn').focus()")
        self.press("Enter")
        page.until(f"!!{q}('.depthmenu')", "the up menu")
        self.check("Enter opens the menu", page.js(f"{q}('#up-btn').getAttribute('aria-expanded')") == "true")
        rows = self.menu_rows()
        self.check("it lists the focus alone, then the levels", rows[:3] == ["customers", "1+customers", "2+customers"], str(rows))
        self.check("the level drawn has the focus", page.js(
            f"document.activeElement.classList.contains('lvrow') && document.activeElement.getAttribute('aria-checked') === 'true'"
            f" && document.activeElement.querySelector('.s').textContent === '1+customers'"))
        self.check("its footer says nothing is downstream", page.js(f"{q}('.depthmenu .lvfoot').textContent") == "Nothing downstream: customers has no children.")
        self.check("each row says what it adds", page.js(
            "[...document.querySelectorAll('.depthmenu .lvrow .a')].map((a) => a.textContent)[0]") == "alone")
        self.press("Escape")
        self.check("Escape closes it and gives the button the focus", page.js(
            f"!{q}('.depthmenu') && document.activeElement === {q}('#up-btn') && {q}('#up-btn').getAttribute('aria-expanded') === 'false'"))
        self.press("Enter")
        page.until(f"!!{q}('.depthmenu')", "the up menu again")
        self.press("ArrowDown")
        self.check("an arrow moves to the next level", page.js("document.activeElement.querySelector('.s').textContent") == "2+customers")
        self.press("Enter")
        self.drawn("2+customers by the keyboard draws 2 and 1", "customers", "2/1")
        self.check("one pick, one request", page.js(
            "performance.getEntriesByType('resource').filter((e) => e.name.includes('/api/lineage?id=model.jaffle_shop.customers&up=2&down=1')).length") == 1)
        self.check("and the menu is gone", page.js(f"!{q}('.depthmenu')"))
        page.js(f"{q}('#up-btn').focus()")
        self.press("Enter")
        page.until(f"!!{q}('.depthmenu')", "the up menu on 2")
        self.press("Enter")
        time.sleep(0.4)
        self.check("the row already drawn only closes the menu", page.js(f"!{q}('.depthmenu') && {q}('#up').value") == "2")
        self.check("the copy button writes the selector as shown", page.js(f"{q}('#depth-copy').getAttribute('aria-label')") == "Copy the selector 2+customers")
        page.js(f"{q}('#depth-copy').focus()")
        self.press("Enter")
        page.until("window.__copied !== ''", "the copy")
        self.check("Enter on copy writes it, a zero side dropped", page.js("window.__copied") == "2+customers")
        self.check("and the toast says so", page.js("[...document.querySelectorAll('#toast .toast')].some((t) => t.textContent === 'Copied 2+customers')"))
        self.depth(1, 1)

        print("--- the control: a model with children ---")
        self.palette("orders")
        self.drawn("orders at 1 and 1", "orders", "1/1")
        self.depth(2, 1)
        page.until(f"{q}('#down-btn b').textContent === '1' && {q}('#up-btn b').textContent === '2'", "orders shown at 2 and 1")
        page.js(f"{q}('#depth-copy').focus()")
        page.js("window.__copied = ''")
        self.press("Enter")
        page.until("window.__copied !== ''", "the copy on orders")
        self.check("both sides in dbt's syntax", page.js("window.__copied") == "2+orders+1")
        self.check("the copy button's title stays the board's", page.js(f"{q}('#depth-copy').title") == "Copy the selector")
        page.js(f"{q}('#down-btn').click()")
        page.until(f"!!{q}('.depthmenu')", "the down menu")
        rows = self.menu_rows()
        self.check("the down menu lists orders+n", rows[:2] == ["orders", "orders+1"], str(rows))
        self.check("its head names the side and the model", page.js(f"{q}('.depthmenu .menuhead').textContent") == "Downstream of orders")
        self.check("and it stays in the window", page.js(
            f"(r => r.left >= 0 && r.right <= innerWidth)({q}('.depthmenu').getBoundingClientRect())"))
        self.check("a level that exists is never empty", all(a for a in page.js(
            "[...document.querySelectorAll('.depthmenu .lvrow .a')].map((a) => a.textContent)")))
        self.press("Escape")

        print("--- the control: a long name ---")
        self.palette(long)
        self.drawn(f"{long} at 1 and 1", long, "1/1")
        self.check("the name is cut in the middle, 24 characters", page.js(f"{q}('#depth-name').textContent") == cut and len(cut) == 24,
                   page.js(f"{q}('#depth-name').textContent"))
        self.check("its title keeps it whole", page.js(f"{q}('#depth-name').title") == long)
        self.check("it has no children: +0 with #down at 1", page.js(f"{q}('#down-btn b').textContent + '/' + {q}('#down').value") == "0/1")
        page.js(f"{q}('#up-btn').click()")
        page.until(f"!!{q}('.depthmenu')", "the long name's menu")
        self.check("the menu's head is cut and whole in its title", page.js(
            f"(h => h.textContent === 'Upstream of {cut}' && h.title === 'Upstream of {long}')({q}('.depthmenu .menuhead'))"))
        self.check("its rows fit", page.js(
            "[...document.querySelectorAll('.depthmenu .lvrow .s, .depthmenu .lvrow .a, .depthmenu .menuhead')]"
            ".every((e) => e.scrollWidth <= e.clientWidth)"),
            str(page.js("[...document.querySelectorAll('.depthmenu .lvrow .s, .depthmenu .lvrow .a')].map((e) => [e.textContent, e.scrollWidth, e.clientWidth])")))
        self.check("a row is named whole for a screen reader", page.js(
            f"document.querySelectorAll('.depthmenu .lvrow')[1].getAttribute('aria-label').startsWith('1+{long}, ')"))
        self.press("Escape")
        page.js(f"{q}('#depth-copy').focus()")
        page.js("window.__copied = ''")
        self.press("Enter")
        page.until("window.__copied !== ''", "the long name's copy")
        self.check("the copy keeps the name whole", page.js("window.__copied") == f"1+{long}")

        print("--- the control: a narrow dock ---")
        # What folded at each width, measured: the name first, then the tabs'
        # labels, then Export's.
        state = (f"[{q}('#lineage-tools').classList.contains('noname'), {q}('#dock > .tabs').classList.contains('folded'),"
                 f" {q}('#lineage-tools').classList.contains('iconic')].map(Number).join('')")
        seen = {}
        for width in range(1400, 520, -20):
            page.call("Emulation.setDeviceMetricsOverride", width=width, height=800, deviceScaleFactor=1, mobile=False)
            time.sleep(0.25)
            seen.setdefault(page.js(state), width)
        self.check("the name folds with the tabs still labelled, at a width only it had to give up",
                   "100" in seen, str(seen))
        self.check("the tabs never fold before the name does", not any(k[0] == "0" and k[1] == "1" for k in seen), str(seen))
        self.check("Export folds last", not any(k[2] == "1" and k[1] == "0" for k in seen), str(seen))
        if "100" in seen:
            page.call("Emulation.setDeviceMetricsOverride", width=seen["100"], height=800, deviceScaleFactor=1, mobile=False)
            time.sleep(0.4)
            self.check("there the name gives way to the bar of its kind", page.js(
                f"getComputedStyle({q}('#depth-name')).display === 'none' && getComputedStyle({q}('#depth-bar')).display === 'block'"
                f" && {q}('#depth-bar').getAttribute('aria-label') === '{long}'"))
            self.check("a bar and not a dot: 3 by 12", page.js(
                f"(r => Math.round(r.width) + 'x' + Math.round(r.height))({q}('#depth-bar').getBoundingClientRect())") == "3x12")
            self.check("the tabs keep their labels", page.js(
                f"[...document.querySelectorAll('#dock > .tabs .tab:not(.hidden) > span')].every((e) => e.getBoundingClientRect().width > 2)"))
            self.check("the control fits the row", page.js(
                f"(r => r.height <= {q}('#dock > .tabs').getBoundingClientRect().height)({q}('#depth-sel').getBoundingClientRect())"))
        # Wide enough for everything, the name is back and the tabs are labelled.
        page.call("Emulation.setDeviceMetricsOverride", width=2200, height=800, deviceScaleFactor=1, mobile=False)
        time.sleep(0.4)
        self.check("wide enough, the name is back", page.js(f"getComputedStyle({q}('#depth-name')).display !== 'none'"))
        page.call("Emulation.clearDeviceMetricsOverride")
        time.sleep(0.3)

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
        page.until("!document.querySelector('#select-bar').classList.contains('hidden')", "the Lineage space")
        # In its panel over the dock (0092): the Code tab is the way back (0140).
        page.js("document.querySelector('#space-code').click()")
        page.until("document.querySelector('#select-bar').classList.contains('hidden')", "Models")
        self.drawn("the Lineage space and back on orders, still 2 and 3", "orders", "2/3")

        self.control()

        print("--- the top bar's fold ---")
        self.fold()

        self.check("no error in the page", not page.errors, "; ".join(page.errors))
        return self.failed


def main():
    os.environ.setdefault("DBT_SEND_ANONYMOUS_USAGE_STATS", "False")
    os.environ.setdefault("DO_NOT_TRACK", "1")
    binary = W.edith_binary()
    with tempfile.TemporaryDirectory(prefix="dbt-edith-depth-") as tmp:
        work = Path(tmp)
        project = work / "jaffle_shop"
        shutil.copytree(W.JAFFLE, project)
        # A model with no children and a name of 40 characters, in this copy only.
        (project / "models" / "customer_orders_by_payment_methods_daily.sql").write_text(
            "select * from {{ ref('orders') }}\n")
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
    print("the depth control behaves")
    return 0


if __name__ == "__main__":
    sys.exit(main())
