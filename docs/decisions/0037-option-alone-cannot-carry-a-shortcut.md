# 0037. Option alone cannot carry a shortcut, so the bound ones ask for Cmd

Date: 2026-10-01 · Status: accepted · Amends 0030

**Trigger:** read before binding `Alt + <letter>` on a Mac, or changing what
`keyCombo` does with `e.code`.

## Context

0030 ends by saying `keyCombo` reads the physical key when Option is held on a
Mac, "where Option turns W into `∑`". That is a US keyboard. On the French one
this tool was built for, neither `Alt + W` nor `Alt + Shift + W` can be typed at
all, and `Alt + B` and `Alt + N` wrote into the open file rather than moving.
Two reasons, both in play:

- **`e.code` names positions, not caps.** The key marked W sits at `KeyZ`, so it
  arrives as `Alt + Z`. The key at `KeyW` is the one marked Z.
- **Option composes.** At `KeyW` it writes `Â`, a letter, and `keyCombo` leaves
  Option plus a letter as typed, since that is someone writing. The guard is
  right, and its comment already named `Â`.

Cmd does not stop the composing: `Cmd + Option + S` still reports `∑`. What it
does is let `keyCombo` fall back to `e.code`, which is why `Mod + Alt + S` has
always worked. That is only sound for a letter in the same place on both
layouts, which rules out A, Q, Z, W and M.

## Decision

A shortcut the app binds carries Cmd and a position-stable letter: back and
forward are `Mod + Alt + P` and `Mod + Alt + N`, closing a tab `Mod + Alt + X`
and closing every tab `Mod + Alt + Shift + X`, X reading as the cross on the
tab. `Alt + W` and `Alt + Shift + W` stay bound for the keyboards they reach,
and the note under the shortcut list says which keys work where.

## Rejected

- **Remapping whenever Option is held**, dropping the letter guard. It still
  misses a French keyboard, where the position is wrong, and takes `Â` from
  whoever was writing it.
- **Remapping only when `e.key` is not a letter**, dropping the Cmd escape. That
  escape is load-bearing: `Cmd + Option + S` reports `∑`, so `Mod + Alt + S`
  would stop working on the keyboard this was tested on.
- **`navigator.keyboard.getLayoutMap()`**, which maps a code to its cap.
  Chromium only, so Safari keeps the bug, and async where `keyCombo` is pure
  and tested as such (0013).

## Consequences

`web/tests/keys.js` carries the French keyboard as cases, not the US one alone.
A new shortcut needs a `case` and a row (0030), and a letter outside A, Q, Z, W
and M.
