//! What a clone copies: a model, seed or snapshot and its direct parents.
//!
//! The statements themselves are written by the page, from the relations the
//! Catalog already resolves, and run by the user: nothing here reaches
//! Snowflake (0058). This module only decides which nodes they cover.
//!
//! Invariants: depth 1, except that an ephemeral parent has no object to copy,
//! so its own parents stand in for it, as far up as ephemerals go. A node met
//! both directly and through an ephemeral is a parent, once. Sources are
//! listed and never copied: they already exist wherever the developer works.
//! The order is the same on every start (0055): parents by name then id, and
//! the model last, since it reads from them.

use crate::graph::{Graph, Kind};
use std::collections::{HashSet, VecDeque};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Why {
    /// The node the clone was asked for.
    Model,
    /// One of its direct parents.
    Parent,
    /// Reached through this ephemeral model, which has no object of its own.
    Via(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Item {
    pub node: u32,
    pub why: Why,
}

#[derive(Debug, Default, PartialEq)]
pub struct Plan {
    /// Models, seeds and snapshots to copy, the model last.
    pub items: Vec<Item>,
    /// Sources the copied models read, listed and left where they are.
    pub sources: Vec<Item>,
    /// Ephemeral models walked through, the focus included when it is one.
    pub ephemeral: Vec<u32>,
    /// Anything else among the parents, which has nothing to copy.
    pub other: Vec<u32>,
}

/// The nodes a clone of `focus` covers.
pub fn plan(graph: &Graph, focus: u32) -> Plan {
    let node = |i: u32| &graph.nodes[i as usize];
    let ephemeral = |i: u32| node(i).kind == Kind::Model && node(i).materialized == "ephemeral";
    let by_name = |list: &[u32]| {
        let mut list = list.to_vec();
        list.sort_by(|a, b| node(*a).name.cmp(&node(*b).name).then_with(|| node(*a).id.cmp(&node(*b).id)));
        list
    };

    let mut out = Plan::default();
    // Membership only: never iterated into the answer.
    let mut seen: HashSet<u32> = HashSet::new();
    seen.insert(focus);
    let direct = by_name(&node(focus).parents);
    seen.extend(direct.iter().copied());
    let mut queue: VecDeque<Item> = direct.into_iter().map(|p| Item { node: p, why: Why::Parent }).collect();
    if ephemeral(focus) {
        out.ephemeral.push(focus);
    }
    while let Some(item) = queue.pop_front() {
        let n = node(item.node);
        if ephemeral(item.node) {
            out.ephemeral.push(item.node);
            for p in by_name(&n.parents) {
                if seen.insert(p) {
                    queue.push_back(Item { node: p, why: Why::Via(item.node) });
                }
            }
            continue;
        }
        match n.kind {
            Kind::Model | Kind::Seed | Kind::Snapshot => out.items.push(item),
            Kind::Source => out.sources.push(item),
            _ => out.other.push(item.node),
        }
    }
    let order = |a: &Item, b: &Item| node(a.node).name.cmp(&node(b.node).name).then_with(|| node(a.node).id.cmp(&node(b.node).id));
    out.items.sort_by(order);
    out.sources.sort_by(order);
    out.ephemeral = by_name(&out.ephemeral);
    out.other = by_name(&out.other);
    if !ephemeral(focus) {
        out.items.push(Item { node: focus, why: Why::Model });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::RawManifest;
    use std::path::Path;

    /// A node with invented names. Its id sorts apart from its name on
    /// purpose, so an order by id shows up as a wrong answer.
    fn node(kind: &str, name: &str, materialized: &str, parents: &[&str]) -> serde_json::Value {
        serde_json::json!({
            "name": name,
            "resource_type": kind,
            "package_name": "shop",
            "original_file_path": format!("models/{name}.sql"),
            "database": "analytics",
            "schema": "marts",
            "alias": name,
            "relation_name": if materialized == "ephemeral" { serde_json::Value::Null } else { format!("analytics.marts.{name}").into() },
            "config": { "materialized": materialized },
            "depends_on": { "nodes": parents },
        })
    }

    fn source(name: &str) -> serde_json::Value {
        serde_json::json!({
            "name": name, "resource_type": "source", "source_name": "crm", "package_name": "shop",
            "database": "raw", "schema": "crm", "identifier": name, "relation_name": format!("raw.crm.{name}"),
        })
    }

    /// `dim_customers` reads `stg_customers` both directly and through the
    /// ephemeral `int_customers`, which reads another ephemeral in turn.
    fn graph() -> Graph {
        let raw: RawManifest = serde_json::from_value(serde_json::json!({
            "nodes": {
                "model.shop.m1": node("model", "zz_orders", "incremental", &["source.shop.crm.orders"]),
                "model.shop.m2": node("model", "stg_customers", "view", &["source.shop.crm.customers"]),
                "model.shop.m3": node("model", "int_customers", "ephemeral", &["model.shop.m2", "model.shop.m4"]),
                "model.shop.m4": node("model", "int_regions", "ephemeral", &["seed.shop.s1", "source.shop.crm.regions"]),
                "model.shop.m5": node("model", "dim_customers", "table", &["model.shop.m3", "model.shop.m2", "model.shop.m1"]),
                "seed.shop.s1": node("seed", "country_codes", "seed", &[]),
                "snapshot.shop.n1": node("snapshot", "customers_snapshot", "snapshot", &["source.shop.crm.customers"]),
            },
            "sources": {
                "source.shop.crm.orders": source("orders"),
                "source.shop.crm.customers": source("customers"),
                "source.shop.crm.regions": source("regions"),
            },
        }))
        .unwrap();
        Graph::build(raw, Path::new("manifest.json"), 0, 0)
    }

    /// The plan as names, so a failure reads as the question it answers.
    fn names(g: &Graph, focus: &str) -> (Vec<String>, Vec<String>, Vec<String>) {
        let p = plan(g, g.index[focus]);
        let name = |i: u32| g.nodes[i as usize].name.clone();
        let item = |it: &Item| match it.why {
            Why::Model => format!("{} (model)", name(it.node)),
            Why::Parent => name(it.node),
            Why::Via(e) => format!("{} via {}", name(it.node), name(e)),
        };
        (
            p.items.iter().map(item).collect(),
            p.sources.iter().map(item).collect(),
            p.ephemeral.iter().map(|&i| name(i)).collect(),
        )
    }

    #[test]
    fn parents_by_name_the_model_last_and_ephemerals_walked_through() {
        let g = graph();
        let (items, sources, ephemeral) = names(&g, "model.shop.m5");
        assert_eq!(items, ["country_codes via int_regions", "stg_customers", "zz_orders", "dim_customers (model)"]);
        assert_eq!(sources, ["crm.regions via int_regions"]);
        assert_eq!(ephemeral, ["int_customers", "int_regions"]);
    }

    #[test]
    fn an_ephemeral_focus_copies_its_parents_and_not_itself() {
        let g = graph();
        let (items, sources, ephemeral) = names(&g, "model.shop.m3");
        assert_eq!(items, ["country_codes via int_regions", "stg_customers"]);
        assert_eq!(sources, ["crm.regions via int_regions"]);
        assert_eq!(ephemeral, ["int_customers", "int_regions"]);
    }

    #[test]
    fn a_seed_is_alone_and_a_snapshot_lists_what_it_reads() {
        let g = graph();
        assert_eq!(names(&g, "seed.shop.s1").0, ["country_codes (model)"]);
        let (items, sources, _) = names(&g, "snapshot.shop.n1");
        assert_eq!(items, ["customers_snapshot (model)"]);
        assert_eq!(sources, ["crm.customers"]);
    }

    #[test]
    fn every_build_of_one_manifest_plans_the_same() {
        let first = names(&graph(), "model.shop.m5");
        for _ in 0..8 {
            assert_eq!(names(&graph(), "model.shop.m5"), first);
        }
    }
}
