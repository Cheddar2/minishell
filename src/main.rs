mod builtins;
mod env;
mod exec;
mod expand;
mod heredoc;
mod lexer;
mod parser;
mod shell;
mod signals;
mod sys;

use std::io::{self, BufRead, Write};
use std::os::fd::{AsFd, AsRawFd, OwnedFd};

use nix::unistd::dup2;

const PROMPT: &str = "minishell$ ";

/// One heredoc body fd, or its open error; `None` for non-heredoc redirs.
type HeredocFd = Option<Result<std::os::fd::OwnedFd, String>>;
/// The heredoc fds of one command, aligned with its redirs.
type PerCmdHeredocs = Vec<HeredocFd>;
/// The heredoc fds of a whole pipeline, aligned with its commands.
type PipelineHeredocs = Vec<PerCmdHeredocs>;

/// One redirection of a command, in redirect order: a file target (expanded)
/// or a heredoc body fd (already opened). The last stdin redirection wins.
type CmdRedir = exec::redirect::FileRedir;
type CmdRedirs = Vec<CmdRedir>;
/// A command's expanded argv plus its redirections, or the error message
/// that must be reported (ambiguous redirect / unopened file).
enum StagePrep {
    Run {
        argv: Vec<String>,
        redirs: CmdRedirs,
    },
    Fail(String),
}

/// Expand a command's words and redirection targets, mixing the already
/// opened heredoc bodies (aligned with `cmd.redirs`) into the redirect list.
fn prepare_cmd(
    cmd: &parser::SimpleCmd,
    shell: &shell::Shell,
    per_cmd: &PerCmdHeredocs,
) -> StagePrep {
    let last_status = shell.last_status;
    // One word can split into several after expansion (see
    // `expand::expand_word_split`); a word that expands to nothing
    // contributes no entry and disappears.
    let mut argv: Vec<String> = Vec::new();
    for segs in &cmd.argv {
        argv.extend(expand::expand_word_split(segs, &shell.env, last_status));
    }
    let mut redirs: CmdRedirs = Vec::with_capacity(cmd.redirs.len());
    for (redir, herfd) in cmd.redirs.iter().zip(per_cmd) {
        if let Some(fd) = herfd {
            // A body that failed to open fails this command with status 1.
            match fd {
                Ok(fd) => match fd.as_fd().try_clone_to_owned() {
                    Ok(fd) => {
                        redirs.push(CmdRedir::Heredoc(fd));
                        continue;
                    }
                    Err(err) => {
                        return StagePrep::Fail(format!(
                            "minishell: error cloning heredoc fd: {err}"
                        ))
                    }
                },
                Err(msg) => return StagePrep::Fail(msg.clone()),
            }
        }
        let mut target = expand::expand_word_split(&redir.target, &shell.env, last_status);
        if target.len() != 1 {
            return StagePrep::Fail(format!(
                "minishell: {}: ambiguous redirect",
                as_typed(&redir.target)
            ));
        }
        redirs.push(CmdRedir::File(
            exec::redirect::redir_kind(&redir.kind),
            target.remove(0),
        ));
    }
    StagePrep::Run { argv, redirs }
}

/// Rebuild a word's original text ("as typed", quotes included): `Plain`
/// segments as-is, `Double` wrapped in `"..."`, `Single` wrapped in `'...'`.
fn as_typed(segs: &[lexer::Segment]) -> String {
    let mut s = String::new();
    for seg in segs {
        match seg {
            lexer::Segment::Plain(t) => s.push_str(t),
            lexer::Segment::Double(t) => {
                s.push('"');
                s.push_str(t);
                s.push('"');
            }
            lexer::Segment::Single(t) => {
                s.push('\'');
                s.push_str(t);
                s.push('\'');
            }
        }
    }
    s
}

/// True when the line is empty or contains only spaces/tabs.
fn is_blank(line: &str) -> bool {
    line.chars().all(|c| c == ' ' || c == '\t')
}

fn process_line(
    line: &str,
    shell: &mut shell::Shell,
    out: &mut dyn Write,
    next_line: &mut dyn FnMut() -> SourceLine,
) {
    let tokens = match lexer::lex(line) {
        Ok(tokens) => tokens,
        Err(lexer::LexError::UnclosedQuote(q)) => {
            eprintln!("minishell: syntax error: unclosed quote `{q}'");
            shell.last_status = 2;
            return;
        }
    };
    let tree = match parser::parse(tokens) {
        Ok(tree) => tree,
        Err(parser::ParseError::Syntax(t)) => {
            eprintln!("minishell: syntax error near unexpected token `{t}'");
            shell.last_status = 2;
            return;
        }
    };
    // Collect the line's simple commands in source order, across the
    // whole tree (M10 step 2: heredocs are read before anything runs).
    let mut cmds = Vec::new();
    collect_cmds(&tree, &mut cmds);
    // Read every heredoc of the line, in order, before anything runs.
    let (heredocs, cancelled) = read_all_heredocs(&cmds, shell, next_line);
    if cancelled {
        // Ctrl-C while reading a body: the whole line is aborted.
        shell.last_status = 130;
        return;
    }
    shell.last_status = run_tree(&tree, &cmds, &heredocs, shell, out);
}

/// Append the tree's simple commands to `out` in source order, descending
/// into subshell groups so their heredocs are read before anything runs.
fn collect_cmds<'a>(tree: &'a parser::Tree, out: &mut Vec<&'a parser::SimpleCmd>) {
    match tree {
        parser::Tree::Empty => {}
        parser::Tree::Pipeline(elems) => {
            for e in elems {
                match e {
                    parser::Elem::Cmd(c) => out.push(c),
                    parser::Elem::Group { tree: inner, .. } => collect_cmds(inner, out),
                }
            }
        }
        parser::Tree::List(left, _, right) => {
            collect_cmds(left, out);
            collect_cmds(right, out);
        }
    }
}

/// Number of simple commands in a subtree (groups count their inner ones).
fn count_cmds(tree: &parser::Tree) -> usize {
    match tree {
        parser::Tree::Empty => 0,
        parser::Tree::Pipeline(elems) => elems
            .iter()
            .map(|e| match e {
                parser::Elem::Cmd(_) => 1,
                parser::Elem::Group { tree: inner, .. } => count_cmds(inner),
            })
            .sum(),
        parser::Tree::List(left, _, right) => count_cmds(left) + count_cmds(right),
    }
}

/// Run a tree: a list runs its left side, then its right side only when
/// the operator is satisfied (`&&`: status 0, `||`: status not 0); the
/// status is that of the last part that ran. A pipeline runs as today.
fn run_tree(
    tree: &parser::Tree,
    cmds: &[&parser::SimpleCmd],
    heredocs: &[PerCmdHeredocs],
    shell: &mut shell::Shell,
    out: &mut dyn Write,
) -> i32 {
    match tree {
        parser::Tree::Empty => 0,
        parser::Tree::Pipeline(elems) => {
            if elems
                .iter()
                .any(|e| matches!(e, parser::Elem::Group { .. }))
            {
                // A group alone in its pipeline runs in a subshell (step 3a);
                // a group inside a longer pipeline waits for step 3b.
                if let [parser::Elem::Group {
                    tree: inner,
                    redirs,
                }] = elems.as_slice()
                {
                    let last_status = shell.last_status;
                    let mut opened: Vec<(i32, OwnedFd)> = Vec::new();
                    let mut fail: Option<String> = None;
                    for redir in redirs {
                        if fail.is_some() {
                            break;
                        }
                        let mut target =
                            expand::expand_word_split(&redir.target, &shell.env, last_status);
                        if target.len() != 1 {
                            fail = Some(format!(
                                "minishell: {}: ambiguous redirect",
                                as_typed(&redir.target)
                            ));
                            continue;
                        }
                        let file = exec::redirect::FileRedir::File(
                            exec::redirect::redir_kind(&redir.kind),
                            target.remove(0),
                        );
                        match exec::redirect::open_all(&[file]) {
                            Ok(fds) => opened.extend(fds),
                            Err(msg) => fail = Some(msg),
                        }
                    }
                    return sys::fork_subshell(move || {
                        if let Some(msg) = fail {
                            eprintln!("{msg}");
                            return 1;
                        }
                        for (target, fd) in opened.drain(..) {
                            if dup2(fd.as_raw_fd(), target).is_err() {
                                return 1;
                            }
                        }
                        run_tree(inner, cmds, heredocs, shell, out)
                    });
                }
                // A group inside a longer pipeline: one stage per element,
                // group elements as `StageKind::Group` (step 3b).
                let last_status = shell.last_status;
                let mut stages = Vec::with_capacity(elems.len());
                let mut idx = 0;
                for e in elems {
                    match e {
                        parser::Elem::Cmd(cmd) => {
                            let per_cmd = &heredocs[idx];
                            idx += 1;
                            let stage = match prepare_cmd(cmd, shell, per_cmd) {
                                StagePrep::Fail(msg) => sys::Stage {
                                    kind: sys::StageKind::Failure {
                                        message: msg,
                                        status: 1,
                                    },
                                    redirs: Vec::new(),
                                },
                                StagePrep::Run { argv, redirs } => {
                                    match exec::redirect::open_all(&redirs) {
                                        Err(msg) => sys::Stage {
                                            kind: sys::StageKind::Failure {
                                                message: msg,
                                                status: 1,
                                            },
                                            redirs: Vec::new(),
                                        },
                                        Ok(opened) => {
                                            let kind = if argv.is_empty() {
                                                sys::StageKind::Empty
                                            } else if builtins::is_builtin(&argv[0]) {
                                                sys::StageKind::Builtin { argv }
                                            } else {
                                                match exec::path::resolve(
                                                    &argv[0],
                                                    shell.env.get("PATH"),
                                                ) {
                                                    Ok(path) => sys::StageKind::External {
                                                        path,
                                                        argv,
                                                        envp: shell.env.to_envp(),
                                                    },
                                                    Err(err) => sys::StageKind::Failure {
                                                        message: err.message(),
                                                        status: err.status(),
                                                    },
                                                }
                                            };
                                            sys::Stage {
                                                kind,
                                                redirs: opened,
                                            }
                                        }
                                    }
                                }
                            };
                            stages.push(stage);
                        }
                        parser::Elem::Group {
                            tree: inner,
                            redirs,
                        } => {
                            let mut opened: Vec<(i32, OwnedFd)> = Vec::new();
                            let mut fail: Option<String> = None;
                            for redir in redirs {
                                if fail.is_some() {
                                    break;
                                }
                                let mut target = expand::expand_word_split(
                                    &redir.target,
                                    &shell.env,
                                    last_status,
                                );
                                if target.len() != 1 {
                                    fail = Some(format!(
                                        "minishell: {}: ambiguous redirect",
                                        as_typed(&redir.target)
                                    ));
                                    continue;
                                }
                                let file = exec::redirect::FileRedir::File(
                                    exec::redirect::redir_kind(&redir.kind),
                                    target.remove(0),
                                );
                                match exec::redirect::open_all(&[file]) {
                                    Ok(fds) => opened.extend(fds),
                                    Err(msg) => fail = Some(msg),
                                }
                            }
                            let n = count_cmds(inner);
                            let cmds_slice = &cmds[idx..idx + n];
                            let heredocs_slice = &heredocs[idx..idx + n];
                            idx += n;
                            // The closure borrows the group's inner tree and
                            // this pipeline's cmd/heredoc slices; it runs only
                            // in the forked child, where the parent's borrows
                            // are gone, so the shared borrows are sound.
                            let kind = sys::StageKind::Group(Box::new(
                                move |child_shell: &mut shell::Shell| {
                                    if let Some(msg) = fail {
                                        eprintln!("{msg}");
                                        return 1;
                                    }
                                    // A locked `StdoutLock` stands in for the
                                    // parent's `out`; flushed explicitly since
                                    // the child dies with `_exit` afterwards.
                                    let mut out = std::io::stdout().lock();
                                    let status = run_tree(
                                        inner,
                                        cmds_slice,
                                        heredocs_slice,
                                        child_shell,
                                        &mut out,
                                    );
                                    let _ = std::io::Write::flush(&mut out);
                                    status
                                },
                            ));
                            stages.push(sys::Stage {
                                kind,
                                redirs: opened,
                            });
                        }
                    }
                }
                let _ = out.flush();
                return sys::run_pipeline(stages, shell);
            }
            run_pipeline(cmds, shell, out, heredocs)
        }
        parser::Tree::List(left, op, right) => {
            let left_count = count_cmds(left);
            let status = run_tree(
                left,
                &cmds[..left_count],
                &heredocs[..left_count],
                shell,
                out,
            );
            shell.last_status = status;
            let run_right = match op {
                parser::ListOp::And => status == 0,
                parser::ListOp::Or => status != 0,
            };
            if !run_right {
                return status;
            }
            let status = run_tree(
                right,
                &cmds[left_count..],
                &heredocs[left_count..],
                shell,
                out,
            );
            shell.last_status = status;
            status
        }
    }
}

/// One line read by the input source while reading heredoc bodies.
enum SourceLine {
    Line(String),
    /// The input source ended (no more lines).
    Eof,
    /// The user interrupted (interactive Ctrl-C).
    Interrupted,
}

/// Read every heredoc of the line, in order (left to right, across the
/// pipeline). Returns the opened body fds aligned with the redirs of each
/// command (`None` for non-heredoc redirs), plus whether an interrupt
/// aborted the reads.
fn read_all_heredocs(
    cmds: &[&parser::SimpleCmd],
    shell: &shell::Shell,
    next_line: &mut dyn FnMut() -> SourceLine,
) -> (PipelineHeredocs, bool) {
    let last_status = shell.last_status;
    let mut cancelled = false;
    let mut per_cmd = Vec::with_capacity(cmds.len());
    'cmds: for cmd in cmds {
        let mut per_redir = Vec::with_capacity(cmd.redirs.len());
        for redir in &cmd.redirs {
            if !matches!(redir.kind, parser::RedirKind::Heredoc) {
                per_redir.push(None);
                continue;
            }
            let delim = heredoc::delimiter(&redir.target);
            // Adapter for read_body: map the source to Option<String>,
            // flag an interrupt so the caller can cancel the whole line.
            let lines = || -> Option<String> {
                match next_line() {
                    SourceLine::Line(s) => Some(s),
                    SourceLine::Eof => None,
                    SourceLine::Interrupted => {
                        cancelled = true;
                        None
                    }
                }
            };
            let hd = heredoc::read_body(&delim, lines);
            if hd.eof {
                eprintln!(
                    "minishell: warning: here-document delimited by end-of-file (wanted `{delim}')"
                );
            }
            let expanded = heredoc::expand_body(&hd.body, redir.quoted, &shell.env, last_status);
            let fd = match exec::redirect::open_heredoc_body(&expanded) {
                Ok(fd) => Some(Ok(fd)),
                Err(msg) => Some(Err(msg)),
            };
            per_redir.push(fd);
            if cancelled {
                break 'cmds;
            }
        }
        per_cmd.push(per_redir);
    }
    (per_cmd, cancelled)
}

/// Run the pipeline: empty is a no-op, one command runs as before, more
/// than one is prepared as `sys::Stage`s and run as a real pipeline.
fn run_pipeline(
    cmds: &[&parser::SimpleCmd],
    shell: &mut shell::Shell,
    out: &mut dyn Write,
    heredocs: &[PerCmdHeredocs],
) -> i32 {
    match cmds.len() {
        0 => 0,
        1 => run_command(cmds[0], shell, out, heredocs.first().unwrap_or(&Vec::new())),
        _ => {
            let mut stages = Vec::with_capacity(cmds.len());
            for (cmd, per_cmd) in cmds.iter().zip(heredocs) {
                let stage = match prepare_cmd(cmd, shell, per_cmd) {
                    StagePrep::Fail(msg) => sys::Stage {
                        kind: sys::StageKind::Failure {
                            message: msg,
                            status: 1,
                        },
                        redirs: Vec::new(),
                    },
                    StagePrep::Run { argv, redirs } => match exec::redirect::open_all(&redirs) {
                        Err(msg) => sys::Stage {
                            kind: sys::StageKind::Failure {
                                message: msg,
                                status: 1,
                            },
                            redirs: Vec::new(),
                        },
                        Ok(opened) => {
                            let kind = if argv.is_empty() {
                                sys::StageKind::Empty
                            } else if builtins::is_builtin(&argv[0]) {
                                sys::StageKind::Builtin { argv }
                            } else {
                                match exec::path::resolve(&argv[0], shell.env.get("PATH")) {
                                    Ok(path) => sys::StageKind::External {
                                        path,
                                        argv,
                                        envp: shell.env.to_envp(),
                                    },
                                    Err(err) => sys::StageKind::Failure {
                                        message: err.message(),
                                        status: err.status(),
                                    },
                                }
                            };
                            sys::Stage {
                                kind,
                                redirs: opened,
                            }
                        }
                    },
                };
                stages.push(stage);
            }
            let _ = out.flush();
            sys::run_pipeline(stages, shell)
        }
    }
}

/// Expand and run one parsed simple command; returns its status.
fn run_command(
    cmd: &parser::SimpleCmd,
    shell: &mut shell::Shell,
    out: &mut dyn Write,
    per_cmd: &PerCmdHeredocs,
) -> i32 {
    let _ = out.flush();

    // The redirect list keeps source order: the last stdin wins.
    let (argv, redirs) = match prepare_cmd(cmd, shell, per_cmd) {
        StagePrep::Run { argv, redirs } => (argv, redirs),
        StagePrep::Fail(msg) => {
            eprintln!("{msg}");
            return 1;
        }
    };

    let opened = match exec::redirect::open_all(&redirs) {
        Ok(opened) => opened,
        Err(msg) => {
            eprintln!("{msg}");
            return 1;
        }
    };

    if argv.is_empty() {
        // Redirections still happened; a bare command is a successful no-op.
        return 0;
    }

    if builtins::is_builtin(&argv[0]) {
        return exec::redirect::run_builtin_redirected(&opened, || {
            builtins::run(&argv, shell, out)
        });
    }

    let path = match exec::path::resolve(&argv[0], shell.env.get("PATH")) {
        Ok(path) => path,
        Err(err) => {
            eprintln!("{}", err.message());
            return err.status();
        }
    };
    let envp = shell.env.to_envp();
    sys::run_external(&path, &argv, &envp, opened)
}

fn main() {
    let mut shell = shell::Shell::new();
    if shell.interactive {
        interactive(&mut shell)
    } else {
        non_interactive(&mut shell)
    }
}

fn interactive(shell: &mut shell::Shell) {
    signals::set_mode(signals::Mode::Prompt);
    let mut rl = match rustyline::DefaultEditor::new() {
        Ok(editor) => editor,
        Err(err) => {
            eprintln!("minishell: {err}");
            std::process::exit(1);
        }
    };
    loop {
        match rl.readline(PROMPT) {
            Ok(line) => {
                if is_blank(&line) {
                    continue;
                }
                let _ = rl.add_history_entry(line.as_str());
                let mut out = io::stdout().lock();
                // Heredoc bodies are read from the same editor, with a
                // secondary prompt and Heredoc signal handling; Ctrl-C
                // (rustyline Interrupted) cancels the whole line.
                let mut next_line = || -> SourceLine {
                    signals::set_mode(signals::Mode::Heredoc);
                    match rl.readline("> ") {
                        Ok(body_line) => SourceLine::Line(body_line),
                        Err(rustyline::error::ReadlineError::Interrupted) => {
                            signals::set_mode(signals::Mode::Prompt);
                            SourceLine::Interrupted
                        }
                        Err(rustyline::error::ReadlineError::Eof) => {
                            signals::set_mode(signals::Mode::Prompt);
                            SourceLine::Eof
                        }
                        Err(err) => {
                            signals::set_mode(signals::Mode::Prompt);
                            eprintln!("minishell: {err}");
                            SourceLine::Eof
                        }
                    }
                };
                process_line(&line, shell, &mut out, &mut next_line);
                signals::set_mode(signals::Mode::Prompt);
                if let Some(code) = shell.exit_code {
                    let _ = out.flush();
                    std::process::exit(code);
                }
            }
            Err(rustyline::error::ReadlineError::Interrupted) => {
                shell.last_status = 130;
            }
            Err(rustyline::error::ReadlineError::Eof) => {
                eprintln!("exit");
                break;
            }
            Err(err) => {
                eprintln!("minishell: {err}");
                shell.last_status = 1;
                break;
            }
        }
    }
    let _ = io::stdout().flush();
    std::process::exit(shell.last_status);
}

fn non_interactive(shell: &mut shell::Shell) {
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();
    let mut out = io::stdout().lock();
    loop {
        let line = match lines.next() {
            Some(Ok(line)) => line,
            Some(Err(_)) => {
                let _ = out.flush();
                std::process::exit(1);
            }
            None => break,
        };
        if is_blank(&line) {
            continue;
        }
        // The SAME stdin iterator also feeds the heredoc bodies.
        let mut next_line = || -> SourceLine {
            match lines.next() {
                Some(Ok(s)) => SourceLine::Line(s),
                Some(Err(_)) => SourceLine::Eof,
                None => SourceLine::Eof,
            }
        };
        process_line(&line, shell, &mut out, &mut next_line);
        if let Some(code) = shell.exit_code {
            let _ = out.flush();
            std::process::exit(code);
        }
    }
    let _ = out.flush();
    std::process::exit(shell.last_status);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_blank_detects_blank_lines() {
        assert!(is_blank(""));
        assert!(is_blank("   \t "));
        assert!(!is_blank("a"));
    }
}
