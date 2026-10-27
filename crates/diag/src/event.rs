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
    /// The message as Rust text. For a message rendered from RTL text (a severity
    /// task's user message) this is the lossy VIEW of [`Self::rtl_message`].
    pub message: String,
    pub location: Option<SourceLoc>,
    pub context: Vec<Frame>,
    pub sim_time: Option<TimeStamp>,
    /// The exact bytes of a message rendered from RTL text (`$info`/`$warning`/
    /// `$error`/`$fatal`: runtime, frame, deferred, elaboration). RTL text is bytes
    /// (IEEE 1800-2017 §21.2.1.8, 8 bits per character), and a byte >= 0x80 must
    /// reach the output unchanged, so the renderer writes these when present and
    /// `message.as_bytes()` otherwise. Set ONLY by [`Diagnostic::rtl`], which keeps
    /// `message` = the lossy view of the same bytes; code-authored text is `None`.
    pub rtl_message: Option<Box<[u8]>>,
}

impl Diagnostic {
    /// A diagnostic whose message is RTL text: the bytes are kept verbatim and
    /// `message` is their lossy view (the one place the two are made together, so
    /// they cannot disagree).
    pub fn rtl(
        severity: Severity,
        code: MsgCode,
        text: Vec<u8>,
        location: Option<SourceLoc>,
        context: Vec<Frame>,
        sim_time: Option<TimeStamp>,
    ) -> Self {
        Diagnostic {
            severity,
            code,
            message: String::from_utf8_lossy(&text).into_owned(),
            location,
            context,
            sim_time,
            rtl_message: Some(text.into_boxed_slice()),
        }
    }

    /// The bytes a renderer writes for the message: the RTL bytes when the
    /// message came from RTL text, else the code-authored `message`.
    pub fn message_bytes(&self) -> &[u8] {
        match &self.rtl_message {
            Some(b) => b,
            None => self.message.as_bytes(),
        }
    }
}

/// Non-diagnostic progress event (banner, "reading file X", run summary).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgressEvent {
    pub message: String,
}

/// User-visible RTL text ($display/$write/$monitor/$strobe) — no severity. The
/// text is BYTES: RTL text is not UTF-8 (a string literal `"\310"` is the byte
/// 0xC8), and every byte reaches the sink unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RtlText {
    pub text: Vec<u8>,
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

/// Resolve an EXPANDED-text byte span back to an original `file:line:col`.
///
/// Elaborate carries AST spans but not the preprocessor's `SourceMap`, and it must
/// not depend on the preprocessor to get one. The front end owns the map and hands
/// the elaborator this view of it, so every elaborate-time diagnostic can point at
/// the declaration it is about — without which 81 identical messages in one run are
/// indistinguishable, and the construct that caused them cannot be found at all.
pub trait SpanResolver {
    fn resolve(&self, lo: u32, hi: u32) -> SourceLoc;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_rtl_message_keeps_its_bytes_and_its_view_is_the_lossy_text() {
        let d = Diagnostic::rtl(
            Severity::Error,
            MsgCode::RunUserError,
            vec![b'a', 0xC8, b'b'],
            None,
            Vec::new(),
            None,
        );
        assert_eq!(d.message_bytes(), &[b'a', 0xC8, b'b']);
        assert_eq!(d.message, "a\u{FFFD}b");
        let e = Diagnostic::rtl(
            Severity::Info,
            MsgCode::RunUserInfo,
            "x \u{e9}".as_bytes().to_vec(),
            None,
            Vec::new(),
            None,
        );
        assert_eq!(e.message, "x \u{e9}");
        assert_eq!(e.message_bytes(), "x \u{e9}".as_bytes());
    }

    #[test]
    fn a_code_authored_message_renders_its_own_utf8() {
        let d = Diagnostic {
            severity: Severity::Warning,
            code: MsgCode::RunUserWarning,
            message: "a \u{2014} \u{a7}".to_string(),
            location: None,
            context: Vec::new(),
            sim_time: None,
            rtl_message: None,
        };
        assert_eq!(d.message_bytes(), "a \u{2014} \u{a7}".as_bytes());
    }
}
