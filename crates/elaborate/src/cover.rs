//! covergroups — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

/// A hierarchical ELEMENT / bit-select WRITE target (`tb.dut.mem[i] = …`,
/// `m.l.v[3] <= …`) whose net does not exist when the lvalue is lowered. Unlike
/// [`DeferredHierWrite`] (which only swaps the chunk's net), resolution must
/// REBUILD the whole `LvalChunk` (array element word / packed bit-slice / vector
/// bit) from the resolved net's shape — that shape is unknown at lowering time.
/// Every index is LOWERED AT LOWERING TIME (full param/genvar/formal context)
/// into `idx_eids`; `resolve_deferred_hier_sel_write` only builds the flat-word /
/// offset arithmetic around those eids and patches the placeholder chunk
/// (sentinel net `HIER_SEL_WRITE_SENTINEL_BASE + index`). Out-of-band
/// (golden-free). The write twin of [`DeferredHierSelect`].
/// N5: one coverpoint's runtime tracker. The hit-bitmap is a fresh 64-bit reg; each
/// `sample()` ORs in `1 << bin(value)`, and `get_coverage()` reports
/// `$countones(bitmap) * 100 / num_bins`. Pure IR-0 (ordinary nets + synthesized ops).
/// One resolved bin of an explicit-bin coverpoint (slice A; `iff` slice B).
#[derive(Clone)]
pub(crate) struct ResolvedBin {
    pub(crate) kind: ast::BinKind,
    /// Closed integer ranges (single value ⇒ `lo==hi`); `$` already clamped to the
    /// coverpoint domain.
    pub(crate) ranges: Vec<(i64, i64)>,
    /// `Some(i)` ⇒ a COUNTING bin occupying bitmap bit `i` (regular, non-default,
    /// non-empty). `None` ⇒ ignore/illegal (feeds gating + `$error`, never counted).
    pub(crate) bit: Option<u32>,
    /// Per-bin `iff (G)` guard (slice B): the bin's bit is set only when `G` is true
    /// at sample time. `None` ⇒ unguarded. (Only on regular/counting bins; a guard on
    /// ignore/illegal is loud-rejected, as the precedence subtraction is static.)
    pub(crate) iff: Option<ast::Expr>,
    /// Per-bin saturating hit COUNTER reg (slice D, `option.at_least > 1`): the
    /// covered-bit is set only once this counter reaches `at_least`. `None` ⇒
    /// `at_least == 1` (the bit is set on the first match — byte-identical path).
    pub(crate) counter: Option<String>,
}

#[derive(Clone)]
pub(crate) struct CoverpointTracker {
    pub(crate) bitmap: String,
    pub(crate) expr: ast::Expr,
    /// Resolved explicit bins (may be empty even when `has_explicit` — e.g. every
    /// regular bin fully subtracted by ignore/illegal, a reversed/empty range).
    pub(crate) bins: Vec<ResolvedBin>,
    pub(crate) num_bins: u32,
    /// `true` iff the SOURCE coverpoint had a `{ bins… }` body. Distinguishes
    /// "no body ⇒ auto-bins (legacy 1<<(v&63))" from "explicit body that resolved
    /// to ZERO counting bins ⇒ vacuous (must NOT fall back to auto-bins)". Sampling
    /// dispatch keys on THIS, never on `bins.is_empty()`.
    pub(crate) has_explicit: bool,
    /// Coverpoint-level `iff (G)` guard (slice B): the whole sample (every bin update
    /// and the illegal `$error`) is gated on `G`. `None` ⇒ unguarded.
    pub(crate) cp_iff: Option<ast::Expr>,
    /// The coverpoint's name for `cross` resolution (slice C): the explicit label,
    /// else the implicit single-ident expr name, else `None`.
    pub(crate) name: Option<String>,
    /// `option.at_least` (slice D): hits required before a bin counts as covered
    /// (default 1). >1 uses per-bin counters (see `ResolvedBin.counter`).
    pub(crate) at_least: u32,
    /// `option.weight` (slice D): this coverpoint's weight in the covergroup average
    /// (default 1).
    pub(crate) weight: u32,
}

/// One cross constituent: `(sampled expr, [effective ranges per counting bin])`.
pub(crate) type CrossPoint = (ast::Expr, Vec<Vec<(i64, i64)>>);

/// One cross of named coverpoints (N5 slice C): a product hit-bitmap whose bit
/// `idx` (mixed-radix over the constituents' counting bins) is set when EVERY
/// constituent's bin at that index matches the SAME sample.
#[derive(Clone)]
pub(crate) struct CrossTracker {
    pub(crate) bitmap: String,
    pub(crate) num_bins: u32,
    pub(crate) points: Vec<CrossPoint>,
}

/// A `cover property(@(clk) [disable iff(e)] seq)` collected during lowering and
/// materialized after the module loop as a clocked match counter + an end-of-sim
/// `$display` of the hit count (SVA-REST). Out-of-band (golden-free).
pub(crate) struct PendingCover {
    pub(crate) clock: ast::Sensitivity,
    pub(crate) disable_iff: Option<ast::Expr>,
    pub(crate) seq: ast::Sequence,
    pub(crate) span: ast::Span,
}

impl Elaborator<'_> {
    /// A fresh ZERO-initialized reg (N5 `option.at_least` counter, slice D). Unlike
    /// `fresh_sva_reg` (X-init), this starts at a KNOWN 0 so `ctr < N` / `ctr + 1`
    /// behave from the first sample (an X-init counter would compare X forever).
    pub(crate) fn fresh_cover_counter(&mut self, width: u32) -> String {
        let w = width.max(1);
        let name = format!("__sva_covctr_{}", self.nets.len());
        let nwords = w.div_ceil(64).max(1) as usize;
        let nv = ir::NetVar {
            kind: ir::NetKind::Reg,
            width: w,
            msb: w.saturating_sub(1),
            lsb: 0,
            signed: false,
            array_len: 1,
            dir: ir::PortDir::Internal,
            init: ir::BitPacked {
                val: vec![0; nwords],
                unk: vec![0; nwords],
            },
        };
        self.add_net(&name, nv);
        name
    }

    /// Fold a coverage `option.*` value (a positive constant); non-const or `< 1` is
    /// loud (follow-on) and yields `default`. Slice D.
    pub(crate) fn fold_cover_opt(&mut self, e: Option<&ast::Expr>, default: u32) -> u32 {
        match e {
            None => default,
            Some(e) => match self.const_eval_in_scope(e) {
                Some(v) if v >= 1 => v as u32,
                Some(_) => {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "coverage option value must be a positive constant",
                    );
                    default
                }
                None => {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "non-constant coverage option value (follow-on)",
                    );
                    default
                }
            },
        }
    }

    /// Register a covergroup instance: allocate one 64-bit hit-bitmap reg per
    /// coverpoint and record the trackers under the instance's FQ name.
    pub(crate) fn register_cover_instance(&mut self, ci: &ast::CoverInstance) {
        let Some(cg) = self.cover_types.get(&ci.cg_type.name).cloned() else {
            self.error(
                MsgCode::ElabUnresolvedName,
                &format!(
                    "unknown covergroup type `{}` (instance `{}`)",
                    ci.cg_type.name, ci.name.name
                ),
            );
            return;
        };
        let mut trackers = Vec::new();
        for cp in &cg.points {
            // Name for `cross` resolution: explicit label, else implicit single-ident.
            let name = cp
                .label
                .as_ref()
                .map(|l| l.name.clone())
                .or_else(|| match &cp.expr.kind {
                    ast::ExprKind::Ident(p) if p.segments.len() == 1 => {
                        Some(p.segments[0].name.clone())
                    }
                    _ => None,
                });
            // Slice D options: effective at_least (cp override, else covergroup
            // default, else 1) and weight (cp, else 1).
            let at_least = self.fold_cover_opt(cp.at_least.as_ref().or(cg.at_least.as_ref()), 1);
            let weight = self.fold_cover_opt(cp.weight.as_ref(), 1);
            if cp.bins.is_empty() {
                // Auto-bin fallback — the byte-identical legacy path (allocation
                // order preserved: num_bins read before the bitmap reg alloc).
                if at_least > 1 {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "`option.at_least > 1` on an auto-bin coverpoint (declare \
                         explicit bins — follow-on)",
                    );
                }
                let num_bins = self.coverpoint_num_bins(&cp.expr);
                let bitmap = self.fresh_sva_reg(64, "cov");
                trackers.push(CoverpointTracker {
                    bitmap,
                    expr: cp.expr.clone(),
                    bins: Vec::new(),
                    num_bins,
                    has_explicit: false,
                    cp_iff: cp.iff.clone(),
                    name,
                    at_least: 1, // auto path always sets on first match
                    weight,
                });
            } else {
                let (mut bins, num_bins) = self.resolve_explicit_bins(cp);
                // at_least > 1: a per-bin saturating counter gates the covered-bit.
                if at_least > 1 {
                    for rb in bins.iter_mut().filter(|b| b.bit.is_some()) {
                        rb.counter = Some(self.fresh_cover_counter(32));
                    }
                }
                let bitmap = self.fresh_sva_reg(64, "cov");
                trackers.push(CoverpointTracker {
                    bitmap,
                    expr: cp.expr.clone(),
                    bins,
                    num_bins,
                    has_explicit: true,
                    cp_iff: cp.iff.clone(),
                    name,
                    at_least,
                    weight,
                });
            }
        }
        // Slice C: resolve crosses (cartesian product of constituents' counting bins).
        let crosses = self.resolve_crosses(&cg.crosses, &trackers);
        let key = self.fq(&ci.name.name);
        // OBS-1b: record this instance's coverage manifest for the end-of-run
        // coverage.json. Resolve each hit-bitmap net id HERE (the fresh `__sva_cov_*`
        // names are visible in this scope) and mirror `synth_cover_get`'s item set:
        // coverpoints (skipped from the average when num_bins==0) + crosses (weight 1).
        let cov_items: Vec<CovItem> = trackers
            .iter()
            .enumerate()
            .filter_map(|(i, t)| {
                self.lookup_net_scoped(&t.bitmap).map(|net| CovItem {
                    name: t.name.clone().unwrap_or_else(|| format!("cp_{i}")),
                    is_cross: false,
                    bitmap_net: net,
                    num_bins: t.num_bins,
                    weight: t.weight,
                })
            })
            .chain(crosses.iter().enumerate().filter_map(|(i, cx)| {
                self.lookup_net_scoped(&cx.bitmap).map(|net| CovItem {
                    name: format!("cross_{i}"),
                    is_cross: true,
                    bitmap_net: net,
                    num_bins: cx.num_bins,
                    weight: 1,
                })
            }))
            .collect();
        self.cover_insts.insert(key.clone(), trackers);
        if !crosses.is_empty() {
            self.cross_insts.insert(key.clone(), crosses);
        }
        if !cov_items.is_empty() {
            self.coverage_manifest.push(CovgInstMeta {
                inst: key,
                items: cov_items,
            });
        }
        // Slice F: a clocked covergroup (`covergroup cg @(ev);`) AUTO-samples each
        // instance on its event — synthesize `always @(ev) inst.sample();` (the call
        // dispatches through the normal inline-task path back to `synth_cover_sample`).
        // Explicit `inst.sample()` still works and coexists (idempotent bitmap OR).
        // Runs in the cover-instance pre-sweep, so module scope is active for `ev`.
        if let Some(clock) = &cg.clock {
            let sp = ci.span;
            let sample_call = ast::Stmt::UserTaskCall {
                name: ast::HierPath {
                    segments: vec![
                        ast::Ident {
                            name: ci.name.name.clone(),
                            span: sp,
                        },
                        ast::Ident {
                            name: "sample".to_string(),
                            span: sp,
                        },
                    ],
                    span: sp,
                },
                args: Vec::new(),
                span: sp,
            };
            let pb = ast::ProceduralBlock {
                kind: ast::ProcKind::Always,
                sensitivity: Some(clock.clone()),
                body: Box::new(sample_call),
                span: sp,
            };
            let proc = self.lower_synth_proc(&pb, "covergroup");
            self.push_process(proc);
        }
    }

    /// Trackers for a covergroup instance named at a call site (current scope).
    pub(crate) fn cover_inst(&self, name: &str) -> Option<Vec<CoverpointTracker>> {
        self.cover_insts.get(&self.fq(name)).cloned()
    }

    /// Cross trackers for a covergroup instance named at a call site.
    pub(crate) fn cross_inst(&self, name: &str) -> Option<Vec<CrossTracker>> {
        self.cross_insts.get(&self.fq(name)).cloned()
    }

    /// Sample a cross: for each product index (mixed-radix over the constituents'
    /// counting bins), `if (match_0 && match_1 && …) crossmap[idx] = 1'b1;`.
    pub(crate) fn synth_cross_sample(&mut self, b: &mut ProcessBuilder, cx: &CrossTracker) {
        let sp = cx.points[0].0.span;
        for idx in 0..cx.num_bins as usize {
            let mut rem = idx;
            let mut cond: Option<ast::Expr> = None;
            for (expr, bins) in cx.points.iter().rev() {
                let bi = rem % bins.len();
                rem /= bins.len();
                let m = cov_bin_match(expr, &bins[bi], sp);
                cond = Some(match cond {
                    None => m,
                    Some(c) => sva_binary(ast::BinOp::LogAnd, m, c, sp),
                });
            }
            let set = ast::Stmt::If {
                cond: cond.unwrap(),
                then_s: Box::new(ast::Stmt::Blocking {
                    lhs: ast::Lvalue::BitSelect {
                        base: Box::new(ast::Lvalue::Ident(ast::HierPath {
                            segments: vec![ast::Ident {
                                name: cx.bitmap.clone(),
                                span: sp,
                            }],
                            span: sp,
                        })),
                        index: Box::new(cov_int_lit(idx as i64, sp)),
                        span: sp,
                    },
                    delay: None,
                    event: None,
                    rhs: sva_one(sp),
                    span: sp,
                }),
                else_s: None,
                span: sp,
            };
            self.lower_stmt(b, &set);
        }
    }

    /// `c.sample();` — record this sample into each coverpoint's hit bitmap.
    /// Auto-bin coverpoints OR `1 << (value & 63)` (the byte-identical legacy path);
    /// explicit-bin coverpoints set the matching counting bin's bit (with
    /// ignore/illegal precedence gating) and fire `$error` on an illegal hit.
    pub(crate) fn synth_cover_sample(&mut self, b: &mut ProcessBuilder, inst: &str) {
        let Some(trackers) = self.cover_inst(inst) else {
            self.error(
                MsgCode::ElabUnresolvedName,
                &format!("`{inst}.sample()` on an unknown covergroup instance"),
            );
            return;
        };
        for t in &trackers {
            if t.has_explicit {
                // Explicit-bin coverpoint — even if it resolved to ZERO counting bins
                // (all values ignored/illegal, reversed/empty range), it must take the
                // explicit path (sets only counting bits + illegal `$error`), NEVER the
                // auto-bin fallback below.
                self.synth_cover_sample_explicit(b, t);
                continue;
            }
            let Some(bnet) = self.lookup_net_scoped(&t.bitmap) else {
                continue;
            };
            // Auto-bins with a coverpoint `iff` (slice B): gate the `1<<(v&63)` sample
            // on the guard. Built as AST (no byte-identity constraint — guarded sampling
            // is new behavior); the UNGUARDED path below stays the byte-identical legacy.
            if let Some(g) = &t.cp_iff {
                let sp = t.expr.span;
                let gated = ast::Stmt::If {
                    cond: g.clone(),
                    then_s: Box::new(cov_auto_sample_stmt(
                        &t.bitmap,
                        &t.expr,
                        t.num_bins.saturating_sub(1),
                        sp,
                    )),
                    else_s: None,
                    span: sp,
                };
                self.lower_stmt(b, &gated);
                continue;
            }
            let val = self.lower_expr(&t.expr);
            // Bin index = value & (num_bins-1). num_bins is a power of two ≤ 64, so
            // this both (a) truncates the context-widened value to the coverpoint's
            // self-determined low bits (the auto-bin domain) and (b) keeps the index
            // < num_bins so $countones(bitmap) can never exceed num_bins (no >100%).
            // For a bare W-bit net (value < 2^W = num_bins) this equals the legacy
            // `& 63`, so those designs stay byte-identical.
            let mask = self.const_u32_expr(t.num_bins.saturating_sub(1), 32);
            let masked = self.push_expr(ir::Expr::Binary {
                op: ir::BinOp::BitAnd,
                lhs: val,
                rhs: mask,
            });
            let one = self.const_u32_expr(1, 64);
            let shifted = self.push_expr(ir::Expr::Binary {
                op: ir::BinOp::Shl,
                lhs: one,
                rhs: masked,
            });
            let cur = self.push_expr(ir::Expr::Signal {
                net: bnet,
                word: None,
            });
            let newbm = self.push_expr(ir::Expr::Binary {
                op: ir::BinOp::BitOr,
                lhs: cur,
                rhs: shifted,
            });
            let sid = self.push_stmt(ir::Stmt::BlockingAssign {
                lhs: whole_net_lvalue(bnet),
                rhs: newbm,
            });
            b.push_stmt_id(sid);
        }
        // Slice C: sample any crosses of this instance (product-bin matches).
        if let Some(crosses) = self.cross_inst(inst) {
            for cx in &crosses {
                self.synth_cross_sample(b, cx);
            }
        }
    }

    /// Explicit-bin sample: for each COUNTING bin, `if (match [&& bin_iff]) bitmap[bit]
    /// = 1'b1;` (its `ranges` are the EFFECTIVE set — ignore/illegal already subtracted,
    /// so no precedence gating is needed); then, if any illegal bins exist,
    /// `if (any_illegal) $error(...)`. The whole body is wrapped in `if (cp_iff)` when
    /// the coverpoint has a guard (slice B). Built as AST and lowered via `lower_stmt`.
    /// `t` is borrowed from the CLONED tracker list, so `&mut self.lower_stmt` is free.
    pub(crate) fn synth_cover_sample_explicit(
        &mut self,
        b: &mut ProcessBuilder,
        t: &CoverpointTracker,
    ) {
        let sp = t.expr.span;
        let any_illegal = cov_match_any(&t.bins, &t.expr, ast::BinKind::Illegal, sp);
        let mut stmts: Vec<ast::Stmt> = Vec::new();
        for rb in &t.bins {
            let Some(bit) = rb.bit else {
                continue; // only counting (regular) bins set a bit
            };
            let mut cond = cov_bin_match(&t.expr, &rb.ranges, sp);
            if let Some(g) = &rb.iff {
                cond = sva_binary(ast::BinOp::LogAnd, cond, g.clone(), sp);
            }
            // at_least == 1 (counter None): set the bit on first match. at_least > 1:
            // a saturating counter gates the covered-bit (set only at the N-th hit).
            let then_s = match &rb.counter {
                None => cov_set_bit_stmt(&t.bitmap, bit, sp),
                Some(ctr) => cov_counter_then(&t.bitmap, bit, ctr, t.at_least, sp),
            };
            stmts.push(ast::Stmt::If {
                cond,
                then_s: Box::new(then_s),
                else_s: None,
                span: sp,
            });
        }
        if let Some(il) = any_illegal {
            stmts.push(ast::Stmt::If {
                cond: il,
                then_s: Box::new(ast::Stmt::SysTaskCall {
                    name: ast::Ident {
                        name: "$error".to_string(),
                        span: sp,
                    },
                    args: vec![ast::Expr {
                        kind: ast::ExprKind::StrLit {
                            raw: "\"illegal coverage bin hit\"".to_string(),
                        },
                        span: sp,
                    }],
                    span: sp,
                }),
                else_s: None,
                span: sp,
            });
        }
        if stmts.is_empty() {
            return;
        }
        // Coverpoint-level `iff`: gate the whole sample on the guard.
        if let Some(g) = &t.cp_iff {
            let wrapped = ast::Stmt::If {
                cond: g.clone(),
                then_s: Box::new(sva_block_or_single(stmts, sp)),
                else_s: None,
                span: sp,
            };
            self.lower_stmt(b, &wrapped);
        } else {
            for s in &stmts {
                self.lower_stmt(b, s);
            }
        }
    }

    /// `c.get_coverage()` — the REAL (f64) weighted average of each coverpoint's
    /// coverage (§19.5): `cp_cov_i = $countones(bitmap_i) * 100.0 / num_bins_i`, then
    /// `avg = sum(cp_cov_i) / N` over the N coverpoints with ≥1 counting bin (default
    /// equal weights; coverpoints with 0 counting bins are excluded — not a coverage
    /// target). Returns `0.0` if there are no counting coverpoints. (Per-coverpoint
    /// average, NOT the old pooled `sum(covered)/sum(total)` — they differ for
    /// heterogeneous coverpoints; `option.weight` is a follow-on.)
    pub(crate) fn synth_cover_get(&mut self, inst: &str) -> u32 {
        let Some(trackers) = self.cover_inst(inst) else {
            self.error(
                MsgCode::ElabUnresolvedName,
                &format!("`{inst}.get_coverage()` on an unknown covergroup instance"),
            );
            return self.placeholder_expr();
        };
        let hundred = self.real_const_expr("100.0");
        let mut sum: Option<u32> = None;
        let mut total_weight: u32 = 0;
        for t in &trackers {
            if t.num_bins == 0 {
                continue; // not a coverage target — excluded from the average
            }
            let bnet = self.lookup_net_scoped(&t.bitmap).unwrap_or(POISON_NET);
            let bm = self.push_expr(ir::Expr::Signal {
                net: bnet,
                word: None,
            });
            let ones = self.push_expr(ir::Expr::SysFunc {
                which: ir::SysFuncId::CountOnes,
                args: vec![bm],
            });
            // ones * 100.0 (int promoted to real) / num_bins → real per-cp coverage.
            let ones100 = self.push_expr(ir::Expr::Binary {
                op: ir::BinOp::Mul,
                lhs: ones,
                rhs: hundred,
            });
            let nb = self.const_u32_expr(t.num_bins, 32);
            let cp_cov = self.push_expr(ir::Expr::Binary {
                op: ir::BinOp::Div,
                lhs: ones100,
                rhs: nb,
            });
            // weight == 1 ⇒ the bare term (byte-identical to the unweighted average);
            // weight N ⇒ N * cp_cov in the numerator, N added to the weight total.
            let term = if t.weight == 1 {
                cp_cov
            } else {
                let w = self.const_u32_expr(t.weight, 32);
                self.push_expr(ir::Expr::Binary {
                    op: ir::BinOp::Mul,
                    lhs: w,
                    rhs: cp_cov,
                })
            };
            sum = Some(match sum {
                None => term,
                Some(acc) => self.push_expr(ir::Expr::Binary {
                    op: ir::BinOp::Add,
                    lhs: acc,
                    rhs: term,
                }),
            });
            total_weight += t.weight;
        }
        // Slice C: crosses join the weighted average as additional terms
        // (cross coverage = $countones(crossmap) * 100.0 / product_bins).
        if let Some(crosses) = self.cross_inst(inst) {
            for cx in &crosses {
                let bnet = self.lookup_net_scoped(&cx.bitmap).unwrap_or(POISON_NET);
                let bm = self.push_expr(ir::Expr::Signal {
                    net: bnet,
                    word: None,
                });
                let ones = self.push_expr(ir::Expr::SysFunc {
                    which: ir::SysFuncId::CountOnes,
                    args: vec![bm],
                });
                let ones100 = self.push_expr(ir::Expr::Binary {
                    op: ir::BinOp::Mul,
                    lhs: ones,
                    rhs: hundred,
                });
                let nb = self.const_u32_expr(cx.num_bins, 32);
                let cx_cov = self.push_expr(ir::Expr::Binary {
                    op: ir::BinOp::Div,
                    lhs: ones100,
                    rhs: nb,
                });
                sum = Some(match sum {
                    None => cx_cov,
                    Some(acc) => self.push_expr(ir::Expr::Binary {
                        op: ir::BinOp::Add,
                        lhs: acc,
                        rhs: cx_cov,
                    }),
                });
                total_weight += 1; // crosses default to weight 1
            }
        }
        let Some(sum) = sum else {
            return self.real_const_expr("0.0"); // no counting coverpoints
        };
        let nexpr = self.const_u32_expr(total_weight.max(1), 32);
        self.push_expr(ir::Expr::Binary {
            op: ir::BinOp::Div,
            lhs: sum,
            rhs: nexpr,
        })
    }

    /// A covergroup method in STATEMENT position (`c.sample();`).
    pub(crate) fn synth_cover_method_stmt(
        &mut self,
        b: &mut ProcessBuilder,
        inst: &str,
        method: &str,
        args: &[ast::Expr],
    ) {
        match method {
            "sample" => {
                if !args.is_empty() {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "covergroup sample() takes no arguments in this slice",
                    );
                }
                self.synth_cover_sample(b, inst);
            }
            "get_coverage" => self.error(
                MsgCode::ElabUnsupported,
                "get_coverage() result must be used (it returns the coverage %)",
            ),
            _ => self.error(
                MsgCode::ElabUnsupported,
                &format!("unsupported covergroup method `.{method}()`"),
            ),
        }
    }

    /// A covergroup method in EXPRESSION position (`c.get_coverage()`).
    pub(crate) fn synth_cover_method_expr(&mut self, inst: &str, method: &str) -> u32 {
        match method {
            "get_coverage" => self.synth_cover_get(inst),
            "sample" => {
                self.error(
                    MsgCode::ElabUnsupported,
                    "sample() is a statement, not an expression",
                );
                self.placeholder_expr()
            }
            _ => {
                self.error(
                    MsgCode::ElabUnsupported,
                    &format!("unsupported covergroup method `.{method}()`"),
                );
                self.placeholder_expr()
            }
        }
    }
}
