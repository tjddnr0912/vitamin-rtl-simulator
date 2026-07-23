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
    let prog = try_compile(ir, &wt, eid, ctx_w, ctx_signed)
        .expect("expression must be native-compilable in this test");
    let native = run(&prog, fake);
    assert_eq!(
        native, oracle,
        "native eval diverged from oracle for eid {eid} ctx ({ctx_w},{ctx_signed})"
    );
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
