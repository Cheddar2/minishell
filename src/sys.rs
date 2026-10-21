use std::ffi::CString;
use std::os::fd::AsRawFd;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

use nix::errno::Errno;
use nix::sys::signal::{signal, SigHandler, Signal};
use nix::sys::wait::{waitpid, WaitStatus};
use nix::unistd::{execve, fork, ForkResult};

use crate::signals::{self, Mode};

/// Fork, exec and wait for an external command.
/// Returns the child's exit code, or 128 + signal if it was signaled.
pub fn run_external(
    path: &Path,
    argv: &[String],
    envp: &[CString],
    redirs: Vec<(i32, std::os::fd::OwnedFd)>,
) -> i32 {
    // Build every C string BEFORE forking so that no child process is
    // created when an argument is invalid.
    let cpath = match CString::new(path.as_os_str().as_bytes()) {
        Ok(c) => c,
        Err(_) => {
            eprintln!("minishell: {}: invalid argument", path.display());
            return 1;
        }
    };
    let mut cargv: Vec<CString> = Vec::with_capacity(argv.len());
    for arg in argv {
        match CString::new(arg.as_bytes()) {
            Ok(c) => cargv.push(c),
            Err(_) => {
                eprintln!("minishell: {arg}: invalid argument");
                return 1;
            }
        }
    }
    signals::set_mode(Mode::Running);
    let result = unsafe {
        // SAFETY: the shell is single-threaded at this point, and the child
        // only calls async-signal-safe functions (set_mode, signal, execve,
        // _exit) before its image is replaced by execve.
        fork()
    };
    let status = match result {
        Ok(ForkResult::Child) => {
            signals::set_mode(Mode::Child);
            for (target, fd) in &redirs {
                let _ = nix::unistd::dup2(fd.as_raw_fd(), *target);
            }
            unsafe {
                // SAFETY: signal() with SIGPIPE/SigDfl uses a valid signal
                // number and a valid handler; the child must behave like a
                // regular program with respect to SIGPIPE.
                let _ = signal(Signal::SIGPIPE, SigHandler::SigDfl);
            }
            match execve(&cpath, &cargv, envp) {
                Ok(never) => match never {},
                Err(_) => unsafe {
                    // SAFETY: _exit terminates the child immediately without
                    // running Rust destructors, which is exactly what a failed
                    // exec requires.
                    libc::_exit(126)
                },
            }
        }
        Err(e) => {
            eprintln!("minishell: fork: {e}");
            signals::set_mode(Mode::Prompt);
            return 1;
        }
        Ok(ForkResult::Parent { child }) => {
            let mut status = 1;
            loop {
                match waitpid(child, None) {
                    Ok(WaitStatus::Exited(_, code)) => {
                        status = code;
                        break;
                    }
                    Ok(WaitStatus::Signaled(_, sig, _core)) => {
                        if sig == Signal::SIGINT {
                            eprintln!();
                        } else if sig == Signal::SIGQUIT {
                            eprintln!("Quit (core dumped)");
                        }
                        status = 128 + sig as i32;
                        break;
                    }
                    Err(Errno::EINTR) => continue,
                    Ok(_) => continue,
                    Err(_) => break,
                }
            }
            status
        }
    };
    signals::set_mode(Mode::Prompt);
    status
}
