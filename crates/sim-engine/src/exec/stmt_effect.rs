//! The `stmt_effect` family members whose body is SHARED between the two
//! `Kernel` implementors (A1-ii, ROADMAP §5.1-f).
//!
//! ## Why these live here and not on `Scheduler`
//!
//! Every member of this family does the same three things: read one or two
//! operands that are NOT the enclosing assignment's rhs, compute, and write a
//! REF ARGUMENT (a seed variable, a `$cast` destination, an iteration key) from
//! inside the call rather than through the statement's own lvalue. The write was
//! never the problem — it already went out through `Kernel::k_write_lvalue`, so
//! it lands in whichever store the calling kernel owns. The READS were: they
//! called `Scheduler::eval` / `SimState::read_net`, which are hard-wired to the
//! engine's nets, the one store a native run never writes.
//!
//! Moving the bodies here and taking `&mut impl Kernel` fixes that without a
//! second spelling: the engine's own seams are the same functions it called
//! before (`k_eval` → `Scheduler::eval`, `k_write_lvalue` → its write funnel), so
//! the engine path is mechanically byte-identical, and tier-3 gets the arena
//! everywhere by construction rather than by review.
//!
//! ⚠️ The alternative — decomposing each into a pure part plus a `(lvalue, value)`
//! the caller applies, as `exec::plusargs::effect` does — does not scale past one
//! member: `$dist_*` reads a VARIABLE number of parameters, so the "pure part"
//! would have to return a `Vec` and the shape argument stops paying for itself.

use sim_ir::Lvalue;

use crate::exec::Kernel;
use crate::value::Value;

/// The whole-net lvalue for a REF ARGUMENT — a `$random` seed, a `$cast`
/// destination, an assoc iteration key. Every one of them is elaborate's
/// whole-net `Signal` contract, so the chunk is offset-free and width-free and
/// the write funnel derives the destination width itself.
fn whole_net_lvalue(net: u32) -> Lvalue {
    Lvalue {
        chunks: vec![sim_ir::LvalChunk {
            net,
            word: None,
            offset: None,
            width: None,
            kind: sim_ir::SelKind::Bit,
        }],
    }
}

/// The net id behind a whole-net `Signal` ExprId, or `None` for any other shape.
/// Every caller here is defending a HAND-BUILT IR — the `kpred` probe already
/// matched the id, and elaborate only ever emits `Signal { word: None }` in these
/// argument positions — so `None` degrades rather than panics.
fn whole_net_of<K: Kernel + ?Sized>(k: &K, eid: u32) -> Option<u32> {
    match k.k_ir().exprs.get(eid as usize) {
        Some(sim_ir::Expr::Signal { net, word: None }) => Some(*net),
        _ => None,
    }
}

/// Read a 32-bit seed. X/Z reads as 0, then the IEEE 1364-2005 Annex-N
/// zero-substitution applies — the same answer an uninitialized iverilog `reg`
/// gives.
fn seed_in<K: Kernel + ?Sized>(k: &K, seed_eid: u32) -> u32 {
    let cur = k.k_eval(seed_eid);
    if cur.has_xz() {
        0
    } else {
        (cur.to_u64().unwrap_or(0) & 0xffff_ffff) as u32
    }
}

/// Write the advanced seed back through the kernel's own funnel — a resize to
/// the variable's width, exactly like any blocking assign.
fn seed_out<K: Kernel + ?Sized>(k: &mut K, net: u32, s: u32) {
    let lv = whole_net_lvalue(net);
    let sv = Value::from_i128(s as i32 as i128, 32, true);
    let off = k.k_resolve_lvalue_offsets(&lv);
    k.k_write_lvalue(&lv, sv, &off);
}

/// `r = $random(seed)` — advance the Annex-N LCG in `seed` and return the draw.
pub(crate) fn random_seeded<K: Kernel + ?Sized>(k: &mut K, rhs: u32) -> Value {
    let seed_eid = match k.k_ir().exprs.get(rhs as usize) {
        Some(sim_ir::Expr::SysFunc { args, .. }) => args.first().copied(),
        _ => None,
    };
    let Some(seed_eid) = seed_eid else {
        return Value::xs(32, true);
    };
    let Some(net) = whole_net_of(k, seed_eid) else {
        return Value::xs(32, true);
    };
    let mut s = seed_in(k, seed_eid);
    let r = crate::rng::annex_n_random(&mut s);
    seed_out(k, net, s);
    Value::from_i128(r as i128, 32, true)
}

/// `r = $dist_*(seed, params…)` — same seed contract, a distribution kernel in
/// place of the raw draw. Parameters are `integer` (signed 32-bit); X/Z reads 0.
pub(crate) fn dist_seeded<K: Kernel + ?Sized>(k: &mut K, rhs: u32) -> Value {
    let (which, seed_eid, params) = match k.k_ir().exprs.get(rhs as usize) {
        Some(sim_ir::Expr::SysFunc { which, args }) if !args.is_empty() => {
            (*which, args[0], args[1..].to_vec())
        }
        _ => return Value::xs(32, true),
    };
    let Some(net) = whole_net_of(k, seed_eid) else {
        return Value::xs(32, true);
    };
    let p: Vec<i32> = params
        .iter()
        .map(|&a| k.k_eval(a).to_u64().unwrap_or(0) as u32 as i32)
        .collect();
    // ⚠️ ORDER: the parameters are evaluated BEFORE the seed is read, which is
    // the engine's order and is observable — a parameter expression may itself
    // be impure (`$dist_uniform(s, 0, $random)`).
    let mut s = seed_in(k, seed_eid);
    let p0 = *p.first().unwrap_or(&0);
    let p1 = *p.get(1).unwrap_or(&0);
    use sim_ir::SysFuncId as F;
    let r = match which {
        F::DistUniform => crate::rng::dist_uniform(&mut s, p0, p1),
        F::DistNormal => crate::rng::dist_normal(&mut s, p0, p1),
        F::DistExponential => crate::rng::dist_exponential(&mut s, p0),
        F::DistPoisson => crate::rng::dist_poisson(&mut s, p0),
        F::DistChiSquare => crate::rng::dist_chi_square(&mut s, p0),
        F::DistT => crate::rng::dist_t(&mut s, p0),
        F::DistErlang => crate::rng::dist_erlang(&mut s, p0, p1),
        // Unreachable behind `kpred::dist_seeded_rhs`; a hand-built IR gets 0
        // rather than a panic, matching every other defensive path here.
        _ => 0,
    };
    seed_out(k, net, s);
    Value::from_i128(r as i128, 32, true)
}

/// The FUNCTION form `ok = $cast(dst, src)` — write the context-sized `src` into
/// the `dst` ref arg and return 1.
///
/// iverilog 13.0 does not support `$cast` (no oracle): hand-IEEE §6.24.2 — an
/// integral assignment always succeeds in this class-free subset, so the status
/// is always 1. A failure status of 0 would need class / strict-enum range
/// checks vita does not model.
pub(crate) fn cast<K: Kernel + ?Sized>(k: &mut K, rhs: u32) -> Value {
    let fail = Value::from_i128(0, 32, true);
    let (dst_eid, src_arg) = match k.k_ir().exprs.get(rhs as usize) {
        Some(sim_ir::Expr::SysFunc { args, .. }) if args.len() == 2 => (args[0], args[1]),
        _ => return fail,
    };
    let Some(net) = whole_net_of(k, dst_eid) else {
        return fail;
    };
    let lv = whole_net_lvalue(net);
    // context-size `src` to the dst width, then write through the funnel.
    let v = k.k_eval_for_lvalue(&lv, src_arg);
    let off = k.k_resolve_lvalue_offsets(&lv);
    k.k_write_lvalue(&lv, v, &off);
    Value::from_i128(1, 32, true)
}

/// `st = aa.first(k)` / `.next` / `.last` / `.prev` — position the iteration key
/// and return the status.
///
/// The locate half stays on `SimState` (`assoc_iter_compute`): it walks
/// `dyn_heap`, which is ONE object both backends share. What this function owns
/// is the store-DEPENDENT part — reading the CURRENT key for `next`/`prev`
/// through this kernel's store, and writing the new key back through its funnel
/// (dirty channel included, so `@(k)` sensitivity and VCD see it like any
/// blocking assign).
pub(crate) fn assoc_iter<K: Kernel + ?Sized>(k: &mut K, lhs: &Lvalue, rhs: u32) -> Value {
    let cur = k.k_assoc_iter_cur_key(rhs).map(|eid| k.k_eval(eid));
    let (key_write, status) = k.k_assoc_iter_compute(rhs, cur);
    if let Some((knet, kval)) = key_write {
        let klv = whole_net_lvalue(knet);
        let off = k.k_resolve_lvalue_offsets(&klv);
        k.k_write_lvalue(&klv, kval, &off);
    }
    // Context-size the int status exactly as `k_queue_pop` sizes its result
    // (self-width of the rhs = 32 signed via the width table).
    let mut v = Value::zeros(32, true);
    v.val[0] = (status as u32) as u64;
    let lw = k.k_lvalue_width(lhs);
    let (sw, ssigned) = k.k_self_width(rhs);
    v.resize_keep_sign(lw.max(sw), ssigned)
}

/// `n = $sscanf(src, fmt, dsts…)` — scan a STRING (no file descriptor) and write
/// each matched destination.
///
/// A1-iv-a. Two things were store-dependent and both are routed now: the SOURCE
/// (`args[0]`, an ordinary expression that usually names a `string` net) and the
/// destination writes, which `scan_write_dst` performs through
/// `Kernel::k_write_lvalue`. The scan itself never touches a store — with
/// `fd = None` the byte source is the `src` slice — which is why this member
/// needs no file-table plumbing at all and ships ahead of its seven siblings.
pub(crate) fn sscanf<K: Kernel + ?Sized>(k: &mut K, rhs: u32) -> Value {
    let args: Vec<u32> = match k.k_ir().exprs.get(rhs as usize) {
        Some(sim_ir::Expr::SysFunc { args, .. }) if args.len() >= 2 => args.clone(),
        _ => return Value::from_i128(-1, 32, true),
    };
    let src: Vec<u8> = k.k_eval(args[0]).to_str_bytes();
    let fmt: Vec<u8> = match k.k_ir().exprs.get(args[1] as usize) {
        Some(sim_ir::Expr::Const { val }) => {
            crate::builtins::const_string(k.k_ir(), *val).into_bytes()
        }
        _ => return Value::from_i128(-1, 32, true),
    };
    let dsts: Vec<u32> = args[2..]
        .iter()
        .filter_map(|&a| match k.k_ir().exprs.get(a as usize) {
            Some(sim_ir::Expr::Signal { net, word: None }) => Some(*net),
            _ => None,
        })
        .collect();
    crate::sched::scan_run(k, None, &src, &fmt, &dsts)
}

// ── A1-iv-b: the fd family ─────────────────────────────────────────────────
// Six of the seven; `$fread` follows in A1-iv-c (it is the only one that reads
// the DESTINATION's prior value and needs an array base, i.e. seams these do
// not). What was store-dependent in all of them is the same two things every
// earlier A1 slice found: the argument reads, and the ref-arg writes. The FILE
// TABLE is not — it lives in `SimState`, one object both backends see, exactly
// as `dyn_heap` does, so this is routing and not a second store.

/// The fd operand of a file function: `None` when the shape is malformed or the
/// value carries x/z, which every member below answers with its own failure code.
fn fd_of<K: Kernel + ?Sized>(k: &K, eid: u32) -> Option<u32> {
    let v = k.k_eval(eid);
    if v.has_xz() {
        return None;
    }
    Some(v.to_u64().unwrap_or(0) as u32)
}

/// The pre-opened descriptors (§21.3.4): always valid, never EOF, never pushable.
fn is_preopened(fd: u32) -> bool {
    (0x8000_0000..=0x8000_0002).contains(&fd)
}

/// `fd = $fopen(name [, mode])`.
///
/// The name/mode arguments are the only store-dependent part: each is a string
/// LITERAL, a runtime `string` value, or a packed reg holding ASCII (elaborate's
/// relaxed contract), and the latter two are net reads.
pub(crate) fn fopen<K: Kernel + ?Sized>(k: &mut K, rhs: u32) -> Value {
    let fail = Value::from_i128(0, 32, true);
    let args: Vec<u32> = match k.k_ir().exprs.get(rhs as usize) {
        Some(sim_ir::Expr::SysFunc { args, .. }) => args.clone(),
        _ => return fail,
    };
    // Resolve a name/mode arg to text: a `Const{StrUtf8}` literal decodes
    // directly; a runtime STRING value (`is_str`) renders its exact bytes; any
    // other packed value is treated as ASCII in a reg (NUL-stripped) — all three
    // are valid `$fopen` argument forms (§21.3, iverilog parity).
    fn resolve<K: Kernel + ?Sized>(k: &K, a: u32) -> String {
        if let Some(sim_ir::Expr::Const { val }) = k.k_ir().exprs.get(a as usize) {
            return crate::builtins::const_string(k.k_ir(), *val);
        }
        let v = k.k_eval(a);
        if v.is_str {
            String::from_utf8_lossy(&v.to_str_bytes()).into_owned()
        } else {
            crate::builtins::fmt_packed_chars_min(&v)
        }
    }
    let Some(&a0) = args.first() else {
        return fail;
    };
    let name = resolve(k, a0);
    let mode = args.get(1).map(|&a| resolve(k, a));
    let fd = k.k_file_open(&name, mode.as_deref());
    let mut v = Value::zeros(32, true);
    v.val[0] = fd as u64;
    v
}

/// `c = $fgetc(fd)` — one byte, or −1 at EOF / on a bad or write-only fd.
pub(crate) fn fgetc<K: Kernel + ?Sized>(k: &mut K, rhs: u32) -> Value {
    let eof = Value::from_i128(-1, 32, true);
    let fd_arg = match k.k_ir().exprs.get(rhs as usize) {
        Some(sim_ir::Expr::SysFunc { args, .. }) if !args.is_empty() => args[0],
        _ => return eof,
    };
    let Some(fd) = fd_of(k, fd_arg) else {
        return eof;
    };
    match k.k_file_read_byte(fd) {
        Some(b) => Value::from_i128(b as i128, 32, true),
        None => eof,
    }
}

/// `$feof(fd)` — 1 at EOF, 0 while readable, −1 for a bad or closed fd.
pub(crate) fn feof<K: Kernel + ?Sized>(k: &mut K, rhs: u32) -> Value {
    let bad = Value::from_i128(-1, 32, true);
    let fd_arg = match k.k_ir().exprs.get(rhs as usize) {
        Some(sim_ir::Expr::SysFunc { args, .. }) if !args.is_empty() => args[0],
        _ => return bad,
    };
    let Some(fd) = fd_of(k, fd_arg) else {
        return bad;
    };
    // §21.3.4: the pre-opened descriptors are always-valid, never-EOF fds
    // (iverilog-pinned: `$feof(STDOUT)` = 0, no warning — mirroring the
    // write-only-fd rule, whose failed `$fgetc` never latches EOF).
    if is_preopened(fd) {
        return Value::from_i128(0, 32, true);
    }
    // a bad/closed fd → −1 (iverilog parity, NOT 0); an open fd that has not yet
    // hit EOF → 0.
    match k.k_file_eof(fd) {
        Some(eof) => Value::from_i128(if eof { 1 } else { 0 }, 32, true),
        None => bad,
    }
}

/// `$ungetc(c, fd)` — push one byte back; 0 on success, −1 otherwise.
pub(crate) fn ungetc<K: Kernel + ?Sized>(k: &mut K, rhs: u32) -> Value {
    let bad = Value::from_i128(-1, 32, true);
    let (c_arg, fd_arg) = match k.k_ir().exprs.get(rhs as usize) {
        Some(sim_ir::Expr::SysFunc { args, .. }) if args.len() >= 2 => (args[0], args[1]),
        _ => return bad,
    };
    let cv = k.k_eval(c_arg);
    let Some(fd) = fd_of(k, fd_arg) else {
        return bad;
    };
    // The EOF sentinel is ONLY the exact int −1 (0xffff_ffff, fully known).
    // iverilog treats every other c — INCLUDING a value with x/z bits — as a
    // normal char and pushes its low byte (x/z bits coerced to 0).
    if !cv.has_xz() && (cv.to_u64().unwrap_or(0) as u32) == 0xffff_ffff {
        return bad;
    }
    // §21.3.4: the pre-opened STDOUT/STDERR follow the write-only rule — −1, no
    // warning. STDIN pushback is part of the deferred stdin-read feature → −1
    // quietly too (nothing to push back into).
    if is_preopened(fd) {
        return bad;
    }
    // the pushed byte = the low 8 bits with x/z bits coerced to 0.
    let mut byte = 0u8;
    for i in 0..8 {
        let (v, u) = cv.get_vu(i);
        if u == 0 && v != 0 {
            byte |= 1 << i;
        }
    }
    if k.k_file_ungetc(fd, byte) {
        Value::from_i128(0, 32, true)
    } else {
        bad
    }
}

/// `n = $fgets(dest, fd)` — read one line into `dest`, return its length.
pub(crate) fn fgets<K: Kernel + ?Sized>(k: &mut K, rhs: u32) -> Value {
    let none = Value::from_i128(0, 32, true);
    // args = [str-dest whole-net Signal, fd] — elaborate's contract.
    let (dest_net, fd_arg) = match k.k_ir().exprs.get(rhs as usize) {
        Some(sim_ir::Expr::SysFunc { args, .. }) if args.len() >= 2 => {
            (whole_net_of(k, args[0]), args[1])
        }
        _ => (None, u32::MAX),
    };
    let Some(net) = dest_net else {
        return none;
    };
    let Some(fd) = fd_of(k, fd_arg) else {
        return none;
    };
    // v7: a SystemVerilog `string` dest is a dynamic HANDLE (`NetKind::String`,
    // net width 0). It has NO byte capacity, so it must NOT fall into the
    // sub-byte (width < 8) reg branch below, which would clear the dest and
    // return 0 (the silent-wrong that fix addressed). Read the WHOLE line
    // uncapped (through a retained newline, else to EOF), pack it MSB-first, and
    // write it via the same string lvalue path as `s = "..."` (§6.16 byte-strip).
    if k.k_ir().nets[net as usize].kind == sim_ir::NetKind::String {
        let mut raw: Vec<u8> = Vec::new();
        while let Some(b) = k.k_file_read_byte(fd) {
            raw.push(b);
            if b == b'\n' {
                break;
            }
        }
        if raw.is_empty() {
            // genuine EOF / bad-fd / write-only: dest UNCHANGED, count 0.
            return none;
        }
        // C-string semantics (iverilog parity, same as the reg path below): the
        // STORED string and the returned count stop at the first NUL, even though
        // the whole line was already consumed from the stream. A leading NUL
        // gives n = 0 → the dest is set to the empty string (distinct from the
        // EOF arm above, which leaves it UNCHANGED).
        let n = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
        let lv = whole_net_lvalue(net);
        let off = k.k_resolve_lvalue_offsets(&lv);
        k.k_write_lvalue(&lv, Value::from_str_bytes(&raw[..n]), &off);
        return Value::from_i128(n as i128, 32, true);
    }
    // capacity = the dest in whole bytes (iverilog reads the FULL width N, not
    // C's N-1 — no NUL is reserved).
    let width = k.k_ir().nets[net as usize].width.max(1);
    let max_bytes = (width / 8) as usize;
    if max_bytes == 0 {
        // sub-byte dest (width < 8): iverilog reads NO stream byte but CLEARS
        // the dest to 0 (C fgets into a too-small buffer => empty string
        // written), returning 0.
        let lv = whole_net_lvalue(net);
        let off = k.k_resolve_lvalue_offsets(&lv);
        k.k_write_lvalue(&lv, Value::zeros(width, false), &off);
        return none;
    }
    // read the line: up to max_bytes OR through a newline (retained).
    let mut raw: Vec<u8> = Vec::new();
    let mut any_read = false;
    while raw.len() < max_bytes {
        match k.k_file_read_byte(fd) {
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
        return none;
    }
    // the RETURNED string stops at the first NUL (C string semantics); the bytes
    // after it were still consumed from the stream above, so the file position
    // matches iverilog.
    let n = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
    // pack the first n bytes right-justified MSB-first (first byte = most
    // significant) into a width-wide value; n == 0 (leading NUL) leaves it
    // all-zero, which CLEARS the dest — iverilog writes 0, not the prior value.
    // n*8 <= width because n <= max_bytes = width / 8.
    let mut v = Value::zeros(width, false);
    for (i, &by) in raw[..n].iter().rev().enumerate() {
        let bit = i * 8;
        v.val[bit / 64] |= (by as u64) << (bit % 64);
    }
    let lv = whole_net_lvalue(net);
    let off = k.k_resolve_lvalue_offsets(&lv);
    k.k_write_lvalue(&lv, v, &off);
    Value::from_i128(n as i128, 32, true)
}

/// `n = $fscanf(fd, fmt, dsts…)` — the descriptor twin of [`sscanf`].
pub(crate) fn fscanf<K: Kernel + ?Sized>(k: &mut K, rhs: u32) -> Value {
    let bad = Value::from_i128(-1, 32, true);
    let args: Vec<u32> = match k.k_ir().exprs.get(rhs as usize) {
        Some(sim_ir::Expr::SysFunc { args, .. }) if args.len() >= 2 => args.clone(),
        _ => return bad,
    };
    let Some(fd) = fd_of(k, args[0]) else {
        return bad;
    };
    let fmt: Vec<u8> = match k.k_ir().exprs.get(args[1] as usize) {
        Some(sim_ir::Expr::Const { val }) => {
            crate::builtins::const_string(k.k_ir(), *val).into_bytes()
        }
        _ => return bad,
    };
    let dsts: Vec<u32> = args[2..]
        .iter()
        .filter_map(|&a| whole_net_of(k, a))
        .collect();
    crate::sched::scan_run(k, Some(fd), &[], &fmt, &dsts)
}
