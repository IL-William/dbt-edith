//! Named selectors, as the manifest records them from `selectors.yml`, and the
//! tree dbt resolves each one to.
//!
//! The manifest stores every selector already parsed, so nothing here reads
//! YAML (0032). It arrives in more than one shape. dbt-core writes `exclude`
//! as a list and keeps each criterion's `indirect_selection`; 1.13 keeps a
//! `method: selector` as a reference by name, where 1.11 writes the referenced
//! definition in place. dbt Fusion matches 1.13 since a fix of 2026-08-18.
//! Older Fusion writes references in place, an exclusion as
//! `{"exclude": <one expression>}` inside an `intersection`, and drops
//! `indirect_selection`. That last loss changes which tests a selector keeps,
//! so it is detected and said, never guessed back.
//!
//! The invariant: a selector is resolved whole or refused whole, with the
//! reason. Dropping a criterion this build cannot read would draw a set that
//! looks right and is not. `selectors.yml` itself is opened only to look for
//! one word, and nothing read from it leaves this module.

use crate::graph::{Graph, Kind};
use crate::select::{self, Bound, Depth, Indirect, Resolved, SelectError, Spec, Tests};
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// How deep one definition may nest, and how many selectors one may reach
/// through. Far past any real `selectors.yml`, and a bound on the recursion
/// whatever a manifest holds.
const MAX_DEPTH: usize = 64;

/// Methods that compare with the artifacts of another run, which dbt takes
/// from `--state` and this tool never reads.
const STATEFUL: &[&str] = &["state", "result", "source_status"];

#[derive(Clone, Debug)]
pub struct Named {
    pub name: String,
    pub description: String,
    /// dbt runs this one when a command names no selection.
    pub default: bool,
    spec: Option<Spec>,
    /// Why it cannot be resolved here, when it cannot.
    pub unsupported: Option<String>,
}

#[derive(Clone, Default)]
pub struct Selectors {
    /// Sorted by name: the manifest's map arrives in no order worth keeping.
    list: Vec<Named>,
    by_name: HashMap<String, u32>,
    /// Some criterion carried `indirect_selection`, so the manifest keeps them.
    carries_indirect: bool,
    /// The manifest carries none while `selectors.yml` sets some.
    lost_indirect: bool,
}

/// Python's truthiness, which is how dbt reads `parents: true` and its kin.
fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}

/// dbt's `_match_to_int`: absent is no limit, and a number may arrive as
/// text, which is how Fusion writes every depth.
fn depth_of(map: Option<&Map<String, Value>>, key: &str) -> Result<Depth, String> {
    let bad = || format!("`{key}` must be a whole number");
    match map.and_then(|m| m.get(key)) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(n)) => n.as_u64().and_then(|n| u32::try_from(n).ok()).map(Some).ok_or_else(bad),
        Some(Value::String(s)) => s.trim().parse::<u32>().map(Some).map_err(|_| bad()),
        Some(_) => Err(bad()),
    }
}

/// `cli.py`'s `parse_from_definition`, over the tree the manifest stored.
struct Parser {
    carries_indirect: bool,
}

impl Parser {
    fn definition(&mut self, v: &Value, depth: usize) -> Result<Spec, String> {
        if depth > MAX_DEPTH {
            return Err(format!("its definition nests deeper than {MAX_DEPTH} levels"));
        }
        match v {
            Value::String(s) => select::term(s)
                .map(|term| Spec::Criteria { term, indirect: Indirect::Eager })
                .map_err(|e| e.to_string()),
            Value::Object(map) => {
                if let Some(items) = map.get("union") {
                    return self.group(items, true, depth);
                }
                if let Some(items) = map.get("intersection") {
                    return self.group(items, false, depth);
                }
                if let Some(method) = map.get("method") {
                    let Value::String(method) = method else {
                        return Err("a criterion's `method` must be text".to_string());
                    };
                    let Some(value) = map.get("value") else {
                        return Err(format!("the `{method}` criterion has no value"));
                    };
                    return self.criterion(method, value, Some(map), depth);
                }
                // `{tag: nightly}` is dbt's shorthand for a method and a value.
                if map.len() == 1 {
                    let (key, value) = map.iter().next().expect("one entry");
                    if key != "exclude" {
                        return self.criterion(key, value, None, depth);
                    }
                }
                Err("expected a method and a value, a union or an intersection".to_string())
            }
            _ => Err("expected a method and a value, a union or an intersection".to_string()),
        }
    }

    /// A union or an intersection, where one item may be the exclusion taken
    /// from the whole group. An item with a method of its own and an `exclude`
    /// is a criterion with its own exclusion, as Fusion writes one.
    fn group(&mut self, items: &Value, union: bool, depth: usize) -> Result<Spec, String> {
        let Value::Array(items) = items else {
            return Err(format!("`{}` must be a list", if union { "union" } else { "intersection" }));
        };
        let mut parts: Vec<Spec> = Vec::with_capacity(items.len());
        let mut exclusion: Option<Spec> = None;
        for item in items {
            if let Value::Object(m) = item {
                if let (Some(ex), false) = (m.get("exclude"), m.contains_key("method")) {
                    if exclusion.is_some() {
                        return Err("two `exclude` blocks at one level, which dbt refuses too".to_string());
                    }
                    exclusion = Some(self.exclusion(ex, depth + 1)?);
                    continue;
                }
            }
            parts.push(self.definition(item, depth + 1)?);
        }
        let group = if union { Spec::Union(parts) } else { Spec::Intersection(parts) };
        Ok(match exclusion {
            Some(ex) => Spec::Difference(Box::new(group), Box::new(ex)),
            None => group,
        })
    }

    /// dbt-core writes an exclusion as a list and older Fusion as one
    /// expression; both mean the union of what they hold.
    fn exclusion(&mut self, v: &Value, depth: usize) -> Result<Spec, String> {
        match v {
            Value::Array(items) => {
                let mut parts = items.iter().map(|i| self.definition(i, depth + 1)).collect::<Result<Vec<_>, _>>()?;
                Ok(if parts.len() == 1 { parts.remove(0) } else { Spec::Union(parts) })
            }
            other => self.definition(other, depth + 1),
        }
    }

    fn criterion(&mut self, method: &str, value: &Value, map: Option<&Map<String, Value>>, depth: usize) -> Result<Spec, String> {
        let value = match value {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            _ => return Err(format!("the `{method}` criterion needs a text value")),
        };
        let flag = |key: &str| map.and_then(|m| m.get(key)).is_some_and(truthy);
        // Both depths are read whether or not their flag is set, as dbt reads
        // them, so a bad one is refused rather than ignored.
        let (up, down) = (depth_of(map, "parents_depth")?, depth_of(map, "children_depth")?);
        let at = flag("childrens_parents");
        let parents = flag("parents").then_some(up);
        let children = flag("children").then_some(down);
        if at && children.is_some() {
            return Err(format!("`@` and a trailing `+` on `{method}:{value}`, which dbt refuses too"));
        }
        let indirect = match map.and_then(|m| m.get("indirect_selection")) {
            None | Some(Value::Null) => Indirect::Eager,
            Some(v) => {
                self.carries_indirect = true;
                match v.as_str().and_then(Indirect::by_name) {
                    Some(mode) => mode,
                    None => return Err(format!("indirect_selection {v} is not eager, cautious, buildable or empty")),
                }
            }
        };
        let spec = if method == "selector" {
            Spec::Ref { name: value, at, parents, children }
        } else if STATEFUL.contains(&method) {
            return Err(format!(
                "`{method}:` compares with the artifacts of another run, which dbt takes from --state \
                 and this tool does not read"
            ));
        } else {
            let term = select::criterion(method, &value, at, parents, children).map_err(|e| e.to_string())?;
            Spec::Criteria { term, indirect }
        };
        match map.and_then(|m| m.get("exclude")) {
            Some(ex) => Ok(Spec::Difference(Box::new(spec), Box::new(self.exclusion(ex, depth + 1)?))),
            None => Ok(spec),
        }
    }
}

fn refs_of(spec: &Spec, out: &mut Vec<String>) {
    match spec {
        Spec::Ref { name, .. } => out.push(name.clone()),
        Spec::Union(parts) | Spec::Intersection(parts) => parts.iter().for_each(|p| refs_of(p, out)),
        Spec::Difference(a, b) => {
            refs_of(a, out);
            refs_of(b, out);
        }
        Spec::Criteria { .. } => {}
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Mark {
    New,
    Open,
    Done,
}

/// Every selector a reference cannot reach is refused before anything is
/// resolved: one naming nothing, one on a cycle, and one leaning on either,
/// so the menu can say so before a click does.
struct Refs<'a> {
    refs: Vec<Vec<String>>,
    by_name: &'a HashMap<String, u32>,
    mark: Vec<Mark>,
    path: Vec<usize>,
    reasons: Vec<Option<String>>,
}

impl Refs<'_> {
    fn visit(&mut self, i: usize, list: &[Named]) {
        self.mark[i] = Mark::Open;
        self.path.push(i);
        for r in self.refs[i].clone() {
            if self.reasons[i].is_some() {
                break;
            }
            let Some(&j) = self.by_name.get(&r) else {
                self.reasons[i] = Some(format!("it refers to `{r}`, which no selector defines"));
                break;
            };
            let j = j as usize;
            match self.mark[j] {
                Mark::Open => {
                    let from = self.path.iter().position(|&k| k == j).unwrap_or(0);
                    let mut chain: Vec<&str> = self.path[from..].iter().map(|&k| list[k].name.as_str()).collect();
                    chain.push(&list[j].name);
                    let reason = format!("it refers back to itself through {}", chain.join(" → "));
                    for &k in &self.path[from..] {
                        self.reasons[k].get_or_insert_with(|| reason.clone());
                    }
                }
                Mark::New if self.path.len() >= MAX_DEPTH => {
                    self.reasons[i] = Some(format!("it reaches through more than {MAX_DEPTH} selectors"));
                }
                Mark::New => self.visit(j, list),
                Mark::Done => {}
            }
            if self.reasons[i].is_none() && self.reasons[j].is_some() {
                self.reasons[i] = Some(format!("it refers to `{r}`, which cannot be resolved here either"));
            }
        }
        self.path.pop();
        self.mark[i] = Mark::Done;
    }
}

impl Selectors {
    pub fn build(raw: Value) -> Selectors {
        let Value::Object(entries) = raw else { return Selectors::default() };
        let mut parser = Parser { carries_indirect: false };
        let mut list: Vec<Named> = entries
            .into_iter()
            .map(|(key, entry)| {
                let text = |k: &str| entry.get(k).and_then(Value::as_str).unwrap_or("").to_string();
                let name = Some(text("name")).filter(|n| !n.is_empty()).unwrap_or(key);
                let parsed = match entry.get("definition") {
                    Some(d) if !d.is_null() => parser.definition(d, 0),
                    _ => Err("it has no definition".to_string()),
                };
                let (spec, unsupported) = match parsed {
                    Ok(spec) => (Some(spec), None),
                    Err(reason) => (None, Some(reason)),
                };
                Named {
                    name,
                    description: text("description"),
                    default: entry.get("default").and_then(Value::as_bool).unwrap_or(false),
                    spec,
                    unsupported,
                }
            })
            .collect();
        list.sort_by(|a, b| a.name.cmp(&b.name));
        let by_name: HashMap<String, u32> = list.iter().enumerate().map(|(i, n)| (n.name.clone(), i as u32)).collect();

        let refs: Vec<Vec<String>> = list
            .iter()
            .map(|n| {
                let mut out = Vec::new();
                if let Some(spec) = &n.spec {
                    refs_of(spec, &mut out);
                }
                out
            })
            .collect();
        let mut walk = Refs {
            refs,
            by_name: &by_name,
            mark: vec![Mark::New; list.len()],
            path: Vec::new(),
            reasons: list.iter().map(|n| n.unsupported.clone()).collect(),
        };
        for i in 0..list.len() {
            if walk.mark[i] == Mark::New {
                walk.visit(i, &list);
            }
        }
        for (named, reason) in list.iter_mut().zip(walk.reasons) {
            if reason.is_some() {
                named.spec = None;
                named.unsupported = reason;
            }
        }

        Selectors { list, by_name, carries_indirect: parser.carries_indirect, lost_indirect: false }
    }

    pub fn list(&self) -> &[Named] {
        &self.list
    }

    pub fn lost_indirect(&self) -> bool {
        self.lost_indirect
    }

    /// Records whether the manifest lost the `indirect_selection` that
    /// `selectors.yml` sets. Only a yes or a no comes back out of the file.
    pub fn check_against(&mut self, root: &Path) {
        self.lost_indirect = !self.list.is_empty() && !self.carries_indirect && mentions_indirect(root);
    }

    fn spec_of(&self, name: &str) -> Option<&Spec> {
        self.by_name.get(name).and_then(|&i| self.list[i as usize].spec.as_ref())
    }

    pub fn get(&self, name: &str) -> Option<&Named> {
        self.by_name.get(name).map(|&i| &self.list[i as usize])
    }

    /// dbt's answer for one selector. Tests are always in the universe here:
    /// a selector's definition decides which ones belong, and the tests box
    /// only decides how much of that answer is drawn (`shown`).
    pub fn resolve(&self, graph: &Graph, name: &str) -> Result<Resolved, SelectError> {
        let Some(named) = self.get(name) else {
            return Err(SelectError::UnknownSelector { name: name.to_string() });
        };
        match (&named.spec, &named.unsupported) {
            (Some(spec), None) => select::resolve_spec(graph, spec, Tests::Eager, &|n: &str| self.spec_of(n)),
            (_, reason) => Err(SelectError::UnsupportedSelector {
                name: name.to_string(),
                reason: reason.clone().unwrap_or_else(|| "it has no definition".to_string()),
            }),
        }
    }
}

impl Selectors {
    /// Whether the `indirect_selection` a manifest lost could change this
    /// selector's answer. It is resolved at both ends of the range, every
    /// criterion bringing the fewest tests it could and the most; when the two
    /// agree, no setting in between changes a thing, and a note saying it
    /// might would only be noise. A selector of tests alone is the usual case.
    pub fn depends_on_indirect(&self, graph: &Graph, name: &str) -> bool {
        let Some(Named { spec: Some(spec), .. }) = self.get(name) else { return false };
        let low = select::resolve_bounded(graph, spec, Tests::Eager, &|n: &str| self.spec_of(n), Bound::Low);
        let high = select::resolve_bounded(graph, spec, Tests::Eager, &|n: &str| self.spec_of(n), Bound::High);
        match (low, high) {
            (Ok(low), Ok(high)) => low.nodes != high.nodes,
            // Unreachable for a selector that resolved; saying so is the safe side.
            _ => true,
        }
    }
}

/// What a named selector's answer puts on the canvas.
#[derive(Debug, Default, PartialEq)]
pub struct Shown {
    pub drawn: Vec<u32>,
    /// Drawn dimmed: a model a drawn test belongs to that the selector itself
    /// did not select, there so the test hangs off something.
    pub context: Vec<u32>,
    /// Selected tests left off the canvas because the tests box is off.
    pub hidden_tests: usize,
}

/// The tests box as a filter on an answer that is already dbt's: off, the
/// tests go; on, they stay with the models they belong to beside them.
pub fn shown(graph: &Graph, picked: &[u32], with_tests: bool) -> Shown {
    let is_test = |i: u32| graph.nodes[i as usize].kind == Kind::Test;
    if !with_tests {
        let drawn: Vec<u32> = picked.iter().copied().filter(|&i| !is_test(i)).collect();
        let hidden_tests = picked.len() - drawn.len();
        return Shown { drawn, context: Vec::new(), hidden_tests };
    }
    let chosen: HashSet<u32> = picked.iter().copied().collect();
    let mut context: Vec<u32> = picked
        .iter()
        .copied()
        .filter(|&i| is_test(i))
        .flat_map(|t| graph.nodes[t as usize].parents.iter().copied())
        .filter(|p| {
            let n = &graph.nodes[*p as usize];
            !chosen.contains(p) && !n.disabled && n.kind.is_graph_node()
        })
        .collect();
    context.sort_unstable();
    context.dedup();
    Shown { drawn: picked.to_vec(), context, hidden_tests: 0 }
}

/// Whether `selectors.yml` sets `indirect_selection` outside a comment. The
/// file is read for that one yes or no and nothing else.
pub fn mentions_indirect(root: &Path) -> bool {
    let path = root.join("selectors.yml");
    let Ok(meta) = std::fs::metadata(&path) else { return false };
    if !meta.is_file() || meta.len() > crate::project::MAX_BYTES {
        return false;
    }
    let Ok(bytes) = std::fs::read(&path) else { return false };
    crate::envs::decode(&bytes)
        .lines()
        .any(|line| line.split('#').next().unwrap_or("").contains("indirect_selection"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::RawManifest;
    use serde_json::json;

    /// Two sources, two staging models, and two marts: `orders` reads both
    /// staging models, `customers` only one of them, so `customers` is never
    /// an ancestor of `orders`. Five tests hang off them, three of them with
    /// two parents each, and a disabled model shares the tests' tag.
    fn graph(selectors: Value) -> Graph {
        let model = |name: &str| json!({ "name": name, "resource_type": "model", "package_name": "shop", "fqn": ["shop", name] });
        let test = |name: &str, tags: Value| {
            json!({ "name": name, "resource_type": "test", "package_name": "shop", "fqn": ["shop", name], "tags": tags })
        };
        let source = |table: &str| {
            json!({ "name": table, "resource_type": "source", "package_name": "shop", "source_name": "crm", "fqn": ["shop", "crm", table] })
        };
        let raw: RawManifest = serde_json::from_value(json!({
            "nodes": {
                "model.shop.stg_orders": model("stg_orders"),
                "model.shop.stg_customers": model("stg_customers"),
                "model.shop.orders": model("orders"),
                "model.shop.customers": model("customers"),
                "test.shop.not_null_orders_id": test("not_null_orders_id", json!([])),
                "test.shop.rel_orders_customers": test("rel_orders_customers", json!([])),
                "test.shop.rel_orders_stg_customers": test("rel_orders_stg_customers", json!([])),
                "test.shop.recon_orders_totals": test("recon_orders_totals", json!(["recon"])),
                "test.shop.recon_customers_count": test("recon_customers_count", json!(["recon"])),
            },
            "sources": {
                "source.shop.crm.orders_raw": source("orders_raw"),
                "source.shop.crm.customers_raw": source("customers_raw"),
            },
            "disabled": {
                "model.shop.legacy": [{
                    "name": "legacy", "resource_type": "model", "package_name": "shop",
                    "fqn": ["shop", "legacy"], "tags": ["recon"], "config": { "enabled": false },
                }],
            },
            "parent_map": {
                "model.shop.stg_orders": ["source.shop.crm.orders_raw"],
                "model.shop.stg_customers": ["source.shop.crm.customers_raw"],
                "model.shop.orders": ["model.shop.stg_orders", "model.shop.stg_customers"],
                "model.shop.customers": ["model.shop.stg_customers"],
                "test.shop.not_null_orders_id": ["model.shop.orders"],
                "test.shop.rel_orders_customers": ["model.shop.orders", "model.shop.customers"],
                "test.shop.rel_orders_stg_customers": ["model.shop.orders", "model.shop.stg_customers"],
                "test.shop.recon_orders_totals": ["model.shop.orders", "model.shop.customers"],
                "test.shop.recon_customers_count": ["model.shop.customers"],
            },
            "selectors": selectors,
        }))
        .unwrap();
        Graph::build(raw, Path::new("manifest.json"), 0, 0)
    }

    /// A selector map from name and definition pairs, the dbt-core way.
    fn named(defs: &[(&str, Value)]) -> Value {
        Value::Object(defs.iter().map(|(n, d)| (n.to_string(), json!({ "name": n, "definition": d }))).collect())
    }

    fn names(g: &Graph, nodes: &[u32]) -> Vec<String> {
        let mut out: Vec<String> = nodes.iter().map(|&i| g.nodes[i as usize].name.clone()).collect();
        out.sort();
        out
    }

    fn pick(g: &Graph, name: &str) -> Vec<String> {
        names(g, &g.selectors.resolve(g, name).expect(name).nodes)
    }

    fn reason(g: &Graph, name: &str) -> String {
        g.selectors.get(name).and_then(|n| n.unsupported.clone()).unwrap_or_default()
    }

    #[test]
    fn each_indirect_mode_keeps_the_tests_dbt_keeps() {
        let with = |mode: &str| json!({ "method": "fqn", "value": "orders", "indirect_selection": mode });
        let g = graph(named(&[
            ("eager", with("eager")),
            ("cautious", with("cautious")),
            ("buildable", with("buildable")),
            ("empty", with("empty")),
        ]));
        assert_eq!(
            pick(&g, "eager"),
            ["not_null_orders_id", "orders", "recon_orders_totals", "rel_orders_customers", "rel_orders_stg_customers"]
        );
        // Every parent selected: only the test on `orders` alone.
        assert_eq!(pick(&g, "cautious"), ["not_null_orders_id", "orders"]);
        // An ancestor counts, so `stg_customers` does; `customers` never does.
        assert_eq!(pick(&g, "buildable"), ["not_null_orders_id", "orders", "rel_orders_stg_customers"]);
        assert_eq!(pick(&g, "empty"), ["orders"]);
    }

    #[test]
    fn a_reference_answers_what_the_selector_it_names_answers() {
        let recon_tests = json!({ "intersection": [
            { "method": "tag", "value": "recon" },
            { "method": "resource_type", "value": "test" },
        ]});
        let customers = json!({ "method": "fqn", "value": "customers", "parents": true, "indirect_selection": "buildable" });
        let g = graph(named(&[
            ("recon_tests", recon_tests.clone()),
            ("by_ref", json!({ "intersection": [customers.clone(), { "method": "selector", "value": "recon_tests" }] })),
            ("inlined", json!({ "intersection": [customers, recon_tests] })),
        ]));
        assert_eq!(pick(&g, "recon_tests"), ["recon_customers_count", "recon_orders_totals"]);
        // Buildable keeps the test whose inputs all sit upstream of `customers`,
        // and leaves the one that also reads `orders` to a selector of its own.
        assert_eq!(pick(&g, "by_ref"), ["recon_customers_count"]);
        assert_eq!(pick(&g, "inlined"), pick(&g, "by_ref"));
    }

    #[test]
    fn a_manifest_that_lost_indirect_selection_answers_eagerly() {
        // The older Fusion shape of the selector above: the reference written
        // in place, the exclusion one expression, and no indirect_selection.
        let g = graph(named(&[(
            "fusion",
            json!({ "intersection": [
                { "method": "fqn", "value": "customers", "parents": true },
                { "intersection": [{ "method": "tag", "value": "recon" }, { "method": "resource_type", "value": "test" }] },
            ]}),
        )]));
        assert_eq!(pick(&g, "fusion"), ["recon_customers_count", "recon_orders_totals"]);
        assert!(!g.selectors.carries_indirect);
    }

    #[test]
    fn a_lost_mode_matters_only_where_it_could_change_the_answer() {
        let recon = json!({ "method": "tag", "value": "recon" });
        let tests = json!({ "method": "resource_type", "value": "test" });
        let g = graph(named(&[
            // Tests picked by name bring none of their own: no mode changes it.
            ("tests_only", json!({ "intersection": [recon.clone(), tests.clone()] })),
            ("tests_minus_recon", json!({ "union": [tests.clone(), { "exclude": [recon.clone()] }] })),
            // A model brings its tests, as many as the mode lets through.
            ("recon_near_customers", json!({ "intersection": [
                { "method": "fqn", "value": "customers", "parents": true }, recon,
            ]})),
            // Under an exclusion the order turns round: more tests in the half
            // taken away leave fewer in the answer, and that is caught too.
            ("orders_minus_customers", json!({ "union": [
                { "method": "fqn", "value": "orders" },
                { "exclude": [{ "method": "fqn", "value": "customers" }] },
            ]})),
            ("by_reference", json!({ "method": "selector", "value": "recon_near_customers" })),
        ]));
        let depends = |name: &str| g.selectors.depends_on_indirect(&g, name);
        assert!(!depends("tests_only"));
        assert!(!depends("tests_minus_recon"));
        assert!(depends("recon_near_customers"));
        assert!(depends("orders_minus_customers"));
        assert!(depends("by_reference"), "through a reference as well");
        assert!(!depends("absent"));
        // And the answer itself is still the eager one, whatever was asked.
        assert_eq!(pick(&g, "recon_near_customers"), ["recon_customers_count", "recon_orders_totals"]);
    }

    #[test]
    fn both_shapes_of_an_exclusion_subtract_the_same_thing() {
        let orders = json!({ "method": "fqn", "value": "orders" });
        let not_null = json!({ "method": "fqn", "value": "not_null_orders_id" });
        let g = graph(named(&[
            ("core", json!({ "union": [orders.clone(), { "exclude": [not_null.clone()] }] })),
            ("fusion", json!({ "intersection": [{ "union": [orders.clone()] }, { "exclude": { "union": [not_null.clone()] } }] })),
            ("atom", json!({ "method": "fqn", "value": "orders", "exclude": [not_null] })),
        ]));
        let want = ["orders", "recon_orders_totals", "rel_orders_customers", "rel_orders_stg_customers"];
        assert_eq!(pick(&g, "core"), want, "a test excluded by name stays out");
        assert_eq!(pick(&g, "fusion"), want);
        assert_eq!(pick(&g, "atom"), want);
    }

    #[test]
    fn a_selector_that_cannot_resolve_is_refused_whole_with_its_reason() {
        let recon = json!({ "method": "tag", "value": "recon" });
        let g = graph(named(&[
            ("a", json!({ "method": "selector", "value": "b" })),
            ("b", json!({ "union": [recon.clone(), { "method": "selector", "value": "a" }] })),
            ("lost", json!({ "method": "selector", "value": "nowhere" })),
            ("ci", json!({ "union": [{ "method": "state", "value": "modified", "children": true, "children_depth": "1" }] })),
            ("after_ci", json!({ "intersection": [recon.clone(), { "method": "selector", "value": "ci" }] })),
            ("two_excludes", json!({ "union": [recon.clone(), { "exclude": [recon.clone()] }, { "exclude": [recon.clone()] }] })),
            ("bad_depth", json!({ "method": "tag", "value": "recon", "parents": true, "parents_depth": "x" })),
            ("bad_mode", json!({ "method": "tag", "value": "recon", "indirect_selection": "sometimes" })),
            ("at_and_plus", json!({ "method": "tag", "value": "recon", "childrens_parents": true, "children": true })),
            ("unknown", json!({ "method": "wat", "value": "x" })),
            ("nothing", Value::Null),
            ("fine", recon),
        ]));
        assert!(reason(&g, "a").contains("refers back to itself"), "{}", reason(&g, "a"));
        assert!(reason(&g, "b").contains("refers back to itself"), "{}", reason(&g, "b"));
        assert!(reason(&g, "lost").contains("`nowhere`"));
        assert!(reason(&g, "ci").contains("--state"));
        assert!(reason(&g, "after_ci").contains("`ci`"));
        assert!(reason(&g, "two_excludes").contains("two `exclude`"));
        assert!(reason(&g, "bad_depth").contains("whole number"));
        assert!(reason(&g, "bad_mode").contains("sometimes"));
        assert!(reason(&g, "at_and_plus").contains("`@`"));
        assert!(reason(&g, "unknown").contains("unknown selector method `wat`"));
        assert!(reason(&g, "nothing").contains("no definition"));
        assert!(reason(&g, "fine").is_empty());

        let refused = g.selectors.resolve(&g, "a").unwrap_err();
        assert_eq!(refused.code(), "unsupported_selector");
        assert_eq!(g.selectors.resolve(&g, "absent").unwrap_err().code(), "unknown_selector");
    }

    #[test]
    fn a_depth_reads_as_a_number_or_as_text_and_shorthand_is_a_criterion() {
        let down = |d: Value| json!({ "method": "fqn", "value": "stg_customers", "children": true, "children_depth": d, "indirect_selection": "empty" });
        let g = graph(named(&[
            ("text", down(json!("1"))),
            ("number", down(json!(1))),
            ("short", json!({ "tag": "recon" })),
            ("string", json!({ "union": ["tag:recon", "customers"] })),
        ]));
        assert_eq!(pick(&g, "text"), ["customers", "orders", "rel_orders_stg_customers", "stg_customers"]);
        assert_eq!(pick(&g, "number"), pick(&g, "text"));
        // The disabled model carries the tag too, and is never returned.
        assert_eq!(pick(&g, "short"), ["recon_customers_count", "recon_orders_totals"]);
        assert!(pick(&g, "string").contains(&"customers".to_string()));
    }

    #[test]
    fn the_list_is_sorted_and_carries_what_the_menu_shows() {
        let g = graph(json!({
            "zeta": { "name": "zeta", "definition": { "tag": "recon" } },
            "alpha": { "name": "alpha", "description": "Every mart.", "default": true, "definition": { "fqn": "orders" } },
        }));
        let list = g.selectors.list();
        assert_eq!(list.iter().map(|n| n.name.as_str()).collect::<Vec<_>>(), ["alpha", "zeta"]);
        assert_eq!((list[0].description.as_str(), list[0].default), ("Every mart.", true));
        assert!(!list[1].default);
        // Anything but a map is no selectors at all, never a failed manifest.
        assert!(Selectors::build(json!([1, 2])).list().is_empty());
        assert!(Selectors::build(Value::Null).list().is_empty());
    }

    #[test]
    fn the_tests_box_filters_the_drawing_and_never_the_answer() {
        let g = graph(named(&[(
            "recon",
            json!({ "intersection": [{ "method": "tag", "value": "recon" }, { "method": "resource_type", "value": "test" }] }),
        )]));
        let picked = g.selectors.resolve(&g, "recon").unwrap().nodes;
        let on = shown(&g, &picked, true);
        assert_eq!(names(&g, &on.drawn), ["recon_customers_count", "recon_orders_totals"]);
        assert_eq!(names(&g, &on.context), ["customers", "orders"], "the models the tests belong to");
        let off = shown(&g, &picked, false);
        assert!(off.drawn.is_empty());
        assert_eq!(off.hidden_tests, 2);

        // A model the selector picked is drawn as itself, never as context.
        let mixed: Vec<u32> = ["model.shop.orders", "test.shop.not_null_orders_id"].iter().map(|id| g.index[*id]).collect();
        let on = shown(&g, &mixed, true);
        assert!(on.context.is_empty());
        assert_eq!(shown(&g, &mixed, false).drawn, [g.index["model.shop.orders"]]);
    }

    #[test]
    fn a_lost_indirect_selection_is_noticed_only_when_the_file_sets_one() {
        let dir = std::env::temp_dir().join(format!("dbt-edith-selectors-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let lost = || {
            let mut g = graph(named(&[("s", json!({ "tag": "recon" }))]));
            g.selectors.check_against(&dir);
            g.selectors.lost_indirect()
        };
        assert!(!lost(), "no file");
        std::fs::write(dir.join("selectors.yml"), "selectors:\n  # indirect_selection: buildable, one day\n").unwrap();
        assert!(!lost(), "a comment sets nothing");
        std::fs::write(dir.join("selectors.yml"), "selectors:\n  - indirect_selection: buildable\n").unwrap();
        assert!(lost());

        // A manifest that kept them has lost nothing, whatever the file says.
        let mut kept = graph(named(&[("s", json!({ "method": "tag", "value": "recon", "indirect_selection": "empty" }))]));
        kept.selectors.check_against(&dir);
        assert!(!kept.selectors.lost_indirect());
        // Nor has a manifest with no selectors in it.
        let mut none = graph(Value::Null);
        none.selectors.check_against(&dir);
        assert!(!none.selectors.lost_indirect());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
