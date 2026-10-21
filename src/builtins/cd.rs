use std::io::Write;

use crate::shell::Shell;

/// Change the shell's working directory (bash `cd`).
pub fn run(args: &[String], shell: &mut Shell, _out: &mut dyn Write) -> i32 {
    if args.len() > 2 {
        eprintln!("minishell: cd: too many arguments");
        return 1;
    }
    let target = match args.get(1) {
        Some(arg) => arg.clone(),
        None => match shell.env.get("HOME") {
            Some(home) => home.to_string(),
            None => {
                eprintln!("minishell: cd: HOME not set");
                return 1;
            }
        },
    };
    let old = match shell.env.get("PWD") {
        Some(pwd) => pwd.to_string(),
        None => std::env::current_dir()
            .map(|dir| dir.to_string_lossy().into_owned())
            .unwrap_or_default(),
    };
    if let Err(e) = nix::unistd::chdir(target.as_str()) {
        eprintln!("minishell: cd: {target}: {}", e.desc());
        return 1;
    }
    shell.env.set("OLDPWD", Some(old), true);
    if let Ok(dir) = std::env::current_dir() {
        shell
            .env
            .set("PWD", Some(dir.to_string_lossy().into_owned()), true);
    }
    0
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
