use crate::lexer::{Segment, Token};

#[derive(Debug, PartialEq)]
pub enum RedirKind {
    In,
    Out,
    Append,
}

#[derive(Debug, PartialEq)]
pub struct Redir {
    pub kind: RedirKind,
    pub target: Vec<Segment>,
}

#[derive(Debug, PartialEq, Default)]
pub struct SimpleCmd {
    pub argv: Vec<Vec<Segment>>,
    pub redirs: Vec<Redir>,
}

#[derive(Debug, PartialEq)]
pub enum ParseError {
    Syntax(String),
}

/// The operator spelling for a non-word token (used in error messages).
fn token_name(t: &Token) -> &'static str {
    match t {
        Token::Pipe => "|",
        Token::RedirIn => "<",
        Token::RedirOut => ">",
        Token::Append => ">>",
        Token::Heredoc => "<<",
        Token::And => "&&",
        Token::Or => "||",
        Token::LParen => "(",
        Token::RParen => ")",
        Token::Word(_) => "word",
    }
}

/// Parse a token stream into a single simple command with its redirections.
pub fn parse(tokens: Vec<Token>) -> Result<SimpleCmd, ParseError> {
    let mut cmd = SimpleCmd::default();
    let mut i = 0;
    while i < tokens.len() {
        match &tokens[i] {
            Token::Word(segs) => cmd.argv.push(segs.clone()),
            Token::RedirIn | Token::RedirOut | Token::Append => {
                let kind = match &tokens[i] {
                    Token::RedirIn => RedirKind::In,
                    Token::RedirOut => RedirKind::Out,
                    _ => RedirKind::Append,
                };
                i += 1;
                if i >= tokens.len() {
                    return Err(ParseError::Syntax("newline".into()));
                }
                match &tokens[i] {
                    Token::Word(segs) => {
                        cmd.redirs.push(Redir {
                            kind,
                            target: segs.clone(),
                        });
                    }
                    other => return Err(ParseError::Syntax(token_name(other).into())),
                }
            }
            other => return Err(ParseError::Syntax(token_name(other).into())),
        }
        i += 1;
    }
    Ok(cmd)
}

#[cfg(test)]
#[path = "parser_tests.rs"]
mod tests;
