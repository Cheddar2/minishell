use std::io::Write;

use crate::shell::Shell;

/// Print the current working directory plus a newline.
pub fn run(_args: &[String], _shell: &mut Shell, out: &mut dyn Write) -> i32 {
    match std::env::current_dir() {
        Ok(dir) => match dir.to_str() {
            Some(s) => {
                if out.write_all(s.as_bytes()).is_err() || out.write_all(b"\n").is_err() {
                    1
                } else {
                    0
                }
            }
            None => {
                eprintln!("minishell: pwd: {0}", std::io::Error::last_os_error());
                1
            }
        },
        Err(e) => {
            eprintln!("minishell: pwd: {e}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::Env;
    use crate::shell::Shell;

    #[test]
    fn pwd_output_ends_with_newline() {
        let mut shell = Shell::new();
        shell.env = Env::new();
        let mut out: Vec<u8> = Vec::new();
        let args = vec![String::from("pwd")];
        let status = run(&args, &mut shell, &mut out);
        assert_eq!(status, 0);
        let s = String::from_utf8(out).unwrap();
        assert!(!s.is_empty());
        assert!(s.ends_with('\n'));
    }
}
