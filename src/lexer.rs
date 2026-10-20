#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum Segment {
    Plain(String),
    Double(String),
    Single(String),
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Word(Vec<Segment>),
    Pipe,
    RedirIn,
    RedirOut,
    Append,
    Heredoc,
    And,
    Or,
    LParen,
    RParen,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LexError {
    UnclosedQuote(char),
}

fn is_space(c: char) -> bool {
    c == ' ' || c == '\t' || c == '\n'
}

/// Push any buffered plain text as a `Segment::Plain` into `segments`.
fn flush_plain(segments: &mut Vec<Segment>, plain: &mut String) {
    if !plain.is_empty() {
        segments.push(Segment::Plain(std::mem::take(plain)));
    }
}

/// End the current word, pushing it as `Token::Word`.
fn push_word(tokens: &mut Vec<Token>, segments: &mut Vec<Segment>, plain: &mut String) {
    flush_plain(segments, plain);
    tokens.push(Token::Word(std::mem::take(segments)));
}

/// Try to read an operator starting at `c`; consumes the second char of
/// two-char operators. `None` means the char is a plain word character.
fn operator(
    c: char,
    next: Option<char>,
    chars: &mut std::iter::Peekable<impl Iterator<Item = char>>,
) -> Option<Token> {
    match (c, next) {
        ('<', Some('<')) => {
            chars.next();
            Some(Token::Heredoc)
        }
        ('>', Some('>')) => {
            chars.next();
            Some(Token::Append)
        }
        ('&', Some('&')) => {
            chars.next();
            Some(Token::And)
        }
        ('|', Some('|')) => {
            chars.next();
            Some(Token::Or)
        }
        ('<', _) => Some(Token::RedirIn),
        ('>', _) => Some(Token::RedirOut),
        ('|', _) => Some(Token::Pipe),
        ('(', _) => Some(Token::LParen),
        (')', _) => Some(Token::RParen),
        _ => None,
    }
}

/// Read a quoted section starting after the opening quote `q`;
/// `chars` already consumed the opener. Returns the content, or the
/// error if input ends before the closing quote.
fn read_quoted<'a>(
    q: char,
    chars: &mut std::iter::Peekable<std::str::Chars<'a>>,
) -> Result<String, LexError> {
    let mut content = String::new();
    for qc in chars.by_ref() {
        if qc == q {
            return Ok(content);
        }
        content.push(qc);
    }
    Err(LexError::UnclosedQuote(q))
}

pub fn lex(input: &str) -> Result<Vec<Token>, LexError> {
    let mut tokens: Vec<Token> = Vec::new();
    let mut segments: Vec<Segment> = Vec::new();
    let mut plain = String::new();
    let mut in_word = false;
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\'' | '"' => {
                in_word = true;
                flush_plain(&mut segments, &mut plain);
                let content = read_quoted(c, &mut chars)?;
                let segment = if c == '\'' {
                    Segment::Single(content)
                } else {
                    Segment::Double(content)
                };
                segments.push(segment);
            }
            c if is_space(c) => {
                if in_word {
                    push_word(&mut tokens, &mut segments, &mut plain);
                    in_word = false;
                }
            }
            c => match operator(c, chars.peek().copied(), &mut chars) {
                Some(tok) => {
                    if in_word {
                        push_word(&mut tokens, &mut segments, &mut plain);
                        in_word = false;
                    }
                    tokens.push(tok);
                }
                None => {
                    in_word = true;
                    plain.push(c);
                }
            },
        }
    }
    if in_word {
        push_word(&mut tokens, &mut segments, &mut plain);
    }
    Ok(tokens)
}

#[cfg(test)]
#[path = "lexer_tests.rs"]
mod tests;
