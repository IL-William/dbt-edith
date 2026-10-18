//! The `--column` mode: one column's lineage, walked over a column lineage
//! cache already beside the manifest, printed, and the process ends (0088).
//!
//! Invariants: the walk is the canvas's own, `Graph::column_lineage`, so the
//! command line and the page cannot disagree. Nothing writes lineage here:
//! collin is never started and Snowflake never asked, whatever the cache's
//! age. A cache older than the manifest still answers, and stderr says so with
//! the steps that would write a new one, the target left for the user to
//! name. stdout carries the answer and nothing else.

use crate::collin::{self, iso_utc, Available, RawColLineage};
use crate::graph::{ColRef, Graph, Kind, Lineage};
use crate::ls::{self, Output, ANSWERED, CANNOT_RUN, REFUSED};
use crate::{collin_run, files, profiles};
use std::collections::{BTreeMap, HashMap};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

/// The levels `all` stands for: more than any project has, and still an `i32`
/// for the walk's signed depths.
pub const ALL: u32 = i32::MAX as u32;

/// The tool whose cache answers, as the column lineage menu names them.
#[derive(clap::ValueEnum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tool {
    Fusion,
    Collin,
    Snowflake,
}

impl Tool {
    fn name(self) -> &'static str {
        match self {
            Tool::Fusion => "fusion",
            Tool::Collin => "collin",
            Tool::Snowflake => "snowflake",
        }
    }
}

/// What the command line asked.
pub struct Ask {
    /// `model.column`, as typed.
    pub column: String,
    pub up: u32,
    pub down: u32,
    /// Roles the walk downstream stops at, as the canvas's stop at menu.
    pub stop_at: Vec<String>,
    pub tool: Option<Tool>,
    /// A cache named outright by `--column-lineage`.
    pub cache: Option<PathBuf>,
    /// `--indirect`: cross collin's indirect edges, which the answer leaves
    /// out by default, as the canvas does (0097).
    pub indirect: bool,
}

/// `--up` and `--down`: a number of levels, or `all`.
pub fn levels(text: &str) -> Result<u32, String> {
    match text.trim() {
        "all" => Ok(ALL),
        n => n.parse::<u32>().map(|n| n.min(ALL)).map_err(|_| format!("{text} is neither a number of levels nor all")),
    }
}

/// The whole mode, from the command line's arguments to the exit code.
pub fn run(project: &Path, manifest: Option<&Path>, ask: &Ask, output: Output) -> i32 {
    let (root, manifest_path, mut graph) = match ls::open(project, manifest) {
        Ok(opened) => opened,
        Err(code) => return code,
    };
    let target_dir = files::folder_of(&manifest_path).unwrap_or_else(|| root.join("target"));
    // A file named outright needs no listing, which reads every cache.
    let found = if ask.cache.is_some() { Vec::new() } else { collin::discover(&target_dir) };
    let mut err = io::stderr().lock();
    let cache = match cache_for(&target_dir, &graph, ask, &found) {
        Ok(path) => path,
        Err(why) => {
            let _ = writeln!(err, "error: {why}");
            let _ = regenerate(&root, &graph, &target_dir, ask.tool.map_or("", Tool::name), &mut err);
            return CANNOT_RUN;
        }
    };
    let raw = match load(&cache) {
        Ok(raw) => raw,
        Err(why) => {
            let _ = writeln!(err, "error: {why}");
            return CANNOT_RUN;
        }
    };
    let producer = raw.producer.clone();
    graph.merge_col_lineage(raw, files::mtime_secs(&cache));
    graph.meta.cll_file = cache.display().to_string();

    let mut out = BufWriter::new(io::stdout().lock());
    let code = match answer(&graph, ask, output, &mut out, &mut err).and_then(|code| out.flush().map(|_| code)) {
        Ok(code) => code,
        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => return ANSWERED,
        Err(e) => {
            let _ = writeln!(err, "error: cannot write the answer: {e}");
            return CANNOT_RUN;
        }
    };
    // After the answer, as for `--select`: the freshness walk is most of the
    // time on a large project. A refusal says which cache it looked in too.
    let _ = ls::provenance(&root, &graph, &mut err);
    let _ = cache_lines(&root, &graph, &cache, &manifest_path, &producer, &mut err);
    code
}

/// The cache at `path`, or why it is not one. Discovery passes over a file
/// without edges; a file named outright is checked here instead, so collin's
/// report named by mistake is said rather than answered as no edges.
fn load(path: &Path) -> Result<RawColLineage, String> {
    let not_one = || format!("{} is not a column lineage cache: it has no version and no edges", path.display());
    match RawColLineage::load(path) {
        Ok(raw) if raw.version == 0 && raw.edges.is_empty() => Err(not_one()),
        Ok(raw) => Ok(raw),
        // A manifest or a catalog, the likeliest files to name by mistake,
        // fail on a field of their own before the check above is reached.
        Err(_) if shape(path).is_some_and(|(version, edges)| !version && !edges) => Err(not_one()),
        Err(e) => Err(format!("cannot read {}: {e}", path.display())),
    }
}

/// Whether a JSON file has a `version` and an `edges` key, every value
/// skipped: None when it is not a JSON object at all.
fn shape(path: &Path) -> Option<(bool, bool)> {
    #[derive(serde::Deserialize)]
    struct Shape {
        version: Option<serde::de::IgnoredAny>,
        edges: Option<serde::de::IgnoredAny>,
    }
    let s: Shape = serde_json::from_slice(&std::fs::read(path).ok()?).ok()?;
    Some((s.version.is_some(), s.edges.is_some()))
}

/// The cache that answers: the one named, the tool's newest, collin's or
/// Fusion's newest, or the one the page loads when nothing was picked. Never
/// a file still to be written: a tool without a cache is an error, Snowflake's
/// included.
fn cache_for(target_dir: &Path, graph: &Graph, ask: &Ask, found: &[Available]) -> Result<PathBuf, String> {
    if let Some(path) = &ask.cache {
        return if path.is_file() { Ok(path.clone()) } else { Err(format!("no column lineage cache at {}", path.display())) };
    }
    if let Some(tool) = ask.tool {
        // Named outright, so Snowflake's switch, kept in the settings this
        // mode does not read, has no say: a file on disk asks nothing of the
        // warehouse.
        return collin::latest_of(found, tool.name())
            .map(|a| target_dir.join(&a.file))
            .ok_or_else(|| format!("no column lineage from {} in {}", tool.name(), target_dir.display()));
    }
    // A tool that reads every model before Snowflake's, which holds only the
    // columns fetched so far: the newest cache would often be that one, and
    // its answer partial without a word.
    if let Some(a) = found.iter().find(|a| a.tool == "collin" || a.tool == "fusion") {
        return Ok(target_dir.join(&a.file));
    }
    let snowflake = crate::settings::snowflake_features(None, &graph.meta.adapter);
    let path = collin::choose(target_dir, found, None, false, snowflake);
    if path.is_file() {
        Ok(path)
    } else {
        Err(format!("no column lineage cache in {}", target_dir.display()))
    }
}

/// Resolves the column and prints its lineage. Returns the exit code; an
/// `Err` is only ever a failed write.
fn answer(graph: &Graph, ask: &Ask, output: Output, out: &mut impl Write, err: &mut impl Write) -> io::Result<i32> {
    let at = match focus(graph, &ask.column) {
        Ok(at) => at,
        Err(why) => {
            writeln!(err, "error: {why}")?;
            return Ok(REFUSED);
        }
    };
    // `unknown` is how a role the cache left empty is shown, so it is how
    // one is asked for.
    let cut: Vec<&str> =
        ask.stop_at.iter().map(|r| r.trim()).filter(|r| !r.is_empty()).map(|r| if r == "unknown" { "" } else { r }).collect();
    let kinds: &[String] = graph.cll.as_ref().map_or(&[], |c| c.kinds.as_slice());
    for role in cut.iter().filter(|r| !kinds.iter().any(|k| k == *r)) {
        let role = if role.is_empty() { "unknown" } else { role };
        writeln!(err, "warning: no edge of this cache has the role {role}; its roles are {}", roles(kinds))?;
    }
    // No cap: a caller asking for every level wants all of them, as
    // `--select` prints a whole selection.
    let lineage = graph.column_lineage(at, ask.up, ask.down, usize::MAX, &cut, ask.indirect);
    let models = reached(graph, &lineage, ask);
    for m in &models {
        match output {
            Output::Name => {
                for column in m.columns.keys() {
                    writeln!(out, "{}.{column}", m.label)?;
                }
            }
            Output::Json => {
                serde_json::to_writer(&mut *out, m)?;
                writeln!(out)?;
            }
        }
    }
    if !lineage.stopped_at.is_empty() {
        let at: Vec<&str> = lineage.stopped_at.iter().map(|r| if r.is_empty() { "unknown" } else { r }).collect();
        writeln!(err, "stopped downstream at {}: what follows those columns is left out, counted in more_down", at.join(", "))?;
    }
    if ask.up == 0 && ask.down == 0 {
        writeln!(err, "only the column itself: --down N or all for what it feeds, --up N or all for what feeds it")?;
    } else if lineage.nodes.len() == 1 {
        writeln!(err, "no edge of this cache reaches or leaves {} in the direction asked", ask.column)?;
    }
    // Said whatever was found: an agent reading the default answer learns
    // that the cache holds edges it left out, and how to ask for them.
    if !ask.indirect && graph.cll.as_ref().is_some_and(|c| c.has_indirect()) {
        writeln!(err, "collin's indirect edges, a filter's, a join key's or a dedup key's, are left out: --indirect crosses them too")?;
    }
    Ok(ANSWERED)
}

/// The roles a cache holds, for a message.
fn roles(kinds: &[String]) -> String {
    let named: Vec<&str> = kinds.iter().map(|k| if k.is_empty() { "unknown" } else { k.as_str() }).collect();
    if named.is_empty() {
        "none".to_string()
    } else {
        named.join(", ")
    }
}

/// `model.column`, `source_name.table.column` or `unique_id.column`: the
/// column is what follows the last dot, the node what `--output name` prints
/// for it, or its unique_id.
fn focus(graph: &Graph, spec: &str) -> Result<ColRef, String> {
    let Some((node, column)) = spec.trim().rsplit_once('.').filter(|(n, c)| !n.is_empty() && !c.is_empty()) else {
        return Err(format!("{spec}: write the column as model.column"));
    };
    let idx = node_named(graph, node)?;
    match graph.col_slot(idx, column) {
        Some(col) => Ok(ColRef { node: idx, col }),
        None => Err(format!("{node} has no column {column}: {}", columns_with_lineage(graph, idx))),
    }
}

/// The node a name stands for: its unique_id, else the one model, seed,
/// snapshot or source of that name, an enabled one over a disabled twin. An
/// exposure or an analysis has no columns, so one of the same name is no
/// rival.
fn node_named(graph: &Graph, name: &str) -> Result<u32, String> {
    if let Some(&i) = graph.index.get(name) {
        return Ok(i);
    }
    let named = |exact: bool| -> Vec<u32> {
        (0..graph.nodes.len() as u32)
            .filter(|&i| {
                let n = &graph.nodes[i as usize];
                matches!(n.kind, Kind::Model | Kind::Seed | Kind::Snapshot | Kind::Source)
                    && if exact { n.name == name } else { n.name.eq_ignore_ascii_case(name) }
            })
            .collect()
    };
    let mut hits = named(true);
    if hits.is_empty() {
        hits = named(false);
    }
    if hits.len() > 1 {
        let enabled: Vec<u32> = hits.iter().copied().filter(|&i| !graph.nodes[i as usize].disabled).collect();
        if enabled.len() == 1 {
            hits = enabled;
        }
    }
    match hits.as_slice() {
        [one] => Ok(*one),
        [] => Err(format!("no model, seed, snapshot or source is named {name}")),
        many => {
            let mut ids: Vec<&str> = many.iter().map(|&i| graph.nodes[i as usize].id.as_str()).collect();
            ids.sort_unstable();
            Err(format!("{name} names {} nodes: write the unique_id of one, {}", ids.len(), ids.join(", ")))
        }
    }
}

/// What to say when a column is not found: the ones the cache does have
/// edges for, which are the ones worth asking about.
fn columns_with_lineage(graph: &Graph, node: u32) -> String {
    const SHOWN: usize = 40;
    let Some(cll) = graph.cll.as_ref() else { return "this cache has no edge at all".to_string() };
    let mut names: Vec<&str> = graph.nodes[node as usize]
        .columns
        .iter()
        .enumerate()
        .filter(|&(i, _)| cll.degree(ColRef { node, col: i as u32 }, true) != (0, 0))
        .map(|(_, c)| c.name.as_str())
        .collect();
    names.sort_unstable();
    match names.len() {
        0 => "none of its columns has an edge in this cache".to_string(),
        n if n > SHOWN => format!("the {n} with an edge include {}, ...", names[..SHOWN].join(", ")),
        _ => format!("those with an edge are {}", names.join(", ")),
    }
}

/// One model of the answer, a JSON line of its own: dbt's key names for the
/// node, and only the columns the walk reached.
#[derive(serde::Serialize)]
struct Reached<'a> {
    #[serde(skip)]
    label: String,
    unique_id: &'a str,
    name: &'a str,
    resource_type: &'a str,
    /// Whose `original_file_path` this is: a package's is under its own
    /// folder, not the project's.
    #[serde(skip_serializing_if = "str::is_empty")]
    package_name: &'a str,
    #[serde(skip_serializing_if = "str::is_empty")]
    original_file_path: &'a str,
    #[serde(skip_serializing_if = "str::is_empty")]
    materialized: &'a str,
    /// Its columns' depth nearest the focus: negative upstream.
    depth: i32,
    columns: BTreeMap<&'a str, Reach<'a>>,
}

#[derive(serde::Serialize)]
struct Reach<'a> {
    depth: i32,
    /// The columns of the answer this one is read from, `model.column`, each
    /// with the role the cache gives the edge. Every edge is listed once, at
    /// the column it reaches.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    from: BTreeMap<String, &'a str>,
    /// A root of collin's: what the SQL sets the column to, from no column,
    /// one text per branch of a union that sets it differently, and none when
    /// collin could not tell all of it (0096).
    #[serde(skip_serializing_if = "Option::is_none")]
    generated: Option<&'a [String]>,
    /// The data tests on this column, by name.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tests: Vec<&'a str>,
    /// Edges the answer leaves out on the side walked: the depth ran out, or
    /// the walk stopped here. A caller asks again with more levels for them.
    #[serde(skip_serializing_if = "is_zero")]
    more_up: usize,
    #[serde(skip_serializing_if = "is_zero")]
    more_down: usize,
}

fn is_zero(n: &usize) -> bool {
    *n == 0
}

/// The walk's columns gathered by model, upstream first, then by name.
fn reached<'a>(graph: &'a Graph, lineage: &Lineage<'a>, ask: &Ask) -> Vec<Reached<'a>> {
    // Every node of a column walk carries its column; without it there is
    // nothing to file it under, so it is left out rather than guessed.
    let at: Vec<Option<ColRef>> = lineage.nodes.iter().map(|n| n.at).collect();
    let owner: Vec<u32> = at.iter().flatten().map(|c| c.node).collect();
    // A model's name says less than its unique_id only when two in the
    // answer share it.
    let mut by_name: HashMap<&str, Vec<u32>> = HashMap::new();
    for &o in &owner {
        let ids = by_name.entry(graph.nodes[o as usize].name.as_str()).or_default();
        if !ids.contains(&o) {
            ids.push(o);
        }
    }
    let label = |o: u32| {
        let n = &graph.nodes[o as usize];
        if by_name.get(n.name.as_str()).map_or(0, Vec::len) > 1 {
            n.id.clone()
        } else {
            n.name.clone()
        }
    };

    let mut models: BTreeMap<u32, Reached<'a>> = BTreeMap::new();
    for (i, c) in lineage.nodes.iter().enumerate() {
        let Some(here) = at[i] else { continue };
        let o = here.node;
        let n = &graph.nodes[o as usize];
        let mut tests: Vec<&str> =
            n.columns[here.col as usize].tests.iter().map(|&t| graph.nodes[t as usize].name.as_str()).collect();
        tests.sort_unstable();
        let m = models.entry(o).or_insert_with(|| Reached {
            label: label(o),
            unique_id: &n.id,
            name: &n.name,
            resource_type: n.id.split('.').next().unwrap_or(""),
            package_name: &n.package,
            original_file_path: &n.file,
            materialized: &n.materialized,
            depth: c.depth,
            columns: BTreeMap::new(),
        });
        if c.depth.abs() < m.depth.abs() {
            m.depth = c.depth;
        }
        m.columns.insert(
            c.name,
            Reach {
                depth: c.depth,
                from: BTreeMap::new(),
                generated: c.generated,
                tests,
                more_up: if c.depth <= 0 && ask.up > 0 { c.hidden_up } else { 0 },
                more_down: if c.depth >= 0 && ask.down > 0 { c.hidden_down } else { 0 },
            },
        );
    }
    for (k, &[from, to]) in lineage.edges.iter().enumerate() {
        let (Some(f), Some(t)) = (at[from], at[to]) else { continue };
        let role = lineage.edge_kinds.get(k).copied().filter(|r| !r.is_empty()).unwrap_or("unknown");
        let parent = format!("{}.{}", label(f.node), lineage.nodes[from].name);
        if let Some(r) = models.get_mut(&t.node).and_then(|m| m.columns.get_mut(lineage.nodes[to].name)) {
            r.from.insert(parent, role);
        }
    }
    let mut out: Vec<Reached<'a>> = models.into_values().collect();
    // Upstream first, the focus, then downstream, so the lines read the way
    // the data flows; the unique_id settles a tie the same way every run.
    out.sort_by(|a, b| a.depth.cmp(&b.depth).then_with(|| a.name.cmp(b.name)).then_with(|| a.unique_id.cmp(b.unique_id)));
    out
}

/// Which cache answered and what it is worth: its tool, its target, its age
/// against the manifest's, and for collin whether its report found the
/// manifest, the catalog and the compiled SQL from more than one target.
fn cache_lines(root: &Path, graph: &Graph, cache: &Path, manifest: &Path, producer: &str, err: &mut impl Write) -> io::Result<()> {
    let m = &graph.meta;
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let from = match (producer, m.cll_source.as_str()) {
        ("", "") => "an unnamed producer".to_string(),
        ("", source) => source.to_string(),
        (producer, _) => producer.to_string(),
    };
    let target = if m.cll_target.is_empty() { "target not recorded".to_string() } else { format!("target {}", m.cll_target) };
    writeln!(
        err,
        "column lineage: {}, from {from}, {target}, written {} ({} ago), {} edges",
        cache.display(),
        iso_utc(m.cll_mtime),
        ls::age(now.saturating_sub(m.cll_mtime)),
        m.cll_edges,
    )?;
    // The surest sign a cache describes another manifest, when the times
    // say nothing: its edges name what this one does not hold.
    if m.cll_dropped > 0 {
        writeln!(
            err,
            "warning: {} edge ends name nothing in the manifest, a node it no longer has or an object dbt does not own: those edges are left out",
            m.cll_dropped
        )?;
    }
    let tool = collin::tool_of(&m.cll_source);
    if tool == "snowflake" {
        writeln!(err, "note: Snowflake's cache holds only the columns fetched so far from the page, so an answer from it can be partial")?;
    }
    if collin_run::stale(cache, manifest) {
        writeln!(err, "warning: the column lineage is older than the manifest: a column added, renamed or moved since may be missing or wrong")?;
        regenerate(root, graph, &files::folder_of(manifest).unwrap_or_else(|| root.join("target")), tool, err)?;
    }
    if tool == "collin" {
        if let Some(line) = alignment(cache) {
            writeln!(err, "{line}")?;
        }
    }
    Ok(())
}

/// collin's report beside the cache, when it was written by the same run:
/// the page's runner names it one way, a `collin generate` by hand another.
fn alignment(cache: &Path) -> Option<String> {
    let dir = cache.parent()?;
    let at = files::mtime_secs(cache);
    let report = [collin_run::REPORT, "column_lineage.report.json"]
        .iter()
        .map(|name| dir.join(name))
        .filter(|p| p.is_file() && files::mtime_secs(p).abs_diff(at) <= 120)
        .min_by_key(|p| files::mtime_secs(p).abs_diff(at))?;
    let read = collin_run::read_report(&report)?;
    let set_aside = read.summary.set_aside;
    let elsewhere = read.overview.findings.iter().find(|f| f.kind == "catalog_elsewhere").map_or(0, |f| f.entries.len());
    Some(if set_aside == 0 && elsewhere == 0 {
        format!("collin's report ({}): no compiled file and no catalog entry from another target", report.display())
    } else {
        format!(
            "warning: collin's report ({}) set aside {set_aside} compiled files compiled for another target and {elsewhere} catalog entries describing another table: \
             the manifest, the catalog and collin's run do not all come from one target",
            report.display()
        )
    })
}

/// The steps that would write a cache describing the manifest. The target is
/// the user's to name, never filled in here: the manifest, the catalog and
/// collin's run must come from one target under its own environment, and
/// nothing here can tell which one they mean. `folder` is where the manifest
/// was read, which dbt and collin are pointed at when it is not their default.
fn regenerate(root: &Path, graph: &Graph, folder: &Path, tool: &str, err: &mut impl Write) -> io::Result<()> {
    match tool {
        "snowflake" => {
            return writeln!(
                err,
                "  Snowflake's lineage is fetched from the page, a column at a time, under the user's own credentials: \
                 this mode never asks the warehouse. --column-tool collin reads collin's cache instead"
            )
        }
        "fusion" => {
            return writeln!(err, "  nothing writes Fusion's cache yet: --column-tool collin reads collin's cache instead")
        }
        _ => {}
    }
    writeln!(
        err,
        "  to write it with collin, ask the user which target it must describe. The manifest, the catalog and collin's run must \
         all come from that target, with its .env loaded. Then, in {}:",
        printable(&root.display().to_string())
    )?;
    // dbt and collin write under `<project>/target` unless told; a manifest
    // read elsewhere would otherwise stay as stale after the steps as before.
    let at = |p: &Path| p.canonicalize().ok();
    let elsewhere = at(folder).is_some_and(|f| Some(f) != at(&root.join("target")));
    let base = at(root).unwrap_or_else(|| root.to_path_buf());
    let dir = match at(folder) {
        Some(f) if elsewhere => printable(&f.strip_prefix(&base).map_or(f.clone(), Path::to_path_buf).display().to_string()),
        _ => "target".to_string(),
    };
    let under = |name: &str| quoted(&format!("{dir}/{name}"));
    let to_dbt = if elsewhere { format!(" --target-path {}", quoted(&dir)) } else { String::new() };
    // The names the page's own run writes (`collin_job`), whatever collin's
    // defaults: the page then loads the cache this run wrote, and reads its
    // report, rather than older ones of the names it keeps.
    let to_collin = format!(
        "{} --out {} --report {}",
        if elsewhere { format!(" --manifest {} --catalog {}", under("manifest.json"), under("catalog.json")) } else { String::new() },
        under("column_lineage.collin.json"),
        under(collin_run::REPORT),
    );
    let version = graph.meta.dbt_version.as_str();
    let fusion = version.starts_with("2.");
    if fusion || version.is_empty() {
        writeln!(err, "    dbt compile --target <TARGET> --write-catalog{to_dbt}        # dbt Fusion")?;
    }
    if !fusion {
        writeln!(err, "    dbt docs generate --target <TARGET>{to_dbt}                  # dbt-core, which compiles too")?;
    }
    writeln!(err, "    collin generate --project . --target <TARGET>{to_collin}")?;
    if let Some(names) = targets(root).filter(|n| !n.is_empty()) {
        let names: Vec<String> = names.iter().map(|n| printable(n)).collect();
        writeln!(err, "  targets in the profile: {}", names.join(", "))?;
    }
    // `DBT_TARGET` is the one value of a `.env` allowed out (0012), and only
    // when it reads as a name: anything else could be a terminal's escape.
    let envs: Vec<String> = crate::envs::discover(root)
        .into_iter()
        .filter(|f| f.auto_hidden.is_none())
        .map(|f| match f.target.as_str() {
            t if plain(t) => format!("{} (DBT_TARGET={t})", printable(&f.file)),
            _ => printable(&f.file),
        })
        .collect();
    if !envs.is_empty() {
        writeln!(err, "  .env files: {}", envs.join(", "))?;
    }
    let collin = collin_run::locate();
    match &collin.refused {
        Some((at, collin_run::Refusal::Tracked)) => {
            writeln!(err, "  {} is not run: git tracks it, so it came with a repository", printable(&at.display().to_string()))?
        }
        Some((at, collin_run::Refusal::Unsure(why))) => writeln!(
            err,
            "  {} is not run: it sits in a repository and git could not say whether it tracks it ({})",
            printable(&at.display().to_string()),
            printable(why)
        )?,
        None => {}
    }
    if collin.bin.is_none() {
        writeln!(err, "  collin is not installed: {}", collin_run::INSTALL)?;
    }
    Ok(())
}

/// A path as a shell would take it whole, quoted when it holds a space.
fn quoted(path: &str) -> String {
    if path.chars().any(char::is_whitespace) {
        format!("\"{path}\"")
    } else {
        path.to_string()
    }
}

/// A target's name as dbt would take it on a command line.
fn plain(value: &str) -> bool {
    !value.is_empty() && value.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
}

/// Text from a file name or a profile, with any control character shown as
/// `?`, so none reaches the terminal as an escape.
fn printable(text: &str) -> String {
    text.chars().map(|c| if c.is_control() { '?' } else { c }).collect()
}

/// The names of the targets of the project's profile, found where dbt looks
/// for it: names only, nothing else of a target. A project naming no profile
/// reads no profile.
fn targets(root: &Path) -> Option<Vec<String>> {
    let profile = crate::project::profile(root)?;
    let lookup = profiles::Lookup::from_env(|name| std::env::var_os(name), cfg!(windows));
    let text = crate::api::profile_text(root, &lookup.path(root)?).ok()?;
    Some(profiles::target_names(&text, &profile))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collin::RawColEdge;
    use crate::manifest::RawManifest;
    use serde_json::json;

    /// raw.orders -> stg_orders -> orders -> customer_ltv -> audit_orders,
    /// the last in another package, one edge with no role, and an exposure
    /// named `orders` too.
    fn graph() -> Graph {
        let model = |name: &str, package: &str, file: &str, materialized: &str| {
            json!({ "name": name, "resource_type": "model", "package_name": package,
                    "original_file_path": file, "config": { "materialized": materialized } })
        };
        // A test hangs off a declared column only, as in the Catalog.
        let mut orders = model("orders", "shop", "models/marts/orders.sql", "table");
        orders["columns"] = json!({ "order_amount": { "name": "order_amount" } });
        let raw: RawManifest = serde_json::from_value(json!({
            "metadata": { "dbt_version": "1.11.2", "adapter_type": "duckdb" },
            "nodes": {
                "model.shop.stg_orders": model("stg_orders", "shop", "models/staging/stg_orders.sql", "view"),
                "model.shop.orders": orders,
                "model.shop.customer_ltv": model("customer_ltv", "shop", "models/marts/customer_ltv.sql", "incremental"),
                "model.audit.audit_orders": model("audit_orders", "audit", "models/audit_orders.sql", "table"),
                "test.shop.not_null_orders_order_amount": {
                    "name": "not_null_orders_order_amount", "resource_type": "test", "package_name": "shop",
                    "original_file_path": "models/marts/schema.yml", "column_name": "order_amount",
                    "attached_node": "model.shop.orders",
                    "depends_on": { "nodes": ["model.shop.orders"] },
                    "test_metadata": { "name": "not_null", "kwargs": { "column_name": "order_amount" } }
                }
            },
            "sources": {
                "source.shop.raw.orders": { "name": "orders", "source_name": "raw", "resource_type": "source",
                    "package_name": "shop", "original_file_path": "models/staging/sources.yml" }
            },
            "exposures": {
                "exposure.shop.orders": { "name": "orders", "resource_type": "exposure", "package_name": "shop",
                    "original_file_path": "models/exposures.yml", "depends_on": { "nodes": ["model.shop.orders"] } }
            },
            "child_map": {}, "parent_map": {}
        }))
        .unwrap();
        let mut g = Graph::build(raw, Path::new("manifest.json"), 0, 0);
        let edge = |from: &str, from_col: &str, to: &str, to_col: &str, kind: &str| RawColEdge {
            from: from.into(),
            from_col: from_col.into(),
            to: to.into(),
            to_col: to_col.into(),
            kind: kind.into(),
        };
        g.merge_col_lineage(
            RawColLineage {
                version: 1,
                source: "collin".into(),
                edges: vec![
                    edge("source.shop.raw.orders", "amount", "model.shop.stg_orders", "amount", "passthrough"),
                    edge("model.shop.stg_orders", "amount", "model.shop.orders", "order_amount", "rename"),
                    edge("model.shop.orders", "order_amount", "model.shop.customer_ltv", "lifetime_value", "aggregate"),
                    edge("model.shop.orders", "order_amount", "model.shop.customer_ltv", "avg_basket", "aggregate"),
                    edge("model.shop.customer_ltv", "lifetime_value", "model.audit.audit_orders", "ltv", "window"),
                    edge("model.shop.customer_ltv", "avg_basket", "model.audit.audit_orders", "basket", ""),
                ],
                ..Default::default()
            },
            1,
        );
        g
    }

    fn ask(column: &str, up: u32, down: u32) -> Ask {
        Ask { column: column.into(), up, down, stop_at: Vec::new(), tool: None, cache: None, indirect: false }
    }

    fn run_on(g: &Graph, a: &Ask, output: Output) -> (String, String, i32) {
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = answer(g, a, output, &mut out, &mut err).unwrap();
        (String::from_utf8(out).unwrap(), String::from_utf8(err).unwrap(), code)
    }

    fn lines(out: &str) -> Vec<serde_json::Value> {
        out.lines().map(|l| serde_json::from_str(l).unwrap()).collect()
    }

    #[test]
    fn levels_take_a_number_or_all() {
        assert_eq!(levels("3"), Ok(3));
        assert_eq!(levels("all"), Ok(ALL));
        assert!(levels("-1").is_err() && levels("deep").is_err());
    }

    #[test]
    fn downstream_follows_renames_one_line_per_model() {
        let (out, _, code) = run_on(&graph(), &ask("stg_orders.amount", 0, ALL), Output::Json);
        assert_eq!(code, ANSWERED);
        let got = lines(&out);
        let ids: Vec<&str> = got.iter().map(|l| l["unique_id"].as_str().unwrap()).collect();
        assert_eq!(ids, ["model.shop.stg_orders", "model.shop.orders", "model.shop.customer_ltv", "model.audit.audit_orders"]);
        assert_eq!(got[1]["original_file_path"], "models/marts/orders.sql");
        assert_eq!(got[1]["columns"]["order_amount"]["from"], json!({ "stg_orders.amount": "rename" }));
        assert_eq!(got[1]["columns"]["order_amount"]["tests"], json!(["not_null_orders_order_amount"]));
        assert_eq!(got[2]["materialized"], "incremental");
        assert_eq!(got[2]["depth"], 2);
        assert_eq!(got[2]["columns"]["avg_basket"]["from"], json!({ "orders.order_amount": "aggregate" }));
        assert_eq!(got[3]["columns"]["basket"]["from"], json!({ "customer_ltv.avg_basket": "unknown" }));
        // A package's file is under its own folder, so the line says whose.
        assert_eq!((got[1]["package_name"].as_str(), got[3]["package_name"].as_str()), (Some("shop"), Some("audit")));
        // The focus is drawn without its own parents: upstream was not asked.
        assert_eq!(got[0]["columns"], json!({ "amount": { "depth": 0 } }));
    }

    /// A root of collin's says what the SQL sets it to, even with the filter
    /// edge `--indirect` sends into it, and a column without one says nothing
    /// new: a cache without roots answers as it always did.
    #[test]
    fn a_generated_column_says_its_value() {
        let mut g = graph();
        let edges = |extra: bool| {
            let edge = |from: &str, from_col: &str, to: &str, to_col: &str, kind: &str| RawColEdge {
                from: from.into(),
                from_col: from_col.into(),
                to: to.into(),
                to_col: to_col.into(),
                kind: kind.into(),
            };
            let mut list = vec![edge("model.shop.stg_orders", "loaded_at", "model.shop.orders", "loaded_at", "passthrough")];
            if extra {
                list.push(edge("source.shop.raw.orders", "status", "model.shop.stg_orders", "loaded_at", "filter"));
            }
            list
        };
        let before = run_on(&graph(), &ask("orders.order_amount", ALL, ALL), Output::Json).0;
        let root = |exprs: &[&str]| crate::collin::RawRoot {
            node: "model.shop.stg_orders".into(),
            column: "loaded_at".into(),
            expressions: exprs.iter().map(|e| e.to_string()).collect(),
        };
        let cache = |extra: bool, roots: Vec<crate::collin::RawRoot>| RawColLineage {
            version: 1,
            source: "collin".into(),
            edges: edges(extra),
            roots,
            ..Default::default()
        };
        g.merge_col_lineage(cache(true, vec![root(&["current_timestamp()"])]), 2);
        // By default the filter edge is left out, as on the canvas (0097).
        let (hidden, _, _) = run_on(&g, &ask("orders.loaded_at", ALL, 0), Output::Json);
        assert!(!hidden.contains("filter") && !hidden.contains("source.shop.raw.orders"), "{hidden}");
        let (out, _, _) = run_on(&g, &Ask { indirect: true, ..ask("orders.loaded_at", ALL, 0) }, Output::Json);
        let got = lines(&out);
        let stg = got.iter().find(|l| l["name"] == "stg_orders").unwrap();
        assert_eq!(stg["columns"]["loaded_at"]["generated"], json!(["current_timestamp()"]), "{out}");
        assert_eq!(stg["columns"]["loaded_at"]["from"].as_object().map(|m| m.len()), Some(1), "the filter edge stays: {out}");
        assert!(got.iter().find(|l| l["name"] == "orders").unwrap()["columns"]["loaded_at"].get("generated").is_none());

        // collin could not tell all of it: generated still, with no value.
        let mut untold = graph();
        untold.merge_col_lineage(cache(false, vec![root(&[])]), 2);
        let (out, _, _) = run_on(&untold, &ask("stg_orders.loaded_at", 0, 0), Output::Json);
        assert_eq!(lines(&out)[0]["columns"]["loaded_at"], json!({ "depth": 0, "generated": [] }));

        assert!(!before.contains("generated"), "{before}");
    }

    #[test]
    fn a_short_walk_counts_what_it_leaves_out() {
        let (out, _, _) = run_on(&graph(), &ask("stg_orders.amount", 1, 1), Output::Json);
        let got = lines(&out);
        assert_eq!(got[0]["unique_id"], "source.shop.raw.orders");
        assert_eq!(got[0]["depth"], -1);
        let orders = got.iter().find(|l| l["name"] == "orders").unwrap();
        assert_eq!(orders["columns"]["order_amount"]["more_down"], 2);
        // Edges into the source's column from nowhere: nothing more up.
        assert!(got[0]["columns"]["amount"].get("more_up").is_none());
    }

    #[test]
    fn names_print_model_dot_column_in_the_order_of_the_flow() {
        let (out, _, _) = run_on(&graph(), &ask("orders.order_amount", ALL, ALL), Output::Name);
        assert_eq!(
            out.lines().collect::<Vec<_>>(),
            [
                "raw.orders.amount",
                "stg_orders.amount",
                "orders.order_amount",
                "customer_ltv.avg_basket",
                "customer_ltv.lifetime_value",
                "audit_orders.basket",
                "audit_orders.ltv",
            ]
        );
    }

    #[test]
    fn a_role_stops_the_walk_as_on_the_canvas() {
        let mut a = ask("stg_orders.amount", 0, ALL);
        a.stop_at = vec!["aggregate".into()];
        let (out, err, _) = run_on(&graph(), &a, Output::Json);
        let got = lines(&out);
        assert!(!out.contains("audit_orders"), "{out}");
        let ltv = got.iter().find(|l| l["name"] == "customer_ltv").unwrap();
        assert_eq!(ltv["columns"]["lifetime_value"]["more_down"], 1);
        assert!(err.contains("stopped downstream at aggregate"), "{err}");
    }

    #[test]
    fn unknown_stops_at_an_edge_the_cache_gave_no_role() {
        let mut a = ask("stg_orders.amount", 0, ALL);
        a.stop_at = vec!["unknown".into()];
        let (out, err, _) = run_on(&graph(), &a, Output::Name);
        assert!(!err.contains("no edge of this cache has the role"), "{err}");
        assert!(err.contains("stopped downstream at unknown:"), "{err}");
        assert!(out.contains("audit_orders.basket\n"), "{out}");
    }

    #[test]
    fn a_role_the_cache_lacks_is_said() {
        let mut a = ask("stg_orders.amount", 0, 1);
        a.stop_at = vec!["hash".into()];
        let (_, err, code) = run_on(&graph(), &a, Output::Name);
        assert_eq!(code, ANSWERED);
        assert!(err.contains("no edge of this cache has the role hash"), "{err}");
    }

    #[test]
    fn a_column_is_named_by_model_source_or_unique_id() {
        let g = graph();
        let at = |spec: &str| focus(&g, spec).map(|c| g.nodes[c.node as usize].id.clone());
        assert_eq!(at("orders.ORDER_AMOUNT").unwrap(), "model.shop.orders");
        assert_eq!(at("raw.orders.amount").unwrap(), "source.shop.raw.orders");
        assert_eq!(at("model.shop.orders.order_amount").unwrap(), "model.shop.orders");
        assert_eq!(at("Customer_LTV.avg_basket").unwrap(), "model.shop.customer_ltv");
        // The exposure named `orders` has no columns, so it is no rival.
        assert_eq!(at("orders.order_amount").unwrap(), "model.shop.orders");
    }

    #[test]
    fn what_cannot_be_found_is_refused_with_what_can() {
        let g = graph();
        let (_, err, code) = run_on(&g, &ask("orders.amount", 0, 1), Output::Name);
        assert_eq!(code, REFUSED);
        assert!(err.contains("orders has no column amount: those with an edge are order_amount"), "{err}");
        assert!(focus(&g, "nothing.x").unwrap_err().contains("no model, seed, snapshot or source is named nothing"));
        assert!(focus(&g, "amount").unwrap_err().contains("write the column as model.column"));
    }

    #[test]
    fn two_models_of_one_name_ask_for_the_unique_id() {
        let mut g = graph();
        let twin = g.nodes.iter().position(|n| n.id == "model.audit.audit_orders").unwrap();
        g.nodes[twin].name = "orders".into();
        let why = focus(&g, "orders.order_amount").unwrap_err();
        assert!(why.contains("write the unique_id of one, model.audit.audit_orders, model.shop.orders"), "{why}");
        // Both in one answer, each is named by its unique_id.
        let (out, _, _) = run_on(&g, &ask("model.shop.orders.order_amount", 0, ALL), Output::Name);
        assert!(out.contains("model.shop.orders.order_amount\n") && out.contains("model.audit.audit_orders.ltv\n"), "{out}");
    }

    #[test]
    fn no_walk_asked_says_how_to_ask() {
        let (out, err, _) = run_on(&graph(), &ask("orders.order_amount", 0, 0), Output::Name);
        assert_eq!(out, "orders.order_amount\n");
        assert!(err.contains("--down N or all"), "{err}");
    }

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dbt-edith-ls-column-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_tool_without_a_cache_is_an_error_never_a_file_to_write() {
        let dir = scratch("tool");
        let g = graph();
        std::fs::write(dir.join("column_lineage.collin.json"), r#"{"version":1,"source":"collin","edges":[]}"#).unwrap();
        let found = collin::discover(&dir);
        let mut a = ask("orders.order_amount", 0, 1);
        assert_eq!(cache_for(&dir, &g, &a, &found).unwrap(), dir.join("column_lineage.collin.json"));
        a.tool = Some(Tool::Snowflake);
        assert!(cache_for(&dir, &g, &a, &found).unwrap_err().contains("no column lineage from snowflake"));
        a.tool = Some(Tool::Collin);
        assert_eq!(cache_for(&dir, &g, &a, &found).unwrap(), dir.join("column_lineage.collin.json"));
        // Snowflake's cache, newer and offered on a Snowflake project, holds
        // only the columns fetched: collin's answers.
        let mut snow = g.clone();
        snow.meta.adapter = "snowflake".into();
        std::fs::write(dir.join("column_lineage.snowflake.json"), r#"{"version":1,"source":"snowflake","edges":[]}"#).unwrap();
        let later = std::time::SystemTime::now() + std::time::Duration::from_secs(60);
        std::fs::File::options().write(true).open(dir.join("column_lineage.snowflake.json")).unwrap().set_modified(later).unwrap();
        let found = collin::discover(&dir);
        assert_eq!(found[0].tool, "snowflake");
        assert_eq!(cache_for(&dir, &snow, &ask("orders.order_amount", 0, 1), &found).unwrap(), dir.join("column_lineage.collin.json"));
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(cache_for(&dir, &g, &ask("orders.order_amount", 0, 1), &[]).unwrap_err().contains("no column lineage cache"));
    }

    #[test]
    fn a_file_named_outright_must_be_a_cache() {
        let dir = scratch("load");
        std::fs::write(dir.join("column_lineage.report.json"), r#"{"totals":{"models":2},"models":[]}"#).unwrap();
        let why = load(&dir.join("column_lineage.report.json")).unwrap_err();
        assert!(why.contains("is not a column lineage cache"), "{why}");
        // A catalog fails on its own `errors: null` before the check.
        std::fs::write(dir.join("catalog.json"), r#"{"metadata":{},"nodes":{},"sources":{},"errors":null}"#).unwrap();
        let why = load(&dir.join("catalog.json")).unwrap_err();
        assert!(why.contains("is not a column lineage cache"), "{why}");
        std::fs::write(dir.join("broken.json"), "{not json").unwrap();
        assert!(load(&dir.join("broken.json")).unwrap_err().starts_with("cannot read"));
        std::fs::write(dir.join("c.json"), r#"{"version":1,"source":"collin","producer":"collin 0.7.0","edges":[]}"#).unwrap();
        assert_eq!(load(&dir.join("c.json")).unwrap().producer, "collin 0.7.0");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_cache_line_says_what_the_manifest_does_not_hold() {
        let dir = scratch("lines");
        let (cache, manifest) = (dir.join("column_lineage.collin.json"), dir.join("manifest.json"));
        std::fs::write(&manifest, "{}").unwrap();
        std::fs::write(&cache, r#"{"version":1,"source":"collin","edges":[]}"#).unwrap();
        let mut g = graph();
        g.merge_col_lineage(
            RawColLineage {
                version: 1,
                source: "snowflake".into(),
                edges: vec![RawColEdge {
                    from: "model.shop.gone".into(),
                    from_col: "x".into(),
                    to: "model.shop.orders".into(),
                    to_col: "order_amount".into(),
                    kind: String::new(),
                }],
                ..Default::default()
            },
            1,
        );
        let mut err = Vec::new();
        cache_lines(&dir, &g, &cache, &manifest, "", &mut err).unwrap();
        let said = String::from_utf8(err).unwrap();
        assert!(said.contains("1 edge ends name nothing in the manifest"), "{said}");
        assert!(said.contains("holds only the columns fetched so far"), "{said}");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_steps_leave_the_target_to_the_user() {
        let dir = scratch("steps");
        std::fs::write(dir.join(".env.preprod"), "DBT_TARGET=preprod\nSECRET=hunter2\n").unwrap();
        // Not a name, so not shown: an escape would reach the terminal.
        std::fs::write(dir.join(".env.odd"), "DBT_TARGET=x\u{1b}]0;title\u{7}\n").unwrap();
        std::fs::create_dir_all(dir.join("target")).unwrap();
        let mut g = graph();
        let mut err = Vec::new();
        regenerate(&dir, &g, &dir.join("target"), "collin", &mut err).unwrap();
        let said = String::from_utf8(err).unwrap();
        assert!(said.contains("ask the user which target"), "{said}");
        assert!(said.contains(".env.odd,") && !said.contains('\u{1b}') && !said.contains("title"), "{said}");
        assert!(!said.contains("--target-path"), "{said}");
        assert!(said.contains("dbt docs generate --target <TARGET>") && !said.contains("--write-catalog"), "{said}");
        assert!(
            said.contains("collin generate --project . --target <TARGET> --out target/column_lineage.collin.json --report target/collin.report.json"),
            "{said}"
        );
        assert!(said.contains(".env.preprod (DBT_TARGET=preprod)") && !said.contains("hunter2"), "{said}");
        // A manifest dbt Fusion wrote gets Fusion's command, whose
        // `docs generate` writes no catalog unless told.
        g.meta.dbt_version = "2.0.8".into();
        let mut err = Vec::new();
        regenerate(&dir, &g, &dir.join("target"), "collin", &mut err).unwrap();
        let said = String::from_utf8(err).unwrap();
        assert!(said.contains("dbt compile --target <TARGET> --write-catalog") && !said.contains("docs generate"), "{said}");
        let mut err = Vec::new();
        regenerate(&dir, &g, &dir.join("target"), "snowflake", &mut err).unwrap();
        assert!(String::from_utf8(err).unwrap().contains("never asks the warehouse"));
        // A manifest read from another folder: dbt and collin are sent there,
        // or the steps would leave its cache as stale as it was.
        std::fs::create_dir_all(dir.join("target-qa")).unwrap();
        let mut err = Vec::new();
        regenerate(&dir, &g, &dir.join("target-qa"), "collin", &mut err).unwrap();
        let said = String::from_utf8(err).unwrap();
        assert!(said.contains("--write-catalog --target-path target-qa"), "{said}");
        assert!(
            said.contains("--manifest target-qa/manifest.json --catalog target-qa/catalog.json --out target-qa/column_lineage.collin.json --report target-qa/collin.report.json"),
            "{said}"
        );
        std::fs::create_dir_all(dir.join("target qa")).unwrap();
        let mut err = Vec::new();
        regenerate(&dir, &g, &dir.join("target qa"), "collin", &mut err).unwrap();
        let said = String::from_utf8(err).unwrap();
        assert!(said.contains("--target-path \"target qa\"") && said.contains("--out \"target qa/column_lineage.collin.json\""), "{said}");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn collins_report_from_the_same_run_says_whether_the_targets_agree() {
        let dir = scratch("report");
        let cache = dir.join("column_lineage.collin.json");
        std::fs::write(&cache, r#"{"version":1,"source":"collin","edges":[]}"#).unwrap();
        assert_eq!(alignment(&cache), None);
        std::fs::write(
            dir.join("column_lineage.report.json"),
            r#"{"totals":{"models":2,"parsed":2,"sql_from_files":1,"sql_from_files_set_aside":1},
                "catalog_elsewhere":[{"unique_id":"model.p.a","relation":"DB.S.A","catalog_relation":"DB.DEV.A"}],
                "models":[]}"#,
        )
        .unwrap();
        let said = alignment(&cache).unwrap();
        assert!(said.contains("set aside 1 compiled files") && said.contains("1 catalog entries"), "{said}");
        std::fs::write(dir.join("column_lineage.report.json"), r#"{"totals":{"models":2,"parsed":2},"models":[]}"#).unwrap();
        assert!(alignment(&cache).unwrap().contains("no compiled file and no catalog entry from another target"));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
