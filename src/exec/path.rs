//! Command resolution against an explicit path or the `PATH` variable.
//!
//! Mirrors the resolution rules bash applies to a command word: a word that
//! contains a `/` is used as-is, otherwise each `:`-separated entry of `PATH`
//! is searched for a regular executable file (an empty entry means `.`).

use std::path::{Path, PathBuf};

use nix::errno::Errno;
use nix::unistd::{access, AccessFlags};

/// Errors that can occur while resolving a command word.
#[derive(Debug, PartialEq)]
pub enum PathError {
    /// The command word was empty or was not found on `PATH` (status 127).
    NotFound(String),
    /// An explicit path does not exist (status 127).
    NoSuchFile(String),
    /// The resolved path is a directory (status 126).
    IsDir(String),
    /// A matching file exists but is not executable (status 126).
    PermissionDenied(String),
}

impl PathError {
    /// The exit status bash reports for this error.
    pub fn status(&self) -> i32 {
        match self {
            Self::NotFound(_) | Self::NoSuchFile(_) => 127,
            Self::IsDir(_) | Self::PermissionDenied(_) => 126,
        }
    }

    /// The shell-style error message for this error.
    pub fn message(&self) -> String {
        match self {
            Self::NotFound(name) => format!("minishell: {name}: command not found"),
            Self::NoSuchFile(path) => {
                format!("minishell: {path}: No such file or directory")
            }
            Self::IsDir(path) => format!("minishell: {path}: Is a directory"),
            Self::PermissionDenied(path) => format!("minishell: {path}: Permission denied"),
        }
    }
}

/// Resolve `cmd` to an executable path.
///
/// A `cmd` containing a `/` is used as-is. Otherwise each `:`-separated
/// entry of `path_var` is searched and the first regular executable file
/// wins. When `PATH` is unset at all, the name is looked up as a file in
/// the current directory (like bash); an empty `PATH` or a name missing
/// from every `PATH` entry yields [`PathError::NotFound`].
pub fn resolve(cmd: &str, path_var: Option<&str>) -> Result<PathBuf, PathError> {
    if cmd.is_empty() {
        return Err(PathError::NotFound(String::new()));
    }
    if cmd.contains('/') {
        return resolve_explicit(cmd);
    }
    let Some(path_var) = path_var else {
        // `PATH` unset: look the name up in the current directory.
        return resolve_explicit(cmd);
    };
    if path_var.is_empty() {
        return Err(PathError::NotFound(cmd.to_string()));
    }

    let mut found_not_executable = false;
    for dir in path_var.split(':') {
        let candidate = Path::new(dir).join(cmd);
        let Ok(meta) = std::fs::metadata(&candidate) else {
            continue;
        };
        if !meta.is_file() {
            continue;
        }
        match access(&candidate, AccessFlags::X_OK) {
            Ok(()) => return Ok(candidate),
            Err(Errno::EACCES) | Err(Errno::EPERM) => found_not_executable = true,
            Err(_) => {}
        }
    }
    if found_not_executable {
        Err(PathError::PermissionDenied(cmd.to_string()))
    } else {
        Err(PathError::NotFound(cmd.to_string()))
    }
}

/// Resolve a command word that already contains a `/`, used as-is.
fn resolve_explicit(cmd: &str) -> Result<PathBuf, PathError> {
    let path = Path::new(cmd);
    let Ok(meta) = std::fs::metadata(path) else {
        return Err(PathError::NoSuchFile(cmd.to_string()));
    };
    if meta.is_dir() {
        return Err(PathError::IsDir(cmd.to_string()));
    }
    if access(path, AccessFlags::X_OK).is_err() {
        return Err(PathError::PermissionDenied(cmd.to_string()));
    }
    Ok(path.to_path_buf())
}
