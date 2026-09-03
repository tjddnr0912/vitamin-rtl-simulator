//! Engine-side IEEE 1364-2005 context-determined width inference.
//!
//! Builds a side table `Vec<SelfWidth>` indexed by ExprId, parallel to
//! `SimIr.exprs`, computed once at `SimState::new`. The frozen sim-ir is read
//! verbatim; this table lives ENTIRELY in engine state. It encodes each expr's
//! self-determined (bottom-up) width and signedness per §5.4.1 / §5.5.
//!
//! See `docs/superpowers/plans/2026-06-04-width-inference-spec.md`.

use sim_ir::SimIr;

/// Self-determined sizing of one expression node (IEEE §5.4.1 / §5.5).
/// The canonical rule now lives in `sim_ir::selfwidth` — TWO crates need the
/// same answer (the engine for evaluation contexts, `elaborate` for deciding
/// whether an index it is about to normalize is signed), and a second spelling
/// is how they drift. This module is the engine's DRIVER over that rule: it
/// memoizes the whole arena and owns the class-field patch, which is a vita
/// sidecar rather than a language rule.
pub(crate) use sim_ir::selfwidth::{const_u32_of_expr, SelfWidth};

/// The whole side table, one entry per `SimIr.exprs[i]`.
pub(crate) struct WidthTable {
    sw: Vec<SelfWidth>,
    /// Static real-ness, decided by the SHARED rule in `sim_ir::realness` and
    /// memoized here. The engine needs it BEFORE evaluating a binary's operands:
    /// IEEE §11.8.1 makes the integral-to-real conversion boundary
    /// self-determined, so a real sibling's 64-bit self width must not become the
    /// integral side's evaluation context.
    real: Vec<bool>,
    /// `levelize::proc_read_alias` — the net a procedural `Signal` read takes
    /// instead of its own (ROADMAP §2 row 33). Empty until `Scheduler::new`
    /// installs it; `u32::MAX` = read the node's own net. Lives here because this
    /// is the one sidecar every evaluator already carries — the interpreter's
    /// `EvalCtx`, `native_eval::lower` and `wprog::compile` — so one table answers
    /// all three and a backend cannot disagree with another about which net a
    /// read names.
    read_alias: Vec<u32>,
}

impl WidthTable {
    #[inline]
    pub(crate) fn get(&self, eid: u32) -> SelfWidth {
        self.sw[eid as usize]
    }
    /// The net a procedural read of `eid` resolves to, when it is a read-through.
    #[inline]
    pub(crate) fn read_alias(&self, eid: u32) -> Option<u32> {
        match self.read_alias.get(eid as usize) {
            Some(&n) if n != u32::MAX => Some(n),
            _ => None,
        }
    }
    pub(crate) fn install_read_alias(&mut self, table: Vec<u32>) {
        self.read_alias = table;
    }
    #[inline]
    pub(crate) fn is_real(&self, eid: u32) -> bool {
        self.real[eid as usize]
    }
    #[inline]
    pub(crate) fn width(&self, eid: u32) -> u32 {
        self.sw[eid as usize].width
    }
    #[inline]
    pub(crate) fn signed(&self, eid: u32) -> bool {
        self.sw[eid as usize].signed
    }
}

const WIDTH_MAX: u32 = 1 << 24;

#[inline]
// `w` is u32 so it is already >= 0; do NOT add `.max(0)` — that is a no-op that
// trips `clippy::unnecessary_min_or_max` under `-D warnings`. A floor of 1 is
// applied separately by callers that need it (`.max(1)`), not here.
fn clamp_w(w: u32) -> u32 {
    w.min(WIDTH_MAX)
}

impl WidthTable {
    /// Build the self-width table by a single forward pass over `ir.exprs`.
    /// PRECONDITION (verified §1): every child ExprId < its parent ExprId, so a
    /// forward scan reads only already-filled entries.
    pub(crate) fn build(ir: &SimIr, ft: &crate::FuncTable) -> WidthTable {
        Self::build_with(ir, ft, &std::collections::BTreeMap::new())
    }

    /// As `build_with`, plus the `real d[]` element sidecar. The plain
    /// `build_with` passes an EMPTY set, which is right for every caller that has
    /// no `SimOpts` in hand (the backend-equivalence harnesses and the unit
    /// tests): a dynamic array of reals cannot exist in an IR they build.
    pub(crate) fn build_full(
        ir: &SimIr,
        ft: &crate::FuncTable,
        class_fields: &std::collections::BTreeMap<u32, (u32, bool)>,
        real_elem_dyn_nets: &std::collections::BTreeSet<u32>,
    ) -> WidthTable {
        let mut t = Self::build_with(ir, ft, class_fields);
        t.real = Self::build_real(ir, ft, real_elem_dyn_nets);
        t
    }

    /// One forward pass, same precondition as the width pass: every child
    /// ExprId < its parent, so the memo slot a node reads is already filled.
    fn build_real(
        ir: &SimIr,
        ft: &crate::FuncTable,
        real_elem_dyn_nets: &std::collections::BTreeSet<u32>,
    ) -> Vec<bool> {
        let func_ret_is_real = |f: u32| {
            ft.get(f as usize)
                .and_then(|m| ir.nets.get((m.base_net + m.return_slot) as usize))
                .is_some_and(|n| matches!(n.kind, sim_ir::NetKind::Real))
        };
        let cx = sim_ir::realness::RealnessCtx {
            exprs: &ir.exprs,
            consts: &ir.consts,
            nets: &ir.nets,
            real_elem_dyn_nets,
            func_ret_is_real: &func_ret_is_real,
        };
        let mut out: Vec<bool> = Vec::with_capacity(ir.exprs.len());
        for i in 0..ir.exprs.len() {
            let r = sim_ir::realness::expr_is_real_node(
                &cx,
                &|id| *out.get(id as usize).unwrap_or(&false),
                i as u32,
            );
            out.push(r);
        }
        out
    }

    /// As `build`, plus N7's class-field override.
    ///
    /// The override is applied INSIDE the forward pass, not afterwards. A class
    /// field-read `Signal` sits on the 32-bit HANDLE net, so the language rule
    /// alone gives it 32/unsigned and the real 8/signed (say) is a vita sidecar
    /// keyed per-ExprId. This used to be a `patch_class_fields` sweep over the
    /// finished table — which fixed the leaf and NOTHING ABOVE IT, because every
    /// parent had already been computed from the handle. So `c.sb` printed -6
    /// while `~c.sb` printed 4294967045 instead of 5, and `(~c.si) < 0` was
    /// false: silently unsigned the moment an operator touched the field
    /// (§4.5.309; found by an equivalence test against `elaborate`'s driver,
    /// which had always applied the map inline).
    pub(crate) fn build_with(
        ir: &SimIr,
        ft: &crate::FuncTable,
        class_fields: &std::collections::BTreeMap<u32, (u32, bool)>,
    ) -> WidthTable {
        let n = ir.exprs.len();
        let mut sw: Vec<SelfWidth> = Vec::with_capacity(n);
        // A user function's declared return type is a sidecar on each side, so
        // the shared rule takes it as a resolver rather than a table type.
        let call_ret = |f: u32| ft.get(f as usize).map(|m| (m.ret_width, m.ret_signed));
        // Hoisted because this loop runs once per expression in the design and
        // the map is EMPTY for every design without a class, so descending a
        // BTreeMap per iteration is pure cost there.
        //
        // ⚠️ It is NOT what a debug build's 4% on picorv32 was — I measured that
        // claim and it is false: hoisting changed nothing, and the release build
        // shows no delta at all (0.859 s → 0.862 s, inside the noise, on
        // byte-identical IR). That 4% is the price of `self_width_of` now living
        // in another crate, where `opt-level = 0` cannot inline it.
        let has_class_fields = !class_fields.is_empty();
        for i in 0..n {
            let s = match if has_class_fields {
                class_fields.get(&(i as u32))
            } else {
                None
            } {
                Some(&(w, signed)) => SelfWidth {
                    width: clamp_w(w.max(1)),
                    signed,
                },
                None => sim_ir::selfwidth::self_width_of(
                    sim_ir::selfwidth::ExprCtx::of(ir),
                    &call_ret,
                    &sw,
                    i as u32,
                ),
            };
            debug_assert_eq!(sw.len(), i, "forward pass invariant");
            sw.push(s);
        }
        // `build_with` has no `SimOpts`, so it cannot see the `real d[]` element
        // sidecar; `build_full` recomputes with it. Every caller that stops here
        // builds its own IR and has no dynamic array of reals in it.
        let real = Self::build_real(ir, ft, &std::collections::BTreeSet::new());
        WidthTable {
            sw,
            real,
            read_alias: Vec::new(),
        }
    }
}

/// Does this lvalue write a REAL net — i.e. lend NO bit width to its right-hand side?
///
/// ⚠️ A REAL HAS NO BIT WIDTH (IEEE §6.12), so the assignment rule `width = max(lhs,
/// self(rhs))` has nothing to take from the left: the right-hand side is
/// SELF-DETERMINED and its value is then converted (§11.8.1 / IEEE 1364 §4.3). Asking
/// `lvalue_width` anyway answers the real's STORAGE size, 64, and evaluating there
/// changes the value for an UNSIGNED operand narrower than that: `byte unsigned b = 8;
/// real r; r = -b;` read 1.84467e+19 instead of 248, because `-b` was computed in a
/// 64-bit context. Both oracles read 248, and `$display("%0d", -b)` — the same
/// expression with no real target — was already 248 here too.
///
/// ⭐ THIRD OCCURRENCE OF ONE TRAP. §4.5.353 caught `ir_lvalue_width` answering 64 for
/// a real assignment target and §4.5.354 caught `ir_bits_of` answering 64 for a real
/// operand; both are in elaborate. This is the engine's copy of the same question, and
/// it is why the predicate is a named function rather than an inline check: the two
/// backends ask it in two places (`Scheduler::eval_for_lvalue` and
/// `k_eval_for_lvalue`) and must not drift.
pub(crate) fn lvalue_targets_real(ir: &SimIr, lhs: &sim_ir::Lvalue) -> bool {
    lhs.chunks.iter().any(|c| {
        ir.nets
            .get(c.net as usize)
            .is_some_and(|n| matches!(n.kind, sim_ir::NetKind::Real))
    })
}
