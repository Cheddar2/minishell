pub mod cd;
pub mod echo;
pub mod env_cmd;
pub mod exit;
pub mod export;
pub mod pwd;
pub mod unset;

use crate::shell::Shell;

/// True when `name` is a builtin command.
pub fn is_builtin(name: &str) -> bool {
    matches!(
        name,
        "echo" | "cd" | "pwd" | "export" | "unset" | "env" | "exit"
    )
}

/// Run a builtin in the shell process; returns the exit status.
pub fn run(args: &[String], shell: &mut Shell, out: &mut dyn std::io::Write) -> i32 {
    match args.first().map(String::as_str) {
        Some("echo") => echo::run(args, shell, out),
        Some("cd") => cd::run(args, shell, out),
        Some("pwd") => pwd::run(args, shell, out),
        Some("env") => env_cmd::run(args, shell, out),
        Some("export") => export::run(args, shell, out),
        Some("unset") => unset::run(args, shell, out),
        Some("exit") => exit::run(args, shell, out),
        _ => 1,
    }
}
