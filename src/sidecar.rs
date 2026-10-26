//! The Snowflake script, `tools/sf_lineage.py serve`, started and stopped here.
//!
//! The binary never talks to a warehouse. It starts the script only once a
//! Snowflake feature wants it: Snowflake picked as the column lineage tool
//! (0031), the query history opened (0048), a relation's Snowsight menu
//! (0051), or the last builds read (0117). It stops once none does. The
//! script connects on its first request, so a sign-in tab can only ever follow
//! a click (0016).
//!
//! Two rules hold everywhere in this file:
//!   - nothing here can hang the server. Every wait has a deadline, and
//!     switching off interrupts a request still waiting on Snowflake.
//!   - nothing the script writes to stderr is logged. Single sign-on can print
//!     a login link there, so the last lines stay in memory, for the UI only.

use crate::collin::RelEdge;
use crate::venv::{self, VenvInfo};
use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, Notify};

/// Embedded, so a binary copied on its own still carries the script it speaks to.
const SCRIPT: &str = include_str!("../tools/sf_lineage.py");

const TAIL_LINES: usize = 20;

#[derive(Clone, Copy)]
pub struct Deadlines {
    /// Python, its imports and the profile: all local, but slow on a cold VM.
    pub ready: Duration,
    /// The first answer of a session may wait on a sign-in in the browser.
    pub first: Duration,
    pub later: Duration,
    /// How long the script gets to quit on its own before it is killed.
    pub quit: Duration,
}

impl Default for Deadlines {
    fn default() -> Self {
        Deadlines {
            ready: Duration::from_secs(60),
            first: Duration::from_secs(180),
            later: Duration::from_secs(60),
            quit: Duration::from_secs(2),
        }
    }
}

#[derive(serde::Serialize, Clone, Debug, Default, PartialEq)]
pub struct Status {
    /// off, starting, ready, busy or failed.
    pub state: &'static str,
    /// What was started, so the user can see which Python runs with their profile.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub python: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub profile: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub target: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub role: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub authenticator: String,
    /// The profiles.yml the script read, announced before it read it, so it is
    /// known even when reading it is what failed. Kept when the script stops.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub profiles: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub error: String,
    /// The script's last stderr lines, kept only after a failure.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub log: Vec<String>,
    /// The profile's Snowflake targets, so the query history can offer their roles.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub targets: Vec<TargetRole>,
}

/// One Snowflake target of the profile, as the script describes it: its name
/// and role, never its user or account, and whether it signs in as the target
/// connected, which is what decides if its queries are in the history read.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default, PartialEq)]
pub struct TargetRole {
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub role: String,
    #[serde(default)]
    pub same_login: bool,
    /// Why it cannot be offered, when it cannot.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub why: String,
}

/// Where the next page of history starts: the oldest row already shown, as
/// its start in epoch nanoseconds, a string because JavaScript cannot hold the
/// number, and its query id, which orders two queries that started together.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub struct Cursor {
    pub at: String,
    pub query_id: String,
}

impl Cursor {
    pub fn valid(&self) -> bool {
        (1..=20).contains(&self.at.len())
            && self.at.bytes().all(|b| b.is_ascii_digit())
            && (1..=64).contains(&self.query_id.len())
            && self.query_id.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-')
    }
}

/// Who the session is and on which account, which the Snowsight link needs.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default, PartialEq)]
pub struct SessionInfo {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub org: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub account: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub user: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub role: String,
}

/// One query of the user's history. Named field by field, so a column the
/// script starts sending reaches the page only once this build lets it.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default, PartialEq)]
pub struct HistoryRow {
    pub query_id: String,
    pub at: String,
    /// The start in UTC, to the millisecond.
    pub start: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ms: Option<u64>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub status: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub error: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub role: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub warehouse: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub database: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub schema: String,
    #[serde(default, rename = "type", skip_serializing_if = "String::is_empty")]
    pub kind: String,
    /// The start of the statement only; `length` says how long it is.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub text: String,
    #[serde(default)]
    pub length: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rows: Option<u64>,
}

/// One page of history, newest first.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default, PartialEq)]
pub struct History {
    #[serde(default)]
    pub session: SessionInfo,
    #[serde(default)]
    pub rows: Vec<HistoryRow>,
    /// Older queries are there for the next page.
    #[serde(default)]
    pub more: bool,
}

/// The last run of one model in one target, as dbt_artifacts recorded it.
/// Named field by field, like `HistoryRow`: the tables also hold the
/// invocation's env_vars, dbt_vars and invocation_args and each model's
/// adapter_response, which the script never selects and this build would not
/// pass on if it did (0117).
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default, PartialEq)]
pub struct BuildRow {
    /// The manifest's unique id, which `node_id` holds as dbt wrote it.
    pub node: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub target: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub status: String,
    /// UTC, to the second.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub started: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub completed: String,
    /// When the last run that succeeded finished, whatever came after it.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub last_success: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub invocation: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub full_refresh: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rows: Option<u64>,
    /// dbt's message, cut by the script.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub message: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub command: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub cloud_job: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub cloud_run: String,
}

/// The last run of each model in one environment's tables.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Builds {
    #[serde(default)]
    pub rows: Vec<BuildRow>,
    /// The table held more models than the script sends.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub cut: bool,
}

/// Whether a text is a relation of three parts, each unquoted or in double
/// quotes with `""` for a quote: what the script accepts, held here too so a
/// body it would refuse never starts it.
pub fn valid_relation(text: &str) -> bool {
    if text.len() > 400 {
        return false;
    }
    let mut chars = text.chars().peekable();
    for part in 0..3 {
        match chars.peek() {
            Some('"') => {
                chars.next();
                let mut any = false;
                loop {
                    match chars.next() {
                        Some('"') if chars.peek() == Some(&'"') => {
                            chars.next();
                            any = true;
                        }
                        Some('"') => break,
                        Some(c) if !c.is_control() => any = true,
                        _ => return false,
                    }
                }
                if !any {
                    return false;
                }
            }
            Some(c) if c.is_ascii_alphabetic() || *c == '_' => {
                while chars.peek().is_some_and(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '$') {
                    chars.next();
                }
            }
            _ => return false,
        }
        match (part, chars.next()) {
            (2, None) | (0 | 1, Some('.')) => {}
            _ => return false,
        }
    }
    true
}

/// Why a query failed, and whose fault it points at: a connection Snowflake
/// refused is about the profile, a query it rejected is about the object or
/// the role, and a script that stopped, timed out or sent what this build
/// cannot read is `script`: no later request to it is likelier to succeed.
#[derive(Debug, Clone, PartialEq)]
pub struct QueryError {
    pub message: String,
    pub phase: String,
}

impl QueryError {
    fn own(message: impl Into<String>) -> QueryError {
        QueryError { message: message.into(), phase: "script".into() }
    }
}

impl std::fmt::Display for QueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

/// A program that can run the script, with the arguments that go before it.
#[derive(Clone, Debug, PartialEq)]
pub struct Interpreter {
    pub program: PathBuf,
    pub args: Vec<String>,
}

/// Where to look for Python, best first: a virtual environment of the project
/// that has the Snowflake connector, then the one the status bar reports, whose
/// error says what is missing, then the PATH.
pub fn interpreters(env: &VenvInfo) -> Vec<Interpreter> {
    let chosen = (!env.chosen.path.is_empty()).then(|| PathBuf::from(&env.chosen.path));
    let mut envs: Vec<PathBuf> = chosen.iter().cloned().collect();
    envs.extend(env.found.iter().map(|v| PathBuf::from(&v.path)).filter(|p| Some(p) != chosen.as_ref()));
    let mut dirs: Vec<PathBuf> = envs.iter().filter(|e| venv::has_snowflake_connector(e)).cloned().collect();
    dirs.extend(chosen.filter(|e| !dirs.contains(e)));

    let mut out: Vec<Interpreter> = dirs
        .iter()
        .map(|dir| Interpreter { program: venv::python_in(dir), args: Vec::new() })
        .filter(|i| i.program.exists())
        .collect();
    let on_path: &[(&str, &[&str])] = if cfg!(windows) {
        &[("python", &[]), ("py", &["-3"])]
    } else {
        &[("python3", &[]), ("python", &[])]
    };
    out.extend(on_path.iter().map(|(program, args)| Interpreter {
        program: PathBuf::from(program),
        args: args.iter().map(|a| a.to_string()).collect(),
    }));
    out
}

/// Writes the embedded script under the configuration directory, named by its
/// hash: two versions of dbt-edith never overwrite each other's copy, and the
/// file is written only when it is missing or different.
pub fn install_script(config_dir: &Path) -> Result<PathBuf, String> {
    let name = format!("sf_lineage-{:016x}.py", crate::settings::fnv1a64(SCRIPT.as_bytes()));
    let path = config_dir.join("sidecar").join(name);
    if std::fs::read(&path).map_or(true, |bytes| bytes != SCRIPT.as_bytes()) {
        crate::settings::write_atomic(&path, SCRIPT.as_bytes()).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    }
    Ok(path)
}

struct Running {
    child: Child,
    stdin: Option<ChildStdin>,
    replies: mpsc::Receiver<serde_json::Value>,
    tail: Arc<Mutex<VecDeque<String>>>,
    stderr_reader: Option<JoinHandle<()>>,
    cancel: Arc<Notify>,
    next_id: u64,
    /// A request has succeeded, so the session is signed in and the shorter
    /// deadline applies.
    signed_in: bool,
}

enum Waited {
    Answer(serde_json::Value),
    Gone,
    Late,
    Cancelled,
}

pub struct Sidecar {
    /// Snowflake is the column lineage tool, so a column click fetches.
    enabled: AtomicBool,
    /// The query history or a Snowsight menu has read from Snowflake, which
    /// keeps the script running when another column lineage tool is picked.
    reading: AtomicBool,
    running: tokio::sync::Mutex<Option<Running>>,
    status: Mutex<Status>,
    /// The profile the script named. Outlives the script, so the file stays
    /// reachable while another tool is picked, which is when it gets corrected.
    profiles: Mutex<Option<PathBuf>>,
    /// What the last start used. A restart repeats it rather than working it
    /// out again, so it cannot quietly pick another interpreter.
    launch: Mutex<Option<(Vec<Interpreter>, PathBuf, PathBuf)>>,
    /// The running script's cancel signal, reachable without the lock that a
    /// request waiting on Snowflake holds.
    cancel: Mutex<Option<Arc<Notify>>>,
    deadlines: Deadlines,
}

impl Sidecar {
    pub fn new(enabled: bool, deadlines: Deadlines) -> Sidecar {
        Sidecar {
            enabled: AtomicBool::new(enabled),
            reading: AtomicBool::new(false),
            running: tokio::sync::Mutex::new(None),
            status: Mutex::new(Status { state: "off", ..Default::default() }),
            profiles: Mutex::new(None),
            launch: Mutex::new(None),
            cancel: Mutex::new(None),
            deadlines,
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    pub fn set_enabled(&self, on: bool) {
        self.enabled.store(on, Ordering::SeqCst);
    }

    pub fn set_reading(&self, on: bool) {
        self.reading.store(on, Ordering::SeqCst);
    }

    /// Some feature still wants the script running.
    pub fn wanted(&self) -> bool {
        self.enabled() || self.reading.load(Ordering::SeqCst)
    }

    /// Says "starting" ahead of a start run in the background, so the page
    /// polls for how it ends rather than reading "off" and stopping there.
    pub fn mark_starting(&self) {
        self.set_state("starting");
    }

    pub fn status(&self) -> Status {
        self.status.lock().map(|s| s.clone()).unwrap_or_default()
    }

    /// Ready or answering: a request can be sent without starting anything.
    pub fn is_up(&self) -> bool {
        matches!(self.status().state, "ready" | "busy")
    }

    /// The profile the script named, if it ever did.
    pub fn profile_path(&self) -> Option<PathBuf> {
        self.profiles.lock().ok().and_then(|p| p.clone())
    }

    fn set_status(&self, mut status: Status) -> Status {
        // Carried by every status, whatever the script is doing now.
        if let Some(path) = self.profile_path() {
            status.profiles = path.display().to_string();
        }
        if let Ok(mut s) = self.status.lock() {
            *s = status.clone();
        }
        status
    }

    fn set_state(&self, state: &'static str) {
        if let Ok(mut s) = self.status.lock() {
            s.state = state;
        }
    }

    fn fail(&self, python: String, error: String, log: Vec<String>) -> Status {
        self.set_status(Status { state: "failed", python, error, log, ..Default::default() })
    }

    /// Installs the script and starts it with the best interpreter there is.
    pub async fn start_for(&self, root: &Path, env: &VenvInfo) -> Status {
        let Some(dir) = crate::settings::config_dir(|key| std::env::var_os(key), cfg!(windows)) else {
            let error = "no configuration directory to install the Snowflake script in: set DBT_EDITH_CONFIG_DIR";
            return self.fail(String::new(), error.into(), Vec::new());
        };
        let script = match tokio::task::spawn_blocking(move || install_script(&dir)).await {
            Ok(Ok(path)) => path,
            Ok(Err(e)) => return self.fail(String::new(), e, Vec::new()),
            Err(e) => return self.fail(String::new(), e.to_string(), Vec::new()),
        };
        self.start(&interpreters(env), &script, root).await
    }

    /// Stops and starts again, on what the last start used. The script reads
    /// the profile once, so this is what a corrected profile needs.
    pub async fn restart(&self, root: &Path, env: &VenvInfo) -> Status {
        self.stop().await;
        let last = self.launch.lock().ok().and_then(|l| l.clone());
        match last {
            Some((candidates, script, cwd)) => self.start(&candidates, &script, &cwd).await,
            None => self.start_for(root, env).await,
        }
    }

    /// Starts the script unless it is running. The script checks everything
    /// that needs no network before it says it is ready, so a broken setup
    /// shows up here rather than on a click.
    pub async fn start(&self, candidates: &[Interpreter], script: &Path, cwd: &Path) -> Status {
        let mut slot = self.running.lock().await;
        if slot.is_some() {
            return self.status();
        }
        if let Ok(mut last) = self.launch.lock() {
            *last = Some((candidates.to_vec(), script.to_path_buf(), cwd.to_path_buf()));
        }
        self.set_status(Status { state: "starting", ..Default::default() });

        let mut error = String::from("no Python found: create the project's virtual environment, or put python on the PATH");
        let mut spawned = None;
        for candidate in candidates {
            // A bare name is looked up on the PATH alone: on Windows `Command`
            // would look beside this binary first, where a repository could
            // have committed a `python.exe` (0094).
            let program = if candidate.program.components().count() == 1 {
                match crate::program::on_path(&candidate.program.to_string_lossy()) {
                    Some(found) => found,
                    None => {
                        error = format!("cannot run {}: it is not on the PATH", candidate.program.display());
                        continue;
                    }
                }
            } else {
                candidate.program.clone()
            };
            let mut cmd = Command::new(&program);
            cmd.args(&candidate.args)
                .arg("-u")
                .arg(script)
                .arg("serve")
                .current_dir(cwd)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
            match cmd.spawn() {
                Ok(child) => {
                    spawned = Some((candidate, child));
                    break;
                }
                Err(e) => error = format!("cannot run {}: {e}", candidate.program.display()),
            }
        }
        let Some((candidate, mut child)) = spawned else {
            return self.fail(String::new(), error, Vec::new());
        };
        let python = std::iter::once(candidate.program.display().to_string())
            .chain(candidate.args.iter().cloned())
            .collect::<Vec<_>>()
            .join(" ");

        let (tx, replies) = mpsc::channel(64);
        if let Some(stdout) = child.stdout.take() {
            std::thread::spawn(move || {
                each_line(stdout, |line| {
                    // One JSON object per line is the protocol. Anything else is
                    // a library talking, and is dropped.
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) {
                        if value.is_object() {
                            let _ = tx.blocking_send(value);
                        }
                    }
                });
            });
        }
        let tail = Arc::new(Mutex::new(VecDeque::new()));
        let stderr_reader = child.stderr.take().map(|stderr| {
            let tail = tail.clone();
            std::thread::spawn(move || {
                each_line(stderr, |line| {
                    if let Ok(mut t) = tail.lock() {
                        if t.len() == TAIL_LINES {
                            t.pop_front();
                        }
                        t.push_back(line.chars().take(300).collect());
                    }
                });
            })
        });
        let mut run = Running {
            stdin: child.stdin.take(),
            child,
            replies,
            tail,
            stderr_reader,
            cancel: Arc::new(Notify::new()),
            next_id: 0,
            signed_in: false,
        };

        let ready = tokio::time::timeout(self.deadlines.ready, async {
            while let Some(value) = run.replies.recv().await {
                match value.get("event").and_then(|e| e.as_str()) {
                    // Announced before the file is read, so a profile that
                    // cannot be read is still a profile that can be opened.
                    Some("profiles") => {
                        if let (Ok(mut held), Some(path)) = (self.profiles.lock(), value.get("path").and_then(|p| p.as_str())) {
                            *held = Some(PathBuf::from(path));
                        }
                    }
                    Some("ready") => return Some(value),
                    _ => {}
                }
            }
            None
        })
        .await;
        match ready {
            Ok(Some(value)) if self.wanted() => {
                let text = |key: &str| value.get(key).and_then(|v| v.as_str()).unwrap_or_default().to_string();
                let targets = value
                    .get("targets")
                    .and_then(|t| serde_json::from_value(t.clone()).ok())
                    .unwrap_or_default();
                if let Ok(mut cancel) = self.cancel.lock() {
                    *cancel = Some(run.cancel.clone());
                }
                *slot = Some(run);
                self.set_status(Status {
                    state: "ready",
                    python,
                    profile: text("profile"),
                    target: text("target"),
                    role: text("role"),
                    authenticator: text("authenticator"),
                    targets,
                    ..Default::default()
                })
            }
            // Switched off while starting: nothing may stay running.
            Ok(Some(_)) => {
                self.shut(run).await;
                self.set_status(Status { state: "off", ..Default::default() })
            }
            Ok(None) => {
                let log = self.shut(run).await;
                let error = explain(&log).unwrap_or_else(|| "the Snowflake script stopped before it was ready".into());
                self.fail(python, error, log)
            }
            Err(_) => {
                let log = self.shut(run).await;
                let error = format!("the Snowflake script was not ready within {} s", self.deadlines.ready.as_secs());
                self.fail(python, error, log)
            }
        }
    }

    /// Column pairs around one column, in one direction.
    pub async fn query(&self, relation: &str, column: &str, direction: &str, depth: u32) -> Result<Vec<RelEdge>, QueryError> {
        let request = serde_json::json!({
            "op": "lineage",
            "relation": relation,
            "column": column,
            "direction": direction,
            "depth": depth,
        });
        let mut reply = self.ask(request).await?;
        let rows = reply.get_mut("rows").map(serde_json::Value::take).unwrap_or_else(|| serde_json::json!([]));
        serde_json::from_value(rows).map_err(unreadable)
    }

    /// One page of the user's own queries: `role` None for the connected
    /// target's, `*` for every role, `before` for the page after it.
    pub async fn history(&self, role: Option<&str>, before: Option<&Cursor>) -> Result<History, QueryError> {
        let request = serde_json::json!({ "op": "history", "role": role, "before": before });
        serde_json::from_value(self.ask(request).await?).map_err(unreadable)
    }

    /// The last run of each model in one environment's dbt_artifacts tables,
    /// named by their relations (0117).
    pub async fn builds(&self, executions: &str, invocations: &str) -> Result<Builds, QueryError> {
        let request = serde_json::json!({ "op": "builds", "executions": executions, "invocations": invocations });
        serde_json::from_value(self.ask(request).await?).map_err(unreadable)
    }

    /// Who the connection is and on which account, connecting first if it has
    /// not yet: what a Snowsight link names.
    pub async fn session(&self) -> Result<SessionInfo, QueryError> {
        let mut reply = self.ask(serde_json::json!({ "op": "session" })).await?;
        let session = reply.get_mut("session").map(serde_json::Value::take).unwrap_or_default();
        serde_json::from_value(session).map_err(unreadable)
    }

    /// Sends one request and waits for its reply. Requests go one at a time,
    /// because the script holds a single connection.
    async fn ask(&self, request: serde_json::Value) -> Result<serde_json::Value, QueryError> {
        let mut slot = self.running.lock().await;
        let Some(run) = slot.as_mut() else {
            return Err(QueryError::own("the Snowflake script is not running"));
        };
        run.next_id += 1;
        let id = run.next_id;
        let mut message = serde_json::Map::new();
        message.insert("id".into(), id.into());
        if let serde_json::Value::Object(fields) = request {
            message.extend(fields);
        }
        let request = serde_json::Value::Object(message);
        let sent = run
            .stdin
            .as_mut()
            .is_some_and(|stdin| writeln!(stdin, "{request}").and_then(|()| stdin.flush()).is_ok());
        let within = if run.signed_in { self.deadlines.later } else { self.deadlines.first };
        let cancel = run.cancel.clone();

        let waited = if sent {
            self.set_state("busy");
            tokio::select! {
                reply = next_reply(&mut run.replies, id) => reply.map_or(Waited::Gone, Waited::Answer),
                () = tokio::time::sleep(within) => Waited::Late,
                () = cancel.notified() => Waited::Cancelled,
            }
        } else {
            Waited::Gone
        };

        match waited {
            Waited::Answer(reply) => {
                self.set_state("ready");
                if let Some(error) = reply.get("error").and_then(|e| e.as_str()) {
                    let phase = reply.get("phase").and_then(|p| p.as_str()).unwrap_or_default();
                    return Err(QueryError { message: error.to_string(), phase: phase.to_string() });
                }
                run.signed_in = true;
                Ok(reply)
            }
            Waited::Cancelled => {
                if let Some(run) = slot.take() {
                    self.shut(run).await;
                }
                Err(QueryError::own("the Snowflake script was stopped"))
            }
            Waited::Gone | Waited::Late => {
                let late = matches!(waited, Waited::Late);
                let Some(run) = slot.take() else {
                    return Err(QueryError::own("the Snowflake script is not running"));
                };
                if let Ok(mut c) = self.cancel.lock() {
                    *c = None;
                }
                let log = self.shut(run).await;
                let error = if late {
                    format!("Snowflake did not answer within {} s, so the script was stopped. The next click starts it again.", within.as_secs())
                } else {
                    explain(&log).unwrap_or_else(|| "the Snowflake script stopped".into())
                };
                let python = self.status().python;
                self.fail(python, error.clone(), log);
                Err(QueryError::own(error))
            }
        }
    }

    /// Stops the script, interrupting a request that is still waiting.
    pub async fn stop(&self) -> Status {
        if let Some(cancel) = self.cancel.lock().ok().and_then(|mut c| c.take()) {
            // Kept as a permit when nothing waits yet, so a request that has
            // not reached its wait still sees it.
            cancel.notify_one();
        }
        let running = self.running.lock().await.take();
        if let Some(run) = running {
            self.shut(run).await;
        }
        self.set_status(Status { state: "off", ..Default::default() })
    }

    async fn shut(&self, run: Running) -> Vec<String> {
        let grace = self.deadlines.quit;
        tokio::task::spawn_blocking(move || shutdown(run, grace)).await.unwrap_or_default()
    }
}

fn unreadable(e: serde_json::Error) -> QueryError {
    QueryError::own(format!("the Snowflake script sent a reply this build cannot read: {e}"))
}

async fn next_reply(replies: &mut mpsc::Receiver<serde_json::Value>, id: u64) -> Option<serde_json::Value> {
    while let Some(value) = replies.recv().await {
        if value.get("id").and_then(|v| v.as_u64()) == Some(id) {
            return Some(value);
        }
    }
    None
}

/// Lines from a pipe, decoded leniently. Reading must never stop early: a pipe
/// nobody drains fills up, and the script then blocks on its next write.
fn each_line(pipe: impl Read, mut f: impl FnMut(String)) {
    let mut reader = BufReader::new(pipe);
    let mut buf = Vec::new();
    loop {
        buf.clear();
        match reader.read_until(b'\n', &mut buf) {
            Ok(0) | Err(_) => break,
            Ok(_) => f(String::from_utf8_lossy(&buf).trim_end_matches(['\r', '\n']).to_string()),
        }
    }
}

/// Asks the script to quit and closes its stdin, then kills it if it is still
/// there after `grace`. Closing stdin comes first because on Windows the
/// python.exe of a virtual environment is a launcher: killing it leaves the
/// interpreter it started running, while the end of stdin stops that one too.
fn shutdown(mut run: Running, grace: Duration) -> Vec<String> {
    if let Some(mut stdin) = run.stdin.take() {
        let _ = stdin.write_all(b"{\"op\":\"quit\"}\n");
    }
    let deadline = Instant::now() + grace;
    loop {
        match run.child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(20)),
            Ok(None) | Err(_) => {
                let _ = run.child.kill();
                let _ = run.child.wait();
                break;
            }
        }
    }
    // The last lines can still be in the pipe. Waited for briefly, never for
    // long: an interpreter the launcher started can keep the pipe open.
    if let Some(reader) = run.stderr_reader.take() {
        let until = Instant::now() + Duration::from_millis(500);
        while !reader.is_finished() && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    run.tail.lock().map(|t| t.iter().cloned().collect()).unwrap_or_default()
}

/// The script's own message when it gave one, otherwise its last line, which
/// after a crash is the exception.
fn explain(log: &[String]) -> Option<String> {
    log.iter()
        .rev()
        .find_map(|line| line.strip_prefix("sf_lineage: ").map(str::to_string))
        .or_else(|| log.iter().rev().find(|line| !line.trim().is_empty()).cloned())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::venv::Venv;

    fn quick() -> Deadlines {
        Deadlines {
            ready: Duration::from_secs(5),
            first: Duration::from_millis(1500),
            later: Duration::from_millis(1500),
            quit: Duration::from_millis(300),
        }
    }

    /// A stand-in for the Python script, run as `sh -u fake.sh serve`: to sh,
    /// the `-u` meant for Python only makes unset variables an error. It
    /// records its pid next to itself.
    fn fake(tag: &str, body: &str) -> (PathBuf, PathBuf) {
        let dir = std::env::temp_dir().join(format!("dbt-edith-sidecar-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let script = dir.join("fake.sh");
        std::fs::write(&script, format!("echo \"$$\" > \"$(dirname \"$0\")/pid\"\n{body}")).unwrap();
        (dir, script)
    }

    /// The first candidate does not exist, so starting also proves the fallback.
    fn sh() -> Vec<Interpreter> {
        vec![
            Interpreter { program: "/nonexistent/python3".into(), args: Vec::new() },
            Interpreter { program: "/bin/sh".into(), args: Vec::new() },
        ]
    }

    const READY: &str = r#"echo 'Initiating login request with your identity provider.'
echo '{"event":"ready","profile":"shop","target":"dev","role":"transformer","authenticator":"externalbrowser"}'
"#;

    fn gone(dir: &Path) -> bool {
        let pid = std::fs::read_to_string(dir.join("pid")).unwrap();
        let alive = Command::new("kill").args(["-0", pid.trim()]).stderr(Stdio::null()).status().unwrap().success();
        !alive
    }

    #[tokio::test]
    async fn replies_are_matched_by_id_and_noise_is_dropped() {
        let body = format!(
            r#"{READY}while IFS= read -r line; do
  case "$line" in
    *'"id":1,'*) echo 'progress: 50%'; echo '{{"id":9,"rows":[]}}'; echo '{{"id":1,"rows":[{{"from_rel":"RAW.CRM.CUSTOMERS","from_col":"id","to_rel":"ANALYTICS.STAGING.STG_CUSTOMERS","to_col":"customer_id","kind":"view","distance":1}}]}}' ;;
    *'"id":2,'*) echo 'sf_lineage: not for this request' >&2; echo '{{"id":2,"error":"Object does not exist"}}' ;;
    *'"op":"quit"'*) exit 0 ;;
  esac
done
"#
        );
        let (dir, script) = fake("replies", &body);
        let car = Sidecar::new(true, quick());

        let status = car.start(&sh(), &script, &dir).await;
        assert_eq!(status.state, "ready", "{status:?}");
        assert_eq!((status.profile.as_str(), status.target.as_str(), status.role.as_str()), ("shop", "dev", "transformer"));
        assert_eq!(status.python, "/bin/sh");
        assert!(car.is_up());

        let rows = car.query("ANALYTICS.STAGING.STG_CUSTOMERS", "customer_id", "UPSTREAM", 2).await.unwrap();
        assert_eq!(
            rows,
            [RelEdge {
                from_rel: "RAW.CRM.CUSTOMERS".into(),
                from_col: "id".into(),
                to_rel: "ANALYTICS.STAGING.STG_CUSTOMERS".into(),
                to_col: "customer_id".into(),
                kind: "view".into(),
            }]
        );
        assert_eq!(car.query("X.Y.Z", "c", "UPSTREAM", 1).await.unwrap_err().message, "Object does not exist");
        assert_eq!(car.status().state, "ready", "an error reply leaves the script running");

        assert_eq!(car.stop().await.state, "off");
        assert!(gone(&dir));
        let gone = car.query("X.Y.Z", "c", "UPSTREAM", 1).await.unwrap_err();
        assert_eq!(gone.phase, "script", "a script that is not running is the script's, not a query's");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[tokio::test]
    async fn a_page_of_history_is_read_and_the_targets_kept() {
        let body = r#"echo '{"event":"ready","profile":"shop","target":"dev","role":"transformer","authenticator":"externalbrowser","targets":[{"name":"dev","role":"transformer","same_login":true},{"name":"ci","role":"ci_runner","same_login":false,"why":"it signs in as another user or account"}]}'
while IFS= read -r line; do
  case "$line" in
    *'"op":"history"'*)
      printf '%s\n' "$line" > "$(dirname "$0")/asked"
      echo '{"id":1,"session":{"org":"MYORG","account":"MY_ACCOUNT","user":"SOMEONE","role":"TRANSFORMER"},"rows":[{"query_id":"01b7-0001","at":"1700000000123456789","start":"2023-11-14T22:13:20.123Z","ms":1200,"status":"success","role":"TRANSFORMER","type":"SELECT","text":"select 1","length":8,"rows":1,"extra":"not for the page"}],"more":true}' ;;
    *'"op":"quit"'*) exit 0 ;;
  esac
done
"#;
        let (dir, script) = fake("history", body);
        // Only the history wants it: the start still keeps the script.
        let car = Sidecar::new(false, quick());
        car.set_reading(true);
        let status = car.start(&sh(), &script, &dir).await;
        assert_eq!(status.state, "ready", "{status:?}");
        assert_eq!(status.targets.len(), 2);
        assert_eq!(
            status.targets[1],
            TargetRole {
                name: "ci".into(),
                role: "ci_runner".into(),
                same_login: false,
                why: "it signs in as another user or account".into(),
            }
        );

        let before = Cursor { at: "1700000000999000000".into(), query_id: "01b7-0002".into() };
        let page = car.history(Some("reporter"), Some(&before)).await.unwrap();
        let asked = std::fs::read_to_string(dir.join("asked")).unwrap();
        assert!(asked.contains(r#""role":"reporter""#), "{asked}");
        assert!(asked.contains(r#""before":{"at":"1700000000999000000","query_id":"01b7-0002"}"#), "{asked}");
        assert_eq!(page.session.org, "MYORG");
        assert!(page.more);
        assert_eq!(page.rows[0].query_id, "01b7-0001");
        assert_eq!(page.rows[0].kind, "SELECT");
        assert_eq!((page.rows[0].ms, page.rows[0].rows), (Some(1200), Some(1)));
        let sent = serde_json::to_string(&page.rows[0]).unwrap();
        assert!(!sent.contains("extra"), "a field this build does not name stays here: {sent}");

        car.stop().await;
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[tokio::test]
    async fn the_last_builds_are_read_and_only_the_named_fields_pass() {
        let body = r#"echo '{"event":"ready","profile":"shop","target":"dev"}'
while IFS= read -r line; do
  case "$line" in
    *'"op":"builds"'*)
      printf '%s\n' "$line" > "$(dirname "$0")/asked"
      echo '{"id":1,"rows":[{"node":"model.jaffle.orders","target":"prod","status":"error","started":"2026-10-21T03:00:05Z","last_success":"2026-10-14T03:01:09Z","invocation":"6f1c-0001","full_refresh":true,"rows":7,"message":"Database Error","command":"build","cloud_job":"4321","env_vars":"{\"TOKEN\":\"hunter2\"}","adapter_response":"not for the page"}],"cut":true}' ;;
    *'"op":"quit"'*) exit 0 ;;
  esac
done
"#;
        let (dir, script) = fake("builds", body);
        let car = Sidecar::new(false, quick());
        car.set_reading(true);
        assert_eq!(car.start(&sh(), &script, &dir).await.state, "ready");

        let read = car.builds("ANALYTICS_META.DBT_ARTIFACTS.MODEL_EXECUTIONS", "ANALYTICS_META.DBT_ARTIFACTS.INVOCATIONS").await.unwrap();
        let asked = std::fs::read_to_string(dir.join("asked")).unwrap();
        assert!(asked.contains(r#""executions":"ANALYTICS_META.DBT_ARTIFACTS.MODEL_EXECUTIONS""#), "{asked}");
        assert!(asked.contains(r#""invocations":"ANALYTICS_META.DBT_ARTIFACTS.INVOCATIONS""#), "{asked}");
        assert!(read.cut);
        assert_eq!(read.rows.len(), 1);
        let row = &read.rows[0];
        assert_eq!((row.node.as_str(), row.target.as_str(), row.status.as_str()), ("model.jaffle.orders", "prod", "error"));
        assert_eq!((row.full_refresh, row.rows), (true, Some(7)));
        assert_eq!(row.last_success, "2026-10-14T03:01:09Z");
        let sent = serde_json::to_string(&read).unwrap();
        assert!(!sent.contains("hunter2") && !sent.contains("env_vars") && !sent.contains("adapter_response"), "a field this build does not name stays here: {sent}");
        assert!(!sent.contains("completed"), "an empty field is left out: {sent}");

        car.stop().await;
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_relation_is_three_parts_quoted_or_not() {
        for ok in ["a.b.c", "ANALYTICS_META.dbt_artifacts.model_executions", r#""my.db"."sch""ema".t"#, "_a$.b1.c_", r#""a"."b"."c""#] {
            assert!(valid_relation(ok), "{ok}");
        }
        for bad in [
            "", "a", "a.b", "a.b.c.d", "a..c", ".b.c", "a.b.", "1a.b.c", "a b.c.d", "a.b.c;", "a.b.c' or 1=1", r#"a."".c"#,
            r#"a."b.c"#, "a.\"b\nc\".d", "a-b.c.d",
        ] {
            assert!(!valid_relation(bad), "{bad:?}");
        }
        assert!(!valid_relation(&format!("a.b.{}", "c".repeat(401))), "too long");
    }

    #[tokio::test]
    async fn the_session_is_asked_on_its_own() {
        let body = r#"echo '{"event":"ready","profile":"shop","target":"dev"}'
while IFS= read -r line; do
  case "$line" in
    *'"op":"session"'*) echo '{"id":1,"session":{"org":"MYORG","account":"MY_ACCOUNT","user":"SOMEONE","role":"TRANSFORMER"}}' ;;
    *'"op":"quit"'*) exit 0 ;;
  esac
done
"#;
        let (dir, script) = fake("session", body);
        let car = Sidecar::new(false, quick());
        car.set_reading(true);
        assert_eq!(car.start(&sh(), &script, &dir).await.state, "ready");
        let session = car.session().await.unwrap();
        assert_eq!((session.org.as_str(), session.account.as_str()), ("MYORG", "MY_ACCOUNT"));
        assert_eq!(car.status().state, "ready");
        car.stop().await;
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_cursor_is_digits_and_a_query_id() {
        let cursor = |at: &str, id: &str| Cursor { at: at.into(), query_id: id.into() };
        assert!(cursor("1700000000123456789", "01b7a2c4-0000-003b").valid());
        assert!(!cursor("", "01b7").valid());
        assert!(!cursor("-1", "01b7").valid());
        assert!(!cursor("1", "x' or 1=1").valid());
        assert!(!cursor("1", &"a".repeat(65)).valid());
    }

    #[tokio::test]
    async fn a_script_that_stops_before_ready_says_why() {
        let (dir, script) = fake(
            "dies",
            "echo 'Traceback (most recent call last):' >&2\necho 'sf_lineage: snowflake-connector-python is not installed in this interpreter' >&2\nexit 2\n",
        );
        let car = Sidecar::new(true, quick());
        let status = car.start(&sh(), &script, &dir).await;
        assert_eq!(status.state, "failed");
        assert_eq!(status.error, "snowflake-connector-python is not installed in this interpreter");
        assert!(status.log.iter().any(|l| l.starts_with("Traceback")), "{:?}", status.log);
        assert!(!car.is_up());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[tokio::test]
    async fn nothing_to_run_is_a_failure_that_names_the_program() {
        let car = Sidecar::new(true, quick());
        let missing = [Interpreter { program: "/nonexistent/python3".into(), args: Vec::new() }];
        let status = car.start(&missing, Path::new("/nonexistent/script.py"), &std::env::temp_dir()).await;
        assert_eq!(status.state, "failed");
        assert!(status.error.starts_with("cannot run /nonexistent/python3"), "{}", status.error);
        assert!(car.start(&[], Path::new("x.py"), &std::env::temp_dir()).await.error.starts_with("no Python found"));
    }

    #[tokio::test]
    async fn a_silent_snowflake_is_given_up_on_at_the_deadline() {
        let (dir, script) = fake("silent", &format!("{READY}while IFS= read -r line; do :; done\n"));
        let car = Sidecar::new(true, quick());
        assert_eq!(car.start(&sh(), &script, &dir).await.state, "ready");

        let started = Instant::now();
        let error = car.query("X.Y.Z", "c", "UPSTREAM", 1).await.unwrap_err();
        assert!(started.elapsed() >= Duration::from_millis(1400), "{:?}", started.elapsed());
        assert!(error.message.starts_with("Snowflake did not answer within 1 s"), "{error}");
        let status = car.status();
        assert_eq!((status.state, status.python.as_str()), ("failed", "/bin/sh"));
        assert!(gone(&dir));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[tokio::test]
    async fn switching_off_interrupts_a_request_that_is_still_waiting() {
        let (dir, script) = fake("cancel", &format!("{READY}while IFS= read -r line; do :; done\n"));
        let slow = Deadlines { first: Duration::from_secs(30), ..quick() };
        let car = Arc::new(Sidecar::new(true, slow));
        assert_eq!(car.start(&sh(), &script, &dir).await.state, "ready");

        let asking = tokio::spawn({
            let car = car.clone();
            async move { car.query("X.Y.Z", "c", "UPSTREAM", 1).await }
        });
        tokio::time::sleep(Duration::from_millis(200)).await;
        let started = Instant::now();
        assert_eq!(car.stop().await.state, "off");
        let answer = tokio::time::timeout(Duration::from_secs(5), asking).await.expect("the request must return").unwrap();
        assert_eq!(answer.unwrap_err().message, "the Snowflake script was stopped");
        assert!(started.elapsed() < Duration::from_secs(3), "{:?}", started.elapsed());
        assert!(gone(&dir));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[tokio::test]
    async fn a_script_that_ignores_quit_is_killed() {
        let (dir, script) = fake("stubborn", &format!("{READY}trap '' TERM\nwhile :; do sleep 0.1; done\n"));
        let car = Sidecar::new(true, quick());
        assert_eq!(car.start(&sh(), &script, &dir).await.state, "ready");
        let started = Instant::now();
        car.stop().await;
        assert!(started.elapsed() >= Duration::from_millis(300), "{:?}", started.elapsed());
        assert!(gone(&dir));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[tokio::test]
    async fn a_start_that_ends_after_switching_off_leaves_nothing_running() {
        let (dir, script) = fake("late", &format!("{READY}while IFS= read -r line; do :; done\n"));
        let car = Sidecar::new(false, quick());
        assert_eq!(car.start(&sh(), &script, &dir).await.state, "off");
        assert!(gone(&dir));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[tokio::test]
    async fn the_profile_is_named_before_ready_and_outlives_the_script() {
        let body = format!(
            r#"echo '{{"event":"profiles","path":"/tmp/named/profiles.yml"}}'
{READY}asked=0
while IFS= read -r line; do
  case "$line" in
    *'"op":"quit"'*) exit 0 ;;
  esac
  asked=$((asked + 1))
  if [ "$asked" = 1 ]; then
    echo '{{"id":1,"error":"251005: User is empty","phase":"connect"}}'
  else
    echo '{{"id":2,"error":"Object does not exist","phase":"query"}}'
  fi
done
"#
        );
        let (dir, script) = fake("named", &body);
        let car = Sidecar::new(true, quick());

        let status = car.start(&sh(), &script, &dir).await;
        assert_eq!((status.state, status.profiles.as_str()), ("ready", "/tmp/named/profiles.yml"));

        // The phase is what says whether the profile is to blame.
        let refused = car.query("X.Y.Z", "c", "UPSTREAM", 1).await.unwrap_err();
        assert_eq!((refused.message.as_str(), refused.phase.as_str()), ("251005: User is empty", "connect"));
        let rejected = car.query("X.Y.Z", "c", "UPSTREAM", 1).await.unwrap_err();
        assert_eq!(rejected.phase, "query");

        // Switched off, the file stays reachable: off is when it gets corrected.
        assert_eq!(car.stop().await.profiles, "/tmp/named/profiles.yml");
        assert_eq!(car.profile_path(), Some(PathBuf::from("/tmp/named/profiles.yml")));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[tokio::test]
    async fn a_restart_repeats_the_last_start() {
        let (dir, script) = fake("restart", &format!("{READY}while IFS= read -r line; do :; done\n"));
        let car = Sidecar::new(true, quick());
        assert_eq!(car.start(&sh(), &script, &dir).await.state, "ready");
        let first = std::fs::read_to_string(dir.join("pid")).unwrap();

        // A root that holds nothing: working the interpreter out again would
        // fail, so a ready script proves the recorded start was repeated.
        let status = car.restart(Path::new("/nonexistent"), &VenvInfo::default()).await;
        assert_eq!(status.state, "ready", "{status:?}");
        assert_ne!(std::fs::read_to_string(dir.join("pid")).unwrap(), first, "a restart starts another process");

        car.stop().await;
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_venv_with_the_connector_comes_first_then_the_status_bar_one_then_the_path() {
        let root = std::env::temp_dir().join(format!("dbt-edith-sidecar-venvs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for name in ["plain-env", "sf-env", "other-env"] {
            std::fs::create_dir_all(root.join(name).join("bin")).unwrap();
            std::fs::write(root.join(name).join("bin").join("python"), "").unwrap();
        }
        std::fs::create_dir_all(root.join("sf-env/lib/python3.12/site-packages/snowflake/connector")).unwrap();
        let found = |name: &str| Venv { name: name.into(), path: root.join(name).display().to_string(), ..Default::default() };
        let env = VenvInfo { chosen: found("plain-env"), found: vec![found("plain-env"), found("sf-env"), found("other-env")], ..Default::default() };
        let programs: Vec<String> = interpreters(&env).iter().map(|i| i.program.display().to_string()).collect();
        assert_eq!(
            programs,
            [
                root.join("sf-env/bin/python").display().to_string(),
                root.join("plain-env/bin/python").display().to_string(),
                "python3".to_string(),
                "python".to_string(),
            ]
        );
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn the_script_is_installed_once_per_version() {
        let dir = std::env::temp_dir().join(format!("dbt-edith-sidecar-install-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = install_script(&dir).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), SCRIPT);
        std::fs::write(&path, "tampered").unwrap();
        assert_eq!(install_script(&dir).unwrap(), path);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), SCRIPT, "a different copy is replaced");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_explanation_prefers_the_scripts_own_words() {
        let log = |lines: &[&str]| lines.iter().map(|l| l.to_string()).collect::<Vec<_>>();
        assert_eq!(explain(&log(&["sf_lineage: no profiles.yml at x", "later noise"])).as_deref(), Some("no profiles.yml at x"));
        assert_eq!(explain(&log(&["Traceback", "TypeError: bad", ""])).as_deref(), Some("TypeError: bad"));
        assert_eq!(explain(&[]), None);
    }
}
