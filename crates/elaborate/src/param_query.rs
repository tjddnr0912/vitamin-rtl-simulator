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
            ast::ExprKind::BitSelect { base, .. } => {
                self.const_array_vals_of_base(base).is_some()
                    || self.count_reads_const_array_elem(base)
            }
            // §3 ⑤ ⓔ: a SELECT of an element (`{A[1][3:0]{4'hA}}`) reads the element
            // through its base — the same fold now answers it, so the same routing
            // rule applies.
            ast::ExprKind::PartSelect { base, .. } | ast::ExprKind::IndexedPart { base, .. } => {
                self.count_reads_const_array_elem(base)
            }
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
            // The package spelling of the same fact. Without it a `pkg::R` in a width
            // bound fell through to `nonconst_bound_reason` and was reported as an
            // undefined name — about a parameter that exists, in a package that is
            // imported. An explicitly qualified name has no shadowing question, so the
            // membership test is the whole answer.
            ast::ExprKind::PkgScoped { pkg, name } => self
                .pkg_real_val
                .get(&pkg.name)
                .is_some_and(|m| m.contains_key(&name.name)),
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
            // §3 ⑤ ⓔ: the loud-gate twin sees through a select of an element too.
            ast::ExprKind::PartSelect { base, .. } | ast::ExprKind::IndexedPart { base, .. } => {
                self.count_reads_array_param_elem(base)
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
            // Declared WIDER than 64 bits with a value that fits i64 (`logic [127:0]
            // SMALL = 128'h7`): such a parameter lives in `hier_params`, not the wide
            // side map, and used to fall to value-inference — `u.SMALL` printed 32 bits
            // where the bare read and both oracles print 128 (§4.5.421 review, both
            // lenses). The i64 value is the parameter's value at 64 bits; extend it to
            // the declared width with the declared sign.
            Some((w, signed)) if w > 64 => ir::ConstVal {
                width: w,
                signed,
                repr: ir::ConstRepr::Numeric,
                bits: resize_bits(&bp_from_limbs(vec![v as u64], 64), 64, w, signed),
            },
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

/// Why a reduction of constants declines: the one input it needs beyond the value.
///
/// ⚠️ Names every case the wide domain's name resolver refuses (`narrow_param_bits`),
/// not only the value-inferred one: a declared `[0:N]` (ascending) or `[H:L]` with a
/// non-zero low bound is a width the DECLARATION states and the bit domain still
/// declines, because it indexes positionally from 0 and carries no direction. Review
/// measured `parameter [0:3] P = 4'b1010; wire [(|P)+2:0] x;` — both oracles 4 — reading
/// a sentence that told the author to declare a range they had declared.
pub(crate) const REDUCTION_WIDTH_UNDECLARED: &str =
    "a reduction of an operand whose width the constant domain cannot read: a parameter \
     sized from its value (no range, type or sized literal), or one declared ascending \
     `[0:N]` / with a non-zero low bound `[H:L]`";

/// Is `e` (parens already peeled) one of the six §11.4.14 reduction operators?
pub(crate) fn is_reduction_top(e: &ast::Expr) -> bool {
    matches!(
        &e.kind,
        ast::ExprKind::Unary {
            op: ast::UnOp::RedAnd
                | ast::UnOp::RedOr
                | ast::UnOp::RedXor
                | ast::UnOp::RedNand
                | ast::UnOp::RedNor
                | ast::UnOp::RedXnor,
            ..
        }
    )
}

/// Does any node of `e` satisfy `pred`, over the arms the constant domain descends
/// into — `const_fold_children` plus the parts of a concatenation / replication
/// (which the placement fold walks on its own)?
pub(crate) fn ast_any(e: &ast::Expr, pred: &dyn Fn(&ast::Expr) -> bool) -> bool {
    if pred(e) {
        return true;
    }
    let parts: Vec<&ast::Expr> = match &e.kind {
        ast::ExprKind::Concat { parts } => parts.iter().collect(),
        ast::ExprKind::Replicate { count, value } => {
            std::iter::once(&**count).chain(value.iter()).collect()
        }
        _ => Elaborator::const_fold_children(e),
    };
    parts.into_iter().any(|p| ast_any(p, pred))
}

/// Does `e` mention a single-segment name for which `is_local` holds?
pub(crate) fn ast_names_any(e: &ast::Expr, is_local: &dyn Fn(&str) -> bool) -> bool {
    ast_any(e, &|x| match &x.kind {
        ast::ExprKind::Ident(p) if p.segments.len() == 1 => is_local(&p.segments[0].name),
        _ => false,
    })
}

fn ast_contains_reduction(e: &ast::Expr) -> bool {
    ast_any(e, &is_reduction_top)
}

/// True iff the expression contains an unsized fill (`'0 '1 'x 'z`) anywhere — the
/// shape whose value depends on the CONTEXT width (§5.7.1), so a sized initializer
/// holding one must be folded at its declared width rather than in the unlimited lane.
pub(crate) fn ast_contains_fill(e: &ast::Expr) -> bool {
    ast_any(e, &|x| crate::const_eval::fill_literal_ast(x).is_some())
}

impl Elaborator<'_> {
    /// Does the width-aware assignment walk own this parameter initializer?
    ///
    /// The OPT-IN half of [`Elaborator::eval_param_init`]'s gate — the CONSUMER asks
    /// for the width-aware evaluator; the evaluator is not widened underneath its
    /// other callers (ENGINEERING_RULES "shared machinery").
    ///
    /// The defect it admits: a parameter initializer folds in the width-UNLIMITED i64
    /// lane and is masked ONCE, at the end. Truncation commutes with `+ - * << & | ^`
    /// and the unary operators, so those cells are right by accident; it does NOT
    /// commute with `/ % >> >>>`, and there the discarded high bits are the ones that
    /// decide the answer. Measured, `localparam K = (8'd200 + 8'd100) >> 1;` binds
    /// `96` where both oracles bind `16` — the inner sum keeps 300 and the shift then
    /// reads a bit an 8-bit sum does not have. `$bits(K)` was already 8 (§4.5.460), so
    /// only the VALUE lane was left behind.
    ///
    /// Two conditions, each answering a MEASURED correct→silent-wrong:
    ///
    /// 1. [`Elaborator::ctx_width_names_are_evident`] — every NAME's width must be
    ///    stated by the expression, not looked up. `const_self_width`'s name arm reads
    ///    `param_meta`, which records an untyped parameter's DEFAULT literal width and
    ///    is replaced by the final override's type the moment `#(.C(…))` arrives
    ///    (§6.20.2). `envw` is empty at a module-scope initializer, so this admits
    ///    literal trees and declines every name — the same line `eval_const_shift_count`
    ///    drew after 21 cells went correct→silent-wrong through the other door.
    /// 2. [`Self::const_ctx_within_i64`] — no node wider than the i64 lane carries.
    ///    `eval_const_assign` computes `ctx = max(self, target).min(64)`, and that
    ///    `.min` is a CLAMP: for `localparam logic signed [64:0] N65 = -65'sd100;`,
    ///    `N65 >>> 1` is `ffff_ffff_ffff_ffce` at HEAD and in iverilog, and
    ///    `7fff_ffff_ffff_ffce` through the width-aware walk, because the sign bit
    ///    lives at bit 64 and masking to 64 deletes it. ROADMAP §2 row 14's reverted
    ///    slice named this leaf; it is still live and this is the clause that answers
    ///    it. (Row 14's OTHER named blocker — a generate-scope `localparam time NM`
    ///    shadowing a module `logic [7:0] NM` — was re-measured at HEAD and is dead:
    ///    both lanes and both oracles answer `12c`.)
    pub(crate) fn param_init_width_aware_ok(&self, e: &ast::Expr) -> bool {
        Self::ctx_width_names_are_evident(e, &ConstWidths::new()) && self.const_ctx_within_i64(e)
    }

    /// An override EXPRESSION's OWN `(width, signed)`, from Table 11-21 — ROADMAP §2
    /// row 25's operator half.
    ///
    /// §6.20.2 gives an UNTYPED, unranged parameter the type of its FINAL override
    /// value. vita had no channel carrying that type for an operator-topped override:
    /// `override_bits` only folds a SELF-DETERMINED top (`wide_top_is_self_determined`
    /// admits the reductions, the comparisons and the selects, not `~`, unary `-`, or
    /// the arithmetic binaries), so the meta chain in `bind_one_param` fell through to
    /// `param_decl_width_opt`'s literal arm — which answers the DEFAULT's type even
    /// when `default_binds == false`. Measured on `module sub #(parameter P = 1)`:
    /// `#(.P(-(|4'b1010)))` bound `ffffffff` at 32 signed bits, `#(.P(~8'h5A))` bound
    /// `ffffffa5` / `-91`, `#(.P(-64'd1))` bound 32 bits where the value needs 64.
    ///
    /// This is the DEFAULT lane's §4.5.460 arm (`params.rs`'s `sized_by_operator`
    /// block) one lane over, and it is a separate resolver ON PURPOSE: widening
    /// `override_bits` instead would route through `override_at_declared_width` and
    /// move ROADMAP §2 rows 16/17's live oracle splits (`~32'd0`, `32'd0-32'd1`) and
    /// the DECLARED-width targets they live on. Measured: on a declared `[63:0]`
    /// target the three tools genuinely disagree about `~32'd0`; on an UNTYPED,
    /// unranged target they do not disagree about any of row 16's five cells, and vita
    /// is alone and wrong on four of them. The consumer's `Implicit && range.is_none()`
    /// guard is what keeps this resolver on the second lane only.
    ///
    /// ORACLE STATUS, measured per cell rather than assumed: every tool answers the
    /// self-determined width when asked DIRECTLY (`$bits(<expr>)`), and each then
    /// contradicts its own direct answer when the identical text is BOUND — verilator
    /// on a reduction top (`$bits(|4'b1010)` is 1, `#(.P(|4'b1010))` binds 32),
    /// iverilog on `+` (`$bits(8'd200+8'd100)` is 8, the binding is 9; ROADMAP §2
    /// already records the second). So the target is the direct answer, which is also
    /// Table 11-21 and is already what vita binds for a reduction top today.
    ///
    /// ACCEPT SET — each conjunct answers a measured hazard, not a taste:
    ///  * `ctx_width_names_are_evident` with an EMPTY `ConstWidths` declines a bare
    ///    NAME in a WIDTH-carrying position (and `SysCall` / `Call` / `PkgScoped` /
    ///    `Replicate` / every select / `$signed`-`$unsigned`). It must:
    ///    `const_self_width`'s `Ident` arm resolves through `param_meta` and GUESSES 32
    ///    on a miss, and `param_meta` is exactly where value-INFERRED widths are
    ///    recorded — the §4.5.363 laundering door, and the declared-width-provenance
    ///    wall ROADMAP §2 rows 14/25/26/30 stand on. Fail-closed keeps a name-bearing
    ///    override at today's answer (`#(.P(W8 + 1'b0))` is unmoved, measured).
    ///
    ///    ⚠️ "declines every name" would be too strong, and the review round measured
    ///    where: a name reaches a SIZE CAST's width through `(W+1)'(3)`, because
    ///    `casts.rs:74` makes a compound size a `CastTarget::Size` and this predicate's
    ///    `Cast` arm recurses into the cast's OPERAND only. That is a VALUE position,
    ///    not a recorded-width one — `const_self_width` folds `W+1` to a number rather
    ///    than reading `param_meta` — so it is not the laundering door, and it lands on
    ///    the right answer (`#(.P((W+1)'(3) + 8'd1))` with `W = 52` binds 53, which is
    ///    verilator's; iverilog says 54, its known `+` self-contradiction). The BARE
    ///    spelling `W'(3)` never reaches here at all: `casts.rs:73` makes it
    ///    `CastTarget::Named`, which this predicate's catch-all refuses.
    ///  * `const_ctx_within_i64` — the value half below re-folds through
    ///    `eval_const_assign`, whose `ctx = max(self, target).min(64)` CLAMPS; a leaf
    ///    past 64 bits loses a sign bit to that clamp (§2 row 14's `N65 >>> 1`).
    ///  * no FILL anywhere — a fill has no width of its own (`const_self_width` answers
    ///    `Some(0)`), and `'1 ^ 1'b0` is a live split (iverilog 1 bit, verilator 32).
    ///  * the TOP is an operator: unary `+ - ~`, any binary, or a ternary — the
    ///    `sized_by_operator` set of the default lane's arm plus the `Ternary` that arm
    ///    keeps in its own block. A reduction / comparison / select top is deliberately
    ///    absent: `override_bits` already answers those and already answers them right.
    ///
    /// The gate costs nothing on the class it exists for — every cell in the row's
    /// census is a literal-only tree. It declines only the name-bearing residue
    /// (`#(.P(W8 + 1'b0))`), which keeps its pre-slice answer.
    pub(crate) fn override_self_meta(&self, e: &ast::Expr) -> Option<(u32, bool)> {
        let mut top = e;
        while let ast::ExprKind::Paren { inner } = &top.kind {
            top = inner;
        }
        let sized_by_operator = matches!(
            &top.kind,
            ast::ExprKind::Unary {
                op: ast::UnOp::Plus | ast::UnOp::Minus | ast::UnOp::BitNot,
                ..
            } | ast::ExprKind::Binary { .. }
                | ast::ExprKind::Ternary { .. }
        );
        // §2 "Index sealing" residue ⓐ: a NAME leaf whose DECLARED width this
        // scope can prove. Empty for a literal-only tree, which is every cell
        // §4.5.463's census covered, so that lane is byte-identical.
        let envw = self.declared_override_widths(e)?;
        if !sized_by_operator
            || ast_contains_fill(e)
            || !Self::ctx_width_names_are_evident(e, &envw)
            || !self.const_ctx_within_i64(e)
        {
            return None;
        }
        let w = self.const_self_width(top, &envw)?;
        // ⚠️ `const_signed_env`, not `const_expr_signed`. The latter's `Ident` arm
        // resolves through `self.fq()` — the CURRENT scope only — where both
        // `const_self_width` and this map walk the scope chain. Measured: with a
        // signed `S8` declared at module scope, `localparam K = S8 >>> 1` inside a
        // `generate if (1) begin:gb` folds to 255 while the identical text at
        // module scope folds to −1, and both oracles say −1 in both places. Taking
        // the sign from the same env the width came from is what stops this slice
        // inheriting that (pre-existing, recorded in ROADMAP §2) defect.
        (w > 0).then(|| (w, self.const_signed_env(top, &envw)))
    }

    /// Every bare NAME in an override expression, at the width its DECLARATION
    /// gives it — or `None` if any one of them cannot be proved.
    ///
    /// ⚠️ WHY THIS IS NOT `param_meta`. `const_self_width`'s `Ident` arm falls back
    /// to `walk_scopes(name, &self.param_meta)`, and `param_meta` holds the width a
    /// parameter's VALUE was inferred at, which for an untyped parameter is 32 and
    /// for an OVERRIDDEN one can be stale. Laundering that through this gate is what
    /// made `#(.P(W8 + 1'b0))` bind at 32 where both oracles bind at 8.
    ///
    /// ⚠️ AND WHY `param_range` ALONE IS NOT ENOUGH EITHER — the gate's own comment
    /// records an attempt that used it and put a design straight back to wrong. The
    /// resolver that works is already in the tree and is already called on this
    /// expression by the sibling channel: [`Self::narrow_param_bits`], which
    /// answers `#(.P(W8))` and `#(.P(W8 | 1'b0))` correctly TODAY, in this same
    /// parent scope. Its guard chain is reproduced verbatim below, and the term the
    /// earlier attempt was missing is the last one: `param_range`'s width and
    /// `param_meta`'s width must AGREE. A disagreement means one of the two was
    /// inferred, which is exactly the stale-entry case, and it is refused.
    ///
    /// Fail-closed as a whole: one unprovable name declines the entire override, so
    /// this can only ever move a name-bearing tree from the value-inferred 32 to a
    /// declared width, never from a declared width to a guess.
    ///
    /// ⚠️ SECOND CONSUMER: `param_decl_width_opt`'s bare-alias / concatenation /
    /// ternary / operator arms call it (paired with
    /// [`Self::ctx_width_names_are_evident`], exactly as `override_self_meta` does) to
    /// decide whether a DECLARATION's initializer may answer under `declared_only`.
    /// The question is the same one on both lanes — "is every NAME leaf's width a
    /// declared fact?" — so it stays one resolver rather than a predicate copied into
    /// a second place.
    pub(crate) fn declared_override_widths(&self, e: &ast::Expr) -> Option<ConstWidths> {
        fn names<'a>(e: &'a ast::Expr, out: &mut Vec<&'a ast::HierPath>) -> bool {
            use ast::ExprKind as K;
            match &e.kind {
                K::Ident(p) if p.segments.len() == 1 => {
                    out.push(p);
                    true
                }
                K::Paren { inner } | K::Unary { operand: inner, .. } => names(inner, out),
                K::Cast {
                    target: ast::CastTarget::Size(_),
                    expr,
                } => names(expr, out),
                K::Binary { lhs, rhs, .. } => names(lhs, out) && names(rhs, out),
                K::Ternary {
                    cond,
                    then_e,
                    else_e,
                } => names(cond, out) && names(then_e, out) && names(else_e, out),
                K::Concat { parts } => parts.iter().all(|q| names(q, out)),
                // Not a name and not a container this gate descends into — leave it
                // to `ctx_width_names_are_evident`, which is the authority on the
                // accept set. Returning `true` here keeps the two in step: this
                // function's job is only to CERTIFY the names that arm will meet.
                _ => true,
            }
        }
        let mut found = Vec::new();
        if !names(e, &mut found) {
            return None;
        }
        let mut out = ConstWidths::new();
        for path in found {
            // The REAL path node, never a synthesized one: `narrow_param_bits`
            // resolves the name through the live scope chain, and handing it a
            // fabricated span would make this resolver answer about a different
            // occurrence than the one the fold is about to evaluate.
            let (_, w, signed) = self.narrow_param_bits(path)?;
            if w == 0 {
                return None;
            }
            out.insert(path.segments[0].name.clone(), (w, signed));
        }
        Some(out)
    }

    /// The same override expression's VALUE, re-folded AT the type
    /// [`Self::override_self_meta`] just gave it.
    ///
    /// The two halves are inseparable, and the measurement that says so is the same one
    /// §4.5.461 recorded for the parameter-initializer lane: the parent-side fold
    /// (`const_eval_in_scope`) runs at unlimited precision and masks ONCE, at the end,
    /// and truncation commutes with `~ - << & | ^ + *` but NOT with `/ % >> >>>`.
    /// Installing the width alone was measured to move `#(.P(-64'd1))` from
    /// `ffffffff` at 32 bits to `00000000ffffffff` at 64 — a NEW wrong answer, right
    /// width over a value folded for the old one — and `#(.P((8'hFF * 8'h02) >> 4))`
    /// answers 31 through the unlimited lane where both oracles answer 15.
    ///
    /// ⚠️ `const_eval_in_scope` must ALREADY answer. That conjunct is the accept set
    /// row 30 states as "correct a value, never create one": without it an override
    /// that declines the i64 lane today (`32'd3037000500 * 32'd3037000500`,
    /// `64'd3 ** 64'd40`) would start manufacturing a value out of an overflow decline
    /// instead of staying loud.
    pub(crate) fn override_self_value(&self, e: &ast::Expr, meta: (u32, bool)) -> Option<i64> {
        self.const_eval_in_scope(e)?;
        self.eval_const_assign(
            e,
            &std::collections::BTreeMap::new(),
            &ConstWidths::new(),
            0,
            Some(meta),
        )
    }

    /// Is every node this domain descends into within the i64 lane's 64 bits?
    ///
    /// Phrased as "no KNOWN width exceeds 64" rather than "every width is known", so
    /// it can also fence the FILL arm of the same gate without narrowing what that
    /// arm already accepts: a fill answers `Some(0)` and an unmodelled leaf answers
    /// `None`, and neither is the hazard. The hazard is a leaf whose width the
    /// evaluation context would have to CLAMP, because a clamp is a silent value
    /// change ([[widen-a-domain-count-what-it-cannot-carry]]).
    pub(crate) fn const_ctx_within_i64(&self, e: &ast::Expr) -> bool {
        if self
            .const_self_width(e, &ConstWidths::new())
            .is_some_and(|w| w > 64)
        {
            return false;
        }
        // ⚠️ `const_fold_children` has NO `Concat` / `Replicate` arm — those answer their
        // width from §11.4.12 (the sum of the parts) without being asked about their
        // parts, so a walk that descends only through it is walked past by wrapping the
        // hazard in braces. The sum makes the TOP check catch a wide part today, so this
        // descent changes no measured cell; it is here because a GUARD must descend
        // through the arms that answer from a rule, even where the ANSWER need not
        // ([[an-arm-that-answers-without-descending-is-where-an-opaque-leaf-hides]]).
        match &e.kind {
            ast::ExprKind::Concat { parts } => parts.iter().all(|p| self.const_ctx_within_i64(p)),
            ast::ExprKind::Replicate { count, value } => {
                self.const_ctx_within_i64(count)
                    && value.iter().all(|p| self.const_ctx_within_i64(p))
            }
            _ => Self::const_fold_children(e)
                .iter()
                .all(|c| self.const_ctx_within_i64(c)),
        }
    }

    /// An UNTYPED parameter whose initializer CONTAINS an unsized fill: its value at
    /// the initializer's own self-determined width, with the `(width, signed)` an
    /// implicit declaration takes from it (§6.20.2) — ROADMAP §2 🆕 C ⓐ.
    ///
    /// `localparam U = '1 ^ 1'b0;` was 4294967295 with `$bits` 33 where both oracles
    /// give 1 with `$bits` 1: the unlimited i64 walk read the fill through
    /// `parse_int_literal`, whose fill is the lane's hard 32. §5.7.1 sizes a fill to
    /// its context, and an implicit parameter's context is the initializer itself —
    /// so the width-aware walk (`const_int_selfdet`, which now knows a fill has no
    /// width of its own) is the evaluator, and its self width is the recorded one.
    /// Only initializers that contain a fill take this route: every other implicit
    /// initializer keeps the tail it has (the declared-vs-inferred wall, row 14).
    /// `None` when there is no fill, or the walk declines (x/z fill, >64 bits) — the
    /// caller's chain then answers as before.
    pub(crate) fn untyped_fill_init(&self, p: &ast::ParamDecl) -> Option<(i64, (u32, bool))> {
        if !matches!(p.ty, ast::ParamType::Implicit) || p.range.is_some() {
            return None;
        }
        if !ast_any(&p.value, &|x| {
            crate::const_eval::fill_literal_ast(x).is_some()
        }) {
            return None;
        }
        let v = self.const_int_selfdet(&p.value)?;
        let w = self
            .const_self_width(&p.value, &ConstWidths::new())
            .unwrap_or(0)
            .max(1);
        Some((v, (w, self.const_expr_signed(&p.value))))
    }

    /// An untyped parameter whose DEFAULT must not be width-inferred on the
    /// OVERRIDDEN lane — one caller left, and it is no longer about loudness.
    ///
    /// ## What it used to be, and why that expired
    ///
    /// §4.5.407 added it as a delta-limiter: `const_eval_in_scope` had just learned to
    /// fold a reduction, so `~(|4'b1010)`, `(|4'b1010) << 2` and `-(|4'b1010)` would
    /// have landed on a value-inferred tail that sized every operator initializer as
    /// `min_signed_bits(v).max(32)` and printed `4294967294`, `4` and `4294967295`
    /// where both oracles print `0`, `0` and `1`. Loud→silent-wrong is a trade the
    /// accuracy ladder forbids, so the four VALUE sites declined instead. Its doc
    /// named its own expiry: *"when the tail learns to size an initializer at its
    /// self-determined width, this predicate goes with it."*
    ///
    /// §4.5.460 built that tail (the Table 11-21 arm in `param_decl_width_opt`) and
    /// this slice retired the guard from all four value sites. Measured across the
    /// five default binders — module `localparam`, module-body `parameter`, ANSI
    /// header, `package`, `generate` — the three cells now bind `0`, `0`, `1` at one
    /// bit, which is what BOTH oracles print. ⚠️ That deletion is only safe because
    /// the same slice made the initializer VALUE lane width-aware
    /// (`param_init_width_aware_ok`): under the old unlimited-then-coerce fold,
    /// truncation does not commute with `/ % >> >>>`, and deleting the guard alone
    /// was measured to turn 8 further cells loud→silent-wrong.
    ///
    /// ## What survives, and why
    ///
    /// One caller, on the `!default_binds` (OVERRIDDEN) lane, where the §4.5.460 arm
    /// does not run. Without it the tail records `(32, unsigned)` from the DEFAULT's
    /// value and the override's own sign is lost: measured,
    /// `sub #(parameter P = ~(|4'b1010))` overridden with `-1` / `-5` binds `-1` / `-5`
    /// today (both oracles) and `4294967295` / `4294967291` with the guard removed.
    /// `%h` is identical in every one of those rows — only the recorded SIGN moves — so
    /// a `%h`-only readout sees nothing.
    ///
    /// ⚠️ This is an ACCIDENTAL IMMUNITY, not a rule: the unguarded twin `~(!4'b0)` is
    /// silent-wrong on that lane RIGHT NOW, for the same reason, and so is every other
    /// untyped default under an override whose sign differs. The class is ROADMAP §2
    /// row 25 (the override reads the DEFAULT's width and sign), and the honest fix is
    /// there, not here. Until row 25 stands, the guard's job is to keep the cells it
    /// already covers OUT of that class — so it is scoped to that lane explicitly and
    /// the "contains a reduction" conjunct still makes no semantic claim.
    pub(crate) fn param_init_kept_loud(&self, p: &ast::ParamDecl) -> bool {
        if !matches!(p.ty, ast::ParamType::Implicit) || p.range.is_some() {
            return false;
        }
        // An initializer holding a FILL takes `untyped_fill_init` — the width-aware
        // walk at the initializer's own region width, which is exactly the evaluator
        // this guard exists to keep the unlimited walk from standing in for. With a
        // fill's self width at 0, `'1 & (|4'hF)` read as a 1-bit context-determined
        // top and this guard refused a value PRE printed correctly (both oracles
        // `1`/`1`) — review finding, §4.5.409.
        if ast_any(&p.value, &|x| {
            crate::const_eval::fill_literal_ast(x).is_some()
        }) {
            return false;
        }
        let mut top = &p.value;
        while let ast::ExprKind::Paren { inner } = &top.kind {
            top = inner;
        }
        let ctx_top = match &top.kind {
            ast::ExprKind::Unary { op, .. } => {
                matches!(op, ast::UnOp::Plus | ast::UnOp::Minus | ast::UnOp::BitNot)
            }
            ast::ExprKind::Binary { op, .. } => binop_result_is_context_determined(*op),
            _ => false,
        };
        if !ctx_top || !ast_contains_reduction(top) {
            return false;
        }
        // A self width of 32 or more is what the tail records anyway, so the two
        // agree there; anything narrower — or unknown — is the disagreeing shape.
        self.const_self_width(top, &ConstWidths::new())
            .is_none_or(|w| w < 32)
    }
}
