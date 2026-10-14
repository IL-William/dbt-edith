//! Whether a newer dbt-edith is published, and the command that installs it
//! here (0068, 0079).
//!
//! The one question this binary puts to the network unasked, besides the
//! project's own fetch behind the freshness badge (0025): which `vX.Y.Z` tags
//! dbt-edith's public repository holds, and, when the running one is behind,
//! the subjects of the commits the newer ones name, its pull requests' titles,
//! once per release for the machine. git asks both, through `ls-remote` and a
//! fetch of those commits alone, as a pull of that repository would, so no
//! HTTP client comes in (0003), and nothing about the project goes with it.
//!
//! Three rules hold here:
//!   - an answer stands until the next turn, every hour on the hour from
//!     midnight to 09:00 in Paris and then midnight, for every dbt-edith on
//!     the machine: it is kept in the config directory with its time, and a
//!     start before the next turn takes it from there. Starts
//!     that overlap before an answer is kept each ask once: nothing locks the
//!     file for a request this small. A look that got no answer is tried again
//!     an hour later, in this process. The menu's button asks at once.
//!     `--no-update-check` asks nothing and shows nothing.
//!   - a failed look stays out of the status bar. Offline, or behind a proxy
//!     git cannot pass, the bar stays as it was, and only the menu says what
//!     git answered.
//!   - nothing installs on its own. The update is a command typed into the
//!     terminal for the user to run, as collin's install is (0042), and only
//!     for a binary built from a clone on this machine, where updating means a
//!     pull and a rebuild. Any other, a copy carried from another machine above
//!     all, is told what is out and why there is no command.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::collin_run::{is_version, semver};

/// dbt-edith's repository, public, so git needs no account to list its tags.
pub const REPO: &str = "https://github.com/IL-William/dbt-edith";

const HOUR: i64 = 3_600;
const DAY: i64 = 86_400;

/// How many of the releases since the running one the menu lists, newest
/// first. Older ones are counted, and the releases page has them.
const LISTED: usize = 20;

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
    /// Every release it listed, newest first, so a start that takes the
    /// answer from here knows which ones it is behind. Absent from a file an
    /// older dbt-edith wrote, which reads past it in turn.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub releases: Vec<String>,
    /// The title of a release, its pull request's, by version: fetched once
    /// for every dbt-edith on the machine, since it never changes.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub titles: BTreeMap<String, String>,
}

impl Seen {
    /// The later of two answers, with the titles both know.
    fn merged(self, other: Seen) -> Seen {
        let (mut later, earlier) = if self.checked >= other.checked { (self, other) } else { (other, self) };
        for (version, title) in earlier.titles {
            later.titles.entry(version).or_insert(title);
        }
        later
    }
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

/// Days from 1970-01-01 to a date, by Howard Hinnant's `days_from_civil`.
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    era * 146_097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719_468
}

/// The year a day counted from 1970-01-01 falls in, by the inverse,
/// `civil_from_days`.
fn year_of(days: i64) -> i64 {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let march_based = (5 * doy + 2) / 153;
    yoe + era * 400 + i64::from(march_based >= 10)
}

/// The last Sunday of a 31-day month, in days from 1970-01-01, a Thursday.
fn last_sunday(y: i64, m: i64) -> i64 {
    let d = days_from_civil(y, m, 31);
    d - (d + 4).rem_euclid(7)
}

/// Paris's offset from UTC at `t`, in seconds: an hour, and two in summer
/// time, which runs from the last Sunday of March to the last Sunday of
/// October, at 01:00 UTC both times, as the EU has set it since 1996. Written
/// out here because the standard library knows no time zone, and a crate for
/// one rule would be a dependency for nothing (0003).
fn paris_offset(t: i64) -> i64 {
    let y = year_of(t.div_euclid(DAY));
    let summer = last_sunday(y, 3) * DAY + HOUR..last_sunday(y, 10) * DAY + HOUR;
    if summer.contains(&t) {
        2 * HOUR
    } else {
        HOUR
    }
}

/// The instant a Paris wall-clock time names, given as seconds since the epoch
/// as if Paris kept UTC. Of the hour the clocks repeat in October, the first;
/// the hour they skip in March names the moment summer time starts.
fn from_paris(wall: i64) -> i64 {
    if paris_offset(wall - 2 * HOUR) == 2 * HOUR {
        wall - 2 * HOUR
    } else {
        wall - HOUR
    }
}

/// Paris midnight that starts the day of `now`, as a wall-clock time, and how
/// far into that day `now` is.
fn paris_day(now: i64) -> (i64, i64) {
    let wall = now + paris_offset(now);
    let day = wall.div_euclid(DAY) * DAY;
    (day, wall - day)
}

/// The last turn at or before `now`, every hour on the hour from midnight to
/// 09:00 in Paris: an answer from before it no longer stands. dbt-edith is
/// released from Paris, overnight there, so a dbt-edith left running sees a
/// release within the hour, and from 09:00 everything the night brought is
/// out. Past 09:00 the next is midnight. Every night crosses one, wherever the
/// machine is, any fifteen hours holding one, so the first start of anyone's
/// day asks.
fn last_turn(now: i64) -> i64 {
    let (day, into) = paris_day(now);
    from_paris(day + (into - into % HOUR).min(9 * HOUR))
}

/// The first turn after `now`, when a running dbt-edith asks again.
fn next_turn(now: u64) -> u64 {
    let now = now as i64;
    let (day, into) = paris_day(now);
    let next = if into < 9 * HOUR { day + into - into % HOUR + HOUR } else { day + DAY };
    from_paris(next).max(now + 1) as u64
}

/// Whether a look made at `checked` no longer stands at `now`: a turn came
/// between them, or it is dated after `now`, by a clock set back since.
fn stale(checked: Option<u64>, now: u64) -> bool {
    checked.map_or(true, |t| t > now || (t as i64) < last_turn(now as i64))
}

/// Every `vX.Y.Z` in what `git ls-remote --tags --refs` printed, newest
/// first. A tag of any other shape, a pre-release among them, is not a
/// release.
pub fn releases(listed: &str) -> Vec<String> {
    let mut found: Vec<&str> = listed
        .lines()
        .filter_map(|line| line.split('\t').nth(1)?.trim().strip_prefix("refs/tags/v"))
        .filter(|v| is_version(v))
        .collect();
    found.sort_by_key(|v| std::cmp::Reverse(semver(v)));
    found.dedup();
    found.into_iter().map(str::to_string).collect()
}

/// The releases after `current`, newest first, up to `LISTED`, and how many
/// older ones that leaves out.
fn missed(releases: &[String], current: &str) -> (Vec<String>, usize) {
    let after: Vec<&String> = releases.iter().filter(|v| newer(current, v)).collect();
    let left_out = after.len().saturating_sub(LISTED);
    (after.into_iter().take(LISTED).cloned().collect(), left_out)
}

/// A squashed commit's subject as the menu lists it: the pull request's
/// title, without the ` (#39)` GitHub adds to it, and cut to a line.
fn title(subject: &str) -> String {
    let subject = subject.trim();
    let bare = subject
        .strip_suffix(')')
        .and_then(|s| s.rsplit_once(" (#"))
        .filter(|(_, n)| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
        .map_or(subject, |(head, _)| head);
    match bare.char_indices().nth(200) {
        Some((cut, _)) => format!("{}…", &bare[..cut]),
        None => bare.to_string(),
    }
}

/// Whether `latest` is a later release than `current`.
pub fn newer(current: &str, latest: &str) -> bool {
    matches!((semver(current), semver(latest)), (Some(c), Some(l)) if l > c)
}

/// git's reason, as the menu shows it: a URL's user and password taken out,
/// since a `url.<base>.insteadOf` in the user's configuration can put them in
/// the address git names, and cut to a line the menu can hold.
fn plain(reason: &str) -> String {
    let mut out = String::new();
    let mut rest = reason;
    while let Some(at) = rest.find("://") {
        let (head, tail) = rest.split_at(at + 3);
        out.push_str(head);
        let host_end = tail.find(|c: char| c == '/' || c == '\'' || c == '"' || c.is_whitespace()).unwrap_or(tail.len());
        let authority = &tail[..host_end];
        out.push_str(authority.rsplit_once('@').map_or(authority, |(_, host)| host));
        rest = &tail[host_end..];
    }
    out.push_str(rest);
    match out.char_indices().nth(240) {
        Some((cut, _)) => format!("{}…", &out[..cut]),
        None => out,
    }
}

/// Asks GitHub, from the temporary directory: `ls-remote` on a URL needs no
/// repository, and none is looked for above it, so no project's git
/// configuration has a say in it. The user's own has, as in their pulls: a
/// proxy the machine needs above all. Every release, newest first, or why
/// there is no answer.
pub fn ask() -> Result<Vec<String>, String> {
    let listed = crate::git::ls_remote_tags(&std::env::temp_dir(), REPO).map_err(|e| plain(&e))?;
    let found = releases(&listed);
    if found.is_empty() {
        return Err("GitHub listed no release".to_string());
    }
    Ok(found)
}

/// The pull request title each of `versions` was released by. Every release
/// is one pull request, squashed into one commit whose subject is its title,
/// and the release workflow tags that commit (0045), so the tagged commits are
/// all it takes: fetched through git, as the look is, without a file.
pub fn titles(versions: Vec<String>) -> Result<Vec<(String, String)>, String> {
    let tags: Vec<String> = versions.iter().map(|v| format!("v{v}")).collect();
    let found = crate::git::tag_subjects(&std::env::temp_dir(), REPO, &tags).map_err(|e| plain(&e))?;
    Ok(found.into_iter().filter_map(|(tag, subject)| Some((tag.strip_prefix('v')?.to_string(), title(&subject)))).collect())
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
    /// This process's last look that got no answer, when and what git said.
    /// The next waits an hour, not until the next turn, so a start made
    /// before the network was up is not left without an answer until midnight.
    failed: Option<(u64, String)>,
}

/// What `/api/update` says of the looks.
#[derive(Default, Debug, PartialEq)]
pub struct Status {
    /// The newest release known, empty until one is.
    pub latest: String,
    /// When that answer was got, 0 for none.
    pub checked: u64,
    /// A look is on its way.
    pub checking: bool,
    /// When the last look got no answer, 0 unless that is later than the
    /// answer, and what git said.
    pub failed: u64,
    pub error: String,
    /// When the next look is due, 0 while one is on its way.
    pub next: u64,
    /// The releases after the running one, newest first, each with its
    /// title, while it is behind and every one listed has its title. Empty
    /// otherwise, and the menu links the newest one's notes alone.
    pub releases: Vec<(String, String)>,
    /// How many older ones the list leaves out.
    pub older: usize,
}

/// The next turn, or an hour after a look that got no answer while no answer
/// stands. Now, when one is due.
fn next_look(k: &Known, now: u64) -> u64 {
    let checked = k.seen.as_ref().map(|s| s.checked);
    match &k.failed {
        _ if !stale(checked, now) => next_turn(now),
        Some((t, _)) if *t <= now => (t + HOUR as u64).max(now),
        _ => now,
    }
}

/// The releases after `current` with their titles, and how many older ones
/// are left out, if every release listed has its title. A list missing one
/// would read as complete, so it is all or nothing.
fn titled(seen: &Seen, current: &str) -> (Vec<(String, String)>, usize) {
    let (after, older) = missed(&seen.releases, current);
    let listed: Option<Vec<(String, String)>> =
        after.into_iter().map(|v| seen.titles.get(&v).map(|t| (v, t.clone()))).collect();
    match listed {
        Some(listed) if !listed.is_empty() => (listed, older),
        _ => (Vec::new(), 0),
    }
}

pub struct Update {
    /// `--no-update-check`: nothing asked, nothing shown.
    pub off: bool,
    /// The running version, which decides which releases it is behind.
    current: String,
    /// Where the answer is kept. None when there is no config directory.
    file: Option<PathBuf>,
    /// The command that installs a newer one here, or why there is none.
    pub how: Result<How, Why>,
    known: Mutex<Known>,
}

impl Update {
    pub fn new(off: bool, file: Option<PathBuf>, how: Result<How, Why>) -> Update {
        Update { off, current: env!("CARGO_PKG_VERSION").to_string(), file, how, known: Mutex::default() }
    }

    /// The same, as if `version` were running.
    #[cfg(test)]
    fn running(mut self, version: &str) -> Update {
        self.current = version.to_string();
        self
    }

    pub fn status(&self, now: u64) -> Status {
        let Ok(k) = self.known.lock() else { return Status::default() };
        let (latest, checked) = k.seen.as_ref().map(|s| (s.latest.clone(), s.checked)).unwrap_or_default();
        let (failed, error) = match &k.failed {
            Some((t, e)) if *t >= checked => (*t, e.clone()),
            _ => (0, String::new()),
        };
        let next = if self.off || k.checking { 0 } else { next_look(&k, now) };
        let (releases, older) = match &k.seen {
            Some(seen) if newer(&self.current, &seen.latest) => titled(seen, &self.current),
            _ => (Vec::new(), 0),
        };
        Status { latest, checked, checking: k.checking, failed, error, next, releases, older }
    }

    /// When the next look is due, for the loop that makes it.
    pub fn next_look(&self, now: u64) -> u64 {
        self.known.lock().map(|k| next_look(&k, now)).unwrap_or(now + HOUR as u64)
    }

    /// The answer kept by any dbt-edith on the machine, as the banner prints
    /// it before the first look.
    pub fn kept(&self) -> Option<Seen> {
        self.file.as_deref().and_then(load)
    }

    /// Looks for a newer release unless the answer kept, here or by another
    /// dbt-edith in the file, still stands.
    pub async fn check(&self) {
        self.look(now_secs(), false, ask, titles).await
    }

    /// Looks now, whatever is kept: the menu's button. Not while a look is
    /// already on its way, whose answer this one would only repeat.
    pub async fn check_now(&self) {
        self.look(now_secs(), true, ask, titles).await
    }

    /// `ask` lists the releases; `titler` names the ones this dbt-edith is
    /// behind and no answer kept has a title for, and only when it is behind.
    /// The look is on its way until both have answered.
    async fn look<F, G>(&self, now: u64, now_anyway: bool, ask: F, titler: G)
    where
        F: FnOnce() -> Result<Vec<String>, String> + Send + 'static,
        G: FnOnce(Vec<String>) -> Result<Vec<(String, String)>, String> + Send + 'static,
    {
        if self.off {
            return;
        }
        let file = self.file.clone();
        let kept = tokio::task::spawn_blocking(move || file.as_deref().and_then(load)).await.ok().flatten();
        let asked = {
            let Ok(mut k) = self.known.lock() else { return };
            let seen = match (kept, k.seen.take()) {
                (Some(a), Some(b)) => Some(a.merged(b)),
                (a, b) => a.or(b),
            };
            let retry = k.failed.as_ref().map_or(true, |(t, _)| *t > now || now - t >= HOUR as u64);
            let due = !k.checking && (now_anyway || (stale(seen.as_ref().map(|s| s.checked), now) && retry));
            k.seen = seen;
            if due {
                k.checking = true;
            }
            due
        };
        if !asked {
            return;
        }
        let listed = tokio::task::spawn_blocking(ask).await.unwrap_or_else(|_| Err("the look stopped".to_string()));
        let answer = match listed {
            Ok(releases) if !releases.is_empty() => {
                let mut titles = self.known.lock().ok().and_then(|k| k.seen.as_ref().map(|s| s.titles.clone())).unwrap_or_default();
                let wanted: Vec<String> = missed(&releases, &self.current).0.into_iter().filter(|v| !titles.contains_key(v)).collect();
                if !wanted.is_empty() {
                    // Without them the menu links the newest release's notes
                    // alone, as before, and the next look tries again.
                    if let Ok(Ok(found)) = tokio::task::spawn_blocking(move || titler(wanted)).await {
                        titles.extend(found);
                    }
                }
                titles.retain(|v, _| releases.contains(v));
                Ok(Seen { checked: now, latest: releases[0].clone(), releases, titles })
            }
            Ok(_) => Err("GitHub listed no release".to_string()),
            Err(why) => Err(why),
        };
        if let Ok(seen) = &answer {
            if let Some(path) = self.file.clone() {
                let kept = seen.clone();
                let _ = tokio::task::spawn_blocking(move || save(&path, &kept)).await;
            }
        }
        if let Ok(mut k) = self.known.lock() {
            match answer {
                Ok(seen) => {
                    k.seen = Some(seen);
                    k.failed = None;
                }
                Err(why) => k.failed = Some((now, why)),
            }
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

/// Every release's notes, for the older ones the menu's list leaves out.
pub fn releases_url() -> String {
    format!("{REPO}/releases")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    /// The newest release in what `git ls-remote --tags --refs` printed.
    fn newest(listed: &str) -> Option<String> {
        releases(listed).into_iter().next()
    }

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

    /// Instants read off `date -u`, so the calendar is not checked against
    /// itself.
    const OCT_13_0700Z: u64 = 1_791_874_800;
    const OCT_13_0730Z: u64 = 1_791_876_600;
    const OCT_13_1200Z: u64 = 1_791_892_800;
    const OCT_13_2200Z: u64 = 1_791_928_800;
    const OCT_14_0300Z: u64 = 1_791_946_800;
    const OCT_14_0400Z: u64 = 1_791_950_400;
    const OCT_14_0600Z: u64 = 1_791_957_600;
    const OCT_14_0659Z: u64 = 1_791_961_140;
    const OCT_14_0700Z: u64 = 1_791_961_200;
    const OCT_14_2200Z: u64 = 1_792_015_200;
    const DEC_01_0700Z: u64 = 1_796_108_400;
    const DEC_01_0800Z: u64 = 1_796_112_000;
    const MAR_28_0000Z: u64 = 1_774_656_000;
    const MAR_29_0000Z: u64 = 1_774_742_400;
    const MAR_29_0100Z: u64 = 1_774_746_000;
    const OCT_25_0000Z: u64 = 1_792_886_400;
    const OCT_25_0100Z: u64 = 1_792_890_000;
    const OCT_25_0200Z: u64 = 1_792_893_600;
    const H: u64 = HOUR as u64;

    #[test]
    fn paris_moves_its_clocks_on_the_last_sundays_of_march_and_october() {
        assert_eq!(paris_offset(MAR_29_0100Z as i64 - 1), HOUR);
        assert_eq!(paris_offset(MAR_29_0100Z as i64), 2 * HOUR);
        assert_eq!(paris_offset(OCT_25_0100Z as i64 - 1), 2 * HOUR);
        assert_eq!(paris_offset(OCT_25_0100Z as i64), HOUR);
        assert_eq!(paris_offset(DEC_01_0700Z as i64), HOUR);
        assert_eq!(year_of(days_from_civil(2024, 2, 29)), 2024);
        assert_eq!(year_of(days_from_civil(2026, 1, 1) - 1), 2025);
    }

    #[test]
    fn a_turn_is_each_hour_from_paris_midnight_to_nine_in_summer_and_in_winter() {
        // 08:59 in Paris, summer time: 08:00 was one, 09:00 is a minute away.
        assert_eq!(last_turn(OCT_14_0659Z as i64), OCT_14_0600Z as i64);
        assert_eq!(next_turn(OCT_14_0659Z), OCT_14_0700Z);
        // From 09:00 the next is midnight.
        assert_eq!(last_turn(OCT_14_0700Z as i64), OCT_14_0700Z as i64);
        assert_eq!(next_turn(OCT_14_0700Z), OCT_14_2200Z);
        assert_eq!(last_turn(OCT_13_1200Z as i64), OCT_13_0700Z as i64);
        assert_eq!(next_turn(OCT_13_1200Z), OCT_13_2200Z);
        assert_eq!(last_turn(OCT_13_2200Z as i64), OCT_13_2200Z as i64);
        // 05:00 in Paris.
        assert_eq!(last_turn(OCT_14_0300Z as i64), OCT_14_0300Z as i64);
        assert_eq!(next_turn(OCT_14_0300Z + 1), OCT_14_0400Z);
        // 08:00 in Paris, winter time.
        assert_eq!(last_turn(DEC_01_0700Z as i64), DEC_01_0700Z as i64);
        assert_eq!(next_turn(DEC_01_0700Z), DEC_01_0800Z);
        // The night summer time starts, 02:00 is skipped, and the night it
        // ends, 02:00 comes twice: the first is the turn.
        assert_eq!(next_turn(MAR_29_0000Z), MAR_29_0100Z);
        assert_eq!(last_turn(MAR_29_0100Z as i64 + 1800), MAR_29_0100Z as i64);
        assert_eq!(last_turn(OCT_25_0100Z as i64 + 1800), OCT_25_0000Z as i64);
        assert_eq!(next_turn(OCT_25_0100Z + 1800), OCT_25_0200Z);
    }

    #[test]
    fn every_moment_lies_between_two_turns_at_most_fifteen_hours_apart() {
        for start in [MAR_28_0000Z, OCT_25_0000Z - 2 * DAY as u64, OCT_13_0700Z] {
            for step in 0..(3 * 96) {
                let now = start + step * 900;
                let (last, next) = (last_turn(now as i64), next_turn(now));
                assert!(last <= now as i64 && now < next, "{now}: {last} {next}");
                assert!(next - now <= 15 * H, "{now}: {next}");
            }
        }
    }

    #[test]
    fn an_answer_stands_until_the_next_turn_and_never_from_the_future() {
        assert!(stale(None, OCT_14_0659Z));
        // Taken yesterday at 09:30 in Paris, it no longer stands at 08:59,
        // where 0068's 24 hours kept it.
        assert!(stale(Some(OCT_13_0730Z), OCT_14_0659Z));
        // Taken at 05:00, it stands until 06:00.
        assert!(!stale(Some(OCT_14_0300Z), OCT_14_0400Z - 1));
        assert!(stale(Some(OCT_14_0300Z), OCT_14_0400Z));
        // Taken at 09:00, it stands the day through, until midnight.
        assert!(!stale(Some(OCT_14_0700Z), OCT_14_2200Z - 1));
        assert!(stale(Some(OCT_14_0700Z), OCT_14_2200Z));
        assert!(stale(Some(OCT_14_0659Z + 60), OCT_14_0659Z));
    }

    #[test]
    fn git_s_reason_reaches_the_menu_without_a_password_in_a_url() {
        assert_eq!(
            plain("fatal: unable to access 'https://ada:s3cret@github.com/IL-William/dbt-edith/': Could not resolve host"),
            "fatal: unable to access 'https://github.com/IL-William/dbt-edith/': Could not resolve host"
        );
        assert_eq!(plain("Failed to connect to proxy http://me:pw@proxy.example:8080 x"), "Failed to connect to proxy http://proxy.example:8080 x");
        assert_eq!(plain("cannot run git: No such file"), "cannot run git: No such file");
        assert_eq!(plain(&"x".repeat(300)).chars().count(), 241);
    }

    #[test]
    fn the_answer_is_kept_and_one_naming_no_version_is_not_believed() {
        let dir = scratch("seen");
        let path = seen_path(&dir);
        let seen = Seen { checked: 42, latest: "1.12.0".into(), ..Seen::default() };
        save(&path, &seen);
        assert_eq!(load(&path), Some(seen));
        std::fs::write(&path, r#"{"checked": 42, "latest": "../../x"}"#).unwrap();
        assert_eq!(load(&path), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// An asker that counts its calls and lists `latest` alone, or fails.
    fn asker(latest: Option<&str>) -> (Arc<AtomicUsize>, impl FnOnce() -> Result<Vec<String>, String> + Send + 'static) {
        let calls = Arc::new(AtomicUsize::new(0));
        let (count, latest) = (calls.clone(), latest.map(str::to_string));
        (calls, move || {
            count.fetch_add(1, Ordering::SeqCst);
            latest.map(|l| vec![l]).ok_or_else(|| "Could not resolve host: github.com".to_string())
        })
    }

    /// For a dbt-edith ahead of every release listed, which wants no title.
    fn never(_: Vec<String>) -> Result<Vec<(String, String)>, String> {
        panic!("a title was asked for with no release to list")
    }

    fn asked(calls: &Arc<AtomicUsize>) -> usize {
        calls.load(Ordering::SeqCst)
    }

    #[tokio::test]
    async fn one_look_per_turn_shared_through_the_file() {
        let dir = scratch("turns");
        let u = Update::new(false, Some(seen_path(&dir)), Err(Why::Elsewhere));

        let (calls, ask) = asker(Some("1.12.0"));
        u.look(OCT_13_0730Z, false, ask, never).await;
        assert_eq!(asked(&calls), 1);
        let after = Status { latest: "1.12.0".into(), checked: OCT_13_0730Z, next: OCT_13_2200Z, ..Status::default() };
        assert_eq!(u.status(OCT_13_0730Z), after);

        // Another start the same day reads it from the file and asks nothing.
        let other = Update::new(false, Some(seen_path(&dir)), Err(Why::Elsewhere));
        let (calls, ask) = asker(Some("9.9.9"));
        other.look(OCT_13_1200Z, false, ask, never).await;
        assert_eq!(asked(&calls), 0);
        assert_eq!(other.status(OCT_13_1200Z).latest, "1.12.0");

        // In the Paris night, the first start asks, and then every hour.
        let (calls, ask) = asker(Some("1.13.0"));
        other.look(OCT_14_0300Z, false, ask, never).await;
        assert_eq!(asked(&calls), 1);
        assert_eq!(other.status(OCT_14_0300Z).next, OCT_14_0400Z);
        let (calls, ask) = asker(Some("1.13.0"));
        other.look(OCT_14_0300Z + 1800, false, ask, never).await;
        assert_eq!(asked(&calls), 0);
        let (calls, ask) = asker(Some("1.14.0"));
        other.look(OCT_14_0400Z, false, ask, never).await;
        assert_eq!(asked(&calls), 1);

        // At 09:00 the last of the night, then nothing until midnight.
        let (calls, ask) = asker(Some("1.15.0"));
        other.look(OCT_14_0700Z, false, ask, never).await;
        assert_eq!(asked(&calls), 1);
        let (calls, ask) = asker(Some("9.9.9"));
        other.look(OCT_14_0700Z + 3 * H, false, ask, never).await;
        assert_eq!(asked(&calls), 0);
        let kept = Seen { checked: OCT_14_0700Z, latest: "1.15.0".into(), releases: vec!["1.15.0".into()], ..Seen::default() };
        assert_eq!(load(&seen_path(&dir)), Some(kept));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn a_look_with_no_answer_is_kept_nowhere_and_tried_again_an_hour_later() {
        let fresh = Update::new(false, None, Err(Why::Elsewhere));
        assert_eq!(fresh.status(OCT_14_0659Z).next, OCT_14_0659Z);
        let (calls, ask) = asker(None);
        fresh.look(OCT_14_0659Z, false, ask, never).await;
        assert_eq!(asked(&calls), 1);
        let failed = Status {
            failed: OCT_14_0659Z,
            error: "Could not resolve host: github.com".into(),
            next: OCT_14_0659Z + H,
            ..Status::default()
        };
        assert_eq!(fresh.status(OCT_14_0659Z), failed);

        // Not before the hour is out, a turn passed or not.
        let (calls, ask) = asker(Some("1.12.0"));
        fresh.look(OCT_14_0700Z, false, ask, never).await;
        assert_eq!(asked(&calls), 0);
        let (calls, ask) = asker(Some("1.12.0"));
        fresh.look(OCT_14_0659Z + H, false, ask, never).await;
        assert_eq!(asked(&calls), 1);
        let answered = Status { latest: "1.12.0".into(), checked: OCT_14_0659Z + H, next: OCT_14_2200Z, ..Status::default() };
        assert_eq!(fresh.status(OCT_14_0659Z + H), answered);

        // A failure after an answer is said beside it, which still stands.
        let (_, ask) = asker(None);
        fresh.look(OCT_14_2200Z, false, ask, never).await;
        let status = fresh.status(OCT_14_2200Z);
        assert_eq!((status.latest.as_str(), status.failed, status.next), ("1.12.0", OCT_14_2200Z, OCT_14_2200Z + H));
    }

    #[test]
    fn the_releases_are_every_version_tag_newest_first() {
        let listed = "a1\trefs/tags/v1.9.3\nb2\trefs/tags/v1.10.0\nc3\trefs/tags/v2.0.0-rc1\n\
                      d4\trefs/tags/collin-v9.0.0\ne5\trefs/tags/v1.2\nf6\trefs/tags/v1.10.0\ng7\trefs/tags/v1.9.10\n";
        assert_eq!(releases(listed), ["1.10.0", "1.9.10", "1.9.3"]);
    }

    #[test]
    fn the_list_holds_the_releases_after_the_running_one_twenty_at_most() {
        let all: Vec<String> = (0..30).rev().map(|n| format!("1.{n}.0")).collect();
        let (after, older) = missed(&all, "1.17.0");
        assert_eq!(after.first().map(String::as_str), Some("1.29.0"));
        assert_eq!(after.last().map(String::as_str), Some("1.18.0"));
        assert_eq!((after.len(), older), (12, 0));
        let (after, older) = missed(&all, "1.2.0");
        assert_eq!((after.len(), older), (20, 7));
        assert_eq!(after.last().map(String::as_str), Some("1.10.0"));
        assert_eq!(missed(&all, "1.29.0"), (Vec::new(), 0));
    }

    #[test]
    fn a_title_is_the_pull_request_s_without_its_number() {
        assert_eq!(title("Show each incremental strategy on the lineage (#39)\n"), "Show each incremental strategy on the lineage");
        assert_eq!(title("Bump the version"), "Bump the version");
        assert_eq!(title("Name the venv (it is #2)"), "Name the venv (it is #2)");
        assert_eq!(title("Read (#) as written"), "Read (#) as written");
        assert_eq!(title(&"x".repeat(250)).chars().count(), 201);
    }

    #[test]
    fn a_file_an_older_dbt_edith_wrote_still_reads_and_titles_merge() {
        let dir = scratch("old-file");
        let path = seen_path(&dir);
        std::fs::write(&path, r#"{"checked": 42, "latest": "1.17.0"}"#).unwrap();
        let old = load(&path).unwrap();
        assert_eq!((old.latest.as_str(), old.releases.len(), old.titles.len()), ("1.17.0", 0, 0));

        let titles = |pairs: &[(&str, &str)]| pairs.iter().map(|(v, t)| (v.to_string(), t.to_string())).collect();
        let earlier = Seen { checked: 1, latest: "1.14.0".into(), titles: titles(&[("1.14.0", "Colour the venv")]), ..Seen::default() };
        let later = Seen { checked: 2, latest: "1.15.0".into(), titles: titles(&[("1.15.0", "Sort the jobs")]), ..Seen::default() };
        let both = later.clone().merged(earlier.clone());
        assert_eq!((both.checked, both.titles.len()), (2, 2));
        assert_eq!(earlier.merged(later), both);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// An asker listing `releases`, newest first.
    fn lister(releases: &[&str]) -> impl FnOnce() -> Result<Vec<String>, String> + Send + 'static {
        let releases: Vec<String> = releases.iter().map(|v| v.to_string()).collect();
        move || Ok(releases)
    }

    /// A titler that records what it was asked for, and names each version,
    /// or fails.
    fn titler(
        works: bool,
    ) -> (Arc<Mutex<Vec<Vec<String>>>>, impl FnOnce(Vec<String>) -> Result<Vec<(String, String)>, String> + Send + 'static) {
        let asked = Arc::new(Mutex::new(Vec::new()));
        let record = asked.clone();
        (asked, move |versions: Vec<String>| {
            record.lock().unwrap().push(versions.clone());
            if !works {
                return Err("Could not resolve host: github.com".to_string());
            }
            Ok(versions.into_iter().map(|v| (v.clone(), format!("Release {v}"))).collect())
        })
    }

    #[tokio::test]
    async fn a_dbt_edith_behind_lists_each_release_since_and_fetches_a_title_once() {
        let dir = scratch("titles");
        let u = Update::new(false, Some(seen_path(&dir)), Err(Why::Elsewhere)).running("1.14.0");
        let (asked, titles) = titler(true);
        u.look(OCT_14_0300Z, false, lister(&["1.16.0", "1.15.0", "1.14.0", "1.13.0"]), titles).await;
        assert_eq!(*asked.lock().unwrap(), [["1.16.0", "1.15.0"]]);
        let status = u.status(OCT_14_0300Z);
        assert_eq!(status.latest, "1.16.0");
        let named = |pairs: &[&str]| pairs.iter().map(|v| (v.to_string(), format!("Release {v}"))).collect::<Vec<_>>();
        assert_eq!((status.releases, status.older), (named(&["1.16.0", "1.15.0"]), 0));

        // Another dbt-edith, as far behind, at the next turn: only the new
        // release's title is fetched, the others are in the file.
        let other = Update::new(false, Some(seen_path(&dir)), Err(Why::Elsewhere)).running("1.14.0");
        let (asked, titles) = titler(true);
        other.look(OCT_14_0400Z, false, lister(&["1.17.0", "1.16.0", "1.15.0", "1.14.0"]), titles).await;
        assert_eq!(*asked.lock().unwrap(), [["1.17.0"]]);
        assert_eq!(other.status(OCT_14_0400Z).releases, named(&["1.17.0", "1.16.0", "1.15.0"]));

        // One that runs the newest asks for none, and lists none.
        let newest = Update::new(false, Some(seen_path(&dir)), Err(Why::Elsewhere)).running("1.17.0");
        newest.look(OCT_14_0700Z, false, lister(&["1.17.0", "1.16.0"]), never).await;
        assert_eq!((newest.status(OCT_14_0700Z).releases, newest.status(OCT_14_0700Z).older), (Vec::new(), 0));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn without_every_title_there_is_no_list_and_the_next_look_tries_again() {
        let u = Update::new(false, None, Err(Why::Elsewhere)).running("1.14.0");
        let (asked, titles) = titler(false);
        u.look(OCT_14_0300Z, false, lister(&["1.16.0", "1.15.0", "1.14.0"]), titles).await;
        assert_eq!(asked.lock().unwrap().len(), 1);
        let status = u.status(OCT_14_0300Z);
        // The look itself answered: the newer release is known, and nothing
        // of the failed titles reaches the status bar.
        assert_eq!((status.latest.as_str(), status.failed, status.error.as_str()), ("1.16.0", 0, ""));
        assert!(status.releases.is_empty());

        let (asked, titles) = titler(true);
        u.look(OCT_14_0400Z, false, lister(&["1.16.0", "1.15.0", "1.14.0"]), titles).await;
        assert_eq!(*asked.lock().unwrap(), [["1.16.0", "1.15.0"]]);
        assert_eq!(u.status(OCT_14_0400Z).releases.len(), 2);
    }

    #[tokio::test]
    async fn twenty_releases_are_listed_and_the_older_ones_counted() {
        let u = Update::new(false, None, Err(Why::Elsewhere)).running("1.0.0");
        let all: Vec<String> = (1..=25).rev().map(|n| format!("1.{n}.0")).collect();
        let listed: Vec<&str> = all.iter().map(String::as_str).collect();
        let (asked, titles) = titler(true);
        u.look(OCT_14_0300Z, false, lister(&listed), titles).await;
        assert_eq!(asked.lock().unwrap()[0].len(), 20);
        let status = u.status(OCT_14_0300Z);
        assert_eq!((status.releases.len(), status.older), (20, 5));
        assert_eq!(status.releases[19].0, "1.6.0");
    }

    #[tokio::test]
    async fn the_button_asks_whatever_is_kept() {
        let dir = scratch("now");
        std::fs::write(seen_path(&dir), format!(r#"{{"checked": {OCT_14_0700Z}, "latest": "1.12.0"}}"#)).unwrap();
        let u = Update::new(false, Some(seen_path(&dir)), Err(Why::Elsewhere));
        let (calls, ask) = asker(Some("1.13.0"));
        u.look(OCT_14_0700Z + 60, false, ask, never).await;
        assert_eq!(asked(&calls), 0);
        let (calls, ask) = asker(Some("1.13.0"));
        u.look(OCT_14_0700Z + 60, true, ask, never).await;
        assert_eq!(asked(&calls), 1);
        assert_eq!(u.status(OCT_14_0700Z + 60).latest, "1.13.0");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn switched_off_it_asks_nothing_even_from_the_button() {
        let u = Update::new(true, None, Err(Why::Elsewhere));
        for now_anyway in [false, true] {
            let (calls, ask) = asker(Some("1.12.0"));
            u.look(OCT_14_0700Z, now_anyway, ask, never).await;
            assert_eq!(asked(&calls), 0);
        }
        assert_eq!(u.status(OCT_14_0700Z), Status::default());
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
        // git's own words when it cannot ask, for the menu to show.
        let missing = crate::git::ls_remote_tags(&inside, remote.join("missing").to_str().unwrap()).unwrap_err();
        assert!(missing.contains("missing"), "{missing}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The real git, on a local repository standing for GitHub, which ignores
    /// the filter GitHub honours: each tag's commit subject, in the order
    /// asked, a lightweight tag's and an annotated one's alike, and nothing
    /// left behind.
    #[test]
    fn git_fetches_the_subject_of_each_tagged_commit() {
        let dir = scratch("subjects");
        let remote = dir.join("remote");
        let git = |args: &[&str]| {
            let out = std::process::Command::new("git")
                .arg("-C")
                .arg(&remote)
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
        git(&["init", "-q"]);
        git(&["commit", "-q", "--allow-empty", "-m", "Pan the lineage from a box (#29)", "-m", "The body, never read."]);
        git(&["tag", "v1.13.2"]);
        git(&["commit", "-q", "--allow-empty", "-m", "Plan the next feature (#33)"]);
        git(&["commit", "-q", "--allow-empty", "-m", "Colour the venv in the status bar (#34)"]);
        git(&["tag", "-a", "v1.14.0", "-m", "Release 1.14.0, the tag's own message"]);
        let scratch_dir = dir.join("scratch");
        std::fs::create_dir_all(&scratch_dir).unwrap();

        let url = remote.to_str().unwrap();
        let tags = ["v1.14.0".to_string(), "v1.13.2".to_string()];
        let found = crate::git::tag_subjects(&scratch_dir, url, &tags).unwrap();
        let named: Vec<(String, String)> = found.into_iter().map(|(t, s)| (t, title(&s))).collect();
        assert_eq!(
            named,
            [("v1.14.0".to_string(), "Colour the venv in the status bar".to_string()), ("v1.13.2".into(), "Pan the lineage from a box".into())]
        );
        assert!(crate::git::tag_subjects(&scratch_dir, url, &["v9.9.9".to_string()]).is_err());
        assert_eq!(std::fs::read_dir(&scratch_dir).unwrap().count(), 0, "the bare repository is removed");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
