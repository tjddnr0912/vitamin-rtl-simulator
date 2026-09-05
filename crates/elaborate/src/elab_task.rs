//! §4.5.428: elaboration system tasks (IEEE 1800-2017 §20.11). The parser carries a
//! module-item `$info` / `$warning` / `$error` / `$fatal` as a synthetic `initial` whose
//! call is renamed under `hdl_ast::ELAB_TASK_PREFIX`; this runs it ONCE, at elaboration,
//! in the scope being elaborated — the message is rendered from CONSTANT arguments (a
//! parameter, a literal, `%m`), `$warning`/`$info` continue, `$error`/`$fatal` fail
//! elaboration (no simulation runs). No process is created. Both oracles evaluate these
//! at elaboration (iverilog accepts a single string argument only; verilator formats).

use super::*;

impl Elaborator<'_> {
    /// `true` when `p` is an elaboration task (run here); `false` = an ordinary process.
    pub(crate) fn try_elab_task(&mut self, p: &ast::ProceduralBlock) -> bool {
        let ast::Stmt::SysTaskCall { name, args, span } = &*p.body else {
            return false;
        };
        let Some(kind) = name.name.strip_prefix(hdl_ast::ELAB_TASK_PREFIX) else {
            return false;
        };
        let saved = self.cur_span;
        self.cur_span = Some(*span);
        // `$fatal([finish_number,] fmt, …)`: a leading NUMERIC argument is the finish
        // number (§20.11), not the message.
        let rest: &[ast::Expr] = if kind == "fatal"
            && args
                .first()
                .is_some_and(|a| self.const_str_in_scope(a).is_none())
        {
            args.get(1..).unwrap_or(&[])
        } else {
            args
        };
        let msg = match self.elab_task_message(rest) {
            Some(m) => m,
            None => {
                self.error(
                    MsgCode::ElabUnsupported,
                    &format!(
                        "an elaboration `${kind}` argument is not a constant this pass can \
                         render (a literal, a parameter, `%m` and the `%d %h %b %o %s %c` \
                         specs are)"
                    ),
                );
                self.cur_span = saved;
                return true;
            }
        };
        match kind {
            "info" => self.info_code(MsgCode::ElabUserInfo, &msg),
            "warning" => self.warn_code(MsgCode::ElabUserWarning, &msg),
            "error" => self.error(MsgCode::ElabUserError, &msg),
            _ => self.error(MsgCode::ElabUserFatal, &msg),
        }
        self.cur_span = saved;
        true
    }

    /// Render `fmt, args…` from constants. `None` when an argument is not constant.
    /// Field rules (`%5d`, `%-8s`, `%04h`, bare `%d` default width, `%s` of a packed
    /// value) are `diag::fmt`, the SAME functions the runtime renderer calls — a
    /// second renderer must inherit the runtime's rules, not re-derive them.
    fn elab_task_message(&mut self, args: &[ast::Expr]) -> Option<String> {
        let Some(first) = args.first() else {
            return Some(String::new());
        };
        // `const_str_in_scope` yields the RAW literal (quotes and escapes kept — it is
        // re-emitted as a string const elsewhere); decode it here.
        let fmt = crate::literal::parse_str_literal_text(&self.const_str_in_scope(first)?);
        let mut out = String::new();
        let mut ai = 1usize;
        let mut chars = fmt.chars().peekable();
        while let Some(c) = chars.next() {
            if c != '%' {
                out.push(c);
                continue;
            }
            let (left_just, plus, min_zero, field_width, _prec) =
                diag::fmt::parse_flags(&mut chars);
            let spec = chars.next()?;
            match spec {
                '%' => out.push('%'),
                'm' | 'M' => {
                    out.push_str(&diag::fmt::justify(
                        &self.display_prefix(),
                        field_width,
                        left_just,
                    ));
                }
                's' | 'S' => {
                    let a = args.get(ai)?;
                    ai += 1;
                    let content = match self.const_str_in_scope(a) {
                        Some(s) => crate::literal::parse_str_literal_text(&s),
                        None => {
                            let v = self.const_eval_in_scope(a)?;
                            let (w, _) = self.elab_arg_shape(a).unwrap_or((32, true));
                            let nbytes = (w as usize).div_ceil(8).max(1);
                            // MSB-first bytes; a constant reaches this pass as an i64, so
                            // bytes above the 8th are the zero extension.
                            let bytes: Vec<u8> = (0..nbytes)
                                .rev()
                                .map(|bi| {
                                    if bi >= 8 {
                                        0
                                    } else {
                                        ((v as u64) >> (bi * 8)) as u8
                                    }
                                })
                                .collect();
                            // `%0s` / `%-Ns` / `%Ns` strip leading NULs (runtime parity).
                            diag::fmt::packed_chars(&bytes, left_just || field_width.is_some())
                        }
                    };
                    out.push_str(&diag::fmt::justify(&content, field_width, left_just));
                }
                'd' | 'D' | 'h' | 'H' | 'x' | 'X' | 'b' | 'B' | 'o' | 'O' | 'c' | 'C' => {
                    let a = args.get(ai)?;
                    ai += 1;
                    let v = self.const_eval_in_scope(a)?;
                    // Review B F1: the argument's WIDTH and sign (a declared parameter's
                    // meta, a sized literal's own; 32-bit signed otherwise, as the runtime
                    // renderer reads an unsized value) — `%h` of a `logic signed [7:0]`
                    // −1 is `ff`, not sixteen `f`s, and `%b` pads to the width unless `%0b`.
                    let (w, signed) = self.elab_arg_shape(a).unwrap_or((32, true));
                    let mask: u64 = if w >= 64 { u64::MAX } else { (1u64 << w) - 1 };
                    let bits = (v as u64) & mask;
                    let radix = |bits: u64, per: u32| -> String {
                        let digits = (w as usize).div_ceil(per as usize).max(1);
                        let s = match per {
                            4 => format!("{bits:x}"),
                            3 => format!("{bits:o}"),
                            _ => format!("{bits:b}"),
                        };
                        format!("{s:0>digits$}")
                    };
                    match spec.to_ascii_lowercase() {
                        'd' => {
                            let mut text = if signed {
                                let sv = if w < 64 && (bits >> (w - 1)) & 1 == 1 {
                                    (bits | !mask) as i64
                                } else {
                                    bits as i64
                                };
                                sv.to_string()
                            } else {
                                bits.to_string()
                            };
                            if plus && !text.starts_with('-') {
                                text.insert(0, '+');
                            }
                            out.push_str(&diag::fmt::pad_dec(
                                &text,
                                min_zero,
                                field_width,
                                left_just,
                                diag::fmt::dec_field_width(w, signed),
                            ));
                        }
                        'h' | 'x' => out.push_str(&diag::fmt::pad_radix(
                            radix(bits, 4),
                            min_zero,
                            field_width,
                            left_just,
                        )),
                        'o' => out.push_str(&diag::fmt::pad_radix(
                            radix(bits, 3),
                            min_zero,
                            field_width,
                            left_just,
                        )),
                        'b' => out.push_str(&diag::fmt::pad_radix(
                            radix(bits, 1),
                            min_zero,
                            field_width,
                            left_just,
                        )),
                        _ => out.push_str(&diag::fmt::justify(
                            &char::from_u32((bits & 0xff) as u32)
                                .unwrap_or('?')
                                .to_string(),
                            field_width,
                            left_just,
                        )),
                    }
                }
                _ => return None,
            }
        }
        Some(out)
    }

    /// `(width, signed)` of an elaboration-task argument: a declared parameter's meta,
    /// a sized/based literal's own, else `None` (the caller assumes a 32-bit signed
    /// integer, the runtime renderer's reading of an unsized value).
    fn elab_arg_shape(&self, a: &ast::Expr) -> Option<(u32, bool)> {
        match &a.kind {
            ast::ExprKind::Paren { inner } => self.elab_arg_shape(inner),
            ast::ExprKind::Ident(p) if p.segments.len() == 1 => {
                self.walk_scopes(&p.segments[0].name, &self.param_meta)
            }
            ast::ExprKind::IntLit {
                kind: ast::IntLitKind::Sized,
                raw,
            } => {
                let (w, _) = raw.split_once('\'')?;
                let w: u32 = w.trim().parse().ok()?;
                let signed = raw
                    .split_once('\'')
                    .map(|(_, r)| matches!(r.chars().next(), Some('s') | Some('S')))
                    .unwrap_or(false);
                Some((w, signed))
            }
            _ => None,
        }
    }
}
