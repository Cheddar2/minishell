use super::*;

fn w(s: &str) -> Token {
    Token::Word(vec![Segment::Plain(s.into())])
}

/// Parse a single-command line and return its `SimpleCmd`.
fn cmd_of(tokens: Vec<Token>) -> SimpleCmd {
    match parse(tokens).unwrap() {
        Tree::Pipeline(elems) => match elems.into_iter().next().unwrap() {
            Elem::Cmd(cmd) => cmd,
            other => panic!("expected a simple command, got {other:?}"),
        },
        other => panic!("expected a pipeline, got {other:?}"),
    }
}

/// Parse a pipeline line and return its elements.
fn elems_of(tokens: Vec<Token>) -> Vec<Elem> {
    match parse(tokens).unwrap() {
        Tree::Pipeline(elems) => elems,
        other => panic!("expected a pipeline, got {other:?}"),
    }
}

/// Parse a line and return its full tree.
fn tree_of(tokens: Vec<Token>) -> Tree {
    parse(tokens).unwrap()
}

/// Assert that parsing fails with a syntax error on the given token.
fn err_token(tokens: Vec<Token>) -> String {
    match parse(tokens) {
        Ok(t) => panic!("expected a parse error, got {t:?}"),
        Err(ParseError::Syntax(t)) => t,
    }
}

fn cmd(toks: Vec<Token>) -> Elem {
    match parse_simple(toks) {
        Ok(c) => Elem::Cmd(c),
        Err(e) => panic!("parse_simple: {e:?}"),
    }
}

#[test]
fn two_words() {
    let cmd = cmd_of(vec![w("ls"), w("-l")]);
    assert_eq!(cmd.argv.len(), 2);
    assert!(cmd.redirs.is_empty());
}

#[test]
fn redir_in() {
    let cmd = cmd_of(vec![w("cat"), Token::RedirIn, w("in")]);
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
    let cmd = cmd_of(vec![w("ls"), Token::RedirOut, w("out")]);
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
    let cmd = cmd_of(vec![w("echo"), w("hi"), Token::Append, w("log")]);
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
    let cmd = cmd_of(vec![Token::RedirIn, w("in"), w("cat")]);
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
    let cmd = cmd_of(vec![
        w("cat"),
        Token::RedirIn,
        w("a"),
        Token::RedirIn,
        w("b"),
    ]);
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
    let cmd = cmd_of(vec![
        w("echo"),
        w("a"),
        Token::RedirOut,
        w("f1"),
        Token::RedirOut,
        w("f2"),
    ]);
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
    assert_eq!(err_token(vec![Token::RedirOut]), "newline");
}

#[test]
fn redir_followed_by_operator() {
    assert_eq!(err_token(vec![Token::RedirIn, Token::RedirOut]), ">");
}

#[test]
fn pipe_not_supported() {
    assert_eq!(err_token(vec![Token::Pipe]), "|");
}

#[test]
fn heredoc_plain() {
    let cmd = cmd_of(vec![w("cat"), Token::Heredoc, w("EOF")]);
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
    let cmd = cmd_of(vec![
        w("cat"),
        Token::Heredoc,
        Token::Word(vec![Segment::Single("EOF".into())]),
    ]);
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
    let cmd = cmd_of(vec![
        w("cat"),
        Token::Heredoc,
        Token::Word(vec![Segment::Double("EOF".into())]),
    ]);
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
    let cmd = cmd_of(vec![
        w("cat"),
        Token::Heredoc,
        Token::Word(vec![
            Segment::Double("E".into()),
            Segment::Plain("OF".into()),
        ]),
    ]);
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
    assert_eq!(err_token(vec![Token::Heredoc]), "newline");
}

#[test]
fn heredoc_followed_by_operator() {
    assert_eq!(
        err_token(vec![w("cat"), Token::Heredoc, Token::RedirOut]),
        ">"
    );
}

#[test]
fn pipe_two_commands() {
    let cmds = elems_of(vec![w("ls"), Token::Pipe, w("wc")]);
    assert_eq!(cmds.len(), 2);
}

#[test]
fn pipe_three_commands() {
    let cmds = elems_of(vec![w("a"), Token::Pipe, w("b"), Token::Pipe, w("c")]);
    assert_eq!(cmds.len(), 3);
}

#[test]
fn leading_pipe() {
    assert_eq!(err_token(vec![Token::Pipe, w("ls")]), "|");
}

#[test]
fn trailing_pipe() {
    assert_eq!(err_token(vec![w("ls"), Token::Pipe]), "|");
}

#[test]
fn consecutive_pipes() {
    assert_eq!(
        err_token(vec![w("ls"), Token::Pipe, Token::Pipe, w("wc")]),
        "|"
    );
}

#[test]
fn pipe_with_redirs() {
    let cmds = elems_of(vec![
        w("cat"),
        Token::RedirIn,
        w("in"),
        Token::Pipe,
        w("wc"),
        Token::RedirOut,
        w("out"),
    ]);
    assert_eq!(cmds.len(), 2);
    match &cmds[0] {
        Elem::Cmd(c) => {
            assert_eq!(
                c.redirs,
                vec![Redir {
                    kind: RedirKind::In,
                    target: vec![Segment::Plain("in".into())],
                    quoted: false
                }]
            );
        }
        other => panic!("expected a simple command, got {other:?}"),
    }
    match &cmds[1] {
        Elem::Cmd(c) => {
            assert_eq!(
                c.redirs,
                vec![Redir {
                    kind: RedirKind::Out,
                    target: vec![Segment::Plain("out".into())],
                    quoted: false
                }]
            );
        }
        other => panic!("expected a simple command, got {other:?}"),
    }
}

#[test]
fn empty_tokens() {
    assert_eq!(parse(vec![]).unwrap(), Tree::Empty);
}

// --- M10: &&, || and ( ) -------------------------------------------------

#[test]
fn plain_pipeline_is_plain() {
    assert!(is_plain_pipeline(&tree_of(vec![
        w("ls"),
        Token::Pipe,
        w("wc")
    ])));
    assert!(is_plain_pipeline(&Tree::Empty));
}

#[test]
fn and_list_shape() {
    assert_eq!(
        tree_of(vec![w("a"), Token::And, w("b")]),
        Tree::List(
            Box::new(Tree::Pipeline(vec![cmd(vec![w("a")])])),
            ListOp::And,
            Box::new(Tree::Pipeline(vec![cmd(vec![w("b")])]))
        )
    );
}

#[test]
fn or_list_shape() {
    assert_eq!(
        tree_of(vec![w("a"), Token::Or, w("b")]),
        Tree::List(
            Box::new(Tree::Pipeline(vec![cmd(vec![w("a")])])),
            ListOp::Or,
            Box::new(Tree::Pipeline(vec![cmd(vec![w("b")])]))
        )
    );
}

#[test]
fn and_or_same_precedence_left_to_right() {
    // `a || b && c` means `(a || b) && c`.
    assert_eq!(
        tree_of(vec![w("a"), Token::Or, w("b"), Token::And, w("c")]),
        Tree::List(
            Box::new(Tree::List(
                Box::new(Tree::Pipeline(vec![cmd(vec![w("a")])])),
                ListOp::Or,
                Box::new(Tree::Pipeline(vec![cmd(vec![w("b")])]))
            )),
            ListOp::And,
            Box::new(Tree::Pipeline(vec![cmd(vec![w("c")])]))
        )
    );
}

#[test]
fn pipe_binds_tighter_than_and_or() {
    // `a | b && c` means `(a | b) && c`.
    assert_eq!(
        tree_of(vec![w("a"), Token::Pipe, w("b"), Token::And, w("c")]),
        Tree::List(
            Box::new(Tree::Pipeline(vec![cmd(vec![w("a")]), cmd(vec![w("b")])])),
            ListOp::And,
            Box::new(Tree::Pipeline(vec![cmd(vec![w("c")])]))
        )
    );
}

#[test]
fn and_and_left_associative() {
    // `a && b && c` means `(a && b) && c`.
    assert_eq!(
        tree_of(vec![w("a"), Token::And, w("b"), Token::And, w("c")]),
        Tree::List(
            Box::new(Tree::List(
                Box::new(Tree::Pipeline(vec![cmd(vec![w("a")])])),
                ListOp::And,
                Box::new(Tree::Pipeline(vec![cmd(vec![w("b")])]))
            )),
            ListOp::And,
            Box::new(Tree::Pipeline(vec![cmd(vec![w("c")])]))
        )
    );
}

#[test]
fn group_as_pipeline_element() {
    // `(echo a && echo b) | cat`
    let elems = elems_of(vec![
        Token::LParen,
        w("echo"),
        w("a"),
        Token::And,
        w("echo"),
        w("b"),
        Token::RParen,
        Token::Pipe,
        w("cat"),
    ]);
    assert_eq!(elems.len(), 2);
    match &elems[0] {
        Elem::Group { tree, redirs } => {
            assert!(redirs.is_empty());
            assert_eq!(
                *tree,
                Tree::List(
                    Box::new(Tree::Pipeline(vec![cmd(vec![w("echo"), w("a")])])),
                    ListOp::And,
                    Box::new(Tree::Pipeline(vec![cmd(vec![w("echo"), w("b")])]))
                )
            );
        }
        other => panic!("expected a group, got {other:?}"),
    }
    assert!(matches!(elems[1], Elem::Cmd(_)));
}

#[test]
fn group_takes_redirs_after_paren() {
    // `(echo a && echo b) > f`
    let tree = tree_of(vec![
        Token::LParen,
        w("echo"),
        w("a"),
        Token::And,
        w("echo"),
        w("b"),
        Token::RParen,
        Token::RedirOut,
        w("f"),
    ]);
    match tree {
        Tree::Pipeline(elems) => match elems.into_iter().next().unwrap() {
            Elem::Group { redirs, .. } => {
                assert_eq!(
                    redirs,
                    vec![Redir {
                        kind: RedirKind::Out,
                        target: vec![Segment::Plain("f".into())],
                        quoted: false
                    }]
                );
            }
            other => panic!("expected a group, got {other:?}"),
        },
        other => panic!("expected a pipeline, got {other:?}"),
    }
}

#[test]
fn nested_groups() {
    let tree = tree_of(vec![
        Token::LParen,
        Token::LParen,
        w("a"),
        Token::RParen,
        Token::RParen,
    ]);
    match tree {
        Tree::Pipeline(elems) => match elems.into_iter().next().unwrap() {
            Elem::Group { tree, .. } => match tree {
                Tree::Pipeline(elems) => match elems.into_iter().next().unwrap() {
                    Elem::Group { tree, .. } => {
                        assert_eq!(tree, Tree::Pipeline(vec![cmd(vec![w("a")])]));
                    }
                    Elem::Cmd(_) => panic!("expected a group, got a simple command"),
                },
                other => panic!("expected a pipeline, got {other:?}"),
            },
            other => panic!("expected a group, got {other:?}"),
        },
        other => panic!("expected a pipeline, got {other:?}"),
    }
}

#[test]
fn group_with_heredoc_inside() {
    let tree = tree_of(vec![
        Token::LParen,
        w("cat"),
        Token::Heredoc,
        w("EOF"),
        Token::RParen,
    ]);
    match tree {
        Tree::Pipeline(elems) => match elems.into_iter().next().unwrap() {
            Elem::Group { tree, .. } => match tree {
                Tree::Pipeline(elems) => match elems.into_iter().next().unwrap() {
                    Elem::Cmd(c) => {
                        assert_eq!(c.argv, vec![vec![Segment::Plain("cat".into())]]);
                        assert_eq!(c.redirs.len(), 1);
                    }
                    Elem::Group { .. } => panic!("expected a simple command, got a group"),
                },
                other => panic!("expected a pipeline, got {other:?}"),
            },
            other => panic!("expected a group, got {other:?}"),
        },
        other => panic!("expected a pipeline, got {other:?}"),
    }
}

#[test]
fn not_plain_when_listed() {
    assert!(!is_plain_pipeline(&tree_of(vec![
        w("a"),
        Token::And,
        w("b")
    ])));
    assert!(!is_plain_pipeline(&tree_of(vec![
        Token::LParen,
        w("a"),
        Token::RParen,
        Token::Pipe,
        w("cat")
    ])));
}

#[test]
fn leading_and_is_error() {
    assert_eq!(err_token(vec![Token::And, w("b")]), "&&");
}

#[test]
fn leading_or_is_error() {
    assert_eq!(err_token(vec![Token::Or, w("b")]), "||");
}

#[test]
fn leading_rparen_is_error() {
    assert_eq!(err_token(vec![Token::RParen]), ")");
}

#[test]
fn two_operators_in_a_row() {
    assert_eq!(err_token(vec![w("a"), Token::And, Token::Or, w("b")]), "||");
}

#[test]
fn trailing_and_is_error() {
    assert_eq!(err_token(vec![w("a"), Token::And]), "newline");
}

#[test]
fn trailing_or_is_error() {
    assert_eq!(err_token(vec![w("a"), Token::Or]), "newline");
}

#[test]
fn empty_group_is_error() {
    assert_eq!(err_token(vec![Token::LParen, Token::RParen]), ")");
}

#[test]
fn unbalanced_lparen_is_error() {
    assert_eq!(err_token(vec![Token::LParen, w("a")]), "newline");
}

#[test]
fn unbalanced_rparen_is_error() {
    assert_eq!(err_token(vec![w("a"), Token::RParen]), ")");
}

#[test]
fn word_after_group_is_error() {
    assert_eq!(
        err_token(vec![Token::LParen, w("a"), Token::RParen, w("b")]),
        "b"
    );
}
