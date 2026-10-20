//! Environment table (M3, step 1).

/// A single variable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Var {
    pub name: String,
    pub value: Option<String>,
    pub exported: bool,
}

/// Environment table; the `Vec` keeps insertion order.
#[derive(Debug, Default)]
pub struct Env {
    vars: Vec<Var>,
}

impl Env {
    /// Empty environment.
    pub fn new() -> Self {
        Self { vars: Vec::new() }
    }

    /// Environment inherited from the current process.
    ///
    /// Entries that are not valid UTF-8 are skipped; all kept entries are
    /// exported. `SHLVL` is bumped by one (missing or invalid becomes 1).
    pub fn from_process() -> Self {
        let mut env = Self::new();
        for (name, value) in std::env::vars_os() {
            let Some(name) = name.to_str() else {
                continue;
            };
            let Some(value) = value.to_str() else {
                continue;
            };
            env.set(name, Some(value.to_string()), true);
        }
        let shlvl = env.get("SHLVL");
        env.set("SHLVL", Some(next_shlvl(shlvl)), true);
        env
    }

    /// Value of `name`, or `None` if the variable is missing or has no value.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.vars.iter().find(|v| v.name == name)?.value.as_deref()
    }

    /// Set `name` to `value`, exported or not.
    ///
    /// If the variable exists, update it in place (keeping its position);
    /// a `None` value keeps the old value. Otherwise push a new entry.
    pub fn set(&mut self, name: &str, value: Option<String>, exported: bool) {
        if let Some(var) = self.vars.iter_mut().find(|v| v.name == name) {
            if let Some(value) = value {
                var.value = Some(value);
            }
            var.exported = exported;
        } else {
            self.vars.push(Var {
                name: name.to_string(),
                value,
                exported,
            });
        }
    }

    /// Remove `name` if present; removing a missing variable is a no-op.
    #[allow(dead_code)]
    pub fn unset(&mut self, name: &str) {
        self.vars.retain(|v| v.name != name);
    }

    /// `NAME=value` entries for exported variables that have a value.
    /// Entries containing NUL bytes are skipped.
    #[allow(dead_code)]
    pub fn to_envp(&self) -> Vec<std::ffi::CString> {
        let mut out = Vec::new();
        for var in &self.vars {
            if !var.exported {
                continue;
            }
            let Some(value) = var.value.as_deref() else {
                continue;
            };
            if var.name.contains('\0') || value.contains('\0') {
                continue;
            }
            let entry = format!("{}={value}", var.name);
            if let Ok(cstring) = std::ffi::CString::new(entry) {
                out.push(cstring);
            }
        }
        out
    }
}

/// Compute the next `SHLVL` value: parse the old one (missing/invalid -> 0)
/// and return `old + 1` as a string.
pub fn next_shlvl(old: Option<&str>) -> String {
    match old.and_then(|s| s.parse::<i32>().ok()) {
        Some(n) => (n + 1).to_string(),
        None => "1".to_string(),
    }
}

/// True when `s` is a valid variable name: `[A-Za-z_][A-Za-z0-9_]*`, non-empty.
#[allow(dead_code)]
pub fn is_valid_name(s: &str) -> bool {
    let mut chars = s.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
#[path = "env_tests.rs"]
mod tests;
