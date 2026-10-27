//! The `--job` mode: one DAG or dbt Cloud job, step by step, with the nodes
//! each step runs and the order its files declare, printed, and the process
//! ends (0115, 0124).
//!
//! Invariants: the jobs are the ones the project's own `.dbt-edith/jobs.yml`
//! names, read by `jobs::known`, so the command line answers as the Jobs tab
//! does over the same files, and no word of the line becomes a path. A project
//! without that file is not a question this mode can answer, and says so in
//! the exit code the modes keep for what could not be asked. stdout carries the
//! answer and nothing else, as the other modes keep it (`ls::provenance` goes
//! to stderr).

use crate::graph::Graph;
use crate::jobs::{self, Known};
use crate::ls::{self, Output, ANSWERED, CANNOT_RUN, REFUSED};
use std::io::{self, BufWriter, Write};
use std::path::Path;

const NO_NAME: &str = "error: --job wants the id of a DAG or the name of a dbt Cloud job";

/// The whole mode, from the command line's arguments to the exit code.
pub fn run_job(project: &Path, manifest: Option<&Path>, catalog: Option<&Path>, name: &str, output: Output) -> i32 {
    if let Some(why) = no_name(name) {
        eprintln!("{why}");
        return REFUSED;
    }
    let (root, _, graph) = match ls::open_with_catalog(project, manifest, catalog) {
        Ok(opened) => opened,
        Err(why) => return ls::cannot_open(&why),
    };
    let mut err = io::stderr().lock();
    let mut out = BufWriter::new(io::stdout().lock());
    let written = respond_job(&root, &graph, None, name, output, &mut out, &mut err);
    ls::ended(written, &mut err)
}

/// Why nothing can be asked, when no name was given. Before the project is
/// opened: reading a manifest to say so is a wait for nothing.
pub(crate) fn no_name(name: &str) -> Option<&'static str> {
    name.trim().is_empty().then_some(NO_NAME)
}

/// What the mode does once the project is open. `kept` is the jobs a caller
/// holds between calls (MCP); None reads them here, if the project has a
/// sources file.
pub(crate) fn respond_job(
    root: &Path,
    graph: &Graph,
    kept: Option<&Known>,
    name: &str,
    output: Output,
    out: &mut impl Write,
    err: &mut impl Write,
) -> io::Result<i32> {
    let code = job(root, graph, kept, name.trim(), output, out, err)?;
    out.flush()?;
    let _ = ls::provenance(root, graph, err);
    Ok(code)
}

fn job(root: &Path, graph: &Graph, kept: Option<&Known>, name: &str, output: Output, out: &mut impl Write, err: &mut impl Write) -> io::Result<i32> {
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
    let found = jobs::named(&read, name);
    let one = match found.as_slice() {
        [one] => *one,
        [] => {
            out.flush()?;
            let mut why = jobs::unknown(&read, name);
            if let Some(u) = jobs::unread(&read) {
                why.push_str(&format!("; {} not read", u.text()));
            }
            writeln!(err, "error: {}", clean(&why))?;
            return Ok(REFUSED);
        }
        many => {
            out.flush()?;
            // The source, since two sources may hold a job of one name.
            let listed: Vec<String> = many.iter().map(|&i| jobs::listed_as(&read, i)).collect();
            writeln!(err, "error: {} names {} jobs: {}", clean(name), many.len(), clean(&listed.join(", ")))?;
            return Ok(REFUSED);
        }
    };
    let answer = jobs::answer(graph, &read, one);
    match output {
        Output::Json => {
            serde_json::to_writer(&mut *out, &answer)?;
            writeln!(out)?;
        }
        // A card is read, so compact has nothing to drop from it: both print
        // the text.
        Output::Name | Output::Compact => write_text(&answer, out)?,
    }
    Ok(ANSWERED)
}

/// A line with every control character but the tab left out: the names and
/// commands are the project's text, and an escape sequence in one would be
/// the terminal's to run, not the reader's to see.
fn clean(text: &str) -> String {
    text.chars().filter(|&c| c == '\t' || !c.is_control()).collect()
}

fn write_text(a: &jobs::JobAnswer, out: &mut impl Write) -> io::Result<()> {
    let head = if a.kind == jobs::AIRFLOW { format!("DAG {}", a.name) } else { format!("job {}", a.name) };
    let ident = if a.ident.is_empty() { String::new() } else { format!(" [[{}]]", a.ident) };
    writeln!(out, "{}", clean(&format!("{head}{ident}  {}  {}:{}", a.schedule, a.file, a.line)))?;
    for line in a.order_lines() {
        writeln!(out, "{}", clean(&line))?;
    }
    for s in &a.steps {
        let mut line = format!("  {}  {}", s.label, s.command);
        if !s.unresolved.is_empty() {
            line.push_str(&format!("  not resolved: {}", s.unresolved));
        } else if !s.runs_nothing.is_empty() {
            line.push_str(&format!("  ({})", s.runs_nothing));
        } else {
            line.push_str(&format!("  ({})", if s.node_count == 1 { "1 node".to_string() } else { format!("{} nodes", s.node_count) }));
        }
        writeln!(out, "{}", clean(&line))?;
        // Marked, so it never reads as a node line (`    kind name`) below.
        let order = s.order();
        if !order.is_empty() {
            writeln!(out, "{}", clean(&format!("    order: {order}")))?;
        }
        for w in &s.warnings {
            writeln!(out, "{}", clean(&format!("    note: {w}")))?;
        }
        for n in &s.nodes {
            writeln!(out, "{}", clean(&format!("    {} {}", n.kind, n.name)))?;
        }
    }
    writeln!(out, "nodes {}", a.nodes)?;
    if let Some(u) = &a.not_read {
        writeln!(out, "not read: {}", clean(&u.text()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::RawManifest;
    use crate::settings::JobSource;
    use serde_json::json;

    /// Jaffle Shop in miniature: two staging models, two marts and a seed.
    fn graph() -> Graph {
        let model = |name: &str, dir: &str, tags: &[&str]| {
            json!({
                "name": name, "resource_type": "model", "package_name": "jaffle_shop",
                "original_file_path": format!("models/{dir}/{name}.sql"),
                "fqn": ["jaffle_shop", dir, name], "tags": tags, "config": { "materialized": "table" },
            })
        };
        let raw: RawManifest = serde_json::from_value(json!({
            "metadata": { "project_name": "jaffle_shop" },
            "nodes": {
                "model.jaffle_shop.stg_orders": model("stg_orders", "staging", &["staging"]),
                "model.jaffle_shop.stg_customers": model("stg_customers", "staging", &["staging"]),
                "model.jaffle_shop.orders": model("orders", "marts", &[]),
                "model.jaffle_shop.customers": model("customers", "marts", &[]),
                "seed.jaffle_shop.raw_orders": {
                    "name": "raw_orders", "resource_type": "seed", "package_name": "jaffle_shop",
                    "original_file_path": "seeds/raw_orders.csv", "fqn": ["jaffle_shop", "raw_orders"],
                },
            },
            "sources": {
                "source.jaffle_shop.raw.payments": {
                    "name": "payments", "resource_type": "source", "package_name": "jaffle_shop",
                    "source_name": "raw", "original_file_path": "models/staging/sources.yml",
                    "fqn": ["jaffle_shop", "staging", "raw", "payments"],
                },
            },
            "parent_map": {
                "model.jaffle_shop.stg_orders": ["seed.jaffle_shop.raw_orders"],
                "model.jaffle_shop.orders": ["model.jaffle_shop.stg_orders", "model.jaffle_shop.customers"],
                "model.jaffle_shop.customers": ["model.jaffle_shop.stg_customers"],
            },
        }))
        .unwrap();
        Graph::build(raw, Path::new("manifest.json"), 0, 0)
    }

    /// A project folder whose sources file names the job fixtures, once per
    /// `(kind, folder, label)`.
    fn project(tag: &str, sources: &[(&str, &str, &str)]) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!("dbt-edith-job-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join(".dbt-edith")).unwrap();
        // Inside the project: its file names paths from the project only.
        let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures").join("jobs");
        jobs::copy_tree(&fixtures, &root.join("jobs"));
        let list: Vec<JobSource> = sources
            .iter()
            .map(|(kind, dir, label)| JobSource {
                kind: kind.to_string(),
                path: format!("jobs/{dir}"),
                label: label.to_string(),
                // The fixture names its own keys.
                keys: if *dir == "airflow_renamed" { jobs::RENAMED_KEYS.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect() } else { Default::default() },
                ..Default::default()
            })
            .collect();
        std::fs::write(root.join(".dbt-edith/jobs.yml"), jobs::sources_yaml(&list)).unwrap();
        root
    }

    fn ask(root: &Path, name: &str, output: Output) -> (String, String, i32) {
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = job(root, &graph(), None, name, output, &mut out, &mut err).unwrap();
        (String::from_utf8(out).unwrap(), String::from_utf8(err).unwrap(), code)
    }

    #[test]
    fn a_dag_is_printed_step_by_step_with_the_nodes_each_step_runs() {
        let root = project("dag", &[("airflow", "airflow", ""), ("dbt_cloud", "dbt_cloud", "")]);
        let (out, err, code) = ask(&root, "jaffle_daily", Output::Name);
        assert_eq!((code, err.as_str()), (ANSWERED, ""));
        assert_eq!(
            out,
            "DAG jaffle_daily  scheduled 0 6 * * *  jaffle_daily.yml:2\n\
             \x20 task load_seeds  dbt seed  (1 node)\n\
             \x20   order: before stage\n\
             \x20   seed raw_orders\n\
             \x20 task stage  dbt run --select tag:staging  (2 nodes)\n\
             \x20   order: after load_seeds · before marts\n\
             \x20   model stg_customers\n\
             \x20   model stg_orders\n\
             \x20 task marts  dbt run -s +customers orders --exclude tag:staging  (2 nodes)\n\
             \x20   order: after stage · before reconcile · updates jaffle_marts, which starts jaffle_on_marts\n\
             \x20   model customers\n\
             \x20   model orders\n\
             \x20 task reconcile  dbt test --selector recon_tests  not resolved: no selector named `recon_tests` in this manifest; \
             the Selectors menu lists the ones it records\n\
             \x20   order: after marts\n\
             nodes 5\n"
        );
        // Compact reads as the text does.
        assert_eq!(ask(&root, "jaffle_daily", Output::Compact).0, out);
    }

    #[test]
    fn a_dbt_cloud_job_is_found_by_its_name_its_case_or_its_identifier() {
        let root = project("cloud", &[("dbt_cloud", "dbt_cloud", "")]);
        let (out, _, code) = ask(&root, "Nightly build", Output::Name);
        assert_eq!(code, ANSWERED);
        let head = out.lines().next().unwrap();
        assert_eq!(head, "job Nightly build [[nightly]]  scheduled 0 2 * * *  jobs.yml:22");
        assert!(out.contains("  step 1/3  dbt seed  (1 node)\n"), "{out}");
        assert!(out.contains("  step 2/3  dbt build --select +orders --exclude-resource-type test  ("), "{out}");
        for other in ["NIGHTLY BUILD", "nightly"] {
            assert_eq!(ask(&root, other, Output::Name).0, out, "{other}");
        }
        let (json, _, _) = ask(&root, "Nightly build", Output::Json);
        let json: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!((json["kind"].as_str(), json["ident"].as_str(), json["schedule"].as_str()), (Some("dbt_cloud"), Some("nightly"), Some("scheduled 0 2 * * *")));
        assert_eq!(json["steps"][0]["node_count"], 1);
        assert_eq!(json["steps"][0]["nodes"][0], json!({"id": "seed.jaffle_shop.raw_orders", "kind": "seed", "name": "raw_orders"}));
        assert_eq!(json["nodes"], 5, "distinct: the seed is run by two steps and counted once");
    }

    #[test]
    fn a_step_that_compiles_runs_no_node_and_a_step_that_selects_nothing_says_zero() {
        let root = project("compile", &[("airflow", "airflow", "")]);
        let (out, _, _) = ask(&root, "jaffle_on_marts", Output::Name);
        assert!(out.contains("  task compile_all  dbt compile  (dbt compile runs no node)\n"), "{out}");
        assert!(!out.contains("(0 nodes)\n    model"), "a compile step lists no node: {out}");
    }

    #[test]
    fn several_jobs_of_a_name_are_refused_with_their_sources_and_an_unknown_one_with_the_nearest() {
        let root = project("many", &[("airflow", "airflow", "nightly"), ("airflow", "airflow", "copy")]);
        let (out, err, code) = ask(&root, "jaffle_daily", Output::Name);
        assert_eq!((out.as_str(), code), ("", REFUSED));
        assert!(err.starts_with("error: jaffle_daily names 2 jobs: nightly: jaffle_daily, copy: jaffle_daily\n"), "{err}");
        let (out, err, code) = ask(&root, "jaffle", Output::Name);
        assert_eq!((out.as_str(), code), ("", REFUSED));
        assert!(err.starts_with("error: no job named jaffle; did you mean jaffle_daily, jaffle_on_marts\n"), "{err}");
        let (_, err, _) = ask(&root, "nothing_like_it", Output::Name);
        assert!(err.starts_with("error: no job named nothing_like_it; the jobs read include jaffle_daily, jaffle_on_marts\n"), "{err}");
        // A task is no job.
        assert_eq!(ask(&root, "marts", Output::Name).2, REFUSED);
    }

    #[test]
    fn without_a_sources_file_or_with_one_that_does_not_read_nothing_can_be_asked() {
        let none = std::env::temp_dir().join(format!("dbt-edith-job-none-{}", std::process::id()));
        std::fs::create_dir_all(&none).unwrap();
        let (out, err, code) = ask(&none, "jaffle_daily", Output::Name);
        assert_eq!((out.as_str(), code), ("", CANNOT_RUN));
        assert!(err.starts_with("error: no .dbt-edith/jobs.yml in this project; name the sources in the Jobs settings\n"), "{err}");
        let root = project("broken", &[("airflow", "airflow", "")]);
        std::fs::write(root.join(".dbt-edith/jobs.yml"), "jobs: nope\n").unwrap();
        let (_, err, code) = ask(&root, "jaffle_daily", Output::Name);
        assert_eq!(code, CANNOT_RUN);
        assert!(err.starts_with("error: .dbt-edith/jobs.yml is not read, line 1: `jobs:` is not a list of sources\n"), "{err}");
        assert_eq!(no_name("  "), Some(NO_NAME));
        assert_eq!(no_name("jaffle_daily"), None);
    }

    #[test]
    fn a_job_answer_says_which_sources_were_not_read() {
        let root = project("unread", &[("airflow", "airflow", "")]);
        std::fs::write(
            root.join(".dbt-edith/jobs.yml"),
            "jobs:\n  - kind: airflow\n    path: jobs/airflow\n  - kind: airflow\n    path: jobs/missing\n",
        )
        .unwrap();
        let (out, _, code) = ask(&root, "jaffle_daily", Output::Name);
        assert_eq!(code, ANSWERED);
        assert!(out.ends_with("nodes 5\nnot read: 1 of 2 sources (nothing at this path)\n"), "{out}");
        let json: serde_json::Value = serde_json::from_str(&ask(&root, "jaffle_daily", Output::Json).0).unwrap();
        assert_eq!(json["not_read"], json!({"sources": 1, "of": 2, "reason": "nothing at this path"}));
        // Not found, and a source unread: the refusal says it could be there.
        let (_, err, code) = ask(&root, "nothing_like_it", Output::Name);
        assert_eq!(code, REFUSED);
        assert!(err.contains("; 1 of 2 sources (nothing at this path) not read\n"), "{err}");
    }

    /// A DAG of its own beside the fixtures: a task that names a sensor in
    /// `depends_on`, a whole-run sensor, a sensor with no id, and a task with
    /// no dbt command that updates an asset another DAG is scheduled on.
    fn with_gated_dag(root: &Path) {
        let dir = root.join("jobs").join("gated");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("jaffle_gated.yml"),
            "dag_id: jaffle_gated\nschedule: null\nsensors:\n  - task_id: wait_stage\n    external_dag_id: jaffle_daily\n    external_task_id: stage\n  \
             - task_id: wait_all\n    external_dag_id: jaffle_daily\n  - external_dag_id: jaffle_daily\n    external_task_id: marts\n\
             tasks:\n  - task_id: publish\n    command: dbt run -s customers\n    depends_on: [wait_stage, load_raw]\n  \
             - task_id: load_raw\n    python_callable: load\n    outlets: [raw_loaded]\n  \
             - task_id: load_more\n    python_callable: load\n    outlets: [raw_loaded]\n",
        )
        .unwrap();
        std::fs::write(dir.join("jaffle_next.yml"), "dag_id: jaffle_next\nschedule: [raw_loaded]\ntasks:\n  - task_id: build\n    command: dbt run -s orders\n").unwrap();
        let list = [JobSource { kind: "airflow".into(), path: "jobs/airflow".into(), ..Default::default() }, JobSource { kind: "airflow".into(), path: "jobs/gated".into(), ..Default::default() }];
        std::fs::write(root.join(".dbt-edith/jobs.yml"), jobs::sources_yaml(&list)).unwrap();
    }

    #[test]
    fn a_dag_says_what_its_sensors_wait_for_and_a_task_what_awaits_it() {
        let root = project("sensors", &[("airflow", "airflow", ""), ("airflow", "airflow_renamed", "")]);
        let (out, _, _) = ask(&root, "jaffle_reports", Output::Name);
        assert!(out.starts_with("DAG jaffle_reports  manual  jaffle_reports.yml:4\n  sensor wait_marts  waits for jaffle_daily › marts\n  task refresh  "), "{out}");
        assert!(!out.contains("order:"), "no task of it names another: {out}");
        let (out, _, _) = ask(&root, "jaffle_daily", Output::Name);
        assert!(out.contains("    order: after stage · before reconcile · awaited by jaffle_reports › wait_marts · updates jaffle_marts, which starts jaffle_on_marts\n"), "{out}");
    }

    #[test]
    fn a_task_names_its_sensor_a_whole_run_sensor_holds_every_task_and_a_nameless_one_is_the_dag() {
        let root = project("gated", &[]);
        with_gated_dag(&root);
        let (out, err, code) = ask(&root, "jaffle_gated", Output::Name);
        assert_eq!((code, err.as_str()), (ANSWERED, ""));
        assert!(
            out.starts_with(
                "DAG jaffle_gated  manual  jaffle_gated.yml:1\n\
                 \x20 sensor wait_stage  waits for jaffle_daily › stage\n\
                 \x20 sensor wait_all  waits for jaffle_daily\n\
                 \x20 sensor waits for jaffle_daily › marts\n\
                 \x20 updates raw_loaded, which starts jaffle_next\n\
                 \x20 task publish  dbt run -s customers  (1 node)\n\
                 \x20   order: after wait_stage, load_raw\n"
            ),
            "{out}"
        );
        let (out, _, _) = ask(&root, "jaffle_daily", Output::Name);
        assert!(out.contains("    order: before stage · awaited by jaffle_gated › wait_all (whole run)\n"), "{out}");
        assert!(out.contains("    order: after load_seeds · before marts · awaited by jaffle_gated › wait_stage, jaffle_gated › wait_all (whole run)\n"), "{out}");
        // The nameless sensor is the DAG alone, with no separator after it.
        assert!(out.contains(" · awaited by jaffle_gated › wait_all (whole run), jaffle_gated · updates jaffle_marts"), "{out}");

        let json: serde_json::Value = serde_json::from_str(&ask(&root, "jaffle_gated", Output::Json).0).unwrap();
        assert_eq!(
            json["sensors"],
            json!([
                {"id": "wait_stage", "dag": "jaffle_daily", "task": "stage"},
                {"id": "wait_all", "dag": "jaffle_daily"},
                {"dag": "jaffle_daily", "task": "marts"},
            ])
        );
        assert_eq!((&json["outlets"], &json["feeds"]), (&json!(["raw_loaded"]), &json!(["jaffle_next"])));
        assert_eq!(json["steps"][0]["upstream"], json!(["wait_stage", "load_raw"]));
        let json: serde_json::Value = serde_json::from_str(&ask(&root, "jaffle_daily", Output::Json).0).unwrap();
        assert_eq!(json["steps"][2]["waited_by"], json!(["jaffle_gated › wait_all (whole run)", "jaffle_gated"]));
    }

    #[test]
    fn a_job_answer_in_json_carries_the_order_and_leaves_out_what_is_empty() {
        let root = project("order-json", &[("airflow", "airflow", ""), ("airflow", "airflow_renamed", "")]);
        let json: serde_json::Value = serde_json::from_str(&ask(&root, "jaffle_daily", Output::Json).0).unwrap();
        let steps = &json["steps"];
        assert!(steps[0].get("upstream").is_none(), "load_seeds waits for nothing: {steps}");
        assert_eq!(steps[0]["downstream"], json!(["stage"]));
        assert_eq!((&steps[1]["upstream"], &steps[1]["downstream"]), (&json!(["load_seeds"]), &json!(["marts"])));
        assert_eq!(steps[2]["waited_by"], json!(["jaffle_reports › wait_marts"]));
        assert_eq!((&steps[2]["outlets"], &steps[2]["feeds"]), (&json!(["jaffle_marts"]), &json!(["jaffle_on_marts"])));
        // The notify task has no dbt command, so it is no step and no `before`.
        assert_eq!(steps[3]["upstream"], json!(["marts"]));
        assert!(steps[3].get("downstream").is_none(), "{}", steps[3]);
        for absent in ["sensors", "assets", "assets_unread", "after", "after_unread", "triggers", "outlets", "feeds"] {
            assert!(json.get(absent).is_none(), "{absent}: {json}");
        }
        let json: serde_json::Value = serde_json::from_str(&ask(&root, "jaffle_on_marts", Output::Json).0).unwrap();
        assert_eq!(json["assets"], json!(["jaffle_marts"]));
        let json: serde_json::Value = serde_json::from_str(&ask(&root, "jaffle_reports", Output::Json).0).unwrap();
        assert_eq!(json["sensors"], json!([{"id": "wait_marts", "dag": "jaffle_daily", "task": "marts"}]));
    }

    #[test]
    fn a_sensor_on_several_tasks_or_a_group_is_worded_as_such_in_the_text_and_the_json() {
        let root = project("sensor-tasks", &[]);
        with_gated_dag(&root);
        std::fs::write(
            root.join("jobs/gated/jaffle_gated.yml"),
            "dag_id: jaffle_gated\nschedule: null\nsensors:\n  - task_id: both\n    external_dag_id: jaffle_daily\n    external_task_ids: [stage, marts]\n  \
             - task_id: grp\n    external_dag_id: jaffle_daily\n    external_task_group_id: loading\n  \
             - task_id: one\n    external_dag_id: jaffle_daily\n    external_task_ids: [stage]\n\
             tasks:\n  - task_id: publish\n    command: dbt run -s customers\n",
        )
        .unwrap();
        let (out, _, _) = ask(&root, "jaffle_gated", Output::Name);
        assert!(
            out.contains(
                "  sensor both  waits for jaffle_daily › stage, marts\n\
                 \x20 sensor grp  waits for jaffle_daily › task group loading, whose tasks no file lists\n\
                 \x20 sensor one  waits for jaffle_daily › stage\n"
            ),
            "{out}"
        );
        let json: serde_json::Value = serde_json::from_str(&ask(&root, "jaffle_gated", Output::Json).0).unwrap();
        assert_eq!(
            json["sensors"],
            json!([
                {"id": "both", "dag": "jaffle_daily", "tasks": ["stage", "marts"]},
                {"id": "grp", "dag": "jaffle_daily", "task_group": "loading"},
                {"id": "one", "dag": "jaffle_daily", "task": "stage"},
            ])
        );
        let (out, _, _) = ask(&root, "jaffle_daily", Output::Name);
        assert!(out.contains("order: after load_seeds · before marts · awaited by jaffle_gated › both, jaffle_gated › one\n"), "{out}");
        assert!(!out.contains("whole run") && !out.contains("jaffle_gated › grp"), "{out}");
    }

    #[test]
    fn what_no_file_read_defines_or_updates_is_flagged_beside_its_value_in_the_json() {
        let root = project("unread-json", &[]);
        let dir = root.join("jobs/loose");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("orphan.yml"), "dag_id: jaffle_orphan\nschedule: [never_updated]\ntasks:\n  - task_id: build\n    command: dbt run -s orders\n").unwrap();
        std::fs::write(
            dir.join("jobs.yml"),
            "jobs:\n  lone:\n    name: Lone\n    execute_steps:\n      - dbt seed\n    job_completion_trigger_condition:\n      condition:\n        job_id: 4242\n",
        )
        .unwrap();
        let list = [
            JobSource { kind: "airflow".into(), path: "jobs/loose".into(), ..Default::default() },
            JobSource { kind: "dbt_cloud".into(), path: "jobs/loose".into(), ..Default::default() },
        ];
        std::fs::write(root.join(".dbt-edith/jobs.yml"), jobs::sources_yaml(&list)).unwrap();
        let json: serde_json::Value = serde_json::from_str(&ask(&root, "jaffle_orphan", Output::Json).0).unwrap();
        assert_eq!((&json["assets"], &json["assets_unread"]), (&json!(["never_updated"]), &json!(["never_updated"])));
        let json: serde_json::Value = serde_json::from_str(&ask(&root, "Lone", Output::Json).0).unwrap();
        assert_eq!((&json["after"], &json["after_unread"]), (&json!("4242"), &json!(true)));
    }

    #[test]
    fn a_dbt_cloud_job_says_which_jobs_its_end_starts() {
        let root = project("starts", &[("dbt_cloud", "dbt_cloud", "")]);
        let (out, _, _) = ask(&root, "Nightly build", Output::Name);
        assert_eq!(out.lines().nth(1), Some("  starts Exports when it ends"), "{out}");
        // The started job's head says it, the order lines do not say it twice.
        let (out, _, _) = ask(&root, "Exports", Output::Name);
        assert!(out.starts_with("job Exports [[exports]]  scheduled after Nightly build  "), "{out}");
        assert!(!out.contains("starts") && !out.contains("order:"), "{out}");
        let json: serde_json::Value = serde_json::from_str(&ask(&root, "Nightly build", Output::Json).0).unwrap();
        assert_eq!(json["triggers"], json!(["Exports"]));
        let json: serde_json::Value = serde_json::from_str(&ask(&root, "Exports", Output::Json).0).unwrap();
        assert_eq!(json["after"], "Nightly build");
        assert!(json.get("triggers").is_none());
    }
}
