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
/// (status, write, warning) — the warning carries the radix name and the
/// offending value text so the caller's W4028 can quote them without
/// re-deriving the prefix match (one spelling of the scan).
pub(crate) type PlusargsOutcome = (
    Value,
    Option<(Lvalue, Value)>,
    Option<(&'static str, String)>,
);

pub(crate) fn effect(ir: &SimIr, plusargs: &[String], rhs: u32) -> PlusargsOutcome {
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
        _ => return (miss, None, None),
    };
    let Some(net) = var_net else {
        return (miss, None, None);
    };
    // split "prefix%C" — elaborate validated exactly one supported spec.
    let Some(pct) = fmt.find('%') else {
        // degenerate no-spec format: a pure test probe, nothing written.
        let hit = plusargs.iter().any(|p| p.starts_with(&fmt));
        return (Value::from_i128(hit as i128, 32, true), None, None);
    };
    let prefix = &fmt[..pct];
    let conv = fmt[pct + 1..].chars().next().unwrap_or('d');
    let Some(rest) = plusargs
        .iter()
        .find_map(|p| p.strip_prefix(prefix).map(|r| r.to_string()))
    else {
        return (miss, None, None); // MISS: var untouched
    };
    let radix: u32 = match conv {
        'd' | 'D' => 10,
        'h' | 'H' | 'x' | 'X' => 16,
        'o' | 'O' => 8,
        'b' | 'B' => 2,
        _ => 0, // %s
    };
    let dest_w = ir.nets[net as usize].width;
    // `invalid` ⇒ the variable is written ALL-X and the caller emits W4028 —
    // iverilog-measured (it warns and writes X; status stays 1).
    let mut invalid = false;
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
        // The rules are the LITERAL conventions, iverilog-pinned per case
        // (scratch p1–p8 of the grounding): underscores are separators (never
        // leading), x/z digits parse positionally for the bit radixes and the
        // MSB digit's kind extends to the destination width, a lone x/z is a
        // whole-value x/z for %d, and anything else in the string makes the
        // value INVALID — warn + all-X, not a silent leading-digit parse.
        let (neg, digits) = match rest.strip_prefix('-') {
            Some(d) => (true, d),
            None => (false, rest.as_str()),
        };
        if radix == 10 {
            decimal_value(digits, neg, dest_w, &mut invalid)
        } else {
            let bits_per = match radix {
                16 => 4u32,
                8 => 3,
                _ => 2, // radix 2 → 1 bit; `_ => 2` is unreachable (radixes are fixed above)
            };
            let bits_per = if radix == 2 { 1 } else { bits_per };
            bit_radix_value(digits, radix, bits_per, neg, dest_w, &mut invalid)
        }
    };
    let warn = if invalid {
        let radix_name = match conv {
            'd' | 'D' => "decimal",
            'h' | 'H' | 'x' | 'X' => "hex",
            'o' | 'O' => "octal",
            _ => "binary",
        };
        Some((radix_name, rest.clone()))
    } else {
        None
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
    (Value::from_i128(1, 32, true), Some((lv, value)), warn)
}

/// `%d`: after an optional sign — empty ⇒ 0; a lone unsigned `x`/`z` ⇒ a
/// whole-value x/z; `[0-9_]` with at least one digit and no leading
/// underscore ⇒ magnitude wrapping at the destination width, negated across
/// it (that wrap is what makes sign-extension fall out); anything else ⇒
/// invalid (all-X + W4028). All iverilog-measured, including `-x` being
/// invalid while bare `x` is not.
fn decimal_value(digits: &str, neg: bool, dest_w: u32, invalid: &mut bool) -> Value {
    if digits.is_empty() {
        return Value::zeros(dest_w.max(1), false); // "" and bare "-" ⇒ 0, no warn
    }
    if !neg {
        match digits {
            "x" | "X" => return Value::xs(dest_w.max(1), false),
            "z" | "Z" => return all_z(dest_w.max(1)),
            _ => {}
        }
    }
    let well_formed = !digits.starts_with('_')
        && digits.chars().all(|c| c.is_ascii_digit() || c == '_')
        && digits.chars().any(|c| c.is_ascii_digit());
    if !well_formed {
        *invalid = true;
        return Value::xs(dest_w.max(1), false);
    }
    let mut v = Value::zeros(dest_w.max(1), false);
    let nw = v.val.len();
    for c in digits.chars().filter(char::is_ascii_digit) {
        let mut carry: u128 = c.to_digit(10).unwrap() as u128;
        for word in v.val.iter_mut().take(nw) {
            let t = (*word as u128) * 10 + carry;
            *word = t as u64;
            carry = t >> 64;
        }
    }
    if neg {
        negate_words(&mut v);
    }
    v.mask_top();
    v
}

/// `%h`/`%o`/`%b`: positional 4-state parse. Digits, `x`/`z` (either case) and
/// non-leading underscores are the alphabet; anything else ⇒ invalid (all-X +
/// W4028). Empty ⇒ all-X without the warning (also bare `-`). The MSB digit's
/// kind extends to the destination width (`z1` ⇒ `zz…z1`), truncation keeps
/// the LOW bits, and a `-` on a value containing x/z is 4-state arithmetic's
/// all-X — every rule iverilog-measured.
fn bit_radix_value(
    digits: &str,
    radix: u32,
    bits_per: u32,
    neg: bool,
    dest_w: u32,
    invalid: &mut bool,
) -> Value {
    if digits.is_empty() {
        return Value::xs(dest_w.max(1), false);
    }
    enum D {
        Num(u64),
        X,
        Z,
    }
    let mut parsed: Vec<D> = Vec::new();
    let mut ok = !digits.starts_with('_');
    for c in digits.chars() {
        match c {
            '_' => {}
            'x' | 'X' => parsed.push(D::X),
            'z' | 'Z' => parsed.push(D::Z),
            _ => match c.to_digit(radix) {
                Some(d) => parsed.push(D::Num(d as u64)),
                None => {
                    ok = false;
                    break;
                }
            },
        }
    }
    if !ok || parsed.is_empty() {
        *invalid = true;
        return Value::xs(dest_w.max(1), false);
    }
    let w = dest_w.max((parsed.len() as u32 * bits_per).max(1));
    let mut v = Value::zeros(w, false);
    let nw = v.val.len();
    let digit_mask = (1u64 << bits_per) - 1;
    for d in &parsed {
        // shift both planes left by the digit's bits, then OR the digit in.
        let (dv, du) = match d {
            D::Num(n) => (*n, 0u64),
            D::X => (0, digit_mask),
            D::Z => (digit_mask, digit_mask),
        };
        let mut carry_v = dv;
        let mut carry_u = du;
        for i in 0..nw {
            let tv = (v.val[i] << bits_per) | carry_v;
            let tu = (v.unk[i] << bits_per) | carry_u;
            carry_v = v.val[i] >> (64 - bits_per);
            carry_u = v.unk[i] >> (64 - bits_per);
            v.val[i] = tv;
            v.unk[i] = tu;
        }
    }
    // MSB-digit x/z extends to the full width (the literal convention).
    let used = parsed.len() as u32 * bits_per;
    if used < w {
        let (ev, eu) = match parsed.first() {
            Some(D::X) => (false, true),
            Some(D::Z) => (true, true),
            _ => (false, false),
        };
        if eu {
            for bit in used..w {
                let (wi, bi) = ((bit / 64) as usize, bit % 64);
                v.unk[wi] |= 1u64 << bi;
                if ev {
                    v.val[wi] |= 1u64 << bi;
                } else {
                    v.val[wi] &= !(1u64 << bi);
                }
            }
        }
    }
    if neg {
        if v.unk.iter().any(|&u| u != 0) {
            return Value::xs(dest_w.max(1), false); // -(x-bearing) ⇒ all-X, no warn
        }
        negate_words(&mut v);
    }
    v.mask_top();
    v
}

fn all_z(w: u32) -> Value {
    let mut v = Value::zeros(w, false);
    for i in 0..v.val.len() {
        v.val[i] = u64::MAX;
        v.unk[i] = u64::MAX;
    }
    v.mask_top();
    v
}

fn negate_words(v: &mut Value) {
    let mut carry = 1u64;
    for word in v.val.iter_mut() {
        let t = (!*word as u128) + carry as u128;
        *word = t as u64;
        carry = (t >> 64) as u64;
    }
}
