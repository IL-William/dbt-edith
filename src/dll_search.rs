//! Which folders Windows looks in when this process loads a DLL by name.
//!
//! The terminal's `portable-pty` loads `conpty.dll` by its bare name (its
//! `win/psuedocon.rs`, through `LoadLibraryW`), looking for a sideloaded copy
//! before it falls back to the one inside kernel32. Windows has no such file
//! in System32, so the default search goes on to the folder the `.exe` was
//! started from, the current folder and the PATH: a `conpty.dll` committed in a
//! dbt project, or left beside a downloaded `.exe`, would run as the user the
//! moment the terminal opens (0149). Called first in `main`, this narrows every
//! later by-name load to System32, where that file does not exist, so the
//! sideload fails and `portable-pty` takes kernel32's.
//!
//! Nothing else is lost: the other by-name loads in this process (kernel32 for
//! the terminal, the system libraries the standard library asks for) live in
//! System32. It covers loads made once `main` has started: what the loader
//! resolves before that, the import table, is not under this call's control, the
//! C runtime of the msvc `.exe` included.

/// `LOAD_LIBRARY_SEARCH_SYSTEM32`: only `%windows%\system32`.
#[cfg(windows)]
const SEARCH_SYSTEM32: u32 = 0x0000_0800;

#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn SetDefaultDllDirectories(directory_flags: u32) -> i32;
}

/// Restricts by-name DLL loads to System32 for the life of the process. A
/// failure is logged once and the run goes on: the terminal then behaves as it
/// did before, which is no worse than not having called it. No-op off Windows.
pub fn system32_only() {
    #[cfg(windows)]
    {
        // SAFETY: a plain call with a flag value, no pointers involved.
        if unsafe { SetDefaultDllDirectories(SEARCH_SYSTEM32) } == 0 {
            eprintln!(
                "warning: could not limit DLL loading to System32 ({}); the terminal may load a conpty.dll from other folders",
                std::io::Error::last_os_error()
            );
        }
    }
}
