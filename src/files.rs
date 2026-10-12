//! Filesystem access, confined to the opened project directory.

use std::path::{Path, PathBuf};

const IGNORED: &[&str] = &[".git", ".DS_Store", "node_modules", "__pycache__", ".pytest_cache", ".mypy_cache", ".ruff_cache"];
/// Skipped by the file index only: the tree still browses them, but they are
/// generated or vendored and would drown a search. `target` alone holds more
/// files than the whole project.
const NOT_INDEXED: &[&str] = &["target", "logs", ".git", "node_modules", "__pycache__", ".pytest_cache", ".mypy_cache", ".ruff_cache", ".cortex"];
const MAX_INDEXED: usize = 60_000;
const MAX_READ_BYTES: u64 = 4 * 1024 * 1024;

/// Seconds since the epoch, or 0 when the file is missing or unreadable. Every
/// freshness comparison in the tool is between two of these, so a missing file
/// reads as infinitely old rather than as an error to thread through.
pub fn mtime_secs(path: &Path) -> u64 {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// A file parsed once and kept until its size or modification time changes:
/// dbt's artifacts run to megabytes, and the page asks again on every showing.
/// None when the file is not there, and the parser's error when it is there
/// and cannot be read, as when dbt is still writing it. The error is kept as
/// an answer is, so a file left broken is not read again on every poll.
#[allow(clippy::type_complexity)]
pub struct Cached<T>(std::sync::Mutex<Option<(u64, Option<std::time::SystemTime>, Result<std::sync::Arc<T>, String>)>>);

impl<T> Default for Cached<T> {
    fn default() -> Self {
        Cached(std::sync::Mutex::new(None))
    }
}

impl<T> Cached<T> {
    pub fn get(&self, path: &Path, parse: impl FnOnce(&[u8]) -> Result<T, String>) -> Result<Option<std::sync::Arc<T>>, String> {
        let mut slot = self.0.lock().unwrap_or_else(|e| e.into_inner());
        let Ok(meta) = std::fs::metadata(path) else {
            *slot = None;
            return Ok(None);
        };
        let (len, modified) = (meta.len(), meta.modified().ok());
        if let Some((l, m, held)) = slot.as_ref() {
            if *l == len && *m == modified {
                return held.clone().map(Some);
            }
        }
        let parsed = std::fs::read(path).map_err(|e| e.to_string()).and_then(|bytes| parse(&bytes)).map(std::sync::Arc::new);
        *slot = Some((len, modified, parsed.clone()));
        parsed.map(Some)
    }
}

/// A canonical path in the spelling other programs accept. On Windows,
/// `canonicalize` answers `\\?\C:\...`, and cmd.exe refuses that as a working
/// directory: it falls back to `C:\Windows`, so dbt started from the terminal
/// through a `.bat` shim, the way pyenv-win installs it, wrote its log there.
/// Nothing is lost by dropping the prefix: Rust puts it back by itself on a
/// path too long to go without.
pub fn plain(path: PathBuf) -> PathBuf {
    let Some(s) = path.to_str() else { return path };
    if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
        return PathBuf::from(format!(r"\\{rest}"));
    }
    match s.strip_prefix(r"\\?\") {
        Some(rest) if rest.as_bytes().get(1) == Some(&b':') => PathBuf::from(rest),
        _ => path,
    }
}

/// Turns a project-relative path into an absolute one, refusing anything that
/// would escape the project root (`..`, absolute paths, symlinks pointing out).
pub fn resolve(root: &Path, rel: &str) -> anyhow::Result<PathBuf> {
    let out = joined(root, rel)?;
    confined(root, &root.canonicalize()?, out)
}

/// `rel` under `root`, refused when its spelling alone leaves it.
fn joined(root: &Path, rel: &str) -> anyhow::Result<PathBuf> {
    let mut out = root.to_path_buf();
    for seg in rel.replace('\\', "/").split('/') {
        match seg {
            "" | "." => continue,
            ".." => anyhow::bail!("path escapes the project root"),
            // A drive letter: on Windows `push` would replace the root with it.
            s if s.ends_with(':') => anyhow::bail!("path escapes the project root"),
            s => out.push(s),
        }
    }
    Ok(out)
}

/// `out`, refused when a link takes it out of the project. Apart from `resolve`
/// so a search can canonicalize the root once rather than once per file.
fn confined(root: &Path, canon_root: &Path, out: PathBuf) -> anyhow::Result<PathBuf> {
    // A path that does not exist yet (a new file, or a deleted one being
    // diffed) is judged by its nearest existing ancestor.
    let mut probe = out.as_path();
    while !probe.exists() {
        probe = probe.parent().unwrap_or(root);
    }
    let canon = probe.canonicalize()?;
    if !canon.starts_with(canon_root) {
        anyhow::bail!("path escapes the project root");
    }
    Ok(out)
}

#[derive(serde::Serialize)]
pub struct Entry {
    pub name: String,
    pub path: String,
    pub dir: bool,
    pub size: u64,
}

pub fn list_dir(root: &Path, rel: &str) -> anyhow::Result<Vec<Entry>> {
    let dir = resolve(root, rel)?;
    let prefix = rel.trim_matches('/');
    let mut out = Vec::new();
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if IGNORED.contains(&name.as_str()) {
            continue;
        }
        let meta = entry.metadata()?;
        out.push(Entry {
            path: if prefix.is_empty() { name.clone() } else { format!("{prefix}/{name}") },
            name,
            dir: meta.is_dir(),
            size: if meta.is_dir() { 0 } else { meta.len() },
        });
    }
    out.sort_by(|a, b| b.dir.cmp(&a.dir).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    Ok(out)
}

#[derive(serde::Serialize)]
pub struct FileBody {
    pub path: String,
    pub content: String,
    pub size: u64,
    pub truncated: bool,
    /// The version of the file this content was read from, for the page to
    /// tell when the disk has moved on without it (`stamp`).
    pub stamp: String,
}

/// What tells one version of a file from the next without reading it: its size
/// and modification time, to the nanosecond where the filesystem keeps them. A
/// `git pull`, a checkout or another editor rewriting an open file all move it.
/// A string, because nanoseconds since the epoch do not fit a JavaScript number.
/// None when there is no file there.
pub fn stamp(path: &Path) -> Option<String> {
    let meta = std::fs::metadata(path).ok()?;
    meta.is_file().then(|| stamp_of(&meta))
}

fn stamp_of(meta: &std::fs::Metadata) -> String {
    let at = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{}-{at}", meta.len())
}

pub fn read_file(root: &Path, rel: &str) -> anyhow::Result<FileBody> {
    let path = resolve(root, rel)?;
    // Taken before the read, so a write landing in between leaves the stamp
    // older than the content: the page then reads the file once more, and
    // finds nothing new, rather than holding a stamp for content it never saw.
    let meta = std::fs::metadata(&path)?;
    let size = meta.len();
    let bytes = if size > MAX_READ_BYTES {
        use std::io::Read;
        let mut buf = vec![0u8; MAX_READ_BYTES as usize];
        let mut f = std::fs::File::open(&path)?;
        let n = f.read(&mut buf)?;
        buf.truncate(n);
        buf
    } else {
        std::fs::read(&path)?
    };
    if bytes.contains(&0) {
        anyhow::bail!("binary file");
    }
    Ok(FileBody {
        path: rel.to_string(),
        content: String::from_utf8_lossy(&bytes).into_owned(),
        size,
        truncated: size > MAX_READ_BYTES,
        stamp: stamp_of(&meta),
    })
}

/// Flat list of project-relative file paths, for the search palette.
/// Virtualenvs are skipped wholesale: they hold thousands of vendored files and
/// nobody searches for one by name.
pub fn scan(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![(root.to_path_buf(), String::new())];
    while let Some((dir, prefix)) = stack.pop() {
        if out.len() >= MAX_INDEXED {
            break;
        }
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            // Dotfiles stay in: .env, .sqlfluff and .github are all things
            // people open. Only the generated directories are skipped.
            if NOT_INDEXED.contains(&name.as_str()) || name == ".DS_Store" {
                continue;
            }
            let rel = if prefix.is_empty() { name.clone() } else { format!("{prefix}/{name}") };
            let path = entry.path();
            let Ok(meta) = entry.metadata() else { continue };
            if meta.is_dir() {
                if path.join("pyvenv.cfg").exists() {
                    continue; // a virtualenv
                }
                stack.push((path, rel));
            } else if meta.is_file() {
                out.push(rel);
            }
        }
    }
    out.sort();
    out
}

/// Ranked substring search over the index. Deliberately not fuzzy: with paths
/// this long, a subsequence match returns everything and ranks nothing.
// ------------------------------------------------------------ content search --

/// A search reads files rather than path strings, so it needs its own ceilings.
/// Generous enough for a dbt project, low enough that one query cannot walk a
/// repository for a minute.
const GREP_MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;
const GREP_MAX_SCANNED: usize = 20_000;
const GREP_LINE_CHARS: usize = 200;
/// Opening and reading the files is nearly all of a search's time, and the
/// system serves several at once: eight threads took a search of a 10 000 file
/// project from 550 ms to 165 ms on a Mac.
const GREP_THREADS: usize = 8;
/// Files are read a batch at a time and their results kept in index order, so
/// the ceilings fall where they did when one thread read them all. The first
/// batch is small because a common word reaches the file ceiling early, and
/// what a batch reads past it is wasted.
const GREP_FIRST_BATCH: usize = 64;
const GREP_LAST_BATCH: usize = 1024;

fn is_zero(n: &usize) -> bool {
    *n == 0
}

#[derive(Debug, PartialEq, serde::Serialize)]
pub struct Hit {
    /// 1-based, so it can be shown and jumped to without arithmetic.
    pub line: usize,
    /// The matching line, indentation dropped and clipped around the match.
    /// Where the match falls inside it is left to the browser to find: it knows
    /// the query, and a byte offset from here would not survive the trip into a
    /// UTF-16 string anyway.
    pub text: String,
}

#[derive(Debug, PartialEq, serde::Serialize)]
pub struct FileHits {
    pub path: String,
    pub hits: Vec<Hit>,
    /// Matches past the per-file cap: counted, not listed.
    #[serde(skip_serializing_if = "is_zero")]
    pub more: usize,
}

#[derive(Debug, Default, PartialEq, serde::Serialize)]
pub struct GrepResult {
    pub files: Vec<FileHits>,
    pub total: usize,
    /// The search stopped at a ceiling rather than running out of files.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub capped: bool,
    /// Files passed over for being binary, unreadable or too large.
    #[serde(skip_serializing_if = "is_zero")]
    pub skipped: usize,
}

/// A `.env` file is the one thing a content search must not open. The editor
/// opens it when asked, which is what an editor is for; putting its values in a
/// list of results nobody asked for is a different act (0012, 0019, 0020).
pub fn searchable(rel: &str) -> bool {
    let name = rel.rsplit('/').next().unwrap_or(rel);
    name != ".env" && !name.starts_with(".env.")
}

/// Text, as far as a search is concerned. A NUL byte early on is what every
/// grep uses, and it is right often enough.
fn looks_binary(bytes: &[u8]) -> bool {
    bytes.iter().take(8000).any(|b| *b == 0)
}

/// Case-insensitive substring search, returning a byte offset valid in `hay`.
/// `needle` is already lowercase. Walking `hay`'s own char boundaries rather
/// than indexing into `hay.to_lowercase()`, because lowercasing can change a
/// string's length and the offset would then land mid-character.
fn find_ci(hay: &str, needle: &str) -> Option<usize> {
    if needle.is_empty() || needle.len() > hay.len() {
        return None;
    }
    // Almost every line of a dbt project is ASCII, and the general path below
    // walks an iterator per character position, which is what makes a search of
    // 12 000 files take seconds rather than a fraction of one.
    if hay.is_ascii() && needle.is_ascii() {
        let (h, n) = (hay.as_bytes(), needle.as_bytes());
        return (0..=h.len() - n.len())
            .find(|i| h[*i..*i + n.len()].iter().zip(n).all(|(a, b)| a.to_ascii_lowercase() == *b));
    }
    for (i, _) in hay.char_indices() {
        let mut got = hay[i..].chars().flat_map(char::to_lowercase);
        let mut want = needle.chars();
        loop {
            match (want.next(), got.next()) {
                (None, _) => return Some(i),
                (Some(_), None) => return None,
                (Some(a), Some(b)) if a == b => continue,
                _ => break,
            }
        }
    }
    None
}

/// The matching line with its indentation dropped, clipped around the match so
/// one minified file cannot send a megabyte to the browser.
fn excerpt(line: &str, at: usize) -> String {
    let lead = line.len() - line.trim_start().len();
    let line = &line[lead..];
    let at = at.saturating_sub(lead);
    if line.chars().count() <= GREP_LINE_CHARS {
        return line.to_string();
    }
    // Start a little before the match so it is not flush against the ellipsis.
    let start = line[..at].char_indices().rev().nth(30).map_or(0, |(i, _)| i);
    let end = line[start..].char_indices().nth(GREP_LINE_CHARS).map_or(line.len(), |(i, _)| start + i);
    let mut out = String::new();
    if start > 0 {
        out.push('\u{2026}');
    }
    out.push_str(&line[start..end]);
    if end < line.len() {
        out.push('\u{2026}');
    }
    out
}

/// Case-insensitive search for a word across the indexed files. Hand-written
/// rather than a regex (0003): the query is a word someone typed, not a pattern.
pub fn grep(root: &Path, index: &[String], query: &str, max_files: usize, per_file: usize) -> GrepResult {
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get()).min(GREP_THREADS);
    grep_with(root, index, query, max_files, per_file, threads)
}

fn grep_with(root: &Path, index: &[String], query: &str, max_files: usize, per_file: usize, threads: usize) -> GrepResult {
    let needle = query.trim().to_lowercase();
    let mut out = GrepResult::default();
    if needle.is_empty() {
        return out;
    }
    // Every file would fail to resolve without it.
    let Ok(canon_root) = root.canonicalize() else { return out };
    let mut scanned = 0;
    let (mut at, mut batch) = (0, GREP_FIRST_BATCH);
    while at < index.len() {
        let rels = &index[at..index.len().min(at + batch)];
        for (rel, read) in rels.iter().zip(search_batch(root, &canon_root, rels, &needle, per_file, threads)) {
            if out.files.len() >= max_files || scanned >= GREP_MAX_SCANNED {
                out.capped = true;
                return out;
            }
            match read {
                Searched::Passed => {}
                Searched::TooBig => out.skipped += 1,
                Searched::Unread => {
                    scanned += 1;
                    out.skipped += 1;
                }
                Searched::Read { hits, more, total } => {
                    scanned += 1;
                    out.total += total;
                    if !hits.is_empty() {
                        out.files.push(FileHits { path: rel.clone(), hits, more });
                    }
                }
            }
        }
        at += rels.len();
        batch = (batch * 2).min(GREP_LAST_BATCH);
    }
    out
}

/// What reading one file for a search came to. Counted by the caller, in index
/// order, so the ceilings and the totals do not depend on which thread read it.
enum Searched {
    /// Not searched, and not counted: an `.env` file, or one gone since the
    /// index was built.
    Passed,
    /// Over the size ceiling, so never opened.
    TooBig,
    /// Opened, but unreadable or binary.
    Unread,
    Read { hits: Vec<Hit>, more: usize, total: usize },
}

/// One batch of files, read by `threads` threads taking the next file as each
/// finishes, returned in the batch's order.
fn search_batch(root: &Path, canon_root: &Path, rels: &[String], needle: &str, per_file: usize, threads: usize) -> Vec<Searched> {
    let one = |rel: &String| search_file(root, canon_root, rel, needle, per_file);
    if threads < 2 || rels.len() < 2 {
        return rels.iter().map(one).collect();
    }
    let next = std::sync::atomic::AtomicUsize::new(0);
    let mut read: Vec<(usize, Searched)> = std::thread::scope(|s| {
        let workers: Vec<_> = (0..threads.min(rels.len()))
            .map(|_| {
                s.spawn(|| {
                    let mut mine = Vec::new();
                    loop {
                        let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        let Some(rel) = rels.get(i) else { break mine };
                        mine.push((i, one(rel)));
                    }
                })
            })
            .collect();
        workers.into_iter().flat_map(|w| w.join().unwrap_or_else(|e| std::panic::resume_unwind(e))).collect()
    });
    read.sort_by_key(|(i, _)| *i);
    read.into_iter().map(|(_, r)| r).collect()
}

fn search_file(root: &Path, canon_root: &Path, rel: &str, needle: &str, per_file: usize) -> Searched {
    if !searchable(rel) {
        return Searched::Passed;
    }
    let Ok(path) = joined(root, rel).and_then(|p| confined(root, canon_root, p)) else { return Searched::Passed };
    let Ok(meta) = std::fs::metadata(&path) else { return Searched::Passed };
    if !meta.is_file() {
        return Searched::Passed;
    }
    if meta.len() > GREP_MAX_FILE_BYTES {
        return Searched::TooBig;
    }
    let Ok(bytes) = std::fs::read(&path) else { return Searched::Unread };
    if looks_binary(&bytes) {
        return Searched::Unread;
    }
    let text = crate::envs::decode(&bytes);
    let (mut hits, mut more, mut total) = (Vec::new(), 0, 0);
    // Most files hold no match at all. One pass over the whole text is far
    // cheaper than splitting it into lines and testing each of them.
    if find_ci(&text, needle).is_some() {
        for (i, line) in text.lines().enumerate() {
            let Some(at) = find_ci(line, needle) else { continue };
            total += 1;
            if hits.len() >= per_file {
                more += 1;
                continue;
            }
            hits.push(Hit { line: i + 1, text: excerpt(line, at) });
        }
    }
    Searched::Read { hits, more, total }
}

pub fn search_paths<'a>(paths: &'a [String], query: &str, limit: usize) -> Vec<&'a str> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return paths.iter().take(limit).map(String::as_str).collect();
    }
    let mut hits: Vec<(u32, usize, &str)> = Vec::new();
    for p in paths {
        let lower = p.to_lowercase();
        let base = lower.rsplit('/').next().unwrap_or(&lower);
        let score = if base == q {
            0
        } else if base.starts_with(&q) {
            10
        } else if base.contains(&q) {
            20
        } else if lower.contains(&q) {
            30
        } else {
            continue;
        };
        // Vendored packages rank below the project's own files.
        let vendored = lower.starts_with("dbt_packages/") as u32 * 100;
        hits.push((score + vendored, p.len(), p.as_str()));
    }
    hits.sort_unstable();
    hits.truncate(limit);
    hits.into_iter().map(|(_, _, p)| p).collect()
}

/// What a save found. `Changed` is a file that is no longer the version the
/// page read, written meanwhile by a pull, a checkout or another program,
/// with its stamp now, None when it is gone. It is left as it is.
pub enum Saved {
    Written(String),
    Changed(Option<String>),
}

/// One save at a time, its check and its write together: two saves of one
/// version, from two windows or a double press, cannot both pass the check.
/// A pull is another program and takes no lock of ours, so it can still land
/// in the instant between the two.
static WRITING: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// `seen` is the stamp of the version the edit started from. Given, the file
/// is written only if it is still that version, so a save never puts back what
/// a pull just changed without the page saying so first. An empty one is a
/// file that was gone, written only if it still is. None writes whatever is
/// there.
pub fn write_file(root: &Path, rel: &str, content: &str, seen: Option<&str>) -> anyhow::Result<Saved> {
    use std::io::Write;
    let path = resolve(root, rel)?;
    let _alone = WRITING.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(seen) = seen {
        let now = stamp(&path);
        if now.as_deref().unwrap_or("") != seen {
            return Ok(Saved::Changed(now));
        }
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = std::fs::File::create(&path)?;
    file.write_all(content.as_bytes())?;
    // From the handle, so it is the stamp of what was written here: one read
    // by path could already be a pull's, landed just after, and the tab would
    // take it for its own and never hear of the pull.
    Ok(Saved::Written(stamp_of(&file.metadata()?)))
}

/// Whether this directory looks like the root of a dbt project. Used only to
/// warn at startup: dbt-edith opens whatever it is given, since a monorepo or a
/// manifest passed with `--manifest` is a legitimate way to work.
pub fn is_dbt_project(root: &Path) -> bool {
    root.join("dbt_project.yml").is_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_verbatim_path_is_spelled_the_way_cmd_reads_it() {
        let plain = |s: &str| super::plain(PathBuf::from(s)).to_string_lossy().into_owned();
        assert_eq!(plain(r"\\?\C:\Users\me\shop"), r"C:\Users\me\shop");
        assert_eq!(plain(r"\\?\UNC\server\share\shop"), r"\\server\share\shop");
        // A device or volume path has no shorter spelling, and is left alone.
        assert_eq!(plain(r"\\?\Volume{1234}\shop"), r"\\?\Volume{1234}\shop");
        assert_eq!(plain("/Users/me/shop"), "/Users/me/shop");
    }

    fn grep_dir(tag: &str, files: &[(&str, &[u8])]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dbt-edith-grep-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for (rel, bytes) in files {
            let path = dir.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, bytes).unwrap();
        }
        dir
    }

    fn index(files: &[(&str, &[u8])]) -> Vec<String> {
        files.iter().map(|(r, _)| (*r).to_string()).collect()
    }

    #[test]
    fn a_stamp_belongs_to_a_file_that_is_there() {
        let dir = grep_dir("stamp", &[("models/a.sql", b"select 1\n")]);
        assert!(stamp(&dir.join("models/a.sql")).is_some_and(|s| s.starts_with("9-")), "its size first");
        assert_eq!(stamp(&dir.join("models")), None, "a folder has none");
        assert_eq!(stamp(&dir.join("models/b.sql")), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn grep_finds_a_word_whatever_its_case() {
        let files: &[(&str, &[u8])] = &[
            ("models/a.sql", b"select BrokerCode\nfrom t\n"),
            ("models/b.sql", b"-- nothing here\n"),
        ];
        let dir = grep_dir("case", files);
        let r = grep(&dir, &index(files), "brokercode", 50, 10);
        assert_eq!(r.total, 1);
        assert_eq!(r.files.len(), 1);
        assert_eq!(r.files[0].path, "models/a.sql");
        assert_eq!(r.files[0].hits[0].line, 1, "line numbers are 1-based");
        assert_eq!(r.files[0].hits[0].text, "select BrokerCode");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn grep_never_opens_an_env_file() {
        // The one exclusion the feature exists under (0020).
        let files: &[(&str, &[u8])] = &[
            (".env", b"SNOWFLAKE_PASSWORD=hunter2\n"),
            (".env.uat", b"SNOWFLAKE_PASSWORD=hunter2\n"),
            ("models/a.sql", b"-- hunter2 is written here on purpose\n"),
        ];
        let dir = grep_dir("env", files);
        let r = grep(&dir, &index(files), "hunter2", 50, 10);
        assert_eq!(r.files.len(), 1, "only the model, never the .env files");
        assert_eq!(r.files[0].path, "models/a.sql");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn searchable_names_only_env_files() {
        assert!(!searchable(".env"));
        assert!(!searchable(".env.uat"));
        assert!(!searchable("config/.env.local"));
        assert!(searchable("models/.environment.sql"), "a name that merely starts the same");
        assert!(searchable("models/env.sql"));
        assert!(searchable("dbt_project.yml"));
    }

    #[test]
    fn grep_skips_binaries_and_oversized_files() {
        let big = vec![b'x'; (GREP_MAX_FILE_BYTES + 1) as usize];
        let files: &[(&str, &[u8])] = &[
            ("data/blob.bin", b"xx\0xx needle\n"),
            ("models/a.sql", b"needle\n"),
        ];
        let dir = grep_dir("skip", files);
        std::fs::write(dir.join("big.sql"), &big).unwrap();
        let mut idx = index(files);
        idx.push("big.sql".to_string());
        let r = grep(&dir, &idx, "needle", 50, 10);
        assert_eq!(r.files.len(), 1);
        assert_eq!(r.files[0].path, "models/a.sql");
        assert_eq!(r.skipped, 2, "the binary and the oversized one");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Several threads read the files, but the answer is the one a single
    /// thread gives, the ceilings included: they fall in index order wherever a
    /// batch ends. Past one batch on purpose, with every kind of file a search
    /// passes over or counts as skipped, and two index entries no longer there.
    #[test]
    fn grep_answers_the_same_on_one_thread_or_several() {
        let mut files: Vec<(String, Vec<u8>)> = Vec::new();
        for i in 0..300 {
            let body = match i % 7 {
                0 => b"select needle\nfrom t\n-- NEEDLE again\n".to_vec(),
                1 => b"xx\0 needle\n".to_vec(),
                2 => "needle\n".repeat(i % 5 + 1).into_bytes(),
                _ => b"select 1\n".to_vec(),
            };
            files.push((format!("models/m{i:03}.sql"), body));
        }
        files.push((".env.uat".into(), b"PASSWORD=needle\n".to_vec()));
        files.push(("models/big.sql".into(), vec![b'x'; (GREP_MAX_FILE_BYTES + 1) as usize]));
        let borrowed: Vec<(&str, &[u8])> = files.iter().map(|(r, b)| (r.as_str(), b.as_slice())).collect();
        let dir = grep_dir("threads", &borrowed);
        let mut idx = index(&borrowed);
        idx.extend(["models/gone.sql".to_string(), "models".to_string()]);
        idx.sort();
        for max_files in [1, 5, 63, 64, 65, 120, 500] {
            for per_file in [1, 3] {
                let one = grep_with(&dir, &idx, "needle", max_files, per_file, 1);
                let many = grep_with(&dir, &idx, "needle", max_files, per_file, 8);
                assert_eq!(one, many, "max_files {max_files}, per_file {per_file}");
            }
        }
        let all = grep_with(&dir, &idx, "needle", 500, 3, 8);
        assert!(!all.capped);
        assert_eq!(all.skipped, 44, "43 binaries and the oversized file");
        assert_eq!(all.files.len(), 86);
        assert!(grep_with(&dir, &idx, "needle", 64, 3, 8).capped);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn grep_counts_matches_past_the_per_file_cap() {
        let files: &[(&str, &[u8])] = &[("models/a.sql", b"hit\nhit\nhit\nhit\n")];
        let dir = grep_dir("cap", files);
        let r = grep(&dir, &index(files), "hit", 50, 2);
        assert_eq!(r.files[0].hits.len(), 2);
        assert_eq!(r.files[0].more, 2);
        assert_eq!(r.total, 4, "total counts every match, listed or not");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn grep_stops_at_the_file_ceiling_and_says_so() {
        let files: &[(&str, &[u8])] =
            &[("a.sql", b"x\n"), ("b.sql", b"x\n"), ("c.sql", b"x\n")];
        let dir = grep_dir("ceiling", files);
        let r = grep(&dir, &index(files), "x", 2, 10);
        assert_eq!(r.files.len(), 2);
        assert!(r.capped);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn grep_drops_indentation_and_clips_a_long_line() {
        let long = format!("        {}needle{}\n", "a".repeat(400), "b".repeat(400));
        let files: &[(&str, &[u8])] = &[("models/a.sql", b"")];
        let dir = grep_dir("clip", files);
        std::fs::write(dir.join("models/a.sql"), long.as_bytes()).unwrap();
        let r = grep(&dir, &index(files), "needle", 50, 10);
        let text = &r.files[0].hits[0].text;
        assert!(text.contains("needle"), "the match stays in view");
        assert!(!text.starts_with(' '), "indentation is dropped");
        assert!(text.chars().count() <= GREP_LINE_CHARS + 2, "clipped, got {}", text.chars().count());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn grep_reads_crlf_and_a_bom_like_the_editor_does() {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(b"select 1\r\nwhere broker = 1\r\n");
        let files: &[(&str, &[u8])] = &[("models/a.sql", b"")];
        let dir = grep_dir("crlf", files);
        std::fs::write(dir.join("models/a.sql"), &bytes).unwrap();
        let r = grep(&dir, &index(files), "broker", 50, 10);
        assert_eq!(r.files[0].hits[0].line, 2);
        assert_eq!(r.files[0].hits[0].text, "where broker = 1");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn grep_ignores_an_empty_query() {
        let files: &[(&str, &[u8])] = &[("a.sql", b"anything\n")];
        let dir = grep_dir("empty", files);
        assert_eq!(grep(&dir, &index(files), "   ", 50, 10), GrepResult::default());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn find_ci_offsets_land_on_a_character_boundary() {
        // Lowercasing can change a string's length, so an index into the
        // lowercased copy would not be valid here.
        let line = "SELECT \u{130}STANBUL, brokercode";
        let at = find_ci(line, "brokercode").unwrap();
        assert!(line.is_char_boundary(at));
        assert_eq!(&line[at..at + 10], "brokercode");
        assert_eq!(find_ci("abc", "zzz"), None);
    }

    #[test]
    fn a_dbt_project_is_recognised_by_its_project_file() {
        let dir = std::env::temp_dir().join(format!("dbt-edith-files-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        assert!(!is_dbt_project(&dir), "an empty directory is not a dbt project");

        // A directory of that name is not the project file.
        std::fs::create_dir_all(dir.join("dbt_project.yml")).unwrap();
        assert!(!is_dbt_project(&dir));
        std::fs::remove_dir_all(dir.join("dbt_project.yml")).unwrap();

        std::fs::write(dir.join("dbt_project.yml"), "name: demo\n").unwrap();
        assert!(is_dbt_project(&dir));

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn resolve_confines_every_shape_of_path_to_the_root() {
        let dir = std::env::temp_dir().join(format!("dbt-edith-resolve-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("models")).unwrap();
        std::fs::write(dir.join("models").join("a.sql"), "select 1").unwrap();
        let root = dir.canonicalize().unwrap();

        assert!(resolve(&root, "models/a.sql").is_ok());
        assert!(resolve(&root, "models\\a.sql").is_ok());
        // Not there yet, nor its directory: judged by the nearest existing ancestor.
        assert!(resolve(&root, "models/new/deeper/b.sql").is_ok());

        assert!(resolve(&root, "../x").is_err());
        assert!(resolve(&root, "..\\..\\x").is_err());
        assert!(resolve(&root, "models/../../x").is_err());
        assert!(resolve(&root, "/etc/passwd").map(|p| p.starts_with(&root)).unwrap_or(true));
        assert!(resolve(&root, "C:/x").is_err());
        assert!(resolve(&root, "C:\\Users\\me").is_err());

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink("/", dir.join("escape")).unwrap();
            assert!(resolve(&root, "escape/etc/passwd").is_err());
        }

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
