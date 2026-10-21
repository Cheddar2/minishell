use std::io::Write;

use crate::shell::Shell;

/// Exit the shell, optionally with a given status.
pub fn run(args: &[String], shell: &mut Shell, out: &mut dyn Write) -> i32 {
    let _ = out;
    if shell.interactive {
        eprintln!("exit");
    }
    let code = if args.len() == 1 {
        shell.last_status
    } else {
        match args[1].trim().parse::<i64>() {
            Err(_) => {
                eprintln!("minishell: exit: {}: numeric argument required", args[1]);
                2
            }
            Ok(_) if args.len() > 2 => {
                eprintln!("minishell: exit: too many arguments");
                return 1;
            }
            Ok(n) => n.rem_euclid(256) as i32,
        }
    };
    shell.exit_code = Some(code);
    code
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::Env;

    fn new_shell() -> Shell {
        let mut shell = Shell::new();
        shell.env = Env::new();
        shell.interactive = false;
        shell
    }

    fn run_exit(args: &[&str], shell: &mut Shell) -> i32 {
        let mut out: Vec<u8> = Vec::new();
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        run(&args, shell, &mut out)
    }

    #[test]
    fn exit_without_args_uses_last_status() {
        let mut shell = new_shell();
        shell.last_status = 5;
        let status = run_exit(&["exit"], &mut shell);
        assert_eq!(status, 5);
        assert_eq!(shell.exit_code, Some(5));
    }

    #[test]
    fn exit_with_numeric_arg() {
        let mut shell = new_shell();
        let status = run_exit(&["exit", "42"], &mut shell);
        assert_eq!(status, 42);
        assert_eq!(shell.exit_code, Some(42));
    }

    #[test]
    fn exit_arg_wraps_modulo_256() {
        let mut shell = new_shell();
        let status = run_exit(&["exit", "300"], &mut shell);
        assert_eq!(status, 44);
        assert_eq!(shell.exit_code, Some(44));
    }

    #[test]
    fn exit_negative_arg_wraps() {
        let mut shell = new_shell();
        let status = run_exit(&["exit", "-1"], &mut shell);
        assert_eq!(status, 255);
        assert_eq!(shell.exit_code, Some(255));
    }

    #[test]
    fn exit_non_numeric_arg() {
        let mut shell = new_shell();
        let status = run_exit(&["exit", "abc"], &mut shell);
        assert_eq!(status, 2);
        assert_eq!(shell.exit_code, Some(2));
    }

    #[test]
    fn exit_too_many_args_does_not_exit() {
        let mut shell = new_shell();
        let status = run_exit(&["exit", "1", "2"], &mut shell);
        assert_eq!(status, 1);
        assert_eq!(shell.exit_code, None);
    }

    #[test]
    fn exit_arg_is_trimmed() {
        let mut shell = new_shell();
        let status = run_exit(&["exit", " 7 "], &mut shell);
        assert_eq!(status, 7);
        assert_eq!(shell.exit_code, Some(7));
    }
}
