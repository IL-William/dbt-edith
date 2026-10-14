//! The orchestrator's jobs, read from the folders the user named outside the
//! project, and the nodes each of their steps acts on (0063).
//!
//! Two formats. dbt Cloud jobs are read in dbt-jobs-as-code's public schema.
//! Airflow DAGs are read in the shape this tool proposes, one per file, whose
//! keys a source may rename. Each step's command is read by `src/command.rs`
//! and resolved by the selection engine (0024, 0032), never by dbt.
//!
//! The sources form's picker lists one folder at a time, anywhere the user can
//! read, by the names of what a walk would enter or open there (0070).
//!
//! Invariants: only `.yml` and `.yaml` files are opened, never hidden ones and
//! never through a symlink. What leaves is what was parsed, and the names the
//! picker lists, never a line of a file. The same files over the same manifest
//! answer the same way byte for byte (0055), so every list follows the sorted
//! walk and the document's own order, and the one map here is only ever looked
//! up.

use crate::command::{self, Invocation, Verb};
use crate::graph::{Graph, Kind};
use crate::select::{self, Expr, Indirect, Spec, Tests};
use crate::settings::JobSource;
use crate::yaml::{self, Node, Value};
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex, Weak};
use std::time::SystemTime;

pub const AIRFLOW: &str = "airflow";
pub const DBT_CLOUD: &str = "dbt_cloud";
/// More than anyone lists, and a bound on the walks the page can start.
pub const MAX_SOURCES: usize = 8;
const MAX_FILES: usize = 5_000;
const MAX_DEPTH: usize = 16;
/// Folders a walk never enters, besides hidden ones (0020). Only names no
/// one gives a folder of jobs: `target` and `logs` are layers of DAGs in some
/// repositories, which a longer list here once dropped without a word.
const SKIPPED_DIRS: &[&str] = &["node_modules", "__pycache__"];

/// The Airflow keys a source may rename, with the shape this tool proposes.
/// `tasks` and the sensors' list sit at the top of a file; the others inside
/// one task or one sensor.
pub const AIRFLOW_KEYS: &[(&str, &str)] = &[
    ("dag_id", "dag_id"),
    ("schedule", "schedule"),
    ("tasks", "tasks"),
    ("task_id", "task_id"),
    ("command", "command"),
    ("depends_on", "depends_on"),
    ("outlets", "outlets"),
    ("sensors", "sensors"),
    ("sensor_id", "task_id"),
    ("external_dag_id", "external_dag_id"),
    ("external_task_id", "external_task_id"),
];

fn key(source: &JobSource, name: &'static str) -> String {
    match source.keys.get(name).filter(|v| !v.is_empty()) {
        Some(v) => v.clone(),
        None => AIRFLOW_KEYS.iter().find(|(k, _)| *k == name).map(|(_, v)| v.to_string()).unwrap_or_default(),
    }
}

fn valid_key_path(v: &str) -> bool {
    !v.is_empty()
        && v.len() <= 200
        && v.split('.').all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-'))
}

/// Why a source cannot be saved, or None. Whether its path leads anywhere is
/// the caller's to ask, since it knows the project.
pub fn check(source: &JobSource) -> Option<String> {
    if source.kind != AIRFLOW && source.kind != DBT_CLOUD {
        return Some("a source is airflow or dbt_cloud".into());
    }
    if source.path.trim().is_empty() {
        return Some("a source needs a folder or a file".into());
    }
    if source.path.len() > 1024 || source.path.chars().any(char::is_control) {
        return Some("a path of at most 1024 characters, with no control characters".into());
    }
    if source.label.len() > 80 || source.label.chars().any(char::is_control) {
        return Some("a label of at most 80 characters, with no control characters".into());
    }
    if source.kind == DBT_CLOUD && !source.keys.is_empty() {
        return Some("dbt Cloud jobs are read in dbt-jobs-as-code's own schema, whose keys do not move".into());
    }
    for (k, v) in &source.keys {
        if !AIRFLOW_KEYS.iter().any(|(name, _)| name == k) {
            return Some(format!("`{k}` is not a key this tool reads"));
        }
        if !valid_key_path(v) {
            return Some(format!("`{v}` is not a dotted key path such as pipeline.name"));
        }
    }
    None
}

/// Where a source's path leads: as written when absolute, else from the project.
pub fn locate(root: &Path, path: &str) -> PathBuf {
    let p = Path::new(path.trim());
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        root.join(p)
    }
}

/* ----------------------------------------------------------------- walk -- */

type Stamp = (PathBuf, u64, Option<SystemTime>);

fn is_yaml(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("yml") || e.eq_ignore_ascii_case("yaml"))
}

/// The job files of one source, sorted, each with its size and time, and
/// whether the walk stopped at a bound.
fn walk(path: &Path) -> Result<(Vec<Stamp>, bool), String> {
    let meta = std::fs::symlink_metadata(path).map_err(|_| "nothing at this path".to_string())?;
    if meta.file_type().is_symlink() {
        return Err("a symlink, which this tool does not follow".into());
    }
    if meta.is_file() {
        if !is_yaml(path) {
            return Err("not a .yml or .yaml file".into());
        }
        return Ok((vec![(path.to_path_buf(), meta.len(), meta.modified().ok())], false));
    }
    let mut out = Vec::new();
    let mut capped = false;
    descend(path, 0, &mut out, &mut capped);
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok((out, capped))
}

fn descend(dir: &Path, depth: usize, out: &mut Vec<Stamp>, capped: &mut bool) {
    if depth > MAX_DEPTH {
        *capped = true;
        return;
    }
    let Ok(read) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<std::fs::DirEntry> = read.flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        if out.len() >= MAX_FILES {
            *capped = true;
            return;
        }
        match walked(&e, &e.file_name().to_string_lossy()) {
            Some(true) => descend(&e.path(), depth + 1, out, capped),
            Some(false) => {
                if let Ok(m) = e.metadata() {
                    out.push((e.path(), m.len(), m.modified().ok()));
                }
            }
            None => {}
        }
    }
}

/// What the walk makes of one entry of a folder: `Some(true)` for a folder it
/// enters, `Some(false)` for a file it opens, `None` for anything else. The
/// sources form's picker lists by the same rule (0070).
fn walked(e: &std::fs::DirEntry, name: &str) -> Option<bool> {
    // A `.env` starts with a dot, so it is never opened (0020).
    if name.starts_with('.') {
        return None;
    }
    // From the entry itself, never through the link: a symlink, or a
    // junction on Windows, could lead anywhere on the machine.
    let kind = e.file_type().ok()?;
    if kind.is_symlink() {
        None
    } else if kind.is_dir() {
        (!SKIPPED_DIRS.contains(&name)).then_some(true)
    } else if kind.is_file() && is_yaml(Path::new(name)) {
        Some(false)
    } else {
        None
    }
}

fn shown_path(base: &Path, file: &Path) -> String {
    let rel = file.strip_prefix(base).ok().filter(|r| !r.as_os_str().is_empty());
    let rel = rel.unwrap_or_else(|| Path::new(file.file_name().unwrap_or_default()));
    rel.to_string_lossy().replace('\\', "/")
}

/* --------------------------------------------------------------- browse -- */

/// Far more than a folder of jobs holds, and a bound on one listing.
const MAX_LISTED: usize = 2_000;
/// How far above the project a place may sit and still be named from it: far
/// enough for a repository beside the project's, or beside the repository the
/// project is a folder of.
const MAX_UPS: usize = 2;

/// One folder as the sources form shows it: the folders a walk would enter
/// and the files it would open, and nothing else (0070).
#[derive(serde::Serialize, Debug, PartialEq)]
pub struct Listing {
    /// The folder, in full, as the system spells it.
    pub path: String,
    /// The folder as a source names it.
    pub written: String,
    /// The folder above, as a source names it, absent at a root.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub up: Option<String>,
    /// The file the path asked for, when it is one of the entries.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub picked: String,
    /// The path asked for is not a folder the picker reaches, so the nearest
    /// one above it is listed instead.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub moved: bool,
    /// Why the system would not list the folder, which is then shown empty
    /// and still left from.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub refused: String,
    pub entries: Vec<Entry>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub capped: bool,
}

#[derive(serde::Serialize, Debug, PartialEq)]
pub struct Entry {
    pub name: String,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub dir: bool,
    pub written: String,
}

/// A path with its `.` and `..` worked out from the spelling alone. Nothing
/// listed is ever a symlink, so a `..` leaves the folder it was taken from.
fn normal(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            c => out.push(c.as_os_str()),
        }
    }
    out
}

/// How a source names a place: from the project when it sits at most
/// `MAX_UPS` folders above it, which is short and still right when the
/// checkouts move together; in full otherwise, as on another drive.
fn written(root: &Path, at: &Path) -> String {
    let r: Vec<Component> = root.components().collect();
    let a: Vec<Component> = at.components().collect();
    let common = r.iter().zip(&a).take_while(|(x, y)| x == y).count();
    let ups = r.len() - common;
    if common == 0 || ups > MAX_UPS {
        return at.to_string_lossy().into_owned();
    }
    let mut parts = vec!["..".to_string(); ups];
    parts.extend(a[common..].iter().map(|c| c.as_os_str().to_string_lossy().into_owned()));
    if parts.is_empty() {
        ".".into()
    } else {
        parts.join("/")
    }
}

/// The deepest folder on the way to `asked` the picker stands on: never in a
/// hidden folder past the ones the project itself sits in, which are taken as
/// they are, and never on a link, which no listing shows. A link further up a
/// typed path is followed, as a source's own path is (0063): `/tmp` is one on
/// macOS.
fn reachable(root: &Path, asked: &Path) -> PathBuf {
    let common = root.components().zip(asked.components()).take_while(|(x, y)| x == y).count();
    let mut at: PathBuf = asked.components().take(common).collect();
    for c in asked.components().skip(common) {
        if let Component::Normal(name) = c {
            if name.to_str().is_none_or(|n| n.starts_with('.')) {
                break;
            }
        }
        at.push(c.as_os_str());
    }
    while !std::fs::symlink_metadata(&at).is_ok_and(|m| m.is_dir()) {
        if !at.pop() {
            break;
        }
    }
    at
}

/// The folder a source's path leads to, or the nearest folder above it the
/// picker reaches, with what the walk would see there: no hidden entry, no
/// symlink or junction, no file but YAML. Folders first, then by name,
/// whatever order the system lists them in (0055).
pub fn browse(root: &Path, path: &str) -> Result<Listing, String> {
    let path = path.trim();
    if path.len() > 1024 || path.chars().any(char::is_control) {
        return Err("a path of at most 1024 characters, with no control characters".into());
    }
    let asked = normal(&locate(root, if path.is_empty() { "." } else { path }));
    let at = reachable(root, &asked);
    let mut picked = String::new();
    if asked.parent() == Some(at.as_path()) {
        if let Some(name) = asked.file_name().and_then(|n| n.to_str()) {
            if !name.starts_with('.') && is_yaml(&asked) && std::fs::symlink_metadata(&asked).is_ok_and(|m| m.is_file()) {
                picked = name.to_string();
            }
        }
    }
    let mut found: Vec<(bool, String)> = Vec::new();
    let refused = match std::fs::read_dir(&at) {
        Ok(read) => {
            for e in read.flatten() {
                // A name JSON cannot carry is one no source could be saved with.
                let Some(name) = e.file_name().to_str().map(str::to_owned) else { continue };
                if let Some(dir) = walked(&e, &name) {
                    found.push((dir, name));
                }
            }
            String::new()
        }
        Err(e) => e.to_string(),
    };
    found.sort_by_cached_key(|(dir, name)| (!dir, name.to_lowercase(), name.clone()));
    let capped = found.len() > MAX_LISTED;
    if capped {
        // The file the path named stays listed past the bound, so Use keeps it.
        let past = found.split_off(MAX_LISTED);
        found.extend(past.into_iter().filter(|(dir, name)| !dir && !picked.is_empty() && *name == picked));
    }
    let entries = found.into_iter().map(|(dir, name)| Entry { written: written(root, &at.join(&name)), dir, name }).collect();
    Ok(Listing {
        path: at.to_string_lossy().into_owned(),
        written: written(root, &at),
        up: at.parent().map(|p| written(root, p)),
        moved: at != asked && picked.is_empty(),
        picked,
        refused,
        entries,
        capped,
    })
}

/* --------------------------------------------------------------- model -- */

/// A file that could not be read, at a line when there is one.
#[derive(serde::Serialize, Clone, Debug, PartialEq)]
pub struct Refusal {
    pub file: String,
    #[serde(skip_serializing_if = "is_zero")]
    pub line: usize,
    pub message: String,
}

fn is_zero(n: &usize) -> bool {
    *n == 0
}

/// One source as read, for the settings form to say what it found.
#[derive(serde::Serialize, Clone, Debug, Default)]
pub struct SourceRead {
    pub kind: String,
    pub path: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub label: String,
    #[serde(skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub keys: std::collections::BTreeMap<String, String>,
    /// Where the path led.
    pub resolved: String,
    /// Why nothing was read at all.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub error: String,
    pub files: usize,
    pub jobs: usize,
    pub steps: usize,
    /// Files read that hold no job: other YAML living beside the jobs.
    #[serde(skip_serializing_if = "is_zero")]
    pub skipped: usize,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub refused: Vec<Refusal>,
    /// The walk stopped at its bound on files or depth.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub capped: bool,
}

/// An Airflow sensor: this DAG waits for a task of another, or its whole run.
#[derive(Clone, Debug)]
pub struct Sensor {
    pub id: String,
    pub dag: String,
    pub task: String,
}

/// Whether something starts a job without anyone asking, as far as the files
/// say. Ordered, so that a chain of triggers takes the best of what pulls it
/// and a DAG on several assets the worst of them (0077).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Start {
    /// Nothing does: no schedule, `@once`, CI, a merge, or a chain ending there.
    #[default]
    No,
    /// It hangs on what no file read says: a templated schedule, a schedule
    /// or an asset written as a mapping, an asset no file read updates, a job
    /// no file defines.
    Unread,
    /// A clock, read or not, or a trigger that leads back to one.
    Yes,
}

/// A DAG or a dbt Cloud job.
#[derive(Clone, Debug, Default)]
pub struct Job {
    pub source: usize,
    pub kind: &'static str,
    pub file: String,
    pub line: usize,
    pub name: String,
    /// dbt-jobs-as-code's identifier, the key under `jobs:`.
    pub ident: String,
    /// The ids a completion trigger may name it by: `id` and `linked_id`.
    ids: Vec<String>,
    pub env: String,
    pub target: String,
    /// On a clock: a cron, or an Airflow preset such as `@daily`.
    pub cron: String,
    /// Airflow: runs once these assets are updated.
    pub assets: Vec<String>,
    /// Airflow: it waits on an asset too, one this tool cannot name.
    asset_unread: bool,
    /// dbt Cloud: runs once this job completes, by its id.
    pub after: String,
    after_job: Option<usize>,
    pub ci: bool,
    pub merge: bool,
    /// Why its schedule is not known, when it is not.
    pub note: String,
    pub sensors: Vec<Sensor>,
    pub steps: Vec<usize>,
    /// What its tasks with no dbt command update, which starts DAGs too.
    outlets: Vec<String>,
    /// What its own schedule or trigger says, before any chain.
    own: Start,
    /// What starts it, chains followed (0063, 0077).
    pub start: Start,
    /// Its assets that no file read updates.
    pub assets_unread: Vec<String>,
}

/// A DAG with no dbt task that updates an asset: not a job here, since it
/// runs no node, but it may be what starts one (0077).
#[derive(Clone, Debug, Default)]
pub struct Feeder {
    pub assets: Vec<String>,
    asset_unread: bool,
    pub outlets: Vec<String>,
    own: Start,
}

/// One task of a DAG, or one step of a dbt Cloud job.
#[derive(Clone, Debug, Default)]
pub struct Step {
    pub job: usize,
    /// From 1, within its job.
    pub n: usize,
    /// The Airflow task id. A dbt Cloud step has none.
    pub name: String,
    pub line: usize,
    pub shown: String,
    pub inv: Option<Invocation>,
    /// Added by dbt Cloud for `run_generate_sources`, not written as a step.
    pub implicit: bool,
    pub upstream: Vec<String>,
    pub outlets: Vec<String>,
    /// Node indices, ascending, already narrowed to what the verb acts on.
    pub nodes: Vec<u32>,
    pub warnings: Vec<String>,
    /// Why the step could not be read or resolved; empty when it was.
    pub unresolved: String,
}

impl Step {
    fn verb(&self) -> Option<Verb> {
        self.inv.as_ref().map(|i| i.verb)
    }
    fn executes(&self) -> bool {
        self.unresolved.is_empty() && self.verb().is_some_and(Verb::executes)
    }
}

#[derive(Default)]
pub struct Read {
    pub sources: Vec<SourceRead>,
    pub jobs: Vec<Job>,
    pub steps: Vec<Step>,
    pub feeders: Vec<Feeder>,
}

/* ------------------------------------------------------------- reading -- */

fn read_doc(path: &Path, len: u64) -> Result<Node, (usize, String)> {
    if len > crate::project::MAX_BYTES {
        return Err((0, "larger than 1 MiB, which no job file is".into()));
    }
    let bytes = std::fs::read(path).map_err(|_| (0, "cannot be read".to_string()))?;
    yaml::parse(&crate::envs::decode(&bytes)).map_err(|u| (u.line, u.message.to_string()))
}

fn text(node: Option<&Node>) -> Option<String> {
    node.and_then(Node::str).map(str::to_string)
}

/// YAML 1.1's spellings of true, which PyYAML and so dbt-jobs-as-code read.
fn truthy(node: Option<&Node>) -> bool {
    node.and_then(Node::str).is_some_and(|s| matches!(s, "true" | "True" | "TRUE" | "yes" | "Yes" | "YES" | "on" | "On" | "ON"))
}

/// A list of names, or one name on its own.
fn names(node: Option<&Node>) -> Vec<String> {
    match node.map(|n| &n.value) {
        Some(Value::Str(s)) => vec![s.clone()],
        Some(Value::Seq(items)) => items.iter().filter_map(|n| n.str().map(str::to_string)).collect(),
        _ => Vec::new(),
    }
}

/// The first words of a line that could not be read, for knowing which one
/// it is: never past the first flag, where a value could be a secret.
fn glimpse(line: &str) -> String {
    let words: Vec<&str> = line
        .split_whitespace()
        .skip_while(|w| w.contains('=') && !w.starts_with('-'))
        .take_while(|w| !w.starts_with('-') && !w.contains(['$', '`', '{', '\'', '"']))
        .take(3)
        .collect();
    format!("{} …", words.join(" ")).trim_start().to_string()
}

fn new_step(job: usize, n: usize, name: String, line: usize, command: Option<&str>) -> Step {
    let mut step = Step { job, n, name, line, ..Default::default() };
    match command {
        None => {
            step.shown = "…".into();
            step.unresolved = "a command that is not text".into();
        }
        Some(c) => match command::read(c) {
            Ok(inv) => {
                step.shown = inv.shown.clone();
                step.inv = Some(inv);
            }
            Err(why) => {
                step.shown = glimpse(c);
                step.unresolved = why;
            }
        },
    }
    step
}

/// What an Airflow schedule says: a clock, assets, nothing, or why it cannot
/// be read.
#[derive(Default)]
struct Schedule {
    cron: String,
    assets: Vec<String>,
    asset_unread: bool,
    note: String,
    own: Start,
}

/// Jinja, in either of its two forms, is only known once rendered.
fn templated(s: &str) -> bool {
    s.contains("{{") || s.contains("{%")
}

fn airflow_schedule(node: Option<&Node>) -> Schedule {
    let unread = |note: &str| Schedule { note: note.into(), own: Start::Unread, ..Default::default() };
    match node.map(|n| &n.value) {
        None | Some(Value::Null) => Schedule::default(),
        Some(Value::Str(s)) => match s.as_str() {
            "None" | "null" | "" => Schedule::default(),
            "@once" => Schedule { note: "runs once, at its start date".into(), ..Default::default() },
            s if templated(s) => unread("a schedule only known when the DAG is parsed"),
            s => Schedule { cron: s.to_string(), own: Start::Yes, ..Default::default() },
        },
        Some(Value::Seq(items)) => {
            let assets: Vec<String> = items.iter().filter_map(|n| n.str().map(str::to_string)).collect();
            // An asset written as a mapping, as dag-factory writes one with
            // its `uri`, is waited on all the same: the DAG may start or not,
            // whatever the named ones say.
            let note = if items.is_empty() {
                "an empty list of assets, which this tool does not read"
            } else if assets.len() < items.len() {
                "an asset written as a mapping, which this tool does not read"
            } else {
                ""
            };
            Schedule { assets, asset_unread: !note.is_empty(), note: note.into(), ..Default::default() }
        }
        Some(Value::Map(_)) => unread("a schedule written as a mapping, which this tool does not read"),
    }
}

/// One Airflow DAG file. Ok(false) for a file that holds no dbt task, which
/// still leaves a feeder behind when it updates an asset.
fn read_airflow(
    si: usize,
    src: &JobSource,
    file: &str,
    doc: &Node,
    jobs: &mut Vec<Job>,
    steps: &mut Vec<Step>,
    feeders: &mut Vec<Feeder>,
) -> Result<bool, Refusal> {
    let refuse = |line: usize, message: String| Refusal { file: file.to_string(), line, message };
    let tasks_key = key(src, "tasks");
    let Some(tasks) = doc.at_path(&tasks_key) else { return Ok(false) };
    // A list of tasks each naming itself, or a mapping keyed by task id, the
    // way dag-factory writes them.
    let list: Vec<(Option<&str>, &Node)> = match &tasks.value {
        Value::Seq(items) => items.iter().map(|n| (None, n)).collect(),
        Value::Map(entries) => entries.iter().map(|(k, n)| (Some(k.as_str()), n)).collect(),
        Value::Null => return Ok(false),
        Value::Str(_) => return Err(refuse(tasks.line, format!("`{tasks_key}` is neither a list nor a mapping of tasks"))),
    };
    let command_key = key(src, "command");
    let is_dbt = |t: &Node| t.at_path(&command_key).is_some_and(|c| !c.is_null());
    let (dag_key, outlets_key) = (key(src, "dag_id"), key(src, "outlets"));
    let others: Vec<String> = list.iter().filter(|(_, t)| !is_dbt(t)).flat_map(|(_, t)| names(t.at_path(&outlets_key))).collect();
    if !list.iter().any(|(_, t)| is_dbt(t)) {
        // Never refused: it is no job, and only what it updates matters.
        if !others.is_empty() && text(doc.at_path(&dag_key)).is_some_and(|n| !n.is_empty()) {
            let sched = airflow_schedule(doc.at_path(&key(src, "schedule")));
            feeders.push(Feeder { assets: sched.assets, asset_unread: sched.asset_unread, outlets: others, own: sched.own });
        }
        return Ok(false);
    }
    let Some(dag) = doc.at_path(&dag_key).filter(|n| n.str().is_some_and(|s| !s.is_empty())) else {
        return Err(refuse(0, format!("dbt tasks, and no DAG id at `{dag_key}`")));
    };
    let name = dag.str().unwrap_or_default().to_string();
    let sched = airflow_schedule(doc.at_path(&key(src, "schedule")));
    let mut job = Job {
        source: si,
        kind: AIRFLOW,
        file: file.into(),
        line: dag.line,
        ident: name.clone(),
        name,
        cron: sched.cron,
        assets: sched.assets,
        asset_unread: sched.asset_unread,
        note: sched.note,
        own: sched.own,
        outlets: others,
        ..Default::default()
    };
    let (sid, sdag, stask) = (key(src, "sensor_id"), key(src, "external_dag_id"), key(src, "external_task_id"));
    for s in doc.at_path(&key(src, "sensors")).and_then(Node::items).unwrap_or_default() {
        if let Some(dag) = text(s.at_path(&sdag)) {
            job.sensors.push(Sensor { id: text(s.at_path(&sid)).unwrap_or_default(), dag, task: text(s.at_path(&stask)).unwrap_or_default() });
        }
    }
    let ji = jobs.len();
    let (id_key, deps_key) = (key(src, "task_id"), key(src, "depends_on"));
    for (own, task) in list {
        let Some(command) = task.at_path(&command_key).filter(|c| !c.is_null()) else { continue };
        let name = text(task.at_path(&id_key)).or_else(|| own.map(str::to_string)).unwrap_or_default();
        let mut step = new_step(ji, job.steps.len() + 1, name, command.line, command.str());
        step.upstream = names(task.at_path(&deps_key));
        step.outlets = names(task.at_path(&outlets_key));
        job.steps.push(steps.len());
        steps.push(step);
    }
    jobs.push(job);
    Ok(true)
}

/// One dbt-jobs-as-code file. Ok(false) for a file with no `jobs:`.
fn read_cloud(si: usize, file: &str, doc: &Node, jobs: &mut Vec<Job>, steps: &mut Vec<Step>) -> Result<bool, Refusal> {
    let refuse = |line: usize, message: &str| Refusal { file: file.to_string(), line, message: message.to_string() };
    let Some(all) = doc.get("jobs") else { return Ok(false) };
    let Some(entries) = all.entries() else {
        return Err(refuse(all.line, "`jobs:` is not a mapping of jobs"));
    };
    for (ident, j) in entries {
        if j.entries().is_none() {
            return Err(refuse(j.line, "a job that is not a mapping"));
        }
        let triggers = j.get("triggers");
        let on = |k: &str| truthy(triggers.and_then(|t| t.get(k)));
        let job_type = j.get("job_type").and_then(Node::str).unwrap_or("scheduled");
        let mut job = Job {
            source: si,
            kind: DBT_CLOUD,
            file: file.into(),
            line: j.line,
            name: text(j.get("name")).filter(|n| !n.is_empty()).unwrap_or_else(|| ident.clone()),
            ident: ident.clone(),
            ids: [j.get("id"), j.get("linked_id")].into_iter().filter_map(text).collect(),
            env: text(j.get("environment_id")).unwrap_or_default(),
            target: text(j.at_path("settings.target_name")).unwrap_or_default(),
            ci: on("github_webhook") || on("git_provider_webhook") || job_type == "ci",
            merge: on("on_merge") || job_type == "merge",
            after: text(j.at_path("job_completion_trigger_condition.condition.job_id")).unwrap_or_default(),
            ..Default::default()
        };
        // The trigger alone says whether dbt Cloud runs it on a clock; the
        // cron only says when (0077).
        let switch = text(triggers.and_then(|t| t.get("schedule"))).is_some_and(|t| templated(&t));
        if on("schedule") || switch {
            match text(j.at_path("schedule.cron")) {
                Some(c) if templated(&c) => job.note = "a schedule only the vars file knows".into(),
                Some(c) if !c.is_empty() => job.cron = c,
                _ => job.note = "on a schedule this tool cannot read".into(),
            }
            job.own = if switch { Start::Unread } else { Start::Yes };
        }
        if switch {
            job.note = "a schedule only the vars file turns on".into();
        }
        if templated(&job.after) {
            job.note = "after a job only the vars file names".into();
            job.after.clear();
            job.own = job.own.max(Start::Unread);
        }
        let ji = jobs.len();
        let mut commands: Vec<(usize, Option<String>, bool)> = Vec::new();
        // dbt Cloud runs source freshness first when this is set, as a step
        // of the job that no file writes.
        if truthy(j.get("run_generate_sources")) {
            commands.push((j.get("run_generate_sources").map_or(j.line, |n| n.line), Some("dbt source freshness".into()), true));
        }
        for s in j.get("execute_steps").and_then(Node::items).unwrap_or_default() {
            commands.push((s.line, s.str().map(str::to_string), false));
        }
        for (line, command, implicit) in commands {
            let mut step = new_step(ji, job.steps.len() + 1, String::new(), line, command.as_deref());
            step.implicit = implicit;
            job.steps.push(steps.len());
            steps.push(step);
        }
        jobs.push(job);
    }
    Ok(!entries.is_empty())
}

/* ----------------------------------------------------------- resolving -- */

/// The method of a term, `state` in `@2+state:modified+`.
fn method_of(term: &str) -> &str {
    let t = term.trim_start_matches('@').trim_start_matches(|c: char| c.is_ascii_digit()).trim_start_matches('+');
    t.split_once(':').map_or("", |(m, _)| m)
}

type Answer = Result<(Vec<u32>, Vec<String>), String>;

const LOST_INDIRECT: &str = "this manifest lost the indirect_selection selectors.yml sets, and this selector's tests \
                             depend on it: they follow dbt's default here. Re-parse with a recent dbt Fusion to fix it; \
                             dbt ls settles any doubt";

/// What one invocation selects, before its verb narrows it, the way
/// `ls::resolve` orders it: a named selector wins over `--select`, then the
/// typed terms, then dbt's default selector, then everything.
fn select_nodes(graph: &Graph, inv: &Invocation) -> Answer {
    let mut warnings = Vec::new();
    if let Some(name) = &inv.selector {
        // Said first, as the Custom selection box says it (0032): it changes
        // what every test in the answer means.
        if graph.selectors.lost_indirect() && graph.selectors.depends_on_indirect(graph, name) {
            warnings.push(LOST_INDIRECT.to_string());
        }
        if !inv.select.is_empty() || !inv.exclude.is_empty() {
            warnings.push("dbt ignores --select and --exclude beside --selector".to_string());
        }
        let r = graph.selectors.resolve(graph, name).map_err(|e| e.to_string())?;
        warnings.extend(r.warnings);
        return Ok((r.nodes, warnings));
    }
    for term in inv.select.iter().chain(&inv.exclude) {
        if crate::selectors::STATEFUL.contains(&method_of(term)) {
            return Err(format!("`{term}` compares with another run's artifacts, through --state, which this tool never reads"));
        }
    }
    if inv.select.is_empty() && inv.exclude.is_empty() {
        let defaults: Vec<&str> = graph.selectors.list().iter().filter(|n| n.default).map(|n| n.name.as_str()).collect();
        match defaults.as_slice() {
            [] => {}
            [one] => {
                let r = graph.selectors.resolve(graph, one).map_err(|e| e.to_string())?;
                return Ok((r.nodes, r.warnings));
            }
            _ => return Err("more than one selector is marked default, which dbt refuses".into()),
        }
    }
    // No `--select` is everything dbt would list. Sources are matched by
    // `source:` alone, so they are named apart.
    let everything = ["fqn:*".to_string(), "source:*".to_string()];
    let include: &[String] = if inv.select.is_empty() { &everything } else { &inv.select };
    let expr = Expr::from_terms(include, &inv.exclude).map_err(|e| e.to_string())?;
    let spec = expr.spec_with(inv.indirect.unwrap_or(Indirect::Eager));
    let r = select::resolve_spec(graph, &spec, Tests::Eager, &no_selectors).map_err(|e| e.to_string())?;
    Ok((r.nodes, r.warnings))
}

fn no_selectors(_: &str) -> Option<&'static Spec> {
    None
}

fn resolve_steps(graph: &Graph, steps: &mut [Step]) {
    // Jobs repeat selections; each distinct one is resolved once.
    let mut memo: HashMap<String, Answer> = HashMap::new();
    for step in steps.iter_mut() {
        let Some(inv) = &step.inv else { continue };
        let Some(kinds) = inv.kinds() else { continue };
        let memo_key = format!("{:?}|{:?}|{:?}|{:?}", inv.selector, inv.select, inv.exclude, inv.indirect);
        let answer = memo.entry(memo_key).or_insert_with(|| select_nodes(graph, inv)).clone();
        match answer {
            Ok((nodes, warnings)) => {
                step.nodes = nodes.into_iter().filter(|&i| kinds.contains(&graph.nodes[i as usize].kind)).collect();
                step.warnings = warnings;
            }
            Err(why) => step.unresolved = why,
        }
    }
}

/// Completion triggers to the jobs they name, and what starts each job: its
/// own clock, a job whose end starts it, or the assets it waits on, each
/// updated by a task of a DAG read or by a feeder (0077).
fn link(read: &mut Read) {
    let jobs = &mut read.jobs;
    for i in 0..jobs.len() {
        if jobs[i].after.is_empty() {
            continue;
        }
        let want = jobs[i].after.clone();
        jobs[i].after_job = jobs.iter().position(|j| j.kind == DBT_CLOUD && j.ids.contains(&want));
    }
    // Jobs first, then feeders, in one numbering: a feeder may itself wait on
    // an asset a DAG updates.
    let updates: Vec<Vec<&String>> = jobs
        .iter()
        .map(|j| j.steps.iter().flat_map(|&s| &read.steps[s].outlets).chain(&j.outlets).collect())
        .chain(read.feeders.iter().map(|f| f.outlets.iter().collect()))
        .collect();
    // Built once and only looked up, never walked, so its order is no answer's
    // (0055); each producer once per asset, in the numbering's order.
    let mut producers: HashMap<&str, Vec<usize>> = HashMap::new();
    for (p, outlets) in updates.iter().enumerate() {
        for o in outlets {
            let list = producers.entry(o.as_str()).or_default();
            if list.last() != Some(&p) {
                list.push(p);
            }
        }
    }
    let waits: Vec<&[String]> = jobs.iter().map(|j| j.assets.as_slice()).chain(read.feeders.iter().map(|f| f.assets.as_slice())).collect();
    let feeds: Vec<Vec<Vec<usize>>> = waits.iter().map(|a| a.iter().map(|a| producers.get(a.as_str()).cloned().unwrap_or_default()).collect()).collect();
    let hidden: Vec<bool> = jobs.iter().map(|j| j.asset_unread).chain(read.feeders.iter().map(|f| f.asset_unread)).collect();
    let after: Vec<Option<usize>> = jobs.iter().map(|j| j.after_job).chain(read.feeders.iter().map(|_| None)).collect();
    // A job no file defines may be on a clock or not.
    let mut on: Vec<Start> = jobs
        .iter()
        .map(|j| if !j.after.is_empty() && j.after_job.is_none() { j.own.max(Start::Unread) } else { j.own })
        .chain(read.feeders.iter().map(|f| f.own))
        .collect();
    // A trigger counts once what pulls it counts; cycles simply never start.
    // An asset is updated by any of its producers, or by something no file
    // read says when it has none; a DAG waits on all of its assets.
    loop {
        let mut changed = false;
        for i in 0..on.len() {
            let by_after = after[i].map_or(Start::No, |a| on[a]);
            let by_assets = feeds[i]
                .iter()
                .map(|p| if p.is_empty() { Start::Unread } else { p.iter().map(|&j| on[j]).max().unwrap_or(Start::No) })
                .chain(hidden[i].then_some(Start::Unread))
                .min()
                .unwrap_or(Start::No);
            let best = on[i].max(by_after).max(by_assets);
            if best > on[i] {
                on[i] = best;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    for (i, job) in jobs.iter_mut().enumerate() {
        job.start = on[i];
        job.assets_unread = job.assets.iter().zip(&feeds[i]).filter(|(_, p)| p.is_empty()).map(|(a, _)| a.clone()).collect();
    }
}

/// Reads every source and resolves every step against the graph.
fn build(graph: &Graph, root: &Path, sources: &[JobSource]) -> (Read, Vec<Stamp>) {
    let mut read = Read::default();
    let mut stamps = Vec::new();
    for (si, src) in sources.iter().enumerate() {
        let base = locate(root, &src.path);
        let mut sr = SourceRead {
            kind: src.kind.clone(),
            path: src.path.clone(),
            label: src.label.clone(),
            keys: src.keys.clone(),
            resolved: base.to_string_lossy().into_owned(),
            ..Default::default()
        };
        match walk(&base) {
            Err(e) => sr.error = e,
            Ok((files, capped)) => {
                sr.capped = capped;
                sr.files = files.len();
                for (path, len, _) in &files {
                    let file = shown_path(&base, path);
                    let (jobs_before, steps_before) = (read.jobs.len(), read.steps.len());
                    let outcome = match read_doc(path, *len) {
                        Err((line, message)) => Err(Refusal { file: file.clone(), line, message }),
                        Ok(doc) if src.kind == DBT_CLOUD => read_cloud(si, &file, &doc, &mut read.jobs, &mut read.steps),
                        Ok(doc) => read_airflow(si, src, &file, &doc, &mut read.jobs, &mut read.steps, &mut read.feeders),
                    };
                    match outcome {
                        Ok(true) => {
                            sr.jobs += read.jobs.len() - jobs_before;
                            sr.steps += read.steps.len() - steps_before;
                        }
                        Ok(false) => sr.skipped += 1,
                        Err(refusal) => {
                            // Whole or not at all: a file refused halfway
                            // leaves none of its jobs behind.
                            read.jobs.truncate(jobs_before);
                            read.steps.truncate(steps_before);
                            sr.refused.push(refusal);
                        }
                    }
                }
                stamps.extend(files);
            }
        }
        read.sources.push(sr);
    }
    resolve_steps(graph, &mut read.steps);
    link(&mut read);
    (read, stamps)
}

fn stamps_of(root: &Path, sources: &[JobSource]) -> Vec<Stamp> {
    sources.iter().flat_map(|s| walk(&locate(root, &s.path)).map(|(f, _)| f).unwrap_or_default()).collect()
}

struct Held {
    /// Weak, so a reloaded manifest is not kept alive by this cache, and its
    /// address is not reused while the comparison below still needs it.
    graph: Weak<Graph>,
    sources: Vec<JobSource>,
    stamps: Vec<Stamp>,
    read: Arc<Read>,
}

/// The last read, kept until the manifest is reloaded, the sources change, or
/// a look at the files finds one added, removed or changed. Nothing looks on
/// its own: the page asks to when the Jobs tab is opened or the sources form
/// is shown or saved, never on a click inside the tab, since a job repository
/// moves on a pull, not between two clicks.
#[derive(Default)]
pub struct Cache(Mutex<Option<Held>>);

impl Cache {
    /// Whether the jobs were read at all since dbt-edith started. The Reports
    /// index counts the coverage only once they were, so it never reads the
    /// job files first; after that, a new manifest reads them again as the
    /// Jobs tab would (0066).
    pub fn read_once(&self) -> bool {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).is_some()
    }

    pub fn get(&self, graph: &Arc<Graph>, root: &Path, sources: &[JobSource], look: bool) -> Arc<Read> {
        let mut slot = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(h) = slot.as_ref() {
            if Weak::ptr_eq(&h.graph, &Arc::downgrade(graph))
                && h.sources == sources
                && (!look || stamps_of(root, sources) == h.stamps)
            {
                return h.read.clone();
            }
        }
        let (read, stamps) = build(graph, root, sources);
        let read = Arc::new(read);
        *slot = Some(Held { graph: Arc::downgrade(graph), sources: sources.to_vec(), stamps, read: read.clone() });
        read
    }
}

/* -------------------------------------------------------------- views -- */

#[derive(serde::Serialize, Debug)]
pub struct JobView {
    pub source: String,
    pub kind: &'static str,
    pub file: String,
    pub line: usize,
    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub ident: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub cron: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub assets: Vec<String>,
    /// The job it runs after, by name, or by id when no file read defines it.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub after: String,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub after_unread: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub ci: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub merge: bool,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub note: String,
    pub scheduled: bool,
    /// Whether anything starts it hangs on what no file read says (0077).
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub schedule_unread: bool,
    /// Its assets that no file read updates.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub assets_unread: Vec<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub env: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub target: String,
    /// `dag › task` this DAG's sensors wait for.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub waits_for: Vec<String>,
    /// Jobs started by this one finishing.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub triggers: Vec<String>,
    pub steps: usize,
}

#[derive(serde::Serialize, Debug)]
pub struct StepView {
    /// Across every source, for asking about this step alone.
    pub index: usize,
    pub n: usize,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub name: String,
    pub line: usize,
    pub shown: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub verb: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub ls: String,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub implicit: bool,
    /// The terms of its selection that reach the node asked about.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub via: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub upstream: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub downstream: Vec<String>,
    /// `dag › sensor` waiting on this task.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub waited_by: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub outlets: Vec<String>,
    /// DAGs scheduled on an asset this task updates.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub feeds: Vec<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub unresolved: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
    pub selected: usize,
}

#[derive(serde::Serialize, Debug)]
pub struct Group {
    pub job: JobView,
    pub steps: Vec<StepView>,
}

#[derive(serde::Serialize, Debug, Default)]
pub struct NodeJobs {
    pub configured: bool,
    pub kind: String,
    pub groups: Vec<Group>,
    /// Steps that could not be resolved and could act on a node of this kind.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub maybe: Vec<Group>,
    /// Files refused across the sources, which could hold more.
    #[serde(skip_serializing_if = "is_zero")]
    pub refused: usize,
}

fn source_name(read: &Read, si: usize) -> String {
    let s = &read.sources[si];
    if s.label.is_empty() {
        s.path.clone()
    } else {
        s.label.clone()
    }
}

fn job_view(read: &Read, ji: usize) -> JobView {
    let j = &read.jobs[ji];
    let (after, after_unread) = match (j.after.is_empty(), j.after_job) {
        (true, _) => (String::new(), false),
        (false, Some(a)) => (read.jobs[a].name.clone(), false),
        (false, None) => (j.after.clone(), true),
    };
    JobView {
        source: source_name(read, j.source),
        kind: j.kind,
        file: j.file.clone(),
        line: j.line,
        name: j.name.clone(),
        ident: if j.kind == DBT_CLOUD { j.ident.clone() } else { String::new() },
        cron: j.cron.clone(),
        assets: j.assets.clone(),
        after,
        after_unread,
        ci: j.ci,
        merge: j.merge,
        note: j.note.clone(),
        scheduled: j.start == Start::Yes,
        schedule_unread: j.start == Start::Unread,
        assets_unread: j.assets_unread.clone(),
        env: j.env.clone(),
        target: j.target.clone(),
        waits_for: j.sensors.iter().map(|s| if s.task.is_empty() { s.dag.clone() } else { format!("{} › {}", s.dag, s.task) }).collect(),
        triggers: read.jobs.iter().filter(|o| o.after_job == Some(ji)).map(|o| o.name.clone()).collect(),
        steps: j.steps.len(),
    }
}

fn step_view(graph: &Graph, read: &Read, si: usize, node: Option<u32>) -> StepView {
    let s = &read.steps[si];
    let job = &read.jobs[s.job];
    let mut view = StepView {
        index: si,
        n: s.n,
        name: s.name.clone(),
        line: s.line,
        shown: s.shown.clone(),
        verb: s.verb().map(|v| v.name().to_string()).unwrap_or_default(),
        ls: s.inv.as_ref().and_then(Invocation::ls).unwrap_or_default(),
        implicit: s.implicit,
        via: Vec::new(),
        upstream: s.upstream.clone(),
        downstream: Vec::new(),
        waited_by: Vec::new(),
        outlets: s.outlets.clone(),
        feeds: Vec::new(),
        unresolved: s.unresolved.clone(),
        warnings: s.warnings.clone(),
        selected: s.nodes.len(),
    };
    if job.kind == AIRFLOW && !s.name.is_empty() {
        view.downstream = job
            .steps
            .iter()
            .map(|&o| &read.steps[o])
            .filter(|o| o.upstream.iter().any(|u| *u == s.name))
            .map(|o| o.name.clone())
            .collect();
        for other in read.jobs.iter().filter(|o| o.kind == AIRFLOW) {
            for sensor in &other.sensors {
                if sensor.dag == job.name && (sensor.task.is_empty() || sensor.task == s.name) {
                    view.waited_by.push(format!("{} › {}", other.name, sensor.id));
                }
            }
        }
        view.feeds = read
            .jobs
            .iter()
            .filter(|o| o.assets.iter().any(|a| s.outlets.contains(a)))
            .map(|o| o.name.clone())
            .collect();
    }
    if let (Some(node), Some(inv)) = (node, &s.inv) {
        view.via = via(graph, inv, node);
    }
    view
}

/// The terms of a step's selection that reach `node` on their own.
fn via(graph: &Graph, inv: &Invocation, node: u32) -> Vec<String> {
    if let Some(name) = &inv.selector {
        return vec![format!("--selector {name}")];
    }
    if inv.select.is_empty() {
        let default = graph.selectors.list().iter().find(|n| n.default).map(|n| format!("--selector {} (default)", n.name));
        return vec![default.unwrap_or_else(|| "no --select: everything".into())];
    }
    inv.select
        .iter()
        .filter(|t| {
            let Ok(expr) = Expr::from_terms(std::slice::from_ref(*t), &[]) else { return false };
            let spec = expr.spec_with(inv.indirect.unwrap_or(Indirect::Eager));
            select::resolve_spec(graph, &spec, Tests::Eager, &no_selectors).is_ok_and(|r| r.nodes.binary_search(&node).is_ok())
        })
        .cloned()
        .collect()
}

fn grouped(graph: &Graph, read: &Read, hits: &[usize], node: Option<u32>) -> Vec<Group> {
    let mut groups: Vec<Group> = Vec::new();
    let mut last: Option<usize> = None;
    for &si in hits {
        let ji = read.steps[si].job;
        if last != Some(ji) {
            groups.push(Group { job: job_view(read, ji), steps: Vec::new() });
            last = Some(ji);
        }
        if let Some(g) = groups.last_mut() {
            g.steps.push(step_view(graph, read, si, node));
        }
    }
    groups
}

/// Every step that runs, builds, tests or checks one node, grouped by job. A
/// `dbt compile` selects every model without running one, so it would put its
/// job under every node; the steps that only compile, clone, list or preview
/// are left out, as coverage leaves them out.
pub fn for_node(graph: &Graph, read: &Read, node: u32) -> NodeJobs {
    let kind = graph.nodes[node as usize].kind;
    let hits: Vec<usize> =
        (0..read.steps.len()).filter(|&i| read.steps[i].executes() && read.steps[i].nodes.binary_search(&node).is_ok()).collect();
    let maybe: Vec<usize> = (0..read.steps.len())
        .filter(|&i| {
            let s = &read.steps[i];
            !s.unresolved.is_empty()
                && s.inv.as_ref().is_none_or(|inv| inv.verb.executes() && inv.kinds().is_some_and(|k| k.contains(&kind)))
        })
        .collect();
    NodeJobs {
        configured: !read.sources.is_empty(),
        kind: kind.as_str().to_string(),
        groups: grouped(graph, read, &hits, Some(node)),
        maybe: grouped(graph, read, &maybe, None),
        refused: read.sources.iter().map(|s| s.refused.len()).sum(),
    }
}

/// One step, and every name it selects: what `dbt ls` would print for it,
/// for comparing the two (0033).
#[derive(serde::Serialize, Debug)]
pub struct StepNames {
    pub shown: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub ls: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub unresolved: String,
    pub ids: Vec<String>,
    /// As `dbt ls --output name` prints them: a source as `source.table`.
    pub names: Vec<String>,
}

pub fn step_names(graph: &Graph, read: &Read, index: usize) -> Option<StepNames> {
    let s = read.steps.get(index)?;
    let pick = |f: fn(&crate::graph::Node) -> &String| {
        let mut out: Vec<String> = s.nodes.iter().map(|&i| f(&graph.nodes[i as usize]).clone()).collect();
        out.sort();
        out
    };
    Some(StepNames {
        shown: s.shown.clone(),
        ls: s.inv.as_ref().and_then(Invocation::ls).unwrap_or_default(),
        unresolved: s.unresolved.clone(),
        ids: pick(|n| &n.id),
        names: pick(|n| &n.name),
    })
}

/* ------------------------------------------------------------ coverage -- */

#[derive(serde::Serialize, Debug, Clone)]
pub struct Row {
    pub id: String,
    pub name: String,
    pub kind: &'static str,
    pub folder: String,
}

#[derive(serde::Serialize, Debug)]
pub struct StepRef {
    pub index: usize,
    pub source: String,
    pub job: String,
    pub n: usize,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub name: String,
    pub file: String,
    pub line: usize,
    pub shown: String,
    pub why: String,
}

#[derive(serde::Serialize, Debug, Default)]
pub struct Coverage {
    pub configured: bool,
    /// Models, seeds and snapshots that can run, ephemeral models aside.
    pub total: usize,
    /// Run by no step at all.
    pub never: Vec<Row>,
    /// Run only by jobs nothing schedules.
    pub unscheduled: Vec<Row>,
    /// Run by no scheduled job, but by one whose start hangs on what no file
    /// read says (0077).
    pub unread: Vec<Row>,
    /// Never run on their own, whatever selects them.
    pub ephemeral: Vec<Row>,
    /// Steps that select nothing they act on.
    pub empty: Vec<StepRef>,
    pub unresolved: Vec<StepRef>,
    pub jobs: Vec<JobView>,
}

fn folder(graph: &Graph, i: u32) -> String {
    let n = &graph.nodes[i as usize];
    let dir = n.file.rsplit_once('/').map_or("", |(d, _)| d);
    if !graph.meta.project.is_empty() && n.package != graph.meta.project {
        format!("{}/{dir}", n.package)
    } else {
        dir.to_string()
    }
}

fn step_ref(read: &Read, si: usize, why: String) -> StepRef {
    let s = &read.steps[si];
    let j = &read.jobs[s.job];
    StepRef {
        index: si,
        source: source_name(read, j.source),
        job: j.name.clone(),
        n: s.n,
        name: s.name.clone(),
        file: j.file.clone(),
        line: s.line,
        shown: s.shown.clone(),
        why,
    }
}

/// Which of the graph's nodes some step runs, builds, tests or checks, by
/// index: the guided tour shows one of them when there is one (0074).
pub fn run_by_a_step(graph: &Graph, read: &Read) -> Vec<bool> {
    let mut ran = vec![false; graph.nodes.len()];
    for s in read.steps.iter().filter(|s| s.executes()) {
        for &i in &s.nodes {
            ran[i as usize] = true;
        }
    }
    ran
}

pub fn coverage(graph: &Graph, read: &Read) -> Coverage {
    let n = graph.nodes.len();
    let (mut any, mut on) = (vec![false; n], vec![Start::No; n]);
    for s in read.steps.iter().filter(|s| s.executes()) {
        let start = read.jobs[s.job].start;
        for &i in &s.nodes {
            any[i as usize] = true;
            on[i as usize] = on[i as usize].max(start);
        }
    }
    let mut cov = Coverage { configured: !read.sources.is_empty(), ..Default::default() };
    // An installed package's models are the package's to schedule or not: a
    // project runs the ones it needs, and the rest would bury its own.
    let root = graph.macros.root();
    for (i, node) in graph.nodes.iter().enumerate() {
        if node.disabled || !matches!(node.kind, Kind::Model | Kind::Seed | Kind::Snapshot) {
            continue;
        }
        if !root.is_empty() && node.package != root {
            continue;
        }
        let row = Row { id: node.id.clone(), name: node.name.clone(), kind: node.kind.as_str(), folder: folder(graph, i as u32) };
        if node.kind == Kind::Model && node.materialized == "ephemeral" {
            cov.ephemeral.push(row);
            continue;
        }
        cov.total += 1;
        if !any[i] {
            cov.never.push(row);
        } else if on[i] == Start::No {
            cov.unscheduled.push(row);
        } else if on[i] == Start::Unread {
            cov.unread.push(row);
        }
    }
    let order = |a: &Row, b: &Row| a.folder.cmp(&b.folder).then_with(|| a.name.cmp(&b.name)).then_with(|| a.id.cmp(&b.id));
    cov.never.sort_by(order);
    cov.unscheduled.sort_by(order);
    cov.unread.sort_by(order);
    cov.ephemeral.sort_by(order);
    for (si, s) in read.steps.iter().enumerate() {
        if !s.unresolved.is_empty() {
            cov.unresolved.push(step_ref(read, si, s.unresolved.clone()));
        } else if s.inv.as_ref().is_some_and(|i| i.kinds().is_some()) && s.nodes.is_empty() {
            let why = if s.warnings.is_empty() { "selects nothing it acts on".to_string() } else { s.warnings.join("; ") };
            cov.empty.push(step_ref(read, si, why));
        }
    }
    cov.jobs = (0..read.jobs.len()).map(|j| job_view(read, j)).collect();
    cov
}

/// What the settings form shows once the sources are read.
#[derive(serde::Serialize, Debug, Default)]
pub struct Summary {
    pub sources: Vec<SourceRead>,
    pub jobs: usize,
    pub steps: usize,
    pub unresolved: usize,
}

pub fn summary(read: &Read) -> Summary {
    Summary {
        sources: read.sources.clone(),
        jobs: read.jobs.len(),
        steps: read.steps.len(),
        unresolved: read.steps.iter().filter(|s| !s.unresolved.is_empty()).count(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::RawManifest;
    use serde_json::json;

    /// A shop: a source and a seed under two staging models, two marts, an
    /// ephemeral model nobody runs, a test, a disabled model, and two named
    /// selectors. Invented, like every fixture here (0014).
    fn graph_with(selectors: serde_json::Value) -> Graph {
        let model = |name: &str, dir: &str, tags: &[&str], materialized: &str| {
            json!({
                "name": name, "resource_type": "model", "package_name": "shop",
                "original_file_path": format!("models/{dir}/{name}.sql"),
                "fqn": ["shop", dir, name], "tags": tags,
                "config": { "materialized": materialized },
            })
        };
        let raw: RawManifest = serde_json::from_value(json!({
            "metadata": { "project_name": "shop" },
            "nodes": {
                "model.shop.stg_customers": model("stg_customers", "staging", &["staging"], "view"),
                "model.shop.stg_orders": model("stg_orders", "staging", &["staging"], "view"),
                "model.shop.customers": model("customers", "marts", &["daily"], "table"),
                "model.shop.orders": model("orders", "marts", &["daily"], "table"),
                "model.shop.int_pivot": model("int_pivot", "marts", &[], "ephemeral"),
                "model.shop.test": model("test", "marts", &[], "table"),
                // An installed package's, which coverage leaves to its package.
                "model.artifacts.runs": {
                    "name": "runs", "resource_type": "model", "package_name": "artifacts",
                    "original_file_path": "models/runs.sql", "fqn": ["artifacts", "runs"],
                    "config": { "materialized": "table" },
                },
                "seed.shop.raw_orders": {
                    "name": "raw_orders", "resource_type": "seed", "package_name": "shop",
                    "original_file_path": "seeds/raw_orders.csv", "fqn": ["shop", "raw_orders"],
                },
                "test.shop.not_null_orders_id": {
                    "name": "not_null_orders_id", "resource_type": "test", "package_name": "shop",
                    "original_file_path": "models/marts/schema.yml",
                    "fqn": ["shop", "marts", "not_null_orders_id"], "column_name": "id",
                    "attached_node": "model.shop.orders", "test_metadata": { "name": "not_null" },
                },
            },
            "sources": {
                "source.shop.crm.customers": {
                    "name": "customers", "resource_type": "source", "package_name": "shop",
                    "source_name": "crm", "original_file_path": "models/staging/sources.yml",
                    "fqn": ["shop", "staging", "crm", "customers"],
                },
            },
            "disabled": {
                "model.shop.legacy": [{
                    "name": "legacy", "resource_type": "model", "package_name": "shop",
                    "original_file_path": "models/marts/legacy.sql", "fqn": ["shop", "marts", "legacy"],
                    "config": { "enabled": false },
                }],
            },
            "parent_map": {
                "model.shop.stg_customers": ["source.shop.crm.customers"],
                "model.shop.stg_orders": ["seed.shop.raw_orders"],
                "model.shop.customers": ["model.shop.stg_customers", "model.shop.int_pivot"],
                "model.shop.int_pivot": ["model.shop.stg_orders"],
                "model.shop.orders": ["model.shop.stg_orders"],
                "test.shop.not_null_orders_id": ["model.shop.orders"],
            },
            "selectors": selectors,
        }))
        .unwrap();
        Graph::build(raw, Path::new("manifest.json"), 0, 0)
    }

    fn graph() -> Graph {
        graph_with(json!({
            "marts": { "name": "marts", "definition": { "method": "path", "value": "models/marts" } },
            "changed": { "name": "changed", "definition": { "method": "state", "value": "modified" } },
        }))
    }

    /// A folder of its own under the system's temporary one, emptied first.
    fn dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("dbt-edith-jobs-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn write(dir: &Path, rel: &str, text: &str) {
        let p = dir.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }

    fn source(kind: &str, path: &Path, keys: &[(&str, &str)]) -> JobSource {
        JobSource {
            kind: kind.into(),
            path: path.to_string_lossy().into_owned(),
            label: String::new(),
            keys: keys.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
        }
    }

    fn names_of(g: &Graph, nodes: &[u32]) -> Vec<String> {
        let mut out: Vec<String> = nodes.iter().map(|&i| g.nodes[i as usize].name.clone()).collect();
        out.sort();
        out
    }

    const DAG: &str = "dag_id: shop_daily
schedule: \"0 5 * * *\"
tasks:
  - task_id: stage
    command: dbt run --select tag:staging
  - task_id: marts
    command: run -s +tag:daily --exclude stg_customers
    depends_on: [stage]
    outlets: [marts_ready]
  - task_id: notify
    python: send_mail
";

    const RENAMED: &str = "pipeline:
  name: shop_reports
  cron: null
steps:
  - id: reports
    dbt: >-
      run --full-refresh -s
      customers
  - id: check
    dbt: test --selector marts
waits:
  - id: wait_daily
    upstream_dag: shop_daily
    upstream_task: marts
";

    const ON_ASSET: &str = "dag_id: shop_after
schedule: [marts_ready]
tasks:
  - task_id: seed
    command: seed
";

    #[test]
    fn reads_a_dag_in_the_proposed_shape_and_resolves_its_steps() {
        let d = dir("airflow");
        write(&d, "dags/shop_daily.yml", DAG);
        write(&d, "dags/notes.yml", "owner: someone\n");
        let g = graph();
        let (read, stamps) = build(&g, &d, &[source(AIRFLOW, &d, &[])]);
        assert_eq!(stamps.len(), 2);
        let s = &read.sources[0];
        assert_eq!((s.files, s.jobs, s.steps, s.skipped), (2, 1, 2, 1));
        let job = &read.jobs[0];
        assert_eq!((job.name.as_str(), job.cron.as_str(), job.file.as_str(), job.line), ("shop_daily", "0 5 * * *", "dags/shop_daily.yml", 1));
        assert_eq!(job.start, Start::Yes);
        let (stage, marts) = (&read.steps[0], &read.steps[1]);
        assert_eq!(names_of(&g, &stage.nodes), ["stg_customers", "stg_orders"]);
        // `+tag:daily` reaches the ephemeral model and the staging models;
        // `run` keeps models; the exclude drops one; no source, seed or test.
        assert_eq!(names_of(&g, &marts.nodes), ["customers", "int_pivot", "orders", "stg_orders"]);
        assert_eq!(marts.upstream, ["stage"]);
        assert_eq!(marts.shown, "dbt run -s +tag:daily --exclude stg_customers");
    }

    #[test]
    fn reads_renamed_keys_folded_commands_and_named_selectors() {
        let d = dir("renamed");
        write(&d, "a/shop_daily.yml", DAG);
        write(&d, "b/reports.yaml", RENAMED);
        let keys = [
            ("dag_id", "pipeline.name"),
            ("schedule", "pipeline.cron"),
            ("tasks", "steps"),
            ("task_id", "id"),
            ("command", "dbt"),
            ("sensors", "waits"),
            ("sensor_id", "id"),
            ("external_dag_id", "upstream_dag"),
            ("external_task_id", "upstream_task"),
        ];
        let g = graph();
        let (read, _) = build(&g, &d, &[source(AIRFLOW, &d.join("b"), &keys), source(AIRFLOW, &d.join("a"), &[])]);
        let reports = &read.jobs[0];
        assert_eq!(reports.name, "shop_reports");
        assert_eq!(reports.start, Start::No, "a null schedule is a manual DAG");
        assert_eq!(names_of(&g, &read.steps[0].nodes), ["customers"]);
        // The selector holds models/marts; `test` keeps its tests.
        assert_eq!(names_of(&g, &read.steps[1].nodes), ["not_null_orders_id"]);
        let view = job_view(&read, 0);
        assert_eq!(view.waits_for, ["shop_daily › marts"]);
        let marts = read.steps.iter().position(|s| s.name == "marts").unwrap();
        assert_eq!(step_view(&g, &read, marts, None).waited_by, ["shop_reports › wait_daily"]);
    }

    #[test]
    fn a_dag_on_an_asset_counts_as_scheduled_when_its_producer_does() {
        let d = dir("assets");
        write(&d, "daily.yml", DAG);
        write(&d, "after.yml", ON_ASSET);
        let g = graph();
        let (read, _) = build(&g, &d, &[source(AIRFLOW, &d, &[])]);
        let after = read.jobs.iter().find(|j| j.name == "shop_after").unwrap();
        assert_eq!(after.assets, ["marts_ready"]);
        assert_eq!(after.start, Start::Yes);
        let marts = read.steps.iter().position(|s| s.name == "marts").unwrap();
        assert_eq!(step_view(&g, &read, marts, None).feeds, ["shop_after"]);
        // Without the producer's clock, nothing pulls it.
        write(&d, "daily.yml", &DAG.replace("\"0 5 * * *\"", "null"));
        let (read, _) = build(&g, &d, &[source(AIRFLOW, &d, &[])]);
        assert!(read.jobs.iter().all(|j| j.start == Start::No));
    }

    /// A DAG running one model, on the schedule given as written in YAML.
    fn one_dag(id: &str, schedule: &str, select: &str) -> String {
        format!("dag_id: {id}\nschedule: {schedule}\ntasks:\n  - task_id: t\n    command: dbt run --select {select}\n")
    }

    fn start_of(read: &Read, name: &str) -> Start {
        read.jobs.iter().find(|j| j.name == name).unwrap_or_else(|| panic!("no job {name}")).start
    }

    #[test]
    fn an_asset_is_updated_by_any_task_read_dbt_or_not() {
        let d = dir("feeders");
        // An ingestion DAG runs no dbt, yet starts the DAG waiting on what it loads.
        write(&d, "ingest.yml", "dag_id: ingest\nschedule: \"0 4 * * *\"\ntasks:\n  - task_id: load\n    outlets: [raw_ready]\n");
        write(&d, "on_raw.yml", &one_dag("on_raw", "[raw_ready]", "stg_orders"));
        // A task with no command in a dbt DAG updates an asset as well.
        write(&d, "daily.yml", &format!("{DAG}  - task_id: publish\n    outlets: [published]\n"));
        write(&d, "on_published.yml", &one_dag("on_published", "[published]", "orders"));
        // A feeder waits on an asset a dbt DAG updates, and passes the clock on.
        write(&d, "copy.yml", "dag_id: copy\nschedule: [marts_ready]\ntasks:\n  load:\n    outlets: [copied]\n");
        write(&d, "on_copied.yml", &one_dag("on_copied", "[copied]", "customers"));
        // Nothing read updates `elsewhere`, which an API call may.
        write(&d, "on_elsewhere.yml", &one_dag("on_elsewhere", "[elsewhere]", "stg_customers"));
        write(&d, "on_both.yml", &one_dag("on_both", "[raw_ready, elsewhere]", "stg_customers"));
        // No DAG id: other YAML, neither a feeder nor refused.
        write(&d, "notes.yml", "tasks:\n  - outlets: [elsewhere]\n");
        let g = graph();
        let (read, _) = build(&g, &d, &[source(AIRFLOW, &d, &[])]);
        assert_eq!(read.sources[0].refused, []);
        assert_eq!(read.feeders.len(), 2, "ingest and copy");
        assert_eq!(read.sources[0].skipped, 3, "the feeders and the notes hold no job");
        // A feeder is listed nowhere: not among the jobs read, nor the coverage's.
        assert_eq!(summary(&read).jobs, 6);
        let listed: Vec<String> = coverage(&g, &read).jobs.into_iter().map(|j| j.name).collect();
        assert_eq!(listed.len(), 6);
        assert!(!listed.iter().any(|n| n == "ingest" || n == "copy"), "{listed:?}");
        assert_eq!(start_of(&read, "on_raw"), Start::Yes);
        assert_eq!(start_of(&read, "on_published"), Start::Yes);
        assert_eq!(start_of(&read, "on_copied"), Start::Yes);
        assert_eq!(start_of(&read, "on_elsewhere"), Start::Unread);
        assert_eq!(start_of(&read, "on_both"), Start::Unread, "it waits on both");
        let both = read.jobs.iter().position(|j| j.name == "on_both").unwrap();
        assert_eq!(job_view(&read, both).assets_unread, ["elsewhere"]);
        assert!(job_view(&read, both).schedule_unread && !job_view(&read, both).scheduled);
        // Without the ingestion's clock, nothing pulls what waits on it.
        write(&d, "ingest.yml", "dag_id: ingest\nschedule: null\ntasks:\n  - task_id: load\n    outlets: [raw_ready]\n");
        write(&d, "daily.yml", &DAG.replace("\"0 5 * * *\"", "\"{{ var.value.cron }}\""));
        let (read, _) = build(&g, &d, &[source(AIRFLOW, &d, &[])]);
        assert_eq!(start_of(&read, "on_raw"), Start::No);
        assert_eq!(start_of(&read, "on_both"), Start::No, "raw_ready never comes");
        assert_eq!(start_of(&read, "shop_daily"), Start::Unread);
        assert_eq!(start_of(&read, "on_copied"), Start::Unread, "through a clock not read");
        assert_eq!(start_of(&read, "on_published"), Start::Unread, "daily.yml lost its publish task");
    }

    #[test]
    fn a_clock_no_file_says_counts_neither_way() {
        let d = dir("unread");
        write(&d, "air/templated.yml", &one_dag("templated", "\"{{ var.value.cron }}\"", "stg_orders"));
        write(&d, "air/mapping.yml", &one_dag("mapping", "{ cron: \"0 1 * * *\", timezone: UTC }", "stg_customers"));
        write(&d, "air/once.yml", &one_dag("once", "\"@once\"", "orders"));
        write(
            &d,
            "cloud/jobs.yml",
            "jobs:
  hidden_cron:
    name: Hidden cron
    execute_steps: [dbt seed]
    schedule: { cron: \"{{ cron }}\" }
    triggers: { schedule: true }
  no_cron:
    name: No cron
    execute_steps: [dbt run -s customers]
    triggers: { schedule: true }
  switched:
    name: Switched
    execute_steps: [dbt run -s orders]
    schedule: { cron: \"0 6 * * *\" }
    triggers: { schedule: \"{{ nightly }}\" }
  after_unknown:
    name: After unknown
    execute_steps: [dbt run -s stg_orders]
    triggers: { schedule: false }
    job_completion_trigger_condition: { condition: { job_id: 999 } }
  after_templated:
    name: After templated
    execute_steps: [dbt run -s stg_orders]
    job_completion_trigger_condition: { condition: { job_id: \"{{ upstream }}\" } }
  off:
    name: Off
    execute_steps: [dbt run -s stg_customers]
    schedule: { cron: \"0 6 * * *\" }
    triggers: { schedule: false }
",
        );
        let g = graph();
        let (read, _) = build(&g, &d, &[source(AIRFLOW, &d.join("air"), &[]), source(DBT_CLOUD, &d.join("cloud"), &[])]);
        assert_eq!(start_of(&read, "templated"), Start::Unread);
        assert_eq!(start_of(&read, "mapping"), Start::Unread);
        assert_eq!(start_of(&read, "once"), Start::No);
        // dbt Cloud runs a job on a clock when its trigger says so, whether
        // or not this tool can read when.
        let hidden = read.jobs.iter().find(|j| j.name == "Hidden cron").unwrap();
        assert_eq!((hidden.start, hidden.cron.as_str(), hidden.note.as_str()), (Start::Yes, "", "a schedule only the vars file knows"));
        assert_eq!(start_of(&read, "No cron"), Start::Yes);
        let switched = read.jobs.iter().find(|j| j.name == "Switched").unwrap();
        assert_eq!((switched.start, switched.cron.as_str()), (Start::Unread, "0 6 * * *"));
        assert_eq!(start_of(&read, "After unknown"), Start::Unread);
        assert_eq!(start_of(&read, "After templated"), Start::Unread);
        assert_eq!(start_of(&read, "Off"), Start::No);
    }

    #[test]
    fn an_asset_this_tool_cannot_name_is_waited_on_all_the_same() {
        let d = dir("asset-mapping");
        write(&d, "ingest.yml", "dag_id: ingest\nschedule: \"0 4 * * *\"\ntasks:\n  - task_id: load\n    outlets: [raw_ready]\n");
        write(&d, "by_hand.yml", "dag_id: by_hand\nschedule: null\ntasks:\n  - task_id: load\n    outlets: [typed_in]\n");
        // dag-factory writes an asset as a mapping, by its uri.
        write(&d, "on_uri.yml", &one_dag("on_uri", "[{ __type__: airflow.sdk.Asset, uri: \"s3://bucket/raw\" }]", "stg_orders"));
        write(&d, "on_mixed.yml", &one_dag("on_mixed", "[raw_ready, { uri: \"s3://bucket/raw\" }]", "stg_customers"));
        write(&d, "on_mixed_by_hand.yml", &one_dag("on_mixed_by_hand", "[typed_in, { uri: \"s3://bucket/raw\" }]", "customers"));
        write(&d, "on_nothing.yml", &one_dag("on_nothing", "[]", "orders"));
        // A feeder on such an asset passes on what it knows.
        write(&d, "relay.yml", "dag_id: relay\nschedule: [{ uri: \"s3://bucket/raw\" }]\ntasks:\n  - task_id: copy\n    outlets: [relayed]\n");
        write(&d, "on_relayed.yml", &one_dag("on_relayed", "[relayed]", "orders"));
        // Jinja's statements are as unknown as its expressions.
        write(&d, "statement.yml", &one_dag("statement", "\"{% if prod %}0 5 * * *{% endif %}\"", "orders"));
        let g = graph();
        let (read, _) = build(&g, &d, &[source(AIRFLOW, &d, &[])]);
        assert_eq!(read.sources[0].refused, []);
        assert_eq!(start_of(&read, "on_uri"), Start::Unread);
        let mixed = read.jobs.iter().find(|j| j.name == "on_mixed").unwrap();
        assert_eq!(mixed.assets, ["raw_ready"]);
        assert_eq!((mixed.start, mixed.note.as_str()), (Start::Unread, "an asset written as a mapping, which this tool does not read"));
        assert_eq!(start_of(&read, "on_mixed_by_hand"), Start::No, "typed_in never comes on its own");
        assert_eq!(start_of(&read, "on_nothing"), Start::Unread);
        assert_eq!(start_of(&read, "on_relayed"), Start::Unread);
        assert_eq!(start_of(&read, "statement"), Start::Unread);
        let cloud = dir("cloud-statement");
        write(
            &cloud,
            "jobs.yml",
            "jobs:\n  hidden:\n    name: Hidden\n    execute_steps: [dbt seed]\n    schedule: { cron: \"{% if prod %}0 6 * * *{% endif %}\" }\n    triggers: { schedule: \"{% if prod %}true{% endif %}\" }\n",
        );
        let (read, _) = build(&g, &cloud, &[source(DBT_CLOUD, &cloud, &[])]);
        let hidden = &read.jobs[0];
        assert_eq!((hidden.start, hidden.cron.as_str()), (Start::Unread, ""));
    }

    /// A dbt Cloud job alone in its file, started when the job `after` ends.
    fn cloud_after(ident: &str, id: u32, after: u32) -> String {
        format!(
            "jobs:\n  {ident}:\n    name: {ident}\n    id: {id}\n    execute_steps: [dbt run -s stg_orders]\n    triggers: {{ schedule: false }}\n    job_completion_trigger_condition: {{ condition: {{ job_id: {after} }} }}\n"
        )
    }

    #[test]
    fn answers_hold_in_any_order_and_a_cycle_never_starts() {
        let files = [
            ("air", "dag_id: ingest\nschedule: \"0 4 * * *\"\ntasks:\n  - task_id: load\n    outlets: [raw_ready]\n".to_string()),
            ("air", one_dag("stage", "[raw_ready]", "stg_orders") + "  - task_id: done\n    outlets: [staged]\n"),
            ("air", "dag_id: copy\nschedule: [staged]\ntasks:\n  load:\n    outlets: [copied]\n".to_string()),
            ("air", one_dag("on_copied", "[copied, elsewhere]", "orders")),
            // A cycle through a feeder, one of its assets updated by no file read.
            ("air", one_dag("loop", "[looped, elsewhere]", "customers") + "  - task_id: done\n    outlets: [from_loop]\n"),
            ("air", "dag_id: loop_feed\nschedule: [from_loop]\ntasks:\n  - task_id: load\n    outlets: [looped]\n".to_string()),
            // A chain behind a job no file defines.
            ("cloud", cloud_after("first", 201, 999)),
            ("cloud", cloud_after("second", 202, 201)),
            ("cloud", cloud_after("third", 203, 202)),
        ];
        let g = graph();
        let answers = |name: &str, reverse: bool| {
            let d = dir(name);
            for (i, (sub, text)) in files.iter().enumerate() {
                let n = if reverse { files.len() - i } else { i };
                write(&d, &format!("{sub}/{n:02}.yml"), text);
            }
            let (read, _) = build(&g, &d, &[source(AIRFLOW, &d.join("air"), &[]), source(DBT_CLOUD, &d.join("cloud"), &[])]);
            let mut views: Vec<serde_json::Value> = (0..read.jobs.len())
                .map(|i| {
                    // Only where it was read from differs: the folder and the file.
                    let mut v = serde_json::to_value(job_view(&read, i)).unwrap();
                    let o = v.as_object_mut().unwrap();
                    o.remove("source");
                    o.remove("file");
                    v
                })
                .collect();
            views.sort_by_key(|v| v["name"].as_str().unwrap_or_default().to_string());
            let cov = coverage(&g, &read);
            let names = |rows: &[Row]| rows.iter().map(|r| r.name.clone()).collect::<Vec<_>>();
            let starts: Vec<(String, Start)> = read.jobs.iter().map(|j| (j.name.clone(), j.start)).collect();
            (views, [names(&cov.never), names(&cov.unscheduled), names(&cov.unread)], starts)
        };
        let (views, lists, starts) = answers("order-a", false);
        let (views_b, lists_b, _) = answers("order-b", true);
        assert_eq!(views, views_b);
        assert_eq!(lists, lists_b);
        let start = |n: &str| starts.iter().find(|(name, _)| name == n).unwrap().1;
        assert_eq!(start("stage"), Start::Yes);
        assert_eq!(start("on_copied"), Start::Unread);
        assert_eq!(start("loop"), Start::No, "a cycle never starts, whatever else it waits on");
        assert_eq!([start("first"), start("second"), start("third")], [Start::Unread; 3]);
    }

    #[test]
    fn coverage_keeps_a_schedule_not_read_apart() {
        let d = dir("coverage-unread");
        write(&d, "daily.yml", &one_dag("daily", "\"0 5 * * *\"", "orders"));
        write(&d, "templated.yml", &one_dag("templated", "\"{{ var.value.cron }}\"", "orders stg_orders stg_customers"));
        write(&d, "manual.yml", &one_dag("manual", "null", "stg_customers customers"));
        let g = graph();
        let (read, _) = build(&g, &d, &[source(AIRFLOW, &d, &[])]);
        let cov = coverage(&g, &read);
        let names = |rows: &[Row]| rows.iter().map(|r| r.name.clone()).collect::<Vec<_>>();
        // orders has a scheduled job; stg_customers may have one.
        assert_eq!(names(&cov.unread), ["stg_customers", "stg_orders"]);
        assert_eq!(names(&cov.unscheduled), ["customers"]);
        assert_eq!(names(&cov.never), ["test", "raw_orders"]);
    }

    const CLOUD: &str = "defaults: &defaults
  account_id: 1
  project_id: 2
  environment_id: 3
  settings: { threads: 4, target_name: prod }
  run_generate_sources: false
  generate_docs: false
jobs:
  daily:
    <<: *defaults
    name: Daily build
    linked_id: 101
    run_generate_sources: true
    execute_steps:
      - dbt seed
      - dbt build --select +orders --exclude-resource-type test
    schedule: { cron: \"0 */2 * * *\" }
    triggers: { schedule: true, github_webhook: false }
  exports:
    <<: *defaults
    name: Exports
    execute_steps:
      - dbt run -s customers
    triggers: { schedule: false }
    job_completion_trigger_condition:
      condition: { job_id: 101, project_id: 2, statuses: [10] }
  ci:
    <<: *defaults
    name: CI
    job_type: ci
    execute_steps:
      - dbt build --select state:modified+
    triggers: { github_webhook: true }
  templated:
    <<: *defaults
    name: Templated
    environment_id: {{ env_id }}
    execute_steps:
      - dbt run --select {{ models }}
    triggers: { schedule: false }
";

    #[test]
    fn reads_dbt_cloud_jobs_with_anchors_triggers_and_templates() {
        let d = dir("cloud");
        write(&d, "jobs.yml", CLOUD);
        let g = graph();
        let (read, _) = build(&g, &d.join("jobs.yml"), &[source(DBT_CLOUD, &d.join("jobs.yml"), &[])]);
        assert_eq!(read.sources[0].refused, []);
        assert_eq!(read.jobs.iter().map(|j| j.name.as_str()).collect::<Vec<_>>(), ["Daily build", "Exports", "CI", "Templated"]);
        let daily = &read.jobs[0];
        assert_eq!((daily.cron.as_str(), daily.env.as_str(), daily.target.as_str()), ("0 */2 * * *", "3", "prod"));
        // Freshness first, as dbt Cloud runs it, then the two written steps.
        let steps: Vec<&Step> = daily.steps.iter().map(|&i| &read.steps[i]).collect();
        assert!(steps[0].implicit);
        assert_eq!(names_of(&g, &steps[0].nodes), ["crm.customers"], "the one source");
        assert_eq!(names_of(&g, &steps[1].nodes), ["raw_orders"]);
        assert_eq!(names_of(&g, &steps[2].nodes), ["orders", "raw_orders", "stg_orders"]);
        let exports = &read.jobs[1];
        assert_eq!(exports.start, Start::Yes, "after a scheduled job");
        assert_eq!(job_view(&read, 1).after, "Daily build");
        assert_eq!(job_view(&read, 0).triggers, ["Exports"]);
        let ci = &read.steps[read.jobs[2].steps[0]];
        assert!(ci.unresolved.contains("--state"), "{}", ci.unresolved);
        assert!(read.jobs[2].ci && read.jobs[2].start == Start::No);
        let templated = &read.steps[read.jobs[3].steps[0]];
        assert!(templated.unresolved.contains("only known when the job runs"), "{}", templated.unresolved);
        assert_eq!(read.jobs[3].env, "{{ env_id }}");
    }

    #[test]
    fn answers_for_one_node_with_what_caught_it() {
        let d = dir("node");
        write(&d, "daily.yml", DAG);
        write(&d, "cloud/jobs.yml", CLOUD);
        // Selects orders, and every other model, without running any of them.
        write(
            &d,
            "warm.yml",
            "dag_id: warm\nschedule: \"@daily\"\ntasks:\n  - task_id: c\n    command: dbt compile\n  - task_id: l\n    command: dbt ls -s orders\n  - task_id: s\n    command: dbt compile -s state:modified\n",
        );
        let g = graph();
        let (read, _) = build(&g, &d, &[source(AIRFLOW, &d, &[]), source(DBT_CLOUD, &d.join("cloud"), &[])]);
        let orders = g.index["model.shop.orders"];
        assert!(read.steps.iter().filter(|s| s.nodes.binary_search(&orders).is_ok()).count() > 2);
        let jobs = for_node(&g, &read, orders);
        let names: Vec<&str> = jobs.groups.iter().map(|g| g.job.name.as_str()).collect();
        assert_eq!(names, ["shop_daily", "Daily build"]);
        assert_eq!(jobs.groups[0].steps[0].via, ["+tag:daily"]);
        assert_eq!(jobs.groups[1].steps[0].via, ["+orders"]);
        // The CI step and the templated one could hold any model; an
        // unresolved compile could not run it.
        assert_eq!(jobs.maybe.iter().map(|g| g.job.name.as_str()).collect::<Vec<_>>(), ["CI", "Templated"]);
    }

    #[test]
    fn coverage_separates_never_run_from_run_by_hand() {
        let d = dir("coverage");
        write(&d, "daily.yml", &DAG.replace("tag:staging", "stg_orders"));
        write(&d, "manual.yml", "dag_id: by_hand\nschedule: null\ntasks:\n  - task_id: all\n    command: dbt run --select stg_customers\n");
        let g = graph();
        let (read, _) = build(&g, &d, &[source(AIRFLOW, &d, &[])]);
        let cov = coverage(&g, &read);
        let names = |rows: &[Row]| rows.iter().map(|r| r.name.clone()).collect::<Vec<_>>();
        assert_eq!(names(&cov.never), ["test", "raw_orders"]);
        assert_eq!(names(&cov.unscheduled), ["stg_customers"]);
        assert_eq!(names(&cov.ephemeral), ["int_pivot"]);
        assert_eq!(cov.total, 6, "five models that run and one seed; the disabled model is not one");
        assert_eq!(cov.never[1].folder, "seeds");
    }

    #[test]
    fn a_step_that_selects_nothing_says_why() {
        let d = dir("empty");
        write(&d, "a.yml", "dag_id: a\nschedule: \"@daily\"\ntasks:\n  - task_id: t\n    command: run --select tag:nowhere\n");
        let g = graph();
        let (read, _) = build(&g, &d, &[source(AIRFLOW, &d, &[])]);
        let cov = coverage(&g, &read);
        assert_eq!(cov.empty.len(), 1);
        assert_eq!(cov.empty[0].why, "nothing matches `tag:nowhere`");
        assert_eq!(read.jobs[0].start, Start::Yes, "a preset is a clock");
    }

    #[test]
    fn a_default_selector_answers_a_bare_command() {
        let g = graph_with(json!({
            "nightly": { "name": "nightly", "default": true, "definition": { "method": "tag", "value": "staging" } },
        }));
        let inv = command::read("dbt run").unwrap();
        assert_eq!(names_of(&g, &select_nodes(&g, &inv).unwrap().0), ["stg_customers", "stg_orders"]);
        let two = graph_with(json!({
            "a": { "name": "a", "default": true, "definition": { "method": "tag", "value": "staging" } },
            "b": { "name": "b", "default": true, "definition": { "method": "tag", "value": "daily" } },
        }));
        assert!(select_nodes(&two, &inv).unwrap_err().contains("more than one selector"));
        // No default: everything, sources included for freshness.
        let fresh = command::read("dbt source freshness").unwrap();
        assert_eq!(select_nodes(&graph(), &fresh).unwrap().0.len(), 10);
    }

    #[test]
    fn a_selector_that_lost_its_indirect_selection_says_so() {
        // An older Fusion writes the selector without `indirect_selection`,
        // which selectors.yml sets: its tests are then dbt's default (0032).
        let d = dir("lost");
        write(&d, "selectors.yml", "selectors:\n  - name: orders_only\n    definition:\n      method: fqn\n      value: orders\n      indirect_selection: empty\n");
        let mut g = graph_with(json!({ "orders_only": { "name": "orders_only", "definition": { "method": "fqn", "value": "orders" } } }));
        g.selectors.check_against(&d);
        let (_, warnings) = select_nodes(&g, &command::read("dbt test --selector orders_only").unwrap()).unwrap();
        assert_eq!(warnings, [LOST_INDIRECT]);
        let (_, warnings) = select_nodes(&graph(), &command::read("dbt test --selector marts").unwrap()).unwrap();
        assert!(warnings.is_empty());
    }

    #[test]
    fn a_file_is_read_whole_or_refused_whole() {
        let d = dir("refused");
        write(&d, "bad.yml", "dag_id: x\ntasks:\n  - task_id: a\n    command: run\n  - task_id: b\n\tcommand: run\n");
        write(&d, "noid.yml", "tasks:\n  - task_id: a\n    command: run\n");
        let g = graph();
        let (read, _) = build(&g, &d, &[source(AIRFLOW, &d, &[])]);
        assert!(read.jobs.is_empty() && read.steps.is_empty());
        let refused = &read.sources[0].refused;
        assert_eq!(refused[0], Refusal { file: "bad.yml".into(), line: 6, message: "a tab in the indentation".into() });
        assert_eq!(refused[1].message, "dbt tasks, and no DAG id at `dag_id`");
    }

    #[test]
    fn the_walk_skips_hidden_files_other_files_and_symlinks() {
        let d = dir("walk");
        write(&d, "jobs/a.yml", DAG);
        write(&d, "jobs/.hidden.yml", DAG);
        write(&d, ".env", "SECRET=1\n");
        write(&d, "jobs/b.txt", DAG);
        write(&d, "node_modules/c.yml", DAG);
        write(&d, "target/d.yml", DAG);
        #[cfg(unix)]
        {
            let outside = dir("walk-outside");
            write(&outside, "d.yml", DAG);
            std::os::unix::fs::symlink(&outside, d.join("linked")).unwrap();
            std::os::unix::fs::symlink(outside.join("d.yml"), d.join("jobs/e.yml")).unwrap();
        }
        let (files, capped) = walk(&d).unwrap();
        assert_eq!(files.iter().map(|f| shown_path(&d, &f.0)).collect::<Vec<_>>(), ["jobs/a.yml", "target/d.yml"]);
        assert!(!capped);
        assert!(walk(&d.join("missing")).is_err());
        assert_eq!(walk(&d.join("jobs/b.txt")).unwrap_err(), "not a .yml or .yaml file");
    }

    #[test]
    fn a_listing_shows_what_the_walk_would_read_and_names_it_as_a_source_would() {
        // Canonical, as the project root is: the system's temporary folder
        // sits behind a symlink on macOS.
        let base = crate::files::plain(dir("browse").canonicalize().unwrap());
        let project = base.join("repos/shop/dbt");
        std::fs::create_dir_all(&project).unwrap();
        let jobs = base.join("repos/airflow");
        write(&jobs, "dags/b.yml", DAG);
        write(&jobs, "dags/A.yaml", DAG);
        write(&jobs, "dags/a.yml", DAG);
        write(&jobs, "dags/sub/c.yml", DAG);
        write(&jobs, "dags/.hidden.yml", DAG);
        write(&jobs, "dags/.env", "SECRET=1\n");
        write(&jobs, "dags/.git/config", "");
        write(&jobs, "dags/build.py", "");
        write(&jobs, "dags/__pycache__/x.yml", DAG);
        #[cfg(unix)]
        std::os::unix::fs::symlink(base.join("repos"), jobs.join("dags/linked")).unwrap();

        // Paths compared as paths: on Windows a joined one keeps its `/`.
        let l = browse(&project, "../../airflow/dags").unwrap();
        assert_eq!(Path::new(&l.path), jobs.join("dags"));
        assert_eq!((l.written.as_str(), l.up.as_deref()), ("../../airflow/dags", Some("../../airflow")));
        let names: Vec<(&str, bool, &str)> = l.entries.iter().map(|e| (e.name.as_str(), e.dir, e.written.as_str())).collect();
        assert_eq!(
            names,
            [
                ("sub", true, "../../airflow/dags/sub"),
                ("A.yaml", false, "../../airflow/dags/A.yaml"),
                ("a.yml", false, "../../airflow/dags/a.yml"),
                ("b.yml", false, "../../airflow/dags/b.yml"),
            ],
            "folders first, then by name whatever the case, and never a hidden entry, a symlink or another file"
        );
        assert!(!l.moved && l.picked.is_empty() && !l.capped);

        // Within the project, from it; past two folders up, in full.
        assert_eq!(browse(&project, "").unwrap().written, ".");
        assert_eq!(Path::new(&browse(&project, "./models/..").unwrap().path), project);
        let far = browse(&project, "../../..").unwrap();
        assert_eq!(Path::new(&far.written), base);
        assert_eq!(far.entries.iter().map(|e| e.written.as_str()).collect::<Vec<_>>(), ["../.."]);
        let absolute = jobs.join("dags").to_string_lossy().into_owned();
        assert_eq!(browse(&project, &absolute).unwrap().written, "../../airflow/dags");

        // A file is shown picked in its folder, and a path that leads nowhere,
        // or through a hidden name, lists the nearest folder above it.
        let file = browse(&project, "../../airflow/dags/b.yml").unwrap();
        assert_eq!((file.written.as_str(), file.picked.as_str(), file.moved), ("../../airflow/dags", "b.yml", false));
        let gone = browse(&project, "../../airflow/dags/gone/deeper").unwrap();
        assert_eq!((gone.written.as_str(), gone.picked.as_str(), gone.moved), ("../../airflow/dags", "", true));
        assert!(browse(&project, "../../airflow/dags/.env").unwrap().picked.is_empty());
        let hidden = browse(&project, "../../airflow/dags/.git").unwrap();
        assert_eq!((hidden.written.as_str(), hidden.moved), ("../../airflow/dags", true), "never into a hidden folder");
        #[cfg(unix)]
        assert_eq!(browse(&project, "../../airflow/dags/linked").unwrap().written, "../../airflow/dags", "never on a symlink");
        assert!(browse(&project, "a\nb").is_err());
        assert!(browse(&project, &"x".repeat(1025)).is_err());

        // The folders the project sits in are taken as they are, hidden or not.
        let tucked = base.join(".home/shop");
        std::fs::create_dir_all(&tucked).unwrap();
        let home = browse(&tucked, "").unwrap();
        assert_eq!((home.written.as_str(), home.moved), (".", false));

        // Past the bound, the file the path names is still listed.
        let many = jobs.join("many");
        std::fs::create_dir_all(&many).unwrap();
        for i in 0..=MAX_LISTED {
            std::fs::write(many.join(format!("f{i:04}.yml")), "").unwrap();
        }
        std::fs::write(many.join("zz.yml"), "").unwrap();
        let capped = browse(&project, "../../airflow/many/zz.yml").unwrap();
        assert!(capped.capped && capped.picked == "zz.yml");
        assert_eq!((capped.entries.len(), capped.entries.last().map(|e| e.name.as_str())), (MAX_LISTED + 1, Some("zz.yml")));

        // A folder the system will not list is shown empty, and left from.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let locked = jobs.join("locked");
            std::fs::create_dir_all(&locked).unwrap();
            std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
            // Root lists it all the same.
            if std::fs::read_dir(&locked).is_err() {
                let l = browse(&project, "../../airflow/locked").unwrap();
                assert!(!l.refused.is_empty() && l.entries.is_empty() && !l.moved, "{l:?}");
                assert_eq!(l.up.as_deref(), Some("../../airflow"));
            }
            std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
    }

    #[test]
    fn the_same_files_answer_the_same_on_every_build() {
        let d = dir("same");
        write(&d, "daily.yml", DAG);
        write(&d, "after.yml", ON_ASSET);
        write(&d, "cloud/jobs.yml", CLOUD);
        let sources = [source(AIRFLOW, &d, &[]), source(DBT_CLOUD, &d.join("cloud"), &[])];
        let answers: Vec<String> = (0..9)
            .map(|_| {
                let g = graph();
                let (read, _) = build(&g, &d, &sources);
                let orders = g.index["model.shop.orders"];
                serde_json::to_string(&(summary(&read), for_node(&g, &read, orders), coverage(&g, &read))).unwrap()
            })
            .collect();
        assert!(answers.windows(2).all(|w| w[0] == w[1]));
    }

    #[test]
    fn the_cache_reads_again_once_asked_to_look_and_a_file_changed() {
        let d = dir("cache");
        write(&d, "daily.yml", DAG);
        let g = Arc::new(graph());
        let sources = [source(AIRFLOW, &d, &[])];
        let cache = Cache::default();
        let first = cache.get(&g, &d, &sources, false);
        assert!(Arc::ptr_eq(&first, &cache.get(&g, &d, &sources, true)), "nothing changed, so a look reads nothing");
        write(&d, "daily.yml", &DAG.replace("shop_daily", "shop_renamed").replace("tasks:", "tasks:\n  - task_id: more\n    command: seed"));
        write(&d, "added.yml", "dag_id: added\nschedule: null\ntasks:\n  - task_id: t\n    command: run\n");
        assert!(Arc::ptr_eq(&first, &cache.get(&g, &d, &sources, false)), "a click does not look");
        let second = cache.get(&g, &d, &sources, true);
        let names: Vec<&str> = second.jobs.iter().map(|j| j.name.as_str()).collect();
        assert_eq!(names, ["added", "shop_renamed"]);
        // Another manifest is another answer, even over the same files.
        let other = Arc::new(graph());
        assert!(!Arc::ptr_eq(&second, &cache.get(&other, &d, &sources, false)));
    }

    /// The fixtures `scripts/compare_with_dbt.py` checks against dbt read
    /// whole, in the keys it names for them.
    #[test]
    fn the_job_fixtures_read_whole() {
        let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures").join("jobs");
        let renamed = [
            ("dag_id", "pipeline.name"),
            ("schedule", "pipeline.cron"),
            ("tasks", "steps"),
            ("task_id", "id"),
            ("command", "dbt"),
            ("sensors", "waits"),
            ("sensor_id", "id"),
            ("external_dag_id", "upstream_dag"),
            ("external_task_id", "upstream_task"),
        ];
        let sources = [
            source(AIRFLOW, &fixtures.join("airflow"), &[]),
            source(AIRFLOW, &fixtures.join("airflow_renamed"), &renamed),
            source(DBT_CLOUD, &fixtures.join("dbt_cloud"), &[]),
        ];
        let (read, _) = build(&Graph::build(Default::default(), Path::new("manifest.json"), 0, 0), &fixtures, &sources);
        let counts: Vec<(usize, usize, usize)> = read.sources.iter().map(|s| (s.files, s.jobs, s.steps)).collect();
        assert_eq!(counts, [(2, 2, 6), (1, 1, 4), (1, 3, 5)]);
        assert!(read.sources.iter().all(|s| s.refused.is_empty() && s.error.is_empty()));
        let refresh = read.steps.iter().find(|s| s.name == "refresh").unwrap();
        assert_eq!(refresh.shown, "dbt run --full-refresh -s customers+ 1+orders --exclude stg_payments+");
        let grants = read.steps.iter().find(|s| s.name == "grants").unwrap();
        assert_eq!(grants.shown, "dbt run-operation grant_select --args …");
        let on_marts = read.jobs.iter().find(|j| j.name == "jaffle_on_marts").unwrap();
        assert_eq!(on_marts.start, Start::Yes, "on an asset a scheduled DAG updates");
        assert_eq!(read.jobs.iter().find(|j| j.name == "Exports").unwrap().start, Start::Yes);
        assert_eq!(read.jobs.iter().find(|j| j.name == "CI").unwrap().start, Start::No);
    }

    #[test]
    fn checks_a_source_before_it_is_kept() {
        let ok = source(AIRFLOW, Path::new("../orchestration/dags"), &[("dag_id", "pipeline.name")]);
        assert_eq!(check(&ok), None);
        let cases = [
            (JobSource { kind: "luigi".into(), ..ok.clone() }, "airflow or dbt_cloud"),
            (JobSource { path: " ".into(), ..ok.clone() }, "a folder or a file"),
            (JobSource { path: "a\u{0}b".into(), ..ok.clone() }, "no control characters"),
            (source(AIRFLOW, Path::new("x"), &[("colour", "a")]), "`colour` is not a key"),
            (source(AIRFLOW, Path::new("x"), &[("dag_id", "dag..id")]), "dotted key path"),
            (source(DBT_CLOUD, Path::new("x"), &[("dag_id", "a")]), "keys do not move"),
        ];
        for (src, want) in cases {
            let got = check(&src).unwrap_or_default();
            assert!(got.contains(want), "{got}");
        }
    }
}
