//! The per-node SQL dbt leaves under `target/`, and how fresh it is.
//!
//! Two artifacts, one concern: `compiled/` holds the model's SQL with the Jinja
//! gone, `run/` holds that same SQL wrapped in the statement dbt executed to
//! materialise it. They answer different questions ("what does this model say"
//! against "what did dbt actually send to the warehouse"), so neither is ever
//! shown in the other's place.
//!
//! The Fusion manifest carries no `compiled_path`, so the location is derived
//! from dbt's layout and probed. Every candidate tried is reported, which makes
//! a wrong guess obvious instead of silent.

use std::path::{Path, PathBuf};

/// Which of the two directories under `target/` is being read.
#[derive(Clone, Copy, PartialEq)]
pub enum Kind {
    Compiled,
    Run,
}

impl Kind {
    pub fn dir(self) -> &'static str {
        match self {
            Kind::Compiled => "compiled",
            Kind::Run => "run",
        }
    }
}

/// Changing it changes compiled SQL, through `vars:` and through the config
/// blocks that decide materialization and location.
const PROJECT_FILE: &str = "dbt_project.yml";

/// The caller is a query string, so an unknown name is refused rather than
/// defaulted: guessing would silently show one artifact under the other's name.
pub fn kind(name: &str) -> Option<Kind> {
    match name {
        "compiled" => Some(Kind::Compiled),
        "run" => Some(Kind::Run),
        _ => None,
    }
}

#[derive(serde::Serialize, Default)]
pub struct CompiledInfo {
    /// `compiled` or `run`, so a late answer cannot be painted into the wrong
    /// pane when both are asked for at once.
    pub kind: String,
    pub found: bool,
    /// Absolute, for the tooltip.
    pub path: String,
    /// The same file relative to the project root, which is what the file tree
    /// is keyed by. Empty when `target` sits outside the project, as it does
    /// when `--manifest` points elsewhere: there is then no tree row to reveal.
    pub rel: String,
    /// Paths probed, shown when nothing was found.
    pub candidates: Vec<String>,
    pub compiled_at: u64,
    pub age_secs: u64,
    pub source_at: u64,
    pub stale: bool,
    /// What this file was built from that has moved since, as subjects: "the
    /// model file", "dbt_project.yml". Kept apart from `reasons` so the UI can
    /// introduce the list once rather than repeat "changed after this was run"
    /// behind every entry, which at five entries is a wall and not a sentence.
    pub changed: Vec<String>,
    /// Anything wrong with the file that is not one of its inputs moving.
    pub reasons: Vec<String>,
    pub content: String,
    pub truncated: bool,
    pub bytes: u64,
}

fn mtime(path: &Path) -> u64 {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Where dbt would have written this artifact for this node. The packaged path
/// comes first: a project and an installed package can hold a model of the same
/// name, and the packaged one is the node we were asked about.
fn candidates(target: &Path, kind: Kind, package: &str, file: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if !package.is_empty() {
        out.push(target.join(kind.dir()).join(package).join(file));
    }
    out.push(target.join(kind.dir()).join(file));
    out
}

/// Project-relative and slash-separated, or empty when the file is not under
/// the project at all. The file tree only opens what is inside the root, so a
/// path it could never reveal is better reported as none than as a dead link.
fn relative(root: &Path, path: &Path) -> String {
    match path.strip_prefix(root) {
        Ok(rel) => rel.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/"),
        Err(_) => String::new(),
    }
}

pub fn look_up(
    root: &Path,
    target: &Path,
    kind: Kind,
    package: &str,
    file: &str,
    yml: &str,
    max_bytes: u64,
) -> CompiledInfo {
    let mut info = CompiledInfo { kind: kind.dir().to_string(), ..Default::default() };
    let tried = candidates(target, kind, package, file);
    let hit = tried.iter().find(|p| p.is_file());

    let Some(path) = hit else {
        info.candidates = tried.iter().map(|p| p.display().to_string()).collect();
        return info;
    };

    info.found = true;
    info.path = path.display().to_string();
    info.rel = relative(root, path);
    info.compiled_at = mtime(path);
    info.bytes = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    info.age_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs().saturating_sub(info.compiled_at))
        .unwrap_or(0);

    // Everything dbt reads to produce this file, named so the reason can say
    // which one moved. Never the clock: a file nothing has touched since is
    // still correct at any age, and an age threshold only ever asks the reader
    // to ignore it (0025).
    let mut inputs = vec![(mtime(&root.join(file)), "the model file".to_string())];
    if !yml.is_empty() {
        inputs.push((mtime(&root.join(yml)), "the schema file".to_string()));
    }
    inputs.push((mtime(&root.join(PROJECT_FILE)), PROJECT_FILE.to_string()));
    // Any macro, not the ones this node calls. The manifest's `depends_on` lists
    // the macros reached while parsing the node, which leaves out the ones those
    // in turn call, and most of what it does list is `macro.dbt.*`, built into
    // dbt-core with no file to stat. A macro edit makes every compiled file
    // suspect until it is rebuilt, which is also what dbt does about it.
    if let Some((when, path)) = crate::freshness::newest_input(root, crate::freshness::MACROS_DIR) {
        let name = path.rsplit('/').next().unwrap_or(&path).to_string();
        inputs.push((when, format!("the macro {name}")));
    }
    // dbt writes `compiled/` on every compile but `run/` only on a run, so the
    // compiled sibling being newer is the run file's own question answered:
    // this model has been compiled since it was last executed.
    if kind == Kind::Run {
        let sibling = candidates(target, Kind::Compiled, package, file)
            .iter()
            .find(|p| p.is_file())
            .map(|p| mtime(p))
            .unwrap_or(0);
        if sibling > info.compiled_at {
            info.reasons.push("compiled again after this was run".into());
        }
        info.source_at = sibling;
    }

    for (when, what) in inputs {
        info.source_at = info.source_at.max(when);
        if when > info.compiled_at {
            info.changed.push(what);
        }
    }
    info.stale = !info.changed.is_empty() || !info.reasons.is_empty();

    match std::fs::read(path) {
        Ok(bytes) => {
            info.truncated = bytes.len() as u64 > max_bytes;
            let slice = &bytes[..bytes.len().min(max_bytes as usize)];
            info.content = String::from_utf8_lossy(slice).into_owned();
        }
        Err(e) => info.reasons.push(format!("cannot read it: {e}")),
    }
    info
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static N: AtomicU32 = AtomicU32::new(0);

    /// A project holding one model, with whichever artifacts the test names.
    fn project(artifacts: &[(&str, &str)]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "edith-compiled-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("models/marts")).unwrap();
        std::fs::write(dir.join("models/marts/orders.sql"), "select 1").unwrap();
        for (rel, body) in artifacts {
            let path = dir.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, body).unwrap();
        }
        dir
    }

    fn look(dir: &Path, kind: Kind) -> CompiledInfo {
        look_up(dir, &dir.join("target"), kind, "shop", "models/marts/orders.sql", "", 1 << 20)
    }

    /// Backdate rather than sleep: mtime has one-second resolution and a test
    /// that waits a second is a test nobody runs.
    fn backdate(dir: &Path, rel: &str, secs: u64) {
        let when = std::time::SystemTime::now() - std::time::Duration::from_secs(secs);
        std::fs::File::options().write(true).open(dir.join(rel)).unwrap().set_modified(when).unwrap();
    }

    #[test]
    fn neither_artifact_ever_answers_for_the_other() {
        // The bug this guards: one probe over compiled/ then run/, first hit
        // wins, which showed the executed statement under the Compiled tab
        // whenever dbt had run the model but not compiled it since.
        let dir = project(&[("target/run/shop/models/marts/orders.sql", "create table as select 1")]);
        let run = look(&dir, Kind::Run);
        assert!(run.found);
        assert_eq!(run.content, "create table as select 1");

        let compiled = look(&dir, Kind::Compiled);
        assert!(!compiled.found);
        assert!(compiled.candidates.iter().all(|c| c.contains("compiled")), "{:?}", compiled.candidates);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_packaged_path_is_tried_before_the_bare_one() {
        let dir = project(&[
            ("target/compiled/shop/models/marts/orders.sql", "the project's"),
            ("target/compiled/models/marts/orders.sql", "someone else's"),
        ]);
        assert_eq!(look(&dir, Kind::Compiled).content, "the project's");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn every_path_probed_is_reported_when_nothing_is_found() {
        let dir = project(&[]);
        let info = look(&dir, Kind::Compiled);
        assert!(!info.found);
        assert_eq!(info.candidates.len(), 2, "the packaged path and the bare one");
        assert!(info.rel.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_relative_path_is_what_the_file_tree_is_keyed_by() {
        let dir = project(&[("target/compiled/shop/models/marts/orders.sql", "select 1")]);
        assert_eq!(look(&dir, Kind::Compiled).rel, "target/compiled/shop/models/marts/orders.sql");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_target_outside_the_project_has_no_tree_row_to_point_at() {
        // What `--manifest /elsewhere/manifest.json` produces. The file is still
        // read and still shown; only the link into the tree is withheld.
        let dir = project(&[]);
        let away = project(&[("compiled/shop/models/marts/orders.sql", "select 1")]);
        let info = look_up(&dir, &away, Kind::Compiled, "shop", "models/marts/orders.sql", "", 1 << 20);
        assert!(info.found);
        assert!(!info.path.is_empty());
        assert!(info.rel.is_empty(), "nothing under the root to reveal");
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&away);
    }

    #[test]
    fn a_source_saved_after_the_artifact_is_the_reason_that_bites() {
        let dir = project(&[("target/compiled/shop/models/marts/orders.sql", "select 1")]);
        backdate(&dir, "target/compiled/shop/models/marts/orders.sql", 120);

        let info = look(&dir, Kind::Compiled);
        assert!(info.stale);
        assert_eq!(info.changed, vec!["the model file"]);
        assert!(info.reasons.is_empty(), "an input moving is not a fault in the file");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn age_alone_is_never_a_reason() {
        // The rule this replaced: compiled SQL went amber after an hour and a
        // run after a day, whatever the files said. A week-old file that
        // nothing has touched since is still exactly what dbt would write.
        let dir = project(&[
            ("target/compiled/shop/models/marts/orders.sql", "select 1"),
            ("target/run/shop/models/marts/orders.sql", "create table as select 1"),
        ]);
        backdate(&dir, "models/marts/orders.sql", 9 * 86400);
        backdate(&dir, "target/compiled/shop/models/marts/orders.sql", 8 * 86400);
        backdate(&dir, "target/run/shop/models/marts/orders.sql", 8 * 86400);

        for kind in [Kind::Compiled, Kind::Run] {
            let info = look(&dir, kind);
            assert!(!info.stale, "{:?} {:?}", info.changed, info.reasons);
            assert!(info.age_secs > 7 * 86400, "still reported, just not a reason");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_project_file_is_an_input_too() {
        // vars: and the config blocks that decide materialization are in it, so
        // saving it can change what dbt would compile without touching a model.
        let dir = project(&[
            ("target/compiled/shop/models/marts/orders.sql", "select 1"),
            ("dbt_project.yml", "name: shop"),
        ]);
        backdate(&dir, "models/marts/orders.sql", 300);
        backdate(&dir, "target/compiled/shop/models/marts/orders.sql", 120);

        let info = look(&dir, Kind::Compiled);
        assert_eq!(info.changed, vec!["dbt_project.yml"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_macro_edit_makes_the_compiled_sql_suspect() {
        // Named, because "something changed" sends the reader looking. Any
        // macro counts, not the ones the node declares: see the comment on the
        // walk for why the manifest's own list cannot be trusted here.
        let dir = project(&[
            ("target/compiled/shop/models/marts/orders.sql", "select 1"),
            ("macros/dates/fiscal_year.sql", "{% macro fiscal_year() %}{% endmacro %}"),
        ]);
        backdate(&dir, "models/marts/orders.sql", 300);
        backdate(&dir, "target/compiled/shop/models/marts/orders.sql", 120);

        let info = look(&dir, Kind::Compiled);
        assert_eq!(info.changed, vec!["the macro fiscal_year.sql"]);

        // And an untouched macro says nothing.
        backdate(&dir, "macros/dates/fiscal_year.sql", 300);
        assert!(!look(&dir, Kind::Compiled).stale);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_run_is_behind_once_the_model_has_been_compiled_again() {
        // The run tab's own question, and one only it can ask: dbt writes
        // compiled/ on every compile and run/ only on a run.
        let dir = project(&[
            ("target/compiled/shop/models/marts/orders.sql", "select 1"),
            ("target/run/shop/models/marts/orders.sql", "create table as select 1"),
        ]);
        backdate(&dir, "models/marts/orders.sql", 900);
        backdate(&dir, "target/run/shop/models/marts/orders.sql", 600);

        let run = look(&dir, Kind::Run);
        assert_eq!(run.reasons, vec!["compiled again after this was run"]);
        assert_eq!(run.source_at, mtime(&dir.join("target/compiled/shop/models/marts/orders.sql")));

        // The compiled tab never asks it: it is the thing being compared.
        assert!(!look(&dir, Kind::Compiled).stale);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_schema_file_counts_as_the_source_too() {
        let dir = project(&[
            ("target/compiled/shop/models/marts/orders.sql", "select 1"),
            ("models/marts/schema.yml", "version: 2"),
        ]);
        for p in ["target/compiled/shop/models/marts/orders.sql", "models/marts/orders.sql"] {
            backdate(&dir, p, 120);
        }
        let info = look_up(
            &dir,
            &dir.join("target"),
            Kind::Compiled,
            "shop",
            "models/marts/orders.sql",
            "models/marts/schema.yml",
            1 << 20,
        );
        assert_eq!(info.changed, vec!["the schema file"]);
        assert_eq!(info.source_at, mtime(&dir.join("models/marts/schema.yml")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_unknown_kind_is_refused_rather_than_defaulted() {
        assert!(kind("compiled").is_some());
        assert!(kind("run").is_some());
        assert!(kind("").is_none());
        assert!(kind("Run").is_none());
        assert!(kind("../compiled").is_none());
    }
}
