//! §5.2 Q-MDRVMIX — a packed variable's overlapping continuous and procedural writers
//! (rule X, C1), and the bits an `always_comb`, `always_latch` or `always_ff` writes
//! written by another process (rule Y, C2 ; the latch and ff kinds C3, the owner's
//! decision of 2026-11-18). A
//! READER of the write-extent view (`wext.rs`, ER §5.1): every fact it
//! decides with — which net a write lands on, which bits, the writer, how the statement
//! came to sit in its body, and what was declared — is the view's (`WriteView::writes`,
//! `WriteView::decl`), read through one entry, `check_mixed_drivers`. It re-derives no
//! extent, no declaration fact and no `via`, and reads no `intro_kind` and no net name to
//! decide (the name is read once, for the message). Its home rule is H10 of
//! `crates/elaborate/tests/wext_home.rs` (H10d what the decision reads, H10m what the
//! message reads), its pins `crates/cli/tests/mdrv_mix_pins.rs`.
//!
//! THE TARGET SET (T2, W/mdrv/plan2/PLAN2.txt §1, premise judge 2 K1). A net is a target
//! when the view holds a declaration fact for it (`decl`) and the fact says: `site` is a
//! module body, a generate scope, a module's ANSI header or its non-ANSI port
//! (`DeclSite`, every variant: an interface, a package and a block local write no fact) ;
//! `kind` is `logic`, `reg` or `bit` (the declared keyword, not `ir::NetKind`, which folds
//! `event` and the atom types onto `Reg`) ; `dir` is internal or output (an input or inout
//! port is a net, IEEE 1800-2017 §23.2.2.3) ; `type_param` is false. Every net without a
//! fact (task and function formals and locals, loop variables, temporaries, implicit nets,
//! interface and package members) is outside by construction. PLAN2's fifth term, "no
//! unpacked dimension", is not a term here: the view's extent already decides it,
//! `chunk_extent` gives `Word` to every chunk with a word index and a `Word` is never
//! compared, and over the 442 lane views the 48 nets with `array_len > 1` carry no `Bits`
//! write (premise judge 2 L6: N12 equivalent) ; a 1-element unpacked array (`logic [7:0]
//! one [1]`: no `array_dims` entry and an `array_len` of 1) has no other mark and is quiet
//! by the same `Word` (k7_arr_whole, below). A UDP's own nets (premise judge 2 L9): the
//! parser lowers a primitive to a module (`hdl-parser` `udp.rs`), so its output port
//! (`AnsiPort`, `reg`, output) and its state register (`Module`, `reg`) carry facts and are
//! targets by the fact's own definition ; their only writers are the primitive's own
//! processes, so no pair is reachable today (cell k3_udp_seq_disjoint, quiet), and a
//! variable the PARENT connects to a primitive's output is the parent's own target
//! (k3_udp_out_comb, loud).
//!
//! RULE X (IEEE 1800-2017 §6.5, ieee1800-2017.txt:3495-3497: "it shall be an error to have
//! multiple continuous assignments or a mixture of procedural and continuous assignments
//! writing to any term in the expansion of a written longest static prefix of a variable";
//! :3523-3524 a declaration initializer is a procedural assignment ; :3526-3529 an
//! instance output port is a continuous assignment ; §10.3.2 :10283-10285). On a target
//! net, a continuous writer — a `cont_assigns` row of kind `assign` (a user `assign`, or a
//! gate primitive's output, which arrives with the same kind) or `port` (an instance port
//! hookup) — and a procedural writer — a blocking or non-blocking statement (never a
//! `force` or a `release`, §10.6.2) of a process whose kind is one of `P6`, lowered
//! `Direct` or while a static task was inlined (`TaskInline`: the task's body or its
//! copy-in / copy-out, in the caller's process), or a statement of a frame body (a
//! function, an automatic task, a task called hierarchically, a `$unit` routine: premise
//! judge 2 L2) lowered `Direct` or `TaskInline` — whose extents are both `Bits` (the
//! kernel's or the geometry constant's) and intersect, is E3001. A statement lowered in a
//! deferred assertion's action (`DeferAction`) is not a writer of either side, `final`,
//! `sva`, `covergroup`, `clocking` and `net_init` are not in the lists, `Word`,
//! `Undecided` and `Poison` never compare. A frame body is the procedural side whoever
//! calls it: §6.5 forbids any procedural write beside a continuous one, so the view's
//! `Writer::Body`, which names no caller, needs none here (the re-entry plan's S-b).
//!
//! RULE Y (IEEE 1800-2017 §9.2.2.2, ieee1800-2017.txt:9036-9038: "The variables written
//! on the left-hand side of assignments shall not be written to by any other process.
//! However, multiple assignments made to independent elements of a variable are allowed
//! as long as their longest static prefixes do not overlap" ; §9.2.2.2.2, :9082-9084:
//! "Variables on the left-hand side of assignments within an always_comb procedure,
//! including variables from the contents of a called function, shall not be written to by
//! any other processes, whereas always @* permits multiple processes to write to the same
//! variable" ; §9.2.2.3, :9097-9098: "All statements in 9.2.2.2 shall apply to
//! always_latch" ; §9.2.2.4, :9105-9107: "Variables on the left-hand side of assignments
//! within an always_ff procedure, including variables from the contents of a called
//! function, shall not be written to by any other process"). On a target net, the A side
//! — a blocking or non-blocking statement of the own text (`Direct`) of a process whose
//! kind is on the A-kind list `A_KINDS` (`always_comb`, `always_latch`, `always_ff`: the
//! owner's decision 1 of 2026-11-18, IEEE first) — and the
//! B side — a blocking or non-blocking
//! statement of ANOTHER process (another `pid`: the sweep's `distinct` mode) whose kind is
//! one of P5 (`P6` without the declaration initializer: `always_comb`, `always_ff`,
//! `always_latch`, `always` — an `always @*` among them —, `initial`), lowered `Direct` or
//! while a static task or void function was inlined into it (`TaskInline`: a write
//! through a subroutine belongs to the calling process, the re-entry plan's S-c) — whose
//! extents are both `Bits` and intersect, is E3001. An A-kind process's own statement is
//! on both sides at once, so two such processes pair. Two statements of one
//! process never pair (the two arms of a fork are one `initial`, c08_fork_arm ; one
//! `always_comb` writing twice, k3_comb_twice_self) ; one `always_comb` elaborated in two
//! instances is two processes, and a pair when both write one net upward
//! (k3_two_inst_hier). Never a side of rule Y: a frame body (`Writer::Body`: a function's
//! or an automatic task's frame, a `$unit` routine, a task called hierarchically — the
//! view records no calling process, and IEEE counts a called function's writes as the
//! caller's own, so an `always_ff` and the function it calls writing one bit, cell e2,
//! is one process: R3 of the re-entry's rulings, the plan's S-a), a declaration
//! initializer (`var_init`: rules A and C's), a deferred assertion's action, a `force` /
//! `release`, `final`, `sva`, `covergroup`, `clocking`. An A-kind process that writes the
//! bits only through a subroutine it calls gives no A side (`via` does not tell a void
//! function, which §9.2.2.2.2 names, from a task, which no clause names: R-TASKA,
//! R-VOIDFN, the plan's S-c).
//!
//! X BEFORE Y. A net rule X reports is not asked rule Y (one E3001 per variable ; the
//! clause it names is the continuous driver's), and both rules' diagnostics come out
//! together, lowest net first (pins y_x_first_one_net, y_x_and_y_two_nets).
//!
//! THE PAIR TEST. Per target net, the decided intervals of both sides sorted by `(lo,
//! view order)` and swept once (`sweep`): an incoming interval meets the kept interval of
//! the other side with the largest `hi` (of another writer, where the rule asks for one —
//! rule Y's two processes) when that `hi` is above its `lo`. O(n log n) per net (EL2:
//! 512 writers on its widest net). The kept set is the two largest `hi` of distinct
//! writers per side, because one maximum held by the incoming writer's own interval
//! hides a narrower interval of another (y_sweep_second_best) ; equal to brute force on
//! random intervals (unit test `the_sweep_finds_a_pair_iff_brute_force_does`, premise
//! judge 2 K6's fuzz). The pair REPORTED is the sweep's: the first incoming interval, in
//! `(lo, view order)`, that meets a kept one, and that kept one (premise judge 2 L5) ; its
//! bits are the intersection of the two intervals the sweep compared (`Pair`), and its
//! `a` write is the one on the rule's A side (rule Y: the A-kind process's own statement ;
//! when both are, the incoming one).
//!
//! AN ELABORATION THAT LATCHED AN ERROR IS NOT READ (the re-entry's (a), premise judge 2
//! D3 / D4 of the fix). `check_mixed_drivers` returns at its head when `had_error` is set:
//! the view of such an elaboration is not a fact (`wext.rs`, THE STAND-DOWN's last lines:
//! `placeholder_expr`'s `Const` 0, a refused defparam's default), and every older checker
//! that answers part of this — `multidriver.rs` rules A, B and D (whole writes by name,
//! E3001), `check_whole_net_multidriver` (continuous rows, E3001) and the sole-writer
//! checks `cont_array.rs` (E3009) / `cont_var.rs` (E3018), all run before this one —
//! reports through `error`, which latches the flag, so a variable keeps the older
//! diagnostic alone. Warnings (`multidriver.rs`'s W3060) latch nothing. An error raised
//! AFTER this check (`check_readmem_assoc_waits`, api.rs's passes) prints after its
//! E3001. One E3001 per net, lowest net id first.
//!
//! THE MESSAGE. A word of it is a constant of this file, a field of the view's `Write` or
//! of the swept `Pair`, or the net's one `symbols` key ; a position is `Write::at`
//! resolved by `span_file_line_col` ; a fact the view does not hold is not printed (no
//! scope, no instance, no routine name, no declared range: §5.2 Q-MDRVWHO holds what that
//! loses). It is emitted with no source position and no instance frame of its own
//! (`cur_span` and `cur_prefix` are taken around `error`), so the line carries no
//! `file:line:col:` prefix and no `[in scope]` suffix:
//!   rule X: variable `NAME` is driven by CONT AND written by PROC on bits [HI:LO] of its
//!   packed value (bit 0 is the least significant bit), and a variable driven by a
//!   continuous assignment takes no other writer on those bits (IEEE 1800-2017 §6.5,
//!   §10.3.2)
//!   rule Y: variable `NAME` is written by PROC AND by PROC on bits [HI:LO] of its packed
//!   value (bit 0 is the least significant bit), and the bits an `KIND_A` writes take no
//!   write from another process (IEEE 1800-2017 §C) — the first PROC the A write, the
//!   second the B write, each by the same words as rule X's PROC ; KIND_A the A write's
//!   own kind and §C its entry of `A_CLAUSES` (9.2.2.2, 9.2.2.3, 9.2.2.4) ;
//! - NAME: the `symbols` key whose id is the pair's net, through `display_of` (`#<net>`
//!   when there is none: unreachable for a T2 net, which is declared) ;
//! - HI, LO: the pair's `[lo, hi)` printed as `[hi-1:lo]`, storage bits counted from the
//!   least significant bit as 0 — the sentence after it is a constant, so the bits of a
//!   variable declared `[3:-4]` or `[0:7]` read true ;
//! - CONT: "an `assign` or gate output" (kind `assign`: the view cannot tell a gate from an
//!   `assign`) + " (its target at F:L:C)", or "a port connection" (kind `port`) +
//!   " (at F:L:C)" ;
//! - PROC: "a declaration initializer" (kind `var_init`, never a position: its `at` is
//!   `None` and the arm reads none), "`KIND`" (the process kind as its producer pushed it)
//!   or "a subroutine body" (`Writer::Body`) ; when the write's `via` is `TaskInline`,
//!   " through a subroutine it calls" + " (the write, or the call that makes it, at
//!   F:L:C)" (the view cannot tell a callee's statement from the call's copy-out) — a
//!   `Body` write whose `via` is `TaskInline` (an automatic task's frame body calling a
//!   static task, e06) reads "a subroutine body through a subroutine it calls", and that
//!   wording rests on "no frame lowered inside an inline writes a target net" (measured
//!   e07 e09 e10 e11 e12 ; REVIEW-ONLY beyond them) — else
//!   " (in the statement at F:L:C)" — the store was pushed while the statement whose text
//!   starts there was lowered, and no parsed statement inside it was ;
//! - F:L:C: `Write::at` through `span_file_line_col` ; the bracket is printed only when
//!   `at` is `Some` and the resolver gives a line above 0, otherwise nothing is printed
//!   there (a `.*` connection and a declaration initializer carry no `at`).
//!
//! A WHOLE `always_latch` pair `multidriver.rs`'s Rule B sees (module-body processes
//! writing the bare name whole) is Rule B's E3001 (owner decision 2 of 2026-11-18, D-WW
//! option A), which latches `had_error`, so rule Y prints nothing on that elaboration, as on a
//! whole `always_ff` pair.
//!
//! NOT HERE (each keeps PRE's output, with its cell): `always @*` × `always @*` /
//! `initial` (legal, §9.2.2.2.2) and `always` × `initial`, `initial` × `initial` (no
//! A-kind side: no clause forbids them) ; DISJOINT bits of an `always_ff` (§9.2.2.4 has no
//! independent-element sentence: the text does not settle it, never a pair here) ; an
//! A-kind process writing only through a
//! task (R-TASKA, c01 c02) or a void function called as a statement, which inlines like a
//! task (R-VOIDFN, premise judge 2 L1, k2_voidfn_comb_ff) ; a frame body on either side of
//! rule Y (R-FN: m_c_func_comb_acomb_ppo, k3_fn_two_comb, k2_fnframe_comb_ff, k2_hier_task)
//! ; `final` (R-FINAL, c09) ; an atom-typed variable
//! (R-ATOM, c10 ; an `int` on a child's output port, k8_int_port_x, illegal and silent: an
//! `int` written by `assign` and a process is `cont_var.rs`'s E3018 already) ; a
//! type-parameter-typed variable (k7_tparam_overlap) and a 1- or 2-element unpacked array
//! written whole and per element (k7_arr_whole), both illegal and silent (premise judge 2
//! L8) ; an automatic task's copy-out (`out_binds`, no statement: c04) ; a clocking-block
//! output drive, which reaches the signal through `clocking_outputs`, no write the view
//! records (premise judge 2 L3, k3_clocking_out) ; a deferred assertion's action (c12) ;
//! interface members and interface ports, a package variable (no fact: k3_iface_overlap,
//! k7_iface_port_overlap, i48, n08, n07) ; input-port nets ; continuous × continuous
//! (`check_whole_net_multidriver`, §5.2 Q-MDCONSTOFF) ; unpacked words (§5.2 Q-ARRWORDMD) ;
//! run-time prefixes and kernel-undecided offsets (`Undecided`: `v[i]` with a loop index
//! beside `assign v[0]`, p7_loopidx, illegal by §6.5's longest static prefix, R-RT ; i61,
//! n02, n06, i63) ; every select of an elaboration that held a provisional constant
//! (`wext.rs` THE STAND-DOWN: a4, k1k, reach lost, §5.2 Q-HIERBITS).

use super::*;
use crate::wext::{DeclSite, Extent, Form, Via, Write, WriteView, Writer};

/// The process kinds whose statements are rule X's procedural writers (PLAN2 §2's P6:
/// `proc_idents`' kinds as their producers push them). Rule Y's processes are the same
/// list without the declaration initializer (`in_p5`); `final`, `sva`, `covergroup`,
/// `clocking` are not here.
pub(crate) const P6: [&str; 6] = [
    ALWAYS_COMB,
    ALWAYS_FF,
    ALWAYS_LATCH,
    "always",
    "initial",
    VAR_INIT,
];

/// The process kinds of rule Y's A side (`stmt_flow.rs`'s `always_comb`, `always_latch`,
/// `always_ff`), P6's members rule Y names: the A-kind list. Its clause table is
/// `A_CLAUSES`, entry for entry.
const ALWAYS_COMB: &str = "always_comb";
const ALWAYS_LATCH: &str = "always_latch";
const ALWAYS_FF: &str = "always_ff";
const A_KINDS: [&str; 3] = [ALWAYS_COMB, ALWAYS_LATCH, ALWAYS_FF];

/// The IEEE 1800-2017 clause of each `A_KINDS` entry, in its order, that the rule Y
/// sentence names for the A write's kind: §9.2.2.2 (`always_comb`, txt:9036-9038),
/// §9.2.2.3 (`always_latch`, txt:9097-9098: "All statements in 9.2.2.2 shall apply"),
/// §9.2.2.4 (`always_ff`, txt:9105-9107).
const A_CLAUSES: [&str; 3] = ["9.2.2.2", "9.2.2.3", "9.2.2.4"];

/// The process kind of a declaration initializer (`var_init.rs`), named in the message.
const VAR_INIT: &str = "var_init";

/// The continuous kind of a user `assign` or a gate output, named in the message.
const ASSIGN: &str = "assign";

/// The continuous kind of an instance port hookup (`ports.rs`), named in the message.
const PORT: &str = "port";

/// The `ca_idents` kinds that are rule X's continuous writers: a user `assign` or a gate
/// output (`assign`), an instance port hookup (`port`). `net_init` writes a net, `t0_pulse`
/// a synthesized net.
const CONT: [&str; 2] = [ASSIGN, PORT];

/// Is `net` in the target set (module head, T2)? The view's declaration fact only.
fn in_t2(view: &WriteView, net: u32) -> bool {
    let Some(f) = view.decl(net) else {
        return false;
    };
    matches!(
        f.site,
        DeclSite::Module | DeclSite::Generate | DeclSite::AnsiPort | DeclSite::NonAnsiPort
    ) && matches!(
        f.kind,
        ast::NetVarKind::Logic | ast::NetVarKind::Reg | ast::NetVarKind::Bit
    ) && matches!(f.dir, ir::PortDir::Internal | ir::PortDir::Output)
        && !f.type_param
}

/// A blocking or non-blocking statement lowered from its body's text or from an inlined
/// task (module head): a procedural writer's form and `via`.
fn procedural(w: &Write) -> bool {
    matches!(w.form, Form::Blocking | Form::NonBlocking)
        && matches!(w.via, Some(Via::Direct | Via::TaskInline))
}

/// Rule X's sides of one write, `(a, b)`: `a` continuous, `b` procedural.
fn x_side(w: &Write) -> Option<(bool, bool)> {
    match w.writer {
        Writer::Cont { kind, .. } if CONT.contains(&kind) => Some((true, false)),
        Writer::Proc { kind, .. } if P6.contains(&kind) && procedural(w) => Some((false, true)),
        Writer::Body { .. } if procedural(w) => Some((false, true)),
        _ => None,
    }
}

/// Rule Y's processes (PLAN2 §2's P5): P6 without the declaration initializer, which is
/// not a process (rules A and C answer for it).
fn in_p5(kind: &str) -> bool {
    kind != VAR_INIT && P6.contains(&kind)
}

/// Rule Y's sides of one write, `(a, b)` (module head): `a` a statement of the own text
/// of a process whose kind is on the A-kind list (`A_KINDS`: `always_comb`,
/// `always_latch`, `always_ff` ; `Direct`: not a task or void function it calls, R-TASKA
/// / R-VOIDFN), `b` a statement of any P5 process, its own or inlined (`TaskInline`). Such
/// a process's own statement is both. A frame body is neither (R-FN, cell e2).
fn y_side(w: &Write) -> Option<(bool, bool)> {
    match w.writer {
        Writer::Proc { kind, .. } if in_p5(kind) && procedural(w) => {
            Some((A_KINDS.contains(&kind) && w.via == Some(Via::Direct), true))
        }
        _ => None,
    }
}

/// A writer's identity for the sweep: a continuous row, a process or a frame body.
fn who(w: &Writer) -> u64 {
    match *w {
        Writer::Cont { row, .. } => u64::from(row),
        Writer::Proc { pid, .. } => (1 << 32) | u64::from(pid),
        Writer::Body { func, .. } => (2 << 32) | u64::from(func),
    }
}

/// One decided interval `[lo, hi)` of one write, its writer and its sides (`a`, `b`), and
/// its index in the view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Iv {
    pub(crate) lo: u32,
    pub(crate) hi: u32,
    pub(crate) who: u64,
    pub(crate) a: bool,
    pub(crate) b: bool,
    pub(crate) idx: u32,
}

/// The two largest `hi` of distinct writers among the kept intervals of one side.
#[derive(Default)]
struct Top2([Option<Iv>; 2]);

impl Top2 {
    /// The kept interval of another writer (`distinct`) or of any writer with the largest
    /// `hi`, when it reaches past `lo`.
    fn meets(&self, i: &Iv, distinct: bool) -> Option<u32> {
        for k in self.0.iter().flatten() {
            if distinct && k.who == i.who {
                continue;
            }
            return (k.hi > i.lo).then_some(k.idx);
        }
        None
    }

    fn keep(&mut self, i: Iv) {
        let [t0, t1] = &mut self.0;
        match *t0 {
            None => *t0 = Some(i),
            Some(k) if k.who == i.who => {
                if i.hi > k.hi {
                    *t0 = Some(i);
                }
            }
            Some(k) if i.hi > k.hi => {
                *t1 = Some(k);
                *t0 = Some(i);
            }
            Some(_) => match *t1 {
                Some(k) if k.who == i.who || i.hi <= k.hi => {
                    if k.who == i.who && i.hi > k.hi {
                        *t1 = Some(i);
                    }
                }
                _ => *t1 = Some(i),
            },
        }
    }
}

/// The pair test (module head): the first `(incoming, kept)` pair of an `a` interval and a
/// `b` interval that overlap, of two writers when `distinct`, in `(lo, view order)`.
pub(crate) fn sweep(ivs: &mut [Iv], distinct: bool) -> Option<(u32, u32)> {
    ivs.sort_unstable_by_key(|i| (i.lo, i.idx));
    let (mut ka, mut kb) = (Top2::default(), Top2::default());
    for &i in ivs.iter() {
        if i.a {
            if let Some(k) = kb.meets(&i, distinct) {
                return Some((i.idx, k));
            }
        }
        if i.b {
            if let Some(k) = ka.meets(&i, distinct) {
                return Some((i.idx, k));
            }
        }
        if i.a {
            ka.keep(i);
        }
        if i.b {
            kb.keep(i);
        }
    }
    None
}

/// Which rule a pair is (module head: X first on a net, then Y).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Rule {
    X,
    Y,
}

/// One pair: the rule, the net, the `a` and the `b` write's view indices (rule X: the
/// continuous and the procedural one ; rule Y: the A-kind process's own and the other
/// process's) and the bits both write, `[lo, hi)`, from the two intervals the sweep
/// compared (the one place a pair's extents are read, so the text says what the sweep
/// found).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Pair {
    rule: Rule,
    net: u32,
    a: u32,
    b: u32,
    lo: u32,
    hi: u32,
}

/// One rule's pairs (`side` its sides of a write, `distinct` whether the two writers must
/// differ), one per target net that `skip` does not name, lowest net first.
fn pairs(
    view: &WriteView,
    rule: Rule,
    side: fn(&Write) -> Option<(bool, bool)>,
    distinct: bool,
    skip: impl Fn(u32) -> bool,
) -> Vec<Pair> {
    let mut per: BTreeMap<u32, Vec<Iv>> = BTreeMap::new();
    for (k, w) in view.writes.iter().enumerate() {
        let Extent::Bits { lo, hi, .. } = w.extent else {
            continue;
        };
        let Some((a, b)) = side(w) else {
            continue;
        };
        if !in_t2(view, w.net) {
            continue;
        }
        per.entry(w.net).or_default().push(Iv {
            lo,
            hi,
            who: who(&w.writer),
            a,
            b,
            idx: k as u32,
        });
    }
    let mut out = Vec::new();
    for (net, mut ivs) in per {
        if skip(net) || !ivs.iter().any(|i| i.a) || !ivs.iter().any(|i| i.b) {
            continue;
        }
        if let Some((inc, kept)) = sweep(&mut ivs, distinct) {
            let iv_of = |x: u32| ivs.iter().find(|i| i.idx == x).copied();
            let (Some(i), Some(k)) = (iv_of(inc), iv_of(kept)) else {
                continue;
            };
            // the sweep met an incoming `a` with a kept `b` first, else a kept `a`
            let (a, b) = if i.a && k.b { (i, k) } else { (k, i) };
            out.push(Pair {
                rule,
                net,
                a: a.idx,
                b: b.idx,
                lo: i.lo.max(k.lo),
                hi: i.hi.min(k.hi),
            });
        }
    }
    out
}

impl Elaborator<'_> {
    /// The end phase's rules X and Y (module head), after the older checks: one E3001 per
    /// target net whose writers overlap, rule X's pair where a net has one, lowest net
    /// first ; nothing when the elaboration latched an error (module head, (a)).
    pub(crate) fn check_mixed_drivers(&mut self) {
        if self.had_error {
            return;
        }
        // The one read of the view: each pair's two writes, copied out before emitting.
        let found: Vec<(Pair, Write, Write)> = {
            let view = &self.wext;
            let x = pairs(view, Rule::X, x_side, false, |_| false);
            let xs: BTreeSet<u32> = x.iter().map(|p| p.net).collect();
            let y = pairs(view, Rule::Y, y_side, true, |n| xs.contains(&n));
            let mut all: Vec<Pair> = x.into_iter().chain(y).collect();
            all.sort_by_key(|p| p.net);
            all.into_iter()
                .map(|p| (p, view.writes[p.a as usize], view.writes[p.b as usize]))
                .collect()
        };
        for (p, a, b) in found {
            self.mdrv_emit(p, &a, &b);
        }
    }

    /// One E3001 of rule X or Y, its text from the pair and its two writes only (module
    /// head, THE MESSAGE), with no position and no instance frame of its own.
    fn mdrv_emit(&mut self, x: Pair, a: &Write, b: &Write) {
        let (name, wa, wb) = (
            self.mdrv_net_name(x.net),
            self.mdrv_writer(a),
            self.mdrv_writer(b),
        );
        let msg = match x.rule {
            Rule::X => format!(
                "variable `{name}` is driven by {wa} AND written by {wb} on bits [{}:{}] of \
                 its packed value (bit 0 is the least significant bit), and a variable \
                 driven by a continuous assignment takes no other writer on those bits (IEEE \
                 1800-2017 §6.5, §10.3.2)",
                x.hi - 1,
                x.lo,
            ),
            Rule::Y => {
                // the A write's own kind, an `A_KINDS` entry (`y_side`), and its clause
                let (kind, clause) = match a.writer {
                    Writer::Proc { kind, .. } => A_KINDS
                        .iter()
                        .zip(A_CLAUSES)
                        .find(|(k, _)| **k == kind)
                        .map_or((kind, "9.2.2"), |(k, c)| (*k, c)),
                    _ => ("?", "9.2.2"),
                };
                format!(
                    "variable `{name}` is written by {wa} AND by {wb} on bits [{}:{}] of its \
                     packed value (bit 0 is the least significant bit), and the bits an \
                     `{kind}` writes take no write from another process (IEEE 1800-2017 \
                     §{clause})",
                    x.hi - 1,
                    x.lo,
                )
            }
        };
        let (span, prefix) = (self.cur_span.take(), std::mem::take(&mut self.cur_prefix));
        self.error(MsgCode::ElabMultidriver, &msg);
        self.cur_span = span;
        self.cur_prefix = prefix;
    }

    /// The variable's name in the message (the one name read: the view's net id).
    fn mdrv_net_name(&self, net: u32) -> String {
        self.symbols
            .iter()
            .find(|(_, &id)| id == net)
            .map(|(n, _)| self.display_of(n))
            .unwrap_or_else(|| format!("#{net}"))
    }

    /// One writer as the message names it (module head, THE MESSAGE): its kind words and
    /// the write's own position, nothing else.
    fn mdrv_writer(&self, w: &Write) -> String {
        let pos =
            w.at.map(|sp| self.span_file_line_col(sp))
                .filter(|(_, line, _)| *line > 0)
                .map(|(file, line, col)| format!("{file}:{line}:{col}"));
        let what = match w.writer {
            Writer::Cont { kind, .. } if kind == ASSIGN => {
                return match pos {
                    Some(p) => format!("an `assign` or gate output (its target at {p})"),
                    None => "an `assign` or gate output".to_string(),
                };
            }
            Writer::Cont { kind, .. } if kind == PORT => {
                return match pos {
                    Some(p) => format!("a port connection (at {p})"),
                    None => "a port connection".to_string(),
                };
            }
            // No other continuous kind is a side (`CONT`): the floor that holds for any row.
            Writer::Cont { .. } => return "a continuous assignment".to_string(),
            Writer::Proc { kind, .. } if kind == VAR_INIT => {
                return "a declaration initializer".to_string();
            }
            Writer::Proc { kind, .. } => format!("`{kind}`"),
            Writer::Body { .. } => "a subroutine body".to_string(),
        };
        match (w.via == Some(Via::TaskInline), pos) {
            (true, Some(p)) => format!(
                "{what} through a subroutine it calls (the write, or the call that makes it, \
                 at {p})"
            ),
            (true, None) => format!("{what} through a subroutine it calls"),
            (false, Some(p)) => format!("{what} (in the statement at {p})"),
            (false, None) => what,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every overlapping `(a, b)` pair, of two writers when `distinct` (premise judge 2's
    /// brute force).
    fn brute(ivs: &[Iv], distinct: bool) -> bool {
        ivs.iter().enumerate().any(|(i, x)| {
            ivs.iter().skip(i + 1).any(|y| {
                x.lo < y.hi
                    && y.lo < x.hi
                    && !(distinct && x.who == y.who)
                    && ((x.a && y.b) || (x.b && y.a))
            })
        })
    }

    /// Premise judge 2 K6's fuzz, ported: on random interval sets the sweep finds a pair
    /// exactly when brute force does, and the pair it reports overlaps, sides `a` / `b`,
    /// of two writers when asked.
    #[test]
    fn the_sweep_finds_a_pair_iff_brute_force_does() {
        let mut s: u64 = 0x9e37_79b9_7f4a_7c15;
        let mut rnd = |m: u64| {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            s % m
        };
        let (mut fires, mut quiet) = (0u32, 0u32);
        for t in 0..200_000u32 {
            let distinct = t % 2 == 0;
            let k = 2 + rnd(7) as u32;
            let w = 2 + rnd(14) as u32;
            let np = 1 + rnd(4);
            let ivs: Vec<Iv> = (0..k)
                .map(|idx| {
                    let lo = rnd(u64::from(w)) as u32;
                    let hi = lo + 1 + rnd(u64::from(w - lo)) as u32;
                    let (a, b) = match rnd(3) {
                        0 => (true, true),
                        1 => (false, true),
                        _ => (true, false),
                    };
                    Iv {
                        lo,
                        hi,
                        who: rnd(np),
                        a,
                        b,
                        idx,
                    }
                })
                .collect();
            let want = brute(&ivs, distinct);
            let mut sorted = ivs.clone();
            let got = sweep(&mut sorted, distinct);
            assert_eq!(got.is_some(), want, "case {t}: {ivs:?}");
            if let Some((i, j)) = got {
                let (x, y) = (ivs[i as usize], ivs[j as usize]);
                assert!(x.lo < y.hi && y.lo < x.hi, "case {t}: no overlap");
                assert!((x.a && y.b) || (x.b && y.a), "case {t}: one side");
                assert!(!(distinct && x.who == y.who), "case {t}: one writer");
                fires += 1;
            } else {
                quiet += 1;
            }
        }
        // the fuzz enters both branches
        assert!(
            fires > 50_000 && quiet > 50_000,
            "fires {fires} quiet {quiet}"
        );
    }

    /// Rule Y's sides on a hand-built view (module head): which writes pair, which never do,
    /// and the pair's orientation (`a` the A-kind process's own statement).
    #[test]
    fn rule_y_pairs_an_a_kind_process_with_another_process_only() {
        use crate::wext::{By, DeclFact};
        let fact = DeclFact {
            site: DeclSite::Module,
            kind: ast::NetVarKind::Logic,
            dir: ir::PortDir::Internal,
            type_param: false,
        };
        let bits = |lo, hi| Extent::Bits {
            lo,
            hi,
            by: By::Kernel,
        };
        let w = |writer, form, via, extent| Write {
            writer,
            form,
            stmt: Some(0),
            via: Some(via),
            chunk: 0,
            net: 0,
            extent,
            at: None,
        };
        let p = |pid, kind, via, extent| w(Writer::Proc { pid, kind }, Form::Blocking, via, extent);
        let y = |ws: Vec<Write>, decl: Option<DeclFact>| {
            pairs(
                &WriteView::for_test(ws, vec![decl]),
                Rule::Y,
                y_side,
                true,
                |_| false,
            )
        };
        let n = |ws: Vec<Write>| y(ws, Some(fact)).len();
        let comb = p(0, ALWAYS_COMB, Via::Direct, bits(0, 4));
        // the pair, oriented: the always_comb's own statement is `a`, whatever the view order
        let got = y(
            vec![p(1, "initial", Via::Direct, bits(2, 6)), comb],
            Some(fact),
        );
        assert_eq!(
            got.iter()
                .map(|q| (q.a, q.b, q.lo, q.hi))
                .collect::<Vec<_>>(),
            vec![(1, 0, 2, 4)],
            "pair"
        );
        for kind in [
            "always_comb",
            "always_ff",
            "always_latch",
            "always",
            "initial",
        ] {
            assert_eq!(
                n(vec![comb, p(1, kind, Via::Direct, bits(3, 8))]),
                1,
                "{kind}"
            );
        }
        // another process through a static task it inlines: `b`
        assert_eq!(
            n(vec![comb, p(1, "always_ff", Via::TaskInline, bits(2, 6))]),
            1,
            "b via task"
        );
        // the always_comb only through a task: no `a` (R-TASKA, R-VOIDFN)
        let comb_task = p(0, ALWAYS_COMB, Via::TaskInline, bits(0, 4));
        assert_eq!(
            n(vec![comb_task, p(1, "initial", Via::Direct, bits(2, 6))]),
            0,
            "a via task"
        );
        // one process twice ; disjoint and adjacent bits
        assert_eq!(
            n(vec![comb, p(0, ALWAYS_COMB, Via::Direct, bits(2, 6))]),
            0,
            "same pid"
        );
        assert_eq!(
            n(vec![comb, p(1, "initial", Via::Direct, bits(4, 8))]),
            0,
            "adjacent"
        );
        // extents that never compare, on either side
        for e in [Extent::Word, Extent::Undecided, Extent::Poison] {
            assert_eq!(n(vec![comb, p(1, "initial", Via::Direct, e)]), 0, "{e:?} b");
            let a = p(0, ALWAYS_COMB, Via::Direct, e);
            assert_eq!(
                n(vec![a, p(1, "initial", Via::Direct, bits(0, 8))]),
                0,
                "{e:?} a"
            );
        }
        // not writers of rule Y: a deferred action, an initializer, `final`, a frame body, a force
        assert_eq!(
            n(vec![comb, p(1, "initial", Via::DeferAction, bits(0, 8))]),
            0,
            "defer"
        );
        assert_eq!(
            n(vec![comb, p(1, VAR_INIT, Via::Direct, bits(0, 8))]),
            0,
            "var_init"
        );
        assert_eq!(
            n(vec![comb, p(1, "final", Via::Direct, bits(0, 8))]),
            0,
            "final"
        );
        let body = Writer::Body {
            func: 0,
            task: false,
        };
        assert_eq!(
            n(vec![comb, w(body, Form::Blocking, Via::Direct, bits(0, 8))]),
            0,
            "body"
        );
        let force = w(
            Writer::Proc {
                pid: 1,
                kind: "initial",
            },
            Form::Force,
            Via::Direct,
            bits(0, 8),
        );
        assert_eq!(n(vec![comb, force]), 0, "force");
        // an `always_latch` or `always_ff` own statement is an A side too (C3): the pair,
        // oriented, with the A write first whatever the view order
        for (k1, k2) in [
            ("always_ff", "always_ff"),
            ("always_ff", "always_latch"),
            ("always_ff", "always"),
            ("always_ff", "initial"),
            ("always_latch", "always_latch"),
            ("always_latch", "always"),
            ("always_latch", "initial"),
        ] {
            let ws = vec![
                p(1, k2, Via::Direct, bits(2, 6)),
                p(0, k1, Via::Direct, bits(0, 4)),
            ];
            let got = y(ws, Some(fact));
            let ab: Vec<_> = got.iter().map(|q| (q.a, q.b, q.lo, q.hi)).collect();
            let want = if A_KINDS.contains(&k2) {
                (0, 1, 2, 4) // both A sides: the incoming one, in (lo, view order)
            } else {
                (1, 0, 2, 4)
            };
            assert_eq!(ab, vec![want], "{k1} x {k2}");
        }
        // only through a task: no A side, for every A kind
        for k in A_KINDS {
            let ws = vec![
                p(0, k, Via::TaskInline, bits(0, 4)),
                p(1, "initial", Via::Direct, bits(2, 6)),
            ];
            assert_eq!(n(ws), 0, "{k} via task");
        }
        // no A-kind side: `always @*` and plain pairs stay out
        for (k1, k2) in [
            ("always", "always"),
            ("always", "initial"),
            ("initial", "initial"),
        ] {
            let ws = vec![
                p(0, k1, Via::Direct, bits(0, 4)),
                p(1, k2, Via::Direct, bits(2, 6)),
            ];
            assert_eq!(n(ws), 0, "{k1} x {k2}");
        }
        // a net outside the target set
        assert!(
            y(vec![comb, p(1, "initial", Via::Direct, bits(2, 6))], None).is_empty(),
            "T2"
        );
    }

    /// y_sweep_second_best (PLAN2 §4): one maximum held by the incoming writer's own
    /// interval would hide the narrower interval of another writer.
    #[test]
    fn the_sweep_keeps_the_second_best_writer() {
        let iv = |lo, hi, who, a, b, idx| Iv {
            lo,
            hi,
            who,
            a,
            b,
            idx,
        };
        // writer 0: [0,8) b only ; writer 1: [1,5) b ; writer 0: [2,3) a
        let mut ivs = vec![
            iv(0, 8, 0, false, true, 0),
            iv(1, 5, 1, false, true, 1),
            iv(2, 3, 0, true, false, 2),
        ];
        assert_eq!(sweep(&mut ivs, true), Some((2, 1)));
        // no second writer: no pair
        let mut ivs = vec![iv(0, 8, 0, false, true, 0), iv(2, 3, 0, true, false, 2)];
        assert_eq!(sweep(&mut ivs, true), None);
    }
}
