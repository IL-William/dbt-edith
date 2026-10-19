//! Who calls each macro: the nodes that reach it, themselves or through other
//! macros, and what dbt runs itself, read from the manifest (0105).
//!
//! dbt records each node's and each macro's own calls, `depends_on.macros`.
//! A model calling `round_amount`, which calls `cents`, lists the first and
//! not the second, so a count of the nodes listing a macro would call every
//! helper uncalled: the walk goes up through the macros that list one, to
//! the nodes that list any of them. A node's calls are those dbt rendered
//! when it parsed, which leaves out a branch on `execute`,
//! `is_incremental()` or `target.name`, and a `{% call %}` body, and dbt
//! Fusion records none of a hook's, so the node's own SQL and its hooks' are
//! read for the calls dbt did not record. A macro's are read from its
//! source, branches and all, and need no such reading.
//!
//! Some macros no node calls are run by dbt itself: `generate_schema_name`,
//! a materialization for the models of its name, a strategy macro for the
//! models naming the strategy, a macro of dbt's own the project overrides by
//! name, and whatever those and dbt's own macros call, the dispatch
//! variants dbt picked among them, which a second walk follows down.
//!
//! The invariant: no macro is ever called unused. One that no node reaches
//! has no reference found, and the likeliest reason it may still run,
//! because `dbt run-operation` and a call from another package leave no
//! trace in the manifest.

use crate::graph::{Kind, Node};
use crate::macros::Macros;
use std::cmp::Ordering;
use std::collections::{HashMap, VecDeque};
use std::sync::OnceLock;

/// The three macros dbt calls for every node to place it, never from a
/// node's own SQL.
const PLACING: &[&str] = &["generate_schema_name", "generate_alias_name", "generate_database_name"];

/// Adapters whose macros another's inherit, child then parent: dbt looks in
/// the parent's package after the adapter's own.
const PARENTS: &[(&str, &str)] = &[("redshift", "postgres"), ("databricks", "spark")];

/// How many callers a summary names before it counts the rest.
const CALLERS_NAMED: usize = 3;

#[derive(Clone, Default)]
pub struct MacroUse {
    /// By a macro's position in `Macros`: the enabled nodes listing it.
    users: Vec<Vec<u32>>,
    /// The enabled nodes whose SQL or hooks call it where dbt recorded no
    /// call: a branch it did not render when it parsed, or a hook under dbt
    /// Fusion.
    written: Vec<Vec<u32>>,
    /// The macros listing it, by package then name, so a walk meets them in
    /// the same order on every start (0055): positions follow no order.
    callers: Vec<Vec<u32>>,
    /// The macros it lists, in the same order.
    callees: Vec<Vec<u32>>,
    /// Whether some node reaches it, itself or through macros.
    reached: Vec<bool>,
    /// How many nodes there are, for the walk's own list of those it met.
    nodes: usize,
    /// How many enabled nodes name each strategy macro dbt looks up by name:
    /// `get_incremental_<s>_sql` for an incremental model's strategy,
    /// `snapshot_<s>_strategy` for a snapshot's.
    runs: HashMap<String, usize>,
    /// Per macro, the macro dbt runs itself that reaches it, itself when it
    /// is one. Worked out on first use, once the root project is named.
    from_dbt: OnceLock<Vec<Option<u32>>>,
    /// Per macro, what its card says, worked out once when first asked.
    summaries: Vec<OnceLock<Summary>>,
}

/// The nodes reaching one macro.
#[derive(Debug, Default, PartialEq)]
pub struct Usage {
    /// The nodes listing it themselves, by index.
    pub direct: Vec<u32>,
    /// The nodes reaching it only through other macros, each with the macro
    /// calling it on the shortest way up, by node index.
    pub through: Vec<(u32, u32)>,
    /// The nodes reaching it only where dbt recorded no call, in their SQL
    /// or hooks, each with the macro it calls on the way, if not this one.
    pub written: Vec<(u32, Option<u32>)>,
    /// The macros listing it.
    pub callers: Vec<u32>,
}

/// How `walk` met a node.
#[derive(Clone, Copy)]
enum How {
    Listed,
    Through(u32),
    Written(Option<u32>),
}

/// Why a macro no node reaches may still run: the likeliest reason, from the
/// shape of its name and what calls it, in the order a report groups them:
/// the three nothing explains first, then the rest, a `Run` no node names
/// among them though it is unexplained too (`Summary::unexplained`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Why {
    /// Nothing in the manifest calls it: `dbt run-operation`, or another
    /// package.
    None,
    /// Only macros no node reaches call it.
    Macros,
    /// A generic test, `test_x`, that no enabled test uses.
    Test,
    /// A variant `adapter.dispatch` did not pick for the adapter the manifest
    /// was parsed for: another adapter's, or a default the adapter's shadows.
    Dispatch,
    /// dbt runs it for the nodes that name it: a materialization, or a
    /// strategy macro, with how many there are (`Run`).
    Run,
    /// dbt calls it itself: `generate_schema_name` and its two siblings, a
    /// macro of dbt's own the project overrides by name, or one called from
    /// something dbt runs, its own macros included.
    Dbt,
}

/// What dbt runs a macro for: the models materialized as `name`, or the
/// incremental models or snapshots whose strategy is `name`.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct Run {
    /// `materialized`, `incremental_strategy` or `strategy`, the config key.
    pub key: &'static str,
    pub name: String,
    pub nodes: usize,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct KindCount {
    pub kind: &'static str,
    pub n: usize,
}

/// What a macro's card and its palette row say of who calls it (0105).
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize)]
pub struct Summary {
    /// The nodes reaching it, by kind, in the order a card names them.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub by: Vec<KindCount>,
    /// How many of them reach it only through other macros.
    #[serde(skip_serializing_if = "is_zero")]
    pub through: usize,
    /// How many of them call it only where dbt recorded no call.
    #[serde(skip_serializing_if = "is_zero")]
    pub written: usize,
    /// The first few macros calling it, as the project calls them;
    /// `callers_of` counts them all.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub callers: Vec<String>,
    #[serde(skip_serializing_if = "is_zero")]
    pub callers_of: usize,
    /// When no node reaches it: why it may still run.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub why: Option<Why>,
    /// What dbt runs it for, for a materialization or a strategy macro.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run: Option<Run>,
    /// The macro dbt runs that calls it, when it is not one itself.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub from: String,
}

impl Summary {
    /// Whether the macro is reached by no node and nothing explains it: the
    /// macros the Reports button is lit for.
    pub fn unexplained(&self) -> bool {
        match self.why {
            Some(Why::None | Why::Macros | Why::Test) => true,
            Some(Why::Run) => self.run.as_ref().is_some_and(|r| r.nodes == 0),
            _ => false,
        }
    }
}

fn is_zero(n: &usize) -> bool {
    *n == 0
}

/// The order a summary names the kinds in: what is built first, then what
/// checks it, then the rest.
const KIND_ORDER: &[Kind] = &[
    Kind::Model,
    Kind::Snapshot,
    Kind::Seed,
    Kind::Test,
    Kind::Analysis,
    Kind::Operation,
    Kind::Exposure,
    Kind::Source,
    Kind::Other,
];

/// Where a kind sorts in `KIND_ORDER`.
pub fn kind_rank(kind: Kind) -> usize {
    KIND_ORDER.iter().position(|&k| k == kind).unwrap_or(KIND_ORDER.len())
}

/// A macro as the root project's files call it: bare for its own, with its
/// package for any other's.
pub fn call_of(macros: &Macros, m: u32) -> String {
    let mac = macros.at(m);
    if mac.package == macros.root() { mac.name.clone() } else { format!("{}.{}", mac.package, mac.name) }
}

impl MacroUse {
    /// `listed` pairs each enabled node with the macro ids it lists, and
    /// `written` with the macros its SQL calls, by position; `calls` pairs
    /// each macro's id with the ids it lists; `runs` counts the nodes naming
    /// each strategy macro. An id no macro of `macros` has, and a macro
    /// calling itself, are passed over.
    pub fn build(
        macros: &Macros,
        nodes: usize,
        listed: Vec<(u32, Vec<String>)>,
        written: Vec<(u32, Vec<u32>)>,
        calls: Vec<(String, Vec<String>)>,
        runs: HashMap<String, usize>,
    ) -> MacroUse {
        let n = macros.len();
        let mut users: Vec<Vec<u32>> = vec![Vec::new(); n];
        let mut found: Vec<Vec<u32>> = vec![Vec::new(); n];
        let mut callers: Vec<Vec<u32>> = vec![Vec::new(); n];
        let mut callees: Vec<Vec<u32>> = vec![Vec::new(); n];
        for (node, ids) in listed {
            for m in ids.iter().filter_map(|id| macros.position(id)) {
                users[m as usize].push(node);
            }
        }
        for (node, positions) in written {
            for m in positions {
                found[m as usize].push(node);
            }
        }
        for (id, ids) in calls {
            let Some(caller) = macros.position(&id) else { continue };
            for callee in ids.iter().filter_map(|id| macros.position(id)).filter(|&c| c != caller) {
                callers[callee as usize].push(caller);
                callees[caller as usize].push(callee);
            }
        }
        for list in &mut users {
            list.sort_unstable();
            list.dedup();
        }
        // Written but listed too is listed: dbt rendered that call.
        for (list, listed) in found.iter_mut().zip(&users) {
            list.sort_unstable();
            list.dedup();
            list.retain(|node| listed.binary_search(node).is_err());
        }
        for list in callers.iter_mut().chain(callees.iter_mut()) {
            list.sort_unstable_by(|&a, &b| by_name(macros, a, b));
            list.dedup();
        }
        // Down from every macro a node calls, along what each one calls.
        let mut reached = vec![false; n];
        let mut stack: Vec<u32> =
            (0..n as u32).filter(|&m| !users[m as usize].is_empty() || !found[m as usize].is_empty()).collect();
        for &m in &stack {
            reached[m as usize] = true;
        }
        while let Some(m) = stack.pop() {
            for &c in &callees[m as usize] {
                if !reached[c as usize] {
                    reached[c as usize] = true;
                    stack.push(c);
                }
            }
        }
        let summaries = (0..n).map(|_| OnceLock::new()).collect();
        MacroUse { users, written: found, callers, callees, reached, nodes, runs, from_dbt: OnceLock::new(), summaries }
    }

    /// Whether some node reaches the macro at `m`.
    pub fn reached(&self, m: u32) -> bool {
        self.reached.get(m as usize).copied().unwrap_or(false)
    }

    /// Every node reaching the macro at `m`, once each, with how: listing
    /// it, through the macros calling it, breadth first so a node reaching it
    /// two ways is told through the nearer macro, and only then where dbt
    /// recorded no call, so a node reaching it both ways reads as recorded.
    fn walk(&self, m: u32, mut visit: impl FnMut(u32, How)) {
        let i = m as usize;
        if i >= self.users.len() {
            return;
        }
        let mut met = vec![false; self.nodes];
        let mut meet = |node: u32| !std::mem::replace(&mut met[node as usize], true);
        for &node in &self.users[i] {
            if meet(node) {
                visit(node, How::Listed);
            }
        }
        let mut seen = vec![false; self.users.len()];
        seen[i] = true;
        let mut up: Vec<(u32, u32)> = Vec::new();
        let mut queue: VecDeque<(u32, u32)> = VecDeque::new();
        for &c in &self.callers[i] {
            seen[c as usize] = true;
            queue.push_back((c, c));
        }
        while let Some((c, via)) = queue.pop_front() {
            up.push((c, via));
            for &node in &self.users[c as usize] {
                if meet(node) {
                    visit(node, How::Through(via));
                }
            }
            for &next in &self.callers[c as usize] {
                if !std::mem::replace(&mut seen[next as usize], true) {
                    queue.push_back((next, via));
                }
            }
        }
        for &node in &self.written[i] {
            if meet(node) {
                visit(node, How::Written(None));
            }
        }
        for (c, via) in up {
            for &node in &self.written[c as usize] {
                if meet(node) {
                    visit(node, How::Written(Some(via)));
                }
            }
        }
    }

    /// The nodes reaching the macro at `m`, as `walk` meets them.
    pub fn usage(&self, m: u32) -> Usage {
        let callers = self.callers.get(m as usize).cloned().unwrap_or_default();
        let mut usage = Usage { callers, ..Usage::default() };
        self.walk(m, |node, how| match how {
            How::Listed => usage.direct.push(node),
            How::Through(via) => usage.through.push((node, via)),
            How::Written(via) => usage.written.push((node, via)),
        });
        usage.through.sort_unstable();
        usage.written.sort_unstable();
        usage
    }

    /// What the card of the macro at `m` says, worked out once.
    pub fn summary(&self, macros: &Macros, nodes: &[Node], adapter: &str, m: u32) -> &Summary {
        self.summaries[m as usize].get_or_init(|| {
            let (counts, through, written) = self.tally(nodes, m);
            let by = KIND_ORDER
                .iter()
                .zip(counts)
                .filter(|&(_, n)| n > 0)
                .map(|(kind, n)| KindCount { kind: kind.as_str(), n })
                .collect();
            let (why, run, from) = if self.reached(m) {
                (None, None, String::new())
            } else {
                let (why, run, from) = self.why(macros, nodes, adapter, m);
                (Some(why), run, from.map(|f| call_of(macros, f)).unwrap_or_default())
            };
            let callers = &self.callers[m as usize];
            Summary {
                by,
                through,
                written,
                callers: callers.iter().take(CALLERS_NAMED).map(|&c| call_of(macros, c)).collect(),
                callers_of: callers.len(),
                why,
                run,
                from,
            }
        })
    }

    /// `usage`, counted rather than listed, which is all a card needs: the
    /// nodes reaching the macro at `m` by kind, in `KIND_ORDER`, how many
    /// only through other macros, and how many only where dbt recorded no
    /// call.
    fn tally(&self, nodes: &[Node], m: u32) -> ([usize; KIND_ORDER.len()], usize, usize) {
        let mut counts = [0; KIND_ORDER.len()];
        let (mut through, mut written) = (0, 0);
        self.walk(m, |node, how| {
            counts[kind_rank(nodes[node as usize].kind).min(KIND_ORDER.len() - 1)] += 1;
            match how {
                How::Listed => {}
                How::Through(_) => through += 1,
                How::Written(_) => written += 1,
            }
        });
        (counts, through, written)
    }

    /// Why the macro at `m` may still run, when no node reaches it, on a
    /// manifest parsed for `adapter`: the reason, what dbt runs it for, and
    /// the macro dbt runs that calls it.
    pub fn why(&self, macros: &Macros, nodes: &[Node], adapter: &str, m: u32) -> (Why, Option<Run>, Option<u32>) {
        let name = macros.at(m).name.as_str();
        if let Some(root) = self.from_dbt(macros, nodes, adapter)[m as usize] {
            if root != m {
                return (Why::Dbt, None, Some(root));
            }
            return match self.run(macros, nodes, adapter, m) {
                Some(run) => (Why::Run, Some(run), None),
                None => (Why::Dbt, None, None),
            };
        }
        if materialization(name).is_some() {
            return match self.run(macros, nodes, adapter, m) {
                Some(run) if materialization_wins(macros, adapter, m) => (Why::Run, Some(run), None),
                _ => (Why::Dispatch, None, None),
            };
        }
        if !self.callers[m as usize].is_empty() {
            return (Why::Macros, None, None);
        }
        if name.starts_with("test_") {
            return (Why::Test, None, None);
        }
        if variant(macros, name).is_some() {
            return (Why::Dispatch, None, None);
        }
        (Why::None, None, None)
    }

    /// What dbt runs the macro named `name` for, when it is a materialization
    /// or a strategy macro, with how many enabled nodes name it.
    fn run(&self, macros: &Macros, nodes: &[Node], adapter: &str, at: u32) -> Option<Run> {
        let mac = macros.at(at);
        let name = mac.name.as_str();
        if let Some((m, _)) = materialization(name) {
            let n = if materialization_wins(macros, adapter, at) { materialized(nodes, m) } else { 0 };
            return Some(Run { key: "materialized", name: m.to_string(), nodes: n });
        }
        // A snapshot's strategy may name another package's macro, kept
        // under that package's name (`Graph::build`).
        let key = if mac.package == macros.root() { name.to_string() } else { format!("{}.{name}", mac.package) };
        let count = self.runs.get(&key).copied()?;
        if let Some(s) = name.strip_prefix("get_incremental_").and_then(|r| r.strip_suffix("_sql")) {
            return Some(Run { key: "incremental_strategy", name: s.to_string(), nodes: count });
        }
        let s = name.strip_prefix("snapshot_").and_then(|r| r.strip_suffix("_strategy"))?;
        Some(Run { key: "strategy", name: s.to_string(), nodes: count })
    }

    /// Per macro, the macro dbt runs itself that reaches it: down from the
    /// project's own that dbt runs, then from dbt's and its adapter's.
    fn from_dbt(&self, macros: &Macros, nodes: &[Node], adapter: &str) -> &[Option<u32>] {
        self.from_dbt.get_or_init(|| {
            let n = macros.len() as u32;
            let mut order: Vec<u32> = (0..n).collect();
            order.sort_unstable_by(|&a, &b| by_name(macros, a, b));
            // dbt's and its adapter's own entry points: not the helpers it
            // keeps for projects to call, which run only when called.
            let entry = |mac: &crate::macros::Macro| internal_package(adapter, &mac.package) && !helper(&mac.path);
            // A helper those entry points call runs all the same, and a bare
            // call in dbt's own macros, `equals(`, reaches the project's
            // override first though `depends_on` names dbt's copy, so the
            // override is dbt's call. A call dbt writes as `dbt.x(`
            // (`dbt.string_literal(`, say) reaches dbt's own `x`, never an
            // override, yet a project macro of that name reads as called by
            // dbt. A method call `a.y(` in dbt's macros (`config.get(`,
            // `information_schema.replace(...)`) is recorded as a call of the
            // macro `y` dbt would resolve, the project's first, so a project
            // macro named like a method (`get`, `replace`, `split`) reads as
            // called by dbt. Both err toward explained.
            let mut called = vec![false; n as usize];
            let mut stack: Vec<u32> = order.iter().copied().filter(|&m| entry(macros.at(m))).collect();
            for &m in &stack {
                called[m as usize] = true;
            }
            while let Some(m) = stack.pop() {
                for &c in &self.callees[m as usize] {
                    if !std::mem::replace(&mut called[c as usize], true) {
                        stack.push(c);
                    }
                }
            }
            let dbts = |mac: &crate::macros::Macro| {
                internal_package(adapter, &mac.package)
                    && macros.position(&mac.id).is_some_and(|d| called[d as usize] || !helper(&mac.path))
            };
            let runs = |m: u32| self.run(macros, nodes, adapter, m).is_some_and(|r| r.nodes > 0);
            let theirs = |m: u32| {
                let mac = macros.at(m);
                if mac.package != macros.root() && !internal_package(adapter, &mac.package) {
                    // A package's materialization still runs for its models.
                    return materialization(&mac.name).is_some() && runs(m);
                }
                if mac.package != macros.root() {
                    return false;
                }
                // A materialization of dbt's own name is no override:
                // `materialization_wins` says whether it runs.
                let overrides = materialization(&mac.name).is_none() && macros.named(&mac.name).any(dbts);
                PLACING.contains(&mac.name.as_str()) || overrides || runs(m)
            };
            let mut from = vec![None; n as usize];
            let mut queue = VecDeque::new();
            let roots = order.iter().copied().filter(|&m| theirs(m));
            let own = order.iter().copied().filter(|&m| entry(macros.at(m)));
            for m in roots.chain(own) {
                if from[m as usize].is_none() {
                    from[m as usize] = Some(m);
                    queue.push_back(m);
                }
            }
            while let Some(m) = queue.pop_front() {
                for &c in &self.callees[m as usize] {
                    if from[c as usize].is_none() {
                        from[c as usize] = from[m as usize];
                        queue.push_back(c);
                    }
                }
            }
            from
        })
    }

    /// The root project's macros no node reaches, by name.
    pub fn unreached(&self, macros: &Macros) -> Vec<u32> {
        let mut out: Vec<u32> =
            (0..macros.len() as u32).filter(|&m| !self.reached(m) && macros.at(m).package == macros.root()).collect();
        out.sort_unstable_by(|&a, &b| by_name(macros, a, b));
        out
    }
}

/// Whether `package` holds dbt's own macros or the adapter's, a parent
/// adapter's included.
fn internal_package(adapter: &str, package: &str) -> bool {
    package == "dbt" || package.strip_prefix("dbt_").is_some_and(|a| a == adapter || parent(adapter, a))
}

fn parent(adapter: &str, of: &str) -> bool {
    PARENTS.iter().any(|&(child, p)| child == adapter && p == of)
}

/// How near to `adapter` a materialization written for `written_for` is:
/// its own, then a parent's, then `default`; None for another adapter's.
fn nearness(adapter: &str, written_for: &str) -> Option<u8> {
    if written_for == adapter {
        Some(0)
    } else if parent(adapter, written_for) {
        Some(1)
    } else {
        (written_for == "default").then_some(2)
    }
}

/// Whether the materialization macro at `at` is the one dbt runs on
/// `adapter`, as dbt-core 1.11 picks among `MaterializationCandidate`s: the
/// nearest the adapter, then the root project before a package before dbt's
/// own; and, `require_explicit_package_overrides_for_builtin_materializations`
/// being on by default, a package's that would win where dbt has one of that
/// materialization gives way to the best of dbt's and the project's. Packages
/// tied at the best rank all win.
fn materialization_wins(macros: &Macros, adapter: &str, at: u32) -> bool {
    let Some((m, _)) = materialization(&macros.at(at).name) else { return false };
    // Higher is better: (nearness, locality). Two packages can tie, and the
    // position is no tie-break: `named()` walks `by_package`, a HashMap, and
    // the positions themselves come from a HashMap (`Macros::build`), so
    // ranking on either would let a restart change which of them dbt "runs"
    // (0055). Do not `.find()` or `max_by_key` over `named()` for the same
    // reason. Every candidate tied at the best rank wins instead, which errs
    // toward "explained", as this feature must never call a macro unused.
    let mut candidates: Vec<((u8, u8), u32)> = Vec::new();
    let chain = std::iter::once(adapter)
        .chain(PARENTS.iter().filter(|&&(c, _)| c == adapter).map(|&(_, p)| p))
        .chain(std::iter::once("default"));
    for b in chain {
        for mac in macros.named(&format!("materialization_{m}_{b}")) {
            let (Some(near), Some(pos)) = (nearness(adapter, b), macros.position(&mac.id)) else { continue };
            let locality = if mac.package == macros.root() {
                2
            } else if internal_package(adapter, &mac.package) {
                0
            } else {
                1
            };
            candidates.push(((2 - near, locality), pos));
        }
    }
    let best = |list: &mut dyn Iterator<Item = &((u8, u8), u32)>| list.map(|c| c.0).max();
    let mut rank = best(&mut candidates.iter());
    if rank.is_some_and(|r| r.1 == 1) && candidates.iter().any(|c| c.0 .1 == 0) {
        rank = best(&mut candidates.iter().filter(|c| c.0 .1 != 1));
    }
    rank.is_some_and(|r| candidates.iter().any(|c| c.0 == r && c.1 == at))
}

/// A macro dbt keeps for projects to call, and runs only when one does: its
/// cross-database helpers and its generic tests, which a node calls and dbt
/// records where it does.
fn helper(path: &str) -> bool {
    ["macros/utils/", "tests/generic/", "macros/generic_test_sql/"].iter().any(|p| path.starts_with(p))
}

/// `materialization_<name>_<adapter>`, the name dbt gives a
/// `{% materialization name, adapter='adapter' %}` block, read back into its
/// two halves. dbt's adapter names hold no underscore.
pub fn materialization(name: &str) -> Option<(&str, &str)> {
    name.strip_prefix("materialization_")?.rsplit_once('_').filter(|(m, a)| !m.is_empty() && !a.is_empty())
}

/// How many enabled nodes are materialized as `name`, which a materialization
/// macro is run for, whichever of its variants dbt picks: tests carry `test`.
pub fn materialized(nodes: &[Node], name: &str) -> usize {
    nodes
        .iter()
        .filter(|n| !n.disabled && !matches!(n.kind, Kind::Operation | Kind::Source | Kind::Exposure))
        .filter(|n| n.materialized == name)
        .count()
}

/// The macro dbt looks up by name to run strategy `strategy`: an incremental
/// model's, `delete+insert` read as `delete_insert`, or a snapshot's.
pub fn strategy_macro(snapshot: bool, strategy: &str) -> String {
    if snapshot {
        format!("snapshot_{strategy}_strategy")
    } else {
        format!("get_incremental_{}_sql", strategy.replace('+', "_"))
    }
}

/// The calls a node's SQL writes inside its Jinja, `name(` or
/// `package.name(`, each once, its `{# #}` comments, quoted strings and
/// `{% raw %}` blocks left out. dbt records only what it rendered when it
/// parsed, so a call inside `{% if execute %}`, `is_incremental()`, a test of
/// `target.name` or a `{% call %}` body is found here alone.
pub fn jinja_calls(code: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = code;
    while let Some(at) = rest.find('{') {
        let close = match rest[at..].as_bytes().get(1) {
            Some(b'{') => "}}",
            Some(b'%') => "%}",
            Some(b'#') => "#}",
            _ => {
                rest = &rest[at + 1..];
                continue;
            }
        };
        let body = &rest[at + 2..];
        let Some(end) = body.find(close) else { break };
        rest = &body[end + 2..];
        let inner = body[..end].trim_matches(|c: char| c == '-' || c == '+' || c.is_whitespace());
        if close == "%}" && inner == "raw" {
            // Everything to `{% endraw %}` is text, Jinja written as an example.
            match endraw(rest) {
                Some(after) => rest = after,
                None => break,
            }
            continue;
        }
        if close != "#}" {
            calls_in(&body[..end], &mut out);
        }
    }
    out
}

/// What follows the `{% endraw %}` that closes a raw block, whitespace
/// control and all.
fn endraw(text: &str) -> Option<&str> {
    let mut rest = text;
    while let Some(at) = rest.find("{%") {
        let body = &rest[at + 2..];
        let end = body.find("%}")?;
        if body[..end].trim_matches(|c: char| c == '-' || c == '+' || c.is_whitespace()) == "endraw" {
            return Some(&body[end + 2..]);
        }
        rest = &body[end + 2..];
    }
    None
}

fn calls_in<'a>(body: &'a str, out: &mut Vec<&'a str>) {
    let b = body.as_bytes();
    let mut i = 0;
    // The two words before the current one, which say whether a name is a
    // test: `x is divisibleby(3)`, `x is not sameas(y)`.
    let (mut before, mut last): (&str, &str) = ("", "");
    // The last character that was not a space: a name after `.` is a method,
    // `'x'.upper()`, and after `|` a filter, `1.5 | round(2)`.
    let mut prev = b' ';
    while i < b.len() {
        let c = b[i];
        if c == b'\'' || c == b'"' {
            i += 1;
            while i < b.len() && b[i] != c {
                i += if b[i] == b'\\' { 2 } else { 1 };
            }
            i += 1;
            prev = c;
            (before, last) = (last, "");
        } else if c.is_ascii_alphabetic() || c == b'_' {
            let start = i;
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_' || b[i] == b'.') {
                i += 1;
            }
            let word = &body[start..i];
            let mut j = i;
            while j < b.len() && (b[j] == b' ' || b[j] == b'\t') {
                j += 1;
            }
            // A method of a value, `adapter.dispatch(` or `x.y.z(`, is left
            // for `resolve` to refuse; three names deep is no macro at all.
            let named = word.matches('.').count() <= 1 && !word.ends_with('.');
            let test = last == "is" || (last == "not" && before == "is");
            if b.get(j) == Some(&b'(') && named && prev != b'.' && prev != b'|' && !test && !out.contains(&word) {
                out.push(word);
            }
            prev = b[i - 1];
            (before, last) = (last, word);
        } else {
            if !c.is_ascii_whitespace() {
                prev = c;
                if !c.is_ascii_digit() {
                    (before, last) = (last, "");
                }
            }
            i += 1;
        }
    }
}

/// `<prefix>__<name>` beside a macro named `name`, or another variant of it,
/// read into the prefix and the name: the shape `adapter.dispatch` looks
/// for, in any package, since a project may define a variant for a package's
/// macro, or for one of dbt's own.
fn variant<'a>(macros: &Macros, name: &'a str) -> Option<(&'a str, &'a str)> {
    let (prefix, rest) = name.split_once("__")?;
    if prefix.is_empty() || rest.is_empty() {
        return None;
    }
    let found = macros.named(rest).next().is_some()
        || (prefix != "default" && macros.named(&format!("default__{rest}")).next().is_some());
    found.then_some((prefix, rest))
}

fn by_name(macros: &Macros, a: u32, b: u32) -> Ordering {
    let (x, y) = (macros.at(a), macros.at(b));
    (&x.package, &x.name).cmp(&(&y.package, &y.name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::Graph;
    use crate::manifest::RawManifest;
    use std::path::Path;

    /// A shop on duckdb whose models call `round_amount`, which calls
    /// `cents`, which dispatches to `duckdb__cents`; calls dbt never
    /// rendered; what dbt runs itself, written as dbt writes it, a dispatcher
    /// of dbt's listing the variant it picked; a chain no node reaches; a
    /// cycle; and what a run-operation calls.
    fn graph() -> Graph {
        let mac_at = |name: &str, package: &str, path: &str, calls: &[&str]| {
            serde_json::json!({ "name": name, "package_name": package, "original_file_path": path,
                "depends_on": { "macros": calls } })
        };
        let mac = |name: &str, package: &str, calls: &[&str]| mac_at(name, package, "macros/m.sql", calls);
        let node = |name: &str, kind: &str, calls: &[&str], config: serde_json::Value, code: &str| {
            serde_json::json!({ "name": name, "resource_type": kind, "package_name": "shop",
                "original_file_path": format!("models/{name}.sql"), "config": config, "raw_code": code,
                "depends_on": { "nodes": [], "macros": calls } })
        };
        let table = || serde_json::json!({ "materialized": "table" });
        let macros: serde_json::Map<String, serde_json::Value> = [
            ("macro.shop.cents", mac("cents", "shop", &["macro.shop.duckdb__cents"])),
            ("macro.shop.default__cents", mac("default__cents", "shop", &[])),
            ("macro.shop.duckdb__cents", mac("duckdb__cents", "shop", &[])),
            ("macro.shop.round_amount", mac("round_amount", "shop", &["macro.shop.cents", "macro.shop.round_amount"])),
            ("macro.shop.watermark", mac("watermark", "shop", &[])),
            ("macro.shop.in_call", mac("in_call", "shop", &[])),
            ("macro.shop.hook_helper", mac("hook_helper", "shop", &[])),
            ("macro.shop.pre_helper", mac("pre_helper", "shop", &[])),
            ("macro.shop.orphan", mac("orphan", "shop", &["macro.shop.round_amount", "macro.shop.helper"])),
            ("macro.shop.helper", mac("helper", "shop", &[])),
            ("macro.shop.ping", mac("ping", "shop", &["macro.shop.pong"])),
            ("macro.shop.pong", mac("pong", "shop", &["macro.shop.ping"])),
            ("macro.shop.grant_select", mac("grant_select", "shop", &["macro.dbt.run_query"])),
            ("macro.shop.log_start", mac("log_start", "shop", &[])),
            ("macro.shop.test_is_positive", mac("test_is_positive", "shop", &[])),
            ("macro.shop.test_unused", mac("test_unused", "shop", &[])),
            ("macro.shop.generate_schema_name", mac("generate_schema_name", "shop", &["macro.shop.schema_helper"])),
            ("macro.shop.schema_helper", mac("schema_helper", "shop", &[])),
            ("macro.shop.materialization_table_default",
                mac("materialization_table_default", "shop", &["macro.shop.mat_helper"])),
            ("macro.shop.mat_helper", mac("mat_helper", "shop", &[])),
            ("macro.shop.materialization_table_postgres", mac("materialization_table_postgres", "shop", &[])),
            ("macro.shop.materialization_test_default", mac("materialization_test_default", "shop", &[])),
            ("macro.shop.materialization_fancy_default", mac("materialization_fancy_default", "shop", &[])),
            ("macro.shop.get_incremental_custom_sql", mac("get_incremental_custom_sql", "shop", &[])),
            ("macro.shop.snapshot_mine_strategy", mac("snapshot_mine_strategy", "shop", &[])),
            ("macro.shop.snapshot_other_strategy", mac("snapshot_other_strategy", "shop", &[])),
            ("macro.shop.create_table_as", mac("create_table_as", "shop", &[])),
            ("macro.shop.list_relations_without_caching", mac("list_relations_without_caching", "shop", &[])),
            ("macro.shop.duckdb__list_relations", mac("duckdb__list_relations", "shop", &[])),
            ("macro.shop.duckdb__set_query_tag", mac("duckdb__set_query_tag", "shop", &[])),
            ("macro.shop.postgres__set_query_tag", mac("postgres__set_query_tag", "shop", &[])),
            ("macro.shop.snowflake__star", mac("snowflake__star", "shop", &[])),
            ("macro.dbt_utils.star", mac("star", "dbt_utils", &[])),
            ("macro.dbt.run_query", mac("run_query", "dbt", &[])),
            ("macro.dbt.create_table_as", mac("create_table_as", "dbt", &[])),
            ("macro.dbt.list_relations_without_caching", mac("list_relations_without_caching", "dbt", &[])),
            ("macro.dbt.set_query_tag", mac("set_query_tag", "dbt", &["macro.shop.duckdb__set_query_tag"])),
            ("macro.dbt_duckdb.duckdb__list_relations", mac("duckdb__list_relations", "dbt_duckdb", &[])),
            ("macro.shop.snapshot_custom2_strategy", mac("snapshot_custom2_strategy", "shop", &[])),
            ("macro.shop.one_hook", mac("one_hook", "shop", &[])),
            // dbt's cross-database helpers and generic tests run only when
            // called, and a project's override of one says nothing more.
            ("macro.dbt.datediff", mac_at("datediff", "dbt", "macros/utils/datediff.sql", &[])),
            ("macro.shop.datediff", mac("datediff", "shop", &[])),
            ("macro.dbt.dateadd",
                mac_at("dateadd", "dbt", "macros/utils/dateadd.sql", &["macro.shop.duckdb__dateadd"])),
            ("macro.shop.duckdb__dateadd", mac("duckdb__dateadd", "shop", &[])),
            ("macro.dbt.test_accepted_values",
                mac_at("test_accepted_values", "dbt", "tests/generic/builtin.sql", &[])),
            // But a helper dbt's own macros call bare runs, and the bare call
            // reaches the project's override first: `equals` in a snapshot.
            ("macro.dbt.default__snapshot_staging_table", {
                let path = "macros/snapshots/helpers.sql";
                mac_at("default__snapshot_staging_table", "dbt", path, &["macro.dbt.equals"])
            }),
            ("macro.dbt.equals", mac_at("equals", "dbt", "macros/utils/equals.sql", &[])),
            ("macro.shop.equals", mac("equals", "shop", &[])),
            ("macro.shop.test_accepted_values", mac("test_accepted_values", "shop", &[])),
            // The adapter's own view beats the project's default one, which
            // shares its name with dbt's own default.
            ("macro.dbt_duckdb.materialization_view_duckdb", mac("materialization_view_duckdb", "dbt_duckdb", &[])),
            ("macro.dbt.materialization_view_default",
                mac_at("materialization_view_default", "dbt", "macros/materializations/models/view.sql", &[])),
            ("macro.shop.materialization_view_default",
                mac("materialization_view_default", "shop", &["macro.shop.view_helper"])),
            ("macro.shop.view_helper", mac("view_helper", "shop", &[])),
            // A package's materialization runs for its models, and so does
            // what it calls.
            ("macro.kit.materialization_fancy2_default",
                mac("materialization_fancy2_default", "kit", &["macro.shop.fancy_helper"])),
            ("macro.shop.fancy_helper", mac("fancy_helper", "shop", &[])),
            ("macro.shop.nulls", serde_json::json!({ "name": "nulls", "package_name": "shop", "depends_on": null })),
            ("macro.shop.null_list",
                serde_json::json!({ "name": "null_list", "package_name": "shop", "depends_on": { "macros": null } })),
        ]
        .into_iter()
        .map(|(id, m)| (id.to_string(), m))
        .collect();
        let raw: RawManifest = serde_json::from_value(serde_json::json!({
            "metadata": { "project_name": "shop", "adapter_type": "duckdb" },
            "nodes": {
                "model.shop.orders": node("orders", "model", &["macro.shop.cents"], table(), ""),
                "model.shop.stg_payments": node("stg_payments", "model", &["macro.shop.round_amount"], table(),
                    "{% if is_incremental() %}{{ cents('amount') }}{% endif %}"),
                "model.shop.customers":
                    node("customers", "model", &["macro.shop.round_amount", "macro.shop.cents"], table(), ""),
                "model.shop.off": { "name": "off", "resource_type": "model", "package_name": "shop",
                    "config": { "enabled": false }, "depends_on": { "macros": ["macro.shop.grant_select"] } },
                "model.shop.inc": node("inc", "model", &[],
                    serde_json::json!({ "materialized": "incremental", "incremental_strategy": "custom" }),
                    "select 1 {% if is_incremental() %} where id > {{ watermark() }} {% endif %}\n\
                     {# {{ ghost() }} #} {{ \"quoted()\" }} {{ a.b.c() }}"),
                "model.shop.c": node("c", "model", &[],
                    serde_json::json!({ "materialized": "view", "post-hook": [{ "sql": "{{ hook_helper() }}" }],
                        "pre-hook": ["{{ pre_helper() }}"] }),
                    "{% call statement('x') %} select {{ in_call() }} {% endcall %} select 1"),
                "snapshot.shop.snap": node("snap", "snapshot", &[],
                    serde_json::json!({ "materialized": "snapshot", "strategy": "mine" }), ""),
                "snapshot.shop.snap2": node("snap2", "snapshot", &[],
                    serde_json::json!({ "materialized": "snapshot", "strategy": "shop.custom2",
                        "post-hook": "{{ one_hook() }}" }), ""),
                "model.shop.v": node("v", "model", &[], serde_json::json!({ "materialized": "view" }), ""),
                "model.shop.fancy2": node("fancy2", "model", &[], serde_json::json!({ "materialized": "fancy2" }), ""),
                "test.shop.positive": node("positive", "test", &["macro.shop.test_is_positive", "macro.nowhere.x"],
                    serde_json::json!({ "materialized": "test" }), ""),
                "operation.shop.shop-on-run-start-0":
                    node("shop-on-run-start-0", "operation", &["macro.shop.log_start"], table(), ""),
            },
            "macros": macros,
        }))
        .unwrap();
        Graph::build(raw, Path::new("manifest.json"), 0, 0)
    }

    fn at(g: &Graph, id: &str) -> u32 {
        g.macros.position(id).unwrap_or_else(|| panic!("no {id}"))
    }

    fn names(g: &Graph, nodes: &[u32]) -> Vec<String> {
        nodes.iter().map(|&i| g.nodes[i as usize].name.clone()).collect()
    }

    fn macro_names(g: &Graph, list: &[u32]) -> Vec<String> {
        list.iter().map(|&m| g.macros.at(m).name.clone()).collect()
    }

    fn summary<'a>(g: &'a Graph, id: &str) -> &'a Summary {
        g.macro_use.summary(&g.macros, &g.nodes, &g.meta.adapter, at(g, id))
    }

    /// The reason, what dbt runs it for, and the macro dbt runs it from.
    fn why(g: &Graph, id: &str) -> (Why, Option<(String, usize)>, String) {
        let s = summary(g, id);
        let run = s.run.as_ref().map(|r| (r.key.to_string() + " " + &r.name, r.nodes));
        (s.why.unwrap_or_else(|| panic!("{id} is reached")), run, s.from.clone())
    }

    #[test]
    fn a_macro_is_used_by_the_nodes_listing_it_and_by_those_reaching_it_through_macros() {
        let g = graph();
        let u = g.macro_use.usage(at(&g, "macro.shop.cents"));
        assert_eq!(names(&g, &u.direct), ["customers", "orders"]);
        // stg_payments lists round_amount alone, which calls cents; customers
        // lists both, so it is counted once, as listing cents itself.
        let through: Vec<(String, String)> = u
            .through
            .iter()
            .map(|&(n, via)| (g.nodes[n as usize].name.clone(), g.macros.at(via).name.clone()))
            .collect();
        // stg_payments also calls cents in an `is_incremental()` branch: what
        // dbt recorded, through round_amount, comes first.
        assert_eq!(through, [("stg_payments".to_string(), "round_amount".to_string())]);
        assert!(u.written.is_empty());
        assert_eq!(macro_names(&g, &u.callers), ["round_amount"]);
        let s = summary(&g, "macro.shop.cents");
        assert_eq!(s.by, [KindCount { kind: "model", n: 3 }]);
        assert_eq!((s.through, s.written, s.callers_of, s.why), (1, 0, 1, None));
        assert_eq!(s.callers, ["round_amount"]);
    }

    #[test]
    fn a_dispatched_variant_is_reached_through_its_dispatcher_and_the_other_is_not() {
        let g = graph();
        let u = g.macro_use.usage(at(&g, "macro.shop.duckdb__cents"));
        assert!(u.direct.is_empty());
        assert_eq!(u.through.len(), 3, "orders and customers through cents, stg_payments two macros up");
        assert!(u.through.iter().all(|&(_, via)| g.macros.at(via).name == "cents"), "told through the nearer macro");
        assert_eq!(why(&g, "macro.shop.default__cents").0, Why::Dispatch);
    }

    #[test]
    fn a_call_dbt_did_not_render_is_found_in_the_sql() {
        let g = graph();
        // Inside `{% if is_incremental() %}`, in a `{% call %}` body, and in
        // hooks, which dbt Fusion does not record, as `{ sql }`, a string in a
        // list, or a string alone.
        for id in [
            "macro.shop.watermark",
            "macro.shop.in_call",
            "macro.shop.hook_helper",
            "macro.shop.pre_helper",
            "macro.shop.one_hook",
        ] {
            assert!(g.macro_use.reached(at(&g, id)), "{id}");
        }
        let u = g.macro_use.usage(at(&g, "macro.shop.watermark"));
        assert!(u.direct.is_empty());
        assert_eq!(u.written, [(g.index["model.shop.inc"], None)]);
        let s = summary(&g, "macro.shop.watermark");
        assert_eq!((s.by.clone(), s.written), (vec![KindCount { kind: "model", n: 1 }], 1));
        assert_eq!(
            jinja_calls(
                "select 1 {% if x %}{{ a() }}{% endif %} b() {# {{ c() }} #} {{ \"d()\" }} {{ e.f() }} {{ g.h.i() }}"
            ),
            ["a", "e.f"],
            "SQL outside the Jinja, a comment, a string and three names deep are no calls"
        );
        assert_eq!(jinja_calls("{%- set x = a (1) -%} {{ b() }} {{ a() }} {{ unclosed("), ["a", "b"]);
        // A method, a filter, a test and a raw block are no calls.
        assert_eq!(jinja_calls("{{ 'x'.upper() }} {{ 1.234 | round(2) }} {{ y|default(z()) }}"), ["z"]);
        assert_eq!(jinja_calls("{% if x is divisibleby(3) or x is not sameas(y) or not is_ok() %}"), ["is_ok"]);
        assert_eq!(jinja_calls("{% raw %}{{ legacy() }}{% endraw %}{{ after() }}{%- raw -%}{{ x() }}"), ["after"]);
    }

    #[test]
    fn reached_follows_every_call_down_and_stops_at_a_cycle() {
        let g = graph();
        for id in ["macro.shop.cents", "macro.shop.round_amount", "macro.shop.duckdb__cents", "macro.shop.log_start"] {
            assert!(g.macro_use.reached(at(&g, id)), "{id}");
        }
        for id in ["macro.shop.orphan", "macro.shop.helper", "macro.shop.ping", "macro.shop.pong"] {
            assert!(!g.macro_use.reached(at(&g, id)), "{id}");
        }
        let ping = g.macro_use.usage(at(&g, "macro.shop.ping"));
        assert!(ping.direct.is_empty() && ping.through.is_empty());
        assert_eq!(macro_names(&g, &ping.callers), ["pong"]);
    }

    #[test]
    fn a_disabled_node_reaches_nothing_and_a_macro_listing_itself_is_no_caller() {
        let g = graph();
        assert!(!g.macro_use.reached(at(&g, "macro.shop.grant_select")));
        let round = g.macro_use.usage(at(&g, "macro.shop.round_amount"));
        assert_eq!(macro_names(&g, &round.callers), ["orphan"]);
    }

    #[test]
    fn a_hook_and_a_generic_test_count_as_the_nodes_they_are() {
        let g = graph();
        let hook = g.macro_use.usage(at(&g, "macro.shop.log_start"));
        assert_eq!(g.nodes[hook.direct[0] as usize].kind, Kind::Operation);
        let test = g.macro_use.usage(at(&g, "macro.shop.test_is_positive"));
        assert_eq!(names(&g, &test.direct), ["positive"]);
    }

    #[test]
    fn what_dbt_runs_itself_says_so_and_so_does_what_it_calls() {
        let g = graph();
        let dbt = |from: &str| (Why::Dbt, None, from.to_string());
        let run = |what: &str, n: usize| (Why::Run, Some((what.to_string(), n)), String::new());
        assert_eq!(why(&g, "macro.shop.generate_schema_name"), dbt(""));
        assert_eq!(why(&g, "macro.shop.schema_helper"), dbt("generate_schema_name"));
        // dbt's own dispatcher lists the variant it picked, as dbt writes it.
        assert_eq!(why(&g, "macro.shop.duckdb__set_query_tag"), dbt("dbt.set_query_tag"));
        assert_eq!(why(&g, "macro.shop.postgres__set_query_tag").0, Why::Dispatch);
        // A macro of dbt's own, or of its adapter's, overridden by its name.
        let overrides = [
            "macro.shop.create_table_as",
            "macro.shop.list_relations_without_caching",
            "macro.shop.duckdb__list_relations",
        ];
        for id in overrides {
            assert_eq!(why(&g, id), dbt(""), "{id}");
        }
        // Run for the nodes that name it, a test's `test` included.
        assert_eq!(why(&g, "macro.shop.materialization_table_default"), run("materialized table", 3));
        assert_eq!(why(&g, "macro.shop.mat_helper"), dbt("materialization_table_default"));
        assert_eq!(why(&g, "macro.shop.materialization_test_default"), run("materialized test", 1));
        assert_eq!(why(&g, "macro.shop.materialization_fancy_default"), run("materialized fancy", 0));
        assert_eq!(why(&g, "macro.shop.materialization_table_postgres").0, Why::Dispatch, "another adapter's");
        assert_eq!(why(&g, "macro.shop.get_incremental_custom_sql"), run("incremental_strategy custom", 1));
        assert_eq!(why(&g, "macro.shop.snapshot_mine_strategy"), run("strategy mine", 1));
        // `shop.custom2` names its package, as dbt's strategy_dispatch allows.
        assert_eq!(why(&g, "macro.shop.snapshot_custom2_strategy"), run("strategy custom2", 1));
        assert_eq!(why(&g, "macro.shop.fancy_helper"), dbt("kit.materialization_fancy2_default"));
        assert_eq!(why(&g, "macro.shop.snapshot_other_strategy").0, Why::None, "no snapshot names it");
        assert_eq!(strategy_macro(false, "delete+insert"), "get_incremental_delete_insert_sql");
    }

    #[test]
    fn what_dbt_runs_only_when_called_or_never_picks_is_no_call_of_dbt() {
        let g = graph();
        // An override of a cross-database helper, a variant its dispatcher
        // lists though no node calls it, and a generic test's.
        assert_eq!(why(&g, "macro.shop.datediff").0, Why::None);
        assert_eq!(why(&g, "macro.shop.duckdb__dateadd"), (Why::Macros, None, String::new()));
        assert_eq!(why(&g, "macro.shop.test_accepted_values").0, Why::Test);
        assert_eq!(why(&g, "macro.shop.equals"), (Why::Dbt, None, String::new()), "a helper dbt itself calls bare");
        // The adapter's own view wins over the project's default one, which
        // and whose helper never run.
        assert_eq!(why(&g, "macro.shop.materialization_view_default").0, Why::Dispatch);
        assert_eq!(why(&g, "macro.shop.view_helper").0, Why::Macros);
    }

    /// One graph on `adapter`: the project's macros, by name, the nodes
    /// materialized as each pair says.
    fn on(adapter: &str, macros: &[(&str, &str)], nodes: &[(&str, &str)]) -> Graph {
        let macros: serde_json::Map<String, serde_json::Value> = macros
            .iter()
            .map(|&(package, name)| {
                let m =
                    serde_json::json!({ "name": name, "package_name": package, "original_file_path": "macros/m.sql" });
                (format!("macro.{package}.{name}"), m)
            })
            .collect();
        let nodes: serde_json::Map<String, serde_json::Value> = nodes
            .iter()
            .map(|&(name, mat)| {
                let n = serde_json::json!({ "name": name, "resource_type": "model", "package_name": "shop",
                    "config": { "materialized": mat } });
                (format!("model.shop.{name}"), n)
            })
            .collect();
        let raw: RawManifest = serde_json::from_value(serde_json::json!({
            "metadata": { "project_name": "shop", "adapter_type": adapter }, "nodes": nodes, "macros": macros,
        }))
        .unwrap();
        Graph::build(raw, Path::new("manifest.json"), 0, 0)
    }

    /// dbt-core's `MaterializationCandidate`: the nearest the adapter, then
    /// the project before a package before dbt; and a package's gives way
    /// where dbt has one of that materialization.
    #[test]
    fn the_materialization_dbt_runs_is_the_one_dbt_core_picks() {
        let mac = |name: &str, package: &str, calls: &[&str]| {
            serde_json::json!({ "name": name, "package_name": package, "original_file_path": "macros/m.sql",
                "depends_on": { "macros": calls } })
        };
        let raw: RawManifest = serde_json::from_value(serde_json::json!({
            "metadata": { "project_name": "shop", "adapter_type": "postgres" },
            "nodes": {
                "model.shop.a": { "name": "a", "resource_type": "model", "package_name": "shop",
                    "config": { "materialized": "table" } },
                "model.shop.b": { "name": "b", "resource_type": "model", "package_name": "shop",
                    "config": { "materialized": "gem" } },
                "snapshot.shop.s": { "name": "s", "resource_type": "snapshot", "package_name": "shop",
                    "config": { "materialized": "snapshot", "strategy": "kit.custom" } },
            },
            "macros": {
                "macro.dbt.materialization_table_default": mac("materialization_table_default", "dbt", &[]),
                "macro.shop.materialization_table_default":
                    mac("materialization_table_default", "shop", &["macro.shop.root_table_helper"]),
                "macro.shop.root_table_helper": mac("root_table_helper", "shop", &[]),
                "macro.kit.materialization_table_postgres":
                    mac("materialization_table_postgres", "kit", &["macro.shop.kit_table_helper"]),
                "macro.shop.kit_table_helper": mac("kit_table_helper", "shop", &[]),
                "macro.kit.materialization_gem_default": mac("materialization_gem_default", "kit", &[]),
                "macro.shop.materialization_gem_default": mac("materialization_gem_default", "shop", &[]),
                "macro.shop.snapshot_custom_strategy": mac("snapshot_custom_strategy", "shop", &[]),
                "macro.kit.snapshot_custom_strategy": mac("snapshot_custom_strategy", "kit", &[]),
            },
        }))
        .unwrap();
        let g = Graph::build(raw, Path::new("manifest.json"), 0, 0);
        // kit's is nearer postgres, but dbt has a table of its own: the best
        // of dbt's and the project's runs, the project's.
        assert_eq!(why(&g, "macro.shop.materialization_table_default").1, Some(("materialized table".to_string(), 1)));
        assert_eq!(why(&g, "macro.shop.root_table_helper").2, "materialization_table_default");
        assert_eq!(why(&g, "macro.shop.kit_table_helper").0, Why::Macros, "kit's table never runs");
        // At one nearness, the project's before a package's.
        assert_eq!(why(&g, "macro.shop.materialization_gem_default").1, Some(("materialized gem".to_string(), 1)));
        // `kit.custom` runs kit's strategy, not the project's of that name.
        assert_eq!(why(&g, "macro.shop.snapshot_custom_strategy").0, Why::None);
    }

    /// Two packages that tie for a materialization are both dbt's pick, and
    /// the answer is the same on every start (0055): the tie is no function of
    /// the order the macros came in, which a HashMap decides.
    #[test]
    fn packages_tied_for_a_materialization_read_alike_on_every_build() {
        let mac = |name: &str, package: &str, calls: &[&str]| {
            serde_json::json!({ "name": name, "package_name": package, "original_file_path": "macros/m.sql",
                "depends_on": { "macros": calls } })
        };
        let macros = [
            ("macro.kit.materialization_fancy_default", mac("materialization_fancy_default", "kit", &["macro.kit.kit_helper"])),
            ("macro.kit.kit_helper", mac("kit_helper", "kit", &[])),
            ("macro.kat.materialization_fancy_default", mac("materialization_fancy_default", "kat", &["macro.kat.kat_helper"])),
            ("macro.kat.kat_helper", mac("kat_helper", "kat", &[])),
            ("macro.shop.lonely", mac("lonely", "shop", &[])),
        ];
        let model = r#"{"model.shop.m": {"name": "m", "resource_type": "model", "package_name": "shop",
            "config": {"materialized": "fancy"}}}"#;
        // The macros written out in the given order, so two orders are two
        // different texts; each parse also seeds its own HashMap.
        let build = |order: &[usize]| {
            let body: Vec<String> = order.iter().map(|&i| format!("{:?}: {}", macros[i].0, macros[i].1)).collect();
            let text = format!(
                r#"{{"metadata": {{"project_name": "shop", "adapter_type": "duckdb"}}, "nodes": {model}, "macros": {{{}}}}}"#,
                body.join(",")
            );
            let raw: RawManifest = serde_json::from_str(&text).unwrap();
            Graph::build(raw, Path::new("manifest.json"), 0, 0)
        };
        // Everything the report and the cards say, keyed by id, since a
        // position is not the same twice, and the unreached list with it.
        let answers = |g: &Graph| {
            let mut all = std::collections::BTreeMap::new();
            for m in 0..g.macros.len() as u32 {
                let s = g.macro_use.summary(&g.macros, &g.nodes, &g.meta.adapter, m).clone();
                let (why, run, from) = g.macro_use.why(&g.macros, &g.nodes, &g.meta.adapter, m);
                // `from` is a position, so it is read back as the macro it names.
                let from = from.map(|f| g.macros.at(f).id.clone());
                all.insert(g.macros.at(m).id.clone(), (s, why, run.map(|r| (r.name, r.nodes)), from));
            }
            let unreached = macro_names(g, &g.macro_use.unreached(&g.macros));
            (all, unreached)
        };
        let first = build(&[0, 1, 2, 3, 4]);
        let reference = answers(&first);
        // Each helper names the materialization it comes from, its own package's.
        assert_eq!(
            why(&first, "macro.kit.kit_helper"),
            (Why::Dbt, None, "kit.materialization_fancy_default".to_string()),
            "a tie errs toward explained"
        );
        assert_eq!(
            why(&first, "macro.kat.kat_helper"),
            (Why::Dbt, None, "kat.materialization_fancy_default".to_string())
        );
        assert_eq!(why(&first, "macro.kit.materialization_fancy_default").1, Some(("materialized fancy".to_string(), 1)));
        assert_eq!(why(&first, "macro.kat.materialization_fancy_default").1, Some(("materialized fancy".to_string(), 1)));
        for order in [[0, 1, 2, 3, 4], [4, 3, 2, 1, 0], [2, 3, 0, 1, 4], [3, 0, 4, 1, 2]] {
            for _ in 0..8 {
                assert_eq!(answers(&build(&order)), reference, "order {order:?}");
            }
        }
    }

    #[test]
    fn a_parent_adapters_macros_are_dbts_own_and_its_materializations_run() {
        let g = on(
            "redshift",
            &[
                ("shop", "materialization_x_postgres"),
                ("shop", "materialization_y_postgres"),
                ("dbt_redshift", "materialization_y_redshift"),
                ("shop", "postgres__get_relations"),
                ("dbt_postgres", "postgres__get_relations"),
            ],
            &[("a", "x"), ("b", "y")],
        );
        assert_eq!(why(&g, "macro.shop.materialization_x_postgres").1, Some(("materialized x".to_string(), 1)));
        assert_eq!(why(&g, "macro.shop.materialization_y_postgres").0, Why::Dispatch, "redshift's own is nearer");
        assert_eq!(why(&g, "macro.shop.postgres__get_relations").0, Why::Dbt);
        let g = on("databricks", &[("shop", "spark__get_columns"), ("dbt_spark", "spark__get_columns")], &[]);
        assert_eq!(why(&g, "macro.shop.spark__get_columns").0, Why::Dbt);
        let g = on("duckdb", &[("shop", "spark__get_columns"), ("dbt_spark", "spark__get_columns")], &[]);
        assert_eq!(why(&g, "macro.shop.spark__get_columns").0, Why::None, "spark is no parent of duckdb");
    }

    #[test]
    fn each_other_macro_no_node_reaches_says_why_it_may_still_run() {
        let g = graph();
        let w = |id: &str| why(&g, id).0;
        assert_eq!(w("macro.shop.grant_select"), Why::None);
        assert_eq!(w("macro.shop.orphan"), Why::None);
        assert_eq!(w("macro.shop.helper"), Why::Macros);
        assert_eq!(w("macro.shop.ping"), Why::Macros);
        assert_eq!(w("macro.shop.test_unused"), Why::Test);
        // A variant for a package's macro, for another adapter.
        assert_eq!(w("macro.shop.snowflake__star"), Why::Dispatch);
        assert_eq!(w("macro.shop.nulls"), Why::None);
    }

    #[test]
    fn only_what_nothing_explains_lights_the_reports_button() {
        let g = graph();
        let lit: Vec<String> = g
            .macro_use
            .unreached(&g.macros)
            .into_iter()
            .filter(|&m| g.macro_use.summary(&g.macros, &g.nodes, &g.meta.adapter, m).unexplained())
            .map(|m| g.macros.at(m).name.clone())
            .collect();
        assert_eq!(
            lit,
            [
                "datediff",
                "duckdb__dateadd",
                "grant_select",
                "helper",
                "materialization_fancy_default",
                "null_list",
                "nulls",
                "orphan",
                "ping",
                "pong",
                "snapshot_other_strategy",
                "test_accepted_values",
                "test_unused",
                "view_helper"
            ]
        );
        let first: *const Summary = summary(&g, "macro.shop.cents");
        assert!(std::ptr::eq(first, summary(&g, "macro.shop.cents")), "worked out once");
    }

    #[test]
    fn the_report_lists_the_projects_macros_no_node_reaches_by_name() {
        let g = graph();
        let names = macro_names(&g, &g.macro_use.unreached(&g.macros));
        assert_eq!(names.len(), 33);
        assert!(names.windows(2).all(|w| w[0] < w[1]), "{names:?}");
        let used = ["cents", "watermark", "in_call", "log_start", "test_is_positive"];
        assert!(!names.iter().any(|n| used.contains(&n.as_str())));
    }

    #[test]
    fn a_materialization_is_read_back_into_its_name_and_adapter() {
        assert_eq!(materialization("materialization_table_default"), Some(("table", "default")));
        assert_eq!(materialization("materialization_tagged_view_snowflake"), Some(("tagged_view", "snowflake")));
        assert_eq!(materialization("materialization_x"), None);
        assert_eq!(materialization("table_default"), None);
        let g = graph();
        // The three enabled table models, not the hook nor the disabled one.
        assert_eq!(materialized(&g.nodes, "table"), 3);
    }

    #[test]
    fn a_null_where_a_list_was_expected_costs_nothing() {
        let g = graph();
        assert!(g.macros.position("macro.shop.nulls").is_some() && g.macros.position("macro.shop.null_list").is_some());
        assert!(!g.nodes.is_empty(), "the whole manifest was read");
    }
}
