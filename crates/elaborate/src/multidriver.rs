//! IEEE 1800 §9.2.2.2/§9.2.2.3/§9.2.2.4 process-multidriver diagnostics for
//! module-scope variables — split out of `var_init.rs` (module-size policy).
//!
//! Three rules live here, and they are three because the two oracles split three
//! ways. Measured, one shape per file, verilator 5.052 `--lint-only` and an
//! external xcelium report (`*E,MULAXX`); iverilog says nothing about any of them:
//!
//! ```text
//!   shape                                   verilator     xcelium      vita
//!   decl initializer + always_comb          MULTIDRIVEN   *E,MULAXX    E3001
//!   decl initializer + always_ff            accepts       *E,MULAXX    W3060
//!   decl initializer + always_latch         accepts       *E,MULAXX    W3060
//!   initial + always_ff                     MULTIDRIVEN   *E,MULAXX    E3001
//!   initial + always_comb                   MULTIDRIVEN   *E,MULAXX    E3001
//!   initial + always_latch                  accepts       *E,MULAXX    W3060
//!   two always_ff on one variable           MULTIDRIVEN   *E,MULAXX    E3001
//!   always_ff + always                      MULTIDRIVEN   *E,MULAXX    E3001
//!   always_comb + continuous assign         MULTIDRIVEN   *E,MULAXX    E3001
//!   always_ff + final                       MULTIDRIVEN   *E,MULAXX    E3001
//!   always + always (no always_*)           accepts       accepts      silent
//!   any pair, EITHER side a PARTIAL write    accepts      UNMEASURED    silent
//! ```
//!
//! ⚠️ The last row is the widest one and the only one with a hole in it. Verilator
//! is silent on every pair where one side writes a struct member, an array element,
//! a bit or a part select — ten shapes measured, `always_ff` against `always_ff` and
//! `initial` against `always_ff`, whole-against-partial in both directions. Its rule
//! is "both writers write the WHOLE variable", and [`stmt_writes_whole_ident`] carries
//! the cell list. **Xcelium on a partial write is UNMEASURED — zero observations, not
//! "accepts"**; the external report covered only whole-variable pairs. So vita follows
//! the one tool that was actually run and stays silent, and this row is the first place
//! to re-measure when an xcelium run is available.
//!
//! The severity is the OWNER's split, not the clause's: IEEE §9.2.2.3 words the
//! `always_latch` rule exactly as §9.2.2.2 and §9.2.2.4 word theirs, and verilator
//! still accepts every `always_latch` pair measured. Where the two tools disagree
//! vita warns rather than stops, so:
//!
//! - Both tools reject ⇒ [`MsgCode::ElabMultidriver`] (error).
//! - Only xcelium rejects ⇒ [`MsgCode::ElabMultidriverStrict`] (warning). That code
//!   covers two shapes: a declaration initializer on an `always_ff`/`always_latch`
//!   variable, and an `always_latch` sharing a variable with another process. The
//!   initializer is that register's power-on value; verilator and every synthesis
//!   flow implement it, and this repository's own `obs_procs` fixture is written in
//!   that idiom. An error there was built once and reverted because it rejected
//!   working RTL — a test design breaking is evidence AGAINST a new rejection, not
//!   for it. The warning exists because the design still dies at xcelium sign-off,
//!   and the development loop is where the author still has the context to fix it.
//!
//! Nothing about a simulated VALUE changes here; this is the diagnostic that was
//! missing, not a semantics change.

use super::*;

/// Does this statement tree DECLARE a block-local (or `fork` block) variable named
/// `name`, shadowing a module-scope one?
///
/// ⚠️ This is the guard that lets the diagnostics below be ERRORS rather than
/// warnings, and it was written because the check had a measured false positive.
/// The write walks are name-based, so
///
/// ```text
///   int n = 7;                       // module scope, written by nobody
///   always_ff @(posedge clk) begin
///     int n;                         // a block-local SHADOW
///     n = 3;
///   end
/// ```
///
/// reads as "written": the two `n`s are one name to them. As a warning that is a
/// nuisance; as an error it rejects RTL that iverilog, verilator and xrun all
/// accept.
///
/// ⚠️⚠️ Suppressing the DIAGNOSTIC does not make that design correct here: vita
/// prints `n=3` where both oracles print `n=7`, because v1 flattens a procedural
/// block-local onto a module net BY BARE NAME and the two coalesce. That is a
/// pre-existing silent-wrong of its own, entirely separate from the driver
/// question — and it is the reason this guard is written as "the source declares a
/// shadow", which is a fact about the SOURCE, rather than as anything about which
/// net vita happens to use.
fn declares_local_named(stmts: &[ast::Stmt], name: &str) -> bool {
    fn decl_hit(decls: &[ast::NetVarDecl], name: &str) -> bool {
        decls
            .iter()
            .any(|d| d.names.iter().any(|n| n.name.name == name))
    }
    stmts.iter().any(|st| match st {
        ast::Stmt::Block { decls, stmts, .. } | ast::Stmt::Fork { decls, stmts, .. } => {
            decl_hit(decls, name) || declares_local_named(stmts, name)
        }
        ast::Stmt::If { then_s, else_s, .. } => {
            declares_local_named(std::slice::from_ref(then_s), name)
                || else_s
                    .as_deref()
                    .is_some_and(|e| declares_local_named(std::slice::from_ref(e), name))
        }
        ast::Stmt::For { body, .. }
        | ast::Stmt::While { body, .. }
        | ast::Stmt::Repeat { body, .. }
        | ast::Stmt::Forever { body, .. } => declares_local_named(std::slice::from_ref(body), name),
        // The timing statements carry an OPTIONAL body (`@(posedge clk) begin … end`
        // is one of them, and it is the shape an `always_ff` almost always has).
        ast::Stmt::DelayCtrl { body, .. }
        | ast::Stmt::EventCtrl { body, .. }
        | ast::Stmt::Wait { body, .. } => body
            .as_deref()
            .is_some_and(|b| declares_local_named(std::slice::from_ref(b), name)),
        ast::Stmt::Case { items, .. } => items.iter().any(|it| {
            let (ast::CaseItem::Match { body, .. } | ast::CaseItem::Default { body, .. }) = it;
            declares_local_named(std::slice::from_ref(body), name)
        }),
        _ => false,
    })
}

/// Is lvalue `lv` a write of the WHOLE variable `name` — the bare identifier, with no
/// bit-select, part-select, array index or struct-member suffix?
///
/// ⚠️ This is a NARROWER question than "does this write touch `name`", which is what
/// an accept gate asks. The oracle asks this one — see [`stmt_writes_whole_ident`] for
/// the measurement.
///
/// A concat target (`{name, other} = e;`) writes all of `name`, so a bare identifier
/// inside a concat counts; a selected one inside a concat does not, by the same rule.
fn lvalue_is_whole_ident(lv: &ast::Lvalue, name: &str) -> bool {
    use ast::Lvalue as L;
    match lv {
        L::Ident(p) => p.segments.len() == 1 && p.segments[0].name == name,
        L::Concat { parts, .. } => parts.iter().any(|p| lvalue_is_whole_ident(p, name)),
        // A PARTIAL write: `name[i]`, `name[a:b]`, `name[i +: w]`. The packed-struct
        // member spellings (`s.x`) reach here too — the parser desugars a member access
        // to a part-select on the container.
        L::BitSelect { .. } | L::PartSelect { .. } | L::IndexedPart { .. } => false,
        L::Error(_) => false,
    }
}

/// Does this statement tree write the WHOLE of `name` — an assignment whose lvalue is
/// the bare identifier, reached through the control-flow statements?
///
/// Two narrowings separate this from [`stmt_never_writes_ident`], and each is a
/// measurement against verilator 5.052 `--lint-only`, not a judgement.
///
/// ⚠️ **Only an lvalue, never a call actual.** `stmt_never_writes_ident` is the
/// definite-assignment analysis's walk, built for ACCEPT GATES where over-approximating
/// a write is the safe direction: it treats a name appearing at any CALL ACTUAL as a
/// possible write, because the formal might be an `output`. Here the answer produces a
/// diagnostic, so over-approximation is a false report —
///
/// ```text
///   logic [3:0] tk;
///   task automatic wt(); tk = 3; endtask   // writes `tk` through the task BODY
///   always_ff @(posedge clk) tk <= 1;
///   initial wt();                          // …and `foo(tk)` with an INPUT formal
/// ```
///
/// verilator does not call that pair MULTIDRIVEN, and `foo(tk)` where `tk` binds an
/// `input` formal is not a write at all.
///
/// ⚠️⚠️ **Only a WHOLE write.** Measured, verilator is silent on every pair where at
/// least one side writes a PART of the variable, whatever the two procedures are:
///
/// ```text
///   always_ff s.x <= d;  always_ff s.y <= d;          silent  (struct members)
///   always_ff mem[a] <= 1;  always_ff mem[a+1] <= 2;  silent  (array elements)
///   always_ff bits[0] <= d;  always_ff bits[1] <= d;  silent  (bit selects)
///   initial for (i..) m1[i]=0;  always_ff m1[a] <= 1; silent  (loop over elements)
///   initial m2 = '{default:0};  always_ff m2[a]<=1;   silent  (one side partial)
///   initial m3[0] = 0;  always_ff m3 <= '{default:1}; silent  (the other side)
///   always_ff w <= 1;  initial w[1] = 0;              silent
///   initial $readmemh(.., rm);  always_ff rm[a] <= 1; silent
///   always_ff fr <= 1;  initial force fr = 4'd2;      MULTIDRIVEN  (both whole)
///   always_ff pca <= 1;  initial assign pca = 3;      MULTIDRIVEN  (both whole)
/// ```
///
/// So a writer counts only when its lvalue is the bare identifier. `force` and a
/// procedural `assign` on a bare identifier are whole writes and do count.
///
/// QUEUED: the call-actual refinement — verilator counts an actual bound to an
/// output/inout formal as a write, and does not count one bound to an `input` formal
/// or a write made inside a callee's own body. Neither this walk nor
/// [`stmt_never_writes_ident`] draws that line; resolving the formal's direction here
/// is the follow-on slice. Rule A keeps the conservative walk in the meantime, which
/// is what holds the measured `always_comb bump(acc)` inout cell.
///
/// The match is `_`-free over `ast::Stmt` so a future statement form with a write
/// position is a compile error here rather than a silent blind spot.
fn stmt_writes_whole_ident(s: &ast::Stmt, name: &str) -> bool {
    use ast::Stmt::*;
    let sub = |s: &ast::Stmt| stmt_writes_whole_ident(s, name);
    match s {
        // `x = e;` / `x <= e;` — including the assign-with-timing spellings
        // (`x = #3 e;`, `x <= @(posedge c) e;`), which are the same statement with a
        // delay / intra-assignment event attached. `x++`, `x--` and `x += e` are
        // parsed as `Blocking` too (there is no separate AST node), so they land here.
        Blocking { lhs, .. } | NonBlocking { lhs, .. } => lvalue_is_whole_ident(lhs, name),
        // Procedural continuous assign / force — a driver on `name` in the same sense,
        // and both are MULTIDRIVEN in the measurement above.
        Assign { lhs, .. } | Force { lhs, .. } => lvalue_is_whole_ident(lhs, name),
        Block { stmts, .. } | Fork { stmts, .. } => stmts.iter().any(sub),
        If { then_s, else_s, .. } => sub(then_s) || else_s.as_deref().is_some_and(sub),
        Case { items, .. } => items.iter().any(|it| {
            let (ast::CaseItem::Match { body, .. } | ast::CaseItem::Default { body, .. }) = it;
            sub(body)
        }),
        For {
            init, step, body, ..
        } => sub(init) || sub(step) || sub(body),
        While { body, .. } | Repeat { body, .. } | Forever { body, .. } => sub(body),
        Wait { body, .. } | DelayCtrl { body, .. } | EventCtrl { body, .. } => {
            body.as_deref().is_some_and(sub)
        }
        DeferredAssert { then_s, else_s, .. } => sub(then_s) || sub(else_s),
        ConcurrentAssert { pass, fail, .. } => {
            pass.as_deref().is_some_and(sub) || fail.as_deref().is_some_and(sub)
        }
        // No whole-lvalue write position. `Deassign`/`Release` end a drive rather than
        // starting one; a task / system-task / randomize CALL writes only through a
        // formal, which is exactly the imprecision this walk exists to avoid —
        // `initial $readmemh("x.hex", rm);` beside `always_ff rm[a] <= 1;` is one of
        // the measured silent cells.
        Deassign { .. }
        | Release { .. }
        | Return { .. }
        | UserTaskCall { .. }
        | RandomizeWith { .. }
        | SysTaskCall { .. }
        | EventTrigger { .. }
        | Disable { .. }
        | WaitFork { .. }
        | CoverProperty { .. }
        | Null(_)
        | Error(_) => false,
    }
}

/// One module-scope writer of a variable, as it is named in a diagnostic.
struct Writer {
    /// The writer as a diagnostic renders it, backticks included: ``` `always_ff` ```,
    /// ``` `initial` ```, "a continuous `assign`". Carrying the quoting here is what
    /// keeps a nested-backtick spelling out of the message.
    what: &'static str,
    /// The IEEE clause for an `always_comb`/`always_latch`/`always_ff` writer;
    /// `None` for every other writer kind.
    clause: Option<&'static str>,
    span: ast::Span,
}

fn proc_writer(kind: ast::ProcKind, span: ast::Span) -> Writer {
    // `_`-free so that adding a procedure kind is a forced decision here.
    let (what, clause) = match kind {
        ast::ProcKind::AlwaysComb => ("`always_comb`", Some("9.2.2.2")),
        ast::ProcKind::AlwaysLatch => ("`always_latch`", Some("9.2.2.3")),
        ast::ProcKind::AlwaysFf => ("`always_ff`", Some("9.2.2.4")),
        ast::ProcKind::Always => ("`always`", None),
        ast::ProcKind::Initial => ("`initial`", None),
        ast::ProcKind::Final => ("`final`", None),
    };
    Writer { what, clause, span }
}

impl Elaborator<'_> {
    /// The IEEE §9.2.2.x single-driver check over one module body.
    ///
    /// A pure AST pass, run once before any lowering reorders the body. See the
    /// module documentation for the oracle table that fixes each row's severity.
    ///
    /// ⚠️ Module scope only. A variable or procedure inside a `generate` is NOT
    /// walked: a generate block is its own scope and a `generate for` is
    /// instantiated once per iteration, so folding its names into this flat
    /// bare-name set would make two different variables in two different generate
    /// blocks read as one — a false error, which is the one outcome this check must
    /// not produce. Covering them needs the per-instance scope the elaborator builds
    /// later, not this pass.
    /// Does a process body with NO direct write of `name` reach a WHOLE write of
    /// it through a call (a callee body, transitively)? The direct walk (`Inert`
    /// for every call) decides the direct cases; this only adds the call-borne
    /// ones, so every design without such a call is byte-identical.
    ///
    /// Only an UNCONDITIONAL call counts — a task enable at the top level of the
    /// block (through plain nested `begin … end`), not one under an `if` / `case`
    /// / loop. That is verilator's MULTIDRIVEN table, measured: two blocks each
    /// enabling a writing task are flagged, but `always_comb begin if (src > 3)
    /// tw(src); end` beside `always_comb tw(src);` is not (both oracles run it,
    /// `ACC=8`), while the DIRECT conditional twin (`if (src > 3) acc = 5;`) IS
    /// flagged — and vita's direct walk already counts that one.
    fn proc_writes_whole_via_call(&self, body: &[ast::Stmt], name: &str) -> bool {
        let ignore = |_: &ast::HierPath, _: &[ast::Expr], _: &str| crate::da::CallEffect::Inert;
        if !stmt_never_writes_ident(body, name, Some(&ignore)) {
            return false;
        }
        body.iter()
            .any(|st| self.top_level_call_writes_whole(st, name))
    }

    fn top_level_call_writes_whole(&self, st: &ast::Stmt, name: &str) -> bool {
        match st {
            ast::Stmt::Block { stmts, .. } => stmts
                .iter()
                .any(|s| self.top_level_call_writes_whole(s, name)),
            ast::Stmt::UserTaskCall {
                name: callee, args, ..
            } => matches!(
                self.call_body_writes_whole(callee, args, name, 8),
                crate::da::CallEffect::Writes
            ),
            _ => false,
        }
    }

    /// `Writes` iff `callee(args)` PROVABLY writes `name` whole: an `output` /
    /// `inout` actual (`call_out_actual_writes`), or a resolved single-segment
    /// callee whose body assigns `name` whole, directly or through a further call
    /// (depth-bounded). A hierarchical or unresolvable callee, a callee that
    /// declares a formal / local named `name` (a shadow) and a partial write
    /// answer `Inert` — Rule B is additive, only proofs count.
    fn call_body_writes_whole(
        &self,
        callee: &ast::HierPath,
        args: &[ast::Expr],
        name: &str,
        depth: u32,
    ) -> crate::da::CallEffect {
        use crate::da::CallEffect as E;
        if self.call_out_actual_writes(callee, args, name) {
            return E::Writes;
        }
        if callee.segments.len() != 1 || depth == 0 {
            return E::Inert;
        }
        let nm = callee.segments[0].name.as_str();
        let (body, decls, ports): (&ast::Stmt, &[ast::NetVarDecl], &[ast::TfPort]) =
            if let Some(f) = self.lookup_func(nm) {
                (&f.body, &f.body_decls, &f.ports)
            } else if let Some(t) = self.lookup_task(nm) {
                (&t.body, &t.body_decls, &t.ports)
            } else {
                return E::Inert;
            };
        if decls
            .iter()
            .flat_map(|d| d.names.iter())
            .any(|n| n.name.name == name)
            || ports.iter().any(|p| p.name.name == name)
        {
            return E::Inert;
        }
        if stmt_writes_whole_ident(body, name) {
            return E::Writes;
        }
        let ignore = |_: &ast::HierPath, _: &[ast::Expr], _: &str| crate::da::CallEffect::Inert;
        if !stmt_never_writes_ident(std::slice::from_ref(body), name, Some(&ignore)) {
            return E::Inert;
        }
        let via = |cn: &ast::HierPath, a: &[ast::Expr], n: &str| {
            self.call_body_writes_whole(cn, a, n, depth - 1)
        };
        if !stmt_never_writes_ident(std::slice::from_ref(body), name, Some(&via)) {
            E::Writes
        } else {
            E::Inert
        }
    }

    pub(crate) fn check_multidriver_processes(&mut self, body: &[ast::ModuleItem]) {
        // Every module-scope VARIABLE declaration, in declaration order: the name,
        // the decl span, and whether it carries an initializer. A `wire` initializer
        // is a continuous assign, not this.
        let mut vars: Vec<(String, ast::Span, bool)> = Vec::new();
        for item in body {
            if let ast::ModuleItem::NetVar(d) = item {
                if !netvar_kind_is_var(d.kind) {
                    continue;
                }
                for n in &d.names {
                    vars.push((n.name.name.clone(), d.span, n.init.is_some()));
                }
            }
        }
        if vars.is_empty() {
            return;
        }
        let procs: Vec<&ast::ProceduralBlock> = body
            .iter()
            .filter_map(|it| match it {
                ast::ModuleItem::Proc(p) => Some(p),
                _ => None,
            })
            .collect();
        // Rule A's verdicts, computed first under a shared borrow: the conservative
        // walk is handed the module's call resolver (`call_effect`, the same closure
        // the block-local gate threads into the definite-assignment walk), so an
        // actual bound to an `input` formal is a READ and one bound to an
        // `output`/`inout` formal — or an unresolvable callee — stays a write.
        // Without the resolver every user-call actual counted: `always_comb a6 =
        // id8(u8) * b8;` beside `logic [7:0] u8 = 8'hF7` was E3001 where both oracles
        // print `f609`, while the measured `always_comb bump(acc)` (`inout`) cell must
        // keep its loud. The diagnostics are emitted in the loop below, which needs
        // `&mut self`, hence the two phases.
        //
        // A DRIVER is an actual, not a body write. `call_effect` answers `Unknown` for
        // a resolvable callee whose BODY writes the net by name (`task tw(input int
        // v); acc = v + 1;`), and Rule A took that as a second driver: `always_comb
        // tw(src);` beside `int acc = 0` was E3001 where both oracles run (`ACC=8`,
        // nested callee `ACC=9`) and verilator's MULTIDRIVEN names the `inout` actual
        // shape only (§3.b `mdrv-body-write`). So an `Unknown` whose ACTUALS are all
        // input reads (`call_actuals_only_read`: single-segment, resolved, every
        // mention of the net at an `input` formal) is a READ here; a hierarchical or
        // unresolvable callee, and any `output` / `inout` actual, stay conservative.
        let rule_a_fires: std::collections::BTreeSet<String> = {
            let me: &Self = &*self;
            let out = |cn: &ast::HierPath, args: &[ast::Expr], nm: &str| match me
                .call_effect(cn, args, nm)
            {
                crate::da::CallEffect::Unknown if me.call_actuals_only_read(cn, args, nm) => {
                    crate::da::CallEffect::Reads
                }
                v => v,
            };
            vars.iter()
                .filter(|(name, _, has_init)| {
                    *has_init
                        && procs.iter().any(|p| {
                            p.kind == ast::ProcKind::AlwaysComb
                                && !declares_local_named(std::slice::from_ref(&*p.body), name)
                                && !stmt_never_writes_ident(
                                    std::slice::from_ref(&*p.body),
                                    name,
                                    Some(&out),
                                )
                        })
                })
                .map(|(name, _, _)| name.clone())
                .collect()
        };
        // Rule B's writers through a CALLEE BODY, computed in the same phase. The
        // whole-variable walk below sees lvalues only, so two `always_comb` blocks
        // that each reach `acc` through a task body (`always_comb tw(src);` /
        // `always_comb tw2(src2);`) were two drivers nobody counted — Rule A's
        // initializer gate had been the only thing catching that shape, and the
        // review of §4.5.505 measured it going loud → order-resolved (`22` / `8`
        // by source order; verilator MULTIDRIVEN, the oracles split). A process
        // with NO direct write of `name` whose calls PROVABLY write it whole
        // (`call_body_writes_whole`, transitively, bounded) is a whole writer
        // here; an unresolvable callee proves nothing and adds nothing.
        let via_call_writers: std::collections::BTreeSet<(String, usize)> = {
            let me: &Self = &*self;
            let mut set = std::collections::BTreeSet::new();
            for (name, _, _) in &vars {
                for (pi, p) in procs.iter().enumerate() {
                    if !declares_local_named(std::slice::from_ref(&*p.body), name)
                        && me.proc_writes_whole_via_call(std::slice::from_ref(&*p.body), name)
                    {
                        set.insert((name.clone(), pi));
                    }
                }
            }
            set
        };
        let cont_assigns: Vec<&ast::ContinuousAssign> = body
            .iter()
            .filter_map(|it| match it {
                ast::ModuleItem::ContAssign(ca) => Some(ca),
                _ => None,
            })
            .collect();

        for (name, decl_span, has_init) in vars {
            // ⚠️ The SHADOW guard runs first in EVERY rule below, and it is what makes
            // an error defensible: a procedure that declares its own `name` is writing
            // THAT one, and no name-based walk can tell them apart. Skipping the whole
            // procedure — rather than just its declaring block — is the conservative
            // direction for a diagnostic that stops the run.
            let visible = |p: &&ast::ProceduralBlock| {
                !declares_local_named(std::slice::from_ref(&*p.body), &name)
            };

            // ── Rule A: a declaration initializer plus `always_comb`. ──────────────
            //
            // Runs FIRST and, when it fires, owns the variable: a variable with an
            // initializer and two `always_comb` writers is ONE diagnostic about the
            // initializer, not that one plus a Rule B line at the same caret.
            //
            // ⚠️ This rule keeps the CONSERVATIVE `stmt_never_writes_ident` walk that
            // it has always used, deliberately not the direct-write walk Rules B and C
            // use. Measured, verilator 5.052 reports MULTIDRIVEN for
            // `int acc = 0; task bump(inout int v); …; always_comb bump(acc);` — an
            // actual bound to an `inout` formal IS a driver — and narrowing this rule
            // to lvalue roots dropped that cell. Rules B and C cannot take the same
            // walk, because it also counts an actual bound to an `input` formal, which
            // verilator does not.
            if has_init && rule_a_fires.contains(&name) {
                self.error_at(
                    MsgCode::ElabMultidriver,
                    decl_span,
                    &format!(
                        "variable `{name}` has a declaration initializer AND is written by \
                         `always_comb`, which is two drivers on one variable (IEEE §9.2.2.2) \
                         — drop the initializer or the `always_comb` write"
                    ),
                );
                continue;
            }

            // The WHOLE-variable writers of `name`, in body order — see
            // `stmt_writes_whole_ident` for why this walk and not the other one.
            //
            // A via-call writer joins only the `always_comb` × `always_comb` pair —
            // verilator's table, measured: two comb blocks enabling a writing task
            // are MULTIDRIVEN, while a comb enable beside an `always_ff` direct
            // write, a comb direct write beside an `initial` enable, and two
            // `always_latch` enables are not flagged (and the `initial wt();` +
            // `always_ff` pin in `multidriver_process_pairs.rs` is silent).
            let comb_writers = procs
                .iter()
                .enumerate()
                .filter(|(pi, p)| {
                    p.kind == ast::ProcKind::AlwaysComb
                        && visible(p)
                        && (stmt_writes_whole_ident(&p.body, &name)
                            || via_call_writers.contains(&(name.clone(), *pi)))
                })
                .count();
            let mut writers: Vec<Writer> = Vec::new();
            for (pi, p) in procs.iter().enumerate() {
                if !visible(p) {
                    continue;
                }
                let via = p.kind == ast::ProcKind::AlwaysComb
                    && comb_writers >= 2
                    && via_call_writers.contains(&(name.clone(), pi));
                if stmt_writes_whole_ident(&p.body, &name) || via {
                    writers.push(proc_writer(p.kind, p.span));
                }
            }
            for ca in &cont_assigns {
                if ca
                    .assigns
                    .iter()
                    .any(|(lhs, _)| lvalue_is_whole_ident(lhs, &name))
                {
                    writers.push(Writer {
                        what: "a continuous `assign`",
                        clause: None,
                        span: ca.span,
                    });
                }
            }
            let inferred = writers.iter().position(|w| w.clause.is_some());

            // ── Rule B: an inferring procedure plus ANY other module-scope writer. ─
            //
            // One diagnostic per variable, at the always_* procedure, naming the first
            // other writer. The SEVERITY is the oracle split, not the clause: measured,
            // verilator reports MULTIDRIVEN for the `always_comb` and `always_ff` pairs
            // and is SILENT for every `always_latch` pair, so a latch pair is the
            // warning and the other two are the error.
            if let Some(i) = inferred {
                if writers.len() > 1 {
                    let other = writers
                        .iter()
                        .enumerate()
                        .find(|(j, _)| *j != i)
                        .map(|(_, w)| w.what)
                        .expect("len > 1");
                    let (what, clause, span) = (
                        writers[i].what,
                        writers[i].clause.unwrap_or("9.2.2"),
                        writers[i].span,
                    );
                    if what == "`always_latch`" {
                        self.warn_code_at(
                            MsgCode::ElabMultidriverStrict,
                            span,
                            &format!(
                                "variable `{name}` is written by {what} AND by {other}; \
                                 xcelium rejects this as two drivers (*E,MULAXX, IEEE \
                                 §{clause}) while verilator accepts it — give the variable \
                                 one writing process"
                            ),
                        );
                    } else {
                        self.error_at(
                            MsgCode::ElabMultidriver,
                            span,
                            &format!(
                                "variable `{name}` is written by {what} AND by {other}, \
                                 which is two drivers on one variable (IEEE §{clause}) \
                                 — verilator MULTIDRIVEN / xcelium *E,MULAXX"
                            ),
                        );
                    }
                    continue;
                }
            }

            // ── Rule C: a declaration initializer plus `always_ff` / `always_latch`. ─
            //
            // Only xcelium rejects, so this is a warning — see the module doc.
            if !has_init {
                continue;
            }
            let Some(w) = inferred.map(|i| &writers[i]) else {
                continue;
            };
            let (what, clause) = (w.what, w.clause.unwrap_or("9.2.2"));
            self.warn_code_at(
                MsgCode::ElabMultidriverStrict,
                decl_span,
                &format!(
                    "variable `{name}` has a declaration initializer AND is written by \
                     {what}; xcelium rejects this as two drivers (*E,MULAXX, IEEE \
                     §{clause}) while verilator and synthesis accept the initializer as the \
                     power-on value — drop the initializer or reset explicitly"
                ),
            );
        }
    }
}
