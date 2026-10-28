//! The `--job-order` mode: the dbt dependencies between steps of jobs that
//! start on their own which the job files do not order, printed, and the
//! process ends (0125).
//!
//! Invariants: it reads the jobs as `--job` does (`jobs::known`, the project's
//! own `.dbt-edith/jobs.yml`, one read), so both answer over the same files and
//! no word of the line becomes a path. A step is a consumer of the built nodes
//! it runs (a model that is not a view or ephemeral, a seed, a snapshot) and a
//! producer of the same; tests are neither. Views and ephemerals are always
//! walked through to the built nodes behind them, whoever selects them, and a
//! node the step builds itself ends the walk, since dbt orders inside a step.
//! "Ordered" is what the files can say and nothing else: it is an ancestor in a
//! graph of tasks, sensors and starts, never a guess from a clock. Every list is
//! sorted by names, so the same files give the same answer (0055).

use crate::graph::{Graph, Kind, Node};
use crate::job::clean;
use crate::jobs::{self, Known, Read, Start, AIRFLOW, DBT_CLOUD};
use crate::ls::{self, Output, ANSWERED, CANNOT_RUN};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::io::{self, BufWriter, Write};
use std::path::Path;

/// How many edges a pair lists in text; JSON lists all.
const EDGES_SHOWN: usize = 5;

/// How many left-out jobs the text names.
const NAMES_SHOWN: usize = 5;

/// The whole mode, from the command line's arguments to the exit code.
pub fn run_job_order(project: &Path, manifest: Option<&Path>, catalog: Option<&Path>, output: Output) -> i32 {
    let (root, _, graph) = match ls::open_with_catalog(project, manifest, catalog) {
        Ok(opened) => opened,
        Err(why) => return ls::cannot_open(&why),
    };
    let mut err = io::stderr().lock();
    let mut out = BufWriter::new(io::stdout().lock());
    let written = respond_job_order(&root, &graph, None, output, &mut out, &mut err);
    ls::ended(written, &mut err)
}

/// What the mode does once the project is open. `kept` is the jobs a caller
/// holds between calls (MCP); None reads them here, if the project has a
/// sources file.
pub(crate) fn respond_job_order(
    root: &Path,
    graph: &Graph,
    kept: Option<&Known>,
    output: Output,
    out: &mut impl Write,
    err: &mut impl Write,
) -> io::Result<i32> {
    let code = job_order(root, graph, kept, output, out, err)?;
    out.flush()?;
    let _ = ls::provenance(root, graph, err);
    Ok(code)
}

fn job_order(root: &Path, graph: &Graph, kept: Option<&Known>, output: Output, out: &mut impl Write, err: &mut impl Write) -> io::Result<i32> {
    let known = kept.cloned().unwrap_or_else(|| jobs::known(graph, root));
    let read = match known {
        Known::Absent => {
            writeln!(err, "error: no {} in this project; name the sources in the Jobs settings", clean(jobs::SOURCES_FILE))?;
            return Ok(CANNOT_RUN);
        }
        Known::Refused(why) => {
            writeln!(err, "error: {} is not read, {}", jobs::SOURCES_FILE, clean(&why))?;
            return Ok(CANNOT_RUN);
        }
        Known::Read(read) => read,
    };
    let answer = order(graph, &read);
    match output {
        Output::Json => {
            serde_json::to_writer(&mut *out, &answer)?;
            writeln!(out)?;
        }
        // A list is read, so compact has nothing to drop from it: both print
        // the text.
        Output::Name | Output::Compact => write_text(&answer, out)?,
    }
    Ok(ANSWERED)
}

/* -------------------------------------------------------------- answer -- */

/// Which kind of pair it is, in the order the answer lists them.
#[derive(serde::Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Group {
    /// Nothing the files say puts one before the other.
    #[serde(rename = "none")]
    Unordered,
    /// The two jobs have clocks, and not the same one: only the clock may.
    #[serde(rename = "clock")]
    Clock,
    /// The files order them, the other way round.
    #[serde(rename = "reversed")]
    Reversed,
}

/// One end of a pair: a step, and the job it belongs to.
#[derive(serde::Serialize, Clone, Debug)]
pub struct Side {
    pub kind: &'static str,
    pub job: String,
    pub source: String,
    pub label: String,
    pub schedule: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub cron: String,
    /// Empty for UTC, and for dbt Cloud, whose crons are UTC.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub timezone: String,
    pub file: String,
    pub line: usize,
    /// `DAG shop_daily › task marts`, for the text.
    #[serde(skip)]
    title: String,
}

#[derive(serde::Serialize, Clone, Debug)]
pub struct NodeName {
    pub id: String,
    pub name: String,
}

/// A model reading a parent that another step builds.
#[derive(serde::Serialize, Debug)]
pub struct Edge {
    pub node: String,
    pub node_name: String,
    pub parent: String,
    pub parent_name: String,
    /// The views and ephemerals between them, nearest the model first.
    pub through: Vec<NodeName>,
}

#[derive(serde::Serialize, Debug)]
pub struct Pair {
    group: Group,
    pub step: Side,
    /// The steps that build what it reads, one of which it should wait for.
    pub waits_for: Vec<Side>,
    pub edges: Vec<Edge>,
    /// Those of `waits_for` the files run after the step.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub later: Vec<Side>,
    /// Sensors of the step's DAG that wait for one of `waits_for`, and that
    /// the step does not depend on.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub idle_sensors: Vec<SensorNote>,
    /// Sensors the step depends on that wait for a task group of a DAG holding
    /// one of `waits_for`: the group may order the pair, and its tasks are not
    /// read.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub group_sensors: Vec<GroupSensor>,
}

/// A sensor a pair has something to say of. The text's notes are built from
/// these.
#[derive(serde::Serialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SensorNote {
    /// Absent for a sensor written without an id, which no task can name.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub id: String,
    /// What it waits for, as the Jobs tab words it (`dag › task`).
    pub waits: Vec<String>,
}

#[derive(serde::Serialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct GroupSensor {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub id: String,
    pub waits: Vec<String>,
    /// The step's task that depends on it, `task build`.
    pub task: String,
}

fn who(id: &str) -> String {
    if id.is_empty() {
        "a sensor".to_string()
    } else {
        format!("sensor {id}")
    }
}

#[derive(serde::Serialize, Debug, Default)]
pub struct Counts {
    pub none: usize,
    pub clock: usize,
    pub reversed: usize,
}

/// A step that could not be resolved and could build a node.
#[derive(serde::Serialize, Debug)]
pub struct Unresolved {
    pub source: String,
    pub job: String,
    pub label: String,
    /// The command as parsed.
    pub shown: String,
    pub file: String,
    pub line: usize,
    /// Why it was not resolved.
    pub why: String,
}

/// A job nothing starts, which neither side of a pair is taken from.
#[derive(serde::Serialize, Debug)]
pub struct LeftOut {
    pub source: String,
    pub job: String,
}

#[derive(serde::Serialize, Debug)]
pub struct OrderAnswer {
    /// The sources file names at least one source: an empty answer is told from
    /// no job read at all.
    pub configured: bool,
    /// How many steps were looked at: those of jobs that start on their own
    /// that build a node. The text says what an empty answer holds by it.
    #[serde(skip)]
    checked: usize,
    pub counts: Counts,
    pub pairs: Vec<Pair>,
    /// In-scope steps that could not be resolved, which could hide more pairs.
    pub unresolved: Vec<Unresolved>,
    pub left_out: Vec<LeftOut>,
    /// DAGs whose `depends_on` loops, which Airflow does not load: left out too.
    pub cyclic: Vec<LeftOut>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_read: Option<jobs::Unread>,
}

/* ----------------------------------------------------------------- nodes -- */

/// Materializations a run of dbt neither reads from nor leaves current, because
/// the warehouse refreshes them on its own lag (a dynamic table) or they are no
/// object (a view, an ephemeral): they are walked through. A materialized view
/// is not one: dbt refreshes it on every run, so its step is the one a reader
/// waits for.
fn refreshes_itself(materialized: &str) -> bool {
    matches!(materialized, "ephemeral" | "view" | "" | "dynamic_table")
}

/// A node a run leaves an object for: a model that is not a view, an
/// ephemeral or a dynamic table, a seed, a snapshot; not
/// disabled. A model whose materialization the manifest leaves out is dbt's
/// default, a view.
fn built(n: &Node) -> bool {
    !n.disabled
        && match n.kind {
            Kind::Seed | Kind::Snapshot => true,
            Kind::Model => !refreshes_itself(&n.materialized),
            _ => false,
        }
}

/// A view reads its parents live and an ephemeral is inlined into whoever
/// reads it; a dynamic table follows its parents by itself. So what they read
/// is read through them, and what builds them is no one to wait for.
fn passes(n: &Node) -> bool {
    !n.disabled && n.kind == Kind::Model && refreshes_itself(&n.materialized)
}

/// What a step needs before it, found by walking up from the nodes it builds.
struct Need {
    consumer: u32,
    parent: u32,
    through: Vec<u32>,
}

/* ---------------------------------------------------------------- order -- */

fn words(n: usize) -> usize {
    n.div_ceil(64)
}

fn has(bits: &[u64], i: usize) -> bool {
    bits[i / 64] >> (i % 64) & 1 == 1
}

/// The graph of what the files run before what: one point per dbt step, per
/// task with no dbt command, per sensor and per feeder DAG, and one per job for
/// its start. `plain[x]` are the points x runs after. A DAG scheduled on assets
/// instead starts after the first update of one of each asset it waits on, so
/// its start holds, per asset, the points that update it (`groups`): it is
/// after what ALL of an asset's updaters are after, and after every asset.
struct Order {
    plain: Vec<Vec<usize>>,
    groups: Vec<Vec<Vec<usize>>>,
    /// Where the sensors of each job are.
    sensors: Vec<Vec<usize>>,
    /// Jobs whose `depends_on` loops, which Airflow does not load.
    cyclic: Vec<bool>,
    /// 0 not met, 1 on the walk's stack, 2 settled.
    state: Vec<u8>,
    before: Vec<Vec<u64>>,
}

/// Whether tasks that wait for one another (`(x, p)`: x after p) loop back on
/// themselves, which Airflow refuses to load. Kahn's algorithm: tasks leave as
/// all they wait for has left, and what is left holds a loop. A wait on a point
/// that is not one of `tasks` (a sensor, a name nothing has) is already met.
/// Linear in tasks and waits, and a property of the graph, so it does not
/// depend on where a walk starts.
fn loops(tasks: &[usize], waits: &[(usize, usize)]) -> bool {
    let at: HashMap<usize, usize> = tasks.iter().enumerate().map(|(i, &t)| (t, i)).collect();
    let mut open = vec![0usize; tasks.len()];
    let mut after: Vec<Vec<usize>> = vec![Vec::new(); tasks.len()];
    for &(x, p) in waits {
        if let (Some(&xi), Some(&pi)) = (at.get(&x), at.get(&p)) {
            open[xi] += 1;
            after[pi].push(xi);
        }
    }
    let mut ready: Vec<usize> = (0..tasks.len()).filter(|&i| open[i] == 0).collect();
    let mut gone = 0;
    while let Some(i) = ready.pop() {
        gone += 1;
        for &x in &after[i] {
            open[x] -= 1;
            if open[x] == 0 {
                ready.push(x);
            }
        }
    }
    gone < tasks.len()
}

impl Order {
    fn new(read: &Read) -> Order {
        let (ns, nj) = (read.steps.len(), read.jobs.len());
        // Steps first, so a step's index is its point; then one start per job.
        let mut total = ns + nj;
        let mut link_at: Vec<Vec<usize>> = Vec::new();
        let mut sensor_at: Vec<Vec<usize>> = Vec::new();
        for j in &read.jobs {
            link_at.push((0..j.links.len()).map(|_| { total += 1; total - 1 }).collect());
            sensor_at.push((0..j.sensors.len()).map(|_| { total += 1; total - 1 }).collect());
        }
        let feeder_at: Vec<usize> = (0..read.feeders.len()).map(|_| { total += 1; total - 1 }).collect();
        let mut o = Order {
            plain: vec![Vec::new(); total],
            groups: vec![Vec::new(); total],
            sensors: sensor_at,
            cyclic: vec![false; nj],
            state: vec![0; total],
            before: vec![Vec::new(); total],
        };
        let start = |j: usize| ns + j;
        // The points of a job a name stands for, for `depends_on` and sensors.
        // Only looked up, never walked (0055).
        let mut named: HashMap<(usize, &str), Vec<usize>> = HashMap::new();
        let mut producers: HashMap<&str, Vec<usize>> = HashMap::new();
        for (ji, j) in read.jobs.iter().enumerate() {
            for &si in &j.steps {
                if !read.steps[si].name.is_empty() {
                    named.entry((ji, read.steps[si].name.as_str())).or_default().push(si);
                }
                for a in &read.steps[si].outlets {
                    let list = producers.entry(a.as_str()).or_default();
                    if list.last() != Some(&si) {
                        list.push(si);
                    }
                }
            }
            for (k, l) in j.links.iter().enumerate() {
                let p = link_at[ji][k];
                if !l.name.is_empty() {
                    named.entry((ji, l.name.as_str())).or_default().push(p);
                }
                for a in &l.outlets {
                    let list = producers.entry(a.as_str()).or_default();
                    if list.last() != Some(&p) {
                        list.push(p);
                    }
                }
            }
            for (k, s) in j.sensors.iter().enumerate() {
                if !s.id.is_empty() {
                    named.entry((ji, s.id.as_str())).or_default().push(o.sensors[ji][k]);
                }
            }
        }
        for (fi, f) in read.feeders.iter().enumerate() {
            for a in &f.outlets {
                let list = producers.entry(a.as_str()).or_default();
                if list.last() != Some(&feeder_at[fi]) {
                    list.push(feeder_at[fi]);
                }
            }
        }
        let waits_on = |assets: &[String]| -> Vec<Vec<usize>> {
            assets.iter().filter_map(|a| producers.get(a.as_str())).filter(|p| !p.is_empty()).cloned().collect()
        };
        for (ji, j) in read.jobs.iter().enumerate() {
            if j.kind == DBT_CLOUD {
                for (k, &si) in j.steps.iter().enumerate() {
                    let prev = if k == 0 { start(ji) } else { j.steps[k - 1] };
                    o.plain[si].push(prev);
                }
                if let Some(last) = j.after_job.and_then(|a| read.jobs[a].steps.last()) {
                    o.plain[start(ji)].push(*last);
                }
                continue;
            }
            let tasks: Vec<usize> = j.steps.iter().copied().chain(link_at[ji].iter().copied()).chain(o.sensors[ji].iter().copied()).collect();
            for &x in &tasks {
                o.plain[x].push(start(ji));
            }
            let deps = j.steps.iter().map(|&si| (si, &read.steps[si].upstream)).chain(j.links.iter().enumerate().map(|(k, l)| (link_at[ji][k], &l.upstream)));
            let mut waits: Vec<(usize, usize)> = Vec::new();
            for (x, upstream) in deps {
                for u in upstream {
                    for &p in named.get(&(ji, u.as_str())).into_iter().flatten().filter(|&&p| p != x) {
                        o.plain[x].push(p);
                        waits.push((x, p));
                    }
                }
            }
            o.cyclic[ji] = loops(&j.steps.iter().copied().chain(link_at[ji].iter().copied()).collect::<Vec<_>>(), &waits);
            for (k, s) in j.sensors.iter().enumerate() {
                let x = o.sensors[ji][k];
                // A sensor on its own DAG waits for a previous run, not for
                // anything of this one.
                for (ki, other) in read.jobs.iter().enumerate().filter(|(_, o)| o.kind == AIRFLOW && o.name == s.dag && o.name != j.name) {
                    // Whole run only when it names neither a task nor a group; a
                    // group's tasks are not read, so it orders nothing.
                    if s.whole() {
                        o.plain[x].extend(other.steps.iter().copied().chain(link_at[ki].iter().copied()).chain(o.sensors[ki].iter().copied()));
                    } else {
                        for t in &s.tasks {
                            o.plain[x].extend(named.get(&(ki, t.as_str())).into_iter().flatten().filter(|&&p| p != x));
                        }
                    }
                }
            }
            // A DAG with no dbt task is one point: a sensor on any of its
            // tasks, or on its whole run, is after all of it.
            for (k, s) in j.sensors.iter().enumerate().filter(|(_, s)| s.dag != j.name && (s.whole() || !s.tasks.is_empty())) {
                let x = o.sensors[ji][k];
                o.plain[x].extend(read.feeders.iter().enumerate().filter(|(_, f)| f.dag == s.dag).map(|(fi, _)| feeder_at[fi]));
            }
            let groups = waits_on(&j.assets);
            o.groups[start(ji)] = groups;
        }
        for (fi, f) in read.feeders.iter().enumerate() {
            o.groups[feeder_at[fi]] = waits_on(&f.assets);
        }
        o
    }

    /// Works out the points `root` runs after, and those they do.
    fn settle(&mut self, root: usize) {
        // One bitset per point, so memory grows with the square of the points
        // (about 50 MB at 20 000): accepted.
        if self.state[root] == 2 {
            return;
        }
        self.state[root] = 1;
        let mut stack: Vec<(usize, usize)> = vec![(root, 0)];
        while let Some(&(x, i)) = stack.last() {
            let next = self.plain[x].iter().copied().chain(self.groups[x].iter().flatten().copied()).nth(i);
            if let Some(dep) = next {
                if let Some(top) = stack.last_mut() {
                    top.1 += 1;
                }
                if self.state[dep] == 0 {
                    self.state[dep] = 1;
                    stack.push((dep, 0));
                }
                continue;
            }
            stack.pop();
            let before = self.gather(x);
            self.before[x] = before;
            self.state[x] = 2;
        }
    }

    /// What x runs after, once everything it depends on is settled. A point met
    /// again on the walk's stack is a loop between jobs, through sensors or
    /// assets: it adds nothing, so the walk ends there. Every point is settled
    /// in the same order before any question, so the answer does not depend on
    /// which pair is looked at first. A loop inside one job never gets here:
    /// that job is left out (`Order::cyclic`).
    fn gather(&self, x: usize) -> Vec<u64> {
        let n = self.state.len();
        let mut set = vec![0u64; words(n)];
        let mark = |set: &mut Vec<u64>, i: usize| set[i / 64] |= 1 << (i % 64);
        for &p in self.plain[x].iter().filter(|&&p| self.state[p] == 2) {
            for (w, b) in set.iter_mut().zip(&self.before[p]) {
                *w |= b;
            }
            mark(&mut set, p);
        }
        for group in &self.groups[x] {
            // A run may start on the first update: only what every updater is
            // after is before it.
            if group.iter().any(|&t| self.state[t] != 2) {
                continue;
            }
            let mut common: Option<Vec<u64>> = None;
            for &t in group {
                let mut mine = self.before[t].clone();
                mark(&mut mine, t);
                common = Some(match common {
                    None => mine,
                    Some(c) => c.iter().zip(&mine).map(|(a, b)| a & b).collect(),
                });
            }
            for (w, b) in set.iter_mut().zip(common.iter().flatten()) {
                *w |= b;
            }
        }
        set
    }

    /// Whether `x` runs after `t`, both settled.
    fn after(&self, x: usize, t: usize) -> bool {
        has(&self.before[x], t)
    }
}

/* ----------------------------------------------------------------- clock -- */

/// A cron with its spaces collapsed and an Airflow preset spelled out, so two
/// ways of writing one clock compare equal.
fn normal_cron(cron: &str) -> String {
    let one = cron.split_whitespace().collect::<Vec<_>>().join(" ");
    match one.as_str() {
        "@hourly" => "0 * * * *".into(),
        "@daily" | "@midnight" => "0 0 * * *".into(),
        "@weekly" => "0 0 * * 0".into(),
        "@monthly" => "0 0 1 * *".into(),
        "@yearly" | "@annually" => "0 0 1 1 *".into(),
        _ => one,
    }
}

/// What a job says of itself, worked out once.
struct Info {
    kind: &'static str,
    label: String,
    source: String,
    schedule: String,
    cron: String,
    timezone: String,
    file: String,
}

struct Infos<'a> {
    read: &'a Read,
    held: HashMap<usize, Info>,
}

impl Infos<'_> {
    fn of(&mut self, ji: usize) -> &Info {
        let read = self.read;
        self.held.entry(ji).or_insert_with(|| {
            let v = jobs::job_view(read, ji);
            let mut schedule = v.start_line();
            // The zone beside the cron it reads: a clock is not one without it.
            if !v.timezone.is_empty() && !v.cron.is_empty() {
                if let Some(rest) = schedule.strip_prefix("scheduled ").filter(|r| r.starts_with(&v.cron)) {
                    schedule = format!("scheduled {} {}{}", v.cron, v.timezone, &rest[v.cron.len()..]);
                }
            }
            Info { kind: v.kind, label: v.label(), source: v.source.clone(), schedule, cron: v.cron.clone(), timezone: v.timezone.clone(), file: v.file.clone() }
        })
    }

    fn side(&mut self, si: usize) -> Side {
        let read = self.read;
        let ji = read.steps[si].job;
        let label = jobs::step_label(read, si);
        let line = read.steps[si].line;
        let info = self.of(ji);
        Side {
            kind: info.kind,
            job: read.jobs[ji].name.clone(),
            source: info.source.clone(),
            title: format!("{} › {label}", info.label),
            label,
            schedule: info.schedule.clone(),
            cron: info.cron.clone(),
            timezone: info.timezone.clone(),
            file: info.file.clone(),
            line,
        }
    }

    /// Whether a clock is all that could put these two jobs in order: both
    /// have a cron, and not the same one in the same zone.
    fn clocks_differ(&mut self, a: usize, b: usize) -> bool {
        let mut clock = |ji: usize| {
            let i = self.of(ji);
            (!i.cron.is_empty()).then(|| (normal_cron(&i.cron), if i.timezone.is_empty() { "UTC".to_string() } else { i.timezone.clone() }))
        };
        match (clock(a), clock(b)) {
            (Some(x), Some(y)) => x != y,
            _ => false,
        }
    }
}

/* ---------------------------------------------------------------- the walk -- */

/// Every step of a job that starts on its own that builds a node whose parent
/// another such step builds, and the files do not run the other first.
pub fn order(graph: &Graph, read: &Read) -> OrderAnswer {
    let node = |i: u32| &graph.nodes[i as usize];
    let mut infos = Infos { read, held: HashMap::new() };
    let starts = |ji: usize| read.jobs[ji].start != Start::No;
    let mut order = Order::new(read);
    // A DAG whose `depends_on` loops is not one Airflow loads: it is no side.
    let in_scope: Vec<bool> = read.steps.iter().map(|s| s.executes() && starts(s.job) && !order.cyclic[s.job]).collect();

    let checked = (0..read.steps.len()).filter(|&i| in_scope[i] && read.steps[i].nodes.iter().any(|&n| built(node(n)))).count();

    // Who builds what. Steps are numbered in the files' own order, so each
    // list is sorted.
    let mut runners: HashMap<u32, Vec<usize>> = HashMap::new();
    for (si, s) in read.steps.iter().enumerate().filter(|(i, _)| in_scope[*i]) {
        for &n in s.nodes.iter().filter(|&&n| built(node(n))) {
            runners.entry(n).or_default().push(si);
        }
    }

    // What each step needs: its pairs of (step, who builds what it reads).
    let mut found: BTreeMap<(usize, Vec<usize>), Vec<Need>> = BTreeMap::new();
    for si in (0..read.steps.len()).filter(|&i| in_scope[i]) {
        let s = &read.steps[si];
        for &m in s.nodes.iter().filter(|&&m| built(node(m))) {
            let mut seen: HashSet<u32> = HashSet::from([m]);
            // Breadth first, so the way through is the shortest.
            let mut queue: VecDeque<(u32, Vec<u32>)> = VecDeque::from([(m, Vec::new())]);
            while let Some((x, through)) = queue.pop_front() {
                for &p in &node(x).parents {
                    if !seen.insert(p) {
                        continue;
                    }
                    let pn = node(p);
                    if built(pn) {
                        // dbt orders what one step builds.
                        if s.nodes.binary_search(&p).is_ok() {
                            continue;
                        }
                        if let Some(rs) = runners.get(&p) {
                            found.entry((si, rs.clone())).or_default().push(Need { consumer: m, parent: p, through: through.clone() });
                        }
                        continue;
                    }
                    if passes(pn) {
                        let mut deeper = through.clone();
                        deeper.push(p);
                        queue.push_back((p, deeper));
                    }
                }
            }
        }
    }

    let mut keyed: Vec<((Group, String, String, usize, usize, Vec<(String, String, usize, usize)>), Pair)> = Vec::new();
    if !found.is_empty() {
        for x in 0..order.state.len() {
            order.settle(x);
        }
        for ((si, rs), needs) in found {
            if rs.iter().any(|&t| order.after(si, t)) {
                continue;
            }
            let later: Vec<usize> = rs.iter().copied().filter(|&t| order.after(t, si)).collect();
            let ji = read.steps[si].job;
            let group = if later.len() == rs.len() {
                Group::Reversed
            } else if rs.iter().any(|&t| read.steps[t].job != ji && infos.clocks_differ(ji, read.steps[t].job)) {
                Group::Clock
            } else {
                Group::Unordered
            };
            // A sensor of this DAG that the files put after a step it should
            // wait for, and that this task does not wait on: it would order
            // the pair if wired, and it gates only what names it.
            let mut idle: Vec<SensorNote> = Vec::new();
            for (k, sensor) in read.jobs[ji].sensors.iter().enumerate() {
                let point = order.sensors[ji][k];
                if rs.iter().any(|&t| order.after(point, t)) && !order.after(si, point) {
                    idle.push(SensorNote { id: sensor.id.clone(), waits: sensor.waits() });
                }
            }
            idle.sort();
            idle.dedup();
            // A sensor on a task group of a runner's DAG, which this step depends
            // on: the group may well hold the runner, and its tasks are not read.
            let step_label = jobs::step_label(read, si);
            let mut grouped: Vec<GroupSensor> = Vec::new();
            for (k, sensor) in read.jobs[ji].sensors.iter().enumerate().filter(|(_, s)| !s.group.is_empty()) {
                let holds = rs.iter().any(|&t| {
                    let other = &read.jobs[read.steps[t].job];
                    other.kind == AIRFLOW && other.name == sensor.dag
                });
                if holds && order.after(si, order.sensors[ji][k]) {
                    grouped.push(GroupSensor { id: sensor.id.clone(), waits: sensor.waits(), task: step_label.clone() });
                }
            }
            grouped.sort();
            grouped.dedup();
            let mut edges: Vec<Edge> = needs
                .iter()
                .map(|n| Edge {
                    node: node(n.consumer).id.clone(),
                    node_name: node(n.consumer).name.clone(),
                    parent: node(n.parent).id.clone(),
                    parent_name: node(n.parent).name.clone(),
                    through: n.through.iter().map(|&t| NodeName { id: node(t).id.clone(), name: node(t).name.clone() }).collect(),
                })
                .collect();
            edges.sort_by(|a, b| {
                (&a.node_name, &a.parent_name, a.through.iter().map(|t| &t.name).collect::<Vec<_>>(), &a.node, &a.parent)
                    .cmp(&(&b.node_name, &b.parent_name, b.through.iter().map(|t| &t.name).collect::<Vec<_>>(), &b.node, &b.parent))
            });
            let by = |s: usize| {
                let j = &read.jobs[read.steps[s].job];
                (j.name.clone(), j.file.clone(), j.line, read.steps[s].n)
            };
            let mut key_runners: Vec<(String, String, usize, usize)> = rs.iter().map(|&t| by(t)).collect();
            key_runners.sort();
            let (jn, jf, jl, n) = by(si);
            let pair = Pair {
                group,
                step: infos.side(si),
                waits_for: rs.iter().map(|&t| infos.side(t)).collect(),
                edges,
                later: later.iter().map(|&t| infos.side(t)).collect(),
                idle_sensors: idle,
                group_sensors: grouped,
            };
            keyed.push(((group, jn, jf, jl, n, key_runners), pair));
        }
    }
    keyed.sort_by(|a, b| a.0.cmp(&b.0));
    let pairs: Vec<Pair> = keyed.into_iter().map(|(_, p)| p).collect();
    let count = |g: Group| pairs.iter().filter(|p| p.group == g).count();
    let counts = Counts { none: count(Group::Unordered), clock: count(Group::Clock), reversed: count(Group::Reversed) };

    // What could hide more: a step that could build a node and was not read.
    let mut unresolved: Vec<Unresolved> = Vec::new();
    for (si, s) in read.steps.iter().enumerate().filter(|(_, s)| starts(s.job) && !s.unresolved.is_empty()) {
        let could = s.inv.as_ref().is_none_or(|inv| {
            inv.verb.executes() && inv.kinds().is_some_and(|k| k.iter().any(|k| matches!(k, Kind::Model | Kind::Seed | Kind::Snapshot)))
        });
        if could {
            let source = infos.of(s.job).source.clone();
            let file = infos.of(s.job).file.clone();
            unresolved.push(Unresolved {
                source,
                job: read.jobs[s.job].name.clone(),
                label: jobs::step_label(read, si),
                shown: s.shown.clone(),
                file,
                line: s.line,
                why: s.unresolved.clone(),
            });
        }
    }
    unresolved.sort_by(|a, b| (&a.job, &a.source, &a.label).cmp(&(&b.job, &b.source, &b.label)));
    let mut left_out: Vec<LeftOut> = Vec::new();
    for (ji, j) in read.jobs.iter().enumerate().filter(|(ji, _)| !starts(*ji)) {
        if j.steps.iter().any(|&si| read.steps[si].executes()) {
            left_out.push(LeftOut { source: infos.of(ji).source.clone(), job: j.name.clone() });
        }
    }
    left_out.sort_by(|a, b| (&a.job, &a.source).cmp(&(&b.job, &b.source)));
    left_out.dedup_by(|a, b| a.job == b.job && a.source == b.source);
    let mut cyclic: Vec<LeftOut> = Vec::new();
    for (ji, j) in read.jobs.iter().enumerate().filter(|(ji, _)| starts(*ji) && order.cyclic[*ji]) {
        if j.steps.iter().any(|&si| read.steps[si].executes()) {
            cyclic.push(LeftOut { source: infos.of(ji).source.clone(), job: j.name.clone() });
        }
    }
    cyclic.sort_by(|a, b| (&a.job, &a.source).cmp(&(&b.job, &b.source)));
    cyclic.dedup_by(|a, b| a.job == b.job && a.source == b.source);

    OrderAnswer { configured: !read.sources.is_empty(), checked, counts, pairs, unresolved, left_out, cyclic, not_read: jobs::unread(read) }
}

/* ----------------------------------------------------------------- text -- */

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

fn write_text(a: &OrderAnswer, out: &mut impl Write) -> io::Result<()> {
    let c = &a.counts;
    if a.pairs.is_empty() {
        writeln!(out, "pairs 0")?;
        // Only what holds is said: a step not looked at waits for nothing known.
        if !a.configured {
            // `jobs: []`, worded as --describe and --overview word it.
            writeln!(out, "no sources in {}", jobs::SOURCES_FILE)?;
        } else if a.checked == 0 {
            writeln!(out, "no step of a job that starts on its own builds a node")?;
        } else if !a.unresolved.is_empty() {
            writeln!(out, "no pair among the steps resolved")?;
        } else {
            writeln!(out, "every step waits for the steps that build its parents")?;
        }
    } else {
        writeln!(out, "pairs {}: {} nothing orders them, {} only the clock, {} the other way", a.pairs.len(), c.none, c.clock, c.reversed)?;
    }
    let mut last: Option<Group> = None;
    for p in &a.pairs {
        if last != Some(p.group) {
            let (title, n) = match p.group {
                Group::Unordered => ("nothing orders them", c.none),
                Group::Clock => ("only the clock orders them", c.clock),
                Group::Reversed => ("ordered the other way", c.reversed),
            };
            writeln!(out, "{title}  {n}")?;
            last = Some(p.group);
        }
        let wait = if p.waits_for.len() == 1 { "should wait for" } else { "should wait for one of" };
        let them: Vec<&str> = p.waits_for.iter().map(|s| s.title.as_str()).collect();
        writeln!(out, "{}", clean(&format!("  {}  {wait}  {}", p.step.title, them.join(", "))))?;
        let others: Vec<&str> = p.waits_for.iter().map(|s| s.schedule.as_str()).collect();
        writeln!(out, "{}", clean(&format!("    {}  ·  {}", p.step.schedule, others.join(", "))))?;
        for s in &p.later {
            writeln!(out, "{}", clean(&format!("    {} runs after it", s.title)))?;
        }
        for s in &p.idle_sensors {
            writeln!(out, "{}", clean(&format!("    note: {} of this DAG waits for {}; this task does not depend on it", who(&s.id), s.waits.join(", "))))?;
        }
        for s in &p.group_sensors {
            writeln!(out, "{}", clean(&format!("    note: {} depends on {}, which waits for {}, whose tasks are not read", s.task, who(&s.id), s.waits.join(", "))))?;
        }
        for e in p.edges.iter().take(EDGES_SHOWN) {
            let through = if e.through.is_empty() {
                String::new()
            } else {
                format!(" through {}", e.through.iter().map(|t| t.name.as_str()).collect::<Vec<_>>().join(", "))
            };
            writeln!(out, "{}", clean(&format!("    {} reads {}{through}", e.node_name, e.parent_name)))?;
        }
        if p.edges.len() > EDGES_SHOWN {
            writeln!(out, "    +{} more", p.edges.len() - EDGES_SHOWN)?;
        }
    }
    if !a.unresolved.is_empty() {
        writeln!(out, "{} not resolved, which could hide more", plural(a.unresolved.len(), "step", "steps"))?;
    }
    for c in &a.cyclic {
        let name = if a.cyclic.iter().filter(|o| o.job == c.job).count() > 1 { format!("{}: {}", c.source, c.job) } else { c.job.clone() };
        writeln!(out, "{}", clean(&format!("DAG {name} has a depends_on cycle, which Airflow does not load: left out")))?;
    }
    if !a.left_out.is_empty() {
        // Named with its source when two sources hold a job of one name, so the
        // count and the names agree.
        let mut names: Vec<String> = a
            .left_out
            .iter()
            .map(|l| if a.left_out.iter().filter(|o| o.job == l.job).count() > 1 { format!("{}: {}", l.source, l.job) } else { l.job.clone() })
            .collect();
        let more = names.len().saturating_sub(NAMES_SHOWN);
        names.truncate(NAMES_SHOWN);
        let tail = if more > 0 { format!(", +{more} more") } else { String::new() };
        let verb = if a.left_out.len() == 1 { "job nothing starts" } else { "jobs nothing starts" };
        writeln!(out, "{}", clean(&format!("{} {verb} left out: {}{tail}", a.left_out.len(), names.join(", "))))?;
    }
    if let Some(u) = &a.not_read {
        writeln!(out, "not read: {}", clean(&u.text()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::RawManifest;
    use crate::settings::JobSource;
    use serde_json::json;

    /// Jaffle Shop in miniature, with every materialization the walk tells
    /// apart: tables, a view, an ephemeral and a seed.
    fn graph() -> Graph {
        let model = |name: &str, dir: &str, materialized: &str| {
            json!({
                "name": name, "resource_type": "model", "package_name": "jaffle_shop",
                "original_file_path": format!("models/{dir}/{name}.sql"),
                "fqn": ["jaffle_shop", dir, name], "config": { "materialized": materialized },
            })
        };
        let raw: RawManifest = serde_json::from_value(json!({
            "metadata": { "project_name": "jaffle_shop" },
            "nodes": {
                "model.jaffle_shop.stg_orders": model("stg_orders", "staging", "table"),
                "model.jaffle_shop.stg_customers": model("stg_customers", "staging", "table"),
                "model.jaffle_shop.int_orders": model("int_orders", "intermediate", "ephemeral"),
                "model.jaffle_shop.v_orders": model("v_orders", "intermediate", "view"),
                "model.jaffle_shop.orders": model("orders", "marts", "table"),
                "model.jaffle_shop.customers": model("customers", "marts", "table"),
                "model.jaffle_shop.order_items": model("order_items", "marts", "table"),
                "model.jaffle_shop.orders_summary": model("orders_summary", "marts", "table"),
                "model.jaffle_shop.orders_live": model("orders_live", "marts", "dynamic_table"),
                "model.jaffle_shop.orders_latest": model("orders_latest", "marts", "table"),
                "model.jaffle_shop.orders_mv": model("orders_mv", "marts", "materialized_view"),
                "model.jaffle_shop.mv_report": model("mv_report", "marts", "table"),
                "seed.jaffle_shop.raw_orders": {
                    "name": "raw_orders", "resource_type": "seed", "package_name": "jaffle_shop",
                    "original_file_path": "seeds/raw_orders.csv", "fqn": ["jaffle_shop", "raw_orders"],
                    "config": { "materialized": "seed" },
                },
            },
            "sources": {
                "source.jaffle_shop.raw.customers": {
                    "name": "customers", "resource_type": "source", "package_name": "jaffle_shop",
                    "source_name": "raw", "original_file_path": "models/staging/sources.yml",
                    "fqn": ["jaffle_shop", "staging", "raw", "customers"],
                },
            },
            "parent_map": {
                "model.jaffle_shop.stg_orders": ["seed.jaffle_shop.raw_orders"],
                "model.jaffle_shop.stg_customers": ["source.jaffle_shop.raw.customers"],
                "model.jaffle_shop.int_orders": ["model.jaffle_shop.stg_orders"],
                "model.jaffle_shop.v_orders": ["model.jaffle_shop.stg_orders"],
                "model.jaffle_shop.orders": ["model.jaffle_shop.int_orders"],
                "model.jaffle_shop.customers": ["model.jaffle_shop.stg_customers", "model.jaffle_shop.stg_orders"],
                "model.jaffle_shop.order_items": ["model.jaffle_shop.v_orders"],
                "model.jaffle_shop.orders_summary": ["model.jaffle_shop.orders"],
                "model.jaffle_shop.orders_live": ["model.jaffle_shop.stg_orders"],
                "model.jaffle_shop.orders_latest": ["model.jaffle_shop.orders_live"],
                "model.jaffle_shop.orders_mv": ["model.jaffle_shop.stg_orders"],
                "model.jaffle_shop.mv_report": ["model.jaffle_shop.orders_mv"],
            },
        }))
        .unwrap();
        Graph::build(raw, Path::new("manifest.json"), 0, 0)
    }

    /// A project whose sources file names the folders the files go in, each
    /// `(kind, folder, timezone)`, and `files` the `(path, text)` written.
    fn project(tag: &str, sources: &[(&str, &str, &str)], files: &[(&str, &str)]) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!("dbt-edith-order-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join(".dbt-edith")).unwrap();
        for (path, text) in files {
            let at = root.join(path);
            std::fs::create_dir_all(at.parent().unwrap()).unwrap();
            std::fs::write(at, text).unwrap();
        }
        let list: Vec<JobSource> = sources
            .iter()
            .map(|(kind, dir, zone)| JobSource { kind: kind.to_string(), path: dir.to_string(), timezone: zone.to_string(), ..Default::default() })
            .collect();
        std::fs::write(root.join(".dbt-edith/jobs.yml"), jobs::sources_yaml(&list)).unwrap();
        root
    }

    fn ask(root: &Path, output: Output) -> (String, String, i32) {
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = job_order(root, &graph(), None, output, &mut out, &mut err).unwrap();
        (String::from_utf8(out).unwrap(), String::from_utf8(err).unwrap(), code)
    }

    /// The answer of the files of one folder of Airflow DAGs.
    fn dags(tag: &str, files: &[(&str, &str)]) -> String {
        let files: Vec<(String, &str)> = files.iter().map(|(n, t)| (format!("dags/{n}.yml"), *t)).collect();
        let borrowed: Vec<(&str, &str)> = files.iter().map(|(p, t)| (p.as_str(), *t)).collect();
        let (out, err, code) = ask(&project(tag, &[("airflow", "dags", "")], &borrowed), Output::Name);
        assert_eq!((code, err.as_str()), (ANSWERED, ""), "{out}");
        // A loop is only ever the point of a test that says so by its tag.
        assert!(tag.starts_with("loop") || !out.contains("depends_on cycle"), "{out}");
        out
    }

    const STAGE: &str = "dag_id: jaffle_stage\nschedule: \"0 6 * * *\"\ntasks:\n  - task_id: stage\n    command: dbt run --select stg_orders\n";
    const MARTS: &str = "dag_id: jaffle_marts\nschedule: \"0 6 * * *\"\ntasks:\n  - task_id: build\n    command: dbt run --select orders\n";

    #[test]
    fn a_step_reading_what_another_dag_builds_with_nothing_between_is_a_pair() {
        let out = dags("plain", &[("stage", STAGE), ("marts", MARTS)]);
        assert_eq!(
            out,
            "pairs 1: 1 nothing orders them, 0 only the clock, 0 the other way\n\
             nothing orders them  1\n\
             \x20 DAG jaffle_marts › task build  should wait for  DAG jaffle_stage › task stage\n\
             \x20   scheduled 0 6 * * *  ·  scheduled 0 6 * * *\n\
             \x20   orders reads stg_orders through int_orders\n"
        );
    }

    #[test]
    fn a_sensor_orders_only_the_tasks_that_name_it() {
        let gated = |depends: &str| {
            format!(
                "dag_id: jaffle_marts\nschedule: \"0 6 * * *\"\ntasks:\n  - task_id: build\n    command: dbt run --select orders\n{depends}\
                 sensors:\n  - task_id: wait_stage\n    external_dag_id: jaffle_stage\n    external_task_id: stage\n"
            )
        };
        assert!(dags("sensor-named", &[("stage", STAGE), ("marts", &gated("    depends_on: [wait_stage]\n"))]).starts_with("pairs 0\n"));
        // The sensor is in the DAG, and the task does not name it: it waits for nothing.
        let out = dags("sensor-idle", &[("stage", STAGE), ("marts", &gated(""))]);
        assert!(out.starts_with("pairs 1:"), "{out}");
        assert!(out.contains("    note: sensor wait_stage of this DAG waits for jaffle_stage › stage; this task does not depend on it\n"), "{out}");
        // A whole-run sensor orders what names it all the same.
        let whole = gated("    depends_on: [wait_stage]\n").replace("    external_task_id: stage\n", "");
        assert!(dags("sensor-whole", &[("stage", STAGE), ("marts", &whole)]).starts_with("pairs 0\n"));
    }

    #[test]
    fn a_task_with_no_dbt_command_in_the_chain_keeps_the_order() {
        let marts = "dag_id: jaffle_marts\nschedule: \"0 6 * * *\"\ntasks:\n  - task_id: notify\n    python_callable: send\n    depends_on: [wait_stage]\n\
                     \x20 - task_id: build\n    command: dbt run --select orders\n    depends_on: [notify]\n\
                     sensors:\n  - task_id: wait_stage\n    external_dag_id: jaffle_stage\n    external_task_id: stage\n";
        assert!(dags("chain", &[("stage", STAGE), ("marts", marts)]).starts_with("pairs 0\n"));
    }

    #[test]
    fn a_dag_on_an_asset_waits_for_the_task_that_updates_it_and_not_for_an_earlier_one() {
        let stage = |tasks: &str| format!("dag_id: jaffle_stage\nschedule: \"0 6 * * *\"\ntasks:\n{tasks}");
        let marts = "dag_id: jaffle_marts\nschedule: [orders_ready]\ntasks:\n  - task_id: build\n    command: dbt run --select orders\n";
        let updates = stage("  - task_id: stage\n    command: dbt run --select stg_orders\n    outlets: [orders_ready]\n");
        assert!(dags("asset", &[("stage", &updates), ("marts", marts)]).starts_with("pairs 0\n"));
        // The asset is updated by a Python task that runs before the dbt one.
        let early = stage(
            "  - task_id: announce\n    python_callable: send\n    outlets: [orders_ready]\n\
             \x20 - task_id: stage\n    command: dbt run --select stg_orders\n    depends_on: [announce]\n",
        );
        let out = dags("asset-early", &[("stage", &early), ("marts", marts)]);
        assert!(out.starts_with("pairs 1:"), "{out}");
    }

    #[test]
    fn several_producers_of_an_asset_order_only_what_all_of_them_are_after() {
        // `orders_ready` is updated by two tasks: a run may start on the first.
        let stage = "dag_id: jaffle_stage\nschedule: \"0 6 * * *\"\ntasks:\n  - task_id: stage\n    command: dbt run --select stg_orders\n    outlets: [orders_ready]\n\
                     \x20 - task_id: other\n    python_callable: send\n    outlets: [orders_ready]\n";
        let marts = "dag_id: jaffle_marts\nschedule: [orders_ready]\ntasks:\n  - task_id: build\n    command: dbt run --select orders\n";
        let out = dags("producers", &[("stage", stage), ("marts", marts)]);
        assert!(out.starts_with("pairs 1:"), "{out}");
        // Both are after the same earlier step, so that one is ordered before.
        let both = "dag_id: jaffle_stage\nschedule: \"0 6 * * *\"\ntasks:\n  - task_id: stage\n    command: dbt run --select stg_orders\n\
                    \x20 - task_id: one\n    python_callable: send\n    depends_on: [stage]\n    outlets: [orders_ready]\n\
                    \x20 - task_id: two\n    python_callable: send\n    depends_on: [stage]\n    outlets: [orders_ready]\n";
        assert!(dags("producers-common", &[("stage", both), ("marts", marts)]).starts_with("pairs 0\n"));
    }

    #[test]
    fn two_clocks_are_told_from_one_and_a_preset_is_its_cron() {
        let marts = |schedule: &str| MARTS.replace("\"0 6 * * *\"", schedule);
        let out = dags("clock", &[("stage", STAGE), ("marts", &marts("\"0 7 * * *\""))]);
        assert!(out.contains("only the clock orders them  1\n") && out.contains("scheduled 0 7 * * *  ·  scheduled 0 6 * * *\n"), "{out}");
        let daily = STAGE.replace("\"0 6 * * *\"", "\"@daily\"");
        let out = dags("preset", &[("stage", &daily), ("marts", &marts("\"0  0 * * *\""))]);
        assert!(out.contains("nothing orders them  1\n"), "{out}");
        // The same cron in another zone is another clock.
        let (a, b) = (STAGE, MARTS);
        let files = [("dags_a/stage.yml", a), ("dags_b/marts.yml", b)];
        let root = project("zones", &[("airflow", "dags_a", "Europe/London"), ("airflow", "dags_b", "")], &files);
        let (out, _, _) = ask(&root, Output::Name);
        assert!(out.contains("only the clock orders them  1\n"), "{out}");
        assert!(out.contains("scheduled 0 6 * * *  ·  scheduled 0 6 * * * Europe/London\n"), "{out}");
        let json: serde_json::Value = serde_json::from_str(&ask(&root, Output::Json).0).unwrap();
        assert_eq!(json["pairs"][0]["waits_for"][0]["timezone"], "Europe/London");
        assert_eq!(json["pairs"][0]["waits_for"][0]["cron"], "0 6 * * *");
        assert!(json["pairs"][0]["step"].get("timezone").is_none(), "UTC is left out");
    }

    #[test]
    fn a_view_and_an_ephemeral_are_read_through_and_a_table_nothing_builds_ends_the_walk() {
        // order_items reads a view of stg_orders: the builder of the view
        // consumes nothing, and the builder of stg_orders is what it waits for.
        let items = "dag_id: jaffle_items\nschedule: \"0 6 * * *\"\ntasks:\n  - task_id: items\n    command: dbt run --select order_items\n";
        let views = "dag_id: jaffle_views\nschedule: \"0 6 * * *\"\ntasks:\n  - task_id: views\n    command: dbt run --select v_orders\n";
        let out = dags("view", &[("stage", STAGE), ("items", items), ("views", views)]);
        assert!(out.contains("  DAG jaffle_items › task items  should wait for  DAG jaffle_stage › task stage\n"), "{out}");
        assert!(out.contains("    order_items reads stg_orders through v_orders\n"), "{out}");
        assert!(out.starts_with("pairs 1:"), "the view's builder is no pair: {out}");
        // An ephemeral another step selects is walked through as well.
        let eph = "dag_id: jaffle_eph\nschedule: \"0 6 * * *\"\ntasks:\n  - task_id: eph\n    command: dbt run --select int_orders orders\n";
        let out = dags("ephemeral", &[("stage", STAGE), ("eph", eph)]);
        assert!(out.contains("    orders reads stg_orders through int_orders\n"), "{out}");
        // Another step selecting the ephemeral changes nothing: it builds no object, and consumes none.
        let other = "dag_id: jaffle_other\nschedule: \"0 6 * * *\"\ntasks:\n  - task_id: other\n    command: dbt run --select int_orders\n";
        let out = dags("ephemeral-elsewhere", &[("stage", STAGE), ("marts", MARTS), ("other", other)]);
        assert!(out.starts_with("pairs 1:") && out.contains("DAG jaffle_marts › task build  should wait for  DAG jaffle_stage"), "{out}");
        // orders_summary reads orders, which nothing builds: no walk past it.
        let summary = "dag_id: jaffle_summary\nschedule: \"0 6 * * *\"\ntasks:\n  - task_id: s\n    command: dbt run --select orders_summary\n";
        assert!(dags("nothing-builds", &[("stage", STAGE), ("summary", summary)]).starts_with("pairs 0\n"));
    }

    #[test]
    fn a_node_built_twice_is_waited_for_once_or_by_one_of_them() {
        let other = "dag_id: jaffle_again\nschedule: \"0 6 * * *\"\ntasks:\n  - task_id: again\n    command: dbt run --select stg_orders\n";
        let out = dags("two", &[("again", other), ("marts", MARTS), ("stage", STAGE)]);
        assert!(
            out.contains("  DAG jaffle_marts › task build  should wait for one of  DAG jaffle_again › task again, DAG jaffle_stage › task stage\n"),
            "{out}"
        );
        // It waits for one of them: no pair.
        let marts = "dag_id: jaffle_marts\nschedule: \"0 6 * * *\"\ntasks:\n  - task_id: build\n    command: dbt run --select orders\n    depends_on: [wait]\n\
                     sensors:\n  - task_id: wait\n    external_dag_id: jaffle_again\n";
        assert!(dags("two-one", &[("again", other), ("marts", marts), ("stage", STAGE)]).starts_with("pairs 0\n"));
    }

    #[test]
    fn two_tasks_of_a_dag_with_no_depends_on_are_a_pair_and_a_step_building_both_is_not() {
        let both = "dag_id: jaffle_all\nschedule: \"0 6 * * *\"\ntasks:\n  - task_id: stage\n    command: dbt run --select stg_orders\n\
                    \x20 - task_id: build\n    command: dbt run --select orders\n";
        let out = dags("same-dag", &[("all", both)]);
        assert!(out.contains("nothing orders them  1\n  DAG jaffle_all › task build  should wait for  DAG jaffle_all › task stage\n"), "{out}");
        let ordered = both.replace("command: dbt run --select orders\n", "command: dbt run --select orders\n    depends_on: [stage]\n");
        assert!(dags("same-dag-ordered", &[("all", &ordered)]).starts_with("pairs 0\n"));
        let one = "dag_id: jaffle_all\nschedule: \"0 6 * * *\"\ntasks:\n  - task_id: all\n    command: dbt run --select stg_orders orders\n";
        assert!(dags("same-step", &[("all", one)]).starts_with("pairs 0\n"));
    }

    #[test]
    fn a_pair_the_files_order_the_other_way_is_told_apart_and_a_cycle_ends() {
        let marts = "dag_id: jaffle_marts\nschedule: \"0 6 * * *\"\ntasks:\n  - task_id: build\n    command: dbt run --select orders\n\
                     \x20 - task_id: stage\n    command: dbt run --select stg_orders\n    depends_on: [build]\n";
        let out = dags("reversed", &[("marts", marts)]);
        assert!(out.starts_with("pairs 1: 0 nothing orders them, 0 only the clock, 1 the other way\n"), "{out}");
        assert!(out.contains("ordered the other way  1\n") && out.contains("    DAG jaffle_marts › task stage runs after it\n"), "{out}");
    }

    #[test]
    fn a_dag_whose_depends_on_loops_is_left_out_whatever_order_it_is_written_in() {
        // x needs z, y needs x, z needs y; x builds what orders reads.
        let task = |id: &str, command: &str, needs: &str| format!("  - task_id: {id}\n    {command}\n    depends_on: [{needs}]\n");
        let (x, y, z) = (
            task("x", "command: dbt run --select stg_orders", "z"),
            task("y", "command: dbt run --select customers", "x"),
            task("z", "python_callable: send", "y"),
        );
        let dag = |tasks: [&str; 3]| format!("dag_id: jaffle_loop\nschedule: \"0 6 * * *\"\ntasks:\n{}{}{}", tasks[0], tasks[1], tasks[2]);
        let first = dags("loop-1", &[("loop", &dag([&x, &y, &z])), ("marts", MARTS)]);
        assert_eq!(first, "pairs 0\nevery step waits for the steps that build its parents\nDAG jaffle_loop has a depends_on cycle, which Airflow does not load: left out\n");
        assert_eq!(first, dags("loop-2", &[("loop", &dag([&y, &z, &x])), ("marts", MARTS)]));
        assert_eq!(first, dags("loop-3", &[("loop", &dag([&z, &x, &y])), ("marts", MARTS)]));
        // As a consumer too: it reads what stage builds, and is no one's pair.
        let reader = "dag_id: jaffle_loop\nschedule: \"0 6 * * *\"\ntasks:\n  - task_id: a\n    command: dbt run --select orders\n    depends_on: [b]\n  - task_id: b\n    python_callable: send\n    depends_on: [a]\n";
        assert!(dags("loop-reader", &[("loop", reader), ("stage", STAGE)]).contains("DAG jaffle_loop has a depends_on cycle"));
        let root = project("loop-json", &[("airflow", "dags", "")], &[("dags/loop.yml", &dag([&x, &y, &z]))]);
        let json: serde_json::Value = serde_json::from_str(&ask(&root, Output::Json).0).unwrap();
        assert_eq!(json["cyclic"], json!([{"source": "dags", "job": "jaffle_loop"}]));
    }

    #[test]
    fn a_long_chain_written_backwards_is_not_a_loop_and_is_checked_at_once() {
        // 3000 tasks, each after the one written below it: the worst order for
        // taking tasks off one round at a time.
        let n = 3000;
        let tasks: Vec<usize> = (0..n).collect();
        let chain: Vec<(usize, usize)> = (0..n - 1).map(|i| (i, i + 1)).collect();
        assert!(!loops(&tasks, &chain));
        let mut closed = chain.clone();
        closed.push((n - 1, 0));
        assert!(loops(&tasks, &closed));
        // A wait on a point that is no task (a sensor) is met.
        assert!(!loops(&tasks, &[(0, 99_999)]));
        // And through the files: a DAG of the same shape is read, not left out.
        let mut text = String::from("dag_id: jaffle_chain\nschedule: \"0 6 * * *\"\ntasks:\n");
        for i in (0..2000).rev() {
            // The first one is dbt's, or the file holds no job at all.
            let what = if i == 0 { "command: dbt run --select orders" } else { "python_callable: send" };
            text.push_str(&format!("  - task_id: t{i}\n    {what}\n"));
            if i > 0 {
                text.push_str(&format!("    depends_on: [t{}]\n", i - 1));
            }
        }
        let out = dags("chain-reversed", &[("chain", &text)]);
        assert!(out.starts_with("pairs 0\n"), "{out}");
    }

    #[test]
    fn a_sensor_on_a_list_orders_each_task_and_one_on_a_group_orders_nothing() {
        let stage = "dag_id: jaffle_stage\nschedule: \"0 6 * * *\"\ntasks:\n  - task_id: stage\n    command: dbt run --select stg_orders\n\
                     \x20 - task_id: cust\n    command: dbt run --select stg_customers\n";
        let marts = |wait: &str| {
            format!(
                "dag_id: jaffle_marts\nschedule: \"0 6 * * *\"\ntasks:\n  - task_id: build\n    command: dbt run --select orders customers\n    depends_on: [wait]\n\
                 sensors:\n  - task_id: wait\n    external_dag_id: jaffle_stage\n{wait}"
            )
        };
        // Both named: customers reads stg_orders and stg_customers, each built by one of them.
        let both = dags("sensor-list", &[("stage", stage), ("marts", &marts("    external_task_ids: [stage, cust]\n"))]);
        assert!(both.starts_with("pairs 0\n"), "{both}");
        // Only one named: the other is still unordered.
        let one = dags("sensor-list-one", &[("stage", stage), ("marts", &marts("    external_task_ids: [stage]\n"))]);
        assert!(one.contains("should wait for  DAG jaffle_stage › task cust\n") && one.contains("customers reads stg_customers\n"), "{one}");
        // A group is not the whole run: its tasks are not read, so nothing is ordered.
        // The step does depend on that sensor, so the pairs say the group may order them.
        let group = dags("sensor-group", &[("stage", stage), ("marts", &marts("    external_task_group_id: loading\n"))]);
        assert!(group.starts_with("pairs 2:") && !group.contains("does not depend on it"), "{group}");
        assert!(
            group.matches("    note: task build depends on sensor wait, which waits for jaffle_stage › task group loading, whose tasks are not read\n").count() == 2,
            "{group}"
        );
        let root = project("sensor-group-json", &[("airflow", "dags", "")], &[("dags/s.yml", stage), ("dags/m.yml", &marts("    external_task_group_id: loading\n"))]);
        let json: serde_json::Value = serde_json::from_str(&ask(&root, Output::Json).0).unwrap();
        assert_eq!(
            json["pairs"][0]["group_sensors"],
            json!([{"id": "wait", "waits": ["jaffle_stage › task group loading"], "task": "task build"}])
        );
        assert!(json["pairs"][0].get("idle_sensors").is_none());
        // The whole run, naming neither, orders both.
        let whole = dags("sensor-whole-run", &[("stage", stage), ("marts", &marts(""))]);
        assert!(whole.starts_with("pairs 0\n"), "{whole}");
    }

    #[test]
    fn a_sensor_no_task_names_is_idle_and_one_on_a_later_task_of_the_dag_is_too() {
        let stage = "dag_id: jaffle_stage\nschedule: \"0 6 * * *\"\ntasks:\n  - task_id: stage\n    command: dbt run --select stg_orders\n  - task_id: publish\n    python_callable: send\n    depends_on: [stage]\n";
        let marts = |sensors: &str| format!("{}sensors:\n{sensors}", MARTS);
        // No id: nothing can name it.
        let out = dags("sensor-no-id", &[("stage", STAGE), ("marts", &marts("  - external_dag_id: jaffle_stage\n"))]);
        assert!(out.contains("    note: a sensor of this DAG waits for jaffle_stage; this task does not depend on it\n"), "{out}");
        // On the DAG's later task: it would order the pair if build named it.
        let later = marts("  - task_id: wait_publish\n    external_dag_id: jaffle_stage\n    external_task_id: publish\n");
        let out = dags("sensor-later", &[("stage", stage), ("marts", &later)]);
        assert!(out.contains("    note: sensor wait_publish of this DAG waits for jaffle_stage › publish; this task does not depend on it\n"), "{out}");
        let json: serde_json::Value = serde_json::from_str(&ask(&project("sensor-json", &[("airflow", "dags", "")], &[("dags/s.yml", STAGE), ("dags/m.yml", &marts("  - external_dag_id: jaffle_stage\n"))]), Output::Json).0).unwrap();
        assert_eq!(json["pairs"][0]["idle_sensors"], json!([{"waits": ["jaffle_stage"]}]), "no id, and the whole run");
        // A sensor on a DAG that has nothing to do with the pair is no note.
        let other = marts("  - task_id: wait_x\n    external_dag_id: jaffle_elsewhere\n");
        assert!(!dags("sensor-elsewhere", &[("stage", STAGE), ("marts", &other)]).contains("note:"));
    }

    #[test]
    fn a_sensor_on_its_own_dag_orders_nothing_in_the_run() {
        // It waits for the DAG's previous run: build is no later than stage for it.
        let both = "dag_id: jaffle_all\nschedule: \"0 6 * * *\"\ntasks:\n  - task_id: stage\n    command: dbt run --select stg_orders\n\
                    \x20 - task_id: build\n    command: dbt run --select orders\n    depends_on: [wait_self]\n\
                    sensors:\n  - task_id: wait_self\n    external_dag_id: jaffle_all\n    external_task_id: stage\n";
        let out = dags("sensor-self", &[("all", both)]);
        assert!(out.starts_with("pairs 1:"), "{out}");
    }

    #[test]
    fn a_dynamic_table_is_read_through_like_a_view_and_a_materialized_view_is_built() {
        // orders_latest reads orders_live, a dynamic table over stg_orders.
        let latest = "dag_id: jaffle_latest\nschedule: \"0 6 * * *\"\ntasks:\n  - task_id: latest\n    command: dbt run --select orders_latest\n";
        let out = dags("dynamic", &[("stage", STAGE), ("latest", latest)]);
        assert!(out.contains("    orders_latest reads stg_orders through orders_live\n"), "{out}");
        // Building it consumes nothing, and builds nothing to wait for.
        let live = "dag_id: jaffle_live\nschedule: \"0 6 * * *\"\ntasks:\n  - task_id: live\n    command: dbt run --select orders_live\n";
        assert!(dags("dynamic-builder", &[("stage", STAGE), ("live", live)]).starts_with("pairs 0\n"));
        assert!(dags("dynamic-alone", &[("live", live), ("latest", latest)]).starts_with("pairs 0\n"));
        // A materialized view is refreshed by every run of dbt: its step reads
        // stg_orders, and the step a report reads it through is the one to wait for.
        let mv = "dag_id: jaffle_mv\nschedule: \"0 6 * * *\"\ntasks:\n  - task_id: mv\n    command: dbt run --select orders_mv\n";
        let report = "dag_id: jaffle_report\nschedule: \"0 6 * * *\"\ntasks:\n  - task_id: report\n    command: dbt run --select mv_report\n";
        let out = dags("mv", &[("stage", STAGE), ("mv", mv), ("report", report)]);
        assert!(out.starts_with("pairs 2:"), "{out}");
        assert!(out.contains("  DAG jaffle_mv › task mv  should wait for  DAG jaffle_stage › task stage\n    scheduled 0 6 * * *  ·  scheduled 0 6 * * *\n    orders_mv reads stg_orders\n"), "{out}");
        assert!(out.contains("  DAG jaffle_report › task report  should wait for  DAG jaffle_mv › task mv\n"), "{out}");
        assert!(out.contains("    mv_report reads orders_mv\n") && !out.contains("through orders_mv"), "{out}");
    }

    #[test]
    fn a_sensor_on_a_dag_with_no_dbt_task_is_after_all_of_it() {
        // stage updates an asset; a DAG with no dbt task runs on it; marts waits
        // for that DAG's task and names the sensor.
        let stage = "dag_id: jaffle_stage\nschedule: \"0 6 * * *\"\ntasks:\n  - task_id: stage\n    command: dbt run --select stg_orders\n    outlets: [staged]\n";
        let bridge = "dag_id: jaffle_bridge\nschedule: [staged]\ntasks:\n  - task_id: bridge\n    python_callable: send\n    outlets: [bridged]\n";
        let marts = |wait: &str| {
            format!(
                "dag_id: jaffle_marts\nschedule: \"0 6 * * *\"\ntasks:\n  - task_id: build\n    command: dbt run --select orders\n    depends_on: [wait]\n\
                 sensors:\n  - task_id: wait\n    external_dag_id: jaffle_bridge\n{wait}"
            )
        };
        for (tag, wait) in [("feeder-task", "    external_task_id: bridge\n"), ("feeder-run", "")] {
            let out = dags(tag, &[("stage", stage), ("bridge", bridge), ("marts", &marts(wait))]);
            assert!(out.starts_with("pairs 0\n"), "{tag}: {out}");
        }
        // Not named by the task: the pair stays.
        let loose = marts("    external_task_id: bridge\n").replace("    depends_on: [wait]\n", "");
        assert!(dags("feeder-loose", &[("stage", stage), ("bridge", bridge), ("marts", &loose)]).starts_with("pairs 1:"));
    }

    #[test]
    fn an_empty_answer_says_only_what_was_checked() {
        let manual = "dag_id: jaffle_full_refresh\nschedule: null\ntasks:\n  - task_id: build\n    command: dbt run --select orders\n";
        let lost = "dag_id: jaffle_lost\nschedule: \"0 6 * * *\"\ntasks:\n  - task_id: lost\n    command: dbt run --selector nowhere\n";
        // Nothing in scope builds a node.
        let out = dags("empty-none", &[("manual", manual)]);
        assert!(out.starts_with("pairs 0\nno step of a job that starts on its own builds a node\n"), "{out}");
        // Some steps looked at, one not resolved.
        let out = dags("empty-lost", &[("stage", STAGE), ("lost", lost)]);
        assert!(out.starts_with("pairs 0\nno pair among the steps resolved\n1 step not resolved"), "{out}");
        // Everything looked at.
        assert!(dags("empty-all", &[("stage", STAGE)]).starts_with("pairs 0\nevery step waits for the steps that build its parents\n"));
    }

    #[test]
    fn a_job_of_one_name_in_two_sources_is_counted_and_named_twice() {
        let manual = "dag_id: jaffle_full_refresh\nschedule: null\ntasks:\n  - task_id: build\n    command: dbt run --select orders\n";
        let root = project("two-sources", &[("airflow", "dags_a", ""), ("airflow", "dags_b", "")], &[("dags_a/m.yml", manual), ("dags_b/m.yml", manual)]);
        let (out, _, _) = ask(&root, Output::Name);
        assert!(out.contains("2 jobs nothing starts left out: dags_a: jaffle_full_refresh, dags_b: jaffle_full_refresh\n"), "{out}");
    }

    #[test]
    fn a_dbt_cloud_job_is_ordered_by_its_steps_and_by_the_job_whose_end_starts_it() {
        let cloud = |extra: &str, steps: &str| {
            format!(
                "jobs:\n  stage:\n    name: Stage\n    id: 11\n    execute_steps:\n      - dbt run --select stg_orders\n    triggers:\n      schedule: true\n    schedule:\n      cron: \"0 6 * * *\"\n\
                 \x20 marts:\n    name: Marts\n    execute_steps:\n{steps}    triggers:\n      schedule: true\n    schedule:\n      cron: \"0 6 * * *\"\n{extra}"
            )
        };
        let run = |tag: &str, text: &str| {
            let root = project(tag, &[("dbt_cloud", "cloud", "")], &[("cloud/jobs.yml", text)]);
            ask(&root, Output::Name).0
        };
        let one = "      - dbt run --select orders\n";
        assert!(run("cloud-pair", &cloud("", one)).starts_with("pairs 1:"), "two jobs on one clock");
        let after = "    job_completion_trigger_condition:\n      condition:\n        job_id: 11\n";
        assert!(run("cloud-after", &cloud(after, one)).starts_with("pairs 0\n"));
        // Within a job, a step runs after the one written before it.
        let steps = "      - dbt run --select stg_orders\n      - dbt run --select orders\n";
        let alone = "jobs:\n  daily:\n    name: Daily\n    execute_steps:\n".to_string() + steps + "    triggers:\n      schedule: true\n    schedule:\n      cron: \"0 6 * * *\"\n";
        assert!(run("cloud-steps", &alone).starts_with("pairs 0\n"));
        let backwards = "jobs:\n  daily:\n    name: Daily\n    execute_steps:\n      - dbt run --select orders\n      - dbt run --select stg_orders\n    triggers:\n      schedule: true\n    schedule:\n      cron: \"0 6 * * *\"\n";
        let out = run("cloud-back", backwards);
        assert!(out.contains("ordered the other way  1\n") && out.contains("  job Daily › step 1/2  should wait for  job Daily › step 2/2\n"), "{out}");
    }

    #[test]
    fn a_job_nothing_starts_is_left_out_on_both_sides_and_named() {
        let manual = "dag_id: jaffle_full_refresh\nschedule: null\ntasks:\n  - task_id: build\n    command: dbt run --select orders\n";
        let out = dags("manual", &[("stage", STAGE), ("manual", manual)]);
        assert_eq!(out, "pairs 0\nevery step waits for the steps that build its parents\n1 job nothing starts left out: jaffle_full_refresh\n");
        // And as a producer: a manual job building the parent is no one to wait for.
        let manual_stage = STAGE.replace("schedule: \"0 6 * * *\"", "schedule: null");
        let out = dags("manual-producer", &[("stage", &manual_stage), ("marts", MARTS)]);
        assert!(out.starts_with("pairs 0\n") && out.contains("left out: jaffle_stage"), "{out}");
        let json: serde_json::Value = serde_json::from_str(&ask(&project("manual-json", &[("airflow", "dags", "")], &[("dags/m.yml", manual)]), Output::Json).0).unwrap();
        assert_eq!(json["left_out"][0]["job"], "jaffle_full_refresh");
    }

    #[test]
    fn a_step_that_could_not_be_resolved_is_counted_and_a_test_graph_of_nothing_is_empty() {
        let lost = "dag_id: jaffle_lost\nschedule: \"0 6 * * *\"\ntasks:\n  - task_id: lost\n    command: dbt run --selector nowhere\n";
        let out = dags("unresolved", &[("lost", lost), ("stage", STAGE)]);
        assert!(out.contains("1 step not resolved, which could hide more\n"), "{out}");
        let json: serde_json::Value = serde_json::from_str(&ask(&project("unresolved-json", &[("airflow", "dags", "")], &[("dags/l.yml", lost)]), Output::Json).0).unwrap();
        let lost = &json["unresolved"][0];
        assert_eq!((lost["source"].as_str(), lost["job"].as_str(), lost["label"].as_str()), (Some("dags"), Some("jaffle_lost"), Some("task lost")));
        assert_eq!((lost["shown"].as_str(), lost["file"].as_str(), lost["line"].as_u64()), (Some("dbt run --selector nowhere"), Some("l.yml"), Some(5)));
        assert!(lost["why"].as_str().unwrap().contains("no selector named `nowhere`"), "{lost}");
        assert_eq!(json["configured"], true);
    }

    #[test]
    fn the_json_answer_names_the_pair_its_sides_and_its_edges() {
        let root = project("json", &[("airflow", "dags", "")], &[("dags/stage.yml", STAGE), ("dags/marts.yml", MARTS)]);
        let json: serde_json::Value = serde_json::from_str(&ask(&root, Output::Json).0).unwrap();
        assert_eq!(json["counts"], json!({"none": 1, "clock": 0, "reversed": 0}));
        let pair = &json["pairs"][0];
        assert_eq!(pair["group"], "none");
        assert_eq!(pair["step"]["job"], "jaffle_marts");
        assert_eq!(pair["step"]["label"], "task build");
        assert_eq!(pair["waits_for"][0]["job"], "jaffle_stage");
        assert_eq!(
            pair["edges"][0],
            json!({
                "node": "model.jaffle_shop.orders", "node_name": "orders",
                "parent": "model.jaffle_shop.stg_orders", "parent_name": "stg_orders",
                "through": [{"id": "model.jaffle_shop.int_orders", "name": "int_orders"}],
            })
        );
        assert!(pair.get("later").is_none() && pair.get("idle_sensors").is_none());
        // Compact reads as the text does, and the same files give the same answer.
        assert_eq!(ask(&root, Output::Compact).0, ask(&root, Output::Name).0);
        assert_eq!(ask(&root, Output::Json).0, ask(&root, Output::Json).0);
    }

    #[test]
    fn more_than_five_edges_of_a_pair_are_counted_in_text_and_all_listed_in_json() {
        let many = |n: usize| {
            let selected: Vec<String> = ["orders", "customers", "order_items", "orders_summary"].iter().take(n).map(|s| s.to_string()).collect();
            format!("dag_id: jaffle_marts\nschedule: \"0 6 * * *\"\ntasks:\n  - task_id: build\n    command: dbt run --select {}\n", selected.join(" "))
        };
        let out = dags("edges", &[("stage", STAGE), ("marts", &many(4))]);
        assert!(out.contains("customers reads stg_orders") && out.contains("order_items reads stg_orders through v_orders"), "{out}");
        // The cut is the writer's: seven edges of a hand-made pair.
        let mut a = OrderAnswer { configured: true, checked: 1, counts: Counts::default(), pairs: Vec::new(), unresolved: Vec::new(), left_out: Vec::new(), cyclic: Vec::new(), not_read: None };
        a.pairs.push(Pair {
            group: Group::Unordered,
            step: Side { kind: AIRFLOW, job: "a".into(), source: "s".into(), label: "task x".into(), schedule: "manual".into(), cron: String::new(), timezone: String::new(), file: "a.yml".into(), line: 1, title: "DAG a › task x".into() },
            waits_for: vec![Side { kind: AIRFLOW, job: "b".into(), source: "s".into(), label: "task y".into(), schedule: "manual".into(), cron: String::new(), timezone: String::new(), file: "b.yml".into(), line: 1, title: "DAG b › task y".into() }],
            edges: (0..7)
                .map(|i| Edge { node: format!("m{i}"), node_name: format!("m{i}"), parent: "p".into(), parent_name: "p".into(), through: Vec::new() })
                .collect(),
            later: Vec::new(),
            idle_sensors: Vec::new(),
            group_sensors: Vec::new(),
        });
        a.counts.none = 1;
        let mut text = Vec::new();
        write_text(&a, &mut text).unwrap();
        let text = String::from_utf8(text).unwrap();
        assert!(text.contains("    m4 reads p\n    +2 more\n") && !text.contains("m5 reads"), "{text}");
        assert_eq!(serde_json::to_value(&a).unwrap()["pairs"][0]["edges"].as_array().unwrap().len(), 7);
    }

    #[test]
    fn without_a_sources_file_or_with_one_that_does_not_read_nothing_can_be_asked() {
        let none = std::env::temp_dir().join(format!("dbt-edith-order-none-{}", std::process::id()));
        std::fs::create_dir_all(&none).unwrap();
        let (out, err, code) = ask(&none, Output::Name);
        assert_eq!((out.as_str(), code), ("", CANNOT_RUN));
        assert!(err.starts_with("error: no .dbt-edith/jobs.yml in this project; name the sources in the Jobs settings\n"), "{err}");
        let root = project("broken", &[("airflow", "dags", "")], &[]);
        std::fs::write(root.join(".dbt-edith/jobs.yml"), "jobs: nope\n").unwrap();
        let (_, err, code) = ask(&root, Output::Name);
        assert_eq!(code, CANNOT_RUN);
        assert!(err.starts_with("error: .dbt-edith/jobs.yml is not read, line 1: `jobs:` is not a list of sources\n"), "{err}");
    }

    #[test]
    fn an_empty_answer_is_told_from_no_source_at_all() {
        let root = project("no-source", &[], &[]);
        let (out, _, code) = ask(&root, Output::Name);
        assert_eq!((code, out.as_str()), (ANSWERED, "pairs 0\nno sources in .dbt-edith/jobs.yml\n"));
        let json: serde_json::Value = serde_json::from_str(&ask(&root, Output::Json).0).unwrap();
        assert_eq!(json["configured"], false);
    }

    #[test]
    fn a_source_that_was_not_read_is_said() {
        let root = project("unread", &[("airflow", "dags", ""), ("airflow", "missing", "")], &[("dags/stage.yml", STAGE)]);
        let (out, _, code) = ask(&root, Output::Name);
        assert_eq!(code, ANSWERED);
        assert!(out.ends_with("not read: 1 of 2 sources (nothing at this path)\n"), "{out}");
        let json: serde_json::Value = serde_json::from_str(&ask(&root, Output::Json).0).unwrap();
        assert_eq!(json["not_read"], json!({"sources": 1, "of": 2, "reason": "nothing at this path"}));
    }
}
