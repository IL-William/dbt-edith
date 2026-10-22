//! The tool's own checks on the project, worked out from the manifest alone
//! (0116): four rules dbt itself warns of none of, listed under their own
//! names in Reports, apart from what dbt would warn of (0103).
//!
//! - a source with no freshness threshold, as dbt-core's `has_freshness`
//!   reads the manifest's;
//! - a model nothing tests: no generic test declared on it or one of its
//!   columns, no singular test selecting from it, no unit test of it;
//! - a model with no description;
//! - a source no node reads.
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
}

/// The nodes of each list, by index in the graph.
struct Found {
    no_freshness: Vec<u32>,
    no_test: Vec<u32>,
    no_description: Vec<u32>,
    not_read: Vec<u32>,
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

fn found(graph: &Graph) -> Found {
    let root = graph.macros.root();
    let tested = tested(graph);
    let mut f = Found { no_freshness: Vec::new(), no_test: Vec::new(), no_description: Vec::new(), not_read: Vec::new() };
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
                    f.no_freshness.push(i);
                }
                if n.children.is_empty() {
                    f.not_read.push(i);
                }
            }
            Kind::Model => {
                if !tested[i as usize] {
                    f.no_test.push(i);
                }
                if n.description.trim().is_empty() {
                    f.no_description.push(i);
                }
            }
            _ => {}
        }
    }
    f
}

/// How many nodes the four lists hold in all, without building a row: the
/// Reports index asks it whenever it is read.
pub fn count(graph: &Graph) -> usize {
    let f = found(graph);
    f.no_freshness.len() + f.no_test.len() + f.no_description.len() + f.not_read.len()
}

pub fn checks(graph: &Graph) -> Checks {
    let f = found(graph);
    let rows = |list: Vec<u32>| {
        let total = list.len();
        let mut rows: Vec<Row> = list
            .into_iter()
            .map(|i| {
                let n = &graph.nodes[i as usize];
                Row { id: n.id.clone(), name: n.name.clone(), file: n.file.clone(), folder: crate::jobs::folder(graph, i) }
            })
            .collect();
        rows.sort_by(|a, b| {
            a.folder.cmp(&b.folder).then_with(|| a.file.cmp(&b.file)).then_with(|| a.name.cmp(&b.name)).then_with(|| a.id.cmp(&b.id))
        });
        rows.truncate(MAX_ROWS);
        Check { total, rows }
    };
    Checks {
        sources_without_freshness: rows(f.no_freshness),
        models_without_test: rows(f.no_test),
        models_without_description: rows(f.no_description),
        sources_not_read: rows(f.not_read),
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
        // The index counts every node, not the rows sent.
        assert_eq!(count(&g), 2 * (MAX_ROWS + 5));
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
        assert_eq!(count(&g), 4);
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
}
