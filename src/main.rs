mod builtins;
mod env;
mod exec;
mod expand;
mod lexer;
mod parser;
mod shell;
mod signals;
mod sys;

use std::io::{self, BufRead, Write};

const PROMPT: &str = "minishell$ ";

/// True when the line is empty or contains only spaces/tabs.
fn is_blank(line: &str) -> bool {
    line.chars().all(|c| c == ' ' || c == '\t')
}

fn process_line(line: &str, shell: &mut shell::Shell, out: &mut dyn Write) {
    let tokens = match lexer::lex(line) {
        Ok(tokens) => tokens,
        Err(lexer::LexError::UnclosedQuote(q)) => {
            eprintln!("minishell: syntax error: unclosed quote `{q}'");
            shell.last_status = 2;
            return;
        }
    };
    let cmd = match parser::parse(tokens) {
        Ok(cmd) => cmd,
        Err(parser::ParseError::Syntax(t)) => {
            eprintln!("minishell: syntax error near unexpected token `{t}'");
            shell.last_status = 2;
            return;
        }
    };
    shell.last_status = run_command(&cmd, shell, out);
}

/// `RedirKind` has no `Clone`; rebuild it from the borrowed value.
fn redir_kind(kind: &parser::RedirKind) -> parser::RedirKind {
    match kind {
        parser::RedirKind::In => parser::RedirKind::In,
        parser::RedirKind::Out => parser::RedirKind::Out,
        parser::RedirKind::Append => parser::RedirKind::Append,
    }
}

/// Expand and run one parsed simple command; returns its status.
fn run_command(cmd: &parser::SimpleCmd, shell: &mut shell::Shell, out: &mut dyn Write) -> i32 {
    let _ = out.flush();
    let last_status = shell.last_status;

    // Expand words: words that expand to nothing disappear.
    let argv: Vec<String> = cmd
        .argv
        .iter()
        .filter_map(|segs| expand::expand_word(segs, &shell.env, last_status))
        .collect();

    // Expand redirection targets: a target that expands to nothing is an error.
    let mut redirs: Vec<(parser::RedirKind, String)> = Vec::with_capacity(cmd.redirs.len());
    for redir in &cmd.redirs {
        match expand::expand_word(&redir.target, &shell.env, last_status) {
            Some(target) => redirs.push((redir_kind(&redir.kind), target)),
            None => {
                eprintln!("minishell: ambiguous redirect");
                return 1;
            }
        }
    }

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
                process_line(&line, shell, &mut out);
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
    let lines = stdin.lock().lines();
    let mut out = io::stdout().lock();
    for line in lines {
        let line = match line {
            Ok(line) => line,
            Err(_) => {
                let _ = out.flush();
                std::process::exit(1);
            }
        };
        if is_blank(&line) {
            continue;
        }
        process_line(&line, shell, &mut out);
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
