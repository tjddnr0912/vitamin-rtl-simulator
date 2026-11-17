//! §5.2 Q-MDRVMIX — a packed variable's overlapping continuous and procedural writers
//! (rule X, C1). A READER of the write-extent view (`wext.rs`, ER §5.1): every fact it
//! decides with — which net a write lands on, which bits, the writer, how the statement
//! came to sit in its body, and what was declared — is the view's (`WriteView::writes`,
//! `WriteView::decl`), read through one entry, `check_mixed_drivers`. It re-derives no
//! extent, no declaration fact and no `via`, reads no `intro_kind` and no net name to
//! decide (the name is read once, for the message). Its home rules are H10-H12 of
//! `crates/elaborate/tests/wext_home.rs`, its pins `crates/cli/tests/mdrv_mix_pins.rs`.
//!
//! THE TARGET SET (T2, W/mdrv/plan2/PLAN2.txt §1, premise judge 2 K1). A net is a target
//! when the view holds a declaration fact for it (`decl`) and the fact says: `site` is a
//! module body, a generate scope, a module's ANSI header or its non-ANSI port
//! (`DeclSite`, every variant: an interface, a package and a block local write no fact) ;
//! `kind` is `logic`, `reg` or `bit` (the declared keyword, not `ir::NetKind`, which folds
//! `event` and the atom types onto `Reg`) ; `dir` is internal or output (an input or inout
//! port is a net, IEEE 1800-2017 §23.2.2.3) ; `type_param` is false. Every net without a
//! fact (task and function formals and locals, loop variables, temporaries, implicit nets)
//! is outside by construction. PLAN2's fifth term, "no unpacked dimension", is not a term
//! here: the view's extent already decides it, `chunk_extent` gives `Word` to every chunk
//! with a word index and a `Word` is never compared, and over the 442 lane views the 48
//! nets with `array_len > 1` carry no `Bits` write (premise judge 2 L6: N12 equivalent) ;
//! a 1-element unpacked array (`logic [7:0] one [1]`: no `array_dims` entry and an
//! `array_len` of 1) has no other mark and is quiet by the same `Word` (k7_arr_whole, below).
//! A UDP's own nets (premise judge 2 L9): the parser lowers a primitive to a module
//! (`hdl-parser` `udp.rs`), so its output port (`AnsiPort`, `reg`, output) and its state
//! register (`Module`, `reg`) carry facts and are targets by the fact's own definition ;
//! their only writers are the primitive's own processes, so no pair is reachable today
//! (cell k3_udp_seq_disjoint, quiet), and a variable the PARENT connects to a primitive's
//! output is the parent's own target (k3_udp_out_comb, loud).
//!
//! RULE X (IEEE 1800-2017 §6.5, ieee1800-2017.txt:3495-3497: "it shall be an error to have
//! multiple continuous assignments or a mixture of procedural and continuous assignments
//! writing to any term in the expansion of a written longest static prefix of a variable";
//! :3523-3524 a declaration initializer is a procedural assignment ; :3526-3529 an
//! instance output port is a continuous assignment). On a target net, a continuous writer
//! — a `cont_assigns` row of kind `assign` (a user `assign`, or a gate primitive's output,
//! which arrives with the same kind) or `port` (an instance port hookup) — and a
//! procedural writer — a blocking or non-blocking statement (never a `force` or a
//! `release`, §10.6.2) of a process whose kind is one of `P6`, lowered `Direct` or while a
//! static task was inlined (`TaskInline`: the task's body or its copy-out, in the caller's
//! process), or a statement of a frame body (a function, an automatic task, a task called
//! hierarchically: premise judge 2 L2) lowered `Direct` or `TaskInline` — whose extents
//! are both `Bits` (the kernel's or the geometry constant's) and intersect, is E3001. A
//! statement lowered in a deferred assertion's action (`DeferAction`) is not a writer of
//! either side, `final`, `sva`, `covergroup`, `clocking` and `net_init` are not in the
//! lists, `Word`, `Undecided` and `Poison` never compare.
//!
//! THE PAIR TEST. Per target net, the decided intervals of both sides sorted by `(lo,
//! view order)` and swept once (`sweep`): an incoming interval meets the kept interval of
//! the other side with the largest `hi` (of another writer, where the rule asks for one —
//! rule Y's two processes, C2) when that `hi` is above its `lo`. O(n log n) per net (EL2:
//! 512 writers on its widest net). The kept set is the two largest `hi` of distinct
//! writers per side, because one maximum held by the incoming writer's own interval
//! hides a narrower interval of another (y_sweep_second_best) ; equal to brute force on
//! random intervals (unit test `the_sweep_finds_a_pair_iff_brute_force_does`, premise
//! judge 2 K6's fuzz). The pair REPORTED is the sweep's: the first incoming interval, in
//! `(lo, view order)`, that meets a kept one, and that kept one (premise judge 2 L5).
//!
//! ONE DIAGNOSTIC PER VARIABLE, AND THE OLDER CHECKS FIRST (PLAN2 R14, the first plan's
//! Q12). The three checkers that answer parts of this today are not re-pointed and keep
//! their verdicts: `multidriver.rs` rules A, B and D (whole writes by name, E3001 /
//! W3060), `check_whole_net_multidriver` (continuous rows, E3001) and the sole-writer
//! checks `cont_array.rs` (E3009) / `cont_var.rs` (E3018). This check runs after all of
//! them and skips every net they reported (`cont_var.reported`: rules A, B and D's E3001,
//! the overlap check's E3001, the whole-array E3009, and `cont_var.rs`'s own E3018s, which
//! that check now records there too), so a variable gets the older diagnostic alone. One
//! E3001 per net, lowest net id first, then the variable is recorded as reported.
//!
//! THE MESSAGE says what is located and how (premise judge 2 L4). It is emitted with no
//! source position of its own and no instance frame: both writers are named in the text,
//! each with the `file:line:col` its identity table holds (`ca_idents` / `proc_idents`:
//! the `assign` or gate, the port connection, the process keyword, the declaration for a
//! `var_init`) and the instance it was elaborated under, so two instances of one module
//! (one `file:line:col`, two scopes) read apart. A port row of a `.*` connection has no
//! position (`push_cont_assign_port`) and says so. A frame body has no position in its
//! writer ; it is named by its routine and that routine's declaration (`frame_decl_locs`).
//! The kind `assign` cannot tell an `assign` from a gate output, so the text says "an
//! `assign` or gate output". The bits are storage bits `[hi:lo]` of the overlap, counted
//! from the least significant bit as 0 (said so when the declared range is not `[w-1:0]`).
//!
//! NOT HERE (each keeps PRE's output, with its cell): rule Y, an `always_comb` beside
//! another process (C2: its pins are PRE-text pins until then) ; the `always_ff` /
//! `always_latch` pairs (R-FF) ; an `always_comb` writing only through a task (R-TASKA,
//! c01 c02) or a void function called as a statement, which inlines like a task (R-VOIDFN,
//! premise judge 2 L1, k2_voidfn_comb_ff: both C2's A side) ; `final` (R-FINAL, c09) ; an
//! atom-typed variable (R-ATOM, c10 ; an `int` on a child's output port, k8_int_port_x,
//! illegal and silent: an `int` written by `assign` and a process is `cont_var.rs`'s E3018
//! already) ; a type-parameter-typed variable (k7_tparam_overlap) and a 1- or 2-element
//! unpacked array written whole and per element (k7_arr_whole), both illegal and silent
//! (premise judge 2 L8) ; an automatic task's copy-out
//! (`out_binds`, no statement: c04) ; a clocking-block output drive, which reaches the
//! signal through `clocking_outputs`, no write the view records (premise judge 2 L3,
//! k3_clocking_out) ; a deferred assertion's action (c12) ; interface members and
//! interface ports (no fact: k3_iface_overlap, k7_iface_port_overlap) ; input-port nets ;
//! continuous × continuous (`check_whole_net_multidriver`, §5.2 Q-MDCONSTOFF) ; unpacked
//! words (§5.2 Q-ARRWORDMD) ; run-time prefixes (`Undecided`: `v[i]` with a loop index
//! beside `assign v[0]`, p7_loopidx, illegal by §6.5's longest static prefix, R-RT).

use super::*;
use crate::wext::{DeclSite, Extent, Form, Via, Write, WriteView, Writer};

/// The process kinds whose statements are rule X's procedural writers (PLAN2 §2's P6:
/// `proc_idents`' kinds as their producers push them). Rule Y's B side (C2) is the first
/// five; `final`, `sva`, `covergroup`, `clocking` are not here.
pub(crate) const P6: [&str; 6] = [
    "always_comb",
    "always_ff",
    "always_latch",
    "always",
    "initial",
    VAR_INIT,
];

/// The process kind of a declaration initializer (`var_init.rs`), named in the message.
const VAR_INIT: &str = "var_init";

/// The continuous kind of an instance port hookup (`ports.rs`), named in the message.
const PORT: &str = "port";

/// The `ca_idents` kinds that are rule X's continuous writers: a user `assign` or a gate
/// output (`assign`), an instance port hookup (`port`). `net_init` writes a net, `t0_pulse`
/// a synthesized net.
const CONT: [&str; 2] = ["assign", PORT];

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

/// Rule X's side of one write: `Some(true)` continuous, `Some(false)` procedural.
fn x_side(w: &Write) -> Option<bool> {
    match w.writer {
        Writer::Cont { kind, .. } if CONT.contains(&kind) => Some(true),
        Writer::Proc { kind, .. } if P6.contains(&kind) && procedural(w) => Some(false),
        Writer::Body { .. } if procedural(w) => Some(false),
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

/// One rule X pair: the net, the continuous and the procedural write's view indices and
/// the bits both write, `[lo, hi)`, from the two intervals the sweep compared (the one
/// place a pair's extents are read, so the text says what the sweep found).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct XPair {
    net: u32,
    cont: u32,
    proc: u32,
    lo: u32,
    hi: u32,
}

/// Rule X's pairs, one per target net that `skip` does not name, lowest net first.
fn x_pairs(view: &WriteView, skip: impl Fn(u32) -> bool) -> Vec<XPair> {
    let mut per: BTreeMap<u32, Vec<Iv>> = BTreeMap::new();
    for (k, w) in view.writes.iter().enumerate() {
        let Extent::Bits { lo, hi, .. } = w.extent else {
            continue;
        };
        let Some(cont) = x_side(w) else {
            continue;
        };
        if !in_t2(view, w.net) {
            continue;
        }
        per.entry(w.net).or_default().push(Iv {
            lo,
            hi,
            who: who(&w.writer),
            a: cont,
            b: !cont,
            idx: k as u32,
        });
    }
    let mut out = Vec::new();
    for (net, mut ivs) in per {
        if skip(net) || !ivs.iter().any(|i| i.a) || !ivs.iter().any(|i| i.b) {
            continue;
        }
        if let Some((inc, kept)) = sweep(&mut ivs, false) {
            let at = |x: u32| ivs.iter().find(|i| i.idx == x).copied();
            let (Some(i), Some(k)) = (at(inc), at(kept)) else {
                continue;
            };
            let (c, p) = if i.a { (i, k) } else { (k, i) };
            out.push(XPair {
                net,
                cont: c.idx,
                proc: p.idx,
                lo: i.lo.max(k.lo),
                hi: i.hi.min(k.hi),
            });
        }
    }
    out
}

impl Elaborator<'_> {
    /// The end phase's rule X (module head), after the older checks: one E3001 per target
    /// net whose continuous and procedural writers overlap.
    pub(crate) fn check_mixed_drivers(&mut self) {
        // The one read of the view: each pair's two writes, copied out before emitting.
        let found: Vec<(XPair, Write, Write)> = {
            let (view, reported) = (&self.wext, &self.cont_var.reported);
            x_pairs(view, |n| reported.contains(&n))
                .into_iter()
                .map(|x| {
                    (
                        x,
                        view.writes[x.cont as usize],
                        view.writes[x.proc as usize],
                    )
                })
                .collect()
        };
        for (XPair { net, lo, hi, .. }, c, p) in found {
            let msg = format!(
                "variable `{}` is driven by {} AND written by {} on bits [{}:{lo}]{}, and a \
                 variable driven by a continuous assignment takes no other writer on those \
                 bits (IEEE 1800-2017 §6.5, §10.3.2)",
                self.mdrv_net_name(net),
                self.mdrv_writer(&c),
                self.mdrv_writer(&p),
                hi - 1,
                self.mdrv_bits_note(net),
            );
            let (span, prefix) = (self.cur_span.take(), std::mem::take(&mut self.cur_prefix));
            self.error(MsgCode::ElabMultidriver, &msg);
            self.cur_span = span;
            self.cur_prefix = prefix;
            self.cont_var.reported.insert(net);
        }
    }

    /// The variable's name in the message (the one name read: the view's net id).
    fn mdrv_net_name(&self, net: u32) -> String {
        self.symbols
            .iter()
            .find(|(_, &id)| id == net)
            .map(|(n, _)| self.display_of(n))
            .unwrap_or_else(|| format!("#{net}"))
    }

    /// " (counted from ..)" when the declared range is not `[w-1:0]` (module head).
    fn mdrv_bits_note(&self, net: u32) -> &'static str {
        match self.nets.get(net as usize) {
            Some(n) if n.lsb == 0 && n.msb + 1 == n.width.max(1) => "",
            _ => " (counted from its least significant bit as 0)",
        }
    }

    /// One writer as the message names it (module head).
    fn mdrv_writer(&self, w: &Write) -> String {
        // `file:line:col` and the instance, or the instance alone when the row has none.
        let at = |i: &ProcIdent| {
            let scope = self.display_of(&i.scope);
            if i.line > 0 {
                format!("at {}:{}:{} in `{scope}`", i.file, i.line, i.col)
            } else {
                format!("in `{scope}` (no source position)")
            }
        };
        match w.writer {
            Writer::Cont { row, kind } => match self.ca_idents.get(row as usize) {
                // With a source map, a port row has no position only for a `.*` connection
                // (`push_cont_assign_port`), which connects the port of the variable's own
                // name (IEEE 1800-2017 §23.3.2.4) ; without one (an AST-only entry) no row has
                // a position.
                Some(i) if kind == PORT && i.line == 0 && self.span_resolver.is_some() => {
                    let var = self.mdrv_net_name(w.net);
                    format!(
                        "the `.*` connection of instance `{}` to its port `{}` (a `.*` \
                         connection has no source position)",
                        self.display_of(&i.scope),
                        var.rsplit('.').next().unwrap_or(&var)
                    )
                }
                Some(i) if kind == PORT && i.line == 0 => format!(
                    "a port connection of instance `{}` (no source position)",
                    self.display_of(&i.scope)
                ),
                Some(i) if kind == PORT => format!(
                    "a port connection of instance `{}` at {}:{}:{}",
                    self.display_of(&i.scope),
                    i.file,
                    i.line,
                    i.col
                ),
                Some(i) => format!("an `assign` or gate output {}", at(i)),
                None => format!("continuous row {row}"),
            },
            Writer::Proc { pid, kind } => {
                let what = if kind == VAR_INIT {
                    "a declaration initializer".to_string()
                } else {
                    format!("`{kind}`")
                };
                let place = self
                    .proc_idents
                    .get(pid as usize)
                    .map(at)
                    .unwrap_or_default();
                let through = if w.via == Some(Via::TaskInline) {
                    " (through a subroutine it calls)"
                } else {
                    ""
                };
                format!("{what} {place}{through}")
            }
            Writer::Body { func, .. } => {
                let name = self
                    .frame_func_names
                    .get(func as usize)
                    .map_or(String::new(), |n| {
                        self.display_of(n.strip_prefix('.').unwrap_or(n))
                    });
                match self.frame_decl_locs.get(func as usize).cloned().flatten() {
                    Some(d) => format!(
                        "the body of subroutine `{name}` declared at {}:{}:{}",
                        d.file, d.line, d.col
                    ),
                    None => format!("the body of subroutine `{name}` (no source position)"),
                }
            }
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
