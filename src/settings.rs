//! Per-project settings, stored in the user's config directory.
//!
//! Deliberately never inside the project: a dbt repository is usually shared,
//! and one person's environment names do not belong in it.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const VERSION: u32 = 1;

#[derive(serde::Serialize, serde::Deserialize, Default, Clone, Debug, PartialEq)]
pub struct EnvOverride {
    /// None means the automatic name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// None means automatic: hidden only for templates, backups, and files that
    /// define none of the variables in use. A user can always override it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hidden: Option<bool>,
}

#[derive(serde::Serialize, serde::Deserialize, Default, Clone, Debug)]
pub struct Settings {
    #[serde(default)]
    pub version: u32,
    /// The project these settings belong to, checked on load so that a hash
    /// collision can never apply someone else's names.
    #[serde(default)]
    pub project: String,
    #[serde(default)]
    pub envs: BTreeMap<String, EnvOverride>,
    /// Where a fresh browser tab starts; each tab then keeps its own choice.
    #[serde(default)]
    pub selected: Option<String>,
    /// Snowflake is the picked column-lineage tool, so its script runs and a
    /// column click fetches. Off unless the user picked it (0016, 0031).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub snowflake_lineage: bool,
    /// Whether Snowflake's features are offered at all: its column lineage and
    /// the profile link. None follows the manifest's adapter, so a project on
    /// another warehouse is never offered them unasked (0031). Off is written
    /// out, because on a Snowflake project it is not the default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snowflake_features: Option<bool>,
    /// Which column-lineage cache to merge, by file name. A project can hold one
    /// per producer, and the choice is the user's rather than the newest file's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cll_file: Option<String>,
}

/// Whether Snowflake's features are on: the user's choice for the project, or,
/// until there is one, whether the manifest says the adapter is Snowflake.
pub fn snowflake_features(chosen: Option<bool>, adapter: &str) -> bool {
    chosen.unwrap_or_else(|| adapter.eq_ignore_ascii_case("snowflake"))
}

fn looks_absolute(value: &str, windows: bool) -> bool {
    if windows {
        let b = value.as_bytes();
        value.starts_with(r"\\") || (b.len() > 2 && b[0].is_ascii_alphabetic() && b[1] == b':' && (b[2] == b'\\' || b[2] == b'/'))
    } else {
        value.starts_with('/')
    }
}

/// The dbt-edith config directory, or None when nothing usable is set, in which
/// case the app runs without persistence. Empty or relative values are ignored.
/// On Windows `HOME` is not consulted: under Git Bash it holds an MSYS path.
pub fn config_dir(lookup: impl Fn(&str) -> Option<OsString>, windows: bool) -> Option<PathBuf> {
    let get = |key: &str| {
        lookup(key)
            .map(|v| v.to_string_lossy().into_owned())
            .filter(|v| !v.is_empty() && looks_absolute(v, windows))
            .map(PathBuf::from)
    };
    if let Some(explicit) = get("DBT_EDITH_CONFIG_DIR") {
        return Some(explicit);
    }
    let base = if windows {
        get("APPDATA").or_else(|| get("USERPROFILE").map(|p| p.join("AppData").join("Roaming")))
    } else {
        get("XDG_CONFIG_HOME").or_else(|| get("HOME").map(|p| p.join(".config")))
    };
    base.map(|p| p.join("dbt-edith"))
}

/// A stable spelling of the project path. Windows verbatim prefixes and
/// separators are normalised, so a change in how a path is displayed cannot
/// silently orphan the settings. Case is folded on Windows only: macOS and
/// Linux volumes can be case-sensitive.
pub fn normalise(root: &str, windows: bool) -> String {
    let mut s = if let Some(rest) = root.strip_prefix(r"\\?\UNC\") {
        format!("//{rest}")
    } else {
        root.strip_prefix(r"\\?\").unwrap_or(root).to_string()
    };
    s = s.replace('\\', "/");
    while s.len() > 1 && s.ends_with('/') {
        s.pop();
    }
    if windows {
        s = s.to_lowercase();
    }
    s
}

pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// `<folder>-<hash>.json`: the folder name so a person can find the file, the
/// hash so two projects with the same folder name do not collide.
pub fn file_name(root: &str, windows: bool) -> String {
    let norm = normalise(root, windows);
    let folder = norm.rsplit('/').next().unwrap_or("");
    let slug: String = folder
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c.to_ascii_lowercase() } else { '_' })
        .take(40)
        .collect();
    let slug = if slug.is_empty() { "project".to_string() } else { slug };
    format!("{slug}-{:016x}.json", fnv1a64(norm.as_bytes()))
}

pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    // Named per write, not per process: two writes of one file at once, such
    // as two requests that both start the Snowflake script, each get their
    // own, and the last rename wins.
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let mut tmp_name = path.file_name().unwrap_or_default().to_os_string();
    tmp_name.push(format!(".tmp-{}-{n}", std::process::id()));
    let tmp = path.with_file_name(tmp_name);
    write_private(&tmp, bytes)?;
    // The write changes the content, never who may read it: the file keeps
    // the target's mode, or takes the one any new file there would get.
    // `~/.dbt/profiles.yml` can hold a warehouse password and is commonly
    // 0600 (0017); a rename alone would hand it back with the temporary
    // file's mode. The in-place fallback below truncates rather than
    // replaces, so it keeps the mode on its own.
    settle_mode(path, &tmp);
    let mut last_error = None;
    for attempt in 0..3 {
        match std::fs::rename(&tmp, path) {
            Ok(()) => return Ok(()),
            Err(e) => {
                last_error = Some(e);
                if attempt < 2 {
                    std::thread::sleep(Duration::from_millis(50));
                }
            }
        }
    }
    // On Windows an indexer or antivirus can hold the target long enough for
    // every rename to fail. Writing in place is less tidy than losing the change.
    let _ = std::fs::remove_file(&tmp);
    std::fs::write(path, bytes).map_err(|e| last_error.unwrap_or(e))
}

/// Writes a file only its owner can read from the moment it exists: the bytes
/// may be a profile's password, and a file created through the umask is
/// readable by every account on the machine until its mode is narrowed, long
/// enough for another account to open it and read what follows. `create_new`,
/// so nothing planted at that name is written through; one left by a crashed
/// process of the same id is removed first.
#[cfg(unix)]
fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let open = || std::fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(path);
    let mut file = match open() {
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            std::fs::remove_file(path)?;
            open()?
        }
        other => other?,
    };
    file.write_all(bytes)
}

/// Windows has no mode: a new file takes its ACL from the directory, which is
/// where the target's came from too.
#[cfg(not(unix))]
fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    std::fs::write(path, bytes)
}

/// Gives `tmp` the mode `target` has, or, for a target that does not exist
/// yet, the mode a file created there gets, learnt from an empty one: the
/// umask is not readable without a system call std does not offer.
#[cfg(unix)]
fn settle_mode(target: &Path, tmp: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let mode = match std::fs::metadata(target) {
        Ok(meta) => Some(meta.permissions().mode()),
        Err(_) => {
            let mut probe_name = tmp.file_name().unwrap_or_default().to_os_string();
            probe_name.push(".mode");
            let probe = tmp.with_file_name(probe_name);
            let mode = std::fs::File::create(&probe).and_then(|f| f.metadata()).map(|m| m.permissions().mode()).ok();
            let _ = std::fs::remove_file(&probe);
            mode
        }
    };
    if let Some(mode) = mode {
        let _ = std::fs::set_permissions(tmp, std::fs::Permissions::from_mode(mode));
    }
}

#[cfg(not(unix))]
fn settle_mode(_target: &Path, _tmp: &Path) {}

pub struct Store {
    pub path: Option<PathBuf>,
    project: String,
    lock: tokio::sync::Mutex<()>,
}

impl Store {
    pub fn new(root: &Path) -> Store {
        let windows = cfg!(windows);
        let root = root.to_string_lossy();
        Store {
            path: config_dir(|key| std::env::var_os(key), windows)
                .map(|dir| dir.join("projects").join(file_name(&root, windows))),
            project: normalise(&root, windows),
            lock: tokio::sync::Mutex::new(()),
        }
    }

    /// Defaults whenever there is nothing usable: no config directory, no file,
    /// a file for another project, or a corrupt one. A corrupt file is reported
    /// and left alone until the user next changes something.
    pub fn load(&self) -> Settings {
        let fresh = || Settings { version: VERSION, project: self.project.clone(), ..Default::default() };
        let Some(path) = &self.path else { return fresh() };
        let Ok(bytes) = std::fs::read(path) else { return fresh() };
        match serde_json::from_slice::<Settings>(&bytes) {
            Ok(s) if s.project == self.project => s,
            Ok(_) => fresh(),
            Err(e) => {
                eprintln!("  settings ignored, {} is not valid JSON: {e}", path.display());
                fresh()
            }
        }
    }

    /// Re-reads, applies the change and writes, all under one lock, so two
    /// requests cannot interleave and drop each other's change.
    pub async fn update(&self, change: impl FnOnce(&mut Settings)) -> Result<Settings, String> {
        let _guard = self.lock.lock().await;
        let mut settings = self.load();
        change(&mut settings);
        settings.version = VERSION;
        settings.project = self.project.clone();
        let Some(path) = &self.path else {
            return Err("no config directory available, settings cannot be saved".into());
        };
        let bytes = serde_json::to_vec_pretty(&settings).map_err(|e| e.to_string())?;
        write_atomic(path, &bytes).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
        Ok(settings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(pairs: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<OsString> {
        move |key| pairs.iter().find(|(k, _)| *k == key).map(|(_, v)| OsString::from(*v))
    }

    #[cfg(unix)]
    #[test]
    fn write_atomic_keeps_a_private_file_private() {
        // `~/.dbt/profiles.yml` holds warehouse credentials and is commonly
        // 0600. Replacing it through a rename used to hand it back at 0644.
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("dbt-edith-mode-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("profiles.yml");
        std::fs::write(&path, b"before\n").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();

        write_atomic(&path, b"after\n").unwrap();

        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "the mode must survive the write");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "after\n");
        // No temp file left behind next to a credential file.
        let strays: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.contains("tmp-"))
            .collect();
        assert!(strays.is_empty(), "left behind {strays:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A new file is created as any other file there would be: the private
    /// mode is the temporary file's alone, never the result's.
    #[cfg(unix)]
    #[test]
    fn write_atomic_creates_a_new_file_as_the_umask_says() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("dbt-edith-mode-new-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("settings.json");
        write_atomic(&path, b"{}\n").unwrap();
        let reference = dir.join("reference");
        std::fs::write(&reference, b"").unwrap();
        let mode = |p: &Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(&path), mode(&reference));
        let strays: Vec<_> = std::fs::read_dir(&dir).unwrap().flatten().filter(|e| e.file_name().to_string_lossy().contains("tmp-")).collect();
        assert!(strays.is_empty(), "the probe and the temporary file are gone");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The temporary file holds the new profile, password included, before it
    /// takes the target's place: it must never be readable by another account,
    /// whatever the umask, and whatever the target's mode is later set back to.
    #[cfg(unix)]
    #[test]
    fn the_temporary_file_is_private_from_its_first_byte() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("dbt-edith-mode-tmp-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let tmp = dir.join("profiles.yml.tmp-1-0");
        write_private(&tmp, b"password: hunter2\n").unwrap();
        assert_eq!(std::fs::metadata(&tmp).unwrap().permissions().mode() & 0o777, 0o600);
        // One left behind, world-readable, is replaced rather than written through.
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o644)).unwrap();
        write_private(&tmp, b"password: hunter3\n").unwrap();
        assert_eq!(std::fs::metadata(&tmp).unwrap().permissions().mode() & 0o777, 0o600);
        assert_eq!(std::fs::read_to_string(&tmp).unwrap(), "password: hunter3\n");

        // A target someone made readable stays as readable: the write changes
        // the content, never who may read it.
        let shared = dir.join("shared.yml");
        std::fs::write(&shared, b"before\n").unwrap();
        std::fs::set_permissions(&shared, std::fs::Permissions::from_mode(0o644)).unwrap();
        write_atomic(&shared, b"after\n").unwrap();
        assert_eq!(std::fs::metadata(&shared).unwrap().permissions().mode() & 0o777, 0o644);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Two requests can write one file at once: both starting the Snowflake
    /// script on a version's first run install it together. Each write
    /// succeeds and the file ends whole, as one of them wrote it.
    #[test]
    fn writes_of_one_file_at_once_all_succeed() {
        let dir = std::env::temp_dir().join(format!("dbt-edith-race-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("sf_lineage.py");
        let threads: Vec<_> = (0..8)
            .map(|t| {
                let path = path.clone();
                std::thread::spawn(move || {
                    for i in 0..25 {
                        write_atomic(&path, format!("writer {t} write {i}\n").repeat(200).as_bytes()).unwrap();
                    }
                })
            })
            .collect();
        for t in threads {
            t.join().expect("no write failed");
        }
        let text = std::fs::read_to_string(&path).unwrap();
        let first = text.lines().next().unwrap().to_string();
        assert!(text.lines().all(|l| l == first) && text.lines().count() == 200, "one whole write");
        let strays: Vec<_> = std::fs::read_dir(&dir).unwrap().flatten().filter(|e| e.file_name().to_string_lossy().contains("tmp-")).collect();
        assert!(strays.is_empty(), "left behind {} temporary files", strays.len());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn fnv1a64_matches_reference_vectors() {
        assert_eq!(format!("{:016x}", fnv1a64(b"")), "cbf29ce484222325");
        assert_eq!(format!("{:016x}", fnv1a64(b"a")), "af63dc4c8601ec8c");
    }

    #[test]
    fn config_dir_prefers_the_explicit_override() {
        let dir = config_dir(env(&[("DBT_EDITH_CONFIG_DIR", "/tmp/lens"), ("HOME", "/home/me")]), false);
        assert_eq!(dir, Some(PathBuf::from("/tmp/lens")));
    }

    #[test]
    fn config_dir_on_unix_uses_xdg_then_home() {
        assert_eq!(config_dir(env(&[("XDG_CONFIG_HOME", "/xdg"), ("HOME", "/home/me")]), false), Some(PathBuf::from("/xdg/dbt-edith")));
        assert_eq!(config_dir(env(&[("HOME", "/home/me")]), false), Some(PathBuf::from("/home/me/.config/dbt-edith")));
    }

    #[test]
    fn config_dir_ignores_empty_and_relative_values() {
        assert_eq!(config_dir(env(&[("XDG_CONFIG_HOME", ""), ("HOME", "/home/me")]), false), Some(PathBuf::from("/home/me/.config/dbt-edith")));
        assert_eq!(config_dir(env(&[("XDG_CONFIG_HOME", "relative/dir")]), false), None);
        assert_eq!(config_dir(env(&[]), false), None);
    }

    #[test]
    fn config_dir_on_windows_uses_appdata_and_never_home() {
        let dir = config_dir(env(&[("APPDATA", r"C:\Users\me\AppData\Roaming"), ("HOME", "/c/Users/me")]), true).unwrap();
        let text = dir.to_string_lossy();
        assert!(text.starts_with(r"C:\Users\me\AppData\Roaming") && text.ends_with("dbt-edith"), "{text}");
        let fallback = config_dir(env(&[("USERPROFILE", r"C:\Users\me"), ("HOME", "/c/Users/me")]), true).unwrap();
        assert!(fallback.to_string_lossy().starts_with(r"C:\Users\me"));
        assert_eq!(config_dir(env(&[("HOME", "/c/Users/me")]), true), None);
    }

    #[test]
    fn normalise_collapses_windows_spellings_of_one_path() {
        let a = normalise(r"\\?\C:\Work\Shop\", true);
        let b = normalise(r"c:/work/shop", true);
        assert_eq!(a, b);
        assert_eq!(normalise(r"\\?\UNC\server\share\shop", true), "//server/share/shop");
    }

    #[test]
    fn normalise_keeps_case_on_unix() {
        assert_eq!(normalise("/Users/me/Shop/", false), "/Users/me/Shop");
        assert_ne!(normalise("/Users/me/Shop", false), normalise("/users/me/shop", false));
    }

    #[test]
    fn file_name_is_readable_and_distinct_per_path() {
        let one = file_name("/a/my project", false);
        let two = file_name("/b/my project", false);
        assert!(one.starts_with("my_project-") && one.ends_with(".json"), "{one}");
        assert_ne!(one, two);
    }

    #[tokio::test]
    async fn update_round_trips_and_rejects_another_projects_file() {
        let dir = std::env::temp_dir().join(format!("dbt-edith-settings-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let store = Store { path: Some(dir.join("p.json")), project: "/work/shop".into(), lock: Default::default() };

        let saved = store
            .update(|s| {
                s.selected = Some(".env.uat".into());
                s.envs.insert(".env.uat".into(), EnvOverride { name: Some("Acceptance".into()), hidden: None });
            })
            .await
            .unwrap();
        assert_eq!(saved.selected.as_deref(), Some(".env.uat"));
        let loaded = store.load();
        assert_eq!(loaded.envs[".env.uat"].name.as_deref(), Some("Acceptance"));

        let other = Store { path: Some(dir.join("p.json")), project: "/work/other".into(), lock: Default::default() };
        assert!(other.load().envs.is_empty(), "a file written for another project must not apply");

        std::fs::write(dir.join("p.json"), b"{ not json").unwrap();
        assert!(store.load().envs.is_empty(), "corrupt settings fall back to defaults");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[tokio::test]
    async fn snowflake_lineage_stays_off_unless_it_was_turned_on() {
        let dir = std::env::temp_dir().join(format!("dbt-edith-settings-sf-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let store = Store { path: Some(dir.join("p.json")), project: "/work/shop".into(), lock: Default::default() };

        // A file written before the switch existed.
        std::fs::write(dir.join("p.json"), br#"{"version":1,"project":"/work/shop","envs":{},"selected":".env.uat"}"#).unwrap();
        assert!(!store.load().snowflake_lineage);

        store.update(|s| s.snowflake_lineage = true).await.unwrap();
        let loaded = store.load();
        assert!(loaded.snowflake_lineage);
        assert_eq!(loaded.selected.as_deref(), Some(".env.uat"), "switching must not touch the selection");

        store.update(|s| s.snowflake_lineage = false).await.unwrap();
        let text = std::fs::read_to_string(dir.join("p.json")).unwrap();
        assert!(!text.contains("snowflake_lineage"), "off is the default, so it is not written: {text}");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[tokio::test]
    async fn snowflake_features_follow_the_adapter_until_chosen() {
        let dir = std::env::temp_dir().join(format!("dbt-edith-settings-feat-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let store = Store { path: Some(dir.join("p.json")), project: "/work/shop".into(), lock: Default::default() };

        // A file written before the choice existed has none.
        std::fs::write(dir.join("p.json"), br#"{"version":1,"project":"/work/shop","snowflake_lineage":true}"#).unwrap();
        assert_eq!(store.load().snowflake_features, None);

        // Off is written out: on a Snowflake project it is not the default.
        store.update(|s| s.snowflake_features = Some(false)).await.unwrap();
        let text = std::fs::read_to_string(dir.join("p.json")).unwrap();
        assert!(text.contains("\"snowflake_features\": false"), "{text}");
        let loaded = store.load();
        assert_eq!(loaded.snowflake_features, Some(false));
        assert!(loaded.snowflake_lineage, "choosing must not touch the picked tool");
        std::fs::remove_dir_all(&dir).unwrap();

        assert!(snowflake_features(None, "snowflake"));
        assert!(snowflake_features(None, "Snowflake"));
        assert!(!snowflake_features(None, "postgres"));
        assert!(!snowflake_features(None, ""), "a manifest naming no adapter offers nothing");
        assert!(!snowflake_features(Some(false), "snowflake"), "the user's choice wins");
        assert!(snowflake_features(Some(true), "bigquery"));
    }
}
