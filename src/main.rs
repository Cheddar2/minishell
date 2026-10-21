mod builtins;
mod env;
mod exec;
mod expand;
mod lexer;
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
    match lexer::lex(line) {
        Err(lexer::LexError::UnclosedQuote(q)) => {
            eprintln!("minishell: syntax error: unclosed quote `{q}'");
            shell.last_status = 2;
        }
        Ok(tokens) => {
            let words: Vec<String> = tokens
                .iter()
                .filter_map(|t| match t {
                    lexer::Token::Word(segs) => {
                        expand::expand_word(segs, &shell.env, shell.last_status)
                    }
                    _ => None,
                })
                .collect();
            if tokens.iter().any(|t| !matches!(t, lexer::Token::Word(_))) {
                eprintln!("minishell: pipes and redirections are not implemented yet");
                shell.last_status = 1;
            } else if words.is_empty() {
                shell.last_status = 0;
            } else {
                let _ = out.flush();
                if builtins::is_builtin(&words[0]) {
                    shell.last_status = builtins::run(&words, shell, out);
                    let _ = out.flush();
                    return;
                }
                shell.last_status = match exec::path::resolve(&words[0], shell.env.get("PATH")) {
                    Err(e) => {
                        eprintln!("{}", e.message());
                        e.status()
                    }
                    Ok(p) => sys::run_external(&p, &words, &shell.env.to_envp()),
                };
            }
        }
    }
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
                    let _ = io::stdout().flush();
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
            let _ = io::stdout().flush();
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
