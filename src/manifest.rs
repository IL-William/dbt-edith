//! Raw deserialization of a dbt `manifest.json`.
//!
//! Only the fields the UI needs are declared: serde drops everything else while
//! parsing, which keeps a 90 MB manifest from turning into a 1 GB object graph.

use std::collections::{BTreeMap, HashMap};

#[derive(serde::Deserialize, Default)]
pub struct RawManifest {
    #[serde(default)]
    pub metadata: RawMetadata,
    /// Sorted by unique_id, like `sources`, `exposures` and `disabled`:
    /// `Graph::build` numbers the nodes in the order it reads them, and a
    /// HashMap hands them over in a new order on every start.
    #[serde(default)]
    pub nodes: BTreeMap<String, RawNode>,
    #[serde(default)]
    pub sources: BTreeMap<String, RawNode>,
    #[serde(default)]
    pub exposures: BTreeMap<String, RawNode>,
    #[serde(default)]
    pub parent_map: HashMap<String, Vec<String>>,
    /// Nodes dbt parsed but left out of the graph. Their files still exist, so
    /// the editor must be able to jump to them.
    #[serde(default)]
    pub disabled: BTreeMap<String, Vec<RawNode>>,
    /// Every macro dbt could call, its own and the adapter's included, so the
    /// editor can link a call to the right one (`src/macros.rs`).
    #[serde(default)]
    pub macros: HashMap<String, RawMacro>,
    /// The named selectors of `selectors.yml`, as dbt parsed them. Left as a
    /// value because dbt-core and each Fusion release shape it differently,
    /// and a shape this program did not expect must never fail the whole
    /// manifest: `src/selectors.rs` reads what it can and says what it cannot.
    #[serde(default)]
    pub selectors: serde_json::Value,
    /// dbt's unit tests, from 1.8 a section of their own and never a node:
    /// their name and file only, so the last run can name one and open its
    /// YAML (0065).
    #[serde(default)]
    pub unit_tests: BTreeMap<String, RawUnitTest>,
    /// The kinds the graph has no use for, read for their fqn alone: a path
    /// of `dbt_project.yml` that only one of them begins is used (0103).
    #[serde(default)]
    pub metrics: HashMap<String, RawFqn>,
    #[serde(default)]
    pub semantic_models: HashMap<String, RawFqn>,
    #[serde(default)]
    pub saved_queries: HashMap<String, RawFqn>,
    #[serde(default)]
    pub functions: HashMap<String, RawFqn>,
}

#[derive(serde::Deserialize)]
pub struct RawUnitTest {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub original_file_path: String,
    #[serde(default)]
    pub fqn: Vec<String>,
    /// The model it tests, which is all dbt-core puts in `depends_on.nodes`
    /// (0116). An Option: a null in a section read for the first time here
    /// must not cost the whole manifest.
    #[serde(default)]
    pub depends_on: Option<RawDependsOn>,
}

#[derive(serde::Deserialize)]
pub struct RawFqn {
    #[serde(default)]
    pub fqn: Vec<String>,
}

#[derive(serde::Deserialize, Default)]
pub struct RawMetadata {
    #[serde(default)]
    pub dbt_version: String,
    #[serde(default)]
    pub project_name: String,
    #[serde(default)]
    pub generated_at: String,
    #[serde(default)]
    pub adapter_type: String,
    /// Which dbt command wrote it, to tell whether the last run's results
    /// were written by the same one (0065).
    #[serde(default)]
    pub invocation_id: String,
}

#[derive(serde::Deserialize)]
pub struct RawNode {
    pub name: String,
    pub resource_type: String,
    #[serde(default)]
    pub package_name: String,
    #[serde(default)]
    pub original_file_path: String,
    /// The package name, then the path under the resource root, then the node
    /// name. The implicit selector method matches on it (0024).
    #[serde(default)]
    pub fqn: Vec<String>,
    /// A versioned model's version, and the one a `ref()` naming no version
    /// reaches. A number or a string, as dbt allows either (0104).
    #[serde(default)]
    pub version: Option<serde_json::Value>,
    #[serde(default)]
    pub latest_version: Option<serde_json::Value>,
    #[serde(default)]
    pub patch_path: Option<String>,
    #[serde(default)]
    pub schema: Option<String>,
    #[serde(default)]
    pub database: Option<String>,
    #[serde(default)]
    pub source_name: Option<String>,
    #[serde(default)]
    pub alias: Option<String>,
    /// Sources only: the table name in the warehouse.
    #[serde(default)]
    pub identifier: Option<String>,
    /// Sources only: database and schema exactly as written, Jinja included.
    #[serde(default)]
    pub unrendered_database: Option<String>,
    #[serde(default)]
    pub unrendered_schema: Option<String>,
    /// Models: config as written, before env vars and macros are evaluated.
    #[serde(default)]
    pub unrendered_config: RawPlaceConfig,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub relation_name: Option<String>,
    #[serde(default)]
    pub config: RawConfig,
    #[serde(default)]
    pub columns: HashMap<String, RawColumn>,
    #[serde(default)]
    pub depends_on: RawDependsOn,
    /// The node's SQL as written, read for the macro calls dbt did not
    /// record because it never rendered them when it parsed (0105), and
    /// kept by nothing once read.
    #[serde(default)]
    pub raw_code: Option<String>,
    /// Where dbt wrote this node's compiled SQL, project-relative. The only
    /// thing that knows the file name of a generic test: dbt truncates a long
    /// generated name and appends a hash, which nothing here could reconstruct.
    /// Absent from some manifests, so the layout is still derived when it is.
    #[serde(default)]
    pub compiled_path: Option<String>,
    // Test-only fields, used to attach a test to the column it guards.
    #[serde(default)]
    pub column_name: Option<String>,
    #[serde(default)]
    pub attached_node: Option<String>,
    #[serde(default)]
    pub test_metadata: Option<RawTestMeta>,
    /// Sources only: the threshold dbt resolved for the table, which a table
    /// inherits from its source. Some even when written `null`, which says
    /// there is none; None when the key is absent, as in a manifest of
    /// another producer (0116).
    #[serde(default, deserialize_with = "present")]
    pub freshness: Option<Box<serde_json::Value>>,
}

#[derive(serde::Deserialize, Default)]
pub struct RawTestMeta {
    #[serde(default)]
    pub name: String,
    /// The package the generic was named through, `dbt_utils` for
    /// `dbt_utils.unique_combination_of_columns`. Null when it was written
    /// bare, which says nothing about where it is defined.
    #[serde(default)]
    pub namespace: Option<String>,
}

#[derive(serde::Deserialize, Default)]
pub struct RawConfig {
    #[serde(default)]
    pub materialized: Option<String>,
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub incremental_strategy: Option<String>,
    /// A snapshot's strategy, which names the macro dbt runs for it (0105).
    #[serde(default)]
    pub strategy: Option<serde_json::Value>,
    /// The hooks' SQL, read for the macro calls dbt Fusion does not record
    /// (0105): a list of `{ sql }`, or of plain strings.
    #[serde(default, rename = "pre-hook")]
    pub pre_hook: Option<serde_json::Value>,
    #[serde(default, rename = "post-hook")]
    pub post_hook: Option<serde_json::Value>,
    #[serde(default)]
    pub unique_key: Option<serde_json::Value>,
    #[serde(default)]
    pub database: Option<serde_json::Value>,
    #[serde(default)]
    pub schema: Option<serde_json::Value>,
    #[serde(default)]
    pub alias: Option<serde_json::Value>,
    /// Tests only. A Value, like the location keys: a shape this build does
    /// not expect costs one test its warn mark, not the whole manifest.
    #[serde(default)]
    pub severity: Option<serde_json::Value>,
    /// Tests only, the rest of what changes how one runs, read as Values for
    /// the same reason, and boxed: every node of the manifest passes through
    /// this struct, and a box is a quarter of a Value's size when absent.
    #[serde(default)]
    pub store_failures: Option<Box<serde_json::Value>>,
    #[serde(default)]
    pub store_failures_as: Option<Box<serde_json::Value>>,
    #[serde(default, rename = "where")]
    pub where_: Option<Box<serde_json::Value>>,
    #[serde(default)]
    pub limit: Option<Box<serde_json::Value>>,
    #[serde(default)]
    pub fail_calc: Option<Box<serde_json::Value>>,
    #[serde(default)]
    pub warn_if: Option<Box<serde_json::Value>>,
    #[serde(default)]
    pub error_if: Option<Box<serde_json::Value>>,
    /// Sources only, where dbt-core also writes it, read when the top-level
    /// key is absent (0116).
    #[serde(default, deserialize_with = "present")]
    pub freshness: Option<Box<serde_json::Value>>,
}

/// A value kept when its key is there, `null` included, which `Option` alone
/// reads as absent.
fn present<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Box<serde_json::Value>>, D::Error> {
    Ok(Some(Box::new(serde::Deserialize::deserialize(d)?)))
}

/// Whether a freshness threshold counts for dbt-core, as its `__bool__` has it:
/// `warn_after` or `error_after` holds both a count and a period. A half
/// threshold, `{}` and `null` are none (0116).
pub fn has_freshness(v: &serde_json::Value) -> bool {
    ["warn_after", "error_after"].iter().any(|k| {
        v.get(k).is_some_and(|t| t.get("count").is_some_and(|c| !c.is_null()) && t.get("period").is_some_and(|p| !p.is_null()))
    })
}

/// The three location keys, and nothing else, out of `unrendered_config`.
#[derive(serde::Deserialize, Default)]
pub struct RawPlaceConfig {
    #[serde(default)]
    pub database: Option<serde_json::Value>,
    #[serde(default)]
    pub schema: Option<serde_json::Value>,
    #[serde(default)]
    pub alias: Option<serde_json::Value>,
}

#[derive(serde::Deserialize, Default)]
pub struct RawColumn {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub data_type: Option<String>,
}

#[derive(serde::Deserialize, Default)]
pub struct RawDependsOn {
    #[serde(default)]
    pub nodes: Vec<String>,
    /// The macros it calls, as dbt recorded them: its own calls, not those of
    /// the macros it calls, with `adapter.dispatch` already resolved for the
    /// manifest's adapter. Counted per macro by `src/macro_use.rs` (0105).
    #[serde(default, deserialize_with = "listed")]
    pub macros: Vec<String>,
}

/// A list that may be null, read as empty: a field read here for the first
/// time must not cost the whole manifest when some dbt writes null for it.
fn listed<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<String>, D::Error> {
    let list: Option<Vec<String>> = serde::Deserialize::deserialize(d)?;
    Ok(list.unwrap_or_default())
}

/// A macro, with `macro_sql` only for the few that say which incremental
/// strategy dbt runs: the body is the bulk of the entry. Every field is an
/// Option where a node has plain strings, because a null in a section this
/// program never read before must not cost the whole manifest, lineage
/// included.
#[derive(serde::Deserialize, Default)]
pub struct RawMacro {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub package_name: Option<String>,
    /// Relative to the project for the root project's macros, and to the
    /// package's own directory for anything installed.
    #[serde(default)]
    pub original_file_path: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    /// Only the ones documented in YAML: dbt never reads a macro's signature.
    #[serde(default)]
    pub arguments: Option<Vec<RawMacroArg>>,
    #[serde(default, deserialize_with = "strategy_body")]
    pub macro_sql: Option<String>,
    /// The macros this one calls (`RawDependsOn::macros`).
    #[serde(default)]
    pub depends_on: Option<RawDependsOn>,
}

/// What a body mentions when it can bear on an incremental model's strategy
/// (`src/macros.rs`): the materialization, the strategy macros and the merge
/// macros they call, by their own names, and whatever reads the configured
/// strategy, for a materialization that reads it one call deep.
const STRATEGY_WORDS: &[&str] = &[
    "materialization incremental",
    "get_incremental_",
    "get_merge_sql",
    "get_delete_insert_merge_sql",
    "incremental_strategy",
];

/// A body kept only when it may say something about the strategy. Each one is
/// read whole all the same, but dropped at once, so no more than one other
/// body is held at a time.
fn strategy_body<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    let body: Option<String> = serde::Deserialize::deserialize(d)?;
    Ok(body.filter(|b| STRATEGY_WORDS.iter().any(|w| b.contains(w))))
}

#[derive(serde::Deserialize, Default)]
pub struct RawMacroArg {
    #[serde(default)]
    pub name: Option<String>,
}

/// `catalog.json`, written by `dbt docs generate`. Optional: it carries the real
/// warehouse column types, which the manifest only has when declared in YAML.
#[derive(serde::Deserialize, Default)]
pub struct RawCatalog {
    #[serde(default)]
    pub nodes: HashMap<String, CatalogNode>,
    #[serde(default)]
    pub sources: HashMap<String, CatalogNode>,
}

#[derive(serde::Deserialize, Default)]
pub struct CatalogNode {
    /// Sorted, so two columns whose names differ only in case, which a quoted
    /// identifier allows, meet `merge_catalog` in the same order every time.
    #[serde(default)]
    pub columns: BTreeMap<String, CatalogColumn>,
}

#[derive(serde::Deserialize, Default)]
pub struct CatalogColumn {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub r#type: String,
    #[serde(default)]
    pub comment: Option<String>,
}

impl RawCatalog {
    pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        let bytes = std::fs::read(path)?;
        Ok(serde_json::from_slice(&bytes)?)
    }
}

impl RawManifest {
    pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        let bytes = std::fs::read(path)?;
        Ok(serde_json::from_slice(&bytes)?)
    }
}
