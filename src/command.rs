//! A dbt command line, as an orchestrator's job writes one, read into what it
//! runs and what it selects (0063).
//!
//! The line comes from a file this tool did not write, so every flag is known
//! by name or the line is refused: a flag wrongly taken to have no value would
//! read its value as a selector, and one wrongly taken to have a value would
//! swallow a term. The values of `--vars` and `--args` are never kept, since
//! that is where a secret would sit (0012); the line as shown says `…` there.

use crate::graph::Kind;
use crate::select::Indirect;

/// What a command does to the nodes it selects.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Verb {
    Run,
    Build,
    Test,
    Seed,
    Snapshot,
    SourceFreshness,
    Compile,
    Clone,
    List,
    Show,
    DocsGenerate,
    RunOperation,
    /// Commands that select nothing: deps, parse, debug and their kind.
    Other,
}

impl Verb {
    /// dbt's own words for it, for the line a person reads.
    pub fn name(self) -> &'static str {
        match self {
            Verb::Run => "run",
            Verb::Build => "build",
            Verb::Test => "test",
            Verb::Seed => "seed",
            Verb::Snapshot => "snapshot",
            Verb::SourceFreshness => "source freshness",
            Verb::Compile => "compile",
            Verb::Clone => "clone",
            Verb::List => "ls",
            Verb::Show => "show",
            Verb::DocsGenerate => "docs generate",
            Verb::RunOperation => "run-operation",
            Verb::Other => "other",
        }
    }

    /// The resource types this command acts on, or None for one that acts on
    /// no node at all. `ls` lists every type.
    pub fn kinds(self) -> Option<&'static [Kind]> {
        Some(match self {
            Verb::Run => &[Kind::Model],
            Verb::Build => &[Kind::Model, Kind::Seed, Kind::Snapshot, Kind::Test],
            Verb::Test => &[Kind::Test],
            Verb::Seed => &[Kind::Seed],
            Verb::Snapshot => &[Kind::Snapshot],
            Verb::SourceFreshness => &[Kind::Source],
            Verb::Compile => &[Kind::Model, Kind::Snapshot, Kind::Test, Kind::Analysis],
            Verb::Clone => &[Kind::Model, Kind::Seed, Kind::Snapshot],
            Verb::Show => &[Kind::Model, Kind::Snapshot, Kind::Seed, Kind::Test, Kind::Analysis],
            Verb::List => &[
                Kind::Model,
                Kind::Seed,
                Kind::Snapshot,
                Kind::Test,
                Kind::Source,
                Kind::Exposure,
                Kind::Analysis,
            ],
            Verb::DocsGenerate | Verb::RunOperation | Verb::Other => return None,
        })
    }

    /// Whether what it selects is built, tested or checked, as opposed to
    /// compiled, cloned or listed. Only these count towards coverage.
    pub fn executes(self) -> bool {
        matches!(self, Verb::Run | Verb::Build | Verb::Test | Verb::Seed | Verb::Snapshot | Verb::SourceFreshness)
    }
}

/// One dbt invocation, read.
#[derive(Clone, Debug, PartialEq)]
pub struct Invocation {
    pub verb: Verb,
    /// The `--select` values, each one split on whitespace as dbt splits them.
    pub select: Vec<String>,
    pub exclude: Vec<String>,
    pub selector: Option<String>,
    /// `--resource-type`, narrowing what the verb acts on; empty when absent.
    pub resource_types: Vec<Kind>,
    pub exclude_resource_types: Vec<Kind>,
    pub indirect: Option<Indirect>,
    /// The macro of a `run-operation`.
    pub operation: Option<String>,
    /// The line as written, with `dbt` in front and the values of `--vars`
    /// and `--args` replaced by `…`.
    pub shown: String,
}

impl Invocation {
    /// What it acts on: the verb's types, narrowed by `--resource-type` and
    /// `--exclude-resource-type`. None for a command that acts on no node.
    pub fn kinds(&self) -> Option<Vec<Kind>> {
        let base = self.verb.kinds()?;
        Some(
            base.iter()
                .copied()
                .filter(|k| self.resource_types.is_empty() || self.resource_types.contains(k))
                .filter(|k| !self.exclude_resource_types.contains(k))
                .collect(),
        )
    }

    /// The `dbt ls` that should list what this step acts on, for checking this
    /// tool against dbt (0024). None for a command that selects nothing.
    pub fn ls(&self) -> Option<String> {
        let kinds = self.kinds()?;
        let mut out = String::from("dbt ls");
        if let Some(name) = &self.selector {
            out.push_str(&format!(" --selector \"{name}\""));
        } else {
            if !self.select.is_empty() {
                out.push_str(&format!(" --select \"{}\"", self.select.join(" ")));
            }
            if !self.exclude.is_empty() {
                out.push_str(&format!(" --exclude \"{}\"", self.exclude.join(" ")));
            }
        }
        if self.verb != Verb::List || !self.resource_types.is_empty() || !self.exclude_resource_types.is_empty() {
            for k in &kinds {
                out.push_str(" --resource-type ");
                out.push_str(k.as_str());
            }
        }
        if let Some(mode) = self.indirect {
            out.push_str(" --indirect-selection ");
            out.push_str(match mode {
                Indirect::Eager => "eager",
                Indirect::Cautious => "cautious",
                Indirect::Buildable => "buildable",
                Indirect::Empty => "empty",
            });
        }
        out.push_str(" --output name");
        Some(out)
    }
}

/// Why a selection cannot be typed into the terminal as it is, or nothing.
/// `ls` puts it between double quotes, where the shell still acts on `$`, a
/// backtick, `\\` and `!`, and a control character acts on the line editor;
/// PowerShell, the Windows fallback, reads the typographic double quotes as
/// `"` too. A job file is a project file and its selector is typed on a
/// click, so the rule `src/pty.rs` applies to a path applies here. Refused
/// rather than cleaned, so that the name typed is the name resolved.
fn shell_acts_on(text: &str) -> Option<String> {
    text.chars()
        .find(|c| matches!(c, '"' | '\u{201c}' | '\u{201d}' | '\u{201e}' | '$' | '`' | '\\' | '!') || c.is_control())
        .map(|c| match c {
            '"' | '\u{201c}' | '\u{201d}' | '\u{201e}' => "a quote inside a selector".to_string(),
            c if c.is_control() => "a control character inside a selector".to_string(),
            c => format!("a `{c}` inside a selector, which the shell would act on"),
        })
}

/// Flags that take no value and change nothing about what is selected.
const SWITCHES: &[&str] = &[
    "--full-refresh", "-f", "--no-full-refresh", "--fail-fast", "-x", "--no-fail-fast", "--defer", "--no-defer",
    "--favor-state", "--no-favor-state", "--empty", "--no-empty", "--store-failures", "--warn-error", "--debug",
    "-d", "--no-debug", "--quiet", "-q", "--no-quiet", "--use-colors", "--no-use-colors", "--use-colors-file",
    "--no-use-colors-file", "--partial-parse", "--no-partial-parse", "--static-parser", "--no-static-parser",
    "--version-check", "--no-version-check", "--write-json", "--no-write-json", "--print", "--no-print",
    "--send-anonymous-usage-stats", "--no-send-anonymous-usage-stats", "--cache-selected-only",
    "--no-cache-selected-only", "--introspect", "--no-introspect", "--compile", "--no-compile", "--static",
    "--empty-catalog", "--populate-cache", "--no-populate-cache", "--partial-parse-file-diff",
    "--no-partial-parse-file-diff", "--use-experimental-parser", "--no-use-experimental-parser",
    "--inject-ephemeral-ctes", "--no-inject-ephemeral-ctes", "--export-saved-queries", "--no-export-saved-queries",
    "--include-saved-query", "--no-include-saved-query", "--upgrade", "--lock", "--skip-profile-setup",
    "--show-resource-report", "--no-show-resource-report", "--write-index", "--write-lineage",
];

/// Flags that take one value and change nothing about what is selected.
/// `--project-dir` is among them: a job is taken to run this project (0063).
const VALUED: &[&str] = &[
    "--target", "-t", "--threads", "--profiles-dir", "--project-dir", "--profile", "--target-path", "--log-path",
    "--log-level", "--log-level-file", "--log-format", "--log-format-file", "--log-file-max-bytes", "--state",
    "--defer-state", "--warn-error-options", "--printer-width", "--event-time-start", "--event-time-end",
    "--sample", "--output", "-o", "--output-keys", "--limit", "--record-timing-info", "-r", "--static-analysis",
    "--packages-install-path", "--macro-debugging",
];

/// Flags whose value is never kept nor shown: a secret would sit there.
const HIDDEN: &[&str] = &["--vars", "--args", "--inline"];

const SELECT: &[&str] = &["-s", "--select", "-m", "--models", "--model"];
const EXCLUDE: &[&str] = &["--exclude"];
const RESOURCE: &[&str] = &["--resource-type", "--resource-types"];
const NOT_RESOURCE: &[&str] = &["--exclude-resource-type", "--exclude-resource-types"];

/// A token and whether the shell would have changed it on its own: a `$`,
/// a backtick or a template makes it something only the run knows.
struct Token {
    text: String,
    dynamic: bool,
}

/// Shell words, the way a POSIX shell splits them: whitespace apart, single
/// quotes literal, double quotes with their four escapes, a backslash outside
/// quotes escaping one character. Anything that would make the line more
/// than one command is refused, since only the first would be read.
fn words(line: &str) -> Result<Vec<Token>, String> {
    let mut out: Vec<Token> = Vec::new();
    let mut cur = String::new();
    let mut started = false;
    let mut dynamic = false;
    let mut chars = line.chars().peekable();
    let push = |out: &mut Vec<Token>, cur: &mut String, started: &mut bool, dynamic: &mut bool| {
        if *started {
            out.push(Token { text: std::mem::take(cur), dynamic: *dynamic });
        }
        *started = false;
        *dynamic = false;
    };
    while let Some(c) = chars.next() {
        match c {
            ' ' | '\t' | '\n' | '\r' => push(&mut out, &mut cur, &mut started, &mut dynamic),
            '\'' => {
                started = true;
                loop {
                    match chars.next() {
                        Some('\'') => break,
                        Some(ch) => cur.push(ch),
                        None => return Err("a quote that never closes".into()),
                    }
                }
            }
            '"' => {
                started = true;
                loop {
                    match chars.next() {
                        Some('"') => break,
                        Some('\\') => match chars.next() {
                            Some(e @ ('"' | '\\' | '$' | '`')) => cur.push(e),
                            Some('\n') => {}
                            Some(other) => {
                                cur.push('\\');
                                cur.push(other);
                            }
                            None => return Err("a quote that never closes".into()),
                        },
                        Some(ch) => {
                            if ch == '$' || ch == '`' {
                                dynamic = true;
                            }
                            cur.push(ch);
                        }
                        None => return Err("a quote that never closes".into()),
                    }
                }
            }
            '\\' => match chars.next() {
                Some('\n') => {}
                Some(ch) => {
                    started = true;
                    cur.push(ch);
                }
                None => {}
            },
            // A template is one word whatever it holds, spaces and brackets
            // included, and only the run knows what it becomes.
            '{' if matches!(chars.peek(), Some('{') | Some('%')) => {
                let close = if chars.next() == Some('{') { '}' } else { '%' };
                started = true;
                dynamic = true;
                cur.push('{');
                cur.push(if close == '}' { '{' } else { '%' });
                let mut prev = ' ';
                loop {
                    match chars.next() {
                        Some('}') if prev == close => {
                            cur.push('}');
                            break;
                        }
                        Some(ch) => {
                            cur.push(ch);
                            prev = ch;
                        }
                        None => return Err("a template that never closes".into()),
                    }
                }
            }
            '#' if !started => break,
            ';' | '&' | '|' | '<' | '>' | '(' | ')' => {
                return Err(format!(
                    "`{c}` outside quotes makes this more than one dbt command, and only one is read"
                ))
            }
            '$' | '`' => {
                started = true;
                dynamic = true;
                cur.push(c);
            }
            _ => {
                started = true;
                cur.push(c);
            }
        }
    }
    push(&mut out, &mut cur, &mut started, &mut dynamic);
    for t in &mut out {
        if t.text.contains("{{") || t.text.contains("{%") {
            t.dynamic = true;
        }
    }
    Ok(out)
}

fn verb_of(word: &str, next: Option<&str>) -> Option<(Verb, usize)> {
    Some(match (word, next) {
        ("run", _) => (Verb::Run, 1),
        ("build", _) => (Verb::Build, 1),
        ("test", _) => (Verb::Test, 1),
        ("seed", _) => (Verb::Seed, 1),
        ("snapshot", _) => (Verb::Snapshot, 1),
        ("compile", _) => (Verb::Compile, 1),
        ("clone", _) => (Verb::Clone, 1),
        ("ls" | "list", _) => (Verb::List, 1),
        ("show", _) => (Verb::Show, 1),
        ("run-operation", _) => (Verb::RunOperation, 1),
        ("source", Some("freshness" | "snapshot-freshness")) => (Verb::SourceFreshness, 2),
        ("docs", Some("generate")) => (Verb::DocsGenerate, 2),
        ("docs", Some("serve")) => (Verb::Other, 2),
        ("deps" | "parse" | "debug" | "clean" | "init" | "environment" | "seed-check", _) => (Verb::Other, 1),
        _ => return None,
    })
}

fn kind_of(value: &str) -> Result<Option<Kind>, String> {
    Ok(Some(match value {
        "model" => Kind::Model,
        "seed" => Kind::Seed,
        "snapshot" => Kind::Snapshot,
        "test" | "unit_test" => Kind::Test,
        "source" => Kind::Source,
        "analysis" => Kind::Analysis,
        "exposure" => Kind::Exposure,
        "all" | "default" => return Ok(None),
        "metric" | "semantic_model" | "saved_query" | "function" => Kind::Other,
        other => return Err(format!("`{other}` is not a resource type dbt knows")),
    }))
}

/// How a word reads back in the line shown: quoted when it would split.
fn shown_word(w: &str) -> String {
    if !w.is_empty() && !w.contains(char::is_whitespace) && !w.contains(['"', '\'']) {
        return w.to_string();
    }
    format!("'{}'", w.replace('\'', "'\\''"))
}

/// Reads one command. The `dbt` in front is optional, since a factory may add
/// it, and so are `NAME=value` assignments before it, which are never kept.
pub fn read(line: &str) -> Result<Invocation, String> {
    let words = words(line)?;
    let mut i = 0usize;
    while words.get(i).is_some_and(|w| is_assignment(&w.text)) {
        i += 1;
    }
    if words.get(i).is_some_and(|w| w.text == "dbt") {
        i += 1;
    }
    let mut shown: Vec<String> = vec!["dbt".to_string()];
    let mut inv = Invocation {
        verb: Verb::Other,
        select: Vec::new(),
        exclude: Vec::new(),
        selector: None,
        resource_types: Vec::new(),
        exclude_resource_types: Vec::new(),
        indirect: None,
        operation: None,
        shown: String::new(),
    };

    // Flags dbt takes before its command, `dbt --debug run`.
    while let Some(w) = words.get(i).filter(|w| w.text.starts_with('-')) {
        let used = ignored(&words, i, &mut shown)?;
        if used == 0 {
            return Err(format!("`{}` is a flag this tool does not know, so the line is not read", w.text));
        }
        i += used;
    }
    let Some(first) = words.get(i) else {
        return Err("no dbt command".into());
    };
    if first.dynamic {
        return Err("the dbt command is only known when the job runs".into());
    }
    let next = words.get(i + 1).map(|w| w.text.as_str());
    let Some((verb, used)) = verb_of(&first.text, next) else {
        return Err(format!("`{}` is not a dbt command this tool reads", first.text));
    };
    inv.verb = verb;
    for w in &words[i..i + used] {
        shown.push(w.text.clone());
    }
    i += used;
    if verb == Verb::RunOperation {
        match words.get(i) {
            Some(w) if !w.text.starts_with('-') => {
                inv.operation = Some(w.text.clone());
                shown.push(shown_word(&w.text));
                i += 1;
            }
            _ => return Err("run-operation without the macro it runs".into()),
        }
    }

    while i < words.len() {
        let w = &words[i];
        let (flag, inline) = match w.text.split_once('=') {
            Some((f, v)) if f.starts_with("--") => (f, Some(v)),
            _ => (w.text.as_str(), None),
        };
        let group: Option<&mut Vec<String>> = if SELECT.contains(&flag) {
            Some(&mut inv.select)
        } else if EXCLUDE.contains(&flag) {
            Some(&mut inv.exclude)
        } else {
            None
        };
        if let Some(into) = group {
            let (values, used) = values_after(&words, i, inline)?;
            if values.is_empty() {
                return Err(format!("`{flag}` with no value"));
            }
            for v in &values {
                if v.dynamic {
                    return Err("a selection only known when the job runs, built with $, a backtick or a template".into());
                }
                // Judged piece by piece: a newline or a tab between the terms
                // of a quoted value, as a block scalar in a job file writes
                // them, never reaches the terminal, since `ls` joins the
                // pieces with spaces.
                for piece in v.text.split_whitespace() {
                    if let Some(why) = shell_acts_on(piece) {
                        return Err(why);
                    }
                    into.push(piece.to_string());
                }
            }
            shown.push(flag.to_string());
            shown.extend(values.iter().map(|v| shown_word(&v.text)));
            i += used;
            continue;
        }
        if flag == "--selector" {
            let value = match inline {
                Some(v) => Token { text: v.to_string(), dynamic: w.dynamic },
                None => match words.get(i + 1) {
                    Some(v) if !v.text.starts_with('-') => Token { text: v.text.clone(), dynamic: v.dynamic },
                    _ => return Err("`--selector` with no name".into()),
                },
            };
            if value.dynamic {
                return Err("a selector only known when the job runs, built with $, a backtick or a template".into());
            }
            if let Some(why) = shell_acts_on(&value.text) {
                return Err(why);
            }
            shown.push("--selector".to_string());
            shown.push(shown_word(&value.text));
            inv.selector = Some(value.text);
            i += if inline.is_some() { 1 } else { 2 };
            continue;
        }
        let narrowing = if RESOURCE.contains(&flag) {
            Some(false)
        } else if NOT_RESOURCE.contains(&flag) {
            Some(true)
        } else {
            None
        };
        if let Some(negated) = narrowing {
            let (values, used) = values_after(&words, i, inline)?;
            if values.is_empty() {
                return Err(format!("`{flag}` with no value"));
            }
            shown.push(flag.to_string());
            for v in &values {
                shown.push(shown_word(&v.text));
                for part in v.text.split_whitespace() {
                    if let Some(kind) = kind_of(part)? {
                        let into = if negated { &mut inv.exclude_resource_types } else { &mut inv.resource_types };
                        into.push(kind);
                    }
                }
            }
            i += used;
            continue;
        }
        if flag == "--indirect-selection" {
            let value = match inline {
                Some(v) => v.to_string(),
                None => words.get(i + 1).map(|v| v.text.clone()).unwrap_or_default(),
            };
            let Some(mode) = Indirect::by_name(&value) else {
                return Err("`--indirect-selection` takes eager, cautious, buildable or empty".into());
            };
            inv.indirect = Some(mode);
            shown.push("--indirect-selection".to_string());
            shown.push(value);
            i += if inline.is_some() { 1 } else { 2 };
            continue;
        }
        if !w.text.starts_with('-') || w.text == "-" {
            return Err(format!("`{}` where dbt takes no argument", w.text));
        }
        let used = ignored(&words, i, &mut shown)?;
        if used == 0 {
            return Err(format!("`{flag}` is a flag this tool does not know, so the line is not read"));
        }
        i += used;
    }
    inv.shown = shown.join(" ");
    Ok(inv)
}

/// A flag that selects nothing: how many words it takes, written into the
/// line shown, with a hidden value as `…`. 0 when the flag is not known.
fn ignored(words: &[Token], i: usize, shown: &mut Vec<String>) -> Result<usize, String> {
    let text = words[i].text.as_str();
    let (flag, inline) = match text.split_once('=') {
        Some((f, v)) if f.starts_with("--") => (f, Some(v)),
        _ => (text, None),
    };
    if SWITCHES.contains(&flag) {
        if inline.is_some() {
            return Err(format!("`{flag}` takes no value"));
        }
        shown.push(flag.to_string());
        return Ok(1);
    }
    let hidden = HIDDEN.contains(&flag);
    if !hidden && !VALUED.contains(&flag) {
        return Ok(0);
    }
    let (value, used) = match inline {
        Some(v) => (v.to_string(), 1),
        None => match words.get(i + 1) {
            // A value may begin with a dash, a negative limit for instance,
            // but never with two: that is the next flag.
            Some(v) if !v.text.starts_with("--") => (v.text.clone(), 2),
            _ => return Err(format!("`{flag}` with no value")),
        },
    };
    let value = if hidden { "…".to_string() } else { shown_word(&value) };
    shown.push(if inline.is_some() { format!("{flag}={value}") } else { format!("{flag} {value}") });
    Ok(used)
}

/// The values of a flag that takes several, up to the next flag.
fn values_after<'w>(words: &'w [Token], i: usize, inline: Option<&str>) -> Result<(Vec<Token>, usize), String> {
    if let Some(v) = inline {
        // `--select=a b` gives dbt one value and a stray word, which it refuses.
        return Ok((vec![Token { text: v.to_string(), dynamic: words[i].dynamic }], 1));
    }
    let mut out = Vec::new();
    let mut j = i + 1;
    while let Some(w) = words.get(j).filter(|w| !w.text.starts_with('-')) {
        out.push(Token { text: w.text.clone(), dynamic: w.dynamic });
        j += 1;
    }
    Ok((out, j - i))
}

fn is_assignment(word: &str) -> bool {
    let Some((name, _)) = word.split_once('=') else { return false };
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_') && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(line: &str) -> Invocation {
        read(line).unwrap_or_else(|e| panic!("{line}: {e}"))
    }
    fn owned(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn reads_the_shapes_jobs_write() {
        let inv = ok("run --select tag:sales --exclude tag:wip --exclude tag:old");
        assert_eq!(inv.verb, Verb::Run);
        assert_eq!(inv.select, owned(&["tag:sales"]));
        assert_eq!(inv.exclude, owned(&["tag:wip", "tag:old"]));
        assert_eq!(inv.shown, "dbt run --select tag:sales --exclude tag:wip --exclude tag:old");

        let inv = ok("dbt run --full-refresh -s models/staging orders+ 1+customers --exclude stg_payments+");
        assert_eq!(inv.select, owned(&["models/staging", "orders+", "1+customers"]));
        assert_eq!(inv.exclude, owned(&["stg_payments+"]));

        let inv = ok("test --selector nightly_checks");
        assert_eq!((inv.verb, inv.selector.as_deref()), (Verb::Test, Some("nightly_checks")));

        let inv = ok("seed --full-refresh");
        assert_eq!((inv.verb, inv.select.len()), (Verb::Seed, 0));

        let inv = ok("run-operation refresh_grants --args '{role: reporter}'");
        assert_eq!((inv.verb, inv.operation.as_deref()), (Verb::RunOperation, Some("refresh_grants")));
        assert_eq!(inv.shown, "dbt run-operation refresh_grants --args …");
        assert_eq!(inv.ls(), None);

        assert_eq!(ok("compile").verb, Verb::Compile);
        assert_eq!(ok("dbt source freshness --select source:crm").verb, Verb::SourceFreshness);
        assert_eq!(ok("dbt docs generate").kinds(), None);
    }

    #[test]
    fn reads_the_spellings_dbt_accepts_for_one_flag() {
        for line in [
            "dbt build --select=orders",
            "dbt build -s orders",
            "dbt build -m orders",
            "dbt build --models orders",
            "dbt build --model orders",
            "dbt build --select \"orders\"",
        ] {
            assert_eq!(ok(line).select, owned(&["orders"]), "{line}");
        }
        // A quoted value is split the way dbt splits it.
        assert_eq!(ok("dbt run -s 'a b' c").select, owned(&["a", "b", "c"]));
        assert_eq!(ok("dbt run -s a -s b").select, owned(&["a", "b"]));
        assert_eq!(ok("dbt --debug --log-format json run -s a").select, owned(&["a"]));
        assert_eq!(ok("DBT_PROFILES_DIR=/x dbt run -s a").select, owned(&["a"]));
        assert_eq!(ok("dbt run \\\n  -s a").select, owned(&["a"]));
    }

    #[test]
    fn hides_the_values_where_a_secret_would_sit() {
        let inv = ok("dbt run -s a --vars '{\"password\": \"hunter2\"}' --target prod --args=x");
        assert!(!inv.shown.contains("hunter2"));
        assert_eq!(inv.shown, "dbt run -s a --vars … --target prod --args=…");
        let inv = ok("SECRET=hunter2 dbt run");
        assert!(!inv.shown.contains("hunter2"));
    }

    #[test]
    fn narrows_by_resource_type_and_reads_the_indirect_mode() {
        let inv = ok("dbt build -s orders --resource-type model --resource-type seed --indirect-selection cautious");
        assert_eq!(inv.kinds(), Some(vec![Kind::Model, Kind::Seed]));
        assert_eq!(inv.indirect, Some(Indirect::Cautious));
        let inv = ok("dbt build --exclude-resource-type test");
        assert_eq!(inv.kinds(), Some(vec![Kind::Model, Kind::Seed, Kind::Snapshot]));
        assert_eq!(ok("dbt build --resource-type all").kinds().unwrap().len(), 4);
    }

    /// Inside quotes the shell keeps a carriage return or an escape, and the
    /// terminal's line editor would act on it when the command is typed.
    #[test]
    fn a_selection_the_shell_would_act_on_is_refused_not_retyped() {
        // Single quotes keep these literal in the job file, and double quotes
        // would not in the terminal: no ls command, rather than one with the
        // character dropped, which would type a name other than the one read.
        for line in [
            "test --selector 'night\rly'",
            "run --select 'a\u{1b}[2Jb'",
            "test --selector '$(touch /tmp/x)'",
            "run --select '`id`'",
            "run --select 'a\\b'",
            "run --exclude 'a!b'",
            "test --selector 'é\u{85}'",
            "run --select 'a\u{201d};calc;\u{201c}b'",
            "test --selector 'night\u{201e}ly'",
        ] {
            assert!(read(line).is_err(), "{line:?} was accepted");
        }
        assert_eq!(
            ok("run --select 'tag:nightly+ é'").ls().unwrap(),
            "dbt ls --select \"tag:nightly+ é\" --resource-type model --output name"
        );
        // A block scalar in a job file breaks a quoted value over lines and
        // tabs; those never reach the terminal, since the pieces are joined
        // with spaces, so the step reads as it did before.
        assert_eq!(
            ok("run --select 'model_a\n  model_b\tmodel_c\r\n'").ls().unwrap(),
            "dbt ls --select \"model_a model_b model_c\" --resource-type model --output name"
        );
        // A C1 control that is whitespace to Rust and to Python alike, NEL,
        // splits a `--select` the same way and is typed as the space it is.
        assert_eq!(
            ok("run --select 'é\u{85}b'").ls().unwrap(),
            "dbt ls --select \"é b\" --resource-type model --output name"
        );
    }

    #[test]
    fn writes_the_dbt_ls_that_checks_it() {
        assert_eq!(
            ok("run --select tag:a +b --exclude c").ls().unwrap(),
            "dbt ls --select \"tag:a +b\" --exclude \"c\" --resource-type model --output name"
        );
        assert_eq!(
            ok("test --selector nightly").ls().unwrap(),
            "dbt ls --selector \"nightly\" --resource-type test --output name"
        );
        assert_eq!(ok("seed").ls().unwrap(), "dbt ls --resource-type seed --output name");
        assert_eq!(
            ok("build -s a --indirect-selection empty").ls().unwrap(),
            "dbt ls --select \"a\" --resource-type model --resource-type seed --resource-type snapshot \
             --resource-type test --indirect-selection empty --output name"
        );
    }

    #[test]
    fn refuses_what_it_cannot_read_and_says_why() {
        let cases: &[(&str, &str)] = &[
            ("cd /app && dbt run", "more than one dbt command"),
            ("dbt run; dbt test", "more than one dbt command"),
            ("dbt run --select $MODELS", "only known when the job runs"),
            ("dbt run --select {{ var('x') }}", "only known when the job runs"),
            ("dbt run --frobnicate", "`--frobnicate` is a flag this tool does not know"),
            ("dbt run -e a", "`-e` is a flag this tool does not know"),
            ("dbt run orders", "`orders` where dbt takes no argument"),
            ("dbt run --select", "`--select` with no value"),
            ("dbt run --target", "`--target` with no value"),
            ("dbt run --indirect-selection lazy", "eager, cautious, buildable or empty"),
            ("dbt build --resource-type widget", "`widget` is not a resource type"),
            ("dbt frobnicate", "`frobnicate` is not a dbt command"),
            ("dbt run -s 'a", "a quote that never closes"),
            ("", "no dbt command"),
            ("dbt run-operation", "run-operation without the macro"),
            ("$CMD -s a", "only known when the job runs"),
        ];
        for (line, want) in cases {
            let got = read(line).expect_err(line);
            assert!(got.contains(want), "{line}: {got}");
        }
    }

    #[test]
    fn a_template_outside_the_selection_does_not_stop_the_line() {
        let inv = ok("dbt run -s a --target {{ env }}");
        assert_eq!(inv.select, owned(&["a"]));
    }
}
