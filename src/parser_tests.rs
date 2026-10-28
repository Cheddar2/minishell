use super::*;

fn w(s: &str) -> Token {
    Token::Word(vec![Segment::Plain(s.into())])
}

#[test]
fn two_words() {
    let cmd = &parse(vec![w("ls"), w("-l")]).unwrap()[0];
    assert_eq!(cmd.argv.len(), 2);
    assert!(cmd.redirs.is_empty());
}

#[test]
fn redir_in() {
    let cmd = &parse(vec![w("cat"), Token::RedirIn, w("in")]).unwrap()[0];
    assert_eq!(cmd.argv, vec![vec![Segment::Plain("cat".into())]]);
    assert_eq!(
        cmd.redirs,
        vec![Redir {
            kind: RedirKind::In,
            target: vec![Segment::Plain("in".into())],
            quoted: false
        }]
    );
}

#[test]
fn redir_out() {
    let cmd = &parse(vec![w("ls"), Token::RedirOut, w("out")]).unwrap()[0];
    assert_eq!(
        cmd.redirs,
        vec![Redir {
            kind: RedirKind::Out,
            target: vec![Segment::Plain("out".into())],
            quoted: false
        }]
    );
}

#[test]
fn redir_append() {
    let cmd = &parse(vec![w("echo"), w("hi"), Token::Append, w("log")]).unwrap()[0];
    assert_eq!(cmd.argv.len(), 2);
    assert_eq!(
        cmd.redirs,
        vec![Redir {
            kind: RedirKind::Append,
            target: vec![Segment::Plain("log".into())],
            quoted: false
        }]
    );
}

#[test]
fn redir_before_command() {
    let cmd = &parse(vec![Token::RedirIn, w("in"), w("cat")]).unwrap()[0];
    assert_eq!(cmd.argv, vec![vec![Segment::Plain("cat".into())]]);
    assert_eq!(
        cmd.redirs,
        vec![Redir {
            kind: RedirKind::In,
            target: vec![Segment::Plain("in".into())],
            quoted: false
        }]
    );
}

#[test]
fn two_redir_in_in_order() {
    let cmd = &parse(vec![
        w("cat"),
        Token::RedirIn,
        w("a"),
        Token::RedirIn,
        w("b"),
    ])
    .unwrap()[0];
    assert_eq!(
        cmd.redirs,
        vec![
            Redir {
                kind: RedirKind::In,
                target: vec![Segment::Plain("a".into())],
                quoted: false
            },
            Redir {
                kind: RedirKind::In,
                target: vec![Segment::Plain("b".into())],
                quoted: false
            }
        ]
    );
}

#[test]
fn two_redir_out_in_order() {
    let cmd = &parse(vec![
        w("echo"),
        w("a"),
        Token::RedirOut,
        w("f1"),
        Token::RedirOut,
        w("f2"),
    ])
    .unwrap()[0];
    assert_eq!(
        cmd.redirs,
        vec![
            Redir {
                kind: RedirKind::Out,
                target: vec![Segment::Plain("f1".into())],
                quoted: false
            },
            Redir {
                kind: RedirKind::Out,
                target: vec![Segment::Plain("f2".into())],
                quoted: false
            }
        ]
    );
}

#[test]
fn dangling_redir_at_eol() {
    assert_eq!(
        parse(vec![Token::RedirOut]),
        Err(ParseError::Syntax("newline".into()))
    );
}

#[test]
fn redir_followed_by_operator() {
    assert_eq!(
        parse(vec![Token::RedirIn, Token::RedirOut]),
        Err(ParseError::Syntax(">".into()))
    );
}

#[test]
fn pipe_not_supported() {
    assert_eq!(
        parse(vec![Token::Pipe]),
        Err(ParseError::Syntax("|".into()))
    );
}

#[test]
fn heredoc_plain() {
    let cmd = &parse(vec![w("cat"), Token::Heredoc, w("EOF")]).unwrap()[0];
    assert_eq!(cmd.argv, vec![vec![Segment::Plain("cat".into())]]);
    assert_eq!(
        cmd.redirs,
        vec![Redir {
            kind: RedirKind::Heredoc,
            target: vec![Segment::Plain("EOF".into())],
            quoted: false
        }]
    );
}

#[test]
fn heredoc_single_quoted() {
    let cmd = &parse(vec![
        w("cat"),
        Token::Heredoc,
        Token::Word(vec![Segment::Single("EOF".into())]),
    ])
    .unwrap()[0];
    assert_eq!(
        cmd.redirs,
        vec![Redir {
            kind: RedirKind::Heredoc,
            target: vec![Segment::Single("EOF".into())],
            quoted: true
        }]
    );
}

#[test]
fn heredoc_double_quoted() {
    let cmd = &parse(vec![
        w("cat"),
        Token::Heredoc,
        Token::Word(vec![Segment::Double("EOF".into())]),
    ])
    .unwrap()[0];
    assert_eq!(
        cmd.redirs,
        vec![Redir {
            kind: RedirKind::Heredoc,
            target: vec![Segment::Double("EOF".into())],
            quoted: true
        }]
    );
}

#[test]
fn heredoc_mixed_quoted() {
    let cmd = &parse(vec![
        w("cat"),
        Token::Heredoc,
        Token::Word(vec![
            Segment::Double("E".into()),
            Segment::Plain("OF".into()),
        ]),
    ])
    .unwrap()[0];
    assert_eq!(
        cmd.redirs,
        vec![Redir {
            kind: RedirKind::Heredoc,
            target: vec![Segment::Double("E".into()), Segment::Plain("OF".into())],
            quoted: true
        }]
    );
}

#[test]
fn heredoc_dangling() {
    assert_eq!(
        parse(vec![Token::Heredoc]),
        Err(ParseError::Syntax("newline".into()))
    );
}

#[test]
fn heredoc_followed_by_operator() {
    assert_eq!(
        parse(vec![w("cat"), Token::Heredoc, Token::RedirOut]),
        Err(ParseError::Syntax(">".into()))
    );
}

#[test]
fn pipe_two_commands() {
    let cmds = parse(vec![w("ls"), Token::Pipe, w("wc")]).unwrap();
    assert_eq!(cmds.len(), 2);
}

#[test]
fn pipe_three_commands() {
    let cmds = parse(vec![w("a"), Token::Pipe, w("b"), Token::Pipe, w("c")]).unwrap();
    assert_eq!(cmds.len(), 3);
}

#[test]
fn leading_pipe() {
    assert_eq!(
        parse(vec![Token::Pipe, w("ls")]),
        Err(ParseError::Syntax("|".into()))
    );
}

#[test]
fn trailing_pipe() {
    assert_eq!(
        parse(vec![w("ls"), Token::Pipe]),
        Err(ParseError::Syntax("|".into()))
    );
}

#[test]
fn consecutive_pipes() {
    assert_eq!(
        parse(vec![w("ls"), Token::Pipe, Token::Pipe, w("wc")]),
        Err(ParseError::Syntax("|".into()))
    );
}

#[test]
fn pipe_with_redirs() {
    let cmds = parse(vec![
        w("cat"),
        Token::RedirIn,
        w("in"),
        Token::Pipe,
        w("wc"),
        Token::RedirOut,
        w("out"),
    ])
    .unwrap();
    assert_eq!(cmds.len(), 2);
    assert_eq!(
        cmds[0].redirs,
        vec![Redir {
            kind: RedirKind::In,
            target: vec![Segment::Plain("in".into())],
            quoted: false
        }]
    );
    assert_eq!(
        cmds[1].redirs,
        vec![Redir {
            kind: RedirKind::Out,
            target: vec![Segment::Plain("out".into())],
            quoted: false
        }]
    );
}

#[test]
fn empty_tokens() {
    let cmds = parse(vec![]).unwrap();
    assert_eq!(cmds.len(), 0);
}
