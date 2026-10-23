use std::io::Write;

use crate::shell::Shell;

/// Change the shell's working directory (bash `cd`).
pub fn run(args: &[String], shell: &mut Shell, _out: &mut dyn Write) -> i32 {
    let args = &args[1..];
    if args.len() > 1 {
        eprintln!("minishell: cd: too many arguments");
        return 1;
    }
    let target = if args.is_empty() {
        match shell.env.get("HOME") {
            Some(home) => home.to_string(),
            None => {
                eprintln!("minishell: cd: HOME not set");
                return 1;
            }
        }
    } else {
        args[0].clone()
    };
    let old = shell.env.get("PWD").map(str::to_string).or_else(|| {
        std::env::current_dir()
            .ok()
            .and_then(|d| d.to_str().map(str::to_string))
    });
    if let Err(e) = std::env::set_current_dir(&target) {
        let desc = nix::errno::Errno::from_raw(e.raw_os_error().unwrap_or(0)).desc();
        eprintln!("minishell: cd: {target}: {desc}");
        return 1;
    }
    match std::env::current_dir() {
        Ok(dir) => match dir.to_str() {
            Some(new) => {
                if let Some(old) = old {
                    shell.env.set("OLDPWD", Some(old), true);
                }
                shell.env.set("PWD", Some(new.to_string()), true);
                0
            }
            None => {
                let e = std::io::Error::last_os_error();
                let desc = nix::errno::Errno::from_raw(e.raw_os_error().unwrap_or(0)).desc();
                eprintln!("minishell: cd: {target}: {desc}");
                1
            }
        },
        Err(e) => {
            let desc = nix::errno::Errno::from_raw(e.raw_os_error().unwrap_or(0)).desc();
            eprintln!("minishell: cd: {target}: {desc}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::Env;
    use crate::shell::Shell;

    fn make_shell(fill: impl FnOnce(&mut Env)) -> Shell {
        let mut shell = Shell::new();
        shell.env = Env::new();
        fill(&mut shell.env);
        shell
    }

    #[test]
    fn cd_too_many_arguments() {
        let mut shell = make_shell(|_| {});
        let mut out: Vec<u8> = Vec::new();
        let args = vec![String::from("cd"), String::from("a"), String::from("b")];
        assert_eq!(run(&args, &mut shell, &mut out), 1);
    }

    #[test]
    fn cd_without_home_fails() {
        let mut shell = make_shell(|_| {});
        let mut out: Vec<u8> = Vec::new();
        let args = vec![String::from("cd")];
        assert_eq!(run(&args, &mut shell, &mut out), 1);
    }

    #[test]
    fn cd_missing_path_keeps_pwd() {
        let mut shell = make_shell(|env| {
            env.set("PWD", Some("/some/pwd".to_string()), true);
        });
        let mut out: Vec<u8> = Vec::new();
        let args = vec![String::from("cd"), String::from("/nonexistent_xyz")];
        assert_eq!(run(&args, &mut shell, &mut out), 1);
        assert_eq!(shell.env.get("PWD"), Some("/some/pwd"));
        assert_eq!(shell.env.get("OLDPWD"), None);
    }
}
