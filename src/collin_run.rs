//! collin, the column lineage producer, found and run on request (0042), and
//! what its report says about the models it could not read.
//!
//! collin is an executable of its own, never a dependency: the SQL is parsed
//! outside this binary (0023), and someone who never looks at column lineage
//! never builds it. It is looked for beside dbt-edith, then on the PATH, and
//! installed only when the user asks, by `cargo install` typed into their own
//! terminal. Other versions can sit beside that one, each installed at a tag
//! into its own folder under dbt-edith's config directory, and the one picked
//! for the project is the one that runs (0060).
//!
//! Four rules hold here:
//!   - nothing runs at startup. A run follows picking Collin, clicking a column
//!     while its cache is older than the manifest, the Regenerate button, or
//!     `--column` asked with `--run-collin` on the command line (0099).
//!   - a collin git tracks never runs, beside dbt-edith or on the PATH: it came
//!     with a repository, not from the user (0094).
//!   - one run at a time in the page. A request arriving during a run waits
//!     for that run rather than starting a second over the same files. The
//!     command line's run is another process, which the page cannot wait for.
//!   - the cache is written under a temporary name of this process's own and
//!     renamed, so the watcher never loads half a file, a run from another
//!     process never writes over this one's, and a failed run leaves the old
//!     cache standing.

use std::collections::VecDeque;
use std::ffi::OsString;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// What the install entry types into the terminal, left for the user to run.
/// The repository is public, so cargo needs no account to clone it.
pub const INSTALL: &str = "cargo install --locked --git https://github.com/IL-William/dbt-collin collin-cli";

/// collin's repository, public, so cargo needs no account to clone it.
const REPO: &str = "https://github.com/IL-William/dbt-collin";

/// The same, forced: what the update entry types when the collin found is older
/// than `WANTS`. Without `--force`, cargo leaves a collin installed from
/// another source, a local path or a copied binary, where it is.
pub const UPDATE: &str = "cargo install --locked --force --git https://github.com/IL-William/dbt-collin collin-cli";

/// The oldest collin this dbt-edith accepts. Raised when dbt-edith relies on
/// something a later collin does: 0.8.0 is the first to list its roots, the
/// columns a literal sets, which the `generated` badge is read from (0096).
/// 0.10.0 reads a seed's columns from its CSV header, in the project it is
/// given, which dbt-edith names only for a manifest lying in it (0109).
/// collin's numbers restarted at 0.1.0 with its repository, so the old
/// repository's 0.2.0, which writes no roots, is older again (0047). Every
/// collin since answers `--version`, as that 0.2.0 did and nothing before it,
/// so one that does not is older too (0043, 0098).
pub const WANTS: (u32, u32, u32) = (0, 10, 0);
pub const WANTS_TEXT: &str = "0.10.0";

/// Where collin's coverage report goes. Not `column_lineage*.json`: discovery
/// would offer it as a cache.
pub const REPORT: &str = "collin.report.json";

const TAIL_LINES: usize = 20;

/// How long a run may take, from the page or the command line, before it is
/// stopped: ten minutes, far past what a run takes.
pub const DEADLINE: Duration = Duration::from_secs(600);

/// The first collin that reads `--compiled` (0099). Asked of the collin found
/// only when the folder is named, so a user who never names one is never sent
/// to it.
pub const COMPILED_SINCE: (u32, u32, u32) = (0, 9, 0);
pub const COMPILED_SINCE_TEXT: &str = "0.9.0";

fn exe_name() -> &'static str {
    if cfg!(windows) {
        "collin.exe"
    } else {
        "collin"
    }
}

/// Why a collin found was passed over.
#[derive(Clone, Debug, PartialEq)]
pub enum Refusal {
    /// git tracks it: it came with a repository.
    Tracked,
    /// It sits in a repository and git could not say, for the reason given:
    /// a repository git refuses, another account's by `safe.directory`, its
    /// deadline, or no git on the PATH. Not knowing is not taken for the
    /// user's own (0094).
    Unsure(String),
}

/// What the search for collin found: the one that runs, and the first one it
/// passed over and why, so the menu can say why that one did not run (0094).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Located {
    pub bin: Option<PathBuf>,
    pub refused: Option<(PathBuf, Refusal)>,
}

/// collin beside dbt-edith first, so a copy shipped with it wins over an older
/// one installed long ago, then the first on the PATH. A collin git tracks is
/// passed over wherever it is: a dbt-edith copied into a project would
/// otherwise run the one the repository commits beside it on the first pick,
/// where the user's own copy, untracked, still runs (0094).
pub fn locate() -> Located {
    let beside = std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf));
    locate_in(beside.as_deref(), std::env::var_os("PATH"), refusal)
}

fn locate_in(beside: Option<&Path>, path_var: Option<OsString>, refusal: impl Fn(&Path) -> Option<Refusal>) -> Located {
    let name = exe_name();
    // A relative entry of the PATH is skipped, as `program::on_path` skips
    // it: the current folder would decide what it names, and the `.git` above
    // it would be out of sight of `in_repository`.
    let on_path = path_var.iter().flat_map(std::env::split_paths).filter(|d| d.is_absolute());
    let dirs = beside.map(Path::to_path_buf).into_iter().chain(on_path);
    let mut found = Located::default();
    for bin in dirs.map(|d| d.join(name)).filter(|p| p.is_file()) {
        match refusal(&bin) {
            None => {
                found.bin = Some(bin);
                break;
            }
            Some(why) => {
                found.refused.get_or_insert((bin, why));
            }
        }
    }
    found
}

/// A file's size and modification time, which change when it is written.
type Stamp = Option<(u64, Option<std::time::SystemTime>)>;

fn stamp(path: &Path) -> Stamp {
    std::fs::metadata(path).ok().map(|m| (m.len(), m.modified().ok()))
}

/// What was said of one collin, and what it was said against.
#[derive(Clone)]
struct Seen {
    file: Stamp,
    /// The index that answered and its stamp then; None outside a repository.
    index: Option<(PathBuf, Stamp)>,
    /// Set when git gave no answer: that is kept a few seconds only, so the
    /// menu does not start git on every opening, nor keep a refusal git may
    /// lift.
    until: Option<Instant>,
    refusal: Option<Refusal>,
}

/// How long an answer git could not give is kept: a few seconds, so a
/// refusal git lifts, `safe.directory` set, is seen soon.
const UNSURE_FOR: Duration = Duration::from_secs(5);

/// The same after git ran out of time, which cost its whole deadline: on a
/// share where git hangs, asking again every few seconds would hold every
/// opening of the menu that long.
const TIMED_OUT_FOR: Duration = Duration::from_secs(60);

/// Until when an answer git could not give is kept, by why it failed.
fn unsure_until(why: &str) -> Instant {
    Instant::now() + if why.contains("took longer than") { TIMED_OUT_FOR } else { UNSURE_FOR }
}

/// Whether a folder sits in what looks like a repository: a `.git` in it or
/// above it, which is what git itself looks for.
fn in_repository(dir: &Path) -> bool {
    dir.ancestors().any(|d| d.join(".git").exists())
}

/// Why this collin must not run, or None when it is the user's own. Asked of
/// git in the file's own folder (`git::tracks`): one that arrived with a
/// clone was put there by whoever wrote the repository, as 0053 says of a
/// venv. Outside a repository nobody is asked for more. Inside one, an answer
/// git could not give refuses it, the failure named, rather than run what may
/// be the repository's. The menu asks on every opening, so an answer is kept
/// while neither the file nor the index that gave it has changed (`git add`
/// and `git rm --cached` rewrite the index), and a failure for a few seconds.
fn refusal(bin: &Path) -> Option<Refusal> {
    static SEEN: std::sync::OnceLock<Mutex<std::collections::HashMap<PathBuf, Seen>>> = std::sync::OnceLock::new();
    let seen = SEEN.get_or_init(Default::default);
    let file = stamp(bin);
    let known = seen.lock().ok().and_then(|s| s.get(bin).cloned());
    if let Some(k) = known {
        let fresh = match (&k.until, &k.index) {
            (Some(until), _) => Instant::now() < *until,
            (None, Some((path, then))) => stamp(path) == *then,
            (None, None) => true,
        };
        if k.file == file && fresh {
            return k.refusal;
        }
    }
    let (Some(dir), Some(name)) = (bin.parent(), bin.file_name().and_then(|n| n.to_str())) else { return None };
    let repository = in_repository(dir);
    let (refusal, index, until) = match crate::git::tracks(dir, name) {
        Ok(tracked) if !repository => (tracked.then_some(Refusal::Tracked), None, None),
        Ok(tracked) => match crate::git::index_of(dir) {
            Some(index) => {
                let then = stamp(&index);
                (tracked.then_some(Refusal::Tracked), Some((index, then)), None)
            }
            // A `.git` git calls no repository, one pointing nowhere say:
            // its no is not an answer.
            None => (Some(Refusal::Unsure("git reads no repository from this .git".into())), None, Some(Instant::now() + UNSURE_FOR)),
        },
        Err(why) => {
            let until = unsure_until(&why);
            (repository.then_some(Refusal::Unsure(why)), None, Some(until))
        }
    };
    if let Ok(mut s) = seen.lock() {
        s.insert(bin.to_path_buf(), Seen { file, index, until, refusal: refusal.clone() });
    }
    refusal
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
pub fn semver(version: &str) -> Option<(u32, u32, u32)> {
    let mut parts = version.split_whitespace().next()?.split('.').map(|p| p.parse::<u32>().ok());
    Some((parts.next()??, parts.next()??, parts.next()??))
}

/// Whether the collin found is older than this dbt-edith accepts. One that
/// gave no version is.
pub fn outdated(version: Option<&str>) -> bool {
    version.and_then(semver).map_or(true, |v| v < WANTS)
}

/// The version each collin last asked gave, kept while its binary is the same
/// file: the menu asks on every opening, and an answer is a process start.
/// By path, since several can be installed side by side.
#[derive(Default)]
pub struct Versions(Mutex<std::collections::HashMap<PathBuf, (Option<std::time::SystemTime>, Option<String>)>>);

impl Versions {
    pub fn get(&self, bin: &Path) -> Option<String> {
        let modified = mtime(bin);
        if let Ok(seen) = self.0.lock() {
            if let Some((m, v)) = seen.get(bin) {
                if *m == modified {
                    return v.clone();
                }
            }
        }
        let v = version(bin);
        if let Ok(mut seen) = self.0.lock() {
            seen.insert(bin.to_path_buf(), (modified, v.clone()));
        }
        v
    }
}

/// Where versions installed side by side live: `<config>/collin/<X.Y.Z>/`,
/// each the `--root` of its own `cargo install`, so its binary is under `bin/`.
pub fn installs_dir(config_dir: &Path) -> PathBuf {
    config_dir.join("collin")
}

/// `0.4.0`, and nothing else: three numbers. A version names a folder and a
/// tag, and one from the browser is checked against this before either.
pub fn is_version(text: &str) -> bool {
    let parts: Vec<&str> = text.split('.').collect();
    parts.len() == 3 && parts.iter().all(|p| !p.is_empty() && p.len() <= 5 && p.bytes().all(|b| b.is_ascii_digit()))
}

/// One collin installed side by side.
#[derive(Clone, Debug, PartialEq)]
pub struct Install {
    pub version: String,
    pub bin: PathBuf,
}

/// The versions installed side by side, newest first. A folder that is not a
/// version, or holds no collin yet, as while cargo is still building it, is
/// not one.
pub fn installed(dir: &Path) -> Vec<Install> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut out: Vec<Install> = entries
        .flatten()
        .filter_map(|e| {
            let version = e.file_name().to_str().filter(|n| is_version(n))?.to_string();
            let bin = e.path().join("bin").join(exe_name());
            bin.is_file().then_some(Install { version, bin })
        })
        .collect();
    out.sort_by(|a, b| semver(&b.version).cmp(&semver(&a.version)));
    out
}

/// What installs `version` side by side, typed into the terminal for the user
/// to run, as the plain install is (0042). Its tag, never `main`: a version
/// picked by number has to stay that version. None when the folder's path
/// holds a quote, which no shell would read back as one argument.
pub fn install_command(dir: &Path, version: &str) -> Option<String> {
    let root = dir.join(version);
    let root = root.to_str().filter(|r| !r.contains('"'))?;
    Some(format!("cargo install --locked --git {REPO} --tag v{version} --root \"{root}\" collin-cli"))
}

/// The collin that runs: the version picked for the project when it is
/// installed, else the one beside dbt-edith or on the PATH that `locate`
/// found.
pub fn chosen_bin(installs: &[Install], chosen: Option<&str>, default: Option<&Path>) -> Option<PathBuf> {
    match chosen.and_then(|v| installs.iter().find(|i| i.version == v)) {
        Some(install) => Some(install.bin.clone()),
        None => default.map(Path::to_path_buf),
    }
}

/// Whether the collin that runs is called older than `WANTS`: then the menu
/// offers "Update collin" and the Columns tab says so first. Only the default
/// one can be, since it is the one the update reaches. A version installed by
/// its tag never is, whatever its number: its tag has nothing newer, so the
/// hint would promise an update the menu does not offer (0060, 0098). No
/// collin running, nothing to update.
pub fn update_offered(runs: Option<&Path>, installs: &[Install], version: Option<&str>) -> bool {
    runs.is_some_and(|bin| !installs.iter().any(|i| i.bin == bin)) && outdated(version)
}

/// `collin 0.4.0`, a cache's producer, to `0.4.0`.
pub fn producer_version(producer: &str) -> Option<&str> {
    producer.trim().strip_prefix("collin ").map(|v| v.split_whitespace().next().unwrap_or(v))
}

/// When the file at `path` was last written, following a link to it.
pub fn mtime(path: &Path) -> Option<std::time::SystemTime> {
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
    /// Runs finished since dbt-edith started. Two runs in a row both read
    /// `done`, so this is how the page tells a report it has not seen yet from
    /// the one it has.
    #[serde(skip_serializing_if = "is_zero_u64")]
    pub run: u64,
}

fn is_zero_u64(n: &u64) -> bool {
    *n == 0
}

/// Whether the manifest lies under the project folder, which is when collin
/// may be given `--project` (0109). collin reads a seed's CSV header from the
/// project it is named, so naming one beside another project's manifest would
/// give that manifest this folder's seeds; a manifest named alone is read with
/// what its own `root_path` says (0046, 0047). Both paths are resolved first,
/// links followed, so a link out of the folder is outside it and a link to
/// the folder is the folder, and compared a component at a time, so
/// `/work/jaffle_shop_2` is not under `/work/jaffle_shop`. A path that cannot
/// be resolved, a manifest not there, is not under: collin is then run
/// without the flag and says so itself.
pub fn manifest_in_project(manifest: &Path, project: &Path) -> bool {
    let real = |p: &Path| p.canonicalize().map(crate::files::plain);
    match (real(manifest), real(project)) {
        (Ok(manifest), Ok(project)) => manifest.starts_with(project),
        _ => false,
    }
}

/// The files one run reads and writes.
#[derive(Clone, Debug)]
pub struct Job {
    pub bin: PathBuf,
    pub project: PathBuf,
    pub manifest: PathBuf,
    pub catalog: PathBuf,
    /// The folder dbt compiled into, when the user named one; collin's own
    /// default, `compiled/` beside the manifest, otherwise.
    pub compiled: Option<PathBuf>,
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
    /// The collin the last run used. A request waiting out a run takes its
    /// outcome only when it asked for the same collin: one asking for another
    /// version runs that one.
    last_bin: Mutex<Option<PathBuf>>,
}

impl Runner {
    pub fn new(deadline: Duration) -> Runner {
        Runner {
            one: tokio::sync::Mutex::new(()),
            runs: AtomicU64::new(0),
            status: Mutex::new(Status { state: "idle", ..Default::default() }),
            deadline,
            last_bin: Mutex::new(None),
        }
    }

    pub fn status(&self) -> Status {
        let mut status = self.status.lock().map(|s| s.clone()).unwrap_or_default();
        status.run = self.runs.load(Ordering::SeqCst);
        status
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
        let same_bin = self.last_bin.lock().map(|b| b.as_deref() == Some(job.bin.as_path())).unwrap_or(false);
        if self.runs.load(Ordering::SeqCst) != before && same_bin {
            return self.status();
        }
        self.mark_running();
        if let Ok(mut last) = self.last_bin.lock() {
            *last = Some(job.bin.clone());
        }
        let deadline = self.deadline;
        let outcome = tokio::task::spawn_blocking(move || run(&job, deadline))
            .await
            .unwrap_or_else(|e| Err((format!("collin did not finish: {e}"), Vec::new())));
        let status = match outcome {
            Ok(log) => {
                load().await;
                Status { state: "done", log, ..Default::default() }
            }
            Err((error, log)) => Status { state: "failed", error, log, ..Default::default() },
        };
        self.set(status);
        self.runs.fetch_add(1, Ordering::SeqCst);
        self.status()
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
    /// Models and relations the report page lists, for the badge a run
    /// leaves behind it.
    #[serde(skip_serializing_if = "is_zero")]
    pub to_investigate: usize,
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
    /// The whole report filed by kind, for the report page. Built on the same
    /// read, since the file runs to megabytes.
    pub overview: crate::collin_report::Overview,
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
    use serde::Deserialize;
    let value: serde_json::Value = serde_json::from_slice(&std::fs::read(path).ok()?).ok()?;
    let raw = RawReport::deserialize(&value).ok()?;
    let overview = crate::collin_report::overview(&value);
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
        to_investigate: overview.to_investigate,
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
    Some(Report { summary, notes, overview })
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

/// `path` with this process's id and `.tmp` after its name, where a run
/// writes before the rename. The id keeps the page and a `--run-collin`
/// started beside it out of each other's files, the one removing the other's
/// half-written cache as it starts; the last rename wins. A run killed with
/// its process leaves its file, which discovery never offers, `.json` not
/// ending it.
fn tmp_of(path: &Path) -> PathBuf {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    path.with_file_name(format!("{name}.{}.tmp", std::process::id()))
}

/// One run, its last lines when it succeeds, or why it failed and its last
/// lines. The command line calls this directly; the page goes through
/// `Runner::generate`, which keeps to one run at a time. Neither can be asked
/// to stop before the deadline.
pub fn run(job: &Job, deadline: Duration) -> Result<Vec<String>, (String, Vec<String>)> {
    run_stoppable(job, deadline, &AtomicBool::new(false))
}

/// `run`, ended early when `stop` is set: collin is killed, and what it had
/// begun to write is removed, as for any failed run, so the cache standing
/// before it stays. For a caller that answers someone who can walk away while
/// collin works.
pub fn run_stoppable(job: &Job, deadline: Duration, stop: &AtomicBool) -> Result<Vec<String>, (String, Vec<String>)> {
    // Already asked to stop: collin is not started, and nothing is touched.
    if stop.load(Ordering::Relaxed) {
        return Err(("collin was stopped before it started".to_string(), Vec::new()));
    }
    let tmp = tmp_of(&job.out);
    // The report too: written in place, a failed run left a new report
    // beside the old cache, describing edges nobody could see.
    let report_tmp = tmp_of(&job.report);
    let _ = std::fs::remove_file(&tmp);
    let _ = std::fs::remove_file(&report_tmp);
    let mut command = Command::new(&job.bin);
    command.arg("generate");
    // Only when named: a collin before 0.9.0 refuses the flag.
    if let Some(dir) = &job.compiled {
        command.arg("--compiled").arg(dir);
    }
    // Always: the page draws the indirect edges as one box a model, and the
    // command line crosses them when asked, so a cache without them would
    // leave either to strip what the other wrote (0118).
    command.arg("--indirect");
    // Only when the manifest is the project's: collin reads the project's
    // seeds for it, and another project's manifest must not get them (0109).
    if manifest_in_project(&job.manifest, &job.project) {
        command.arg("--project").arg(&job.project);
    }
    let mut child = command
        .arg("--manifest")
        .arg(&job.manifest)
        .arg("--catalog")
        .arg(&job.catalog)
        .arg("--out")
        .arg(&tmp)
        .arg("--report")
        .arg(&report_tmp)
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
            Ok(None) if stop.load(Ordering::Relaxed) => {
                let _ = child.kill();
                let _ = child.wait();
                break Err("collin was stopped before it finished".to_string());
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
        let _ = std::fs::remove_file(&report_tmp);
        Err((error, log.clone()))
    };
    match exit {
        Err(e) => fail(e),
        Ok(status) if !status.success() => fail(explain(&log).unwrap_or_else(|| format!("collin exited with {status}"))),
        Ok(_) if !tmp.is_file() => fail(format!("collin finished without writing {}", tmp.display())),
        Ok(_) => match std::fs::rename(&tmp, &job.out) {
            Ok(()) => {
                // A report from an earlier run would describe another cache:
                // without a new one, there is none.
                if report_tmp.is_file() {
                    let _ = std::fs::rename(&report_tmp, &job.report);
                } else {
                    let _ = std::fs::remove_file(&job.report);
                }
                Ok(log)
            }
            Err(e) => fail(format!("cannot move collin's cache onto {}: {e}", job.out.display())),
        },
    }
}

#[cfg(all(test, unix))]
pub(crate) mod tests {
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
    pub(crate) fn executable(path: &Path, script: &str) {
        let draft = path.with_extension("draft");
        std::fs::write(&draft, script).unwrap();
        let _ = std::fs::remove_file(path);
        assert!(Command::new("cp").arg(&draft).arg(path).status().unwrap().success());
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    /// A stand-in for collin: a shell script that finds `--out` and `--report`
    /// among its arguments and runs `body` with them in `$out` and `$report`.
    fn fake(dir: &Path, body: &str) -> PathBuf {
        let bin = dir.join("collin");
        let script = format!(
            "#!/bin/sh\nout=''\nreport=''\nwhile [ $# -gt 0 ]; do [ \"$1\" = --out ] && out=\"$2\"; \
             [ \"$1\" = --report ] && report=\"$2\"; shift; done\n\
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
            compiled: None,
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
        let own = |_: &Path| None;
        assert_eq!(locate_in(Some(&beside), Some(path_var.clone()), own).bin, Some(on_path.join(exe_name())));
        std::fs::write(beside.join(exe_name()), "").unwrap();
        assert_eq!(
            locate_in(Some(&beside), Some(path_var), own),
            Located { bin: Some(beside.join(exe_name())), refused: None }
        );
        assert_eq!(locate_in(None, None, own), Located::default());
        // A relative entry names a folder the current one decides: skipped,
        // as `program::on_path` skips it.
        std::fs::remove_file(beside.join(exe_name())).unwrap();
        let relative = std::env::join_paths([Path::new("bin"), Path::new(".")]).unwrap();
        let cwd = std::env::current_dir().unwrap();
        assert_eq!(locate_in(Some(&beside), Some(relative), |_: &Path| None).bin, None, "from {}", cwd.display());
    }

    /// A git that ran out of time cost its whole deadline, so its silence is
    /// kept a minute; any other failure a few seconds.
    #[test]
    fn a_timed_out_answer_is_kept_longer() {
        let left = |why: &str| unsure_until(why).saturating_duration_since(Instant::now());
        assert!(left("git took longer than 10s and was stopped") > Duration::from_secs(50));
        assert!(left("fatal: detected dubious ownership in repository") <= UNSURE_FOR);
    }

    /// A dbt-edith copied into a project sits beside whatever the repository
    /// commits there. The collin git tracks is passed over and named; the
    /// user's own copy, untracked, still runs (0094). On main the tracked one
    /// was the one found.
    #[test]
    fn a_collin_git_tracks_is_passed_over_and_named() {
        let d = dir("tracked");
        let (project, on_path) = (d.join("project"), d.join("bin"));
        std::fs::create_dir_all(&project).unwrap();
        std::fs::create_dir_all(&on_path).unwrap();
        let git = |args: &[&str]| crate::git::run(&project, args, &[], Duration::from_secs(20));
        assert!(git(&["init", "-q"]).ok);
        let shipped = project.join(exe_name());
        executable(&shipped, "#!/bin/sh\ntouch \"$(dirname \"$0\")/shipped-ran\"\n");
        assert!(git(&["add", exe_name()]).ok);
        let path_var = std::env::join_paths([on_path.clone()]).unwrap();
        let tracked = Some((shipped.clone(), Refusal::Tracked));

        let none_else = locate_in(Some(&project), Some(path_var.clone()), refusal);
        assert_eq!(none_else, Located { bin: None, refused: tracked.clone() });
        let installed = on_path.join(exe_name());
        std::fs::write(&installed, "").unwrap();
        let found = locate_in(Some(&project), Some(path_var.clone()), refusal);
        assert_eq!(found, Located { bin: Some(installed.clone()), refused: tracked.clone() }, "the next one runs");
        assert!(!project.join("shipped-ran").exists(), "asking git runs nothing of the repository's");

        // Untracked, the same file is the user's again, though nothing about
        // it changed: the answer was kept against the index, which did.
        assert!(git(&["rm", "-q", "--cached", exe_name()]).ok);
        assert_eq!(locate_in(Some(&project), Some(path_var.clone()), refusal), Located { bin: Some(shipped.clone()), refused: None });
        assert!(git(&["add", exe_name()]).ok);
        assert_eq!(locate_in(Some(&project), Some(path_var.clone()), refusal).refused, tracked, "and tracked again");

        // Committed as `Collin`, where the file system finds it under
        // `collin`, as macOS and Windows do: git is asked case aside.
        assert!(git(&["rm", "-q", "--cached", exe_name()]).ok);
        std::fs::remove_file(&shipped).unwrap();
        let upper = project.join(exe_name().to_uppercase());
        executable(&upper, "#!/bin/sh\n");
        assert!(git(&["add", &exe_name().to_uppercase()]).ok);
        if shipped.is_file() {
            let found = locate_in(Some(&project), Some(path_var), refusal);
            assert_eq!(found.refused, Some((shipped, Refusal::Tracked)), "{found:?}");
        }

        // Outside a repository there is nobody to ask, and nothing is passed over.
        assert_eq!(refusal(&installed), None);
    }

    /// In a repository git cannot answer for, another account's say, not
    /// knowing is not taken for the user's own: the collin is passed over
    /// with git's reason. Outside one, there is nothing to doubt. On the first
    /// round of this change a failure ran it.
    #[test]
    fn a_collin_git_cannot_answer_for_is_passed_over_in_a_repository() {
        let d = dir("unsure");
        let project = d.join("project");
        std::fs::create_dir_all(project.join("bin")).unwrap();
        // A `.git` git refuses: not a repository it can read, nor nothing.
        std::fs::write(project.join(".git"), "gitdir: elsewhere\n").unwrap();
        let bin = project.join("bin").join(exe_name());
        executable(&bin, "#!/bin/sh\n");
        assert!(in_repository(&project.join("bin")));
        match refusal(&bin) {
            Some(Refusal::Unsure(why)) => assert!(!why.is_empty()),
            other => panic!("{other:?}"),
        }
        assert_eq!(locate_in(Some(&project.join("bin")), None, refusal).bin, None);
        let loose = d.join("loose");
        std::fs::create_dir_all(&loose).unwrap();
        assert!(!in_repository(&loose));
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
        assert!(!tmp_of(&d.join("column_lineage.collin.json")).exists());
    }

    #[tokio::test]
    async fn a_failed_run_says_why_and_leaves_the_old_cache_alone() {
        let d = dir("fail");
        std::fs::write(d.join("column_lineage.collin.json"), "old").unwrap();
        let bin = fake(&d, "echo half > \"$out\"\necho 'collin: no manifest at target/manifest.json' >&2\nexit 1");
        let status = Runner::new(Duration::from_secs(10)).generate(job(&d, bin), || async {}).await;
        assert_eq!((status.state, status.error.as_str()), ("failed", "no manifest at target/manifest.json"));
        assert_eq!(std::fs::read_to_string(d.join("column_lineage.collin.json")).unwrap(), "old");
        assert!(!tmp_of(&d.join("column_lineage.collin.json")).exists());

        let silent = fake(&d, "exit 0");
        let status = Runner::new(Duration::from_secs(10)).generate(job(&d, silent), || async {}).await;
        assert!(status.error.starts_with("collin finished without writing"), "{status:?}");
    }

    #[tokio::test]
    async fn the_report_moves_into_place_with_its_cache_and_never_alone() {
        let d = dir("report-run");
        let report = d.join(REPORT);
        std::fs::write(&report, "old").unwrap();
        let failing = fake(&d, "echo new > \"$report\"\necho half > \"$out\"\nexit 1");
        let runner = Runner::new(Duration::from_secs(10));
        assert_eq!(runner.generate(job(&d, failing), || async {}).await.state, "failed");
        assert_eq!(std::fs::read_to_string(&report).unwrap(), "old", "a failed run leaves the report of the cache on screen");
        assert!(!tmp_of(&d.join(REPORT)).exists());

        let writing = fake(&d, "echo new > \"$report\"\necho '{}' > \"$out\"");
        let done = runner.generate(job(&d, writing), || async {}).await;
        assert_eq!(std::fs::read_to_string(&report).unwrap().trim(), "new");
        assert_eq!((done.state, done.run), ("done", 2), "every finished run counts, a failed one too");

        let silent = fake(&d, "echo '{}' > \"$out\"");
        runner.generate(job(&d, silent), || async {}).await;
        assert!(!report.exists(), "a report left from an earlier run would describe another cache");
        assert_eq!(runner.status().run, 3);
    }

    #[test]
    fn a_version_is_three_numbers_and_nothing_a_path_could_hide_in() {
        for ok in ["0.4.0", "10.0.12"] {
            assert!(is_version(ok), "{ok}");
        }
        for bad in ["", "0.4", "0.4.0.1", "v0.4.0", "0.4.0-rc1", "../0.4.0", "0..4", "0.4.x", "123456.0.0"] {
            assert!(!is_version(bad), "{bad}");
        }
    }

    #[test]
    fn versions_side_by_side_are_the_folders_holding_a_collin_newest_first() {
        let d = dir("installed");
        for (folder, with_bin) in [("0.3.0", true), ("0.10.0", true), ("0.4.0", false), ("notes", true)] {
            let bin = d.join(folder).join("bin");
            std::fs::create_dir_all(&bin).unwrap();
            if with_bin {
                std::fs::write(bin.join(exe_name()), "").unwrap();
            }
        }
        let found: Vec<String> = installed(&d).into_iter().map(|i| i.version).collect();
        assert_eq!(found, ["0.10.0", "0.3.0"], "by number, not by name; one still building is not there yet");
        assert!(installed(&d.join("missing")).is_empty());
        let picked = chosen_bin(&installed(&d), Some("0.3.0"), None);
        assert_eq!(picked, Some(d.join("0.3.0").join("bin").join(exe_name())));
        let default = d.join("default");
        assert_eq!(chosen_bin(&installed(&d), Some("0.9.9"), Some(&default)), Some(default), "one not installed falls back");
    }

    #[test]
    fn only_the_default_collin_is_called_older() {
        let default = PathBuf::from("/usr/local/bin/collin");
        let pinned = |v: &str| Install { version: v.to_string(), bin: PathBuf::from(format!("/cfg/collin/{v}/bin/collin")) };
        let old = Some("0.2.0 (a142ee5)");
        assert!(update_offered(Some(&default), &[], old), "an older default is offered the update");
        let same = pinned("0.2.0");
        assert!(
            !update_offered(Some(&same.bin), &[same.clone()], old),
            "the same version installed by its tag and picked is not: no update reaches it"
        );
        let beside = [pinned("0.4.0")];
        assert!(update_offered(Some(&default), &beside, old), "a version installed beside a default that runs changes nothing");
        assert!(!update_offered(Some(&beside[0].bin), &beside, Some("0.4.0")), "nor is an older one picked to compare");
        assert!(!update_offered(Some(&default), &beside, Some("0.10.0 (86decf5)")), "a default at WANTS is not");
        assert!(!update_offered(None, &[], None), "no collin running has nothing to update");
    }

    #[test]
    fn a_version_installs_at_its_tag_into_its_own_folder() {
        let dir = Path::new("/home/me/.config/dbt-edith/collin");
        assert_eq!(
            install_command(dir, "0.3.0").unwrap(),
            "cargo install --locked --git https://github.com/IL-William/dbt-collin --tag v0.3.0 \
             --root \"/home/me/.config/dbt-edith/collin/0.3.0\" collin-cli"
        );
        assert_eq!(install_command(Path::new("/odd\"dir"), "0.3.0"), None);
    }

    #[test]
    fn a_producer_names_its_version() {
        assert_eq!(producer_version("collin 0.4.0"), Some("0.4.0"));
        assert_eq!(producer_version("snowflake"), None);
        assert_eq!(producer_version(""), None);
    }

    #[tokio::test]
    async fn a_request_for_another_collin_does_not_take_this_ones_outcome() {
        let d = dir("two-bins");
        let (a_dir, b_dir) = (d.join("a"), d.join("b"));
        std::fs::create_dir_all(&a_dir).unwrap();
        std::fs::create_dir_all(&b_dir).unwrap();
        let a = fake(&a_dir, "sleep 0.3\necho a > \"$out\"");
        let b = fake(&b_dir, "echo b > \"$out\"");
        let runner = Arc::new(Runner::new(Duration::from_secs(10)));
        let (ra, rb) = (runner.clone(), runner.clone());
        let (ja, jb) = (job(&d, a), job(&d, b));
        let first = tokio::spawn(async move { ra.generate(ja, || async {}).await });
        tokio::time::sleep(Duration::from_millis(50)).await;
        let second = tokio::spawn(async move { rb.generate(jb, || async {}).await });
        assert_eq!(first.await.unwrap().state, "done");
        assert_eq!(second.await.unwrap().state, "done");
        assert_eq!((runs(&a_dir), runs(&b_dir)), (1, 1), "each collin ran once");
        assert_eq!(std::fs::read_to_string(d.join("column_lineage.collin.json")).unwrap().trim(), "b");
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

    #[test]
    fn a_run_whose_stop_flag_is_set_kills_collin_and_leaves_the_old_cache() {
        let d = dir("stopped");
        let bin = fake(&d, "echo half > \"$out\"\necho half > \"$report\"\nexec sleep 30");
        let job = job(&d, bin);
        std::fs::write(&job.out, "old").unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let setter = {
            let stop = stop.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(300));
                stop.store(true, Ordering::Relaxed);
            })
        };
        let started = Instant::now();
        let (why, _) = run_stoppable(&job, Duration::from_secs(60), &stop).unwrap_err();
        setter.join().unwrap();
        assert!(why.contains("was stopped"), "{why}");
        assert!(started.elapsed() < Duration::from_secs(10), "{:?}", started.elapsed());
        assert_eq!(std::fs::read_to_string(&job.out).unwrap(), "old");
        assert!(!tmp_of(&job.out).exists() && !tmp_of(&job.report).exists(), "no half-written file is left");
    }

    #[test]
    fn a_stop_flag_already_set_keeps_collin_from_starting() {
        let d = dir("stopped-first");
        let bin = fake(&d, "echo new > \"$out\"");
        let job = job(&d, bin);
        std::fs::write(&job.out, "old").unwrap();
        let (why, log) = run_stoppable(&job, Duration::from_secs(60), &AtomicBool::new(true)).unwrap_err();
        assert!(why.contains("was stopped before it started"), "{why}");
        assert!(log.is_empty());
        assert_eq!(runs(&d), 0, "collin was never spawned");
        assert_eq!(std::fs::read_to_string(&job.out).unwrap(), "old");
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

    /// Runs `run` on a collin that writes what it was handed into `seen`, and
    /// gives those arguments, one per line.
    fn handed(d: &Path, project: &Path, manifest: &Path) -> Vec<String> {
        let bin = d.join("recorder");
        executable(
            &bin,
            "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$(dirname \"$0\")/seen\"\n\
             while [ $# -gt 0 ]; do [ \"$1\" = --out ] && echo '{}' > \"$2\"; shift; done\n",
        );
        let mut job = job(d, bin);
        job.project = project.to_path_buf();
        job.manifest = manifest.to_path_buf();
        run(&job, Duration::from_secs(10)).unwrap();
        std::fs::read_to_string(d.join("seen")).unwrap().lines().map(str::to_string).collect()
    }

    /// A project folder `name` under `d` with its manifest in `target/`.
    fn project_with_manifest(d: &Path, name: &str) -> (PathBuf, PathBuf) {
        let project = d.join(name);
        std::fs::create_dir_all(project.join("target")).unwrap();
        let manifest = project.join("target").join("manifest.json");
        std::fs::write(&manifest, "{}").unwrap();
        (project, manifest)
    }

    fn names_project(args: &[String], project: &Path) -> bool {
        args.windows(2).any(|w| w[0] == "--project" && Path::new(&w[1]) == project)
    }

    #[test]
    fn project_is_passed_for_a_manifest_under_it() {
        let d = dir("project-under");
        let (project, manifest) = project_with_manifest(&d, "jaffle_shop");
        let args = handed(&d, &project, &manifest);
        assert!(names_project(&args, &project), "collin reads the project's seeds for its own manifest: {args:?}");
        assert!(args.iter().any(|a| a == "--manifest"), "{args:?}");
        assert!(manifest_in_project(&manifest, &project));
    }

    #[test]
    fn project_is_not_passed_for_a_manifest_elsewhere() {
        let d = dir("project-elsewhere");
        let (project, _) = project_with_manifest(&d, "jaffle_shop");
        let (_, other) = project_with_manifest(&d, "jaffle_shop_analytics");
        let args = handed(&d, &project, &other);
        assert!(!args.iter().any(|a| a == "--project"), "another project's manifest gets none of this folder's seeds: {args:?}");
        assert!(args.iter().any(|a| a == "--manifest"), "{args:?}");
        assert!(!manifest_in_project(&other, &project));
    }

    #[test]
    fn a_sibling_sharing_the_name_prefix_is_not_under_the_project() {
        let d = dir("project-sibling");
        let (project, _) = project_with_manifest(&d, "jaffle_shop");
        let (_, sibling) = project_with_manifest(&d, "jaffle_shop_2");
        assert!(sibling.to_string_lossy().starts_with(&*project.to_string_lossy()), "a string prefix would say it is");
        assert!(!manifest_in_project(&sibling, &project));
        assert!(!handed(&d, &project, &sibling).iter().any(|a| a == "--project"));
    }

    #[test]
    fn project_follows_links_on_both_sides() {
        let d = dir("project-links");
        let (project, inside) = project_with_manifest(&d, "jaffle_shop");
        let (_, outside) = project_with_manifest(&d, "jaffle_shop_analytics");

        // A manifest that is a link out of the project is the other one's.
        let out_link = project.join("target").join("elsewhere.json");
        std::os::unix::fs::symlink(&outside, &out_link).unwrap();
        assert!(!manifest_in_project(&out_link, &project));
        assert!(!handed(&d, &project, &out_link).iter().any(|a| a == "--project"));

        // A link inside the project to a manifest inside it is the project's.
        let in_link = project.join("copy.json");
        std::os::unix::fs::symlink(&inside, &in_link).unwrap();
        assert!(manifest_in_project(&in_link, &project));
        assert!(names_project(&handed(&d, &project, &in_link), &project));

        // The project reached by a link is the same folder, and so is a path
        // that climbs out of the project and back into it.
        let by_link = d.join("shop");
        std::os::unix::fs::symlink(&project, &by_link).unwrap();
        assert!(manifest_in_project(&inside, &by_link));
        assert!(manifest_in_project(&by_link.join("target").join("manifest.json"), &project));
        let climbing = project.join("target").join("..").join("..").join("jaffle_shop").join("target").join("manifest.json");
        assert!(manifest_in_project(&climbing, &project));
        let escaping = project.join("..").join("jaffle_shop_analytics").join("target").join("manifest.json");
        assert!(!manifest_in_project(&escaping, &project));
    }

    #[test]
    fn a_manifest_that_is_not_there_is_not_under_the_project() {
        let d = dir("project-missing");
        let (project, _) = project_with_manifest(&d, "jaffle_shop");
        assert!(!manifest_in_project(&project.join("target").join("nothing.json"), &project));
        assert!(!manifest_in_project(&project.join("target").join("manifest.json"), &d.join("no_such_project")));
    }

    #[test]
    fn a_version_is_read_from_collin_and_compared_with_the_one_wanted() {
        let d = dir("version");
        let new = fake(&d, "echo 'collin 0.10.0 (86decf5)'");
        assert_eq!(version(&new).as_deref(), Some("0.10.0 (86decf5)"));
        assert!(!outdated(version(&new).as_deref()));
        assert!(outdated(Some("0.7.0 (77cbad3)")), "0.7.0 lists no roots, which the generated badge needs");
        assert!(outdated(Some("0.9.0 (c37bff0)")), "0.9.0 reads no seed's header (0109)");
        assert!(
            outdated(Some("0.2.0 (a142ee5)")),
            "the 0.2.0 of collin's old repository writes no roots either: once WANTS passed 0.2.0 it is offered the update (0047)"
        );
        let old = d.join("old");
        executable(&old, "#!/bin/sh\necho 'unknown command \"--version\"' >&2\nexit 2\n");
        assert_eq!(version(&old), None, "a collin from before --version has none");
        assert!(outdated(None));
        assert!(outdated(Some("0.0.9")));
        assert!(!outdated(Some("0.10.0 (abc)")), "compared as numbers, not text");
        assert!(outdated(Some("0.9.9")), "0.9.9 is below 0.10.0, which a text comparison gets wrong");
        assert!(!outdated(Some("1.0.0")));
        assert!(outdated(Some("garbage")));
        assert_eq!(semver(WANTS_TEXT), Some(WANTS), "the menu names the version the comparison uses");
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
