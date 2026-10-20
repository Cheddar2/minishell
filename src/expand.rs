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
fn scan_into(s: &str, env: &Env, last_status: i32, out: &mut String) {
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

#[cfg(test)]
#[path = "expand_tests.rs"]
mod tests;
