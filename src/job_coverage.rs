//! The `--job-coverage` mode: what the page's Job coverage report shows, which
//! of the project's own models, seeds and snapshots no scheduled job runs and
//! which steps select nothing, printed, and the process ends (0131).
//!
//! Invariants: it reads the jobs as `--job` and `--job-order` do
//! (`jobs::known`, the project's own `.dbt-edith/jobs.yml`, one read) and
//! asks `jobs::coverage`, the function the page's route asks, so both say the
//! same over the same files. The lists keep that function's order, a node by
//! folder then name then id and a step as the files were walked, so the same
//! files give the same answer (0055). The page's own rows (`jobs`, a step's
//! `index`) are left out of the JSON, which has a view of its own: what
//! `/api/jobs/coverage` sends does not change. A list is the answer, so text
//! and compact cut none of them; the sentence over them says how much there
//! is.

use crate::graph::Graph;
use crate::job::clean;
use crate::jobs::{self, Coverage, Known, Row, StepRef};
use crate::ls::{self, field, Output, ANSWERED, CANNOT_RUN};
use std::io::{self, BufWriter, Write};
use std::path::Path;

/// The header `--output compact` prints first. A node fills `name` and
/// `where` with its folder; a step fills `name` with its job and step and
/// `where` with its source and `file:line`; `why` is a step's alone.
const COMPACT_HEADER: &str = "list\tname\twhere\twhy";

/// The whole mode, from the command line's arguments to the exit code.
pub fn run_job_coverage(project: &Path, manifest: Option<&Path>, catalog: Option<&Path>, output: Output) -> i32 {
    let (root, _, graph) = match ls::open_with_catalog(project, manifest, catalog) {
        Ok(opened) => opened,
        Err(why) => return ls::cannot_open(&why),
    };
    let mut err = io::stderr().lock();
    let mut out = BufWriter::new(io::stdout().lock());
    let written = respond_job_coverage(&root, &graph, None, output, &mut out, &mut err);
    ls::ended(written, &mut err)
}

/// What the mode does once the project is open. `kept` is the jobs a caller
/// holds between calls (MCP); None reads them here, if the project has a
/// sources file.
pub(crate) fn respond_job_coverage(
    root: &Path,
    graph: &Graph,
    kept: Option<&Known>,
    output: Output,
    out: &mut impl Write,
    err: &mut impl Write,
) -> io::Result<i32> {
    let code = job_coverage(root, graph, kept, output, out, err)?;
    out.flush()?;
    let _ = ls::provenance(root, graph, err);
    Ok(code)
}

fn job_coverage(root: &Path, graph: &Graph, kept: Option<&Known>, output: Output, out: &mut impl Write, err: &mut impl Write) -> io::Result<i32> {
    let known = kept.cloned().unwrap_or_else(|| jobs::known(graph, root));
    let read = match known {
        Known::Absent => {
            writeln!(err, "error: no {} in this project; name the sources in the Jobs settings", clean(jobs::SOURCES_FILE))?;
            return Ok(CANNOT_RUN);
        }
        Known::Refused(why) => {
            writeln!(err, "error: {} is not read, {}", jobs::SOURCES_FILE, clean(&why))?;
            return Ok(CANNOT_RUN);
        }
        Known::Read(read) => read,
    };
    // Nothing was read when every source failed outright. A source cut at its
    // bound still gave the files it walked, so it is a partial read (`unread`
    // counts both, for the note).
    let none_read = !read.sources.is_empty() && read.sources.iter().all(|s| !s.error.is_empty());
    // A job file refused inside a source that was read is counted apart by
    // `unread`; its jobs are in no list, so it is said as a count.
    let refused = read.sources.iter().map(|s| s.refused.len()).sum();
    let answer = View::of(jobs::coverage(graph, &read), jobs::unread(&read), refused, none_read);
    match output {
        Output::Json => {
            serde_json::to_writer(&mut *out, &answer)?;
            writeln!(out)?;
        }
        Output::Compact => write_compact(&answer, out, err)?,
        // The default is read, like a card: name prints the text.
        Output::Name => write_text(&answer, out)?,
    }
    Ok(ANSWERED)
}

/* -------------------------------------------------------------- answer -- */

/// A step the report lists, without the position the page opens it by.
#[derive(serde::Serialize, Debug)]
struct Step {
    source: String,
    job: String,
    n: usize,
    #[serde(skip_serializing_if = "String::is_empty")]
    name: String,
    file: String,
    line: usize,
    shown: String,
    why: String,
}

impl Step {
    fn of(s: StepRef) -> Step {
        Step { source: s.source, job: s.job, n: s.n, name: s.name, file: s.file, line: s.line, shown: s.shown, why: s.why }
    }

    /// `job › task`, else `job › step 2`, as the page names it.
    fn title(&self) -> String {
        if self.name.is_empty() {
            format!("{} › step {}", self.job, self.n)
        } else {
            format!("{} › {}", self.job, self.name)
        }
    }

    fn place(&self) -> String {
        format!("{} · {}:{}", self.source, self.file, self.line)
    }
}

/// `Coverage` as the page's route sends it, minus the jobs the page lists
/// apart and a step's index, and plus the sources that were named and not
/// read, which could hold a job that runs what the lists call run by none.
#[derive(serde::Serialize, Debug)]
struct View {
    configured: bool,
    total: usize,
    never: Vec<Row>,
    unscheduled: Vec<Row>,
    unread: Vec<Row>,
    ephemeral: Vec<Row>,
    empty: Vec<Step>,
    unresolved: Vec<Step>,
    #[serde(skip_serializing_if = "Option::is_none")]
    not_read: Option<jobs::Unread>,
    /// Job files refused whole in the sources that were read, left out when 0.
    #[serde(skip_serializing_if = "is_zero")]
    files_refused: usize,
    /// Every source failed outright, so no job was seen.
    #[serde(skip)]
    none_read: bool,
}

fn is_zero(n: &usize) -> bool {
    *n == 0
}

impl View {
    fn of(cov: Coverage, not_read: Option<jobs::Unread>, files_refused: usize, none_read: bool) -> View {
        // With no source, or none that was read, every node is "run by no
        // job", which says nothing of the project; the page draws no list
        // then either. What the project holds, its total and its ephemerals,
        // stays true and is kept.
        if !cov.configured || none_read {
            return View {
                configured: cov.configured,
                total: cov.total,
                never: Vec::new(),
                unscheduled: Vec::new(),
                unread: Vec::new(),
                ephemeral: cov.ephemeral,
                empty: Vec::new(),
                unresolved: Vec::new(),
                not_read: if cov.configured { not_read } else { None },
                files_refused: 0,
                none_read: cov.configured,
            };
        }
        View {
            configured: true,
            total: cov.total,
            never: cov.never,
            unscheduled: cov.unscheduled,
            unread: cov.unread,
            ephemeral: cov.ephemeral,
            empty: cov.empty.into_iter().map(Step::of).collect(),
            unresolved: cov.unresolved.into_iter().map(Step::of).collect(),
            not_read,
            files_refused,
            none_read: false,
        }
    }

    /// Every source named failed outright: no job was seen, so each node would
    /// read as run by no job, and the lists say nothing of the project, as
    /// with no source at all.
    fn none_read(&self) -> bool {
        self.none_read && self.not_read.is_some()
    }

    /// What was not read, as one sentence for the text's line and compact's
    /// row, or None when everything was.
    fn note(&self) -> Option<String> {
        let mut parts = Vec::new();
        if let Some(u) = &self.not_read {
            parts.push(u.text());
        }
        if self.files_refused > 0 {
            parts.push(format!("{} (refused)", plural(self.files_refused, "job file", "job files")));
        }
        (!parts.is_empty()).then(|| parts.join("; "))
    }

    /// The sentence over the lists, worded as the page's `jobsCoverageLine`
    /// words it (`web/app.js`), so the two say the same.
    fn line(&self) -> String {
        let covered = self.total - self.never.len() - self.unscheduled.len() - self.unread.len();
        let mut rest = Vec::new();
        if !self.unread.is_empty() {
            rest.push(format!("{} only in jobs whose schedule is not read", self.unread.len()));
        }
        if !self.unscheduled.is_empty() {
            rest.push(format!("{} only in unscheduled jobs", self.unscheduled.len()));
        }
        if !self.never.is_empty() {
            rest.push(format!("{} in no job", self.never.len()));
        }
        let tail = if rest.is_empty() { String::new() } else { format!("; {}", rest.join(", ")) };
        format!("{covered} of {} models, seeds and snapshots run in a scheduled job{tail}.", self.total)
    }

    /// The node lists in the page's order, with the title the page gives each
    /// and the key the JSON gives it.
    fn nodes(&self) -> [(&'static str, &'static str, &[Row]); 4] {
        [
            ("never", "run by no job", &self.never),
            ("unscheduled", "only in unscheduled jobs", &self.unscheduled),
            ("unread", "only in jobs whose schedule is not read", &self.unread),
            ("ephemeral", "ephemeral, so never run on their own", &self.ephemeral),
        ]
    }

    fn steps(&self) -> [(&'static str, &'static str, &[Step]); 2] {
        [("empty", "steps that select nothing they act on", &self.empty), ("unresolved", "steps not resolved", &self.unresolved)]
    }
}

/// A node by its folder and name, the way a path reads.
fn path_of(row: &Row) -> String {
    if row.folder.is_empty() {
        row.name.clone()
    } else {
        format!("{}/{}", row.folder, row.name)
    }
}

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/* ---------------------------------------------------------------- text -- */

fn write_text(a: &View, out: &mut impl Write) -> io::Result<()> {
    if !a.configured {
        // `jobs: []`, worded as --job-order words it.
        writeln!(out, "no sources in {}", jobs::SOURCES_FILE)?;
        return Ok(());
    }
    // What a source not read holds is unknown, so a list may be too long:
    // said before them, where a client that cuts the answer still has it.
    if a.none_read() {
        if let Some(note) = a.note() {
            writeln!(out, "not read: {}", clean(&note))?;
        }
        return Ok(());
    }
    writeln!(out, "{}", a.line())?;
    if let Some(note) = a.note() {
        writeln!(out, "not read: {}", clean(&note))?;
    }
    for (_, title, rows) in a.nodes() {
        if rows.is_empty() {
            continue;
        }
        writeln!(out, "{title}  {}", rows.len())?;
        for r in rows {
            writeln!(out, "{}", clean(&format!("  {}", path_of(r))))?;
        }
    }
    for (_, title, steps) in a.steps() {
        if steps.is_empty() {
            continue;
        }
        writeln!(out, "{title}  {}", steps.len())?;
        for s in steps {
            writeln!(out, "{}", clean(&format!("  {}  {}", s.title(), s.place())))?;
            if !s.shown.is_empty() {
                writeln!(out, "{}", clean(&format!("    {}", s.shown)))?;
            }
            writeln!(out, "{}", clean(&format!("    {}", s.why)))?;
        }
    }
    Ok(())
}

/* ------------------------------------------------------------- compact -- */

fn write_compact(a: &View, out: &mut impl Write, err: &mut impl Write) -> io::Result<()> {
    writeln!(out, "{COMPACT_HEADER}")?;
    if !a.configured {
        // Nothing to list, and nothing the columns could say: the sentence
        // goes where a warning does.
        writeln!(err, "warning: no sources in {}", jobs::SOURCES_FILE)?;
        return Ok(());
    }
    // A row, not a warning: it changes what the rows below it mean, and a
    // client that keeps stdout alone must still see it.
    if let Some(note) = a.note() {
        writeln!(out, "not_read\t{}\t\t", field(&note))?;
        if a.none_read() {
            return Ok(());
        }
    }
    for (list, _, rows) in a.nodes() {
        for r in rows {
            writeln!(out, "{list}\t{}\t{}\t", field(&r.name), field(&r.folder))?;
        }
    }
    for (list, _, steps) in a.steps() {
        for s in steps {
            writeln!(out, "{list}\t{}\t{}\t{}", field(&s.title()), field(&s.place()), field(&s.why))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::RawManifest;
    use crate::settings::JobSource;
    use serde_json::json;

    /// Jaffle Shop in miniature: tables, a view, an ephemeral and a seed.
    fn graph() -> Graph {
        let model = |name: &str, dir: &str, materialized: &str| {
            json!({
                "name": name, "resource_type": "model", "package_name": "jaffle_shop",
                "original_file_path": format!("models/{dir}/{name}.sql"),
                "fqn": ["jaffle_shop", dir, name], "config": { "materialized": materialized },
            })
        };
        let raw: RawManifest = serde_json::from_value(json!({
            "metadata": { "project_name": "jaffle_shop" },
            "nodes": {
                "model.jaffle_shop.stg_orders": model("stg_orders", "staging", "table"),
                "model.jaffle_shop.stg_customers": model("stg_customers", "staging", "table"),
                "model.jaffle_shop.int_orders": model("int_orders", "intermediate", "ephemeral"),
                "model.jaffle_shop.orders": model("orders", "marts", "table"),
                "model.jaffle_shop.customers": model("customers", "marts", "table"),
                "model.jaffle_shop.orders_summary": model("orders_summary", "marts", "table"),
                "seed.jaffle_shop.raw_orders": {
                    "name": "raw_orders", "resource_type": "seed", "package_name": "jaffle_shop",
                    "original_file_path": "seeds/raw_orders.csv", "fqn": ["jaffle_shop", "raw_orders"],
                    "config": { "materialized": "seed" },
                },
            },
            "parent_map": {
                "model.jaffle_shop.orders": ["model.jaffle_shop.int_orders"],
                "model.jaffle_shop.int_orders": ["model.jaffle_shop.stg_orders"],
                "model.jaffle_shop.stg_orders": ["seed.jaffle_shop.raw_orders"],
            },
        }))
        .unwrap();
        Graph::build(raw, Path::new("manifest.json"), 0, 0)
    }

    /// A project whose sources file names the folders of Airflow DAGs, and
    /// `files` the `(path, text)` written, in the order given.
    fn project(tag: &str, sources: &[&str], files: &[(&str, &str)]) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!("dbt-edith-coverage-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join(".dbt-edith")).unwrap();
        for (path, text) in files {
            let at = root.join(path);
            std::fs::create_dir_all(at.parent().unwrap()).unwrap();
            std::fs::write(at, text).unwrap();
        }
        let list: Vec<JobSource> =
            sources.iter().map(|dir| JobSource { kind: "airflow".to_string(), path: dir.to_string(), ..Default::default() }).collect();
        std::fs::write(root.join(".dbt-edith/jobs.yml"), jobs::sources_yaml(&list)).unwrap();
        root
    }

    fn ask(root: &Path, output: Output) -> (String, String, i32) {
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = job_coverage(root, &graph(), None, output, &mut out, &mut err).unwrap();
        (String::from_utf8(out).unwrap(), String::from_utf8(err).unwrap(), code)
    }

    const DAILY: &str = "dag_id: daily\nschedule: \"0 5 * * *\"\ntasks:\n  - task_id: marts\n    command: dbt run --select orders\n";
    const MANUAL: &str = "dag_id: manual\nschedule: null\ntasks:\n  - task_id: all\n    command: dbt run --select customers\n";
    const LATE: &str = "dag_id: late\nschedule: \"{{ var.value.cron }}\"\ntasks:\n  - task_id: all\n    command: dbt run --select stg_customers\n";
    const NOTHING: &str = "dag_id: nothing\nschedule: \"0 6 * * *\"\ntasks:\n  - task_id: none\n    command: dbt run --select tag:no_such_tag\n";

    /// Every list of the answer holds something: orders runs on a clock,
    /// customers only in a job nothing starts, stg_customers only in one
    /// whose schedule is a template, raw_orders, stg_orders and
    /// orders_summary in no job, int_orders is ephemeral, and one step
    /// selects nothing.
    fn every_list(tag: &str) -> std::path::PathBuf {
        project(tag, &["dags"], &[("dags/daily.yml", DAILY), ("dags/manual.yml", MANUAL), ("dags/late.yml", LATE), ("dags/nothing.yml", NOTHING)])
    }

    #[test]
    fn the_text_says_the_pages_sentence_then_each_list_in_the_pages_order() {
        let (out, err, code) = ask(&every_list("text"), Output::Name);
        assert_eq!((code, err.as_str()), (ANSWERED, ""));
        assert_eq!(
            out,
            "1 of 6 models, seeds and snapshots run in a scheduled job; 1 only in jobs whose schedule is not read, 1 only in unscheduled jobs, 3 in no job.\n\
             run by no job  3\n\
             \x20 models/marts/orders_summary\n\
             \x20 models/staging/stg_orders\n\
             \x20 seeds/raw_orders\n\
             only in unscheduled jobs  1\n\
             \x20 models/marts/customers\n\
             only in jobs whose schedule is not read  1\n\
             \x20 models/staging/stg_customers\n\
             ephemeral, so never run on their own  1\n\
             \x20 models/intermediate/int_orders\n\
             steps that select nothing they act on  1\n\
             \x20 nothing › none  dags · nothing.yml:5\n\
             \x20   dbt run --select tag:no_such_tag\n\
             \x20   nothing matches `tag:no_such_tag`\n"
        );
    }

    #[test]
    fn json_is_the_coverage_without_the_pages_own_rows() {
        let (out, _, code) = ask(&every_list("json"), Output::Json);
        assert_eq!(code, ANSWERED);
        assert_eq!(out.lines().count(), 1, "{out}");
        let json: serde_json::Value = serde_json::from_str(&out).unwrap();
        let keys: Vec<&str> = json.as_object().unwrap().keys().map(String::as_str).collect();
        assert_eq!(keys, ["configured", "empty", "ephemeral", "never", "total", "unread", "unresolved", "unscheduled"]);
        assert_eq!((json["configured"].clone(), json["total"].clone()), (json!(true), json!(6)));
        assert_eq!(json["never"][0], json!({"id": "model.jaffle_shop.orders_summary", "name": "orders_summary", "kind": "model", "folder": "models/marts"}));
        assert_eq!(json["never"][2]["kind"], "seed");
        let step = &json["empty"][0];
        let keys: Vec<&str> = step.as_object().unwrap().keys().map(String::as_str).collect();
        assert_eq!(keys, ["file", "job", "line", "n", "name", "shown", "source", "why"]);
        assert_eq!((step["job"].clone(), step["name"].clone(), step["n"].clone()), (json!("nothing"), json!("none"), json!(1)));
        // The route the page reads is not touched: it still sends both.
        let cov = serde_json::to_value(jobs::coverage(&graph(), &read_of(&every_list("route")))).unwrap();
        assert!(cov.get("jobs").is_some() && cov["empty"][0].get("index").is_some());
    }

    fn read_of(root: &Path) -> std::sync::Arc<jobs::Read> {
        match jobs::known(&graph(), root) {
            Known::Read(read) => read,
            _ => panic!("the sources file is not read"),
        }
    }

    #[test]
    fn compact_is_one_tab_separated_line_per_row_under_a_header() {
        let (out, _, code) = ask(&every_list("compact"), Output::Compact);
        assert_eq!(code, ANSWERED);
        assert_eq!(
            out,
            "list\tname\twhere\twhy\n\
             never\torders_summary\tmodels/marts\t\n\
             never\tstg_orders\tmodels/staging\t\n\
             never\traw_orders\tseeds\t\n\
             unscheduled\tcustomers\tmodels/marts\t\n\
             unread\tstg_customers\tmodels/staging\t\n\
             ephemeral\tint_orders\tmodels/intermediate\t\n\
             empty\tnothing › none\tdags · nothing.yml:5\tnothing matches `tag:no_such_tag`\n"
        );
        assert!(out.lines().all(|l| l.split('\t').count() == 4), "{out}");
    }

    #[test]
    fn a_list_with_nothing_in_it_is_not_printed_and_a_covered_project_is_one_sentence() {
        let all = "dag_id: all\nschedule: \"0 5 * * *\"\ntasks:\n  - task_id: t\n    command: dbt build\n";
        let root = project("covered", &["dags"], &[("dags/all.yml", all)]);
        let (out, _, code) = ask(&root, Output::Name);
        assert_eq!(code, ANSWERED);
        // The ephemeral list is the project's, not the jobs'.
        assert_eq!(
            out,
            "6 of 6 models, seeds and snapshots run in a scheduled job.\nephemeral, so never run on their own  1\n  models/intermediate/int_orders\n"
        );
        let (compact, _, _) = ask(&root, Output::Compact);
        assert_eq!(compact, "list\tname\twhere\twhy\nephemeral\tint_orders\tmodels/intermediate\t\n");
    }

    #[test]
    fn the_sentence_is_the_pages() {
        let view = |total, never, unscheduled, unread| {
            let rows = |n: usize| {
                (0..n).map(|i| Row { id: format!("model.p.m{i}"), name: format!("m{i}"), kind: "model", folder: String::new() }).collect::<Vec<_>>()
            };
            View::of(
                Coverage { configured: true, total, never: rows(never), unscheduled: rows(unscheduled), unread: rows(unread), ..Default::default() },
                None,
                0,
                false,
            )
        };
        assert_eq!(view(4, 0, 0, 0).line(), "4 of 4 models, seeds and snapshots run in a scheduled job.");
        assert_eq!(view(10, 3, 2, 1).line(), "4 of 10 models, seeds and snapshots run in a scheduled job; 1 only in jobs whose schedule is not read, 2 only in unscheduled jobs, 3 in no job.");
        assert_eq!(view(0, 0, 0, 0).line(), "0 of 0 models, seeds and snapshots run in a scheduled job.");
    }

    #[test]
    fn the_same_files_give_the_same_answer_whatever_order_they_were_written_in() {
        let files = [("dags/daily.yml", DAILY), ("dags/manual.yml", MANUAL), ("dags/late.yml", LATE), ("dags/nothing.yml", NOTHING)];
        let mut backwards = files;
        backwards.reverse();
        let (a, b) = (project("order-a", &["dags"], &files), project("order-b", &["dags"], &backwards));
        for output in [Output::Name, Output::Json, Output::Compact] {
            assert_eq!(ask(&a, output).0, ask(&a, output).0);
            assert_eq!(ask(&a, output).0, ask(&b, output).0, "{output:?}");
        }
        // Two steps that select nothing come out in the order the files are walked.
        let other = NOTHING.replace("nothing", "again").replace("tag:no_such_tag", "tag:none_either");
        let (c, d) = (
            project("order-c", &["dags"], &[("dags/a.yml", NOTHING), ("dags/b.yml", &other)]),
            project("order-d", &["dags"], &[("dags/b.yml", &other), ("dags/a.yml", NOTHING)]),
        );
        let (text, _, _) = ask(&c, Output::Name);
        assert!(text.contains("  nothing › none") && text.find("nothing › none") < text.find("again › none"), "{text}");
        assert_eq!(text, ask(&d, Output::Name).0);
    }

    #[test]
    fn a_step_that_cannot_be_resolved_is_listed_with_its_command() {
        let bad = "dag_id: odd\nschedule: \"0 6 * * *\"\ntasks:\n  - task_id: run\n    command: dbt run --select \"{{ var('x') }}\"\n";
        let (out, _, code) = ask(&project("unresolved", &["dags"], &[("dags/odd.yml", bad)]), Output::Name);
        assert_eq!(code, ANSWERED);
        assert!(out.contains("steps not resolved  1\n  odd › run  dags · odd.yml:5\n    dbt run …\n"), "{out}");
    }

    #[test]
    fn a_source_that_was_not_read_is_said_right_under_the_sentence() {
        let root = project("unread-source", &["dags", "missing"], &[("dags/daily.yml", DAILY)]);
        let (out, err, code) = ask(&root, Output::Name);
        assert_eq!((code, err.as_str()), (ANSWERED, ""));
        let mut lines = out.lines();
        assert!(lines.next().unwrap().starts_with("1 of 6 models"), "{out}");
        assert_eq!(lines.next(), Some("not read: 1 of 2 sources (nothing at this path)"), "{out}");
        assert_eq!(lines.next(), Some("run by no job  5"), "{out}");
        assert_eq!(out.matches("not read:").count(), 1, "{out}");
        let json: serde_json::Value = serde_json::from_str(&ask(&root, Output::Json).0).unwrap();
        assert_eq!(json["not_read"], json!({"sources": 1, "of": 2, "reason": "nothing at this path"}));
        // Compact has the note as a row of its own, first, and nothing on stderr.
        let (out, err, _) = ask(&root, Output::Compact);
        assert_eq!(err, "");
        let mut lines = out.lines();
        assert_eq!(lines.next(), Some("list\tname\twhere\twhy"));
        assert_eq!(lines.next(), Some("not_read\t1 of 2 sources (nothing at this path)\t\t"), "{out}");
        assert!(lines.next().unwrap().starts_with("never\t"), "{out}");
        assert!(out.lines().all(|l| l.split('\t').count() == 4), "{out}");
    }

    #[test]
    fn when_no_source_was_read_nothing_is_listed_and_the_note_is_the_answer() {
        let root = project("none-read", &["missing", "gone"], &[]);
        let (out, err, code) = ask(&root, Output::Name);
        assert_eq!((code, err.as_str()), (ANSWERED, ""));
        assert_eq!(out, "not read: all 2 sources (nothing at this path)\n");
        let (out, err, _) = ask(&root, Output::Compact);
        assert_eq!(err, "");
        assert_eq!(out, "list\tname\twhere\twhy\nnot_read\tall 2 sources (nothing at this path)\t\t\n");
        // JSON lists nothing either, as with no source, and keeps what the project holds.
        let json: serde_json::Value = serde_json::from_str(&ask(&root, Output::Json).0).unwrap();
        assert_eq!(json["not_read"], json!({"sources": 2, "of": 2, "reason": "nothing at this path"}));
        assert_eq!((json["configured"].clone(), json["total"].clone(), json["never"].clone()), (json!(true), json!(6), json!([])));
        assert_eq!(json["ephemeral"][0]["name"], "int_orders");
    }

    #[test]
    fn a_job_file_refused_whole_is_counted_and_said_with_the_lists() {
        // A tab in the indentation: YAML does not allow it, and the file is refused whole.
        let tabbed = "dag_id: tabbed\nschedule: \"0 5 * * *\"\ntasks:\n\t- task_id: t\n\t  command: dbt run --select customers\n";
        let root = project("refused", &["dags"], &[("dags/daily.yml", DAILY), ("dags/tabbed.yml", tabbed)]);
        let (out, _, code) = ask(&root, Output::Name);
        assert_eq!(code, ANSWERED);
        let mut lines = out.lines();
        assert!(lines.next().unwrap().starts_with("1 of 6 models"), "{out}");
        assert_eq!(lines.next(), Some("not read: 1 job file (refused)"), "{out}");
        assert!(out.contains("  models/marts/customers\n"), "its model is listed, so the note is what says why");
        let (out, _, _) = ask(&root, Output::Compact);
        assert_eq!(out.lines().nth(1), Some("not_read\t1 job file (refused)\t\t"), "{out}");
        let json: serde_json::Value = serde_json::from_str(&ask(&root, Output::Json).0).unwrap();
        assert_eq!((json["files_refused"].clone(), json.get("not_read").cloned()), (json!(1), None));
        // Left out when none was refused, and joined to a source that was not read.
        let json: serde_json::Value = serde_json::from_str(&ask(&every_list("refused-none"), Output::Json).0).unwrap();
        assert!(json.get("files_refused").is_none());
        let both = project("refused-both", &["dags", "missing"], &[("dags/daily.yml", DAILY), ("dags/tabbed.yml", tabbed)]);
        let (out, _, _) = ask(&both, Output::Name);
        assert_eq!(out.lines().nth(1), Some("not read: 1 of 2 sources (nothing at this path); 1 job file (refused)"), "{out}");
    }

    #[test]
    fn a_source_cut_at_its_bound_is_a_partial_read_and_its_files_are_listed() {
        // One level more than the walk goes down: the folder is cut, the files above it are read.
        let deep = format!("dags/{}/deep.yml", ["d"; 18].join("/"));
        let root = project("capped", &["dags"], &[("dags/daily.yml", DAILY), (&deep, NOTHING)]);
        let (out, _, code) = ask(&root, Output::Name);
        assert_eq!(code, ANSWERED);
        let mut lines = out.lines();
        assert!(lines.next().unwrap().starts_with("1 of 6 models"), "{out}");
        assert_eq!(lines.next(), Some("not read: the source (stopped at its bound on files or depth)"), "{out}");
        assert_eq!(lines.next(), Some("run by no job  5"), "{out}");
        let (out, _, _) = ask(&root, Output::Compact);
        assert!(out.contains("not_read\tthe source (stopped at its bound on files or depth)\t\t\nnever\t"), "{out}");
        let json: serde_json::Value = serde_json::from_str(&ask(&root, Output::Json).0).unwrap();
        assert_eq!(json["never"].as_array().unwrap().len(), 5);
    }

    #[test]
    fn with_no_source_there_is_no_list_and_a_sentence_says_so() {
        let root = project("no-source", &[], &[]);
        let (out, err, code) = ask(&root, Output::Name);
        assert_eq!((code, out.as_str(), err.as_str()), (ANSWERED, "no sources in .dbt-edith/jobs.yml\n", ""));
        // Compact keeps its header, and the sentence goes to stderr.
        let (out, err, code) = ask(&root, Output::Compact);
        assert_eq!((code, out.as_str(), err.as_str()), (ANSWERED, "list\tname\twhere\twhy\n", "warning: no sources in .dbt-edith/jobs.yml\n"));
        // What the project holds is true whatever the jobs: kept.
        let json: serde_json::Value = serde_json::from_str(&ask(&root, Output::Json).0).unwrap();
        assert_eq!((json["configured"].clone(), json["total"].clone(), json["never"].clone()), (json!(false), json!(6), json!([])));
        assert_eq!(json["ephemeral"][0]["name"], "int_orders");
    }

    #[test]
    fn without_a_sources_file_or_with_one_that_does_not_read_nothing_can_be_asked() {
        let none = std::env::temp_dir().join(format!("dbt-edith-coverage-none-{}", std::process::id()));
        std::fs::create_dir_all(&none).unwrap();
        for output in [Output::Name, Output::Json, Output::Compact] {
            let (out, err, code) = ask(&none, output);
            assert_eq!((out.as_str(), code), ("", CANNOT_RUN));
            assert!(err.starts_with("error: no .dbt-edith/jobs.yml in this project; name the sources in the Jobs settings\n"), "{err}");
        }
        let root = project("broken", &["dags"], &[]);
        std::fs::write(root.join(".dbt-edith/jobs.yml"), "jobs: nope\n").unwrap();
        let (out, err, code) = ask(&root, Output::Name);
        assert_eq!((out.as_str(), code), ("", CANNOT_RUN));
        assert!(err.starts_with("error: .dbt-edith/jobs.yml is not read, line 1: `jobs:` is not a list of sources\n"), "{err}");
    }

    #[test]
    fn a_control_character_of_a_job_file_never_reaches_the_terminal() {
        let odd = NOTHING.replace("task_id: none", "task_id: \"no\\x1b[31mne\"");
        let root = project("escape", &["dags"], &[("dags/nothing.yml", &odd)]);
        for output in [Output::Name, Output::Compact] {
            let (out, _, _) = ask(&root, output);
            assert!(out.contains("[31mne") && !out.contains('\u{1b}'), "{out:?}");
        }
    }
}
