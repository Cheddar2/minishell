use super::*;

fn w(s: &str) -> Token {
    Token::Word(vec![Segment::Plain(s.to_string())])
}

fn p(s: &str) -> Segment {
    Segment::Plain(s.to_string())
}

fn d(s: &str) -> Segment {
    Segment::Double(s.to_string())
}

fn s(s: &str) -> Segment {
    Segment::Single(s.to_string())
}

fn ws(segments: Vec<Segment>) -> Token {
    Token::Word(segments)
}

#[test]
fn lex_simple_command() {
    let tokens = lex("ls -l").expect("lex should succeed");
    assert_eq!(tokens, vec![w("ls"), w("-l")]);
}

#[test]
fn lex_empty_input() {
    let tokens = lex("").expect("lex should succeed");
    assert!(tokens.is_empty());
}

#[test]
fn lex_pipe() {
    let tokens = lex("ls|wc").expect("lex should succeed");
    assert_eq!(tokens, vec![w("ls"), Token::Pipe, w("wc")]);
}

#[test]
fn lex_redirects() {
    let tokens = lex("cat < in > out").expect("lex should succeed");
    assert_eq!(
        tokens,
        vec![w("cat"), Token::RedirIn, w("in"), Token::RedirOut, w("out")]
    );
}

#[test]
fn lex_append() {
    let tokens = lex("echo a>>f").expect("lex should succeed");
    assert_eq!(tokens, vec![w("echo"), w("a"), Token::Append, w("f")]);
}

#[test]
fn lex_heredoc() {
    let tokens = lex("cat<<EOF").expect("lex should succeed");
    assert_eq!(tokens, vec![w("cat"), Token::Heredoc, w("EOF")]);
}

#[test]
fn lex_and() {
    let tokens = lex("a && b").expect("lex should succeed");
    assert_eq!(tokens, vec![w("a"), Token::And, w("b")]);
}

#[test]
fn lex_or() {
    let tokens = lex("a || b").expect("lex should succeed");
    assert_eq!(tokens, vec![w("a"), Token::Or, w("b")]);
}

#[test]
fn lex_or_then_pipe() {
    let tokens = lex("a|||b").expect("lex should succeed");
    assert_eq!(tokens, vec![w("a"), Token::Or, Token::Pipe, w("b")]);
}

#[test]
fn lex_triple_chevrons() {
    let tokens = lex("<<<").expect("lex should succeed");
    assert_eq!(tokens, vec![Token::Heredoc, Token::RedirIn]);
    let tokens = lex(">>>").expect("lex should succeed");
    assert_eq!(tokens, vec![Token::Append, Token::RedirOut]);
}

#[test]
fn lex_parentheses_and_ops() {
    let tokens = lex("( a || b ) && c").expect("lex should succeed");
    assert_eq!(
        tokens,
        vec![
            Token::LParen,
            w("a"),
            Token::Or,
            w("b"),
            Token::RParen,
            Token::And,
            w("c")
        ]
    );
}

#[test]
fn lex_single_ampersand_is_word() {
    let tokens = lex("echo &").expect("lex should succeed");
    assert_eq!(tokens, vec![w("echo"), w("&")]);
}

#[test]
fn lex_semicolon_is_word_char() {
    let tokens = lex("a;b").expect("lex should succeed");
    assert_eq!(tokens, vec![w("a;b")]);
}

#[test]
fn lex_extra_whitespace() {
    let tokens = lex("  ls   -l  ").expect("lex should succeed");
    assert_eq!(tokens, vec![w("ls"), w("-l")]);
}

#[test]
fn lex_single_quoted_word() {
    let tokens = lex("'a b'").expect("lex should succeed");
    assert_eq!(tokens, vec![ws(vec![s("a b")])]);
}

#[test]
fn lex_double_quoted_word() {
    let tokens = lex("\"a b\"").expect("lex should succeed");
    assert_eq!(tokens, vec![ws(vec![d("a b")])]);
}

#[test]
fn lex_dollar_in_double_quotes_is_literal() {
    let tokens = lex("\"$HOME\"").expect("lex should succeed");
    assert_eq!(tokens, vec![ws(vec![d("$HOME")])]);
}

#[test]
fn lex_mixed_segments_in_one_word() {
    let tokens = lex("a\"b c\"'d'").expect("lex should succeed");
    assert_eq!(tokens, vec![ws(vec![p("a"), d("b c"), s("d")])]);
}

#[test]
fn lex_empty_double_quotes() {
    let tokens = lex("\"\"").expect("lex should succeed");
    assert_eq!(tokens, vec![ws(vec![d("")])]);
}

#[test]
fn lex_empty_single_quotes() {
    let tokens = lex("''").expect("lex should succeed");
    assert_eq!(tokens, vec![ws(vec![s("")])]);
}

#[test]
fn lex_two_empty_quoted_words() {
    let tokens = lex("\"\" ''").expect("lex should succeed");
    assert_eq!(tokens, vec![ws(vec![d("")]), ws(vec![s("")])]);
}

#[test]
fn lex_pipe_inside_double_quotes() {
    let tokens = lex("echo \"a|b\"").expect("lex should succeed");
    assert_eq!(tokens, vec![w("echo"), ws(vec![d("a|b")])]);
}

#[test]
fn lex_redirect_inside_single_quotes() {
    let tokens = lex("echo 'a>b'").expect("lex should succeed");
    assert_eq!(tokens, vec![w("echo"), ws(vec![s("a>b")])]);
}

#[test]
fn lex_single_quote_inside_double_quotes() {
    let tokens = lex("echo \"it's\"").expect("lex should succeed");
    assert_eq!(tokens, vec![w("echo"), ws(vec![d("it's")])]);
}

#[test]
fn lex_double_quote_inside_single_quotes() {
    let tokens = lex("echo 'say \"hi\"'").expect("lex should succeed");
    assert_eq!(tokens, vec![w("echo"), ws(vec![s("say \"hi\"")])]);
}

#[test]
fn lex_adjacent_quotes_and_plain() {
    let tokens = lex("\"a\"b\"c\"").expect("lex should succeed");
    assert_eq!(tokens, vec![ws(vec![d("a"), p("b"), d("c")])]);
}

#[test]
fn lex_unclosed_single_quote() {
    assert_eq!(lex("'unclosed"), Err(LexError::UnclosedQuote('\'')));
}

#[test]
fn lex_unclosed_double_quote() {
    assert_eq!(lex("\"unclosed"), Err(LexError::UnclosedQuote('"')));
}

#[test]
fn lex_single_quote_inside_double_does_not_close_it() {
    assert_eq!(lex("\"a'b"), Err(LexError::UnclosedQuote('"')));
}

#[test]
fn lex_unclosed_single_quote_after_closed_double() {
    assert_eq!(lex("echo \"ok\" 'bad"), Err(LexError::UnclosedQuote('\'')));
}

#[test]
fn lex_whitespace_only() {
    let tokens = lex("   ").expect("lex should succeed");
    assert!(tokens.is_empty());
}

#[test]
fn lex_command_with_path() {
    let tokens = lex("ls -l /tmp").expect("lex should succeed");
    assert_eq!(tokens, vec![w("ls"), w("-l"), w("/tmp")]);
}

#[test]
fn lex_pipe_with_spaces() {
    let tokens = lex("ls | wc -l").expect("lex should succeed");
    assert_eq!(tokens, vec![w("ls"), Token::Pipe, w("wc"), w("-l")]);
}

#[test]
fn lex_tight_parentheses() {
    let tokens = lex("(a)").expect("lex should succeed");
    assert_eq!(tokens, vec![Token::LParen, w("a"), Token::RParen]);
}

#[test]
fn lex_backslash_is_plain_word_char() {
    let tokens = lex("a\\b").expect("lex should succeed");
    assert_eq!(tokens, vec![ws(vec![p("a\\b")])]);
}

#[test]
fn lex_bare_gt() {
    let tokens = lex(">").expect("lex should succeed");
    assert_eq!(tokens, vec![Token::RedirOut]);
}

#[test]
fn lex_dollar_variable_in_word() {
    let tokens = lex("echo $USER").expect("lex should succeed");
    assert_eq!(tokens, vec![w("echo"), w("$USER")]);
}

#[test]
fn lex_tab_separates_words() {
    let tokens = lex("ls\t-l").expect("lex should succeed");
    assert_eq!(tokens, vec![w("ls"), w("-l")]);
}

#[test]
fn lex_newline_separates_words() {
    let tokens = lex("a\nb").expect("lex should succeed");
    assert_eq!(tokens, vec![w("a"), w("b")]);
}

#[test]
fn lex_double_quoted_then_plain() {
    let tokens = lex("echo \"a b\"c").expect("lex should succeed");
    assert_eq!(tokens, vec![w("echo"), ws(vec![d("a b"), p("c")])]);
}

#[test]
fn lex_quoted_heredoc_delimiter() {
    let tokens = lex("cat <<'EOF'").expect("lex should succeed");
    assert_eq!(tokens, vec![w("cat"), Token::Heredoc, ws(vec![s("EOF")])]);
}

#[test]
fn lex_two_adjacent_empty_double_quotes() {
    let tokens = lex("\"\"\"\"").expect("lex should succeed");
    assert_eq!(tokens, vec![ws(vec![d(""), d("")])]);
}
