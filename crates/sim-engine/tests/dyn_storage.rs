//! v5 increment (C)-③/④/⑤: dynamic-array + queue + assoc ENGINE layer.
//!
//! No front-end syntax exists yet (that is increment ⑥, batched with the .vu
//! flip), so these tests HAND-BUILD a frozen `SimIr` and drive it through the
//! public `simulate`/`simulate_capture` seam — exactly what elaborate will emit
//! once the syntax lands. Semantics oracle: iverilog -g2012 probed live
//! (③ `new[5]`→size 5, `delete()`→0, copy form `new[6](d)`→6; ④ push order
//! 5/10/20, pop_back→20/pop_front→5, `q[size]=v` APPENDS (push_back equiv,
//! IEEE §7.10.1 — silent), far-OOB write ignored+warn, empty pop warn+x,
//! signed byte −1 pops sign-extended / unsigned 255 zero-extended).
//! ⑤ assoc has NO iverilog lane (13.0 rejects the declarations) — hand-IEEE
//! pinned, see the ⑤ section header below.

use sim_engine::{simulate, simulate_capture, Backend, FinishReason, SimOpts};
use sim_ir::{BinOp, Expr, Stmt, SysFuncId, SysTaskId};

#[path = "dyn_storage_util/mod.rs"]
mod util;
#[allow(unused_imports)]
use util::*;

#[test]
fn dyn_new_size_delete_roundtrip() {
    // new[5] → size 5; delete() → size 0. (Oracle: iverilog live.)
    let exprs = vec![
        Expr::Signal { net: 0, word: None }, // 0: handle (DynNew)
        Expr::Const { val: 0 },              // 1: 5
        Expr::Signal { net: 0, word: None }, // 2: handle (size #1)
        Expr::SysFunc {
            which: SysFuncId::DynSize,
            args: vec![2],
        }, // 3
        Expr::Signal { net: 0, word: None }, // 4: handle (DynDelete)
        Expr::Signal { net: 0, word: None }, // 5: handle (size #2)
        Expr::SysFunc {
            which: SysFuncId::DynSize,
            args: vec![5],
        }, // 6
    ];
    let stmts = vec![
        systask(SysTaskId::DynNew, vec![0, 1]),
        systask(SysTaskId::Display, vec![3]),
        systask(SysTaskId::DynDelete, vec![4]),
        systask(SysTaskId::Display, vec![6]),
        systask(SysTaskId::Finish, vec![]),
    ];
    let ir = ir_of(vec![dyn_handle()], vec![int_const(5)], exprs, stmts);
    let (res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(out, "          5\n          0\n"); // unformatted arg = default-width decimal
}

#[test]
fn dyn_new_copy_form_sizes_by_n() {
    // d = new[3]; q = new[6](d) → q.size() == 6 (sized by n, NOT by src —
    // oracle: iverilog live). Element-content checks land with the indexed
    // read/write slice.
    let exprs = vec![
        Expr::Signal { net: 0, word: None }, // 0: d
        Expr::Const { val: 0 },              // 1: 3
        Expr::Signal { net: 1, word: None }, // 2: q
        Expr::Const { val: 1 },              // 3: 6
        Expr::Signal { net: 0, word: None }, // 4: src d
        Expr::Signal { net: 1, word: None }, // 5: q (size)
        Expr::SysFunc {
            which: SysFuncId::DynSize,
            args: vec![5],
        }, // 6
    ];
    let stmts = vec![
        systask(SysTaskId::DynNew, vec![0, 1]),
        systask(SysTaskId::DynNew, vec![2, 3, 4]),
        systask(SysTaskId::Display, vec![6]),
        systask(SysTaskId::Finish, vec![]),
    ];
    let ir = ir_of(
        vec![dyn_handle(), dyn_handle()],
        vec![int_const(3), int_const(6)],
        exprs,
        stmts,
    );
    let (res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(out, "          6\n");
}

#[test]
fn dyn_new_x_size_degrades_to_empty_with_one_warn() {
    // new[n] with an X/Z n → EMPTY array + a single W-RUN-DYN-DEGRADE warning
    // (warn-once per net); new[0] is legal-silent (IEEE §7.5.1, design §4).
    let exprs = vec![
        Expr::Signal { net: 0, word: None }, // 0: handle
        Expr::Const { val: 0 },              // 1: X const
        Expr::Signal { net: 0, word: None }, // 2: handle (X again — same net)
        Expr::Const { val: 0 },              // 3
        Expr::Signal { net: 0, word: None }, // 4: handle (size)
        Expr::SysFunc {
            which: SysFuncId::DynSize,
            args: vec![4],
        }, // 5
    ];
    let stmts = vec![
        systask(SysTaskId::DynNew, vec![0, 1]),
        systask(SysTaskId::DynNew, vec![2, 3]),
        systask(SysTaskId::Display, vec![5]),
        systask(SysTaskId::Finish, vec![]),
    ];
    let ir = ir_of(vec![dyn_handle()], vec![x_const()], exprs, stmts);
    let sink = DiagSink::default();
    let res = simulate(&ir, &sink, SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    let diags = sink.0.into_inner();
    let dyn_warns: Vec<&String> = diags.iter().filter(|d| d.contains("W4020")).collect();
    assert_eq!(
        dyn_warns.len(),
        1,
        "X-size new[] must warn EXACTLY once per net: {diags:?}"
    );
}

#[test]
fn dyn_indexed_write_read_roundtrip() {
    // d = new[3]; d[0]=10; d[1]=20; d[2]=30; read all three back.
    // (Oracle: iverilog live — 10 20 30.)
    let exprs = vec![
        Expr::Signal { net: 0, word: None }, // 0: handle (new)
        Expr::Const { val: 0 },              // 1: 3
        Expr::Const { val: 1 },              // 2: idx 0
        Expr::Const { val: 2 },              // 3: 10
        Expr::Const { val: 3 },              // 4: idx 1
        Expr::Const { val: 4 },              // 5: 20
        Expr::Const { val: 5 },              // 6: idx 2
        Expr::Const { val: 6 },              // 7: 30
        Expr::Signal {
            net: 0,
            word: Some(2),
        }, // 8: d[0]
        Expr::Signal {
            net: 0,
            word: Some(4),
        }, // 9: d[1]
        Expr::Signal {
            net: 0,
            word: Some(6),
        }, // 10: d[2]
    ];
    let consts = vec![
        int_const(3),
        int_const(0),
        int_const(10),
        int_const(1),
        int_const(20),
        int_const(2),
        int_const(30),
    ];
    let stmts = vec![
        systask(SysTaskId::DynNew, vec![0, 1]),
        elem_write(0, 2, 3),
        elem_write(0, 4, 5),
        elem_write(0, 6, 7),
        systask(SysTaskId::Display, vec![8]),
        systask(SysTaskId::Display, vec![9]),
        systask(SysTaskId::Display, vec![10]),
        systask(SysTaskId::Finish, vec![]),
    ];
    let ir = ir_of(vec![dyn_handle32()], consts, exprs, stmts);
    let (res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(out, "        10\n        20\n        30\n");
}

#[test]
fn dyn_oob_and_x_index_read_is_x_with_one_warn() {
    // d = new[2]; read d[5] (OOB) and d[X-idx] → BOTH element-width X, ONE
    // W4020 (warn-once per net). 4-state elements default to X — iverilog's
    // 2-state `int` prints 0 there by ITS element-default rule; ours is X by
    // the design-doc rule (hand-IEEE pin, same family as the force precedent).
    let exprs = vec![
        Expr::Signal { net: 0, word: None }, // 0: handle
        Expr::Const { val: 0 },              // 1: 2
        Expr::Const { val: 1 },              // 2: idx 5 (OOB)
        Expr::Signal {
            net: 0,
            word: Some(2),
        }, // 3: d[5]
        Expr::Const { val: 2 },              // 4: X idx
        Expr::Signal {
            net: 0,
            word: Some(4),
        }, // 5: d[X]
    ];
    let consts = vec![int_const(2), int_const(5), x_const()];
    let stmts = vec![
        systask(SysTaskId::DynNew, vec![0, 1]),
        systask(SysTaskId::Display, vec![3]),
        systask(SysTaskId::Display, vec![5]),
        systask(SysTaskId::Finish, vec![]),
    ];
    let ir = ir_of(vec![dyn_handle32()], consts, exprs, stmts);
    let sink = DiagSink::default();
    let res = simulate(&ir, &sink, SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    let diags = sink.0.into_inner();
    let dyn_warns = diags.iter().filter(|d| d.contains("W4020")).count();
    assert_eq!(dyn_warns, 1, "OOB+X reads on ONE net warn once: {diags:?}");
}

#[test]
fn queue_push_index_size_roundtrip() {
    let (res, out) = simulate_capture(&queue_push_ir(), SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(out, "          3\n         5\n        10\n        20\n");
}

#[test]
fn queue_pop_back_front_values_and_size() {
    let (res, out) = simulate_capture(&queue_pop_ir(), SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(out, "        20\n         5\n          1\n");
}

#[test]
fn queue_pop_empty_is_x_with_one_warn() {
    // pop on a never-touched queue → element-width X + ONE W4020 (iverilog
    // warns per call and prints x; our warn-once-per-net is the established
    // anti-spam policy, the VALUE surface x is the oracle pin).
    let exprs = vec![
        Expr::Signal { net: 0, word: None }, // 0: handle (pop)
        Expr::SysFunc {
            which: SysFuncId::QPopBack,
            args: vec![0],
        }, // 1
        Expr::Signal { net: 1, word: None }, // 2: x
        Expr::Signal { net: 0, word: None }, // 3: handle (size)
        Expr::SysFunc {
            which: SysFuncId::DynSize,
            args: vec![3],
        }, // 4
    ];
    let stmts = vec![
        assign(1, 1), // x = q.pop_back() on empty
        systask(SysTaskId::Display, vec![2]),
        systask(SysTaskId::Display, vec![4]),
        systask(SysTaskId::Finish, vec![]),
    ];
    let ir = ir_of(
        vec![q_handle(32, false), reg32(false)],
        vec![],
        exprs,
        stmts,
    );
    let (res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(out, "         x\n          0\n");
    let sink = DiagSink::default();
    let res = simulate(&ir, &sink, SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    let diags = sink.0.into_inner();
    let warns = diags.iter().filter(|d| d.contains("W4020")).count();
    assert_eq!(warns, 1, "empty pop warns exactly once per net: {diags:?}");
}

#[test]
fn queue_last_index_via_size_minus_one() {
    // The ⑥ `q[$]` desugar contract: `q[DynSize(q)-1]` reads the LAST element.
    // (Oracle: iverilog live — q[$] of {5,10,20} is 20; here {10,20} → 20.)
    let exprs = vec![
        Expr::Signal { net: 0, word: None }, // 0: handle (push 10)
        Expr::Const { val: 0 },              // 1: 10
        Expr::Signal { net: 0, word: None }, // 2: handle (push 20)
        Expr::Const { val: 1 },              // 3: 20
        Expr::Signal { net: 0, word: None }, // 4: handle (size)
        Expr::SysFunc {
            which: SysFuncId::DynSize,
            args: vec![4],
        }, // 5
        Expr::Const { val: 2 },              // 6: 1
        Expr::Binary {
            op: BinOp::Sub,
            lhs: 5,
            rhs: 6,
        }, // 7: size-1
        Expr::Signal {
            net: 0,
            word: Some(7),
        }, // 8: q[$]
    ];
    let consts = vec![int_const(10), int_const(20), int_const(1)];
    let stmts = vec![
        systask(SysTaskId::QPushBack, vec![0, 1]),
        systask(SysTaskId::QPushBack, vec![2, 3]),
        systask(SysTaskId::Display, vec![8]),
        systask(SysTaskId::Finish, vec![]),
    ];
    let ir = ir_of(vec![q_handle(32, false)], consts, exprs, stmts);
    let (res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(out, "        20\n");
}

#[test]
fn queue_last_index_on_empty_is_x_with_warn() {
    // `q[$]` of an EMPTY queue: DynSize-1 = -1 → the u32::MAX OOR sentinel →
    // element-width X + warn-once (deterministic rule; read never grows).
    let exprs = vec![
        Expr::Signal { net: 0, word: None }, // 0: handle (size)
        Expr::SysFunc {
            which: SysFuncId::DynSize,
            args: vec![0],
        }, // 1
        Expr::Const { val: 0 },              // 2: 1
        Expr::Binary {
            op: BinOp::Sub,
            lhs: 1,
            rhs: 2,
        }, // 3: size-1 = -1
        Expr::Signal {
            net: 0,
            word: Some(3),
        }, // 4: q[$]
    ];
    let stmts = vec![
        systask(SysTaskId::Display, vec![4]),
        systask(SysTaskId::Finish, vec![]),
    ];
    let ir = ir_of(vec![q_handle(32, false)], vec![int_const(1)], exprs, stmts);
    let (res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(out, "         x\n");
    let sink = DiagSink::default();
    simulate(&ir, &sink, SimOpts::default());
    let diags = sink.0.into_inner();
    let warns = diags.iter().filter(|d| d.contains("W4020")).count();
    assert_eq!(warns, 1, "{diags:?}");
}

#[test]
fn queue_write_at_size_appends_beyond_is_ignored() {
    // IEEE §7.10.1 (iverilog live): a write to EXACTLY q[size] is push_back —
    // legal, SILENT, grows by one. A write beyond that is ignored + warn.
    // (Design-doc §4 originally said "q[size] write ignored" — corrected by
    // the live oracle, same as the new[0] refinement.)
    let exprs = vec![
        Expr::Signal { net: 0, word: None }, // 0: handle (push 1)
        Expr::Const { val: 0 },              // 1: 1
        Expr::Signal { net: 0, word: None }, // 2: handle (push 2)
        Expr::Const { val: 1 },              // 3: 2
        Expr::Const { val: 2 },              // 4: idx 2 (== size → append)
        Expr::Const { val: 3 },              // 5: 33
        Expr::Const { val: 4 },              // 6: idx 9 (far OOB → ignore)
        Expr::Const { val: 5 },              // 7: 99
        Expr::Signal { net: 0, word: None }, // 8: handle (size)
        Expr::SysFunc {
            which: SysFuncId::DynSize,
            args: vec![8],
        }, // 9
        Expr::Signal {
            net: 0,
            word: Some(4),
        }, // 10: q[2]
    ];
    let consts = vec![
        int_const(1),
        int_const(2),
        int_const(2),
        int_const(33),
        int_const(9),
        int_const(99),
    ];
    let stmts = vec![
        systask(SysTaskId::QPushBack, vec![0, 1]),
        systask(SysTaskId::QPushBack, vec![2, 3]),
        elem_write(0, 4, 5), // q[2] = 33 → append (size 3)
        elem_write(0, 6, 7), // q[9] = 99 → ignored + warn
        systask(SysTaskId::Display, vec![9]),
        systask(SysTaskId::Display, vec![10]),
        systask(SysTaskId::Finish, vec![]),
    ];
    let ir = ir_of(vec![q_handle(32, false)], consts, exprs, stmts);
    let (res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(out, "          3\n        33\n");
    let sink = DiagSink::default();
    simulate(&ir, &sink, SimOpts::default());
    let diags = sink.0.into_inner();
    let warns = diags.iter().filter(|d| d.contains("W4020")).count();
    assert_eq!(warns, 1, "append is silent, far OOB warns once: {diags:?}");
}

#[test]
fn queue_delete_empties() {
    let exprs = vec![
        Expr::Signal { net: 0, word: None }, // 0: handle (push 7)
        Expr::Const { val: 0 },              // 1: 7
        Expr::Signal { net: 0, word: None }, // 2: handle (push 8)
        Expr::Const { val: 1 },              // 3: 8
        Expr::Signal { net: 0, word: None }, // 4: handle (size #1)
        Expr::SysFunc {
            which: SysFuncId::DynSize,
            args: vec![4],
        }, // 5
        Expr::Signal { net: 0, word: None }, // 6: handle (delete)
        Expr::Signal { net: 0, word: None }, // 7: handle (size #2)
        Expr::SysFunc {
            which: SysFuncId::DynSize,
            args: vec![7],
        }, // 8
    ];
    let stmts = vec![
        systask(SysTaskId::QPushBack, vec![0, 1]),
        systask(SysTaskId::QPushBack, vec![2, 3]),
        systask(SysTaskId::Display, vec![5]),
        systask(SysTaskId::DynDelete, vec![6]),
        systask(SysTaskId::Display, vec![8]),
        systask(SysTaskId::Finish, vec![]),
    ];
    let ir = ir_of(
        vec![q_handle(32, false)],
        vec![int_const(7), int_const(8)],
        exprs,
        stmts,
    );
    let (res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(out, "          2\n          0\n");
}

#[test]
fn queue_pop_extends_by_element_signedness() {
    // 8-bit SIGNED queue: push −1 (stored 0xFF), push 300 (truncates → 44);
    // pops into a 32-bit dst sign-extend → −1, 44. 8-bit UNSIGNED queue:
    // push 255; pop zero-extends → 255 (NOT −1). Oracle: iverilog live.
    let exprs = vec![
        Expr::Signal { net: 0, word: None }, // 0: sq (push −1)
        Expr::Const { val: 0 },              // 1: −1
        Expr::Signal { net: 0, word: None }, // 2: sq (push 300)
        Expr::Const { val: 1 },              // 3: 300
        Expr::Signal { net: 1, word: None }, // 4: uq (push 255)
        Expr::Const { val: 2 },              // 5: 255
        Expr::Signal { net: 0, word: None }, // 6: sq (pop #1)
        Expr::SysFunc {
            which: SysFuncId::QPopFront,
            args: vec![6],
        }, // 7
        Expr::Signal { net: 0, word: None }, // 8: sq (pop #2)
        Expr::SysFunc {
            which: SysFuncId::QPopFront,
            args: vec![8],
        }, // 9
        Expr::Signal { net: 1, word: None }, // 10: uq (pop)
        Expr::SysFunc {
            which: SysFuncId::QPopBack,
            args: vec![10],
        }, // 11
        Expr::Signal { net: 2, word: None }, // 12: xs
        Expr::Signal { net: 3, word: None }, // 13: xu
    ];
    let consts = vec![int_const(0xffff_ffff), int_const(300), int_const(255)];
    let stmts = vec![
        systask(SysTaskId::QPushBack, vec![0, 1]),
        systask(SysTaskId::QPushBack, vec![2, 3]),
        systask(SysTaskId::QPushBack, vec![4, 5]),
        assign(2, 7), // xs = sq.pop_front() → −1 (sign-extend)
        systask(SysTaskId::Display, vec![12]),
        assign(2, 9), // xs = sq.pop_front() → 44
        systask(SysTaskId::Display, vec![12]),
        assign(3, 11), // xu = uq.pop_back() → 255 (zero-extend)
        systask(SysTaskId::Display, vec![13]),
        systask(SysTaskId::Finish, vec![]),
    ];
    let ir = ir_of(
        vec![
            q_handle(8, true),
            q_handle(8, false),
            reg32(true),
            reg32(false),
        ],
        consts,
        exprs,
        stmts,
    );
    let (res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(out, "         -1\n         44\n       255\n");
}

#[test]
fn queue_nba_pop_rhs_is_x_and_queue_intact() {
    // MVP contract: pop is a STATEMENT-level effect, valid only as the direct
    // RHS of a blocking assign. An NBA rhs pop X-poisons (with the W4020
    // once-latch) and the queue is NOT mutated — identical on both backends
    // by construction (same eval funnel); ⑥ elaborate will loud-reject it.
    let exprs = vec![
        Expr::Signal { net: 0, word: None }, // 0: handle (push 10)
        Expr::Const { val: 0 },              // 1: 10
        Expr::Signal { net: 0, word: None }, // 2: handle (pop)
        Expr::SysFunc {
            which: SysFuncId::QPopBack,
            args: vec![2],
        }, // 3
        Expr::Signal { net: 0, word: None }, // 4: handle (size)
        Expr::SysFunc {
            which: SysFuncId::DynSize,
            args: vec![4],
        }, // 5
    ];
    let stmts = vec![
        systask(SysTaskId::QPushBack, vec![0, 1]),
        Stmt::NonblockingAssign {
            lhs: sim_ir::Lvalue {
                chunks: vec![sim_ir::LvalChunk {
                    net: 1,
                    word: None,
                    offset: None,
                    width: None,
                    kind: sim_ir::SelKind::Bit,
                }],
            },
            rhs: 3,
            delay: None,
        },
        systask(SysTaskId::Display, vec![5]),
        systask(SysTaskId::Finish, vec![]),
    ];
    let ir = ir_of(
        vec![q_handle(32, false), reg32(false)],
        vec![int_const(10)],
        exprs,
        stmts,
    );
    let (res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(
        out, "          1\n",
        "queue must NOT be drained by an NBA pop"
    );
    let sink = DiagSink::default();
    simulate(&ir, &sink, SimOpts::default());
    let diags = sink.0.into_inner();
    let warns = diags.iter().filter(|d| d.contains("W4020")).count();
    assert_eq!(warns, 1, "NBA pop degrades loudly (X): {diags:?}");
}

#[test]
fn queue_vm_backend_byte_parity() {
    // P5-style gate, pre-⑥ form: the SAME hand-built IR must produce
    // byte-identical stdout on Interpreter and Bytecode. The push-only body is
    // codegen-able (SysTask rides the shared kernel dispatch); the pop body is
    // excluded by the P9 allow-list and falls back to the interpreter.
    for ir in [queue_push_ir(), queue_pop_ir()] {
        let (ri, oi) = simulate_capture(&ir, SimOpts::default());
        let (rv, ov) = simulate_capture(
            &ir,
            SimOpts {
                backend: Backend::Bytecode,
                ..SimOpts::default()
            },
        );
        assert_eq!(ri.finish_reason, rv.finish_reason);
        assert_eq!(oi, ov, "interp vs VM stdout must be byte-identical");
    }
}

#[test]
fn dyn_oob_write_is_ignored_with_warn() {
    // d = new[2]; d[0]=7; d[9]=99 (OOB, ignored); size stays 2, d[0] intact.
    let exprs = vec![
        Expr::Signal { net: 0, word: None }, // 0: handle
        Expr::Const { val: 0 },              // 1: 2
        Expr::Const { val: 1 },              // 2: idx 0
        Expr::Const { val: 2 },              // 3: 7
        Expr::Const { val: 3 },              // 4: idx 9 (OOB)
        Expr::Const { val: 4 },              // 5: 99
        Expr::Signal { net: 0, word: None }, // 6: handle (size)
        Expr::SysFunc {
            which: SysFuncId::DynSize,
            args: vec![6],
        }, // 7
        Expr::Signal {
            net: 0,
            word: Some(2),
        }, // 8: d[0]
    ];
    let consts = vec![
        int_const(2),
        int_const(0),
        int_const(7),
        int_const(9),
        int_const(99),
    ];
    let stmts = vec![
        systask(SysTaskId::DynNew, vec![0, 1]),
        elem_write(0, 2, 3),
        elem_write(0, 4, 5),
        systask(SysTaskId::Display, vec![7]),
        systask(SysTaskId::Display, vec![8]),
        systask(SysTaskId::Finish, vec![]),
    ];
    let ir = ir_of(vec![dyn_handle32()], consts, exprs, stmts);
    // (the W4020 once-latch is covered by the read test above; here we pin
    // the VALUE surface: size unchanged + neighbour intact)
    let (res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(
        out, "          2\n         7\n",
        "size unchanged, d[0] intact"
    );
}

#[test]
fn assoc_write_read_num_roundtrip() {
    let (res, out) = simulate_capture(&assoc_rw_ir(), SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(out, "        10\n        20\n          2\n");
}

#[test]
fn assoc_key_beyond_u32_roundtrips() {
    // a[2^40] = 7 → reads 7 back. Proves the key path is i64 end-to-end, not
    // the static-array u32 funnel (whose OOR sentinel would X this).
    let exprs = vec![
        Expr::Const { val: 0 }, // 0: key 2^40
        Expr::Const { val: 1 }, // 1: 7
        Expr::Signal {
            net: 0,
            word: Some(0),
        }, // 2: a[2^40]
    ];
    let consts = vec![long_const(1u64 << 40), int_const(7)];
    let stmts = vec![
        elem_write(0, 0, 1),
        systask(SysTaskId::Display, vec![2]),
        systask(SysTaskId::Finish, vec![]),
    ];
    let ir = ir_of(vec![a_handle(32, false)], consts, exprs, stmts);
    let (res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(out, "         7\n");
}

#[test]
fn assoc_missing_key_read_is_x_with_one_warn() {
    // Read of a never-written key → element-width X + ONE W4020 (§7.8.6;
    // the missing heap entry IS the empty assoc).
    let exprs = vec![
        Expr::Const { val: 0 }, // 0: key 7
        Expr::Signal {
            net: 0,
            word: Some(0),
        }, // 1: a[7]
    ];
    let stmts = vec![
        systask(SysTaskId::Display, vec![1]),
        systask(SysTaskId::Finish, vec![]),
    ];
    let ir = ir_of(vec![a_handle(32, false)], vec![int_const(7)], exprs, stmts);
    let (res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(out, "         x\n");
    let sink = DiagSink::default();
    simulate(&ir, &sink, SimOpts::default());
    let diags = sink.0.into_inner();
    let warns = diags.iter().filter(|d| d.contains("W4020")).count();
    assert_eq!(warns, 1, "missing-key read warns once: {diags:?}");
}

#[test]
fn assoc_x_key_read_x_write_ignored_with_warn() {
    // a[5]=10; a[X]=99 (IGNORED); a[X] reads X; num stays 1; a[5] intact.
    // X/Z key = "invalid index" (§7.8.6): write ignored, read X — both W4020,
    // latched once per net.
    let exprs = vec![
        Expr::Const { val: 0 }, // 0: key 5
        Expr::Const { val: 1 }, // 1: 10
        Expr::Const { val: 2 }, // 2: X key
        Expr::Const { val: 3 }, // 3: 99
        Expr::Signal {
            net: 0,
            word: Some(2),
        }, // 4: a[X]
        Expr::Signal { net: 0, word: None }, // 5: handle (num)
        Expr::SysFunc {
            which: SysFuncId::AssocNum,
            args: vec![5],
        }, // 6
        Expr::Signal {
            net: 0,
            word: Some(0),
        }, // 7: a[5]
    ];
    let consts = vec![int_const(5), int_const(10), x_const(), int_const(99)];
    let stmts = vec![
        elem_write(0, 0, 1),
        elem_write(0, 2, 3),
        systask(SysTaskId::Display, vec![4]),
        systask(SysTaskId::Display, vec![6]),
        systask(SysTaskId::Display, vec![7]),
        systask(SysTaskId::Finish, vec![]),
    ];
    let ir = ir_of(vec![a_handle(32, false)], consts, exprs, stmts);
    let (res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(out, "         x\n          1\n        10\n");
    let sink = DiagSink::default();
    simulate(&ir, &sink, SimOpts::default());
    let diags = sink.0.into_inner();
    let warns = diags.iter().filter(|d| d.contains("W4020")).count();
    assert_eq!(
        warns, 1,
        "X-key lanes share the once-per-net latch: {diags:?}"
    );
}
