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

use std::cell::RefCell;

use diag::{LogEvent, LogSink};
use sim_engine::{simulate, simulate_capture, Backend, FinishReason, SimOpts};
use sim_ir::{
    BasicBlock, BinOp, BitPacked, ConstRepr, ConstVal, DelayRegion, Expr, Instance, JoinState,
    NetKind, NetVar, PortDir, ProcFlags, Process, RegionTag, SensKind, Sensitivity, SimIr, Stmt,
    SuspendState, SysFuncId, SysTaskId, Terminator, WakeCond, WakeKey,
};

/// Diagnostic collector (runtime warns ride the LogSink, not stdout).
#[derive(Default)]
struct DiagSink(RefCell<Vec<String>>);
impl LogSink for DiagSink {
    fn emit(&self, e: LogEvent) {
        if let LogEvent::Diagnostic(d) = e {
            self.0.borrow_mut().push(format!(
                "{}[{}]: {}",
                d.severity.token(),
                d.code.code_num(),
                d.message
            ));
        }
    }
}

fn suspend0() -> SuspendState {
    SuspendState {
        resume_pc: 0,
        locals: Vec::new(),
        join_state: JoinState {
            parent: None,
            children: Vec::new(),
            detached: Vec::new(),
            flags: ProcFlags(0),
        },
        wake_key: WakeKey {
            cond: WakeCond::Level { nets: Vec::new() },
            region: RegionTag::Active,
            tie_break: 0,
        },
        call_stack: Vec::new(),
        frame_arena: Vec::new(),
    }
}

/// 8-bit dyn-array HANDLE net: element width 8, `array_len 0`, flat-store cell
/// is a well-formed all-X byte the engine never reads through the dyn path.
fn dyn_handle() -> NetVar {
    NetVar {
        kind: NetKind::DynArray,
        width: 8,
        msb: 7,
        lsb: 0,
        signed: false,
        array_len: 0,
        dir: PortDir::Internal,
        init: BitPacked {
            val: vec![0],
            unk: vec![0xff],
        },
    }
}

fn int_const(v: u64) -> ConstVal {
    ConstVal {
        width: 32,
        signed: true,
        repr: ConstRepr::Numeric,
        bits: BitPacked {
            val: vec![v],
            unk: vec![0],
        },
    }
}

fn x_const() -> ConstVal {
    ConstVal {
        width: 32,
        signed: true,
        repr: ConstRepr::Numeric,
        bits: BitPacked {
            val: vec![0],
            unk: vec![0xffff_ffff],
        },
    }
}

/// One initial process over the given arenas; every stmt in one BB → Return.
fn ir_of(nets: Vec<NetVar>, consts: Vec<ConstVal>, exprs: Vec<Expr>, stmts: Vec<Stmt>) -> SimIr {
    let stmt_ids: Vec<u32> = (0..stmts.len() as u32).collect();
    SimIr {
        instances: vec![Instance {
            parent: None,
            module: 0,
            first_net: 0,
            net_count: nets.len() as u32,
        }],
        nets,
        processes: vec![Process {
            sensitivity: Sensitivity {
                kind: SensKind::Initial,
                edges: Vec::new(),
            },
            body: vec![BasicBlock {
                stmts: stmt_ids,
                term: Terminator::Return,
            }],
            entry: 0,
            suspend: suspend0(),
        }],
        cont_assigns: Vec::new(),
        funcs: Vec::new(),
        exprs,
        stmts,
        blocks: Vec::new(),
        consts,
    }
}

fn systask(which: SysTaskId, args: Vec<u32>) -> Stmt {
    Stmt::SysTask {
        which,
        fmt: None,
        args,
    }
}

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

/// 32-bit-element handle (padding-stable Display fields, like slice 3a).
fn dyn_handle32() -> NetVar {
    NetVar {
        kind: NetKind::DynArray,
        width: 32,
        msb: 31,
        lsb: 0,
        signed: false,
        array_len: 0,
        dir: PortDir::Internal,
        init: BitPacked {
            val: vec![0],
            unk: vec![0xffff_ffff],
        },
    }
}

fn elem_write(net: u32, idx_eid: u32, rhs_eid: u32) -> Stmt {
    Stmt::BlockingAssign {
        lhs: sim_ir::Lvalue {
            chunks: vec![sim_ir::LvalChunk {
                net,
                word: Some(idx_eid),
                offset: None,
                width: None,
                kind: sim_ir::SelKind::Bit,
            }],
        },
        rhs: rhs_eid,
    }
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

// ───────────────────────── ④ queue engine layer ─────────────────────────

/// Queue HANDLE net: element width/signedness, `array_len 0` (same handle
/// shape as `dyn_handle`, kind = Queue).
fn q_handle(width: u32, signed: bool) -> NetVar {
    NetVar {
        kind: NetKind::Queue,
        width,
        msb: width - 1,
        lsb: 0,
        signed,
        array_len: 0,
        dir: PortDir::Internal,
        init: BitPacked {
            val: vec![0],
            unk: vec![if width >= 64 {
                u64::MAX
            } else {
                (1u64 << width) - 1
            }],
        },
    }
}

/// Plain 32-bit variable net (pop destination).
fn reg32(signed: bool) -> NetVar {
    NetVar {
        kind: NetKind::Reg,
        width: 32,
        msb: 31,
        lsb: 0,
        signed,
        array_len: 0,
        dir: PortDir::Internal,
        init: BitPacked {
            val: vec![0],
            unk: vec![0xffff_ffff],
        },
    }
}

/// Whole-net blocking assign `net = rhs_eid`.
fn assign(net: u32, rhs_eid: u32) -> Stmt {
    Stmt::BlockingAssign {
        lhs: sim_ir::Lvalue {
            chunks: vec![sim_ir::LvalChunk {
                net,
                word: None,
                offset: None,
                width: None,
                kind: sim_ir::SelKind::Bit,
            }],
        },
        rhs: rhs_eid,
    }
}

/// push_back 10, push_back 20, push_front 5 → size 3, q = {5, 10, 20}.
/// (Oracle: iverilog live — 3 / 5 / 10 / 20.)
fn queue_push_ir() -> SimIr {
    let exprs = vec![
        Expr::Signal { net: 0, word: None }, // 0: handle (push 10)
        Expr::Const { val: 0 },              // 1: 10
        Expr::Signal { net: 0, word: None }, // 2: handle (push 20)
        Expr::Const { val: 1 },              // 3: 20
        Expr::Signal { net: 0, word: None }, // 4: handle (push_front 5)
        Expr::Const { val: 2 },              // 5: 5
        Expr::Signal { net: 0, word: None }, // 6: handle (size)
        Expr::SysFunc {
            which: SysFuncId::DynSize,
            args: vec![6],
        }, // 7
        Expr::Const { val: 3 },              // 8: idx 0
        Expr::Signal {
            net: 0,
            word: Some(8),
        }, // 9: q[0]
        Expr::Const { val: 4 },              // 10: idx 1
        Expr::Signal {
            net: 0,
            word: Some(10),
        }, // 11: q[1]
        Expr::Const { val: 5 },              // 12: idx 2
        Expr::Signal {
            net: 0,
            word: Some(12),
        }, // 13: q[2]
    ];
    let consts = vec![
        int_const(10),
        int_const(20),
        int_const(5),
        int_const(0),
        int_const(1),
        int_const(2),
    ];
    let stmts = vec![
        systask(SysTaskId::QPushBack, vec![0, 1]),
        systask(SysTaskId::QPushBack, vec![2, 3]),
        systask(SysTaskId::QPushFront, vec![4, 5]),
        systask(SysTaskId::Display, vec![7]),
        systask(SysTaskId::Display, vec![9]),
        systask(SysTaskId::Display, vec![11]),
        systask(SysTaskId::Display, vec![13]),
        systask(SysTaskId::Finish, vec![]),
    ];
    ir_of(vec![q_handle(32, false)], consts, exprs, stmts)
}

#[test]
fn queue_push_index_size_roundtrip() {
    let (res, out) = simulate_capture(&queue_push_ir(), SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(out, "          3\n         5\n        10\n        20\n");
}

/// pushes {5,10,20}, then x = pop_back (20), y = pop_front (5), size 1.
/// (Oracle: iverilog live.)
fn queue_pop_ir() -> SimIr {
    let exprs = vec![
        Expr::Signal { net: 0, word: None }, // 0: handle (push 10)
        Expr::Const { val: 0 },              // 1: 10
        Expr::Signal { net: 0, word: None }, // 2: handle (push 20)
        Expr::Const { val: 1 },              // 3: 20
        Expr::Signal { net: 0, word: None }, // 4: handle (push_front 5)
        Expr::Const { val: 2 },              // 5: 5
        Expr::Signal { net: 0, word: None }, // 6: handle (pop_back)
        Expr::SysFunc {
            which: SysFuncId::QPopBack,
            args: vec![6],
        }, // 7
        Expr::Signal { net: 0, word: None }, // 8: handle (pop_front)
        Expr::SysFunc {
            which: SysFuncId::QPopFront,
            args: vec![8],
        }, // 9
        Expr::Signal { net: 1, word: None }, // 10: x
        Expr::Signal { net: 2, word: None }, // 11: y
        Expr::Signal { net: 0, word: None }, // 12: handle (size)
        Expr::SysFunc {
            which: SysFuncId::DynSize,
            args: vec![12],
        }, // 13
    ];
    let consts = vec![int_const(10), int_const(20), int_const(5)];
    let stmts = vec![
        systask(SysTaskId::QPushBack, vec![0, 1]),
        systask(SysTaskId::QPushBack, vec![2, 3]),
        systask(SysTaskId::QPushFront, vec![4, 5]),
        assign(1, 7), // x = q.pop_back()  → 20
        assign(2, 9), // y = q.pop_front() → 5
        systask(SysTaskId::Display, vec![10]),
        systask(SysTaskId::Display, vec![11]),
        systask(SysTaskId::Display, vec![13]),
        systask(SysTaskId::Finish, vec![]),
    ];
    ir_of(
        vec![q_handle(32, false), reg32(false), reg32(false)],
        consts,
        exprs,
        stmts,
    )
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

// ───────────────────────── ⑤ assoc engine layer ─────────────────────────
//
// iverilog 13.0 does NOT support associative arrays (the `[int]`/`[longint]`/
// `[*]` declarations are syntax/elaboration errors — probed live 2026-06-11),
// so unlike ③/④ there is no live-oracle lane. Semantics below are HAND-IEEE
// pinned (1800-2017 §7.8 / §7.9, same precedent as the expression-force lane):
//   read missing key / X-Z key  → element-width X + W4020 (once per net, §7.8.6)
//   write X/Z key               → IGNORED + W4020 (§7.8.6)
//   write missing key           → creates the element (§7.8)
//   exists(k)                   → 1/0; X key → 0 (+ the same once-latch warn)
//   num()/size()                → entry count (int)
//   delete(k) on a missing key  → silent no-op (§7.9); delete() clears
// Key domain at the ENGINE seam = i64 (⑥ elaborate casts the surface key type
// down/up before the IR, so negative AND beyond-u32 keys must round-trip).

/// Assoc HANDLE net: element width/signedness, `array_len 0` (same handle
/// shape as `q_handle`, kind = Assoc).
fn a_handle(width: u32, signed: bool) -> NetVar {
    NetVar {
        kind: NetKind::Assoc,
        width,
        msb: width - 1,
        lsb: 0,
        signed,
        array_len: 0,
        dir: PortDir::Internal,
        init: BitPacked {
            val: vec![0],
            unk: vec![if width >= 64 {
                u64::MAX
            } else {
                (1u64 << width) - 1
            }],
        },
    }
}

/// 64-bit signed const (keys beyond the u32 sentinel domain).
fn long_const(v: u64) -> ConstVal {
    ConstVal {
        width: 64,
        signed: true,
        repr: ConstRepr::Numeric,
        bits: BitPacked {
            val: vec![v],
            unk: vec![0],
        },
    }
}

/// a[5]=10; a[-3]=20 → a[5], a[-3], num() = 10 / 20 / 2. The −3 key pins the
/// SIGNED i64 key domain (a u32-index funnel would sentinel it to X).
fn assoc_rw_ir() -> SimIr {
    let exprs = vec![
        Expr::Const { val: 0 }, // 0: key 5
        Expr::Const { val: 1 }, // 1: 10
        Expr::Const { val: 2 }, // 2: key −3
        Expr::Const { val: 3 }, // 3: 20
        Expr::Signal {
            net: 0,
            word: Some(0),
        }, // 4: a[5]
        Expr::Signal {
            net: 0,
            word: Some(2),
        }, // 5: a[-3]
        Expr::Signal { net: 0, word: None }, // 6: handle (num)
        Expr::SysFunc {
            which: SysFuncId::AssocNum,
            args: vec![6],
        }, // 7
    ];
    let consts = vec![
        int_const(5),
        int_const(10),
        int_const(0xFFFF_FFFD), // 32-bit signed −3
        int_const(20),
    ];
    let stmts = vec![
        elem_write(0, 0, 1),
        elem_write(0, 2, 3),
        systask(SysTaskId::Display, vec![4]),
        systask(SysTaskId::Display, vec![5]),
        systask(SysTaskId::Display, vec![7]),
        systask(SysTaskId::Finish, vec![]),
    ];
    ir_of(vec![a_handle(32, false)], consts, exprs, stmts)
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

/// a[5]=1; exists(5)/exists(6)/exists(X) → 1 / 0 / 0 (X key matches nothing).
fn assoc_exists_ir() -> SimIr {
    let exprs = vec![
        Expr::Const { val: 0 },              // 0: key 5
        Expr::Const { val: 1 },              // 1: 1
        Expr::Signal { net: 0, word: None }, // 2: handle
        Expr::Const { val: 0 },              // 3: key 5 (exists hit)
        Expr::SysFunc {
            which: SysFuncId::AssocExists,
            args: vec![2, 3],
        }, // 4
        Expr::Signal { net: 0, word: None }, // 5: handle
        Expr::Const { val: 2 },              // 6: key 6 (exists miss)
        Expr::SysFunc {
            which: SysFuncId::AssocExists,
            args: vec![5, 6],
        }, // 7
        Expr::Signal { net: 0, word: None }, // 8: handle
        Expr::Const { val: 3 },              // 9: X key
        Expr::SysFunc {
            which: SysFuncId::AssocExists,
            args: vec![8, 9],
        }, // 10
    ];
    let consts = vec![int_const(5), int_const(1), int_const(6), x_const()];
    let stmts = vec![
        elem_write(0, 0, 1),
        systask(SysTaskId::Display, vec![4]),
        systask(SysTaskId::Display, vec![7]),
        systask(SysTaskId::Display, vec![10]),
        systask(SysTaskId::Finish, vec![]),
    ];
    ir_of(vec![a_handle(32, false)], consts, exprs, stmts)
}

#[test]
fn assoc_exists_hit_miss_and_x_key() {
    let (res, out) = simulate_capture(&assoc_exists_ir(), SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(out, "1\n0\n0\n"); // exists is 1-bit (width table)
}

/// a[1]=10; a[2]=20; delete(1) → num 1, exists(1) 0; delete(99) (missing —
/// SILENT no-op, §7.9); delete() → num 0.
fn assoc_delete_ir() -> SimIr {
    let exprs = vec![
        Expr::Const { val: 0 },              // 0: key 1
        Expr::Const { val: 1 },              // 1: 10
        Expr::Const { val: 2 },              // 2: key 2
        Expr::Const { val: 3 },              // 3: 20
        Expr::Signal { net: 0, word: None }, // 4: handle (delete(1))
        Expr::Const { val: 0 },              // 5: key 1
        Expr::Signal { net: 0, word: None }, // 6: handle (num #1)
        Expr::SysFunc {
            which: SysFuncId::AssocNum,
            args: vec![6],
        }, // 7
        Expr::Signal { net: 0, word: None }, // 8: handle
        Expr::Const { val: 0 },              // 9: key 1
        Expr::SysFunc {
            which: SysFuncId::AssocExists,
            args: vec![8, 9],
        }, // 10
        Expr::Signal { net: 0, word: None }, // 11: handle (delete(99))
        Expr::Const { val: 4 },              // 12: key 99
        Expr::Signal { net: 0, word: None }, // 13: handle (num #2)
        Expr::SysFunc {
            which: SysFuncId::AssocNum,
            args: vec![13],
        }, // 14
        Expr::Signal { net: 0, word: None }, // 15: handle (delete())
        Expr::Signal { net: 0, word: None }, // 16: handle (num #3)
        Expr::SysFunc {
            which: SysFuncId::AssocNum,
            args: vec![16],
        }, // 17
    ];
    let consts = vec![
        int_const(1),
        int_const(10),
        int_const(2),
        int_const(20),
        int_const(99),
    ];
    let stmts = vec![
        elem_write(0, 0, 1),
        elem_write(0, 2, 3),
        systask(SysTaskId::AssocDeleteKey, vec![4, 5]),
        systask(SysTaskId::Display, vec![7]),
        systask(SysTaskId::Display, vec![10]),
        systask(SysTaskId::AssocDeleteKey, vec![11, 12]),
        systask(SysTaskId::Display, vec![14]),
        systask(SysTaskId::DynDelete, vec![15]),
        systask(SysTaskId::Display, vec![17]),
        systask(SysTaskId::Finish, vec![]),
    ];
    ir_of(vec![a_handle(32, false)], consts, exprs, stmts)
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

// ───────────────────────── v6 follow-on batch ─────────────────────────
//
// queue `.insert(i, v)` / `.delete(i)` — iverilog 13.0 live oracle
// (2026-06-11): insert middle shifts right; `insert(size, v)` APPENDS;
// OOB insert = warning + not added; `delete(i)` erases one; OOB delete =
// warning + skip.
//
// assoc `.first/.next/.last/.prev(k)` — NO iverilog lane (assoc unsupported),
// HAND-IEEE pinned (1800-2017 §7.9.4):
//   found → key var WRITTEN, return 1; none/empty → key var UNCHANGED, ret 0;
//   key does not fit the (too-narrow) ref var → TRUNCATED write + return −1
//   (+ our W4020 once-latch). Order = signed-i64 ascending (the engine key
//   domain; a `[time]`-keyed array with ≥2^63 keys deviates from unsigned
//   order — documented limitation).
// On dyn/queue handles the same SysFuncIds serve the DENSE 0..size-1 walk
// (the internal `foreach` desugar target — user surface stays assoc-only).

/// `st = handle.first/next/…(k)` — BlockingAssign with the iter SysFunc rhs.
fn iter_assign(
    st_net: u32,
    which: SysFuncId,
    handle_eid: u32,
    key_eid: u32,
    exprs: &mut Vec<Expr>,
) -> Stmt {
    let rhs = exprs.len() as u32;
    exprs.push(Expr::SysFunc {
        which,
        args: vec![handle_eid, key_eid],
    });
    assign(st_net, rhs)
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

/// a[-3]=1, a[7]=2, a[100]=3, then first/next ×3 → ascending keys, last 0.
fn assoc_iter_ir(descend: bool) -> SimIr {
    // nets: 0 = assoc handle, 1 = k (32-bit signed), 2 = st (32-bit signed)
    let mut exprs = vec![
        Expr::Signal { net: 0, word: None }, // 0: handle
        Expr::Const { val: 0 },              // 1: key −3
        Expr::Const { val: 1 },              // 2: 1
        Expr::Const { val: 2 },              // 3: key 7
        Expr::Const { val: 3 },              // 4: 2
        Expr::Const { val: 4 },              // 5: key 100
        Expr::Const { val: 5 },              // 6: 3
        Expr::Signal { net: 1, word: None }, // 7: k (read + ref arg)
        Expr::Signal { net: 2, word: None }, // 8: st
    ];
    let consts = vec![
        int_const(0xFFFF_FFFD), // −3
        int_const(1),
        int_const(7),
        int_const(2),
        int_const(100),
        int_const(3),
    ];
    let (a, b) = if descend {
        (SysFuncId::AssocLast, SysFuncId::AssocPrev)
    } else {
        (SysFuncId::AssocFirst, SysFuncId::AssocNext)
    };
    let mut stmts = vec![
        elem_write(0, 1, 2),
        elem_write(0, 3, 4),
        elem_write(0, 5, 6),
    ];
    stmts.push(iter_assign(2, a, 0, 7, &mut exprs));
    stmts.push(systask(SysTaskId::Display, vec![8])); // st
    stmts.push(systask(SysTaskId::Display, vec![7])); // k
    for _ in 0..3 {
        stmts.push(iter_assign(2, b, 0, 7, &mut exprs));
        stmts.push(systask(SysTaskId::Display, vec![8]));
        stmts.push(systask(SysTaskId::Display, vec![7]));
    }
    stmts.push(systask(SysTaskId::Finish, vec![]));
    ir_of(
        vec![a_handle(32, false), reg32(true), reg32(true)],
        consts,
        exprs,
        stmts,
    )
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

// ── v6 string-keyed assoc (NetKind::AssocStr) ──
//
// NO iverilog lane (assoc unsupported) — hand-IEEE pinned (§7.8.2 family):
// keys are byte strings; an integral key expression converts by stripping
// leading 0x00 bytes (packed-ASCII, §6.16 conversion family), so the SAME
// text at different packed widths is the SAME key. X/Z key = invalid index
// (read X / write ignored / exists 0, each + W4020 once). Order for first/
// next = lexicographic byte order (IEEE string compare).

/// String-keyed assoc HANDLE net.
fn as_handle(width: u32, signed: bool) -> NetVar {
    NetVar {
        kind: NetKind::AssocStr,
        width,
        msb: width - 1,
        lsb: 0,
        signed,
        array_len: 0,
        dir: PortDir::Internal,
        init: BitPacked {
            val: vec![0],
            unk: vec![if width >= 64 {
                u64::MAX
            } else {
                (1u64 << width) - 1
            }],
        },
    }
}

/// Packed-ASCII const of `s` (width = 8×len, unsigned), optionally zero-padded
/// to `pad_width` bits (leading 0x00 bytes — must be the SAME key).
fn str_const(s: &str, pad_width: u32) -> ConstVal {
    let mut v: u64 = 0;
    for b in s.bytes() {
        v = (v << 8) | b as u64;
    }
    let w = (s.len() as u32 * 8).max(8).max(pad_width);
    ConstVal {
        width: w,
        signed: false,
        repr: ConstRepr::Numeric,
        bits: BitPacked {
            val: vec![v],
            unk: vec![0],
        },
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
