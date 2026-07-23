//! split part of `eval` (mechanical move).

use super::*;

impl<N: NetReader> EvalCtx<'_, N> {
    // ── SysFunc ────────────────────────────────────────────────────────────

    pub(crate) fn eval_sysfunc(&self, which: SysFuncId, args: &[u32]) -> Value {
        match which {
            // v5 (C)-③: `.size()` of a dyn handle. The arg is the handle's
            // Signal expr — resolve the NetId and ask the reader; a non-handle
            // (or a non-engine reader) X-poisons defensively, never panics.
            SysFuncId::DynSize => {
                let net = args
                    .first()
                    .and_then(|&a| match self.ir.exprs.get(a as usize) {
                        Some(Expr::Signal { net, word: None }) => Some(*net),
                        _ => None,
                    });
                match net.and_then(|n| self.nets.dyn_size(n)) {
                    Some(n) => {
                        let mut v = Value::zeros(32, true);
                        v.val[0] = n.min(i32::MAX as u64);
                        v
                    }
                    None => Value::xs(32, true),
                }
            }
            // v5 ④: queue pops are SIDE-EFFECTING — legal only as the DIRECT
            // rhs of a blocking assign, where the executor intercepts them
            // BEFORE eval (`StmtEffect::QPop`). Reaching THIS arm means an
            // unsupported placement (NBA rhs, nested expr, $monitor arg, …):
            // degrade LOUDLY to element-width X and do NOT pop — eval is the
            // pure READ phase (P7a) and must never mutate the heap.
            SysFuncId::QPopBack | SysFuncId::QPopFront => {
                let net = args
                    .first()
                    .and_then(|&a| match self.ir.exprs.get(a as usize) {
                        Some(Expr::Signal { net, word: None }) => Some(*net),
                        _ => None,
                    });
                match net.and_then(|n| self.ir.nets.get(n as usize).map(|nv| (n, nv))) {
                    Some((n, nv)) => {
                        self.nets.dyn_warn(
                            n,
                            "queue pop outside a direct blocking assign (X; not popped)",
                        );
                        Value::xs(nv.width.max(1), nv.signed)
                    }
                    None => Value::xs(32, false),
                }
            }
            // v6: assoc iteration methods WRITE their ref key argument — like
            // the pops they are legal only as the DIRECT rhs of a blocking
            // assign (statement-level intercept). Reaching THIS arm is an
            // unsupported placement: X status, no key write, loud.
            SysFuncId::AssocFirst
            | SysFuncId::AssocNext
            | SysFuncId::AssocLast
            | SysFuncId::AssocPrev => {
                if let Some(net) = args
                    .first()
                    .and_then(|&a| match self.ir.exprs.get(a as usize) {
                        Some(Expr::Signal { net, word: None }) => Some(*net),
                        _ => None,
                    })
                {
                    self.nets.dyn_warn(
                        net,
                        "assoc first/next/last/prev outside a direct blocking assign (X; key not written)",
                    );
                }
                Value::xs(32, true)
            }
            // v5 ⑤: `a.num()` — the entry count, same recipe as DynSize (the
            // reader's `dyn_size` covers assoc: num == size, IEEE §7.9.1).
            SysFuncId::AssocNum => {
                let net = args
                    .first()
                    .and_then(|&a| match self.ir.exprs.get(a as usize) {
                        Some(Expr::Signal { net, word: None }) => Some(*net),
                        _ => None,
                    });
                match net.and_then(|n| self.nets.dyn_size(n)) {
                    Some(n) => {
                        let mut v = Value::zeros(32, true);
                        v.val[0] = n.min(i32::MAX as u64);
                        v
                    }
                    None => Value::xs(32, true),
                }
            }
            // v5 ⑤: `a.exists(k)` — args = [handle, key]. PURE (a query, no
            // heap mutation), so unlike the pops it lives in the eval arm and
            // is VM-correct by construction. 1/0; X/Z key matches nothing.
            SysFuncId::AssocExists => {
                let net = args
                    .first()
                    .and_then(|&a| match self.ir.exprs.get(a as usize) {
                        Some(Expr::Signal { net, word: None }) => Some(*net),
                        _ => None,
                    });
                // v6: dispatch on the handle's key domain (string vs i64).
                let hit = net.and_then(|n| {
                    if self.nets.is_assoc_str(n) {
                        let key = args.get(1).and_then(|&k| self.assoc_str_key(k));
                        self.nets.assoc_str_exists(n, &key)
                    } else {
                        let key = args.get(1).and_then(|&k| self.assoc_key(k));
                        self.nets.assoc_exists(n, key)
                    }
                });
                match hit {
                    Some(b) => {
                        let mut v = Value::zeros(1, false);
                        v.val[0] = b as u64;
                        v
                    }
                    None => Value::xs(1, false),
                }
            }
            // ⓑ-breadth (v15/v17): array reduction methods (IEEE §7.12.3). PURE —
            // a read query that left-folds the element snapshot. Without a `with`
            // clause the result takes the element (handle) type and folds the raw
            // elements; WITH a `with (expr)` clause (args[1]) the result takes the
            // with-expr type and folds `expr(item)` per element. Empty → 0; x/z
            // propagates through the normal 4-state arithmetic/bitwise.
            SysFuncId::ArrSum
            | SysFuncId::ArrProduct
            | SysFuncId::ArrAnd
            | SysFuncId::ArrOr
            | SysFuncId::ArrXor => {
                let net = args
                    .first()
                    .and_then(|&a| match self.ir.exprs.get(a as usize) {
                        Some(Expr::Signal { net, word: None }) => Some(*net),
                        _ => None,
                    });
                let with_eid = args.get(1).copied();
                let elem_signed = net
                    .and_then(|n| self.ir.nets.get(n as usize))
                    .map(|nv| nv.signed)
                    .unwrap_or(true);
                // result type: with-expr width/sign, else element type
                let (w, signed) = match with_eid {
                    Some(weid) => {
                        let sw = self.wt.get(weid);
                        (sw.width.max(1), sw.signed)
                    }
                    None => net
                        .and_then(|n| self.ir.nets.get(n as usize))
                        .map(|nv| (nv.width.max(1), nv.signed))
                        .unwrap_or((32, true)),
                };
                let fold = |this: &Self, acc: &Value, e: &Value| -> Value {
                    match which {
                        SysFuncId::ArrSum => this.arith(BinOp::Add, acc, e),
                        SysFuncId::ArrProduct => this.arith(BinOp::Mul, acc, e),
                        SysFuncId::ArrAnd => this.bitwise(acc, e, and_w),
                        SysFuncId::ArrOr => this.bitwise(acc, e, or_w),
                        _ => this.bitwise(acc, e, xor_w),
                    }
                };
                match net.and_then(|n| self.nets.dyn_values(n)) {
                    Some(elems) if !elems.is_empty() => {
                        let saved = self.nets.swap_array_item(None);
                        let mut acc: Option<Value> = None;
                        for (i, e) in elems.iter().enumerate() {
                            let v = match with_eid {
                                Some(weid) => {
                                    self.nets.swap_array_item(Some((e.clone(), i as u64)));
                                    self.eval(weid)
                                }
                                None => {
                                    let mut x = e.clone();
                                    x.signed = elem_signed;
                                    x
                                }
                            };
                            acc = Some(match acc {
                                None => v,
                                Some(a) => fold(self, &a, &v),
                            });
                        }
                        self.nets.swap_array_item(saved);
                        acc.unwrap_or_else(|| Value::zeros(w, signed))
                            .resize_keep_sign(w, signed)
                    }
                    // empty array → element-type 0 (documented pin)
                    Some(_) => Value::zeros(w, signed),
                    // non-handle / string handle → defensive X
                    None => Value::xs(w, signed),
                }
            }
            SysFuncId::Time => {
                // $time: current time in the CALLING module's units, ROUNDED to the
                // nearest integer — IEEE 1800-2017 §20.3.1: "scaled to the time unit
                // of the module ... and rounded to an integer value" (NOT truncated).
                // `now` is global-precision ticks; scale by the module multiplier M
                // with round-half-up (= round-half-away-from-zero, since time ≥ 0 —
                // matches iverilog: 1.5→2, 2.5→3, 4.6→5).
                let m = self.time_mult.max(1);
                let mut v = Value::zeros(64, false);
                v.val[0] = self.now.saturating_add(m / 2) / m;
                v
            }
            SysFuncId::Realtime => {
                // $realtime: same as $time but keeping the sub-unit fraction.
                let m = self.time_mult.max(1) as f64;
                Value::from_f64(self.now as f64 / m)
            }
            SysFuncId::Rtoi => {
                // real → int, TRUNCATE toward zero. Result is a plain integer Value.
                let x = self.eval(args[0]).to_f64().unwrap_or(0.0);
                Value::from_i128(x.trunc() as i128, 32, true)
            }
            SysFuncId::Itor => {
                // int → real, exact convert.
                let i = self.eval(args[0]).to_i128_signed().unwrap_or(0);
                Value::from_f64(i as f64)
            }
            SysFuncId::RealToBits => {
                // real → 64-bit vector (raw IEEE bits). val[0] already holds
                // to_bits(); clear is_real so it reads as a plain 64-bit vector.
                let mut v = self.eval(args[0]);
                v.is_real = false;
                v.signed = false;
                v.width = 64;
                v
            }
            SysFuncId::BitsToReal => {
                // 64-bit vector → real. Same bits, set is_real. X/Z → NaN poison
                // (§6.2 "cannot convert X/Z to real") rather than a fabricated real.
                let src = self.eval(args[0]);
                if src.has_xz() {
                    return Value::from_f64(f64::NAN);
                }
                let mut v = src;
                v.is_real = true;
                v.signed = true;
                v.width = 64;
                v
            }
            SysFuncId::Signed => {
                let mut a = self.eval(args[0]);
                a.signed = true;
                a
            }
            SysFuncId::Unsigned => {
                let mut a = self.eval(args[0]);
                a.signed = false;
                a
            }
            SysFuncId::Clog2 => {
                // Exact at any width: $clog2(n) = highest set bit of (n-1) + 1.
                // Word-wise so a >64-bit argument is not truncated (P0-4).
                let mut a = self.eval(args[0]);
                if a.is_real {
                    // $clog2 of a real rounds to the nearest integer (ties away
                    // from zero, per IEEE real→int) and then computes clog2 —
                    // iverilog: $clog2(100.0) == $clog2(100) == 7. Reading a.val
                    // directly runs clog2 over the IEEE-754 bit pattern (silently
                    // wrong: 63). A negative / non-finite / ≥2^64 "size" has no
                    // meaningful clog2 (iverilog's negative value is a 32-bit-wrap
                    // artifact), so it yields X — never a confident wrong number.
                    let r = a.to_f64().unwrap_or(f64::NAN).round();
                    // Strict `< 2^64`: the low 64 bits must hold the value for the
                    // word-logic below. `u64::MAX as f64` rounds UP to 2^64, so a
                    // `<=` guard would let exactly 2^64 through, where `from_i128`
                    // masks it to 0 → clog2(0)=0 (a confident wrong bus width).
                    if !(r.is_finite() && r >= 0.0 && r < 2.0_f64.powi(64)) {
                        return Value::xs(32, false);
                    }
                    a = Value::from_i128(r as i128, 64, false);
                }
                if a.has_xz() {
                    return Value::xs(32, false);
                }
                let mut words: Vec<u64> = a.val.iter().copied().collect();
                let le_one = {
                    let w0 = words.first().copied().unwrap_or(0);
                    w0 <= 1 && words.iter().skip(1).all(|&w| w == 0)
                };
                let bits = if le_one {
                    0u64
                } else {
                    // n -= 1 (word-wise borrow), then locate the highest set bit.
                    for w in words.iter_mut() {
                        if *w > 0 {
                            *w -= 1;
                            break;
                        }
                        *w = u64::MAX;
                    }
                    let (k, top) = words
                        .iter()
                        .enumerate()
                        .rev()
                        .find(|(_, &w)| w != 0)
                        .map(|(k, &w)| (k as u64, w))
                        .unwrap_or((0, 0));
                    64 * k + (64 - top.leading_zeros() as u64)
                };
                let mut v = Value::zeros(32, false);
                v.val[0] = bits;
                v
            }
            // v7 bit-vector predicates (iverilog-pinned): x/z bits never count
            // as 1; the result is always KNOWN (a 1x1z operand gives co=2,
            // onehot=0 — not x). Word-parallel popcount over val & !unk.
            SysFuncId::CountOnes
            | SysFuncId::OneHot
            | SysFuncId::OneHot0
            | SysFuncId::IsUnknown => {
                let Some(&a0) = args.first() else {
                    // malformed arity (defensive — elaborate emits 1 arg)
                    return match which {
                        SysFuncId::CountOnes => Value::xs(32, true),
                        _ => Value::xs(1, false),
                    };
                };
                let a = self.eval(a0);
                let mut ones: u64 = 0;
                let mut unk_any = false;
                for k in 0..nwords(a.width).max(1) {
                    let v = a.val.get(k).copied().unwrap_or(0);
                    let u = a.unk.get(k).copied().unwrap_or(0);
                    ones += (v & !u).count_ones() as u64;
                    unk_any |= u != 0;
                }
                match which {
                    SysFuncId::CountOnes => {
                        let mut v = Value::zeros(32, true);
                        v.val[0] = ones;
                        v
                    }
                    SysFuncId::OneHot => Value::logic(ones == 1),
                    SysFuncId::OneHot0 => Value::logic(ones <= 1),
                    _ => Value::logic(unk_any),
                }
            }
            // v7 `$random` — no-arg form only (the seeded form is a
            // statement-level intercept that writes the ref seed back;
            // elaborate rejects any other seeded placement, so an arg here
            // is a hand-built IR — defensive X, no state advance).
            SysFuncId::Random => {
                if !args.is_empty() {
                    return Value::xs(32, true);
                }
                let mut s = self.rng.random.get();
                let r = crate::rng::annex_n_random(&mut s);
                self.rng.random.set(s);
                Value::from_i128(r as i128, 32, true)
            }
            // v7 `$urandom[(seed)]` — the optional seed is INPUT-only (IEEE
            // §18.13.1: not written back, unlike $random): it re-seeds the
            // generator, then the draw proceeds. X/Z seed = 0.
            SysFuncId::Urandom => {
                if let Some(&a0) = args.first() {
                    let seed = self.eval(a0).to_u64().unwrap_or(0);
                    self.rng.urandom.set(seed);
                }
                let mut st = self.rng.urandom.get();
                let r = crate::rng::splitmix_urandom(&mut st);
                self.rng.urandom.set(st);
                let mut v = Value::zeros(32, false);
                v.val[0] = r as u64;
                v
            }
            // v7 `$urandom_range(maxval[, minval])` — inclusive; swapped
            // bounds auto-correct (IEEE §18.13.3). X/Z bound → X result.
            SysFuncId::UrandomRange => {
                let bound = |i: usize| -> Option<u32> {
                    args.get(i)
                        .map(|&a| self.eval(a))
                        .filter(|v| !v.has_xz())
                        .and_then(|v| v.to_u64())
                        .map(|v| v as u32)
                };
                let Some(b0) = bound(0) else {
                    return Value::xs(32, false);
                };
                let b1 = if args.len() > 1 {
                    match bound(1) {
                        Some(b) => b,
                        None => return Value::xs(32, false),
                    }
                } else {
                    0
                };
                let (lo, hi) = if b0 <= b1 { (b0, b1) } else { (b1, b0) };
                let range = (hi - lo) as u64 + 1;
                let mut st = self.rng.urandom.get();
                let r = crate::rng::splitmix_urandom(&mut st);
                self.rng.urandom.set(st);
                let mut v = Value::zeros(32, false);
                v.val[0] = lo as u64 + (r as u64 % range);
                v
            }
            // v7 `$stime` — the low 32 bits of `$time` (1364 §17.7.2). Like `$time`,
            // the module-unit scaling ROUNDS to nearest (round-half-up) BEFORE the
            // truncation to unsigned 32 bit — not a bare truncating divide.
            SysFuncId::Stime => {
                let m = self.time_mult.max(1);
                let mut v = Value::zeros(32, false);
                v.val[0] = (self.now.saturating_add(m / 2) / m) & 0xffff_ffff;
                v
            }
            // v7 `$test$plusargs(query)` — true iff some plusarg STARTS WITH
            // the query (iverilog-pinned prefix rule). The query is a string
            // LITERAL (elaborate enforces); a hand-built non-literal is X.
            SysFuncId::TestPlusargs => {
                let q = args.first().and_then(|&a| self.const_str_arg(a));
                match q {
                    Some(q) => {
                        let hit = self.plusargs.iter().any(|p| p.starts_with(&q));
                        Value::from_i128(hit as i128, 32, true)
                    }
                    None => Value::xs(32, true),
                }
            }
            // v7 P2-C string methods. args[0] = the handle's Signal (elaborate
            // contract); a malformed handle X-poisons, never panics. Methods
            // are PURE heap reads (putc is the one mutator = SysTask).
            SysFuncId::StrLen => {
                let b = self.handle_str_bytes(args.first());
                match b {
                    Some(b) => Value::from_i128(b.len() as i128, 32, true),
                    None => Value::xs(32, true),
                }
            }
            SysFuncId::StrGetC => {
                let b = self.handle_str_bytes(args.first());
                let i = args.get(1).and_then(|&a| self.eval(a).to_u64());
                match (b, i) {
                    // OOB index reads 0 (IEEE §6.16.2).
                    (Some(b), Some(i)) => {
                        let c = b.get(i as usize).copied().unwrap_or(0);
                        let mut v = Value::zeros(8, false);
                        v.val[0] = c as u64;
                        v
                    }
                    _ => Value::xs(8, false),
                }
            }
            SysFuncId::StrSubstr => {
                let b = self.handle_str_bytes(args.first());
                let i = args.get(1).and_then(|&a| self.eval(a).to_u64());
                let j = args.get(2).and_then(|&a| self.eval(a).to_u64());
                match (b, i, j) {
                    // inclusive [i..=j]; any invalid range = "" (IEEE §6.16.8).
                    (Some(b), Some(i), Some(j)) => {
                        let (i, j) = (i as usize, j as usize);
                        if i > j || j >= b.len() {
                            Value::from_str_bytes(&[])
                        } else {
                            Value::from_str_bytes(&b[i..=j])
                        }
                    }
                    _ => Value::from_str_bytes(&[]),
                }
            }
            SysFuncId::StrToUpper | SysFuncId::StrToLower => {
                let b = self.handle_str_bytes(args.first());
                match b {
                    Some(b) => {
                        let mapped: Vec<u8> = if matches!(which, SysFuncId::StrToUpper) {
                            b.iter().map(|c| c.to_ascii_uppercase()).collect()
                        } else {
                            b.iter().map(|c| c.to_ascii_lowercase()).collect()
                        };
                        Value::from_str_bytes(&mapped)
                    }
                    None => Value::from_str_bytes(&[]),
                }
            }
            // lexicographic compare of the two args' DENOTED byte strings
            // (§6.16 conversion: leading NULs strip) — backs both the
            // `.compare()` method and every string relational operator.
            SysFuncId::StrCmp => {
                let a = args.first().map(|&a| self.eval(a).to_str_bytes());
                let b = args.get(1).map(|&a| self.eval(a).to_str_bytes());
                match (a, b) {
                    (Some(a), Some(b)) => {
                        let r = match a.cmp(&b) {
                            std::cmp::Ordering::Less => -1i64,
                            std::cmp::Ordering::Equal => 0,
                            std::cmp::Ordering::Greater => 1,
                        };
                        Value::from_i128(r as i128, 32, true)
                    }
                    _ => Value::xs(32, true),
                }
            }
            // ⓑ-breadth (v18): string→number conversions (IEEE §6.16.9-13).
            // Parse the leading numeric prefix in the requested base; a leading
            // sign is honored for decimal (IEEE §6.16.9 — note iverilog 13 drops
            // it, its bug). Empty / non-numeric prefix → 0. Result truncates to
            // a 32-bit int.
            SysFuncId::StrAtoi
            | SysFuncId::StrAtohex
            | SysFuncId::StrAtooct
            | SysFuncId::StrAtobin => match self.handle_str_bytes(args.first()) {
                Some(b) => {
                    let (radix, signed) = match which {
                        SysFuncId::StrAtoi => (10, true),
                        SysFuncId::StrAtohex => (16, false),
                        SysFuncId::StrAtooct => (8, false),
                        _ => (2, false),
                    };
                    let n = parse_radix_prefix(&b, radix, signed);
                    Value::from_i128((n as i32) as i128, 32, true)
                }
                None => Value::xs(32, true),
            },
            SysFuncId::StrAtoreal => match self.handle_str_bytes(args.first()) {
                Some(b) => Value::from_f64(parse_real_prefix(&b)),
                None => Value::xs(64, true),
            },
            // v7 shape, features not wired yet (elaborate still rejects the
            // names): defensive X at each func's declared self-width.
            // `ValuePlusargs` here = unsupported placement (the legal direct-rhs
            // form is intercepted statement-level).
            SysFuncId::Fopen | SysFuncId::ValuePlusargs => Value::xs(32, true),
            // G1 (IEEE §6.16): an EXPRESSION-context `$sformatf` reaches eval ONLY
            // as elaborate's string-concat desugar — `$sformatf("%s%s…%s", p0, p1,
            // …)`, fmt = "%s"×N, args[1..] = the N concat parts. It renders to a
            // STRING-domain value so `$display("%s", {a,b})`, `{a,b}=="…"`, and
            // func/task args all see the concatenated bytes. The render matches the
            // statement-assign path's `%s` handler byte-for-byte (the oracle): a
            // string-literal Const → its decoded text; a string-domain value → its
            // exact bytes; any other integral value → its packed char bytes (§6.16).
            // (The statement-level `s = $sformatf(...)` form is intercepted before
            // eval; the kernel `format_args_str` honors arbitrary specs there.)
            SysFuncId::Sformatf => {
                let mut bytes: Vec<u8> = Vec::new();
                for &p in args.iter().skip(1) {
                    if let Some(s) = self.const_str_arg(p) {
                        bytes.extend_from_slice(s.as_bytes());
                    } else {
                        let v = self.eval(p);
                        if v.is_str {
                            bytes.extend_from_slice(&v.to_str_bytes());
                        } else {
                            bytes.extend_from_slice(
                                crate::builtins::fmt_packed_chars(&v).as_bytes(),
                            );
                        }
                    }
                }
                Value::from_str_bytes(&bytes)
            }
            // Round-9 FIO: $feof is PURE — read the fd's EOF flag through the
            // NetReader. `SimState` overrides `fd_eof` with the live file state;
            // the native-eval fakes keep the default X. Mirrors `k_feof`'s VALUE
            // contract exactly (pre-opened fd → 0, bad/closed fd → −1, else the
            // EOF flag), minus the bad-fd warning (eval is `&self`; the direct-rhs
            // `e = $feof(fd)` form still warns via `k_feof`). Re-evaluated each
            // iteration of `while (!$feof(fd))` — exactly correct for a pure fn.
            SysFuncId::Feof => match args.first().map(|&a| self.eval(a)) {
                Some(v) if !v.has_xz() => self.nets.fd_eof(v.to_u64().unwrap_or(0) as u32),
                _ => Value::from_i128(-1, 32, true),
            },
            // v9 shape-bump placeholders: the side-effecting file-read family,
            // $dist_*, and the $cast function form are all intercepted at the
            // statement level (direct rhs of a blocking assign) once ranks 5-6
            // wire them; in a non-intercepted eval context they yield X (the
            // same contract as `Fopen`/`ValuePlusargs`). elaborate emits none
            // of these yet, so this arm is dead until then.
            SysFuncId::Fgets
            | SysFuncId::Fscanf
            | SysFuncId::Sscanf
            | SysFuncId::Fread
            | SysFuncId::Fgetc
            | SysFuncId::Ungetc
            | SysFuncId::DistUniform
            | SysFuncId::DistNormal
            | SysFuncId::DistExponential
            | SysFuncId::DistPoisson
            | SysFuncId::DistChiSquare
            | SysFuncId::DistT
            | SysFuncId::DistErlang
            | SysFuncId::Cast => Value::xs(32, true),
            // ── v19: N6 real-math (IEEE §20.8.2), computed via the vendored
            //   pure-Rust libm (3-OS byte-identical). An integral argument
            //   coerces to real (signed); a real argument is used as-is. Domain
            //   errors (e.g. $sqrt(-1), $acos(2), $ln(0)) propagate NaN/±inf
            //   exactly like C/iverilog. PURE — no seed/heap mutation. ──
            SysFuncId::Ln
            | SysFuncId::Log10
            | SysFuncId::Exp
            | SysFuncId::Sqrt
            | SysFuncId::Pow
            | SysFuncId::Floor
            | SysFuncId::Ceil
            | SysFuncId::Sin
            | SysFuncId::Cos
            | SysFuncId::Tan
            | SysFuncId::Asin
            | SysFuncId::Acos
            | SysFuncId::Atan
            | SysFuncId::Atan2
            | SysFuncId::Hypot
            | SysFuncId::Sinh
            | SysFuncId::Cosh
            | SysFuncId::Tanh
            | SysFuncId::Asinh
            | SysFuncId::Acosh
            | SysFuncId::Atanh => {
                let real_arg = |i: usize| -> f64 {
                    match args.get(i) {
                        Some(&a) => {
                            let v = self.eval(a);
                            if v.is_real {
                                v.to_f64().unwrap_or(0.0)
                            } else {
                                // integral operand → real (signed); X/Z → 0.0.
                                v.to_i128_signed().unwrap_or(0) as f64
                            }
                        }
                        None => 0.0,
                    }
                };
                let x = real_arg(0);
                let r = match which {
                    SysFuncId::Ln => libm::log(x),
                    SysFuncId::Log10 => libm::log10(x),
                    SysFuncId::Exp => libm::exp(x),
                    SysFuncId::Sqrt => libm::sqrt(x),
                    SysFuncId::Pow => libm::pow(x, real_arg(1)),
                    SysFuncId::Floor => libm::floor(x),
                    SysFuncId::Ceil => libm::ceil(x),
                    SysFuncId::Sin => libm::sin(x),
                    SysFuncId::Cos => libm::cos(x),
                    SysFuncId::Tan => libm::tan(x),
                    SysFuncId::Asin => libm::asin(x),
                    SysFuncId::Acos => libm::acos(x),
                    SysFuncId::Atan => libm::atan(x),
                    SysFuncId::Atan2 => libm::atan2(x, real_arg(1)),
                    SysFuncId::Hypot => libm::hypot(x, real_arg(1)),
                    SysFuncId::Sinh => libm::sinh(x),
                    SysFuncId::Cosh => libm::cosh(x),
                    SysFuncId::Tanh => libm::tanh(x),
                    SysFuncId::Asinh => libm::asinh(x),
                    SysFuncId::Acosh => libm::acosh(x),
                    SysFuncId::Atanh => libm::atanh(x),
                    _ => unreachable!("real-math arm gates on the same id set"),
                };
                Value::from_f64(r)
            }
        }
    }

    /// `$signed`/`$unsigned`/`$time`/`$clog2` in context: cast preserves width
    /// but flips sign; $time/$clog2 produce a fixed-width value then extend.
    pub(crate) fn eval_sysfunc_ctx(
        &self,
        which: SysFuncId,
        args: &[u32],
        w: u32,
        eff_signed: bool,
    ) -> Value {
        match which {
            SysFuncId::Signed => {
                // operand at its OWN self width. `$signed` re-stamps it signed, but
                // the EXTENSION FILL is governed by `eff_signed` (= self-signed AND
                // ctx_signed), NOT the unconditional cast: under the global-unsigned
                // rule (§5.5.1) an unsigned sibling makes the whole region unsigned,
                // so `$signed(x)` must ZERO-extend there, not sign-extend. Setting
                // `.signed = eff_signed` BEFORE the resize makes the fill policy
                // unambiguously flag-driven.
                let mut a = self.eval(args[0]);
                a.signed = eff_signed;
                a.resize_keep_sign(w, eff_signed)
            }
            SysFuncId::Unsigned => {
                let mut a = self.eval(args[0]);
                a.signed = false;
                a.resize_keep_sign(w, false) // unsigned cast → zero-extend
            }
            // $time/$realtime (64-bit) and $clog2 (32-bit): natural value, then
            // resize to context (zero/sign per eff_signed).
            _ => self
                .eval_sysfunc(which, args)
                .resize_keep_sign(w, eff_signed),
        }
    }

    // ── helpers ────────────────────────────────────────────────────────────

    pub(crate) fn truthiness(&self, a: &Value) -> Tri {
        // A real is logically true iff it is != 0.0 (IEEE 1364: `-0.0 == 0.0`,
        // so both signed zeros are FALSE; NaN != 0.0 → truthy). Reinterpreting a
        // real's f64 bits as a 4-state vector would wrongly read `-0.0`
        // (sign bit set, value zero) as true in `if`/`!`/ternary/`&&`/`||`.
        if a.is_real {
            return if a.to_f64().unwrap_or(0.0) != 0.0 {
                Tri::True
            } else {
                Tri::False
            };
        }
        let mut any_unknown = false;
        for i in 0..a.width {
            let (v, u) = a.get_vu(i);
            if u == 0 && v == 1 {
                return Tri::True;
            }
            if u != 0 {
                any_unknown = true;
            }
        }
        if any_unknown {
            Tri::Unknown
        } else {
            Tri::False
        }
    }
}
pub(crate) fn ipow_signed(base: i128, exp: i128) -> u128 {
    if exp < 0 {
        // negative exponent on integers → 0 (except base==1 → 1, base==-1 → ±1)
        return match base {
            1 => 1,
            -1 => {
                if exp % 2 == 0 {
                    1
                } else {
                    (-1i128) as u128
                }
            }
            _ => 0,
        };
    }
    let mut acc: i128 = 1;
    let mut e = exp;
    let mut b = base;
    while e > 0 {
        if e & 1 == 1 {
            acc = acc.wrapping_mul(b);
        }
        b = b.wrapping_mul(b);
        e >>= 1;
    }
    acc as u128
}

// ── multi-word kernels (Phase-1.x ⑥) — all operate on little-endian u64
//    word vectors of equal length; callers mask to the target width. ──────

pub(crate) fn mw_add(a: &[u64], b: &[u64]) -> Vec<u64> {
    let mut out = vec![0u64; a.len()];
    let mut carry = 0u64;
    for k in 0..a.len() {
        let (s1, c1) = a[k].overflowing_add(b.get(k).copied().unwrap_or(0));
        let (s2, c2) = s1.overflowing_add(carry);
        out[k] = s2;
        carry = (c1 as u64) + (c2 as u64);
    }
    out
}

/// In-place `dest += b` on the word grid; bit-identical to
/// `dest = mw_add(&dest, b)` but reuses `dest`'s allocation (hot in the
/// restoring-division loop, where `b` is the pre-negated divisor).
pub(crate) fn mw_add_inplace(dest: &mut [u64], b: &[u64]) {
    let mut carry = 0u64;
    for (k, d) in dest.iter_mut().enumerate() {
        let (s1, c1) = d.overflowing_add(b.get(k).copied().unwrap_or(0));
        let (s2, c2) = s1.overflowing_add(carry);
        *d = s2;
        carry = (c1 as u64) + (c2 as u64);
    }
}

/// Two's complement on the word grid (`!a + 1`); caller masks to width.
pub(crate) fn mw_neg(a: &[u64]) -> Vec<u64> {
    let mut out = vec![0u64; a.len()];
    let mut carry = 1u64;
    for k in 0..a.len() {
        let (s, c) = (!a[k]).overflowing_add(carry);
        out[k] = s;
        carry = c as u64;
    }
    out
}

pub(crate) fn mw_mask(mut a: Vec<u64>, w: u32) -> Vec<u64> {
    let n = nwords(w).max(1);
    a.truncate(n);
    a.resize(n, 0);
    let top = w - 64 * (n as u32 - 1);
    a[n - 1] &= low_mask(top);
    a
}

pub(crate) fn mw_is_zero(a: &[u64]) -> bool {
    a.iter().all(|&x| x == 0)
}

pub(crate) fn mw_one(n: usize) -> Vec<u64> {
    let mut v = vec![0u64; n];
    v[0] = 1;
    v
}

/// School multiplication, LOW `n` words (mod 2^(64n)).
pub(crate) fn mw_mul(a: &[u64], b: &[u64], n: usize) -> Vec<u64> {
    let mut out = vec![0u64; n];
    for i in 0..n.min(a.len()) {
        if a[i] == 0 {
            continue;
        }
        let mut carry = 0u128;
        for j in 0..n - i {
            let bj = b.get(j).copied().unwrap_or(0);
            let cur = (a[i] as u128) * (bj as u128) + (out[i + j] as u128) + carry;
            out[i + j] = cur as u64;
            carry = cur >> 64;
        }
    }
    out
}

pub(crate) fn mw_cmp(a: &[u64], b: &[u64]) -> std::cmp::Ordering {
    for k in (0..a.len().max(b.len())).rev() {
        let av = a.get(k).copied().unwrap_or(0);
        let bv = b.get(k).copied().unwrap_or(0);
        match av.cmp(&bv) {
            std::cmp::Ordering::Equal => continue,
            o => return o,
        }
    }
    std::cmp::Ordering::Equal
}
