//! Variable expansion + quote removal (M3 Step 2).

use crate::env::Env;
use crate::lexer::Segment;

fn is_name_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_name_cont(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Scan one unquoted/double-quoted section for `$` expansions, appending
/// the result to `out`.
pub(crate) fn scan_into(s: &str, env: &Env, last_status: i32, out: &mut String) {
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '$' {
            out.push(c);
            continue;
        }
        let next = chars.peek().copied();
        match next {
            Some('?') => {
                chars.next();
                out.push_str(&last_status.to_string());
            }
            Some(n) if is_name_start(n) => {
                chars.next();
                let mut name = String::new();
                name.push(n);
                while let Some(nc) = chars.peek() {
                    if is_name_cont(*nc) {
                        name.push(*nc);
                        chars.next();
                    } else {
                        break;
                    }
                }
                if let Some(v) = env.get(&name) {
                    out.push_str(v);
                }
            }
            Some(n) if n.is_ascii_digit() => {
                // `$<digit>`: consume the digit, expand to "".
                chars.next();
            }
            // `$` followed by anything else, or at the end: literal `$`.
            _ => out.push('$'),
        }
    }
}

/// Expand one word: apply variable expansion in `Plain`/`Double` segments,
/// copy `Single` segments literally (this IS quote removal).
///
/// Returns `None` (the word disappears) only if the word has NO quoted
/// segments and the result is empty; otherwise `Some(result)`.
#[cfg(test)]
pub fn expand_word(segs: &[Segment], env: &Env, last_status: i32) -> Option<String> {
    let mut result = String::new();
    let mut has_quoted = false;
    for seg in segs {
        match seg {
            Segment::Single(s) => {
                has_quoted = true;
                result.push_str(s);
            }
            Segment::Plain(s) => scan_into(s, env, last_status, &mut result),
            Segment::Double(s) => {
                has_quoted = true;
                scan_into(s, env, last_status, &mut result);
            }
        }
    }
    if result.is_empty() && !has_quoted {
        None
    } else {
        Some(result)
    }
}

/// IFS-like default separators: space, tab, newline.
fn is_ws(c: char) -> bool {
    c == ' ' || c == '\t' || c == '\n'
}

/// Append one chunk of text to the word list. `open` is true while the
/// last pushed chunk is unquoted and can still be joined by the next
/// chunk without starting a new word.
pub(crate) fn push_chunk(words: &mut Vec<String>, s: &str, open: &mut bool) {
    if *open {
        if let Some(last) = words.last_mut() {
            last.push_str(s);
        } else {
            words.push(s.to_string());
            *open = true;
        }
    } else if !s.is_empty() {
        words.push(s.to_string());
        *open = true;
    } else {
        // Empty chunk (e.g. `''`): opens a (possibly empty) word that
        // later chunks may join.
        words.push(String::new());
        *open = true;
    }
}

/// Split expanded text from an unquoted part on spaces, tabs and
/// newlines, appending each non-empty piece as a chunk. Any whitespace
/// run (interior or trailing) closes the open word, since the next
/// non-empty piece starts a new word; leading whitespace (with no open
/// word) starts nothing. The last non-empty piece leaves the word open,
/// so later chunks (e.g. quoted text) join it.
pub(crate) fn push_split(words: &mut Vec<String>, s: &str, open: &mut bool) {
    let mut seen_piece = false;
    for p in s.split(is_ws) {
        if p.is_empty() {
            *open = false;
        } else {
            // A piece following a previous one (or following leading
            // whitespace) starts a new word; only the very first piece may
            // join an already-open word (e.g. a literal prefix like `x$A`).
            if seen_piece {
                *open = false;
            }
            push_chunk(words, p, open);
            seen_piece = true;
        }
    }
    if s.trim_matches(is_ws).is_empty() && !s.is_empty() {
        *open = false;
    } else if s.chars().next_back().map(is_ws).unwrap_or(false) {
        // Trailing whitespace closes the word, so a later chunk
        // (e.g. the next expansion in `$A$B`) starts a new one.
        *open = false;
    }
}

/// Expand one word for the command line, applying bash word splitting:
/// the text produced by expanding `$NAME`/`$?` in `Plain` segments is
/// split on spaces, tabs and newlines into separate words; literal text
/// and everything inside quotes is never split. Runs of whitespace count
/// as one separator and never create empty words. A word with no quoted
/// segments that expands to nothing disappears.
pub fn expand_word_split(segs: &[Segment], env: &Env, last_status: i32) -> Vec<String> {
    let mut words: Vec<String> = Vec::new();
    let mut open = false;
    let mut has_quoted = false;
    for seg in segs {
        match seg {
            Segment::Single(s) => {
                has_quoted = true;
                push_chunk(&mut words, s, &mut open);
                // A quoted segment never starts a new word on its own; the
                // word it lands in stays open so the next unquoted segment
                // (e.g. the `x` in `"$HOME"x`) joins it, as in bash.
            }
            Segment::Double(s) => {
                has_quoted = true;
                let mut t = String::new();
                scan_into(s, env, last_status, &mut t);
                push_chunk(&mut words, &t, &mut open);
            }
            Segment::Plain(s) => {
                // Walk the segment, splitting only text that came from an
                // expansion; literal pieces are never split.
                let mut buf = String::new();
                let mut flush = |buf: &mut String| {
                    if buf.is_empty() {
                        return;
                    }
                    if buf.contains('$') {
                        let mut t = String::new();
                        scan_into(buf, env, last_status, &mut t);
                        push_split(&mut words, &t, &mut open);
                    } else {
                        // Unquoted literal text is also subject to word
                        // splitting (`x   y` -> `x`, `y`), as in bash.
                        push_split(&mut words, buf, &mut open);
                    }
                    buf.clear();
                };
                let mut chars = s.chars().peekable();
                while let Some(c) = chars.next() {
                    if c == '$' {
                        flush(&mut buf);
                        buf.push('$');
                        if let Some(n) = chars.peek().copied() {
                            if n == '?' || is_name_start(n) || n.is_ascii_digit() {
                                buf.push(n);
                                chars.next();
                                if n == '?' || n.is_ascii_digit() {
                                    continue;
                                }
                                while let Some(nc) = chars.peek().copied() {
                                    if is_name_cont(nc) {
                                        buf.push(nc);
                                        chars.next();
                                    } else {
                                        break;
                                    }
                                }
                            }
                        }
                    } else {
                        buf.push(c);
                    }
                }
                flush(&mut buf);
            }
        }
    }
    if words.is_empty() && has_quoted {
        words.push(String::new());
    } else {
        words.retain(|w| !w.is_empty() || has_quoted);
    }
    words
}

#[cfg(test)]
#[path = "expand_tests.rs"]
mod tests;
