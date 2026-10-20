use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn smoke_echoes_input_and_exits_zero() {
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
        .write_all(b"hello\n\n   \nworld\n")
        .expect("failed to write child stdin");
    let output = child
        .wait_with_output()
        .expect("failed to read minishell output");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout, "hello\nworld\n");
}

#[test]
fn smoke_unclosed_quote_no_echo_exit_two() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_minishell"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn minishell");
    child
        .stdin
        .take()
        .expect("failed to take child stdin")
        .write_all(b"echo 'oops\n")
        .expect("failed to write child stdin");
    let output = child
        .wait_with_output()
        .expect("failed to read minishell output");
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
}

#[test]
fn smoke_expands_double_single_and_unset_vars() {
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
        .write_all(b"echo \"$HOME\" '$HOME' $UNSET_XYZ.\n")
        .expect("failed to write child stdin");
    let output = child
        .wait_with_output()
        .expect("failed to read minishell output");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let expected = format!(
        "echo {} $HOME .\n",
        std::env::var("HOME").unwrap_or_default()
    );
    assert_eq!(stdout, expected);
}

#[test]
fn smoke_nested_single_quote_error_exit_two() {
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
        .write_all(b"echo 'x\necho $?\n")
        .expect("failed to write child stdin");
    let output = child
        .wait_with_output()
        .expect("failed to read minishell output");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout, "echo 2\n");
}
