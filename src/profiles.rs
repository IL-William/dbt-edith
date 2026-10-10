//! `profiles.yml`: where dbt looks for it.
//!
//! The one file outside the project dbt-edith opens (0017), found the way dbt
//! finds it rather than named by the browser, so no request can point it at
//! another file (0049). `tools/sf_lineage.py` repeats the same order in Python,
//! from the same directory and the same environment, so the two agree.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// What dbt's search depends on besides the project, read once at startup:
/// dbt-edith's environment does not change while it runs, and the terminal and
/// the Snowflake script inherit it.
#[derive(Debug, Clone, Default)]
pub struct Lookup {
    /// `DBT_PROFILES_DIR`, which wins over everything else.
    pub dir: Option<PathBuf>,
    pub home: Option<PathBuf>,
}

impl Lookup {
    /// On Windows the home is `USERPROFILE`, as Python and dbt read it: under
    /// Git Bash `HOME` holds an MSYS path.
    pub fn from_env(lookup: impl Fn(&str) -> Option<OsString>, windows: bool) -> Lookup {
        let get = |key: &str| lookup(key).filter(|v| !v.is_empty()).map(PathBuf::from);
        Lookup { dir: get("DBT_PROFILES_DIR"), home: get(if windows { "USERPROFILE" } else { "HOME" }) }
    }

    /// The flag aside, which nothing here passes: `DBT_PROFILES_DIR`, then the
    /// directory dbt runs in when it holds one, then `~/.dbt`. A relative
    /// `DBT_PROFILES_DIR` is taken from the project, where the terminal opens.
    /// None only without a home to fall back on.
    pub fn path(&self, root: &Path) -> Option<PathBuf> {
        if let Some(dir) = &self.dir {
            return Some(root.join(dir).join("profiles.yml"));
        }
        let local = root.join("profiles.yml");
        if local.is_file() {
            return Some(local);
        }
        self.home.as_ref().map(|h| h.join(".dbt").join("profiles.yml"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<OsString> + 'a {
        move |key| pairs.iter().find(|(k, _)| *k == key).map(|(_, v)| OsString::from(*v))
    }

    fn project(tag: &str, with_profile: bool) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dbt-edith-profiles-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        if with_profile {
            std::fs::write(dir.join("profiles.yml"), "shop:\n").unwrap();
        }
        dir
    }

    #[test]
    fn the_home_is_the_fallback() {
        let root = project("home", false);
        let lookup = Lookup::from_env(env(&[("HOME", "/home/me"), ("USERPROFILE", r"C:\Users\me")]), false);
        assert_eq!(lookup.path(&root), Some(PathBuf::from("/home/me/.dbt/profiles.yml")));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn windows_reads_userprofile_not_the_msys_home() {
        let lookup = Lookup::from_env(env(&[("HOME", "/c/Users/me"), ("USERPROFILE", r"C:\Users\me")]), true);
        assert_eq!(lookup.home, Some(PathBuf::from(r"C:\Users\me")));
    }

    #[test]
    fn a_profile_in_the_project_wins_over_the_home() {
        let root = project("local", true);
        let lookup = Lookup::from_env(env(&[("HOME", "/home/me")]), false);
        assert_eq!(lookup.path(&root), Some(root.join("profiles.yml")));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn dbt_profiles_dir_wins_over_both_even_when_empty_of_one() {
        let root = project("explicit", true);
        let lookup = Lookup::from_env(env(&[("HOME", "/home/me"), ("DBT_PROFILES_DIR", "/etc/dbt")]), false);
        assert_eq!(lookup.path(&root), Some(PathBuf::from("/etc/dbt/profiles.yml")));
        // Relative, it is read from the project, where dbt runs.
        let relative = Lookup::from_env(env(&[("DBT_PROFILES_DIR", "config")]), false);
        assert_eq!(relative.path(&root), Some(root.join("config").join("profiles.yml")));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn empty_values_are_unset() {
        let root = project("empty", false);
        let lookup = Lookup::from_env(env(&[("HOME", ""), ("DBT_PROFILES_DIR", "")]), false);
        assert_eq!(lookup.path(&root), None);
        let _ = std::fs::remove_dir_all(&root);
    }
}
