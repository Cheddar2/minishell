use std::ffi::CString;
use std::os::fd::{AsRawFd, OwnedFd};
use std::path::Path;

use nix::sys::signal::{self, SigHandler, Signal};
use nix::sys::wait::{waitpid, WaitStatus};
use nix::unistd::{dup2, execve, fork, ForkResult};

use crate::signals::{self, Mode};

/// Build the `CString`s for the path and the argv, printing the required
/// error for any argument that contains a NUL byte.
fn prepare_c_args(path: &Path, argv: &[String]) -> Option<(CString, Vec<CString>)> {
    let c_path = match path.to_str() {
        Some(s) => match CString::new(s) {
            Ok(c) => c,
            Err(_) => {
                eprintln!("minishell: {s}: invalid argument");
                return None;
            }
        },
        None => {
            eprintln!("minishell: {}: invalid argument", path.display());
            return None;
        }
    };
    let mut c_argv = Vec::with_capacity(argv.len());
    for arg in argv {
        match CString::new(arg.as_bytes()) {
            Ok(c) => c_argv.push(c),
            Err(_) => {
                let shown = String::from_utf8_lossy(arg.as_bytes());
                eprintln!("minishell: {shown}: invalid argument");
                return None;
            }
        }
    }
    Some((c_path, c_argv))
}

/// Child-side cleanup: use `libc::_exit` so that no Rust destructors run and
/// we never return into the REPL.
fn child_die(status: i32) -> ! {
    unsafe {
        // SAFETY: `_exit` terminates the process; `status` is a valid exit
        // code and there are no resources left to release.
        libc::_exit(status);
    }
}

/// Fork, exec and wait for an external command.
/// Returns the child's exit code, or 128 + signal if it was signaled.
pub fn run_external(
    path: &Path,
    argv: &[String],
    envp: &[CString],
    redirs: Vec<(i32, OwnedFd)>,
) -> i32 {
    // Everything that allocates happens before the fork.
    let (c_path, c_argv) = match prepare_c_args(path, argv) {
        Some(prepared) => prepared,
        None => return 1,
    };

    signals::set_mode(Mode::Running);

    let pid = match unsafe {
        // SAFETY: `fork` needs no arguments; the result is checked and the
        // child branch never returns to shared Rust state.
        fork()
    } {
        Ok(ForkResult::Parent { child }) => child,
        Ok(ForkResult::Child) => {
            // Child: never return to the parent's flow from here on.
            signals::set_mode(Mode::Child);
            let sig = unsafe {
                // SAFETY: restoring SIGPIPE's default disposition from the
                // child before exec is the only global state we touch.
                signal::signal(Signal::SIGPIPE, SigHandler::SigDfl)
            };
            if sig.is_err() {
                child_die(126);
            }
            for (target, fd) in redirs {
                // The `OwnedFd`s are dropped (and closed) when `redirs` is
                // drained; after `dup2` they are never used again.
                if dup2(fd.as_raw_fd(), target).is_err() {
                    child_die(126);
                }
            }
            if execve(c_path.as_c_str(), &c_argv, envp).is_err() {
                // execve failed: end immediately with 126, no destructors.
                child_die(126);
            }
            unreachable!("execve either replaced the process or failed")
        }
        Err(err) => {
            signals::set_mode(Mode::Prompt);
            eprintln!("minishell: fork: {err}");
            return 1;
        }
    };

    // Parent: wait, retrying on EINTR and on stop/continue status changes.
    let code = loop {
        let status = loop {
            match waitpid(pid, None) {
                Ok(status) => break status,
                Err(nix::errno::Errno::EINTR) => continue,
                Err(err) => {
                    signals::set_mode(Mode::Prompt);
                    eprintln!("minishell: waitpid: {err}");
                    return 1;
                }
            }
        };
        match status {
            WaitStatus::Exited(_, code) => break code,
            WaitStatus::Signaled(_, sig, _) => {
                let num = sig as i32;
                if num == Signal::SIGINT as i32 {
                    eprintln!();
                } else if num == Signal::SIGQUIT as i32 {
                    eprintln!("Quit (core dumped)");
                }
                break 128 + num;
            }
            // Stopped/continued: keep waiting for the real outcome.
            _ => {}
        }
    };

    signals::set_mode(Mode::Prompt);
    code
}
