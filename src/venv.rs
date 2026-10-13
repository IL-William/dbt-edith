//! Which Python environment is in play, for the status bar.
//!
//! Reports the one this process was launched with, and otherwise the ones
//! sitting in the project, so the answer is never a bare "none" when there is
//! something obvious to activate. Each one found is described too, so the page
//! can name whichever the terminal says it activated later (0069).

use std::path::{Path, PathBuf};

#[derive(serde::Serialize, Default, Clone)]
pub struct VenvInfo {
    /// "activated" when $VIRTUAL_ENV was set, "project" when merely found on disk.
    pub source: String,
    /// The one reported: the venv this process was started in, else the best
    /// one found. Empty when there is neither.
    #[serde(flatten)]
    pub chosen: Venv,
    /// Environments found in the project that git tracks, so came with the
    /// repository: listed, never run, and never the one reported (0053).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub committed: Vec<String>,
    /// Every environment found in the project that git does not track, the
    /// reported one included, best first: what the status bar can name when the
    /// terminal reports one of them (0069).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub found: Vec<Venv>,
}

/// One environment, as read from its files.
#[derive(serde::Serialize, Default, Clone, PartialEq, Debug)]
pub struct Venv {
    pub name: String,
    pub path: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub python: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub dbt: String,
    /// The `VIRTUAL_ENV` its bash activate script sets, so the spelling the
    /// terminal reports: `path` comes from the canonical project root, which a
    /// mapped drive, a `subst` or a symlink spells otherwise (0069).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub activates: String,
    /// The line that activates it in the terminal's shell, which the status bar
    /// types when clicked and leaves Enter to the user (0072). Empty when that
    /// shell has no script in it.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub activation: String,
}

impl Venv {
    /// Read, never run: opening a project must not execute a program it holds
    /// before anyone has asked for anything (0053).
    fn read(dir: &Path) -> Venv {
        Venv {
            name: dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
            path: dir.display().to_string(),
            python: python_version(dir),
            dbt: dbt_versions(dir),
            activates: activates(dir),
            activation: String::new(),
        }
    }
}

impl VenvInfo {
    /// Each venv with the line that activates it in `shell`: the one the
    /// terminal types for the venv dbt-edith was started in (0050), so a click
    /// types what startup would have.
    pub fn with_activation(mut self, shell: &crate::pty::ShellSpec) -> VenvInfo {
        for v in std::iter::once(&mut self.chosen).chain(self.found.iter_mut()) {
            if !v.path.is_empty() {
                v.activation = crate::pty::activation(shell, Path::new(&v.path)).unwrap_or_default();
            }
        }
        self
    }
}

/// The folder a venv's bash activate script sets `VIRTUAL_ENV` to, when it
/// writes it out: the path the venv was made at, as venv, virtualenv and uv
/// all do, inside `cygpath` on Windows. A script that works it out at run
/// time, as a relocatable venv's does, gives nothing.
fn activates(venv: &Path) -> String {
    let script = venv.join(if cfg!(windows) { "Scripts" } else { "bin" }).join("activate");
    let Ok(text) = std::fs::read_to_string(script) else {
        return String::new();
    };
    text.lines()
        .filter_map(|line| {
            let line = line.trim();
            let value = line.strip_prefix("export ").unwrap_or(line).strip_prefix("VIRTUAL_ENV=")?;
            let value = match value.strip_prefix("$(cygpath ") {
                Some(inner) => inner.strip_suffix(')')?.trim_start_matches("-u ").trim(),
                None => value.trim(),
            };
            let value = ['"', '\''].iter().find_map(|q| value.strip_prefix(*q)?.strip_suffix(*q)).unwrap_or(value);
            // Anything left to expand, or a quote left over, is not a path written out.
            (!value.is_empty() && !value.contains(['$', '`', '"', '\''])).then(|| value.to_string())
        })
        .next()
        .unwrap_or_default()
}

fn bin(venv: &Path, exe: &str) -> PathBuf {
    if cfg!(windows) {
        venv.join("Scripts").join(format!("{exe}.exe"))
    } else {
        venv.join("bin").join(exe)
    }
}

/// The Python version a venv was made with, as `pyvenv.cfg` records it:
/// `version` from the venv module, `version_info` from virtualenv and uv, which
/// may run on to `3.12.4.final.0`.
fn python_version(venv: &Path) -> String {
    let Ok(cfg) = std::fs::read_to_string(venv.join("pyvenv.cfg")) else {
        return String::new();
    };
    let value = |wanted: &str| {
        cfg.lines().find_map(|line| {
            let (key, value) = line.split_once('=')?;
            (key.trim() == wanted).then(|| value.trim().to_string())
        })
    };
    let raw = value("version").or_else(|| value("version_info")).unwrap_or_default();
    raw.split('.').take(3).collect::<Vec<_>>().join(".")
}

/// Every `site-packages` of a venv: `Lib\site-packages` on Windows,
/// `lib/pythonX.Y/site-packages` elsewhere.
fn site_packages(venv: &Path) -> Vec<PathBuf> {
    let mut out = vec![venv.join("Lib").join("site-packages")];
    if let Ok(entries) = std::fs::read_dir(venv.join("lib")) {
        out.extend(entries.flatten().map(|e| e.path().join("site-packages")));
    }
    out.retain(|p| p.is_dir());
    out
}

/// Which dbt a venv holds, and its adapters, read from the names of their
/// `.dist-info` folders: `dbt_core-1.8.7.dist-info` says what `dbt --version`
/// would, without running anything the project put there. dbt Fusion is the
/// package PyPI calls plain `dbt`, from version 2 on.
fn dbt_versions(venv: &Path) -> String {
    let (mut fusion, mut core, mut adapters) = (None, None, Vec::new());
    for site in site_packages(venv) {
        let Ok(entries) = std::fs::read_dir(&site) else { continue };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let Some(stem) = name.strip_suffix(".dist-info") else { continue };
            let Some((package, version)) = stem.split_once('-') else { continue };
            match package.to_ascii_lowercase().replace('-', "_").as_str() {
                "dbt" => fusion = Some(version.to_string()),
                "dbt_core" => core = Some(version.to_string()),
                // The base every adapter builds on, not one the user picked.
                "dbt_adapters" => {}
                p if p.starts_with("dbt_") && is_adapter(&entry.path()) => {
                    adapters.push(format!("{} {version}", p.replace('_', "-")));
                }
                _ => {}
            }
        }
    }
    adapters.sort();
    adapters.dedup();
    let fusion = fusion.map(|v| {
        let major: u32 = v.split('.').next().and_then(|m| m.parse().ok()).unwrap_or(0);
        // Before Fusion, `dbt` on PyPI was dbt-core's old umbrella package.
        if major >= 2 { format!("dbt-fusion {v}") } else { format!("dbt {v}") }
    });
    fusion.into_iter().chain(core.map(|v| format!("dbt-core {v}"))).chain(adapters).collect::<Vec<_>>().join(", ")
}

/// An adapter installs its module under `dbt/adapters/`, which its `RECORD`
/// lists. dbt's other packages, the experimental parser, `dbt-autofix` or
/// Fusion's package tools, install nothing there, so their names stay out of
/// the line that says which dbt runs.
fn is_adapter(dist_info: &Path) -> bool {
    std::fs::read_to_string(dist_info.join("RECORD")).is_ok_and(|record| record.lines().any(|l| l.starts_with("dbt/adapters/")))
}

/// Whether git tracks this venv, which no venv anyone made for themselves is:
/// one that arrived with a clone was put there by whoever wrote the repository,
/// and its `python` is theirs to have written too. Asked of git rather than
/// guessed, and false outside a repository, where there is nobody to ask.
fn committed(root: &Path, venv: &Path) -> bool {
    let Ok(rel) = venv.strip_prefix(root) else {
        return false;
    };
    let rel = rel.to_string_lossy().replace('\\', "/");
    if rel.is_empty() {
        return false;
    }
    let listed = crate::git::run(root, &["ls-files", "--", &rel], &[], std::time::Duration::from_secs(10));
    listed.ok && !listed.stdout.trim().is_empty()
}

/// The interpreter inside a virtual environment.
pub fn python_in(venv: &Path) -> PathBuf {
    bin(venv, "python")
}

/// Whether the Snowflake connector is installed, looked up on disk so that
/// nothing has to be started to find out.
pub fn has_snowflake_connector(venv: &Path) -> bool {
    let installed = |site: PathBuf| site.join("snowflake").join("connector").is_dir();
    // Windows: Lib\site-packages. Elsewhere: lib/pythonX.Y/site-packages.
    installed(venv.join("Lib").join("site-packages"))
        || std::fs::read_dir(venv.join("lib"))
            .map(|entries| entries.flatten().any(|e| installed(e.path().join("site-packages"))))
            .unwrap_or(false)
}

/// The virtual environment this process was started in, if it still exists.
pub fn activated() -> Option<PathBuf> {
    std::env::var("VIRTUAL_ENV").ok().map(PathBuf::from).filter(|p| p.exists())
}

fn is_venv(p: &Path) -> bool {
    p.join("pyvenv.cfg").exists() || bin(p, "python").exists()
}

/// What the status bar reports, from the disk alone: nothing found here is
/// started, since `pyvenv.cfg` and the package folders already say which
/// Python and which dbt a venv holds.
pub fn detect(root: &Path) -> VenvInfo {
    detect_with(root, activated())
}

fn detect_with(root: &Path, active: Option<PathBuf>) -> VenvInfo {
    let mut info = VenvInfo::default();

    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(root) {
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() && is_venv(&p) {
                found.push(p);
            }
        }
    }
    // A committed venv is named so its absence is explained, and offered to
    // nothing that would run it: not the status bar, not the Snowflake script.
    let (committed, kept): (Vec<PathBuf>, Vec<PathBuf>) = found.into_iter().partition(|p| committed(root, p));
    info.committed = committed.iter().filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned())).collect();
    let mut found = kept;
    // A bare venv with no dbt in it is the least useful answer, so rank those last.
    found.sort_by_key(|p| (!bin(p, "dbt").exists(), p.clone()));

    let chosen = match &active {
        Some(p) => Some(p.clone()),
        None => found.first().cloned(),
    };

    let Some(venv) = chosen else {
        return info;
    };
    info.source = if active.is_some() { "activated".into() } else { "project".into() };
    info.found = found.iter().map(|p| Venv::read(p)).collect();
    // Read once when it is also one of the project's.
    info.chosen = match found.iter().position(|p| *p == venv) {
        Some(i) => info.found[i].clone(),
        None => Venv::read(&venv),
    };
    info
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dbt-edith-venv-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir.canonicalize().unwrap()
    }

    /// A venv whose `python` and `dbt` leave a mark when they run, so a test
    /// can tell that nothing ran them.
    fn venv(root: &Path, name: &str, cfg: &str, packages: &[&str]) -> PathBuf {
        let dir = root.join(name);
        std::fs::create_dir_all(dir.join("bin")).unwrap();
        std::fs::create_dir_all(dir.join("Scripts")).unwrap();
        std::fs::write(dir.join("pyvenv.cfg"), cfg).unwrap();
        let mark = root.join(format!("{name}-ran"));
        for exe in ["python", "dbt", "python.exe", "dbt.exe"] {
            for folder in ["bin", "Scripts"] {
                let path = dir.join(folder).join(exe);
                std::fs::write(&path, format!("#!/bin/sh\ntouch '{}'\n", mark.display())).unwrap();
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
                }
            }
        }
        let site = dir.join("lib").join("python3.12").join("site-packages");
        // `adapter:` marks a package that installs under dbt/adapters, as its
        // RECORD then says; every other one installs its own module.
        for package in packages {
            let (adapter, package) = match package.strip_prefix("adapter:") {
                Some(p) => (true, p),
                None => (false, *package),
            };
            let info = site.join(format!("{package}.dist-info"));
            std::fs::create_dir_all(&info).unwrap();
            let module = package.split('-').next().unwrap();
            let record = if adapter {
                format!("dbt/adapters/{}/__init__.py,,\n", module.trim_start_matches("dbt_"))
            } else {
                format!("{module}/__init__.py,,\n")
            };
            std::fs::write(info.join("RECORD"), record).unwrap();
        }
        dir
    }

    #[test]
    fn a_venv_is_read_from_disk_and_nothing_in_it_runs() {
        let root = project("read");
        venv(&root, ".venv", "home = /usr/bin\nversion = 3.12.4\n", &[
            "dbt_core-1.8.7", "adapter:dbt_snowflake-1.8.3", "dbt_common-1.10.0", "adapter:dbt_adapters-1.7.0", "requests-2.32.3",
            "dbt_core_experimental_parser-2.0.5", "dbt_extractor-0.6.0",
        ]);
        let info = detect_with(&root, None);
        assert_eq!(info.chosen.name, ".venv");
        assert_eq!(info.source, "project");
        assert_eq!(info.chosen.python, "3.12.4");
        assert_eq!(info.chosen.dbt, "dbt-core 1.8.7, dbt-snowflake 1.8.3", "dbt-core, then the adapter, and none of its parts");
        assert!(!root.join(".venv-ran").exists(), "opening a project ran a program it holds");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The status bar types what startup types (0072): the line for the
    /// terminal's shell, on every venv with a script for it, and none on one
    /// without or for a shell with no way to source one.
    #[test]
    fn each_venv_carries_the_line_that_activates_it() {
        let root = project("activation");
        let scripted = venv(&root, "core-env", "version = 3.12.4\n", &[]);
        venv(&root, "bare", "version = 3.12.4\n", &[]);
        for folder in ["bin", "Scripts"] {
            std::fs::write(scripted.join(folder).join("activate"), "").unwrap();
        }
        let shell = |program: &str| crate::pty::ShellSpec { program: program.into(), args: Vec::new() };
        let line = crate::pty::activation(&shell("bash"), &scripted).unwrap();
        let info = detect_with(&root, None).with_activation(&shell("bash"));
        let lines: Vec<(&str, &str)> = info.found.iter().map(|v| (v.name.as_str(), v.activation.as_str())).collect();
        assert_eq!(lines, [("bare", ""), ("core-env", line.as_str())]);
        assert_eq!(info.chosen.activation, "", "the one reported is the first found, and has no script");

        let started = detect_with(&root, Some(scripted.clone())).with_activation(&shell("bash"));
        assert_eq!(started.chosen.activation, line, "the one dbt-edith was started in, once deactivated");
        let sh = detect_with(&root, None).with_activation(&shell("sh"));
        assert!(sh.found.iter().all(|v| v.activation.is_empty()));
        assert!(!root.join("core-env-ran").exists() && !root.join("bare-ran").exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    /// dbt Fusion installs from PyPI as plain `dbt`, beside tools whose names
    /// start with `dbt` too. `dbt --version` said `dbt-fusion 2.0.0-preview.196`
    /// for the venv below; its package is what says so now.
    #[test]
    fn fusion_is_named_and_the_tools_beside_it_are_not() {
        let root = project("fusion");
        let dir = venv(&root, "fusion-env", "version = 3.12.13\n", &[
            "dbt-2.0.0rc196", "dbt_autofix-0.21.1", "dbt_fusion_package_tools-0.21.1", "dbt_protos-1.0.541",
        ]);
        assert_eq!(dbt_versions(&dir), "dbt-fusion 2.0.0rc196");
        let old = venv(&root, "old-env", "version = 3.8.10\n", &["dbt-0.18.1", "adapter:dbt_postgres-0.18.1"]);
        assert_eq!(dbt_versions(&old), "dbt 0.18.1, dbt-postgres 0.18.1");
        let none = venv(&root, "plain", "version = 3.12.1\n", &["requests-2.32.3"]);
        assert_eq!(dbt_versions(&none), "");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Whichever venv the terminal reports later, the page can name it with
    /// its versions, read the same way as the one reported at startup (0069).
    #[test]
    fn every_venv_found_is_described_for_the_terminal_to_name() {
        let root = project("found");
        venv(&root, "core-env", "version = 3.12.4\n", &["dbt_core-1.8.7", "adapter:dbt_snowflake-1.8.3"]);
        venv(&root, "fusion-env", "version = 3.12.13\n", &["dbt-2.0.0rc196"]);
        venv(&root, "bare", "version = 3.11.9\n", &[]);
        std::fs::remove_file(root.join("bare/bin/dbt")).unwrap();
        std::fs::remove_file(root.join("bare/Scripts/dbt.exe")).unwrap();

        let info = detect_with(&root, None);
        let found: Vec<(&str, &str, &str)> =
            info.found.iter().map(|v| (v.name.as_str(), v.python.as_str(), v.dbt.as_str())).collect();
        assert_eq!(found, [
            ("core-env", "3.12.4", "dbt-core 1.8.7, dbt-snowflake 1.8.3"),
            ("fusion-env", "3.12.13", "dbt-fusion 2.0.0rc196"),
            ("bare", "3.11.9", ""),
        ], "in the order the status bar ranks them");
        assert_eq!(info.chosen, info.found[0], "the best one found, read once");
        assert_eq!(info.found[1].path, root.join("fusion-env").display().to_string());

        // One started elsewhere is the one reported, and the project's are still all there.
        let elsewhere = project("found-elsewhere");
        let started = venv(&elsewhere, "mine", "version = 3.13.0\n", &[]);
        let info = detect_with(&root, Some(started));
        assert_eq!((info.source.as_str(), info.chosen.name.as_str(), info.chosen.python.as_str()), ("activated", "mine", "3.13.0"));
        assert_eq!(info.found.len(), 3);
        assert!(!root.join("core-env-ran").exists() && !root.join("fusion-env-ran").exists());
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&elsewhere);
    }

    /// The terminal reports the folder the activate script sets, which is the
    /// one the venv was made at and may be spelled otherwise than the
    /// canonical project root: read from the script, never by running it.
    #[test]
    fn the_folder_an_activate_script_sets_is_read_from_it() {
        let root = project("activates");
        let dir = venv(&root, "env", "version = 3.12.4\n", &[]);
        let script = dir.join(if cfg!(windows) { "Scripts" } else { "bin" }).join("activate");
        for (label, body, want) in [
            ("venv, 3.12.13", "case \"$(uname)\" in\n    MINGW*)\n        VIRTUAL_ENV=$(cygpath /h/proj/env)\n        export VIRTUAL_ENV\n        ;;\n    *)\n        export VIRTUAL_ENV=/h/proj/env\n", "/h/proj/env"),
            ("venv, 3.12 on Windows", "    export VIRTUAL_ENV=$(cygpath \"H:\\proj\\env\")\nelse\n    export VIRTUAL_ENV=\"H:\\proj\\env\"\n", r"H:\proj\env"),
            ("venv, 3.11", "VIRTUAL_ENV=\"C:\\Users\\me\\proj\\env\"\nexport VIRTUAL_ENV\n", r"C:\Users\me\proj\env"),
            ("virtualenv and uv", "VIRTUAL_ENV='/Users/John Doe/proj/env'\nif [ \"$OSTYPE\" = msys ]; then\n    VIRTUAL_ENV=$(cygpath -u \"$VIRTUAL_ENV\")\nfi\n", "/Users/John Doe/proj/env"),
            ("relocatable", "VIRTUAL_ENV=''\"$(dirname -- \"$(dirname -- \"$SCRIPT_PATH\")\")\"''\n", ""),
            ("quoted by shlex", "export VIRTUAL_ENV='/tmp/it'\"'\"'s/env'\n", ""),
        ] {
            std::fs::write(&script, body).unwrap();
            assert_eq!(activates(&dir), want, "{label}");
        }
        std::fs::remove_file(&script).unwrap();
        assert_eq!(activates(&dir), "", "no script");
        assert!(!root.join("env-ran").exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_python_version_is_read_whichever_tool_made_the_venv() {
        let root = project("cfg");
        for (name, cfg, want) in [
            ("venv", "version = 3.11.9\n", "3.11.9"),
            ("virtualenv", "version_info = 3.10.14.final.0\n", "3.10.14"),
            ("uv", "home = /x\nimplementation = CPython\nversion_info = 3.13.0\n", "3.13.0"),
            ("none", "home = /x\n", ""),
        ] {
            let dir = venv(&root, name, cfg, &[]);
            assert_eq!(python_version(&dir), want, "{name}");
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A venv that came with the clone was written by whoever wrote the
    /// repository. It is named, never chosen, and never offered to anything
    /// that would run its `python` (0053).
    #[test]
    fn a_venv_git_tracks_is_named_and_never_chosen() {
        let root = project("committed");
        let git = |args: &[&str]| crate::git::run(&root, args, &[], std::time::Duration::from_secs(20));
        assert!(git(&["init", "-q"]).ok);
        venv(&root, "shipped", "version = 3.12.4\n", &["dbt_core-1.8.7"]);
        venv(&root, "mine", "version = 3.12.1\n", &[]);
        assert!(git(&["add", "shipped"]).ok);

        let info = detect_with(&root, None);
        assert_eq!(info.committed, ["shipped"]);
        assert_eq!(info.chosen.name, "mine", "first by name, but set aside since git tracks it");
        let found: Vec<&str> = info.found.iter().map(|v| v.name.as_str()).collect();
        assert_eq!(found, ["mine"], "nor offered to the terminal's report");
        let programs = crate::sidecar::interpreters(&info);
        assert!(programs.iter().all(|i| !i.program.starts_with(root.join("shipped"))), "{programs:?}");
        assert!(!root.join("shipped-ran").exists());

        // Outside a repository there is nobody to ask, and nothing is set aside.
        let loose = project("loose");
        venv(&loose, "shipped", "version = 3.12.4\n", &[]);
        assert!(detect_with(&loose, None).committed.is_empty());
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&loose);
    }
}
