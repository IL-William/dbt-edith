//! Whether `manifest.json` still describes the files on disk.
//!
//! The question is narrow: would `dbt parse` now produce a different manifest
//! than the one the lineage is drawn from? Only file contents decide that, so
//! this compares modification times and opens nothing. git is consulted for one
//! thing: telling a change the user just saved from one that arrived with a
//! checkout or a pull, because the advice differs. Whether the branch has
//! fallen behind the default branch is reported beside that answer, never
//! folded into it: a feature branch behind `main` still has a manifest that is
//! true for the code in front of you.
//!
//! Invariants: nothing here reads a file's contents, runs dbt, or touches the
//! network. The fetch that keeps `origin/main` current is a background task in
//! `api`, and this only reads the refs it leaves behind.

use crate::files::mtime_secs;
use crate::git::{Drift, GitInfo, Head};
use std::collections::BTreeSet;
use std::path::Path;

/// What dbt reads when it parses. `.md` is in because a docs block lives there.
const EXTS: &[&str] = &["sql", "yml", "yaml", "csv", "md"];

/// Read at the project root whatever the resource directories turn out to be.
const ROOT_FILES: &[&str] = &["dbt_project.yml", "packages.yml", "dependencies.yml", "selectors.yml"];

/// Conventional when `macro-paths` is not set, and macros carry no node in the
/// manifest to derive it from the way the other directories are derived.
pub const MACROS_DIR: &str = "macros";

/// dbt writes into these, or vendors into them. A resource root should never
/// be one of them, and the walk refuses to enter them even if one ever is:
/// `target` alone holds more files than the whole project.
const GENERATED: &[&str] = &["target", "logs", "dbt_packages", "node_modules", "__pycache__"];

/// Enough paths to recognise what changed without turning the card into a list.
const SAMPLE: usize = 6;

/// Stop walking here. A project big enough to hit this has a wrong answer
/// either way, and an unbounded walk on a timer is the worse failure.
const MAX_WALK: usize = 60_000;

#[derive(serde::Serialize, Clone, Default)]
pub struct Freshness {
    /// `missing`, `stale`, `edited` or `fresh`. The badge's colour, and the
    /// only field the UI branches on.
    pub state: String,
    pub manifest_at: u64,
    pub age_secs: u64,
    /// Saved but not committed, and so the user's own work in progress.
    pub edited: Vec<String>,
    pub edited_n: usize,
    /// Newer than the manifest yet clean in git: they arrived with a checkout,
    /// a pull or a merge, which is the case the user cannot have in mind.
    pub committed: Vec<String>,
    pub committed_n: usize,
    /// Files the manifest names that are no longer there. A deletion moves no
    /// mtime, so without this it would be the one change that stays invisible.
    pub gone: Vec<String>,
    pub gone_n: usize,
    pub head: Head,
    pub drift: Drift,
    /// Empty when there is nothing to do.
    pub advice: String,
}

/// dbt writes `original_file_path` with a leading `./` in some versions and
/// without it in others. git never uses one, and the two are compared, so the
/// prefix comes off before anything here looks at a path.
pub fn rel(path: &str) -> &str {
    path.strip_prefix("./").unwrap_or(path)
}

/// The directories dbt parses, derived from where the manifest says its own
/// nodes live rather than from `dbt_project.yml`, which would need a parser
/// (0018) to answer a question the manifest has already answered. A project
/// with `macro-paths` set somewhere unconventional is the known gap.
pub fn roots(root: &Path, node_files: &[&str]) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for f in node_files {
        // Normalised here too, not only by the caller: a root derived from a
        // path still carrying dbt's "./" would be "." and the walk would cover
        // the whole project, target/ and all.
        let Some((top, _)) = rel(f).split_once('/') else { continue };
        if top.is_empty() || GENERATED.contains(&top) || top.starts_with('.') {
            continue;
        }
        if root.join(top).is_dir() {
            out.insert(top.to_string());
        }
    }
    if root.join(MACROS_DIR).is_dir() {
        out.insert(MACROS_DIR.to_string());
    }
    out
}

fn parses_into_manifest(name: &str) -> bool {
    match name.rsplit_once('.') {
        Some((_, ext)) => EXTS.contains(&ext.to_ascii_lowercase().as_str()),
        None => false,
    }
}

/// One pass over a resource directory, answering both questions the badge asks
/// of the filesystem: which files are newer than the manifest, and which of the
/// files it names are still there. Two passes would mean two full walks of a
/// directory holding thousands of models.
///
/// `newer` is strictly past `at`: mtime has one-second resolution, so a file
/// saved in the same second as the parse that read it would otherwise flip the
/// badge for a second every time. Dot-directories are skipped whole, because
/// dbt reads from none of them and a touched `.github/workflows/ci.yml` is not
/// a stale manifest.
struct Walk {
    newer: Vec<(u64, String)>,
    seen: BTreeSet<String>,
    /// The walk stopped at `MAX_WALK` rather than running out of files, so
    /// `seen` is a partial answer and nothing may be called missing from it.
    capped: bool,
}

fn walk_under(root: &Path, dir: &str, at: u64, w: &mut Walk, budget: &mut usize) {
    let mut stack = vec![(root.join(dir), dir.to_string())];
    while let Some((path, prefix)) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&path) else { continue };
        for entry in entries.flatten() {
            if *budget == 0 {
                w.capped = true;
                return;
            }
            *budget -= 1;
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') || GENERATED.contains(&name.as_str()) {
                continue;
            }
            let rel = format!("{prefix}/{name}");
            let Ok(meta) = entry.metadata() else { continue };
            if meta.is_dir() {
                stack.push((entry.path(), rel));
            } else if meta.is_file() && parses_into_manifest(&name) {
                let when = mtime_secs(&entry.path());
                if when > at {
                    w.newer.push((when, rel.clone()));
                }
                w.seen.insert(rel);
            }
        }
    }
}

/// The newest file dbt would parse under one resource directory, with its path.
/// Asks of one directory what `check` asks of the whole project, so a single
/// artifact under `target/` can be compared with the things it was built from
/// rather than with the clock.
///
/// Capped like the walk above, and best effort: a project big enough to hit the
/// cap is answered from the part that was walked, which can only under-report a
/// change and never invent one.
pub fn newest_input(root: &Path, dir: &str) -> Option<(u64, String)> {
    let mut best: Option<(u64, String)> = None;
    let mut budget = MAX_WALK;
    let mut stack = vec![(root.join(dir), dir.to_string())];
    while let Some((path, prefix)) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&path) else { continue };
        for entry in entries.flatten() {
            if budget == 0 {
                return best;
            }
            budget -= 1;
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') || GENERATED.contains(&name.as_str()) {
                continue;
            }
            let rel = format!("{prefix}/{name}");
            let Ok(meta) = entry.metadata() else { continue };
            if meta.is_dir() {
                stack.push((entry.path(), rel));
            } else if meta.is_file() && parses_into_manifest(&name) {
                let when = mtime_secs(&entry.path());
                if best.as_ref().map(|(b, _)| when > *b).unwrap_or(true) {
                    best = Some((when, rel));
                }
            }
        }
    }
    best
}

/// Whether git has this path as changed. Entries ending in `/` are how git
/// reports a collapsed directory of untracked files, so a prefix counts too.
pub fn is_dirty(path: &str, git: &GitInfo) -> bool {
    git.modified
        .iter()
        .chain(git.untracked.iter())
        .any(|e| e == path || (e.ends_with('/') && path.starts_with(e.as_str())))
}

/// Sorts the paths newer than the manifest into the user's own uncommitted work
/// and everything else, which by elimination arrived already committed.
pub fn classify(newer: Vec<String>, git: &GitInfo) -> (Vec<String>, Vec<String>) {
    // Without a repository nothing can be called committed, and calling a whole
    // project stale on a checkout that never happened would be the worse guess.
    if !git.repo {
        return (newer, Vec::new());
    }
    newer.into_iter().partition(|p| is_dirty(p, git))
}

fn sample(all: &[String]) -> Vec<String> {
    all.iter().take(SAMPLE).cloned().collect()
}

/// What to do about it, and nothing else. The card above this line has already
/// said what the state is and why, so repeating it here would cost the one line
/// that is worth reading twice.
pub fn advise(state: &str, drift: &Drift) -> String {
    match state {
        "missing" => "Run dbt parse to create one.".to_string(),
        "stale" => "Run dbt parse before trusting this lineage.".to_string(),
        "edited" => "Run dbt parse to see them.".to_string(),
        // Nothing is wrong with the manifest, so the only thing left to suggest
        // is catching the branch up before reading the graph as production.
        _ if drift.behind > 0 => format!("Pull {}, then parse again.", drift.base),
        _ => String::new(),
    }
}

/// `manifest_at` is the manifest's own mtime, already held in the graph's meta,
/// and `node_files` the project-relative file of every node in it that belongs
/// to this project. Nodes from installed packages are the caller's to leave
/// out: their paths are relative to the package, not to the project, so every
/// one of them would read as a file that has gone missing.
pub fn check(root: &Path, manifest_at: u64, node_files: &[String], git: &GitInfo) -> Freshness {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    // Asked only of a repository: outside one every git call would spawn a
    // subprocess to fail, four times over, on a poll that repeats every few
    // seconds. The badge works without git, it just has less to say.
    let (head, drift) = if git.repo {
        (crate::git::head(root), crate::git::drift(root))
    } else {
        Default::default()
    };
    let mut f = Freshness { manifest_at, age_secs: now.saturating_sub(manifest_at), head, drift, ..Default::default() };
    if manifest_at == 0 {
        f.state = "missing".into();
        f.age_secs = 0;
        f.advice = advise(&f.state, &f.drift);
        return f;
    }

    let files: Vec<&str> = node_files.iter().map(|p| rel(p)).collect();
    let dirs = roots(root, &files);
    let mut w = Walk { newer: Vec::new(), seen: BTreeSet::new(), capped: false };
    let mut budget = MAX_WALK;
    for dir in &dirs {
        walk_under(root, dir, manifest_at, &mut w, &mut budget);
    }
    for name in ROOT_FILES {
        let at = mtime_secs(&root.join(name));
        if at > manifest_at {
            w.newer.push((at, (*name).to_string()));
        }
    }
    // Newest first: the six shown are then the six that explain the badge best.
    w.newer.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    let newer: Vec<String> = w.newer.into_iter().map(|(_, p)| p).collect();

    let (edited, committed) = classify(newer, git);
    // Only under a directory the walk covered. A node configured in
    // `dbt_project.yml` names that file as its own, and a file the walk never
    // looked at cannot be reported as one it failed to find.
    let walked = |p: &str| p.split_once('/').is_some_and(|(top, _)| dirs.contains(top));
    let gone: Vec<String> = if w.capped {
        Vec::new()
    } else {
        files
            .iter()
            .filter(|p| walked(p) && !w.seen.contains(**p))
            .map(|p| p.to_string())
            .collect()
    };

    f.edited_n = edited.len();
    f.committed_n = committed.len();
    f.gone_n = gone.len();
    f.edited = sample(&edited);
    f.committed = sample(&committed);
    f.gone = sample(&gone);

    // A vanished file is a committed change as far as advice goes: it is not
    // something the user is holding unsaved, it is the project having moved.
    f.state = if f.committed_n > 0 || f.gone_n > 0 {
        "stale"
    } else if f.edited_n > 0 {
        "edited"
    } else {
        "fresh"
    }
    .into();
    f.advice = advise(&f.state, &f.drift);
    f
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::GitInfo;

    fn repo(modified: &[&str], untracked: &[&str]) -> GitInfo {
        GitInfo {
            repo: true,
            modified: modified.iter().map(|s| s.to_string()).collect(),
            untracked: untracked.iter().map(|s| s.to_string()).collect(),
            ..Default::default()
        }
    }

    fn v(paths: &[&str]) -> Vec<String> {
        paths.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn only_dbt_inputs_count() {
        assert!(parses_into_manifest("orders.sql"));
        assert!(parses_into_manifest("schema.yml"));
        assert!(parses_into_manifest("sources.yaml"));
        assert!(parses_into_manifest("countries.csv"));
        assert!(parses_into_manifest("docs.md"));
        assert!(parses_into_manifest("ORDERS.SQL"));
        assert!(!parses_into_manifest("manifest.json"));
        assert!(!parses_into_manifest("README"));
        assert!(!parses_into_manifest("run.py"));
    }

    #[test]
    fn a_saved_edit_is_the_users_own_work() {
        let git = repo(&["models/marts/orders.sql"], &[]);
        let (edited, committed) = classify(v(&["models/marts/orders.sql"]), &git);
        assert_eq!(edited, v(&["models/marts/orders.sql"]));
        assert!(committed.is_empty());
    }

    #[test]
    fn a_clean_file_newer_than_the_manifest_arrived_committed() {
        let git = repo(&["models/marts/orders.sql"], &[]);
        let (edited, committed) = classify(v(&["models/marts/orders.sql", "models/stg/stg_customers.sql"]), &git);
        assert_eq!(edited, v(&["models/marts/orders.sql"]));
        assert_eq!(committed, v(&["models/stg/stg_customers.sql"]));
    }

    #[test]
    fn a_collapsed_untracked_directory_covers_what_is_under_it() {
        let git = repo(&[], &["models/new_mart/"]);
        assert!(is_dirty("models/new_mart/revenue.sql", &git));
        assert!(!is_dirty("models/new_marts_elsewhere.sql", &git));
    }

    #[test]
    fn outside_a_repository_git_is_not_asked_anything() {
        let dir = std::env::temp_dir().join(format!("edith-fresh-nogit-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = check(&dir, 2_000_000_000, &[], &GitInfo::default());
        assert_eq!(f.state, "fresh");
        assert_eq!(f.head.sha, "");
        assert_eq!(f.drift.base, "");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn outside_a_repository_nothing_is_called_committed() {
        let (edited, committed) = classify(v(&["models/marts/orders.sql"]), &GitInfo::default());
        assert_eq!(edited.len(), 1);
        assert!(committed.is_empty());
    }

    #[test]
    fn advice_is_the_action_and_not_the_diagnosis() {
        let level = Drift { base: "origin/main".into(), behind: 0, fetched_at: 0 };
        let behind = Drift { base: "origin/main".into(), behind: 4, fetched_at: 0 };
        // Nothing to do, so nothing is said.
        assert_eq!(advise("fresh", &level), "");
        assert_eq!(advise("fresh", &behind), "Pull origin/main, then parse again.");
        assert_eq!(advise("edited", &level), "Run dbt parse to see them.");
        // Drift never displaces the reason the lineage itself cannot be trusted.
        assert_eq!(advise("stale", &behind), "Run dbt parse before trusting this lineage.");
    }

    #[test]
    fn a_missing_manifest_has_no_age() {
        let dir = std::env::temp_dir().join(format!("edith-fresh-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = check(&dir, 0, &[], &GitInfo::default());
        assert_eq!(f.state, "missing");
        assert_eq!(f.age_secs, 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_file_saved_after_the_parse_makes_the_manifest_edited() {
        let dir = std::env::temp_dir().join(format!("edith-fresh-edit-{}", std::process::id()));
        let models = dir.join("models");
        std::fs::create_dir_all(&models).unwrap();
        std::fs::write(models.join("orders.sql"), "select 1").unwrap();
        let at = mtime_secs(&models.join("orders.sql")) - 60;

        let clean = check(&dir, at, &v(&["models/orders.sql"]), &repo(&[], &[]));
        assert_eq!(clean.state, "stale", "clean in git means it arrived committed");
        assert_eq!(clean.committed, v(&["models/orders.sql"]));

        let dirty = check(&dir, at, &v(&["models/orders.sql"]), &repo(&["models/orders.sql"], &[]));
        assert_eq!(dirty.state, "edited");
        assert_eq!(dirty.edited, v(&["models/orders.sql"]));

        let untouched = check(&dir, at + 3600, &v(&["models/orders.sql"]), &repo(&[], &[]));
        assert_eq!(untouched.state, "fresh");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_node_whose_file_is_gone_is_stale_though_no_mtime_moved() {
        let dir = std::env::temp_dir().join(format!("edith-fresh-gone-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("models")).unwrap();
        std::fs::write(dir.join("models/kept.sql"), "select 1").unwrap();
        let f = check(&dir, 2_000_000_000, &v(&["models/kept.sql", "models/deleted.sql"]), &repo(&[], &[]));
        assert_eq!(f.state, "stale");
        assert_eq!(f.gone, v(&["models/deleted.sql"]));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_node_configured_in_dbt_project_yml_has_not_gone_missing() {
        // dbt gives such a node `dbt_project.yml` as its own file. It sits at
        // the project root, which no resource walk covers, and reporting it as
        // deleted turned every project with one permanently red.
        let dir = std::env::temp_dir().join(format!("edith-fresh-cfg-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("models")).unwrap();
        std::fs::write(dir.join("models/orders.sql"), "select 1").unwrap();
        std::fs::write(dir.join("dbt_project.yml"), "name: shop").unwrap();
        let f = check(&dir, 2_000_000_000, &v(&["models/orders.sql", "dbt_project.yml"]), &repo(&[], &[]));
        assert_eq!(f.gone_n, 0);
        assert_eq!(f.state, "fresh");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn dbts_leading_dot_slash_comes_off_before_anything_compares_a_path() {
        // The bug this guards: with "./models/orders.sql" left as dbt wrote it,
        // the resource root read as "." and the walk covered target/ and
        // dbt_packages/, while every path failed to match what git reports.
        assert_eq!(rel("./models/orders.sql"), "models/orders.sql");
        assert_eq!(rel("models/orders.sql"), "models/orders.sql");
        let git = repo(&["models/orders.sql"], &[]);
        assert!(is_dirty(rel("./models/orders.sql"), &git));
    }

    #[test]
    fn a_resource_root_is_never_a_generated_directory() {
        let dir = std::env::temp_dir().join(format!("edith-fresh-roots-{}", std::process::id()));
        for d in ["models", "macros", "target", "dbt_packages", ".github"] {
            std::fs::create_dir_all(dir.join(d)).unwrap();
        }
        let found = roots(&dir, &["./models/a.sql", "target/x.sql", "dbt_packages/pkg/b.sql", ".github/c.yml", "gone/d.sql"]);
        assert_eq!(found.into_iter().collect::<Vec<_>>(), v(&["macros", "models"]));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_dot_directory_is_not_a_dbt_directory() {
        let dir = std::env::temp_dir().join(format!("edith-fresh-dot-{}", std::process::id()));
        let models = dir.join("models");
        std::fs::create_dir_all(models.join(".hidden")).unwrap();
        std::fs::write(models.join("kept.sql"), "select 1").unwrap();
        std::fs::write(models.join(".hidden/ci.yml"), "on: push").unwrap();
        let at = mtime_secs(&models.join("kept.sql")) - 60;

        let mut w = Walk { newer: Vec::new(), seen: BTreeSet::new(), capped: false };
        let mut budget = MAX_WALK;
        walk_under(&dir, "models", at, &mut w, &mut budget);
        assert_eq!(w.newer.iter().map(|(_, p)| p.clone()).collect::<Vec<_>>(), v(&["models/kept.sql"]));
        assert!(!w.seen.contains("models/.hidden/ci.yml"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
