use super::*;

fn w(s: &str) -> Token {
    Token::Word(vec![Segment::Plain(s.into())])
}

#[test]
fn two_words() {
    let cmd = parse(vec![w("ls"), w("-l")]).unwrap();
    assert_eq!(cmd.argv.len(), 2);
    assert!(cmd.redirs.is_empty());
}

#[test]
fn redir_in() {
    let cmd = parse(vec![w("cat"), Token::RedirIn, w("in")]).unwrap();
    assert_eq!(cmd.argv, vec![vec![Segment::Plain("cat".into())]]);
    assert_eq!(
        cmd.redirs,
        vec![Redir {
            kind: RedirKind::In,
            target: vec![Segment::Plain("in".into())]
        }]
    );
}

#[test]
fn redir_out() {
    let cmd = parse(vec![w("ls"), Token::RedirOut, w("out")]).unwrap();
    assert_eq!(
        cmd.redirs,
        vec![Redir {
            kind: RedirKind::Out,
            target: vec![Segment::Plain("out".into())]
        }]
    );
}

#[test]
fn redir_append() {
    let cmd = parse(vec![w("echo"), w("hi"), Token::Append, w("log")]).unwrap();
    assert_eq!(cmd.argv.len(), 2);
    assert_eq!(
        cmd.redirs,
        vec![Redir {
            kind: RedirKind::Append,
            target: vec![Segment::Plain("log".into())]
        }]
    );
}

#[test]
fn redir_before_command() {
    let cmd = parse(vec![Token::RedirIn, w("in"), w("cat")]).unwrap();
    assert_eq!(cmd.argv, vec![vec![Segment::Plain("cat".into())]]);
    assert_eq!(
        cmd.redirs,
        vec![Redir {
            kind: RedirKind::In,
            target: vec![Segment::Plain("in".into())]
        }]
    );
}

#[test]
fn two_redir_in_in_order() {
    let cmd = parse(vec![
        w("cat"),
        Token::RedirIn,
        w("a"),
        Token::RedirIn,
        w("b"),
    ])
    .unwrap();
    assert_eq!(
        cmd.redirs,
        vec![
            Redir {
                kind: RedirKind::In,
                target: vec![Segment::Plain("a".into())]
            },
            Redir {
                kind: RedirKind::In,
                target: vec![Segment::Plain("b".into())]
            }
        ]
    );
}

#[test]
fn two_redir_out_in_order() {
    let cmd = parse(vec![
        w("echo"),
        w("a"),
        Token::RedirOut,
        w("f1"),
        Token::RedirOut,
        w("f2"),
    ])
    .unwrap();
    assert_eq!(
        cmd.redirs,
        vec![
            Redir {
                kind: RedirKind::Out,
                target: vec![Segment::Plain("f1".into())]
            },
            Redir {
                kind: RedirKind::Out,
                target: vec![Segment::Plain("f2".into())]
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
fn heredoc_not_supported() {
    assert_eq!(
        parse(vec![w("cat"), Token::Heredoc, w("x")]),
        Err(ParseError::Syntax("<<".into()))
    );
}
