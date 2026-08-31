//! constant-domain BIT / PART / INDEXED-PART select of a PARAMETER.
//!
//! `localparam logic [31:0] W = 32'h34; logic [W[7:0]-1:0] v;` declared a **1-bit**
//! net at exit 0 with no diagnostic, where both oracles declare 52 bits. The value
//! was never in doubt — the RUNTIME lane already prints `W[7:0]` as 52 in all three
//! tools — and neither was the width: what was missing is that
//! [`Elaborator::const_eval_in_scope`] has no arm for a select at all. Its one
//! `BitSelect` arm is a const-ARRAY-ELEMENT lookup (`ROT[i]`), which declines for a
//! scalar param, and there is no `PartSelect` / `IndexedPart` arm before the
//! catch-all. So every consumer of a constant bound — packed range, unpacked
//! dimension, replication count, `$bits`, a loop bound — fell through to
//! `clamp_bound_u32(None)` = 0 = width 1.
//!
//! ⚠️ This is NOT the cast cluster's tree-wide AST self-width pass. A select's
//! self width is `|msb − lsb| + 1` read straight off AST constants; nothing has to
//! be lowered to know it.
//!
//! ## The two hazards, and why each one declines rather than guesses
//!
//! **Direction / declared LSB.** Extracting bits needs the base's DECLARED range,
//! not just its width. `localparam logic [39:8] B = 32'h34; B[15:8]` is 52 in both
//! oracles, but reading internal bits 8..15 of the stored value yields 0 — so an
//! implementation that assumes `[w-1:0]` would replace one silent-wrong (width 1)
//! with a DIFFERENT silent-wrong, which the accuracy ladder forbids outright. The
//! declared range is already recorded in `param_range` and resolved by
//! [`Elaborator::param_sel_range`] — the SAME function the runtime lowering
//! normalizes offsets with, so the two domains cannot drift. (`param_decl_range`
//! grew the ascending arm for this: `[0:31]` has LSB 0 like `[31:0]` and used to
//! record nothing at all.)
//!
//! **Unknown declared width.** `const_self_width`'s Ident arm defaults an
//! unrecorded parameter to 32. Slicing bit 40 of a base the folder believes is 32
//! bits wide invents zeros, so this fold DECLINES on a missing `param_meta` entry
//! instead — §4.5.344's "width-unknown leaf" residue, kept declined.
//!
//! Anything declined lands exactly where it landed before, and where the residue
//! was already loud its MESSAGE is now truthful: `wide_param_name_in` walks the
//! const domain's own traversal instead of the select-blind `collect_bare_idents`,
//! so a >64-bit base says "wider than 64 bits" rather than "undefined name `K`"
//! about a name declared two lines up.

use super::*;

/// The three shapes a select can take, destructured out of EITHER enum.
///
/// ⚠️ `ast::Expr` and `ast::Lvalue` spell the same three selects in two separate
/// enums, and the §11.5.1 span rule must not be written twice — a read and a
/// write that disagree about which bits `x[b -: w]` names is a silent-wrong by
/// construction. Both matchers produce this and call [`Elaborator::select_span`].
pub(crate) enum SelParts<'a> {
    Bit(&'a ast::Expr),
    /// `[msb : lsb]`
    Range(&'a ast::Expr, &'a ast::Expr),
    /// `[offset +: width]` / `[offset -: width]`
    Indexed(&'a ast::Expr, &'a ast::Expr, ast::PartDir),
}

impl Elaborator<'_> {
    /// `(hi, lo, directed)` — the bit span a select names. `directed` is true
    /// only for an explicit `[msb:lsb]`, whose endpoints may be given in either
    /// order and are resolved against the base's declared direction.
    ///
    /// ⚠️ Folded with `eval_const_env_self`, NOT the width-unlimited walk:
    /// §11.5.1 makes a select's index a constant expression and Table 11-21
    /// makes it SELF-DETERMINED. `W[4'd15+4'd1]` names bit 0 — the 4-bit sum
    /// wraps — and the unlimited walk read bit 16 instead.
    ///
    /// The `env`/`envw` pair is what lets a CONST-FUNCTION body fold an index
    /// that mentions a loop variable (`f[i*32 +: 32]`). Module-scope callers
    /// pass empty ones, which is exactly `const_int_selfdet` — that function is
    /// literally this evaluator with an empty environment, so threading the
    /// environment through is a no-op for every pre-existing caller.
    pub(crate) fn select_span(
        &self,
        p: SelParts<'_>,
        env: &std::collections::BTreeMap<String, i64>,
        envw: &crate::const_fn_width::ConstWidths,
        depth: u32,
    ) -> Option<(i64, i64, bool)> {
        match p {
            SelParts::Bit(index) => {
                let i = self.eval_const_env_self(index, env, envw, depth)?;
                Some((i, i, false))
            }
            SelParts::Range(msb, lsb) => Some((
                self.eval_const_env_self(msb, env, envw, depth)?,
                self.eval_const_env_self(lsb, env, envw, depth)?,
                true,
            )),
            // `b +: w` / `b -: w` (§11.5.1): the WIDTH is a positive constant and
            // the span runs numerically up or down from the offset.
            SelParts::Indexed(offset, width, dir) => {
                let o = self.eval_const_env_self(offset, env, envw, depth)?;
                let w = self.eval_const_env_self(width, env, envw, depth)?;
                if w <= 0 {
                    return None;
                }
                Some(match dir {
                    ast::PartDir::PlusColon => (o, o.checked_add(w - 1)?, false),
                    ast::PartDir::MinusColon => (o.checked_sub(w - 1)?, o, false),
                })
            }
        }
    }

    /// [`SelParts`] for an LVALUE select over a bare single-segment name, with
    /// that name. `None` for anything else (a hierarchical or nested base, an
    /// array element) — those are not const-function locals.
    pub(crate) fn lvalue_sel_parts(lv: &ast::Lvalue) -> Option<(&ast::HierPath, SelParts<'_>)> {
        let (base, parts) = match lv {
            ast::Lvalue::BitSelect { base, index, .. } => (base, SelParts::Bit(index)),
            ast::Lvalue::PartSelect { base, msb, lsb, .. } => (base, SelParts::Range(msb, lsb)),
            ast::Lvalue::IndexedPart {
                base,
                offset,
                width,
                dir,
                ..
            } => (base, SelParts::Indexed(offset, width, *dir)),
            _ => return None,
        };
        match &**base {
            ast::Lvalue::Ident(path) if path.segments.len() == 1 => Some((path, parts)),
            _ => None,
        }
    }
}

impl Elaborator<'_> {
    /// A constant BIT / PART / INDEXED-PART select of a parameter, in the i64
    /// const domain. `None` = decline → the caller keeps the behavior it had.
    ///
    /// Mirrors the runtime lowering piece for piece: the same
    /// [`Self::param_sel_range`] declared range, the same substitution-shadow
    /// rule, the same `hi − raw` ascending normalization that
    /// `norm_offset_for_range` performs. The result of a select is UNSIGNED
    /// (§11.5.1) whatever the base's signedness, and its width is the number of
    /// bits selected.
    pub(crate) fn const_param_select(&self, e: &ast::Expr) -> Option<i64> {
        self.const_param_select_env(
            e,
            &std::collections::BTreeMap::new(),
            &crate::const_fn_width::ConstWidths::new(),
            0,
        )
    }

    /// [`Self::const_param_select`] with a const-function ENVIRONMENT, so an index
    /// that mentions a loop variable folds (`M_ADDR_WIDTH[i*32 +: 32]`).
    ///
    /// ⚠️ The empty-environment call above is provably the previous behaviour:
    /// `const_int_selfdet` — what this chain folded indices with before — IS
    /// `eval_const_env_self` with an empty environment, by its own definition.
    pub(crate) fn const_param_select_env(
        &self,
        e: &ast::Expr,
        env: &std::collections::BTreeMap<String, i64>,
        envw: &crate::const_fn_width::ConstWidths,
        depth: u32,
    ) -> Option<i64> {
        let (val, lo_n, hi_n, dlo, dwidth, asc) =
            self.const_select_resolved(e, env, envw, depth)?;
        let n = (hi_n - lo_n + 1) as u32;
        if n > 63 {
            // 64 unsigned magnitude bits do not fit the i64 const domain — the same
            // boundary `const_placement_env` spells out. Declining keeps the loud.
            return None;
        }
        // Normalize BOTH endpoints into internal bit positions and take the lower:
        // ascending flips the order, so the numerically-low DECLARED index is the
        // numerically-HIGH internal one.
        let shift = Self::const_norm_bit(lo_n, i64::from(dlo), dwidth, asc)
            .min(Self::const_norm_bit(hi_n, i64::from(dlo), dwidth, asc));
        let mask = if n == 63 { i64::MAX } else { (1i64 << n) - 1 };
        Some(((val >> shift) as i64) & mask)
    }

    /// One declared-domain index → its internal bit position. The i64 twin of
    /// `norm_offset_for_range`, which the ENGINE side uses for the same job.
    fn const_norm_bit(idx: i64, dlo: i64, dwidth: u32, asc: bool) -> u32 {
        let p = if asc {
            // `[lo:hi]` — the LARGEST source index is internal bit 0.
            dlo + i64::from(dwidth) - 1 - idx
        } else {
            idx - dlo
        };
        p.clamp(0, 63) as u32
    }

    /// A select fully resolved against its base:
    /// `(base value, numeric low index, numeric high index, declared lo, declared
    /// width, ascending)`. The one place the direction rule is written, so the fold
    /// and the self-width answer cannot disagree about which bits are named.
    #[allow(clippy::type_complexity)] // one tuple, one caller pair — a struct would not earn its name
    fn const_select_resolved(
        &self,
        e: &ast::Expr,
        env: &std::collections::BTreeMap<String, i64>,
        envw: &crate::const_fn_width::ConstWidths,
        depth: u32,
    ) -> Option<(u64, i64, i64, u32, u32, bool)> {
        let (base, a, b, directed) = self.const_select_bounds(e, env, envw, depth)?;
        let (val, dlo, dwidth, asc) = self.const_select_base(base)?;
        // ⚠️ `[m:l]` is written LEFT:RIGHT in the base's own declared direction, so
        // "which endpoint is numerically larger" is decided by the BASE, not by the
        // select. A descending base wants `m >= l` and an ascending one `m <= l`;
        // the other way round is the out-of-order select vita already rejects
        // loudly at the runtime lowering, and it must not fold here either.
        let (lo_n, hi_n) = if directed {
            // ⚠️ `a == b` is legal in BOTH directions — `W[5:5]` is the ordinary way
            // to write a one-bit slice, and a parameterised `[K:K]` degenerates to it.
            // An earlier `(asc, a <= b)` table put the descending equal-endpoint case
            // in the reject arm, so `W[5:5]` declined while `A[26:26]` folded.
            match asc {
                true if a <= b => (a, b),
                false if a >= b => (b, a),
                _ => return None,
            }
        } else {
            // A bit-select and an indexed part-select name a numeric span outright
            // (`b +: w` is `b ..= b+w-1` whatever the direction), so there is no
            // order to check.
            (a.min(b), a.max(b))
        };
        // A select reaching outside the declared range reads `x` for the outside
        // bits (§11.5.1) — an UNKNOWN this integer domain cannot represent.
        // Decline; the runtime lane is the one that models x.
        let (dlo_i, dhi_i) = (i64::from(dlo), i64::from(dlo) + i64::from(dwidth) - 1);
        if lo_n < dlo_i || hi_n > dhi_i {
            return None;
        }
        Some((val, lo_n, hi_n, dlo, dwidth, asc))
    }

    /// The two endpoints of a select. The `bool` is `true` when the pair is written
    /// LEFT:RIGHT and its order must be validated against the base's direction.
    /// `None` for a non-select or an unfoldable bound.
    ///
    /// ⚠️ The bounds fold through `const_int_selfdet`, NOT the width-unlimited
    /// `const_eval_in_scope`: §11.5.1 makes a select's index a constant expression
    /// and Table 11-21 makes it a SELF-DETERMINED position. `W[4'd15+4'd1]` names
    /// bit 0 — the 4-bit sum wraps — and the unlimited walk read bit 16 instead, so
    /// a bound built on it declared 10 bits where both oracles declare 9. vita's own
    /// RUNTIME lane already answered 0 for the same text.
    fn const_select_bounds<'a>(
        &self,
        e: &'a ast::Expr,
        env: &std::collections::BTreeMap<String, i64>,
        envw: &crate::const_fn_width::ConstWidths,
        depth: u32,
    ) -> Option<(&'a ast::Expr, i64, i64, bool)> {
        let (base, parts) = match &e.kind {
            ast::ExprKind::BitSelect { base, index } => (base, SelParts::Bit(index)),
            ast::ExprKind::PartSelect { base, msb, lsb } => (base, SelParts::Range(msb, lsb)),
            ast::ExprKind::IndexedPart {
                base,
                offset,
                width,
                dir,
            } => (base, SelParts::Indexed(offset, width, *dir)),
            _ => return None,
        };
        let (a, b, directed) = self.select_span(parts, env, envw, depth)?;
        Some((base, a, b, directed))
    }

    /// The base of a constant select: `(value, declared lo, declared width,
    /// ascending)`. Restricted to a BARE single-segment parameter whose declared
    /// width is RECORDED — see the module doc for why each restriction is a
    /// decline and not a guess.
    fn const_select_base(&self, base: &ast::Expr) -> Option<(u64, u32, u32, bool)> {
        // `pkg::W` — the same declaration, in the scope that exists to share it. The
        // value comes from `pkg_consts` and the range from `pkg_const_range`, which
        // is filled by the SAME `param_decl_range_opt` the module twin below reads
        // back through `param_sel_range`, so the provenance rule is one rule.
        //
        // ⚠️ A package ARRAY parameter is refused for the reason the module arm
        // refuses one: `pkg::ROT[2]` is an ELEMENT read, and `const_array_vals_of_base`
        // is what answers it. A package STRING or REAL parameter is not in
        // `pkg_consts` at all (they live in their own side maps), so it declines here
        // and stays loud, as it does at module scope.
        if let ast::ExprKind::PkgScoped { pkg, name } = &base.kind {
            if self.const_array_vals_of_base(base).is_some() {
                return None;
            }
            let v = *self.pkg_consts.get(&pkg.name)?.get(&name.name)?;
            let (dlo, dwidth, asc) = self.param_sel_range(base)?;
            return Self::select_base_at_declared(v, dlo, dwidth, asc);
        }
        let ast::ExprKind::Ident(path) = &base.kind else {
            return None; // a hierarchical / element base is not this arm
        };
        let [seg] = path.segments.as_slice() else {
            return None;
        };
        // An inline-function formal or a task output formal SHADOWS the param: the
        // VALUE would resolve to the substitution, so folding the param here would
        // answer for a different object.
        //
        // ⚠️ REDUNDANT, AND KEPT DELIBERATELY. The mutation battery proved this guard
        // equivalent to its own absence: `param_sel_range` below opens with the
        // identical test on the identical name and its `None` propagates through the
        // `?`. It stays because the two are one rule and the reader should see it at
        // the point of use — not because it is load-bearing. If `param_sel_range`
        // ever stops making that check, this is what keeps the answer right.
        if self.subst_lookup(&seg.name).is_some() || self.out_subst_lookup(&seg.name).is_some() {
            return None;
        }
        // ⚠️⚠️ ARM ORDER, MIRRORED. `const_eval_in_scope`'s `BitSelect` arm tries the
        // const-ARRAY-element lookup FIRST and only then this fold, so this fold must
        // refuse every base that lookup claims — otherwise the value arm and the
        // width helper resolve the SAME base to two different objects.
        //
        // They did. With an inner scalar shadowing an outer const array —
        //   `localparam int ROT [0:3] = '{10,20,30,40};`
        //   `generate if (1) begin : g localparam int ROT = 99; logic [ROT[1]:0] v;`
        // — `lookup_scoped` finds the inner 99 (width 1), while the value arm's
        // array-first branch answers the OUTER element 20; masking 20 to the 1-bit
        // self width gave 0, so `$bits(v)` went 21 → 1 where PRE said 21. Both are
        // wrong (verilator, the sole oracle here — iverilog rejects unpacked array
        // parameters outright — says 2), but trading one silent wrong for a different
        // one is the move the ladder forbids, so this restores PRE exactly.
        //
        // ⚠️ The ROOT is older and stays recorded, not fixed here: the first branch of
        // `const_array_vals_of_base` returns on a `walk_scopes_key` hit WITHOUT the
        // inner-wins shadow check its own second branch performs, so it reaches past
        // the shadow. Closing that lands the oracle's 2 — and the MODULE-scope
        // spelling of the same shadow is already correct (and improved by this slice,
        // see `round4_report_gaps`), which is exactly the "one rule, one spelling
        // short" shape. Fixing it is a GAP-G slice, not this one.
        if self.const_array_vals_of_base(base).is_some() {
            return None;
        }
        let v = self.lookup_scoped(&seg.name)?;
        // ⚠️⚠️ `param_sel_range` IS THE PROVENANCE GATE, and it is the only source of
        // a width here. Reading `param_meta` instead — or falling back to it — is
        // exactly the measured regression: `param_decl_width`'s last arm sizes an
        // untyped EXPRESSION initializer from its folded VALUE, so
        // `localparam W = ~8'hCB;` records 32 for an 8-bit value and `W[15:8]` then
        // extracts bits that do not exist (`logic [(W[15:8])+8-1:0] v;` declared a
        // 263-bit net where iverilog declares 1, and PRE agreed with iverilog).
        // `param_decl_range` answers only for a DECLARED range or a TYPE/LITERAL
        // width, so a value-inferred one has no entry and this declines.
        let (dlo, dwidth, asc) = self.param_sel_range(base)?;
        Self::select_base_at_declared(v, dlo, dwidth, asc)
    }

    /// A resolved base value narrowed to its DECLARED width. One spelling for the
    /// module-scope and the package-scope arm, so the two cannot disagree about
    /// which bits the declaration owns.
    ///
    /// A signed negative param carries its sign bits above the declared width in the
    /// i64 container, and those are not part of the value: `localparam signed [31:0]
    /// W = -32'sd52; W[15:8]` is 255 in both oracles, which is the masked byte, not
    /// a sign extension.
    fn select_base_at_declared(
        v: i64,
        dlo: u32,
        dwidth: u32,
        asc: bool,
    ) -> Option<(u64, u32, u32, bool)> {
        if dwidth == 0 || dwidth > 64 {
            return None;
        }
        let keep = if dwidth >= 64 {
            u64::MAX
        } else {
            (1u64 << dwidth) - 1
        };
        Some((v as u64 & keep, dlo, dwidth, asc))
    }

    /// Fold a DECLARED RANGE bound (`logic [<e>:0]`).
    ///
    /// ⚠️ This is the guard that keeps the new select fold from trading one
    /// silent-wrong for another. A select is a NARROW leaf — `W[3:2]` is 2 bits — so
    /// arithmetic above it wraps at a width the width-UNLIMITED module fold does not
    /// model: `logic [W[3:2]-4'd3:0] q;` is 15 bits in both oracles (1−3 wraps to 14
    /// at 4 bits) and the unlimited fold answers −2 ⇒ 3. Before this slice that cell
    /// declined and clamped to width 1; landing it on 3 instead would be a silent
    /// wrong swapped for a different silent wrong, which the ladder forbids.
    ///
    /// ⚠️⚠️ IT LIVES HERE, ON THE CONSUMER, AND NOT IN `const_eval_in_scope`. A first
    /// draft put the redirect inside that evaluator's Binary arm, and the adversarial
    /// review measured three separate costs, because that function is shared by
    /// consumers with OPPOSITE width rules:
    ///   * a PARAMETER VALUE is an assignment, not a self-determined position (§11.6),
    ///     so `localparam int Q = W[7:0] + 8'd240;` folded 36 where both oracles fold
    ///     292 — and it had been honest-loud before, i.e. loud→silent-wrong;
    ///   * the guard reached only ONE arm, so `~W[3:0]` still took the unlimited walk
    ///     and traded silent 1 for a different silent 6 (both oracles 12);
    ///   * it ran on every Binary node and walked that node's whole subtree, making a
    ///     select-FREE 8000-term constant expression elaborate 121× slower — Θ(n²) on
    ///     expressions with no select in them at all.
    ///
    /// A declared range bound IS self-determined by definition (§11.6.1), so applying
    /// the rule here is both correct and O(1) per bound. Every other constant-bound
    /// consumer already goes through the tiered `const_bound_u32` funnel, whose
    /// `ast_const_leaves_min32` fails closed on a select for the same reason.
    ///
    /// A bound with no select takes the identical call it always did.
    pub(crate) fn const_range_bound_fold(&self, e: &ast::Expr) -> Option<i64> {
        if self.ast_has_param_select(e) {
            return self.const_int_selfdet(e);
        }
        self.const_eval_in_scope(e)
    }

    /// Does `e` contain a foldable parameter SELECT anywhere? See
    /// [`Self::const_range_bound_fold`], its only caller.
    fn ast_has_param_select(&self, e: &ast::Expr) -> bool {
        if matches!(
            &e.kind,
            ast::ExprKind::BitSelect { .. }
                | ast::ExprKind::PartSelect { .. }
                | ast::ExprKind::IndexedPart { .. }
        ) && self.const_select_self_width(e).is_some()
        {
            return true;
        }
        Self::const_fold_children(e)
            .iter()
            .any(|c| self.ast_has_param_select(c))
    }

    /// The SELF width of a constant select (§11.5.1): the number of bits it names,
    /// independent of the base's width. Kept beside the fold so the two cannot
    /// disagree about which bits a select denotes.
    ///
    /// ⚠️ Gated on the SAME base resolution as the fold, and that gate is
    /// load-bearing: `ROT[i]` on a const ARRAY is also a `BitSelect`, and it names
    /// a whole ELEMENT, not one bit. Answering 1 for it would mask a 32-bit element
    /// value down to its low bit in the self-determined walk. A const-array element
    /// has no recorded width here — it stays the §4.5.344 width-unknown leaf, which
    /// degrades to the unlimited domain exactly as before.
    pub(crate) fn const_select_self_width(&self, e: &ast::Expr) -> Option<u32> {
        let (_, lo_n, hi_n, ..) = self.const_select_resolved(
            e,
            &std::collections::BTreeMap::new(),
            &crate::const_fn_width::ConstWidths::new(),
            0,
        )?;
        u32::try_from(hi_n - lo_n + 1).ok()
    }
}
