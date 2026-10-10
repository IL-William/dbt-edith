//! Terminal backend: a real PTY per WebSocket connection.
//!
//! `portable-pty` gives a ConPTY on Windows and a Unix98 PTY elsewhere, so the
//! same code drives zsh on macOS and Git Bash on the VM. The shell gets the
//! environment dbt-edith was started with, and the virtual environment active
//! then is activated again in it, so that venv is the one the terminal's `dbt`
//! runs from, whatever the shell's startup files put first.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, Weak};

type Master = Box<dyn portable_pty::MasterPty + Send>;

#[derive(Clone, Debug)]
pub struct ShellSpec {
    pub program: String,
    pub args: Vec<String>,
}

impl ShellSpec {
    /// Resolves the shell to spawn: an explicit `--shell` wins, then Git Bash on
    /// Windows / `$SHELL` elsewhere, then a platform fallback.
    pub fn detect(explicit: Option<String>) -> ShellSpec {
        if let Some(spec) = explicit.as_deref().and_then(parse_shell) {
            return spec;
        }
        #[cfg(windows)]
        {
            if let Some(bash) = find_git_bash() {
                return ShellSpec { program: bash.display().to_string(), args: vec!["-i".into()] };
            }
            return ShellSpec { program: "powershell.exe".into(), args: vec!["-NoLogo".into()] };
        }
        #[cfg(not(windows))]
        {
            let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into());
            ShellSpec { program: shell, args: vec!["-l".into()] }
        }
    }
}

/// Splits a `--shell` value into a program and its arguments.
///
/// Splitting on whitespace alone cannot express the usual Windows shell,
/// `C:\Program Files\Git\bin\bash.exe`. So a quoted program is honoured, and
/// an unquoted value naming an existing file is taken whole instead of split.
fn parse_shell(line: &str) -> Option<ShellSpec> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    if let Some(rest) = line.strip_prefix('"') {
        let (program, args) = rest.split_once('"').unwrap_or((rest, ""));
        return Some(ShellSpec {
            program: program.to_string(),
            args: args.split_whitespace().map(str::to_string).collect(),
        });
    }
    if Path::new(line).is_file() {
        return Some(ShellSpec { program: line.to_string(), args: Vec::new() });
    }
    let mut parts = line.split_whitespace().map(str::to_string);
    Some(ShellSpec { program: parts.next()?, args: parts.collect() })
}

#[cfg(windows)]
fn find_git_bash() -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    for var in ["ProgramFiles", "ProgramW6432", "ProgramFiles(x86)", "LOCALAPPDATA"] {
        if let Ok(base) = std::env::var(var) {
            candidates.push(Path::new(&base).join("Git").join("bin").join("bash.exe"));
            candidates.push(Path::new(&base).join("Programs").join("Git").join("bin").join("bash.exe"));
        }
    }
    candidates.into_iter().find(|p| p.exists())
}

#[cfg(not(windows))]
#[allow(dead_code)]
fn find_git_bash() -> Option<PathBuf> {
    let _ = Path::new("");
    None
}

/// The shell's command, in the project, with dbt-edith's own environment.
///
/// `portable-pty` does not simply inherit it: on Windows it overwrites what was
/// inherited with the variables in the registry. `PATH` then loses the
/// `Scripts` folder an activated venv put first, while `VIRTUAL_ENV`, which the
/// registry does not hold, survives. The shell looks activated, and `dbt` is
/// whichever one the machine's `PATH` finds, often a dbt-core without the
/// project's adapter. So the environment is cleared and copied from this
/// process, the way an editor's terminal inherits the editor's.
fn command(shell: &ShellSpec, cwd: &Path) -> portable_pty::CommandBuilder {
    let mut cmd = portable_pty::CommandBuilder::new(&shell.program);
    cmd.args(&shell.args);
    cmd.cwd(cwd);
    cmd.env_clear();
    for (key, value) in std::env::vars_os() {
        cmd.env(key, value);
    }
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");
    cmd.env("DBT_EDITH", "1");
    cmd
}

/// The line that activates `venv` in `shell`, typed into each new terminal.
///
/// The environment alone does not keep a venv first: the shell runs its
/// startup files again, and one that puts pyenv's shims or conda before `PATH`
/// puts another `dbt` in front of the venv's. Sourcing the venv's own script
/// once they have run is what an editor's terminal does: its folder leads
/// `PATH` again, and `deactivate` exists. Only the venv dbt-edith was started
/// in is activated, so the terminal still gets the environment it was handed
/// rather than one dbt-edith picked (0050). A shell the venv has no script for
/// gets nothing typed.
fn activation(shell: &ShellSpec, venv: &Path) -> Option<String> {
    let name = Path::new(&shell.program).file_stem()?.to_string_lossy().to_ascii_lowercase();
    let (script, line): (&str, fn(&str) -> String) = match name.as_str() {
        "bash" | "zsh" => ("activate", |p| format!("source '{}'", p.replace('\'', r"'\''"))),
        "fish" => ("activate.fish", |p| format!("source '{}'", p.replace('\\', r"\\").replace('\'', r"\'"))),
        "pwsh" | "powershell" => ("Activate.ps1", |p| format!("& '{}'", p.replace('\'', "''"))),
        _ => return None,
    };
    let path = venv.join(if cfg!(windows) { "Scripts" } else { "bin" }).join(script);
    if !path.is_file() {
        return None;
    }
    let path = path.display().to_string();
    // Git Bash takes `C:/...` for the Windows path it is, with no backslash to
    // wonder about inside the quotes. PowerShell keeps the native spelling.
    let path = if cfg!(windows) && script != "Activate.ps1" { path.replace('\\', "/") } else { path };
    Some(line(&path))
}

/// Whether the shell reads input through its line editor yet, the moment a
/// typed line shows once.
///
/// Before that a Unix terminal echoes what it is sent by itself, and the line
/// editor prints it again when it starts: the line would show above the prompt
/// and after it. zsh, bash and fish turn echo off at their prompt, so this waits
/// for that, for as long as startup files running pyenv or conda may take. A
/// shell that never does gets nothing typed.
#[cfg(unix)]
fn line_editor_ready(master: &Weak<Mutex<Master>>) -> bool {
    use std::time::{Duration, Instant};
    const ECHO: u64 = 0o10; // <termios.h>, the same bit on macOS and Linux
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        let termios = match master.upgrade() {
            Some(master) => master.lock().ok().and_then(|m| m.get_termios()),
            None => return false,
        };
        match termios {
            Some(t) if u64::from(t.local_flags.bits()) & ECHO == 0 => return true,
            Some(_) => std::thread::sleep(Duration::from_millis(50)),
            None => return false,
        }
    }
    false
}

/// A Windows console echoes input only when it is read, so a line queued at once
/// waits for the shell like a key pressed early, and shows once, after the
/// prompt its startup files lead to.
#[cfg(windows)]
fn line_editor_ready(_master: &Weak<Mutex<Master>>) -> bool {
    true
}

pub enum FromPty {
    Output(Vec<u8>),
    Exited,
}

pub struct PtySession {
    master: Arc<Mutex<Master>>,
    input: std::sync::mpsc::Sender<Vec<u8>>,
    child: Arc<Mutex<Box<dyn portable_pty::Child + Send + Sync>>>,
}

impl PtySession {
    pub fn spawn(
        shell: &ShellSpec,
        cwd: &Path,
        cols: u16,
        rows: u16,
        out: tokio::sync::mpsc::Sender<FromPty>,
    ) -> anyhow::Result<PtySession> {
        let activate = crate::venv::activated().and_then(|venv| activation(shell, &venv));
        Self::start(shell, cwd, cols, rows, out, activate)
    }

    fn start(
        shell: &ShellSpec,
        cwd: &Path,
        cols: u16,
        rows: u16,
        out: tokio::sync::mpsc::Sender<FromPty>,
        activate: Option<String>,
    ) -> anyhow::Result<PtySession> {
        let pty = portable_pty::native_pty_system();
        let pair = pty.openpty(portable_pty::PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })?;

        let child = pair.slave.spawn_command(command(shell, cwd))?;
        drop(pair.slave); // so the reader sees EOF when the shell exits

        let mut reader = pair.master.try_clone_reader()?;
        let out_reader = out.clone();
        std::thread::spawn(move || {
            let mut buf = [0u8; 8192];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if out_reader.blocking_send(FromPty::Output(buf[..n].to_vec())).is_err() {
                            break;
                        }
                    }
                }
            }
            let _ = out_reader.blocking_send(FromPty::Exited);
        });

        let mut writer = pair.master.take_writer()?;
        let master = Arc::new(Mutex::new(pair.master));
        let watched = Arc::downgrade(&master);
        let (tx, rx) = std::sync::mpsc::channel::<Vec<u8>>();
        std::thread::spawn(move || {
            // Whatever the page sends meanwhile, a key or a `dbt ls`, queues
            // behind the activation, so it runs with the venv first.
            if let Some(line) = activate {
                if line_editor_ready(&watched) {
                    let line = format!("{line}\r");
                    if writer.write_all(line.as_bytes()).is_err() || writer.flush().is_err() {
                        return;
                    }
                }
            }
            while let Ok(chunk) = rx.recv() {
                if writer.write_all(&chunk).is_err() || writer.flush().is_err() {
                    break;
                }
            }
        });

        Ok(PtySession { master, input: tx, child: Arc::new(Mutex::new(child)) })
    }

    pub fn write(&self, data: Vec<u8>) {
        let _ = self.input.send(data);
    }

    pub fn resize(&self, cols: u16, rows: u16) {
        if let Ok(master) = self.master.lock() {
            let _ = master.resize(portable_pty::PtySize { rows, cols, pixel_width: 0, pixel_height: 0 });
        }
    }

    pub fn kill(&self) {
        if let Ok(mut child) = self.child.lock() {
            let _ = child.kill();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn spec(line: &str) -> Option<(String, Vec<String>)> {
        parse_shell(line).map(|s| (s.program, s.args))
    }

    #[test]
    fn a_shell_and_its_flags_are_split() {
        assert_eq!(spec("zsh -l"), Some(("zsh".into(), vec!["-l".into()])));
        assert_eq!(spec("bash"), Some(("bash".into(), vec![])));
    }

    #[test]
    fn a_quoted_program_may_contain_spaces() {
        // The shape a Windows user needs: the Git Bash path holds a space.
        assert_eq!(
            spec(r#""C:\Program Files\Git\bin\bash.exe" -i"#),
            Some((r"C:\Program Files\Git\bin\bash.exe".into(), vec!["-i".into()])),
        );
        assert_eq!(
            spec(r#""C:\Program Files\Git\bin\bash.exe""#),
            Some((r"C:\Program Files\Git\bin\bash.exe".into(), vec![])),
        );
        // An unterminated quote still yields the path rather than nothing.
        assert_eq!(spec(r#""/opt/my shell"#), Some(("/opt/my shell".into(), vec![])));
    }

    #[test]
    fn an_unquoted_path_that_exists_is_not_split() {
        let dir = std::env::temp_dir().join(format!("dbt-edith-pty-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("my shell");
        std::fs::write(&path, "#!/bin/sh\n").unwrap();

        let line = path.display().to_string();
        assert_eq!(spec(&line), Some((line.clone(), vec![])));

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_shell_gets_this_process_environment_and_nothing_else() {
        let shell = ShellSpec { program: "sh".into(), args: vec!["-l".into()] };
        let cmd = command(&shell, Path::new("."));

        let mut expected: std::collections::BTreeMap<String, String> = std::env::vars().collect();
        expected.insert("TERM".into(), "xterm-256color".into());
        expected.insert("COLORTERM".into(), "truecolor".into());
        expected.insert("DBT_EDITH".into(), "1".into());
        let got: std::collections::BTreeMap<String, String> =
            cmd.iter_full_env_as_str().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        // Names only: the values may be secrets, and a failure prints this.
        let differ: std::collections::BTreeSet<&String> =
            expected.keys().chain(got.keys()).filter(|k| expected.get(*k) != got.get(*k)).collect();
        assert!(differ.is_empty(), "not as in this process: {differ:?}");
        assert_eq!(cmd.get_env("PATH"), std::env::var_os("PATH").as_deref());
    }

    /// A venv holding only the activate scripts named, in a folder of its own.
    fn fake_venv(name: &str, scripts: &[(&str, &str)]) -> PathBuf {
        let venv = std::env::temp_dir().join(format!("dbt-edith-venv-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&venv);
        let bin = venv.join(if cfg!(windows) { "Scripts" } else { "bin" });
        std::fs::create_dir_all(&bin).unwrap();
        for (script, body) in scripts {
            std::fs::write(bin.join(script), body).unwrap();
        }
        venv
    }

    fn shell(program: &str) -> ShellSpec {
        ShellSpec { program: program.into(), args: Vec::new() }
    }

    #[test]
    fn each_shell_sources_the_venv_script_written_for_it() {
        let venv = fake_venv("each", &[("activate", ""), ("activate.fish", ""), ("Activate.ps1", "")]);
        let bin = venv.join(if cfg!(windows) { "Scripts" } else { "bin" });
        let at = |script: &str| bin.join(script).display().to_string();

        let posix = format!("source '{}'", at("activate"));
        assert_eq!(activation(&shell("bash"), &venv), Some(posix.clone()));
        assert_eq!(activation(&shell("/bin/zsh"), &venv), Some(posix));
        assert_eq!(activation(&shell("fish"), &venv), Some(format!("source '{}'", at("activate.fish"))));
        assert_eq!(activation(&shell("pwsh"), &venv), Some(format!("& '{}'", at("Activate.ps1"))));
        // No script to source, or no knowing how: nothing is typed.
        assert_eq!(activation(&shell("sh"), &venv), None);
        assert_eq!(activation(&shell("nu"), &venv), None);
        std::fs::remove_file(bin.join("activate")).unwrap();
        assert_eq!(activation(&shell("bash"), &venv), None);

        std::fs::remove_dir_all(&venv).unwrap();
    }

    #[test]
    fn a_quote_in_the_venv_path_stays_inside_the_quotes() {
        let venv = fake_venv("it's", &[("activate", ""), ("Activate.ps1", "")]);
        let bash = activation(&shell("bash"), &venv).unwrap();
        assert!(bash.contains(r"-it'\''s/"), "{bash}");
        let pwsh = activation(&shell("pwsh"), &venv).unwrap();
        assert!(pwsh.contains("it''s"), "{pwsh}");
        std::fs::remove_dir_all(&venv).unwrap();
    }

    /// The whole path on a real terminal: the line is typed once bash's line
    /// editor reads, so it shows once and the venv's script runs.
    #[cfg(unix)]
    #[test]
    fn the_activation_runs_and_shows_once() {
        // The marker is computed, so the typed line cannot contain it. Unsetting
        // HISTFILE keeps the line out of the developer's own history.
        let venv = fake_venv("typed", &[("activate", "unset HISTFILE\necho venv-$((40 + 2))\n")]);
        let bash = ShellSpec {
            program: "bash".into(),
            args: vec!["--norc".into(), "--noprofile".into(), "-i".into()],
        };
        let line = activation(&bash, &venv).unwrap();
        let (tx, mut rx) = tokio::sync::mpsc::channel(512);
        // Wide enough for the line not to wrap, which would split it in the output.
        let session = PtySession::start(&bash, &venv, 400, 24, tx, Some(line.clone())).unwrap();

        let mut seen = String::new();
        let deadline = Instant::now() + Duration::from_secs(20);
        while !seen.contains("venv-42") && Instant::now() < deadline {
            match rx.try_recv() {
                Ok(FromPty::Output(bytes)) => seen.push_str(&String::from_utf8_lossy(&bytes)),
                Ok(FromPty::Exited) => break,
                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                    std::thread::sleep(Duration::from_millis(20))
                }
                Err(_) => break,
            }
        }
        session.kill();
        std::fs::remove_dir_all(&venv).unwrap();

        assert!(seen.contains("venv-42"), "the venv's script did not run: {seen:?}");
        assert_eq!(seen.matches(line.as_str()).count(), 1, "shown other than once: {seen:?}");
    }

    #[test]
    fn nothing_usable_falls_back_to_detection() {
        assert!(parse_shell("").is_none());
        assert!(parse_shell("   ").is_none());
    }
}
