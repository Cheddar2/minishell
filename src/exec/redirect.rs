//! Open redirection files (M6 step 2).

use std::os::fd::OwnedFd;
use std::os::unix::fs::OpenOptionsExt;

use crate::parser::RedirKind;

/// (target_fd, opened). target_fd is 0 for input, 1 for output.
pub fn open_all(redirs: &[(RedirKind, String)]) -> Result<Vec<(i32, OwnedFd)>, String> {
    let mut opened = Vec::new();
    for (kind, path) in redirs {
        let mut opts = std::fs::OpenOptions::new();
        match kind {
            RedirKind::In => opts.read(true),
            RedirKind::Out => opts.write(true).create(true).truncate(true).mode(0o644),
            RedirKind::Append => opts.append(true).create(true).mode(0o644),
        };
        let file = match opts.open(path) {
            Ok(f) => f,
            Err(e) => {
                let desc = nix::errno::Errno::from_raw(e.raw_os_error().unwrap_or(0)).desc();
                return Err(format!("minishell: {path}: {desc}"));
            }
        };
        let target = if matches!(kind, RedirKind::In) { 0 } else { 1 };
        opened.push((target, OwnedFd::from(file)));
    }
    Ok(opened)
}

/// Run a builtin with the given fds applied via dup2, saving and restoring
/// the original stdin/stdout around it.
pub fn run_builtin_redirected(opened: &[(i32, OwnedFd)], run: impl FnOnce() -> i32) -> i32 {
    use std::os::fd::{AsFd, AsRawFd};
    let saved0 = std::io::stdin().as_fd().try_clone_to_owned().ok();
    let saved1 = std::io::stdout().as_fd().try_clone_to_owned().ok();
    for (target, fd) in opened {
        let _ = nix::unistd::dup2(fd.as_raw_fd(), *target);
    }
    let status = run();
    let _ = std::io::Write::flush(&mut std::io::stdout());
    if let Some(s) = &saved0 {
        let _ = nix::unistd::dup2(s.as_raw_fd(), 0);
    }
    if let Some(s) = &saved1 {
        let _ = nix::unistd::dup2(s.as_raw_fd(), 1);
    }
    status
}
