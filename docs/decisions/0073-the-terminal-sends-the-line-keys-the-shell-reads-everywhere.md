# 0073. The terminal sends the line keys in a form the shell reads anywhere

Date: 2026-10-13 · Status: accepted · Amends 0044

**Trigger:** read before handling a key that moves or deletes in the
terminal's command line, or changing what one sends the shell.

## Context

On the Windows VM, Ctrl+Left, Ctrl+Right and Home did nothing in Git Bash,
where VS Code's terminal moves. xterm.js sends `ESC [1;5D`, `ESC [1;5C` and
`ESC [H`; ConPTY turns them into keys, the MSYS2 runtime asks for them as
sequences again, and Git's `inputrc` and readline bind all three. Each step
reads right, and no Windows machine could be driven from here to see which
loses them. `portable-pty` asks for `PSEUDOCONSOLE_WIN32_INPUT_MODE`, which
xterm.js does not speak, and uses the system's ConPTY: either may be where it
parts from VS Code. On a Mac, xterm.js sends nothing for a key held with Cmd,
so Cmd+Left and Cmd+Right did nothing and Cmd+Backspace deleted a character;
VS Code binds those keys itself.

## Decision

`termLineKey`, in the key handler 0044 set up, answers these keys before
xterm.js does. To Git Bash on Windows, it sends keys readline binds by itself
in emacs mode: Ctrl+A and Ctrl+E for Home and End, ESC b and ESC f for
Ctrl+Left and Ctrl+Right, and VS Code's Ctrl+W and ESC d for Ctrl+Backspace
and Ctrl+Delete. Ctrl+A, Ctrl+E and Ctrl+W are single bytes; the ESC ones
still cross ConPTY as sequences, read as Alt with a letter. On a Mac, in any
shell, it sends VS Code's Ctrl+A, Ctrl+E and Ctrl+U for Cmd+Left, Cmd+Right
and Cmd+Backspace, and ESC d for Option+Delete, which zsh leaves unbound.

Only the shell's own line gets them. bash's prompt hook (0069) says when bash
reads its line again, and Enter that it no longer does, so whatever bash
started, a pager kept on the normal screen by `less -X`, PowerShell, a native
prompt, gets xterm.js's keys. On Windows, when bash never said, a program
asking for application cursor keys stands for one: less does, Git Bash's
prompt does not. So does the alternate screen, Shift, AltGr, an input method
composing, PowerShell as the shell, and Linux, where readline binds xterm.js's
sequences.

## Rejected

- **Waiting for the cause on the VM.** The keys stayed broken meanwhile.
- **Application cursor keys alone.** On Windows, nothing says that less's
  request crosses ConPTY, and a bash that reported its prompt is surer.
- **Application cursor keys on a Mac.** oh-my-zsh, prezto and Debian's
  `zshrc` ask for them at every prompt, which would take the Cmd keys back. A
  pager under zsh reads Ctrl+E as a line down, as in VS Code, where Cmd did
  nothing before.
- **The same bytes to PowerShell.** PSReadLine reads the keys themselves, and
  in its Windows mode Ctrl+A selects the line.
- **`macOptionIsMeta`.** It does not reach Cmd, and it would stop Option
  typing `|`, `{` and `[` on a French keyboard.

## Consequences

`web/tests/terminal.js` holds the keys, the prompt's state and what is left
alone. If Home and End move on the VM and Ctrl+Left does not, ConPTY's reading
of ESC is the step that loses them. When bash's hook is dropped, by a
`~/.bashrc` assigning `PROMPT_COMMAND`, or by ConPTY swallowing the OSC, Git
Bash gets these keys in any program that asks for no cursor keys. A line left
open by a quote or a trailing `\` gets xterm.js's keys under PS2, since bash
runs no hook there, and a binding that runs the line without Enter, Ctrl+O,
leaves its command bash's keys. A bash in vi mode gets emacs keys.
