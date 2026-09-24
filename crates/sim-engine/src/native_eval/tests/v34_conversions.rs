//! format_version 34: `SysFuncId::RealToInt` and `SysFuncId::TwoState`, the
//! single-mention store conversions the inline function lane emits. The
//! interpreter is their only evaluator — the native lane must DECLINE both, so a
//! compiled program never answers for them.

use super::*;

fn real_net() -> sim_ir::NetVar {
    let mut n = nv(64, true);
    n.kind = sim_ir::NetKind::Real;
    n
}

fn eval_interp(ir: &SimIr, eid: u32, ctx_w: u32, ctx_signed: bool, nets: &[Value]) -> Value {
    let wt = WidthTable::build(ir, &crate::FuncTable::new());
    let fake = FakeNets(nets.to_vec());
    let rng = crate::state::RngCells::default();
    let ctx = crate::eval::EvalCtx {
        ir,
        nets: &fake,
        now: 0,
        wt: &wt,
        time_mult: 1,
        rng: &rng,
        plusargs: &[],
    };
    ctx.eval_ctx(eid, ctx_w, ctx_signed)
}

fn rti_ir() -> SimIr {
    ir_of(
        vec![
            sig(0),
            Expr::SysFunc {
                which: sim_ir::SysFuncId::RealToInt,
                args: vec![0],
            },
        ],
        vec![],
        vec![real_net()],
    )
}

fn rti(x: f64) -> Value {
    eval_interp(&rti_ir(), 1, 128, true, &[Value::from_f64(x)])
}

#[test]
fn real_to_int_rounds_half_away_from_zero_into_a_signed_128_bit_integer() {
    for (x, want) in [
        (2.5, 3i128),
        (-2.5, -3),
        (0.49, 0),
        (-0.5, -1),
        (300.7, 301),
        // 2^52 + 1: exact (the f64 ulp is 1 there, so `x + 0.5` would be a tie)
        (4_503_599_627_370_497.0, 4_503_599_627_370_497),
        // beyond i64: exact up to 2^127
        (2f64.powi(70), 1i128 << 70),
        (-(2f64.powi(70)), -(1i128 << 70)),
        (1e30, 1_000_000_000_000_000_019_884_624_838_656),
        (f64::NAN, 0),
    ] {
        let v = rti(x);
        assert!(!v.is_real, "{x}: the result is an integer");
        assert_eq!((v.width, v.signed), (128, true), "{x}");
        assert!(!v.has_xz(), "{x}: never unknown");
        assert_eq!(v.to_i128_signed(), Some(want), "{x}");
    }
}

#[test]
fn real_to_int_out_of_range_matches_the_net_store() {
    // At and beyond 2^127 the conversion saturates to the i128 extremes, exactly
    // as `real_to_int_round` does at a 128-bit net store — so a converted operand
    // and a stored one cannot disagree.
    for x in [1e40, -1e40, f64::INFINITY, f64::NEG_INFINITY] {
        let v = rti(x);
        assert_eq!(v, crate::value::real_to_int_round(x, 128, true), "{x}");
        let want = if x > 0.0 { i128::MAX } else { i128::MIN };
        assert_eq!(v.to_i128_signed(), Some(want), "{x}");
    }
}

#[test]
fn real_to_int_of_an_unknown_integral_operand_is_zero() {
    // Elaborate emits it only over a real; the documented fallback for an
    // integral operand with any unknown bit is 0 for the whole value, never x.
    let ir = ir_of(
        vec![
            sig(0),
            Expr::SysFunc {
                which: sim_ir::SysFuncId::RealToInt,
                args: vec![0],
            },
        ],
        vec![],
        vec![nv(8, false)],
    );
    let v = eval_interp(&ir, 1, 128, true, &[vw_xz(8, 0x07, 0x80)]);
    assert!(!v.has_xz());
    assert_eq!(v.to_i128_signed(), Some(0));
    let v = eval_interp(&ir, 1, 128, true, &[vw(8, 0x2d)]);
    assert_eq!(v.to_i128_signed(), Some(0x2d));
}

#[test]
fn two_state_drops_x_and_z_and_keeps_width_and_sign() {
    let ir = ir_of(
        vec![
            sig(0),
            Expr::SysFunc {
                which: sim_ir::SysFuncId::TwoState,
                args: vec![0],
            },
        ],
        vec![],
        vec![nv(8, true)],
    );
    // bit 7 = x (val 0, unk 1), bit 6 = z (val 1, unk 1), bits 2..0 = 1.
    let mut a = Value::zeros(8, true);
    a.val[0] = 0b0100_0111;
    a.unk[0] = 0b1100_0000;
    let v = eval_interp(&ir, 1, 8, true, std::slice::from_ref(&a));
    assert!(!v.has_xz());
    assert_eq!((v.width, v.signed), (8, true));
    assert_eq!(v.val[0], 0b0000_0111);
    // A negative, fully known value passes unchanged and sign-extends in a wider
    // signed context.
    let v = eval_interp(&ir, 1, 16, true, &[vws(8, 0xfd)]);
    assert_eq!(v.width, 16);
    assert_eq!(v.to_i128_signed(), Some(-3));
}

#[test]
fn the_native_lane_declines_both() {
    // Over an 8-bit integral net, which the native lane compiles on its own, so
    // the decline is the conversion node's and not the operand's.
    for which in [sim_ir::SysFuncId::RealToInt, sim_ir::SysFuncId::TwoState] {
        let ir = ir_of(
            vec![
                sig(0),
                Expr::SysFunc {
                    which,
                    args: vec![0],
                },
            ],
            vec![],
            vec![nv(8, false)],
        );
        let wt = WidthTable::build(&ir, &crate::FuncTable::new());
        assert!(try_compile(&ir, &wt, &ineligible_nets(&ir), 0, 8, false).is_some());
        assert!(
            try_compile(&ir, &wt, &ineligible_nets(&ir), 1, 64, false).is_none(),
            "{which:?}"
        );
    }
}
