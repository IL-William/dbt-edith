//! The project the MCP server has opened, kept between calls (0110).
//!
//! Invariants: nothing is read until a call needs a graph. There are two
//! graphs, as the command line has: `--select` and `--column` load the
//! manifest alone, `--describe` and `--overview` also place the packages and
//! merge the catalog, and one shared graph would not answer byte for byte as
//! the command line does. A slot is loaded again when any file its load reads
//! has moved, so an edit to `selectors.yml` or a `dbt parse` in between never
//! answers from the old graph. A load that fails empties its slot: a half
//! written manifest is never kept.

use crate::graph::Graph;
use crate::{api, ls, macros};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// A file as the cache sees it, at the precision the filesystem gives: its
/// length and when it was last written, or None for a file that is not there,
/// so its appearing counts as a change too.
type Stamp = Option<(u64, Option<SystemTime>)>;

fn stamp(path: &Path) -> Stamp {
    std::fs::metadata(path).ok().map(|m| (m.len(), m.modified().ok()))
}

/// What a slot holds, and the stamps it was loaded under.
struct Kept {
    stamps: Vec<Stamp>,
    graph: Graph,
    /// What the load said and the command line prints before its answer:
    /// `catalog.json ignored: ...`.
    warning: Option<String>,
}

/// How many times each graph was read from disk, so a test can tell a call
/// that reloaded from one that did not.
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub struct Loads {
    pub manifest: usize,
    pub base: usize,
}

/// The base graph, with what its load said.
pub struct Base<'a> {
    pub graph: &'a Graph,
    /// The line the command line prints to stderr while it loads, when the
    /// catalog did not read.
    pub warning: Option<&'a str>,
}

pub struct Graphs {
    root: PathBuf,
    manifest_path: PathBuf,
    /// The catalog the server was started with, if it was named.
    catalog: Option<PathBuf>,
    manifest: Option<Kept>,
    base: Option<Kept>,
    loads: Loads,
}

impl Graphs {
    /// Nothing is read here. `root` is the project, canonical; `manifest` is
    /// where the server was told the manifest is, else `target/manifest.json`.
    pub fn new(root: PathBuf, manifest: Option<PathBuf>, catalog: Option<PathBuf>) -> Graphs {
        let manifest_path = manifest.unwrap_or_else(|| root.join("target").join("manifest.json"));
        Graphs { root, manifest_path, catalog, manifest: None, base: None, loads: Loads::default() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn manifest_path(&self) -> &Path {
        &self.manifest_path
    }

    /// The catalog named when the server started, if one was.
    pub fn catalog(&self) -> Option<&Path> {
        self.catalog.as_deref()
    }

    pub fn loads(&self) -> Loads {
        self.loads
    }

    /// Lets go of both graphs, for a call that panicked while it held one.
    pub fn forget(&mut self) {
        self.manifest = None;
        self.base = None;
    }

    /// Every file the manifest graph's load reads: the manifest, the project
    /// file (`project::name`, `project::dispatch`) and `selectors.yml`
    /// (`selectors::check_against`).
    fn manifest_files(&self) -> Vec<PathBuf> {
        vec![self.manifest_path.clone(), self.root.join("dbt_project.yml"), self.root.join("selectors.yml")]
    }

    /// Those, and what the base graph adds: the catalog in use, and the folders
    /// the packages are placed from (`Graph::place_packages`).
    fn base_files(&self) -> Vec<PathBuf> {
        let mut files = self.manifest_files();
        files.push(ls::catalog_path(&self.root, &self.manifest_path, self.catalog.as_deref()));
        files.extend(macros::PACKAGE_DIRS.iter().map(|dir| self.root.join(dir)));
        files
    }

    /// The manifest alone, loaded when no call has yet or a file it reads has
    /// moved: what `ls::open` gives `--select` and `--column`. The graph is
    /// shared, so `--column` clones it before merging its cache.
    pub fn manifest(&mut self) -> Result<&Graph, String> {
        self.refresh_manifest()?;
        self.manifest.as_ref().map(|k| &k.graph).ok_or_else(|| "error: the manifest is not loaded".to_string())
    }

    /// The manifest with the packages placed and the catalog merged, as
    /// `ls::open_with_catalog` gives `--describe` and `--overview`. It is built
    /// from a copy of the manifest graph, so the manifest is parsed once.
    pub fn base(&mut self) -> Result<Base<'_>, String> {
        self.refresh_base()?;
        self.base
            .as_ref()
            .map(|k| Base { graph: &k.graph, warning: k.warning.as_deref() })
            .ok_or_else(|| "error: the manifest is not loaded".to_string())
    }

    fn refresh_manifest(&mut self) -> Result<(), String> {
        let stamps: Vec<Stamp> = self.manifest_files().iter().map(|p| stamp(p)).collect();
        if self.manifest.as_ref().is_some_and(|k| k.stamps == stamps) {
            return Ok(());
        }
        // Emptied first: a load that fails leaves no graph of the old file.
        self.manifest = None;
        let (_, _, graph) = ls::open(&self.root, Some(&self.manifest_path)).map_err(for_the_server)?;
        self.loads.manifest += 1;
        self.manifest = Some(Kept { stamps, graph, warning: None });
        Ok(())
    }

    fn refresh_base(&mut self) -> Result<(), String> {
        // Before anything is read, as the command line does: a catalog named
        // and not there is left out of an answer without a word otherwise.
        if let Some(why) = ls::missing_catalog(self.catalog.as_deref()) {
            self.base = None;
            return Err(why);
        }
        let stamps: Vec<Stamp> = self.base_files().iter().map(|p| stamp(p)).collect();
        if self.base.as_ref().is_some_and(|k| k.stamps == stamps) {
            return Ok(());
        }
        self.base = None;
        self.refresh_manifest()?;
        let Some(manifest) = &self.manifest else { return Err("error: the manifest is not loaded".to_string()) };
        let mut graph = manifest.graph.clone();
        let catalog = ls::catalog_path(&self.root, &self.manifest_path, self.catalog.as_deref());
        let warning = api::add_catalog(&mut graph, &self.root, &catalog);
        self.loads.base += 1;
        self.base = Some(Kept { stamps, graph, warning });
        Ok(())
    }
}

/// An open error with the command line's advice swapped for the server's: a
/// line cannot carry `--manifest`, which is named in the arguments the server
/// was started with.
fn for_the_server(why: String) -> String {
    why.replace("run dbt parse, or pass --manifest", "run dbt parse, or name it in the arguments the server was started with")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// A Jaffle Shop project with a manifest, a catalog, a project file, a
    /// selectors file, and a model of an installed package whose file sits
    /// under `dbt_packages/`.
    fn project(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dbt-edith-mcp-graphs-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("target")).unwrap();
        std::fs::create_dir_all(dir.join("dbt_packages/dbt_utils/models")).unwrap();
        std::fs::write(dir.join("dbt_packages/dbt_utils/models/helper.sql"), "select 1").unwrap();
        std::fs::write(dir.join("dbt_project.yml"), "name: jaffle_shop\nversion: '1.0'\n").unwrap();
        std::fs::write(dir.join("selectors.yml"), "selectors:\n  - name: marts\n    definition: path:models/marts\n").unwrap();
        let manifest = serde_json::json!({
            "metadata": { "project_name": "jaffle_shop", "adapter_type": "duckdb" },
            "nodes": {
                "model.jaffle_shop.stg_orders": {
                    "name": "stg_orders", "resource_type": "model", "package_name": "jaffle_shop",
                    "original_file_path": "models/staging/stg_orders.sql", "fqn": ["jaffle_shop", "staging", "stg_orders"],
                    "config": { "materialized": "view" },
                    "columns": { "order_id": { "name": "order_id", "data_type": "integer" } },
                },
                "model.jaffle_shop.orders": {
                    "name": "orders", "resource_type": "model", "package_name": "jaffle_shop",
                    "original_file_path": "models/marts/orders.sql", "fqn": ["jaffle_shop", "marts", "orders"],
                    "config": { "materialized": "table" },
                    "columns": { "order_id": { "name": "order_id" } },
                },
                "model.dbt_utils.helper": {
                    "name": "helper", "resource_type": "model", "package_name": "dbt_utils",
                    "original_file_path": "models/helper.sql", "fqn": ["dbt_utils", "helper"],
                    "config": { "materialized": "ephemeral" },
                },
            },
            "parent_map": { "model.jaffle_shop.orders": ["model.jaffle_shop.stg_orders"] },
        });
        std::fs::write(dir.join("target/manifest.json"), manifest.to_string()).unwrap();
        std::fs::write(
            dir.join("target/catalog.json"),
            r#"{"nodes":{"model.jaffle_shop.orders":{"columns":{"order_id":{"name":"order_id","type":"BIGINT","index":1}}}}}"#,
        )
        .unwrap();
        dir
    }

    /// Moves a file's or a folder's mtime on, so its stamp differs whatever
    /// the clock's grain.
    fn touch(path: &Path) {
        let file = std::fs::File::open(path).unwrap();
        let was = std::fs::metadata(path).unwrap().modified().unwrap();
        file.set_modified(was + Duration::from_secs(10)).unwrap();
    }

    fn graphs(dir: &Path) -> Graphs {
        Graphs::new(dir.canonicalize().unwrap(), None, None)
    }

    #[test]
    fn nothing_is_read_until_a_call_needs_a_graph() {
        let dir = project("lazy");
        let g = graphs(&dir);
        assert_eq!(g.loads(), Loads::default());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_second_call_does_not_load_again_and_the_base_reuses_the_parse() {
        let dir = project("twice");
        let mut g = graphs(&dir);
        g.manifest().unwrap();
        g.manifest().unwrap();
        assert_eq!(g.loads(), Loads { manifest: 1, base: 0 });
        g.base().unwrap();
        g.base().unwrap();
        g.manifest().unwrap();
        assert_eq!(g.loads(), Loads { manifest: 1, base: 1 }, "the base is made from the manifest already read");
        // Asked for the base first, the manifest is read once for both.
        let mut fresh = graphs(&dir);
        fresh.base().unwrap();
        fresh.manifest().unwrap();
        assert_eq!(fresh.loads(), Loads { manifest: 1, base: 1 });
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn each_file_a_load_reads_reloads_the_slots_that_read_it_and_no_other() {
        let dir = project("stamps");
        let mut g = graphs(&dir);
        g.manifest().unwrap();
        g.base().unwrap();
        let mut at = g.loads();
        // (what moved, whether the manifest slot reads it)
        for (rel, manifest_reads) in [
            ("target/manifest.json", true),
            ("dbt_project.yml", true),
            ("selectors.yml", true),
            ("target/catalog.json", false),
            ("dbt_packages", false),
            ("dbt_modules", false),
        ] {
            if rel == "dbt_modules" {
                // Not there at the start: its appearing counts.
                std::fs::create_dir(dir.join(rel)).unwrap();
            } else {
                touch(&dir.join(rel));
            }
            g.manifest().unwrap();
            g.base().unwrap();
            let now = g.loads();
            assert_eq!(now.manifest, at.manifest + usize::from(manifest_reads), "{rel}: the manifest slot");
            assert_eq!(now.base, at.base + 1, "{rel}: the base slot");
            at = now;
        }
        // Nothing moved: nothing is read.
        g.manifest().unwrap();
        g.base().unwrap();
        assert_eq!(g.loads(), at);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_catalog_named_by_the_server_is_the_one_stamped() {
        let dir = project("named");
        std::fs::write(dir.join("other.json"), r#"{"nodes":{}}"#).unwrap();
        let root = dir.canonicalize().unwrap();
        let mut g = Graphs::new(root.clone(), None, Some(root.join("other.json")));
        g.base().unwrap();
        touch(&dir.join("target/catalog.json"));
        g.base().unwrap();
        assert_eq!(g.loads().base, 1, "the catalog beside the manifest is not the one in use");
        touch(&dir.join("other.json"));
        g.base().unwrap();
        assert_eq!(g.loads().base, 2);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_manifest_made_unreadable_empties_the_slots_and_a_mended_one_answers_again() {
        let dir = project("broken");
        let mut g = graphs(&dir);
        let good = std::fs::read(dir.join("target/manifest.json")).unwrap();
        g.manifest().unwrap();
        g.base().unwrap();
        std::fs::write(dir.join("target/manifest.json"), "{ \"nodes\": ").unwrap();
        let why = g.manifest().err().unwrap();
        assert!(why.starts_with(&format!("error: cannot read {}", dir.canonicalize().unwrap().join("target/manifest.json").display())), "{why}");
        assert!(g.base().is_err(), "the base is not served from the graph of the old file");
        std::fs::remove_file(dir.join("target/manifest.json")).unwrap();
        let why = g.manifest().err().unwrap();
        assert!(why.contains("no manifest at") && why.contains("run dbt parse, or name it in the arguments the server was started with"), "{why}");
        assert!(!why.contains("--manifest"), "a line cannot pass that flag: {why}");
        std::fs::write(dir.join("target/manifest.json"), good).unwrap();
        let before = g.loads();
        g.manifest().unwrap();
        g.base().unwrap();
        assert_eq!(g.loads().manifest, before.manifest + 1);
        assert_eq!(g.loads().base, before.base + 1);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_catalog_named_and_not_there_is_refused_before_the_manifest_is_read() {
        let dir = project("nocatalog");
        let root = dir.canonicalize().unwrap();
        let mut g = Graphs::new(root.clone(), None, Some(root.join("nowhere.json")));
        let why = g.base().err().unwrap();
        assert_eq!(why, format!("error: no catalog at {}", root.join("nowhere.json").display()));
        assert_eq!(g.loads(), Loads::default());
        // The manifest alone does not read a catalog.
        g.manifest().unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_catalog_that_does_not_read_is_a_warning_the_base_keeps() {
        let dir = project("warning");
        std::fs::write(dir.join("target/catalog.json"), "{ not json").unwrap();
        let mut g = graphs(&dir);
        let base = g.base().unwrap();
        assert!(base.warning.is_some_and(|w| w.starts_with("  catalog.json ignored: ")), "{:?}", base.warning);
        std::fs::write(dir.join("target/catalog.json"), r#"{"nodes":{}}"#).unwrap();
        assert_eq!(g.base().unwrap().warning, None, "mended, the warning goes");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// What a mode writes: stdout, then stderr.
    fn said(run: impl FnOnce(&mut Vec<u8>, &mut Vec<u8>) -> std::io::Result<i32>) -> (String, String, i32) {
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = run(&mut out, &mut err).unwrap();
        (String::from_utf8(out).unwrap(), String::from_utf8(err).unwrap(), code)
    }

    #[test]
    fn the_base_made_from_a_copy_answers_as_the_command_lines_own_open_does() {
        let dir = project("same");
        let root = dir.canonicalize().unwrap();
        let (_, manifest_path, opened) = ls::open_with_catalog(&dir, None, None).unwrap();
        let mut g = graphs(&dir);
        let base = g.base().unwrap().graph;
        let names = ["orders", "helper", "stg_orders"].map(String::from);
        for output in [ls::Output::Name, ls::Output::Json] {
            let want = said(|o, e| crate::describe::respond_describe(&root, &opened, &names, true, output, o, e));
            let got = said(|o, e| crate::describe::respond_describe(&root, base, &names, true, output, o, e));
            assert_eq!(got.0, want.0, "describe {output:?}");
            assert!(got.0.contains("helper") && got.0.contains("BIGINT"), "the cards hold the package's model and the catalog's type: {}", got.0);
            assert_eq!(got.2, want.2);
            // The freshness line's age is the one thing the clock moves.
            let strip = |s: &str| s.lines().filter(|l| !l.starts_with("manifest:")).collect::<Vec<_>>().join("\n");
            assert_eq!(strip(&got.1), strip(&want.1), "describe {output:?}");
            let want = said(|o, e| crate::describe::respond_overview(&root, &manifest_path, &opened, None, 3, output, o, e));
            let got = said(|o, e| crate::describe::respond_overview(&root, &manifest_path, base, None, 3, output, o, e));
            assert_eq!(got.0, want.0, "overview {output:?}");
            assert!(got.0.contains("jaffle_shop"), "{}", got.0);
            assert_eq!(got.2, want.2);
        }
        // The packages were placed, which the manifest alone does not do.
        let helper = g.manifest().unwrap().index["model.dbt_utils.helper"] as usize;
        assert_eq!(g.manifest().unwrap().nodes[helper].place, "");
        let helper = g.base().unwrap().graph.index["model.dbt_utils.helper"] as usize;
        assert_eq!(g.base().unwrap().graph.nodes[helper].place, "dbt_packages/dbt_utils/models/helper.sql");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_manifest_graph_answers_a_selection_as_the_command_lines_own_open_does() {
        let dir = project("select");
        let root = dir.canonicalize().unwrap();
        let (_, _, opened) = ls::open(&dir, None).unwrap();
        let mut g = graphs(&dir);
        let query = ls::Query::Line { select: "+orders".to_string(), exclude: String::new() };
        let want = said(|o, e| ls::respond(&root, &opened, &query, ls::Output::Name, o, e));
        let got = said(|o, e| ls::respond(&root, g.manifest().unwrap(), &query, ls::Output::Name, o, e));
        assert_eq!(got.0, want.0);
        assert_eq!(got.0, "orders\nstg_orders\n");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn forgetting_the_graphs_makes_the_next_call_load_them_again() {
        let dir = project("forget");
        let mut g = graphs(&dir);
        g.base().unwrap();
        g.forget();
        g.base().unwrap();
        assert_eq!(g.loads(), Loads { manifest: 2, base: 2 });
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
