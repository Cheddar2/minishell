use super::*;

#[test]
fn set_then_get() {
    let mut env = Env::new();
    env.set("HOME", Some("/home/adam".to_string()), true);
    assert_eq!(env.get("HOME"), Some("/home/adam"));
}

#[test]
fn overwrite_keeps_order() {
    let mut env = Env::new();
    env.set("A", Some("1".to_string()), true);
    env.set("B", Some("2".to_string()), true);
    env.set("A", Some("3".to_string()), true);
    let names: Vec<&str> = env.vars.iter().map(|v| v.name.as_str()).collect();
    assert_eq!(names, vec!["A", "B"]);
    assert_eq!(env.get("A"), Some("3"));
    assert_eq!(env.get("B"), Some("2"));
}

#[test]
fn set_none_keeps_old_value() {
    let mut env = Env::new();
    env.set("X", Some("old".to_string()), true);
    env.set("X", None, false);
    assert_eq!(env.get("X"), Some("old"));
    let var = env.vars.iter().find(|v| v.name == "X").expect("X exists");
    assert!(!var.exported);
}

#[test]
fn set_none_on_new_variable() {
    let mut env = Env::new();
    env.set("X", None, true);
    assert_eq!(env.vars.len(), 1);
    assert_eq!(env.get("X"), None);
}

#[test]
fn unset_removes_variable() {
    let mut env = Env::new();
    env.set("A", Some("1".to_string()), true);
    env.set("B", Some("2".to_string()), true);
    env.unset("A");
    assert_eq!(env.get("A"), None);
    assert_eq!(env.get("B"), Some("2"));
    assert_eq!(env.vars.len(), 1);
}

#[test]
fn unset_missing_is_fine() {
    let mut env = Env::new();
    env.unset("NOPE");
    assert_eq!(env.vars.len(), 0);
}

#[test]
fn get_none_value_is_none() {
    let mut env = Env::new();
    env.set("X", None, true);
    assert_eq!(env.get("X"), None);
}

#[test]
fn to_envp_skips_unexported_and_none() {
    let mut env = Env::new();
    env.set("A", Some("1".to_string()), true);
    env.set("B", Some("2".to_string()), false);
    env.set("C", None, true);
    env.set("D", Some("4".to_string()), true);
    let envp: Vec<String> = env
        .to_envp()
        .into_iter()
        .map(|c| c.into_string().unwrap())
        .collect();
    assert_eq!(envp, vec!["A=1", "D=4"]);
}

#[test]
fn to_envp_keeps_insertion_order() {
    let mut env = Env::new();
    env.set("Z", Some("z".to_string()), true);
    env.set("A", Some("a".to_string()), true);
    let envp: Vec<String> = env
        .to_envp()
        .into_iter()
        .map(|c| c.into_string().unwrap())
        .collect();
    assert_eq!(envp, vec!["Z=z", "A=a"]);
}

#[test]
fn is_valid_name_accepts_valid() {
    assert!(is_valid_name("A"));
    assert!(is_valid_name("_x1"));
    assert!(is_valid_name("a_b"));
    assert!(is_valid_name("_"));
    assert!(is_valid_name("SHLVL"));
}

#[test]
fn is_valid_name_rejects_invalid() {
    assert!(!is_valid_name(""));
    assert!(!is_valid_name("1A"));
    assert!(!is_valid_name("a-b"));
    assert!(!is_valid_name("a b"));
    assert!(!is_valid_name("-x"));
    assert!(!is_valid_name("a$b"));
}

#[test]
fn next_shlvl_bumps() {
    assert_eq!(next_shlvl(Some("1")), "2");
    assert_eq!(next_shlvl(None), "1");
    assert_eq!(next_shlvl(Some("abc")), "1");
    assert_eq!(next_shlvl(Some("42")), "43");
    assert_eq!(next_shlvl(Some("")), "1");
}

#[test]
fn from_process_has_shlvl_at_least_one() {
    let env = Env::from_process();
    let shlvl = env.get("SHLVL").expect("SHLVL exists");
    let n: i32 = shlvl.parse().expect("SHLVL is an integer");
    assert!(n >= 1);
}
