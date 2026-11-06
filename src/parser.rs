use crate::lexer::{Segment, Token};

#[derive(Debug, PartialEq)]
pub enum RedirKind {
    In,
    Out,
    Append,
    Heredoc,
}

#[derive(Debug, PartialEq)]
pub struct Redir {
    pub kind: RedirKind,
    /// The redirection's word; for a heredoc this is the delimiter.
    pub target: Vec<Segment>,
    /// Heredocs only: whether any part of the delimiter was quoted
    /// (quoted delimiters make the body literal).
    pub quoted: bool,
}

#[derive(Debug, PartialEq, Default)]
pub struct SimpleCmd {
    pub argv: Vec<Vec<Segment>>,
    pub redirs: Vec<Redir>,
}

/// The operator of a list node: `&&` runs the right side only when the
/// left side's status is 0, `||` only when it is not 0.
#[derive(Debug, PartialEq)]
pub enum ListOp {
    And,
    Or,
}

/// One element of a pipeline: a simple command, or a subshell group
/// (an inner tree plus the redirections written after its `)`).
#[derive(Debug, PartialEq)]
pub enum Elem {
    Cmd(SimpleCmd),
    Group { tree: Tree, redirs: Vec<Redir> },
}

/// The parse tree of a line: a list of pipelines joined by `&&`/`||`
/// (same precedence, left-associative), a pipeline of one or more
/// elements joined by `|` (tightest binding), and an empty line.
#[derive(Debug, PartialEq)]
pub enum Tree {
    Empty,
    Pipeline(Vec<Elem>),
    List(Box<Tree>, ListOp, Box<Tree>),
}

/// True when the tree is a bare pipeline (the common case); anything
/// else (`&&`, `||`, or a subshell group) needs the not-yet-wired
/// execution path in M10 step 1.
#[allow(dead_code)] // kept for the parser tests; run_tree supersedes it
pub fn is_plain_pipeline(tree: &Tree) -> bool {
    match tree {
        Tree::Empty => true,
        Tree::Pipeline(elems) => elems.iter().all(|e| matches!(e, Elem::Cmd(_))),
        Tree::List(..) => false,
    }
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

/// Rebuild a word's original text ("as typed", quotes included): `Plain`
/// segments as-is, `Double` wrapped in `"..."`, `Single` wrapped in `'...'`.
fn segs_as_typed(segs: &[Segment]) -> String {
    let mut s = String::new();
    for seg in segs {
        match seg {
            Segment::Plain(t) => s.push_str(t),
            Segment::Double(t) => {
                s.push('"');
                s.push_str(t);
                s.push('"');
            }
            Segment::Single(t) => {
                s.push('\'');
                s.push_str(t);
                s.push('\'');
            }
        }
    }
    s
}

/// Parse a token stream into a tree (list of pipelines of elements).
///
/// Precedence: `|` binds tightest, then `&&`/`||` which share the same
/// precedence and group left to right, so `a || b && c` parses as
/// `(a || b) && c` and `a | b && c` as `(a | b) && c`.
pub fn parse(tokens: Vec<Token>) -> Result<Tree, ParseError> {
    if tokens.is_empty() {
        return Ok(Tree::Empty);
    }
    let mut p = P { toks: tokens, i: 0 };
    let tree = p.list()?;
    // `list` either consumed everything or errored; anything left here
    // is an unexpected token (e.g. `(a) b` or a stray `)`). A leftover
    // word is reported as its actual text, like bash (`near token `b'`).
    if p.i < p.toks.len() {
        let t = &p.toks[p.i];
        let msg = match t {
            Token::Word(segs) => segs_as_typed(segs),
            other => token_name(other).to_string(),
        };
        return Err(ParseError::Syntax(msg));
    }
    Ok(tree)
}

struct P {
    toks: Vec<Token>,
    i: usize,
}

impl P {
    fn peek(&self) -> Option<&Token> {
        self.toks.get(self.i)
    }

    /// `list := pipeline ( (And|Or) pipeline )*` (left-associative).
    fn list(&mut self) -> Result<Tree, ParseError> {
        let mut tree = self.pipeline()?;
        while matches!(self.peek(), Some(Token::And) | Some(Token::Or)) {
            let op = match self.toks[self.i] {
                Token::And => ListOp::And,
                _ => ListOp::Or,
            };
            self.i += 1;
            // `a &&` (EOF) or `a && || b` (operator after operator)
            // both fail here, near the offending token.
            let right = self.pipeline()?;
            tree = Tree::List(Box::new(tree), op, Box::new(right));
        }
        Ok(tree)
    }

    /// `pipeline := elem (Pipe elem)*`.
    fn pipeline(&mut self) -> Result<Tree, ParseError> {
        let mut elems = Vec::new();
        elems.push(self.elem()?);
        while let Some(Token::Pipe) = self.peek() {
            self.i += 1;
            // Trailing `|` with nothing after it.
            if self.peek().is_none() {
                return Err(ParseError::Syntax("|".into()));
            }
            elems.push(self.elem()?);
        }
        Ok(Tree::Pipeline(elems))
    }

    /// `elem := simple_cmd | group`, with `group := '(' list ')' redirs*`.
    ///
    /// Returns `Ok(None)` only when the next token is `)` (a group
    /// being closed from inside); every other position for `|`,
    /// `&&`, `||`, `)` or EOF is a syntax error.
    fn elem(&mut self) -> Result<Elem, ParseError> {
        match self.peek() {
            None => Err(ParseError::Syntax("newline".into())),
            Some(Token::Pipe) => Err(ParseError::Syntax("|".into())),
            Some(Token::And) => Err(ParseError::Syntax("&&".into())),
            Some(Token::Or) => Err(ParseError::Syntax("||".into())),
            Some(Token::RParen) => Err(ParseError::Syntax(")".into())),
            Some(Token::LParen) => {
                self.i += 1;
                // `()` (empty group) or an unbalanced `(` at EOL.
                match self.peek() {
                    Some(Token::RParen) => return Err(ParseError::Syntax(")".into())),
                    None => return Err(ParseError::Syntax("newline".into())),
                    Some(_) => {}
                }
                let tree = self.list()?;
                match self.peek() {
                    Some(Token::RParen) => {
                        self.i += 1;
                        let redirs = self.redirs()?;
                        Ok(Elem::Group { tree, redirs })
                    }
                    Some(t) => Err(ParseError::Syntax(token_name(t).into())),
                    None => Err(ParseError::Syntax("newline".into())),
                }
            }
            Some(_) => {
                let mut piece = Vec::new();
                loop {
                    match self.peek() {
                        None => break,
                        Some(
                            Token::Pipe | Token::And | Token::Or | Token::RParen | Token::LParen,
                        ) => break,
                        Some(t) => {
                            piece.push(t.clone());
                            self.i += 1;
                        }
                    }
                }
                Ok(Elem::Cmd(parse_simple(piece)?))
            }
        }
    }

    /// The redirections following a command or a group's `)`.
    fn redirs(&mut self) -> Result<Vec<Redir>, ParseError> {
        let mut redirs = Vec::new();
        while let Some(t) = self.peek() {
            let kind = match t {
                Token::RedirIn => RedirKind::In,
                Token::RedirOut => RedirKind::Out,
                Token::Append => RedirKind::Append,
                Token::Heredoc => RedirKind::Heredoc,
                _ => break,
            };
            self.i += 1;
            match self.peek() {
                Some(Token::Word(segs)) => {
                    let segs = segs.clone();
                    self.i += 1;
                    let quoted = segs
                        .iter()
                        .any(|s| matches!(s, Segment::Single(_) | Segment::Double(_)));
                    redirs.push(Redir {
                        kind,
                        target: segs,
                        quoted,
                    });
                }
                // `a >` (EOF) or `a > | b` and the like.
                Some(other) => return Err(ParseError::Syntax(token_name(other).into())),
                None => return Err(ParseError::Syntax("newline".into())),
            }
        }
        Ok(redirs)
    }
}

/// Parse a token stream into a single simple command with its redirections.
fn parse_simple(tokens: Vec<Token>) -> Result<SimpleCmd, ParseError> {
    let mut cmd = SimpleCmd::default();
    let mut i = 0;
    while i < tokens.len() {
        match &tokens[i] {
            Token::Word(segs) => cmd.argv.push(segs.clone()),
            Token::RedirIn | Token::RedirOut | Token::Append | Token::Heredoc => {
                let kind = match &tokens[i] {
                    Token::RedirIn => RedirKind::In,
                    Token::RedirOut => RedirKind::Out,
                    Token::Append => RedirKind::Append,
                    _ => RedirKind::Heredoc,
                };
                i += 1;
                if i >= tokens.len() {
                    return Err(ParseError::Syntax("newline".into()));
                }
                match &tokens[i] {
                    Token::Word(segs) => {
                        let quoted = segs
                            .iter()
                            .any(|s| matches!(s, Segment::Single(_) | Segment::Double(_)));
                        cmd.redirs.push(Redir {
                            kind,
                            target: segs.clone(),
                            quoted,
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
