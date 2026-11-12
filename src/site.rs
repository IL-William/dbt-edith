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
//! and the cards in buckets loaded when a node is first opened.

use crate::api::{self, Assets};
use crate::graph::{Graph, Kind, MAX_LEVELS};
use crate::{git, ls};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::{Component, Path, PathBuf};

/// What `--site` was given besides the project, the manifest and the catalog.
pub struct Ask {
    /// None for the default, `<project>/target/edith-site`.
    pub dir: Option<PathBuf>,
    pub environment: Option<String>,
    pub commit: Option<String>,
    pub repo_url: Option<String>,
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
    // What can be refused without reading anything is refused first.
    let flags = (|| -> Result<_, String> {
        let commit = ask.commit.as_deref().map(valid_commit).transpose()?;
        let repo = ask.repo_url.as_deref().map(repo_from_flag).transpose()?;
        let environment = ask.environment.as_deref().map(valid_environment).transpose()?;
        Ok((commit, repo, environment))
    })();
    let (commit_flag, repo_flag, environment) = match flags {
        Ok(f) => f,
        Err(why) => {
            let _ = writeln!(err, "error: {why}");
            return ls::CANNOT_RUN;
        }
    };
    let (root, manifest_path, graph) = match ls::open_with_catalog(project, manifest, catalog) {
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

    // Whether it was merged, not whether a file is there: one that did not read is left out of the cards.
    let has_catalog = graph.meta.catalog_mtime > 0 || graph.meta.catalog_columns > 0;
    if !ls::catalog_path(&root, &manifest_path, catalog).is_file() {
        let _ = writeln!(err, "no catalog.json: columns keep the types the YAML declares");
    }
    let commit = match commit_flag {
        Some(c) => Some(c),
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

    let meta = json!({
        "project": graph.meta.project,
        "environment": environment.clone().unwrap_or_default(),
        "commit": commit.clone().unwrap_or_default(),
        "commit_url": commit_url,
        "repo_url": repo.as_ref().map(|r| r.url.clone()).unwrap_or_default(),
        "generated_at": graph.meta.generated_at,
        "dbt_version": graph.meta.dbt_version,
        "adapter": graph.meta.adapter,
        "counts": graph.meta.counts,
        "catalog": has_catalog,
        "edith": env!("CARGO_PKG_VERSION"),
    });
    let title = title_of(&graph.meta.project, environment.as_deref().unwrap_or(""));
    let built = match build(&graph, meta, &title) {
        Ok(b) => b,
        Err(why) => {
            let _ = writeln!(err, "error: {why}");
            return ls::CANNOT_RUN;
        }
    };
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
    ls::ANSWERED
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
        Host::Other => "commit",
    };
    format!("{}/{pattern}/{sha}", repo.url)
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
    /// The assets the page loads, by their path in the folder.
    pub assets: Vec<(String, Vec<u8>)>,
}

/// Every node's card, in contiguous runs of node numbers that close at
/// `max_nodes` cards or `max_bytes` of text, whichever comes first: the page
/// loads a run when one of its cards is first opened. Where each run starts, and
/// the text of its file.
fn cards_in(graph: &Graph, max_nodes: usize, max_bytes: usize) -> Result<(Vec<usize>, Vec<String>), String> {
    let mut cuts: Vec<usize> = Vec::new();
    let mut buckets: Vec<String> = Vec::new();
    let mut run: Vec<String> = Vec::new();
    let mut bytes = 0;
    let mut start = 0;
    let flush = |run: &mut Vec<String>, start: usize, buckets: &mut Vec<String>| {
        buckets.push(format!("EDITH_SITE_CARDS({start},[{}]);\n", run.join(",")));
        run.clear();
    };
    for i in 0..graph.nodes.len() {
        if run.is_empty() {
            start = i;
            cuts.push(i);
            bytes = 0;
        }
        let card = js_json(&serde_json::to_value(api::node_detail(graph, i as u32, None, None)).map_err(|e| e.to_string())?);
        bytes += card.len() + 1;
        run.push(card);
        if run.len() >= max_nodes || bytes >= max_bytes {
            flush(&mut run, start, &mut buckets);
        }
    }
    if !run.is_empty() {
        flush(&mut run, start, &mut buckets);
    }
    Ok((cuts, buckets))
}

/// Lays out the index and the cards, and rewrites the page: everything the
/// folder holds, as text, before any of it is written.
pub(crate) fn build(graph: &Graph, meta: Value, title: &str) -> Result<Built, String> {
    let mut index = index_of(graph, meta);
    let (cuts, buckets) = cards_in(graph, BUCKET_NODES, BUCKET_BYTES)?;
    index["cuts"] = json!(cuts);

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
    Ok(Built { html, index: format!("window.EDITH_SITE={};\n", js_json(&index)), buckets, assets })
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
        let ask = Ask { dir: Some(into.clone()), environment: None, commit: None, repo_url: None };
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
        let extra: BTreeSet<&String> = written.keys().filter(|p| !loaded.contains(*p) && !p.starts_with("data/cards/")).collect();
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
        let first = files_of(&dir);
        assert!(first.contains_key(MARKER) && first.contains_key("data/index.js") && first.contains_key("data/cards/0000.js"));
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
    fn case_graph() -> Graph {
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
        let raw: crate::manifest::RawManifest = serde_json::from_value(manifest).unwrap();
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
        graph
    }

    /// The whole fixture, as the text of the file. What `web/site.js` is held to.
    fn case_text() -> String {
        let graph = case_graph();
        let meta = json!({ "project": "shop", "environment": "qa", "commit": "", "commit_url": "", "repo_url": "", "catalog": false });
        let mut index = index_of(&graph, meta);
        // Small buckets, so the lookup of a card by its number is exercised.
        let (cuts, buckets) = cards_in(&graph, 7, BUCKET_BYTES).unwrap();
        index["cuts"] = json!(cuts);

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
            ("chain", "model", Some(200_000)),
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
        let nodes: Vec<Value> = ["model.shop.dia_a", "model.shop.orders.v1", "exposure.shop.dashboard", "model.shop.old_hub"]
            .iter()
            .map(|name| {
                let want = serde_json::to_value(api::node_detail(&graph, id(name), None, None)).unwrap();
                json!({ "path": format!("/api/node?id={}", enc(name)), "want": want })
            })
            .collect();

        let case = json!({ "site": index, "buckets": buckets, "lineage": cases, "search": searches, "node": nodes, "dir": dirs });
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
        let (cuts, buckets) = cards_in(&graph, 7, BUCKET_BYTES).unwrap();
        assert_eq!(cuts, (0..n).step_by(7).collect::<Vec<_>>());
        assert_eq!(buckets.len(), cuts.len());
        // The byte limit closes a bucket as the count does.
        let (cuts, _) = cards_in(&graph, 1000, 1).unwrap();
        assert_eq!(cuts.len(), n);
    }
}
