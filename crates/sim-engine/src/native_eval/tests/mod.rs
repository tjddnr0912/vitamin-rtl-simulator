use super::*;
use sim_ir::SimIr;

mod arith_bits;
mod cmp_select;

/// A `NetReader` returning a fixed `Value` per NetId (and all-X for any other).
struct FakeNets(Vec<Value>);
impl NetReader for FakeNets {
    fn read_net(&self, net: u32, _word: Option<u32>) -> Value {
        self.0
            .get(net as usize)
            .cloned()
            .unwrap_or_else(|| Value::xs(1, false))
    }
}

fn nv(width: u32, signed: bool) -> sim_ir::NetVar {
    sim_ir::NetVar {
        kind: sim_ir::NetKind::Reg,
        width,
        msb: width.saturating_sub(1),
        lsb: 0,
        signed,
        array_len: 1,
        dir: sim_ir::PortDir::Internal,
        init: sim_ir::BitPacked {
            val: vec![0],
            unk: vec![0],
        },
    }
}

/// Minimal `SimIr` carrying only the arenas width inference + native eval read:
/// `exprs`, `consts`, `nets`. Everything else is empty.
fn ir_of(exprs: Vec<Expr>, consts: Vec<sim_ir::ConstVal>, nets: Vec<sim_ir::NetVar>) -> SimIr {
    SimIr {
        instances: vec![],
        nets,
        processes: vec![],
        cont_assigns: vec![],
        funcs: vec![],
        exprs,
        stmts: vec![],
        blocks: vec![],
        consts,
    }
}

/// Cross-check native `try_compile + run` against the interpreter oracle
/// `EvalCtx::eval_ctx` for `eid` in context `(ctx_w, ctx_signed)` over a set of
/// net `Value`s. Asserts the produced `Value`s are byte-identical (val, unk,
/// width, signed) — the same equality the P5 gate enforces end-to-end.
fn assert_matches_oracle(ir: &SimIr, eid: u32, ctx_w: u32, ctx_signed: bool, nets: &[Value]) {
    assert_matches_oracle_on(ir, eid, ctx_w, ctx_signed, &FakeNets(nets.to_vec()));
}

/// Generic core: same byte-identity contrast over ANY `NetReader` (the
/// array-indexed lane needs word-indexed fakes).
fn assert_matches_oracle_on(
    ir: &SimIr,
    eid: u32,
    ctx_w: u32,
    ctx_signed: bool,
    fake: &impl NetReader,
) {
    let wt = WidthTable::build(ir, &crate::FuncTable::new());
    let oracle = {
        let rng = crate::state::RngCells::default();
        let ctx = crate::eval::EvalCtx {
            ir,
            nets: fake,
            now: 0,
            wt: &wt,
            time_mult: 1,
            rng: &rng,
            plusargs: &[],
        };
        ctx.eval_ctx(eid, ctx_w, ctx_signed)
    };
    let prog = try_compile(ir, &wt, &ineligible_nets(ir), eid, ctx_w, ctx_signed)
        .expect("expression must be native-compilable in this test");
    let native = run(&prog, fake, &mut NativeScratch::default());
    assert_eq!(
        native, oracle,
        "native eval diverged from oracle for eid {eid} ctx ({ctx_w},{ctx_signed})"
    );
    // TRIVIAL-SHAPE SHORTCUT: when this program took the shortcut, the op loop is its
    // oracle — run it too and require byte identity. Every existing case in this file
    // therefore also covers the shortcut, including the `Const`-only and `LoadScalar`-only
    // programs that are 46.3% of real executions.
    if prog.fast_shape() != crate::native_eval::FastShape::Vm {
        let via_vm = run(
            &prog.forced_through_the_vm(),
            fake,
            &mut NativeScratch::default(),
        );
        assert_eq!(
            native,
            via_vm,
            "shortcut {:?} diverged from the op loop for eid {eid} ctx ({ctx_w},{ctx_signed})",
            prog.fast_shape()
        );
    }
}

fn sig(net: u32) -> Expr {
    Expr::Signal { net, word: None }
}
fn bin(op: BinOp, lhs: u32, rhs: u32) -> Expr {
    Expr::Binary { op, lhs, rhs }
}

// ── a 64-bit clean Value with given low word ──
fn v64(x: u64) -> Value {
    let mut v = Value::zeros(64, false);
    v.val[0] = x;
    v
}
// ── an X/Z-bearing Value: `xmask` marks unknown bits ──
fn v64_xz(val: u64, xmask: u64) -> Value {
    let mut v = Value::zeros(64, false);
    v.val[0] = val & !xmask;
    v.unk[0] = xmask;
    v
}

// ── follow-on increment: comparisons / shifts / div-mod / ternary /
//    reductions / logical (REMAINING_WORK "native-eval follow-on") ──

fn un(op: UnOp, operand: u32) -> Expr {
    Expr::Unary { op, operand }
}
fn vw(w: u32, x: u64) -> Value {
    let mut v = Value::zeros(w, false);
    v.val[0] = x & low_mask(w);
    v
}
fn vws(w: u32, x: u64) -> Value {
    let mut v = Value::zeros(w, true);
    v.val[0] = x & low_mask(w);
    v
}
fn vw_xz(w: u32, x: u64, xm: u64) -> Value {
    let mut v = Value::zeros(w, false);
    let m = low_mask(w);
    v.val[0] = x & !xm & m;
    v.unk[0] = xm & m;
    v
}

// ── follow-on increment 2: select / concat / replicate (REMAINING_WORK
//    "native-eval >64bit/real/select/concat lane" — the ≤64-bit half) ──

fn cnum(w: u32, x: u64) -> sim_ir::ConstVal {
    sim_ir::ConstVal {
        width: w,
        signed: false,
        repr: sim_ir::ConstRepr::Numeric,
        bits: sim_ir::BitPacked {
            val: vec![x],
            unk: vec![0],
        },
    }
}

// ── C6 lane: array-indexed Signal + the 65..=128-bit two-word wide lane ──

/// Per-net fake honoring the array word index (mirrors `net_word_packed`'s
/// OOR ⇒ all-X-at-element-width contract; the contrast only needs both
/// sides to see the SAME reader).
enum FakeNet {
    Scalar(Value),
    Array(Vec<Value>, u32), // elements, element width
}
struct FakeMem(Vec<FakeNet>);
impl NetReader for FakeMem {
    fn read_net(&self, net: u32, word: Option<u32>) -> Value {
        match (&self.0[net as usize], word) {
            (FakeNet::Scalar(v), None) => v.clone(),
            (FakeNet::Array(els, ew), Some(i)) => els
                .get(i as usize)
                .cloned()
                .unwrap_or_else(|| Value::xs(*ew, false)),
            (FakeNet::Scalar(v), Some(_)) => v.clone(),
            (FakeNet::Array(_, ew), None) => Value::xs(*ew, false),
        }
    }
}

/// 2-word Value builders for the wide lane.
fn vwide(w: u32, lo: u64, hi: u64) -> Value {
    let mut v = Value::zeros(w, false);
    v.val[0] = lo;
    if v.val.len() > 1 {
        v.val[1] = hi;
    }
    v.mask_top();
    v
}
fn vwide_s(w: u32, lo: u64, hi: u64) -> Value {
    let mut v = vwide(w, lo, hi);
    v.signed = true;
    v
}
fn vwide_xz(w: u32, lo: u64, hi: u64, xlo: u64, xhi: u64) -> Value {
    let mut v = vwide(w, lo & !xlo, hi & !xhi);
    v.unk[0] = xlo;
    if v.unk.len() > 1 {
        v.unk[1] = xhi;
    }
    v.mask_top();
    v
}
/// Two-word numeric const.
fn cnum2(w: u32, lo: u64, hi: u64) -> sim_ir::ConstVal {
    sim_ir::ConstVal {
        width: w,
        signed: false,
        repr: sim_ir::ConstRepr::Numeric,
        bits: sim_ir::BitPacked {
            val: vec![lo, hi],
            unk: vec![0, 0],
        },
    }
}

/// The TRIVIAL-SHAPE SHORTCUT, against the op loop it replaces.
///
/// 46.3% of native-eval executions on a real design are ONE op — 27.4% a lone
/// `LoadScalar`, 18.9% a lone `Const` — so `run` skips the loop for them entirely and
/// `NativeProg::fast` says which. The loop is the shortcut's oracle, and this is the
/// only place that contrast happens: every OTHER test in this file compiles a compound
/// expression, so not one of them produces a program the shortcut claims. Written after
/// discovering exactly that — the first version of this check was folded into
/// `assert_matches_oracle_on` and passed with the shortcut deliberately broken, because
/// its condition was never true.
///
/// Both shapes are swept over the value axes `run` actually branches on: net width vs
/// context width (which decides truncation vs extension), signedness (sign vs zero
/// extension), and X/Z bits (which must survive into `unk`).
#[test]
fn the_trivial_shape_shortcut_matches_the_op_loop() {
    let mut claimed = 0usize;
    for &(net_w, ctx_w) in &[
        (1u32, 1u32),
        (1, 8),
        (8, 8),
        (8, 4),
        (8, 32),
        (32, 32),
        (32, 64),
        (64, 64),
        (7, 33),
        (33, 7),
    ] {
        for &net_signed in &[false, true] {
            for &ctx_signed in &[false, true] {
                let ir = ir_of(vec![sig(0)], vec![], vec![nv(net_w, net_signed)]);
                let wt = WidthTable::build(&ir, &crate::FuncTable::new());
                let prog = try_compile(&ir, &wt, &ineligible_nets(&ir), 0, ctx_w, ctx_signed)
                    .expect("a bare signal read must compile");
                assert!(
                    matches!(
                        prog.fast_shape(),
                        crate::native_eval::FastShape::LoadScalar { .. }
                    ),
                    "a bare signal read is one op and must take the shortcut"
                );
                claimed += 1;

                // Values covering zero, all-ones, a set high bit (sign extension), and
                // both unknown states.
                let mut vals = vec![Value::zeros(net_w, net_signed)];
                let mut ones = Value::zeros(net_w, net_signed);
                for i in 0..net_w {
                    ones.set_vu(i, 1, 0);
                }
                vals.push(ones);
                let mut hi = Value::zeros(net_w, net_signed);
                hi.set_vu(net_w - 1, 1, 0);
                vals.push(hi);
                let mut xs = Value::zeros(net_w, net_signed);
                xs.set_vu(0, 0, 1);
                vals.push(xs);
                let mut zs = Value::zeros(net_w, net_signed);
                zs.set_vu(0, 1, 1);
                vals.push(zs);

                for v in vals {
                    let fake = FakeNets(vec![v.clone()]);
                    let via_fast = run(&prog, &fake, &mut NativeScratch::default());
                    let via_vm = run(
                        &prog.forced_through_the_vm(),
                        &fake,
                        &mut NativeScratch::default(),
                    );
                    assert_eq!(
                        via_fast, via_vm,
                        "LoadScalar shortcut diverged: net {net_w}/{net_signed} \
                         ctx {ctx_w}/{ctx_signed}"
                    );
                }
            }
        }
    }

    // The `Const`-only shape, over the same context axes. `unk` is swept too — a
    // literal can carry X/Z (`4'b10x1`), and a shortcut that returned only `val` would
    // otherwise pass every case here.
    for &(bits, unk) in &[
        (0u64, 0u64),
        (1, 0),
        (0xff, 0),
        (0x8000_0000, 0),
        (u64::MAX, 0),
        (0, 0xf),
        (0xff, 0x0f),
        (0x5555, 0xaaaa),
    ] {
        for &ctx_w in &[1u32, 8, 32, 64] {
            for &ctx_signed in &[false, true] {
                let ir = ir_of(
                    vec![Expr::Const { val: 0 }],
                    vec![sim_ir::ConstVal {
                        width: 64,
                        signed: false,
                        repr: sim_ir::ConstRepr::Numeric,
                        bits: sim_ir::BitPacked {
                            val: vec![bits],
                            unk: vec![unk],
                        },
                    }],
                    vec![],
                );
                let wt = WidthTable::build(&ir, &crate::FuncTable::new());
                let Some(prog) = try_compile(&ir, &wt, &ineligible_nets(&ir), 0, ctx_w, ctx_signed)
                else {
                    continue;
                };
                assert!(
                    matches!(
                        prog.fast_shape(),
                        crate::native_eval::FastShape::Const { .. }
                    ),
                    "a bare const is one op and must take the shortcut"
                );
                claimed += 1;
                let fake = FakeNets(vec![]);
                let via_fast = run(&prog, &fake, &mut NativeScratch::default());
                let via_vm = run(
                    &prog.forced_through_the_vm(),
                    &fake,
                    &mut NativeScratch::default(),
                );
                assert_eq!(
                    via_fast, via_vm,
                    "Const shortcut diverged: {bits:#x} ctx {ctx_w}"
                );
            }
        }
    }
    assert!(claimed > 60, "only {claimed} programs took the shortcut");
}
