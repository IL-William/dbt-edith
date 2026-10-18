//! Stamps the binary with the build it came from, and a Windows `.exe` with
//! who made it.
//!
//! The version in `Cargo.toml` only moves at a release, so between two tags it
//! cannot tell two builds apart. That is exactly the question asked after
//! reinstalling on the Windows VM, so `git describe` goes in as well, and a
//! build date for a source tree copied without its `.git`.
//!
//! Everything is composed here: `DBT_EDITH_BUILD` is the one string the rest of
//! the program reads, and it is never empty.
//!
//! The `.exe` also carries a version resource naming the publisher, the product
//! and the version, the properties an antivirus weighs on an unsigned file
//! (0093). It never stops a build: without it the binary runs the same.

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let mut parts = Vec::new();
    if let Some(described) = git_describe() {
        parts.push(described);
    }
    parts.push(format!("built {}", build_date()));
    println!("cargo:rustc-env=DBT_EDITH_BUILD={}", parts.join(", "));

    println!("cargo:rerun-if-changed=build.rs");
    // Only when there is a .git to watch: cargo treats a missing path as
    // changed, which would rebuild the crate on every single invocation.
    //
    // The consequence of watching anything at all is that a local `cargo build`
    // after editing only `src/` keeps the previous stamp, since none of these
    // moved. `cargo install`, which is how the VM updates, builds in a fresh
    // target directory every time and so always restamps.
    if Path::new(".git").is_dir() {
        println!("cargo:rerun-if-changed=.git/HEAD");
        println!("cargo:rerun-if-changed=.git/refs");
    }

    // The target's, not the host's: this script runs on the Mac that
    // cross-builds the `.exe` as well as on the Windows machine that builds it.
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        version_resource();
    }
}

/// Compiles the version resource with windres and links it into the `.exe`.
/// Only the GNU toolchain: its windres comes in the same binutils as the `as`
/// and `dlltool` that linking there already needs (docs/windows.md), where
/// MSVC's `rc.exe` is off `PATH` outside a developer prompt.
fn version_resource() {
    let skip = |why: String| {
        println!("cargo:warning=dbt-edith.exe is built without its version resource: {why} (0093)");
    };
    if env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("gnu") {
        return skip("only the GNU toolchain's windres is used".into());
    }
    let out = PathBuf::from(env::var("OUT_DIR").expect("cargo sets OUT_DIR"));
    std::fs::write(out.join("version.rc"), version_rc()).expect("OUT_DIR is writable");
    let arch = match env::var("CARGO_CFG_TARGET_ARCH").as_deref() {
        Ok("x86") => "i686".to_string(),
        Ok(arch) => arch.to_string(),
        Err(_) => "x86_64".to_string(),
    };
    // Prefixed as Homebrew and Linux packages name it, bare as WinLibs does.
    let tools = [format!("{arch}-w64-mingw32-windres"), "windres".to_string()];
    let mut why = format!("neither {} is on PATH", tools.join(" nor "));
    for tool in &tools {
        // Relative to OUT_DIR, so the command windres hands its preprocessor
        // through a shell holds no folder a user named, spaces and all.
        let run = Command::new(tool)
            .current_dir(&out)
            .args(["-O", "coff", "-i", "version.rc", "-o", "version.o"])
            .output();
        match run {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Ok(o) if o.status.success() => {
                // An object, not an archive, so the linker keeps it although
                // nothing refers to it.
                println!("cargo:rustc-link-arg-bins={}", out.join("version.o").display());
                return;
            }
            Ok(o) => why = format!("{tool} failed: {}", String::from_utf8_lossy(&o.stderr).trim()),
            Err(e) => why = format!("{tool} did not start: {e}"),
        }
        break;
    }
    // Watched only once missed: the next build reuses this script's output, so
    // a windres put on PATH afterwards would go unseen until build.rs changed.
    println!("cargo:rerun-if-env-changed=PATH");
    skip(why);
}

/// The resource script. Numbers stand for winver.h's names (VOS_NT_WINDOWS32,
/// VFT_APP), so the preprocessor needs no include path; 0x409 and 1200 are US
/// English and UTF-16, the block name "040904b0" spelled as numbers.
fn version_rc() -> String {
    let var = |key: &str| env::var(key).unwrap_or_default();
    let name = var("CARGO_PKG_NAME");
    let version = var("CARGO_PKG_VERSION");
    let numeric = format!(
        "{},{},{},0",
        var("CARGO_PKG_VERSION_MAJOR"),
        var("CARGO_PKG_VERSION_MINOR"),
        var("CARGO_PKG_VERSION_PATCH")
    );
    // The notice line of LICENSE and README (0046), with the licence named
    // since the properties dialog is all some readers of the file will see.
    let copyright = format!("Copyright 2026 Datadorelix, licensed under {}", var("CARGO_PKG_LICENSE"));
    let strings = [
        ("CompanyName", "Datadorelix".to_string()),
        ("FileDescription", format!("{name}, a browser IDE for dbt projects")),
        ("FileVersion", version.clone()),
        ("InternalName", name.clone()),
        ("LegalCopyright", copyright),
        ("OriginalFilename", format!("{name}.exe")),
        ("ProductName", name),
        ("ProductVersion", version),
    ];
    let mut rc = format!(
        "1 VERSIONINFO\nFILEVERSION {numeric}\nPRODUCTVERSION {numeric}\n\
         FILEFLAGSMASK 0x3f\nFILEFLAGS 0x0\nFILEOS 0x40004\nFILETYPE 0x1\nFILESUBTYPE 0x0\n\
         BEGIN\n  BLOCK \"StringFileInfo\"\n  BEGIN\n    BLOCK \"040904b0\"\n    BEGIN\n"
    );
    for (key, value) in strings {
        // A double quote is doubled inside an .rc string; nothing else needs it.
        rc.push_str(&format!("      VALUE \"{key}\", \"{}\"\n", value.replace('"', "\"\"")));
    }
    rc.push_str(
        "    END\n  END\n  BLOCK \"VarFileInfo\"\n  BEGIN\n    VALUE \"Translation\", 0x409, 1200\n  END\nEND\n",
    );
    rc
}

/// The nearest tag, how far past it, the short commit, and whether the tree was
/// dirty. None when there is no `.git`, or no git on PATH, which is not an
/// error: the date alone still answers "is this the binary I just built".
fn git_describe() -> Option<String> {
    let out = Command::new("git").args(["describe", "--tags", "--always", "--dirty"]).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// UTC, to the day. A stamp is for telling two builds apart, not for timing one.
fn build_date() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (y, m, d) = civil_from_days((secs / 86_400) as i64);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Days since the epoch to a calendar date, by Howard Hinnant's algorithm. Here
/// rather than from a crate, for one line of output (0003).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}
