//! Redirections: open targets and apply them around builtins.

use std::fs::OpenOptions;
use std::io::Write;
use std::os::fd::{AsFd, AsRawFd, OwnedFd};
use std::os::unix::fs::OpenOptionsExt;

use nix::errno::Errno;
use nix::unistd::dup2;

use crate::parser::RedirKind;

fn open_error(path: &str, err: &std::io::Error) -> String {
    let text = Errno::from_raw(err.raw_os_error().unwrap_or(0)).desc();
    format!("minishell: {path}: {text}")
}

/// Open one redirection target. Returns (fd number it replaces, opened file):
/// input targets replace fd 0, output/append targets replace fd 1.
fn open_one(kind: &RedirKind, path: &str) -> Result<(i32, OwnedFd), String> {
    let mut opts = OpenOptions::new();
    let target: i32 = match kind {
        RedirKind::In => {
            opts.read(true);
            0
        }
        RedirKind::Out => {
            opts.write(true).create(true).truncate(true);
            1
        }
        RedirKind::Append => {
            opts.write(true).create(true).append(true);
            1
        }
    };
    opts.mode(0o644);
    opts.open(path)
        .map(|file| (target, OwnedFd::from(file)))
        .map_err(|err| open_error(path, &err))
}

/// Open all redirection targets in order. On the first failure the already
/// opened files are dropped and the error message is returned.
pub fn open_all(redirs: &[(RedirKind, String)]) -> Result<Vec<(i32, OwnedFd)>, String> {
    redirs
        .iter()
        .try_fold(Vec::new(), |mut opened, (kind, path)| {
            let pair = open_one(kind, path)?;
            opened.push(pair);
            Ok(opened)
        })
}

/// Run a builtin with the opened fds applied, restoring stdin/stdout afterwards.
pub fn run_builtin_redirected(opened: &[(i32, OwnedFd)], run: impl FnOnce() -> i32) -> i32 {
    let old_stdin = std::io::stdin().as_fd().try_clone_to_owned();
    let old_stdout = std::io::stdout().as_fd().try_clone_to_owned();

    // Apply in order: the last entry for each fd wins.
    for (target, file) in opened {
        let _ = dup2(file.as_raw_fd(), *target);
    }

    let status = run();

    // Flush so redirected output is complete before restoring fd 1.
    let _ = std::io::stdout().flush();

    // Restore the original stdin/stdout exactly.
    if let Ok(old) = old_stdout {
        let _ = dup2(old.as_raw_fd(), 1);
    }
    if let Ok(old) = old_stdin {
        let _ = dup2(old.as_raw_fd(), 0);
    }

    status
}
