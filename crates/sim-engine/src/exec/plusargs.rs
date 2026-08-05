//! `$value$plusargs` — the parse/match/convert half, shared by both kernels.
//!
//! Extracted from the engine's `k_value_plusargs` under the §4.5.302 rule: the
//! format split, the plusarg scan, and the radix conversion are exactly the
//! semantics two spellings would quietly disagree on, while the halves that
//! touch a store (resolving the destination's offsets, the write itself) stay
//! with each backend. The function returns the STATUS value and, on a hit with
//! a conversion, the destination `Lvalue` plus the value to write — the caller
//! performs the write through its own funnel.
//!
//! The conversion is width-aware (the extraction's occasion): the previous
//! spelling parsed every radix through `u64::from_str_radix(..).unwrap_or(0)`,
//! so a `%h` with more than 16 digits — or a `%d` past `u64::MAX`, or a `%b`
//! past 64 digits — silently wrote ZERO, and a negative `%d` into a
//! wider-than-64 destination was zero- instead of sign-extended (iverilog
//! writes the full value / `fff…fb`; measured). Both were silent-wrong with an
//! oracle. Bit-radixes accumulate into as many words as the digits need;
//! decimal multiply-accumulates wrapping at the DESTINATION width, which is
//! also what makes two's-complement negation come out sign-extended. For a
//! destination of 64 bits or less every path is value-identical to the old
//! spelling (mod-2^w arithmetic: wrap-at-64 then truncate ≡ wrap-at-w).

use sim_ir::{Lvalue, SimIr};

use crate::value::Value;

/// The `(status, Some((dest, value)))` of one `$value$plusargs(fmt, var)`
/// evaluation. `None` write ⇒ nothing lands (a miss, a degenerate no-`%`
/// probe, or a malformed call — elaborate's contract makes the last
/// unreachable from real designs, defended anyway).
pub(crate) fn effect(
    ir: &SimIr,
    plusargs: &[String],
    rhs: u32,
) -> (Value, Option<(Lvalue, Value)>) {
    let miss = Value::from_i128(0, 32, true);
    // args = [fmt string-literal Const, ref-var whole-net Signal] —
    // elaborate's contract; defend a hand-built IR by returning 0.
    let (fmt_eid, var_net) = match ir.exprs.get(rhs as usize) {
        Some(sim_ir::Expr::SysFunc { args, .. }) if args.len() == 2 => {
            let var = match ir.exprs.get(args[1] as usize) {
                Some(sim_ir::Expr::Signal { net, word: None }) => Some(*net),
                _ => None,
            };
            (args[0], var)
        }
        _ => (u32::MAX, None),
    };
    let fmt = match ir.exprs.get(fmt_eid as usize) {
        Some(sim_ir::Expr::Const { val }) => crate::builtins::const_string(ir, *val),
        _ => return (miss, None),
    };
    let Some(net) = var_net else {
        return (miss, None);
    };
    // split "prefix%C" — elaborate validated exactly one supported spec.
    let Some(pct) = fmt.find('%') else {
        // degenerate no-spec format: a pure test probe, nothing written.
        let hit = plusargs.iter().any(|p| p.starts_with(&fmt));
        return (Value::from_i128(hit as i128, 32, true), None);
    };
    let prefix = &fmt[..pct];
    let conv = fmt[pct + 1..].chars().next().unwrap_or('d');
    let Some(rest) = plusargs
        .iter()
        .find_map(|p| p.strip_prefix(prefix).map(|r| r.to_string()))
    else {
        return (miss, None); // MISS: var untouched
    };
    let radix: u32 = match conv {
        'd' | 'D' => 10,
        'h' | 'H' | 'x' | 'X' => 16,
        'o' | 'O' => 8,
        'b' | 'B' => 2,
        _ => 0, // %s
    };
    let dest_w = ir.nets[net as usize].width;
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
        let lead: Vec<u32> = digits.chars().map_while(|c| c.to_digit(radix)).collect();
        // Build at least the destination's width so negation spans it (the
        // sign-extension axis) and truncation of a wider parse keeps the low
        // bits (the iverilog-measured direction for every radix).
        let bits_per = match radix {
            16 => 4u32,
            8 => 3,
            2 => 1,
            _ => 0, // decimal: wrap at dest width below
        };
        let w = if bits_per == 0 {
            dest_w.max(1)
        } else {
            dest_w.max((lead.len() as u32 * bits_per).max(1))
        };
        let mut v = Value::zeros(w, false);
        let nw = v.val.len();
        if bits_per == 0 {
            // decimal: multiply-accumulate ×10 + digit, wrapping at `w`.
            for &d in &lead {
                let mut carry: u128 = d as u128;
                for word in v.val.iter_mut().take(nw) {
                    let t = (*word as u128) * 10 + carry;
                    *word = t as u64;
                    carry = t >> 64;
                }
            }
        } else {
            // bit radix: shift left by the digit's bits, OR the digit in.
            for &d in &lead {
                let mut carry: u64 = d as u64;
                for word in v.val.iter_mut().take(nw) {
                    let t = (*word << bits_per) | carry;
                    carry = *word >> (64 - bits_per);
                    *word = t;
                }
            }
        }
        if neg {
            // two's complement across the built width — within `dest_w` this
            // is exactly the sign-extended value iverilog writes.
            let mut carry = 1u64;
            for word in v.val.iter_mut() {
                let t = (!*word as u128) + carry as u128;
                *word = t as u64;
                carry = (t >> 64) as u64;
            }
        }
        v.mask_top();
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
    (Value::from_i128(1, 32, true), Some((lv, value)))
}
