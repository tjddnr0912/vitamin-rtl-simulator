/// 5-level severity lattice (13 §Severity).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Severity {
    Note,
    Info,
    Warning,
    Error,
    Fatal,
}

impl Severity {
    /// Output token prefix, e.g. `error` in `error[VITA-E3001]:`.
    pub const fn token(self) -> &'static str {
        match self {
            Severity::Note => "note",
            Severity::Info => "info",
            Severity::Warning => "warning",
            Severity::Error => "error",
            Severity::Fatal => "fatal",
        }
    }
}
