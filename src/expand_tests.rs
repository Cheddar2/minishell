use super::*;

#[test]
fn plain_var_expands() {
    let mut env = Env::new();
    env.set("X", Some("hi".into()), false);
    let segs = vec![Segment::Plain("$X".into())];
    assert_eq!(expand_word(&segs, &env, 0), Some("hi".into()));
}

#[test]
fn single_quoted_is_literal() {
    let mut env = Env::new();
    env.set("X", Some("hi".into()), false);
    let segs = vec![Segment::Single("$X".into())];
    assert_eq!(expand_word(&segs, &env, 0), Some("$X".into()));
}

#[test]
fn plain_unset_empty_dropped() {
    let env = Env::new();
    let segs = vec![Segment::Plain("$UNSET".into())];
    assert_eq!(expand_word(&segs, &env, 0), None);
}

#[test]
fn double_unset_stays_empty() {
    let env = Env::new();
    let segs = vec![Segment::Double("$UNSET".into())];
    assert_eq!(expand_word(&segs, &env, 0), Some("".into()));
}

#[test]
fn dollar_question_expands_last_status() {
    let env = Env::new();
    let segs = vec![Segment::Plain("$?".into())];
    assert_eq!(expand_word(&segs, &env, 130), Some("130".into()));
}

#[test]
fn plain_unset_with_literal() {
    let env = Env::new();
    let segs = vec![Segment::Plain("$UNSET.".into())];
    assert_eq!(expand_word(&segs, &env, 0), Some(".".into()));
}

#[test]
fn dollar_question_twice() {
    let env = Env::new();
    let segs = vec![Segment::Plain("$?$?".into())];
    assert_eq!(expand_word(&segs, &env, 0), Some("00".into()));
}

#[test]
fn plain_bare_dollar() {
    let env = Env::new();
    let segs = vec![Segment::Plain("$".into())];
    assert_eq!(expand_word(&segs, &env, 0), Some("$".into()));
}

#[test]
fn double_bare_dollar() {
    let env = Env::new();
    let segs = vec![Segment::Double("$".into())];
    assert_eq!(expand_word(&segs, &env, 0), Some("$".into()));
}

#[test]
fn plain_positional_param_expands_empty() {
    let env = Env::new();
    let segs = vec![Segment::Plain("$1abc".into())];
    assert_eq!(expand_word(&segs, &env, 0), Some("abc".into()));
}

#[test]
fn plain_var_with_underscore() {
    let mut env = Env::new();
    env.set("X", Some("hi".into()), false);
    env.set("Y", Some("yo".into()), false);
    env.set("X_Y", Some("under".into()), false);
    let segs = vec![Segment::Plain("$X_Y".into())];
    assert_eq!(expand_word(&segs, &env, 0), Some("under".into()));
}

#[test]
fn plain_var_stops_at_dash() {
    let mut env = Env::new();
    env.set("X", Some("hi".into()), false);
    env.set("Y", Some("yo".into()), false);
    env.set("X_Y", Some("under".into()), false);
    let segs = vec![Segment::Plain("$X-Y".into())];
    assert_eq!(expand_word(&segs, &env, 0), Some("hi-Y".into()));
}

#[test]
fn multiple_segments_concatenated() {
    let mut env = Env::new();
    env.set("X", Some("hi".into()), false);
    env.set("Y", Some("yo".into()), false);
    env.set("X_Y", Some("under".into()), false);
    let segs = vec![
        Segment::Plain("a".into()),
        Segment::Double("b $X".into()),
        Segment::Single("c $X".into()),
    ];
    assert_eq!(expand_word(&segs, &env, 0), Some("ab hic $X".into()));
}

#[test]
fn adjacent_vars_expand() {
    let mut env = Env::new();
    env.set("X", Some("hi".into()), false);
    env.set("Y", Some("yo".into()), false);
    env.set("X_Y", Some("under".into()), false);
    let segs = vec![Segment::Plain("$X$Y".into())];
    assert_eq!(expand_word(&segs, &env, 0), Some("hiyo".into()));
}

#[test]
fn double_empty_segment_kept() {
    let env = Env::new();
    let segs = vec![Segment::Double("".into())];
    assert_eq!(expand_word(&segs, &env, 0), Some("".into()));
}

#[test]
fn var_with_space_not_split() {
    let mut env = Env::new();
    env.set("X", Some("a b".into()), false);
    env.set("Y", Some("yo".into()), false);
    env.set("X_Y", Some("under".into()), false);
    let segs = vec![Segment::Plain("$X".into())];
    assert_eq!(expand_word(&segs, &env, 0), Some("a b".into()));
}

#[test]
fn split_literal_spaces() {
    let mut env = Env::new();
    env.set("A", Some("x   y".into()), false);
    let segs = vec![Segment::Plain("$A".into())];
    assert_eq!(
        expand_word_split(&segs, &env, 0),
        vec!["x".to_string(), "y".to_string()]
    );
}

#[test]
fn quoted_spaces_not_split() {
    let env = Env::new();
    let segs = vec![Segment::Double("x   y".into())];
    assert_eq!(expand_word_split(&segs, &env, 0), vec!["x   y".to_string()]);
}

#[test]
fn split_expansion_prefix() {
    let mut env = Env::new();
    env.set("A", Some("a b".into()), false);
    let segs = vec![Segment::Plain("x$A".into())];
    assert_eq!(
        expand_word_split(&segs, &env, 0),
        vec!["xa".to_string(), "b".to_string()]
    );
}

#[test]
fn quoted_joins_last_split_piece() {
    let mut env = Env::new();
    env.set("A", Some("a b".into()), false);
    let segs = vec![Segment::Plain("$A".into()), Segment::Single("q".into())];
    assert_eq!(
        expand_word_split(&segs, &env, 0),
        vec!["a".to_string(), "bq".to_string()]
    );
}

#[test]
fn whitespace_only_expansion_dropped() {
    let mut env = Env::new();
    env.set("A", Some("  ".into()), false);
    let segs = vec![Segment::Plain("[$A]".into())];
    assert_eq!(
        expand_word_split(&segs, &env, 0),
        vec!["[".to_string(), "]".to_string()]
    );
}

#[test]
fn empty_expansion_alone_no_words() {
    let mut env = Env::new();
    env.set("A", Some("".into()), false);
    let segs = vec![Segment::Plain("$A".into())];
    assert_eq!(expand_word_split(&segs, &env, 0), Vec::<String>::new());
}

#[test]
fn quoted_last_status_then_literal() {
    let env = Env::new();
    let segs = vec![Segment::Double("$?".into()), Segment::Plain("x".into())];
    assert_eq!(expand_word_split(&segs, &env, 0), vec!["0x".to_string()]);
}

#[test]
fn regression_mixed_quotes() {
    let mut env = Env::new();
    env.set("HOME", Some("/home/adam".into()), false);
    let segs = vec![
        Segment::Double("$HOME".into()),
        Segment::Plain("x".into()),
        Segment::Single("$HOME".into()),
        Segment::Double("$NOPE".into()),
        Segment::Plain("y".into()),
    ];
    assert_eq!(
        expand_word_split(&segs, &env, 0),
        vec!["/home/adamx$HOMEy".to_string()]
    );
}
