//! The keys a node declares (0127): a list of columns that identify a row,
//! from an incremental model's or a snapshot's `unique_key`, from a `unique`
//! test on one column, or from a `unique_combination_of_columns` test.
//!
//! - Two declarations of one list are one key, whatever the order and the case
//!   of its columns, and a key says every place that declares it.
//! - A column of a key is nullable when no enabled `not_null` test of the node
//!   names it: dbt's `unique` skips nulls, and a merge on a null key matches
//!   nothing and inserts a duplicate.
//! - Only what the manifest says is read, never the SQL: a test with a `where:`
//!   checks part of the rows, a kwarg that is not a list of strings is Jinja
//!   left unrendered, and a test named through another package than dbt's or
//!   dbt_utils is not known for the test it looks like.
//!
//! The order of a node's keys is the same on every start: `unique_key` first,
//! then the fewest columns, then the lowercased column list. The page letters
//! the keys in that order.

use crate::graph::{declaring, Graph, Kind, Node};
use std::collections::BTreeSet;

/// One place that declares a key: the config, or a test by its id and name.
#[derive(serde::Serialize, Debug, Clone, PartialEq)]
pub struct KeyFrom {
    /// `unique_key`, `unique` or `combination`.
    pub kind: &'static str,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub name: String,
}

#[derive(serde::Serialize, Debug, Clone, PartialEq)]
pub struct NodeKey {
    /// The columns as the first declaration wrote them, in its order: the
    /// `unique_key` when the key is declared there, else the test's list.
    pub entries: Vec<String>,
    pub from: Vec<KeyFrom>,
    /// The entries no enabled `not_null` test of the node names. An expression
    /// of a `unique_key` is no column and is never listed.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub nullable: Vec<String>,
    /// The key's columns as `norm` writes them, sorted: what two declarations
    /// are compared by.
    #[serde(skip)]
    set: BTreeSet<String>,
}

impl NodeKey {
    /// Whether a test declares it, not only the config.
    pub fn tested(&self) -> bool {
        self.from.iter().any(|f| f.kind != "unique_key")
    }

    pub fn in_config(&self) -> bool {
        self.from.iter().any(|f| f.kind == "unique_key")
    }
}

/// A column name as the warehouse compares it: trimmed, quotes off, lower case.
/// The same reading as the page's `columnKey`.
fn norm(name: &str) -> String {
    let s = name.trim();
    let quoted = [('"', '"'), ('`', '`'), ('[', ']')];
    for (open, close) in quoted {
        if s.len() >= 2 && s.starts_with(open) && s.ends_with(close) {
            return s[1..s.len() - 1].to_lowercase();
        }
    }
    s.to_lowercase()
}

/// Whether a `unique_key` entry names a column, quoted or a bare word, rather
/// than an expression such as `order_id || '-' || line_number`, which is not
/// parsed.
fn is_column(entry: &str) -> bool {
    let s = entry.trim();
    let quoted = [('"', '"'), ('`', '`'), ('[', ']')].iter().any(|&(o, c)| s.len() >= 2 && s.starts_with(o) && s.ends_with(c));
    quoted || (!s.is_empty() && s.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '$'))
}

/// A test of dbt's own or of the package that defines `generic`: bare, or
/// named through that package. Another namespace is another package's test of
/// the same name, which nothing here knows to mean the same.
fn is_generic(t: &Node, generic: &str, package: &str) -> bool {
    t.test_name == generic && (t.namespace.is_empty() || t.namespace == package)
}

/// The column a `unique` or `not_null` test names: the node's own, else the
/// kwarg of a test written under its model. Empty when it names none.
fn column_of(t: &Node) -> &str {
    if t.column.trim().is_empty() { t.column_arg.trim() } else { t.column.trim() }
}

/// Whether the test checks every row: a `where:` limits it to some.
fn whole(t: &Node) -> bool {
    !t.settings.iter().any(|s| s.key == "where")
}

/// A key as it is gathered, borrowing from the graph: the checks' count wants
/// no copy of what it only counts, and `keys` copies what it sends.
struct Draft<'a> {
    entries: Vec<&'a str>,
    set: BTreeSet<String>,
    /// The test that declares it, none for the `unique_key`.
    from: Vec<(&'static str, Option<&'a Node>)>,
}

impl Draft<'_> {
    fn in_config(&self) -> bool {
        self.from.iter().any(|f| f.1.is_none())
    }

    fn tested(&self) -> bool {
        self.from.iter().any(|f| f.1.is_some())
    }

    /// The entries no `not_null` covers. An expression is no column and is
    /// never one of them.
    fn nullable<'s>(&'s self, covered: &'s BTreeSet<String>) -> impl Iterator<Item = &'s str> {
        self.entries.iter().copied().filter(move |e| is_column(e) && !covered.contains(&norm(e)))
    }
}

/// A declaration of a key: a new key, or one more place declaring one of the
/// same columns.
fn add<'a>(drafts: &mut Vec<Draft<'a>>, entries: Vec<&'a str>, from: (&'static str, Option<&'a Node>)) {
    let set: BTreeSet<String> = entries.iter().map(|e| norm(e)).collect();
    match drafts.iter_mut().find(|k| k.set == set) {
        Some(k) => k.from.push(from),
        None => drafts.push(Draft { entries, set, from: vec![from] }),
    }
}

/// Whether dbt ignores the model's `unique_key`: the strategy as the box shows
/// it, else the configured one, inserts without matching on a key: `append`,
/// `insert_overwrite` and Databricks' `replace_where`, and `microbatch` on
/// every adapter but Postgres, whose own microbatch merges on it. An unread or
/// custom strategy is not assumed to. The Columns tag keeps the key all the
/// same; the checks leave a key only the config declares.
pub fn config_ignored(adapter: &str, n: &Node) -> bool {
    if !n.is_incremental() {
        return false;
    }
    let strategy = n.shown.as_ref().map(|s| s.shown.as_str()).unwrap_or(n.strategy.as_str());
    let is = |name: &str| strategy.eq_ignore_ascii_case(name);
    is("append") || is("insert_overwrite") || is("replace_where") || (is("microbatch") && !adapter.eq_ignore_ascii_case("postgres"))
}

/// The keys of node `i` and the columns its `not_null` tests cover, from the
/// tests it declares and its `unique_key`; none for a node dbt never runs.
fn gather(graph: &Graph, i: u32) -> (Vec<Draft<'_>>, BTreeSet<String>) {
    let n = &graph.nodes[i as usize];
    if n.disabled || matches!(n.kind, Kind::Test | Kind::Exposure) {
        return (Vec::new(), BTreeSet::new());
    }
    // Its tests as the node lists them: the ones with it as a parent, of which
    // some only point at it, a relationships target. Those it declares are
    // found by `declaring`, as everywhere else.
    let mut tests: Vec<&Node> = n
        .tests
        .iter()
        .map(|&t| &graph.nodes[t as usize])
        .filter(|t| declaring(&graph.nodes, &graph.index, t) == Some(i))
        .collect();
    tests.sort_by(|a, b| a.id.cmp(&b.id));

    let mut drafts: Vec<Draft> = Vec::new();
    if (n.is_incremental() || n.kind == Kind::Snapshot) && !n.unique_key.is_empty() {
        add(&mut drafts, n.unique_key.iter().map(String::as_str).collect(), ("unique_key", None));
    }
    let mut covered: BTreeSet<String> = BTreeSet::new();
    for &t in &tests {
        if !whole(t) {
            continue;
        }
        let column = column_of(t);
        if is_generic(t, "unique", "dbt") && !column.is_empty() {
            add(&mut drafts, vec![column], ("unique", Some(t)));
        } else if is_generic(t, "unique_combination_of_columns", "dbt_utils") && !t.combination.is_empty() {
            add(&mut drafts, t.combination.iter().map(String::as_str).collect(), ("combination", Some(t)));
        } else if is_generic(t, "not_null", "dbt") && !column.is_empty() {
            covered.insert(norm(column));
        }
    }
    (drafts, covered)
}

/// The keys of node `i`, from the tests it declares and its `unique_key`. It
/// reads the node's own list of tests, so asking it of every model walks each
/// test once and never models times tests.
pub fn keys(graph: &Graph, i: u32) -> Vec<NodeKey> {
    let (drafts, covered) = gather(graph, i);
    let mut keys: Vec<NodeKey> = drafts
        .into_iter()
        .map(|d| NodeKey {
            nullable: d.nullable(&covered).map(str::to_string).collect(),
            entries: d.entries.iter().map(|e| e.to_string()).collect(),
            from: d
                .from
                .iter()
                .map(|&(kind, t)| KeyFrom {
                    kind,
                    id: t.map(|t| t.id.clone()).unwrap_or_default(),
                    name: t.map(|t| t.name.clone()).unwrap_or_default(),
                })
                .collect(),
            set: d.set,
        })
        .collect();
    // `unique_key` first, then the fewest columns, then the column list.
    keys.sort_by_cached_key(|k| (!k.in_config(), k.set.len(), k.set.iter().cloned().collect::<Vec<_>>().join(",")));
    keys
}

/// What the checks count of a node's keys, without a row or a string of what
/// they would say: the Reports index asks it of every model whenever it is
/// read.
#[derive(Default)]
pub struct Tally {
    /// Keys with a column no `not_null` covers.
    pub nullable: usize,
    /// The `unique_key` is declared by no test.
    pub config_untested: bool,
    /// Some test declares a key.
    pub tested: bool,
}

pub fn tally(graph: &Graph, i: u32) -> Tally {
    let (drafts, covered) = gather(graph, i);
    let ignored = config_ignored(&graph.meta.adapter, &graph.nodes[i as usize]);
    Tally {
        nullable: drafts.iter().filter(|d| !(ignored && d.in_config() && !d.tested()) && d.nullable(&covered).next().is_some()).count(),
        config_untested: !ignored && drafts.iter().any(|d| d.in_config() && !d.tested()),
        tested: drafts.iter().any(Draft::tested),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::RawManifest;
    use serde_json::{json, Value};
    use std::path::Path;

    fn model(name: &str, extra: Value) -> Value {
        let mut m = json!({
            "name": name, "resource_type": "model", "package_name": "shop",
            "original_file_path": format!("models/{name}.sql"), "fqn": ["shop", name],
            "config": { "materialized": "table" },
        });
        m.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        m
    }

    fn incremental(name: &str, unique_key: Value) -> Value {
        model(name, json!({ "config": { "materialized": "incremental", "unique_key": unique_key } }))
    }

    /// A generic test of `orders`, by its short name.
    fn test(id: &str, generic: &str, namespace: Option<&str>, column: Option<&str>, kwargs: Value) -> (String, Value) {
        let mut t = json!({
            "name": id, "resource_type": "test", "package_name": "shop",
            "original_file_path": "models/schema.yml", "fqn": ["shop", id],
            "attached_node": "model.shop.orders",
            "test_metadata": { "name": generic, "namespace": namespace, "kwargs": kwargs },
        });
        if let Some(c) = column {
            t["column_name"] = json!(c);
        }
        (format!("test.shop.{id}"), t)
    }

    fn graph(orders: Value, tests: Vec<(String, Value)>) -> Graph {
        let mut nodes = serde_json::Map::new();
        nodes.insert("model.shop.orders".into(), orders);
        let mut parents = serde_json::Map::new();
        for (id, t) in tests {
            parents.insert(id.clone(), json!(["model.shop.orders"]));
            nodes.insert(id, t);
        }
        let raw: RawManifest = serde_json::from_value(json!({
            "metadata": { "project_name": "shop" }, "nodes": nodes, "parent_map": parents,
        }))
        .unwrap();
        Graph::build(raw, Path::new("manifest.json"), 1, 0)
    }

    fn orders(g: &Graph) -> Vec<NodeKey> {
        keys(g, g.index["model.shop.orders"])
    }

    fn unique(id: &str, column: &str) -> (String, Value) {
        test(id, "unique", None, Some(column), json!({}))
    }

    fn not_null(id: &str, column: &str) -> (String, Value) {
        test(id, "not_null", None, Some(column), json!({}))
    }

    fn combo(id: &str, columns: Value) -> (String, Value) {
        test(id, "unique_combination_of_columns", Some("dbt_utils"), None, json!({ "combination_of_columns": columns }))
    }

    fn kinds(k: &NodeKey) -> Vec<&str> {
        k.from.iter().map(|f| f.kind).collect()
    }

    #[test]
    fn a_config_and_a_test_on_one_set_are_one_key_in_whatever_order_and_case() {
        let g = graph(
            incremental("orders", json!(["Order_ID", "\"Line\""])),
            vec![combo("c", json!(["line", "order_id"])), not_null("n1", "order_id"), not_null("n2", "LINE")],
        );
        let k = orders(&g);
        assert_eq!(k.len(), 1);
        // The config declared it first, so its order and spelling stand.
        assert_eq!(k[0].entries, ["Order_ID", "\"Line\""]);
        assert_eq!(kinds(&k[0]), ["unique_key", "combination"]);
        assert!(k[0].nullable.is_empty());
    }

    #[test]
    fn two_combination_tests_of_one_set_merge_and_name_each_test() {
        let g = graph(model("orders", json!({})), vec![combo("a", json!(["x", "y"])), combo("b", json!(["Y", "X"]))]);
        let k = orders(&g);
        assert_eq!(k.len(), 1);
        assert_eq!(k[0].from.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(), ["a", "b"]);
        // Without a config, the first test's list sets the order.
        assert_eq!(k[0].entries, ["x", "y"]);
        assert_eq!(k[0].nullable, ["x", "y"]);
    }

    #[test]
    fn keys_are_ordered_config_first_then_by_columns_then_by_list() {
        let g = graph(
            incremental("orders", json!(["b", "c"])),
            vec![unique("u_z", "z"), unique("u_a", "a"), combo("c", json!(["a", "b", "c"]))],
        );
        let sets: Vec<Vec<String>> = orders(&g).into_iter().map(|k| k.entries).collect();
        assert_eq!(sets, [vec!["b", "c"], vec!["a"], vec!["z"], vec!["a", "b", "c"]]);
    }

    #[test]
    fn a_not_null_on_a_column_the_yaml_does_not_declare_still_covers_it() {
        // The model declares no column at all: coverage is read from the tests.
        let g = graph(model("orders", json!({})), vec![unique("u", "id"), not_null("n", "ID")]);
        let k = orders(&g);
        assert_eq!(kinds(&k[0]), ["unique"]);
        assert!(k[0].nullable.is_empty());
        let g = graph(model("orders", json!({})), vec![unique("u", "id")]);
        assert_eq!(orders(&g)[0].nullable, ["id"]);
    }

    #[test]
    fn a_test_with_a_where_or_another_namespace_or_unrendered_columns_declares_nothing() {
        let mut limited = unique("limited", "id");
        limited.1["config"] = json!({ "where": "created > '2020-01-01'" });
        let mut limited_nn = not_null("limited_nn", "code");
        limited_nn.1["config"] = json!({ "where": "created > '2020-01-01'" });
        let g = graph(
            model("orders", json!({})),
            vec![
                limited,
                limited_nn,
                test("other", "unique", Some("kit"), Some("id"), json!({})),
                test("foreign", "unique_combination_of_columns", Some("kit"), None, json!({ "combination_of_columns": ["a", "b"] })),
                combo("jinja", json!("{{ var('cols') }}")),
                combo("mixed", json!(["a", 3])),
                unique("code_unique", "code"),
            ],
        );
        let k = orders(&g);
        assert_eq!(k.len(), 1);
        assert_eq!(k[0].entries, ["code"]);
        // A not_null limited by a where covers only some rows.
        assert_eq!(k[0].nullable, ["code"]);
    }

    #[test]
    fn an_expression_entry_stays_an_entry_and_is_never_nullable() {
        let g = graph(incremental("orders", json!(["order_id || '-' || line", "line"])), vec![unique("u", "line")]);
        let k = orders(&g);
        assert_eq!(k.len(), 2);
        assert_eq!(k[0].entries.len(), 2);
        assert_eq!(k[0].nullable, ["line"]);
        assert!(!k[0].tested());
        assert_eq!(kinds(&k[1]), ["unique"]);
    }

    #[test]
    fn a_unique_key_belongs_to_incremental_models_and_snapshots_only() {
        let mut view = model("orders", json!({ "config": { "materialized": "view", "unique_key": "id" } }));
        assert!(orders(&graph(view.clone(), vec![])).is_empty());
        view["resource_type"] = json!("snapshot");
        view["config"] = json!({ "unique_key": "id", "strategy": "check" });
        assert_eq!(orders(&graph(view, vec![])).len(), 1);
    }

    #[test]
    fn a_test_declared_on_another_node_is_not_this_nodes() {
        // The test points at `orders` too, as a relationships test points at
        // its target, and is declared on `elsewhere`.
        let mut other = unique("u", "id");
        other.1["attached_node"] = json!("model.shop.elsewhere");
        let raw: RawManifest = serde_json::from_value(json!({
            "metadata": { "project_name": "shop" },
            "nodes": {
                "model.shop.orders": model("orders", json!({})),
                "model.shop.elsewhere": model("elsewhere", json!({})),
                other.0.clone(): other.1,
            },
            "parent_map": { "test.shop.u": ["model.shop.elsewhere", "model.shop.orders"] },
        }))
        .unwrap();
        let g = Graph::build(raw, Path::new("manifest.json"), 1, 0);
        assert!(orders(&g).is_empty());
        assert_eq!(keys(&g, g.index["model.shop.elsewhere"]).len(), 1);
    }

    #[test]
    fn the_tally_counts_what_the_keys_say() {
        let g = graph(
            incremental("orders", json!(["id", "line"])),
            vec![unique("u", "code"), not_null("n", "code"), unique("v", "id")],
        );
        let t = tally(&g, g.index["model.shop.orders"]);
        // The config key and `id` have a column left, `code` has none.
        assert_eq!((t.nullable, t.config_untested, t.tested), (2, true, true));
    }

    #[test]
    fn a_key_serialises_without_what_is_empty() {
        let g = graph(incremental("orders", json!("id")), vec![not_null("n", "id")]);
        let v = serde_json::to_value(orders(&g)).unwrap();
        assert_eq!(v, json!([{ "entries": ["id"], "from": [{ "kind": "unique_key" }] }]));
    }

    /// dbt-core writes a test under the model's own `data_tests:` with a null
    /// `column_name`: the column is only in the kwargs.
    fn under_model(id: &str, generic: &str, column: Value) -> (String, Value) {
        test(id, generic, None, None, json!({ "column_name": column }))
    }

    #[test]
    fn a_test_written_under_its_model_names_its_column_in_the_kwargs() {
        let g = graph(model("orders", json!({})), vec![under_model("u", "unique", json!("id")), under_model("n", "not_null", json!("ID"))]);
        let k = orders(&g);
        assert_eq!(kinds(&k[0]), ["unique"]);
        assert!(k[0].nullable.is_empty());
        // A kwarg that is no string names no column.
        let g = graph(model("orders", json!({})), vec![under_model("u", "unique", json!(["id"])), under_model("v", "unique", Value::Null)]);
        assert!(orders(&g).is_empty());
        // The node's own column wins, and the chips still read `column`.
        let mut both = under_model("u", "unique", json!("other"));
        both.1["column_name"] = json!("id");
        assert_eq!(orders(&graph(model("orders", json!({})), vec![both]))[0].entries, ["id"]);
    }

    #[test]
    fn a_unique_key_expression_merges_with_a_model_level_unique_on_the_same_text() {
        let expression = "order_id || '-' || line";
        let g = graph(incremental("orders", json!([expression])), vec![under_model("u", "unique", json!(expression))]);
        let k = orders(&g);
        assert_eq!(k.len(), 1);
        assert_eq!(kinds(&k[0]), ["unique_key", "unique"]);
        assert!(k[0].nullable.is_empty(), "an expression is no column");
    }

    #[test]
    fn some_strategies_ignore_a_unique_key_and_nothing_else_does() {
        let build = |name: &str, adapter: &str| {
            let mut m = incremental("orders", json!("id"));
            m["config"]["incremental_strategy"] = json!(name);
            let raw: RawManifest = serde_json::from_value(json!({
                "metadata": { "project_name": "shop", "adapter_type": adapter },
                "nodes": { "model.shop.orders": m },
            }))
            .unwrap();
            Graph::build(raw, Path::new("manifest.json"), 1, 0)
        };
        let ignored = |name: &str, adapter: &str| {
            let g = build(name, adapter);
            config_ignored(&g.meta.adapter, &g.nodes[g.index["model.shop.orders"] as usize])
        };
        for name in ["append", "Insert_Overwrite", "replace_where"] {
            assert!(ignored(name, "snowflake") && ignored(name, "postgres"), "{name}");
        }
        // Postgres' microbatch merges on the key; the others replace by event time.
        assert!(!ignored("microbatch", "postgres") && ignored("microbatch", "snowflake") && ignored("microbatch", ""));
        assert!(!ignored("merge", "snowflake") && !ignored("delete+insert", "snowflake") && !ignored("my_own", "snowflake"));
        // The Columns tag keeps the key, and the tally leaves it out.
        let g = build("append", "snowflake");
        assert_eq!(orders(&g).len(), 1);
        let t = tally(&g, g.index["model.shop.orders"]);
        assert_eq!((t.nullable, t.config_untested), (0, false));
    }

    #[test]
    fn a_kwarg_column_holding_jinja_names_no_column() {
        let g = graph(model("orders", json!({})), vec![under_model("u", "unique", json!("{{ var('pk') }}")), under_model("v", "unique", json!("id"))]);
        let k = orders(&g);
        assert_eq!(k.len(), 1);
        assert_eq!(k[0].entries, ["id"]);
    }
}
