//! split part of `builtins` (mechanical move).

use super::*;

pub(crate) fn render_template<N: crate::eval::NetReader + ?Sized>(
    st: &SimState,
    nets: &N,
    template: &str,
    args: &[u32],
    argi: &mut usize,
    out: &mut String,
) {
    let mut chars = template.chars().peekable();

    while let Some(c) = chars.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        // optional width/flags: %0d, %5h, %8.2f, %-5d, %+d …  (v1 records `0` for
        // integer specs; width/precision are threaded into the real `%f/%e/%g`
        // formatters; `-` left-justifies, `+` forces a sign).
        let mut min_zero = false;
        let mut left_just = false;
        let mut plus = false;
        let mut width_digits = String::new();
        while let Some(&d) = chars.peek() {
            if d == '-' && width_digits.is_empty() {
                // `-` (left-justify) flag: the content right-pads with spaces to
                // the field width and OVERRIDES the `0` flag (iverilog-pinned:
                // `%-05d` of 42 → "42   ", spaces not zeros). It must PRECEDE the
                // width digits — iverilog itself rejects `%0-5d` (echoes it raw),
                // so gating on an empty width run keeps that malformed order loud.
                left_just = true;
                chars.next();
            } else if d == '+' && width_digits.is_empty() {
                // `+` flag: force a leading `+` on a non-negative SIGNED-decimal
                // (`%d`) or real (`%f/%e/%g`) value (iverilog ignores it for the
                // unsigned `%h/%o/%b`, `%t`, `%c`). Order-independent with `-`.
                plus = true;
                chars.next();
            } else if d == '0' && width_digits.is_empty() {
                min_zero = true;
                width_digits.push('0');
                chars.next();
            } else if d.is_ascii_digit() {
                width_digits.push(d);
                chars.next();
            } else {
                break;
            }
        }
        let mut prec_digits = String::new();
        let mut has_prec = false;
        if chars.peek() == Some(&'.') {
            has_prec = true;
            chars.next();
            while let Some(&d) = chars.peek() {
                if d.is_ascii_digit() {
                    prec_digits.push(d);
                    chars.next();
                } else {
                    break;
                }
            }
        }
        let field_width: Option<usize> = width_digits
            .trim_start_matches('0')
            .parse::<usize>()
            .ok()
            .or_else(|| {
                if width_digits.chars().all(|c| c == '0') && !width_digits.is_empty() {
                    Some(0)
                } else {
                    None
                }
            });
        let precision: Option<usize> = if has_prec {
            Some(prec_digits.parse::<usize>().unwrap_or(0))
        } else {
            None
        };
        let spec = chars.next().unwrap_or('%');
        match spec {
            '%' => out.push('%'),
            // P2-11: hierarchical scope of the EXECUTING process (was: always
            // the literal "top"). Strobe/monitor renders restore the
            // REGISTERING process's scope first (FmtCapture.scope).
            'm' | 'M' => {
                // N1: `%m` inside a subroutine body reports `module.function`
                // (IEEE §21.2.1) — append the active frame-subroutine name(s), resolved
                // from FuncId via `func_names`. Empty stack (module-level) ⇒ just
                // `cur_scope`, byte-identical to before.
                let fs = st.frame_scope.borrow();
                let scope = if fs.is_empty() {
                    st.cur_scope.clone()
                } else {
                    let mut s = st.cur_scope.clone();
                    for &fid in fs.iter() {
                        if let Some(n) = st.func_names.get(fid as usize) {
                            if !n.is_empty() {
                                s.push('.');
                                s.push_str(n);
                            }
                        }
                    }
                    s
                };
                out.push_str(&justify(&scope, field_width, left_just));
            }
            't' | 'T' => {
                // `%T` aliases `%t` (consumes one time arg). Full §21.3.2 semantics:
                // the arg (a time in the DISPLAYING module's unit) is rescaled to the
                // `$timeformat` units (default: the global precision) and justified
                // in the min field width (default 20; an explicit `%Nt`/`%0t`
                // OVERRIDES it — iverilog-pinned).
                let v = next_arg_with(st, nets, args, argi);
                out.push_str(&fmt_time_spec(st, &v, field_width, min_zero, left_just));
            }
            'd' | 'D' => {
                let v = next_arg_with(st, nets, args, argi);
                // IEEE 1364 %d: right-justify in a field width. `%0d` ⇒ minimal;
                // `%Nd` ⇒ width N; bare `%d` ⇒ the operand's default decimal width
                // (digit count of its max value). An X/Z prints as a right-justified
                // `x`/`z` in that field, like a numeric value.
                let mut s = fmt_dec(&v);
                // `+` forces a leading sign on a NON-negative value (iverilog:
                // `%+d` of 42 → "+42", of x → "+X"; a negative already carries
                // `-`). The sign counts toward the field width.
                if plus && !s.starts_with('-') {
                    s.insert(0, '+');
                }
                // `%0d` (bare leading zero, no width) = minimal; `%0Nd` = zero-pad
                // to N (sign-aware: "-42"→"-00042"); `%Nd` = space-pad to N; bare
                // `%d` = the operand's default decimal field width (iverilog-pinned).
                let fw = match (min_zero, field_width) {
                    (true, Some(0)) => 0,
                    (_, Some(n)) => n,
                    // bare `%d` of a REAL has NO default field width — iverilog
                    // prints the rounded value unpadded (`%d` of 4.0 → "4", not the
                    // 20-wide u64 field `dec_field_width(64)` would give). An
                    // explicit `%Nd`/`%0Nd` still pads (handled above).
                    (_, None) if v.is_real => 0,
                    (_, None) => dec_field_width(v.width, v.signed),
                };
                if s.len() < fw {
                    let pad = fw - s.len();
                    if left_just {
                        // `-` right-pads with spaces (and overrides `0`).
                        out.push_str(&s);
                        out.push_str(&" ".repeat(pad));
                    } else if min_zero {
                        // zero-pad AFTER any leading sign (`-` or `+`): "-42" →
                        // "-00042", "+42" → "+00042".
                        if let Some(rest) = s.strip_prefix(['-', '+']) {
                            out.push_str(&s[..1]);
                            out.push_str(&"0".repeat(pad));
                            out.push_str(rest);
                        } else {
                            out.push_str(&"0".repeat(pad));
                            out.push_str(&s);
                        }
                    } else {
                        out.push_str(&" ".repeat(pad));
                        out.push_str(&s);
                    }
                } else {
                    out.push_str(&s);
                }
            }
            'h' | 'H' | 'x' | 'X' => {
                let v = next_arg_with(st, nets, args, argi);
                out.push_str(&fmt_radix(&v, 4, min_zero, field_width, left_just));
            }
            'o' | 'O' => {
                let v = next_arg_with(st, nets, args, argi);
                out.push_str(&fmt_radix(&v, 3, min_zero, field_width, left_just));
            }
            'b' | 'B' => {
                let v = next_arg_with(st, nets, args, argi);
                out.push_str(&fmt_radix(&v, 1, min_zero, field_width, left_just));
            }
            'f' | 'F' | 'g' | 'G' | 'e' | 'E' => {
                let v = next_arg_with(st, nets, args, argi);
                let s = fmt_real(&v, spec, field_width, precision, left_just, min_zero, plus);
                // `%E`/`%G` uppercase the exponent letter and non-finite labels
                // (iverilog: `%E` → "1.5E+20", `%G` → "1E-05", `%E` of inf → "INF").
                // `%F` is identical to `%f` for ALL values including inf/nan — iverilog
                // outputs lowercase "inf"/"nan" for `%F` (only `%E`/`%G` uppercase them).
                if spec == 'E' || spec == 'G' {
                    // ASCII-only output (digits/`.`/`-`/`+`/`e`/inf/nan) — uppercase
                    // affects only `e`→`E` and inf/nan→INF/NAN.
                    out.push_str(&s.to_ascii_uppercase());
                } else {
                    out.push_str(&s);
                }
            }
            'c' | 'C' => {
                let v = next_arg_with(st, nets, args, argi);
                out.push_str(&justify(&char_of(&v).to_string(), field_width, left_just));
            }
            's' | 'S' => {
                let e = args.get(*argi).copied();
                *argi += 1;
                // Build the content string, then justify it in the field width (a
                // MINIMUM — a longer string overflows, it is never truncated). The
                // content for an explicit-width `%Ns`/`%0s` on a packed reg is its
                // leading-NUL-stripped form; a RIGHT-justified bare `%s` keeps the
                // full reg-width form (NUL → space). A string literal / string-domain
                // value renders its exact text either way. (iverilog-pinned: 64-bit
                // "hello" → `%s` "   hello", `%2s` "hello", `%10s` "     hello".)
                //
                // Under `-` on a packed reg with NO explicit width, iverilog strips
                // the NUL padding and LEFT-justifies in the reg's byte width (`%-s`
                // of a 64-bit "cpu0" → "cpu0    ", not the NUL→space "    cpu0"), so
                // a default width is supplied and the stripped content used.
                let (content, default_w) = match e {
                    // string LITERAL (StrUtf8): decoded text (the classic fmt-arg
                    // path). A NUMERIC const (e.g. `%s` of 16'h0041) is deliberately
                    // NOT matched here — it falls through to the packed-value branch
                    // below so it renders via fmt_packed_chars (NUL byte → space,
                    // full reg width), byte-identical to the runtime-value path.
                    // const_string would instead strip trailing NUL and emit an
                    // embedded NUL literally (iverilog renders every 0x00 as a space).
                    Some(eid)
                        if matches!(
                            st.ir.exprs.get(eid as usize),
                            Some(sim_ir::Expr::Const { val })
                                if st.ir.consts[*val as usize].repr
                                    == sim_ir::ConstRepr::StrUtf8
                        ) =>
                    {
                        // `arg_string`, not a reader-threaded twin: this arm is guarded on
                        // `Expr::Const` of `StrUtf8` just above, so the value is a string
                        // LITERAL and its evaluation cannot touch a net. A `_with` twin here
                        // would carry a parameter no test could pin — the mutation that drops
                        // it survives by construction, which reads exactly like a real gap.
                        // `$dumpfile(name)` (dispatch.rs) is the caller that WILL need one,
                        // and that is S1d-4b-2's, where a test can pin it.
                        (crate::builtins::arg_string(st, Some(eid)), None)
                    }
                    // A NESTED `$sformatf` argument (`$sformatf("<%s>", $sformatf(…))`,
                    // and every string CONCAT, which elaborate desugars to exactly that
                    // shape). It must be rendered through THIS formatter: the generic
                    // `eval_sysfunc` arm cannot see a format string and would dump the raw
                    // args, so the inner text came back blank — `<%s>` of `$sformatf("%0d",
                    // 7)` printed `<   >`. That blank is why the `$sformatf` hoist exists at
                    // all, and why it could not reach a frame FUNCTION body, whose temp
                    // would have to be a module net.
                    Some(eid)
                        if matches!(
                            st.ir.exprs.get(eid as usize),
                            Some(sim_ir::Expr::SysFunc {
                                which: sim_ir::SysFuncId::Sformatf,
                                ..
                            })
                        ) =>
                    {
                        let Some(sim_ir::Expr::SysFunc { args: sa, .. }) =
                            st.ir.exprs.get(eid as usize)
                        else {
                            unreachable!("matched just above")
                        };
                        let (f, rest) = (sa.first().copied(), sa.get(1..).unwrap_or(&[]).to_vec());
                        (
                            crate::builtins::format_args_str_with(st, nets, f, &rest, None),
                            None,
                        )
                    }
                    Some(eid) => {
                        let v = st.eval_expr_with(nets, eid);
                        if v.is_str {
                            // v7 P2-C: a STRING-domain value renders its EXACT bytes.
                            (
                                String::from_utf8_lossy(&v.to_str_bytes()).into_owned(),
                                None,
                            )
                        } else if left_just {
                            // stripped content + default field = reg byte width.
                            let nbytes = (v.width as usize).div_ceil(8).max(1);
                            (fmt_packed_chars_min(&v), Some(nbytes))
                        } else if field_width.is_some() {
                            // `%0s` / `%Ns` strip leading NUL padding (all-NUL → "").
                            (fmt_packed_chars_min(&v), None)
                        } else {
                            // bare `%s` pads to the reg width (NUL → space).
                            (fmt_packed_chars(&v), None)
                        }
                    }
                    None => (String::new(), None),
                };
                // Explicit width wins over the packed-reg default; `justify`
                // right-pads spaces when `-` is set, else left-pads (byte-identical
                // to the old match when `left_just` is false and `default_w` None).
                out.push_str(&justify(&content, field_width.or(default_w), left_just));
            }
            // P0-8③: the remaining IEEE specs CONSUME their argument — leaving
            // them unconsumed shifted every later spec onto the wrong arg.
            'v' | 'V' => {
                let v = next_arg_with(st, nets, args, argi);
                out.push_str(&justify(&strength_form(&v), field_width, left_just));
            }
            // binary-dump specs: consume; vitamin emits no text for them (v1 —
            // the IEEE form writes raw bytes, useless in a text log).
            'u' | 'U' | 'z' | 'Z' => {
                let _ = next_arg_with(st, nets, args, argi);
            }
            // `%p` (SV assignment pattern, §21.2.1.7) — the one conversion that
            // asks for a RENDERING rather than for a value, and therefore the one
            // whose argument may denote a whole aggregate. The two rules and the
            // measured verilator format live in `builtins::pattern`.
            //
            // The argument eid is PEEKED before it is consumed: an aggregate has no
            // scalar value, so `next_arg_with` must not be the thing that decides.
            // `Signal { word: None }` is the shape elaborate emits for a whole
            // aggregate in this position — and only in this position, because a
            // whole aggregate anywhere else is still E3009.
            //
            // RESIDUAL (no oracle to match, and no net to render): an UNPACKED
            // struct and a fixed-size array of `string` are not declarable in vita
            // at all (E3010 at the declaration), so `'{x:7, y:-2}` has nothing to
            // render from. That is a declaration gap, not a `%p` gap.
            'p' | 'P' => {
                let aggregate =
                    args.get(*argi)
                        .copied()
                        .and_then(|eid| match st.ir.exprs.get(eid as usize) {
                            Some(sim_ir::Expr::Signal { net, word: None }) => {
                                crate::builtins::pattern_of_net(st, nets, *net)
                            }
                            _ => None,
                        });
                match aggregate {
                    Some(s) => {
                        *argi += 1;
                        out.push_str(&justify(&s, field_width, left_just));
                    }
                    None => {
                        let v = next_arg_with(st, nets, args, argi);
                        out.push_str(&crate::builtins::pattern_scalar(
                            &v,
                            min_zero,
                            field_width,
                            precision,
                            left_just,
                            plus,
                        ));
                    }
                }
            }
            other => {
                out.push('%');
                out.push(other);
            }
        }
    }
}

/// `%s` on a packed value: width/8 chars MSB-first, NUL bytes render as
/// spaces (iverilog live pin, probe t7).
pub(crate) fn fmt_packed_chars(v: &Value) -> String {
    let nbytes = (v.width as usize).div_ceil(8).max(1);
    let mut s = String::with_capacity(nbytes);
    for bi in (0..nbytes).rev() {
        let byte = packed_byte(v, bi);
        s.push(if byte == 0 { ' ' } else { byte as char });
    }
    s
}

/// Byte `bi` of a packed value (LSB byte = 0). Unknown (x/z) bits are masked OFF
/// before the byte is read — so an x byte (val=0) AND a z byte (val=1,unk=1) both
/// read as `0` (a NUL → rendered as a space), matching iverilog. vita's z-bit
/// encoding is val=1, which would otherwise leak `0xFF`.
pub(crate) fn packed_byte(v: &Value, bi: usize) -> u8 {
    let bit = bi * 8;
    let w = bit / 64;
    let known = v.val.get(w).copied().unwrap_or(0) & !v.unk.get(w).copied().unwrap_or(0);
    (known >> (bit % 64)) as u8
}

/// `%0s` on a packed value: like [`fmt_packed_chars`] but the LEADING NUL bytes
/// (the high zero-byte padding of a string in a wider reg) are dropped rather than
/// rendered as spaces. Once the first non-NUL byte is seen, every later byte is
/// emitted (an embedded or trailing NUL still becomes a space). An all-NUL value
/// yields the empty string. iverilog-pinned: 64-bit "hello" → "hello"; "hi\0\0" →
/// "hi  "; "\0h\0i" → "h i"; all-NUL → "".
pub(crate) fn fmt_packed_chars_min(v: &Value) -> String {
    let nbytes = (v.width as usize).div_ceil(8).max(1);
    let mut s = String::with_capacity(nbytes);
    let mut started = false;
    for bi in (0..nbytes).rev() {
        let byte = packed_byte(v, bi); // x/z masked → 0, so leading x/z also strips
        if !started {
            if byte == 0 {
                continue; // skip leading NUL (and x/z) padding
            }
            started = true;
        }
        s.push(if byte == 0 { ' ' } else { byte as char });
    }
    s
}

/// `next_arg` against an alternate net store — see `format_args_str_with`.
pub(crate) fn next_arg_with<N: crate::eval::NetReader + ?Sized>(
    st: &SimState,
    nets: &N,
    args: &[u32],
    argi: &mut usize,
) -> Value {
    let e = args.get(*argi).copied();
    *argi += 1;
    e.map(|x| st.eval_expr_with(nets, x))
        .unwrap_or_else(Value::x1)
}

/// IEEE %d default field width = decimal digit count of an `n`-bit operand's max
/// value (`2^n − 1`): 1-bit→1, 8-bit→3, 32-bit→10. Computed exactly up to 128 bits,
/// then via `n·log10(2)` (a column-alignment hint; exactness beyond 128 is moot).
pub(crate) fn dec_field_width(n: u32, signed: bool) -> usize {
    if n == 0 {
        return 1;
    }
    if signed && n > 1 {
        // A signed `%d` field holds a sign char plus the digits of the most-negative
        // magnitude 2^(n-1) (iverilog-pinned: 8-bit → "-128" = 4, 32-bit →
        // "-2147483648" = 11). This is NOT simply unsigned_width + 1 — for some
        // widths the two coincide (10-bit: signed "-512" and unsigned 1023 are both
        // 4 wide), so the magnitude must be computed directly. A 1-bit signed value
        // is the exception: iverilog gives it field width 1 (NOT 2), so n==1 falls
        // through to the unsigned branch below (the lone `-1` overflows the 1-col
        // field, as in iverilog).
        if n <= 128 {
            let mag: u128 = 1u128 << (n - 1);
            1 + mag.to_string().len()
        } else {
            // wide: digits(2^(n-1)) ≈ (n-1)*log10(2)+1, plus the sign.
            2 + ((n - 1) as f64 * std::f64::consts::LOG10_2) as usize
        }
    } else if n <= 128 {
        let maxv: u128 = if n == 128 {
            u128::MAX
        } else {
            (1u128 << n) - 1
        };
        maxv.to_string().len()
    } else {
        (n as f64 * std::f64::consts::LOG10_2) as usize + 1
    }
}

/// IEEE 1800 §21.2.1.2 letter for a bit range `[lo,hi)` that contains ≥1 unknown
/// bit. The letter is LOWERCASE only when the group is UNIFORM — no known bit AND
/// a single unknown kind (entirely x → `x`, entirely z → `z`). ANY mixing —
/// known+unknown, or x+z together — is UPPERCASE (`X` if any x, else `Z`). x takes
/// precedence over z. `None` when the group is fully known. (iverilog-pinned: e.g.
/// `8'bxxxxzzzz` prints `X`, not `x`.)
pub(crate) fn unknown_group_char(v: &Value, lo: u32, hi: u32) -> Option<char> {
    let (mut has_known, mut has_x, mut has_z) = (false, false, false);
    for i in lo..hi {
        let (val, unk) = v.get_vu(i);
        if unk == 1 {
            if val == 0 {
                has_x = true; // x = (val0, unk1); z = (val1, unk1)
            } else {
                has_z = true;
            }
        } else {
            has_known = true;
        }
    }
    if !has_x && !has_z {
        return None;
    }
    let uppercase = has_known || (has_x && has_z);
    Some(match (uppercase, has_x) {
        (false, true) => 'x',  // uniform all-x, no known
        (false, false) => 'z', // uniform all-z, no known
        (true, true) => 'X',   // mixed (known and/or x+z), some x
        (true, false) => 'Z',  // mixed, all unknowns are z
    })
}

/// %d: decimal. A value with any X/Z renders one §21.2.1.2 letter for the whole
/// field (`x`/`z` if entirely unknown, `X`/`Z` if partially). A real ROUNDS
/// half-away (saturating to i64 extremes; NaN → 0).
pub(crate) fn fmt_dec(v: &Value) -> String {
    if v.is_real {
        let x = v.to_f64().unwrap_or(0.0);
        // round half-away; large |x| SATURATES to i64::MAX/MIN; NaN.round() as i64 == 0.
        return format!("{}", x.round() as i64);
    }
    if let Some(c) = unknown_group_char(v, 0, v.width) {
        return c.to_string();
    }
    // Exact decimal at ANY width (Phase-1.x ⑥): a wide signed value renders
    // sign + two's-complement magnitude; unsigned long-divides by 10^19.
    // (%d used to render signed >64 as unsigned and TRUNCATE past 128 bits.)
    let n = crate::value::nwords(v.width).max(1);
    let mut words: Vec<u64> = (0..n).map(|k| v.val.get(k).copied().unwrap_or(0)).collect();
    let neg = v.signed && v.width >= 1 && v.get_vu(v.width - 1).0 == 1;
    if neg {
        words = crate::eval::mw_mask(crate::eval::mw_neg(&words), v.width);
    }
    let s = crate::eval::mw_decimal(&words);
    if neg {
        format!("-{s}")
    } else {
        s
    }
}

/// `%t` (IEEE 1800 §21.3.2): render a time VALUE (expressed in the displaying
/// module's time unit) per the live `$timeformat` state. iverilog-pinned:
/// - display value = arg × M (the module unit in ticks), decimal-shifted by
///   `units_exp − global_prec_exp` places (default units = the global precision,
///   so a 1ns/1ps module shows ps: `%0t` of `$time`=5 → "5000");
/// - INTEGER args are exact decimal-string math and TRUNCATE the fraction to
///   `prec` digits (9995ns @ −6/prec 1 → "9.9", never rounds); REAL args go
///   through f64 and round like `%f` (5.556 @ prec 2 → "5.56");
/// - an unknown-bearing integer keeps its collapsed x/X/z/Z char while the net
///   shift only APPENDS zeros ("X000", "X.00 ns") and falls back to numeric with
///   the unknown bits cleared once the shift cuts into the digits ("0.00us");
/// - the suffix appends verbatim; the result is right-justified in the explicit
///   `%Nt`/`%0t` width if given, else the min field width (default 20); a
///   NEGATIVE min width LEFT-justifies in |width|.
pub(crate) fn fmt_time_spec(
    st: &SimState,
    v: &Value,
    explicit_width: Option<usize>,
    min_zero: bool,
    left_just: bool,
) -> String {
    let (units_exp, prec, suffix, minw): (i32, usize, &str, i64) = match &st.timeformat {
        Some(tf) => (
            tf.units_exp,
            tf.prec as usize,
            tf.suffix.as_str(),
            tf.minw as i64,
        ),
        None => (st.global_prec_exp as i32, 0, "", 20),
    };
    let k = units_exp as i64 - st.global_prec_exp as i64;
    let m = st.cur_time_mult.max(1);
    // M is 10^(unit_exp − global_prec_exp) by the timescale-wiring contract, so
    // ilog10 recovers the exponent and the integer path stays exact string math.
    let mz = m.ilog10() as i64;
    debug_assert_eq!(10u64.checked_pow(mz as u32), Some(m));
    let shift = mz - k; // net decimal shift: ≥0 appends zeros, <0 cuts into digits
    let body = if v.is_real {
        // `$realtime`-style value: scale in f64, then %f-round to `prec` digits
        // (mirrors fmt_real's −0.0 normalization and non-finite C spelling).
        let x = v.to_f64().unwrap_or(0.0) * (m as f64) / 10f64.powi(k as i32);
        let x = if x == 0.0 { 0.0 } else { x };
        if x.is_finite() {
            format!("{x:.prec$}")
        } else {
            nonfinite_c(x)
        }
    } else if let Some(c) = unknown_group_char(v, 0, v.width) {
        if shift >= 0 {
            // Collapsed unknown char + appended zeros + zero-filled fraction.
            let frac = if prec > 0 {
                format!(".{}", "0".repeat(prec))
            } else {
                String::new()
            };
            format!("{c}{}{frac}", "0".repeat(shift as usize))
        } else {
            // Scale-down goes numeric with unknown bits cleared (iverilog live).
            let mut cleared = v.clone();
            for i in 0..cleared.val.len() {
                let u = cleared.unk[i];
                cleared.val[i] &= !u;
                cleared.unk[i] = 0;
            }
            time_digits_shift(&fmt_dec(&cleared), shift, prec)
        }
    } else {
        time_digits_shift(&fmt_dec(v), shift, prec)
    };
    let full = format!("{body}{suffix}");
    let width: i64 = explicit_width.map(|w| w as i64).unwrap_or(minw);
    let n = full.chars().count() as i64;
    if left_just {
        // `%-Nt`: left-justify (space right-pad) in |width|, overriding `0`.
        let aw = width.abs();
        if n < aw {
            return format!("{full}{}", " ".repeat((aw - n) as usize));
        }
    } else if width >= 0 {
        if n < width {
            // `%0Nt`: zero-fill; zeros go before any sign char (iverilog-pinned:
            // `%08t` of -1000 → "000-1000", not sign-aware like `%0Nd`).
            let fill = if min_zero { "0" } else { " " };
            return format!("{}{full}", fill.repeat((width - n) as usize));
        }
    } else if n < -width {
        // Negative min width ⇒ left-justify in |width| (iverilog-pinned).
        return format!("{full}{}", " ".repeat((-width - n) as usize));
    }
    full
}

/// Shift the decimal point of an exact decimal string (optionally `-`-signed) by
/// `shift` places (≥0 appends zeros; <0 inserts a point) and TRUNCATE/zero-fill
/// the fraction to `prec` digits — the iverilog `%t` integer behavior (it never
/// rounds: 9.995 @ prec 1 renders "9.9").
pub(crate) fn time_digits_shift(digits: &str, shift: i64, prec: usize) -> String {
    let (sign, mag) = match digits.strip_prefix('-') {
        Some(m) => ("-", m),
        None => ("", digits),
    };
    let body = if shift >= 0 {
        // A zero value stays "0" — appending scale zeros would render "0000"
        // (iverilog-pinned: `%t` of $time=0 in a 1ns/1ps module is "0").
        let int_part = if mag == "0" {
            mag.to_string()
        } else {
            format!("{mag}{}", "0".repeat(shift as usize))
        };
        if prec > 0 {
            format!("{int_part}.{}", "0".repeat(prec))
        } else {
            int_part
        }
    } else {
        let p = (-shift) as usize;
        // Left-pad so at least one integer digit remains ("2" @ p=3 → "0002").
        let padded = if mag.len() <= p {
            format!("{}{mag}", "0".repeat(p + 1 - mag.len()))
        } else {
            mag.to_string()
        };
        let cut = padded.len() - p;
        if prec == 0 {
            padded[..cut].to_string()
        } else {
            let frac: String = padded[cut..]
                .chars()
                .chain(std::iter::repeat('0'))
                .take(prec)
                .collect();
            format!("{}.{frac}", &padded[..cut])
        }
    };
    format!("{sign}{body}")
}

/// `$timeformat` executor (§21.3.2): 0 args resets the defaults; 4 args set
/// (units, precision, suffix, min width). Elaborate guarantees the 0/4 arity
/// (1–3 args are a compile-time error, iverilog parity). Values are evaluated
/// at EXECUTION time (runtime-variable args are legal). A negative precision
/// clamps to 0. Units and min width are taken arithmetically like iverilog, but
/// each caps at a hostile-input bound (|units−precision| ≤ 64 decimal places,
/// |min width| ≤ 4096, precision ≤ 64 digits — the WIDE-ARITH-CAP pattern):
/// `$timeformat(-2_000_000_000, …)` would otherwise make `%t` allocate a
/// multi-GiB zero string. The caps sit far above every physical unit (the IEEE
/// table spans 2…−15) so no real testbench can observe them.
/// The suffix is %s-coerced NOW: a string-domain value keeps its exact bytes; a
/// packed value renders its NUL-stripped ASCII form (8'h6E → "n").
/// OBS-3: a `$vita_stage("label", vals…)` call — a no-op `Display` intercepted by
/// StmtId. Pure no-op unless `+STAGE_TRACE` armed `stage_enabled`; then append a
/// `stage.jsonl` line `{v,t,kind:"stage",label,idx,vals[]}` — arg[0] is the label
/// (string), args[1..] are the values (each formatted like `$display %0d`, as JSON
/// strings so x/z is representable). NEVER prints (a stage is a structured record).
///
/// Slice #6 threads the reader. The RAIL needs no routing — `stage_lines`,
/// `stage_idx` and `stage_enabled` are `SimState`, one object both kernels
/// borrow, and `$vita_stage` is an explicit CALL SITE rather than a change hook
/// (unlike `--probe`, which rides `note_change`). What was store-bound was the
/// two argument reads: an unthreaded label and value list record whatever the
/// ENGINE's slots hold, so a native run's `stage.jsonl` would be a trace of a
/// store the design never wrote — a G2 artifact that is silently wrong rather
/// than absent, the same failure mode A7's `coverage.json` had.
pub(crate) fn run_vita_stage<N: crate::eval::NetReader + ?Sized>(
    sched: &mut Scheduler,
    nets: Option<&N>,
    args: &[u32],
) -> Ctl {
    if !sched.st.stage_enabled {
        return Ctl::Continue; // no capture without +STAGE_TRACE
    }
    let label = args
        .first()
        .map(|&a| {
            let v = crate::builtins::eval_task_arg(sched, nets, a);
            String::from_utf8_lossy(&crate::eval::value_str_bytes(&v)).into_owned()
        })
        .unwrap_or_default();
    let vals: Vec<String> = args
        .iter()
        .skip(1)
        .map(|&a| fmt_dec(&crate::builtins::eval_task_arg(sched, nets, a)))
        .collect();
    let now = sched.st.now;
    let idx = sched.st.stage_idx;
    sched.st.stage_idx += 1;
    let mut line = String::from("{\"v\":1,\"t\":");
    line.push_str(&now.to_string());
    line.push_str(",\"kind\":\"stage\",\"label\":");
    crate::json_push_str(&mut line, &label);
    line.push_str(",\"idx\":");
    line.push_str(&idx.to_string());
    line.push_str(",\"vals\":[");
    for (i, v) in vals.iter().enumerate() {
        if i > 0 {
            line.push(',');
        }
        crate::json_push_str(&mut line, v);
    }
    line.push_str("]}");
    sched.st.stage_lines.push(line);
    Ctl::Continue
}

/// `$timeformat`, optionally against an alternate net store.
///
/// Its arguments are deliberately allowed to be runtime values (iverilog-pinned),
/// so they are net reads that never touch the format engine — a `$timeformat`
/// with variable units read the ENGINE's store while the rest of a tier-3 run
/// read the arena. It also cannot be refused by task id: `$timeformat` lowers to
/// a `Display` plus a `timeformat_stmts` sid, so a refusal keyed on `which` is
/// structurally the wrong key. Threading it is what removes both problems.
pub(crate) fn run_timeformat_with<N: crate::eval::NetReader + ?Sized>(
    sched: &mut Scheduler,
    nets: Option<&N>,
    args: &[u32],
) -> Ctl {
    if args.is_empty() {
        sched.st.timeformat = None;
        return Ctl::Continue;
    }
    fn int_arg<N: crate::eval::NetReader + ?Sized>(
        sched: &Scheduler,
        nets: Option<&N>,
        args: &[u32],
        i: usize,
    ) -> i64 {
        args.get(i)
            .map(|&e| {
                crate::builtins::eval_task_arg(sched, nets, e)
                    .to_i128_signed()
                    .unwrap_or(0)
                    .clamp(i64::MIN as i128, i64::MAX as i128) as i64
            })
            .unwrap_or(0)
    }
    let gp = sched.st.global_prec_exp as i64;
    let units_exp = int_arg(sched, nets, args, 0).clamp(gp - 64, gp + 64) as i32;
    let prec = int_arg(sched, nets, args, 1).clamp(0, 64) as u32;
    let suffix = match args.get(2).copied() {
        // String LITERAL: exact text. Every OTHER expr — including a NUMERIC
        // literal — goes through the same value path (`fmt_packed_chars_min`),
        // so a literal 8'h6E and a reg holding 8'h6E render the identical "n"
        // (soundness review Q7: the old const_string route stripped trailing
        // NULs while the value route spaces them — value-dependent divergence).
        Some(eid) => match str_const_of_expr(sched.st, eid) {
            Some(text) => text,
            None => {
                let v = crate::builtins::eval_task_arg(sched, nets, eid);
                if v.is_str {
                    String::from_utf8_lossy(&v.to_str_bytes()).into_owned()
                } else {
                    fmt_packed_chars_min(&v)
                }
            }
        },
        None => String::new(),
    };
    let minw = int_arg(sched, nets, args, 3).clamp(-4096, 4096) as i32;
    sched.st.timeformat = Some(crate::state::TfState {
        units_exp,
        prec,
        suffix,
        minw,
    });
    Ctl::Continue
}

/// C/iverilog spelling of a non-finite f64: lowercase `nan` (any sign — iverilog
/// never prints `-nan`) and `inf`/`-inf`. Rust's Display gives `NaN`, so every
/// real formatter must coerce here for `$display` parity (a silent-wrong
/// otherwise — caught by the N6 real-math domain-error oracle).
pub(crate) fn nonfinite_c(x: f64) -> String {
    if x.is_nan() {
        "nan".to_string()
    } else if x < 0.0 {
        "-inf".to_string()
    } else {
        "inf".to_string()
    }
}

/// `%f`/`%e`/`%g` of a real Value (the arg may be an integer promoted to real).
/// `width`/`prec` are the optional field-width / precision modifiers (`%8.2f`).
pub(crate) fn fmt_real(
    v: &Value,
    spec: char,
    width: Option<usize>,
    prec: Option<usize>,
    left_just: bool,
    min_zero: bool,
    plus: bool,
) -> String {
    // Normalize IEEE-754 negative zero to +0.0 so every real spec displays it as
    // a plain "0" (the %g path / VCD `fmt_g` already do). iverilog prints a
    // constant/literal -0.0 as "0"/"0.000000"; matching it here keeps %f/%e/%g
    // internally consistent and was a $display silent-wrong (Rust's "{:.6}" of
    // -0.0 emits "-0.000000"). NOTE: a -0.0 deliberately CONSTRUCTED at runtime
    // (e.g. `-5.0*0.0`) also normalizes here — iverilog keeps that one's sign
    // ("-0"); that single corner is an accepted, documented divergence (negative
    // zero display is implementation-defined; IEEE 1800 does not pin it).
    let x = v.to_f64().unwrap_or(0.0);
    let x = if x == 0.0 { 0.0 } else { x };
    let body = if !x.is_finite() {
        // nan / inf / -inf — spelled the C way for every spec (incl. %g, which
        // fmt_g would also lowercase; routing here keeps all specs consistent).
        nonfinite_c(x)
    } else {
        match spec {
            'f' | 'F' => format!("{:.*}", prec.unwrap_or(6), x), // default 6 fractional digits
            'e' | 'E' => fmt_real_e(x, prec),
            'g' | 'G' => format_g(x, prec),
            _ => format!("{x}"),
        }
    };
    // `+` forces a leading sign on a non-negative value (iverilog: `%+f` of 3.14
    // → "+3.140000", of +inf → "+inf"; `nan` gets NO sign; a negative already
    // carries `-"). The sign counts toward the field width.
    let body = if plus && !body.starts_with('-') && body != "nan" {
        format!("+{body}")
    } else {
        body
    };
    if let Some(w) = width {
        if body.len() < w {
            let pad = w - body.len();
            return if left_just {
                // `-` right-pads with spaces (overrides `0`).
                format!("{body}{}", " ".repeat(pad))
            } else if min_zero && x.is_finite() {
                // `%0Nf` zero-pads AFTER any leading sign (`-`/`+`): "-3.14" →
                // "-003.14" (iverilog-pinned; a bare `%Nf` space-pads). The `0`
                // flag is IGNORED for a non-finite body (inf/nan) — C/iverilog
                // space-pad those regardless, so they fall to the else below.
                if let Some(rest) = body.strip_prefix(['-', '+']) {
                    format!("{}{}{rest}", &body[..1], "0".repeat(pad))
                } else {
                    format!("{}{body}", "0".repeat(pad))
                }
            } else {
                format!("{}{body}", " ".repeat(pad))
            };
        }
    }
    body
}

/// %e → C/printf/LRM form: `prec` mantissa fraction digits (default 6), signed
/// exponent zero-padded to AT LEAST 2 digits (`1.500000e+03`). Non-finite is
/// handled by the caller (`fmt_real` short-circuits via `nonfinite_c`); this
/// guard defends direct callers.
pub(crate) fn fmt_real_e(x: f64, prec: Option<usize>) -> String {
    if !x.is_finite() {
        return nonfinite_c(x); // inf / -inf / nan
    }
    let p = prec.unwrap_or(6);
    let s = format!("{x:.p$e}"); // e.g. "1.500000e3" or "1.234500e-5"
    let (mant, exp) = s.split_once('e').expect("rust {:e} always emits 'e'");
    let (sign, digits) = match exp.strip_prefix('-') {
        Some(d) => ('-', d),
        None => ('+', exp),
    };
    let padded = if digits.len() < 2 {
        format!("{digits:0>2}")
    } else {
        digits.to_string()
    };
    format!("{mant}e{sign}{padded}")
}

/// %g: shortest of %e/%f with trailing zeros stripped, per C/LRM. `prec` is the
/// total significant-digit precision P (default 6). REALG-DEDUP: delegates to the
/// single shared `vcd_writer::fmt_g` so the `$display %g` and VCD `%g` formatters
/// can never drift apart.
pub(crate) fn format_g(x: f64, prec: Option<usize>) -> String {
    vcd_writer::fmt_g(x, prec.unwrap_or(6).max(1) as i32)
}
