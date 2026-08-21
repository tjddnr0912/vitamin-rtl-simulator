//! expression context width — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

pub(crate) fn map_unop(op: ast::UnOp) -> ir::UnOp {
    use ast::UnOp::*;
    match op {
        Plus => ir::UnOp::Plus,
        Minus => ir::UnOp::Minus,
        LogNot => ir::UnOp::LogNot,
        BitNot => ir::UnOp::BitNot,
        RedAnd => ir::UnOp::RedAnd,
        RedNand => ir::UnOp::RedNand,
        RedOr => ir::UnOp::RedOr,
        RedNor => ir::UnOp::RedNor,
        RedXor => ir::UnOp::RedXor,
        RedXnor => ir::UnOp::RedXnor,
    }
}

pub(crate) fn map_binop(op: ast::BinOp) -> ir::BinOp {
    use ast::BinOp::*;
    match op {
        Add => ir::BinOp::Add,
        Sub => ir::BinOp::Sub,
        Mul => ir::BinOp::Mul,
        Div => ir::BinOp::Div,
        Mod => ir::BinOp::Mod,
        Pow => ir::BinOp::Pow,
        Shl => ir::BinOp::Shl,
        Shr => ir::BinOp::Shr,
        AShl => ir::BinOp::AShl,
        AShr => ir::BinOp::AShr,
        Lt => ir::BinOp::Lt,
        Le => ir::BinOp::Le,
        Gt => ir::BinOp::Gt,
        Ge => ir::BinOp::Ge,
        Eq => ir::BinOp::Eq,
        Ne => ir::BinOp::Ne,
        CaseEq => ir::BinOp::CaseEq,
        CaseNe => ir::BinOp::CaseNe,
        // `==?`/`!=?` are intercepted by `lower_wildcard_eq` BEFORE every
        // map_binop call site (both lower_expr Binary arms). These arms exist
        // only for match exhaustiveness: a missed interception degrades to the
        // masked-compare CORE op (plain eq — correct whenever the pattern has
        // no x/z bits) and trips the debug_assert in a debug build.
        WildEq => {
            debug_assert!(false, "WildEq must be lowered via lower_wildcard_eq");
            ir::BinOp::Eq
        }
        WildNe => {
            debug_assert!(false, "WildNe must be lowered via lower_wildcard_eq");
            ir::BinOp::Ne
        }
        BitAnd => ir::BinOp::BitAnd,
        BitXor => ir::BinOp::BitXor,
        BitXnor => ir::BinOp::BitXnor,
        BitOr => ir::BinOp::BitOr,
        LogAnd => ir::BinOp::LogAnd,
        LogOr => ir::BinOp::LogOr,
    }
}

/// Walk a `name[i][j]…` chain to its single-segment root ident, counting the
/// indices ($bits prescan key — the indices are NOT evaluated, IEEE §20.6.2).
pub(crate) fn ident_index_chain(e: &ast::Expr) -> Option<(&str, usize)> {
    let mut depth = 0usize;
    let mut cur = e;
    loop {
        match &cur.kind {
            ast::ExprKind::BitSelect { base, .. } => {
                depth += 1;
                cur = base;
            }
            ast::ExprKind::Paren { inner } => cur = inner,
            ast::ExprKind::Ident(p) => {
                return match p.segments.as_slice() {
                    [seg] => Some((seg.name.as_str(), depth)),
                    _ => None,
                };
            }
            _ => return None,
        }
    }
}

/// Reinterpret a parent connection `Expr` as an `ast::Lvalue` for an OUTPUT port
/// (the connection target must be a net / select / concat). Returns None for a
/// non-lvalue expression (a literal or an arithmetic result) — the caller emits
/// `ElabPortMismatch`. Mirrors the `Expr`/`Lvalue` variant shapes 1:1.
pub(crate) fn expr_to_lvalue(e: &ast::Expr) -> Option<ast::Lvalue> {
    match &e.kind {
        ast::ExprKind::Ident(path) => Some(ast::Lvalue::Ident(path.clone())),
        ast::ExprKind::Paren { inner } => expr_to_lvalue(inner),
        ast::ExprKind::BitSelect { base, index } => Some(ast::Lvalue::BitSelect {
            base: Box::new(expr_to_lvalue(base)?),
            index: index.clone(),
            span: e.span,
        }),
        ast::ExprKind::PartSelect { base, msb, lsb } => Some(ast::Lvalue::PartSelect {
            base: Box::new(expr_to_lvalue(base)?),
            msb: msb.clone(),
            lsb: lsb.clone(),
            span: e.span,
        }),
        ast::ExprKind::IndexedPart {
            base,
            offset,
            width,
            dir,
        } => Some(ast::Lvalue::IndexedPart {
            base: Box::new(expr_to_lvalue(base)?),
            offset: offset.clone(),
            width: width.clone(),
            dir: *dir,
            span: e.span,
        }),
        ast::ExprKind::Concat { parts } => {
            let lv_parts: Option<Vec<ast::Lvalue>> = parts.iter().map(expr_to_lvalue).collect();
            Some(ast::Lvalue::Concat {
                parts: lv_parts?,
                span: e.span,
            })
        }
        _ => None,
    }
}

/// A binary op whose value can DIFFER between a self-width eval-then-extend and a
/// wider context-width eval — so lowering it at self width and resizing is wrong.
/// `+`/`-` carry, `*`/`**` product and `<<`/`<<<` shifted-out bits gain magnitude; a
/// `>>`/`>>>` of a SIGNED operand drags the context-width sign extension into view; a
/// `/` of a SIGNED operand overflows at self width (INT_MIN / -1); an unsigned `~^`
/// (XNOR) flips the high extension bits to 1 (like unary `~`). Signedness isn't known
/// to this pure AST pass, so `>>`/`>>>`/`/`/`~^` route conservatively (a case where
/// the operand's sign makes it context-insensitive only produces harmless churn).
/// `%`, `&`/`|`/`^` and comparison ops are genuinely context-insensitive.
pub(crate) fn binop_is_widening(op: ast::BinOp) -> bool {
    use ast::BinOp::*;
    matches!(
        op,
        Add | Sub | Mul | Pow | Shl | AShl | Shr | AShr | Div | BitXnor
    )
}

/// Does `e` (a context-determined position with overall context width `cw`) contain a
/// widening op whose OWN self width is below `cw` — i.e. one the inline path evaluates
/// too narrowly and then extends? IEEE §11.6: every context-determined node in a region
/// shares one width `cw = max(target, self-widths in the region)`; a widening op is
/// mis-lowered exactly when its self width < `cw` (a wider sibling — a mask constant, a
/// ternary default, a wide literal — can lift `cw` above a nested narrow op's width).
/// `cw == None` = unknown context width (param-width target / unfoldable operand) ⇒ any
/// widening op counts (fail closed). Recurses only into context-determined positions; a
/// self-determined position (concat/replicate parts, comparison/logical/reduction
/// operands, a shift count / power exponent) evaluates at its own width and is skipped.
pub(crate) fn ctx_widening_below(
    e: &ast::Expr,
    cw: Option<u32>,
    widths: &std::collections::HashMap<String, u32>,
    func_widths: &std::collections::HashMap<String, u32>,
) -> bool {
    use ast::ExprKind::*;
    // A widening node is mis-lowered iff its own self width is below the context width.
    let below = |node: &ast::Expr| match (cw, ast_expr_self_width(node, widths, func_widths)) {
        (Some(c), Some(sw)) => sw < c,
        _ => true, // unknown context or unknown self width ⇒ conservative
    };
    let rec = |x: &ast::Expr| ctx_widening_below(x, cw, widths, func_widths);
    match &e.kind {
        Paren { inner } => rec(inner),
        Binary { op, lhs, rhs } => {
            use ast::BinOp::*;
            if binop_is_widening(*op) && below(e) {
                return true;
            }
            match op {
                Lt | Le | Gt | Ge | Eq | Ne | CaseEq | CaseNe | WildEq | WildNe | LogAnd
                | LogOr => false, // self-determined 1-bit result
                // A shift's count / power's exponent is self-determined — recurse only
                // into the (context-determined) LEFT operand.
                Shl | Shr | AShl | AShr | Pow => rec(lhs),
                // `%` / bitwise: both operands are context-determined — a narrower
                // widening op may nest in either.
                _ => rec(lhs) || rec(rhs),
            }
        }
        Unary { op, operand } => {
            use ast::UnOp::*;
            if matches!(op, Minus | BitNot) && below(e) {
                return true;
            }
            matches!(op, Plus | Minus | BitNot) && rec(operand)
        }
        Ternary { then_e, else_e, .. } => rec(then_e) || rec(else_e),
        _ => false,
    }
}

/// Self-determined bit width of an expression (`None` = unknown ⇒ conservative).
/// `widths` maps a formal/local NAME to its declared width.
pub(crate) fn ast_expr_self_width(
    e: &ast::Expr,
    widths: &std::collections::HashMap<String, u32>,
    func_widths: &std::collections::HashMap<String, u32>,
) -> Option<u32> {
    use ast::ExprKind::*;
    let rec = |x| ast_expr_self_width(x, widths, func_widths);
    match &e.kind {
        Paren { inner } => rec(inner),
        Ident(p) => match p.segments.as_slice() {
            [seg] => widths.get(&seg.name).copied(),
            _ => None,
        },
        // A single-segment call is a function call → its return width. (A 2-segment
        // `h.method()` is a handle method — unknown here → conservative `None`.)
        Call { name, .. } => match name.segments.as_slice() {
            [seg] => func_widths.get(&seg.name).copied(),
            _ => None,
        },
        IntLit { kind, raw } => literal::parse_int_literal(raw, *kind).map(|cv| cv.width),
        BitSelect { .. } => Some(1),
        PartSelect { msb, lsb, .. } => {
            let m = ast_decimal_lit_i64(msb)?;
            let l = ast_decimal_lit_i64(lsb)?;
            u32::try_from(m.abs_diff(l) + 1).ok()
        }
        IndexedPart { width, .. } => u32::try_from(ast_decimal_lit_i64(width)?).ok(),
        Binary { op, lhs, rhs } => {
            use ast::BinOp::*;
            match op {
                Lt | Le | Gt | Ge | Eq | Ne | CaseEq | CaseNe | WildEq | WildNe | LogAnd
                | LogOr => Some(1),
                // §11.6.1: a shift / power self-width is the LEFT operand's width.
                Shl | Shr | AShl | AShr | Pow => rec(lhs),
                _ => Some(rec(lhs)?.max(rec(rhs)?)),
            }
        }
        Unary { op, operand } => match op {
            ast::UnOp::LogNot
            | ast::UnOp::RedAnd
            | ast::UnOp::RedNand
            | ast::UnOp::RedOr
            | ast::UnOp::RedNor
            | ast::UnOp::RedXor
            | ast::UnOp::RedXnor => Some(1),
            _ => rec(operand),
        },
        Ternary { then_e, else_e, .. } => Some(rec(then_e)?.max(rec(else_e)?)),
        Concat { parts } => {
            let mut sum: u32 = 0;
            for p in parts {
                sum = sum.checked_add(rec(p)?)?;
            }
            Some(sum)
        }
        Replicate { count, value } => {
            let c = u32::try_from(ast_decimal_lit_i64(count)?).ok()?;
            let mut sum: u32 = 0;
            for v in value {
                sum = sum.checked_add(rec(v)?)?;
            }
            c.checked_mul(sum)
        }
        _ => None,
    }
}

/// Walk a straight-line body; `true` if any `lhs = rhs;` is a WIDENING context-
/// sensitive assignment — i.e. the RHS carries a widening op whose self width is below
/// the assignment's context width `CW = max(target width, whole-RHS self width)`. An
/// unfoldable (param-width) target, or an unknown RHS self width, makes `CW` unknown ⇒
/// any widening op routes (conservative → frame path, which is always correct).
pub(crate) fn assign_needs_context_width(
    s: &ast::Stmt,
    widths: &std::collections::HashMap<String, u32>,
    unknown_bv: &std::collections::HashSet<String>,
    func_widths: &std::collections::HashMap<String, u32>,
) -> bool {
    use ast::Stmt::*;
    match s {
        Block { stmts, .. } => stmts
            .iter()
            .any(|st| assign_needs_context_width(st, widths, unknown_bv, func_widths)),
        Blocking { lhs, rhs, .. } => {
            let ast::Lvalue::Ident(p) = lhs else {
                return false;
            };
            let [seg] = p.segments.as_slice() else {
                return false;
            };
            // Target width: `Some(w)` known; `None` but present in `unknown_bv` =
            // param-width bit-vector (conservative); neither = `real`/`string`/handle,
            // not a bit-width context (never route).
            let target_w = if let Some(&w) = widths.get(&seg.name) {
                Some(w)
            } else if unknown_bv.contains(&seg.name) {
                None
            } else {
                return false;
            };
            // Context width = max(target, whole-RHS self width). Unknown on either side
            // ⇒ `None` (conservative). Route iff some widening op is narrower than it.
            let cw = match (target_w, ast_expr_self_width(rhs, widths, func_widths)) {
                (Some(t), Some(r)) => Some(t.max(r)),
                _ => None,
            };
            ctx_widening_below(rhs, cw, widths, func_widths)
        }
        _ => false, // control flow ⇒ already framed by `body_needs_frame`
    }
}

impl Elaborator<'_> {
    /// Self-determined signedness of an ALREADY-LOWERED IR expr (IEEE §11.8.1),
    /// faithful to the engine width-table rule (`sim-engine/src/width.rs`). Only
    /// consulted where a cast must INHERIT the operand sign (size-cast truncate);
    /// function calls / exotic system functions conservatively report unsigned.
    pub(crate) fn expr_self_signed(&self, eid: u32) -> bool {
        match self.exprs.get(eid as usize) {
            Some(ir::Expr::Const { val }) => self
                .consts
                .get(*val as usize)
                .map(|c| {
                    matches!(c.repr, ir::ConstRepr::Real)
                        || (matches!(c.repr, ir::ConstRepr::Numeric) && c.signed)
                })
                .unwrap_or(false),
            Some(ir::Expr::Signal { net, .. }) => self
                .nets
                .get(*net as usize)
                .map(|n| n.signed)
                .unwrap_or(false),
            Some(ir::Expr::ArrayItem { signed, .. }) => *signed,
            // bit/part-select, concat, replicate are ALWAYS unsigned (§5.4.1).
            Some(ir::Expr::Select { .. })
            | Some(ir::Expr::Concat { .. })
            | Some(ir::Expr::Replicate { .. }) => false,
            // context-determined unary (+/-/~) follows the operand's sign;
            // reductions / logical-not are 1-bit unsigned.
            Some(ir::Expr::Unary {
                op: ir::UnOp::Plus | ir::UnOp::Minus | ir::UnOp::BitNot,
                operand,
            }) => self.expr_self_signed(*operand),
            Some(ir::Expr::Unary { .. }) => false,
            Some(ir::Expr::Binary { op, lhs, rhs }) => match op {
                ir::BinOp::Add
                | ir::BinOp::Sub
                | ir::BinOp::Mul
                | ir::BinOp::Div
                | ir::BinOp::Mod
                | ir::BinOp::BitAnd
                | ir::BinOp::BitOr
                | ir::BinOp::BitXor
                | ir::BinOp::BitXnor => self.expr_self_signed(*lhs) && self.expr_self_signed(*rhs),
                // power & shifts: sign follows the LEFT (base) operand only.
                ir::BinOp::Pow
                | ir::BinOp::Shl
                | ir::BinOp::Shr
                | ir::BinOp::AShl
                | ir::BinOp::AShr => self.expr_self_signed(*lhs),
                _ => false, // comparisons / case-eq / logical: 1-bit unsigned
            },
            Some(ir::Expr::Ternary { then_e, else_e, .. }) => {
                self.expr_self_signed(*then_e) && self.expr_self_signed(*else_e)
            }
            Some(ir::Expr::SysFunc { which, .. }) => matches!(
                which,
                ir::SysFuncId::Signed
                    | ir::SysFuncId::DynSize
                    | ir::SysFuncId::AssocNum
                    | ir::SysFuncId::AssocFirst
                    | ir::SysFuncId::AssocNext
                    | ir::SysFuncId::AssocLast
                    | ir::SysFuncId::AssocPrev
                    | ir::SysFuncId::Rtoi
                    | ir::SysFuncId::Stime
            ),
            _ => false, // Call / unhandled: conservatively unsigned
        }
    }

    /// Fold a straight-line combinational function body to one return ExprId.
    /// Supported shapes: a single `f = <expr>;`, or a `begin … end` of blocking
    /// assigns to locals (SSA-by-substitution) ending in the return-var assign.
    /// Anything with control flow / nonblocking / task call ⇒ E-ELAB-UNSUPPORTED.
    /// Resize an inlined-function assignment's rhs to the LHS-declared (width,
    /// sign). The inline SSA path (`fold_straight_line`) lowers each `lhs = rhs;`
    /// to a pure ExprId and never writes a net, so — unlike the frame-call/net
    /// path, which the engine resizes on every write — the return value and each
    /// body local otherwise keep their self-determined rhs width/sign. That was a
    /// silent-wrong: `function logic[3:0] f; f=8'hAB` returned `ab` not `b`;
    /// `function logic[7:0] f; f=4'hF` returned `f` not `0f`; a signed return read
    /// in a wider signed context did not sign-extend. Apply §10.7 here: truncate
    /// to / extend (by the RHS's OWN sign) to the declared width, then stamp the
    /// TARGET sign so a readback in a wider context extends correctly.
    ///
    /// The `$signed`/`$unsigned` tail is also the SEAL, for the reason
    /// `lower_size_cast` documents: the declared width is a self-determined
    /// boundary, but a bare `Binary`/`Unary`/`Ternary` node is CONTEXT-determined
    /// to the engine, so an enclosing width propagates through it and re-runs the
    /// whole body expression wider. The stamp used to be CONDITIONAL — skipped
    /// when the target's sign already matched the rhs's — and that is exactly the
    /// case where nothing sealed: `function [7:0] fm(input [7:0] x); fm = x*x;`
    /// read into a 64-bit destination printed `fe01` where iverilog prints `0001`.
    /// A 1,440-cell sweep put **48 wrong cells** all in that one hole (declared
    /// width == rhs self width AND unsigned target — a SIGNED target was sealed by
    /// accident, because an unsigned rhs made the stamp differ and appear).
    ///
    /// ⚠️ Sealing needs a TRUSTWORTHY rhs width: `ir_bits_of` answers `None` for a
    /// placeholder / string-producing / array-reduction rhs (and `rw` is then the
    /// declared `w`, forcing the same-width arm), and answers a FABRICATED `Some`
    /// for a class field (the 32-bit handle net). Sealing on either is a rung down
    /// — measured both directions in §4.5.320 — so the pre-slice tail is kept
    /// verbatim there.
    pub(crate) fn resize_inline_assign(&mut self, e: u32, w: u32, target_signed: bool) -> u32 {
        // real values are not bit-resizable — leave them untouched.
        if self.expr_is_real(e) {
            return e;
        }
        // G4: a string ExprId (`$sformatf(...)`, a string local/return) is a heap value,
        // not a bit-vector — a resize would corrupt it. Leave untouched.
        if self.ir_expr_is_string(e) {
            return e;
        }
        let trusted = self.trusted_self_width(e);
        let rw = trusted.or_else(|| self.ir_bits_of(e)).unwrap_or(w);
        let trusted_w = trusted.is_some();
        // The extension direction still comes from the old mirror — deliberately,
        // and NOT because the canonical rule is unreachable here. Two shapes reach
        // the widening arm with a canonical-vs-mirror sign disagreement, and they
        // want opposite things: a frame `Expr::Call` is impure, so `extend_to`'s
        // sign fill (a SECOND mention of the operand) would evaluate it twice;
        // a signed CLASS FIELD is a pure repeatable net read and would simply be
        // fixed (`function signed [63:0] fw; fw = c.sf;` with `sf = 8'hAB` is
        // `00…ab` for hand-IEEE's `ff…ab`). Adopting the canonical sign is
        // therefore a real fix gated on a repeatability predicate, i.e. its own
        // slice — ROADMAP §2 carries both shapes.
        let rhs_signed = self.expr_self_signed(e);
        let resized = match w.cmp(&rw) {
            std::cmp::Ordering::Equal => e,
            // Extend by the OPERAND's sign (§11.6.1); 4-state-preserving Concat.
            std::cmp::Ordering::Greater => self.extend_to(e, rw, w, rhs_signed),
            // Truncate to the low W bits (Select is unsigned).
            std::cmp::Ordering::Less => self.select_low(e, w),
        };
        if !trusted_w {
            // Width fabricated ⇒ no seal; the conditional stamp below is the
            // pre-slice tail verbatim.
            let resized_signed = if w == rw { rhs_signed } else { false };
            return if target_signed && !resized_signed {
                self.push_expr(ir::Expr::SysFunc {
                    which: ir::SysFuncId::Signed,
                    args: vec![resized],
                })
            } else if !target_signed && resized_signed {
                self.push_expr(ir::Expr::SysFunc {
                    which: ir::SysFuncId::Unsigned,
                    args: vec![resized],
                })
            } else {
                resized
            };
        }
        // `extend_to`/`select_low` are unsigned and the same-width arm keeps `e`'s
        // own sign, so the stamp is what makes the value carry the TARGET's sign —
        // and, unconditionally, what makes it self-determined.
        let which = if target_signed {
            ir::SysFuncId::Signed
        } else {
            ir::SysFuncId::Unsigned
        };
        self.push_expr(ir::Expr::SysFunc {
            which,
            args: vec![resized],
        })
    }

    /// `e`'s self width, but only when it can be TRUSTED — the one spelling of the
    /// §4.5.320/321 guard. Elaborate's own mirror (`ir_bits_of`) is the answer, and
    /// the canonical rule is the check on it: `None` from the mirror means the width
    /// is unknown here (a placeholder / string-producing / array-reduction rhs), and
    /// a canonical answer that DISAGREES means the mirror fabricated one (a class
    /// field lowers to a 32-bit handle net whose real width lives only in a sidecar).
    /// Resizing or sealing on either is a rung down the ladder — measured in both
    /// directions in §4.5.320 — so a caller that gets `None` must keep its pre-slice
    /// behavior rather than guess.
    pub(crate) fn trusted_self_width(&mut self, e: u32) -> Option<u32> {
        let known = self.ir_bits_of(e)?;
        let canon = self.canonical_self_width(e).map(|s| s.width);
        canon.is_none_or(|c| c == known).then_some(known)
    }

    // ── const + expr helpers (single arena append points) ──────────
    /// THE deterministic expr append point.
    #[inline]
    pub(crate) fn push_expr(&mut self, e: ir::Expr) -> u32 {
        let id = self.exprs.len() as u32;
        self.exprs.push(e);
        id
    }

    /// §5.7.1: re-size a fill-literal rhs to the assignment-context width. An
    /// unsized FILL literal (`'0`/`'1`/`'x`/`'z`) is CONTEXT-determined — `a = '1`
    /// into a 64-bit reg means all 64 ones, not a 1-bit one zero-extended.
    ///
    /// The caller lowers `rhs` normally first; when the rhs carries a fill in a
    /// context-propagating position this RE-LOWERS the whole subtree at the lvalue
    /// width and returns the new id, abandoning the first. The abandoned id is
    /// unreachable from any statement root — `lower_expr`/`lower_expr_ctx` never take
    /// a `ProcessBuilder` and so cannot emit a statement — so it costs dead arena and
    /// duplicated ELABORATE-TIME diagnostics, never a duplicated runtime effect.
    ///
    /// ⚠️ Two claims this comment used to make are false and were removed (§4.5.353):
    /// there is no `width != 32` guard (the re-lowering is unconditional once a fill is
    /// present, so a 32-bit lvalue does NOT keep its bytes), and the result is not
    /// necessarily a `Const` (a fill-bearing Binary/Ternary/Concat comes back as its
    /// own node). What IS true: for a NON-fill rhs the lvalue width is never read and
    /// `rhs_id` is returned untouched, which is what makes every fill-free design
    /// byte-identical — an error-recovery lvalue (`x = 1` for undeclared `x`) included.
    pub(crate) fn resize_fill_rhs(&mut self, rhs: &ast::Expr, rhs_id: u32, lv: &ir::Lvalue) -> u32 {
        // The rhs has no fill in a context-propagating position ⇒ untouched
        // (byte-identical; `lower_expr` already produced the right IR).
        if !expr_contains_fill(rhs) {
            return rhs_id;
        }
        // ⚠️ A `real` TARGET HAS NO BIT CONTEXT (IEEE 1800 §6.12 / §11.6: a real has
        // no width to propagate, and §5.7.1's "fill every bit of the context" has no
        // bits to fill). `ir_lvalue_width` answers 64 for a real net — the storage
        // size — and taking that as the context turns `'1` into 2^64-1, which the
        // engine then converts to 1.84467e+19. Both oracles give 1.0, i.e. the fill's
        // own self-determined 1-bit value converted to real.
        //
        // The guard lives HERE and not at the call sites for two reasons: it is one
        // spelling of one rule for all five callers, and the pre-existing callers were
        // ALREADY wrong this way — `real e; e = '1;`, `real d = '1;` and
        // `real f[0] = '1;` all read 1.84467e+19 before this slice. Adding the third
        // and fourth call sites (force / procedural continuous assign) would have
        // spread that from three spellings to five; putting the guard in the shared
        // funnel repairs all of them instead. (§4.5.353, found by adversarial review:
        // the first draft guarded nothing and regressed `force r = '1;` from a correct
        // 1 to 1.84467e+19 — correct→silent-wrong, which the ladder forbids.)
        if self.lvalue_targets_real(lv) {
            return rhs_id;
        }
        let lv_width = self.ir_lvalue_width(lv);
        // Re-lower the rhs with the lvalue width as the assignment context so every
        // fill in a context-determined position grows to that width (IEEE §11.6).
        // The originally-lowered `rhs_id` (sized self-determined) becomes dead — a
        // fill-bearing rhs has no golden to preserve, so this is harmless.
        self.lower_expr_ctx(rhs, lv_width)
    }

    /// Does this lvalue write a `real`? Used only to withhold the fill context
    /// (a real has no bit width to propagate).
    ///
    /// ANY chunk being real is enough: a concat mixing a real with a vector is not a
    /// legal assignment target, so the question cannot be half-true in valid code, and
    /// answering `true` on a malformed one merely leaves `lower_expr`'s IR in place —
    /// the fail-safe direction, since that is the behaviour before this guard existed.
    pub(crate) fn lvalue_targets_real(&self, lv: &ir::Lvalue) -> bool {
        lv.chunks.iter().any(|c| {
            self.nets
                .get(c.net as usize)
                .is_some_and(|n| n.kind == ir::NetKind::Real)
        })
    }

    /// Lower `e` in a context of width `ctx` (IEEE §11.6/§11.8.1), propagating the
    /// width to context-determined operand positions so an unsized fill grows to
    /// the context width. Only reached for fill-bearing expressions (the gate in
    /// `lower_expr` and `lower_ctx_or_plain`); a non-fill sub-expression falls
    /// through to the byte-identical `lower_expr`.
    pub(crate) fn lower_expr_ctx(&mut self, e: &ast::Expr, ctx: u32) -> u32 {
        use ast::ExprKind::*;
        match &e.kind {
            Paren { inner } => self.lower_expr_ctx(inner, ctx),
            // A fill literal → a const of the context width (≥ 1 bit).
            IntLit { kind, raw } if literal::is_fill_literal(raw, *kind) => {
                let w = ctx.max(1);
                let cv = literal::fill_literal_const(raw, *kind, w)
                    .unwrap_or_else(|| make_const_u32(0, w));
                let cid = self.intern_const(cv);
                self.push_expr(ir::Expr::Const { val: cid })
            }
            Binary { op, lhs, rhs } => {
                use ast::BinOp::*;
                // §11.4.6: wildcard equality is context-INDEPENDENT (a 1-bit
                // self-determined comparison) — route to the dedicated lowering.
                if matches!(op, WildEq | WildNe) {
                    return self.lower_wildcard_eq(lhs, rhs, matches!(op, WildNe));
                }
                // ⚠️ NEITHER A STRING COMPARE NOR A HANDLE COMPARE HAS A BIT-WIDTH
                // CONTEXT TO TAKE (IEEE §6.16 / §8.4) — the same shape as the `real`
                // guard in `resize_fill_rhs` (§6.12). `lower_expr`'s `Binary` arm
                // routes both, in this position; this twin routed neither, so a fill
                // literal ANYWHERE in the node was a bypass of both:
                //   • `s < {"a",'1}` compared PACKED (zero-extends MSB-side, so not
                //     lexicographic for unequal lengths) ⇒ 0, where both oracles say
                //     1 — and where the twin `s < {"a",1'b1}` already said 1;
                //   • `h == '1` printed a made-up 0 at exit 0, while the twin
                //     `h == 1'b1` is E3009. That one is loud→silent.
                if self.binary_stops_ctx(*op, lhs, rhs) {
                    return self.lower_expr_ungated(e);
                }
                let irop = map_binop(*op);
                // logical &&/|| : operands self-determined (1-bit truth) — ctx stops.
                if matches!(op, LogAnd | LogOr) {
                    let l = self.lower_ctx_or_plain(lhs, 0);
                    let r = self.lower_ctx_or_plain(rhs, 0);
                    if let Some(id) = self.binary_real_operand_route(irop, l, r) {
                        return id;
                    }
                    return self.push_expr(ir::Expr::Binary {
                        op: irop,
                        lhs: l,
                        rhs: r,
                    });
                }
                // shifts and `**`: LEFT operand context-determined, RIGHT
                // self-determined (IEEE Table 11-21 puts a power's exponent in the
                // same row as a shift's amount). `Pow` was missing here, so a
                // context width leaked into the exponent: `logic [7:0] r = a ** '1`
                // sized the fill to 8 bits and computed 2**255 instead of 2**1
                // (vita 0, both oracles 2), and a narrow signed literal exponent in
                // a continuous assign widened into a positive number.
                if matches!(op, Shl | Shr | AShl | AShr | Pow) {
                    let l = self.lower_ctx_or_plain(lhs, ctx);
                    let r = self.lower_expr(rhs);
                    // ⚠️ This is the branch that owns BOTH real rules a fill used to
                    // switch off: the shifts are permanently illegal on a real operand
                    // and `**` is the §11.4.9 `$pow` ROUTE, not a diagnostic.
                    if let Some(id) = self.binary_real_operand_route(irop, l, r) {
                        return id;
                    }
                    return self.push_expr(ir::Expr::Binary {
                        op: irop,
                        lhs: l,
                        rhs: r,
                    });
                }
                // arith/bitwise: operands sized to max(ctx, both self-widths).
                // comparison: operands sized to max of the two self-widths ONLY (the
                // 1-bit result does not let the outer ctx into the operands).
                let is_cmp = matches!(op, Eq | Ne | Lt | Le | Gt | Ge | CaseEq | CaseNe);
                let base = if is_cmp { 0 } else { ctx };
                let lf = expr_contains_fill(lhs);
                let rf = expr_contains_fill(rhs);
                let (l, r) = if lf && !rf {
                    // lower the NON-fill side first; its width sets the fill side's ctx.
                    let r = self.lower_expr(rhs);
                    let w = self.sibling_ctx(base, r);
                    let l = self.lower_expr_ctx(lhs, w);
                    (l, r)
                } else if rf && !lf {
                    let l = self.lower_expr(lhs);
                    let w = self.sibling_ctx(base, l);
                    let r = self.lower_expr_ctx(rhs, w);
                    (l, r)
                } else {
                    // both fills (or a fill nested under each) — size to ctx (≥1).
                    let w = base.max(1);
                    (self.lower_expr_ctx(lhs, w), self.lower_expr_ctx(rhs, w))
                };
                if let Some(id) = self.binary_real_operand_route(irop, l, r) {
                    return id;
                }
                self.push_expr(ir::Expr::Binary {
                    op: irop,
                    lhs: l,
                    rhs: r,
                })
            }
            Unary { op, operand } => {
                use ast::UnOp::*;
                // reductions / ! : operand self-determined; +,-,~ : context-determined.
                let self_det = matches!(
                    op,
                    LogNot | RedAnd | RedNand | RedOr | RedNor | RedXor | RedXnor
                );
                let o = self.lower_ctx_or_plain(operand, if self_det { 0 } else { ctx });
                self.push_expr(ir::Expr::Unary {
                    op: map_unop(*op),
                    operand: o,
                })
            }
            Ternary {
                cond,
                then_e,
                else_e,
            } => {
                let c = self.lower_expr(cond); // condition self-determined
                                               // branches are sized to max(ctx, both branch self-widths) — like a
                                               // binary op, so a fill branch grows to its sibling's width even in a
                                               // self-determined outer context (`(c)?'1:32'd7` ⇒ 32-bit).
                let tf = expr_contains_fill(then_e);
                let ff = expr_contains_fill(else_e);
                let (t, f) = if tf && !ff {
                    let f = self.lower_expr(else_e);
                    // Same real rule as the binary arm: `c ? r : '1` read 0 where both
                    // oracles (and `c ? r : 1'b1`) read 1.
                    let w = self.sibling_ctx(ctx, f);
                    (self.lower_expr_ctx(then_e, w), f)
                } else if ff && !tf {
                    let t = self.lower_expr(then_e);
                    let w = self.sibling_ctx(ctx, t);
                    (t, self.lower_expr_ctx(else_e, w))
                } else {
                    let w = ctx.max(1);
                    (
                        self.lower_expr_ctx(then_e, w),
                        self.lower_expr_ctx(else_e, w),
                    )
                };
                self.push_expr(ir::Expr::Ternary {
                    cond: c,
                    then_e: t,
                    else_e: f,
                })
            }
            // ⭐ `Concat` and `Replicate` HAVE NO ARM HERE, on purpose. Their operands
            // are SELF-determined (§11.4.12), so this function's whole job — carrying
            // a width inward — does not apply: the arms that used to be here passed
            // `ctx = 0` to every operand, which is by definition what `lower_expr`
            // already does. They were pure duplication, and duplication that had
            // silently fallen behind the original: the `Concat` twin never set
            // `repl_zero_ok`, so `{ {0{x}}, '1 }` was FALSE-LOUD where iverilog prints
            // `000000000001` and where the no-fill `{ {0{x}}, 1'b1 }` already worked;
            // the `Replicate` twin lowered its count through a bare `lower_index_expr`
            // with none of the §11.4.12.2 rules, so `parameter int N = -2; {N{'1}}`
            // rendered `111111111111` at exit 0 while `{N{1'b1}}` was correctly E3009.
            // Deleting them fixes both by construction and cannot drift again.
            // The transparent branch selector: `lower_expr` picks `typ` and drops the
            // other two, so the context belongs to `typ`. Without this arm the node
            // would fall to `_` below, be lowered without a context, and lose the
            // width that `expr_contains_fill`'s `MinTypMax` arm just earned it.
            MinTypMax { typ, .. } => self.lower_expr_ctx(typ, ctx),
            // ⚠️⚠️ `lower_expr_ungated` HERE IS LOAD-BEARING, NOT STYLE. Since the
            // `Concat`/`Replicate` arms above were deleted, those two kinds fall to
            // THIS arm — and both are `is_ctx_node` kinds carrying a fill, so plain
            // `lower_expr(e)` would re-fire the front gate and bounce straight back
            // into this function. That is the original §4.5.354 non-termination,
            // rebuilt. (An earlier draft of this comment claimed the two spellings
            // were byte-identical "because every `is_ctx_node` kind has an explicit
            // arm above". That was true before the deletion and false after it; the
            // mutation battery is what caught the stale claim — the mutant that
            // restores `lower_expr(e)` fails 8 tests.)
            _ => self.lower_expr_ungated(e),
        }
    }

    /// The context a lowered SIBLING lends to the fill-bearing side: the wider of the
    /// enclosing context and the sibling's own width — unless the sibling is REAL, in
    /// which case it lends nothing.
    ///
    /// Three callers, all the same shape: a binary operator's other operand, a
    /// ternary's other branch, and a `case` selector against its labels.
    ///
    /// ⚠️ A REAL OPERAND HAS NO BIT WIDTH (IEEE §6.12), and `ir_bits_of` answers 64
    /// for one — its STORAGE size, not a width the language ever exposes. Taking that
    /// as the fill's context made `real r = 2.5; a = r + '1;` compute
    /// `2.5 + (2^64 - 1)` and print 0, where BOTH oracles print 4 and where the
    /// `r + 1'b1` spelling beside it already printed 4. §11.8.1 makes the integral
    /// operand of a mixed expression convert to real; its own self-determined width
    /// is all it has, so the fill is one bit — which is exactly what the oracles'
    /// `2.5 + 1 = 3.5 → 4` says it is. Same trap as `ir_lvalue_width` in §4.5.353,
    /// one level down: at the OPERATOR rather than the assignment.
    pub(crate) fn sibling_ctx(&self, base: u32, sibling: u32) -> u32 {
        if self.expr_is_real(sibling) {
            return 0;
        }
        base.max(self.ir_bits_of(sibling).unwrap_or(32))
    }

    /// Does `lower_expr`'s `Binary` arm send this comparison down a route whose
    /// result has NO BIT WIDTH, so an enclosing width context must stop?
    ///
    /// ⭐ Both arms are VERBATIM copies of the conditions at the `lower_expr` site,
    /// in that site's order, so the two can be diffed rather than reasoned about; a
    /// route added there and not here is this bug again. `WildEq`/`WildNe` are absent
    /// because BOTH paths intercept them before either route.
    pub(crate) fn binary_stops_ctx(
        &self,
        op: ast::BinOp,
        lhs: &ast::Expr,
        rhs: &ast::Expr,
    ) -> bool {
        // expr_main.rs `Binary` arm — N7 handle type gate (IEEE §8.4). True for the
        // LEGAL handle compare as well as the loud one: an object id has no width
        // either, and copying the site's own `any_handle` test keeps this a copy
        // rather than a guess at that test's outcome.
        let lk = self.ast_handle_kind(lhs);
        let rk = self.ast_handle_kind(rhs);
        if matches!(lk, HKind::Handle | HKind::Null) || matches!(rk, HKind::Handle | HKind::Null) {
            return true;
        }
        // expr_main.rs `Binary` arm — v7 P2-C StrCmp route.
        matches!(
            op,
            ast::BinOp::Eq
                | ast::BinOp::Ne
                | ast::BinOp::Lt
                | ast::BinOp::Le
                | ast::BinOp::Gt
                | ast::BinOp::Ge
        ) && (self.expr_is_string_ast(lhs) || self.expr_is_string_ast(rhs))
    }

    /// Lower `e` with context width `ctx` if it contains a fill in a context-
    /// propagating position; otherwise the byte-identical plain `lower_expr`.
    pub(crate) fn lower_ctx_or_plain(&mut self, e: &ast::Expr, ctx: u32) -> u32 {
        if expr_contains_fill(e) {
            self.lower_expr_ctx(e, ctx)
        } else {
            self.lower_expr(e)
        }
    }

    /// Placeholder used after an error so downstream edges stay valid.
    pub(crate) fn placeholder_expr(&mut self) -> u32 {
        let cid = self.intern_const(make_const_u32(0, 1));
        self.push_expr(ir::Expr::Const { val: cid })
    }

    /// Self-determined width of an already-lowered expr (mirrors the engine's
    /// width table rules over the partial arena). `None` ⇒ loud at the caller.
    pub(crate) fn ir_bits_of(&self, eid: u32) -> Option<u32> {
        let e = self.exprs.get(eid as usize)?;
        Some(match e {
            ir::Expr::Const { val } => self.consts.get(*val as usize)?.width.max(1),
            ir::Expr::Signal { net, .. } => {
                let nv = self.nets.get(*net as usize)?;
                // review F1: a String handle's table width is 0 — `.max(1)`
                // made `$bits(s)` a silent 1. Dynamic length ⇒ loud at site.
                if nv.kind == ir::NetKind::String {
                    return None;
                }
                nv.width.max(1)
            }
            ir::Expr::Select { width, kind, .. } => match kind {
                ir::SelKind::Bit => 1,
                // direct Const OR the synthesized `Add(Sub(msb,lsb),1)` width
                // tree (mirrors the engine's shallow width-edge fold).
                _ => self.width_edge_u32(*width)?,
            },
            ir::Expr::Concat { parts } => {
                let mut s: u64 = 0;
                for &p in parts {
                    s += self.ir_bits_of(p)? as u64;
                }
                u32::try_from(s).ok()?
            }
            ir::Expr::Replicate { count, value } => {
                let c = self.width_edge_u32(*count)? as u64;
                let vw = self.ir_bits_of(*value)? as u64;
                u32::try_from(c * vw).ok()?
            }
            ir::Expr::Unary { op, operand } => match op {
                ir::UnOp::Plus | ir::UnOp::Minus | ir::UnOp::BitNot => self.ir_bits_of(*operand)?,
                _ => 1, // reductions / LogNot
            },
            ir::Expr::Binary { op, lhs, rhs } => {
                use ir::BinOp::*;
                match op {
                    Add | Sub | Mul | Div | Mod | BitAnd | BitOr | BitXor | BitXnor => {
                        self.ir_bits_of(*lhs)?.max(self.ir_bits_of(*rhs)?)
                    }
                    // power / shifts: width = LEFT operand (IEEE Table 11-21); the
                    // RHS (exponent / shift amount) is self-determined.
                    Pow | Shl | Shr | AShl | AShr => self.ir_bits_of(*lhs)?,
                    _ => 1, // comparisons / case(z/x) / logical
                }
            }
            ir::Expr::Ternary { then_e, else_e, .. } => {
                self.ir_bits_of(*then_e)?.max(self.ir_bits_of(*else_e)?)
            }
            // ⓑ-breadth (v17): with-clause iterator carries its own width.
            ir::Expr::ArrayItem { width, .. } => (*width).max(1),
            ir::Expr::SysFunc { which, args } => {
                use ir::SysFuncId as F;
                match which {
                    F::Time | F::Realtime | F::Itor | F::BitsToReal | F::RealToBits
                    // v18: `.atoreal()` → 64-bit real.
                    | F::StrAtoreal
                    // v19: N6 real-math (§20.8.2) — all return 64-bit real.
                    | F::Ln
                    | F::Log10
                    | F::Exp
                    | F::Sqrt
                    | F::Pow
                    | F::Floor
                    | F::Ceil
                    | F::Sin
                    | F::Cos
                    | F::Tan
                    | F::Asin
                    | F::Acos
                    | F::Atan
                    | F::Atan2
                    | F::Hypot
                    | F::Sinh
                    | F::Cosh
                    | F::Tanh
                    | F::Asinh
                    | F::Acosh
                    | F::Atanh => 64,
                    F::Signed | F::Unsigned => {
                        let a = *args.first()?;
                        self.ir_bits_of(a)?
                    }
                    F::Clog2
                    | F::Rtoi
                    | F::DynSize
                    | F::AssocNum
                    | F::AssocFirst
                    | F::AssocNext
                    | F::AssocLast
                    | F::AssocPrev
                    | F::Random
                    | F::Urandom
                    | F::UrandomRange
                    | F::CountOnes
                    | F::Stime
                    | F::Fopen
                    | F::TestPlusargs
                    | F::ValuePlusargs
                    | F::StrLen
                    | F::StrCmp
                    // v9 file-read family + $dist_* + $cast: all return `int`.
                    | F::Fgets
                    | F::Fscanf
                    | F::Sscanf
                    | F::Fread
                    | F::Feof
                    | F::Fgetc
                    | F::Ungetc
                    | F::DistUniform
                    | F::DistNormal
                    | F::DistExponential
                    | F::DistPoisson
                    | F::DistChiSquare
                    | F::DistT
                    | F::DistErlang
                    | F::Cast
                    // v18: string→int conversions — all `int`.
                    | F::StrAtoi
                    | F::StrAtohex
                    | F::StrAtooct
                    | F::StrAtobin => 32,
                    F::AssocExists | F::OneHot | F::OneHot0 | F::IsUnknown => 1,
                    F::StrGetC => 8,
                    // element-typed pops / dynamic-length string producers /
                    // element-typed array reductions — width is the element
                    // (handle) type, not a SysFunc-intrinsic constant.
                    F::QPopBack
                    | F::QPopFront
                    | F::Sformatf
                    | F::StrSubstr
                    | F::StrToUpper
                    | F::StrToLower
                    | F::ArrSum
                    | F::ArrProduct
                    | F::ArrAnd
                    | F::ArrOr
                    | F::ArrXor => return None,
                }
            }
            // A user function call's width is its declared return width (so a fill
            // sibling — `case(f8()) '1:`, `x == f16()` — sizes to it, not 32).
            ir::Expr::Call { func, .. } => self.func_metas.get(*func as usize)?.ret_width.max(1),
        })
    }
}
