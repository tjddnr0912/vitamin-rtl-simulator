use crate::{MsgCode, Severity};

/// Resolved source location (file:line:col + byte span).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLoc {
    pub file: String,
    pub line: u32,
    pub col: u32,
    pub byte_start: u32,
    pub byte_end: u32,
}

/// Simulation timestamp attached to runtime severity events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeStamp {
    pub ticks: u64,
}

/// Context frame: include/macro/-f stack or instance/hierarchy path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub label: String,
    pub location: Option<SourceLoc>,
}

/// One diagnostic. `diag` renders exactly this; it knows nothing of counts/exit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: MsgCode,
    pub message: String,
    pub location: Option<SourceLoc>,
    pub context: Vec<Frame>,
    pub sim_time: Option<TimeStamp>,
}

/// Non-diagnostic progress event (banner, "reading file X", run summary).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgressEvent {
    pub message: String,
}

/// User-visible RTL text ($display/$write/$monitor/$strobe) — no severity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RtlText {
    pub text: String,
    pub sim_time: Option<TimeStamp>,
}

/// Single event model (13 §single event model). One stream, fanned out by the sink.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogEvent {
    Diagnostic(Diagnostic),
    Progress(ProgressEvent),
    RtlOutput(RtlText),
}

/// Boundary trait. Emitters take `&dyn LogSink` and depend only on `diag`,
/// keeping `diag` a leaf (no IO / tracing). The concrete sink lives in `vita-log`.
pub trait LogSink {
    fn emit(&self, event: LogEvent);
}
