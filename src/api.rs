//! HTTP + WebSocket surface. Everything the browser UI talks to lives here.

use crate::collin::{self, RawColLineage};
use crate::compiled;
use crate::envs;
use crate::files::{self, mtime_secs};
use crate::freshness;
use crate::git::{self, GitInfo};
use crate::graph::{ColRef, Graph, Kind, Place};
use crate::manifest::{RawCatalog, RawManifest};
use crate::pty::{FromPty, PtySession, ShellSpec};
use crate::select;
use crate::selectors;
use crate::sidecar;
use crate::venv::VenvInfo;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::http::{header, Method, StatusCode, Uri};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct AppState {
    /// The port actually bound. Host and Origin are checked against it.
    pub port: u16,
    pub root: PathBuf,
    pub manifest_path: PathBuf,
    pub catalog_path: PathBuf,
    /// The cache currently merged into the graph. Changeable, because the
    /// project can hold one file per producer and the user picks which one.
    pub cll_path: std::sync::RwLock<PathBuf>,
    /// Where dbt writes its artifacts; the compiled SQL lives under it.
    pub target_dir: PathBuf,
    pub venv: VenvInfo,
    pub file_index: RwLock<Arc<Vec<String>>>,
    pub settings: crate::settings::Store,
    /// The graph as drawn: `base` with the chosen column lineage merged in.
    pub graph: RwLock<Arc<Graph>>,
    /// The manifest and the catalog alone, never any column lineage. Switching
    /// caches merges into a copy of it rather than reading the manifest again,
    /// and must: merging adds the columns a cache knows and the YAML does not,
    /// and never takes them away, so a cache laid over another would keep the
    /// first one's columns. Replaced with `graph` whenever the manifest or the
    /// catalog is read again.
    pub base: RwLock<Arc<Graph>>,
    /// Headers of the caches beside the manifest, read again only when a file
    /// changes, since listing them is what opening the menu costs.
    pub cll_headers: collin::Headers,
    pub shell: ShellSpec,
    pub git: tokio::sync::Mutex<Option<(std::time::Instant, GitInfo)>>,
    /// Same idea as `git`, for the same reason: the badge polls on a timer and
    /// each answer costs a walk of the resource directories.
    pub fresh: tokio::sync::Mutex<Option<(std::time::Instant, freshness::Freshness)>>,
    /// The Snowflake script, running only while Snowflake is the picked tool (0016).
    pub sidecar: sidecar::Sidecar,
    /// The user's choice of Snowflake's features for this project, None until
    /// there is one. Held here as well as in the settings, so the choice lasts
    /// the session when there is no configuration directory to keep it in.
    pub snowflake_features: std::sync::Mutex<Option<bool>>,
    /// Held while the column-lineage cache is merged and written, and while the
    /// watcher reloads: a click's edges must land in the graph that stays.
    pub cll_lock: tokio::sync::Mutex<()>,
    /// Modification times of manifest, catalog and cache already acted on. A
    /// click records the cache it wrote, or the watcher would re-read the whole
    /// manifest for it three seconds later.
    pub seen: std::sync::Mutex<[u64; 3]>,
}

impl AppState {
    /// The active cache path. Cloned rather than borrowed: the lock must not be
    /// held across an await, and every caller wants an owned path anyway.
    pub fn cll(&self) -> PathBuf {
        self.cll_path.read().map(|p| p.clone()).unwrap_or_default()
    }
    pub fn set_cll(&self, path: PathBuf) {
        if let Ok(mut slot) = self.cll_path.write() {
            *slot = path;
        }
    }

    /// Whether Snowflake's features are on (0031). The adapter is read from the
    /// graph each time, because a manifest rewritten by another dbt can change it.
    pub async fn snowflake_allowed(&self) -> bool {
        let chosen = self.snowflake_features.lock().map(|c| *c).unwrap_or(None);
        let adapter = self.graph.read().await.meta.adapter.clone();
        crate::settings::snowflake_features(chosen, &adapter)
    }
}

/// Why a Snowflake route answers nothing (0031).
const SNOWFLAKE_OFF: &str = "Snowflake features are off for this project: switch them on in the settings menu";

#[derive(rust_embed::RustEmbed)]
#[folder = "web/"]
struct Assets;

/// The manifest with the catalog merged, and no column lineage: what `base`
/// holds.
///
/// `project` is read only for what a manifest can leave out: dbt-core before
/// 1.6 does not name the root project, and its macros are found by that name.
pub fn load_base(project: &Path, manifest_path: &Path, catalog_path: &Path) -> anyhow::Result<Graph> {
    let started = std::time::Instant::now();
    let mtime = mtime_secs(manifest_path);
    let raw = RawManifest::load(manifest_path)?;
    let mut graph = Graph::build(raw, manifest_path, mtime, started.elapsed().as_millis());
    if !graph.macros.has_root() {
        if let Some(name) = crate::project::name(project) {
            graph.macros.set_root(name);
        }
    }
    graph.selectors.check_against(project);
    if catalog_path.exists() {
        match RawCatalog::load(catalog_path) {
            Ok(cat) => {
                let n = graph.merge_catalog(cat, mtime_secs(catalog_path));
                graph.meta.catalog_columns = n;
                graph.meta.catalog_mtime = graph.catalog_mtime;
            }
            Err(e) => eprintln!("  catalog.json ignored: {e}"),
        }
    }
    graph.meta.load_ms = started.elapsed().as_millis();
    Ok(graph)
}

/// `base` with one column-lineage cache merged into a copy of it. Copying
/// 18 825 nodes is a fraction of reading the 110 MB manifest they came from.
pub fn with_cache(base: &Graph, cll_path: &Path) -> Graph {
    let mut graph = base.clone();
    merge_cache(&mut graph, cll_path);
    graph
}

/// Reads the manifest and the catalog again, lays the cache on screen over
/// them, and replaces both graphs. The caller holds `cll_lock`, so a switch or
/// a fetch cannot land between the two.
async fn load_all(st: &AppState) -> anyhow::Result<Arc<Graph>> {
    let (root, mp, cp, lp) = (st.root.clone(), st.manifest_path.clone(), st.catalog_path.clone(), st.cll());
    let (base, graph) = tokio::task::spawn_blocking(move || -> anyhow::Result<(Graph, Graph)> {
        let base = load_base(&root, &mp, &cp)?;
        let graph = with_cache(&base, &lp);
        Ok((base, graph))
    })
    .await??;
    // What was just read is what the watcher has acted on, so it does not
    // read it all again three seconds later.
    if let Ok(mut seen) = st.seen.lock() {
        *seen = [graph.meta.manifest_mtime, graph.meta.catalog_mtime, graph.meta.cll_mtime];
    }
    let graph = Arc::new(graph);
    *st.base.write().await = Arc::new(base);
    *st.graph.write().await = graph.clone();
    Ok(graph)
}

/// The caches beside the manifest, headers read only where a file changed.
async fn caches(st: &Arc<AppState>) -> Vec<collin::Available> {
    let st = st.clone();
    tokio::task::spawn_blocking(move || collin::discover_cached(&st.target_dir, &st.cll_headers)).await.unwrap_or_default()
}

/// Merges one column-lineage cache, if the file is there. An empty path is how
/// "no column lineage" is spelled, and merges nothing.
///
/// After merge_catalog, never before: both re-sort Node.columns and the column
/// slots recorded by the lineage merge are positions in that final order.
pub fn merge_cache(graph: &mut Graph, cll_path: &Path) {
    if cll_path.exists() {
        match RawColLineage::load(cll_path) {
            Ok(raw) => {
                let n = graph.merge_col_lineage(raw, mtime_secs(cll_path));
                graph.meta.cll_file = cll_path.display().to_string();
                eprintln!("  column lineage merged ({n} edges)");
            }
            Err(e) => eprintln!("  column lineage ignored: {e}"),
        }
    }
}

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/meta", get(meta))
        .route("/api/search", get(search))
        .route("/api/node", get(node))
        .route("/api/envs", get(list_envs).put(save_envs))
        .route("/api/envs/select", post(select_env))
        .route("/api/vars", get(list_vars))
        .route("/api/compiled", get(compiled_sql))
        .route("/api/lineage", get(lineage))
        .route("/api/select", get(selection))
        .route("/api/selectors", get(list_selectors))
        .route("/api/collineage", get(col_lineage))
        .route("/api/collineage/source", post(select_cll_source))
        .route("/api/collineage/fetch", post(fetch_col_lineage))
        .route("/api/sidecar", get(sidecar_status))
        .route("/api/features", post(set_features))
        .route("/api/profiles", get(read_profile).put(write_profile))
        .route("/api/dir", get(dir))
        .route("/api/files", get(file_search))
        .route("/api/grep", get(grep))
        .route("/api/file", get(read_file).put(write_file))
        .route("/api/resolve", post(resolve))
        .route("/api/macros/resolve", post(resolve_macros))
        .route("/api/git", get(git_status))
        .route("/api/freshness", get(freshness_status))
        .route("/api/git/branches", get(git_branches))
        .route("/api/git/diff", get(git_diff))
        .route("/api/git/outgoing", get(git_outgoing))
        .route("/api/git/checkout", post(git_checkout))
        .route("/api/git/stage", post(git_stage))
        .route("/api/git/unstage", post(git_unstage))
        .route("/api/git/commit", post(git_commit))
        .route("/api/git/push", post(git_push))
        .route("/api/git/pull", post(git_pull))
        .route("/api/git/fetch", post(git_fetch))
        .route("/api/git/merge-abort", post(git_merge_abort))
        .route("/api/reload", post(reload))
        .route("/ws/pty", get(ws_pty))
        .fallback(static_asset)
        .layer(middleware::from_fn_with_state(state.clone(), guard))
        .with_state(state)
}

// ----------------------------------------------------------------- guard ----
//
// Binding to 127.0.0.1 keeps other machines out, not other web pages. A page
// from any site can open a WebSocket to localhost, can POST here without a
// preflight, and, through DNS rebinding, can become same-origin with this
// server. So every request states where it comes from, and this checks (0015).

/// `Host` must name this server exactly: a rebound domain never does.
fn host_allowed(host: Option<&str>, port: u16) -> bool {
    match host {
        Some(h) => h == format!("127.0.0.1:{port}") || h == format!("localhost:{port}"),
        None => false,
    }
}

/// `Origin` must be this server's own page. Browsers always send it on a
/// WebSocket handshake and on cross-site requests; a missing one is a local
/// tool such as curl, accepted unless `required`.
fn origin_allowed(origin: Option<&str>, port: u16, required: bool) -> bool {
    match origin {
        Some(o) => o == format!("http://127.0.0.1:{port}") || o == format!("http://localhost:{port}"),
        None => !required,
    }
}

fn header_str<'a>(req: &'a axum::extract::Request, name: header::HeaderName) -> Option<&'a str> {
    req.headers().get(name).and_then(|v| v.to_str().ok())
}

/// Carried by every reply. The page loads nothing from anywhere else, so the
/// policy says exactly that, and `frame-ancestors` keeps the terminal out of
/// an invisible frame on someone else's site. `unsafe-inline` for styles is
/// xterm.js, which injects a stylesheet of its own at runtime.
const HEADERS: &[(&str, &str)] = &[
    (
        "content-security-policy",
        concat!(
            "default-src 'none'; ",
            "script-src 'self'; ",
            "style-src 'self' 'unsafe-inline'; ",
            "img-src 'self' data:; ",
            "connect-src 'self' ws://127.0.0.1:* ws://localhost:*; ",
            "base-uri 'none'; form-action 'none'; frame-ancestors 'none'",
        ),
    ),
    ("x-content-type-options", "nosniff"),
    ("referrer-policy", "no-referrer"),
];

async fn guard(State(st): State<Arc<AppState>>, req: axum::extract::Request, next: Next) -> Response {
    let refused = || (StatusCode::FORBIDDEN, "refused: not from this server's own page").into_response();
    if !host_allowed(header_str(&req, header::HOST), st.port) {
        return refused();
    }
    let upgrade = header_str(&req, header::UPGRADE).is_some_and(|u| u.eq_ignore_ascii_case("websocket"));
    let reads_only = matches!(*req.method(), Method::GET | Method::HEAD) && !upgrade;
    if !reads_only && !origin_allowed(header_str(&req, header::ORIGIN), st.port, upgrade) {
        return refused();
    }
    let mut res = next.run(req).await;
    let headers = res.headers_mut();
    for (name, value) in HEADERS {
        headers.insert(
            header::HeaderName::from_static(name),
            header::HeaderValue::from_static(value),
        );
    }
    res
}

// ---------------------------------------------------------------- static ----

async fn static_asset(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };
    match Assets::get(path) {
        Some(file) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            ([(header::CONTENT_TYPE, mime.as_ref())], file.data).into_response()
        }
        None => (StatusCode::NOT_FOUND, "not found").into_response(),
    }
}

// ------------------------------------------------------------------- api ----

fn err(e: impl std::fmt::Display) -> Response {
    (StatusCode::BAD_REQUEST, e.to_string()).into_response()
}

#[derive(serde::Serialize)]
struct MetaBody {
    root: String,
    shell: String,
    /// The package version, and which build it is. A release binary embeds
    /// `web/` (0005), so the page saying which build drew it is the quickest
    /// answer to "my frontend fix did nothing".
    version: &'static str,
    build: &'static str,
    venv: VenvInfo,
    meta: crate::graph::Meta,
    /// Every cache found beside the manifest that may be offered, so the UI can
    /// list them without a second round trip. Headers only: no edge array is
    /// parsed for this.
    cll_sources: Vec<collin::Available>,
    /// File name of the active one, matching one of `cll_sources`, or empty.
    cll_active: String,
    /// The tool whose edges the graph holds: Snowflake whenever it is picked,
    /// even before its first fetch, else the producer of the cache on screen.
    cll_tool: &'static str,
    features: Features,
    sidecar: SidecarBody,
}

#[derive(serde::Serialize)]
struct Features {
    snowflake: bool,
    /// Chosen for this project, rather than following the adapter.
    snowflake_set: bool,
}

/// What the page needs to paint the lineage menu and the settings menu. Every
/// route that changes either answers with it, so the page never paints half of
/// a change.
async fn meta_body(st: &Arc<AppState>) -> MetaBody {
    let graph = st.graph.read().await.clone();
    let found = caches(st).await;
    let chosen = st.snowflake_features.lock().map(|c| *c).unwrap_or(None);
    let snowflake = crate::settings::snowflake_features(chosen, &graph.meta.adapter);
    let cll_active = st
        .cll()
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let live = st.sidecar.enabled();
    MetaBody {
        root: st.root.display().to_string(),
        shell: format!("{} {}", st.shell.program, st.shell.args.join(" ")).trim().to_string(),
        version: env!("CARGO_PKG_VERSION"),
        build: env!("DBT_EDITH_BUILD"),
        venv: st.venv.clone(),
        cll_tool: if live { "snowflake" } else { collin::tool_of(&graph.meta.cll_source) },
        meta: graph.meta.clone(),
        cll_sources: found.into_iter().filter(|a| collin::offered(a, snowflake)).collect(),
        cll_active,
        features: Features { snowflake, snowflake_set: chosen.is_some() },
        sidecar: SidecarBody { enabled: live, status: st.sidecar.status() },
    }
}

async fn meta(State(st): State<Arc<AppState>>) -> Response {
    Json(meta_body(&st).await).into_response()
}

/// Loads `path` as the graph's column lineage, over the manifest already read,
/// under the lock the watcher and a fetch take, and records its time so the
/// watcher does not load it again.
async fn show_cache(st: &AppState, path: PathBuf) -> Result<(), Response> {
    let _cache = st.cll_lock.lock().await;
    st.set_cll(path.clone());
    let base = st.base.read().await.clone();
    let g = tokio::task::spawn_blocking(move || with_cache(&base, &path)).await.map_err(err)?;
    if let Ok(mut seen) = st.seen.lock() {
        seen[2] = g.meta.cll_mtime;
    }
    *st.graph.write().await = Arc::new(g);
    Ok(())
}

/// Switches the script off at once and stops it in the background: stopping
/// waits on a start still in progress, which can take a minute on a cold VM,
/// and nothing should wait on that to show another tool's edges.
fn stop_fetching(st: &Arc<AppState>) {
    st.sidecar.set_enabled(false);
    let st = st.clone();
    tokio::spawn(async move {
        // Picked again meanwhile: the start that follows is not to be undone.
        if !st.sidecar.enabled() {
            st.sidecar.stop().await;
        }
    });
}

#[derive(Deserialize)]
struct CllSourceBody {
    /// One of `collin::TOOLS`.
    #[serde(default)]
    tool: Option<String>,
    /// A file name as `/api/meta` listed it, never a path.
    #[serde(default)]
    file: Option<String>,
}

/// Switches which column lineage the graph holds, and with it whether a column
/// click fetches from Snowflake: one request, so the two cannot disagree (0031).
///
/// The graph can only carry one source at a time: `merge_col_lineage` replaces
/// the edge set rather than adding to it, which is what keeps two producers'
/// answers from being blended into something neither of them said.
///
/// Neither name is joined onto the target directory, so nothing the browser
/// sends can reach another file (0015).
async fn select_cll_source(State(st): State<Arc<AppState>>, Json(b): Json<CllSourceBody>) -> Response {
    let wanted = match (b.tool.as_deref(), b.file.as_deref()) {
        (Some(tool), _) => collin::Wanted::Tool(tool),
        (None, Some(file)) => collin::Wanted::File(file),
        (None, None) => return (StatusCode::BAD_REQUEST, "name a tool or a cache file").into_response(),
    };
    let found = caches(&st).await;
    let (path, live) = match collin::pick(&st.target_dir, &found, wanted, st.snowflake_allowed().await) {
        Ok(chosen) => chosen,
        Err(collin::Refusal::SnowflakeOff) => return (StatusCode::CONFLICT, SNOWFLAKE_OFF).into_response(),
        Err(collin::Refusal::UnknownTool) => return (StatusCode::BAD_REQUEST, "not a column lineage tool").into_response(),
        Err(collin::Refusal::NoCache(why)) => return (StatusCode::NOT_FOUND, why).into_response(),
    };
    if !live {
        stop_fetching(&st);
    }
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned());
    // Remembered like the environment selection. Without a configuration
    // directory the choice still holds until dbt-edith stops.
    if let Err(e) = st
        .settings
        .update(|s| {
            s.cll_file = name;
            s.snowflake_lineage = live;
        })
        .await
    {
        eprintln!("  column lineage choice not saved: {e}");
    }
    if let Err(response) = show_cache(&st, path).await {
        return response;
    }
    if live {
        st.sidecar.set_enabled(true);
        if !st.sidecar.is_up() {
            // Starting checks Python, the profile and the connector, and runs
            // no query (0016). The page polls for the outcome instead of
            // waiting on it here.
            st.sidecar.mark_starting();
            let st = st.clone();
            tokio::spawn(async move {
                if st.sidecar.enabled() {
                    st.sidecar.start_for(&st.root, &st.venv).await;
                }
            });
        }
    }
    Json(meta_body(&st).await).into_response()
}

#[derive(Deserialize)]
struct FeaturesBody {
    /// Optional, so a later feature adds a field rather than a route.
    #[serde(default)]
    snowflake: Option<bool>,
}

/// Switches a family of features on or off for this project (0031). Turning
/// Snowflake's on starts nothing: it only offers Snowflake in the menu. Turning
/// them off stops its script and takes its edges off the graph, which then
/// holds what startup would have loaded with Snowflake off.
async fn set_features(State(st): State<Arc<AppState>>, Json(b): Json<FeaturesBody>) -> Response {
    if let Some(on) = b.snowflake {
        if let Ok(mut chosen) = st.snowflake_features.lock() {
            *chosen = Some(on);
        }
        let saved = st
            .settings
            .update(|s| {
                s.snowflake_features = Some(on);
                if !on {
                    s.snowflake_lineage = false;
                }
            })
            .await;
        if let Err(e) = &saved {
            eprintln!("  snowflake features choice not saved: {e}");
        }
        // A path with no file behind it is chosen again too: it may be where
        // a Snowflake dump would land and be loaded by the watcher.
        let shown = collin::tool_of(&st.graph.read().await.meta.cll_source) == "snowflake";
        if !on && (st.sidecar.enabled() || shown || !st.cll().is_file()) {
            stop_fetching(&st);
            let found = caches(&st).await;
            let remembered = saved.map_or_else(|_| st.settings.load(), |s| s).cll_file;
            let path = collin::choose(&st.target_dir, &found, remembered.as_deref(), false, false);
            let name = path.file_name().map(|n| n.to_string_lossy().into_owned());
            let _ = st.settings.update(|s| s.cll_file = name).await;
            if let Err(response) = show_cache(&st, path).await {
                return response;
            }
        }
    }
    Json(meta_body(&st).await).into_response()
}

async fn reload(State(st): State<Arc<AppState>>) -> Response {
    let _cache = st.cll_lock.lock().await;
    match load_all(&st).await {
        Ok(g) => Json(g.meta.clone()).into_response(),
        Err(e) => err(e),
    }
}

#[derive(Deserialize)]
struct SearchQuery {
    #[serde(default)]
    q: String,
    #[serde(default)]
    kind: String,
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(serde::Serialize)]
struct Hit<'a> {
    id: &'a str,
    name: &'a str,
    kind: Kind,
    file: &'a str,
    schema: &'a str,
    materialized: &'a str,
    tests: usize,
    disabled: bool,
}

fn parse_kinds(spec: &str) -> Vec<Kind> {
    spec.split(',')
        .filter(|s| !s.is_empty())
        .filter_map(|s| match s {
            "model" => Some(Kind::Model),
            "source" => Some(Kind::Source),
            "seed" => Some(Kind::Seed),
            "snapshot" => Some(Kind::Snapshot),
            "test" => Some(Kind::Test),
            "exposure" => Some(Kind::Exposure),
            _ => None,
        })
        .collect()
}

async fn search(State(st): State<Arc<AppState>>, Query(q): Query<SearchQuery>) -> Response {
    let graph = st.graph.read().await.clone();
    let kinds = parse_kinds(&q.kind);
    let hits = graph.search(&q.q, &kinds, q.limit.unwrap_or(200).min(2000));
    let body: Vec<Hit> = hits
        .iter()
        .map(|&i| {
            let n = &graph.nodes[i as usize];
            Hit {
                id: &n.id,
                name: &n.name,
                kind: n.kind,
                file: &n.file,
                schema: &n.schema,
                materialized: &n.materialized,
                tests: n.tests.len(),
                disabled: n.disabled,
            }
        })
        .collect();
    Json(body).into_response()
}

#[derive(Deserialize)]
struct NodeQuery {
    #[serde(default)]
    id: String,
    #[serde(default)]
    file: String,
}

#[derive(serde::Serialize)]
struct Ref<'a> {
    id: &'a str,
    name: &'a str,
    kind: Kind,
    file: &'a str,
    materialized: &'a str,
    disabled: bool,
    /// Test nodes only: the generic's short name, and the column it guards when
    /// it guards one. Empty on every other kind, so a parent or a child pays
    /// nothing for them.
    #[serde(skip_serializing_if = "str::is_empty")]
    test_name: &'a str,
    #[serde(skip_serializing_if = "str::is_empty")]
    column: &'a str,
}

/// One data test guarding one column. `label` is what the chip says, which is
/// the generic's short name; `name` is the name dbt generated for the test,
/// which is the only thing that tells two tests of one generic apart.
#[derive(serde::Serialize)]
struct ColTest<'a> {
    id: &'a str,
    label: &'a str,
    name: &'a str,
    #[serde(skip_serializing_if = "str::is_empty")]
    file: &'a str,
}

#[derive(serde::Serialize)]
struct Col<'a> {
    name: &'a str,
    data_type: &'a str,
    description: &'a str,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tests: Vec<ColTest<'a>>,
    undeclared: bool,
    up: usize,
    down: usize,
}

/// The same node at three stages: as written, as dbt parsed it, and where it
/// was actually built once the generate_*_name macros ran. `parsed` and `built`
/// come from the same parse, which is what makes comparing them meaningful.
#[derive(serde::Serialize)]
struct Location<'a> {
    written: &'a Place,
    parsed: &'a Place,
    built: Place,
    /// `written` evaluated against each discovered `.env` file on its own,
    /// keyed by file name. The browser picks one, so switching environment
    /// needs no request.
    #[serde(skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    envs: std::collections::BTreeMap<String, envs::Resolution>,
}

#[derive(serde::Serialize)]
struct NodeDetail<'a> {
    id: &'a str,
    name: &'a str,
    kind: Kind,
    file: &'a str,
    yml: &'a str,
    schema: &'a str,
    database: &'a str,
    relation: &'a str,
    materialized: &'a str,
    strategy: &'a str,
    unique_key: &'a str,
    package: &'a str,
    description: &'a str,
    tags: &'a [String],
    disabled: bool,
    location: Location<'a>,
    columns: Vec<Col<'a>>,
    upstream_total: usize,
    downstream_total: usize,
    parents: Vec<Ref<'a>>,
    children: Vec<Ref<'a>>,
    tests: Vec<Ref<'a>>,
}

/// Node detail, looked up either by unique_id or by the file currently open in
/// the editor (so the editor and the lineage view stay in sync).
async fn node(State(st): State<Arc<AppState>>, Query(q): Query<NodeQuery>) -> Response {
    let graph = st.graph.read().await.clone();
    let idx = if !q.id.is_empty() {
        graph.index.get(&q.id).copied()
    } else {
        let key = q.file.replace('\\', "/");
        graph
            .by_file
            .get(&key)
            .and_then(|v| v.iter().find(|&&i| graph.nodes[i as usize].kind != Kind::Test).or(v.first()))
            .copied()
    };
    let Some(idx) = idx else {
        return (StatusCode::NOT_FOUND, "unknown node").into_response();
    };
    let n = &graph.nodes[idx as usize];
    // A handful of files of a few kilobytes each, read fresh so an edit to a
    // .env file shows up on the next click.
    let env_files = {
        let root = st.root.clone();
        tokio::task::spawn_blocking(move || envs::discover(&root)).await.unwrap_or_default()
    };
    let has_location = !n.written.database.is_empty() || !n.written.schema.is_empty() || !n.parsed.database.is_empty();
    let as_ref = |i: &u32| {
        let t = &graph.nodes[*i as usize];
        Ref {
            id: &t.id,
            name: &t.name,
            kind: t.kind,
            file: &t.file,
            materialized: &t.materialized,
            disabled: t.disabled,
            test_name: &t.test_name,
            column: &t.column,
        }
    };
    Json(NodeDetail {
        id: &n.id,
        name: &n.name,
        kind: n.kind,
        file: &n.file,
        yml: &n.yml,
        schema: &n.schema,
        database: &n.database,
        relation: &n.relation,
        materialized: &n.materialized,
        strategy: &n.strategy,
        unique_key: &n.unique_key,
        package: &n.package,
        description: &n.description,
        tags: &n.tags,
        disabled: n.disabled,
        location: Location {
            written: &n.written,
            parsed: &n.parsed,
            built: Place { database: n.database.clone(), schema: n.schema.clone(), alias: n.alias.clone() },
            envs: if has_location { envs::resolve_all(&env_files, &n.written, &n.parsed) } else { Default::default() },
        },
        columns: n
            .columns
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let (up, down) = graph
                    .cll
                    .as_ref()
                    .map(|l| l.degree(ColRef { node: idx, col: i as u32 }))
                    .unwrap_or((0, 0));
                Col {
                    name: &c.name,
                    data_type: &c.data_type,
                    description: &c.description,
                    tests: c
                        .tests
                        .iter()
                        .map(|&t| {
                            let t = &graph.nodes[t as usize];
                            ColTest {
                                id: &t.id,
                                label: if t.test_name.is_empty() { &t.name } else { &t.test_name },
                                name: &t.name,
                                file: &t.file,
                            }
                        })
                        .collect(),
                    undeclared: c.undeclared,
                    up,
                    down,
                }
            })
            .collect(),
        upstream_total: graph.reach(idx, true),
        downstream_total: graph.reach(idx, false),
        parents: n.parents.iter().map(as_ref).collect(),
        children: n.children.iter().map(as_ref).collect(),
        tests: n.tests.iter().map(as_ref).collect(),
    })
    .into_response()
}

#[derive(Deserialize)]
struct ResolveBody {
    names: Vec<String>,
}

#[derive(serde::Serialize)]
struct Resolved<'a> {
    name: &'a str,
    id: &'a str,
    file: &'a str,
    kind: Kind,
    materialized: &'a str,
    description: &'a str,
    disabled: bool,
}

/// Maps the names found in `ref()` / `source()` calls onto dbt nodes, so the
/// editor can turn them into links.
async fn resolve(State(st): State<Arc<AppState>>, Json(body): Json<ResolveBody>) -> Response {
    let graph = st.graph.read().await.clone();
    let out: Vec<Resolved> = body
        .names
        .iter()
        .filter_map(|name| {
            let &i = graph.by_name.get(name.as_str())?;
            let n = &graph.nodes[i as usize];
            Some(Resolved {
                name: &n.name,
                id: &n.id,
                file: &n.file,
                kind: n.kind,
                materialized: &n.materialized,
                description: n.description.split('\n').next().unwrap_or(""),
                disabled: n.disabled,
            })
        })
        .collect();
    Json(out).into_response()
}

#[derive(Deserialize)]
struct MacroCallsBody {
    /// The file the calls were read from. A bare name is looked up in its
    /// package before the root project, the way dbt looks one up for a node.
    #[serde(default)]
    file: String,
    calls: Vec<String>,
}

#[derive(serde::Serialize)]
struct MacroLink {
    /// The call as the editor sent it, which is how it finds its marks again.
    call: String,
    id: String,
    name: String,
    package: String,
    file: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    description: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    args: Vec<String>,
}

/// More distinct calls than any file holds: past it, the request is not an
/// editor asking.
const MAX_MACRO_CALLS: usize = 2_000;

/// Maps the macro calls found in the editor onto the macros they reach, and
/// answers only for those whose file is in the project, since nothing else can
/// be opened. A call it leaves out is a context function, a Jinja builtin, a
/// macro of dbt's own, or a typo, and the editor leaves all four as text.
async fn resolve_macros(State(st): State<Arc<AppState>>, Json(b): Json<MacroCallsBody>) -> Response {
    let graph = st.graph.read().await.clone();
    let root = st.root.clone();
    // A stat or two per call, which on the VM's disk is not free.
    let found = tokio::task::spawn_blocking(move || {
        let macros = &graph.macros;
        let local = macros.package_of(&b.file);
        let mut seen = std::collections::HashSet::new();
        b.calls
            .iter()
            .filter(|c| seen.insert(c.as_str()))
            .take(MAX_MACRO_CALLS)
            .filter_map(|call| {
                let m = macros.resolve(call, local)?;
                let file = macros.place(&root, m)?;
                Some(MacroLink {
                    call: call.clone(),
                    id: m.id.clone(),
                    name: m.name.clone(),
                    package: m.package.clone(),
                    file,
                    description: m.description.clone(),
                    args: m.args.clone(),
                })
            })
            .collect::<Vec<_>>()
    })
    .await;
    match found {
        Ok(links) => Json(links).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
struct LineageQuery {
    id: String,
    #[serde(default = "two")]
    up: u32,
    #[serde(default = "two")]
    down: u32,
    #[serde(default)]
    tests: u8,
    #[serde(default)]
    max: Option<usize>,
}
fn two() -> u32 {
    2
}

async fn lineage(State(st): State<Arc<AppState>>, Query(q): Query<LineageQuery>) -> Response {
    let graph = st.graph.read().await.clone();
    let Some(&idx) = graph.index.get(&q.id) else {
        return (StatusCode::NOT_FOUND, "unknown node").into_response();
    };
    let sub = graph.lineage(idx, q.up.min(20), q.down.min(20), q.tests == 1, q.max.unwrap_or(400).min(3000));
    Json(sub).into_response()
}

/// Names sent back for the copy button. Far past any selection worth reading,
/// and short enough that one expression cannot answer with megabytes.
const MAX_NAMES: usize = 5000;

#[derive(Deserialize)]
struct SelectorQuery {
    q: String,
    #[serde(default)]
    exclude: String,
    #[serde(default)]
    tests: u8,
    #[serde(default)]
    max: Option<usize>,
}

#[derive(serde::Serialize)]
struct SelectorBody<'a> {
    /// Flattened, so the payload keeps the shape the canvas already renders.
    #[serde(flatten)]
    graph: crate::graph::Lineage<'a>,
    /// How many nodes matched, which is more than were drawn when the canvas
    /// capped: the summary tells the truth even when the picture cannot.
    matched: usize,
    /// Everything that matched, counted by kind. Counted here rather than off
    /// the drawn nodes so a capped canvas still reports the whole selection.
    counts: std::collections::HashMap<String, usize>,
    /// Every match, not only what was drawn, so the copied list is the whole
    /// answer. Sorted the way `dbt ls --output name` prints it.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    names: Vec<&'a str>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    warnings: Vec<String>,
    /// The include half as parsed, with any pasted command prefix stripped.
    select: String,
    /// True when a `dbt ls -s` was stripped off the front. The box rewrites
    /// itself to `select` then, so what is on screen is what was resolved.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    stripped: bool,
    #[serde(skip_serializing_if = "String::is_empty")]
    exclude: String,
    /// The named selector resolved, when the box held `--selector NAME`.
    /// `select` is empty then: the selector is the whole selection.
    #[serde(skip_serializing_if = "str::is_empty")]
    selector: &'a str,
    #[serde(skip_serializing_if = "str::is_empty")]
    description: &'a str,
    /// The manifest dropped the `indirect_selection` that `selectors.yml`
    /// sets, so this answer keeps tests the way dbt's default would.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    lost_indirect: bool,
    /// Selected tests left off the canvas because the tests box is off. A
    /// named selector's answer is dbt's whatever the box says.
    #[serde(skip_serializing_if = "is_zero")]
    hidden_tests: usize,
}

fn is_zero(n: &usize) -> bool {
    *n == 0
}

#[derive(serde::Serialize)]
struct SelectorFailed {
    error: String,
    code: &'static str,
    /// Byte offset of the term at fault, for a caret under it.
    #[serde(skip_serializing_if = "Option::is_none")]
    at: Option<usize>,
}

/// Resolves a dbt node-selection expression against the manifest graph. dbt is
/// never run for it, and never will be (0002, 0024).
///
/// `tests` means two things. For a typed line it puts tests in the universe or
/// leaves them out, as it always has. For `--selector NAME` it only filters
/// what is drawn: the selector's definition already decided which tests
/// belong, and the answer stays dbt's either way (0032).
async fn selection(State(st): State<Arc<AppState>>, Query(q): Query<SelectorQuery>) -> Response {
    let graph = st.graph.read().await.clone();
    let max = q.max.unwrap_or(400).min(3000);
    let failed = |e: select::SelectError| {
        let body = SelectorFailed { error: e.to_string(), code: e.code(), at: e.pos() };
        (StatusCode::BAD_REQUEST, Json(body)).into_response()
    };
    let named = match select::selector_name(&q.q, &q.exclude) {
        Ok(named) => named,
        Err(e) => return failed(e),
    };

    if let Some((name, stripped)) = named {
        let res = match graph.selectors.resolve(&graph, &name) {
            Ok(res) => res,
            Err(e) => return failed(e),
        };
        let shown = selectors::shown(&graph, &res.nodes, q.tests == 1);
        let sub = graph.selection(&shown.drawn, &shown.context, q.tests == 1, max);
        let (counts, names) = tally(&graph, &res.nodes);
        let named = graph.selectors.get(&name);
        return Json(SelectorBody {
            matched: res.nodes.len(),
            counts,
            names,
            warnings: res.warnings,
            select: String::new(),
            stripped,
            exclude: String::new(),
            selector: named.map(|n| n.name.as_str()).unwrap_or(""),
            description: named.map(|n| n.description.as_str()).unwrap_or(""),
            lost_indirect: graph.selectors.lost_indirect(),
            hidden_tests: shown.hidden_tests,
            graph: sub,
        })
        .into_response();
    }

    let tests = if q.tests == 1 { select::Tests::Eager } else { select::Tests::Excluded };
    let (expr, res) = match select::select(&graph, &q.q, &q.exclude, tests) {
        Ok(v) => v,
        Err(e) => return failed(e),
    };
    let sub = graph.selection(&res.nodes, &[], tests == select::Tests::Eager, max);
    let (counts, names) = tally(&graph, &res.nodes);
    Json(SelectorBody {
        matched: res.nodes.len(),
        counts,
        names,
        warnings: res.warnings,
        select: expr.select,
        stripped: expr.stripped,
        exclude: expr.excluded,
        selector: "",
        description: "",
        lost_indirect: false,
        hidden_tests: 0,
        graph: sub,
    })
    .into_response()
}

/// Everything a selection matched, counted by kind and listed by name, off
/// the whole answer rather than what the canvas could draw.
fn tally<'g>(graph: &'g Graph, nodes: &[u32]) -> (std::collections::HashMap<String, usize>, Vec<&'g str>) {
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for &i in nodes {
        *counts.entry(graph.nodes[i as usize].kind.as_str().to_string()).or_insert(0) += 1;
    }
    let mut names: Vec<&str> = nodes.iter().take(MAX_NAMES).map(|&i| graph.nodes[i as usize].name.as_str()).collect();
    names.sort_unstable();
    (counts, names)
}

#[derive(serde::Serialize)]
struct SelectorEntry<'a> {
    name: &'a str,
    #[serde(skip_serializing_if = "str::is_empty")]
    description: &'a str,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    default: bool,
    /// Why this one cannot be resolved here. It is still listed, so the menu
    /// can say so and the dbt ls button can still settle it.
    #[serde(skip_serializing_if = "Option::is_none")]
    unsupported: Option<&'a str>,
}

#[derive(serde::Serialize)]
struct SelectorsBody<'a> {
    selectors: Vec<SelectorEntry<'a>>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    lost_indirect: bool,
}

/// The named selectors the manifest records, for the menu beside the
/// Selection box. Read-only, and nothing in it comes from `selectors.yml`
/// itself, only from what dbt parsed out of it (0032).
async fn list_selectors(State(st): State<Arc<AppState>>) -> Response {
    let graph = st.graph.read().await.clone();
    let selectors = graph
        .selectors
        .list()
        .iter()
        .map(|n| SelectorEntry {
            name: &n.name,
            description: &n.description,
            default: n.default,
            unsupported: n.unsupported.as_deref(),
        })
        .collect();
    Json(SelectorsBody { selectors, lost_indirect: graph.selectors.lost_indirect() }).into_response()
}

#[derive(serde::Serialize)]
struct EnvEntry {
    file: String,
    name: String,
    auto_name: String,
    /// `DBT_TARGET` from the file: the one raw value this endpoint returns,
    /// shown as a cross-check against the name.
    target: String,
    target_mismatch: bool,
    hidden: bool,
    hidden_reason: Option<String>,
    /// What automatic detection alone would decide, so the panel can tell an
    /// explicit choice from a default and store only real overrides.
    auto_hidden_reason: Option<String>,
    coverage: envs::Coverage,
    agreement: envs::Agreement,
    warnings: Vec<envs::Warning>,
}

#[derive(serde::Serialize)]
struct EnvsBody {
    files: Vec<EnvEntry>,
    referenced: Vec<envs::RefVar>,
    selected: Option<String>,
    settings_path: String,
    persist: bool,
}

/// The discovered `.env` files and how each one covers the variables the
/// project's location config reads. Variable names only, never their values.
async fn list_envs(State(st): State<Arc<AppState>>) -> Response {
    let graph = st.graph.read().await.clone();
    let root = st.root.clone();
    let settings = st.settings.load();
    let settings_path = st.settings.path.as_ref().map(|p| p.display().to_string()).unwrap_or_default();
    let body = tokio::task::spawn_blocking(move || {
        let places: Vec<(&Place, &Place)> =
            graph.nodes.iter().filter(|n| n.kind != Kind::Test).map(|n| (&n.written, &n.parsed)).collect();
        let referenced = envs::referenced(&places);
        let files = envs::discover(&root)
            .into_iter()
            .map(|f| {
                let coverage = envs::coverage(&f, &referenced);
                let auto_reason = envs::auto_hidden(&f, &coverage, &referenced);
                let over = settings.envs.get(&f.file).cloned().unwrap_or_default();
                // A user's explicit choice beats the automatic one in both directions.
                let (hidden, hidden_reason) = match over.hidden {
                    Some(true) => (true, Some("hidden by you".to_string())),
                    Some(false) => (false, None),
                    None => (auto_reason.is_some(), auto_reason.map(str::to_string)),
                };
                EnvEntry {
                    name: over.name.clone().unwrap_or_else(|| f.auto_name.clone()),
                    auto_name: f.auto_name.clone(),
                    target_mismatch: envs::target_mismatch(&f),
                    hidden,
                    hidden_reason,
                    auto_hidden_reason: auto_reason.map(str::to_string),
                    agreement: envs::agreement(places.iter().copied(), &f.vars),
                    coverage,
                    target: f.target.clone(),
                    warnings: f.warnings.clone(),
                    file: f.file,
                }
            })
            .collect::<Vec<EnvEntry>>();
        // A stored choice only counts while its file still exists and is visible.
        let selected = settings.selected.filter(|s| files.iter().any(|f| &f.file == s && !f.hidden));
        let persist = !settings_path.is_empty();
        EnvsBody { files, referenced, selected, settings_path, persist }
    })
    .await;
    match body {
        Ok(body) => Json(body).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// `.env` or `.env.<suffix>`, a plain file name and nothing that could walk out
/// of the project directory.
fn valid_env_file(file: &str) -> bool {
    (file == ".env" || file.starts_with(".env.")) && !file.contains(['/', '\\']) && !file.contains("..")
}

#[derive(Deserialize)]
struct SelectBody {
    file: Option<String>,
}

/// Stores where a fresh tab starts. Touches nothing but the selection, so it
/// cannot undo a rename saved from another tab.
async fn select_env(State(st): State<Arc<AppState>>, Json(b): Json<SelectBody>) -> Response {
    if let Some(f) = &b.file {
        if !valid_env_file(f) {
            return (StatusCode::BAD_REQUEST, "not an env file name").into_response();
        }
    }
    match st.settings.update(|s| s.selected = b.file).await {
        Ok(_) => Json(serde_json::json!({ "ok": true })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    }
}

/// The project's vars, and any env vars the editor asked about by name.
///
/// This is the one route that returns a value derived from a `.env` file, which
/// 0012 forbade and 0019 allows under two server-side guards: a
/// `DBT_ENV_SECRET_*` is never substituted, and a name that reads as a
/// credential comes back with its status and no value. Nothing here is logged.
#[derive(Deserialize)]
struct VarsQuery {
    /// The `.env` file this tab has selected, "" for none.
    #[serde(default)]
    env: String,
    /// Comma separated env var names, for the editor's `env_var()` hover.
    /// One field rather than a repeated key: `serde_urlencoded` does not
    /// collect repeated keys into a Vec.
    #[serde(default)]
    names: String,
}

/// At most this many names per request, so one crafted URL cannot walk a file.
const MAX_VAR_NAMES: usize = 64;

#[derive(serde::Serialize)]
struct VarOut<'a> {
    name: &'a str,
    line: usize,
    /// The value as written in dbt_project.yml.
    raw: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    list: Option<&'a [String]>,
    /// Present only for a Jinja value that was resolved, and never when redacted.
    #[serde(skip_serializing_if = "Option::is_none")]
    resolved: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<envs::Status>,
    /// The env var names the expression reads. Names only, as everywhere else.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    vars: Vec<String>,
    /// The value came from the default written in the `env_var()` call rather
    /// than from the file, which the card has to be able to say.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    default_used: bool,
    /// The name reads as a credential, so no value is returned (0019).
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    redacted: bool,
    /// Declared with no value, so var() returns null.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    null: bool,
}

#[derive(serde::Serialize)]
struct EnvVarOut {
    name: String,
    status: envs::Status,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    redacted: bool,
}

#[derive(serde::Serialize)]
struct VarsBody<'a> {
    file: &'static str,
    found: bool,
    /// The `.env` the values were resolved with, "" for none.
    env: &'a str,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    vars: Vec<VarOut<'a>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    env_vars: Vec<EnvVarOut>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    packages: Vec<&'a crate::project::PackageVars>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    unparsed: Vec<&'a crate::project::Unparsed>,
}

async fn list_vars(State(st): State<Arc<AppState>>, Query(q): Query<VarsQuery>) -> Response {
    if !q.env.is_empty() && !valid_env_file(&q.env) {
        return (StatusCode::BAD_REQUEST, "not an env file name").into_response();
    }
    let names: Vec<String> =
        q.names.split(',').map(str::trim).filter(|s| !s.is_empty()).take(MAX_VAR_NAMES).map(String::from).collect();

    let root = st.root.clone();
    let want_env = q.env.clone();
    // The project file and the .env files are read fresh, so an edit to either
    // shows up on the next hover rather than on the next restart.
    let loaded = tokio::task::spawn_blocking(move || {
        let project = crate::project::read(&root);
        let vars = if want_env.is_empty() {
            Some(envs::Vars::new())
        } else {
            envs::discover(&root).into_iter().find(|f| f.file == want_env).map(|f| f.vars)
        };
        (project, vars)
    })
    .await;
    let Ok((project, found_vars)) = loaded else {
        return (StatusCode::INTERNAL_SERVER_ERROR, "could not read the project").into_response();
    };
    let Some(vars) = found_vars else {
        return (StatusCode::BAD_REQUEST, "unknown env file").into_response();
    };

    let mut out = Vec::new();
    for v in &project.vars {
        let mut row = VarOut {
            name: &v.name,
            line: v.line,
            raw: &v.raw,
            list: v.list.as_deref(),
            resolved: None,
            status: None,
            vars: Vec::new(),
            default_used: false,
            redacted: envs::sensitive_name(&v.name),
            null: v.null,
        };
        if v.jinja {
            let (value, cell) = envs::resolve(&v.raw, "", &vars);
            // Reading from the file and falling back to the default written in
            // the call both end up as Env, and the card has to tell them apart.
            row.default_used = envs::used_default(&cell, &vars);
            row.redacted = row.redacted || cell.vars.iter().any(|n| envs::sensitive_name(n));
            row.status = Some(cell.kind);
            row.vars = cell.vars;
            if !row.redacted {
                row.resolved = Some(value);
            }
        } else if row.redacted {
            // Committed to the repository rather than read from a .env, so 0012
            // does not reach it. Handing over a credential because of where it
            // happens to be written is still not a distinction worth defending.
            row.raw = "";
            row.list = None;
        }
        out.push(row);
    }

    let env_vars = names
        .into_iter()
        .map(|name| {
            let (value, cell) = envs::lookup(&name, &vars);
            let redacted = envs::sensitive_name(&name);
            EnvVarOut {
                name,
                status: cell.kind,
                value: if redacted || cell.kind != envs::Status::Env && cell.kind != envs::Status::Placeholder {
                    None
                } else {
                    Some(value)
                },
                redacted,
            }
        })
        .collect();

    Json(VarsBody {
        file: "dbt_project.yml",
        found: project.found,
        env: &q.env,
        vars: out,
        env_vars,
        packages: project.packages.iter().collect(),
        unparsed: project.unparsed.iter().collect(),
    })
    .into_response()
}

#[derive(Deserialize)]
struct OverridesBody {
    envs: std::collections::BTreeMap<String, crate::settings::EnvOverride>,
}

/// Saves names and visibility from the Manage panel. Touches nothing but those.
async fn save_envs(State(st): State<Arc<AppState>>, Json(b): Json<OverridesBody>) -> Response {
    let mut cleaned = std::collections::BTreeMap::new();
    for (file, over) in b.envs {
        if !valid_env_file(&file) {
            return (StatusCode::BAD_REQUEST, "not an env file name").into_response();
        }
        // An empty name means "use the automatic one".
        let name = over.name.map(|n| n.trim().chars().take(32).collect::<String>()).filter(|n| !n.is_empty());
        cleaned.insert(file, crate::settings::EnvOverride { name, hidden: over.hidden });
    }
    let result = st
        .settings
        .update(|s| {
            for (file, over) in cleaned {
                if over == crate::settings::EnvOverride::default() {
                    s.envs.remove(&file);
                } else {
                    s.envs.insert(file, over);
                }
            }
        })
        .await;
    match result {
        Ok(_) => Json(serde_json::json!({ "ok": true })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    }
}

#[derive(Deserialize)]
struct ArtifactQuery {
    id: String,
    /// `compiled` or `run`. Defaulted rather than required, because the
    /// compiled file is what this route answered before the run file existed.
    #[serde(default = "compiled_kind")]
    kind: String,
}

fn compiled_kind() -> String {
    "compiled".to_string()
}

/// The SQL dbt left under `target/` for one node, with the freshness signals
/// the UI colours on. `kind` picks the artifact: `compiled/` for the model's
/// own SQL, `run/` for the statement dbt executed.
async fn compiled_sql(State(st): State<Arc<AppState>>, Query(q): Query<ArtifactQuery>) -> Response {
    let Some(kind) = compiled::kind(&q.kind) else {
        return (StatusCode::BAD_REQUEST, "kind must be compiled or run").into_response();
    };
    let graph = st.graph.read().await.clone();
    let Some(&idx) = graph.index.get(&q.id) else {
        return (StatusCode::NOT_FOUND, "unknown node").into_response();
    };
    let n = &graph.nodes[idx as usize];
    let (root, target) = (st.root.clone(), st.target_dir.clone());
    let (package, file, yml) = (n.package.clone(), n.file.clone(), n.yml.clone());
    let (compiled_at, name, alias) = (n.compiled.clone(), n.name.clone(), n.alias.clone());
    match tokio::task::spawn_blocking(move || {
        let subject = compiled::Subject {
            package: &package,
            file: &file,
            yml: &yml,
            compiled: &compiled_at,
            name: &name,
            alias: &alias,
        };
        compiled::look_up(&root, &target, kind, &subject, 2 * 1024 * 1024)
    })
    .await
    {
        Ok(info) => Json(info).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
struct ColLineageQuery {
    id: String,
    column: String,
    #[serde(default = "two")]
    up: u32,
    #[serde(default = "two")]
    down: u32,
    #[serde(default)]
    max: Option<usize>,
}

/// Column-level lineage around one column. Columns fan out harder than models,
/// hence the lower default cap.
async fn col_lineage(State(st): State<Arc<AppState>>, Query(q): Query<ColLineageQuery>) -> Response {
    let graph = st.graph.read().await.clone();
    if graph.cll.is_none() {
        return (StatusCode::NOT_FOUND, "no column lineage").into_response();
    }
    let Some(&idx) = graph.index.get(&q.id) else {
        return (StatusCode::NOT_FOUND, "unknown node").into_response();
    };
    let Some(col) = graph.col_slot(idx, &q.column) else {
        return (StatusCode::NOT_FOUND, "unknown column").into_response();
    };
    let sub = graph.column_lineage(
        ColRef { node: idx, col },
        q.up.min(20),
        q.down.min(20),
        q.max.unwrap_or(200).min(2000),
    );
    Json(sub).into_response()
}

// ------------------------------------------------------------- snowflake ----

#[derive(serde::Serialize)]
struct SidecarBody {
    enabled: bool,
    #[serde(flatten)]
    status: sidecar::Status,
}

async fn sidecar_status(State(st): State<Arc<AppState>>) -> Response {
    Json(SidecarBody { enabled: st.sidecar.enabled(), status: st.sidecar.status() }).into_response()
}

/// A dbt profile is a few kilobytes; anything of this size is not one.
const MAX_PROFILE_BYTES: u64 = 512 * 1024;

#[derive(serde::Serialize)]
struct ProfileBody {
    path: String,
    content: String,
}

fn profile_on_disk(path: &Path) -> Result<ProfileBody, String> {
    let meta = std::fs::metadata(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    if meta.len() > MAX_PROFILE_BYTES {
        return Err(format!("{} is far larger than a dbt profile, so it is left alone", path.display()));
    }
    let content = std::fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    Ok(ProfileBody { path: path.display().to_string(), content })
}

/// Why a column click fetches nothing while Snowflake's features are on.
const NOT_PICKED: &str = "Snowflake is not the column lineage tool: pick it in the column lineage menu";

/// Why there is nothing to open yet.
const NO_PROFILE: &str = "no profile yet: pick Snowflake in the column lineage menu once, so the script says which file it reads";

/// The dbt profile the Snowflake script read: the one file outside the project
/// dbt-edith opens, and only because the script named it first (0017). No path
/// comes from the browser, so no request can widen the exception, and with
/// Snowflake's features off nothing on the page needs it, so it is closed (0031).
async fn read_profile(State(st): State<Arc<AppState>>) -> Response {
    if !st.snowflake_allowed().await {
        return (StatusCode::CONFLICT, SNOWFLAKE_OFF).into_response();
    }
    let Some(path) = st.sidecar.profile_path() else {
        return (StatusCode::CONFLICT, NO_PROFILE).into_response();
    };
    match tokio::task::spawn_blocking(move || profile_on_disk(&path)).await {
        Ok(Ok(body)) => Json(body).into_response(),
        Ok(Err(e)) => (StatusCode::NOT_FOUND, e).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
struct ProfileWrite {
    content: String,
}

#[derive(serde::Serialize)]
struct ProfileSaved {
    path: String,
    /// The script reads the profile once, when it starts, so saving restarts it.
    restarted: bool,
    #[serde(flatten)]
    status: sidecar::Status,
}

/// Saves that same file, and only if it is already there: this route never
/// creates a file, and never takes a path (0017). The write is atomic, so a
/// half-written profile cannot be left behind.
async fn write_profile(State(st): State<Arc<AppState>>, Json(b): Json<ProfileWrite>) -> Response {
    if !st.snowflake_allowed().await {
        return (StatusCode::CONFLICT, SNOWFLAKE_OFF).into_response();
    }
    let Some(path) = st.sidecar.profile_path() else {
        return (StatusCode::CONFLICT, NO_PROFILE).into_response();
    };
    if b.content.len() as u64 > MAX_PROFILE_BYTES {
        return (StatusCode::BAD_REQUEST, "far larger than a dbt profile, so it is not written").into_response();
    }
    if !path.is_file() {
        return (StatusCode::NOT_FOUND, format!("{} is no longer there", path.display())).into_response();
    }
    let (target, content) = (path.clone(), b.content);
    match tokio::task::spawn_blocking(move || crate::settings::write_atomic(&target, content.as_bytes())).await {
        Ok(Ok(())) => {}
        Ok(Err(e)) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("cannot write {}: {e}", path.display())).into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
    // A correction nobody reads is worse than no correction: the script holds
    // the profile it read at startup, so it starts again on the new one.
    let restarted = st.sidecar.enabled();
    let status = if restarted { st.sidecar.restart(&st.root, &st.venv).await } else { st.sidecar.status() };
    Json(ProfileSaved { path: path.display().to_string(), restarted, status }).into_response()
}

#[derive(Deserialize)]
struct FetchBody {
    id: String,
    column: String,
    /// The clicked node's relation as the browser shows it for the selected
    /// environment, so Snowflake is asked about exactly what the user sees.
    relation: String,
    /// The selected `.env` file, empty for the manifest.
    #[serde(default)]
    env: String,
    #[serde(default = "two")]
    up: u32,
    #[serde(default = "two")]
    down: u32,
}

/// A fetch that failed, with the phase that says whether the profile is to
/// blame, so the UI can point at it without reading Snowflake's error codes.
#[derive(serde::Serialize)]
struct FetchFailed {
    error: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    phase: String,
}

#[derive(serde::Serialize)]
struct Fetched {
    relation: String,
    /// Column pairs Snowflake returned, both directions together.
    rows: usize,
    /// Edges the cache did not have yet.
    added: usize,
    /// Edges on the clicked column now, from the cache as it stands.
    up: usize,
    down: usize,
    /// Objects Snowflake named that are no node of this project, the first 20.
    unmatched: Vec<String>,
    unmatched_total: usize,
}

/// Column lineage from Snowflake around one clicked column, merged into the
/// cache file and into the graph in memory. POST and never GET: a GET passes
/// the guard on its Host alone, so any page could run warehouse queries
/// through an image tag (0015, 0016).
async fn fetch_col_lineage(State(st): State<Arc<AppState>>, Json(b): Json<FetchBody>) -> Response {
    if !st.snowflake_allowed().await {
        return (StatusCode::CONFLICT, SNOWFLAKE_OFF).into_response();
    }
    if !st.sidecar.enabled() {
        return (StatusCode::CONFLICT, NOT_PICKED).into_response();
    }
    if !collin::valid_relation(&b.relation) {
        return (StatusCode::BAD_REQUEST, "not a database.schema.object relation").into_response();
    }
    if b.column.trim().is_empty() || b.column.len() > 256 || b.column.chars().any(char::is_control) {
        return (StatusCode::BAD_REQUEST, "not a column name").into_response();
    }
    if !b.env.is_empty() && !valid_env_file(&b.env) {
        return (StatusCode::BAD_REQUEST, "not an env file name").into_response();
    }
    {
        // Refused before any query: a result that cannot be stored is not
        // worth a sign-in tab.
        let graph = st.graph.read().await;
        if !graph.index.contains_key(&b.id) {
            return (StatusCode::NOT_FOUND, "unknown node").into_response();
        }
        // No conflict check any more: Snowflake writes to its own file, so it
        // cannot contend with another producer's. What it still does is take
        // over the graph, which holds one source at a time, and the UI says so
        // by showing which cache is active.
    }
    // The file's values stay on the server: they only decide which node an
    // object stands for.
    let vars = if b.env.is_empty() {
        None
    } else {
        let (root, file) = (st.root.clone(), b.env.clone());
        match tokio::task::spawn_blocking(move || envs::discover(&root).into_iter().find(|f| f.file == file)).await {
            Ok(Some(found)) => Some(found.vars),
            _ => return (StatusCode::NOT_FOUND, "that env file is no longer in the project").into_response(),
        }
    };

    if !st.sidecar.is_up() {
        let status = st.sidecar.start_for(&st.root, &st.venv).await;
        if status.state != "ready" {
            let why = if status.error.is_empty() { NOT_PICKED.to_string() } else { status.error };
            return (StatusCode::BAD_GATEWAY, why).into_response();
        }
    }
    let mut rows = Vec::new();
    for (direction, depth) in [("UPSTREAM", b.up), ("DOWNSTREAM", b.down)] {
        if depth == 0 {
            continue;
        }
        // GET_LINEAGE goes five levels deep at most.
        match st.sidecar.query(&b.relation, &b.column, direction, depth.min(5)).await {
            Ok(found) => rows.extend(found),
            Err(e) => {
                let body = FetchFailed { error: e.message, phase: e.phase };
                return (StatusCode::BAD_GATEWAY, Json(body)).into_response();
            }
        }
    }

    let total = rows.len();
    let graph = st.graph.read().await.clone();
    let Some(&focus) = graph.index.get(&b.id) else {
        return (StatusCode::NOT_FOUND, "unknown node").into_response();
    };
    let relation = b.relation.clone();
    // The graph moves into the task and is dropped with it, so the merge below
    // can usually update the graph in place instead of copying it.
    let mapped = tokio::task::spawn_blocking(move || collin::edges_for(&graph, &rows, focus, &relation, vars.as_ref())).await;
    let Ok((edges, unmatched)) = mapped else {
        return (StatusCode::INTERNAL_SERVER_ERROR, "matching the lineage to the project failed").into_response();
    };

    let _cache = st.cll_lock.lock().await;
    // Another tool picked while Snowflake was answering: the answer is not
    // wanted any more, and storing it would take the graph back.
    if !st.sidecar.enabled() {
        return (StatusCode::CONFLICT, NOT_PICKED).into_response();
    }
    // The Snowflake cache on screen, which is its newest, so a fetch adds to
    // the dump being read rather than hiding it behind a file of one column.
    // Otherwise Snowflake's own file. Never another producer's: one file per
    // producer is what keeps a half Snowflake, half other cache from existing.
    let shown = st.cll();
    let path = if collin::tool_of(&st.graph.read().await.meta.cll_source) == "snowflake" && shown.parent() == Some(st.target_dir.as_path()) {
        shown
    } else {
        collin::path_for(&st.target_dir, "snowflake")
    };
    let target = st.sidecar.status().target;
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let written = path.clone();
    let merged = match tokio::task::spawn_blocking(move || collin::add_to_file(&written, edges, &target, now)).await {
        Ok(Ok(merged)) => merged,
        Ok(Err(e)) => return (StatusCode::CONFLICT, e).into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };
    let mut added = 0;
    if let Some((cache, n)) = merged {
        added = n;
        // Fetching is an explicit request for Snowflake's answer, so its cache
        // becomes the active one. The graph could not show both anyway.
        let switched = st.cll() != path;
        if switched {
            st.set_cll(path.clone());
            if let Some(name) = path.file_name().map(|n| n.to_string_lossy().into_owned()) {
                let _ = st.settings.update(|s| s.cll_file = Some(name)).await;
            }
        }
        let mtime = mtime_secs(&path);
        if let Ok(mut seen) = st.seen.lock() {
            seen[2] = mtime;
        }
        let file = path.display().to_string();
        if switched {
            // Another file was on screen, and the columns it added go with it.
            let base = st.base.read().await.clone();
            let Ok(fresh) = tokio::task::spawn_blocking(move || {
                let mut graph = (*base).clone();
                graph.merge_col_lineage(cache, mtime);
                graph.meta.cll_file = file;
                graph
            })
            .await
            else {
                return (StatusCode::INTERNAL_SERVER_ERROR, "merging the lineage failed").into_response();
            };
            *st.graph.write().await = Arc::new(fresh);
        } else {
            // The same file, grown: every column it added before is still in it.
            let mut current = st.graph.write().await;
            let graph = Arc::make_mut(&mut current);
            graph.merge_col_lineage(cache, mtime);
            graph.meta.cll_file = file;
        }
    }
    let graph = st.graph.read().await.clone();
    let (up, down) = match (graph.cll.as_ref(), graph.index.get(&b.id)) {
        (Some(cll), Some(&node)) => graph.col_slot(node, &b.column).map_or((0, 0), |col| cll.degree(ColRef { node, col })),
        _ => (0, 0),
    };
    Json(Fetched {
        relation: b.relation,
        rows: total,
        added,
        up,
        down,
        unmatched_total: unmatched.len(),
        unmatched: unmatched.into_iter().take(20).collect(),
    })
    .into_response()
}

#[derive(Deserialize)]
struct PathQuery {
    #[serde(default)]
    path: String,
}

async fn dir(State(st): State<Arc<AppState>>, Query(q): Query<PathQuery>) -> Response {
    match files::list_dir(&st.root, &q.path) {
        Ok(entries) => Json(entries).into_response(),
        Err(e) => err(e),
    }
}

#[derive(Deserialize)]
struct FileQuery {
    #[serde(default)]
    q: String,
    #[serde(default)]
    limit: Option<usize>,
}

/// Every file in the project, not just the ones dbt knows about: generic test
/// definitions, macros, scripts and dotfiles are all things you search for.
async fn file_search(State(st): State<Arc<AppState>>, Query(q): Query<FileQuery>) -> Response {
    let index = st.file_index.read().await.clone();
    let hits = files::search_paths(&index, &q.q, q.limit.unwrap_or(60).min(500));
    Json(hits).into_response()
}

#[derive(Deserialize)]
struct GrepQuery {
    q: String,
    /// Files to report, not matches: a word in 400 models is a real answer.
    #[serde(default)]
    limit: Option<usize>,
}

/// The shortest query worth walking the project for. One or two characters
/// match nearly every file and cost a full read of each.
const GREP_MIN_QUERY: usize = 3;
const GREP_DEFAULT_FILES: usize = 200;
const GREP_MAX_FILES: usize = 500;
const GREP_PER_FILE: usize = 20;

/// Searches file contents, which the path index cannot do. `.env` files are
/// never opened here (0020).
async fn grep(State(st): State<Arc<AppState>>, Query(q): Query<GrepQuery>) -> Response {
    let needle = q.q.trim().to_string();
    if needle.chars().count() < GREP_MIN_QUERY {
        return Json(files::GrepResult::default()).into_response();
    }
    let index = st.file_index.read().await.clone();
    let root = st.root.clone();
    let limit = q.limit.unwrap_or(GREP_DEFAULT_FILES).clamp(1, GREP_MAX_FILES);
    // Reads every indexed file in the worst case, so never on the async runtime.
    match tokio::task::spawn_blocking(move || files::grep(&root, &index, &needle, limit, GREP_PER_FILE)).await {
        Ok(result) => Json(result).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// The index is a directory walk, cheap enough to simply redo on a timer so a
/// branch switch or a new file shows up without any invalidation logic.
pub async fn watch_files(st: Arc<AppState>) {
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(20)).await;
        let root = st.root.clone();
        if let Ok(list) = tokio::task::spawn_blocking(move || files::scan(&root)).await {
            *st.file_index.write().await = Arc::new(list);
        }
    }
}

async fn read_file(State(st): State<Arc<AppState>>, Query(q): Query<PathQuery>) -> Response {
    match files::read_file(&st.root, &q.path) {
        Ok(body) => Json(body).into_response(),
        Err(e) => err(e),
    }
}

#[derive(Deserialize)]
struct WriteBody {
    path: String,
    content: String,
}

async fn write_file(State(st): State<Arc<AppState>>, Json(body): Json<WriteBody>) -> Response {
    match files::write_file(&st.root, &body.path, &body.content) {
        Ok(()) => Json(serde_json::json!({"ok": true})).into_response(),
        Err(e) => err(e),
    }
}

/// Working-tree status, cached briefly so that polling from the explorer does
/// not fork a git process on every tick.
/// The working-tree status, cached for a moment. The git panel and the
/// freshness badge poll on their own timers and both need it, so they share one
/// `git status` rather than running one each.
async fn git_cached(st: &Arc<AppState>) -> GitInfo {
    let mut cached = st.git.lock().await;
    if let Some((at, info)) = cached.as_ref() {
        if at.elapsed() < std::time::Duration::from_millis(1500) {
            return info.clone();
        }
    }
    let root = st.root.clone();
    let info = tokio::task::spawn_blocking(move || git::status(&root))
        .await
        .unwrap_or_default();
    *cached = Some((std::time::Instant::now(), info.clone()));
    info
}

async fn git_status(State(st): State<Arc<AppState>>) -> Response {
    Json(git_cached(&st).await).into_response()
}

/// How far `manifest.json` has drifted from the files it was parsed out of.
/// Polled beside the git status, so it is cached for about as long.
async fn freshness_status(State(st): State<Arc<AppState>>) -> Response {
    {
        let cached = st.fresh.lock().await;
        if let Some((at, f)) = cached.as_ref() {
            if at.elapsed() < std::time::Duration::from_millis(2500) {
                return Json(f.clone()).into_response();
            }
        }
    }
    let git = git_cached(&st).await;
    let graph = st.graph.read().await.clone();
    let manifest_at = graph.meta.manifest_mtime;
    // This project's own nodes, both files each: a model's schema.yml
    // disappearing changes the manifest as surely as the model's file does. A
    // node from an installed package is left out because its path is relative
    // to the package directory, so it would read as a file that had vanished.
    let project = graph.meta.project.clone();
    let mut node_files: Vec<String> = graph
        .nodes
        .iter()
        .filter(|n| n.package.is_empty() || n.package == project)
        .flat_map(|n| [n.file.clone(), n.yml.clone()])
        .filter(|p| !p.is_empty())
        .collect();
    node_files.sort();
    node_files.dedup();

    let root = st.root.clone();
    let f = tokio::task::spawn_blocking(move || freshness::check(&root, manifest_at, &node_files, &git))
        .await
        .unwrap_or_default();
    *st.fresh.lock().await = Some((std::time::Instant::now(), f.clone()));
    Json(f).into_response()
}

/// Runs one git action off the async runtime, then drops the status cache so
/// the next poll shows the new state rather than a stale one.
async fn git_do<F>(st: &Arc<AppState>, f: F) -> Response
where
    F: FnOnce(PathBuf) -> git::GitRun + Send + 'static,
{
    let root = st.root.clone();
    let out = tokio::task::spawn_blocking(move || f(root)).await;
    *st.git.lock().await = None;
    *st.fresh.lock().await = None;
    match out {
        Ok(run) => Json(run).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Paths come from the browser, so keep them inside the repository. git would
/// refuse anything outside anyway; this just fails earlier and more clearly.
/// Both separators count, and so does a drive or UNC prefix: on Windows,
/// `Path::join` replaces the root with either.
fn sane_path(p: &str) -> bool {
    let unified = p.replace('\\', "/");
    !p.is_empty()
        && !unified.starts_with('/')
        && !unified.split('/').any(|seg| seg == ".." || seg.ends_with(':'))
}

fn sane_paths(paths: &[String]) -> Result<(), Response> {
    for p in paths {
        if !sane_path(p) {
            return Err((StatusCode::BAD_REQUEST, format!("refusing path {p:?}")).into_response());
        }
    }
    Ok(())
}

async fn git_branches(State(st): State<Arc<AppState>>) -> Response {
    let root = st.root.clone();
    match tokio::task::spawn_blocking(move || git::branches(&root)).await {
        Ok(b) => Json(b).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// The working-tree side is read from disk, so the path goes through the same
/// confinement as the editor rather than the lighter check git actions get.
async fn git_diff(State(st): State<Arc<AppState>>, Query(q): Query<PathQuery>) -> Response {
    let on_disk = match files::resolve(&st.root, &q.path) {
        Ok(p) => p,
        Err(e) => return err(e),
    };
    let (root, rel) = (st.root.clone(), q.path.replace('\\', "/"));
    match tokio::task::spawn_blocking(move || git::diff(&root, &rel, &on_disk, 2 * 1024 * 1024)).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn git_outgoing(State(st): State<Arc<AppState>>) -> Response {
    let root = st.root.clone();
    match tokio::task::spawn_blocking(move || git::outgoing(&root)).await {
        Ok(c) => Json(c).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
struct CheckoutBody {
    branch: String,
    /// Only ever true when the user has seen the blocking files and said so.
    #[serde(default)]
    stash: bool,
}

async fn git_checkout(State(st): State<Arc<AppState>>, Json(b): Json<CheckoutBody>) -> Response {
    if b.branch.is_empty() || b.branch.starts_with('-') {
        return (StatusCode::BAD_REQUEST, "bad branch name").into_response();
    }
    git_do(&st, move |root| git::checkout(&root, &b.branch, b.stash)).await
}

#[derive(Deserialize)]
struct PathsBody {
    paths: Vec<String>,
}

async fn git_stage(State(st): State<Arc<AppState>>, Json(b): Json<PathsBody>) -> Response {
    if let Err(e) = sane_paths(&b.paths) {
        return e;
    }
    git_do(&st, move |root| git::stage(&root, &b.paths)).await
}

async fn git_unstage(State(st): State<Arc<AppState>>, Json(b): Json<PathsBody>) -> Response {
    if let Err(e) = sane_paths(&b.paths) {
        return e;
    }
    git_do(&st, move |root| git::unstage(&root, &b.paths)).await
}

#[derive(Deserialize)]
struct CommitBody {
    message: String,
}

#[derive(serde::Serialize)]
struct CommitResult {
    #[serde(flatten)]
    run: git::GitRun,
    #[serde(skip_serializing_if = "Option::is_none")]
    fetch_note: Option<String>,
}

async fn git_commit(State(st): State<Arc<AppState>>, Json(b): Json<CommitBody>) -> Response {
    if b.message.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, "empty commit message").into_response();
    }
    let root = st.root.clone();
    let out = tokio::task::spawn_blocking(move || git::commit(&root, &b.message)).await;
    *st.git.lock().await = None;
    match out {
        Ok((run, fetch_note)) => Json(CommitResult { run, fetch_note }).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn git_push(State(st): State<Arc<AppState>>) -> Response {
    git_do(&st, |root| git::push(&root)).await
}

async fn git_pull(State(st): State<Arc<AppState>>) -> Response {
    git_do(&st, |root| git::pull(&root)).await
}

async fn git_fetch(State(st): State<Arc<AppState>>) -> Response {
    git_do(&st, |root| git::fetch(&root)).await
}

async fn git_merge_abort(State(st): State<Arc<AppState>>) -> Response {
    git_do(&st, |root| git::merge_abort(&root)).await
}

// -------------------------------------------------------------- terminal ----

#[derive(Deserialize)]
struct TermQuery {
    #[serde(default = "default_cols")]
    cols: u16,
    #[serde(default = "default_rows")]
    rows: u16,
}
fn default_cols() -> u16 {
    120
}
fn default_rows() -> u16 {
    30
}

#[derive(Deserialize)]
#[serde(tag = "t")]
enum ClientMsg {
    #[serde(rename = "i")]
    Input { d: String },
    #[serde(rename = "r")]
    Resize { cols: u16, rows: u16 },
}

async fn ws_pty(ws: WebSocketUpgrade, State(st): State<Arc<AppState>>, Query(q): Query<TermQuery>) -> Response {
    ws.on_upgrade(move |socket| terminal_loop(socket, st, q))
}

async fn terminal_loop(socket: WebSocket, st: Arc<AppState>, q: TermQuery) {
    let (mut sink, mut stream) = socket.split();
    let (tx, mut rx) = tokio::sync::mpsc::channel::<FromPty>(512);

    let session = match PtySession::spawn(&st.shell, &st.root, q.cols.max(20), q.rows.max(5), tx) {
        Ok(s) => s,
        Err(e) => {
            let _ = sink.send(Message::Text(format!("\r\n[dbt-edith] cannot start shell: {e}\r\n").into())).await;
            return;
        }
    };

    loop {
        tokio::select! {
            from_pty = rx.recv() => match from_pty {
                Some(FromPty::Output(bytes)) => {
                    if sink.send(Message::Binary(bytes.into())).await.is_err() { break; }
                }
                _ => {
                    let _ = sink.send(Message::Text("\r\n[dbt-edith] shell exited\r\n".into())).await;
                    break;
                }
            },
            from_ws = stream.next() => match from_ws {
                Some(Ok(Message::Text(text))) => {
                    match serde_json::from_str::<ClientMsg>(&text) {
                        Ok(ClientMsg::Input { d }) => session.write(d.into_bytes()),
                        Ok(ClientMsg::Resize { cols, rows }) => session.resize(cols.max(20), rows.max(5)),
                        Err(_) => {}
                    }
                }
                Some(Ok(Message::Binary(bytes))) => session.write(bytes.to_vec()),
                Some(Ok(_)) => {}
                _ => break,
            },
        }
    }
    session.kill();
}

/// Keeps `origin/main` current for the freshness badge, which reads refs off
/// disk and never reaches the network itself. Ten minutes because the badge
/// only counts commits, and a fetch per poll would be a network call every five
/// seconds for an answer that changes a few times a day. Read-only, deadlined,
/// and silent on failure: offline, the badge says when the refs were last
/// refreshed instead of claiming the branch is up to date (0007).
pub async fn watch_remote(st: Arc<AppState>) {
    loop {
        let root = st.root.clone();
        if tokio::task::spawn_blocking(move || git::fetch_quiet(&root)).await.unwrap_or(false) {
            *st.fresh.lock().await = None;
        }
        tokio::time::sleep(std::time::Duration::from_secs(600)).await;
    }
}

/// Background poll: reloads whenever dbt rewrites manifest.json or catalog.json,
/// or something other than a click rewrites the column-lineage cache.
pub async fn watch_artifacts(st: Arc<AppState>) {
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        // Held through the reload: a click merging fetched lineage meanwhile
        // would otherwise merge into the graph this reload is about to replace.
        let _cache = st.cll_lock.lock().await;
        let (mp, cp, lp) = (st.manifest_path.clone(), st.catalog_path.clone(), st.cll());
        let stamps = tokio::task::spawn_blocking(move || [mtime_secs(&mp), mtime_secs(&cp), mtime_secs(&lp)])
            .await
            .unwrap_or([0, 0, 0]);
        let (changed, artifacts) = st
            .seen
            .lock()
            .map(|mut seen| {
                let moved = |i: usize| stamps[i] != 0 && stamps[i] != seen[i];
                let artifacts = moved(0) || moved(1);
                let changed = artifacts || moved(2);
                if changed {
                    *seen = stamps;
                }
                (changed, artifacts)
            })
            .unwrap_or((false, false));
        if !changed {
            continue;
        }
        // Give dbt a moment to finish writing.
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        if artifacts {
            if let Ok(g) = load_all(&st).await {
                eprintln!("  artifacts reloaded ({} nodes, {} ms)", g.nodes.len(), g.meta.load_ms);
                // A parse just landed, so the badge should turn green on the
                // next poll rather than a cache lifetime later.
                *st.fresh.lock().await = None;
            }
        } else {
            // Only the cache moved: the manifest under it is the one already read.
            let (base, lp) = (st.base.read().await.clone(), st.cll());
            if let Ok(g) = tokio::task::spawn_blocking(move || with_cache(&base, &lp)).await {
                *st.graph.write().await = Arc::new(g);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[test]
    fn host_must_name_this_server_exactly() {
        assert!(host_allowed(Some("127.0.0.1:4321"), 4321));
        assert!(host_allowed(Some("localhost:4321"), 4321));
        assert!(!host_allowed(Some("127.0.0.1:4322"), 4321));
        assert!(!host_allowed(Some("127.0.0.1"), 4321));
        assert!(!host_allowed(Some("evil.example:4321"), 4321));
        assert!(!host_allowed(Some("127.0.0.1:4321.evil.example"), 4321));
        assert!(!host_allowed(None, 4321));
    }

    #[test]
    fn origin_must_be_this_servers_page() {
        assert!(origin_allowed(Some("http://127.0.0.1:4321"), 4321, true));
        assert!(origin_allowed(Some("http://localhost:4321"), 4321, true));
        assert!(!origin_allowed(Some("https://evil.example"), 4321, false));
        assert!(!origin_allowed(Some("http://127.0.0.1:4322"), 4321, false));
        assert!(!origin_allowed(Some("null"), 4321, false));
        // Absent means a local tool, not a browser: fine for a plain request,
        // never for a WebSocket handshake, where browsers always send it.
        assert!(origin_allowed(None, 4321, false));
        assert!(!origin_allowed(None, 4321, true));
    }

    #[test]
    fn git_paths_stay_inside_the_repository_on_every_platform() {
        assert!(sane_path("models/stg_orders.sql"));
        assert!(sane_path("models\\stg_orders.sql"));
        assert!(!sane_path(""));
        assert!(!sane_path("/etc/passwd"));
        assert!(!sane_path("../secret"));
        assert!(!sane_path("..\\..\\secret"));
        assert!(!sane_path("models/../../secret"));
        assert!(!sane_path("C:/Users/me/.ssh/id_rsa"));
        assert!(!sane_path("C:\\Users\\me\\.ssh\\id_rsa"));
        assert!(!sane_path("\\\\server\\share\\x"));
    }

    fn temp_project(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dbt-edith-api-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("dbt_project.yml"), "name: demo\n").unwrap();
        std::fs::write(dir.join(".env"), "DBT_ENV_SECRET_PW=hunter2\n").unwrap();
        dir.canonicalize().unwrap()
    }

    /// A real server on a free port, so the guard is exercised the way a
    /// browser reaches it: raw HTTP over TCP, no client library.
    async fn serve(root: &Path) -> (u16, Arc<AppState>, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let manifest = root.join("target").join("manifest.json");
        // Never the developer's own configuration directory: a route that
        // saves a choice would write into it.
        let mut settings = crate::settings::Store::new(root);
        settings.path = None;
        let state: Arc<AppState> = Arc::new(AppState {
            port,
            root: root.to_path_buf(),
            manifest_path: manifest.clone(),
            catalog_path: root.join("target").join("catalog.json"),
            cll_path: std::sync::RwLock::new(root.join("target").join("column_lineage.json")),
            target_dir: root.join("target"),
            venv: VenvInfo::default(),
            file_index: RwLock::new(Arc::new(Vec::new())),
            settings,
            graph: RwLock::new(Arc::new(Graph::build(Default::default(), &manifest, 0, 0))),
            base: RwLock::new(Arc::new(Graph::build(Default::default(), &manifest, 0, 0))),
            cll_headers: Default::default(),
            git: tokio::sync::Mutex::new(None),
            fresh: tokio::sync::Mutex::new(None),
            shell: ShellSpec { program: "/bin/sh".into(), args: Vec::new() },
            sidecar: sidecar::Sidecar::new(false, Default::default()),
            snowflake_features: std::sync::Mutex::new(None),
            cll_lock: tokio::sync::Mutex::new(()),
            seen: std::sync::Mutex::new([0; 3]),
        });
        let held = state.clone();
        let task = tokio::spawn(async move {
            axum::serve(listener, router(state)).await.unwrap();
        });
        (port, held, task)
    }

    /// Sends one raw request and returns the status line.
    async fn status_of(port: u16, request: String) -> String {
        head_of(port, request).await.lines().next().unwrap_or("").to_string()
    }

    /// The whole response head, for asserting on headers.
    async fn head_of(port: u16, request: String) -> String {
        let mut s = tokio::net::TcpStream::connect(("127.0.0.1", port)).await.unwrap();
        s.write_all(request.as_bytes()).await.unwrap();
        let mut buf = Vec::new();
        let read = async {
            let mut chunk = [0u8; 1024];
            loop {
                let n = s.read(&mut chunk).await.unwrap_or(0);
                if n == 0 {
                    break;
                }
                buf.extend_from_slice(&chunk[..n]);
                if buf.windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
            }
        };
        let _ = tokio::time::timeout(std::time::Duration::from_secs(5), read).await;
        let text = String::from_utf8_lossy(&buf).into_owned();
        // The last chunk usually carries the start of the body with it.
        text.split("\r\n\r\n").next().unwrap_or("").to_string()
    }

    /// The whole response, head and body, for a route whose answer matters.
    async fn body_of(port: u16, request: String) -> String {
        let mut s = tokio::net::TcpStream::connect(("127.0.0.1", port)).await.unwrap();
        s.write_all(request.as_bytes()).await.unwrap();
        let mut buf = Vec::new();
        let _ = tokio::time::timeout(std::time::Duration::from_secs(5), s.read_to_end(&mut buf)).await;
        String::from_utf8_lossy(&buf).into_owned()
    }

    fn get(path: &str, headers: &[(&str, &str)]) -> String {
        raw("GET", path, headers)
    }

    fn raw(method: &str, path: &str, headers: &[(&str, &str)]) -> String {
        let mut r = format!("{method} {path} HTTP/1.1\r\n");
        for (k, v) in headers {
            r.push_str(&format!("{k}: {v}\r\n"));
        }
        r.push_str("Content-Length: 0\r\nConnection: close\r\n\r\n");
        r
    }

    /// The graph `serve()` builds is empty, so this asserts on status codes and
    /// on the shape of the answer. What a selector actually matches is settled
    /// in `select.rs`, against a graph with nodes in it.
    #[tokio::test]
    async fn a_selector_answers_this_page_and_names_what_it_could_not_parse() {
        let root = temp_project("selector");
        let (port, _st, server) = serve(&root).await;
        let host = format!("127.0.0.1:{port}");
        let h = [("Host", host.as_str())];

        let ok = body_of(port, get("/api/select?q=dim_customers", &h)).await;
        assert!(ok.starts_with("HTTP/1.1 200"), "{ok}");
        assert!(ok.contains(r#""matched":0"#), "{ok}");
        assert!(ok.contains("nothing matches"), "an empty graph matches nothing, and says so");
        assert!(ok.contains(r#""mode":"select""#), "{ok}");
        assert!(!ok.contains(r#""focus""#), "a selection sends no focus");

        for (query, code) in [
            ("q=", "empty"),
            ("q=state%3Amodified", "unknown_method"),
            ("q=--wat%20a", "unknown_flag"),
            ("q=a%2C%2Cb", "empty_term"),
            ("q=--selector%20nightly", "unknown_selector"),
            ("q=--selector%20a%20b", "selector_alone"),
            ("q=--selector", "missing_selector"),
        ] {
            let body = body_of(port, get(&format!("/api/select?{query}"), &h)).await;
            assert!(body.starts_with("HTTP/1.1 400"), "{query}: {body}");
            assert!(body.contains(code), "{query}: {body}");
        }

        // A pasted file is refused before any of it is parsed.
        let long = format!("/api/select?q={}", "x".repeat(5000));
        assert!(status_of(port, get(&long, &h)).await.contains("400"));

        // Read-only, so the guard asks for the Host and nothing more.
        assert!(status_of(port, get("/api/select?q=a", &[("Host", "evil.test")])).await.ends_with("403 Forbidden"));

        server.abort();
        let _ = std::fs::remove_dir_all(&root);
    }

    /// What a named selector keeps is settled in `selectors.rs`. This is what
    /// only the route decides: the tests box filters the drawing and never the
    /// answer, a model drawn for a test's sake says so, and the list names
    /// the selector it cannot resolve rather than hiding it.
    #[tokio::test]
    async fn a_named_selector_is_listed_and_the_tests_box_only_filters_it() {
        let root = temp_project("named");
        std::fs::write(root.join("selectors.yml"), "selectors:\n  - indirect_selection: buildable\n").unwrap();
        let (port, st, server) = serve(&root).await;
        let raw: RawManifest = serde_json::from_value(serde_json::json!({
            "nodes": {
                "model.demo.orders": { "name": "orders", "resource_type": "model", "package_name": "demo", "fqn": ["demo", "orders"] },
                "test.demo.not_null_orders_id": {
                    "name": "not_null_orders_id", "resource_type": "test", "package_name": "demo",
                    "fqn": ["demo", "not_null_orders_id"],
                },
            },
            "parent_map": { "test.demo.not_null_orders_id": ["model.demo.orders"] },
            "selectors": {
                "checks": { "name": "checks", "description": "Every test.", "definition": { "method": "resource_type", "value": "test" } },
                "ci": { "name": "ci", "definition": { "method": "state", "value": "modified" } },
            },
        }))
        .unwrap();
        let mut graph = Graph::build(raw, &root.join("target").join("manifest.json"), 0, 0);
        graph.selectors.check_against(&root);
        *st.graph.write().await = Arc::new(graph);
        let host = format!("127.0.0.1:{port}");
        let h = [("Host", host.as_str())];

        let list = body_of(port, get("/api/selectors", &h)).await;
        assert!(list.starts_with("HTTP/1.1 200"), "{list}");
        assert!(list.contains(r#"{"name":"checks","description":"Every test."}"#), "{list}");
        assert!(list.contains(r#""name":"ci","unsupported":"#), "listed, with its reason: {list}");
        assert!(list.contains(r#""lost_indirect":true"#), "{list}");

        let on = body_of(port, get("/api/select?q=--selector%20checks&tests=1", &h)).await;
        assert!(on.starts_with("HTTP/1.1 200"), "{on}");
        assert!(on.contains(r#""selector":"checks""#) && on.contains(r#""description":"Every test.""#), "{on}");
        assert!(on.contains(r#""matched":1"#) && on.contains(r#""names":["not_null_orders_id"]"#), "{on}");
        assert_eq!(on.matches(r#""context":true"#).count(), 1, "the model is drawn for the test's sake: {on}");

        // Off, the answer is the same one test, and none of it is drawn.
        let off = body_of(port, get("/api/select?q=--selector%20checks&tests=0", &h)).await;
        assert!(off.contains(r#""matched":1"#) && off.contains(r#""hidden_tests":1"#), "{off}");
        assert!(off.contains(r#""nodes":[]"#), "{off}");

        let pasted = body_of(port, get("/api/select?q=dbt%20ls%20--selector%20checks", &h)).await;
        assert!(pasted.contains(r#""stripped":true"#), "{pasted}");
        let ci = body_of(port, get("/api/select?q=--selector%20ci", &h)).await;
        assert!(ci.starts_with("HTTP/1.1 400") && ci.contains("unsupported_selector"), "{ci}");

        assert!(status_of(port, get("/api/selectors", &[("Host", "evil.test")])).await.ends_with("403 Forbidden"));
        server.abort();
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Which macro a call reaches is settled in `macros.rs`. This is the part
    /// only a server can check: a link is sent only for a file that is really
    /// in the project, and only to this page.
    #[tokio::test]
    async fn a_macro_call_links_only_to_a_file_in_the_project() {
        let root = temp_project("macros");
        std::fs::create_dir_all(root.join("macros")).unwrap();
        std::fs::write(root.join("macros").join("cents.sql"), "{% macro cents(x) %}{{ x }} / 100{% endmacro %}\n").unwrap();
        let (port, st, server) = serve(&root).await;
        let raw: RawManifest = serde_json::from_value(serde_json::json!({
            "metadata": { "project_name": "demo" },
            "macros": {
                "macro.demo.cents": { "name": "cents", "package_name": "demo", "original_file_path": "macros/cents.sql" },
                "macro.dbt.is_incremental": {
                    "name": "is_incremental", "package_name": "dbt",
                    "original_file_path": "macros/materializations/models/incremental/is_incremental.sql",
                },
            },
        }))
        .unwrap();
        *st.graph.write().await = Arc::new(Graph::build(raw, &root.join("target").join("manifest.json"), 0, 0));
        let host = format!("127.0.0.1:{port}");
        let own = format!("http://{host}");
        let body = r#"{"file":"models/orders.sql","calls":["cents","dbt.is_incremental","is_incremental","log","cents"]}"#;

        let answer = body_of(port, with_json("POST", "/api/macros/resolve", &[("Host", &host), ("Origin", &own)], body)).await;
        assert!(answer.starts_with("HTTP/1.1 200"), "{answer}");
        assert!(answer.contains(r#""call":"cents""#), "{answer}");
        assert!(answer.contains(r#""file":"macros/cents.sql""#), "{answer}");
        // `dbt.is_incremental` resolves, to a file in site-packages, so only the
        // disk check keeps it out. A bare `is_incremental` never reaches dbt's
        // package from a model, and `log` is no macro at all.
        assert_eq!(answer.matches(r#""call":"#).count(), 1, "one link, asked for twice: {answer}");

        let foreign = with_json("POST", "/api/macros/resolve", &[("Host", &host), ("Origin", "https://evil.example")], body);
        assert!(status_of(port, foreign).await.ends_with("403 Forbidden"));

        server.abort();
        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn the_guard_refuses_other_pages_and_other_hosts() {
        let root = temp_project("guard");
        let (port, _st, server) = serve(&root).await;
        let host = format!("127.0.0.1:{port}");
        let own = format!("http://127.0.0.1:{port}");

        // The browser's own page keeps working, through either name.
        assert!(status_of(port, get("/api/file?path=.env", &[("Host", &host)])).await.ends_with("200 OK"));
        assert!(status_of(port, get("/api/meta", &[("Host", &format!("localhost:{port}"))])).await.ends_with("200 OK"));
        assert!(status_of(port, raw("POST", "/api/reload", &[("Host", &host), ("Origin", &own)])).await.ends_with("400 Bad Request"));
        // A local tool sends no Origin: still allowed on a plain request.
        assert!(status_of(port, raw("POST", "/api/reload", &[("Host", &host)])).await.ends_with("400 Bad Request"));

        // DNS rebinding: same IP, another name in Host.
        assert!(status_of(port, get("/api/file?path=.env", &[("Host", "evil.example")])).await.ends_with("403 Forbidden"));
        assert!(status_of(port, get("/api/file?path=.env", &[])).await.ends_with("403 Forbidden"));

        // Cross-site POST: no preflight protects a bodyless request, the guard does.
        let cross = raw("POST", "/api/git/fetch", &[("Host", &host), ("Origin", "https://evil.example")]);
        assert!(status_of(port, cross).await.ends_with("403 Forbidden"));
        let wrong_port = raw("POST", "/api/git/fetch", &[("Host", &host), ("Origin", "http://127.0.0.1:1")]);
        assert!(status_of(port, wrong_port).await.ends_with("403 Forbidden"));

        // The terminal: a handshake from another page, or with no Origin at all.
        let ws = |origin: Option<&str>| {
            let mut h = vec![
                ("Host", host.as_str()),
                ("Connection", "Upgrade"),
                ("Upgrade", "websocket"),
                ("Sec-WebSocket-Version", "13"),
                ("Sec-WebSocket-Key", "dGhlIHNhbXBsZSBub25jZQ=="),
            ];
            if let Some(o) = origin {
                h.push(("Origin", o));
            }
            get("/ws/pty", &h)
        };
        assert!(status_of(port, ws(Some("https://evil.example"))).await.ends_with("403 Forbidden"));
        assert!(status_of(port, ws(None)).await.ends_with("403 Forbidden"));
        assert!(status_of(port, ws(Some(&own))).await.ends_with("101 Switching Protocols"));

        server.abort();
        let _ = std::fs::remove_dir_all(&root);
    }

    fn with_json(method: &str, path: &str, headers: &[(&str, &str)], body: &str) -> String {
        let mut r = format!("{method} {path} HTTP/1.1\r\n");
        for (k, v) in headers {
            r.push_str(&format!("{k}: {v}\r\n"));
        }
        r.push_str(&format!("Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()));
        r
    }

    // Every route that can lead to warehouse queries under the user's own
    // identity, or start the script that makes them, answers this page alone,
    // and nothing runs while Snowflake is not the picked tool.
    #[tokio::test]
    async fn snowflake_lineage_answers_only_this_page_and_only_when_picked() {
        let root = temp_project("snowflake");
        let (port, st, server) = serve(&root).await;
        let host = format!("127.0.0.1:{port}");
        let own = format!("http://127.0.0.1:{port}");
        let fetch = r#"{"id":"model.shop.dim_customers","column":"customer_id","relation":"analytics.marts.dim_customers"}"#;

        for (path, body) in [
            ("/api/collineage/source", r#"{"tool":"snowflake"}"#),
            ("/api/collineage/fetch", fetch),
            ("/api/features", r#"{"snowflake":true}"#),
        ] {
            let foreign = with_json("POST", path, &[("Host", &host), ("Origin", "https://evil.example")], body);
            assert!(status_of(port, foreign).await.ends_with("403 Forbidden"), "{path}");
        }
        // A plain read cannot start anything, and the old switch is gone.
        assert!(status_of(port, get("/api/collineage/fetch", &[("Host", &host)])).await.ends_with("405 Method Not Allowed"));
        assert!(status_of(port, get("/api/features", &[("Host", &host)])).await.ends_with("405 Method Not Allowed"));
        assert!(status_of(port, get("/api/sidecar", &[("Host", &host)])).await.ends_with("200 OK"));
        let old = with_json("POST", "/api/sidecar", &[("Host", &host), ("Origin", &own)], r#"{"enabled":true}"#);
        assert!(status_of(port, old).await.ends_with("405 Method Not Allowed"));

        // Features on, Snowflake not picked: still nothing runs.
        *st.snowflake_features.lock().unwrap() = Some(true);
        let off = with_json("POST", "/api/collineage/fetch", &[("Host", &host), ("Origin", &own)], fetch);
        let answer = body_of(port, off).await;
        assert!(answer.starts_with("HTTP/1.1 409"), "{answer}");
        assert!(answer.contains(NOT_PICKED), "{answer}");

        server.abort();
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Why `base` exists: merging adds the columns a cache knows and the YAML
    /// does not, and never removes them, so switching merges into a copy of the
    /// graph as the manifest and catalog left it.
    #[test]
    fn a_cache_laid_over_another_keeps_none_of_its_columns() {
        let root = temp_project("base");
        let target = root.join("target");
        std::fs::create_dir_all(&target).unwrap();
        let model = |name: &str| {
            serde_json::json!({
                "name": name, "resource_type": "model", "package_name": "demo",
                "original_file_path": format!("models/{name}.sql"),
                "columns": { "id": { "name": "id" } },
            })
        };
        let manifest = serde_json::json!({
            "metadata": { "project_name": "demo" },
            "nodes": { "model.demo.stg": model("stg"), "model.demo.orders": model("orders") },
        });
        std::fs::write(target.join("manifest.json"), manifest.to_string()).unwrap();
        let cache = |file: &str, source: &str, col: &str| {
            let edge = serde_json::json!({ "from": "model.demo.stg", "from_col": col, "to": "model.demo.orders", "to_col": col });
            let body = serde_json::json!({ "version": 1, "source": source, "edges": [edge] });
            std::fs::write(target.join(file), body.to_string()).unwrap();
            target.join(file)
        };
        let (a, b) = (cache("column_lineage.collin.json", "collin", "only_a"), cache("column_lineage.x.json", "x", "only_b"));
        let columns = |g: &Graph| -> Vec<String> {
            g.nodes[g.index["model.demo.orders"] as usize].columns.iter().map(|c| c.name.clone()).collect()
        };

        let base = load_base(&root, &target.join("manifest.json"), &target.join("catalog.json")).unwrap();
        let over_a = with_cache(&base, &a);
        assert_eq!(columns(&over_a), ["id", "only_a"]);
        assert_eq!(columns(&with_cache(&base, &b)), ["id", "only_b"]);
        assert_eq!(columns(&with_cache(&base, Path::new(""))), ["id"], "no cache, no lineage columns");
        assert_eq!(columns(&base), ["id"], "the base is never merged into");

        // What switching over the graph on screen would have drawn.
        let mut laid_over = over_a.clone();
        merge_cache(&mut laid_over, &b);
        assert_eq!(columns(&laid_over), ["id", "only_a", "only_b"]);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Off, Snowflake is not offered at all: not its tool, not a cache it
    /// wrote, not the profile. The adapter decides until the user does (0031).
    #[tokio::test]
    async fn snowflake_is_offered_only_while_its_features_are_on() {
        let root = temp_project("features");
        let target = root.join("target");
        std::fs::create_dir_all(&target).unwrap();
        let manifest = |adapter: &str| {
            let body = format!(r#"{{"metadata":{{"project_name":"demo","adapter_type":"{adapter}"}}}}"#);
            std::fs::write(target.join("manifest.json"), body).unwrap();
        };
        let cache = |file: &str, source: &str| {
            std::fs::write(target.join(file), format!(r#"{{"version":1,"source":"{source}","edges":[]}}"#)).unwrap();
        };
        cache("column_lineage.snowflake.json", "snowflake");
        cache("column_lineage.collin.json", "collin");
        manifest("postgres");
        let (port, st, server) = serve(&root).await;
        let load = || Arc::new(load_base(&root, &target.join("manifest.json"), &target.join("catalog.json")).unwrap());
        *st.base.write().await = load();
        *st.graph.write().await = load();
        let host = format!("127.0.0.1:{port}");
        let own = format!("http://127.0.0.1:{port}");
        let post = |path: &str, body: &str| with_json("POST", path, &[("Host", &host), ("Origin", &own)], body);
        let fetch = r#"{"id":"model.demo.orders","column":"id","relation":"analytics.marts.orders"}"#;

        let meta = body_of(port, get("/api/meta", &[("Host", &host)])).await;
        assert!(meta.contains(r#""features":{"snowflake":false,"snowflake_set":false}"#), "{meta}");
        assert!(meta.contains("column_lineage.collin.json"), "{meta}");
        assert!(!meta.contains("column_lineage.snowflake.json"), "not even listed: {meta}");
        for (path, body) in [
            ("/api/collineage/source", r#"{"tool":"snowflake"}"#),
            ("/api/collineage/source", r#"{"file":"column_lineage.snowflake.json"}"#),
            ("/api/collineage/fetch", fetch),
        ] {
            assert!(status_of(port, post(path, body)).await.ends_with("409 Conflict"), "{path} {body}");
        }
        assert!(status_of(port, get("/api/profiles", &[("Host", &host)])).await.ends_with("409 Conflict"));
        let put = with_json("PUT", "/api/profiles", &[("Host", &host), ("Origin", &own)], r#"{"content":"x"}"#);
        assert!(status_of(port, put).await.ends_with("409 Conflict"));

        let picked = body_of(port, post("/api/collineage/source", r#"{"tool":"collin"}"#)).await;
        assert!(picked.starts_with("HTTP/1.1 200"), "{picked}");
        assert!(picked.contains(r#""cll_active":"column_lineage.collin.json","cll_tool":"collin""#), "{picked}");
        assert!(status_of(port, post("/api/collineage/source", r#"{"tool":"fusion"}"#)).await.ends_with("404 Not Found"));
        assert!(status_of(port, post("/api/collineage/source", r#"{"tool":"../x"}"#)).await.ends_with("400 Bad Request"));
        assert!(status_of(port, post("/api/collineage/source", "{}")).await.ends_with("400 Bad Request"));

        // A Snowflake manifest offers it without being asked.
        manifest("snowflake");
        *st.base.write().await = load();
        *st.graph.write().await = load();
        let meta = body_of(port, get("/api/meta", &[("Host", &host)])).await;
        assert!(meta.contains(r#""features":{"snowflake":true,"snowflake_set":false}"#), "{meta}");
        assert!(meta.contains("column_lineage.snowflake.json"), "{meta}");
        // Picked by its file, a cache is read as it is: no script starts.
        let picked = body_of(port, post("/api/collineage/source", r#"{"file":"column_lineage.snowflake.json"}"#)).await;
        assert!(picked.contains(r#""cll_tool":"snowflake""#), "{picked}");
        assert!(!st.sidecar.enabled());

        // Switched off, the user's choice wins over the adapter, and the graph
        // no longer holds Snowflake's answer.
        let off = body_of(port, post("/api/features", r#"{"snowflake":false}"#)).await;
        assert!(off.starts_with("HTTP/1.1 200"), "{off}");
        assert!(off.contains(r#""features":{"snowflake":false,"snowflake_set":true}"#), "{off}");
        assert!(off.contains(r#""cll_active":"column_lineage.collin.json","cll_tool":"collin""#), "{off}");
        assert!(status_of(port, get("/api/profiles", &[("Host", &host)])).await.ends_with("409 Conflict"));

        server.abort();
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The one file outside the project dbt-edith opens, and only because the
    /// script named it: the route itself takes no path at all (0017).
    #[cfg(unix)]
    #[tokio::test]
    async fn the_profile_is_read_and_written_where_the_script_said() {
        let root = temp_project("profile");
        let (port, st, server) = serve(&root).await;
        *st.snowflake_features.lock().unwrap() = Some(true);
        let host = format!("127.0.0.1:{port}");
        let own = format!("http://127.0.0.1:{port}");
        let put = |body: &str| with_json("PUT", "/api/profiles", &[("Host", &host), ("Origin", &own)], body);

        // Nothing has named a profile yet, so there is nothing to open.
        let none = body_of(port, get("/api/profiles", &[("Host", &host)])).await;
        assert!(none.starts_with("HTTP/1.1 409") && none.contains(NO_PROFILE), "{none}");
        assert!(status_of(port, put(r#"{"content":"x"}"#)).await.ends_with("409 Conflict"));
        // And no other page may write it.
        let foreign = with_json("PUT", "/api/profiles", &[("Host", &host), ("Origin", "https://evil.example")], r#"{"content":"x"}"#);
        assert!(status_of(port, foreign).await.ends_with("403 Forbidden"));

        // A script that names a profile, outside the project on purpose.
        let outside = std::env::temp_dir().join(format!("dbt-edith-profile-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&outside);
        std::fs::create_dir_all(&outside).unwrap();
        let profile = outside.join("profiles.yml");
        std::fs::write(&profile, "shop:\n  target: dev\n").unwrap();
        let script = outside.join("fake.sh");
        let announce = format!(
            "echo '{{\"event\":\"profiles\",\"path\":\"{}\"}}'\necho '{{\"event\":\"ready\"}}'\nwhile IFS= read -r line; do :; done\n",
            profile.display(),
        );
        std::fs::write(&script, announce).unwrap();
        let sh = [crate::sidecar::Interpreter { program: "/bin/sh".into(), args: Vec::new() }];
        st.sidecar.set_enabled(true);
        assert_eq!(st.sidecar.start(&sh, &script, &outside).await.state, "ready");

        let answer = body_of(port, get("/api/profiles", &[("Host", &host)])).await;
        assert!(answer.contains("target: dev"), "{answer}");
        assert!(answer.contains(&profile.display().to_string()), "{answer}");

        let saved = body_of(port, put(r#"{"content":"shop:\n  target: prod\n"}"#)).await;
        assert!(saved.contains("\"restarted\":true"), "{saved}");
        assert_eq!(std::fs::read_to_string(&profile).unwrap(), "shop:\n  target: prod\n");

        st.sidecar.stop().await;
        server.abort();
        let _ = std::fs::remove_dir_all(&outside);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn every_reply_carries_the_security_headers() {
        let root = temp_project("headers");
        let (port, _st, server) = serve(&root).await;
        let host = format!("127.0.0.1:{port}");
        for path in ["/", "/api/meta"] {
            let head = head_of(port, get(path, &[("Host", &host)])).await.to_lowercase();
            assert!(head.contains("x-content-type-options: nosniff"), "{path}: {head}");
            assert!(head.contains("referrer-policy: no-referrer"), "{path}: {head}");
            // Clickjacking the terminal is the one this closes.
            assert!(head.contains("frame-ancestors 'none'"), "{path}: {head}");
            assert!(head.contains("default-src 'none'"), "{path}: {head}");
            // The stray whitespace a line continuation leaves behind is not
            // wrong, but it is a sign the policy was edited carelessly.
            assert!(!head.contains("  "), "double space in a header: {head}");
        }
        server.abort();
        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn a_diff_path_cannot_leave_the_project() {
        let root = temp_project("diff");
        let (port, _st, server) = serve(&root).await;
        let host = format!("127.0.0.1:{port}");
        for p in ["..%5C..%5Cx", "../x", "C:/x", "C:%5Cx"] {
            let line = status_of(port, get(&format!("/api/git/diff?path={p}"), &[("Host", &host)])).await;
            assert!(line.ends_with("400 Bad Request"), "{p}: {line}");
        }
        // An absolute path is read relative to the root, as the editor does.
        let line = status_of(port, get("/api/git/diff?path=/etc/passwd", &[("Host", &host)])).await;
        assert!(line.ends_with("200 OK"), "{line}");
        // A deleted file in a deleted directory is still a valid diff target.
        let line = status_of(port, get("/api/git/diff?path=gone/away.sql", &[("Host", &host)])).await;
        assert!(line.ends_with("200 OK"), "{line}");
        server.abort();
        let _ = std::fs::remove_dir_all(&root);
    }
}
