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
//!     pull and a rebuild, or, on Windows, for the `.exe` a release carries, which
//!     downloads the next one: offered first, beside the build where there is
//!     one (0123). Any other, a copy carried from another machine above
//!     all, is told what is out and why there is no command. Once another
//!     binary sits at the running one's path, the page says to restart
//!     rather than offer the command again (0095).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::collin_run::{is_version, mtime, semver};

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

/// Asks GitHub, from `scratch`: `ls-remote` on a URL needs no repository, and
/// none is looked for above it, so no project's git configuration has a say
/// in it. The user's own has, as in their pulls: a proxy the machine needs
/// above all. Every release, newest first, or why there is no answer.
pub fn ask(scratch: &Path) -> Result<Vec<String>, String> {
    let listed = crate::git::ls_remote_tags(scratch, REPO).map_err(|e| plain(&e))?;
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
pub fn titles(scratch: &Path, versions: Vec<String>) -> Result<Vec<(String, String)>, String> {
    let tags: Vec<String> = versions.iter().map(|v| format!("v{v}")).collect();
    let found = crate::git::tag_subjects(scratch, REPO, &tags).map_err(|e| plain(&e))?;
    Ok(found.into_iter().filter_map(|(tag, subject)| Some((tag.strip_prefix('v')?.to_string(), title(&subject)))).collect())
}

/// Where git runs for a look, given the file the answer is kept in: its
/// directory, the config one, which is the user's own, rather than the
/// temporary one, which on a shared machine is every account's. There,
/// another account could put a directory or a link where the titles'
/// repository is made, under a name the pid makes predictable, or a `.git` for
/// `ls-remote` to find. Without a config directory, or when it cannot be made,
/// the temporary one serves, and the repository is then made only where
/// nothing was before (`git::tag_subjects`). Called inside the look, once it
/// is due, so a check with nothing to do makes nothing.
fn look_dir(file: Option<&Path>) -> PathBuf {
    if let Some(dir) = file.and_then(Path::parent) {
        let _ = std::fs::create_dir_all(dir);
        if dir.is_dir() {
            return dir.to_path_buf();
        }
    }
    std::env::temp_dir()
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
/// stands. Now, when one is due. A minute from now while a look is on its
/// way, whose answer decides.
fn next_look(k: &Known, now: u64) -> u64 {
    let checked = k.seen.as_ref().map(|s| s.checked);
    match &k.failed {
        _ if k.checking => now + 60,
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
    /// The commands that install a newer one here, the recommended first, or
    /// why there is none (0123). Private: a clone's has no command until
    /// `hows_for` composes it for a release (0129).
    hows: Result<Vec<How>, Why>,
    known: Mutex<Known>,
    spread: u64,
    /// The running binary's path, and when its file was last written as this
    /// process started, for telling that another has been put in its place.
    binary: Option<(PathBuf, std::time::SystemTime)>,
}

/// Seconds from 0 to 119, different from one process to the next: the
/// standard library's hasher is seeded at random for each.
fn spread() -> u64 {
    use std::hash::{BuildHasher, Hasher};
    std::collections::hash_map::RandomState::new().build_hasher().finish() % 120
}

/// Puts `checking` back when a look is dropped on its way: a request the
/// browser gave up on drops its future, and the flag would stay set for the
/// life of the process, every later look refused and the menu looking forever.
struct OnItsWay<'a>(Option<&'a Mutex<Known>>);

impl Drop for OnItsWay<'_> {
    fn drop(&mut self) {
        if let Some(Ok(mut k)) = self.0.map(Mutex::lock) {
            k.checking = false;
        }
    }
}

impl Update {
    pub fn new(off: bool, file: Option<PathBuf>, hows: Result<Vec<How>, Why>) -> Update {
        Update {
            off,
            current: env!("CARGO_PKG_VERSION").to_string(),
            file,
            hows,
            known: Mutex::default(),
            spread: spread(),
            binary: None,
        }
    }

    /// Watches the file `exe` names, the running binary, from now on.
    pub fn watching(mut self, exe: Option<PathBuf>) -> Update {
        self.binary = exe.and_then(|exe| mtime(&exe).map(|at| (exe, at)));
        self
    }

    /// Whether another binary sits at the running one's path since it
    /// started: an update installed, which only a restart runs (0095). Its
    /// file's time is all that is read, never what is inside, as for telling
    /// how it was installed (0068). A path left empty, while the update has
    /// renamed the running `.exe` on Windows and cargo is still building, is
    /// not one yet.
    pub fn replaced(&self) -> bool {
        self.binary.as_ref().is_some_and(|(exe, at)| mtime(exe).is_some_and(|now| now != *at))
    }

    /// The commands that install release `latest` here, or why there are none.
    /// A clone's command names the tag it brings the clone up to, so it is
    /// composed for each answer; one whose release is not a version number is
    /// left out (0129), and the menu says why when that leaves nothing.
    pub fn hows_for(&self, latest: &str) -> Result<Vec<How>, Why> {
        let hows = self.hows.as_ref().map_err(|why| *why)?;
        let composed: Vec<How> = hows.iter().filter_map(|h| h.for_release(latest)).collect();
        if composed.is_empty() { Err(Why::Release) } else { Ok(composed) }
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

    /// How long this process waits past a turn before it looks: up to two
    /// minutes, drawn once. Every dbt-edith on the machine wakes at the same
    /// turn, and the one that asks first keeps its answer in the file, which
    /// the others read rather than ask GitHub again.
    pub fn spread(&self) -> u64 {
        self.spread
    }

    /// The answer kept by any dbt-edith on the machine, as the banner prints
    /// it before the first look.
    pub fn kept(&self) -> Option<Seen> {
        self.file.as_deref().and_then(load)
    }

    /// Looks for a newer release unless the answer kept, here or by another
    /// dbt-edith in the file, still stands.
    pub async fn check(&self) {
        let (asked, titled) = (self.file.clone(), self.file.clone());
        self.look(now_secs(), false, move || ask(&look_dir(asked.as_deref())), move |v| titles(&look_dir(titled.as_deref()), v))
            .await
    }

    /// Looks now, whatever is kept: the menu's button. Not while a look is
    /// already on its way, whose answer this one would only repeat.
    pub async fn check_now(&self) {
        let (asked, titled) = (self.file.clone(), self.file.clone());
        self.look(now_secs(), true, move || ask(&look_dir(asked.as_deref())), move |v| titles(&look_dir(titled.as_deref()), v))
            .await
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
        let mut on_its_way = OnItsWay(Some(&self.known));
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
        on_its_way.0 = None;
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
    /// `cargo build --release` left in the clone, `download` for the release's
    /// own `.exe` (0123).
    pub kind: &'static str,
    /// The clone it pulls, or for `download` the folder it downloads from.
    pub source: String,
    /// The running binary's file name.
    pub exe: String,
    /// Whether the running binary is renamed out of the way first: Windows
    /// will not replace an executable that runs, but lets it be renamed.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub renames: bool,
    /// What a command that pulls the clone is made of, kept to compose it
    /// again for each release the menu announces (0129). None for a download,
    /// whose command names no version.
    #[serde(skip)]
    pub plan: Option<Plan>,
}

/// The parts of a clone's command that do not change with the release.
#[derive(Clone, Debug, PartialEq)]
pub struct Plan {
    shell: Shell,
    source: String,
    step: String,
    clean: Option<String>,
    swap: Option<String>,
}

impl How {
    /// The command that brings the clone up to `version`, the release the menu
    /// announces, and installs it. A `version` that is not three runs of
    /// digits gets none, since it comes off the network and goes into a typed
    /// line (0129). A download is the same for every release and comes back
    /// as it is.
    pub fn for_release(&self, version: &str) -> Option<How> {
        let Some(plan) = &self.plan else { return Some(self.clone()) };
        if !is_version(version) {
            return None;
        }
        let command =
            compose(plan.shell, &plan.source, version, &plan.step, plan.clean.as_deref(), plan.swap.as_deref());
        Some(How { command, ..self.clone() })
    }
}

/// Why there is no command to offer, for the page to say.
#[derive(serde::Serialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Why {
    /// Neither installed by cargo nor built in a clone still on this machine.
    Elsewhere,
    /// Built in its clone, on Windows, or anywhere else under the clone's
    /// `target/`, a developer's build a download would replace (0123). cargo
    /// links the binary in `target/release` from the one it writes in `deps/`,
    /// the same file under another name, so renaming the running one frees its
    /// name but not that file, which the build must overwrite.
    Locked,
    /// The terminal's shell is none the command is written for.
    Shell,
    /// A path holds a character a shell would read inside the quotes.
    Path,
    /// The release the menu announces is not a version number, so there is no
    /// tag to bring the clone up to (0129).
    Release,
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
    // Only an install: a binary built in the clone runs from its `target/`,
    // which the clean would delete. `--release` because an install builds
    // nothing else: the clone's `target/debug` and test binaries, which
    // someone developing in it uses (and an editor may hold open on Windows),
    // are left alone. `--target-dir` names the clone's own folder, so a
    // `CARGO_TARGET_DIR` or `build.target-dir` shared with other projects is
    // never the one emptied; when cargo built elsewhere, this finds nothing to
    // remove and succeeds (0120).
    let clean = if installed {
        Some(format!(
            "cargo clean --release --manifest-path \"{}\" --target-dir \"{}\"",
            spell(&source.join("Cargo.toml"))?,
            spell(&source.join("target"))?
        ))
    } else {
        None
    };
    let swap = if windows { Some(spell(exe)?) } else { None };
    // No command until a release is announced: the clone is brought up to
    // that tag, so `Update::hows_for` composes it from the plan (0129).
    Ok(How {
        command: String::new(),
        kind,
        source: src.clone(),
        exe: exe.file_name().ok_or(Why::Elsewhere)?.to_string_lossy().into_owned(),
        renames: windows,
        plan: Some(Plan { shell, source: src, step, clean, swap }),
    })
}

/// Every command that can update this binary here, the recommended first, or
/// why there is none (0123). Off Windows that is the one `how` gives. On
/// Windows a download of the release's `.exe` comes first, and the build or the
/// install `how` gives follows it for a binary cargo installed, so a machine
/// that can compile still can. A binary anywhere under the clone's `target/`,
/// `release`, `debug` or a target triple's, is a developer's build, which a
/// download would replace: no command. A list that comes out empty carries the
/// first refusal, the download's.
pub fn hows(source: &Path, exe: &Path, shell: Option<Shell>, windows: bool) -> Result<Vec<How>, Why> {
    let cargo = how(source, exe, shell, windows);
    if windows && exe.starts_with(source.join("target")) {
        return Err(Why::Locked);
    }
    if !windows {
        return cargo.map(|c| vec![c]);
    }
    match (download(exe, shell, windows), cargo) {
        (Ok(d), Ok(c)) => Ok(vec![d, c]),
        (Ok(d), Err(_)) => Ok(vec![d]),
        (Err(_), Ok(c)) => Ok(vec![c]),
        (Err(why), Err(_)) => Err(why),
    }
}

/// The commands for a binary whose clone may be gone: `source` is None when
/// the folder it was built from cannot be found, a copy cross-built elsewhere
/// or an install whose clone was moved. On Windows the download needs no clone,
/// so it is still offered (0123); anywhere else there is nothing to pull.
pub fn hows_from(source: Option<&Path>, exe: &Path, shell: Option<Shell>, windows: bool) -> Result<Vec<How>, Why> {
    match source {
        Some(source) => hows(source, exe, shell, windows),
        None if windows => download(exe, shell, windows).map(|d| vec![d]),
        None => Err(Why::Elsewhere),
    }
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

/// Bring the clone up to the release, then rebuild, each only once the step
/// before it worked. `tag` is the version the menu announces, which
/// `How::for_release` has checked is three runs of digits. The merge
/// fast-forwards whatever the clone has out, a branch or a detached HEAD, to
/// that tag and no further, so a commit merged after the release is not built
/// under its number (0129). A clone already at the tag or past it is "Already
/// up to date" and builds what it holds; one that cannot fast-forward to it
/// stops the command there, as a pull does. Release tags are lightweight, so
/// the merge names a commit.
/// The tags come first, in a step of their own whose failure stops nothing:
/// the merge needs the tag, and the build stamp is `git describe` in the
/// clone, which would name the last release the clone had heard of rather than
/// the one it builds. Forced, since a clone may hold a release tag the
/// repository has since moved, which `pull --tags` refuses to clobber, failing
/// the whole pull (0095). Both shells read `;` alike.
/// With `swap`, the running binary is renamed to `<exe>.old` before the
/// rebuild and put back when the rebuild fails, so a failed update still
/// leaves one to start.
/// The last thing it prints is `DONE`, and only once the install worked: the
/// restore after a failed build must itself end in a failure, or the shell
/// would read the put-back as success and claim an install that never was
/// (0111). The text holds no quote, `$` or backtick, since it sits inside a
/// string in either shell.
/// With `clean`, the clone's build folder is emptied once the install worked
/// (0120), since cargo never removes what an earlier build left in it. It sits
/// after the restore group, never inside it, and its failure changes nothing:
/// the install worked, the binary stays, and the closing line is still
/// printed. A Posix brace group ends its last command on `;`, and PowerShell
/// goes on after a failed native command by itself.
fn compose(shell: Shell, source: &str, tag: &str, step: &str, clean: Option<&str>, swap: Option<&str>) -> String {
    let bring =
        format!("git -C \"{source}\" fetch --tags --force origin; git -C \"{source}\" merge --ff-only v{tag}");
    let printed = format!("printf '\\n%s\\n' '{DONE}'");
    let closing = match clean {
        None => printed,
        Some(clean) => format!("{{ {clean}; {printed}; }}"),
    };
    let cleaned = clean.map(|clean| format!("{clean}; ")).unwrap_or_default();
    match (shell, swap) {
        (Shell::Posix, None) => format!("{bring} && {step} && {closing}"),
        (Shell::Posix, Some(exe)) => {
            format!(
                "{bring} && mv -f \"{exe}\" \"{exe}.old\" && ({step} || {{ mv -f \"{exe}.old\" \"{exe}\"; false; }}) \
                 && {closing}"
            )
        }
        (Shell::PowerShell, None) => {
            format!("{bring}; if ($?) {{ {step}; if ($?) {{ {cleaned}Write-Host \"\"; Write-Host \"{DONE}\" }} }}")
        }
        (Shell::PowerShell, Some(exe)) => format!(
            "{bring}; if ($?) {{ Move-Item -Force -LiteralPath \"{exe}\" -Destination \"{exe}.old\"; if ($?) {{ {step}; \
             if ($?) {{ {cleaned}Write-Host \"\"; Write-Host \"{DONE}\" }} else {{ Move-Item -Force -LiteralPath \"{exe}.old\" -Destination \"{exe}\" }} }} }}"
        ),
    }
}

/// What the command ends on when the install worked (0111).
const DONE: &str = "dbt-edith is installed. Stop dbt-edith where you started it (Ctrl+C) and start it again to run it.";

/// The file a release carries for the architecture this binary was built for
/// (0123): the x64 one, which runs on Windows on ARM too, or the native ARM64
/// one. An update keeps to the architecture it has, so an ARM64 build never
/// swaps itself for the emulated one.
fn asset(arm64: bool) -> &'static str {
    if arm64 { "dbt-edith-arm64.exe" } else { "dbt-edith.exe" }
}

/// The command that replaces the running `.exe` of a release with the next
/// one, downloaded from the latest release and checked against its SHA-256
/// first (0123). `latest` names no version, so the command is the same all the
/// time and `How` is made once, at start. Only a Windows build is ever the
/// release's own, so anywhere else there is no command.
pub fn download(exe: &Path, shell: Option<Shell>, windows: bool) -> Result<How, Why> {
    download_of(asset(cfg!(target_arch = "aarch64")), exe, shell, windows)
}

fn download_of(asset: &str, exe: &Path, shell: Option<Shell>, windows: bool) -> Result<How, Why> {
    if !windows {
        return Err(Why::Elsewhere);
    }
    let shell = shell.ok_or(Why::Shell)?;
    let base = format!("{REPO}/releases/latest/download");
    let path = quotable(exe, windows).ok_or(Why::Path)?;
    Ok(How {
        command: compose_download(shell, &base, asset, &path),
        kind: "download",
        source: base,
        exe: exe.file_name().ok_or(Why::Elsewhere)?.to_string_lossy().into_owned(),
        renames: true,
        plan: None,
    })
}

/// Download `<base>/<asset>` and its hash beside `exe`, whatever `exe` is
/// called, compare them, and only then swap: the running file steps aside as
/// `<exe>.old`, the new one takes its name, and the old one comes back if that
/// fails. A failure anywhere leaves `exe` as it was, removes what was
/// downloaded and prints no `DONE`, so a failed update is a failure (0111).
/// `base` is a parameter for the tests, which read a folder through `file://`
/// and a server on this machine; curl is held to the base's scheme, and its
/// redirects to HTTPS, which is what guards the file in transit (the hash comes
/// from the same release, so it catches a truncated download, not a tampered
/// one). The same rules as `compose` hold for the text: no quote, `$` or
/// backtick in what is printed.
fn compose_download(shell: Shell, base: &str, asset: &str, exe: &str) -> String {
    let (new, sum) = (format!("{exe}.new"), format!("{exe}.sha256"));
    let (url, sum_url) = (format!("{base}/{asset}"), format!("{base}/{asset}.sha256"));
    // Quoted: zsh reads an unquoted `=https` as a command to look up.
    let proto = format!("--proto '={}' --proto-redir '=https'", base.split("://").next().unwrap_or("https"));
    match shell {
        Shell::Posix => format!(
            "(curl -fsSL {proto} -o \"{new}\" \"{url}\" && curl -fsSL {proto} -o \"{sum}\" \"{sum_url}\" && \
             {{ [ \"$(sha256sum \"{new}\" | cut -d' ' -f1)\" = \"$(cut -d' ' -f1 \"{sum}\")\" ] \
             || {{ printf '%s\\n' '{MISMATCH}'; false; }}; }} && rm -f \"{sum}\" \
             || {{ rm -f \"{new}\" \"{sum}\"; false; }}) \
             && {{ mv -f \"{exe}\" \"{exe}.old\" && (mv -f \"{new}\" \"{exe}\" || {{ mv -f \"{exe}.old\" \"{exe}\"; false; }}) \
             || {{ rm -f \"{new}\"; false; }}; }} \
             && printf '\\n%s\\n' '{DONE}'"
        ),
        // 5.1, which Windows ships, has no `&&`; it also offers TLS 1.0 first
        // and shows a progress bar that slows the download to a crawl. In a
        // script block, so neither preference leaks into the user's session,
        // and a profile's `$ErrorActionPreference='Stop'` cannot make a failed
        // move throw past the put-back; each call names its `-ErrorAction`,
        // which a profile's `$PSDefaultParameterValues` does not override. TLS
        // 1.2 is added to what is on, not set, since that is process-wide; on
        // the system default (0) it is left alone, which already allows it,
        // where `0 -bor Tls12` would be Tls12 alone.
        Shell::PowerShell => format!(
            "& {{ $ErrorActionPreference='Continue'; $ProgressPreference='SilentlyContinue'; \
             if ([Net.ServicePointManager]::SecurityProtocol -ne 0) {{ [Net.ServicePointManager]::SecurityProtocol = \
             [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12 }}; \
             Invoke-WebRequest -ErrorAction Continue -UseBasicParsing -Uri \"{url}\" -OutFile \"{new}\"; if ($?) {{ \
             Invoke-WebRequest -ErrorAction Continue -UseBasicParsing -Uri \"{sum_url}\" -OutFile \"{sum}\"; if ($?) {{ \
             if ((Get-FileHash -ErrorAction Continue -Algorithm SHA256 -LiteralPath \"{new}\").Hash -eq ((Get-Content -ErrorAction Continue -LiteralPath \"{sum}\") -split ' ')[0]) {{ \
             Move-Item -ErrorAction Continue -Force -LiteralPath \"{exe}\" -Destination \"{exe}.old\"; if ($?) {{ \
             Move-Item -ErrorAction Continue -Force -LiteralPath \"{new}\" -Destination \"{exe}\"; \
             if ($?) {{ Write-Host \"\"; Write-Host \"{DONE}\" }} else {{ Move-Item -ErrorAction Continue -Force -LiteralPath \"{exe}.old\" -Destination \"{exe}\" }} }} \
             }} else {{ Write-Host \"{MISMATCH}\" }} }} }}; \
             Remove-Item -Force -ErrorAction SilentlyContinue -LiteralPath \"{new}\", \"{sum}\" }}"
        ),
    }
}

/// What the download prints when the hash is not the published one (0123).
const MISMATCH: &str = "The download does not match its SHA-256, so nothing was replaced.";

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

    /// The look runs from the config directory, made on the way, and from the
    /// temporary one when there is none or it cannot be made, as 1.25.0 did
    /// everywhere: a locked-down home must not stop the look.
    #[test]
    fn the_look_runs_from_the_config_directory_or_the_temporary_one() {
        let dir = scratch("lookdir");
        let file = dir.join("config").join("update.json");
        assert_eq!(look_dir(Some(&file)), dir.join("config"));
        assert!(dir.join("config").is_dir(), "made on the way");
        std::fs::write(dir.join("blocker"), "").unwrap();
        let blocked = dir.join("blocker").join("config").join("update.json");
        assert_eq!(look_dir(Some(&blocked)), std::env::temp_dir());
        assert_eq!(look_dir(None), std::env::temp_dir());
        let _ = std::fs::remove_dir_all(&dir);
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

    #[tokio::test]
    async fn a_look_dropped_on_its_way_lets_the_next_one_ask() {
        let u = Update::new(false, None, Err(Why::Elsewhere));
        // git still waiting on a proxy when the page that asked is closed.
        let slow = || {
            std::thread::sleep(std::time::Duration::from_millis(300));
            Ok(vec!["1.12.0".to_string()])
        };
        let dropped = tokio::time::timeout(std::time::Duration::from_millis(20), u.look(OCT_14_0659Z, true, slow, never)).await;
        assert!(dropped.is_err());
        assert!(!u.status(OCT_14_0659Z).checking);
        let (calls, ask) = asker(Some("1.12.0"));
        u.look(OCT_14_0659Z, true, ask, never).await;
        assert_eq!(asked(&calls), 1);
        assert_eq!(u.status(OCT_14_0659Z).latest, "1.12.0");
    }

    #[test]
    fn while_a_look_is_on_its_way_the_loop_waits_a_minute_and_each_process_its_spread() {
        let looking = Known { checking: true, ..Known::default() };
        assert_eq!(next_look(&looking, OCT_14_0659Z), OCT_14_0659Z + 60);
        assert!((0..20).all(|_| Update::new(false, None, Err(Why::Elsewhere)).spread() < 120));
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
        let how = how.for_release("1.2.0").unwrap();
        let s = source.display();
        assert_eq!(
            how.command,
            format!(
                "git -C \"{s}\" fetch --tags --force origin; git -C \"{s}\" merge --ff-only v1.2.0 && \
                 cargo install --locked --path \"{s}\" && {{ cargo clean --release --manifest-path \"{s}/Cargo.toml\" \
                 --target-dir \"{s}/target\"; printf '\\n%s\\n' '{DONE}'; }}"
            )
        );
        assert_eq!((how.kind, how.renames, how.exe.as_str()), ("install", false, "dbt-edith"));
        let _ = std::fs::remove_dir_all(source.parent().unwrap());
    }

    /// The clean is built by `how()` itself on Windows: in the success branch
    /// of the install and not in the one that puts the binary back (0120).
    #[test]
    fn on_windows_the_clean_is_spelled_by_how_and_runs_after_the_install_only() {
        let (source, root) = layout("winclean");
        let exe = root.join("bin").join("dbt-edith");
        let how = how(&source, &exe, Some(Shell::PowerShell), true).unwrap().for_release("1.2.0").unwrap();
        let s = quotable(&source, true).unwrap();
        let clean = format!("cargo clean --release --manifest-path \"{s}/Cargo.toml\" --target-dir \"{s}/target\"");
        let (ok, failed) = how.command.split_once(" else ").unwrap();
        assert!(ok.ends_with(&format!("{clean}; Write-Host \"\"; Write-Host \"{DONE}\" }}")), "{}", how.command);
        assert!(failed.starts_with("{ Move-Item") && !failed.contains("cargo clean"), "{}", how.command);
        assert_eq!(how.command.matches("cargo clean").count(), 1, "{}", how.command);
        let _ = std::fs::remove_dir_all(source.parent().unwrap());
    }

    #[test]
    fn a_binary_built_in_the_clone_is_built_there_again() {
        let (source, _) = layout("built");
        let exe = source.join("target").join("release").join("dbt-edith");
        let built = how(&source, &exe, Some(Shell::Posix), false).unwrap().for_release("1.2.0").unwrap();
        assert_eq!(built.kind, "build");
        assert!(built.command.contains(&format!("cargo build --release --locked --manifest-path \"{}\"", source.join("Cargo.toml").display())));
        assert!(built.command.ends_with(&format!("&& printf '\\n%s\\n' '{DONE}'")));
        // The binary just built sits in the folder a clean would empty.
        assert!(!built.command.contains("cargo clean"), "{}", built.command);
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
            assert_eq!(download(&odd.join("dbt-edith.exe"), Some(Shell::Posix), true), Err(Why::Path), "{name}");
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
            compose(Shell::Posix, "C:/src/dbt-edith", "1.2.0", step, None, Some(exe)),
            format!(
                "git -C \"C:/src/dbt-edith\" fetch --tags --force origin; \
                 git -C \"C:/src/dbt-edith\" merge --ff-only v1.2.0 && mv -f \"{exe}\" \"{exe}.old\" && \
                 ({step} || {{ mv -f \"{exe}.old\" \"{exe}\"; false; }}) && printf '\\n%s\\n' '{DONE}'"
            )
        );
        assert_eq!(
            compose(Shell::PowerShell, "C:/src/dbt-edith", "1.2.0", step, None, Some(exe)),
            format!(
                "git -C \"C:/src/dbt-edith\" fetch --tags --force origin; \
                 git -C \"C:/src/dbt-edith\" merge --ff-only v1.2.0; if ($?) {{ \
                 Move-Item -Force -LiteralPath \"{exe}\" -Destination \"{exe}.old\"; if ($?) {{ {step}; \
                 if ($?) {{ Write-Host \"\"; Write-Host \"{DONE}\" }} else {{ Move-Item -Force -LiteralPath \"{exe}.old\" -Destination \"{exe}\" }} }} }}"
            )
        );
        assert_eq!(quotable(Path::new(r"C:\Users\ada\.cargo\bin\dbt-edith.exe"), true).as_deref(), Some(exe));
        // With the clean, it follows the install in the success branch alone,
        // outside the group that puts the binary back (0120).
        let clean = "cargo clean --release --manifest-path \"C:/src/dbt-edith/Cargo.toml\" --target-dir \"C:/src/dbt-edith/target\"";
        assert_eq!(
            compose(Shell::Posix, "C:/src/dbt-edith", "1.2.0", step, Some(clean), Some(exe)),
            format!(
                "git -C \"C:/src/dbt-edith\" fetch --tags --force origin; \
                 git -C \"C:/src/dbt-edith\" merge --ff-only v1.2.0 && mv -f \"{exe}\" \"{exe}.old\" && \
                 ({step} || {{ mv -f \"{exe}.old\" \"{exe}\"; false; }}) && \
                 {{ {clean}; printf '\\n%s\\n' '{DONE}'; }}"
            )
        );
        assert_eq!(
            compose(Shell::PowerShell, "C:/src/dbt-edith", "1.2.0", step, Some(clean), Some(exe)),
            format!(
                "git -C \"C:/src/dbt-edith\" fetch --tags --force origin; \
                 git -C \"C:/src/dbt-edith\" merge --ff-only v1.2.0; if ($?) {{ \
                 Move-Item -Force -LiteralPath \"{exe}\" -Destination \"{exe}.old\"; if ($?) {{ {step}; \
                 if ($?) {{ {clean}; Write-Host \"\"; Write-Host \"{DONE}\" }} else {{ Move-Item -Force -LiteralPath \"{exe}.old\" -Destination \"{exe}\" }} }} }}"
            )
        );
    }

    #[test]
    fn a_release_exe_on_windows_downloads_the_next_one() {
        let exe = Path::new("C:/Users/ada/bin/dbt-edith.exe");
        let base = "https://github.com/IL-William/dbt-edith/releases/latest/download";
        let (url, sum_url) = (format!("{base}/dbt-edith.exe"), format!("{base}/dbt-edith.exe.sha256"));
        let e = "C:/Users/ada/bin/dbt-edith.exe";

        let sh = download_of("dbt-edith.exe", exe, Some(Shell::Posix), true).unwrap();
        assert_eq!((sh.kind, sh.renames, sh.exe.as_str(), sh.source.as_str()), ("download", true, "dbt-edith.exe", base));
        let c = &sh.command;
        // curl may only speak and be redirected to HTTPS, what guards the file in transit.
        let proto = "--proto '=https' --proto-redir '=https'";
        assert!(c.starts_with(&format!("(curl -fsSL {proto} -o \"{e}.new\" \"{url}\" && curl -fsSL {proto} -o \"{e}.sha256\" \"{sum_url}\"")), "{c}");
        assert!(c.contains(&format!("sha256sum \"{e}.new\"")) && c.contains(&format!("cut -d' ' -f1 \"{e}.sha256\"")), "the hash is checked: {c}");
        // Nothing is swapped before the check, and a failure removes what came down.
        let (check, swap) = c.split_once(&format!("&& {{ mv -f \"{e}\" \"{e}.old\"")).unwrap();
        assert!(check.contains(&format!("{{ rm -f \"{e}.new\" \"{e}.sha256\"; false; }}")), "{c}");
        assert!(check.contains(MISMATCH), "{c}");
        // The new file takes the name, the old one comes back if that fails,
        // and that ends in a failure (0111). Whatever fails after the download
        // removes it.
        assert!(swap.starts_with(&format!(" && (mv -f \"{e}.new\" \"{e}\" || {{ mv -f \"{e}.old\" \"{e}\"; false; }})")), "{c}");
        assert!(swap.contains(&format!("|| {{ rm -f \"{e}.new\"; false; }}; }} && printf")), "{c}");
        assert!(c.ends_with(&format!("&& printf '\\n%s\\n' '{DONE}'")), "DONE last: {c}");

        let ps = download_of("dbt-edith.exe", exe, Some(Shell::PowerShell), true).unwrap();
        assert_eq!((ps.kind, ps.renames), ("download", true));
        let c = &ps.command;
        // In a script block, so a profile's `Stop` cannot throw past the put-back
        // and nothing set here outlives the command; each call names its own
        // `-ErrorAction`; TLS 1.2 is added only when a protocol is set, never over the system default (0).
        assert!(c.starts_with("& { $ErrorActionPreference='Continue'; $ProgressPreference='SilentlyContinue'; "), "{c}");
        assert!(
            c.contains(
                "if ([Net.ServicePointManager]::SecurityProtocol -ne 0) { [Net.ServicePointManager]::SecurityProtocol = \
                 [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12 }; "
            ),
            "{c}"
        );
        let act = "-ErrorAction Continue";
        assert!(c.contains(&format!("Invoke-WebRequest {act} -UseBasicParsing -Uri \"{url}\" -OutFile \"{e}.new\"")), "{c}");
        assert!(c.contains(&format!("Invoke-WebRequest {act} -UseBasicParsing -Uri \"{sum_url}\" -OutFile \"{e}.sha256\"")), "{c}");
        // Every move names it too, the put-back included, and so do the hash and
        // the file it is checked against.
        assert_eq!(c.matches("Move-Item -ErrorAction Continue -Force").count(), 3, "{c}");
        assert_eq!(c.matches("Move-Item").count(), 3, "{c}");
        assert!(
            c.contains(&format!(
                "(Get-FileHash -ErrorAction Continue -Algorithm SHA256 -LiteralPath \"{e}.new\").Hash -eq ((Get-Content -ErrorAction Continue -LiteralPath \"{e}.sha256\") -split ' ')[0]"
            )),
            "{c}"
        );
        let (swapped, failed) = c.split_once(" else { Move-Item").unwrap();
        assert!(swapped.contains(&format!("Move-Item -ErrorAction Continue -Force -LiteralPath \"{e}\" -Destination \"{e}.old\"")), "{c}");
        assert!(swapped.ends_with(&format!("if ($?) {{ Write-Host \"\"; Write-Host \"{DONE}\" }}")), "DONE after the swap: {c}");
        // The put-back prints nothing, and a hash that differs says so.
        assert!(failed.trim_start().starts_with(&format!("-ErrorAction Continue -Force -LiteralPath \"{e}.old\" -Destination \"{e}\" }}")), "{c}");
        assert!(c.contains(&format!("else {{ Write-Host \"{MISMATCH}\" }}")), "{c}");
        assert!(c.ends_with(&format!("Remove-Item -Force -ErrorAction SilentlyContinue -LiteralPath \"{e}.new\", \"{e}.sha256\" }}")), "{c}");
        assert!(!MISMATCH.contains(['\'', '"', '$', '`', '\\']), "it sits inside a string in either shell");
    }

    /// The choice (0123): on Windows the download is offered first, and the
    /// cargo command beside it for a binary cargo installed; a developer's
    /// build in the clone is offered neither.
    #[test]
    fn on_windows_the_download_comes_first_and_cargo_follows_for_a_binary_it_installed() {
        let kinds = |r: Result<Vec<How>, Why>| r.map(|h| h.iter().map(|h| h.kind).collect::<Vec<_>>());
        let (source, root) = layout("choice");
        let installed = root.join("bin").join("dbt-edith");
        let posix = Some(Shell::Posix);
        assert_eq!(kinds(hows(&source, &installed, posix, true)), Ok(vec!["download", "install"]));
        // Anywhere else, a copy carried here or a clone gone: the download alone.
        assert_eq!(kinds(hows(&source.join("gone"), &installed, posix, true)), Ok(vec!["download"]));
        let carried = source.parent().unwrap().join("carried").join("dbt-edith");
        assert_eq!(kinds(hows(&source, &carried, posix, true)), Ok(vec!["download"]));
        // The clone's own build is a developer's, which a download would replace.
        let built = source.join("target").join("release").join("dbt-edith");
        assert_eq!(hows(&source, &built, posix, true), Err(Why::Locked));
        // Nor is a debug build, or one for a target triple.
        for under in [["target", "debug"], ["target", "x86_64-pc-windows-msvc"]] {
            let dev = under.iter().fold(source.clone(), |p, part| p.join(part)).join("dbt-edith.exe");
            assert_eq!(hows(&source, &dev, posix, true), Err(Why::Locked), "{under:?}");
        }
        let triple = source.join("target").join("x86_64-pc-windows-msvc").join("release").join("dbt-edith.exe");
        assert_eq!(hows(&source, &triple, posix, true), Err(Why::Locked));
        // A folder that only starts with the name is not under it.
        let near = source.join("targets").join("dbt-edith.exe");
        assert_eq!(kinds(hows(&source, &near, posix, true)), Ok(vec!["download"]));
        // Off Windows nothing changes.
        assert_eq!(kinds(hows(&source, &installed, posix, false)), Ok(vec!["install"]));
        assert_eq!(hows(&source, &carried, posix, false), Err(Why::Elsewhere));
        let debug = source.join("target").join("debug").join("dbt-edith");
        assert_eq!(hows(&source, &debug, posix, false), Err(Why::Elsewhere), "off Windows it is as it was");
        // A refusal drops its command; both refused, the download's is told.
        assert_eq!(hows(&source, &installed, shell_kind("cmd.exe"), true), Err(Why::Shell));
        let odd = source.parent().unwrap().join("it's $HOME");
        std::fs::rename(&source, &odd).unwrap();
        assert_eq!(kinds(hows(&odd, &installed, posix, true)), Ok(vec!["download"]), "the clone's path alone");
        assert_eq!(hows(&odd, &odd.join("dbt-edith.exe"), posix, true), Err(Why::Path));
        std::fs::rename(&odd, &source).unwrap();
        let _ = std::fs::remove_dir_all(source.parent().unwrap());
    }

    /// A copy cross-built on another machine, or an install whose clone was
    /// moved, has no clone to find: on Windows it is still offered the download.
    #[test]
    fn a_binary_with_no_clone_is_still_offered_the_download_on_windows() {
        let kinds = |r: Result<Vec<How>, Why>| r.map(|h| h.iter().map(|h| h.kind).collect::<Vec<_>>());
        let exe = Path::new("C:/Users/ada/bin/dbt-edith.exe");
        assert_eq!(kinds(hows_from(None, exe, Some(Shell::Posix), true)), Ok(vec!["download"]));
        assert_eq!(kinds(hows_from(None, exe, Some(Shell::PowerShell), true)), Ok(vec!["download"]));
        assert_eq!(hows_from(None, exe, shell_kind("cmd.exe"), true), Err(Why::Shell));
        assert_eq!(hows_from(None, Path::new("C:/it's $HOME/dbt-edith.exe"), Some(Shell::Posix), true), Err(Why::Path));
        // Anywhere else, nothing is pulled and nothing downloaded.
        assert_eq!(hows_from(None, Path::new("/home/ada/bin/dbt-edith"), Some(Shell::Posix), false), Err(Why::Elsewhere));
        // With a clone it is `hows`.
        let (source, root) = layout("fromclone");
        let installed = root.join("bin").join("dbt-edith");
        assert_eq!(kinds(hows_from(Some(&source), &installed, Some(Shell::Posix), true)), Ok(vec!["download", "install"]));
        let _ = std::fs::remove_dir_all(source.parent().unwrap());
    }

    #[test]
    fn a_release_exe_off_windows_gets_no_command() {
        let exe = Path::new("/home/ada/bin/dbt-edith");
        assert_eq!(download(exe, Some(Shell::Posix), false), Err(Why::Elsewhere));
        // Windows, in a shell neither language is for.
        let exe = Path::new("C:/Users/ada/bin/dbt-edith.exe");
        assert_eq!(download(exe, shell_kind("cmd.exe"), true), Err(Why::Shell));
    }

    /// The architecture decides the file fetched, and the file lands under the
    /// running one's own name, whatever that is called (0123).
    #[test]
    fn an_arm64_build_downloads_the_arm64_file_and_an_x64_one_the_x64_file() {
        assert_eq!(asset(false), "dbt-edith.exe");
        assert_eq!(asset(true), "dbt-edith-arm64.exe");
        let exe = Path::new("C:/Users/ada/bin/edith.exe");
        let base = "https://github.com/IL-William/dbt-edith/releases/latest/download";
        for (name, arm) in [("dbt-edith.exe", false), ("dbt-edith-arm64.exe", true)] {
            for shell in [Shell::Posix, Shell::PowerShell] {
                let how = download_of(asset(arm), exe, Some(shell), true).unwrap();
                let c = &how.command;
                assert!(c.contains(&format!("\"{base}/{name}\"")) && c.contains(&format!("\"{base}/{name}.sha256\"")), "{c}");
                assert!(c.contains("\"C:/Users/ada/bin/edith.exe.new\""), "{c}");
                let other = if arm { "dbt-edith.exe" } else { "dbt-edith-arm64.exe" };
                assert!(!c.contains(&format!("{base}/{other}")), "{c}");
                assert_eq!(how.exe, "edith.exe");
            }
        }
    }

    /// The real sh and curl, on a folder standing for the release: a hash that
    /// matches swaps the file in and ends on the line; one that does not, or
    /// a hash file that is missing, leaves the file as it was, takes back what
    /// came down and prints no line (0123). Needs `sha256sum`, which macOS
    /// has from `coreutils` (`brew install coreutils`) if it lacks one.
    #[cfg(unix)]
    #[test]
    fn sh_swaps_in_a_download_whose_hash_matches_and_keeps_the_exe_when_it_does_not() {
        let dir = scratch("download");
        let (release, bin) = (dir.join("release"), dir.join("bin"));
        std::fs::create_dir_all(&release).unwrap();
        std::fs::create_dir_all(&bin).unwrap();
        let (exe, old) = (bin.join("dbt-edith.exe"), bin.join("dbt-edith.exe.old"));
        let (asset, sum) = (release.join("dbt-edith.exe"), release.join("dbt-edith.exe.sha256"));
        std::fs::write(&asset, "the next one").unwrap();
        let hash = {
            let out = std::process::Command::new("sha256sum").arg(&asset).output().expect("sha256sum is installed");
            String::from_utf8_lossy(&out.stdout).split(' ').next().unwrap().to_string()
        };
        let command = compose_download(Shell::Posix, &format!("file://{}", release.display()), "dbt-edith.exe", exe.to_str().unwrap());
        let run = || {
            let out = std::process::Command::new("sh").args(["-c", &command]).output().unwrap();
            (out.status.success(), String::from_utf8_lossy(&out.stdout).to_string())
        };
        let untouched = |why: &str| {
            assert_eq!(std::fs::read_to_string(&exe).unwrap(), "the running one", "{why}");
            assert!(!old.exists(), "{why}");
            assert!(!bin.join("dbt-edith.exe.new").exists(), "{why}: what came down is removed");
            assert!(!bin.join("dbt-edith.exe.sha256").exists(), "{why}: so is its hash");
        };

        std::fs::write(&exe, "the running one").unwrap();
        std::fs::write(&sum, format!("{hash}  dbt-edith.exe\n")).unwrap();
        let (ok, out) = run();
        assert!(ok && out.trim_end().ends_with(DONE), "{out}");
        assert_eq!(std::fs::read_to_string(&exe).unwrap(), "the next one");
        assert_eq!(std::fs::read_to_string(&old).unwrap(), "the running one", "the running file steps aside");
        assert!(!bin.join("dbt-edith.exe.new").exists() && !bin.join("dbt-edith.exe.sha256").exists());

        std::fs::remove_file(&old).unwrap();
        std::fs::write(&exe, "the running one").unwrap();
        std::fs::write(&sum, format!("{}  dbt-edith.exe\n", "0".repeat(64))).unwrap();
        let (ok, out) = run();
        assert!(!ok && !out.contains(DONE) && out.contains(MISMATCH), "{out}");
        untouched("a hash that differs");

        std::fs::remove_file(&sum).unwrap();
        let (ok, out) = run();
        assert!(!ok && !out.contains(DONE), "{out}");
        untouched("a hash file that is not there");

        // The running file cannot be moved aside, here for being gone: what
        // came down is removed all the same, and nothing claims an install.
        std::fs::write(&sum, format!("{hash}  dbt-edith.exe\n")).unwrap();
        std::fs::remove_file(&exe).unwrap();
        let (ok, out) = run();
        assert!(!ok && !out.contains(DONE), "{out}");
        assert!(!exe.exists() && !old.exists(), "nothing was swapped");
        assert!(!bin.join("dbt-edith.exe.new").exists(), "the download is removed when the rename aside fails");
        assert!(!bin.join("dbt-edith.exe.sha256").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The closing line is chained after the install in every shell and swap
    /// combination, and no failure path reaches it (0111). The clean (0120)
    /// rides with it, and a missing clean leaves every string as it was.
    #[test]
    fn the_closing_line_follows_the_install_alone() {
        assert!(!DONE.contains(['\'', '"', '$', '`', '\\']), "it sits inside a string in either shell");
        let (exe, step) = ("/bin/dbt-edith", "cargo install --locked --path \"/src\"");
        let clean = "cargo clean --release --manifest-path \"/src/Cargo.toml\" --target-dir \"/src/target\"";
        let posix = compose(Shell::Posix, "/src", "1.2.0", step, None, None);
        assert!(posix.ends_with(&format!("{step} && printf '\\n%s\\n' '{DONE}'")), "{posix}");
        // The restore ends in `false`, or the subshell would exit 0 after a
        // failed build and the chain would go on to the line.
        let swapped = compose(Shell::Posix, "/src", "1.2.0", step, None, Some(exe));
        assert!(swapped.contains(&format!("({step} || {{ mv -f \"{exe}.old\" \"{exe}\"; false; }}) && printf")), "{swapped}");
        let ps = compose(Shell::PowerShell, "/src", "1.2.0", step, None, None);
        assert!(ps.ends_with(&format!("{{ {step}; if ($?) {{ Write-Host \"\"; Write-Host \"{DONE}\" }} }}")), "{ps}");
        let ps_swapped = compose(Shell::PowerShell, "/src", "1.2.0", step, None, Some(exe));
        let (ok, failed) = ps_swapped.split_once(" else ").unwrap();
        assert!(ok.ends_with(&format!("{step}; if ($?) {{ Write-Host \"\"; Write-Host \"{DONE}\" }}")), "{ps_swapped}");
        assert!(failed.starts_with("{ Move-Item") && !failed.contains("Write-Host"), "{ps_swapped}");

        // With the clean: after the install, in a group ending on `;` that the
        // restore does not contain, before the line.
        let group = format!("{{ {clean}; printf '\\n%s\\n' '{DONE}'; }}");
        let posix = compose(Shell::Posix, "/src", "1.2.0", step, Some(clean), None);
        assert!(posix.ends_with(&format!("{step} && {group}")), "{posix}");
        let swapped = compose(Shell::Posix, "/src", "1.2.0", step, Some(clean), Some(exe));
        assert!(swapped.ends_with(&format!("({step} || {{ mv -f \"{exe}.old\" \"{exe}\"; false; }}) && {group}")), "{swapped}");
        assert_eq!(swapped.matches("cargo clean").count(), 1, "{swapped}");
        let line = format!("{{ {clean}; Write-Host \"\"; Write-Host \"{DONE}\" }}");
        let ps = compose(Shell::PowerShell, "/src", "1.2.0", step, Some(clean), None);
        assert!(ps.ends_with(&format!("{{ {step}; if ($?) {line} }}")), "{ps}");
        let ps_swapped = compose(Shell::PowerShell, "/src", "1.2.0", step, Some(clean), Some(exe));
        let (ok, failed) = ps_swapped.split_once(" else ").unwrap();
        assert!(ok.ends_with(&format!("{step}; if ($?) {line}")), "{ps_swapped}");
        assert!(failed.starts_with("{ Move-Item") && !failed.contains("cargo clean"), "{ps_swapped}");
        assert_eq!(ps_swapped.matches("cargo clean").count(), 1, "{ps_swapped}");
    }

    /// The real sh, with a stand-in for the install step: success prints the
    /// line, a failed step does not, and with the swap the binary is put back
    /// and the command still fails. The clean (0120) runs after a good
    /// install only, and a failing one neither hides the line nor puts the
    /// binary back.
    #[cfg(unix)]
    #[test]
    fn sh_prints_the_closing_line_only_when_the_install_worked() {
        let dir = scratch("closing");
        let exe = dir.join("dbt-edith");
        let repo = dir.join("clone");
        let marker = dir.join("cleaned");
        let git = |args: &[&str]| {
            let out = std::process::Command::new("git")
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
        let (remote, clone) = (dir.join("remote"), repo.to_str().unwrap());
        let r = remote.to_str().unwrap();
        git(&["init", "-q", "-b", "main", r]);
        git(&["-C", r, "commit", "-q", "--allow-empty", "-m", "one"]);
        git(&["-C", r, "tag", "v1.2.0"]);
        git(&["clone", "-q", r, clone]);
        let run = |command: &str| {
            let out = std::process::Command::new("sh").args(["-c", command]).output().unwrap();
            (out.status.success(), String::from_utf8_lossy(&out.stdout).to_string())
        };
        let e = exe.to_str().unwrap();
        let touches = format!("touch \"{}\"", marker.display());
        for swap in [None, Some(e)] {
            std::fs::write(&exe, "old").unwrap();
            let (ok, out) = run(&compose(Shell::Posix, clone, "1.2.0", "true", None, swap));
            assert!(ok && out.contains(DONE), "{swap:?}: {out}");
            std::fs::write(&exe, "old").unwrap();
            let (ok, out) = run(&compose(Shell::Posix, clone, "1.2.0", "false", None, swap));
            assert!(!ok && !out.contains(DONE), "{swap:?}: a failed step must not claim an install: {out}");
            if swap.is_some() {
                assert_eq!(std::fs::read_to_string(&exe).unwrap(), "old", "the binary is put back");
            }

            // The install worked and the clean did too: it ran, then the line.
            let _ = std::fs::remove_file(&marker);
            std::fs::write(&exe, "old").unwrap();
            let (ok, out) = run(&compose(Shell::Posix, clone, "1.2.0", "true", Some(&touches), swap));
            assert!(ok && out.contains(DONE) && marker.exists(), "{swap:?}: {out}");
            // The install worked and the clean failed: the line still comes,
            // and the binary is not put back over the new one.
            std::fs::write(&exe, "old").unwrap();
            let installs = format!("printf new > \"{e}\"");
            let (ok, out) = run(&compose(Shell::Posix, clone, "1.2.0", &installs, Some("false"), swap));
            assert!(ok && out.contains(DONE), "{swap:?}: a failed clean must not hide the line: {out}");
            assert_eq!(std::fs::read_to_string(&exe).unwrap(), "new", "{swap:?}: the install stays");
            // The install failed: the clean never ran.
            let _ = std::fs::remove_file(&marker);
            std::fs::write(&exe, "old").unwrap();
            let (ok, out) = run(&compose(Shell::Posix, clone, "1.2.0", "false", Some(&touches), swap));
            assert!(!ok && !out.contains(DONE) && !marker.exists(), "{swap:?}: {out}");
            if swap.is_some() {
                assert_eq!(std::fs::read_to_string(&exe).unwrap(), "old", "the binary is put back");
            }
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The real download commands in the real Windows shells, Git Bash and
    /// Windows PowerShell 5.1, against a server on this machine standing for
    /// GitHub: a matching hash swaps the file in and ends on the line, a wrong
    /// one replaces nothing and removes what came down (0123). The command goes
    /// in on the shell's input, as typed at its prompt, which is how it runs
    /// for the user and spares any quoting of it on a command line.
    #[cfg(windows)]
    #[test]
    fn the_real_windows_shells_download_check_and_swap() {
        use std::io::{Read, Write};
        let dir = scratch("windows-download");
        let bin = dir.join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        let (exe, old) = (bin.join("dbt-edith.exe"), bin.join("dbt-edith.exe.old"));
        let new = bin.join("dbt-edith.exe.new");
        let sum = bin.join("dbt-edith.exe.sha256");

        // The file's hash, from PowerShell's own, as the shell under test would read it.
        let asset = dir.join("asset.exe");
        std::fs::write(&asset, "the next one").unwrap();
        let hash = {
            let out = std::process::Command::new("powershell.exe")
                .args(["-NoProfile", "-Command", &format!("(Get-FileHash -Algorithm SHA256 -LiteralPath '{}').Hash", asset.display())])
                .output()
                .expect("powershell.exe runs");
            // CI's `sha256sum` writes it in lower case.
            String::from_utf8_lossy(&out.stdout).trim().to_ascii_lowercase()
        };
        assert_eq!(hash.len(), 64, "{hash}");

        let files = Arc::new(Mutex::new(std::collections::HashMap::<String, Vec<u8>>::new()));
        files.lock().unwrap().insert("/dbt-edith.exe".into(), b"the next one".to_vec());
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let base = format!("http://127.0.0.1:{}", listener.local_addr().unwrap().port());
        let served = files.clone();
        // One thread per connection, for as long as the test process lives.
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let served = served.clone();
                std::thread::spawn(move || {
                    let mut stream = stream;
                    let mut head = Vec::new();
                    let mut byte = [0u8; 1];
                    while !head.ends_with(b"\r\n\r\n") && stream.read(&mut byte).is_ok_and(|n| n == 1) {
                        head.push(byte[0]);
                    }
                    let head = String::from_utf8_lossy(&head).to_string();
                    let path = head.split_whitespace().nth(1).unwrap_or("").to_string();
                    let body = served.lock().unwrap().get(&path).cloned();
                    let (status, body) = match body {
                        Some(body) => ("200 OK", body),
                        None => ("404 Not Found", Vec::new()),
                    };
                    let _ = write!(stream, "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
                    let _ = stream.write_all(&body);
                });
            }
        });

        let e = quotable(&exe, true).expect("a path a shell reads as it is");
        let bash = crate::pty::find_git_bash().or_else(|| {
            // Not WSL's launcher, which would read a Windows path as a Linux one.
            crate::program::on_path("bash").filter(|p| !p.to_string_lossy().to_ascii_lowercase().contains("system32"))
        });
        let mut shells: Vec<(&str, Shell, std::process::Command)> = Vec::new();
        match bash {
            Some(bash) => {
                let mut c = std::process::Command::new(bash);
                c.args(["--noprofile", "--norc"]);
                shells.push(("Git Bash", Shell::Posix, c));
            }
            // A skip that CI would not notice is a test that proves nothing.
            None if std::env::var_os("CI").is_some() => panic!("CI has no Git Bash, so the test would prove half of it"),
            None => eprintln!("no Git Bash here: only PowerShell is run"),
        }
        let mut ps = std::process::Command::new("powershell.exe");
        ps.args(["-NoProfile", "-Command", "-"]);
        shells.push(("PowerShell", Shell::PowerShell, ps));

        for (name, shell, mut shell_cmd) in shells {
            let command = compose_download(shell, &base, "dbt-edith.exe", &e);
            // A profile may set `Stop`, which the command must not be at the mercy of.
            let prelude = if shell == Shell::PowerShell { "$ErrorActionPreference='Stop'\n" } else { "" };
            let mut run = |sum_text: Option<String>| {
                std::fs::write(&exe, "the running one").unwrap();
                // An `.old` left by an earlier update, which the swap overwrites.
                std::fs::write(&old, "a stale one").unwrap();
                for left in [&new, &sum] {
                    let _ = std::fs::remove_file(left);
                }
                // None serves no hash at all: a 404.
                match sum_text {
                    Some(text) => files.lock().unwrap().insert("/dbt-edith.exe.sha256".into(), text.into_bytes()),
                    None => files.lock().unwrap().remove("/dbt-edith.exe.sha256"),
                };
                let mut child = shell_cmd
                    .stdin(std::process::Stdio::piped())
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
                    .spawn()
                    .unwrap();
                child.stdin.take().unwrap().write_all(format!("{prelude}{command}\n").as_bytes()).unwrap();
                let out = child.wait_with_output().unwrap();
                String::from_utf8_lossy(&out.stdout).to_string() + &String::from_utf8_lossy(&out.stderr)
            };
            let out = run(Some(format!("{hash}  dbt-edith.exe\n")));
            assert!(out.contains(DONE), "{name}: {out}");
            assert_eq!(std::fs::read_to_string(&exe).unwrap(), "the next one", "{name}: {out}");
            assert_eq!(std::fs::read_to_string(&old).unwrap(), "the running one", "{name}");
            assert!(!new.exists() && !sum.exists(), "{name}: nothing is left beside it");

            let out = run(Some(format!("{}  dbt-edith.exe\n", "0".repeat(64))));
            assert!(!out.contains(DONE) && out.contains(MISMATCH), "{name}: {out}");
            assert_eq!(std::fs::read_to_string(&exe).unwrap(), "the running one", "{name}: untouched");
            assert_eq!(std::fs::read_to_string(&old).unwrap(), "a stale one", "{name}: so is the `.old`");
            assert!(!new.exists() && !sum.exists(), "{name}: and what came down is removed");

            let out = run(None);
            assert!(!out.contains(DONE) && !out.contains(MISMATCH), "{name}: it stops at the failed download, not at a comparison: {out}");
            assert_eq!(std::fs::read_to_string(&exe).unwrap(), "the running one", "{name}: untouched by a hash that is not there");
            assert_eq!(std::fs::read_to_string(&old).unwrap(), "a stale one", "{name}: {out}");
            assert!(!new.exists() && !sum.exists(), "{name}: nothing left beside it: {out}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The menu says to restart once another binary sits at this one's
    /// path, and not while the path is empty or holds this one again: the
    /// update on Windows renames the running `.exe` away, then back when the
    /// build fails, which keeps its time.
    #[test]
    fn a_binary_put_in_the_running_ones_place_is_told() {
        let dir = scratch("replaced");
        let exe = dir.join("dbt-edith.exe");
        std::fs::write(&exe, "1.25.0").unwrap();
        let u = Update::new(false, None, Err(Why::Elsewhere)).watching(Some(exe.clone()));
        assert!(!u.replaced());
        let old = dir.join("dbt-edith.exe.old");
        std::fs::rename(&exe, &old).unwrap();
        assert!(!u.replaced(), "renamed away while cargo builds");
        std::fs::rename(&old, &exe).unwrap();
        assert!(!u.replaced(), "put back when the build fails");
        std::fs::rename(&exe, &old).unwrap();
        std::fs::write(&exe, "1.28.0").unwrap();
        let later = mtime(&old).unwrap() + std::time::Duration::from_secs(60);
        std::fs::File::options().write(true).open(&exe).unwrap().set_modified(later).unwrap();
        assert!(u.replaced(), "installed");
        // Nothing to watch, nothing to tell.
        assert!(!Update::new(false, None, Err(Why::Elsewhere)).watching(None).replaced());
        assert!(!Update::new(false, None, Err(Why::Elsewhere)).watching(Some(dir.join("missing"))).replaced());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The real git and sh, on a local repository standing for GitHub: the
    /// update brings the release's tag, which a pull naming a branch alone
    /// leaves behind, so the build stamp names the release it builds. The
    /// clone holds a release tag the repository has since moved, as the
    /// clones of the repository recreated under the FSL do: `pull --tags`
    /// refuses to clobber it and stops before moving the clone at all. A
    /// commit merged after the release is not brought along (0129).
    #[cfg(unix)]
    #[test]
    fn the_update_brings_the_tags_the_build_stamp_names() {
        let dir = scratch("tags");
        let (remote, clone) = (dir.join("remote"), dir.join("clone"));
        let run = |program: &str, args: &[&str]| {
            let out = std::process::Command::new(program)
                .args(args)
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env("GIT_AUTHOR_NAME", "Ada")
                .env("GIT_AUTHOR_EMAIL", "ada@example.com")
                .env("GIT_COMMITTER_NAME", "Ada")
                .env("GIT_COMMITTER_EMAIL", "ada@example.com")
                .output()
                .unwrap();
            assert!(out.status.success(), "{program} {args:?}: {}", String::from_utf8_lossy(&out.stderr));
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        };
        let (r, c) = (remote.to_str().unwrap(), clone.to_str().unwrap());
        run("git", &["init", "-q", "-b", "main", r]);
        run("git", &["-C", r, "commit", "-q", "--allow-empty", "-m", "one"]);
        run("git", &["-C", r, "tag", "v1.25.0"]);
        run("git", &["clone", "-q", r, c]);
        run("git", &["-C", c, "commit", "-q", "--allow-empty", "-m", "an older history"]);
        run("git", &["-C", c, "tag", "-f", "v1.25.0"]);
        run("git", &["-C", c, "reset", "-q", "--hard", "origin/main"]);
        run("git", &["-C", r, "commit", "-q", "--allow-empty", "-m", "two"]);
        run("git", &["-C", r, "tag", "v1.28.0"]);
        run("git", &["-C", r, "commit", "-q", "--allow-empty", "-m", "three"]);

        run("sh", &["-c", &compose(Shell::Posix, c, "1.28.0", "true", None, None)]);
        assert_eq!(run("git", &["-C", c, "log", "-1", "--format=%s"]), "two", "the clone moved to the tag, not to main");
        assert_eq!(run("git", &["-C", c, "describe", "--tags"]), "v1.28.0");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The real git: the clone is brought up to the announced tag whatever it
    /// has out, a branch or a detached HEAD, is left alone when already at or
    /// past the tag, and stops the command when it cannot fast-forward (0129).
    #[cfg(unix)]
    #[test]
    fn the_clone_goes_up_to_the_tag_and_no_further() {
        let dir = scratch("uptotag");
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
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        };
        let update_to = |clone: &Path, tag: &str| {
            let out = std::process::Command::new("sh")
                .args(["-c", &compose(Shell::Posix, clone.to_str().unwrap(), tag, "true", None, None)])
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .output()
                .unwrap();
            (out.status.success(), String::from_utf8_lossy(&out.stdout).contains(DONE))
        };
        let update = |clone: &Path| update_to(clone, "1.2.0");
        let r = remote.to_str().unwrap();
        git(&dir, &["init", "-q", "-b", "main", r]);
        git(&remote, &["commit", "-q", "--allow-empty", "-m", "one"]);
        // Cloned before the release, so the tag and the commit it names are
        // both news to it.
        let clones: Vec<PathBuf> = ["on_main", "on_branch", "detached", "past", "diverged"]
            .iter()
            .map(|n| {
                let c = dir.join(n);
                git(&dir, &["clone", "-q", r, c.to_str().unwrap()]);
                c
            })
            .collect();
        let one = git(&remote, &["rev-parse", "HEAD"]);
        git(&remote, &["commit", "-q", "--allow-empty", "-m", "release"]);
        git(&remote, &["tag", "v1.2.0"]);
        let tagged = git(&remote, &["rev-parse", "HEAD"]);
        git(&remote, &["commit", "-q", "--allow-empty", "-m", "after the release"]);
        let after = git(&remote, &["rev-parse", "HEAD"]);
        let head = |c: &Path| git(c, &["rev-parse", "HEAD"]);

        let [on_main, on_branch, detached, past, diverged] = &clones[..] else { unreachable!() };
        assert_eq!(update(on_main), (true, true));
        assert_eq!((head(on_main), git(on_main, &["describe", "--tags"])), (tagged.clone(), "v1.2.0".into()));
        // Another branch than main fast-forwards too, and stays the branch
        // out, so a plain pull there still works later.
        git(on_branch, &["checkout", "-q", "-b", "mine"]);
        assert_eq!(update(on_branch), (true, true));
        assert_eq!((head(on_branch), git(on_branch, &["branch", "--show-current"])), (tagged.clone(), "mine".into()));
        // A detached HEAD moves along with the merge.
        git(detached, &["checkout", "-q", "--detach", &one]);
        assert_eq!(update(detached), (true, true));
        assert_eq!((head(detached), git(detached, &["branch", "--show-current"])), (tagged.clone(), String::new()));
        // Already past the tag: "Already up to date", and it builds what it holds.
        git(past, &["fetch", "-q", "origin"]);
        git(past, &["merge", "-q", "--ff-only", "origin/main"]);
        assert_eq!(head(past), after);
        assert_eq!(update(past), (true, true));
        assert_eq!(head(past), after, "left where it was");
        // A local commit the tag does not contain: the merge cannot
        // fast-forward and nothing is installed.
        git(diverged, &["commit", "-q", "--allow-empty", "-m", "mine alone"]);
        let mine = head(diverged);
        assert_eq!(update(diverged), (false, false));
        assert_eq!(head(diverged), mine);
        // A tag origin does not have: git cannot find it and nothing moves.
        let before = head(on_branch);
        assert_eq!(update_to(on_branch, "9.9.9"), (false, false));
        assert_eq!(head(on_branch), before);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The release comes off the network and enters a typed line, so only three
    /// runs of digits get a command (0129).
    #[test]
    fn a_release_that_is_not_a_version_gets_no_clone_command() {
        let (source, root) = layout("badtag");
        let installed = how(&source, &root.join("bin").join("dbt-edith"), Some(Shell::Posix), false).unwrap();
        assert!(installed.for_release("1.2.0").unwrap().command.contains("merge --ff-only v1.2.0 &&"));
        for bad in ["1.2.0;rm", "1.2", "", "1.2.0 ", "v1.2.0", "1.2.0.1", "1.2.x", "$(id).1.2", "1..2", "-1.2.3"] {
            assert_eq!(installed.for_release(bad), None, "{bad:?}");
        }
        // The server composes per answer: a clone alone is refused with a
        // reason, a download beside it is still offered.
        let update = Update::new(true, None, Ok(vec![installed.clone()]));
        assert_eq!(update.hows_for("1.2.0;rm"), Err(Why::Release));
        assert_eq!(update.hows_for("1.2.0").unwrap()[0].command, installed.for_release("1.2.0").unwrap().command);
        let exe = Path::new("C:/Users/ada/bin/dbt-edith.exe");
        let both = Update::new(true, None, Ok(vec![download(exe, Some(Shell::Posix), true).unwrap(), installed]));
        let offered = both.hows_for("1.2.0;rm").unwrap();
        assert_eq!(offered.iter().map(|h| h.kind).collect::<Vec<_>>(), ["download"]);
        assert_eq!(Update::new(true, None, Err(Why::Shell)).hows_for("1.2.0"), Err(Why::Shell));
        let _ = std::fs::remove_dir_all(source.parent().unwrap());
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
