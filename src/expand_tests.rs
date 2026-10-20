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
