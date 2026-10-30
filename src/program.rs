//! A program started by its name, found on the PATH and nowhere else.
//!
//! On Windows, `Command::new("git")` looks in this binary's own folder before
//! the PATH. A `dbt-edith.exe` kept in a project's folder sits beside whatever
//! the repository commits there, so a `git.exe`, `cmd.exe` or `python.exe`
//! committed beside it ran in its place: git on the first status poll, cmd to
//! open the browser at startup (0094). A name is therefore turned into a path
//! here, from the PATH alone, before anything starts it.
//!
//! The invariant: a relative entry of the PATH is skipped, as the current
//! folder would decide what it names, and on Unix a file without an execute
//! bit is passed over, as the shell passes over it.
//!
//! What the terminal's shell types is not held to that: some shells read the
//! current folder first, so `reads_cwd` tells the page when the one it starts
//! does, by its kind and its platform (0130).

use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// The file `name` names on the PATH, `.exe` added on Windows as `Command`
/// adds it, or None when no entry holds one.
pub fn on_path(name: &str) -> Option<PathBuf> {
    on_path_in(name, std::env::var_os("PATH"), cfg!(windows))
}

/// Why the terminal's shell could run a program in the project in place of
/// git, dbt or cargo (0130).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Cwd {
    /// The PATH holds a relative entry, `.` or `bin`, which a shell reads from
    /// the folder it stands in.
    Relative,
    /// The PATH holds an empty entry (a leading or trailing separator, a
    /// doubled one), which the shell reads as the current folder.
    Empty,
    /// cmd.exe looks in the current folder before it reads the PATH.
    Cmd,
}

/// The reason, and for a relative entry the first one found, as the PATH
/// writes it: the user's own PATH, sent to the page as text and to nothing else.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Reads {
    pub why: Cwd,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub entry: String,
}

impl Reads {
    fn new(why: Cwd, entry: &Path) -> Reads {
        Reads { why, entry: entry.to_string_lossy().into_owned() }
    }

    /// The entry in quotes as the PATH writes it, a backslash included, with
    /// only a control character escaped: printed raw to a terminal it could
    /// move the cursor or rewrite the line.
    pub fn quoted(&self) -> String {
        let inner: String = self.entry.chars().flat_map(|c| if c.is_control() { c.escape_default().collect::<Vec<_>>() } else { vec![c] }).collect();
        format!("\"{inner}\"")
    }
}

/// Whether the shell `shell` (`pty::shell_name`), started with the PATH
/// `path_var`, reads the current folder, and by which door.
///
/// What a shell does with the PATH it is handed is not what the PATH says:
/// - A relative entry is read from the current folder by bash and zsh
///   everywhere, Git Bash included, and the same rule is kept for any other
///   shell (fish, a custom `--shell`), whose handling of an empty entry is not
///   known here, so only a relative one is reported for it.
/// - An empty entry survives only where nothing rewrites the PATH before the
///   first prompt. On macOS a login shell, which is how `$SHELL -l` starts,
///   runs `path_helper -s` from /etc/zprofile and /etc/profile, which drops
///   empty entries and keeps relative ones. Git Bash's msys2 converts the
///   Windows PATH to its own form skipping empty elements, a trailing `;`
///   included. Elsewhere on Unix bash, zsh, sh, dash, ksh and mksh keep them.
/// - PowerShell drops empty entries, and reads an entry that starts with `.`
///   against the current location (its command discovery resolves those in
///   `ResolveCurrentDirectoryInLookupPaths`); `bin` it leaves alone. So `.`,
///   `.\bin` and `..\x` are reported for it, nothing else.
/// - cmd.exe looks in the current folder first, whatever the PATH holds.
///
/// Nothing is reported for a missing PATH: bash then falls back to a default
/// one that ends in `.`, which is not worth a warning in a terminal that was
/// handed no PATH. bash also expands a `~` leading an entry where zsh does not,
/// so `~/bin` is reported as relative for both, a false alarm for bash only.
pub fn reads_cwd(path_var: Option<&OsString>, shell: &str, windows: bool, macos: bool, login: bool) -> Option<Reads> {
    if shell == "cmd" {
        return Some(Reads { why: Cwd::Cmd, entry: String::new() });
    }
    let entries: Vec<_> = path_var.iter().flat_map(|v| std::env::split_paths(v)).collect();
    let powershell = matches!(shell, "powershell" | "pwsh");
    let relative = |dir: &&PathBuf| {
        if powershell {
            dir.to_string_lossy().trim().starts_with('.')
        } else {
            !dir.as_os_str().is_empty() && !dir.is_absolute()
        }
    };
    if let Some(dir) = entries.iter().find(relative) {
        return Some(Reads::new(Cwd::Relative, dir));
    }
    let survives = matches!(shell, "bash" | "zsh" | "sh" | "dash" | "ksh" | "mksh") && !windows && !(macos && login);
    (survives && entries.iter().any(|dir| dir.as_os_str().is_empty())).then(|| Reads::new(Cwd::Empty, Path::new("")))
}

fn on_path_in(name: &str, path_var: Option<OsString>, windows: bool) -> Option<PathBuf> {
    let file = if windows && Path::new(name).extension().is_none() { format!("{name}.exe") } else { name.to_string() };
    path_var
        .iter()
        .flat_map(std::env::split_paths)
        .filter(|dir| dir.is_absolute())
        .map(|dir| dir.join(&file))
        .find(|p| runnable(p))
}

#[cfg(unix)]
fn runnable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
}

/// A file, links followed. Only where `metadata` fails is the entry itself
/// judged: the Store's `python.exe` under `WindowsApps` is an App Execution
/// Alias, a reparse point `metadata` may not follow though Windows starts it.
/// Such an alias is no symlink to Rust, so a dangling link, or one to a
/// folder, is still passed over for a later entry that starts.
#[cfg(not(unix))]
fn runnable(path: &Path) -> bool {
    match std::fs::metadata(path) {
        Ok(meta) => meta.is_file(),
        Err(_) => std::fs::symlink_metadata(path).is_ok_and(|m| !m.is_dir() && !m.file_type().is_symlink()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What each shell does with the shapes of a PATH, on each platform, as
    /// measured: on a Mac `path_helper -s` and a login `bash -l` or `zsh -l`
    /// drop an empty entry and keep `.`.
    #[test]
    fn a_shell_reads_the_current_folder_by_its_kind_and_platform() {
        let sep = if cfg!(windows) { ";" } else { ":" };
        let (abs, abs2) = if cfg!(windows) { (r"C:\bin", r"C:\tools") } else { ("/usr/bin", "/opt/tools") };
        let var = |s: String| Some(OsString::from(s));
        let clean = var(format!("{abs}{sep}{abs2}"));
        let leading = var(format!("{sep}{abs}"));
        let trailing = var(format!("{abs}{sep}"));
        let doubled = var(format!("{abs}{sep}{sep}{abs2}"));
        let dot = var(format!("{abs}{sep}."));
        let named = var(format!("bin{sep}{abs}"));
        let percent = var(format!(r"%X%\bin{sep}{abs}"));
        // (shell, windows, macos, login)
        let linux = ("bash", false, false, true);
        let mac_login = ("zsh", false, true, true);
        let mac_plain = ("zsh", false, true, false);
        let git_bash = ("bash", true, false, false);
        let why = |v: Option<&OsString>, shell: &str, w: bool, m: bool, l: bool| reads_cwd(v, shell, w, m, l).map(|r| r.why);
        let at = |v: &Option<OsString>, k: (&str, bool, bool, bool)| reads_cwd(v.as_ref(), k.0, k.1, k.2, k.3).map(|r| r.why);
        for k in [linux, mac_login, mac_plain, git_bash] {
            assert_eq!(at(&clean, k), None, "a clean PATH, {k:?}");
            assert_eq!(at(&dot, k), Some(Cwd::Relative), "a dot, {k:?}");
            assert_eq!(at(&named, k), Some(Cwd::Relative), "a relative folder, {k:?}");
            assert_eq!(at(&percent, k), Some(Cwd::Relative), "an unexpanded variable, {k:?}");
        }
        for v in [&leading, &trailing, &doubled] {
            assert_eq!(at(v, linux), Some(Cwd::Empty), "kept on Linux: {v:?}");
            assert_eq!(at(v, mac_plain), Some(Cwd::Empty), "kept by a macOS shell that is not a login one: {v:?}");
            assert_eq!(at(v, mac_login), None, "dropped by path_helper: {v:?}");
            assert_eq!(at(v, git_bash), None, "dropped by msys2: {v:?}");
        }
        // An empty value is one empty entry.
        assert_eq!(at(&var(String::new()), linux), Some(Cwd::Empty));
        assert_eq!(at(&None, linux), None, "no PATH");
        // fish and a custom shell: relative entries only.
        assert_eq!(why(dot.as_ref(), "fish", false, false, false), Some(Cwd::Relative));
        assert_eq!(why(leading.as_ref(), "fish", false, false, false), None);
        assert_eq!(why(leading.as_ref(), "nu", false, false, false), None);
        // sh, dash, ksh and mksh keep an empty entry as bash does.
        for shell in ["sh", "dash", "ksh", "mksh"] {
            assert_eq!(why(leading.as_ref(), shell, false, false, false), Some(Cwd::Empty), "{shell}");
            assert_eq!(why(leading.as_ref(), shell, false, true, true), None, "{shell} at a macOS login");
        }
        // PowerShell reads an entry that starts with a dot, never an empty
        // one, and leaves `bin` alone; cmd.exe always reads the folder.
        let dot_slash = var(format!(r".\bin{sep}{abs}"));
        for shell in ["powershell", "pwsh"] {
            assert_eq!(why(dot.as_ref(), shell, true, false, false), Some(Cwd::Relative), "{shell}");
            assert_eq!(why(dot_slash.as_ref(), shell, true, false, false), Some(Cwd::Relative), "{shell}");
            assert_eq!(why(named.as_ref(), shell, true, false, false), None, "{shell}");
            assert_eq!(why(leading.as_ref(), shell, true, false, false), None, "{shell}");
            assert_eq!(why(clean.as_ref(), shell, true, false, false), None, "{shell}");
        }
        // Quoted as written: a backslash stays one, a control character is escaped.
        let shown = |entry: &str| Reads { why: Cwd::Relative, entry: entry.into() }.quoted();
        assert_eq!(shown(r".\bin"), r#"".\bin""#);
        assert_eq!(shown("a\x1b[2Jb"), r#""a\u{1b}[2Jb""#);
        // The first relative entry is the one named, as the PATH writes it.
        let two = var(format!("{abs}{sep}bin{sep}."));
        let read = reads_cwd(two.as_ref(), "bash", false, false, false).unwrap();
        assert_eq!((read.why, read.entry.as_str()), (Cwd::Relative, "bin"));
        assert_eq!(reads_cwd(leading.as_ref(), "bash", false, false, false).unwrap().entry, "", "an empty entry names nothing");
        assert_eq!(why(clean.as_ref(), "cmd", true, false, false), Some(Cwd::Cmd));
        assert_eq!(why(None, "cmd", true, false, false), Some(Cwd::Cmd));
    }

    /// Only what the PATH names, never a relative entry. The folder this
    /// binary sits in is not one of them, where `Command`'s own search on
    /// Windows looks first.
    #[cfg(unix)]
    #[test]
    fn a_name_is_found_on_the_path_alone() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("dbt-edith-program-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let (first, second) = (dir.join("first"), dir.join("second"));
        std::fs::create_dir_all(&first).unwrap();
        std::fs::create_dir_all(&second).unwrap();
        let tool = |at: &Path, mode: u32| {
            std::fs::write(at.join("tool"), "#!/bin/sh\n").unwrap();
            std::fs::set_permissions(at.join("tool"), std::fs::Permissions::from_mode(mode)).unwrap();
        };
        tool(&first, 0o644);
        tool(&second, 0o755);
        let path = |dirs: &[&Path]| Some(std::env::join_paths(dirs).unwrap());
        assert_eq!(on_path_in("tool", path(&[&first, &second]), false), Some(second.join("tool")), "not executable, passed over");
        assert_eq!(on_path_in("tool", path(&[Path::new("second"), Path::new(".")]), false), None, "a relative entry is skipped");
        assert_eq!(on_path_in("tool", None, false), None);
        std::fs::write(second.join("tool.exe"), "").unwrap();
        std::fs::set_permissions(second.join("tool.exe"), std::fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(on_path_in("tool", path(&[&second]), true), Some(second.join("tool.exe")), "Windows adds .exe");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
