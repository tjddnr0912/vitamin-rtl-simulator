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
use sim_ir::{
    BasicBlock, BitPacked, DelayRegion, Expr, Instance, NetKind, NetVar, PortDir, Process,
    SensKind, Sensitivity, SimIr, Stmt, SysFuncId, SysTaskId, Terminator,
};

#[path = "dyn_storage_util/mod.rs"]
mod util;
#[allow(unused_imports)]
use util::*;

#[test]
fn assoc_exists_hit_miss_and_x_key() {
    let (res, out) = simulate_capture(&assoc_exists_ir(), SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(out, "1\n0\n0\n"); // exists is 1-bit (width table)
}

#[test]
fn assoc_delete_key_missing_silent_then_clear() {
    let (res, out) = simulate_capture(&assoc_delete_ir(), SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(out, "          1\n0\n          1\n          0\n");
    let sink = DiagSink::default();
    simulate(&assoc_delete_ir(), &sink, SimOpts::default());
    let diags = sink.0.into_inner();
    let warns = diags.iter().filter(|d| d.contains("W4020")).count();
    assert_eq!(
        warns, 0,
        "delete of a missing key is SILENT (§7.9): {diags:?}"
    );
}

#[test]
fn assoc_element_signedness_extension() {
    // signed byte element −1 reads back sign-extended into an int (−1);
    // unsigned byte 255 stays 255 — the same §5.5 lanes as queue pop.
    let exprs = vec![
        Expr::Const { val: 0 }, // 0: key 0
        Expr::Const { val: 1 }, // 1: −1
        Expr::Const { val: 0 }, // 2: key 0
        Expr::Const { val: 2 }, // 3: 255
        Expr::Signal {
            net: 0,
            word: Some(0),
        }, // 4: a[0] (signed byte)
        Expr::Signal {
            net: 1,
            word: Some(2),
        }, // 5: b[0] (unsigned byte)
        Expr::Signal { net: 2, word: None }, // 6: r
        Expr::Signal { net: 3, word: None }, // 7: s
    ];
    let consts = vec![int_const(0), int_const(0xFFFF_FFFF), int_const(255)];
    let stmts = vec![
        elem_write(0, 0, 1), // a[0] = −1  (stored as 8'hFF)
        elem_write(1, 2, 3), // b[0] = 255 (stored as 8'hFF)
        assign(2, 4),        // r = a[0] → sign-extend → −1
        assign(3, 5),        // s = b[0] → zero-extend → 255
        systask(SysTaskId::Display, vec![6]),
        systask(SysTaskId::Display, vec![7]),
        systask(SysTaskId::Finish, vec![]),
    ];
    let ir = ir_of(
        vec![
            a_handle(8, true),
            a_handle(8, false),
            reg32(true),
            reg32(true),
        ],
        consts,
        exprs,
        stmts,
    );
    let (res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(out, "         -1\n        255\n");
}

#[test]
fn assoc_nba_write_lands() {
    // a[5] <= 7 in the NBA region, observed after a #1 — the AssocKey offsets
    // variant must survive the schedule→apply trip (same write_lvalue funnel).
    let exprs = vec![
        Expr::Const { val: 0 }, // 0: key 5
        Expr::Const { val: 1 }, // 1: 7
        Expr::Signal {
            net: 0,
            word: Some(0),
        }, // 2: a[5]
        Expr::Const { val: 2 }, // 3: delay 1
    ];
    let consts = vec![int_const(5), int_const(7), int_const(1)];
    let stmts = vec![
        Stmt::NonblockingAssign {
            lhs: sim_ir::Lvalue {
                chunks: vec![sim_ir::LvalChunk {
                    net: 0,
                    word: Some(0),
                    offset: None,
                    width: None,
                    kind: sim_ir::SelKind::Bit,
                }],
            },
            rhs: 1,
            delay: None,
        },
        systask(SysTaskId::Display, vec![2]),
        systask(SysTaskId::Finish, vec![]),
    ];
    let ir = SimIr {
        instances: vec![Instance {
            parent: None,
            module: 0,
            first_net: 0,
            net_count: 1,
        }],
        nets: vec![a_handle(32, false)],
        processes: vec![Process {
            sensitivity: Sensitivity {
                kind: SensKind::Initial,
                edges: Vec::new(),
            },
            body: vec![
                BasicBlock {
                    stmts: vec![0],
                    term: Terminator::Delay {
                        amount: 3,
                        region: DelayRegion::Active,
                        resume: 1,
                    },
                },
                BasicBlock {
                    stmts: vec![1, 2],
                    term: Terminator::Return,
                },
            ],
            entry: 0,
            suspend: suspend0(),
        }],
        cont_assigns: Vec::new(),
        funcs: Vec::new(),
        exprs,
        stmts,
        blocks: Vec::new(),
        consts,
    };
    let (res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(out, "         7\n");
}

#[test]
fn assoc_concat_lvalue_chunk_degrades_loud() {
    // {a[5], r} = 77 — an assoc element inside a CONCAT lvalue is outside the
    // MVP shape (⑥ will loud-reject it). The engine degrades: the assoc chunk
    // is IGNORED + W4020, the sibling reg still gets its slice (77).
    let exprs = vec![
        Expr::Const { val: 0 },              // 0: key 5
        Expr::Const { val: 1 },              // 1: 77
        Expr::Signal { net: 0, word: None }, // 2: handle (num)
        Expr::SysFunc {
            which: SysFuncId::AssocNum,
            args: vec![2],
        }, // 3
        Expr::Signal { net: 1, word: None }, // 4: r
    ];
    let consts = vec![int_const(5), int_const(77)];
    let stmts = vec![
        Stmt::BlockingAssign {
            lhs: sim_ir::Lvalue {
                chunks: vec![
                    sim_ir::LvalChunk {
                        net: 0,
                        word: Some(0),
                        offset: None,
                        width: None,
                        kind: sim_ir::SelKind::Bit,
                    },
                    sim_ir::LvalChunk {
                        net: 1,
                        word: None,
                        offset: None,
                        width: None,
                        kind: sim_ir::SelKind::Bit,
                    },
                ],
            },
            rhs: 1,
        },
        systask(SysTaskId::Display, vec![3]),
        systask(SysTaskId::Display, vec![4]),
        systask(SysTaskId::Finish, vec![]),
    ];
    let ir = ir_of(
        vec![a_handle(32, false), reg32(false)],
        consts,
        exprs,
        stmts,
    );
    let (res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(out, "          0\n        77\n");
    let sink = DiagSink::default();
    simulate(&ir, &sink, SimOpts::default());
    let diags = sink.0.into_inner();
    let warns = diags.iter().filter(|d| d.contains("W4020")).count();
    assert_eq!(warns, 1, "concat assoc chunk degrades loudly: {diags:?}");
}

#[test]
fn assoc_vm_backend_byte_parity() {
    // Same P5-style pre-⑥ gate as the queue: byte-identical stdout on
    // Interpreter vs Bytecode. exists/num are PURE eval arms and delete rides
    // the shared SysTask dispatch, so no new P9 exclusions are involved —
    // parity must hold by construction.
    for ir in [assoc_rw_ir(), assoc_exists_ir(), assoc_delete_ir()] {
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
fn queue_insert_middle_append_and_oob() {
    // {10,20,30}; insert(1,99) → {10,99,20,30}; insert(4,77) appends;
    // insert(9,55) OOB → warn + no-op. (iverilog live.)
    let exprs = vec![
        Expr::Signal { net: 0, word: None }, // 0: handle
        Expr::Const { val: 0 },              // 1: 10
        Expr::Const { val: 1 },              // 2: 20
        Expr::Const { val: 2 },              // 3: 30
        Expr::Const { val: 3 },              // 4: idx 1
        Expr::Const { val: 4 },              // 5: 99
        Expr::Const { val: 5 },              // 6: idx 4
        Expr::Const { val: 6 },              // 7: 77
        Expr::Const { val: 7 },              // 8: idx 9
        Expr::Const { val: 8 },              // 9: 55
        Expr::SysFunc {
            which: SysFuncId::DynSize,
            args: vec![0],
        }, // 10: size
        Expr::Const { val: 3 },              // 11: idx 1 (reused for read)
        Expr::Signal {
            net: 0,
            word: Some(11),
        }, // 12: q[1]
        Expr::Const { val: 9 },              // 13: idx 0
        Expr::Signal {
            net: 0,
            word: Some(13),
        }, // 14: q[0]
        Expr::Const { val: 10 },             // 15: idx 4
        Expr::Signal {
            net: 0,
            word: Some(15),
        }, // 16: q[4]
    ];
    let consts = vec![
        int_const(10),
        int_const(20),
        int_const(30),
        int_const(1),
        int_const(99),
        int_const(4),
        int_const(77),
        int_const(9),
        int_const(55),
        int_const(0),
        int_const(4),
    ];
    let stmts = vec![
        systask(SysTaskId::QPushBack, vec![0, 1]),
        systask(SysTaskId::QPushBack, vec![0, 2]),
        systask(SysTaskId::QPushBack, vec![0, 3]),
        systask(SysTaskId::QInsert, vec![0, 4, 5]), // insert(1, 99)
        systask(SysTaskId::Display, vec![10]),      // 4
        systask(SysTaskId::Display, vec![14]),      // 10
        systask(SysTaskId::Display, vec![12]),      // 99
        systask(SysTaskId::QInsert, vec![0, 6, 7]), // insert(4, 77) — append
        systask(SysTaskId::Display, vec![10]),      // 5
        systask(SysTaskId::Display, vec![16]),      // 77
        systask(SysTaskId::QInsert, vec![0, 8, 9]), // insert(9, 55) — OOB
        systask(SysTaskId::Display, vec![10]),      // 5 (unchanged)
        systask(SysTaskId::Finish, vec![]),
    ];
    let ir = ir_of(vec![q_handle(32, false)], consts, exprs, stmts);
    let (res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(
        out,
        "          4\n        10\n        99\n          5\n        77\n          5\n"
    );
    let sink = DiagSink::default();
    simulate(&ir, &sink, SimOpts::default());
    let diags = sink.0.into_inner();
    assert_eq!(
        diags.iter().filter(|d| d.contains("W4020")).count(),
        1,
        "OOB insert warns once: {diags:?}"
    );
}

#[test]
fn queue_delete_index_and_oob() {
    // {10,99,20,30}; delete(0) → {99,20,30}; delete(2) → {99,20};
    // delete(7) OOB → warn + skip. (iverilog live.)
    let exprs = vec![
        Expr::Signal { net: 0, word: None }, // 0: handle
        Expr::Const { val: 0 },              // 1: 10
        Expr::Const { val: 1 },              // 2: 99
        Expr::Const { val: 2 },              // 3: 20
        Expr::Const { val: 3 },              // 4: 30
        Expr::Const { val: 4 },              // 5: idx 0
        Expr::Const { val: 5 },              // 6: idx 2
        Expr::Const { val: 6 },              // 7: idx 7
        Expr::SysFunc {
            which: SysFuncId::DynSize,
            args: vec![0],
        }, // 8: size
        Expr::Signal {
            net: 0,
            word: Some(5),
        }, // 9: q[0]
        Expr::Const { val: 7 },              // 10: idx 1
        Expr::Signal {
            net: 0,
            word: Some(10),
        }, // 11: q[1]
    ];
    let consts = vec![
        int_const(10),
        int_const(99),
        int_const(20),
        int_const(30),
        int_const(0),
        int_const(2),
        int_const(7),
        int_const(1),
    ];
    let stmts = vec![
        systask(SysTaskId::QPushBack, vec![0, 1]),
        systask(SysTaskId::QPushBack, vec![0, 2]),
        systask(SysTaskId::QPushBack, vec![0, 3]),
        systask(SysTaskId::QPushBack, vec![0, 4]),
        systask(SysTaskId::QDeleteIdx, vec![0, 5]), // delete(0)
        systask(SysTaskId::QDeleteIdx, vec![0, 6]), // delete(2) → erase 30
        systask(SysTaskId::Display, vec![8]),       // 2
        systask(SysTaskId::Display, vec![9]),       // 99
        systask(SysTaskId::Display, vec![11]),      // 20
        systask(SysTaskId::QDeleteIdx, vec![0, 7]), // delete(7) — OOB
        systask(SysTaskId::Display, vec![8]),       // 2 (unchanged)
        systask(SysTaskId::Finish, vec![]),
    ];
    let ir = ir_of(vec![q_handle(32, false)], consts, exprs, stmts);
    let (res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(out, "          2\n        99\n        20\n          2\n");
    let sink = DiagSink::default();
    simulate(&ir, &sink, SimOpts::default());
    let diags = sink.0.into_inner();
    assert_eq!(
        diags.iter().filter(|d| d.contains("W4020")).count(),
        1,
        "OOB delete warns once: {diags:?}"
    );
}

#[test]
fn assoc_first_next_ascending_then_exhausted() {
    let (res, out) = simulate_capture(&assoc_iter_ir(false), SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    // st/k pairs: (1,−3) (1,7) (1,100) (0,100 — k UNCHANGED on exhaustion).
    assert_eq!(
        out,
        "          1\n         -3\n          1\n          7\n          1\n        100\n          0\n        100\n"
    );
}

#[test]
fn assoc_last_prev_descending_then_exhausted() {
    let (res, out) = simulate_capture(&assoc_iter_ir(true), SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(
        out,
        "          1\n        100\n          1\n          7\n          1\n         -3\n          0\n         -3\n"
    );
}

#[test]
fn assoc_first_on_empty_returns_zero_key_unchanged() {
    let mut exprs = vec![
        Expr::Signal { net: 0, word: None }, // 0: handle
        Expr::Const { val: 0 },              // 1: 5
        Expr::Signal { net: 1, word: None }, // 2: k
        Expr::Signal { net: 2, word: None }, // 3: st
    ];
    let consts = vec![int_const(5)];
    let mut stmts = vec![assign(1, 1)]; // k = 5
    stmts.push(iter_assign(2, SysFuncId::AssocFirst, 0, 2, &mut exprs));
    stmts.push(systask(SysTaskId::Display, vec![3])); // st = 0
    stmts.push(systask(SysTaskId::Display, vec![2])); // k stays 5
    stmts.push(systask(SysTaskId::Finish, vec![]));
    let ir = ir_of(
        vec![a_handle(32, false), reg32(true), reg32(true)],
        consts,
        exprs,
        stmts,
    );
    let (res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(out, "          0\n          5\n");
}

#[test]
fn assoc_iter_narrow_key_truncates_with_minus1() {
    // a[300]=1 with an 8-bit signed ref var: 300 does not fit → st = −1,
    // k = truncated low byte (300 & 0xFF = 44). Hand-IEEE §7.9.4.
    let narrow_reg = NetVar {
        kind: NetKind::Reg,
        width: 8,
        msb: 7,
        lsb: 0,
        signed: true,
        array_len: 0,
        dir: PortDir::Internal,
        init: BitPacked {
            val: vec![0],
            unk: vec![0xff],
        },
    };
    let mut exprs = vec![
        Expr::Signal { net: 0, word: None }, // 0: handle
        Expr::Const { val: 0 },              // 1: key 300
        Expr::Const { val: 1 },              // 2: 1
        Expr::Signal { net: 1, word: None }, // 3: k8
        Expr::Signal { net: 2, word: None }, // 4: st
    ];
    let consts = vec![int_const(300), int_const(1)];
    let mut stmts = vec![elem_write(0, 1, 2)];
    stmts.push(iter_assign(2, SysFuncId::AssocFirst, 0, 3, &mut exprs));
    stmts.push(systask(SysTaskId::Display, vec![4])); // st = −1
    stmts.push(systask(SysTaskId::Display, vec![3])); // k8 = 44
    stmts.push(systask(SysTaskId::Finish, vec![]));
    let ir = ir_of(
        vec![a_handle(32, false), narrow_reg, reg32(true)],
        consts,
        exprs,
        stmts,
    );
    let (res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(out, "         -1\n  44\n");
    let sink = DiagSink::default();
    simulate(&ir, &sink, SimOpts::default());
    let diags = sink.0.into_inner();
    assert_eq!(
        diags.iter().filter(|d| d.contains("W4020")).count(),
        1,
        "truncated iter key warns once: {diags:?}"
    );
}

#[test]
fn queue_dense_first_next_walk() {
    // The internal foreach target: first/next on a QUEUE = dense 0..size-1.
    // {5,10}: first → k=0 st=1; next → k=1 st=1; next → st=0, k unchanged.
    let mut exprs = vec![
        Expr::Signal { net: 0, word: None }, // 0: handle
        Expr::Const { val: 0 },              // 1: 5
        Expr::Const { val: 1 },              // 2: 10
        Expr::Signal { net: 1, word: None }, // 3: k
        Expr::Signal { net: 2, word: None }, // 4: st
    ];
    let consts = vec![int_const(5), int_const(10)];
    let mut stmts = vec![
        systask(SysTaskId::QPushBack, vec![0, 1]),
        systask(SysTaskId::QPushBack, vec![0, 2]),
    ];
    stmts.push(iter_assign(2, SysFuncId::AssocFirst, 0, 3, &mut exprs));
    stmts.push(systask(SysTaskId::Display, vec![4]));
    stmts.push(systask(SysTaskId::Display, vec![3]));
    stmts.push(iter_assign(2, SysFuncId::AssocNext, 0, 3, &mut exprs));
    stmts.push(systask(SysTaskId::Display, vec![4]));
    stmts.push(systask(SysTaskId::Display, vec![3]));
    stmts.push(iter_assign(2, SysFuncId::AssocNext, 0, 3, &mut exprs));
    stmts.push(systask(SysTaskId::Display, vec![4]));
    stmts.push(systask(SysTaskId::Display, vec![3]));
    stmts.push(systask(SysTaskId::Finish, vec![]));
    let ir = ir_of(
        vec![q_handle(32, false), reg32(true), reg32(true)],
        consts,
        exprs,
        stmts,
    );
    let (res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(
        out,
        "          1\n          0\n          1\n          1\n          0\n          1\n"
    );
}

#[test]
fn assoc_iter_vm_backend_byte_parity() {
    // The iter rhs is side-effecting → excluded from codegen (P9 allow-list);
    // the VM must fall back to the interpreter and stay byte-identical.
    for ir in [assoc_iter_ir(false), assoc_iter_ir(true)] {
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
fn assoc_str_write_read_exists_delete_roundtrip() {
    // a["ab"]=7 (16-bit key); read back via a 32-bit ZERO-PADDED "ab" (same
    // key after the leading-null strip); exists("ab")=1, exists("cd")=0;
    // delete("ab") → num 0; delete("zz") silent.
    let exprs = vec![
        Expr::Const { val: 0 }, // 0: "ab" @16
        Expr::Const { val: 1 }, // 1: 7
        Expr::Const { val: 2 }, // 2: "ab" @32 (padded)
        Expr::Signal {
            net: 0,
            word: Some(2),
        }, // 3: a["ab" padded]
        Expr::Signal { net: 0, word: None }, // 4: handle
        Expr::SysFunc {
            which: SysFuncId::AssocExists,
            args: vec![4, 0],
        }, // 5: exists("ab")
        Expr::Const { val: 3 }, // 6: "cd"
        Expr::SysFunc {
            which: SysFuncId::AssocExists,
            args: vec![4, 6],
        }, // 7: exists("cd")
        Expr::SysFunc {
            which: SysFuncId::AssocNum,
            args: vec![4],
        }, // 8: num
        Expr::Const { val: 4 }, // 9: "zz"
    ];
    let consts = vec![
        str_const("ab", 0),
        int_const(7),
        str_const("ab", 32),
        str_const("cd", 0),
        str_const("zz", 0),
    ];
    let stmts = vec![
        elem_write(0, 0, 1),
        systask(SysTaskId::Display, vec![3]),           // 7
        systask(SysTaskId::Display, vec![5]),           // 1
        systask(SysTaskId::Display, vec![7]),           // 0
        systask(SysTaskId::Display, vec![8]),           // 1
        systask(SysTaskId::AssocDeleteKey, vec![4, 9]), // delete("zz") silent
        systask(SysTaskId::AssocDeleteKey, vec![4, 0]), // delete("ab")
        systask(SysTaskId::Display, vec![8]),           // 0
        systask(SysTaskId::Finish, vec![]),
    ];
    let ir = ir_of(vec![as_handle(32, false)], consts, exprs, stmts);
    let (res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(out, "         7\n1\n0\n          1\n          0\n");
    let sink = DiagSink::default();
    simulate(&ir, &sink, SimOpts::default());
    let diags = sink.0.into_inner();
    assert!(
        diags.iter().filter(|d| d.contains("W4020")).count() == 0,
        "clean roundtrip warns nothing: {diags:?}"
    );
}

#[test]
fn assoc_str_first_next_lexicographic() {
    // keys "b", "aa" → lexicographic byte order: "aa" < "b". 64-bit ref var
    // receives the packed bytes right-justified.
    let key_reg = NetVar {
        kind: NetKind::Reg,
        width: 64,
        msb: 63,
        lsb: 0,
        signed: false,
        array_len: 0,
        dir: PortDir::Internal,
        init: BitPacked {
            val: vec![0],
            unk: vec![u64::MAX],
        },
    };
    let mut exprs = vec![
        Expr::Signal { net: 0, word: None }, // 0: handle
        Expr::Const { val: 0 },              // 1: "b"
        Expr::Const { val: 1 },              // 2: 1
        Expr::Const { val: 2 },              // 3: "aa"
        Expr::Const { val: 3 },              // 4: 2
        Expr::Signal { net: 1, word: None }, // 5: k64
        Expr::Signal { net: 2, word: None }, // 6: st
    ];
    let consts = vec![
        str_const("b", 0),
        int_const(1),
        str_const("aa", 0),
        int_const(2),
    ];
    let mut stmts = vec![elem_write(0, 1, 2), elem_write(0, 3, 4)];
    stmts.push(iter_assign(2, SysFuncId::AssocFirst, 0, 5, &mut exprs));
    stmts.push(systask(SysTaskId::Display, vec![6])); // 1
    stmts.push(systask(SysTaskId::Display, vec![5])); // "aa" = 0x6161 = 24929
    stmts.push(iter_assign(2, SysFuncId::AssocNext, 0, 5, &mut exprs));
    stmts.push(systask(SysTaskId::Display, vec![6])); // 1
    stmts.push(systask(SysTaskId::Display, vec![5])); // "b" = 0x62 = 98
    stmts.push(iter_assign(2, SysFuncId::AssocNext, 0, 5, &mut exprs));
    stmts.push(systask(SysTaskId::Display, vec![6])); // 0
    stmts.push(systask(SysTaskId::Finish, vec![]));
    let ir = ir_of(
        vec![as_handle(32, false), key_reg, reg32(true)],
        consts,
        exprs,
        stmts,
    );
    let (res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    // 64-bit unsigned display pads to 20 columns.
    assert_eq!(
        out,
        "          1\n               24929\n          1\n                  98\n          0\n"
    );
}

#[test]
fn assoc_str_x_key_lanes_warn() {
    // X key: write ignored, read X, exists 0 — same family as the i64 lanes.
    let exprs = vec![
        Expr::Const { val: 0 }, // 0: X key
        Expr::Const { val: 1 }, // 1: 7
        Expr::Signal {
            net: 0,
            word: Some(0),
        }, // 2: a[X]
        Expr::Signal { net: 0, word: None }, // 3: handle
        Expr::SysFunc {
            which: SysFuncId::AssocExists,
            args: vec![3, 0],
        }, // 4
        Expr::SysFunc {
            which: SysFuncId::AssocNum,
            args: vec![3],
        }, // 5
    ];
    let consts = vec![x_const(), int_const(7)];
    let stmts = vec![
        elem_write(0, 0, 1),                  // write a[X] — ignored
        systask(SysTaskId::Display, vec![5]), // num 0
        systask(SysTaskId::Display, vec![2]), // read a[X] — X
        systask(SysTaskId::Display, vec![4]), // exists X — 0
        systask(SysTaskId::Finish, vec![]),
    ];
    let ir = ir_of(vec![as_handle(8, false)], consts, exprs, stmts);
    let (res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(out, "          0\n  x\n0\n");
    let sink = DiagSink::default();
    simulate(&ir, &sink, SimOpts::default());
    let diags = sink.0.into_inner();
    assert_eq!(
        diags.iter().filter(|d| d.contains("W4020")).count(),
        1,
        "X-key lanes share the once-latch: {diags:?}"
    );
}
