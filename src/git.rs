//! Working-tree status and the handful of git commands the UI can run.
//!
//! Three rules hold everywhere in this file:
//!   - nothing here destroys work. No `-f`, no `--hard`, no `clean`, no
//!     `push --force`. The worst any button can do is create a stash.
//!   - nothing here can hang the server. Network commands run with terminal
//!     prompts disabled and ssh in batch mode, and every command has a deadline.
//!   - the git that runs is the one on the PATH, never one a repository
//!     commits beside this binary (0094).
//!   - a repository's own configuration never gets to name a program edith
//!     starts (0121). Every call turns the file-system monitor off; the calls
//!     made on a timer, with no click behind them, also turn hooks off; and no
//!     call is made at all in a repository whose local or worktree
//!     configuration holds a key outside `allowed`. That list assumes edith's
//!     fixed set of commands: all builtins, never an alias, a merge, a diff
//!     driver, a mergetool, a signature check or a credential prompt of the
//!     repository's choosing. A command added here that reaches one of those
//!     has to shrink the list first.

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime};

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
    /// The first key of the repository's own configuration that names a
    /// program, when there is one: edith then runs no git here (0121). Empty
    /// otherwise. `repo` is false meanwhile, as the page has nothing to show.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub paused: String,
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

/// Fail fast instead of waiting on a passphrase prompt nobody can answer. The
/// host alias from ~/.ssh/config still resolves normally. Set on every call,
/// not only the network ones: git reaches a remote on its own, to fetch an
/// object a partial clone left out, and the variable wins over the
/// repository's `core.sshCommand` (0121).
const SSH_BATCH: &str = "ssh -o BatchMode=yes";

const NET_ENV: &[(&str, &str)] = &[("GIT_TERMINAL_PROMPT", "0"), ("GIT_SSH_COMMAND", SSH_BATCH)];

/// The transports a remote is reached by, for a call that may fetch on its own
/// and was not asked to: the repository's own `protocol.*` keys cannot widen
/// them (0121).
const TRANSPORTS: [&str; 8] = [
    "-c", "protocol.allow=never", "-c", "protocol.https.allow=always", "-c", "protocol.http.allow=always", "-c",
    "protocol.ssh.allow=always",
];

/// The subcommand of an argument list: the first argument that is not a
/// `-c <pair>`.
fn subcommand<'a>(args: &[&'a str]) -> &'a str {
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if *a == "-c" {
            it.next();
        } else {
            return a;
        }
    }
    ""
}

/// The git on the PATH: never one beside this binary, which on Windows
/// `Command` would find first (0094). Kept once found, as the PATH it came
/// from does not change while dbt-edith runs; looked for again until then,
/// so a git installed while it runs is found without a restart.
fn program() -> Option<&'static Path> {
    static GIT: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
    if let Some(git) = GIT.get() {
        return Some(git);
    }
    let found = crate::program::on_path("git")?;
    Some(GIT.get_or_init(|| found))
}

/// Runs git with a deadline, draining both pipes so a chatty hook cannot deadlock.
/// For what a click asked: hooks run, as the user knows them. Nothing runs in a
/// repository whose configuration is paused (0121).
pub fn run(root: &Path, args: &[&str], env: &[(&str, &str)], timeout: Duration) -> GitRun {
    guarded(root, args, env, timeout, false)
}

/// As `run`, for what nobody clicked: a poll or a timer. Hooks are off, so a
/// repository that arrived with some cannot run them by being opened (0121).
fn run_auto(root: &Path, args: &[&str], env: &[(&str, &str)], timeout: Duration) -> GitRun {
    guarded(root, args, env, timeout, true)
}

fn guarded(root: &Path, args: &[&str], env: &[(&str, &str)], timeout: Duration, auto: bool) -> GitRun {
    if let Some(key) = paused(root) {
        return GitRun { ok: false, code: -1, stdout: String::new(), stderr: pause_note(&key), blocking: Vec::new() };
    }
    exec(root, args, env, timeout, auto)
}

fn exec(root: &Path, args: &[&str], env: &[(&str, &str)], timeout: Duration, auto: bool) -> GitRun {
    let Some(git) = program() else {
        let stderr = "cannot run git: it is not on the PATH".to_string();
        return GitRun { ok: false, code: -1, stdout: String::new(), stderr, blocking: Vec::new() };
    };
    let mut cmd = Command::new(git);
    // A repository names a program to ask whether files changed; `false` is
    // git's own way of saying it must not. On the command line it wins over
    // the repository's key, and the first thing `status` would do is run it.
    cmd.arg("-C").arg(root).args(["-c", "core.fsmonitor=false"]);
    if auto {
        // On Windows git resolves this to a path that does not exist, which
        // is all that is wanted: no hooks directory, so no hook.
        cmd.args(["-c", "core.hooksPath=/dev/null"]);
    }
    // A repository's submodules have configurations of their own that the
    // gate never reads, and a checkout that recurses into one runs its filters.
    cmd.args(["-c", "submodule.recurse=false"]);
    // A folder that looks like a bare repository (HEAD, refs, objects, a
    // config) is one git would use implicitly, and the gate looks for a
    // working tree's `.git`. Only a repository named by `--git-dir` or
    // `GIT_DIR` is taken, as `tag_subjects` names its own (git 2.38 or later;
    // an older git ignores the key).
    cmd.args(["-c", "safe.bareRepository=explicit"]);
    cmd.args(args);
    cmd.env("GIT_SSH_COMMAND", SSH_BATCH);
    // A partial clone fetches a missing object from its promisor remote
    // whenever a command wants it, the poll's `status` included. Only the
    // commands that exist to reach a remote, and a checkout, which needs the
    // files it writes, may.
    if !matches!(subcommand(args), "fetch" | "pull" | "push" | "checkout") {
        cmd.env("GIT_NO_LAZY_FETCH", "1");
    }
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

fn read_auto(root: &Path, args: &[&str]) -> Option<String> {
    let r = run_auto(root, args, &[], Duration::from_secs(20));
    r.ok.then_some(r.stdout)
}

// ------------------------------------------------------------------ gate ----

/// Where a repository keeps what the gate reads.
#[derive(PartialEq, Clone)]
struct Repo {
    git_dir: PathBuf,
    /// Where `config` lives: the main repository's, for a linked worktree.
    common_dir: PathBuf,
    /// A `.git` that is a file, as in a linked worktree or a submodule.
    gitfile: Option<PathBuf>,
}

/// mtime and size of a file, or None when it is not there.
type Stamp = Option<(SystemTime, u64)>;

fn stamp(path: &Path) -> Stamp {
    let meta = std::fs::metadata(path).ok()?;
    Some((meta.modified().unwrap_or(SystemTime::UNIX_EPOCH), meta.len()))
}

struct Verdict {
    repo: Repo,
    /// Every file the verdict was read from, as it stood.
    files: Vec<(PathBuf, Stamp)>,
    key: Option<String>,
}

fn verdicts() -> std::sync::MutexGuard<'static, HashMap<PathBuf, Verdict>> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, Verdict>>> = OnceLock::new();
    CACHE.get_or_init(Default::default).lock().unwrap_or_else(|e| e.into_inner())
}

/// Git's own test of a git directory: a `HEAD` (a file or a link, even a
/// dangling one) and an `objects` and a `refs` that merely exist. What they
/// hold is not looked at, so a plain file called `objects` passes, as it does
/// for git.
fn is_git_dir(path: &Path) -> bool {
    std::fs::symlink_metadata(path.join("HEAD")).is_ok() && path.join("objects").exists() && path.join("refs").exists()
}

/// The repository git would find from `root`, found by looking rather than by
/// asking: this runs before every call, and the poll must not pay a process
/// for it. In each folder from `root` upward git tries a `.git` file (a
/// linked worktree or a submodule names its directory there), then a `.git`
/// directory that passes `is_git_dir` (one that does not is skipped and the
/// search goes on), then the folder itself, a bare repository.
fn locate(root: &Path) -> Option<Repo> {
    for dir in root.ancestors() {
        let dot = dir.join(".git");
        if let Ok(meta) = std::fs::metadata(&dot) {
            if meta.is_dir() {
                if is_git_dir(&dot) {
                    return Some(Repo { git_dir: dot.clone(), common_dir: dot, gitfile: None });
                }
            } else {
                let named = std::fs::read_to_string(&dot)
                    .ok()
                    .and_then(|t| t.trim().strip_prefix("gitdir:").map(|p| dir.join(p.trim())));
                // A file git cannot read fails every call on its own; its stamp
                // still stands for it, so mending it is noticed.
                let git_dir = named.unwrap_or_else(|| dot.clone());
                let common_dir = match std::fs::read_to_string(git_dir.join("commondir")) {
                    Ok(c) => git_dir.join(c.trim()),
                    Err(_) => git_dir.clone(),
                };
                return Some(Repo { git_dir, common_dir, gitfile: Some(dot) });
            }
        }
        if is_git_dir(dir) {
            return Some(Repo { git_dir: dir.to_path_buf(), common_dir: dir.to_path_buf(), gitfile: None });
        }
    }
    None
}

/// Whether git finds a repository from `dir`, by the test it uses (`locate`):
/// what decides that a call it could not answer is a refusal, not an
/// outside-a-repository no.
pub fn in_repository(dir: &Path) -> bool {
    locate(dir).is_some()
}

/// Whether two paths name the same place; false when either cannot be resolved.
fn same_place(a: &Path, b: &Path) -> bool {
    matches!((a.canonicalize(), b.canonicalize()), (Ok(a), Ok(b)) if a == b)
}

/// The first key of the repository's own configuration outside `allowed`, as
/// git would show it, or None when edith may run git there (0121). Outside a
/// repository there is nothing to read and no process is started. A repository
/// whose configuration cannot be listed (git too old for the flags, a timeout)
/// is paused with git's reason, as `UNREAD` names it, and not remembered: no
/// answer is not a clean one.
///
/// Reading the configuration runs nothing; running git is what runs what it
/// names. The verdict is kept per root against the stamps of `config` and
/// `config.worktree`, so the five second poll costs a few `stat`s. An
/// `include` is not followed: the key itself pauses git, so no file it names
/// can change a verdict. A file written in the last two seconds is not trusted
/// to have changed its stamp, on a file system that rounds to them, and is read
/// again next time. The configuration read is the one git itself uses for
/// `root`, and a verdict is kept only when git names the same directories as
/// `locate` did: a disagreement is answered for the call, not remembered.
fn paused(root: &Path) -> Option<String> {
    let repo = locate(root)?;
    if let Some(v) = verdicts().get(root) {
        if v.repo == repo && v.files.iter().all(|(p, s)| stamp(p) == *s) {
            return v.key.clone();
        }
    }
    let mut files: Vec<PathBuf> = vec![repo.common_dir.join("config"), repo.git_dir.join("config.worktree")];
    files.extend(repo.gitfile.clone());
    let mut key = None;
    // Which repository git really uses, so a verdict is never kept against a
    // configuration it does not read. A bare-looking folder it refuses fails
    // here, and the repository is paused as unreadable.
    let at = exec(root, &["rev-parse", "--absolute-git-dir", "--git-common-dir"], ENGLISH, Duration::from_secs(10), true);
    if !at.ok {
        verdicts().remove(root);
        // Git found no repository where the search by eye did (a `HEAD` that
        // is no reference): every call it is given fails the same way.
        if at.stderr.contains("not a git repository") {
            return None;
        }
        return Some(format!("{UNREAD}{}", reason(&at)));
    }
    let mut lines = at.stdout.lines().map(str::trim);
    let (used_git, used_common) = (lines.next().unwrap_or(""), lines.next().unwrap_or(""));
    let agrees = same_place(&repo.git_dir, Path::new(used_git)) && same_place(&repo.common_dir, &root.join(used_common));
    let args = ["config", "--list", "--show-scope", "--name-only"];
    let r = exec(root, &args, &[], Duration::from_secs(10), true);
    if !r.ok {
        verdicts().remove(root);
        return Some(format!("{UNREAD}{}", reason(&r)));
    }
    for line in r.stdout.lines() {
        let Some((scope, name)) = line.split_once('\t') else { continue };
        if (scope == "local" || scope == "worktree") && !allowed(name) {
            key = Some(shown(name));
            break;
        }
    }
    let files: Vec<(PathBuf, Stamp)> = files.into_iter().map(|p| { let s = stamp(&p); (p, s) }).collect();
    let recent = SystemTime::now() - Duration::from_secs(2);
    let settled = files.iter().all(|(_, s)| s.is_none_or(|(at, _)| at < recent));
    if settled && agrees {
        verdicts().insert(root.to_path_buf(), Verdict { repo, files, key: key.clone() });
    } else {
        verdicts().remove(root);
    }
    key
}

/// What `paused` answers in place of a key when the configuration could not be
/// listed; the reason follows.
const UNREAD: &str = "unreadable: ";

/// A key as the page shows it. A subsection can be a URL, and a URL can hold
/// the password a credential section is about, so one with an `@` is not shown.
fn shown(key: &str) -> String {
    match (key.split_once('.'), key.rsplit_once('.')) {
        (Some((section, _)), Some((_, name))) if key.contains('@') && section.len() + name.len() + 1 < key.len() => {
            format!("{section}.<url>.{name}")
        }
        _ => key.to_string(),
    }
}

/// What a refused call says, and what the page says of a paused repository.
fn pause_note(key: &str) -> String {
    if let Some(why) = key.strip_prefix(UNREAD) {
        return format!(
            "git is paused here: this repository's own configuration could not be read ({why}). \
             git in the terminal is unaffected."
        );
    }
    format!(
        "git is paused here: this repository's own configuration names a program ({key}). \
         git in the terminal is unaffected."
    )
}

/// Whether a key of a repository's own configuration is only data to the
/// commands edith runs. Section and key name are compared case-insensitively
/// and the subsection, which can hold dots, not at all. What is not listed
/// pauses git, notably `credential.*.helper`, `filter.*`, `core.askpass`,
/// `core.gitproxy`, `core.pager`, `core.editor`, `remote.*.uploadpack`,
/// `remote.*.vcs`, `protocol.*`, `diff.*.textconv`, `merge.*.driver`,
/// `gpg.program`, `submodule.*.update`, `uploadpack.*`, `receive.*`,
/// `*tool.*.cmd`, `include.path` and `includeIf.*.path` (a file that appears
/// or an `onbranch` that turns true later would change a verdict kept against
/// the stamp of `config` alone), `lfs.extension.*` and `lfs.customtransfer.*`,
/// and the proxies: `http.proxy` and `remote.*.proxy` would send the timed
/// fetch, and the credentials it finds, through a host the repository names.
/// `core.fsmonitor`, `core.hooksPath` and `core.sshCommand` are
/// listed because a command-line key, a hooks-free call or the ssh command
/// edith sets takes their place, not because they are harmless.
fn allowed(key: &str) -> bool {
    let Some((section, rest)) = key.split_once('.') else { return false };
    let (sub, name) = match rest.rsplit_once('.') {
        Some((sub, name)) => (Some(sub), name),
        None => (None, rest),
    };
    let section = section.to_ascii_lowercase();
    let name = name.to_ascii_lowercase();
    let name = name.as_str();
    match section.as_str() {
        "branch" | "user" | "author" | "committer" | "color" | "extensions" | "maintenance" | "safe" | "alias" => true,
        // Transfer tuning only: no proxy, no cookie file, no certificate.
        "http" => matches!(name, "postbuffer" | "lowspeedlimit" | "lowspeedtime" | "version" | "maxrequests"),
        // `lfs.extension.*.clean` runs on a status, `lfs.customtransfer.*.path`
        // on a pull: only the addresses and the filters on what is fetched.
        "lfs" => match sub {
            None => matches!(name, "url" | "pushurl" | "repositoryformatversion" | "locksverify" | "fetchinclude" | "fetchexclude"),
            Some(_) => matches!(name, "access" | "locksverify"),
        },
        "remote" => match sub {
            Some(_) => matches!(
                name,
                "url" | "pushurl" | "fetch" | "push" | "tagopt" | "prune" | "prunetags" | "mirror" | "skipdefaultupdate"
                    | "skipfetchall" | "promisor" | "partialclonefilter" | "gh-resolved"
            ),
            None => name == "pushdefault",
        },
        "core" => {
            sub.is_none()
                && matches!(
                    name,
                    "repositoryformatversion" | "filemode" | "bare" | "logallrefupdates" | "ignorecase" | "precomposeunicode"
                        | "symlinks" | "autocrlf" | "safecrlf" | "eol" | "longpaths" | "fscache" | "untrackedcache"
                        | "sparsecheckout" | "sparsecheckoutcone" | "splitindex" | "commitgraph" | "multipackindex"
                        | "checkstat" | "trustctime" | "quotepath" | "abbrev" | "worktree" | "fsmonitor" | "hookspath"
                        | "sshcommand" | "excludesfile" | "preloadindex" | "whitespace"
                )
        }
        // Left by git gui on exit and by everyday habits; each is read as
        // data (git-config(1)), none names a program. Not `gui.editor`,
        // `gui.*` otherwise being git gui's own business.
        "gui" => sub.is_none() && matches!(name, "wmstate" | "geometry"),
        "blame" => {
            sub.is_none()
                && matches!(
                    name,
                    "ignorerevsfile" | "markignoredlines" | "markunblamablelines" | "coloring" | "date" | "showemail"
                        | "showroot" | "blankboundary"
                )
        }
        "rerere" => sub.is_none() && matches!(name, "enabled" | "autoupdate"),
        "i18n" => sub.is_none() && matches!(name, "commitencoding" | "logoutputencoding"),
        // Only switches for help messages.
        "advice" => sub.is_none(),
        "commit" | "tag" | "pull" | "push" | "fetch" | "rebase" | "merge" | "status" | "log" | "init" | "pack" | "index"
        | "feature" => sub.is_none(),
        "diff" => sub.is_none() && name != "external",
        "gc" => sub.is_none() && name != "recentobjectshook",
        "url" => sub.is_some() && matches!(name, "insteadof" | "pushinsteadof"),
        "gpg" => match sub {
            None => matches!(name, "format" | "mintrustlevel"),
            Some(_) => name == "allowedsignersfile",
        },
        "credential" => matches!(name, "username" | "usehttppath"),
        "submodule" => {
            sub.is_none() || matches!(name, "url" | "active" | "branch" | "fetchrecursesubmodules" | "ignore" | "shallow")
        }
        _ => false,
    }
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
    if let Some(key) = paused(root) {
        info.paused = key;
        return info;
    }
    let Some(dirs) = read_auto(root, &["rev-parse", "--show-prefix", "--git-dir"]) else {
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
    // Submodules are told by the superproject's index, which holds no config
    // of theirs: `dirty` stops git from running `status` inside each one,
    // where a repository nobody checked would be asked.
    let raw = run_auto(
        root,
        &["status", "--porcelain=v2", "--branch", "--ahead-behind", "-z", "--no-renames", "--ignore-submodules=dirty"],
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
    // A checkout is let lazy-fetch the files it writes, so it is held to the
    // transports the timed fetch uses.
    let mut args = TRANSPORTS.to_vec();
    args.extend(["checkout", branch]);
    let mut r = run(root, &args, &[], Duration::from_secs(60));
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
    let fetched = run(root, &["fetch", "--quiet", "--no-recurse-submodules"], NET_ENV, Duration::from_secs(120));
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
    run(root, &["pull", "--ff-only", "--no-recurse-submodules"], NET_ENV, Duration::from_secs(180))
}

pub fn fetch(root: &Path) -> GitRun {
    run(root, &["fetch", "--prune", "--no-recurse-submodules"], NET_ENV, Duration::from_secs(120))
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
    let Some(line) = read_auto(root, &["log", "-1", "--format=%h%x09%ct%x09%s"]) else {
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
    if let Some(r) = read_auto(root, &["rev-parse", "--abbrev-ref", "origin/HEAD"]) {
        let r = r.trim();
        if !r.is_empty() && r != "origin/HEAD" {
            return r.to_string();
        }
    }
    for name in ["origin/main", "origin/master"] {
        if read_auto(root, &["rev-parse", "--verify", "--quiet", &format!("refs/remotes/{name}")]).is_some() {
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
    d.behind = read_auto(root, &["rev-list", "--count", &format!("HEAD..{}", d.base)])
        .and_then(|c| c.trim().parse().ok())
        .unwrap_or(0);
    d
}

/// The fetch behind the freshness badge, run on a long timer rather than by a
/// click. Read-only, no prune, and a failure is dropped: offline, or on a
/// machine whose key needs a passphrase, the badge falls back to refs already
/// on disk and says when they were last refreshed.
pub fn fetch_quiet(root: &Path) -> bool {
    // Nobody asked for this one and it reaches the network, so it is held to
    // the transports a remote is reached by, and told not to start a
    // maintenance, a gc or a fetch inside the submodules, each of which runs
    // more of the repository's configuration (0121). A remote on a local path
    // is not refreshed here: the badge then says when its refs were last
    // fetched, as it does offline.
    let mut args = vec!["-c", "maintenance.auto=false", "-c", "gc.auto=0"];
    args.extend(TRANSPORTS);
    args.extend(["fetch", "--quiet", "--no-recurse-submodules"]);
    run_auto(root, &args, NET_ENV, Duration::from_secs(60)).ok
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
    let r = exec(dir, &["ls-remote", "--tags", "--refs", url], &env, Duration::from_secs(30), true);
    if r.ok {
        return Ok(r.stdout);
    }
    Err(reason(&r))
}

/// git in English, so "not a git repository" is told from a refusal.
const ENGLISH: &[(&str, &str)] = &[("LC_ALL", "C"), ("LANGUAGE", "")];

/// Whether git tracks `rel`, a file or a folder below `dir`, in whichever
/// repository holds `dir`: what came with a clone was put there by whoever
/// wrote it, not by the user (0053, 0094). Case aside and as spelt, never as a
/// pattern: macOS and Windows find a committed `Collin` under `collin`. One
/// git call. Outside a repository the answer is no, there being nobody to
/// ask; any other failure, past the deadline, git not on the PATH, or a
/// repository git refuses (another account's, by `safe.directory`), is the
/// first line git said, for the caller to judge.
pub fn tracks(dir: &Path, rel: &str) -> Result<bool, String> {
    // An answer nobody can give is an error, not a no: whatever it guards
    // stays refused while git is paused (0121).
    if let Some(key) = paused(dir) {
        return Err(pause_note(&key));
    }
    let listed = run_auto(dir, &["ls-files", "--", &format!(":(icase,literal){rel}")], ENGLISH, Duration::from_secs(10));
    if listed.ok {
        return Ok(!listed.stdout.trim().is_empty());
    }
    if listed.stderr.contains("not a git repository") {
        return Ok(false);
    }
    Err(reason(&listed))
}

/// The index of the repository holding `dir`, which `git add` and
/// `git rm --cached` rewrite while the files stay as they are: an answer of
/// `tracks` may be kept while it is unchanged. None outside a repository.
pub fn index_of(dir: &Path) -> Option<std::path::PathBuf> {
    let at = run_auto(dir, &["rev-parse", "--git-path", "index"], ENGLISH, Duration::from_secs(10));
    at.ok.then(|| dir.join(at.stdout.trim()))
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
    // The repository just made is bare, which `exec` refuses to find on its
    // own; `.` is `dir` itself, as git runs there.
    let named = [env.as_slice(), &[("GIT_DIR", ".")]].concat();
    let answer = (|| {
        let init = exec(&dir, &["init", "--bare", "-q", "."], &env, Duration::from_secs(10), true);
        if !init.ok {
            return Err(reason(&init));
        }
        let env = named;
        let specs: Vec<String> = tags.iter().map(|t| format!("refs/tags/{t}:refs/tags/{t}")).collect();
        let mut args = vec!["fetch", "-q", "--depth=1", "--filter=tree:0", "--no-tags", url];
        args.extend(specs.iter().map(String::as_str));
        let fetched = exec(&dir, &args, &env, Duration::from_secs(30), true);
        if !fetched.ok {
            return Err(reason(&fetched));
        }
        // `*subject` is the commit's under an annotated tag, whose own
        // `subject` is the tag's message; a lightweight tag has only `subject`.
        let format = "--format=%(refname:strip=2)%09%(*subject)%09%(subject)";
        let listed = exec(&dir, &["for-each-ref", format, "refs/tags"], &env, Duration::from_secs(10), true);
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

    /// What came with a clone, asked case aside and as spelt, from a folder
    /// below the repository's root, with the index that answered (0094). On
    /// main the exact pathspec missed a committed `Collin` that macOS and
    /// Windows find under `collin`.
    #[test]
    fn a_tracked_path_is_told_case_aside_and_never_as_a_pattern() {
        let dir = scratch("tracks");
        git(&dir, &["init", "-q", "-b", "main"]);
        write(&dir, "bin/Collin", "x");
        write(&dir, "bin/weird", "x");
        write(&dir, "venvs/shipped/pyvenv.cfg", "x");
        git(&dir, &["add", "bin/Collin", "bin/weird", "venvs/shipped"]);
        let bin = dir.join("bin");
        assert_eq!(tracks(&bin, "collin"), Ok(true));
        let index = index_of(&bin).map(|p| p.canonicalize().unwrap());
        assert_eq!(index, Some(dir.join(".git").join("index").canonicalize().unwrap()));
        assert_eq!(tracks(&bin, "COLLIN.EXE"), Ok(false));
        assert_eq!(tracks(&bin, "we*d"), Ok(false), "a star is a star, not `weird`");
        assert_eq!(tracks(&bin, "weird"), Ok(true));
        assert_eq!(tracks(&dir, "venvs/shipped"), Ok(true), "a folder, by what is under it");
        assert_eq!(tracks(&dir, "venvs/mine"), Ok(false));

        // Outside a repository: no, and no index to keep the answer against.
        let loose = scratch("tracks-loose");
        assert_eq!(tracks(&loose, "collin"), Ok(false));
        assert_eq!(index_of(&loose), None);
        // A folder git cannot enter is no answer at all, never a no.
        assert!(tracks(&loose.join("missing"), "collin").is_err());
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&loose);
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
        // The gate reads a configuration written this second again on every
        // call, which is not what is measured here.
        let old = SystemTime::now() - Duration::from_secs(3600);
        std::fs::OpenOptions::new().write(true).open(dir.join(".git/config")).unwrap().set_modified(old).unwrap();
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

    /// A script that leaves a marker, to tell whether git ran it.
    #[cfg(unix)]
    fn script(dir: &Path, name: &str, marker: &Path, tail: &str) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let path = dir.join(name);
        std::fs::write(&path, format!("#!/bin/sh\ntouch '{}'\n{tail}\n", marker.display())).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    /// Writing a file with the content it has makes the index stale, which
    /// `status` mends by writing it, and that is when git runs a hook and asks
    /// the file-system monitor.
    fn stale_index(dir: &Path) {
        write(dir, "README.md", "one\n");
    }

    /// What a project copied with its `.git` can bring that a clone cannot: a
    /// monitor and a hook the poll would run (0121). Both are neutralised and
    /// the repository is not paused, as `core.fsmonitor` is on the list.
    #[cfg(unix)]
    #[test]
    fn status_runs_neither_the_monitor_nor_the_hooks_a_repository_brings() {
        let dir = scratch("monitor");
        repo(&dir);
        let marks = scratch("monitor-marks");
        let monitor = script(&marks, "monitor.sh", &marks.join("monitor"), "printf '\\0'");
        let hook = script(&dir.join(".git/hooks"), "post-index-change", &marks.join("hook"), "");
        git(&dir, &["config", "core.fsmonitor", monitor.to_str().unwrap()]);

        // Without the two flags both marks appear: the test is about them.
        stale_index(&dir);
        let bare = Command::new("git").arg("-C").arg(&dir).args(["status", "--porcelain"]).output().unwrap();
        assert!(bare.status.success());
        assert!(marks.join("monitor").exists() && marks.join("hook").exists(), "the repository should run both");
        let _ = std::fs::remove_file(marks.join("monitor"));
        let _ = std::fs::remove_file(marks.join("hook"));
        assert!(hook.exists());

        stale_index(&dir);
        let info = status(&dir);
        assert!(info.repo && info.paused.is_empty(), "{}", info.paused);
        assert_eq!(info.branch, "main");
        assert!(!marks.join("monitor").exists(), "the monitor ran");
        assert!(!marks.join("hook").exists(), "a hook ran");
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&marks);
    }

    /// A clean filter runs when `status` hashes a file whose stat changed, so
    /// it is what a key outside the list would be able to do. Git is then not
    /// run at all, and says why.
    #[cfg(unix)]
    #[test]
    fn a_repository_whose_config_names_a_program_is_paused() {
        let dir = scratch("paused");
        repo(&dir);
        write(&dir, "models/a.sql", "select 1\n");
        write(&dir, ".gitattributes", "*.sql filter=evil\n");
        git(&dir, &["add", "."]);
        git(&dir, &["commit", "-q", "-m", "files"]);
        let marks = scratch("paused-marks");
        let clean = script(&marks, "clean.sh", &marks.join("filter"), "cat");
        git(&dir, &["config", "filter.evil.clean", clean.to_str().unwrap()]);
        write(&dir, "models/a.sql", "select 1\n");

        let info = status(&dir);
        assert_eq!(info.paused, "filter.evil.clean");
        assert!(!info.repo && info.branch.is_empty());
        assert!(!marks.join("filter").exists(), "git ran in a paused repository");
        let said = tracks(&dir, "models").unwrap_err();
        assert!(said.contains("filter.evil.clean") && said.contains("terminal"), "{said}");
        assert!(!said.contains("not a git repository"));
        assert!(index_of(&dir).is_none());
        assert!(head(&dir).sha.is_empty());
        assert!(!run(&dir, &["status"], &[], Duration::from_secs(20)).ok);
        assert!(!fetch_quiet(&dir));
        assert!(!marks.join("filter").exists(), "git ran in a paused repository");

        // The key is the repository's own, whichever way it got there.
        let json = serde_json::to_value(&info).unwrap();
        assert_eq!(json["paused"], "filter.evil.clean");

        // Mending the configuration lifts the pause, the cache notwithstanding.
        git(&dir, &["config", "--unset", "filter.evil.clean"]);
        let mended = status(&dir);
        assert!(mended.repo && mended.paused.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&marks);
    }

    #[test]
    fn a_credential_helper_or_an_include_pauses_git_too() {
        let dir = scratch("paused-keys");
        repo(&dir);
        git(&dir, &["config", "credential.https://example.com.helper", "!true"]);
        assert_eq!(paused(&dir).as_deref(), Some("credential.https://example.com.helper"));
        git(&dir, &["config", "--unset", "credential.https://example.com.helper"]);
        assert_eq!(paused(&dir), None);

        // A subsection that is a URL with a password is never shown.
        git(&dir, &["config", "credential.https://ada:pw@example.com.helper", "!true"]);
        assert_eq!(paused(&dir).as_deref(), Some("credential.<url>.helper"));
        git(&dir, &["config", "--remove-section", "credential.https://ada:pw@example.com"]);

        // An include is not followed, so a file that appears later or a
        // condition that turns true cannot change a verdict: the key pauses.
        git(&dir, &["config", "include.path", "../extra.cfg"]);
        assert_eq!(paused(&dir).as_deref(), Some("include.path"));
        git(&dir, &["config", "--unset", "include.path"]);
        git(&dir, &["config", "includeIf.onbranch:main.path", "../extra.cfg"]);
        assert_eq!(paused(&dir).as_deref(), Some("includeif.onbranch:main.path"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The keys a clone, a fork and a signing setup leave in `.git/config`.
    #[test]
    fn a_repository_with_only_data_keys_is_not_paused() {
        let dir = scratch("data-keys");
        repo(&dir);
        for (k, v) in [
            ("remote.origin.url", "https://example.com/jaffle.git"),
            ("remote.origin.fetch", "+refs/heads/*:refs/remotes/origin/*"),
            ("branch.main.remote", "origin"),
            ("branch.main.merge", "refs/heads/main"),
            ("core.autocrlf", "input"),
            ("core.fsmonitor", "true"),
            ("user.name", "Ada"),
            ("user.email", "ada@example.com"),
            ("gpg.format", "ssh"),
            ("commit.gpgsign", "true"),
            ("submodule.vendor.url", "https://example.com/vendor.git"),
            ("submodule.vendor.active", "true"),
        ] {
            git(&dir, &["config", k, v]);
        }
        let info = status(&dir);
        assert!(info.repo && info.paused.is_empty(), "{}", info.paused);
        assert_eq!(info.branch, "main");
        assert_eq!(tracks(&dir, "README.md"), Ok(true));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn outside_a_repository_nothing_is_paused() {
        let dir = scratch("not-paused");
        assert_eq!(paused(&dir), None);
        assert!(status(&dir).paused.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The list, key by key: what runs a program pauses, what is only read
    /// does not, and the case of a section or a key counts for nothing.
    #[test]
    fn the_list_of_keys_that_are_only_data() {
        for key in [
            "core.repositoryformatversion", "core.fsmonitor", "core.hooksPath", "core.sshCommand", "branch.main.merge",
            "branch.autoSetupMerge", "remote.origin.url", "remote.Origin.pushurl", "user.signingkey", "commit.gpgsign",
            "gpg.format", "gpg.ssh.allowedSignersFile", "credential.username", "credential.https://x.example.useHttpPath",
            "submodule.a.b.url", "submodule.recurse", "url.git@example.com:.insteadOf", "diff.algorithm", "gc.auto",
            "color.ui", "lfs.url", "lfs.pushurl", "lfs.https://x.example/info/lfs.access", "lfs.locksverify",
            "lfs.fetchinclude", "http.postBuffer", "http.https://x.example/.lowSpeedLimit", "remote.origin.gh-resolved",
            "alias.st", "safe.directory", "pull.ff", "REMOTE.origin.URL", "tag.gpgsign", "gui.wmstate", "gui.geometry",
            "blame.ignoreRevsFile", "blame.markIgnoredLines", "blame.markUnblamableLines", "blame.coloring", "blame.date",
            "blame.showEmail", "blame.showRoot", "blame.blankBoundary", "rerere.enabled", "rerere.autoUpdate",
            "core.excludesFile", "core.preloadIndex", "core.whitespace", "i18n.commitEncoding", "i18n.logOutputEncoding",
            "advice.detachedHead", "advice.statusHints",
        ] {
            assert!(allowed(key), "{key} should be allowed");
        }
        for key in [
            "credential.helper", "credential.https://x.example.helper", "filter.lfs.clean", "filter.lfs.smudge", "core.askpass",
            "core.gitProxy", "core.editor", "core.pager", "core.alternateRefsCommand", "remote.origin.uploadpack",
            "remote.origin.receivepack", "remote.origin.vcs", "protocol.allow", "protocol.ext.allow", "diff.external",
            "diff.word.textconv", "diff.x.command", "merge.x.driver", "gpg.program", "gpg.ssh.program",
            "gpg.ssh.defaultKeyCommand", "submodule.vendor.update", "uploadpack.packObjectsHook", "receive.denyCurrentBranch",
            "trailer.x.cmd", "mergetool.x.cmd", "difftool.x.cmd", "gc.recentObjectsHook", "sequence.editor", "core",
            "ssh.variant", "http", "unknown.key", "include.path", "includeIf.gitdir:~/work/.path", "lfs.extension.x.clean",
            "lfs.customtransfer.x.path", "http.proxy", "http.sslVerify", "http.cookieFile", "http.https://x.example/.proxy",
            "remote.origin.proxy", "remote.origin.proxyAuthMethod", "gui.editor", "gui.fontui", "blame.x.date",
        ] {
            assert!(!allowed(key), "{key} should pause git");
        }
    }

    /// The verdict is kept while the files it was read from stand as they
    /// were: a planted answer is what comes back until one of them changes.
    #[test]
    fn the_verdict_is_kept_until_the_config_changes() {
        let dir = scratch("verdict-cache");
        repo(&dir);
        // Old enough to be trusted: a stamp from this second is read again.
        let config = dir.join(".git/config");
        let old = SystemTime::now() - Duration::from_secs(3600);
        std::fs::OpenOptions::new().write(true).open(&config).unwrap().set_modified(old).unwrap();
        assert_eq!(paused(&dir), None);
        verdicts().get_mut(&dir).unwrap().key = Some("planted.key".into());
        assert_eq!(paused(&dir).as_deref(), Some("planted.key"), "read again with nothing changed");

        let mut text = std::fs::read_to_string(&config).unwrap();
        text.push_str("[user]\n\tname = Ada\n");
        std::fs::write(&config, text).unwrap();
        assert_eq!(paused(&dir), None, "not read again after the config changed");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// What the keys of a clone made by hand, with ssh, signing and a
    /// submodule leave behind, nothing more.
    #[test]
    fn the_keys_an_ordinary_clone_leaves_do_not_pause_git() {
        for key in [
            "core.repositoryformatversion", "core.filemode", "core.bare", "core.logallrefupdates", "core.ignorecase",
            "core.precomposeunicode", "core.sshcommand", "branch.main.remote", "branch.main.merge", "remote.origin.url",
            "remote.origin.fetch", "user.name", "user.email", "gpg.format", "gpg.ssh.allowedsignersfile",
            "commit.gpgsign", "tag.gpgsign", "submodule.vendor.url", "submodule.vendor.active",
        ] {
            assert!(allowed(key), "{key} should not pause git");
        }
    }

    /// A configuration git cannot list is no clean answer: git is paused with
    /// git's reason, and the pause is not remembered.
    #[test]
    fn a_configuration_that_cannot_be_listed_pauses_git_and_is_not_kept() {
        let dir = scratch("unreadable");
        std::fs::write(dir.join(".git"), "not a gitfile\n").unwrap();
        let old = SystemTime::now() - Duration::from_secs(3600);
        std::fs::OpenOptions::new().write(true).open(dir.join(".git")).unwrap().set_modified(old).unwrap();
        let said = paused(&dir).expect("paused");
        assert!(said.starts_with(UNREAD), "{said}");
        assert!(pause_note(&said).contains("could not be read"), "{}", pause_note(&said));
        assert!(!verdicts().contains_key(&dir), "an unread configuration was kept as an answer");
        assert_eq!(status(&dir).paused, said);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A partial clone fetches what it lacks from its promisor remote, and the
    /// poll's `status` wants the commit's tree: the repository's own ssh
    /// command must not be what reaches the remote.
    #[cfg(unix)]
    #[test]
    fn status_in_a_partial_clone_does_not_run_the_repositorys_ssh_command() {
        let dir = scratch("lazy");
        repo(&dir);
        let marks = scratch("lazy-marks");
        let ssh = script(&marks, "ssh.sh", &marks.join("ssh"), "exit 1");
        for (k, v) in [
            ("core.repositoryformatversion", "1"),
            ("extensions.partialClone", "origin"),
            ("remote.origin.promisor", "true"),
            ("remote.origin.partialclonefilter", "blob:none"),
            ("remote.origin.url", "ssh://example.invalid/jaffle.git"),
            ("core.sshCommand", ssh.to_str().unwrap()),
        ] {
            git(&dir, &["config", k, v]);
        }
        let tree = git(&dir, &["rev-parse", "HEAD^{tree}"]).trim().to_string();
        let object = dir.join(".git/objects").join(&tree[..2]).join(&tree[2..]);
        let mut mode = std::fs::metadata(&object).unwrap().permissions();
        std::os::unix::fs::PermissionsExt::set_mode(&mut mode, 0o644);
        std::fs::set_permissions(&object, mode).unwrap();
        std::fs::remove_file(&object).unwrap();

        // Without the two variables git reaches for the remote, by that command.
        let bare = Command::new("git").arg("-C").arg(&dir).args(["status", "--porcelain"]).output().unwrap();
        assert!(!bare.status.success());
        assert!(marks.join("ssh").exists(), "the repository should reach for its remote");
        let _ = std::fs::remove_file(marks.join("ssh"));

        let info = status(&dir);
        assert!(info.paused.is_empty(), "{}", info.paused);
        assert!(!marks.join("ssh").exists(), "the repository's ssh command ran");
        let _ = head(&dir);
        assert!(!marks.join("ssh").exists(), "the repository's ssh command ran");
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&marks);
    }
    /// A project folder that is itself a repository's insides, a `HEAD`, `refs`,
    /// an `objects` that git is content to find, a config naming a filter and
    /// a working tree: git takes it for a bare repository, the search by eye
    /// must not miss it, and nothing may run (0121). The `objects` file is
    /// there to be what the old search, which wanted a directory, missed.
    #[cfg(unix)]
    #[test]
    fn a_folder_that_is_a_repositorys_insides_runs_no_filter() {
        use std::os::unix::fs::PermissionsExt;
        let dir = scratch("embedded");
        let marks = scratch("embedded-marks");
        let clean = script(&marks, "clean.sh", &marks.join("filter"), "cat");
        git(&dir, &["init", "-q", "-b", "main"]);
        write(&dir, "models/a.sql", "select 1\n");
        write(&dir, ".gitattributes", "*.sql filter=x\n");
        git(&dir, &["add", "."]);
        for e in std::fs::read_dir(dir.join(".git")).unwrap().flatten() {
            std::fs::rename(e.path(), dir.join(e.file_name())).unwrap();
        }
        std::fs::remove_dir(dir.join(".git")).unwrap();
        for (k, v) in [("core.bare", "false"), ("core.worktree", "."), ("filter.x.clean", clean.to_str().unwrap())] {
            git(&dir, &["config", "--file", "config", k, v]);
        }
        // Git asks only that `objects` can be entered, which a file marked
        // executable passes.
        std::fs::remove_dir_all(dir.join("objects")).unwrap();
        std::fs::write(dir.join("objects"), "").unwrap();
        std::fs::set_permissions(dir.join("objects"), std::fs::Permissions::from_mode(0o755)).unwrap();
        // The stat of the file no longer matches the index, so a status reads it.
        std::thread::sleep(Duration::from_millis(1100));
        write(&dir, "models/a.sql", "select 1\n");

        let found = Command::new("git").arg("-C").arg(&dir).args(["status", "--porcelain"]).env("GIT_CONFIG_GLOBAL", "/dev/null").output().unwrap();
        assert!(found.status.success(), "{}", String::from_utf8_lossy(&found.stderr));
        assert!(marks.join("filter").exists(), "git should take this folder for a repository and run its filter");
        let _ = std::fs::remove_file(marks.join("filter"));

        assert!(locate(&dir).is_some(), "the search missed what git finds");
        assert!(paused(&dir).is_some());
        let info = status(&dir);
        assert!(!info.paused.is_empty());
        assert!(!run(&dir, &["status"], &[], Duration::from_secs(20)).ok);
        assert!(!marks.join("filter").exists(), "the repository's filter ran");
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&marks);
    }

    /// The same refusal without the pieces that need a Unix: a bare repository
    /// in the project folder is one git would use by itself, and is not used.
    #[test]
    fn a_bare_repository_in_the_project_folder_is_not_used_implicitly() {
        let dir = scratch("bare-folder");
        git(&dir, &["init", "-q", "--bare"]);
        git(&dir, &["config", "filter.x.clean", "true"]);
        let said = paused(&dir).expect("paused");
        assert!(said.starts_with(UNREAD), "{said}");
        assert!(!status(&dir).paused.is_empty());
        assert!(!run(&dir, &["status"], &[], Duration::from_secs(20)).ok);
        assert!(!verdicts().contains_key(&dir), "a refusal was kept as an answer");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Where git looks, and in the order it looks (`locate`).
    #[test]
    fn the_search_for_a_repository_follows_git() {
        let canon = |p: &Path| p.canonicalize().unwrap();

        // A bare repository is its own git directory.
        let bare = scratch("locate-bare");
        git(&bare, &["init", "-q", "--bare"]);
        let at = locate(&bare).expect("bare");
        assert_eq!((canon(&at.git_dir), canon(&at.common_dir), at.gitfile), (canon(&bare), canon(&bare), None));

        // A linked worktree: a `.git` file, and the configuration of the main repository.
        let main = scratch("locate-main");
        repo(&main);
        let linked = scratch("locate-linked");
        git(&main, &["worktree", "add", "-q", "-b", "side", linked.to_str().unwrap()]);
        let at = locate(&linked).expect("linked");
        assert_eq!(at.gitfile.as_deref(), Some(linked.join(".git").as_path()));
        assert_eq!(canon(&at.common_dir), canon(&main.join(".git")));
        assert_ne!(canon(&at.git_dir), canon(&main.join(".git")));

        // A submodule: a `.git` file with a relative path into the superproject.
        let top = scratch("locate-top");
        git(&top, &["init", "-q"]);
        let module = top.join(".git/modules/x");
        std::fs::create_dir_all(top.join("x")).unwrap();
        git(&top, &["init", "-q", "--bare", module.to_str().unwrap()]);
        std::fs::write(top.join("x/.git"), "gitdir: ../.git/modules/x\n").unwrap();
        let at = locate(&top.join("x")).expect("submodule");
        assert_eq!(canon(&at.git_dir), canon(&module));
        assert_eq!(canon(&at.common_dir), canon(&module));
        assert_eq!(at.gitfile.as_deref(), Some(top.join("x/.git").as_path()));

        // A `.git` directory that is no git directory is passed over, as git does.
        let parent = scratch("locate-invalid");
        git(&parent, &["init", "-q"]);
        std::fs::create_dir_all(parent.join("child/.git")).unwrap();
        let at = locate(&parent.join("child")).expect("parent's");
        assert_eq!(canon(&at.git_dir), canon(&parent.join(".git")));
        assert!(status(&parent.join("child")).repo, "git should find the parent's repository too");

        for d in [bare, main, linked, top, parent] {
            let _ = std::fs::remove_dir_all(&d);
        }
    }

    /// A linked worktree reads the main repository's `config`, so a key added
    /// there is seen from the worktree.
    #[test]
    fn a_change_of_the_main_configuration_is_seen_from_a_linked_worktree() {
        let main = scratch("shared-main");
        repo(&main);
        let linked = scratch("shared-linked");
        git(&main, &["worktree", "add", "-q", "-b", "side", linked.to_str().unwrap()]);
        // Old enough to be trusted: a stamp from this second is read again.
        let old = SystemTime::now() - Duration::from_secs(3600);
        for file in [main.join(".git/config"), linked.join(".git")] {
            std::fs::OpenOptions::new().write(true).open(file).unwrap().set_modified(old).unwrap();
        }
        assert_eq!(paused(&linked), None);
        assert!(verdicts().contains_key(&linked), "a clean verdict was not kept");

        let mut text = std::fs::read_to_string(main.join(".git/config")).unwrap();
        text.push_str("[filter \"x\"]\n\tclean = cat\n");
        std::fs::write(main.join(".git/config"), text).unwrap();
        std::thread::sleep(Duration::from_millis(2100));
        assert_eq!(paused(&linked).as_deref(), Some("filter.x.clean"));
        assert!(!status(&linked).paused.is_empty());
        let _ = std::fs::remove_dir_all(&main);
        let _ = std::fs::remove_dir_all(&linked);
    }
}
