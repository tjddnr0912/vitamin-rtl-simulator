//! SVA properties — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

/// during statement lowering and materialized AFTER the module's process loop as
/// a synthesized clocked checker (v8 SVA subset). It is a continuously-checking
/// background process, not a one-shot procedural statement, so it cannot be
/// lowered inline — see `materialize_sva_checkers`.
pub(crate) struct PendingSva {
    pub(crate) clock: ast::Sensitivity,
    /// `disable iff (expr)` reset condition (slice S12), if any.
    pub(crate) disable_iff: Option<ast::Expr>,
    pub(crate) ante: ast::Sequence,
    pub(crate) kind: ast::ImplicationKind,
    /// The consequent (slice S14: a `Sequence`; a boolean consequent is
    /// `Sequence::Boolean` and keeps the byte-identical lowering).
    pub(crate) cons: ast::Sequence,
    /// Action block (slice S11): `fail` (the `else` statement) replaces the
    /// default `$error` on a violation; `pass` runs on a non-vacuous success.
    pub(crate) pass: Option<Box<ast::Stmt>>,
    pub(crate) fail: Option<Box<ast::Stmt>>,
    /// Consequent clocking event (slice A3, multi-clock): `Some(c2)` selects the
    /// two-process handoff synthesis for `@(c1) ante |=> @(c2) cons`. Out-of-band
    /// (elaborate-internal, NOT serialized → golden-free).
    pub(crate) cons_clock: Option<ast::Sensitivity>,
    /// Property-expression tree (slice N2d): `Some(_)` selects `synth_prop_expr`
    /// (the property-level `and`/`or` / recursion reduction) instead of the flat
    /// implication path. When set, `ante/kind/cons` hold inert placeholders.
    pub(crate) prop_expr: Option<ast::PropExpr>,
    /// The own name of the property being synthesized, when this `PendingSva` came
    /// from splicing a named `property NAME` whose body has a `prop_expr`. Used to
    /// recognise the legal tail-`|=>` self-reference (recursion) during synthesis.
    /// `None` for an inline `assert property(...)` (no name ⇒ no recursion site).
    pub(crate) prop_self_name: Option<String>,
    /// Sequence/property LOCAL VARIABLE declarations (slice N2c, IEEE §16.10). Empty
    /// (the common case) keeps the byte-identical lowering; a non-empty list routes
    /// the assertion to `synth_local_var_assert` (the data-tracking shift register).
    pub(crate) local_vars: Vec<ast::SvaLocalDecl>,
    pub(crate) span: ast::Span,
}

/// Collected sampled-value state for one concurrent assertion: each distinct
/// sampled signal gets ONE prev-register (shared across `$past`/`$rose`/etc.),
/// plus the per-clock `prev <= signal` NBA updates that maintain them.
#[derive(Default)]
pub(crate) struct SvaRegs {
    /// signal name → prev-register name (dedup so `$rose(a)` + `$stable(a)` share).
    pub(crate) by_signal: Vec<(String, String)>,
    /// `prev <= signal;` NBA updates appended to the checker's clocked body.
    pub(crate) nbas: Vec<ast::Stmt>,
}

/// A flattened FIXED-DELAY antecedent term carrying sequence local-variable
/// captures (slice N2c): the boolean term, the `##d` hop BEFORE it (0 for the first
/// term), and the `(name, expr)` captures triggered when the term matches. Built by
/// `flatten_lv_antecedent`, consumed by `synth_local_var_assert`.
pub(crate) struct FlatLvTerm {
    pub(crate) term: ast::Expr,
    pub(crate) hop: u32,
    pub(crate) captures: Vec<(ast::Ident, ast::Expr)>,
}

/// Peel a TOP-LEVEL `always` wrapper from a property expression (recursively):
/// `always p` at the top of a per-clock-re-attempted assertion is exactly `p` (the
/// re-attempt supplies the "every clock"), and `always s_eventually p` (recurrent
/// liveness) is `s_eventually p`. A NESTED `always` is left in place (loud-rejected).
pub(crate) fn peel_top_always(pe: ast::PropExpr) -> ast::PropExpr {
    match pe {
        ast::PropExpr::Always(inner) => peel_top_always(*inner),
        other => other,
    }
}

impl Elaborator<'_> {
    /// Property-references-property (slice A4): if the consequent is a bare named
    /// OVERLAP property `q`, replace it with the boolean `!q.ante || q.cons` (the
    /// single-tick meaning of `q`'s `b |-> c`). A no-op for any other consequent
    /// (literal / sequence / net / non-property name) — byte-identical. A guard
    /// violation leaves a benign `1'b1` consequent (the loud diagnostic already gates
    /// the run), so there is no spurious fire on top of the error.
    pub(crate) fn flatten_prop_consequent(&mut self, sva: &mut PendingSva) {
        let sp = sva.span;
        let name = match &sva.cons {
            ast::Sequence::Boolean(e) => match &e.kind {
                ast::ExprKind::Ident(p) if p.segments.len() == 1 => p.segments[0].name.clone(),
                _ => return,
            },
            _ => return,
        };
        // A real net of the same name wins the leaf path (preserves byte-identity for
        // a design with a net named like a property); a non-property name is left for
        // the ordinary lowering (a loud undeclared-net if neither).
        if self.lookup_net_scoped(&name).is_some() || !self.prop_table.contains_key(&name) {
            return;
        }
        // Inner NON-OVERLAP property reference (slice SVA-R2): `a |-> (b |=> c)`
        // ≡ `(a && b) |=> c` — the obligation spans a clock, so (unlike A4's
        // overlap `!b || c`) it cannot collapse to a single-tick boolean. Only the
        // canonical shape folds: an OVERLAP outer with a BOOLEAN outer antecedent
        // `a`. Rewriting the top-level `sva` to `(a && b) |=> c` (kind=NonOverlap)
        // hands the 1-cycle skew to the existing top-level `|=>` pend-reg machinery
        // below. Everything else (a 2-cycle skew from an outer `|=>`, a sequence
        // outer antecedent, an inner property whose own sides are property refs)
        // falls through to the overlap flattener's loud `|=>`-as-consequent reject.
        if matches!(sva.kind, ast::ImplicationKind::Overlap) {
            match &sva.ante {
                // BOOLEAN outer antecedent (fast path — kept verbatim for
                // byte-identity): `a |-> (b |=> c)` ≡ `(a && b) |=> c`. a and b are
                // fused at the same clock = a single boolean conjunction.
                ast::Sequence::Boolean(a) => {
                    let a = a.clone();
                    if let Some((b, c)) = self.peel_nonoverlap_property(&name, &sva.clock) {
                        sva.ante = ast::Sequence::Boolean(sva_binary(ast::BinOp::LogAnd, a, b, sp));
                        sva.cons = ast::Sequence::Boolean(c);
                        sva.kind = ast::ImplicationKind::NonOverlap;
                        return;
                    }
                    // Slice #6 deeper homogeneous chain: `a |-> (b |=> c |=> … |=> z)`
                    // ≡ `((a && b) ##1 rest) |=> z` — the overlap `|->` fuses a and the
                    // first link's antecedent b at the SAME clock (`a && b`), then each
                    // remaining `|=>` adds one `##1` into the antecedent sequence (rest =
                    // `c ##1 …`), terminating at the top-level `|=>` pend reg.
                    if let Some((b, rest, z)) = self.peel_nonoverlap_chain(&name, &sva.clock) {
                        sva.ante = ast::Sequence::Delay {
                            min: 1,
                            max: Some(1),
                            lhs: Box::new(ast::Sequence::Boolean(sva_binary(
                                ast::BinOp::LogAnd,
                                a,
                                b,
                                sp,
                            ))),
                            rhs: Box::new(rest),
                        };
                        sva.cons = ast::Sequence::Boolean(z);
                        sva.kind = ast::ImplicationKind::NonOverlap;
                        return;
                    }
                }
                // SEQUENCE outer antecedent (slice A.3): `seq |-> (b |=> c)`
                // ≡ `(seq ##0 b) |=> c` (IEEE 1800 §16.12). The overlap `|->` fuses
                // b onto the END of `seq` at the SAME clock (the `##0` connector); the
                // inner `|=>` then skews b→c by one clock, supplied by the top-level
                // `|=>` pend reg. So rewrite the antecedent to the SEQUENCE
                // `seq ##0 b` (kind→NonOverlap) and let the existing sequence pipeline
                // + pend-reg machinery produce the obligation — no new synthesis.
                orig_ante => {
                    let orig_ante = orig_ante.clone();
                    if let Some((b, c)) = self.peel_nonoverlap_property(&name, &sva.clock) {
                        sva.ante = ast::Sequence::Delay {
                            min: 0,
                            max: Some(0),
                            lhs: Box::new(orig_ante),
                            rhs: Box::new(ast::Sequence::Boolean(b)),
                        };
                        sva.cons = ast::Sequence::Boolean(c);
                        sva.kind = ast::ImplicationKind::NonOverlap;
                        return;
                    }
                    // Slice #6 deeper homogeneous chain: `seq |-> (b |=> c |=> … |=> z)`
                    // ≡ `(seq ##0 b ##1 rest) |=> z` — the overlap `|->` fuses b onto the
                    // END of `seq` at the SAME clock (`##0`), then each remaining `|=>`
                    // adds one `##1` (rest = `c ##1 …`), to the top-level pend reg.
                    if let Some((b, rest, z)) = self.peel_nonoverlap_chain(&name, &sva.clock) {
                        let seq_b = ast::Sequence::Delay {
                            min: 0,
                            max: Some(0),
                            lhs: Box::new(orig_ante),
                            rhs: Box::new(ast::Sequence::Boolean(b)),
                        };
                        sva.ante = ast::Sequence::Delay {
                            min: 1,
                            max: Some(1),
                            lhs: Box::new(seq_b),
                            rhs: Box::new(rest),
                        };
                        sva.cons = ast::Sequence::Boolean(z);
                        sva.kind = ast::ImplicationKind::NonOverlap;
                        return;
                    }
                }
            }
        }
        // Slice N2b: genuine 2-cycle skew — an outer NON-OVERLAP `a |=> q` whose
        // referenced property is a clean inner NON-OVERLAP `q: b |=> c`.
        // `a |=> (b |=> c)` ≡ `(a ##1 b) |=> c` (IEEE 1800 §16.12 textual
        // substitution): the outer `|=>` skews a→b by one clock — exactly the `##1`
        // connector — and the inner `|=>` skews b→c by one more, which the top-level
        // `|=>` pend reg already supplies. So rewrite the antecedent to the SEQUENCE
        // `a ##1 b` (kept NonOverlap) and let the existing sequence pipeline +
        // pend-reg machinery produce BOTH skews — no new synthesis. (Unlike the
        // OVERLAP outer above, where a and b are fused at the same clock = `a && b`.)
        // Deeper chains (inner consequent is itself a property ref) and a sequence
        // outer antecedent fall through to the overlap flattener's loud `|=>` reject.
        if matches!(sva.kind, ast::ImplicationKind::NonOverlap) {
            // Both the boolean and sequence outer-antecedent cases produce the SAME
            // rewrite `(orig_ante ##1 b) |=> c`; the only difference is whether
            // `orig_ante` is wrapped `Boolean(a)` (fast path, kept verbatim for
            // byte-identity) or an already-built sequence (slice A.3:
            // `seq |=> (b |=> c)` ≡ `(seq ##1 b) |=> c`, §16.12). The outer `|=>`
            // skews orig_ante→b by one clock (`##1`); the inner `|=>` skews b→c by one
            // more, supplied by the top-level pend reg (a total 2-clock obligation).
            let orig_ante = match &sva.ante {
                ast::Sequence::Boolean(a) => ast::Sequence::Boolean(a.clone()),
                other => other.clone(),
            };
            if let Some((b, c)) = self.peel_nonoverlap_property(&name, &sva.clock) {
                sva.ante = ast::Sequence::Delay {
                    min: 1,
                    max: Some(1),
                    lhs: Box::new(orig_ante),
                    rhs: Box::new(ast::Sequence::Boolean(b)),
                };
                sva.cons = ast::Sequence::Boolean(c);
                return;
            }
            // Slice #6 deeper homogeneous chain: `a |=> (b |=> c |=> … |=> z)`
            // ≡ `(a ##1 b ##1 rest) |=> z` — the outer `|=>` skews a→b by one clock
            // (`##1`) and each remaining `|=>` adds one more `##1` (rest = `c ##1 …`),
            // terminating at the top-level `|=>` pend reg. The same rewrite serves a
            // boolean or sequence outer antecedent (only the leading term differs).
            if let Some((b, rest, z)) = self.peel_nonoverlap_chain(&name, &sva.clock) {
                let ante_b = ast::Sequence::Delay {
                    min: 1,
                    max: Some(1),
                    lhs: Box::new(orig_ante),
                    rhs: Box::new(ast::Sequence::Boolean(b)),
                };
                sva.ante = ast::Sequence::Delay {
                    min: 1,
                    max: Some(1),
                    lhs: Box::new(ante_b),
                    rhs: Box::new(rest),
                };
                sva.cons = ast::Sequence::Boolean(z);
                return;
            }
        }
        match self.flatten_overlap_property(&name, &sva.clock, sp) {
            Some(b) => sva.cons = ast::Sequence::Boolean(b),
            None => sva.cons = ast::Sequence::Boolean(sva_one(sp)),
        }
    }

    /// Non-emitting probe (slice SVA-R2): returns `(inner_ante, inner_cons)` iff
    /// `name` is a clean single-clock NON-OVERLAP property `b |=> c` whose
    /// antecedent and consequent are both plain booleans (NOT themselves property
    /// references), with no formals / `disable iff` / consequent clock and the SAME
    /// single bare-ident clock as the outer assertion. Returns `None` silently for
    /// any other shape, so the caller falls through to the loud overlap flattener.
    pub(crate) fn peel_nonoverlap_property(
        &self,
        name: &str,
        clock: &ast::Sensitivity,
    ) -> Option<(ast::Expr, ast::Expr)> {
        let pd = self.prop_table.get(name)?;
        if !matches!(pd.implication_kind, ast::ImplicationKind::NonOverlap)
            || !pd.formals.is_empty()
            || pd.disable_iff.is_some()
            || pd.consequent_clock.is_some()
        {
            return None;
        }
        let outer = sva_clock_signal(clock);
        if outer.is_none() || outer != sva_clock_signal(&pd.clock) {
            return None;
        }
        let (ast::Sequence::Boolean(b), ast::Sequence::Boolean(c)) =
            (&pd.antecedent, &pd.consequent)
        else {
            return None;
        };
        // A bare property-name on either side is a deeper (nested) skew beyond this
        // slice → `None` → loud fallthrough.
        if self.is_property_name(b) || self.is_property_name(c) {
            return None;
        }
        Some((b.clone(), c.clone()))
    }

    /// Slice #6: a DEEPER homogeneous non-overlap chain whose inner consequent is
    /// itself a property reference (`q1: b |=> q2`, `q2: c |=> q3`, … terminating in
    /// a boolean). Returns `(b, rest, z)` where `b` is `q1`'s antecedent boolean,
    /// `rest` is the SEQUENCE `c ##1 d ##1 … ##1 y` (every later property's antecedent
    /// joined by exactly one `##1` per `|=>`, IEEE 1800-2017 §16.12 textual
    /// substitution), and `z` is the final consequent boolean. The full inner
    /// antecedent is `b ##1 rest`; each caller prepends its own connector to `b`.
    ///
    /// Returns `None` (silently → caller falls through to the loud overlap flattener /
    /// E3009) for ANY non-deeper or unsupported shape, so the strict 2-deep fast paths
    /// — which the callers try FIRST and which are kept verbatim — own those cases
    /// byte-identically. A `None` is returned for: a 2-deep leaf (`q1`'s consequent is
    /// boolean), a non-boolean antecedent anywhere, a sequence/mixed consequent, a
    /// non-overlap-mixing inner, formals / `disable iff` / consequent-clock /
    /// `prop_expr` / local-vars on any link, a clock mismatch on any link, or a cyclic
    /// reference (caught LOUD via `sva_inline_stack` so it rejects, not hangs).
    pub(crate) fn peel_nonoverlap_chain(
        &mut self,
        name: &str,
        clock: &ast::Sensitivity,
    ) -> Option<(ast::Expr, ast::Sequence, ast::Expr)> {
        // q1 itself must be a clean non-overlap link whose antecedent is boolean.
        let pd = self.prop_table.get(name)?.clone();
        let b = self.chain_link_antecedent(&pd, clock)?;
        // q1's consequent MUST be a bare property name for a DEEPER chain. A boolean
        // consequent is the 2-deep case — already handled byte-identically by the
        // fast path; return None so this never disturbs it.
        let inner = self.chain_consequent_property(&pd)?;
        // The full inner antecedent of the rest of the chain (`c ##1 … ##1 y`) and the
        // final boolean consequent, recursing under the cycle guard.
        self.sva_inline_stack.push(name.to_string());
        let rest = self.peel_chain_full(&inner, clock);
        self.sva_inline_stack.pop();
        let (rest_seq, z) = rest?;
        Some((b, rest_seq, z))
    }

    /// The recursive worker for [`Self::peel_nonoverlap_chain`]: returns the FULL inner
    /// antecedent sequence of property `name` plus its final boolean consequent. A leaf
    /// link (`q: y |=> z`, `z` boolean) returns `(Boolean(y), z)`; a deeper link
    /// (`q: y |=> q_next`) returns `(y ##1 <full inner antecedent of q_next>, z)` —
    /// exactly one `##1` per `|=>`. Cycle-guarded via `sva_inline_stack` (a self- or
    /// mutually-recursive property emits the loud `recursive property` diagnostic and
    /// returns `None`, so it rejects rather than hangs).
    pub(crate) fn peel_chain_full(
        &mut self,
        name: &str,
        clock: &ast::Sensitivity,
    ) -> Option<(ast::Sequence, ast::Expr)> {
        if self.sva_inline_stack.iter().any(|n| n == name) {
            self.error(
                MsgCode::ElabUnsupported,
                &format!("recursive property `{name}` is illegal (IEEE 1800 §16.12)"),
            );
            return None;
        }
        let pd = self.prop_table.get(name)?.clone();
        let y = self.chain_link_antecedent(&pd, clock)?;
        // Boolean consequent ⇒ leaf: the full inner antecedent is just `y`.
        if let ast::Sequence::Boolean(z) = &pd.consequent {
            if !self.is_property_name(z) {
                return Some((ast::Sequence::Boolean(y), z.clone()));
            }
        }
        // Property-named consequent ⇒ recurse, prepending `y ##1` (one clock per `|=>`).
        let inner = self.chain_consequent_property(&pd)?;
        self.sva_inline_stack.push(name.to_string());
        let deeper = self.peel_chain_full(&inner, clock);
        self.sva_inline_stack.pop();
        let (deeper_seq, z) = deeper?;
        Some((
            ast::Sequence::Delay {
                min: 1,
                max: Some(1),
                lhs: Box::new(ast::Sequence::Boolean(y)),
                rhs: Box::new(deeper_seq),
            },
            z,
        ))
    }

    /// Validate one chain link `pd` is a clean single-clock NON-OVERLAP property with a
    /// BOOLEAN antecedent and no formals / `disable iff` / consequent-clock /
    /// `prop_expr` / local-vars, and the SAME bare-ident clock as the outer assertion.
    /// Returns the antecedent boolean expression, or `None` for any disqualifying shape
    /// (the caller then falls through to the loud path). RED-preserving: every guard the
    /// 2-deep peel enforces is re-checked on every deeper link.
    pub(crate) fn chain_link_antecedent(
        &self,
        pd: &ast::PropDecl,
        clock: &ast::Sensitivity,
    ) -> Option<ast::Expr> {
        if !matches!(pd.implication_kind, ast::ImplicationKind::NonOverlap)
            || !pd.formals.is_empty()
            || pd.disable_iff.is_some()
            || pd.consequent_clock.is_some()
            || pd.prop_expr.is_some()
            || !pd.local_vars.is_empty()
        {
            return None;
        }
        let outer = sva_clock_signal(clock);
        if outer.is_none() || outer != sva_clock_signal(&pd.clock) {
            return None;
        }
        let ast::Sequence::Boolean(ante) = &pd.antecedent else {
            return None;
        };
        // A property-named antecedent is a nested shape we do not model here → loud.
        if self.is_property_name(ante) {
            return None;
        }
        Some(ante.clone())
    }

    /// If `pd`'s consequent is a bare property-name reference, return that name; else
    /// `None` (a boolean / sequence / non-property consequent — not a deeper link).
    pub(crate) fn chain_consequent_property(&self, pd: &ast::PropDecl) -> Option<String> {
        if let ast::Sequence::Boolean(e) = &pd.consequent {
            if let ast::ExprKind::Ident(p) = &e.kind {
                if p.segments.len() == 1 {
                    let n = &p.segments[0].name;
                    if self.lookup_net_scoped(n).is_none() && self.prop_table.contains_key(n) {
                        return Some(n.clone());
                    }
                }
            }
        }
        None
    }

    /// Flatten a named OVERLAP property `name` to the boolean `!ante || cons` (slice
    /// A4) — the single-tick meaning of `b |-> c`. Recurses when `cons` is itself a
    /// bare overlap-property reference (cycle-guarded via `sva_inline_stack`).
    /// Returns `None` (after emitting a loud diagnostic) for any unsupported inner
    /// form: a different clock (multi-clock), `disable iff`, a consequent clock,
    /// formal arguments, a non-overlap `|=>`, a non-boolean antecedent/consequent, or
    /// recursion.
    pub(crate) fn flatten_overlap_property(
        &mut self,
        name: &str,
        clock: &ast::Sensitivity,
        sp: ast::Span,
    ) -> Option<ast::Expr> {
        if self.sva_inline_stack.iter().any(|n| n == name) {
            self.error(
                MsgCode::ElabUnsupported,
                &format!("recursive property `{name}` is illegal (IEEE 1800 §16.12)"),
            );
            return None;
        }
        let pd = self.prop_table.get(name).cloned()?;
        // A named property whose body is a property-LEVEL operator tree
        // (`always`/`not`/`s_eventually`/`nexttime`/…) is stored as `prop_expr` with
        // INERT placeholder flat fields (antecedent/consequent = `1'b1`, kind =
        // Overlap). Without this guard the overlap fold would compute `!1 || 1 = 1`
        // and SILENTLY replace the real obligation with constant-true (a dropped
        // assertion — found by the #6 adversarial review). The same form used
        // standalone is already loud (`always p` as a nested consequent is
        // unsupported), so correct-or-loud demands a loud reject here too. (The
        // sequence/chain peelers already guard on `prop_expr`; this is the
        // pre-existing hole in the overlap flattener.)
        if pd.prop_expr.is_some() {
            self.error(
                MsgCode::ElabUnsupported,
                "a named property with a property-level operator \
                 (`always`/`not`/`s_eventually`/`nexttime`/…) used as a consequent \
                 is unsupported in this subset",
            );
            return None;
        }
        if !pd.formals.is_empty() {
            self.error(
                MsgCode::ElabUnsupported,
                "a parameterized property as a consequent is unsupported in this subset",
            );
            return None;
        }
        // Span-insensitive clock match: both must be the SAME single bare-ident edge.
        let outer = sva_clock_signal(clock);
        if pd.consequent_clock.is_some() || outer.is_none() || outer != sva_clock_signal(&pd.clock)
        {
            self.error(
                MsgCode::ElabUnsupported,
                "a named property consequent with a different / multi-clock clocking \
                 event is unsupported in this subset",
            );
            return None;
        }
        if pd.disable_iff.is_some() {
            self.error(
                MsgCode::ElabUnsupported,
                "a named property consequent with its own `disable iff` is unsupported \
                 in this subset",
            );
            return None;
        }
        if !matches!(pd.implication_kind, ast::ImplicationKind::Overlap) {
            self.error(
                MsgCode::ElabUnsupported,
                "a named `|=>` property used as a consequent is unsupported (overlap \
                 `|->` only in this subset)",
            );
            return None;
        }
        let ast::Sequence::Boolean(ante_e) = &pd.antecedent else {
            self.error(
                MsgCode::ElabUnsupported,
                "a named property consequent with a sequence antecedent is unsupported \
                 in this subset",
            );
            return None;
        };
        let ante_e = ante_e.clone();
        self.sva_inline_stack.push(name.to_string());
        // The inner consequent: a boolean leaf, or itself a bare overlap-property ref.
        let cons_b = match &pd.consequent {
            ast::Sequence::Boolean(e) => match &e.kind {
                ast::ExprKind::Ident(p)
                    if p.segments.len() == 1
                        && self.lookup_net_scoped(&p.segments[0].name).is_none()
                        && self.prop_table.contains_key(&p.segments[0].name) =>
                {
                    let inner = p.segments[0].name.clone();
                    self.flatten_overlap_property(&inner, clock, sp)
                }
                _ => Some(e.clone()),
            },
            _ => {
                self.error(
                    MsgCode::ElabUnsupported,
                    "a named property consequent with a sequence consequent is \
                     unsupported in this subset",
                );
                None
            }
        };
        self.sva_inline_stack.pop();
        let cons_b = cons_b?;
        // `b |-> c` at one tick ≡ `!b || c`.
        Some(sva_binary(
            ast::BinOp::LogOr,
            sva_unary(ast::UnOp::LogNot, ante_e, sp),
            cons_b,
            sp,
        ))
    }

    /// Flatten a fixed-delay local-variable antecedent into an ordered list of
    /// (boolean-term, hop-before-term, captures). Returns `false` (after a loud
    /// diagnostic) for any out-of-subset form: a ranged / unbounded delay, a
    /// repetition, goto/nonconsec, within/throughout, a nested re-clock, or a capture
    /// under a repetition. The first term's hop is 0.
    pub(crate) fn flatten_lv_antecedent(
        &mut self,
        seq: &ast::Sequence,
        out: &mut Vec<FlatLvTerm>,
    ) -> bool {
        match seq {
            ast::Sequence::Boolean(e) => {
                out.push(FlatLvTerm {
                    term: e.clone(),
                    hop: if out.is_empty() { 0 } else { 1 },
                    captures: Vec::new(),
                });
                true
            }
            ast::Sequence::MatchItem { seq, assigns } => {
                // The match-item's inner sequence must be a plain boolean (a capture on
                // a multi-term / repeated sub-sequence is out of subset).
                let ast::Sequence::Boolean(e) = &**seq else {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "a local-variable capture requires a boolean term `(b, x = e)` \
                         in this subset",
                    );
                    return false;
                };
                out.push(FlatLvTerm {
                    term: e.clone(),
                    hop: if out.is_empty() { 0 } else { 1 },
                    captures: assigns.clone(),
                });
                true
            }
            ast::Sequence::Delay { min, max, lhs, rhs } => {
                // FIXED delay only: `##d` (min == max). A RANGE `##[m:n]` / unbounded
                // `##[m:$]` lets two attempts CONVERGE on one completion stage (a data
                // collision) → loud, the cardinal correctness boundary of this slice.
                if *max != Some(*min) {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "a RANGED delay (`##[m:n]` / `##[m:$]`) in a sequence carrying a \
                         local variable is unsupported (two attempts converge on one \
                         data stage — a collision) in this subset",
                    );
                    return false;
                }
                if !self.flatten_lv_antecedent(lhs, out) {
                    return false;
                }
                // The first term of `rhs` is reached via the `##min` hop. Flatten rhs
                // into a temp, set its first term's hop to `min`, and append.
                let mark = out.len();
                if !self.flatten_lv_antecedent(rhs, out) {
                    return false;
                }
                if let Some(first) = out.get_mut(mark) {
                    first.hop = *min;
                }
                true
            }
            // Every other form (Repeat / Throughout / Within / Clocked / Instance)
            // carries either a convergence hazard (ranges/repetition), a multi-stage
            // guard, or no oracle — loud (never a silent capture drop).
            _ => {
                self.error(
                    MsgCode::ElabUnsupported,
                    "a sequence local variable is only supported on a FIXED-DELAY \
                     boolean sequence (no repetition / range / goto / within / \
                     throughout / re-clock) in this subset",
                );
                false
            }
        }
    }

    /// Synthesize a clocked checker for a property-level `and`/`or` / recursive
    /// property (slice N2d). Reduces the `PropExpr` tree to a SINGLE per-clock
    /// boolean VIOLATION expression (`prop_expr_violation`) and emits
    /// `always @(clk) if (violation) <fail>; <pend/prev NBAs>` — pure IR-0 (no
    /// sim-ir change). Out-of-subset feature combinations (a consequent clock,
    /// `disable iff`, a pass action) are loud-rejected; per-operand restrictions
    /// (boolean-only operands, legal recursion sites) are enforced in the reduction.
    pub(crate) fn synth_prop_expr(&mut self, sva: PendingSva, sp: ast::Span) {
        let Some(pe) = sva.prop_expr.clone() else {
            return; // dispatched only when Some
        };
        // A top-level `always p` is exactly `p` under per-clock re-attempt — peel it
        // (recursively) so the per-clock reducer sees `p`. A nested `always` stays in
        // `pe` and is loud-rejected by `prop_expr_violation`'s `Always` arm.
        let pe = peel_top_always(pe);
        if sva.cons_clock.is_some() {
            self.error(
                MsgCode::ElabUnsupported,
                "a multi-clock consequent combined with a property-level `and`/`or` \
                 is unsupported in this subset",
            );
            return;
        }
        if sva.disable_iff.is_some() {
            self.error(
                MsgCode::ElabUnsupported,
                "`disable iff` combined with a property-level `and`/`or` is \
                 unsupported in this subset",
            );
            return;
        }
        if sva.pass.is_some() {
            self.error(
                MsgCode::ElabUnsupported,
                "a pass action combined with a property-level `and`/`or` is \
                 unsupported in this subset",
            );
            return;
        }
        let self_name = sva.prop_self_name.clone();
        let mut regs = SvaRegs::default();
        let mut pend_nbas: Vec<ast::Stmt> = Vec::new();
        // The returned top-level skew only shifts WHEN a verdict is reported
        // (verdict-safe — the attempt-aligned operands are enforced internally).
        let Some((violation, _skew)) =
            self.prop_expr_violation(&pe, self_name.as_deref(), &mut regs, &mut pend_nbas, 0, sp)
        else {
            return; // a loud diagnostic was already emitted
        };
        // Fail action: the call-site `else` statement, or the default `$error`.
        let fail_stmt_raw = match sva.fail {
            Some(s) => *s,
            None => ast::Stmt::SysTaskCall {
                name: ast::Ident {
                    name: "$error".to_string(),
                    span: sp,
                },
                args: vec![ast::Expr {
                    kind: ast::ExprKind::StrLit {
                        raw: "\"Assertion property violation\"".to_string(),
                    },
                    span: sp,
                }],
                span: sp,
            },
        };
        let fail_stmt = self.rewrite_sampled_stmt(&fail_stmt_raw, &mut regs);
        let if_fail = ast::Stmt::If {
            cond: violation,
            then_s: Box::new(fail_stmt),
            else_s: None,
            span: sp,
        };
        // Check FIRST (reads the prior clock's pend/prev regs), then the NBA updates
        // apply in the NBA region for the next clock.
        let mut stmts = vec![if_fail];
        stmts.extend(regs.nbas);
        stmts.extend(pend_nbas);
        let body = if stmts.len() == 1 {
            stmts.pop().unwrap()
        } else {
            ast::Stmt::Block {
                label: None,
                decls: Vec::new(),
                stmts,
                span: sp,
            }
        };
        let pb = ast::ProceduralBlock {
            kind: ast::ProcKind::Always,
            sensitivity: Some(sva.clock),
            body: Box::new(body),
            span: sp,
        };
        let proc = self.lower_proc_block(&pb);
        self.push_process(proc);
    }

    /// Extract the boolean expression of a sequence operand in a property tree
    /// (slice N2d). Loud-rejects (returns `None`) a multi-term / clocked / named
    /// sequence operand (only a boolean leaf is in subset), a BARE recursion
    /// reference (a self-name not in a `|=>` consequent position), and a reference
    /// to ANOTHER declared property (cross-property trees are out of subset).
    pub(crate) fn prop_bool_operand(
        &mut self,
        seq: &ast::Sequence,
        self_name: Option<&str>,
    ) -> Option<ast::Expr> {
        let ast::Sequence::Boolean(e) = seq else {
            self.error(
                MsgCode::ElabUnsupported,
                "a property-level `and`/`or` operand must be a boolean (a multi-term \
                 / re-clocked / named sequence operand is unsupported in this subset)",
            );
            return None;
        };
        if let ast::ExprKind::Ident(p) = &e.kind {
            if p.segments.len() == 1 && self.lookup_net_scoped(&p.segments[0].name).is_none() {
                let n = &p.segments[0].name;
                if Some(n.as_str()) == self_name {
                    self.error(
                        MsgCode::ElabUnsupported,
                        &format!(
                            "a recursive reference to property `{n}` is legal only as the \
                             consequent of `|=>` (a bare / overlap / antecedent recursion \
                             is unsupported in this subset)"
                        ),
                    );
                    return None;
                }
                if self.prop_table.contains_key(n) {
                    self.error(
                        MsgCode::ElabUnsupported,
                        &format!(
                            "a reference to another property `{n}` inside a property-level \
                             `and`/`or` is unsupported in this subset"
                        ),
                    );
                    return None;
                }
                if self.seq_table.contains_key(n) {
                    self.error(
                        MsgCode::ElabUnsupported,
                        &format!(
                            "a reference to named sequence `{n}` inside a property-level \
                             `and`/`or` is unsupported in this subset"
                        ),
                    );
                    return None;
                }
            }
        }
        Some(e.clone())
    }

    /// True iff `cons` is exactly a bare reference to the recursive property's own
    /// name (`self_name`), not shadowed by a real net — the legal tail-`|=>`
    /// recursion site `… |=> NAME`.
    pub(crate) fn prop_cons_is_self_recursion(
        &self,
        cons: &ast::PropExpr,
        self_name: Option<&str>,
    ) -> bool {
        let Some(sn) = self_name else {
            return false;
        };
        let ast::PropExpr::Seq(ast::Sequence::Boolean(e)) = cons else {
            return false;
        };
        if let ast::ExprKind::Ident(p) = &e.kind {
            return p.segments.len() == 1
                && p.segments[0].name == sn
                && self.lookup_net_scoped(sn).is_none();
        }
        false
    }

    /// Build the (violation, completion) signals for a SEQUENCE consequent
    /// (`ante |-> b ##1 c`, slice S14) as an obligation chain. `cond_lhs` is the
    /// antecedent match (already +1-clock-delayed for `|=>`), which SEEDS the
    /// obligation. For each flattened term `term_k` due `hop_k` clocks after the
    /// prior term held:
    ///   viol_k     = due_k && !|term_k          (the obligation breaks here)
    ///   due_{k+1}  = delay_{hop_{k+1}}(due_k && |term_k)
    /// violation = OR_k viol_k; completion = due_{last} && |term_last (a
    /// non-vacuous success for the pass action). A reg read yields the PRIOR
    /// clock's value (the if-check runs before the NBAs), so the chain advances
    /// one term per clock. Bounded, single-alternative, boolean-term consequents
    /// only (ranges / goto / nonconsec / unbounded / throughout / within → loud).
    /// Pure IR-0.
    pub(crate) fn build_seq_consequent(
        &mut self,
        cons: &ast::Sequence,
        cond_lhs: &ast::Expr,
        regs: &mut SvaRegs,
        chain_nbas: &mut Vec<ast::Stmt>,
        sp: ast::Span,
    ) -> (ast::Expr, ast::Expr) {
        let mut alts = self.expand_sequence(cons, regs);
        let ok = alts.len() == 1
            && alts[0].1.is_none()
            && alts[0]
                .0
                .iter()
                .all(|(t, h)| matches!(t, SeqTerm::Bool(_)) && matches!(h, SeqHop::Fixed(_)));
        if !ok {
            self.error(
                MsgCode::ElabUnsupported,
                "a sequence consequent must be a single bounded boolean sequence \
                 (ranges / goto / nonconsec / unbounded / throughout / within / \
                 multi-clock consequents are unsupported in this subset)",
            );
            // Recovery: never-violate / never-complete (the run aborts on error).
            return (sva_zero(sp), sva_zero(sp));
        }
        let (terms, _) = alts.pop().unwrap();
        // Seed the obligation with the BOOLEAN truthiness of the antecedent match.
        // `due` is advanced each term with a width-preserving `BitAnd(due, |term)`,
        // so a multi-bit `cond_lhs` (e.g. `valid_vec |-> …`, where the |-> match
        // expr is the raw, un-reduced antecedent) MUST be reduced first — else a
        // truthy value with bit0=0 (2'b10 & 2'b01 = 0) would silently drop the
        // obligation (S14 review HIGH). `|=>` already reduces cond_lhs to the
        // 1-bit pend reg, and RedOr of a 1-bit value is idempotent, so this is
        // uniformly correct.
        let mut due = sva_unary(ast::UnOp::RedOr, cond_lhs.clone(), sp);
        let mut viols: Vec<ast::Expr> = Vec::new();
        for (k, (term, hop)) in terms.into_iter().enumerate() {
            let SeqTerm::Bool(e) = term else {
                unreachable!("ok-check guarantees Bool terms")
            };
            let tb = sva_match(e, sp); // |term_k === 1'b1 (§16.13.5: X/Z = non-match)
                                       // Delay the obligation by the hop BEFORE this term (hop_0 unused: the
                                       // first term is due the seed clock).
            if k > 0 {
                if let SeqHop::Fixed(d) = hop {
                    for _ in 0..d {
                        due = self.seq_delay_reg(due, chain_nbas, sp);
                    }
                }
            }
            // viol_k = due && !|term_k.
            viols.push(sva_binary(
                ast::BinOp::LogAnd,
                due.clone(),
                sva_unary(ast::UnOp::LogNot, tb.clone(), sp),
                sp,
            ));
            // Advance: due_next (combinational) = due && |term_k. The next
            // iteration's hop registers it.
            due = sva_binary(ast::BinOp::BitAnd, due, tb, sp);
        }
        // After the last term, `due` = due_last && |term_last = "consequent
        // completed this clock" (the pass-action success signal).
        let completed = due;
        let mut it = viols.into_iter();
        let mut violation = it
            .next()
            .expect("a sequence consequent has at least one term");
        for v in it {
            violation = sva_binary(ast::BinOp::BitOr, violation, v, sp);
        }
        (violation, completed)
    }

    /// Get-or-create the shared prev-register for `sig` (matching its declared
    /// width), registering the `prev <= signal` NBA on first creation.
    pub(crate) fn sva_prev_for(
        &mut self,
        sig: &str,
        sig_expr: &ast::Expr,
        regs: &mut SvaRegs,
    ) -> String {
        if let Some((_, prev)) = regs.by_signal.iter().find(|(s, _)| s == sig) {
            return prev.clone();
        }
        let width = self
            .lookup_net_scoped(sig)
            .map(|id| self.nets[id as usize].width)
            .unwrap_or(1);
        let prev = self.fresh_sva_reg(width, "prev");
        let sp = sig_expr.span;
        let prev_path = ast::HierPath {
            segments: vec![ast::Ident {
                name: prev.clone(),
                span: sp,
            }],
            span: sp,
        };
        regs.nbas.push(ast::Stmt::NonBlocking {
            lhs: ast::Lvalue::Ident(prev_path),
            delay: None,
            event: None,
            rhs: sig_expr.clone(),
            span: sp,
        });
        regs.by_signal.push((sig.to_string(), prev.clone()));
        prev
    }
}
