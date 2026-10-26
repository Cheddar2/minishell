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

/// Parse a token stream into a pipeline of simple commands.
///
/// Splits on `Token::Pipe`; any empty group (leading `|`, trailing `|`, or
/// consecutive pipes) is a syntax error on `|`.
pub fn parse(tokens: Vec<Token>) -> Result<Vec<SimpleCmd>, ParseError> {
    let mut cmds = Vec::new();
    let mut piece = Vec::new();
    for token in tokens {
        match token {
            Token::Pipe => {
                if piece.is_empty() {
                    return Err(ParseError::Syntax("|".into()));
                }
                cmds.push(parse_simple(std::mem::take(&mut piece))?);
            }
            other => piece.push(other),
        }
    }
    if piece.is_empty() {
        if !cmds.is_empty() {
            // Trailing `|` with nothing after it.
            return Err(ParseError::Syntax("|".into()));
        }
        return Ok(cmds);
    }
    cmds.push(parse_simple(piece)?);
    Ok(cmds)
}

/// Parse a token stream into a single simple command with its redirections.
fn parse_simple(tokens: Vec<Token>) -> Result<SimpleCmd, ParseError> {
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
