//! `profiles.yml`: where dbt looks for it, and which of its targets dbt uses.
//!
//! The one file outside the project dbt-edith opens (0017), found the way dbt
//! finds it rather than named by the browser, so no request can point it at
//! another file (0049). One the project holds is held to the project, as any
//! of its files is: a `profiles.yml` there linking out is not opened (0094). `tools/sf_lineage.py` repeats the same order in Python,
//! from the same directory and the same environment, so the two agree.
//!
//! The active target's database and schema are where a clone lands (0058).
//! They are read with the line scanner `redact` hides secrets with, not a YAML
//! parser (0003, 0018), and nothing else of the target leaves this module.

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
    /// `DBT_TARGET`, which picks the target the way `--target` does. Read from
    /// this process alone, never from the project's `.env`, which dbt does not
    /// read either.
    pub target: Option<String>,
}

impl Lookup {
    /// On Windows the home is `USERPROFILE`, as Python and dbt read it: under
    /// Git Bash `HOME` holds an MSYS path.
    pub fn from_env(lookup: impl Fn(&str) -> Option<OsString>, windows: bool) -> Lookup {
        let get = |key: &str| lookup(key).filter(|v| !v.is_empty()).map(PathBuf::from);
        let target = lookup("DBT_TARGET").map(|v| v.to_string_lossy().trim().to_string()).filter(|v| !v.is_empty());
        Lookup { dir: get("DBT_PROFILES_DIR"), home: get(if windows { "USERPROFILE" } else { "HOME" }), target }
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

/// Why a profile dbt would read is not opened here: it sits in a folder of
/// the project and links out of it.
pub const LINKS_OUT: &str = "profiles.yml in the project links to a file outside it, so dbt-edith does not open it. \
     dbt still follows the link: start dbt-edith with DBT_PROFILES_DIR set to the folder it links to, to open it here";

/// Whether the profile at `path` may be opened. One in a folder of the
/// project, after links, is a file of the repository, which can commit it as
/// a link to any file of the user's, a key say, in which redaction finds no
/// key to hide: it is held to the project as the editor's paths are, by
/// `files::resolve`. One outside the project is the user's own, linked from a
/// dotfiles repository or not, and is followed as dbt follows it (0094).
pub fn held(root: &Path, path: &Path) -> Result<(), String> {
    let (Ok(canon_root), Some(Ok(folder)), Some(name)) =
        (root.canonicalize(), path.parent().map(Path::canonicalize), path.file_name())
    else {
        return Ok(());
    };
    let Ok(rel) = folder.strip_prefix(&canon_root) else { return Ok(()) };
    let rel = rel.join(name).to_string_lossy().into_owned();
    crate::files::resolve(&canon_root, &rel).map(|_| ()).map_err(|_| LINKS_OUT.to_string())
}

/// One line, without its ending, and the ending it had: a profile written on
/// Windows keeps its `\r\n`.
pub(crate) fn lines(text: &str) -> Vec<(&str, &str)> {
    text.split_inclusive('\n')
        .map(|l| {
            let body = l.trim_end_matches(['\n', '\r']);
            (body, &l[body.len()..])
        })
        .collect()
}

pub(crate) fn indent(body: &str) -> usize {
    body.len() - body.trim_start_matches(' ').len()
}

/// The key a line opens, where its value starts, and its depth: `password: x`
/// gives `password`, the byte just past the colon, and its indentation. A list
/// item, `- name: x`, counts as indented to its key. None for a line that is
/// blank, a comment, or the continuation of a value.
pub(crate) fn key_line(body: &str) -> Option<(String, usize, usize)> {
    let mut depth = indent(body);
    let mut rest = &body[depth..];
    if rest.is_empty() || rest.starts_with('#') {
        return None;
    }
    while let Some(after) = rest.strip_prefix("- ") {
        let gap = after.len() - after.trim_start_matches(' ').len();
        depth += 2 + gap;
        rest = &after[gap..];
    }
    let colon = rest.find(": ").or_else(|| rest.ends_with(':').then(|| rest.len() - 1))?;
    let key = rest[..colon].trim().trim_matches(|c| c == '"' || c == '\'');
    if key.is_empty() || key.contains(['{', '[', '#']) {
        return None;
    }
    Some((key.to_string(), body.len() - rest.len() + colon + 1, depth))
}

/// Every key of a profile, with the keys above it and its value as written
/// on its own line, in file order. The lines of a block scalar, such as a
/// `private_key: |`, are its value and never read as keys.
fn entries(text: &str) -> Vec<(Vec<String>, String)> {
    let all = lines(text);
    let mut out = Vec::new();
    let mut stack: Vec<(usize, String)> = Vec::new();
    let mut i = 0;
    while i < all.len() {
        let body = all[i].0;
        i += 1;
        let Some((key, at, depth)) = key_line(body) else { continue };
        while stack.last().is_some_and(|(d, _)| *d >= depth) {
            stack.pop();
        }
        stack.push((depth, key));
        let inline = body[at..].trim();
        out.push((stack.iter().map(|(_, k)| k.clone()).collect(), inline.to_string()));
        if inline.starts_with('|') || inline.starts_with('>') {
            while i < all.len() && (all[i].0.trim().is_empty() || indent(all[i].0) > depth) {
                i += 1;
            }
        }
    }
    out
}

/// The target dbt would use and where it builds, or why that cannot be said.
/// Names and locations only: a target's credentials never reach this struct.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize)]
pub struct Target {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub profile: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub target: String,
    /// `DBT_TARGET`, `profile` or `default`: what picked the target.
    #[serde(skip_serializing_if = "str::is_empty")]
    pub target_from: &'static str,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub database: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub schema: String,
    /// Set whenever database and schema are not, and never otherwise.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub why: String,
}

/// The names of a profile's targets, sorted: names only, so a caller can ask
/// which one without anything of a target leaving this module.
pub fn target_names(text: &str, profile: &str) -> Vec<String> {
    let mut names: Vec<String> = entries(text)
        .into_iter()
        .filter(|(p, _)| p.len() == 3 && p[0] == profile && p[1] == "outputs")
        .map(|(mut p, _)| p.swap_remove(2))
        .collect();
    names.sort();
    names.dedup();
    names
}

/// A profile's target and its database and schema, chosen as dbt chooses:
/// `DBT_TARGET`, then the profile's `target:`, then `default`. Only a plain
/// value is taken. A target written as `env_var('DBT_TARGET', 'dev')` is
/// `dev` here, since `DBT_TARGET` is unset once `from_env` is None, but any
/// other Jinja is left to the user to read, because guessing a schema would
/// put a clone somewhere they did not mean.
pub fn active_target(text: &str, profile: &str, from_env: Option<&str>) -> Target {
    let mut t = Target { profile: profile.to_string(), ..Default::default() };
    if profile.is_empty() {
        t.why = "dbt_project.yml names no profile".into();
        return t;
    }
    let all = entries(text);
    // A key written twice is the last one, as for a YAML loader. Inside, None
    // is a value that is not a plain scalar: an alias, a list, a block.
    let get = |path: &[&str]| {
        all.iter()
            .rev()
            .find(|(p, _)| p.len() == path.len() && p.iter().zip(path).all(|(a, b)| a == b))
            .map(|(_, v)| crate::project::scalar(v))
    };
    let jinja = |v: &str| v.contains("{{") || v.contains("{%");
    let has = |path: &[&str]| all.iter().any(|(p, _)| p.len() > path.len() && p.iter().zip(path).all(|(a, b)| a == b));
    if !all.iter().any(|(p, _)| p.len() == 1 && p[0] == profile) {
        t.why = format!("profiles.yml has no profile named {profile}");
        return t;
    }

    match from_env {
        Some(name) => {
            t.target = name.to_string();
            t.target_from = "DBT_TARGET";
        }
        None => match get(&[profile, "target"]) {
            Some(None) => {
                t.why = format!("the target of profile {profile} is not a plain value");
                return t;
            }
            Some(Some(raw)) if jinja(&raw) => {
                let (value, cell) = crate::envs::resolve(&raw, "", &crate::envs::Vars::new());
                if cell.kind != crate::envs::Status::Env || cell.vars != ["DBT_TARGET"] || value.is_empty() {
                    t.why = format!("the target of profile {profile} is Jinja that dbt-edith does not evaluate");
                    return t;
                }
                t.target = value;
                t.target_from = "profile";
            }
            Some(Some(raw)) if !raw.is_empty() => {
                t.target = raw;
                t.target_from = "profile";
            }
            _ => {
                t.target = "default".into();
                t.target_from = "default";
            }
        },
    }

    let target = t.target.clone();
    let output = [profile, "outputs", target.as_str()];
    if !has(&output) {
        let names = target_names(text, profile);
        t.why = if names.is_empty() {
            format!("profile {profile} has no outputs")
        } else {
            format!("profile {profile} has no target {target}, only {}", names.join(", "))
        };
        return t;
    }
    let field = |key: &str| get(&[profile, "outputs", target.as_str(), key]);
    if let Some(kind) = field("type").flatten().filter(|k| !k.is_empty() && !k.eq_ignore_ascii_case("snowflake")) {
        t.why = format!("target {target} is a {kind} target, not a Snowflake one");
        return t;
    }
    let merged = get(&[profile, "outputs", target.as_str(), "<<"]).is_some();
    let mut place = Vec::new();
    for key in ["database", "schema"] {
        match field(key) {
            Some(None) => {
                t.why = format!("the {key} of target {target} is not a plain value");
                return t;
            }
            Some(Some(v)) if jinja(&v) => {
                t.why = format!("the {key} of target {target} is Jinja that dbt-edith does not evaluate");
                return t;
            }
            Some(Some(v)) if !v.is_empty() => place.push(v),
            _ if merged => {
                t.why = format!("target {target} takes its {key} from a YAML merge key, which dbt-edith does not follow");
                return t;
            }
            _ => {
                t.why = format!("target {target} sets no {key}");
                return t;
            }
        }
    }
    t.schema = place.pop().unwrap_or_default();
    t.database = place.pop().unwrap_or_default();
    t
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

    /// The project's own `profiles.yml` is held to the project, whichever
    /// way dbt found it there; one outside is followed wherever it links,
    /// as dbt follows it (0094). On main every one of these was opened.
    #[cfg(unix)]
    #[test]
    fn a_profile_in_the_project_is_held_to_it() {
        use std::os::unix::fs::symlink;
        let root = project("held", false);
        let outside = project("held-outside", false);
        let key = outside.join("id_ed25519");
        std::fs::write(&key, "-----BEGIN OPENSSH PRIVATE KEY-----\n").unwrap();
        let local = root.join("profiles.yml");
        assert_eq!(held(&root, &local), Ok(()), "none there yet: the read says so");
        std::fs::write(&local, "shop:\n").unwrap();
        assert_eq!(held(&root, &local), Ok(()), "a plain file, as before");

        std::fs::remove_file(&local).unwrap();
        symlink(&key, &local).unwrap();
        assert_eq!(held(&root, &local), Err(LINKS_OUT.to_string()));
        // Found through a DBT_PROFILES_DIR naming the project, however spelt.
        std::fs::create_dir_all(root.join("config")).unwrap();
        for dir in [".", "config/..", root.to_str().unwrap()] {
            let named = Lookup::from_env(env(&[("DBT_PROFILES_DIR", dir)]), false);
            assert_eq!(held(&root, &named.path(&root).unwrap()), Err(LINKS_OUT.to_string()), "{dir}");
        }

        // A link that stays in the project is a file of it like any other.
        std::fs::remove_file(&local).unwrap();
        std::fs::write(root.join("config").join("profiles.yml"), "shop:\n").unwrap();
        symlink("config/profiles.yml", &local).unwrap();
        assert_eq!(held(&root, &local), Ok(()));

        // Outside the project the user's own links are followed, a dotfiles
        // one included, and a DBT_PROFILES_DIR spelt from the project but
        // naming a folder beside it is outside too.
        let home = outside.join(".dbt");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::write(outside.join("dotfiles.yml"), "shop:\n").unwrap();
        symlink(outside.join("dotfiles.yml"), home.join("profiles.yml")).unwrap();
        assert_eq!(held(&root, &home.join("profiles.yml")), Ok(()));
        let beside = format!("../{}/.dbt", outside.file_name().unwrap().to_str().unwrap());
        let named = Lookup::from_env(env(&[("DBT_PROFILES_DIR", beside.as_str())]), false);
        assert_eq!(held(&root, &named.path(&root).unwrap()), Ok(()));
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&outside);
    }

    #[test]
    fn empty_values_are_unset() {
        let root = project("empty", false);
        let lookup = Lookup::from_env(env(&[("HOME", ""), ("DBT_PROFILES_DIR", "")]), false);
        assert_eq!(lookup.path(&root), None);
        let _ = std::fs::remove_dir_all(&root);
    }
    /// Invented: two profiles that share target names, credentials of every
    /// shape beside the keys that matter, and a block scalar whose lines look
    /// like keys.
    const PROFILES: &str = "\
other:
  target: dev
  outputs:
    dev:
      type: snowflake
      database: OTHER_DB
      schema: other_schema
shop:
  target: dev   # the usual one
  outputs:
    dev:
      type: snowflake
      account: ab12345.eu-west-1
      user: ANALYST
      password: 'hunter2'
      private_key: |
        database: NOT_A_KEY
        schema: not_a_key
      role: TRANSFORMER
      database: \"DEV_DB\"
      schema: dbt_someone
    ci:
      type: snowflake
      database: CI_DB
      schema: \"{{ env_var('CI_SCHEMA') }}\"
    pg:
      type: postgres
      dbname: shop
      schema: public
";

    #[test]
    fn the_active_target_gives_its_database_and_schema() {
        let t = active_target(PROFILES, "shop", None);
        assert_eq!((t.target.as_str(), t.target_from), ("dev", "profile"));
        assert_eq!((t.database.as_str(), t.schema.as_str(), t.why.as_str()), ("DEV_DB", "dbt_someone", ""));
        // The other profile's dev is another target altogether.
        assert_eq!(active_target(PROFILES, "other", None).database, "OTHER_DB");
        // Windows line endings change nothing.
        assert_eq!(active_target(&PROFILES.replace('\n', "\r\n"), "shop", None), t);
    }

    #[test]
    fn no_credential_reaches_the_target() {
        let json = serde_json::to_string(&active_target(PROFILES, "shop", None)).unwrap();
        for (i, value) in ["hunter2", "ANALYST", "ab12345", "TRANSFORMER", "NOT_A_KEY"].iter().enumerate() {
            assert!(!json.contains(value), "value number {i} reached the target: {json}");
        }
    }

    #[test]
    fn dbt_target_wins_over_the_profile_and_the_profile_over_default() {
        let ci = active_target(PROFILES, "shop", Some("pg"));
        assert_eq!((ci.target.as_str(), ci.target_from), ("pg", "DBT_TARGET"));
        assert_eq!(ci.why, "target pg is a postgres target, not a Snowflake one");
        let bare = "shop:\n  outputs:\n    default:\n      database: D\n      schema: s\n";
        let t = active_target(bare, "shop", None);
        assert_eq!((t.target.as_str(), t.target_from, t.database.as_str()), ("default", "default", "D"));
        // Read the way dbt reads it while DBT_TARGET is unset: its default.
        let jinja = "shop:\n  target: \"{{ env_var('DBT_TARGET', 'dev') }}\"\n  outputs:\n    dev:\n      database: D\n      schema: s\n";
        assert_eq!(active_target(jinja, "shop", None).target, "dev");
        assert_eq!(active_target(jinja, "shop", Some("prod")).why, "profile shop has no target prod, only dev");
        let other = "shop:\n  target: \"{{ env_var('WHO', 'dev') }}\"\n  outputs:\n    dev:\n      database: D\n";
        assert_eq!(active_target(other, "shop", None).why, "the target of profile shop is Jinja that dbt-edith does not evaluate");
    }

    #[test]
    fn what_cannot_be_read_says_why_and_gives_no_place() {
        for (text, profile, env, why) in [
            (PROFILES, "", None, "dbt_project.yml names no profile"),
            (PROFILES, "finance", None, "profiles.yml has no profile named finance"),
            (PROFILES, "shop", Some("prod"), "profile shop has no target prod, only ci, dev, pg"),
            (PROFILES, "shop", Some("ci"), "the schema of target ci is Jinja that dbt-edith does not evaluate"),
            ("shop:\n  target: dev\n", "shop", None, "profile shop has no outputs"),
            ("shop:\n  outputs:\n    dev:\n      database: D\n", "shop", Some("dev"), "target dev sets no schema"),
            ("shop:\n  outputs:\n    dev:\n      <<: *base\n      schema: s\n", "shop", Some("dev"), "target dev takes its database from a YAML merge key, which dbt-edith does not follow"),
            ("shop:\n  outputs:\n    dev:\n      database: *db\n      schema: s\n", "shop", Some("dev"), "the database of target dev is not a plain value"),
        ] {
            let t = active_target(text, profile, env);
            assert_eq!(t.why, why, "{text}");
            assert!(t.database.is_empty() && t.schema.is_empty(), "{why}");
        }
    }

    #[test]
    fn dbt_target_comes_from_this_process_and_empty_is_unset() {
        assert_eq!(Lookup::from_env(env(&[("DBT_TARGET", " uat ")]), false).target.as_deref(), Some("uat"));
        assert_eq!(Lookup::from_env(env(&[("DBT_TARGET", "")]), false).target, None);
    }
}
