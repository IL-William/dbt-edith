//! What in a node's SQL or in a macro reads the environment it compiles in, so
//! SQL shown for another environment can say where it may differ beyond its
//! relations. Only findings are kept, never a body (0139).
//!
//! The reading is a scan of the Jinja blocks for the few things whose answer
//! depends on where dbt ran: a key of `target`, an `env_var()`, a call that
//! asks the warehouse, `is_incremental()`. Nothing is evaluated, so a branch on
//! `target.name` is named, not followed.
//!
//! Invariants: a `{# #}` comment and a `{% raw %}` block are text, not reads;
//! the name of an `env_var` that reads as a credential is withheld here, at the
//! scan, so no later path can send it (0012, 0090); a line is counted from the
//! first line of the text scanned, which for a macro is the line its
//! `{% macro %}` opens on.

use crate::envs;
use crate::graph::{Graph, Kind};
use std::collections::HashSet;

/// What a scan looks for, cheaply, before it reads any Jinja: most bodies
/// hold none of these and cost one pass per word.
const WORDS: &[&str] = &["target", "env_var", "run_query", "load_result", "load_relation", "adapter.get_", "is_incremental"];

/// Keys of `target` that read the same in every environment.
const SAME_EVERYWHERE: &[&str] = &["type", "threads"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum What {
    /// A key of `target`: `name`, `schema`...
    Target(String),
    /// `env_var()`, with the variable's name, or nothing when it is withheld.
    EnvVar(String),
    /// A call that asks the warehouse: `run_query`, `adapter.get_relation`...
    Warehouse(String),
    /// `is_incremental()`, whose answer depends on the table existing.
    Incremental,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Read {
    pub what: What,
    /// From 1, in the text scanned.
    pub line: u32,
}

/// Every read in `sql`, each kind of read once, at its first line.
pub fn scan(sql: &str) -> Vec<Read> {
    let mut out = Vec::new();
    if !WORDS.iter().any(|w| sql.contains(w)) {
        return out;
    }
    let b = sql.as_bytes();
    let mut lines = Lines { at: 0, line: 1 };
    let mut i = 0;
    while let Some(found) = sql[i..].find('{') {
        let at = i + found;
        let close = match b.get(at + 1) {
            Some(b'{') => "}}",
            Some(b'%') => "%}",
            Some(b'#') => "#}",
            _ => {
                i = at + 1;
                continue;
            }
        };
        let start = at + 2;
        let Some(len) = sql[start..].find(close) else { break };
        let end = start + len;
        i = end + 2;
        let inner = sql[start..end].trim_matches(|c: char| c == '-' || c == '+' || c.is_whitespace());
        if close == "%}" && inner == "raw" {
            match crate::macro_use::endraw(&sql[i..]) {
                Some(after) => i = sql.len() - after.len(),
                None => break,
            }
            continue;
        }
        if close != "#}" {
            reads_in(sql, start, end, &mut lines, &mut out);
        }
    }
    out
}

/// Line numbers for positions met in increasing order, counted once.
struct Lines {
    at: usize,
    line: u32,
}

impl Lines {
    fn of(&mut self, sql: &str, pos: usize) -> u32 {
        self.line += sql.as_bytes()[self.at..pos].iter().filter(|&&c| c == b'\n').count() as u32;
        self.at = pos;
        self.line
    }
}

fn reads_in(sql: &str, from: usize, to: usize, lines: &mut Lines, out: &mut Vec<Read>) {
    let b = sql.as_bytes();
    let mut i = from;
    let mut prev = b' ';
    while i < to {
        let c = b[i];
        if c == b'\'' || c == b'"' {
            i += 1;
            while i < to && b[i] != c {
                i += if b[i] == b'\\' { 2 } else { 1 };
            }
            i += 1;
            prev = c;
        } else if c.is_ascii_alphabetic() || c == b'_' {
            let start = i;
            while i < to && (b[i].is_ascii_alphanumeric() || b[i] == b'_' || b[i] == b'.') {
                i += 1;
            }
            let word = &sql[start..i];
            let mut j = i;
            while j < to && b[j].is_ascii_whitespace() {
                j += 1;
            }
            let next = if j < to { b[j] } else { b' ' };
            let last = word.rsplit('.').next().unwrap_or(word);
            // A name after `.` is a method of something, never the thing itself.
            if prev != b'.' {
                let mut found = None;
                if word == "target" && next == b'[' {
                    found = string_at(sql, j + 1, to).map(What::Target);
                } else if let Some(key) = word.strip_prefix("target.") {
                    let key = key.split('.').next().unwrap_or("");
                    found = if key == "get" && next == b'(' {
                        string_at(sql, j + 1, to).map(What::Target)
                    } else {
                        Some(What::Target(key.to_string()))
                    };
                } else if next == b'(' {
                    if word == "env_var" {
                        let name = string_at(sql, j + 1, to).unwrap_or_default();
                        let name = if envs::is_secret(&name) || envs::sensitive_name(&name) { String::new() } else { name };
                        found = Some(What::EnvVar(name));
                    } else if word.starts_with("adapter.get_") || matches!(last, "run_query" | "load_result" | "load_relation") {
                        found = Some(What::Warehouse(word.to_string()));
                    } else if last == "is_incremental" {
                        found = Some(What::Incremental);
                    }
                }
                match found {
                    Some(What::Target(key)) if key.is_empty() || SAME_EVERYWHERE.contains(&key.as_str()) => {}
                    Some(what) if !out.iter().any(|r| r.what == what) => out.push(Read { what, line: lines.of(sql, start) }),
                    _ => {}
                }
            }
            prev = b[i - 1];
        } else {
            if !c.is_ascii_whitespace() {
                prev = c;
            }
            i += 1;
        }
    }
}

/// The quoted string starting at the first non-space byte from `from`, or None
/// when that byte is not a quote or the string does not close before `to`.
fn string_at(sql: &str, from: usize, to: usize) -> Option<String> {
    let b = sql.as_bytes();
    let mut i = from;
    while i < to && b[i].is_ascii_whitespace() {
        i += 1;
    }
    let quote = *b.get(i).filter(|c| **c == b'\'' || **c == b'"')?;
    let close = sql[i + 1..to].find(quote as char)? + i + 1;
    Some(sql[i + 1..close].to_string())
}

/// One thing a node's SQL, or a macro it reaches, reads from the environment.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Finding {
    /// The macro's unique_id, empty for the node's own SQL.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub macro_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub macro_name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub package: String,
    /// The macro's file inside the project, when it is written there, which a
    /// click opens; set by the route, which knows the project.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub file: String,
    /// `target`, `env_var`, `warehouse`, `incremental` or `placing`.
    pub kind: &'static str,
    /// The key, the variable or the call; empty for the last two kinds, and
    /// for a variable whose name is withheld.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub detail: String,
    /// A placing finding whose read sits in a macro the override calls: that
    /// macro's name, and no line, since the line would be in another text.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub through: String,
    /// In the node's file, or in the macro's own text; 0 with `through`.
    pub line: u32,
}

fn finding(macro_id: &str, macro_name: &str, package: &str, r: &Read) -> Finding {
    let (kind, detail) = match &r.what {
        What::Target(k) => ("target", k.clone()),
        What::EnvVar(n) => ("env_var", n.clone()),
        What::Warehouse(c) => ("warehouse", c.clone()),
        What::Incremental => ("incremental", String::new()),
    };
    Finding {
        macro_id: macro_id.into(),
        macro_name: macro_name.into(),
        package: package.into(),
        file: String::new(),
        kind,
        detail,
        through: String::new(),
        line: r.line,
    }
}

/// How many lines the whitespace a file opens with spans. dbt-core keeps a
/// node's `raw_code` stripped, so the lines scanned in it start that many lines
/// above the file's own.
pub fn leading_lines(file: &str) -> u32 {
    let lead = &file[..file.len() - file.trim_start().len()];
    lead.matches('\n').count() as u32
}

/// The node's own findings, those with no macro, moved down by `by` lines.
pub fn shift_own(findings: &mut [Finding], by: u32) {
    for f in findings.iter_mut().filter(|f| f.macro_id.is_empty()) {
        f.line += by;
    }
}

/// What the SQL of node `idx` may read from the environment beyond the
/// relations the page swaps: its own SQL, then each macro it reaches, then
/// the project's own placing macros when one reads the target. Node first,
/// then by macro name, the same on every start (0055).
pub fn for_node(graph: &Graph, idx: u32) -> Vec<Finding> {
    let n = &graph.nodes[idx as usize];
    let adapter = graph.meta.adapter.as_str();
    let macros = &graph.macros;
    let internal = |m: u32| crate::macro_use::internal_package(adapter, &macros.at(m).package);

    // The variables the node's location reads are what the swap replaces.
    let located: HashSet<String> = [&n.written.database, &n.written.schema, &n.written.alias]
        .into_iter()
        .flat_map(|w| envs::var_names(w))
        .collect();

    let mut out: Vec<Finding> = n
        .reads
        .iter()
        .filter(|r| !matches!(&r.what, What::EnvVar(name) if located.contains(name)))
        .map(|r| finding("", "", "", r))
        .collect();

    // A project's own `generate_*_name` that follows the target decides where
    // every model builds, which the page cannot evaluate.
    let mut placing: HashSet<u32> = HashSet::new();
    if matches!(n.kind, Kind::Model | Kind::Seed | Kind::Snapshot) && macros.has_root() {
        for name in crate::macro_use::PLACING {
            let Some(m) = macros.get(macros.root(), name).and_then(|m| macros.position(&m.id)) else { continue };
            let below = graph.macro_use.below(vec![m], |c| !internal(c));
            let target = |c: u32| macros.at(c).reads.iter().find(|r| matches!(r.what, What::Target(_)));
            // A line is the override's own only when the read is in it; one in a
            // macro it calls is named by that macro, with no line to give.
            let (line, through) = match target(m) {
                Some(r) => (r.line, String::new()),
                None => match below.iter().find(|&&c| c != m && target(c).is_some()) {
                    Some(&c) => (0, macros.at(c).name.clone()),
                    None => continue,
                },
            };
            let mac = macros.at(m);
            placing.insert(m);
            out.push(Finding {
                macro_id: mac.id.clone(),
                macro_name: mac.name.clone(),
                package: String::new(),
                file: String::new(),
                kind: "placing",
                detail: String::new(),
                through,
                line,
            });
        }
    }

    let start = graph.macro_use.called_by(idx);
    for m in graph.macro_use.below(start, |c| !internal(c)) {
        let mac = macros.at(m);
        let package = if mac.package == macros.root() { "" } else { mac.package.as_str() };
        for r in mac.reads.iter() {
            if placing.contains(&m) && matches!(r.what, What::Target(_)) {
                continue;
            }
            out.push(finding(&mac.id, &mac.name, package, r));
        }
    }

    out.sort_by(|a, b| {
        (!a.macro_id.is_empty(), &a.macro_name, &a.macro_id, a.kind, &a.detail, a.line)
            .cmp(&(!b.macro_id.is_empty(), &b.macro_name, &b.macro_id, b.kind, &b.detail, b.line))
    });
    out.dedup_by(|b, a| (&a.macro_id, a.kind, &a.detail) == (&b.macro_id, b.kind, &b.detail));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::RawManifest;
    use std::path::Path;

    fn whats(sql: &str) -> Vec<(What, u32)> {
        scan(sql).into_iter().map(|r| (r.what, r.line)).collect()
    }

    fn target(k: &str) -> What {
        What::Target(k.into())
    }

    #[test]
    fn a_branch_on_the_target_is_a_read_at_its_line() {
        let sql = "select 1\n{% if target.name == 'prod' %}\n  from a\n{% else %}\n  from {{ target['schema'] }}.b\n{% endif %}";
        assert_eq!(whats(sql), vec![(target("name"), 2), (target("schema"), 5)]);
    }

    #[test]
    fn what_is_the_same_everywhere_or_only_text_is_not_a_read() {
        let sql = "{{ target.type }} {{ target.threads }}\n{# target.name #}\n{% raw %}{{ target.name }}{% endraw %}\n-- target.name\n{{ 'target.name' }}\nselect {{ ref('x') }}";
        assert_eq!(whats(sql), vec![]);
    }

    #[test]
    fn target_get_reads_the_key_it_names() {
        assert_eq!(whats("{{ target.get('role', 'x') }}"), vec![(target("role"), 1)]);
    }

    #[test]
    fn an_env_var_is_named_unless_it_reads_as_a_credential() {
        let sql = "{{ env_var('DBT_REGION') }}\n{{ env_var('DBT_ENV_SECRET_PW') }}\n{{ env_var(\"API_KEY\", 'x') }}";
        assert_eq!(whats(sql), vec![(What::EnvVar("DBT_REGION".into()), 1), (What::EnvVar(String::new()), 2)]);
    }

    #[test]
    fn a_call_that_asks_the_warehouse_or_the_table_is_a_read() {
        let sql = "{% set rows = run_query('select 1') %}\n{{ adapter.get_columns_in_relation(this) }}\n{{ dbt.load_relation(x) }}\n{% if is_incremental() %}{% endif %}\n{{ load_result('a') }}";
        assert_eq!(
            whats(sql),
            vec![
                (What::Warehouse("run_query".into()), 1),
                (What::Warehouse("adapter.get_columns_in_relation".into()), 2),
                (What::Warehouse("dbt.load_relation".into()), 3),
                (What::Incremental, 4),
                (What::Warehouse("load_result".into()), 5),
            ]
        );
    }

    #[test]
    fn a_read_is_listed_once_at_its_first_line() {
        assert_eq!(whats("{{ target.name }}\n{{ target.name }}"), vec![(target("name"), 1)]);
    }

    fn graph(macros: serde_json::Value, nodes: serde_json::Value) -> Graph {
        let raw: RawManifest = serde_json::from_value(serde_json::json!({
            "metadata": { "project_name": "shop", "adapter_type": "snowflake" },
            "nodes": nodes,
            "macros": macros,
        }))
        .unwrap();
        Graph::build(raw, Path::new("target/manifest.json"), 0, 0)
    }

    fn mac(name: &str, package: &str, sql: &str, calls: &[&str]) -> serde_json::Value {
        serde_json::json!({ "name": name, "package_name": package, "macro_sql": sql, "depends_on": { "macros": calls } })
    }

    fn model(name: &str, code: &str, calls: &[&str]) -> serde_json::Value {
        serde_json::json!({
            "name": name, "resource_type": "model", "package_name": "shop", "raw_code": code,
            "original_file_path": format!("models/{name}.sql"),
            "unrendered_config": { "schema": "{{ env_var('DBT_SCHEMA') }}" },
            "depends_on": { "nodes": [], "macros": calls },
        })
    }

    fn found(g: &Graph, name: &str) -> Vec<(String, &'static str, String, u32)> {
        let idx = g.nodes.iter().position(|n| n.name == name).unwrap() as u32;
        for_node(g, idx).into_iter().map(|f| (f.macro_name, f.kind, f.detail, f.line)).collect()
    }

    fn row(name: &str, kind: &'static str, detail: &str, line: u32) -> (String, &'static str, String, u32) {
        (name.into(), kind, detail.into(), line)
    }

    #[test]
    fn a_macro_a_model_only_reaches_through_another_is_named_with_its_line() {
        let g = graph(
            serde_json::json!({
                "macro.shop.stamp": mac("stamp", "shop", "{% macro stamp() %}\n  {{ helper() }}\n{% endmacro %}", &["macro.shop.helper"]),
                "macro.shop.helper": mac("helper", "shop", "{% macro helper() %}\n\n  {{ target.name }}\n{% endmacro %}", &[]),
                "macro.shop.unused": mac("unused", "shop", "{{ target.role }}", &[]),
            }),
            serde_json::json!({ "model.shop.orders": model("orders", "select {{ stamp() }}", &["macro.shop.stamp"]) }),
        );
        assert_eq!(found(&g, "orders"), vec![row("helper", "target", "name", 3)]);
    }

    #[test]
    fn the_models_own_sql_comes_first_and_a_variable_its_location_reads_is_left_out() {
        let g = graph(
            serde_json::json!({ "macro.shop.a": mac("a", "shop", "{{ env_var('DBT_OTHER') }}", &[]) }),
            serde_json::json!({ "model.shop.orders": model(
                "orders",
                "{{ config(schema=env_var('DBT_SCHEMA')) }}\n{{ env_var('DBT_REGION') }} {{ a() }}",
                &["macro.shop.a"],
            ) }),
        );
        assert_eq!(found(&g, "orders"), vec![row("", "env_var", "DBT_REGION", 2), row("a", "env_var", "DBT_OTHER", 1)]);
    }

    #[test]
    fn dbts_own_macros_are_skipped_but_is_incremental_is_named() {
        let g = graph(
            serde_json::json!({
                "macro.dbt.run_query": mac("run_query", "dbt", "{{ target.name }}", &[]),
                "macro.dbt_snowflake.x": mac("x", "dbt_snowflake", "{{ target.name }}", &[]),
                "macro.dbt.is_incremental": mac("is_incremental", "dbt", "{{ run_query('x') }}", &[]),
            }),
            serde_json::json!({ "model.shop.orders": model(
                "orders",
                "select 1 {% if is_incremental() %} where 1 {% endif %}",
                &["macro.dbt.run_query", "macro.dbt_snowflake.x", "macro.dbt.is_incremental"],
            ) }),
        );
        assert_eq!(found(&g, "orders"), vec![row("", "incremental", "", 1)]);
    }

    #[test]
    fn a_project_override_of_a_placing_macro_that_reads_the_target_is_one_finding_on_every_model() {
        let g = graph(
            serde_json::json!({
                "macro.shop.generate_schema_name": mac(
                    "generate_schema_name", "shop",
                    "{% macro generate_schema_name(c, n) %}\n{{ schema_part() }}\n{% endmacro %}", &["macro.shop.schema_part"],
                ),
                "macro.shop.schema_part": mac("schema_part", "shop", "{% if target.name == 'ci' %}{% endif %}", &[]),
                "macro.shop.generate_alias_name": mac("generate_alias_name", "shop", "{{ node.name }}", &[]),
            }),
            serde_json::json!({
                "model.shop.orders": model("orders", "select 1", &[]),
                "model.shop.items": model("items", "select 2", &[]),
            }),
        );
        // Reported against the override, named through the macro it calls, with
        // no line: the line would be in another text.
        let at = |g: &Graph| {
            let idx = g.nodes.iter().position(|n| n.name == "orders").unwrap() as u32;
            for_node(g, idx).into_iter().map(|f| (f.macro_name, f.through, f.line)).collect::<Vec<_>>()
        };
        assert_eq!(at(&g), vec![("generate_schema_name".to_string(), "schema_part".to_string(), 0)]);
        assert_eq!(found(&g, "items"), found(&g, "orders"));
    }

    #[test]
    fn an_override_reading_the_target_itself_gives_its_own_line() {
        let g = graph(
            serde_json::json!({
                "macro.shop.generate_schema_name": mac(
                    "generate_schema_name", "shop",
                    "{% macro generate_schema_name(c, n) %}\n\n  {{ target.schema }}\n{% endmacro %}", &["macro.shop.deep"],
                ),
                "macro.shop.deep": mac("deep", "shop", "{{ target.name }}", &[]),
            }),
            serde_json::json!({ "model.shop.orders": model("orders", "select 1", &[]) }),
        );
        assert_eq!(found(&g, "orders"), vec![row("generate_schema_name", "placing", "", 3)]);
    }

    #[test]
    fn blank_lines_a_file_opens_with_are_counted() {
        assert_eq!(leading_lines("select 1"), 0);
        assert_eq!(leading_lines("\r\n\n  \nselect 1\n\n"), 3);
        assert_eq!(leading_lines("  select 1"), 0);
        let mut list = vec![
            Finding { macro_id: String::new(), macro_name: String::new(), package: String::new(), file: String::new(),
                kind: "target", detail: "name".into(), through: String::new(), line: 2 },
            Finding { macro_id: "macro.shop.a".into(), macro_name: "a".into(), package: String::new(), file: String::new(),
                kind: "target", detail: "name".into(), through: String::new(), line: 2 },
        ];
        shift_own(&mut list, 3);
        assert_eq!((list[0].line, list[1].line), (5, 2), "a macro's text is not the file's");
    }

    #[test]
    fn the_findings_come_in_name_order_and_a_macro_reached_twice_is_listed_once() {
        let g = graph(
            serde_json::json!({
                "macro.shop.zeta": mac("zeta", "shop", "{{ target.name }}", &["macro.shop.alpha"]),
                "macro.shop.alpha": mac("alpha", "shop", "{{ target.name }}", &[]),
            }),
            serde_json::json!({ "model.shop.orders": model("orders", "{{ alpha() }}{{ zeta() }}", &["macro.shop.zeta", "macro.shop.alpha"]) }),
        );
        assert_eq!(found(&g, "orders"), vec![row("alpha", "target", "name", 1), row("zeta", "target", "name", 1)]);
    }
}
