//! The per-node SQL dbt leaves under `target/`, and how fresh it is.
//!
//! Two artifacts, one concern: `compiled/` holds the model's SQL with the Jinja
//! gone, `run/` holds that same SQL wrapped in the statement dbt executed to
//! materialise it. They answer different questions ("what does this model say"
//! against "what did dbt actually send to the warehouse"), so neither is ever
//! shown in the other's place.
//!
//! dbt records a `compiled_path` per node, and where it does that is the answer:
//! a generic test's file name is a truncated generated name plus a hash, which
//! nothing here could reconstruct. Some manifests carry none, so the location is
//! still derived from dbt's layout and probed when it is missing. Every
//! candidate tried is reported, which makes a wrong guess obvious instead of
//! silent.

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

/// The node an artifact is looked up for: what dbt recorded about where it put
/// it, and what the layout can be derived from when dbt recorded nothing.
pub struct Subject<'a> {
    pub package: &'a str,
    pub file: &'a str,
    pub yml: &'a str,
    /// dbt's own `compiled_path`, project-relative and slashed, or empty.
    pub compiled: &'a str,
    /// dbt's generated name, and the alias it writes the file under when that
    /// name was too long to use whole. Both are guesses at a generic test's
    /// file name, tried only when `compiled` is empty.
    pub name: &'a str,
    pub alias: &'a str,
}

/// A generic test has no file of its own: it is declared in a schema file, and
/// that is what the manifest gives as its path.
fn is_schema(file: &str) -> bool {
    file.ends_with(".yml") || file.ends_with(".yaml")
}

/// `compiled_path` minus the directory it names, so what is left can be joined
/// onto the target directory actually in use. The two need not agree, since
/// `--manifest` elsewhere puts the target where dbt never wrote, and only the
/// part inside is dbt's to decide. The first component is that directory,
/// whatever `target-path` happens to call it.
fn inside_target(compiled: &str) -> Option<&str> {
    match compiled.split_once('/') {
        Some((_, rest)) if !rest.is_empty() => Some(rest),
        _ => None,
    }
}

/// That same path under the other of the two directories. dbt records one
/// `compiled_path` and nothing at all for `run/`, but it mirrors the layout into
/// both, so swapping the leading component says what the record does not.
fn under(inside: &str, kind: Kind) -> String {
    match inside.split_once('/') {
        Some((first, rest)) if first == Kind::Compiled.dir() => format!("{}/{rest}", kind.dir()),
        _ => inside.to_string(),
    }
}

/// Where dbt would have written this artifact for this node. Its own record
/// comes first where it left one. Then the packaged path before the bare one: a
/// project and an installed package can hold a model of the same name, and the
/// packaged one is the node we were asked about.
fn candidates(target: &Path, kind: Kind, s: &Subject) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(inside) = inside_target(s.compiled) {
        out.push(target.join(under(inside, kind)));
    }
    let dir = target.join(kind.dir());
    // dbt writes a generic test into a directory named after the schema file
    // that declares it, one .sql per test. Only ever reached with no
    // compiled_path to read, because the file name is then a guess: the
    // generated name, or the alias dbt shortened it to.
    if is_schema(s.file) {
        for stem in [s.name, s.alias] {
            if stem.is_empty() {
                continue;
            }
            let leaf = format!("{}/{stem}.sql", s.file);
            if !s.package.is_empty() {
                out.push(dir.join(s.package).join(&leaf));
            }
            out.push(dir.join(&leaf));
        }
    }
    if !s.package.is_empty() {
        out.push(dir.join(s.package).join(s.file));
    }
    out.push(dir.join(s.file));
    out.dedup();
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

pub fn look_up(root: &Path, target: &Path, kind: Kind, s: &Subject, max_bytes: u64) -> CompiledInfo {
    let mut info = CompiledInfo { kind: kind.dir().to_string(), ..Default::default() };
    let tried = candidates(target, kind, s);
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
    // A generic test's own file is the schema file that declares it, so calling
    // that one "the model file" would name it as something it is not.
    let own = if is_schema(s.file) { "the schema file" } else { "the model file" };
    let mut inputs = vec![(mtime(&root.join(s.file)), own.to_string())];
    if !s.yml.is_empty() && s.yml != s.file {
        inputs.push((mtime(&root.join(s.yml)), "the schema file".to_string()));
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
        let sibling = candidates(target, Kind::Compiled, s)
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

    fn subject<'a>(file: &'a str) -> Subject<'a> {
        Subject { package: "shop", file, yml: "", compiled: "", name: "", alias: "" }
    }

    fn look(dir: &Path, kind: Kind) -> CompiledInfo {
        look_up(dir, &dir.join("target"), kind, &subject("models/marts/orders.sql"), 1 << 20)
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
        let info = look_up(&dir, &away, Kind::Compiled, &subject("models/marts/orders.sql"), 1 << 20);
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
            &Subject { yml: "models/marts/schema.yml", ..subject("models/marts/orders.sql") },
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

    /// The file name of a generic test is dbt's, not ours: a long generated
    /// name is truncated and given a hash, so the only way to it is the path
    /// dbt recorded. Nothing derived from the node could produce this one.
    #[test]
    fn the_path_dbt_recorded_is_what_finds_a_hashed_generic_test() {
        let hashed = "target/compiled/shop/target/generic_tests/relationships_orders_cus_ab12cd34ef56.sql";
        let dir = project(&[(hashed, "select customer_id from orders")]);
        let s = Subject {
            file: "models/marts/schema.yml",
            compiled: hashed,
            name: "relationships_orders_customer_id__customer_id__ref_customers_",
            ..subject("models/marts/schema.yml")
        };
        let info = look_up(&dir, &dir.join("target"), Kind::Compiled, &s, 1 << 20);
        assert!(info.found, "tried {:?}", info.candidates);
        assert!(info.path.ends_with("relationships_orders_cus_ab12cd34ef56.sql"));
        assert_eq!(info.content, "select customer_id from orders");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// dbt records a compiled_path and nothing for `run/`, so the run file is
    /// found by reading that one under the other directory. Without it a
    /// generic test would have a Compiled tab and never a Run one.
    #[test]
    fn the_run_file_is_the_recorded_path_under_the_other_directory() {
        let compiled = "target/compiled/shop/target/generic_tests/not_null_orders_id.sql";
        let dir = project(&[("target/run/shop/target/generic_tests/not_null_orders_id.sql", "select 1")]);
        let s = Subject { compiled, ..subject("models/marts/schema.yml") };
        let run = look_up(&dir, &dir.join("target"), Kind::Run, &s, 1 << 20);
        assert!(run.found, "tried {:?}", run.candidates);
        assert!(run.path.contains("/run/"));
        // And the compiled one is still absent, rather than answered by the run.
        let compiled = look_up(&dir, &dir.join("target"), Kind::Compiled, &s, 1 << 20);
        assert!(!compiled.found);
        assert!(compiled.candidates.iter().all(|c| c.contains("compiled")), "{:?}", compiled.candidates);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// `--manifest elsewhere` puts the target where dbt never wrote, so the
    /// directory the recorded path names is dropped and only the layout inside
    /// it is kept. Joining the path whole would look under a target that is not
    /// the one being read.
    #[test]
    fn a_recorded_path_is_read_under_the_target_actually_in_use() {
        let dir = project(&[("elsewhere/compiled/shop/target/generic_tests/unique_orders_id.sql", "select 1")]);
        let s = Subject {
            compiled: "target/compiled/shop/target/generic_tests/unique_orders_id.sql",
            ..subject("models/marts/schema.yml")
        };
        let info = look_up(&dir, &dir.join("elsewhere"), Kind::Compiled, &s, 1 << 20);
        assert!(info.found, "tried {:?}", info.candidates);
        assert_eq!(info.rel, "elsewhere/compiled/shop/target/generic_tests/unique_orders_id.sql");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A manifest carrying no compiled_path leaves the layout to be derived:
    /// dbt writes a generic test into a directory named after the schema file
    /// that declares it, one .sql per test.
    #[test]
    fn a_generic_test_without_a_recorded_path_is_looked_for_beside_its_schema_file() {
        let dir = project(&[(
            "target/compiled/shop/models/marts/schema.yml/not_null_orders_id.sql",
            "select 1",
        )]);
        let by_name = Subject { name: "not_null_orders_id", ..subject("models/marts/schema.yml") };
        assert!(look_up(&dir, &dir.join("target"), Kind::Compiled, &by_name, 1 << 20).found);

        // The alias is tried too: dbt writes the file under the shortened name
        // when the generated one was too long to use whole.
        let by_alias = Subject {
            name: "not_null_orders_a_very_long_generated_name_nobody_would_guess",
            alias: "not_null_orders_id",
            ..subject("models/marts/schema.yml")
        };
        assert!(look_up(&dir, &dir.join("target"), Kind::Compiled, &by_alias, 1 << 20).found);

        // And a miss reports every path it tried, including both guesses.
        let miss = Subject { name: "unique_orders_id", ..subject("models/marts/schema.yml") };
        let info = look_up(&dir, &dir.join("target"), Kind::Compiled, &miss, 1 << 20);
        assert!(!info.found);
        assert!(info.candidates.iter().any(|c| c.ends_with("schema.yml/unique_orders_id.sql")), "{:?}", info.candidates);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A generic test's own file is a schema file. Naming it "the model file"
    /// in the freshness bar would name it as something it is not, and the model
    /// it guards is a different file again.
    #[test]
    fn a_generic_tests_own_file_is_named_as_the_schema_file() {
        let dir = project(&[
            ("target/compiled/shop/models/marts/schema.yml/not_null_orders_id.sql", "select 1"),
            ("models/marts/schema.yml", "version: 2"),
        ]);
        backdate(&dir, "target/compiled/shop/models/marts/schema.yml/not_null_orders_id.sql", 120);
        let s = Subject { name: "not_null_orders_id", ..subject("models/marts/schema.yml") };
        let info = look_up(&dir, &dir.join("target"), Kind::Compiled, &s, 1 << 20);
        assert!(info.found);
        assert!(info.changed.contains(&"the schema file".to_string()), "{:?}", info.changed);
        assert!(!info.changed.contains(&"the model file".to_string()), "{:?}", info.changed);
        // Named once, not twice, when the node's file and its schema file are one.
        assert_eq!(info.changed.iter().filter(|c| *c == "the schema file").count(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A singular test needs none of this: it has a file of its own, so the
    /// derived path already found it and still must.
    #[test]
    fn a_singular_test_is_found_the_way_a_model_is() {
        let dir = project(&[("target/compiled/shop/tests/singular/revenue_reconciles.sql", "select 1")]);
        let s = subject("tests/singular/revenue_reconciles.sql");
        let info = look_up(&dir, &dir.join("target"), Kind::Compiled, &s, 1 << 20);
        assert!(info.found, "tried {:?}", info.candidates);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
