//! r19 follow-on — an output/inout-formal call in an ARBITRARY expression position.
//!
//! §4.5.274 opened the positions a *statement-shaped* call can occupy (a bare call
//! statement, `void'(…)`, a direct rhs, a plain condition, one operand of a top-level
//! `&&`/`||`). The moment a call RETURNS A VALUE it can appear anywhere an expression
//! can, and every other position stayed loud: a concat part, another call's argument, a
//! `case` scrutinee, a `$display` argument, a `?:` arm buried inside a bigger
//! expression, an lvalue index. This module closes the general case.
//!
//! Three walkers share ONE shape description (`shape`), so the classifier that decides
//! "this is hoistable" and the transform that does the hoisting cannot disagree — the
//! failure mode `docs/ENGINEERING_RULES.md` records as the classifier/lowering mismatch.
//! Polarity is chosen so a walker that cannot answer makes the general path STAND DOWN
//! (the statement lowers exactly as before, and a call in a genuinely unreachable
//! position stays loud at `emit_frame_call`) — never a silent partial transform.
//!
//! The two conditionally-evaluated shapes (`&&`/`||` right operand, `?:` arms) are
//! handled by emitting the copy-out inside a GUARD BLOCK, keeping the operator node
//! itself in place. Keeping the node is what makes this sound for `?:`: the arms stay
//! context-determined by the enclosing expression exactly as they were, so the
//! §4.5.217 sign/width divergence that the arm-isolating transform had to gate against
//! cannot arise here.

use super::general_ast::*;
use super::*;
use std::collections::{BTreeMap, BTreeSet};

/// How the general hoister models one expression node: which children it evaluates,
/// in which order, and under what condition. SINGLE SOURCE OF TRUTH — the detector,
/// the reachability gate, the eval-order gate and the transform all read this.
pub(crate) enum Shape<'a> {
    /// Every child is evaluated unconditionally, exactly once, in this order.
    Uncond(Vec<&'a ast::Expr>),
    /// `A && B` / `A || B` — `lhs` unconditional, `rhs` only when `lhs` does not
    /// short-circuit (IEEE 1800 §11.4.7). `sc_val` is the captured truth value that
    /// short-circuits: `0` for `&&`, `1` for `||`.
    ShortCircuit {
        sc_val: u32,
        lhs: &'a ast::Expr,
        rhs: &'a ast::Expr,
    },
    /// `c ? T : E` — `cond` unconditional; `T` unless `c` is definitely false, `E`
    /// unless `c` is definitely true (an x `c` evaluates BOTH, IEEE §11.4.11).
    Ternary {
        cond: &'a ast::Expr,
        then_e: &'a ast::Expr,
        else_e: &'a ast::Expr,
    },
    /// A node the hoister must not lift a copy-out out of, but whose children ARE
    /// evaluated and so DO read: a `min:typ:max` choice (one of three runs), a
    /// per-element `with` iterator, a constraint sampler. Listing the children keeps the
    /// detector honest (so an unrelated node here does not make the whole statement stand
    /// down) while `inout_hoistable_general` still refuses a call inside one.
    NoHoist(Vec<&'a ast::Expr>),
    /// A node whose children are NOT EVALUATED at all — `$bits` and the array queries
    /// report a property of the operand's TYPE (IEEE 1800 §20.5/§20.6). Nothing here reads
    /// anything at run time, so these children must not be recorded as reads either.
    Unevaluated(Vec<&'a ast::Expr>),
}

/// The shape of `e` — see [`Shape`]. Children are listed in iverilog's measured
/// left-to-right evaluation order.
///
/// Children that must fold to a CONSTANT (a part-select bound, a replication count, a
/// cast size) are listed as ordinary unconditional children on purpose: a call there
/// can never be constant, so hoisting it merely moves the failure to the const-fold,
/// which reports "not constant" — accurate, and it keeps this shape description
/// exhaustive, which is what lets the four walkers stay in agreement.
pub(crate) fn shape(e: &ast::Expr) -> Shape<'_> {
    use ast::ExprKind as K;
    match &e.kind {
        K::Binary { op, lhs, rhs } => match op {
            ast::BinOp::LogAnd => Shape::ShortCircuit {
                sc_val: 0,
                lhs,
                rhs,
            },
            ast::BinOp::LogOr => Shape::ShortCircuit {
                sc_val: 1,
                lhs,
                rhs,
            },
            _ => Shape::Uncond(vec![lhs, rhs]),
        },
        K::Ternary {
            cond,
            then_e,
            else_e,
        } => Shape::Ternary {
            cond,
            then_e,
            else_e,
        },
        K::Unary { operand, .. } => Shape::Uncond(vec![operand]),
        K::Paren { inner } => Shape::Uncond(vec![inner]),
        K::Concat { parts } => Shape::Uncond(parts.iter().collect()),
        K::Replicate { count, value } => {
            let mut cs = vec![count.as_ref()];
            cs.extend(value.iter());
            Shape::Uncond(cs)
        }
        // A system function that does not evaluate its operand (`$bits`, the array
        // queries) must not have a copy-out hoisted out of it — that would perform a side
        // effect the source never performs — and its operand reads nothing at run time.
        K::SysCall { name, args } if syscall_does_not_evaluate(&name.name) => {
            Shape::Unevaluated(args.iter().collect())
        }
        K::Call { args, .. } | K::SysCall { args, .. } | K::ClassNew { args } => {
            Shape::Uncond(args.iter().collect())
        }
        K::MethodCall { recv, args, .. } => {
            let mut cs = vec![recv.as_ref()];
            cs.extend(args.iter());
            Shape::Uncond(cs)
        }
        K::BitSelect { base, index } => Shape::Uncond(vec![base, index]),
        K::PartSelect { base, msb, lsb } => Shape::Uncond(vec![base, msb, lsb]),
        K::IndexedPart {
            base,
            offset,
            width,
            ..
        } => Shape::Uncond(vec![base, offset, width]),
        K::Cast { target, expr } => match target {
            ast::CastTarget::Size(s) => Shape::Uncond(vec![s, expr]),
            _ => Shape::Uncond(vec![expr]),
        },
        K::AssignPattern(parts) => Shape::Uncond(parts.iter().collect()),
        K::NamedArg { value, .. } => Shape::Uncond(value.iter().map(|v| v.as_ref()).collect()),
        K::New { size, src } => {
            let mut cs = vec![size.as_ref()];
            cs.extend(src.iter().map(|s| s.as_ref()));
            Shape::Uncond(cs)
        }
        K::TimeLit { num, .. } => Shape::Uncond(vec![num]),
        // Leaves — no child can carry a call.
        K::IntLit { .. }
        | K::RealLit { .. }
        | K::StrLit { .. }
        | K::PkgScoped { .. }
        | K::Ident(_)
        | K::Null
        | K::Dollar
        | K::Error => Shape::Uncond(vec![]),
        // `min:typ:max` picks ONE of three, so hoisting all three would fire two copy-outs
        // the source never performs. A `with` iterator runs per element and a constraint
        // sampler is not a once-through evaluation. All three DO read, though, so their
        // children are listed: the eval-order walk has to see those reads, and the detector
        // has to answer honestly (a `$bits`/`min:typ:max` sitting elsewhere in the statement
        // must not make the whole thing stand down).
        K::MinTypMax { min, typ, max } => Shape::NoHoist(vec![min, typ, max]),
        K::Dist { value, items } => {
            let mut cs = vec![value.as_ref()];
            for it in items {
                cs.push(it.lo.as_ref());
                cs.extend(it.hi.as_deref());
                cs.push(it.weight.as_ref());
            }
            Shape::NoHoist(cs)
        }
        K::RandomizeWith(rw) => {
            let mut cs: Vec<&ast::Expr> = rw.args.iter().collect();
            cs.extend(rw.constraints.iter());
            Shape::NoHoist(cs)
        }
        K::ArrayMethodWith(am) => Shape::NoHoist(vec![&am.with_expr]),
    }
}

/// The state threaded through [`Elaborator::order_walk`], left to right across a whole
/// statement's expressions.
struct OrderState {
    /// Roots read so far in positions that REMAIN in the rewritten expression, and that a
    /// pre-call snapshot can therefore repair.
    read: BTreeSet<String>,
    /// Roots read somewhere the rewrite cannot reach — a hierarchical path, or a callee's
    /// body. A hazard on one of these is unrepairable.
    opaque: BTreeSet<String>,
    /// Roots that need a snapshot (or, with `unrepairable`, that make the statement stand
    /// down).
    hazards: BTreeSet<String>,
    unrepairable: bool,
    /// Every root any output actual in the sequence writes — known before the walk so a
    /// callee-body check has something to ask about.
    candidates: BTreeSet<String>,
    /// While walking a `Shape::NoHoist` child: those reads are real, but the AST rewrite
    /// does not descend into such a node (`shape_children` lists no children for it, so the
    /// hoist and the substitution stay in step), which means a snapshot cannot repair them.
    /// Record them as OPAQUE so the statement stands down instead.
    reads_are_opaque: bool,
}

impl Elaborator<'_> {
    /// Does `e` carry an output/inout-formal call anywhere the general hoister can see?
    /// Conservative at an `Opaque` node (answers "yes"), which can only make the general
    /// path stand down — `inout_hoistable_general` rejects `Opaque` too.
    ///
    /// Deliberately a SEPARATE walker from `expr_has_inout_call`: that one is the input
    /// to the §4.5.215/216 gates, and widening it in place would change which shapes
    /// those reviewed paths claim (`docs/ENGINEERING_RULES.md` — shared machinery must
    /// be opt-in).
    pub(crate) fn expr_has_inout_call_deep(&self, e: &ast::Expr) -> bool {
        if self.inout_call_target(e).is_some() {
            return true;
        }
        match shape(e) {
            Shape::Uncond(cs) => cs.iter().any(|c| self.expr_has_inout_call_deep(c)),
            Shape::ShortCircuit { lhs, rhs, .. } => {
                self.expr_has_inout_call_deep(lhs) || self.expr_has_inout_call_deep(rhs)
            }
            Shape::Ternary {
                cond,
                then_e,
                else_e,
            } => {
                self.expr_has_inout_call_deep(cond)
                    || self.expr_has_inout_call_deep(then_e)
                    || self.expr_has_inout_call_deep(else_e)
            }
            // Honest: walk the children. Answering a blanket `true` here made every
            // statement containing a `$bits`/`min:typ:max` — even one carrying no call at
            // all — stand down, which false-louded designs that used to work.
            Shape::NoHoist(cs) | Shape::Unevaluated(cs) => {
                cs.iter().any(|c| self.expr_has_inout_call_deep(c))
            }
        }
    }

    /// Is EVERY output/inout-formal call in `e` in a position `hoist_inout_general`
    /// reaches? False ⇒ the caller stands down and the call stays loud at
    /// `emit_frame_call` (correct-or-loud), never a partial rewrite.
    pub(crate) fn inout_hoistable_general(&self, e: &ast::Expr) -> bool {
        // The call itself is hoistable; its ARGUMENTS are the copy-in, evaluated at the
        // hoisted call site, so a nested call in one of them must be hoistable too.
        if self.inout_call_target(e).is_some() {
            let ast::ExprKind::Call { args, .. } = &e.kind else {
                return false;
            };
            return args.iter().all(|a| self.inout_hoistable_general(a));
        }
        match shape(e) {
            Shape::Uncond(cs) => cs.iter().all(|c| self.inout_hoistable_general(c)),
            Shape::ShortCircuit { lhs, rhs, .. } => {
                self.inout_hoistable_general(lhs) && self.inout_hoistable_general(rhs)
            }
            Shape::Ternary {
                cond,
                then_e,
                else_e,
            } => {
                self.inout_hoistable_general(cond)
                    && self.inout_hoistable_general(then_e)
                    && self.inout_hoistable_general(else_e)
            }
            // Not a hoist site: reachable only if there is no call in there at all.
            Shape::NoHoist(_) | Shape::Unevaluated(_) => !self.expr_has_inout_call_deep(e),
        }
    }

    /// R20: can the call `cn(args)`, which the hoister does NOT lift, be proven unable to read
    /// `v` from anywhere the rewrite cannot reach?
    ///
    /// The hoisted copy-outs all land before the expression, so anything the surviving call
    /// reads is read AFTER them. A read inside this expression is repairable (the pre-call
    /// snapshot substitutes it); a read the expression does not contain is not. Two such places
    /// are answered here, and the round-2 review found the second one missing after this site
    /// stopped calling `call_effect`:
    ///
    ///   1. the callee's BODY ([`Elaborator::callee_body_cannot_touch`]);
    ///   2. an OMITTED formal's DEFAULT, which is lowered in the CALLER's scope and so can
    ///      name `v` while no written-out argument does. `call_effect` had an explicit clause
    ///      for this (R19); `callee_body_cannot_touch` never looks at `ports[i].default`.
    ///      Measured: `q = rd() + nxt(5,o)` with `rd(input int x = o)` returned a value
    ///      inconsistent with vita's own explicit `rd(o)` spelling, at exit 0. It is loud now:
    ///      the default lives on the callee's port, outside this expression, so no snapshot
    ///      substitution can reach it.
    ///
    /// A THIRD place holds such a read and is NOT answered here: a call the body makes at a
    /// level the body walk cannot see. `stmt_no_ref_deep` threads its inertness resolver in at
    /// `UserTaskCall` only, so `expr_no_ref_with`'s `Call` arm (`path_ok(cn) && args.all(..)`)
    /// never consults it and ONE level of function indirection hides everything below it
    /// (`f1 -> f0 -> return o` measures `q=12` where the direct `f0` form is loud). That is
    /// PRE-EXISTING and unchanged by this site: `call_effect` reached the same predicate
    /// through `call_is_inert`, so the depth limit was already load-bearing here. Recorded in
    /// ROADMAP §2; closing it means making that expression walker resolver-aware, which is a
    /// shared-walker change with its own consumers and its own review.
    fn enclosing_call_cannot_read(&self, cn: &ast::HierPath, args: &[ast::Expr], v: &str) -> bool {
        // A TWO-segment `Call` is a method on its head, and `callee_body_cannot_touch` answers
        // `false` for it (no single-segment body to walk) — which stood every such statement
        // down. `call_is_inert` had an arm for this that did not get carried over, and losing it
        // was a loud REGRESSION: `q = qq.size() + nxt(5,o)` and `q = ss.len() + nxt(5,o)` both
        // worked at `8cf4165` and went loud.
        //
        // The old arm accepted ANY 2-segment call, which was unsound for a CLASS method — its
        // body can reach a module net through a hierarchical path (measured at `8cf4165`:
        // `function int get(); return t.o; endfunction` gave `q=12` where 11 is correct, a
        // silent-wrong). A BUILT-IN container/string query is different: it has no user body at
        // all, so the only storage it can read is its receiver (checked here) and its arguments
        // (walked as `shape` children, hence repairable).
        //
        // `container_method_is_pure` alone does NOT select for "built-in" — it is a whitelist of
        // METHOD NAMES, and a user subroutine may carry any of them. Round 3 measured that a
        // class method, a child-instance function and a plain module-scope function all named
        // `size` were admitted and read the POST-call value (`q=12`, iverilog 11) — 30 of the 34
        // whitelisted names, in three receiver forms. So the RECEIVER is identified positively:
        // it must resolve to a net that actually IS a container or string. A class handle is an
        // integral net, and a module instance or the enclosing module's own name is not a net at
        // all, so all three unsound forms fall through to the conservative branch below.
        if let [recv, method] = &cn.segments[..] {
            // Resolved with `dyn_handle` — the SAME resolver the LOWERING uses, which is the
            // whole point. A first version asked `lookup_net_scoped(&recv.name)` and lost the
            // routed fixed `string` array: it is registered under a MANGLED net name
            // (`<name>$sad`, deliberately, so the bare name stays free in the module
            // namespace), so the declared name resolves to nothing and `string rv[3];
            // rv.size()` went from working to loud — the classifier/lowering-resolver mismatch
            // this codebase keeps re-learning. `dyn_handle` consults the side map first with the
            // shadow-aware walk and falls back to `symbols`, exactly as the lowering does.
            //
            // Container/string KINDS only. An `|| n.array_len != 1` clause was tried and removed
            // as pure liability: vita routes every `x.m()` whose head is a plain unpacked-ARRAY
            // net to the hierarchical function-call path, so it admitted an arbitrary user body
            // (measured `q=12` where iverilog says 11, via a generate-scope instance shadowing
            // an array of the same name) — and it bought nothing, since all nine fixed-array
            // methods are loud here anyway.
            // `dyn_handle` covers the heap-backed containers INCLUDING the routed fixed string
            // array; it does not report a scalar `string`, whose methods (`len`, `substr`, …)
            // are equally body-less, so that one is resolved plainly.
            let recv_is_container = self.dyn_handle(&recv.name).is_some()
                || self
                    .lookup_net_scoped(&recv.name)
                    .and_then(|id| self.nets.get(id as usize))
                    .is_some_and(|n| matches!(n.kind, ir::NetKind::String));
            // A PACKAGE-scoped call `pk::h(...)` is safe for a different reason, and vita has
            // already proven it: `inline_pkg_function` admits only a "self-contained,
            // straight-line" package function — its body may reference nothing but its own
            // formals/locals and same-package constants, and any body that reads a module net
            // (or has control flow) is loud there, at PRE and POST alike (measured). That IS
            // obligation 1 of this predicate, discharged upstream, so re-deriving it here would
            // only lose ground: without this arm every `pk::h(3) + nxt(5, gv)` went from working
            // (PRE `q=12`) to loud.
            if self.pkg_funcs.contains_key(&recv.name) {
                return recv.name != v && method.name != v;
            }
            return recv_is_container
                && crate::da::container_method_is_pure(&method.name)
                && recv.name != v
                && method.name != v;
        }
        self.callee_body_cannot_touch(cn, v, Self::CALL_INERT_DEPTH)
            && self.callee_arg_binds(cn, args).is_some_and(|b| {
                b.iter().all(|(_, p, a)| {
                    // A binding that came from the formal's DEFAULT is the caller-scope
                    // expression of obligation 2; a written-out actual is repairable.
                    !p.default.as_ref().is_some_and(|d| std::ptr::eq(d, *a))
                        || crate::da::expr_no_ref_deep(a, v)
                })
            })
    }

    /// Is hoisting the copy-outs out of `e` EVAL-ORDER-safe?
    ///
    /// A hoist moves a call's write to before the whole expression is evaluated, while
    /// every read that REMAINS in the expression is evaluated after all of them. So a
    /// read that the source evaluates BEFORE the call would silently see the post-call
    /// value. Walking left to right and remembering the roots read so far catches exactly
    /// that: a call whose output actual roots are already in the set is a hazard.
    ///
    /// A read to the RIGHT of the call is safe and allowed — measured against iverilog
    /// (`q = nxt(5,o) + o` is `6 + 50`, and `if (nxt(5,o)==6 && o==50)` is taken).
    /// Declining those was the conservatism that kept the report's own
    /// `while (rsp_next(fd,r)==1 && r.len > 0)` shape loud.
    ///
    /// Left-to-right walk over the whole statement's ordered sequence. `st.read` holds the
    /// roots read so far in positions that stay in the rewritten expression, `st.opaque`
    /// the ones read somewhere the rewrite cannot reach; a call mutating a root from either
    /// set is a hazard, and one from `opaque` is an UNREPAIRABLE hazard.
    fn order_walk(&self, e: &ast::Expr, st: &mut OrderState) {
        if self.inout_call_target(e).is_some() {
            let ast::ExprKind::Call { args, .. } = &e.kind else {
                return;
            };
            // The arguments are the copy-in: they are evaluated AT the hoisted call site,
            // so they keep their order relative to every other hoisted call and their reads
            // are not hazards. Walk them only to reach nested calls — hence a scratch copy
            // of the read sets, SEEDED with what is already there so a nested call still
            // sees the reads to its left.
            let mut moved = OrderState {
                read: st.read.clone(),
                opaque: st.opaque.clone(),
                hazards: std::mem::take(&mut st.hazards),
                unrepairable: st.unrepairable,
                candidates: st.candidates.clone(),
                reads_are_opaque: st.reads_are_opaque,
            };
            for a in args {
                self.order_walk(a, &mut moved);
            }
            st.hazards = std::mem::take(&mut moved.hazards);
            st.unrepairable |= moved.unrepairable;

            for v in self.mutated_roots_of_call(e) {
                if st.opaque.contains(&v) {
                    st.unrepairable = true;
                    st.hazards.insert(v);
                } else if st.read.contains(&v) {
                    st.hazards.insert(v);
                }
            }
            return;
        }
        if let ast::ExprKind::Ident(p) = &e.kind {
            match p.segments.as_slice() {
                [seg] => {
                    if st.reads_are_opaque {
                        st.opaque.insert(seg.name.clone());
                    } else {
                        st.read.insert(seg.name.clone());
                    }
                }
                // A HIERARCHICAL read can name the very net a single-segment output actual
                // writes — v1 flattens block-locals to module nets by BARE NAME, so a
                // self-path `t.o` and `o` are the same storage. The rewrite substitutes
                // single-segment reads only, so such a hazard cannot be repaired: record it
                // as opaque so the statement stands down (measured: `q = t.o + nxt(5,o)` was
                // silently reading the POST-call value, in PRE too).
                //
                // Poison only the candidate the path actually ALIASES, by NET identity —
                // keying on the segment SPELLING made an unrelated child-scope variable
                // (`sub.o`) poison the module's own `o` and false-louded a working design.
                segs => {
                    if !self.hier_path_is_self(segs) {
                        // A path THROUGH A CHILD INSTANCE reaches another scope's nets and can
                        // never be the current scope's `o`. Poisoning on the segment SPELLING
                        // false-louded `q = sub.o + nxt(5, o)`, which is correct code.
                        return;
                    }
                    if let Some(last) = segs.last() {
                        if st.candidates.contains(&last.name) {
                            st.opaque.insert(last.name.clone());
                        }
                    }
                }
            }
            return;
        }
        // A call the hoister does not lift stays in the expression, and its BODY is
        // evaluated there — after every hoisted copy-out. If the body can reach a root a
        // hoisted call writes, that read moved, and no substitution can fix it because it
        // is not in this expression. So an inert callee costs nothing
        // (`q = h(5) + nxt(5,o)` keeps working) while `function rd(); return o;` is caught.
        //
        // R20: the question is about what the callee can read WITHOUT the read appearing in
        // this expression. This asked `call_effect`, whose `Inert` also requires the call's OWN
        // ACTUALS to be free of `v` — and the actual here IS the nested call that writes `v`
        // (`q = other(nxt(fd, v));`). So the enclosing call's inertness was denied by the very
        // hoist being planned, `v` went opaque, and the hazard read as UNREPAIRABLE. Every
        // user-function argument was false-loud (pre-existing: measured identical at
        // `8cf4165`; `$signed(nxt(fd,v))` and `tk(nxt(fd,v))` worked, `other(nxt(fd,v))` did
        // not). The written-out actuals are already walked as `shape` children, so they are
        // legitimately repairable and must NOT be asked about here.
        if let ast::ExprKind::Call { name, args } = &e.kind {
            for v in st.candidates.clone() {
                if !self.enclosing_call_cannot_read(name, args, &v) {
                    st.opaque.insert(v);
                }
            }
        }
        // A method call / `new` / randomize has a body this walk cannot resolve at all.
        if matches!(
            &e.kind,
            ast::ExprKind::MethodCall { .. } | ast::ExprKind::ClassNew { .. }
        ) {
            st.opaque.extend(st.candidates.clone());
        }
        match shape(e) {
            Shape::Uncond(cs) => {
                for c in cs {
                    self.order_walk(c, st);
                }
            }
            // The left operand / condition is CAPTURED into a truth temp at the point the
            // source evaluates it — before the guarded copy-out — so its own reads are not
            // hazards for calls to their right. A call INSIDE it still has to see the reads
            // to ITS left, so the scratch state is seeded from the current one rather than
            // started empty (measured: `q = o + (nxt(5,o) && 1)` was silently 51 not 8).
            Shape::ShortCircuit { lhs, rhs, .. } => {
                self.order_walk_captured(lhs, st);
                self.order_walk(rhs, st);
            }
            Shape::Ternary {
                cond,
                then_e,
                else_e,
            } => {
                self.order_walk_captured(cond, st);
                self.order_walk(then_e, st);
                self.order_walk(else_e, st);
            }
            // These are NOT hoist sites, but their children are evaluated, so the reads
            // there are real and a hoisted copy-out to their right would move past them.
            // Recording them is what `order_clean` needs — the NARROW hoister uses it too,
            // and that path never consults `inout_hoistable_general`, so "the general path
            // would have rejected this node anyway" is not a defence here.
            Shape::NoHoist(cs) => {
                let saved = st.reads_are_opaque;
                st.reads_are_opaque = true;
                for c in cs {
                    self.order_walk(c, st);
                }
                st.reads_are_opaque = saved;
            }
            // Not evaluated at run time ⇒ reads nothing.
            Shape::Unevaluated(_) => {}
        }
    }

    /// Walk a CAPTURED operand (a `&&`/`||` left operand, a `?:` condition): its reads are
    /// materialized at the capture point, so they do not escape to the caller's read set,
    /// but calls inside it must still be checked against the reads to their left.
    fn order_walk_captured(&self, e: &ast::Expr, st: &mut OrderState) {
        let mut inner = OrderState {
            read: st.read.clone(),
            opaque: st.opaque.clone(),
            hazards: std::mem::take(&mut st.hazards),
            unrepairable: st.unrepairable,
            candidates: st.candidates.clone(),
            reads_are_opaque: st.reads_are_opaque,
        };
        self.order_walk(e, &mut inner);
        st.hazards = std::mem::take(&mut inner.hazards);
        st.unrepairable |= inner.unrepairable;
    }

    /// The general hoister's gate for ONE expression — see [`Self::order_plan`] for the
    /// statement-wide form the transform actually uses.
    pub(crate) fn general_hoist_ok(&self, e: &ast::Expr) -> bool {
        self.general_hoist_ok_seq(std::slice::from_ref(&e))
    }

    /// The gate for a SEQUENCE the statement evaluates one after another (a rhs then the
    /// lvalue's indices; an argument list): every call reachable, and every eval-order
    /// hazard across the WHOLE sequence repairable.
    pub(crate) fn general_hoist_ok_seq(&self, seq: &[&ast::Expr]) -> bool {
        seq.iter().any(|e| self.expr_has_inout_call_deep(e))
            && seq.iter().all(|e| self.inout_hoistable_general(e))
            && self.order_plan(seq).is_some()
    }

    /// The roots that must be SNAPSHOT before the hoisted copy-outs so that a read the
    /// source evaluates BEFORE the call still sees the pre-call value (`q = o + nxt(5, o)`
    /// is `7 + 6`, not `50 + 6` — measured). `Some(empty)` ⇒ no hazard at all.
    ///
    /// Runs over the whole SEQUENCE with one shared read set, because the transform hoists
    /// across sub-expression boundaries: an lvalue index's copy-out lands before the rhs is
    /// evaluated, and one argument's copy-out lands before the next argument. Analysing each
    /// piece alone made those cross-boundary hazards invisible — and also defeated the
    /// two-calls-one-root guard below, since neither piece saw both calls.
    ///
    /// `None` ⇒ the hazard cannot be repaired, so the caller stands down and the call stays
    /// loud. Three reasons, all deliberate:
    ///  * the read is somewhere the rewrite cannot reach — a hierarchical path, or inside a
    ///    callee's body;
    ///  * the root is not a plain whole bit-vector net — an unpacked array / struct root
    ///    fans out to several nets, which one `snap = v` assignment cannot copy;
    ///  * the root is written by MORE THAN ONE call in the sequence, where a single snapshot
    ///    cannot serve the reads between them (each needs its own generation).
    pub(crate) fn order_plan(&self, seq: &[&ast::Expr]) -> Option<BTreeSet<String>> {
        // The candidate roots have to be known before the walk, so a callee-body check can
        // ask about them.
        // Shape-driven, because `collect_inout_mutated` only descends Unary/Binary/Paren/
        // Ternary: a call reached through a concat or another call's argument list left
        // `candidates` EMPTY, and then the callee-body and method-body opacity checks — the
        // only things that consult it — never fired at all.
        let mut candidates = BTreeSet::new();
        for e in seq {
            self.collect_mutated_deep(e, &mut candidates);
        }
        let mut st = OrderState {
            read: BTreeSet::new(),
            opaque: BTreeSet::new(),
            hazards: BTreeSet::new(),
            unrepairable: false,
            candidates,
            reads_are_opaque: false,
        };
        for e in seq {
            self.order_walk(e, &mut st);
        }
        if st.unrepairable {
            return None;
        }
        for root in &st.hazards {
            let net = self.lookup_net_scoped(root)?;
            let nv = self.nets.get(net as usize)?;
            if nv.array_len != 1
                || !matches!(
                    nv.kind,
                    ir::NetKind::Wire
                        | ir::NetKind::Reg
                        | ir::NetKind::Logic
                        | ir::NetKind::Integer
                )
            {
                return None;
            }
            let n: u32 = seq.iter().map(|e| self.count_mutating_calls(e, root)).sum();
            if n > 1 {
                return None;
            }
        }
        Some(st.hazards)
    }

    /// Is hoisting the copy-outs out of `seq` order-safe with NO repair needed? The narrow
    /// hoister (`hoist_inout_calls`) and the §4.5.216 arm transforms emit no snapshot, so
    /// they may only claim a sequence with no eval-order hazard at all; anything with a
    /// REPAIRABLE hazard falls through to the general hoister, which does emit one.
    ///
    /// This replaced a per-name walk (`hoist_is_safe` + `reads_ident_outside_inout`) that was
    /// both too strict and too lax: it declined a harmless read to the RIGHT of the call, and
    /// it missed a read spelled as a HIERARCHICAL path or hidden in a callee's BODY — both
    /// measured silently reading the post-call value. One predicate for both paths is also
    /// what keeps the classifier and the transform from drifting apart.
    pub(crate) fn order_clean(&self, e: &ast::Expr) -> bool {
        // `inout_hoistable_general` is part of the gate on purpose: the narrow hoister's own
        // reachability predicate cannot see inside a `min:typ:max` / `with` / constraint
        // node, so without this a call there would be lifted past the reads in it.
        self.inout_hoistable_general(e)
            && self
                .order_plan(std::slice::from_ref(&e))
                .is_some_and(|h| h.is_empty())
    }

    /// How many output/inout-formal calls in `e` write `root` as an output actual.
    fn count_mutating_calls(&self, e: &ast::Expr, root: &str) -> u32 {
        let mut n = 0;
        if self.inout_call_target(e).is_some() {
            if self.mutated_roots_of_call(e).contains(root) {
                n += 1;
            }
            if let ast::ExprKind::Call { args, .. } = &e.kind {
                for a in args {
                    n += self.count_mutating_calls(a, root);
                }
            }
            return n;
        }
        match shape(e) {
            Shape::Uncond(cs) => cs.iter().map(|c| self.count_mutating_calls(c, root)).sum(),
            Shape::ShortCircuit { lhs, rhs, .. } => {
                self.count_mutating_calls(lhs, root) + self.count_mutating_calls(rhs, root)
            }
            Shape::Ternary {
                cond,
                then_e,
                else_e,
            } => {
                self.count_mutating_calls(cond, root)
                    + self.count_mutating_calls(then_e, root)
                    + self.count_mutating_calls(else_e, root)
            }
            // Not a hoist site, so `inout_hoistable_general` has already refused any call
            // in there — counting zero cannot under-count a call the transform will lift.
            Shape::NoHoist(_) | Shape::Unevaluated(_) => 0,
        }
    }

    /// The general hoister's ENTRY POINT for ONE expression.
    pub(crate) fn hoist_inout_general_top(
        &mut self,
        b: &mut ProcessBuilder,
        e: &ast::Expr,
    ) -> ast::Expr {
        let mut out = self.hoist_inout_general_seq(b, std::slice::from_ref(&e));
        out.pop().unwrap_or_else(|| e.clone())
    }

    /// The general hoister's ENTRY POINT for a SEQUENCE the statement evaluates one after
    /// another (a rhs then the lvalue's indices; an argument list). Repairs the eval-order
    /// hazards across the WHOLE sequence, then hoists each expression in order. Only ever
    /// called when [`Self::general_hoist_ok_seq`] holds.
    ///
    /// A hazard root `v` gets `snap = v` emitted before anything else, and the reads of `v`
    /// the source evaluates BEFORE its mutating call are rewritten to read `snap` — so
    /// `q = o + nxt(5, o)` keeps the left `o` at its pre-call value while the copy-out still
    /// lands ahead of the whole expression. `passed` is threaded across the sequence, so an
    /// earlier argument's read is repaired against a LATER argument's call.
    pub(crate) fn hoist_inout_general_seq(
        &mut self,
        b: &mut ProcessBuilder,
        seq: &[&ast::Expr],
    ) -> Vec<ast::Expr> {
        // `None` means an UNREPAIRABLE hazard. Every caller gates on `general_hoist_ok_seq`
        // first, so this cannot fire — but defaulting to "no snapshots" would be a silent
        // partial transform, so decline outright instead.
        let Some(roots) = self.order_plan(seq) else {
            return seq.iter().map(|e| (*e).clone()).collect();
        };
        let mut snaps: BTreeMap<String, String> = BTreeMap::new();
        for root in &roots {
            if let Some(name) = self.emit_pre_call_snapshot(b, root) {
                snaps.insert(root.clone(), name);
            }
        }
        let mut passed = BTreeSet::new();
        let substituted: Vec<ast::Expr> = seq
            .iter()
            .map(|e| self.subst_pre_call_reads(e, &snaps, &mut passed))
            .collect();
        substituted
            .iter()
            .map(|e| self.hoist_inout_general(b, e))
            .collect()
    }

    /// Emit `snap = <root>` (a whole-net copy) into the current block and return the
    /// snapshot's name. `None` if `root` does not resolve to a net — `order_plan` already
    /// required that it does, so this only guards a stale lookup.
    fn emit_pre_call_snapshot(&mut self, b: &mut ProcessBuilder, root: &str) -> Option<String> {
        let src = self.lookup_net_scoped(root)?;
        let mut nv = self.nets.get(src as usize)?.clone();
        // A synthesized temp is internal STORAGE, whatever the source net was: a `wire`
        // clone would be a procedurally-assigned wire, and the source's `init` is the
        // source's business.
        nv.dir = ir::PortDir::Internal;
        nv.kind = if nv.kind == ir::NetKind::Real {
            ir::NetKind::Real
        } else {
            ir::NetKind::Reg
        };
        nv.init = default_init(
            if nv.kind == ir::NetKind::Real {
                ast::NetVarKind::Real
            } else {
                ast::NetVarKind::Reg
            },
            nv.width,
        );
        let name = format!("$ia_snap${}", self.nets.len());
        self.add_net(&name, nv);
        let snap = (self.nets.len() - 1) as u32;
        let rhs = self.push_expr(ir::Expr::Signal {
            net: src,
            word: None,
        });
        let sid = self.push_stmt(ir::Stmt::BlockingAssign {
            lhs: whole_net_lvalue(snap),
            rhs,
        });
        b.push_stmt_id(sid);
        Some(name)
    }

    /// Rewrite the reads of `snaps`' roots that lie to the LEFT of the call that mutates
    /// them so they read the pre-call snapshot. Left to right; once a root's mutating call
    /// has been passed, later reads are left alone — they legitimately see the post-call
    /// value (`q = nxt(5,o) + o` is `6 + 50`).
    ///
    /// A call's own ARGUMENTS are never substituted: they are evaluated at the hoisted call
    /// site, so they already read the same generation the source reads there.
    fn subst_pre_call_reads(
        &self,
        e: &ast::Expr,
        snaps: &BTreeMap<String, String>,
        passed: &mut BTreeSet<String>,
    ) -> ast::Expr {
        use ast::ExprKind as K;
        if self.inout_call_target(e).is_some() {
            passed.extend(self.mutated_roots_of_call(e));
            return e.clone();
        }
        if let K::Ident(p) = &e.kind {
            if let [seg] = p.segments.as_slice() {
                if let Some(snap) = snaps.get(&seg.name) {
                    if !passed.contains(&seg.name) {
                        return ident_expr(snap.clone(), e.span);
                    }
                }
            }
            return e.clone();
        }
        // Children in `shape` order — the same order the source evaluates them, so `passed`
        // accumulates correctly as the walk moves right.
        let children: Vec<ast::Expr> = shape_children(e)
            .iter()
            .map(|c| self.subst_pre_call_reads(c, snaps, passed))
            .collect();
        rebuild(e, children)
    }

    /// Rewrite `e` with every output/inout-formal call replaced by a read of a fresh temp
    /// that a copy-out `Terminator::Call` fills, emitted into `b`. Only ever called when
    /// [`Self::general_hoist_ok`] holds.
    ///
    /// A subtree with no call is returned by `clone()` without being rebuilt, so a
    /// statement's call-free parts are structurally untouched.
    pub(crate) fn hoist_inout_general(
        &mut self,
        b: &mut ProcessBuilder,
        e: &ast::Expr,
    ) -> ast::Expr {
        use ast::ExprKind as K;
        if !self.expr_has_inout_call_deep(e) {
            return e.clone();
        }
        if let Some((fid, func)) = self.inout_call_target(e) {
            let K::Call { args, .. } = &e.kind else {
                return e.clone();
            };
            // A nested call in an argument is part of THIS call's copy-in, so its
            // copy-out has to be emitted first.
            let args2: Vec<ast::Expr> = args
                .iter()
                .map(|a| self.hoist_inout_general(b, a))
                .collect();
            let (rw, rsig) = self
                .func_metas
                .get(fid as usize)
                .map(|m| (m.ret_width, m.ret_signed))
                .unwrap_or((32, true));
            let (tmp_net, tmp_name) = self.fresh_ret_temp(&func, rw, rsig);
            self.emit_frame_func_out_call(b, fid, &func, &args2, whole_net_lvalue(tmp_net));
            return ident_expr(tmp_name, e.span);
        }
        match shape(e) {
            Shape::ShortCircuit { sc_val, lhs, rhs } if self.expr_has_inout_call_deep(rhs) => {
                let op = match &e.kind {
                    K::Binary { op, .. } => *op,
                    _ => unreachable!("ShortCircuit comes only from a Binary"),
                };
                // The left operand is unconditional: hoist it here, then CAPTURE its
                // truth so the final expression reads the capture instead of evaluating
                // it a second time. `&&`/`||` operands are self-determined (§11.4.7), so
                // only the truth matters and the substitution is value-preserving.
                let l = self.hoist_inout_general(b, lhs);
                let (ta_net, ta_name) = self.capture_truth(b, &l);
                let r = self.guarded_hoist(b, ta_net, sc_val, rhs);
                ast::Expr {
                    kind: K::Binary {
                        op,
                        lhs: Box::new(ident_expr(ta_name, lhs.span)),
                        rhs: Box::new(r),
                    },
                    span: e.span,
                }
            }
            Shape::Ternary {
                cond,
                then_e,
                else_e,
            } if self.expr_has_inout_call_deep(then_e) || self.expr_has_inout_call_deep(else_e) => {
                let c = self.hoist_inout_general(b, cond);
                let (cc_net, cc_name) = self.capture_truth(b, &c);
                // `T` is evaluated unless the condition is definitely FALSE, `E` unless it
                // is definitely TRUE — so an x condition runs both, which is what
                // §11.4.11's bit-merge needs. The `?:` node is KEPT: the arms stay
                // context-determined by the enclosing expression, so no coercion changes.
                let t2 = self.guarded_hoist(b, cc_net, 0, then_e);
                let e2 = self.guarded_hoist(b, cc_net, 1, else_e);
                ast::Expr {
                    kind: K::Ternary {
                        cond: Box::new(ident_expr(cc_name, cond.span)),
                        then_e: Box::new(t2),
                        else_e: Box::new(e2),
                    },
                    span: e.span,
                }
            }
            _ => self.rebuild_uncond(b, e),
        }
    }

    /// Rewrite `e`'s unconditionally-evaluated children in order, rebuilding the node
    /// around them. Reached for every non-conditional shape (and for a `&&`/`?:` whose
    /// conditional part carries no call, where the plain rebuild is correct because
    /// nothing has to be guarded).
    fn rebuild_uncond(&mut self, b: &mut ProcessBuilder, e: &ast::Expr) -> ast::Expr {
        let children: Vec<ast::Expr> = shape_children(e)
            .iter()
            .map(|c| self.hoist_inout_general(b, c))
            .collect();
        rebuild(e, children)
    }

    /// Emit `tmp = bool(e)` into the current block and return the 1-bit temp's net. The
    /// truth is captured so the operand is evaluated EXACTLY ONCE at the point the source
    /// evaluates it, and so a guarded copy-out to its right cannot perturb the value the
    /// final expression combines with.
    fn capture_truth(&mut self, b: &mut ProcessBuilder, e: &ast::Expr) -> (u32, String) {
        let id = self.lower_expr(e);
        let truth = self.bool_of(id);
        let (net, name) = self.fresh_bool_temp();
        let sid = self.push_stmt(ir::Stmt::BlockingAssign {
            lhs: whole_net_lvalue(net),
            rhs: truth,
        });
        b.push_stmt_id(sid);
        (net, name)
    }

    /// Hoist the copy-outs in `arm` inside a block entered only when the captured truth
    /// `cc` is NOT `sc_val` — the value that makes the source skip `arm` (`0` for a `&&`
    /// right operand and a `?:` then-arm, `1` for a `||` right operand and an else-arm).
    /// Case-inequality, so an x truth enters the block: that is exactly when IEEE
    /// evaluates the operand anyway (`log_and(x, B)` needs `B`; a `?:` with an x
    /// condition evaluates both arms).
    ///
    /// On the skipped path the arm's temp keeps its default value and is read but not
    /// selected: `log_and(0, anything)` is `0`, `log_or(1, anything)` is `1`, and a `?:`
    /// with a definite condition returns the other arm.
    fn guarded_hoist(
        &mut self,
        b: &mut ProcessBuilder,
        cc: u32,
        sc_val: u32,
        arm: &ast::Expr,
    ) -> ast::Expr {
        if !self.expr_has_inout_call_deep(arm) {
            return arm.clone();
        }
        let cc_id = self.push_expr(ir::Expr::Signal {
            net: cc,
            word: None,
        });
        let lit = self.const_u32_expr(sc_val, 1);
        let test = self.push_expr(ir::Expr::Binary {
            op: ir::BinOp::CaseNe,
            lhs: cc_id,
            rhs: lit,
        });
        let guard = b.new_block();
        let merge = b.new_block();
        b.end_block_with(ir::Terminator::Branch {
            cond: test,
            then_bb: guard.raw(),
            else_bb: merge.raw(),
        });
        b.start_block(guard);
        let out = self.hoist_inout_general(b, arm);
        b.goto(merge);
        b.start_block(merge);
        out
    }

    /// A fresh private 1-bit truth temp (`$ia_cc$<n>` — `$` keeps it collision-proof
    /// against user identifiers, like `$ia_tmp$`/`$ia_ret$`).
    fn fresh_bool_temp(&mut self) -> (u32, String) {
        let name = format!("$ia_cc${}", self.nets.len());
        self.add_net(
            &name,
            ir::NetVar {
                kind: ir::NetKind::Reg,
                width: 1,
                msb: 0,
                lsb: 0,
                signed: false,
                array_len: 1,
                dir: ir::PortDir::Internal,
                init: default_init(ast::NetVarKind::Reg, 1),
            },
        );
        ((self.nets.len() - 1) as u32, name)
    }
}
