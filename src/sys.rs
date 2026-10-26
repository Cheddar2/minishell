use std::ffi::CString;
use std::os::fd::{AsRawFd, OwnedFd};
use std::path::Path;

use nix::sys::signal::{self, SigHandler, Signal};
use nix::sys::wait::{waitpid, WaitStatus};
use nix::unistd::{dup2, execve, fork, pipe, ForkResult, Pid};

use crate::signals::{self, Mode};

/// The kind of command one pipeline stage runs.
pub enum StageKind {
    /// An external program: resolved path plus argv.
    External {
        path: std::path::PathBuf,
        argv: Vec<String>,
        envp: Vec<CString>,
    },
    /// A builtin, run in a child process (its `Shell` copy is the parent's).
    Builtin { argv: Vec<String> },
    /// A failure that must be reported inside the child (e.g. command not
    /// found, bad redirect): prints the message and exits with `status`.
    Failure { message: String, status: i32 },
    /// An empty stage (only redirections): exits with 0.
    Empty,
}

/// One prepared stage of a pipeline: its command plus its opened
/// redirections (fd number, opened file), applied after the pipe connections.
pub struct Stage {
    pub kind: StageKind,
    pub redirs: Vec<(i32, OwnedFd)>,
}

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

/// Wait for one child, retrying on EINTR and stop/continue status changes,
/// and turn its outcome into a shell status: the exit code, or 128 + signal
/// (with the SIGINT newline / SIGQUIT message). On a hard waitpid error the
/// prompt handlers are restored and 1 is returned.
fn wait_child(pid: Pid) -> i32 {
    loop {
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
    }
}

/// One stage with all its `CString`s already built (before any fork).
enum Prepared {
    External {
        path: CString,
        argv: Vec<CString>,
        envp: Vec<CString>,
    },
    Builtin {
        argv: Vec<String>,
    },
    Failure {
        message: String,
        status: i32,
    },
    Empty,
}

/// Child-side plumbing for one stage of a pipeline. Never returns.
fn child_run_pipeline(
    prep: &Prepared,
    mut redirs: Vec<(i32, OwnedFd)>,
    prev_read: Option<OwnedFd>,
    pipe: Option<(OwnedFd, OwnedFd)>,
    shell: &mut crate::shell::Shell,
) -> ! {
    signals::set_mode(Mode::Child);
    let sig = unsafe {
        // SAFETY: restoring SIGPIPE's default disposition in the child
        // before exec or running a builtin is the only global state touched.
        signal::signal(Signal::SIGPIPE, SigHandler::SigDfl)
    };
    if sig.is_err() {
        child_die(126);
    }
    // stdin from the previous stage's read end, stdout to this pipe's
    // write end; `dup2` copies the fd, so the original ends are still open.
    if let Some(read_end) = prev_read.as_ref() {
        if dup2(read_end.as_raw_fd(), 0).is_err() {
            child_die(1);
        }
    }
    if let Some((_, write_end)) = pipe.as_ref() {
        if dup2(write_end.as_raw_fd(), 1).is_err() {
            child_die(1);
        }
    }
    // Close every pipe end we still hold, or readers never get EOF.
    if let Some((read_end, write_end)) = pipe {
        drop(read_end);
        drop(write_end);
    }
    drop(prev_read);
    for (target, fd) in redirs.drain(..) {
        if dup2(fd.as_raw_fd(), target).is_err() {
            child_die(1);
        }
        // The `OwnedFd`s are dropped (and closed) by the drain; after
        // `dup2` they are never used again.
    }
    match prep {
        Prepared::External {
            path: c_path,
            argv: c_argv,
            envp,
        } => {
            if execve(c_path, c_argv, envp).is_err() {
                // execve failed: end immediately with 126, no destructors.
                child_die(126);
            }
            unreachable!("execve either replaced the process or failed");
        }
        Prepared::Builtin { argv } => {
            // A locked `StdoutLock` gives the builtin its own handle; since
            // `child_die` uses `_exit`, flush explicitly or output without a
            // trailing newline is lost.
            let mut out = std::io::stdout().lock();
            let status = crate::builtins::run(argv, shell, &mut out);
            let _ = std::io::Write::flush(&mut out);
            drop(out);
            child_die(status);
        }
        Prepared::Failure { message, status } => {
            // The NUL-byte case already printed its error during
            // preparation, so an empty message means nothing to print.
            if !message.is_empty() {
                eprintln!("{message}");
            }
            child_die(*status);
        }
        Prepared::Empty => child_die(0),
    }
}

/// Run every stage of a pipeline and return the last stage's status.
pub fn run_pipeline(stages: Vec<Stage>, shell: &mut crate::shell::Shell) -> i32 {
    // Build every `CString` before the first fork, so no allocation happens
    // in the child; a NUL byte turns that stage into a status-1 failure.
    let mut prepared: Vec<(Prepared, Vec<(i32, OwnedFd)>)> = Vec::with_capacity(stages.len());
    for stage in stages {
        let entry = match stage.kind {
            StageKind::External { path, argv, envp } => match prepare_c_args(&path, &argv) {
                Some((c_path, c_argv)) => (
                    Prepared::External {
                        path: c_path,
                        argv: c_argv,
                        envp,
                    },
                    stage.redirs,
                ),
                None => (
                    Prepared::Failure {
                        message: String::new(),
                        status: 1,
                    },
                    stage.redirs,
                ),
            },
            StageKind::Builtin { argv } => (Prepared::Builtin { argv }, stage.redirs),
            StageKind::Failure { message, status } => {
                (Prepared::Failure { message, status }, stage.redirs)
            }
            StageKind::Empty => (Prepared::Empty, stage.redirs),
        };
        prepared.push(entry);
    }

    signals::set_mode(Mode::Running);
    let mut pids: Vec<Pid> = Vec::with_capacity(prepared.len());
    let mut prev_read: Option<OwnedFd> = None;
    let mut failed = false;
    let total = prepared.len();
    for (idx, (prep, redirs)) in prepared.iter_mut().enumerate() {
        let pipe = if idx + 1 < total {
            match pipe() {
                Ok(p) => Some(p),
                Err(err) => {
                    eprintln!("minishell: pipe: {err}");
                    failed = true;
                    break;
                }
            }
        } else {
            None
        };
        let pid = match unsafe {
            // SAFETY: `fork` takes no arguments; the result is checked and
            // the child branch never returns into shared Rust state.
            fork()
        } {
            Ok(ForkResult::Parent { child }) => child,
            Ok(ForkResult::Child) => {
                child_run_pipeline(prep, std::mem::take(redirs), prev_read.take(), pipe, shell);
            }
            Err(err) => {
                eprintln!("minishell: fork: {err}");
                failed = true;
                break;
            }
        };
        pids.push(pid);
        // Parent: keep only the new read end for the next stage; drop the
        // write end (the child holds its own copy) and the old read end.
        match pipe {
            Some((read_end, write_end)) => {
                drop(write_end);
                prev_read = Some(read_end);
            }
            None => drop(prev_read.take()),
        }
    }
    // Drop the leftover read end before waiting, or a child may never see
    // EOF and hang.
    drop(prev_read);

    let mut last = 0;
    for pid in pids {
        last = wait_child(pid);
    }
    signals::set_mode(Mode::Prompt);
    if failed {
        1
    } else {
        last
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
    let code = wait_child(pid);

    signals::set_mode(Mode::Prompt);
    code
}
