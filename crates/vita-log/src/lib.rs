//! vita-log — the diagnostic gate layer (doc-13 bucket C).
//!
//! First real increment: per-code suppress/promote gates riding BETWEEN the
//! producers (preprocess/parse/elaborate/sim-engine, which only `emit`) and the
//! concrete rendering sink the CLI installs. Policy is pure output-stream
//! filtering — it never reaches `PreprocInputs`/`ElabInputs`, so `.vu`/`.velab`
//! hashes and the SimIr golden are structurally unaffected (doc-13 RULE API).
//!
//! Contract (doc-13):
//! - `-Wno-<MNEMONIC>` drops Warning/Info diagnostics with that code. The
//!   always-logged spine is untouchable: Error/Fatal are NEVER suppressed
//!   (a `-Wno-E-…` flag is accepted — the mnemonic is real — but has no effect).
//! - `-Werror` promotes EVERY warning to Error; `-Werror=<MNEMONIC>` promotes
//!   just that code. A promoted diagnostic keeps its original (stable) code
//!   number — only the severity (and therefore the exit class) changes.
//! - Mnemonics are the 1st-class stable key (doc-15); an unknown mnemonic in a
//!   gate flag is a CLI usage error, not a silent no-op.

use diag::{LogEvent, LogSink, MsgCode, Severity};
use std::collections::BTreeSet;

/// Per-code suppress/promote policy, built from `-Wno-*` / `-Werror[=*]` flags.
#[derive(Debug, Clone, Default)]
pub struct GatePolicy {
    suppress: BTreeSet<&'static str>,
    promote_all: bool,
    promote: BTreeSet<&'static str>,
}

/// Resolve a user-supplied mnemonic to the interned `&'static str` key of a
/// real `MsgCode` (the exhaustive enum is the source of truth).
fn intern_mnemonic(name: &str) -> Option<&'static str> {
    MsgCode::ALL
        .iter()
        .map(|c| c.mnemonic())
        .find(|m| *m == name)
}

impl GatePolicy {
    /// Try to consume one CLI argument as a gate flag.
    ///
    /// - `None` — not a gate flag (caller keeps parsing).
    /// - `Some(Ok(()))` — consumed.
    /// - `Some(Err(msg))` — it WAS a gate flag but the mnemonic is unknown;
    ///   the caller reports a usage error (doc-15: mnemonics are stable keys,
    ///   so a typo must be loud).
    pub fn parse_arg(&mut self, arg: &str) -> Option<Result<(), String>> {
        if let Some(code) = arg.strip_prefix("-Wno-") {
            return Some(match intern_mnemonic(code) {
                Some(m) => {
                    self.suppress.insert(m);
                    Ok(())
                }
                None => Err(format!("unknown diagnostic code '{code}' in '-Wno-'")),
            });
        }
        if arg == "-Werror" {
            self.promote_all = true;
            return Some(Ok(()));
        }
        if let Some(code) = arg.strip_prefix("-Werror=") {
            return Some(match intern_mnemonic(code) {
                Some(m) => {
                    self.promote.insert(m);
                    Ok(())
                }
                None => Err(format!("unknown diagnostic code '{code}' in '-Werror='")),
            });
        }
        None
    }

    /// True when no flag was given (the gate is then a pure pass-through).
    pub fn is_empty(&self) -> bool {
        self.suppress.is_empty() && !self.promote_all && self.promote.is_empty()
    }

    fn suppresses(&self, code: MsgCode) -> bool {
        self.suppress.contains(code.mnemonic())
    }

    fn promotes(&self, code: MsgCode) -> bool {
        self.promote_all || self.promote.contains(code.mnemonic())
    }
}

/// A `LogSink` adapter applying a [`GatePolicy`] in front of an inner sink.
/// Suppressed diagnostics never reach the inner sink (so its Error/Fatal
/// counters see exactly the post-gate stream — a PROMOTED warning counts as a
/// real error there, which is what drives the exit code).
pub struct GatedSink<'a> {
    inner: &'a dyn LogSink,
    policy: GatePolicy,
}

impl<'a> GatedSink<'a> {
    pub fn new(inner: &'a dyn LogSink, policy: GatePolicy) -> Self {
        GatedSink { inner, policy }
    }
}

impl LogSink for GatedSink<'_> {
    fn emit(&self, event: LogEvent) {
        match event {
            LogEvent::Diagnostic(mut d) => {
                match d.severity {
                    // Error/Fatal: the always-logged spine — gates never apply.
                    Severity::Error | Severity::Fatal => {}
                    Severity::Warning => {
                        if self.policy.suppresses(d.code) {
                            return;
                        }
                        if self.policy.promotes(d.code) {
                            d.severity = Severity::Error;
                        }
                    }
                    _ => {
                        // Info-class diagnostics: suppressible, never promoted.
                        if self.policy.suppresses(d.code) {
                            return;
                        }
                    }
                }
                self.inner.emit(LogEvent::Diagnostic(d));
            }
            other => self.inner.emit(other),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use diag::Diagnostic;
    use std::cell::RefCell;

    /// Records the post-gate diagnostic stream as `(severity, mnemonic)`.
    #[derive(Default)]
    struct Rec(RefCell<Vec<(Severity, &'static str)>>);
    impl LogSink for Rec {
        fn emit(&self, e: LogEvent) {
            if let LogEvent::Diagnostic(d) = e {
                self.0.borrow_mut().push((d.severity, d.code.mnemonic()));
            }
        }
    }

    fn diag(severity: Severity, code: MsgCode) -> LogEvent {
        LogEvent::Diagnostic(Diagnostic {
            severity,
            code,
            message: String::new(),
            location: None,
            context: Vec::new(),
            sim_time: None,
        })
    }

    #[test]
    fn suppress_drops_warning_only() {
        let mut p = GatePolicy::default();
        assert!(matches!(
            p.parse_arg("-Wno-W-PP-TIMESCALE-DEFAULT"),
            Some(Ok(()))
        ));
        let rec = Rec::default();
        let gate = GatedSink::new(&rec, p);
        gate.emit(diag(Severity::Warning, MsgCode::PpTimescaleDefault)); // dropped
        gate.emit(diag(Severity::Warning, MsgCode::PpMacroRedefined)); // passes
        let seen = rec.0.borrow();
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].1, "W-PP-MACRO-REDEFINED");
    }

    #[test]
    fn promote_all_and_targeted() {
        let mut all = GatePolicy::default();
        assert!(matches!(all.parse_arg("-Werror"), Some(Ok(()))));
        let rec = Rec::default();
        GatedSink::new(&rec, all).emit(diag(Severity::Warning, MsgCode::PpTimescaleDefault));
        assert_eq!(rec.0.borrow()[0].0, Severity::Error);

        let mut tgt = GatePolicy::default();
        assert!(matches!(
            tgt.parse_arg("-Werror=W-PP-MACRO-REDEFINED"),
            Some(Ok(()))
        ));
        let rec2 = Rec::default();
        let gate = GatedSink::new(&rec2, tgt);
        gate.emit(diag(Severity::Warning, MsgCode::PpTimescaleDefault)); // not promoted
        gate.emit(diag(Severity::Warning, MsgCode::PpMacroRedefined)); // promoted
        let seen = rec2.0.borrow();
        assert_eq!(seen[0].0, Severity::Warning);
        assert_eq!(seen[1].0, Severity::Error);
    }

    #[test]
    fn errors_pass_untouched_even_when_named() {
        let mut p = GatePolicy::default();
        assert!(matches!(
            p.parse_arg("-Wno-E-ELAB-UNRESOLVED-NAME"),
            Some(Ok(()))
        ));
        let rec = Rec::default();
        GatedSink::new(&rec, p).emit(diag(Severity::Error, MsgCode::ElabUnresolvedName));
        assert_eq!(rec.0.borrow().len(), 1, "spine: errors are unsuppressible");
    }

    #[test]
    fn unknown_mnemonic_is_an_error_and_non_gate_flags_pass() {
        let mut p = GatePolicy::default();
        assert!(matches!(p.parse_arg("-Wno-NOPE"), Some(Err(_))));
        assert!(p.parse_arg("--threads").is_none());
        assert!(p.parse_arg("design.sv").is_none());
    }
}
