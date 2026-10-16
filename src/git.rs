//! Working-tree status and the handful of git commands the UI can run.
//!
//! Two rules hold everywhere in this file:
//!   - nothing here destroys work. No `-f`, no `--hard`, no `clean`, no
//!     `push --force`. The worst any button can do is create a stash.
//!   - nothing here can hang the server. Network commands run with terminal
//!     prompts disabled and ssh in batch mode, and every command has a deadline.

use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[derive(serde::Serialize, Clone, Default)]
pub struct FileChange {
    pub path: String,
    /// The two-letter porcelain code, index status then worktree status.
    pub code: String,
}

#[derive(serde::Serialize, Clone, Default)]
pub struct GitInfo {
    pub repo: bool,
    pub branch: String,
    pub upstream: String,
    pub ahead: u32,
    pub behind: u32,
    /// A merge is in progress, so a commit would conclude it.
    pub merging: bool,
    /// Tracked paths that differ from HEAD, and untracked ones. An entry ending
    /// in `/` stands for a whole directory, which is how git reports collapsed
    /// results. Both feed the explorer colours.
    pub modified: Vec<String>,
    pub untracked: Vec<String>,
    pub staged: Vec<FileChange>,
    pub unstaged: Vec<FileChange>,
    pub conflicted: Vec<String>,
    /// As `git rev-parse --git-dir` printed it, for the freshness check to find
    /// FETCH_HEAD without asking again. Never sent: it can be an absolute path.
    #[serde(skip)]
    pub git_dir: String,
}

#[derive(serde::Serialize, Clone, Default)]
pub struct GitRun {
    pub ok: bool,
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
    /// Set when a checkout was refused because it would overwrite local work.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub blocking: Vec<String>,
}

const NET_ENV: &[(&str, &str)] = &[
    ("GIT_TERMINAL_PROMPT", "0"),
    // Fail fast instead of waiting on a passphrase prompt nobody can answer.
    // The host alias from ~/.ssh/config still resolves normally.
    ("GIT_SSH_COMMAND", "ssh -o BatchMode=yes"),
];

/// Runs git with a deadline, draining both pipes so a chatty hook cannot deadlock.
pub fn run(root: &Path, args: &[&str], env: &[(&str, &str)], timeout: Duration) -> GitRun {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(root).args(args);
    for (k, v) in env {
        cmd.env(k, v);
    }
    cmd.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            return GitRun { ok: false, code: -1, stdout: String::new(), stderr: format!("cannot run git: {e}"), blocking: Vec::new() }
        }
    };

    let mut out_pipe = child.stdout.take();
    let mut err_pipe = child.stderr.take();
    let out_thread = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(p) = out_pipe.as_mut() {
            let _ = p.read_to_end(&mut buf);
        }
        buf
    });
    let err_thread = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(p) = err_pipe.as_mut() {
            let _ = p.read_to_end(&mut buf);
        }
        buf
    });

    let deadline = Instant::now() + timeout;
    // Most of what the UI asks git is answered in a few milliseconds, so the
    // first naps are short: a fixed 30 ms one made every such call cost 30, and
    // the status poll makes several every five seconds. A long command still
    // settles at one check per 30 ms.
    let mut nap = Duration::from_millis(1);
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break Ok(s),
            Ok(None) => {
                if Instant::now() > deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    break Err(format!("git took longer than {}s and was stopped", timeout.as_secs()));
                }
                std::thread::sleep(nap);
                nap = (nap * 2).min(Duration::from_millis(30));
            }
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                break Err(format!("git could not be waited for and was stopped: {e}"));
            }
        }
    };

    // Waited for even once git is stopped: a helper it started, as
    // git-remote-https on a proxy that never answers, holds both pipes until
    // it gives up, minutes later. Left behind, one would pile up per call.
    let stdout = String::from_utf8_lossy(&out_thread.join().unwrap_or_default()).into_owned();
    let stderr = String::from_utf8_lossy(&err_thread.join().unwrap_or_default()).into_owned();
    match status {
        Ok(s) => GitRun { ok: s.success(), code: s.code().unwrap_or(-1), stdout, stderr, blocking: Vec::new() },
        Err(why) => GitRun { ok: false, code: -1, stdout, stderr: why, blocking: Vec::new() },
    }
}

fn read(root: &Path, args: &[&str]) -> Option<String> {
    let r = run(root, args, &[], Duration::from_secs(20));
    r.ok.then_some(r.stdout)
}

// ---------------------------------------------------------------- status ----

/// Two git calls, because the status is polled every five seconds: one for
/// where the project sits in the repository, one for everything else, the
/// branch and its upstream included.
pub fn status(root: &Path) -> GitInfo {
    let mut info = GitInfo::default();

    // Neither flag touches HEAD, which fails in a repository with no commit
    // yet, and such a repository still has a status worth showing. They print
    // one line each, in the order asked, the prefix being empty at the top.
    let Some(dirs) = read(root, &["rev-parse", "--show-prefix", "--git-dir"]) else {
        return info;
    };
    info.repo = true;
    let mut lines = dirs.split('\n').map(str::trim);
    let prefix = lines.next().unwrap_or("").to_string();
    info.git_dir = lines.next().unwrap_or("").to_string();
    info.merging = !info.git_dir.is_empty() && root.join(&info.git_dir).join("MERGE_HEAD").exists();

    // --no-renames keeps every entry to a single record, so there is no paired
    // "old path" entry to track while parsing. --branch adds the headers that
    // `branch --show-current`, `rev-parse @{u}` and `rev-list --count` used to
    // answer in three more processes.
    let raw = run(
        root,
        &["status", "--porcelain=v2", "--branch", "--ahead-behind", "-z", "--no-renames"],
        &[],
        Duration::from_secs(30),
    );
    if !raw.ok {
        return info;
    }

    let (mut upstream, mut counts) = ("", None);
    let mut entries = raw.stdout.split('\0');
    while let Some(entry) = entries.next() {
        if let Some(header) = entry.strip_prefix("# ") {
            match header.split_once(' ') {
                // What `branch --show-current` printed: nothing when detached.
                Some(("branch.head", "(detached)")) => {}
                Some(("branch.head", name)) => info.branch = name.to_string(),
                Some(("branch.upstream", name)) => upstream = name,
                Some(("branch.ab", ab)) => counts = Some(ab),
                _ => {}
            }
            continue;
        }
        let Some((code, path)) = porcelain_v2(entry) else {
            // A rename record is followed by its old path in an entry of its
            // own. --no-renames should mean there are none; if one comes
            // anyway, its old path must not be read as a record.
            if entry.starts_with("2 ") {
                entries.next();
            }
            continue;
        };
        let path = match path.strip_prefix(prefix.as_str()) {
            Some(p) if !prefix.is_empty() => p.to_string(),
            _ if prefix.is_empty() => path.to_string(),
            _ => continue, // outside the opened project
        };
        if path.is_empty() {
            continue;
        }
        let (x, y) = (code.as_bytes()[0], code.as_bytes()[1]);

        if code == "??" {
            info.untracked.push(path);
            continue;
        }
        info.modified.push(path.clone());

        // Both sides "unmerged" in any combination means a conflict.
        let conflict = matches!(code.as_str(), "DD" | "AU" | "UD" | "UA" | "DU" | "AA" | "UU");
        if conflict {
            info.conflicted.push(path);
            continue;
        }
        if x != b' ' {
            info.staged.push(FileChange { path: path.clone(), code: code.clone() });
        }
        if y != b' ' {
            info.unstaged.push(FileChange { path, code });
        }
    }
    // git names an upstream as soon as one is configured, and counts commits
    // only when its ref exists. `rev-parse @{u}` failed for one whose remote
    // branch was deleted, and the page offers to publish such a branch again,
    // so an upstream without counts is no upstream.
    if let Some(ab) = counts {
        let mut it = ab.split(' ');
        info.upstream = upstream.to_string();
        info.ahead = it.next().and_then(|v| v.strip_prefix('+')?.parse().ok()).unwrap_or(0);
        info.behind = it.next().and_then(|v| v.strip_prefix('-')?.parse().ok()).unwrap_or(0);
    }
    info.modified.sort();
    info.untracked.sort();
    info.staged.sort_by(|a, b| a.path.cmp(&b.path));
    info.unstaged.sort_by(|a, b| a.path.cmp(&b.path));
    info.conflicted.sort();
    info
}

/// The two-letter code and the path of one porcelain v2 record, the code
/// spelled as v1 spelled it, with a space where v2 writes a dot, since that is
/// what the page reads. Paths come last on each line and may hold spaces, so
/// they are what remains after a fixed number of fields: 8 for a change, 10 for
/// a conflict. None for anything else: headers, ignored files, renames.
fn porcelain_v2(entry: &str) -> Option<(String, &str)> {
    let (code, path) = match entry.split_once(' ')? {
        ("?", path) => ("??", path),
        ("1", rest) => (rest.get(..2)?, rest.splitn(8, ' ').nth(7)?),
        ("u", rest) => (rest.get(..2)?, rest.splitn(10, ' ').nth(9)?),
        _ => return None,
    };
    Some((code.replace('.', " "), path))
}

// -------------------------------------------------------------- branches ----

#[derive(serde::Serialize, Clone)]
pub struct Branch {
    pub name: String,
    pub current: bool,
    pub upstream: String,
    pub when: String,
}

/// Local branches, most recently committed first. Long-lived repositories carry
/// hundreds, so the UI filters rather than lists.
pub fn branches(root: &Path) -> Vec<Branch> {
    let current = read(root, &["branch", "--show-current"]).unwrap_or_default().trim().to_string();
    let Some(out) = read(
        root,
        &[
            "for-each-ref",
            "--sort=-committerdate",
            "--format=%(refname:short)\t%(upstream:short)\t%(committerdate:relative)",
            "refs/heads",
        ],
    ) else {
        return Vec::new();
    };
    out.lines()
        .filter_map(|l| {
            let mut f = l.split('\t');
            let name = f.next()?.to_string();
            if name.is_empty() {
                return None;
            }
            Some(Branch {
                current: name == current,
                upstream: f.next().unwrap_or("").to_string(),
                when: f.next().unwrap_or("").to_string(),
                name,
            })
        })
        .collect()
}

// --------------------------------------------------------------- actions ----

/// Switches branch. Never forces: if git refuses because local work is in the
/// way, the blocking paths come back so the UI can offer to stash them.
pub fn checkout(root: &Path, branch: &str, stash_first: bool) -> GitRun {
    if stash_first {
        let from = read(root, &["branch", "--show-current"]).unwrap_or_default().trim().to_string();
        let label = format!("dbt-edith: switching away from {}", if from.is_empty() { "detached HEAD" } else { &from });
        let stashed = run(
            root,
            &["stash", "push", "--include-untracked", "-m", &label],
            &[],
            Duration::from_secs(60),
        );
        if !stashed.ok {
            return stashed;
        }
    }
    let mut r = run(root, &["checkout", branch], &[], Duration::from_secs(60));
    if !r.ok {
        r.blocking = parse_blocking(&r.stderr);
    }
    r
}

/// Pulls the file list out of git's "your local changes would be overwritten" refusal.
fn parse_blocking(stderr: &str) -> Vec<String> {
    let mut files = Vec::new();
    let mut collecting = false;
    for line in stderr.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("error:") || trimmed.starts_with("Your local changes") {
            collecting = true;
            continue;
        }
        if trimmed.starts_with("Please commit") || trimmed.starts_with("Aborting") || trimmed.is_empty() {
            collecting = false;
            continue;
        }
        if collecting && !trimmed.contains(' ') {
            files.push(trimmed.to_string());
        }
    }
    files
}

pub fn stage(root: &Path, paths: &[String]) -> GitRun {
    let mut args = vec!["add", "--"];
    args.extend(paths.iter().map(String::as_str));
    run(root, &args, &[], Duration::from_secs(60))
}

pub fn unstage(root: &Path, paths: &[String]) -> GitRun {
    let mut args = vec!["restore", "--staged", "--"];
    args.extend(paths.iter().map(String::as_str));
    run(root, &args, &[], Duration::from_secs(60))
}

/// Fetches first, as asked, so the ahead/behind shown next to the commit is
/// current. A failing fetch never blocks the commit: offline still commits.
pub fn commit(root: &Path, message: &str) -> (GitRun, Option<String>) {
    let fetched = run(root, &["fetch", "--quiet"], NET_ENV, Duration::from_secs(120));
    let fetch_note = (!fetched.ok).then(|| {
        let line = fetched.stderr.lines().next().unwrap_or("fetch failed").trim().to_string();
        format!("fetch failed, committed anyway: {line}")
    });
    // Hooks run: a project may have several, and silently bypassing them with
    // --no-verify would be a nasty surprise. They can be slow, hence 5 minutes.
    let r = run(root, &["commit", "-m", message], &[], Duration::from_secs(300));
    (r, fetch_note)
}

pub fn push(root: &Path) -> GitRun {
    let branch = read(root, &["branch", "--show-current"]).unwrap_or_default().trim().to_string();
    if branch.is_empty() {
        return GitRun { ok: false, code: -1, stdout: String::new(), stderr: "detached HEAD, nothing to push".into(), blocking: Vec::new() };
    }
    // --set-upstream is safe and makes the first push of a topic branch work.
    run(root, &["push", "--set-upstream", "origin", &branch], NET_ENV, Duration::from_secs(180))
}

/// --ff-only: a pull must never start a merge behind your back. If the branches
/// have diverged it stops and says so, and you decide what to do.
pub fn pull(root: &Path) -> GitRun {
    run(root, &["pull", "--ff-only"], NET_ENV, Duration::from_secs(180))
}

pub fn fetch(root: &Path) -> GitRun {
    run(root, &["fetch", "--prune"], NET_ENV, Duration::from_secs(120))
}

/// The commits a push would send.
pub fn outgoing(root: &Path) -> Vec<String> {
    read(root, &["log", "--oneline", "--no-decorate", "@{u}..HEAD"])
        .map(|o| o.lines().map(str::to_string).collect())
        .unwrap_or_default()
}

#[derive(serde::Serialize, Default)]
pub struct DiffView {
    pub path: String,
    /// The committed version, empty when the file is new.
    pub before: String,
    /// What is on disk now, empty when the file was deleted.
    pub after: String,
    pub before_missing: bool,
    pub after_missing: bool,
    pub binary: bool,
    pub truncated: bool,
}

/// HEAD against the working tree, which is what "see my changes" means. The
/// `HEAD:./path` form lets git resolve the path relative to the project, so a
/// project sitting below the repository root needs no special handling.
/// `on_disk` is the same path already confined to the project by the caller.
pub fn diff(root: &Path, rel: &str, on_disk: &Path, max_bytes: usize) -> DiffView {
    let mut view = DiffView { path: rel.to_string(), ..Default::default() };

    let show = run(root, &["show", &format!("HEAD:./{rel}")], &[], Duration::from_secs(30));
    if show.ok {
        view.before = show.stdout;
    } else {
        view.before_missing = true;
    }

    match std::fs::read(on_disk) {
        Ok(bytes) => {
            view.binary = bytes.contains(&0);
            view.after = String::from_utf8_lossy(&bytes).into_owned();
        }
        Err(_) => view.after_missing = true,
    }
    view.binary = view.binary || view.before.as_bytes().contains(&0);

    if view.before.len() > max_bytes || view.after.len() > max_bytes {
        view.truncated = true;
        view.before.truncate(max_bytes);
        view.after.truncate(max_bytes);
    }
    view
}

pub fn merge_abort(root: &Path) -> GitRun {
    run(root, &["merge", "--abort"], &[], Duration::from_secs(60))
}

// ----------------------------------------------------------------- drift ----

#[derive(serde::Serialize, Clone, Default)]
pub struct Head {
    /// Abbreviated, because it is shown and never passed back to git.
    pub sha: String,
    pub subject: String,
    /// Committer date. Not compared against the manifest's mtime: committing
    /// changes no file, so a commit made after a parse leaves it still true.
    pub at: u64,
}

pub fn head(root: &Path) -> Head {
    let Some(line) = read(root, &["log", "-1", "--format=%h%x09%ct%x09%s"]) else {
        return Head::default();
    };
    let mut f = line.trim_end().splitn(3, '\t');
    Head {
        sha: f.next().unwrap_or("").to_string(),
        at: f.next().and_then(|v| v.parse().ok()).unwrap_or(0),
        subject: f.next().unwrap_or("").to_string(),
    }
}

#[derive(serde::Serialize, Clone, Default)]
pub struct Drift {
    /// The default branch as this clone knows it, `origin/main` or similar.
    pub base: String,
    /// Commits on `base` that this branch does not have.
    pub behind: u32,
    /// When a fetch last wrote FETCH_HEAD, so an answer resting on week-old
    /// refs can say so instead of looking authoritative.
    pub fetched_at: u64,
}

/// Which remote branch the others are cut from. `origin/HEAD` is what a clone
/// records; a repository initialised locally has no such ref, hence the two
/// conventional names as a fallback.
fn base_ref(root: &Path) -> String {
    if let Some(r) = read(root, &["rev-parse", "--abbrev-ref", "origin/HEAD"]) {
        let r = r.trim();
        if !r.is_empty() && r != "origin/HEAD" {
            return r.to_string();
        }
    }
    for name in ["origin/main", "origin/master"] {
        if read(root, &["rev-parse", "--verify", "--quiet", &format!("refs/remotes/{name}")]).is_some() {
            return name.to_string();
        }
    }
    String::new()
}

/// How far this branch has fallen behind the default branch, read from refs
/// already on disk. Nothing here touches the network: `fetch_quiet` is what
/// keeps those refs current, and this only reports what it finds. `git_dir` is
/// the one `status` read, which saves the poll a process.
pub fn drift(root: &Path, git_dir: &str) -> Drift {
    let mut d = Drift {
        fetched_at: if git_dir.is_empty() { 0 } else { crate::files::mtime_secs(&root.join(git_dir).join("FETCH_HEAD")) },
        ..Default::default()
    };
    d.base = base_ref(root);
    if d.base.is_empty() {
        return d;
    }
    d.behind = read(root, &["rev-list", "--count", &format!("HEAD..{}", d.base)])
        .and_then(|c| c.trim().parse().ok())
        .unwrap_or(0);
    d
}

/// The fetch behind the freshness badge, run on a long timer rather than by a
/// click. Read-only, no prune, and a failure is dropped: offline, or on a
/// machine whose key needs a passphrase, the badge falls back to refs already
/// on disk and says when they were last refreshed.
pub fn fetch_quiet(root: &Path) -> bool {
    run(root, &["fetch", "--quiet"], NET_ENV, Duration::from_secs(60)).ok
}

/// The tags a remote holds, as `git ls-remote --tags --refs` lists them, for
/// the look for a newer dbt-edith (0068, 0079). Run from `dir`, which only
/// has to exist: the URL is the remote, so no repository is involved, and git
/// looks for none above `dir`, whose configuration could rewrite the URL. When
/// git could not ask, offline above all, the first line it printed, for the
/// menu that says when the last look was made and how it went.
pub fn ls_remote_tags(dir: &Path, url: &str) -> Result<String, String> {
    let ceiling = dir.parent().and_then(Path::to_str).unwrap_or("");
    let env = [NET_ENV, &[("GIT_CEILING_DIRECTORIES", ceiling)]].concat();
    let r = run(dir, &["ls-remote", "--tags", "--refs", url], &env, Duration::from_secs(30));
    if r.ok {
        return Ok(r.stdout);
    }
    Err(reason(&r))
}

/// The first line git printed when a command failed, or its exit code.
fn reason(r: &GitRun) -> String {
    let line = r.stderr.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("");
    if line.is_empty() {
        format!("git exited with {}", r.code)
    } else {
        line.to_string()
    }
}

/// The subject of the commit each of `tags` names on a remote, in the order
/// given, for the list of what each newer dbt-edith brought (0079). Fetched
/// into a bare repository made for it under `scratch`, which only has to
/// exist, and removed afterwards: the tagged commits alone, no history, no
/// tree and no file, so the request is a few kilobytes. git looks for no
/// repository above it, as `ls_remote_tags` does not. A tag the remote does
/// not hold fails the fetch, so every one asked for comes from its listing.
pub fn tag_subjects(scratch: &Path, url: &str, tags: &[String]) -> Result<Vec<(String, String)>, String> {
    static MADE: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let made = MADE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = scratch.join(format!("dbt-edith-titles-{}-{made}", std::process::id()));
    // Made here or not at all: `create_dir_all` takes a directory already
    // there, which in a shared temporary directory could be another account's,
    // planted under this predictable name, or a link into somewhere the fetch
    // would then write. A leftover of our own from a crash is removed first;
    // someone else's cannot be, and the look fails rather than use it.
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir(&dir).map_err(|e| format!("cannot make {}: {e}", dir.display()))?;
    let ceiling = scratch.to_str().unwrap_or("");
    let env = [NET_ENV, &[("GIT_CEILING_DIRECTORIES", ceiling)]].concat();
    let answer = (|| {
        let init = run(&dir, &["init", "--bare", "-q", "."], &env, Duration::from_secs(10));
        if !init.ok {
            return Err(reason(&init));
        }
        let specs: Vec<String> = tags.iter().map(|t| format!("refs/tags/{t}:refs/tags/{t}")).collect();
        let mut args = vec!["fetch", "-q", "--depth=1", "--filter=tree:0", "--no-tags", url];
        args.extend(specs.iter().map(String::as_str));
        let fetched = run(&dir, &args, &env, Duration::from_secs(30));
        if !fetched.ok {
            return Err(reason(&fetched));
        }
        // `*subject` is the commit's under an annotated tag, whose own
        // `subject` is the tag's message; a lightweight tag has only `subject`.
        let format = "--format=%(refname:strip=2)%09%(*subject)%09%(subject)";
        let listed = run(&dir, &["for-each-ref", format, "refs/tags"], &env, Duration::from_secs(10));
        if !listed.ok {
            return Err(reason(&listed));
        }
        let subjects: std::collections::HashMap<&str, &str> = listed
            .stdout
            .lines()
            .filter_map(|line| {
                let mut parts = line.splitn(3, '\t');
                let (tag, peeled, own) = (parts.next()?, parts.next()?, parts.next().unwrap_or(""));
                Some((tag, if peeled.is_empty() { own } else { peeled }))
            })
            .collect();
        Ok(tags.iter().filter_map(|t| Some((t.clone(), subjects.get(t.as_str())?.to_string()))).collect())
    })();
    let _ = std::fs::remove_dir_all(&dir);
    answer
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dbt-edith-git-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Setup commands, kept away from the machine's own git configuration: a
    /// global `commit.gpgsign` or hook would otherwise decide whether they work.
    /// The identity is given, not left to git to guess: it guesses one from the
    /// host name on a Mac, and refuses on a CI machine whose name has no domain.
    fn setup(dir: &Path, args: &[&str]) -> std::process::Output {
        Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_NAME", "Ada")
            .env("GIT_AUTHOR_EMAIL", "ada@example.com")
            .env("GIT_COMMITTER_NAME", "Ada")
            .env("GIT_COMMITTER_EMAIL", "ada@example.com")
            .output()
            .unwrap()
    }

    fn git(dir: &Path, args: &[&str]) -> String {
        let out = setup(dir, args);
        assert!(out.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&out.stderr));
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    fn write(dir: &Path, rel: &str, text: &str) {
        let path = dir.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    fn repo(dir: &Path) {
        git(dir, &["init", "-q", "-b", "main"]);
        write(dir, "README.md", "one\n");
        git(dir, &["add", "README.md"]);
        git(dir, &["commit", "-q", "-m", "first"]);
    }

    fn changes(v: &[FileChange]) -> Vec<(String, String)> {
        v.iter().map(|c| (c.code.clone(), c.path.clone())).collect()
    }

    fn pairs(v: &[(&str, &str)]) -> Vec<(String, String)> {
        v.iter().map(|(c, p)| (c.to_string(), p.to_string())).collect()
    }

    #[test]
    fn outside_a_repository_there_is_no_status() {
        let dir = scratch("none");
        let info = status(&dir);
        assert!(!info.repo);
        assert!(info.branch.is_empty() && info.git_dir.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_repository_without_a_commit_still_has_a_status() {
        let dir = scratch("unborn");
        git(&dir, &["init", "-q", "-b", "trunk"]);
        write(&dir, "models/a.sql", "select 1\n");
        write(&dir, "models/b.sql", "select 2\n");
        git(&dir, &["add", "models/a.sql"]);
        let info = status(&dir);
        assert!(info.repo);
        assert_eq!(info.branch, "trunk");
        assert_eq!(info.git_dir, ".git");
        assert!(info.upstream.is_empty());
        assert_eq!(changes(&info.staged), pairs(&[("A ", "models/a.sql")]));
        assert_eq!(info.untracked, ["models/b.sql"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The codes are what porcelain v1 printed, which the page reads: a space
    /// where nothing changed, never v2's dot. Paths are relative to the project,
    /// which here sits one folder down, and a change outside it is left out.
    #[test]
    fn every_change_is_listed_under_the_project_with_v1_codes() {
        let dir = scratch("codes");
        repo(&dir);
        for f in ["proj/models/edited.sql", "proj/models/staged.sql", "proj/models/both.sql", "proj/models/gone.sql", "proj/models/my model.sql", "elsewhere.txt"] {
            write(&dir, f, "select 1\n");
        }
        git(&dir, &["add", "."]);
        git(&dir, &["commit", "-q", "-m", "files"]);
        write(&dir, "proj/models/edited.sql", "select 2\n");
        write(&dir, "proj/models/staged.sql", "select 2\n");
        write(&dir, "proj/models/both.sql", "select 2\n");
        git(&dir, &["add", "proj/models/staged.sql", "proj/models/both.sql"]);
        write(&dir, "proj/models/both.sql", "select 3\n");
        std::fs::remove_file(dir.join("proj/models/gone.sql")).unwrap();
        write(&dir, "proj/models/my model.sql", "select 2\n");
        write(&dir, "proj/models/added.sql", "select 1\n");
        git(&dir, &["add", "proj/models/added.sql"]);
        write(&dir, "proj/new_mart/revenue.sql", "select 1\n");
        write(&dir, "elsewhere.txt", "changed\n");

        let info = status(&dir.join("proj"));
        assert!(info.repo && !info.merging);
        assert_eq!(info.branch, "main");
        assert!(Path::new(&info.git_dir).is_absolute(), "asked from a subfolder, git answers an absolute path");
        assert_eq!(
            changes(&info.staged),
            pairs(&[("A ", "models/added.sql"), ("MM", "models/both.sql"), ("M ", "models/staged.sql")])
        );
        assert_eq!(
            changes(&info.unstaged),
            pairs(&[("MM", "models/both.sql"), (" M", "models/edited.sql"), (" D", "models/gone.sql"), (" M", "models/my model.sql")])
        );
        assert_eq!(
            info.modified,
            ["models/added.sql", "models/both.sql", "models/edited.sql", "models/gone.sql", "models/my model.sql", "models/staged.sql"]
        );
        assert_eq!(info.untracked, ["new_mart/"], "a new folder comes collapsed, as v1 gave it");
        assert!(info.conflicted.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_conflict_is_listed_once_and_a_merge_is_noticed() {
        let dir = scratch("conflict");
        repo(&dir);
        git(&dir, &["checkout", "-q", "-b", "other"]);
        write(&dir, "README.md", "theirs\n");
        git(&dir, &["commit", "-q", "-am", "theirs"]);
        git(&dir, &["checkout", "-q", "main"]);
        write(&dir, "README.md", "ours\n");
        git(&dir, &["commit", "-q", "-am", "ours"]);
        let merge = setup(&dir, &["merge", "-q", "other"]);
        assert!(!merge.status.success(), "the merge should stop on the conflict");
        assert!(String::from_utf8_lossy(&merge.stdout).contains("CONFLICT"), "stopped for another reason: {merge:?}");

        let info = status(&dir);
        assert!(info.merging);
        assert_eq!(info.conflicted, ["README.md"]);
        assert_eq!(info.modified, ["README.md"]);
        assert!(info.staged.is_empty() && info.unstaged.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_upstream_and_how_far_apart_come_with_the_status() {
        let dir = scratch("upstream");
        let origin = dir.join("origin");
        std::fs::create_dir_all(&origin).unwrap();
        repo(&origin);
        git(&dir, &["clone", "-q", origin.to_str().unwrap(), "clone"]);
        let clone = dir.join("clone");
        write(&origin, "README.md", "two\n");
        git(&origin, &["commit", "-q", "-am", "upstream moved"]);
        git(&clone, &["fetch", "-q"]);
        for n in ["a", "b"] {
            write(&clone, &format!("{n}.sql"), "select 1\n");
            git(&clone, &["add", "."]);
            git(&clone, &["commit", "-q", "-m", n]);
        }

        let info = status(&clone);
        assert_eq!((info.branch.as_str(), info.upstream.as_str()), ("main", "origin/main"));
        assert_eq!((info.ahead, info.behind), (2, 1));

        // A branch whose remote branch was deleted keeps its configuration,
        // and git still names it, but `rev-parse @{u}` failed for it: the page
        // offers to publish it again.
        git(&clone, &["checkout", "-q", "-b", "feature"]);
        git(&clone, &["push", "-q", "-u", "origin", "feature"]);
        assert_eq!(status(&clone).upstream, "origin/feature");
        git(&origin, &["branch", "-q", "-D", "feature"]);
        git(&clone, &["fetch", "-q", "--prune"]);
        let gone = status(&clone);
        assert_eq!(gone.branch, "feature");
        assert!(gone.upstream.is_empty(), "{}", gone.upstream);
        assert_eq!((gone.ahead, gone.behind), (0, 0));

        // Detached, there is no branch to name and no upstream to count from.
        git(&clone, &["checkout", "-q", "--detach", "main"]);
        let detached = status(&clone);
        assert!(detached.branch.is_empty() && detached.upstream.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_v2_record_gives_its_code_and_whole_path() {
        let sha = "0".repeat(40);
        let change = format!("1 .M N... 100644 100644 100644 {sha} {sha} models/my model.sql");
        assert_eq!(porcelain_v2(&change), Some((" M".to_string(), "models/my model.sql")));
        let conflict = format!("u UU N... 100644 100644 100644 100644 {sha} {sha} {sha} a b.sql");
        assert_eq!(porcelain_v2(&conflict), Some(("UU".to_string(), "a b.sql")));
        assert_eq!(porcelain_v2("? new dir/"), Some(("??".to_string(), "new dir/")));
        assert_eq!(porcelain_v2("! ignored.log"), None);
        assert_eq!(porcelain_v2("# branch.head main"), None);
        assert_eq!(porcelain_v2(""), None);
    }

    /// The nap that replaced a fixed 30 ms one, which every call paid however
    /// quick git was. The fastest of ten is judged rather than their average,
    /// so a busy CI machine slowing a few of them proves nothing either way.
    #[test]
    fn a_quick_command_is_not_held_back() {
        let dir = scratch("quick");
        repo(&dir);
        let fastest = (0..10)
            .map(|_| {
                let t = Instant::now();
                assert!(run(&dir, &["rev-parse", "--git-dir"], &[], Duration::from_secs(20)).ok);
                t.elapsed()
            })
            .min()
            .unwrap();
        assert!(fastest < Duration::from_millis(25), "{fastest:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
