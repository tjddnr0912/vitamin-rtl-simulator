//! SVA sequence FSMs — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

impl Elaborator<'_> {
    /// Synthesize the boolean MATCH-THIS-CLOCK expression of a sequence antecedent:
    /// peel a top-level named sequence, route a top `within` to `synth_within`, else
    /// expand into a disjunction of (term-list, throughout-guard) alternatives, build
    /// each one's match signal (a shift-register pipeline for ≥2 terms / a goto/
    /// nonconsec FSM), and OR them. A single plain-boolean alternative returns the raw
    /// expression (flat byte-identical). Shared by the flat assertion path and
    /// `cover property` (SVA-REST) — extracting it changes neither's net allocation.
    pub(crate) fn synth_seq_match(
        &mut self,
        ante: &ast::Sequence,
        regs: &mut SvaRegs,
        pipeline_nbas: &mut Vec<ast::Stmt>,
        sp: ast::Span,
    ) -> ast::Expr {
        // Peel a top-level NAMED-sequence reference to its declared body FIRST, so a
        // named sequence whose body is a top-level `within` reaches synth_within
        // exactly like the literal-antecedent path (review 2026-06-16: named ≠ inline
        // for `within`). Cycle-guarded; byte-identical for a literal antecedent.
        let resolved_ante = self.resolve_named_top(ante);
        if let ast::Sequence::Within { seq1, seq2 } = &resolved_ante {
            // `seq1 within seq2` combines two sub-pipelines — synthesized whole.
            return self.synth_within(seq1, seq2, regs, pipeline_nbas, sp);
        }
        let mut alternatives = self.expand_sequence(&resolved_ante, regs);
        if alternatives.len() > SVA_SEQ_ALT_CAP {
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "an SVA sequence expanded to {} alternatives (cap {}); narrow the bounded ranges",
                    alternatives.len(),
                    SVA_SEQ_ALT_CAP
                ),
            );
            alternatives.truncate(SVA_SEQ_ALT_CAP);
        }
        let match_sigs: Vec<ast::Expr> = alternatives
            .into_iter()
            .map(|(terms, guard)| {
                // Flat byte-identical path: a single PLAIN BOOLEAN term with no
                // throughout guard reproduces the old `ante` exactly. A single goto/
                // nonconsec term still needs the FSM (synth).
                if terms.len() == 1 && guard.is_none() {
                    if let SeqTerm::Bool(_) = terms[0].0 {
                        let (SeqTerm::Bool(e), _) = terms.into_iter().next().unwrap() else {
                            unreachable!()
                        };
                        return e;
                    }
                }
                self.synth_seq_pipeline(terms, guard, pipeline_nbas, sp)
            })
            .collect();
        // OR the alternatives' match signals. A single signal stays raw (flat byte-
        // identical); multiple are each reduced to a boolean before the bitwise OR.
        if match_sigs.len() == 1 {
            match_sigs.into_iter().next().unwrap()
        } else {
            let mut it = match_sigs.into_iter();
            let mut acc = sva_unary(ast::UnOp::RedOr, it.next().unwrap(), sp);
            for m in it {
                let mb = sva_unary(ast::UnOp::RedOr, m, sp);
                acc = sva_binary(ast::BinOp::BitOr, acc, mb, sp);
            }
            acc
        }
    }

    /// Goto repetition `b[->n]` (slice S8). `act` is the (this-clock) activation:
    /// a counting thread starts wherever `act` is true. Existence-latch FSM with
    /// `n` 1-bit regs `reg_0..reg_{n-1}` (`reg_s` = "∃ thread that has seen `s`
    /// b's, pending the next"); a `b` advances every stage. Returns the
    /// match-this-clock signal `b & avail_{n-1}` (the n-th b). Exact for the
    /// `|->` any-completion semantics. Pure IR-0.
    pub(crate) fn goto_fsm(
        &mut self,
        act: ast::Expr,
        b: ast::Expr,
        n: u32,
        nbas: &mut Vec<ast::Stmt>,
        sp: ast::Span,
    ) -> ast::Expr {
        let n = n.max(1) as usize;
        let regs: Vec<String> = (0..n).map(|_| self.fresh_sva_reg(1, "gto")).collect();
        let bb = || sva_unary(ast::UnOp::RedOr, b.clone(), sp); // |b
        let nbb = || sva_unary(ast::UnOp::BitNot, bb(), sp); // ~|b
                                                             // avail_s = reg_s, except avail_0 also admits a freshly-activated thread.
        let avail = |s: usize| -> ast::Expr {
            if s == 0 {
                sva_binary(
                    ast::BinOp::BitOr,
                    sva_ident_expr(&regs[0], sp),
                    act.clone(),
                    sp,
                )
            } else {
                sva_ident_expr(&regs[s], sp)
            }
        };
        // reg_0 <= avail_0 & ~b  (seen-0 threads persist while no b)
        nbas.push(sva_nb(
            &regs[0],
            sva_binary(ast::BinOp::BitAnd, avail(0), nbb(), sp),
            sp,
        ));
        // reg_s <= (b & avail_{s-1}) | (~b & reg_s)  for s = 1..n-1
        #[allow(clippy::needless_range_loop)] // s indexes regs AND feeds avail(s-1)
        for s in 1..n {
            let adv = sva_binary(ast::BinOp::BitAnd, bb(), avail(s - 1), sp);
            let stay = sva_binary(ast::BinOp::BitAnd, nbb(), sva_ident_expr(&regs[s], sp), sp);
            nbas.push(sva_nb(
                &regs[s],
                sva_binary(ast::BinOp::BitOr, adv, stay, sp),
                sp,
            ));
        }
        // match = b & avail_{n-1}
        sva_binary(ast::BinOp::BitAnd, bb(), avail(n - 1), sp)
    }

    /// Nonconsecutive repetition `b[=n]` (slice S8) = goto to the n-th b, then an
    /// `ext` latch that keeps the match alive on subsequent non-b clocks (a
    /// further b would be the (n+1)-th and breaks it). Output this clock =
    /// `match_g | (ext & ~b)`; the latch holds exactly that. Pure IR-0.
    pub(crate) fn nonconsec_fsm(
        &mut self,
        act: ast::Expr,
        b: ast::Expr,
        n: u32,
        nbas: &mut Vec<ast::Stmt>,
        sp: ast::Span,
    ) -> ast::Expr {
        let match_g = self.goto_fsm(act, b.clone(), n, nbas, sp);
        let ext = self.fresh_sva_reg(1, "ncx");
        let nbb = sva_unary(ast::UnOp::BitNot, sva_unary(ast::UnOp::RedOr, b, sp), sp);
        let ext_alive = sva_binary(ast::BinOp::BitAnd, sva_ident_expr(&ext, sp), nbb, sp);
        // cur = match_g | (ext & ~b); ext <= cur (the same expression).
        let cur = sva_binary(ast::BinOp::BitOr, match_g, ext_alive, sp);
        nbas.push(sva_nb(&ext, cur.clone(), sp));
        cur
    }

    /// Unbounded consecutive repetition `b[*m:$]` (slice S13). `act` is the
    /// (this-clock) activation: a run may START wherever `act` is true. A gated
    /// run-latch with `m` 1-bit regs `c_1..c_m`, where `c_k` = "an alive thread
    /// (started at a valid activation) has now seen `k` consecutive `b`'s":
    ///   c_1 = act & |b                                  (a run begins)
    ///   c_k = reg(c_{k-1}) & |b              for 1<k<m  (the run advances)
    ///   c_m = (reg(c_{m-1}) | reg(c_m)) & |b            (advance OR self-latch ≥m)
    /// (`m == 1` collapses to the single self-latch `c_1 = (act|reg(c_1)) & |b`).
    /// Returns the match-this-clock signal `c_m` ("run ≥ m ends now"), exact for
    /// the `|->` any-completion semantics. A reg read yields the PRIOR clock's
    /// value (the checker's if-check runs before the NBAs), so the chain advances
    /// one count per clock; a non-`b` clock zeroes `c_1` and (one clock later)
    /// collapses the chain. X-init regs stay don't-know until the first real run
    /// (lenient: `if(X)` never fires). Pure IR-0.
    pub(crate) fn consec_run_fsm(
        &mut self,
        act: ast::Expr,
        b: ast::Expr,
        m: u32,
        nbas: &mut Vec<ast::Stmt>,
        sp: ast::Span,
    ) -> ast::Expr {
        let m = m.max(1) as usize;
        let regs: Vec<String> = (0..m).map(|_| self.fresh_sva_reg(1, "crl")).collect();
        let bb = || sva_unary(ast::UnOp::RedOr, b.clone(), sp); // |b
                                                                // c_1 (also the self-latch when m == 1).
        let c1 = if m == 1 {
            let or = sva_binary(ast::BinOp::BitOr, act, sva_ident_expr(&regs[0], sp), sp);
            sva_binary(ast::BinOp::BitAnd, or, bb(), sp)
        } else {
            sva_binary(ast::BinOp::BitAnd, act, bb(), sp)
        };
        nbas.push(sva_nb(&regs[0], c1.clone(), sp));
        let mut last = c1;
        for k in 2..=m {
            // reg(c_{k-1}) — the prior-clock count-(k-1) state.
            let prior_prev = sva_ident_expr(&regs[k - 2], sp);
            let ck = if k < m {
                sva_binary(ast::BinOp::BitAnd, prior_prev, bb(), sp)
            } else {
                // top reg saturates at ≥ m: (reg(c_{m-1}) | reg(c_m)) & |b.
                let or = sva_binary(
                    ast::BinOp::BitOr,
                    prior_prev,
                    sva_ident_expr(&regs[k - 1], sp),
                    sp,
                );
                sva_binary(ast::BinOp::BitAnd, or, bb(), sp)
            };
            nbas.push(sva_nb(&regs[k - 1], ck.clone(), sp));
            last = ck;
        }
        last
    }
}
