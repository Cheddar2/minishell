//! Heredoc body reading + expansion (M8, step 2).
//!
//! Not wired into the REPL yet (step 3): the functions here are pure and
//! take a line source as a closure, so they are unit-testable on their own.

#![allow(dead_code)]

use crate::env::Env;
use crate::expand::scan_into;
use crate::lexer::Segment;

/// The result of reading a heredoc body.
pub struct Heredoc {
    /// The body: every line read, each with a trailing `\n`. The delimiter
    /// line itself (when found) is NOT part of the body.
    pub body: String,
    /// True when the source ended before the delimiter was found. The
    /// caller prints the `here-document delimited by end-of-file` warning.
    pub eof: bool,
}

/// The delimiter text of a heredoc: the target's segments with their
/// quotes removed (concatenation of the inner strings).
pub fn delimiter(segs: &[Segment]) -> String {
    let mut text = String::new();
    for seg in segs {
        match seg {
            Segment::Plain(s) | Segment::Double(s) | Segment::Single(s) => text.push_str(s),
        }
    }
    text
}

/// Read lines from `lines` (each without its trailing newline) until one
/// that is exactly `delim`, or the source ends. Every line read is
/// appended to the body with its `\n` kept; the delimiter line, when
/// found, is not included. `eof` is reported when the source ended first.
pub fn read_body<L: FnMut() -> Option<String>>(delim: &str, mut lines: L) -> Heredoc {
    let mut body = String::new();
    let mut eof = true;
    while let Some(line) = lines() {
        if line == delim {
            eof = false;
            break;
        }
        body.push_str(&line);
        body.push('\n');
    }
    Heredoc { body, eof }
}

/// Expand a heredoc body: when the delimiter was NOT quoted, `$NAME`/`$?`
/// are expanded over the whole text (quotes inside the body are ordinary
/// characters and stay as-is). A quoted delimiter keeps the body literal.
pub fn expand_body(body: &str, quoted: bool, env: &Env, last_status: i32) -> String {
    if quoted {
        return body.to_string();
    }
    let mut out = String::with_capacity(body.len());
    scan_into(body, env, last_status, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// In-memory line source over `lines` (returned without trailing `\n`).
    /// The lines are owned as `String`s so the box can be `'static`.
    fn src(lines: Vec<&str>) -> Box<dyn FnMut() -> Option<String>> {
        let owned: Vec<String> = lines.into_iter().map(|s| s.to_string()).collect();
        let mut it = owned.into_iter();
        Box::new(move || it.next())
    }

    fn env_home() -> Env {
        let mut env = Env::new();
        env.set("HOME", Some("/home/adam".to_string()), true);
        env
    }

    #[test]
    fn stops_at_delimiter() {
        let mut count = 0;
        let mut lines = ["one", "two", "EOF", "three"].into_iter();
        let h = read_body("EOF", || {
            count += 1;
            lines.next().map(|s| s.to_string())
        });
        assert_eq!(h.body, "one\ntwo\n");
        assert!(!h.eof);
        // The line after the delimiter must never be consumed.
        assert_eq!(count, 3);
    }

    #[test]
    fn delimiter_line_is_excluded() {
        let h = read_body("EOF", &mut src(vec!["a", "EOF", "b"]));
        assert_eq!(h.body, "a\n");
        assert!(!h.eof);
    }

    #[test]
    fn every_line_keeps_its_newline() {
        let h = read_body("E", &mut src(vec!["x", "y"]));
        assert_eq!(h.body, "x\ny\n");
        // A single line before the delimiter still keeps its newline.
        let h1 = read_body("E", &mut src(vec!["x"]));
        assert_eq!(h1.body, "x\n");
    }

    #[test]
    fn line_merely_containing_delimiter_does_not_stop() {
        let h = read_body(
            "EOF",
            &mut src(vec!["a EOF b", "EOF extra", "x EOF", "EOF"]),
        );
        assert_eq!(h.body, "a EOF b\nEOF extra\nx EOF\n");
        assert!(!h.eof);
    }

    #[test]
    fn eof_before_delimiter_reports_it() {
        let h = read_body("EOF", &mut src(vec!["a", "b"]));
        assert_eq!(h.body, "a\nb\n");
        assert!(h.eof);
    }

    #[test]
    fn unquoted_body_is_expanded() {
        let env = env_home();
        let body = "a $HOME\nstatus $?\n";
        assert_eq!(
            expand_body(body, false, &env, 7),
            "a /home/adam\nstatus 7\n"
        );
    }

    #[test]
    fn quoted_delimiter_keeps_body_literal() {
        let env = env_home();
        let body = "a $HOME\nstatus $?\n";
        assert_eq!(expand_body(body, true, &env, 7), "a $HOME\nstatus $?\n");
    }

    #[test]
    fn quotes_inside_body_stay_as_is() {
        let mut env = env_home();
        env.set("NAME", Some("Bob".to_string()), true);
        let body = "he said \"hi $NAME\" and 'bye'\n";
        assert_eq!(
            expand_body(body, false, &env, 0),
            "he said \"hi Bob\" and 'bye'\n"
        );
    }

    #[test]
    fn delimiter_concatenates_segments() {
        let segs = vec![
            Segment::Plain("E".into()),
            Segment::Double("O".into()),
            Segment::Single("F".into()),
        ];
        assert_eq!(delimiter(&segs), "EOF");
        assert_eq!(delimiter(&[Segment::Plain("plain".into())]), "plain");
        assert_eq!(delimiter(&[]), "");
    }
}
