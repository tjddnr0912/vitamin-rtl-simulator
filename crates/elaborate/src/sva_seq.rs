//! SVA sequences — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

/// The cycle gap before a sequence term in a flattened alternative: a fixed
/// `##d` delay (`d` shift-register stages), or an unbounded `##[m:$]` (≥m — an
/// `m-1` fixed delay followed by a never-reset `armed` latch, so every later
/// term clock re-completes the match).
#[derive(Clone, Copy)]
pub(crate) enum SeqHop {
    Fixed(u32),
    AtLeast(u32),
    /// A BOUNDED range `##[m:n]` (m <= n) carried as a SINGLE hop rather than
    /// fanned out into `n-m+1` `Fixed(d)` alternatives (SVA-QUAD collapse). Emitted
    /// only when `collapse_window()` is on; `synth_seq_pipeline` lowers it to a
    /// shared O(n-m) sliding-OR window (one shift chain whose every depth is OR-ed),
    /// verdict-identical to the fan-out (proven by the OR-vs-delay commutation lemma,
    /// `reg(x|y) == reg(x)|reg(y)`). Pure IR-0 (internal enum, never serialized).
    Range(u32, u32),
}

/// A term in a flattened sequence alternative: a plain boolean, or a
/// goto/nonconsecutive repetition of a boolean (synthesized as an existence-
/// latch FSM rather than a fixed shift).
#[derive(Clone)]
pub(crate) enum SeqTerm {
    Bool(ast::Expr),
    /// `b[->n]` — completes on the n-th (gap-allowed) occurrence of `b`.
    Goto(ast::Expr, u32),
    /// `b[=n]` — n occurrences of `b`, match extends past the n-th until the next.
    Nonconsec(ast::Expr, u32),
    /// `b[*m:$]` — `b` true for ≥ m CONSECUTIVE clocks (slice S13). Synthesized as
    /// a gated run-latch (a chain of 1-bit regs that saturates at the count `m`)
    /// rather than a fixed shift, since the upper bound is unbounded.
    ConsecAtLeast(ast::Expr, u32),
    /// The EMPTY (zero-repetition) match of `b[*0:..]` — a zero-extent thread that
    /// carries no boolean. For `X ##hop_in b[*0:n] ##hop_out Y` with FIXED hop_in>=1
    /// AND FIXED hop_out>=1, `synth_seq_pipeline` fuses the empty branch with net
    /// delay D=(hop_in-1)+hop_out (§16.9.2.1 `(r ##n ε)=(r ##(n-1) `true)`, the
    /// empty absorbing exactly one clock of hop_in): the empty emits its `hop_in-1`
    /// remaining shifts with no AND, then Y's hop loop applies the hop_out shifts.
    /// Honest-loud (subtle §16.9.2.1 algebra, no differential oracle — never a
    /// guessed/silent delay): leading `##0` (an absorption discontinuity), trailing
    /// `##0` (a bitten off-by-one), `##[m:$]` (a range), and the SEED position.
    Empty,
}

/// One expanded sequence alternative: an ordered (term, hop) list plus an
/// optional already-reduced (1-bit) `throughout` guard that must hold at every
/// clock of the window.
pub(crate) type SeqAlt = (Vec<(SeqTerm, SeqHop)>, Option<ast::Expr>);

/// A single never-matching (`1'b0`) sequence alternative — the recovery value
/// substituted when a sequence form is rejected (e.g. a repetition count over
/// the cap). `1'b0` (rather than `1'b1`) so the errored design can never produce
/// a spurious assertion fire on the abort path.
pub(crate) fn sva_never_alt(seq: &ast::Sequence) -> Vec<SeqAlt> {
    vec![(
        vec![(SeqTerm::Bool(sva_zero(seq_span(seq))), SeqHop::Fixed(0))],
        None,
    )]
}

/// The clock-window length (in clocks) of a flattened bounded Bool-only
/// alternative: 1 (the seed clock) plus the sum of the inter-term `##d` delays.
pub(crate) fn window_len(terms: &[(SeqTerm, SeqHop)]) -> u32 {
    1 + terms
        .iter()
        .skip(1)
        .map(|(_, h)| match h {
            SeqHop::Fixed(d) => *d,
            SeqHop::AtLeast(_) => 0,
            // The window's MAX length (largest fan-out alternative). `window_len` is
            // only consumed by `synth_within` / `build_seq_consequent`, both of which
            // loud-reject a non-`Fixed` hop before reaching here, so this is a defined-
            // but-unreached fallback (kept total).
            SeqHop::Range(_, n) => *n,
        })
        .sum::<u32>()
}

/// A representative span for a `Sequence` node (its first boolean leaf).
pub(crate) fn seq_span(seq: &ast::Sequence) -> ast::Span {
    match seq {
        ast::Sequence::Boolean(e) => e.span,
        ast::Sequence::Delay { lhs, .. } => seq_span(lhs),
        ast::Sequence::Repeat { seq, .. } => seq_span(seq),
        ast::Sequence::Throughout { cond, .. } => cond.span,
        ast::Sequence::Within { seq1, .. } => seq_span(seq1),
        ast::Sequence::Clocked { seq, .. } => seq_span(seq),
        ast::Sequence::Instance { span, .. } => *span,
        ast::Sequence::MatchItem { seq, .. } => seq_span(seq),
    }
}

/// True iff `seq` contains any re-clocking `@(clk)` boundary (slice N2a) — used to
/// route a cross-clock sequence to `synth_crossclock` instead of the single-clock
/// pipeline.
pub(crate) fn seq_has_clocked(seq: &ast::Sequence) -> bool {
    match seq {
        ast::Sequence::Clocked { .. } => true,
        ast::Sequence::Boolean(_) | ast::Sequence::Instance { .. } => false,
        ast::Sequence::Delay { lhs, rhs, .. } => seq_has_clocked(lhs) || seq_has_clocked(rhs),
        ast::Sequence::Repeat { seq, .. } => seq_has_clocked(seq),
        ast::Sequence::Throughout { seq, .. } => seq_has_clocked(seq),
        ast::Sequence::Within { seq1, seq2 } => seq_has_clocked(seq1) || seq_has_clocked(seq2),
        ast::Sequence::MatchItem { seq, .. } => seq_has_clocked(seq),
    }
}

/// True iff `seq` contains a local-variable CAPTURE `(b, x = e)` anywhere (slice
/// N2c) — used to route an assertion carrying captures to `synth_local_var_assert`
/// (the data-tracking shift register) instead of the byte-identical flat path.
pub(crate) fn seq_has_match_item(seq: &ast::Sequence) -> bool {
    match seq {
        ast::Sequence::MatchItem { .. } => true,
        ast::Sequence::Boolean(_) | ast::Sequence::Instance { .. } => false,
        ast::Sequence::Delay { lhs, rhs, .. } => seq_has_match_item(lhs) || seq_has_match_item(rhs),
        ast::Sequence::Repeat { seq, .. } => seq_has_match_item(seq),
        ast::Sequence::Throughout { seq, .. } => seq_has_match_item(seq),
        ast::Sequence::Within { seq1, seq2 } => {
            seq_has_match_item(seq1) || seq_has_match_item(seq2)
        }
        ast::Sequence::Clocked { seq, .. } => seq_has_match_item(seq),
    }
}

/// Fold a REDUNDANT same-clock re-clocking `@(clk) seq` (where `clk` equals the
/// property's leading clock) to plain `seq` — it is a §16.13 no-op. This lets a
/// degenerate `@(clk) a ##1 @(clk) b` share the byte-correct SINGLE-clock pipeline
/// instead of the lossy cross-clock handoff (review N2a-1: a same-clock re-clock
/// otherwise under-fired ~2x, because the handoff's NBA reads the prior coincident
/// edge). Only Clocked nodes whose clock is span-agnostically identical to
/// `prop_clock` are stripped; a genuinely different clock is preserved (→
/// `synth_crossclock`). Called only when the antecedent already contains a Clocked
/// node, so the common no-Clocked path is byte-identical.
pub(crate) fn strip_redundant_clocks(
    seq: &ast::Sequence,
    prop_clock: &ast::Sensitivity,
) -> ast::Sequence {
    match seq {
        ast::Sequence::Clocked { clock, seq: inner } => {
            let stripped = strip_redundant_clocks(inner, prop_clock);
            let same = sva_clock_signal(clock).is_some()
                && sva_clock_signal(clock) == sva_clock_signal(prop_clock);
            if same {
                stripped
            } else {
                ast::Sequence::Clocked {
                    clock: clock.clone(),
                    seq: Box::new(stripped),
                }
            }
        }
        ast::Sequence::Delay { min, max, lhs, rhs } => ast::Sequence::Delay {
            min: *min,
            max: *max,
            lhs: Box::new(strip_redundant_clocks(lhs, prop_clock)),
            rhs: Box::new(strip_redundant_clocks(rhs, prop_clock)),
        },
        ast::Sequence::Repeat {
            seq,
            min,
            max,
            kind,
        } => ast::Sequence::Repeat {
            seq: Box::new(strip_redundant_clocks(seq, prop_clock)),
            min: *min,
            max: *max,
            kind: *kind,
        },
        ast::Sequence::Throughout { cond, seq } => ast::Sequence::Throughout {
            cond: cond.clone(),
            seq: Box::new(strip_redundant_clocks(seq, prop_clock)),
        },
        ast::Sequence::Within { seq1, seq2 } => ast::Sequence::Within {
            seq1: Box::new(strip_redundant_clocks(seq1, prop_clock)),
            seq2: Box::new(strip_redundant_clocks(seq2, prop_clock)),
        },
        ast::Sequence::MatchItem { seq, assigns } => ast::Sequence::MatchItem {
            seq: Box::new(strip_redundant_clocks(seq, prop_clock)),
            assigns: assigns.clone(),
        },
        ast::Sequence::Boolean(_) | ast::Sequence::Instance { .. } => seq.clone(),
    }
}

/// Substitute SVA formals (slice A1) through a `Sequence`, recursing into the
/// boolean leaves / guards. A nested named-sequence `Instance` whose NAME is itself
/// a formal bound to a bare-ident actual is renamed; its args are substituted too.
pub(crate) fn subst_sequence(
    seq: &ast::Sequence,
    map: &BTreeMap<String, ast::Expr>,
) -> ast::Sequence {
    use ast::Sequence as S;
    match seq {
        S::Boolean(e) => S::Boolean(subst_expr(e, map)),
        S::Delay { min, max, lhs, rhs } => S::Delay {
            min: *min,
            max: *max,
            lhs: Box::new(subst_sequence(lhs, map)),
            rhs: Box::new(subst_sequence(rhs, map)),
        },
        S::Repeat {
            seq,
            min,
            max,
            kind,
        } => S::Repeat {
            seq: Box::new(subst_sequence(seq, map)),
            min: *min,
            max: *max,
            kind: *kind,
        },
        S::Throughout { cond, seq } => S::Throughout {
            cond: Box::new(subst_expr(cond, map)),
            seq: Box::new(subst_sequence(seq, map)),
        },
        S::Within { seq1, seq2 } => S::Within {
            seq1: Box::new(subst_sequence(seq1, map)),
            seq2: Box::new(subst_sequence(seq2, map)),
        },
        S::Clocked { clock, seq } => S::Clocked {
            clock: clock.clone(),
            seq: Box::new(subst_sequence(seq, map)),
        },
        S::Instance { name, args, span } => {
            let name = match map.get(&name.name) {
                Some(ast::Expr {
                    kind: ast::ExprKind::Ident(p),
                    ..
                }) if p.segments.len() == 1 => ast::Ident {
                    name: p.segments[0].name.clone(),
                    span: name.span,
                },
                _ => name.clone(),
            };
            S::Instance {
                name,
                args: args.iter().map(|a| subst_expr(a, map)).collect(),
                span: *span,
            }
        }
        S::MatchItem { seq, assigns } => S::MatchItem {
            seq: Box::new(subst_sequence(seq, map)),
            assigns: assigns
                .iter()
                .map(|(n, v)| (n.clone(), subst_expr(v, map)))
                .collect(),
        },
    }
}

/// Cap on the number of disjunctive ALTERNATIVES a bounded SVA sequence range
/// (`##[m:n]`/`[*m:n]`, possibly nested/producted) may expand to before a loud
/// reject — the range-blowup guard. Note the synthesized pipeline regs are NOT
/// prefix-shared across alternatives, so a single `[*1:N]` allocates ~N²/2 regs
/// (each `[*k]` alternative its own k-1 stage chain); the cap therefore bounds
/// the worst-case reg count quadratically (≈cap²/2), not linearly. That still
/// elaborates deterministically at the cap; prefix-sharing is a perf follow-on.
pub(crate) const SVA_SEQ_ALT_CAP: usize = 256;

/// SVA-QUAD collapse selector. When ON, a bounded `##[m:n]` (m < n) delay is
/// lowered to ONE shared O(n-m) sliding-OR window (`SeqHop::Range`) instead of
/// `n-m+1` fanned `Fixed(d)` alternatives (the O(N^2) prefix-rebuild). The two
/// lowerings are VERDICT-IDENTICAL (the differential gate in
/// `crates/cli/tests/sva_quad_differential.rs` proves it across a directed +
/// randomized fuzz corpus); the collapse only changes the internal SVA-checker reg
/// COUNT/NAMES, so the emitted VCD's internal `__sva_*` net set differs for
/// `##[m:n]` designs — that is checker plumbing, NOT observable design behavior,
/// and assertion verdicts (fire/hold + exit + diagnostics) are preserved exactly.
///
/// Default = OFF (fan-out), so the WHOLE existing suite — including any full-VCD
/// byte-identity goldens on `##[m:n]` designs — is byte-identical to today. The
/// fan-out impl is retained as the PERMANENT differential oracle. Set
/// `VITA_SVA_COLLAPSE` to enable the collapse (the differential test sets it on one
/// of its two runs to compare verdict streams). Flipping the default is a one-line
/// change here once a full-VCD audit clears the net-set blast radius (deferred —
/// see `impl2-SVA-QUAD-collapse.md`).
pub(crate) fn collapse_window() -> bool {
    std::env::var_os("VITA_SVA_COLLAPSE").is_some()
}

/// Loud message for an empty-match repetition `b[*0:..]` adjacent to a delay the
/// fusion cannot verify. Any FIXED hop_in>=1 AND FIXED hop_out>=1 fuses with net
/// delay D=(hop_in-1)+hop_out (slice A.1, §16.9.2.1 `(r ##n ε)=(r ##(n-1) `true)`).
/// The RESIDUAL loud set is: leading `##0` (a genuine §16.9.2.1 absorption
/// discontinuity, D=hop_out not hop_out-1), trailing `##0` (a historically-bitten
/// off-by-one), and `##[m:$]` (an unverifiable range) — all with no differential
/// oracle (iverilog rejects concurrent SVA), so honest-loud (never guessed).
pub(crate) const EMPTY_MATCH_HASH1_ONLY: &str =
    "an empty-match repetition `b[*0:..]` requires a FIXED `##d` (d>=1) delay on \
     both sides (e.g. `a ##2 b[*0:n] ##1 c`); a leading/trailing `##0` or an \
     `##[m:$]` adjacent to the empty is unsupported in this subset";

impl Elaborator<'_> {
    /// Reject an SVA repetition count that exceeds the synthesis cap
    /// (`SVA_SEQ_ALT_CAP`). Every repetition count synthesizes O(count) 1-bit
    /// helper regs (goto/nonconsec/unbounded-consec FSMs) or fans a bounded
    /// `[*n]` into an n-term shift pipeline, so an absurd literal would hang
    /// elaboration; this caps it loudly, mirroring the post-expansion alternative
    /// cap and the bounded-range / `within` guards. Returns `false` (with the
    /// error already emitted) when the count is over the cap.
    pub(crate) fn sva_count_within_cap(&mut self, count: u32, what: &str) -> bool {
        if count as usize > SVA_SEQ_ALT_CAP {
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "an SVA {what} count {count} exceeds the cap {SVA_SEQ_ALT_CAP}; narrow it"
                ),
            );
            false
        } else {
            true
        }
    }

    /// Expand a `Sequence` into a DISJUNCTION of conjunctive term-lists — each a
    /// `Vec<(boolean-term, hop-delay)>` where `hop-delay` is the `##d` cycle gap
    /// BEFORE that term (the first term's delay is unused — it is the seed). A
    /// bounded range `##[m:n]` / `[*m:n]` fans out into `n-m+1` (or a product of
    /// such) alternatives; the antecedent matches if ANY alternative completes.
    /// `[*k]` repetitions expand to `##1` chains. Each leaf term is passed
    /// through `rewrite_sampled` (deduped by signal), so a sampled-value fn
    /// inside a sequence term still works and is allocated once.
    /// Inline a named sequence INSTANCE: cycle-guard, then expand the declared
    /// body (which may itself reference other named sequences — handled by the
    /// recursion). A self/mutual-recursive sequence (IEEE 1800 §16.8: illegal) is
    /// rejected loud and yields a never-matching `1'b0` alternative so elaboration
    /// continues. Parameterized decls are rejected loud (reserved for a follow-on).
    pub(crate) fn inline_named_sequence(
        &mut self,
        decl: &ast::SeqDecl,
        args: &[ast::Expr],
        regs: &mut SvaRegs,
    ) -> Vec<SeqAlt> {
        // Positional formal-argument binding (slice A1). Arity mismatch (including a
        // non-parameterized sequence given args, or vice versa) is loud.
        if decl.formals.len() != args.len() {
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "named sequence `{}` expects {} formal argument(s), got {}",
                    decl.name.name,
                    decl.formals.len(),
                    args.len()
                ),
            );
            return sva_never_alt(&decl.body);
        }
        if self.sva_inline_stack.iter().any(|n| n == &decl.name.name) {
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "recursive sequence `{}` is illegal (IEEE 1800 §16.8)",
                    decl.name.name
                ),
            );
            return sva_never_alt(&decl.body);
        }
        // The no-formal path clones (then expands) the body, structurally identical
        // to expanding `&decl.body` directly → byte-identical to before slice A1.
        let body = if decl.formals.is_empty() {
            decl.body.clone()
        } else {
            subst_sequence(&decl.body, &sva_formal_map(&decl.formals, args))
        };
        self.sva_inline_stack.push(decl.name.name.clone());
        let out = self.expand_sequence(&body, regs);
        self.sva_inline_stack.pop();
        out
    }

    /// SEQ-DEPTH guard: `expand_sequence` recurses on `##`/`[*]`/`and`/`or`/
    /// `within` sub-sequences with no depth cap, so a 35k-deep nested sequence
    /// overflows the elaborate stack (raw SIGABRT). Cap the recursion at
    /// `SVA_SEQ_ALT_CAP` (256 — the same robustness budget the prop-level and/or
    /// reduction and the per-attempt alternative count use) → loud
    /// `ElabUnsupported` + a never-matching alternative (`had_error` discards the
    /// IR, so the never-alt is never actually simulated).
    pub(crate) fn expand_sequence(
        &mut self,
        seq: &ast::Sequence,
        regs: &mut SvaRegs,
    ) -> Vec<SeqAlt> {
        self.sva_seq_depth += 1;
        if self.sva_seq_depth as usize > SVA_SEQ_ALT_CAP {
            self.sva_seq_depth -= 1;
            self.error(
                MsgCode::ElabUnsupported,
                &format!("sequence nesting exceeds the depth cap ({SVA_SEQ_ALT_CAP}); narrow it"),
            );
            return sva_never_alt(seq);
        }
        let r = self.expand_sequence_inner(seq, regs);
        self.sva_seq_depth -= 1;
        r
    }

    pub(crate) fn expand_sequence_inner(
        &mut self,
        seq: &ast::Sequence,
        regs: &mut SvaRegs,
    ) -> Vec<SeqAlt> {
        match seq {
            ast::Sequence::Boolean(e) => {
                // A bare single-segment identifier that names a declared sequence is
                // a sequence INSTANCE — inline its body (cycle-guarded). Anything
                // else (a real net, an expression) is an ordinary boolean leaf, so a
                // net and a sequence of the same name coexist (lookup miss → leaf).
                if let ast::ExprKind::Ident(path) = &e.kind {
                    if path.segments.len() == 1 {
                        if let Some(decl) = self.seq_table.get(&path.segments[0].name).cloned() {
                            return self.inline_named_sequence(&decl, &[], regs);
                        }
                    }
                }
                // A `Call` whose callee names a declared sequence is a PARAMETERIZED
                // sequence instance `s(a,b)` (it parses as a boolean-leaf `Call` in a
                // sequence body / antecedent). Bind the actuals and inline. A callee
                // that is NOT a declared sequence falls through to the ordinary
                // boolean leaf (an actual user function call → the usual lowering).
                if let ast::ExprKind::Call { name, args } = &e.kind {
                    if name.segments.len() == 1 {
                        if let Some(decl) = self.seq_table.get(&name.segments[0].name).cloned() {
                            return self.inline_named_sequence(&decl, args, regs);
                        }
                    }
                }
                let term = self.rewrite_sampled(e, regs);
                vec![(vec![(SeqTerm::Bool(term), SeqHop::Fixed(0))], None)]
            }
            // An explicit named instance reaching a sequence position (a property
            // instance is spliced at collect time and never gets here; this arm
            // covers a named SEQUENCE instance / future forms): resolve it against
            // the sequence table and inline. Unknown name / non-empty args are loud.
            ast::Sequence::Instance { name, args, .. } => {
                match self.seq_table.get(&name.name).cloned() {
                    // Inline (binding `args` to the declared formals — slice A1).
                    Some(decl) => self.inline_named_sequence(&decl, args, regs),
                    None => {
                        self.error(
                            MsgCode::ElabUnsupported,
                            &format!("unknown sequence `{}`", name.name),
                        );
                        sva_never_alt(seq)
                    }
                }
            }
            ast::Sequence::Delay {
                min, max, lhs, rhs, ..
            } => {
                let ls = self.expand_sequence(lhs, regs);
                let rs = self.expand_sequence(rhs, regs);
                let mut out = Vec::new();
                // An unbounded `##[m:$]` cannot fan out — it becomes one
                // `AtLeast(m)` hop (synthesized as a latch). A bounded `##[m:n]`
                // fans into the n-m+1 `Fixed(d)` delay alternatives (the O(N^2)
                // path), UNLESS the SVA-QUAD collapse is enabled and the range is
                // non-degenerate (n > m), in which case ONE `Range(m,n)` hop carries
                // the whole window (lowered to a shared O(n-m) sliding-OR in
                // `synth_seq_pipeline`). `n == m` is a single `Fixed(n)` either way.
                let hops: Vec<SeqHop> = match max {
                    None => vec![SeqHop::AtLeast(*min)],
                    Some(n) if collapse_window() && *n > *min => {
                        // SVA-QUAD collapse: ONE `Range(m,n)` hop carries the whole
                        // window, but it MUST respect the SAME alternative cap as the
                        // fan-out path (which would loud-reject `n-m+1 > SVA_SEQ_ALT_CAP`
                        // alternatives at `alternatives.len()` post-expansion). Without
                        // this, collapse would silently run a window the fan-out oracle
                        // rejects (a loud-vs-correct divergence that breaks the
                        // differential premise) and would allocate `n-m` 1-bit regs
                        // uncapped (`##[1:5_000_000]` → OOM). Capping the window depth
                        // makes collapse and fan-out agree at the boundary AND bounds
                        // the sliding-OR reg allocation.
                        if self.sva_count_within_cap(*n - *min + 1, "`##[m:n]` window") {
                            vec![SeqHop::Range(*min, *n)]
                        } else {
                            // Error already emitted; degrade to a single Fixed hop so
                            // elaboration recovers (the assertion is already invalid).
                            vec![SeqHop::Fixed(*min)]
                        }
                    }
                    Some(n) => (*min..=*n).map(SeqHop::Fixed).collect(),
                };
                for hop in hops {
                    for (lt, lg) in &ls {
                        for (rt, rg) in &rs {
                            let mut combined = lt.clone();
                            let mut r2 = rt.clone();
                            // The first term of `rhs` is reached via `hop` after
                            // the last term of `lhs`.
                            if let Some(first) = r2.first_mut() {
                                first.1 = hop;
                            }
                            combined.extend(r2);
                            out.push((combined, and_opt(lg.clone(), rg.clone(), seq_span(lhs))));
                        }
                    }
                }
                out
            }
            ast::Sequence::Repeat {
                seq,
                min,
                kind: kind @ (ast::RepeatKind::Goto | ast::RepeatKind::Nonconsec),
                ..
            } => {
                // goto `[->n]` / nonconsec `[=n]` synthesize an existence-latch
                // FSM rather than a fixed shift, so they become a single FSM
                // term (boolean operand only; `min == max == n`).
                let n = (*min).max(1);
                if !self.sva_count_within_cap(n, "goto/nonconsec repetition") {
                    return sva_never_alt(seq);
                }
                let ast::Sequence::Boolean(b) = &**seq else {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "goto/nonconsec repetition requires a boolean operand in this subset",
                    );
                    return vec![(
                        vec![(SeqTerm::Bool(sva_one(seq_span(seq))), SeqHop::Fixed(0))],
                        None,
                    )];
                };
                let bt = self.rewrite_sampled(b, regs);
                let term = match kind {
                    ast::RepeatKind::Goto => SeqTerm::Goto(bt, n),
                    _ => SeqTerm::Nonconsec(bt, n),
                };
                vec![(vec![(term, SeqHop::Fixed(0))], None)]
            }
            ast::Sequence::Repeat {
                seq,
                min,
                max: None,
                kind: ast::RepeatKind::Consec,
            } => {
                // `b[*m:$]` — unbounded consecutive repeat (≥ m). Cannot fan out;
                // synthesize a gated run-latch (a single ConsecAtLeast term).
                // Boolean operand only (S8 goto/nonconsec precedent). `[*0:$]` /
                // `[*]` (m == 0, empty-or-more) also yields the EMPTY alternative.
                let want_empty = *min == 0;
                let m = (*min).max(1);
                if !self.sva_count_within_cap(m, "unbounded consecutive repetition") {
                    return sva_never_alt(seq);
                }
                let ast::Sequence::Boolean(b) = &**seq else {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "unbounded consecutive repetition `[*m:$]` requires a boolean operand in this subset",
                    );
                    return vec![(
                        vec![(SeqTerm::Bool(sva_one(seq_span(seq))), SeqHop::Fixed(0))],
                        None,
                    )];
                };
                let bt = self.rewrite_sampled(b, regs);
                let mut out = Vec::new();
                if want_empty {
                    out.push((vec![(SeqTerm::Empty, SeqHop::Fixed(0))], None));
                }
                out.push((
                    vec![(SeqTerm::ConsecAtLeast(bt, m), SeqHop::Fixed(0))],
                    None,
                ));
                out
            }
            ast::Sequence::Repeat {
                seq,
                min,
                max,
                kind: ast::RepeatKind::Consec,
            } => {
                // `[*0:n]` (m == 0) admits the EMPTY (zero-repetition) alternative
                // ALONGSIDE the `[*1:n]` fan-out. `[*0]` / `[*0:0]` (hi == 0) is
                // EXACTLY empty — the pure zero-extent match, no non-empty alts.
                let want_empty = *min == 0;
                let hi_raw = max.unwrap_or(*min); // this arm only fires for max: Some(_)
                if hi_raw == 0 {
                    return vec![(vec![(SeqTerm::Empty, SeqHop::Fixed(0))], None)];
                }
                let base = self.expand_sequence(seq, regs);
                let lo = (*min).max(1);
                let hi = hi_raw.max(lo);
                // Cap the upper count: each copy adds `base`'s terms to every
                // alternative (an `n`-term shift pipeline for the exact `[*n]`
                // case, which the post-expansion alternative-COUNT cap misses),
                // so an absurd literal would hang the fan-out below.
                if !self.sva_count_within_cap(hi, "consecutive repetition") {
                    return sva_never_alt(seq);
                }
                let mut out = Vec::new();
                if want_empty {
                    out.push((vec![(SeqTerm::Empty, SeqHop::Fixed(0))], None));
                }
                'kloop: for k in lo..=hi {
                    // k copies of `base`, each copy after the first prefixed with
                    // `##1`. A multi-alternative base makes this a k-fold product.
                    let mut combos: Vec<SeqAlt> = vec![(Vec::new(), None)];
                    for i in 0..k {
                        let mut next = Vec::new();
                        for (pterms, pg) in &combos {
                            for (bterms, bg) in &base {
                                let mut copy = bterms.clone();
                                if i > 0 {
                                    if let Some(first) = copy.first_mut() {
                                        first.1 = SeqHop::Fixed(1);
                                    }
                                }
                                let mut merged = pterms.clone();
                                merged.extend(copy);
                                let sp = seq_span(seq);
                                next.push((merged, and_opt(pg.clone(), bg.clone(), sp)));
                            }
                        }
                        combos = next;
                        // Guard the k-fold PRODUCT of a multi-alternative base
                        // (e.g. `(a ##[1:2] b)[*1:20]` = 2^20) from exploding the
                        // build before the post-expansion cap can truncate it.
                        if out.len() + combos.len() > SVA_SEQ_ALT_CAP {
                            self.error(
                                MsgCode::ElabUnsupported,
                                &format!(
                                    "an SVA bounded repetition expanded past the cap {SVA_SEQ_ALT_CAP}; narrow the ranges"
                                ),
                            );
                            break 'kloop;
                        }
                    }
                    out.extend(combos);
                }
                out
            }
            ast::Sequence::Throughout { cond, seq } => {
                let inner = self.expand_sequence(seq, regs);
                let sp = cond.span;
                let g = sva_unary(ast::UnOp::RedOr, self.rewrite_sampled(cond, regs), sp);
                let mut out = Vec::new();
                for (terms, og) in inner {
                    // `throughout` over an unbounded inner hop (`##[m:$]`) or a
                    // goto/nonconsec FSM term would need the guard threaded
                    // through the latch/FSM — deferred.
                    if terms.iter().any(|(t, h)| {
                        matches!(h, SeqHop::AtLeast(_)) || !matches!(t, SeqTerm::Bool(_))
                    }) {
                        self.error(
                            MsgCode::ElabUnsupported,
                            "`throughout` over an unbounded or goto/nonconsec sequence is unsupported in this subset",
                        );
                    }
                    out.push((terms, and_opt(og, Some(g.clone()), sp)));
                }
                out
            }
            ast::Sequence::Within { seq1, .. } => {
                // `within` is synthesized as a whole (two sub-pipelines combined)
                // by `synth_within` at the top level — it cannot appear as a term
                // inside a larger `##` chain in this subset.
                self.error(
                    MsgCode::ElabUnsupported,
                    "`within` is only supported as a top-level concurrent-assertion antecedent",
                );
                vec![(
                    vec![(SeqTerm::Bool(sva_zero(seq_span(seq1))), SeqHop::Fixed(0))],
                    None,
                )]
            }
            ast::Sequence::Clocked { seq, .. } => {
                // A re-clocking boundary is handled WHOLE by `synth_crossclock` (the
                // top-level `a ##1 @(c2) b` cross-clock antecedent, slice N2a-1) — it
                // is routed away before this single-clock pipeline. Reaching here means
                // a `@(clk)` boundary in an unsupported position (nested / inside a
                // larger chain) — loud, never a silent single-clock mis-sample.
                self.error(
                    MsgCode::ElabUnsupported,
                    "a re-clocking `@(clk)` boundary is only supported as the top-level \
                     `a ##1 @(c2) b` cross-clock antecedent in this subset",
                );
                vec![(
                    vec![(SeqTerm::Bool(sva_zero(seq_span(seq))), SeqHop::Fixed(0))],
                    None,
                )]
            }
            ast::Sequence::MatchItem { seq, .. } => {
                // A local-variable CAPTURE `(b, x = e)` reaching the generic expansion
                // is in an UNSUPPORTED position: the data-tracking single-capture path
                // (`synth_local_var_assert`) handles a fixed-delay antecedent capture
                // WHOLE before this point. A capture inside a consequent, a disjunction,
                // a repetition, or a ranged antecedent is out of subset → loud (never a
                // silent drop of the capture). Recover by expanding the bare boolean
                // term (the liveness is still correct; only the capture is dropped, and
                // the loud error discards the IR so it is never simulated).
                self.error(
                    MsgCode::ElabUnsupported,
                    "a sequence local-variable capture `(b, x = e)` is only supported in \
                     a single-clock FIXED-DELAY antecedent read in the consequent \
                     (ranges / disjunction / repetition / capture-in-consequent are \
                     unsupported in this subset)",
                );
                self.expand_sequence(seq, regs)
            }
        }
    }

    /// Synthesize the top-level `seq1 within seq2` antecedent match signal
    /// (slice S9): `seq1` must match entirely inside a `seq2` match. For bounded
    /// boolean operands this is `match(seq2) & OR_{i=0}^{L-k1} reg^i(match(seq1))`
    /// — over a seq2 window of length `L`, seq1 (length `k1`) completed at some
    /// clock that both ends within the window and starts at/after its start. ORed
    /// over every (seq2-alt × seq1-alt) combination. Pure IR-0.
    pub(crate) fn synth_within(
        &mut self,
        seq1: &ast::Sequence,
        seq2: &ast::Sequence,
        regs: &mut SvaRegs,
        nbas: &mut Vec<ast::Stmt>,
        sp: ast::Span,
    ) -> ast::Expr {
        let s1 = self.expand_sequence(seq1, regs);
        let s2 = self.expand_sequence(seq2, regs);
        // Both operands must be bounded, boolean-only, guard-free (no `##[m:$]`,
        // goto/nonconsec, throughout, or nested within inside).
        let ok = |alts: &[SeqAlt]| {
            alts.iter().all(|(t, g)| {
                g.is_none()
                    && t.iter().all(|(tm, h)| {
                        matches!(tm, SeqTerm::Bool(_)) && matches!(h, SeqHop::Fixed(_))
                    })
            })
        };
        if !ok(&s1) || !ok(&s2) {
            self.error(
                MsgCode::ElabUnsupported,
                "`within` requires bounded boolean sequences in this subset",
            );
            return sva_zero(sp);
        }
        if s1.len() * s2.len() > SVA_SEQ_ALT_CAP {
            self.error(
                MsgCode::ElabUnsupported,
                "a `within` expanded past the sequence alternative cap; narrow the bounded ranges",
            );
            return sva_zero(sp);
        }
        let mut combos: Vec<ast::Expr> = Vec::new();
        for (s2t, _) in &s2 {
            let l = window_len(s2t);
            let match_2 = self.synth_seq_pipeline(s2t.clone(), None, nbas, sp);
            for (s1t, _) in &s1 {
                let k1 = window_len(s1t);
                if k1 > l {
                    continue; // seq1 cannot fit in this seq2 window
                }
                let match_1 = self.synth_seq_pipeline(s1t.clone(), None, nbas, sp);
                // OR match_1 over the last (L - k1 + 1) clocks (the positions
                // where seq1 fits inside the seq2 window ending now).
                let mut acc = match_1.clone();
                let mut cur = match_1;
                for _ in 0..(l - k1) {
                    cur = self.seq_delay_reg(cur, nbas, sp);
                    acc = sva_binary(ast::BinOp::BitOr, acc, cur.clone(), sp);
                }
                combos.push(sva_binary(ast::BinOp::BitAnd, match_2.clone(), acc, sp));
            }
        }
        if combos.is_empty() {
            return sva_zero(sp); // seq1 longer than every seq2 window
        }
        let mut it = combos.into_iter();
        let mut acc = it.next().unwrap();
        for c in it {
            acc = sva_binary(ast::BinOp::BitOr, acc, c, sp);
        }
        acc
    }

    /// Synthesize the "sequence matches and ends THIS clock" boolean from a
    /// flattened (≥2-term) term list as a shift-register pipeline of 1-bit
    /// pending regs. `cur` starts as term0's truthiness, re-seeded every clock
    /// (overlapping match threads are inherent to the NBA shift). For each later
    /// term with hop-delay `d`, delay `cur` by `d` registered clocks (`d == 0` is
    /// same-cycle `##0` fusion — no register), then AND with that term's
    /// truthiness. A stage-reg read yields the PRIOR clock's value (the checker's
    /// if-check runs before the NBAs), so the chain advances one term per clock.
    /// Every term is reduced with `|` so a multi-bit term is a boolean (the F1
    /// reduction-OR rule). Pure IR-0 — only pre-existing sim-ir nodes.
    pub(crate) fn synth_seq_pipeline(
        &mut self,
        terms: Vec<(SeqTerm, SeqHop)>,
        guard: Option<ast::Expr>,
        pipeline_nbas: &mut Vec<ast::Stmt>,
        sp: ast::Span,
    ) -> ast::Expr {
        // A `throughout` guard `g` (already 1-bit) must hold at EVERY clock the
        // thread is alive: AND it into the seed and after every shift stage.
        let guard_and = |cur: ast::Expr| match &guard {
            Some(g) => sva_binary(ast::BinOp::BitAnd, cur, g.clone(), sp),
            None => cur,
        };
        let mut it = terms.into_iter();
        let (t0, _) = it
            .next()
            .expect("synth_seq_pipeline requires at least one term");
        // Seed: a Bool term is just `|t0`; a leading goto/nonconsec activates a
        // counting thread every clock (act = 1'b1).
        let seed = match t0 {
            SeqTerm::Bool(e) => sva_unary(ast::UnOp::RedOr, e, sp),
            SeqTerm::Goto(b, n) => self.goto_fsm(sva_one(sp), b, n, pipeline_nbas, sp),
            SeqTerm::Nonconsec(b, n) => self.nonconsec_fsm(sva_one(sp), b, n, pipeline_nbas, sp),
            SeqTerm::ConsecAtLeast(b, m) => {
                self.consec_run_fsm(sva_one(sp), b, m, pipeline_nbas, sp)
            }
            // An empty match as the SEED is a leading / standalone `b[*0:..]` —
            // its zero-extent thread starts one clock "before" the attempt, an
            // offset the start-of-pipeline cannot carry. Honest-loud (never a
            // silent miss); recover with a never-matching `1'b0`.
            SeqTerm::Empty => {
                self.error(
                    MsgCode::ElabUnsupported,
                    "an empty-match repetition `b[*0:..]` as the leading or standalone \
                     term of a sequence is unsupported in this subset (place it after a \
                     term, e.g. `a ##1 b[*0:n]`)",
                );
                sva_zero(sp)
            }
        };
        let mut cur = guard_and(seed);
        // True once an EMPTY (zero-repetition) term has been emitted: the hop of
        // the term that FOLLOWS it must be `##1` (see below).
        let mut after_empty = false;
        for (term, hop) in it {
            // Empty-match fusion (IEEE 1800-2017 §16.9.2.1, `(r ##n ε)=(r ##(n-1)
            // `true)`): for `X ##hop_in b[*0:n] ##hop_out Y` the empty branch has
            // net delay D = (hop_in-1) + hop_out — the length-0 empty absorbs
            // exactly ONE clock of hop_in. This holds for any FIXED hop_in>=1 AND
            // FIXED hop_out>=1 (slice A.1, "P1"). The hop FOLLOWING an empty
            // (= hop_out) may be any `Fixed(d>=1)`; Y's own hop loop below applies
            // those d shifts. Trailing `##0` (Fixed(0)) is a historically-bitten
            // off-by-one and `##[m:$]` (AtLeast) is a range with no oracle — both
            // stay HONEST-LOUD rather than a guessed (silent-wrong) delay:
            if after_empty && !matches!(hop, SeqHop::Fixed(d) if d >= 1) {
                self.error(MsgCode::ElabUnsupported, EMPTY_MATCH_HASH1_ONLY);
            }
            after_empty = false;
            // An EMPTY term: its own hop (= hop_in) must be `Fixed(d>=1)`. The
            // empty consumes exactly one clock of that hop (the length-0 match),
            // so emit the remaining `d-1` shifts here, then pass the thread on with
            // NO boolean AND. Leading `##0` (Fixed(0)) is a genuine §16.9.2.1
            // absorption discontinuity (D=hop_out, not hop_out-1) with no oracle,
            // and `##[m:$]` (AtLeast) is an unverifiable range — both HONEST-LOUD.
            if matches!(term, SeqTerm::Empty) {
                match hop {
                    SeqHop::Fixed(d) if d >= 1 => {
                        for _ in 0..(d - 1) {
                            cur = guard_and(self.seq_delay_reg(cur, pipeline_nbas, sp));
                        }
                    }
                    _ => self.error(MsgCode::ElabUnsupported, EMPTY_MATCH_HASH1_ONLY),
                }
                after_empty = true;
                continue;
            }
            match hop {
                SeqHop::Fixed(d) => {
                    for _ in 0..d {
                        cur = guard_and(self.seq_delay_reg(cur, pipeline_nbas, sp));
                    }
                }
                // SVA-QUAD collapse: a bounded `##[m:n]` (m < n) lowered as ONE
                // shared sliding-OR window instead of `n-m+1` fanned alternatives.
                // Entering, `cur` = `guard_and(seed)` = the guard-anded COMBINATIONAL
                // this-clock prefix activation. Reach depth d=m (m shifts), seed the
                // window there, then OR each further depth d=m+1..n. The window union
                // feeds the term AND below.
                //
                // Hazard #1 (m==0 same-clock): with m==0 the first loop runs 0 times,
                // so the window seed is the COMBINATIONAL `cur` (NOT a reg read) — the
                // d=0 (b in a's own clock) completion is present without any OR-back
                // (unlike `AtLeast`, whose latch reads one clock late). Hazard #2
                // (throughout): every shift is `guard_and`-wrapped and the seed was
                // guard-anded, so window element-d carries `guard@[T-d..T]`, identical
                // to fan-out alt-d (the shared chain propagates the guard into every
                // deeper element). Hazard #3 (trailing hop): `cur = win` (the union)
                // feeds the next term, so it samples the whole window, not one delay.
                // Verdict-identical to the fan-out by `reg(x|y) == reg(x)|reg(y)`.
                SeqHop::Range(m, n) => {
                    for _ in 0..m {
                        cur = guard_and(self.seq_delay_reg(cur, pipeline_nbas, sp));
                    }
                    let mut win = cur.clone(); // depth d=m (combinational when m==0)
                    for _ in 0..(n - m) {
                        cur = guard_and(self.seq_delay_reg(cur, pipeline_nbas, sp));
                        win = sva_binary(ast::BinOp::BitOr, win, cur.clone(), sp);
                    }
                    cur = win;
                }
                SeqHop::AtLeast(m) => {
                    // `##[m:$]`: delay `m-1` fixed clocks, then a never-reset
                    // `armed` latch — `armed <= armed | cur`. Reads of `armed`
                    // give the PRIOR-clock value (= "the prefix matched at some
                    // clock ≥ m ago"), so the match stays alive and re-completes
                    // on every later term clock. X-init armed stays don't-know
                    // until the first prefix match (no spurious fire via if(X)).
                    for _ in 0..m.saturating_sub(1) {
                        cur = self.seq_delay_reg(cur, pipeline_nbas, sp);
                    }
                    // This-clock activation (pre-latch). For m==0 the range admits
                    // d=0 — the term in the SAME clock as the prefix — which the
                    // prior-value `armed` read drops; keep it to OR back in below.
                    let cur_now = cur.clone();
                    let armed = self.fresh_sva_reg(1, "arm");
                    let armed_path = ast::HierPath {
                        segments: vec![ast::Ident {
                            name: armed.clone(),
                            span: sp,
                        }],
                        span: sp,
                    };
                    let latch_rhs =
                        sva_binary(ast::BinOp::BitOr, sva_ident_expr(&armed, sp), cur, sp);
                    pipeline_nbas.push(ast::Stmt::NonBlocking {
                        lhs: ast::Lvalue::Ident(armed_path),
                        delay: None,
                        event: None,
                        rhs: latch_rhs,
                        span: sp,
                    });
                    // m≥1: the term arrives strictly after the (m-1)-delayed prefix,
                    // so prior-clock `armed` == "matched ≥ m clocks ago" exactly.
                    // m==0: `##[0:$]` also admits d=0 (term in the prefix's own
                    // clock), which the one-clock-late `armed` omits — OR the
                    // this-clock activation back in so the same-clock completion
                    // fires too (matches IEEE 1800 §16.9.2.1; prior code silently
                    // dropped it). d≥1 is unaffected since `armed` already covers it.
                    cur = if m == 0 {
                        sva_binary(ast::BinOp::BitOr, sva_ident_expr(&armed, sp), cur_now, sp)
                    } else {
                        sva_ident_expr(&armed, sp)
                    };
                }
            }
            // Apply the term to the (post-hop) activation `cur`:
            cur = match term {
                SeqTerm::Bool(e) => sva_binary(
                    ast::BinOp::BitAnd,
                    cur,
                    sva_unary(ast::UnOp::RedOr, e, sp),
                    sp,
                ),
                SeqTerm::Goto(b, n) => self.goto_fsm(cur, b, n, pipeline_nbas, sp),
                SeqTerm::Nonconsec(b, n) => self.nonconsec_fsm(cur, b, n, pipeline_nbas, sp),
                SeqTerm::ConsecAtLeast(b, m) => self.consec_run_fsm(cur, b, m, pipeline_nbas, sp),
                // Handled (advance + passthrough) by the `continue` arm above.
                SeqTerm::Empty => unreachable!("empty term is handled before the hop loop"),
            };
        }
        cur
    }

    /// Register `cur` into a fresh 1-bit `seq` stage (one clock of delay) and
    /// return a read of it (which yields the PRIOR clock's value). Shared by the
    /// fixed-delay shift and the `##[m:$]` latch's `m-1` pre-delay.
    pub(crate) fn seq_delay_reg(
        &mut self,
        cur: ast::Expr,
        pipeline_nbas: &mut Vec<ast::Stmt>,
        sp: ast::Span,
    ) -> ast::Expr {
        let r = self.fresh_sva_reg(1, "seq");
        let r_path = ast::HierPath {
            segments: vec![ast::Ident {
                name: r.clone(),
                span: sp,
            }],
            span: sp,
        };
        pipeline_nbas.push(ast::Stmt::NonBlocking {
            lhs: ast::Lvalue::Ident(r_path),
            delay: None,
            event: None,
            rhs: cur,
            span: sp,
        });
        sva_ident_expr(&r, sp)
    }
}
