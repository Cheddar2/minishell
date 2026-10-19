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
        .write_all(b"hello\nworld\n")
        .expect("failed to write child stdin");
    let output = child
        .wait_with_output()
        .expect("failed to read minishell output");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("hello"), "stdout was: {stdout}");
    assert!(stdout.contains("world"), "stdout was: {stdout}");
}
