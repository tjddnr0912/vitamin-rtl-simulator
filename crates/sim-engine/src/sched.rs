//! Event scheduler: time wheel + IEEE-1364 stratified region queues
//! (Active → Inactive → NBA), deterministic ready ordering, the delta loop,
//! the infinite-delta cap, NBA sample/apply, and net-change propagation with
//! edge detection.

use std::collections::BTreeMap;
use std::rc::Rc;

use sim_ir::{BitPacked, EdgeKind, EdgeTerm, Lvalue, RegionTag, SensKind, Terminator, WaitCause};

use elaborate::{ForkModeTable, JoinMode};

use crate::builtins::{format_args_str, write_out};
use crate::eval::{EvalCtx, NetReader};
use crate::exec::{run_process, Kernel, Offsets, Step};
use crate::state::SimState;
use crate::value::Value;
use crate::DeferRegion;

/// v9 `$fread` byte-into-register fill (iverilog-pinned). The `bytes` read from
/// the file (the first is most significant) fill the `w`-bit register's byte
/// slots from the MSB end: byte `i` → slot `capacity-1-i` (slot `k` = bits
/// `[k*8 .. min((k+1)*8, w))`, so the top slot is partial when `w` is not a
/// multiple of 8). Slots left unwritten (a partial fill, when fewer than
/// `capacity` bytes were read) KEEP their prior value, so `prior` is the
/// register's current value. A full read overwrites every slot.
fn fill_reg_slots(prior: &Value, w: u32, bytes: &[u8]) -> Value {
    let capacity = w.div_ceil(8) as usize;
    let mut v = prior.clone();
    for (i, &byte) in bytes.iter().enumerate().take(capacity) {
        let slot = capacity - 1 - i; // from the LSB; byte 0 fills the top slot
        let lo = (slot * 8) as u32;
        let sw = 8u32.min(w - lo); // the top slot may be narrower than a byte
        for k in 0..sw {
            v.set_vu(lo + k, ((byte >> k) & 1) as u64, 0);
        }
    }
    v
}

// ── v9 scanf parser ($fscanf / $sscanf), the FIRST multi-ref-write intercept ──
fn scan_is_ws(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}

/// Next source byte. For `$fscanf` (fd = Some) it comes from the file stream
/// (honoring the `$ungetc`/scanf pushback); for `$sscanf` (fd = None) from the
/// `src` buffer at `*pos` (advanced). Returns None at end of input.
fn scan_next(sched: &mut Scheduler, fd: Option<u32>, src: &[u8], pos: &mut usize) -> Option<u8> {
    match fd {
        Some(fd) => crate::builtins::file_read_byte(sched, fd),
        None => {
            let b = src.get(*pos).copied();
            if b.is_some() {
                *pos += 1;
            }
            b
        }
    }
}

/// Put one over-read byte back. For `$fscanf` it pushes onto the fd pushback
/// stack (so it survives to the next directive AND the next call); for
/// `$sscanf` it rewinds the cursor.
fn scan_unget(sched: &mut Scheduler, fd: Option<u32>, pos: &mut usize, b: u8) {
    match fd {
        Some(fd) => sched.st.read_state.entry(fd).or_default().pushback.push(b),
        None => *pos = pos.saturating_sub(1),
    }
}

/// Pack bytes MSB-first (first byte = most significant) into a Value of width
/// 8×len — the §5.9 string packing the dest write funnel then resizes.
fn scan_pack_str(bytes: &[u8]) -> Value {
    let w = (bytes.len() as u32 * 8).max(8);
    let mut v = Value::zeros(w, false);
    for (i, &by) in bytes.iter().rev().enumerate() {
        let bit = i * 8;
        v.val[bit / 64] |= (by as u64) << (bit % 64);
    }
    v
}

fn scan_write_dst(sched: &mut Scheduler, net: u32, v: Value) {
    let lv = Lvalue {
        chunks: vec![sim_ir::LvalChunk {
            net,
            word: None,
            offset: None,
            width: None,
            kind: sim_ir::SelKind::Bit,
        }],
    };
    let off = sched.resolve_lvalue_offsets(&lv);
    sched.k_write_lvalue(&lv, v, &off);
}

/// Build a numeric scanf value sized to `dst_w` from the collected digit
/// chars. Decimal (radix 10): parse `[sign][digits]` into a 128-bit two's-
/// complement magnitude (wrapping), then sign-extend / truncate to `dst_w`
/// (negative fills the high bits with 1, positive with 0). Hex/oct/bin:
/// per-digit 4-state groups MSB-first (`x`/`X` → an X group, `z`/`Z`/`?` → a
/// Z group), then extend to `dst_w` filling the high bits with x/z when the
/// value's MSB is x/z and with 0 otherwise — the Verilog 4-state extension
/// rule iverilog applies (e.g. `"x"` %h → all-X, `"1x"` → `001x`, `"ff"` →
/// zero-extended).
fn scan_build_numeric(chars: &[u8], radix: u32, dst_w: u32) -> Value {
    let dst_w = dst_w.max(1);
    let mut v = Value::zeros(dst_w, false);
    if radix == 10 {
        let neg = chars.first() == Some(&b'-');
        let mut mag: u128 = 0;
        for &c in chars {
            if let Some(d) = (c as char).to_digit(10) {
                mag = mag.wrapping_mul(10).wrapping_add(d as u128);
            }
        }
        let raw = if neg {
            (mag as i128).wrapping_neg() as u128
        } else {
            mag
        };
        for i in 0..dst_w.min(128) {
            if (raw >> i) & 1 == 1 {
                v.set_vu(i, 1, 0);
            }
        }
        if neg && dst_w > 128 {
            for i in 128..dst_w {
                v.set_vu(i, 1, 0);
            }
        }
        return v;
    }
    let bits_per: u32 = match radix {
        16 => 4,
        8 => 3,
        _ => 1,
    };
    let mask = (1u64 << bits_per) - 1;
    let ndig = chars.len();
    let nbits = ndig as u32 * bits_per;
    for (idx, &c) in chars.iter().enumerate() {
        let base = (ndig - 1 - idx) as u32 * bits_per; // LSB bit of this digit
        let (gv, gu): (u64, u64) = match c {
            b'x' | b'X' => (0, mask),
            b'z' | b'Z' | b'?' => (mask, mask),
            _ => ((c as char).to_digit(radix).unwrap_or(0) as u64, 0),
        };
        for k in 0..bits_per {
            let bit = base + k;
            if bit < dst_w {
                v.set_vu(bit, (gv >> k) & 1, (gu >> k) & 1);
            }
        }
    }
    // 4-state extension: fill the high bits using the value's MSB (x/z → x/z,
    // known → 0).
    if nbits < dst_w && nbits > 0 {
        let (mv, mu) = v.get_vu(nbits - 1);
        if mu != 0 {
            for bit in nbits..dst_w {
                v.set_vu(bit, mv, mu);
            }
        }
    }
    v
}

/// The shared scanf engine. Walks `fmt`, matching whitespace runs (zero+ input
/// ws), literal chars (exact, a mismatch stops), and `%` conversions
/// (`d`/`h`/`x`/`o`/`b`/`c`/`s`, optional `*` suppress + width). Each
/// non-suppressed successful conversion writes `dsts[di]` and counts. Returns
/// the conversion count, or −1 when NO source byte was ever available
/// (genuine EOF; whitespace-only input that converts nothing returns 0).
fn scan_run(sched: &mut Scheduler, fd: Option<u32>, src: &[u8], fmt: &[u8], dsts: &[u32]) -> Value {
    let mut pos = 0usize;
    let mut count: i64 = 0;
    let mut di = 0usize;
    let mut fi = 0usize;

    // The -1 (EOF) vs 0 (matched nothing) decision is based on whether ANY
    // source byte is AVAILABLE at entry — not on whether the scan read one
    // (an empty format / an unsupported first conversion still returns 0 on a
    // non-empty source). Peek one byte and put it back.
    let at_eof = match scan_next(sched, fd, src, &mut pos) {
        Some(b) => {
            scan_unget(sched, fd, &mut pos, b);
            false
        }
        None => true,
    };

    macro_rules! next {
        () => {
            scan_next(sched, fd, src, &mut pos)
        };
    }

    'fmt: while fi < fmt.len() {
        let fc = fmt[fi];
        if scan_is_ws(fc) {
            fi += 1;
            while let Some(b) = next!() {
                if !scan_is_ws(b) {
                    scan_unget(sched, fd, &mut pos, b);
                    break;
                }
            }
            continue;
        }
        if fc != b'%' {
            // literal char: must match exactly (a mismatch stops the scan).
            fi += 1;
            match next!() {
                Some(b) if b == fc => {}
                Some(b) => {
                    scan_unget(sched, fd, &mut pos, b);
                    break 'fmt;
                }
                None => break 'fmt,
            }
            continue;
        }
        // '%' — parse the conversion spec.
        fi += 1;
        if fi < fmt.len() && fmt[fi] == b'%' {
            fi += 1;
            match next!() {
                Some(b'%') => {}
                Some(b) => {
                    scan_unget(sched, fd, &mut pos, b);
                    break 'fmt;
                }
                None => break 'fmt,
            }
            continue;
        }
        let suppress = fi < fmt.len() && fmt[fi] == b'*';
        if suppress {
            fi += 1;
        }
        let mut width = 0usize;
        let mut had_width = false;
        while fi < fmt.len() && fmt[fi].is_ascii_digit() {
            had_width = true;
            // saturate: any width >= the input length already means "read all",
            // so clamping to usize::MAX is harmless and avoids a `*10` overflow.
            width = width
                .saturating_mul(10)
                .saturating_add((fmt[fi] - b'0') as usize);
            fi += 1;
        }
        // an EXPLICIT field width of 0 (`%0d`) reads nothing => the conversion
        // matches nothing; an ABSENT width is unbounded.
        let width = if had_width { width } else { usize::MAX };
        let Some(&conv) = fmt.get(fi) else { break };
        fi += 1;

        let value: Option<Value> = match conv {
            b'c' => {
                // exactly ONE char, NO leading-ws skip — iverilog IGNORES any
                // explicit width on %c (reads one), EXCEPT an explicit `%0c`
                // reads zero (matches nothing).
                let w = if had_width && width == 0 { 0 } else { 1 };
                let mut bytes = Vec::new();
                for _ in 0..w {
                    match next!() {
                        Some(b) => bytes.push(b),
                        None => break,
                    }
                }
                (!bytes.is_empty()).then(|| scan_pack_str(&bytes))
            }
            b's' | b'S' => {
                // skip leading ws, then a ws-delimited run (up to width).
                while let Some(b) = next!() {
                    if !scan_is_ws(b) {
                        scan_unget(sched, fd, &mut pos, b);
                        break;
                    }
                }
                let mut bytes = Vec::new();
                while bytes.len() < width {
                    match next!() {
                        Some(b) if scan_is_ws(b) => {
                            scan_unget(sched, fd, &mut pos, b);
                            break;
                        }
                        Some(b) => bytes.push(b),
                        None => break,
                    }
                }
                (!bytes.is_empty()).then(|| scan_pack_str(&bytes))
            }
            b'd' | b'D' | b'h' | b'H' | b'x' | b'X' | b'o' | b'O' | b'b' | b'B' => {
                let radix: u32 = match conv {
                    b'd' | b'D' => 10,
                    b'o' | b'O' => 8,
                    b'b' | b'B' => 2,
                    _ => 16,
                };
                let is_dec = radix == 10;
                // skip leading ws.
                while let Some(b) = next!() {
                    if !scan_is_ws(b) {
                        scan_unget(sched, fd, &mut pos, b);
                        break;
                    }
                }
                // read [sign?][digits] up to `width` chars. A sign is honored
                // ONLY for %d (iverilog rejects +/- for %h/%o/%b); the 4-state
                // digits x/X/z/Z/? are honored ONLY for %h/%o/%b (iverilog's
                // %d on x/z aborts — out of scope).
                let mut chars: Vec<u8> = Vec::new();
                while chars.len() < width {
                    match next!() {
                        Some(b) => {
                            let ok = (chars.is_empty() && is_dec && (b == b'-' || b == b'+'))
                                || (b as char).is_digit(radix)
                                || (!is_dec && matches!(b, b'x' | b'X' | b'z' | b'Z' | b'?'));
                            if ok {
                                chars.push(b);
                            } else {
                                scan_unget(sched, fd, &mut pos, b);
                                break;
                            }
                        }
                        None => break,
                    }
                }
                let has_digit = chars.iter().any(|&c| {
                    (c as char).is_digit(radix)
                        || (!is_dec && matches!(c, b'x' | b'X' | b'z' | b'Z' | b'?'))
                });
                if !has_digit {
                    None
                } else {
                    let dst_w = dsts
                        .get(di)
                        .and_then(|&n| sched.st.ir.nets.get(n as usize))
                        .map(|nv| nv.width.max(1))
                        .unwrap_or(64);
                    Some(scan_build_numeric(&chars, radix, dst_w))
                }
            }
            _ => break, // unsupported conversion -> stop
        };

        match value {
            Some(v) => {
                if !suppress {
                    if let Some(&net) = dsts.get(di) {
                        scan_write_dst(sched, net, v);
                    }
                    di += 1;
                    count += 1;
                }
            }
            None => break 'fmt, // a matching failure stops the scan
        }
    }

    if count > 0 {
        Value::from_i128(count as i128, 32, true)
    } else if at_eof {
        Value::from_i128(-1, 32, true) // genuine EOF: no source byte available
    } else {
        Value::from_i128(0, 32, true) // input present but nothing converted
    }
}

/// A schedulable process resume. `proc` is a runtime ACTIVITY id (index into
/// `Scheduler::activities`), NOT a declaration index: top-level processes seed
/// activities `0..nproc` 1:1, and `fork` APPENDS child activities (id ≥ nproc).
/// `tie` is the deterministic intra-region order key (doc-06 tie-break); for a
/// top-level activity it equals the declaration index, for a fork child it is the
/// composite of `(parent_tie, child_idx)` from [`compose_child_tie`].
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct Ready {
    pub tie: u32,
    pub proc: u32,
    pub block: u32,
}

/// Per-activity private state. Top-level processes are pre-seeded 1:1 with
/// `ir.processes`; fork children are appended (id ≥ `ir.processes.len()`). The
/// arena only ever GROWS — ids are never reused or reindexed — so any `Ready`
/// stored by value in `wheel`/`waiters`/`net_to_edge` stays valid after a later
/// fork appends.
pub(crate) struct Activity {
    /// Index into `ir.processes` for the body/sensitivity TEMPLATE this activity
    /// runs. Multiple activities may share a template (a child runs a different BB
    /// sub-chain of the SAME `body` Vec as its parent).
    pub template: u32,
    /// Deterministic ordering key (top-level: == template; child: composite).
    pub tie: u32,
    /// If this activity is a fork child, the barrier id it reports completion to.
    /// `None` for top-level processes.
    pub join_ref: Option<u32>,
    /// Role bit: is this a spawned fork child? Children never re-arm.
    pub is_child: bool,
    /// Completion-report guard: set true the FIRST time this child reaches its
    /// barrier's join_bb. A second report is an internal error (double-decrement).
    /// Always `false` for top-level activities.
    pub reported: bool,
    /// P2-E `disable fork`: a killed descendant. Stale queue/waiter/wheel
    /// entries survive; the single dispatch choke (`run_body`) drops them.
    pub dead: bool,
    /// v8 `wait fork`: when `Some`, this (parent) activity is parked until all
    /// its outstanding immediate children report completion. `None` otherwise.
    pub wait_fork: Option<WaitForkPark>,
    /// `true` while this activity is SUSPENDED MID-BODY — it has run from its
    /// `entry` and hit a blocking control (`#delay` / `wait` / `@(...)` /
    /// `fork` / `wait fork`) without yet reaching `Return`. An edge-sensitive
    /// `always`'s permanent `net_to_edge` entry must NOT re-trigger it from
    /// `entry` while busy (IEEE: a process is not re-entered until it completes
    /// and re-arms). Set on `Step::Suspended`, cleared on `Step::Done`. In-body
    /// waiter wakes (the `resume` block) are unaffected — only the static
    /// top-sensitivity re-fire is gated.
    pub busy: bool,
    /// Incarnation counter for this activity SLOT (§16.4 deferred-assert keying):
    /// bumped each time the slot is re-issued to a new fork child via
    /// `free_activities`. Distinguishes a completed activation's pending deferred
    /// report from a later activation that recycled the same `aid`. Top-level
    /// processes never recycle, so their generation stays 0.
    pub gen: u32,
}

/// A process blocked on `wait fork;` (IEEE §9.6.1) — parked until all of its
/// outstanding forked children report completion. Tracked directly on the
/// parent activity (not a `JoinBarrier`) because the cumulative child set spans
/// every prior `fork ... join_none` / surplus `join_any` child, which report to
/// their OWN barriers; the count here is decremented by `on_child_complete`.
pub(crate) struct WaitForkPark {
    /// Continuation BB where the parent resumes once `outstanding` hits 0.
    pub resume_bb: u32,
    /// Count of the parent's still-running immediate children.
    pub outstanding: u32,
}

/// One live fork's join barrier.
pub(crate) struct JoinBarrier {
    /// Activity id of the parent that is (or will be) blocked here.
    pub parent: u32,
    /// The join convergence BB (Fork.join), in the parent's template body. Used as
    /// the child-completion sentinel; NEVER fetched as a real block.
    pub join_bb: u32,
    /// Parent's continuation BB (Fork.resume_bb), in the parent's template body.
    pub resume_bb: u32,
    /// Join mode recovered from the elaborate side table.
    pub mode: JoinMode,
    /// Count of children that have NOT yet reached the join.
    pub outstanding: u32,
    /// Has the parent already been resumed past this barrier? (fire-once guard.)
    pub fired: bool,
}

/// A pending nonblocking LHS update: RHS sampled in Active, applied in NBA.
pub(crate) struct NbaUpdate {
    pub seq: u64,
    pub lhs: Lvalue,
    pub sampled: Value,
    /// Per-chunk `(bit-offset, array-word)` sampled in the ACTIVE region when the
    /// `<=` executed (so `a[i] <= x; i = i+1;` / `m[k] <= x;` use the OLD `i`/`k`),
    /// one per `lhs.chunks`.
    pub offsets: Offsets,
}

/// One simulation time's three region buckets. Active/Inactive hold process
/// resumes + continuous-assign re-evals; NBA holds sampled updates.
#[derive(Default)]
struct SlotQueues {
    active: Vec<Ready>,
    inactive: Vec<Ready>,
}

/// A process suspended on a Wait condition.
struct Waiter {
    cause: WaitCause,
    ready: Ready,
    /// For an IN-BODY `@(sig)`/`@(*)` (a `WaitCause::Level` from `suspend_on`):
    /// the net values snapshot AT ARM TIME, one per `Level.nets`. The waiter fires
    /// only when a net differs from this snapshot — so a change that completed
    /// BEFORE the wait armed (e.g. the t0 `X→init` settle done by another initial
    /// block before `@(sig)` suspended) does NOT spuriously trigger it. `None` for
    /// a STATIC always/comb sensitivity (those re-fire on any change, by design).
    arm: Option<Vec<BitPacked>>,
}

/// One INERTIAL-delay continuous-assign write: `(cont-assign index,
/// generation, lhs, value, per-chunk (offset, word))`. Applied when the
/// simulation reaches the scheduled tick IF the generation still matches
/// `ca_gen[ci]` — a later RHS change bumps the generation, so the stale
/// pending write is silently dropped (IEEE inertial pulse filtering: a pulse
/// narrower than the delay never reaches the LHS; iverilog-pinned live).
type DelayedWrite = (u32, u64, Lvalue, Value, Offsets);

pub(crate) struct Scheduler<'a, 'ir> {
    pub st: &'a mut SimState<'ir>,
    /// Current time's Active/Inactive buckets.
    cur: SlotQueues,
    /// NBA region (applied as a batch when Active+Inactive empty).
    nba: Vec<NbaUpdate>,
    nba_seq: u64,
    /// Future events keyed by absolute tick.
    wheel: BTreeMap<u64, Vec<(RegionTag, Ready)>>,
    /// Processes blocked on Wait conditions.
    waiters: Vec<Waiter>,
    /// WAITER-POOL p2: running counts of `Expr`/`Level` waiters in `waiters`,
    /// maintained exactly at the two push sites (`suspend_on` Expr, `arm_
    /// sensitivity`/`suspend_on` Level) and the single `propagate_changes`
    /// `retain` removal. When a count is 0 the matching `retain` match-arm
    /// cannot fire, so `propagate_changes` skips building the corresponding
    /// `expr_now`/`level_fire` precompute buffer entirely — an all-`Edge`
    /// design (every flop `always @(posedge clk)`, no `wait(expr)`/`@*`) pays
    /// nothing for those scans. Byte-identical: the skipped buffer was unused.
    n_expr_waiters: usize,
    n_level_waiters: usize,
    /// Activity id currently executing a body (set by `run_body`, the single
    /// dispatch choke) — `disable fork` kills THIS activity's descendants.
    cur_aid: u32,
    /// net → edge-sensitive process resumes.
    net_to_edge: Vec<Vec<(EdgeKind, Ready)>>,
    /// Per-activity private state. `index == Ready.proc` (activity id). Seeded 1:1
    /// with `ir.processes` at t0; fork appends children (append-only, never reused).
    activities: Vec<Activity>,
    /// Live fork join barriers. `index == JoinBarrier id` (a child's `join_ref`).
    /// Slots are RECYCLED through `free_barriers` once every child has reported
    /// (P3-1) — no live reference can outlast that point, so no ABA.
    barriers: Vec<JoinBarrier>,
    /// P3-1 free lists: completed fork-child activity slots / fully-drained
    /// barrier slots, recycled by the next `exec_fork`. Without these a
    /// `forever fork … join_none` loop grows both arenas O(timesteps)
    /// (~800 MB over 10M cycles). Determinism: the free-list state is a pure
    /// function of the (deterministic) execution, and ids are internal — VCD/
    /// stdout bytes are unchanged (P5 gate + corpus enforce).
    free_activities: Vec<u32>,
    free_barriers: Vec<u32>,
    /// Join-mode side table from elaborate, keyed `(template, join_bb)`.
    fork_modes: ForkModeTable,
    /// Last RHS value seen per cont-assign — only used by DELAYED `assign #d`
    /// (change detection, so a delayed write schedules once per RHS change).
    last_ca: Vec<Option<Value>>,
    /// S1: last value this cont-assign actually DROVE onto the net (the gate's
    /// own output node — a superseded/inertially-cancelled pending write never
    /// updates it). This is the baseline for the per-bit rise/fall/turnoff delay
    /// (IEEE 1364 §7.14: a transition's delay is measured from the value the net
    /// CURRENTLY holds, not the previous RHS — they differ on inertial supersede).
    last_ca_drv: Vec<Option<Value>>,
    /// Per-cont-assign schedule generation — bumped on every new RHS change,
    /// invalidating any pending write carrying an older generation (the
    /// inertial cancel; see `DelayedWrite`).
    ca_gen: Vec<u64>,
    /// MULTI-DRIVER: nets driven by ≥2 cont-assigns that are ALL whole-net,
    /// non-delayed targets → `(net, driver cont-assign indices, resolution kind)`.
    /// kind 0=WIRE (z yields; equal keeps; conflict→x), 1=WAND (wired-AND),
    /// 2=WOR (wired-OR) — z is the identity for all three. Settled by per-bit
    /// 4-state resolution instead of last-write-wins. Empty for every single-driver
    /// design ⇒ byte-identical (multi-driver was E3001-rejected before). Partial/
    /// dynamic/array/delayed overlaps stay E3001 (whole-net buses are the v1 scope).
    md_nets: Vec<(u32, Vec<usize>, u8)>,
    /// MULTI-DRIVER: per-cont-assign flag — this driver is a member of an
    /// `md_nets` group, so the per-driver individual write in `settle_cont_assigns`
    /// is SKIPPED (the net is written once by the resolution instead).
    ca_md: Vec<bool>,
    /// Pending inertial-delay cont-assign writes, keyed by absolute apply tick.
    delayed_ca: BTreeMap<u64, Vec<DelayedWrite>>,
    /// Transport NBAs (`q <= #d v`, v5 increment A): updates due at a FUTURE
    /// tick's NBA region. Drained into `nba` when time advances to the key;
    /// `apply_nba`'s global-seq sort keeps statement order across both paths.
    delayed_nba: BTreeMap<u64, Vec<NbaUpdate>>,
    delta_count: u64,
    max_deltas: u64,
    time_limit: Option<u64>,
    /// Scratch buffers reused across `propagate_changes` calls (take/restore —
    /// the alternative per-call `Vec::new` allocates on every delta).
    scratch_changed: Vec<u32>,
    /// GLITCH/SELF-RETRIG: per-changed-net `(net, slot_edge_mask, blocking_writer)`.
    /// `mask` = the net's intra-slot bit0 edge summary (`SimState::slot_edge`), so
    /// both the static edge-wake pass (a) and the in-body `Edge` waiter pass (b)
    /// fire on a glitch the endpoint `prev/cur` compare would lose (single write ⇒
    /// mask == `edge_fires(kind, prev, cur)` ⇒ byte-identical). `blocking_writer` =
    /// the activity that authored the change via a BLOCKING write (`u32::MAX`
    /// otherwise); snapshotting it here lets the `retain` closure suppress
    /// re-firing a process on a net it itself wrote without re-borrowing `self.st`.
    scratch_edges: Vec<(u32, u8, u32)>,
    /// N4 / multi-edge dedup: per-`propagate_changes` markers so a process
    /// sensitive to MULTIPLE nets that ALL change in the SAME delta is woken
    /// EXACTLY ONCE (IEEE §9: an `always @(posedge c1 or posedge c2)` ticks once
    /// per slot even when both edges occur together — vita previously pushed it
    /// twice → ran the body twice). `seen[proc]` marks an already-queued proc this
    /// pass; `marked` records the touched indices so cleanup is O(#fired), not
    /// O(#procs). Empty/untouched for single-edge designs ⇒ byte-identical.
    scratch_edge_seen: Vec<bool>,
    scratch_edge_marked: Vec<u32>,
    /// C-FORCE-REEVAL-p2: reused buffer of the force KEYS (target nets) selected
    /// for re-eval this delta (changed-net-triggered ∪ always-reeval), kept
    /// sorted to preserve the BTreeMap re-eval order contract.
    scratch_force_keys: Vec<u32>,
    /// WAITER-POOL: reused `expr_now`/`level_fire` precompute buffers in
    /// `propagate_changes` (was two fresh `Vec<bool>` per non-idle delta).
    scratch_expr_now: Vec<bool>,
    scratch_level_fire: Vec<bool>,
    /// VM-REGPOOL: recycled bytecode-VM register/offset files (was a fresh
    /// `vec![None; n]` pair per `vm_exec` activation). Leased in `vm_run_body`.
    vm_regs_pool: Vec<crate::backend::RegFile>,
    vm_offs_pool: Vec<crate::backend::OffFile>,
    /// Recycled wheel-bucket Vecs: `wheel.remove` would otherwise drop one
    /// bucket allocation per distinct simulation time (O(timesteps) churn).
    bucket_pool: Vec<Vec<(RegionTag, Ready)>>,
    /// Generation of the CURRENTLY-running activity (`activities[cur_aid].gen`),
    /// set alongside `cur_aid`. Keys §16.4 deferred reports so a recycled `aid`
    /// cannot flush a completed prior instance's pending report.
    cur_gen: u32,
}

/// Why the run ended (scheduler precedence order).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinishReason {
    Finish,
    Stop,
    Quiescent,
    DeltaLimit,
    Error,
}

impl<'a, 'ir> Scheduler<'a, 'ir> {
    pub fn new(
        st: &'a mut SimState<'ir>,
        max_deltas: u64,
        time_limit: Option<u64>,
        fork_modes: ForkModeTable,
    ) -> Self {
        let nnets = st.nets.len();
        let nca = st.ir.cont_assigns.len();
        // MULTI-DRIVER: identify nets driven by ≥2 cont-assigns that are ALL
        // whole-net (single chunk, no word/offset/width select) and non-delayed.
        // A net with ANY partial/dynamic/array/delayed driver is excluded (those
        // overlaps stay E3001 at elaborate). Computed once from `ir.cont_assigns`
        // — no sidecar. Empty unless a design actually has a multi-driven net.
        let mut whole: std::collections::BTreeMap<u32, Vec<usize>> =
            std::collections::BTreeMap::new();
        let mut excluded: std::collections::BTreeSet<u32> = std::collections::BTreeSet::new();
        for (ci, ca) in st.ir.cont_assigns.iter().enumerate() {
            let is_whole = ca.delay.is_none() && ca.lhs.chunks.len() == 1 && {
                let c = &ca.lhs.chunks[0];
                c.word.is_none() && c.offset.is_none() && c.width.is_none()
            };
            if is_whole {
                whole.entry(ca.lhs.chunks[0].net).or_default().push(ci);
            } else {
                for c in &ca.lhs.chunks {
                    excluded.insert(c.net);
                }
            }
        }
        let mut md_nets: Vec<(u32, Vec<usize>, u8)> = Vec::new();
        let mut ca_md = vec![false; nca];
        for (net, cis) in whole {
            if cis.len() >= 2 && !excluded.contains(&net) {
                for &ci in &cis {
                    ca_md[ci] = true;
                }
                // WAND/WOR resolution kind (default WIRE).
                let kind = if st.wired_and_nets.contains(&net) {
                    1
                } else if st.wired_or_nets.contains(&net) {
                    2
                } else {
                    0
                };
                md_nets.push((net, cis, kind));
            }
        }
        Scheduler {
            st,
            cur: SlotQueues::default(),
            nba: Vec::new(),
            nba_seq: 0,
            wheel: BTreeMap::new(),
            waiters: Vec::new(),
            n_expr_waiters: 0,
            n_level_waiters: 0,
            cur_aid: 0,
            net_to_edge: vec![Vec::new(); nnets],
            activities: Vec::new(),
            barriers: Vec::new(),
            free_activities: Vec::new(),
            free_barriers: Vec::new(),
            fork_modes,
            last_ca: vec![None; nca],
            last_ca_drv: vec![None; nca],
            ca_gen: vec![0; nca],
            md_nets,
            ca_md,
            delayed_ca: BTreeMap::new(),
            delayed_nba: BTreeMap::new(),
            delta_count: 0,
            max_deltas,
            time_limit,
            scratch_changed: Vec::new(),
            scratch_edges: Vec::new(),
            scratch_edge_seen: Vec::new(),
            scratch_edge_marked: Vec::new(),
            scratch_force_keys: Vec::new(),
            scratch_expr_now: Vec::new(),
            scratch_level_fire: Vec::new(),
            vm_regs_pool: Vec::new(),
            vm_offs_pool: Vec::new(),
            bucket_pool: Vec::new(),
            cur_gen: 0,
        }
    }

    // ── t0 init ──────────────────────────────────────────────────────────

    /// Settle continuous assigns to a fixpoint, re-evaluating every cont-assign in
    /// declaration order until no net changes. `None` ⇒ could not converge within
    /// the delta budget (a cont-assign oscillator) — the caller MUST stop the run,
    /// else an unbounded `assign`-only loop would spin `max_deltas` iters on EVERY
    /// outer delta and the outer `DeltaLimit` would never fire (cur.active stays
    /// empty). `Some(changed)` ⇒ converged; `changed` is whether ANY net moved, so
    /// the caller can run edge/level propagation on the cont-assign-driven nets
    /// (e.g. a port-bound clock `child.clk = parent.c` whose posedge must reach the
    /// child's `always @(posedge clk)`). One delta budget per time-step (doc-06).
    #[must_use]
    pub fn settle_cont_assigns(&mut self) -> Option<bool> {
        let mut any = false;
        loop {
            let mut changed = false;
            for ci in 0..self.st.ir.cont_assigns.len() {
                if self.st.ir.cont_assigns[ci].delay.is_some() {
                    continue; // a delayed `assign #d` is scheduled below, not now
                }
                if self.ca_md[ci] {
                    continue; // MULTI-DRIVER member: written once by resolution below
                }
                let ca_rhs = self.st.ir.cont_assigns[ci].rhs;
                let lhs = self.st.ir.cont_assigns[ci].lhs.clone();
                let v = self.eval_for_lvalue(&lhs, ca_rhs); // CONTEXT-SIZED to lhs width
                let offs = self.resolve_lvalue_offsets(&lhs); // dynamic index NOW (settle time)
                changed |= self.st.write_lvalue(&lhs, v, &offs);
            }
            // MULTI-DRIVER: resolve each multi-driven net from ALL its whole-net
            // drivers by 4-state wire resolution, then write the net once. Part of
            // the same fixpoint (a driver's RHS can depend on another resolved net).
            for mi in 0..self.md_nets.len() {
                let net = self.md_nets[mi].0;
                let net_w = self.st.nets[net as usize].width;
                // Accumulator starts at all-Z (the wire-resolution identity).
                let mut acc = Value::zeros(net_w, false);
                for w in 0..acc.val.len() {
                    acc.val[w] = u64::MAX;
                    acc.unk[w] = u64::MAX;
                }
                acc.mask_top();
                let cis = self.md_nets[mi].1.clone();
                let kind = self.md_nets[mi].2;
                for ci in cis {
                    let ca_rhs = self.st.ir.cont_assigns[ci].rhs;
                    let lhs = self.st.ir.cont_assigns[ci].lhs.clone();
                    let v = self.eval_for_lvalue(&lhs, ca_rhs);
                    match kind {
                        1 => resolve_wand_into(&mut acc, &v),
                        2 => resolve_wor_into(&mut acc, &v),
                        _ => resolve_wire_into(&mut acc, &v),
                    }
                }
                let lhs = self.st.ir.cont_assigns[self.md_nets[mi].1[0]].lhs.clone();
                let offs = self.resolve_lvalue_offsets(&lhs);
                changed |= self.st.write_lvalue(&lhs, acc, &offs);
            }
            if !changed {
                break;
            }
            any = true;
            self.delta_count += 1;
            if self.delta_count > self.max_deltas {
                self.fatal_delta_limit();
                return None;
            }
        }
        // Delayed `assign #d y = rhs`: the zero-delay fixpoint has settled, so
        // the RHS is stable. On each RHS-value CHANGE, schedule an INERTIAL
        // write of the new value at `now + d` — bumping `ca_gen[ci]` cancels
        // any still-pending older write for THIS assign (a pulse narrower
        // than d never lands; a pulse of EXACTLY d survives because pending
        // writes apply at the tick start, before processes re-change the RHS
        // — both iverilog-pinned live, 2026-06-12).
        for ci in 0..self.st.ir.cont_assigns.len() {
            let Some(d) = self.st.ir.cont_assigns[ci].delay else {
                continue;
            };
            let ca_rhs = self.st.ir.cont_assigns[ci].rhs;
            let lhs = self.st.ir.cont_assigns[ci].lhs.clone();
            let v = self.eval_for_lvalue(&lhs, ca_rhs);
            if self.last_ca[ci].as_ref() == Some(&v) {
                continue; // RHS unchanged → no new scheduled write
            }
            // S1: the per-bit rise/fall/turnoff delay is measured from the value
            // the net CURRENTLY holds — i.e. the last value this assign actually
            // DROVE (`last_ca_drv`), NOT the previous RHS (`last_ca`). The two
            // differ on inertial supersede (a new RHS change before the prior
            // delayed write lands): the pending write never updated `last_ca_drv`,
            // so the baseline correctly stays the net's present output.
            let old = self.last_ca_drv[ci].clone();
            self.last_ca[ci] = Some(v.clone());
            self.ca_gen[ci] += 1;
            let offs = self.resolve_lvalue_offsets(&lhs);
            // S1: when this cont-assign has differing rise/fall/turnoff delays,
            // the net updates atomically at `now + max(per-changed-bit dest
            // delay)`. Absent a sidecar entry, the uniform `d` is used (byte-
            // identical to the old behaviour).
            let eff_d = match self.st.ca_delays.get(&(ci as u32)) {
                Some(&(rise, fall, toff)) => transition_delay(old.as_ref(), &v, rise, fall, toff),
                None => d,
            };
            let tick = self.st.now + eff_d as u64;
            self.delayed_ca.entry(tick).or_default().push((
                ci as u32,
                self.ca_gen[ci],
                lhs,
                v,
                offs,
            ));
        }
        Some(any)
    }

    /// Arm processes at t0 per Verilog initial/always semantics.
    pub fn arm_processes(&mut self) {
        // Pre-seed top-level activities 1:1 with process declarations. `tie ==
        // template == declaration index` so existing single-process ordering is
        // byte-identical to before the activity-id refactor.
        self.free_activities.clear();
        self.free_barriers.clear();
        self.activities = (0..self.st.ir.processes.len() as u32)
            .map(|pi| Activity {
                template: pi,
                tie: pi,
                join_ref: None,
                is_child: false,
                reported: false,
                dead: false,
                wait_fork: None,
                busy: false,
                gen: 0,
            })
            .collect();

        // TOTAL-OR-FATAL mode gate: every `Terminator::Fork` in every body MUST
        // have a matching `(template, join_bb)` entry in `fork_modes`. A miss means
        // a keying mismatch / lost sidecar (the trailer rides outside the schema
        // gate, so a truncated `.velab` can reach here) — P1-7: emit a FATAL
        // diagnostic and end the run at t0 (was: panic), never a fabricated
        // default that would silently miscompile join_any/join_none.
        let mut missing: Option<(u32, u32)> = None;
        for (proc_id, p) in self.st.ir.processes.iter().enumerate() {
            for blk in &p.body {
                if let Terminator::Fork { join, .. } = &blk.term {
                    if !self.fork_modes.contains_key(&(proc_id as u32, *join)) {
                        missing = Some((proc_id as u32, *join));
                        break;
                    }
                }
            }
        }
        if let Some((tmpl, join)) = missing {
            self.fatal_fork_mode_missing(tmpl, join);
            return; // nothing armed; run() sees `finished` and ends immediately
        }

        for aid in 0..self.activities.len() as u32 {
            let tmpl = self.activities[aid as usize].template as usize;
            // P2-E: `final` blocks are Initial-shaped in the IR but never
            // armed — `run_finals` executes them after the main loop ends.
            if self.st.final_procs.contains(&(tmpl as u32)) {
                continue;
            }
            let tie = self.activities[aid as usize].tie;
            let entry = self.st.ir.processes[tmpl].entry;
            let ready = Ready {
                tie,
                proc: aid,
                block: entry,
            };
            match self.st.ir.processes[tmpl].sensitivity.kind {
                // initial + combinational/latch blocks run at t0.
                SensKind::Initial | SensKind::Comb | SensKind::Latch => {
                    push_sorted(&mut self.cur.active, ready);
                }
                // edge / level blocks wait for the first event (no t0 run).
                SensKind::Edge | SensKind::Level => self.arm_sensitivity(aid),
            }
        }
    }

    /// Register an always block's static sensitivity as waiters / edge map. `pi`
    /// is an ACTIVITY id; the body/sensitivity is resolved through its template.
    /// Only ever called for TOP-LEVEL activities (children have no static
    /// sensitivity — they run a sub-chain of their template body).
    fn arm_sensitivity(&mut self, pi: u32) {
        let tmpl = self.activities[pi as usize].template as usize;
        let tie = self.activities[pi as usize].tie;
        let p = &self.st.ir.processes[tmpl];
        let entry = p.entry;
        let ready = Ready {
            tie,
            proc: pi,
            block: entry,
        };
        match p.sensitivity.kind {
            SensKind::Edge => {
                let edges: Vec<EdgeTerm> = p.sensitivity.edges.clone();
                for et in edges {
                    self.net_to_edge[et.net as usize].push((et.kind, ready));
                }
            }
            // Level AND inferred-combinational (`@*`/`always_comb`/`always_latch`,
            // whose `Comb`/`Latch` edges hold the elaborate-inferred read-set):
            // re-fire on ANY change of a read net. Empty edges (e.g. a bare
            // self-timed `always` that re-arms via in-body #/@) register nothing.
            SensKind::Level | SensKind::Comb | SensKind::Latch => {
                let nets: Vec<u32> = p.sensitivity.edges.iter().map(|e| e.net).collect();
                if !nets.is_empty() {
                    self.waiters.push(Waiter {
                        cause: WaitCause::Level { nets },
                        ready,
                        arm: None, // static sensitivity: re-fire on any change
                    });
                    self.n_level_waiters += 1; // WAITER-POOL p2
                }
            }
            _ => {}
        }
    }

    /// P2-E: execute every `final` block ONCE, ascending ProcId, after the
    /// main loop ends (any finish reason). Bodies are zero-time by elaborate
    /// contract (timing controls rejected), so each runs entry→Return in one
    /// activation; a `$finish` inside one is absorbed (the run is already
    /// ending — IEEE end-of-sim re-entry must not recurse).
    pub(crate) fn run_finals(&mut self) {
        let finals: Vec<u32> = self.st.final_procs.iter().copied().collect();
        // KNOWN LIMITATION (SVA-REST liveness, documented — not silent): a `$finish`
        // reached in the Active region terminates the timestep WITHOUT draining its
        // pending edge-triggered processes (the `Step::Finish` arm returns before
        // `propagate_changes`). So when `$finish` coincides EXACTLY with the assertion
        // clock edge — `initial #N $finish` with N landing on a sampling posedge — the
        // clocked liveness checker does not sample that final edge (the same pre-existing
        // behavior that makes a clocked `cnt<=cnt+1` miss a finish-coincident edge), and
        // the end-of-sim `final` obligation check reads the prior edge's pend. A
        // correct fix needs `$finish`/timestep-drain ordering changes (broad golden-VCD
        // impact) — deferred to a dedicated scheduler slice. Workaround: offset `$finish`
        // from the sampling edge (e.g. a non-edge finish time, or `#1 $finish`).
        for pid in finals {
            if (pid as usize) >= self.activities.len() {
                continue; // defensive: stale side table
            }
            let entry = self.st.ir.processes[pid as usize].entry;
            let _ = self.run_body(pid, entry);
            // flush any $strobe/$monitor the final body queued (postponed
            // machinery is per-timestep; end-of-sim is the last timestep).
            self.flush_postponed();
        }
    }

    // ── main loop ────────────────────────────────────────────────────────

    /// THE single process-body dispatch seam (P4). The interpreter is the
    /// always-available reference; the Bytecode backend (P0a) routes codegen-able
    /// bodies (the P9 suspend-free allow-list) to the VM and falls back to the
    /// interpreter for the rest. A design routinely MIXES the two — e.g. `always_ff` is
    /// codegen-able while its testbench's `initial #1 …` is not (P9b proves the mix is
    /// byte-identical to all-interpreter). The codegen-ability decision + compile is
    /// memoized per template by `vm_compiled` (decide-once cache).
    fn run_body(&mut self, proc: u32, block: u32) -> Step {
        // P2-E `disable fork`: a killed activity's stale resume entries
        // (slot queues, waiters, delay wheel) all funnel through here — drop.
        if self.activities.get(proc as usize).is_some_and(|a| a.dead) {
            return Step::Done;
        }
        // N4 clocking: commit handlers are applied at EDGE DETECTION in
        // `propagate_changes` (before the Active batch), never run here — so no
        // hot-path check is needed in `run_body`.
        self.cur_aid = proc;
        self.cur_gen = self.activities[proc as usize].gen;
        // SELF-RETRIG: tag blocking writes made by THIS body to their author, so
        // it is not re-triggered by its own write. Cleared on return — NBA apply,
        // cont-assign settle and clocking commit (all outside `run_body`) then
        // author their writes as `None` (= re-fire normally).
        self.st.blocking_writer = Some(proc);
        let step = match self.st.backend {
            crate::Backend::Interpreter => run_process(self, proc, block),
            crate::Backend::Bytecode => {
                let tmpl = self.activity_template(proc) as usize;
                match self.st.vm_compiled(tmpl) {
                    Some(body) => self.vm_run_body(proc, tmpl, block, body),
                    None => run_process(self, proc, block),
                }
            }
        };
        self.st.blocking_writer = None;
        step
    }

    /// Bytecode-VM body entry (Stage C / C2). The P9 predicate (via `vm_compiled`) has
    /// confirmed this body is suspend-free; `body` is its compiled form, handed in as an
    /// owned `Rc` so this `&mut self` kernel call cannot alias the cache (§2.3).
    ///
    /// The VM bypasses `run_process` — the SOLE writer of `cur_time_mult` — so the
    /// PROLOGUE sets it from THIS process's module multiplier exactly as exec.rs:80-87
    /// does, before `vm_exec` evaluates any `$time`/`$realtime`. The per-activation
    /// termination guard then lives inside `vm_exec` (mirror of exec.rs:176-180).
    fn vm_run_body(
        &mut self,
        proc: u32,
        tmpl: usize,
        block: u32,
        body: Rc<crate::backend::CompiledBody>,
    ) -> Step {
        self.st.cur_time_mult = self
            .st
            .proc_multipliers
            .get(tmpl)
            .copied()
            .unwrap_or(1)
            .max(1);
        // VM-REGPOOL: lease the register/offset files from the pool, sized to this
        // body, and return them afterwards (a `pop` yields an OWNED buffer, so it no
        // longer borrows `self` and cannot alias the `&mut self` kernel call).
        let mut regs = self.vm_regs_pool.pop().unwrap_or_default();
        regs.clear();
        regs.resize(body.nregs as usize, None);
        let mut offs = self.vm_offs_pool.pop().unwrap_or_default();
        offs.clear();
        offs.resize(body.noffs as usize, None);
        let step = crate::backend::vm_exec(self, &body, proc, block, &mut regs, &mut offs);
        regs.clear();
        self.vm_regs_pool.push(regs);
        offs.clear();
        self.vm_offs_pool.push(offs);
        step
    }

    pub fn run(&mut self) -> FinishReason {
        // N4 clocking: seed the preponed snapshot for the FIRST time slot (t=0) —
        // a clocking edge at t=0 then samples the init values. No-op without
        // clocking blocks ⇒ byte-identical.
        self.st.snapshot_preponed();
        loop {
            if self.st.finished {
                return self.finish_kind();
            }
            // Drain the current time to a stable point.
            self.delta_count = 0;
            loop {
                // B1: a frame-call runaway latched during a prior region's eval
                // (or the settle just below) surfaces here — every region action
                // `continue`s back to this loop top, so one check covers them all.
                if self.check_call_fatal() {
                    return FinishReason::Error;
                }
                // ACTIVE: continuous assigns settle, then drain processes.
                match self.settle_cont_assigns() {
                    None => return FinishReason::DeltaLimit, // cont-assign oscillator
                    // A settle that moved nets may have produced an EDGE on a
                    // cont-assign-driven net (a port-bound clock). Run change
                    // propagation so those edges/level-waiters fire before we
                    // decide the timestep is stable.
                    Some(true) => self.propagate_changes(),
                    Some(false) => {}
                }
                // A cont-assign RHS (`assign y = deep_recursive_fn(x);`) can latch
                // the fatal during settle, then `break` out below if all region
                // buckets are empty — so it MUST be caught HERE, before the break.
                if self.check_call_fatal() {
                    return FinishReason::Error;
                }
                if !self.cur.active.is_empty() {
                    // Take the batch so wakes triggered DURING it land in a fresh
                    // `cur.active`; iterate borrowed (`Ready: Copy`) so the Vec can
                    // be handed back below — consuming it dropped one allocation
                    // per delta.
                    let mut batch = std::mem::take(&mut self.cur.active);
                    for &r in &batch {
                        if self.st.finished {
                            return self.finish_kind();
                        }
                        // B1: a runaway latched in a PRIOR process body of THIS
                        // batch ends the run before the next body can read a
                        // corrupt static slab (the residue is never observed).
                        if self.check_call_fatal() {
                            return FinishReason::Error;
                        }
                        match self.run_body(r.proc, r.block) {
                            // P1-6 (IEEE 1364-2005 §5.4/§17): drain the CURRENT
                            // timestep's postponed region ($strobe/$monitor) before
                            // terminating — Icarus/VCS parity. $fatal/$stop are
                            // $finish-class terminations and drain identically.
                            Step::Finish => {
                                self.st.finished = true;
                                self.drain_deferred_on_finish();
                                self.flush_postponed();
                                return FinishReason::Finish;
                            }
                            Step::Stop => {
                                self.st.finished = true;
                                self.drain_deferred_on_finish();
                                self.flush_postponed();
                                return FinishReason::Stop;
                            }
                            Step::Fatal => {
                                self.st.finished = true;
                                self.st.had_fatal = true;
                                self.drain_deferred_on_finish();
                                self.flush_postponed();
                                return FinishReason::Error;
                            }
                            // Track mid-body suspension so the permanent
                            // edge-sensitivity entry does not re-trigger a
                            // process that is still parked inside its body.
                            Step::Suspended => {
                                if let Some(a) = self.activities.get_mut(r.proc as usize) {
                                    a.busy = true;
                                }
                            }
                            Step::Done => {
                                if let Some(a) = self.activities.get_mut(r.proc as usize) {
                                    a.busy = false;
                                }
                            }
                        }
                    }
                    // Recycle the drained batch Vec when no process was woken
                    // mid-batch (the overwhelmingly common case).
                    batch.clear();
                    if self.cur.active.is_empty() {
                        self.cur.active = batch;
                    }
                    self.propagate_changes();
                    self.delta_count += 1;
                    if self.delta_count > self.max_deltas {
                        self.fatal_delta_limit();
                        return FinishReason::DeltaLimit;
                    }
                    continue;
                }
                // INACTIVE (#0): promote to Active.
                if !self.cur.inactive.is_empty() {
                    self.cur.active = std::mem::take(&mut self.cur.inactive);
                    // GATED-CLOCK: a #0 batch is a NEW event cluster — an edge it
                    // produces (e.g. an independent `negedge rst` scheduled via #0)
                    // must be able to re-fire a process already woken this timestep.
                    self.reset_edge_seen_marks();
                    self.delta_count += 1;
                    if self.delta_count > self.max_deltas {
                        self.fatal_delta_limit();
                        return FinishReason::DeltaLimit;
                    }
                    continue;
                }
                // NBA: apply the sampled batch.
                if !self.nba.is_empty() {
                    // GATED-CLOCK: the NBA region is a NEW event cluster — an edge
                    // an NBA update produces must re-fire a process already woken
                    // this timestep (an independent `rst <= 0` re-fires `@(… or
                    // negedge rst)`). Reset BEFORE the post-apply propagation.
                    self.reset_edge_seen_marks();
                    self.apply_nba();
                    self.propagate_changes();
                    self.delta_count += 1;
                    if self.delta_count > self.max_deltas {
                        self.fatal_delta_limit();
                        return FinishReason::DeltaLimit;
                    }
                    continue;
                }
                // OBSERVED (IEEE 1800 §4.4 / §16.4): `assert #0` deferred reports
                // mature here — Active/Inactive/NBA empty, cont-assigns at
                // fixpoint, nets settled, time NOT advanced. A matured action that
                // re-activates a process re-drains Active/NBA before Reactive (the
                // `continue` re-runs the region cascade from the top).
                if !self.st.postponed.deferred_observed.is_empty() {
                    if let Some(step) = self.mature_deferred(DeferRegion::Observed) {
                        self.st.finished = true;
                        if let Step::Fatal = step {
                            self.st.had_fatal = true;
                        }
                        self.flush_postponed();
                        return match step {
                            Step::Stop => FinishReason::Stop,
                            Step::Fatal => FinishReason::Error,
                            _ => FinishReason::Finish,
                        };
                    }
                    self.propagate_changes();
                    self.delta_count += 1;
                    if self.delta_count > self.max_deltas {
                        self.fatal_delta_limit();
                        return FinishReason::DeltaLimit;
                    }
                    continue;
                }
                // REACTIVE (IEEE 1800 §4.4 / §16.4): `assert final` deferred
                // reports mature AFTER Observed, BEFORE Postponed.
                if !self.st.postponed.deferred_reactive.is_empty() {
                    if let Some(step) = self.mature_deferred(DeferRegion::Reactive) {
                        self.st.finished = true;
                        if let Step::Fatal = step {
                            self.st.had_fatal = true;
                        }
                        self.flush_postponed();
                        return match step {
                            Step::Stop => FinishReason::Stop,
                            Step::Fatal => FinishReason::Error,
                            _ => FinishReason::Finish,
                        };
                    }
                    self.propagate_changes();
                    self.delta_count += 1;
                    if self.delta_count > self.max_deltas {
                        self.fatal_delta_limit();
                        return FinishReason::DeltaLimit;
                    }
                    continue;
                }
                break; // time-step stable
            }

            // POSTPONED REGION (IEEE 1364-2005 §5.4): now == settled time, all
            // region buckets (Active/Inactive/NBA) empty, cont-assigns at
            // fixpoint, time NOT yet advanced. Reads settled `cur` net values.
            // Drain strobes (call order) then the monitor (print-on-change).
            self.flush_postponed();

            // Advance time to the next scheduled tick — the earliest of a wheel
            // event OR a pending transport-delay cont-assign write.
            let next = match [
                self.wheel.keys().next().copied(),
                self.delayed_ca.keys().next().copied(),
                self.delayed_nba.keys().next().copied(),
            ]
            .into_iter()
            .flatten()
            .min()
            {
                None => return FinishReason::Quiescent,
                Some(t) => t,
            };
            if let Some(lim) = self.time_limit {
                if next > lim {
                    return FinishReason::Quiescent;
                }
            }
            self.st.now = next;
            // GATED-CLOCK: a new timestep is a fresh edge-collapse cluster.
            self.reset_edge_seen_marks();
            // N4 clocking: take the PREPONED snapshot of clocking inputs at the
            // START of the new time slot (before any slot activity — the true
            // preponed value entering the slot). No-op when no clocking blocks ⇒
            // byte-identical for the whole non-clocking corpus.
            self.st.snapshot_preponed();
            // No prev snapshot needed here (R2): at the settled point every
            // mutation path has been swept — `propagate_changes` step (c)
            // already refreshed `prev` for every changed net, and unchanged
            // nets have `prev == cur` by induction (the only `prev` writers
            // are step (c) and the t0 constructor, both setting prev = cur).
            // The old full-net `snapshot_prev` was therefore a no-op pass
            // costing O(nets) per timestep (512 idle nets ≈ half the
            // nets-heavy wall-clock). Soundness is pinned by the byte-compare
            // suites (staged/threads/corpus/differential).
            // Apply inertial-delay cont-assign writes due at this tick; propagate
            // so edges/level-waiters on the delayed net fire (these are NET writes,
            // not process resumes, so the loop-top settle would not see them).
            // A write whose generation was superseded by a later RHS change is
            // STALE — dropped (the inertial cancel).
            if let Some(writes) = self.delayed_ca.remove(&next) {
                let mut moved = false;
                for (ci, gen, lhs, v, offs) in writes {
                    if self.ca_gen[ci as usize] != gen {
                        continue;
                    }
                    // S1: this write actually LANDS — record it as the gate's
                    // output for the next transition's delay baseline. (Stale
                    // superseded writes `continue` above, so they never update it.)
                    self.last_ca_drv[ci as usize] = Some(v.clone());
                    moved |= self.st.write_lvalue(&lhs, v, &offs);
                }
                if moved {
                    self.propagate_changes();
                }
            }
            // Transport NBAs due now join this tick's NBA region; the global
            // seq sort in `apply_nba` interleaves them with NBAs scheduled
            // during the tick in original statement order.
            if let Some(ups) = self.delayed_nba.remove(&next) {
                self.nba.extend(ups);
            }
            let mut events = self.wheel.remove(&next).unwrap_or_default();
            for (region, ready) in events.drain(..) {
                match region {
                    RegionTag::Inactive => push_sorted(&mut self.cur.inactive, ready),
                    _ => push_sorted(&mut self.cur.active, ready),
                }
            }
            // Return the drained bucket to the pool for the next schedule().
            self.bucket_pool.push(events);
            // a fresh time may also need cont-assign settle before draining (loop top).
        }
    }

    fn finish_kind(&self) -> FinishReason {
        if self.st.had_fatal {
            FinishReason::Error
        } else {
            FinishReason::Finish
        }
    }

    /// B1 frame-call: surface a runaway-recursion fatal LATCHED on the `&self`
    /// read path. `eval_call` cannot return a `Step`, so it sets `call_fatal`
    /// (a `Cell`) and finishes its in-flight eval with X; the scheduler polls
    /// this at the region seams where an eval can have fired (cont-assign
    /// settle, a process body, a deferred-action arg, a Branch cond). On the
    /// first consume it mirrors the user-`$fatal` termination (drain deferred,
    /// flush postponed, latch finished/had_fatal) so the run ends before any
    /// subsequent call can read a corrupt static slab. Returns true once it is
    /// consumed (the caller then returns `FinishReason::Error`).
    fn check_call_fatal(&mut self) -> bool {
        if self.st.call_fatal.get() && !self.st.finished {
            self.st.finished = true;
            self.st.had_fatal = true;
            self.drain_deferred_on_finish();
            self.flush_postponed();
            return true;
        }
        false
    }

    // ── §16.4 deferred immediate assertions ───────────────────────────────

    /// `builtins::dispatch` calls this at its top for every SysTask. Returns
    /// `true` if the call was INTERCEPTED as a deferred-assert marker/action (so
    /// `dispatch` must do nothing further), `false` if it should run normally.
    ///
    /// A MARKER cancels any prior pending report for this assertion instance —
    /// keyed `(marker_sid, cur_aid, cur_gen)` so flush-on-re-reach hits only THIS
    /// activation, never a recycled-slot predecessor (§16.4). An ACTION renders
    /// its text NOW (reach-time arg values, §16.4.3) and enqueues/REPLACES it for
    /// region maturation.
    pub(crate) fn try_defer(
        &mut self,
        which: sim_ir::SysTaskId,
        fmt: Option<u32>,
        args: &[u32],
        sid: u32,
    ) -> bool {
        if let Some(&region) = self.st.defer_marks.get(&sid) {
            let key = (sid, self.cur_aid, self.cur_gen);
            match region {
                DeferRegion::Observed => {
                    self.st.postponed.deferred_observed.remove(&key);
                }
                DeferRegion::Reactive => {
                    self.st.postponed.deferred_reactive.remove(&key);
                }
            }
            return true; // marker is a no-op (suppressed empty $display)
        }
        if let Some(&(marker, region)) = self.st.defer_acts.get(&sid) {
            // §16.4.3: render the action text NOW, at reach (sampling reach-time
            // arg values / `$time` / `%m`). A severity task renders with no
            // default radix (matching `run_severity`); a plain print uses its
            // b/o/h radix.
            let radix = if self.st.severities.contains_key(&sid) {
                None
            } else {
                self.st.radixes.get(&sid).copied()
            };
            let message = crate::builtins::format_args_str(self, fmt, args, radix);
            let report = crate::state::DeferredReport {
                action_sid: sid,
                which,
                message,
            };
            let key = (marker, self.cur_aid, self.cur_gen);
            match region {
                DeferRegion::Observed => {
                    self.st.postponed.deferred_observed.insert(key, report);
                }
                DeferRegion::Reactive => {
                    self.st.postponed.deferred_reactive.insert(key, report);
                }
            }
            return true;
        }
        false
    }

    /// §16.4: drain ONE deferred-assert maturation queue, emitting each surviving
    /// pending report (text already rendered at reach). A severity action routes
    /// to the diagnostic stream + exit class ($fatal aborts); a plain print goes
    /// to stdout. Deterministic `(marker, aid, gen)` BTreeMap order. Returns
    /// `Some(step)` if a deferred `$fatal` matured.
    fn mature_deferred(&mut self, region: DeferRegion) -> Option<Step> {
        let map = match region {
            DeferRegion::Observed => std::mem::take(&mut self.st.postponed.deferred_observed),
            DeferRegion::Reactive => std::mem::take(&mut self.st.postponed.deferred_reactive),
        };
        if map.is_empty() {
            return None;
        }
        let mut term: Option<Step> = None;
        for (_key, rpt) in map {
            if let Some(sev) = self.st.severities.get(&rpt.action_sid).copied() {
                match crate::builtins::emit_severity_message(self, sev, rpt.message) {
                    crate::builtins::Ctl::Fatal => {
                        term = Some(Step::Fatal);
                        break;
                    }
                    crate::builtins::Ctl::Finish => {
                        term = Some(Step::Finish);
                        break;
                    }
                    crate::builtins::Ctl::Stop => {
                        term = Some(Step::Stop);
                        break;
                    }
                    crate::builtins::Ctl::Continue => {}
                }
            } else {
                // Plain $display/$write deferred action: stdout, newline for the
                // Display family ($write keeps none).
                let mut line = rpt.message;
                if !matches!(rpt.which, sim_ir::SysTaskId::Write) {
                    line.push('\n');
                }
                write_out(self.st, &line);
            }
        }
        term
    }

    /// §16.4 termination drain: mature the current slot's pending deferred
    /// reports (Observed then Reactive) before a `$finish`/`$stop`/`$fatal`
    /// exit, mirroring the postponed drain — a verdict already evaluated this
    /// slot is not lost. Any further `$fatal` is ignored (already terminating).
    fn drain_deferred_on_finish(&mut self) {
        let _ = self.mature_deferred(DeferRegion::Observed);
        let _ = self.mature_deferred(DeferRegion::Reactive);
    }

    // ── postponed region ($strobe FIFO drain + $monitor change-detect) ─────

    /// IEEE 1364-2005 §5.4 postponed region. Called at the settled point of each
    /// timestep (Active/Inactive/NBA empty, cont-assigns at fixpoint, `now` =
    /// settled time, time NOT yet advanced). Read-only w.r.t. net state: it only
    /// EVALUATES ExprIds (reading settled `cur` net values via `NetReader`) and
    /// writes to `self.st.out`. NOTE: this flush has nothing to do with
    /// `prev`/`snapshot_prev` — those are edge-detection state that `run()` rolls
    /// at the *start of the next* timestep (after time advance); the postponed
    /// render reads only the settled `cur` values and is independent of edge state.
    ///
    /// ORDER (frozen, documented for the golden gate): all strobes first in call
    /// order, then the single monitor line. IEEE leaves this tie-break
    /// implementation-defined; vita freezes strobes-then-monitor for byte-stable
    /// 3-OS golden output.
    fn flush_postponed(&mut self) {
        // `$time`/`$realtime` inside a postponed render must scale by the
        // REGISTERING module's multiplier, not the scheduler's live
        // `cur_time_mult` (which now holds the LAST-run process's `M` — a
        // different module under mixed timescales). Each `FmtCapture` carries its
        // own `time_mult`; we drive `cur_time_mult` from it per render and RESTORE
        // the entering value on exit so nothing downstream (e.g. a cont-assign
        // `$time` at the next loop-top settle) observes a render's multiplier.
        let saved_mult = self.st.cur_time_mult;
        // (a) STROBES — drain the FIFO in call order, render NOW (settled values),
        //     print, then CLEAR (one-shot per call: a strobe never repeats next
        //     step unless its statement re-executes and re-registers).
        if !self.st.postponed.strobes.is_empty() {
            // `mem::take` first to end the immutable borrow that `format_args_str`
            // needs against `&self` (mirrors `apply_nba`'s `mem::take(&mut nba)`).
            let batch = std::mem::take(&mut self.st.postponed.strobes);
            for cap in &batch {
                self.st.cur_time_mult = cap.time_mult; // registering module's M
                self.st.cur_scope = cap.scope.clone(); // registering module's %m
                let mut line = format_args_str(self, cap.fmt, &cap.args, cap.radix);
                line.push('\n');
                write_out(self.st, &line);
            }
            // `batch` dropped here; `self.st.postponed.strobes` is now empty.
        }

        // (b) MONITOR — IEEE 1364-2005 §17.1: reprint whenever any monitored
        //     expression VALUE changes (4-state-aware), NOT when the rendered
        //     string changes. We therefore evaluate the arg ExprIds to a
        //     `Vec<Value>` and compare against the stored baseline with
        //     `Value`'s derived `PartialEq` (exact `(val, unk)` bit-plane
        //     equality). Only when the value list differs (or was never seeded:
        //     establishment / replace) do we render + print and re-seed.
        //
        //     Borrow shape: hoist EVERYTHING out of the `&self.st.postponed`
        //     borrow into locals (copy `enabled`, copy `fmt`, clone `args`,
        //     `take` the previous `last_vals`) and DROP that borrow before any
        //     `&self`-eval / `&mut self.st`-write. No NLL dependence on a binding
        //     surviving across the render — render-then-record, zero overlap.
        //     `Option::take` is used so the old baseline is moved out (not
        //     cloned) and the slot is rewritten unconditionally below.
        let mon = match self.st.postponed.monitor.as_mut() {
            Some(m) => {
                let fmt = m.cap.fmt;
                let args = m.cap.args.clone();
                let tmult = m.cap.time_mult; // monitoring module's M (see (a) above)
                let radix = m.cap.radix;
                let scope = m.cap.scope.clone();
                let prev = m.last_vals.take(); // moves baseline out; slot now None
                Some((fmt, args, tmult, radix, scope, prev))
            }
            // no monitor established → nothing to do. (DISABLED is no longer
            // gated here: the establishment print must fire even when disabled,
            // so the enable check moved INTO the change-detection below.)
            _ => None,
        };
        if let Some((fmt, args, tmult, radix, scope, prev)) = mon {
            if fmt.is_none() && args.is_empty() {
                // No-arg monitor (`$monitor;` → fmt=None, args=[]) prints nothing —
                // not even a bare newline. Guarded so a future bare-`$monitor`
                // lowering cannot silently inject a lone "\n" into golden RTL output
                // (see §7.4). Seed an empty baseline (`[]` == `[]` keeps it silent
                // forever); a zero-expression monitor has no value to track.
                if let Some(m) = self.st.postponed.monitor.as_mut() {
                    m.last_vals = Some(Vec::new());
                }
            } else {
                // Evaluate every monitored expression to a settled 4-state Value,
                // scaling `$time`/`$realtime` by the monitoring module's M.
                // `self.eval` builds `EvalCtx` from `self.st`, reading settled `cur`.
                // IEEE §17.1.3 (P0-9): a DIRECT `$time`/`$realtime` argument does
                // NOT participate in change detection (it is rendered, but time
                // advancing must not retrigger the monitor) — filter those out of
                // the comparison vector. Change compare is on the BIT PLANES
                // (width/val/unk) only, not the derived `PartialEq` (which also
                // compares the static signed/is_real metadata).
                self.st.cur_time_mult = tmult;
                self.st.cur_scope = scope;
                let is_direct_time = |eid: u32| {
                    matches!(
                        self.st.ir.exprs[eid as usize],
                        sim_ir::Expr::SysFunc {
                            which: sim_ir::SysFuncId::Time | sim_ir::SysFuncId::Realtime,
                            ..
                        }
                    )
                };
                // v9 rank 6: an ESTABLISHMENT (or `$monitoron`-forced) flush —
                // `prev == None` — ALWAYS prints, bypassing `$monitoroff` (the
                // establishment line is bound to the `$monitor` time-step; a
                // same-tick or prior `$monitoroff` does not cancel it). Only
                // CHANGE-triggered reprints (`prev == Some`) obey the global flag.
                let was_establishment = prev.is_none();
                let monitor_disabled = self.st.postponed.monitor_disabled;
                // P3-2: reuse the previous baseline Vec — evaluate each arg,
                // compare against the old slot, then overwrite IN PLACE (one
                // allocation per monitor lifetime, not one per timestep).
                let (changed, cur_vals) = match prev {
                    None => (
                        true, // establishment / replace → print
                        args.iter()
                            .filter(|&&eid| !is_direct_time(eid))
                            .map(|&eid| self.eval(eid))
                            .collect::<Vec<Value>>(),
                    ),
                    Some(mut old) => {
                        let mut changed = false;
                        let mut i = 0usize;
                        for &eid in args.iter().filter(|&&eid| !is_direct_time(eid)) {
                            let v = self.eval(eid);
                            match old.get_mut(i) {
                                Some(slot) => {
                                    if !(slot.width == v.width
                                        && slot.val == v.val
                                        && slot.unk == v.unk)
                                    {
                                        changed = true;
                                    }
                                    *slot = v;
                                }
                                None => {
                                    changed = true;
                                    old.push(v);
                                }
                            }
                            i += 1;
                        }
                        if i != old.len() {
                            changed = true;
                            old.truncate(i);
                        }
                        (changed, old)
                    }
                };
                // establishment / forced reprint prints unconditionally; a
                // change-reprint prints only while the monitor is enabled.
                let do_print = was_establishment || (changed && !monitor_disabled);
                if do_print {
                    let mut line = format_args_str(self, fmt, &args, radix);
                    line.push('\n');
                    write_out(self.st, &line);
                }
                // Re-seed the baseline with the freshly-evaluated values regardless
                // of whether we printed: an unchanged step must keep the same
                // baseline, and a printed step adopts the new one.
                if let Some(m) = self.st.postponed.monitor.as_mut() {
                    m.last_vals = Some(cur_vals);
                }
            }
        }
        // Restore the entering multiplier (see the save at the top of this fn).
        self.st.cur_time_mult = saved_mult;
    }

    // ── NBA ──────────────────────────────────────────────────────────────

    fn apply_nba(&mut self) {
        let mut batch = std::mem::take(&mut self.nba);
        batch.sort_by_key(|u| u.seq);
        for u in batch.drain(..) {
            self.st.write_lvalue(&u.lhs, u.sampled, &u.offsets);
        }
        // Hand the drained Vec back so the next timestep's NBA pushes reuse its
        // capacity (consuming it dropped one allocation per NBA flush).
        self.nba = batch;
    }

    // ── change propagation + edge detection ──────────────────────────────

    /// Diff prev→cur for every net; fire static edge-sensitive `always` blocks,
    /// in-body `Wait{Edge}`/`Wait{Level}` waiters; then refresh prev. Dependent
    /// continuous assigns are covered by `settle_cont_assigns` (re-evaluated each
    /// delta), so no explicit re-enqueue is needed here.
    ///
    /// ORDER IS LOAD-BEARING: every edge comparison (static map AND waiters) must
    /// read the PRE-change `prev`, so the prev-refresh is deferred to the very end
    /// of this pass. (A previous version refreshed prev per-net inside the static
    /// loop, which silently masked all in-body `@(edge)` waits — their later
    /// `prev`/`cur` read saw `prev == cur` → no edge.)
    /// Re-evaluate every live expression force (BTree order = deterministic)
    /// and re-pin its target. The forcing module's time multiplier is restored
    /// around each eval so `$time` in a force RHS renders with the right scale.
    fn reeval_active_forces(&mut self) {
        // ACTIVE pins only — both force (§9.3.2) and proc-assign (§9.3.1)
        // re-evaluate continuously; a latent (force-displaced) assign does not
        // run until release re-pins it.
        // C-FORCE-REEVAL-p2: select only the forces whose inputs changed this
        // delta (via the net→forces sidecar) PLUS every always-reeval force
        // (volatile $time/$random RHS, or zero-net const RHS — these yield a
        // fresh value with frozen inputs, so a net-sensitivity skip would
        // silently FREEZE them). The selected keys are SORTED, so the executed
        // subset re-evaluates in the exact BTreeMap (ascending-key) order the
        // old all-forces loop used for that subset — a force's re-pin can write
        // a net feeding another force, so order is load-bearing for output.
        //
        // p2 FIX (force-feeds-force CHAIN): `reeval_active_forces` runs ONCE per
        // `propagate_changes`, and the dirty list it triggers off is then CONSUMED
        // by the sweep below. So a force re-pin that writes a net feeding ANOTHER
        // force must re-trigger that downstream force WITHIN THIS CALL — there is
        // no later sweep that re-runs it (the new dirtiness is transient, gone by
        // the next delta). We therefore iterate to a FIXPOINT: each pass re-pins a
        // worklist of forces, records which design nets actually changed value,
        // then re-selects the forces those nets feed and repeats until quiescent.
        // The old all-reeval code converged the whole DAG across successive
        // sweeps; here we converge it in one call (input nets are frozen — only
        // force-target nets move — so the fixpoint always settles, bounded by the
        // number of live forces; a guard caps pathological cyclic forces).
        let mut keys = std::mem::take(&mut self.scratch_force_keys);
        keys.clear();
        // Seed the worklist from the externally-changed nets. `self.st.dirty`
        // (still unconsumed at this point in `propagate_changes`) is the same
        // changed-net set that gates this call; using it raw (not the
        // cur!=prev–filtered set) is a safe superset — it never skips a force the
        // old unconditional loop ran. Always-reeval forces (volatile / zero-net)
        // join the FIRST pass unconditionally.
        for &n in &self.st.dirty {
            if let Some(set) = self.st.force_net_to_forces.get(&n) {
                keys.extend(set.iter().copied());
            }
        }
        keys.extend(self.st.force_always_reeval.iter().copied());
        keys.sort_unstable();
        keys.dedup();

        let saved = self.st.cur_time_mult;
        // Fixpoint bound: each force can re-pin its target at most once per pass,
        // and a pass only re-runs forces fed by a net that CHANGED VALUE in the
        // previous pass. A non-cyclic force DAG settles in ≤ (#live forces) passes;
        // the cap defends against a pathological cyclic force chain (which would
        // also oscillate under the old per-sweep convergence — IEEE leaves such a
        // zero-delay loop unstable). +2 slack over the live-force count.
        let mut budget = self.st.active_forces.len().saturating_add(2);
        while !keys.is_empty() && budget > 0 {
            budget -= 1;
            // `next_changed` collects the design nets whose VALUE actually moved
            // from a force re-pin in THIS pass — the trigger set for the next.
            let mut next_changed: Vec<u32> = Vec::new();
            for &k in &keys {
                // Look up live registry data by key. A key could have been removed
                // by an earlier force's re-pin side effects; skip a vanished entry.
                let Some((lv, rhs, mult, _weak)) = self.st.active_forces.get(&k).cloned() else {
                    continue;
                };
                self.st.cur_time_mult = mult;
                let v = self.eval_for_lvalue(&lv, rhs);
                // `force_write` returns whether the target net's value moved. Only
                // a real change can re-trigger a downstream force; an unchanged
                // re-pin is a no-op (preserves the old same-value-drop behavior).
                if self.st.force_write(&lv, v) {
                    next_changed.push(k);
                }
            }
            // Re-select the forces fed by the nets that just changed value. A
            // force whose source net did not move is NOT re-run (byte-identical to
            // a steady chain). De-dup + sort to keep the ascending-key order.
            keys.clear();
            for &n in &next_changed {
                if let Some(set) = self.st.force_net_to_forces.get(&n) {
                    keys.extend(set.iter().copied());
                }
            }
            keys.sort_unstable();
            keys.dedup();
        }
        self.st.cur_time_mult = saved;
        keys.clear();
        self.scratch_force_keys = keys;
    }

    /// GATED-CLOCK: clear the per-process "edge-woken this CLUSTER" markers. An
    /// edge-sensitive `always` fires at most once per ACTIVE-region settle cluster
    /// (so a cont-assign-derived/gated clock whose edge lands a later delta of the
    /// SAME cluster — `gclk = clk & en` — collapses to one firing, matching
    /// iverilog). But a GENUINELY NEW event batch from a region boundary (#0
    /// Inactive promotion, NBA apply, or time advance) is a fresh cluster: those
    /// edges (e.g. an independent `negedge rst` scheduled via `#0`/NBA into a later
    /// delta) MUST be able to re-fire the process, so the markers reset at each
    /// boundary. Within a cluster the markers persist across deltas (cont-assigns
    /// settle one delta at a time). Clears only the touched entries (O(#woken)).
    fn reset_edge_seen_marks(&mut self) {
        for &p in &self.scratch_edge_marked {
            if let Some(slot) = self.scratch_edge_seen.get_mut(p as usize) {
                *slot = false;
            }
        }
        self.scratch_edge_marked.clear();
    }

    fn propagate_changes(&mut self) {
        // IEEE §9.3.2 continuous force: while a force with an expression RHS is
        // live, re-evaluate it whenever ANYTHING changed this delta and re-pin
        // the target through the force funnel. Over-sensitivity is harmless (a
        // same-value re-pin is dropped by the write funnel) and the re-force
        // lands in the SAME sweep below via the dirty list. Designs without a
        // live force never enter (empty registry).
        if !self.st.active_forces.is_empty() && !self.st.dirty.is_empty() {
            self.reeval_active_forces();
        }
        // Take/restore the scratch buffers: this runs once per delta, and a
        // fresh Vec pair per call was measurable allocator traffic.
        let mut changed_nets = std::mem::take(&mut self.scratch_changed);
        changed_nets.clear();
        // Dirty sweep (scheduler R2): every net that took an ACTUAL bit change
        // since the last sweep is a candidate — the previous full O(nets)
        // `cur != prev` scan taxed every idle net on every delta (512 idle
        // regs ≈ 9x the 8-net wall-clock). Sorting restores the old scan's
        // ascending order (byte-identity). GLITCH FIX: `dirty` membership ALONE
        // is the changed set — a net is on `dirty` iff `note_change` saw a real
        // transition, so an A→B→A round-trip (cur ends == prev) is STILL a
        // change the observer must see (IEEE §9 fires a glitch once). The old
        // `cur != prev` endpoint filter silently dropped exactly those nets,
        // leaving `always @(a)`/`@(posedge a)` un-fired. The dropped set is
        // EMPTY for any design without a same-slot round-trip, so non-glitch
        // designs stay byte-identical; the bit0 edge KIND for the wake is
        // recovered from `slot_edge` (built per transition, not from endpoints).
        let mut cand = std::mem::take(&mut self.st.dirty);
        cand.sort_unstable();
        for &n in &cand {
            self.st.dirty_flag[n as usize] = false;
            changed_nets.push(n);
        }
        cand.clear();
        self.st.dirty = cand; // hand the capacity back for the next writes
        if changed_nets.is_empty() {
            self.scratch_changed = changed_nets;
            return;
        }

        // Precompute the per-net intra-slot edge mask for each changed net. The
        // mask (`slot_edge`) is maintained by the write funnel for every
        // `is_edge_target` net and is fresh for any net on `changed_nets` (the
        // first-dirty reset ran this slot). For a non-edge-target net the mask is
        // unused below (it is in no `net_to_edge` list and no `Edge` waiter), so
        // a stale value is harmless. Snapshotting into `edges` lets the `retain`
        // closure in (b) read the mask without re-borrowing `self.st`.
        let mut edges = std::mem::take(&mut self.scratch_edges);
        edges.clear();
        edges.extend(changed_nets.iter().map(|&net| {
            (
                net,
                self.st.slot_edge[net as usize],
                self.st.last_blocking_writer[net as usize],
            )
        }));

        // (a) wake statically edge-sensitive `always` processes.
        // Multi-edge dedup: a process sensitive to SEVERAL nets that all change in
        // THIS delta must be queued ONCE (IEEE §9: `always @(posedge c1 or posedge
        // c2)` ticks once even when both edges land together). Borrow the markers
        // out for the pass (take/restore avoids a per-delta alloc).
        let mut edge_seen = std::mem::take(&mut self.scratch_edge_seen);
        let mut edge_marked = std::mem::take(&mut self.scratch_edge_marked);
        if edge_seen.len() < self.activities.len() {
            edge_seen.resize(self.activities.len(), false);
        }
        for &(net, mask, writer) in &edges {
            // P3-4: index loop instead of cloning the per-net waiter list every
            // delta — the body only pushes into `cur.active`, never mutates
            // `net_to_edge`, so the indexed re-borrow is sound.
            for k in 0..self.net_to_edge[net as usize].len() {
                let (kind, ready) = self.net_to_edge[net as usize][k];
                // Skip a process that is still SUSPENDED MID-BODY: its static
                // edge entry is permanent (never deregistered), but IEEE does
                // not re-enter an `always` until it completes and re-arms. Its
                // legitimate in-body wake comes via the waiter path (b) below.
                // GLITCH: fire from the intra-slot `mask` (== endpoint for a
                // single write) so an A→B→A clock pulse still ticks once.
                // SELF-RETRIG: skip a process firing on a net IT itself
                // blocking-wrote — it already saw the value before re-arming.
                if edge_fires_slot(mask, kind)
                    && !self.activities[ready.proc as usize].busy
                    && writer != ready.proc
                {
                    // N4: a clocking-commit handler applies `preponed_buf → holding`
                    // HERE — at edge DETECTION, before the Active batch drains — so
                    // EVERY same-slot reader of `cb.sig` sees the committed sample,
                    // independent of process tie order. Crucially this fixes the
                    // cross-hierarchy case (`dut.cb.sig` read from a parent whose
                    // process tie sorts BEFORE the submodule's handler). The handler
                    // carries no real body, so it is not run in Active.
                    // `commit_clocking` returns true iff `proc` was a handler.
                    if self.st.commit_clocking(ready.proc) {
                        continue;
                    }
                    // Dedup: this proc may also be registered on another net that
                    // fired this delta — push it to Active only once.
                    let p = ready.proc as usize;
                    if edge_seen[p] {
                        continue;
                    }
                    edge_seen[p] = true;
                    edge_marked.push(ready.proc);
                    push_sorted(&mut self.cur.active, ready);
                }
            }
        }
        // GATED-CLOCK: do NOT clear the markers here — `edge_seen` is now
        // TIMESTEP-scoped (reset at time advance), so a process woken by one edge
        // this timestep is NOT re-woken by a later-delta edge of its sensitivity
        // (a derived/gated clock). `edge_marked` accumulates this timestep's woken
        // procs across all of its deltas for the O(#woken) reset at time advance.
        // Within a single delta the same-net/multi-net dedup is unchanged (the
        // `edge_seen[p]` guard is set during the pass). Restore for the next pass.
        self.scratch_edge_seen = edge_seen;
        self.scratch_edge_marked = edge_marked;

        // (b) wake in-body waiters. `wait(expr)` (WaitCause::Expr) RE-CHECKS its
        // predicate against the post-change net values and resumes only when it
        // becomes true — pre-evaluated here because the `retain` closure cannot
        // also borrow `&self` for `truthy`. (Previously Expr fell through `_ =>
        // true` and never woke, so a `false→true` transition hung the process.)
        // WAITER-POOL p2: only FILL `expr_now` when an `Expr` waiter actually
        // exists. When `n_expr_waiters == 0` the `retain` `WaitCause::Expr` arm
        // below cannot match any waiter, so `expr_now[wi]` is never indexed —
        // the empty buffer is sound. The fill scanned the FULL waiter vector
        // every non-idle delta even for all-`Edge` designs.
        let mut expr_now = std::mem::take(&mut self.scratch_expr_now); // WAITER-POOL
        expr_now.clear();
        if self.n_expr_waiters > 0 {
            expr_now.extend(self.waiters.iter().map(|w| match &w.cause {
                WaitCause::Expr { expr } => self.truthy(*expr),
                _ => false,
            }));
        }
        // Pre-compute Level firing (the retain closure cannot also borrow `&self`):
        // an in-body `@(sig)` (arm=Some) fires when a net differs from its ARM-TIME
        // value; a static sensitivity (arm=None) fires on any net change.
        // p2: same zero-skip guard — no `Level` waiter ⇒ `level_fire` is unused.
        let mut level_fire = std::mem::take(&mut self.scratch_level_fire); // WAITER-POOL
        level_fire.clear();
        if self.n_level_waiters > 0 {
            level_fire.extend(self.waiters.iter().map(|w| {
                match (&w.cause, &w.arm) {
                    // An in-body `@(sig)`/`@(*)` (arm=Some) fires when a watched net
                    // differs from its ARM-TIME value. NOTE: this intentionally does
                    // NOT consult `changed_nets` — a same-slot change that happened
                    // BEFORE the arm (e.g. `@(*)` arming at t0 right after its inputs
                    // were initialized) is already reflected in `arm`, so firing on
                    // dirtiness would spuriously re-run it in the arming slot. The
                    // cost is one narrow miss: an in-body `@(a)` waiting through a
                    // glitch that RETURNS to the arm value (ROADMAP §4.5.4) — a far
                    // rarer corner than the `@(*)` t0 behavior this preserves.
                    // SELF-RETRIG: a watched net the waiter's OWN process
                    // blocking-wrote does not wake it (it saw the value pre-rearm).
                    (WaitCause::Level { nets }, Some(arm)) => {
                        nets.iter().zip(arm).any(|(&n, av)| {
                            self.st.nets[n as usize].cur != *av
                                && self.st.last_blocking_writer[n as usize] != w.ready.proc
                        })
                    }
                    (WaitCause::Level { nets }, None) => nets.iter().any(|&n| {
                        changed_nets.contains(&n)
                            && self.st.last_blocking_writer[n as usize] != w.ready.proc
                    }),
                    _ => false,
                }
            }));
        }
        let mut woken: Vec<Ready> = Vec::new();
        let mut wi = 0usize;
        // WAITER-POOL p2: tally removed Expr/Level waiters here (the retain
        // closure cannot borrow `&mut self`), apply to the running counts after.
        // A removed Level waiter that RE-ARMS re-pushes via `arm_sensitivity`/
        // `suspend_on` on resume → the count re-increments there; a consumed
        // Expr/Edge does not re-push, so the decrement is net-correct.
        let mut removed_expr = 0usize;
        let mut removed_level = 0usize;
        self.waiters.retain(|w| {
            let keep = match &w.cause {
                // Level + inferred-comb: fire per the pre-computed arm/any-change test.
                WaitCause::Level { .. } => !level_fire[wi],
                // GLITCH: an in-body `@(posedge x)` fires from the intra-slot
                // mask, so a clock pulse that glitches back wakes the waiter.
                // SELF-RETRIG: but not on a net this waiter's own process wrote.
                WaitCause::Edge { net, kind } => !edges.iter().any(|&(en, mask, writer)| {
                    en == *net && edge_fires_slot(mask, *kind) && writer != w.ready.proc
                }),
                // wait(expr): consume + resume only when the predicate is now true.
                WaitCause::Expr { .. } => !expr_now[wi],
                _ => true,
            };
            if !keep {
                woken.push(w.ready); // re-armed on resume (Level/Expr) or consumed (Edge)
                match &w.cause {
                    WaitCause::Expr { .. } => removed_expr += 1,
                    WaitCause::Level { .. } => removed_level += 1,
                    _ => {}
                }
            }
            wi += 1;
            keep
        });
        self.n_expr_waiters -= removed_expr;
        self.n_level_waiters -= removed_level;
        for r in woken {
            push_sorted(&mut self.cur.active, r);
        }

        // (c) refresh prev LAST, now that every edge observer has run.
        // Per-field clone_from reuses prev's Vec allocations (a whole-struct
        // clone allocated two fresh Vecs per changed net per delta).
        for &net in &changed_nets {
            let slot = &mut self.st.nets[net as usize];
            slot.prev.val.clone_from(&slot.cur.val);
            slot.prev.unk.clone_from(&slot.cur.unk);
        }
        self.scratch_changed = changed_nets;
        self.scratch_edges = edges;
        expr_now.clear();
        self.scratch_expr_now = expr_now; // WAITER-POOL: hand the capacity back
        level_fire.clear();
        self.scratch_level_fire = level_fire;
    }

    /// P2-3: every delta-limit overflow path (t0/run-loop settle, the
    /// interpreter's in-body activation guard via `mark_fatal`, the VM guard via
    /// `k_mark_fatal`) funnels here — emit ONE `F-RUN-NO-CONVERGE` diagnostic
    /// (was: exit 1 with zero diagnostic lines) and flag the fatal exit class.
    /// The `had_fatal` check keeps it single-shot per run.
    fn fatal_delta_limit(&mut self) {
        if !self.st.had_fatal {
            use diag::{Diagnostic, LogEvent, MsgCode, Severity, TimeStamp};
            self.st.sink.emit(LogEvent::Diagnostic(Diagnostic {
                severity: Severity::Fatal,
                code: MsgCode::RunNoConverge,
                message: format!(
                    "did not converge: delta limit ({}) exceeded at time {} \
                     (zero-delay loop / combinational oscillation)",
                    self.max_deltas, self.st.now
                ),
                location: None,
                context: Vec::new(),
                sim_time: Some(TimeStamp { ticks: self.st.now }),
            }));
        }
        self.st.had_fatal = true;
    }

    // ── expression eval + executor-facing API (called from exec.rs) ──────

    pub(crate) fn eval(&self, eid: u32) -> Value {
        let ctx = EvalCtx {
            ir: self.st.ir,
            nets: self.st,
            now: self.st.now,
            wt: &self.st.wt,
            time_mult: self.st.cur_time_mult,
            rng: &self.st.rng,
            plusargs: &self.st.plusargs,
        };
        ctx.eval(eid)
    }

    pub(crate) fn truthy(&self, eid: u32) -> bool {
        let ctx = EvalCtx {
            ir: self.st.ir,
            nets: self.st,
            now: self.st.now,
            wt: &self.st.wt,
            time_mult: self.st.cur_time_mult,
            rng: &self.st.rng,
            plusargs: &self.st.plusargs,
        };
        ctx.truthy(eid)
    }

    /// Evaluate `rhs` in the context of `lhs`'s width (IEEE assignment rule):
    /// width = max(lhs_width, self_width(rhs)); sign = rhs self-sign (lhs sign
    /// does NOT propagate).
    pub(crate) fn eval_for_lvalue(&self, lhs: &Lvalue, rhs: u32) -> Value {
        let lw = self.st.lvalue_width(lhs);
        let sw = self.st.wt.get(rhs);
        let ctx_w = lw.max(sw.width);
        self.eval_ctx_top(rhs, ctx_w, sw.signed)
    }

    /// Evaluate each LHS chunk's bit-offset expression NOW (read-only EvalCtx),
    /// returning one offset per chunk (0 for a whole-net `None` chunk). The
    /// `&mut self` write path has no EvalCtx, so dynamic indices like `a[i]` are
    /// resolved here at the correct sampling moment. An X/Z or unresolvable index
    /// yields the `u32::MAX` sentinel → `write_chunk` drops the bit (out-of-range
    /// no-op), matching the READ side where `eval_select` returns X for `a[x]`.
    pub(crate) fn resolve_lvalue_offsets(&self, lhs: &Lvalue) -> Offsets {
        // ── v5 ⑤: single-chunk assoc-element lvalue → i64 key side-channel ──
        // (the SIGNED key domain cannot ride the u32 pairs). Concat/offset
        // shapes fall through to the pair path, where the dyn write funnel
        // degrades them loud+ignored (outside the MVP; ⑥ rejects them).
        if let [c] = lhs.chunks.as_slice() {
            if c.offset.is_none() && c.width.is_none() {
                if let Some(weid) = c.word {
                    if self.st.is_assoc_str(c.net) {
                        return Offsets::AssocStrKey(self.assoc_str_key_of(weid));
                    }
                    if self.st.is_assoc(c.net) {
                        return Offsets::AssocKey(self.assoc_key_of(weid));
                    }
                }
            }
        }
        let ev = |eid: u32| {
            let v = self.eval(eid);
            // Resolve a runtime select index to a bit-position offset. The valid
            // domain is the SIGNED i32 range: a small negative (`a[-1+:4]`) is a
            // legitimate underflow that partial-writes the in-range bits (P0-IPU),
            // and a small/large positive writes (or lands OOB-high and drops). An
            // index OUTSIDE that domain is dropped entirely (iverilog parity):
            //   - X/Z          → the bit position is UNKNOWN
            //   - huge positive / UNSIGNED > i31 / clean beyond-u32 / > 64-bit
            //                  → out of any net's range
            // `OOR_DROP` (2^30) sits far above any net width (≤2^20) so every
            // selected bit lands out of range for bit/part/indexed-part and
            // array-word chunks alike. Signed-aware: an unsigned 0xFFFFFFFF is the
            // huge 4294967295 (drop), NOT a wrapped −1 (which would partial-write).
            const OOR_DROP: u32 = 1 << 30;
            if v.has_xz() {
                return OOR_DROP;
            }
            match v.to_i128_signed() {
                Some(i) if (i32::MIN as i128..=i32::MAX as i128).contains(&i) => i as i32 as u32,
                _ => OOR_DROP,
            }
        };
        let pair = |c: &sim_ir::LvalChunk| {
            let off = c.offset.map(ev).unwrap_or(0);
            // `word` is an ExprId array index (`mem[k] = …`); resolve NOW.
            let word = c.word.map(ev).unwrap_or(0);
            (off, word)
        };
        // Inline the ≤2-chunk case (virtually all lvalues) — no allocation.
        if lhs.chunks.len() <= 2 {
            let mut buf = [(0u32, 0u32); 2];
            for (i, c) in lhs.chunks.iter().enumerate() {
                buf[i] = pair(c);
            }
            Offsets::Inline {
                buf,
                len: lhs.chunks.len() as u8,
            }
        } else {
            Offsets::Heap(lhs.chunks.iter().map(pair).collect())
        }
    }

    /// Evaluate a `Terminator::Delay` amount (format_version 4: an ExprId of
    /// the raw delay value in module units) into global-precision ticks:
    /// real → round(v × M) clamped at 0; any X/Z → 0 (iverilog parity);
    /// integral → v × M, u64-saturating. `M` = the CURRENT process's
    /// timescale multiplier (`cur_time_mult`, set per activation) — exactly
    /// the scaling the old const-fold path applied at elaborate time.
    pub(crate) fn delay_ticks(&self, eid: u32) -> u64 {
        let v = self.eval(eid);
        let mult = self.st.cur_time_mult.max(1);
        if v.is_real {
            let x = v.to_f64().unwrap_or(0.0) * mult as f64;
            if x <= 0.0 {
                return 0;
            }
            return x.round() as u64;
        }
        if v.has_xz() {
            return 0;
        }
        v.to_u64().unwrap_or(u64::MAX).saturating_mul(mult)
    }

    /// Run a pre-compiled native expression program against the net table (VM-only
    /// fast path). `self.st` is the same `NetReader` `eval_ctx_top` builds its EvalCtx
    /// over, so a native leaf load reads exactly what the interpreter would.
    pub(crate) fn eval_native(&self, prog: &crate::native_eval::NativeProg) -> Value {
        crate::native_eval::run(prog, self.st)
    }

    /// Build an EvalCtx and run eval_ctx (mirror of the `eval` façade).
    pub(crate) fn eval_ctx_top(&self, eid: u32, ctx_width: u32, ctx_signed: bool) -> Value {
        let ctx = EvalCtx {
            ir: self.st.ir,
            nets: self.st,
            now: self.st.now,
            wt: &self.st.wt,
            time_mult: self.st.cur_time_mult,
            rng: &self.st.rng,
            plusargs: &self.st.plusargs,
        };
        ctx.eval_ctx(eid, ctx_width, ctx_signed)
    }

    /// v5 ⑤: READ-phase assoc-key resolution (the scheduler-side mirror of
    /// `EvalCtx::assoc_key` — one recipe, two entry points: lvalue offsets
    /// here, rvalue reads in the eval arm).
    pub(crate) fn assoc_key_of(&self, eid: u32) -> Option<i64> {
        let ctx = EvalCtx {
            ir: self.st.ir,
            nets: self.st,
            now: self.st.now,
            wt: &self.st.wt,
            time_mult: self.st.cur_time_mult,
            rng: &self.st.rng,
            plusargs: &self.st.plusargs,
        };
        ctx.assoc_key(eid)
    }

    /// v6: the byte-string twin of `assoc_key_of` (string-keyed assoc).
    pub(crate) fn assoc_str_key_of(&self, eid: u32) -> Option<Vec<u8>> {
        let ctx = EvalCtx {
            ir: self.st.ir,
            nets: self.st,
            now: self.st.now,
            wt: &self.st.wt,
            time_mult: self.st.cur_time_mult,
            rng: &self.st.rng,
            plusargs: &self.st.plusargs,
        };
        ctx.assoc_str_key(eid)
    }

    /// v6: one assoc-iteration step (`first`/`next`/`last`/`prev`) — the
    /// WRITE-phase body behind `k_assoc_iter`. Returns the int status
    /// (1 found / 0 none / −1 found-but-truncated, hand-IEEE §7.9.4). On a
    /// hit the ref key variable is written through the NORMAL lvalue funnel
    /// (so `@(k)` sensitivity and VCD see it like any blocking assign). On
    /// dyn/queue handles the walk is the DENSE 0..size-1 order (the internal
    /// `foreach` desugar target).
    fn assoc_iter_step(&mut self, rhs: u32) -> i32 {
        use std::ops::Bound;
        let Some(sim_ir::Expr::SysFunc { which, args }) = self.st.ir.exprs.get(rhs as usize) else {
            return 0; // defensive: hand-built IR only (the rhs probe matched)
        };
        let which = *which;
        let net_of = |a: Option<&u32>| {
            a.and_then(|&e| match self.st.ir.exprs.get(e as usize) {
                Some(sim_ir::Expr::Signal { net, word: None }) => Some(*net),
                _ => None,
            })
        };
        let (Some(hnet), Some(knet)) = (net_of(args.first()), net_of(args.get(1))) else {
            return 0; // malformed args: degrade, never panic
        };
        let (kw, ks) = self
            .st
            .ir
            .nets
            .get(knet as usize)
            .map(|nv| (nv.width.max(1), nv.signed))
            .unwrap_or((32, true));
        use sim_ir::SysFuncId as F;
        let needs_cur = matches!(which, F::AssocNext | F::AssocPrev);
        let cur_val = if needs_cur {
            let v = self.st.read_net(knet, None);
            if v.has_xz() {
                self.st
                    .dyn_warn_once_at(hnet, "assoc iteration key variable is X/Z (status 0)");
                return 0;
            }
            Some(v)
        } else {
            None
        };

        // The located key, in whichever key domain the handle uses.
        enum Hit {
            Int(i64),
            Str(Vec<u8>),
        }
        let hit: Option<Hit> = match self.st.dyn_heap.get(hnet as usize).and_then(|o| o.as_ref()) {
            Some(crate::state::DynObj::Assoc { map }) => {
                let cur = cur_val.as_ref().map(|v| {
                    // Current key from the var's OWN width/signedness (the
                    // same extension `assoc_key` applies to an expr).
                    let raw = v.val.first().copied().unwrap_or(0);
                    if kw >= 64 {
                        raw as i64
                    } else {
                        let m = (1u64 << kw) - 1;
                        let r = raw & m;
                        if ks && (r >> (kw - 1)) & 1 == 1 {
                            (r | !m) as i64
                        } else {
                            r as i64
                        }
                    }
                });
                match which {
                    F::AssocFirst => map.keys().next().copied(),
                    F::AssocLast => map.keys().next_back().copied(),
                    F::AssocNext => map
                        .range((Bound::Excluded(cur.unwrap_or(0)), Bound::Unbounded))
                        .next()
                        .map(|(k, _)| *k),
                    _ => map
                        .range((Bound::Unbounded, Bound::Excluded(cur.unwrap_or(0))))
                        .next_back()
                        .map(|(k, _)| *k),
                }
                .map(Hit::Int)
            }
            Some(crate::state::DynObj::AssocStr { map }) => {
                let cur = cur_val.as_ref().map(crate::eval::value_str_bytes);
                match which {
                    F::AssocFirst => map.keys().next().cloned(),
                    F::AssocLast => map.keys().next_back().cloned(),
                    F::AssocNext => map
                        .range((
                            Bound::Excluded(cur.clone().unwrap_or_default()),
                            Bound::Unbounded,
                        ))
                        .next()
                        .map(|(k, _)| k.clone()),
                    _ => map
                        .range::<Vec<u8>, _>((
                            Bound::Unbounded,
                            Bound::Excluded(&cur.unwrap_or_default()),
                        ))
                        .next_back()
                        .map(|(k, _)| k.clone()),
                }
                .map(Hit::Str)
            }
            // Dense walk on dyn/queue (a missing entry IS the empty object).
            other => {
                let len = other.map(|o| o.len() as u64).unwrap_or(0);
                let cur = cur_val.as_ref().and_then(|v| v.to_u64());
                let dense = match which {
                    F::AssocFirst => (len > 0).then_some(0),
                    F::AssocLast => len.checked_sub(1),
                    F::AssocNext => cur.and_then(|c| c.checked_add(1)).filter(|&n| n < len),
                    _ => cur.and_then(|c| c.checked_sub(1)).filter(|&p| p < len),
                };
                dense.map(|d| Hit::Int(d as i64))
            }
        };

        let Some(hit) = hit else {
            return 0; // none/empty/exhausted: key var UNCHANGED (§7.9.4)
        };

        // Build the key-var value + the "fits the ref var" verdict.
        let (kval, fits) = match hit {
            Hit::Int(k) => {
                let fits = if kw >= 64 {
                    true // i64 domain always round-trips through ≥64 bits
                } else if ks {
                    let m = (1u64 << kw) - 1;
                    let t = (k as u64) & m;
                    let back = if (t >> (kw - 1)) & 1 == 1 {
                        (t | !m) as i64
                    } else {
                        t as i64
                    };
                    back == k
                } else {
                    k >= 0 && (k as u64) >> kw.min(63) == 0
                };
                let mut v = Value::zeros(kw, ks);
                let sign_fill = if k < 0 { u64::MAX } else { 0 };
                for (i, w) in v.val.iter_mut().enumerate() {
                    *w = if i == 0 { k as u64 } else { sign_fill };
                }
                v.mask_top();
                (v, fits)
            }
            Hit::Str(bytes) => {
                let fits = (bytes.len() as u64) * 8 <= kw as u64;
                let mut v = Value::zeros(kw, ks);
                for (i, b) in bytes.iter().rev().enumerate() {
                    for bit in 0..8u32 {
                        let idx = i as u32 * 8 + bit;
                        if idx < kw {
                            v.set_vu(idx, ((b >> bit) & 1) as u64, 0);
                        }
                    }
                }
                (v, fits)
            }
        };
        if !fits {
            self.st.dyn_warn_once_at(
                hnet,
                "assoc iteration key does not fit the index variable (truncated, status -1)",
            );
        }
        // Whole-var write through the normal funnel (dirty channel included).
        let klv = sim_ir::Lvalue {
            chunks: vec![sim_ir::LvalChunk {
                net: knet,
                word: None,
                offset: None,
                width: None,
                kind: sim_ir::SelKind::Bit,
            }],
        };
        self.st.write_lvalue(
            &klv,
            kval,
            &Offsets::Inline {
                buf: [(0, 0); 2],
                len: 1,
            },
        );
        if fits {
            1
        } else {
            -1
        }
    }

    pub(crate) fn now(&self) -> u64 {
        self.st.now
    }

    pub(crate) fn max_deltas_guard(&self) -> u64 {
        self.max_deltas
    }

    pub(crate) fn mark_fatal(&mut self) {
        // Only the in-body delta guards (interpreter + VM `k_mark_fatal`) call
        // this — same condition class, same single-shot diagnostic.
        self.fatal_delta_limit();
    }

    pub(crate) fn schedule_resume(&mut self, proc: u32, block: u32, tick: u64, inactive: bool) {
        // `proc` is an activity id; read its deterministic tie (NOT the id) so two
        // sibling children land in distinct-tie wheel slots in declaration order.
        let tie = self.activities[proc as usize].tie;
        let ready = Ready { tie, proc, block };
        if tick == self.st.now {
            if inactive {
                push_sorted(&mut self.cur.inactive, ready);
            } else {
                push_sorted(&mut self.cur.active, ready);
            }
        } else {
            let region = if inactive {
                RegionTag::Inactive
            } else {
                RegionTag::Active
            };
            // Reuse a pooled bucket on first insert at a new tick (or_default
            // would allocate a fresh Vec per distinct simulation time).
            let pool = &mut self.bucket_pool;
            self.wheel
                .entry(tick)
                .or_insert_with(|| pool.pop().unwrap_or_default())
                .push((region, ready));
        }
    }

    /// Suspend a process on an in-body `@(...)`/`wait(...)`. (The `wait(expr)`
    /// already-true short-circuit is handled in the executor.)
    ///
    /// An in-body `Wait{Edge}` is registered ONLY as a `waiter`, NOT in
    /// `net_to_edge`. `waiters` entries are consumed when they fire
    /// (`propagate_changes` `retain`s them out); `net_to_edge` entries are
    /// permanent. Registering in both would leave an orphan `net_to_edge` entry
    /// that re-fires the process on every future edge of that net for the rest of
    /// the sim (resuming a block it already passed) — an unbounded leak and a
    /// correctness bug for any process that loops back through the same wait. The
    /// `waiters` Edge arm in `propagate_changes` performs the exact same edge test.
    pub(crate) fn suspend_on(&mut self, proc: u32, block: u32, cause: WaitCause) {
        // `proc` is an activity id; carry its distinct tie so two siblings waiting
        // on the same event are distinguishable (neither lost nor double-counted).
        let tie = self.activities[proc as usize].tie;
        let ready = Ready { tie, proc, block };
        // Snapshot the watched nets so an in-body `@(sig)` fires on the next change
        // AFTER this point, not on one already applied this delta before it armed.
        let arm = match &cause {
            WaitCause::Level { nets } => Some(
                nets.iter()
                    .map(|&n| self.st.nets[n as usize].cur.clone())
                    .collect(),
            ),
            _ => None,
        };
        // WAITER-POOL p2: keep the running counts exact at this push site.
        match &cause {
            WaitCause::Expr { .. } => self.n_expr_waiters += 1,
            WaitCause::Level { .. } => self.n_level_waiters += 1,
            _ => {}
        }
        self.waiters.push(Waiter { cause, ready, arm });
    }

    /// v5 increment (A): a transport NBA — index + value sampled NOW, update
    /// due in the NBA region of `now + ticks` (saturating).
    pub(crate) fn schedule_nba_at(&mut self, lhs: Lvalue, sampled: Value, ticks: u64) {
        let offsets = self.resolve_lvalue_offsets(&lhs);
        let seq = self.nba_seq;
        self.nba_seq += 1;
        let at = self.st.now.saturating_add(ticks);
        self.delayed_nba.entry(at).or_default().push(NbaUpdate {
            seq,
            lhs,
            sampled,
            offsets,
        });
    }

    pub(crate) fn schedule_nba(&mut self, lhs: Lvalue, sampled: Value) {
        // Sample the dynamic LHS index NOW (Active region), BEFORE the mutable
        // push, so a later same-step write to the index net cannot move the target.
        let offsets = self.resolve_lvalue_offsets(&lhs);
        let seq = self.nba_seq;
        self.nba_seq += 1;
        self.nba.push(NbaUpdate {
            seq,
            lhs,
            sampled,
            offsets,
        });
    }

    /// Re-arm a process after it Returns. The Edge/Level asymmetry is
    /// LOAD-BEARING (verified):
    /// - `SensKind::Edge` registers a *permanent* entry in `net_to_edge` that is
    ///   read-not-consumed on fire (`propagate_changes` clones, never removes).
    ///   Re-pushing it on every Return would duplicate the registration → the
    ///   process fires 2^k times on edge k (wrong results for blocking edge
    ///   bodies + a 2^k termination hazard). So Edge MUST NOT re-arm.
    /// - `SensKind::Level` waiters ARE consumed on fire (`waiters.retain` returns
    ///   `false`), so Level MUST re-arm or it would never wake again — and the
    ///   `infinite_delta_guard_trips` test depends on this re-registration.
    /// - `Initial` is one-shot (dead after its single run).
    pub(crate) fn rearm(&mut self, proc: u32) {
        // Fork children NEVER re-arm: a child's reaching its join is a one-shot
        // completion, routed by the run_process loop-top intercept to
        // on_child_complete. (A child has no static sensitivity of its own anyway.)
        if self.activities[proc as usize].is_child {
            return;
        }
        let tmpl = self.activities[proc as usize].template as usize;
        let kind = self.st.ir.processes[tmpl].sensitivity.kind;
        match kind {
            // permanent net_to_edge entry / one-shot: do NOT re-register.
            SensKind::Edge | SensKind::Initial => {}
            // consumed waiter: must re-register to wake on the next change.
            SensKind::Comb | SensKind::Latch | SensKind::Level => self.arm_sensitivity(proc),
        }
    }

    // ── fork/join support (activity arena + join barriers) ────────────────

    /// Recover a fork's join mode from the elaborate side table. TOTAL-OR-FATAL:
    /// a miss is impossible after the `arm_processes` gate, but we never default to
    /// a blocking join — a fabricated `All` would silently turn a lost-side-channel
    /// `join_none`/`join_any` into a deadlock with no diagnostic. Panic instead.
    pub(crate) fn fork_mode(&self, template: u32, join_bb: u32) -> Option<JoinMode> {
        self.fork_modes.get(&(template, join_bb)).copied()
    }

    /// P1-7: a `Terminator::Fork` with no ForkModeTable entry means the `.velab`
    /// fork-mode trailer was lost/stale (it rides OUTSIDE the schema gate, so a
    /// hand-truncated artifact can reach here). Fabricating a mode would silently
    /// turn a `join_none` into a deadlock — emit a FATAL artifact diagnostic and
    /// end the run instead (was: panic!).
    fn fatal_fork_mode_missing(&mut self, template: u32, join_bb: u32) {
        use diag::{Diagnostic, LogEvent, MsgCode, Severity, TimeStamp};
        self.st.sink.emit(LogEvent::Diagnostic(Diagnostic {
            severity: Severity::Fatal,
            code: MsgCode::ArtFormatMismatch,
            message: format!(
                "fork join-mode sidecar entry missing for (template={template}, \
                 join_bb={join_bb}) — .velab trailer lost or stale; re-run velab"
            ),
            location: None,
            context: Vec::new(),
            sim_time: Some(TimeStamp { ticks: self.st.now }),
        }));
        self.st.had_fatal = true;
        self.st.finished = true;
    }

    /// Accessors the executor's loop-top intercept + body fetch use (the
    /// `activities`/`barriers` fields are private to sched.rs).
    pub(crate) fn activity_template(&self, aid: u32) -> u32 {
        self.activities[aid as usize].template
    }
    pub(crate) fn activity_is_child(&self, aid: u32) -> bool {
        self.activities[aid as usize].is_child
    }
    /// The barrier's completion-sentinel join_bb (for the loop-top intercept).
    pub(crate) fn barrier_join_bb(&self, jr: u32) -> u32 {
        self.barriers[jr as usize].join_bb
    }
    pub(crate) fn activity_join_ref(&self, aid: u32) -> Option<u32> {
        self.activities[aid as usize].join_ref
    }

    /// Debug-only: assert no LIVE barrier (same template) has its join_bb equal to
    /// `bb` for a non-child activity. A non-child fetching a live join_bb would mean
    /// the parent walked the never-executed sentinel — a builder/engine bug.
    #[cfg(debug_assertions)]
    pub(crate) fn assert_not_parent_at_join(&self, aid: u32, bb: u32) {
        if self.activities[aid as usize].is_child {
            return;
        }
        let tmpl = self.activities[aid as usize].template;
        let bad = self.barriers.iter().any(|b| {
            !b.fired && b.join_bb == bb && self.activities[b.parent as usize].template == tmpl
        });
        debug_assert!(
            !bad,
            "parent/top-level activity {aid} fetched a live barrier join_bb sentinel {bb}"
        );
    }

    /// Execute a `Terminator::Fork`: register the barrier, spawn each child as a new
    /// activity (sharing the parent's template, entering at its own child-entry BB),
    /// and either suspend the parent (All/Any with ≥1 child) or fall through to
    /// `resume_bb` (None, or zero children). Returns `Some(resume_bb)` when the
    /// parent continues THIS activation (the executor sets `bb = resume_bb`), or
    /// `None` when the parent suspends on the barrier.
    pub(crate) fn exec_fork(
        &mut self,
        parent_aid: u32,
        children: &[u32],
        join: u32,
        resume_bb: u32,
    ) -> Option<u32> {
        let parent_tmpl = self.activities[parent_aid as usize].template;
        // P1-7: missing sidecar entry → graceful FATAL stop (never a default mode).
        let Some(mode) = self.fork_mode(parent_tmpl, join) else {
            self.fatal_fork_mode_missing(parent_tmpl, join);
            return None; // parent parks; run loop sees `finished` and ends the run
        };

        // Register the barrier, recycling a drained slot when available (P3-1).
        let barrier = JoinBarrier {
            parent: parent_aid,
            join_bb: join,
            resume_bb,
            mode,
            outstanding: children.len() as u32,
            fired: false,
        };
        let join_ref = match self.free_barriers.pop() {
            Some(id) => {
                self.barriers[id as usize] = barrier;
                id
            }
            None => {
                self.barriers.push(barrier);
                (self.barriers.len() - 1) as u32
            }
        };

        // Spawn each child as a NEW activity. Deterministic: declaration order ==
        // the order of `children`; each child's tie composes parent.tie + child idx.
        // NOTE: nested fork is an elaborate ERROR, so `parent_tmpl` here is always a
        // TOP-LEVEL process and `parent.tie` its dense top-level declaration index.
        let parent_tie = self.activities[parent_aid as usize].tie;
        // FORK-TIE-CAP: `compose_child_tie` packs `(parent_tie + 1)` into the high
        // 16 bits and the child index into the low 16. Past ~65535 top-level
        // processes the high half overflows the u32 (`(parent_tie+1) << 16`); past
        // ~65536 arms the low half aliases (`child_idx & 0xFFFF`). Either silently
        // collides sibling ties → nondeterministic ordering. Reject loud (graceful
        // fatal) — both limits are far above any real testbench.
        if parent_tie >= 0xFFFF || children.len() > 0x1_0000 {
            self.st.fatal_run(&format!(
                "fork exceeds the v1 tie-encoding limit (parent process tie {parent_tie}, \
                 {} arms); deterministic ordering supports \u{2264} 65534 top-level \
                 processes and \u{2264} 65536 arms per fork",
                children.len()
            ));
            return None; // parent parks; run loop sees `finished` and ends the run
        }
        for (child_idx, &child_entry) in children.iter().enumerate() {
            let child_tie = compose_child_tie(parent_tie, child_idx as u32);
            let child = Activity {
                template: parent_tmpl,
                tie: child_tie,
                join_ref: Some(join_ref),
                is_child: true,
                reported: false,
                dead: false,
                wait_fork: None,
                busy: false,
                gen: 0,
            };
            // Recycle a completed child slot when available (P3-1): a freed slot's
            // old activity has reported (it cannot be queued/waiting anywhere).
            // BUMP the generation on reuse so a §16.4 deferred report still pending
            // under the OLD incarnation's `(marker, aid, gen)` key is not flushed
            // by this fresh incarnation reaching the same marker.
            let child_aid = match self.free_activities.pop() {
                Some(id) => {
                    let next_gen = self.activities[id as usize].gen.wrapping_add(1);
                    self.activities[id as usize] = Activity {
                        gen: next_gen,
                        ..child
                    };
                    id
                }
                None => {
                    self.activities.push(child);
                    (self.activities.len() - 1) as u32
                }
            };
            // Make the child runnable NOW (same instant, Active region); push_sorted
            // by the composed tie keeps siblings in declaration order.
            push_sorted(
                &mut self.cur.active,
                Ready {
                    tie: child_tie,
                    proc: child_aid,
                    block: child_entry,
                },
            );
        }

        match mode {
            JoinMode::None => {
                // join_none: parent does NOT block. Mark fired (never resumes via the
                // barrier) and continue at resume_bb THIS instant; children run on as
                // background activities concurrently with the continuation.
                self.barriers[join_ref as usize].fired = true;
                Some(resume_bb)
            }
            JoinMode::All | JoinMode::Any => {
                if children.is_empty() {
                    // fork join / fork join_any with zero children: resume now.
                    self.barriers[join_ref as usize].fired = true;
                    Some(resume_bb)
                } else {
                    // Parent blocks on the join. It holds NO cur/wheel/waiter entry —
                    // it is parked solely by the barrier and re-enqueued by
                    // on_child_complete when the join condition fires.
                    None
                }
            }
        }
    }

    /// Execute `wait fork;` (IEEE §9.6.1). Counts this process's outstanding
    /// immediate children — every live (non-dead, non-reported) child activity
    /// whose join barrier names this parent (covers the CUMULATIVE set across
    /// all prior `fork ... join_none` and surplus `join_any` children). Returns
    /// `true` if the parent may continue THIS activation (zero outstanding), or
    /// `false` after parking it (`on_child_complete` re-enqueues at `resume_bb`).
    pub(crate) fn exec_wait_fork(&mut self, parent_aid: u32, resume_bb: u32) -> bool {
        let barriers = &self.barriers;
        let outstanding = self
            .activities
            .iter()
            .filter(|c| {
                c.is_child
                    && !c.dead
                    && !c.reported
                    && c.join_ref
                        .is_some_and(|jr| barriers[jr as usize].parent == parent_aid)
            })
            .count() as u32;
        if outstanding == 0 {
            return true; // no live children → fall through immediately
        }
        self.activities[parent_aid as usize].wait_fork = Some(WaitForkPark {
            resume_bb,
            outstanding,
        });
        false // parked; on_child_complete resumes the parent at resume_bb
    }

    /// A fork child has reached its barrier's join_bb. Decrement and, on the firing
    /// condition for the mode, re-enqueue the parent at `resume_bb` exactly once.
    pub(crate) fn on_child_complete(&mut self, join_ref: u32, child_aid: u32) {
        // Per-child fire-once: a child may reach its join at most once. A second
        // report would under-decrement `outstanding` and fire an All-barrier EARLY.
        debug_assert!(
            !self.activities[child_aid as usize].reported,
            "internal error: child {child_aid} reported completion twice"
        );
        self.activities[child_aid as usize].reported = true;
        // P3-1: the reporting child is DEAD past this point (its run_process
        // returns Step::Done right after; children never re-arm) — recycle.
        self.free_activities.push(child_aid);

        // v8 `wait fork`: capture the forking parent BEFORE the barrier may be
        // recycled below — used by the wait-fork hook at the end.
        let parent_aid = self.barriers[join_ref as usize].parent;

        let b = &mut self.barriers[join_ref as usize];
        debug_assert!(
            b.outstanding > 0,
            "internal error: barrier {join_ref} outstanding underflow"
        );
        b.outstanding -= 1;
        if b.outstanding == 0 {
            // Every child has reported: nothing references this barrier anymore
            // (the parent resume below reads its fields by value) — recycle.
            self.free_barriers.push(join_ref);
        }
        let fire = match b.mode {
            JoinMode::All => b.outstanding == 0, // last child
            JoinMode::Any => true,               // first child (later guarded by `fired`)
            JoinMode::None => false,             // never (parent already continued)
        };
        if fire && !b.fired {
            b.fired = true;
            let parent = b.parent;
            let resume_bb = b.resume_bb;
            let tie = self.activities[parent as usize].tie;
            // Re-enqueue the parent at resume_bb THIS instant (Active region).
            // Surplus children (join_any) stay live and run to completion; their
            // later on_child_complete sees `fired == true` → no-op.
            push_sorted(
                &mut self.cur.active,
                Ready {
                    tie,
                    proc: parent,
                    block: resume_bb,
                },
            );
        }

        // v8 `wait fork`: this completion also counts against the parent's
        // parked wait-fork set (a join_none/join_any-surplus child reports to
        // its OWN barrier above, but its parent may be blocked on `wait fork`).
        // Decrement and resume the parent once its last child reports.
        let wf_resume = {
            let pa = &mut self.activities[parent_aid as usize];
            if let Some(wf) = pa.wait_fork.as_mut() {
                wf.outstanding = wf.outstanding.saturating_sub(1);
                if wf.outstanding == 0 {
                    Some(wf.resume_bb)
                } else {
                    None
                }
            } else {
                None
            }
        };
        if let Some(resume_bb) = wf_resume {
            self.activities[parent_aid as usize].wait_fork = None;
            let tie = self.activities[parent_aid as usize].tie;
            push_sorted(
                &mut self.cur.active,
                Ready {
                    tie,
                    proc: parent_aid,
                    block: resume_bb,
                },
            );
        }
    }
}

/// [P7b] `Scheduler` is the interpreter's implementation of the body↔kernel ABI:
/// each method forwards to the inherent method of the same purpose (the `k_*` prefix
/// keeps the trait surface distinct from the inherent one, so there is no shadowing).
/// The statement executor (`exec::compute_effect`/`apply_effect`) drives the
/// interpreter through exactly this surface, so the existing suite already exercises
/// the seam byte-identically — and a Stage-C compiled body will call the same methods.
impl Kernel for Scheduler<'_, '_> {
    fn k_eval_for_lvalue(&self, lhs: &Lvalue, rhs: u32) -> Value {
        self.eval_for_lvalue(lhs, rhs)
    }
    fn k_eval_native(&self, prog: &crate::native_eval::NativeProg) -> Value {
        self.eval_native(prog)
    }
    fn k_resolve_lvalue_offsets(&self, lhs: &Lvalue) -> Offsets {
        self.resolve_lvalue_offsets(lhs)
    }
    fn k_force(&mut self, lhs: &Lvalue, value: Value, rhs: u32, sid: u32) {
        // The multiplier is snapshot at registration so a `$time` in the RHS
        // keeps rendering with the right scale on later re-evals (C7 lesson).
        let net = lhs.chunks[0].net;
        let mult = self.st.cur_time_mult;
        let weak = self.st.assign_ranks.contains(&sid);
        if weak {
            // §9.3.1 proc-assign: an active FORCE keeps priority — park the
            // assign as latent (it takes control at release). Otherwise (re)pin
            // at assign rank (a second assign overrides the first).
            if matches!(self.st.active_forces.get(&net), Some((.., false))) {
                self.st.latent_assigns.insert(net, (lhs.clone(), rhs, mult));
                return;
            }
            self.st.latent_assigns.remove(&net);
        } else if let Some((plv, prhs, pmult, true)) = self.st.active_forces.get(&net).cloned() {
            // real force displacing an active assign: park it for release.
            self.st.latent_assigns.insert(net, (plv, prhs, pmult));
        }
        self.st.force_write(lhs, value);
        // Register for continuous re-evaluation (IEEE §9.3.2 / §9.3.1).
        self.st
            .active_forces
            .insert(net, (lhs.clone(), rhs, mult, weak));
        // C-FORCE-REEVAL-p2: refresh this force's net→forces sensitivity (or
        // mark it always-reeval if volatile / zero-net) so the per-delta reeval
        // can skip forces whose inputs are unchanged.
        self.st.register_force_sensitivity(net, rhs);
    }
    fn k_release(&mut self, lhs: &Lvalue, sid: u32) {
        let net = lhs.chunks[0].net;
        if self.st.assign_ranks.contains(&sid) {
            // `deassign`: drop the assign wherever it lives. An active STRONG
            // force is untouched; an active assign unpins (the variable HOLDS
            // its value, §9.3.1); a latent assign is just forgotten.
            self.st.latent_assigns.remove(&net);
            if matches!(self.st.active_forces.get(&net), Some((.., true))) {
                self.st.active_forces.remove(&net);
                self.st.unregister_force_sensitivity(net);
                self.st.release(lhs);
            }
            return;
        }
        // `release`: removes the FORCE. A parked proc-assign resumes control
        // (re-pin + re-evaluate NOW, §9.3.1); an active assign is NOT a force
        // and keeps control; otherwise plain unpin.
        match self.st.active_forces.get(&net) {
            Some((.., true)) => {} // assign active, no force: release is a no-op
            _ => {
                self.st.active_forces.remove(&net);
                self.st.unregister_force_sensitivity(net);
                self.st.release(lhs);
                if let Some((alv, arhs, amult)) = self.st.latent_assigns.remove(&net) {
                    let saved = self.st.cur_time_mult;
                    self.st.cur_time_mult = amult;
                    let v = self.eval_for_lvalue(&alv, arhs);
                    self.st.force_write(&alv, v);
                    self.st.cur_time_mult = saved;
                    self.st.active_forces.insert(net, (alv, arhs, amult, true));
                    // Re-register the resumed latent assign's sensitivity.
                    self.st.register_force_sensitivity(net, arhs);
                }
            }
        }
    }
    fn k_write_lvalue(&mut self, lhs: &Lvalue, value: Value, offsets: &Offsets) {
        self.st.write_lvalue(lhs, value, offsets);
    }
    fn k_schedule_nba(&mut self, lhs: Lvalue, value: Value) {
        self.schedule_nba(lhs, value);
    }
    fn k_delay_ticks(&self, eid: u32) -> u64 {
        self.delay_ticks(eid)
    }
    fn k_schedule_nba_at(&mut self, lhs: Lvalue, value: Value, ticks: u64) {
        self.schedule_nba_at(lhs, value, ticks);
    }
    fn k_dispatch_systask(
        &mut self,
        which: sim_ir::SysTaskId,
        fmt: Option<u32>,
        args: &[u32],
        sid: u32,
    ) -> crate::builtins::Ctl {
        crate::builtins::dispatch(self, which, fmt, args, sid)
    }
    fn k_queue_pop_rhs(&self, rhs: u32) -> bool {
        matches!(
            self.st.ir.exprs.get(rhs as usize),
            Some(sim_ir::Expr::SysFunc {
                which: sim_ir::SysFuncId::QPopBack | sim_ir::SysFuncId::QPopFront,
                ..
            })
        )
    }
    fn k_random_seeded_rhs(&self, rhs: u32) -> bool {
        matches!(
            self.st.ir.exprs.get(rhs as usize),
            Some(sim_ir::Expr::SysFunc {
                which: sim_ir::SysFuncId::Random,
                args,
            }) if !args.is_empty()
        )
    }
    fn k_random_seeded(&mut self, rhs: u32) -> Value {
        // shape guaranteed by `k_random_seeded_rhs` + elaborate's whole-net
        // seed contract; everything below defends a hand-built IR.
        let seed_net = match self.st.ir.exprs.get(rhs as usize) {
            Some(sim_ir::Expr::SysFunc { args, .. }) => {
                args.first()
                    .and_then(|&a| match self.st.ir.exprs.get(a as usize) {
                        Some(sim_ir::Expr::Signal { net, word: None }) => Some(*net),
                        _ => None,
                    })
            }
            _ => None,
        };
        let Some(net) = seed_net else {
            return Value::xs(32, true);
        };
        // seed in: low 32 bits of the variable; X/Z reads as 0 (then the
        // Annex zero-substitution applies, like an uninitialized iverilog reg).
        let cur = self.st.read_net(net, None);
        let mut s = if cur.has_xz() {
            0
        } else {
            (cur.to_u64().unwrap_or(0) & 0xffff_ffff) as u32
        };
        let r = crate::rng::annex_n_random(&mut s);
        // write the updated seed back through the normal lvalue funnel
        // (resizes to the variable's width like any blocking assign).
        let lv = Lvalue {
            chunks: vec![sim_ir::LvalChunk {
                net,
                word: None,
                offset: None,
                width: None,
                kind: sim_ir::SelKind::Bit,
            }],
        };
        let sv = Value::from_i128(s as i32 as i128, 32, true);
        let off = self.resolve_lvalue_offsets(&lv);
        self.k_write_lvalue(&lv, sv, &off);
        Value::from_i128(r as i128, 32, true)
    }
    fn k_dist_seeded_rhs(&self, rhs: u32) -> bool {
        matches!(
            self.st.ir.exprs.get(rhs as usize),
            Some(sim_ir::Expr::SysFunc { which, args })
                if matches!(
                    which,
                    sim_ir::SysFuncId::DistUniform
                        | sim_ir::SysFuncId::DistNormal
                        | sim_ir::SysFuncId::DistExponential
                        | sim_ir::SysFuncId::DistPoisson
                        | sim_ir::SysFuncId::DistChiSquare
                        | sim_ir::SysFuncId::DistT
                        | sim_ir::SysFuncId::DistErlang
                ) && !args.is_empty()
        )
    }
    fn k_dist_seeded(&mut self, rhs: u32) -> Value {
        // shape guaranteed by `k_dist_seeded_rhs` + elaborate's whole-net seed
        // contract; everything below defends a hand-built IR.
        let (which, seed_net, params) = match self.st.ir.exprs.get(rhs as usize) {
            Some(sim_ir::Expr::SysFunc { which, args }) if !args.is_empty() => {
                let net = match self.st.ir.exprs.get(args[0] as usize) {
                    Some(sim_ir::Expr::Signal { net, word: None }) => Some(*net),
                    _ => None,
                };
                (*which, net, args[1..].to_vec())
            }
            _ => return Value::xs(32, true),
        };
        let Some(net) = seed_net else {
            return Value::xs(32, true);
        };
        // dist params are `integer` (signed 32-bit); X/Z reads as 0.
        let p: Vec<i32> = params
            .iter()
            .map(|&a| self.eval(a).to_u64().unwrap_or(0) as u32 as i32)
            .collect();
        // seed in: low 32 bits; X/Z → 0 (uninitialized-reg parity). The dist
        // kernels advance it via the Annex `69069*s+1` integer LCG.
        let cur = self.st.read_net(net, None);
        let mut s = if cur.has_xz() {
            0
        } else {
            (cur.to_u64().unwrap_or(0) & 0xffff_ffff) as u32
        };
        let r = match which {
            sim_ir::SysFuncId::DistUniform => {
                crate::rng::dist_uniform(&mut s, *p.first().unwrap_or(&0), *p.get(1).unwrap_or(&0))
            }
            sim_ir::SysFuncId::DistNormal => {
                crate::rng::dist_normal(&mut s, *p.first().unwrap_or(&0), *p.get(1).unwrap_or(&0))
            }
            sim_ir::SysFuncId::DistExponential => {
                crate::rng::dist_exponential(&mut s, *p.first().unwrap_or(&0))
            }
            sim_ir::SysFuncId::DistPoisson => {
                crate::rng::dist_poisson(&mut s, *p.first().unwrap_or(&0))
            }
            sim_ir::SysFuncId::DistChiSquare => {
                crate::rng::dist_chi_square(&mut s, *p.first().unwrap_or(&0))
            }
            sim_ir::SysFuncId::DistT => crate::rng::dist_t(&mut s, *p.first().unwrap_or(&0)),
            sim_ir::SysFuncId::DistErlang => {
                crate::rng::dist_erlang(&mut s, *p.first().unwrap_or(&0), *p.get(1).unwrap_or(&0))
            }
            _ => 0,
        };
        let lv = Lvalue {
            chunks: vec![sim_ir::LvalChunk {
                net,
                word: None,
                offset: None,
                width: None,
                kind: sim_ir::SelKind::Bit,
            }],
        };
        let sv = Value::from_i128(s as i32 as i128, 32, true);
        let off = self.resolve_lvalue_offsets(&lv);
        self.k_write_lvalue(&lv, sv, &off);
        Value::from_i128(r as i128, 32, true)
    }
    fn k_cast_rhs(&self, rhs: u32) -> bool {
        matches!(
            self.st.ir.exprs.get(rhs as usize),
            Some(sim_ir::Expr::SysFunc {
                which: sim_ir::SysFuncId::Cast,
                args,
            }) if args.len() == 2
        )
    }
    fn k_cast(&mut self, rhs: u32) -> Value {
        // func-form `ok = $cast(dst, src)`: write the resized `src` into the `dst`
        // ref arg and return 1. iverilog 13.0 does NOT support $cast (no oracle):
        // hand-IEEE §6.24.2 — an integral assignment always succeeds in this
        // class-free subset, so the status is always 1 (failure=0 needs class /
        // strict-enum range checks vita does not model).
        let (dst_net, src_arg) = match self.st.ir.exprs.get(rhs as usize) {
            Some(sim_ir::Expr::SysFunc { args, .. }) if args.len() == 2 => {
                let net = match self.st.ir.exprs.get(args[0] as usize) {
                    Some(sim_ir::Expr::Signal { net, word: None }) => Some(*net),
                    _ => None,
                };
                (net, args[1])
            }
            _ => return Value::from_i128(0, 32, true),
        };
        let Some(net) = dst_net else {
            return Value::from_i128(0, 32, true);
        };
        let lv = Lvalue {
            chunks: vec![sim_ir::LvalChunk {
                net,
                word: None,
                offset: None,
                width: None,
                kind: sim_ir::SelKind::Bit,
            }],
        };
        // context-size `src` to the dst width, then write through the funnel.
        let v = self.eval_for_lvalue(&lv, src_arg);
        let off = self.resolve_lvalue_offsets(&lv);
        self.k_write_lvalue(&lv, v, &off);
        Value::from_i128(1, 32, true)
    }
    fn k_value_plusargs_rhs(&self, rhs: u32) -> bool {
        matches!(
            self.st.ir.exprs.get(rhs as usize),
            Some(sim_ir::Expr::SysFunc {
                which: sim_ir::SysFuncId::ValuePlusargs,
                ..
            })
        )
    }
    fn k_value_plusargs(&mut self, rhs: u32) -> Value {
        // args = [fmt string-literal Const, ref-var whole-net Signal] —
        // elaborate's contract; defend a hand-built IR by returning 0.
        let (fmt_eid, var_net) = match self.st.ir.exprs.get(rhs as usize) {
            Some(sim_ir::Expr::SysFunc { args, .. }) if args.len() == 2 => {
                let var = match self.st.ir.exprs.get(args[1] as usize) {
                    Some(sim_ir::Expr::Signal { net, word: None }) => Some(*net),
                    _ => None,
                };
                (args[0], var)
            }
            _ => (u32::MAX, None),
        };
        let fmt = match self.st.ir.exprs.get(fmt_eid as usize) {
            Some(sim_ir::Expr::Const { val }) => crate::builtins::const_string(self.st.ir, *val),
            _ => return Value::from_i128(0, 32, true),
        };
        let Some(net) = var_net else {
            return Value::from_i128(0, 32, true);
        };
        // split "prefix%C" — elaborate validated exactly one supported spec.
        let Some(pct) = fmt.find('%') else {
            // degenerate no-spec format: a pure test probe, nothing written.
            let hit = self.st.plusargs.iter().any(|p| p.starts_with(&fmt));
            return Value::from_i128(hit as i128, 32, true);
        };
        let prefix = &fmt[..pct];
        let conv = fmt[pct + 1..].chars().next().unwrap_or('d');
        let Some(rest) = self
            .st
            .plusargs
            .iter()
            .find_map(|p| p.strip_prefix(prefix).map(|r| r.to_string()))
        else {
            return Value::from_i128(0, 32, true); // MISS: var untouched
        };
        let radix = match conv {
            'd' | 'D' => 10,
            'h' | 'H' | 'x' | 'X' => 16,
            'o' | 'O' => 8,
            'b' | 'B' => 2,
            _ => 0, // %s
        };
        let value = if radix == 0 {
            // %s: pack the raw bytes MSB-first (IEEE §5.9 string packing).
            let bytes = rest.as_bytes();
            let w = (bytes.len() as u32 * 8).max(8);
            let mut v = Value::zeros(w, false);
            for (i, &by) in bytes.iter().rev().enumerate() {
                let bit = i * 8;
                v.val[bit / 64] |= (by as u64) << (bit % 64);
            }
            v
        } else {
            // scanf-style: optional sign, then leading digits of the radix.
            let (neg, digits) = match rest.strip_prefix('-') {
                Some(d) => (true, d),
                None => (false, rest.as_str()),
            };
            let lead: String = digits.chars().take_while(|c| c.is_digit(radix)).collect();
            let mag = u64::from_str_radix(&lead, radix).unwrap_or(0);
            let raw = if neg {
                (mag as i64).wrapping_neg() as u64
            } else {
                mag
            };
            let mut v = Value::zeros(64, false);
            v.val[0] = raw;
            v
        };
        let lv = Lvalue {
            chunks: vec![sim_ir::LvalChunk {
                net,
                word: None,
                offset: None,
                width: None,
                kind: sim_ir::SelKind::Bit,
            }],
        };
        let off = self.resolve_lvalue_offsets(&lv);
        self.k_write_lvalue(&lv, value, &off);
        Value::from_i128(1, 32, true)
    }
    fn k_fopen_rhs(&self, rhs: u32) -> bool {
        matches!(
            self.st.ir.exprs.get(rhs as usize),
            Some(sim_ir::Expr::SysFunc {
                which: sim_ir::SysFuncId::Fopen,
                ..
            })
        )
    }
    fn k_fopen(&mut self, rhs: u32) -> Value {
        // args = [name strconst (, mode strconst)] — elaborate's contract.
        let args = match self.st.ir.exprs.get(rhs as usize) {
            Some(sim_ir::Expr::SysFunc { args, .. }) => args.clone(),
            _ => return Value::from_i128(0, 32, true),
        };
        let name = match args.first().map(|&a| self.st.ir.exprs.get(a as usize)) {
            Some(Some(sim_ir::Expr::Const { val })) => {
                crate::builtins::const_string(self.st.ir, *val)
            }
            _ => return Value::from_i128(0, 32, true),
        };
        let mode = args
            .get(1)
            .and_then(|&a| match self.st.ir.exprs.get(a as usize) {
                Some(sim_ir::Expr::Const { val }) => {
                    Some(crate::builtins::const_string(self.st.ir, *val))
                }
                _ => None,
            });
        let open = |mode: &str| -> std::io::Result<std::fs::File> {
            let mut o = std::fs::OpenOptions::new();
            // a '+' mode (r+/w+/a+) is read-AND-write; plain w/a are write-only.
            let plus = mode.contains('+');
            match mode.trim_end_matches('b') {
                "r" | "r+" => o.read(true).write(plus),
                "a" | "a+" => o.create(true).append(true).read(plus),
                // "w"/"w+" and anything unrecognized: truncate-write (the
                // overwhelmingly common TB mode; unknown modes behave as "w").
                _ => o.create(true).write(true).truncate(true).read(plus),
            };
            o.open(&name)
        };
        let fd = match mode {
            Some(m) => match open(&m) {
                Ok(f) => {
                    let n = self.st.next_fd;
                    self.st.next_fd = self.st.next_fd.saturating_add(1); // FD-RECLAIM: no wrap
                    let fd = 0x8000_0000 | n;
                    self.st.files.insert(fd, f);
                    // v9 SYS-READ: a mode with 'r' or '+' is read-capable
                    // (r/r+/w+/a+); plain "w"/"a" stays write-only and absent.
                    if m.contains('r') || m.contains('+') {
                        self.st.readable_fds.insert(fd);
                    }
                    fd
                }
                Err(_) => 0, // IEEE: $fopen failure returns 0
            },
            None => match open("w") {
                Ok(f) => {
                    // MCD-RECLAIM: hand out the LOWEST channel bit not currently
                    // open (bit 0 = stdout, reserved), so a $fclose'd bit is
                    // reused (iverilog reclaims). Byte-identical to the old
                    // monotonic counter when nothing has been freed.
                    match (1..31u32).find(|b| !self.st.mcd_files.contains_key(b)) {
                        Some(bit) => {
                            self.st.mcd_files.insert(bit, f);
                            1u32 << bit
                        }
                        None => return Value::from_i128(0, 32, true), // space full
                    }
                }
                Err(_) => 0,
            },
        };
        let mut v = Value::zeros(32, true);
        v.val[0] = fd as u64;
        v
    }
    // ── v9 SYS-READ: file-read int functions ($fgetc/$feof/$ungetc) ──
    fn k_fgetc_rhs(&self, rhs: u32) -> bool {
        matches!(
            self.st.ir.exprs.get(rhs as usize),
            Some(sim_ir::Expr::SysFunc {
                which: sim_ir::SysFuncId::Fgetc,
                ..
            })
        )
    }
    fn k_fgetc(&mut self, rhs: u32) -> Value {
        let fd_arg = match self.st.ir.exprs.get(rhs as usize) {
            Some(sim_ir::Expr::SysFunc { args, .. }) if !args.is_empty() => args[0],
            _ => return Value::from_i128(-1, 32, true),
        };
        let fdv = self.eval(fd_arg);
        if fdv.has_xz() {
            return Value::from_i128(-1, 32, true);
        }
        let fd = fdv.to_u64().unwrap_or(0) as u32;
        match crate::builtins::file_read_byte(self, fd) {
            Some(b) => Value::from_i128(b as i128, 32, true),
            None => Value::from_i128(-1, 32, true),
        }
    }
    fn k_feof_rhs(&self, rhs: u32) -> bool {
        matches!(
            self.st.ir.exprs.get(rhs as usize),
            Some(sim_ir::Expr::SysFunc {
                which: sim_ir::SysFuncId::Feof,
                ..
            })
        )
    }
    fn k_feof(&mut self, rhs: u32) -> Value {
        let fd_arg = match self.st.ir.exprs.get(rhs as usize) {
            Some(sim_ir::Expr::SysFunc { args, .. }) if !args.is_empty() => args[0],
            _ => return Value::from_i128(-1, 32, true),
        };
        let fdv = self.eval(fd_arg);
        if fdv.has_xz() {
            return Value::from_i128(-1, 32, true);
        }
        let fd = fdv.to_u64().unwrap_or(0) as u32;
        // §21.3.4: the pre-opened descriptors are always-valid, never-EOF fds
        // (iverilog-pinned: `$feof(STDOUT)` = 0, no warning — mirroring the
        // write-only-fd rule, whose failed `$fgetc` never latches EOF).
        if (0x8000_0000..=0x8000_0002).contains(&fd) {
            return Value::from_i128(0, 32, true);
        }
        // a bad/closed fd → −1 (iverilog parity, NOT 0); an open fd that has
        // not yet hit EOF → 0.
        if fd & 0x8000_0000 == 0 || !self.st.files.contains_key(&fd) {
            crate::builtins::bad_fd_warn(self, fd);
            return Value::from_i128(-1, 32, true);
        }
        let eof = self.st.read_state.get(&fd).map(|s| s.eof).unwrap_or(false);
        Value::from_i128(if eof { 1 } else { 0 }, 32, true)
    }
    fn k_ungetc_rhs(&self, rhs: u32) -> bool {
        matches!(
            self.st.ir.exprs.get(rhs as usize),
            Some(sim_ir::Expr::SysFunc {
                which: sim_ir::SysFuncId::Ungetc,
                ..
            })
        )
    }
    fn k_ungetc(&mut self, rhs: u32) -> Value {
        let (c_arg, fd_arg) = match self.st.ir.exprs.get(rhs as usize) {
            Some(sim_ir::Expr::SysFunc { args, .. }) if args.len() >= 2 => (args[0], args[1]),
            _ => return Value::from_i128(-1, 32, true),
        };
        let cv = self.eval(c_arg);
        let fdv = self.eval(fd_arg);
        if fdv.has_xz() {
            return Value::from_i128(-1, 32, true);
        }
        let fd = fdv.to_u64().unwrap_or(0) as u32;
        // The EOF sentinel is ONLY the exact int −1 (0xffff_ffff, fully known).
        // iverilog treats every other c — INCLUDING a value with x/z bits — as
        // a normal char and pushes its low byte (x/z bits coerced to 0).
        if !cv.has_xz() && (cv.to_u64().unwrap_or(0) as u32) == 0xffff_ffff {
            return Value::from_i128(-1, 32, true);
        }
        // §21.3.4: the pre-opened STDOUT/STDERR follow the write-only rule —
        // −1, no warning. STDIN pushback is part of the deferred stdin-read
        // feature → −1 quietly too (nothing to push back into).
        if (0x8000_0000..=0x8000_0002).contains(&fd) {
            return Value::from_i128(-1, 32, true);
        }
        // a bad/closed fd warns + returns −1; a valid but write-only ("w"/"a")
        // fd returns −1 WITHOUT a warning (iverilog: a write stream is not
        // pushable and never becomes readable). Only a read-capable fd accepts
        // a pushback.
        if fd & 0x8000_0000 == 0 || !self.st.files.contains_key(&fd) {
            crate::builtins::bad_fd_warn(self, fd);
            return Value::from_i128(-1, 32, true);
        }
        if !self.st.readable_fds.contains(&fd) {
            return Value::from_i128(-1, 32, true);
        }
        // the pushed byte = the low 8 bits with x/z bits coerced to 0.
        let mut byte = 0u8;
        for i in 0..8 {
            let (v, u) = cv.get_vu(i);
            if u == 0 && v != 0 {
                byte |= 1 << i;
            }
        }
        // LIFO push (iverilog retains every pushed byte); pushing clears EOF
        // (there is data to read again).
        let st = self.st.read_state.entry(fd).or_default();
        st.pushback.push(byte);
        st.eof = false;
        Value::from_i128(0, 32, true)
    }
    fn k_fgets_rhs(&self, rhs: u32) -> bool {
        matches!(
            self.st.ir.exprs.get(rhs as usize),
            Some(sim_ir::Expr::SysFunc {
                which: sim_ir::SysFuncId::Fgets,
                ..
            })
        )
    }
    fn k_fgets(&mut self, rhs: u32) -> Value {
        // args = [str-dest whole-net Signal, fd] — elaborate's contract.
        let (dest_net, fd_arg) = match self.st.ir.exprs.get(rhs as usize) {
            Some(sim_ir::Expr::SysFunc { args, .. }) if args.len() >= 2 => {
                let net = match self.st.ir.exprs.get(args[0] as usize) {
                    Some(sim_ir::Expr::Signal { net, word: None }) => Some(*net),
                    _ => None,
                };
                (net, args[1])
            }
            _ => (None, u32::MAX),
        };
        let Some(net) = dest_net else {
            return Value::from_i128(0, 32, true);
        };
        let fdv = self.eval(fd_arg);
        if fdv.has_xz() {
            return Value::from_i128(0, 32, true);
        }
        let fd = fdv.to_u64().unwrap_or(0) as u32;
        // capacity = the dest in whole bytes (iverilog reads the FULL width N,
        // not C's N-1 — no NUL is reserved).
        let width = self.st.ir.nets[net as usize].width.max(1);
        let max_bytes = (width / 8) as usize;
        let whole_net = |net: u32| Lvalue {
            chunks: vec![sim_ir::LvalChunk {
                net,
                word: None,
                offset: None,
                width: None,
                kind: sim_ir::SelKind::Bit,
            }],
        };
        if max_bytes == 0 {
            // sub-byte dest (width < 8): iverilog reads NO stream byte but
            // CLEARS the dest to 0 (C fgets into a too-small buffer => empty
            // string written), returning 0.
            let lv = whole_net(net);
            let off = self.resolve_lvalue_offsets(&lv);
            self.k_write_lvalue(&lv, Value::zeros(width, false), &off);
            return Value::from_i128(0, 32, true);
        }
        // read the line: up to max_bytes OR through a newline (retained).
        let mut raw: Vec<u8> = Vec::new();
        let mut any_read = false;
        while raw.len() < max_bytes {
            match crate::builtins::file_read_byte(self, fd) {
                Some(b) => {
                    any_read = true;
                    raw.push(b);
                    if b == b'\n' {
                        break;
                    }
                }
                None => break,
            }
        }
        if !any_read {
            // genuine EOF / bad-fd / write-only: dest UNCHANGED, count 0.
            return Value::from_i128(0, 32, true);
        }
        // the RETURNED string stops at the first NUL (C string semantics); the
        // bytes after it were still consumed from the stream above, so the file
        // position matches iverilog.
        let n = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
        // pack the first n bytes right-justified MSB-first (first byte = most
        // significant) into a width-wide value; n == 0 (leading NUL) leaves it
        // all-zero, which CLEARS the dest — iverilog writes 0, not the prior
        // value. n*8 <= width because n <= max_bytes = width / 8.
        let mut v = Value::zeros(width, false);
        for (i, &by) in raw[..n].iter().rev().enumerate() {
            let bit = i * 8;
            v.val[bit / 64] |= (by as u64) << (bit % 64);
        }
        let lv = whole_net(net);
        let off = self.resolve_lvalue_offsets(&lv);
        self.k_write_lvalue(&lv, v, &off);
        Value::from_i128(n as i128, 32, true)
    }
    fn k_fread_rhs(&self, rhs: u32) -> bool {
        matches!(
            self.st.ir.exprs.get(rhs as usize),
            Some(sim_ir::Expr::SysFunc {
                which: sim_ir::SysFuncId::Fread,
                ..
            })
        )
    }
    fn k_fread(&mut self, rhs: u32) -> Value {
        // args = [target whole-net Signal, fd, start?, count?] — elaborate's
        // contract (a single reg OR a whole memory; element-select is loud).
        let (net, fd_arg, start_arg, count_arg) = match self.st.ir.exprs.get(rhs as usize) {
            Some(sim_ir::Expr::SysFunc { args, .. }) if args.len() >= 2 => {
                let net = match self.st.ir.exprs.get(args[0] as usize) {
                    Some(sim_ir::Expr::Signal { net, word: None }) => Some(*net),
                    _ => None,
                };
                (net, args[1], args.get(2).copied(), args.get(3).copied())
            }
            _ => (None, u32::MAX, None, None),
        };
        let Some(net) = net else {
            return Value::from_i128(0, 32, true);
        };
        let fdv = self.eval(fd_arg);
        if fdv.has_xz() {
            return Value::from_i128(0, 32, true);
        }
        let fd = fdv.to_u64().unwrap_or(0) as u32;
        let (w, alen) = {
            let nv = &self.st.ir.nets[net as usize];
            (nv.width.max(1), nv.array_len.max(1) as u64)
        };
        let cap_per = ((w + 7) / 8) as usize; // bytes per element/reg
        if alen <= 1 {
            // ── single reg/vector: read ceil(w/8) bytes, MSB-slot fill ──
            let mut got: Vec<u8> = Vec::new();
            for _ in 0..cap_per {
                match crate::builtins::file_read_byte(self, fd) {
                    Some(b) => got.push(b),
                    None => break,
                }
            }
            if got.is_empty() {
                return Value::from_i128(0, 32, true); // EOF: dest UNCHANGED
            }
            let prior = self.st.read_net(net, None);
            let v = fill_reg_slots(&prior, w, &got);
            let lv = Lvalue {
                chunks: vec![sim_ir::LvalChunk {
                    net,
                    word: None,
                    offset: None,
                    width: None,
                    kind: sim_ir::SelKind::Bit,
                }],
            };
            let off = self.resolve_lvalue_offsets(&lv);
            self.k_write_lvalue(&lv, v, &off);
            return Value::from_i128(got.len() as i128, 32, true);
        }
        // ── memory: fill elements ascending from `start` (declared index) ──
        let base = self
            .st
            .net_dims
            .get(&net)
            .and_then(|d| d.first())
            .map(|&(lo, hi)| lo.min(hi) as u64)
            .unwrap_or(0);
        let last = base + alen - 1;
        // iverilog evaluates the start/count operands by coercing each x/z bit
        // to 0 (NOT by treating an x/z operand as absent), so a present operand
        // always counts — `4'b001x` => 2, `3'bxxx` => 0 (an explicit count 0).
        let coerce_xz0 = |v: &Value| -> u64 {
            v.val.first().copied().unwrap_or(0) & !v.unk.first().copied().unwrap_or(0)
        };
        let start = match start_arg {
            Some(a) => {
                let v = self.eval(a);
                coerce_xz0(&v)
            }
            None => base,
        };
        let count = count_arg.map(|a| {
            let v = self.eval(a);
            coerce_xz0(&v)
        });
        let warn = |sched: &mut Scheduler, msg: String| {
            sched
                .st
                .sink
                .emit(diag::LogEvent::Diagnostic(diag::Diagnostic {
                    severity: diag::Severity::Warning,
                    code: diag::MsgCode::RunReadmem,
                    message: msg,
                    location: None,
                    context: Vec::new(),
                    sim_time: Some(diag::TimeStamp {
                        ticks: sched.st.now,
                    }),
                }));
        };
        if start < base || start > last {
            warn(
                self,
                format!(
                    "$fread start argument ({start}) is outside the memory range [{base}:{last}]"
                ),
            );
            return Value::from_i128(0, 32, true);
        }
        let avail = last - start + 1;
        let n_elems = match count {
            Some(c) if c > avail => {
                warn(
                    self,
                    format!("$fread count argument ({c}) is too large for start ({start}) and the memory range [{base}:{last}]; clamped"),
                );
                avail
            }
            Some(c) => c,
            None => avail,
        };
        let mut rc: u64 = 0;
        for e in 0..n_elems {
            let word = (start + e - base) as u32;
            let mut got: Vec<u8> = Vec::new();
            let mut hit_eof = false;
            for _ in 0..cap_per {
                match crate::builtins::file_read_byte(self, fd) {
                    Some(b) => got.push(b),
                    None => {
                        hit_eof = true;
                        break;
                    }
                }
            }
            if got.is_empty() {
                break; // no more data — leave this and later elements untouched
            }
            rc += got.len() as u64;
            let prior = self.st.read_net(net, Some(word));
            let v = fill_reg_slots(&prior, w, &got);
            let lv = Lvalue {
                chunks: vec![sim_ir::LvalChunk {
                    net,
                    word: Some(0),
                    offset: None,
                    width: None,
                    kind: sim_ir::SelKind::Bit,
                }],
            };
            let off = crate::exec::Offsets::Inline {
                buf: [(0, word), (0, 0)],
                len: 1,
            };
            self.st.write_lvalue(&lv, v, &off);
            if hit_eof {
                break;
            }
        }
        Value::from_i128(rc as i128, 32, true)
    }
    fn k_fscanf_rhs(&self, rhs: u32) -> bool {
        matches!(
            self.st.ir.exprs.get(rhs as usize),
            Some(sim_ir::Expr::SysFunc {
                which: sim_ir::SysFuncId::Fscanf,
                ..
            })
        )
    }
    fn k_fscanf(&mut self, rhs: u32) -> Value {
        // args = [fd, fmt strconst, dst0, dst1, ...] — elaborate's contract.
        let args: Vec<u32> = match self.st.ir.exprs.get(rhs as usize) {
            Some(sim_ir::Expr::SysFunc { args, .. }) if args.len() >= 2 => args.clone(),
            _ => return Value::from_i128(-1, 32, true),
        };
        let fdv = self.eval(args[0]);
        if fdv.has_xz() {
            return Value::from_i128(-1, 32, true);
        }
        let fd = fdv.to_u64().unwrap_or(0) as u32;
        let fmt: Vec<u8> = match self.st.ir.exprs.get(args[1] as usize) {
            Some(sim_ir::Expr::Const { val }) => {
                crate::builtins::const_string(self.st.ir, *val).into_bytes()
            }
            _ => return Value::from_i128(-1, 32, true),
        };
        let dsts: Vec<u32> = args[2..]
            .iter()
            .filter_map(|&a| match self.st.ir.exprs.get(a as usize) {
                Some(sim_ir::Expr::Signal { net, word: None }) => Some(*net),
                _ => None,
            })
            .collect();
        scan_run(self, Some(fd), &[], &fmt, &dsts)
    }
    fn k_sscanf_rhs(&self, rhs: u32) -> bool {
        matches!(
            self.st.ir.exprs.get(rhs as usize),
            Some(sim_ir::Expr::SysFunc {
                which: sim_ir::SysFuncId::Sscanf,
                ..
            })
        )
    }
    fn k_sscanf(&mut self, rhs: u32) -> Value {
        // args = [source string-VALUE, fmt strconst, dst0, ...].
        let args: Vec<u32> = match self.st.ir.exprs.get(rhs as usize) {
            Some(sim_ir::Expr::SysFunc { args, .. }) if args.len() >= 2 => args.clone(),
            _ => return Value::from_i128(-1, 32, true),
        };
        let src: Vec<u8> = self.eval(args[0]).to_str_bytes();
        let fmt: Vec<u8> = match self.st.ir.exprs.get(args[1] as usize) {
            Some(sim_ir::Expr::Const { val }) => {
                crate::builtins::const_string(self.st.ir, *val).into_bytes()
            }
            _ => return Value::from_i128(-1, 32, true),
        };
        let dsts: Vec<u32> = args[2..]
            .iter()
            .filter_map(|&a| match self.st.ir.exprs.get(a as usize) {
                Some(sim_ir::Expr::Signal { net, word: None }) => Some(*net),
                _ => None,
            })
            .collect();
        scan_run(self, None, &src, &fmt, &dsts)
    }
    fn k_sformatf_rhs(&self, rhs: u32) -> bool {
        matches!(
            self.st.ir.exprs.get(rhs as usize),
            Some(sim_ir::Expr::SysFunc {
                which: sim_ir::SysFuncId::Sformatf,
                ..
            })
        )
    }
    fn k_sformatf(&mut self, rhs: u32) -> Value {
        // args = [fmt string-literal Const, value args…] (elaborate contract).
        let Some(sim_ir::Expr::SysFunc { args, .. }) = self.st.ir.exprs.get(rhs as usize) else {
            return Value::from_str_bytes(&[]);
        };
        let (fmt, rest) = (args.first().copied(), args.get(1..).unwrap_or(&[]).to_vec());
        let text = crate::builtins::format_args_str(self, fmt, &rest, None);
        Value::from_str_bytes(text.as_bytes())
    }
    fn k_disable_fork(&mut self) {
        // IEEE §9.6.3: terminate every ACTIVE DESCENDANT of the calling
        // process. Transitive walk: barriers parented by the kill set spread
        // to their children. The arena is append-only and the walk is
        // index-ordered — deterministic. Stale resume entries are dropped at
        // the `run_body` choke.
        let mut kill: std::collections::BTreeSet<u32> = std::collections::BTreeSet::new();
        kill.insert(self.cur_aid);
        loop {
            let mut grew = false;
            for (aid, a) in self.activities.iter().enumerate() {
                if a.dead || a.reported || kill.contains(&(aid as u32)) {
                    continue;
                }
                let Some(jr) = a.join_ref else { continue };
                let parent = self.barriers[jr as usize].parent;
                if kill.contains(&parent) {
                    kill.insert(aid as u32);
                    grew = true;
                }
            }
            if !grew {
                break;
            }
        }
        kill.remove(&self.cur_aid); // the caller itself lives on
                                    // §16.4: a deferred report pending in a KILLED process is cancelled (the
                                    // action never matures). Drop by `(aid, gen)` of the LIVE killed
                                    // activities so a recycled slot's COMPLETED predecessor report (a
                                    // different gen under the same aid) is NOT also cancelled.
        let mut kill_keys: std::collections::BTreeSet<(u32, u32)> =
            std::collections::BTreeSet::new();
        for &aid in &kill {
            self.activities[aid as usize].dead = true;
            kill_keys.insert((aid, self.activities[aid as usize].gen));
        }
        if !self.st.postponed.deferred_observed.is_empty() {
            self.st
                .postponed
                .deferred_observed
                .retain(|&(_, aid, gen), _| !kill_keys.contains(&(aid, gen)));
        }
        if !self.st.postponed.deferred_reactive.is_empty() {
            self.st
                .postponed
                .deferred_reactive
                .retain(|&(_, aid, gen), _| !kill_keys.contains(&(aid, gen)));
        }
    }
    fn k_queue_pop(&mut self, lhs: &Lvalue, rhs: u32) -> Value {
        // `k_queue_pop_rhs` guaranteed the shape; everything below is
        // defensive against a hand-built IR — degrade, never panic.
        let Some(sim_ir::Expr::SysFunc { which, args }) = self.st.ir.exprs.get(rhs as usize) else {
            return Value::xs(1, false);
        };
        let front = matches!(which, sim_ir::SysFuncId::QPopFront);
        let net = args
            .first()
            .and_then(|&a| match self.st.ir.exprs.get(a as usize) {
                Some(sim_ir::Expr::Signal { net, word: None }) => Some(*net),
                _ => None,
            });
        let popped = match net {
            Some(n) => {
                let (w, signed) = self
                    .st
                    .ir
                    .nets
                    .get(n as usize)
                    .map(|nv| (nv.width.max(1), nv.signed))
                    .unwrap_or((1, false));
                match self
                    .st
                    .dyn_heap
                    .get_mut(n as usize)
                    .and_then(|o| o.as_mut())
                {
                    Some(crate::state::DynObj::Queue { elems }) if !elems.is_empty() => {
                        let v = if front {
                            elems.pop_front()
                        } else {
                            elems.pop_back()
                        };
                        v.unwrap_or_else(|| Value::xs(w, signed))
                    }
                    _ => {
                        // empty (a missing entry IS the empty queue) or a
                        // non-queue object: element-width X + warn-once
                        // (iverilog live: per-call warning + x; our once-latch
                        // is the established anti-spam policy).
                        self.st.dyn_warn_once_at(n, "pop on an empty queue (X)");
                        Value::xs(w, signed)
                    }
                }
            }
            None => Value::xs(1, false),
        };
        // Context-size EXACTLY as `k_eval_for_lvalue` sizes an rhs: width =
        // max(lhs width, pop self-width), extension driven by the pop's
        // self-signedness (= the ELEMENT's, via the width table).
        let lw = self.st.lvalue_width(lhs);
        let sw = self.st.wt.get(rhs);
        popped.resize_keep_sign(lw.max(sw.width), sw.signed)
    }
    fn k_assoc_iter_rhs(&self, rhs: u32) -> bool {
        matches!(
            self.st.ir.exprs.get(rhs as usize),
            Some(sim_ir::Expr::SysFunc {
                which: sim_ir::SysFuncId::AssocFirst
                    | sim_ir::SysFuncId::AssocNext
                    | sim_ir::SysFuncId::AssocLast
                    | sim_ir::SysFuncId::AssocPrev,
                ..
            })
        )
    }
    fn k_assoc_iter(&mut self, lhs: &Lvalue, rhs: u32) -> Value {
        let status = self.assoc_iter_step(rhs);
        // Context-size the int status exactly as `k_queue_pop` sizes its
        // result (self-width of the rhs = 32 signed via the width table).
        let mut v = Value::zeros(32, true);
        v.val[0] = (status as u32) as u64;
        let lw = self.st.lvalue_width(lhs);
        let sw = self.st.wt.get(rhs);
        v.resize_keep_sign(lw.max(sw.width), sw.signed)
    }

    // ── terminator / control surface (C1) — pure forwarders ──
    fn k_truthy(&self, eid: u32) -> bool {
        self.truthy(eid)
    }
    fn k_rearm(&mut self, proc: u32) {
        self.rearm(proc);
    }
    fn k_max_deltas(&self) -> u64 {
        self.max_deltas_guard()
    }
    fn k_mark_fatal(&mut self) {
        self.mark_fatal();
    }
    fn k_class_new_site(&self, sid: u32) -> Option<u32> {
        self.st.class_new_sites.get(&sid).copied()
    }
    fn k_class_alloc(&mut self, class_id: u32) -> Value {
        let id = self.st.class_alloc(class_id);
        // CLASS-HEAP-CAP: the heap is never garbage-collected, so an unbounded
        // `new()` in a loop would grow without limit. Bound it to a loud fatal
        // (graceful $finish) instead of an OOM — this is the single allocation
        // chokepoint (`class_alloc`'s only caller).
        if self.st.class_heap.borrow().len() as u64 > self.st.max_class_objs {
            self.st.fatal_class_limit();
        }
        // The handle holds the object-id as a 32-bit unsigned integer (0 = null).
        Value::from_i128(id as i128, 32, false)
    }
}

// ── helpers ──────────────────────────────────────────────────────────────

/// Insert keeping sorted by `tie` (stable; equal ties keep insertion order).
fn push_sorted(q: &mut Vec<Ready>, r: Ready) {
    let pos = q.partition_point(|x| x.tie <= r.tie);
    q.insert(pos, r);
}

/// Child tie = `(parent_tie+1)` in the high 16 bits, child declaration index in
/// the low 16. `parent` is ALWAYS a top-level process (nested fork is an
/// elaborate ERROR), so `parent_tie ∈ [0, nproc)` is a small dense int and the
/// shift is applied EXACTLY ONCE — never chained. The `+1` offset makes children
/// sort STRICTLY AFTER their parent for all `parent_tie` (including 0), while
/// preserving relative parent ordering and declaration order among siblings. v1
/// limits — ≤ 65534 top-level processes and ≤ 65536 children per fork — are
/// ENFORCED at the spawn site (FORK-TIE-CAP in `exec_fork`); above them the high
/// or low half would overflow/alias, so this helper is only ever reached within
/// the safe range.
fn compose_child_tie(parent_tie: u32, child_idx: u32) -> u32 {
    ((parent_tie + 1) << 16) | (child_idx & 0xFFFF)
}

/// GLITCH: edge firing from a net's intra-slot `slot_edge` accumulator mask
/// (bit0 = posedge occurred, bit1 = negedge, bit2 = bit0 changed at all). For a
/// net written ONCE in the slot the mask is exactly `{is_posedge, is_negedge,
/// prev!=cur}` of the single `prev→cur` transition, so this returns the same as
/// `edge_fires(kind, prev, cur)` — only an A→B→A glitch (which the endpoint
/// compare loses) diverges, recovering the IEEE §9 "fire once per slot".
fn edge_fires_slot(mask: u8, kind: EdgeKind) -> bool {
    match kind {
        EdgeKind::Posedge => mask & 1 != 0,
        EdgeKind::Negedge => mask & 2 != 0,
        EdgeKind::AnyEdge => mask & 4 != 0,
    }
}

/// MULTI-DRIVER: fold one driver value `d` into accumulator `acc` by IEEE 1364
/// 4-state WIRE resolution, bitwise per word. Encoding (val,unk): (0,0)=0,
/// (1,0)=1, (0,1)=X, (1,1)=Z. Z (=val&unk) is the identity — it yields to the
/// other driver; two equal non-Z bits keep the value; two differing non-Z bits
/// (a 0/1 conflict, or any X) resolve to X. Commutative + associative, so
/// folding every driver from an all-Z start is order-independent (matches the
/// oracle table verified across all 16 (a,b) pairs). `acc` and `d` share width.
fn resolve_wire_into(acc: &mut Value, d: &Value) {
    let n = acc.val.len();
    for w in 0..n {
        let av = acc.val[w];
        let au = acc.unk[w];
        let bv = d.val.get(w).copied().unwrap_or(0);
        let bu = d.unk.get(w).copied().unwrap_or(0);
        let az = av & au; // acc bit is Z
        let bz = bv & bu; // driver bit is Z
        let same = !(av ^ bv) & !(au ^ bu); // (av,au) == (bv,bu)
        let take_b = az; // acc Z → take the driver
        let take_a = !az & (bz | same); // driver Z, or equal → keep acc
        let x_bits = !az & !bz & !same; // both non-Z and differ → X
        acc.val[w] = (take_b & bv) | (take_a & av);
        acc.unk[w] = (take_b & bu) | (take_a & au) | x_bits;
    }
    acc.mask_top();
}

/// MULTI-DRIVER (WAND): fold a driver into `acc` by IEEE wired-AND resolution
/// (oracle-verified 16 pairs). Z is the identity; a 0 on any driver forces 0;
/// two 1s give 1; anything else with an X gives X.
fn resolve_wand_into(acc: &mut Value, d: &Value) {
    for w in 0..acc.val.len() {
        let (av, au) = (acc.val[w], acc.unk[w]);
        let bv = d.val.get(w).copied().unwrap_or(0);
        let bu = d.unk.get(w).copied().unwrap_or(0);
        let az = av & au;
        let bz = bv & bu;
        let take_b = az;
        let take_a = !az & bz;
        let rest = !az & !bz;
        let a0 = !av & !au;
        let b0 = !bv & !bu;
        let a1 = av & !au;
        let b1 = bv & !bu;
        let one = rest & a1 & b1;
        let xx = rest & !(a0 | b0) & !(a1 & b1);
        acc.val[w] = (take_b & bv) | (take_a & av) | one;
        acc.unk[w] = (take_b & bu) | (take_a & au) | xx;
    }
    acc.mask_top();
}

/// MULTI-DRIVER (WOR): fold a driver into `acc` by IEEE wired-OR resolution.
/// Z is the identity; a 1 on any driver forces 1; two 0s give 0; else X.
fn resolve_wor_into(acc: &mut Value, d: &Value) {
    for w in 0..acc.val.len() {
        let (av, au) = (acc.val[w], acc.unk[w]);
        let bv = d.val.get(w).copied().unwrap_or(0);
        let bu = d.unk.get(w).copied().unwrap_or(0);
        let az = av & au;
        let bz = bv & bu;
        let take_b = az;
        let take_a = !az & bz;
        let rest = !az & !bz;
        let a0 = !av & !au;
        let b0 = !bv & !bu;
        let a1 = av & !au;
        let b1 = bv & !bu;
        let one = rest & (a1 | b1);
        let xx = rest & !(a1 | b1) & !(a0 & b0);
        acc.val[w] = (take_b & bv) | (take_a & av) | one;
        acc.unk[w] = (take_b & bu) | (take_a & au) | xx;
    }
    acc.mask_top();
}

/// S1: effective inertial delay for a delayed continuous-assign / gate-prim
/// value change `old → new` with distinct rise/fall/turnoff specs (IEEE 1364
/// §7.14 / §28 — confirmed against iverilog 13.0 as ATOMIC-at-max, not per-bit
/// separate arrival). The whole net updates at `now + D` where `D` is the MAX,
/// over every bit `i` that actually changes (`old[i] != new[i]`), of that bit's
/// DESTINATION-based delay:
///   new bit → 1  : `rise`
///   new bit → 0  : `fall`
///   new bit → z  : `turnoff`
///   new bit → x  : `min(rise, fall, turnoff)`
/// First drive (`old == None`) treats ALL bits as changed. `get_vu` 4-state
/// encoding: (0,0)=0, (1,0)=1, (0,1)=X, (1,1)=Z. This is only reached when the
/// value actually changed, so at least one bit differs; the `rise` fallback for
/// the impossible no-change case is harmless.
fn transition_delay(old: Option<&Value>, new: &Value, rise: u32, fall: u32, toff: u32) -> u32 {
    let x_delay = rise.min(fall).min(toff);
    let mut d: Option<u32> = None;
    for i in 0..new.width {
        let (nv, nu) = new.get_vu(i);
        // A bit is "changed" if there is no prior value (first drive) or the old
        // bit differs from the new bit.
        let changed = match old {
            None => true,
            Some(o) => {
                // Compare only within the old value's width; bits beyond it (the
                // first-drive case for wider news) count as changed.
                if i < o.width {
                    o.get_vu(i) != (nv, nu)
                } else {
                    true
                }
            }
        };
        if !changed {
            continue;
        }
        let bit_d = match (nv, nu) {
            (1, 0) => rise, // → 1
            (0, 0) => fall, // → 0
            (1, 1) => toff, // → z
            _ => x_delay,   // → x  (0,1)
        };
        d = Some(d.map_or(bit_d, |m| m.max(bit_d)));
    }
    d.unwrap_or(rise)
}
