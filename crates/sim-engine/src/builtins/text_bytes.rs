//! RTL text as BYTES (Q-STRBYTES): the const-text decoders the format engine reads,
//! the justify wrapper, and the severity-task diagnostic constructor. RTL text stays
//! `Vec<u8>` from its producer to the sink write; the one lossy view here
//! ([`const_string`]) is for the named OS / name boundaries only.

use crate::state::SimState;

/// The diagnostic of a severity task: its user message is RTL text, so it travels
/// as BYTES (`Diagnostic::rtl`); an empty message is the code's title (code-authored
/// text). One spelling for the statement path and the frame path.
pub(crate) fn severity_diag(
    sev: crate::SeverityKind,
    message: Vec<u8>,
    location: Option<diag::SourceLoc>,
    context: Vec<diag::Frame>,
    sim_time: Option<diag::TimeStamp>,
) -> diag::Diagnostic {
    // One spelling, shared with the frame-path emitter — see `SeverityKind`.
    let (severity, code) = sev.diag_class();
    if message.is_empty() {
        let message = code.title().to_string();
        #[rustfmt::skip]
        return diag::Diagnostic { severity, code, message, location, context, sim_time, rtl_message: None };
    }
    diag::Diagnostic::rtl(severity, code, message, location, context, sim_time)
}
/// Resolve an ExprId that is a `Const{val}` into its const bytes (format str).
pub(crate) fn expr_const_bytes(st: &SimState, eid: u32) -> Vec<u8> {
    if let sim_ir::Expr::Const { val } = &st.ir.exprs[eid as usize] {
        st.fmt_const_bytes(*val) // FMT-CACHE: memoized by ConstId
    } else {
        Vec::new()
    }
}

/// The lossy Rust-text VIEW of [`const_bytes`], for the boundaries that need text:
/// OS file names (`$fopen`, `$readmem*`/`$writemem*`, `$dumpfile`) and `$dumpvars`
/// scope names (Q-STRBYTES L1 / L5). RTL text itself is [`const_bytes`].
pub(crate) fn const_string(ir: &sim_ir::SimIr, cid: u32) -> String {
    String::from_utf8_lossy(&const_bytes(ir, cid)).into_owned()
}

/// Decode a `ConstVal` (StrUtf8 → its bytes; numeric → packed bytes), byte-exact.
pub(crate) fn const_bytes(ir: &sim_ir::SimIr, cid: u32) -> Vec<u8> {
    let c = &ir.consts[cid as usize];
    let nbytes = c.width.div_ceil(8) as usize;
    let mut bytes = Vec::with_capacity(nbytes);
    // StrUtf8 packs in IEEE §5.9 order (v6): the FIRST character is the MOST
    // significant byte — read the value top byte down to recover source order.
    for b in (0..nbytes).rev() {
        let bit = (b as u32) * 8;
        let w = (bit / 64) as usize;
        let s = bit % 64;
        let byte = if s <= 56 {
            (c.bits.val.get(w).copied().unwrap_or(0) >> s) as u8
        } else {
            let lo = c.bits.val.get(w).copied().unwrap_or(0) >> s;
            let hi = c.bits.val.get(w + 1).copied().unwrap_or(0) << (64 - s);
            (lo | hi) as u8
        };
        bytes.push(byte);
    }
    while bytes.last() == Some(&0) {
        bytes.pop();
    }
    // …and, for a NUMERIC constant, the leading ones: the padding of a value wider than
    // its text (§6.16 — NUL characters are ignored when an integral value is used as a
    // string), the rule `value_str_bytes` already applies to a non-constant argument. A
    // `parameter [8*16-1:0] F = "k.hex"` used as a file name opened `"\0…\0k.hex"`
    // where both oracles open `k.hex`. A string LITERAL keeps its bytes: `%s` of
    // `"\000a"` renders the NUL (both oracles print it as a space), and its length counts.
    let lead = if c.repr == sim_ir::ConstRepr::Numeric {
        bytes.iter().take_while(|&&b| b == 0).count()
    } else {
        0
    };
    bytes.drain(..lead);
    bytes
}
/// The argument ExprId IFF it is a string-literal constant (`ConstRepr::StrUtf8`).
pub(crate) fn str_const_of_expr(st: &SimState, eid: u32) -> Option<Vec<u8>> {
    if let sim_ir::Expr::Const { val } = &st.ir.exprs[eid as usize] {
        if st.ir.consts[*val as usize].repr == sim_ir::ConstRepr::StrUtf8 {
            return Some(st.fmt_const_bytes(*val)); // FMT-CACHE
        }
    }
    None
}
/// Pad `content` (bytes) to `field_width` (a MINIMUM — never truncated, the width
/// counts bytes): right-justified by default, or left-justified under `-`.
pub(crate) fn justify(content: &[u8], field_width: Option<usize>, left_just: bool) -> Vec<u8> {
    diag::fmt::justify(content, field_width, left_just)
}

impl SimState<'_> {
    /// FMT-CACHE: decode a format/string const, memoized by ConstId. The
    /// bit-unpack (`const_bytes`) runs once per const; later calls clone the
    /// cached bytes. Byte-identical to calling `const_bytes(self.ir, cid)`.
    pub(crate) fn fmt_const_bytes(&self, cid: u32) -> Vec<u8> {
        let mut cache = self.fmt_cache.borrow_mut();
        let slot = &mut cache[cid as usize];
        if slot.is_none() {
            *slot = Some(crate::builtins::const_bytes(self.ir, cid).into_boxed_slice());
        }
        slot.as_deref().unwrap().to_vec()
    }
}
