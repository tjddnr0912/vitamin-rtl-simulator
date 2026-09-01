//! §3 ③ — the direct-rhs-only system FUNCTIONS in an arbitrary expression position.
//!
//! `$fgetc`, `$fgets`, `$fscanf`, `$value$plusargs` and their siblings each MUTATE state
//! from inside the call (an fd's read position, a ref argument), so vita lowers them as
//! statement-level special forms in `sys_special.rs` and `expr_main.rs` loud-rejects every
//! other placement (`E3009 "… only as the direct rhs of a blocking assignment"`). Both
//! oracles accept them anywhere an expression goes, and a 40-cell census found vita loud in
//! 39 of 40 — the exception being `if ($value$plusargs(…))`, which `lower_branch_cond`
//! desugars for exactly this reason, for exactly one family member, in exactly one
//! position. This module is that desugar generalised.
//!
//! The transform is the same one `hoist/general.rs` performs for an output-formal call:
//! evaluate the call into a fresh temp BEFORE the statement, then read the temp where the
//! call stood. It reuses `general.rs`'s [`shape`] as the single source of truth for which
//! children are evaluated, unconditionally, in what order — so the gate that decides
//! "hoistable" and the walk that rewrites cannot disagree about a node.
//!
//! What it deliberately does NOT do:
//!
//! * **Conditionally-evaluated positions** (a `&&`/`||` right operand, a `?:` arm). Lifting
//!   the call there makes an effect that IEEE 1800 §11.4.7/§11.4.11 may skip happen every
//!   time. `general.rs` handles those with guard blocks; a guarded fd read is a bigger
//!   claim than this slice measures, so they stay loud.
//! * **Loop conditions** (`while`/`for`). Those re-evaluate per iteration; a one-shot hoist
//!   at the statement would read the fd once and spin. `repeat (n)` is the opposite —
//!   the count is evaluated once — and is hoisted, matching `hoist_stmt_general`.
//! * **`$sformatf`** (returns a string, and its `sformatf_expr_ok` machinery already
//!   encodes a measured degenerate-eval trap), the SEEDED `$random`/`$dist_*` family (the
//!   non-uniform siblings carry a pre-existing vita-vs-iverilog divergence that new
//!   positions would multiply — `stmt_main.rs` records it), and `$cast` (its temp's type
//!   follows the destination, not the family's int).

use super::general::{shape, Shape};
use super::general_ast::*;
use super::*;
use std::collections::BTreeSet;

/// The lvalue's own index expressions, in the order `relvalue` re-reads them.
fn lvalue_index_seq(lv: &ast::Lvalue) -> Vec<&ast::Expr> {
    let mut v = Vec::new();
    lvalue_index_exprs(lv, &mut v);
    v
}

/// The direct-rhs-only file/plusargs system functions this module hoists — the v7/v9
/// families whose result is an INT and whose statement form is already implemented in
/// `sys_special.rs`. ONE spelling; it is a SUBSET of the names `expr_main.rs` loud-rejects,
/// so a name dropped from here simply keeps the old diagnostic (never a silent miss).
pub(crate) fn is_hoistable_rhs_only_sysfunc(name: &str) -> bool {
    matches!(
        name,
        "$fgetc"
            | "$ungetc"
            | "$fgets"
            | "$fread"
            | "$fscanf"
            | "$sscanf"
            | "$value$plusargs"
            | "$fopen"
    )
}

/// The argument positions a hoistable form WRITES (its ref/destination actuals), as a
/// half-open range over `args`. Hoisting moves that write earlier, so the caller refuses
/// when one of these roots is also read elsewhere in the statement.
///
/// `$fgetc`/`$ungetc`/`$fopen` write no ARGUMENT — but "writes no argument" is not "has no
/// effect": they all move fd state, and `$feof(fd)` READS it. See [`Reads::fd_observer`].
///
/// ⚠️ The destination is argument ZERO for `$fgets(str, fd)` (IEEE 1800 §21.3.4.3) and
/// `$fread(mem, fd[, start[, count]])` (§21.3.4.5) — only `$value$plusargs(fmt, var)` writes
/// argument one. Grouping the three together pointed the overlap gate at the `fd`, which no
/// surviving expression ever holds, so the gate passed everything: measured
/// `n = mem[0] + $fread(mem, fd);` at 73 against iverilog's 15, at exit 0. vita's own
/// recognizers say which it is — `sys_special.rs` lowers `args[0]` and calls
/// `deny_readonly_write(net, "$fgets into" / "$fread into")` on it.
fn write_arg_range(name: &str, argc: usize) -> std::ops::Range<usize> {
    match name {
        "$fgets" | "$fread" => 0..1.min(argc),
        "$value$plusargs" => 1.min(argc)..2.min(argc),
        "$fscanf" | "$sscanf" => 2.min(argc)..argc,
        _ => 0..0,
    }
}

/// What a walk over an expression learned about the names it READS.
///
/// `opaque` is the fail-closed half: a read this walker cannot attribute to a plain
/// single-segment root. Two ways that happens, both measured as silent-wrongs before
/// the flag existed:
///
/// * a **self-hierarchical** path — v1 flattens block-locals to module nets by bare
///   name, so `m.a` and `a` are ONE net while their first segments (`m`, `a`) look
///   disjoint. `hoist/general_query.rs` keeps `hier_path_is_self` for exactly this.
/// * a **package-scoped** name — `p::v` is `ExprKind::PkgScoped`, a `shape()` leaf that
///   the `Ident` arm never sees, yet `import p::*` makes bare `v` the same net.
///
/// A `Shape::NoHoist` node sets it too. `shape_children` reports no children there, but
/// `rebuild` returns such a node by `clone()` — so its reads SURVIVE the rewrite while
/// staying invisible to the set. `hoist/general.rs` calls the same condition `opaque`
/// and stands the statement down on it.
#[derive(Default)]
struct Reads {
    roots: BTreeSet<String>,
    opaque: bool,
    /// A surviving read of FD STATE — in practice `$feof(fd)`, the only observer of it in
    /// vita's system-function table.
    ///
    /// ⚠️⚠️ This exists because the first version of this module asserted the opposite in a
    /// comment: *"they mutate only fd state, which no expression in the statement can read
    /// (`$feof` … is PURE and so never hoisted — it would be re-evaluated where it stands
    /// either way)"*. Being re-evaluated where it stands is exactly the defect: the
    /// MUTATION moves in front of it. `$feof` is pure in its ARGUMENT, not in the file
    /// position, and `sim-ir/src/analysis.rs` already files it in the same arm as `$fgetc`.
    ///
    /// A root-name gate cannot see this — the endangered resource is the descriptor's
    /// state, which no surviving expression names — so `written.roots` stays empty and its
    /// emptiness was itself the reason to pass. Measured: with the file exhausted,
    /// `x = $feof(fd)*10 + $fgetc(fd)` gave 9 where iverilog gives -1, at exit 0. And it is
    /// value-dependent — mid-file the two agree — so a probe that does not run the read up
    /// to EOF is green on the wrong answer.
    fd_observer: bool,
}

/// Does `name` observe fd state (rather than an argument)? `$feof` is the whole list:
/// `map_sysfunc` has no `$ftell`/`$ferror`, and every other fd-touching form is either a
/// task or a member of the hoisted family itself.
fn observes_fd_state(name: &str) -> bool {
    name == "$feof"
}

impl Elaborator<'_> {
    /// A fresh 32-bit SIGNED temp for a hoisted call's result.
    ///
    /// Signed, deliberately: every form here returns `int`, and three of them report
    /// failure as **-1** (`$fgetc` at EOF, `$fgets`/`$fread` on error). `fresh_ia_tmp`
    /// builds an UNSIGNED `Reg`, which would turn `$fgetc(fd) != -1` into
    /// `4294967295 != -1` — always true — and spin. The direct-rhs form never showed this
    /// because it assigns straight to the user's own `integer`.
    ///
    /// ⚠️ It keeps the `$ia_tmp$` SIGIL even though it is not `fresh_ia_tmp`. That sigil is
    /// what the waveform writer filters on (`sim-engine/src/builtins/queues_io.rs` — a
    /// synthetic temp must not appear as a signal, because no other simulator emits one). A
    /// private `$rhs_only_tmp$` name leaked into VCD and FST, and it did so on shapes that
    /// ALREADY WORKED: `if ($value$plusargs(…))` was desugared by `lower_branch_cond` with
    /// a filtered `$ia_tmp$`, and this pass now intercepts it — so a new sigil changed the
    /// waveform of a previously supported design.
    fn fresh_rhs_only_tmp(&mut self) -> String {
        let name = format!("$ia_tmp${}", self.nets.len());
        self.add_net(
            &name,
            ir::NetVar {
                kind: ir::NetKind::Integer,
                width: 32,
                msb: 31,
                lsb: 0,
                signed: true,
                array_len: 1,
                dir: ir::PortDir::Internal,
                init: default_init(ast::NetVarKind::Reg, 32),
            },
        );
        name
    }

    /// Is `e` a hoistable direct-rhs-only system call?
    fn rhs_only_call<'a>(&self, e: &'a ast::Expr) -> Option<(&'a str, &'a [ast::Expr])> {
        if let ast::ExprKind::SysCall { name, args } = &e.kind {
            if is_hoistable_rhs_only_sysfunc(&name.name) {
                return Some((name.name.as_str(), args));
            }
        }
        None
    }

    /// Does `e` carry one of these calls anywhere the shape walk can see?
    pub(crate) fn has_rhs_only_call(&self, e: &ast::Expr) -> bool {
        if self.rhs_only_call(e).is_some() {
            return true;
        }
        shape_children(e).iter().any(|c| self.has_rhs_only_call(c))
    }

    /// Is EVERY such call in `e` in a position [`hoist_rhs_only`] rewrites — i.e. reached
    /// through unconditionally-evaluated children only? False ⇒ the caller stands down and
    /// the call keeps its existing `E3009` (correct-or-loud), never a partial rewrite.
    fn rhs_only_hoistable(&self, e: &ast::Expr) -> bool {
        if let Some((_, args)) = self.rhs_only_call(e) {
            // The arguments are evaluated AT the hoisted call site, so a nested call in one
            // of them must be hoistable too — its temp is emitted first and read there.
            return args.iter().all(|a| self.rhs_only_hoistable(a));
        }
        match shape(e) {
            Shape::Uncond(cs) => cs.iter().all(|c| self.rhs_only_hoistable(c)),
            // Conditional or repeated evaluation, and the `$bits`-family children that are
            // not evaluated at all: a call under any of these must not move. Walk them so a
            // call-free node here does not make the statement stand down.
            Shape::ShortCircuit { lhs, rhs, .. } => {
                self.rhs_only_hoistable(lhs) && !self.has_rhs_only_call(rhs)
            }
            Shape::Ternary {
                cond,
                then_e,
                else_e,
            } => {
                self.rhs_only_hoistable(cond)
                    && !self.has_rhs_only_call(then_e)
                    && !self.has_rhs_only_call(else_e)
            }
            Shape::NoHoist(cs) | Shape::Unevaluated(cs) => {
                cs.iter().all(|c| !self.has_rhs_only_call(c))
            }
        }
    }

    /// Walk `e` for read roots. `skip_calls` omits each hoistable call's own subtree — its
    /// arguments move WITH the call, so they are not left behind to observe the write.
    fn read_roots(&self, e: &ast::Expr, skip_calls: bool, out: &mut Reads) {
        if skip_calls && self.rhs_only_call(e).is_some() {
            return;
        }
        match &e.kind {
            ast::ExprKind::Ident(p) => {
                let Some(seg) = p.segments.first() else {
                    out.opaque = true;
                    return;
                };
                if p.segments.len() > 1 {
                    // Only a SELF-path can alias a bare name; a path into a child instance
                    // names a different module's net and cannot collide.
                    if self.hier_path_is_self(&p.segments) {
                        out.opaque = true;
                    }
                    return;
                }
                out.roots.insert(seg.name.clone());
            }
            ast::ExprKind::PkgScoped { .. } => out.opaque = true,
            ast::ExprKind::SysCall { name, .. } if observes_fd_state(&name.name) => {
                out.fd_observer = true;
            }
            _ => {}
        }
        // One `shape()` per node: it allocates, and this walk runs per statement.
        let children = match shape(e) {
            Shape::NoHoist(cs) => {
                out.opaque = true;
                cs
            }
            // ⚠️ `Unevaluated` must NOT be folded in with `Uncond`. `$bits` and the array
            // queries report a property of the operand's TYPE (IEEE 1800 §20.5/§20.6) and
            // read nothing at run time, so their children are not reads —
            // `general_ast.rs::shape_children` returns `[]` for them, and collapsing the
            // two arms here made `$bits(a)` contribute `a` to the surviving set, which
            // false-rejected `x = $bits(a) + $value$plusargs("N=%d", a)`.
            Shape::Unevaluated(_) => vec![],
            Shape::Uncond(cs) => cs,
            Shape::ShortCircuit { lhs, rhs, .. } => vec![lhs, rhs],
            Shape::Ternary {
                cond,
                then_e,
                else_e,
            } => vec![cond, then_e, else_e],
        };
        for c in children {
            self.read_roots(c, skip_calls, out);
        }
    }

    /// The roots the hoistable calls in `e` WRITE (their ref actuals). A destination this
    /// walker cannot name is `opaque` here too — an unnamed write is exactly the one the
    /// disjointness test would wave through.
    fn written_roots(&self, e: &ast::Expr, out: &mut Reads) {
        if let Some((name, args)) = self.rhs_only_call(e) {
            for a in &args[write_arg_range(name, args.len())] {
                let before = out.roots.len();
                self.read_roots(a, false, out);
                if out.roots.len() == before {
                    // The destination contributed no root at all — a hierarchical or
                    // package-scoped name, or anything else this walker does not model.
                    // Fail closed: an unnamed write is exactly the one the disjointness
                    // test would wave through. (A concat destination is NOT this case: its
                    // parts are ordinary children and do contribute roots.)
                    out.opaque = true;
                }
            }
        }
        for c in shape_children(e) {
            self.written_roots(c, out);
        }
    }

    /// Is the whole analysed sequence safe to hoist? Every call must be in a reachable
    /// position, AND no root one of them writes may be read anywhere else in the sequence:
    /// the hoist moves that write BEFORE the surrounding expression, so a read left behind
    /// would see the post-call value where both oracles show the pre-call one.
    /// `n = $value$plusargs("N=%d", n)` is fine (the read IS the write's own argument);
    /// `x = n + $value$plusargs("N=%d", n)` is not.
    ///
    /// The forms that write no ARGUMENT — `$fgetc`, `$ungetc`, `$fopen`, the majority and
    /// the darkriscv target — skip the ROOT comparison, since there is no root to compare.
    /// They do NOT skip the walk: they still move fd state, and a surviving `$feof` reads
    /// it ([`Reads::fd_observer`]).
    pub(crate) fn rhs_only_hoist_ok_seq(&self, seq: &[&ast::Expr]) -> bool {
        self.rhs_only_hoist_ok_seq_with(seq, &[])
    }

    /// As above, plus `extra`: expressions that are NOT rewritten but ARE read where they
    /// stand. A task call's `inout` actual is the case — it has a copy-IN, so it reads its
    /// actual, but it is also the write destination and so must pass through untouched.
    /// Leaving those reads out of the surviving set let `tk(a, $fscanf(fd,"%d",a))` copy in
    /// the POST-scan `a` where iverilog copies in the pre-scan one.
    pub(crate) fn rhs_only_hoist_ok_seq_with(
        &self,
        seq: &[&ast::Expr],
        extra: &[&ast::Expr],
    ) -> bool {
        if !seq.iter().any(|e| self.has_rhs_only_call(e)) {
            return false;
        }
        if !seq.iter().all(|e| self.rhs_only_hoistable(e)) {
            return false;
        }
        let mut written = Reads::default();
        for e in seq {
            self.written_roots(e, &mut written);
        }
        if written.opaque {
            return false;
        }
        // Always walked — an fd observer is a hazard for EVERY member of the family, not
        // only the ones that write an argument.
        let mut surviving = Reads::default();
        for e in seq.iter().chain(extra.iter()) {
            self.read_roots(e, true, &mut surviving);
        }
        if surviving.fd_observer {
            return false;
        }
        if written.roots.is_empty() {
            return true;
        }
        !surviving.opaque && written.roots.is_disjoint(&surviving.roots)
    }

    /// Is this blocking assign ALREADY the supported direct form with nothing else in it
    /// to hoist — a bare call as the whole rhs and no hoistable call in an lvalue index?
    ///
    /// That is exactly the statement this module synthesizes for every temp, so it is the
    /// termination condition: hoisting it would re-enter forever. It is deliberately not
    /// the weaker "bare call as the whole rhs" — see the `S::Blocking` arm.
    fn blocking_is_already_the_direct_form(&self, lhs: &ast::Lvalue, rhs: &ast::Expr) -> bool {
        self.rhs_only_call(rhs).is_some()
            && !lvalue_index_seq(lhs)
                .iter()
                .any(|e| self.has_rhs_only_call(e))
    }

    /// One-expression form of [`rhs_only_hoist_ok_seq`].
    pub(crate) fn rhs_only_hoist_ok(&self, e: &ast::Expr) -> bool {
        self.rhs_only_hoist_ok_seq(&[e])
    }

    /// Rewrite `e`, emitting `tmp = <call>` into `b` for each hoistable call — in
    /// [`shape`]'s left-to-right evaluation order, so the calls keep their order relative
    /// to each other. A subtree with no call is returned by `clone()` untouched.
    pub(crate) fn hoist_rhs_only(&mut self, b: &mut ProcessBuilder, e: &ast::Expr) -> ast::Expr {
        if !self.has_rhs_only_call(e) {
            return e.clone();
        }
        if self.rhs_only_call(e).is_some() {
            let ast::ExprKind::SysCall { name, args } = &e.kind else {
                return e.clone();
            };
            // A nested call in an argument is evaluated at THIS call site, so its temp is
            // emitted first.
            let args2: Vec<ast::Expr> = args.iter().map(|a| self.hoist_rhs_only(b, a)).collect();
            let tmp_name = self.fresh_rhs_only_tmp();
            // Lower the SUPPORTED form. Routing the synthetic statement through `lower_stmt`
            // is what keeps every family member covered by construction: whichever
            // `*_special` recognizer owns this name is the one that runs, including its own
            // arity/`delay` diagnostics. The rhs is now a bare call, so this module's
            // pre-pass declines on re-entry and the recursion terminates.
            let synth = ast::Stmt::Blocking {
                lhs: ast::Lvalue::Ident(ast::HierPath {
                    segments: vec![ast::Ident {
                        name: tmp_name.clone(),
                        span: e.span,
                    }],
                    span: e.span,
                }),
                delay: None,
                event: None,
                rhs: ast::Expr {
                    kind: ast::ExprKind::SysCall {
                        name: name.clone(),
                        args: args2,
                    },
                    span: e.span,
                },
                span: e.span,
            };
            self.lower_stmt(b, &synth);
            return ident_expr(tmp_name, e.span);
        }
        let children: Vec<ast::Expr> = shape_children(e)
            .into_iter()
            .map(|c| self.hoist_rhs_only(b, c))
            .collect();
        rebuild(e, children)
    }

    /// Sequence form — one left-to-right pass, so an earlier expression's calls are emitted
    /// before a later one's.
    pub(crate) fn hoist_rhs_only_seq(
        &mut self,
        b: &mut ProcessBuilder,
        seq: &[&ast::Expr],
    ) -> Vec<ast::Expr> {
        seq.iter().map(|e| self.hoist_rhs_only(b, e)).collect()
    }
}

impl Elaborator<'_> {
    /// §3 ③ statement dispatch — the FIRST thing `hoist_stmt_top` tries. The rewritten
    /// statement re-enters `lower_stmt`, where every existing arm (the inout hoisters
    /// included) sees it with the calls already replaced by temp reads; this pass declines
    /// on that second visit because no hoistable call remains.
    ///
    /// The arms mirror `hoist_stmt_general`'s, for the same measured reasons: `repeat`
    /// evaluates its count once but a `while`/`for` condition does not; a
    /// `$monitor`/`$strobe` re-renders its arguments later, so a hoist there would freeze
    /// them at the statement.
    pub(crate) fn hoist_stmt_rhs_only(
        &mut self,
        b: &mut ProcessBuilder,
        s: &ast::Stmt,
    ) -> Option<ast::Stmt> {
        use ast::Stmt as S;
        // A frame (function OR task) body: the temp is a module net, and these forms are
        // statement-level effects the frame executors route separately, so there is no
        // established place for the synthetic write. This is BROADER than
        // `hoist_stmt_general`, which stands down only in a frame FUNCTION (R23 §3.1
        // restored the task case for the copy-out it hoists) — that argument is about a
        // `Terminator::Call` and does not carry here. Measured: both bodies are loud in
        // PRE and in POST, so the width of the stand-down costs nothing today.
        if self.in_frame_body() {
            return None;
        }
        match s {
            S::If {
                cond,
                then_s,
                else_s,
                span,
            } if self.rhs_only_hoist_ok(cond) => {
                let cond2 = self.hoist_rhs_only(b, cond);
                Some(S::If {
                    cond: cond2,
                    then_s: then_s.clone(),
                    else_s: else_s.clone(),
                    span: *span,
                })
            }
            // The rhs and the lvalue's index expressions, in iverilog's measured order
            // (rhs first).
            //
            // ⚠️⚠️ TERMINATION. `x = $fgetc(fd)` with a plain-identifier lvalue is both the
            // SUPPORTED direct form and the exact shape this module emits for every temp,
            // so hoisting it re-enters here forever (measured: stack overflow). The guard
            // is therefore "bare call as the whole rhs AND nothing else in the statement to
            // hoist" — which is precisely the synthetic statement, and precisely the form
            // that already lowers correctly without help.
            //
            // ⚠️ It is NOT "bare call as the whole rhs", full stop. That version left the
            // rhs in place and hoisted only the lvalue's indices, which reversed them:
            // iverilog evaluates the RHS FIRST (measured: `m[$fgetc(f)] = $fgetc(f)` stores
            // 65 at index 66), and a hoisted index is emitted BEFORE the statement, so the
            // index took 65 and the rhs 66 — a silent swap at exit 0.
            S::Blocking {
                lhs,
                delay,
                event,
                rhs,
                span,
            } if delay.is_none()
                && event.is_none()
                && !self.blocking_is_already_the_direct_form(lhs, rhs)
                && self.rhs_only_hoist_ok_seq(&assign_seq(lhs, rhs)) =>
            {
                let mut it = self
                    .hoist_rhs_only_seq(b, &assign_seq(lhs, rhs))
                    .into_iter();
                let rhs2 = it.next()?;
                let lhs2 = relvalue(lhs, &mut it);
                Some(S::Blocking {
                    lhs: lhs2,
                    delay: delay.clone(),
                    event: event.clone(),
                    rhs: rhs2,
                    span: *span,
                })
            }
            // A non-blocking rhs is SAMPLED in Active exactly like a blocking one — only
            // the write is deferred — so the call runs at the same point either way. There
            // is no direct form here to preserve (that is what §3 ③ names as the darkriscv
            // blocker: `UART_RFIFO <= $fgetc(fd);`), so a bare whole-rhs call hoists too.
            S::NonBlocking {
                lhs,
                delay,
                event,
                rhs,
                span,
            } if delay.is_none()
                && event.is_none()
                && self.rhs_only_hoist_ok_seq(&assign_seq(lhs, rhs)) =>
            {
                let mut it = self
                    .hoist_rhs_only_seq(b, &assign_seq(lhs, rhs))
                    .into_iter();
                let rhs2 = it.next()?;
                let lhs2 = relvalue(lhs, &mut it);
                Some(S::NonBlocking {
                    lhs: lhs2,
                    delay: delay.clone(),
                    event: event.clone(),
                    rhs: rhs2,
                    span: *span,
                })
            }
            S::Case {
                kind,
                scrutinee,
                items,
                span,
            } if self.rhs_only_hoist_ok(scrutinee) => {
                let s2 = self.hoist_rhs_only(b, scrutinee);
                Some(S::Case {
                    kind: *kind,
                    scrutinee: s2,
                    items: items.clone(),
                    span: *span,
                })
            }
            S::Repeat { count, body, span } if self.rhs_only_hoist_ok(count) => {
                let c2 = self.hoist_rhs_only(b, count);
                Some(S::Repeat {
                    count: c2,
                    body: body.clone(),
                    span: *span,
                })
            }
            S::SysTaskCall { name, args, span }
                if !is_deferred_print_task(&name.name)
                    && self.rhs_only_hoist_ok_seq(&args.iter().collect::<Vec<_>>()) =>
            {
                let args2 = self.hoist_rhs_only_seq(b, &args.iter().collect::<Vec<_>>());
                Some(S::SysTaskCall {
                    name: name.clone(),
                    args: args2,
                    span: *span,
                })
            }
            // Only the `input` actuals are analysed and rewritten: an output/inout actual is
            // a write DESTINATION, and redirecting one to a temp loses the callee's write
            // (`hoist_stmt_general` records the measured case).
            S::UserTaskCall { name, args, span }
                if self.task_call_input_args(name, args).is_some_and(|inputs| {
                    let non_inputs: Vec<&ast::Expr> = args
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| !self.task_call_arg_is_input(name, args, *i))
                        .map(|(_, a)| a)
                        .collect();
                    self.rhs_only_hoist_ok_seq_with(&inputs, &non_inputs)
                        && !args.iter().enumerate().any(|(i, a)| {
                            !self.task_call_arg_is_input(name, args, i) && self.has_rhs_only_call(a)
                        })
                        // ⚠️ An `inout` actual has a copy-IN, so it READS its actual — and
                        // only the INPUT actuals are in the analysed sequence, so that read
                        // is invisible to the overlap gate. Measured before this condition
                        // was restored: `tk(a, $fscanf(fd,"%d",a))` copied in the POST-scan
                        // `a` (42) where iverilog copies in 5. `hoist_stmt_general`'s arm
                        // carries the same guard; mirroring it and dropping this was the
                        // whole defect.
                        && !self.task_call_inout_root_written(name, args, &inputs)
                }) =>
            {
                let inputs = self.task_call_input_args(name, args)?;
                let mut rewritten = self.hoist_rhs_only_seq(b, &inputs).into_iter();
                let args2 = args
                    .iter()
                    .enumerate()
                    .map(|(i, a)| {
                        if self.task_call_arg_is_input(name, args, i) {
                            rewritten.next().unwrap_or_else(|| a.clone())
                        } else {
                            a.clone()
                        }
                    })
                    .collect();
                Some(S::UserTaskCall {
                    name: name.clone(),
                    args: args2,
                    span: *span,
                })
            }
            _ => None,
        }
    }
}
