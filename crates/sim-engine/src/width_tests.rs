//! IEEE 1364-2005 context-determined width-inference tests (T1–T18).
//!
//! These build minimal `SimIr`s directly with the frozen constructors (no
//! parser dependency), then drive `eval` / `eval_for_lvalue` through a real
//! `SimState` + `Scheduler`. Placed inside the crate (rather than `tests/`)
//! because `SimState`/`Scheduler`/`eval_for_lvalue` are `pub(crate)`.

use sim_ir::{
    BinOp, BitPacked, ConstRepr, ConstVal, Expr, LvalChunk, Lvalue, NetKind, NetVar, PortDir,
    SelKind, SimIr, SysFuncId,
};

use crate::sched::Scheduler;
use crate::state::SimState;
use crate::width::{SelfWidth, WidthTable};

// ── builders for a hand-rolled SimIr ────────────────────────────────────────

#[derive(Default)]
struct IrBuilder {
    nets: Vec<NetVar>,
    consts: Vec<ConstVal>,
    exprs: Vec<Expr>,
}

impl IrBuilder {
    fn net(&mut self, width: u32, signed: bool) -> u32 {
        let id = self.nets.len() as u32;
        self.nets.push(NetVar {
            kind: NetKind::Reg,
            width,
            msb: width.saturating_sub(1),
            lsb: 0,
            signed,
            array_len: 1,
            dir: PortDir::Internal,
            init: BitPacked {
                val: vec![0],
                unk: vec![0],
            },
        });
        id
    }

    /// A net pre-loaded with a definite value (so reads return `val`).
    fn net_val(&mut self, width: u32, signed: bool, val: u64) -> u32 {
        let id = self.net(width, signed);
        self.nets[id as usize].init = BitPacked {
            val: vec![val],
            unk: vec![0],
        };
        id
    }

    /// A net pre-loaded with explicit (val, unk) planes (for X/Z bits).
    fn net_vu(&mut self, width: u32, signed: bool, val: u64, unk: u64) -> u32 {
        let id = self.net(width, signed);
        self.nets[id as usize].init = BitPacked {
            val: vec![val],
            unk: vec![unk],
        };
        id
    }

    fn signal(&mut self, net: u32) -> u32 {
        self.push_expr(Expr::Signal { net, word: None })
    }

    fn const_num(&mut self, width: u32, signed: bool, val: u64) -> u32 {
        let cid = self.consts.len() as u32;
        self.consts.push(ConstVal {
            width,
            signed,
            repr: ConstRepr::Numeric,
            bits: BitPacked {
                val: vec![val],
                unk: vec![0],
            },
        });
        self.push_expr(Expr::Const { val: cid })
    }

    fn const_real(&mut self, x: f64) -> u32 {
        let cid = self.consts.len() as u32;
        self.consts.push(ConstVal {
            width: 64,
            signed: true,
            repr: ConstRepr::Real,
            bits: BitPacked {
                val: vec![x.to_bits()],
                unk: vec![0],
            },
        });
        self.push_expr(Expr::Const { val: cid })
    }

    fn bin(&mut self, op: BinOp, lhs: u32, rhs: u32) -> u32 {
        self.push_expr(Expr::Binary { op, lhs, rhs })
    }

    fn ternary(&mut self, cond: u32, then_e: u32, else_e: u32) -> u32 {
        self.push_expr(Expr::Ternary {
            cond,
            then_e,
            else_e,
        })
    }

    fn concat(&mut self, parts: Vec<u32>) -> u32 {
        self.push_expr(Expr::Concat { parts })
    }

    fn sysfunc(&mut self, which: SysFuncId, args: Vec<u32>) -> u32 {
        self.push_expr(Expr::SysFunc { which, args })
    }

    /// `base[msb:lsb]` part-select with a const-folded `Add(Sub(msb,lsb),1)`
    /// width tree, mirroring elaborate's `width_from_msb_lsb_checked`.
    fn part_const(&mut self, base: u32, msb: u32, lsb: u32) -> u32 {
        let msb_c = self.const_num(32, false, msb as u64);
        let lsb_c = self.const_num(32, false, lsb as u64);
        let one = self.const_num(32, false, 1);
        let sub = self.bin(BinOp::Sub, msb_c, lsb_c);
        let width = self.bin(BinOp::Add, sub, one);
        let offset = self.const_num(32, false, lsb as u64);
        self.push_expr(Expr::Select {
            base,
            offset,
            width,
            kind: SelKind::PartConst,
        })
    }

    fn push_expr(&mut self, e: Expr) -> u32 {
        let id = self.exprs.len() as u32;
        self.exprs.push(e);
        id
    }

    fn build(self) -> SimIr {
        SimIr {
            instances: Vec::new(),
            nets: self.nets,
            processes: Vec::new(),
            cont_assigns: Vec::new(),
            funcs: Vec::new(),
            exprs: self.exprs,
            stmts: Vec::new(),
            blocks: Vec::new(),
            consts: self.consts,
        }
    }
}

/// Whole-net lvalue.
fn whole(net: u32) -> Lvalue {
    Lvalue {
        chunks: vec![LvalChunk {
            net,
            word: None,
            offset: None,
            width: None,
            kind: SelKind::Bit,
        }],
    }
}

/// A diagnostic sink that drops everything (for the width unit tests).
struct NullSink;
impl diag::LogSink for NullSink {
    fn emit(&self, _e: diag::LogEvent) {}
}

/// Build a SimState (no VCD) over `ir`, run a closure with a Scheduler.
fn with_sched<R>(ir: &SimIr, f: impl FnOnce(&Scheduler) -> R) -> R {
    let out: Box<dyn std::io::Write> = Box::new(std::io::sink());
    let null = NullSink;
    let mut st = SimState::new(ir, out, &null, "1ns".to_string(), "test".to_string(), None);
    let sched = Scheduler::new(&mut st, 1_000_000, None, Default::default());
    f(&sched)
}

fn table(ir: &SimIr) -> WidthTable {
    WidthTable::build(ir, &crate::FuncTable::new())
}

// ── T1: self-width table values ─────────────────────────────────────────────

#[test]
fn t1_self_width_table_values() {
    let mut b = IrBuilder::default();
    let na = b.net(4, false);
    let nb = b.net(5, true);
    let a = b.signal(na);
    let bb = b.signal(nb);
    let add = b.bin(BinOp::Add, a, bb);
    let eq = b.bin(BinOp::Eq, a, bb);
    let cat = b.concat(vec![a, bb]);
    let amt = b.const_num(32, false, 3);
    let shl = b.bin(BinOp::Shl, a, amt);
    let ir = b.build();
    let wt = table(&ir);

    assert_eq!(
        wt.get(a),
        SelfWidth {
            width: 4,
            signed: false
        }
    );
    assert_eq!(
        wt.get(bb),
        SelfWidth {
            width: 5,
            signed: true
        }
    );
    assert_eq!(
        wt.get(add),
        SelfWidth {
            width: 5,
            signed: false
        },
        "max(4,5), not both signed"
    );
    assert_eq!(
        wt.get(eq),
        SelfWidth {
            width: 1,
            signed: false
        }
    );
    assert_eq!(
        wt.get(cat),
        SelfWidth {
            width: 9,
            signed: false
        },
        "4+5 unsigned"
    );
    assert_eq!(
        wt.get(shl),
        SelfWidth {
            width: 4,
            signed: false
        },
        "LEFT width, amount irrelevant"
    );
}

// ── T2: narrow-add carry into wide lhs ──────────────────────────────────────

#[test]
fn t2_narrow_add_carry_into_wide_lhs() {
    let mut b = IrBuilder::default();
    let na = b.net_val(4, false, 9);
    let nb = b.net_val(4, false, 9);
    let nsum = b.net(5, false);
    let a = b.signal(na);
    let bb = b.signal(nb);
    let add = b.bin(BinOp::Add, a, bb);
    let ir = b.build();
    let sum_lhs = whole(nsum);
    with_sched(&ir, |sched| {
        let v = sched.eval_for_lvalue(&sum_lhs, add); // ctx_width = max(5,4) = 5
        assert_eq!(v.width, 5);
        assert_eq!(v.to_u64(), Some(18), "9+9=18 — carry in bit 4 PRESERVED");
    });
}

// ── T3: 1-bit comparison zero-extended into wide AND ────────────────────────

#[test]
fn t3_comparison_zero_extended_into_and() {
    // result = (x==y) & mask, 8-bit lhs.
    // mask=0xFF, x==y true → 0x01 & 0xFF = 0x01 (never sign-replicates).
    let mut b = IrBuilder::default();
    let nx = b.net_val(8, false, 0x33);
    let ny = b.net_val(8, false, 0x33);
    let nres = b.net(8, false);
    let x = b.signal(nx);
    let y = b.signal(ny);
    let eq = b.bin(BinOp::Eq, x, y);
    let mask = b.const_num(8, false, 0xFF);
    let and_expr = b.bin(BinOp::BitAnd, eq, mask);
    let ir = b.build();
    let res_lhs = whole(nres);
    with_sched(&ir, |sched| {
        let v = sched.eval_for_lvalue(&res_lhs, and_expr);
        assert_eq!(v.width, 8);
        assert_eq!(
            v.to_u64(),
            Some(0x01),
            "(x==y)→1 zero-ext to 8, &0xFF = 0x01"
        );
    });
}

#[test]
fn t3b_comparison_masked_to_zero() {
    // mask=0xF0, x==y true → 0x01 & 0xF0 = 0x00.
    let mut b = IrBuilder::default();
    let nx = b.net_val(8, false, 0x33);
    let ny = b.net_val(8, false, 0x33);
    let nres = b.net(8, false);
    let x = b.signal(nx);
    let y = b.signal(ny);
    let eq = b.bin(BinOp::Eq, x, y);
    let mask = b.const_num(8, false, 0xF0);
    let and_expr = b.bin(BinOp::BitAnd, eq, mask);
    let ir = b.build();
    let res_lhs = whole(nres);
    with_sched(&ir, |sched| {
        let v = sched.eval_for_lvalue(&res_lhs, and_expr);
        assert_eq!(v.width, 8);
        assert_eq!(v.to_u64(), Some(0x00));
    });
}

// ── T4: signed × unsigned becomes unsigned ──────────────────────────────────

#[test]
fn t4_signed_times_unsigned_is_unsigned() {
    let mut b = IrBuilder::default();
    let ns = b.net_val(8, true, 0xFF); // -1 signed
    let nu = b.net_val(8, false, 2);
    let np = b.net(16, false);
    let s = b.signal(ns);
    let u = b.signal(nu);
    let mul = b.bin(BinOp::Mul, s, u);
    let ir = b.build();
    let p_lhs = whole(np);
    with_sched(&ir, |sched| {
        let v = sched.eval_for_lvalue(&p_lhs, mul); // ctx 16; u unsigned ⇒ unsigned
        assert_eq!(v.width, 16);
        assert_eq!(
            v.to_u64(),
            Some(0x00FF * 2),
            "s ZERO-extended to 0x00FF, ×2 = 0x01FE = 510"
        );
        assert!(!v.signed);
    });
}

// ── T5: arithmetic shift sign (`>>>` on signed) ─────────────────────────────

#[test]
fn t5_arithmetic_shift_signed() {
    let mut b = IrBuilder::default();
    let ns = b.net_val(8, true, 0x80); // -128
    let nr = b.net(8, true);
    let s = b.signal(ns);
    let amt = b.const_num(32, false, 2);
    let ashr = b.bin(BinOp::AShr, s, amt);
    let ir = b.build();
    let r_lhs = whole(nr);
    with_sched(&ir, |sched| {
        let v = sched.eval_for_lvalue(&r_lhs, ashr);
        assert_eq!(v.width, 8);
        assert_eq!(
            v.to_u64(),
            Some(0xE0),
            "-128>>>2 = -32 = 0xE0 (sign-filled)"
        );
    });
}

// ── T6: logical shift on same value (`>>`) ──────────────────────────────────

#[test]
fn t6_logical_shift() {
    let mut b = IrBuilder::default();
    let ns = b.net_val(8, true, 0x80);
    let nr = b.net(8, true);
    let s = b.signal(ns);
    let amt = b.const_num(32, false, 2);
    let shr = b.bin(BinOp::Shr, s, amt);
    let ir = b.build();
    let r_lhs = whole(nr);
    with_sched(&ir, |sched| {
        let v = sched.eval_for_lvalue(&r_lhs, shr);
        assert_eq!(v.to_u64(), Some(0x20), "zero-filled: 0x80>>2 = 0x20");
    });
}

// ── T7: concat operand NOT widened by context ───────────────────────────────

#[test]
fn t7_concat_operand_not_widened() {
    let mut b = IrBuilder::default();
    let na = b.net_val(4, false, 0xA);
    let nb = b.net_val(4, false, 0x5);
    let nw = b.net(12, false);
    let a = b.signal(na);
    let bb = b.signal(nb);
    let cat = b.concat(vec![a, bb]);
    let ir = b.build();
    let w_lhs = whole(nw);
    with_sched(&ir, |sched| {
        let v = sched.eval_for_lvalue(&w_lhs, cat); // cat self-width 8, ctx 12
        assert_eq!(v.width, 12);
        assert_eq!(
            v.to_u64(),
            Some(0x0A5),
            "{{A,5}}=0xA5 then zero-extended to 12 bits"
        );
    });
}

// ── T8: ternary branch widening ─────────────────────────────────────────────

#[test]
fn t8_ternary_branch_widening() {
    // out = c ? hi : lo, out 8-bit. hi=0xAB (8b), lo=0xC (4b), c=0 → lo.
    let mut b = IrBuilder::default();
    let nhi = b.net_val(8, false, 0xAB);
    let nlo = b.net_val(4, false, 0xC);
    let nc = b.net_val(1, false, 0);
    let nout = b.net(8, false);
    let hi = b.signal(nhi);
    let lo = b.signal(nlo);
    let c = b.signal(nc);
    let tern = b.ternary(c, hi, lo);
    let ir = b.build();
    let out_lhs = whole(nout);
    with_sched(&ir, |sched| {
        let v = sched.eval_for_lvalue(&out_lhs, tern);
        assert_eq!(v.width, 8);
        assert_eq!(
            v.to_u64(),
            Some(0x0C),
            "lo selected, delivered 8-bit (0x0C)"
        );
    });

    // c=1 → hi
    let mut b2 = IrBuilder::default();
    let nhi = b2.net_val(8, false, 0xAB);
    let nlo = b2.net_val(4, false, 0xC);
    let nc = b2.net_val(1, false, 1);
    let nout = b2.net(8, false);
    let hi = b2.signal(nhi);
    let lo = b2.signal(nlo);
    let c = b2.signal(nc);
    let tern = b2.ternary(c, hi, lo);
    let ir2 = b2.build();
    let out_lhs2 = whole(nout);
    with_sched(&ir2, |sched| {
        let v = sched.eval_for_lvalue(&out_lhs2, tern);
        assert_eq!(v.to_u64(), Some(0xAB));
    });
}

// ── T9: comparison operands mutually sized (narrow vs wide) ─────────────────

#[test]
fn t9_comparison_operands_mutually_sized() {
    // n 4-bit=0xF (15), m 8-bit=0x0F (15), n==m → 1.
    let mut b = IrBuilder::default();
    let nn = b.net_val(4, false, 0xF);
    let nm = b.net_val(8, false, 0x0F);
    let n = b.signal(nn);
    let m = b.signal(nm);
    let eq = b.bin(BinOp::Eq, n, m);
    let ir = b.build();
    with_sched(&ir, |sched| {
        let v = sched.eval(eq); // self-determined 1-bit
        assert_eq!(v.width, 1);
        assert_eq!(v.to_u64(), Some(1), "15==15: n zero-extended to 8");
    });

    // m=0xFF: 15 != 255 → 0.
    let mut b2 = IrBuilder::default();
    let nn = b2.net_val(4, false, 0xF);
    let nm = b2.net_val(8, false, 0xFF);
    let n = b2.signal(nn);
    let m = b2.signal(nm);
    let eq = b2.bin(BinOp::Eq, n, m);
    let ir2 = b2.build();
    with_sched(&ir2, |sched| {
        let v = sched.eval(eq);
        assert_eq!(v.to_u64(), Some(0), "15 != 255");
    });
}

// ── T10: shift amount is self-determined ────────────────────────────────────

#[test]
fn t10_shift_amount_self_determined() {
    // out[7:0] = l4 << sh, l=0x3, sh=1 → 0x06.
    let mut b = IrBuilder::default();
    let nl = b.net_val(4, false, 0x3);
    let nsh = b.net_val(32, false, 1);
    let nout = b.net(8, false);
    let l = b.signal(nl);
    let sh = b.signal(nsh);
    let shl = b.bin(BinOp::Shl, l, sh);
    let ir = b.build();
    let out_lhs = whole(nout);
    with_sched(&ir, |sched| {
        let v = sched.eval_for_lvalue(&out_lhs, shl);
        assert_eq!(v.width, 8);
        assert_eq!(v.to_u64(), Some(0x06), "3<<1=6, no bits lost");
    });

    // l=0x9, out 4-bit: 9<<1=18 & 0xF = 2.
    let mut b2 = IrBuilder::default();
    let nl = b2.net_val(4, false, 0x9);
    let nsh = b2.net_val(32, false, 1);
    let nout = b2.net(4, false);
    let l = b2.signal(nl);
    let sh = b2.signal(nsh);
    let shl = b2.bin(BinOp::Shl, l, sh);
    let ir2 = b2.build();
    let out_lhs2 = whole(nout);
    with_sched(&ir2, |sched| {
        let v = sched.eval_for_lvalue(&out_lhs2, shl);
        assert_eq!(v.to_u64(), Some(2), "9<<1=18 truncated to 4-bit = 2");
    });
}

// ── T11: `$signed`/`$unsigned` width+sign ───────────────────────────────────

#[test]
fn t11_signed_unsigned_width_sign() {
    // $signed(u) + 4'sd0 into 8-bit signed lhs; u unsigned 4-bit = 0xF (-1).
    let mut b = IrBuilder::default();
    let nu = b.net_val(4, false, 0xF);
    let ns8 = b.net(8, true);
    let u = b.signal(nu);
    let su = b.sysfunc(SysFuncId::Signed, vec![u]);
    let zero = b.const_num(4, true, 0);
    let add_signed = b.bin(BinOp::Add, su, zero);
    let ir = b.build();
    let s8_lhs = whole(ns8);
    with_sched(&ir, |sched| {
        let v = sched.eval_for_lvalue(&s8_lhs, add_signed);
        assert_eq!(v.to_u64(), Some(0xFF), "-1 sign-extended to 8 bits");
    });

    // $unsigned of a signed −1 4-bit → 0x0F (zero-extended).
    let mut b2 = IrBuilder::default();
    let ns = b2.net_val(4, true, 0xF); // -1 signed
    let nu8 = b2.net(8, false);
    let s = b2.signal(ns);
    let unsigned_of_neg = b2.sysfunc(SysFuncId::Unsigned, vec![s]);
    let ir2 = b2.build();
    let u8_lhs = whole(nu8);
    with_sched(&ir2, |sched| {
        let w = sched.eval_for_lvalue(&u8_lhs, unsigned_of_neg);
        assert_eq!(w.to_u64(), Some(0x0F), "zero-extended, stays 15");
    });
}

// ── T12: `$clog2` fixed 32-bit, `$time` 64-bit ──────────────────────────────

#[test]
fn t12_clog2_time_fixed_widths() {
    let mut b = IrBuilder::default();
    let eight = b.const_num(8, false, 8);
    let clog2 = b.sysfunc(SysFuncId::Clog2, vec![eight]);
    let time = b.sysfunc(SysFuncId::Time, vec![]);
    let ir = b.build();
    let wt = table(&ir);
    assert_eq!(
        wt.get(clog2),
        SelfWidth {
            width: 32,
            signed: true
        }
    );
    assert_eq!(
        wt.get(time),
        SelfWidth {
            width: 64,
            signed: false
        }
    );
    with_sched(&ir, |sched| {
        let v = sched.eval(clog2);
        assert_eq!(v.width, 32);
        assert_eq!(v.to_u64(), Some(3), "clog2(8)=3");
    });
}

// ── T13: REGRESSION — equal-width eval == eval_for_lvalue ────────────────────

#[test]
fn t13_equal_width_eval_eq_eval_for_lvalue() {
    // A corpus of equal-width exprs: every one must give field-identical results
    // for self-determined eval vs eval_for_lvalue at matching width.
    fn check(make: impl Fn(&mut IrBuilder) -> (u32, u32)) {
        let mut b = IrBuilder::default();
        let (expr, lhs_net) = make(&mut b);
        let ir = b.build();
        let lhs = whole(lhs_net);
        with_sched(&ir, |sched| {
            let old = sched.eval(expr);
            let new = sched.eval_for_lvalue(&lhs, expr);
            assert_eq!(old.width, new.width, "width");
            assert_eq!(old.val, new.val, "val");
            assert_eq!(old.unk, new.unk, "unk");
            assert_eq!(old.signed, new.signed, "signed");
        });
    }

    // a & b, 8-bit
    check(|b| {
        let na = b.net_val(8, false, 0xAA);
        let nb = b.net_val(8, false, 0x0F);
        let nc = b.net(8, false);
        let a = b.signal(na);
        let bb = b.signal(nb);
        (b.bin(BinOp::BitAnd, a, bb), nc)
    });
    // a + b, 8-bit
    check(|b| {
        let na = b.net_val(8, false, 0x12);
        let nb = b.net_val(8, false, 0x34);
        let nc = b.net(8, false);
        let a = b.signal(na);
        let bb = b.signal(nb);
        (b.bin(BinOp::Add, a, bb), nc)
    });
    // a | b, 8-bit
    check(|b| {
        let na = b.net_val(8, false, 0xA0);
        let nb = b.net_val(8, false, 0x0B);
        let nc = b.net(8, false);
        let a = b.signal(na);
        let bb = b.signal(nb);
        (b.bin(BinOp::BitOr, a, bb), nc)
    });
    // a ^ b, 8-bit
    check(|b| {
        let na = b.net_val(8, false, 0xFF);
        let nb = b.net_val(8, false, 0x0F);
        let nc = b.net(8, false);
        let a = b.signal(na);
        let bb = b.signal(nb);
        (b.bin(BinOp::BitXor, a, bb), nc)
    });
    // {a,b} into exact 8-bit
    check(|b| {
        let na = b.net_val(4, false, 0xA);
        let nb = b.net_val(4, false, 0x5);
        let nc = b.net(8, false);
        let a = b.signal(na);
        let bb = b.signal(nb);
        (b.concat(vec![a, bb]), nc)
    });
    // ternary equal-branches, 8-bit (cond unknown → merge)
    check(|b| {
        let nhi = b.net_val(8, false, 0xAB);
        let nlo = b.net_val(8, false, 0xAB);
        let nc = b.net(8, false); // cond X (uninit) → unknown branch
        let nout = b.net(8, false);
        let hi = b.signal(nhi);
        let lo = b.signal(nlo);
        let c = b.signal(nc);
        (b.ternary(c, hi, lo), nout)
    });
}

// ── T14: X/Z preservation unchanged under widening ──────────────────────────

#[test]
fn t14_xz_preservation_under_widening() {
    // a 4-bit = 4'b10x1 (bit1 = X), sum = a + 4'd0 into 8-bit lhs.
    // val plane 0b1011 with bit1 unknown: val=0b1011, unk=0b0010.
    let mut b = IrBuilder::default();
    let na = b.net_vu(4, false, 0b1011, 0b0010);
    let nw8 = b.net(8, false);
    let a = b.signal(na);
    let zero = b.const_num(4, false, 0);
    let add0 = b.bin(BinOp::Add, a, zero);
    let ir = b.build();
    let w8_lhs = whole(nw8);
    with_sched(&ir, |sched| {
        let v = sched.eval_for_lvalue(&w8_lhs, add0);
        assert_eq!(v.width, 8);
        // arith poisons whole result to X on any operand X/Z (documented v1).
        assert_eq!(v.get_vu(1), (0, 1), "bit1 still X");
    });

    // Bitwise path keeps non-X bits clean while extending; verify zero-extension.
    let mut b2 = IrBuilder::default();
    let na = b2.net_vu(4, false, 0b1011, 0b0010);
    let nw8 = b2.net(8, false);
    let a = b2.signal(na);
    let allone = b2.const_num(4, false, 0xF);
    let or_expr = b2.bin(BinOp::BitOr, a, allone); // |0xF clears low X via OR-1
    let ir2 = b2.build();
    let w8_lhs2 = whole(nw8);
    with_sched(&ir2, |sched| {
        let v = sched.eval_for_lvalue(&w8_lhs2, or_expr);
        assert_eq!(v.get_vu(4), (0, 0), "zero-extended (unsigned)");
    });
}

// ── T15: MANDATORY — `>>>` keeps sign-fill under UNSIGNED context ────────────

#[test]
fn t15_ashr_keeps_sign_under_unsigned_ctx() {
    // s signed 8-bit = 0x80 (-128); r = (s>>>2) into UNSIGNED 8-bit lvalue.
    let mut b = IrBuilder::default();
    let ns = b.net_val(8, true, 0x80);
    let nu8 = b.net(8, false); // unsigned lvalue
    let s = b.signal(ns);
    let amt = b.const_num(32, false, 2);
    let ashr = b.bin(BinOp::AShr, s, amt);
    let ir = b.build();
    let u8_lhs = whole(nu8);
    with_sched(&ir, |sched| {
        let v = sched.eval_for_lvalue(&u8_lhs, ashr); // ctx unsigned
        assert_eq!(v.width, 8);
        assert_eq!(v.to_u64(), Some(0xE0), "ARITHMETIC fill (sign-bit), -32");
        assert_ne!(v.to_u64(), Some(0x20), "NOT the buggy logical-fill 0x20");
    });
}

// ── T16: MANDATORY — `**` keeps BASE sign under UNSIGNED exponent ────────────

#[test]
fn t16_pow_keeps_base_sign_unsigned_exponent() {
    // b signed 4-bit = 0xF (-1), e unsigned = 3; p = b**e into signed 8-bit lhs.
    let mut bld = IrBuilder::default();
    let nb = bld.net_val(4, true, 0xF); // -1 signed
    let ne = bld.net_val(4, false, 3); // unsigned exponent
    let ns8 = bld.net(8, true);
    let base = bld.signal(nb);
    let exp = bld.signal(ne);
    let pow = bld.bin(BinOp::Pow, base, exp);
    let ir = bld.build();
    let wt = table(&ir);
    assert_eq!(
        wt.get(pow),
        SelfWidth {
            width: 4,
            signed: true
        },
        "max(4, e.width)→base width; BASE-signed"
    );
    let s8_lhs = whole(ns8);
    with_sched(&ir, |sched| {
        let v = sched.eval_for_lvalue(&s8_lhs, pow);
        assert_eq!(
            v.to_u64(),
            Some(0xFF),
            "(-1)**3 = -1 = 0xFF (sign-extended)"
        );
        assert_ne!(
            v.to_u64(),
            Some(3375 & 0xFF),
            "NOT the buggy both-signed zero-extend (15**3)"
        );
    });
}

// ── T17: MANDATORY — `$signed(x)` in an UNSIGNED region zero-extends ─────────

#[test]
fn t17_signed_in_unsigned_region_zero_extends() {
    // r = $signed(u4) | s8 ; the OR has an unsigned sibling so region unsigned.
    // u4 = 0xF, s8 = 0x00. Result 0x0F (zero-extended, not 0xFF).
    let mut b = IrBuilder::default();
    let nu4 = b.net_val(4, false, 0xF);
    let ns8 = b.net_val(8, false, 0x00); // unsigned sibling
    let nu8 = b.net(8, false);
    let u4 = b.signal(nu4);
    let s8 = b.signal(ns8);
    let signed_u4 = b.sysfunc(SysFuncId::Signed, vec![u4]);
    let or_expr = b.bin(BinOp::BitOr, signed_u4, s8);
    let ir = b.build();
    let u8_lhs = whole(nu8);
    with_sched(&ir, |sched| {
        let v = sched.eval_for_lvalue(&u8_lhs, or_expr);
        assert_eq!(v.to_u64(), Some(0x0F), "$signed(0xF) ZERO-extended, |0x00");
        assert_ne!(
            v.to_u64(),
            Some(0xFF),
            "NOT a fragile unconditional sign-ext"
        );
    });
}

// ── T18: MANDATORY — const-folded part-select width feeds parent context ────

#[test]
fn t18_part_select_width_fold_feeds_context() {
    // c 12-bit; sel = c[11:4] (8-bit range); out = c[11:4] + d[3:0] into wide lhs.
    // The MANDATORY pin is the WIDTH-TABLE fold: `Select.width` is an ExprId
    // pointing at `Add(Sub(11,4),1)`; the table must const-fold it to 8 (NOT the
    // fallback 1) so the parent add sizes to max(8,4)=8 rather than 4.
    //
    // (The exact VALUE of a part-select is governed by `eval_select`, whose
    // pre-existing v1 behavior — treating `Select.width` as a literal — is
    // explicitly OUT OF SCOPE per the spec §10. We therefore assert the width
    // contribution, which is what this fix actually changes.)
    let mut b = IrBuilder::default();
    let nc = b.net_val(12, false, 0xABC);
    let nd = b.net_val(4, false, 0x1);
    let out_w = 8u32;
    let nout = b.net(out_w, false);
    let c = b.signal(nc);
    let d = b.signal(nd);
    let sel = b.part_const(c, 11, 4);
    let add = b.bin(BinOp::Add, sel, d);
    let ir = b.build();
    let wt = table(&ir);
    assert_eq!(
        wt.get(sel),
        SelfWidth {
            width: 8,
            signed: false
        },
        "(11-4)+1 = 8, NOT fallback 1"
    );
    // The add must size to max(self_width(sel)=8, self_width(d)=4) = 8, NOT 4.
    assert_eq!(
        wt.get(add).width,
        8,
        "add sized to max(8,4)=8 (REGRESSION: old Sub-only fold gives sel=1 → add=4)"
    );
    let out_lhs = whole(nout);
    with_sched(&ir, |sched| {
        let v = sched.eval_for_lvalue(&out_lhs, add); // add sized to max(8,4)=8
        assert_eq!(
            v.width.max(out_w),
            out_w,
            "select contributed its full 8 bits (not under-sized to 1)"
        );
    });
}

// ── REAL self-width (ConstRepr::Real + the four real-domain sysfuncs) ────────

#[test]
fn real_const_self_width_is_64_signed() {
    let mut b = IrBuilder::default();
    let c = b.const_real(3.5);
    let ir = b.build();
    let wt = table(&ir);
    assert_eq!(
        wt.get(c),
        SelfWidth {
            width: 64,
            signed: true
        },
        "a real const is {{64, signed}}"
    );
}

#[test]
fn real_sysfunc_self_widths() {
    let mut b = IrBuilder::default();
    let nr = b.net(64, true);
    let r = b.signal(nr);
    let ni = b.net(32, true);
    let i = b.signal(ni);
    let rtoi = b.sysfunc(SysFuncId::Rtoi, vec![r]);
    let itor = b.sysfunc(SysFuncId::Itor, vec![i]);
    let r2b = b.sysfunc(SysFuncId::RealToBits, vec![r]);
    let b2r = b.sysfunc(SysFuncId::BitsToReal, vec![r]);
    let rt = b.sysfunc(SysFuncId::Realtime, vec![]);
    let ir = b.build();
    let wt = table(&ir);
    // $rtoi → 32-bit signed integer.
    assert_eq!(
        wt.get(rtoi),
        SelfWidth {
            width: 32,
            signed: true
        }
    );
    // $itor / $bitstoreal → 64-bit signed (real domain).
    assert_eq!(
        wt.get(itor),
        SelfWidth {
            width: 64,
            signed: true
        }
    );
    assert_eq!(
        wt.get(b2r),
        SelfWidth {
            width: 64,
            signed: true
        }
    );
    // $realtobits → 64-bit unsigned raw vector.
    assert_eq!(
        wt.get(r2b),
        SelfWidth {
            width: 64,
            signed: false
        }
    );
    // $realtime self-width stays {64, false} (the §2.3 resize guard protects bits).
    assert_eq!(
        wt.get(rt),
        SelfWidth {
            width: 64,
            signed: false
        }
    );
}
