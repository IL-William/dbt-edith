//! The `--describe` and `--overview` modes: what the manifest says about a
//! node, and about the project, printed, and the process ends (0102).
//!
//! Invariants: both read the `Graph` the Catalog is painted from, the catalog
//! merged in, so the page and the command line cannot disagree about a node.
//! stdout carries the answer and nothing else; what it is worth, the
//! manifest's date and whether files changed since, goes to stderr
//! (`ls::provenance`). Nothing is served, stored or run: the one file read
//! beyond the artifacts is a node's own, under `--sql`, through
//! `files::read_file`, which holds it to the project root as every read the
//! page asks for.

use crate::collin::{self, iso_utc, Available};
use crate::graph::{Graph, Kind, Node};
use crate::ls::{self, node_named, Output, ANSWERED, CANNOT_RUN, REFUSED};
use crate::files;
use std::collections::BTreeMap;
use std::io::{self, BufWriter, Write};
use std::path::Path;

/// How many tags, and how many selectors, an overview names before it counts
/// the rest: a project of thousands of models tags them with dozens and names
/// dozens of selectors, and the head of the list is what a first look wants.
/// JSON holds them all.
const SHOWN: usize = 15;

/// A text answer as rows under a heading: a key and its value on one line, or
/// a key and the lines under it. One value column per answer, so a reader's
/// eye, and `cut -c`, find every value in the same place, and a row with
/// nothing to say is never made.
#[derive(Default)]
struct Sheet {
    rows: Vec<Row>,
}

enum Row {
    Line(String, String),
    /// A key, what follows it on its own line (often nothing), and the lines
    /// under it, each already indented relative to the block.
    Block(String, String, Vec<String>),
}

impl Sheet {
    fn line(&mut self, key: impl Into<String>, value: impl Into<String>) {
        let value = value.into();
        if !value.is_empty() {
            self.rows.push(Row::Line(key.into(), value));
        }
    }

    fn block(&mut self, key: impl Into<String>, head: impl Into<String>, lines: Vec<String>) {
        if !lines.is_empty() {
            self.rows.push(Row::Block(key.into(), head.into(), lines));
        }
    }

    /// `indent` spaces before every key, the lines of a block two further in.
    fn write(&self, indent: usize, out: &mut impl Write) -> io::Result<()> {
        let pad = " ".repeat(indent);
        let width = self
            .rows
            .iter()
            .filter_map(|r| match r {
                Row::Line(key, _) => Some(key.len()),
                Row::Block(key, head, _) if !head.is_empty() => Some(key.len()),
                Row::Block(..) => None,
            })
            .max()
            .unwrap_or(0)
            + 2;
        for row in &self.rows {
            match row {
                Row::Line(key, value) => writeln!(out, "{pad}{key:<width$}{}", clean(value))?,
                Row::Block(key, head, lines) => {
                    if head.is_empty() {
                        writeln!(out, "{pad}{key}")?;
                    } else {
                        writeln!(out, "{pad}{key:<width$}{}", clean(head))?;
                    }
                    for line in lines {
                        let line = clean(line);
                        // A blank line inside a description stays blank, with
                        // no trailing spaces for an agent to pay for.
                        if line.is_empty() {
                            writeln!(out)?;
                        } else {
                            writeln!(out, "{pad}  {line}")?;
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

/// `1 model`, `5 models`.
fn counted(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// Whitespace runs of any kind as one space: a description in a line that a
/// tab or a newline would split in two.
fn flat(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A line with every control character but the tab left out: a description or
/// a SQL file is the project's text, and an escape sequence in it would be
/// the terminal's to run, not the reader's to see. The tab stays because the
/// columns line is tab-separated.
fn clean(text: &str) -> String {
    text.chars().filter(|&c| c == '\t' || !c.is_control()).collect()
}

// ---------------------------------------------------------------- --describe

/// The whole mode, from the command line's arguments to the exit code. The
/// names that resolve are all printed, then the ones that do not are refused,
/// so a caller asking for three gets the two that exist.
pub fn run_describe(
    project: &Path,
    manifest: Option<&Path>,
    catalog: Option<&Path>,
    names: &[String],
    sql: bool,
    output: Output,
) -> i32 {
    // Before the project is opened: reading a manifest and a catalog to say
    // that no name was given is a wait for nothing.
    if wanted(names).is_empty() {
        eprintln!("{NO_NAME}");
        return REFUSED;
    }
    let (root, _, graph) = match ls::open_with_catalog(project, manifest, catalog) {
        Ok(opened) => opened,
        Err(code) => return code,
    };
    let mut err = io::stderr().lock();
    let mut out = BufWriter::new(io::stdout().lock());
    let code = match describe(&graph, &root, names, sql, output, &mut out, &mut err).and_then(|code| out.flush().map(|_| code)) {
        Ok(code) => code,
        // `| head` closing the pipe is someone having read enough, not a failure.
        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => return ANSWERED,
        Err(e) => {
            let _ = writeln!(err, "error: cannot write the answer: {e}");
            return CANNOT_RUN;
        }
    };
    let _ = ls::provenance(&root, &graph, &mut err);
    code
}

const NO_NAME: &str = "error: --describe wants the name or the unique_id of a node";

/// The names that were written, without the blanks a trailing comma leaves.
fn wanted(names: &[String]) -> Vec<&str> {
    names.iter().map(|n| n.trim()).filter(|n| !n.is_empty()).collect()
}

/// Resolves each name and prints its card. An `Err` is only ever a failed write.
///
/// stdout is buffered and stderr is not, so on a terminal a line of stderr
/// would reach the screen before the cards written ahead of it: stdout is
/// flushed before any of stderr is written, and the refusals come after the
/// last card, as the mode promises.
fn describe(
    graph: &Graph,
    root: &Path,
    names: &[String],
    sql: bool,
    output: Output,
    out: &mut impl Write,
    err: &mut impl Write,
) -> io::Result<i32> {
    let names = wanted(names);
    if names.is_empty() {
        writeln!(err, "{NO_NAME}")?;
        return Ok(REFUSED);
    }
    let mut code = ANSWERED;
    let mut done: Vec<u32> = Vec::new();
    let mut refused: Vec<String> = Vec::new();
    for name in names {
        let idx = match node_named(graph, name, &[]) {
            Ok(idx) => idx,
            Err(why) => {
                refused.push(why);
                code = REFUSED;
                continue;
            }
        };
        // A name and its unique_id in one call are one node, said once.
        if done.contains(&idx) {
            continue;
        }
        let n = &graph.nodes[idx as usize];
        let text = matches!(output, Output::Name | Output::Compact);
        // The empty line between two cards goes out before a note of
        // `--sql` on stderr, so the note does not read as the end of the
        // card above it.
        if text && !done.is_empty() {
            writeln!(out)?;
        }
        let body = if sql {
            out.flush()?;
            read_sql(graph, root, n, err)?
        } else {
            None
        };
        match output {
            Output::Json => {
                serde_json::to_writer(&mut *out, &Described::of(graph, idx, body))?;
                writeln!(out)?;
            }
            // A card is read, so compact has nothing to drop from it: both
            // print the text.
            Output::Name | Output::Compact => {
                let (first, mut sheet) = card(graph, idx);
                if let Some(body) = body.as_deref().map(str::trim_end).filter(|b| !b.is_empty()) {
                    sheet.block("sql", "", body.lines().map(|l| l.trim_end().to_string()).collect());
                }
                writeln!(out, "{}", clean(&first))?;
                sheet.write(2, out)?;
            }
        }
        done.push(idx);
    }
    if !refused.is_empty() {
        out.flush()?;
        for why in refused {
            writeln!(err, "error: {why}")?;
        }
    }
    Ok(code)
}

/// Whether the node's own file holds code: a model, a snapshot, an analysis or
/// a singular test. A generic test shares its properties file, a source and an
/// exposure are declared in one, and a seed's is data.
fn has_code(path: &str) -> bool {
    Path::new(path).extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("sql") || e.eq_ignore_ascii_case("py"))
}

/// The node's own file, or None once stderr says why there is none: the
/// answer is still the card, and the exit code is not changed by a file.
fn read_sql(graph: &Graph, root: &Path, n: &Node, err: &mut impl Write) -> io::Result<Option<String>> {
    let path = ls::path_of(graph, n);
    if !has_code(&path) {
        writeln!(err, "{}: no SQL file of its own", n.name)?;
        return Ok(None);
    }
    if ls::unplaced(graph, n) {
        writeln!(err, "{}: its file is in package {}, which is not installed under the project: run dbt deps", n.name, n.package)?;
        return Ok(None);
    }
    match files::read_file(root, &path) {
        Ok(body) => {
            if body.truncated {
                writeln!(err, "{}: {path} is larger than dbt-edith reads of a file, so its end is left out", n.name)?;
            }
            Ok(Some(body.content))
        }
        Err(e) => {
            writeln!(err, "{}: cannot read {path}: {e}", n.name)?;
            Ok(None)
        }
    }
}

/// Where a node is built: dbt's own `relation_name`, else what is left of
/// database, schema and alias for a node that is built at all. An ephemeral
/// model, an analysis and a test are never built, whatever their location says.
fn relation(n: &Node) -> String {
    if !n.relation.is_empty() {
        return n.relation.clone();
    }
    let built = matches!(n.kind, Kind::Model | Kind::Seed | Kind::Snapshot | Kind::Source) && n.materialized != "ephemeral";
    if !built {
        return String::new();
    }
    [n.database.as_str(), n.schema.as_str(), n.alias.as_str()].iter().filter(|p| !p.is_empty()).copied().collect::<Vec<_>>().join(".")
}

/// An incremental model's strategy as its box shows it, else as configured,
/// else empty: what dbt falls back to depends on the adapter, and the
/// manifest does not say, so no name is made up for it.
fn strategy_of(n: &Node) -> &str {
    let facts = n.strategy_facts();
    [facts.strategy_shown, n.strategy.as_str()].into_iter().find(|s| !s.is_empty()).unwrap_or("")
}

/// The nodes by name and then unique_id, so a card is the same on every run
/// (0055).
fn sorted_by_name<'a>(graph: &'a Graph, ids: &[u32]) -> Vec<&'a Node> {
    let mut sorted = ids.to_vec();
    sorted.sort_by(|&a, &b| graph.by_name_then_id(a, b));
    sorted.into_iter().map(|i| &graph.nodes[i as usize]).collect()
}

/// `name (materialized)` for each node, sorted by name; a node with no
/// materialization says its kind, as a source or an exposure has nothing else
/// to say.
fn refs(graph: &Graph, ids: &[u32], with_how: bool) -> String {
    sorted_by_name(graph, ids)
        .iter()
        .map(|n| {
            if with_how {
                format!("{} ({})", n.name, if n.materialized.is_empty() { n.resource_type() } else { &n.materialized })
            } else {
                n.name.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// The settings of a test that are off dbt's defaults, `where x, limit 5`.
fn settings_of(t: &Node) -> Vec<String> {
    t.settings.iter().map(|s| format!("{} {}", s.key, flat(&s.value))).collect()
}

/// One entry of a `tests` line: what the test is, what it checks, and what
/// sets it apart from a plain one. Seen from a model that is not its host, as a
/// `relationships` test is from the model it points at, it names the host.
fn test_entry(graph: &Graph, owner: u32, t: &Node) -> String {
    let mut extra: Vec<String> = Vec::new();
    if t.warn {
        extra.push("warn".to_string());
    }
    extra.extend(settings_of(t));
    if t.test_name.is_empty() {
        let mut inside = vec!["singular".to_string()];
        inside.extend(extra);
        return format!("{} ({})", t.name, inside.join(", "));
    }
    let host = graph.host_of(t).filter(|h| h.id != graph.nodes[owner as usize].id).map(|h| h.name.as_str());
    let target = [host.unwrap_or(""), t.column.as_str()].iter().filter(|p| !p.is_empty()).copied().collect::<Vec<_>>().join(".");
    let mut entry = format!("{}: {}", t.name, t.test_name);
    if !target.is_empty() {
        entry.push_str(&format!(" on {target}"));
    }
    for e in extra {
        entry.push_str(&format!(", {e}"));
    }
    entry
}

/// The first line of a card, and the rows under it. The first line says the
/// kind, the name, how it is built and where it is written, so the rows below
/// need not. How it is built is left off when it only repeats the kind, as a
/// source's and a test's `materialized` do.
fn card(graph: &Graph, idx: u32) -> (String, Sheet) {
    let n = &graph.nodes[idx as usize];
    let kind = n.resource_type();
    let how = if n.materialized == kind { "" } else { n.materialized.as_str() };
    let first = [format!("{kind} {}", n.name), how.to_string(), ls::path_of(graph, n)]
        .into_iter()
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("  ");
    let mut sheet = Sheet::default();
    if n.disabled {
        sheet.line("status", "disabled");
    }
    match n.kind {
        Kind::Test => test_rows(graph, n, &mut sheet),
        Kind::Exposure | Kind::Operation | Kind::Other => {
            common_rows(graph, n, &mut sheet);
            description_rows(n, &mut sheet);
            sheet.line(format!("parents ({})", n.parents.len()), refs(graph, &n.parents, true));
        }
        Kind::Model | Kind::Seed | Kind::Snapshot | Kind::Analysis | Kind::Source => built_rows(graph, idx, &mut sheet),
    }
    (first, sheet)
}

/// What every kind of node can carry: its package when it is not the
/// project's, its tags, and its description whole.
fn common_rows(graph: &Graph, n: &Node, sheet: &mut Sheet) {
    if !n.package.is_empty() && n.package != graph.macros.root() {
        sheet.line("package", n.package.as_str());
    }
    sheet.line("tags", n.tags.join(", "));
}

fn description_rows(n: &Node, sheet: &mut Sheet) {
    let text = n.description.trim();
    if !text.is_empty() {
        sheet.block("description", "", text.lines().map(|l| l.trim_end().to_string()).collect());
    }
}

/// A model, seed, snapshot, analysis or source: the nodes with columns, and a
/// place in the graph.
fn built_rows(graph: &Graph, idx: u32, sheet: &mut Sheet) {
    let n = &graph.nodes[idx as usize];
    sheet.line("relation", relation(n));
    common_rows(graph, n, sheet);
    sheet.line("yml", n.yml.as_str());
    if n.is_incremental() {
        let mut how: Vec<String> = Vec::new();
        if !strategy_of(n).is_empty() {
            how.push(strategy_of(n).to_string());
        }
        if !n.unique_key.is_empty() {
            how.push(format!("unique key {}", n.unique_key.join(", ")));
        }
        sheet.line("incremental", how.join(", "));
    }
    description_rows(n, sheet);
    let lines: Vec<String> = n
        .columns
        .iter()
        .map(|c| {
            let tests: Vec<&str> = c
                .tests
                .iter()
                .map(|&t| {
                    let t = &graph.nodes[t as usize];
                    if t.test_name.is_empty() { t.name.as_str() } else { t.test_name.as_str() }
                })
                .collect();
            // A column only the warehouse has is said, since no YAML or SQL
            // of the project mentions it.
            let kind = if c.undeclared { format!("{} (catalog only)", c.data_type).trim().to_string() } else { c.data_type.clone() };
            // A field with nothing after it is left off, with its tab: a wide
            // model has hundreds of columns with no test and no description,
            // and `cut -f` reads a missing field as an empty one.
            format!("{}\t{kind}\t{}\t{}", c.name, tests.join(", "), flat(&c.description)).trim_end_matches('\t').to_string()
        })
        .collect();
    sheet.block(format!("columns ({})", lines.len()), "", lines);
    sheet.line(format!("parents ({})", n.parents.len()), refs(graph, &n.parents, true));
    sheet.line(format!("children ({})", n.children.len()), refs(graph, &n.children, true));
    let entries: Vec<String> = sorted_by_name(graph, &n.tests).iter().map(|t| test_entry(graph, idx, t)).collect();
    sheet.line(format!("tests ({})", entries.len()), entries.join("; "));
    // A source has nothing upstream to count.
    let down = graph.reach(idx, false);
    sheet.line(
        "reach",
        if n.kind == Kind::Source { format!("{down} downstream") } else { format!("{} upstream, {down} downstream", graph.reach(idx, true)) },
    );
}

/// A test: what it checks, on what, and how it is set, in place of the
/// columns and the reach of the nodes it guards.
fn test_rows(graph: &Graph, n: &Node, sheet: &mut Sheet) {
    let host = graph.host_of(n).map(|h| h.name.as_str()).unwrap_or("");
    let target = [host, n.column.as_str()].iter().filter(|p| !p.is_empty()).copied().collect::<Vec<_>>().join(".");
    sheet.line(
        "checks",
        match (n.test_name.is_empty(), target.is_empty()) {
            (true, _) => "a query of its own".to_string(),
            (false, true) => n.test_name.clone(),
            (false, false) => format!("{} on {target}", n.test_name),
        },
    );
    if n.warn {
        sheet.line("severity", "warn");
    }
    sheet.line("settings", settings_of(n).join(", "));
    if !n.namespace.is_empty() && !n.test_name.is_empty() {
        sheet.line("generic", format!("{}.{}", n.namespace, n.test_name));
    }
    common_rows(graph, n, sheet);
    description_rows(n, sheet);
    sheet.line(format!("parents ({})", n.parents.len()), refs(graph, &n.parents, false));
}

/// One node of a `--output json` answer, a line of its own. dbt's key names
/// where dbt has them, and nothing empty.
#[derive(serde::Serialize)]
struct Described<'a> {
    unique_id: &'a str,
    name: &'a str,
    resource_type: &'a str,
    #[serde(skip_serializing_if = "str::is_empty")]
    package_name: &'a str,
    /// dbt's own, inside its package for a package's node, as `--select`'s
    /// JSON gives it.
    #[serde(skip_serializing_if = "str::is_empty")]
    original_file_path: &'a str,
    /// Where the file is, or would be under the project, when that is not
    /// `original_file_path`: an installed package's node (`ls::path_of`).
    #[serde(skip_serializing_if = "String::is_empty")]
    path: String,
    #[serde(skip_serializing_if = "str::is_empty")]
    patch_path: &'a str,
    #[serde(skip_serializing_if = "str::is_empty")]
    materialized: &'a str,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    disabled: bool,
    #[serde(skip_serializing_if = "str::is_empty")]
    database: &'a str,
    #[serde(skip_serializing_if = "str::is_empty")]
    schema: &'a str,
    #[serde(skip_serializing_if = "str::is_empty")]
    alias: &'a str,
    #[serde(skip_serializing_if = "String::is_empty")]
    relation_name: String,
    /// An incremental model's only, as on its card: a view inherits the
    /// setting of its folder and does not use it.
    #[serde(skip_serializing_if = "str::is_empty")]
    incremental_strategy: &'a str,
    #[serde(skip_serializing_if = "<[String]>::is_empty")]
    unique_key: &'a [String],
    #[serde(skip_serializing_if = "str::is_empty")]
    description: &'a str,
    #[serde(skip_serializing_if = "<[String]>::is_empty")]
    tags: &'a [String],
    #[serde(skip_serializing_if = "Vec::is_empty")]
    columns: Vec<DescribedColumn<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    depends_on: Option<DependsOn<'a>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    children: Vec<&'a str>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tests: Vec<DescribedTest<'a>>,
    /// What the card's reach line counts: tests excluded, as the Catalog's do.
    #[serde(skip_serializing_if = "Option::is_none")]
    upstream_total: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    downstream_total: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    test: Option<TestFacts<'a>>,
    /// The node's own file, with `--sql`, once it was read.
    #[serde(skip_serializing_if = "Option::is_none")]
    sql: Option<String>,
}

#[derive(serde::Serialize)]
struct DescribedColumn<'a> {
    name: &'a str,
    #[serde(skip_serializing_if = "str::is_empty")]
    data_type: &'a str,
    #[serde(skip_serializing_if = "str::is_empty")]
    description: &'a str,
    /// By the name of each test node, which `tests` of the node lists in full.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tests: Vec<&'a str>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    undeclared: bool,
}

#[derive(serde::Serialize)]
struct DependsOn<'a> {
    nodes: Vec<&'a str>,
}

#[derive(serde::Serialize)]
struct DescribedTest<'a> {
    name: &'a str,
    #[serde(skip_serializing_if = "str::is_empty")]
    test_name: &'a str,
    #[serde(skip_serializing_if = "str::is_empty")]
    column: &'a str,
    /// The unique_id of the node whose properties declare it, when that is
    /// not the node described: `column` is that node's, as a `relationships`
    /// test is seen from the model it points at.
    #[serde(skip_serializing_if = "str::is_empty")]
    host: &'a str,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    warn: bool,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    settings: BTreeMap<&'static str, &'a str>,
}

/// A test node's own facts, besides the ones every node has.
#[derive(serde::Serialize)]
struct TestFacts<'a> {
    /// The generic's short name, or `singular` for a query of its own.
    checks: &'a str,
    #[serde(skip_serializing_if = "str::is_empty")]
    column: &'a str,
    /// The unique_id of the node whose properties declare it.
    #[serde(skip_serializing_if = "str::is_empty")]
    host: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    severity: Option<&'static str>,
    /// `package.name` of a generic named through a package.
    #[serde(skip_serializing_if = "String::is_empty")]
    generic: String,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    settings: BTreeMap<&'static str, &'a str>,
}

impl<'a> Described<'a> {
    fn of(graph: &'a Graph, idx: u32, sql: Option<String>) -> Described<'a> {
        let n = &graph.nodes[idx as usize];
        let ids = |list: &[u32]| {
            let mut ids: Vec<&str> = list.iter().map(|&i| graph.nodes[i as usize].id.as_str()).collect();
            // Sorted, so a node's line is the same on every run whatever
            // order the manifest listed its edges in (0055).
            ids.sort_unstable();
            ids
        };
        let parents = ids(&n.parents);
        let settings = |t: &'a Node| t.settings.iter().map(|s| (s.key, s.value.as_str())).collect::<BTreeMap<_, _>>();
        let tests = sorted_by_name(graph, &n.tests);
        let reach = !matches!(n.kind, Kind::Test | Kind::Exposure | Kind::Operation | Kind::Other);
        let path = ls::path_of(graph, n);
        Described {
            unique_id: &n.id,
            name: &n.name,
            resource_type: n.resource_type(),
            package_name: &n.package,
            original_file_path: &n.file,
            path: if path == n.file { String::new() } else { path },
            patch_path: &n.yml,
            materialized: &n.materialized,
            disabled: n.disabled,
            database: &n.database,
            schema: &n.schema,
            alias: &n.alias,
            relation_name: relation(n),
            incremental_strategy: if n.is_incremental() { strategy_of(n) } else { "" },
            unique_key: n.box_key(),
            description: n.description.trim(),
            tags: &n.tags,
            columns: n
                .columns
                .iter()
                .map(|c| {
                    let mut tests: Vec<&str> = c.tests.iter().map(|&t| graph.nodes[t as usize].name.as_str()).collect();
                    tests.sort_unstable();
                    DescribedColumn {
                        name: &c.name,
                        data_type: &c.data_type,
                        description: &c.description,
                        tests,
                        undeclared: c.undeclared,
                    }
                })
                .collect(),
            depends_on: (!parents.is_empty()).then_some(DependsOn { nodes: parents }),
            children: ids(&n.children),
            tests: tests
                .into_iter()
                .map(|t| DescribedTest {
                    name: &t.name,
                    test_name: &t.test_name,
                    column: &t.column,
                    host: graph.host_of(t).filter(|h| h.id != n.id).map(|h| h.id.as_str()).unwrap_or(""),
                    warn: t.warn,
                    settings: settings(t),
                })
                .collect(),
            upstream_total: (reach && n.kind != Kind::Source).then(|| graph.reach(idx, true)),
            downstream_total: reach.then(|| graph.reach(idx, false)),
            test: (n.kind == Kind::Test).then(|| TestFacts {
                checks: if n.test_name.is_empty() { "singular" } else { &n.test_name },
                column: &n.column,
                host: graph.host_of(n).map(|h| h.id.as_str()).unwrap_or(""),
                severity: n.warn.then_some("warn"),
                generic: if n.namespace.is_empty() || n.test_name.is_empty() {
                    String::new()
                } else {
                    format!("{}.{}", n.namespace, n.test_name)
                },
                settings: settings(n),
            }),
            sql,
        }
    }
}

// ---------------------------------------------------------------- --overview

/// The whole mode, from the command line's arguments to the exit code.
pub fn run_overview(project: &Path, manifest: Option<&Path>, catalog: Option<&Path>, depth: usize, output: Output) -> i32 {
    let (root, manifest_path, graph) = match ls::open_with_catalog(project, manifest, catalog) {
        Ok(opened) => opened,
        Err(code) => return code,
    };
    let target_dir = files::folder_of(&manifest_path).unwrap_or_else(|| root.join("target"));
    // The caches beside the manifest, as the page lists them, whichever of
    // them it would load.
    let caches = collin::discover(&target_dir);
    let whole = |path: &Path| std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf()).display().to_string();
    let found = Overview::of(
        &graph,
        &whole(&manifest_path),
        &whole(&ls::catalog_path(&root, &manifest_path, catalog)),
        &caches,
        depth,
        ls::now_secs(),
    );
    let mut err = io::stderr().lock();
    let mut out = BufWriter::new(io::stdout().lock());
    let written = match output {
        Output::Json => serde_json::to_writer(&mut out, &found).map_err(io::Error::from).and_then(|_| writeln!(out)),
        Output::Name | Output::Compact => found.write_text(&mut out),
    }
    .and_then(|_| out.flush());
    match written {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => return ANSWERED,
        Err(e) => {
            let _ = writeln!(err, "error: cannot write the answer: {e}");
            return CANNOT_RUN;
        }
    }
    let _ = ls::provenance(&root, &graph, &mut err);
    ANSWERED
}

/// What the project holds, as a JSON answer and as the text drawn from it, so
/// the two cannot differ.
#[derive(serde::Serialize)]
struct Overview {
    #[serde(skip_serializing_if = "String::is_empty")]
    project: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    dbt_version: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    adapter: String,
    manifest: ManifestFacts,
    #[serde(skip_serializing_if = "Option::is_none")]
    catalog: Option<CatalogFacts>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    column_lineage: Vec<CacheFacts>,
    /// By kind, the disabled apart: a disabled node is counted nowhere else.
    nodes: BTreeMap<String, usize>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    materializations: BTreeMap<String, usize>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    incremental_strategies: BTreeMap<String, usize>,
    macros: MacroFacts,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    selectors: Vec<SelectorFacts>,
    /// Of every enabled node, a test included: `tag:recon` selects tests, and
    /// a test carries only the tags it is given.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    tags: BTreeMap<String, usize>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    folders: Vec<FolderFacts>,
    /// The levels `folders` goes down, for the text's heading.
    #[serde(skip)]
    depth: usize,
}

#[derive(serde::Serialize)]
struct ManifestFacts {
    path: String,
    written: String,
    /// For the text only: a second apart, two runs of the JSON would differ.
    #[serde(skip)]
    age_s: u64,
}

#[derive(serde::Serialize)]
struct CatalogFacts {
    path: String,
    columns_typed: usize,
}

#[derive(serde::Serialize)]
struct CacheFacts {
    file: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    source: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    producer: String,
    written: String,
    #[serde(skip)]
    age_s: u64,
}

#[derive(serde::Serialize)]
struct MacroFacts {
    total: usize,
    /// The root project's own, once the manifest or `dbt_project.yml` names
    /// the root: until then there is no count to give.
    #[serde(skip_serializing_if = "Option::is_none")]
    project: Option<usize>,
}

#[derive(serde::Serialize)]
struct SelectorFacts {
    name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    description: String,
    /// Why this build cannot resolve it, when it cannot.
    #[serde(skip_serializing_if = "Option::is_none")]
    unsupported: Option<String>,
}

/// One folder's subtree, counted: every enabled node whose file sits under it.
#[derive(serde::Serialize)]
struct FolderFacts {
    path: String,
    depth: usize,
    #[serde(flatten)]
    tally: Tally,
}

#[derive(serde::Serialize, Default)]
struct Tally {
    #[serde(skip_serializing_if = "is_zero")]
    models: usize,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    materializations: BTreeMap<String, usize>,
    #[serde(skip_serializing_if = "is_zero")]
    sources: usize,
    #[serde(skip_serializing_if = "is_zero")]
    seeds: usize,
    #[serde(skip_serializing_if = "is_zero")]
    snapshots: usize,
    #[serde(skip_serializing_if = "is_zero")]
    tests: usize,
    #[serde(skip_serializing_if = "is_zero")]
    exposures: usize,
    #[serde(skip_serializing_if = "is_zero")]
    analyses: usize,
}

fn is_zero(n: &usize) -> bool {
    *n == 0
}

impl Tally {
    fn add(&mut self, n: &Node) {
        match n.kind {
            Kind::Model => {
                self.models += 1;
                *self.materializations.entry(materialization(n)).or_insert(0) += 1;
            }
            Kind::Source => self.sources += 1,
            Kind::Seed => self.seeds += 1,
            Kind::Snapshot => self.snapshots += 1,
            Kind::Test => self.tests += 1,
            Kind::Exposure => self.exposures += 1,
            Kind::Analysis => self.analyses += 1,
            Kind::Operation | Kind::Other => {}
        }
    }

    /// `5 models (3 view, 1 table, 1 incremental), 1 source, 4 tests`.
    fn text(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        if self.models > 0 {
            parts.push(format!("{} ({})", counted(self.models, "model", "models"), by_count(&self.materializations)));
        }
        for (n, one, many) in [
            (self.sources, "source", "sources"),
            (self.seeds, "seed", "seeds"),
            (self.snapshots, "snapshot", "snapshots"),
            (self.tests, "test", "tests"),
            (self.exposures, "exposure", "exposures"),
            (self.analyses, "analysis", "analyses"),
        ] {
            if n > 0 {
                parts.push(counted(n, one, many));
            }
        }
        parts.join(", ")
    }
}

/// How a model is built, `unset` for one whose config says nothing.
fn materialization(n: &Node) -> String {
    if n.materialized.is_empty() { "unset".to_string() } else { n.materialized.clone() }
}

/// The first `SHOWN` entries, then how many the line leaves out.
fn headed(entries: &[String]) -> String {
    let mut text = entries.iter().take(SHOWN).cloned().collect::<Vec<_>>().join(", ");
    if entries.len() > SHOWN {
        text.push_str(&format!(", +{} more", entries.len() - SHOWN));
    }
    text
}

/// The entries the most first, ties by name so the order is the same on every
/// run.
fn sorted_by_count(counts: &BTreeMap<String, usize>) -> Vec<(&String, &usize)> {
    let mut rows: Vec<(&String, &usize)> = counts.iter().collect();
    rows.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
    rows
}

/// `3 view, 1 table`.
fn by_count(counts: &BTreeMap<String, usize>) -> String {
    sorted_by_count(counts).iter().map(|(name, n)| format!("{n} {name}")).collect::<Vec<_>>().join(", ")
}

/// The kinds of `Meta::counts` in the order an overview names them.
const KINDS: [(&str, &str, &str); 10] = [
    ("model", "model", "models"),
    ("source", "source", "sources"),
    ("seed", "seed", "seeds"),
    ("snapshot", "snapshot", "snapshots"),
    ("test", "test", "tests"),
    ("exposure", "exposure", "exposures"),
    ("analysis", "analysis", "analyses"),
    ("operation", "operation", "operations"),
    ("other", "other", "others"),
    ("disabled", "disabled", "disabled"),
];

impl Overview {
    /// `manifest` and `catalog` are the paths to print, already whole; `now`
    /// the clock the ages are read against.
    fn of(graph: &Graph, manifest: &str, catalog: &str, caches: &[Available], depth: usize, now: u64) -> Overview {
        let m = &graph.meta;
        let enabled = || graph.nodes.iter().filter(|n| !n.disabled);

        let mut materializations: BTreeMap<String, usize> = BTreeMap::new();
        let mut strategies: BTreeMap<String, usize> = BTreeMap::new();
        for n in enabled().filter(|n| n.kind == Kind::Model) {
            *materializations.entry(materialization(n)).or_insert(0) += 1;
            // Only the strategies the manifest names are counted.
            if n.is_incremental() && !strategy_of(n).is_empty() {
                *strategies.entry(strategy_of(n).to_string()).or_insert(0) += 1;
            }
        }

        let mut tags: BTreeMap<String, usize> = BTreeMap::new();
        for n in enabled() {
            let mut own: Vec<&String> = n.tags.iter().collect();
            own.sort_unstable();
            own.dedup();
            for tag in own {
                *tags.entry(tag.clone()).or_insert(0) += 1;
            }
        }

        let root = graph.macros.root();
        Overview {
            // A manifest from a dbt before 1.6 does not name its project;
            // loading it took the name from dbt_project.yml.
            project: if m.project.is_empty() { root.to_string() } else { m.project.clone() },
            dbt_version: m.dbt_version.clone(),
            adapter: m.adapter.clone(),
            manifest: ManifestFacts {
                path: manifest.to_string(),
                written: iso_utc(m.manifest_mtime),
                age_s: now.saturating_sub(m.manifest_mtime),
            },
            // The catalog is merged or it is not: its time is set only by a
            // merge, so a file that was there and would not read says none.
            catalog: (m.catalog_mtime > 0).then(|| CatalogFacts { path: catalog.to_string(), columns_typed: m.catalog_columns }),
            column_lineage: caches
                .iter()
                .map(|a| CacheFacts {
                    file: a.file.clone(),
                    source: a.source.clone(),
                    producer: a.producer.clone(),
                    written: iso_utc(a.mtime),
                    age_s: now.saturating_sub(a.mtime),
                })
                .collect(),
            nodes: m.counts.clone(),
            materializations,
            incremental_strategies: strategies,
            macros: MacroFacts {
                total: graph.macros.len(),
                project: (!root.is_empty()).then(|| graph.macros.count_of(root)),
            },
            selectors: graph
                .selectors
                .list()
                .iter()
                .map(|s| SelectorFacts {
                    name: s.name.clone(),
                    description: s.description.clone(),
                    unsupported: s.unsupported.clone(),
                })
                .collect(),
            tags,
            folders: folders(graph, depth),
            depth,
        }
    }

    fn write_text(&self, out: &mut impl Write) -> io::Result<()> {
        let mut sheet = Sheet::default();
        let mut who: Vec<String> = Vec::new();
        if !self.project.is_empty() {
            who.push(self.project.clone());
        }
        if !self.dbt_version.is_empty() {
            who.push(format!("dbt {}", self.dbt_version));
        }
        if !self.adapter.is_empty() {
            who.push(format!("adapter {}", self.adapter));
        }
        sheet.line("project", who.join("  "));
        sheet.line(
            "manifest",
            format!("{}  written {} ({} ago)", self.manifest.path, self.manifest.written, ls::age(self.manifest.age_s)),
        );
        sheet.line(
            "catalog",
            match &self.catalog {
                Some(c) => format!("{}  {}", c.path, counted(c.columns_typed, "column typed", "columns typed")),
                None => "none (column types stay as declared in YAML)".to_string(),
            },
        );
        if self.column_lineage.is_empty() {
            sheet.line("column lineage", "none");
        }
        for c in &self.column_lineage {
            let by = match (c.producer.as_str(), c.source.as_str()) {
                ("", "") => String::new(),
                ("", source) => format!(" from {source}"),
                (producer, _) => format!(" by {producer}"),
            };
            sheet.line("column lineage", format!("{}{by}, written {} ({} ago)", c.file, c.written, ls::age(c.age_s)));
        }

        let kinds: Vec<String> = KINDS
            .iter()
            .filter_map(|&(key, one, many)| self.nodes.get(key).filter(|&&n| n > 0).map(|&n| counted(n, one, many)))
            .collect();
        sheet.line("nodes", kinds.join(", "));
        if !self.materializations.is_empty() {
            let how: Vec<String> = sorted_by_count(&self.materializations)
                .iter()
                .map(|(name, n)| match name.as_str() {
                    // The strategies hang off the one entry they belong to.
                    "incremental" if !self.incremental_strategies.is_empty() => {
                        let by: Vec<String> =
                            sorted_by_count(&self.incremental_strategies).iter().map(|(s, n)| format!("{s} {n}")).collect();
                        format!("{n} {name} ({})", by.join(", "))
                    }
                    _ => format!("{n} {name}"),
                })
                .collect();
            sheet.line("models", how.join(", "));
        }
        if self.macros.total > 0 {
            let own = self.macros.project.map(|n| format!(", {n} the project's")).unwrap_or_default();
            sheet.line("macros", format!("{}{own}", self.macros.total));
        }
        let selectors: Vec<String> = self
            .selectors
            .iter()
            .map(|s| if s.unsupported.is_some() { format!("{} (not resolvable here)", s.name) } else { s.name.clone() })
            .collect();
        sheet.line("selectors", headed(&selectors));
        if !self.tags.is_empty() {
            let tags: Vec<String> = sorted_by_count(&self.tags).iter().map(|(tag, n)| format!("{tag} {n}")).collect();
            sheet.line("tags", headed(&tags));
        }
        // Counts line up in one column whatever the depth of the folder, so
        // the name is padded to the longest one shown, indentation included.
        let label = |f: &FolderFacts| format!("{}{}", "  ".repeat(f.depth - 1), f.path.rsplit('/').next().unwrap_or(""));
        let width = self.folders.iter().map(|f| label(f).chars().count()).max().unwrap_or(0) + 2;
        let lines: Vec<String> = self.folders.iter().map(|f| format!("{:<width$}{}", label(f), f.tally.text())).collect();
        sheet.block("folders", format!("(by folder, {})", counted(self.depth, "level", "levels")), lines);
        sheet.write(0, out)
    }
}

/// Every enabled node filed under the folders of its file, down to `depth`
/// levels, each folder counting its whole subtree. Preorder, siblings by name:
/// numbered layer folders then sort the way they are numbered.
fn folders(graph: &Graph, depth: usize) -> Vec<FolderFacts> {
    #[derive(Default)]
    struct Folder {
        tally: Tally,
        sub: BTreeMap<String, Folder>,
    }
    let mut top: BTreeMap<String, Folder> = BTreeMap::new();
    for n in graph.nodes.iter().filter(|n| !n.disabled && !matches!(n.kind, Kind::Operation | Kind::Other)) {
        // The folders of the path, the file's own name left off. A package's
        // node nobody found on disk is filed where `dbt deps` would have put
        // it, never among the project's own (`ls::path_of`).
        let path = ls::path_of(graph, n);
        let mut parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty() && *p != ".").collect();
        parts.pop();
        let mut level = &mut top;
        for part in parts.into_iter().take(depth) {
            let f = level.entry(part.to_string()).or_default();
            f.tally.add(n);
            level = &mut f.sub;
        }
    }
    fn walk(level: BTreeMap<String, Folder>, path: &str, depth: usize, out: &mut Vec<FolderFacts>) {
        for (name, f) in level {
            let path = if path.is_empty() { name } else { format!("{path}/{name}") };
            out.push(FolderFacts { path: path.clone(), depth, tally: f.tally });
            walk(f.sub, &path, depth + 1, out);
        }
    }
    let mut out = Vec::new();
    walk(top, "", 1, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{RawCatalog, RawManifest};
    use serde_json::json;

    /// A source feeding a staging model feeding a mart with typed columns, two
    /// generic tests, a singular one and an exposure; an incremental model
    /// downstream with a generic from a package; a disabled model; a second
    /// `customers` in another package; and two named selectors.
    fn graph() -> Graph {
        graph_with(|_| {})
    }

    /// `graph()` with the manifest's JSON edited first, for a test that needs
    /// one node more than every other does.
    fn graph_with(edit: impl FnOnce(&mut serde_json::Value)) -> Graph {
        let mut manifest = json!({
            "metadata": { "dbt_version": "1.12.5", "project_name": "shop", "adapter_type": "duckdb" },
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
                    "patch_path": "shop://models/marts/schema.yml",
                    "fqn": ["shop", "marts", "orders"], "alias": "fct_orders",
                    "database": "DB", "schema": "SCHEMA", "relation_name": "DB.SCHEMA.fct_orders",
                    "description": "One row per order.\n\nSecond paragraph.", "tags": ["finance", "nightly"],
                    "config": { "materialized": "table" },
                    "columns": {
                        "order_id": { "name": "order_id", "data_type": "integer", "description": "The id of the order" },
                        "amount": { "name": "amount", "data_type": "numeric", "description": "Net amount\n  in euros" },
                    },
                },
                "model.shop.orders_daily": {
                    "name": "orders_daily", "resource_type": "model", "package_name": "shop",
                    "original_file_path": "models/marts/orders_daily.sql",
                    "fqn": ["shop", "marts", "orders_daily"],
                    "config": { "materialized": "incremental", "incremental_strategy": "merge", "unique_key": ["order_id", "day"] },
                },
                "model.shop.customers": {
                    "name": "customers", "resource_type": "model", "package_name": "shop",
                    "original_file_path": "models/marts/customers.sql",
                    "fqn": ["shop", "marts", "customers"],
                    "config": { "materialized": "table" },
                },
                "model.audit.customers": {
                    "name": "customers", "resource_type": "model", "package_name": "audit",
                    "original_file_path": "models/customers.sql",
                    "fqn": ["audit", "customers"],
                    "config": { "materialized": "view" },
                },
                "test.shop.not_null_orders_order_id": {
                    "name": "not_null_orders_order_id", "resource_type": "test", "package_name": "shop",
                    "original_file_path": "models/marts/schema.yml",
                    "column_name": "order_id", "attached_node": "model.shop.orders",
                    "test_metadata": { "name": "not_null" },
                },
                "test.shop.unique_orders_order_id": {
                    "name": "unique_orders_order_id", "resource_type": "test", "package_name": "shop",
                    "original_file_path": "models/marts/schema.yml",
                    "column_name": "order_id", "attached_node": "model.shop.orders",
                    "test_metadata": { "name": "unique" },
                },
                "test.shop.assert_positive_amounts": {
                    "name": "assert_positive_amounts", "resource_type": "test", "package_name": "shop",
                    "original_file_path": "tests/assert_positive_amounts.sql",
                    "config": { "severity": "warn" },
                },
                "test.shop.dbt_utils_unique_combination_of_columns_orders_daily": {
                    "name": "dbt_utils_unique_combination_of_columns_orders_daily", "resource_type": "test",
                    "package_name": "shop", "original_file_path": "models/marts/schema.yml",
                    "attached_node": "model.shop.orders_daily", "tags": ["nightly"],
                    "test_metadata": { "name": "unique_combination_of_columns", "namespace": "dbt_utils" },
                    "config": { "severity": "WARN", "where": "day > 0", "limit": 5 },
                },
            },
            "sources": {
                "source.shop.raw.orders": {
                    "name": "orders", "resource_type": "source", "package_name": "shop",
                    "source_name": "raw", "identifier": "orders", "database": "RAW", "schema": "SHOP",
                    "relation_name": "RAW.SHOP.orders", "description": "Orders as the shop writes them.",
                    "original_file_path": "models/staging/sources.yml",
                    "fqn": ["shop", "staging", "raw", "orders"],
                    "columns": { "id": { "name": "id", "description": "Primary key" } },
                },
            },
            "exposures": {
                "exposure.shop.weekly_report": {
                    "name": "weekly_report", "resource_type": "exposure", "package_name": "shop",
                    "original_file_path": "models/marts/exposures.yml", "description": "Read on Mondays.",
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
                "model.shop.orders_daily": ["model.shop.orders"],
                "exposure.shop.weekly_report": ["model.shop.orders"],
                "test.shop.not_null_orders_order_id": ["model.shop.orders"],
                "test.shop.unique_orders_order_id": ["model.shop.orders"],
                "test.shop.assert_positive_amounts": ["model.shop.orders"],
                "test.shop.dbt_utils_unique_combination_of_columns_orders_daily": ["model.shop.orders_daily"],
            },
            "macros": {
                "macro.shop.cents_to_dollars": { "name": "cents_to_dollars", "package_name": "shop", "original_file_path": "macros/money.sql" },
                "macro.shop.pivot": { "name": "pivot", "package_name": "shop", "original_file_path": "macros/pivot.sql" },
                "macro.dbt.ref": { "name": "ref", "package_name": "dbt", "original_file_path": "macros/ref.sql" },
                "macro.dbt.source": { "name": "source", "package_name": "dbt", "original_file_path": "macros/source.sql" },
                "macro.dbt.run_query": { "name": "run_query", "package_name": "dbt", "original_file_path": "macros/run_query.sql" },
            },
            "selectors": {
                "marts": { "name": "marts", "description": "The marts.", "definition": { "method": "path", "value": "models/marts" } },
                "ci": { "name": "ci", "definition": { "method": "state", "value": "modified" } },
            },
        });
        edit(&mut manifest);
        let raw: RawManifest = serde_json::from_value(manifest).unwrap();
        let mut g = Graph::build(raw, Path::new("/p/target/manifest.json"), MANIFEST_AT, 0);
        // The warehouse knows a column nobody declared, and types the id.
        let cat: RawCatalog = serde_json::from_value(json!({
            "nodes": { "model.shop.orders": { "columns": {
                "order_id": { "name": "order_id", "type": "BIGINT" },
                "created_at": { "name": "created_at", "type": "TIMESTAMP" },
            } } },
        }))
        .unwrap();
        g.meta.catalog_columns = g.merge_catalog(cat, CATALOG_AT);
        g.meta.catalog_mtime = CATALOG_AT;
        g
    }

    /// 2026-03-01T10:00:00Z, and a catalog three minutes later.
    const MANIFEST_AT: u64 = 1_772_359_200;
    const CATALOG_AT: u64 = MANIFEST_AT + 180;

    fn ask(names: &[&str], output: Output) -> (String, String, i32) {
        ask_in(Path::new("/nowhere"), names, false, output)
    }

    /// stdout, stderr and the exit code of one call.
    fn ask_in(root: &Path, names: &[&str], sql: bool, output: Output) -> (String, String, i32) {
        ask_on(&graph(), root, names, sql, output)
    }

    fn ask_on(g: &Graph, root: &Path, names: &[&str], sql: bool, output: Output) -> (String, String, i32) {
        let names: Vec<String> = names.iter().map(|n| n.to_string()).collect();
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = describe(g, root, &names, sql, output, &mut out, &mut err).unwrap();
        (String::from_utf8(out).unwrap(), String::from_utf8(err).unwrap(), code)
    }

    /// Lines joined the way an answer ends: each with its newline.
    fn lines(rows: &[&str]) -> String {
        rows.iter().map(|r| format!("{r}\n")).collect()
    }

    #[test]
    fn a_model_prints_its_card() {
        let (out, err, code) = ask(&["orders"], Output::Name);
        assert_eq!((code, err.as_str()), (ANSWERED, ""));
        assert_eq!(
            out,
            lines(&[
                "model orders  table  models/marts/orders.sql",
                "  relation      DB.SCHEMA.fct_orders",
                "  tags          finance, nightly",
                "  yml           models/marts/schema.yml",
                "  description",
                "    One row per order.",
                "",
                "    Second paragraph.",
                "  columns (3)",
                // Name, type, tests, description: tabs, so `cut -f` reads them, and
                // a description's own line breaks and indentation flattened.
                "    amount\tnumeric\t\tNet amount in euros",
                "    created_at\tTIMESTAMP (catalog only)",
                "    order_id\tinteger\tnot_null, unique\tThe id of the order",
                "  parents (2)   customers (table), stg_orders (view)",
                "  children (2)  orders_daily (incremental), weekly_report (exposure)",
                "  tests (3)     assert_positive_amounts (singular, warn); \
                 not_null_orders_order_id: not_null on order_id; unique_orders_order_id: unique on order_id",
                "  reach         3 upstream, 2 downstream",
            ])
        );
        // A card is read, so compact has nothing to drop from it.
        assert_eq!(ask(&["orders"], Output::Compact).0, out);
    }

    #[test]
    fn an_incremental_model_names_its_strategy_and_key() {
        let (out, _, _) = ask(&["orders_daily"], Output::Name);
        assert_eq!(
            out,
            lines(&[
                "model orders_daily  incremental  models/marts/orders_daily.sql",
                "  incremental  merge, unique key order_id, day",
                "  parents (1)  orders (table)",
                // The generic from a package, seen from its model: severity and
                // settings after what it checks.
                "  tests (1)    dbt_utils_unique_combination_of_columns_orders_daily: \
                 unique_combination_of_columns, warn, where day > 0, limit 5",
                "  reach        4 upstream, 0 downstream",
            ])
        );
    }

    #[test]
    fn a_test_says_what_it_checks_and_how_it_is_set() {
        let (out, _, _) = ask(&["not_null_orders_order_id"], Output::Name);
        assert_eq!(
            out,
            lines(&[
                "test not_null_orders_order_id  models/marts/schema.yml",
                "  checks       not_null on orders.order_id",
                "  parents (1)  orders",
            ])
        );
        let (out, _, _) = ask(&["assert_positive_amounts"], Output::Name);
        assert_eq!(
            out,
            lines(&[
                "test assert_positive_amounts  tests/assert_positive_amounts.sql",
                "  checks       a query of its own",
                "  severity     warn",
                "  parents (1)  orders",
            ])
        );
        let (out, _, _) = ask(&["dbt_utils_unique_combination_of_columns_orders_daily"], Output::Name);
        assert_eq!(
            out,
            lines(&[
                "test dbt_utils_unique_combination_of_columns_orders_daily  models/marts/schema.yml",
                "  checks       unique_combination_of_columns on orders_daily",
                "  severity     warn",
                "  settings     where day > 0, limit 5",
                "  generic      dbt_utils.unique_combination_of_columns",
                "  tags         nightly",
                "  parents (1)  orders_daily",
            ])
        );
    }

    #[test]
    fn a_source_has_nothing_upstream_and_an_exposure_only_its_parents() {
        let (out, _, _) = ask(&["raw.orders"], Output::Name);
        assert_eq!(
            out,
            lines(&[
                "source raw.orders  models/staging/sources.yml",
                "  relation      RAW.SHOP.orders",
                "  description",
                "    Orders as the shop writes them.",
                "  columns (1)",
                "    id\t\t\tPrimary key",
                "  children (1)  stg_orders (view)",
                "  reach         4 downstream",
            ])
        );
        let (out, _, _) = ask(&["weekly_report"], Output::Name);
        assert_eq!(
            out,
            lines(&[
                "exposure weekly_report  models/marts/exposures.yml",
                "  description",
                "    Read on Mondays.",
                "  parents (1)  orders (table)",
            ])
        );
    }

    #[test]
    fn a_disabled_node_says_so() {
        let (out, _, _) = ask(&["legacy"], Output::Name);
        assert!(out.starts_with("model legacy  table  models/marts/legacy.sql\n  status  disabled\n"), "{out}");
    }

    #[test]
    fn json_has_dbt_keys_and_nothing_empty() {
        let (out, _, code) = ask(&["orders"], Output::Json);
        assert_eq!(code, ANSWERED);
        assert_eq!(out.lines().count(), 1, "{out}");
        let v: serde_json::Value = serde_json::from_str(out.trim()).unwrap();
        assert_eq!(v["unique_id"], "model.shop.orders");
        assert_eq!(v["resource_type"], "model");
        assert_eq!(v["package_name"], "shop");
        assert_eq!(v["original_file_path"], "models/marts/orders.sql");
        assert_eq!(v["patch_path"], "models/marts/schema.yml");
        assert_eq!(v["materialized"], "table");
        assert_eq!((v["database"].as_str(), v["schema"].as_str(), v["alias"].as_str()), (Some("DB"), Some("SCHEMA"), Some("fct_orders")));
        assert_eq!(v["relation_name"], "DB.SCHEMA.fct_orders");
        assert_eq!(v["description"], "One row per order.\n\nSecond paragraph.");
        assert_eq!(v["tags"], json!(["finance", "nightly"]));
        assert_eq!(
            v["columns"],
            json!([
                { "name": "amount", "data_type": "numeric", "description": "Net amount\n  in euros" },
                { "name": "created_at", "data_type": "TIMESTAMP", "undeclared": true },
                {
                    "name": "order_id", "data_type": "integer", "description": "The id of the order",
                    "tests": ["not_null_orders_order_id", "unique_orders_order_id"]
                },
            ])
        );
        assert_eq!(v["depends_on"]["nodes"], json!(["model.shop.customers", "model.shop.stg_orders"]));
        assert_eq!(v["children"], json!(["exposure.shop.weekly_report", "model.shop.orders_daily"]));
        assert_eq!(
            v["tests"],
            json!([
                { "name": "assert_positive_amounts", "warn": true },
                { "name": "not_null_orders_order_id", "test_name": "not_null", "column": "order_id" },
                { "name": "unique_orders_order_id", "test_name": "unique", "column": "order_id" },
            ])
        );
        assert_eq!((v["upstream_total"].as_u64(), v["downstream_total"].as_u64()), (Some(3), Some(2)));
        // What a model does not have is left out, not written empty.
        for absent in ["disabled", "incremental_strategy", "unique_key", "test", "sql", "path"] {
            assert!(v.get(absent).is_none(), "{absent} in {v}");
        }

        // An incremental model says its strategy and key, a test what it
        // checks, with its severity and the generic's package.
        let (out, _, _) = ask(&["orders_daily"], Output::Json);
        let v: serde_json::Value = serde_json::from_str(out.trim()).unwrap();
        assert_eq!((v["incremental_strategy"].as_str(), v["unique_key"].clone()), (Some("merge"), json!(["order_id", "day"])));
        assert_eq!(
            v["tests"],
            json!([{
                "name": "dbt_utils_unique_combination_of_columns_orders_daily", "test_name": "unique_combination_of_columns",
                "warn": true, "settings": { "limit": "5", "where": "day > 0" }
            }])
        );
        let (out, _, _) = ask(&["dbt_utils_unique_combination_of_columns_orders_daily"], Output::Json);
        let v: serde_json::Value = serde_json::from_str(out.trim()).unwrap();
        assert_eq!(
            v["test"],
            json!({
                "checks": "unique_combination_of_columns", "host": "model.shop.orders_daily", "severity": "warn",
                "generic": "dbt_utils.unique_combination_of_columns", "settings": { "limit": "5", "where": "day > 0" }
            })
        );
        // Nothing to count above a test, and nothing above a source.
        assert!(v.get("upstream_total").is_none() && v.get("downstream_total").is_none(), "{v}");
        let (out, _, _) = ask(&["assert_positive_amounts"], Output::Json);
        let v: serde_json::Value = serde_json::from_str(out.trim()).unwrap();
        assert_eq!(v["test"], json!({ "checks": "singular", "severity": "warn" }));
        let (out, _, _) = ask(&["raw.orders"], Output::Json);
        let v: serde_json::Value = serde_json::from_str(out.trim()).unwrap();
        assert!(v.get("upstream_total").is_none() && v["downstream_total"] == 4, "{v}");
    }

    #[test]
    fn a_name_that_is_nothing_is_refused_with_what_is_near() {
        let (out, err, code) = ask(&["order"], Output::Name);
        assert_eq!((out.as_str(), code), ("", REFUSED));
        // Models and sources before the tests that carry their names.
        assert_eq!(err, "error: no node named order; did you mean orders, orders_daily, raw.orders, stg_orders\n");
        let (_, err, _) = ask(&["nothing_like_it"], Output::Name);
        assert_eq!(err, "error: no node named nothing_like_it\n");
        // Only a test could be meant: the tests are offered.
        let (_, err, _) = ask(&["not_null_orders_order"], Output::Name);
        assert_eq!(err, "error: no node named not_null_orders_order; did you mean not_null_orders_order_id\n");
        let (out, err, code) = ask(&[" ", ""], Output::Name);
        assert_eq!((out.as_str(), code), ("", REFUSED));
        assert!(err.starts_with("error: --describe wants"), "{err}");
    }

    #[test]
    fn two_nodes_of_one_name_ask_for_the_unique_id() {
        let (_, err, code) = ask(&["customers"], Output::Name);
        assert_eq!(code, REFUSED);
        assert_eq!(err, "error: customers names 2 nodes: write the unique_id of one, model.audit.customers, model.shop.customers\n");
        let (out, _, code) = ask(&["model.audit.customers"], Output::Name);
        assert_eq!(code, ANSWERED);
        // Its file is a package's, not the project's own of the same path.
        assert!(out.starts_with("model customers  view  dbt_packages/audit/models/customers.sql\n  package  audit\n"), "{out}");
    }

    #[test]
    fn a_name_is_matched_exactly_then_in_any_case_and_a_twin_gives_way() {
        let mut g = graph();
        let id = |g: &Graph, name: &str| g.nodes[node_named(g, name, &[]).unwrap() as usize].id.clone();
        assert_eq!(id(&g, "ORDERS"), "model.shop.orders");
        assert_eq!(id(&g, "Raw.Orders"), "source.shop.raw.orders");
        // An exposure of the same name does not rival the model, and a
        // disabled model gives way to the enabled one.
        let at = |g: &Graph, unique_id: &str| g.index[unique_id] as usize;
        let (weekly, legacy) = (at(&g, "exposure.shop.weekly_report"), at(&g, "model.shop.legacy"));
        g.nodes[weekly].name = "orders".into();
        g.nodes[legacy].name = "stg_orders".into();
        assert_eq!(id(&g, "orders"), "model.shop.orders");
        assert_eq!(id(&g, "stg_orders"), "model.shop.stg_orders");
        // Two that nothing tells apart are not guessed between, the exposure
        // having given way to them.
        let audit = at(&g, "model.audit.customers");
        g.nodes[audit].name = "orders".into();
        assert!(node_named(&g, "orders", &[]).unwrap_err().starts_with("orders names 2 nodes"));
    }

    #[test]
    fn the_two_narrowings_of_a_name_combine() {
        let mut g = graph_with(|m| {
            m["disabled"]["model.pkg.orders"] = json!([{
                "name": "orders", "resource_type": "model", "package_name": "pkg",
                "original_file_path": "models/orders.sql", "config": { "enabled": false },
            }]);
        });
        let weekly = g.index["exposure.shop.weekly_report"] as usize;
        g.nodes[weekly].name = "orders".into();
        // Three nodes of one name: the enabled ones are a model and an
        // exposure, and of those the model.
        let hits = g.nodes.iter().filter(|n| n.name == "orders" && n.kind != Kind::Source).count();
        assert_eq!(hits, 3);
        assert_eq!(g.nodes[node_named(&g, "orders", &[]).unwrap() as usize].id, "model.shop.orders");
        // Where the enabled model is not alone, the call is still refused.
        let twin = g.index["model.shop.customers"] as usize;
        g.nodes[twin].name = "orders".into();
        assert!(node_named(&g, "orders", &[]).unwrap_err().starts_with("orders names 2 nodes"));
    }

    #[test]
    fn a_version_or_a_package_before_the_name_finds_the_node() {
        let g = graph_with(|m| {
            m["nodes"]["model.shop.dim.v2"] = json!({
                "name": "dim", "resource_type": "model", "package_name": "shop",
                "original_file_path": "models/marts/dim_v2.sql", "config": { "materialized": "table" },
            });
        });
        let id = |name: &str| node_named(&g, name, &[]).map(|i| g.nodes[i as usize].id.clone());
        assert_eq!(id("dim").unwrap(), "model.shop.dim.v2");
        assert_eq!(id("dim.v2").unwrap(), "model.shop.dim.v2");
        assert_eq!(id("DIM.V2").unwrap(), "model.shop.dim.v2");
        assert_eq!(id("shop.orders_daily").unwrap(), "model.shop.orders_daily");
        // The end of a word is not a package.
        assert!(id("m.v2").unwrap_err().starts_with("no node named m.v2"));
        // A package that holds two of them is not guessed between.
        assert!(id("customers").unwrap_err().starts_with("customers names 2 nodes"));
        assert_eq!(id("shop.customers").unwrap(), "model.shop.customers");
    }

    #[test]
    fn several_names_print_several_cards_and_the_unknown_ones_are_refused_last() {
        let (out, err, code) = ask(&["stg_orders", "nothing", "weekly_report", "model.shop.stg_orders"], Output::Name);
        assert_eq!(code, REFUSED);
        assert_eq!(err, "error: no node named nothing\n");
        // One empty line between cards, and the same node named twice is one.
        assert_eq!(
            out,
            lines(&[
                "model stg_orders  view  models/staging/stg_orders.sql",
                "  tags          nightly",
                "  parents (1)   raw.orders (source)",
                "  children (1)  orders (table)",
                "  reach         1 upstream, 3 downstream",
                "",
                "exposure weekly_report  models/marts/exposures.yml",
                "  description",
                "    Read on Mondays.",
                "  parents (1)  orders (table)",
            ])
        );
        // JSON is one object a line, with no empty line between.
        let (out, _, _) = ask(&["stg_orders", "weekly_report"], Output::Json);
        let names: Vec<String> = out.lines().map(|l| serde_json::from_str::<serde_json::Value>(l).unwrap()["name"].to_string()).collect();
        assert_eq!(names, ["\"stg_orders\"", "\"weekly_report\""]);
    }

    /// A `relationships` test declared on `orders` that points at `customers`:
    /// a parent of both, attached to the first.
    fn with_relationships() -> Graph {
        graph_with(|m| {
            m["nodes"]["test.shop.relationships_orders_customer_id"] = json!({
                "name": "relationships_orders_customer_id", "resource_type": "test", "package_name": "shop",
                "original_file_path": "models/marts/schema.yml",
                "column_name": "customer_id", "attached_node": "model.shop.orders",
                "test_metadata": { "name": "relationships" },
            });
            m["parent_map"]["test.shop.relationships_orders_customer_id"] = json!(["model.shop.orders", "model.shop.customers"]);
        })
    }

    fn json_of(g: &Graph, name: &str) -> serde_json::Value {
        serde_json::to_value(Described::of(g, node_named(g, name, &[]).unwrap(), None)).unwrap()
    }

    #[test]
    fn a_test_seen_from_the_model_it_points_at_names_its_host() {
        let g = with_relationships();
        // Under customers the column is orders', so the host is said.
        assert_eq!(
            json_of(&g, "model.shop.customers")["tests"],
            json!([{
                "name": "relationships_orders_customer_id", "test_name": "relationships", "column": "customer_id",
                "host": "model.shop.orders"
            }])
        );
        // Under its own model there is nothing to add.
        let own = json_of(&g, "orders")["tests"].clone();
        let entry = own.as_array().unwrap().iter().find(|t| t["test_name"] == "relationships").unwrap();
        assert_eq!(entry, &json!({ "name": "relationships_orders_customer_id", "test_name": "relationships", "column": "customer_id" }));
        // The card says the same thing in its words.
        let (_, sheet) = card(&g, node_named(&g, "model.shop.customers", &[]).unwrap());
        let mut text = Vec::new();
        sheet.write(2, &mut text).unwrap();
        assert!(String::from_utf8(text).unwrap().contains("relationships_orders_customer_id: relationships on orders.customer_id"));
    }

    #[test]
    fn a_package_nodes_path_is_the_same_on_the_card_and_in_json() {
        let mut g = graph();
        let audit = "model.audit.customers";
        // Not looked for on disk: where `dbt deps` would have put it, while
        // dbt's own path stays the package's.
        let (first, _) = card(&g, g.index[audit]);
        assert_eq!(first, "model customers  view  dbt_packages/audit/models/customers.sql");
        let v = json_of(&g, audit);
        assert_eq!(v["original_file_path"], "models/customers.sql");
        assert_eq!(v["path"], "dbt_packages/audit/models/customers.sql");
        // Found under the project: where it is.
        let at = g.index[audit] as usize;
        g.nodes[at].place = "dbt_modules/audit/models/customers.sql".into();
        assert_eq!(card(&g, at as u32).0, "model customers  view  dbt_modules/audit/models/customers.sql");
        let v = json_of(&g, audit);
        assert_eq!(v["original_file_path"], "models/customers.sql");
        assert_eq!(v["path"], "dbt_modules/audit/models/customers.sql");
        // The project's own node has one path.
        assert!(json_of(&g, "orders").get("path").is_none());
    }

    #[test]
    fn a_terminal_escape_in_the_projects_text_never_reaches_the_answer() {
        let mut g = graph();
        let orders = g.index["model.shop.orders"] as usize;
        g.nodes[orders].description = "Red \u{1b}[31mtext\u{7}\r\nnext\tline".into();
        g.nodes[orders].tags = vec!["a\u{1b}[0m".into()];
        g.nodes[orders].name = "or\u{1b}ders".into();
        let (text, err, code) = ask_on(&g, Path::new("/nowhere"), &["model.shop.orders"], false, Output::Name);
        assert_eq!((code, err.as_str()), (ANSWERED, ""));
        assert!(text.starts_with("model orders  table"), "{text:?}");
        assert!(text.chars().all(|c| c == '\n' || c == '\t' || !c.is_control()), "{text:?}");
        assert!(text.contains("Red [31mtext\n    next\tline"), "{text:?}");
        assert!(text.contains("a[0m"), "{text:?}");
    }

    #[test]
    fn sql_follows_the_card_from_the_nodes_own_file() {
        let dir = std::env::temp_dir().join(format!("dbt-edith-describe-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("models/marts")).unwrap();
        std::fs::create_dir_all(dir.join("tests")).unwrap();
        std::fs::write(dir.join("models/marts/orders.sql"), "select\n    id,\n\n    amount\nfrom {{ ref('stg_orders') }}\n").unwrap();
        std::fs::write(dir.join("tests/assert_positive_amounts.sql"), "select 1 where false\n").unwrap();

        let (out, err, code) = ask_in(&dir, &["orders"], true, Output::Name);
        assert_eq!((code, err.as_str()), (ANSWERED, ""));
        let tail = lines(&[
            "  reach         3 upstream, 2 downstream",
            "  sql",
            "    select",
            "        id,",
            "",
            "        amount",
            "    from {{ ref('stg_orders') }}",
        ]);
        assert!(out.ends_with(&tail), "{out}");

        // A singular test has a file of its own; a generic one shares the
        // properties file, a source is declared in one, and a seed is data.
        let (out, err, _) = ask_in(&dir, &["assert_positive_amounts"], true, Output::Name);
        assert!(out.ends_with("  parents (1)  orders\n  sql\n    select 1 where false\n"), "{out}");
        assert_eq!(err, "");
        let (out, err, code) = ask_in(&dir, &["not_null_orders_order_id", "raw.orders"], true, Output::Name);
        assert!(!out.contains("  sql\n"), "{out}");
        assert_eq!(err, "not_null_orders_order_id: no SQL file of its own\nraw.orders: no SQL file of its own\n");
        assert_eq!(code, ANSWERED);

        // A package's file nobody put under the project is not the project's
        // file of the same path.
        std::fs::create_dir_all(dir.join("models")).unwrap();
        std::fs::write(dir.join("models/customers.sql"), "select 'the project''s own'\n").unwrap();
        let (out, err, _) = ask_in(&dir, &["model.audit.customers"], true, Output::Name);
        assert!(!out.contains("the project"), "{out}");
        assert_eq!(err, "customers: its file is in package audit, which is not installed under the project: run dbt deps\n");

        // A file that is not there is said, and the card is still printed.
        let (out, err, code) = ask_in(&dir, &["orders_daily"], true, Output::Name);
        assert!(out.starts_with("model orders_daily  incremental"), "{out}");
        assert!(err.starts_with("orders_daily: cannot read models/marts/orders_daily.sql: "), "{err}");
        assert_eq!(code, ANSWERED);

        // In JSON the file is a key of the node, whole, only when it was read.
        let (out, _, _) = ask_in(&dir, &["orders"], true, Output::Json);
        let v: serde_json::Value = serde_json::from_str(out.trim()).unwrap();
        assert_eq!(v["sql"], "select\n    id,\n\n    amount\nfrom {{ ref('stg_orders') }}\n");
        // Without the flag no file is read: a root that is nowhere is no error.
        assert!(ask(&["orders"], Output::Json).0.contains("\"unique_id\"") && !ask(&["orders"], Output::Json).0.contains("\"sql\""));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Two incremental models whose strategy nothing names: one with a key
    /// and one with nothing.
    fn with_unnamed_strategies() -> Graph {
        graph_with(|m| {
            for (name, config) in [
                ("events", json!({ "materialized": "incremental", "unique_key": ["id"] })),
                ("raw_events", json!({ "materialized": "incremental" })),
            ] {
                m["nodes"][format!("model.shop.{name}")] = json!({
                    "name": name, "resource_type": "model", "package_name": "shop",
                    "original_file_path": format!("models/marts/{name}.sql"), "config": config,
                });
            }
        })
    }

    #[test]
    fn an_incremental_model_whose_strategy_is_unknown_names_none() {
        let g = with_unnamed_strategies();
        let root = Path::new("/nowhere");
        // Only the key is left of the line, and nothing of it with neither.
        let (out, _, _) = ask_on(&g, root, &["events"], false, Output::Name);
        assert!(out.contains("\n  incremental  unique key id\n") && !out.contains("default"), "{out}");
        let (out, _, _) = ask_on(&g, root, &["raw_events"], false, Output::Name);
        assert!(!out.contains("\n  incremental") && !out.contains("default"), "{out}");
        let (out, _, _) = ask_on(&g, root, &["events"], false, Output::Json);
        let v: serde_json::Value = serde_json::from_str(out.trim()).unwrap();
        assert!(v.get("incremental_strategy").is_none() && v["unique_key"] == json!(["id"]), "{v}");

        // Three incremental models, and one strategy that is known.
        let o = Overview::of(&g, "/p/target/manifest.json", "/p/target/catalog.json", &[], 1, MANIFEST_AT);
        assert!(text(&o).contains("\nmodels          3 incremental (merge 1), 2 table, 2 view\n"), "{}", text(&o));
        assert_eq!(serde_json::to_value(&o).unwrap()["incremental_strategies"], json!({ "merge": 1 }));

        // None known: no parentheses, and no key in the JSON.
        let mut none = graph();
        let daily = none.index["model.shop.orders_daily"] as usize;
        none.nodes[daily].strategy.clear();
        let o = Overview::of(&none, "/p/target/manifest.json", "/p/target/catalog.json", &[], 1, MANIFEST_AT);
        assert!(text(&o).contains("\nmodels          2 table, 2 view, 1 incremental\n"), "{}", text(&o));
        assert!(serde_json::to_value(&o).unwrap().get("incremental_strategies").is_none());
    }

    #[test]
    fn the_cards_come_out_before_the_refusals_whatever_the_streams_buffer() {
        use std::{cell::RefCell, rc::Rc};
        // What reached the terminal, in the order it did, by stream.
        type Log = Rc<RefCell<Vec<(&'static str, String)>>>;
        struct Tap(&'static str, Log);
        impl Write for Tap {
            fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                self.1.borrow_mut().push((self.0, String::from_utf8_lossy(bytes).into_owned()));
                Ok(bytes.len())
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let log: Log = Rc::default();
        let (mut out, mut err) = (BufWriter::new(Tap("out", log.clone())), Tap("err", log.clone()));
        let names: Vec<String> = ["stg_orders", "nothing", "weekly_report"].iter().map(|n| n.to_string()).collect();
        let code = describe(&graph(), Path::new("/nowhere"), &names, false, Output::Name, &mut out, &mut err).unwrap();
        assert_eq!(code, REFUSED);
        let seen = log.borrow();
        let first_error = seen.iter().position(|(stream, _)| *stream == "err").unwrap();
        let last_card = seen.iter().rposition(|(stream, _)| *stream == "out").unwrap();
        assert!(last_card < first_error, "{seen:?}");
        assert!(seen[last_card].1.contains("weekly_report"), "{seen:?}");
    }

    #[test]
    fn no_name_is_refused_before_the_project_is_opened() {
        // A project that is nowhere would be the other exit code, 2, had it
        // been opened first.
        let blank = [" ".to_string(), String::new()];
        assert_eq!(run_describe(Path::new("/nowhere/at/all"), None, None, &blank, false, Output::Name), REFUSED);
    }

    fn cache(file: &str, source: &str, tool: &'static str, producer: &str, mtime: u64) -> Available {
        Available {
            file: file.into(),
            source: source.into(),
            tool,
            target: String::new(),
            generated_at: String::new(),
            mtime,
            producer: producer.into(),
        }
    }

    fn caches() -> Vec<Available> {
        vec![cache("column_lineage.collin.json", "collin", "collin", "collin 0.9.0", MANIFEST_AT + 300)]
    }

    fn overview(depth: usize) -> Overview {
        Overview::of(&graph(), "/p/target/manifest.json", "/p/target/catalog.json", &caches(), depth, MANIFEST_AT + 8 * 86_400)
    }

    fn text(o: &Overview) -> String {
        let mut out = Vec::new();
        o.write_text(&mut out).unwrap();
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn the_overview_says_what_the_project_holds() {
        assert_eq!(
            text(&overview(3)),
            lines(&[
                "project         shop  dbt 1.12.5  adapter duckdb",
                "manifest        /p/target/manifest.json  written 2026-03-01T10:00:00Z (8 days ago)",
                "catalog         /p/target/catalog.json  2 columns typed",
                "column lineage  column_lineage.collin.json by collin 0.9.0, written 2026-03-01T10:05:00Z (7 days ago)",
                // The disabled model is counted here and nowhere below.
                "nodes           5 models, 1 source, 4 tests, 1 exposure, 1 disabled",
                "models          2 table, 2 view, 1 incremental (merge 1)",
                "macros          5, 2 the project's",
                "selectors       ci (not resolvable here), marts",
                // A test counts for the tags it is given, as `tag:nightly` selects it.
                "tags            nightly 3, finance 1",
                "folders         (by folder, 3 levels)",
                // A package's model nobody found on disk is filed where `dbt deps` puts it.
                "  dbt_packages  1 model (1 view)",
                "    audit       1 model (1 view)",
                "      models    1 model (1 view)",
                "  models        4 models (2 table, 1 incremental, 1 view), 1 source, 3 tests, 1 exposure",
                "    marts       3 models (2 table, 1 incremental), 3 tests, 1 exposure",
                "    staging     1 model (1 view), 1 source",
                "  tests         1 test",
            ])
        );
        assert_eq!(text(&overview(3)), text(&overview(3)), "the same on every run");
    }

    #[test]
    fn depth_cuts_the_folders_and_not_their_counts() {
        let one = text(&overview(1));
        assert!(one.contains("folders         (by folder, 1 level)\n  dbt_packages  1 model (1 view)\n  models        4 models"), "{one}");
        assert!(!one.contains("    marts ") && !one.contains("    staging "), "{one}");
        let two = text(&overview(2));
        assert!(two.contains("    marts       3 models") && two.contains("    audit       1 model") && !two.contains("      models"), "{two}");
        // The subtree's counts are the same at every depth.
        assert!(two.contains("  models        4 models (2 table, 1 incremental, 1 view), 1 source, 3 tests, 1 exposure\n"), "{two}");
        // No levels, no folders, and no heading for none.
        let none = text(&overview(0));
        assert!(!none.contains("folders") && none.contains("tags"), "{none}");
    }

    #[test]
    fn the_overview_in_json_is_one_object() {
        let o = overview(2);
        let v = serde_json::to_value(&o).unwrap();
        assert_eq!((v["project"].as_str(), v["dbt_version"].as_str(), v["adapter"].as_str()), (Some("shop"), Some("1.12.5"), Some("duckdb")));
        // No age: two runs a second apart would differ, and the text has it.
        assert_eq!(v["manifest"], json!({ "path": "/p/target/manifest.json", "written": "2026-03-01T10:00:00Z" }));
        assert!(!serde_json::to_string(&v).unwrap().contains("age_s"), "{v}");
        assert_eq!(v["catalog"], json!({ "path": "/p/target/catalog.json", "columns_typed": 2 }));
        assert_eq!(
            v["column_lineage"],
            json!([{ "file": "column_lineage.collin.json", "source": "collin", "producer": "collin 0.9.0", "written": "2026-03-01T10:05:00Z" }])
        );
        assert_eq!(v["nodes"], json!({ "disabled": 1, "exposure": 1, "model": 5, "source": 1, "test": 4 }));
        assert_eq!(v["materializations"], json!({ "incremental": 1, "table": 2, "view": 2 }));
        assert_eq!(v["incremental_strategies"], json!({ "merge": 1 }));
        assert_eq!(v["macros"], json!({ "total": 5, "project": 2 }));
        assert_eq!(v["selectors"][1], json!({ "name": "marts", "description": "The marts." }));
        assert!(v["selectors"][0]["unsupported"].as_str().unwrap().contains("`state:`"), "{v}");
        assert_eq!(v["tags"], json!({ "finance": 1, "nightly": 3 }));
        // A flat list, parents before their children; a zero is left out.
        let paths: Vec<&str> = v["folders"].as_array().unwrap().iter().map(|f| f["path"].as_str().unwrap()).collect();
        assert_eq!(paths, ["dbt_packages", "dbt_packages/audit", "models", "models/marts", "models/staging", "tests"]);
        assert_eq!(
            v["folders"][3],
            json!({
                "path": "models/marts", "depth": 2, "models": 3, "materializations": { "incremental": 1, "table": 2 },
                "tests": 3, "exposures": 1
            })
        );
    }

    #[test]
    fn what_is_missing_is_said_and_a_long_tag_list_is_cut() {
        let mut g = graph();
        g.meta.catalog_mtime = 0;
        g.meta.project.clear();
        g.macros.set_root(String::new());
        let mut o = Overview::of(&g, "/p/target/manifest.json", "/p/target/catalog.json", &[], 1, MANIFEST_AT);
        for i in 0..17 {
            o.tags.insert(format!("t{i:02}"), 1);
        }
        let text = text(&o);
        assert!(text.starts_with("project         dbt 1.12.5  adapter duckdb\n"), "{text}");
        assert!(text.contains("catalog         none (column types stay as declared in YAML)\ncolumn lineage  none\n"), "{text}");
        // The most used first, the rest counted: 19 tags, 15 shown.
        assert!(text.contains("\ntags            nightly 3, finance 1, t00 1, t01 1,"), "{text}");
        assert!(text.contains(", t12 1, +4 more\n"), "{text}");
        assert!(serde_json::to_value(&o).unwrap().get("catalog").is_none());
    }

    #[test]
    fn a_manifest_that_does_not_name_its_project_gets_the_name_dbt_project_yml_gave() {
        let mut g = graph_with(|m| {
            m["metadata"].as_object_mut().unwrap().remove("project_name");
        });
        assert!(g.meta.project.is_empty());
        // What `load_manifest` does for a manifest from a dbt before 1.6.
        g.macros.set_root("shop".into());
        let o = Overview::of(&g, "/p/target/manifest.json", "/p/target/catalog.json", &[], 1, MANIFEST_AT);
        assert!(text(&o).starts_with("project         shop  dbt 1.12.5  adapter duckdb\n"), "{}", text(&o));
        assert_eq!(serde_json::to_value(&o).unwrap()["project"], "shop");
    }

    #[test]
    fn an_unknown_root_project_has_no_count_of_its_own_macros() {
        let mut g = graph();
        g.macros.set_root(String::new());
        let o = Overview::of(&g, "/p/target/manifest.json", "/p/target/catalog.json", &[], 1, MANIFEST_AT);
        assert!(text(&o).contains("\nmacros          5\n"), "{}", text(&o));
        assert_eq!(serde_json::to_value(&o).unwrap()["macros"], json!({ "total": 5 }));
        // A root that owns none is a count of none, which is not unknown.
        g.macros.set_root("nobody".into());
        let o = Overview::of(&g, "/p/target/manifest.json", "/p/target/catalog.json", &[], 1, MANIFEST_AT);
        assert!(text(&o).contains("\nmacros          5, 0 the project's\n"), "{}", text(&o));
        assert_eq!(serde_json::to_value(&o).unwrap()["macros"], json!({ "total": 5, "project": 0 }));
    }

    #[test]
    fn a_cache_with_no_producer_says_where_it_came_from() {
        let caches = [
            cache("column_lineage.json", "fusion", "fusion", "", MANIFEST_AT + 120),
            cache("column_lineage.other.json", "", "", "", MANIFEST_AT + 120),
        ];
        let o = Overview::of(&graph(), "/p/target/manifest.json", "/p/target/catalog.json", &caches, 1, MANIFEST_AT + 300);
        let text = text(&o);
        assert!(text.contains("\ncolumn lineage  column_lineage.json from fusion, written 2026-03-01T10:02:00Z (3 min ago)\n"), "{text}");
        assert!(text.contains("\ncolumn lineage  column_lineage.other.json, written 2026-03-01T10:02:00Z (3 min ago)\n"), "{text}");
    }
}
