//! The guided tour's facts from the server: which model it shows, whether it
//! was offered already, and the newest version that served a page here (0074).
//!
//! Two rules hold here:
//!   - the model is picked from the graph alone, by its counts and then its
//!     name, so the same manifest gives the same model on every start (0055).
//!   - the offer is made once per machine, not per project: the tour shows the
//!     tool, which is the same everywhere. `tour.json` in the config directory
//!     says when it was made and the newest dbt-edith a page came from, and
//!     holds nothing else. That version is what a tour of what is new since
//!     will start from, so it is kept from the first release with a tour on.
//!     Without a config directory nothing can remember either, so the tour is
//!     never offered on its own there, and stays one click away in the
//!     shortcut list.

use std::io::Write;
use std::path::{Path, PathBuf};

use crate::graph::{Graph, Kind};

/// Where the offer is remembered, beside `update.json` (0011, 0068).
pub fn offered_path(config_dir: &Path) -> PathBuf {
    config_dir.join("tour.json")
}

/// The model the tour opens, and how many boxes stand on each side of it.
#[derive(serde::Serialize, Clone, Debug, PartialEq)]
pub struct Pick {
    pub id: String,
    pub name: String,
    pub file: String,
    pub parents: usize,
    pub children: usize,
}

/// A model of the project with parents and children, so that the lineage,
/// the Catalog and the Jobs tab all have something to show, and among those
/// one a job runs, when `ran` says which (`jobs::run_by_a_step`; empty
/// without a job source). Then the one with the most on its thinner side, up
/// to three, then the one whose
/// neighbours come closest to eight, the fewer of two as close: a canvas
/// someone can read at a glance, never a hub with hundreds of children. Then
/// the first by name. Tests do not count, since the canvas hangs them under
/// their model (0034). A project whose own package has no model falls back
/// on its packages'.
pub fn pick(graph: &Graph, ran: &[bool]) -> Option<Pick> {
    let side = |ids: &[u32]| {
        ids.iter().filter(|&&i| {
            let n = &graph.nodes[i as usize];
            n.kind.is_graph_node() && !n.disabled
        }).count()
    };
    let rank = |own: bool| {
        graph
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.kind == Kind::Model && !n.disabled && !n.on_disk().is_empty())
            .filter(|(_, n)| !own || n.package == graph.meta.project)
            .map(|(i, n)| (n, side(&n.parents), side(&n.children), ran.get(i).copied().unwrap_or(false)))
            .max_by(|(a, ap, ac, ar), (b, bp, bc, br)| {
                let (at, bt) = (ap + ac, bp + bc);
                let both = |p: usize, c: usize| p > 0 && c > 0;
                both(*ap, *ac)
                    .cmp(&both(*bp, *bc))
                    .then(ar.cmp(br))
                    .then((*ap).min(*ac).min(3).cmp(&(*bp).min(*bc).min(3)))
                    .then(bt.abs_diff(8).cmp(&at.abs_diff(8)))
                    .then(bt.cmp(&at))
                    .then(b.name.cmp(&a.name))
                    .then(b.id.cmp(&a.id))
            })
    };
    let (n, parents, children, _) = rank(true).or_else(|| rank(false))?;
    Some(Pick { id: n.id.clone(), name: n.name.clone(), file: n.on_disk().to_string(), parents, children })
}

/// What `tour.json` holds.
#[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq)]
pub struct Kept {
    /// When the tour was offered, in seconds since the epoch.
    #[serde(default)]
    pub offered: u64,
    /// The newest dbt-edith that served a page on this machine.
    #[serde(default)]
    pub version: String,
}

/// Whether the tour was offered on this machine, and the claim to offer it.
pub struct Offer {
    /// `tour.json`, or None without a config directory.
    pub path: Option<PathBuf>,
}

impl Offer {
    /// A page of `version` was served: true for the first on this machine,
    /// which is the one that offers the tour, and false for every later one,
    /// whatever tab or process asks. The file is created rather than written
    /// over, so two tabs opened at once cannot both be told yes. A file that
    /// cannot be written says no: an offer nothing remembers would come back
    /// at every start. Later pages raise the version kept, never lower it, so
    /// an older binary started after a newer one changes nothing.
    pub fn visit(&self, version: &str) -> bool {
        let Some(path) = &self.path else { return false };
        if let Some(dir) = path.parent() {
            if std::fs::create_dir_all(dir).is_err() {
                return false;
            }
        }
        let body = |kept: &Kept| format!("{:#}\n", serde_json::json!(kept));
        let first = Kept { offered: crate::update::now_secs(), version: version.to_string() };
        if let Ok(mut file) = std::fs::OpenOptions::new().write(true).create_new(true).open(path) {
            return file.write_all(body(&first).as_bytes()).is_ok();
        }
        // A file that is not its shape is written again in its shape.
        let loaded = load(path);
        let unreadable = loaded.is_none();
        let mut kept = loaded.unwrap_or(first);
        if unreadable || kept.version.is_empty() || crate::update::newer(&kept.version, version) {
            kept.version = version.to_string();
            let _ = crate::settings::write_atomic(path, body(&kept).as_bytes());
        }
        false
    }
}

/// `tour.json` as kept, or None when it is missing or not its shape.
pub fn load(path: &Path) -> Option<Kept> {
    serde_json::from_slice(&std::fs::read(path).ok()?).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::RawManifest;

    /// A manifest from a list of (name, kind, package, parents): enough for
    /// the pick, which reads nothing else.
    fn graph(project: &str, nodes: &[(&str, &str, &str, &[&str])]) -> Graph {
        let id = |name: &str, kind: &str, package: &str| format!("{kind}.{package}.{name}");
        let mut parent_map = serde_json::Map::new();
        let mut manifest_nodes = serde_json::Map::new();
        for (name, kind, package, parents) in nodes {
            let uid = id(name, kind, package);
            let parents: Vec<String> = parents
                .iter()
                .map(|p| nodes.iter().find(|n| n.0 == *p).map(|n| id(n.0, n.1, n.2)).unwrap())
                .collect();
            parent_map.insert(uid.clone(), serde_json::json!(parents));
            manifest_nodes.insert(
                uid.clone(),
                serde_json::json!({
                    "unique_id": uid,
                    "name": name,
                    "resource_type": kind,
                    "package_name": package,
                    "original_file_path": format!("models/{name}.sql"),
                    "depends_on": { "nodes": parents },
                }),
            );
        }
        let raw: RawManifest = serde_json::from_value(serde_json::json!({
            "metadata": { "project_name": project },
            "nodes": manifest_nodes,
            "parent_map": parent_map,
        }))
        .unwrap();
        Graph::build(raw, Path::new("target/manifest.json"), 0, 0)
    }

    #[test]
    fn the_pick_has_parents_and_children_and_ties_go_by_name() {
        // Jaffle Shop's shape: three staging models between the seeds and the marts.
        let g = graph(
            "jaffle_shop",
            &[
                ("raw_customers", "seed", "jaffle_shop", &[]),
                ("raw_orders", "seed", "jaffle_shop", &[]),
                ("raw_payments", "seed", "jaffle_shop", &[]),
                ("stg_customers", "model", "jaffle_shop", &["raw_customers"]),
                ("stg_orders", "model", "jaffle_shop", &["raw_orders"]),
                ("stg_payments", "model", "jaffle_shop", &["raw_payments"]),
                ("customers", "model", "jaffle_shop", &["stg_customers", "stg_orders", "stg_payments"]),
                ("orders", "model", "jaffle_shop", &["stg_orders", "stg_payments"]),
            ],
        );
        let p = pick(&g, &[]).unwrap();
        // stg_orders and stg_payments both feed two marts from one seed: the name decides.
        assert_eq!((p.name.as_str(), p.parents, p.children), ("stg_orders", 1, 2));
        assert_eq!(p.file, "models/stg_orders.sql");
    }

    #[test]
    fn a_hub_loses_to_a_model_with_a_readable_neighbourhood() {
        let mut nodes: Vec<(String, &str, &str, Vec<String>)> = vec![
            ("src".into(), "seed", "p", vec![]),
            ("hub".into(), "model", "p", vec!["src".into()]),
            ("mid".into(), "model", "p", vec!["src".into()]),
            ("end".into(), "model", "p", vec!["mid".into()]),
        ];
        for i in 0..30 {
            nodes.push((format!("leaf_{i:02}"), "model", "p", vec!["hub".into()]));
        }
        let names: Vec<(String, &str, &str, Vec<&str>)> =
            nodes.iter().map(|(n, k, p, ps)| (n.clone(), *k, *p, ps.iter().map(String::as_str).collect())).collect();
        let refs: Vec<(&str, &str, &str, &[&str])> =
            names.iter().map(|(n, k, p, ps)| (n.as_str(), *k, *p, ps.as_slice())).collect();
        let g = graph("p", &refs);
        assert_eq!(pick(&g, &[]).unwrap().name, "mid");
        // A job running the hub wins it the pick: a Jobs tab with nothing in
        // it shows less than a busy canvas.
        let mut ran = vec![false; g.nodes.len()];
        ran[g.index["model.p.hub"] as usize] = true;
        assert_eq!(pick(&g, &ran).unwrap().name, "hub");
        // But never over parents and children: a job running the end of the
        // chain, which has no child, leaves the pick where it was.
        let mut ran = vec![false; g.nodes.len()];
        ran[g.index["model.p.end"] as usize] = true;
        assert_eq!(pick(&g, &ran).unwrap().name, "mid");
    }

    #[test]
    fn the_project_s_own_models_come_before_a_package_s() {
        let g = graph(
            "shop",
            &[
                ("a", "model", "pkg", &[]),
                ("b", "model", "pkg", &["a"]),
                ("c", "model", "pkg", &["b"]),
                ("mine", "model", "shop", &[]),
            ],
        );
        assert_eq!(pick(&g, &[]).unwrap().name, "mine");
        let only_packages = graph("shop", &[("a", "model", "pkg", &[]), ("b", "model", "pkg", &["a"]), ("c", "model", "pkg", &["b"])]);
        assert_eq!(pick(&only_packages, &[]).unwrap().name, "b");
        assert_eq!(pick(&graph("shop", &[("raw", "seed", "shop", &[])]), &[]), None);
    }

    #[test]
    fn the_offer_is_claimed_once_and_never_without_a_place_to_remember_it() {
        let dir = std::env::temp_dir().join(format!("dbt-edith-tour-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = offered_path(&dir.join("config"));
        let offer = Offer { path: Some(path.clone()) };
        assert!(offer.visit("1.16.0"), "the first start offers it, creating the directory");
        assert!(!offer.visit("1.16.0"), "and no later one");
        assert!(!Offer { path: Some(path.clone()) }.visit("1.16.0"), "nor another process");
        let kept: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        let keys: Vec<&String> = kept.as_object().unwrap().keys().collect();
        assert_eq!(keys, ["offered", "version"], "nothing else is kept");
        assert!(!Offer { path: None }.visit("1.16.0"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_version_kept_is_the_newest_that_served_a_page() {
        let dir = std::env::temp_dir().join(format!("dbt-edith-tour-version-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = offered_path(&dir);
        let offer = Offer { path: Some(path.clone()) };
        assert!(offer.visit("1.16.0"));
        let offered = load(&path).unwrap().offered;
        assert!(!offer.visit("1.18.2"));
        assert_eq!(load(&path), Some(Kept { offered, version: "1.18.2".into() }), "an update raises it");
        assert!(!offer.visit("1.17.0"));
        assert_eq!(load(&path).unwrap().version, "1.18.2", "an older binary started since does not lower it");
        std::fs::write(&path, "not json").unwrap();
        assert!(!offer.visit("1.18.2"), "a file it cannot read was still an offer made");
        assert_eq!(load(&path).map(|k| k.version), Some("1.18.2".into()), "and is written again in its shape");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
