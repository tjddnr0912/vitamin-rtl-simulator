//! The process-header level list that names a CONSTANT beside a LIVE term:
//! `always @(K or clk)`, `always @(K[0] or clk)`, `always @(p::C or clk)`.
//!
//! IEEE 1800 §9.4.2: an `always @(list)` process is armed on its list before
//! time 0, and a level term fires on a CHANGE. The time-0 settle hands the
//! process an initial change of every constant term, so the process runs ONCE at
//! time 0 and then on every change of the live terms. MEASURED, iverilog 13.0 and
//! verilator 5.052 agree:
//!
//! - `localparam int K = 99; always @(K) $display("LVL at %0t", $time);` prints
//!   `LVL at 0` and nothing more;
//! - `always @(K or clk)` with `clk` rising at 1 and falling at 2 prints
//!   `MIX at 0` / `MIX at 1` / `MIX at 2`, where the header lane printed only 1 / 2
//!   (the constant term was dropped with its time-0 fire, at exit 0);
//! - the same holds for every constant kind (parameter incl. a per-instance
//!   override, localparam, genvar, enum label, `$unit`, real, string, generate
//!   scope, `import p::*`, `import p::C`, `p::C`), for constant selects (`K[0]`,
//!   `K[3:0]`, `K[0+:2]`, `K[g]`) and for a generate-scope constant shadowing a net.
//!
//! The lane keeps the header `Level` process EXACTLY — the same body, the same
//! live terms through the same `sens_event_net` — and replaces the constant terms
//! with ONE `AnyEdge` term on the design's time-0 pulse: an internal 1-bit wire,
//! z-initialised, driven by a continuous assign of `1'b1`. The time-0 settle moves
//! it z→1 before the processes are armed; that change survives arming and fires
//! the waiter at the first propagate of time 0, in the ACTIVE region after the
//! first batch of `initial` statements, and the pulse never changes again.
//! MEASURED on PRE with the spelling `wire kw; assign kw = 1'b1;` in place of the
//! constant (18 cells, order and count cells included): every one printed
//! iverilog's text, and both oracles' wherever they agree.
//!
//! ⚠️ Not a `#0` in front of the body: an `initial` declared BEFORE the `always`
//! whose own `#0` continuation reads or writes what the body touches runs first
//! then, where both oracles run the body first (`initial begin #0 $display(y);
//! end` + `always @(K) y = 7;` prints `y=7`). And not the in-body wait: vita's
//! in-body level wait does not see a same-step 1→0→1 glitch that the header
//! waiter sees.
//!
//! ⚠️ Admitted only when the body cannot SUSPEND the process
//! ([`Elaborator::body_suspend_blocker`]): with a suspending body iverilog still
//! runs it at time 0 and verilator does not. Every declined list keeps the header
//! lane — a constant term beside a live one is dropped, and an all-constant list is
//! refused (`error_header_level_const`) with the reason ([`T0Decline`]).
//!
//! A list with NO live term (`always @(K)`, `@(K[0])`, `@(p::C)`, `@(K or K2)`) is
//! admitted the same way: its sensitivity is the pulse alone, so the process runs
//! once at time 0 and never again, as both oracles do. A `$finish` reaching time 0
//! through any channel no longer erases that run: a `$finish` ends the run at the
//! END of its time step (`sched/run_loop.rs`), so the process the pulse woke still
//! runs. MEASURED: `initial $finish;` in the first batch, in a child module, after
//! `#0`, in a task, behind `-> ev`, and inside a sibling `always @(K)` body — the
//! time-0 run prints in vita and in both oracles wherever iverilog does not halt it
//! at its first system-task call (vvp halts every OTHER thread there once a
//! `$finish` is pending; a self-contradiction that disqualifies it on those cells).
//!
//! ⚠️ Only a USER-written block: `lower_proc_block`'s `user_written` is set by
//! `lower_user_proc` alone. A block vita synthesizes (an SVA checker, a covergroup
//! sampler, a clocking commit, a declaration-initializer flush) keeps the header
//! lane byte for byte — its clock is not a list the oracles were measured on.

use super::*;

/// `ProcIdent::kind` of the pulse's continuous assign. The OBS profile skips the
/// row by this producer label (`cli/src/obs.rs`); nothing a user writes carries it.
pub const T0_PULSE_KIND: &str = "t0_pulse";

/// Why a process-header level list naming a constant does NOT take the time-0
/// lane — the tail of the header lane's refusal when nothing live is left.
#[derive(Clone, Debug)]
pub(crate) enum T0Decline {
    /// Not a header level list with a constant term: the refusal cannot see it.
    NotACandidate,
    /// The clock of an assertion / covergroup / clocking block vita builds.
    Synthesized,
    /// An `iff` guard on a term.
    Iff,
    /// `always_ff`.
    AlwaysFf,
    /// An edge term beside the level ones.
    EdgeTerm,
    /// The body holds `what`. `split`: MEASURED, iverilog runs such a body at time 0
    /// and verilator does not; otherwise the construct is simply not admitted.
    Body { what: String, split: bool },
}

impl T0Decline {
    /// The clause a refusal ends with: why THIS block was declined.
    pub(crate) fn clause(&self) -> String {
        match self {
            T0Decline::NotACandidate => "this is not a process-header level list".into(),
            T0Decline::Synthesized => {
                "this list is the clock of an assertion, covergroup or clocking block".into()
            }
            T0Decline::Iff => "this list carries an `iff` guard".into(),
            T0Decline::AlwaysFf => "this block is an `always_ff`".into(),
            T0Decline::EdgeTerm => "this list also has an edge term (verilator runs such a \
                                    process at time 0, iverilog does not)"
                .into(),
            T0Decline::Body { what, split: true } => format!(
                "the body can suspend at {what} (iverilog runs such a process at time 0, \
                 verilator does not)"
            ),
            T0Decline::Body { what, split: false } => {
                format!("the body holds {what}, which vita does not admit")
            }
        }
    }
}

impl Elaborator<'_> {
    /// Does `p` take the time-0 lane? `Ok` for an `always` the user wrote whose
    /// header is an explicit list of NON-EDGE, `iff`-free terms, not a clocking-block
    /// event, with at least one term whose head binds a constant (every term may),
    /// and whose body cannot suspend; otherwise the reason. `iff_desugared`: `p` is the rewrite of
    /// a single-term `@(… iff g)` (`desugar_event_iff`).
    pub(crate) fn header_const_level_t0(
        &self,
        p: &ast::ProceduralBlock,
        user_written: bool,
        iff_desugared: bool,
    ) -> Result<(), T0Decline> {
        let list = match (p.kind, p.sensitivity.as_ref()) {
            (ast::ProcKind::Always, Some(ast::Sensitivity::List(l))) => l,
            (ast::ProcKind::AlwaysFf, _) => return Err(T0Decline::AlwaysFf),
            _ => return Err(T0Decline::NotACandidate),
        };
        if !user_written {
            return Err(T0Decline::Synthesized);
        }
        if iff_desugared || list.iter().any(|ev| ev.iff.is_some()) {
            return Err(T0Decline::Iff);
        }
        if self.clocking_event_subst(p.sensitivity.as_ref()).is_some() {
            return Err(T0Decline::NotACandidate);
        }
        if list.iter().any(|ev| !matches!(ev.edge, ast::Edge::NoEdge)) {
            return Err(T0Decline::EdgeTerm);
        }
        if !list
            .iter()
            .any(|ev| self.expr_head_binds_constant_strict(&ev.expr))
        {
            return Err(T0Decline::NotACandidate);
        }
        if let Some(why) = self.body_suspend_blocker(&p.body, &mut Vec::new()) {
            return Err(why);
        }
        Ok(())
    }

    /// `lower_proc_block`'s sensitivity: the time-0 lane's when it admits `p`, else
    /// `lower_sensitivity`'s with the decline reason parked in `t0_decline` for the
    /// header lane's constant refusal ([`Self::error_header_level_const`]).
    pub(crate) fn proc_sensitivity(
        &mut self,
        p: &ast::ProceduralBlock,
        user_written: bool,
        iff_desugared: bool,
    ) -> ir::Sensitivity {
        match self.header_const_level_t0(p, user_written, iff_desugared) {
            Ok(()) => self.header_const_level_sensitivity(p),
            Err(why) => {
                self.t0_decline = Some(why);
                let s = self.lower_sensitivity(p.kind, p.sensitivity.as_ref(), &p.body);
                self.t0_decline = None;
                s
            }
        }
    }

    /// The sensitivity of a [`Self::header_const_level_t0`] process: the header
    /// lane's `Level` over the LIVE terms (the constant ones held aside by the same
    /// `header_level_term_is_const`) plus the time-0 pulse. An all-constant list
    /// leaves no live term, so its sensitivity is the pulse alone.
    pub(crate) fn header_const_level_sensitivity(
        &mut self,
        p: &ast::ProceduralBlock,
    ) -> ir::Sensitivity {
        let Some(ast::Sensitivity::List(list)) = p.sensitivity.as_ref() else {
            unreachable!("header_const_level_t0 admits only an explicit list")
        };
        let live: Vec<&ast::EventExpr> = list
            .iter()
            .filter(|ev| !self.header_level_term_is_const(ev))
            .collect();
        let mut edges = self.header_live_edges(&live, /* any_edge = */ false);
        edges.push(ir::EdgeTerm {
            net: self.t0_pulse_net(),
            kind: ir::EdgeKind::AnyEdge,
        });
        ir::Sensitivity {
            kind: ir::SensKind::Level,
            edges,
        }
    }

    /// Refuse a header LEVEL term that binds a constant and has no live sibling,
    /// saying which of the two constants it is.
    ///
    /// Reachable only for a list the time-0 lane (`const_level_header.rs`) declined,
    /// and the message ends with THAT block's reason (`T0Decline`, parked in
    /// `t0_decline` by `lower_proc_block`): a body that can suspend, named — MEASURED
    /// split for `#`, `@`, `wait fork`, intra-assignment `#` / `@`, `fork … join` /
    /// `join_any`, a task holding one (iverilog runs the body at time 0, verilator
    /// does not) — or holding a construct not admitted; an `iff` guard; `always_ff`;
    /// an edge sibling (split the other way); a synthesized clock.
    ///
    /// `event_term_never_wakes` deliberately answers `false` here (a header level
    /// term on a constant fires ONCE at time 0 in both oracles), and the UNSHADOWED
    /// spelling was already loud through `resolve_net`. The SHADOWED one was not: `lookup_net_scoped` walks `symbols`
    /// alone, so `generate if (1) begin : g localparam int V = 2; always @(V) …`
    /// armed the OUTER net and fired again when it changed (MEASURED `HDR fired at
    /// 0` + `HDR fired at 1`, where iverilog 13.0 and verilator 5.052 both print
    /// `HDR fired at 0` then `DONE`). One IEEE question was answered loud in one
    /// spelling and silently wrong in its shadow twin; this is the parity.
    ///
    /// The two messages are the §2 🆕 O pair: `error_const_shadows_net` when a net
    /// of the same name is in scope (the reader must know WHICH object vita took),
    /// and the plain constant sentence otherwise — replacing `resolve_net`'s `E3010
    /// undeclared net/variable`, which was a false statement about the program
    /// (`localparam int K = 99;` IS a declaration). Both end with the time-0 lane's
    /// admission rule, the cause the reader can act on. The plain sentence prints the
    /// term as written (`K[0]`, `p::C`); the shadow branch is for a BARE head only —
    /// a select's base and a `p::C` name cannot be shadowed by a net.
    ///
    /// ⚠️ ONLY when the term is the WHOLE sensitivity — see the two-pass loop in
    /// `classify_event_list`. Beside a LIVE term the constant is dropped instead,
    /// or the refusal swallows a sibling that does wake the process.
    pub(crate) fn error_header_level_const(&mut self, ev: &ast::EventExpr) {
        let why = self
            .t0_decline
            .as_ref()
            .map_or_else(|| T0Decline::NotACandidate.clause(), T0Decline::clause);
        let bare = Self::event_bare_head(&ev.expr);
        if let Some((name, _)) = bare.filter(|&(n, at)| self.bare_const_shadows_net(n, at)) {
            self.error_const_shadows_net(
                name,
                &format!(
                    "the level event control `@({name})` has no live term, so the process \
                     runs once at time 0 and never again; vita runs that only for an \
                     `always` written in the source, with level terms and no `iff` guard, \
                     whose body cannot suspend — here {why}"
                ),
            );
        } else {
            let term = event_term_text(&ev.expr);
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "a level event control on the constant `{term}` (a parameter / \
                     localparam / genvar / enum label, or a select of one) with no live term \
                     runs its process once at time 0 and never again; vita runs that only for \
                     an `always` written in the source, with level terms and no `iff` guard, \
                     whose body cannot suspend — here {why}. `posedge`/`negedge` on a \
                     constant is accepted and simply never fires"
                ),
            );
        }
    }

    /// The design's time-0 pulse net, minted on the first admission and shared by
    /// every later one: `wire $ia_tmp$<n>` (z) driven by `assign … = 1'b1;`. The
    /// `$ia_tmp$` sigil is the one the VCD/FST writer already drops (`queues_io.rs`)
    /// and no identifier can spell; its continuous assign carries [`T0_PULSE_KIND`].
    fn t0_pulse_net(&mut self) -> u32 {
        if let Some(n) = self.t0_pulse {
            return n;
        }
        let net = self.nets.len() as u32;
        self.add_net(
            &format!("$ia_tmp${net}"),
            ir::NetVar {
                kind: ir::NetKind::Wire,
                width: 1,
                msb: 0,
                lsb: 0,
                signed: false,
                array_len: 1,
                dir: ir::PortDir::Internal,
                init: default_init(ast::NetVarKind::Wire, 1),
            },
        );
        if self.nets.len() as u32 == net {
            return POISON_NET; // the net arena cap refused it (already loud)
        }
        let one = self.intern_const(make_const_u32(1, 1));
        let rhs = self.push_expr(ir::Expr::Const { val: one });
        self.push_cont_assign(
            ir::ContAssign {
                lhs: whole_net_lvalue(net),
                rhs,
                delay: None,
            },
            T0_PULSE_KIND,
            None,
        );
        self.t0_pulse = Some(net);
        net
    }

    /// The first construct of `s` that can suspend the process running it, or one
    /// this lane does not admit — `None` when there is none. `_`-free, so a new
    /// statement kind is a compile error rather than a silent admission. `callers`
    /// is the chain of task keys being walked (the recursion guard).
    ///
    /// Admitted, each MEASURED under `always @(K or clk)` with both oracles running
    /// at time 0 and at every `clk` change: a blocking or non-blocking assignment
    /// without a timing control (a function call in its value included), `if` /
    /// `case` / `for` / `while` / `repeat` over admitted bodies, a system task, `->
    /// ev`, `disable` of a named block (a loop's `break` included; not `disable
    /// fork`), `return` inside a task body, a statement
    /// function call (`void'(f(1))`), and a task enable
    /// [`Self::task_enable_blocker`] admits.
    ///
    /// Suspending, MEASURED split (iverilog runs the body at time 0, verilator does
    /// not): `#d`, `@(e)`, `wait fork`, `x = #1 y`, `x = @(e) y`, `fork … join` even
    /// without timing, `fork … join_any`. Not admitted: every `fork … join_none` (a
    /// child against a `disable` of the block splits the oracles and leaves a line
    /// neither prints), `disable fork`, `x <= #1 y` (verilator loses
    /// the write the time-0 run schedules), `x <= @(e) y` (verilator crashes),
    /// `wait (c)` (split for a variable `c`, both run for `wait (1)`), `forever`, and
    /// the kinds not measured here — `randomize() with`, procedural `assign` /
    /// `force` family, deferred and concurrent assertions, `cover`, `return` outside
    /// a task.
    pub(crate) fn body_suspend_blocker(
        &self,
        s: &ast::Stmt,
        callers: &mut Vec<String>,
    ) -> Option<T0Decline> {
        use ast::Stmt as S;
        let body = |what: &str, split: bool| {
            Some(T0Decline::Body {
                what: what.to_string(),
                split,
            })
        };
        match s {
            S::Null(_) | S::SysTaskCall { .. } | S::EventTrigger { .. } => None,
            // `disable fork` parses as a `disable` of the keyword `fork`, which no
            // block can be named: with a forked child holding `#2`, verilator skips
            // the time-0 run that iverilog makes.
            S::Disable { target, .. } => match target.segments.as_slice() {
                [seg] if seg.name == "fork" => body("`disable fork`", false),
                _ => None,
            },
            S::Blocking { delay, event, .. } => match (delay, event) {
                (Some(_), _) => body("an intra-assignment `#` delay", true),
                (_, Some(_)) => body("an intra-assignment `@` event control", true),
                (None, None) => None,
            },
            S::NonBlocking { delay, event, .. } => match (delay, event) {
                (Some(_), _) => body(
                    "a non-blocking assignment's intra-assignment `#` delay",
                    false,
                ),
                (_, Some(_)) => body(
                    "a non-blocking assignment's intra-assignment `@` event control",
                    false,
                ),
                (None, None) => None,
            },
            S::If { then_s, else_s, .. } => {
                self.body_suspend_blocker(then_s, callers).or_else(|| {
                    else_s
                        .as_deref()
                        .and_then(|e| self.body_suspend_blocker(e, callers))
                })
            }
            S::Case { items, .. } => items
                .iter()
                .find_map(|it| self.body_suspend_blocker(case_item_body(it), callers)),
            S::For {
                init,
                step,
                body: b,
                ..
            } => self
                .body_suspend_blocker(init, callers)
                .or_else(|| self.body_suspend_blocker(step, callers))
                .or_else(|| self.body_suspend_blocker(b, callers)),
            S::While { body: b, .. } | S::Repeat { body: b, .. } => {
                self.body_suspend_blocker(b, callers)
            }
            S::Block { stmts, .. } => stmts
                .iter()
                .find_map(|x| self.body_suspend_blocker(x, callers)),
            // EVERY fork: `join` / `join_any` split the oracles at time 0, and a
            // `join_none` child against a `disable` of the block or of the fork is
            // where both a split and a wrong extra line were measured.
            S::Fork { join, .. } => match join {
                ast::JoinKind::JoinNone => body("`fork … join_none`", false),
                ast::JoinKind::Join => body("`fork … join`", true),
                ast::JoinKind::JoinAny => body("`fork … join_any`", true),
            },
            S::Return { .. } if !callers.is_empty() => None,
            S::Return { .. } => body("`return`", false),
            S::UserTaskCall { name, .. } => self.task_enable_blocker(name, callers),
            S::DelayCtrl { .. } => body("a `#` delay", true),
            S::EventCtrl { .. } => body("an `@` event control", true),
            S::WaitFork { .. } => body("`wait fork`", true),
            S::Wait { .. } => body("`wait (…)`", false),
            S::Forever { .. } => body("`forever`", false),
            S::RandomizeWith { .. } => body("`randomize() with`", false),
            S::ConcurrentAssert { .. } => body("a concurrent assertion", false),
            S::DeferredAssert { .. } => body("a deferred assertion", false),
            S::CoverProperty { .. } => body("`cover property`", false),
            S::Assign { .. } | S::Deassign { .. } | S::Force { .. } | S::Release { .. } => body(
                "a procedural `assign` / `deassign` / `force` / `release`",
                false,
            ),
            S::Error(_) => body("a statement that did not parse", false),
        }
    }

    /// Admit the enable `name` only when it resolves — through the resolver the
    /// lowering uses (`UserTaskCall` arm → `inline_task`) — to a function (a
    /// statement call: a function cannot suspend) or to a MODULE-LOCAL task whose
    /// body admits transitively: one segment, `has_func` first, then
    /// `resolve_rtn_key` into `task_table`, declared in no package (`rtn_key_pkg`).
    /// MEASURED, both oracles run the process at time 0 for a static task without
    /// timing, an automatic one, a task writing an `output` formal, and a task
    /// enabling another such task; a task enabling a task with `#1` splits.
    ///
    /// ⚠️ A NESTED enable (inside a callee's body) is resolved from the process's scope
    /// here, while a framed (automatic) callee's body is lowered under its declaring
    /// prefix; the two agree unless a generate scope declares a routine, so nested
    /// enables are admitted only when this module declares none (`rtn_decl_scope`).
    /// A hierarchical `u.t`, a method, an imported task and a recursive chain keep
    /// the header lane.
    fn task_enable_blocker(
        &self,
        name: &ast::HierPath,
        callers: &mut Vec<String>,
    ) -> Option<T0Decline> {
        let text = name
            .segments
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>()
            .join(".");
        let refuse = |what: String| Some(T0Decline::Body { what, split: false });
        let [seg] = name.segments.as_slice() else {
            return refuse(format!(
                "the enable `{text}` of a task not declared in this module"
            ));
        };
        if self.has_func(&seg.name) {
            return None;
        }
        if !callers.is_empty() && !self.rtn_decl_scope.is_empty() {
            return refuse(format!(
                "the enable `{text}` inside a task, in a module that declares a \
                 generate-scoped routine"
            ));
        }
        let key = self.resolve_rtn_key(&seg.name);
        if callers.contains(&key) {
            return refuse(format!("the recursive enable of task `{text}`"));
        }
        let task = match self.task_table.get(&key) {
            Some(t) if self.rtn_key_pkg(&key).is_none() => t,
            _ => {
                return refuse(format!(
                    "the enable `{text}` of a task not declared in this module"
                ))
            }
        };
        callers.push(key);
        let inner = self.body_suspend_blocker(&task.body, callers);
        callers.pop();
        inner.map(|why| match why {
            T0Decline::Body { what, split } => T0Decline::Body {
                what: format!("{what} in task `{text}`"),
                split,
            },
            other => other,
        })
    }
}

/// An event term as written, for a diagnostic: names, `p::name`, selects with
/// literal or name indices, parentheses; any other index prints as `…`.
pub(crate) fn event_term_text(e: &ast::Expr) -> String {
    let t = event_term_text;
    match &e.kind {
        ast::ExprKind::Paren { inner } => format!("({})", t(inner)),
        ast::ExprKind::BitSelect { base, index } => format!("{}[{}]", t(base), t(index)),
        ast::ExprKind::PartSelect { base, msb, lsb } => {
            format!("{}[{}:{}]", t(base), t(msb), t(lsb))
        }
        ast::ExprKind::IndexedPart {
            base,
            offset,
            width,
            dir,
        } => {
            let op = match dir {
                ast::PartDir::PlusColon => "+:",
                ast::PartDir::MinusColon => "-:",
            };
            format!("{}[{}{op}{}]", t(base), t(offset), t(width))
        }
        ast::ExprKind::Ident(path) => path
            .segments
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>()
            .join("."),
        ast::ExprKind::PkgScoped { pkg, name } => format!("{}::{}", pkg.name, name.name),
        ast::ExprKind::IntLit { raw, .. } | ast::ExprKind::RealLit { raw, .. } => raw.clone(),
        ast::ExprKind::Unary { op, operand } => format!("{}{}", unop_text(*op), t(operand)),
        ast::ExprKind::Binary { op, lhs, rhs } => {
            format!("{} {} {}", t(lhs), binop_text(*op), t(rhs))
        }
        ast::ExprKind::Ternary {
            cond,
            then_e,
            else_e,
        } => format!("{} ? {} : {}", t(cond), t(then_e), t(else_e)),
        ast::ExprKind::Call { name, args } => {
            let head = name
                .segments
                .iter()
                .map(|s| s.name.as_str())
                .collect::<Vec<_>>()
                .join(".");
            let args = args.iter().map(t).collect::<Vec<_>>().join(", ");
            format!("{head}({args})")
        }
        ast::ExprKind::SysCall { name, args } => {
            let args = args.iter().map(t).collect::<Vec<_>>().join(", ");
            format!("{}({args})", name.name)
        }
        _ => "…".to_string(),
    }
}

fn unop_text(op: ast::UnOp) -> &'static str {
    use ast::UnOp as U;
    match op {
        U::Plus => "+",
        U::Minus => "-",
        U::LogNot => "!",
        U::BitNot => "~",
        U::RedAnd => "&",
        U::RedNand => "~&",
        U::RedOr => "|",
        U::RedNor => "~|",
        U::RedXor => "^",
        U::RedXnor => "~^",
    }
}

fn binop_text(op: ast::BinOp) -> &'static str {
    use ast::BinOp as B;
    match op {
        B::Add => "+",
        B::Sub => "-",
        B::Mul => "*",
        B::Div => "/",
        B::Mod => "%",
        B::Pow => "**",
        B::Shl => "<<",
        B::Shr => ">>",
        B::AShl => "<<<",
        B::AShr => ">>>",
        B::Lt => "<",
        B::Le => "<=",
        B::Gt => ">",
        B::Ge => ">=",
        B::Eq => "==",
        B::Ne => "!=",
        B::CaseEq => "===",
        B::CaseNe => "!==",
        B::WildEq => "==?",
        B::WildNe => "!=?",
        B::BitAnd => "&",
        B::BitXor => "^",
        B::BitXnor => "~^",
        B::BitOr => "|",
        B::LogAnd => "&&",
        B::LogOr => "||",
    }
}
