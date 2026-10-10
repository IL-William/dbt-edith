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
use crate::graph::{Graph, Node};
use crate::select::{self, Resolved, SelectError, Tests};
use crate::{api, files, freshness, git};
use std::io::{self, BufWriter, Write};
use std::path::Path;

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
}

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
    let root = match project.canonicalize() {
        Ok(p) => files::plain(p),
        Err(e) => {
            eprintln!("error: cannot open {}: {e}", project.display());
            return CANNOT_RUN;
        }
    };
    let manifest_path = manifest.map(Path::to_path_buf).unwrap_or_else(|| root.join("target").join("manifest.json"));
    if !manifest_path.is_file() {
        eprintln!("error: no manifest at {}: run dbt parse, or pass --manifest", manifest_path.display());
        return CANNOT_RUN;
    }
    let graph = match api::load_manifest(&root, &manifest_path) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("error: cannot read {}: {e}", manifest_path.display());
            return CANNOT_RUN;
        }
    };

    let mut err = io::stderr().lock();
    // Buffered, because a selection of thousands of names would otherwise be
    // one write per line.
    let mut out = BufWriter::new(io::stdout().lock());
    let code = match answer(&graph, query, output, &mut out, &mut err).and_then(|code| out.flush().map(|_| code)) {
        Ok(code) => code,
        // `| head` closing the pipe is someone having read enough, not a failure.
        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => return ANSWERED,
        Err(e) => {
            let _ = writeln!(err, "error: cannot write the answer: {e}");
            return CANNOT_RUN;
        }
    };
    // After the answer is out, so a slow walk of a large project never holds
    // it back.
    let _ = provenance(&root, &graph, &mut err);
    code
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
    for n in &nodes {
        match output {
            Output::Name => writeln!(out, "{}", n.name)?,
            Output::Json => {
                serde_json::to_writer(&mut *out, &Listed::of(graph, n))?;
                writeln!(out)?;
            }
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
/// prints only in its full JSON and which are what tells an edge.
#[derive(serde::Serialize)]
struct Listed<'a> {
    unique_id: &'a str,
    /// The name `--output name` prints: `source_name.table` for a source.
    name: &'a str,
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
            resource_type: n.id.split('.').next().unwrap_or(""),
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

/// Which manifest answered and whether the project moved since, because the
/// answer is the manifest's and not the working tree's.
fn provenance(root: &Path, graph: &Graph, err: &mut impl Write) -> io::Result<()> {
    let m = &graph.meta;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
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

fn age(secs: u64) -> String {
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
        let raw: RawManifest = serde_json::from_value(json!({
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
        }))
        .unwrap();
        Graph::build(raw, Path::new("manifest.json"), 0, 0)
    }

    fn line(select: &str, exclude: &str) -> Query {
        Query::Line { select: select.to_string(), exclude: exclude.to_string() }
    }

    /// stdout, stderr and the exit code of one query.
    fn ask(query: &Query, output: Output) -> (String, String, i32) {
        let g = graph();
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = answer(&g, query, output, &mut out, &mut err).unwrap();
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
