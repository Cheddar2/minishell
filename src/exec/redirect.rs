//! Redirections: open targets and apply them around builtins.

use std::fs::{self, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::os::fd::{AsFd, AsRawFd, OwnedFd};
use std::os::unix::fs::OpenOptionsExt;

use nix::errno::Errno;
use nix::unistd::dup2;

use crate::parser::RedirKind;

fn open_error(path: &str, err: &std::io::Error) -> String {
    let text = Errno::from_raw(err.raw_os_error().unwrap_or(0)).desc();
    format!("minishell: {path}: {text}")
}

/// One redirection to open, in redirect order: a file target (expanded) or a
/// heredoc body fd (already opened). Heredoc fds replace stdin like `In`.
pub enum FileRedir {
    File(RedirKind, String),
    Heredoc(OwnedFd),
}

/// `RedirKind` has no `Clone`; rebuild it from the borrowed value.
pub fn redir_kind(kind: &RedirKind) -> RedirKind {
    match kind {
        RedirKind::In => RedirKind::In,
        RedirKind::Out => RedirKind::Out,
        RedirKind::Append => RedirKind::Append,
        RedirKind::Heredoc => RedirKind::Heredoc,
    }
}

/// Open one file redirection. Returns (fd number it replaces, opened file):
/// input targets replace fd 0, output/append targets replace fd 1.
fn open_file(kind: &RedirKind, path: &str) -> Result<(i32, OwnedFd), String> {
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
        // Heredoc bodies are passed in already opened as `FileRedir::Heredoc`.
        RedirKind::Heredoc => return Err("minishell: internal: unopened heredoc".into()),
    };
    opts.mode(0o644);
    opts.open(path)
        .map(|file| (target, OwnedFd::from(file)))
        .map_err(|err| open_error(path, &err))
}

/// Open all redirections in redirect order, mixing file targets and already
/// opened heredoc bodies. On the first failure the already opened files are
/// dropped and the error message is returned.
pub fn open_all(redirs: &[FileRedir]) -> Result<Vec<(i32, OwnedFd)>, String> {
    redirs.iter().try_fold(Vec::new(), |mut opened, redir| {
        let pair = match redir {
            FileRedir::File(kind, path) => open_file(kind, path)?,
            // The body fd is the redirect's stdin replacement.
            FileRedir::Heredoc(fd) => {
                let fd = fd
                    .as_fd()
                    .try_clone_to_owned()
                    .map_err(|err| format!("minishell: error cloning heredoc fd: {err}"))?;
                (0, fd)
            }
        };
        opened.push(pair);
        Ok(opened)
    })
}

/// Open a heredoc body as a readable fd: the body is written to a temp file
/// that is removed right away (POSIX unlink semantics), so arbitrarily large
/// bodies stream without a pipe deadlock.
pub fn open_heredoc_body(body: &str) -> Result<OwnedFd, String> {
    let path = std::env::temp_dir().join(format!(
        "minishell-heredoc-{}-{}",
        std::process::id(),
        body.len()
    ));
    let fail = |err: &std::io::Error| format!("minishell: {path:?}: {}", err.kind());
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .map_err(|err| fail(&err))?;
    if let Err(err) = file.write_all(body.as_bytes()) {
        let _ = fs::remove_file(&path);
        return Err(fail(&err));
    }
    // Reuse the open file description (seeked back to the start) as the
    // reader fd, then unlink the path: the fd stays valid.
    if let Err(err) = file.seek(SeekFrom::Start(0)) {
        let _ = fs::remove_file(&path);
        return Err(fail(&err));
    }
    let fd = file
        .as_fd()
        .try_clone_to_owned()
        .map_err(|err| fail(&err))?;
    if let Err(err) = fs::remove_file(&path) {
        return Err(fail(&err));
    }
    Ok(fd)
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
