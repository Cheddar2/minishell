use super::path::*;

#[test]
fn resolve_finds_command_on_path() {
    assert!(resolve("ls", Some("/usr/bin:/bin")).is_ok());
}

#[test]
fn resolve_missing_path_entry_is_not_found() {
    assert_eq!(
        resolve("ls", Some("/nonexistent")),
        Err(PathError::NotFound("ls".into()))
    );
}

#[test]
fn resolve_without_path_var_is_not_found() {
    assert_eq!(resolve("ls", None), Err(PathError::NotFound("ls".into())));
}

#[test]
fn resolve_empty_command_is_not_found() {
    assert_eq!(
        resolve("", Some("/bin")),
        Err(PathError::NotFound(String::new()))
    );
}

#[test]
fn resolve_explicit_path_is_ok() {
    assert!(resolve("/bin/ls", None).is_ok());
}

#[test]
fn resolve_explicit_missing_path_is_no_such_file() {
    assert_eq!(
        resolve("/nope/x", None),
        Err(PathError::NoSuchFile("/nope/x".into()))
    );
}

#[test]
fn resolve_explicit_directory_is_dir() {
    assert_eq!(resolve("/tmp", None), Err(PathError::IsDir("/tmp".into())));
}

#[test]
fn status_codes_match_bash() {
    assert_eq!(PathError::NotFound("ls".into()).status(), 127);
    assert_eq!(PathError::IsDir("/tmp".into()).status(), 126);
}
