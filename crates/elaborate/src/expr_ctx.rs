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
    /// TARGET sign so a readback in a wider context extends correctly. A genuine
    /// no-op when the rhs already matches the declared width and sign — every such
    /// inline function (the common case) stays byte-identical.
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
        let rw = self.ir_bits_of(e).unwrap_or(w);
        let rhs_signed = self.expr_self_signed(e);
        let resized = match w.cmp(&rw) {
            std::cmp::Ordering::Equal => e,
            // Extend by the OPERAND's sign (§11.6.1); 4-state-preserving Concat.
            std::cmp::Ordering::Greater => self.extend_to(e, rw, w, rhs_signed),
            // Truncate to the low W bits (Select is unsigned).
            std::cmp::Ordering::Less => self.select_low(e, w),
        };
        // extend_to / select_low yield UNSIGNED results; the Equal case keeps `e`'s
        // own sign. Stamp the target sign ONLY when it differs from what `resized`
        // already carries — so an unsigned target with an unsigned rhs, or a signed
        // target with an already-signed rhs, adds no node (byte-identical).
        let resized_signed = if matches!(w.cmp(&rw), std::cmp::Ordering::Equal) {
            rhs_signed
        } else {
            false
        };
        if target_signed && !resized_signed {
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
        }
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
    /// unsized single-bit FILL literal (`'0`/`'1`/`'x`/`'z`) is CONTEXT-determined
    /// — `a = '1` into a 64-bit reg means all 64 ones, not the self-determined
    /// 32-bit default. The caller lowers `rhs` NORMALLY first (so non-fill ExprId
    /// ordering is untouched); this returns a fresh Const sized to the lvalue
    /// width ONLY when `rhs` is a fill AND that width != 32 (the 32-bit case is
    /// already correct and stays byte-identical — every existing golden keeps its
    /// bytes). For a NON-fill rhs the lvalue width is never even read, so an
    /// error-recovery lvalue (e.g. `x = 1` for undeclared `x`) is untouched.
    pub(crate) fn resize_fill_rhs(&mut self, rhs: &ast::Expr, rhs_id: u32, lv: &ir::Lvalue) -> u32 {
        // The rhs has no fill in a context-propagating position ⇒ untouched
        // (byte-identical; `lower_expr` already produced the right IR).
        if !expr_contains_fill(rhs) {
            return rhs_id;
        }
        let lv_width = self.ir_lvalue_width(lv);
        // Re-lower the rhs with the lvalue width as the assignment context so every
        // fill in a context-determined position grows to that width (IEEE §11.6).
        // The originally-lowered `rhs_id` (sized self-determined) becomes dead — a
        // fill-bearing rhs has no golden to preserve, so this is harmless.
        self.lower_expr_ctx(rhs, lv_width)
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
                let irop = map_binop(*op);
                // logical &&/|| : operands self-determined (1-bit truth) — ctx stops.
                if matches!(op, LogAnd | LogOr) {
                    let l = self.lower_ctx_or_plain(lhs, 0);
                    let r = self.lower_ctx_or_plain(rhs, 0);
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
                    let w = base.max(self.ir_bits_of(r).unwrap_or(32));
                    let l = self.lower_expr_ctx(lhs, w);
                    (l, r)
                } else if rf && !lf {
                    let l = self.lower_expr(lhs);
                    let w = base.max(self.ir_bits_of(l).unwrap_or(32));
                    let r = self.lower_expr_ctx(rhs, w);
                    (l, r)
                } else {
                    // both fills (or a fill nested under each) — size to ctx (≥1).
                    let w = base.max(1);
                    (self.lower_expr_ctx(lhs, w), self.lower_expr_ctx(rhs, w))
                };
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
                    let w = ctx.max(self.ir_bits_of(f).unwrap_or(32));
                    (self.lower_expr_ctx(then_e, w), f)
                } else if ff && !tf {
                    let t = self.lower_expr(then_e);
                    let w = ctx.max(self.ir_bits_of(t).unwrap_or(32));
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
            // concat/replication operands are SELF-determined → a fill is 1 bit.
            Concat { parts } => {
                if parts.iter().any(|p| self.expr_is_string_ast(p)) {
                    return self.lower_expr(e); // string concat path (loud / desugar)
                }
                let part_ids: Vec<u32> = parts
                    .iter()
                    .map(|p| self.lower_ctx_or_plain(p, 0))
                    .collect();
                if part_ids.iter().any(|&p| self.expr_is_real(p)) {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "real may not appear in concatenation (use $realtobits)",
                    );
                }
                self.push_expr(ir::Expr::Concat { parts: part_ids })
            }
            Replicate { count, value } => {
                // r19/S3: the SECOND replication-count lowering site, reached
                // whenever the replicated value contains a `'0`/`'1` fill.
                // Gating only the other one left `{R{'1}}` printing `ff`.
                let count = self.lower_index_expr(count);
                let part_ids: Vec<u32> = value
                    .iter()
                    .map(|p| self.lower_ctx_or_plain(p, 0))
                    .collect();
                if part_ids.iter().any(|&p| self.expr_is_real(p)) {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "real may not appear in concatenation (use $realtobits)",
                    );
                }
                let value = self.push_expr(ir::Expr::Concat { parts: part_ids });
                self.push_expr(ir::Expr::Replicate { count, value })
            }
            _ => self.lower_expr(e),
        }
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
