//! The MCP server over stdio (0110): one JSON-RPC message per line in, one per
//! line out, and one tool, `edith`, that answers a command line.
//!
//! Invariants. stdout carries the protocol and nothing else: no `println!`
//! anywhere in the program, and no child inherits it (git, collin and the
//! version probes all get a null stdin and a piped stdout), so a child cannot
//! write into the stream or read from it. Nothing is read from the project
//! until a call needs a graph. The legacy handshake only (`initialize`, 2025-11-25
//! and the two before it): a client that opens with `server/discover` gets
//! -32601 and falls back, and the modern one is left for later.
//!
//! Two threads. The one that reads stdin answers `initialize`, `ping` and
//! `tools/list` itself, and acts on `notifications/cancelled` and on end of
//! input, so none of them waits behind a call. Every `tools/call` goes to a
//! worker that answers them in order. The only call that lasts is
//! `--column --run-collin`, whose collin keeps its 600 s deadline; a
//! cancellation or the end of input sets the stop flag that kills it, and a
//! cancelled call writes no response, as the protocol asks.

use crate::mcp_graphs::Graphs;
use crate::mcp_line::{self, HELP, INSTRUCTIONS, TOOL};
use crate::{collin_run, describe, job, job_order, ls, ls_column, Question};
use serde_json::{json, Map, Value};
use std::collections::HashSet;
use std::io::{self, BufRead, Read, Write};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex, MutexGuard, PoisonError};

/// The versions `initialize` answers with its own: the client's, when it is
/// one of these, else the first.
const VERSIONS: [&str; 3] = ["2025-11-25", "2025-06-18", "2025-03-26"];

/// The longest message read. A line past it is answered and skipped, never
/// held whole.
const MAX_MESSAGE: usize = 1024 * 1024;

const PARSE_ERROR: i64 = -32700;
const INVALID_REQUEST: i64 = -32600;
const METHOD_NOT_FOUND: i64 = -32601;
const INVALID_PARAMS: i64 = -32602;
const INTERNAL_ERROR: i64 = -32603;

/// What a call answers: the command line's stdout and stderr, and whether it
/// ended the way the command line's exit code says it failed.
pub struct Outcome {
    pub out: String,
    pub err: String,
    pub is_error: bool,
}

impl Outcome {
    fn failed(why: &str) -> Outcome {
        Outcome { out: String::new(), err: format!("{why}\n"), is_error: true }
    }
}

/// The server on the standard streams, until stdin closes. The exit code.
pub fn serve(root: PathBuf, manifest: Option<PathBuf>, catalog: Option<PathBuf>) -> i32 {
    serve_on(io::stdin().lock(), io::stdout(), Graphs::new(root, manifest, catalog), answer_line);
    0
}

/// What the reading thread knows of the calls: the ones handed to the worker
/// and not yet answered, those among them that were cancelled, the one the
/// worker is on, and whether stdin has closed.
#[derive(Default)]
struct State {
    pending: HashSet<String>,
    cancelled: HashSet<String>,
    current: Option<(String, Arc<AtomicBool>)>,
    closed: bool,
}

/// The two things both threads touch: the state above, and stdout, written
/// whole lines at a time under one lock. Neither lock stays poisoned: a
/// panic in one thread must not take the other's answers with it.
struct Wire<W> {
    state: Mutex<State>,
    out: Mutex<W>,
}

impl<W: Write> Wire<W> {
    fn new(out: W) -> Wire<W> {
        Wire { state: Mutex::new(State::default()), out: Mutex::new(out) }
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// One message as one line. `to_string` escapes every newline inside a
    /// string, so the line holds none of its own.
    fn emit(&self, message: &Value) {
        let mut line = message.to_string();
        line.push('\n');
        let mut out = self.out.lock().unwrap_or_else(PoisonError::into_inner);
        let _ = out.write_all(line.as_bytes()).and_then(|_| out.flush());
    }

    /// The worker takes up a call: None when it was cancelled while it waited,
    /// else the flag that ends it early. Stdin already closed starts it
    /// stopped.
    fn begin(&self, key: &str) -> Option<Arc<AtomicBool>> {
        let mut state = self.state();
        if state.cancelled.remove(key) {
            state.pending.remove(key);
            return None;
        }
        let stop = Arc::new(AtomicBool::new(state.closed));
        state.current = Some((key.to_string(), stop.clone()));
        Some(stop)
    }

    /// The worker has answered: written unless the call was cancelled, in
    /// which case the protocol wants no further message for it.
    fn finish(&self, key: &str, message: &Value) {
        let cancelled = {
            let mut state = self.state();
            state.current = None;
            state.pending.remove(key);
            state.cancelled.remove(key)
        };
        if !cancelled {
            self.emit(message);
        }
    }

    /// `notifications/cancelled`: a call still waiting is skipped, the one
    /// running is stopped, and either way its response is dropped. A request
    /// that is not pending (answered already, or never sent) is ignored.
    fn cancel(&self, key: &str) {
        let mut state = self.state();
        if !state.pending.contains(key) {
            return;
        }
        state.cancelled.insert(key.to_string());
        if let Some((current, stop)) = &state.current {
            if current == key {
                stop.store(true, Ordering::Relaxed);
            }
        }
    }

    /// Stdin has closed: the call running is stopped, and the ones after it
    /// start stopped.
    fn close(&self) {
        let mut state = self.state();
        state.closed = true;
        if let Some((_, stop)) = &state.current {
            stop.store(true, Ordering::Relaxed);
        }
    }
}

/// A `tools/call` handed to the worker.
struct Job {
    id: Value,
    key: String,
    line: String,
}

/// The server on any streams. `answer` is what a call runs: `answer_line` for
/// the program, and a stand-in where a test wants a call that waits or fails.
fn serve_on<R, W, F>(mut input: R, output: W, mut graphs: Graphs, mut answer: F)
where
    R: BufRead,
    W: Write + Send,
    F: FnMut(&mut Graphs, &str, &AtomicBool) -> Outcome + Send,
{
    let wire = Wire::new(output);
    let (jobs, queue) = mpsc::channel::<Job>();
    std::thread::scope(|scope| {
        let wire = &wire;
        scope.spawn(move || {
            for job in queue {
                let Some(stop) = wire.begin(&job.key) else { continue };
                let called = catch_unwind(AssertUnwindSafe(|| answer(&mut graphs, &job.line, &stop)));
                let message = match called {
                    Ok(outcome) => result(&job.id, tool_result(&outcome)),
                    Err(panic) => {
                        // The call may have left a graph half made.
                        graphs.forget();
                        error(&job.id, INTERNAL_ERROR, &format!("internal error: {}", panic_text(&*panic)))
                    }
                };
                wire.finish(&job.key, &message);
            }
        });
        let mut line = Vec::new();
        loop {
            match read_message(&mut input, &mut line, MAX_MESSAGE) {
                Ok(Line::Message) => handle(wire, &jobs, &line),
                Ok(Line::TooLong) => wire.emit(&error(&Value::Null, INVALID_REQUEST, "the message is over 1 MiB")),
                Ok(Line::Eof) | Err(_) => break,
            }
        }
        wire.close();
        // The worker ends when its queue does, after the call it is on.
        drop(jobs);
    });
}

enum Line {
    Eof,
    Message,
    TooLong,
}

/// The next line of `input` into `line`, without its newline. At most `max`
/// bytes of it are read: a longer one is skipped up to its newline and
/// reported, so a client cannot make the server hold it whole. A last line
/// with no newline is a message.
fn read_message<R: BufRead>(input: &mut R, line: &mut Vec<u8>, max: usize) -> io::Result<Line> {
    line.clear();
    let n = Read::take(&mut *input, max as u64 + 1).read_until(b'\n', line)?;
    if n == 0 {
        return Ok(Line::Eof);
    }
    if line.last() == Some(&b'\n') {
        line.pop();
    } else if line.len() > max {
        let mut skipped = Vec::new();
        loop {
            skipped.clear();
            let n = Read::take(&mut *input, 64 * 1024).read_until(b'\n', &mut skipped)?;
            if n == 0 || skipped.last() == Some(&b'\n') {
                break;
            }
        }
        line.clear();
        return Ok(Line::TooLong);
    }
    Ok(Line::Message)
}

fn result(id: &Value, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

fn error(id: &Value, code: i64, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}

fn panic_text(panic: &(dyn std::any::Any + Send)) -> String {
    panic
        .downcast_ref::<&str>()
        .map(|s| s.to_string())
        .or_else(|| panic.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "a call panicked".to_string())
}

/// One line of stdin, read as one message and answered, or handed to the
/// worker.
fn handle<W: Write>(wire: &Wire<W>, jobs: &mpsc::Sender<Job>, line: &[u8]) {
    let line = line.strip_suffix(b"\r").unwrap_or(line);
    if line.iter().all(u8::is_ascii_whitespace) {
        return;
    }
    let message: Value = match serde_json::from_slice(line) {
        Ok(v) => v,
        Err(e) => return wire.emit(&error(&Value::Null, PARSE_ERROR, &format!("parse error: {e}"))),
    };
    let Value::Object(message) = message else {
        let why = if message.is_array() { "batches are not supported" } else { "a message is a JSON object" };
        return wire.emit(&error(&Value::Null, INVALID_REQUEST, why));
    };
    // Without an id it is a notification, which is never answered, not even
    // to say it is wrong.
    let id = message.get("id");
    if id.is_some_and(|i| !(i.is_string() || i.is_number())) {
        return wire.emit(&error(&Value::Null, INVALID_REQUEST, "an id is a string or a number"));
    }
    let reply_to = id.cloned().unwrap_or(Value::Null);
    let method = match (message.get("jsonrpc"), message.get("method")) {
        (Some(Value::String(v)), Some(Value::String(m))) if v == "2.0" => m.as_str(),
        _ => return wire.emit(&error(&reply_to, INVALID_REQUEST, "a request is {\"jsonrpc\":\"2.0\",\"method\":...}")),
    };
    let params = message.get("params");
    // Only a notification (no id) cancels: a request of that name is a
    // request for a method the server does not have, and is answered so.
    if method == "notifications/cancelled" && id.is_none() {
        if let Some(requested) = params.and_then(|p| p.get("requestId")) {
            wire.cancel(&requested.to_string());
        }
        return;
    }
    if id.is_none() {
        return;
    }
    match method {
        "initialize" => wire.emit(&result(&reply_to, initialized(params))),
        "ping" => wire.emit(&result(&reply_to, json!({}))),
        "tools/list" => wire.emit(&match listed(params) {
            Ok(tools) => result(&reply_to, tools),
            Err(why) => error(&reply_to, INVALID_PARAMS, &why),
        }),
        "tools/call" => call(wire, jobs, reply_to, params),
        other => wire.emit(&error(&reply_to, METHOD_NOT_FOUND, &format!("Method not found: {other}"))),
    }
}

/// `initialize`: the client's version when it is one served, else the latest.
fn initialized(params: Option<&Value>) -> Value {
    let asked = params.and_then(|p| p.get("protocolVersion")).and_then(Value::as_str);
    let version = asked.filter(|v| VERSIONS.contains(v)).unwrap_or(VERSIONS[0]);
    json!({
        "protocolVersion": version,
        "capabilities": {"tools": {}},
        "serverInfo": {"name": "dbt-edith", "version": env!("CARGO_PKG_VERSION")},
        "instructions": INSTRUCTIONS,
    })
}

/// `tools/list`: the one tool. There is no next page to ask for.
fn listed(params: Option<&Value>) -> Result<Value, String> {
    if params.and_then(|p| p.get("cursor")).is_some_and(|c| !c.is_null()) {
        return Err("the list is one page: there is no cursor".to_string());
    }
    let tool: Value = serde_json::from_str(TOOL).map_err(|e| format!("the tool does not read: {e}"))?;
    Ok(json!({"tools": [tool]}))
}

/// `tools/call`: checked here, so a wrong name or type is answered at once;
/// the line itself goes to the worker.
fn call<W: Write>(wire: &Wire<W>, jobs: &mpsc::Sender<Job>, id: Value, params: Option<&Value>) {
    let invalid = |why: &str| wire.emit(&error(&id, INVALID_PARAMS, why));
    let Some(Value::Object(params)) = params else { return invalid("tools/call takes params: {\"name\", \"arguments\"}") };
    let name = params.get("name").and_then(Value::as_str);
    if name != Some("edith") {
        return invalid(&format!("Unknown tool: {}", name.unwrap_or("")));
    }
    let empty = Map::new();
    let arguments = match params.get("arguments") {
        None | Some(Value::Null) => &empty,
        Some(Value::Object(a)) => a,
        Some(_) => return invalid("arguments is an object: {\"line\": \"...\"}"),
    };
    let line = match arguments.get("line") {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(l)) => l.clone(),
        Some(_) => return invalid("line is a string"),
    };
    // Input that does not fit the schema is the model's to correct: a tool
    // error, not a protocol one.
    if let Some(extra) = arguments.keys().find(|k| *k != "line") {
        let why = format!("error: the tool takes one argument, `line`, and not `{extra}`.\n`help` lists every flag.");
        return wire.emit(&result(&id, tool_result(&Outcome { out: String::new(), err: why, is_error: true })));
    }
    let key = id.to_string();
    if !wire.state().pending.insert(key.clone()) {
        return wire.emit(&error(&id, INVALID_REQUEST, "that id is in use by a call still running"));
    }
    if jobs.send(Job { id: id.clone(), key: key.clone(), line }).is_err() {
        wire.state().pending.remove(&key);
        wire.emit(&error(&id, INTERNAL_ERROR, "internal error: the worker has stopped"));
    }
}

/// The result of a call: the command line's stdout, then its stderr, as two
/// text blocks, an empty one left out.
fn tool_result(outcome: &Outcome) -> Value {
    let content: Vec<Value> =
        [&outcome.out, &outcome.err].into_iter().filter(|text| !text.is_empty()).map(|text| json!({"type": "text", "text": text})).collect();
    json!({"content": content, "isError": outcome.is_error})
}

/// A call: one command line, read against the allow-list and answered from
/// the project the server was started on, as the command line answers it. The
/// text it writes is the command line's, stdout and stderr, and `is_error` is
/// its exit code not being 0.
pub(crate) fn answer_line(graphs: &mut Graphs, line: &str, stop: &AtomicBool) -> Outcome {
    answer_with(graphs, line, stop, collin_run::locate)
}

/// `answer_line` with the search for collin handed in, which a test replaces
/// by a stand-in instead of changing the process's PATH.
fn answer_with(graphs: &mut Graphs, line: &str, stop: &AtomicBool, locate: impl FnOnce() -> collin_run::Located) -> Outcome {
    let root = graphs.root().to_path_buf();
    let manifest_path = graphs.manifest_path().to_path_buf();
    let catalog = graphs.catalog().map(Path::to_path_buf);
    let (question, output) = match mcp_line::read(line, &root, Some(&manifest_path), catalog.as_deref()) {
        Ok(mcp_line::Read::Ask { question, output }) => (question, output),
        Ok(mcp_line::Read::Help) => return Outcome { out: HELP.to_string(), err: String::new(), is_error: false },
        Err(refusal) => return Outcome { out: String::new(), err: refusal, is_error: true },
    };
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let written = match question {
        Question::Select(query, count) => match graphs.manifest() {
            Ok(graph) => ls::respond(&root, graph, &query, count, output, &mut out, &mut err),
            Err(why) => return Outcome::failed(&why),
        },
        Question::Describe { names, sql } => {
            // As the command line does: before the project is opened.
            if let Some(why) = describe::no_name(&names) {
                return Outcome::failed(why);
            }
            match graphs.base_and_jobs() {
                Ok((base, jobs)) => {
                    kept_warning(base.warning, &mut err);
                    describe::respond_describe(&root, base.graph, Some(&jobs), &names, sql, output, &mut out, &mut err)
                }
                Err(why) => return Outcome::failed(&why),
            }
        }
        Question::Overview { depth } => match graphs.base_and_jobs() {
            Ok((base, jobs)) => {
                kept_warning(base.warning, &mut err);
                describe::respond_overview(&root, &manifest_path, base.graph, Some(&jobs), catalog.as_deref(), depth, output, &mut out, &mut err)
            }
            Err(why) => return Outcome::failed(&why),
        },
        Question::Job { name } => {
            // As the command line does: before the project is opened.
            if let Some(why) = job::no_name(&name) {
                return Outcome::failed(why);
            }
            match graphs.base_and_jobs() {
                Ok((base, jobs)) => {
                    kept_warning(base.warning, &mut err);
                    job::respond_job(&root, base.graph, Some(&jobs), &name, output, &mut out, &mut err)
                }
                Err(why) => return Outcome::failed(&why),
            }
        }
        Question::JobOrder => match graphs.base_and_jobs() {
            Ok((base, jobs)) => {
                kept_warning(base.warning, &mut err);
                job_order::respond_job_order(&root, base.graph, Some(&jobs), output, &mut out, &mut err)
            }
            Err(why) => return Outcome::failed(&why),
        },
        Question::Column(ask) => match graphs.manifest() {
            // The cache is merged into a copy: the kept graph stays the manifest's.
            Ok(graph) => ls_column::respond(&root, &manifest_path, graph.clone(), &ask, output, stop, locate, &mut out, &mut err),
            Err(why) => return Outcome::failed(&why),
        },
    };
    let code = ls::ended(written, &mut err);
    Outcome {
        out: String::from_utf8_lossy(&out).into_owned(),
        err: String::from_utf8_lossy(&err).into_owned(),
        is_error: code != ls::ANSWERED,
    }
}

/// What the load said, such as `catalog.json ignored: ...`, which the command
/// line prints to stderr before its answer.
fn kept_warning(warning: Option<&str>, err: &mut Vec<u8>) {
    if let Some(line) = warning {
        let _ = writeln!(err, "{line}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp_graphs::Loads;
    use std::io::BufReader;
    use std::sync::mpsc::{Receiver, Sender};
    use std::time::Duration;

    const WAIT: Duration = Duration::from_secs(30);

    /// Reads what a test sends, blocking like a pipe, and ends when the
    /// sender is dropped.
    struct PipeIn {
        chunks: Receiver<Vec<u8>>,
        held: Vec<u8>,
        at: usize,
    }

    impl Read for PipeIn {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if self.at == self.held.len() {
                match self.chunks.recv() {
                    Ok(chunk) => (self.held, self.at) = (chunk, 0),
                    Err(_) => return Ok(0),
                }
            }
            let n = buf.len().min(self.held.len() - self.at);
            buf[..n].copy_from_slice(&self.held[self.at..self.at + n]);
            self.at += n;
            Ok(n)
        }
    }

    /// What the server writes: whatever one flush holds goes to the test.
    struct PipeOut {
        buffer: Vec<u8>,
        sent: Sender<String>,
    }

    impl Write for PipeOut {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.buffer.extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            let text = String::from_utf8(std::mem::take(&mut self.buffer)).expect("the server writes UTF-8");
            let _ = self.sent.send(text);
            Ok(())
        }
    }

    /// A server running on a thread, and the client's end of its pipes.
    struct Session {
        to: Option<Sender<Vec<u8>>>,
        from: Receiver<String>,
        done: Receiver<()>,
        loads: Arc<Mutex<Vec<Loads>>>,
    }

    impl Session {
        fn start(root: &Path) -> Session {
            Session::with(root, answer_line)
        }

        /// `answer` stands in for `answer_line`; it is handed the same
        /// arguments. The graphs' loads after each call are kept.
        fn with(root: &Path, answer: impl Fn(&mut Graphs, &str, &AtomicBool) -> Outcome + Send + 'static) -> Session {
            let (to, chunks) = mpsc::channel();
            let (sent, from) = mpsc::channel();
            let (finished, done) = mpsc::channel();
            let loads = Arc::new(Mutex::new(Vec::new()));
            let kept = loads.clone();
            let graphs = Graphs::new(root.canonicalize().unwrap(), None, None);
            std::thread::spawn(move || {
                let input = BufReader::new(PipeIn { chunks, held: Vec::new(), at: 0 });
                serve_on(input, PipeOut { buffer: Vec::new(), sent }, graphs, move |g, line, stop| {
                    let outcome = answer(g, line, stop);
                    kept.lock().unwrap().push(g.loads());
                    outcome
                });
                let _ = finished.send(());
            });
            Session { to: Some(to), from, done, loads }
        }

        fn raw(&self, bytes: &[u8]) {
            self.to.as_ref().unwrap().send(bytes.to_vec()).unwrap();
        }

        fn send(&self, message: Value) {
            self.raw(format!("{message}\n").as_bytes());
        }

        fn request(&self, id: impl Into<Value>, method: &str, params: Value) {
            self.send(json!({"jsonrpc": "2.0", "id": id.into(), "method": method, "params": params}));
        }

        fn notify(&self, method: &str, params: Value) {
            self.send(json!({"jsonrpc": "2.0", "method": method, "params": params}));
        }

        fn call(&self, id: impl Into<Value>, line: &str) {
            self.request(id, "tools/call", json!({"name": "edith", "arguments": {"line": line}}));
        }

        /// The next message, which must be one line of JSON.
        fn recv(&self) -> Value {
            let text = self.from.recv_timeout(WAIT).expect("the server answered");
            assert!(text.ends_with('\n') && text.matches('\n').count() == 1, "one line, no raw newline: {text:?}");
            let message: Value = serde_json::from_str(&text).expect("a line of JSON");
            assert_eq!(message["jsonrpc"], "2.0", "{message}");
            message
        }

        fn silent(&self) {
            assert!(self.from.recv_timeout(Duration::from_millis(300)).is_err(), "the server wrote something");
        }

        /// Closes stdin and waits for the server to end.
        fn close(&mut self) {
            self.to = None;
            self.done.recv_timeout(WAIT).expect("the server ended");
        }

        fn ask(&self, line: &str) -> Value {
            self.call(1, line);
            self.recv()
        }
    }

    /// The text blocks of a call's result.
    fn blocks(response: &Value) -> Vec<String> {
        response["result"]["content"].as_array().unwrap().iter().map(|b| b["text"].as_str().unwrap().to_string()).collect()
    }

    fn is_error(response: &Value) -> bool {
        response["result"]["isError"].as_bool().unwrap()
    }

    fn code(response: &Value) -> i64 {
        response["error"]["code"].as_i64().unwrap_or_else(|| panic!("not an error: {response}"))
    }

    fn project(tag: &str) -> PathBuf {
        crate::mcp_graphs::tests::project(&format!("mcp-{tag}"))
    }

    /// Without the clock: the manifest line carries its age.
    fn without_ages(text: &str) -> String {
        text.lines().filter(|l| !l.starts_with("manifest:")).collect::<Vec<_>>().join("\n")
    }

    #[test]
    fn a_line_is_read_whole_with_a_cr_dropped_and_an_empty_one_skipped() {
        let mut s = Session::start(&project("framing"));
        s.raw(b"\n");
        s.raw(b"   \r\n");
        s.raw(b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}\r\n");
        assert_eq!(s.recv(), json!({"jsonrpc": "2.0", "id": 1, "result": {}}));
        // Two messages in one chunk, and one split across chunks.
        s.raw(b"{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"ping\"}\n{\"jsonrpc\":\"2.0\",\"id\":\"b\",");
        s.raw(b"\"method\":\"ping\"}\n");
        assert_eq!(s.recv()["id"], 2);
        assert_eq!(s.recv()["id"], "b", "an id keeps its type");
        // The last line with no newline is a message too.
        s.raw(b"{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"ping\"}");
        s.close();
        assert_eq!(s.recv()["id"], 3);
    }

    #[test]
    fn what_is_not_a_request_is_answered_with_its_code() {
        let s = Session::start(&project("invalid"));
        s.raw(b"{not json\n");
        let said = s.recv();
        assert_eq!((code(&said), &said["id"]), (-32700, &Value::Null));
        s.raw(&[b'{', 0xff, 0xfe, b'}', b'\n']);
        assert_eq!(code(&s.recv()), -32700, "not UTF-8");
        s.raw(b"[{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}]\n");
        let said = s.recv();
        assert_eq!((code(&said), said["error"]["message"].as_str().unwrap()), (-32600, "batches are not supported"));
        s.raw(b"5\n");
        assert_eq!(code(&s.recv()), -32600);
        s.send(json!({"jsonrpc": "2.0", "id": 7}));
        let said = s.recv();
        assert_eq!((code(&said), &said["id"]), (-32600, &json!(7)), "the id it had is the one answered");
        s.send(json!({"jsonrpc": "1.0", "id": 8, "method": "ping"}));
        assert_eq!(code(&s.recv()), -32600);
        s.send(json!({"jsonrpc": "2.0", "id": null, "method": "ping"}));
        assert_eq!(code(&s.recv()), -32600, "an id is a string or a number");
        s.send(json!({"jsonrpc": "2.0", "id": [1], "method": "ping"}));
        assert_eq!(code(&s.recv()), -32600);
        // None of that ended the server.
        s.request(9, "ping", json!({}));
        assert_eq!(s.recv()["id"], 9);
    }

    #[test]
    fn a_line_over_a_mebibyte_is_answered_and_skipped_and_the_next_is_read() {
        let s = Session::start(&project("long"));
        let mut long = br#"{"jsonrpc":"2.0","id":1,"method":"ping","params":{"x":""#.to_vec();
        long.extend(std::iter::repeat(b'a').take(MAX_MESSAGE + 10));
        long.extend_from_slice(b"\"}}\n");
        s.raw(&long);
        let said = s.recv();
        assert_eq!((code(&said), &said["id"]), (-32600, &Value::Null));
        s.request(2, "ping", json!({}));
        assert_eq!(s.recv()["id"], 2);
        s.silent();
    }

    #[test]
    fn a_line_is_read_at_most_to_the_limit_before_it_is_refused() {
        // 10 MB of one line: only the limit and one more byte are held.
        let mut input = BufReader::new(io::repeat(b'a').take(10_000_000));
        let mut line = Vec::new();
        assert!(matches!(read_message(&mut input, &mut line, 1024).unwrap(), Line::TooLong));
        assert!(line.capacity() < 64 * 1024, "{}", line.capacity());
        assert!(matches!(read_message(&mut input, &mut line, 1024).unwrap(), Line::Eof), "the rest was skipped");
        // On the limit: a line of `max` bytes is read, one more is not.
        let mut input = std::io::Cursor::new(b"abcd\nabcde\nfg\n".to_vec());
        assert!(matches!(read_message(&mut input, &mut line, 4).unwrap(), Line::Message));
        assert_eq!(line, b"abcd");
        assert!(matches!(read_message(&mut input, &mut line, 4).unwrap(), Line::TooLong));
        assert!(matches!(read_message(&mut input, &mut line, 4).unwrap(), Line::Message));
        assert_eq!(line, b"fg");
    }

    #[test]
    fn a_notification_is_never_answered() {
        let s = Session::start(&project("notified"));
        s.notify("notifications/initialized", json!({}));
        s.notify("notifications/something/else", json!({"x": 1}));
        s.notify("ping", json!({}));
        s.notify("tools/call", json!({"name": "edith", "arguments": {"line": "--overview"}}));
        s.notify("notifications/cancelled", json!({"requestId": 99}));
        s.silent();
        s.request(1, "ping", json!({}));
        assert_eq!(s.recv()["id"], 1);
    }

    #[test]
    fn initialize_answers_the_clients_version_when_it_is_served_and_the_latest_when_not() {
        let s = Session::start(&project("initialize"));
        for (asked, answered) in [
            ("2025-11-25", "2025-11-25"),
            ("2025-06-18", "2025-06-18"),
            ("2025-03-26", "2025-03-26"),
            ("2024-11-05", "2025-11-25"),
            ("2099-01-01", "2025-11-25"),
        ] {
            s.request(
                1,
                "initialize",
                json!({"protocolVersion": asked, "capabilities": {}, "clientInfo": {"name": "t", "version": "0"}}),
            );
            let said = s.recv();
            let result = &said["result"];
            assert_eq!(result["protocolVersion"], answered, "{asked}");
            assert_eq!(result["capabilities"], json!({"tools": {}}));
            assert_eq!(result["serverInfo"], json!({"name": "dbt-edith", "version": env!("CARGO_PKG_VERSION")}));
            assert_eq!(result["instructions"], INSTRUCTIONS);
            // Legacy only: nothing of the modern era is written.
            for key in ["resultType", "ttlMs", "cacheScope", "_meta"] {
                assert!(result.get(key).is_none(), "{key}");
            }
        }
        s.notify("notifications/initialized", json!({}));
        s.request(2, "initialize", json!({}));
        assert_eq!(s.recv()["result"]["protocolVersion"], "2025-11-25", "no params at all");
        s.silent();
    }

    #[test]
    fn ping_gives_an_empty_result_and_an_unknown_method_is_not_found() {
        let s = Session::start(&project("methods"));
        s.request("p", "ping", json!({}));
        assert_eq!(s.recv(), json!({"jsonrpc": "2.0", "id": "p", "result": {}}));
        // The modern probe: its refusal is what sends a client to initialize.
        s.request(2, "server/discover", json!({"_meta": {"io.modelcontextprotocol/protocolVersion": "2026-07-28"}}));
        let said = s.recv();
        assert_eq!((code(&said), said["error"]["message"].as_str().unwrap()), (-32601, "Method not found: server/discover"));
        for method in ["resources/list", "prompts/list", "logging/setLevel", "notifications/initialized"] {
            s.request(3, method, json!({}));
            assert_eq!(code(&s.recv()), -32601, "{method}");
        }
    }

    #[test]
    fn the_tool_list_is_one_tool_under_its_byte_ceiling_and_the_same_every_time() {
        let s = Session::start(&project("list"));
        s.request(1, "tools/list", json!({}));
        let first = s.recv();
        s.request(2, "tools/list", Value::Null);
        s.request(3, "tools/list", json!({"cursor": null}));
        let (second, third) = (s.recv(), s.recv());
        for later in [&second, &third] {
            assert_eq!(later["result"], first["result"]);
        }
        let result = first["result"].as_object().unwrap();
        assert_eq!(result.keys().collect::<Vec<_>>(), ["tools"], "no resultType, ttlMs or cacheScope");
        let tools = result["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["name"], "edith");
        assert_eq!(tools[0]["annotations"], json!({"openWorldHint": false}));
        assert_eq!(tools[0]["inputSchema"]["type"], "object");
        let bytes = result["tools"].to_string().len();
        assert!(bytes <= 512, "the tools array is {bytes} bytes");
        s.request(4, "tools/list", json!({"cursor": "next"}));
        assert_eq!(code(&s.recv()), -32602, "there is no second page");
    }

    #[test]
    fn a_call_that_is_not_well_formed_is_a_protocol_error_or_a_tool_error_as_the_spec_says() {
        let s = Session::start(&project("invalid-call"));
        s.request(1, "tools/call", json!({"name": "other", "arguments": {}}));
        let said = s.recv();
        assert_eq!((code(&said), said["error"]["message"].as_str().unwrap()), (-32602, "Unknown tool: other"));
        s.request(2, "tools/call", json!({"arguments": {}}));
        assert_eq!(code(&s.recv()), -32602);
        s.request(3, "tools/call", Value::Null);
        assert_eq!(code(&s.recv()), -32602);
        s.request(4, "tools/call", json!({"name": "edith", "arguments": {"line": 5}}));
        assert_eq!(code(&s.recv()), -32602);
        s.request(5, "tools/call", json!({"name": "edith", "arguments": "--overview"}));
        assert_eq!(code(&s.recv()), -32602);
        // A schema mismatch the model can fix is a tool error.
        s.request(6, "tools/call", json!({"name": "edith", "arguments": {"line": "--overview", "output": "json"}}));
        let said = s.recv();
        assert!(is_error(&said));
        assert!(blocks(&said)[0].contains("not `output`") && blocks(&said)[0].ends_with("`help` lists every flag."), "{said}");
    }

    #[test]
    fn help_is_asked_with_no_line_an_empty_one_or_the_word() {
        let s = Session::start(&project("help"));
        s.request(1, "tools/call", json!({"name": "edith"}));
        s.request(2, "tools/call", json!({"name": "edith", "arguments": {}}));
        s.call(3, "");
        s.call(4, "help");
        s.call(5, "--describe orders --help");
        for _ in 0..5 {
            let said = s.recv();
            assert_eq!(blocks(&said), [HELP], "{said}");
            assert!(!is_error(&said));
        }
        // Nothing was read to say so.
        assert!(s.loads.lock().unwrap().iter().all(|l| *l == Loads::default()));
    }

    #[test]
    fn a_line_that_is_not_a_question_is_a_tool_error_in_one_block() {
        let s = Session::start(&project("refused"));
        for line in ["--overview | cut -f1", "--overview --port 4321", "/tmp --overview", "--output json", "--sql"] {
            let said = s.ask(line);
            assert!(is_error(&said), "{line}");
            let blocks = blocks(&said);
            assert_eq!(blocks.len(), 1, "{line}");
            assert!(blocks[0].starts_with("error: ") && blocks[0].ends_with("`help` lists every flag."), "{line}: {blocks:?}");
        }
        assert_eq!(s.loads.lock().unwrap().len(), 5);
        assert!(s.loads.lock().unwrap().iter().all(|l| *l == Loads::default()), "a refused line reads nothing");
    }

    #[test]
    fn an_answer_is_the_command_lines_stdout_then_its_stderr() {
        let dir = project("answer");
        let root = dir.canonicalize().unwrap();
        let s = Session::start(&dir);
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let (_, _, graph) = ls::open(&root, None).unwrap();
        let query = ls::Query::Line { select: "+orders".to_string(), exclude: String::new() };
        assert_eq!(ls::respond(&root, &graph, &query, None, ls::Output::Name, &mut out, &mut err).unwrap(), 0);
        let said = s.ask("-s '+orders'");
        let got = blocks(&said);
        assert_eq!(got.len(), 2, "{said}");
        assert!(!is_error(&said));
        assert_eq!(got[0], String::from_utf8(out).unwrap());
        assert_eq!(got[0], "orders\nstg_orders\n");
        assert!(got[1].starts_with("manifest:"), "{}", got[1]);
        assert_eq!(without_ages(&got[1]), without_ages(&String::from_utf8(err).unwrap()));
        // Again, from the graph kept: one load for both.
        s.ask("-s stg_orders");
        s.ask("--selector marts");
        assert_eq!(s.loads.lock().unwrap().last().unwrap().manifest, 1);
        assert_eq!(s.loads.lock().unwrap().last().unwrap().base, 0);
    }

    #[test]
    fn a_selection_is_counted_over_the_line_when_it_asks() {
        let dir = project("count");
        let s = Session::start(&dir);
        let said = s.ask("-s '+orders' --count-by kind");
        assert!(!is_error(&said), "{said}");
        assert_eq!(blocks(&said)[0], "count\tkind\n2\tmodel\n2\t(all)\n");
    }

    #[test]
    fn a_description_is_answered_from_the_base_graph_the_way_the_command_line_does() {
        let dir = project("describe");
        let root = dir.canonicalize().unwrap();
        let s = Session::start(&dir);
        let (_, _, graph) = ls::open_with_catalog(&root, None, None).unwrap();
        let names = ["helper".to_string(), "orders".to_string()];
        let (mut out, mut err) = (Vec::new(), Vec::new());
        describe::respond_describe(&root, &graph, None, &names, false, ls::Output::Name, &mut out, &mut err).unwrap();
        let said = s.ask("--describe helper,orders");
        let got = blocks(&said);
        assert_eq!(got[0], String::from_utf8(out).unwrap());
        assert!(got[0].contains("BIGINT"), "the catalog is merged: {}", got[0]);
        assert_eq!(without_ages(&got[1]), without_ages(&String::from_utf8(err).unwrap()));
        let said = s.ask("--overview --output json");
        assert!(!is_error(&said));
        assert!(blocks(&said)[0].contains("\"project\":\"jaffle_shop\""), "{said}");
    }

    #[test]
    fn the_jobs_of_the_sources_file_are_answered_over_a_line_as_on_the_command_line() {
        let dir = project("jobs");
        let root = dir.canonicalize().unwrap();
        let s = Session::start(&dir);
        // No sources file: a card says nothing of jobs, and the question cannot be asked.
        assert!(!blocks(&s.ask("--describe orders"))[0].contains("jobs"));
        let said = s.ask("--job daily");
        assert!(is_error(&said) && said.to_string().contains("no .dbt-edith/jobs.yml in this project"), "{said}");
        let said = s.ask("--job-order");
        assert!(is_error(&said) && said.to_string().contains("no .dbt-edith/jobs.yml in this project"), "{said}");

        std::fs::create_dir_all(dir.join("dags")).unwrap();
        std::fs::write(
            dir.join("dags/daily.yml"),
            "dag_id: daily\nschedule: \"0 5 * * *\"\ntasks:\n  - task_id: marts\n    command: dbt run --select orders\n",
        )
        .unwrap();
        std::fs::create_dir_all(dir.join(".dbt-edith")).unwrap();
        let source = crate::settings::JobSource { kind: "airflow".into(), path: "dags".into(), ..Default::default() };
        std::fs::write(dir.join(".dbt-edith/jobs.yml"), crate::jobs::sources_yaml(&[source])).unwrap();
        let (_, _, graph) = ls::open_with_catalog(&root, None, None).unwrap();

        let (mut out, mut err) = (Vec::new(), Vec::new());
        describe::respond_describe(&root, &graph, None, &["orders".to_string()], false, ls::Output::Name, &mut out, &mut err).unwrap();
        let card = String::from_utf8(out).unwrap();
        assert!(card.contains("  jobs (1)"), "{card}");
        assert_eq!(blocks(&s.ask("--describe orders"))[0], card);

        for output in [ls::Output::Name, ls::Output::Json] {
            let (mut out, mut err) = (Vec::new(), Vec::new());
            crate::job::respond_job(&root, &graph, None, "daily", output, &mut out, &mut err).unwrap();
            let line = if output == ls::Output::Json { "--job daily --output json" } else { "--job daily" };
            let said = s.ask(line);
            assert!(!is_error(&said), "{said}");
            assert_eq!(blocks(&said)[0], String::from_utf8(out).unwrap());
        }
        for output in [ls::Output::Name, ls::Output::Json] {
            let (mut out, mut err) = (Vec::new(), Vec::new());
            crate::job_order::respond_job_order(&root, &graph, None, output, &mut out, &mut err).unwrap();
            let line = if output == ls::Output::Json { "--job-order --output json" } else { "--job-order" };
            let said = s.ask(line);
            assert!(!is_error(&said), "{said}");
            assert_eq!(blocks(&said)[0], String::from_utf8(out).unwrap());
        }
        let said = s.ask("--job nothing");
        assert!(is_error(&said) && said.to_string().contains("no job named nothing"), "{said}");
        let overview = blocks(&s.ask("--overview"))[0].clone();
        assert!(overview.lines().any(|l| l.starts_with("jobs ") && l.ends_with(" 1 from .dbt-edith/jobs.yml: daily")), "{overview}");
    }

    #[test]
    fn a_catalog_that_does_not_read_is_the_first_line_of_stderr_as_on_the_command_line() {
        let dir = project("warning");
        std::fs::write(dir.join("target/catalog.json"), "{ not json").unwrap();
        let s = Session::start(&dir);
        let got = blocks(&s.ask("--describe orders"));
        assert!(got[1].starts_with("  catalog.json ignored: "), "{}", got[1]);
        assert!(got[1].contains("manifest:"), "{}", got[1]);
    }

    #[test]
    fn the_exit_codes_are_the_error_flag() {
        let dir = project("codes");
        let s = Session::start(&dir);
        // 0, an empty answer included: the stdout block is the one left out.
        let said = s.ask("-s tag:nothing_has_this");
        assert!(!is_error(&said), "{said}");
        let got = blocks(&said);
        assert_eq!(got.len(), 1, "an empty block is left out: {said}");
        assert!(got[0].contains("manifest:"), "{got:?}");
        // 1, REFUSED: the cards that could be printed come first.
        let said = s.ask("--describe orders,nope");
        assert!(is_error(&said));
        let got = blocks(&said);
        assert!(got[0].contains("orders") && got[1].contains("nope"), "{got:?}");
        let said = s.ask("-s 'method:'");
        assert!(is_error(&said), "{said}");
        // 2, CANNOT_RUN: a cache that is not there.
        let said = s.ask("--column orders.order_id");
        assert!(is_error(&said), "{said}");
        assert!(blocks(&said).last().unwrap().contains("error:"), "{said}");
    }

    #[test]
    fn a_describe_with_no_name_is_the_command_lines_one_line_before_the_project_is_opened() {
        let dir = project("noname");
        // No manifest: opening the project would say so instead.
        std::fs::remove_file(dir.join("target/manifest.json")).unwrap();
        let s = Session::start(&dir);
        for line in ["--describe ''", "--describe ,", "--describe ' , '"] {
            let said = s.ask(line);
            assert!(is_error(&said), "{line}");
            assert_eq!(blocks(&said), ["error: --describe wants the name or the unique_id of a node\n"], "{line}");
        }
    }

    #[test]
    fn a_missing_manifest_is_each_calls_error_and_names_the_servers_arguments_not_a_flag() {
        let dir = project("nomanifest");
        std::fs::remove_file(dir.join("target/manifest.json")).unwrap();
        let s = Session::start(&dir);
        for line in ["--overview", "-s orders", "--describe orders", "--column orders.order_id"] {
            let said = s.ask(line);
            assert!(is_error(&said), "{line}");
            let got = blocks(&said);
            assert_eq!(got.len(), 1, "{line}: {got:?}");
            assert!(got[0].starts_with("error: no manifest at "), "{line}: {got:?}");
            assert!(got[0].contains("name it in the arguments the server was started with") && !got[0].contains("--manifest"), "{got:?}");
        }
        // Mended, the next call answers: the server was never the worse for it.
        std::fs::write(dir.join("target/manifest.json"), "{\"nodes\":{}}").unwrap();
        assert!(!is_error(&s.ask("--overview")));
    }

    #[test]
    fn nothing_is_read_before_a_call_needs_a_graph() {
        let dir = project("lazy");
        let mut s = Session::start(&dir);
        s.request(1, "initialize", json!({"protocolVersion": "2025-11-25"}));
        s.notify("notifications/initialized", json!({}));
        s.request(2, "tools/list", json!({}));
        s.request(3, "ping", json!({}));
        s.call(4, "help");
        s.call(5, "--overview --port 1");
        for _ in 0..5 {
            s.recv();
        }
        assert!(s.loads.lock().unwrap().iter().all(|l| *l == Loads::default()));
        s.call(6, "--overview");
        s.recv();
        assert_eq!(s.loads.lock().unwrap().last().unwrap(), &Loads { manifest: 1, base: 1 });
        s.close();
    }

    #[test]
    fn a_panic_in_a_call_is_an_internal_error_and_the_next_call_is_answered() {
        let dir = project("panic");
        let mut s = Session::with(&dir, |g, line, stop| {
            if line == "boom" {
                g.manifest().unwrap();
                panic!("a call went wrong");
            }
            answer_line(g, line, stop)
        });
        s.call(1, "--overview");
        assert!(!is_error(&s.recv()));
        s.call("two", "boom");
        let said = s.recv();
        assert_eq!((code(&said), &said["id"]), (-32603, &json!("two")));
        assert_eq!(said["error"]["message"], "internal error: a call went wrong");
        s.call(3, "-s orders");
        let said = s.recv();
        assert!(!is_error(&said));
        assert_eq!(blocks(&said)[0], "orders\n");
        // The graphs were let go, so the call after it read them again.
        assert_eq!(s.loads.lock().unwrap().last().unwrap().manifest, 2, "read again after being forgotten");
        s.close();
    }

    #[test]
    fn a_poisoned_lock_does_not_stop_the_writing() {
        let wire = Wire::new(Vec::new());
        let poisoned = std::thread::scope(|scope| {
            scope
                .spawn(|| {
                    let _out = wire.out.lock().unwrap();
                    let _state = wire.state.lock().unwrap();
                    panic!("holding both");
                })
                .join()
        });
        assert!(poisoned.is_err() && wire.out.is_poisoned() && wire.state.is_poisoned());
        wire.emit(&json!({"jsonrpc": "2.0", "id": 1, "result": {}}));
        wire.cancel("1");
        assert_eq!(wire.out.lock().unwrap_or_else(PoisonError::into_inner).as_slice(), b"{\"id\":1,\"jsonrpc\":\"2.0\",\"result\":{}}\n");
    }

    /// A call that waits for the test to say go, or for its own stop flag.
    struct Gate {
        go: Arc<AtomicBool>,
        stopped: Arc<AtomicBool>,
        started: Arc<Mutex<Vec<String>>>,
    }

    impl Gate {
        fn new() -> Gate {
            Gate { go: Arc::default(), stopped: Arc::default(), started: Arc::default() }
        }

        /// `wait` blocks until released or stopped; any other line is answered
        /// as it is.
        fn session(&self, root: &Path) -> Session {
            let (go, stopped, started) = (self.go.clone(), self.stopped.clone(), self.started.clone());
            Session::with(root, move |g, line, stop| {
                started.lock().unwrap().push(line.to_string());
                if !line.starts_with("wait") {
                    return answer_line(g, line, stop);
                }
                let begun = std::time::Instant::now();
                while !go.load(Ordering::Relaxed) && !stop.load(Ordering::Relaxed) && begun.elapsed() < WAIT {
                    std::thread::sleep(Duration::from_millis(5));
                }
                stopped.store(stop.load(Ordering::Relaxed), Ordering::Relaxed);
                Outcome { out: format!("{line}\n"), err: String::new(), is_error: false }
            })
        }
    }

    #[test]
    fn initialize_ping_and_the_tool_list_are_answered_while_a_call_runs() {
        let gate = Gate::new();
        let s = gate.session(&project("busy"));
        s.call("slow", "wait");
        while gate.started.lock().unwrap().is_empty() {
            std::thread::sleep(Duration::from_millis(5));
        }
        s.request(1, "ping", json!({}));
        assert_eq!(s.recv()["id"], 1, "the call is still running");
        s.request(2, "initialize", json!({"protocolVersion": "2025-11-25"}));
        assert_eq!(s.recv()["id"], 2);
        s.request(3, "tools/list", json!({}));
        assert_eq!(s.recv()["id"], 3);
        s.request(4, "tools/call", json!({"name": "nope"}));
        assert_eq!(s.recv()["id"], 4, "a wrong call is refused at once too");
        gate.go.store(true, Ordering::Relaxed);
        let said = s.recv();
        assert_eq!((&said["id"], blocks(&said)), (&json!("slow"), vec!["wait\n".to_string()]));
    }

    #[test]
    fn calls_are_answered_in_the_order_they_came() {
        let gate = Gate::new();
        let s = gate.session(&project("order"));
        s.call(1, "wait");
        s.call(2, "-s orders");
        s.call(3, "-s stg_orders");
        gate.go.store(true, Ordering::Relaxed);
        let ids: Vec<Value> = (0..3).map(|_| s.recv()["id"].clone()).collect();
        assert_eq!(ids, [json!(1), json!(2), json!(3)]);
    }

    #[test]
    fn a_cancelled_call_is_stopped_and_writes_no_response() {
        let gate = Gate::new();
        let mut s = gate.session(&project("cancel"));
        s.call(10, "wait");
        while gate.started.lock().unwrap().is_empty() {
            std::thread::sleep(Duration::from_millis(5));
        }
        s.notify("notifications/cancelled", json!({"requestId": 10, "reason": "the user stopped it"}));
        // The next call is answered, which it cannot be before the first ended.
        s.call(11, "-s orders");
        let said = s.recv();
        assert_eq!(said["id"], 11, "no response for 10: {said}");
        assert!(gate.stopped.load(Ordering::Relaxed), "the call saw its stop flag");
        s.silent();
        s.close();
    }

    #[test]
    fn a_request_named_cancelled_is_answered_not_taken_for_the_notification() {
        let gate = Gate::new();
        let mut s = gate.session(&project("cancel-request"));
        s.call(1, "wait");
        while gate.started.lock().unwrap().is_empty() {
            std::thread::sleep(Duration::from_millis(5));
        }
        // It has an id: a request for a method the server does not have.
        s.request("c", "notifications/cancelled", json!({"requestId": 1}));
        let said = s.recv();
        assert_eq!((&said["id"], &said["error"]["code"]), (&json!("c"), &json!(METHOD_NOT_FOUND)), "{said}");
        assert!(!gate.stopped.load(Ordering::Relaxed), "the call was not stopped");
        gate.go.store(true, Ordering::Relaxed);
        assert_eq!(s.recv()["id"], 1);
        s.close();
    }

    #[test]
    fn a_call_cancelled_while_it_waits_is_never_run() {
        let gate = Gate::new();
        let mut s = gate.session(&project("cancel-queued"));
        s.call(1, "wait");
        s.call("two", "wait skipped");
        s.call(3, "-s orders");
        while gate.started.lock().unwrap().is_empty() {
            std::thread::sleep(Duration::from_millis(5));
        }
        s.notify("notifications/cancelled", json!({"requestId": "two"}));
        // A request the server does not hold, answered already, or a number
        // for a string: nothing happens.
        s.notify("notifications/cancelled", json!({"requestId": 2}));
        s.notify("notifications/cancelled", json!({"requestId": 999}));
        s.notify("notifications/cancelled", json!({}));
        s.request("sync", "ping", json!({}));
        assert_eq!(s.recv()["id"], "sync");
        gate.go.store(true, Ordering::Relaxed);
        assert_eq!(s.recv()["id"], 1);
        assert_eq!(s.recv()["id"], 3);
        assert_eq!(*gate.started.lock().unwrap(), ["wait", "-s orders"]);
        // The id is free again once its call is over.
        s.call("two", "-s orders");
        assert_eq!(s.recv()["id"], "two");
        s.close();
    }

    #[test]
    fn the_end_of_input_stops_the_call_running_and_ends_the_server() {
        let gate = Gate::new();
        let mut s = gate.session(&project("eof"));
        s.call(1, "wait");
        while gate.started.lock().unwrap().is_empty() {
            std::thread::sleep(Duration::from_millis(5));
        }
        s.close();
        assert!(gate.stopped.load(Ordering::Relaxed));
    }

    #[test]
    fn the_calls_sent_before_the_input_closed_are_still_answered() {
        let mut s = Session::start(&project("piped"));
        s.call(1, "-s orders");
        s.call(2, "--overview");
        s.close();
        assert_eq!(s.recv()["id"], 1);
        assert_eq!(s.recv()["id"], 2);
    }

    #[cfg(unix)]
    mod collin {
        use super::*;
        use std::os::unix::fs::PermissionsExt;

        /// A collin that is a script in `dir`, handed to the call in place of
        /// the search of the PATH, which the other tests would see changed.
        struct StandIn {
            dir: PathBuf,
        }

        impl StandIn {
            fn new(dir: &Path, body: &str) -> StandIn {
                let bin = dir.join("collin");
                let script = format!(
                    "#!/bin/sh\n[ \"$1\" = --version ] && echo 'collin 0.10.0' && exit 0\nout=''\nreport=''\n\
                     while [ $# -gt 0 ]; do [ \"$1\" = --out ] && out=\"$2\"; [ \"$1\" = --report ] && report=\"$2\"; shift; done\n\
                     here=\"$(dirname \"$0\")\"\n{body}\n"
                );
                crate::collin_run::tests::executable(&bin, &script);
                assert!(std::fs::metadata(&bin).unwrap().permissions().mode() & 0o111 != 0);
                StandIn { dir: dir.to_path_buf() }
            }

            /// A server whose calls find this collin.
            fn serving(&self, root: &Path) -> Session {
                let bin = self.dir.join("collin");
                Session::with(root, move |g, line, stop| {
                    answer_with(g, line, stop, || collin_run::Located { bin: Some(bin.clone()), ..Default::default() })
                })
            }
        }

        /// Every file under the project, with its size.
        fn tree(dir: &Path) -> Vec<(String, u64)> {
            fn walk(dir: &Path, base: &Path, into: &mut Vec<(String, u64)>) {
                for entry in std::fs::read_dir(dir).unwrap().flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        walk(&path, base, into);
                    } else {
                        into.push((path.strip_prefix(base).unwrap().display().to_string(), entry.metadata().unwrap().len()));
                    }
                }
            }
            let mut found = Vec::new();
            walk(dir, dir, &mut found);
            found.sort();
            found
        }

        fn alive(pid: &str) -> bool {
            std::process::Command::new("kill").args(["-0", pid]).stderr(std::process::Stdio::null()).status().unwrap().success()
        }

        /// The stand-in's pid, once it has started.
        fn started(bin_dir: &Path) -> String {
            let begun = std::time::Instant::now();
            while !bin_dir.join("started").exists() {
                assert!(begun.elapsed() < WAIT, "collin never started");
                std::thread::sleep(Duration::from_millis(10));
            }
            std::fs::read_to_string(bin_dir.join("pid")).unwrap().trim().to_string()
        }

        fn gone(pid: &str) {
            let begun = std::time::Instant::now();
            while alive(pid) {
                assert!(begun.elapsed() < Duration::from_secs(5), "collin {pid} was not killed");
                std::thread::sleep(Duration::from_millis(20));
            }
        }

        const OLD_CACHE: &str = r#"{"version":1,"source":"collin","producer":"collin 0.10.0","edges":[]}"#;

        /// A project whose cache stands, and a collin that starts, writes part
        /// of a file and waits.
        fn waiting(tag: &str) -> (PathBuf, PathBuf) {
            let dir = project(tag);
            std::fs::write(dir.join("target/column_lineage.collin.json"), OLD_CACHE).unwrap();
            let bin = dir.join("bin");
            std::fs::create_dir_all(&bin).unwrap();
            (dir, bin)
        }

        // The pid is written before `started` appears: `started()` reads it as soon as
        // that file exists.
        const WAITS: &str = "echo partial > \"$out\"\necho $$ > \"$here/pid\"\n: > \"$here/started\"\nexec sleep 60";

        #[test]
        fn a_cancelled_run_of_collin_is_killed_and_leaves_the_old_cache_and_no_temporary_file() {
            let (dir, bin) = waiting("collin-cancel");
            let stand = StandIn::new(&bin, WAITS);
            let mut s = stand.serving(&dir);
            s.call(7, "--column orders.order_id --run-collin");
            let pid = started(&bin);
            assert!(alive(&pid));
            s.notify("notifications/cancelled", json!({"requestId": 7}));
            gone(&pid);
            // The worker is free again, and the cancelled call wrote nothing.
            s.call(8, "help");
            assert_eq!(s.recv()["id"], 8);
            s.silent();
            assert_eq!(std::fs::read_to_string(dir.join("target/column_lineage.collin.json")).unwrap(), OLD_CACHE);
            let left: Vec<_> = tree(&dir).into_iter().filter(|(p, _)| p.ends_with(".tmp")).collect();
            assert!(left.is_empty(), "{left:?}");
            s.close();
        }

        #[test]
        fn the_end_of_input_kills_a_run_of_collin_and_the_server_ends() {
            let (dir, bin) = waiting("collin-eof");
            let stand = StandIn::new(&bin, WAITS);
            let mut s = stand.serving(&dir);
            s.call(1, "--column orders.order_id --run-collin");
            let pid = started(&bin);
            s.close();
            gone(&pid);
            assert_eq!(std::fs::read_to_string(dir.join("target/column_lineage.collin.json")).unwrap(), OLD_CACHE);
            assert!(tree(&dir).iter().all(|(p, _)| !p.ends_with(".tmp")));
        }

        #[test]
        fn a_collin_that_is_killed_from_outside_is_a_tool_error_and_the_server_goes_on() {
            let (dir, bin) = waiting("collin-killed");
            let stand = StandIn::new(&bin, "kill -9 $$");
            let mut s = stand.serving(&dir);
            let said = s.ask("--column orders.order_id --run-collin");
            assert!(is_error(&said), "{said}");
            let got = blocks(&said);
            assert!(got.last().unwrap().contains("collin"), "{got:?}");
            assert_eq!(std::fs::read_to_string(dir.join("target/column_lineage.collin.json")).unwrap(), OLD_CACHE);
            assert!(!is_error(&s.ask("-s orders")), "the server answers the next call");
            s.close();
        }

        #[test]
        fn a_run_of_collin_changes_exactly_the_two_files_beside_the_manifest() {
            let dir = project("collin-writes");
            let bin = dir.join("bin");
            std::fs::create_dir_all(&bin).unwrap();
            let writes = format!(
                "echo '{}' > \"$out\"\necho '{{\"totals\":{{\"models\":1,\"parsed\":1}},\"models\":[]}}' > \"$report\"",
                OLD_CACHE
            );
            let stand = StandIn::new(&bin, &writes);
            let s = stand.serving(&dir);
            let before = tree(&dir);
            let said = s.ask("--column orders.order_id --run-collin");
            assert!(!is_error(&said), "{said}");
            let after = tree(&dir);
            let changed: Vec<_> = after.iter().filter(|f| !before.contains(f)).map(|(p, _)| p.as_str()).collect();
            assert_eq!(changed, ["target/collin.report.json", "target/column_lineage.collin.json"]);
            assert!(before.iter().all(|(p, _)| after.iter().any(|(q, _)| p == q)), "nothing was removed");
            assert!(blocks(&said).last().unwrap().contains("collin 0.10.0 wrote"), "{said}");
        }
    }
}
