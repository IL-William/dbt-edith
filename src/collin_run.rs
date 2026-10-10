//! collin, the column lineage producer, found and run on request (0042), and
//! what its report says about the models it could not read.
//!
//! collin is an executable of its own, never a dependency: the SQL is parsed
//! outside this binary (0023), and someone who never looks at column lineage
//! never builds it. It is looked for beside dbt-edith, then on the PATH, and
//! installed only when the user asks, by `cargo install` typed into their own
//! terminal.
//!
//! Three rules hold here:
//!   - nothing runs at startup. A run follows picking Collin, clicking a column
//!     while its cache is older than the manifest, or the Regenerate button.
//!   - one run at a time. A request arriving during a run waits for that run
//!     rather than starting a second over the same files.
//!   - the cache is written under a temporary name and renamed, so the watcher
//!     never loads half a file, and a failed run leaves the old cache standing.

use std::collections::VecDeque;
use std::ffi::OsString;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// What the install entry types into the terminal, left for the user to run.
/// The repository is public, so cargo needs no account to clone it.
pub const INSTALL: &str = "cargo install --locked --git https://github.com/IL-William/dbt-collin collin-cli";

/// The same, forced: what the update entry types when the collin found is older
/// than `WANTS`. Without `--force`, cargo leaves a collin installed from
/// another source, a local path or a copied binary, where it is.
pub const UPDATE: &str = "cargo install --locked --force --git https://github.com/IL-William/dbt-collin collin-cli";

/// The oldest collin this dbt-edith accepts. Raised when dbt-edith relies on
/// something a later collin does. collin's numbers restarted at 0.1.0 with its
/// repository, and every collin since answers `--version`, as the old 0.2.0
/// did and nothing before it, so one that does not is older (0043, 0047).
pub const WANTS: (u32, u32, u32) = (0, 1, 0);
pub const WANTS_TEXT: &str = "0.1.0";

/// Where collin's coverage report goes. Not `column_lineage*.json`: discovery
/// would offer it as a cache.
pub const REPORT: &str = "collin.report.json";

const TAIL_LINES: usize = 20;

fn exe_name() -> &'static str {
    if cfg!(windows) {
        "collin.exe"
    } else {
        "collin"
    }
}

/// collin beside dbt-edith first, so a copy shipped with it wins over an older
/// one installed long ago, then the first on the PATH.
pub fn locate() -> Option<PathBuf> {
    let beside = std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf));
    locate_in(beside.as_deref(), std::env::var_os("PATH"))
}

fn locate_in(beside: Option<&Path>, path_var: Option<OsString>) -> Option<PathBuf> {
    let name = exe_name();
    let dirs = beside.map(Path::to_path_buf).into_iter().chain(path_var.iter().flat_map(std::env::split_paths));
    dirs.map(|d| d.join(name)).find(|p| p.is_file())
}

/// What `collin --version` says after `collin `, `0.2.0 (a142ee5)`, or None
/// when it says nothing of the kind: a collin from before `--version` answers with its
/// usage and a non-zero exit. Bounded, like a run: a binary that does not
/// answer must not hold up the menu.
pub fn version(bin: &Path) -> Option<String> {
    let mut child = Command::new(bin)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => break,
            Ok(Some(_)) => return None,
            Ok(None) if started.elapsed() >= Duration::from_secs(5) => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(_) => return None,
        }
    }
    let mut out = String::new();
    child.stdout.take()?.read_to_string(&mut out).ok()?;
    out.lines().next()?.trim().strip_prefix("collin ").map(str::to_string)
}

/// `0.2.0 (a142ee5)` to (0, 2, 0).
fn semver(version: &str) -> Option<(u32, u32, u32)> {
    let mut parts = version.split_whitespace().next()?.split('.').map(|p| p.parse::<u32>().ok());
    Some((parts.next()??, parts.next()??, parts.next()??))
}

/// Whether the collin found is older than this dbt-edith accepts. One that
/// gave no version is.
pub fn outdated(version: Option<&str>) -> bool {
    version.and_then(semver).map_or(true, |v| v < WANTS)
}

/// The version of the collin last asked, kept while the binary is the same
/// file: the menu asks on every opening, and an answer is a process start.
#[derive(Default)]
pub struct Versions(Mutex<Option<(PathBuf, Option<std::time::SystemTime>, Option<String>)>>);

impl Versions {
    pub fn get(&self, bin: &Path) -> Option<String> {
        let modified = mtime(bin);
        if let Ok(slot) = self.0.lock() {
            if let Some((path, m, v)) = slot.as_ref() {
                if path == bin && *m == modified {
                    return v.clone();
                }
            }
        }
        let v = version(bin);
        if let Ok(mut slot) = self.0.lock() {
            *slot = Some((bin.to_path_buf(), modified, v.clone()));
        }
        v
    }
}

fn mtime(path: &Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

/// Whether the cache no longer describes the manifest: missing, or written
/// before it. With no manifest there is nothing to run collin on.
pub fn stale(cache: &Path, manifest: &Path) -> bool {
    match (mtime(cache), mtime(manifest)) {
        (_, None) => false,
        (None, Some(_)) => true,
        (Some(c), Some(m)) => c < m,
    }
}

#[derive(serde::Serialize, Clone, Debug, Default, PartialEq)]
pub struct Status {
    /// idle, running, done or failed.
    pub state: &'static str,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub error: String,
    /// The last lines collin printed, for the tooltip only. Never logged: they
    /// name the project's models.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub log: Vec<String>,
}

/// The files one run reads and writes.
#[derive(Clone, Debug)]
pub struct Job {
    pub bin: PathBuf,
    pub project: PathBuf,
    pub manifest: PathBuf,
    pub catalog: PathBuf,
    pub out: PathBuf,
    pub report: PathBuf,
}

pub struct Runner {
    one: tokio::sync::Mutex<()>,
    /// Runs finished. A caller that waited out someone else's run sees it move
    /// and takes that run's outcome instead of starting another.
    runs: AtomicU64,
    status: Mutex<Status>,
    deadline: Duration,
}

impl Runner {
    pub fn new(deadline: Duration) -> Runner {
        Runner {
            one: tokio::sync::Mutex::new(()),
            runs: AtomicU64::new(0),
            status: Mutex::new(Status { state: "idle", ..Default::default() }),
            deadline,
        }
    }

    pub fn status(&self) -> Status {
        self.status.lock().map(|s| s.clone()).unwrap_or_default()
    }

    fn set(&self, status: Status) {
        if let Ok(mut s) = self.status.lock() {
            *s = status;
        }
    }

    /// Said before a run is spawned in the background, so the answer that
    /// spawned it already shows it running.
    pub fn mark_running(&self) {
        self.set(Status { state: "running", ..Default::default() });
    }

    /// Runs collin, or waits for the run already going and returns its outcome.
    ///
    /// `load` puts the new cache on screen, and runs before the run reads as
    /// done: a page polling for the end of a run must find the edges there
    /// once it sees "done", or it stops asking with nothing drawn.
    pub async fn generate<F, Fut>(&self, job: Job, load: F) -> Status
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = ()>,
    {
        let before = self.runs.load(Ordering::SeqCst);
        let _one = self.one.lock().await;
        if self.runs.load(Ordering::SeqCst) != before {
            return self.status();
        }
        self.mark_running();
        let deadline = self.deadline;
        let outcome = tokio::task::spawn_blocking(move || run(&job, deadline))
            .await
            .unwrap_or_else(|e| Err((format!("collin did not finish: {e}"), Vec::new())));
        let status = match outcome {
            Ok(log) => {
                load().await;
                Status { state: "done", error: String::new(), log }
            }
            Err((error, log)) => Status { state: "failed", error, log },
        };
        self.set(status.clone());
        self.runs.fetch_add(1, Ordering::SeqCst);
        status
    }
}

/// What collin's report says about the run as a whole, cut down to what the
/// page shows: how many models it read the SQL of, and why the others failed.
#[derive(serde::Serialize, Clone, Debug, Default, PartialEq)]
pub struct Summary {
    pub models: usize,
    pub parsed: usize,
    /// Models whose SQL came from a file under `target/compiled/`, the
    /// manifest having none, since collin 0.1.0. Such a file can be older than
    /// the manifest.
    #[serde(skip_serializing_if = "is_zero")]
    pub from_files: usize,
    /// Of those, the files collin set aside as compiled against another graph.
    /// They count as parsed, though none of their edges was read from the SQL.
    #[serde(skip_serializing_if = "is_zero")]
    pub set_aside: usize,
    /// The commonest reasons a model's SQL was not read, most models first.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub reasons: Vec<Reason>,
}

#[derive(serde::Serialize, Clone, Debug, PartialEq)]
pub struct Reason {
    pub why: String,
    pub models: usize,
}

/// One model the report lists. collin lists a model only when something about
/// it is a fault, so a model absent here has nothing to say.
#[derive(serde::Serialize, Clone, Debug, Default, PartialEq)]
pub struct ModelNote {
    /// parsed, per_column, inferred or unresolved.
    pub provenance: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub parse_error: String,
    #[serde(skip_serializing_if = "is_zero")]
    pub issues: usize,
    /// The compiled file the SQL was read from, under the target directory,
    /// when the manifest had none.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub sql_file: String,
    /// That file reads a table the manifest does not give the model, so
    /// collin matched its edges by name instead.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub set_aside: bool,
    /// The columns collin says something about, one entry per thing said.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub columns: Vec<ColumnNote>,
}

/// One thing collin's report says about one column of a model. `kind` names
/// the list it came from; `detail` and `names` carry what that list gives,
/// and the page words it.
#[derive(serde::Serialize, Clone, Debug, Default, PartialEq)]
pub struct ColumnNote {
    /// Lower case, as collin writes it.
    pub column: String,
    /// `lost`, `missing`, `unexpected`, `unbacked`, `read_downstream` or
    /// `ambiguous`.
    pub kind: &'static str,
    /// Where a lost column's walk stopped, the CTE an unbacked read names, the
    /// list a downstream read was missing from, or how an ambiguous name was
    /// settled.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub detail: String,
    /// The CTE or the names a lost column ends at, the relations that do have
    /// an unbacked name, the models reading a column, or the parents an
    /// ambiguous one was taken from; at most `NAMES` of them.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub names: Vec<String>,
    /// How many `names` there were before the cut, when there were more.
    #[serde(skip_serializing_if = "is_zero")]
    pub of: usize,
}

/// A column read by two hundred models says so in a line, not a list.
const NAMES: usize = 5;

/// The per-column lists of one model's entry. Read field by field out of the
/// raw JSON rather than through typed structs: these are collin's diagnostics,
/// and a later collin reshaping one must cost that list, not the whole report.
fn column_notes(m: &serde_json::Value) -> Vec<ColumnNote> {
    use serde_json::Value;
    let list = |key: &str| m.get(key).and_then(Value::as_array).map(Vec::as_slice).unwrap_or_default();
    let text = |v: &Value, key: &str| v.get(key).and_then(Value::as_str).unwrap_or_default().to_string();
    let texts = |v: Option<&Value>| -> Vec<String> {
        v.and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).map(str::to_string).collect()).unwrap_or_default()
    };
    let note = |column: String, kind: &'static str, detail: String, mut names: Vec<String>| {
        let of = if names.len() > NAMES { names.len() } else { 0 };
        names.truncate(NAMES);
        ColumnNote { column: column.to_lowercase(), kind, detail, names, of }
    };
    let mut out = Vec::new();
    for c in list("lost_columns") {
        let column = text(c, "column");
        let ends = c.get("ends").and_then(Value::as_array).map(Vec::as_slice).unwrap_or_default();
        if ends.is_empty() {
            out.push(note(column.clone(), "lost", String::new(), Vec::new()));
        }
        for end in ends {
            let reason = text(end, "reason");
            let names = match end.get("via").and_then(Value::as_str) {
                Some(via) => vec![via.to_string()],
                None => texts(end.get("names")),
            };
            out.push(note(column.clone(), "lost", reason, names));
        }
    }
    for (key, kind) in [("missing_columns", "missing"), ("unexpected_columns", "unexpected")] {
        for c in list(key).iter().filter_map(Value::as_str) {
            out.push(note(c.to_string(), kind, String::new(), Vec::new()));
        }
    }
    for c in list("unbacked") {
        out.push(note(text(c, "column"), "unbacked", text(c, "cte"), texts(c.get("owners"))));
    }
    for c in list("read_downstream_but_absent") {
        out.push(note(text(c, "column"), "read_downstream", text(c, "against"), texts(c.get("read_by"))));
    }
    for c in list("ambiguous_inferred") {
        out.push(note(text(c, "column"), "ambiguous", text(c, "by"), texts(c.get("chosen"))));
    }
    out.retain(|n| !n.column.is_empty());
    out
}

fn is_zero(n: &usize) -> bool {
    *n == 0
}

#[derive(Default)]
pub struct Report {
    pub summary: Summary,
    pub notes: std::collections::HashMap<String, ModelNote>,
}

/// The report as collin writes it, every field optional: it is collin's file,
/// not a contract like the cache's version 1, and what it adds is ignored.
#[derive(serde::Deserialize, Default)]
struct RawReport {
    #[serde(default)]
    totals: RawTotals,
    #[serde(default)]
    models: Vec<RawModel>,
}

#[derive(serde::Deserialize, Default)]
struct RawTotals {
    #[serde(default)]
    models: usize,
    #[serde(default)]
    parsed: usize,
    #[serde(default)]
    sql_from_files: usize,
    #[serde(default)]
    sql_from_files_set_aside: usize,
}

#[derive(serde::Deserialize)]
struct RawModel {
    #[serde(default)]
    unique_id: String,
    #[serde(default)]
    provenance: String,
    #[serde(default)]
    parse_error: Option<String>,
    #[serde(default)]
    issues: Vec<serde_json::Value>,
    #[serde(default)]
    sql_file: Option<String>,
    #[serde(default)]
    sql_file_set_aside: bool,
    /// Everything else in the entry, for `column_notes`.
    #[serde(flatten)]
    rest: serde_json::Value,
}

const REASONS: usize = 3;

pub fn read_report(path: &Path) -> Option<Report> {
    let raw: RawReport = serde_json::from_slice(&std::fs::read(path).ok()?).ok()?;
    let mut counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for m in &raw.models {
        if let Some(why) = m.parse_error.as_deref().filter(|w| !w.is_empty()) {
            *counts.entry(why).or_default() += 1;
        }
    }
    let mut reasons: Vec<Reason> = counts.into_iter().map(|(why, models)| Reason { why: why.to_string(), models }).collect();
    reasons.sort_by(|a, b| b.models.cmp(&a.models).then_with(|| a.why.cmp(&b.why)));
    reasons.truncate(REASONS);
    let t = &raw.totals;
    let summary = Summary {
        models: t.models,
        parsed: t.parsed,
        from_files: t.sql_from_files,
        set_aside: t.sql_from_files_set_aside,
        reasons,
    };
    let notes = raw
        .models
        .into_iter()
        .filter(|m| !m.unique_id.is_empty())
        .map(|m| {
            let note = ModelNote {
                provenance: m.provenance,
                parse_error: m.parse_error.unwrap_or_default(),
                issues: m.issues.len(),
                sql_file: m.sql_file.unwrap_or_default(),
                set_aside: m.sql_file_set_aside,
                columns: column_notes(&m.rest),
            };
            (m.unique_id, note)
        })
        .collect();
    Some(Report { summary, notes })
}

/// The last report read, kept until the file changes: it runs to megabytes on
/// a large project, and the page asks for it on every menu opening and every
/// model shown.
#[derive(Default)]
pub struct Reports(Mutex<Option<(u64, Option<std::time::SystemTime>, Arc<Report>)>>);

impl Reports {
    pub fn get(&self, path: &Path) -> Option<Arc<Report>> {
        let meta = std::fs::metadata(path).ok()?;
        let (len, modified) = (meta.len(), meta.modified().ok());
        let mut slot = self.0.lock().ok()?;
        if let Some((l, m, report)) = slot.as_ref() {
            if *l == len && *m == modified {
                return Some(report.clone());
            }
        }
        let report = Arc::new(read_report(path)?);
        *slot = Some((len, modified, report.clone()));
        Some(report)
    }
}

fn keep_tail<R: Read + Send + 'static>(from: R, tail: Arc<Mutex<VecDeque<String>>>) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        for line in BufReader::new(from).lines().map_while(Result::ok) {
            let line = line.trim_end().to_string();
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(mut t) = tail.lock() {
                if t.len() == TAIL_LINES {
                    t.pop_front();
                }
                t.push_back(line);
            }
        }
    })
}

/// collin's own complaint, `collin: <why>`, else its last line.
fn explain(log: &[String]) -> Option<String> {
    log.iter()
        .rev()
        .find_map(|l| l.strip_prefix("collin: ").map(str::to_string))
        .or_else(|| log.last().cloned())
}

fn run(job: &Job, deadline: Duration) -> Result<Vec<String>, (String, Vec<String>)> {
    let name = job.out.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let tmp = job.out.with_file_name(format!("{name}.tmp"));
    let _ = std::fs::remove_file(&tmp);
    let mut child = Command::new(&job.bin)
        .arg("generate")
        .arg("--project")
        .arg(&job.project)
        .arg("--manifest")
        .arg(&job.manifest)
        .arg("--catalog")
        .arg(&job.catalog)
        .arg("--out")
        .arg(&tmp)
        .arg("--report")
        .arg(&job.report)
        .current_dir(&job.project)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| (format!("cannot run {}: {e}", job.bin.display()), Vec::new()))?;
    let tail = Arc::new(Mutex::new(VecDeque::new()));
    let readers: Vec<_> = [
        child.stdout.take().map(|o| keep_tail(o, tail.clone())),
        child.stderr.take().map(|e| keep_tail(e, tail.clone())),
    ]
    .into_iter()
    .flatten()
    .collect();

    let started = Instant::now();
    let exit = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None) if started.elapsed() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                break Err(format!("collin took longer than {} s and was stopped", deadline.as_secs()));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(e) => break Err(format!("lost track of collin: {e}")),
        }
    };
    // A grandchild holding a pipe open must not hang the server: once collin
    // itself is gone, its last lines are whatever arrives within a moment.
    let grace = Instant::now();
    for r in readers {
        while !r.is_finished() && grace.elapsed() < Duration::from_millis(300) {
            std::thread::sleep(Duration::from_millis(10));
        }
        if r.is_finished() {
            let _ = r.join();
        }
    }
    let log: Vec<String> = tail.lock().map(|t| t.iter().cloned().collect()).unwrap_or_default();
    let fail = |error: String| {
        let _ = std::fs::remove_file(&tmp);
        Err((error, log.clone()))
    };
    match exit {
        Err(e) => fail(e),
        Ok(status) if !status.success() => fail(explain(&log).unwrap_or_else(|| format!("collin exited with {status}"))),
        Ok(_) if !tmp.is_file() => fail(format!("collin finished without writing {}", tmp.display())),
        Ok(_) => match std::fs::rename(&tmp, &job.out) {
            Ok(()) => Ok(log),
            Err(e) => fail(format!("cannot move collin's cache onto {}: {e}", job.out.display())),
        },
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("dbt-edith-collin-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// Writes an executable script at `path` without this process ever holding
    /// it open for writing. Written in place, a test running beside this one
    /// can fork while the file is open: the child holds the write descriptor
    /// until it execs, and running the script in that moment fails on Linux
    /// with "Text file busy". `cp` writes it from a process of its own.
    fn executable(path: &Path, script: &str) {
        let draft = path.with_extension("draft");
        std::fs::write(&draft, script).unwrap();
        let _ = std::fs::remove_file(path);
        assert!(Command::new("cp").arg(&draft).arg(path).status().unwrap().success());
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    /// A stand-in for collin: a shell script that finds `--out` among its
    /// arguments and runs `body` with it in `$out`.
    fn fake(dir: &Path, body: &str) -> PathBuf {
        let bin = dir.join("collin");
        let script = format!(
            "#!/bin/sh\nout=''\nwhile [ $# -gt 0 ]; do [ \"$1\" = --out ] && out=\"$2\"; shift; done\n\
             echo run >> \"$(dirname \"$0\")/runs\"\n{body}\n"
        );
        executable(&bin, &script);
        bin
    }

    fn job(dir: &Path, bin: PathBuf) -> Job {
        Job {
            bin,
            project: dir.to_path_buf(),
            manifest: dir.join("manifest.json"),
            catalog: dir.join("catalog.json"),
            out: dir.join("column_lineage.collin.json"),
            report: dir.join(REPORT),
        }
    }

    fn runs(dir: &Path) -> usize {
        std::fs::read_to_string(dir.join("runs")).map_or(0, |s| s.lines().count())
    }

    #[test]
    fn the_copy_beside_the_binary_wins_over_the_path() {
        let d = dir("locate");
        let (beside, on_path) = (d.join("beside"), d.join("bin"));
        std::fs::create_dir_all(&beside).unwrap();
        std::fs::create_dir_all(&on_path).unwrap();
        std::fs::write(on_path.join(exe_name()), "").unwrap();
        let path_var = std::env::join_paths([d.join("missing"), on_path.clone()]).unwrap();
        assert_eq!(locate_in(Some(&beside), Some(path_var.clone())), Some(on_path.join(exe_name())));
        std::fs::write(beside.join(exe_name()), "").unwrap();
        assert_eq!(locate_in(Some(&beside), Some(path_var)), Some(beside.join(exe_name())));
        assert_eq!(locate_in(None, None), None);
    }

    #[test]
    fn a_cache_is_stale_when_missing_or_older_than_the_manifest() {
        let d = dir("stale");
        let (cache, manifest) = (d.join("cache.json"), d.join("manifest.json"));
        assert!(!stale(&cache, &manifest), "no manifest, nothing to run on");
        std::fs::write(&manifest, "{}").unwrap();
        assert!(stale(&cache, &manifest));
        std::fs::write(&cache, "{}").unwrap();
        let old = std::time::SystemTime::now() - Duration::from_secs(60);
        std::fs::File::options().write(true).open(&cache).unwrap().set_modified(old).unwrap();
        assert!(stale(&cache, &manifest));
        std::fs::File::options().write(true).open(&manifest).unwrap().set_modified(old - Duration::from_secs(60)).unwrap();
        assert!(!stale(&cache, &manifest));
    }

    #[tokio::test]
    async fn a_run_renames_its_cache_into_place_and_keeps_what_it_printed() {
        let d = dir("ok");
        let bin = fake(&d, "echo '  12 models in 40 ms'\necho '{\"version\":1,\"source\":\"collin\",\"edges\":[]}' > \"$out\"");
        let runner = Runner::new(Duration::from_secs(10));
        let status = runner.generate(job(&d, bin), || async {}).await;
        assert_eq!(status.state, "done", "{status:?}");
        assert_eq!(status.log, vec!["  12 models in 40 ms".to_string()]);
        assert!(d.join("column_lineage.collin.json").is_file());
        assert!(!d.join("column_lineage.collin.json.tmp").exists());
    }

    #[tokio::test]
    async fn a_failed_run_says_why_and_leaves_the_old_cache_alone() {
        let d = dir("fail");
        std::fs::write(d.join("column_lineage.collin.json"), "old").unwrap();
        let bin = fake(&d, "echo half > \"$out\"\necho 'collin: no manifest at target/manifest.json' >&2\nexit 1");
        let status = Runner::new(Duration::from_secs(10)).generate(job(&d, bin), || async {}).await;
        assert_eq!((status.state, status.error.as_str()), ("failed", "no manifest at target/manifest.json"));
        assert_eq!(std::fs::read_to_string(d.join("column_lineage.collin.json")).unwrap(), "old");
        assert!(!d.join("column_lineage.collin.json.tmp").exists());

        let silent = fake(&d, "exit 0");
        let status = Runner::new(Duration::from_secs(10)).generate(job(&d, silent), || async {}).await;
        assert!(status.error.starts_with("collin finished without writing"), "{status:?}");
    }

    #[tokio::test]
    async fn a_run_past_its_deadline_is_stopped() {
        let d = dir("slow");
        let bin = fake(&d, "exec sleep 30");
        let started = Instant::now();
        let status = Runner::new(Duration::from_millis(300)).generate(job(&d, bin), || async {}).await;
        assert_eq!(status.state, "failed");
        assert!(status.error.contains("was stopped"), "{status:?}");
        assert!(started.elapsed() < Duration::from_secs(10));
    }

    #[tokio::test]
    async fn two_requests_during_one_run_make_one_run() {
        let d = dir("once");
        let bin = fake(&d, "sleep 0.3\necho '{}' > \"$out\"");
        let runner = Arc::new(Runner::new(Duration::from_secs(10)));
        let (a, b) = (runner.clone(), runner.clone());
        let (ja, jb) = (job(&d, bin.clone()), job(&d, bin));
        let loads = Arc::new(AtomicU64::new(0));
        let (la, lb) = (loads.clone(), loads.clone());
        let first = tokio::spawn(async move {
            a.generate(ja, || async move {
                la.fetch_add(1, Ordering::SeqCst);
            })
            .await
        });
        tokio::time::sleep(Duration::from_millis(50)).await;
        let second = tokio::spawn(async move {
            b.generate(jb, || async move {
                lb.fetch_add(1, Ordering::SeqCst);
            })
            .await
        });
        assert_eq!(first.await.unwrap().state, "done");
        assert_eq!(second.await.unwrap().state, "done", "the second took the first's outcome");
        assert_eq!(runs(&d), 1);
        assert_eq!(loads.load(Ordering::SeqCst), 1, "only the run loads its cache");
    }

    #[tokio::test]
    async fn a_run_reads_as_running_until_its_cache_is_loaded() {
        let d = dir("load");
        let bin = fake(&d, "echo '{}' > \"$out\"");
        let runner = Arc::new(Runner::new(Duration::from_secs(10)));
        let seen = Arc::new(Mutex::new(""));
        let (r, s) = (runner.clone(), seen.clone());
        let status = runner
            .generate(job(&d, bin), || async move {
                *s.lock().unwrap() = r.status().state;
            })
            .await;
        assert_eq!(*seen.lock().unwrap(), "running");
        assert_eq!(status.state, "done");
        assert_eq!(runner.status().state, "done");
    }

    #[test]
    fn a_report_says_how_many_models_were_read_and_why_the_others_were_not() {
        let d = dir("report");
        let path = d.join(REPORT);
        std::fs::write(
            &path,
            r#"{"generated_at":"x","totals":{"models":5,"parsed":1,"edges_inferred":9},"models":[
              {"name":"a","unique_id":"model.shop.a","provenance":"inferred","agreement":"unchecked","parse_error":"no compiled_code and no compiled file"},
              {"name":"b","unique_id":"model.shop.b","provenance":"inferred","parse_error":"no compiled_code and no compiled file"},
              {"name":"c","unique_id":"model.shop.c","provenance":"inferred","parse_error":"sql parser error: Expected ), found: EOF"},
              {"name":"d","unique_id":"model.shop.d","provenance":"parsed","issues":[{"code":"x"},{"code":"y"}],"a_field_from_a_later_collin":true}
            ]}"#,
        )
        .unwrap();
        let report = read_report(&path).unwrap();
        assert_eq!((report.summary.models, report.summary.parsed), (5, 1));
        assert_eq!(
            report.summary.reasons,
            vec![
                Reason { why: "no compiled_code and no compiled file".into(), models: 2 },
                Reason { why: "sql parser error: Expected ), found: EOF".into(), models: 1 },
            ]
        );
        assert_eq!(report.notes["model.shop.a"].parse_error, "no compiled_code and no compiled file");
        assert_eq!(report.notes["model.shop.d"], ModelNote { provenance: "parsed".into(), issues: 2, ..Default::default() });
        assert!(!report.notes.contains_key("model.shop.e"), "a model with nothing to say is not listed");
        assert!(read_report(&d.join("missing.json")).is_none());
        std::fs::write(d.join("broken.json"), "{").unwrap();
        assert!(read_report(&d.join("broken.json")).is_none());
    }

    #[test]
    fn a_report_says_which_models_were_read_from_a_compiled_file() {
        let d = dir("files");
        let path = d.join(REPORT);
        std::fs::write(
            &path,
            r#"{"compiled_files":"/p/target/compiled","totals":{"models":4,"parsed":4,"without_compiled_code":4,
              "sql_from_files":3,"sql_from_files_set_aside":1},"models":[
              {"unique_id":"model.shop.a","provenance":"inferred","sql_file":"compiled/shop/models/a.sql","sql_file_set_aside":true,
               "undeclared_relations":["DEV.OTHER.ORDERS"]},
              {"unique_id":"model.shop.b","provenance":"per_column","sql_file":"compiled/shop/models/b.sql"}
            ]}"#,
        )
        .unwrap();
        let report = read_report(&path).unwrap();
        assert_eq!((report.summary.from_files, report.summary.set_aside), (3, 1));
        let a = &report.notes["model.shop.a"];
        assert_eq!((a.sql_file.as_str(), a.set_aside), ("compiled/shop/models/a.sql", true));
        assert!(!report.notes["model.shop.b"].set_aside);
        let sent = serde_json::to_string(&report.summary).unwrap();
        assert!(sent.contains(r#""from_files":3,"set_aside":1"#), "{sent}");

        // A collin that does not read compiled files writes none of it, and the page gets none of it.
        std::fs::write(&path, r#"{"totals":{"models":2,"parsed":2},"models":[{"unique_id":"model.shop.a","provenance":"parsed"}]}"#).unwrap();
        let old = read_report(&path).unwrap();
        let sent = serde_json::to_string(&old.summary).unwrap() + &serde_json::to_string(&old.notes["model.shop.a"]).unwrap();
        assert!(!sent.contains("from_files") && !sent.contains("set_aside") && !sent.contains("sql_file"), "{sent}");
    }

    #[test]
    fn a_report_says_what_it_found_about_each_column() {
        let d = dir("columns");
        let path = d.join(REPORT);
        std::fs::write(
            &path,
            r#"{"totals":{"models":2,"parsed":2},"models":[
              {"unique_id":"model.shop.a","provenance":"parsed",
               "lost_columns":[{"column":"Total","ends":[{"reason":"phantom","via":"base","relations":[]},
                                                       {"reason":"names_unreached","names":["x","y"]}]}],
               "missing_columns":["old_flag"],"unexpected_columns":["new_flag"],
               "unbacked":[{"cte":"final","column":"amount","owners":["stg_payments"]}],
               "read_downstream_but_absent":[{"column":"status","against":"warehouse","in_compile":true,
                 "read_by":["model.shop.b","model.shop.c","model.shop.d","model.shop.e","model.shop.f","model.shop.g"]}],
               "ambiguous_inferred":[{"column":"id","candidates":["model.shop.p","model.shop.q"],"chosen":["model.shop.p","model.shop.q"],"by":"every"}]},
              {"unique_id":"model.shop.b","provenance":"parsed","missing_columns":"not a list","unbacked":[{"cte":"c"}],
               "lost_columns":[{"column":"z"}]}
            ]}"#,
        )
        .unwrap();
        let report = read_report(&path).unwrap();
        let a = &report.notes["model.shop.a"].columns;
        let said: Vec<(&str, &str, &str, String)> =
            a.iter().map(|n| (n.column.as_str(), n.kind, n.detail.as_str(), n.names.join(","))).collect();
        assert_eq!(
            said,
            vec![
                ("total", "lost", "phantom", "base".to_string()),
                ("total", "lost", "names_unreached", "x,y".to_string()),
                ("old_flag", "missing", "", String::new()),
                ("new_flag", "unexpected", "", String::new()),
                ("amount", "unbacked", "final", "stg_payments".to_string()),
                ("status", "read_downstream", "warehouse", "model.shop.b,model.shop.c,model.shop.d,model.shop.e,model.shop.f".to_string()),
                ("id", "ambiguous", "every", "model.shop.p,model.shop.q".to_string()),
            ]
        );
        assert_eq!(a[5].of, 6, "a list cut short says how long it was");
        assert_eq!(a[0].of, 0);
        // A list reshaped by a later collin costs that list, not the report.
        let b = &report.notes["model.shop.b"].columns;
        assert_eq!(b.iter().map(|n| (n.column.as_str(), n.kind)).collect::<Vec<_>>(), vec![("z", "lost")]);
        assert_eq!(report.notes["model.shop.b"].provenance, "parsed");
    }

    #[test]
    fn a_report_is_read_again_only_when_it_changes() {
        let d = dir("reports");
        let path = d.join(REPORT);
        std::fs::write(&path, r#"{"totals":{"models":2,"parsed":2},"models":[]}"#).unwrap();
        let reports = Reports::default();
        let first = reports.get(&path).unwrap();
        assert!(Arc::ptr_eq(&first, &reports.get(&path).unwrap()));
        std::fs::write(&path, r#"{"totals":{"models":3,"parsed":0},"models":[]}"#).unwrap();
        assert_eq!(reports.get(&path).unwrap().summary.models, 3);
        std::fs::remove_file(&path).unwrap();
        assert!(reports.get(&path).is_none());
    }

    #[test]
    fn a_version_is_read_from_collin_and_compared_with_the_one_wanted() {
        let d = dir("version");
        let new = fake(&d, "echo 'collin 0.1.0 (de9f2cd)'");
        assert_eq!(version(&new).as_deref(), Some("0.1.0 (de9f2cd)"));
        assert!(!outdated(version(&new).as_deref()));
        assert!(!outdated(Some("0.2.0 (a142ee5)")), "the 0.2.0 of collin's old repository");
        let old = d.join("old");
        executable(&old, "#!/bin/sh\necho 'unknown command \"--version\"' >&2\nexit 2\n");
        assert_eq!(version(&old), None, "a collin from before --version has none");
        assert!(outdated(None));
        assert!(outdated(Some("0.0.9")));
        assert!(!outdated(Some("0.10.0 (abc)")), "compared as numbers, not text");
        assert!(!outdated(Some("1.0.0")));
        assert!(outdated(Some("garbage")));
    }

    #[test]
    fn a_version_is_asked_again_only_when_the_binary_changes() {
        let d = dir("versions");
        let bin = fake(&d, "echo 'collin 0.2.0'");
        let versions = Versions::default();
        assert_eq!(versions.get(&bin).as_deref(), Some("0.2.0"));
        assert_eq!(versions.get(&bin).as_deref(), Some("0.2.0"));
        assert_eq!(runs(&d), 1, "the second answer came from memory");
        executable(&bin, "#!/bin/sh\necho 'collin 0.3.0'\n");
        // Read only: a write descriptor on a script about to run is the very
        // thing `executable` avoids. The owner may set the time through either.
        let later = std::time::SystemTime::now() + Duration::from_secs(5);
        std::fs::File::open(&bin).unwrap().set_modified(later).unwrap();
        assert_eq!(versions.get(&bin).as_deref(), Some("0.3.0"));
    }

    #[tokio::test]
    async fn a_failed_run_loads_nothing() {
        let d = dir("noload");
        let bin = fake(&d, "exit 3");
        let loaded = Arc::new(AtomicU64::new(0));
        let l = loaded.clone();
        let status = Runner::new(Duration::from_secs(10))
            .generate(job(&d, bin), || async move {
                l.fetch_add(1, Ordering::SeqCst);
            })
            .await;
        assert_eq!(status.state, "failed");
        assert_eq!(loaded.load(Ordering::SeqCst), 0);
    }
}
