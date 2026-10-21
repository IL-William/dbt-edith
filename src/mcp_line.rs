//! The MCP tool's one argument, a command line, read into the questions the
//! command line asks (0110).
//!
//! Invariants: no shell ever sees the line. It is split by `split` below,
//! parsed by the command line's own clap definition, and only the flags in
//! `ALLOWED` pass: a flag is accepted by being listed, never refused by being
//! known to be bad, so one added to `Args` later is refused until someone
//! decides it is a question. The project, the manifest and the catalog are the
//! server's, fixed when it starts, and no word of the line becomes a path.

use crate::ls::Output;
use crate::{question_of, stray, Args, Question};
use clap::error::ErrorKind;
use clap::parser::ValueSource;
use clap::{CommandFactory, FromArgMatches};
use std::path::Path;

/// The tool, as `tools/list` gives it. Only `openWorldHint` is written: the
/// spec's default is true, and the tool reaches nothing outside the project,
/// so false is the truth (VS Code asks to confirm a result only when the hint
/// is truthy, which it is not). The other hints stay at their defaults because
/// they are the truth: `--run-collin` replaces collin's cache, which may have
/// come from another catalog or other SQL (0099).
pub const TOOL: &str = r#"{"name":"edith","description":"Answers one dbt-edith command line about this dbt project, from its manifest: --overview, --describe NAME, -s 'SELECTION', --selector NAME or --column MODEL.COLUMN, with their flags. `help` lists every flag.","inputSchema":{"type":"object","properties":{"line":{"type":"string","description":"the flags, no project: --describe orders --sql"}},"additionalProperties":false},"annotations":{"openWorldHint":false}}"#;

/// What the server tells a client about itself, once, at `initialize`.
pub const INSTRUCTIONS: &str = "Questions about this dbt project's models, sources, tests, selections or column lineage: ask the edith tool, which answers from the manifest and never runs dbt.";

/// The reference a `help` call returns: kept out of the tool's description,
/// which VS Code puts in every conversation where the server is enabled.
pub const HELP: &str = concat!(
    "dbt-edith ",
    env!("CARGO_PKG_VERSION"),
    " answers from the manifest dbt wrote; it never runs dbt. One question per call, with the flags of its command line and no project path.\n",
    "--overview [--depth N]          the project's shape: kinds, materializations, tags, selectors, folders. Ask first.\n",
    "--describe NAME[,NAME] [--sql]  a node's card: columns, types, tests, parents, children, reach. --sql adds its file.\n",
    "-s/--select 'LINE' [--exclude 'LINE']  a dbt selection, as dbt ls answers it, tests included: add --exclude resource_type:test\n",
    "  --count-by kind|materialized|source|folder|tag|package  with -s or --selector: count, not list; folder: --depth N\n",
    "--selector NAME                 a selector of selectors.yml\n",
    "--column MODEL.COLUMN           a column's lineage, over the cache beside the manifest\n",
    "  --up N|all --down N|all       how far, 0 by default\n",
    "  --stop-at window,aggregate    roles not walked past downstream\n",
    "  --indirect                    cross filter, join and dedup keys too\n",
    "  --column-tool fusion|collin|snowflake  whose cache answers\n",
    "  --run-collin                  run collin first, which rewrites its cache under target/\n",
    "--output name|json|compact      name by default; compact: one tab-separated line per node or column, under a header\n",
    "Quote a line holding spaces: -s 'stg_orders+1 orders'. No pipe, no shell: count with --count-by KEY, or filter the answer yourself.\n",
    "After the answer: the manifest that answered, then freshness. When it says stale, run dbt parse and ask again.\n",
);

/// The flags a line may carry, by clap argument id: the questions and what
/// shapes their answer.
const ALLOWED: [&str; 16] = [
    "select",
    "exclude",
    "selector",
    "column",
    "up",
    "down",
    "stop_at",
    "indirect",
    "column_tool",
    "run_collin",
    "describe",
    "sql",
    "overview",
    "count_by",
    "depth",
    "output",
];

/// The flags that are refused on purpose, with why. `project` has a sentence
/// of its own, since it names the server's root.
const REFUSED: [(&str, &str); 10] = [
    ("project", ""),
    ("manifest", "named when the server starts, in its own arguments"),
    ("catalog", "named when the server starts, in its own arguments"),
    ("column_lineage", "a path; the server reads the cache beside its manifest"),
    ("compiled", "a path; the server reads the folder beside its manifest"),
    ("port", "not a question"),
    ("shell", "not a question"),
    ("no_open", "not a question"),
    ("no_update_check", "not a question"),
    ("mcp", "not a question"),
];

/// The longest line read. Far past any question.
const MAX_LINE: usize = 16 * 1024;

const SHELL: &str = "|;&<>`$";

/// What a line asks.
pub enum Read {
    /// The reference above.
    Help,
    /// A question, with the shape its answer takes.
    Ask { question: Question, output: Output },
}

/// Reads one line into a question, or says why it is not one. `root`,
/// `manifest` and `catalog` are the server's own, from its start: they are
/// what the question runs against, and nothing in the line can change them.
/// An `Err` is the text of a refusal, ready for the model.
pub fn read(line: &str, root: &Path, manifest: Option<&Path>, catalog: Option<&Path>) -> Result<Read, String> {
    read_with(Args::command(), line, root, manifest, catalog)
}

/// `read` against a given command definition: the command line's own, bar in
/// the test that adds a flag to it.
fn read_with(command: clap::Command, line: &str, root: &Path, manifest: Option<&Path>, catalog: Option<&Path>) -> Result<Read, String> {
    let mut words = split(line).map_err(refusal)?;
    // The recipes in the docs begin with the program's name and the project:
    // pasted as they are, they mean this server's.
    if words.first().is_some_and(|w| w == "dbt-edith" || w == "dbt-edith.exe") {
        words.remove(0);
    }
    if words.first().is_some_and(|w| w == ".") {
        words.remove(0);
    }
    if words.is_empty() || words == ["help"] {
        return Ok(Read::Help);
    }
    let matches = match command.clone().try_get_matches_from(std::iter::once("dbt-edith".to_string()).chain(words)) {
        Ok(m) => m,
        Err(e) => {
            return match e.kind() {
                ErrorKind::DisplayHelp => Ok(Read::Help),
                ErrorKind::DisplayVersion => Err(refusal("--version is not a question".to_string())),
                _ => Err(refusal(clap_said(&e.to_string(), &command))),
            }
        }
    };
    // Only what was typed counts: a default is the command line's own.
    let typed = |id: &str| matches.value_source(id) == Some(ValueSource::CommandLine);
    for arg in command.get_arguments() {
        let id = arg.get_id().as_str();
        if !typed(id) || ALLOWED.contains(&id) {
            continue;
        }
        return Err(refusal(match id {
            "project" if typed("select") || typed("exclude") => "quote a selection holding spaces: -s 'a b'".to_string(),
            "project" => format!(
                "the project is fixed when the server starts: {}. A word left over after a flag's value is read as a project: quote a value holding spaces",
                root.display()
            ),
            _ => {
                let why = REFUSED.iter().find(|(r, _)| *r == id).map_or("not a question", |(_, why)| *why);
                format!("{} is {why}", flag_of(arg))
            }
        }));
    }
    let mut args = Args::from_arg_matches(&matches).map_err(|e| refusal(clap_said(&e.to_string(), &command)))?;
    args.project = root.to_path_buf();
    args.manifest = manifest.map(Path::to_path_buf);
    args.catalog = catalog.map(Path::to_path_buf);
    if let Some(sentence) = stray(&args) {
        return Err(refusal(sentence));
    }
    match question_of(&args) {
        Some(question) => Ok(Read::Ask { question, output: args.output }),
        None => Err(refusal("ask one of --overview, --describe, -s, --selector, --column".to_string())),
    }
}

/// The refusal as the model reads it: one `error:` line, whatever tip clap
/// gave, and where the flags are listed.
fn refusal(why: String) -> String {
    let first = why.lines().next().unwrap_or_default();
    let rest = why.lines().skip(1).collect::<Vec<_>>().join("\n");
    let head = if first.starts_with("error:") { first.to_string() } else { format!("error: {first}") };
    let tail = if rest.is_empty() { String::new() } else { format!("\n{rest}") };
    format!("{head}{tail}\n`help` lists every flag.")
}

/// An argument as the line would spell it.
fn flag_of(arg: &clap::Arg) -> String {
    match (arg.get_long(), arg.get_short()) {
        (Some(long), _) => format!("--{long}"),
        (None, Some(short)) => format!("-{short}"),
        (None, None) => arg.get_id().to_string(),
    }
}

/// What clap said, cut to its `error:` line and the `tip:` after it, without
/// the usage, which names the whole command line. A tip is kept only when it
/// points at what a line may carry: clap suggests `--port` for `--prot`, and
/// the server refuses it.
fn clap_said(text: &str, command: &clap::Command) -> String {
    let mut lines = text.lines();
    let mut said = lines.find(|l| l.starts_with("error:")).unwrap_or("error: the line is not a question of the command line").to_string();
    let mut rest = lines.peekable();
    // An error that ends in a colon lists what it means on the lines after it.
    if said.ends_with(':') {
        while let Some(item) = rest.next_if(|l| l.starts_with("  ") && !l.trim_start().starts_with("tip:")) {
            said.push_str(&format!("\n{}", item.trim_end()));
        }
    }
    match rest.find(|l| l.trim_start().starts_with("tip:")).filter(|tip| tip_is_allowed(tip, command)) {
        Some(tip) => format!("{said}\n{}", tip.trim_end()),
        None => said,
    }
}

/// Whether a tip suggests only flags a line may carry. A tip that is not about
/// a similar argument or value, such as `to pass '-x' as a value, use '-- -x'`,
/// is dropped: `--` would make a project of the rest.
fn tip_is_allowed(tip: &str, command: &clap::Command) -> bool {
    let tip = tip.trim_start();
    if tip.starts_with("tip: a similar value exists") || tip.starts_with("tip: some similar values exist") {
        return true;
    }
    if !(tip.starts_with("tip: a similar argument exists") || tip.starts_with("tip: some similar arguments exist")) {
        return false;
    }
    let named: Vec<&str> = tip.split('\'').skip(1).step_by(2).collect();
    !named.is_empty()
        && named.iter().all(|name| {
            command.get_arguments().any(|a| {
                let spelled = name.trim_start_matches('-');
                let is = if name.starts_with("--") {
                    a.get_long() == Some(spelled)
                } else {
                    a.get_short().is_some_and(|s| s.to_string() == spelled)
                };
                is && ALLOWED.contains(&a.get_id().as_str())
            })
        })
}

/// The line as words, the way a shell would cut it if it only cut. Whitespace
/// separates words; `'...'` is literal; `"..."` is literal but for `\"` and
/// `\\`; a backslash outside quotes is a backslash; `*` and `?` stay, since
/// dbt's `path:` globs need them. Shell syntax outside quotes is refused: the
/// model would otherwise believe a pipe or a substitution happened.
fn split(line: &str) -> Result<Vec<String>, String> {
    if line.len() > MAX_LINE {
        return Err(format!("the line is over {} KiB", MAX_LINE / 1024));
    }
    let mut words = Vec::new();
    let mut word = String::new();
    // A pair of empty quotes is a word, which is why this is not `word.is_empty()`.
    let mut in_word = false;
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        match c {
            c if c.is_whitespace() => {
                if in_word {
                    words.push(std::mem::take(&mut word));
                    in_word = false;
                }
            }
            '\'' => {
                in_word = true;
                loop {
                    match chars.next() {
                        Some('\'') => break,
                        Some(c) => word.push(c),
                        None => return Err("the line has a quote that is never closed".to_string()),
                    }
                }
            }
            '"' => {
                in_word = true;
                loop {
                    match chars.next() {
                        Some('"') => break,
                        Some('\\') => match chars.clone().next() {
                            Some(next @ ('"' | '\\')) => {
                                word.push(next);
                                chars.next();
                            }
                            _ => word.push('\\'),
                        },
                        Some(c) => word.push(c),
                        None => return Err("the line has a quote that is never closed".to_string()),
                    }
                }
            }
            c if SHELL.contains(c) => {
                // A pipe is almost always a count (`| sort | uniq -c`), which a flag answers.
                let tip = if c == '|' {
                    ": count a selection with --count-by KEY, or ask one question and filter its answer yourself"
                } else {
                    ": ask one question, and filter its answer yourself"
                };
                return Err(format!("`{c}` is shell syntax, and the line is read by dbt-edith alone, never by a shell{tip}"));
            }
            c => {
                in_word = true;
                word.push(c);
            }
        }
    }
    if in_word {
        words.push(word);
    }
    Ok(words)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn root() -> &'static Path {
        Path::new("/work/jaffle_shop")
    }

    fn got(line: &str) -> Result<Read, String> {
        read(line, root(), None, None)
    }

    fn words(line: &str) -> Vec<String> {
        split(line).unwrap()
    }

    /// The refusal text, or a panic naming the line that was not refused.
    fn refused(line: &str) -> String {
        match got(line) {
            Err(why) => why,
            Ok(_) => panic!("{line:?} was accepted"),
        }
    }

    fn asked(line: &str) -> (Question, Output) {
        match got(line) {
            Ok(Read::Ask { question, output }) => (question, output),
            Ok(Read::Help) => panic!("{line:?} gave the help"),
            Err(why) => panic!("{line:?} was refused: {why}"),
        }
    }

    #[test]
    fn the_splitter_reads_quotes_and_leaves_the_rest_alone() {
        assert_eq!(words("-s  stg_orders+1\torders"), ["-s", "stg_orders+1", "orders"]);
        assert_eq!(words("-s 'stg_orders+1 orders'"), ["-s", "stg_orders+1 orders"]);
        assert_eq!(words(r#"-s "a \"b\" c\\d""#), ["-s", r#"a "b" c\d"#]);
        assert_eq!(words(r#"-s "a\nb""#), ["-s", r"a\nb"], "a backslash that escapes nothing stays");
        assert_eq!(words(r"-s a\b"), ["-s", r"a\b"], "outside quotes a backslash is a backslash");
        assert_eq!(words("-s path:models/*/stg_?.sql"), ["-s", "path:models/*/stg_?.sql"]);
        assert_eq!(words("a'b c'd"), ["ab cd"], "quotes inside a word join it");
        assert_eq!(words("-s ''"), ["-s", ""], "a pair of quotes is a word");
        assert_eq!(words("--exclude 'it''s'"), ["--exclude", "its"]);
        assert!(words("   ").is_empty());
    }

    #[test]
    fn an_unclosed_quote_is_refused() {
        assert!(split("-s 'a b").unwrap_err().contains("never closed"));
        assert!(split("-s \"a b").unwrap_err().contains("never closed"));
        assert!(split(r#"-s "a b\""#).unwrap_err().contains("never closed"), "an escaped quote does not close");
    }

    #[test]
    fn shell_syntax_is_refused_outside_quotes_and_read_inside_them() {
        for c in SHELL.chars() {
            let why = split(&format!("--overview {c} cut -f1")).unwrap_err();
            assert!(why.contains(&format!("`{c}` is shell syntax")), "{why}");
            assert!(why.contains("never by a shell"), "{why}");
            assert_eq!(why.contains("--count-by"), c == '|', "{why}");
            assert_eq!(words(&format!("-s 'a{c}b'")), ["-s", &format!("a{c}b")]);
            assert_eq!(words(&format!("-s \"a{c}b\"")), ["-s", &format!("a{c}b")]);
        }
    }

    #[test]
    fn a_line_over_sixteen_kib_is_refused() {
        let long = format!("-s {}", "a".repeat(MAX_LINE));
        assert!(split(&long).unwrap_err().contains("over 16 KiB"));
        assert!(split(&"a".repeat(MAX_LINE)).is_ok(), "the limit is the length itself");
    }

    #[test]
    fn the_programs_name_and_a_lone_dot_are_dropped_so_a_recipe_pastes_as_it_is() {
        for line in ["--overview", "dbt-edith --overview", "dbt-edith . --overview", "dbt-edith.exe --overview", ". --overview"] {
            assert!(matches!(asked(line).0, Question::Overview { depth: 3 }), "{line}");
        }
        let why = refused("dbt-edith /tmp --overview");
        assert!(why.contains("the project is fixed when the server starts: /work/jaffle_shop"), "{why}");
    }

    #[test]
    fn help_is_asked_four_ways_and_by_a_flag_beside_a_mode() {
        for line in ["", "   ", "help", "--help", "-h", "dbt-edith --help", "--describe orders --help", "dbt-edith help"] {
            assert!(matches!(got(line), Ok(Read::Help)), "{line:?}");
        }
    }

    #[test]
    fn the_version_is_not_a_question() {
        for line in ["-V", "--version"] {
            let why = refused(line);
            assert!(why.starts_with("error: --version is not a question"), "{why}");
            assert!(why.ends_with("`help` lists every flag."), "{why}");
        }
    }

    #[test]
    fn each_mode_reads_to_its_question_and_the_flags_beside_it() {
        let (q, output) = asked("-s 'stg_orders+1' --exclude resource_type:test --output compact");
        assert!(matches!(q, Question::Select(crate::ls::Query::Line { ref select, ref exclude }, None) if select == "stg_orders+1" && exclude == "resource_type:test"));
        assert_eq!(output, Output::Compact);
        assert!(matches!(asked("--select=orders").0, Question::Select(..)), "the = form");
        assert!(matches!(asked("--selector nightly").0, Question::Select(crate::ls::Query::Named(ref n), None) if n == "nightly"));
        assert!(matches!(asked("--describe orders,customers --sql").0, Question::Describe { ref names, sql: true } if names.len() == 2));
        assert!(matches!(asked("--overview --depth 1 --output=json").0, Question::Overview { depth: 1 }));
        let (q, _) = asked("--column orders.amount --up all --down 2 --stop-at window,aggregate --indirect --column-tool collin");
        assert!(matches!(q, Question::Column(ref a) if a.down == 2 && a.stop_at.len() == 2 && a.indirect && a.tool.is_some()));
        let (q, _) = asked("--column orders.amount --run-collin --indirect");
        assert!(matches!(q, Question::Column(ref a) if a.produce.is_some()));
    }

    #[test]
    fn the_catalog_collin_is_handed_is_the_servers_and_never_the_lines() {
        let q = match read("--column orders.amount --run-collin", root(), None, Some(Path::new("/work/c.json"))) {
            Ok(Read::Ask { question: Question::Column(ask), .. }) => ask,
            _ => panic!("a column question"),
        };
        assert_eq!(q.produce.unwrap().catalog.as_deref(), Some(Path::new("/work/c.json")));
        assert!(refused("--column orders.amount --run-collin --catalog x.json").contains("--catalog is named when the server starts"));
    }

    #[test]
    fn every_flag_that_is_not_a_question_is_refused_with_its_reason() {
        for (line, flag, why) in [
            ("--overview --manifest m.json", "--manifest", "named when the server starts"),
            ("--overview --catalog c.json", "--catalog", "named when the server starts"),
            ("--column a.b --column-lineage c.json", "--column-lineage", "a path"),
            ("--column a.b --run-collin --compiled sql", "--compiled", "a path"),
            ("--overview --port 4321", "--port", "not a question"),
            ("--overview --shell bash", "--shell", "not a question"),
            ("--overview --no-open", "--no-open", "not a question"),
            ("--overview --no-update-check", "--no-update-check", "not a question"),
        ] {
            let said = refused(line);
            assert!(said.starts_with(&format!("error: {flag} is ")) && said.contains(why), "{line}: {said}");
            assert!(said.ends_with("`help` lists every flag."), "{said}");
        }
    }

    #[test]
    fn a_word_left_after_a_selection_says_to_quote_it_and_a_third_is_clap_s() {
        let said = refused("-s stg_orders orders");
        assert!(said.starts_with("error: quote a selection holding spaces: -s 'a b'"), "{said}");
        let said = refused("-s a --exclude b c");
        assert!(said.contains("quote a selection holding spaces"), "{said}");
        let said = refused("-s a b c");
        assert!(said.starts_with("error: unexpected argument 'c' found"), "{said}");
        assert!(!said.contains("Usage"), "{said}");
        let said = refused("--describe orders customers");
        assert!(said.contains("the project is fixed when the server starts"), "{said}");
    }

    #[test]
    fn clap_s_suggestion_survives_when_it_points_at_a_flag_a_line_may_carry() {
        let said = refused("--colum orders.amount");
        assert!(said.starts_with("error: unexpected argument '--colum' found"), "{said}");
        assert!(said.contains("tip: a similar argument exists: '--column'"), "{said}");
        assert!(!said.contains("Usage"), "{said}");
    }

    #[test]
    fn a_suggestion_of_a_refused_flag_is_dropped() {
        let said = refused("--prot 4321");
        assert!(said.starts_with("error: unexpected argument '--prot' found"), "{said}");
        assert!(!said.contains("tip:") && !said.contains("--port"), "{said}");
        let said = refused("--manifes m.json");
        assert!(!said.contains("tip:"), "{said}");
    }

    #[test]
    fn clap_s_own_refusals_come_through_in_a_line() {
        assert!(refused("--column a.b --up lots").contains("lots is neither a number of levels nor all"));
        assert!(refused("--output xml --overview").starts_with("error: invalid value 'xml' for '--output <FORMAT>'"));
        assert!(refused("--overview -s orders").contains("cannot be used with"));
        let said = refused("--sql");
        assert!(said.starts_with("error: the following required arguments were not provided:\n  --describe"), "{said}");
    }

    #[test]
    fn a_flag_of_another_mode_is_named_by_the_command_lines_own_check() {
        assert_eq!(refused("--overview --sql"), "error: --sql goes with --describe\n`help` lists every flag.");
        assert!(refused("--describe orders --indirect").contains("--indirect goes with --column"));
    }

    #[test]
    fn a_line_with_no_question_is_refused() {
        // A default is not a typed flag: this asks nothing.
        let said = refused("--output json");
        assert_eq!(said, "error: ask one of --overview, --describe, -s, --selector, --column\n`help` lists every flag.");
    }

    #[test]
    fn a_default_is_the_command_lines_and_does_not_count_as_typed() {
        // The project and the output have defaults; neither is refused unless typed.
        assert!(got("--overview").is_ok());
        assert!(refused("--overview --output json --port 4321").contains("--port"));
    }

    /// One line per clap argument id that exercises it. A flag added to `Args`
    /// has no line here, and the test below fails until someone decides
    /// whether a line may carry it.
    const SAMPLES: [(&str, &str); 26] = [
        ("project", "--overview /work/other"),
        ("port", "--overview --port 4321"),
        ("manifest", "--overview --manifest m.json"),
        ("catalog", "--overview --catalog c.json"),
        ("column_lineage", "--column a.b --column-lineage c.json"),
        ("shell", "--overview --shell bash"),
        ("no_open", "--overview --no-open"),
        ("no_update_check", "--overview --no-update-check"),
        ("select", "-s orders"),
        ("exclude", "-s orders --exclude resource_type:test"),
        ("selector", "--selector nightly"),
        ("column", "--column orders.amount"),
        ("up", "--column orders.amount --up 1"),
        ("down", "--column orders.amount --down 1"),
        ("stop_at", "--column orders.amount --stop-at window"),
        ("indirect", "--column orders.amount --indirect"),
        ("column_tool", "--column orders.amount --column-tool collin"),
        ("run_collin", "--column orders.amount --run-collin"),
        ("compiled", "--column orders.amount --run-collin --compiled sql"),
        ("describe", "--describe orders"),
        ("sql", "--describe orders --sql"),
        ("overview", "--overview"),
        ("count_by", "-s orders --count-by folder --depth 2"),
        ("depth", "--overview --depth 2"),
        ("output", "--overview --output json"),
        ("mcp", "--mcp"),
    ];

    #[test]
    fn every_flag_of_the_command_line_is_allowed_on_purpose_or_refused() {
        let ids: BTreeSet<String> = Args::command().get_arguments().map(|a| a.get_id().to_string()).collect();
        let sampled: BTreeSet<String> = SAMPLES.iter().map(|(id, _)| id.to_string()).collect();
        assert_eq!(ids, sampled, "an argument of Args with no sample line, or a sample for none");
        for id in &ids {
            let allowed = ALLOWED.contains(&id.as_str());
            let refused_on_purpose = REFUSED.iter().any(|(r, _)| r == id);
            assert!(allowed != refused_on_purpose, "{id} must be in exactly one of ALLOWED and REFUSED");
            let line = SAMPLES.iter().find(|(s, _)| s == id).unwrap().1;
            match (allowed, got(line)) {
                (true, Ok(_)) => {}
                (true, Err(why)) => panic!("{id} is allowed, and {line:?} was refused: {why}"),
                (false, Ok(_)) => panic!("{id} is refused, and {line:?} was accepted"),
                (false, Err(why)) => assert!(why.starts_with("error: ") && why.ends_with("`help` lists every flag."), "{line:?}: {why}"),
            }
        }
        // Every id the table names is an argument.
        for (id, _) in REFUSED {
            assert!(ids.contains(id), "{id} is in REFUSED and not in Args");
        }
    }

    #[test]
    fn a_flag_not_listed_is_refused_whatever_it_is_called() {
        // The allow-list accepts by name: a command with one more flag is
        // refused for it until ALLOWED lists it.
        let command = Args::command().arg(clap::Arg::new("future").long("future").action(clap::ArgAction::SetTrue));
        let Err(said) = read_with(command, "--overview --future", root(), None, None) else { panic!("--future was accepted") };
        assert!(said.starts_with("error: --future is not a question"), "{said}");
        assert!(said.ends_with("`help` lists every flag."), "{said}");
    }

    /// The flags named in a text: `--word` and `-w` as the tool's help writes them.
    fn flags_in(text: &str) -> BTreeSet<String> {
        text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '-')).filter(|w| w.starts_with("--") && w.len() > 2).map(str::to_string).collect()
    }

    #[test]
    fn the_help_names_every_allowed_flag_and_no_other_in_both_spellings() {
        let command = Args::command();
        let allowed: BTreeSet<String> = command
            .get_arguments()
            .filter(|a| ALLOWED.contains(&a.get_id().as_str()))
            .map(|a| format!("--{}", a.get_long().expect("an allowed flag has a long name")))
            .collect();
        assert_eq!(flags_in(HELP), allowed);
        for a in command.get_arguments().filter(|a| ALLOWED.contains(&a.get_id().as_str())) {
            if let Some(short) = a.get_short() {
                assert!(HELP.contains(&format!("-{short}/--{}", a.get_long().unwrap())), "-{short} is not in the help");
            }
        }
        // The line the help shows is read: both spellings.
        assert!(got("-s orders").is_ok() && got("--select orders").is_ok());
    }

    #[test]
    fn the_texts_stay_small() {
        let tools = format!("[{TOOL}]");
        assert!(tools.len() <= 512, "tools array: {} bytes", tools.len());
        assert!(INSTRUCTIONS.len() <= 192, "instructions: {} bytes", INSTRUCTIONS.len());
        assert!(HELP.len() <= 1536, "help: {} bytes", HELP.len());
        let tool: serde_json::Value = serde_json::from_str(TOOL).unwrap();
        let description = tool["description"].as_str().unwrap();
        assert!(description.len() <= 240, "description: {} bytes", description.len());
        assert_eq!(tool["name"], "edith");
        assert_eq!(tool["inputSchema"]["type"], "object", "a schema's root is an object");
        assert_eq!(tool["annotations"], serde_json::json!({"openWorldHint": false}));
        assert!(!HELP.contains('\u{2014}') && !TOOL.contains('\u{2014}'), "no em dash");
    }
}
