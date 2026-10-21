use std::io::Write;

use crate::shell::Shell;

/// True when `arg` is a `-n` flag: `-` followed by one or more `n`, nothing else.
fn is_n_flag(arg: &str) -> bool {
    let Some(rest) = arg.strip_prefix('-') else {
        return false;
    };
    !rest.is_empty() && rest.bytes().all(|b| b == b'n')
}

/// Print `args[1..]` joined by spaces, plus a newline unless a `-n` flag was seen.
pub fn run(args: &[String], _shell: &mut Shell, out: &mut dyn Write) -> i32 {
    let mut i = 1;
    let mut newline = true;
    while i < args.len() && is_n_flag(&args[i]) {
        newline = false;
        i += 1;
    }
    let mut status = 0;
    for (k, arg) in args[i..].iter().enumerate() {
        if k > 0 && out.write_all(b" ").is_err() {
            status = 1;
        }
        if out.write_all(arg.as_bytes()).is_err() {
            status = 1;
        }
    }
    if newline && out.write_all(b"\n").is_err() {
        status = 1;
    }
    status
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::Env;
    use crate::shell::Shell;

    fn run_echo(args: &[&str]) -> String {
        let mut shell = Shell::new();
        shell.env = Env::new();
        let mut out: Vec<u8> = Vec::new();
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let status = run(&args, &mut shell, &mut out);
        assert_eq!(status, 0);
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn echo_no_args_prints_newline() {
        assert_eq!(run_echo(&["echo"]), "\n");
    }

    #[test]
    fn echo_args_joined_by_spaces() {
        assert_eq!(run_echo(&["echo", "a", "b"]), "a b\n");
    }

    #[test]
    fn echo_n_suppresses_newline() {
        assert_eq!(run_echo(&["echo", "-n", "a"]), "a");
    }

    #[test]
    fn echo_repeated_n_flags_consumed() {
        assert_eq!(run_echo(&["echo", "-nnn", "-n", "a", "-n"]), "a -n");
    }

    #[test]
    fn echo_non_n_dash_arg_not_consumed() {
        assert_eq!(run_echo(&["echo", "-nx", "a"]), "-nx a\n");
    }

    #[test]
    fn echo_only_n_prints_nothing() {
        assert_eq!(run_echo(&["echo", "-n"]), "");
    }
}
