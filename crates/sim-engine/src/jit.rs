//! NATIVE CODEGEN (option A) — cranelift-backed machine code for a `NativeProg`.
//!
//! Behind the `jit` feature and OFF by default. This is the third implementation of
//! expression semantics in this engine, after the tree-walking interpreter and the
//! bytecode VM, and the second one drifted from the first in four independent, silent
//! ways (§4.5.279). It exists to answer a measured question, not because the other two
//! were insufficient.
//!
//! WHAT IT CAN AND CANNOT REMOVE. Every leaf load is a CALL back into Rust — the net
//! table is a Rust data structure with real branches (frame-local, handle, array, wide),
//! and `load_scalar` is where they live. So a compiled program is: call, call, a handful
//! of inline instructions, return. What the machine code removes is the op dispatch and
//! the value stack; what it keeps is the calls and the arithmetic. On the measured
//! distribution the multi-op programs average three ops, of which one or two are loads.
//!
//! DETERMINISM. Cranelift IR defines its own semantics rather than passing the host
//! ISA's through: `ushr(x, 64)` is `ushr(x, 0)` on both aarch64 and x86-64, because the
//! shift count is masked by the IR, not by the machine. Measured on aarch64-apple-darwin
//! — the earlier claim that x86 and arm shift semantics would diverge HERE was wrong.
//! What remains is that cranelift's definition is not Verilog's (Verilog wants 0 for a
//! count at or above the width), which is one architecture-independent guard, not a
//! per-target reimplementation. Ops whose Verilog semantics this module does not
//! reproduce exactly are simply refused (`None`), and the program runs on the VM.

use cranelift_codegen::ir::{types, AbiParam, InstBuilder, Value as CV};
use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module};

use crate::native_eval::NOp;
use crate::value::low_mask;

/// A `(val, unk)` word pair returned in REGISTERS. `#[repr(C)]` with two `u64`s comes
/// back in `x0:x1` / `rax:rdx`, so a compiled function hands its result over without
/// touching memory — the first version wrote through an out-pointer and paid two stores
/// plus two loads on every evaluation, and the leaf callback did the same again.
#[repr(C)]
pub(crate) struct Pair(pub u64, pub u64);

macro_rules! ctx {
    ($p:expr) => {
        // SAFETY: every shim is reached only from machine code this module emitted, which
        // forwards the `*mut BodyCtx` `run_body_jit` handed it and never stores it.
        unsafe { &mut *$p }
    };
}

// ── shims for the ops whose semantics are a loop or a table ──────────────────
//
// Each calls the SAME function the VM's arm calls (`native_eval::op_*`), so these ops
// have exactly one implementation in the engine, not a second one in cranelift IR. That
// is deliberate: `Select` alone was blocking 281 of the refusals, and its rule is a
// per-bit loop with out-of-range and X-offset cases — precisely the kind of thing a
// re-expression gets subtly wrong and no test notices.

extern "C" fn s_op_select(
    av: u64,
    au: u64,
    bv: u64,
    bu: u64,
    k: u32,
    sel_w: u32,
    src_w: u32,
) -> Pair {
    let kind = match k {
        0 => sim_ir::SelKind::Bit,
        1 => sim_ir::SelKind::PartConst,
        2 => sim_ir::SelKind::PartIdxUp,
        _ => sim_ir::SelKind::PartIdxDown,
    };
    let (v, u) = crate::native_eval::op_select(av, au, bv, bu, kind, sel_w, src_w);
    Pair(v, u)
}

extern "C" fn s_op_reduce(av: u64, au: u64, k: u32, neg: u32, opw: u32) -> Pair {
    let kind = match k {
        0 => crate::native_eval::RedK::And,
        1 => crate::native_eval::RedK::Or,
        _ => crate::native_eval::RedK::Xor,
    };
    let (v, u) = crate::native_eval::op_reduce(av, au, kind, neg != 0, opw);
    Pair(v, u)
}

extern "C" fn s_op_ternary(
    cv: u64,
    cu: u64,
    tv: u64,
    tu: u64,
    ev: u64,
    eu: u64,
    w: u32,
    cw: u32,
) -> Pair {
    let (v, u) = crate::native_eval::op_ternary(cv, cu, tv, tu, ev, eu, w, cw);
    Pair(v, u)
}

extern "C" fn s_op_load_indexed(
    p: *mut BodyCtx,
    net: u32,
    iv: u64,
    iu: u64,
    w: u32,
    sg: u32,
) -> Pair {
    let c = ctx!(p);
    let v =
        c.k.k_nets()
            .read_net(net, Some(crate::native_eval::word_index(iv, iu)))
            .resize_keep_sign(w, sg != 0);
    let m = crate::value::low_mask(w);
    Pair(
        v.val.first().copied().unwrap_or(0) & m,
        v.unk.first().copied().unwrap_or(0) & m,
    )
}

/// One JIT module, kept alive for the process: the machine code it owns is referenced by
/// raw function pointers, so it must never be dropped while a compiled body is live.
pub(crate) struct JitEngine {
    module: JITModule,
    fbctx: FunctionBuilderContext,
    next: usize,
}

impl JitEngine {
    pub(crate) fn new() -> Option<Self> {
        let mut flags = settings::builder();
        flags.set("opt_level", "speed").ok()?;
        flags.set("enable_verifier", "false").ok()?;
        let isa = cranelift_native::builder()
            .ok()?
            .finish(settings::Flags::new(flags))
            .ok()?;
        let mut b = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
        // Every kernel call a compiled body makes goes through one of these, so the
        // body's SEMANTICS stay in the one place the VM and the interpreter already
        // share — the `Kernel` trait — and only expression evaluation and control flow
        // are new machine code.
        b.symbol("s_load", s_load as *const u8);
        b.symbol("s_write_scalar", s_write_scalar as *const u8);
        b.symbol("s_nba_scalar", s_nba_scalar as *const u8);
        b.symbol("s_nba", s_nba as *const u8);
        b.symbol("s_resolve_off", s_resolve_off as *const u8);
        b.symbol("s_write_lval", s_write_lval as *const u8);
        b.symbol("s_eval_for_lval", s_eval_for_lval as *const u8);
        b.symbol("s_write_lval_pending", s_write_lval_pending as *const u8);
        b.symbol(
            "s_write_scalar_pending",
            s_write_scalar_pending as *const u8,
        );
        b.symbol("s_nba_pending", s_nba_pending as *const u8);
        b.symbol("s_systask", s_systask as *const u8);
        b.symbol("s_truthy", s_truthy as *const u8);
        b.symbol("s_truthy_expr", s_truthy_expr as *const u8);
        b.symbol("s_rearm", s_rearm as *const u8);
        b.symbol("s_max_deltas", s_max_deltas as *const u8);
        b.symbol("s_mark_fatal", s_mark_fatal as *const u8);
        b.symbol("s_op_select", s_op_select as *const u8);
        b.symbol("s_op_reduce", s_op_reduce as *const u8);
        b.symbol("s_op_ternary", s_op_ternary as *const u8);
        b.symbol("s_op_load_indexed", s_op_load_indexed as *const u8);
        Some(JitEngine {
            module: JITModule::new(b),
            fbctx: FunctionBuilderContext::new(),
            next: 0,
        })
    }
}

/// The expression ops this module reproduces. Everything else refuses the whole BODY,
/// which then runs on the VM — the reference — exactly as before.
///
/// Chosen from the measured execution histogram. It is deliberately small: each entry is
/// a 4-state rule (X propagation, tri-valued truthiness) rewritten in cranelift IR, and
/// every such rewrite is a chance to disagree silently with the two implementations that
/// already exist. Widening it is measured work, not a formality — on picorv32 the ops
/// still blocking compilation are, by frequency: Select 281, LogBin 259, ConcatPair 70,
/// Ternary 37, Arith 33, Bitwise 27, Reduce 19, Not 8, Shl 5, LoadIndexed 4, Cmp 3.
fn supported(op: &NOp) -> bool {
    matches!(
        op,
        NOp::Const { .. }
            | NOp::LoadScalar { .. }
            | NOp::LoadIndexed { .. }
            | NOp::EqNe { .. }
            | NOp::CaseEqNe { .. }
            | NOp::LogNot { .. }
            | NOp::LogBin { .. }
            | NOp::Bitwise { .. }
            | NOp::Not { .. }
            | NOp::Neg { .. }
            | NOp::Arith { .. }
            | NOp::Cmp { .. }
            | NOp::ConcatPair { .. }
            | NOp::Select { .. }
            | NOp::Reduce { .. }
            | NOp::Ternary { .. }
    )
}

pub(crate) static BODY_OK: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
pub(crate) static BODY_NO: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
pub(crate) static BODY_RUNS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Coverage, not timing: a compiled-body experiment is only meaningful in proportion to
/// the activations it actually claims.
pub(crate) fn jit_stats() {
    use std::sync::atomic::Ordering::Relaxed;
    eprintln!(
        "JITBODY templates_compiled={} refused={} activations={}",
        BODY_OK.load(Relaxed),
        BODY_NO.load(Relaxed),
        BODY_RUNS.load(Relaxed)
    );
}

/// `!a & !b` and `a & !b`, each spelled once. Cranelift's builder takes `&mut
/// self`, so the nested form these rules read most naturally in (`and(x, not(y))`) does
/// not typecheck; these keep the emitters looking like the Rust they mirror.
fn nand2(fb: &mut FunctionBuilder, a: CV, b: CV) -> CV {
    let na = not64(fb, a);
    let nb = not64(fb, b);
    fb.ins().band(na, nb)
}
fn andn(fb: &mut FunctionBuilder, a: CV, b: CV) -> CV {
    let nb = not64(fb, b);
    fb.ins().band(a, nb)
}

/// Tri-valued truthiness of one operand: `(definitely_true, unknown)` as 0/1 words.
/// `definitely_false` is `!t & !u`, which the caller forms.
fn tri(fb: &mut FunctionBuilder, v: CV, u: CV, m: u64) -> (CV, CV) {
    let t1 = andn(fb, v, u);
    let t = nz(fb, t1, m);
    let raw_u = nz(fb, u, m);
    // `tri` checks definite-1 FIRST, so a truthy operand is never also "unknown".
    let unk = andn(fb, raw_u, t);
    let one = fb.ins().iconst(types::I64, 1);
    let tt = fb.ins().band(t, one);
    let uu = fb.ins().band(unk, one);
    (tt, uu)
}

fn sel_kind_code(k: sim_ir::SelKind) -> u32 {
    match k {
        sim_ir::SelKind::Bit => 0,
        sim_ir::SelKind::PartConst => 1,
        sim_ir::SelKind::PartIdxUp => 2,
        sim_ir::SelKind::PartIdxDown => 3,
    }
}

fn red_kind_code(k: crate::native_eval::RedK) -> u32 {
    match k {
        crate::native_eval::RedK::And => 0,
        crate::native_eval::RedK::Or => 1,
        crate::native_eval::RedK::Xor => 2,
    }
}

/// The `FuncRef`s an emitted expression may need. Bundled so `emit` keeps one extra
/// parameter rather than five.
#[derive(Clone, Copy)]
pub(crate) struct Shims {
    pub load: cranelift_codegen::ir::FuncRef,
    pub select: cranelift_codegen::ir::FuncRef,
    pub reduce: cranelift_codegen::ir::FuncRef,
    pub ternary: cranelift_codegen::ir::FuncRef,
    pub load_indexed: cranelift_codegen::ir::FuncRef,
}

/// `(x & m) != 0` as a 0/1 i64 — the shape every 4-state rule below is written from.
fn nz(fb: &mut FunctionBuilder, x: CV, m: u64) -> CV {
    let mv = fb.ins().iconst(types::I64, m as i64);
    let a = fb.ins().band(x, mv);
    let z = fb.ins().iconst(types::I64, 0);
    let c = fb
        .ins()
        .icmp(cranelift_codegen::ir::condcodes::IntCC::NotEqual, a, z);
    fb.ins().uextend(types::I64, c)
}

fn not64(fb: &mut FunctionBuilder, x: CV) -> CV {
    fb.ins().bnot(x)
}

/// Emit one op. Returns false when this module does not model it — the caller then
/// abandons the whole program to the VM.
#[allow(clippy::too_many_arguments)]
fn emit(fb: &mut FunctionBuilder, st: &mut Vec<(CV, CV)>, op: &NOp, ctx_v: CV, sh: Shims) {
    match *op {
        NOp::Const { val, unk } => {
            let v = fb.ins().iconst(types::I64, val as i64);
            let u = fb.ins().iconst(types::I64, unk as i64);
            st.push((v, u));
        }
        NOp::LoadScalar { net, w, signed } => {
            let n = fb.ins().iconst(types::I32, net as i64);
            let wv = fb.ins().iconst(types::I32, w as i64);
            let sv = fb.ins().iconst(types::I32, signed as i64);
            let c = fb.ins().call(sh.load, &[ctx_v, n, wv, sv]);
            let r = fb.inst_results(c);
            st.push((r[0], r[1]));
        }
        // §11.4.5 equality: a both-known differing bit decides; X only when the compare
        // is AMBIGUOUS. Byte-for-byte the `NOp::EqNe` arm in `exec_vm`, written
        // branchlessly with selects.
        NOp::EqNe { ne, w } => {
            let Some((bv, bu)) = st.pop() else { return };
            let Some((av, au)) = st.pop() else { return };
            let m = low_mask(w);
            let mv = fb.ins().iconst(types::I64, m as i64);
            let uor = fb.ins().bor(au, bu);
            let u = fb.ins().band(uor, mv);
            let x = fb.ins().bxor(av, bv);
            let nu = not64(fb, u);
            let d1 = fb.ins().band(x, nu);
            let differ = nz(fb, d1, m);
            let unknown = nz(fb, u, u64::MAX);
            let one = fb.ins().iconst(types::I64, 1);
            let zero = fb.ins().iconst(types::I64, 0);
            let ne_v = fb.ins().iconst(types::I64, ne as i64);
            let eq_v = fb.ins().iconst(types::I64, (!ne) as i64);
            // differ ? ne : (unknown ? (0, 1) : (!ne, 0))
            let amb_v = fb.ins().select(unknown, zero, eq_v);
            let amb_u = fb.ins().select(unknown, one, zero);
            let rv = fb.ins().select(differ, ne_v, amb_v);
            let ru = fb.ins().select(differ, zero, amb_u);
            st.push((rv, ru));
        }
        // `===`/`!==`: a bit-for-bit compare of BOTH planes, never X.
        NOp::CaseEqNe { ne, w } => {
            let Some((bv, bu)) = st.pop() else { return };
            let Some((av, au)) = st.pop() else { return };
            let m = low_mask(w);
            let mv = fb.ins().iconst(types::I64, m as i64);
            let a1 = fb.ins().band(av, mv);
            let b1 = fb.ins().band(bv, mv);
            let a2 = fb.ins().band(au, mv);
            let b2 = fb.ins().band(bu, mv);
            use cranelift_codegen::ir::condcodes::IntCC;
            let e1 = fb.ins().icmp(IntCC::Equal, a1, b1);
            let e2 = fb.ins().icmp(IntCC::Equal, a2, b2);
            let e = fb.ins().band(e1, e2);
            let e64 = fb.ins().uextend(types::I64, e);
            let nev = fb.ins().iconst(types::I64, ne as i64);
            let rv = fb.ins().bxor(e64, nev);
            let zero = fb.ins().iconst(types::I64, 0);
            st.push((rv, zero));
        }
        // truthy → 0, unknown → X, falsy → 1.
        NOp::LogNot { opw } => {
            let Some((av, au)) = st.pop() else { return };
            let m = low_mask(opw);
            let nau = not64(fb, au);
            let t1 = fb.ins().band(av, nau);
            let truthy = nz(fb, t1, m);
            let unknown = nz(fb, au, m);
            let one = fb.ins().iconst(types::I64, 1);
            let zero = fb.ins().iconst(types::I64, 0);
            let fv = fb.ins().select(unknown, zero, one); // not truthy: X ? 0 : 1
            let fu = fb.ins().select(unknown, one, zero);
            let rv = fb.ins().select(truthy, zero, fv);
            let ru = fb.ins().select(truthy, zero, fu);
            st.push((rv, ru));
        }

        // ── inlined: a handful of branchless bit operations each ──
        //
        // Every rule below is the VM arm rewritten with selects instead of branches. That
        // IS a second expression of the semantics, which is why the set is small and why
        // the whole suite runs under `VITA_JIT=1` against the VM as oracle.
        NOp::Bitwise { kind, w } => {
            let Some((bv, bu)) = st.pop() else { return };
            let Some((av, au)) = st.pop() else { return };
            let m = low_mask(w);
            let (rv0, ru0) = match kind {
                crate::native_eval::BitKind::And => {
                    // known0 = (!au & !av) | (!bu & !bv); known1 = (!au & av) & (!bu & bv)
                    let a0 = nand2(fb, au, av);
                    let b0 = nand2(fb, bu, bv);
                    let k0 = fb.ins().bor(a0, b0);
                    let a1 = andn(fb, av, au);
                    let b1 = andn(fb, bv, bu);
                    let k1 = fb.ins().band(a1, b1);
                    let u = nand2(fb, k0, k1);
                    (k1, u)
                }
                crate::native_eval::BitKind::Or => {
                    let a1 = andn(fb, av, au);
                    let b1 = andn(fb, bv, bu);
                    let k1 = fb.ins().bor(a1, b1);
                    let a0 = nand2(fb, au, av);
                    let b0 = nand2(fb, bu, bv);
                    let k0 = fb.ins().band(a0, b0);
                    let u = nand2(fb, k1, k0);
                    (k1, u)
                }
                crate::native_eval::BitKind::Xor => {
                    let ru = fb.ins().bor(au, bu);
                    let x = fb.ins().bxor(av, bv);
                    let v = andn(fb, x, ru);
                    (v, ru)
                }
                crate::native_eval::BitKind::Xnor => {
                    let ru = fb.ins().bor(au, bu);
                    let x0 = fb.ins().bxor(av, bv);
                    let x = not64(fb, x0);
                    let v = andn(fb, x, ru);
                    (v, ru)
                }
            };
            let mv = fb.ins().iconst(types::I64, m as i64);
            let rv = fb.ins().band(rv0, mv);
            let ru = fb.ins().band(ru0, mv);
            st.push((rv, ru));
        }
        NOp::Not { w } => {
            let Some((av, au)) = st.pop() else { return };
            let m = low_mask(w);
            let mv = fb.ins().iconst(types::I64, m as i64);
            // not_w: (!av & !au, au)
            let rv0 = nand2(fb, av, au);
            let rv = fb.ins().band(rv0, mv);
            let ru = fb.ins().band(au, mv);
            st.push((rv, ru));
        }
        // any X/Z in either operand poisons the whole result to X = (0, m)
        NOp::Arith { kind, w } => {
            let Some((bv, bu)) = st.pop() else { return };
            let Some((av, au)) = st.pop() else { return };
            let m = low_mask(w);
            let mv = fb.ins().iconst(types::I64, m as i64);
            let uor = fb.ins().bor(au, bu);
            let bad = nz(fb, uor, m);
            let raw = match kind {
                crate::native_eval::ArithKind::Add => fb.ins().iadd(av, bv),
                crate::native_eval::ArithKind::Sub => fb.ins().isub(av, bv),
                crate::native_eval::ArithKind::Mul => fb.ins().imul(av, bv),
            };
            let good = fb.ins().band(raw, mv);
            let zero = fb.ins().iconst(types::I64, 0);
            let rv = fb.ins().select(bad, zero, good);
            let ru = fb.ins().select(bad, mv, zero);
            st.push((rv, ru));
        }
        NOp::Neg { w } => {
            let Some((av, au)) = st.pop() else { return };
            let m = low_mask(w);
            let mv = fb.ins().iconst(types::I64, m as i64);
            let bad = nz(fb, au, m);
            let neg = fb.ins().ineg(av);
            let good = fb.ins().band(neg, mv);
            let zero = fb.ins().iconst(types::I64, 0);
            let rv = fb.ins().select(bad, zero, good);
            let ru = fb.ins().select(bad, mv, zero);
            st.push((rv, ru));
        }
        // any X/Z → 1-bit X; else the ordered compare at `w`, signed or not
        NOp::Cmp { kind, w, signed } => {
            let Some((bv, bu)) = st.pop() else { return };
            let Some((av, au)) = st.pop() else { return };
            let m = low_mask(w);
            let mv = fb.ins().iconst(types::I64, m as i64);
            let uor = fb.ins().bor(au, bu);
            let bad = nz(fb, uor, m);
            let a = fb.ins().band(av, mv);
            let b = fb.ins().band(bv, mv);
            // sign-extend from bit w-1 when the compare is signed
            let (a, b) = if signed && w < 64 {
                let sh = fb.ins().iconst(types::I64, (64 - w) as i64);
                let a1 = fb.ins().ishl(a, sh);
                let b1 = fb.ins().ishl(b, sh);
                (fb.ins().sshr(a1, sh), fb.ins().sshr(b1, sh))
            } else {
                (a, b)
            };
            let cc = match (kind, signed) {
                (crate::native_eval::CmpKind::Lt, false) => IntCC::UnsignedLessThan,
                (crate::native_eval::CmpKind::Le, false) => IntCC::UnsignedLessThanOrEqual,
                (crate::native_eval::CmpKind::Gt, false) => IntCC::UnsignedGreaterThan,
                (crate::native_eval::CmpKind::Ge, false) => IntCC::UnsignedGreaterThanOrEqual,
                (crate::native_eval::CmpKind::Lt, true) => IntCC::SignedLessThan,
                (crate::native_eval::CmpKind::Le, true) => IntCC::SignedLessThanOrEqual,
                (crate::native_eval::CmpKind::Gt, true) => IntCC::SignedGreaterThan,
                (crate::native_eval::CmpKind::Ge, true) => IntCC::SignedGreaterThanOrEqual,
            };
            let c = fb.ins().icmp(cc, a, b);
            let c64 = fb.ins().uextend(types::I64, c);
            let zero = fb.ins().iconst(types::I64, 0);
            let one = fb.ins().iconst(types::I64, 1);
            let rv = fb.ins().select(bad, zero, c64);
            let ru = fb.ins().select(bad, one, zero);
            st.push((rv, ru));
        }
        NOp::ConcatPair { lo_w, w } => {
            let Some((lo_v, lo_u)) = st.pop() else { return };
            let Some((hi_v, hi_u)) = st.pop() else { return };
            let m = low_mask(w);
            let mv = fb.ins().iconst(types::I64, m as i64);
            let sh = fb.ins().iconst(types::I64, lo_w as i64);
            let hv = fb.ins().ishl(hi_v, sh);
            let hu = fb.ins().ishl(hi_u, sh);
            let rv0 = fb.ins().bor(hv, lo_v);
            let ru0 = fb.ins().bor(hu, lo_u);
            let rv = fb.ins().band(rv0, mv);
            let ru = fb.ins().band(ru0, mv);
            st.push((rv, ru));
        }
        // tri-valued && / || over two independently-masked operands
        NOp::LogBin { and, lw, rw } => {
            let Some((bv, bu)) = st.pop() else { return };
            let Some((av, au)) = st.pop() else { return };
            let (lt, lu2) = tri(fb, av, au, low_mask(lw));
            let (rt, ru2) = tri(fb, bv, bu, low_mask(rw));
            let lf = nand2(fb, lt, lu2);
            let rf = nand2(fb, rt, ru2);
            let (val, unk) = if and {
                let both = fb.ins().band(lt, rt);
                let anyf = fb.ins().bor(lf, rf);
                let u = nand2(fb, anyf, both);
                (both, u)
            } else {
                let anyt = fb.ins().bor(lt, rt);
                let bothf = fb.ins().band(lf, rf);
                let u = nand2(fb, anyt, bothf);
                (anyt, u)
            };
            let one = fb.ins().iconst(types::I64, 1);
            let v = fb.ins().band(val, one);
            let u = fb.ins().band(unk, one);
            st.push((v, u));
        }
        // ── shimmed: the loop/table rules, run by the VM's own function ──
        NOp::Select { kind, sel_w, src_w } => {
            let Some((ov, ou)) = st.pop() else { return };
            let Some((sv, su)) = st.pop() else { return };
            let k = fb.ins().iconst(types::I32, sel_kind_code(kind) as i64);
            let a = fb.ins().iconst(types::I32, sel_w as i64);
            let b = fb.ins().iconst(types::I32, src_w as i64);
            let c = fb.ins().call(sh.select, &[sv, su, ov, ou, k, a, b]);
            let r = fb.inst_results(c);
            st.push((r[0], r[1]));
        }
        NOp::Reduce { kind, neg, opw } => {
            let Some((av, au)) = st.pop() else { return };
            let k = fb.ins().iconst(types::I32, red_kind_code(kind) as i64);
            let n = fb.ins().iconst(types::I32, neg as i64);
            let w = fb.ins().iconst(types::I32, opw as i64);
            let c = fb.ins().call(sh.reduce, &[av, au, k, n, w]);
            let r = fb.inst_results(c);
            st.push((r[0], r[1]));
        }
        NOp::Ternary { w, cond_w } => {
            let Some((ev, eu)) = st.pop() else { return };
            let Some((tv, tu)) = st.pop() else { return };
            let Some((cv, cu)) = st.pop() else { return };
            let a = fb.ins().iconst(types::I32, w as i64);
            let b = fb.ins().iconst(types::I32, cond_w as i64);
            let c = fb.ins().call(sh.ternary, &[cv, cu, tv, tu, ev, eu, a, b]);
            let r = fb.inst_results(c);
            st.push((r[0], r[1]));
        }
        NOp::LoadIndexed { net, w, signed } => {
            let Some((iv, iu)) = st.pop() else { return };
            let n = fb.ins().iconst(types::I32, net as i64);
            let a = fb.ins().iconst(types::I32, w as i64);
            let b = fb.ins().iconst(types::I32, signed as i64);
            let c = fb.ins().call(sh.load_indexed, &[ctx_v, n, iv, iu, a, b]);
            let r = fb.inst_results(c);
            st.push((r[0], r[1]));
        }
        _ => unreachable!("`supported` admitted an op `emit` does not model"),
    }
}

// ── PHASE 2: BODY-LEVEL CODEGEN ──────────────────────────────────────────────
//
// Phase 1 compiled one EXPRESSION per function and lost, by 15%. The reason was
// arithmetic, not codegen quality: the FFI boundary costs ~33 ns (measured with a
// callback-free `Const`-only program — 1,228,796 runs, +32.6 ns each, for machine code
// whose whole body is "return two constants"), and `eval_native` is called 6,509,189
// times. 6.5M x 33 ns = 215 ms of pure boundary against a 130 ms target. It could not
// have won.
//
// `run_body` is called 542,883 times — 12x fewer — against a ~300 ms body region. Same
// boundary, 18 ms instead of 215. That is the unit a compiled simulator actually uses:
// VCS, Xcelium and Verilator compile a process body, not an expression.
//
// And the body is where the removable work is. Per activation today: 13.3 op dispatches,
// 12 `eval_native` calls each CONSTRUCTING a ~64-byte `Value` that the very next op
// consumes, and a `Vec<Option<Value>>` register file. Compiled as one function, the
// expression values stay in registers and none of that exists.
//
// What makes this tractable here and not in general: `is_codegen_able` already admits
// only SUSPEND-FREE bodies, so the hardest part of a compiled simulator — parking and
// resuming a body mid-execution — is out of scope by construction.

use crate::backend::{CompiledBody, CompiledTerm, Op};
use crate::exec::{Kernel, Offsets, Step};
use crate::value::Value;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::Block;

/// Everything a compiled body reaches. The machine code only ever passes the thin
/// pointer through to a shim; every field is touched from Rust.
pub(crate) struct BodyCtx<'a, 'b, 'c> {
    pub k: &'a mut dyn Kernel,
    pub body: &'b CompiledBody,
    pub proc: u32,
    /// `Op::ResolveOff`'s result, waiting for the `Op::WriteLval` that consumes it.
    /// `noffs` is 1, so one slot is the whole file.
    pub off: Option<Offsets>,
    /// `Op::EvalForLval`'s result — the RHS shapes native-eval cannot compile still
    /// produce a `Value`, and it crosses from one shim to the next here rather than
    /// through the machine code.
    pub pending: Option<Value>,
    pub _m: std::marker::PhantomData<&'c ()>,
}

/// Step codes the compiled body returns. `Suspended` cannot occur: the P9 allow-list
/// admits only suspend-free bodies.
const STEP_DONE: i32 = 0;
const STEP_FINISH: i32 = 1;
const STEP_STOP: i32 = 2;
const STEP_FATAL: i32 = 3;

pub(crate) type BodyFn = extern "C" fn(*mut BodyCtx) -> i32;

extern "C" fn s_load(p: *mut BodyCtx, net: u32, w: u32, signed: u32) -> Pair {
    let c = ctx!(p);
    let (v, u) = crate::native_eval::load_scalar(c.k.k_nets(), net, w, signed != 0);
    Pair(v, u)
}

/// Rebuild the `Value` a native program would have produced. Width and signedness are
/// COMPILE-TIME constants (`NativeProg::root_*`), so they ride the instruction stream
/// instead of a struct.
fn mk(val: u64, unk: u64, w: u32, signed: u32) -> Value {
    let mut v = Value::zeros(w, signed != 0);
    v.val[0] = val;
    v.unk[0] = unk;
    v
}

extern "C" fn s_write_scalar(
    p: *mut BodyCtx,
    lhs: u32,
    net: u32,
    val: u64,
    unk: u64,
    w: u32,
    sg: u32,
) {
    let c = ctx!(p);
    let lv = c.body.lvalue(lhs);
    c.k.k_write_scalar(lv, net, mk(val, unk, w, sg));
}

extern "C" fn s_nba_scalar(p: *mut BodyCtx, lhs: u32, val: u64, unk: u64, w: u32, sg: u32) {
    let c = ctx!(p);
    let lv = c.body.lvalue(lhs);
    c.k.k_schedule_nba_scalar(lv, mk(val, unk, w, sg));
}

extern "C" fn s_nba(p: *mut BodyCtx, lhs: u32, val: u64, unk: u64, w: u32, sg: u32) {
    let c = ctx!(p);
    let lv = c.body.lvalue(lhs);
    c.k.k_schedule_nba(lv, mk(val, unk, w, sg));
}

extern "C" fn s_resolve_off(p: *mut BodyCtx, lhs: u32) {
    let c = ctx!(p);
    let lv = c.body.lvalue(lhs);
    c.off = Some(c.k.k_resolve_lvalue_offsets(lv));
}

extern "C" fn s_write_lval(p: *mut BodyCtx, lhs: u32, val: u64, unk: u64, w: u32, sg: u32) {
    let c = ctx!(p);
    let o = c.off.take().expect("WriteLval before ResolveOff");
    let lv = c.body.lvalue(lhs);
    c.k.k_write_lvalue(lv, mk(val, unk, w, sg), &o);
}

/// `Op::EvalForLval` — an RHS native-eval refused. The `Value` it produces is not
/// expressible in registers, so it waits in the context for its consumer.
extern "C" fn s_eval_for_lval(p: *mut BodyCtx, lhs: u32, rhs: u32) {
    let c = ctx!(p);
    let lv = c.body.lvalue(lhs);
    c.pending = Some(c.k.k_eval_for_lvalue(lv, rhs));
}

extern "C" fn s_write_lval_pending(p: *mut BodyCtx, lhs: u32) {
    let c = ctx!(p);
    let v = c.pending.take().expect("WriteLval before EvalForLval");
    let o = c.off.take().expect("WriteLval before ResolveOff");
    let lv = c.body.lvalue(lhs);
    c.k.k_write_lvalue(lv, v, &o);
}

/// `Op::WriteScalar` whose RHS native-eval refused. It has NO `ResolveOff` — the
/// compile-time specialisation removed it, the offsets being the known constant — so it
/// must NOT consume `off`. Routing it through `s_write_lval_pending` panicked with
/// "WriteLval before ResolveOff"; the JIT-on suite caught it.
extern "C" fn s_write_scalar_pending(p: *mut BodyCtx, lhs: u32, net: u32) {
    let c = ctx!(p);
    let v = c.pending.take().expect("WriteScalar before EvalForLval");
    let lv = c.body.lvalue(lhs);
    c.k.k_write_scalar(lv, net, v);
}

extern "C" fn s_nba_pending(p: *mut BodyCtx, lhs: u32) {
    let c = ctx!(p);
    let v = c.pending.take().expect("ScheduleNba before EvalForLval");
    let lv = c.body.lvalue(lhs);
    c.k.k_schedule_nba(lv, v);
}

/// Identified by POSITION, so the op's own fields (a `SysTaskId`, an `Option<u32>`) never
/// have to be encoded into the instruction stream.
extern "C" fn s_systask(p: *mut BodyCtx, blk: u32, opi: u32) -> i32 {
    let c = ctx!(p);
    let Op::SysTask {
        which,
        fmt,
        args,
        sid,
    } = c.body.op_at(blk, opi)
    else {
        unreachable!("s_systask reached a non-SysTask op")
    };
    let a = c.body.arglist(args).to_vec();
    match c.k.k_dispatch_systask(which, fmt, &a, sid) {
        crate::builtins::Ctl::Continue => STEP_DONE,
        crate::builtins::Ctl::Finish => STEP_FINISH,
        crate::builtins::Ctl::Stop => STEP_STOP,
        crate::builtins::Ctl::Fatal => STEP_FATAL,
    }
}

/// The branch condition's tri-valued rule — the SAME `k_truthy_value` the VM uses, never
/// a second copy of it.
extern "C" fn s_truthy(p: *mut BodyCtx, val: u64, unk: u64, w: u32, sg: u32) -> i32 {
    let c = ctx!(p);
    c.k.k_truthy_value(&mk(val, unk, w, sg)) as i32
}

extern "C" fn s_truthy_expr(p: *mut BodyCtx, cond: u32) -> i32 {
    let c = ctx!(p);
    c.k.k_truthy(cond) as i32
}

extern "C" fn s_rearm(p: *mut BodyCtx) {
    let c = ctx!(p);
    let proc = c.proc;
    c.k.k_rearm(proc);
}

extern "C" fn s_max_deltas(p: *mut BodyCtx) -> u64 {
    let c = ctx!(p);
    c.k.k_max_deltas()
}

extern "C" fn s_mark_fatal(p: *mut BodyCtx) {
    let c = ctx!(p);
    c.k.k_mark_fatal();
}

/// Can this whole body be compiled? Positive allow-list: one unsupported op refuses the
/// BODY, which then runs on the VM exactly as before. Refusal is never a diagnostic.
fn body_supported(body: &CompiledBody) -> bool {
    body.blocks().iter().all(|b| {
        let ops_ok = b.ops().iter().all(|op| match op {
            // A native RHS is inlined only when its own ops are ones `emit` models;
            // otherwise the whole body goes back to the VM rather than mixing paths.
            Op::EvalNative { native, .. } => {
                let p = body.native(*native);
                !p.needs_wide() && p.root_width() <= 64 && p.ops().iter().all(supported)
            }
            Op::EvalForLval { .. }
            | Op::ResolveOff { .. }
            | Op::WriteLval { .. }
            | Op::WriteScalar { .. }
            | Op::ScheduleNba { .. }
            | Op::ScheduleNbaScalar { .. }
            | Op::SysTask { .. } => true,
        });
        // A branch condition is ALSO a native program, emitted inline exactly like an
        // RHS, so it needs the same test. Checking only the ops let a body through whose
        // CONDITION used an op `emit` does not model, and the mismatch surfaced as an
        // `unreachable!` mid-codegen instead of a refusal.
        let term_ok = match b.term() {
            CompiledTerm::Goto(_) | CompiledTerm::Return => true,
            CompiledTerm::Branch { native: None, .. } => true,
            CompiledTerm::Branch {
                native: Some(ni), ..
            } => {
                let p = body.native(ni);
                !p.needs_wide() && p.root_width() <= 64 && p.ops().iter().all(supported)
            }
        };
        ops_ok && term_ok
    })
}

impl JitEngine {
    /// Compile a whole `CompiledBody` into one machine-code function.
    ///
    /// The expression values never become `Value`s: a native RHS is emitted inline and
    /// its `(val, unk)` pair goes straight into the shim that consumes it, carrying the
    /// width and signedness as immediates. That is the work Phase 1 could not reach —
    /// there the pair had to be packed into a `Value` and returned across the boundary
    /// on every one of 6.5 million evaluations.
    pub(crate) fn compile_body(&mut self, body: &CompiledBody) -> Option<BodyFn> {
        use std::sync::atomic::Ordering::Relaxed;
        if !body_supported(body) {
            BODY_NO.fetch_add(1, Relaxed);
            return None;
        }
        BODY_OK.fetch_add(1, Relaxed);
        let ptr = self.module.target_config().pointer_type();
        let i64t = types::I64;
        let i32t = types::I32;

        // shim signatures, declared once per compiled body
        let decl = |m: &mut JITModule, name: &str, params: &[types::Type], rets: &[types::Type]| {
            let mut sg = m.make_signature();
            for t in params {
                sg.params.push(AbiParam::new(*t));
            }
            for t in rets {
                sg.returns.push(AbiParam::new(*t));
            }
            m.declare_function(name, Linkage::Import, &sg).ok()
        };
        let f_load = decl(
            &mut self.module,
            "s_load",
            &[ptr, i32t, i32t, i32t],
            &[i64t, i64t],
        )?;
        let f_ws = decl(
            &mut self.module,
            "s_write_scalar",
            &[ptr, i32t, i32t, i64t, i64t, i32t, i32t],
            &[],
        )?;
        let f_ns = decl(
            &mut self.module,
            "s_nba_scalar",
            &[ptr, i32t, i64t, i64t, i32t, i32t],
            &[],
        )?;
        let f_nb = decl(
            &mut self.module,
            "s_nba",
            &[ptr, i32t, i64t, i64t, i32t, i32t],
            &[],
        )?;
        let f_ro = decl(&mut self.module, "s_resolve_off", &[ptr, i32t], &[])?;
        let f_wl = decl(
            &mut self.module,
            "s_write_lval",
            &[ptr, i32t, i64t, i64t, i32t, i32t],
            &[],
        )?;
        let f_ev = decl(&mut self.module, "s_eval_for_lval", &[ptr, i32t, i32t], &[])?;
        let f_wp = decl(&mut self.module, "s_write_lval_pending", &[ptr, i32t], &[])?;
        let f_wsp = decl(
            &mut self.module,
            "s_write_scalar_pending",
            &[ptr, i32t, i32t],
            &[],
        )?;
        let f_np = decl(&mut self.module, "s_nba_pending", &[ptr, i32t], &[])?;
        let f_st = decl(&mut self.module, "s_systask", &[ptr, i32t, i32t], &[i32t])?;
        let f_tv = decl(
            &mut self.module,
            "s_truthy",
            &[ptr, i64t, i64t, i32t, i32t],
            &[i32t],
        )?;
        let f_te = decl(&mut self.module, "s_truthy_expr", &[ptr, i32t], &[i32t])?;
        let f_ra = decl(&mut self.module, "s_rearm", &[ptr], &[])?;
        let f_md = decl(&mut self.module, "s_max_deltas", &[ptr], &[i64t])?;
        let f_mf = decl(&mut self.module, "s_mark_fatal", &[ptr], &[])?;
        let f_sel = decl(
            &mut self.module,
            "s_op_select",
            &[i64t, i64t, i64t, i64t, i32t, i32t, i32t],
            &[i64t, i64t],
        )?;
        let f_red = decl(
            &mut self.module,
            "s_op_reduce",
            &[i64t, i64t, i32t, i32t, i32t],
            &[i64t, i64t],
        )?;
        let f_ter = decl(
            &mut self.module,
            "s_op_ternary",
            &[i64t, i64t, i64t, i64t, i64t, i64t, i32t, i32t],
            &[i64t, i64t],
        )?;
        let f_li = decl(
            &mut self.module,
            "s_op_load_indexed",
            &[ptr, i32t, i64t, i64t, i32t, i32t],
            &[i64t, i64t],
        )?;

        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(ptr));
        sig.returns.push(AbiParam::new(i32t));
        self.next += 1;
        let name = format!("vita_body_{}", self.next);
        let id = self
            .module
            .declare_function(&name, Linkage::Export, &sig)
            .ok()?;

        let mut cctx = self.module.make_context();
        cctx.func.signature = sig;
        {
            let mut fb = FunctionBuilder::new(&mut cctx.func, &mut self.fbctx);
            let m = &mut self.module;
            let rload = m.declare_func_in_func(f_load, fb.func);
            let rws = m.declare_func_in_func(f_ws, fb.func);
            let rns = m.declare_func_in_func(f_ns, fb.func);
            let rnb = m.declare_func_in_func(f_nb, fb.func);
            let rro = m.declare_func_in_func(f_ro, fb.func);
            let rwl = m.declare_func_in_func(f_wl, fb.func);
            let rev = m.declare_func_in_func(f_ev, fb.func);
            let rwp = m.declare_func_in_func(f_wp, fb.func);
            let rwsp = m.declare_func_in_func(f_wsp, fb.func);
            let rnp = m.declare_func_in_func(f_np, fb.func);
            let rst = m.declare_func_in_func(f_st, fb.func);
            let rtv = m.declare_func_in_func(f_tv, fb.func);
            let rte = m.declare_func_in_func(f_te, fb.func);
            let rra = m.declare_func_in_func(f_ra, fb.func);
            let rmd = m.declare_func_in_func(f_md, fb.func);
            let rmf = m.declare_func_in_func(f_mf, fb.func);
            let shims = Shims {
                load: rload,
                select: m.declare_func_in_func(f_sel, fb.func),
                reduce: m.declare_func_in_func(f_red, fb.func),
                ternary: m.declare_func_in_func(f_ter, fb.func),
                load_indexed: m.declare_func_in_func(f_li, fb.func),
            };

            let entry = fb.create_block();
            fb.append_block_params_for_function_params(entry);
            let blocks: Vec<Block> = (0..body.blocks().len())
                .map(|_| fb.create_block())
                .collect();
            // The `guard`/`max_deltas` pair `vm_exec` keeps as locals: a body whose blocks
            // form a loop must still hit the delta cap rather than spin forever.
            let guard = cranelift_frontend::Variable::from_u32(0);
            fb.declare_var(guard, i64t);
            let cap = cranelift_frontend::Variable::from_u32(1);
            fb.declare_var(cap, i64t);

            fb.switch_to_block(entry);
            fb.seal_block(entry);
            let ctxv = fb.block_params(entry)[0];
            let zero64 = fb.ins().iconst(i64t, 0);
            fb.def_var(guard, zero64);
            let c = fb.ins().call(rmd, &[ctxv]);
            let capv = fb.inst_results(c)[0];
            fb.def_var(cap, capv);
            fb.ins().jump(blocks[0], &[]);

            let ret = |fb: &mut FunctionBuilder, code: i32| {
                let v = fb.ins().iconst(i32t, code as i64);
                fb.ins().return_(&[v]);
            };

            for (bi, blk) in body.blocks().iter().enumerate() {
                fb.switch_to_block(blocks[bi]);
                for (oi, op) in blk.ops().iter().enumerate() {
                    match *op {
                        Op::EvalNative { .. } => {} // consumed by its writer below
                        Op::EvalForLval { lhs, rhs, .. } => {
                            let a = fb.ins().iconst(i32t, lhs as i64);
                            let b = fb.ins().iconst(i32t, rhs as i64);
                            fb.ins().call(rev, &[ctxv, a, b]);
                        }
                        Op::ResolveOff { lhs, .. } => {
                            let a = fb.ins().iconst(i32t, lhs as i64);
                            fb.ins().call(rro, &[ctxv, a]);
                        }
                        Op::WriteScalar { lhs, net, .. } => {
                            let (v, u, w, sg) =
                                inline_rhs(&mut fb, blk.ops(), oi, body, ctxv, shims);
                            let a = fb.ins().iconst(i32t, lhs as i64);
                            let n = fb.ins().iconst(i32t, net as i64);
                            match (v, u) {
                                (Some(v), Some(u)) => {
                                    let wv = fb.ins().iconst(i32t, w as i64);
                                    let sv = fb.ins().iconst(i32t, sg as i64);
                                    fb.ins().call(rws, &[ctxv, a, n, v, u, wv, sv]);
                                }
                                _ => {
                                    fb.ins().call(rwsp, &[ctxv, a, n]);
                                }
                            }
                        }
                        Op::WriteLval { lhs, .. } => {
                            let (v, u, w, sg) =
                                inline_rhs(&mut fb, blk.ops(), oi, body, ctxv, shims);
                            let a = fb.ins().iconst(i32t, lhs as i64);
                            match (v, u) {
                                (Some(v), Some(u)) => {
                                    let wv = fb.ins().iconst(i32t, w as i64);
                                    let sv = fb.ins().iconst(i32t, sg as i64);
                                    fb.ins().call(rwl, &[ctxv, a, v, u, wv, sv]);
                                }
                                _ => {
                                    fb.ins().call(rwp, &[ctxv, a]);
                                }
                            }
                        }
                        Op::ScheduleNbaScalar { lhs, .. } | Op::ScheduleNba { lhs, .. } => {
                            let scalar = matches!(op, Op::ScheduleNbaScalar { .. });
                            let (v, u, w, sg) =
                                inline_rhs(&mut fb, blk.ops(), oi, body, ctxv, shims);
                            let a = fb.ins().iconst(i32t, lhs as i64);
                            match (v, u) {
                                (Some(v), Some(u)) => {
                                    let wv = fb.ins().iconst(i32t, w as i64);
                                    let sv = fb.ins().iconst(i32t, sg as i64);
                                    fb.ins().call(
                                        if scalar { rns } else { rnb },
                                        &[ctxv, a, v, u, wv, sv],
                                    );
                                }
                                _ => {
                                    fb.ins().call(rnp, &[ctxv, a]);
                                }
                            }
                        }
                        Op::SysTask { .. } => {
                            let b = fb.ins().iconst(i32t, bi as i64);
                            let o = fb.ins().iconst(i32t, oi as i64);
                            let c = fb.ins().call(rst, &[ctxv, b, o]);
                            let code = fb.inst_results(c)[0];
                            let cont = fb.create_block();
                            let stop = fb.create_block();
                            let z = fb.ins().iconst(i32t, STEP_DONE as i64);
                            let eq = fb.ins().icmp(IntCC::Equal, code, z);
                            fb.ins().brif(eq, cont, &[], stop, &[]);
                            fb.switch_to_block(stop);
                            fb.seal_block(stop);
                            fb.ins().return_(&[code]);
                            fb.switch_to_block(cont);
                            fb.seal_block(cont);
                        }
                    }
                }
                // terminator + the delta guard `vm_exec` applies after every block
                match blk.term() {
                    CompiledTerm::Return => {
                        fb.ins().call(rra, &[ctxv]);
                        ret(&mut fb, STEP_DONE);
                    }
                    CompiledTerm::Goto(t) => {
                        let cont = tick_guard(&mut fb, ctxv, guard, cap, rmf, i32t, i64t);
                        fb.switch_to_block(cont);
                        fb.seal_block(cont);
                        fb.ins().jump(blocks[t as usize], &[]);
                    }
                    CompiledTerm::Branch {
                        cond,
                        native,
                        then_bb,
                        else_bb,
                    } => {
                        let t = match native {
                            Some(ni) => {
                                let p = body.native(ni);
                                let mut st: Vec<(CV, CV)> = Vec::new();
                                for o in p.ops() {
                                    emit(&mut fb, &mut st, o, ctxv, shims);
                                }
                                let (v, u) = st[0];
                                let wv = fb.ins().iconst(i32t, p.root_width() as i64);
                                let sv = fb.ins().iconst(i32t, p.root_signed() as i64);
                                let c = fb.ins().call(rtv, &[ctxv, v, u, wv, sv]);
                                fb.inst_results(c)[0]
                            }
                            None => {
                                let cv = fb.ins().iconst(i32t, cond as i64);
                                let c = fb.ins().call(rte, &[ctxv, cv]);
                                fb.inst_results(c)[0]
                            }
                        };
                        let cont = tick_guard(&mut fb, ctxv, guard, cap, rmf, i32t, i64t);
                        fb.switch_to_block(cont);
                        fb.seal_block(cont);
                        fb.ins().brif(
                            t,
                            blocks[then_bb as usize],
                            &[],
                            blocks[else_bb as usize],
                            &[],
                        );
                    }
                }
            }
            for b in &blocks {
                fb.seal_block(*b);
            }
            fb.finalize();
        }
        self.module.define_function(id, &mut cctx).ok()?;
        self.module.clear_context(&mut cctx);
        self.module.finalize_definitions().ok()?;
        let p = self.module.get_finalized_function(id);
        // SAFETY: machine code cranelift just emitted for the signature declared above;
        // `self.module`, its owner, is never dropped.
        Some(unsafe { std::mem::transmute::<*const u8, BodyFn>(p) })
    }
}

/// `guard += 1; if guard > cap { mark_fatal; return Fatal }` — the same cap `vm_exec`
/// applies after each block, so a compiled body with a cyclic block graph still ends.
/// Returns the block execution continues in.
#[allow(clippy::too_many_arguments)]
fn tick_guard(
    fb: &mut FunctionBuilder,
    ctxv: CV,
    guard: cranelift_frontend::Variable,
    cap: cranelift_frontend::Variable,
    rmf: cranelift_codegen::ir::FuncRef,
    i32t: types::Type,
    i64t: types::Type,
) -> Block {
    let g = fb.use_var(guard);
    let one = fb.ins().iconst(i64t, 1);
    let g2 = fb.ins().iadd(g, one);
    fb.def_var(guard, g2);
    let capv = fb.use_var(cap);
    let over = fb.ins().icmp(IntCC::UnsignedGreaterThan, g2, capv);
    let fatal = fb.create_block();
    let cont = fb.create_block();
    fb.ins().brif(over, fatal, &[], cont, &[]);
    fb.switch_to_block(fatal);
    fb.seal_block(fatal);
    fb.ins().call(rmf, &[ctxv]);
    let f = fb.ins().iconst(i32t, STEP_FATAL as i64);
    fb.ins().return_(&[f]);
    cont
}

/// Emit the native RHS that feeds the write at `oi`, inline.
///
/// The eval op sits immediately before its consumer (the compiler emits them as one
/// contiguous group), so the RHS is found by looking back rather than by threading a
/// register file through. `None` means the RHS was an `EvalForLval` whose `Value` is
/// already parked in the context.
fn inline_rhs(
    fb: &mut FunctionBuilder,
    ops: &[Op],
    oi: usize,
    body: &CompiledBody,
    ctxv: CV,
    shims: Shims,
) -> (Option<CV>, Option<CV>, u32, u32) {
    for back in 1..=2usize {
        if oi < back {
            break;
        }
        match ops[oi - back] {
            Op::EvalNative { native, .. } => {
                let p = body.native(native);
                let mut st: Vec<(CV, CV)> = Vec::new();
                for o in p.ops() {
                    emit(fb, &mut st, o, ctxv, shims);
                }
                let (v, u) = st[0];
                return (Some(v), Some(u), p.root_width(), p.root_signed() as u32);
            }
            Op::EvalForLval { .. } => return (None, None, 0, 0),
            _ => {}
        }
    }
    (None, None, 0, 0)
}

/// Run a compiled body. The only place the body-level boundary is crossed — once per
/// activation, against 12 native-eval crossings the compiled form removes.
pub(crate) fn run_body_jit(f: BodyFn, k: &mut dyn Kernel, body: &CompiledBody, proc: u32) -> Step {
    BODY_RUNS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let mut ctx = BodyCtx {
        k,
        body,
        proc,
        off: None,
        pending: None,
        _m: std::marker::PhantomData,
    };
    match f(&mut ctx as *mut BodyCtx) {
        STEP_FINISH => Step::Finish,
        STEP_STOP => Step::Stop,
        STEP_FATAL => Step::Fatal,
        _ => Step::Done,
    }
}
