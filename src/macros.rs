//! Macros, as the manifest lists them, and which one a call in the editor
//! reaches.
//!
//! A call is resolved by name, the way dbt resolves it for a node: `pkg.name`
//! is that package's macro, and a bare `name` is the calling package's own,
//! then the root project's. The order matters in practice, because a project
//! can define a macro under a package macro's name: a bare `hub` then reaches
//! the project's own, while `automate_dv.hub` still reaches the package's. An
//! installed package is never searched for a bare name, since dbt never
//! searches one either, and dbt's own macros come last in dbt's order but have
//! no file here, so they need no place in this one.
//!
//! The invariant: a macro is linked only to a file that exists inside the
//! project. The manifest gives a package macro's path relative to the package,
//! so it is looked for where `dbt deps` puts packages, and a macro of dbt's own,
//! which lives in site-packages, is never found and stays plain text. A custom
//! `packages-install-path` is the known gap.

use crate::manifest::RawMacro;
use std::collections::HashMap;
use std::path::Path;

/// Where `dbt deps` installs a package, in a folder named after it: the name
/// dbt has used since 1.0, then the one before it.
const PACKAGE_DIRS: &[&str] = &["dbt_packages", "dbt_modules"];

#[derive(Clone, Debug)]
pub struct Macro {
    pub id: String,
    pub name: String,
    pub package: String,
    /// `original_file_path`, slashed: relative to the project for the root
    /// project's macros, relative to the package for an installed one's.
    pub path: String,
    pub description: String,
    /// The arguments the YAML documents, which is all the manifest knows.
    pub args: Vec<String>,
}

#[derive(Clone, Default)]
pub struct Macros {
    list: Vec<Macro>,
    /// Package, then macro name, to a position in `list`. dbt refuses two
    /// macros of one name in one package, so the pair is a key.
    by_package: HashMap<String, HashMap<String, u32>>,
    /// The root project's name, which is the package its macros carry.
    root: String,
}

impl Macros {
    pub fn build(raw: HashMap<String, RawMacro>, root: &str) -> Macros {
        let mut list = Vec::with_capacity(raw.len());
        let mut by_package: HashMap<String, HashMap<String, u32>> = HashMap::new();
        for (id, m) in raw {
            let (Some(name), Some(package)) = (m.name, m.package_name) else { continue };
            if name.is_empty() {
                continue;
            }
            let slot = by_package.entry(package.clone()).or_default();
            if slot.contains_key(&name) {
                continue;
            }
            slot.insert(name.clone(), list.len() as u32);
            list.push(Macro {
                id,
                name,
                package,
                path: crate::graph::slashed(m.original_file_path.unwrap_or_default()),
                description: m.description.unwrap_or_default(),
                args: m
                    .arguments
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|a| a.name)
                    .filter(|a| !a.is_empty())
                    .collect(),
            });
        }
        Macros { list, by_package, root: root.to_string() }
    }

    pub fn len(&self) -> usize {
        self.list.len()
    }

    /// Whether the manifest named the root project. dbt-core wrote the name
    /// from 1.6 on; before, `load_graph` takes it from `dbt_project.yml`.
    pub fn has_root(&self) -> bool {
        !self.root.is_empty()
    }

    pub fn set_root(&mut self, name: String) {
        self.root = name;
    }

    fn get(&self, package: &str, name: &str) -> Option<&Macro> {
        let &i = self.by_package.get(package)?.get(name)?;
        Some(&self.list[i as usize])
    }

    /// The macro `call` reaches from a file of package `local`. Anything with
    /// more than one dot is an attribute of something, never a macro.
    pub fn resolve(&self, call: &str, local: &str) -> Option<&Macro> {
        match call.split_once('.') {
            Some((package, name)) if !name.contains('.') => self.get(package, name),
            Some(_) => None,
            None => self.get(local, call).or_else(|| self.get(&self.root, call)),
        }
    }

    /// The package a project file belongs to: the installed one whose folder
    /// it sits in, or the root project.
    pub fn package_of<'a>(&'a self, file: &'a str) -> &'a str {
        let mut parts = file.split(['/', '\\']).filter(|p| !p.is_empty() && *p != ".");
        match (parts.next(), parts.next(), parts.next()) {
            (Some(dir), Some(package), Some(_)) if PACKAGE_DIRS.contains(&dir) => package,
            _ => &self.root,
        }
    }

    /// Where `m` is written inside the project, or None when it is not there:
    /// a macro of dbt's own, a package not installed, or a file the manifest
    /// remembers and a later commit moved.
    pub fn place(&self, project: &Path, m: &Macro) -> Option<String> {
        if m.path.is_empty() {
            return None;
        }
        let candidates: Vec<String> = if m.package == self.root {
            vec![m.path.clone()]
        } else {
            PACKAGE_DIRS.iter().map(|dir| format!("{dir}/{}/{}", m.package, m.path)).collect()
        };
        // Through files::resolve, so a macro path climbing out with `..`, or a
        // package symlinked in from elsewhere, is refused like any other path.
        candidates
            .into_iter()
            .find(|rel| crate::files::resolve(project, rel).is_ok_and(|p| p.is_file()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::RawManifest;

    fn macros() -> Macros {
        let m = |name: &str, package: &str, path: &str| {
            serde_json::json!({
                "name": name,
                "package_name": package,
                "original_file_path": path,
                "macro_sql": "{% macro x() %}{% endmacro %}",
            })
        };
        let raw: RawManifest = serde_json::from_value(serde_json::json!({
            "metadata": { "project_name": "shop" },
            "macros": {
                "macro.shop.hub": m("hub", "shop", "macros\\vault\\hub.sql"),
                "macro.shop.cents": m("cents", "shop", "macros/money.sql"),
                "macro.automate_dv.hub": m("hub", "automate_dv", "macros/tables/snowflake/hub.sql"),
                "macro.automate_dv.prefix": m("prefix", "automate_dv", "macros/internal/prefix.sql"),
                "macro.dbt_utils.star": m("star", "dbt_utils", "macros/sql/star.sql"),
                "macro.dbt.is_incremental": m("is_incremental", "dbt", "macros/materializations/is_incremental.sql"),
            },
        }))
        .unwrap();
        crate::graph::Graph::build(raw, Path::new("manifest.json"), 0, 0).macros
    }

    fn id(found: Option<&Macro>) -> &str {
        found.map(|m| m.id.as_str()).unwrap_or("none")
    }

    #[test]
    fn a_bare_name_reaches_the_project_before_a_package_of_the_same_name() {
        let m = macros();
        assert_eq!(id(m.resolve("hub", "shop")), "macro.shop.hub");
        assert_eq!(id(m.resolve("automate_dv.hub", "shop")), "macro.automate_dv.hub");
        // Qualifying with the project's own name is legal dbt, and means the same.
        assert_eq!(id(m.resolve("shop.hub", "shop")), "macro.shop.hub");
    }

    #[test]
    fn a_bare_name_never_reaches_an_installed_package() {
        let m = macros();
        // dbt answers `'star' is undefined` for this call, so a link would lie.
        assert_eq!(id(m.resolve("star", "shop")), "none");
        assert_eq!(id(m.resolve("dbt_utils.star", "shop")), "macro.dbt_utils.star");
        assert_eq!(id(m.resolve("dbt_utils.nothing", "shop")), "none");
        assert_eq!(id(m.resolve("adapter.dispatch", "shop")), "none");
        assert_eq!(id(m.resolve("a.b.c", "shop")), "none");
    }

    #[test]
    fn inside_a_package_its_own_macros_come_first() {
        let m = macros();
        let local = m.package_of("dbt_packages/automate_dv/macros/tables/snowflake/sat.sql");
        assert_eq!(local, "automate_dv");
        assert_eq!(id(m.resolve("hub", local)), "macro.automate_dv.hub");
        assert_eq!(id(m.resolve("prefix", local)), "macro.automate_dv.prefix");
        // And the root project still answers what the package does not define.
        assert_eq!(id(m.resolve("cents", local)), "macro.shop.cents");
    }

    #[test]
    fn a_file_belongs_to_the_root_project_unless_it_sits_in_an_installed_package() {
        let m = macros();
        assert_eq!(m.package_of("models/marts/orders.sql"), "shop");
        assert_eq!(m.package_of("./dbt_packages/dbt_utils/macros/sql/star.sql"), "dbt_utils");
        assert_eq!(m.package_of("dbt_modules\\dbt_utils\\macros\\star.sql"), "dbt_utils");
        // The folder itself, or a file loose in it, belongs to no package.
        assert_eq!(m.package_of("dbt_packages/README.md"), "shop");
        assert_eq!(m.package_of(""), "shop");
    }

    fn project(tag: &str, files: &[&str]) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("dbt-edith-macros-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for f in files {
            let p = dir.join(f);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(&p, "{% macro x() %}{% endmacro %}\n").unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();
        dir.canonicalize().unwrap()
    }

    #[test]
    fn a_macro_is_placed_where_its_file_really_is() {
        let m = macros();
        let dir = project("place", &["macros/vault/hub.sql", "dbt_modules/dbt_utils/macros/sql/star.sql"]);
        let place = |call: &str| m.resolve(call, "shop").and_then(|found| m.place(&dir, found));
        // A manifest parsed on Windows still places a project macro.
        assert_eq!(place("hub").as_deref(), Some("macros/vault/hub.sql"));
        // A package is found under the older folder name too.
        assert_eq!(place("dbt_utils.star").as_deref(), Some("dbt_modules/dbt_utils/macros/sql/star.sql"));
        // Resolved, and still no link: not installed, or not in the project at all.
        assert_eq!(place("automate_dv.hub"), None);
        assert_eq!(place("dbt.is_incremental"), None);
        // A file the manifest remembers and the disk no longer has.
        assert_eq!(place("cents"), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_macro_path_cannot_leave_the_project() {
        // The file exists, one level above the project, so only the refusal
        // keeps it out.
        let base = project("escape", &["shop/macros/a.sql", "elsewhere/x.sql"]);
        let outside = Macro {
            id: "macro.shop.x".into(),
            name: "x".into(),
            package: "shop".into(),
            path: "../elsewhere/x.sql".into(),
            description: String::new(),
            args: Vec::new(),
        };
        assert!(base.join("shop").join(&outside.path).is_file());
        assert_eq!(macros().place(&base.join("shop"), &outside), None);
        let _ = std::fs::remove_dir_all(&base);
    }

    /// dbt-core before 1.6 writes no `metadata.project_name`, only a hash of it,
    /// so the root package is named by `dbt_project.yml` instead.
    #[test]
    fn a_manifest_that_names_no_project_takes_the_name_from_its_project_file() {
        let dir = project("unnamed", &["macros/money.sql"]);
        std::fs::write(dir.join("dbt_project.yml"), "name: 'shop'\nversion: '1.0'\n").unwrap();
        std::fs::create_dir_all(dir.join("target")).unwrap();
        let manifest = dir.join("target").join("manifest.json");
        let json = serde_json::json!({
            "metadata": { "dbt_version": "1.5.0", "project_id": "0f3e3a5d" },
            "macros": {
                "macro.shop.cents": { "name": "cents", "package_name": "shop", "original_file_path": "macros/money.sql" },
            },
        });
        std::fs::write(&manifest, json.to_string()).unwrap();
        let none = dir.join("target").join("none.json");
        let g = crate::api::load_graph(&dir, &manifest, &none, &none).unwrap();
        let local = g.macros.package_of("models/orders.sql");
        let cents = g.macros.resolve("cents", local).expect("a bare call reaches the project");
        assert_eq!(g.macros.place(&dir, cents).as_deref(), Some("macros/money.sql"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_null_in_a_macro_costs_that_macro_and_nothing_else() {
        let raw: RawManifest = serde_json::from_value(serde_json::json!({
            "metadata": { "project_name": "shop" },
            "nodes": { "model.shop.orders": { "name": "orders", "resource_type": "model" } },
            "macros": {
                "macro.shop.odd": { "name": null, "package_name": "shop" },
                "macro.shop.cents": {
                    "name": "cents", "package_name": "shop", "original_file_path": null,
                    "description": null, "arguments": null,
                },
                "macro.shop.fx": {
                    "name": "fx", "package_name": "shop", "original_file_path": "macros/fx.sql",
                    "description": "Converts to euros.", "arguments": [{ "name": "amount" }, { "name": null }],
                },
            },
        }))
        .unwrap();
        let g = crate::graph::Graph::build(raw, Path::new("manifest.json"), 0, 0);
        assert_eq!(g.nodes.len(), 1, "the lineage still loads");
        assert_eq!(g.macros.len(), 2);
        let fx = g.macros.resolve("fx", "shop").unwrap();
        assert_eq!(fx.description, "Converts to euros.");
        assert_eq!(fx.args, vec!["amount"]);
        assert_eq!(g.macros.resolve("cents", "shop").unwrap().path, "");
    }
}
