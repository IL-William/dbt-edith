#!/usr/bin/env python3
"""Edith's menu under the pointer, driven in headless Chrome on Jaffle Shop.

The name at the top left opens its menu with no click (0086). This moves a
real pointer, through the DevTools protocol, and checks what that menu does:

  - resting on the name opens it, and leaves the focus in the editor;
  - crossing the name on the way elsewhere opens nothing;
  - it stays while the pointer is on the name or in it, and closes once the
    pointer has left both;
  - a key typed closes it, Escape included, and reaches the editor that has
    the focus; a modifier pressed alone leaves it open;
  - a click on the name with no menu open opens one, focused, a second
    closes it;
  - a click on the name, or in the menu, holds it open after the pointer
    leaves, the name's click giving it the focus, and the keys then reach
    it; Escape closes it;
  - a menu clicked open elsewhere, the GitHub one or a breadcrumb's, which
    keeps its own, is not swapped for or joined by Edith's when the pointer
    rests on the name.

It runs as scripts/walk_tour.py does, whose copy of Jaffle Shop, server and
DevTools client it borrows: it needs dbt to parse the copy, and Chrome, and
skips itself without either, exit 2.

Exit 0 when the menu behaves, 1 when it does not, 2 when skipped.
"""

import os
import shutil
import sys
import tempfile
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import walk_tour as W  # noqa: E402  skips, exit 2, without dbt or Chrome

OPEN = "!!document.querySelector('.updmenu')"
# Where the focus is: the menu, or the element that had it before.
FOCUS = ("(() => { const a = document.activeElement; if (a && a.closest('.updmenu')) return 'menu';"
         " return a ? a.tagName + (a.id ? '#' + a.id : '') : ''; })()")
# Longer than MENU_REST and MENU_GRACE in web/app.js, with room for a slow run.
PAST = 0.8
# A pointer on the name and off it within this proves the rest is waited for,
# with room under MENU_REST for the second move to land.
QUICK = 0.1
# The server runs with --no-update-check, which leaves the menu nothing to
# focus: a focus check would then compare the name with itself. The page is
# told the looks are on, so Check for updates is there to take the focus.
LOOKS_ON = """(() => { const f = window.fetch;
  window.fetch = async (...a) => {
    const r = await f(...a);
    if (!/\\/api\\/update(\\?|$)/.test(String(a[0])) || (a[1] && a[1].method)) return r;
    const u = await r.json();
    return new Response(JSON.stringify(Object.assign(u, { off: false })), { status: r.status, headers: r.headers });
  }; })()"""


class Walk:
    def __init__(self, page):
        self.page = page
        self.failed = 0

    def check(self, label, ok, detail=""):
        print(("PASS  " if ok else "FAIL  ") + label + ("" if ok else f"\n        {detail}"))
        self.failed += not ok

    def centre(self, selector):
        r = self.page.js(f"(r => [r.left + r.width / 2, r.top + r.height / 2])"
                         f"(document.querySelector('{selector}').getBoundingClientRect())")
        return r[0], r[1]

    def move(self, at):
        self.page.call("Input.dispatchMouseEvent", type="mouseMoved", x=at[0], y=at[1])

    def click(self, at):
        self.move(at)
        for kind in ("mousePressed", "mouseReleased"):
            self.page.call("Input.dispatchMouseEvent", type=kind, x=at[0], y=at[1], button="left", clickCount=1)

    def opens(self, label):
        try:
            self.page.until(OPEN, label, timeout=3)
        except AssertionError:
            pass
        self.check(label, self.page.js(OPEN))

    def closes(self, label):
        try:
            self.page.until("!" + OPEN, label, timeout=3)
        except AssertionError:
            pass
        self.check(label, not self.page.js(OPEN))

    def stays(self, label):
        time.sleep(PAST)
        self.check(label, self.page.js(OPEN))

    def crossing(self, brand, away):
        """Whether a pointer on the name and off it again within QUICK opened
        the menu, or None when no pair of moves was that quick: a slow one
        proves nothing on a loaded machine, and is tried again. A menu opened
        on the way would be gone again MENU_GRACE after the pointer, so an
        observer notes it the moment it is added."""
        for _ in range(3):
            self.page.js("window.menuSeen = false; window.menuWatch = new MutationObserver(() => {"
                         " if (document.querySelector('.updmenu')) window.menuSeen = true; });"
                         " window.menuWatch.observe(document.body, { childList: true }); true")
            start = time.monotonic()
            self.move(brand)
            self.move(away)
            quick = time.monotonic() - start < QUICK
            time.sleep(PAST)
            opened = self.page.js("window.menuWatch.disconnect(); window.menuSeen")
            if quick:
                return opened
            if opened:
                self.page.until("!" + OPEN, "the menu closing again", timeout=3)
        return None

    def run(self, url):
        page = self.page
        page.call("Page.addScriptToEvaluateOnNewDocument", source=LOOKS_ON)
        page.call("Page.navigate", url=url)
        page.until("document.querySelectorAll('#tree .row').length > 0", "the page", 30)
        page.js("document.querySelectorAll('.tour-offer').forEach((o) => o.remove())")
        away = self.centre("#graph")
        self.move(away)
        brand = self.centre("#brand")
        # The editor's focus, which the menu must leave alone.
        page.js("[...document.querySelectorAll('#tree .row')].find((r) => r.textContent.includes('dbt_project.yml')).click()")
        page.until("!!document.querySelector('.CodeMirror')?.CodeMirror?.getValue()", "dbt_project.yml in the editor")
        page.js("(cm => { cm.setCursor(0, 0); cm.focus(); })(document.querySelector('.CodeMirror').CodeMirror)")
        was = page.js(FOCUS)

        print("--- under the pointer ---")
        self.check("the name has no tooltip to land on the menu", page.js("!document.querySelector('#brand').title"))
        crossed = self.crossing(brand, away)
        if crossed is None:
            print(f"NOTE  crossing the name: no pair of moves under {QUICK * 1000:.0f} ms, so not checked")
        else:
            self.check("crossing the name opens nothing", not crossed)
        self.move(brand)
        self.opens("resting on the name opens the menu")
        self.check("and leaves the focus in the editor", was == "TEXTAREA" and page.js(FOCUS) == was,
                   f"{was} before, {page.js(FOCUS)} after")
        self.check("which Check for updates would otherwise take", page.js("!!document.querySelector('.updmenu .updcheck')"))
        self.move(self.centre(".updmenu"))
        self.stays("the pointer moved into the menu keeps it")
        self.move(brand)
        self.stays("and back on the name")
        self.move(away)
        self.closes("the pointer gone from both closes it")

        print("--- keys ---")
        self.move(brand)
        self.opens("open again under the pointer")
        page.call("Input.dispatchKeyEvent", type="rawKeyDown", key="Meta", code="MetaLeft",
                  windowsVirtualKeyCode=91, nativeVirtualKeyCode=91, modifiers=4)
        time.sleep(0.2)
        self.check("a modifier pressed alone leaves it open, for a Cmd/Ctrl + click", page.js(OPEN))
        page.call("Input.dispatchKeyEvent", type="keyUp", key="Meta", code="MetaLeft",
                  windowsVirtualKeyCode=91, nativeVirtualKeyCode=91)
        page.key("x", "KeyX", 88, "x")
        self.closes("a key typed closes it")
        self.check("and goes on to the editor", page.js("document.querySelector('.CodeMirror').CodeMirror.getValue().startsWith('x')"))
        self.move(away)
        self.move(brand)
        self.opens("open again")
        page.js("document.querySelector('.CodeMirror textarea').addEventListener('keydown',"
                " (e) => { if (e.key === 'Escape') window.escapeReached = true; }); true")
        page.key(*W.ESCAPE)
        self.closes("Escape closes it")
        self.check("and goes on to the editor too, as vim in the terminal wants it", page.js("window.escapeReached === true"))
        self.move(away)

        print("--- a click alone ---")
        self.move(away)
        self.click(brand)
        self.check("a click on the name opens it at once", page.js(OPEN))
        self.check("focused, as a clicked menu is", page.js(FOCUS) == "menu", page.js(FOCUS))
        self.click(brand)
        self.closes("a second click closes it")
        self.move(away)

        print("--- held by a click ---")
        self.move(brand)
        self.opens("open under the pointer")
        self.click(brand)
        self.check("a click on the name keeps it open", page.js(OPEN))
        self.check("and gives it the focus, as a click that opened it would", page.js(FOCUS) == "menu", page.js(FOCUS))
        self.move(away)
        self.stays("held, the pointer leaving does not close it")
        page.key("x", "KeyX", 88, "x")
        self.check("and a key typed no longer closes it", page.js(OPEN))
        page.key(*W.ESCAPE)
        self.closes("Escape closes the held menu")
        self.move(brand)
        self.opens("open under the pointer")
        self.click(self.centre(".updmenu .menuhead"))
        self.move(away)
        self.stays("a click in the menu holds it too")
        self.click(away)
        self.closes("a click elsewhere closes it")
        self.move(away)

        print("--- another menu open ---")
        self.click(self.centre("#github-btn"))
        page.until("!!document.querySelector('.ghmenu')", "the GitHub menu")
        self.move(brand)
        time.sleep(PAST)
        self.check("a pointer on the name does not swap a menu clicked open",
                   page.js("!!document.querySelector('.ghmenu') && !document.querySelector('.updmenu')"))
        page.key(*W.ESCAPE)
        page.until("!document.querySelector('.ghmenu')", "the GitHub menu closed")
        self.move(away)
        crumb = ("[...document.querySelectorAll('#crumbs .crumb')]"
                 ".find((b) => b.textContent.includes('dbt_project.yml'))")
        at = page.js(f"(r => [r.left + r.width / 2, r.top + r.height / 2])({crumb}.getBoundingClientRect())")
        self.click(at)
        page.until("!!document.querySelector('.crumbmenu')", "the breadcrumb's menu")
        self.move(brand)
        time.sleep(PAST)
        self.check("nor joins a breadcrumb's menu, which keeps its own",
                   page.js("!!document.querySelector('.crumbmenu') && !document.querySelector('.updmenu')"))

        self.check("no error in the page", not page.errors, "; ".join(page.errors))
        return self.failed


def main():
    os.environ.setdefault("DBT_SEND_ANONYMOUS_USAGE_STATS", "False")
    os.environ.setdefault("DO_NOT_TRACK", "1")
    binary = W.edith_binary()
    with tempfile.TemporaryDirectory(prefix="dbt-edith-menu-") as tmp:
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
    print("Edith's menu behaves under the pointer")
    return 0


if __name__ == "__main__":
    sys.exit(main())
