//! `--site`: a static documentation site, written from what dbt wrote (0177).
//!
//! One concern: turning a manifest (and a catalog, when there is one) into a
//! folder that opens from `file://` and from any static host, with no request
//! of its own. Three invariants hold here:
//!   - what goes in is the manifest, the catalog and git's answer about HEAD
//!     and the origin, through the gate (0121). No `.env` is read, no
//!     `env_var()` resolved, no profile opened, nothing sent to a warehouse.
//!   - the same inputs give the same bytes: nothing written holds an absolute
//!     path, a timestamp of this run or a duration.
//!   - Edith writes only into a folder it made (0178): strictly inside the
//!     project, and either empty or carrying its marker.
//!
//! The browser answers the page's questions itself, with the same walks the
//! server makes (`web/site.js`), from the data this file lays out: one index,
//! and the cards and the SQL in buckets loaded when a node is first opened. The
//! SQL is the manifest's own `raw_code` and `compiled_code`, read in a second
//! pass over the same file, and never a file of `target/` (0180).
//!
//! The one other file it reads is collin's column lineage cache beside the manifest, and its report
//! for the cards' notes, never started by `--site` alone: only `--run-collin` runs collin, first
//! (0182). The cache is laid out whole in `data/cll.js`, loaded when a column is first asked about.

use crate::api::{self, Assets};
use crate::collin_run::{self, ModelNote};
use crate::graph::{ColRef, Graph, Kind, MAX_LEVELS};
use crate::{collin, files, git, jobs, ls, ls_column};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::AtomicBool;

/// What `--site` was given besides the project, the manifest and the catalog.
pub struct Ask {
    /// None for the default, `<project>/target/edith-site`.
    pub dir: Option<PathBuf>,
    pub environment: Option<String>,
    pub commit: Option<String>,
    pub repo_url: Option<String>,
    /// `--run-collin`: collin writes the column lineage first, over the manifest as it stands (0182).
    pub produce: Option<ls_column::Produce>,
    /// `--jobs-commit`: the commit the job files are linked at, for every jobs source, in place of the one git says (0183).
    pub jobs_commit: Option<String>,
    /// `--jobs-repo-url`: the address the job files' repository is linked by, for every jobs source.
    pub jobs_repo_url: Option<String>,
}

/// Written first into a folder, and what a later run recognises it by.
const MARKER: &str = ".dbt-edith-site";

/// A folder is emptied only if it carries this file, or is empty already.
const MARKER_TEXT: &str = "Written by dbt-edith --site. This folder is emptied and rewritten by the next run.\n";

/// A bucket of cards closes at this many nodes or this many bytes, whichever
/// comes first: the page loads one when a card is first opened.
const BUCKET_NODES: usize = 200;
const BUCKET_BYTES: usize = 512 * 1024;

/// The page may load its own files and nothing else: no network, no frame, no
/// form. `connect-src 'none'` is what makes "no request" a promise the browser
/// keeps rather than a promise of the code.
const CSP: &str = "default-src 'none'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'none'; \
                   base-uri 'none'; form-action 'none'";

/// The mode, from the command line's arguments to the exit code.
pub fn run(project: &Path, manifest: Option<&Path>, catalog: Option<&Path>, ask: &Ask) -> i32 {
    let mut out = std::io::stdout().lock();
    let mut err = std::io::stderr().lock();
    generate(project, manifest, catalog, ask, &mut out, &mut err)
}

/// `run`, over buffers: the exit code, with the answer on `out` and what the
/// run could not do on `err`. 0 when the folder is written; 2 when nothing
/// could be (the same code `ls` gives for no manifest and a wrong flag).
pub(crate) fn generate(
    project: &Path, manifest: Option<&Path>, catalog: Option<&Path>, ask: &Ask, out: &mut impl Write, err: &mut impl Write,
) -> i32 {
    generate_with(project, manifest, catalog, ask, collin_run::locate, out, err)
}

/// `generate` with the collin `--run-collin` would run named by `locate`, asked only when a run is.
pub(crate) fn generate_with(
    project: &Path, manifest: Option<&Path>, catalog: Option<&Path>, ask: &Ask, locate: impl FnOnce() -> collin_run::Located,
    out: &mut impl Write, err: &mut impl Write,
) -> i32 {
    generate_at(project, manifest, catalog, ask, locate, jobs::home_dir(), out, err)
}

/// `generate_with` with the home folder named by the caller: no repository at it or above it is read (0183).
#[allow(clippy::too_many_arguments)] // the run's inputs, each named once; a struct would only rename them
pub(crate) fn generate_at(
    project: &Path, manifest: Option<&Path>, catalog: Option<&Path>, ask: &Ask, locate: impl FnOnce() -> collin_run::Located,
    home: Option<PathBuf>, out: &mut impl Write, err: &mut impl Write,
) -> i32 {
    // What can be refused without reading anything is refused first.
    let flags = (|| -> Result<_, String> {
        let commit = ask.commit.as_deref().map(valid_commit).transpose()?;
        let repo = ask.repo_url.as_deref().map(repo_from_flag).transpose()?;
        let environment = ask.environment.as_deref().map(valid_environment).transpose()?;
        let jobs_commit =
            ask.jobs_commit.as_deref().map(valid_commit).transpose().map_err(|why| why.replacen("--commit", "--jobs-commit", 1))?;
        let jobs_repo =
            ask.jobs_repo_url.as_deref().map(repo_from_flag).transpose().map_err(|why| why.replacen("--repo-url", "--jobs-repo-url", 1))?;
        Ok((commit, repo, environment, jobs_commit, jobs_repo))
    })();
    let (commit_flag, repo_flag, environment, jobs_commit_flag, jobs_repo_flag) = match flags {
        Ok(f) => f,
        Err(why) => {
            let _ = writeln!(err, "error: {why}");
            return ls::CANNOT_RUN;
        }
    };
    let (root, manifest_path, mut graph) = match ls::open_with_catalog(project, manifest, catalog) {
        Ok(opened) => opened,
        Err(why) => {
            let _ = writeln!(err, "{why}");
            return ls::CANNOT_RUN;
        }
    };
    let dir = match site_dir(&root, ask.dir.as_deref()) {
        Ok(dir) => dir,
        Err(why) => {
            let _ = writeln!(err, "error: {why}");
            return ls::CANNOT_RUN;
        }
    };

    // collin runs only once the folder is known to be ours to write, and writes beside the manifest as
    // its own run does: a run that makes no cache leaves the site unwritten, the way `--column` ends.
    let target_dir = files::folder_of(&manifest_path).unwrap_or_else(|| root.join("target"));
    if let Some(p) = &ask.produce {
        match ls_column::produce(&root, &manifest_path, &target_dir, p, locate(), &AtomicBool::new(false), err) {
            Ok(Some(_)) => {}
            _ => return ls::CANNOT_RUN,
        }
    }
    let Ok(cll_stale) = merge_collin(&mut graph, &target_dir, &manifest_path, ask.produce.is_some(), err) else {
        return ls::CANNOT_RUN;
    };
    let notes = notes_of(&graph, &target_dir, &root);

    // Whether it was merged, not whether a file is there: one that did not read is left out of the cards.
    let has_catalog = graph.meta.catalog_mtime > 0 || graph.meta.catalog_columns > 0;
    if !ls::catalog_path(&root, &manifest_path, catalog).is_file() {
        let _ = writeln!(err, "no catalog.json: columns keep the types the YAML declares");
    }
    // A repository at the home folder or above it is no project's own: a dotfiles repository would give its commit and its
    // origin as the project's, and a file link below a prefix that is the machine's own layout (0183).
    let project_top = git::top_level(&root).and_then(|t| t.canonicalize().ok()).map(files::plain);
    let home_repo = project_top.as_deref().is_some_and(|t| at_or_above_home(t, home.as_deref()));
    if home_repo {
        let _ = writeln!(err, "git is not read for the project: {HOME_REPOSITORY}; pass --commit and --repo-url for the links");
    }
    let commit = match commit_flag {
        Some(c) => Some(c),
        None if home_repo => None,
        None => match git::head_commit(&root) {
            Ok(c) => Some(c),
            Err(why) => {
                let _ = writeln!(err, "no commit: {why}");
                None
            }
        },
    };
    let repo = match repo_flag {
        Some(r) => Some(r),
        None if home_repo => None,
        None => match git::origin_url(&root) {
            // Never printed: an origin can carry a token, and what is derived
            // from it is all the site and this line need.
            Some(origin) => {
                let derived = repo_from_origin(&origin);
                if derived.is_none() {
                    let _ = writeln!(err, "no link: the origin is not on github.com, gitlab.com or bitbucket.org, pass --repo-url");
                }
                derived
            }
            None => {
                let _ = writeln!(err, "no link: no origin remote, pass --repo-url");
                None
            }
        },
    };
    let commit_url = match (&repo, &commit) {
        (Some(r), Some(c)) => commit_link(r, c),
        _ => String::new(),
    };
    let file_base = match (&repo, &commit) {
        (Some(_), Some(_)) if home_repo => None,
        (Some(r), Some(c)) => file_links(r, c, &root, err),
        _ => None,
    };
    let code = match read_code(&manifest_path).map(|c| sql_of(&graph, c)) {
        Ok(code) => code,
        Err(why) => {
            let _ = writeln!(err, "error: {why}");
            return ls::CANNOT_RUN;
        }
    };
    if !code.iter().flatten().any(|s| s.compiled.is_some()) {
        let _ = writeln!(err, "no compiled SQL in the manifest: run --site after dbt build, compile or docs generate");
    }

    let jobs = jobs_of(&graph, &root, project_top.as_deref(), home.as_deref(), jobs_commit_flag.as_deref(), jobs_repo_flag.as_ref(), err);

    let meta = json!({
        "project": graph.meta.project,
        // The root project's package, which a node's own tells its files from a package's: the manifest's
        // name, or `dbt_project.yml`'s when the manifest has none, as the tree reads it.
        "root": graph.macros.root(),
        "environment": environment.clone().unwrap_or_default(),
        "commit": commit.clone().unwrap_or_default(),
        "commit_url": commit_url,
        "repo_url": repo.as_ref().map(|r| r.url.clone()).unwrap_or_default(),
        "file_base": file_base,
        "generated_at": graph.meta.generated_at,
        "dbt_version": graph.meta.dbt_version,
        "adapter": graph.meta.adapter,
        "counts": graph.meta.counts,
        "catalog": has_catalog,
        "edith": env!("CARGO_PKG_VERSION"),
    });
    let mut meta = meta;
    cll_meta(&graph, cll_stale, &mut meta);
    if let Some(j) = &jobs {
        meta["jobs"] = j.meta.clone();
    }
    let title = title_of(&graph.meta.project, environment.as_deref().unwrap_or(""));
    let mut built = match build(&graph, meta, &title, code, notes.as_ref()) {
        Ok(b) => b,
        Err(why) => {
            let _ = writeln!(err, "error: {why}");
            return ls::CANNOT_RUN;
        }
    };
    let said_jobs = jobs
        .as_ref()
        .map(|j| (j.meta["jobs"].as_u64().unwrap_or(0), j.meta["steps"].as_u64().unwrap_or(0), j.meta["present"].as_u64().unwrap_or(0)));
    built.jobs = jobs.map(|j| j.js);
    // The folder was judged before the data was built, which took a while: it must still be the same place.
    if site_dir(&root, ask.dir.as_deref()).as_ref() != Ok(&dir) {
        let _ = writeln!(err, "error: {} changed while the site was built: nothing was written", dir.display());
        return ls::CANNOT_RUN;
    }
    if let Err(e) = write_site(&dir, &built) {
        let _ = writeln!(err, "error: cannot write {}: {e}", dir.display());
        return ls::CANNOT_RUN;
    }
    let c = &graph.meta.counts;
    let _ = writeln!(out, "{}", dir.display());
    let _ = writeln!(
        out,
        "  {} nodes ({} models, {} sources, {} tests) in {} cards file(s)",
        graph.nodes.len(),
        c.get("model").unwrap_or(&0),
        c.get("source").unwrap_or(&0),
        c.get("test").unwrap_or(&0),
        built.buckets.len()
    );
    if cll_stale.is_some() {
        let _ = writeln!(out, "  column lineage: {} edges from {}", graph.meta.cll_edges, cll_tool(&graph));
    }
    if let Some((jobs, steps, sources)) = said_jobs {
        let _ = writeln!(out, "  jobs: {jobs} jobs and {steps} steps from {sources} source(s)");
    }
    ls::ANSWERED
}

// ----------------------------------------------------------- column lineage ----

/// Merges collin's cache beside the manifest, `column_lineage.collin.json`, and says on `err` what it
/// found. Only that file, and only when the same listing the page's menu is made from lists it: the
/// report, a file of a newer version and one cut short are not a cache. Ok(Some(stale)) when it was
/// merged, `stale` being whether the manifest is newer than it (by mtime, which a checkout resets: the
/// warning is a guess that errs towards saying so); Ok(None) when there is none to merge; Err when
/// the file the run itself just wrote cannot be read, which ends the run.
fn merge_collin(graph: &mut Graph, target: &Path, manifest: &Path, produced: bool, err: &mut impl Write) -> Result<Option<bool>, ()> {
    let path = collin::path_for(target, "collin");
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    if !path.is_file() {
        let _ = writeln!(err, "no column lineage: collin has written no cache beside the manifest, and --run-collin writes one");
        return Ok(None);
    }
    let listed = collin::discover(target).iter().any(|a| a.file == name);
    let why = match listed.then(|| api::try_merge_cache(graph, &path)).flatten() {
        Some(Ok(_)) => None,
        Some(Err(e)) => Some(e),
        None => Some(format!("{name} is not a column lineage cache this build can read")),
    };
    if let Some(why) = why {
        if produced {
            let _ = writeln!(err, "error: collin wrote a cache that cannot be read: {why}");
            return Err(());
        }
        let _ = writeln!(err, "warning: no column lineage, the cache was ignored: {why}");
        return Ok(None);
    }
    let stale = collin_run::stale(&path, manifest);
    if stale {
        let _ = writeln!(
            err,
            "warning: collin's column lineage is older than the manifest: it may not match this commit; --run-collin writes it again"
        );
    }
    Ok(Some(stale))
}

/// What the header and the page's Columns tab say of the column lineage, written into the index's
/// meta. The file is named bare, never by the path it was read from, and nothing here is the time the
/// cache was read or what wrote it, which the page has no use for: `cll_generated_at` is the cache's
/// own, from the file's contents.
fn cll_meta(graph: &Graph, stale: Option<bool>, meta: &mut Value) {
    let Some(stale) = stale.filter(|_| graph.cll.is_some()) else {
        meta["cll"] = json!(false);
        return;
    };
    let m = &graph.meta;
    meta["cll"] = json!(true);
    meta["cll_tool"] = json!(cll_tool(graph));
    meta["cll_file"] = json!(CLL_FILE);
    meta["cll_source"] = json!(m.cll_source);
    meta["cll_target"] = json!(m.cll_target);
    meta["cll_generated_at"] = json!(m.cll_generated_at);
    meta["cll_edges"] = json!(m.cll_edges);
    meta["cll_dropped"] = json!(m.cll_dropped);
    meta["cll_kinds"] = json!(m.cll_kinds);
    meta["cll_stale"] = json!(stale);
}

/// The tool a cache's own `source` names, as the server's menu does, and collin's when it names none: the
/// file is the one collin writes.
fn cll_tool(graph: &Graph) -> &'static str {
    match collin::tool_of(&graph.meta.cll_source) {
        "" => "collin",
        tool => tool,
    }
}

/// The file `cll_meta` names, which is also where the cache is read from.
const CLL_FILE: &str = "column_lineage.collin.json";

/// collin's notes on its models, for the cards, when collin is the tool whose cache was merged: the
/// server's own rule, from the report beside it. A note may name where collin found a model's SQL, and
/// a site is published, so the project's root is cut from every text in it, and a file that is not
/// below the project is dropped.
fn notes_of(graph: &Graph, target: &Path, root: &Path) -> Option<HashMap<String, ModelNote>> {
    if graph.cll.is_none() || collin::tool_of(&graph.meta.cll_source) != "collin" {
        return None;
    }
    let report = collin_run::read_report(&target.join(collin_run::REPORT))?;
    Some(report.notes.into_iter().map(|(id, note)| (id, scrubbed(note, root))).collect())
}

/// Whether a word is a path from the root of a machine: `/x`, `\\x` (a Windows root, `\\?\C:\x` and a share
/// included) or `C:\x` and `C:/x`. A lone separator is a symbol of the text, not a path.
fn looks_absolute(word: &str) -> bool {
    let b = word.as_bytes();
    let drive = b.len() > 2 && b[0].is_ascii_alphabetic() && b[1] == b':' && (b[2] == b'/' || b[2] == b'\\');
    (b.len() > 1 && (b[0] == b'/' || b[0] == b'\\')) || drive
}

/// Whether `text` still holds an absolute path after the root was cut. Words end at white space, a quote, a
/// bracket, a comma or a semicolon; a path with a space in it shows by its first word, which is why the text
/// is judged whole and never rewritten word by word, which would keep the tail.
fn holds_path(text: &str) -> bool {
    text.split(|c: char| c.is_whitespace() || "'\"`()[]<>,;".contains(c)).any(looks_absolute)
}

/// `note` with the project's root cut from each text, and any text that still holds an absolute path, a file that
/// is not below the project included, dropped whole (a name is removed from its list): a path outside the project
/// names where the machine keeps it, and a published card has no use for that. `sql_file` is made relative to the
/// project, or dropped when it is not below it.
fn scrubbed(mut note: ModelNote, root: &Path) -> ModelNote {
    let base = root.display().to_string();
    let below = |text: &str| -> String {
        let mut out = text.to_string();
        for sep in ["/", "\\"] {
            out = out.replace(&format!("{base}{sep}"), "");
        }
        out
    };
    let cut = |text: &str| {
        let out = below(text).replace(&base, "");
        if holds_path(&out) {
            String::new()
        } else {
            out
        }
    };
    note.sql_file = below(&note.sql_file);
    if looks_absolute(&note.sql_file) || note.sql_file.split(['/', '\\']).any(|part| part == "..") {
        note.sql_file.clear();
    }
    note.parse_error = cut(&note.parse_error);
    note.provenance = cut(&note.provenance);
    for c in &mut note.columns {
        c.detail = cut(&c.detail);
        c.names = c.names.iter().map(|n| cut(n)).filter(|n| !n.is_empty()).collect();
    }
    note
}

/// The column lineage as the script `data/cll.js` calls, or None without any: the cache as it was
/// merged, laid out for the walks of `web/site.js` (0182). A column is a number, where its node starts
/// plus its slot, so the order of a `ColRef` is the number's and a walk compares integers. What a
/// walk decides is not here, only what it reads: the edges in the order the walks read them, and what
/// the cards of columns say, by column. A name and a type are interned: the same dozen types and
/// the same few hundred names are every model's.
pub(crate) fn cll_of(graph: &Graph) -> Option<Value> {
    let cll = graph.cll.as_ref()?;
    let mut at: Vec<usize> = Vec::with_capacity(graph.nodes.len() + 1);
    let mut total = 0;
    for n in &graph.nodes {
        at.push(total);
        total += n.columns.len();
    }
    at.push(total);
    let num = |c: ColRef| at[c.node as usize] + c.col as usize;

    let mut names: Vec<&str> = Vec::new();
    let mut types: Vec<&str> = Vec::new();
    let (mut name_at, mut type_at): (HashMap<&str, usize>, HashMap<&str, usize>) = (HashMap::new(), HashMap::new());
    let (mut nm, mut ty, mut tests): (Vec<usize>, Vec<usize>, Vec<usize>) =
        (Vec::with_capacity(total), Vec::with_capacity(total), Vec::new());
    for n in &graph.nodes {
        for c in &n.columns {
            let name = *name_at.entry(c.name.as_str()).or_insert_with(|| {
                names.push(c.name.as_str());
                names.len() - 1
            });
            let ty_id = *type_at.entry(c.data_type.as_str()).or_insert_with(|| {
                types.push(c.data_type.as_str());
                types.len() - 1
            });
            if !c.tests.is_empty() {
                tests.extend([nm.len(), c.tests.len()]);
            }
            nm.push(name);
            ty.push(ty_id);
        }
    }

    // Sorted by (from, to), as `edges` is held; `from` as the step from the one before.
    let (mut from, mut to, mut kind) = (Vec::new(), Vec::new(), Vec::new());
    let mut last = 0;
    for e in cll.edge_list() {
        let f = num(e.from);
        from.push(f - last);
        last = f;
        to.push(num(e.to));
        kind.push(e.kind);
    }

    let roots: Vec<Value> = cll.root_list().iter().map(|(c, values)| json!([num(*c), values])).collect();
    let expressions: Vec<Value> =
        cll.expression_list().iter().map(|(c, x)| json!([num(*c), x.kind, x.text, x.steps, u8::from(x.cut)])).collect();
    let lost: Vec<Value> = cll.lost_columns().into_iter().map(|c| json!([num(c), cll.lost(c)])).collect();
    let outside: Vec<usize> = cll.outside_list().iter().map(|c| num(*c)).collect();

    // A model's rules as the card says them, each with the columns it reads by number: any column of
    // the node it names whose name is that one in ASCII case, as the card matches them. A rule that
    // reads none is never said.
    let mut rules: Vec<Value> = Vec::new();
    let mut held: Option<(u32, Vec<Value>)> = None;
    for (node, r) in cll.rule_list() {
        let mut cols: Vec<usize> = Vec::new();
        for x in &r.columns {
            let Some(&m) = graph.index.get(&x.node) else { continue };
            for (slot, c) in graph.nodes[m as usize].columns.iter().enumerate() {
                if x.column.eq_ignore_ascii_case(&c.name) {
                    cols.push(at[m as usize] + slot);
                }
            }
        }
        cols.sort_unstable();
        cols.dedup();
        if cols.is_empty() {
            continue;
        }
        let text = format!("{} {}", r.clause, crate::graph::said_cut(&ls::field(&r.text), r.cut));
        match held.as_mut().filter(|(h, _)| h == node) {
            Some((_, list)) => list.push(json!([text, cols])),
            None => {
                if let Some((h, list)) = held.take() {
                    rules.push(json!([h, list]));
                }
                held = Some((*node, vec![json!([text, cols])]));
            }
        }
    }
    if let Some((h, list)) = held {
        rules.push(json!([h, list]));
    }

    Some(json!({
        "v": 1,
        "kinds": cll.kinds,
        "rank": cll.kinds.iter().map(|k| crate::graph::row_rank(k)).collect::<Vec<_>>(),
        "row": cll.row_kind_list(),
        "at": at,
        "names": names,
        "nm": nm,
        "types": types,
        "ty": ty,
        "tests": tests,
        "from": from,
        "to": to,
        "ek": kind,
        "roots": roots,
        "expr": expressions,
        "lost": lost,
        "outside": outside,
        "rules": rules,
    }))
}

// -------------------------------------------------------------------- jobs ----

/// What the jobs add to a site (0183): the entry of the index's meta, and the script `data/jobs.js`.
pub(crate) struct JobsData {
    pub meta: Value,
    pub js: String,
}

/// Where the files of one job source are on its host, at a commit.
#[derive(Default, Debug, PartialEq)]
pub(crate) struct JobLinks {
    /// The commit's first seven characters, "" when none is known.
    pub commit: String,
    pub commit_url: String,
    /// Up to what the page appends, the job file's path in its source's folder; None when no file is linked.
    pub base: Option<String>,
    /// What follows `#` before a line number: GitHub and GitLab say `L`, Bitbucket `lines-`.
    pub anchor: &'static str,
}

/// What stderr says of a repository at the home folder or above it.
const HOME_REPOSITORY: &str = "the repository it sits in is at the home folder or above it, which is no project's own";

/// Whether a repository's top is the home folder or a folder above it, where the home folder is as the system says:
/// a dotfiles repository, which holds everything under it and is nobody's project. Not asked when the home folder
/// is not known, as `resolve_real`'s own check is not (0115).
fn at_or_above_home(top: &Path, home: Option<&Path>) -> bool {
    home.and_then(|h| h.canonicalize().ok()).map(files::plain).is_some_and(|h| h.starts_with(top))
}

/// Whether the repository a source's folder was found in may be read for it. git finds one by walking up with no
/// ceiling, so a source with no repository of its own would be placed in whatever holds the project's
/// parents: the commit and the address of the wrong repository, linked to as if they were the jobs'. It is
/// taken when it is the dbt project's own repository, where jobs kept in the project live, or does not hold the
/// project at all; and never when it is at the home folder or above it, whoever's repository that is.
pub(crate) fn repository_is_the_sources(top: &Path, project: &Path, project_top: Option<&Path>, home: Option<&Path>) -> bool {
    !at_or_above_home(top, home) && (project_top == Some(top) || !project.starts_with(top))
}

/// The commit and the file links of one source that was read. The commit and its link come from
/// `--jobs-commit` and `--jobs-repo-url` when they are given, whatever git says; what git is asked, through the gate
/// (0121) at the source's real folder, is the top of its repository, its `HEAD`, its origin and the folder's place
/// in it, and only when the repository is the sources' own. A file is linked only where that place is known: with no
/// repository there is none, flags or not. stderr says what is missing.
fn job_links(
    root: &Path, project_top: Option<&Path>, home: Option<&Path>, folder: Option<&Path>, name: &str, commit: Option<&str>,
    repo: Option<&Repo>, err: &mut impl Write,
) -> JobLinks {
    let mut say = |why: &str| {
        let _ = writeln!(err, "no job file links for {name}: {why}");
    };
    // Whether git may be asked about this source at all, and if not, why.
    let project = files::plain(root.canonicalize().unwrap_or_else(|_| root.to_path_buf()));
    let asked = match folder {
        None => Err("its folder cannot be placed".to_string()),
        Some(f) => match git::top_level(f).and_then(|t| t.canonicalize().ok()).map(files::plain) {
            None if git::in_repository(f) => Err("git did not say where its repository is".to_string()),
            None => Err("not a git repository".to_string()),
            Some(top) if !repository_is_the_sources(&top, &project, project_top, home) => Err(if at_or_above_home(&top, home) {
                HOME_REPOSITORY.to_string()
            } else {
                "the repository it sits in holds the project's parent folders, not its own".to_string()
            }),
            Some(_) => Ok(f),
        },
    };
    let git_folder = asked.as_ref().ok().copied();
    let sha = match (commit, git_folder) {
        (Some(c), _) => Some(c.to_string()),
        (None, Some(f)) => match git::head_commit(f) {
            Ok(c) => Some(c),
            Err(why) => {
                say(&format!("no commit: {why}"));
                None
            }
        },
        (None, None) => None,
    };
    let derived;
    let repo = match (repo, git_folder) {
        (Some(r), _) => Some(r),
        (None, Some(f)) => {
            derived = git::origin_url(f).and_then(|o| repo_from_origin(&o));
            if derived.is_none() {
                say("its origin is not on github.com, gitlab.com or bitbucket.org, pass --jobs-repo-url");
            }
            derived.as_ref()
        }
        (None, None) => None,
    };
    let mut links = JobLinks::default();
    if let (Some(sha), Some(repo)) = (&sha, repo) {
        links = JobLinks { commit: sha.chars().take(7).collect(), commit_url: commit_link(repo, sha), base: None, anchor: "L" };
        if repo.host == Host::Bitbucket {
            links.anchor = "lines-";
        }
    }
    if let Err(why) = &asked {
        // No folder's place is known, so no file is linked; the commit's link is the flags' when they gave both.
        let rest = if links.commit_url.is_empty() {
            ", pass --jobs-commit and --jobs-repo-url for the commit link"
        } else {
            ", the commit link is from the flags"
        };
        say(&format!("{why}{rest}"));
        return links;
    }
    let (Some(sha), Some(repo), Some(folder)) = (sha, repo, git_folder) else { return links };
    if repo.host == Host::Azure {
        say("the file addresses of Azure DevOps are not written yet");
        return links;
    }
    links.base = git::show_prefix(folder).and_then(|prefix| file_base(repo, &sha, &prefix));
    if links.base.is_none() {
        say("git did not say where its folder sits in the repository");
    }
    links
}

/// Reads the jobs the project's `.dbt-edith/jobs.yml` names, as the command line does (0115), and lays them
/// out for the page: None when there is no file, so no tab. A file that cannot be read, a source that cannot
/// be walked and a job file that does not parse are said on stderr and carried in the data, never as a job
/// that does not exist.
fn jobs_of(
    graph: &Graph, root: &Path, project_top: Option<&Path>, home: Option<&Path>, commit: Option<&str>, repo: Option<&Repo>,
    err: &mut impl Write,
) -> Option<JobsData> {
    let sources = jobs::sources(root, &[]);
    let (known, _) = jobs::known_from(graph, root, &sources);
    let refused = match &known {
        jobs::Known::Absent => return None,
        jobs::Known::Refused(why) => Some(why.clone()),
        jobs::Known::Read(_) => None,
    };
    let empty = jobs::Read::default();
    let read: &jobs::Read = match &known {
        jobs::Known::Read(r) => r,
        _ => &empty,
    };
    // `jobs: []` names nothing to read: a tab that offered to add a source would be the server's page, not a site's.
    if refused.is_none() && read.sources.is_empty() {
        return None;
    }
    let mut links: Vec<JobLinks> = Vec::new();
    for (si, s) in read.sources.iter().enumerate() {
        let name = jobs::source_name(read, si);
        if !jobs::source_was_read(s) {
            let _ = writeln!(
                err,
                "warning: jobs not read: {name} ({})",
                if s.error.is_empty() { "stopped at its bound on files or depth" } else { &s.error }
            );
            links.push(JobLinks::default());
            continue;
        }
        if !s.refused.is_empty() {
            let _ = writeln!(err, "warning: {} job file(s) of {name} could not be read and may hold more", s.refused.len());
        }
        let folder = sources.list.get(si).and_then(|src| jobs::walked_folder(root, src));
        links.push(job_links(root, project_top, home, folder.as_deref(), &name, commit, repo, err));
    }
    if let Some(why) = &refused {
        let _ = writeln!(err, "warning: jobs not read: {} is not read: {why}", jobs::SOURCES_FILE);
    }
    let meta = jobs_meta(read, refused.as_deref(), &links);
    let js = format!("EDITH_SITE_JOBS({});\n", js_json(&jobs_payload(graph, read, refused.as_deref())));
    Some(JobsData { meta, js })
}

/// The index's entry for the jobs (`meta.jobs`): who was read, where each source's files are linked, and the counts the
/// page says. Nothing of this machine, and no time.
fn jobs_meta(read: &jobs::Read, refused: Option<&str>, links: &[JobLinks]) -> Value {
    json!({
        "sources": read.sources.iter().enumerate().map(|(si, _)| {
            let l = &links[si];
            json!({ "name": jobs::source_name(read, si), "commit": l.commit, "commit_url": l.commit_url, "base": l.base, "anchor": l.anchor })
        }).collect::<Vec<_>>(),
        "present": read.sources.iter().filter(|s| jobs::source_was_read(s)).count(),
        "not_read": read.sources.iter().filter(|s| !jobs::source_was_read(s)).count(),
        "refused": read.sources.iter().map(|s| s.refused.len()).sum::<usize>(),
        "sources_refused": refused.unwrap_or_default(),
        "jobs": read.jobs.len(),
        "steps": read.steps.len(),
    })
}

/// A sorted list of node numbers as runs, `[gap, length, ...]`: each run starts `gap` after the end of the one before
/// (the first, after 0). A step that selects nearly everything is a few numbers, not one per node.
pub(crate) fn node_runs(nodes: &[u32]) -> Vec<u32> {
    let mut out: Vec<u32> = Vec::new();
    let mut end = 0u32;
    let mut i = 0;
    while i < nodes.len() {
        let start = nodes[i];
        let mut len = 1;
        while i + len < nodes.len() && nodes[i + len] == start + len as u32 {
            len += 1;
        }
        out.extend([start - end, len as u32]);
        end = start + len as u32;
        i += len;
    }
    out
}

/// The argument of `EDITH_SITE_JOBS`: the summary `/api/jobs` gives, and what `/api/jobs/node` answers for every
/// node, worked out by the server's own code and laid out so that nothing grows with the product of nodes and
/// steps. A job and a step are in a list of their own, in the order of the files they were read from, a step
/// as `StepView` writes it for no node in particular. A step that runs nodes says which as runs of node numbers
/// (`runs`), and why, by node, as the terms of its selection with the nodes each reaches on its own (`via`: `["t",
/// [[term], [term, runs]]]`, a term with no runs of its own reaching what the step does) or as one fixed line
/// (`["f", [line]]`); `web/site.js` finds the steps of a node by looking it up in each, and the terms that reach
/// it the same way. What a step that could not be resolved might run is by kind of node, not by node (`maybe`).
/// What every node's answer says the same (the sources read, those that were not, the files refused) is said once.
pub(crate) fn jobs_payload(graph: &Graph, read: &jobs::Read, refused: Option<&str>) -> Value {
    let mut summary = serde_json::to_value(jobs::summary(read)).unwrap_or(Value::Null);
    // Where a source led on this machine is not the site's to say: the source is as the file wrote it.
    if let Some(list) = summary["sources"].as_array_mut() {
        for s in list {
            if let Some(o) = s.as_object_mut() {
                o.remove("resolved");
            }
        }
    }
    summary["from"] = json!("project");
    if let Some(why) = refused {
        summary["refused"] = json!(why);
    }

    // What a node of each kind is told that the server's answer says alike for every node of that kind: the
    // steps that might run it, and the rest of the answer, which is the same for all.
    let mut first_of_kind: BTreeMap<&str, u32> = BTreeMap::new();
    for (i, n) in graph.nodes.iter().enumerate() {
        first_of_kind.entry(n.kind.as_str()).or_insert(i as u32);
    }
    let mut reach = jobs::Reach::default();
    let mut common: Option<Value> = None;
    let mut maybe: BTreeMap<String, Vec<(usize, Vec<usize>)>> = BTreeMap::new();
    for (kind, node) in &first_of_kind {
        let found = jobs::for_node_with(graph, read, *node, &mut reach);
        if common.is_none() {
            let mut shared = serde_json::to_value(&found).unwrap_or(Value::Null);
            if let Some(o) = shared.as_object_mut() {
                for k in ["kind", "groups", "maybe"] {
                    o.remove(k);
                }
            }
            common = Some(shared);
        }
        let groups: Vec<(usize, Vec<usize>)> =
            found.maybe.iter().map(|g| (read.steps[g.steps[0].index].job, g.steps.iter().map(|s| s.index).collect())).collect();
        if !groups.is_empty() {
            maybe.insert((*kind).to_string(), groups);
        }
    }
    let mut common = common.unwrap_or_else(|| json!({ "configured": !read.sources.is_empty() }));
    if let Some(why) = refused {
        common["sources_refused"] = json!(why);
    }

    // The steps that run some node, and those that might run one, each once, in the order they were read.
    let mut shipped: Vec<usize> =
        (0..read.steps.len()).filter(|&si| read.steps[si].executes() && !read.steps[si].nodes.is_empty()).collect();
    shipped.extend(maybe.values().flatten().flat_map(|(_, steps)| steps.iter().copied()));
    shipped.sort_unstable();
    shipped.dedup();
    let step_at: HashMap<usize, usize> = shipped.iter().enumerate().map(|(at, &si)| (si, at)).collect();
    let job_ids: std::collections::BTreeSet<usize> = shipped.iter().map(|&si| read.steps[si].job).collect();
    let job_at: HashMap<usize, usize> = job_ids.iter().enumerate().map(|(at, &ji)| (ji, at)).collect();

    let mut runs: Vec<Value> = Vec::with_capacity(shipped.len());
    let mut via: Vec<Value> = Vec::with_capacity(shipped.len());
    let mut terms = jobs::TermMemo::default();
    for &si in &shipped {
        let step = &read.steps[si];
        if !step.executes() || step.nodes.is_empty() {
            runs.push(json!(0));
            via.push(json!(0));
            continue;
        }
        let own = node_runs(&step.nodes);
        // A step of one term and no exclusion runs what that term reaches, filtered by kind: every node it runs is reached by
        // the term, and the term is not resolved a second time.
        let lone = step.inv.as_ref().is_some_and(|inv| inv.selector.is_none() && inv.select.len() == 1 && inv.exclude.is_empty());
        via.push(
            match step.inv.as_ref().map(|inv| {
                if lone {
                    jobs::ViaOf::Terms(vec![(inv.select[0].clone(), Default::default())])
                } else {
                    jobs::via_of(graph, inv, &mut terms)
                }
            }) {
                Some(jobs::ViaOf::Fixed(line)) => json!(["f", line]),
                Some(jobs::ViaOf::Terms(found)) => json!([
                    "t",
                    found
                        .iter()
                        .map(|(text, nodes)| {
                            // A term that reaches what the step runs says only its text.
                            if lone || node_runs(nodes) == own {
                                json!([text])
                            } else {
                                json!([text, node_runs(nodes)])
                            }
                        })
                        .collect::<Vec<_>>()
                ]),
                None => json!(0),
            },
        );
        runs.push(json!(own));
    }
    let packed_maybe: serde_json::Map<String, Value> = maybe
        .iter()
        .map(|(kind, groups)| {
            let list: Vec<Value> =
                groups.iter().map(|(ji, steps)| json!([job_at[ji], steps.iter().map(|si| step_at[si]).collect::<Vec<_>>()])).collect();
            (kind.clone(), json!(list))
        })
        .collect();
    json!({
        "v": 2,
        "summary": summary,
        "common": common,
        "jobs": job_ids.iter().map(|&ji| serde_json::to_value(jobs::job_view(read, ji)).unwrap_or(Value::Null)).collect::<Vec<_>>(),
        "steps": shipped.iter().map(|&si| serde_json::to_value(jobs::step_view(graph, read, si, None)).unwrap_or(Value::Null)).collect::<Vec<_>>(),
        "step_job": shipped.iter().map(|&si| job_at[&read.steps[si].job]).collect::<Vec<_>>(),
        "runs": runs,
        "via": via,
        "maybe": packed_maybe,
    })
}

// ------------------------------------------------------------------ flags ----

/// `--commit`: 7 to 64 hex characters, what a link to a commit can name.
fn valid_commit(sha: &str) -> Result<String, String> {
    let sha = sha.trim();
    if (7..=64).contains(&sha.len()) && sha.bytes().all(|b| b.is_ascii_hexdigit()) {
        Ok(sha.to_ascii_lowercase())
    } else {
        Err("--commit wants 7 to 64 hexadecimal characters".to_string())
    }
}

/// `--environment`: a label for the header and the title, so any text that
/// fits on a line.
fn valid_environment(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 100 || name.chars().any(char::is_control) {
        Err("--environment wants a name of one to 100 characters, on one line".to_string())
    } else {
        Ok(name.to_string())
    }
}

// ------------------------------------------------------------------- links ----

/// Which pattern a commit's page has on the host the repository is on.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Host {
    GitLab,
    Bitbucket,
    /// Azure DevOps: a commit has the common shape, but a file's address takes a
    /// query (`?path=`), which is not written yet.
    Azure,
    /// GitHub, and every host that is told by `--repo-url` without saying
    /// which it is: `/commit/<sha>` is the common shape.
    Other,
}

/// The repository a site links to: an https address with no credentials.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct Repo {
    pub url: String,
    host: Host,
}

/// The path of a repository, as its segments: each one made of the characters
/// a host allows in a name, so nothing written into a link can change what the
/// link points at. A `.git` ending is dropped.
fn segments(path: &str) -> Option<Vec<&str>> {
    let path = path.trim_matches('/');
    let path = path.strip_suffix(".git").unwrap_or(path);
    let parts: Vec<&str> = path.split('/').collect();
    let clean = |s: &&str| !s.is_empty() && *s != "." && *s != ".." && s.bytes().all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b));
    (parts.iter().all(clean)).then_some(parts)
}

/// What `remote.origin.url` says, turned into the repository's https address,
/// for exactly three hosts: github.com, gitlab.com and bitbucket.org. The scp
/// form, `ssh://` and https are read, a user and a token dropped, and a port of
/// ssh with them. A host that only looks like one of them, an alias in
/// `~/.ssh/config` (`work.github.com`), a suffix (`github.com.evil.io`) or a
/// user name before an `@`, gives no link at all: a link guessed for another
/// host is a link to the wrong place.
pub(crate) fn repo_from_origin(origin: &str) -> Option<Repo> {
    let origin = origin.trim();
    // A backslash is a slash to some readers of a URL and not to others.
    if origin.contains('\\') || origin.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return None;
    }
    let (host, path) = match origin.split_once("://") {
        Some((scheme, rest)) => {
            let scheme = scheme.to_ascii_lowercase();
            if scheme != "ssh" && scheme != "https" {
                return None;
            }
            let (auth, path) = rest.split_once('/')?;
            let hostport = auth.rsplit_once('@').map_or(auth, |(_, h)| h);
            let (host, port) = match hostport.split_once(':') {
                Some((h, p)) => (h, Some(p)),
                None => (hostport, None),
            };
            match port {
                // An https address on a port is not the host's public site.
                Some(_) if scheme == "https" => return None,
                Some(p) if p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit()) => return None,
                _ => {}
            }
            (host, path)
        }
        None => {
            // `user@host:path`: the colon comes before any slash.
            let (auth, path) = origin.split_once(':')?;
            if auth.contains('/') {
                return None;
            }
            (auth.rsplit_once('@').map_or(auth, |(_, h)| h), path)
        }
    };
    let host = host.to_ascii_lowercase();
    let (kind, exact) = match host.as_str() {
        "github.com" => (Host::Other, true),
        "gitlab.com" => (Host::GitLab, false),
        "bitbucket.org" => (Host::Bitbucket, true),
        _ => return None,
    };
    let parts = segments(path)?;
    // A user and a repository on GitHub and Bitbucket; GitLab nests groups.
    if parts.len() < 2 || (exact && parts.len() != 2) {
        return None;
    }
    Some(Repo { url: format!("https://{host}/{}", parts.join("/")), host: kind })
}

/// `--repo-url`: https, no user, no query, no fragment. Any host, since it was
/// named: a GitLab or a Bitbucket of the company is told by its name.
pub(crate) fn repo_from_flag(url: &str) -> Result<Repo, String> {
    let bad = || "--repo-url wants an https address with no user, query or fragment, as https://host/team/repo".to_string();
    let url = url.trim();
    let rest = url.get(..8).filter(|s| s.eq_ignore_ascii_case("https://")).map(|_| &url[8..]).ok_or_else(bad)?;
    if rest.chars().any(|c| matches!(c, '?' | '#' | '@' | '\\') || c.is_control() || c.is_whitespace()) {
        return Err(bad());
    }
    let (hostport, path) = rest.split_once('/').unwrap_or((rest, ""));
    let (host, port) = match hostport.split_once(':') {
        Some((h, p)) => (h, Some(p)),
        None => (hostport, None),
    };
    let host = host.to_ascii_lowercase();
    let name_ok = !host.is_empty()
        && !host.starts_with(['.', '-'])
        && !host.ends_with(['.', '-'])
        && host.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'.' || b == b'-');
    if !name_ok || port.is_some_and(|p| p.is_empty() || p.len() > 5 || !p.bytes().all(|b| b.is_ascii_digit())) {
        return Err(bad());
    }
    let parts = segments(path).ok_or_else(bad)?;
    let kind = if host.contains("gitlab") {
        Host::GitLab
    } else if host.contains("bitbucket") {
        Host::Bitbucket
    } else if host.contains("dev.azure.com") || host.contains("visualstudio.com") {
        Host::Azure
    } else {
        Host::Other
    };
    let at = match port {
        Some(p) => format!("{host}:{p}"),
        None => host,
    };
    Ok(Repo { url: format!("https://{at}/{}", parts.join("/")), host: kind })
}

/// The page of one commit: the pattern is the host's.
pub(crate) fn commit_link(repo: &Repo, sha: &str) -> String {
    let pattern = match repo.host {
        Host::GitLab => "-/commit",
        Host::Bitbucket => "commits",
        Host::Other | Host::Azure => "commit",
    };
    format!("{}/{pattern}/{sha}", repo.url)
}

/// A percent-encoding as `encodeURIComponent` writes it: what a path's segment
/// may hold on a host's address, so a `#`, a `?` or a space in a folder's name
/// stays in its segment.
fn escape_segment(text: &str) -> String {
    text.bytes()
        .map(|b| if b.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") })
        .collect()
}

/// Where the files of the project are on the host, at one commit, up to what the
/// page appends: a node's path, each segment escaped there. `prefix` is the
/// project's place in the repository (`git::show_prefix`), "" at its root. None
/// for a host whose file address this does not write (Azure's takes a query).
pub(crate) fn file_base(repo: &Repo, sha: &str, prefix: &str) -> Option<String> {
    let pattern = match repo.host {
        Host::GitLab => "-/blob",
        Host::Bitbucket => "src",
        Host::Other => "blob",
        Host::Azure => return None,
    };
    let prefix: String = prefix.split('/').filter(|s| !s.is_empty()).map(|s| format!("{}/", escape_segment(s))).collect();
    Some(format!("{}/{pattern}/{sha}/{prefix}", repo.url))
}

/// `file_base` for the run, with what stderr says when there is none. The
/// project's place in its repository is asked of git through the gate (0121);
/// without it a link would guess a path, so no file is linked.
fn file_links(repo: &Repo, sha: &str, root: &Path, err: &mut impl Write) -> Option<String> {
    if repo.host == Host::Azure {
        let _ = writeln!(err, "no file links: the file addresses of Azure DevOps are not written yet");
        return None;
    }
    let base = git::show_prefix(root).and_then(|prefix| file_base(repo, sha, &prefix));
    if base.is_none() {
        let why =
            if git::in_repository(root) { "git did not say where the project sits in its repository" } else { "not a git repository" };
        let _ = writeln!(err, "no file links: {why}");
    }
    base
}

// ------------------------------------------------------------------ folder ----

/// Where the site goes: strictly inside the project, never the project
/// itself, and either empty or marked as Edith's own. Anything else is refused
/// with the reason, since the folder is about to be emptied. `root` is the
/// project's canonical path.
///
/// A symlink at the folder, or any on the way to it, that leads out of the
/// project is refused by looking where the path really is, not at what it was
/// written as. A `..` is refused outright: a path that needs resolving twice
/// is a path someone has to read twice.
pub(crate) fn site_dir(root: &Path, given: Option<&Path>) -> Result<PathBuf, String> {
    let wanted = match given {
        Some(p) => std::path::absolute(p).map_err(|e| format!("cannot use {}: {e}", p.display()))?,
        None => root.join("target").join("edith-site"),
    };
    if wanted.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(format!("{} goes through `..`: name the folder without it", wanted.display()));
    }
    let in_git = |p: &Path| p.components().any(|c| c.as_os_str().eq_ignore_ascii_case(".git"));
    if in_git(&wanted) {
        return Err(format!("{} is inside a .git folder", wanted.display()));
    }
    if std::fs::symlink_metadata(&wanted).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err(format!("{} is a link: name the folder itself", wanted.display()));
    }
    // The nearest folder that exists is resolved; what is below it is made.
    let mut head = wanted.as_path();
    let mut below: Vec<&std::ffi::OsStr> = Vec::new();
    while std::fs::symlink_metadata(head).is_err() {
        let (Some(name), Some(parent)) = (head.file_name(), head.parent()) else {
            return Err(format!("cannot place {}", wanted.display()));
        };
        below.push(name);
        head = parent;
    }
    let mut full = crate::files::plain(head.canonicalize().map_err(|e| format!("cannot resolve {}: {e}", head.display()))?);
    full.extend(below.iter().rev());
    if full == root || !full.starts_with(root) {
        return Err(format!(
            "{} is not inside the project {}: the site is written below it, in a folder of its own (--site DIR)",
            wanted.display(),
            root.display()
        ));
    }
    // Where a link on the way leads, as well as what was written: a link inside the project to `.git`
    // is a way into it.
    if full.strip_prefix(root).is_ok_and(in_git) {
        return Err(format!("{} is inside a .git folder", full.display()));
    }
    if full.exists() {
        if !full.is_dir() {
            return Err(format!("{} is not a folder", full.display()));
        }
        let empty = std::fs::read_dir(&full).map_err(|e| format!("cannot read {}: {e}", full.display()))?.next().is_none();
        let marked = std::fs::symlink_metadata(full.join(MARKER)).is_ok_and(|m| m.is_file());
        if !empty && !marked {
            return Err(format!(
                "{} is not empty and was not written by --site: it is left alone. Name an empty folder, or one a previous run wrote",
                full.display()
            ));
        }
    }
    Ok(full)
}

/// Empties a folder `site_dir` accepted, but for its marker, and writes the
/// marker first: a run that stops half way leaves a folder the next run still
/// recognises as its own.
fn clear(dir: &Path) -> std::io::Result<()> {
    // A link put at the folder since `site_dir` looked would send the emptying where it leads.
    if std::fs::symlink_metadata(dir).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err(std::io::Error::other("the folder is a link"));
    }
    std::fs::create_dir_all(dir)?;
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        if entry.file_name() == MARKER {
            continue;
        }
        // Never followed: a link is removed, what it points at is not.
        if entry.file_type()?.is_dir() {
            std::fs::remove_dir_all(entry.path())?;
        } else {
            std::fs::remove_file(entry.path())?;
        }
    }
    std::fs::write(dir.join(MARKER), MARKER_TEXT)
}

// -------------------------------------------------------------------- page ----

/// The page's title: the project, the environment when there is one, and the
/// tool, the way the page words it for itself.
pub(crate) fn title_of(project: &str, environment: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    if !project.is_empty() {
        parts.push(project);
    }
    if !environment.is_empty() {
        parts.push(environment);
    }
    parts.push("dbt-edith");
    parts.join(" \u{b7} ")
}

fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// `web/index.html` made the site's: each anchor is replaced exactly once or
/// the run fails, so a markup change in the page cannot silently leave the
/// site without its policy or its data. There is no inline script.
pub(crate) fn page(index_html: &str, title: &str) -> Result<String, String> {
    let anchors: [(&str, String); 4] = [
        ("<html lang=\"en\" data-theme=\"night\">", "<html lang=\"en\" data-theme=\"night\" data-site>".to_string()),
        ("<meta charset=\"utf-8\">", format!("<meta charset=\"utf-8\">\n<meta http-equiv=\"Content-Security-Policy\" content=\"{CSP}\">")),
        ("<title>dbt-edith</title>", format!("<title>{}</title>", escape_html(title))),
        (
            "<script src=\"lineage.js\"></script>",
            "<script src=\"data/index.js\"></script>\n<script src=\"site.js\"></script>\n<script src=\"lineage.js\"></script>".to_string(),
        ),
    ];
    let mut html = index_html.to_string();
    for (anchor, with) in &anchors {
        let found = html.matches(anchor).count();
        if found != 1 {
            return Err(format!("web/index.html has {found} of `{anchor}`, and the site's page needs exactly one"));
        }
        html = html.replacen(anchor, with, 1);
    }
    Ok(html)
}

/// The files a page loads by a relative address, in the order they appear:
/// every `src` and `href` that is not a fragment, a data URI or another site.
pub(crate) fn loaded_by(html: &str) -> Vec<String> {
    let mut found = Vec::new();
    for attr in ["src=\"", "href=\""] {
        for (at, _) in html.match_indices(attr) {
            // A word boundary: `data-src="` and the like are not this.
            if html[..at].chars().next_back().is_some_and(|c| c.is_ascii_alphanumeric() || c == '-') {
                continue;
            }
            let rest = &html[at + attr.len()..];
            let Some(end) = rest.find('"') else { continue };
            let value = &rest[..end];
            if value.is_empty() || value.starts_with('#') || value.starts_with("data:") || value.contains("//") {
                continue;
            }
            found.push((at, value.to_string()));
        }
    }
    found.sort();
    found.dedup_by(|a, b| a.1 == b.1);
    found.into_iter().map(|(_, v)| v).collect()
}

// -------------------------------------------------------------------- data ----

/// JSON that is also a JavaScript expression wherever it is put: U+2028 and
/// U+2029 end a line there, and `<` would let a name close a `<script>`.
pub(crate) fn js_json(value: &Value) -> String {
    let text = serde_json::to_string(value).unwrap_or_else(|_| "null".to_string());
    text.replace('<', "\\u003c").replace('\u{2028}', "\\u2028").replace('\u{2029}', "\\u2029")
}

/// Where a node's file sits in the tree the page draws: a model, a seed or a snapshot that is not
/// disabled, as `Graph::node_built_in` takes one, at the file dbt-edith found for it under
/// `dbt_packages`, else its own file for the root project. A package node with no place, because the
/// package was not installed where the site was written, is put where `dbt deps` would put it. The
/// flag says that last case. None for what no file builds: a source, an exposure, a test.
fn tree_path(graph: &Graph, n: &crate::graph::Node) -> Option<(String, bool)> {
    if !matches!(n.kind, Kind::Model | Kind::Seed | Kind::Snapshot) || n.disabled {
        return None;
    }
    if !n.place.is_empty() {
        return Some((n.place.clone(), false));
    }
    if n.file.is_empty() {
        return None;
    }
    let root = graph.macros.root();
    if n.package.is_empty() || root.is_empty() || n.package == root {
        Some((n.file.clone(), false))
    } else {
        Some((format!("dbt_packages/{}/{}", n.package, n.file), true))
    }
}

/// The overview of the project's model folders (the editor's tree, of what the manifest builds): per
/// folder, its entries as `/api/dir` lists them, folders first and then by lower-cased name, a
/// folder as -1 and a file as its node. Ordered here, so the page compares no string and lower-cases
/// none: the order is Rust's, `to_lowercase` of any name a path can hold included. A folder is every
/// ancestor of such a file and nothing else, so no `tests/`, no `macros/` and no folder of YAML only
/// appears. A file several nodes sit in, a legacy snapshot file, is listed once, with the first of
/// them by name, which is the one `node_built_in` returns. Also, the nodes whose file is not where
/// their `file` says, for the page to find a node's row.
pub(crate) fn tree_of(graph: &Graph) -> (BTreeMap<String, Vec<(String, i64)>>, BTreeMap<String, String>) {
    let mut at: BTreeMap<String, u32> = BTreeMap::new();
    let mut placed: BTreeMap<String, String> = BTreeMap::new();
    for (i, n) in graph.nodes.iter().enumerate() {
        let Some((path, moved)) = tree_path(graph, n) else { continue };
        let first = at.get(&path).is_none_or(|&held| graph.by_name_then_id(i as u32, held).is_lt());
        if first {
            at.insert(path.clone(), i as u32);
        }
        if moved {
            placed.insert(i.to_string(), path);
        }
    }
    let mut dirs: BTreeMap<String, BTreeMap<String, i64>> = BTreeMap::new();
    for (path, n) in &at {
        let parts: Vec<&str> = path.split('/').collect();
        for (k, name) in parts.iter().enumerate() {
            let dir = parts[..k].join("/");
            let entry = dirs.entry(dir).or_default();
            if k + 1 == parts.len() {
                entry.insert((*name).to_string(), i64::from(*n));
            } else {
                entry.entry((*name).to_string()).or_insert(-1);
            }
        }
    }
    let tree = dirs
        .into_iter()
        .map(|(dir, names)| {
            let mut list: Vec<(String, i64)> = names.into_iter().collect();
            list.sort_by(|a, b| {
                (a.1 >= 0).cmp(&(b.1 >= 0)).then_with(|| a.0.to_lowercase().cmp(&b.0.to_lowercase())).then_with(|| a.0.cmp(&b.0))
            });
            (dir, list)
        })
        .collect();
    (tree, placed)
}

/// Everything the page needs besides the cards, one entry per node number.
/// `base` is each node as the lineage draws it, minus what a walk decides
/// (`Graph::box_base`); the adjacency, the rank by name and the search key are
/// what the walks in `web/site.js` read, so none of them compares a string.
pub(crate) fn index_of(graph: &Graph, meta: Value) -> Value {
    let n = graph.nodes.len() as u32;
    let mut order: Vec<u32> = (0..n).collect();
    order.sort_by(|&a, &b| graph.by_name_then_id(a, b));
    let mut rank = vec![0u32; n as usize];
    for (r, &i) in order.iter().enumerate() {
        rank[i as usize] = r as u32;
    }
    let nodes = &graph.nodes;
    let (tree, placed) = tree_of(graph);
    json!({
        "v": 1,
        "meta": meta,
        "limits": {
            "max_levels": MAX_LEVELS,
            "depth": api::LINEAGE_DEPTH,
            "depth_max": api::LINEAGE_DEPTH_MAX,
            "max": api::LINEAGE_NODES,
            "max_max": api::LINEAGE_NODES_MAX,
            "search": api::SEARCH_DEFAULT,
            "search_max": api::SEARCH_MAX,
            // The column canvas's: two levels each way unless asked, twenty at most, a cap of boxes drawn
            // unless a click lifts it, and the most it may be lifted to; and how many names a card lists.
            "col_depth": api::LINEAGE_DEPTH,
            "col_depth_max": api::COL_DEPTH_MAX,
            "col_drawn": api::COL_DRAWN,
            "col_most": api::COL_MOST,
            "rows_by": crate::graph::ROWS_BY,
        },
        "base": (0..n).map(|i| graph.box_base(i)).collect::<Vec<_>>(),
        "parents": nodes.iter().map(|n| &n.parents).collect::<Vec<_>>(),
        "children": nodes.iter().map(|n| &n.children).collect::<Vec<_>>(),
        "tests": nodes.iter().map(|n| &n.tests).collect::<Vec<_>>(),
        "rank": rank,
        "key": nodes.iter().map(|n| &n.search_key).collect::<Vec<_>>(),
        "tree": tree,
        "placed": placed,
        "att": nodes.iter().map(|n| graph.attached_index(n).map_or(-1, i64::from)).collect::<Vec<_>>(),
        // Only what the depth menu tallies: a test or an operation is never one.
        "lv": (0..n).map(|i| if nodes[i as usize].kind.is_graph_node() { graph.level_label(i) } else { String::new() }).collect::<Vec<_>>(),
    })
}

/// What `build` hands to the writer: every file's text, nothing on disk yet.
pub(crate) struct Built {
    pub html: String,
    pub index: String,
    /// One file per bucket of cards, in node order.
    pub buckets: Vec<String>,
    /// One file per bucket of SQL, cut by its own rule.
    pub sql: Vec<String>,
    /// The column lineage, one file, when a cache was merged (0182).
    pub cll: Option<String>,
    /// The jobs, one file, when the project names job sources (0183). Set by the run, which reads them.
    pub jobs: Option<String>,
    /// The assets the page loads, by their path in the folder.
    pub assets: Vec<(String, Vec<u8>)>,
}

/// One node's code as the manifest holds it (0180). Read from the manifest alone, never from a file
/// of `target/`, so the site stays the picture of one manifest.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Sql {
    pub raw: String,
    /// None when dbt did not compile the node when it wrote the manifest: a `dbt parse` or the
    /// language server writes none.
    pub compiled: Option<String>,
    /// A Python model: shown as plain text, there being no mode for it.
    pub python: bool,
}

/// The fields of a node the SQL is read from, in a second pass over the manifest's file: the server's
/// `RawNode` keeps no code in memory and stays as it is.
#[derive(serde::Deserialize, Default)]
struct CodeNode {
    #[serde(default)]
    raw_code: Option<String>,
    #[serde(default)]
    compiled_code: Option<String>,
    #[serde(default)]
    language: Option<String>,
}

#[derive(serde::Deserialize, Default)]
pub(crate) struct Code {
    #[serde(default)]
    nodes: HashMap<String, CodeNode>,
    #[serde(default)]
    disabled: HashMap<String, Vec<CodeNode>>,
}

/// The manifest's code, read from the same file the graph was: only these three fields of a node, the
/// rest skipped as it is read.
pub(crate) fn read_code(manifest: &Path) -> Result<Code, String> {
    let file = std::fs::File::open(manifest).map_err(|e| format!("cannot read {}: {e}", manifest.display()))?;
    serde_json::from_reader(std::io::BufReader::new(file)).map_err(|e| format!("cannot read the SQL of {}: {e}", manifest.display()))
}

/// What a card has a Code tab for: `web/app.js` lists the same kinds. A seed, a source and an
/// exposure have no code of their own.
fn has_code(kind: Kind) -> bool {
    matches!(kind, Kind::Model | Kind::Snapshot | Kind::Test | Kind::Analysis | Kind::Operation)
}

/// A line ending is one: a manifest written on Windows holds the file's CRLF, which a page shows twice.
fn lf(text: String) -> String {
    if text.contains("\r\n") {
        text.replace("\r\n", "\n")
    } else {
        text
    }
}

/// The code of each node, by the graph's numbering: a node the graph took from `nodes`, and a disabled
/// one from the first entry of `disabled`, as `Graph::build` does (the two cannot be told apart by id
/// alone: a disabled node is in `disabled` and not in `nodes`). None for a node with no code to show,
/// a seed's empty `raw_code` included.
pub(crate) fn sql_of(graph: &Graph, mut code: Code) -> Vec<Option<Sql>> {
    graph
        .nodes
        .iter()
        .map(|n| {
            if !has_code(n.kind) {
                return None;
            }
            let node = if n.disabled {
                let held = code.disabled.get_mut(&n.id)?;
                if held.is_empty() {
                    return None;
                }
                held.swap_remove(0)
            } else {
                code.nodes.remove(&n.id)?
            };
            let raw = node.raw_code.unwrap_or_default();
            if raw.is_empty() && node.compiled_code.is_none() {
                return None;
            }
            Some(Sql { raw: lf(raw), compiled: node.compiled_code.map(lf), python: node.language.as_deref() == Some("python") })
        })
        .collect()
}

/// A bucket's entry: the raw text, the compiled when there is one, `lang` only for what is not SQL.
fn sql_entry(sql: Option<Sql>) -> Value {
    let Some(s) = sql else { return json!(0) };
    let mut entry = json!({ "raw": s.raw });
    if let Some(c) = s.compiled {
        entry["compiled"] = json!(c);
    }
    if s.python {
        entry["lang"] = json!("python");
    }
    entry
}

/// Every item as the text of one script call, in contiguous runs of node numbers that close at
/// `max_nodes` items or `max_bytes` of text, whichever comes first: the page loads a run when one of
/// its items is first opened. `call` is the function the script calls, `start` its first argument.
/// Where each run starts, and the text of its file.
fn runs_of(
    items: impl Iterator<Item = Result<String, String>>, call: &str, max_nodes: usize, max_bytes: usize,
) -> Result<(Vec<usize>, Vec<String>), String> {
    let mut cuts: Vec<usize> = Vec::new();
    let mut buckets: Vec<String> = Vec::new();
    let mut run: Vec<String> = Vec::new();
    let mut bytes = 0;
    let mut start = 0;
    let flush = |run: &mut Vec<String>, start: usize, buckets: &mut Vec<String>| {
        buckets.push(format!("{call}({start},[{}]);\n", run.join(",")));
        run.clear();
    };
    for (i, item) in items.enumerate() {
        if run.is_empty() {
            start = i;
            cuts.push(i);
            bytes = 0;
        }
        let item = item?;
        bytes += item.len() + 1;
        run.push(item);
        if run.len() >= max_nodes || bytes >= max_bytes {
            flush(&mut run, start, &mut buckets);
        }
    }
    if !run.is_empty() {
        flush(&mut run, start, &mut buckets);
    }
    Ok((cuts, buckets))
}

/// Every node's card in runs (`runs_of`), with collin's note on the model when its report had one.
fn cards_in(
    graph: &Graph, notes: Option<&HashMap<String, ModelNote>>, max_nodes: usize, max_bytes: usize,
) -> Result<(Vec<usize>, Vec<String>), String> {
    let cards = (0..graph.nodes.len()).map(|i| {
        let note = notes.and_then(|m| m.get(&graph.nodes[i].id)).cloned();
        serde_json::to_value(api::node_detail(graph, i as u32, note, None)).map(|v| js_json(&v)).map_err(|e| e.to_string())
    });
    runs_of(cards, "EDITH_SITE_CARDS", max_nodes, max_bytes)
}

/// Every node's code in runs, on a cut of their own: a card is a few kilobytes and a node's SQL
/// may be a hundred times that, so the two do not close together.
fn sql_in(code: Vec<Option<Sql>>, max_nodes: usize, max_bytes: usize) -> Result<(Vec<usize>, Vec<String>), String> {
    runs_of(code.into_iter().map(|s| Ok(js_json(&sql_entry(s)))), "EDITH_SITE_SQL", max_nodes, max_bytes)
}

/// Lays out the index, the cards and the SQL, and rewrites the page: everything the
/// folder holds, as text, before any of it is written.
pub(crate) fn build(
    graph: &Graph, meta: Value, title: &str, code: Vec<Option<Sql>>, notes: Option<&HashMap<String, ModelNote>>,
) -> Result<Built, String> {
    let mut index = index_of(graph, meta);
    let (cuts, buckets) = cards_in(graph, notes, BUCKET_NODES, BUCKET_BYTES)?;
    index["cuts"] = json!(cuts);
    let (sql_cuts, sql) = sql_in(code, BUCKET_NODES, BUCKET_BYTES)?;
    index["sql_cuts"] = json!(sql_cuts);

    let embedded = Assets::get("index.html").ok_or("the page is not embedded")?;
    let html = page(&String::from_utf8_lossy(&embedded.data), title)?;
    let mut paths: BTreeSet<String> = loaded_by(&String::from_utf8_lossy(&embedded.data)).into_iter().collect();
    // Not in the page it is made from: the page above adds it.
    paths.insert("site.js".to_string());
    let mut assets = Vec::new();
    for path in paths {
        let file = Assets::get(&path).ok_or_else(|| format!("web/index.html loads {path}, which is not in web/"))?;
        assets.push((path, file.data.into_owned()));
    }
    assets.push(("LICENSE.txt".to_string(), include_bytes!("../LICENSE").to_vec()));
    assets.push(("THIRD_PARTY_NOTICES.md".to_string(), include_bytes!("../THIRD_PARTY_NOTICES.md").to_vec()));
    let cll = cll_of(graph).map(|v| format!("EDITH_SITE_CLL({});\n", js_json(&v)));
    Ok(Built { html, index: format!("window.EDITH_SITE={};\n", js_json(&index)), buckets, sql, cll, jobs: None, assets })
}

/// Empties the folder and writes every file, the marker first.
fn write_site(dir: &Path, built: &Built) -> std::io::Result<()> {
    clear(dir)?;
    let put = |rel: &str, bytes: &[u8]| -> std::io::Result<()> {
        let path = dir.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, bytes)
    };
    put("index.html", built.html.as_bytes())?;
    for (path, bytes) in &built.assets {
        put(path, bytes)?;
    }
    put("data/index.js", built.index.as_bytes())?;
    for (i, bucket) in built.buckets.iter().enumerate() {
        put(&format!("data/cards/{i:04}.js"), bucket.as_bytes())?;
    }
    for (i, bucket) in built.sql.iter().enumerate() {
        put(&format!("data/sql/{i:04}.js"), bucket.as_bytes())?;
    }
    if let Some(cll) = &built.cll {
        put("data/cll.js", cll.as_bytes())?;
    }
    if let Some(jobs) = &built.jobs {
        put("data/jobs.js", jobs.as_bytes())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dbt-edith-site-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir.canonicalize().unwrap()
    }

    // ---- links

    #[test]
    fn an_origin_is_turned_into_a_link_for_three_hosts_only() {
        let github = |p: &str| Some(Repo { url: format!("https://github.com/{p}"), host: Host::Other });
        for (origin, want) in [
            ("git@github.com:acme/shop.git", github("acme/shop")),
            ("git@github.com:acme/shop", github("acme/shop")),
            ("ssh://git@github.com/acme/shop.git", github("acme/shop")),
            ("ssh://git@github.com:22/acme/shop.git", github("acme/shop")),
            ("https://github.com/acme/shop.git", github("acme/shop")),
            ("https://github.com/acme/shop/", github("acme/shop")),
            ("https://x-access-token:ghs_abc123@github.com/acme/shop.git", github("acme/shop")),
            ("HTTPS://GitHub.COM/acme/Shop", github("acme/Shop")),
            ("  https://github.com/acme/shop.git\n", github("acme/shop")),
            ("git@gitlab.com:acme/data/shop.git", Some(Repo { url: "https://gitlab.com/acme/data/shop".into(), host: Host::GitLab })),
            ("https://gitlab.com/acme/shop", Some(Repo { url: "https://gitlab.com/acme/shop".into(), host: Host::GitLab })),
            ("git@bitbucket.org:acme/shop.git", Some(Repo { url: "https://bitbucket.org/acme/shop".into(), host: Host::Bitbucket })),
            // An alias of ~/.ssh/config names a host that only looks like one.
            ("git@work.github.com:acme/shop.git", None),
            ("https://github.com.evil.io/acme/shop", None),
            ("https://evil.io/github.com/acme/shop", None),
            ("https://github.com@evil.io/acme/shop", None),
            ("git@evilgithub.com:acme/shop.git", None),
            ("https://github.com:8443/acme/shop", None),
            ("ssh://git@github.com:port/acme/shop", None),
            ("https://evil.io\\@github.com/acme/shop", None),
            // The shapes a repository has on each host.
            ("git@github.com:shop.git", None),
            ("git@github.com:acme/shop/extra", None),
            ("git@bitbucket.org:acme/team/shop", None),
            ("https://github.com/acme/shop?tab=readme", None),
            ("https://github.com/acme/shop#readme", None),
            ("https://github.com/acme/%2e%2e", None),
            ("https://github.com/acme/..", None),
            ("https://github.com/acme/sh op", None),
            // Not a remote of the web.
            ("file:///srv/git/shop.git", None),
            ("git://github.com/acme/shop.git", None),
            ("http://github.com/acme/shop.git", None),
            ("/srv/git/shop.git", None),
            ("../shop", None),
            ("C:\\work\\shop", None),
            ("", None),
        ] {
            assert_eq!(repo_from_origin(origin), want, "{origin:?}");
        }
    }

    #[test]
    fn a_repo_url_is_https_without_a_user_a_query_or_a_fragment() {
        let ok = |url: &str| repo_from_flag(url).unwrap_or_else(|e| panic!("{url}: {e}"));
        assert_eq!(ok("https://github.com/acme/shop").url, "https://github.com/acme/shop");
        assert_eq!(ok("https://git.example.com/team/sub/shop.git/").url, "https://git.example.com/team/sub/shop");
        assert_eq!(ok("https://git.example.com:8443/shop").url, "https://git.example.com:8443/shop");
        assert_eq!(ok("HTTPS://Git.Example.com/shop").url, "https://git.example.com/shop");
        assert_eq!(ok("https://gitlab.example.com/team/shop").host, Host::GitLab);
        assert_eq!(ok("https://bitbucket.example.com/team/shop").host, Host::Bitbucket);
        assert_eq!(ok("https://git.example.com/gitlab-mirror/shop").host, Host::Other, "the host says which, not the path");
        for bad in [
            "http://github.com/acme/shop",
            "git@github.com:acme/shop.git",
            "ssh://github.com/acme/shop",
            "https://user@github.com/acme/shop",
            "https://user:token@github.com/acme/shop",
            "https://github.com/acme/shop?x=1",
            "https://github.com/acme/shop#top",
            "https://github.com/acme/%2e%2e/shop",
            "https://github.com/acme/../shop",
            "https://github.com/",
            "https://github.com",
            "https://-bad.example.com/shop",
            "https://exa mple.com/shop",
            "https://example.com:port/shop",
            "https:///shop",
            "file:///shop",
            "",
        ] {
            assert!(repo_from_flag(bad).is_err(), "{bad:?} was accepted");
        }
    }

    #[test]
    fn a_commit_is_linked_by_its_hosts_pattern() {
        let sha = "0123456789abcdef0123456789abcdef01234567";
        let link = |url: &str| commit_link(&repo_from_flag(url).unwrap(), sha);
        assert_eq!(link("https://github.com/acme/shop"), format!("https://github.com/acme/shop/commit/{sha}"));
        assert_eq!(link("https://gitlab.com/acme/shop"), format!("https://gitlab.com/acme/shop/-/commit/{sha}"));
        assert_eq!(link("https://bitbucket.org/acme/shop"), format!("https://bitbucket.org/acme/shop/commits/{sha}"));
        assert_eq!(link("https://git.example.com/shop"), format!("https://git.example.com/shop/commit/{sha}"));
        // A derived URL picks its pattern by the exact host.
        let derived = repo_from_origin("git@gitlab.com:acme/data/shop.git").unwrap();
        assert_eq!(commit_link(&derived, sha), format!("https://gitlab.com/acme/data/shop/-/commit/{sha}"));
    }

    #[test]
    fn a_commit_flag_is_seven_to_sixty_four_hex_characters() {
        assert_eq!(valid_commit("ABCDEF1"), Ok("abcdef1".to_string()));
        assert_eq!(valid_commit(&"a".repeat(64)), Ok("a".repeat(64)));
        for bad in ["abcdef", &"a".repeat(65), "abcdefg", "main", "", "abc def1", "abcdef1\n--x"] {
            assert!(valid_commit(bad).is_err(), "{bad:?}");
        }
    }

    // ------------------------------------------------------------ folders ----

    #[test]
    fn the_folder_is_inside_the_project_and_empty_or_marked() {
        let root = scratch("folder");
        std::fs::create_dir_all(root.join("models")).unwrap();
        std::fs::write(root.join("models/a.sql"), "select 1").unwrap();

        // The default, made below the target folder.
        assert_eq!(site_dir(&root, None), Ok(root.join("target").join("edith-site")));
        assert_eq!(site_dir(&root, Some(&root.join("docs_site"))), Ok(root.join("docs_site")));

        let refused = |given: &Path| site_dir(&root, Some(given)).unwrap_err();
        assert!(refused(&root).contains("not inside the project"), "the root itself");
        assert!(refused(&root.join("target").join("..").join("site")).contains(".."), "a .. in the path");
        assert!(refused(&root.join("..").join("elsewhere")).contains(".."));
        let outside = scratch("folder-outside");
        assert!(refused(&outside).contains("not inside the project"), "an absolute path elsewhere");
        assert!(refused(&root.join(".git").join("site")).contains(".git"));
        assert!(refused(&root.join(".GIT").join("x")).contains(".git"), "whatever the case");
        // Not empty, not marked: left alone.
        assert!(refused(&root.join("models")).contains("not empty"));
        assert!(refused(&root.join("models").join("a.sql")).contains("not a folder"));
        // A file at the place.
        std::fs::write(root.join("page.html"), "x").unwrap();
        assert!(refused(&root.join("page.html")).contains("not a folder"));

        // Marked: accepted, with whatever it holds.
        std::fs::create_dir_all(root.join("old_site")).unwrap();
        std::fs::write(root.join("old_site").join(MARKER), MARKER_TEXT).unwrap();
        std::fs::write(root.join("old_site").join("index.html"), "old").unwrap();
        assert_eq!(site_dir(&root, Some(&root.join("old_site"))), Ok(root.join("old_site")));
        // Empty: accepted.
        std::fs::create_dir_all(root.join("fresh")).unwrap();
        assert_eq!(site_dir(&root, Some(&root.join("fresh"))), Ok(root.join("fresh")));
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&outside);
    }

    #[cfg(unix)]
    #[test]
    fn a_link_leading_out_of_the_project_is_not_followed() {
        let root = scratch("folder-link");
        let outside = scratch("folder-link-outside");
        std::os::unix::fs::symlink(&outside, root.join("site")).unwrap();
        let why = site_dir(&root, Some(&root.join("site"))).unwrap_err();
        assert!(why.contains("is a link"), "{why}");
        // A link on the way: the folder itself is not one, but where it is is outside.
        std::os::unix::fs::symlink(&outside, root.join("out")).unwrap();
        let why = site_dir(&root, Some(&root.join("out").join("site"))).unwrap_err();
        assert!(why.contains("not inside the project"), "{why}");
        // A dangling link is no folder either.
        std::os::unix::fs::symlink(root.join("nothing"), root.join("dangling")).unwrap();
        assert!(site_dir(&root, Some(&root.join("dangling").join("site"))).is_err());
        // Nothing was written where it leads.
        assert_eq!(std::fs::read_dir(&outside).unwrap().count(), 0);
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&outside);
    }

    #[cfg(unix)]
    #[test]
    fn a_link_inside_the_project_to_its_git_folder_is_not_a_way_in() {
        let root = scratch("folder-git");
        std::fs::create_dir_all(root.join(".git")).unwrap();
        std::os::unix::fs::symlink(root.join(".git"), root.join("docs_link")).unwrap();
        let why = site_dir(&root, Some(&root.join("docs_link").join("x"))).unwrap_err();
        assert!(why.contains(".git"), "{why}");
        // A folder that is a link is not emptied either, whatever was judged before.
        let outside = scratch("folder-git-outside");
        std::fs::write(outside.join("keep"), "x").unwrap();
        let held = root.join("held");
        std::os::unix::fs::symlink(&outside, &held).unwrap();
        assert!(clear(&held).is_err());
        assert!(outside.join("keep").exists() && !outside.join(MARKER).exists());
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&outside);
    }

    #[test]
    fn a_marked_folder_is_emptied_and_the_marker_stays_first() {
        let dir = scratch("clear");
        std::fs::write(dir.join(MARKER), "old marker").unwrap();
        std::fs::create_dir_all(dir.join("data/cards")).unwrap();
        std::fs::write(dir.join("data/cards/0099.js"), "stale").unwrap();
        std::fs::write(dir.join("index.html"), "stale").unwrap();
        clear(&dir).unwrap();
        let left: Vec<String> = std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
        assert_eq!(left, vec![MARKER.to_string()]);
        assert_eq!(std::fs::read_to_string(dir.join(MARKER)).unwrap(), MARKER_TEXT);
        let _ = std::fs::remove_dir_all(&dir);
    }

    // --------------------------------------------------------------- page ----

    #[test]
    fn the_page_is_the_embedded_one_with_four_anchors_replaced_once() {
        let embedded = Assets::get("index.html").expect("the page is embedded");
        let source = String::from_utf8_lossy(&embedded.data).into_owned();
        let html = page(&source, "shop \u{b7} prod \u{b7} dbt-edith").unwrap();
        assert!(html.contains("<html lang=\"en\" data-theme=\"night\" data-site>"));
        assert_eq!(html.matches("Content-Security-Policy").count(), 1);
        assert!(html.contains(&format!("content=\"{CSP}\"")));
        assert!(html.contains("<title>shop \u{b7} prod \u{b7} dbt-edith</title>"));
        let order: Vec<usize> = ["data/index.js", "site.js", "lineage.js", "app.js"]
            .iter()
            .map(|s| html.find(&format!("<script src=\"{s}\">")).unwrap())
            .collect();
        assert!(order.windows(2).all(|w| w[0] < w[1]), "the data and site.js load before the canvas and the page: {order:?}");
        // Nothing inline runs: every script has a source, and no handler.
        assert_eq!(html.matches("<script").count(), html.matches("<script src=").count());
        assert!(!html.contains(" onclick=") && !html.contains("javascript:"));
        // The title is text, not markup.
        assert!(page(&source, "<b>x</b> & \"y\"").unwrap().contains("<title>&lt;b&gt;x&lt;/b&gt; &amp; &quot;y&quot;</title>"));
    }

    #[test]
    fn an_anchor_that_is_missing_or_repeated_fails_the_run() {
        let embedded = Assets::get("index.html").unwrap();
        let source = String::from_utf8_lossy(&embedded.data).into_owned();
        let without = source.replacen("<title>dbt-edith</title>", "", 1);
        assert!(page(&without, "t").unwrap_err().contains("<title>dbt-edith</title>"));
        let twice = source.replacen("</head>", "<title>dbt-edith</title></head>", 1);
        assert!(page(&twice, "t").unwrap_err().contains("has 2 of"));
    }

    #[test]
    fn the_assets_copied_are_the_files_the_page_loads() {
        let project = scratch("assets");
        write_project(&project);
        let into = project.join("out");
        let ask = Ask {
            dir: Some(into.clone()),
            environment: None,
            commit: None,
            repo_url: None,
            produce: None,
            jobs_commit: None,
            jobs_repo_url: None,
        };
        let (mut out, mut err) = (Vec::new(), Vec::new());
        assert_eq!(generate(&project, None, None, &ask, &mut out, &mut err), ls::ANSWERED, "{}", String::from_utf8_lossy(&err));

        let written = files_of(&into);
        let html = String::from_utf8(written["index.html"].clone()).unwrap();
        let loaded: BTreeSet<String> = loaded_by(&html).into_iter().collect();
        // Everything the page loads is in the folder, and the folder holds
        // nothing a page does not load but its data, its notices and its marker.
        for path in &loaded {
            assert!(written.contains_key(path), "{path} is loaded and was not written");
        }
        let extra: BTreeSet<&String> =
            written.keys().filter(|p| !loaded.contains(*p) && !p.starts_with("data/cards/") && !p.starts_with("data/sql/")).collect();
        let want: BTreeSet<String> = ["index.html", MARKER, "LICENSE.txt", "THIRD_PARTY_NOTICES.md"].map(String::from).into();
        assert_eq!(extra.into_iter().cloned().collect::<BTreeSet<_>>(), want);
        assert!(loaded.contains("data/index.js") && loaded.contains("site.js") && loaded.contains("vendor/codemirror.js"));
        let _ = std::fs::remove_dir_all(&project);
    }

    // --------------------------------------------------------------- data ----

    #[test]
    fn the_json_is_safe_to_put_in_a_script() {
        let text = js_json(&json!({ "a": "x</script><!--\u{2028}\u{2029}y", "b": "caf\u{e9} \u{1f600}" }));
        assert!(!text.contains('<') && !text.contains('\u{2028}') && !text.contains('\u{2029}'), "{text}");
        let back: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(back["a"], "x</script><!--\u{2028}\u{2029}y");
        assert_eq!(back["b"], "caf\u{e9} \u{1f600}");
    }

    // ------------------------------------------------- the whole, end to end ----

    /// A project with an `.env` that must never be read, and a manifest of
    /// invented models whose database and schema are written as `env_var()`
    /// calls: written as they are, never resolved.
    pub(super) fn write_project(root: &Path) {
        std::fs::create_dir_all(root.join("target")).unwrap();
        std::fs::write(root.join("dbt_project.yml"), "name: shop\nversion: '1.0'\nprofile: shop\n").unwrap();
        std::fs::write(root.join(".env"), "DBT_DATABASE=hunter2-canary\nDBT_SCHEMA=hunter2-canary\n").unwrap();
        let model = |name: &str, parents: &[&str]| {
            json!({
                "name": name, "resource_type": "model", "package_name": "shop",
                "original_file_path": format!("models/{name}.sql"),
                "patch_path": format!("shop://models/{name}.yml"),
                "database": "analytics", "schema": "marts",
                "unrendered_config": { "database": "{{ env_var('DBT_DATABASE') }}", "schema": "{{ env_var('DBT_SCHEMA') }}" },
                "config": { "materialized": "table" },
                "description": "A model.",
                "raw_code": format!("select 1 as id -- {name}"),
                "language": "sql",
                "columns": { "id": { "name": "id", "description": "The key.", "data_type": "int" } },
                "depends_on": { "nodes": parents.iter().map(|p| format!("model.shop.{p}")).collect::<Vec<_>>() },
            })
        };
        let manifest = json!({
            "metadata": { "project_name": "shop", "dbt_version": "1.9.0", "generated_at": "2030-01-02T03:04:05Z", "adapter_type": "duckdb" },
            "nodes": {
                "model.shop.raw_orders": model("raw_orders", &[]),
                "model.shop.stg_orders": model("stg_orders", &["raw_orders"]),
                "model.shop.orders": model("orders", &["stg_orders"]),
            },
            "parent_map": {
                "model.shop.raw_orders": [],
                "model.shop.stg_orders": ["model.shop.raw_orders"],
                "model.shop.orders": ["model.shop.stg_orders"],
            },
        });
        std::fs::write(root.join("target/manifest.json"), serde_json::to_vec(&manifest).unwrap()).unwrap();
    }

    pub(super) fn files_of(dir: &Path) -> BTreeMap<String, Vec<u8>> {
        fn walk(base: &Path, at: &Path, into: &mut BTreeMap<String, Vec<u8>>) {
            for entry in std::fs::read_dir(at).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    walk(base, &path, into);
                } else {
                    let rel = path.strip_prefix(base).unwrap().to_string_lossy().replace('\\', "/");
                    into.insert(rel, std::fs::read(&path).unwrap());
                }
            }
        }
        let mut into = BTreeMap::new();
        walk(dir, dir, &mut into);
        into
    }

    #[test]
    fn the_site_is_the_same_bytes_twice_and_holds_no_path_and_no_secret() {
        let project = scratch("whole");
        write_project(&project);
        let ask = Ask {
            dir: None,
            environment: Some("prod".to_string()),
            commit: Some("0123456789abcdef0123456789abcdef01234567".to_string()),
            repo_url: Some("https://github.com/acme/shop".to_string()),
            produce: None,
            jobs_commit: None,
            jobs_repo_url: None,
        };
        let run = |ask: &Ask| {
            let (mut out, mut err) = (Vec::new(), Vec::new());
            let code = generate(&project, None, None, ask, &mut out, &mut err);
            (code, String::from_utf8(out).unwrap(), String::from_utf8(err).unwrap())
        };
        let (code, out, err) = run(&ask);
        assert_eq!(code, ls::ANSWERED, "{err}");
        let dir = project.join("target").join("edith-site");
        assert!(out.starts_with(&dir.display().to_string()), "{out}");
        assert!(err.contains("no catalog.json"), "{err}");
        assert!(err.contains("no compiled SQL in the manifest: run --site after dbt build, compile or docs generate"), "{err}");
        assert!(err.contains("no file links: not a git repository"), "{err}");
        let first = files_of(&dir);
        assert!(first.contains_key(MARKER) && first.contains_key("data/index.js") && first.contains_key("data/cards/0000.js"));
        assert!(first.contains_key("data/sql/0000.js"));
        // The SQL is in a script that no name in it can close, and no absolute path is in it.
        let sql = String::from_utf8_lossy(&first["data/sql/0000.js"]).into_owned();
        assert!(sql.starts_with("EDITH_SITE_SQL(0,[{\"raw\":\"select 1 as id -- orders\"}"), "{sql}");
        assert!(!first.contains_key(".env"));

        // A stale file of an earlier run is gone, and the bytes are the same.
        std::fs::write(dir.join("data/cards/0042.js"), "stale").unwrap();
        let (code, _, err) = run(&ask);
        assert_eq!(code, ls::ANSWERED, "{err}");
        assert_eq!(files_of(&dir), first);

        // No absolute path of this machine, no value of the `.env`, no mtime.
        let temp = std::env::temp_dir().display().to_string();
        let home = std::env::var("HOME").unwrap_or_default();
        for (name, bytes) in &first {
            if name.starts_with("vendor/") {
                continue;
            }
            let text = String::from_utf8_lossy(bytes);
            for needle in ["hunter2-canary", project.to_string_lossy().as_ref(), temp.as_str()] {
                assert!(!text.contains(needle), "{name} holds {needle}");
            }
            if !home.is_empty() && home != "/" {
                assert!(!text.contains(&home), "{name} holds the home folder");
            }
        }
        // The written location is shown as written, never resolved.
        let cards = String::from_utf8_lossy(&first["data/cards/0000.js"]).into_owned();
        assert!(cards.contains("env_var('DBT_DATABASE')"));

        // The header's facts.
        let index = String::from_utf8_lossy(&first["data/index.js"]).into_owned();
        let json: Value = serde_json::from_str(index.trim_end().strip_prefix("window.EDITH_SITE=").unwrap().trim_end_matches(';')).unwrap();
        let meta = &json["meta"];
        assert_eq!(meta["environment"], "prod");
        assert_eq!(meta["commit_url"], "https://github.com/acme/shop/commit/0123456789abcdef0123456789abcdef01234567");
        // Not in a repository: the commit is linked, no file is.
        assert_eq!(meta["file_base"], Value::Null);
        assert_eq!(json["sql_cuts"], json!([0]));
        assert_eq!(meta["generated_at"], "2030-01-02T03:04:05Z");
        assert_eq!(meta["dbt_version"], "1.9.0");
        assert_eq!(meta["catalog"], false);
        // The tree: the folder of the models, and the three files built from, no more.
        assert_eq!(json["tree"][""], json!([["models", -1]]));
        assert_eq!(json["tree"]["models"].as_array().unwrap().len(), 3);
        assert!(first["index.html"]
            .windows(b"<title>shop \xc2\xb7 prod \xc2\xb7 dbt-edith</title>".len())
            .any(|w| w == b"<title>shop \xc2\xb7 prod \xc2\xb7 dbt-edith</title>"));
        let _ = std::fs::remove_dir_all(&project);
    }

    #[test]
    fn a_bad_flag_a_missing_manifest_or_a_refused_folder_exits_with_two_and_writes_nothing() {
        let project = scratch("refuse");
        let ask = |dir: Option<PathBuf>, commit: Option<&str>, repo: Option<&str>| Ask {
            dir,
            environment: None,
            commit: commit.map(String::from),
            repo_url: repo.map(String::from),
            produce: None,
            jobs_commit: None,
            jobs_repo_url: None,
        };
        let go = |ask: &Ask| {
            let (mut out, mut err) = (Vec::new(), Vec::new());
            (generate(&project, None, None, ask, &mut out, &mut err), String::from_utf8(err).unwrap())
        };
        let (code, why) = go(&ask(None, None, None));
        assert_eq!((code, why.contains("no manifest")), (2, true), "{why}");
        write_project(&project);
        let (code, why) = go(&ask(None, Some("zzz"), None));
        assert_eq!((code, why.contains("--commit")), (2, true), "{why}");
        let (code, why) = go(&ask(None, None, Some("http://github.com/a/b")));
        assert_eq!((code, why.contains("--repo-url")), (2, true), "{why}");
        let (code, why) = go(&ask(Some(project.clone()), None, None));
        assert_eq!((code, why.contains("not inside the project")), (2, true), "{why}");
        let (code, why) = go(&ask(Some(project.join("target")), None, None));
        assert_eq!((code, why.contains("not empty")), (2, true), "{why}");
        assert!(project.join("target/manifest.json").is_file(), "a folder refused is a folder untouched");
        assert!(!project.join("target/edith-site").exists());
        let _ = std::fs::remove_dir_all(&project);
    }

    // ---------------------------------------------------- files and the SQL ----

    #[test]
    fn a_file_is_linked_by_its_hosts_pattern_below_the_projects_place() {
        let sha = "0123456789abcdef0123456789abcdef01234567";
        let base = |url: &str, prefix: &str| file_base(&repo_from_flag(url).unwrap(), sha, prefix);
        assert_eq!(base("https://github.com/acme/shop", "").as_deref(), Some(format!("https://github.com/acme/shop/blob/{sha}/").as_str()));
        assert_eq!(
            base("https://github.com/acme/shop", "analytics/dbt/").as_deref(),
            Some(format!("https://github.com/acme/shop/blob/{sha}/analytics/dbt/").as_str())
        );
        assert_eq!(
            base("https://gitlab.com/acme/data/shop", "dbt/").as_deref(),
            Some(format!("https://gitlab.com/acme/data/shop/-/blob/{sha}/dbt/").as_str())
        );
        assert_eq!(
            base("https://bitbucket.org/acme/shop", "dbt/").as_deref(),
            Some(format!("https://bitbucket.org/acme/shop/src/{sha}/dbt/").as_str())
        );
        assert_eq!(base("https://git.example.com/shop", "").as_deref(), Some(format!("https://git.example.com/shop/blob/{sha}/").as_str()));
        // A folder's name stays in its segment, whatever it holds.
        assert_eq!(
            base("https://github.com/acme/shop", "my project/a#b/").as_deref(),
            Some(format!("https://github.com/acme/shop/blob/{sha}/my%20project/a%23b/").as_str())
        );
        // Azure's file address takes a query: its commit is linked, its files are not.
        for azure in ["https://dev.azure.com/acme/shop/_git/shop", "https://acme.visualstudio.com/shop/_git/shop"] {
            let repo = repo_from_flag(azure).unwrap();
            assert_eq!(repo.host, Host::Azure, "{azure}");
            assert_eq!(file_base(&repo, sha, ""), None, "{azure}");
            assert_eq!(commit_link(&repo, sha), format!("{}/commit/{sha}", repo.url));
        }
    }

    /// Writes the site for `project` and reads back its index, with what stderr said.
    fn written(project: &Path, ask: &Ask) -> (Value, String) {
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = generate(project, None, None, ask, &mut out, &mut err);
        let err = String::from_utf8(err).unwrap();
        assert_eq!(code, ls::ANSWERED, "{err}");
        let dir = ask.dir.clone().unwrap_or_else(|| project.join("target").join("edith-site"));
        let index = std::fs::read_to_string(dir.join("data/index.js")).unwrap();
        (serde_json::from_str(index.trim_end().strip_prefix("window.EDITH_SITE=").unwrap().trim_end_matches(';')).unwrap(), err)
    }

    fn ask_for(repo: &str) -> Ask {
        Ask {
            dir: None,
            environment: None,
            commit: Some("0123456789abcdef0123456789abcdef01234567".to_string()),
            repo_url: Some(repo.to_string()),
            produce: None,
            jobs_commit: None,
            jobs_repo_url: None,
        }
    }

    fn init_repository(dir: &Path) {
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(["init", "-q", "-b", "main"])
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .output()
            .unwrap();
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    }

    #[test]
    fn a_project_below_the_repositorys_root_links_its_files_with_the_prefix() {
        let root = scratch("links-prefix");
        init_repository(&root);
        let project = root.join("analytics");
        write_project(&project);
        let (json, err) = written(&project, &ask_for("https://github.com/acme/shop"));
        assert_eq!(json["meta"]["file_base"], "https://github.com/acme/shop/blob/0123456789abcdef0123456789abcdef01234567/analytics/");
        assert!(!err.contains("no file links"), "{err}");

        // At the root of the repository the prefix is empty.
        let flat = scratch("links-flat");
        init_repository(&flat);
        write_project(&flat);
        let (json, _) = written(&flat, &ask_for("https://gitlab.com/acme/shop"));
        assert_eq!(json["meta"]["file_base"], "https://gitlab.com/acme/shop/-/blob/0123456789abcdef0123456789abcdef01234567/");
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&flat);
    }

    #[test]
    fn no_repository_gives_no_file_links_but_keeps_the_commit_link() {
        let project = scratch("links-none");
        write_project(&project);
        let (json, err) = written(&project, &ask_for("https://github.com/acme/shop"));
        assert_eq!(json["meta"]["file_base"], Value::Null);
        assert_eq!(json["meta"]["commit_url"], "https://github.com/acme/shop/commit/0123456789abcdef0123456789abcdef01234567");
        assert!(err.contains("no file links: not a git repository"), "{err}");
        let _ = std::fs::remove_dir_all(&project);

        // Azure is a repository that is read, and still has no file address to write.
        let azure = scratch("links-azure");
        init_repository(&azure);
        write_project(&azure);
        let (json, err) = written(&azure, &ask_for("https://dev.azure.com/acme/shop/_git/shop"));
        assert_eq!(json["meta"]["file_base"], Value::Null);
        assert!(json["meta"]["commit_url"].as_str().unwrap().ends_with("/commit/0123456789abcdef0123456789abcdef01234567"));
        assert!(err.contains("no file links: the file addresses of Azure DevOps"), "{err}");
        let _ = std::fs::remove_dir_all(&azure);
    }

    #[test]
    fn a_paused_repository_gives_no_file_links() {
        let project = scratch("links-paused");
        init_repository(&project);
        let config = std::process::Command::new("git")
            .arg("-C")
            .arg(&project)
            .args(["config", "filter.evil.clean", "cat"])
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .output()
            .unwrap();
        assert!(config.status.success());
        write_project(&project);
        let (json, err) = written(&project, &ask_for("https://github.com/acme/shop"));
        assert_eq!(json["meta"]["file_base"], Value::Null);
        assert!(err.contains("no file links: git did not say where the project sits"), "{err}");
        let _ = std::fs::remove_dir_all(&project);
    }

    /// A manifest with no `metadata.project_name`, which `dbt_project.yml` then names: the root package is the
    /// one the tree already takes, and the page's links go by it, not by the manifest's blank.
    #[test]
    fn the_root_is_dbt_projects_name_when_the_manifest_has_none() {
        let repo = scratch("root-name");
        init_repository(&repo);
        let project = repo.join("analytics");
        write_project(&project);
        let manifest = project.join("target/manifest.json");
        let mut value: Value = serde_json::from_slice(&std::fs::read(&manifest).unwrap()).unwrap();
        value["metadata"].as_object_mut().unwrap().remove("project_name");
        let mut package = value["nodes"]["model.shop.orders"].clone();
        package["name"] = json!("accounts");
        package["package_name"] = json!("crm");
        package["original_file_path"] = json!("models/accounts.sql");
        value["nodes"]["model.crm.accounts"] = package;
        std::fs::write(&manifest, serde_json::to_vec(&value).unwrap()).unwrap();

        let (json, _) = written(&project, &ask_for("https://github.com/acme/shop"));
        assert_eq!(json["meta"]["project"], "", "the manifest names none");
        assert_eq!(json["meta"]["root"], "shop", "dbt_project.yml does");
        assert_eq!(json["meta"]["file_base"], "https://github.com/acme/shop/blob/0123456789abcdef0123456789abcdef01234567/analytics/");
        // The tree agrees: the project's own files at their paths, the package's where `dbt deps` puts it.
        let models: Vec<&str> = json["tree"]["models"].as_array().unwrap().iter().map(|e| e[0].as_str().unwrap()).collect();
        assert_eq!(models, ["orders.sql", "raw_orders.sql", "stg_orders.sql"]);
        assert!(json["tree"].get("dbt_packages/crm/models").is_some(), "{}", json["tree"]);
        let _ = std::fs::remove_dir_all(&repo);
    }

    /// An invented manifest with one node of each shape the SQL can take.
    fn code_manifest() -> Value {
        let node = |name: &str, kind: &str, extra: Value| {
            let mut v =
                json!({ "name": name, "resource_type": kind, "package_name": "shop", "original_file_path": format!("models/{name}.sql") });
            for (k, x) in extra.as_object().unwrap() {
                v[k] = x.clone();
            }
            v
        };
        json!({
            "metadata": { "project_name": "shop", "dbt_version": "1.9.0", "generated_at": "2030-01-02T03:04:05Z", "adapter_type": "duckdb" },
            "nodes": {
                "model.shop.a": node("a", "model", json!({ "raw_code": "select 1\r\nfrom x", "compiled_code": "select 1\r\nfrom db.x", "language": "sql" })),
                "model.shop.b": node("b", "model", json!({ "raw_code": "select '</script><!--\u{2028}\u{2029}' as x" })),
                "model.shop.py": node("py", "model", json!({
                    "raw_code": "def model(dbt, session):\n    return dbt.ref('a')", "compiled_code": "def model(dbt, session):\n    return dbt.ref('a')", "language": "python" })),
                "model.shop.empty": node("empty", "model", json!({ "raw_code": "" })),
                "model.shop.only_compiled": node("only_compiled", "model", json!({ "compiled_code": "select 2" })),
                "model.shop.in_nodes_off": node("in_nodes_off", "model", json!({ "raw_code": "select 'nodes'", "config": { "enabled": false } })),
                "seed.shop.countries": node("countries", "seed", json!({ "raw_code": "" })),
                "snapshot.shop.s": node("s", "snapshot", json!({ "raw_code": "select * from {{ source('a', 'b') }}", "compiled_code": "select * from a.b" })),
                "analysis.shop.q": node("q", "analysis", json!({ "raw_code": "select 3", "compiled_code": "select 3" })),
                "operation.shop.shop-on-run-end-0": node("shop-on-run-end-0", "operation", json!({ "raw_code": "grant select", "compiled_code": "grant select" })),
                // A generic test: the macro call is the raw code, the SQL it makes the compiled one.
                "test.shop.unique_a_id": node("unique_a_id", "test", json!({
                    "raw_code": "{{ test_unique(**_dbt_generic_test_kwargs) }}", "compiled_code": "select id from db.a group by id having count(*) > 1" })),
            },
            "sources": { "source.shop.raw.orders": node("orders", "source", json!({ "source_name": "raw", "raw_code": "not code" })) },
            "disabled": {
                "model.shop.off": [
                    node("off", "model", json!({ "raw_code": "select 'first'", "config": { "enabled": false } })),
                    node("off", "model", json!({ "raw_code": "select 'second'", "config": { "enabled": false } })),
                ],
                "model.shop.in_nodes_off": [node("in_nodes_off", "model", json!({ "raw_code": "select 'disabled'", "config": { "enabled": false } }))],
                "test.shop.off_test": [node("off_test", "test", json!({ "raw_code": "select 4", "config": { "enabled": false } }))],
            },
        })
    }

    fn code_graph(manifest: &Value) -> Graph {
        let raw: crate::manifest::RawManifest = serde_json::from_value(manifest.clone()).unwrap();
        Graph::build(raw, Path::new("manifest.json"), 0, 0)
    }

    #[test]
    fn the_code_is_the_manifests_by_the_graphs_numbering() {
        let manifest = code_manifest();
        let graph = code_graph(&manifest);
        let code = sql_of(&graph, serde_json::from_value(manifest).unwrap());
        assert_eq!(code.len(), graph.nodes.len());
        let of = |id: &str| code[graph.index[id] as usize].as_ref();
        let sql = |raw: &str, compiled: Option<&str>, python: bool| Sql { raw: raw.into(), compiled: compiled.map(String::from), python };

        // A line ending is one.
        assert_eq!(of("model.shop.a"), Some(&sql("select 1\nfrom x", Some("select 1\nfrom db.x"), false)));
        // No compiled code: the page says so, the site never reads target/compiled.
        assert_eq!(of("model.shop.b").map(|s| s.compiled.clone()), Some(None));
        assert_eq!(of("model.shop.py").map(|s| s.python), Some(true));
        assert_eq!(of("model.shop.only_compiled"), Some(&sql("", Some("select 2"), false)));
        assert_eq!(of("snapshot.shop.s").map(|s| s.raw.as_str()), Some("select * from {{ source('a', 'b') }}"));
        assert!(of("analysis.shop.q").is_some() && of("operation.shop.shop-on-run-end-0").is_some());
        // A generic test's raw code is its macro call, shown as it is.
        assert_eq!(of("test.shop.unique_a_id").map(|s| s.raw.as_str()), Some("{{ test_unique(**_dbt_generic_test_kwargs) }}"));
        // Nothing to show: an empty raw_code, a seed, a source.
        assert_eq!(of("model.shop.empty"), None);
        assert_eq!(of("seed.shop.countries"), None);
        assert_eq!(of("source.shop.raw.orders"), None);
        // A disabled node is numbered from the first entry of `disabled`, as the graph takes it; one
        // `nodes` left out because it is disabled never shows its code.
        assert_eq!(of("model.shop.off").map(|s| s.raw.as_str()), Some("select 'first'"));
        assert_eq!(of("model.shop.in_nodes_off").map(|s| s.raw.as_str()), Some("select 'disabled'"));
        assert!(!graph.index.contains_key("test.shop.off_test"), "a disabled test is not in the graph, so it has no code");
    }

    #[test]
    fn the_sql_is_safe_in_a_script_and_cut_by_its_own_rule() {
        let manifest = code_manifest();
        let graph = code_graph(&manifest);
        let code = sql_of(&graph, serde_json::from_value(manifest).unwrap());
        let (cuts, buckets) = sql_in(code.clone(), 4, BUCKET_BYTES).unwrap();
        assert_eq!(cuts, (0..graph.nodes.len()).step_by(4).collect::<Vec<_>>());
        let all = buckets.join("");
        assert!(!all.contains('<') && !all.contains('\u{2028}') && !all.contains('\u{2029}'), "{all}");
        // Read back, the entry is what was written, whatever it held.
        let at = graph.index["model.shop.b"] as usize;
        let bucket = &buckets[at / 4];
        let list = bucket.strip_prefix(&format!("EDITH_SITE_SQL({},[", at / 4 * 4)).unwrap().strip_suffix("]);\n").unwrap();
        let entries: Vec<Value> = serde_json::from_str(&format!("[{list}]")).unwrap();
        assert_eq!(entries[at % 4], json!({ "raw": "select '</script><!--\u{2028}\u{2029}' as x" }));
        // A node with nothing to show is a 0, a Python model says so, and a missing compiled is absent.
        let py = graph.index["model.shop.py"] as usize;
        assert_eq!(sql_entry(code[py].clone())["lang"], "python");
        assert_eq!(sql_entry(code[graph.index["seed.shop.countries"] as usize].clone()), json!(0));
        assert!(sql_entry(code[at].clone()).get("compiled").is_none());
        // The byte limit closes a bucket as the count does.
        let (cuts, _) = sql_in(code, 1000, 1).unwrap();
        assert_eq!(cuts.len(), graph.nodes.len());
    }

    #[test]
    fn the_site_says_when_no_node_was_compiled_and_is_quiet_when_one_was() {
        let project = scratch("compiled-line");
        write_project(&project);
        let manifest = project.join("target/manifest.json");
        let (_, err) = written(&project, &ask_for("https://github.com/acme/shop"));
        assert!(err.contains("no compiled SQL in the manifest"), "{err}");

        let mut value: Value = serde_json::from_slice(&std::fs::read(&manifest).unwrap()).unwrap();
        value["nodes"]["model.shop.orders"]["compiled_code"] = json!("select 1 as id from analytics.marts.stg_orders");
        std::fs::write(&manifest, serde_json::to_vec(&value).unwrap()).unwrap();
        let (_, err) = written(&project, &ask_for("https://github.com/acme/shop"));
        assert!(!err.contains("no compiled SQL"), "{err}");
        let sql = std::fs::read_to_string(project.join("target/edith-site/data/sql/0000.js")).unwrap();
        assert!(sql.contains("\"compiled\":\"select 1 as id from analytics.marts.stg_orders\""), "{sql}");
        let _ = std::fs::remove_dir_all(&project);
    }

    // ------------------------------------------------------ column lineage ----

    /// The invented cache the fixture is held to: every shape `ColLineage` reads, over the case graph.
    /// Roles with an empty name, in capitals and outside ASCII; filters, join keys and dedup keys into
    /// every column of a model and out of one column into many; a stop reached two ways; a chain
    /// longer than the depth menu lists; a cycle and a self-edge; columns spelled in two cases and one
    /// the YAML does not declare; edges that name a node the manifest lacks; roots, `[]` among them;
    /// expressions at 32 steps and cut; gaps in every reason; and more rules than a card names.
    fn case_cache() -> crate::collin::RawColLineage {
        let m = |n: &str| format!("model.shop.{n}");
        let edge = |from: &str, fc: &str, to: &str, tc: &str, kind: &str| json!({ "from": from, "from_col": fc, "to": to, "to_col": tc, "kind": kind });
        let mut edges: Vec<Value> = Vec::new();
        // A chain of one column, 22 levels.
        for i in 2..=22 {
            edges.push(edge(&m(&format!("chain_{:02}", i - 1)), "k", &m(&format!("chain_{i:02}")), "k", "passthrough"));
        }
        // A table no node owns, then the source, the seed and a diamond of columns, a stop reached two ways.
        edges.push(edge("rel:analytics.raw.stage", "id", "source.shop.raw.orders", "id", "passthrough"));
        edges.push(edge("rel:analytics.raw.stage", "id", &m("dia_c"), "id", "filter"));
        edges.push(edge("source.shop.raw.orders", "id", &m("chain_01"), "k", "rename"));
        edges.push(edge("source.shop.raw.orders", "id", &m("dia_a"), "id", "rename"));
        edges.push(edge("seed.shop.countries", "id", &m("dia_a"), "id", "passthrough"));
        edges.push(edge(&m("dia_a"), "id", &m("dia_b"), "id", "window"));
        edges.push(edge(&m("dia_a"), "id", &m("dia_c"), "id", "passthrough"));
        edges.push(edge(&m("dia_c"), "id", &m("dia_b"), "id", "passthrough"));
        edges.push(edge(&m("dia_b"), "id", &m("dia_d"), "id", "aggregate"));
        edges.push(edge(&m("dia_c"), "id", &m("dia_d"), "id", "transform"));
        edges.push(edge(&m("dia_c"), "id", &m("dia_d"), "Amount", ""));
        edges.push(edge(&m("dia_d"), "id", &m("hub"), "id", "passthrough"));
        // A column the YAML does not declare, in two spellings, and one that is not ASCII.
        edges.push(edge(&m("dia_a"), "Amount", &m("dia_b"), "amount", "cast"));
        edges.push(edge(&m("dia_b"), "AMOUNT", &m("dia_d"), "Total_\u{c4}", "aggregate"));
        edges.push(edge(&m("dia_d"), "total_\u{e4}", &m("hub"), "metric", "r\u{e9}\u{e9}crit"));
        // A model reading itself, and a column reading itself.
        edges.push(edge(&m("dia_b"), "id", &m("dia_b"), "amount", "filter"));
        edges.push(edge(&m("fork"), "s", &m("fork"), "s", "passthrough"));
        // A cycle between two models.
        edges.push(edge(&m("fork_a"), "c1", &m("fork_b"), "c1", "passthrough"));
        edges.push(edge(&m("fork_b"), "c1", &m("fork_a"), "c1", "transform"));
        edges.push(edge(&m("dia_d"), "id", &m("fork_a"), "c1", "rename"));
        // Indirect edges from the seed's key to many models, a role by capital letters, one by its strongest
        // of several, and ten columns that keep the rows of one.
        let roles = ["filter", "join_key", "dedup_key", "FILTER"];
        for i in 1..=10 {
            edges.push(edge("seed.shop.countries", "id", &m(&format!("chain_{i:02}")), "j", roles[i % 4]));
            edges.push(edge(&m(&format!("chain_{i:02}")), "k", &m("hub"), "metric", roles[(i + 1) % 4]));
        }
        edges.push(edge("seed.shop.countries", "id", &m("chain_02"), "k", "dedup_key"));
        edges.push(edge("seed.shop.countries", "id", &m("dia_b"), "id", "join_key"));
        edges.push(edge("seed.shop.countries", "id", &m("dia_b"), "amount", "join_key"));
        edges.push(edge(&m("dia_a"), "id", &m("dia_d"), "id", "filter"));
        // A stop that comes later in a column's edges than a column the cap turns away: taken back whatever the cap.
        edges.push(edge(&m("chain_12"), "m", &m("chain_13"), "m", "passthrough"));
        edges.push(edge(&m("chain_12"), "m", &m("chain_20"), "m", "window"));
        edges.push(edge(&m("chain_13"), "m", &m("chain_14"), "m", "passthrough"));
        edges.push(edge(&m("chain_13"), "m", &m("chain_20"), "m", "passthrough"));
        // A stop that a column the walk only draws also reads, by another role: its status line names only what stopped it.
        edges.push(edge(&m("chain_15"), "q", &m("chain_16"), "q", "passthrough"));
        edges.push(edge(&m("chain_15"), "q", &m("chain_21"), "q", "window"));
        edges.push(edge(&m("chain_16"), "q", &m("chain_17"), "q", "passthrough"));
        edges.push(edge(&m("chain_17"), "q", &m("chain_21"), "q", "aggregate"));
        // Two roles of the same strength into one model: the first by column is the one its box says.
        edges.push(edge("seed.shop.countries", "id", &m("chain_11"), "j", "filter"));
        edges.push(edge("seed.shop.countries", "id", &m("chain_11"), "k", "FILTER"));
        // Two models of one name keeping the rows of another's column by one role: named once, as the card says them; and a model
        // that is first by its id and not by its name, whose rule is read after another's.
        edges.push(edge("model.shop.customers", "id", &m("hub"), "metric", "filter"));
        edges.push(edge("model.crm.customers", "id", &m("hub"), "metric", "filter"));
        edges.push(edge("seed.shop.countries", "id", "model.crm.customers", "id", "filter"));
        // A model kept the rows of from columns at two levels: its box is drawn past the deepest of them.
        edges.push(edge(&m("chain_18"), "p", &m("chain_19"), "p", "passthrough"));
        edges.push(edge(&m("chain_19"), "p", &m("chain_20"), "p", "passthrough"));
        edges.push(edge(&m("chain_18"), "p", &m("leaf_01"), "r", "filter"));
        edges.push(edge(&m("chain_20"), "p", &m("leaf_01"), "r", "join_key"));
        // Edges that name a node the manifest lacks.
        edges.push(edge(&m("ghost"), "id", &m("dia_a"), "id", "passthrough"));
        edges.push(edge(&m("dia_a"), "id", &m("ghost"), "id", "passthrough"));

        let rule = |node: &str, clause: &str, text: &str, cut: bool, reads: &[(&str, &str)]| {
            json!({ "node": node, "clause": clause, "text": text, "cut": cut,
                    "columns": reads.iter().map(|(n, c)| json!({ "node": n, "column": c })).collect::<Vec<_>>() })
        };
        let mut rules: Vec<Value> = Vec::new();
        for i in 0..10 {
            let clause = ["WHERE", "JOIN", "QUALIFY", "HAVING"][i % 4];
            // A control character in a text is a space on a card; one rule's text was cut by collin.
            let text = if i == 3 { "c.id = o.id\tAND\nx".to_string() } else { format!("p{i} > {i}") };
            rules.push(rule(&m("chain_03"), clause, &text, i == 5, &[("seed.shop.countries", "ID")]));
        }
        rules.push(rule(&m("chain_03"), "WHERE", "reads nothing the cache knows", false, &[("model.shop.ghost", "id")]));
        rules.push(rule(&m("chain_04"), "JOIN", "a.id = b.id", false, &[("seed.shop.countries", "id"), (&m("chain_04"), "j")]));
        rules.push(rule(&m("dia_b"), "WHERE", "status <> 'x'", false, &[("seed.shop.countries", "Id")]));
        rules.push(rule("model.crm.customers", "WHERE", "region = 'eu'", false, &[("seed.shop.countries", "id")]));

        let value = json!({
            "version": 1, "source": "collin", "target": "qa", "generated_at": "2030-01-02T03:04:05Z", "producer": "collin 0.12.0",
            "edges": edges,
            "roots": [
                { "node": m("dia_a"), "column": "id", "expressions": ["'web'", "'shop'"] },
                { "node": m("dia_b"), "column": "amount", "expressions": [] },
                { "node": m("dia_c"), "column": "Region", "expressions": ["'a'"] },
                { "node": m("dia_c"), "column": "region", "expressions": ["'b'"] },
                { "node": m("dia_d"), "column": "Region", "expressions": ["'a'"] },
                { "node": m("dia_d"), "column": "region", "expressions": [] },
                { "node": m("ghost"), "column": "id", "expressions": ["1"] },
            ],
            "rules": rules,
            "expressions": [
                { "node": m("dia_d"), "column": "Total_\u{c4}", "kind": "aggregate", "text": "sum(amount)", "steps": 3 },
                { "node": m("dia_b"), "column": "amount", "kind": "cast", "text": "cast(x as int)", "steps": 32, "cut": true },
                { "node": m("dia_b"), "column": "AMOUNT", "kind": "cast", "text": "short", "steps": 2 },
                { "node": m("hub"), "column": "metric", "kind": "", "text": "", "steps": 0 },
            ],
            "partial": [
                { "node": m("dia_b"), "column": "amount", "reason": "name_unresolved", "name": "b" },
                { "node": m("dia_b"), "column": "amount", "reason": "relation_unknown", "name": "a" },
                { "node": m("dia_b"), "column": "amount", "reason": "cte_lacks_name", "name": "a" },
                { "node": m("dia_b"), "column": "amount", "reason": "steps_capped", "name": "x" },
                { "node": m("dia_c"), "column": "id", "reason": "steps_capped", "name": "id" },
                { "node": m("dia_d"), "column": "id", "reason": "window_key_unbound", "name": "" },
                { "node": m("hub"), "column": "metric", "reason": "name_unresolved", "name": "z\u{e9}" },
            ],
        });
        serde_json::from_value(value).unwrap()
    }

    /// collin's notes on a few models of the case, as `notes_of` hands them to the cards: its report named
    /// where a model's SQL was read, under the project, and what it found wrong with a column.
    fn case_notes() -> HashMap<String, ModelNote> {
        let root = Path::new("/work/shop");
        let note = |provenance: &str, parse_error: &str, sql_file: &str, columns: Vec<collin_run::ColumnNote>| {
            scrubbed(
                ModelNote {
                    provenance: provenance.into(),
                    parse_error: parse_error.into(),
                    issues: columns.len(),
                    sql_file: sql_file.into(),
                    set_aside: false,
                    columns,
                },
                root,
            )
        };
        let lost = collin_run::ColumnNote {
            column: "amount".into(),
            kind: "lost",
            detail: "name_unresolved".into(),
            names: vec!["b".into()],
            of: 0,
        };
        HashMap::from([
            ("model.shop.dia_a".to_string(), note("per_column", "", "/work/shop/target/compiled/shop/models/marts/dia_a.sql", vec![lost])),
            ("model.shop.orders.v1".to_string(), note("unresolved", "cannot read /work/shop/models/marts/orders.sql", "", Vec::new())),
            ("model.shop.dia_b".to_string(), note("parsed", "", "/elsewhere/dia_b.sql", Vec::new())),
        ])
    }

    /// The case graph with `cache` merged, as `generate` does it: after the catalog, which the case has none of.
    fn merged(mut graph: Graph, cache: crate::collin::RawColLineage) -> Graph {
        graph.merge_col_lineage(cache, 0);
        graph
    }

    /// A cache of invented edges, roots, expressions, gaps and rules among a dozen nodes of the case, from a
    /// seed: cycles, self-edges, roles of every kind and stops arise as they will. Another cache over the
    /// same index, which the invented one above is not.
    fn random_cache(seed: u64) -> crate::collin::RawColLineage {
        let mut state = seed;
        let mut roll = move |n: u64| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state % n
        };
        let nodes = [
            "model.shop.dia_a", "model.shop.dia_b", "model.shop.dia_c", "model.shop.dia_d", "model.shop.hub", "seed.shop.countries",
            "model.shop.orders.v1", "model.shop.orders.v2", "model.shop.customers", "model.shop.fork", "model.shop.fork_a",
            "model.shop.fork_b",
        ];
        let columns = ["id", "c0", "c1", "C2"];
        let kinds =
            ["passthrough", "rename", "cast", "window", "aggregate", "transform", "filter", "join_key", "dedup_key", "JOIN_KEY", ""];
        let pick = |roll: &mut dyn FnMut(u64) -> u64| (nodes[roll(12) as usize], columns[roll(4) as usize]);
        let mut edges = Vec::new();
        for _ in 0..45 {
            let (from, fc) = pick(&mut roll);
            let (to, tc) = pick(&mut roll);
            let kind = kinds[roll(11) as usize];
            edges.push(json!({ "from": from, "from_col": fc, "to": to, "to_col": tc, "kind": kind }));
        }
        let mut roots = Vec::new();
        let mut expressions = Vec::new();
        let mut partial = Vec::new();
        let mut rules = Vec::new();
        for _ in 0..12 {
            let (n, c) = pick(&mut roll);
            let values: Vec<String> = (0..roll(3)).map(|i| format!("{i}")).collect();
            roots.push(json!({ "node": n, "column": c, "expressions": values }));
            let (n, c) = pick(&mut roll);
            let (kind, text, steps, cut) = (kinds[roll(6) as usize], format!("e{}", roll(9)), roll(40), roll(4) == 0);
            expressions.push(json!({ "node": n, "column": c, "kind": kind, "text": text, "steps": steps, "cut": cut }));
            let (n, c) = pick(&mut roll);
            let (reason, name) =
                (["name_unresolved", "steps_capped", "cte_lacks_name"][roll(3) as usize], ["", "a", "b"][roll(3) as usize]);
            partial.push(json!({ "node": n, "column": c, "reason": reason, "name": name }));
            let (n, _) = pick(&mut roll);
            let (r, rc) = pick(&mut roll);
            let text = format!("w{}", roll(9));
            rules.push(json!({ "node": n, "clause": "WHERE", "text": text, "cut": false, "columns": [{ "node": r, "column": rc }] }));
        }
        serde_json::from_value(json!({
            "version": 1, "source": "collin", "target": "qa", "generated_at": "2030-01-03T00:00:00Z", "producer": "collin 0.12.0",
            "edges": edges, "roots": roots, "expressions": expressions, "partial": partial, "rules": rules,
        }))
        .unwrap()
    }

    /// What an older collin writes, or Fusion: edges and nothing else, no root, no rule, no indirect role, and a column from a
    /// table no node owns. With no roots a column's lack of an edge says nothing, so no mark is drawn.
    fn plain_cache() -> crate::collin::RawColLineage {
        let m = |n: &str| format!("model.shop.{n}");
        let edge = |from: &str, fc: &str, to: &str, tc: &str, kind: &str| json!({ "from": from, "from_col": fc, "to": to, "to_col": tc, "kind": kind });
        serde_json::from_value(json!({
            "version": 1, "source": "fusion", "target": "qa", "generated_at": "2030-01-04T00:00:00Z",
            "edges": [
                edge("rel:analytics.raw.stage", "id", "source.shop.raw.orders", "id", ""),
                edge("source.shop.raw.orders", "id", &m("dia_a"), "id", ""),
                edge(&m("dia_a"), "id", &m("dia_b"), "id", ""),
                edge(&m("dia_b"), "id", &m("dia_d"), "id", ""),
                edge(&m("dia_a"), "id", &m("dia_c"), "id", ""),
                edge(&m("dia_c"), "id", &m("dia_d"), "id", ""),
            ],
        }))
        .unwrap()
    }

    /// `/api/collineage` and `/api/colreach` as the server answers them for each of the cases below, over
    /// a graph with a cache merged, with the file the page loads them from.
    fn column_cases(graph: &Graph, deep: bool, every: usize) -> (String, Vec<Value>, Vec<Value>) {
        let text = format!("EDITH_SITE_CLL({});\n", js_json(&cll_of(graph).expect("a merged cache")));
        let ask = |id: &str, column: &str, up: u32, down: u32, max: Option<usize>, cut: &str, indirect: u8| {
            let mut path = format!("/api/collineage?id={}&column={}&up={up}&down={down}&indirect={indirect}", enc(id), enc(column));
            if let Some(max) = max {
                path.push_str(&format!("&max={max}"));
            }
            if !cut.is_empty() {
                path.push_str(&format!("&cut={}", enc(cut)));
            }
            let idx = graph.index[id];
            let col = graph.col_slot(idx, column).unwrap_or_else(|| panic!("no column {column} in {id}"));
            let cuts: Vec<&str> = cut.split(',').filter(|r| !r.is_empty()).collect();
            let want =
                serde_json::to_value(api::col_lineage_answer(graph, ColRef { node: idx, col }, up, down, max, &cuts, indirect)).unwrap();
            json!({ "path": path, "want": want })
        };
        let mut cases: Vec<Value> = Vec::new();
        // The columns the cache says something of, at the depth the page opens on: with the indirect button on where an
        // indirect edge touches the column, off elsewhere, where on would answer the same. A column it names nothing of
        // answers as a lone box, so one such column stands for them. Of a chain whose links answer alike, every few.
        let cll = graph.cll.as_ref().expect("a merged cache");
        let mut said: std::collections::HashSet<ColRef> = std::collections::HashSet::new();
        for e in cll.edge_list() {
            said.extend([e.from, e.to]);
        }
        said.extend(cll.root_list().iter().map(|(c, _)| *c));
        said.extend(cll.expression_list().iter().map(|(c, _)| *c));
        said.extend(cll.lost_columns());
        said.extend(cll.outside_list().iter().copied());
        let touched_by_rows: std::collections::HashSet<ColRef> =
            cll.edge_list().iter().filter(|e| cll.is_indirect(e)).flat_map(|e| [e.from, e.to]).collect();
        let (mut quiet, mut seen) = (0, 0);
        for (i, node) in graph.nodes.iter().enumerate() {
            let link = node.id.strip_prefix("model.shop.chain_").and_then(|n| n.parse::<u32>().ok());
            if link.is_some_and(|n| ![1, 11, 12, 15, 22].contains(&n)) {
                continue;
            }
            for (slot, column) in node.columns.iter().enumerate() {
                let c = ColRef { node: i as u32, col: slot as u32 };
                // Asked far both ways below, where the same column shows more.
                if [
                    ("model.shop.dia_a", "id"),
                    ("model.shop.chain_01", "k"),
                    ("model.shop.chain_11", "k"),
                    ("model.shop.chain_22", "k"),
                    ("model.shop.dia_b", "id"),
                    ("model.shop.dia_d", "id"),
                ]
                .contains(&(node.id.as_str(), column.name.as_str()))
                {
                    continue;
                }
                if !said.contains(&c) {
                    quiet += 1;
                    if quiet > 1 {
                        continue;
                    }
                } else {
                    seen += 1;
                    if seen % every != 0 {
                        continue;
                    }
                }
                for indirect in if touched_by_rows.contains(&c) { vec![0, 1] } else { vec![0] } {
                    cases.push(ask(&node.id, &column.name, 2, 2, None, "", indirect));
                }
            }
        }
        if deep {
            let (chain, dia, hub, fork) = ("model.shop.chain_11", "model.shop.dia_a", "model.shop.hub", "model.shop.fork_a");
            // As far as the menu goes, a chain that goes further, and under a cap that cuts it on either side.
            for (id, column, up, down, max, cut, indirect) in [
                (chain, "k", 20, 20, None, "", 0),
                (chain, "k", 20, 20, Some(7), "", 0),
                (chain, "k", 20, 20, Some(3), "", 1),
                // A cap met only on the walk's extra level past the twentieth cuts nothing drawn.
                ("model.shop.chain_01", "k", 0, 20, Some(21), "", 0),
                ("model.shop.chain_22", "k", 20, 0, Some(21), "", 0),
                ("model.shop.chain_01", "k", 0, 20, Some(20), "", 0),
                // A stop reached around the cut, and the roles named for it, one that is none.
                (dia, "id", 1, 3, None, "window", 0),
                (dia, "id", 2, 3, None, "window,nonsense", 0),
                (dia, "id", 0, 2, Some(3), "window", 0),
                (dia, "id", 0, 4, Some(4), "window,aggregate", 1),
                ("source.shop.raw.orders", "id", 0, 5, None, "rename,window", 0),
                ("model.shop.chain_12", "m", 0, 3, Some(3), "window", 0),
                ("model.shop.chain_12", "m", 0, 3, Some(4), "window", 1),
                ("model.shop.chain_18", "p", 0, 3, None, "", 1),
                ("model.shop.chain_15", "q", 0, 2, None, "window,aggregate", 0),
                ("model.shop.chain_15", "q", 0, 3, None, "window,aggregate", 1),
                // The indirect edges' boxes under a cap, past the columns that reach them.
                ("seed.shop.countries", "id", 0, 2, Some(3), "", 1),
                ("seed.shop.countries", "id", 1, 3, Some(12), "", 1),
                ("seed.shop.countries", "id", 2, 2, Some(1), "", 1),
                (hub, "metric", 20, 20, Some(5), "", 1),
                (hub, "metric", 20, 20, None, "window", 0),
                // Cycles: two models, and a column that reads itself.
                (fork, "c1", 20, 20, None, "", 0),
                (fork, "c1", 3, 3, Some(2), "", 1),
                ("model.shop.fork", "s", 2, 2, None, "", 1),
                // The route's clamps, a column in another case, one that is not ASCII.
                (dia, "ID", 99, 99, Some(99_999), "", 1),
                ("model.shop.dia_d", "TOTAL_\u{c4}", 2, 2, None, "", 0),
            ] {
                cases.push(ask(id, column, up, down, max, cut, indirect));
            }
            // No depth, no cap and no indirect given: the route's own defaults.
            let (idx, col) = (graph.index[dia], graph.col_slot(graph.index[dia], "id").unwrap());
            let want = serde_json::to_value(api::col_lineage_answer(graph, ColRef { node: idx, col }, 2, 2, None, &[], 0)).unwrap();
            cases.push(json!({ "path": format!("/api/collineage?id={}&column=id", enc(dia)), "want": want }));
        } else {
            // A cache from a seed: every column far both ways, under a cap and a cut, where the graph is a tangle.
            for node in graph.nodes.iter().filter(|n| n.columns.iter().any(|c| c.name == "c0")).take(1) {
                cases.push(ask(&node.id, "c0", 20, 20, Some(9), "window,aggregate", 1));
                cases.push(ask(&node.id, "c0", 20, 20, Some(14), "filter,join_key", 0));
            }
        }
        // Every node with a column, and one with none, which answers an empty list.
        let mut bare = false;
        let reach = graph
            .nodes
            .iter()
            .filter(|n| {
                let first = n.columns.is_empty() && !bare;
                bare |= first;
                !n.columns.is_empty() || first
            })
            .map(|n| {
                let want = serde_json::to_value(api::col_reach_rows(graph, graph.index[&n.id])).unwrap();
                json!({ "path": format!("/api/colreach?id={}", enc(&n.id)), "want": want })
            })
            .collect();
        (text, cases, reach)
    }

    // ------------------------------------------ column lineage, from a folder ----

    /// A cache of invented edges between the three models `write_project` has, for `generate` to find
    /// beside the manifest, and the report collin writes with it, one model noted.
    fn write_cache(project: &Path, source: &str) {
        let target = project.join("target");
        let edge = |from: &str, to: &str, kind: &str| json!({ "from": format!("model.shop.{from}"), "from_col": "id", "to": format!("model.shop.{to}"), "to_col": "id", "kind": kind });
        let cache = json!({
            "version": 1, "source": source, "target": "qa", "generated_at": "2030-01-02T03:04:05Z", "producer": "collin 0.12.0",
            "edges": [edge("raw_orders", "stg_orders", "passthrough"), edge("stg_orders", "orders", "transform")],
        });
        std::fs::write(target.join("column_lineage.collin.json"), serde_json::to_vec(&cache).unwrap()).unwrap();
        // The model's SQL was read from a file under the project, which a published card must not name by its place on this machine.
        let sql_file = format!("{}/target/compiled/shop/models/orders.sql", project.display());
        let report = json!({
            "totals": { "models": 3, "parsed": 3 },
            "models": [{ "unique_id": "model.shop.orders", "provenance": "per_column", "parse_error": format!("cannot read {sql_file}, /elsewhere/deep/b.sql, C:\\Users\\x\\c.sql and \\\\?\\D:\\t\\e.sql"),
                         "sql_file": sql_file, "lost_columns": [{ "column": "id", "ends": [] }] }],
        });
        std::fs::write(target.join(collin_run::REPORT), serde_json::to_vec(&report).unwrap()).unwrap();
    }

    fn site_index(project: &Path) -> Value {
        let text = std::fs::read_to_string(project.join("target/edith-site/data/index.js")).unwrap();
        serde_json::from_str(text.trim_end().strip_prefix("window.EDITH_SITE=").unwrap().trim_end_matches(';')).unwrap()
    }

    fn no_collin() -> collin_run::Located {
        panic!("collin was asked for, and --site alone never starts it")
    }

    fn run_site(project: &Path, ask: &Ask, locate: impl FnOnce() -> collin_run::Located) -> (i32, String, String) {
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = generate_with(project, None, None, ask, locate, &mut out, &mut err);
        (code, String::from_utf8(out).unwrap(), String::from_utf8(err).unwrap())
    }

    #[test]
    fn a_site_with_no_cache_says_so_and_starts_nothing() {
        let project = scratch("cll-none");
        write_project(&project);
        let (code, out, err) = run_site(&project, &ask_for("https://github.com/acme/shop"), no_collin);
        assert_eq!(code, ls::ANSWERED, "{err}");
        assert!(err.contains("no column lineage: collin has written no cache beside the manifest, and --run-collin writes one"), "{err}");
        assert!(!out.contains("column lineage:"), "{out}");
        let site = project.join("target/edith-site");
        assert!(!site.join("data/cll.js").exists());
        let json = site_index(&project);
        assert_eq!(json["meta"]["cll"], false);
        assert!(json["meta"].get("cll_file").is_none());
        // The limits are there whatever the cache is: a page asks them of any site.
        assert_eq!(json["limits"]["col_depth"], 2);
        let _ = std::fs::remove_dir_all(&project);
    }

    #[test]
    fn a_cache_beside_the_manifest_is_carried_whole_and_named_bare() {
        let project = scratch("cll-cache");
        write_project(&project);
        write_cache(&project, "collin");
        let ask = ask_for("https://github.com/acme/shop");
        let (code, out, err) = run_site(&project, &ask, no_collin);
        assert_eq!(code, ls::ANSWERED, "{err}");
        assert!(out.contains("  column lineage: 2 edges from collin"), "{out}");
        assert!(!err.contains("no column lineage") && !err.contains("older than the manifest"), "{err}");
        let json = site_index(&project);
        let meta = &json["meta"];
        assert_eq!(
            (meta["cll"].clone(), meta["cll_tool"].clone(), meta["cll_file"].clone(), meta["cll_edges"].clone(), meta["cll_stale"].clone()),
            (json!(true), json!("collin"), json!("column_lineage.collin.json"), json!(2), json!(false))
        );
        assert_eq!(
            (meta["cll_source"].as_str(), meta["cll_target"].as_str(), meta["cll_generated_at"].as_str()),
            (Some("collin"), Some("qa"), Some("2030-01-02T03:04:05Z"))
        );
        assert_eq!(meta["cll_kinds"], json!([{ "kind": "passthrough", "edges": 1 }, { "kind": "transform", "edges": 1 }]));
        // Neither a time nor a producer, which the page has no use for and a run would change.
        assert!(meta.get("cll_mtime").is_none() && meta.get("cll_producer").is_none());

        // The file is the cache as merged, and the same bytes on every run, with no path of this machine in any of it.
        let first = files_of(&project.join("target/edith-site"));
        let cll = String::from_utf8(first["data/cll.js"].clone()).unwrap();
        assert!(cll.starts_with("EDITH_SITE_CLL({") && cll.ends_with("});\n"), "{cll}");
        let (code, _, err) = run_site(&project, &ask, no_collin);
        assert_eq!(code, ls::ANSWERED, "{err}");
        assert_eq!(files_of(&project.join("target/edith-site")), first);
        let temp = std::env::temp_dir().display().to_string();
        for (name, bytes) in &first {
            if name.starts_with("vendor/") {
                continue;
            }
            let text = String::from_utf8_lossy(bytes);
            for needle in [project.to_string_lossy().as_ref(), temp.as_str()] {
                assert!(!text.contains(needle), "{name} holds {needle}");
            }
        }
        // The card of the model collin noted says what it found, and not where on this machine it looked.
        let cards = String::from_utf8_lossy(&first["data/cards/0000.js"]).into_owned();
        assert!(cards.contains("\"provenance\":\"per_column\""), "{cards}");
        assert!(cards.contains("\"sql_file\":\"target/compiled/shop/models/orders.sql\"") && !cards.contains("parse_error"), "{cards}");
        for outside in ["elsewhere", "Users", "\\\\\\\\?"] {
            assert!(!cards.contains(outside), "a card holds {outside}: {cards}");
        }
        let _ = std::fs::remove_dir_all(&project);
    }

    #[test]
    fn a_cache_older_than_the_manifest_is_carried_with_its_warning() {
        let project = scratch("cll-stale");
        write_project(&project);
        write_cache(&project, "collin");
        let cache = project.join("target/column_lineage.collin.json");
        let aged = |file: &Path, by: u64| {
            let f = std::fs::OpenOptions::new().write(true).open(file).unwrap();
            f.set_modified(std::time::SystemTime::now() - std::time::Duration::from_secs(by)).unwrap();
        };
        // The manifest an hour old, the cache a day: the cache is behind.
        aged(&project.join("target/manifest.json"), 3600);
        aged(&cache, 86_400);
        let (code, out, err) = run_site(&project, &ask_for("https://github.com/acme/shop"), no_collin);
        assert_eq!(code, ls::ANSWERED, "{err}");
        assert!(
            err.contains(
                "warning: collin's column lineage is older than the manifest: it may not match this commit; --run-collin writes it again"
            ),
            "{err}"
        );
        assert!(out.contains("column lineage: 2 edges from collin"), "{out}");
        assert_eq!(site_index(&project)["meta"]["cll_stale"], true);
        assert!(project.join("target/edith-site/data/cll.js").is_file(), "a stale cache is carried, with its warning");

        // Written after the manifest: current.
        aged(&cache, 60);
        let (_, _, err) = run_site(&project, &ask_for("https://github.com/acme/shop"), no_collin);
        assert!(!err.contains("older than the manifest"), "{err}");
        assert_eq!(site_index(&project)["meta"]["cll_stale"], false);
        let _ = std::fs::remove_dir_all(&project);
    }

    #[test]
    fn a_cache_that_cannot_be_read_is_left_out_and_said() {
        let project = scratch("cll-broken");
        write_project(&project);
        let cache = project.join("target/column_lineage.collin.json");
        let ask = ask_for("https://github.com/acme/shop");
        for (text, why) in [
            ("{ not json", "warning: no column lineage, the cache was ignored:"),
            ("{\"version\":2,\"source\":\"collin\",\"edges\":[]}", "is not a column lineage cache this build can read"),
            // A report left under the cache's name: no edges, no cache.
            ("{\"totals\":{},\"models\":[]}", "is not a column lineage cache this build can read"),
        ] {
            std::fs::write(&cache, text).unwrap();
            let (code, out, err) = run_site(&project, &ask, no_collin);
            assert_eq!(code, ls::ANSWERED, "{err}");
            assert!(err.contains(why), "{text}: {err}");
            assert!(!out.contains("column lineage:") && !project.join("target/edith-site/data/cll.js").exists(), "{text}");
            assert_eq!(site_index(&project)["meta"]["cll"], false);
        }
        let _ = std::fs::remove_dir_all(&project);
    }

    #[test]
    fn a_cache_another_tool_wrote_names_that_tool_and_reads_no_report() {
        let project = scratch("cll-fusion");
        write_project(&project);
        write_cache(&project, "fusion");
        let (code, out, err) = run_site(&project, &ask_for("https://github.com/acme/shop"), no_collin);
        assert_eq!(code, ls::ANSWERED, "{err}");
        assert!(out.contains("  column lineage: 2 edges from fusion"), "{out}");
        assert_eq!(site_index(&project)["meta"]["cll_tool"], "fusion");
        // collin's report is collin's: no card carries a note when another tool's cache is the one carried.
        let cards = String::from_utf8_lossy(&std::fs::read(project.join("target/edith-site/data/cards/0000.js")).unwrap()).into_owned();
        assert!(!cards.contains("per_column"), "{cards}");
        let _ = std::fs::remove_dir_all(&project);
    }

    #[cfg(unix)]
    fn stand_in(dir: &Path, body: &str) -> collin_run::Located {
        let bin = dir.join("collin-stand-in");
        let script = format!(
            "#!/bin/sh\n[ \"$1\" = --version ] && echo 'collin 0.12.0' && exit 0\nout=''\nreport=''\n\
             while [ $# -gt 0 ]; do [ \"$1\" = --out ] && out=\"$2\"; [ \"$1\" = --report ] && report=\"$2\"; shift; done\n{body}\n"
        );
        collin_run::tests::executable(&bin, &script);
        collin_run::Located { bin: Some(bin), ..Default::default() }
    }

    #[cfg(unix)]
    #[test]
    fn run_collin_writes_the_cache_first_and_the_site_carries_it() {
        let project = scratch("cll-run");
        write_project(&project);
        let tools = scratch("cll-run-tools");
        let writes = r#"echo '{"version":1,"source":"collin","producer":"collin 0.12.0","edges":[{"from":"model.shop.raw_orders","from_col":"id","to":"model.shop.orders","to_col":"id","kind":"passthrough"}]}' > "$out"
echo '{"totals":{"models":3,"parsed":3},"models":[]}' > "$report""#;
        let ask = Ask { produce: Some(ls_column::Produce { catalog: None, compiled: None }), ..ask_for("https://github.com/acme/shop") };
        let (code, out, err) = run_site(&project, &ask, || stand_in(&tools, writes));
        assert_eq!(code, ls::ANSWERED, "{err}");
        assert!(err.starts_with("collin 0.12.0 wrote"), "{err}");
        assert!(out.contains("column lineage: 1 edges from collin"), "{out}");
        assert!(project.join("target/column_lineage.collin.json").is_file() && project.join("target").join(collin_run::REPORT).is_file());
        assert_eq!(site_index(&project)["meta"]["cll_stale"], false, "the cache was just written");
        assert!(project.join("target/edith-site/data/cll.js").is_file());

        // A run that writes nothing leaves no site: the folder it would have been is as it was.
        let project2 = scratch("cll-run-fails");
        write_project(&project2);
        let (code, _, err) = run_site(&project2, &ask, || stand_in(&tools, "exit 1"));
        assert_eq!(code, ls::CANNOT_RUN, "{err}");
        assert!(err.contains("error: collin wrote no cache"), "{err}");
        assert!(!project2.join("target/edith-site").exists(), "nothing was written");
        // No collin to run: the same, with the way to get one.
        let (code, _, err) = run_site(&project2, &ask, collin_run::Located::default);
        assert_eq!(code, ls::CANNOT_RUN, "{err}");
        assert!(err.contains("--run-collin found no collin to run"), "{err}");
        assert!(!project2.join("target/edith-site").exists());
        // A cache that was written and cannot be read ends the run too, rather than a site without it.
        let (code, _, err) = run_site(&project2, &ask, || stand_in(&tools, r#"echo '{ broken' > "$out""#));
        assert_eq!(code, ls::CANNOT_RUN, "{err}");
        assert!(!project2.join("target/edith-site").exists());
        // A folder refused is refused before collin runs: no cache is written for nothing.
        let project3 = scratch("cll-run-refused");
        write_project(&project3);
        let refused = Ask { dir: Some(project3.join("target")), ..ask };
        let (code, _, err) = run_site(&project3, &refused, no_collin);
        assert_eq!(code, ls::CANNOT_RUN, "{err}");
        assert!(!project3.join("target/column_lineage.collin.json").exists());
        for dir in [&project, &project2, &project3, &tools] {
            let _ = std::fs::remove_dir_all(dir);
        }
    }

    #[test]
    fn a_note_names_nothing_of_this_machine() {
        let root = Path::new("/work/shop");
        let scrubbed = |sql_file: &str, parse_error: &str| {
            let note = ModelNote { sql_file: sql_file.into(), parse_error: parse_error.into(), ..ModelNote::default() };
            let note = super::scrubbed(note, root);
            (note.sql_file, note.parse_error)
        };
        assert_eq!(scrubbed("/work/shop/target/compiled/a.sql", "").0, "target/compiled/a.sql");
        assert_eq!(scrubbed("target/compiled/a.sql", "").0, "target/compiled/a.sql", "a file already relative stays");
        assert_eq!(scrubbed("/elsewhere/a.sql", "").0, "", "a file outside the project is dropped");
        assert_eq!(scrubbed("../up/a.sql", "").0, "");
        assert_eq!(scrubbed("C:\\other\\a.sql", "").0, "");
        assert_eq!(scrubbed("", "no file /work/shop/models/a.sql and /work/shop").1, "no file models/a.sql and ");
        // A text that still holds a path outside the root, a Windows one, a verbatim one or a share is dropped whole, and so
        // is one whose path has a space in it, which a word-by-word rewrite would leave the tail of.
        for text in [
            "cannot read /elsewhere/deep/b.sql: no such file",
            "cannot read 'C:\\Users\\x\\c.sql' or (D:/data/d.sql)",
            "at \\\\?\\C:\\tmp\\e.sql",
            "\\\\host\\share\\f.sql",
            "cannot read /Users/John Smith/x.sql",
            "cannot read C:\\Users\\John Smith\\x.sql",
        ] {
            assert_eq!(scrubbed("", text).1, "", "{text}");
        }
        assert_eq!(
            scrubbed("", "a / b, a/b and 5:30 stay").1,
            "a / b, a/b and 5:30 stay",
            "division, a relative path and a time are no path"
        );
        assert_eq!(
            scrubbed("", "no file /work/shop/my dir/a.sql").1,
            "no file my dir/a.sql",
            "a space below the root stays: the root is cut"
        );
        assert_eq!(scrubbed("/work/shop/../x.sql", "").0, "", "a way up out of the project is dropped");
        assert_eq!(scrubbed("\\\\?\\C:\\tmp\\g.sql", "").0, "");
        assert_eq!(scrubbed("D:/data/h.sql", "").0, "");
        let column = collin_run::ColumnNote {
            column: "id".into(),
            kind: "lost",
            detail: "/work/shop/x and /other/z.sql".into(),
            names: vec!["/work/shop/y".into(), "C:\\w\\v.sql".into(), "/Users/John Smith/w.sql".into()],
            of: 0,
        };
        let note =
            super::scrubbed(ModelNote { columns: vec![column], provenance: "from /srv/q/p.sql".into(), ..ModelNote::default() }, root);
        assert_eq!(note.provenance, "");
        assert_eq!(note.columns[0].detail, "");
        assert_eq!(note.columns[0].names, ["y"], "a name holding a path is removed from the list");
    }

    #[test]
    fn a_column_is_a_number_and_the_file_is_the_cache_as_merged() {
        let graph = merged(case_graph(), case_cache());
        let cll = graph.cll.as_ref().unwrap();
        let file = cll_of(&graph).unwrap();
        // A column's number is where its node starts plus its slot, so the order of a column is the number's.
        let at: Vec<usize> = file["at"].as_array().unwrap().iter().map(|v| v.as_u64().unwrap() as usize).collect();
        assert_eq!(at.len(), graph.nodes.len() + 1);
        for (i, n) in graph.nodes.iter().enumerate() {
            assert_eq!(at[i + 1] - at[i], n.columns.len(), "{}", n.id);
        }
        assert_eq!(file["nm"].as_array().unwrap().len(), at[graph.nodes.len()]);
        // The edges come back as they are held: by source then target, which is the order the walks meet them in.
        let (mut f, mut seen) = (0, Vec::new());
        for (i, d) in file["from"].as_array().unwrap().iter().enumerate() {
            f += d.as_u64().unwrap() as usize;
            seen.push((f, file["to"][i].as_u64().unwrap() as usize, file["ek"][i].as_u64().unwrap() as u8));
        }
        let held: Vec<(usize, usize, u8)> = cll
            .edge_list()
            .iter()
            .map(|e| (at[e.from.node as usize] + e.from.col as usize, at[e.to.node as usize] + e.to.col as usize, e.kind))
            .collect();
        assert_eq!(seen, held);
        assert!(seen.windows(2).all(|w| (w[0].0, w[0].1) < (w[1].0, w[1].1)), "sorted by source then target, never twice");
        // A name and a type are interned, and each column's is the one the node has.
        let names = file["names"].as_array().unwrap();
        let types = file["types"].as_array().unwrap();
        for (i, n) in graph.nodes.iter().enumerate() {
            for (slot, c) in n.columns.iter().enumerate() {
                assert_eq!(names[file["nm"][at[i] + slot].as_u64().unwrap() as usize], c.name.as_str());
                assert_eq!(types[file["ty"][at[i] + slot].as_u64().unwrap() as usize], c.data_type.as_str());
            }
        }
        // The roles that say which rows exist are named by their kind, with their strength.
        let kinds: Vec<&str> = file["kinds"].as_array().unwrap().iter().map(|k| k.as_str().unwrap()).collect();
        let rank = |k: &str| file["rank"][kinds.iter().position(|x| *x == k).unwrap()].as_u64().unwrap();
        assert_eq!(
            (rank("filter"), rank("FILTER"), rank("join_key"), rank("dedup_key"), rank("passthrough"), rank("")),
            (1, 1, 2, 3, 0, 0)
        );
        let row: Vec<&str> = file["row"].as_array().unwrap().iter().map(|k| kinds[k.as_u64().unwrap() as usize]).collect();
        assert!(row.contains(&"FILTER") && row.contains(&"dedup_key") && !row.contains(&"window"), "{row:?}");
        // The same file every time, and no path in it.
        assert_eq!(js_json(&file), js_json(&cll_of(&graph).unwrap()));
        assert!(cll_of(&case_graph()).is_none(), "no cache, no file");
    }

    /// A model's box on a column canvas is the node as the lineage draws it on its own, with a depth, its
    /// own line and no +N: `web/site.js` builds it from the index's base, so the two must be one.
    #[test]
    fn a_models_box_on_a_column_canvas_is_its_base_and_four_fields() {
        let graph = case_graph();
        for i in 0..graph.nodes.len() as u32 {
            let got = serde_json::to_value(graph.whole_model_node(i, 3)).unwrap();
            let mut want = graph.box_base(i);
            let n = &graph.nodes[i as usize];
            // What only a test says of itself is not on a model's box, whatever node a cache names.
            for key in ["test_name", "column", "namespace", "warn", "host"] {
                want.as_object_mut().unwrap().remove(key);
            }
            want["depth"] = json!(3);
            want["hidden_up"] = json!(0);
            want["hidden_down"] = json!(0);
            want["sub"] = json!(if n.materialized.is_empty() {
                "whole model".to_string()
            } else {
                format!("{}  \u{b7}  whole model", n.materialized)
            });
            assert_eq!(got, want, "{}", n.id);
        }
    }

    /// The site's page and the server's say the same words to a column, which the golden file proves for the
    /// walks: these are the clamps the page is told, which are the route's own constants.
    #[test]
    fn the_columns_limits_are_the_routes_own() {
        let index = index_of(&case_graph(), json!({}));
        let l = &index["limits"];
        assert_eq!(
            (l["col_depth"].as_u64(), l["col_depth_max"].as_u64(), l["col_drawn"].as_u64(), l["col_most"].as_u64(), l["rows_by"].as_u64()),
            (Some(2), Some(20), Some(1000), Some(3000), Some(8))
        );
        assert_eq!(api::COL_DEPTH_MAX as usize, MAX_LEVELS, "the menu lists as many levels as the route walks");
    }

    // ----------------------------------------------------- the golden case ----

    /// A percent-encoding as `encodeURIComponent` writes it, which is how the page
    /// builds its queries.
    fn enc(text: &str) -> String {
        text.bytes()
            .map(|b| if b.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") })
            .collect()
    }

    /// The invented graph the fixture is made of: a diamond, a test that guards
    /// two models, a disabled node, the versions of one model, an operation and an
    /// exposure, a chain of 22 levels, a fan-out, accents and emoji, the same name
    /// in two packages, and two materializations whose names order differently
    /// by code point than by UTF-16 unit.
    fn case_parts() -> (Graph, Value) {
        let node = |name: &str, package: &str, kind: &str, folder: &str, parents: &[&str], extra: Value| {
            // A version is part of the id, as dbt writes it.
            let id = match extra.get("version") {
                Some(version) => format!("{kind}.{package}.{name}.v{version}"),
                None => format!("{kind}.{package}.{name}"),
            };
            let mut v = json!({
                "name": name, "resource_type": kind, "package_name": package,
                "original_file_path": format!("models/{folder}/{name}.sql"),
                "database": "analytics", "schema": "marts",
                "config": { "materialized": "table" },
                "description": format!("The {name} node."),
                "columns": { "id": { "name": "id", "description": "The key.", "data_type": "int" } },
                "depends_on": { "nodes": parents },
            });
            if let (Some(o), Some(e)) = (v.as_object_mut(), extra.as_object()) {
                for (k, x) in e {
                    o.insert(k.clone(), x.clone());
                }
                // The chain and the fan-out are shapes, not cards: they keep their fixture small.
                if folder == "chain" || folder == "leaves" {
                    o.remove("columns");
                    o.remove("description");
                }
            }
            (id, v)
        };
        let mat = |m: &str| json!({ "config": { "materialized": m } });
        let mut nodes: Vec<(String, Value)> = Vec::new();
        let mut parent_map: Vec<(String, Vec<String>)> = Vec::new();
        let mut add = |entry: (String, Value)| {
            let parents: Vec<String> =
                entry.1["depends_on"]["nodes"].as_array().unwrap().iter().map(|p| p.as_str().unwrap().to_string()).collect();
            parent_map.push((entry.0.clone(), parents));
            nodes.push(entry);
        };
        let m = |n: &str| format!("model.shop.{n}");

        // A chain 22 levels deep, so the depth menu says it goes on.
        for i in 1..=22 {
            let parents = if i == 1 { vec![] } else { vec![m(&format!("chain_{:02}", i - 1))] };
            let parents: Vec<&str> = parents.iter().map(String::as_str).collect();
            add(node(&format!("chain_{i:02}"), "shop", "model", "chain", &parents, mat(if i % 3 == 0 { "view" } else { "table" })));
        }
        // A diamond, fed by a seed and a source, read by an exposure.
        add(node("countries", "shop", "seed", "seeds", &[], json!({})));
        let raw = (
            "source.shop.raw.orders".to_string(),
            json!({
                "name": "orders", "source_name": "raw", "resource_type": "source", "package_name": "shop",
                "original_file_path": "models/staging/sources.yml", "database": "raw", "schema": "shop",
                "depends_on": { "nodes": [] },
            }),
        );
        add(raw);
        add(node("dia_a", "shop", "model", "marts", &["seed.shop.countries", "source.shop.raw.orders"], mat("incremental")));
        add(node("dia_b", "shop", "model", "marts", &[&m("dia_a")], mat("view")));
        add(node("dia_c", "shop", "model", "marts", &[&m("dia_a")], mat("")));
        add(node("dia_d", "shop", "model", "marts", &[&m("dia_b"), &m("dia_c")], mat("table")));
        // A fan-out: more children than a cap of 5 holds.
        add(node("hub", "shop", "model", "marts", &[&m("dia_d")], mat("table")));
        for i in 1..=6 {
            add(node(&format!("leaf_{i:02}"), "shop", "model", "leaves", &[&m("hub")], mat("view")));
        }
        // A version pair, and the same name in two packages.
        add(node("orders", "shop", "model", "marts", &["seed.shop.countries"], json!({ "version": 1, "latest_version": 2 })));
        add(node("orders", "shop", "model", "marts", &[&m("orders.v1")], json!({ "version": 2, "latest_version": 2 })));
        add(node("customers", "shop", "model", "marts", &["seed.shop.countries"], mat("table")));
        add(node("customers", "crm", "model", "crm", &["seed.shop.countries"], mat("table")));
        // A path with a space and a `#`, which a link to the file must keep in one segment each.
        let odd = json!({ "original_file_path": "models/odd dir/a#b.sql", "config": { "materialized": "table" } });
        add(node("spaced", "shop", "model", "odd dir", &[&m("dia_d")], odd));
        // Accents, emoji, capitals and Unicode spaces in names and paths.
        add(node("caf\u{e9}_orders", "shop", "model", "caf\u{e9}", &[&m("dia_d")], mat("table")));
        add(node("orders_\u{1f600}", "shop", "model", "marts", &[&m("dia_d")], mat("table")));
        add(node("\u{c9}CLAIR", "shop", "model", "marts", &[&m("dia_d")], mat("table")));
        add(node("zz_\u{ff21}", "shop", "model", "marts", &[&m("dia_d")], mat("table")));
        // Folders for the tree: names that differ by case, a package model with no place, and two
        // snapshots in one file, a legacy file of the project.
        add(node("Zeta", "shop", "model", "Mixed", &[], mat("table")));
        add(node("alpha", "shop", "model", "Mixed", &[], mat("view")));
        add(node("accounts", "crm", "model", "crm", &[], mat("table")));
        for name in ["s_b", "s_a"] {
            let file = json!({ "original_file_path": "snapshots/snaps.sql" });
            add(node(name, "shop", "snapshot", "snapshots", &[], file));
        }
        // Two children whose materializations order differently by code point.
        add(node("fork", "shop", "model", "marts", &[&m("dia_d")], mat("table")));
        add(node("fork_a", "shop", "model", "marts", &[&m("fork")], mat("mat\u{ff21}")));
        add(node("fork_b", "shop", "model", "marts", &[&m("fork")], mat("mat\u{1f600}")));
        // An operation under the hub, which no walk crosses, and an exposure.
        add(node("shop-on-run-end-0", "shop", "operation", "ops", &[&m("hub")], json!({})));
        let mut exposures = serde_json::Map::new();
        exposures.insert(
            "exposure.shop.dashboard".to_string(),
            json!({ "name": "dashboard", "resource_type": "exposure", "package_name": "shop", "original_file_path": "models/exposures.yml",
                    "depends_on": { "nodes": [m("dia_d"), m("hub")] } }),
        );

        // Tests: a generic one per model on the diamond, a relationships test that
        // guards two models, a singular one with two parents, and several on the hub.
        let test = |name: &str, host: &str, parents: &[&str], column: &str, generic: &str| {
            let mut v = json!({
                "name": name, "resource_type": "test", "package_name": "shop",
                "original_file_path": "models/marts/schema.yml", "attached_node": host,
                "depends_on": { "nodes": parents },
                "test_metadata": { "name": generic, "kwargs": { "column_name": column } },
                "column_name": column, "config": { "severity": "ERROR" },
            });
            if generic.is_empty() {
                v.as_object_mut().unwrap().remove("test_metadata");
                v.as_object_mut().unwrap().remove("attached_node");
            }
            (format!("test.shop.{name}"), v)
        };
        for (name, host, parents, column, generic) in [
            ("unique_dia_a_id", "model.shop.dia_a", vec![m("dia_a")], "id", "unique"),
            ("not_null_dia_a_id", "model.shop.dia_a", vec![m("dia_a")], "id", "not_null"),
            ("unique_dia_d_id", "model.shop.dia_d", vec![m("dia_d")], "id", "unique"),
            ("relationships_dia_d_id__id__ref_dia_b_", "model.shop.dia_d", vec![m("dia_d"), m("dia_b")], "id", "relationships"),
            ("unique_hub_id", "model.shop.hub", vec![m("hub")], "id", "unique"),
            ("not_null_hub_id", "model.shop.hub", vec![m("hub")], "id", "not_null"),
            ("unique_chain_05_id", "model.shop.chain_05", vec![m("chain_05")], "id", "unique"),
            ("assert_hub_matches_dia_d", "", vec![m("hub"), m("dia_d")], "", ""),
        ] {
            let parents: Vec<&str> = parents.iter().map(String::as_str).collect();
            add(test(name, host, &parents, column, generic));
        }

        parent_map.push(("exposure.shop.dashboard".to_string(), vec![m("dia_d"), m("hub")]));

        let disabled = json!({
            "model.shop.old_hub": [{
                "name": "old_hub", "resource_type": "model", "package_name": "shop", "original_file_path": "models/old/old_hub.sql",
                "config": { "materialized": "table", "enabled": false }, "depends_on": { "nodes": [m("dia_d")] },
            }],
        });
        let manifest = json!({
            "metadata": { "project_name": "shop", "dbt_version": "1.9.0", "generated_at": "2030-01-02T03:04:05Z", "adapter_type": "duckdb" },
            "nodes": nodes.into_iter().collect::<BTreeMap<_, _>>(),
            "exposures": exposures,
            "disabled": disabled,
            "parent_map": parent_map.into_iter().collect::<BTreeMap<_, _>>(),
        });
        let mut manifest = manifest;
        // The SQL: one node of each shape the page shows, and a path that needs escaping.
        for (id, code) in case_code() {
            let at = if id.starts_with("model.shop.old_hub") { &mut manifest["disabled"][id][0] } else { &mut manifest["nodes"][id] };
            for (k, v) in code.as_object().unwrap() {
                at[k] = v.clone();
            }
        }
        let raw: crate::manifest::RawManifest = serde_json::from_value(manifest.clone()).unwrap();
        let mut graph = Graph::build(raw, Path::new("manifest.json"), 0, 0);
        // A package model `dbt deps` put where it was found, as `place_packages` records it.
        let at = graph.index["model.crm.customers"] as usize;
        let place = "dbt_packages/crm/models/crm/customers.sql".to_string();
        let file = graph.nodes[at].file.clone();
        if let Some(held) = graph.by_file.get_mut(&file) {
            held.retain(|&j| j != at as u32);
        }
        graph.by_file.entry(place.clone()).or_default().push(at as u32);
        graph.nodes[at].place = place;
        (graph, manifest)
    }

    fn case_graph() -> Graph {
        case_parts().0
    }

    /// The code the fixture's nodes carry, by id: what each shape of node holds, and a file whose path
    /// needs escaping in a link.
    fn case_code() -> Vec<(&'static str, Value)> {
        vec![
            // A line ending that is two characters, and a compiled text.
            (
                "model.shop.dia_a",
                json!({ "raw_code": "select 1\r\nfrom {{ ref('countries') }}", "compiled_code": "select 1\nfrom analytics.seeds.countries", "language": "sql" }),
            ),
            // Not compiled, and a name that would close a script or end a line there.
            ("model.shop.dia_b", json!({ "raw_code": "select '</script><!--\u{2028}\u{2029}' as a" })),
            (
                "model.shop.orders.v1",
                json!({ "raw_code": "def model(dbt, session):\n    return dbt.ref('countries')", "compiled_code": "def model(dbt, session):\n    return dbt.ref('countries')", "language": "python" }),
            ),
            // A package's file is not in the repository: no link.
            ("model.crm.customers", json!({ "raw_code": "select 'crm'", "compiled_code": "select 'crm'" })),
            (
                "model.shop.caf\u{e9}_orders",
                json!({ "raw_code": "select '\u{e9}' as caf\u{e9}", "compiled_code": "select '\u{e9}' as caf\u{e9}" }),
            ),
            ("model.shop.spaced", json!({ "raw_code": "select 1", "compiled_code": "select 1" })),
            (
                "operation.shop.shop-on-run-end-0",
                json!({ "raw_code": "grant select on all tables", "compiled_code": "grant select on all tables" }),
            ),
            (
                "test.shop.unique_hub_id",
                json!({ "raw_code": "{{ test_unique(**_dbt_generic_test_kwargs) }}", "compiled_code": "select id from analytics.marts.hub group by id having count(*) > 1" }),
            ),
            ("test.shop.assert_hub_matches_dia_d", json!({ "raw_code": "select 1 where false", "compiled_code": "select 1 where false" })),
            // A disabled node has code and was never compiled.
            ("model.shop.old_hub", json!({ "raw_code": "select 'old'" })),
        ]
    }

    /// What the golden file holds of the jobs: the script, and the server's own answers to compare the page's with.
    struct CaseJobs {
        meta: Value,
        script: String,
        summary: Value,
        nodes: Vec<Value>,
        refused_script: String,
        refused_summary: Value,
        refused_node: Value,
    }

    /// Two DAGs of invented tasks over the case graph, a file that does not parse, and a source that is not there: so a node
    /// has jobs by a term, by a step that selects it however it is reached, and by one that could run it and cannot be
    /// resolved, and every node says one source was not read. The answers are `for_node`'s and `summary`'s, written here
    /// by the server's own types and never by `jobs_payload`.
    fn case_jobs(graph: &Graph) -> CaseJobs {
        let dir = scratch("case-jobs");
        let dags = dir.join("dags");
        std::fs::create_dir_all(&dags).unwrap();
        std::fs::write(
            dags.join("acme_nightly.yml"),
            "dag_id: acme_nightly\nschedule: \"0 3 * * *\"\ntasks:\n  - task_id: upstream_of_hub\n    command: dbt run --select +path:models/marts/hub.sql\n  - task_id: versions\n    command: dbt build --select path:models/marts/orders.sql config.materialized:view --exclude path:models/marts/dia_d.sql\n    depends_on: [upstream_of_hub]\n  - task_id: seeds\n    command: dbt seed\n  - task_id: changed\n    command: dbt run --select state:modified+\n",
        )
        .unwrap();
        std::fs::write(
            dags.join("acme_weekly.yml"),
            "dag_id: acme_weekly\nschedule: \"0 4 * * 1\"\ntasks:\n  - task_id: check\n    command: dbt build --select path:models/marts/hub.sql\n",
        )
        .unwrap();
        std::fs::write(dags.join("acme_broken.yml"), "dag_id: [unclosed\n").unwrap();
        let source = |path: &str, label: &str| crate::settings::JobSource {
            kind: "airflow".into(),
            path: path.into(),
            label: label.into(),
            keys: Default::default(),
            timezone: String::new(),
            from_file: false,
        };
        let (read, _) = jobs::build(graph, &dir, &[source("dags", "orchestration"), source("nowhere", "gone")]);
        let links: Vec<JobLinks> = read.sources.iter().map(|_| JobLinks::default()).collect();
        let summary_of = |read: &jobs::Read, refused: Option<&str>| {
            let mut v = serde_json::to_value(jobs::summary(read)).unwrap();
            for s in v["sources"].as_array_mut().unwrap() {
                s.as_object_mut().unwrap().remove("resolved");
            }
            v["from"] = json!("project");
            if let Some(why) = refused {
                v["refused"] = json!(why);
            }
            v
        };
        let nodes = (0..graph.nodes.len())
            .map(|i| {
                let want = serde_json::to_value(jobs::for_node(graph, &read, i as u32)).unwrap();
                json!({ "path": format!("/api/jobs/node?id={}", enc(&graph.nodes[i].id)), "want": want })
            })
            .collect();
        let why = "line 2: a source with no kind or no path";
        let empty = jobs::Read::default();
        let mut refused_node = serde_json::to_value(jobs::for_node(graph, &empty, 0)).unwrap();
        refused_node["sources_refused"] = json!(why);
        let out = CaseJobs {
            meta: jobs_meta(&read, None, &links),
            script: format!("EDITH_SITE_JOBS({});\n", js_json(&jobs_payload(graph, &read, None))),
            summary: json!({ "path": "/api/jobs", "want": summary_of(&read, None) }),
            nodes,
            refused_script: format!("EDITH_SITE_JOBS({});\n", js_json(&jobs_payload(graph, &empty, Some(why)))),
            refused_summary: json!({ "path": "/api/jobs", "want": summary_of(&empty, Some(why)) }),
            refused_node: json!({ "path": format!("/api/jobs/node?id={}", enc(&graph.nodes[0].id)), "want": refused_node }),
        };
        let _ = std::fs::remove_dir_all(&dir);
        out
    }

    /// The whole fixture, as the text of the file. What `web/site.js` is held to.
    fn case_text() -> String {
        let (graph, manifest) = case_parts();
        // With the invented cache merged, as `generate` merges one: the cards carry what the cache says of their columns.
        let graph = merged(graph, case_cache());
        let notes = case_notes();
        let base = "https://github.com/acme/shop/blob/0123456789abcdef0123456789abcdef01234567/analytics/";
        let mut meta = json!({ "project": "shop", "root": "shop", "environment": "qa", "commit": "", "commit_url": "", "repo_url": "", "file_base": base, "catalog": false });
        cll_meta(&graph, Some(false), &mut meta);
        let jobs = case_jobs(&graph);
        meta["jobs"] = jobs.meta.clone();
        let mut index = index_of(&graph, meta);
        // Small buckets, so the lookup of a card by its number is exercised, and the SQL's own cut is not the cards'.
        let (cuts, buckets) = cards_in(&graph, Some(&notes), 7, BUCKET_BYTES).unwrap();
        index["cuts"] = json!(cuts);
        let code = sql_of(&graph, serde_json::from_value(manifest).unwrap());
        let (sql_cuts, sql) = sql_in(code.clone(), 5, BUCKET_BYTES).unwrap();
        index["sql_cuts"] = json!(sql_cuts);

        let id = |name: &str| graph.index.get(name).copied().unwrap_or_else(|| panic!("no node {name}"));
        let lineage = |focus: &str, up: u32, down: u32, tests: bool, max: usize| {
            let path = format!("/api/lineage?id={}&up={up}&down={down}&tests={}&max={max}", enc(focus), u8::from(tests));
            let want = serde_json::to_value(api::lineage_answer(&graph, id(focus), up, down, tests, Some(max))).unwrap();
            json!({ "path": path, "want": want })
        };
        let mut cases: Vec<Value> = Vec::new();
        // One level each way, with the tests drawn for the nodes that have some: one of each kind of
        // node and of each shape in the graph. The middle of the chain and of the fan-out add
        // nothing their ends do not.
        for name in [
            "model.shop.dia_b", "seed.shop.countries", "source.shop.raw.orders", "model.shop.orders.v1", "model.shop.customers",
            "model.crm.customers", "model.shop.fork_a", "model.shop.chain_22", "model.shop.old_hub", "operation.shop.shop-on-run-end-0",
            "test.shop.unique_hub_id", "test.shop.assert_hub_matches_dia_d",
        ] {
            let tests = !graph.nodes[id(name) as usize].tests.is_empty();
            cases.push(lineage(name, 1, 1, tests, 400));
        }
        // A few foci at the depths the page offers, with and without tests, and under its cap and a small one.
        // Not every pair: where two give one answer the second proves nothing the first did not.
        for focus in ["model.shop.chain_01", "model.shop.hub", "model.shop.dia_a", "exposure.shop.dashboard"] {
            let deepest = if focus == "model.shop.chain_01" || focus == "model.shop.dia_a" { 20 } else { 3 };
            for (up, down, tests, max) in [(0, 0, true, 400), (2, 2, false, 400), (2, 2, true, 5), (deepest, deepest, false, 400)] {
                cases.push(lineage(focus, up, down, tests, max));
            }
        }
        // Where the walk stops: under a cap, in one direction, with the tests.
        cases.push(lineage("model.shop.hub", 20, 20, true, 5));
        cases.push(lineage("model.shop.hub", 1, 3, true, 9));
        // Tests under a cap, one per model a turn: the order in which the cap is met.
        cases.push(lineage("model.shop.hub", 1, 0, true, 5));
        // The route's defaults and its clamps: no depth, no tests, no cap given; more than the most.
        let focus = "model.shop.orders.v1";
        {
            let want =
                serde_json::to_value(api::lineage_answer(&graph, id(focus), api::LINEAGE_DEPTH, api::LINEAGE_DEPTH, false, None)).unwrap();
            cases.push(json!({ "path": format!("/api/lineage?id={}", enc(focus)), "want": want }));
            let want = serde_json::to_value(api::lineage_answer(&graph, id(focus), 99, 99, true, Some(99_999))).unwrap();
            cases.push(json!({ "path": format!("/api/lineage?id={}&up=99&down=99&tests=1&max=99999", enc(focus)), "want": want }));
        }

        let search = |q: &str, kind: &str, limit: Option<usize>| {
            let mut path = format!("/api/search?q={}&kind={kind}", enc(q));
            if let Some(l) = limit {
                path.push_str(&format!("&limit={l}"));
            }
            let want = serde_json::to_value(api::search_answer(&graph, q, kind, limit)).unwrap();
            json!({ "path": path, "want": want })
        };
        let kinds = "model,source,seed,snapshot";
        let mut searches: Vec<Value> = Vec::new();
        for (q, kind, limit) in [
            ("", "", Some(4)),
            ("", "", None),
            ("", "model", Some(5)),
            ("", "test", Some(3)),
            ("", "exposure", None),
            ("", "bogus", Some(3)),
            ("", "model,nonsense,seed", Some(4)),
            ("orders", kinds, Some(8)),
            ("ORDERS", "", Some(3)),
            ("orders", "model", Some(1)),
            ("\u{85}orders", "", Some(3)),
            ("\u{feff}orders", "", Some(3)),
            ("staging/", "", Some(3)),
            ("caf", "", Some(3)),
            ("\u{e9}clair", "", Some(3)),
            ("\u{1f600}", "", Some(3)),
            ("customers", "", Some(4)),
            ("leaf_", "model", Some(3)),
            ("chain_1", "model", Some(200_000)),
            ("raw", "", Some(3)),
            ("unique", "test", Some(3)),
            ("nothing matches this", "", None),
            ("hub", "", Some(0)),
            ("a", "", Some(12)),
        ] {
            searches.push(search(q, kind, limit));
        }

        // The tree, every folder of it, as /api/dir lists it: what the server answers is the files the
        // manifest builds from, with the kind and the materialization its icon is drawn by.
        let (tree, _) = tree_of(&graph);
        let dirs: Vec<Value> = tree
            .iter()
            .map(|(dir, list)| {
                let want: Vec<Value> = list
                    .iter()
                    .map(|(name, n)| {
                        let path = if dir.is_empty() { name.clone() } else { format!("{dir}/{name}") };
                        if *n < 0 {
                            return json!({ "name": name, "path": path, "dir": true, "size": 0 });
                        }
                        let node = &graph.nodes[*n as usize];
                        let mut e =
                            json!({ "name": name, "path": path, "dir": false, "size": 0, "kind": node.kind.as_str(), "id": node.id });
                        if node.kind == Kind::Model && !node.materialized.is_empty() {
                            e["materialized"] = json!(node.materialized);
                        }
                        e
                    })
                    .collect();
                json!({ "path": format!("/api/dir?path={}", enc(dir)), "want": want })
            })
            .collect();

        // The cards: a handful by their id, through the buckets.
        let nodes: Vec<Value> = [
            "model.shop.dia_a", "model.shop.orders.v1", "exposure.shop.dashboard", "model.shop.old_hub", "model.shop.dia_b",
            "model.shop.hub",
        ]
        .iter()
        .map(|name| {
            let want = serde_json::to_value(api::node_detail(&graph, id(name), notes.get(*name).cloned(), None)).unwrap();
            json!({ "path": format!("/api/node?id={}", enc(name)), "want": want })
        })
        .collect();

        // The code of a node, with the file it came from and the link to it: only the project's own files are in the
        // repository, so a package's has none. A node with no code answers an empty one.
        let codes: Vec<Value> = case_code()
            .iter()
            .map(|(name, _)| *name)
            .chain(["seed.shop.countries", "model.shop.dia_c"])
            .map(|name| {
                let i = id(name);
                let n = &graph.nodes[i as usize];
                let sql = code[i as usize].as_ref();
                let url = if n.package == "shop" {
                    json!(format!("{base}{}", n.on_disk().split('/').map(enc).collect::<Vec<_>>().join("/")))
                } else {
                    Value::Null
                };
                let want = json!({
                    "raw": sql.map_or("", |s| s.raw.as_str()),
                    "compiled": sql.and_then(|s| s.compiled.clone()),
                    "lang": if sql.is_some_and(|s| s.python) { "python" } else { "sql" },
                    "file": n.on_disk(),
                    "url": url,
                });
                json!({ "path": format!("/api/site/code?id={}", enc(name)), "want": want })
            })
            .collect();

        // Column lineage: the invented cache, and one from a seed over the same index.
        let (cll, collineage, colreach) = column_cases(&graph, true, 1);
        let random = merged(case_parts().0, random_cache(0x5eed_1234_abcd_ef01));
        let (random_cll, random_collineage, random_colreach) = column_cases(&random, false, 10);
        let plain = merged(case_parts().0, plain_cache());
        let (plain_cll, plain_collineage, plain_colreach) = column_cases(&plain, false, 1);

        let case = json!({
            "site": index, "buckets": buckets, "sql": sql, "lineage": cases, "search": searches, "node": nodes, "dir": dirs, "code": codes,
            "jobs": jobs.script, "jobs_summary": jobs.summary, "jobs_node": jobs.nodes,
            "jobs_refused": { "script": jobs.refused_script, "summary": jobs.refused_summary, "node": jobs.refused_node },
            "cll": cll, "collineage": collineage, "colreach": colreach,
            "random": { "cll": random_cll, "collineage": random_collineage, "colreach": random_colreach },
            "plain": { "cll": plain_cll, "collineage": plain_collineage, "colreach": plain_colreach },
        });
        format!(
            "// Written by `DBT_EDITH_BLESS=1 cargo test site_case` from an invented graph: Rust's answers, which web/tests/site.js\n\
             // holds web/site.js to. Never edited by hand.\nvar SITE_CASE = {};\n",
            js_json(&case)
        )
    }

    /// Rust's answers to the page's questions, as a file `web/tests/site.js` reads
    /// the browser's answers against. A change to a walk that moves an answer fails
    /// here until the fixture is written again, and there until `web/site.js` follows.
    #[test]
    fn site_case_is_what_rust_answers() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/site_case.js");
        let text = case_text();
        if std::env::var("DBT_EDITH_BLESS").is_ok_and(|v| v == "1") {
            std::fs::write(&path, &text).unwrap();
            return;
        }
        let held = std::fs::read_to_string(&path).unwrap_or_default();
        assert!(held == text, "tests/fixtures/site_case.js is not what Rust answers: rewrite it with DBT_EDITH_BLESS=1 cargo test site_case, and run web/tests/site.js");
    }

    /// The hits of the search are a box's own fields, which is why the site ships no
    /// second copy of them.
    #[test]
    fn a_hit_is_a_boxs_fields() {
        let graph = case_graph();
        for i in 0..graph.nodes.len() as u32 {
            let b = graph.box_base(i);
            let mut want = serde_json::Map::new();
            for k in ["id", "name", "kind", "file", "schema", "materialized", "tests", "disabled", "test_name", "column"] {
                if let Some(v) = b.get(k) {
                    want.insert(k.to_string(), v.clone());
                }
            }
            assert_eq!(serde_json::to_value(api::hit(&graph, i)).unwrap(), Value::Object(want), "{}", graph.nodes[i as usize].id);
        }
    }

    /// The tree is the files the manifest builds from, and the folders above them: no test, no
    /// macro, no folder of YAML.
    #[test]
    fn the_tree_holds_the_files_models_seeds_and_snapshots_are_built_from() {
        let graph = case_graph();
        let (tree, placed) = tree_of(&graph);
        let names = |dir: &str| tree[dir].iter().map(|(n, k)| format!("{}{}", n, if *k < 0 { "/" } else { "" })).collect::<Vec<_>>();
        // Folders first, then by lower-cased name: `Mixed` and `caf\u{e9}` are not ordered by their capitals.
        assert_eq!(names(""), ["dbt_packages/", "models/", "snapshots/"]);
        assert_eq!(names("models/Mixed"), ["alpha.sql", "Zeta.sql"], "case does not order a name");
        assert!(names("models").iter().position(|n| n == "caf\u{e9}/") < names("models").iter().position(|n| n == "chain/"));
        // A package model at the place it was found, and one at the place `dbt deps` would put it.
        assert_eq!(names("dbt_packages/crm/models/crm"), ["accounts.sql", "customers.sql"]);
        assert_eq!(placed.len(), 1, "{placed:?}");
        assert!(placed.values().all(|p| p == "dbt_packages/crm/models/crm/accounts.sql"));
        // Two snapshots in a file: listed once, with the first by name.
        assert_eq!(names("snapshots"), ["snaps.sql"]);
        let id = &graph.nodes[tree["snapshots"][0].1 as usize].id;
        assert_eq!(id, "snapshot.shop.s_a");
        // Not what no file builds: tests, the operation, the exposure, the source, the disabled node.
        let listed: Vec<&str> = tree.values().flatten().filter(|(_, k)| *k >= 0).map(|(n, _)| n.as_str()).collect();
        for gone in ["old_hub.sql", "shop-on-run-end-0.sql", "dashboard.sql", "sources.yml", "schema.yml"] {
            assert!(!listed.contains(&gone), "{gone}");
        }
        assert!(
            !tree.contains_key("models/old") && !tree.keys().any(|d| d.contains("ops") || d.contains("exposures")),
            "{:?}",
            tree.keys()
        );
        // Every folder listed has a file below it, and is listed by its parent.
        for dir in tree.keys().filter(|d| !d.is_empty()) {
            let (parent, name) = dir.rsplit_once('/').unwrap_or(("", dir));
            assert!(tree[parent].iter().any(|(n, k)| n == name && *k < 0), "{dir}");
        }
    }

    /// The two rules are one: a file the tree lists is the file `node_built_in` answers for, with the
    /// node it names, and a node it answers with is in the tree. CI runs this, not the walk.
    #[test]
    fn the_tree_and_node_built_in_agree() {
        let graph = case_graph();
        let (tree, placed) = tree_of(&graph);
        let moved: BTreeSet<&String> = placed.values().collect();
        let mut rows = BTreeSet::new();
        for (dir, list) in &tree {
            for (name, n) in list.iter().filter(|(_, n)| *n >= 0) {
                let path = if dir.is_empty() { name.clone() } else { format!("{dir}/{name}") };
                rows.insert(*n as u32);
                if moved.contains(&path) {
                    continue;
                }
                let found = graph.node_built_in(&path).map(|f| f.id.as_str());
                assert_eq!(found, Some(graph.nodes[*n as usize].id.as_str()), "{path}");
            }
        }
        for n in &graph.nodes {
            for file in [&n.file, &n.place] {
                if let Some(found) = graph.node_built_in(file) {
                    assert!(rows.contains(&graph.index[&found.id]), "{} answers for {file} and has no row", found.id);
                }
            }
        }
    }

    /// The page reads counts from the base and the lists from the adjacency: they must
    /// be the same numbers, or `hidden_up` would count what the card does not list.
    #[test]
    fn the_index_agrees_with_itself() {
        let graph = case_graph();
        let index = index_of(&graph, json!({}));
        let n = graph.nodes.len();
        for key in ["base", "parents", "children", "tests", "rank", "key", "att", "lv"] {
            assert_eq!(index[key].as_array().unwrap().len(), n, "{key}");
        }
        for i in 0..n {
            for (count, list) in [("parents", "parents"), ("children", "children"), ("tests", "tests")] {
                assert_eq!(index["base"][i][count].as_u64().unwrap() as usize, index[list][i].as_array().unwrap().len(), "{count} of {i}");
            }
        }
        let mut ranks: Vec<u64> = index["rank"].as_array().unwrap().iter().map(|r| r.as_u64().unwrap()).collect();
        ranks.sort_unstable();
        assert_eq!(ranks, (0..n as u64).collect::<Vec<_>>(), "a rank is a position: each one once");
        // Buckets are contiguous and cover every node.
        let (cuts, buckets) = cards_in(&graph, None, 7, BUCKET_BYTES).unwrap();
        assert_eq!(cuts, (0..n).step_by(7).collect::<Vec<_>>());
        assert_eq!(buckets.len(), cuts.len());
        // The byte limit closes a bucket as the count does.
        let (cuts, _) = cards_in(&graph, None, 1000, 1).unwrap();
        assert_eq!(cuts.len(), n);
    }

    // ----------------------------------------------------------------- jobs ----

    const DAG_DAILY: &str = "dag_id: acme_daily\nschedule: \"0 6 * * *\"\ntasks:\n  - task_id: stage\n    command: dbt run --select stg_orders+\n  - task_id: check\n    command: dbt test --select orders\n    depends_on: [stage]\n";
    // `state:modified` cannot be resolved from a manifest alone, so its step is one that might run a model.
    const DAG_ODD: &str = "dag_id: acme_odd\nschedule: \"30 2 * * *\"\ntasks:\n  - task_id: everything\n    command: dbt build\n  - task_id: changed\n    command: dbt run --select state:modified+\n";

    /// A shop beside an `airflow_repo`, the shape a pipeline clones: the project names the DAGs by a path that goes up one folder.
    /// `dags` is where in the repository the DAG files sit.
    fn jobs_project(tag: &str, dags: &str) -> (PathBuf, PathBuf, PathBuf) {
        let parent = scratch(tag);
        let project = parent.join("shop");
        write_project(&project);
        // A selection by `fqn:*`, which `dbt build` with no `--select` is, wants the manifest's fqn.
        let path = project.join("target/manifest.json");
        let mut manifest: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        for (_, node) in manifest["nodes"].as_object_mut().unwrap() {
            node["fqn"] = json!(["shop", node["name"].clone()]);
        }
        std::fs::write(&path, serde_json::to_vec(&manifest).unwrap()).unwrap();
        let airflow = parent.join("airflow_repo");
        let at = airflow.join(dags);
        std::fs::create_dir_all(&at).unwrap();
        std::fs::write(at.join("acme_daily.yml"), DAG_DAILY).unwrap();
        std::fs::write(at.join("acme_odd.yml"), DAG_ODD).unwrap();
        std::fs::create_dir_all(project.join(".dbt-edith")).unwrap();
        let named = format!("jobs:\n  - kind: airflow\n    path: \"../airflow_repo/{dags}\"\n    label: \"orchestration\"\n");
        std::fs::write(project.join(".dbt-edith/jobs.yml"), named).unwrap();
        (parent, project, airflow)
    }

    fn git_in(dir: &Path, args: &[&str]) -> String {
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .output()
            .unwrap();
        assert!(out.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&out.stderr));
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    /// A repository with one commit, and the sha of it. Made with no configuration of the machine's.
    fn commit_in(dir: &Path, origin: Option<&str>) -> String {
        init_repository(dir);
        if let Some(url) = origin {
            git_in(dir, &["remote", "add", "origin", url]);
        }
        git_in(
            dir,
            &[
                "-c", "user.name=acme", "-c", "user.email=acme@example.com", "-c", "commit.gpgsign=false", "commit", "--allow-empty", "-q",
                "-m", "first",
            ],
        );
        git_in(dir, &["rev-parse", "HEAD"])
    }

    fn read_script(dir: &Path, file: &str, call: &str) -> Value {
        let text = std::fs::read_to_string(dir.join(file)).unwrap();
        let inner = text.trim_end().strip_prefix(call).unwrap().trim_end_matches(';').trim_end_matches(')').to_string();
        serde_json::from_str(&inner).unwrap()
    }

    /// Whether a node is in runs `[gap, length, ...]`, as `web/site.js` reads them.
    fn in_runs(list: &Value, node: u64) -> bool {
        let (mut at, a) = (0u64, list.as_array().unwrap());
        a.chunks(2).any(|c| {
            at += c[0].as_u64().unwrap();
            let start = at;
            at += c[1].as_u64().unwrap();
            (start..at).contains(&node)
        })
    }

    /// What `web/site.js` does with the packed jobs: a node's answer, the server's own, back.
    fn expanded(payload: &Value, node: usize, kind: &str) -> Value {
        let steps = payload["steps"].as_array().unwrap();
        let mut groups: Vec<Value> = Vec::new();
        let mut last = u64::MAX;
        for (at, step) in steps.iter().enumerate() {
            let own = &payload["runs"][at];
            if own.is_null() || own == &json!(0) || !in_runs(own, node as u64) {
                continue;
            }
            let why = &payload["via"][at];
            let via: Vec<Value> = match why[0].as_str() {
                Some("f") => why[1].as_array().unwrap().clone(),
                Some(_) => why[1]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|t| in_runs(t.get(1).unwrap_or(own), node as u64))
                    .map(|t| t[0].clone())
                    .collect(),
                None => Vec::new(),
            };
            let mut step = step.clone();
            if !via.is_empty() {
                step["via"] = json!(via);
            }
            let job = payload["step_job"][at].as_u64().unwrap();
            if job != last {
                groups.push(json!({ "job": payload["jobs"][job as usize], "steps": [] }));
                last = job;
            }
            groups.last_mut().unwrap()["steps"].as_array_mut().unwrap().push(step);
        }
        let mut answer = payload["common"].clone();
        answer["kind"] = json!(kind);
        answer["groups"] = json!(groups);
        if let Some(list) = payload["maybe"].get(kind) {
            let list: Vec<Value> = list
                .as_array()
                .unwrap()
                .iter()
                .map(|g| {
                    let steps: Vec<Value> =
                        g[1].as_array().unwrap().iter().map(|at| steps[at.as_u64().unwrap() as usize].clone()).collect();
                    json!({ "job": payload["jobs"][g[0].as_u64().unwrap() as usize], "steps": steps })
                })
                .collect();
            answer["maybe"] = json!(list);
        }
        answer
    }

    #[test]
    fn runs_of_nodes_are_gaps_and_lengths() {
        assert_eq!(node_runs(&[]), Vec::<u32>::new());
        assert_eq!(node_runs(&[0, 1, 2, 5, 7, 8]), [0, 3, 2, 1, 1, 2]);
        assert_eq!(node_runs(&[4]), [4, 1]);
        let all: Vec<u32> = (0..50_000).collect();
        assert_eq!(node_runs(&all), [0, 50_000], "a step that selects everything is one run");
    }

    #[test]
    fn a_job_and_a_step_are_shipped_once_and_every_node_gets_the_servers_own_answer() {
        let (parent, project, _) = jobs_project("jobs-whole", "dags");
        let (code, out, err) = run_site(&project, &ask_for("https://github.com/acme/shop"), no_collin);
        assert_eq!(code, ls::ANSWERED, "{err}");
        assert!(out.contains("jobs: 2 jobs and 4 steps from 1 source(s)"), "{out}");
        let dir = project.join("target").join("edith-site");
        let payload = read_script(&dir, "data/jobs.js", "EDITH_SITE_JOBS(");

        let (root, _, graph) = ls::open_with_catalog(&project, None, None).unwrap();
        let sources = jobs::sources(&root, &[]);
        let (read, _) = jobs::build(&graph, &root, &sources.list);
        let mut with_jobs = 0;
        for (i, n) in graph.nodes.iter().enumerate() {
            let want = serde_json::to_value(jobs::for_node(&graph, &read, i as u32)).unwrap();
            assert_eq!(expanded(&payload, i, n.kind.as_str()), want, "{}", n.id);
            if !want["groups"].as_array().unwrap().is_empty() {
                with_jobs += 1;
            }
        }
        assert!(with_jobs >= 2, "the fixture must reach a node: {}", payload.to_string().chars().take(1500).collect::<String>());
        // A model that a step selects through a term says which term.
        let stg = graph.index["model.shop.stg_orders"] as usize;
        assert_eq!(expanded(&payload, stg, "model")["groups"][0]["steps"][0]["via"], json!(["stg_orders+"]));

        // Each job and each step once, in the order of the files; no node holds a job's fields.
        assert_eq!(payload["jobs"].as_array().unwrap().len(), 2);
        assert_eq!(payload["jobs"][0]["name"], "acme_daily");
        assert_eq!(payload["jobs"][1]["name"], "acme_odd");
        assert!(payload["steps"].as_array().unwrap().iter().all(|s| s.get("via").is_none()));
        // The unresolved step could run a model, and is said once for the kind, not once for each node.
        assert_eq!(payload["maybe"]["model"].as_array().unwrap().len(), 1);
        assert!(
            payload["steps"].as_array().unwrap().len() <= 4
                && payload["runs"].as_array().unwrap().len() == payload["steps"].as_array().unwrap().len()
        );
        assert!(payload.get("nodes").is_none(), "nothing grows with the number of nodes");

        // What the summary says is the server's, with the places of this machine left off.
        let want = serde_json::to_value(jobs::summary(&read)).unwrap();
        assert_eq!(payload["summary"]["jobs"], want["jobs"]);
        assert_eq!(payload["summary"]["steps"], want["steps"]);
        assert_eq!(payload["summary"]["from"], "project");
        let source = &payload["summary"]["sources"][0];
        assert!(source.get("resolved").is_none());
        assert_eq!((source["path"].as_str(), source["label"].as_str()), (Some("../airflow_repo/dags"), Some("orchestration")));

        // Nothing of this machine: not the project, the folder above it, the temporary folder or the home.
        let home = std::env::var("HOME").unwrap_or_default();
        for file in ["data/jobs.js", "data/index.js"] {
            let text = std::fs::read_to_string(dir.join(file)).unwrap();
            for needle in
                [parent.to_string_lossy().as_ref(), project.to_string_lossy().as_ref(), std::env::temp_dir().to_string_lossy().as_ref()]
            {
                assert!(!text.contains(needle), "{file} holds {needle}");
            }
            if !home.is_empty() && home != "/" {
                assert!(!text.contains(&home), "{file} holds the home folder");
            }
        }
        // The index says there are jobs, and counts: no time, no path.
        let index = read_script(&dir, "data/index.js", "window.EDITH_SITE=");
        let meta = &index["meta"]["jobs"];
        assert_eq!(
            (&meta["jobs"], &meta["steps"], &meta["present"], &meta["not_read"], &meta["refused"]),
            (&json!(2), &json!(4), &json!(1), &json!(0), &json!(0))
        );
        assert_eq!(meta["sources_refused"], "");
        assert_eq!(meta["sources"][0]["name"], "orchestration");
        let _ = std::fs::remove_dir_all(&parent);
    }

    #[test]
    fn the_same_jobs_give_the_same_bytes_twice_and_a_project_without_the_file_has_none() {
        let (parent, project, _) = jobs_project("jobs-twice", "dags");
        let first = {
            assert_eq!(run_site(&project, &ask_for("https://github.com/acme/shop"), no_collin).0, ls::ANSWERED);
            files_of(&project.join("target/edith-site"))
        };
        assert!(first.contains_key("data/jobs.js"));
        assert_eq!(run_site(&project, &ask_for("https://github.com/acme/shop"), no_collin).0, ls::ANSWERED);
        assert_eq!(files_of(&project.join("target/edith-site")), first);

        // A file that lists no source names no jobs: no tab, no file.
        std::fs::write(project.join(".dbt-edith/jobs.yml"), "jobs: []\n").unwrap();
        assert_eq!(run_site(&project, &ask_for("https://github.com/acme/shop"), no_collin).0, ls::ANSWERED);
        assert!(!files_of(&project.join("target/edith-site")).contains_key("data/jobs.js"));

        std::fs::remove_file(project.join(".dbt-edith/jobs.yml")).unwrap();
        let (code, out, err) = run_site(&project, &ask_for("https://github.com/acme/shop"), no_collin);
        assert_eq!(code, ls::ANSWERED, "{err}");
        let bare = files_of(&project.join("target/edith-site"));
        assert!(!bare.contains_key("data/jobs.js"));
        assert!(!out.contains("jobs:") && !err.contains("jobs"), "{out}{err}");
        let index = read_script(&project.join("target/edith-site"), "data/index.js", "window.EDITH_SITE=");
        assert!(index["meta"].get("jobs").is_none());
        let _ = std::fs::remove_dir_all(&parent);
    }

    #[test]
    fn a_source_that_is_missing_or_a_file_that_is_refused_is_carried_as_not_read_and_never_as_no_job() {
        // The repository was not cloned beside the project.
        let (parent, project, airflow) = jobs_project("jobs-missing", "dags");
        std::fs::remove_dir_all(&airflow).unwrap();
        let (code, _, err) = run_site(&project, &ask_for("https://github.com/acme/shop"), no_collin);
        assert_eq!(code, ls::ANSWERED, "{err}");
        assert!(err.contains("warning: jobs not read: orchestration (nothing at this path)"), "{err}");
        let dir = project.join("target/edith-site");
        let payload = read_script(&dir, "data/jobs.js", "EDITH_SITE_JOBS(");
        assert_eq!(payload["summary"]["sources"][0]["error"], "nothing at this path");
        assert_eq!(payload["common"]["configured"], true);
        assert_eq!(payload["common"]["not_read"]["sources"], 1);
        assert_eq!(payload["common"]["not_read"]["reason"], "nothing at this path");
        assert_eq!(payload["common"]["not_read_names"], json!(["orchestration"]));
        assert_eq!((payload["jobs"].as_array().unwrap().len(), payload["steps"].as_array().unwrap().len()), (0, 0));
        let index = read_script(&dir, "data/index.js", "window.EDITH_SITE=");
        assert_eq!((&index["meta"]["jobs"]["present"], &index["meta"]["jobs"]["not_read"]), (&json!(0), &json!(1)));
        let _ = std::fs::remove_dir_all(&parent);

        // The project's file itself does not read.
        let (parent, project, _) = jobs_project("jobs-refused", "dags");
        std::fs::write(project.join(".dbt-edith/jobs.yml"), "jobs:\n  - kind: airflow\n").unwrap();
        let (code, _, err) = run_site(&project, &ask_for("https://github.com/acme/shop"), no_collin);
        assert_eq!(code, ls::ANSWERED, "{err}");
        assert!(err.contains("warning: jobs not read: .dbt-edith/jobs.yml is not read: line 2: a source with no kind or no path"), "{err}");
        let dir = project.join("target/edith-site");
        let payload = read_script(&dir, "data/jobs.js", "EDITH_SITE_JOBS(");
        assert_eq!(payload["common"]["configured"], false);
        assert_eq!(payload["common"]["sources_refused"], "line 2: a source with no kind or no path");
        assert_eq!(payload["summary"]["refused"], "line 2: a source with no kind or no path");
        let index = read_script(&dir, "data/index.js", "window.EDITH_SITE=");
        assert_eq!(index["meta"]["jobs"]["sources_refused"], "line 2: a source with no kind or no path");
        let _ = std::fs::remove_dir_all(&parent);

        // One DAG file that does not parse inside a source that was read: counted, the other jobs kept.
        let (parent, project, airflow) = jobs_project("jobs-file", "dags");
        std::fs::write(airflow.join("dags/acme_broken.yml"), "dag_id: [unclosed\n").unwrap();
        let (code, _, err) = run_site(&project, &ask_for("https://github.com/acme/shop"), no_collin);
        assert_eq!(code, ls::ANSWERED, "{err}");
        assert!(err.contains("1 job file(s) of orchestration could not be read"), "{err}");
        let payload = read_script(&project.join("target/edith-site"), "data/jobs.js", "EDITH_SITE_JOBS(");
        assert_eq!(payload["common"]["refused"], 1);
        assert_eq!(payload["jobs"].as_array().unwrap().len(), 2);
        let _ = std::fs::remove_dir_all(&parent);
    }

    #[test]
    fn the_jobs_flags_are_checked_like_the_commit_and_the_repository() {
        let (parent, project, _) = jobs_project("jobs-flags", "dags");
        let go = |commit: Option<&str>, repo: Option<&str>| {
            let ask = Ask {
                jobs_commit: commit.map(String::from),
                jobs_repo_url: repo.map(String::from),
                ..ask_for("https://github.com/acme/shop")
            };
            run_site(&project, &ask, no_collin)
        };
        let (code, _, err) = go(Some("zzz"), None);
        assert_eq!((code, err.contains("--jobs-commit wants 7 to 64")), (2, true), "{err}");
        let (code, _, err) = go(None, Some("http://github.com/acme/airflow"));
        assert_eq!((code, err.contains("--jobs-repo-url wants an https address")), (2, true), "{err}");
        assert!(!project.join("target/edith-site").exists(), "a flag refused is a folder untouched");
        assert_eq!(go(Some("ABCDEF1"), Some("https://github.com/acme/airflow")).0, ls::ANSWERED);
        let _ = std::fs::remove_dir_all(&parent);
    }

    #[test]
    fn a_job_file_is_linked_at_the_commit_of_its_repository_below_the_folder_it_sits_in() {
        let (parent, project, airflow) = jobs_project("jobs-links", "orchestration/dags");
        let sha = commit_in(&airflow, Some("git@github.com:acme/airflow.git"));
        let source = |asked: &Ask| {
            let (index, err) = written(&project, asked);
            (index["meta"]["jobs"]["sources"][0].clone(), err)
        };
        // From git alone: HEAD of the jobs repository and its origin, not the project's (which gives no link here).
        let (s, err) = source(&Ask { repo_url: None, commit: None, ..ask_for("") });
        assert_eq!(s["base"], format!("https://github.com/acme/airflow/blob/{sha}/orchestration/dags/"), "{err}");
        assert_eq!((s["anchor"].clone(), s["commit"].clone()), (json!("L"), json!(&sha[..7])));
        assert_eq!(s["commit_url"], format!("https://github.com/acme/airflow/commit/{sha}"));
        // The flags win, for every source, and an address names its host's pattern and its line anchor.
        for (url, base, anchor) in [
            (
                "https://gitlab.com/acme/data/airflow",
                "https://gitlab.com/acme/data/airflow/-/blob/abcdef1/orchestration/dags/".to_string(),
                "L",
            ),
            (
                "https://bitbucket.org/acme/airflow",
                "https://bitbucket.org/acme/airflow/src/abcdef1/orchestration/dags/".to_string(),
                "lines-",
            ),
            (
                "https://git.example.com/acme/airflow",
                "https://git.example.com/acme/airflow/blob/abcdef1/orchestration/dags/".to_string(),
                "L",
            ),
        ] {
            let (s, err) = source(&Ask {
                jobs_commit: Some("abcdef1".into()),
                jobs_repo_url: Some(url.into()),
                ..ask_for("https://github.com/acme/shop")
            });
            assert_eq!(
                (s["base"].as_str(), s["anchor"].as_str(), s["commit"].as_str()),
                (Some(base.as_str()), Some(anchor), Some("abcdef1")),
                "{url}: {err}"
            );
        }
        // Azure has the commit and no file address.
        let (s, err) = source(&Ask {
            jobs_repo_url: Some("https://dev.azure.com/acme/data/_git/airflow".into()),
            ..ask_for("https://github.com/acme/shop")
        });
        assert_eq!(s["base"], Value::Null);
        assert!(s["commit_url"].as_str().unwrap().ends_with(&format!("/commit/{sha}")), "{s}");
        assert!(err.contains("no job file links for orchestration: the file addresses of Azure DevOps"), "{err}");
        // An origin that is only like a host's gives no link, and the run says to pass the flag.
        git_in(&airflow, &["remote", "set-url", "origin", "git@work.github.com:acme/airflow.git"]);
        let (s, err) = source(&Ask { repo_url: None, commit: None, ..ask_for("") });
        assert_eq!((s["base"].clone(), s["commit"].clone()), (Value::Null, json!("")));
        assert!(
            err.contains(
                "no job file links for orchestration: its origin is not on github.com, gitlab.com or bitbucket.org, pass --jobs-repo-url"
            ),
            "{err}"
        );
        // No repository: no link, and the site is written all the same.
        std::fs::remove_dir_all(airflow.join(".git")).unwrap();
        let (s, err) = source(&Ask { repo_url: None, commit: None, ..ask_for("") });
        assert_eq!(s["base"], Value::Null);
        assert!(err.contains("no job file links for orchestration: not a git repository"), "{err}");
        let _ = std::fs::remove_dir_all(&parent);
    }

    #[test]
    fn jobs_kept_in_the_project_are_linked_by_the_projects_own_repository() {
        let parent = scratch("jobs-inside");
        let project = parent.join("shop");
        write_project(&project);
        std::fs::create_dir_all(project.join("orchestration")).unwrap();
        std::fs::write(project.join("orchestration/acme_daily.yml"), DAG_DAILY).unwrap();
        std::fs::create_dir_all(project.join(".dbt-edith")).unwrap();
        std::fs::write(project.join(".dbt-edith/jobs.yml"), "jobs:\n  - kind: airflow\n    path: orchestration\n").unwrap();
        let sha = commit_in(&project, Some("https://github.com/acme/shop.git"));
        let (index, err) = written(&project, &Ask { repo_url: None, commit: None, ..ask_for("") });
        assert_eq!(index["meta"]["jobs"]["sources"][0]["base"], format!("https://github.com/acme/shop/blob/{sha}/orchestration/"), "{err}");
        let _ = std::fs::remove_dir_all(&parent);
    }

    #[test]
    fn a_dotfiles_repository_above_the_project_is_not_the_jobs_repository() {
        // The home folder is a repository, the shop has its own, and the jobs were not cloned as one: git, asked from
        // the jobs' folder, would walk up to the home and name its commit and origin as the jobs'.
        let (parent, project, _) = jobs_project("jobs-dotfiles", "dags");
        commit_in(&parent, Some("git@github.com:acme/dotfiles.git"));
        commit_in(&project, Some("git@github.com:acme/shop.git"));
        let (index, err) = written(
            &project,
            &Ask {
                jobs_commit: Some("abcdef1".into()),
                jobs_repo_url: Some("https://github.com/acme/airflow".into()),
                ..ask_for("https://github.com/acme/shop")
            },
        );
        let s = &index["meta"]["jobs"]["sources"][0];
        // The flags still give the commit, and git is not asked: no file is linked, its place in the repository being a guess.
        assert_eq!(
            (s["base"].clone(), s["commit"].clone(), s["commit_url"].clone()),
            (Value::Null, json!("abcdef1"), json!("https://github.com/acme/airflow/commit/abcdef1")),
            "{s}"
        );
        assert!(
            err.contains("no job file links for orchestration: the repository it sits in holds the project's parent folders, not its own"),
            "{err}"
        );
        assert_eq!(index["meta"]["jobs"]["present"], 1, "the jobs are read all the same");

        // The rule itself: the project's own repository, or one that does not hold the project.
        let project_path = Path::new("/work/shop");
        assert!(repository_is_the_sources(Path::new("/work/shop"), project_path, Some(Path::new("/work/shop")), None));
        assert!(repository_is_the_sources(Path::new("/work/airflow"), project_path, Some(Path::new("/work/shop")), None));
        assert!(repository_is_the_sources(Path::new("/work/airflow"), project_path, None, None));
        assert!(!repository_is_the_sources(Path::new("/work"), project_path, Some(Path::new("/work/shop")), None));
        assert!(!repository_is_the_sources(Path::new("/"), project_path, None, None));
        let _ = std::fs::remove_dir_all(&parent);
    }

    /// `run_site` with the home folder named, as the system says it is.
    fn run_site_at(project: &Path, ask: &Ask, home: Option<&Path>) -> (i32, String, String) {
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = generate_at(project, None, None, ask, no_collin, home.map(Path::to_path_buf), &mut out, &mut err);
        (code, String::from_utf8(out).unwrap(), String::from_utf8(err).unwrap())
    }

    fn read_index(project: &Path) -> Value {
        read_script(&project.join("target/edith-site"), "data/index.js", "window.EDITH_SITE=")
    }

    #[test]
    fn the_jobs_flags_give_the_commit_with_no_repository_and_link_no_file() {
        let (parent, project, _) = jobs_project("jobs-norepo", "dags");
        let ask = |commit: Option<&str>, repo: Option<&str>| Ask {
            jobs_commit: commit.map(String::from),
            jobs_repo_url: repo.map(String::from),
            ..ask_for("https://github.com/acme/shop")
        };
        let (code, _, err) = run_site_at(&project, &ask(Some("abcdef1"), Some("https://github.com/acme/airflow")), None);
        assert_eq!(code, ls::ANSWERED, "{err}");
        let s = read_index(&project)["meta"]["jobs"]["sources"][0].clone();
        assert_eq!((s["commit"].clone(), s["base"].clone()), (json!("abcdef1"), Value::Null), "{s}");
        assert_eq!(s["commit_url"], "https://github.com/acme/airflow/commit/abcdef1");
        assert!(err.contains("no job file links for orchestration: not a git repository, the commit link is from the flags"), "{err}");
        // One flag alone is not a link: the page has no address for the commit.
        let (_, _, err) = run_site_at(&project, &ask(Some("abcdef1"), None), None);
        let s = read_index(&project)["meta"]["jobs"]["sources"][0].clone();
        assert_eq!((s["commit"].clone(), s["commit_url"].clone()), (json!(""), json!("")), "{s}");
        assert!(err.contains("not a git repository, pass --jobs-commit and --jobs-repo-url for the commit link"), "{err}");
        let _ = std::fs::remove_dir_all(&parent);
    }

    #[test]
    fn a_repository_at_the_home_folder_or_above_it_is_neither_the_projects_nor_the_jobs() {
        // `~` is a repository (dotfiles), the project is `~/work/shop` and the jobs are beside it, and neither has a repository of its own.
        let (parent, project, _) = jobs_project("jobs-home", "dags");
        let sha = commit_in(&parent, Some("git@github.com:acme/dotfiles.git"));
        let bare = Ask { commit: None, repo_url: None, ..ask_for("") };

        let (code, _, err) = run_site_at(&project, &bare, Some(&parent));
        assert_eq!(code, ls::ANSWERED, "{err}");
        let index = read_index(&project);
        let meta = &index["meta"];
        assert_eq!(
            (meta["commit"].clone(), meta["commit_url"].clone(), meta["file_base"].clone()),
            (json!(""), json!(""), Value::Null),
            "{meta}"
        );
        assert!(err.contains("git is not read for the project: the repository it sits in is at the home folder or above it"), "{err}");
        let s = &meta["jobs"]["sources"][0];
        assert_eq!((s["commit"].clone(), s["base"].clone()), (json!(""), Value::Null), "{s}");
        assert!(err.contains("no job file links for orchestration: the repository it sits in is at the home folder or above it"), "{err}");
        // A repository above the home folder is above it as well.
        let deeper = parent.join("work");
        std::fs::create_dir_all(&deeper).unwrap();
        let (_, _, err) = run_site_at(&project, &bare, Some(&deeper));
        assert!(err.contains("git is not read for the project"), "{err}");

        // The flags still give what they can, and no file is linked: the prefix would be the machine's own layout.
        let flags = Ask {
            jobs_commit: Some("abcdef1".into()),
            jobs_repo_url: Some("https://github.com/acme/airflow".into()),
            ..ask_for("https://github.com/acme/shop")
        };
        run_site_at(&project, &flags, Some(&parent));
        let meta = read_index(&project)["meta"].clone();
        assert_eq!(meta["commit_url"], "https://github.com/acme/shop/commit/0123456789abcdef0123456789abcdef01234567");
        assert_eq!((meta["file_base"].clone(), meta["jobs"]["sources"][0]["base"].clone()), (Value::Null, Value::Null));

        // With no home folder known, as `resolve_real` has none to check, the repository is the project's and the jobs' own.
        let (_, _, err) = run_site_at(&project, &bare, None);
        let meta = read_index(&project)["meta"].clone();
        assert_eq!(meta["commit"], sha, "{err}");
        assert_eq!(meta["file_base"], format!("https://github.com/acme/dotfiles/blob/{sha}/shop/"), "{err}");
        let s = &meta["jobs"]["sources"][0];
        assert_eq!(
            (s["commit"].clone(), s["base"].clone()),
            (json!(&sha[..7]), json!(format!("https://github.com/acme/dotfiles/blob/{sha}/airflow_repo/dags/")))
        );
        // And one elsewhere is no one's home.
        let elsewhere = scratch("jobs-home-elsewhere");
        let (_, _, err) = run_site_at(&project, &bare, Some(&elsewhere));
        assert!(!err.contains("home folder"), "{err}");
        let _ = std::fs::remove_dir_all(&parent);
        let _ = std::fs::remove_dir_all(&elsewhere);
    }
}
