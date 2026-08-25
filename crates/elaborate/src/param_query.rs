//! Parameter QUERY + expression-patch helpers — the read-only side of `params.rs`,
//! split out when that file crossed the 1000-line policy. Nothing here binds a
//! parameter; these answer questions ABOUT one (does this expression read a real
//! param / a const-array element / a real-lowering param?) and rewrite an already-
//! lowered expression slot to a folded constant.

use super::*;

impl Elaborator<'_> {
    /// True if a (constant) replication-count expression reads a const-array
    /// ELEMENT (`CNT[i]`) anywhere — directly or inside an arithmetic wrapper
    /// (`CNT[0]+1`, `-CNT[0]`, `c ? CNT[0] : 1`). Such an element read is not a
    /// runtime net the engine can fold, so a foldable count containing one must
    /// be materialized as a literal (else it reads 0 → 0-width). Recurses only
    /// the node kinds a constant count uses; `const_array_vals_of_base` gates the
    /// `BitSelect` on a genuine const array (a packed-vector bit-select or a
    /// runtime array read is NOT one → left to the ordinary lowering).
    pub(crate) fn count_reads_const_array_elem(&self, e: &ast::Expr) -> bool {
        match &e.kind {
            ast::ExprKind::BitSelect { base, .. } => self.const_array_vals_of_base(base).is_some(),
            ast::ExprKind::Paren { inner } => self.count_reads_const_array_elem(inner),
            ast::ExprKind::Unary { operand, .. } => self.count_reads_const_array_elem(operand),
            ast::ExprKind::Binary { lhs, rhs, .. } => {
                self.count_reads_const_array_elem(lhs) || self.count_reads_const_array_elem(rhs)
            }
            ast::ExprKind::Ternary {
                cond,
                then_e,
                else_e,
            } => {
                self.count_reads_const_array_elem(cond)
                    || self.count_reads_const_array_elem(then_e)
                    || self.count_reads_const_array_elem(else_e)
            }
            // `$clog2(CNT[i])` etc. — the element read hides inside a system-call
            // arg (`const_eval_in_scope` folds `$clog2`/`$bits`).
            ast::ExprKind::SysCall { args, .. } => {
                args.iter().any(|a| self.count_reads_const_array_elem(a))
            }
            _ => false,
        }
    }

    /// r19: is `name` a REAL parameter with NO exact integer twin? A real param whose
    /// initializer const-folded to an i64 is registered in BOTH `real_param_val` and
    /// `params` — both representations are exact and agree — so it keeps every integral
    /// capability it had before this slice (`logic [R-1:0]`, `generate if (R > 2)`, …)
    /// while `R/2` still divides in the real domain. Only a param with no i64 twin
    /// (`= 1.5`) is non-integral and must go loud in an integral context.
    ///
    /// Resolves over the COMBINED binding set — an independent walk of `real_param_val` alone
    /// would match an OUTER real param even when an inner net / numeric param shadows
    /// it, resolving one name two different ways.
    pub(crate) fn real_param_is_non_integral(&self, name: &str) -> bool {
        let Some(key) = self.walk_scopes_key(name, |k| {
            self.real_param_val.contains_key(k)
                || self.params.contains_key(k)
                || self.symbols.contains_key(k)
        }) else {
            return false;
        };
        self.real_param_val.contains_key(&key) && !self.params.contains_key(&key)
    }

    /// r19: does `e` read a REAL-valued parameter? A real param is deliberately kept
    /// out of `params` (it has no i64 value), so `const_eval_in_scope` returns None
    /// for it and a constant-required context that lacks its own loud gate silently
    /// folded to 0 — `{int'(R){1'b1}}` printed `0` instead of `11`. The loud twin of
    /// the array-element / runtime-net count detectors, same recursive shape.
    /// r19/B2: does `name` lower to a REAL value? `lower_expr`'s Ident arm prefers
    /// `real_param_val` over `params`, so a real param WITH an exact i64 twin still
    /// lowers to a real `Const`. A consumer that goes through `lower_expr` must ask
    /// this, not `real_param_is_non_integral` — that one models the const-FOLD
    /// resolver (`params`), and a `parameter real R = 4;` answers the two questions
    /// differently. Asking the wrong one let a real count reach `ir::Expr::Replicate`
    /// and emit 2^24 bits at exit 0. Same predicate, two resolvers: pick by consumer.
    pub(crate) fn real_param_lowers_real(&self, name: &str) -> bool {
        self.walk_scopes_key(name, |k| {
            self.real_param_val.contains_key(k)
                || self.params.contains_key(k)
                || self.symbols.contains_key(k)
        })
        .is_some_and(|k| self.real_param_val.contains_key(&k))
    }

    /// Is `e` an EXPLICIT real→integral conversion whose integer value this constant
    /// domain can actually PROVE?
    ///
    /// Both halves are load-bearing. The syntactic half names the context boundary:
    /// `int'(R)`, `$clog2(R)` and `$rtoi(R)` are integral by construction, so a
    /// "does this read a real parameter" walk that descends through them is asking
    /// about the wrong node. The `const_eval_in_scope` half is what keeps standing
    /// down FAIL-CLOSED — the gate this feeds exists because a replication count
    /// that does not fold becomes a SILENT 0-width replication, so a stand-down on
    /// the syntax alone would trade the loud for exactly that failure. Standing down
    /// only where a value has already been proved cannot.
    ///
    /// It deliberately does NOT cover an IMPLICIT conversion (a bare `R` in the same
    /// position). That is not a gap: the two oracles disagree there and in opposite
    /// directions — iverilog rejects `{R{1'b1}}` while verilator replicates 3 times,
    /// and for `logic [R-1:0]` it is verilator that rejects while iverilog sizes 3.
    /// An axis where the oracles split is one vita stays loud on.
    fn real_conversion_is_folded(&self, e: &ast::Expr) -> bool {
        let converts = match &e.kind {
            ast::ExprKind::Paren { inner } => return self.real_conversion_is_folded(inner),
            // `real'(…)` is excluded: its target is not integral, so it is not a
            // boundary into this domain at all.
            ast::ExprKind::Cast {
                target: ast::CastTarget::Prim(p),
                ..
            } => cast_prim_wsign(*p).is_some(),
            ast::ExprKind::SysCall { name, args } => {
                args.len() == 1 && matches!(name.name.as_str(), "$clog2" | "$rtoi")
            }
            _ => false,
        };
        converts && self.const_eval_in_scope(e).is_some()
    }

    /// r19: the `lower_expr`-resolver twin of [`Self::count_reads_real_param`], for
    /// consumers that lower their operand rather than const-folding it.
    pub(crate) fn count_lowers_real_param(&self, e: &ast::Expr) -> bool {
        // An EXPLICIT real→integral conversion is where this walk has to stop: at an
        // `int'()` cast or a `$clog2`/`$rtoi` call, everything below the node is real
        // and the node itself is integral by construction (§6.24.1 / §20.8.1 /
        // §20.10). Descending through it is what made `{int'(R){1'b1}}` loud on a
        // design BOTH oracles answer 7.
        if self.real_conversion_is_folded(e) {
            return false;
        }
        match &e.kind {
            ast::ExprKind::Ident(p) if p.segments.len() == 1 => {
                self.real_param_lowers_real(&p.segments[0].name)
            }
            ast::ExprKind::Call { args, .. } => {
                args.iter().any(|a| self.count_lowers_real_param(a))
            }
            ast::ExprKind::Paren { inner } => self.count_lowers_real_param(inner),
            ast::ExprKind::Unary { operand, .. } => self.count_lowers_real_param(operand),
            ast::ExprKind::Cast { expr, .. } => self.count_lowers_real_param(expr),
            ast::ExprKind::Binary { lhs, rhs, .. } => {
                self.count_lowers_real_param(lhs) || self.count_lowers_real_param(rhs)
            }
            // These two MUST be mirrored rather than delegated: the `_` fallback below
            // reaches `count_reads_real_param`, i.e. back to the const-FOLD resolver,
            // which answers `false` for a real param that has an exact i64 twin. That
            // is how `{$clog2(R){1'b1}}` with `parameter real R = 4;` still folded to a
            // silent 0 — one syntactic layer was enough to re-enter the wrong resolver.
            ast::ExprKind::Ternary {
                cond,
                then_e,
                else_e,
            } => {
                self.count_lowers_real_param(cond)
                    || self.count_lowers_real_param(then_e)
                    || self.count_lowers_real_param(else_e)
            }
            ast::ExprKind::SysCall { args, .. } => {
                args.iter().any(|a| self.count_lowers_real_param(a))
            }
            _ => self.count_reads_real_param(e),
        }
    }

    pub(crate) fn count_reads_real_param(&self, e: &ast::Expr) -> bool {
        match &e.kind {
            ast::ExprKind::Ident(p) if p.segments.len() == 1 => {
                self.real_param_is_non_integral(&p.segments[0].name)
            }
            // A const-FUNCTION call is the hole the bound guard was meant to be the only
            // net for: neither this walk nor `nonconst_bound_reason` descended into call
            // args, so `logic [f(R)-1:0]` folded to None and `clamp_bound_u32` silently
            // gave width 1 on a design iverilog answers.
            ast::ExprKind::Call { args, .. } => args.iter().any(|a| self.count_reads_real_param(a)),
            ast::ExprKind::Paren { inner } => self.count_reads_real_param(inner),
            ast::ExprKind::Unary { operand, .. } => self.count_reads_real_param(operand),
            ast::ExprKind::Cast { expr, .. } => self.count_reads_real_param(expr),
            ast::ExprKind::Binary { lhs, rhs, .. } => {
                self.count_reads_real_param(lhs) || self.count_reads_real_param(rhs)
            }
            ast::ExprKind::Ternary {
                cond,
                then_e,
                else_e,
            } => {
                self.count_reads_real_param(cond)
                    || self.count_reads_real_param(then_e)
                    || self.count_reads_real_param(else_e)
            }
            ast::ExprKind::SysCall { args, .. } => {
                args.iter().any(|a| self.count_reads_real_param(a))
            }
            // A SELECT of a real param (`logic [R[7:0]-1:0] v;`). Without these arms
            // this gate missed, control fell through to `nonconst_bound_reason`, and
            // the message was *"undefined name `R`"* about a param declared one line
            // up — while the plain `logic [R-1:0]` twin said the true thing. Both
            // spellings are loud either way (iverilog: "can not select part of real
            // parameter"), so this moves no rung; it stops the diagnostic from lying.
            // Unlike `collect_bare_idents` this walk has ONE consumer, so widening it
            // carries no shared-path hazard.
            ast::ExprKind::BitSelect { base, index } => {
                self.count_reads_real_param(base) || self.count_reads_real_param(index)
            }
            ast::ExprKind::PartSelect { base, msb, lsb } => {
                self.count_reads_real_param(base)
                    || self.count_reads_real_param(msb)
                    || self.count_reads_real_param(lsb)
            }
            ast::ExprKind::IndexedPart {
                base,
                offset,
                width,
                ..
            } => {
                self.count_reads_real_param(base)
                    || self.count_reads_real_param(offset)
                    || self.count_reads_real_param(width)
            }
            _ => false,
        }
    }

    /// True if a replication-count expression reads an UNPACKED-ARRAY element of
    /// ANY shape — including shapes `const_array_vals_of_base` cannot fold
    /// (descending, non-zero-based, multi-dimensional) and a RUNTIME array. Uses
    /// the array net directly (`net_is_static_array`), so it is the loud-gate
    /// twin of [`Self::count_reads_const_array_elem`]: a count that reads such an
    /// element but does NOT const-fold is an invalid/unsupported constant count
    /// and must be LOUD (the engine would otherwise read 0 → silent 0-width),
    /// mirroring the loud `localparam R = ROT[i]` binding site. A scalar
    /// (packed-vector) net has `array_len == 1` → NOT flagged, so a packed
    /// bit/part-select count is left to the ordinary lowering (byte-identical).
    pub(crate) fn count_reads_array_param_elem(&self, e: &ast::Expr) -> bool {
        match &e.kind {
            ast::ExprKind::BitSelect { base, .. } => {
                self.base_is_array_net(base) || self.count_reads_array_param_elem(base)
            }
            ast::ExprKind::Paren { inner } => self.count_reads_array_param_elem(inner),
            ast::ExprKind::Unary { operand, .. } => self.count_reads_array_param_elem(operand),
            ast::ExprKind::Binary { lhs, rhs, .. } => {
                self.count_reads_array_param_elem(lhs) || self.count_reads_array_param_elem(rhs)
            }
            ast::ExprKind::Ternary {
                cond,
                then_e,
                else_e,
            } => {
                self.count_reads_array_param_elem(cond)
                    || self.count_reads_array_param_elem(then_e)
                    || self.count_reads_array_param_elem(else_e)
            }
            ast::ExprKind::SysCall { args, .. } => {
                args.iter().any(|a| self.count_reads_array_param_elem(a))
            }
            _ => false,
        }
    }

    /// Overwrite the deferred placeholder at `eid` (a `Signal`) with a `Const`
    /// folding the i64 hierarchical-param value `v` — same width/sign as
    /// [`Self::const_param_expr`] (byte-identical to how a bare param folds), but
    /// written IN PLACE so the existing arena edge keeps pointing at it.
    pub(crate) fn patch_expr_param_const(&mut self, eid: u32, v: i64) {
        let cv = if let Ok(u) = u32::try_from(v) {
            make_const_u32(u, 32)
        } else if i32::try_from(v).is_ok() {
            make_const_i64(v, 32, true)
        } else {
            make_const_i64(v, 64, v < 0)
        };
        let cid = self.intern_const(cv);
        if let Some(slot) = self.exprs.get_mut(eid as usize) {
            *slot = ir::Expr::Const { val: cid };
        }
    }

    /// Width-aware [`Self::patch_expr_param_const`]: a hierarchical read of a TYPED
    /// param (`dut.W` where `W` is `logic [63:0]`) materializes at its DECLARED
    /// width, mirroring the bare-param [`Self::const_param_expr_w`]. `None` meta
    /// (untyped param / no recorded width) falls back to value-inference.
    pub(crate) fn patch_expr_param_const_w(&mut self, eid: u32, v: i64, meta: Option<(u32, bool)>) {
        let cv = match meta {
            Some((w, signed)) if (1..=64).contains(&w) => make_const_i64(v, w, signed),
            _ => {
                self.patch_expr_param_const(eid, v);
                return;
            }
        };
        let cid = self.intern_const(cv);
        if let Some(slot) = self.exprs.get_mut(eid as usize) {
            *slot = ir::Expr::Const { val: cid };
        }
    }
}
