//! Whether a newer dbt-edith is published, and the command that installs it
//! here (0068).
//!
//! The one question this binary puts to the network unasked, besides the
//! project's own fetch behind the freshness badge (0025): which `vX.Y.Z` tags
//! dbt-edith's public repository holds. git asks it, through `ls-remote`, as a
//! pull of that repository would, so no HTTP client comes in (0003), and
//! nothing about the project goes with it.
//!
//! Three rules hold here:
//!   - once a day, for every dbt-edith on the machine: the answer is kept in
//!     the config directory with its time, and a start within the day takes it
//!     from there. Starts that overlap before an answer is kept each ask once:
//!     nothing locks the file for a request this small. A failed attempt waits
//!     the day out too, in this process. `--no-update-check` asks nothing and
//!     shows nothing.
//!   - a failed check says nothing. Offline, or behind a proxy git cannot
//!     pass, the status bar stays as it was.
//!   - nothing installs on its own. The update is a command typed into the
//!     terminal for the user to run, as collin's install is (0042), and only
//!     for a binary built from a clone on this machine, where updating means a
//!     pull and a rebuild. Any other, a copy carried from another machine above
//!     all, is told what is out and why there is no command.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::collin_run::{is_version, semver};

/// dbt-edith's repository, public, so git needs no account to list its tags.
pub const REPO: &str = "https://github.com/IL-William/dbt-edith";

/// How long an answer stands before the next look.
const DAY: u64 = 86_400;

/// Where the answer is kept, beside the per-project settings (0011).
pub fn seen_path(config_dir: &Path) -> PathBuf {
    config_dir.join("update.json")
}

/// What GitHub last answered, and when.
#[derive(serde::Serialize, serde::Deserialize, Default, Clone, Debug, PartialEq)]
pub struct Seen {
    /// Seconds since the epoch.
    pub checked: u64,
    /// The newest release it listed, `1.12.0`.
    pub latest: String,
}

/// The answer kept, unless the file is missing or names no version.
pub fn load(path: &Path) -> Option<Seen> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str::<Seen>(&text).ok().filter(|s| is_version(&s.latest))
}

/// Kept for the next start. A failure costs that start one more look.
fn save(path: &Path, seen: &Seen) {
    if let Ok(bytes) = serde_json::to_vec_pretty(seen) {
        let _ = crate::settings::write_atomic(path, &bytes);
    }
}

/// Whether a look made at `checked` is too old to stand. One dated after
/// `now`, by a clock set back since, is.
fn due(checked: Option<u64>, now: u64) -> bool {
    checked.map_or(true, |t| t > now || now - t >= DAY)
}

/// The newest `vX.Y.Z` in what `git ls-remote --tags --refs` printed. A tag
/// of any other shape, a pre-release among them, is not a release.
pub fn newest(listed: &str) -> Option<String> {
    listed
        .lines()
        .filter_map(|line| line.split('\t').nth(1)?.trim().strip_prefix("refs/tags/v"))
        .filter(|v| is_version(v))
        .max_by_key(|v| semver(v))
        .map(str::to_string)
}

/// Whether `latest` is a later release than `current`.
pub fn newer(current: &str, latest: &str) -> bool {
    matches!((semver(current), semver(latest)), (Some(c), Some(l)) if l > c)
}

/// Asks GitHub, from the temporary directory: `ls-remote` on a URL needs no
/// repository, and none is looked for above it, so no project's git
/// configuration has a say in it. The user's own has, as in their pulls: a
/// proxy the machine needs above all.
pub fn ask() -> Option<String> {
    crate::git::ls_remote_tags(&std::env::temp_dir(), REPO).as_deref().and_then(newest)
}

pub fn now_secs() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// What the status bar is told, and what decides when to look again.
#[derive(Default)]
struct Known {
    checking: bool,
    /// The last answer this process got, for a machine with no config
    /// directory to keep it in.
    seen: Option<Seen>,
    /// When this process last asked, answered or not.
    asked: Option<u64>,
}

pub struct Update {
    /// `--no-update-check`: nothing asked, nothing shown.
    pub off: bool,
    /// Where the answer is kept. None when there is no config directory.
    file: Option<PathBuf>,
    /// The command that installs a newer one here, or why there is none.
    pub how: Result<How, Why>,
    known: Mutex<Known>,
}

impl Update {
    pub fn new(off: bool, file: Option<PathBuf>, how: Result<How, Why>) -> Update {
        Update { off, file, how, known: Mutex::default() }
    }

    /// The newest release known, empty until one is, and whether a look is
    /// on its way.
    pub fn status(&self) -> (String, bool) {
        self.known
            .lock()
            .map(|k| (k.seen.as_ref().map(|s| s.latest.clone()).unwrap_or_default(), k.checking))
            .unwrap_or_default()
    }

    /// The answer kept by any dbt-edith on the machine, as the banner prints
    /// it before the first look.
    pub fn kept(&self) -> Option<Seen> {
        self.file.as_deref().and_then(load)
    }

    /// Looks for a newer release unless the day's look was made, here or by
    /// another dbt-edith whose answer is in the file.
    pub async fn check(&self) {
        self.check_with(now_secs(), ask).await
    }

    async fn check_with<F>(&self, now: u64, ask: F)
    where
        F: FnOnce() -> Option<String> + Send + 'static,
    {
        if self.off {
            return;
        }
        let file = self.file.clone();
        let kept = tokio::task::spawn_blocking(move || file.as_deref().and_then(load)).await.ok().flatten();
        let asked = {
            let Ok(mut k) = self.known.lock() else { return };
            let seen = match (kept, k.seen.take()) {
                (Some(a), Some(b)) => Some(if a.checked >= b.checked { a } else { b }),
                (a, b) => a.or(b),
            };
            let due = due(seen.as_ref().map(|s| s.checked), now) && due(k.asked, now);
            k.seen = seen;
            if due {
                k.asked = Some(now);
                k.checking = true;
            }
            due
        };
        if !asked {
            return;
        }
        let latest = tokio::task::spawn_blocking(ask).await.ok().flatten();
        if let Some(latest) = &latest {
            let seen = Seen { checked: now, latest: latest.clone() };
            if let Some(path) = self.file.clone() {
                let kept = seen.clone();
                let _ = tokio::task::spawn_blocking(move || save(&path, &kept)).await;
            }
            if let Ok(mut k) = self.known.lock() {
                k.seen = Some(seen);
            }
        }
        if let Ok(mut k) = self.known.lock() {
            k.checking = false;
        }
    }
}

/// What the update entry types into the terminal, and what the page says
/// about it.
#[derive(serde::Serialize, Clone, Debug, PartialEq)]
pub struct How {
    pub command: String,
    /// `install` for a binary `cargo install` put in place, `build` for one
    /// `cargo build --release` left in the clone.
    pub kind: &'static str,
    /// The clone it pulls.
    pub source: String,
    /// The running binary's file name.
    pub exe: String,
    /// Whether the running binary is renamed out of the way first: Windows
    /// will not replace an executable that runs, but lets it be renamed.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub renames: bool,
}

/// Why there is no command to offer, for the page to say.
#[derive(serde::Serialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Why {
    /// Neither installed by cargo nor built in a clone still on this machine.
    Elsewhere,
    /// Built in its clone, on Windows. cargo links the binary there from the
    /// one it writes in `deps/`, the same file under another name, so
    /// renaming the running one frees its name but not that file, which the
    /// build must overwrite.
    Locked,
    /// The terminal's shell is none the command is written for.
    Shell,
    /// A path holds a character a shell would read inside the quotes.
    Path,
}

/// The two command languages the update can be written in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Shell {
    Posix,
    PowerShell,
}

/// The terminal's shell, by the program's name. Any other gets no command:
/// `cmd` has neither language, fish reads the parentheses as a command
/// substitution, nushell has no `&&`, and `wsl` would look for a Windows path
/// inside Linux.
pub fn shell_kind(program: &str) -> Option<Shell> {
    let name = program.rsplit(['/', '\\']).next().unwrap_or(program).to_ascii_lowercase();
    match name.strip_suffix(".exe").unwrap_or(&name) {
        "sh" | "bash" | "zsh" | "dash" | "ksh" => Some(Shell::Posix),
        "powershell" | "pwsh" => Some(Shell::PowerShell),
        _ => None,
    }
}

/// How this binary was made, read from where it sits, and the command that
/// makes it again from a newer clone. `source` is the folder it was built
/// from, which has to be a clone still on this machine; `exe` is the running
/// binary. It was installed by cargo when it sits in a `bin/` whose parent
/// holds cargo's records of what it installed, and built in place when it sits
/// in the clone's `target/release/`, except on Windows. Anything else gets no
/// command, and the reason. Both paths come canonical, so equal places compare
/// equal.
pub fn how(source: &Path, exe: &Path, shell: Option<Shell>, windows: bool) -> Result<How, Why> {
    if !source.join(".git").exists() || !source.join("Cargo.toml").is_file() {
        return Err(Why::Elsewhere);
    }
    let dir = exe.parent().ok_or(Why::Elsewhere)?;
    let installed = dir.file_name().is_some_and(|n| n == "bin")
        && dir.parent().is_some_and(|root| root.join(".crates2.json").is_file() || root.join(".crates.toml").is_file());
    let built = dir == source.join("target").join("release");
    if !installed && !built {
        return Err(Why::Elsewhere);
    }
    if built && windows {
        return Err(Why::Locked);
    }
    let shell = shell.ok_or(Why::Shell)?;
    let spell = |p: &Path| quotable(p, windows).ok_or(Why::Path);
    let src = spell(source)?;
    let (kind, step) = if installed {
        ("install", format!("cargo install --locked --path \"{src}\""))
    } else {
        ("build", format!("cargo build --release --locked --manifest-path \"{}\"", spell(&source.join("Cargo.toml"))?))
    };
    let swap = if windows { Some(spell(exe)?) } else { None };
    Ok(How {
        command: compose(shell, &src, &step, swap.as_deref()),
        kind,
        source: src,
        exe: exe.file_name().ok_or(Why::Elsewhere)?.to_string_lossy().into_owned(),
        renames: windows,
    })
}

/// A path as both shells read it inside double quotes: forward slashes on
/// Windows, which Git Bash and PowerShell both take. None for one holding a
/// character either would expand or end the quotes on: `!` is bash's and
/// zsh's history there, and PowerShell ends a string on a typographic quote
/// too. On Windows, a bracket as well, which PowerShell reads as a wildcard.
fn quotable(path: &Path, windows: bool) -> Option<String> {
    let s = path.to_str()?;
    let s = if windows { s.replace('\\', "/") } else { s.to_string() };
    let odd = |c: char| {
        matches!(c, '"' | '\'' | '$' | '`' | '\\' | '!' | '\u{201C}' | '\u{201D}' | '\u{201E}')
            || (windows && matches!(c, '[' | ']'))
            || c.is_control()
    };
    (!s.chars().any(odd)).then_some(s)
}

/// Pull, then rebuild, each only once the step before it worked. The pull
/// names `main`, where every release is tagged (0045), whatever branch the
/// clone has out: one that cannot fast-forward to it stops the command there.
/// With `swap`, the running binary is renamed to `<exe>.old` before the
/// rebuild and put back when the rebuild fails, so a failed update still
/// leaves one to start.
fn compose(shell: Shell, source: &str, step: &str, swap: Option<&str>) -> String {
    let pull = format!("git -C \"{source}\" pull --ff-only origin main");
    match (shell, swap) {
        (Shell::Posix, None) => format!("{pull} && {step}"),
        (Shell::Posix, Some(exe)) => {
            format!("{pull} && mv -f \"{exe}\" \"{exe}.old\" && ({step} || mv -f \"{exe}.old\" \"{exe}\")")
        }
        (Shell::PowerShell, None) => format!("{pull}; if ($?) {{ {step} }}"),
        (Shell::PowerShell, Some(exe)) => format!(
            "{pull}; if ($?) {{ Move-Item -Force -LiteralPath \"{exe}\" -Destination \"{exe}.old\"; if ($?) {{ {step}; \
             if (-not $?) {{ Move-Item -Force -LiteralPath \"{exe}.old\" -Destination \"{exe}\" }} }} }}"
        ),
    }
}

/// The page that says what a release changed: its section of CHANGELOG.md,
/// published by the release workflow (0045).
pub fn notes_url(version: &str) -> String {
    format!("{REPO}/releases/tag/v{version}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dbt-edith-update-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn touch(path: &Path) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, "").unwrap();
    }

    #[test]
    fn the_newest_release_is_the_highest_version_tag_and_nothing_else() {
        let listed = "a1\trefs/tags/v1.9.3\nb2\trefs/tags/v1.10.0\nc3\trefs/tags/v2.0.0-rc1\n\
                      d4\trefs/tags/collin-v9.0.0\ne5\trefs/tags/v1.2\nf6\trefs/heads/v3.0.0\n";
        assert_eq!(newest(listed), Some("1.10.0".to_string()));
        assert_eq!(newest(""), None);
        assert_eq!(newest("a1\trefs/tags/latest\n"), None);
    }

    #[test]
    fn a_release_is_newer_by_its_numbers_not_its_spelling() {
        assert!(newer("1.9.0", "1.10.0"));
        assert!(newer("1.11.0", "2.0.0"));
        assert!(!newer("1.11.0", "1.11.0"));
        assert!(!newer("1.12.0", "1.11.0"));
        assert!(!newer("1.11.0", ""));
    }

    #[test]
    fn a_look_stands_for_a_day_and_never_from_the_future() {
        let now = 10 * DAY;
        assert!(due(None, now));
        assert!(!due(Some(now - DAY + 1), now));
        assert!(due(Some(now - DAY), now));
        assert!(due(Some(now + 60), now));
    }

    #[test]
    fn the_answer_is_kept_and_one_naming_no_version_is_not_believed() {
        let dir = scratch("seen");
        let path = seen_path(&dir);
        let seen = Seen { checked: 42, latest: "1.12.0".into() };
        save(&path, &seen);
        assert_eq!(load(&path), Some(seen));
        std::fs::write(&path, r#"{"checked": 42, "latest": "../../x"}"#).unwrap();
        assert_eq!(load(&path), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// An asker that counts its calls and answers `latest`.
    fn asker(latest: Option<&str>) -> (Arc<AtomicUsize>, impl FnOnce() -> Option<String> + Send + 'static) {
        let calls = Arc::new(AtomicUsize::new(0));
        let (count, latest) = (calls.clone(), latest.map(str::to_string));
        (calls, move || {
            count.fetch_add(1, Ordering::SeqCst);
            latest
        })
    }

    #[tokio::test]
    async fn one_look_a_day_answered_or_not_and_the_answer_is_shared_through_the_file() {
        let dir = scratch("daily");
        let now = 100 * DAY;
        let u = Update::new(false, Some(seen_path(&dir)), Err(Why::Elsewhere));

        let (calls, ask) = asker(Some("1.12.0"));
        u.check_with(now, ask).await;
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(u.status(), ("1.12.0".to_string(), false));

        // Another start the same day reads it from the file and asks nothing.
        let other = Update::new(false, Some(seen_path(&dir)), Err(Why::Elsewhere));
        let (calls, ask) = asker(Some("9.9.9"));
        other.check_with(now + 3600, ask).await;
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(other.status().0, "1.12.0");

        // A day later it asks again.
        let (calls, ask) = asker(Some("1.13.0"));
        other.check_with(now + DAY, ask).await;
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(other.status().0, "1.13.0");

        // A failed look is kept nowhere, yet waits the day out in its process.
        let fresh = Update::new(false, None, Err(Why::Elsewhere));
        let (calls, ask) = asker(None);
        fresh.check_with(now, ask).await;
        let (again, ask) = asker(Some("1.12.0"));
        fresh.check_with(now + 3600, ask).await;
        assert_eq!((calls.load(Ordering::SeqCst), again.load(Ordering::SeqCst)), (1, 0));
        assert_eq!(fresh.status(), (String::new(), false));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn switched_off_it_asks_nothing() {
        let u = Update::new(true, None, Err(Why::Elsewhere));
        let (calls, ask) = asker(Some("1.12.0"));
        u.check_with(100 * DAY, ask).await;
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(u.status(), (String::new(), false));
    }

    /// A clone, and the cargo install root beside it, both in the folder the
    /// test removes.
    fn layout(tag: &str) -> (PathBuf, PathBuf) {
        let dir = scratch(tag);
        let source = dir.join("dbt-edith");
        std::fs::create_dir_all(source.join(".git")).unwrap();
        touch(&source.join("Cargo.toml"));
        let root = dir.join("cargo");
        touch(&root.join(".crates2.json"));
        touch(&root.join("bin").join("dbt-edith"));
        (source, root)
    }

    #[test]
    fn a_binary_cargo_installed_is_pulled_and_installed_again() {
        let (source, root) = layout("installed");
        let how = how(&source, &root.join("bin").join("dbt-edith"), Some(Shell::Posix), false).unwrap();
        let s = source.display();
        assert_eq!(
            how.command,
            format!("git -C \"{s}\" pull --ff-only origin main && cargo install --locked --path \"{s}\"")
        );
        assert_eq!((how.kind, how.renames, how.exe.as_str()), ("install", false, "dbt-edith"));
        let _ = std::fs::remove_dir_all(source.parent().unwrap());
    }

    #[test]
    fn a_binary_built_in_the_clone_is_built_there_again() {
        let (source, _) = layout("built");
        let exe = source.join("target").join("release").join("dbt-edith");
        let built = how(&source, &exe, Some(Shell::Posix), false).unwrap();
        assert_eq!(built.kind, "build");
        assert!(built.command.ends_with(&format!("cargo build --release --locked --manifest-path \"{}\"", source.join("Cargo.toml").display())));
        // Not on Windows, where the build would overwrite the running file.
        assert_eq!(how(&source, &exe, Some(Shell::Posix), true), Err(Why::Locked));
        let _ = std::fs::remove_dir_all(source.parent().unwrap());
    }

    #[test]
    fn a_binary_from_anywhere_else_gets_no_command() {
        let (source, root) = layout("elsewhere");
        let exe = root.join("bin").join("dbt-edith");
        // Carried here from the machine that built it.
        assert_eq!(how(&source.join("gone"), &exe, Some(Shell::Posix), false), Err(Why::Elsewhere));
        // A debug build, which `cargo run` makes.
        let debug = source.join("target").join("debug").join("dbt-edith");
        assert_eq!(how(&source, &debug, Some(Shell::Posix), false), Err(Why::Elsewhere));
        // A `bin/` cargo did not fill.
        std::fs::remove_file(root.join(".crates2.json")).unwrap();
        assert_eq!(how(&source, &exe, Some(Shell::Posix), false), Err(Why::Elsewhere));
        // A shell neither language is for, said once the place is known.
        touch(&root.join(".crates.toml"));
        assert_eq!(how(&source, &exe, shell_kind("C:\\Windows\\System32\\cmd.exe"), false), Err(Why::Shell));
        assert_eq!(how(&source.join("gone"), &exe, None, false), Err(Why::Elsewhere));
        assert!(how(&source, &exe, Some(Shell::Posix), false).is_ok());
        let _ = std::fs::remove_dir_all(source.parent().unwrap());
    }

    #[test]
    fn a_path_a_shell_would_expand_gets_no_command() {
        let (source, root) = layout("quoted");
        let exe = root.join("bin").join("dbt-edith");
        for name in ["it's $HOME", "src!", "\u{201C}quoted\u{201D}"] {
            let odd = source.parent().unwrap().join(name);
            std::fs::rename(&source, &odd).unwrap();
            assert_eq!(how(&odd, &exe, Some(Shell::Posix), false), Err(Why::Path), "{name}");
            std::fs::rename(&odd, &source).unwrap();
        }
        // A bracket is a wildcard to PowerShell, so it counts on Windows alone.
        assert_eq!(quotable(Path::new("/src/tools[1]"), false).as_deref(), Some("/src/tools[1]"));
        assert_eq!(quotable(Path::new(r"C:\tools[1]\bin"), true), None);
        let _ = std::fs::remove_dir_all(source.parent().unwrap());
    }

    #[test]
    fn on_windows_the_running_exe_steps_aside_and_comes_back_if_the_build_fails() {
        let exe = "C:/Users/ada/.cargo/bin/dbt-edith.exe";
        let step = "cargo install --locked --path \"C:/src/dbt-edith\"";
        assert_eq!(
            compose(Shell::Posix, "C:/src/dbt-edith", step, Some(exe)),
            format!(
                "git -C \"C:/src/dbt-edith\" pull --ff-only origin main && mv -f \"{exe}\" \"{exe}.old\" && \
                 ({step} || mv -f \"{exe}.old\" \"{exe}\")"
            )
        );
        assert_eq!(
            compose(Shell::PowerShell, "C:/src/dbt-edith", step, Some(exe)),
            format!(
                "git -C \"C:/src/dbt-edith\" pull --ff-only origin main; if ($?) {{ \
                 Move-Item -Force -LiteralPath \"{exe}\" -Destination \"{exe}.old\"; if ($?) {{ {step}; \
                 if (-not $?) {{ Move-Item -Force -LiteralPath \"{exe}.old\" -Destination \"{exe}\" }} }} }}"
            )
        );
        assert_eq!(quotable(Path::new(r"C:\Users\ada\.cargo\bin\dbt-edith.exe"), true).as_deref(), Some(exe));
    }

    #[test]
    fn the_shell_is_told_by_its_program() {
        assert_eq!(shell_kind("/bin/zsh"), Some(Shell::Posix));
        assert_eq!(shell_kind(r"C:\Program Files\Git\bin\bash.exe"), Some(Shell::Posix));
        assert_eq!(shell_kind("powershell.exe"), Some(Shell::PowerShell));
        assert_eq!(shell_kind("pwsh"), Some(Shell::PowerShell));
        assert_eq!(shell_kind("CMD.EXE"), None);
        assert_eq!(shell_kind("/opt/homebrew/bin/fish"), None);
        assert_eq!(shell_kind("nu"), None);
        assert_eq!(shell_kind(r"C:\Windows\System32\wsl.exe"), None);
    }

    /// The real git, on a local repository standing for GitHub, asked from
    /// inside another repository whose configuration would send it elsewhere.
    #[test]
    fn git_lists_the_tags_the_newest_is_read_from() {
        let dir = scratch("remote");
        let remote = dir.join("remote");
        let git = |at: &Path, args: &[&str]| {
            let out = std::process::Command::new("git")
                .arg("-C")
                .arg(at)
                .args(args)
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env("GIT_AUTHOR_NAME", "Ada")
                .env("GIT_AUTHOR_EMAIL", "ada@example.com")
                .env("GIT_COMMITTER_NAME", "Ada")
                .env("GIT_COMMITTER_EMAIL", "ada@example.com")
                .output()
                .unwrap();
            assert!(out.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&out.stderr));
        };
        std::fs::create_dir_all(&remote).unwrap();
        git(&remote, &["init", "-q"]);
        git(&remote, &["commit", "-q", "--allow-empty", "-m", "one"]);
        for tag in ["v1.9.3", "v1.10.0", "v2.0.0-rc1"] {
            git(&remote, &["tag", tag]);
        }
        let url = remote.to_str().unwrap();
        let inside = dir.join("inside");
        std::fs::create_dir_all(&inside).unwrap();
        git(&dir, &["init", "-q"]);
        git(&dir, &["config", &format!("url.{}.insteadOf", dir.join("nowhere").display()), url]);

        let listed = crate::git::ls_remote_tags(&inside, url).unwrap();
        assert_eq!(newest(&listed), Some("1.10.0".to_string()));
        assert_eq!(crate::git::ls_remote_tags(&inside, remote.join("missing").to_str().unwrap()), None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
