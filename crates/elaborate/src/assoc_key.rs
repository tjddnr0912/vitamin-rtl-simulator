//! §5.2 row 20: the integral KEY TYPE of an associative array.
//!
//! The engine stores every integral key as a signed `i64` and evaluates a key
//! expression on its own (`assoc_key`: the expression, widened to 64 bits by its
//! own sign). It does not know the declared key type, so elaborate converts each
//! index expression to that type before the engine sees it (IEEE 1800-2017
//! §7.8.4: an index expression is cast to the index type; the expression itself
//! is self-determined, as every index is, §11.5.1):
//!
//! * wider than the key: the low `W` bits, then the key's sign;
//! * as wide: the key's sign;
//! * narrower: extended by the EXPRESSION's sign to `W`, then the key's sign.
//!
//! The result is wrapped in `$signed` / `$unsigned`, the repo's seal primitive:
//! the engine's 64-bit key context stops there, so `c + d` over two `byte`s wraps
//! at 8 bits as the expression's own width says, not at 64.
//!
//! An x or z anywhere in the evaluated EXPRESSION makes the key invalid (§7.8.6;
//! the pre-row-20 engine rule, kept): truncation goes through `% 2^W`, which turns
//! any unknown operand bit into an all-x result, rather than a select that would
//! drop an x above bit `W`.
//!
//! The keyword atoms (`[int]`, `[integer]`, `[byte]`, `[shortint]`, `[longint]`)
//! are NOT converted: their index keeps the pre-row-20 reading (see
//! [`Elaborator::assoc_key_convert`]).
//!
//! A key type the signed-i64 domain cannot hold exactly is refused by name: wider
//! than 64 bits, or 64-bit unsigned (`longint unsigned`, `bit [63:0]` — a key at
//! or above 2^63 would order as negative). `[time]` keeps its own pre-row-20
//! variant and is not converted (ROADMAP §2).

use super::*;

/// The folded key type of one associative-array handle net.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AssocKeyShape {
    pub(crate) width: u32,
    pub(crate) signed: bool,
    /// A bare keyword atom (`[int]`, `[integer]`, `[byte]`, `[shortint]`,
    /// `[longint]`) — the spellings that predate row 20. They keep the
    /// pre-row-20 index (not cast), `foreach` index (`integer`), iteration-variable,
    /// locator, copy and `%p` rules.
    pub(crate) legacy: bool,
}

/// What a `foreach` over an assoc declares its index as (the parser's desugar):
/// `integer`, 32-bit signed.
const FOREACH_INDEX: (u32, bool) = (32, true);

impl Elaborator<'_> {
    /// Fold the key type a declaration's dimensions give an assoc handle.
    /// `Ok(None)`: not a typed integral key (`[time]`, `[string]`, not assoc).
    /// `Err(())`: refused, the error already reported.
    pub(crate) fn assoc_key_shape_of(
        &mut self,
        dims: &[ast::Dim],
    ) -> Result<Option<AssocKeyShape>, ()> {
        let Some(kt) = dims.iter().find_map(|d| match d {
            ast::Dim::Assoc(ast::AssocKey::Typed(kt)) => Some(kt.clone()),
            _ => None,
        }) else {
            return Ok(None);
        };
        use ast::NetVarKind as K;
        let atom = match kt.kind {
            K::Int | K::Integer => Some(32u64),
            K::Byte => Some(8),
            K::Shortint => Some(16),
            K::Longint | K::Time => Some(64),
            _ => None,
        };
        let width: u64 = match atom {
            Some(w) => w,
            None => {
                let mut w: u64 = 1;
                for r in kt.range.iter().chain(kt.packed.iter()) {
                    let (Some(m), Some(l)) = (
                        self.const_range_bound_fold(&r.msb),
                        self.const_range_bound_fold(&r.lsb),
                    ) else {
                        self.error(
                            MsgCode::ElabUnsupported,
                            "an associative-array key type's bounds must be constant \
                             expressions",
                        );
                        return Err(());
                    };
                    w = w.saturating_mul(m.abs_diff(l).saturating_add(1));
                }
                w
            }
        };
        if width > 64 {
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "an associative-array key type of {width} bits is wider than vita's \
                     64-bit key domain"
                ),
            );
            return Err(());
        }
        if width == 64 && !kt.signed {
            self.error(
                MsgCode::ElabUnsupported,
                "an associative-array key of a 64-bit UNSIGNED type (`longint unsigned`, \
                 `bit [63:0]`) does not fit vita's signed 64-bit key domain: a key at or \
                 above 2^63 would order as negative",
            );
            return Err(());
        }
        // Exactly the spellings PRE accepted (round-2 soundness R2-1): a bare atom,
        // no typedef name, no `signed`/`unsigned` keyword. `[int signed]` was E2002
        // before row 20 and is a typed key.
        let legacy = !kt.named
            && !kt.explicit_sign
            && kt.signed
            && kt.range.is_none()
            && kt.packed.is_empty()
            && matches!(
                kt.kind,
                K::Int | K::Integer | K::Byte | K::Shortint | K::Longint
            );
        Ok(Some(AssocKeyShape {
            width: width as u32,
            signed: kt.signed,
            legacy,
        }))
    }

    /// Convert the already-lowered key expression `key` of assoc handle `net` to
    /// the handle's key type (module doc). A handle with no typed key returns
    /// `key` unchanged, and so does a keyword atom (`[int]`, `[byte]`, …).
    ///
    /// The keyword atoms keep the pre-row-20 index exactly (round-1 soundness F1):
    /// their index is the expression widened to 64 bits, never cast. Casting it
    /// stores a key at or above 2^(W-1) written through an unsigned index as
    /// negative, and the engine reads a `next`/`prev` argument by the VARIABLE's own
    /// sign and tests the fit by value (`frame_eval.rs` `assoc_iter_compute`), so an
    /// unsigned iteration variable stopped early or looped forever (`[integer]` with
    /// `logic [31:0] k`: `last`/`prev` never ended). The two halves cancel only
    /// together; the wrong values of the uncast index are ROADMAP §2 lines. A typed
    /// key is immune: its iteration variable must have the key type.
    pub(crate) fn assoc_key_convert(&mut self, net: u32, key: u32) -> u32 {
        let Some(shape) = self.assoc_key_shapes.get(&net).copied() else {
            return key;
        };
        if shape.legacy || self.error_placeholders.contains(&key) {
            return key;
        }
        // Round-2 differential B2: `m['1]` has two readings with no oracle to settle
        // them — the index self-determined, so `'1` is one bit (key 1), or the fill
        // sized by the cast to the key type (§6.24.1; all ones, verilator 5.052) —
        // and vita's own `T'('1)` takes the second. Refused by name.
        if self.bare_fill_index_eids.contains(&key) {
            self.error(
                MsgCode::ElabUnsupported,
                "an unbased unsized fill literal (`'0`, `'1`, `'x`, `'z`) as the index of an \
                 associative array with a typed key: its width (one bit, or the key's) has \
                 no settled reading — write the key at its width (`16'hFFFF`)",
            );
            return self.placeholder_expr();
        }
        let Some(sw) = self.canonical_self_width(key) else {
            // A width not knowable yet (a hierarchical placeholder with no recorded
            // shape).
            self.error(
                MsgCode::ElabUnsupported,
                "an associative-array key expression whose width is not known where it \
                 is converted to the key type",
            );
            return self.placeholder_expr();
        };
        let (w, s) = (sw.width.max(1), sw.signed);
        let (kw, ks) = (shape.width, shape.signed);
        let seal = |me: &mut Self, e: u32, signed: bool| {
            me.push_expr(ir::Expr::SysFunc {
                which: if signed {
                    ir::SysFuncId::Signed
                } else {
                    ir::SysFuncId::Unsigned
                },
                args: vec![e],
            })
        };
        if w > kw {
            // Truncate to the low `kw` bits. Over an operand that can carry x/z,
            // `$unsigned(e) % 2^kw` keeps any unknown bit (an all-x result) where a
            // plain select would drop one above bit `kw`.
            let low = if self.expr_may_be_unknown(key) {
                let u = seal(self, key, false);
                let mut val = vec![0u64; (kw as usize + 1).div_ceil(64)];
                val[(kw / 64) as usize] |= 1u64 << (kw % 64);
                let unk = vec![0u64; val.len()];
                let m = self.intern_const(ir::ConstVal {
                    width: kw + 1,
                    signed: false,
                    repr: ir::ConstRepr::Numeric,
                    bits: ir::BitPacked { val, unk },
                });
                let m = self.push_expr(ir::Expr::Const { val: m });
                self.push_expr(ir::Expr::Binary {
                    op: ir::BinOp::Mod,
                    lhs: u,
                    rhs: m,
                })
            } else {
                key
            };
            let lo = self.select_low(low, kw);
            return seal(self, lo, ks);
        }
        if w < kw && s && !ks {
            // A signed expression into a wider UNSIGNED key: sign-extend to the key
            // width first (naming the operand once), then read it unsigned.
            let ext = self.extend_signed_once(key, kw);
            return seal(self, ext, false);
        }
        // As wide: the key's sign. Narrower and unsigned: zero-extended, which is
        // non-negative in either key sign. Narrower, signed, into a signed key: the
        // engine's sign extension from `w` equals extension to `kw` and then to 64.
        seal(self, key, if w == kw { ks } else { s })
    }

    /// The iteration variable `knet` of `h.first/next/last/prev(k)` on assoc
    /// handle `net`. For a typed key other than the keyword atoms it must have the
    /// key's width and sign: the engine reads `next`/`prev`'s current key as the
    /// variable's own value and writes a found key truncated to the variable
    /// (IEEE 1800-2017 §7.9.8 casts the argument to the index type and returns −1
    /// for a variable smaller than the index type — neither is carried). `foreach`
    /// declares its index `integer`, so it walks only a 32-bit signed key. `true`
    /// = refused (reported).
    ///
    /// A `foreach` lowers to `first` and then `next`; only `first` reports, so one
    /// loop is one diagnostic.
    pub(crate) fn assoc_iter_var_refused(
        &mut self,
        net: u32,
        knet: u32,
        foreach: bool,
        method: &str,
    ) -> bool {
        let Some(shape) = self.assoc_key_shapes.get(&net).copied() else {
            return false;
        };
        if shape.legacy {
            return false;
        }
        let key = (shape.width, shape.signed);
        let sign = |s: bool| if s { "signed" } else { "unsigned" };
        if foreach {
            if key == FOREACH_INDEX {
                return false;
            }
            if method != "first" {
                return true;
            }
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "`foreach` over an associative array keyed by a {}-bit {} type: v1 \
                     declares the loop index `integer`, which does not hold that key type \
                     (walk it with `first`/`next` and an index variable of the key type)",
                    key.0,
                    sign(key.1)
                ),
            );
            return true;
        }
        let Some(nv) = self.nets.get(knet as usize) else {
            return false;
        };
        if (nv.width, nv.signed) == key {
            return false;
        }
        self.error(
            MsgCode::ElabUnsupported,
            &format!(
                "the iteration variable of an associative array keyed by a {}-bit {} type \
                 must have that type (it is {}-bit {})",
                key.0,
                sign(key.1),
                nv.width,
                sign(nv.signed)
            ),
        );
        true
    }

    /// A `with` clause over assoc handle `net` whose lowered expression (every node
    /// pushed from `start`) reads `item.index`, or an index-returning locator: the
    /// engine hands both the FLAT position, not the key. Refused for a typed key
    /// other than the keyword atoms (whose positional reading predates row 20;
    /// ROADMAP §2). `true` = refused (reported).
    pub(crate) fn assoc_index_read_refused(
        &mut self,
        net: u32,
        start: usize,
        index_locator: bool,
    ) -> bool {
        let Some(shape) = self.assoc_key_shapes.get(&net).copied() else {
            return false;
        };
        if shape.legacy {
            return false;
        }
        let reads_index = self.exprs[start.min(self.exprs.len())..]
            .iter()
            .any(|e| matches!(e, ir::Expr::ArrayItem { index: true, .. }));
        if !(index_locator || reads_index) {
            return false;
        }
        self.error(
            MsgCode::ElabUnsupported,
            "an index-returning locator or `item.index` over an associative array keyed \
             by a typed key: v1 answers the element's position, not its key",
        );
        true
    }

    /// Whole-handle copy `dst = src` between two assoc handles: the keys are
    /// copied as stored, so two typed key types must agree in width and sign, and a
    /// typed array never copies to or from a keyword-atom one. Two keyword
    /// spellings (`[int]` / `[integer]`, or `[time]`) keep their pre-row-20 copy
    /// (ROADMAP §2: IEEE §7.9.9 wants one index type). `true` = refused (reported).
    pub(crate) fn assoc_copy_refused(&mut self, dst: u32, src: u32) -> bool {
        let d = self.assoc_key_shapes.get(&dst).copied();
        let s = self.assoc_key_shapes.get(&src).copied();
        let legacy = |k: Option<AssocKeyShape>| k.is_none_or(|k| k.legacy);
        if legacy(d) && legacy(s) {
            return false;
        }
        // A keyword-atom array holds UNCAST keys (`assoc_key_convert`), so its keys
        // need not lie in a typed array's domain even at the same width and sign:
        // only two typed arrays of one key type copy.
        if legacy(d) != legacy(s) {
            self.error(
                MsgCode::ElabUnsupported,
                "whole-handle copy between an associative array with a keyword key \
                 (`[int]`, `[integer]`, `[byte]`, `[shortint]`, `[longint]`, `[time]`) and \
                 one with a typed key: vita never copies between the two (the keyword \
                 array holds its keys uncast), whatever their widths and signs",
            );
            return true;
        }
        let ty = |k: Option<AssocKeyShape>| k.map(|k| (k.width, k.signed));
        if ty(d) == ty(s) {
            return false;
        }
        self.error(
            MsgCode::ElabUnsupported,
            "whole-handle copy between associative arrays needs the same key type on \
             both sides (width and sign)",
        );
        true
    }

    /// `%p` (a whole-array format argument) of assoc handle `net`. The engine
    /// renders every integral key as its stored signed i64 (`builtins::pattern`), so
    /// a NEGATIVE key of a typed signed key type prints 64 bits wide
    /// (`'hfffffffffffffffe` for a `bit signed [11:0]` key -2, where verilator 5.052
    /// prints `'hffe`), and the key type does not reach the formatter. Refused for a
    /// typed signed key (round-1 soundness F2). A typed unsigned key is stored
    /// non-negative and prints as verilator does (`[int unsigned]` -2:
    /// `'hfffffffe`); the keyword atoms keep their pre-row-20 render (ROADMAP §3.b
    /// `%p-ⓒ`). `true` = refused (reported).
    pub(crate) fn assoc_pattern_refused(&mut self, net: u32) -> bool {
        let Some(shape) = self.assoc_key_shapes.get(&net).copied() else {
            return false;
        };
        if shape.legacy || !shape.signed {
            return false;
        }
        self.error(
            MsgCode::ElabUnsupported,
            &format!(
                "`%p` of an associative array keyed by a {}-bit signed typed key: v1 \
                 renders a negative key 64 bits wide",
                shape.width
            ),
        );
        true
    }
}
