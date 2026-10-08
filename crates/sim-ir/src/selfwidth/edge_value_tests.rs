//! `edge_value` against a reference written from IEEE 1800 §11.6.1 / §11.8.1-2 (one
//! self-determined region per bound: width = the widest leaf, signed iff every leaf is
//! signed, operands extended by the region's sign, every operation wrapping at the
//! region's width), over every pair of leaves up to 6 bits and seeded random trees of
//! depth 3; plus the region rule's parity with `self_width_of`, the `$clog2` arm's
//! parity with the engine fold, wide leaves and x/z.

use super::*;
use crate::{BitPacked, ConstRepr, ConstVal};

#[derive(Clone, Debug)]
enum Tree {
    Leaf { w: u32, s: bool, v: u128 },
    Add(Box<Tree>, Box<Tree>),
    Sub(Box<Tree>, Box<Tree>),
    Clog2 { w: u32, s: bool, v: u128 },
}

#[derive(Default)]
struct Arena {
    exprs: Vec<Expr>,
    consts: Vec<ConstVal>,
}

impl Arena {
    fn konst(&mut self, w: u32, s: bool, v: u128, unk: u64) -> u32 {
        let lo = v as u64;
        let hi = (v >> 64) as u64;
        let nw = w.div_ceil(64).max(1) as usize;
        let mut val = vec![0u64; nw];
        val[0] = lo;
        if nw > 1 {
            val[1] = hi;
        }
        let mut u = vec![0u64; nw];
        u[0] = unk;
        self.consts.push(ConstVal {
            width: w,
            signed: s,
            repr: ConstRepr::Numeric,
            bits: BitPacked { val, unk: u },
        });
        self.exprs.push(Expr::Const {
            val: (self.consts.len() - 1) as u32,
        });
        (self.exprs.len() - 1) as u32
    }
    fn push(&mut self, e: Expr) -> u32 {
        self.exprs.push(e);
        (self.exprs.len() - 1) as u32
    }
    fn build(&mut self, t: &Tree) -> u32 {
        match t {
            Tree::Leaf { w, s, v } => self.konst(*w, *s, *v, 0),
            Tree::Add(a, b) | Tree::Sub(a, b) => {
                let l = self.build(a);
                let r = self.build(b);
                let op = if matches!(t, Tree::Add(..)) {
                    BinOp::Add
                } else {
                    BinOp::Sub
                };
                self.push(Expr::Binary { op, lhs: l, rhs: r })
            }
            Tree::Clog2 { w, s, v } => {
                let a = self.konst(*w, *s, *v, 0);
                self.push(Expr::SysFunc {
                    which: SysFuncId::Clog2,
                    args: vec![a],
                })
            }
        }
    }
    fn ctx(&self) -> ExprCtx<'_> {
        ExprCtx {
            exprs: &self.exprs,
            consts: &self.consts,
            nets: &[],
        }
    }
}

fn clog2_ref(v: u128) -> u128 {
    if v <= 1 {
        0
    } else {
        u128::from(128 - (v - 1).leading_zeros())
    }
}

/// (region width, signed) — §11.6.1: the widest operand; §11.8.1: signed iff all are.
fn region(t: &Tree) -> (u32, bool) {
    match t {
        Tree::Leaf { w, s, .. } => (*w, *s),
        Tree::Clog2 { .. } => (32, true),
        Tree::Add(a, b) | Tree::Sub(a, b) => {
            let (wa, sa) = region(a);
            let (wb, sb) = region(b);
            (wa.max(wb), sa && sb)
        }
    }
}

fn wrap(x: i128, w: u32) -> i128 {
    x.rem_euclid(1i128 << w)
}

/// The value of `t` modulo 2^w with operands extended per the region's sign.
fn eval_ref(t: &Tree, w: u32, s: bool) -> i128 {
    match t {
        Tree::Leaf { w: lw, s: ls, v } => {
            let raw = (*v & ((1u128 << lw) - 1)) as i128;
            let ext = if s && *ls && (raw >> (lw - 1)) & 1 == 1 {
                raw - (1i128 << lw)
            } else {
                raw
            };
            wrap(ext, w)
        }
        Tree::Clog2 { v, .. } => wrap(clog2_ref(*v) as i128, w),
        Tree::Add(a, b) => wrap(eval_ref(a, w, s) + eval_ref(b, w, s), w),
        Tree::Sub(a, b) => wrap(eval_ref(a, w, s) - eval_ref(b, w, s), w),
    }
}

fn reference(t: &Tree) -> i128 {
    let (w, s) = region(t);
    let r = eval_ref(t, w, s);
    if s && r >= 1i128 << (w - 1) {
        r - (1i128 << w)
    } else {
        r
    }
}

fn leaves() -> Vec<Tree> {
    let mut out = Vec::new();
    for w in 1..=6u32 {
        for s in [false, true] {
            for v in 0..(1u128 << w) {
                out.push(Tree::Leaf { w, s, v });
            }
        }
    }
    out
}

#[test]
fn every_pair_of_small_leaves_matches_the_reference() {
    let ls = leaves();
    let mut n = 0u64;
    for a in &ls {
        for b in &ls {
            for t in [
                Tree::Add(Box::new(a.clone()), Box::new(b.clone())),
                Tree::Sub(Box::new(a.clone()), Box::new(b.clone())),
            ] {
                let mut ar = Arena::default();
                let e = ar.build(&t);
                let want = reference(&t);
                assert_eq!(edge_bound_value(ar.ctx(), e), Ok(want), "{t:?}");
                let count = edge_value(ar.ctx(), EdgeSpec::Count { n: e });
                if want < 0 {
                    assert_eq!(count, Err(EdgeFault::Negative), "{t:?}");
                } else {
                    assert_eq!(count, Ok(want as u64), "{t:?}");
                }
                n += 1;
            }
        }
    }
    assert!(n > 100_000);
}

/// A seeded generator (no `rand` dependency): xorshift64.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
    fn leaf(&mut self, max_w: u32) -> Tree {
        let w = 1 + self.below(u64::from(max_w)) as u32;
        let s = self.below(2) == 1;
        let v = u128::from(self.next()) | (u128::from(self.next()) << 64);
        let v = if w >= 128 { v } else { v & ((1u128 << w) - 1) };
        if self.below(16) == 0 {
            // the engine arm folds a single-word, non-negative argument only
            let w = w.min(64);
            let v = if w >= 64 {
                v as u64 as u128
            } else {
                v & ((1u128 << w) - 1)
            };
            Tree::Clog2 { w, s: false, v }
        } else {
            Tree::Leaf { w, s, v }
        }
    }
    fn tree(&mut self, depth: u32, max_w: u32) -> Tree {
        if depth == 0 || self.below(3) == 0 {
            return self.leaf(max_w);
        }
        let a = Box::new(self.tree(depth - 1, max_w));
        let b = Box::new(self.tree(depth - 1, max_w));
        if self.below(2) == 0 {
            Tree::Add(a, b)
        } else {
            Tree::Sub(a, b)
        }
    }
}

#[test]
fn random_depth_three_trees_match_the_reference() {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    for max_w in [6u32, 16, 40, 100] {
        for _ in 0..20_000 {
            let t = rng.tree(3, max_w);
            let mut ar = Arena::default();
            let e = ar.build(&t);
            assert_eq!(edge_bound_value(ar.ctx(), e), Ok(reference(&t)), "{t:?}");
        }
    }
}

#[test]
fn the_region_is_the_canonical_self_width() {
    let mut rng = Rng(0x0123_4567_89AB_CDEF);
    for _ in 0..5_000 {
        let t = rng.tree(3, 70);
        let mut ar = Arena::default();
        let root = ar.build(&t);
        let mut sw: Vec<SelfWidth> = Vec::new();
        for i in 0..ar.exprs.len() as u32 {
            let w = self_width_of(ar.ctx(), &|_| None, &sw, i);
            sw.push(w);
        }
        let canon = sw[root as usize];
        assert_eq!(
            edge_region(ar.ctx(), root),
            Ok((canon.width, canon.signed)),
            "{t:?}"
        );
    }
}

#[test]
fn part_widths_follow_the_declared_direction() {
    for m in 0..8i128 {
        for l in 0..8i128 {
            let mut ar = Arena::default();
            let me = ar.konst(32, false, m as u128, 0);
            let le = ar.konst(32, false, l as u128, 0);
            let desc = edge_value(
                ar.ctx(),
                EdgeSpec::Part {
                    msb: me,
                    lsb: le,
                    desc: true,
                },
            );
            let asc = edge_value(
                ar.ctx(),
                EdgeSpec::Part {
                    msb: me,
                    lsb: le,
                    desc: false,
                },
            );
            if m >= l {
                assert_eq!(desc, Ok((m - l + 1) as u64));
            } else {
                assert_eq!(desc, Err(EdgeFault::Negative));
            }
            if l >= m {
                assert_eq!(asc, Ok((l - m + 1) as u64));
            } else {
                assert_eq!(asc, Err(EdgeFault::Negative));
            }
        }
    }
    // a signed negative bound: `[3 : N + 2]` with `N = -2` (32-bit int) is `[3:0]`
    let mut ar = Arena::default();
    let three = ar.konst(32, true, 3, 0);
    let n = ar.konst(32, true, u128::from(u32::MAX - 1), 0);
    let two = ar.konst(32, true, 2, 0);
    let lsb = ar.push(Expr::Binary {
        op: BinOp::Add,
        lhs: n,
        rhs: two,
    });
    assert_eq!(
        edge_value(
            ar.ctx(),
            EdgeSpec::Part {
                msb: three,
                lsb,
                desc: true
            }
        ),
        Ok(4)
    );
}

#[test]
fn wide_leaves_large_values_and_unknown_bits() {
    // 72-bit leaves: (2^64 + 5) - 2^64 = 5, past the engine fold's 2^24 clamp
    let mut ar = Arena::default();
    let a = ar.konst(72, false, (1u128 << 64) + 5, 0);
    let b = ar.konst(72, false, 1u128 << 64, 0);
    let d = ar.push(Expr::Binary {
        op: BinOp::Sub,
        lhs: a,
        rhs: b,
    });
    assert_eq!(edge_value(ar.ctx(), EdgeSpec::Count { n: d }), Ok(5));
    assert_eq!(const_u32_of_expr_ctx(ar.ctx(), d), Some(0)); // the engine's clamp (G1)
                                                             // values at and past 2^24, 2^32, 2^64
    for v in [
        1u128 << 24,
        (1u128 << 24) + 3,
        1u128 << 32,
        (1u128 << 64) + 9,
    ] {
        let mut ar = Arena::default();
        let c = ar.konst(80, false, v, 0);
        assert_eq!(
            edge_value(ar.ctx(), EdgeSpec::Count { n: c }),
            Ok(u64::try_from(v).unwrap_or(u64::MAX))
        );
    }
    // a 32-bit unsigned wrap: 32'hFFFF_FFFE + 6 = 4
    let mut ar = Arena::default();
    let u = ar.konst(32, false, 0xFFFF_FFFE, 0);
    let six = ar.konst(32, true, 6, 0);
    let s = ar.push(Expr::Binary {
        op: BinOp::Add,
        lhs: u,
        rhs: six,
    });
    assert_eq!(edge_value(ar.ctx(), EdgeSpec::Count { n: s }), Ok(4));
    // a signed 4-bit leaf in a signed region: 4'sb1111 + 5 = 4
    let mut ar = Arena::default();
    let m1 = ar.konst(4, true, 0xF, 0);
    let five = ar.konst(32, true, 5, 0);
    let s = ar.push(Expr::Binary {
        op: BinOp::Add,
        lhs: m1,
        rhs: five,
    });
    assert_eq!(edge_value(ar.ctx(), EdgeSpec::Count { n: s }), Ok(4));
    // …and in an unsigned one (5'd5): 15 + 5 = 20 (§11.8.1)
    let mut ar = Arena::default();
    let m1 = ar.konst(4, true, 0xF, 0);
    let five = ar.konst(5, false, 5, 0);
    let s = ar.push(Expr::Binary {
        op: BinOp::Add,
        lhs: m1,
        rhs: five,
    });
    assert_eq!(edge_value(ar.ctx(), EdgeSpec::Count { n: s }), Ok(20));
    // x/z
    let mut ar = Arena::default();
    let x = ar.konst(8, false, 0, 0b1);
    let one = ar.konst(8, false, 1, 0);
    let s = ar.push(Expr::Binary {
        op: BinOp::Add,
        lhs: x,
        rhs: one,
    });
    assert_eq!(
        edge_value(ar.ctx(), EdgeSpec::Count { n: s }),
        Err(EdgeFault::Unknown)
    );
    // a node outside the engine fold's arms
    let mut ar = Arena::default();
    let a = ar.konst(8, false, 3, 0);
    let b = ar.konst(8, false, 2, 0);
    let s = ar.push(Expr::Binary {
        op: BinOp::Mul,
        lhs: a,
        rhs: b,
    });
    assert_eq!(
        edge_value(ar.ctx(), EdgeSpec::Count { n: s }),
        Err(EdgeFault::Unreduced)
    );
}

#[test]
fn clog2_follows_the_engine_arm() {
    let mut rng = Rng(42);
    for _ in 0..5_000 {
        let w = 1 + rng.below(70) as u32;
        let s = rng.below(2) == 1;
        let v = u128::from(rng.next())
            & if w >= 128 {
                u128::MAX
            } else {
                (1u128 << w) - 1
            };
        let mut ar = Arena::default();
        let e = ar.build(&Tree::Clog2 { w, s, v });
        match const_u32_of_expr_ctx(ar.ctx(), e) {
            Some(k) => assert_eq!(edge_bound_value(ar.ctx(), e), Ok(i128::from(k))),
            None => assert_eq!(edge_bound_value(ar.ctx(), e), Err(EdgeFault::Unreduced)),
        }
    }
}
