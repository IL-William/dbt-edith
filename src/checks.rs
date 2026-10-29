//! The tool's own checks on the project, worked out from the manifest alone
//! (0116): seven rules dbt itself warns of none of, listed under their own
//! names in Reports, apart from what dbt would warn of (0103).
//!
//! - a source with no freshness threshold, as dbt-core's `has_freshness`
//!   reads the manifest's;
//! - a model nothing tests: no generic test declared on it or one of its
//!   columns, no singular test selecting from it, no unit test of it;
//! - a model with no description;
//! - a source no node reads;
//! - a key of a model, from its `unique_key` or a uniqueness test, with a
//!   column no `not_null` test covers (0127);
//! - an incremental model whose `unique_key` no uniqueness test checks;
//! - a model with no uniqueness test at all.
//!
//! Only the root project's own enabled nodes are looked at: an installed
//! package's models and sources are the package's to document, test and read,
//! and would bury the project's.

use crate::graph::{declaring, Graph, Kind};

/// One node a check lists, with what the page needs to open it and to group it.
#[derive(serde::Serialize, Debug, Clone, PartialEq)]
pub struct Row {
    pub id: String,
    pub name: String,
    pub file: String,
    pub folder: String,
    /// What a row of a key rule says of its key, the columns and the ones left
    /// uncovered. Empty on the other rules.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub detail: String,
    /// The column a row opens in the Catalog's Columns tab, if it has one.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub column: String,
    /// The columns of the key a key row is about, as declared, for a reader
    /// that must not take them out of `detail`, where an expression may hold
    /// a comma.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub key: Vec<String>,
}

/// More rows of one check than a report sends: a project with thousands of
/// undescribed models would otherwise send and draw every one. The count says
/// how many there are.
pub const MAX_ROWS: usize = 1000;

/// One check: how many nodes it finds, and the first `MAX_ROWS` of them by
/// folder, then file, then name.
#[derive(serde::Serialize, Debug, Default)]
pub struct Check {
    pub total: usize,
    pub rows: Vec<Row>,
}

#[derive(serde::Serialize, Debug, Default)]
pub struct Checks {
    pub sources_without_freshness: Check,
    pub models_without_test: Check,
    pub models_without_description: Check,
    pub sources_not_read: Check,
    pub keys_maybe_null: Check,
    pub unique_keys_untested: Check,
    pub models_without_key: Check,
}

/// One node a list holds, by index in the graph, with what a key rule says of
/// its key: a model may hold several keys, one row each.
struct Hit {
    node: u32,
    detail: String,
    column: String,
    key: Vec<String>,
}

impl Hit {
    fn of(node: u32) -> Hit {
        Hit { node, detail: String::new(), column: String::new(), key: Vec::new() }
    }
}

/// The nodes of each list.
struct Found {
    no_freshness: Vec<Hit>,
    no_test: Vec<Hit>,
    no_description: Vec<Hit>,
    not_read: Vec<Hit>,
    nullable_key: Vec<Hit>,
    untested_key: Vec<Hit>,
    no_key: Vec<Hit>,
}

/// Which models some test counts for: a generic one declared on the model or
/// on one of its columns, a singular one selecting from it, or a unit test of
/// it. A generic test also has the model it points at as a parent, a
/// `relationships` test's target, which it does not test.
fn tested(graph: &Graph) -> Vec<bool> {
    let mut tested = vec![false; graph.nodes.len()];
    for t in graph.nodes.iter().filter(|n| n.kind == Kind::Test) {
        if t.test_name.is_empty() {
            // A singular test is a file of its own: the models it refs are its parents.
            for &p in &t.parents {
                tested[p as usize] = true;
            }
        } else if let Some(owner) = declaring(&graph.nodes, &graph.index, t) {
            tested[owner as usize] = true;
        }
    }
    for id in graph.unit_tests.values().flat_map(|u| &u.targets) {
        if let Some(&i) = graph.index.get(id) {
            tested[i as usize] = true;
        }
    }
    tested
}

/// The three key rules for model `i`. A key whose columns are not all covered
/// by a `not_null` is listed whoever declares it, the `unique_key` of a merge
/// included, since a null never matches and is inserted again (0127). Counting
/// only wants how many rows there would be, so it builds none of their text.
fn keys_of(graph: &Graph, i: u32, f: &mut Found, rows: bool) {
    if !rows {
        let t = crate::keys::tally(graph, i);
        f.nullable_key.extend((0..t.nullable).map(|_| Hit::of(i)));
        if t.config_untested && graph.nodes[i as usize].is_incremental() {
            f.untested_key.push(Hit::of(i));
        }
        if !t.tested {
            f.no_key.push(Hit::of(i));
        }
        return;
    }
    let keys = crate::keys::keys(graph, i);
    // A `unique_key` dbt ignores, and no test declares, is no key to check.
    let ignored = crate::keys::config_ignored(&graph.nodes[i as usize]);
    for k in keys.iter().filter(|k| !k.nullable.is_empty() && !(ignored && k.in_config() && !k.tested())) {
        let detail = format!("key {}: no not_null on {}", k.entries.join(", "), k.nullable.join(", "));
        f.nullable_key.push(Hit { node: i, detail, column: k.nullable[0].clone(), key: k.entries.clone() });
    }
    if graph.nodes[i as usize].is_incremental() && !ignored {
        if let Some(k) = keys.iter().find(|k| k.in_config()).filter(|k| !k.tested()) {
            f.untested_key.push(Hit { node: i, detail: format!("unique_key {}", k.entries.join(", ")), column: String::new(), key: k.entries.clone() });
        }
    }
    if !keys.iter().any(|k| k.tested()) {
        f.no_key.push(Hit::of(i));
    }
}

fn found(graph: &Graph, rows: bool) -> Found {
    let root = graph.macros.root();
    let tested = tested(graph);
    let mut f = Found {
        no_freshness: Vec::new(),
        no_test: Vec::new(),
        no_description: Vec::new(),
        not_read: Vec::new(),
        nullable_key: Vec::new(),
        untested_key: Vec::new(),
        no_key: Vec::new(),
    };
    for (i, n) in graph.nodes.iter().enumerate() {
        // Disabled nodes never run, and a package's are its own to look after,
        // as the job coverage leaves them (`jobs::coverage`).
        if n.disabled || (!root.is_empty() && n.package != root) {
            continue;
        }
        let i = i as u32;
        match n.kind {
            Kind::Source => {
                // Only `Some(false)` is a verdict: a manifest that does not say
                // gives none, never a missing threshold.
                if n.freshness == Some(false) {
                    f.no_freshness.push(Hit::of(i));
                }
                if n.children.is_empty() {
                    f.not_read.push(Hit::of(i));
                }
            }
            Kind::Model => {
                if !tested[i as usize] {
                    f.no_test.push(Hit::of(i));
                }
                if n.description.trim().is_empty() {
                    f.no_description.push(Hit::of(i));
                }
                keys_of(graph, i, &mut f, rows);
            }
            _ => {}
        }
    }
    f
}

/// How many rows the seven lists hold in all, without building one: the
/// Reports index asks it whenever it is read.
pub fn count(graph: &Graph) -> usize {
    let f = found(graph, false);
    f.no_freshness.len()
        + f.no_test.len()
        + f.no_description.len()
        + f.not_read.len()
        + f.nullable_key.len()
        + f.untested_key.len()
        + f.no_key.len()
}

pub fn checks(graph: &Graph) -> Checks {
    let f = found(graph, true);
    let rows = |list: Vec<Hit>| {
        let total = list.len();
        let mut rows: Vec<Row> = list
            .into_iter()
            .map(|h| {
                let n = &graph.nodes[h.node as usize];
                Row {
                    id: n.id.clone(),
                    name: n.name.clone(),
                    file: n.file.clone(),
                    folder: crate::jobs::folder(graph, h.node),
                    detail: h.detail,
                    column: h.column,
                    key: h.key,
                }
            })
            .collect();
        // A model with two keys listed is two rows, told apart by their detail.
        rows.sort_by(|a, b| {
            a.folder
                .cmp(&b.folder)
                .then_with(|| a.file.cmp(&b.file))
                .then_with(|| a.name.cmp(&b.name))
                .then_with(|| a.id.cmp(&b.id))
                .then_with(|| a.detail.cmp(&b.detail))
        });
        rows.truncate(MAX_ROWS);
        Check { total, rows }
    };
    Checks {
        sources_without_freshness: rows(f.no_freshness),
        models_without_test: rows(f.no_test),
        models_without_description: rows(f.no_description),
        sources_not_read: rows(f.not_read),
        keys_maybe_null: rows(f.nullable_key),
        unique_keys_untested: rows(f.untested_key),
        models_without_key: rows(f.no_key),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::RawManifest;
    use serde_json::{json, Value};
    use std::path::Path;

    fn model(name: &str, package: &str, description: &str) -> Value {
        json!({
            "name": name, "resource_type": "model", "package_name": package,
            "original_file_path": format!("models/{name}.sql"), "fqn": [package, name],
            "description": description, "config": { "materialized": "table" },
        })
    }

    fn source(table: &str, package: &str, extra: Value) -> Value {
        let mut s = json!({
            "name": table, "resource_type": "source", "package_name": package, "source_name": "crm",
            "original_file_path": "models/sources.yml", "fqn": [package, "crm", table],
        });
        s.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        s
    }

    fn generic(name: &str, attached: Option<&str>) -> Value {
        let mut t = json!({
            "name": name, "resource_type": "test", "package_name": "shop",
            "original_file_path": "models/schema.yml", "fqn": ["shop", name],
            "test_metadata": { "name": "relationships" },
        });
        if let Some(a) = attached {
            t["attached_node"] = json!(a);
        }
        t
    }

    fn build(mut manifest: Value) -> Graph {
        manifest["metadata"] = json!({ "project_name": "shop" });
        let raw: RawManifest = serde_json::from_value(manifest).unwrap();
        Graph::build(raw, Path::new("manifest.json"), 1, 0)
    }

    fn names(rows: &[Row]) -> Vec<&str> {
        rows.iter().map(|r| r.name.as_str()).collect()
    }

    #[test]
    fn a_test_declared_on_a_model_counts_and_one_that_only_points_at_it_does_not() {
        let g = build(json!({
            "nodes": {
                "model.shop.customers": model("customers", "shop", "x"),
                "model.shop.orders": model("orders", "shop", "x"),
                "model.shop.lonely": model("lonely", "shop", "x"),
                "test.shop.rel_orders_customers": generic("rel_orders_customers", Some("model.shop.orders")),
            },
            "parent_map": { "test.shop.rel_orders_customers": ["model.shop.customers", "model.shop.orders"] },
        }));
        let c = checks(&g);
        // `customers` is only the target of the test declared on `orders`.
        assert_eq!(names(&c.models_without_test.rows), ["customers", "lonely"]);
    }

    #[test]
    fn a_singular_test_tests_the_models_it_selects_from() {
        let g = build(json!({
            "nodes": {
                "model.shop.customers": model("customers", "shop", "x"),
                "model.shop.orders": model("orders", "shop", "x"),
                "model.shop.lonely": model("lonely", "shop", "x"),
                "test.shop.assert_orders_match": {
                    "name": "assert_orders_match", "resource_type": "test", "package_name": "shop",
                    "original_file_path": "tests/assert_orders_match.sql", "fqn": ["shop", "assert_orders_match"],
                },
            },
            "parent_map": { "test.shop.assert_orders_match": ["model.shop.customers", "model.shop.orders"] },
        }));
        assert_eq!(names(&checks(&g).models_without_test.rows), ["lonely"]);
    }

    #[test]
    fn a_generic_test_found_by_its_file_when_the_manifest_names_no_attached_node_counts() {
        let mut test = generic("not_null_orders_id", None);
        test["column_name"] = json!("id");
        test["test_metadata"] = json!({ "name": "not_null" });
        let mut orders = model("orders", "shop", "x");
        orders["patch_path"] = json!("shop://models/schema.yml");
        let g = build(json!({
            "nodes": {
                "model.shop.orders": orders,
                "model.shop.lonely": model("lonely", "shop", "x"),
                "test.shop.not_null_orders_id": test,
            },
            "parent_map": { "test.shop.not_null_orders_id": ["model.shop.orders"] },
        }));
        assert_eq!(names(&checks(&g).models_without_test.rows), ["lonely"]);
    }

    #[test]
    fn a_source_test_pointing_at_a_model_of_the_same_file_does_not_test_the_model() {
        // dbt-core gives a source's test no attached node, and the model it
        // points at is a parent too, in the same YAML file.
        let mut test = generic("source_relationships_crm_customers_id__id__ref_orders_", None);
        test["column_name"] = json!("id");
        let mut orders = model("orders", "shop", "x");
        orders["patch_path"] = json!("shop://models/schema.yml");
        let mut customers = source("customers", "shop", json!({}));
        customers["original_file_path"] = json!("models/schema.yml");
        let g = build(json!({
            "nodes": { "model.shop.orders": orders, "test.shop.rel_customers_orders": test },
            "sources": { "source.shop.crm.customers": customers },
            "parent_map": { "test.shop.rel_customers_orders": ["model.shop.orders", "source.shop.crm.customers"] },
        }));
        assert_eq!(names(&checks(&g).models_without_test.rows), ["orders"]);
    }

    #[test]
    fn a_model_test_pointing_at_a_source_of_the_same_file_stays_on_the_model() {
        // A manifest with no attached node on a model's test: the source it
        // points at is a parent too, in the same file, and does not take it.
        let mut test = generic("relationships_orders_id__id__source_crm_customers_", None);
        test["column_name"] = json!("id");
        let mut orders = model("orders", "shop", "x");
        orders["patch_path"] = json!("shop://models/schema.yml");
        let mut customers = source("customers", "shop", json!({}));
        customers["original_file_path"] = json!("models/schema.yml");
        let g = build(json!({
            "nodes": { "model.shop.orders": orders, "test.shop.rel_orders_customers": test },
            "sources": { "source.shop.crm.customers": customers },
            "parent_map": { "test.shop.rel_orders_customers": ["model.shop.orders", "source.shop.crm.customers"] },
        }));
        assert!(checks(&g).models_without_test.rows.is_empty());
    }

    /// A model `orders` and a source table `customers` in one YAML file, and a
    /// test of `test_name` with no attached node, pointing at both.
    fn same_file(test_name: &str, file_key_name: Option<&str>, namespace: Option<&str>) -> Graph {
        let mut test = generic(test_name, None);
        test["column_name"] = json!("id");
        if let Some(k) = file_key_name {
            test["file_key_name"] = json!(k);
        }
        if let Some(n) = namespace {
            test["test_metadata"]["namespace"] = json!(n);
        }
        let mut orders = model("orders", "shop", "x");
        orders["patch_path"] = json!("shop://models/schema.yml");
        let mut customers = source("customers", "shop", json!({}));
        customers["original_file_path"] = json!("models/schema.yml");
        build(json!({
            "nodes": { "model.shop.orders": orders, "test.shop.t": test },
            "sources": { "source.shop.crm.customers": customers },
            "parent_map": { "test.shop.t": ["model.shop.orders", "source.shop.crm.customers"] },
        }))
    }

    #[test]
    fn a_source_test_is_the_sources_by_the_key_dbt_writes_whatever_it_is_named() {
        // A package's generic, which prefixes its namespace, and a `name:` of
        // the user's: neither has the `source_` prefix at its start.
        for name in ["kit_source_relationships_where_customers_id", "my_own_check"] {
            let g = same_file(name, Some("sources.crm"), Some("kit"));
            assert_eq!(names(&checks(&g).models_without_test.rows), ["orders"], "{name}");
        }
    }

    #[test]
    fn a_model_test_stays_on_the_model_with_the_key_and_without() {
        let g = same_file("relationships_orders_id__id__source_crm_customers_", Some("models.orders"), None);
        assert!(checks(&g).models_without_test.rows.is_empty());
        // A custom name that starts like a source's, and a key that says otherwise.
        let g = same_file("source_check", Some("models.orders"), None);
        assert!(checks(&g).models_without_test.rows.is_empty());
        let g = same_file("relationships_orders_id__id__source_crm_customers_", None, None);
        assert!(checks(&g).models_without_test.rows.is_empty());
    }

    #[test]
    fn a_key_that_names_no_parent_leaves_the_name_to_say() {
        // Another producer's shape: the table too, which no source is named.
        let g = same_file("source_relationships_crm_customers_id", Some("sources.crm.customers"), None);
        assert_eq!(names(&checks(&g).models_without_test.rows), ["orders"]);
        // And with a name that says nothing, the first parent of the file.
        let g = same_file("my_own_check", Some("sources.crm.customers"), None);
        assert!(checks(&g).models_without_test.rows.is_empty());
    }

    #[test]
    fn without_the_key_a_namespaced_source_name_is_still_a_sources() {
        let g = same_file("kit_source_relationships_where_customers_id", None, Some("kit"));
        assert_eq!(names(&checks(&g).models_without_test.rows), ["orders"]);
        // The prefix after any other word is not dbt's.
        let g = same_file("a_source_of_trouble", None, Some("kit"));
        assert!(checks(&g).models_without_test.rows.is_empty());
    }

    #[test]
    fn a_versioned_model_is_named_by_the_key_without_its_version() {
        let version = |v: u32| {
            let mut m = model("orders", "shop", "x");
            m["version"] = json!(v);
            m["patch_path"] = json!("shop://models/schema.yml");
            m
        };
        let mut test = generic("not_null_orders_id", None);
        test["file_key_name"] = json!("models.orders");
        let g = build(json!({
            "nodes": { "model.shop.orders.v1": version(1), "model.shop.orders.v2": version(2), "test.shop.t": test },
            "parent_map": { "test.shop.t": ["model.shop.orders.v2"] },
        }));
        assert_eq!(names(&checks(&g).models_without_test.rows), ["orders.v1"]);
    }

    #[test]
    fn a_check_sends_the_first_thousand_rows_and_says_how_many_there_are() {
        let mut nodes = serde_json::Map::new();
        for i in 0..MAX_ROWS + 5 {
            nodes.insert(format!("model.shop.m{i:04}"), model(&format!("m{i:04}"), "shop", ""));
        }
        let g = build(json!({ "nodes": nodes }));
        let c = checks(&g);
        assert_eq!(c.models_without_description.total, MAX_ROWS + 5);
        assert_eq!(c.models_without_description.rows.len(), MAX_ROWS);
        // The first by path, not the first read.
        assert_eq!(c.models_without_description.rows[MAX_ROWS - 1].name, format!("m{:04}", MAX_ROWS - 1));
        assert_eq!(c.models_without_test.total, MAX_ROWS + 5);
        // The index counts every node, not the rows sent: no description, no
        // test, and no key.
        assert_eq!(c.models_without_key.total, MAX_ROWS + 5);
        assert_eq!(count(&g), 3 * (MAX_ROWS + 5));
    }

    #[test]
    fn a_unit_test_tests_its_model() {
        let g = build(json!({
            "nodes": {
                "model.shop.orders": model("orders", "shop", "x"),
                "model.shop.lonely": model("lonely", "shop", "x"),
            },
            "unit_tests": {
                "unit_test.shop.orders.totals": {
                    "name": "totals", "original_file_path": "models/schema.yml",
                    "depends_on": { "macros": [], "nodes": ["model.shop.orders"] },
                },
                // A null where dbt writes an object costs the manifest nothing.
                "unit_test.shop.orders.nothing": { "name": "nothing", "depends_on": null },
            },
        }));
        assert_eq!(names(&checks(&g).models_without_test.rows), ["lonely"]);
    }

    #[test]
    fn only_a_model_is_held_to_a_test_and_a_description() {
        let mut snapshot = model("history", "shop", "");
        snapshot["resource_type"] = json!("snapshot");
        let mut seed = model("raw_orders", "shop", "");
        seed["resource_type"] = json!("seed");
        let g = build(json!({
            "nodes": { "snapshot.shop.history": snapshot, "seed.shop.raw_orders": seed },
        }));
        let c = checks(&g);
        assert!(c.models_without_test.rows.is_empty() && c.models_without_description.rows.is_empty());
    }

    #[test]
    fn a_description_of_blanks_is_none() {
        let g = build(json!({
            "nodes": {
                "model.shop.a": model("a", "shop", ""),
                "model.shop.b": model("b", "shop", " \n\t "),
                "model.shop.c": model("c", "shop", "Orders, one a row."),
            },
        }));
        assert_eq!(names(&checks(&g).models_without_description.rows), ["a", "b"]);
    }

    #[test]
    fn a_threshold_counts_for_dbt_only_with_a_count_and_a_period() {
        let none = json!({
            "warn_after": { "count": null, "period": null }, "error_after": { "count": null, "period": null }, "filter": null,
        });
        let g = build(json!({
            "sources": {
                "source.shop.crm.inherits": source("inherits", "shop", json!({
                    "freshness": { "warn_after": { "count": 1, "period": "day" }, "error_after": { "count": null, "period": null } },
                })),
                "source.shop.crm.own_error": source("own_error", "shop", json!({
                    "freshness": { "warn_after": { "count": null, "period": null }, "error_after": { "count": 2, "period": "hour" } },
                })),
                // `freshness: null` on the table, which switches it off.
                "source.shop.crm.switched_off": source("switched_off", "shop", json!({ "freshness": null })),
                "source.shop.crm.nothing": source("nothing", "shop", json!({ "freshness": none })),
                "source.shop.crm.half": source("half", "shop", json!({
                    "freshness": { "warn_after": { "count": 1, "period": null }, "error_after": { "count": null, "period": null } },
                })),
                "source.shop.crm.empty": source("empty", "shop", json!({ "freshness": {} })),
                // Only the config says it, as a table that was never given the top-level key.
                "source.shop.crm.in_config": source("in_config", "shop", json!({
                    "config": { "freshness": { "error_after": { "count": 3, "period": "day" } } },
                })),
                // A manifest that says nothing of freshness gives no verdict.
                "source.shop.crm.unsaid": source("unsaid", "shop", json!({})),
            },
        }));
        assert_eq!(names(&checks(&g).sources_without_freshness.rows), ["crm.empty", "crm.half", "crm.nothing", "crm.switched_off"]);
    }

    #[test]
    fn a_source_is_read_by_a_node_and_not_by_a_test() {
        let g = build(json!({
            "nodes": {
                "model.shop.stg": model("stg", "shop", "x"),
                "test.shop.fresh_ids": generic("fresh_ids", None),
            },
            "sources": {
                "source.shop.crm.read": source("read", "shop", json!({})),
                "source.shop.crm.only_tested": source("only_tested", "shop", json!({})),
                "source.shop.crm.exposed": source("exposed", "shop", json!({})),
                "source.shop.crm.alone": source("alone", "shop", json!({})),
            },
            "exposures": {
                "exposure.shop.dash": {
                    "name": "dash", "resource_type": "exposure", "package_name": "shop",
                    "original_file_path": "models/exposures.yml", "fqn": ["shop", "dash"],
                },
            },
            "parent_map": {
                "model.shop.stg": ["source.shop.crm.read"],
                "test.shop.fresh_ids": ["source.shop.crm.only_tested"],
                "exposure.shop.dash": ["source.shop.crm.exposed"],
            },
        }));
        assert_eq!(names(&checks(&g).sources_not_read.rows), ["crm.alone", "crm.only_tested"]);
    }

    #[test]
    fn a_disabled_node_and_a_package_are_left_out() {
        let g = build(json!({
            "nodes": {
                "model.shop.kept": model("kept", "shop", ""),
                "model.kit.helper": model("helper", "kit", ""),
            },
            "sources": {
                "source.kit.api.events": source("events", "kit", json!({ "freshness": null })),
                "source.shop.crm.kept": source("kept", "shop", json!({ "freshness": null })),
            },
            "disabled": {
                "model.shop.old": [{
                    "name": "old", "resource_type": "model", "package_name": "shop",
                    "original_file_path": "models/old.sql", "fqn": ["shop", "old"],
                    "config": { "enabled": false },
                }],
                "source.shop.crm.retired": [{
                    "name": "retired", "resource_type": "source", "package_name": "shop", "source_name": "crm",
                    "original_file_path": "models/sources.yml", "fqn": ["shop", "crm", "retired"],
                    "freshness": null, "config": { "enabled": false },
                }],
            },
        }));
        let c = checks(&g);
        assert_eq!(names(&c.models_without_test.rows), ["kept"]);
        assert_eq!(names(&c.models_without_description.rows), ["kept"]);
        assert_eq!(names(&c.sources_without_freshness.rows), ["crm.kept"]);
        assert_eq!(names(&c.sources_not_read.rows), ["crm.kept"]);
        assert_eq!(names(&c.models_without_key.rows), ["kept"]);
        assert_eq!(count(&g), 5);
    }

    #[test]
    fn a_row_is_sorted_by_its_path_and_carries_its_folder() {
        let at = |name: &str, dir: &str| {
            let mut m = model(name, "shop", "");
            m["original_file_path"] = json!(format!("models/{dir}/{name}.sql"));
            m
        };
        let g = build(json!({
            "nodes": {
                "model.shop.z": at("z", "marts"),
                "model.shop.b": at("b", "staging"),
                "model.shop.a": at("a", "staging"),
                "model.shop.top": model("top", "shop", ""),
            },
        }));
        let rows = checks(&g).models_without_description.rows;
        let seen: Vec<(&str, &str)> = rows.iter().map(|r| (r.folder.as_str(), r.name.as_str())).collect();
        assert_eq!(seen, [("models", "top"), ("models/marts", "z"), ("models/staging", "a"), ("models/staging", "b")]);
        assert_eq!(rows[1].id, "model.shop.z");
        assert_eq!(rows[1].file, "models/marts/z.sql");
    }

    /// A model `orders` carrying `config`, and the tests of it given.
    fn keyed(config: Value, tests: Vec<(&str, &str, Option<&str>, Value)>) -> Graph {
        let mut orders = model("orders", "shop", "x");
        orders["config"] = config;
        let mut nodes = serde_json::Map::new();
        nodes.insert("model.shop.orders".into(), orders);
        nodes.insert("model.shop.lonely".into(), model("lonely", "shop", "x"));
        let mut parents = serde_json::Map::new();
        for (name, generic, column, kwargs) in tests {
            let mut t = generic_test(name, "model.shop.orders", generic, kwargs);
            if let Some(c) = column {
                t["column_name"] = json!(c);
            }
            parents.insert(format!("test.shop.{name}"), json!(["model.shop.orders"]));
            nodes.insert(format!("test.shop.{name}"), t);
        }
        build(json!({ "nodes": nodes, "parent_map": parents }))
    }

    fn generic_test(name: &str, attached: &str, generic: &str, kwargs: Value) -> Value {
        let namespace = if generic == "unique_combination_of_columns" { json!("dbt_utils") } else { Value::Null };
        json!({
            "name": name, "resource_type": "test", "package_name": "shop",
            "original_file_path": "models/schema.yml", "fqn": ["shop", name], "attached_node": attached,
            "test_metadata": { "name": generic, "namespace": namespace, "kwargs": kwargs },
        })
    }

    #[test]
    fn a_key_with_a_column_no_not_null_covers_is_listed_with_that_column() {
        let g = keyed(
            json!({ "materialized": "table" }),
            vec![
                ("u_id", "unique", Some("id"), json!({})),
                ("u_code", "unique", Some("code"), json!({})),
                ("nn_code", "not_null", Some("code"), json!({})),
                ("combo", "unique_combination_of_columns", None, json!({ "combination_of_columns": ["order_id", "line"] })),
                ("nn_order", "not_null", Some("order_id"), json!({})),
            ],
        );
        let rows = checks(&g).keys_maybe_null;
        assert_eq!(rows.total, 2);
        let seen: Vec<(&str, &str)> = rows.rows.iter().map(|r| (r.detail.as_str(), r.column.as_str())).collect();
        assert_eq!(seen, [("key id: no not_null on id", "id"), ("key order_id, line: no not_null on line", "line")]);
        assert_eq!(rows.rows[1].key, ["order_id", "line"]);
        // The count the index reads is the rows', built without any of them.
        assert_eq!(count(&g), checks(&g).keys_maybe_null.total + checks(&g).models_without_key.total + checks(&g).models_without_test.total + checks(&g).models_without_description.total + checks(&g).unique_keys_untested.total);
        // Both rows are the same model, and neither is a model with no key.
        assert!(checks(&g).models_without_key.rows.iter().all(|r| r.name == "lonely"));
    }

    #[test]
    fn a_unique_key_of_an_incremental_model_wants_a_uniqueness_test_of_its_columns() {
        let incremental = |key: Value| json!({ "materialized": "incremental", "unique_key": key });
        let g = keyed(incremental(json!(["id", "line"])), vec![("u", "unique", Some("id"), json!({}))]);
        let c = checks(&g);
        assert_eq!(names(&c.unique_keys_untested.rows), ["orders"]);
        assert_eq!(c.unique_keys_untested.rows[0].detail, "unique_key id, line");
        // The test of the whole list, in another order, checks it.
        let g = keyed(incremental(json!(["id", "line"])), vec![("c", "unique_combination_of_columns", None, json!({ "combination_of_columns": ["line", "id"] }))]);
        assert!(checks(&g).unique_keys_untested.rows.is_empty());
        // A table with a unique_key is not held to it: it does not merge.
        let g = keyed(json!({ "materialized": "table", "unique_key": "id" }), vec![]);
        assert!(checks(&g).unique_keys_untested.rows.is_empty());
    }

    #[test]
    fn a_model_with_no_uniqueness_test_has_no_key_even_with_a_unique_key() {
        let g = keyed(json!({ "materialized": "incremental", "unique_key": "id" }), vec![("nn", "not_null", Some("id"), json!({}))]);
        assert_eq!(names(&checks(&g).models_without_key.rows), ["lonely", "orders"]);
        let g = keyed(json!({ "materialized": "table" }), vec![("u", "unique", Some("id"), json!({}))]);
        assert_eq!(names(&checks(&g).models_without_key.rows), ["lonely"]);
    }

    #[test]
    fn the_key_rules_leave_out_a_package_and_a_disabled_model() {
        let g = build(json!({
            "nodes": { "model.kit.helper": model("helper", "kit", "x") },
            "disabled": { "model.shop.old": [{
                "name": "old", "resource_type": "model", "package_name": "shop",
                "original_file_path": "models/old.sql", "fqn": ["shop", "old"], "config": { "enabled": false },
            }] },
        }));
        assert!(checks(&g).models_without_key.rows.is_empty());
    }

    #[test]
    fn a_unique_key_dbt_ignores_is_not_checked_unless_a_test_declares_it() {
        let append = json!({ "materialized": "incremental", "incremental_strategy": "append", "unique_key": "id" });
        let g = keyed(append.clone(), vec![]);
        let c = checks(&g);
        assert!(c.unique_keys_untested.rows.is_empty() && c.keys_maybe_null.rows.is_empty());
        let total = |c: &Checks| {
            c.sources_without_freshness.total + c.models_without_test.total + c.models_without_description.total + c.sources_not_read.total
                + c.keys_maybe_null.total + c.unique_keys_untested.total + c.models_without_key.total
        };
        assert_eq!(count(&g), total(&c), "the count leaves it out too");
        // A test of the same column makes it a key the checks look at.
        let g = keyed(append, vec![("u", "unique", Some("id"), json!({}))]);
        assert_eq!(checks(&g).keys_maybe_null.total, 1);
        let merge = json!({ "materialized": "incremental", "incremental_strategy": "merge", "unique_key": "id" });
        assert_eq!(checks(&keyed(merge, vec![])).unique_keys_untested.total, 1);
    }
}
