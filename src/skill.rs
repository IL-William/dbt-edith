//! `--skill`: the agent skill shipped as `docs/agent-skill/SKILL.md`, or with
//! `mcp` its variant for a client that defers MCP tools, printed for the binary
//! that prints it (0114, 0136).
//!
//! Invariants: nothing is read, written or started. The files are compiled in,
//! so the text is the one this version was released with, and the binary never
//! writes a client's settings or skill folder: a caller redirects the output
//! where its client looks. The tests hold each file to the binary: every long
//! flag written anywhere in it is one this version has, and every command or
//! tool line it cites parses and is accepted, every selection it cites resolves
//! on a small graph and every `--count-by` key it lists is a key. They do not
//! check what a recipe means, so one that is accepted and wrong is caught only
//! by reading.

use std::io::Write;

/// Which skill `--skill` prints. Two files, since a client that defers MCP
/// tools has to be told to load the tool, which a client with a terminal has
/// no use for (0136).
#[derive(clap::ValueEnum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    /// The commands, for a client with a terminal. What `--skill` alone prints.
    Command,
    /// The `edith` tool, for a client that defers MCP tools until it searches.
    Mcp,
}

/// The files as shipped, with `{version}` still in them.
const TEMPLATE: &str = include_str!("../docs/agent-skill/SKILL.md");
const MCP_TEMPLATE: &str = include_str!("../docs/agent-skill/mcp/SKILL.md");

/// The skill for this version, the one place a file's `{version}` is filled.
pub fn text(kind: Kind) -> String {
    let template = match kind {
        Kind::Command => TEMPLATE,
        Kind::Mcp => MCP_TEMPLATE,
    };
    template.replace("{version}", env!("CARGO_PKG_VERSION"))
}

/// Prints the skill to stdout and returns the exit code.
pub fn print(kind: Kind) -> i32 {
    write_to(kind, &mut std::io::stdout().lock(), &mut std::io::stderr())
}

/// Written as the other modes write: a closed pipe (`dbt-edith --skill | head`)
/// is someone having read enough, any other failure is said and is exit 2.
fn write_to(kind: Kind, out: &mut impl Write, err: &mut impl Write) -> i32 {
    crate::ls::ended(out.write_all(text(kind).as_bytes()).and_then(|_| out.flush()).map(|_| 0), err)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{mcp_line, question_of, stray, Args};
    use clap::{CommandFactory, Parser, ValueEnum};
    use serde_json::json;

    /// A span with the placeholders a reader fills in made plain words.
    fn plain(span: &str) -> String {
        // `model.column` first: `model` alone is the next one.
        span.replace("model.column", "orders.amount").replace("name[,name...]", "orders,customers").replace("<flags>", "--overview").replace("model", "orders")
    }

    /// The commands a text cites: its code spans that begin with the program's
    /// name, the placeholders made plain.
    fn commands(text: &str) -> Vec<String> {
        text.split('`').skip(1).step_by(2).filter(|span| span.starts_with("dbt-edith ")).map(plain).collect()
    }

    /// The lines a text cites for the tool: its code spans that start with a
    /// flag, hold a space and have a question's flag among their words, so a
    /// flag the tool refuses cannot hide behind the question's. A bare
    /// `--describe` is a flag spoken of, not a line; `--job-order` and
    /// `--job-coverage` alone are lines.
    fn tool_lines(text: &str) -> Vec<String> {
        const QUESTIONS: [&str; 7] = ["--overview", "--describe", "--job", "--job-coverage", "--column", "--selector", "-s"];
        text.split('`')
            .skip(1)
            .step_by(2)
            .filter(|span| (span.starts_with('-') && span.contains(' ') && span.split(' ').any(|word| QUESTIONS.contains(&word))) || *span == "--job-order" || *span == "--job-coverage")
            .map(plain)
            .collect()
    }

    /// What the tool's lines are as commands: the program and the project put
    /// in front, so one check serves both skills.
    fn as_commands(text: &str) -> Vec<String> {
        tool_lines(text).into_iter().map(|line| format!("dbt-edith . {line}")).collect()
    }

    /// Every `--word` in a text, wherever it stands: a sentence, a table cell,
    /// the description.
    fn long_flags(text: &str) -> Vec<String> {
        let mut found = Vec::new();
        let mut rest = text;
        while let Some(at) = rest.find("--") {
            let after = &rest[at + 2..];
            let word: String = after.chars().take_while(|c| c.is_ascii_lowercase() || *c == '-').collect();
            if word.chars().next().is_some_and(|c| c.is_ascii_lowercase()) {
                found.push(word.trim_end_matches('-').to_string());
            }
            rest = after;
        }
        found
    }

    /// A small graph to resolve the file's selections on: a source feeding a
    /// staging model feeding a mart.
    fn graph() -> crate::graph::Graph {
        let manifest = json!({
            "metadata": { "dbt_version": "1.12.5", "project_name": "shop" },
            "nodes": {
                "model.shop.stg_orders": {
                    "name": "stg_orders", "resource_type": "model", "package_name": "shop",
                    "original_file_path": "models/staging/stg_orders.sql", "fqn": ["shop", "staging", "stg_orders"],
                },
                "model.shop.orders": {
                    "name": "orders", "resource_type": "model", "package_name": "shop",
                    "original_file_path": "models/marts/orders.sql", "fqn": ["shop", "marts", "orders"],
                },
            },
            "sources": {
                "source.shop.raw.orders": {
                    "name": "orders", "resource_type": "source", "package_name": "shop", "source_name": "raw",
                    "original_file_path": "models/staging/sources.yml", "fqn": ["shop", "staging", "raw", "orders"],
                },
            },
            "parent_map": { "model.shop.stg_orders": ["source.shop.raw.orders"], "model.shop.orders": ["model.shop.stg_orders"] },
        });
        crate::graph::Graph::build(serde_json::from_value(manifest).unwrap(), std::path::Path::new("manifest.json"), 0, 0)
    }

    /// The selections a text cites, as `(line, exclude)`: the `-s` of every
    /// command, and every bare span made of a model and its `+` and numbers.
    fn selections(text: &str, commands: &[String]) -> Vec<(String, String)> {
        let mut found = Vec::new();
        for command in commands {
            let Ok(words) = mcp_line::split(command) else { continue };
            if let Ok(args) = Args::try_parse_from(&words) {
                if let Some(select) = args.select {
                    found.push((select, args.exclude.unwrap_or_default()));
                }
            }
        }
        for span in text.split('`').skip(1).step_by(2) {
            if span.contains("model") && span.chars().all(|c| "0123456789+model".contains(c)) {
                found.push((span.replace("model", "orders"), String::new()));
            }
        }
        found
    }

    /// What is wrong with a text as a skill for this binary, one sentence each,
    /// its `commands` being the ones it cites, spelled as the command line.
    fn flaws(text: &str, commands: &[String]) -> Vec<String> {
        let mut flaws = Vec::new();
        let known: Vec<String> = Args::command().get_arguments().filter_map(|a| a.get_long().map(str::to_string)).collect();
        for flag in long_flags(text) {
            if !known.contains(&flag) {
                flaws.push(format!("--{flag} is not a flag of this binary"));
            }
        }
        let g = graph();
        for (line, exclude) in selections(text, commands) {
            // A term that matches nothing is a warning, not an error: the
            // graph is built so that every recipe selects something.
            match crate::select::select(&g, &line, &exclude, crate::select::Tests::Eager) {
                Err(e) => flaws.push(format!("-s '{line}' --exclude '{exclude}' does not resolve: {e:?}")),
                Ok((_, resolved)) if resolved.nodes.is_empty() => flaws.push(format!("-s '{line}' --exclude '{exclude}' selects nothing: {:?}", resolved.warnings)),
                Ok(_) => {}
            }
        }
        // The sentence that lists the keys: each backticked word is one.
        if let Some(sentence) = text.lines().find(|l| l.starts_with("`--count-by` counts")) {
            for key in sentence.split('`').skip(1).step_by(2).skip(1).filter(|k| !k.starts_with("--")) {
                if crate::ls::CountBy::from_str(key, false).is_err() {
                    flaws.push(format!("`{key}` is not a --count-by key"));
                }
            }
        } else {
            flaws.push("no sentence lists the --count-by keys".to_string());
        }
        let mut rest = text;
        while let Some(at) = rest.find("--count-by ") {
            rest = &rest[at + "--count-by ".len()..];
            let value: String = rest.chars().take_while(|c| c.is_ascii_lowercase()).collect();
            if !value.is_empty() && crate::ls::CountBy::from_str(&value, false).is_err() {
                flaws.push(format!("--count-by {value} is not a key"));
            }
        }
        for command in commands {
            let words = match mcp_line::split(command) {
                Ok(words) => words,
                Err(e) => {
                    flaws.push(format!("{command}: {e}"));
                    continue;
                }
            };
            let args = match Args::try_parse_from(&words) {
                Ok(args) => args,
                Err(e) => {
                    flaws.push(format!("{command}: {e}"));
                    continue;
                }
            };
            if let Some(why) = stray(&args) {
                flaws.push(format!("{command}: {why}"));
            }
            // The one command that takes no project, and asks no question.
            if args.skill.is_some() {
                continue;
            }
            if args.project != std::path::Path::new(".") {
                flaws.push(format!("{command}: the project is not `.`"));
            }
            if question_of(&args).is_none() {
                flaws.push(format!("{command} asks nothing"));
            }
            // Over MCP the same line, which the server reads as its own.
            if mcp_line::read(command, std::path::Path::new("."), None, None).is_err() {
                flaws.push(format!("{command} is refused over MCP"));
            }
        }
        flaws
    }

    /// The commands a text of this kind cites, as `flaws` wants them. The
    /// command skill also writes bare lines, such as the job names' recipe,
    /// which get the tool's check as well.
    fn cited_in(kind: Kind, text: &str) -> Vec<String> {
        match kind {
            Kind::Command => commands(text).into_iter().chain(as_commands(text)).collect(),
            Kind::Mcp => as_commands(text),
        }
    }

    /// The commands a skill cites, as `flaws` wants them.
    fn cited(kind: Kind) -> Vec<String> {
        cited_in(kind, template(kind))
    }

    fn template(kind: Kind) -> &'static str {
        match kind {
            Kind::Command => TEMPLATE,
            Kind::Mcp => MCP_TEMPLATE,
        }
    }

    #[test]
    fn the_text_is_the_file_with_the_version_filled_in() {
        for kind in Kind::value_variants() {
            let text = text(*kind);
            assert!(!text.contains("{version}"));
            assert!(text.contains(&format!("<!-- written by dbt-edith {}; ", env!("CARGO_PKG_VERSION"))));
            assert_eq!(text.replace(env!("CARGO_PKG_VERSION"), "{version}"), template(*kind));
        }
        assert_ne!(text(Kind::Command), text(Kind::Mcp));
    }

    #[test]
    fn the_frontmatter_names_the_skill_and_describes_it_within_the_limit() {
        for (kind, name) in [(Kind::Command, "dbt-edith"), (Kind::Mcp, "dbt-edith-mcp")] {
            let rest = template(kind).strip_prefix("---\n").expect("the file starts with ---");
            let (front, body) = rest.split_once("\n---\n").expect("the frontmatter ends with ---");
            assert!(front.lines().any(|l| l == format!("name: {name}")), "{name}");
            let description = front.lines().find_map(|l| l.strip_prefix("description: ")).expect("a description");
            // Quoted, since a plain YAML scalar cannot hold ": ".
            let description = description.strip_prefix('"').and_then(|d| d.strip_suffix('"')).expect("a double-quoted description");
            assert!(!description.contains('"') && !description.contains('\\'), "nothing to escape in a quoted scalar");
            // Past 1000 characters a client may cut or refuse it.
            assert!(description.chars().count() <= 1000, "{name} description: {} characters", description.chars().count());
            // The description is what decides whether an agent loads the skill.
            assert!(description.contains("USE FIRST"));
            // One description line and no more keys: a second `description:` or a
            // folded one would be read as a different skill.
            assert_eq!(front.lines().count(), 2);
            // The comment right under the frontmatter says which version wrote it.
            assert!(body.starts_with("<!-- written by dbt-edith {version}; "));
        }
        // A client holding both, one in each folder, must not see one name twice.
        let name_of = |kind| text(kind).lines().nth(1).and_then(|l| l.strip_prefix("name: ")).map(str::to_string).expect("a name line");
        assert_ne!(name_of(Kind::Command), name_of(Kind::Mcp));
        // The tool is deferred: only the description is in context before the
        // skill is loaded, so it is where the agent is told to load the tool.
        assert!(MCP_TEMPLATE.lines().nth(2).unwrap().contains("deferred: load it first"));
    }

    #[test]
    fn the_files_stay_short_and_have_no_em_dash() {
        for kind in Kind::value_variants() {
            let file = template(*kind);
            assert!(file.len() <= 4096, "{kind:?}: {} bytes", file.len());
            assert!(!file.contains('\u{2014}'));
        }
    }

    #[test]
    fn every_flag_and_command_the_file_cites_is_one_this_binary_accepts() {
        for kind in Kind::value_variants() {
            // A tripwire on the extraction: the recipes are several.
            let cited = cited(*kind);
            assert!(cited.len() >= 9, "{kind:?}: {} commands found", cited.len());
            assert_eq!(flaws(template(*kind), &cited), Vec::<String>::new(), "{kind:?}");
        }
    }

    #[test]
    fn every_line_the_tool_skill_cites_is_one_the_tool_reads_as_a_question() {
        let lines = tool_lines(MCP_TEMPLATE);
        assert!(lines.len() >= 9, "{} lines found", lines.len());
        for line in lines {
            // As the client sends it: no program, no project.
            assert!(matches!(mcp_line::read(&line, std::path::Path::new("."), None, None), Ok(mcp_line::Read::Ask { .. })), "{line}");
        }
        // The lines are the tool's: the only command it cites is the one that
        // prints this skill again, and none of its lines carries a project.
        assert!(commands(MCP_TEMPLATE).iter().all(|c| c == "dbt-edith --skill mcp"));
        assert!(!MCP_TEMPLATE.contains("`. -"));
    }

    #[test]
    fn a_misspelled_flag_or_key_anywhere_in_the_file_is_found() {
        // The same check on copies of the file with one mistake each: in a
        // command, in a sentence, in a table cell and in a count key.
        for (right, wrong) in [
            ("--sql", "--sqll"),
            ("--down all", "--dwon all"),
            // Known to the command line and refused over MCP.
            ("--column model.column --up all", "--column model.column --port 1 --up all"),
            // The refused flag first: the line's first word is not a question's.
            ("-s '+model'", "--port 1 -s '+model'"),
            ("--depth N", "--dept N"),
            ("--run-collin", "--run-colin"),
            ("--exclude", "--exclud"),
            ("--count-by source", "--count-by sources"),
            ("--count-by folder", "--count-by dir"),
            ("-s 'model+'", "-s 'model+' --nope"),
            ("--exclude resource_type:test", "--exclude resource_typ:test"),
            ("`folder`", "`foldr`"),
            ("`1+model+1`", "`1+model+1+`"),
            ("`1+model`", "`1++model`"),
        ] {
            for kind in Kind::value_variants() {
                let file = template(*kind);
                assert!(file.contains(right), "{kind:?}: {right} is not in the file");
                let mistaken = file.replacen(right, wrong, 1);
                let commands = cited_in(*kind, &mistaken);
                assert!(!flaws(&mistaken, &commands).is_empty(), "{kind:?}: {wrong} went unnoticed");
            }
        }
    }

    #[test]
    fn the_skill_is_written_whole_and_a_failed_write_is_said() {
        for kind in Kind::value_variants() {
            let mut out = Vec::new();
            let mut err = Vec::new();
            assert_eq!(write_to(*kind, &mut out, &mut err), 0);
            assert_eq!(String::from_utf8(out).unwrap(), text(*kind));
            assert!(err.is_empty());
        }

        /// A writer the way a full disk or a closed pipe fails.
        struct Fails(std::io::ErrorKind);
        impl Write for Fails {
            fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                Err(self.0.into())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let mut err = Vec::new();
        assert_eq!(write_to(Kind::Mcp, &mut Fails(std::io::ErrorKind::BrokenPipe), &mut err), 0, "a closed pipe is someone having read enough");
        assert!(err.is_empty());
        assert_eq!(write_to(Kind::Mcp, &mut Fails(std::io::ErrorKind::Other), &mut err), crate::ls::CANNOT_RUN);
        assert!(String::from_utf8(err).unwrap().starts_with("error: "));
    }

    #[test]
    fn every_key_of_count_by_is_named() {
        for kind in Kind::value_variants() {
            for key in crate::ls::CountBy::value_variants() {
                let name = key.to_possible_value().unwrap().get_name().to_string();
                let file = template(*kind);
                assert!(file.contains(&format!("`{name}`")) || file.contains(&format!("--count-by {name}")), "{kind:?}: {name} is not in the skill");
            }
        }
    }

    #[test]
    fn both_skills_lead_with_the_shallow_overview_and_the_tool_says_so() {
        for kind in Kind::value_variants() {
            let first = template(*kind).lines().find(|l| l.starts_with("| the project, its job count")).expect("the overview row");
            assert!(first.contains("--overview --depth 2`, once, first"), "{kind:?}: {first}");
        }
        assert!(mcp_line::HELP.contains("Ask first: --depth 2."));
    }
}
