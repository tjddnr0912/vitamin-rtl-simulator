//! split part of `sched` (mechanical move).

use super::*;

/// v9 `$fread` byte-into-register fill (iverilog-pinned). The `bytes` read from
/// the file (the first is most significant) fill the `w`-bit register's byte
/// slots from the MSB end: byte `i` → slot `capacity-1-i` (slot `k` = bits
/// `[k*8 .. min((k+1)*8, w))`, so the top slot is partial when `w` is not a
/// multiple of 8). Slots left unwritten (a partial fill, when fewer than
/// `capacity` bytes were read) KEEP their prior value, so `prior` is the
/// register's current value. A full read overwrites every slot.
pub(crate) fn fill_reg_slots(prior: &Value, w: u32, bytes: &[u8]) -> Value {
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
pub(crate) fn scan_is_ws(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}

/// Next source byte. For `$fscanf` (fd = Some) it comes from the file stream
/// (honoring the `$ungetc`/scanf pushback); for `$sscanf` (fd = None) from the
/// `src` buffer at `*pos` (advanced). Returns None at end of input.
pub(crate) fn scan_next(
    sched: &mut Scheduler,
    fd: Option<u32>,
    src: &[u8],
    pos: &mut usize,
) -> Option<u8> {
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
pub(crate) fn scan_unget(sched: &mut Scheduler, fd: Option<u32>, pos: &mut usize, b: u8) {
    match fd {
        Some(fd) => sched.st.read_state.entry(fd).or_default().pushback.push(b),
        None => *pos = pos.saturating_sub(1),
    }
}

/// Pack bytes MSB-first (first byte = most significant) into a Value of width
/// 8×len — the §5.9 string packing the dest write funnel then resizes.
pub(crate) fn scan_pack_str(bytes: &[u8]) -> Value {
    let w = (bytes.len() as u32 * 8).max(8);
    let mut v = Value::zeros(w, false);
    for (i, &by) in bytes.iter().rev().enumerate() {
        let bit = i * 8;
        v.val[bit / 64] |= (by as u64) << (bit % 64);
    }
    v
}

pub(crate) fn scan_write_dst(sched: &mut Scheduler, net: u32, v: Value) {
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
pub(crate) fn scan_build_numeric(chars: &[u8], radix: u32, dst_w: u32) -> Value {
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
pub(crate) fn scan_run(
    sched: &mut Scheduler,
    fd: Option<u32>,
    src: &[u8],
    fmt: &[u8],
    dsts: &[u32],
) -> Value {
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
        // Total cont-assign drivers per base net (ANY delay / shape — one count
        // per distinct net a cont-assign's lhs touches). A delayed driver only
        // earns the initial-X drive when it is the SOLE driver of every net it
        // touches; else the every-delta X-drive could fight a concurrent driver.
        let mut net_drivers: std::collections::BTreeMap<u32, u32> =
            std::collections::BTreeMap::new();
        for ca in &st.ir.cont_assigns {
            let mut nets_seen: std::collections::BTreeSet<u32> = std::collections::BTreeSet::new();
            for c in &ca.lhs.chunks {
                nets_seen.insert(c.net);
            }
            for net in nets_seen {
                *net_drivers.entry(net).or_default() += 1;
            }
        }
        let delayed_sole: Vec<bool> = st
            .ir
            .cont_assigns
            .iter()
            .map(|ca| {
                ca.delay.is_some()
                    && ca
                        .lhs
                        .chunks
                        .iter()
                        .all(|c| net_drivers.get(&c.net) == Some(&1))
            })
            .collect();
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
            delayed_sole,
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
                    // A delayed driver's output register holds x until its FIRST
                    // delayed write lands — iverilog-pinned: `assign #3 o = a&b`
                    // and `and #3 (o,a,b)` read `o == x` (NOT the undriven-z net
                    // default) during [0, d). Drive that initial x INSIDE the
                    // fixpoint (not just below) so it propagates to downstream
                    // cont-assigns; the computed value is scheduled at now+d below.
                    // Skipped once the first write has landed (`last_ca_drv` Some),
                    // and unless this delayed driver is the SOLE driver of its net
                    // (`delayed_sole`) — a shared net's every-delta x-drive would
                    // oscillate against the concurrent driver (see the field doc).
                    if self.last_ca_drv[ci].is_none() && self.delayed_sole[ci] {
                        let lhs = self.st.ir.cont_assigns[ci].lhs.clone();
                        let ca_rhs = self.st.ir.cont_assigns[ci].rhs;
                        let w = self.eval_for_lvalue(&lhs, ca_rhs).width;
                        let offs = self.resolve_lvalue_offsets(&lhs);
                        changed |= self.st.write_lvalue(&lhs, Value::xs(w, false), &offs);
                    }
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
        //
        // §4.5.256: unless the design carries a `proc_ties` permutation, which reorders
        // the t0 queue WITHOUT touching ProcIds. Static initialization runs "before any
        // initial or always block starts" (IEEE 1800 §6.21) and, among the initializers,
        // in a scope order the elaboration pass structure cannot produce — a child
        // instance's initializers precede its parent's, while the parent's own processes
        // are created before the child even exists. `tie` is the one thing the t0 queue
        // sorts on, so expressing the order there needs nothing else: `template` still
        // indexes the process, only the RUN order changes. EMPTY ⇒ identity ⇒ unchanged.
        self.free_activities.clear();
        self.free_barriers.clear();
        let ties = std::mem::take(&mut self.st.proc_ties);
        self.activities = (0..self.st.ir.processes.len() as u32)
            .map(|pi| Activity {
                call_stack: Vec::new(),
                template: pi,
                tie: ties.get(pi as usize).copied().unwrap_or(pi),
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
    pub(crate) fn arm_sensitivity(&mut self, pi: u32) {
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
    pub(crate) fn run_body(&mut self, proc: u32, block: u32) -> Step {
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
    pub(crate) fn vm_run_body(
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
}
