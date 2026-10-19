//! dbt-edith: a small browser IDE for dbt projects.
//! Editor + terminal + lineage, served from one self-contained binary.

mod api;
mod clone;
mod collin;
mod collin_compare;
mod collin_report;
mod collin_run;
mod command;
mod compiled;
mod envs;
mod files;
mod freshness;
mod git;
mod graph;
mod incremental;
mod jobs;
mod ls;
mod ls_column;
mod macros;
mod manifest;
mod profiles;
mod program;
mod project;
mod run_results;
mod pty;
mod redact;
mod select;
mod selectors;
mod settings;
mod sidecar;
mod tour;
mod update;
mod venv;
mod yaml;

use clap::Parser;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Parser)]
/// `version` is composed rather than taken from Cargo.toml alone: the package
/// version only moves at a release, and the question after reinstalling is
/// which build this is (see `build.rs`).
#[command(
    name = "dbt-edith",
    version = concat!(env!("CARGO_PKG_VERSION"), " (", env!("DBT_EDITH_BUILD"), ")"),
    about = "Editor, terminal and dbt lineage in the browser"
)]
struct Args {
    /// dbt project root
    #[arg(default_value = ".")]
    project: PathBuf,

    /// Port to listen on (the next free port is used if this one is taken)
    #[arg(short, long, default_value_t = 4321)]
    port: u16,

    /// Path to manifest.json (default: <project>/target/manifest.json)
    #[arg(long)]
    manifest: Option<PathBuf>,

    /// Path to catalog.json (default: <project>/target/catalog.json; for --run-collin, catalog.json beside the manifest)
    #[arg(long)]
    catalog: Option<PathBuf>,

    /// Path to the column lineage cache (default: the tool last picked in the menu, else the newest target/column_lineage*.json; for --column, collin's or Fusion's newest, else the newest)
    #[arg(long)]
    column_lineage: Option<PathBuf>,

    /// Shell for the integrated terminal (default: $SHELL, or Git Bash on Windows)
    #[arg(long)]
    shell: Option<String>,

    /// Do not open a browser window on startup
    #[arg(long)]
    no_open: bool,

    /// Never look on GitHub for a newer dbt-edith: no hourly look from midnight to 09:00 Paris time, and no Check for updates in Edith's menu
    #[arg(long)]
    no_update_check: bool,

    /// Resolve this dbt selection against the manifest, print it and exit, without serving
    #[arg(short = 's', long, value_name = "LINE", allow_hyphen_values = true)]
    select: Option<String>,

    /// Leave these nodes out of --select, in the same syntax
    #[arg(long, value_name = "LINE", allow_hyphen_values = true, requires = "select")]
    exclude: Option<String>,

    /// Resolve a selector of selectors.yml by name, print it and exit, without serving
    #[arg(long, value_name = "NAME", conflicts_with = "select")]
    selector: Option<String>,

    /// Walk one column's lineage over the column lineage cache beside the manifest, print it and exit, without serving
    #[arg(long, value_name = "MODEL.COLUMN", conflicts_with_all = ["select", "selector"])]
    column: Option<String>,

    /// Levels --column walks upstream: a number, or all (default 0)
    #[arg(long, value_name = "N", value_parser = ls_column::levels, requires = "column")]
    up: Option<u32>,

    /// Levels --column walks downstream: a number, or all (default 0)
    #[arg(long, value_name = "N", value_parser = ls_column::levels, requires = "column")]
    down: Option<u32>,

    /// Roles --column does not walk past downstream, comma-separated: window,aggregate
    #[arg(long, value_name = "ROLES", value_delimiter = ',', requires = "column")]
    stop_at: Vec<String>,

    /// --column crosses collin's indirect edges too, a filter's, a join key's or a dedup key's, which it leaves out by default
    #[arg(long, requires = "column")]
    indirect: bool,

    /// The tool whose cache --column reads (default: collin's or Fusion's newest, else the newest)
    #[arg(long, value_enum, value_name = "TOOL", requires = "column", conflicts_with = "column_lineage")]
    column_tool: Option<ls_column::Tool>,

    /// Run collin over the manifest first, and answer --column from the cache it writes beside the manifest
    #[arg(long, requires = "column", conflicts_with_all = ["column_lineage", "column_tool"])]
    run_collin: bool,

    /// The folder dbt compiled into, one folder per package, that --run-collin reads a model's SQL from when the manifest has none (default: compiled/ beside the manifest; needs collin 0.9.0)
    #[arg(long, value_name = "DIR", requires = "run_collin")]
    compiled: Option<PathBuf>,

    /// What --select, --selector and --column print: one name per line, or one JSON object per node, per model for --column
    #[arg(long, value_enum, value_name = "FORMAT", default_value = "name")]
    output: ls::Output,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    // Answered before anything a server needs: no port, no key, no settings
    // read or written, no catalog, no watcher (0057).
    let query = match (args.select, args.selector) {
        (Some(select), _) => Some(ls::Query::Line { select, exclude: args.exclude.unwrap_or_default() }),
        (None, Some(name)) => Some(ls::Query::Named(name)),
        (None, None) => None,
    };
    if let Some(query) = query {
        std::process::exit(ls::run(&args.project, args.manifest.as_deref(), &query, args.output));
    }
    if let Some(column) = args.column {
        let ask = ls_column::Ask {
            column,
            up: args.up.unwrap_or(0),
            down: args.down.unwrap_or(0),
            stop_at: args.stop_at,
            tool: args.column_tool,
            cache: args.column_lineage,
            indirect: args.indirect,
            produce: args.run_collin.then(|| ls_column::Produce {
                catalog: args.catalog,
                compiled: args.compiled,
                indirect: args.indirect,
            }),
        };
        std::process::exit(ls_column::run(&args.project, args.manifest.as_deref(), &ask, args.output));
    }
    let root = args
        .project
        .canonicalize()
        .map(files::plain)
        .map_err(|e| anyhow::anyhow!("cannot open {}: {e}", args.project.display()))?;
    if !files::is_dbt_project(&root) {
        eprintln!("  no dbt_project.yml in {} - point dbt-edith at the dbt project root", root.display());
    }
    let manifest_path = args.manifest.unwrap_or_else(|| root.join("target").join("manifest.json"));
    let catalog_path = args.catalog.unwrap_or_else(|| root.join("target").join("catalog.json"));
    // The store is built here rather than inside AppState because the cache the
    // user last chose decides which file is loaded before the state exists.
    let settings = settings::Store::new(&root);
    let saved = settings.load();
    let target_dir = files::folder_of(&manifest_path).unwrap_or_else(|| root.join("target"));

    // Loaded without column lineage first. That is the base every cache is
    // merged into, and whether a Snowflake cache may be loaded at all depends on
    // the adapter this manifest names (0031).
    let base = if manifest_path.exists() {
        eprintln!("  reading {}", manifest_path.display());
        let (project, path, cat) = (root.clone(), manifest_path.clone(), catalog_path.clone());
        let g = tokio::task::spawn_blocking(move || api::load_base(&project, &path, &cat)).await??;
        let c = &g.meta.counts;
        eprintln!(
            "  {} nodes in {} ms  ({} models, {} sources, {} tests, {} macros)",
            g.nodes.len(),
            g.meta.load_ms,
            c.get("model").unwrap_or(&0),
            c.get("source").unwrap_or(&0),
            c.get("test").unwrap_or(&0),
            g.macros.len(),
        );
        if g.meta.catalog_columns > 0 {
            eprintln!("  catalog.json merged ({} columns typed)", g.meta.catalog_columns);
        } else {
            eprintln!("  no catalog.json - column types stay as declared in YAML (run dbt compile --write-catalog)");
        }
        g
    } else {
        eprintln!("  no manifest at {} - lineage stays empty until dbt writes one", manifest_path.display());
        graph::Graph::build(Default::default(), &manifest_path, 0, 0)
    };

    let snowflake = settings::snowflake_features(saved.snowflake_features, &base.meta.adapter);
    let live = saved.snowflake_lineage && snowflake;
    let found = collin::discover(&target_dir);
    let cll_path = match args.column_lineage.clone() {
        // Named on the command line, and still not loaded if it is Snowflake's
        // answer while Snowflake's features are off.
        Some(path) if !snowflake && collin::tool_in(&path) == "snowflake" => {
            eprintln!("  {} not loaded: it holds Snowflake's lineage, and Snowflake's features are off", path.display());
            PathBuf::new()
        }
        // A bare name as `./name`, the way `folder_of` names the folder of a
        // bare `--manifest`: the paths built from that folder are compared
        // with this one, and `x` is not `./x` to a Path.
        Some(path) if path.parent().is_some_and(|p| p.as_os_str().is_empty()) => Path::new(".").join(path),
        Some(path) => path,
        None => {
            // A project can hold one cache per producer: the saved choice, else
            // the most recent that may be offered.
            let offered: Vec<&str> = found
                .iter()
                .filter(|a| collin::offered(a, snowflake))
                .map(|a| if a.source.is_empty() { a.file.as_str() } else { a.source.as_str() })
                .collect();
            if offered.len() > 1 {
                eprintln!("  {} column lineage caches: {}", offered.len(), offered.join(", "));
            }
            collin::choose(&target_dir, &found, saved.cll_file.as_deref(), live, snowflake)
        }
    };
    let (base, graph) = {
        let path = cll_path.clone();
        tokio::task::spawn_blocking(move || {
            let graph = api::with_cache(&base, &path);
            (base, graph)
        })
        .await?
    };

    let seen = [graph.meta.manifest_mtime, graph.meta.catalog_mtime, graph.meta.cll_mtime];
    let shell = pty::ShellSpec::detect(args.shell);
    let venv = venv::detect(&root, &shell);
    // Bound before the state exists: the guard in api.rs compares Host and
    // Origin against the port actually taken, which may not be the one asked for.
    let listener = bind(args.port).await?;
    // The key is in the link and nowhere else: whoever opens the page from it
    // gets the cookie the API asks for, and nobody else can (0052).
    let key = api::new_key();
    let url = format!("http://{}/?key={key}", listener.local_addr()?);
    let config_dir = settings::config_dir(|key| std::env::var_os(key), cfg!(windows));
    // The running binary, by the one path the update names and the page
    // watches for a replacement alike (0068, 0095). One that cannot be made
    // canonical gets no command, and is still watched where it was started.
    let started = std::env::current_exe().ok();
    let exe = started.as_ref().and_then(|p| p.canonicalize().ok()).map(files::plain);
    // Switched off, it neither asks GitHub nor looks at where it was built,
    // and shows nothing, so the reason it gives is never read.
    let how = if args.no_update_check { Err(update::Why::Elsewhere) } else { update_how(&shell, exe.as_deref()) };
    let update = update::Update::new(args.no_update_check, config_dir.as_deref().map(update::seen_path), how)
        .watching(exe.or(started));
    let state = Arc::new(api::AppState {
        port: listener.local_addr()?.port(),
        root: root.clone(),
        manifest_path,
        catalog_path,
        cll_path: std::sync::RwLock::new(cll_path),
        target_dir,
        venv: venv.clone(),
        file_index: tokio::sync::RwLock::new(Arc::new(files::scan(&root))),
        settings,
        graph: tokio::sync::RwLock::new(Arc::new(graph)),
        base: tokio::sync::RwLock::new(Arc::new(base)),
        cll_headers: Default::default(),
        git: tokio::sync::Mutex::new(None),
        fresh: tokio::sync::Mutex::new(None),
        shell: shell.clone(),
        sidecar: sidecar::Sidecar::new(live, Default::default()),
        // Nothing runs here: collin is found and started only once asked (0042).
        collin: collin_run::Runner::new(collin_run::DEADLINE),
        collin_report: Default::default(),
        collin_version: Default::default(),
        collin_installs: config_dir.as_deref().map(collin_run::installs_dir),
        collin_chosen: std::sync::Mutex::new(saved.collin_version.clone()),
        snowflake_features: std::sync::Mutex::new(saved.snowflake_features),
        cll_lock: tokio::sync::Mutex::new(()),
        seen: std::sync::Mutex::new(seen),
        profiles: profiles::Lookup::from_env(|name| std::env::var_os(name), cfg!(windows)),
        key,
        // Nothing is read here: the first look at the Jobs tab reads them (0063).
        job_sources: std::sync::Mutex::new(saved.job_sources.clone()),
        jobs: Default::default(),
        run_results: Default::default(),
        update,
        tour: tour::Offer { path: config_dir.as_deref().map(tour::offered_path) },
    });

    tokio::spawn(api::watch_artifacts(state.clone()));
    tokio::spawn(api::watch_remote(state.clone()));
    tokio::spawn(api::watch_files(state.clone()));
    tokio::spawn(api::watch_update(state.clone()));
    if live {
        // Starting runs no query: the script connects on the first click (0016).
        let st = state.clone();
        tokio::spawn(async move { st.sidecar.start_for(&st.root, &st.venv).await });
    }

    eprintln!("\n  dbt-edith  {}  ({})", env!("CARGO_PKG_VERSION"), env!("DBT_EDITH_BUILD"));
    eprintln!("  project   {}", root.display());
    eprintln!("  shell     {} {}", shell.program, shell.args.join(" "));
    if !venv.chosen.name.is_empty() {
        eprintln!(
            "  venv      {} ({}{})",
            venv.chosen.name,
            if venv.source == "activated" { "activated" } else { "found in project, not activated" },
            if venv.chosen.python.is_empty() { String::new() } else { format!(", python {}", venv.chosen.python) },
        );
    }
    if live {
        eprintln!("  column lineage from Snowflake, connecting on the first column click");
    }
    // What the last look found, by this start or another: the look this one
    // makes is in the background, and the page says what it finds.
    let kept = if state.update.off { None } else { state.update.kept() };
    if let Some(seen) = kept.filter(|s| update::newer(env!("CARGO_PKG_VERSION"), &s.latest)) {
        eprintln!("  update    dbt-edith {} is out, and the version in the status bar offers it", seen.latest);
    }
    eprintln!("  open      {url}");
    eprintln!("            the key in it is this launch's: a tab opened without it can read nothing\n");

    if !args.no_open {
        open_browser(&url);
    }

    axum::serve(listener, api::router(state.clone()))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
            eprintln!("\n  bye");
        })
        .await?;
    state.sidecar.stop().await;
    Ok(())
}

/// The command that updates this binary, from the clone it was built from
/// when that clone is still on this machine, or why there is none (0068).
fn update_how(shell: &pty::ShellSpec, exe: Option<&Path>) -> Result<update::How, update::Why> {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).canonicalize().map_err(|_| update::Why::Elsewhere)?;
    let source = files::plain(source);
    let exe = exe.ok_or(update::Why::Elsewhere)?;
    update::how(&source, exe, update::shell_kind(&shell.program), cfg!(windows))
}

/// Binds to localhost only, walking forward if the port is already in use.
async fn bind(port: u16) -> anyhow::Result<tokio::net::TcpListener> {
    for candidate in port..port.saturating_add(20) {
        match tokio::net::TcpListener::bind(("127.0.0.1", candidate)).await {
            Ok(l) => return Ok(l),
            Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => continue,
            Err(e) => return Err(e.into()),
        }
    }
    anyhow::bail!("no free port in {}..{}", port, port.saturating_add(20))
}

fn open_browser(url: &str) {
    #[cfg(target_os = "macos")]
    let mut cmd = {
        let mut c = std::process::Command::new("open");
        c.arg(url);
        c
    };
    // From the PATH, never beside this binary, where a repository could
    // have committed a `cmd.exe` that would run at every start (0094).
    #[cfg(target_os = "windows")]
    let mut cmd = {
        let Some(cmd) = program::on_path("cmd") else { return };
        let mut c = std::process::Command::new(cmd);
        c.args(["/C", "start", "", url]);
        c
    };
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    let mut cmd = {
        let mut c = std::process::Command::new("xdg-open");
        c.arg(url);
        c
    };
    let _ = cmd.stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn();
}
