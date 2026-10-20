use std::io::IsTerminal;

use crate::env::Env;

/// Top-level shell state.
pub struct Shell {
    /// Exit status of the last command.
    pub last_status: i32,
    /// True when stdin is a terminal.
    pub interactive: bool,
    /// Environment table (read by later M3 steps).
    pub env: Env,
}

impl Shell {
    pub fn new() -> Self {
        Self {
            last_status: 0,
            interactive: std::io::stdin().is_terminal(),
            env: Env::from_process(),
        }
    }
}
