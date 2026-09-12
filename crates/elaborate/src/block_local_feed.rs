//! §2 Scoping queue row 1 — feeding a package subroutine body to the block-local
//! classifier at the ONE site that binds it ON DEMAND.
//!
//! `block_local_class.rs` computes the scoping map once per module instance and
//! again at step (3.6a) once the IMPORTED package routine bodies are bound. A call
//! written by its scoped spelling (`pk::g()`) whose routine no `import` brought in
//! reaches neither: `inline_pkg_function` reserves and lowers that body's frame on
//! demand in pass 7. This module holds the phase-(1) gather the two phases were
//! split around and the on-demand feed that uses it.

use super::*;
use crate::block_local_class::BranchPath;

/// Phase-(1) state of [`Elaborator::compute_scoped_block_locals`]: every admitted
/// block-local declaring span, per bare NAME, plus the generate branch each span
/// sits under. Split out of the compute so a package subroutine body bound to the
/// instance AFTER step (3.6a) — a `pk::g()` scoped call reserves its frame on
/// demand — can be gathered onto the SAME base and re-classified jointly.
///
/// In-memory only (an `elaborate` field), never a `SchemaHash` / `sim-ir` /
/// `hdl-ast` type, so it cannot move the golden root.
#[derive(Clone, Default)]
pub(crate) struct ScopedGather {
    pub(crate) per_name: BTreeMap<String, Vec<(u32, u32, AdmitReason)>>,
    pub(crate) branch_of: BTreeMap<(u32, u32), BranchPath>,
}

impl Elaborator<'_> {
    /// §2 Scoping queue row 1: give a package subroutine body bound to this instance
    /// ON DEMAND the same block-local classification step (3.6a) gives an imported
    /// one, then the same scope-leak gate.
    ///
    /// A `pk::g()` scoped call whose routine is NOT in `rtn_pkg` (no `import pk::g`
    /// and no `import pk::*` in this module) never reaches (3.6a): the body is
    /// reserved and lowered on demand in `inline_fn.rs`, in pass 7. So two same-named
    /// sibling block-locals in it flattened onto ONE net — measured `Z=88` where both
    /// oracles say 44, on every call spelling that reaches this funnel (module
    /// process, `generate if`, continuous assign, `always_comb`, class method,
    /// interface body; 19 silent-wrong census cells). The funnel is the right site
    /// because a pre-scan of `module.body` for a `pkg::name` reference misses two of
    /// them outright (a class method body and an interface body hold the call).
    ///
    /// Candidacy is a JOINT property of every fed body (`spans.len() < 2`, the nesting
    /// filters), so the body is added to the instance's accumulated gather and the
    /// whole map re-classified — never classified on its own.
    ///
    /// ⚠️ Only the spans INSIDE this body take their new classification; the rest of
    /// the map is kept verbatim, and that narrowing is deliberate. The body's own
    /// spans are safe to change here because `reserve_frame_func` runs on the line
    /// after this call, so their nets are created from the map this installs. A
    /// MODULE span is not: this runs in pass 7, while the Nets-phase hoist created
    /// that block's nets at step (4) (`instance.rs`, the net passes), so a module
    /// block that gained a `$blk$<lo>` segment here would be resolved under a segment
    /// with no net by any process lowered after this call. The joint re-classification
    /// can only ADD entries for a module span — a span is dropped from candidacy only
    /// by CONTAINMENT, and a package declaration and a module one are in disjoint
    /// source regions — so the narrowing never withdraws a scope either.
    ///
    /// Probed, not assumed, in both directions: `cases2/p01`-`p05` of the census scratch
    /// (a module subroutine, a same-process module block and a later-process module
    /// block each declaring the package body's name once, `static` and `automatic`)
    /// print the oracle value with the narrowing and WITHOUT it alike, so no measured
    /// cell needs the wider map. The narrowing is the no-widening choice, kept because
    /// the ordering above says the wider one has no way to be consistent.
    pub(crate) fn feed_scoped_block_locals(&mut self, body: &ast::Stmt) {
        let sp = Self::stmt_span_key(body);
        if !self.scoped_gather_fed.insert(sp) {
            return;
        }
        let shadow_names = self.names_with_pkg_var_aliases(&self.local_decl_names.clone());
        let mut g = std::mem::take(&mut self.scoped_gather);
        Self::gather_scoped_one(body, &Vec::new(), true, &shadow_names, &mut g);
        let joint = Self::classify_scoped_block_locals(&g, &shadow_names);
        self.scoped_gather = g;
        for (lo, names) in joint {
            if lo >= sp.0 && lo <= sp.1 {
                self.scoped_block_locals
                    .entry(lo)
                    .or_default()
                    .extend(names);
            }
        }
        // ⚠️ This feed deliberately does NOT run `check_block_local_scope_leaks`.
        //
        // An earlier round did, to give the scoped spelling the parity its module and import
        // twins have (census c18). The gate's shadow predicate keys on a NAME collision, so
        // running it here turned a body whose inner block-local merely SHADOWS the outer one
        // from correct into loud. Three attempts to narrow the predicate instead — "the inner
        // declaration is never referenced and has no initializer", then that plus a
        // geometry match against the resolved outer twin — each produced a new defect on the
        // same axis (measured: a §11.6.1 context-width hijack through the inline lane's
        // name-keyed `scope.dims` lookup, then a correct → loud regression on
        // `SCRATCH/review6/soundness-r2/d6.sv`, then a missed outer twin when the routine
        // body's ROOT statement is the enclosing block,
        // `SCRATCH/review6/differential-r3/q12.sv`). Three blockers on one axis is the signal
        // that the axis is wrong (CLAUDE.md D8), so the whole narrowing is reverted and the
        // scoped spelling is simply left ungated — which is PRE byte for byte.
        //
        // Cost, a ROADMAP §2 row: the nested scope-leak shape keeps its PRE value on the
        // scoped spelling (census c18 `Z=14` where both oracles say `Z=7`) while its module
        // and import twins stay LOUD. Closing it needs the gate to resolve the BINDING a
        // post-block reference takes, not to key on the name — the same prerequisite the
        // inline-fold lane's context lookup needs.
    }
}
