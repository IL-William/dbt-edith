//! Which Python environment is in play, for the status bar.
//!
//! Reports the one this process was launched with, and otherwise the ones
//! sitting in the project, so the answer is never a bare "none" when there is
//! something obvious to activate.

use std::path::{Path, PathBuf};

#[derive(serde::Serialize, Default, Clone)]
pub struct VenvInfo {
    /// "activated" when $VIRTUAL_ENV was set, "project" when merely found on disk.
    pub source: String,
    pub name: String,
    pub path: String,
    pub python: String,
    pub dbt: String,
    /// Other environments found next to the project, none of them active.
    pub others: Vec<String>,
    /// Environments found in the project that git tracks, so came with the
    /// repository: listed, never run, and never the one reported (0053).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub committed: Vec<String>,
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

/// Packages that come with dbt-core rather than being chosen: naming them
/// beside the adapter says nothing the user picked.
const DBT_PARTS: [&str; 6] = ["dbt_common", "dbt_adapters", "dbt_extractor", "dbt_semantic_interfaces", "dbt_protos", "dbt_core_interface"];

/// dbt-core's version and the adapters beside it, read from the names of their
/// `.dist-info` folders: `dbt_core-1.8.7.dist-info` says what `dbt --version`
/// would, without running anything the project put there.
fn dbt_versions(venv: &Path) -> String {
    let mut found: Vec<(String, String)> = Vec::new();
    for site in site_packages(venv) {
        let Ok(entries) = std::fs::read_dir(&site) else { continue };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let Some(stem) = name.strip_suffix(".dist-info") else { continue };
            let Some((package, version)) = stem.split_once('-') else { continue };
            let package = package.to_ascii_lowercase().replace('-', "_");
            if package.starts_with("dbt_") && !DBT_PARTS.contains(&package.as_str()) {
                found.push((package.replace('_', "-"), version.to_string()));
            }
        }
    }
    // dbt-core first, then the adapters by name.
    found.sort_by_key(|(name, _)| (name != "dbt-core", name.clone()));
    found.dedup();
    found.iter().map(|(name, version)| format!("{name} {version}")).collect::<Vec<_>>().join(", ")
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
    info.name = venv.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    info.path = venv.display().to_string();
    // Read, never run: opening a project must not execute a program it holds
    // before anyone has asked for anything (0053).
    info.python = python_version(&venv);
    info.dbt = dbt_versions(&venv);
    info.others = found
        .iter()
        .filter(|p| **p != venv)
        .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .collect();
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
        for package in packages {
            std::fs::create_dir_all(site.join(format!("{package}.dist-info"))).unwrap();
        }
        dir
    }

    #[test]
    fn a_venv_is_read_from_disk_and_nothing_in_it_runs() {
        let root = project("read");
        venv(&root, ".venv", "home = /usr/bin\nversion = 3.12.4\n", &[
            "dbt_core-1.8.7", "dbt_snowflake-1.8.3", "dbt_common-1.10.0", "dbt_adapters-1.7.0", "requests-2.32.3",
        ]);
        let info = detect_with(&root, None);
        assert_eq!(info.name, ".venv");
        assert_eq!(info.source, "project");
        assert_eq!(info.python, "3.12.4");
        assert_eq!(info.dbt, "dbt-core 1.8.7, dbt-snowflake 1.8.3", "dbt-core, then the adapter, and none of its parts");
        assert!(!root.join(".venv-ran").exists(), "opening a project ran a program it holds");
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
        assert_eq!(info.name, "mine", "first by name, but set aside since git tracks it");
        assert!(info.others.is_empty(), "{:?}", info.others);
        let programs = crate::sidecar::interpreters(&root, &info);
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
