//! The config paths of `dbt_project.yml` that apply to no resource, worked
//! out from the manifest without running dbt, as dbt finds them when it
//! parses and warns of them: `UnusedResourceConfigPath`, I051 in dbt-core,
//! dbt1097 in Fusion (0103).
//!
//! dbt's rule is `_get_config_paths` and `warn_for_unused_resource_config_paths`
//! in dbt-core's `config/runtime.py`, which Fusion agrees with. Under each
//! block, a key holding a mapping is a path segment unless it starts with `+`
//! (0039), and a path is recorded once the mapping it names holds anything
//! else: a `+` key, or a value that is no mapping. A path is used when it
//! begins the fqn of a resource of its block's kind, or of any disabled node,
//! whatever its kind.
//!
//! Invariant: what is used is the manifest's answer, never the disk's. A
//! folder that exists and holds no node the manifest knows is as unused to
//! dbt as one that does not exist, which is what tells this apart from the
//! dashed key of 0036. The file is read whole by `src/yaml.rs`, or refused
//! at a line with that reader's fixed message.

use crate::manifest::RawManifest;
use crate::project::Unparsed;
use crate::yaml::{self, Node, Value};
use std::path::Path;

/// The blocks dbt checks, in its order (`get_resource_config_paths`), each
/// with the resource type whose fqns can use its paths.
const BLOCKS: [(&str, &str); 11] = [
    ("models", "model"),
    ("seeds", "seed"),
    ("snapshots", "snapshot"),
    ("sources", "source"),
    ("data_tests", "test"),
    ("unit_tests", "unit_test"),
    ("metrics", "metric"),
    ("semantic_models", "semantic_model"),
    ("saved_queries", "saved_query"),
    ("exposures", "exposure"),
    ("functions", "function"),
];

/// Between the parts of a joined fqn. No folder or node name holds it, so a
/// path ending `v1` cannot be taken for the start of a folder `v1.2`, as a
/// dot would let it.
const SEP: char = '\u{1f}';

/// Every fqn the manifest holds, joined and sorted, so that whether a path
/// begins one is two binary searches.
pub struct Fqns {
    /// One list per entry of `BLOCKS`.
    by_block: Vec<Vec<Box<str>>>,
    /// Every disabled node's, of any kind: dbt counts them for every block.
    disabled: Vec<Box<str>>,
}

fn joined(parts: &[String]) -> Box<str> {
    parts.join(&SEP.to_string()).into_boxed_str()
}

impl Fqns {
    /// Read off the manifest before the graph consumes it. A node of the
    /// `nodes` section counts for its own kind whatever its `enabled` says,
    /// since that section is what dbt-core reads them from.
    pub fn of(raw: &RawManifest) -> Fqns {
        let mut by_block: Vec<Vec<Box<str>>> = vec![Vec::new(); BLOCKS.len()];
        let mut put = |kind: &str, fqn: &[String]| {
            if let Some(i) = BLOCKS.iter().position(|(_, k)| *k == kind) {
                by_block[i].push(joined(fqn));
            }
        };
        for n in raw.nodes.values().chain(raw.sources.values()).chain(raw.exposures.values()) {
            put(&n.resource_type, &n.fqn);
        }
        for u in raw.unit_tests.values() {
            put("unit_test", &u.fqn);
        }
        for (kind, section) in [
            ("metric", &raw.metrics),
            ("semantic_model", &raw.semantic_models),
            ("saved_query", &raw.saved_queries),
            ("function", &raw.functions),
        ] {
            for f in section.values() {
                put(kind, &f.fqn);
            }
        }
        let mut disabled: Vec<Box<str>> = raw.disabled.values().flatten().map(|n| joined(&n.fqn)).collect();
        for list in by_block.iter_mut().chain(std::iter::once(&mut disabled)) {
            list.sort_unstable();
            list.dedup();
        }
        Fqns { by_block, disabled }
    }

    /// Whether `chain` begins an fqn of block `b`'s kind or a disabled one.
    /// The empty chain, the block itself, begins any.
    fn used(&self, b: usize, chain: &[String]) -> bool {
        let lists = [&self.by_block[b], &self.disabled];
        if chain.is_empty() {
            return lists.iter().any(|l| !l.is_empty());
        }
        let exact = joined(chain);
        let below = format!("{exact}{SEP}");
        lists.iter().any(|l| {
            l.binary_search(&exact).is_ok() || l.get(l.partition_point(|f| **f < *below)).is_some_and(|f| f.starts_with(&below))
        })
    }
}

/// One path no resource's fqn begins.
#[derive(serde::Serialize, Debug, Clone, PartialEq)]
pub struct Unused {
    /// The block as dbt names it in its warning: `data_tests` for a block
    /// written `tests:`, which dbt reads under that name.
    pub block: &'static str,
    /// The block as written, when that is not its name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub written: Option<&'static str>,
    /// The keys below the block, empty for the block itself.
    pub chain: Vec<String>,
    /// The line of the last key, or of the alias that brings it in, which
    /// is where the path is written; the block's when the chain is empty.
    pub line: usize,
}

/// What `dbt_project.yml` holds that dbt would warn of, against one manifest.
#[derive(serde::Serialize, Debug, Default)]
pub struct Report {
    /// When the file was last written, in seconds.
    pub at: u64,
    pub manifest_at: u64,
    /// In the order of the file.
    pub unused: Vec<Unused>,
    /// How many paths were checked, unused or not.
    pub checked: usize,
    /// Where the file stopped being read, when it was refused.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unread: Option<Unparsed>,
}

/// The paths of one block, with the line of the key each one ends on. A key
/// under the block is a segment when it holds a mapping and does not start
/// with `+`; anything else records the path of the mapping it sits in, once,
/// as dbt keeps them in a set.
fn paths_of(map: &Node, chain: &mut Vec<String>, out: &mut Vec<(Vec<String>, usize)>) {
    let Value::Map(entries) = &map.value else { return };
    let mut sets = false;
    for (key, value) in entries {
        if matches!(value.value, Value::Map(_)) && !key.starts_with('+') {
            chain.push(key.clone());
            paths_of(value, chain, out);
            chain.pop();
        } else {
            sets = true;
        }
    }
    if sets {
        out.push((chain.clone(), map.key_line));
    }
}

/// The unused paths of a project file already read into a tree. dbt reads a
/// `tests:` block as `data_tests:` when there is no `data_tests:`, and leaves
/// it out when there is one.
pub fn unused_in(doc: &Node, fqns: &Fqns) -> (Vec<Unused>, usize) {
    let mut unused = Vec::new();
    let mut checked = 0;
    for (b, (name, _)) in BLOCKS.iter().enumerate() {
        let (written, node) = match doc.get(name) {
            Some(n) => (None, n),
            None if *name == "data_tests" => match doc.get("tests") {
                Some(n) => (Some("tests"), n),
                None => continue,
            },
            None => continue,
        };
        let mut paths = Vec::new();
        paths_of(node, &mut Vec::new(), &mut paths);
        checked += paths.len();
        for (chain, line) in paths {
            if !fqns.used(b, &chain) {
                unused.push(Unused { block: name, written, chain, line });
            }
        }
    }
    unused.sort_by_key(|u| u.line);
    (unused, checked)
}

/// The report for the project at `root`, or None when it has no
/// `dbt_project.yml` this can read at all.
pub fn read(root: &Path, fqns: &Fqns, manifest_at: u64) -> Option<Report> {
    let text = crate::project::project_text(root)?;
    let at = crate::files::mtime_secs(&root.join("dbt_project.yml"));
    let mut report = Report { at, manifest_at, ..Report::default() };
    match yaml::parse(&text) {
        Ok(doc) => (report.unused, report.checked) = unused_in(&doc, fqns),
        Err(u) => report.unread = Some(u),
    }
    Some(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fqns(manifest: serde_json::Value) -> Fqns {
        Fqns::of(&serde_json::from_value(manifest).unwrap())
    }

    fn node(kind: &str, fqn: &[&str]) -> serde_json::Value {
        serde_json::json!({ "name": fqn.last().unwrap(), "resource_type": kind, "fqn": fqn })
    }

    fn said(text: &str, f: &Fqns) -> Vec<String> {
        let (unused, _) = unused_in(&yaml::parse(text).unwrap(), f);
        unused.iter().map(|u| std::iter::once(u.block.to_string()).chain(u.chain.iter().cloned()).collect::<Vec<_>>().join(".")).collect()
    }

    /// The cases dbt-core 1.12 and Fusion 2.0 were both run on, each giving
    /// the same list (0103): the folder keys, the shapes that record a path,
    /// the renamed test block, and the disabled node that uses a path of
    /// another kind's block.
    #[test]
    fn a_path_is_unused_when_no_fqn_of_its_kind_begins_it() {
        let f = fqns(serde_json::json!({
            "nodes": {
                "model.shop.stg_orders": node("model", &["shop", "staging", "stg_orders"]),
                "model.shop.y2023": node("model", &["shop", "2023", "y2023"]),
                "model.shop.dim": node("model", &["shop", "marts", "dim", "v1"]),
                "seed.shop.raw": node("seed", &["shop", "raw"]),
                "test.shop.t": node("test", &["shop", "staging", "t"]),
            },
            "sources": { "source.shop.app.orders": node("source", &["shop", "app", "orders"]) },
        }));
        let text = "\
models:
  shop:
    +materialized: table
    staging:
      +tags: ['s']
      nope:
        materialized: view
    stagign: &shared
      +tags: ['typo']
    '2023':
      +tags: ['y']
    marts:
      dim:
        +tags: ['versioned']
      core:
        +tags: ['deep']
    meta:
      owner: me
    empty: {}
    nulled:
    merged:
      <<: *shared
  other_package:
    +tags: ['pkg']
seeds:
  +quote_columns: false
snapshots:
  +tags: ['snap']
sources:
  shop:
    app:
      +tags: ['src']
    ap:
      +tags: ['typo']
tests:
  shop:
    staging:
      +severity: warn
    nowhere:
      +severity: warn
";
        assert_eq!(
            said(text, &f),
            [
                "models.shop.staging.nope",
                "models.shop.stagign",
                "models.shop.marts.core",
                "models.shop.meta",
                "models.shop.merged",
                "models.other_package",
                "snapshots",
                "sources.shop.ap",
                "data_tests.shop.nowhere",
            ]
        );
        // One disabled node of any kind uses every block itself.
        let mut with_off = serde_json::json!({ "nodes": {}, "disabled": { "model.shop.old": [node("model", &["shop", "legacy", "old"])] } });
        with_off["nodes"]["seed.shop.raw"] = node("seed", &["shop", "raw"]);
        assert_eq!(said("snapshots:\n  +tags: ['x']\nseeds:\n  shop:\n    legacy:\n      +tags: ['y']\n", &fqns(with_off)), Vec::<String>::new());
    }

    #[test]
    fn each_path_says_where_its_key_is_written() {
        let f = Fqns::of(&RawManifest::default());
        let (unused, checked) = unused_in(
            &yaml::parse("name: shop\n\nmodels:\n  shop:\n    # a note\n    staging:\n\n      +tags: ['s']\n      +materialized: view\ntests:\n  +severity: warn\ndata_tests:\n  +severity: error\n").unwrap(),
            &f,
        );
        assert_eq!(checked, 2, "a path set twice is one path, and tests: beside data_tests: is not read");
        assert_eq!(unused[0], Unused { block: "models", written: None, chain: vec!["shop".into(), "staging".into()], line: 6 });
        assert_eq!(unused[1], Unused { block: "data_tests", written: None, chain: vec![], line: 12 });
        let (renamed, _) = unused_in(&yaml::parse("tests:\n  +severity: warn\n").unwrap(), &f);
        assert_eq!(renamed[0].written, Some("tests"));
    }

    /// Each kind the graph leaves out of its nodes still uses its block's
    /// paths: without its fqn, each of these would be called unused.
    #[test]
    fn every_kind_of_resource_uses_its_block() {
        let text = "\
exposures:\n  shop:\n    weekly:\n      +tags: [x]\n\
unit_tests:\n  shop:\n    orders:\n      +tags: [x]\n\
metrics:\n  shop:\n    revenue:\n      +tags: [x]\n\
semantic_models:\n  shop:\n    sales:\n      +tags: [x]\n\
saved_queries:\n  shop:\n    daily:\n      +tags: [x]\n\
functions:\n  shop:\n    to_cents:\n      +tags: [x]\n";
        let one = |fqn: &[&str]| serde_json::json!({ "fqn": fqn });
        let all = fqns(serde_json::json!({
            "exposures": { "exposure.shop.weekly": node("exposure", &["shop", "weekly"]) },
            "unit_tests": { "unit_test.shop.orders.t": { "name": "t", "fqn": ["shop", "orders", "t"] } },
            "metrics": { "metric.shop.revenue": one(&["shop", "revenue"]) },
            "semantic_models": { "semantic_model.shop.sales": one(&["shop", "sales"]) },
            "saved_queries": { "saved_query.shop.daily": one(&["shop", "daily"]) },
            "functions": { "function.shop.to_cents": one(&["shop", "to_cents"]) },
        }));
        assert_eq!(said(text, &all), Vec::<String>::new());
        assert_eq!(
            said(text, &fqns(serde_json::json!({}))),
            [
                "exposures.shop.weekly",
                "unit_tests.shop.orders",
                "metrics.shop.revenue",
                "semantic_models.shop.sales",
                "saved_queries.shop.daily",
                "functions.shop.to_cents",
            ]
        );
    }

    /// A path an alias brings in is reported where the alias is written, not
    /// at the anchor's keys, which are another path and may be used.
    #[test]
    fn a_path_through_an_alias_is_at_the_alias() {
        let f = fqns(serde_json::json!({ "nodes": { "model.shop.a": node("model", &["shop", "with_anchor", "inner", "a"]) } }));
        let (unused, _) = unused_in(
            &yaml::parse("models:\n  shop:\n    with_anchor: &anc\n      inner:\n        +tags: [x]\n    aliased: *anc\n").unwrap(),
            &f,
        );
        assert_eq!(unused, [Unused { block: "models", written: None, chain: vec!["shop".into(), "aliased".into(), "inner".into()], line: 6 }]);
    }

    /// A prefix is a whole part, never part of one, whatever the parts hold.
    #[test]
    fn a_path_begins_an_fqn_by_whole_parts() {
        let f = fqns(serde_json::json!({ "nodes": {
            "model.shop.a": node("model", &["shop", "v1.2", "a"]),
            "model.shop.b": node("model", &["shop", "stagingx", "b"]),
        }}));
        assert_eq!(said("models:\n  shop:\n    v1:\n      '2':\n        +tags: ['x']\n    staging:\n      +tags: ['x']\n    'v1.2':\n      +tags: ['x']\n", &f), ["models.shop.v1.2", "models.shop.staging"]);
    }
}
