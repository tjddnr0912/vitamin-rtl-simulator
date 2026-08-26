//! split part of `builtins` (mechanical move).

use super::*;

/// %h/%o/%b: group bits per digit (1=bin,3=oct,4=hex), MSB-first; a group with
/// any X → 'x', any Z (no X) → 'z'.
pub(crate) fn fmt_radix(
    v: &Value,
    bits_per_digit: u32,
    min_zero: bool,
    field_width: Option<usize>,
    left_just: bool,
) -> String {
    if v.width == 0 {
        return "0".to_string();
    }
    let ndig = v.width.div_ceil(bits_per_digit);
    let mut s = String::new();
    for d in (0..ndig).rev() {
        let base = d * bits_per_digit;
        let mut val = 0u32;
        let mut has_x = false;
        let mut has_z = false;
        let mut has_known = false;
        for k in 0..bits_per_digit {
            let bi = base + k;
            if bi >= v.width {
                continue;
            }
            let (b, u) = v.get_vu(bi);
            match (b, u) {
                (_, 0) => {
                    has_known = true;
                    if b == 1 {
                        val |= 1 << k;
                    }
                }
                (0, 1) => has_x = true,
                (1, 1) => has_z = true,
                _ => {}
            }
        }
        // §21.2.1.2 per-digit: lowercase x/z only when the digit is UNIFORM (no
        // known bit AND a single unknown kind); any mixing (known+unknown, or x+z)
        // is uppercase X/Z. x takes precedence over z.
        s.push(if has_x || has_z {
            let uppercase = has_known || (has_x && has_z);
            match (uppercase, has_x) {
                (false, true) => 'x',
                (false, false) => 'z',
                (true, true) => 'X',
                (true, false) => 'Z',
            }
        } else {
            std::char::from_digit(val, 16).unwrap()
        });
    }
    // Width/flag handling (iverilog-pinned):
    //   `%h`   → full vector width (leading zeros retained)
    //   `%0h`  → minimum width: strip leading zeros (keep ≥1 digit)
    //   `%0Nh` → zero-pad to N digits
    //   `%Nh`  → space-pad to N digits (over the full-width form)
    let base = if min_zero && field_width == Some(0) {
        let trimmed = s.trim_start_matches('0');
        if trimmed.is_empty() {
            "0".to_string()
        } else {
            trimmed.to_string()
        }
    } else {
        s
    };
    match field_width {
        Some(w) if base.len() < w => {
            let n = w - base.len();
            if left_just {
                // `-` right-pads with spaces (overrides `0`); the natural
                // zero-padded digit string is the content (e.g. 8'hA → "0a").
                format!("{base}{}", " ".repeat(n))
            } else {
                let pad = if min_zero { '0' } else { ' ' };
                let mut p: String = std::iter::repeat(pad).take(n).collect();
                p.push_str(&base);
                p
            }
        }
        _ => base,
    }
}

pub(crate) fn char_of(v: &Value) -> char {
    // IEEE %c: the LOW 8 bits regardless of value width — a wide value with
    // high bits set must not degrade to NUL under the strict no-truncation
    // `to_u64`. X/Z keeps the old None→0 policy.
    if v.has_xz() {
        return '\0';
    }
    let byte = (v.val.first().copied().unwrap_or(0) & 0xFF) as u8;
    byte as char
}

/// v5 (C): resolve a dyn-method HANDLE argument (the ExprId of the handle's
/// whole-net `Signal`) to its NetId. Anything else → None (defensive no-op).
pub(crate) fn dyn_handle_net(sched: &Scheduler, arg: Option<&u32>) -> Option<u32> {
    let &eid = arg?;
    match sched.st.ir.exprs.get(eid as usize) {
        Some(sim_ir::Expr::Signal { net, word: None }) => Some(*net),
        _ => None,
    }
}

/// V34-4: the element COUNT of a fixed-size unpacked array net, or `None` when
/// the net is one of the heap kinds (which own their own storage) or is not an
/// array at all (`array_len == 0` is exactly how a handle net is spelled).
///
/// The elaborate gate is what decides a fixed array may reach the §7.12 methods
/// at all — 1-D, integral elements, an oracle-backed shape. This is the engine's
/// own answer to "does this net have positional words", and it must not widen
/// that gate: it is only ever asked about a net an `ArrSort`/`ArrRsort`/
/// `ArrReverse`/`Arr*` argument already named.
pub(crate) fn static_array_len(sched: &Scheduler, net: u32) -> Option<u32> {
    let nv = sched.st.ir.nets.get(net as usize)?;
    if matches!(
        nv.kind,
        sim_ir::NetKind::DynArray
            | sim_ir::NetKind::Queue
            | sim_ir::NetKind::Assoc
            | sim_ir::NetKind::AssocStr
            | sim_ir::NetKind::String
    ) || nv.array_len == 0
    {
        return None;
    }
    Some(nv.array_len)
}

/// V34-4: the §7.12.2 ordering methods applied to a FIXED-SIZE unpacked array.
///
/// Returns `true` once it has handled the receiver, `false` when `net` is not a
/// static array (the heap kinds, which the caller's own arm owns). It lives here
/// rather than inline in `dispatch.rs` because that file sits at the 1000-line
/// module ceiling and the ordering rule (`apply_order`) already lives here.
///
/// It reads and writes through exactly the two seams `$readmem*`/`$writemem*`
/// use, and for the same reason both of those were threaded:
///
///  * the `nets` READER, so a tier-3 run reads the arena rather than the
///    engine's untouched `SimState` slot, and
///  * the `out` WRITE funnel, so the element writes are scheduled the way every
///    other funnel-outside task write is — which is also what puts the sort into
///    the VCD at the right tick (measured: `#1 a.sort()` on an `int a[3]` of
///    3,1,2 dumps `1 2 3` at `#1`).
///
/// The receiver is proven writable BEFORE the design reaches here:
/// `lower_static_array_order` asks `check_lvalue_kind`, so a `wire` array is
/// `E3018` at elaborate. Without that gate the sort landed under the continuous
/// drivers and vanished at exit 0.
pub(crate) fn order_static_array<N: crate::eval::NetReader + ?Sized>(
    sched: &mut Scheduler,
    nets: Option<&N>,
    out: &mut super::TaskWrites<'_>,
    net: u32,
    which: SysTaskId,
    signed: bool,
) -> bool {
    let Some(len) = static_array_len(sched, net) else {
        return false;
    };
    let mut elems: Vec<Value> = (0..len)
        .map(|i| super::queues_io::read_task_net(sched, nets, net, Some(i)))
        .collect();
    apply_order(elems.as_mut_slice(), which, signed);
    for (i, v) in elems.into_iter().enumerate() {
        let lv = sim_ir::Lvalue {
            chunks: vec![sim_ir::LvalChunk {
                net,
                // The dummy `word: Some(0)` ExprId is never evaluated —
                // `write_chunk` takes the resolved word from the offsets pair.
                // The same shape `$readmem`'s funnel builds.
                word: Some(0),
                offset: None,
                width: None,
                kind: sim_ir::SelKind::Bit,
            }],
        };
        let off = crate::exec::Offsets::Inline {
            buf: [(0, i as u32), (0, 0)],
            len: 1,
        };
        out.put(sched, lv, v, off);
    }
    true
}

/// ⓑ-breadth (v16): total order over array elements for `sort`/`rsort`. CLEAN
/// (no x/z) values compare by signed/unsigned numeric value; x/z values sort
/// AFTER all clean values, among themselves by a deterministic raw-bit order so
/// the sort is stable across runs/OSes and never panics (IEEE leaves the x/z
/// ordering implementation-defined).
///
/// `signed` is the array's DECLARED element type (§6.11.1), NOT each element's
/// stored provenance: a `32'h80000000` pushed into a signed `int q[$]` must sort
/// as a negative. We therefore force the comparison-domain sign onto a clone (the
/// stored `Value.signed` reflects how the element literal was written, which is
/// irrelevant to the array's order) rather than calling `to_i128_signed` on the
/// raw element, whose own `self.signed` gate would otherwise leak the literal's
/// provenance.
pub(crate) fn arr_cmp(a: &Value, b: &Value, signed: bool) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    // Sign-extend at the element width regardless of the element's stored flag.
    let signed_key = |v: &Value| -> i128 {
        let mut t = v.clone();
        t.signed = true;
        t.to_i128_signed().unwrap_or(0)
    };
    match (a.has_xz(), b.has_xz()) {
        (false, false) => {
            if signed {
                signed_key(a).cmp(&signed_key(b))
            } else {
                a.to_u128().unwrap_or(0).cmp(&b.to_u128().unwrap_or(0))
            }
        }
        (false, true) => Ordering::Less,
        (true, false) => Ordering::Greater,
        (true, true) => (&*a.unk, &*a.val).cmp(&(&*b.unk, &*b.val)),
    }
}

/// Apply an in-place ordering method to a contiguous element slice.
pub(crate) fn apply_order(slice: &mut [Value], which: SysTaskId, signed: bool) {
    match which {
        SysTaskId::ArrSort => slice.sort_by(|a, b| arr_cmp(a, b, signed)),
        SysTaskId::ArrRsort => slice.sort_by(|a, b| arr_cmp(b, a, signed)),
        _ => slice.reverse(),
    }
}

/// ⓑ-breadth (v17): execute a queue-returning locator method (IEEE §7.12.1).
/// `args = [dst, src, kind_const, with_pred?]`. The source is snapshotted up
/// front so `dst == src` aliasing (`q = q.unique();`) is safe. Result elements
/// are cast to the dst element type before storage.
pub(crate) fn arr_locator(sched: &mut Scheduler, args: &[u32]) {
    let Some(dst_net) = dyn_handle_net(sched, args.first()) else {
        return;
    };
    let Some(src_net) = dyn_handle_net(sched, args.get(1)) else {
        return;
    };
    let code = args
        .get(2)
        .and_then(|&e| match sched.st.ir.exprs.get(e as usize) {
            Some(sim_ir::Expr::Const { val }) => sched
                .st
                .ir
                .consts
                .get(*val as usize)
                .and_then(|c| c.bits.val.first().copied()),
            _ => None,
        })
        .unwrap_or(0);
    let pred = args.get(3).copied();
    let signed = sched
        .st
        .ir
        .nets
        .get(src_net as usize)
        .map(|nv| nv.signed)
        .unwrap_or(true);
    let (dw, dsigned) = sched
        .st
        .ir
        .nets
        .get(dst_net as usize)
        .map(|nv| (nv.width.max(1), nv.signed))
        .unwrap_or((32, true));
    let elems = sched.st.dyn_values(src_net).unwrap_or_default();
    let idx_val = |i: usize| {
        let mut v = Value::zeros(32, true);
        v.val[0] = (i as u64).min(i32::MAX as u64);
        v
    };
    let mut result: Vec<Value> = Vec::new();
    match code {
        0 => {
            if let Some(m) = elems.iter().min_by(|a, b| arr_cmp(a, b, signed)) {
                result.push(m.clone());
            }
        }
        1 => {
            if let Some(m) = elems.iter().max_by(|a, b| arr_cmp(a, b, signed)) {
                result.push(m.clone());
            }
        }
        2 => {
            for e in &elems {
                if !result.iter().any(|r| r == e) {
                    result.push(e.clone());
                }
            }
        }
        9 => {
            let mut seen: Vec<Value> = Vec::new();
            for (i, e) in elems.iter().enumerate() {
                if !seen.iter().any(|r| r == e) {
                    seen.push(e.clone());
                    result.push(idx_val(i));
                }
            }
        }
        // find family — predicate-driven (`with` clause)
        _ => {
            if let Some(pred) = pred {
                let saved = sched.st.swap_array_item(None);
                let mut matches: Vec<(usize, Value)> = Vec::new();
                for (i, e) in elems.iter().enumerate() {
                    sched.st.swap_array_item(Some((e.clone(), i as u64)));
                    if sched.truthy(pred) {
                        matches.push((i, e.clone()));
                    }
                }
                sched.st.swap_array_item(saved);
                match code {
                    3 => result.extend(matches.iter().map(|(_, e)| e.clone())),
                    4 => result.extend(matches.iter().map(|(i, _)| idx_val(*i))),
                    5 => {
                        if let Some((_, e)) = matches.first() {
                            result.push(e.clone());
                        }
                    }
                    6 => {
                        if let Some((_, e)) = matches.last() {
                            result.push(e.clone());
                        }
                    }
                    7 => {
                        if let Some((i, _)) = matches.first() {
                            result.push(idx_val(*i));
                        }
                    }
                    8 => {
                        if let Some((i, _)) = matches.last() {
                            result.push(idx_val(*i));
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    let elems: std::collections::VecDeque<Value> = result
        .into_iter()
        .map(|v| v.resize_keep_sign(dw, dsigned))
        .collect();
    sched.st.dyn_heap.borrow_mut()[dst_net as usize] = Some(crate::state::DynObj::Queue { elems });
}

/// One W-RUN-DYN-DEGRADE per handle net (latched in `dyn_warned`) — a degraded
/// dyn op inside a loop must not spam the diagnostic stream.
pub(crate) fn dyn_warn_once(sched: &mut Scheduler, net: u32, msg: &str) {
    sched.st.dyn_warn_once_at(net, msg);
}
