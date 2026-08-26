//! `{<<N{…}}` — the right-to-left streaming concatenation (IEEE 1800-2017 §11.4.14).
//!
//! The parser cannot build this: the number of blocks is `$bits(operand) / N`, and the
//! operand's width is not known until elaborate. So the parser desugars a streaming
//! concatenation into a marker system call ([`STREAM_REV`] / [`STREAM_FWD`] — vita's
//! reserved `$__vita_*` channel, a name being the only thing the parser can say to
//! elaborate without changing the frozen AST shape) and this module expands it.
//!
//! `{>>N{…}}` is the identity for a packed operand (§11.4.14 ignores its slice size),
//! so its expansion is just the operand — but it still gets a marker, because
//! §11.4.14.3's assignment padding distinguishes it from the plain concatenation it
//! would otherwise become. See [`Elaborator::pad_stream_rhs`].
//!
//! ## The rule, as measured
//!
//! iverilog 13.0 is NOT an oracle here — it refuses the operator outright
//! (`{<<8{a}}` is a syntax error; `{<<{a}}` parses and then says "sorry: Streaming
//! concatenation not supported"), so every expectation below is verilator 5.050 plus
//! hand-IEEE:
//!
//! ```text
//!  {<<8{32'hAABBCCDD}} = ddccbbaa      {>>8{32'hAABBCCDD}} = aabbccdd
//!  {<<{8'b11010010}}   = 01001011      {<<16{32'hAABBCCDD}} = ccddaabb
//!  {<<8{12'hABC}}      = bca           {<<3{8'b11010010}}   = 4b
//!  {<<5{16'h1234}}     = a448          {<<64{32'hAABBCCDD}} = aabbccdd
//! ```
//!
//! The operand is cut into `N`-bit blocks **starting at the LSB**; the leftover high
//! bits (fewer than `N`) form a short final block. The blocks are then emitted in
//! reverse order, so the block that held the LOW bits becomes the leftmost and the
//! SHORT block ends up rightmost — `12'hABC` → `{8'hBC, 4'hA}` = `12'hBCA`. (The
//! external report claimed the partial block "stays leftmost"; `bca` and `4b` say
//! otherwise, and `{<<5{16'h1234}}` = `a448` pins the three-full-blocks-plus-one-bit
//! case exactly.) `N ≥ $bits(operand)` leaves one short block = the identity.
//!
//! The result is `$bits(operand)` bits wide and unsigned, which is what a `Concat`
//! of the slices is on its own — no width or sign is asserted anywhere below.
use super::*;

/// The marker name the parser emits for `{<<N{…}}` — defined once in `hdl-ast` so
/// producer and consumer cannot drift. It is in vita's reserved `$__vita_` namespace,
/// which the parser refuses to let SOURCE spell, so it cannot collide with a user's
/// system function.
pub(crate) const STREAM_REV: &str = ast::STREAM_REV_FUNC;
/// The `{>>N{…}}` twin. See [`ast::STREAM_FWD_FUNC`] for why it exists at all.
pub(crate) const STREAM_FWD: &str = ast::STREAM_FWD_FUNC;

/// What every rejection here says. One sentence, because the reader's problem is
/// always the same: which part of §11.4.14 does vita have?
pub(crate) const STREAM_LIMIT_HINT: &str =
    "vita implements the streaming operator (IEEE 1800-2017 §11.4.14) only as a \
     right-hand-side expression over a PACKED operand of statically known width, \
     built from names, selects, literals and concatenations of those";

impl Elaborator<'_> {
    /// Expand `$__vita_stream_fwd(operand)` — the value IS the operand, lowered once.
    /// No purity gate is needed here (one reference, one evaluation) and no width is
    /// asked for; the marker existed only so the assignment funnel could see it.
    pub(crate) fn lower_stream_fwd(&mut self, args: &[ast::Expr]) -> u32 {
        debug_assert_eq!(args.len(), 1, "stream marker arity is the parser's job");
        match args.first() {
            Some(a) => self.lower_expr(a),
            None => self.placeholder_expr(),
        }
    }

    /// Expand `$__vita_stream_rev(slice_size, operand)`.
    pub(crate) fn lower_stream_rev(&mut self, args: &[ast::Expr]) -> u32 {
        // The parser builds exactly two args; a hand-written `$__vita_stream_rev` is
        // impossible (the parser rejects the whole `$__vita_` namespace in source), so
        // a wrong arity means a desugar bug, not bad input.
        debug_assert_eq!(args.len(), 2, "stream marker arity is the parser's job");
        if args.len() != 2 {
            self.error(MsgCode::ElabUnsupported, STREAM_LIMIT_HINT);
            return self.placeholder_expr();
        }
        // ⚠️ The slices SHARE one lowered operand (`base: op_id` in every `Select`),
        // and the evaluator walks the arena as a tree — a shared node is evaluated
        // once per reference. That is invisible for a value that only READS state,
        // and a silent-wrong for one that does not: verilator draws `{<<8{$random}}`
        // ONCE, while four references would draw four different bytes. Hence the
        // purity gate below, which is an allow-list (`_ => false`), so a new
        // `ExprKind` is refused until someone classifies it.
        if !stream_operand_is_repeatable(&args[1]) {
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "a streaming-concatenation operand that calls a function or a \
                     system function is not supported — its value would be recomputed \
                     once per output block. {STREAM_LIMIT_HINT}"
                ),
            );
            return self.placeholder_expr();
        }
        let slice = match self.const_eval_in_scope(&args[0]) {
            Some(n) if n > 0 => n as u64,
            // §11.4.14 requires a positive constant slice size. A non-constant one is
            // the same refusal: the block count must be known at elaborate.
            _ => {
                self.error(
                    MsgCode::ElabUnsupported,
                    &format!(
                        "the slice size of `{{<<N{{…}}}}` must be a positive constant \
                         known at elaborate. {STREAM_LIMIT_HINT}"
                    ),
                );
                return self.placeholder_expr();
            }
        };
        let op_id = self.lower_expr(&args[1]);
        // `trusted_self_width` is the canonical "may I rely on this width" gate
        // (§4.5.320/321): `None` means unknown HERE — a string, an array view, a
        // class handle — and guessing one is how a 1-bit net gets declared at exit 0.
        let Some(width) = self.trusted_self_width(op_id) else {
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "the operand of a streaming concatenation has no statically known \
                     packed width here. {STREAM_LIMIT_HINT}"
                ),
            );
            return self.placeholder_expr();
        };
        if width == 0 {
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "the operand of a streaming concatenation is 0 bits wide. {STREAM_LIMIT_HINT}"
                ),
            );
            return self.placeholder_expr();
        }
        // Blocks from the LSB up, emitted in that order — `parts[0]` is the CONCAT's
        // most significant part, so block 0 (the operand's low bits) lands leftmost.
        // The `min` keeps the last block short instead of reading past the operand,
        // which also covers `N > width` (one block, the identity).
        let mut parts = Vec::new();
        let mut off: u64 = 0;
        while off < width as u64 {
            let w = slice.min(width as u64 - off) as u32;
            let offset = self.const_u32_expr(off as u32, 32);
            let w_id = self.const_u32_expr(w, 32);
            parts.push(self.push_expr(ir::Expr::Select {
                base: op_id,
                offset,
                width: w_id,
                kind: ir::SelKind::PartConst,
            }));
            off += w as u64;
        }
        self.push_expr(ir::Expr::Concat { parts })
    }
}

impl Elaborator<'_> {
    /// §11.4.14.3: **a streaming concatenation assigned to a WIDER target is
    /// left-justified**, where every other expression zero-extends. Measured
    /// (verilator 5.050, `logic [63:0] w`, `a = 32'hAABBCCDD`):
    ///
    /// ```text
    ///   w = {<<8{a}};          ddccbbaa00000000     w = ({<<8{a}}) | 64'h0;  00000000ddccbbaa
    ///   w = {>>{a}};           aabbccdd00000000     w = {32'h0, {<<8{a}}};   00000000ddccbbaa
    ///   w <= {<<8{a}};         ddccbbaa00000000     f({<<8{a}})              00000000ddccbbaa
    ///   assign w = {<<8{a}};   ddccbbaa00000000     .p({<<8{a}})             00000000ddccbbaa
    /// ```
    ///
    /// So the rule is exactly "the rhs OF AN ASSIGNMENT", not "anywhere the value
    /// flows": an operator, a concatenation, a `$display` argument, a subroutine
    /// argument and a port connection all take the stream at its own width. That is
    /// why this lives at the assignment funnel and the expansion does not do it — the
    /// value has no context of its own, and the leaf is the wrong place to spend one
    /// (§4.5.221: convert at the context boundary, not the leaf).
    ///
    /// A NARROWER target is an error in §11.4.14.3; vita and verilator both truncate
    /// it the ordinary way (drop the high bits — verilator gives `bbaa` for a 16-bit
    /// target), so nothing is done here and nothing new is claimed.
    ///
    /// Returns `None` when the rhs is not a stream, so the caller keeps its id
    /// byte-identically.
    pub(crate) fn pad_stream_rhs(
        &mut self,
        rhs: &ast::Expr,
        rhs_id: u32,
        lv: &ir::Lvalue,
    ) -> Option<u32> {
        if !is_stream_rhs(rhs) {
            return None;
        }
        // A real target has no bit context to left-justify into (the same reason
        // `resize_rhs_for_lvalue` withholds the fill context from one), and a DEFERRED
        // hierarchical target has no width YET — `ir_lvalue_width` answers 1 for its
        // sentinel net, which would "pad" the stream into nothing. Neither can be
        // answered here, so neither is guessed: loud, because the alternative is a
        // wrong number at exit 0.
        if self.lvalue_targets_real(lv) || self.deferred_lvalue_sentinel(lv).is_some() {
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "a streaming concatenation assigned to a `real` or to a \
                     not-yet-elaborated hierarchical target has no assignment width \
                     here, and §11.4.14.3 needs one to know where to pad. \
                     {STREAM_LIMIT_HINT}"
                ),
            );
            return Some(rhs_id);
        }
        let lv_width = self.ir_lvalue_width(lv);
        // `None` = the expansion already failed loudly (a placeholder); padding it
        // would only invent a width for a value that has none.
        let stream_width = self.trusted_self_width(rhs_id)?;
        if lv_width <= stream_width {
            return Some(rhs_id);
        }
        let pad_w = lv_width - stream_width;
        let zero = self.const_u32_expr(0, pad_w);
        Some(self.push_expr(ir::Expr::Concat {
            parts: vec![rhs_id, zero],
        }))
    }
}

/// Is this rhs a streaming concatenation AS THE WHOLE rhs? Parentheses are peeled —
/// `w = ({<<8{a}});` is still the direct rhs of the assignment — but nothing else is:
/// `w = {32'h0, {<<8{a}}};` and `w = ({<<8{a}}) | 0;` are ordinary expressions that
/// merely CONTAIN a stream, and verilator zero-extends both (measured above).
fn is_stream_rhs(e: &ast::Expr) -> bool {
    match &e.kind {
        ast::ExprKind::SysCall { name, .. } => {
            name.name == ast::STREAM_REV_FUNC || name.name == ast::STREAM_FWD_FUNC
        }
        ast::ExprKind::Paren { inner } => is_stream_rhs(inner),
        _ => false,
    }
}

/// May this operand's value be computed more than once with the same answer and no
/// other effect? An ALLOW-list: every kind that is a pure function of state it only
/// reads. Anything else — a user call, a system call (`$random`, `$fgetc`, `$time`),
/// a method, `new`, a cast, an array reduction — is refused, so the shared-node
/// re-evaluation described above can never change what a design means.
fn stream_operand_is_repeatable(e: &ast::Expr) -> bool {
    match &e.kind {
        ast::ExprKind::IntLit { .. }
        | ast::ExprKind::Ident(_)
        | ast::ExprKind::PkgScoped { .. } => true,
        ast::ExprKind::Paren { inner } => stream_operand_is_repeatable(inner),
        ast::ExprKind::Unary { operand, .. } => stream_operand_is_repeatable(operand),
        ast::ExprKind::Binary { lhs, rhs, .. } => {
            stream_operand_is_repeatable(lhs) && stream_operand_is_repeatable(rhs)
        }
        ast::ExprKind::Ternary {
            cond,
            then_e,
            else_e,
        } => {
            stream_operand_is_repeatable(cond)
                && stream_operand_is_repeatable(then_e)
                && stream_operand_is_repeatable(else_e)
        }
        ast::ExprKind::BitSelect { base, index } => {
            stream_operand_is_repeatable(base) && stream_operand_is_repeatable(index)
        }
        ast::ExprKind::PartSelect { base, msb, lsb } => {
            stream_operand_is_repeatable(base)
                && stream_operand_is_repeatable(msb)
                && stream_operand_is_repeatable(lsb)
        }
        ast::ExprKind::IndexedPart {
            base,
            offset,
            width,
            ..
        } => {
            stream_operand_is_repeatable(base)
                && stream_operand_is_repeatable(offset)
                && stream_operand_is_repeatable(width)
        }
        // A NESTED streaming concatenation (`{<<8{{<<8{a}}}}`, which verilator answers
        // `aabbccdd` — the operator is its own inverse at a fixed slice size). It is
        // pure exactly when its own operand is, so recurse rather than refuse; this is
        // the one `SysCall` spelling admitted, and only because vita synthesized it.
        ast::ExprKind::SysCall { name, args }
            if name.name == ast::STREAM_REV_FUNC || name.name == ast::STREAM_FWD_FUNC =>
        {
            args.iter().all(stream_operand_is_repeatable)
        }
        ast::ExprKind::Concat { parts } => parts.iter().all(stream_operand_is_repeatable),
        ast::ExprKind::Replicate { count, value } => {
            stream_operand_is_repeatable(count) && value.iter().all(stream_operand_is_repeatable)
        }
        _ => false,
    }
}
