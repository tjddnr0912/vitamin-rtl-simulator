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
//! [`Elaborator::check_always_ff_timing`] refuses every `always_ff` the predicate
//! rejects, `VITA-E3061`, naming each violation ([`FfViolation`]): no event control; no
//! header and only an intra-assignment one; no header and the one `@(…)` outside the
//! every-pass positions (a rule of vita's, stricter than the IEEE text, which counts that
//! `@` as the one event control: such a block cannot run, and iverilog refuses it as "the
//! first statement of an always_ff process must be an event control statement"; owner
//! decision 2026-10-06: an error, not a warning; running it is ROADMAP §7 FF-MISSED-AT,
//! gated on §5.b NBA-GROWTH); two and more (the header plus the body's); one inside a
//! `fork`; a blocking timing control. So the edge lane's W3056 arm is reached only beside
//! that error, and says nothing.
//!
//! The tools, one shape per row (iverilog 13.0 `-g2012`; verilator 5.052 `--binary
//! --timing`; sv2v 0.0.13 → iverilog 13.0). iverilog refuses every row; verilator and
//! sv2v run most of them (each exception is noted; sv2v cannot parse `join_none`). So
//! the error is an owner ruling (2026-10-06, an exception to ER §10.4: "loud where IEEE
//! says error") for a count of 0 or of 2 and more, and the same text decides the rest:
//!
//! ```text
//!   @(posedge clk) begin q <= d; @(posedge clk) q <= ~d; end     count 2
//!     iverilog  error: an event control is not allowed in an always_comb, always_ff or
//!               always_latch process. / error: there must only be a single event control
//!               and no blocking delays in an always_ff process.
//!   begin @(posedge clk) q <= d; @(posedge clk) q <= ~d; end      count 2, no header
//!     iverilog  error: the first statement of an always_ff process must be an event
//!               control statement.
//!   @* begin @(posedge clk) q <= d; end                           count 2
//!     iverilog  error: an event control is not allowed in … / there must only be a
//!               single event control …
//!   @(posedge clk) q <= @(negedge clk) d;                         count 2 (intra `@`)
//!     iverilog  error: A non-blocking assignment cannot be synthesized with an event
//!               control in an always_ff process.
//!     sv2v      prints its own value (`q=1` at c1, where verilator and vita read the
//!               edge's `d`): it drops the intra-assignment `@`
//!   q <= d;                                                       count 0
//!     iverilog  error: the first statement of an always_ff process must be an event
//!               control statement.
//!     sv2v      then iverilog: error: always process does not have any delay.
//!   begin if (en) @(posedge clk) q <= d; q2 <= d; end   no header, `@` not every pass
//!     iverilog  error: the first statement of an always_ff process must be an event
//!               control statement.
//!     verilator spins (killed at 20 s); its plain-`always` twin in vita grows past 1.5 GB
//!   begin repeat (cnt) @(posedge clk); q2 <= d; end      the same, `cnt` 0
//!     iverilog  the "first statement" error / warning: A repeat statement cannot be
//!               synthesized in an always_ff process.
//!     verilator q=0 q2=0
//!   q <= @(posedge clk) d;                    no statement-level event control
//!     iverilog  the "first statement" error / A non-blocking assignment cannot be
//!               synthesized with an event control in an always_ff process.
//!     verilator %Error: Internal Error: …: ../V3Active.cpp:552: Should not reach here
//!               when walking body without --timing
//!     sv2v      then iverilog: error: always process does not have any delay.
//!   begin fork @(posedge clk) q <= d; join_none end               `@` in a fork
//!     iverilog  error: the first statement … / A fork/join_none statement cannot be
//!               synthesized in an always_ff process.
//!     verilator %Error-DIDNOTCONVERGE: Active region did not converge after
//!               '--converge-limit' of 10000 tries (sv2v cannot parse it)
//!   @(posedge clk) begin fork begin @(negedge clk) q2 <= d; end join_none q <= d; end
//!     iverilog  error: A fork/join_none statement cannot be synthesized in an always_ff
//!               process. (verilator runs it; sv2v cannot parse it)
//!   @(posedge clk) begin #1 q <= d; end                           `#`
//!     iverilog  error: a blocking delay is not allowed in an always_comb, always_ff or
//!               always_latch process. / error: there must only be a single event control
//!               and no blocking delays in an always_ff process.
//!   @(posedge clk) begin wait (d) q <= ~q; end                    `wait`
//!     iverilog  error: a wait statement is not allowed in … / there must only be …
//!   @(posedge clk) q = #1 d;   @(posedge clk) q = @(negedge clk) d;   blocking intra
//!     iverilog  error: a blocking delay is not allowed in … / an event control is not
//!               allowed in … / there must only be …
//!   @(posedge clk) begin fork q2 <= d; join_none wait fork; q <= d; end  `wait fork`
//!     iverilog  error: an event control is not allowed in … (sv2v cannot parse it)
//!   @(posedge clk) fork #1 q <= d; join                           `#` in a fork branch
//!     iverilog  error: a blocking delay is not allowed in …
//!   @(posedge clk) begin fork #1 q2 <= d; join_none q <= d; end   `#` in a join_none branch
//!     iverilog  error: A fork/join_none statement cannot be synthesized in an always_ff
//!               process. (sv2v cannot parse it; verilator runs it)
//! ```
//!
//! Legal and unchanged: `q <= #1 d` (a non-blocking intra-assignment delay does not
//! block); an edge list with no edge (`always_ff @(a or b)`, iverilog "Synthesis requires
//! the sensitivity list of an always_ff process to only be edge sensitive." as a
//! warning); a `fork` with no timing in it. `always_ff do @(posedge clk) q <= d; while
//! (0);` has one event control, as written, and runs as its twin (`q=1`, as verilator).
//! Not counted: an event control or delay inside a called task (iverilog counts it:
//! `always_ff @(posedge clk) begin t; q <= d; end` with `task t; #1; endtask` is "a
//! blocking delay is not allowed"; PROBE_CATALOG §4.5.597).

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

    /// The one predicate the lane ([`always_ff_lane`]) and the check
    /// ([`Elaborator::check_always_ff_timing`]) share: does a body with this timing,
    /// under a header (`header`) or none, satisfy §9.2.2.4 as vita reads it? Exactly
    /// one event control outside a `fork` — the header, or with no header an `@(…)`
    /// statement that every pass of the body reaches — none inside a `fork`, and no
    /// blocking timing control.
    pub(crate) fn satisfies(&self, header: bool) -> bool {
        self.violations(header).is_empty()
    }

    /// Every way the block breaks the rule [`Self::satisfies`] states, in report order.
    pub(crate) fn violations(&self, header: bool) -> Vec<FfViolation> {
        let mut v = Vec::new();
        let count = usize::from(header) + self.events.len();
        match (count, self.events.first()) {
            (0, _) if self.fork_events.is_empty() => v.push(FfViolation::NoEvent),
            (1, Some(&(span, FfEvent::NbaIntra))) if !header => {
                v.push(FfViolation::NoStatementEvent(span))
            }
            (1, Some(&(span, FfEvent::Stmt { every_pass: false }))) if !header => {
                v.push(FfViolation::NotEveryPass(span))
            }
            (2.., _) => v.push(FfViolation::TooMany {
                count,
                header,
                at: self.events[1 - usize::from(header)].0,
            }),
            _ => {}
        }
        if let Some(&span) = self.fork_events.first() {
            v.push(FfViolation::InFork(span));
        }
        if let Some((span, what)) = self.blocking {
            v.push(FfViolation::Blocking(span, what));
        }
        v
    }
}

/// One way an `always_ff` breaks §9.2.2.4, as [`FfBodyTiming::violations`] finds it.
#[derive(Clone, Copy, Debug)]
pub(crate) enum FfViolation {
    /// No event control at all, outside a `fork` or in one.
    NoEvent,
    /// No header, and the one event control is an intra-assignment `@` on a
    /// non-blocking assignment: nothing suspends the process.
    NoStatementEvent(ast::Span),
    /// No header, and the one event control is an `@(…)` statement some pass can
    /// miss: a pass would end without suspending.
    NotEveryPass(ast::Span),
    /// Two event controls or more; `at` is the first past the one allowed.
    TooMany {
        count: usize,
        header: bool,
        at: ast::Span,
    },
    /// An event control inside a `fork` branch: a forked process cannot be the
    /// procedure's one event control.
    InFork(ast::Span),
    /// A blocking timing control.
    Blocking(ast::Span, &'static str),
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
/// [`FfBodyTiming::satisfies`] decides it, as it decides the check, so a header-less or
/// `@*` block is either routed here or refused there.
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

impl Elaborator<'_> {
    /// Refuse an `always_ff` that breaks §9.2.2.4 ([`FfBodyTiming::violations`]):
    /// `VITA-E3061`, one diagnostic per violation. A count other than one is anchored at
    /// the first event control past the one allowed, or at the block when there is none;
    /// the others at the construct they name.
    pub(crate) fn check_always_ff_timing(&mut self, p: &ast::ProceduralBlock) {
        let header = p.sensitivity.is_some();
        for v in ff_body_timing(&p.body).violations(header) {
            let (span, msg) = match v {
                FfViolation::NoEvent => (
                    p.span,
                    "`always_ff` has no event control; IEEE 1800-2017 §9.2.2.4 requires \
                     exactly one — an `@(…)` header, or one `@(…)` statement in the body"
                        .to_string(),
                ),
                FfViolation::NoStatementEvent(span) => (
                    span,
                    "`always_ff` has no statement-level event control: with no header, its \
                     one event control is an intra-assignment `@` on a non-blocking \
                     assignment, which does not suspend the process; give it an `@(…)` \
                     header or an `@(…)` statement (IEEE 1800-2017 §9.2.2.4)"
                        .to_string(),
                ),
                FfViolation::NotEveryPass(span) => (
                    span,
                    "`always_ff` has no header, and its one event control is not reached on \
                     every pass (it sits in an `if`, a `case` or a loop, or after a loop or a \
                     `disable`), so a pass could end without suspending; put the `@(…)` first \
                     in the body, or give the block an `@(…)` header (a rule of vita's, \
                     stricter than IEEE 1800-2017 §9.2.2.4)"
                        .to_string(),
                ),
                FfViolation::TooMany { count, header, at } => {
                    let body = count - usize::from(header);
                    let place = if header {
                        format!("the header and {body} in the body")
                    } else {
                        format!("{body} in the body")
                    };
                    (
                        at,
                        format!(
                            "`always_ff` has {count} event controls ({place}); IEEE \
                             1800-2017 §9.2.2.4 allows exactly one"
                        ),
                    )
                }
                FfViolation::InFork(span) => (
                    span,
                    "`always_ff` has an event control inside a `fork`; a forked process \
                     cannot be the procedure's one event control (IEEE 1800-2017 §9.2.2.4)"
                        .to_string(),
                ),
                FfViolation::Blocking(span, what) => (
                    span,
                    format!(
                        "`always_ff` contains {what}, a blocking timing control; IEEE \
                         1800-2017 §9.2.2.4 allows none"
                    ),
                ),
            };
            self.error_at(MsgCode::ElabAlwaysFfTiming, span, &msg);
        }
    }
}
