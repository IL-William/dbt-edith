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
mod describe;
mod envs;
mod files;
mod freshness;
mod git;
mod graph;
mod incremental;
mod job;
mod jobs;
mod ls;
mod look;
mod ls_column;
mod macro_use;
mod macros;
mod manifest;
mod mcp;
mod mcp_graphs;
mod mcp_line;
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
mod skill;
mod tour;
mod unused_paths;
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

    /// Path to catalog.json (default: <project>/target/catalog.json; for --run-collin, --describe and --overview, catalog.json beside the manifest)
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

    /// Print one node's card, as the Catalog shows it, and exit, without serving: a model, source (source_name.table), seed, snapshot, test or exposure by name, or any node by unique_id; several, comma-separated
    #[arg(long, value_name = "NAMES", value_delimiter = ',', conflicts_with_all = ["select", "selector", "column"])]
    describe: Vec<String>,

    /// --describe prints the node's own SQL file after its card
    #[arg(long, requires = "describe")]
    sql: bool,

    /// Print what the project holds, by kind, materialization and folder, and exit, without serving
    #[arg(long, conflicts_with_all = ["select", "selector", "column", "describe"])]
    overview: bool,

    /// Print one DAG or dbt Cloud job, step by step, with the nodes each step runs, and exit, without serving: its id or name. Needs .dbt-edith/jobs.yml, which the Jobs settings write
    #[arg(long, value_name = "NAME", conflicts_with_all = ["select", "selector", "column", "describe", "overview"])]
    job: Option<String>,

    /// Count what --select or --selector matched instead of listing it, one row per value of KEY and a total
    #[arg(long, value_enum, value_name = "KEY")]
    count_by: Option<ls::CountBy>,

    /// How many folder levels --overview lists, or --count-by folder counts (default 3)
    #[arg(long, value_name = "N")]
    depth: Option<usize>,

    /// What --select, --selector, --column, --describe, --overview and --job print: one name per line, one JSON object per node (per model for --column, one for --overview or --job), or compact tab-separated lines; --describe, --overview and --job print a text card unless json; with --count-by, json prints one object and name and compact print the same table
    #[arg(long, value_enum, value_name = "FORMAT", default_value = "name")]
    output: ls::Output,

    /// Answer an MCP client over stdio, through one tool that takes these flags as a command line, and exit when it closes stdin, without serving. Only PROJECT, --manifest and --catalog go with it
    #[arg(
        long,
        conflicts_with_all = [
            "port", "column_lineage", "shell", "no_open", "no_update_check", "select", "exclude", "selector", "column", "up", "down",
            "stop_at", "indirect", "column_tool", "run_collin", "compiled", "describe", "sql", "overview", "job", "count_by", "depth", "output", "skill",
        ]
    )]
    mcp: bool,

    /// Print the agent skill that matches this version, a SKILL.md for a client to load, and exit. It takes nothing beside it and reads, writes and starts nothing
    #[arg(
        long,
        conflicts_with_all = [
            "project", "port", "manifest", "catalog", "column_lineage", "shell", "no_open", "no_update_check", "select", "exclude", "selector",
            "column", "up", "down", "stop_at", "indirect", "column_tool", "run_collin", "compiled", "describe", "sql", "overview", "job", "count_by",
            "depth", "output", "mcp",
        ]
    )]
    skill: bool,
}

/// The first flag that is set beside a mode it does not belong to, as a
/// sentence. clap's `requires` is not enforced when the flag it requires
/// conflicts with one that is present, so `--overview --sql` parses, and the
/// flag would be dropped without a word.
fn stray(args: &Args) -> Option<String> {
    let flags = [
        (args.sql, "--sql", "--describe", !args.describe.is_empty()),
        (args.exclude.is_some(), "--exclude", "--select", args.select.is_some()),
        (args.count_by.is_some(), "--count-by", "--select or --selector", args.select.is_some() || args.selector.is_some()),
        (args.depth.is_some(), "--depth", "--overview or --count-by folder", args.overview || args.count_by == Some(ls::CountBy::Folder)),
        (args.up.is_some(), "--up", "--column", args.column.is_some()),
        (args.down.is_some(), "--down", "--column", args.column.is_some()),
        (!args.stop_at.is_empty(), "--stop-at", "--column", args.column.is_some()),
        (args.indirect, "--indirect", "--column", args.column.is_some()),
        (args.column_tool.is_some(), "--column-tool", "--column", args.column.is_some()),
        (args.run_collin, "--run-collin", "--column", args.column.is_some()),
        (args.compiled.is_some(), "--compiled", "--run-collin", args.run_collin),
        // A cache may be named for the page as for `--column`; the two modes
        // that read none would drop it without a word.
        (
            args.column_lineage.is_some(),
            "--column-lineage",
            "--column, or the page",
            args.column.is_some()
                || (args.describe.is_empty() && !args.overview && args.job.is_none() && args.select.is_none() && args.selector.is_none()),
        ),
    ];
    let stray = flags.iter().find(|(set, _, _, asked)| *set && !asked).map(|(_, flag, mode, _)| format!("{flag} goes with {mode}"));
    // Every folder is deeper than level 0, so a count to that depth would be
    // one row of nothing. The overview's 0 leaves its folders out on purpose.
    stray.or_else(|| {
        (args.count_by == Some(ls::CountBy::Folder) && args.depth == Some(0)).then(|| "--depth 0 counts no folder; use 1 or more".to_string())
    })
}

/// What the command line asks, when it asks a question and does not serve.
/// Beside the project, the manifest and the catalog, which come from where
/// the question is asked: the command line's arguments, or the MCP server's
/// own.
enum Question {
    /// A selection, listed or, with a `Count`, counted (0112).
    Select(ls::Query, Option<ls::Count>),
    Describe { names: Vec<String>, sql: bool },
    Overview { depth: usize },
    Job { name: String },
    Column(ls_column::Ask),
}

/// The question the arguments ask, or None for the page. One mapping from
/// flags to mode, so the command line and the MCP tool cannot read the same
/// flags two ways. A selection is looked at first, then a name to describe,
/// the overview, a job and the column, which is the order the modes were added in.
fn question_of(args: &Args) -> Option<Question> {
    // The depth's 3 is applied here, as the overview's is below.
    let count = args.count_by.map(|by| ls::Count { by, depth: args.depth.unwrap_or(3) });
    match (&args.select, &args.selector) {
        (Some(select), _) => {
            return Some(Question::Select(
                ls::Query::Line { select: select.clone(), exclude: args.exclude.clone().unwrap_or_default() },
                count,
            ))
        }
        (None, Some(name)) => return Some(Question::Select(ls::Query::Named(name.clone()), count)),
        (None, None) => {}
    }
    if !args.describe.is_empty() {
        return Some(Question::Describe { names: args.describe.clone(), sql: args.sql });
    }
    if args.overview {
        // Not clap's `default_value_t`: a flag with a default is always set,
        // and `stray` could not tell it was asked for.
        return Some(Question::Overview { depth: args.depth.unwrap_or(3) });
    }
    if let Some(name) = &args.job {
        return Some(Question::Job { name: name.clone() });
    }
    args.column.as_ref().map(|column| {
        Question::Column(ls_column::Ask {
            column: column.clone(),
            up: args.up.unwrap_or(0),
            down: args.down.unwrap_or(0),
            stop_at: args.stop_at.clone(),
            tool: args.column_tool,
            cache: args.column_lineage.clone(),
            indirect: args.indirect,
            produce: args.run_collin.then(|| ls_column::Produce {
                catalog: args.catalog.clone(),
                compiled: args.compiled.clone(),
                indirect: args.indirect,
            }),
        })
    })
}

impl Question {
    /// The mode, from the command line's arguments to the exit code.
    fn run(&self, args: &Args) -> i32 {
        let (project, manifest, catalog) = (&args.project, args.manifest.as_deref(), args.catalog.as_deref());
        match self {
            Question::Select(query, count) => ls::run(project, manifest, query, *count, args.output),
            Question::Describe { names, sql } => describe::run_describe(project, manifest, catalog, names, *sql, args.output),
            Question::Overview { depth } => describe::run_overview(project, manifest, catalog, *depth, args.output),
            Question::Job { name } => job::run_job(project, manifest, catalog, name, args.output),
            Question::Column(ask) => ls_column::run(project, manifest, ask, args.output),
        }
    }
}

/// What `--mcp` starts on: the project, canonical, and the manifest and the
/// catalog if they were named, each made whole against the folder the server
/// was started in. Neither is looked for here: a manifest that is not there
/// is each call's error, so a client can be configured before `dbt parse`.
fn mcp_start(args: &Args) -> Result<(PathBuf, Option<PathBuf>, Option<PathBuf>), String> {
    let root = match args.project.canonicalize() {
        Ok(p) if p.is_dir() => files::plain(p),
        Ok(p) => return Err(format!("error: cannot open {}: not a folder", p.display())),
        Err(e) => return Err(format!("error: cannot open {}: {e}", args.project.display())),
    };
    let whole = |path: &PathBuf| std::path::absolute(path).unwrap_or_else(|_| path.clone());
    Ok((root, args.manifest.as_ref().map(whole), args.catalog.as_ref().map(whole)))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    // First of all, ahead of even `--mcp`: it is a file printed, and nothing
    // else is read, written or started (0114).
    if args.skill {
        std::process::exit(skill::print());
    }
    // Before everything a server needs: no port, no settings, no update look,
    // no browser, and no file read until a call asks (0110).
    if args.mcp {
        std::process::exit(match mcp_start(&args) {
            Ok((root, manifest, catalog)) => mcp::serve(root, manifest, catalog),
            Err(why) => {
                eprintln!("{why}");
                ls::CANNOT_RUN
            }
        });
    }
    if let Some(sentence) = stray(&args) {
        eprintln!("error: {sentence}");
        std::process::exit(ls::CANNOT_RUN);
    }
    // Answered before anything a server needs: no port, no key, no settings
    // read or written, no catalog, no watcher (0057).
    if let Some(question) = question_of(&args) {
        std::process::exit(question.run(&args));
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
        look: look::Look { path: config_dir.as_deref().map(look::look_path) },
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

#[cfg(test)]
mod tests {
    use super::*;

    fn asked(line: &str) -> Args {
        match Args::try_parse_from(std::iter::once("dbt-edith").chain(line.split_whitespace())) {
            Ok(args) => args,
            Err(e) => panic!("{line}: {e}"),
        }
    }

    #[test]
    fn a_flag_of_another_mode_is_named_and_not_dropped() {
        for (line, flag) in [
            ("--overview --sql", "--sql goes with --describe"),
            ("--describe orders --depth 2", "--depth goes with --overview or --count-by folder"),
            ("-s orders --depth 2", "--depth goes with --overview or --count-by folder"),
            ("-s orders --count-by kind --depth 2", "--depth goes with --overview or --count-by folder"),
            ("-s orders --count-by folder --depth 0", "--depth 0 counts no folder; use 1 or more"),
            ("--count-by kind", "--count-by goes with --select or --selector"),
            ("--describe orders --count-by kind", "--count-by goes with --select or --selector"),
            ("--overview --count-by folder", "--count-by goes with --select or --selector"),
            ("--column a.b --count-by tag", "--count-by goes with --select or --selector"),
            ("--describe orders --exclude resource_type:test", "--exclude goes with --select"),
            ("--describe orders --up 1", "--up goes with --column"),
            ("--overview --indirect", "--indirect goes with --column"),
            ("--describe orders --run-collin", "--run-collin goes with --column"),
            ("--select orders --down 1", "--down goes with --column"),
            ("--overview --stop-at window", "--stop-at goes with --column"),
            ("--column a.b --column-lineage c.json --compiled x", "--compiled goes with --run-collin"),
            ("--column a.b --column-tool collin --compiled x", "--compiled goes with --run-collin"),
            ("--overview --column-lineage n.json", "--column-lineage goes with --column, or the page"),
            ("--describe orders --column-lineage n.json", "--column-lineage goes with --column, or the page"),
            ("-s orders --column-lineage n.json", "--column-lineage goes with --column, or the page"),
            ("--job jaffle_daily --depth 2", "--depth goes with --overview or --count-by folder"),
            ("--job jaffle_daily --count-by kind", "--count-by goes with --select or --selector"),
            ("--job jaffle_daily --up 1", "--up goes with --column"),
            ("--job jaffle_daily --column-lineage n.json", "--column-lineage goes with --column, or the page"),
        ] {
            assert_eq!(stray(&asked(line)).as_deref(), Some(flag), "{line}");
        }
    }

    #[test]
    fn mcp_goes_alone_with_the_project_the_manifest_and_the_catalog() {
        for line in ["--mcp", "--mcp /work/jaffle_shop", "--mcp . --manifest m.json", "--mcp --manifest m.json --catalog c.json /work/jaffle_shop"] {
            let args = asked(line);
            assert!(args.mcp && question_of(&args).is_none() && stray(&args).is_none(), "{line}");
        }
        assert!(!asked("--overview").mcp);
    }

    #[test]
    fn mcp_is_refused_beside_every_other_flag_and_names_it() {
        // A default is not typed: `--mcp` alone parses with the port and the
        // output it would otherwise have.
        let args = asked("--mcp");
        assert_eq!((args.port, args.output), (4321, ls::Output::Name));
        for (flag, line) in [
            ("--port", "--mcp --port 4321"),
            ("--column-lineage", "--mcp --column-lineage c.json"),
            ("--shell", "--mcp --shell bash"),
            ("--no-open", "--mcp --no-open"),
            ("--no-update-check", "--mcp --no-update-check"),
            ("--select", "--mcp -s orders"),
            ("--exclude", "--mcp --exclude x"),
            ("--selector", "--mcp --selector nightly"),
            ("--column", "--mcp --column a.b"),
            ("--up", "--mcp --up 1"),
            ("--down", "--mcp --down 1"),
            ("--stop-at", "--mcp --stop-at window"),
            ("--indirect", "--mcp --indirect"),
            ("--column-tool", "--mcp --column-tool collin"),
            ("--run-collin", "--mcp --run-collin"),
            ("--compiled", "--mcp --compiled sql"),
            ("--describe", "--mcp --describe orders"),
            ("--sql", "--mcp --sql"),
            ("--overview", "--mcp --overview"),
            ("--job", "--mcp --job jaffle_daily"),
            ("--count-by", "--mcp --count-by kind"),
            ("--depth", "--mcp --depth 2"),
            ("--output", "--mcp --output json"),
            ("--skill", "--mcp --skill"),
        ] {
            let why = match Args::try_parse_from(std::iter::once("dbt-edith").chain(line.split_whitespace())) {
                Ok(_) => panic!("{line} was accepted"),
                Err(e) => e.to_string(),
            };
            assert!(why.contains(&format!("'{flag}")) && why.contains("'--mcp"), "{line}: {why}");
        }
        // Every flag of the command line is on the list, so one added later
        // is refused beside --mcp until someone decides it is not.
        use clap::CommandFactory;
        let command = Args::command();
        let conflicts: std::collections::BTreeSet<&str> =
            command.get_arg_conflicts_with(command.get_arguments().find(|a| a.get_id() == "mcp").unwrap()).iter().map(|a| a.get_id().as_str()).collect();
        let others: std::collections::BTreeSet<&str> =
            command.get_arguments().map(|a| a.get_id().as_str()).filter(|id| !["mcp", "project", "manifest", "catalog"].contains(id)).collect();
        assert_eq!(conflicts, others);
    }

    #[test]
    fn skill_goes_alone_and_every_other_flag_is_refused_beside_it() {
        // As with --mcp, a default is not typed: `--skill` alone parses with
        // the project and the port it would otherwise have.
        let args = asked("--skill");
        assert!(args.skill && !args.mcp && question_of(&args).is_none() && stray(&args).is_none());
        assert_eq!((args.port, args.output), (4321, ls::Output::Name));
        for line in [
            "--skill .",
            "--skill /work/jaffle_shop",
            "--skill --manifest m.json",
            "--skill --catalog c.json",
            "--skill --port 4321",
            "--skill -s orders",
            "--skill --describe orders",
            "--skill --overview",
            "--skill --job jaffle_daily",
            "--skill --output json",
            "--skill --mcp",
            "--mcp --skill",
        ] {
            let why = match Args::try_parse_from(std::iter::once("dbt-edith").chain(line.split_whitespace())) {
                Ok(_) => panic!("{line} was accepted"),
                Err(e) => e.to_string(),
            };
            assert!(why.contains("'--skill"), "{line}: {why}");
        }
        // Every other flag is on its list, so one added later is refused
        // beside --skill until someone decides it is not.
        use clap::CommandFactory;
        let command = Args::command();
        let conflicts: std::collections::BTreeSet<&str> =
            command.get_arg_conflicts_with(command.get_arguments().find(|a| a.get_id() == "skill").unwrap()).iter().map(|a| a.get_id().as_str()).collect();
        let others: std::collections::BTreeSet<&str> = command.get_arguments().map(|a| a.get_id().as_str()).filter(|id| *id != "skill").collect();
        assert_eq!(conflicts, others);
    }

    #[test]
    fn mcp_starts_on_a_project_folder_and_reads_nothing_else() {
        let dir = std::env::temp_dir().join(format!("dbt-edith-mcp-start-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let (root, manifest, catalog) = mcp_start(&asked(&format!("--mcp {}", dir.display()))).unwrap();
        assert_eq!((root, manifest, catalog), (files::plain(dir.canonicalize().unwrap()), None, None));
        // Named, and not there: not looked for.
        let (_, manifest, catalog) = mcp_start(&asked(&format!("--mcp {} --manifest m.json --catalog /x/c.json", dir.display()))).unwrap();
        assert_eq!(manifest, Some(std::path::absolute("m.json").unwrap()));
        assert_eq!(catalog, Some(PathBuf::from("/x/c.json")));
        let why = mcp_start(&asked(&format!("--mcp {}", dir.join("nowhere").display()))).unwrap_err();
        assert!(why.starts_with("error: cannot open ") && why.contains("nowhere"), "{why}");
        std::fs::write(dir.join("file"), "").unwrap();
        let why = mcp_start(&asked(&format!("--mcp {}", dir.join("file").display()))).unwrap_err();
        assert!(why.starts_with("error: cannot open ") && why.ends_with("not a folder"), "{why}");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_flags_of_a_mode_beside_it_are_no_stray() {
        for line in [
            "--describe orders --sql",
            "--overview --depth 2",
            "-s a --count-by folder --depth 2",
            "--selector nightly --count-by kind",
            "-s a --exclude b",
            "--column a.b --up 1",
            "--column a.b --down all --stop-at window --indirect --column-tool collin",
            "--column a.b --run-collin --compiled compiled",
            "--column a.b --column-lineage c.json",
            "--column-lineage c.json",
            "--overview",
            "--job jaffle_daily",
            "--job jaffle_daily --output json",
            "",
        ] {
            assert_eq!(stray(&asked(line)), None, "{line}");
        }
    }

    #[test]
    fn each_mode_is_the_question_its_flags_ask() {
        let text = |line: &str| match question_of(&asked(line)) {
            Some(Question::Select(ls::Query::Line { select, exclude }, None)) => format!("select {select} | {exclude}"),
            Some(Question::Select(ls::Query::Named(name), None)) => format!("selector {name}"),
            Some(Question::Select(_, Some(ls::Count { by, depth }))) => format!("count {by:?} {depth}"),
            Some(Question::Describe { names, sql }) => format!("describe {} {sql}", names.join(",")),
            Some(Question::Overview { depth }) => format!("overview {depth}"),
            Some(Question::Job { name }) => format!("job {name}"),
            Some(Question::Column(ask)) => format!(
                "column {} {} {} {:?} produce={}",
                ask.column,
                ask.up,
                ask.down,
                ask.stop_at,
                ask.produce.as_ref().is_some_and(|p| p.indirect)
            ),
            None => "page".to_string(),
        };
        assert_eq!(text("-s stg_orders+1 --exclude resource_type:test"), "select stg_orders+1 | resource_type:test");
        assert_eq!(text("--selector nightly"), "selector nightly");
        assert_eq!(text("--describe orders,customers --sql"), "describe orders,customers true");
        assert_eq!(text("-s orders --count-by kind"), "count Kind 3");
        assert_eq!(text("--selector nightly --count-by folder --depth 1"), "count Folder 1");
        assert_eq!(text("--overview"), "overview 3");
        assert_eq!(text("--overview --depth 1"), "overview 1");
        assert_eq!(text("--job jaffle_daily"), "job jaffle_daily");
        assert_eq!(text("--column orders.amount --up 2 --stop-at window"), "column orders.amount 2 0 [\"window\"] produce=false");
        assert_eq!(text("--column orders.amount --run-collin --indirect"), "column orders.amount 0 0 [] produce=true");
        assert_eq!(text(""), "page");
    }

    #[test]
    fn a_job_goes_alone_among_the_questions() {
        for line in ["--job a --overview", "--job a --describe orders", "--job a -s orders", "--job a --selector s", "--job a --column a.b"] {
            let why = match Args::try_parse_from(std::iter::once("dbt-edith").chain(line.split_whitespace())) {
                Ok(_) => panic!("{line} was accepted"),
                Err(e) => e.to_string(),
            };
            assert!(why.contains("cannot be used with") && why.contains("--job"), "{line}: {why}");
        }
    }

    #[test]
    fn a_flag_with_no_mode_at_all_is_refused_by_clap() {
        assert!(Args::try_parse_from(["dbt-edith", "--sql"]).is_err());
    }
}
