use std::io::Write;

use crate::env::is_valid_name;
use crate::shell::Shell;

/// List exported variables (no args) or mark variables as exported (with args).
pub fn run(args: &[String], shell: &mut Shell, out: &mut dyn Write) -> i32 {
    let args = args.get(1..).unwrap_or(&[]);
    if args.is_empty() {
        let _ = list_exported(shell, out);
        return 0;
    }
    let mut status = 0;
    for arg in args {
        let (name, value) = split_arg(arg);
        if is_valid_name(name) {
            shell.env.set(name, value, true);
        } else {
            eprintln!("minishell: export: `{arg}': not a valid identifier");
            status = 1;
        }
    }
    status
}

/// Print `declare -x NAME="value"` (or bare `NAME`) for each exported
/// variable except `_`, sorted by name.
fn list_exported(shell: &Shell, out: &mut dyn Write) -> std::io::Result<()> {
    let mut vars: Vec<&crate::env::Var> = shell
        .env
        .vars()
        .iter()
        .filter(|v| v.exported && v.name != "_")
        .collect();
    vars.sort_by(|a, b| a.name.cmp(&b.name));
    for var in vars {
        match &var.value {
            Some(value) => writeln!(out, "declare -x {}=\"{}\"", var.name, escape_value(value))?,
            None => writeln!(out, "declare -x {}", var.name)?,
        }
    }
    Ok(())
}

/// Escape the characters `"`, `\\`, `$` and backtick with a backslash.
fn escape_value(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for c in value.chars() {
        if matches!(c, '"' | '\\' | '$' | '`') {
            escaped.push('\\');
        }
        escaped.push(c);
    }
    escaped
}

/// Split `NAME=value` into its parts; no `=` means the whole arg is the name
/// and no value is set.
fn split_arg(arg: &str) -> (&str, Option<String>) {
    match arg.split_once('=') {
        Some((name, value)) => (name, Some(value.to_string())),
        None => (arg, None),
    }
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
