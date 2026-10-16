//! §5.2 row 27: `$readmemb` / `$readmemh` into an associative array.
//!
//! IEEE 1800-2017 §21.4: the file's words load one element each; an `@address` line
//! moves the address; consecutive words take consecutive addresses. For an
//! associative array each address is the KEY of one entry: an address the file
//! supplies creates (or overwrites) that entry and nothing else, and entries the
//! file does not name keep their value.
//!
//! Elaborate lowers the memory argument of such a call to the marker
//! `SysFunc { ReadmemAssocTarget, [Signal { net, word: None }, k] }`, where `k` is a
//! `Const` whose width and sign are the array's key type
//! (`Elaborator::readmem_assoc_target`). No element select lowers to that shape, so
//! `$readmemh(f, mem[5])` stays "not a memory". The engine stores every
//! integral key as a signed `i64`, and the value it is asked to hold is the address
//! itself, so every address must be a value of the key type: then the address cast
//! to the key type (§7.8.4, a typed key's index), the address read uncast (a keyword
//! key's index, `[int]`, `[byte]`, …) and the address are the same number. An
//! address outside the key type's range has no single reading (truncated by the
//! cast, verilator 5.052; or past "the highest address in the memory", §21.4) and is
//! refused by name (W4023, the load stops there), as is a word before any address
//! into a SIGNED key with no start argument (§21.4's default start is "the lowest
//! address in the memory": the key type's minimum, or verilator's 0).
//!
//! The start / finish window, its direction and its warnings are the fixed-memory
//! loader's (`readmem`, iverilog-pinned): addresses in the file must lie inside the
//! window (§21.4: "otherwise … the load operation is terminated"), a finish below
//! the start loads downward, and a file shorter than an explicit window warns. A
//! word or address token with a character that is not a digit of the radix stops
//! the load (iverilog 13: `Invalid input character`, the words before it loaded).

use super::*;

/// The key type of an associative-array `$readmem*` target: the range of addresses
/// it can hold exactly in the engine's signed-`i64` key domain.
#[derive(Debug, Clone, Copy)]
pub(crate) struct AssocMemKey {
    min: i128,
    max: i128,
    width: u32,
    signed: bool,
}

impl AssocMemKey {
    fn contains(&self, a: i128) -> bool {
        (self.min..=self.max).contains(&a)
    }

    fn describe(&self) -> String {
        format!(
            "{}-bit {} key",
            self.width,
            if self.signed { "signed" } else { "unsigned" }
        )
    }
}

/// The target array and key type of `$readmem*`'s associative-array marker
/// (`SysFuncId::ReadmemAssocTarget`, whose `args` are `target`): `[Signal { net,
/// word: None }, Const k]`. `None` when the marker is not that shape or `net` is not
/// an integer-keyed associative array.
pub(crate) fn assoc_readmem_key(ir: &sim_ir::SimIr, target: &[u32]) -> Option<(u32, AssocMemKey)> {
    let [sig, key] = target else {
        return None;
    };
    let sim_ir::Expr::Signal { net, word: None } = ir.exprs.get(*sig as usize)? else {
        return None;
    };
    Some((*net, assoc_key_range(ir, *net, *key)?))
}

/// The key type of the associative array `net`, read off the width and sign of the
/// `Const` expression `key`. A 64-bit unsigned key is `[time]` (row 20 refuses every
/// other one at its declaration): its upper half does not fit the signed key domain,
/// so its range ends at `i64::MAX`.
pub(crate) fn assoc_key_range(ir: &sim_ir::SimIr, net: u32, key: u32) -> Option<AssocMemKey> {
    if ir.nets.get(net as usize)?.kind != sim_ir::NetKind::Assoc {
        return None;
    }
    let sim_ir::Expr::Const { val } = ir.exprs.get(key as usize)? else {
        return None;
    };
    let c = ir.consts.get(*val as usize)?;
    let (width, signed) = (c.width, c.signed);
    if !(1..=64).contains(&width) {
        return None;
    }
    let (min, max) = if signed {
        (-(1i128 << (width - 1)), (1i128 << (width - 1)) - 1)
    } else {
        (0, ((1i128 << width) - 1).min(i128::from(i64::MAX)))
    };
    Some(AssocMemKey {
        min,
        max,
        width,
        signed,
    })
}

/// An `@address` token's digits (hex in both variants, §21.4). `Err` on an empty
/// token or a character that is not a hex digit; a value past `u128` saturates
/// (it fits no key type anyway).
fn parse_addr(digits: &str) -> Result<u128, ()> {
    if digits.is_empty() {
        return Err(());
    }
    let mut v: u128 = 0;
    for ch in digits.chars() {
        let d = ch.to_digit(16).ok_or(())?;
        v = v.saturating_mul(16).saturating_add(u128::from(d));
    }
    Ok(v)
}

/// A data word every character of which `parse_mem_token` reads as a digit (or as
/// x / z / `_`). The fixed-memory loader skips a stray character instead — a
/// pre-existing silent reading (ROADMAP §2); this loader stops there.
fn word_is_valid(tok: &str, hex: bool) -> bool {
    tok.chars().all(|c| {
        matches!(c, '_' | 'x' | 'X' | 'z' | 'Z' | '?') || c.is_digit(if hex { 16 } else { 2 })
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn readmem_assoc<N: crate::eval::NetReader + ?Sized>(
    sched: &mut Scheduler,
    nets: Option<&N>,
    out: &mut crate::builtins::TaskWrites<'_>,
    args: &[u32],
    hex: bool,
    name: &str,
    net: u32,
    key: AssocMemKey,
) {
    let warn = |sched: &mut Scheduler, msg: String| sched.st.warn_readmem(msg);
    let file = printable(name);
    let Ok(text) = std::fs::read_to_string(name) else {
        warn(
            sched,
            format!("$readmem: unable to open '{file}' for reading"),
        );
        return;
    };
    let cleaned = strip_mem_comments(&text);
    // The window bounds, self-determined (each argument at its own width and
    // sign, as the fixed-memory loader reads them). An x / z bound, or one that is
    // not a value of the key type, loads nothing.
    let mut bounds = [None, None];
    for (slot, (pos, what)) in bounds.iter_mut().zip([(2usize, "start"), (3, "finish")]) {
        let Some(&a) = args.get(pos) else { continue };
        let v = crate::builtins::eval_task_arg(sched, nets, a);
        if v.is_real || v.has_xz() {
            warn(
                sched,
                format!(
                    "$readmem('{file}'): the {what} address of an associative-array load is \
                     x or z; nothing loaded"
                ),
            );
            return;
        }
        match v.to_i128_signed().filter(|&k| key.contains(k)) {
            Some(k) => *slot = Some(k),
            None => {
                warn(
                    sched,
                    format!(
                        "$readmem('{file}'): the {what} address {} is not a value of the \
                         associative array's {}; nothing loaded",
                        fmt_dec(&v),
                        key.describe()
                    ),
                );
                return;
            }
        }
    }
    let [r_start, r_finish] = bounds;
    let finish_given = r_finish.is_some();
    let (win_lo, win_hi, step) = match (r_start, r_finish) {
        (Some(s), Some(f)) => (s.min(f), s.max(f), if s > f { -1i128 } else { 1 }),
        (Some(s), None) => (s, key.max, 1),
        _ => (key.min, key.max, 1),
    };
    // Where the first word goes when the file names no address first: the start
    // argument, else 0 for an unsigned key (its lowest value, and verilator's).
    let mut addr: Option<i128> = r_start.or((!key.signed).then_some(key.min));
    let w = sched.st.ir.nets[net as usize].width.max(1);
    let two_state = sched
        .st
        .two_state
        .get(net as usize)
        .copied()
        .unwrap_or(false);
    let mut wrote: u128 = 0;
    let mut had_at = false;
    for tok in cleaned.split_whitespace() {
        if let Some(a) = tok.strip_prefix('@') {
            had_at = true;
            let Ok(v) = parse_addr(a) else {
                warn(
                    sched,
                    format!("$readmem('{file}'): bad address token '@{a}'; stopped"),
                );
                return;
            };
            match i128::try_from(v).ok().filter(|&k| key.contains(k)) {
                Some(k) => addr = Some(k),
                None => {
                    warn(
                        sched,
                        format!(
                            "$readmem('{file}'): address @{a} is not a value of the \
                             associative array's {}; stopped",
                            key.describe()
                        ),
                    );
                    return;
                }
            }
            continue;
        }
        let Some(a) = addr else {
            warn(
                sched,
                format!(
                    "$readmem('{file}'): a word before any `@address`, into an associative \
                     array with a {} and no start address: the default start (IEEE §21.4 \
                     'the lowest address in the memory', or 0) has no settled reading; \
                     nothing loaded (give a start address or an `@` line)",
                    key.describe()
                ),
            );
            return;
        };
        if a < win_lo || a > win_hi {
            // Past an explicit finish, or an `@` line outside the window: the fixed
            // loader's message. Past the key type's own end (no finish given): the
            // key's.
            let msg = if finish_given || key.contains(a) {
                format!("$readmem('{file}'): address {a} outside the load range; stopped")
            } else {
                format!(
                    "$readmem('{file}'): the next address {a} is not a value of the \
                     associative array's {}; stopped",
                    key.describe()
                )
            };
            warn(sched, msg);
            return;
        }
        if !word_is_valid(tok, hex) {
            warn(
                sched,
                format!("$readmem('{file}'): bad data word '{tok}'; stopped"),
            );
            return;
        }
        let mut val = parse_mem_token(tok, w, hex);
        if two_state {
            // A 2-state element holds no x or z (IEEE 1800-2017 §6.11.3): both read 0.
            for (v, u) in val.val.iter_mut().zip(val.unk.iter_mut()) {
                *v &= !*u;
                *u = 0;
            }
        }
        // The key rides out of band (`Offsets::AssocKey`), as `resolve_offsets` hands
        // an element write `a[k] = v`; the chunk's `word` is never evaluated.
        let lv = sim_ir::Lvalue {
            chunks: vec![sim_ir::LvalChunk {
                net,
                word: Some(0),
                offset: None,
                width: None,
                kind: sim_ir::SelKind::Bit,
            }],
        };
        out.put(
            sched,
            lv,
            val,
            crate::exec::Offsets::AssocKey(Some(a as i64)),
        );
        wrote += 1;
        addr = Some(a + step);
    }
    let window = (win_hi - win_lo) as u128 + 1;
    if !had_at && finish_given && wrote < window {
        let (start, finish) = (r_start.unwrap_or(win_lo), r_finish.unwrap_or(win_hi));
        warn(
            sched,
            format!(
                "$readmem('{file}'): not enough words for the range \
                 [{start}:{finish}] ({wrote} of {window}); rest unchanged"
            ),
        );
    }
}
