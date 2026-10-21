use std::io::Write;

use crate::shell::Shell;

/// Print `NAME=value` for every exported variable that has a value.
pub fn run(args: &[String], shell: &mut Shell, out: &mut dyn Write) -> i32 {
    if args.len() > 1 {
        eprintln!("minishell: env: too many arguments");
        return 1;
    }
    let mut status = 0;
    for v in shell.env.vars() {
        let Some(value) = &v.value else {
            continue;
        };
        if !v.exported {
            continue;
        }
        if out.write_all(v.name.as_bytes()).is_err()
            || out.write_all(b"=").is_err()
            || out.write_all(value.as_bytes()).is_err()
            || out.write_all(b"\n").is_err()
        {
            status = 1;
        }
    }
    status
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::Env;
    use crate::shell::Shell;

    fn run_env(args: &[&str], setup: impl FnOnce(&mut Env)) -> (i32, String) {
        let mut shell = Shell::new();
        shell.env = Env::new();
        setup(&mut shell.env);
        let mut out: Vec<u8> = Vec::new();
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let status = run(&args, &mut shell, &mut out);
        (status, String::from_utf8(out).unwrap())
    }

    #[test]
    fn env_prints_exported_vars_with_values() {
        let (status, s) = run_env(&["env"], |env| {
            env.set("A", Some("1".to_string()), true);
            env.set("B", Some("2".to_string()), false);
            env.set("C", None, true);
        });
        assert_eq!(status, 0);
        assert_eq!(s, "A=1\n");
    }

    #[test]
    fn env_extra_args_error() {
        let (status, s) = run_env(&["env", "x"], |_| {});
        assert_eq!(status, 1);
        assert_eq!(s, "");
    }
}
