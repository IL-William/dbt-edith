//! collin's report read whole, for the report page: every finding it holds,
//! filed by kind, so the page can group them by what would remove them.
//!
//! Built from the lists collin has written since its 0.1.0, never from a
//! section a later collin would have to add: a report from any collin a user
//! still has installed reads the same way. Nothing here words a finding. The
//! page does, as it words a column's notes, and the kinds are the contract
//! between the two.
//!
//! The report is collin's file, not a format this binary owns, so every field
//! is read as optional and a list reshaped by a later collin costs that kind
//! alone.

use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};

/// The kinds, in the order the page lists them. One model can be in several.
pub const KINDS: [&str; 19] = [
    "no_compiled_sql",
    "compiled_elsewhere",
    "catalog_elsewhere",
    "columns_missing",
    "columns_unexpected",
    "yaml_stale",
    "thin",
    "undeclared_relation",
    "unknown_parent",
    "unbacked_read",
    "merged_scope",
    "relation_collision",
    "short_upstream",
    "phantom",
    "parse_failed",
    "ambiguous",
    "engine_issue",
    "lost_in_expression",
    "lost_unresolved",
];

/// A model with two hundred missing columns says so in a count, not a list.
const ITEMS: usize = 20;

#[derive(serde::Serialize, Clone, Debug, Default, PartialEq)]
pub struct Overview {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub generated_at: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub project: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub dbt_version: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub adapter: String,
    /// collin's counters as it wrote them, numbers only. Passed through rather
    /// than picked, so a counter a later collin adds reaches the page without
    /// a change here.
    pub totals: Map<String, Value>,
    /// Only the kinds with an entry, in `KINDS` order.
    pub findings: Vec<Finding>,
    /// Models and relations the findings name, each once however many kinds
    /// it is in: the number the badge shows.
    pub to_investigate: usize,
}

#[derive(serde::Serialize, Clone, Debug, PartialEq)]
pub struct Finding {
    pub kind: &'static str,
    /// Sorted by id, so two reads of one report list it alike (0055).
    pub entries: Vec<Entry>,
}

#[derive(serde::Serialize, Clone, Debug, Default, PartialEq)]
pub struct Entry {
    /// A dbt unique_id, or for a relation collision the relation.
    pub id: String,
    /// What the kind carries besides a list: a parse error, a compiled file,
    /// the table a catalog entry describes, the CTEs a column was read from.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub detail: String,
    /// Columns, relations, names or issue codes, at most `ITEMS`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<String>,
    /// How many `items` there were before the cut, when there were more.
    #[serde(skip_serializing_if = "is_zero")]
    pub of: usize,
}

fn is_zero(n: &usize) -> bool {
    *n == 0
}

fn text(v: &Value, key: &str) -> String {
    v.get(key).and_then(Value::as_str).unwrap_or_default().to_string()
}

fn list<'a>(v: &'a Value, key: &str) -> &'a [Value] {
    v.get(key).and_then(Value::as_array).map(Vec::as_slice).unwrap_or_default()
}

fn texts(v: &Value, key: &str) -> Vec<String> {
    list(v, key).iter().filter_map(Value::as_str).map(str::to_string).collect()
}

/// Each name once, in the order first met.
fn once(names: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    names.into_iter().filter(|n| !n.is_empty() && seen.insert(n.clone())).collect()
}

fn entry(id: String, detail: String, mut items: Vec<String>) -> Entry {
    let of = if items.len() > ITEMS { items.len() } else { 0 };
    items.truncate(ITEMS);
    Entry { id, detail, items, of }
}

pub fn overview(report: &Value) -> Overview {
    let mut by: BTreeMap<&'static str, Vec<Entry>> = BTreeMap::new();
    let mut add = |kind: &'static str, e: Entry| {
        if !e.id.is_empty() {
            by.entry(kind).or_default().push(e);
        }
    };

    for c in list(report, "catalog_elsewhere") {
        add("catalog_elsewhere", entry(text(c, "unique_id"), text(c, "catalog_relation"), Vec::new()));
    }
    for c in list(report, "relation_collisions") {
        add("relation_collision", entry(text(c, "relation"), String::new(), texts(c, "nodes")));
    }
    for m in list(report, "models") {
        let id = text(m, "unique_id");
        for (kind, detail, items) in model_findings(m) {
            add(kind, entry(id.clone(), detail, items));
        }
    }

    let findings: Vec<Finding> = KINDS
        .iter()
        .filter_map(|kind| {
            let mut entries = by.remove(kind)?;
            entries.sort_by(|a, b| a.id.cmp(&b.id).then_with(|| a.detail.cmp(&b.detail)));
            Some(Finding { kind, entries })
        })
        .collect();
    let to_investigate = findings.iter().flat_map(|f| f.entries.iter().map(|e| e.id.as_str())).collect::<BTreeSet<_>>().len();
    let totals = report
        .get("totals")
        .and_then(Value::as_object)
        .map(|t| t.iter().filter(|(_, v)| v.is_number()).map(|(k, v)| (k.clone(), v.clone())).collect())
        .unwrap_or_default();
    Overview {
        generated_at: text(report, "generated_at"),
        project: text(report, "project"),
        dbt_version: text(report, "dbt_version"),
        adapter: text(report, "adapter"),
        totals,
        findings,
        to_investigate,
    }
}

/// What one entry of `models` holds, kind by kind, each with its detail and
/// its list.
fn model_findings(m: &Value) -> Vec<(&'static str, String, Vec<String>)> {
    let mut out: Vec<(&'static str, String, Vec<String>)> = Vec::new();
    let parse_error = text(m, "parse_error");
    // dbt wrote no SQL for the model: a compile is missing, which is not the
    // parser's failure. collin words every such case this way.
    if parse_error.starts_with("no compiled_code") {
        out.push(("no_compiled_sql", parse_error.clone(), Vec::new()));
    } else if !parse_error.is_empty() {
        out.push(("parse_failed", parse_error.clone(), Vec::new()));
    }
    if m.get("sql_file_set_aside").and_then(Value::as_bool) == Some(true) {
        out.push(("compiled_elsewhere", text(m, "sql_file"), Vec::new()));
    }
    // A compile that did not parse produced no column, so every column of the
    // table reads as missing from it, which says nothing about the table.
    if parse_error.is_empty() {
        for (key, kind) in [("missing_columns", "columns_missing"), ("unexpected_columns", "columns_unexpected")] {
            let columns = texts(m, key);
            if !columns.is_empty() {
                out.push((kind, String::new(), columns));
            }
        }
    }
    if m.get("yaml_stale").and_then(Value::as_bool) == Some(true) {
        out.push(("yaml_stale", String::new(), Vec::new()));
    }
    if let Some(thin) = m.get("thin").filter(|t| t.is_object()) {
        let n = |k: &str| thin.get(k).and_then(Value::as_u64).unwrap_or(0);
        out.push(("thin", format!("{} of {}", n("computed"), n("declared")), Vec::new()));
    }
    for (key, kind) in [("undeclared_relations", "undeclared_relation"), ("unknown_relations", "unknown_parent")] {
        let relations = texts(m, key);
        if !relations.is_empty() {
            out.push((kind, String::new(), relations));
        }
    }
    let unbacked = list(m, "unbacked");
    if !unbacked.is_empty() {
        let ctes = once(unbacked.iter().map(|u| text(u, "cte")));
        out.push(("unbacked_read", ctes.join(", "), once(unbacked.iter().map(|u| text(u, "column")))));
    }
    // Only where it set the compile aside, as collin before 0.4.0 did with two
    // CTEs of one name. A later collin reads the two apart and only names
    // them, on a model it still calls parsed, where there is nothing to fix.
    let merged = list(m, "merged_scopes");
    let set_aside = matches!(text(m, "provenance").as_str(), "inferred" | "unresolved");
    if !merged.is_empty() && set_aside {
        out.push(("merged_scope", String::new(), once(merged.iter().map(|s| text(s, "name")))));
    }
    let short = list(m, "read_downstream_but_absent");
    if !short.is_empty() {
        out.push(("short_upstream", String::new(), once(short.iter().map(|r| text(r, "column")))));
    }
    // Only a name taken from every candidate is one to read by hand: one
    // settled by the compile or by manifest order was settled by something.
    let every = list(m, "ambiguous_inferred").iter().filter(|a| text(a, "by") == "every");
    let every = once(every.map(|a| text(a, "column")));
    if !every.is_empty() {
        out.push(("ambiguous", String::new(), every));
    }
    // A lost column is filed under each place a walk from it stopped, since
    // two walks from one column can stop for two reasons.
    let mut lost: BTreeMap<&'static str, (Vec<String>, Vec<String>)> = BTreeMap::new();
    for c in list(m, "lost_columns") {
        let column = text(c, "column");
        let ends = list(c, "ends");
        if ends.is_empty() {
            lost.entry("lost_unresolved").or_default().0.push(column.clone());
        }
        for end in ends {
            let (kind, via) = match end.get("reason").and_then(Value::as_str) {
                Some("phantom") => ("phantom", text(end, "via")),
                Some("names_unreached") => ("lost_in_expression", String::new()),
                _ => ("lost_unresolved", String::new()),
            };
            let slot = lost.entry(kind).or_default();
            slot.0.push(column.clone());
            slot.1.push(via);
        }
    }
    for (kind, (columns, vias)) in lost {
        out.push((kind, once(vias).join(", "), once(columns)));
    }
    // The engine's issues, when nothing above already explains the entry. A
    // model reading its own table is not one (collin's 0012).
    let codes = once(
        list(m, "issues")
            .iter()
            .filter(|i| i.get("self_reference").and_then(Value::as_bool) != Some(true))
            .map(|i| text(i, "code")),
    );
    if out.is_empty() && !codes.is_empty() {
        out.push(("engine_issue", String::new(), codes));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn kinds(o: &Overview) -> Vec<&'static str> {
        o.findings.iter().map(|f| f.kind).collect()
    }

    fn of<'a>(o: &'a Overview, kind: &str) -> &'a [Entry] {
        o.findings.iter().find(|f| f.kind == kind).map(|f| f.entries.as_slice()).unwrap_or_default()
    }

    #[test]
    fn a_report_with_nothing_to_say_has_no_finding() {
        let o = overview(&json!({ "totals": { "models": 3, "parsed": 3 }, "models": [] }));
        assert!(o.findings.is_empty());
        assert_eq!(o.to_investigate, 0);
        assert_eq!(o.totals["models"], 3);
        assert_eq!(overview(&json!({})), Overview::default(), "a report reshaped beyond reading is an empty one");
    }

    #[test]
    fn a_missing_compile_is_told_apart_from_sql_that_did_not_parse() {
        let o = overview(&json!({ "models": [
            { "unique_id": "model.p.a", "parse_error": "no compiled_code and no compiled file",
              "missing_columns": ["id", "name"] },
            { "unique_id": "model.p.b", "parse_error": "sql parser error: Expected ), found: EOF" },
            { "unique_id": "model.p.c", "parse_error": "no compiled_code, and its compiled file is empty" },
        ] }));
        assert_eq!(kinds(&o), ["no_compiled_sql", "parse_failed"]);
        assert_eq!(of(&o, "no_compiled_sql").iter().map(|e| e.id.as_str()).collect::<Vec<_>>(), ["model.p.a", "model.p.c"]);
        assert_eq!(of(&o, "parse_failed")[0].detail, "sql parser error: Expected ), found: EOF");
        // A compile that never parsed produced no column, so its table's
        // columns are not missing from anything.
        assert!(of(&o, "columns_missing").is_empty());
    }

    #[test]
    fn every_list_collin_writes_on_a_model_is_filed_under_its_kind() {
        let o = overview(&json!({
            "catalog_elsewhere": [{ "unique_id": "model.p.a", "relation": "DB.S.A", "catalog_relation": "DB.DEV.A" }],
            "relation_collisions": [{ "relation": "DB.S.X", "nodes": ["model.p.x", "seed.p.x"] }],
            "models": [{
                "unique_id": "model.p.a", "sql_file": "compiled/p/models/a.sql", "sql_file_set_aside": true,
                "missing_columns": ["old"], "unexpected_columns": ["new"], "yaml_stale": true,
                "thin": { "declared": 40, "computed": 4, "shared": 4 },
                "undeclared_relations": ["RAW.DBO.ORDERS"], "unknown_relations": ["DB.S.P"],
                "unbacked": [{ "cte": "base", "column": "k", "owners": [] }, { "cte": "base", "column": "k", "owners": [] }],
                "provenance": "inferred", "merged_scopes": [{ "kind": "cte", "name": "ranked", "count": 2 }],
                "read_downstream_but_absent": [{ "column": "c", "against": "warehouse", "in_compile": true, "read_by": ["model.p.z"] }],
                "ambiguous_inferred": [
                    { "column": "u", "candidates": [], "chosen": [], "by": "every" },
                    { "column": "v", "candidates": [], "chosen": [], "by": "order" }
                ],
                "lost_columns": [
                    { "column": "f", "ends": [{ "reason": "phantom", "via": "joined" }, { "reason": "names_unreached", "names": ["x"] }] },
                    { "column": "g", "ends": [] }
                ],
                "issues": [{ "code": "UNRESOLVED_REFERENCE", "severity": "warning" }]
            }]
        }));
        assert_eq!(
            kinds(&o),
            [
                "compiled_elsewhere", "catalog_elsewhere", "columns_missing", "columns_unexpected", "yaml_stale", "thin",
                "undeclared_relation", "unknown_parent", "unbacked_read", "merged_scope", "relation_collision",
                "short_upstream", "phantom", "ambiguous", "lost_in_expression", "lost_unresolved",
            ]
        );
        assert_eq!(of(&o, "catalog_elsewhere")[0].detail, "DB.DEV.A");
        assert_eq!(of(&o, "relation_collision")[0].items, ["model.p.x", "seed.p.x"]);
        assert_eq!(of(&o, "thin")[0].detail, "4 of 40");
        assert_eq!(of(&o, "unbacked_read")[0], Entry { id: "model.p.a".into(), detail: "base".into(), items: vec!["k".into()], of: 0 });
        assert_eq!(of(&o, "ambiguous")[0].items, ["u"], "only a name taken from every candidate is one to read by hand");
        assert_eq!(of(&o, "phantom")[0].detail, "joined");
        assert_eq!(of(&o, "lost_in_expression")[0].items, ["f"], "a column whose walks stopped twice is filed twice");
        assert_eq!(of(&o, "lost_unresolved")[0].items, ["g"]);
        assert!(of(&o, "engine_issue").is_empty(), "issues on a model already explained add no entry");
        assert_eq!(o.to_investigate, 2, "the model and the contested relation, each once");
    }

    #[test]
    fn two_scopes_of_one_name_are_a_finding_only_where_they_set_the_compile_aside() {
        let scopes = json!([{ "kind": "cte", "name": "final", "count": 2 }]);
        let o = overview(&json!({ "models": [
            { "unique_id": "model.p.read_apart", "provenance": "parsed", "merged_scopes": scopes },
            { "unique_id": "model.p.set_aside", "provenance": "inferred", "merged_scopes": scopes },
        ] }));
        assert_eq!(of(&o, "merged_scope").iter().map(|e| e.id.as_str()).collect::<Vec<_>>(), ["model.p.set_aside"]);
    }

    #[test]
    fn an_issue_alone_is_a_finding_unless_the_model_reads_itself() {
        let o = overview(&json!({ "models": [
            { "unique_id": "model.p.a", "issues": [{ "code": "W1" }, { "code": "W1" }, { "code": "W2" }] },
            { "unique_id": "model.p.b", "issues": [{ "code": "UNRESOLVED_REFERENCE", "self_reference": true }] },
        ] }));
        assert_eq!(of(&o, "engine_issue"), [Entry { id: "model.p.a".into(), items: vec!["W1".into(), "W2".into()], ..Default::default() }]);
    }

    #[test]
    fn a_long_list_is_cut_and_says_how_long_it_was() {
        let columns: Vec<String> = (0..25).map(|i| format!("c{i:02}")).collect();
        let o = overview(&json!({ "models": [{ "unique_id": "model.p.a", "missing_columns": columns }] }));
        let e = &of(&o, "columns_missing")[0];
        assert_eq!((e.items.len(), e.of), (ITEMS, 25));
    }

    #[test]
    fn entries_come_out_in_one_order_whatever_order_the_report_has() {
        let a = overview(&json!({ "models": [{ "unique_id": "model.p.b", "yaml_stale": true }, { "unique_id": "model.p.a", "yaml_stale": true }] }));
        assert_eq!(of(&a, "yaml_stale").iter().map(|e| e.id.as_str()).collect::<Vec<_>>(), ["model.p.a", "model.p.b"]);
    }

    #[test]
    fn only_numbers_pass_through_from_the_totals() {
        let o = overview(&json!({ "totals": { "models": 2, "note": "x", "columns_root": 0 } }));
        assert_eq!(o.totals.keys().collect::<Vec<_>>(), ["columns_root", "models"]);
    }
}
