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

use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// The file `name` names on the PATH, `.exe` added on Windows as `Command`
/// adds it, or None when no entry holds one.
pub fn on_path(name: &str) -> Option<PathBuf> {
    on_path_in(name, std::env::var_os("PATH"), cfg!(windows))
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
