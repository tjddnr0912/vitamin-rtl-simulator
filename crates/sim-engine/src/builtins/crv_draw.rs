//! split part of `builtins` (mechanical move).

use super::*;

/// Run the randomize() draw; returns whether it SUCCEEDED (§18.11). A null/X
/// handle fails (0); a class with no rand fields succeeds trivially (1); the
/// rejection sampler succeeds when it finds a satisfying assignment within the
/// cap, else fails (fields left unchanged).
pub(crate) fn class_randomize_run(sched: &mut Scheduler, args: &[u32]) -> bool {
    let Some(&handle_e) = args.first() else {
        return false;
    };
    let hv = sched.eval_ctx_top(handle_e, 32, false);
    if hv.unk.iter().any(|&u| u != 0) {
        return false; // X/Z handle → fail
    }
    let id = hv.val.first().copied().unwrap_or(0) as u32;
    if id == 0 {
        return false; // null handle → fail
    }
    let class_id = match sched.st.class_heap.borrow().get(&id) {
        Some(o) => o.class_id,
        None => return false,
    };
    let Some(mut rand) = sched.st.class_rand.get(class_id as usize).cloned() else {
        return true; // no rand table → nothing to randomize → success
    };
    if rand.is_empty() {
        return true; // no rand fields → trivially succeeds
    }
    let mut preds: Vec<Vec<sim_ir::COp>> = sched
        .st
        .class_constraints
        .get(class_id as usize)
        .cloned()
        .unwrap_or_default();
    // N7-REST B-CRV final: inline `randomize() with {…}` (IEEE §18.7) — the with-id
    // is a Const arg. Its per-call domain overrides INTERSECT the class `[lo,hi]`
    // (an empty intersection ⇒ infeasible ⇒ fail BEFORE drawing); its predicates
    // are ANDed with the class predicates. Absent ⇒ byte-identical to B2.
    if let Some((domains, extra)) = inline_with_call(sched, args) {
        for (fi, ilo, ihi) in domains {
            if let Some(b) = rand.iter_mut().find(|r| r.0 == fi) {
                b.3 = b.3.max(ilo);
                b.4 = b.4.min(ihi);
                b.5 = true; // constrained ⇒ draw within the narrowed [lo,hi]
                if b.3 > b.4 {
                    return false; // inline ∩ class is empty → randomize fails (§18.11)
                }
            }
        }
        preds.extend(extra);
    }
    let mut seed = sched.st.randomize_seed.get();
    // Rejection sampling (B2): draw every rand field from its `class_rand` domain,
    // keep the candidate only if every predicate holds. Draw order + the per-field
    // draw are IDENTICAL to B1, so a design with no general predicate accepts on the
    // FIRST try — byte-identical to B1. SOFT constraints (§18.5.14) are tried first
    // (phase 1 = hard+soft); if that is infeasible, the soft predicates are dropped
    // and only the hard ones retried (phase 2). The seed stream is consumed
    // deterministically → reproducible + 3-OS byte-identical.
    let dist: Vec<crate::DistField> = sched
        .st
        .class_dist
        .get(class_id as usize)
        .cloned()
        .unwrap_or_default();
    let randc: Vec<crate::RandcField> = sched
        .st
        .class_randc
        .get(class_id as usize)
        .cloned()
        .unwrap_or_default();
    // `randc` fields (cyclic, B2 v1 = unconstrained) are drawn SEPARATELY via a
    // per-instance permutation (a value visited once per cycle), not through the
    // rejection loop. The non-randc fields go through `try_solve`.
    let mut randc_updates: Vec<(u32, Value)> = Vec::new();
    for &(field_idx, lo, hi) in &randc {
        let (w, s) = rand
            .iter()
            .find(|r| r.0 == field_idx)
            .map(|r| (r.1, r.2))
            .unwrap_or((32, false));
        let v = draw_randc(
            &mut sched.st.randc_state,
            &mut seed,
            (id, field_idx),
            lo,
            hi,
            w,
            s,
        );
        randc_updates.push((field_idx, v));
    }
    let rand_rej: Vec<crate::RandBound> = rand
        .iter()
        .filter(|r| !randc.iter().any(|rc| rc.0 == r.0))
        .cloned()
        .collect();
    let has_soft = preds
        .iter()
        .any(|p| matches!(p.first(), Some(sim_ir::COp::SoftMarker)));
    let mut accepted = try_solve(&rand_rej, &preds, &dist, &mut seed, false);
    if accepted.is_none() && has_soft {
        accepted = try_solve(&rand_rej, &preds, &dist, &mut seed, true);
    }
    sched.st.randomize_seed.set(seed);
    match accepted {
        Some(updates) => {
            let mut heap = sched.st.class_heap.borrow_mut();
            if let Some(obj) = heap.get_mut(&id) {
                for (idx, v) in updates.into_iter().chain(randc_updates) {
                    if let Some(slot) = obj.fields.get_mut(idx as usize) {
                        *slot = v;
                    }
                }
            }
            true
        }
        None => false, // cap exhausted / unsatisfiable → fields unchanged, fail
    }
}

/// Draw the next value of a `randc` field: a random permutation of `[lo,hi]` per
/// (object, field) is consumed one value at a time, reshuffled when exhausted
/// (seeded Fisher-Yates → deterministic + reproducible).
pub(crate) fn draw_randc(
    state: &mut std::collections::HashMap<(u32, u32), (Vec<i64>, usize)>,
    seed: &mut u32,
    key: (u32, u32),
    lo: i64,
    hi: i64,
    width: u32,
    signed: bool,
) -> Value {
    let entry = state.entry(key).or_insert((Vec::new(), 0));
    if entry.0.is_empty() || entry.1 >= entry.0.len() {
        let n = (hi - lo + 1).max(1) as usize;
        let mut perm: Vec<i64> = (0..n as i64).map(|i| lo + i).collect();
        for i in (1..n).rev() {
            let j = crate::rng::dist_uniform(seed, 0, i as i32) as usize;
            perm.swap(i, j);
        }
        entry.0 = perm;
        entry.1 = 0;
    }
    let v = entry.0[entry.1];
    entry.1 += 1;
    value_from_bits(v as u64, width, signed)
}

/// One rejection-sampling pass: draw every field, accept the first candidate that
/// satisfies all predicates (skipping SOFT predicates when `drop_soft`). Returns
/// the accepted draws, or None if the cap is exhausted. The per-field draw matches
/// B1 exactly so a constraint-free design is byte-identical.
pub(crate) fn try_solve(
    rand: &[crate::RandBound],
    preds: &[Vec<sim_ir::COp>],
    dist: &[crate::DistField],
    seed: &mut u32,
    drop_soft: bool,
) -> Option<Vec<(u32, Value)>> {
    const MAX_TRIES: u32 = 10_000;
    for _ in 0..MAX_TRIES {
        let mut cand: std::collections::HashMap<u32, i64> = Default::default();
        let mut draws: Vec<(u32, Value)> = Vec::with_capacity(rand.len());
        for &(field_idx, width, signed, lo, hi, constrained) in rand {
            let v = if let Some((_, entries)) = dist.iter().find(|(fi, _)| *fi == field_idx) {
                // `dist`: weighted-sample from the distribution (§18.5.4).
                draw_dist(seed, entries, width, signed)
            } else if constrained {
                draw_in_range(seed, lo, hi, width, signed)
            } else {
                random_full_width(seed, width, signed)
            };
            cand.insert(field_idx, value_to_i64(&v, width, signed));
            draws.push((field_idx, v));
        }
        let ok = preds.iter().all(|p| {
            if drop_soft && matches!(p.first(), Some(sim_ir::COp::SoftMarker)) {
                return true; // soft predicate dropped this phase
            }
            eval_pred(p, &cand)
        });
        if ok {
            return Some(draws);
        }
    }
    None
}

/// Weighted-sample a `dist` field: pick an entry with probability proportional to
/// its total weight, then a uniform value within that entry's `[lo,hi]`. Seeded /
/// deterministic. An empty / all-zero-weight distribution draws 0.
pub(crate) fn draw_dist(
    seed: &mut u32,
    entries: &[(i64, i64, i64)],
    width: u32,
    signed: bool,
) -> Value {
    let total: i64 = entries.iter().map(|e| e.2).sum();
    if total <= 0 {
        return value_from_bits(0, width, signed);
    }
    let r = if total - 1 <= i32::MAX as i64 {
        crate::rng::dist_uniform(seed, 0, (total - 1) as i32) as i64
    } else {
        (draw_u64(seed) % total as u64) as i64
    };
    let mut acc = 0i64;
    for &(lo, hi, w) in entries {
        acc += w;
        if r < acc {
            let v = draw_i64_range(seed, lo, hi);
            return value_from_bits(v as u64, width, signed);
        }
    }
    value_from_bits(entries[0].0 as u64, width, signed)
}

/// A uniform i64 in `[lo, hi]` (matches `draw_in_range`'s value lane: ≤i32 fast
/// path, else i128-span modulo). `lo == hi` ⇒ `lo` with no draw.
pub(crate) fn draw_i64_range(seed: &mut u32, lo: i64, hi: i64) -> i64 {
    if lo >= hi {
        return lo;
    }
    if lo >= i32::MIN as i64 && hi <= i32::MAX as i64 {
        crate::rng::dist_uniform(seed, lo as i32, hi as i32) as i64
    } else {
        let span = (hi as i128 - lo as i128 + 1) as u128;
        lo + (draw_u64(seed) as u128 % span) as i64
    }
}

/// Extract the signed i64 value of a drawn field for constraint-predicate eval
/// (the draw is always 2-state). Sign-extends a signed field narrower than 64;
/// a field ≥64 bits uses its low 64 bits (B2 evaluates predicates in i64).
pub(crate) fn value_to_i64(v: &Value, width: u32, signed: bool) -> i64 {
    let bits = v.val.first().copied().unwrap_or(0);
    if signed && width < 64 {
        let shift = 64 - width;
        ((bits << shift) as i64) >> shift
    } else {
        bits as i64
    }
}

/// Evaluate a postfix constraint predicate against the candidate field values.
pub(crate) fn eval_pred(prog: &[sim_ir::COp], cand: &std::collections::HashMap<u32, i64>) -> bool {
    let mut stack: Vec<i64> = Vec::with_capacity(8);
    for op in prog {
        match op {
            sim_ir::COp::Field(idx) => stack.push(cand.get(idx).copied().unwrap_or(0)),
            sim_ir::COp::Const(v) => stack.push(*v),
            sim_ir::COp::Not => {
                let a = stack.pop().unwrap_or(0);
                stack.push((a == 0) as i64);
            }
            sim_ir::COp::SoftMarker => {} // tag only — no stack effect
            sim_ir::COp::Bin(b) => {
                let r = stack.pop().unwrap_or(0);
                let l = stack.pop().unwrap_or(0);
                stack.push(apply_cbin(*b, l, r));
            }
        }
    }
    stack.pop().map(|v| v != 0).unwrap_or(false)
}

pub(crate) fn apply_cbin(op: sim_ir::CBinOp, l: i64, r: i64) -> i64 {
    use sim_ir::CBinOp as C;
    match op {
        C::Add => l.wrapping_add(r),
        C::Sub => l.wrapping_sub(r),
        C::Mul => l.wrapping_mul(r),
        C::Div => {
            if r == 0 {
                0
            } else {
                l.wrapping_div(r)
            }
        }
        C::Mod => {
            if r == 0 {
                0
            } else {
                l.wrapping_rem(r)
            }
        }
        C::Lt => (l < r) as i64,
        C::Le => (l <= r) as i64,
        C::Gt => (l > r) as i64,
        C::Ge => (l >= r) as i64,
        C::Eq => (l == r) as i64,
        C::Ne => (l != r) as i64,
        C::And => ((l != 0) && (r != 0)) as i64,
        C::Or => ((l != 0) || (r != 0)) as i64,
    }
}

/// Draw a uniform value in the inclusive `[lo, hi]` constraint range, honoring the
/// bound at ANY width: bounds that fit i32 use the iverilog-pinned `dist_uniform`
/// (preserving the verified ≤32-bit draws); wider i64 bounds use a 64-bit draw
/// reduced modulo the (u128) span. The result is masked to the field width.
pub(crate) fn draw_in_range(seed: &mut u32, lo: i64, hi: i64, width: u32, signed: bool) -> Value {
    if lo > hi {
        return value_from_bits(0, width, signed); // empty range (elaborate rejected)
    }
    if lo >= i32::MIN as i64 && hi <= i32::MAX as i64 {
        let drawn = crate::rng::dist_uniform(seed, lo as i32, hi as i32);
        return value_from_bits(drawn as i64 as u64, width, signed);
    }
    // Wide/large bounds: a 64-bit draw mapped into the span. The span fits u128 even
    // for [i64::MIN, i64::MAX]; for power-of-two spans the modulo is exactly uniform.
    let span = (hi as i128 - lo as i128 + 1) as u128;
    let r = draw_u64(seed) as u128;
    let v = lo as i128 + (r % span) as i128;
    value_from_bits(v as i64 as u64, width, signed)
}

/// A 64-bit seeded draw assembled from two `dist_uniform` half-word draws (shares
/// the one deterministic `randomize()` stream).
pub(crate) fn draw_u64(seed: &mut u32) -> u64 {
    let hi = crate::rng::dist_uniform(seed, i32::MIN, i32::MAX) as u32 as u64;
    let lo = crate::rng::dist_uniform(seed, i32::MIN, i32::MAX) as u32 as u64;
    (hi << 32) | lo
}

/// A `width`-bit `Value` holding the low bits of `bits` (two's-complement when
/// signed); used for a ranged rand draw.
pub(crate) fn value_from_bits(bits: u64, width: u32, signed: bool) -> Value {
    // The drawn value lives in the LOW word. For a field wider than 64 bits, the
    // high words must be SIGN-filled for a negative signed draw (a `[-100,-50]`
    // constraint on a `rand bit signed [127:0]`); a plain single-word pack would
    // zero-pad and silently store a huge positive out-of-range value.
    let n = (width.max(1) as usize).div_ceil(64);
    let fill = if signed && (bits as i64) < 0 {
        u64::MAX
    } else {
        0
    };
    let mut val = vec![fill; n];
    val[0] = bits;
    Value::from_packed(
        &sim_ir::BitPacked {
            val,
            unk: vec![0; n],
        },
        width.max(1),
        signed,
    )
}

/// A full-width seeded random `Value` (an UNCONSTRAINED rand field), one 64-bit
/// `draw_u64` per word, masked to `width`.
pub(crate) fn random_full_width(seed: &mut u32, width: u32, signed: bool) -> Value {
    let nwords = (width as usize).div_ceil(64).max(1);
    let words: Vec<u64> = (0..nwords).map(|_| draw_u64(seed)).collect();
    Value::from_packed(
        &sim_ir::BitPacked {
            val: words,
            unk: vec![0; nwords],
        },
        width.max(1),
        signed,
    )
}

pub(crate) fn cast_task<N: crate::eval::NetReader + ?Sized>(
    sched: &mut Scheduler,
    nets: Option<&N>,
    out: &mut crate::builtins::TaskWrites<'_>,
    args: &[u32],
) {
    if args.len() != 2 {
        return;
    }
    let dst = match sched.st.ir.exprs.get(args[0] as usize) {
        Some(sim_ir::Expr::Signal { net, word: None }) => Some(*net),
        _ => None,
    };
    if let Some(net) = dst {
        let lv = sim_ir::Lvalue {
            chunks: vec![sim_ir::LvalChunk {
                net,
                word: None,
                offset: None,
                width: None,
                kind: sim_ir::SelKind::Bit,
            }],
        };
        // Context-size `src` to the dst width — `Scheduler::eval_for_lvalue`'s
        // rule, spelled through the THREADED reader. A1-iii MEASURED what the
        // untreaded call cost: on a native run `sc = 8'd200; $cast(dc, sc);`
        // printed `dc=x`, because `sc` had been written to the arena while
        // `eval_for_lvalue` read `SimState`.
        let lw = sched.st.lvalue_width(&lv);
        let sw = sched.st.wt.get(args[1]);
        let v =
            crate::builtins::eval_task_arg_ctx(sched, nets, args[1], lw.max(sw.width), sw.signed);
        let off = sched.resolve_lvalue_offsets(&lv);
        out.put(sched, lv, v, off);
    }
}

/// v7 `$readmemb/h(file, mem[, start[, finish]])` — iverilog-pinned (t11–14):
/// default fill = LOWEST declared index ascending (1364-2005), `@addr` is hex
/// in BOTH variants and lives in the DECLARED index domain, unwritten
/// elements keep their value, token shortfall warns only for directive-free
/// files, and every problem is W4023 + continue (exit parity with iverilog).
pub(crate) fn readmem<N: crate::eval::NetReader + ?Sized>(
    sched: &mut Scheduler,
    nets: Option<&N>,
    out: &mut crate::builtins::TaskWrites<'_>,
    args: &[u32],
    hex: bool,
) {
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
    let Some(&a0) = args.first() else { return };
    let name = match sched.st.ir.exprs.get(a0 as usize) {
        Some(sim_ir::Expr::Const { val }) => const_string(sched.st.ir, *val),
        _ => return,
    };
    let net = match args.get(1).and_then(|&a| sched.st.ir.exprs.get(a as usize)) {
        Some(sim_ir::Expr::Signal { net, word: None }) => *net,
        _ => {
            warn(sched, "$readmem target is not a memory".to_string());
            return;
        }
    };
    let (alen, w) = {
        let nv = &sched.st.ir.nets[net as usize];
        (nv.array_len.max(1) as u64, nv.width.max(1))
    };
    // declared base = min index of dim 0 (sparse table; absent ⇒ 0-based).
    // Multi-dim memories use flat word-offset addressing from that base.
    let Some(base) = super::queues_io::declared_array_base(&sched.st.net_dims, net) else {
        warn(
            sched,
            "$readmem into a memory with a NEGATIVE declared base (e.g. `reg m[-1:1]`) \
             is not supported; no elements loaded"
                .to_string(),
        );
        return;
    };
    let Ok(text) = std::fs::read_to_string(&name) else {
        warn(
            sched,
            format!("$readmem: unable to open '{name}' for reading"),
        );
        return;
    };
    // strip // line and /* */ block comments.
    let mut cleaned = String::with_capacity(text.len());
    let mut rest = text.as_str();
    'outer: while !rest.is_empty() {
        let line_c = rest.find("//");
        let block_c = rest.find("/*");
        match (line_c, block_c) {
            (Some(l), b) if b.is_none_or(|b| l < b) => {
                cleaned.push_str(&rest[..l]);
                match rest[l..].find('\n') {
                    Some(nl) => rest = &rest[l + nl..],
                    None => break 'outer,
                }
            }
            (_, Some(bs)) => {
                cleaned.push_str(&rest[..bs]);
                match rest[bs..].find("*/") {
                    Some(be) => rest = &rest[bs + be + 2..],
                    None => break 'outer,
                }
            }
            _ => {
                cleaned.push_str(rest);
                break 'outer;
            }
        }
    }
    // range window (declared-index domain). Default: full array ascending.
    // Threaded: the optional window bounds are ordinary expressions and may name
    // nets (`$readmemh(f, m, lo, hi)`), so they are a second store read exactly
    // as the slice-2c task arguments were. (`writemem`'s twin below is NOT
    // threaded and does not need to be: `$writemem*` reads the memory itself,
    // which is why `systask_refusal` refuses it outright.)
    let r_start = args
        .get(2)
        .and_then(|&a| crate::builtins::eval_task_arg(sched, nets, a).to_u64());
    let r_finish = args
        .get(3)
        .and_then(|&a| crate::builtins::eval_task_arg(sched, nets, a).to_u64());
    let (start, finish) = match (r_start, r_finish) {
        (Some(s), Some(f)) => (s, f),
        (Some(s), None) => (s, base + alen - 1),
        _ => (base, base + alen - 1),
    };
    let step: i64 = if start <= finish { 1 } else { -1 };
    let (win_lo, win_hi) = (start.min(finish), start.max(finish));
    let window = win_hi - win_lo + 1;

    let mut addr = start as i64;
    let mut wrote: u64 = 0;
    let mut had_at = false;
    for tok in cleaned.split_whitespace() {
        if let Some(a) = tok.strip_prefix('@') {
            had_at = true;
            match u64::from_str_radix(a, 16) {
                Ok(v) => addr = v as i64,
                Err(_) => warn(sched, format!("$readmem: bad address token '@{a}'")),
            }
            continue;
        }
        let a = addr as u64;
        if addr < 0 || a < win_lo || a > win_hi || a < base || a - base >= alen {
            warn(
                sched,
                format!("$readmem('{name}'): address {addr} outside the load range; stopped"),
            );
            return;
        }
        let val = parse_mem_token(tok, w, hex);
        let word = (a - base) as u32;
        // funnel write: the dummy `word: Some(0)` ExprId is never evaluated —
        // `write_chunk` takes the resolved word from the offsets pair.
        let lv = sim_ir::Lvalue {
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
        out.put(sched, lv, val, off);
        wrote += 1;
        addr += step;
    }
    if !had_at && wrote < window {
        warn(
            sched,
            format!(
                "$readmem('{name}'): not enough words for the range \
                 [{start}:{finish}] ({wrote} of {window}); rest unchanged"
            ),
        );
    }
}

/// v9 `$writememb/h(file, mem[, start[, finish]])` — the write-side mirror of
/// `readmem`, iverilog-pinned: the FIRST line is ALWAYS the literal
/// `// 0x00000000` header (it never reflects the base/start); each element is
/// one line (every line incl the last ends '\n'); the optional (start[,finish])
/// is an inclusive declared-index window, descending when finish < start; an
/// out-of-range start/finish is non-fatal (a warning, the file is NOT created,
/// the sim continues). Element values come from the engine word-read path; hex
/// uses per-nibble X/Z compression, bin is per-bit uncompressed.
pub(crate) fn writemem(sched: &mut Scheduler, args: &[u32], hex: bool) {
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
    let Some(&a0) = args.first() else { return };
    let name = match sched.st.ir.exprs.get(a0 as usize) {
        Some(sim_ir::Expr::Const { val }) => const_string(sched.st.ir, *val),
        _ => return,
    };
    let net = match args.get(1).and_then(|&a| sched.st.ir.exprs.get(a as usize)) {
        Some(sim_ir::Expr::Signal { net, word: None }) => *net,
        _ => {
            warn(sched, "$writemem target is not a memory".to_string());
            return;
        }
    };
    let (alen, w) = {
        let nv = &sched.st.ir.nets[net as usize];
        (nv.array_len.max(1) as u64, nv.width.max(1))
    };
    // declared base = min index of dim 0 (sparse table; absent ⇒ 0-based).
    let Some(base) = super::queues_io::declared_array_base(&sched.st.net_dims, net) else {
        warn(
            sched,
            "$writemem from a memory with a NEGATIVE declared base (e.g. `reg m[-1:1]`) \
             is not supported; no file written"
                .to_string(),
        );
        return;
    };
    let last = base + alen - 1;
    // range window (declared-index domain). Default: full array ascending.
    let r_start = args.get(2).and_then(|&a| sched.eval(a).to_u64());
    let r_finish = args.get(3).and_then(|&a| sched.eval(a).to_u64());
    let (start, finish) = match (r_start, r_finish) {
        (Some(s), Some(f)) => (s, f),
        (Some(s), None) => (s, last),
        _ => (base, last),
    };
    // OOB start/finish is non-fatal AND the file is NOT created (iverilog
    // validates before opening — file never appears). Mirror its report text.
    for (label, idx) in [("Start", start), ("Finish", finish)] {
        if idx < base || idx > last {
            warn(
                sched,
                format!(
                    "$writemem('{name}'): {label} address {idx} is out of bounds \
                     for the memory [{base}:{last}]; file not written"
                ),
            );
            return;
        }
    }
    let step: i64 = if start <= finish { 1 } else { -1 };
    let mut body = String::from("// 0x00000000\n");
    let mut addr = start as i64;
    loop {
        let word = (addr as u64 - base) as u32;
        let v = sched.st.read_net(net, Some(word));
        if hex {
            fmt_writemem_hex(&v, w, &mut body);
        } else {
            fmt_writemem_bin(&v, w, &mut body);
        }
        body.push('\n');
        if addr as u64 == finish {
            break;
        }
        addr += step;
    }
    if let Err(e) = std::fs::write(&name, body) {
        warn(
            sched,
            format!("$writemem: unable to open '{name}' for writing: {e}"),
        );
    }
}

/// One memory element → a `$writememh` hex field: ceil(w/4) lowercase digits,
/// MSB-first, with iverilog's per-nibble X/Z compression. The compression
/// examines ONLY the REAL bits of each nibble — a partial top nibble's phantom
/// zero-pad bit does NOT participate (iverilog-pinned: an all-x 3-bit top
/// nibble renders 'x', not 'X'). Rules: clean ⇒ hex digit; all-x ⇒ 'x';
/// all-z ⇒ 'z'; any x mixed in ⇒ 'X' (X dominates Z); else z mixed ⇒ 'Z'.
pub(crate) fn fmt_writemem_hex(v: &Value, w: u32, out: &mut String) {
    let ndig = w.div_ceil(4);
    for nib in (0..ndig).rev() {
        let (mut xc, mut zc, mut nbits, mut val) = (0u32, 0u32, 0u32, 0u32);
        for k in 0..4 {
            let bit = nib * 4 + k;
            if bit >= w {
                continue; // phantom pad bit — excluded from value AND compression
            }
            nbits += 1;
            let (bv, bu) = v.get_vu(bit);
            if bu != 0 {
                if bv != 0 {
                    zc += 1;
                } else {
                    xc += 1;
                }
            } else if bv != 0 {
                val |= 1 << k;
            }
        }
        let ch = if xc == 0 && zc == 0 {
            std::char::from_digit(val, 16).unwrap()
        } else if xc == nbits {
            'x'
        } else if zc == nbits {
            'z'
        } else if xc > 0 {
            'X'
        } else {
            'Z'
        };
        out.push(ch);
    }
}

/// One memory element → a `$writememb` binary field: exactly `w` per-bit chars,
/// MSB-first, NO compression (0/1/x/z lowercase).
pub(crate) fn fmt_writemem_bin(v: &Value, w: u32, out: &mut String) {
    for bit in (0..w).rev() {
        let (bv, bu) = v.get_vu(bit);
        out.push(match (bv != 0, bu != 0) {
            (false, false) => '0',
            (true, false) => '1',
            (false, true) => 'x',
            (true, true) => 'z',
        });
    }
}

/// One memory-file token → a `Value` of element width `w` (right-aligned,
/// high bits zero; surplus digits truncate on the left). Hex digits are 4
/// bits, binary 1; `x`/`z` poison their digit's bits; `_` is ignored.
pub(crate) fn parse_mem_token(tok: &str, w: u32, hex: bool) -> Value {
    let bits_per = if hex { 4u32 } else { 1 };
    // per-bit (val, unk) MSB-first
    let mut bits: Vec<(bool, bool)> = Vec::with_capacity(tok.len() * bits_per as usize);
    for ch in tok.chars() {
        match ch {
            '_' => {}
            'x' | 'X' => bits.extend(std::iter::repeat_n((false, true), bits_per as usize)),
            'z' | 'Z' | '?' => bits.extend(std::iter::repeat_n((true, true), bits_per as usize)),
            // a non-digit stray char skips (comment-residue defensiveness)
            c => {
                if let Some(d) = c.to_digit(if hex { 16 } else { 2 }) {
                    for k in (0..bits_per).rev() {
                        bits.push(((d >> k) & 1 != 0, false));
                    }
                }
            }
        }
    }
    let mut v = Value::zeros(w, false);
    // place LSB-first from the token's tail; bits beyond w truncate (left).
    for (i, &(bv, bu)) in bits.iter().rev().enumerate() {
        if (i as u32) >= w {
            break;
        }
        let word = i / 64;
        let sh = i % 64;
        if bv {
            v.val[word] |= 1u64 << sh;
        }
        if bu {
            v.unk[word] |= 1u64 << sh;
        }
    }
    v.mask_top();
    v
}

/// v7: route `text` to a descriptor — fd form (bit 31) hits one file; MCD
/// form broadcasts to every set channel bit (bit 0 = stdout). A bad/closed
/// fd warns once (W4022) and drops the write, iverilog parity.
pub(crate) fn file_write(sched: &mut Scheduler, fd: u32, text: &str) {
    use std::io::Write as _;
    if fd & 0x8000_0000 != 0 {
        // §21.3.4 pre-opened descriptors. STDOUT (0x8000_0001) routes through
        // the SAME deterministic sink as `$display`/MCD-bit-0, so it interleaves
        // in statement order (iverilog-pinned) inside the golden stdout stream.
        // STDERR (0x8000_0002) goes to the process stderr like iverilog. A write
        // to the read-only STDIN (0x8000_0000) falls through to the files-map
        // miss → W4022 warn + drop (iverilog drops it SILENTLY; the warn is
        // strictly more diagnostic, output bytes identical).
        match fd {
            0x8000_0001 => {
                write_out(sched.st, text);
                return;
            }
            0x8000_0002 => {
                let _ = std::io::stderr().write_all(text.as_bytes());
                return;
            }
            _ => {}
        }
        match sched.st.files.get_mut(&fd) {
            Some(f) => {
                let _ = f.write_all(text.as_bytes());
            }
            None => bad_fd_warn(sched, fd),
        }
        return;
    }
    // MCD broadcast.
    if fd == 0 {
        bad_fd_warn(sched, fd);
        return;
    }
    if fd & 1 != 0 {
        write_out(sched.st, text);
    }
    for bit in 1..31u32 {
        if fd & (1 << bit) != 0 {
            match sched.st.mcd_files.get_mut(&bit) {
                Some(f) => {
                    let _ = f.write_all(text.as_bytes());
                }
                None => bad_fd_warn(sched, fd),
            }
        }
    }
}

/// Read one byte from an fd-form descriptor for the v9 SYS-READ family,
/// honoring the `$ungetc` pushback stack and tracking lazy EOF. Returns
/// `Some(byte)` or `None` at EOF / bad-fd / write-only-fd. Only fd-form
/// descriptors (bit 31 set) opened with read capability (a mode containing
/// 'r' or '+') are readable — MCD channels are write-only broadcast masks, and
/// a plain "w"/"a" descriptor is write-only. A genuinely bad/closed fd warns
/// once (W4022); a valid-but-write-only fd returns `None` WITHOUT a warning and
/// WITHOUT latching EOF (iverilog parity — `$fgetc`=-1 yet `$feof`=0). The lazy
/// EOF flag is set only by a FAILED read on a READABLE fd (a read returning
/// zero bytes), matching iverilog's `$feof` timing.
pub(crate) fn file_read_byte(sched: &mut Scheduler, fd: u32) -> Option<u8> {
    // a pushed-back byte ($ungetc) is served before the underlying stream
    // (LIFO — the top of the pushback stack). Only readable fds ever carry a
    // pushback (k_ungetc rejects write-only/bad fds), so this is safe first.
    if let Some(s) = sched.st.read_state.get_mut(&fd) {
        if let Some(b) = s.pushback.pop() {
            return Some(b);
        }
    }
    // §21.3.4: reads on the pre-opened STDOUT/STDERR behave like any write-only
    // fd — `$fgetc`=-1 with NO warning and NO EOF latch (iverilog-pinned:
    // fgetc=-1, $feof stays 0). STDIN (0x8000_0000) is DELIBERATELY excluded
    // from this early return: reading it is a deferred feature (a stdin-driven
    // sim breaks byte-determinism), so it falls through to the files-map miss
    // → W4022 warn + -1 (iverilog reads stdin).
    if fd == 0x8000_0001 || fd == 0x8000_0002 {
        return None;
    }
    if fd & 0x8000_0000 == 0 || !sched.st.files.contains_key(&fd) {
        bad_fd_warn(sched, fd);
        return None;
    }
    if !sched.st.readable_fds.contains(&fd) {
        // a valid but write-only ("w"/"a") fd: reads fail WITHOUT a warning and
        // WITHOUT latching EOF (iverilog: $fgetc=-1, $feof stays 0).
        return None;
    }
    let file = sched
        .st
        .files
        .get_mut(&fd)
        .expect("readable fd is in files");
    let mut buf = [0u8; 1];
    match std::io::Read::read(file, &mut buf) {
        Ok(1) => Some(buf[0]),
        // EOF (0 bytes) or a read error sets the lazy EOF flag.
        _ => {
            sched.st.read_state.entry(fd).or_default().eof = true;
            None
        }
    }
}
