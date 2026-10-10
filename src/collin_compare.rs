//! What two collin versions read in one project, side by side: the cache and
//! the report each version's last run left, kept apart from the ones on screen.
//!
//! After a run, its cache and report are copied under `target/collin/<X.Y.Z>/`.
//! Copied, not linked: collin run by hand writes its files in place, and a
//! link would let that rewrite what was kept. Each version keeps its latest
//! run only (0061).
//!
//! The comparison says what moved and never which side is right: an edge one
//! version has and the other lacks is a difference to read in the SQL.

use crate::collin::{RawColEdge, RawColLineage};
use crate::collin_run::{is_version, Report};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

pub const CACHE: &str = "column_lineage.collin.json";
pub const REPORT: &str = "collin.report.json";

/// Edges listed per side of a difference, sorted. The counts are whole.
const SAMPLE: usize = 300;
/// Models listed with what moved in them, the most first.
const MODELS: usize = 200;

pub fn kept_dir(target_dir: &Path) -> PathBuf {
    target_dir.join("collin")
}

/// Keeps a run's cache and report as `version`'s, replacing what that version
/// kept before. A report missing is kept missing, so the pair always describes
/// one run.
pub fn keep(target_dir: &Path, version: &str, cache: &Path, report: &Path) -> std::io::Result<()> {
    if !is_version(version) {
        return Ok(());
    }
    let dir = kept_dir(target_dir).join(version);
    std::fs::create_dir_all(&dir)?;
    for (from, name) in [(cache, CACHE), (report, REPORT)] {
        let to = dir.join(name);
        let _ = std::fs::remove_file(&to);
        if from.is_file() {
            std::fs::copy(from, &to)?;
        }
    }
    Ok(())
}

/// The versions with a kept run, newest first: a folder named for a version,
/// holding a cache.
pub fn kept(target_dir: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(kept_dir(target_dir)) else { return Vec::new() };
    let mut out: Vec<String> = entries
        .flatten()
        .filter_map(|e| e.file_name().to_str().filter(|n| is_version(n)).map(str::to_string))
        .filter(|v| kept_dir(target_dir).join(v).join(CACHE).is_file())
        .collect();
    out.sort_by_key(|v| std::cmp::Reverse(numbers(v)));
    out
}

fn numbers(version: &str) -> Vec<u32> {
    version.split('.').map(|p| p.parse().unwrap_or(0)).collect()
}

#[derive(serde::Serialize, Clone, Debug, Default, PartialEq)]
pub struct Side {
    pub version: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub generated_at: String,
    /// Seconds since the epoch the kept cache was written, for its age and to
    /// say when it is older than the manifest.
    pub mtime: u64,
    pub edges: usize,
    /// Its report was kept with it. A collin's run without one compares by its
    /// edges alone.
    pub report: bool,
}

/// One figure on both sides. Absent on a side when that version's report does
/// not have it.
#[derive(serde::Serialize, Clone, Debug, PartialEq)]
pub struct Delta {
    pub key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub a: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub b: Option<u64>,
}

#[derive(serde::Serialize, Clone, Debug, PartialEq)]
pub struct Edge {
    pub from: String,
    pub from_col: String,
    pub to: String,
    pub to_col: String,
    pub kind: String,
}

/// An edge both have, with another role on each side.
#[derive(serde::Serialize, Clone, Debug, PartialEq)]
pub struct Changed {
    #[serde(flatten)]
    pub edge: Edge,
    pub b_kind: String,
}

#[derive(serde::Serialize, Clone, Debug, Default, PartialEq)]
pub struct ModelMove {
    pub id: String,
    /// How each side read it: parsed, per_column, inferred or unresolved. A
    /// model absent from a report is clean, so parsed.
    pub a: String,
    pub b: String,
    #[serde(skip_serializing_if = "is_zero")]
    pub gained: usize,
    #[serde(skip_serializing_if = "is_zero")]
    pub lost: usize,
    #[serde(skip_serializing_if = "is_zero")]
    pub changed: usize,
}

fn is_zero(n: &usize) -> bool {
    *n == 0
}

#[derive(serde::Serialize, Clone, Debug, Default, PartialEq)]
pub struct Comparison {
    pub a: Side,
    pub b: Side,
    /// Every counter either report has, by name.
    pub totals: Vec<Delta>,
    /// Entries per kind of finding, as the report page files them.
    pub findings: Vec<Delta>,
    /// Edges by role, and by what they come from: a model, a source, a seed,
    /// a snapshot, or a relation no node owns (`rel`).
    pub kinds: Vec<Delta>,
    pub origins: Vec<Delta>,
    /// Edges only B has, only A has, and with another role on each side, by
    /// node and column, columns compared without case as the graph does.
    pub gained: usize,
    pub lost: usize,
    pub changed: usize,
    pub gained_edges: Vec<Edge>,
    pub lost_edges: Vec<Edge>,
    pub changed_edges: Vec<Changed>,
    /// Models read another way, or with edges that moved, the most first.
    pub models: Vec<ModelMove>,
    /// How many models that was before the cut.
    pub models_moved: usize,
}

fn origin(from: &str) -> &'static str {
    match from.split(['.', ':']).next().unwrap_or_default() {
        "model" => "model",
        "source" => "source",
        "seed" => "seed",
        "snapshot" => "snapshot",
        "rel" => "rel",
        _ => "other",
    }
}

type Key = (String, String, String, String);

fn key(e: &RawColEdge) -> Key {
    (e.from.clone(), e.from_col.to_lowercase(), e.to.clone(), e.to_col.to_lowercase())
}

fn edge(k: &Key, kind: &str) -> Edge {
    Edge { from: k.0.clone(), from_col: k.1.clone(), to: k.2.clone(), to_col: k.3.clone(), kind: kind.to_string() }
}

fn counts<'a>(items: impl Iterator<Item = &'a str>) -> BTreeMap<String, u64> {
    let mut out = BTreeMap::new();
    for i in items {
        *out.entry(i.to_string()).or_insert(0) += 1;
    }
    out
}

/// `counted` says, per side, whether a name it lacks is a zero: true for
/// edges and findings, which are counted whole, false for the report's
/// counters, which an older collin did not have.
fn deltas(a: &BTreeMap<String, u64>, b: &BTreeMap<String, u64>, counted: (bool, bool)) -> Vec<Delta> {
    let keys: BTreeSet<&String> = a.keys().chain(b.keys()).collect();
    let side = |m: &BTreeMap<String, u64>, k: &String, zero: bool| m.get(k).copied().or(zero.then_some(0));
    keys.into_iter().map(|k| Delta { key: k.clone(), a: side(a, k, counted.0), b: side(b, k, counted.1) }).collect()
}

fn mtime(path: &Path) -> u64 {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_secs())
}

struct Read {
    side: Side,
    edges: HashMap<Key, String>,
    report: Option<Report>,
}

fn read(target_dir: &Path, version: &str) -> Result<Read, String> {
    let dir = kept_dir(target_dir).join(version);
    let cache = RawColLineage::load(&dir.join(CACHE)).map_err(|e| format!("collin {version}'s kept cache: {e}"))?;
    let report = crate::collin_run::read_report(&dir.join(REPORT));
    let edges: HashMap<Key, String> = cache.edges.iter().map(|e| (key(e), e.kind.clone())).collect();
    let side = Side {
        version: version.to_string(),
        generated_at: cache.generated_at,
        mtime: mtime(&dir.join(CACHE)),
        edges: edges.len(),
        report: report.is_some(),
    };
    Ok(Read { side, edges, report })
}

/// Compares the runs `a` and `b` kept, B against A: gained is what B has and A
/// lacks.
pub fn compare(target_dir: &Path, a: &str, b: &str) -> Result<Comparison, String> {
    let (ra, rb) = (read(target_dir, a)?, read(target_dir, b)?);
    let totals = |r: &Read| -> BTreeMap<String, u64> {
        r.report
            .as_ref()
            .map(|rep| rep.overview.totals.iter().filter_map(|(k, v)| Some((k.clone(), v.as_u64()?))).collect())
            .unwrap_or_default()
    };
    let findings = |r: &Read| -> BTreeMap<String, u64> {
        r.report
            .as_ref()
            .map(|rep| rep.overview.findings.iter().map(|f| (f.kind.to_string(), f.entries.len() as u64)).collect())
            .unwrap_or_default()
    };
    let kinds = |r: &Read| counts(r.edges.values().map(String::as_str));
    let origins = |r: &Read| counts(r.edges.keys().map(|k| origin(&k.0)));

    let mut gained: Vec<Edge> = rb.edges.iter().filter(|(k, _)| !ra.edges.contains_key(*k)).map(|(k, kind)| edge(k, kind)).collect();
    let mut lost: Vec<Edge> = ra.edges.iter().filter(|(k, _)| !rb.edges.contains_key(*k)).map(|(k, kind)| edge(k, kind)).collect();
    let mut changed: Vec<Changed> = ra
        .edges
        .iter()
        .filter_map(|(k, ka)| {
            let kb = rb.edges.get(k)?;
            (kb != ka).then(|| Changed { edge: edge(k, ka), b_kind: kb.clone() })
        })
        .collect();
    let order = |e: &Edge| (e.to.clone(), e.to_col.clone(), e.from.clone(), e.from_col.clone());
    gained.sort_by_key(order);
    lost.sort_by_key(order);
    changed.sort_by_key(|c| order(&c.edge));

    // A model is described by the edges into it.
    let mut moves: BTreeMap<String, ModelMove> = BTreeMap::new();
    fn slot<'m>(moves: &'m mut BTreeMap<String, ModelMove>, id: &str) -> &'m mut ModelMove {
        moves.entry(id.to_string()).or_insert_with(|| ModelMove { id: id.to_string(), ..Default::default() })
    }
    for e in &gained {
        slot(&mut moves, &e.to).gained += 1;
    }
    for e in &lost {
        slot(&mut moves, &e.to).lost += 1;
    }
    for c in &changed {
        slot(&mut moves, &c.edge.to).changed += 1;
    }
    let provenance = |r: &Read, id: &str| -> String {
        let Some(rep) = &r.report else { return String::new() };
        rep.notes.get(id).map_or_else(|| "parsed".to_string(), |n| n.provenance.clone())
    };
    // And by how each side read it, when the two reports disagree.
    if let (Some(na), Some(nb)) = (&ra.report, &rb.report) {
        let ids: BTreeSet<&String> = na.notes.keys().chain(nb.notes.keys()).collect();
        for id in ids {
            if provenance(&ra, id) != provenance(&rb, id) {
                slot(&mut moves, id);
            }
        }
    }
    let mut models: Vec<ModelMove> = moves
        .into_values()
        .map(|mut m| {
            m.a = provenance(&ra, &m.id);
            m.b = provenance(&rb, &m.id);
            m
        })
        .collect();
    let weight = |m: &ModelMove| m.gained + m.lost + m.changed + usize::from(m.a != m.b);
    models.sort_by(|x, y| weight(y).cmp(&weight(x)).then_with(|| x.id.cmp(&y.id)));
    let models_moved = models.len();
    models.truncate(MODELS);

    let (n_gained, n_lost, n_changed) = (gained.len(), lost.len(), changed.len());
    gained.truncate(SAMPLE);
    lost.truncate(SAMPLE);
    changed.truncate(SAMPLE);
    Ok(Comparison {
        totals: deltas(&totals(&ra), &totals(&rb), (false, false)),
        findings: deltas(&findings(&ra), &findings(&rb), (ra.report.is_some(), rb.report.is_some())),
        kinds: deltas(&kinds(&ra), &kinds(&rb), (true, true)),
        origins: deltas(&origins(&ra), &origins(&rb), (true, true)),
        gained: n_gained,
        lost: n_lost,
        changed: n_changed,
        gained_edges: gained,
        lost_edges: lost,
        changed_edges: changed,
        models,
        models_moved,
        a: ra.side,
        b: rb.side,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("dbt-edith-compare-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn cache(edges: &[(&str, &str, &str, &str, &str)]) -> String {
        let edges: Vec<serde_json::Value> = edges
            .iter()
            .map(|(f, fc, t, tc, k)| serde_json::json!({ "from": f, "from_col": fc, "to": t, "to_col": tc, "kind": k }))
            .collect();
        serde_json::json!({ "version": 1, "source": "collin", "producer": "collin x", "edges": edges }).to_string()
    }

    /// Writes a run where collin would, then keeps it as `version`'s.
    fn run(target: &Path, version: &str, edges: &[(&str, &str, &str, &str, &str)], report: Option<serde_json::Value>) {
        let (c, r) = (target.join(CACHE), target.join(REPORT));
        // Written in place, as collin run by hand writes: what was kept must not move.
        std::fs::write(&c, cache(edges)).unwrap();
        match report {
            Some(rep) => std::fs::write(&r, rep.to_string()).unwrap(),
            None => {
                let _ = std::fs::remove_file(&r);
            }
        }
        keep(target, version, &c, &r).unwrap();
    }

    #[test]
    fn a_kept_run_outlives_the_next_run_and_versions_list_newest_first() {
        let t = dir("keep");
        run(&t, "0.3.0", &[("model.p.a", "x", "model.p.b", "x", "passthrough")], None);
        run(&t, "0.10.0", &[], None);
        run(&t, "not-a-version", &[], None);
        assert_eq!(kept(&t), ["0.10.0", "0.3.0"]);
        let old = RawColLineage::load(&kept_dir(&t).join("0.3.0").join(CACHE)).unwrap();
        assert_eq!(old.edges.len(), 1, "the run after it rewrote the cache on screen, not the one kept");
        assert!(!kept_dir(&t).join("0.3.0").join(REPORT).exists(), "no report then, none kept");
    }

    #[test]
    fn b_is_read_against_a_edge_by_edge_and_model_by_model() {
        let t = dir("compare");
        let report = |models: serde_json::Value, parsed: u64| {
            Some(serde_json::json!({ "totals": { "models": 3, "parsed": parsed }, "models": models }))
        };
        run(
            &t,
            "0.1.0",
            &[
                ("model.p.a", "id", "model.p.m", "id", "inferred"),
                ("source.p.s.t", "x", "model.p.m", "x", "passthrough"),
                ("rel:DB.S.RAW", "k", "model.p.n", "k", "passthrough"),
            ],
            report(serde_json::json!([{ "unique_id": "model.p.m", "provenance": "inferred" }]), 2),
        );
        run(
            &t,
            "0.4.0",
            &[
                ("model.p.a", "ID", "model.p.m", "id", "passthrough"),
                ("source.p.s.t", "x", "model.p.m", "x", "passthrough"),
                ("model.p.a", "y", "model.p.m", "y", "rename"),
            ],
            report(serde_json::json!([]), 3),
        );
        let c = compare(&t, "0.1.0", "0.4.0").unwrap();
        assert_eq!((c.gained, c.lost, c.changed), (1, 1, 1), "a column's case is no difference, its role is");
        assert_eq!(c.gained_edges[0].to_col, "y");
        assert_eq!(c.lost_edges[0].from, "rel:DB.S.RAW");
        assert_eq!((c.changed_edges[0].edge.kind.as_str(), c.changed_edges[0].b_kind.as_str()), ("inferred", "passthrough"));
        let parsed = c.totals.iter().find(|d| d.key == "parsed").unwrap();
        assert_eq!((parsed.a, parsed.b), (Some(2), Some(3)));
        let rel = c.origins.iter().find(|d| d.key == "rel").unwrap();
        assert_eq!((rel.a, rel.b), (Some(1), Some(0)), "an origin B has no edge from is a zero");
        let inferred = c.kinds.iter().find(|d| d.key == "inferred").unwrap();
        assert_eq!((inferred.a, inferred.b), (Some(1), Some(0)));
        let m = c.models.iter().find(|m| m.id == "model.p.m").unwrap();
        assert_eq!(
            (m.a.as_str(), m.b.as_str(), m.gained, m.changed),
            ("inferred", "parsed", 1, 1),
            "a model the second report leaves out is clean"
        );
        assert_eq!(c.models[0].id, "model.p.m", "the model where the most moved comes first");
        assert_eq!((c.a.edges, c.b.edges, c.a.report), (3, 3, true));
    }

    #[test]
    fn a_version_not_kept_is_no_comparison() {
        let t = dir("missing");
        run(&t, "0.4.0", &[], None);
        assert!(compare(&t, "0.4.0", "0.3.0").is_err());
        let alone = compare(&t, "0.4.0", "0.4.0").unwrap();
        assert!(!alone.a.report && alone.totals.is_empty(), "a run with no report compares by its edges alone");
    }
}
