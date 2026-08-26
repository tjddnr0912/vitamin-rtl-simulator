//! `%p` — the IEEE 1800 §21.2.1.7 ASSIGNMENT-PATTERN format.
//!
//! `%p` is the one conversion that asks for a RENDERING of an aggregate rather
//! than for its value, so it is the one conversion whose argument may be a whole
//! unpacked array / dynamic array / queue / associative array. Everything else in
//! `render_template` starts by evaluating the argument to a `Value`; this module
//! is what runs INSTEAD of that step when the argument denotes an aggregate.
//!
//! ## Which oracle
//!
//! **iverilog 13 does not implement `%p` at all** — measured, not assumed: it
//! warns `unknown format $display<%p>`, emits the four characters `<%p>` and then
//! prints the argument with the default radix, and it refuses the aggregate
//! argument outright (`$display does not support argument type (vpiMemory)`).
//! So the project's primary oracle has nothing to say here and verilator 5.050 is
//! the only tool that implements the spec. The format below is verilator's,
//! measured on 5.050:
//!
//! ```text
//!   int a[3] = '{1,2,3};        %p -> '{'h1, 'h2, 'h3}
//!   int q[$] = {7,8};           %p -> '{'h7, 'h8}
//!   int m[string]; m["k"]=9;    %p -> '{"k":'h9}
//!   int n[int];    n[3]=4;      %p -> '{'h3:'h4}
//!   int e[$];                   %p -> '{}
//!   int a2[2][3];               %p -> '{'{'h1, 'h2, 'h3}, '{'h4, 'h5, 'h6}}
//!   struct packed {…} s = 8'hA5;%p -> 165            %0p -> 'ha5
//!   int i = -5;                 %p -> -5             %0p -> 'hfffffffb
//!   real r = 2.5;               %p -> 2.5            %0p -> 2.5
//!   string s = "x";             %p -> "x"            %0p -> "x"
//! ```
//!
//! Two things fall out of that table and both are structural, not cosmetic:
//!
//! * the `0` flag only means anything on a NON-aggregate — `%p` and `%0p` of an
//!   array print the same text — and on a non-aggregate it selects between the
//!   decimal form (`%0d`) and the `'h` form;
//! * an ELEMENT of an aggregate is always rendered in the `'h` form. So "render
//!   one element" and "render a scalar under `%0p`" are the same function
//!   ([`pattern_leaf`]), which is why this module has one leaf renderer and not
//!   two.
//!
//! ## Where vita deliberately differs, and why
//!
//! * **Associative-array ITERATION ORDER with a negative integer key.** vita
//!   iterates its `BTreeMap<i64, _>` in SIGNED order, which is the order IEEE
//!   §7.9.4 gives `first`/`next` for a signed index type and the order vita's own
//!   `first`/`next` already use. verilator orders `n[-1]` AFTER `n[2]` (it sorts
//!   the rendered hex). Matching verilator here would make `%p` disagree with
//!   vita's own `foreach`, so it is not matched — and the divergence is confined
//!   to a negative key, which no test in the corpus uses.
//! * **Width of a negative integer KEY.** `DynObj::Assoc` stores keys as `i64`;
//!   the declared key type's width is cast away before the IR (see the `Assoc`
//!   doc in `state/mod.rs`), so a key of `-1` renders as the 64-bit two's
//!   complement `'hffffffffffffffff` where verilator, which still has the
//!   declared type, prints `'hffffffff` for an `int` key. Non-negative keys —
//!   every key the corpus and the LRM examples use — agree exactly.
//! * **x/z digits.** verilator is 2-state and cannot answer. An unknown nibble
//!   renders exactly as `%0h` renders it (`fmt_radix`, iverilog-pinned per-digit
//!   x/z/X/Z rules), because `'h` IS the hex form and inventing a second
//!   convention for the same digits would be the only way to be wrong twice.

use super::*;

/// One ELEMENT of an assignment pattern — and, identically, a non-aggregate
/// under `%0p`.
///
/// The three domains are asked in the order that cannot lose information: a
/// `real` first (its bits are an f64, and the hex of an f64 bit pattern is not a
/// rendering of the number), then a string-domain value (whose bytes are text,
/// not a magnitude), then the integral `'h` form.
pub(crate) fn pattern_leaf(v: &Value) -> String {
    if v.is_real {
        // `%g` — the shortest spelling that reads back as the same number, i.e.
        // what an assignment pattern of a real looks like. Same call the `%p`
        // real path has made since the real arm landed.
        fmt_real(v, 'g', None, None, false, false, false)
    } else if v.is_str {
        format!("\"{}\"", String::from_utf8_lossy(&v.to_str_bytes()))
    } else {
        // `'h` + minimal hex = `fmt_radix(.., min_zero, Some(0))`, the exact
        // `%0h` spelling; leading zeros stripped, at least one digit, x/z per
        // §21.2.1.2.
        format!("'h{}", fmt_radix(v, 4, true, Some(0), false))
    }
}

/// A non-aggregate under `%p`/`%0p`.
///
/// `min_zero` is the `0` flag, and only the INTEGRAL domain reads it (verilator
/// prints a real and a string identically under both spellings): it selects the
/// decimal form for bare `%p` and [`pattern_leaf`]'s `'h` form for `%0p`.
///
/// The real arm keeps taking the full flag set rather than forwarding to
/// [`pattern_leaf`] — `%p` of a real has honoured width/precision/`-`/`+` since
/// that arm landed, and dropping them here would be a silent regression on a
/// spelling no aggregate can produce anyway (an ELEMENT is always rendered
/// unpadded).
#[allow(clippy::too_many_arguments)]
pub(crate) fn pattern_scalar(
    v: &Value,
    min_zero: bool,
    field_width: Option<usize>,
    precision: Option<usize>,
    left_just: bool,
    plus: bool,
) -> String {
    if v.is_real {
        fmt_real(v, 'g', field_width, precision, left_just, min_zero, plus)
    } else if v.is_str || min_zero {
        justify(&pattern_leaf(v), field_width, left_just)
    } else {
        justify(&fmt_dec(v), field_width, left_just)
    }
}

/// The whole-aggregate render, or `None` when `net` does not denote an aggregate
/// this function can render — in which case the caller falls back to the ordinary
/// value path.
///
/// `None` is returned for a `String` net ON PURPOSE: a string already HAS a whole
/// value surface (`eval_expr_with` yields an `is_str` `Value`), so routing it
/// here would be a second spelling of [`pattern_leaf`]'s string arm.
///
/// ⚠️ The static-array arm tests `array_len > 1`, which cannot see a ONE-element
/// unpacked array (`int a[0:0]` stores `array_len = 1`, exactly like a scalar —
/// elaborate tracks that case in `unpacked_array_nets`, a table that never
/// reaches the engine). That shape is REFUSED at elaborate instead of silently
/// rendering as a bare scalar; see `Elaborator::lower_fmt_value_arg` /
/// `pattern_arg_is_unrenderable_array` there, and the deferred-hier twin in
/// `hier_defer::read` for the cross-instance spelling.
pub(crate) fn pattern_of_net<N: crate::eval::NetReader + ?Sized>(
    st: &SimState,
    nets: &N,
    net: u32,
) -> Option<String> {
    let nv = st.ir.nets.get(net as usize)?;
    match nv.kind {
        sim_ir::NetKind::DynArray | sim_ir::NetKind::Queue => {
            let heap = st.dyn_heap.borrow();
            // A handle that was never `new`ed / pushed has NO heap entry, and
            // IEEE says that IS the empty aggregate — the same lazy contract
            // every other dyn read uses, so `'{}' rather than `x`.
            let body = match heap.get(net as usize).and_then(|o| o.as_ref()) {
                Some(crate::state::DynObj::DynArray { elems }) => join_leaves(elems.iter()),
                Some(crate::state::DynObj::Queue { elems }) => join_leaves(elems.iter()),
                _ => String::new(),
            };
            Some(format!("'{{{body}}}"))
        }
        sim_ir::NetKind::Assoc => {
            let heap = st.dyn_heap.borrow();
            let body = match heap.get(net as usize).and_then(|o| o.as_ref()) {
                Some(crate::state::DynObj::Assoc { map }) => map
                    .iter()
                    .map(|(k, v)| format!("'h{:x}:{}", *k as u64, pattern_leaf(v)))
                    .collect::<Vec<_>>()
                    .join(", "),
                _ => String::new(),
            };
            Some(format!("'{{{body}}}"))
        }
        sim_ir::NetKind::AssocStr => {
            let heap = st.dyn_heap.borrow();
            let body = match heap.get(net as usize).and_then(|o| o.as_ref()) {
                Some(crate::state::DynObj::AssocStr { map }) => map
                    .iter()
                    .map(|(k, v)| format!("\"{}\":{}", String::from_utf8_lossy(k), pattern_leaf(v)))
                    .collect::<Vec<_>>()
                    .join(", "),
                _ => String::new(),
            };
            Some(format!("'{{{body}}}"))
        }
        // A string's whole value is a `Value`; see the doc above.
        sim_ir::NetKind::String => None,
        sim_ir::NetKind::Wire
        | sim_ir::NetKind::Reg
        | sim_ir::NetKind::Logic
        | sim_ir::NetKind::Integer
        | sim_ir::NetKind::Real => {
            if nv.array_len <= 1 {
                return None; // a scalar (or the one-element array elaborate refuses)
            }
            // Dimensions come from the `net_dims` sidecar, the SAME table
            // `flatten_word` derives its strides from, so the nesting here is the
            // inverse of the addressing the design itself uses. Absent ⇒ plain
            // 0-based 1-D (the sidecar is sparse by contract), which is exactly
            // the fallback `net_dim_extents` uses in elaborate.
            let owned;
            let dims: &[(i64, u32)] = match st.net_dims.get(&net) {
                Some(d) if !d.is_empty() => d,
                _ => {
                    owned = vec![(0i64, nv.array_len)];
                    &owned
                }
            };
            Some(render_dims(nets, net, dims, 0))
        }
    }
}

fn join_leaves<'a>(it: impl Iterator<Item = &'a Value>) -> String {
    it.map(pattern_leaf).collect::<Vec<_>>().join(", ")
}

/// Row-major recursion over the unpacked dimensions.
///
/// `base` is the flat word index of this sub-array's element 0. The stride of a
/// dimension is the product of the sizes AFTER it — the suffix product
/// `flatten_word` uses when it lowers `a[i][j]`, so a `%p` render and an element
/// read agree about which word is which by construction rather than by test.
fn render_dims<N: crate::eval::NetReader + ?Sized>(
    nets: &N,
    net: u32,
    dims: &[(i64, u32)],
    base: usize,
) -> String {
    let (_, size) = dims[0];
    let parts: Vec<String> = if dims.len() == 1 {
        (0..size as usize)
            .map(|k| pattern_leaf(&nets.read_net(net, Some((base + k) as u32))))
            .collect()
    } else {
        let stride: usize = dims[1..].iter().map(|&(_, n)| n as usize).product();
        (0..size as usize)
            .map(|k| render_dims(nets, net, &dims[1..], base + k * stride))
            .collect()
    };
    format!("'{{{}}}", parts.join(", "))
}
