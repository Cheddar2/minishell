use std::io::{self, BufRead, Write};

const PROMPT: &str = "minishell$ ";

fn handle_line(line: &str, out: &mut dyn Write) -> io::Result<()> {
    writeln!(out, "{line}")
}

fn main() -> io::Result<()> {
    let mut out = io::stdout().lock();
    for line in io::stdin().lock().lines() {
        write!(out, "{PROMPT}")?;
        out.flush()?;
        handle_line(&line?, &mut out)?;
        out.flush()?;
    }
    writeln!(out, "exit")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handle_line_echoes_with_newline() {
        let mut buf: Vec<u8> = Vec::new();
        assert!(handle_line("hello", &mut buf).is_ok());
        assert_eq!(buf, b"hello\n");
    }
}
