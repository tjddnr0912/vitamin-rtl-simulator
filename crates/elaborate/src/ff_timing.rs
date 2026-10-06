//! IEEE 1800-2017 §9.2.2.4: "The always_ff procedure imposes the restriction that it
//! contains one and only one event control and no blocking timing controls."
//!
//! [`ff_body_timing`] counts what an `always_ff` BODY holds of that restriction; the
//! header `@(…)` the parser folds onto `ProceduralBlock.sensitivity` is the caller's
//! to add. Containment is by text: an `@(…)` statement and an intra-assignment `@` on a
//! non-blocking assignment (`q <= @(e) d`, an `event_control` by the grammar) are event
//! controls; one inside a `fork` branch is kept apart, because a forked process cannot be
//! the procedure's one event control. A called task's body is not walked.
//!
//! [`FfBodyTiming::satisfies`] is the one predicate. [`always_ff_lane`] routes the two
//! header shapes that satisfy it without an edge list to the `always` lane spelled the
//! same way:
//!
//! - no header and the ONE event control an `@(…)` statement that every pass of the body
//!   reaches — outside any `if`, `case`, loop body or `fork`, after no loop or `disable`
//!   (`always_ff begin @(posedge clk) q <= d; end`; [`ff_body_timing`] lists the positions) —
//!   is the self-timed `always`;
//! - `always_ff @*` with no event control in the body is `always @*`.
//!
//! MEASURED (iverilog 13.0, verilator 5.052, sv2v 0.0.13 → iverilog): both print the
//! plain-`always` twin's `1 0 1 1 1`, where the edge lane armed nothing and printed
//! `x` on every line under W3056.
//!
//! An `@` inside a `fork` never routes: `always_ff begin fork @(posedge clk) q <= d;
//! join_none end` as a self-timed `always` forks a child per pass without suspending,
//! and its plain-`always` twin grows without bound (1.5 GB in 1.44 s; `join_any` the
//! same in 4.8 s). Nor does an intra-assignment `@` alone: `always_ff q <= @(posedge clk)
//! d;` would loop at its time step scheduling assignments. Nor does an `@` some pass can
//! miss: `always_ff begin if (en) @(posedge clk) q <= d; q2 <= d; end` schedules `q2 <= d`
//! on every pass without suspending while `en` is 0 — its plain-`always` twin reaches
//! 1.5 GB in 0.69 s, and with four such assignments 6 GB before the body-step limit fires
//! (it bounds steps, not memory) — and `repeat (cnt) @(…);` with `cnt` 0 the same. The
//! rule is the POSITIVE set of positions in [`ff_body_timing`], not a list of the bad ones.
//!
//! Every other `always_ff` keeps the edge lane it had (W3056, armed on nothing). `always_ff
//! do @(posedge clk) q <= d; while (0);` has one event control, as written, and runs as
//! its twin (`q=1`, as verilator).

use super::*;

/// An event control of an `always_ff` body outside any `fork`, by kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FfEvent {
    /// An `@(…) S` statement: it suspends the process. `every_pass`: it sits in a
    /// position every pass of the body reaches (see [`ff_walk`]).
    Stmt { every_pass: bool },
    /// An intra-assignment `@` (or `repeat (n) @`) on a non-blocking assignment,
    /// `q <= @(e) d`: an `event_control` by the grammar of IEEE 1800-2017 §10.4.2,
    /// which does not suspend the process.
    NbaIntra,
}

/// What an `always_ff` body holds of §9.2.2.4's restriction, in source order.
#[derive(Clone, Debug, Default)]
pub(crate) struct FfBodyTiming {
    /// Every event control outside a `fork`: its span and kind.
    pub(crate) events: Vec<(ast::Span, FfEvent)>,
    /// Every event control inside a `fork` branch, at any depth.
    pub(crate) fork_events: Vec<ast::Span>,
    /// The first blocking timing control: its span and what it is.
    pub(crate) blocking: Option<(ast::Span, &'static str)>,
    /// Statements seen so far that can keep what follows them from being reached:
    /// a loop (`for`, `while`, `repeat`, `forever`, a `do … while`'s `while`) and a
    /// `disable` (`break` and `continue` lower to one) or `return`.
    stoppers: usize,
}

impl FfBodyTiming {
    fn block(&mut self, span: ast::Span, what: &'static str) {
        if self.blocking.is_none() {
            self.blocking = Some((span, what));
        }
    }

    fn event(&mut self, span: ast::Span, kind: FfEvent, in_fork: bool) {
        if in_fork {
            self.fork_events.push(span);
        } else {
            self.events.push((span, kind));
        }
    }

    /// The predicate [`always_ff_lane`] asks: does a body with this timing, under a
    /// header (`header`) or none, satisfy §9.2.2.4 as vita reads it? Exactly one event
    /// control outside a `fork` — the header, or with no header an `@(…)` statement
    /// that every pass of the body reaches — none inside a `fork`, and no blocking timing
    /// control.
    pub(crate) fn satisfies(&self, header: bool) -> bool {
        let one = matches!(
            (header, self.events.as_slice()),
            (true, []) | (false, [(_, FfEvent::Stmt { every_pass: true })])
        );
        one && self.fork_events.is_empty() && self.blocking.is_none()
    }
}

/// Count the event controls and find the first blocking timing control in an
/// `always_ff` body. Every statement kind is named: a new kind is a compile error here,
/// not a silent miss.
///
/// - An event control: an `@(…) S` statement, and an intra-assignment `@` on a
///   non-blocking assignment. Inside a `fork` branch (`join`, `join_any`,
///   `join_none`, at any depth) it is kept apart, in `fork_events`.
/// - Blocking timing: a `#` delay statement, `wait`, `wait fork`, and a BLOCKING
///   assignment with an intra-assignment `#` or `@`. A non-blocking `q <= #1 d` does
///   not block the process and is neither.
/// - Recursed like `stmt_has_timing` (the `final` check's walker): blocks, `fork`
///   branches, `if`, `case`, loops, and the body of a `#` / `@` / `wait` statement. The
///   parser lowers `do S while (c)` to `begin S; while (c) S; end` with a CLONE of
///   `S`; the clone is not walked, so the count is the source's.
/// - Every pass (`every_pass`), a POSITIVE set, each arm saying whether its children
///   keep it: the body itself; a statement of a `begin … end` (named or not) that is
///   itself reached every pass and that no earlier statement of the block can stop
///   short of (a loop, a `disable`, `break`, `continue`, `return`); the first statement
///   of a `do … while`; the body of a `forever`; the body of a `#` / `@` / `wait`
///   statement. Not kept: `if` / `else`, `case` arms, the bodies of `for`, `while` and
///   `repeat` (a `foreach` is a `for`), `fork` branches.
/// - Not recursed: a task call (its body is the task's), and the action blocks of an
///   assertion or `cover` (the assertion's, not the process's).
pub(crate) fn ff_body_timing(s: &ast::Stmt) -> FfBodyTiming {
    let mut t = FfBodyTiming::default();
    ff_walk(s, &mut t, false, true);
    t
}

/// The parser's `do S while (c)`: `Block { S, While { S } }`, the block and the
/// `while` on the one span, the `while` body equal to `S` — spans included, so a
/// written `begin S; while (c) S; end` (two spans for `S`, and the `while`'s own)
/// never matches. `hdl-parser/src/stmt_ctl.rs`, the `do` arm.
fn is_do_while(stmts: &[ast::Stmt], span: ast::Span) -> bool {
    matches!(
        stmts,
        [first, ast::Stmt::While { body, span: ws, .. }] if *ws == span && **body == *first
    )
}

/// One step of [`ff_body_timing`]: `in_fork` — `s` is inside a `fork` branch;
/// `every_pass` — every pass of the body reaches `s` (the positive set in
/// [`ff_body_timing`]'s doc).
fn ff_walk(s: &ast::Stmt, t: &mut FfBodyTiming, in_fork: bool, every_pass: bool) {
    use ast::Stmt as S;
    match s {
        S::Blocking {
            delay, event, span, ..
        } => {
            if delay.is_some() {
                t.block(
                    *span,
                    "an intra-assignment delay on a blocking assignment (`= #…`)",
                );
            } else if event.is_some() {
                t.block(
                    *span,
                    "an intra-assignment event control on a blocking assignment (`= @(…)`)",
                );
            }
        }
        S::NonBlocking { event, span, .. } => {
            if event.is_some() {
                t.event(*span, FfEvent::NbaIntra, in_fork);
            }
        }
        S::DelayCtrl { body, span, .. } => {
            t.block(*span, "a `#` delay");
            if let Some(b) = body {
                ff_walk(b, t, in_fork, every_pass);
            }
        }
        S::EventCtrl { body, span, .. } => {
            t.event(*span, FfEvent::Stmt { every_pass }, in_fork);
            if let Some(b) = body {
                ff_walk(b, t, in_fork, every_pass);
            }
        }
        S::Wait { body, span, .. } => {
            t.block(*span, "a `wait`");
            if let Some(b) = body {
                ff_walk(b, t, in_fork, every_pass);
            }
        }
        S::WaitFork { span } => t.block(*span, "a `wait fork`"),
        // A sequential list keeps `every_pass` until a statement that can stop short of
        // the rest. A `do … while` walks its body once, and its `while` is a loop.
        S::Block { stmts, span, .. } => {
            let do_while = is_do_while(stmts, *span);
            let walked = if do_while { &stmts[..1] } else { &stmts[..] };
            let mut reached = every_pass;
            for st in walked {
                let before = t.stoppers;
                ff_walk(st, t, in_fork, reached);
                if t.stoppers != before {
                    reached = false;
                }
            }
            if do_while {
                t.stoppers += 1;
            }
        }
        S::Fork { stmts, .. } => {
            for st in stmts {
                ff_walk(st, t, true, false);
            }
        }
        S::If { then_s, else_s, .. } => {
            ff_walk(then_s, t, in_fork, false);
            if let Some(e) = else_s {
                ff_walk(e, t, in_fork, false);
            }
        }
        S::Case { items, .. } => {
            for it in items {
                ff_walk(case_item_body(it), t, in_fork, false);
            }
        }
        S::For {
            init, step, body, ..
        } => {
            t.stoppers += 1;
            ff_walk(init, t, in_fork, false);
            ff_walk(step, t, in_fork, false);
            ff_walk(body, t, in_fork, false);
        }
        S::While { body, .. } | S::Repeat { body, .. } => {
            t.stoppers += 1;
            ff_walk(body, t, in_fork, false)
        }
        S::Forever { body, .. } => {
            t.stoppers += 1;
            ff_walk(body, t, in_fork, every_pass)
        }
        // `disable fork` ends child processes, not this one's control flow.
        S::Disable { target, .. } => {
            if !matches!(target.segments.as_slice(), [seg] if seg.name == "fork") {
                t.stoppers += 1;
            }
        }
        S::Return { .. } => t.stoppers += 1,
        S::SysTaskCall { .. }
        | S::UserTaskCall { .. }
        | S::RandomizeWith { .. }
        | S::EventTrigger { .. }
        | S::ConcurrentAssert { .. }
        | S::Assign { .. }
        | S::Deassign { .. }
        | S::Force { .. }
        | S::Release { .. }
        | S::DeferredAssert { .. }
        | S::CoverProperty { .. }
        | S::Null(_)
        | S::Error(_) => {}
    }
}

/// The lane an `always_ff` lowers through: `Always` for the two shapes that satisfy
/// §9.2.2.4 without an edge list (see the module doc), its own kind for everything
/// else, and any other kind unchanged. The block keeps its own kind everywhere the
/// lane is not asked: its process label, and the single-writer check of §9.2.2.4.
/// [`FfBodyTiming::satisfies`] decides it.
pub(crate) fn always_ff_lane(p: &ast::ProceduralBlock) -> ast::ProcKind {
    if !matches!(p.kind, ast::ProcKind::AlwaysFf)
        || matches!(p.sensitivity, Some(ast::Sensitivity::List(_)))
    {
        return p.kind;
    }
    if ff_body_timing(&p.body).satisfies(p.sensitivity.is_some()) {
        ast::ProcKind::Always
    } else {
        p.kind
    }
}
