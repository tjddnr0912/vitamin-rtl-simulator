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
pub(crate) fn scan_next<K: crate::exec::Kernel + ?Sized>(
    k: &mut K,
    fd: Option<u32>,
    src: &[u8],
    pos: &mut usize,
) -> Option<u8> {
    match fd {
        Some(fd) => k.k_file_read_byte(fd),
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
pub(crate) fn scan_unget<K: crate::exec::Kernel + ?Sized>(
    k: &mut K,
    fd: Option<u32>,
    pos: &mut usize,
    b: u8,
) {
    match fd {
        Some(fd) => k.k_file_unget(fd, b),
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

pub(crate) fn scan_write_dst<K: crate::exec::Kernel + ?Sized>(k: &mut K, net: u32, v: Value) {
    let lv = Lvalue {
        chunks: vec![sim_ir::LvalChunk {
            net,
            word: None,
            offset: None,
            width: None,
            kind: sim_ir::SelKind::Bit,
        }],
    };
    let off = k.k_resolve_lvalue_offsets(&lv);
    k.k_write_lvalue(&lv, v, &off);
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
pub(crate) fn scan_run<K: crate::exec::Kernel + ?Sized>(
    k: &mut K,
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
    let at_eof = match scan_next(k, fd, src, &mut pos) {
        Some(b) => {
            scan_unget(k, fd, &mut pos, b);
            false
        }
        None => true,
    };

    macro_rules! next {
        () => {
            scan_next(k, fd, src, &mut pos)
        };
    }

    'fmt: while fi < fmt.len() {
        let fc = fmt[fi];
        if scan_is_ws(fc) {
            fi += 1;
            while let Some(b) = next!() {
                if !scan_is_ws(b) {
                    scan_unget(k, fd, &mut pos, b);
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
                    scan_unget(k, fd, &mut pos, b);
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
                    scan_unget(k, fd, &mut pos, b);
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
            // `%[set]` / `%[^set]` — a SCANSET (C `sscanf`, which IEEE 1800 §21.3.4.2
            // defers to). Neither vita nor iverilog implemented it; iverilog at least
            // REFUSES it loudly ("invalid format code: %["), while vita matched
            // nothing and returned 0 with no diagnostic — which is how an AES vector
            // file's `#keylen=256` header silently fell back to defaults and left the
            // 192/256-bit and decrypt paths untested behind a PASS.
            //
            // Rules, all C's: `^` first negates; a `]` immediately after `[` or `[^`
            // is a literal `]`; `a-z` is a range; a `-` first or last is literal; NO
            // leading-whitespace skip (unlike `%s`); read while the byte is in (or,
            // negated, out of) the set, bounded by an explicit field width; matching
            // ZERO bytes fails the conversion and stops the scan.
            b'[' => {
                let negate = fmt.get(fi) == Some(&b'^');
                if negate {
                    fi += 1;
                }
                let mut set = [false; 256];
                let mut first = true;
                let mut closed = false;
                while fi < fmt.len() {
                    let c = fmt[fi];
                    if c == b']' && !first {
                        fi += 1;
                        closed = true;
                        break;
                    }
                    first = false;
                    // `a-z`: a `-` that is neither first nor last is a range.
                    if fmt.get(fi + 1) == Some(&b'-') && fmt.get(fi + 2).is_some_and(|&e| e != b']')
                    {
                        let end = fmt[fi + 2];
                        let (lo, hi) = if c <= end { (c, end) } else { (end, c) };
                        for b in lo..=hi {
                            set[b as usize] = true;
                        }
                        fi += 3;
                        continue;
                    }
                    set[c as usize] = true;
                    fi += 1;
                }
                if !closed {
                    // An unterminated scanset is a malformed format, not an empty set:
                    // stop rather than consume the rest of the input.
                    break 'fmt;
                }
                let mut bytes = Vec::new();
                while bytes.len() < width {
                    match next!() {
                        Some(b) if set[b as usize] != negate => bytes.push(b),
                        Some(b) => {
                            scan_unget(k, fd, &mut pos, b);
                            break;
                        }
                        None => break,
                    }
                }
                (!bytes.is_empty()).then(|| scan_pack_str(&bytes))
            }
            b's' | b'S' => {
                // skip leading ws, then a ws-delimited run (up to width).
                while let Some(b) = next!() {
                    if !scan_is_ws(b) {
                        scan_unget(k, fd, &mut pos, b);
                        break;
                    }
                }
                let mut bytes = Vec::new();
                while bytes.len() < width {
                    match next!() {
                        Some(b) if scan_is_ws(b) => {
                            scan_unget(k, fd, &mut pos, b);
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
                        scan_unget(k, fd, &mut pos, b);
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
                                scan_unget(k, fd, &mut pos, b);
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
                        .and_then(|&n| k.k_ir().nets.get(n as usize))
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
                        scan_write_dst(k, net, v);
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

/// MULTI-DRIVER groups: nets driven by >=2 continuous assigns that are ALL
/// whole-net (single chunk, no word/offset/width select) and non-delayed. A net
/// with ANY partial/dynamic/array/delayed driver is EXCLUDED — those overlaps
/// stay `E3001` at elaborate, and a per-bit `for (g...) assign y[g] = ...;` is
/// one logical driver that needs only last-write-wins.
///
/// A free function because two callers need the same answer at different times:
/// `Scheduler::new` builds its resolution table from it, and `simulate` asks the
/// tier-3 run gate whether to refuse — BEFORE any scheduler exists. Two
/// spellings would let the gate and the executor disagree about one design.
pub(crate) fn multi_driver_groups(
    ir: &sim_ir::SimIr,
) -> std::collections::BTreeMap<u32, Vec<usize>> {
    let mut whole: std::collections::BTreeMap<u32, Vec<usize>> = std::collections::BTreeMap::new();
    let mut excluded: std::collections::BTreeSet<u32> = std::collections::BTreeSet::new();
    for (ci, ca) in ir.cont_assigns.iter().enumerate() {
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
    whole.retain(|net, cis| cis.len() >= 2 && !excluded.contains(net));
    whole
}

/// 4-state resolution of ONE multi-driven group from its drivers' already-
/// evaluated values: the accumulator starts at all-Z (the identity for all
/// three resolutions — a driver at z yields) and each driver folds in through
/// the kind's table. kind 0=WIRE (equal keeps, conflict→x), 1=WAND, 2=WOR.
///
/// A free function shared by BOTH settle loops (engine and tier-3), extracted
/// under §4.5.302's rule: the identity, the fold order, and the kind dispatch
/// are exactly the semantics two spellings would quietly disagree on, while the
/// halves that touch a store (evaluating each driver's RHS, resolving the LHS
/// offsets, the write) stay with their backend. The fold emits no diagnostics,
/// so evaluating all drivers BEFORE folding — which sharing requires — is
/// observationally the order the engine always had.
pub(crate) fn resolve_md_group(
    kind: u8,
    net_w: u32,
    drivers: impl IntoIterator<Item = Value>,
) -> Value {
    let mut acc = Value::zeros(net_w, false);
    for w in 0..acc.val.len() {
        acc.val[w] = u64::MAX;
        acc.unk[w] = u64::MAX;
    }
    acc.mask_top();
    for v in drivers {
        match kind {
            1 => resolve_wand_into(&mut acc, &v),
            2 => resolve_wor_into(&mut acc, &v),
            _ => resolve_wire_into(&mut acc, &v),
        }
    }
    acc
}

impl<'a, 'ir> Scheduler<'a, 'ir> {
    pub fn new(
        st: &'a mut SimState<'ir>,
        max_deltas: u64,
        max_body_steps: u64,
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
        // ONE SPELLING with the tier-3 run gate, which asks the same question
        // before any scheduler exists. An inline copy lived here and the shared
        // function's own doc claimed it did not — the two were character-
        // identical, so nothing differed, but the invariant was not enforced.
        let whole = multi_driver_groups(st.ir);
        let mut md_nets: Vec<(u32, Vec<usize>, u8)> = Vec::new();
        let mut ca_md = vec![false; nca];
        for (net, cis) in whole {
            {
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
        // The delayed set, decided ONCE. `schedule_delayed_cas` iterates this
        // instead of rescanning every assign per settle; ascending order is the
        // declaration order its old `0..len` loop had, and the `delay.is_some()`
        // test here is the same field the body still reads.
        let delayed_ca_idx: Vec<u32> = st
            .ir
            .cont_assigns
            .iter()
            .enumerate()
            .filter(|(_, ca)| ca.delay.is_some())
            .map(|(i, _)| i as u32)
            .collect();
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
        // DIRTY-SETTLE: split the assigns into "always re-evaluate" and "re-evaluate
        // only when a dependency moved", and build the net → dependents reverse index
        // the write funnel marks through. `ca_deps` certifies skippability; anything it
        // refuses (delayed, impure RHS, heap-handle dependency) lands in `ca_always`, as
        // does every multi-driver member, whose value comes from the resolution below
        // rather than from its own RHS.
        // NATIVE-TYPE GUARD: fill the per-net eligibility bitset BEFORE the `heap`
        // closure takes its immutable borrow of `st` (the fill needs `&mut self` to
        // memoize).
        st.build_plain_scalar();
        let ca_nonint = st.native_ineligible();
        let heap = |net: u32| -> bool {
            let i = net as usize;
            st.dyn_is_handle.get(i).copied().unwrap_or(false)
                || st.class_is_handle.get(i).copied().unwrap_or(false)
        };
        // CONT-ASSIGN NATIVE: compile each RHS once, in the SAME context
        // `eval_for_lvalue` builds (`lvalue_width(lhs).max(self_width(rhs))`, signed from
        // the RHS). `try_compile` returns `None` for anything it cannot lower, so an
        // uncompilable assign simply keeps the interpreter path.
        let ca_native: Vec<Option<crate::native_eval::NativeProg>> = st
            .ir
            .cont_assigns
            .iter()
            .map(|c| {
                // B1 frame-call: an RHS that REACHES a user function call must stay on
                // the interpreter. `is_codegen_able` enforces this for process bodies at
                // the BODY level, not inside `try_compile`, so reusing `try_compile`
                // alone silently dropped the precondition — the frame evaluator runs only
                // on the `&self` interpreter read path (re-entrant frame arena, and the
                // left-to-right operand order static recursion depends on), and routing a
                // call through the native funnel broke 18 tests across package-scoped
                // calls, enum-returning functions and a cont-assign-originated runaway.
                if crate::backend::expr_has_call(&st.ir.exprs, c.rhs) {
                    return None;
                }
                let ctx_w = st.lvalue_width(&c.lhs).max(st.wt.get(c.rhs).width);
                crate::native_eval::try_compile(
                    st.ir,
                    &st.wt,
                    &ca_nonint,
                    c.rhs,
                    ctx_w,
                    st.wt.get(c.rhs).signed,
                )
            })
            .collect();
        // The frame layout `func_read_deps` needs to separate a callee's own locals from
        // the module nets it reads. `func_table` is index-aligned to `ir.funcs`; a
        // length mismatch (a hand-built sidecar in a test fake) makes every call decline
        // rather than mis-attribute a net.
        let windows: Vec<(u32, u32)> = st
            .func_table
            .iter()
            .map(|m| (m.base_net, m.locals_len))
            .collect();
        let deps = crate::levelize::ca_deps(st.ir, &windows, &heap);
        let mut ca_always: Vec<u32> = Vec::new();
        let mut ca_of_net: Vec<Vec<u32>> = vec![Vec::new(); nnets];
        for (ci, (dep, ok)) in deps.iter().enumerate() {
            if !*ok || ca_md[ci] {
                ca_always.push(ci as u32);
                continue;
            }
            for &net in dep {
                if let Some(slot) = ca_of_net.get_mut(net as usize) {
                    slot.push(ci as u32);
                }
            }
        }
        st.ca_of_net = ca_of_net;
        // §2 row 33: procedural read-through of whole-net copies (`crate::alias`).
        let (alias, alias_word) = crate::alias::copy_alias(st.ir, &st.two_state);
        st.wt.install_read_alias(crate::levelize::proc_read_alias(
            st.ir,
            &alias,
            &alias_word,
            &st.task_calls_proc,
            &st.task_calls_func,
        ));
        st.ca_dirty_flag = vec![false; nca];
        // Seed EVERY certified assign: nothing has been evaluated yet, so the first
        // settle must behave exactly like the old full pass.
        st.ca_dirty = (0..nca as u32).collect();
        for &ci in &st.ca_dirty {
            st.ca_dirty_flag[ci as usize] = true;
        }
        Scheduler {
            ca_native,
            ca_always,
            st,
            cur: SlotQueues::default(),
            nba: Vec::new(),
            nba_scratch_lhs: Lvalue {
                chunks: Vec::with_capacity(1),
            },
            nba_seq: 0,
            native_scratch: std::cell::RefCell::new(Default::default()),
            #[cfg(feature = "jit")]
            jit: std::cell::RefCell::new(
                std::env::var_os("VITA_JIT").and_then(|_| crate::jit::JitEngine::new()),
            ),
            #[cfg(feature = "jit")]
            jit_bodies: std::cell::RefCell::new(Default::default()),
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
            delayed_ca_idx,
            delayed_ca: BTreeMap::new(),
            delayed_nba: BTreeMap::new(),
            delta_count: 0,
            max_deltas,
            max_body_steps,
            time_limit,
            scratch_changed: Vec::new(),
            scratch_edges: Vec::new(),
            scratch_edge_seen: Vec::new(),
            scratch_edge_marked: Vec::new(),
            scratch_expr_now: Vec::new(),
            scratch_level_fire: Vec::new(),
            #[cfg(feature = "oracle")]
            vm_regs_pool: Vec::new(),
            #[cfg(feature = "oracle")]
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
        // ── R14 (ROADMAP §3 ⑭): the settle-loop counter, and WHERE ITS COST IS,
        // measured rather than argued ──
        //
        // This loop runs once per continuous assign per delta — 12.8M times on
        // a 64-stage synthetic — so it is where a profile flag could plausibly
        // tax a run that did not ask for one. It does: PRE vs POST on that
        // synthetic is ~+1.3%.
        //
        // ⚠️ TWO PLAUSIBLE CAUSES WERE WRONG. Hoisting the `proc_prof` read out
        // of the fixpoint (it was two pointer hops per visit) moved nothing:
        // +1.45% before, +1.45% after. Collapsing a profiled/unprofiled ARM
        // SPLIT — each arm carried its own call to the evaluator, which inlines,
        // so the split doubled the loop body — moved nothing either. What DID
        // move it was compiling the counters out entirely (`profiling` forced to
        // a literal `false`): **+0.11%**. So the residue is the per-visit test
        // itself, and the only way to remove it is a settle pass monomorphised
        // over `const PROF: bool` = a second copy of this loop. Not taken.
        //
        // The shape below is kept because it is the SMALLER one (one call site,
        // charge after), not because it was faster. Both flags are hoisted
        // because the profile cannot be switched on mid-run, so one read for the
        // whole fixpoint is exactly equivalent — that part is free either way.
        let profiling = self.st.proc_prof.is_some();
        let prof_timed = self.st.proc_prof.as_ref().is_some_and(|p| p.timed);
        loop {
            let mut changed = false;
            // DIRTY-SETTLE: visit the assigns that must be re-evaluated, not all of
            // them. `ca_always` holds every assign `levelize::ca_deps` refused to
            // certify (delayed, multi-driver member, impure RHS, heap-handle
            // dependency); `st.ca_dirty` holds the certified ones whose dependency
            // nets moved since the last pass. The union is visited in ASCENDING index
            // = declaration order, which is the order the fixpoint has always used and
            // which several goldens depend on.
            //
            // Skipping is sound precisely because a certified assign whose inputs did
            // not move recomputes its previous value, and the write funnel drops a
            // same-value write without noting a change — so the visit it replaces was
            // observationally a no-op. The teeth are in `ca_deps` being COMPLETE.
            let pass: Vec<u32> = {
                let mut v = std::mem::take(&mut self.st.ca_dirty);
                for &ci in &v {
                    self.st.ca_dirty_flag[ci as usize] = false;
                }
                v.extend_from_slice(&self.ca_always);
                v.sort_unstable();
                v.dedup();
                v
            };
            for ci in pass.into_iter().map(|c| c as usize) {
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
                let ca_t0 = prof_timed.then(std::time::Instant::now);
                let v = self.eval_cont_assign(ci, &lhs, ca_rhs); // CONTEXT-SIZED to lhs width
                if profiling {
                    self.charge_ca(ci, ca_t0);
                }
                let offs = self.resolve_lvalue_offsets(&lhs); // dynamic index NOW (settle time)
                changed |= self.st.write_lvalue(&lhs, v, &offs);
            }
            // MULTI-DRIVER: resolve each multi-driven net from ALL its whole-net
            // drivers by 4-state wire resolution, then write the net once. Part of
            // the same fixpoint (a driver's RHS can depend on another resolved net).
            for mi in 0..self.md_nets.len() {
                let net = self.md_nets[mi].0;
                let net_w = self.st.nets[net as usize].width;
                let cis = self.md_nets[mi].1.clone();
                let kind = self.md_nets[mi].2;
                // Evaluate every driver first, fold second — the fold is the
                // shared `resolve_md_group` and emits nothing, so the
                // diagnostic stream is the interleaved loop's.
                let mut vals = Vec::with_capacity(cis.len());
                for ci in cis {
                    let ca_rhs = self.st.ir.cont_assigns[ci].rhs;
                    let lhs = self.st.ir.cont_assigns[ci].lhs.clone();
                    let ca_t0 = prof_timed.then(std::time::Instant::now);
                    vals.push(self.eval_cont_assign(ci, &lhs, ca_rhs));
                    if profiling {
                        self.charge_ca(ci, ca_t0);
                    }
                }
                let acc = resolve_md_group(kind, net_w, vals);
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
        self.schedule_delayed_cas::<SimState>(None);
        Some(any)
    }

    /// Delayed `assign #d y = rhs`: the zero-delay fixpoint has settled, so the
    /// RHS is stable. On each RHS-value CHANGE, schedule an INERTIAL write of
    /// the new value at `now + d` — bumping `ca_gen[ci]` cancels any still
    /// pending older write for THIS assign (a pulse narrower than d never
    /// lands; a pulse of EXACTLY d survives because pending writes apply at the
    /// tick start, before processes re-change the RHS — both iverilog-pinned
    /// live, 2026-06-12).
    ///
    /// `nets` is the tier-3 seam (S1d-4d-3): `None` reads this scheduler's own
    /// store, `Some(arena)` reads the alternate one. Only the RHS EVALUATION
    /// crosses — the generation bookkeeping, the transition-delay selection and
    /// the wheel are scheduler state either way, which is why this is one
    /// function rather than two.
    pub(crate) fn schedule_delayed_cas<N: crate::eval::NetReader + ?Sized>(
        &mut self,
        nets: Option<&N>,
    ) {
        // The iteration set is `delayed_ca_idx`, not `0..cont_assigns.len()` —
        // the same indices in the same order, minus the ones whose only effect
        // was to reach the `continue` below. The `let Some(d) = ... else` is
        // KEPT verbatim rather than replaced by an `unwrap`: the delay is still
        // read from the one field that defines it, so the pre-filter cannot
        // become a second spelling of the rule. (`self.st.ir` is a `&SimIr`
        // fixed for this scheduler's life, so the filter cannot go stale.)
        for i in 0..self.delayed_ca_idx.len() {
            let ci = self.delayed_ca_idx[i] as usize;
            let Some(d) = self.st.ir.cont_assigns[ci].delay else {
                continue;
            };
            let ca_rhs = self.st.ir.cont_assigns[ci].rhs;
            let lhs = self.st.ir.cont_assigns[ci].lhs.clone();
            // R14: charged around the WHOLE match and not inside the `None`
            // arm, because tier-3 takes the `Some(arena)` arm — a counter on one
            // arm would make `ca_evals` for a DELAYED assign depend on
            // `--backend`, and the point of the profile is comparable runs.
            // (This loop visits only DELAYED assigns, which are rare, so it
            // reads `proc_prof` directly rather than hoisting.)
            let ca_t0 = match &self.st.proc_prof {
                Some(p) if p.timed => Some(std::time::Instant::now()),
                _ => None,
            };
            let v = match nets {
                // Same assignment rule as `eval_for_lvalue`: width is
                // max(lhs, self(rhs)), sign is the rhs's own. `lvalue_width`
                // reads IR-derived widths, identical for both stores.
                Some(r) => {
                    let lw = self.st.lvalue_width(&lhs);
                    let sw = self.st.wt.get(ca_rhs);
                    self.st
                        .mk_eval_ctx_with(r)
                        .eval_ctx(ca_rhs, lw.max(sw.width), sw.signed)
                }
                None => self.eval_cont_assign(ci, &lhs, ca_rhs),
            };
            self.charge_ca(ci, ca_t0);
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
            // THE LHS OFFSETS MUST COME FROM THE SAME STORE AS THE RHS. This
            // read `self.resolve_lvalue_offsets` unconditionally — the engine's
            // own `EvalCtx` — while the value above already crossed the seam.
            // On a native run `sched.st` is never written, so a dynamic index
            // (`assign #1 y[i] = v;`) read X, became the out-of-range sentinel
            // and the write was DROPPED: same exit code, same everything, one
            // bit silently missing. Two halves of one feature reading two
            // stores — and the X-drive half in `native/run.rs` was already
            // using the arena, which is what made the survivor visible as an
            // `x` at the right bit position.
            let offs = match nets {
                Some(r) => crate::eval::resolve_offsets(&self.st.mk_eval_ctx_with(r), &lhs),
                None => self.resolve_lvalue_offsets(&lhs),
            };
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
    }

    /// The delayed cont-assign writes due at `tick`, generation-filtered — the
    /// inertial cancel. The CALLER performs the write, because the two backends
    /// write different stores; everything about WHICH writes survive is here.
    ///
    /// `last_ca_drv` is updated for each surviving write: it is the baseline the
    /// next transition's per-bit delay is measured from, and a superseded write
    /// must not move it (which is why the filter and the update are one step).
    pub(crate) fn take_due_delayed_ca(&mut self, tick: u64) -> Vec<(Lvalue, Value, Offsets)> {
        let mut out = Vec::new();
        if let Some(writes) = self.delayed_ca.remove(&tick) {
            for (ci, gen, lhs, v, offs) in writes {
                if self.ca_gen[ci as usize] != gen {
                    continue;
                }
                self.last_ca_drv[ci as usize] = Some(v.clone());
                out.push((lhs, v, offs));
            }
        }
        out
    }

    /// The earliest tick with a pending delayed cont-assign write, if any.
    pub(crate) fn next_delayed_ca(&self) -> Option<u64> {
        self.delayed_ca.keys().next().copied()
    }

    /// Does this delayed cont-assign still owe its initial X drive?
    ///
    /// `assign #3 o = a & b` reads `o == x` during `[0, d)` — iverilog-pinned —
    /// and that x has to propagate through the fixpoint, so it is driven inside
    /// the settle rather than at the wheel. Only while the first real write has
    /// not landed, and only for a SOLE driver (a shared net's every-delta
    /// x-drive would oscillate against the concurrent driver).
    pub(crate) fn delayed_owes_initial_x(&self, ci: usize) -> bool {
        self.last_ca_drv[ci].is_none() && self.delayed_sole[ci]
    }

    /// Arm processes at t0 per Verilog initial/always semantics.
    pub fn arm_processes(&mut self) {
        // Pre-seed top-level activities 1:1 with process declarations. `tie ==
        // template == declaration index` so existing single-process ordering is
        // byte-identical to before the activity-id refactor.
        //
        self.free_activities.clear();
        self.free_barriers.clear();
        self.seed_base_activities();
        self.arm_processes_after_seed();
    }

    /// The base activities, 1:1 with process declarations — `tie == template ==
    /// declaration index`, which is what keeps single-process ordering identical
    /// to before the activity-id refactor.
    ///
    /// A4 split this out of `arm_processes`: tier-3 has its own arming
    /// (`native::run::arm_t0`) and does not call that function, but a fork needs
    /// the same arena to hang children off. One spelling, because "which
    /// activity is which process" is the assumption both executors' ready
    /// ordering rests on.
    pub(crate) fn seed_base_activities(&mut self) {
        self.activities = (0..self.st.ir.processes.len() as u32)
            .map(|pi| Activity {
                call_stack: Vec::new(),
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
    }

    /// The rest of t0 arming, after the activity arena exists.
    pub(crate) fn arm_processes_after_seed(&mut self) {
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

        // §4.5.256: STATIC INITIALIZATION, before anything is armed. IEEE 1800 §6.21 puts
        // a declaration initializer "before any initial or always block starts", and
        // measurement says that is literal — iverilog gives `always @clk` no edge for
        // `reg clk = 0;`, and none for a non-constant `int nc = src + 1;` either. Running
        // these as ordinary t0 processes produced both edges, and running them in the
        // right ORDER (the previous half of this slice) does not help: the arming has
        // already happened. They are executed here, in initialization order, and then
        // skipped by the arming loop exactly like a `final` block.
        //
        // A synthesized initializer body is straight-line — blocking assignments plus the
        // queue / dyn-array `'{…}` expansions — so it cannot suspend, and `run_body` runs
        // it to completion the same way `run_finals` does.
        let inits = std::mem::take(&mut self.st.init_procs);
        // Only what the INIT PHASE itself made dirty is un-dirtied below, so the mark is
        // taken first. `settle_cont_assigns` already ran (lib.rs, before arming) and its
        // t0 writes are on this same list; clearing the list wholesale threw those away
        // too, and they are not recoverable — the settle inside `run()` writes the same
        // value, and `note_change` only records an ACTUAL change, so an
        // `always @(w)` on `assign w = 1'b1;` simply never fired. Design-wide, since one
        // unrelated `reg r = 1'b0;` anywhere is enough to enter this branch.
        let settled = self.st.dirty.len();
        for pid in &inits {
            // An out-of-range ProcId means a truncated / mismatched sidecar, exactly like
            // the fork-mode gate above — silently skipping it would drop a design's
            // initializers with no diagnostic. The IR is unusable either way, so say so.
            if (*pid as usize) >= self.activities.len() {
                self.fatal_init_proc_missing(*pid);
                return;
            }
            let entry = self.st.ir.processes[*pid as usize].entry;
            let _ = self.run_body(*pid, entry);
        }
        // COPY-NET REPAIR — the tier-3 twin is `native::run::arm_t0`, and the
        // whole argument lives in `crate::alias`. A net whose every continuous
        // driver MOVES bits rather than computing them has no state of its own,
        // but the settle above evaluated those drivers while their sources still
        // held their declared defaults, so the copies are redone here
        // (initializers have landed) and then handed their sources' event status.
        // Without it the run loop's first delta does the copy, and THAT move is
        // the transition the source never made (ROADMAP §2-N).
        //
        // The writes go in before the rollback so they are dropped with the
        // initializers'; the suppression goes after it, so each source's dirt is
        // the settle's answer alone. `copy_nets` is in dependency order, so one
        // pass repairs a chain and every source's flag is final when its reader
        // asks.
        let copies = crate::alias::copy_nets(self.st.ir);
        for cn in &copies {
            for &ci in &cn.cas {
                let lhs = self.st.ir.cont_assigns[ci].lhs.clone();
                let ca_rhs = self.st.ir.cont_assigns[ci].rhs;
                let v = self.eval_cont_assign(ci, &lhs, ca_rhs);
                let offs = self.resolve_lvalue_offsets(&lhs);
                self.st.write_lvalue(&lhs, v, &offs);
            }
        }

        // …and drop what those writes made dirty. "Before any process is armed" means the
        // initialization is not a transition anyone can observe: `reg clk = 0;` must not
        // hand `always @clk` an X→0 edge, and `int nc = src + 1;` must not hand one to
        // `always @nc` either (both measured against iverilog, both wrong before). The
        // t0 continuous-assign settle re-evaluates every assign from scratch rather than
        // from this list, so clearing it costs nothing there.
        for n in self.st.dirty.split_off(settled) {
            self.st.dirty_flag[n as usize] = false;
        }
        // SUPPRESSION ONLY, and TRANSITIVE — the tier-3 twin says why: a copy net
        // cannot carry an event nothing in its source CHAIN had, but it CAN
        // legitimately stay put while a source moves, because the two nets have
        // their own storage defaults. `moved` forwards a source's movement past a
        // copy whose own default masked it.
        let mut moved: std::collections::BTreeMap<u32, bool> = std::collections::BTreeMap::new();
        let mut dropped_any = false;
        for cn in &copies {
            let d = cn.dst as usize;
            let m = cn.srcs.iter().any(|&s| {
                self.st.dirty_flag[s as usize] || moved.get(&s).copied().unwrap_or(false)
            });
            moved.insert(cn.dst, m);
            if !m && self.st.dirty_flag[d] {
                self.st.dirty_flag[d] = false;
                dropped_any = true;
            }
        }
        if dropped_any {
            // `dirty` is a list with a membership flag beside it, so a cleared
            // flag has to be paid for by compacting the list — `propagate_changes`
            // reads the LIST, not the flags.
            let mut v = std::mem::take(&mut self.st.dirty);
            v.retain(|n| self.st.dirty_flag[*n as usize]);
            self.st.dirty = v;
        }
        let init_set: std::collections::BTreeSet<u32> = inits.iter().copied().collect();

        for aid in 0..self.activities.len() as u32 {
            let tmpl = self.activities[aid as usize].template as usize;
            // P2-E: `final` blocks are Initial-shaped in the IR but never
            // armed — `run_finals` executes them after the main loop ends.
            if self.st.final_procs.contains(&(tmpl as u32)) {
                continue;
            }
            // …and neither is a declaration-initializer body: it already ran above.
            if init_set.contains(&(tmpl as u32)) {
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
    /// Does a LIVE static-level waiter exist for `pi`? The engine-side twin of
    /// `native::wake::WakeTable::level_armed`, so the tier-3 differential can
    /// compare re-arming against the engine instead of against a hard-coded
    /// expectation. `arm = None` is what makes a waiter STATIC sensitivity.
    #[cfg(test)]
    pub(crate) fn has_static_level_waiter(&self, pi: u32) -> bool {
        self.waiters.iter().any(|w| {
            w.ready.proc == pi
                && w.arm.is_none()
                && matches!(w.cause, crate::sched::WaitCause::Level { .. })
        })
    }

    /// How many EDGE registrations does `pi` hold? The other half of the arm
    /// state, and the differential was one-sided without it: `has_static_level_
    /// waiter` inspects only `WaitCause::Level`, so a `Scheduler::rearm` that
    /// wrongly re-registered an Edge landed in `net_to_edge` where no probe
    /// looked — the exact 2^k bug the tier-3 restatement exists to mirror, and it
    /// passed. Measured.
    #[cfg(test)]
    pub(crate) fn edge_registration_count(&self, pi: u32) -> usize {
        self.net_to_edge
            .iter()
            .flat_map(|v| v.iter())
            .filter(|(_, r)| r.proc == pi)
            .count()
    }

    /// The continuous assigns `levelize::ca_deps` refused to certify — the ones a
    /// settle pass must visit unconditionally, whatever the dirty worklist says.
    /// Read by the tier-3 settle so the two visit the same set.
    pub(crate) fn ca_always(&self) -> &[u32] {
        &self.ca_always
    }

    /// The multi-driven groups `(net, driver cont-assign indices, kind)` this
    /// scheduler resolves. Read by the tier-3 settle so both loops resolve the
    /// SAME groups with the same member order — the table is built once in
    /// `Scheduler::new` from `multi_driver_groups` plus the wired-kind sidecars,
    /// and a second derivation could classify a design differently.
    pub(crate) fn md_groups(&self) -> &[(u32, Vec<usize>, u8)] {
        &self.md_nets
    }

    /// Whether cont-assign `ci` is a member of a multi-driven group — such a
    /// driver is written ONCE by the group resolution, never individually.
    /// Read by the tier-3 settle's per-driver loop for the same skip the
    /// engine's loop takes.
    pub(crate) fn ca_is_md(&self, ci: usize) -> bool {
        self.ca_md[ci]
    }

    /// Every pending activation, as `(tick, inactive, proc, block)`.
    ///
    /// The engine-side twin of `NativeKernel::pending_resumes_for_test`, so a
    /// `Terminator::Delay` can be compared for WHERE it filed the resume rather
    /// than only for what it wrote — a suspension writes nothing, so the store
    /// comparison the body differential already makes is blind to it entirely.
    /// Current-time buckets report `now` as their tick.
    #[cfg(test)]
    pub(crate) fn pending_resumes_for_test(&self) -> Vec<(u64, bool, u32, u32)> {
        let mut v: Vec<(u64, bool, u32, u32)> = Vec::new();
        for r in &self.cur.active {
            v.push((self.st.now, false, r.proc, r.block));
        }
        for r in &self.cur.inactive {
            v.push((self.st.now, true, r.proc, r.block));
        }
        for (&t, evs) in &self.wheel {
            for (region, r) in evs {
                v.push((t, matches!(region, RegionTag::Inactive), r.proc, r.block));
            }
        }
        v
    }

    /// Drop `pi`'s live static-level waiter, as a fire does. Test-only: the
    /// differential needs the same consume-then-re-arm sequence on both sides.
    #[cfg(test)]
    pub(crate) fn consume_static_level_waiter_for_test(&mut self, pi: u32) {
        let before = self.waiters.len();
        self.waiters.retain(|w| {
            !(w.ready.proc == pi
                && w.arm.is_none()
                && matches!(w.cause, crate::sched::WaitCause::Level { .. }))
        });
        self.n_level_waiters -= before - self.waiters.len();
    }

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
        self.set_cur_activity(proc);
        // ── R14 (ROADMAP §3 ⑭): per-process activation profile ──
        // ONE read of `proc_prof`, ONE test: the template lookup and the clock
        // read live inside the `Some` arm, so a run without the flag pays for
        // neither. This is the simulator's per-activation seam, and on a design
        // whose bodies are one statement each the seam IS the run — but the
        // measured tax lives in the SETTLE loop, not here (see
        // `settle_cont_assigns`: forcing that loop's flag to a literal `false`
        // took a 64-stage synthetic from +1.3% to +0.11% with this counter still
        // in place).
        //
        // Two `map`s and not one closure: the second one needs `&self` for
        // `activity_template`, which cannot be borrowed inside a closure already
        // holding `&self.st.proc_prof`. The first `map` copies the `bool` out to
        // end that borrow.
        let prof = self.st.proc_prof.as_ref().map(|p| p.timed).map(|timed| {
            (
                self.activity_template(proc) as usize,
                timed.then(std::time::Instant::now),
            )
        });
        // SELF-RETRIG: tag blocking writes made by THIS body to their author, so
        // it is not re-triggered by its own write. Cleared on return — NBA apply,
        // cont-assign settle and clocking commit (all outside `run_body`) then
        // author their writes as `None` (= re-fire normally).
        self.st.blocking_writer = Some(proc);
        let step = match self.st.backend {
            // `Native` cannot arrive here, and since S1d-4c-2c the reason is
            // different from what this comment used to say. It is no longer
            // "there is no native executor": there is, and `simulate` drives it
            // through `native::run::run`, which never calls `Scheduler::run_body`.
            //
            // ⚠️ And the old promise — "the REFERENCE semantics are the safe
            // default, never a panic" — is NOT true any more, so do not lean on
            // it. On a native run `Scheduler::new` is constructed but
            // `arm_processes` is never called, so `self.activities` is EMPTY and
            // the line above (`self.activities[proc].gen`) would panic before
            // this match is reached. Any new `Scheduler` call site on the native
            // path inherits that; the arm is kept because the interpreter and
            // the VM still need it.
            // ⚠️ B2': the `Native` arm is here for the reason the note above
            // gives — it is unreachable, because `simulate` drives tier-3 through
            // `native::run::run` and never calls this. It stays so the match is
            // total in BOTH builds; with `oracle` off it is the only arm left.
            crate::Backend::Native => run_process(self, proc, block),
            #[cfg(feature = "oracle")]
            crate::Backend::Interpreter => run_process(self, proc, block),
            #[cfg(feature = "oracle")]
            crate::Backend::Bytecode => {
                let tmpl = self.activity_template(proc) as usize;
                match self.st.vm_compiled(tmpl) {
                    Some(body) => self.vm_run_body(proc, tmpl, block, body),
                    None => run_process(self, proc, block),
                }
            }
        };
        self.st.blocking_writer = None;
        // R14: charge the activation AFTER the body ran, so `nanos` covers the
        // executor that actually ran it (interpreter, VM or JIT — `run_body` is
        // the seam all three arms leave through).
        if let Some((tmpl, t0)) = prof {
            let ns = t0.map_or(0, |t0| t0.elapsed().as_nanos() as u64);
            if let Some(p) = self.st.proc_prof.as_mut() {
                p.bump_proc(tmpl, ns);
            }
        }
        step
    }

    /// Get-or-compile the machine code for a process TEMPLATE.
    ///
    /// One spelling for two callers. Tier-2 reaches it from `vm_run_body` and
    /// tier-3 from `native::run::dispatch_body`; a `None` entry means "tried and
    /// refused" and must be remembered, which is the part a second copy of this
    /// lookup would be most likely to drop (ROADMAP §5.1-be).
    ///
    /// The cache is keyed by template and not by kernel because a `BodyFn` is a
    /// function of the `CompiledBody` alone — everything store-dependent that it
    /// does leaves through a shim taking the `&mut dyn Kernel` the CALLER
    /// supplies.
    #[cfg(feature = "jit")]
    pub(crate) fn jit_body_for(
        &self,
        tmpl: usize,
        body: &crate::backend::CompiledBody,
    ) -> Option<crate::jit::BodyFn> {
        self.jit.borrow().as_ref()?;
        if let Some(f) = self.jit_bodies.borrow().get(&tmpl) {
            return *f;
        }
        let f = self
            .jit
            .borrow_mut()
            .as_mut()
            .and_then(|e| e.compile_body(body));
        self.jit_bodies.borrow_mut().insert(tmpl, f);
        f
    }

    /// Bytecode-VM body entry (Stage C / C2). The P9 predicate (via `vm_compiled`) has
    /// confirmed this body is suspend-free; `body` is its compiled form, handed in as an
    /// owned `Rc` so this `&mut self` kernel call cannot alias the cache (§2.3).
    ///
    /// The VM bypasses `run_process`, which is where the per-body prologue lives, so it
    /// calls the SAME `exec::enter_body` before `vm_exec` evaluates anything. It used to
    /// carry a hand-copied excerpt that set `cur_time_mult` only; `cur_prec_mult` and the
    /// `%m` scope were missing, so a submodule `$display("%m")` printed whatever scope
    /// another process had left behind. The per-activation termination guard lives inside
    /// `vm_exec` (mirror of exec.rs:176-180).
    #[cfg(feature = "oracle")]
    pub(crate) fn vm_run_body(
        &mut self,
        proc: u32,
        tmpl: usize,
        block: u32,
        body: Rc<crate::backend::CompiledBody>,
    ) -> Step {
        self.k_enter_body(tmpl as u32);
        #[cfg(feature = "jit")]
        {
            // BODY-LEVEL CODEGEN: one boundary crossing per activation instead of one per
            // expression. Compiled once per TEMPLATE (a `None` entry means "tried and
            // refused" and must be remembered).
            {
                let f = self.jit_body_for(tmpl, &body);
                if let Some(f) = f {
                    let b = std::rc::Rc::clone(&body);
                    return crate::jit::run_body_jit(f, self, &b, proc);
                }
            }
        }
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
