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

fn tmp(name: &str) -> String {
    std::env::temp_dir()
        .join(format!("m6_{name}_{}", std::process::id()))
        .to_string_lossy()
        .into_owned()
}

#[test]
fn smoke_redir_out_creates_file() {
    let f = tmp("a");
    run(&format!("echo hi > {f}\n"));
    let content = std::fs::read_to_string(&f).unwrap();
    assert_eq!(content, "hi\n");
    let _ = std::fs::remove_file(&f);
}

#[test]
fn smoke_redir_append() {
    let f = tmp("b");
    run(&format!("echo a > {f}\necho b >> {f}\n"));
    let content = std::fs::read_to_string(&f).unwrap();
    assert_eq!(content, "a\nb\n");
    let _ = std::fs::remove_file(&f);
}

#[test]
fn smoke_redir_in_feeds_cat() {
    let f = tmp("c");
    let (stdout, _code) = run(&format!("echo first > {f}\ncat < {f}\n"));
    assert_eq!(stdout, "first\n");
    let _ = std::fs::remove_file(&f);
}

#[test]
fn smoke_redir_two_outs_last_wins() {
    let f1 = tmp("d1");
    let f2 = tmp("d2");
    run(&format!("echo x > {f1} > {f2}\n"));
    let c1 = std::fs::read_to_string(&f1).unwrap();
    let c2 = std::fs::read_to_string(&f2).unwrap();
    assert_eq!(c1, "");
    assert_eq!(c2, "x\n");
    let _ = std::fs::remove_file(&f1);
    let _ = std::fs::remove_file(&f2);
}

#[test]
fn smoke_redir_in_missing_file_status_one() {
    let (stdout, _code) = run("cat < /nonexistent_m6_xyz\necho $?\n");
    assert_eq!(stdout, "1\n");
}

#[test]
fn smoke_bare_redir_creates_empty_file() {
    let f = tmp("f");
    let (_stdout, code) = run(&format!("> {f}\n"));
    assert_eq!(code, 0);
    let content = std::fs::read_to_string(&f).unwrap();
    assert_eq!(content, "");
    let _ = std::fs::remove_file(&f);
}

#[test]
fn smoke_builtin_with_redir() {
    let (stdout, _code) = run("export Z=zz > /dev/null\necho $Z\n");
    assert_eq!(stdout, "zz\n");
}

#[test]
fn smoke_dangling_redir_syntax_error() {
    let (stdout, _code) = run("echo >\necho $?\n");
    assert_eq!(stdout, "2\n");
}

#[test]
fn smoke_pipe_basic() {
    let (stdout, code) = run("echo hello | cat\n");
    assert_eq!(stdout, "hello\n");
    assert_eq!(code, 0);
}

#[test]
fn smoke_pipe_wc_count() {
    let (stdout, _code) = run("echo a | wc -l\n");
    assert_eq!(stdout.trim(), "1");
}

#[test]
fn smoke_pipe_left_exit_code() {
    let (stdout, _code) = run("echo x | false\necho $?\n");
    assert_eq!(stdout, "1\n");
}

#[test]
fn smoke_pipe_right_exit_code() {
    let (stdout, _code) = run("false | echo y\necho $?\n");
    assert_eq!(stdout, "y\n0\n");
}

#[test]
fn smoke_pipe_left_not_found() {
    let (stdout, _code) = run("nosuch_m7 | echo z\necho $?\n");
    assert_eq!(stdout, "z\n0\n");
}

#[test]
fn smoke_pipe_right_not_found() {
    let (stdout, _code) = run("echo z | nosuch_m7\necho $?\n");
    assert_eq!(stdout, "127\n");
}

#[test]
fn smoke_pipe_terminates() {
    let (stdout, _code) = run("yes | head -n 1\n");
    assert_eq!(stdout, "y\n");
}

#[test]
fn smoke_pipe_cd_then_pwd() {
    let (stdout, _code) = run("cd / | echo ok\npwd\n");
    let mut lines = stdout.lines();
    let _ = lines.next();
    let cwd = lines.next().unwrap().to_string();
    let expected = std::env::current_dir()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    assert_eq!(cwd, expected);
}

#[test]
fn smoke_pipe_exit_is_not_special() {
    let (stdout, code) = run("exit 5 | echo still\necho alive\n");
    assert_eq!(stdout, "still\nalive\n");
    assert_eq!(code, 0);
}

#[test]
fn smoke_pipe_then_redir_to_file() {
    let f = tmp("m7a");
    let _ = run(&format!("echo hi | cat > {f}\n"));
    let content = std::fs::read_to_string(&f).unwrap();
    assert_eq!(content, "hi\n");
    let _ = std::fs::remove_file(&f);
}

#[test]
fn smoke_pipe_chain_of_three() {
    let (_stdout, code) = run("cat | cat | ls /\n");
    assert_eq!(code, 0);
}

#[test]
fn smoke_pipe_dangling_left() {
    let (stdout, _code) = run("ls |\necho $?\n");
    assert_eq!(stdout, "2\n");
}

#[test]
fn smoke_pipe_dangling_right() {
    let (stdout, _code) = run("| ls\necho $?\n");
    assert_eq!(stdout, "2\n");
}

#[test]
fn smoke_pipe_no_trailing_newline() {
    let (stdout, _code) = run("echo -n hi | cat\n");
    assert_eq!(stdout, "hi");
}
