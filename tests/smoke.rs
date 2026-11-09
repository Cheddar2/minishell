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
fn smoke_and_runs_right_when_left_ok() {
    let (stdout, _code) = run("true && echo a\n");
    assert_eq!(stdout, "a\n");
}

#[test]
fn smoke_and_skips_right_when_left_fails() {
    let (stdout, _code) = run("false && echo a\necho $?\n");
    assert_eq!(stdout, "1\n");
}

#[test]
fn smoke_or_runs_right_when_left_fails() {
    let (stdout, _code) = run("false || echo b\n");
    assert_eq!(stdout, "b\n");
}

#[test]
fn smoke_or_skips_right_when_left_ok() {
    let (stdout, _code) = run("true || echo b\necho $?\n");
    assert_eq!(stdout, "0\n");
}

#[test]
fn smoke_mixed_list_short_circuits_left_to_right() {
    // `false || false && echo c` = `(false || false) && echo c`: both `||`
    // sides ran, left side's status is 1, so `echo c` is skipped.
    let (stdout, _code) = run("false || false && echo c\necho $?\n");
    assert_eq!(stdout, "1\n");
}

#[test]
fn smoke_mixed_list_runs_all_when_ok() {
    // `true || false && echo d` = `(true || false) && echo d`: left side
    // short-circuits on `true` (status 0), so `echo d` runs.
    let (stdout, _code) = run("true || false && echo d\n");
    assert_eq!(stdout, "d\n");
}

#[test]
fn smoke_pipeline_then_and() {
    let (stdout, _code) = run("echo x | cat && echo y\n");
    assert_eq!(stdout, "x\ny\n");
}

#[test]
fn smoke_heredoc_then_and() {
    let (stdout, _code) = run("cat << E && echo z\nbody\nE\n");
    assert_eq!(stdout, "body\nz\n");
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

#[test]
fn smoke_heredoc_basic() {
    let (stdout, _code) = run("cat << E\nhello\nE\n");
    assert_eq!(stdout, "hello\n");
}

#[test]
fn smoke_heredoc_expansion() {
    let (stdout, _code) = run("cat << E\n$HOME\nE\n");
    assert_eq!(stdout, format!("{}\n", std::env::var("HOME").unwrap()));
}

#[test]
fn smoke_heredoc_quoted_delimiter() {
    let (stdout, _code) = run("cat << \"E\"\n$HOME\nE\n");
    assert_eq!(stdout, "$HOME\n");
}

#[test]
fn smoke_heredoc_last_wins() {
    let (stdout, _code) = run("cat << A << B\nfirst\nA\nsecond\nB\n");
    assert_eq!(stdout, "second\n");
}

#[test]
fn smoke_heredoc_in_pipeline() {
    let (stdout, _code) = run("cat << E | wc -l\na\nb\nc\nE\n");
    assert_eq!(stdout.trim(), "3");
}

#[test]
fn smoke_heredoc_eof_before_delimiter() {
    let (stdout, _code) = run("cat << E\nonly\n");
    assert_eq!(stdout, "only\n");
}

#[test]
fn smoke_heredoc_missing_word() {
    let (stdout, _code) = run("<<\necho $?\n");
    assert!(stdout.contains("2"));
}

#[test]
fn smoke_heredoc_command_after_delimiter() {
    let (stdout, _code) = run("cat << E\nbody\nE\necho after\n");
    assert_eq!(stdout, "body\nafter\n");
}

#[test]
fn smoke_word_splitting_unquoted_expansion() {
    let (stdout, code) = run("export A=\"x   y\"\necho $A\n");
    assert_eq!(stdout, "x y\n");
    assert_eq!(code, 0);
}

#[test]
fn smoke_ambiguous_redirect_exit_1() {
    let (stdout, code) = run("export A=\"a b\"\ncat < $A\necho $?\n");
    assert_eq!(stdout, "1\n");
    assert_eq!(code, 0);
}

#[test]
fn smoke_unset_path_missing_command_exit_127() {
    let (stdout, code) = run("unset PATH\nls\necho $?\n");
    assert_eq!(stdout, "127\n");
    assert_eq!(code, 0);
}

#[test]
fn smoke_group_left_of_pipe() {
    let (stdout, code) = run("(echo a && echo b) | cat\n");
    assert_eq!(stdout, "a\nb\n");
    assert_eq!(code, 0);
}

#[test]
fn smoke_group_right_of_pipe() {
    let (stdout, code) = run("echo x | (cat && echo y)\n");
    assert_eq!(stdout, "x\ny\n");
    assert_eq!(code, 0);
}

fn wild_temp_dir(tag: &str) -> std::path::PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!("minishell_wild_{}_{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

fn wild_touch(dir: &std::path::Path, name: &str) {
    std::fs::write(dir.join(name), b"seed\n").unwrap();
}

#[test]
fn smoke_wildcard_star_matches_visible_sorted() {
    let dir = wild_temp_dir("star");
    wild_touch(&dir, "z.rs");
    wild_touch(&dir, "b.txt");
    wild_touch(&dir, "a.rs");
    wild_touch(&dir, ".hidden");
    let (stdout, _code) = run(&format!("cd {}\necho *\n", dir.display()));
    assert_eq!(stdout, "a.rs b.txt z.rs\n");
}

#[test]
fn smoke_wildcard_hidden_only_with_dot() {
    let dir = wild_temp_dir("hidden");
    wild_touch(&dir, ".git");
    wild_touch(&dir, ".env");
    wild_touch(&dir, "a");
    let (stdout, _code) = run(&format!("cd {}\necho .*\n", dir.display()));
    assert_eq!(stdout, ".env .git\n");
}

#[test]
fn smoke_wildcard_prefix_suffix_and_multi_star() {
    let dir = wild_temp_dir("patterns");
    wild_touch(&dir, "a.rs");
    wild_touch(&dir, "ab.rs");
    wild_touch(&dir, "b.c");
    wild_touch(&dir, "xab.c");
    wild_touch(&dir, "a");
    let (stdout, _code) = run(&format!(
        "cd {}\necho *.rs\necho a*\necho *b*\necho a**\n",
        dir.display()
    ));
    assert_eq!(
        stdout,
        "a.rs ab.rs\na a.rs ab.rs\nab.rs b.c xab.c\na a.rs ab.rs\n"
    );
}

#[test]
fn smoke_wildcard_word_with_slash_left_as_is() {
    let dir = wild_temp_dir("slash");
    wild_touch(&dir, "a");
    let (stdout, _code) = run(&format!("cd {}\necho ./a*\n", dir.display()));
    assert_eq!(stdout, "./a*\n");
}

#[test]
fn smoke_wildcard_no_match_stays_literal() {
    let dir = wild_temp_dir("nomatch");
    wild_touch(&dir, "a");
    let (stdout, _code) = run(&format!("cd {}\necho *.nothing\n", dir.display()));
    assert_eq!(stdout, "*.nothing\n");
}

#[test]
fn smoke_wildcard_quoted_star_is_literal() {
    let dir = wild_temp_dir("quoted");
    wild_touch(&dir, "a");
    let (stdout, _code) = run(&format!(
        "cd {}\necho \"*\"\necho '*'\necho \"*\".rs\n",
        dir.display()
    ));
    assert_eq!(stdout, "*\n*\n*.rs\n");
}

#[test]
fn smoke_wildcard_only_unquoted_star_expands() {
    let dir = wild_temp_dir("mixed");
    wild_touch(&dir, "ab");
    wild_touch(&dir, "x");
    wild_touch(&dir, "*x");
    wild_touch(&dir, "*xy");
    let (stdout, _code) = run(&format!(
        "cd {}\necho \"a\"*\necho \"*\"x*\n",
        dir.display()
    ));
    assert_eq!(stdout, "ab\n*x *xy\n");
}

#[test]
fn smoke_wildcard_from_variable_expands() {
    let dir = wild_temp_dir("var");
    wild_touch(&dir, "a");
    wild_touch(&dir, "b");
    let (stdout, _code) = run(&format!("cd {}\nexport X='*'\necho $X\n", dir.display()));
    assert_eq!(stdout, "a b\n");
}

#[test]
fn smoke_wildcard_redirect_single_match() {
    let dir = wild_temp_dir("redir1");
    wild_touch(&dir, "t.txt");
    let (stdout, _code) = run(&format!(
        "cd {}\necho hi > *.txt\ncat \"t.txt\"\n",
        dir.display()
    ));
    assert_eq!(stdout, "hi\n");
}

#[test]
fn smoke_wildcard_redirect_no_match_literal() {
    let dir = wild_temp_dir("redir0");
    wild_touch(&dir, "a");
    let (stdout, _code) = run(&format!(
        "cd {}\necho hi > *.nothing\ncat \"*.nothing\"\n",
        dir.display()
    ));
    assert_eq!(stdout, "hi\n");
}

#[test]
fn smoke_wildcard_redirect_ambiguous_exit_1() {
    let dir = wild_temp_dir("redir2");
    wild_touch(&dir, "a.txt");
    wild_touch(&dir, "b.txt");
    let (stdout, _code) = run(&format!("cd {}\necho hi > *.txt\necho $?\n", dir.display()));
    assert_eq!(stdout, "1\n");
}

#[test]
fn smoke_wildcard_heredoc_delimiter_not_expanded() {
    let dir = wild_temp_dir("heredoc");
    wild_touch(&dir, "STOP");
    let (stdout, _code) = run(&format!("cd {}\ncat << STOP\nbody\nSTOP\n", dir.display()));
    assert_eq!(stdout, "body\n");
}
