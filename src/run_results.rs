//! The last dbt command's results, read from `target/run_results.json` as dbt
//! left it (0065): which command, what it selected, how each node ended and
//! how long it took, against the manifest on screen.
//!
//! dbt writes the file again on every command that runs or compiles nodes, so
//! it describes one invocation and never a history, and nothing here outlives
//! it. Every field is read as optional, and a status this module does not know
//! is passed on as written. Of the arguments only the selection leaves:
//! `invocation_command` and `vars` can carry a secret, as `env` can (0012),
//! and a result's `adapter_response` and compiled code are never read.
//!
//! A node absent from the file was not run, which says nothing about it: what
//! is counted is what ran, beside how many the manifest holds of each kind.

use crate::graph::{Graph, Node};
use serde_json::Value;
use std::collections::BTreeMap;

pub const FILE: &str = "run_results.json";

/// A message is a first line to recognise, not a log to read: the log is in
/// the terminal that ran dbt.
const MESSAGE: usize = 600;
const PROBLEMS: usize = 500;
const WARNED: usize = 500;
const SKIPPED: usize = 200;
const SLOWEST: usize = 25;
const UNKNOWN: usize = 50;

#[derive(serde::Deserialize, Default)]
pub struct Raw {
    #[serde(default)]
    metadata: RawMeta,
    #[serde(default)]
    results: Vec<RawResult>,
    #[serde(default)]
    elapsed_time: Option<f64>,
    #[serde(default)]
    args: RawArgs,
}

#[derive(serde::Deserialize, Default)]
struct RawMeta {
    #[serde(default)]
    dbt_schema_version: String,
    #[serde(default)]
    dbt_version: String,
    #[serde(default)]
    generated_at: String,
    #[serde(default)]
    invocation_id: String,
}

/// Only the selection: dbt-core names the command `which`, Fusion `command`.
#[derive(serde::Deserialize, Default)]
struct RawArgs {
    #[serde(default)]
    which: String,
    #[serde(default)]
    command: String,
    #[serde(default)]
    select: Value,
    #[serde(default)]
    exclude: Value,
    #[serde(default)]
    selector: Value,
}

#[derive(serde::Deserialize, Default)]
struct RawResult {
    #[serde(default)]
    unique_id: String,
    #[serde(default)]
    status: Value,
    #[serde(default)]
    execution_time: Option<f64>,
    #[serde(default)]
    failures: Value,
    #[serde(default)]
    message: Value,
    #[serde(default)]
    relation_name: Value,
}

pub fn parse(bytes: &[u8]) -> Result<Raw, String> {
    serde_json::from_slice(bytes).map_err(|e| e.to_string())
}

/// dbt's statuses in a few words the page knows, each kind of result having
/// its own: a model succeeds, a test passes. One this module does not know is
/// kept as written, lower case.
pub fn status(raw: &str) -> String {
    let s = raw.trim().to_ascii_lowercase().replace(['_', '-'], " ");
    match s.as_str() {
        "success" | "pass" => "ok",
        "error" | "runtime error" => "error",
        "fail" => "fail",
        "warn" => "warn",
        "skipped" => "skipped",
        "partial success" => "partial",
        "no op" | "noop" => "noop",
        "reused" => "reused",
        _ => return s,
    }
    .to_string()
}

/// What the page is told of the last run.
#[derive(serde::Serialize, Default)]
pub struct Report {
    /// `build`, `test`, `compile`..., or empty when the file does not say.
    pub command: String,
    pub selection: Vec<String>,
    pub generated_at: String,
    pub dbt_version: String,
    /// The schema's version, `v6` and the like.
    pub schema: String,
    /// When the file was written, in seconds.
    pub at: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elapsed: Option<f64>,
    /// The manifest was written by this same invocation.
    pub same_invocation: bool,
    /// The manifest was written since, by another command: the project it
    /// describes may have moved on from what ran.
    pub older_than_manifest: bool,
    /// Results naming a table the manifest does not give their node: a run
    /// for another target than the manifest's.
    pub other_target: usize,
    pub kinds: Vec<KindRun>,
    pub statuses: BTreeMap<String, usize>,
    pub problems: Listed<Entry>,
    pub warned: Listed<Entry>,
    pub skipped: Listed<Entry>,
    pub slowest: Vec<Entry>,
    pub unknown: Listed<String>,
}

/// One kind of node: how many ran, of how many the manifest holds enabled,
/// and how they ended.
#[derive(serde::Serialize)]
pub struct KindRun {
    pub kind: String,
    pub ran: usize,
    pub of: usize,
    pub statuses: BTreeMap<String, usize>,
}

/// A list cut to its first entries, with how many there were.
#[derive(serde::Serialize)]
pub struct Listed<T> {
    pub items: Vec<T>,
    pub of: usize,
}

impl<T> Default for Listed<T> {
    fn default() -> Self {
        Listed { items: Vec::new(), of: 0 }
    }
}

#[derive(serde::Serialize, Clone)]
pub struct Entry {
    pub id: String,
    pub name: String,
    pub kind: String,
    /// The file to open, empty for a node the manifest does not have.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub file: String,
    pub status: String,
    pub secs: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failures: Option<u64>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub message: String,
    /// A test only: what finds its line in its YAML (0041).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub test_name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub column: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub host: String,
}

/// The counts the Reports button and the last run's dot in the rail need,
/// without the lists.
#[derive(serde::Serialize, Clone, PartialEq, Debug)]
pub struct Counts {
    pub command: String,
    pub error: usize,
    pub fail: usize,
    pub warn: usize,
    pub partial: usize,
    /// Results that passed, were reused or had nothing to do.
    pub ok: usize,
    /// Results in a status this module has no word for.
    pub other: usize,
}

pub fn counts(raw: &Raw) -> Counts {
    let mut c = Counts { command: command(raw), error: 0, fail: 0, warn: 0, partial: 0, ok: 0, other: 0 };
    for r in &raw.results {
        match status(r.status.as_str().unwrap_or("")).as_str() {
            "error" => c.error += 1,
            "fail" => c.fail += 1,
            "warn" => c.warn += 1,
            "partial" => c.partial += 1,
            "ok" | "noop" | "reused" => c.ok += 1,
            "skipped" => {}
            _ => c.other += 1,
        }
    }
    c
}

/// The command as it is typed: dbt-core names `docs generate` by its last
/// word alone.
fn command(raw: &Raw) -> String {
    let which = if raw.args.which.is_empty() { &raw.args.command } else { &raw.args.which };
    match which.trim() {
        "generate" => "docs generate".to_string(),
        other => other.to_string(),
    }
}

/// The kinds of node the graph holds, which a result can be missing from:
/// a macro run by `run-operation` or `show --inline`'s query never is one.
const INDEXED: [&str; 8] = ["model", "seed", "snapshot", "test", "source", "analysis", "exposure", "operation"];

/// `--select a b`, as dbt was given it, a list or a string alike.
fn terms(flag: &str, v: &Value) -> Option<String> {
    let words: Vec<String> = match v {
        Value::String(s) if !s.trim().is_empty() => vec![s.trim().to_string()],
        Value::Array(a) => a.iter().filter_map(|x| x.as_str()).map(str::trim).filter(|s| !s.is_empty()).map(String::from).collect(),
        _ => Vec::new(),
    };
    (!words.is_empty()).then(|| format!("{flag} {}", words.join(" ")))
}

fn text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// A relation as the warehouse would name it either way: quotes and case are
/// how dbt spelled it, not which table it is.
fn relation_key(s: &str) -> String {
    s.chars().filter(|c| !matches!(c, '"' | '`' | '[' | ']')).collect::<String>().to_ascii_lowercase()
}

fn cut(s: &str) -> String {
    let s = s.trim();
    if s.chars().count() <= MESSAGE {
        return s.to_string();
    }
    let mut out: String = s.chars().take(MESSAGE).collect();
    out.push('…');
    out
}

fn kind_of(id: &str, node: Option<&Node>) -> String {
    match node {
        Some(n) => n.kind.as_str().to_string(),
        None => id.split('.').next().unwrap_or("").to_string(),
    }
}

/// Whether a node of this kind runs on its own: an ephemeral model is
/// inlined into the models that read it and never in the results.
fn runs(n: &Node) -> bool {
    !n.disabled && n.materialized != "ephemeral"
}

/// The run against the manifest the page holds. `at` is the file's time and
/// `manifest_invocation` the id the manifest was written under.
pub fn report(raw: &Raw, graph: &Graph, at: u64, manifest_invocation: &str) -> Report {
    let invocation = raw.metadata.invocation_id.trim();
    let same_invocation = !invocation.is_empty() && invocation == manifest_invocation.trim();
    let schema = raw.metadata.dbt_schema_version.rsplit('/').next().unwrap_or("").trim_end_matches(".json").to_string();
    let selection = [terms("--select", &raw.args.select), terms("--exclude", &raw.args.exclude), terms("--selector", &raw.args.selector)]
        .into_iter()
        .flatten()
        .collect();

    let mut kinds: BTreeMap<String, KindRun> = BTreeMap::new();
    let mut statuses: BTreeMap<String, usize> = BTreeMap::new();
    let (mut problems, mut warned, mut skipped, mut all) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let mut unknown = Vec::new();
    let mut other_target = 0;
    for r in &raw.results {
        let node = graph.index.get(&r.unique_id).map(|&i| &graph.nodes[i as usize]);
        let unit = graph.unit_tests.get(&r.unique_id);
        let kind = kind_of(&r.unique_id, node);
        // A node of a kind the graph holds and missing from it is gone from
        // the manifest; a macro or an inline query never was in it.
        let gone = node.is_none() && unit.is_none() && INDEXED.contains(&kind.as_str());
        if gone && !r.unique_id.is_empty() {
            unknown.push(r.unique_id.clone());
        }
        let st = status(r.status.as_str().unwrap_or(""));
        *statuses.entry(st.clone()).or_default() += 1;
        if !gone {
            let k = kinds.entry(kind.clone()).or_insert_with(|| KindRun { kind: kind.clone(), ran: 0, of: 0, statuses: BTreeMap::new() });
            k.ran += 1;
            *k.statuses.entry(st.clone()).or_default() += 1;
        }
        if let (Some(n), Some(rel)) = (node, r.relation_name.as_str()) {
            if !rel.is_empty() && !n.relation.is_empty() && relation_key(rel) != relation_key(&n.relation) {
                other_target += 1;
            }
        }
        let entry = Entry {
            id: r.unique_id.clone(),
            name: node
                .map(|n| n.name.clone())
                .or_else(|| unit.map(|u| u.name.clone()))
                .unwrap_or_else(|| r.unique_id.rsplit('.').next().unwrap_or("").to_string()),
            kind,
            file: node.map(|n| n.on_disk().to_string()).or_else(|| unit.map(|u| u.file.clone())).unwrap_or_default(),
            status: st.clone(),
            secs: r.execution_time.unwrap_or(0.0),
            failures: r.failures.as_u64(),
            message: cut(&text(&r.message)),
            test_name: node.map(|n| n.test_name.clone()).unwrap_or_default(),
            column: node.map(|n| n.column.clone()).unwrap_or_default(),
            host: node.and_then(|n| graph.host_of(n)).map(|h| h.name.clone()).unwrap_or_default(),
        };
        match st.as_str() {
            "error" | "fail" => problems.push(entry.clone()),
            "warn" | "partial" => warned.push(entry.clone()),
            "skipped" => skipped.push(entry.clone()),
            _ => {}
        }
        all.push(entry);
    }
    for n in graph.nodes.iter().filter(|n| runs(n)) {
        if let Some(k) = kinds.get_mut(n.kind.as_str()) {
            k.of += 1;
        }
    }
    if let Some(k) = kinds.get_mut("unit_test") {
        k.of = graph.unit_tests.len();
    }

    // In an order that is the same on every start (0055): what to look at
    // first, then by name.
    let by_name = |a: &Entry, b: &Entry| a.name.cmp(&b.name).then_with(|| a.id.cmp(&b.id));
    problems.sort_by(|a, b| (a.status != "error").cmp(&(b.status != "error")).then_with(|| by_name(a, b)));
    warned.sort_by(by_name);
    skipped.sort_by(by_name);
    all.sort_by(|a, b| b.secs.total_cmp(&a.secs).then_with(|| a.id.cmp(&b.id)));
    all.truncate(SLOWEST);
    unknown.sort();

    let listed = |mut items: Vec<Entry>, cap: usize| {
        let of = items.len();
        items.truncate(cap);
        Listed { items, of }
    };
    let manifest_at = graph.meta.manifest_mtime;
    Report {
        command: command(raw),
        selection,
        generated_at: raw.metadata.generated_at.clone(),
        dbt_version: raw.metadata.dbt_version.clone(),
        schema,
        at,
        elapsed: raw.elapsed_time,
        same_invocation,
        older_than_manifest: !same_invocation && manifest_at > at,
        other_target,
        kinds: kinds.into_values().collect(),
        statuses,
        problems: listed(problems, PROBLEMS),
        warned: listed(warned, WARNED),
        skipped: listed(skipped, SKIPPED),
        slowest: all,
        unknown: Listed { of: unknown.len(), items: unknown.into_iter().take(UNKNOWN).collect() },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::RawManifest;

    fn graph() -> Graph {
        let raw: RawManifest = serde_json::from_value(serde_json::json!({
            "metadata": { "invocation_id": "inv-1" },
            "nodes": {
                "model.shop.orders": { "name": "orders", "resource_type": "model", "package_name": "shop",
                    "original_file_path": "models/orders.sql", "fqn": ["shop", "orders"],
                    "relation_name": "\"DB\".\"MARTS\".\"ORDERS\"" },
                "model.shop.customers": { "name": "customers", "resource_type": "model", "package_name": "shop",
                    "original_file_path": "models/customers.sql", "fqn": ["shop", "customers"] },
                "model.shop.payments": { "name": "payments", "resource_type": "model", "package_name": "shop",
                    "original_file_path": "models/payments.sql", "fqn": ["shop", "payments"] },
                "seed.shop.raw": { "name": "raw", "resource_type": "seed", "package_name": "shop",
                    "original_file_path": "seeds/raw.csv", "fqn": ["shop", "raw"] },
                "model.shop.inlined": { "name": "inlined", "resource_type": "model", "package_name": "shop",
                    "original_file_path": "models/inlined.sql", "fqn": ["shop", "inlined"],
                    "config": { "materialized": "ephemeral" } },
                "test.shop.not_null_orders_id.abc": { "name": "not_null_orders_id", "resource_type": "test",
                    "package_name": "shop", "original_file_path": "models/schema.yml", "fqn": ["shop", "not_null_orders_id"],
                    "column_name": "id", "attached_node": "model.shop.orders",
                    "test_metadata": { "name": "not_null", "kwargs": {} },
                    "depends_on": { "nodes": ["model.shop.orders"] } },
            },
            "unit_tests": {
                "unit_test.shop.orders.counts_refunds": { "name": "counts_refunds", "resource_type": "unit_test",
                    "original_file_path": "models/orders.yml" },
            },
        }))
        .unwrap();
        Graph::build(raw, std::path::Path::new("/p/target/manifest.json"), 100, 0)
    }

    fn v6() -> Raw {
        parse(
            serde_json::json!({
                "metadata": { "dbt_schema_version": "https://schemas.getdbt.com/dbt/run-results/v6.json",
                    "dbt_version": "1.9.0", "generated_at": "2026-01-01T06:00:00Z", "invocation_id": "inv-2",
                    "env": { "DBT_ENV_SECRET_PW": "hunter2" } },
                "elapsed_time": 12.5,
                "args": { "which": "build", "select": ["orders+", "raw"], "exclude": [], "selector": null,
                    "vars": { "password": "hunter2" }, "invocation_command": "dbt build --vars '{password: hunter2}'" },
                "results": [
                    { "unique_id": "model.shop.orders", "status": "error", "execution_time": 2.0,
                      "message": "Database Error in model orders\n  syntax error", "relation_name": "\"DB\".\"MARTS\".\"ORDERS\"",
                      "adapter_response": { "query_id": "q-1" }, "compiled_code": "select hunter2" },
                    { "unique_id": "model.shop.customers", "status": "success", "execution_time": 9.5, "message": "OK" },
                    { "unique_id": "test.shop.not_null_orders_id.abc", "status": "skipped", "execution_time": 0 },
                    { "unique_id": "seed.shop.raw", "status": "partial success", "execution_time": 1.0,
                      "batch_results": { "successful": [], "failed": [] } },
                    { "unique_id": "model.shop.gone", "status": "no-op", "execution_time": 0.1 },
                ],
            })
            .to_string()
            .as_bytes(),
        )
        .unwrap()
    }

    #[test]
    fn statuses_read_in_a_few_words_and_an_unknown_one_as_written() {
        for (raw, want) in [("success", "ok"), ("pass", "ok"), ("error", "error"), ("runtime error", "error"),
            ("RUNTIME_ERROR", "error"), ("fail", "fail"), ("warn", "warn"), ("skipped", "skipped"),
            ("partial success", "partial"), ("no-op", "noop"), ("reused", "reused"), ("Interrupted", "interrupted")]
        {
            assert_eq!(status(raw), want, "{raw}");
        }
    }

    #[test]
    fn a_build_is_read_against_the_manifest() {
        let g = graph();
        let r = report(&v6(), &g, 200, "inv-1");
        assert_eq!(r.command, "build");
        assert_eq!(r.selection, vec!["--select orders+ raw"]);
        assert_eq!((r.schema.as_str(), r.dbt_version.as_str(), r.elapsed), ("v6", "1.9.0", Some(12.5)));
        assert!(!r.same_invocation && !r.older_than_manifest, "the run came after the manifest");
        assert_eq!(r.other_target, 0, "the same table, quoted the same way");
        let models = r.kinds.iter().find(|k| k.kind == "model").unwrap();
        assert_eq!((models.ran, models.of), (2, 3), "payments did not run, the gone model is not counted, the ephemeral one never runs");
        assert_eq!(r.problems.items.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(), vec!["orders"]);
        let e = &r.problems.items[0];
        assert_eq!((e.file.as_str(), e.message.as_str()), ("models/orders.sql", "Database Error in model orders\n  syntax error"));
        assert_eq!(r.warned.items[0].status, "partial");
        let skipped = &r.skipped.items[0];
        assert_eq!((skipped.test_name.as_str(), skipped.column.as_str(), skipped.host.as_str()), ("not_null", "id", "orders"));
        assert_eq!(r.slowest[0].name, "customers");
        assert_eq!(r.unknown.items, vec!["model.shop.gone"]);
        assert_eq!(r.statuses.get("noop"), Some(&1));
    }

    #[test]
    fn a_unit_test_a_macro_and_an_inline_query_are_no_node_gone() {
        let g = graph();
        let raw = parse(br#"{"args":{"which":"build"},"results":[
            {"unique_id":"unit_test.shop.orders.counts_refunds","status":"fail","failures":1},
            {"unique_id":"macro.shop.grant_select","status":"success"},
            {"unique_id":"sql_operation.shop.inline_query","status":"success"}]}"#)
        .unwrap();
        let r = report(&raw, &g, 200, "");
        assert_eq!(r.unknown.of, 0, "{:?}", r.unknown.items);
        let unit = &r.problems.items[0];
        assert_eq!((unit.name.as_str(), unit.file.as_str(), unit.kind.as_str()), ("counts_refunds", "models/orders.yml", "unit_test"));
        let units = r.kinds.iter().find(|k| k.kind == "unit_test").unwrap();
        assert_eq!((units.ran, units.of), (1, 1));
        assert!(r.kinds.iter().any(|k| k.kind == "macro" && k.ran == 1));
    }

    #[test]
    fn docs_generate_is_named_as_it_is_typed() {
        let raw = parse(br#"{"args":{"which":"generate"},"results":[]}"#).unwrap();
        assert_eq!(report(&raw, &graph(), 200, "").command, "docs generate");
        let op = parse(br#"{"args":{"which":"run-operation"},"results":[]}"#).unwrap();
        assert_eq!(counts(&op).command, "run-operation");
    }

    #[test]
    fn nothing_secret_leaves() {
        let body = serde_json::to_string(&report(&v6(), &graph(), 200, "")).unwrap();
        for leak in ["hunter2", "query_id", "invocation_command", "DBT_ENV"] {
            assert!(!body.contains(leak), "{leak} in {body}");
        }
    }

    #[test]
    fn the_manifest_tells_whose_run_it_is() {
        let g = graph();
        let mut raw = v6();
        raw.metadata.invocation_id = "inv-1".into();
        assert!(report(&raw, &g, 50, "inv-1").same_invocation, "written by the run that wrote the manifest");
        assert!(!report(&raw, &g, 50, "inv-1").older_than_manifest, "even though dbt wrote the manifest first");
        raw.metadata.invocation_id = "inv-0".into();
        assert!(report(&raw, &g, 50, "inv-1").older_than_manifest, "a parse since rewrote the manifest");
    }

    #[test]
    fn a_table_of_another_target_is_counted() {
        let g = graph();
        let mut raw = v6();
        raw.results[0].relation_name = Value::String("\"DB\".\"DEV_ME\".\"ORDERS\"".into());
        assert_eq!(report(&raw, &g, 200, "").other_target, 1);
    }

    #[test]
    fn a_fusion_file_and_a_test_run_read_too() {
        let g = graph();
        let raw = parse(br#"{"metadata":{"dbt_version":"2.0.0","invocation_id":"x","env":{}},
            "args":{"command":"test","which":"","__other__":{"command":"test"}},
            "results":[{"unique_id":"test.shop.not_null_orders_id.abc","status":"Fail","failures":3,
                        "message":"Got 3 results","execution_time":0.2}]}"#)
        .unwrap();
        let r = report(&raw, &g, 200, "");
        assert_eq!(r.command, "test");
        assert_eq!(r.kinds.len(), 1, "only the tests ran");
        assert_eq!((r.problems.items[0].status.as_str(), r.problems.items[0].failures), ("fail", Some(3)));
        assert_eq!(counts(&raw), Counts { command: "test".into(), error: 0, fail: 1, warn: 0, partial: 0, ok: 0, other: 0 });
        let part = parse(br#"{"metadata":{},"args":{"which":"build"},"results":[{"unique_id":"model.shop.orders","status":"partial success"}]}"#).unwrap();
        assert_eq!(counts(&part).partial, 1, "a partial success is counted");
        let word = |s: &str| {
            let raw = format!(r#"{{"metadata":{{}},"args":{{"which":"build"}},"results":[{{"unique_id":"model.shop.orders","status":"{s}"}}]}}"#);
            let c = counts(&parse(raw.as_bytes()).unwrap());
            (c.ok, c.other, c.error + c.fail + c.warn + c.partial)
        };
        assert_eq!(word("success"), (1, 0, 0));
        assert_eq!(word("no-op"), (1, 0, 0));
        assert_eq!(word("reused"), (1, 0, 0));
        assert_eq!(word("Interrupted"), (0, 1, 0), "a status with no word is counted apart, not as passed");
        assert_eq!(word("skipped"), (0, 0, 0), "a skipped result is neither passed nor a problem");
        let none = counts(&parse(br#"{"metadata":{},"args":{"which":"build"},"results":[]}"#).unwrap());
        assert_eq!((none.ok, none.other, none.error + none.fail + none.warn + none.partial), (0, 0, 0), "no results at all");
        let empty = parse(br#"{"metadata":{},"results":[],"args":{"which":"compile"}}"#).unwrap();
        let r = report(&empty, &g, 200, "");
        assert_eq!((r.command.as_str(), r.kinds.len(), r.slowest.len()), ("compile", 0, 0));
        assert!(parse(b"{\"results\": [").is_err(), "a file dbt is still writing");
    }

    #[test]
    fn a_long_message_and_a_long_list_are_cut() {
        let g = graph();
        let mut raw = v6();
        raw.results[0].message = Value::String("x".repeat(2000));
        let r = report(&raw, &g, 200, "");
        assert_eq!(r.problems.items[0].message.chars().count(), MESSAGE + 1);
        raw.results = (0..PROBLEMS + 7)
            .map(|i| RawResult { unique_id: format!("model.shop.m{i:04}"), status: Value::String("error".into()), ..Default::default() })
            .collect();
        let r = report(&raw, &g, 200, "");
        assert_eq!((r.problems.items.len(), r.problems.of), (PROBLEMS, PROBLEMS + 7));
        assert_eq!((r.unknown.items.len(), r.unknown.of), (UNKNOWN, PROBLEMS + 7));
    }

    #[test]
    fn the_same_file_answers_the_same_bytes() {
        let g = graph();
        let first = serde_json::to_string(&report(&v6(), &g, 200, "")).unwrap();
        for _ in 0..8 {
            assert_eq!(serde_json::to_string(&report(&v6(), &graph(), 200, "")).unwrap(), first);
        }
    }
}
