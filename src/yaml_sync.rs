//! What a model's YAML would change to say what its table says (0188).
//!
//! One concern: compare the columns a model's YAML declares with the columns
//! the catalog found in its table, and say, per model, what the YAML lacks,
//! what it declares that the table does not have, and which types it spells
//! differently or not at all. It never edits a file: the page applies this
//! answer to the text of the file, which the user reviews and saves (0188).
//!
//! Invariants:
//! - Every rule about a type lives here and nowhere else: the base type two
//!   spellings fold to, the spelling written, the contract trap and the case a
//!   name is written in. The page applies what it is sent and compares nothing.
//! - A model is compared only when the answer would mean something. One the
//!   catalog does not list, lists no column of, or lists for another
//!   relation than the manifest's is `not_checked` with its reason, never
//!   `disagrees`: an empty or foreign column list is no evidence of drift.
//! - Types are compared on their base alone, as a contract does: `TEXT` is
//!   `varchar(100)`, `NUMBER` is `number(38,2)`.
//! - Freshness is mtimes and never a clock (0025): the model's `run/` file
//!   against catalog.json, and against the code it was built from, by the
//!   inputs `compiled::look_up_with` already compares. A model with no run
//!   file has no flag. Only a model that `disagrees` is looked up, since the
//!   flags are what an update is warned by.
//! - The answer follows the graph's order, which is the same on every start
//!   (0055).

use crate::compiled;
use crate::envs::{self, EnvFile};
use crate::graph::{Column, Graph, Kind, Node, TableRelation};
use std::path::Path;

#[derive(serde::Serialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Agrees,
    /// Columns to add or remove, or a type that is another type.
    Disagrees,
    /// Nothing but columns that declare no `data_type`.
    Untyped,
    NotChecked,
}

/// A column of the table the YAML lacks.
#[derive(serde::Serialize, Debug, PartialEq)]
pub struct Missing {
    /// As it is to be written.
    pub name: String,
    /// What to write under `data_type`; empty when it cannot be (see `held`).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub data_type: String,
    /// The column before it in the table, by the name the YAML has or will
    /// have for it; empty at the top.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub after: String,
}

/// A declared column the table does not have, with what removing it loses.
#[derive(serde::Serialize, Debug, PartialEq)]
pub struct Extra {
    pub name: String,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub described: bool,
    #[serde(skip_serializing_if = "is_zero")]
    pub tests: usize,
}

fn is_zero(n: &usize) -> bool {
    *n == 0
}

/// A declared type whose base differs from the table's.
#[derive(serde::Serialize, Debug, PartialEq)]
pub struct Retype {
    pub name: String,
    /// As the YAML wrote it, which the page checks the text against.
    pub from: String,
    pub to: String,
}

/// A declared column that has no `data_type`.
#[derive(serde::Serialize, Debug, PartialEq)]
pub struct Untyped {
    pub name: String,
    pub to: String,
}

/// A column whose type is not written, and why.
#[derive(serde::Serialize, Debug, PartialEq)]
pub struct Held {
    pub name: String,
    pub reason: String,
}

#[derive(serde::Serialize, Debug)]
pub struct ModelSync {
    pub id: String,
    pub name: String,
    /// The properties file the manifest says declares the model, project
    /// relative; empty when it has none, and the page then proposes one.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub yml: String,
    /// The model's own file, beside which a new properties file goes.
    pub file: String,
    pub status: Status,
    /// Why a model is `not_checked`.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub reason: String,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub contract: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub missing: Vec<Missing>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub extra: Vec<Extra>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub retype: Vec<Retype>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub untyped: Vec<Untyped>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub held: Vec<Held>,
    /// Which environment catalog.json describes: `manifest` when its relation
    /// is the one the manifest names, else the `.env` file whose resolved
    /// location it is (0188). Empty for a model not checked.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub catalog_env: String,
    /// The catalog describes a deployed environment, not the manifest's target.
    /// The run-file flags below do not apply to it, since a local run builds
    /// the manifest's target.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub deployed: bool,
    /// Deployed: the model's code is newer than catalog.json, so the table
    /// there may have been built from older code.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub code_after_catalog: bool,
    /// The model's run file is newer than catalog.json: the table was rebuilt
    /// after the catalog read it.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub catalog_behind_run: bool,
    /// The run file is older than the code it was built from: the table is
    /// not what the code now says.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub table_behind_code: bool,
}

#[derive(serde::Serialize, Debug)]
pub struct Answer {
    /// When catalog.json was written; 0 when there is none.
    pub catalog_at: u64,
    /// The environment most of the models checked found the catalog to
    /// describe, `manifest` or a `.env` file's name.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub catalog_env: String,
    pub models: Vec<ModelSync>,
}

/// The project's `.env` files the environment menu shows, read once for the
/// answer and only if a model needs one. A file the menu hides, a template, a
/// backup or one the user hid, says nothing of an environment.
struct Files<'a> {
    graph: &'a Graph,
    root: &'a Path,
    chosen: &'a dyn Fn(&str) -> Option<bool>,
    found: std::cell::OnceCell<Vec<EnvFile>>,
}

impl Files<'_> {
    fn all(&self) -> &[EnvFile] {
        self.found.get_or_init(|| {
            let places: Vec<_> = self.graph.nodes.iter().filter(|n| n.kind != Kind::Test).map(|n| (&n.written, &n.parsed)).collect();
            envs::visible(self.root, &places, self.chosen)
        })
    }
}

/// Every model of the project, or the ones at `only`.
///
/// Only the models named get their freshness flags: they cost a stat or more per
/// model, which a whole-project answer would pay thousands of times, so that
/// answer carries none. `chosen` is
/// the user's choice to hide an `.env` file, or not, by its name.
pub fn check(graph: &Graph, root: &Path, target: &Path, only: Option<&[u32]>, chosen: &dyn Fn(&str) -> Option<bool>) -> Answer {
    let catalog_at = graph.catalog_mtime;
    let snowflake = graph.meta.adapter.eq_ignore_ascii_case("snowflake");
    let files = Files { graph, root, chosen, found: std::cell::OnceCell::new() };
    // The macros are walked once, and only if a model that differs has a run file.
    let macros = std::cell::OnceCell::new();
    let newest = || macros.get_or_init(|| crate::freshness::newest_input(root, crate::freshness::MACROS_DIR)).clone();
    let one = |n: &Node| {
        let mut m = compare(graph, n, snowflake, &files);
        let differs = matches!(m.status, Status::Disagrees | Status::Untyped);
        m.deployed = differs && m.catalog_env != "manifest";
        if differs && only.is_some() && catalog_at > 0 {
            if m.deployed {
                m.code_after_catalog = crate::files::resolve(root, &n.file).map(|p| mtime(&p)).unwrap_or(0) > catalog_at;
            } else {
                let subject = compiled::Subject {
                    package: &n.package,
                    file: &n.file,
                    yml: &n.yml,
                    compiled: &n.compiled,
                    name: &n.name,
                    alias: &n.alias,
                };
                // No content wanted, one byte is read.
                let run = compiled::look_up_with(root, target, compiled::Kind::Run, &subject, 0, newest);
                if run.found {
                    m.catalog_behind_run = run.compiled_at > catalog_at;
                    // The schema file changes the table's columns only through a contract.
                    m.table_behind_code = run.changed.iter().any(|c| n.contract || c != "the schema file");
                }
            }
        }
        m
    };
    let models: Vec<ModelSync> = match only {
        Some(list) => list.iter().map(|&i| one(&graph.nodes[i as usize])).collect(),
        None => graph.nodes.iter().filter(|n| n.kind == Kind::Model && !n.disabled).map(one).collect(),
    };
    let mut seen: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for m in models.iter().filter(|m| !m.catalog_env.is_empty()) {
        *seen.entry(m.catalog_env.as_str()).or_default() += 1;
    }
    // The most models, the first by name when several tie: the rule each model's own pick follows.
    let catalog_env =
        seen.iter().min_by_key(|&(name, count)| (std::cmp::Reverse(*count), *name)).map(|(name, _)| name.to_string()).unwrap_or_default();
    Answer { catalog_at, catalog_env, models }
}

fn mtime(path: &Path) -> u64 {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn compare(graph: &Graph, n: &Node, snowflake: bool, files: &Files) -> ModelSync {
    let mut out = ModelSync {
        id: n.id.clone(),
        name: n.name.clone(),
        yml: n.yml.clone(),
        file: n.file.clone(),
        status: Status::NotChecked,
        reason: String::new(),
        contract: n.contract,
        missing: Vec::new(),
        extra: Vec::new(),
        retype: Vec::new(),
        untyped: Vec::new(),
        held: Vec::new(),
        catalog_env: String::new(),
        deployed: false,
        code_after_catalog: false,
        catalog_behind_run: false,
        table_behind_code: false,
    };
    match placed(graph, n, files) {
        Err(why) => out.reason = why.into(),
        Ok(env) => {
            out.catalog_env = env;
            let d = diff(&n.columns, snowflake, n.contract);
            let real = !(d.missing.is_empty() && d.extra.is_empty() && d.retype.is_empty()) || d.held_retype;
            out.status = if real {
                Status::Disagrees
            } else if d.untyped.is_empty() && d.held.is_empty() {
                Status::Agrees
            } else {
                Status::Untyped
            };
            (out.missing, out.extra, out.retype, out.untyped, out.held) = (d.missing, d.extra, d.retype, d.untyped, d.held);
        }
    }
    out
}

/// The environment a model's catalog entry describes, or why the model cannot
/// be compared, in the words the page shows.
fn placed(graph: &Graph, n: &Node, files: &Files) -> Result<String, &'static str> {
    if n.package != graph.meta.project {
        return Err("it belongs to an installed package");
    }
    if n.version.is_some() {
        return Err("it is a versioned model, whose columns are declared per version");
    }
    if n.materialized == "ephemeral" {
        return Err("it is ephemeral: it has no table");
    }
    let Some(table) = n.table.as_deref() else { return Err("it is not in catalog.json") };
    if !n.columns.iter().any(|c| c.index > 0) {
        // A table never has none: the catalog could not read it.
        return Err("catalog.json lists no column for it");
    }
    let alias = if n.alias.is_empty() { &n.name } else { &n.alias };
    if agrees(table, &n.database, &n.schema, alias) {
        return Ok("manifest".into());
    }
    // A catalog is often written against a deployed environment, which is a
    // reference worth comparing with: the `.env` file whose resolved location
    // for the model is the one it describes. A file counts only when the place
    // really depends on it: every part resolved, the schema non-empty (the
    // database and the alias fall back to the manifest's), and one of them read
    // from the file. A model
    // whose location is literal, or written nowhere, resolves alike under every
    // file and would be credited to the first. Only the file's name leaves (0012).
    for f in files.all() {
        let r = envs::resolve_place(&n.written, &n.parsed, &f.vars);
        let st = [&r.status.database, &r.status.schema, &r.status.alias];
        let resolved = st.iter().all(|c| matches!(c.kind, envs::Status::Literal | envs::Status::Env | envs::Status::Parsed));
        let reads_it = st.iter().any(|c| c.kind == envs::Status::Env);
        let there = if r.place.alias.is_empty() { alias } else { &r.place.alias };
        // A database the config leaves out is the target's, as the alias is the model's.
        let database = if r.place.database.is_empty() { &n.database } else { &r.place.database };
        if resolved && reads_it && !r.place.schema.is_empty() && agrees(table, database, &r.place.schema, there) {
            return Ok(f.file.clone());
        }
    }
    Err("catalog.json describes another table than the manifest names for it, and no .env file places it there")
}

/// The catalog's relation against a location, each part compared the way a
/// warehouse does and only where both say it: a catalog written for another
/// target describes another schema, and its columns are no evidence.
fn agrees(table: &TableRelation, database: &str, schema: &str, alias: &str) -> bool {
    let part = |catalog: &Option<String>, ours: &str| match catalog {
        Some(c) if !ours.is_empty() && !c.is_empty() => c.eq_ignore_ascii_case(ours),
        _ => true,
    };
    part(&table.database, database) && part(&table.schema, schema) && part(&table.name, alias)
}

#[derive(Default, Debug)]
struct Diff {
    missing: Vec<Missing>,
    extra: Vec<Extra>,
    retype: Vec<Retype>,
    untyped: Vec<Untyped>,
    held: Vec<Held>,
    /// A type that differs and cannot be written: still a disagreement.
    held_retype: bool,
}

/// Why a bare `number` is not written under a contract (0188).
const TRAP: &str =
    "its precision is not in catalog.json: a bare number is number(38,0) under a contract and rounds decimals; read it from Snowflake";

/// The columns the table has, in its own order, against the ones the YAML
/// declares. `merge_catalog` placed each (`index`) and kept what the YAML wrote
/// (`typed`), so nothing is matched here.
fn diff(columns: &[Column], snowflake: bool, contract: bool) -> Diff {
    let mut table: Vec<&Column> = columns.iter().filter(|c| c.index > 0).collect();
    table.sort_by(|a, b| a.index.cmp(&b.index).then_with(|| a.name.cmp(&b.name)));
    let mut out = Diff::default();
    let mut before = String::new();
    for c in table {
        let ty = c.table_type.as_deref().unwrap_or(&c.data_type);
        let (to, held) = to_write(snowflake, contract, ty);
        if c.undeclared {
            let name = written_name(snowflake, &c.name);
            if let Some(reason) = held {
                out.held.push(Held { name: name.clone(), reason: reason.into() });
            }
            out.missing.push(Missing { name: name.clone(), data_type: to, after: std::mem::take(&mut before) });
            before = name;
            continue;
        }
        let declared = if c.typed { c.data_type.as_str() } else { "" };
        if declared.trim().is_empty() {
            match held {
                Some(reason) => out.held.push(Held { name: c.name.clone(), reason: reason.into() }),
                None => out.untyped.push(Untyped { name: c.name.clone(), to }),
            }
        } else if differ(snowflake, declared, ty) {
            match held {
                Some(reason) => {
                    out.held.push(Held { name: c.name.clone(), reason: reason.into() });
                    out.held_retype = true;
                }
                None => out.retype.push(Retype { name: c.name.clone(), from: declared.to_string(), to }),
            }
        }
        before = c.name.clone();
    }
    for c in columns.iter().filter(|c| !c.undeclared && c.index == 0) {
        out.extra.push(Extra { name: c.name.clone(), described: !c.description.is_empty(), tests: c.tests.len() });
    }
    out
}

/// What goes under `data_type` for a table column's type, and why nothing
/// does when it cannot.
fn to_write(snowflake: bool, contract: bool, ty: &str) -> (String, Option<&'static str>) {
    if snowflake && contract && base(true, ty) == "fixed" && !ty.contains('(') {
        return (String::new(), Some(TRAP));
    }
    (written(snowflake, ty), None)
}

/// A type reduced to what a contract compares: lower-cased, its parameters
/// cut, and on Snowflake its aliases folded to one name each. Other adapters'
/// aliases are not folded: what they are is theirs to say.
fn base(snowflake: bool, ty: &str) -> String {
    let lower = ty.trim().to_lowercase();
    let head = lower.split('(').next().unwrap_or("").split_whitespace().collect::<Vec<_>>().join(" ");
    if !snowflake {
        return head;
    }
    match head.as_str() {
        "varchar" | "text" | "string" | "char" | "character" | "nchar" | "nvarchar" | "nvarchar2" | "varchar2" | "character varying"
        | "char varying" | "nchar varying" => "text",
        "number" | "numeric" | "decimal" | "int" | "integer" | "bigint" | "smallint" | "tinyint" | "byteint" => "fixed",
        "float" | "float4" | "float8" | "double" | "double precision" | "real" => "real",
        "datetime" | "timestamp_ntz" => "timestamp_ntz",
        "binary" | "varbinary" => "binary",
        other => other,
    }
    .to_string()
}

/// Whether the YAML's type and the table's are two types. A bare `timestamp`
/// is never one against a `timestamp_*`: which it means depends on the
/// account's TIMESTAMP_TYPE_MAPPING, which nothing here reads.
fn differ(snowflake: bool, yaml: &str, table: &str) -> bool {
    let (y, t) = (base(snowflake, yaml), base(snowflake, table));
    if y == t {
        return false;
    }
    !(snowflake && y == "timestamp" && t.starts_with("timestamp"))
}

/// The spelling a table's type is written in. Snowflake's catalog gives no
/// size, so a number has no precision to write unless the type carries one
/// (0188).
fn written(snowflake: bool, ty: &str) -> String {
    let lower = ty.trim().to_lowercase();
    if !snowflake {
        return lower;
    }
    match base(true, ty).as_str() {
        "text" => "varchar".into(),
        "fixed" if lower.contains('(') => lower.split_whitespace().collect::<Vec<_>>().join("").replacen("numeric", "number", 1),
        "fixed" => "number".into(),
        "real" => "float".into(),
        "timestamp_ntz" => "timestamp_ntz".into(),
        _ => lower,
    }
}

/// Snowflake keeps a name upper-case unless it was quoted, and a YAML writes
/// the unquoted one lower-case. A name that is anything else, a quoted lower
/// case one or one with a space, is written as it is.
fn written_name(snowflake: bool, name: &str) -> String {
    let plain = !name.is_empty()
        && !name.starts_with(|c: char| c.is_ascii_digit() || c == '$')
        && name.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_' || c == '$');
    if snowflake && plain {
        name.to_lowercase()
    } else {
        name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{RawCatalog, RawManifest};
    use serde_json::{json, Value};
    use std::sync::atomic::{AtomicU32, Ordering};

    fn model(name: &str, columns: Value, extra: Value) -> Value {
        let mut m = json!({
            "name": name, "resource_type": "model", "package_name": "shop",
            "original_file_path": format!("models/{name}.sql"), "fqn": ["shop", name],
            "patch_path": "shop://models/schema.yml",
            "database": "ANALYTICS", "schema": "MARTS", "alias": name,
            "config": { "materialized": "table" }, "columns": columns,
        });
        m.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        m
    }

    fn col(name: &str, ty: &str) -> Value {
        let mut c = json!({ "name": name });
        if !ty.is_empty() {
            c["data_type"] = json!(ty);
        }
        c
    }

    fn cols(list: &[(&str, &str)]) -> Value {
        Value::Object(list.iter().map(|(n, t)| (n.to_string(), col(n, t))).collect())
    }

    /// A catalog entry: columns in the order given, `index` 1-based.
    fn table(id: &str, relation: [&str; 3], list: &[(&str, &str)]) -> Value {
        let columns: serde_json::Map<String, Value> =
            list.iter().enumerate().map(|(i, (n, t))| (n.to_string(), json!({ "name": n, "type": t, "index": i + 1 }))).collect();
        json!({ id: { "metadata": { "database": relation[0], "schema": relation[1], "name": relation[2] }, "columns": columns } })
    }

    fn build(adapter: &str, nodes: Value, catalog: Option<Value>) -> Graph {
        let manifest = json!({ "metadata": { "project_name": "shop", "adapter_type": adapter }, "nodes": nodes });
        let raw: RawManifest = serde_json::from_value(manifest).unwrap();
        let mut g = Graph::build(raw, Path::new("manifest.json"), 1, 0);
        g.meta.adapter = adapter.into();
        if let Some(nodes) = catalog {
            let cat: RawCatalog = serde_json::from_value(json!({ "nodes": nodes })).unwrap();
            g.merge_catalog(cat, 1_000);
        }
        g
    }

    fn one(g: &Graph) -> ModelSync {
        let mut answer = check(g, Path::new("/nonexistent"), Path::new("/nonexistent/target"), None, &|_| None);
        assert_eq!(answer.models.len(), 1);
        answer.models.remove(0)
    }

    const ID: &str = "model.shop.orders";

    fn snow(declared: &[(&str, &str)], warehouse: &[(&str, &str)], extra: Value) -> ModelSync {
        let nodes = json!({ ID: model("orders", cols(declared), extra) });
        one(&build("snowflake", nodes, Some(table(ID, ["ANALYTICS", "MARTS", "ORDERS"], warehouse))))
    }

    #[test]
    fn a_yaml_that_says_what_the_table_says_agrees_whatever_the_case_and_the_size() {
        let m = snow(&[("order_id", "number(38,0)"), ("Status", "varchar(10)")], &[("ORDER_ID", "NUMBER"), ("STATUS", "TEXT")], json!({}));
        assert_eq!(m.status, Status::Agrees, "{m:?}");
    }

    #[test]
    fn the_columns_the_yaml_lacks_come_in_the_tables_order_each_after_the_one_before_it() {
        let m = snow(
            &[("id", "number"), ("amount", "float")],
            &[("ID", "NUMBER"), ("CUSTOMER", "TEXT"), ("PLACED_AT", "TIMESTAMP_NTZ"), ("AMOUNT", "FLOAT")],
            json!({}),
        );
        assert_eq!(m.status, Status::Disagrees);
        assert_eq!(
            m.missing,
            vec![
                Missing { name: "customer".into(), data_type: "varchar".into(), after: "id".into() },
                Missing { name: "placed_at".into(), data_type: "timestamp_ntz".into(), after: "customer".into() },
            ]
        );
    }

    #[test]
    fn a_column_at_the_top_of_the_table_has_nothing_before_it() {
        let m = snow(&[("id", "number")], &[("CREATED", "DATE"), ("ID", "NUMBER")], json!({}));
        assert_eq!(m.missing, vec![Missing { name: "created".into(), data_type: "date".into(), after: String::new() }]);
    }

    #[test]
    fn the_index_orders_the_table_and_not_the_name() {
        let catalog = json!({ ID: { "metadata": { "database": "ANALYTICS", "schema": "MARTS", "name": "ORDERS" }, "columns": {
            "A_LAST": { "name": "A_LAST", "type": "TEXT", "index": 3 },
            "Z_FIRST": { "name": "Z_FIRST", "type": "TEXT", "index": 1 },
            "M_MIDDLE": { "name": "M_MIDDLE", "type": "TEXT", "index": 2 },
        }}});
        let nodes = json!({ ID: model("orders", json!({}), json!({})) });
        let m = one(&build("snowflake", nodes, Some(catalog)));
        let names: Vec<&str> = m.missing.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["z_first", "m_middle", "a_last"]);
    }

    #[test]
    fn a_declared_column_the_table_lacks_says_what_removing_it_loses() {
        let nodes = json!({ ID: model("orders", json!({
            "id": { "name": "id", "data_type": "number" },
            "legacy": { "name": "legacy", "data_type": "text", "description": "old" },
        }), json!({})) });
        let m = one(&build("snowflake", nodes, Some(table(ID, ["ANALYTICS", "MARTS", "ORDERS"], &[("ID", "NUMBER")]))));
        assert_eq!(m.extra, vec![Extra { name: "legacy".into(), described: true, tests: 0 }]);
        assert_eq!(m.status, Status::Disagrees);

        // What guards the column is counted, and a column with neither is said to lose neither.
        let mut nodes = json!({ ID: model("orders", cols(&[("id", "number"), ("legacy", "text")]), json!({})) });
        nodes["test.shop.not_null_orders_legacy"] = json!({
            "name": "not_null_orders_legacy", "resource_type": "test", "package_name": "shop",
            "original_file_path": "models/schema.yml", "fqn": ["shop", "not_null_orders_legacy"],
            "attached_node": ID, "column_name": "legacy", "test_metadata": { "name": "not_null" },
            "depends_on": { "nodes": [ID] },
        });
        let g = build("snowflake", nodes, Some(table(ID, ["ANALYTICS", "MARTS", "ORDERS"], &[("ID", "NUMBER")])));
        let m = check(&g, Path::new("/nonexistent"), Path::new("/nonexistent/target"), None, &|_| None).models.remove(0);
        assert_eq!(m.extra, vec![Extra { name: "legacy".into(), described: false, tests: 1 }]);
    }

    #[test]
    fn a_base_that_differs_is_retyped_and_an_alias_of_it_is_not() {
        let m = snow(
            &[("a", "varchar(10)"), ("b", "integer"), ("c", "number(10,2)"), ("d", "double"), ("e", "datetime")],
            &[("A", "NUMBER"), ("B", "NUMBER"), ("C", "TEXT"), ("D", "FLOAT"), ("E", "TIMESTAMP_NTZ")],
            json!({}),
        );
        let names: Vec<(&str, &str, &str)> = m.retype.iter().map(|r| (r.name.as_str(), r.from.as_str(), r.to.as_str())).collect();
        assert_eq!(names, [("a", "varchar(10)", "number"), ("c", "number(10,2)", "varchar")]);
    }

    #[test]
    fn a_bare_timestamp_is_never_a_mismatch_against_a_timestamp() {
        let m = snow(&[("a", "timestamp"), ("b", "timestamp")], &[("A", "TIMESTAMP_LTZ"), ("B", "TIMESTAMP_TZ")], json!({}));
        assert_eq!(m.status, Status::Agrees, "{m:?}");
        let m = snow(&[("a", "timestamp")], &[("A", "DATE")], json!({}));
        assert_eq!(m.retype.len(), 1);
    }

    #[test]
    fn a_declared_column_with_no_type_is_typed() {
        let m = snow(&[("id", ""), ("name", "")], &[("ID", "NUMBER"), ("NAME", "TEXT")], json!({}));
        // Types alone are a status of their own, apart from columns to add or remove.
        assert_eq!(m.status, Status::Untyped);
        let m = snow(&[("id", "")], &[("ID", "NUMBER"), ("NAME", "TEXT")], json!({}));
        assert_eq!(m.status, Status::Disagrees);
        let m = snow(&[("id", ""), ("name", "")], &[("ID", "NUMBER"), ("NAME", "TEXT")], json!({}));
        assert_eq!(
            m.untyped,
            vec![Untyped { name: "id".into(), to: "number".into() }, Untyped { name: "name".into(), to: "varchar".into() }]
        );
    }

    #[test]
    fn the_spelling_written_is_one_per_base_on_snowflake() {
        for (catalog, written_as) in [
            ("TEXT", "varchar"),
            ("NUMBER", "number"),
            ("FLOAT", "float"),
            ("BOOLEAN", "boolean"),
            ("DATE", "date"),
            ("TIME", "time"),
            ("TIMESTAMP_NTZ", "timestamp_ntz"),
            ("TIMESTAMP_LTZ", "timestamp_ltz"),
            ("TIMESTAMP_TZ", "timestamp_tz"),
            ("BINARY", "binary"),
            ("VARIANT", "variant"),
            ("OBJECT", "object"),
            ("ARRAY", "array"),
        ] {
            assert_eq!(written(true, catalog), written_as, "{catalog}");
        }
        assert_eq!(written(true, "NUMBER(10,2)"), "number(10,2)");
    }

    #[test]
    fn another_adapter_gets_the_catalogs_type_lower_cased_and_nothing_folded() {
        let nodes = json!({ ID: model("orders", cols(&[("a", "int"), ("b", "")]), json!({})) });
        let m =
            one(&build("duckdb", nodes, Some(table(ID, ["ANALYTICS", "MARTS", "ORDERS"], &[("a", "INTEGER"), ("b", "DECIMAL(18,3)")]))));
        // `int` is not folded to `integer` off Snowflake: the YAML is changed.
        assert_eq!(m.retype, vec![Retype { name: "a".into(), from: "int".into(), to: "integer".into() }]);
        assert_eq!(m.untyped, vec![Untyped { name: "b".into(), to: "decimal(18,3)".into() }]);
        let nodes = json!({ ID: model("orders", cols(&[("a", "VARCHAR(20)")]), json!({})) });
        let m = one(&build("duckdb", nodes, Some(table(ID, ["ANALYTICS", "MARTS", "ORDERS"], &[("a", "varchar")]))));
        assert_eq!(m.status, Status::Agrees, "{m:?}");
    }

    #[test]
    fn under_a_contract_a_bare_number_is_held_and_never_written() {
        let contract = json!({ "config": { "materialized": "table", "contract": { "enforced": true } } });
        let m = snow(
            &[("id", ""), ("qty", "varchar")],
            &[("ID", "NUMBER"), ("QTY", "NUMBER"), ("PRICE", "NUMBER"), ("NOTE", "TEXT")],
            contract,
        );
        assert!(m.contract);
        assert!(m.untyped.is_empty() && m.retype.is_empty(), "{m:?}");
        let held: Vec<&str> = m.held.iter().map(|h| h.name.as_str()).collect();
        assert_eq!(held, ["id", "qty", "price"]);
        // The column is still added, with no type, so it is not left out of the contract by the tool.
        assert_eq!(m.missing[0], Missing { name: "price".into(), data_type: String::new(), after: "qty".into() });
        assert_eq!(m.missing[1].data_type, "varchar");
        // Off a contract the same number is plain `number`.
        let m = snow(&[("id", "")], &[("ID", "NUMBER")], json!({}));
        assert!(m.held.is_empty());
        assert_eq!(m.untyped[0].to, "number");
        // And a number that carries its precision is written with it.
        let contract = json!({ "config": { "materialized": "table", "contract": { "enforced": true } } });
        let m = snow(&[("id", "")], &[("ID", "NUMBER(10,2)")], contract);
        assert_eq!(m.untyped[0].to, "number(10,2)");
    }

    #[test]
    fn names_are_matched_without_case_and_written_by_snowflakes_rule() {
        let m = snow(
            &[("Order_Id", "number")],
            &[("ORDER_ID", "NUMBER"), ("CUSTOMER_ID", "NUMBER"), ("customer name", "TEXT"), ("Mixed", "TEXT")],
            json!({}),
        );
        let names: Vec<&str> = m.missing.iter().map(|c| c.name.as_str()).collect();
        // All upper case is the unquoted default, written lower case; any other spelling is kept.
        assert_eq!(names, ["customer_id", "customer name", "Mixed"]);
        assert!(m.extra.is_empty());
        assert_eq!(m.missing[0].after, "Order_Id", "named as the YAML has it");
        // Another adapter keeps every name as it is.
        assert_eq!(written_name(false, "ORDER_ID"), "ORDER_ID");
    }

    #[test]
    fn two_quoted_columns_that_differ_only_in_case_each_find_their_own() {
        let m = snow(&[("id", "number"), ("ID", "number")], &[("id", "NUMBER"), ("ID", "NUMBER")], json!({}));
        assert_eq!(m.status, Status::Agrees, "{m:?}");
    }

    fn reason(g: &Graph) -> String {
        let m = one(g);
        assert_eq!(m.status, Status::NotChecked, "{m:?}");
        m.reason
    }

    #[test]
    fn a_model_the_comparison_would_say_nothing_true_of_is_not_checked_and_says_why() {
        let rel = ["ANALYTICS", "MARTS", "ORDERS"];
        let declared = cols(&[("id", "number")]);
        let with =
            |extra: Value, catalog: Option<Value>| build("snowflake", json!({ ID: model("orders", declared.clone(), extra) }), catalog);

        assert!(reason(&with(json!({}), None)).contains("not in catalog.json"));
        assert!(reason(&with(json!({}), Some(json!({})))).contains("not in catalog.json"));
        assert!(reason(&with(json!({ "config": { "materialized": "ephemeral" } }), Some(table(ID, rel, &[("ID", "NUMBER")]))))
            .contains("ephemeral"));
        // Listed with no column: unreadable, never drift.
        assert!(reason(&with(json!({}), Some(table(ID, rel, &[])))).contains("lists no column"));
        // Another schema: a catalog written for another target.
        assert!(reason(&with(json!({}), Some(table(ID, ["ANALYTICS", "DEV_ME", "ORDERS"], &[("ID", "NUMBER")])))).contains("another table"));
        assert!(reason(&with(json!({}), Some(table(ID, ["OTHER_DB", "MARTS", "ORDERS"], &[("ID", "NUMBER")])))).contains("another table"));
        assert!(
            reason(&with(json!({ "version": 2, "latest_version": 2 }), Some(table(ID, rel, &[("ID", "NUMBER")])))).contains("versioned")
        );
    }

    #[test]
    fn the_relation_is_compared_without_case_and_by_the_alias() {
        let nodes = json!({ ID: model("orders", cols(&[("id", "number")]), json!({ "alias": "orders_v1" })) });
        let g = build("snowflake", nodes.clone(), Some(table(ID, ["analytics", "marts", "ORDERS_V1"], &[("ID", "NUMBER")])));
        assert_eq!(one(&g).status, Status::Agrees);
        let g = build("snowflake", nodes, Some(table(ID, ["ANALYTICS", "MARTS", "ORDERS"], &[("ID", "NUMBER")])));
        assert_eq!(one(&g).status, Status::NotChecked, "the table of the model's name is not the one it is aliased to");
    }

    #[test]
    fn a_catalog_that_names_no_relation_is_not_held_to_one() {
        let catalog = json!({ ID: { "columns": { "ID": { "name": "ID", "type": "NUMBER", "index": 1 } } } });
        let g = build("snowflake", json!({ ID: model("orders", cols(&[("id", "number")]), json!({})) }), Some(catalog));
        assert_eq!(one(&g).status, Status::Agrees);
    }

    #[test]
    fn the_yaml_is_read_before_the_catalog_filled_its_types() {
        // `merge_catalog` fills an empty type in place, which would make a YAML that declares none look as if it did.
        let g = build(
            "snowflake",
            json!({ ID: model("orders", cols(&[("id", "")]), json!({})) }),
            Some(table(ID, ["ANALYTICS", "MARTS", "ORDERS"], &[("ID", "NUMBER")])),
        );
        assert_eq!(g.nodes[0].columns[0].data_type, "NUMBER");
        assert_eq!(one(&g).untyped.len(), 1);
    }

    static N: AtomicU32 = AtomicU32::new(0);

    fn project() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("edith-yaml-sync-{}-{}", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("models")).unwrap();
        std::fs::create_dir_all(dir.join("target/run/shop/models")).unwrap();
        std::fs::write(dir.join("models/orders.sql"), "select 1").unwrap();
        std::fs::write(dir.join("models/schema.yml"), "version: 2").unwrap();
        std::fs::write(dir.join("dbt_project.yml"), "name: shop").unwrap();
        std::fs::write(dir.join("target/run/shop/models/orders.sql"), "create table").unwrap();
        dir
    }

    fn set(dir: &Path, rel: &str, secs: u64) {
        // Backdated rather than waited for: mtime has one-second resolution.
        let when = std::time::UNIX_EPOCH + std::time::Duration::from_secs(secs);
        std::fs::File::options().write(true).open(dir.join(rel)).unwrap().set_modified(when).unwrap();
    }

    fn flags(dir: &Path, catalog_at: u64) -> (bool, bool) {
        flags_of(dir, catalog_at, false)
    }

    fn flags_of(dir: &Path, catalog_at: u64, contract: bool) -> (bool, bool) {
        let config = json!({ "config": { "materialized": "table", "contract": { "enforced": contract } } });
        let nodes = json!({ ID: model("orders", cols(&[("id", "number")]), config) });
        let mut g = build("snowflake", nodes, Some(table(ID, ["ANALYTICS", "MARTS", "ORDERS"], &[("ID", "NUMBER"), ("X", "TEXT")])));
        g.catalog_mtime = catalog_at;
        let m = check(&g, dir, &dir.join("target"), Some(&[0]), &|_| None).models.remove(0);
        assert_eq!(m.status, Status::Disagrees);
        (m.catalog_behind_run, m.table_behind_code)
    }

    #[test]
    fn a_run_file_newer_than_the_catalog_says_the_catalog_is_behind() {
        let dir = project();
        for f in ["models/orders.sql", "models/schema.yml", "dbt_project.yml"] {
            set(&dir, f, 100);
        }
        set(&dir, "target/run/shop/models/orders.sql", 500);
        assert_eq!(flags(&dir, 400), (true, false));
        assert_eq!(flags(&dir, 600), (false, false));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_run_file_older_than_the_code_says_the_table_is_behind() {
        let dir = project();
        for f in ["models/orders.sql", "models/schema.yml", "dbt_project.yml"] {
            set(&dir, f, 100);
        }
        set(&dir, "target/run/shop/models/orders.sql", 500);
        set(&dir, "models/orders.sql", 700);
        assert_eq!(flags(&dir, 800), (false, true));
        let _ = std::fs::remove_dir_all(&dir);
        // dbt_project.yml counts too.
        let dir = project();
        for f in ["models/orders.sql", "models/schema.yml"] {
            set(&dir, f, 100);
        }
        set(&dir, "target/run/shop/models/orders.sql", 500);
        set(&dir, "dbt_project.yml", 700);
        assert_eq!(flags(&dir, 800), (false, true));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_schema_file_makes_the_table_behind_only_under_a_contract() {
        // Without a contract the YAML does not change the table's columns.
        let dir = project();
        for f in ["models/orders.sql", "dbt_project.yml"] {
            set(&dir, f, 100);
        }
        set(&dir, "target/run/shop/models/orders.sql", 500);
        set(&dir, "models/schema.yml", 700);
        assert_eq!(flags_of(&dir, 800, false), (false, false));
        assert_eq!(flags_of(&dir, 800, true), (false, true));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_whole_project_answer_carries_no_freshness_flag() {
        let dir = project();
        for f in ["models/orders.sql", "models/schema.yml", "dbt_project.yml"] {
            set(&dir, f, 100);
        }
        set(&dir, "target/run/shop/models/orders.sql", 500);
        let nodes = json!({ ID: model("orders", cols(&[("id", "number")]), json!({})) });
        let mut g = build("snowflake", nodes, Some(table(ID, ["ANALYTICS", "MARTS", "ORDERS"], &[("ID", "NUMBER"), ("X", "TEXT")])));
        g.catalog_mtime = 400;
        let m = check(&g, &dir, &dir.join("target"), None, &|_| None).models.remove(0);
        assert_eq!((m.status, m.catalog_behind_run, m.table_behind_code), (Status::Disagrees, false, false));
        let m = check(&g, &dir, &dir.join("target"), Some(&[0]), &|_| None).models.remove(0);
        assert!(m.catalog_behind_run);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn no_run_file_no_flag() {
        let dir = project();
        std::fs::remove_file(dir.join("target/run/shop/models/orders.sql")).unwrap();
        assert_eq!(flags(&dir, 800), (false, false));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A project whose models' place comes from the environment, with the
    /// `.env` files given, and a model built in the manifest's own schema.
    fn deployed(envs: &[(&str, &str)], relation: [&str; 3], extra: Value) -> (std::path::PathBuf, Graph) {
        let dir = project();
        for (file, body) in envs {
            std::fs::write(dir.join(file), body).unwrap();
        }
        let mut config = json!({
            "database": "DEV_DB", "schema": "dev_me",
            "config": { "materialized": "table", "database": "DEV_DB", "schema": "dev_me" },
            "unrendered_config": { "database": "{{ env_var('SF_DB') }}", "schema": "{{ env_var('SF_SCHEMA', 'marts') }}" },
        });
        config.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        let nodes = json!({ ID: model("orders", cols(&[("id", "number")]), config) });
        let g = build("snowflake", nodes, Some(table(ID, relation, &[("ID", "NUMBER"), ("NOTE", "TEXT")])));
        (dir, g)
    }

    #[test]
    fn a_catalog_written_for_a_deployed_environment_is_compared_and_names_its_file() {
        let envs = [(".env.qa", "SF_DB=QA_DB\nSF_SCHEMA=MARTS_QA\n"), (".env.prod", "SF_DB=PROD_DB\nSF_SCHEMA=MARTS\n")];
        let (dir, g) = deployed(&envs, ["prod_db", "marts", "orders"], json!({}));
        let answer = check(&g, &dir, &dir.join("target"), None, &|_| None);
        let m = &answer.models[0];
        assert_eq!((m.status, m.catalog_env.as_str(), m.deployed), (Status::Disagrees, ".env.prod", true), "{m:?}");
        assert_eq!(answer.catalog_env, ".env.prod");
        assert_eq!(m.missing, vec![Missing { name: "note".into(), data_type: "varchar".into(), after: "id".into() }]);
        let v = serde_json::to_value(&answer).unwrap();
        assert_eq!(v["models"][0]["catalog_env"], ".env.prod");
        assert!(!v.to_string().contains("QA_DB"), "no value of another file leaves");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_manifests_own_relation_is_the_manifest_and_not_deployed() {
        let (dir, g) = deployed(&[(".env.prod", "SF_DB=PROD_DB\n")], ["DEV_DB", "DEV_ME", "ORDERS"], json!({}));
        let answer = check(&g, &dir, &dir.join("target"), None, &|_| None);
        assert_eq!(
            (answer.models[0].catalog_env.as_str(), answer.models[0].deployed, answer.catalog_env.as_str()),
            ("manifest", false, "manifest")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_relation_no_file_resolves_to_is_not_checked() {
        // `.env.qa` leaves SF_DB out: its place is not resolved in full, so it counts for nothing.
        let (dir, g) = deployed(&[(".env.qa", "SF_SCHEMA=MARTS\n")], ["PROD_DB", "MARTS", "ORDERS"], json!({}));
        let m = check(&g, &dir, &dir.join("target"), None, &|_| None).models.remove(0);
        assert_eq!(m.status, Status::NotChecked);
        assert!(m.reason.contains("no .env file places it there"), "{}", m.reason);
        let answer = check(&g, &dir, &dir.join("target"), None, &|_| None);
        assert_eq!(answer.catalog_env, "", "no model was checked");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn two_files_resolving_alike_give_the_first_by_name() {
        let same = "SF_DB=PROD_DB\nSF_SCHEMA=MARTS\n";
        let (dir, g) = deployed(&[(".env.uat", same), (".env.prod", same)], ["PROD_DB", "MARTS", "ORDERS"], json!({}));
        assert_eq!(check(&g, &dir, &dir.join("target"), None, &|_| None).models[0].catalog_env, ".env.prod");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_deployed_catalog_has_no_run_file_flag_but_says_when_the_code_is_newer() {
        let envs = [(".env.prod", "SF_DB=PROD_DB\nSF_SCHEMA=MARTS\n")];
        let (dir, g) = deployed(&envs, ["PROD_DB", "MARTS", "ORDERS"], json!({}));
        let mut g = g;
        g.catalog_mtime = 400;
        for f in ["models/orders.sql", "models/schema.yml", "dbt_project.yml"] {
            set(&dir, f, 100);
        }
        // A local run is the manifest's target, so it says nothing of the deployed table.
        set(&dir, "target/run/shop/models/orders.sql", 900);
        let m = check(&g, &dir, &dir.join("target"), Some(&[0]), &|_| None).models.remove(0);
        assert_eq!((m.deployed, m.code_after_catalog, m.catalog_behind_run, m.table_behind_code), (true, false, false, false));
        set(&dir, "models/orders.sql", 500);
        let m = check(&g, &dir, &dir.join("target"), Some(&[0]), &|_| None).models.remove(0);
        assert_eq!((m.deployed, m.code_after_catalog, m.catalog_behind_run, m.table_behind_code), (true, true, false, false));
        // Whole project: `deployed` is said, the stat is not made.
        let m = check(&g, &dir, &dir.join("target"), None, &|_| None).models.remove(0);
        assert_eq!((m.deployed, m.code_after_catalog), (true, false));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_model_with_no_location_of_its_own_is_never_credited_to_an_env_file() {
        // Written nowhere, it resolves to empty parts under any file, which matched anything.
        let dir = project();
        std::fs::write(dir.join(".env"), "WAREHOUSE_USER=me\n").unwrap();
        let nodes = json!({ ID: model("orders", cols(&[("id", "number")]), json!({})) });
        let g = build("snowflake", nodes, Some(table(ID, ["CI_DB", "CI_SCHEMA", "ORDERS"], &[("ID", "NUMBER"), ("NOTE", "TEXT")])));
        let m = check(&g, &dir, &dir.join("target"), None, &|_| None).models.remove(0);
        assert_eq!((m.status, m.deployed), (Status::NotChecked, false), "{m:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_literal_location_resolves_alike_under_every_file_and_credits_none() {
        let literal = json!({
            "config": { "materialized": "table", "database": "PROD_DB", "schema": "marts" },
            "unrendered_config": { "database": "PROD_DB", "schema": "marts" },
        });
        let (dir, g) = deployed(&[(".env", "X=1\n"), (".env.prod", "Y=1\n")], ["PROD_DB", "MARTS", "ORDERS"], literal);
        let m = check(&g, &dir, &dir.join("target"), None, &|_| None).models.remove(0);
        assert_eq!((m.status, m.deployed), (Status::NotChecked, false), "{m:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_file_the_menu_hides_says_nothing_of_an_environment() {
        let body = "SF_DB=PROD_DB\nSF_SCHEMA=MARTS\n";
        let rel = ["PROD_DB", "MARTS", "ORDERS"];
        let env_of = |g: &Graph, dir: &Path, chosen: &dyn Fn(&str) -> Option<bool>| {
            let m = check(g, dir, &dir.join("target"), None, chosen).models.remove(0);
            (m.status, m.catalog_env)
        };
        // A template by its name.
        let (dir, g) = deployed(&[(".env.prod.example", body)], rel, json!({}));
        assert_eq!(env_of(&g, &dir, &|_| None).0, Status::NotChecked);
        // Shown anyway when the user said so, as the menu does.
        assert_eq!(env_of(&g, &dir, &|f| (f == ".env.prod.example").then_some(false)), (Status::Disagrees, ".env.prod.example".into()));
        let _ = std::fs::remove_dir_all(&dir);
        // Hidden by the user.
        let (dir, g) = deployed(&[(".env.prod", body)], rel, json!({}));
        assert_eq!(env_of(&g, &dir, &|_| None), (Status::Disagrees, ".env.prod".into()));
        assert_eq!(env_of(&g, &dir, &|f| (f == ".env.prod").then_some(true)).0, Status::NotChecked);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_catalogs_environment_on_a_tie_is_the_first_by_name() {
        let dir = project();
        std::fs::write(dir.join(".env.prod"), "SF_DB=PROD_DB\nSF_SCHEMA=MARTS\n").unwrap();
        let location = json!({
            "config": { "materialized": "table", "database": "DEV_DB", "schema": "dev_me" },
            "unrendered_config": { "database": "{{ env_var('SF_DB') }}", "schema": "{{ env_var('SF_SCHEMA') }}" },
            "database": "DEV_DB", "schema": "dev_me",
        });
        let second = "model.shop.payments";
        let mut here = model("orders", cols(&[("id", "number")]), location.clone());
        here["database"] = json!("DEV_DB");
        let mut there = model("payments", cols(&[("id", "number")]), location);
        there["database"] = json!("DEV_DB");
        let nodes = json!({ ID: here, second: there });
        let mut catalog = table(ID, ["PROD_DB", "MARTS", "ORDERS"], &[("ID", "NUMBER")]);
        // The second model's catalog is the manifest's own relation.
        catalog
            .as_object_mut()
            .unwrap()
            .extend(table(second, ["DEV_DB", "DEV_ME", "PAYMENTS"], &[("ID", "NUMBER")]).as_object().unwrap().clone());
        let g = build("snowflake", nodes, Some(catalog));
        let answer = check(&g, &dir, &dir.join("target"), None, &|_| None);
        let mut seen: Vec<&str> = answer.models.iter().map(|m| m.catalog_env.as_str()).collect();
        seen.sort_unstable();
        assert_eq!(seen, [".env.prod", "manifest"]);
        assert_eq!(answer.catalog_env, ".env.prod", "one each: the first by name");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_database_the_config_leaves_out_is_the_manifests() {
        // The schema comes from the environment, the database from the profile's target.
        let dir = project();
        std::fs::write(dir.join(".env.prod"), "DBT_SCHEMA=MARTS\n").unwrap();
        let config = json!({
            "config": { "materialized": "table", "schema": "dev_me" },
            "unrendered_config": { "schema": "{{ env_var('DBT_SCHEMA') }}" },
            "database": "ANALYTICS", "schema": "dev_me",
        });
        let nodes = json!({ ID: model("orders", cols(&[("id", "number")]), config) });
        let g = build("snowflake", nodes, Some(table(ID, ["ANALYTICS", "MARTS", "ORDERS"], &[("ID", "NUMBER"), ("NOTE", "TEXT")])));
        let m = check(&g, &dir, &dir.join("target"), None, &|_| None).models.remove(0);
        assert_eq!((m.status, m.catalog_env.as_str(), m.deployed), (Status::Disagrees, ".env.prod", true), "{m:?}");
        // The same schema in another database is another table.
        let nodes = json!({ ID: model("orders", cols(&[("id", "number")]), json!({ "config": { "schema": "dev_me" }, "unrendered_config": { "schema": "{{ env_var('DBT_SCHEMA') }}" } })) });
        let g = build("snowflake", nodes, Some(table(ID, ["OTHER_DB", "MARTS", "ORDERS"], &[("ID", "NUMBER")])));
        assert_eq!(check(&g, &dir, &dir.join("target"), None, &|_| None).models[0].status, Status::NotChecked);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_list_of_models_is_answered_with_the_flags_of_each_and_of_none_else() {
        let dir = project();
        for f in ["models/orders.sql", "models/schema.yml", "dbt_project.yml"] {
            set(&dir, f, 100);
        }
        set(&dir, "target/run/shop/models/orders.sql", 500);
        let two = "model.shop.payments";
        let mut nodes = json!({ ID: model("orders", cols(&[("id", "number")]), json!({})) });
        nodes[two] = model("payments", cols(&[("id", "number")]), json!({}));
        let mut catalog = table(ID, ["ANALYTICS", "MARTS", "ORDERS"], &[("ID", "NUMBER"), ("X", "TEXT")]);
        catalog
            .as_object_mut()
            .unwrap()
            .extend(table(two, ["ANALYTICS", "MARTS", "PAYMENTS"], &[("ID", "NUMBER"), ("X", "TEXT")]).as_object().unwrap().clone());
        let mut g = build("snowflake", nodes, Some(catalog));
        g.catalog_mtime = 400;
        let answer = check(&g, &dir, &dir.join("target"), Some(&[0, 1]), &|_| None);
        let flagged: Vec<(&str, bool)> = answer.models.iter().map(|m| (m.name.as_str(), m.catalog_behind_run)).collect();
        assert_eq!(flagged, [("orders", true), ("payments", false)], "payments has no run file");
        let answer = check(&g, &dir, &dir.join("target"), Some(&[1]), &|_| None);
        assert_eq!(answer.models.len(), 1);
        assert_eq!(answer.models[0].name, "payments");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn one_model_can_be_asked_for_and_the_answer_is_serialised_without_what_is_empty() {
        let nodes = json!({ ID: model("orders", cols(&[("id", "number")]), json!({})) });
        let g = build("snowflake", nodes, Some(table(ID, ["ANALYTICS", "MARTS", "ORDERS"], &[("ID", "NUMBER")])));
        let answer = check(&g, Path::new("/nonexistent"), Path::new("/nonexistent/target"), Some(&[0]), &|_| None);
        let v = serde_json::to_value(&answer).unwrap();
        assert_eq!(v["catalog_at"], 1_000);
        assert_eq!(v["models"][0]["status"], "agrees");
        assert!(v["models"][0].get("missing").is_none());
        assert!(v["models"][0].get("reason").is_none());
    }
}
