//! Command execution support (M4).

pub mod path;
pub mod redirect;

#[cfg(test)]
#[path = "path_tests.rs"]
mod tests;
