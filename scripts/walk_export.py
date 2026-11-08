#!/usr/bin/env python3
"""An exported lineage with the Halloween theme on, driven in headless Chrome on Jaffle Shop.

The theme draws art on the lineage's boxes and cuts their names to the room it
leaves (0107). Neither may reach a file or a picture pasted in a ticket, which
`Lineage.snapshot()` draws, so this checks, for each of the three box styles:

  - the art, the dewdrops on the edges and the web on the selected box are on
    the canvas, so the check is not made against nothing;
  - what the snapshot writes is the same, character for character, as what it
    writes of the same canvas with the look off: no art, no `halloween`, no
    names cut short or squeezed to the look's fonts;
  - the page keeps its theme afterwards, <html>'s attribute and classes and
    the art;
  - the theme switched off with a box selected leaves no web, art or dewdrop on
    the canvas, and on again grows one web;
  - the file's colours are the theme on screen's, Night under Halloween (0150):
    the export's :root line holds the same --bg under Halloween as under Night,
    which is not the page's own, and under Light it holds Light's, its colour
    scheme included, with the snapshot still the plain canvas's;
  - the Export menu (0153) opens from the keyboard onto its first row, says what
    the canvas shows and names the file, steps with the arrows, wraps, closes on
    Escape with the focus back on its button, and each row saves what it says: an
    HTML page, a PNG with a PNG's first bytes, and a copy of the picture; in the
    Lineage space it names a selector's file.

Two models whose names the styles cut, and the plain look does not, are added
to the copy of Jaffle Shop.

It runs as scripts/walk_tour.py does, whose copy of Jaffle Shop, server and
DevTools client it borrows: it needs dbt to parse the copy, and Chrome, and
skips itself without either, exit 2.

Exit 0 when the exports match, 1 when they do not, 2 when skipped.
"""

import os
import re
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
THEME = "document.documentElement.dataset.theme"
LOOK_ON = f"{THEME} === 'halloween'"


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

    def theme(self, name):
        page = self.page
        if page.js(THEME) != name:
            page.js("document.querySelector('#theme-btn').click()")
            page.until(f"!!document.querySelector('.thememenu [data-key=theme-{name}]')", "the theme menu")
            page.js(f"document.querySelector('.thememenu [data-key=theme-{name}]').click()")
        self.settle(f"{THEME} === '{name}'", f"the {name} theme")

    def look(self, on):
        self.theme("halloween" if on else "night")

    def hook(self):
        """Records the blob a download is made of, and the name it is given."""
        self.page.js("window.__saved = null; window.__name = null; if (!window.__hooked) { window.__hooked = true;"
                     " const o = URL.createObjectURL; URL.createObjectURL = (b) => { window.__saved = b; return o.call(URL, b); };"
                     " const c = HTMLAnchorElement.prototype.click;"
                     " HTMLAnchorElement.prototype.click = function () { window.__name = this.download; return c.call(this); }; }")

    def pick(self, key):
        """Opens the Export menu with a click and picks one of its rows."""
        page = self.page
        page.js("document.querySelector('#export-btn').click()")
        page.until("!!document.querySelector('.exportmenu')", "the Export menu")
        page.js(f"document.querySelector('.exportmenu [data-key={key}]').click()")

    def toast(self):
        return self.page.js("document.querySelector('#toast .toast:last-child')?.textContent || ''")

    def exported(self):
        """The :root line of the file the Export menu saves as HTML: its --bg and its colour scheme."""
        page = self.page
        self.hook()
        self.pick("export-html")
        page.until("!!window.__saved", "the export")
        root = page.js("window.__saved.text().then((t) => (/:root \\{[^}]*\\}/.exec(t) || [''])[0])")
        bg = root.split("--bg: ")[1].split(";")[0] if "--bg: " in root else ""
        scheme = root.split("color-scheme: ")[1].split(";")[0] if "color-scheme: " in root else ""
        return bg, scheme

    def style(self, name):
        page = self.page
        page.js("document.querySelector('#settings-btn').click()")
        page.until("!!document.querySelector('#settings-nav')", "the settings")
        page.js("[...document.querySelectorAll('#settings-nav button')].find((b) => b.textContent.startsWith('Theme')).click()")
        page.until(f"!!document.querySelector('[data-key=look-boxes-{name}]')", "the Theme section")
        page.js(f"document.querySelector('[data-key=look-boxes-{name}]').click()")
        time.sleep(0.4)
        # A tab never toggles: Code is the way back.
        page.js("document.querySelector('#space-code').click()")
        self.settle(f"document.documentElement.classList.contains('hw-boxes-{name}')", f"the {name}")

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
            kept = page.js(f"{LOOK_ON} && document.documentElement.classList.contains('hw-boxes-{name}')"
                           " && document.querySelectorAll('.hw-art').length > 0")
            self.check("the page keeps its theme after it", kept)

        print("--- the look off and on again, a box selected ---")
        self.look(False)
        left = page.js("document.querySelectorAll('#graph .hw-web, #graph .hw-art, #graph .hw-bead').length")
        self.check("with the look off, no web, art or dewdrop is left on the canvas", left == 0, f"{left} left")
        self.look(True)
        self.check("and on again, the selected box grows one web", page.js("document.querySelectorAll('#graph .nd.sel > .hw-web').length") == 1)

        print("--- the colours of the file ---")
        self.look(False)
        night_bg, night_scheme = self.exported()
        self.look(True)
        halloween_bg, halloween_scheme = self.exported()
        own = page.js("getComputedStyle(document.documentElement).getPropertyValue('--bg').trim()")
        self.check("an export under Halloween holds Night's --bg", halloween_bg == night_bg and bool(night_bg), f"{halloween_bg!r} against {night_bg!r}")
        self.check("which is not the page's own", halloween_bg != own, f"both {own!r}")
        self.check("and Night's colour scheme", halloween_scheme == night_scheme == "dark", f"{halloween_scheme!r}, {night_scheme!r}")
        self.theme("light")
        light_bg, light_scheme = self.exported()
        self.check("under Light it holds Light's --bg and says light", (light_bg, light_scheme) == ("#fbfbfa", "light"), f"{light_bg!r}, {light_scheme!r}")
        self.check("and the snapshot is still the plain canvas's", page.js(SNAPSHOT) == plain)
        self.theme("night")
        self.menu()
        return self.failed

    def menu(self):
        page = self.page
        print("--- the menu ---")
        self.hook()
        self.check("there is no button of its own for the image, and the menu is shut",
                   page.js("!document.querySelector('#image-btn') && !document.querySelector('.exportmenu')"
                           " && document.querySelector('#export-btn').getAttribute('aria-expanded') === 'false'"))
        page.js("document.querySelector('#export-btn').focus()")
        page.key("Enter", "Enter", 13, text="\r")
        page.until("!!document.querySelector('.exportmenu')", "the menu, opened from the keyboard")
        here = page.js("""(() => {
          const menu = document.querySelector('.exportmenu').getBoundingClientRect();
          const button = document.querySelector('#export-btn').getBoundingClientRect();
          return {
            expanded: document.querySelector('#export-btn').getAttribute('aria-expanded'),
            focus: document.activeElement.dataset.key,
            head: document.querySelector('.exportmenu .menuhead').firstChild.textContent,
            n: document.querySelector('.exportmenu .menuhead .n').textContent,
            status: document.querySelector('#lineage-status').textContent,
            rows: [...document.querySelectorAll('.exportmenu .xm-item')].map((b) => b.textContent),
            foot: document.querySelector('.exportmenu .xm-foot')?.textContent || '',
            inside: menu.left >= 0 && menu.right <= innerWidth,
            aligned: Math.abs(menu.right - button.right) <= 1,
          };
        })()""")
        foot = here["foot"]
        self.check("Enter opens the menu, on its first row", here["expanded"] == "true" and here["focus"] == "export-copy", f"{here}")
        self.check("it says what the canvas shows, and how much", here["head"] == "What the canvas shows" and bool(here["n"])
                   and here["status"].startswith(here["n"]), f"{here['n']!r} against {here['status']!r}")
        self.check("with the three outputs, their formats and uses", here["rows"] == [
            "Copy as imagePNGOn the clipboard, to paste into a ticket or a chat",
            "Save as image.pngThe same picture, as a file",
            "Save as HTML page.htmlOpens offline, zooms without blurring, prints on one page"], f"{here['rows']}")
        self.check("and the name the files take", re.fullmatch(rf"lineage-{LONG}-\d{{4}}-\d{{2}}-\d{{2}}", foot) is not None, foot)
        self.check("held inside the window, from the button's right edge", here["inside"] and here["aligned"], f"{here}")
        for key, name in (("ArrowDown", "export-png"), ("ArrowDown", "export-html"), ("ArrowDown", "export-copy"), ("ArrowUp", "export-html")):
            page.key(key, key, 40 if key == "ArrowDown" else 38)
            got = page.js("document.activeElement.dataset.key")
            self.check(f"{key} reaches {name}", got == name, f"{got!r}")
        page.js("window.__saved = null; window.__name = null")
        page.key("Enter", "Enter", 13, text="\r")
        page.until("!!window.__saved", "the HTML page")
        kind = page.js("window.__saved.type")
        named = page.js("window.__name")
        self.check("Enter on Save as HTML page saves an HTML file named as the footer says",
                   kind.startswith("text/html") and named == foot + ".html", f"{kind!r}, {named!r}")
        self.check("and says so", self.toast() == f"Saved {foot}.html", self.toast())
        self.check("the menu is gone, the button has the focus back and is not expanded",
                   page.js("!document.querySelector('.exportmenu') && document.activeElement.id === 'export-btn'"
                           " && document.querySelector('#export-btn').getAttribute('aria-expanded') === 'false'"))
        page.key("Enter", "Enter", 13, text="\r")
        page.until("!!document.querySelector('.exportmenu')", "the menu, opened again")
        page.key(*W.ESCAPE)
        self.check("Escape closes it and gives the focus back",
                   page.js("!document.querySelector('.exportmenu') && document.activeElement.id === 'export-btn'"))
        page.js("window.__saved = null; window.__name = null")
        self.pick("export-png")
        page.until("!!window.__saved", "the PNG")
        kind = page.js("window.__saved.type")
        magic = page.js("window.__saved.slice(0, 4).arrayBuffer().then((b) => [...new Uint8Array(b)])")
        named = page.js("window.__name")
        self.check("Save as image saves a PNG, by its first bytes, named as the footer says",
                   kind == "image/png" and magic == [137, 80, 78, 71] and named == foot + ".png", f"{kind!r}, {magic}, {named!r}")
        page.until("document.querySelector('#toast .toast:last-child')?.textContent.startsWith('Saved ')", "its toast")
        self.check("and says so", self.toast() == f"Saved {foot}.png", self.toast())
        page.js("window.__saved = null; window.__name = null")
        self.pick("export-copy")
        # Headless Chrome may refuse the clipboard, which saves the file instead.
        page.until("/^(Image copied: paste it into the ticket|The clipboard took no image)/"
                   ".test(document.querySelector('#toast .toast:last-child')?.textContent || '')", "the copy's toast", 20)
        toast = self.toast()
        if toast.startswith("The clipboard took no image"):
            page.until("!!window.__saved", "the PNG the clipboard fell back to")
            kind = page.js("window.__saved && window.__saved.type")
            named = page.js("window.__name")
            self.check("when the clipboard takes no image, the PNG is saved instead, named as the footer says",
                       kind == "image/png" and named == foot + ".png", f"{kind!r}, {named!r}")
        else:
            self.check("Copy as image says the image is copied", toast.startswith("Image copied"), toast)

        print("--- the menu in the Lineage space ---")
        # The value goes in first: openSelection() seeds and runs an empty box, but keeps and runs one that holds something else.
        page.js("document.querySelector('#select-input').value = '--selector orders_eager'")
        page.js("document.querySelector('[data-mode=select]').click()")
        page.until("document.querySelector('#select-sum').textContent !== ''", "the selection")
        self.check("the button moved with the other tools", page.js("document.querySelector('#export-btn').parentNode.id") == "selpanel-tools")
        page.js("document.querySelector('#export-btn').click()")
        page.until("!!document.querySelector('.exportmenu')", "the menu in the Lineage space")
        selector_foot = page.js("document.querySelector('.exportmenu .xm-foot').textContent")
        self.check("it names a selector's file after the selector", selector_foot.startswith("lineage-selector-orders_eager-"), selector_foot)
        page.key(*W.ESCAPE)
        page.js("document.querySelector('#space-code').click()")


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
    print("an exported lineage is the same with the theme on")
    return 0


if __name__ == "__main__":
    sys.exit(main())
