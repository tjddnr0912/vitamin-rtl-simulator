//! The `Kernel` trait's store-INDEPENDENT half: the `k_*_rhs` predicates that
//! decide WHICH `StmtEffect` a statement builds.
//!
//! Every one of them is a function of `ir.exprs` alone — no net state, no
//! scheduler state — which is why they can live here as free functions instead
//! of on an implementor. That is not a tidiness argument, it is a correctness
//! one, and S1d-4a is where it stops being optional:
//!
//! `compute_effect` asks these questions to choose between "evaluate the rhs
//! through the pure funnel" and "run a statement-level effect in the WRITE
//! phase". Two `Kernel` implementors that answer one of them differently do not
//! produce a wrong VALUE — they produce a DIFFERENT STATEMENT, and the
//! difference is silent (a `$random(seed)` rhs routed to the pure funnel draws
//! from the wrong stream and never writes the seed back; nothing reports it).
//! With one spelling that divergence is not a bug to find, it is unrepresentable.
//!
//! §4.5.291 is the reason this module exists rather than a second copy of the
//! `matches!` arms: that slice shipped a predicate written as a sibling of an
//! `_`-free one and inherited none of its exhaustiveness, and the miss it cost
//! was exactly of this kind. The rule merged into ENGINEERING_RULES from it —
//! "a twin predicate does not inherit its twin's properties" — has one reliable
//! enforcement, which is not writing the twin.
//!
//! NOT here: `k_class_new_site`, the one classification question that is NOT a
//! function of `ir.exprs`. It reads the `class_new_sites` side table elaborate
//! builds, so an implementor must hold that map; it is passed by reference to
//! both, which buys the same single-spelling property by a different route.

use sim_ir::{Expr, SysFuncId};

/// `$q.pop_front()` / `$q.pop_back()` — the pop MUTATES the queue, so it belongs
/// to the WRITE phase (`StmtEffect::QPop`).
pub(crate) fn queue_pop_rhs(exprs: &[Expr], rhs: u32) -> bool {
    matches!(
        exprs.get(rhs as usize),
        Some(Expr::SysFunc {
            which: SysFuncId::QPopBack | SysFuncId::QPopFront,
            ..
        })
    )
}

/// A SEEDED `$random(seed)` (v7) — writes the advanced LCG state back into the
/// ref seed variable. The UNSEEDED form has no args and stays a pure rhs.
pub(crate) fn random_seeded_rhs(exprs: &[Expr], rhs: u32) -> bool {
    matches!(
        exprs.get(rhs as usize),
        Some(Expr::SysFunc {
            which: SysFuncId::Random,
            args,
        }) if !args.is_empty()
    )
}

/// A seeded `$dist_*(seed, …)` (v9, rank 6) — the `random_seeded_rhs` family.
pub(crate) fn dist_seeded_rhs(exprs: &[Expr], rhs: u32) -> bool {
    matches!(
        exprs.get(rhs as usize),
        Some(Expr::SysFunc { which, args })
            if matches!(
                which,
                SysFuncId::DistUniform
                    | SysFuncId::DistNormal
                    | SysFuncId::DistExponential
                    | SysFuncId::DistPoisson
                    | SysFuncId::DistChiSquare
                    | SysFuncId::DistT
                    | SysFuncId::DistErlang
            ) && !args.is_empty()
    )
}

/// `$cast(dst, src)` func-form — writes the `dst` ref arg.
pub(crate) fn cast_rhs(exprs: &[Expr], rhs: u32) -> bool {
    matches!(
        exprs.get(rhs as usize),
        Some(Expr::SysFunc {
            which: SysFuncId::Cast,
            args,
        }) if args.len() == 2
    )
}

/// `$value$plusargs(fmt, var)` — writes the ref VAR on a match.
pub(crate) fn value_plusargs_rhs(exprs: &[Expr], rhs: u32) -> bool {
    matches!(
        exprs.get(rhs as usize),
        Some(Expr::SysFunc {
            which: SysFuncId::ValuePlusargs,
            ..
        })
    )
}

/// `$fopen(...)` — mutates the file table.
pub(crate) fn fopen_rhs(exprs: &[Expr], rhs: u32) -> bool {
    matches!(
        exprs.get(rhs as usize),
        Some(Expr::SysFunc {
            which: SysFuncId::Fopen,
            ..
        })
    )
}

/// `$fgetc(fd)` — advances the fd read position.
pub(crate) fn fgetc_rhs(exprs: &[Expr], rhs: u32) -> bool {
    matches!(
        exprs.get(rhs as usize),
        Some(Expr::SysFunc {
            which: SysFuncId::Fgetc,
            ..
        })
    )
}

/// `$feof(fd)` — routed through the same direct-rhs family as `$fgetc` so a fd
/// read and its EOF test share one evaluation order.
///
/// ⚠️ MEASURED OVER-MARK (§4.5.291, ROADMAP §5.1): the worker is a PURE READ of
/// the lazy-EOF flag, so this predicate is stricter than the effect it names.
/// It is preserved verbatim because the canonical `sim_ir::sysfunc_is_stmt_effect`
/// says the same thing and the two must agree; narrowing it is a slice that
/// changes BOTH, since tier-2's compile gate reads the canonical one.
pub(crate) fn feof_rhs(exprs: &[Expr], rhs: u32) -> bool {
    matches!(
        exprs.get(rhs as usize),
        Some(Expr::SysFunc {
            which: SysFuncId::Feof,
            ..
        })
    )
}

/// `$ungetc(c, fd)` — mutates the fd pushback buffer.
pub(crate) fn ungetc_rhs(exprs: &[Expr], rhs: u32) -> bool {
    matches!(
        exprs.get(rhs as usize),
        Some(Expr::SysFunc {
            which: SysFuncId::Ungetc,
            ..
        })
    )
}

/// `$fgets(str, fd)` — advances the fd AND writes the str destination.
pub(crate) fn fgets_rhs(exprs: &[Expr], rhs: u32) -> bool {
    matches!(
        exprs.get(rhs as usize),
        Some(Expr::SysFunc {
            which: SysFuncId::Fgets,
            ..
        })
    )
}

/// `$fread(target, fd[, start[, count]])` — reads binary bytes into a reg/memory.
pub(crate) fn fread_rhs(exprs: &[Expr], rhs: u32) -> bool {
    matches!(
        exprs.get(rhs as usize),
        Some(Expr::SysFunc {
            which: SysFuncId::Fread,
            ..
        })
    )
}

/// `$fscanf(fd, fmt, args…)` — the first multi-ref-write intercept.
pub(crate) fn fscanf_rhs(exprs: &[Expr], rhs: u32) -> bool {
    matches!(
        exprs.get(rhs as usize),
        Some(Expr::SysFunc {
            which: SysFuncId::Fscanf,
            ..
        })
    )
}

/// `$sscanf(str, fmt, args…)` — `$fscanf` with a string VALUE source.
pub(crate) fn sscanf_rhs(exprs: &[Expr], rhs: u32) -> bool {
    matches!(
        exprs.get(rhs as usize),
        Some(Expr::SysFunc {
            which: SysFuncId::Sscanf,
            ..
        })
    )
}

/// `$sformatf(fmt, args…)` — rendering needs the full kernel, so it is a
/// statement-level effect rather than a pure rhs.
pub(crate) fn sformatf_rhs(exprs: &[Expr], rhs: u32) -> bool {
    matches!(
        exprs.get(rhs as usize),
        Some(Expr::SysFunc {
            which: SysFuncId::Sformatf,
            ..
        })
    )
}

/// `first`/`next`/`last`/`prev` — they WRITE their ref key argument.
pub(crate) fn assoc_iter_rhs(exprs: &[Expr], rhs: u32) -> bool {
    matches!(
        exprs.get(rhs as usize),
        Some(Expr::SysFunc {
            which: SysFuncId::AssocFirst
                | SysFuncId::AssocNext
                | SysFuncId::AssocLast
                | SysFuncId::AssocPrev,
            ..
        })
    )
}

/// The DISJUNCTION: does `compute_effect` route this rhs to a statement-effect
/// WORKER (rather than through the pure eval funnel)?
///
/// This is the question a consumer actually wants, and it is NOT
/// `sim_ir::rhs_is_stmt_effect`. That predicate answers "can the `&self` frame
/// executor run this", and it says `false` for `$sformatf` on purpose
/// (sim-ir/src/analysis.rs:27) — the frame path has its own intercept. Tier-2
/// discovered the gap and patched it with an explicit extra reject; tier-3's
/// first draft did not, and a `$sformatf`-rhs design was admitted into a walk
/// whose kernel panics on it. Asking the disjunction of the arms
/// `compute_effect` actually branches on cannot under-approximate that way.
#[allow(dead_code)] // The S1d-4a gate's admission filter is the only caller today.
                    // It is NOT wired into `native::design_eligibility` on purpose: that gate
                    // reports SCOPE (`eligible`) and today's STORAGE (`buildable`), and
                    // "today's kernel can run it" is a third layer S1d-4b/4c has to add —
                    // folding this in now would make an unbuilt method read as out of scope.
pub(crate) fn rhs_routes_to_worker(exprs: &[Expr], rhs: u32) -> bool {
    queue_pop_rhs(exprs, rhs)
        || assoc_iter_rhs(exprs, rhs)
        || random_seeded_rhs(exprs, rhs)
        || dist_seeded_rhs(exprs, rhs)
        || cast_rhs(exprs, rhs)
        || value_plusargs_rhs(exprs, rhs)
        || fopen_rhs(exprs, rhs)
        || sformatf_rhs(exprs, rhs)
        || fgetc_rhs(exprs, rhs)
        || feof_rhs(exprs, rhs)
        || ungetc_rhs(exprs, rhs)
        || fgets_rhs(exprs, rhs)
        || fread_rhs(exprs, rhs)
        || fscanf_rhs(exprs, rhs)
        || sscanf_rhs(exprs, rhs)
}
