#![allow(dead_code)]
#![allow(unused_imports)]
// shared helpers for the split dyn_storage integration tests (mechanical move)

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
pub struct DiagSink(pub RefCell<Vec<String>>);
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

pub fn suspend0() -> SuspendState {
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
pub fn dyn_handle() -> NetVar {
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

pub fn int_const(v: u64) -> ConstVal {
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

pub fn x_const() -> ConstVal {
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
pub fn ir_of(
    nets: Vec<NetVar>,
    consts: Vec<ConstVal>,
    exprs: Vec<Expr>,
    stmts: Vec<Stmt>,
) -> SimIr {
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

pub fn systask(which: SysTaskId, args: Vec<u32>) -> Stmt {
    Stmt::SysTask {
        which,
        fmt: None,
        args,
    }
}

/// 32-bit-element handle (padding-stable Display fields, like slice 3a).
pub fn dyn_handle32() -> NetVar {
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

pub fn elem_write(net: u32, idx_eid: u32, rhs_eid: u32) -> Stmt {
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

// ───────────────────────── ④ queue engine layer ─────────────────────────

/// Queue HANDLE net: element width/signedness, `array_len 0` (same handle
/// shape as `dyn_handle`, kind = Queue).
pub fn q_handle(width: u32, signed: bool) -> NetVar {
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
pub fn reg32(signed: bool) -> NetVar {
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
pub fn assign(net: u32, rhs_eid: u32) -> Stmt {
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
pub fn queue_push_ir() -> SimIr {
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

/// pushes {5,10,20}, then x = pop_back (20), y = pop_front (5), size 1.
/// (Oracle: iverilog live.)
pub fn queue_pop_ir() -> SimIr {
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
pub fn a_handle(width: u32, signed: bool) -> NetVar {
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
pub fn long_const(v: u64) -> ConstVal {
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
pub fn assoc_rw_ir() -> SimIr {
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

/// a[5]=1; exists(5)/exists(6)/exists(X) → 1 / 0 / 0 (X key matches nothing).
pub fn assoc_exists_ir() -> SimIr {
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

/// a[1]=10; a[2]=20; delete(1) → num 1, exists(1) 0; delete(99) (missing —
/// SILENT no-op, §7.9); delete() → num 0.
pub fn assoc_delete_ir() -> SimIr {
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
pub fn iter_assign(
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

/// a[-3]=1, a[7]=2, a[100]=3, then first/next ×3 → ascending keys, last 0.
pub fn assoc_iter_ir(descend: bool) -> SimIr {
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

// ── v6 string-keyed assoc (NetKind::AssocStr) ──
//
// NO iverilog lane (assoc unsupported) — hand-IEEE pinned (§7.8.2 family):
// keys are byte strings; an integral key expression converts by stripping
// leading 0x00 bytes (packed-ASCII, §6.16 conversion family), so the SAME
// text at different packed widths is the SAME key. X/Z key = invalid index
// (read X / write ignored / exists 0, each + W4020 once). Order for first/
// next = lexicographic byte order (IEEE string compare).

/// String-keyed assoc HANDLE net.
pub fn as_handle(width: u32, signed: bool) -> NetVar {
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
pub fn str_const(s: &str, pad_width: u32) -> ConstVal {
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
