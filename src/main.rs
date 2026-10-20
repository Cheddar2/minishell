mod env;
mod expand;
mod lexer;
mod shell;
mod signals;

use std::io::{self, BufRead, Write};

const PROMPT: &str = "minishell$ ";

/// True when the line is empty or contains only spaces/tabs.
fn is_blank(line: &str) -> bool {
    line.chars().all(|c| c == ' ' || c == '\t')
}

fn handle_line(line: &str, out: &mut dyn Write) -> io::Result<()> {
    writeln!(out, "{line}")
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
            let _ = handle_line(&words.join(" "), out);
            shell.last_status = 0;
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

    #[test]
    fn handle_line_echoes_with_newline() {
        let mut buf: Vec<u8> = Vec::new();
        assert!(handle_line("hello", &mut buf).is_ok());
        assert_eq!(buf, b"hello\n");
    }
}
