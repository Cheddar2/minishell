use std::io::Write;

use crate::env::is_valid_name;
use crate::shell::Shell;

/// List exported variables (no args) or mark variables as exported (with args).
pub fn run(args: &[String], shell: &mut Shell, out: &mut dyn Write) -> i32 {
    if args.len() == 1 {
        let mut vars: Vec<_> = shell
            .env
            .vars()
            .iter()
            .filter(|v| v.exported && v.name != "_")
            .collect();
        vars.sort_by(|a, b| a.name.cmp(&b.name));
        let mut status = 0;
        for v in vars {
            let line = match &v.value {
                Some(value) => format!("declare -x {}=\"{}\"\n", v.name, escape(value)),
                None => format!("declare -x {}\n", v.name),
            };
            if out.write_all(line.as_bytes()).is_err() {
                status = 1;
            }
        }
        return status;
    }
    let mut status = 0;
    for arg in &args[1..] {
        let (name, value) = arg
            .split_once('=')
            .map(|(n, v)| (n, Some(v)))
            .unwrap_or((arg.as_str(), None));
        if !is_valid_name(name) {
            eprintln!("minishell: export: `{arg}': not a valid identifier");
            status = 1;
            continue;
        }
        shell.env.set(name, value.map(String::from), true);
    }
    status
}

/// Escape `"`, `\`, `$` and backtick with a backslash.
fn escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        if matches!(c, '"' | '\\' | '$' | '`') {
            out.push('\\');
        }
        out.push(c);
    }
    out
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

    fn run_export(args: &[&str], shell: &mut Shell) -> (i32, String) {
        let mut out: Vec<u8> = Vec::new();
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let status = run(&args, shell, &mut out);
        (status, String::from_utf8(out).unwrap())
    }

    #[test]
    fn export_sets_value() {
        let mut shell = new_shell();
        let (status, s) = run_export(&["export", "A=1"], &mut shell);
        assert_eq!(status, 0);
        assert_eq!(s, "");
        assert_eq!(shell.env.get("A"), Some("1"));
    }

    #[test]
    fn export_bare_name_lists_declare_without_value() {
        let mut shell = new_shell();
        let (status, _) = run_export(&["export", "B"], &mut shell);
        assert_eq!(status, 0);
        let (_, s) = run_export(&["export"], &mut shell);
        assert!(s.contains("declare -x B\n"));
    }

    #[test]
    fn export_invalid_name() {
        let mut shell = new_shell();
        let (status, s) = run_export(&["export", "1A=x"], &mut shell);
        assert_eq!(status, 1);
        assert_eq!(s, "");
    }

    #[test]
    fn export_escapes_quotes_in_listing() {
        let mut shell = new_shell();
        let (status, _) = run_export(&["export", r#"X=a"b"#], &mut shell);
        assert_eq!(status, 0);
        let (_, s) = run_export(&["export"], &mut shell);
        assert!(s.contains(r#"declare -x X="a\"b""#));
    }

    #[test]
    fn export_listing_sorted_by_name() {
        let mut shell = new_shell();
        let (status, _) = run_export(&["export", "Z=1", "A=2"], &mut shell);
        assert_eq!(status, 0);
        let (_, s) = run_export(&["export"], &mut shell);
        assert_eq!(s, "declare -x A=\"2\"\ndeclare -x Z=\"1\"\n");
    }

    #[test]
    fn export_mixed_args_status_and_values() {
        let mut shell = new_shell();
        let (status, _) = run_export(&["export", "A=1", "2B=x", "C=3"], &mut shell);
        assert_eq!(status, 1);
        assert_eq!(shell.env.get("A"), Some("1"));
        assert_eq!(shell.env.get("C"), Some("3"));
    }
}
