//! The `--select` and `--selector` modes: one selection resolved against the
//! manifest, printed, and the process ends (0057).
//!
//! Invariants: the selection goes through the same entry points as the
//! Selection box, so the command line and the page cannot disagree. stdout
//! carries the answer and nothing else; what the answer is worth, the
//! manifest's date and whether files changed since, goes to stderr. Nothing is
//! served, stored or run, beyond the read-only git questions the freshness
//! badge already asks.

use crate::collin::iso_utc;
use crate::graph::{newer_version, Graph, Kind, Node};
use crate::select::{self, Resolved, SelectError, Tests};
use crate::{api, files, freshness, git};
use std::collections::HashSet;
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

/// What to resolve: a typed line, as the Selection box takes it, or a
/// selector from `selectors.yml` by name.
pub enum Query {
    Line { select: String, exclude: String },
    Named(String),
}

#[derive(clap::ValueEnum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Output {
    /// One name per line, as `dbt ls --output name` prints them.
    Name,
    /// One JSON object per line, as `dbt ls --output json` prints them.
    Json,
    /// One tab-separated line per node, or per column for --column, under a
    /// header line, without what the name and the path already say.
    Compact,
}

/// The header `--output compact` prints first, so the answer says what its
/// columns are. Tabs separate them, and a tab inside a field is written as a
/// space (`field`), so `cut -f` reads every line the same way.
pub const COMPACT_HEADER: &str = "name\tkind\tmaterialized\tpath\ttags\tparents";

/// The selection was answered, even by nothing.
pub const ANSWERED: i32 = 0;
/// The selection was refused: a parse error, a method this build does not
/// know, or a selector it cannot resolve.
pub const REFUSED: i32 = 1;
/// Nothing could be asked: no project, no manifest, or one that does not read.
/// clap exits with the same code for a wrong flag.
pub const CANNOT_RUN: i32 = 2;

/// The whole mode, from the command line's arguments to the exit code.
pub fn run(project: &Path, manifest: Option<&Path>, query: &Query, output: Output) -> i32 {
    let (root, _, graph) = match open(project, manifest) {
        Ok(opened) => opened,
        Err(why) => return cannot_open(&why),
    };

    let mut err = io::stderr().lock();
    // Buffered, because a selection of thousands of names would otherwise be
    // one write per line.
    let mut out = BufWriter::new(io::stdout().lock());
    let written = respond(&root, &graph, query, output, &mut out, &mut err);
    ended(written, &mut err)
}

/// What the mode does once the project is open: the answer, flushed, then what
/// it is worth, after the answer is out, so a slow walk of a large project
/// never holds it back. The exit code, or the write that failed. A caller that
/// is not a process hands it two buffers in place of the standard streams.
pub(crate) fn respond(
    root: &Path,
    graph: &Graph,
    query: &Query,
    output: Output,
    out: &mut impl Write,
    err: &mut impl Write,
) -> io::Result<i32> {
    let code = answer(graph, query, output, out, err)?;
    out.flush()?;
    let _ = provenance(root, graph, err);
    Ok(code)
}

/// The exit code of a mode whose answer was written to the standard streams.
pub(crate) fn ended(written: io::Result<i32>, err: &mut impl Write) -> i32 {
    match written {
        Ok(code) => code,
        // `| head` closing the pipe is someone having read enough, not a failure.
        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => ANSWERED,
        Err(e) => {
            let _ = writeln!(err, "error: cannot write the answer: {e}");
            CANNOT_RUN
        }
    }
}

/// Prints the reason a project did not open, as the line it was returned as,
/// and gives the exit code the modes end with.
pub(crate) fn cannot_open(why: &str) -> i32 {
    eprintln!("{why}");
    CANNOT_RUN
}

/// The project's root, the manifest's path and the graph it holds, or the
/// reason it cannot be read, as the `error:` line to print. Returned and not
/// printed, so an answer that is not a process can carry it. `--column` opens
/// the project the same way.
pub(crate) fn open(project: &Path, manifest: Option<&Path>) -> Result<(PathBuf, PathBuf, Graph), String> {
    open_with(project, manifest, |root, path| api::load_manifest(root, path))
}

/// `open`, with the catalog merged in when there is one, for an answer that
/// shows a node's columns as the Catalog does: the catalog named, else the
/// one beside the manifest (0102).
pub(crate) fn open_with_catalog(
    project: &Path,
    manifest: Option<&Path>,
    catalog: Option<&Path>,
) -> Result<(PathBuf, PathBuf, Graph), String> {
    // Asked for by name and not there, it would be left out of the answer
    // without a word, and the card would show the types of the YAML as if the
    // catalog had none to give.
    if let Some(named) = catalog.filter(|c| !c.is_file()) {
        return Err(format!("error: no catalog at {}", named.display()));
    }
    open_with(project, manifest, |root, path| api::load_base(root, path, &catalog_path(root, path, catalog)))
}

/// The catalog that is merged: the one named, else the one beside the
/// manifest, where `dbt docs generate` writes it with the manifest it read.
pub(crate) fn catalog_path(root: &Path, manifest: &Path, named: Option<&Path>) -> PathBuf {
    named
        .map(Path::to_path_buf)
        .unwrap_or_else(|| files::folder_of(manifest).unwrap_or_else(|| root.join("target")).join("catalog.json"))
}

fn open_with(
    project: &Path,
    manifest: Option<&Path>,
    load: impl FnOnce(&Path, &Path) -> anyhow::Result<Graph>,
) -> Result<(PathBuf, PathBuf, Graph), String> {
    let root = match project.canonicalize() {
        Ok(p) => files::plain(p),
        Err(e) => return Err(format!("error: cannot open {}: {e}", project.display())),
    };
    let manifest_path = manifest.map(Path::to_path_buf).unwrap_or_else(|| root.join("target").join("manifest.json"));
    if !manifest_path.is_file() {
        return Err(format!("error: no manifest at {}: run dbt parse, or pass --manifest", manifest_path.display()));
    }
    match load(&root, &manifest_path) {
        Ok(graph) => Ok((root, manifest_path, graph)),
        Err(e) => Err(format!("error: cannot read {}: {e}", manifest_path.display())),
    }
}

/// Resolves the query and prints it. Returns the exit code; an `Err` is only
/// ever a failed write.
fn answer(graph: &Graph, query: &Query, output: Output, out: &mut impl Write, err: &mut impl Write) -> io::Result<i32> {
    let resolved = match resolve(graph, query) {
        Ok(r) => r,
        Err(e) => {
            refused(query, &e, err)?;
            return Ok(REFUSED);
        }
    };
    let mut nodes: Vec<&Node> = resolved.nodes.iter().map(|&i| &graph.nodes[i as usize]).collect();
    // dbt sorts by name. The id breaks a tie, so two nodes of one name come
    // out the same way on every run (0055).
    nodes.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.cmp(&b.id)));
    if output == Output::Compact && !nodes.is_empty() {
        writeln!(out, "{COMPACT_HEADER}")?;
    }
    let shared = if output == Output::Compact { shared_names(graph) } else { HashSet::new() };
    for n in &nodes {
        match output {
            Output::Name => writeln!(out, "{}", n.name)?,
            Output::Json => {
                serde_json::to_writer(&mut *out, &Listed::of(graph, n))?;
                writeln!(out)?;
            }
            Output::Compact => writeln!(out, "{}", compact_line(graph, n, &shared))?,
        }
    }
    for w in &resolved.warnings {
        writeln!(err, "warning: {w}")?;
    }
    if nodes.is_empty() {
        writeln!(err, "nothing matched")?;
    }
    Ok(ANSWERED)
}

/// The route's own steps, in its order: a `--selector` inside the line wins,
/// as it does in the box, and a typed line always counts tests in, as
/// `dbt ls` does.
fn resolve(graph: &Graph, query: &Query) -> Result<Resolved, SelectError> {
    match query {
        Query::Named(name) => graph.selectors.resolve(graph, name),
        Query::Line { select, exclude } => match select::selector_name(select, exclude)? {
            Some((name, _)) => graph.selectors.resolve(graph, &name),
            None => select::select(graph, select, exclude, Tests::Eager).map(|(_, r)| r),
        },
    }
}

/// The error the page would show, with a caret under the term at fault when
/// there is one to point at.
fn refused(query: &Query, e: &SelectError, err: &mut impl Write) -> io::Result<()> {
    writeln!(err, "error: {e}")?;
    // An offset from the exclude half counts from that half's own start, so it
    // is only drawn under the line when there is no such half.
    if let (Query::Line { select, exclude }, Some(pos)) = (query, e.pos()) {
        if exclude.is_empty() {
            if let Some(before) = select.get(..pos) {
                writeln!(err, "  {select}")?;
                writeln!(err, "  {}^", " ".repeat(before.chars().count()))?;
            }
        }
    }
    Ok(())
}

/// One node of a `--output json` answer. dbt's key names, so a script written
/// against `dbt ls --output json` reads it, plus the parents, which `dbt ls`
/// prints only in its full JSON and which are what tells an edge. `name` is
/// the one `--output name` prints, where dbt's JSON writes a source's table
/// or a version's model bare, so a name read here can be selected by.
#[derive(serde::Serialize)]
struct Listed<'a> {
    unique_id: &'a str,
    /// The name `--output name` prints: `source_name.table` for a source,
    /// `name.v2` for a versioned model.
    name: &'a str,
    /// A versioned model's, as the manifest writes them, a number or a string.
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<&'a serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    latest_version: Option<&'a serde_json::Value>,
    /// The unique_id's first part, which is dbt's own resource type, unit
    /// tests and semantic models included.
    resource_type: &'a str,
    #[serde(skip_serializing_if = "str::is_empty")]
    package_name: &'a str,
    #[serde(skip_serializing_if = "str::is_empty")]
    original_file_path: &'a str,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    fqn: Vec<&'a str>,
    #[serde(skip_serializing_if = "str::is_empty")]
    materialized: &'a str,
    #[serde(skip_serializing_if = "<[String]>::is_empty")]
    tags: &'a [String],
    #[serde(skip_serializing_if = "str::is_empty")]
    alias: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    depends_on: Option<DependsOn<'a>>,
}

#[derive(serde::Serialize)]
struct DependsOn<'a> {
    nodes: Vec<&'a str>,
}

impl<'a> Listed<'a> {
    fn of(graph: &'a Graph, n: &'a Node) -> Listed<'a> {
        let mut parents: Vec<&str> = n.parents.iter().map(|&p| graph.nodes[p as usize].id.as_str()).collect();
        // Sorted, so a node's line is the same on every run whatever order
        // the manifest listed its parents in.
        parents.sort_unstable();
        parents.dedup();
        Listed {
            unique_id: &n.id,
            name: &n.name,
            version: n.version.as_ref().map(|v| &v.own),
            latest_version: n.version.as_ref().and_then(|v| v.latest.as_ref()),
            resource_type: n.resource_type(),
            package_name: &n.package,
            original_file_path: &n.file,
            fqn: if n.fqn.is_empty() { Vec::new() } else { n.fqn.split('.').collect() },
            materialized: &n.materialized,
            tags: &n.tags,
            alias: &n.alias,
            depends_on: (!parents.is_empty()).then_some(DependsOn { nodes: parents }),
        }
    }
}

/// A node of an installed package whose files have not been looked for or not
/// found under the project: its `original_file_path` is the one inside the
/// package, which read as the project's would be another file, or none
/// (`Graph::place_packages`).
pub(crate) fn unplaced(graph: &Graph, n: &Node) -> bool {
    let root = graph.macros.root();
    !root.is_empty() && !n.package.is_empty() && n.package != root && n.place.is_empty()
}

/// Where a node's file is, or would be, for a line with no column for the
/// package: the place under the project once found, else for an installed
/// package's node the folder `dbt deps` puts that package in, so the path
/// says whose file it is.
pub(crate) fn path_of(graph: &Graph, n: &Node) -> String {
    if unplaced(graph, n) {
        format!("dbt_packages/{}/{}", n.package, n.file)
    } else {
        n.on_disk().to_string()
    }
}

/// How many names `did you mean` offers.
const SUGGESTED: usize = 5;

/// The seconds since the epoch, for the age of a file.
pub(crate) fn now_secs() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// The node a name stands for, for the modes that take one by its name
/// (`--describe`, and `--column` for its model): the unique_id; else, among
/// `kinds` (any node when empty), the nodes of that name together with each
/// package's latest version of a model so named, exact before
/// case-insensitive; else the nodes whose unique_id ends in `.name`, which is
/// how a versioned model (`dim.v2`) or a package's node (`shop.orders`) is
/// written. Then an enabled node over a disabled twin and, when any kind
/// may answer, a model, seed, snapshot or source over a test or an exposure
/// that shares the name. Each narrowing is taken when it leaves at least one
/// node, so they combine; what still names several is refused, because a
/// guess between them would be an answer that changes with the next edit of
/// the project.
pub(crate) fn node_named(graph: &Graph, name: &str, kinds: &[Kind]) -> Result<u32, String> {
    if let Some(&i) = graph.index.get(name) {
        return Ok(i);
    }
    // A versioned model is named `dim.v2` where the manifest records its
    // version (0104), and by the end of its unique_id alone where it records
    // none.
    let found = |by_id: bool, exact: bool| -> Vec<u32> {
        (0..graph.nodes.len() as u32)
            .filter(|&i| {
                let n = &graph.nodes[i as usize];
                (kinds.is_empty() || kinds.contains(&n.kind))
                    && match (by_id, exact) {
                        (false, true) => n.name == name,
                        (false, false) => n.name.eq_ignore_ascii_case(name),
                        (true, exact) => ends_with_name(&n.id, name, exact),
                    }
            })
            .collect()
    };
    // A versioned model named without its version is its latest, as in a
    // `ref()`, where each version's own name carries its version (0104). One
    // per package, since a package is a namespace of its own; they are pooled
    // with the nodes of that very name, so a twin in another package, a seed
    // or a disabled model is weighed against them like any other and several
    // are refused, never picked by the order of the manifest.
    let latest = |exact: bool| -> Vec<u32> {
        let mut newest: Vec<(&str, &str, u32)> = Vec::new();
        for (i, n) in graph.nodes.iter().enumerate() {
            let base = n.base_name();
            if n.version.is_none() || !(if exact { base == name } else { base.eq_ignore_ascii_case(name) }) {
                continue;
            }
            match newest.iter_mut().find(|(pkg, b, _)| *pkg == n.package && *b == base) {
                Some(slot) if !newer_version(&graph.nodes[slot.2 as usize], n) => {}
                Some(slot) => slot.2 = i as u32,
                None => newest.push((&n.package, base, i as u32)),
            }
        }
        newest.into_iter().map(|(_, _, i)| i).filter(|&i| kinds.is_empty() || kinds.contains(&graph.nodes[i as usize].kind)).collect()
    };
    // Only a dotted name can be the end of a unique_id, since its parts are
    // dotted: a bare `v2` or `payments` must not reach a version or a source
    // by its table alone. A dotted name may be both a source's and the end of
    // a package model's unique_id, so the two are pooled and several refused,
    // rather than one of them guessed.
    let dotted = name.contains('.');
    let mut hits = Vec::new();
    for exact in [true, false] {
        hits = found(false, exact);
        for i in latest(exact) {
            if !hits.contains(&i) {
                hits.push(i);
            }
        }
        if dotted {
            for i in found(true, exact) {
                if !hits.contains(&i) {
                    hits.push(i);
                }
            }
        }
        if !hits.is_empty() {
            break;
        }
    }
    fn enabled(n: &Node) -> bool {
        !n.disabled
    }
    fn written(n: &Node) -> bool {
        matches!(n.kind, Kind::Model | Kind::Seed | Kind::Snapshot | Kind::Source)
    }
    // Kinds that were asked for are already the ones that may answer.
    let narrowings: &[fn(&Node) -> bool] = if kinds.is_empty() { &[enabled, written] } else { &[enabled] };
    for keep in narrowings {
        let kept: Vec<u32> = hits.iter().copied().filter(|&i| keep(&graph.nodes[i as usize])).collect();
        if !kept.is_empty() {
            hits = kept;
        }
    }
    match hits.as_slice() {
        [one] => Ok(*one),
        [] => {
            // The nodes a person writes first when any may answer, because the
            // name of a generic test contains those of its model and its
            // column, and would fill the list; a name that only a test could
            // be meant by is offered the tests.
            let mut near_ids = if kinds.is_empty() {
                let written = [Kind::Model, Kind::Source, Kind::Seed, Kind::Snapshot, Kind::Exposure, Kind::Analysis];
                graph.search(name, &written, SUGGESTED)
            } else {
                graph.search(name, kinds, SUGGESTED)
            };
            if near_ids.is_empty() && kinds.is_empty() {
                near_ids = graph.search(name, &[], SUGGESTED);
            }
            let mut near: Vec<&str> = Vec::new();
            for i in near_ids {
                let found = graph.nodes[i as usize].name.as_str();
                if !near.contains(&found) {
                    near.push(found);
                }
            }
            let none = if kinds.is_empty() { format!("no node named {name}") } else { format!("no {} is named {name}", kind_words(kinds)) };
            if near.is_empty() {
                Err(none)
            } else {
                Err(format!("{none}; did you mean {}", near.join(", ")))
            }
        }
        many => {
            let mut ids: Vec<&str> = many.iter().map(|&i| graph.nodes[i as usize].id.as_str()).collect();
            ids.sort_unstable();
            Err(format!("{name} names {} nodes: write the unique_id of one, {}", ids.len(), ids.join(", ")))
        }
    }
}

/// Whether a unique_id ends in `.name`: the part after a dot, never a piece of
/// a word.
fn ends_with_name(id: &str, name: &str, exact: bool) -> bool {
    let (id, name) = (id.as_bytes(), name.as_bytes());
    let Some(at) = id.len().checked_sub(name.len() + 1) else { return false };
    id[at] == b'.' && if exact { &id[at + 1..] == name } else { id[at + 1..].eq_ignore_ascii_case(name) }
}

/// `model, seed, snapshot or source`, for a refusal that names what was looked
/// among.
fn kind_words(kinds: &[Kind]) -> String {
    let words: Vec<&str> = kinds
        .iter()
        .map(|k| match k {
            Kind::Model => "model",
            Kind::Source => "source",
            Kind::Seed => "seed",
            Kind::Snapshot => "snapshot",
            Kind::Test => "test",
            Kind::Exposure => "exposure",
            Kind::Analysis => "analysis",
            Kind::Operation => "operation",
            Kind::Other => "node",
        })
        .collect();
    match words.split_last() {
        Some((last, [])) => last.to_string(),
        Some((last, rest)) => format!("{} or {last}", rest.join(", ")),
        None => String::new(),
    }
}

/// The names more than one node of the graph carries, as two models of one
/// name in two packages do.
fn shared_names(graph: &Graph) -> HashSet<&str> {
    let mut seen: HashSet<&str> = HashSet::new();
    // A disabled twin is never listed, so it makes no name ambiguous.
    graph.nodes.iter().filter(|n| !n.disabled).map(|n| n.name.as_str()).filter(|&name| !seen.insert(name)).collect()
}

/// A text in one field of a compact line: any tab, newline or other control
/// character is a space, so the line keeps its columns and no escape sequence
/// of the project's reaches the terminal.
pub(crate) fn field(text: &str) -> String {
    text.chars().map(|c| if c.is_control() { ' ' } else { c }).collect()
}

/// One node of a `--output compact` answer: the columns of `COMPACT_HEADER`.
/// The parents by the name `--output name` prints, so a line's last column
/// names other lines of the same answer, and nothing a reader can derive: no
/// unique_id, which is the kind and the name, no fqn, which is the path, no
/// package, which the path says. Half an answer's bytes were those (0102). A
/// name two nodes share would join them in one entry, so it is written as the
/// unique_id, in both columns.
fn compact_line(graph: &Graph, n: &Node, shared: &HashSet<&str>) -> String {
    let label = |n: &Node| field(if shared.contains(n.name.as_str()) { &n.id } else { &n.name });
    let mut parents: Vec<String> = n.parents.iter().map(|&p| label(&graph.nodes[p as usize])).collect();
    parents.sort_unstable();
    parents.dedup();
    format!(
        "{}\t{}\t{}\t{}\t{}\t{}",
        label(n),
        field(n.resource_type()),
        field(&n.materialized),
        field(&path_of(graph, n)),
        n.tags.iter().map(|t| field(t)).collect::<Vec<_>>().join(","),
        parents.join(","),
    )
}

/// Which manifest answered and whether the project moved since, because the
/// answer is the manifest's and not the working tree's.
pub(crate) fn provenance(root: &Path, graph: &Graph, err: &mut impl Write) -> io::Result<()> {
    let m = &graph.meta;
    let now = now_secs();
    let by = if m.dbt_version.is_empty() { String::new() } else { format!(" by dbt {}", m.dbt_version) };
    writeln!(
        err,
        "manifest: {}, written {} ({} ago){by}, {} nodes",
        m.manifest_path,
        iso_utc(m.manifest_mtime),
        age(now.saturating_sub(m.manifest_mtime)),
        graph.nodes.len(),
    )?;
    if !files::is_dbt_project(root) {
        // Every file of the manifest would read as gone from a folder that is
        // not the project, so no verdict beats a wrong one.
        return writeln!(err, "freshness: not checked, no dbt_project.yml in {}", root.display());
    }
    let f = freshness::check(root, m.manifest_mtime, &freshness::project_files(graph), &git::status(root));
    writeln!(err, "{}", freshness_line(&f))
}

/// One line a reader, or a script, can act on: the state first, as the badge
/// shows it, then what moved, then the badge's own advice.
fn freshness_line(f: &freshness::Freshness) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut moved = |n: usize, sample: &[String], what: &str| {
        if n == 0 {
            return;
        }
        let more = if n > sample.len() { format!(", +{}", n - sample.len()) } else { String::new() };
        parts.push(format!("{n} {what} ({}{more})", sample.join(", ")));
    };
    moved(f.edited_n, &f.edited, "edited and not committed");
    moved(f.committed_n, &f.committed, "changed by a commit");
    moved(f.gone_n, &f.gone, "gone");
    let mut line = format!("freshness: {}", f.state);
    if !parts.is_empty() {
        line.push_str(&format!(", files newer than the manifest: {}", parts.join("; ")));
    }
    if !f.advice.is_empty() {
        line.push_str(&format!(". {}", f.advice));
    }
    line
}

pub(crate) fn age(secs: u64) -> String {
    match secs {
        0..=59 => format!("{secs} s"),
        60..=3_599 => format!("{} min", secs / 60),
        3_600..=172_799 => format!("{} h", secs / 3_600),
        _ => format!("{} days", secs / 86_400),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::RawManifest;
    use serde_json::json;

    /// A source feeding a staging model feeding a mart, a test on the mart, a
    /// disabled model, a source table and a model sharing a name, and two
    /// named selectors, one of them unresolvable here.
    fn graph() -> Graph {
        graph_with(|_| {})
    }

    /// `graph()` with the manifest's JSON edited first.
    fn graph_with(edit: impl FnOnce(&mut serde_json::Value)) -> Graph {
        let mut manifest = json!({
            "metadata": { "dbt_version": "1.12.5", "project_name": "shop" },
            "nodes": {
                "model.shop.stg_orders": {
                    "name": "stg_orders", "resource_type": "model", "package_name": "shop",
                    "original_file_path": "models/staging/stg_orders.sql",
                    "fqn": ["shop", "staging", "stg_orders"], "tags": ["nightly"],
                    "config": { "materialized": "view" },
                },
                "model.shop.orders": {
                    "name": "orders", "resource_type": "model", "package_name": "shop",
                    "original_file_path": "models/marts/orders.sql",
                    "fqn": ["shop", "marts", "orders"], "alias": "fct_orders",
                    "config": { "materialized": "table" },
                },
                "model.shop.customers": {
                    "name": "customers", "resource_type": "model", "package_name": "shop",
                    "original_file_path": "models/marts/customers.sql",
                    "fqn": ["shop", "marts", "customers"],
                    "config": { "materialized": "table" },
                },
                "test.shop.not_null_orders_id": {
                    "name": "not_null_orders_id", "resource_type": "test", "package_name": "shop",
                    "original_file_path": "models/marts/schema.yml",
                    "fqn": ["shop", "marts", "not_null_orders_id"],
                    "column_name": "id", "attached_node": "model.shop.orders",
                    "test_metadata": { "name": "not_null" },
                },
            },
            "sources": {
                "source.shop.raw.orders": {
                    "name": "orders", "resource_type": "source", "package_name": "shop",
                    "source_name": "raw", "identifier": "orders",
                    "original_file_path": "models/staging/sources.yml",
                    "fqn": ["shop", "staging", "raw", "orders"],
                },
            },
            "disabled": {
                "model.shop.legacy": [{
                    "name": "legacy", "resource_type": "model", "package_name": "shop",
                    "original_file_path": "models/marts/legacy.sql",
                    "fqn": ["shop", "marts", "legacy"],
                    "config": { "enabled": false, "materialized": "table" },
                }],
            },
            "parent_map": {
                "model.shop.stg_orders": ["source.shop.raw.orders"],
                "model.shop.orders": ["model.shop.stg_orders", "model.shop.customers"],
                "test.shop.not_null_orders_id": ["model.shop.orders"],
            },
            "selectors": {
                "marts": { "name": "marts", "definition": { "method": "path", "value": "models/marts" } },
                "ci": { "name": "ci", "definition": { "method": "state", "value": "modified" } },
            },
        });
        edit(&mut manifest);
        let raw: RawManifest = serde_json::from_value(manifest).unwrap();
        Graph::build(raw, Path::new("manifest.json"), 0, 0)
    }

    fn line(select: &str, exclude: &str) -> Query {
        Query::Line { select: select.to_string(), exclude: exclude.to_string() }
    }

    /// stdout, stderr and the exit code of one query.
    fn ask(query: &Query, output: Output) -> (String, String, i32) {
        ask_on(&graph(), query, output)
    }

    fn ask_on(g: &Graph, query: &Query, output: Output) -> (String, String, i32) {
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = answer(g, query, output, &mut out, &mut err).unwrap();
        (String::from_utf8(out).unwrap(), String::from_utf8(err).unwrap(), code)
    }

    #[test]
    fn names_are_the_whole_answer_sorted_with_tests_in() {
        let (out, err, code) = ask(&line("+orders+", ""), Output::Name);
        assert_eq!(code, ANSWERED);
        assert_eq!(out, "customers\nnot_null_orders_id\norders\nraw.orders\nstg_orders\n");
        assert_eq!(err, "");
    }

    #[test]
    fn exclude_drops_what_it_names() {
        let (out, _, _) = ask(&line("+orders+", "resource_type:test"), Output::Name);
        assert_eq!(out, "customers\norders\nraw.orders\nstg_orders\n");
        // The `--exclude` inside the line joins the flag's, as in the box. A
        // source is reached by `source:` alone, as in dbt (0056).
        let (out, _, _) = ask(&line("+orders+ --exclude source:raw.orders", "resource_type:test"), Output::Name);
        assert_eq!(out, "customers\norders\nstg_orders\n");
    }

    #[test]
    fn a_pasted_dbt_command_is_stripped() {
        let (out, _, code) = ask(&line("dbt ls -s stg_orders+1", ""), Output::Name);
        assert_eq!(code, ANSWERED);
        // The test comes with `orders`, by dbt's eager indirect selection.
        assert_eq!(out, "not_null_orders_id\norders\nstg_orders\n");
    }

    #[test]
    fn json_has_one_object_per_node_with_dbt_keys_and_parents() {
        let (out, _, code) = ask(&line("orders", "resource_type:test"), Output::Json);
        assert_eq!(code, ANSWERED);
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 1, "{out}");
        let v: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(v["unique_id"], "model.shop.orders");
        assert_eq!(v["resource_type"], "model");
        assert_eq!(v["package_name"], "shop");
        assert_eq!(v["original_file_path"], "models/marts/orders.sql");
        assert_eq!(v["fqn"], json!(["shop", "marts", "orders"]));
        assert_eq!(v["materialized"], "table");
        assert_eq!(v["alias"], "fct_orders");
        assert_eq!(v["depends_on"]["nodes"], json!(["model.shop.customers", "model.shop.stg_orders"]));
        // Empty fields are left out, as every payload here leaves them.
        assert!(v.get("tags").is_none(), "{v}");
    }

    #[test]
    fn json_names_a_source_by_its_dbt_resource_type() {
        let (out, _, _) = ask(&line("source:raw", ""), Output::Json);
        let v: serde_json::Value = serde_json::from_str(out.trim()).unwrap();
        assert_eq!(v["resource_type"], "source");
        assert_eq!(v["name"], "raw.orders");
        assert!(v.get("depends_on").is_none(), "{v}");
    }

    /// A version is named as `--output name` names it, in either output, and
    /// its JSON carries the manifest's two version keys, a number staying one;
    /// an unversioned node carries neither (0104).
    #[test]
    fn a_version_is_listed_by_its_name_with_its_version_keys() {
        let version = |v: i64| {
            serde_json::json!({ "name": "totals", "resource_type": "model", "package_name": "shop",
                "original_file_path": format!("models/totals_v{v}.sql"), "fqn": ["shop", "totals", format!("v{v}")],
                "version": v, "latest_version": 2 })
        };
        let raw: RawManifest = serde_json::from_value(serde_json::json!({
            "nodes": {
                "model.shop.totals.v1": version(1), "model.shop.totals.v2": version(2),
                "model.shop.plain": { "name": "plain", "resource_type": "model", "fqn": ["shop", "plain"] },
            },
        }))
        .unwrap();
        let g = Graph::build(raw, Path::new("manifest.json"), 0, 0);
        let run = |select: &str, output: Output| {
            let (mut out, mut err) = (Vec::new(), Vec::new());
            answer(&g, &line(select, ""), output, &mut out, &mut err).unwrap();
            String::from_utf8(out).unwrap()
        };
        assert_eq!(run("totals", Output::Name), "totals.v1\ntotals.v2\n");
        let listed: Vec<serde_json::Value> =
            run("totals.v1 plain", Output::Json).lines().map(|l| serde_json::from_str(l).unwrap()).collect();
        assert_eq!(listed[0]["name"], "plain");
        assert!(listed[0].get("version").is_none() && listed[0].get("latest_version").is_none(), "{}", listed[0]);
        assert_eq!(listed[1]["name"], "totals.v1");
        assert_eq!((&listed[1]["version"], &listed[1]["latest_version"]), (&serde_json::json!(1), &serde_json::json!(2)));
    }

    #[test]
    fn compact_is_one_tab_separated_line_per_node_under_a_header() {
        let (out, _, code) = ask(&line("+orders+", ""), Output::Compact);
        assert_eq!(code, ANSWERED);
        assert_eq!(
            out,
            "name\tkind\tmaterialized\tpath\ttags\tparents\n\
             customers\tmodel\ttable\tmodels/marts/customers.sql\t\t\n\
             not_null_orders_id\ttest\t\tmodels/marts/schema.yml\t\torders\n\
             orders\tmodel\ttable\tmodels/marts/orders.sql\t\tcustomers,stg_orders\n\
             raw.orders\tsource\tsource\tmodels/staging/sources.yml\t\t\n\
             stg_orders\tmodel\tview\tmodels/staging/stg_orders.sql\tnightly\traw.orders\n"
        );
        // Nothing matched: no header either, so an empty answer stays empty.
        let (out, _, _) = ask(&line("tag:nowhere", ""), Output::Compact);
        assert_eq!(out, "");
    }

    #[test]
    fn compact_paths_say_whose_file_an_installed_packages_node_is() {
        let mut g = graph();
        let at = g.index["model.shop.customers"] as usize;
        g.nodes[at].package = "audit".into();
        // Not looked for on disk: where `dbt deps` would have put it.
        let line = |g: &Graph| compact_line(g, &g.nodes[at], &shared_names(g));
        assert!(line(&g).contains("\tdbt_packages/audit/models/marts/customers.sql\t"));
        // Looked for and found: where it is.
        g.nodes[at].place = "dbt_modules/audit/models/marts/customers.sql".into();
        assert!(line(&g).contains("\tdbt_modules/audit/models/marts/customers.sql\t"));
    }

    #[test]
    fn compact_writes_the_unique_id_of_a_name_two_nodes_share() {
        // The same name in two packages, both parents of `orders`: a name
        // alone would join them in one entry of the last column.
        let g = graph_with(|m| {
            m["nodes"]["model.audit.customers"] = json!({
                "name": "customers", "resource_type": "model", "package_name": "audit",
                "original_file_path": "models/customers.sql", "config": { "materialized": "table" },
            });
            m["parent_map"]["model.shop.orders"] = json!(["model.shop.stg_orders", "model.shop.customers", "model.audit.customers"]);
        });
        let (out, _, code) = ask_on(&g, &line("+orders", "resource_type:test"), Output::Compact);
        assert_eq!(code, ANSWERED);
        assert_eq!(
            out,
            "name\tkind\tmaterialized\tpath\ttags\tparents\n\
             model.audit.customers\tmodel\ttable\tdbt_packages/audit/models/customers.sql\t\t\n\
             model.shop.customers\tmodel\ttable\tmodels/marts/customers.sql\t\t\n\
             orders\tmodel\ttable\tmodels/marts/orders.sql\t\tmodel.audit.customers,model.shop.customers,stg_orders\n\
             raw.orders\tsource\tsource\tmodels/staging/sources.yml\t\t\n\
             stg_orders\tmodel\tview\tmodels/staging/stg_orders.sql\tnightly\traw.orders\n"
        );
    }

    #[test]
    fn a_tab_or_a_newline_inside_a_field_keeps_the_columns() {
        let mut g = graph();
        let at = g.index["model.shop.stg_orders"] as usize;
        g.nodes[at].name = "stg\torders".into();
        g.nodes[at].tags = vec!["a\tb".into(), "c\nd\u{1b}[0m".into()];
        g.nodes[at].file = "models/stg\torders.sql".into();
        let row = compact_line(&g, &g.nodes[at], &shared_names(&g));
        assert_eq!(row.split('\t').count(), 6, "{row:?}");
        assert_eq!(row, "stg orders\tmodel\tview\tmodels/stg orders.sql\ta b,c d [0m\traw.orders");
    }

    #[test]
    fn a_name_is_looked_for_among_the_kinds_asked_and_refused_in_their_words() {
        const FOUR: [Kind; 4] = [Kind::Model, Kind::Seed, Kind::Snapshot, Kind::Source];
        // An exposure of the model's name: a rival to any node, none to a
        // model among models.
        let g = graph_with(|m| {
            m["exposures"] = json!({
                "exposure.shop.orders": { "name": "orders", "resource_type": "exposure", "package_name": "shop",
                    "original_file_path": "models/exposures.yml" },
            });
        });
        let id = |name: &str, kinds: &[Kind]| node_named(&g, name, kinds).map(|i| g.nodes[i as usize].id.clone());
        assert_eq!(id("orders", &FOUR).unwrap(), "model.shop.orders");
        assert_eq!(id("orders", &[]).unwrap(), "model.shop.orders", "a model over an exposure when any may answer");
        assert_eq!(id("orders", &[Kind::Exposure]).unwrap(), "exposure.shop.orders");
        assert_eq!(id("exposure.shop.orders", &FOUR).unwrap(), "exposure.shop.orders", "a unique_id is a node, whatever kind");
        assert_eq!(id("ORDERS", &FOUR).unwrap(), "model.shop.orders");
        assert_eq!(id("shop.stg_orders", &FOUR).unwrap(), "model.shop.stg_orders");

        assert_eq!(id("nope", &[]).unwrap_err(), "no node named nope");
        assert_eq!(id("nope", &FOUR).unwrap_err(), "no model, seed, snapshot or source is named nope");
        assert_eq!(id("nope", &[Kind::Model]).unwrap_err(), "no model is named nope");
        // The names near it are of the kinds asked: a test only for any node.
        assert_eq!(id("not_null_orders", &[]).unwrap_err(), "no node named not_null_orders; did you mean not_null_orders_id");
        assert_eq!(id("not_null_orders", &FOUR).unwrap_err(), "no model, seed, snapshot or source is named not_null_orders");
        assert!(
            id("order", &FOUR).unwrap_err().starts_with("no model, seed, snapshot or source is named order; did you mean orders, "),
            "{:?}",
            id("order", &FOUR)
        );
    }

    #[test]
    fn a_dotted_name_reaches_the_end_of_a_unique_id_and_a_bare_one_does_not() {
        // A version of a model, and a source and a package's model that one
        // dotted name could mean.
        let g = graph_with(|m| {
            m["nodes"]["model.shop.dim.v2"] = json!({
                "name": "dim", "resource_type": "model", "package_name": "shop",
                "original_file_path": "models/marts/dim_v2.sql", "config": { "materialized": "table" },
            });
            m["nodes"]["model.raw.payments"] = json!({
                "name": "payments", "resource_type": "model", "package_name": "raw",
                "original_file_path": "models/payments.sql", "config": { "materialized": "view" },
            });
            m["sources"]["source.shop.raw.payments"] = json!({
                "name": "payments", "resource_type": "source", "package_name": "shop",
                "source_name": "raw", "identifier": "payments", "original_file_path": "models/staging/sources.yml",
            });
        });
        let id = |name: &str| node_named(&g, name, &[]).map(|i| g.nodes[i as usize].id.clone());
        assert_eq!(id("dim.v2").unwrap(), "model.shop.dim.v2");
        assert!(id("v2").unwrap_err().starts_with("no node named v2"), "a bare word is no end of a unique_id");
        assert_eq!(id("payments").unwrap(), "model.raw.payments", "by name, never by a source's table alone");
        assert_eq!(
            id("raw.payments").unwrap_err(),
            "raw.payments names 2 nodes: write the unique_id of one, model.raw.payments, source.shop.raw.payments"
        );
    }

    /// The bare name of a versioned model is its latest version per package,
    /// pooled with every other node of that name and judged by the same
    /// narrowings, so it is refused where nothing on the command line says
    /// which package (0104).
    #[test]
    fn a_bare_name_weighs_the_latest_version_of_each_package_with_the_other_nodes_of_it() {
        let version = |pkg: &str, name: &str, v: i64, latest: i64| {
            serde_json::json!({ "name": name, "resource_type": "model", "package_name": pkg,
                "original_file_path": format!("models/{name}_v{v}.sql"), "fqn": [pkg, name, format!("v{v}")],
                "version": v, "latest_version": latest })
        };
        let plain = |kind: &str, pkg: &str, name: &str, enabled: bool| {
            serde_json::json!({ "name": name, "resource_type": kind, "package_name": pkg,
                "original_file_path": format!("models/{name}.sql"), "fqn": [pkg, name], "config": { "enabled": enabled } })
        };
        let g = graph_with(|m| {
            for (pkg, name) in [("shop", "solo"), ("shop", "dimv"), ("other", "dimv"), ("shop", "mixed"), ("shop", "sv"), ("shop", "totals")] {
                for v in 1..=2 {
                    m["nodes"][format!("model.{pkg}.{name}.v{v}")] = version(pkg, name, v, 2);
                }
            }
            m["nodes"]["model.other.mixed"] = plain("model", "other", "mixed", true);
            m["nodes"]["seed.other.sv"] = plain("seed", "other", "sv", true);
            m["disabled"]["model.shop.totals"] = serde_json::json!([plain("model", "shop", "totals", false)]);
            // The latest is the one the manifest names, not the highest.
            for v in 1..=2 {
                m["nodes"][format!("model.shop.held.v{v}")] = version("shop", "held", v, 1);
            }
        });
        let id = |name: &str| node_named(&g, name, &[]).map(|i| g.nodes[i as usize].id.clone());
        assert_eq!(id("solo").unwrap(), "model.shop.solo.v2");
        assert_eq!(id("SoLo").unwrap(), "model.shop.solo.v2", "in any case, as a name of one node is");
        assert_eq!(id("held").unwrap(), "model.shop.held.v1");
        assert_eq!(id("solo.v1").unwrap(), "model.shop.solo.v1");
        // A model of that name in another package, versioned or not, a seed
        // beside it: not guessed between, in either case.
        for name in ["dimv", "DIMV"] {
            assert_eq!(
                id(name).unwrap_err(),
                format!("{name} names 2 nodes: write the unique_id of one, model.other.dimv.v2, model.shop.dimv.v2")
            );
        }
        assert_eq!(
            id("mixed").unwrap_err(),
            "mixed names 2 nodes: write the unique_id of one, model.other.mixed, model.shop.mixed.v2"
        );
        assert_eq!(id("sv").unwrap_err(), "sv names 2 nodes: write the unique_id of one, model.shop.sv.v2, seed.other.sv");
        // A disabled model of the name gives way to the enabled versions.
        assert_eq!(id("totals").unwrap(), "model.shop.totals.v2");
        assert_eq!(id("TOTALS").unwrap(), "model.shop.totals.v2");
        // Among the kinds asked, the seed is not a rival.
        assert_eq!(node_named(&g, "sv", &[Kind::Model]).map(|i| g.nodes[i as usize].id.clone()).unwrap(), "model.shop.sv.v2");
    }

    #[test]
    fn a_catalog_named_and_not_a_file_is_refused_before_anything_is_read() {
        let dir = std::env::temp_dir().join(format!("dbt-edith-ls-catalog-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("target")).unwrap();
        std::fs::write(dir.join("target/manifest.json"), r#"{"metadata":{"project_name":"shop"},"nodes":{}}"#).unwrap();
        std::fs::write(dir.join("catalog.json"), r#"{"nodes":{}}"#).unwrap();
        let open = |catalog: Option<&Path>| open_with_catalog(&dir, None, catalog).err();
        // The manifest reads, so what refuses below is the catalog alone.
        assert_eq!(open(None), None);
        assert_eq!(open(Some(&dir.join("catalog.json"))), None);
        assert_eq!(open(Some(&dir.join("nowhere.json"))), Some(format!("error: no catalog at {}", dir.join("nowhere.json").display())));
        assert!(open(Some(&dir)).is_some(), "a folder is not a catalog");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_refused_line_says_why_and_points_at_the_term() {
        let (out, err, code) = ask(&line("orders state:modified", ""), Output::Name);
        assert_eq!(code, REFUSED);
        assert_eq!(out, "");
        assert!(err.starts_with("error: unknown selector method `state`"), "{err}");
        assert!(err.ends_with("  orders state:modified\n         ^\n"), "{err}");
    }

    #[test]
    fn no_caret_when_the_fault_may_sit_in_the_exclude_half() {
        let (_, err, code) = ask(&line("orders", "state:modified"), Output::Name);
        assert_eq!(code, REFUSED);
        assert_eq!(err.lines().count(), 1, "{err}");
    }

    #[test]
    fn an_empty_answer_is_an_answer() {
        let (out, err, code) = ask(&line("tag:nowhere", ""), Output::Name);
        assert_eq!(code, ANSWERED);
        assert_eq!(out, "");
        assert!(err.ends_with("nothing matched\n"), "{err}");
    }

    #[test]
    fn a_disabled_model_is_never_listed() {
        let (out, _, _) = ask(&line("path:models/marts", "resource_type:test"), Output::Name);
        assert_eq!(out, "customers\norders\n");
    }

    #[test]
    fn named_selectors_resolve_by_flag_and_inside_the_line() {
        let (by_flag, _, code) = ask(&Query::Named("marts".into()), Output::Name);
        assert_eq!(code, ANSWERED);
        assert_eq!(by_flag, "customers\nnot_null_orders_id\norders\n");
        let (in_line, _, _) = ask(&line("--selector marts", ""), Output::Name);
        assert_eq!(in_line, by_flag);
    }

    #[test]
    fn a_selector_that_cannot_be_resolved_is_refused() {
        let (_, err, code) = ask(&Query::Named("nope".into()), Output::Name);
        assert_eq!(code, REFUSED);
        assert!(err.starts_with("error: no selector named `nope`"), "{err}");
        let (_, err, code) = ask(&Query::Named("ci".into()), Output::Name);
        assert_eq!(code, REFUSED);
        assert!(err.starts_with("error: selector `ci` cannot be resolved here"), "{err}");
    }

    #[test]
    fn the_freshness_line_leads_with_the_state() {
        let mut f = freshness::Freshness { state: "fresh".into(), ..Default::default() };
        assert_eq!(freshness_line(&f), "freshness: fresh");
        f.state = "stale".into();
        f.committed_n = 8;
        f.committed = (0..6).map(|i| format!("models/m{i}.sql")).collect();
        f.edited_n = 1;
        f.edited = vec!["models/a.sql".into()];
        f.advice = "Run dbt parse before trusting this lineage.".into();
        assert_eq!(
            freshness_line(&f),
            "freshness: stale, files newer than the manifest: 1 edited and not committed (models/a.sql); \
             8 changed by a commit (models/m0.sql, models/m1.sql, models/m2.sql, models/m3.sql, \
             models/m4.sql, models/m5.sql, +2). Run dbt parse before trusting this lineage."
        );
    }

    #[test]
    fn ages_read_in_the_unit_that_fits() {
        assert_eq!(age(42), "42 s");
        assert_eq!(age(32 * 60), "32 min");
        assert_eq!(age(5 * 3_600), "5 h");
        assert_eq!(age(3 * 86_400), "3 days");
    }
}
