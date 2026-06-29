//! diag — diagnostic data model + renderer boundary (leaf, IO-free).
mod code;
mod event;
mod severity;

pub use code::MsgCode;
pub use event::{
    Diagnostic, Frame, LogEvent, LogSink, ProgressEvent, RtlText, SourceLoc, TimeStamp,
};
pub use severity::Severity;
