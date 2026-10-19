use std::io::IsTerminal;

/// Top-level shell state.
pub struct Shell {
    /// Exit status of the last command.
    pub last_status: i32,
    /// True when stdin is a terminal.
    pub interactive: bool,
}

impl Shell {
    pub fn new() -> Self {
        Self {
            last_status: 0,
            interactive: std::io::stdin().is_terminal(),
        }
    }
}
