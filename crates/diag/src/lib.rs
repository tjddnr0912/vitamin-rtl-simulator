//! diag — diagnostic data model + renderer boundary (leaf, IO-free).
mod code;
mod event;
pub mod fmt;
mod severity;

pub use code::MsgCode;
pub use event::{
    Diagnostic, Frame, LogEvent, LogSink, ProgressEvent, RtlText, SourceLoc, SpanResolver,
    TimeStamp,
};
pub use severity::Severity;
