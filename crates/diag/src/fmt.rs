//! Pure `$display`-family field rules shared by the runtime renderer (sim-engine
//! `render_template`) and the elaboration-task renderer (elaborate `elab_task`). One
//! definition: a second renderer that re-derives these rules drifts (§4.5.428 review).
//! Every function is value-free — it takes digit strings and widths, never a `Value`.

/// Default `%d` field width of an `n`-bit operand (iverilog-pinned). A signed field
/// holds a sign plus the digits of the most-negative magnitude 2^(n-1) (8-bit →
/// "-128" = 4, 32-bit → 11); this is NOT unsigned_width + 1 (10-bit: "-512" and 1023
/// are both 4). A 1-bit signed value is width 1, not 2. `n == 0` → 1.
pub fn dec_field_width(n: u32, signed: bool) -> usize {
    if n == 0 {
        return 1;
    }
    if signed && n > 1 {
        if n <= 128 {
            let mag: u128 = 1u128 << (n - 1);
            1 + mag.to_string().len()
        } else {
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

/// Pad `content` to `field_width` (a MINIMUM — never truncates): right-justified, or
/// left-justified (space right-pad) under the `-` flag. `None` → verbatim. The
/// plain-content specs (`%c` `%v` `%m` `%s`).
pub fn justify(content: &str, field_width: Option<usize>, left_just: bool) -> String {
    match field_width {
        Some(n) => {
            let clen = content.chars().count();
            if clen < n {
                let pad = " ".repeat(n - clen);
                if left_just {
                    format!("{content}{pad}")
                } else {
                    format!("{pad}{content}")
                }
            } else {
                content.to_string()
            }
        }
        None => content.to_string(),
    }
}

/// `%d` field: `s` is the decimal text (sign included). `%0d` → minimal; `%Nd` →
/// space-pad to N; `%0Nd` → zero-pad AFTER a leading sign ("-42" → "-00042");
/// `-` right-pads with spaces and overrides `0`; bare `%d` → `default_width`.
pub fn pad_dec(
    s: &str,
    min_zero: bool,
    field_width: Option<usize>,
    left_just: bool,
    default_width: usize,
) -> String {
    let fw = match (min_zero, field_width) {
        (true, Some(0)) => 0,
        (_, Some(n)) => n,
        (_, None) => default_width,
    };
    if s.len() >= fw {
        return s.to_string();
    }
    let pad = fw - s.len();
    if left_just {
        format!("{s}{}", " ".repeat(pad))
    } else if min_zero {
        if let Some(rest) = s.strip_prefix(['-', '+']) {
            format!("{}{}{rest}", &s[..1], "0".repeat(pad))
        } else {
            format!("{}{s}", "0".repeat(pad))
        }
    } else {
        format!("{}{s}", " ".repeat(pad))
    }
}

/// `%h`/`%o`/`%b` field over the FULL-width digit string `s` (leading zeros kept):
/// `%0h` → strip leading zeros (keep ≥1 digit); `%0Nh` → zero-pad to N; `%Nh` →
/// space-pad to N; `-` right-pads with spaces (overrides `0`).
pub fn pad_radix(s: String, min_zero: bool, field_width: Option<usize>, left_just: bool) -> String {
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
                format!("{base}{}", " ".repeat(n))
            } else {
                let pad = if min_zero { '0' } else { ' ' };
                let mut p: String = std::iter::repeat_n(pad, n).collect();
                p.push_str(&base);
                p
            }
        }
        _ => base,
    }
}

/// `%s` of a PACKED value given its bytes MSB-first (ceil(width/8) of them, ≥1): a NUL
/// byte renders as a space. With `min` (`%0s`, or any explicit width / `-`) the
/// LEADING NUL bytes are dropped instead; an all-NUL value is then "".
pub fn packed_chars(bytes_msb_first: &[u8], min: bool) -> String {
    let mut s = String::with_capacity(bytes_msb_first.len());
    let mut started = !min;
    for &byte in bytes_msb_first {
        if !started {
            if byte == 0 {
                continue;
            }
            started = true;
        }
        s.push(if byte == 0 { ' ' } else { byte as char });
    }
    s
}

/// Parse the flags/width run after `%` (IEEE §21.2.1.3, iverilog-pinned): `-`
/// (left-justify; must precede the digits), `+`, a leading `0` (zero-pad; also
/// counts as the width run so bare `%0d` yields `Some(0)`), digits, and `.prec`.
/// Returns `(left_just, plus, min_zero, field_width, precision)` and leaves `chars`
/// at the spec letter.
pub fn parse_flags<I: Iterator<Item = char>>(
    chars: &mut std::iter::Peekable<I>,
) -> (bool, bool, bool, Option<usize>, Option<usize>) {
    let (mut left_just, mut plus, mut min_zero) = (false, false, false);
    let mut width_digits = String::new();
    while let Some(&d) = chars.peek() {
        if d == '-' && width_digits.is_empty() {
            left_just = true;
            chars.next();
        } else if d == '+' && width_digits.is_empty() {
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
    let mut precision = None;
    if chars.peek() == Some(&'.') {
        chars.next();
        let mut p = String::new();
        while let Some(&d) = chars.peek() {
            if d.is_ascii_digit() {
                p.push(d);
                chars.next();
            } else {
                break;
            }
        }
        precision = Some(p.parse::<usize>().unwrap_or(0));
    }
    let field_width = width_digits
        .trim_start_matches('0')
        .parse::<usize>()
        .ok()
        .or_else(|| {
            if !width_digits.is_empty() && width_digits.chars().all(|c| c == '0') {
                Some(0)
            } else {
                None
            }
        });
    (left_just, plus, min_zero, field_width, precision)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dec_width_pins() {
        assert_eq!(dec_field_width(8, true), 4);
        assert_eq!(dec_field_width(8, false), 3);
        assert_eq!(dec_field_width(10, true), 4);
        assert_eq!(dec_field_width(32, true), 11);
        assert_eq!(dec_field_width(1, true), 1);
        assert_eq!(dec_field_width(0, false), 1);
    }

    #[test]
    fn pad_rules() {
        assert_eq!(pad_dec("-42", true, Some(6), false, 0), "-00042");
        assert_eq!(pad_dec("42", false, Some(5), true, 0), "42   ");
        assert_eq!(pad_dec("4", false, None, false, 3), "  4");
        assert_eq!(pad_dec("4", true, Some(0), false, 3), "4");
        assert_eq!(pad_radix("0041".into(), true, Some(0), false), "41");
        assert_eq!(pad_radix("41".into(), true, Some(4), false), "0041");
        assert_eq!(pad_radix("0a".into(), false, Some(4), true), "0a  ");
        assert_eq!(packed_chars(&[0, b'h', b'i'], false), " hi");
        assert_eq!(packed_chars(&[0, b'h', 0, b'i'], true), "h i");
        assert_eq!(packed_chars(&[0, 0], true), "");
        let mut it = "-05d".chars().peekable();
        assert_eq!(parse_flags(&mut it), (true, false, true, Some(5), None));
        assert_eq!(it.next(), Some('d'));
        let mut it = "0h".chars().peekable();
        assert_eq!(parse_flags(&mut it), (false, false, true, Some(0), None));
        let mut it = "8.3f".chars().peekable();
        assert_eq!(
            parse_flags(&mut it),
            (false, false, false, Some(8), Some(3))
        );
    }
}
