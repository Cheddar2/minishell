use std::io::Write;
use std::process::{Command, Stdio};

fn run(input: &str) -> (String, i32) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_minishell"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn minishell");
    child
        .stdin
        .take()
        .expect("failed to take child stdin")
        .write_all(input.as_bytes())
        .expect("failed to write child stdin");
    let output = child
        .wait_with_output()
        .expect("failed to read minishell output");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let code = output.status.code().unwrap_or(-1);
    (stdout, code)
}

#[test]
fn smoke_echo() {
    let (stdout, _code) = run("echo hello\n");
    assert_eq!(stdout, "hello\n");
}

#[test]
fn smoke_expands_double_single_and_unset_vars() {
    let (stdout, _code) = run("echo \"$HOME\" '$HOME' $NOPE_XYZ.\n");
    let expected = format!("{} $HOME .\n", std::env::var("HOME").unwrap_or_default());
    assert_eq!(stdout, expected);
}

#[test]
fn smoke_unclosed_quote_exit_two() {
    let (stdout, _code) = run("echo 'x\necho $?\n");
    assert_eq!(stdout, "2\n");
}

#[test]
fn smoke_command_not_found_exit_127() {
    let (stdout, _code) = run("nosuchcmd_xyz\necho $?\n");
    assert_eq!(stdout, "127\n");
}

#[test]
fn smoke_not_executable_exit_126() {
    let (stdout, _code) = run("/tmp\necho $?\n");
    assert_eq!(stdout, "126\n");
}

#[test]
fn smoke_false_exit_1() {
    let (stdout, _code) = run("/bin/false\necho $?\n");
    assert_eq!(stdout, "1\n");
}

#[test]
fn smoke_blank_input_exit_zero() {
    let out = run("\n   \n");
    assert_eq!(out, (String::new(), 0));
}

#[test]
fn smoke_echo_n_suppresses_newline() {
    let (stdout, _code) = run("echo -n hi\n");
    assert_eq!(stdout, "hi");
}

#[test]
fn smoke_echo_n_flags() {
    let (stdout, _code) = run("echo -nnn -n a -n\n");
    assert_eq!(stdout, "a -n");
}

#[test]
fn smoke_export_assignment() {
    let (stdout, _code) = run("export A=1 B\necho $A\n");
    assert_eq!(stdout, "1\n");
}

#[test]
fn smoke_export_invalid_name() {
    let (stdout, _code) = run("export 1A=x\necho $?\n");
    assert_eq!(stdout, "1\n");
}

#[test]
fn smoke_unset() {
    let (stdout, _code) = run("unset HOME\necho $HOME.\n");
    assert_eq!(stdout, ".\n");
}

#[test]
fn smoke_cd_pwd() {
    let (stdout, _code) = run("cd /tmp\npwd\n");
    assert_eq!(stdout, "/tmp\n");
}

#[test]
fn smoke_cd_missing_exit_one() {
    let (stdout, _code) = run("cd /nonexistent_xyz\necho $?\n");
    assert_eq!(stdout, "1\n");
}

#[test]
fn smoke_cd_olddpwd() {
    let (stdout, _code) = run("cd /tmp\ncd /\necho $OLDPWD\n");
    assert_eq!(stdout, "/tmp\n");
}

#[test]
fn smoke_exit_code() {
    let (_stdout, code) = run("exit 42\n");
    assert_eq!(code, 42);
}

#[test]
fn smoke_exit_wraps_modulo_256() {
    let (_stdout, code) = run("exit 300\n");
    assert_eq!(code, 44);
}

#[test]
fn smoke_exit_non_numeric_exit_two() {
    let (_stdout, code) = run("exit abc\n");
    assert_eq!(code, 2);
}

#[test]
fn smoke_exit_too_many_args_keeps_running() {
    let out = run("exit 1 2\necho $?\n");
    assert_eq!(out, ("1\n".to_string(), 0));
}

#[test]
fn smoke_env_lists_path() {
    let (stdout, _code) = run("env\n");
    assert!(stdout.lines().any(|l| l.starts_with("PATH=")));
}

#[test]
fn smoke_external_false_exit_one() {
    let (_stdout, code) = run("/bin/false\nexit\n");
    assert_eq!(code, 1);
}
