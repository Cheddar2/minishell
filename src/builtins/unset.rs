use std::io::Write;

use crate::env::is_valid_name;
use crate::shell::Shell;

/// Remove variables from the environment.
pub fn run(args: &[String], shell: &mut Shell, out: &mut dyn Write) -> i32 {
    let _ = out;
    let mut status = 0;
    for arg in &args[1..] {
        if !is_valid_name(arg) {
            eprintln!("minishell: unset: `{arg}': not a valid identifier");
            status = 1;
            continue;
        }
        shell.env.unset(arg);
    }
    status
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::Env;

    fn new_shell() -> Shell {
        let mut shell = Shell::new();
        shell.env = Env::new();
        shell
    }

    fn run_unset(args: &[&str], shell: &mut Shell) -> i32 {
        let mut out: Vec<u8> = Vec::new();
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        run(&args, shell, &mut out)
    }

    #[test]
    fn unset_removes_existing_var() {
        let mut shell = new_shell();
        shell.env.set("A", Some("1".to_string()), true);
        let status = run_unset(&["unset", "A"], &mut shell);
        assert_eq!(status, 0);
        assert_eq!(shell.env.get("A"), None);
    }

    #[test]
    fn unset_missing_var_is_no_error() {
        let mut shell = new_shell();
        let status = run_unset(&["unset", "NOPE"], &mut shell);
        assert_eq!(status, 0);
    }

    #[test]
    fn unset_invalid_name() {
        let mut shell = new_shell();
        let status = run_unset(&["unset", "1A"], &mut shell);
        assert_eq!(status, 1);
    }
}
