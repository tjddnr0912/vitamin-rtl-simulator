//! Frame-call model (automatic/recursive functions) — ENGINE layer (B1
//! Increment 2). The runtime lifts the v1 loud rejection of automatic/recursive
//! functions by lowering each callee body ONCE into the reserved `ir.blocks`
//! func arena and executing it against a per-invocation frame (IR-0: the
//! `Frame`/`FuncDef`/`Expr::Call` shapes were pre-frozen at PR1-B/M3, so
//! `format_version` stays 8).
//!
//! No front-end syntax exists yet (that is Increment 4, batched with the `.vu`
//! flip), so these tests HAND-BUILD a frozen `SimIr` + a populated `FuncTable`
//! and drive them through the public `simulate`/`simulate_capture` seam — exactly
//! what elaborate will emit once the syntax lands (the assoc/iface precedent).
//!
//! Oracle: iverilog 13.0 models automatic recursion (fresh per-call storage,
//! IEEE 1800 §13.4.2) AND static-lifetime corruption faithfully. The probe
//! (`acc = n*10` before the recursive call, read `acc` after) is the lifetime
//! discriminator: for `f = f(n-1) + acc`, automatic `probe(3)=60` (each frame
//! keeps its own `acc`) vs static `probe(3)=30` (the shared `acc` is clobbered
//! to the deepest frame's `10`, so every level adds 10). Oracle-verified live;
//! the REAL-pipeline differential lands at Increment 5 (the `#[ignore]`d section
//! at the bottom). The deep/runaway corpus runs on a large-stack worker thread
//! so the depth CAP — not a host stack overflow — is the guard.

use diag::{LogEvent, LogSink};
use sim_engine::{simulate, simulate_capture, ExitClass, FinishReason, FuncMeta, SimOpts};
use sim_ir::{
    BasicBlock, BinOp, BitPacked, ConstRepr, ConstVal, Expr, FuncDef, Instance, JoinState,
    LvalChunk, Lvalue, NetKind, NetVar, PortDir, ProcFlags, Process, RegionTag, SelKind, SensKind,
    Sensitivity, SimIr, Stmt, SuspendState, SysTaskId, Terminator, WakeCond, WakeKey,
};

// ── construction helpers ─────────────────────────────────────────────────────

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

/// An `integer`/`reg [w-1:0]` net (`Internal`, array_len 1, X-init).
fn int_net(width: u32, signed: bool) -> NetVar {
    let kind = if width == 32 && signed {
        NetKind::Integer
    } else {
        NetKind::Reg
    };
    NetVar {
        kind,
        width,
        msb: width.saturating_sub(1),
        lsb: 0,
        signed,
        array_len: 1,
        dir: PortDir::Internal,
        init: BitPacked {
            val: vec![0],
            unk: vec![(1u64 << width.min(64)).wrapping_sub(1)],
        },
    }
}

/// Incremental builder for a hand-lowered frame design: a single `initial`
/// process plus a `funcs`/`blocks` arena, mirroring elaborate's future emission.
#[derive(Default)]
struct B {
    nets: Vec<NetVar>,
    consts: Vec<ConstVal>,
    exprs: Vec<Expr>,
    stmts: Vec<Stmt>,
    blocks: Vec<BasicBlock>,
    funcs: Vec<FuncDef>,
    func_table: Vec<FuncMeta>,
}

impl B {
    fn net(&mut self, nv: NetVar) -> u32 {
        self.nets.push(nv);
        self.nets.len() as u32 - 1
    }
    /// A `Const` expr of value `v` (32-bit signed numeric). Returns its ExprId.
    fn k(&mut self, v: i64) -> u32 {
        let cid = self.consts.len() as u32;
        self.consts.push(ConstVal {
            width: 32,
            signed: true,
            repr: ConstRepr::Numeric,
            bits: BitPacked {
                val: vec![v as u64 & 0xffff_ffff],
                unk: vec![0],
            },
        });
        self.expr(Expr::Const { val: cid })
    }
    fn sig(&mut self, net: u32) -> u32 {
        self.expr(Expr::Signal { net, word: None })
    }
    fn bin(&mut self, op: BinOp, lhs: u32, rhs: u32) -> u32 {
        self.expr(Expr::Binary { op, lhs, rhs })
    }
    fn call(&mut self, func: u32, args: Vec<u32>) -> u32 {
        self.expr(Expr::Call { func, args })
    }
    fn expr(&mut self, e: Expr) -> u32 {
        self.exprs.push(e);
        self.exprs.len() as u32 - 1
    }
    /// `net = rhs` whole-net blocking assign. Returns its StmtId.
    fn assign(&mut self, net: u32, rhs: u32) -> u32 {
        self.stmts.push(Stmt::BlockingAssign {
            lhs: Lvalue {
                chunks: vec![LvalChunk {
                    net,
                    word: None,
                    offset: None,
                    width: None,
                    kind: SelKind::Bit,
                }],
            },
            rhs,
        });
        self.stmts.len() as u32 - 1
    }
    fn block(&mut self, stmts: Vec<u32>, term: Terminator) -> u32 {
        self.blocks.push(BasicBlock { stmts, term });
        self.blocks.len() as u32 - 1
    }
    fn display(&mut self, arg: u32) -> u32 {
        self.stmts.push(Stmt::SysTask {
            which: SysTaskId::Display,
            fmt: None,
            args: vec![arg],
        });
        self.stmts.len() as u32 - 1
    }
    fn finish(&mut self) -> u32 {
        self.stmts.push(Stmt::SysTask {
            which: SysTaskId::Finish,
            fmt: None,
            args: vec![],
        });
        self.stmts.len() as u32 - 1
    }

    /// Lower a recursive function into the func arena: 2 frame nets (n, return)
    /// for the plain factorial `f(n) = (n<=1)?1 : n*f(n-1)`, or 3 nets
    /// (n, return, acc) for the lifetime `probe`. `probe` computes `acc = n*10`
    /// UNCONDITIONALLY (in the entry block, before the branch), then
    /// `f = (n<=1)? acc : f(n-1) + acc` — the discriminator: automatic keeps a
    /// per-frame `acc`, static shares one slot. `automatic` only flips the
    /// FuncMeta storage policy. Returns the FuncId.
    fn lower_recursive(
        &mut self,
        automatic: bool,
        probe: bool,
        ret_w: u32,
        ret_signed: bool,
    ) -> u32 {
        let func = self.funcs.len() as u32;
        let base = self.nets.len() as u32;
        // slots: [0]=n, [1]=return, [2]=acc (probe only)
        let n_net = self.net(int_net(32, true));
        let _ret_net = self.net(int_net(ret_w, ret_signed));
        let acc_net = if probe {
            Some(self.net(int_net(32, true)))
        } else {
            None
        };
        let return_slot = 1u32;
        let locals_len = if probe { 3 } else { 2 };
        let ret_slot_net = base + return_slot;

        // entry-block pre-statement (probe: acc = n*10, computed unconditionally).
        let mut entry_stmts = Vec::new();
        if let Some(acc) = acc_net {
            let n_a = self.sig(n_net);
            let ten = self.k(10);
            let n10 = self.bin(BinOp::Mul, n_a, ten);
            entry_stmts.push(self.assign(acc, n10));
        }

        // condition: n <= 1
        let n_c = self.sig(n_net);
        let one_c = self.k(1);
        let cond = self.bin(BinOp::Le, n_c, one_c);

        // then (base case): probe → return = acc; factorial → return = 1.
        let then_rhs = if let Some(acc) = acc_net {
            self.sig(acc)
        } else {
            self.k(1)
        };
        let s_then = self.assign(ret_slot_net, then_rhs);

        // else (recursive): probe → f(n-1) + acc; factorial → n * f(n-1).
        let n_r = self.sig(n_net);
        let one_r = self.k(1);
        let nm1 = self.bin(BinOp::Sub, n_r, one_r);
        let rec = self.call(func, vec![nm1]);
        let else_rhs = if let Some(acc) = acc_net {
            let acc_rd = self.sig(acc); // f(n-1) + acc  (call+acc operand order)
            self.bin(BinOp::Add, rec, acc_rd)
        } else {
            let n_m = self.sig(n_net); // n * f(n-1)
            self.bin(BinOp::Mul, n_m, rec)
        };
        let s_else = self.assign(ret_slot_net, else_rhs);

        // blocks: entry(pre + Branch) → then(Return) / else(Return)
        let b_then = self.block(vec![s_then], Terminator::Return);
        let b_else = self.block(vec![s_else], Terminator::Return);
        let b_entry = self.block(
            entry_stmts,
            Terminator::Branch {
                cond,
                then_bb: b_then,
                else_bb: b_else,
            },
        );

        self.funcs.push(FuncDef {
            entry: b_entry,
            n_params: 1,
            locals_len,
            is_task: false,
        });
        self.func_table.push(FuncMeta {
            base_net: base,
            n_params: 1,
            return_slot,
            locals_len,
            is_automatic: automatic,
            ret_width: ret_w,
            ret_signed,
            auto_override: 0,
        });
        func
    }

    /// Lower `cnt(n) = (n<=0)?0 : n + cnt(n-1)` (a deep-recursion counter:
    /// `cnt(k)=k*(k+1)/2`). Single func, 2 frame nets, 3 BBs. Returns FuncId.
    fn lower_counter(&mut self, automatic: bool) -> u32 {
        let func = self.funcs.len() as u32;
        let base = self.nets.len() as u32;
        let n_net = self.net(int_net(32, true));
        let _ret = self.net(int_net(32, true));
        let ret_slot_net = base + 1;
        let n = self.sig(n_net);
        let zero = self.k(0);
        let cond = self.bin(BinOp::Le, n, zero);
        let then_rhs = self.k(0);
        let n2 = self.sig(n_net);
        let one = self.k(1);
        let nm1 = self.bin(BinOp::Sub, n2, one);
        let rec = self.call(func, vec![nm1]);
        let n3 = self.sig(n_net);
        let sum = self.bin(BinOp::Add, n3, rec);
        let s_then = self.assign(ret_slot_net, then_rhs);
        let s_else = self.assign(ret_slot_net, sum);
        let b_then = self.block(vec![s_then], Terminator::Return);
        let b_else = self.block(vec![s_else], Terminator::Return);
        let b_entry = self.block(
            vec![],
            Terminator::Branch {
                cond,
                then_bb: b_then,
                else_bb: b_else,
            },
        );
        self.funcs.push(FuncDef {
            entry: b_entry,
            n_params: 1,
            locals_len: 2,
            is_task: false,
        });
        self.func_table.push(FuncMeta {
            base_net: base,
            n_params: 1,
            return_slot: 1,
            locals_len: 2,
            is_automatic: automatic,
            ret_width: 32,
            ret_signed: true,
            auto_override: 0,
        });
        func
    }

    /// Finish the build: wrap `proc_stmts` in one `initial` process and assemble
    /// the `SimIr` + matching `SimOpts.func_table`.
    fn build(self, proc_stmts: Vec<u32>) -> (SimIr, SimOpts) {
        let ir = SimIr {
            instances: vec![Instance {
                parent: None,
                module: 0,
                first_net: 0,
                net_count: self.nets.len() as u32,
            }],
            nets: self.nets,
            processes: vec![Process {
                sensitivity: Sensitivity {
                    kind: SensKind::Initial,
                    edges: Vec::new(),
                },
                body: vec![BasicBlock {
                    stmts: proc_stmts,
                    term: Terminator::Return,
                }],
                entry: 0,
                suspend: suspend0(),
            }],
            cont_assigns: Vec::new(),
            funcs: self.funcs,
            exprs: self.exprs,
            stmts: self.stmts,
            blocks: self.blocks,
            consts: self.consts,
        };
        let opts = SimOpts {
            func_table: self.func_table,
            ..SimOpts::default()
        };
        (ir, opts)
    }
}

/// Run on a 256 MiB-stack worker thread so the depth CAP, not a native stack
/// overflow, is the guard for the deep/runaway recursion corpus. The closure
/// owns everything it builds (nothing crosses the thread boundary).
fn on_big_stack<R: Send + 'static>(f: impl FnOnce() -> R + Send + 'static) -> R {
    std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(f)
        .expect("spawn big-stack worker")
        .join()
        .expect("big-stack worker panicked")
}

fn lines_trimmed(out: &str) -> Vec<String> {
    out.lines().map(|l| l.trim().to_string()).collect()
}

#[derive(Default)]
struct DiagSink(std::cell::RefCell<Vec<String>>);
impl LogSink for DiagSink {
    fn emit(&self, e: LogEvent) {
        if let LogEvent::Diagnostic(d) = e {
            self.0
                .borrow_mut()
                .push(format!("{}: {}", d.code.code_num(), d.message));
        }
    }
}

// ── Increment-2 engine tests ────────────────────────────────────────────────

#[test]
fn recursive_automatic_function_factorial() {
    // fact(0)=1, fact(1)=1, fact(5)=120, fact(10)=3628800 (value-pinned).
    let mut b = B::default();
    let fact = b.lower_recursive(true, false, 32, true);
    let c5 = b.k(5);
    let a5 = b.call(fact, vec![c5]);
    let c0 = b.k(0);
    let a0 = b.call(fact, vec![c0]);
    let c1 = b.k(1);
    let a1 = b.call(fact, vec![c1]);
    let c10 = b.k(10);
    let a10 = b.call(fact, vec![c10]);
    let s0 = b.display(a5);
    let s1 = b.display(a0);
    let s2 = b.display(a1);
    let s3 = b.display(a10);
    let s4 = b.finish();
    let (ir, opts) = b.build(vec![s0, s1, s2, s3, s4]);
    let (res, out) = simulate_capture(&ir, opts);
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(lines_trimmed(&out), vec!["120", "1", "1", "3628800"]);
}

#[test]
fn static_recursion_shared_slot_corruption_probe() {
    // The lifetime discriminator (`f = f(n-1) + acc`, call+acc order):
    //   automatic probe(3) = 30 + (20 + 10) = 60  (each frame keeps its acc)
    //   static    probe(3) = 30                   (shared acc clobbered to 10)
    // Both value-pinned UNCONDITIONALLY — a wrong static impl emitting 60 must
    // fail. (Oracle: iverilog live; real-pipeline diff at Increment 5.)
    let mut b = B::default();
    let pa = b.lower_recursive(true, true, 32, true); // probe_auto  (func 0)
    let ps = b.lower_recursive(false, true, 32, true); // probe_static (func 1)
    let c3a = b.k(3);
    let va = b.call(pa, vec![c3a]);
    let c3s = b.k(3);
    let vs = b.call(ps, vec![c3s]);
    let s0 = b.display(va);
    let s1 = b.display(vs);
    let s2 = b.finish();
    let (ir, opts) = b.build(vec![s0, s1, s2]);
    let (res, out) = simulate_capture(&ir, opts);
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(
        lines_trimmed(&out),
        vec!["60", "30"],
        "automatic=60 (per-frame acc), static=30 (shared-slot corruption)"
    );
}

#[test]
fn static_persistence_across_separate_calls() {
    // A static local is X-init on the FIRST call and PERSISTS its residue across
    // subsequent top-level calls (do NOT zero on entry). cnt is recursive so the
    // slab is well-exercised; two independent calls must each compute correctly
    // (the slab is reset by the deepest leaf each time, not stale-leaking a wrong
    // result): cnt(4)=10, cnt(3)=6.
    let mut b = B::default();
    let cnt = b.lower_counter(false);
    let c4 = b.k(4);
    let v4 = b.call(cnt, vec![c4]);
    let c3 = b.k(3);
    let v3 = b.call(cnt, vec![c3]);
    let s0 = b.display(v4);
    let s1 = b.display(v3);
    let s2 = b.finish();
    let (ir, opts) = b.build(vec![s0, s1, s2]);
    let (res, out) = simulate_capture(&ir, opts);
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(lines_trimmed(&out), vec!["10", "6"]);
}

#[test]
fn non_default_return_width_truncates() {
    // `function [15:0] f` — UNSIGNED 16-bit return. fact(8)=40320 fits exactly
    // (<65536); fact(9)=9*40320=362880 truncates to 16 bits = 362880 & 0xFFFF =
    // 35200. Pins the declared-return-width path (the engine debug_asserts the
    // return-var net width/sign == ret_width/ret_signed).
    let mut b = B::default();
    let fact = b.lower_recursive(true, false, 16, false);
    let c8 = b.k(8);
    let a8 = b.call(fact, vec![c8]);
    let c9 = b.k(9);
    let a9 = b.call(fact, vec![c9]);
    let s0 = b.display(a8);
    let s1 = b.display(a9);
    let s2 = b.finish();
    let (ir, opts) = b.build(vec![s0, s1, s2]);
    let (res, out) = simulate_capture(&ir, opts);
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(lines_trimmed(&out), vec!["40320", "35200"]);
}

#[test]
fn legal_deep_recursion_does_not_falsely_fatal() {
    // cnt(2000) = 2000*2001/2 = 2001000 — a depth iverilog completes cleanly;
    // the cap (65536) must NOT fire. Runs on the big-stack worker.
    let out = on_big_stack(|| {
        let mut b = B::default();
        let cnt = b.lower_counter(true);
        let c = b.k(2000);
        let v = b.call(cnt, vec![c]);
        let s0 = b.display(v);
        let s1 = b.finish();
        let (ir, opts) = b.build(vec![s0, s1]);
        let (res, out) = simulate_capture(&ir, opts);
        (res.finish_reason, out)
    });
    assert_eq!(out.0, FinishReason::Finish);
    assert_eq!(lines_trimmed(&out.1), vec!["2001000"]);
}

#[test]
fn runaway_recursion_hits_depth_cap_fatal() {
    // `bad(n) = bad(n-1) + 1` with no base case → unbounded recursion. The cap
    // latches call_fatal → FinishReason::Error / ExitClass::Fatal, NO host
    // SIGSEGV (big-stack worker). Non-differential (iverilog SIGSEGVs exit 139).
    let (reason, class, fatal_diags) = on_big_stack(|| {
        let mut b = B::default();
        // hand-lower an UNCONDITIONAL recursion: f(n) = f(n-1) + 1
        let func = b.funcs.len() as u32;
        let base = b.nets.len() as u32;
        let n_net = b.net(int_net(32, true));
        let _ret = b.net(int_net(32, true));
        let ret_slot_net = base + 1;
        let n = b.sig(n_net);
        let one = b.k(1);
        let nm1 = b.bin(BinOp::Sub, n, one);
        let rec = b.call(func, vec![nm1]);
        let one2 = b.k(1);
        let body = b.bin(BinOp::Add, rec, one2);
        let s = b.assign(ret_slot_net, body);
        let entry = b.block(vec![s], Terminator::Return);
        b.funcs.push(FuncDef {
            entry,
            n_params: 1,
            locals_len: 2,
            is_task: false,
        });
        b.func_table.push(FuncMeta {
            base_net: base,
            n_params: 1,
            return_slot: 1,
            locals_len: 2,
            is_automatic: true,
            ret_width: 32,
            ret_signed: true,
            auto_override: 0,
        });
        let c = b.k(5);
        let v = b.call(func, vec![c]);
        // NO trailing $finish: a $finish in the same body AFTER the runaway would
        // mask the latched fatal (Step::Finish wins). The process completes (the
        // display prints X), then the scheduler's post-batch check surfaces the
        // fatal as Error.
        let s0 = b.display(v);
        let (ir, opts) = b.build(vec![s0]);
        let sink = DiagSink::default();
        let res = simulate(&ir, &sink, opts);
        let fatals: Vec<String> = sink.0.into_inner();
        (res.finish_reason, res.exit_class, fatals)
    });
    assert_eq!(reason, FinishReason::Error, "runaway must end in Error");
    assert!(
        matches!(class, ExitClass::Fatal),
        "runaway exit class must be Fatal, got {class:?}"
    );
    assert!(
        fatal_diags.iter().any(|d| d.contains("recursion exceeded")),
        "expected a depth-limit fatal diagnostic, got {fatal_diags:?}"
    );
}

#[test]
fn cont_assign_originated_runaway_terminates() {
    // CRITICAL fatal-surfacing fix: the runaway originates in a CONT-ASSIGN RHS
    // (`assign y = bad(x);`), not a process body — the scheduler must catch
    // call_fatal at the settle seam and TERMINATE (no hang, Error). A
    // process-body-only check would miss this.
    let reason = on_big_stack(|| {
        // bad(n) = bad(n-1) + 1 (unbounded), driven by a cont-assign.
        let mut b = B::default();
        let func = b.funcs.len() as u32;
        let fbase = b.nets.len() as u32;
        let fn_net = b.net(int_net(32, true));
        let _fret = b.net(int_net(32, true));
        let ret_slot_net = fbase + 1;
        let n = b.sig(fn_net);
        let one = b.k(1);
        let nm1 = b.bin(BinOp::Sub, n, one);
        let rec = b.call(func, vec![nm1]);
        let one2 = b.k(1);
        let body = b.bin(BinOp::Add, rec, one2);
        let s = b.assign(ret_slot_net, body);
        let entry = b.block(vec![s], Terminator::Return);
        b.funcs.push(FuncDef {
            entry,
            n_params: 1,
            locals_len: 2,
            is_task: false,
        });
        b.func_table.push(FuncMeta {
            base_net: fbase,
            n_params: 1,
            return_slot: 1,
            locals_len: 2,
            is_automatic: true,
            ret_width: 32,
            ret_signed: true,
            auto_override: 0,
        });
        // module nets: x (driver, =5) and y (cont-assign target).
        let x = b.net(int_net(32, true));
        let y = b.net(int_net(32, true));
        let xr = b.sig(x);
        let cy = b.call(func, vec![xr]); // bad(x)
                                         // initial: x = 5; (so the cont-assign RHS has a defined input)
        let five = b.k(5);
        let sx = b.assign(x, five);
        // assemble manually to add a cont_assign.
        let mut ir = SimIr {
            instances: vec![Instance {
                parent: None,
                module: 0,
                first_net: 0,
                net_count: b.nets.len() as u32,
            }],
            nets: b.nets,
            processes: vec![Process {
                sensitivity: Sensitivity {
                    kind: SensKind::Initial,
                    edges: Vec::new(),
                },
                body: vec![BasicBlock {
                    stmts: vec![sx],
                    term: Terminator::Return,
                }],
                entry: 0,
                suspend: suspend0(),
            }],
            cont_assigns: Vec::new(),
            funcs: b.funcs,
            exprs: b.exprs,
            stmts: b.stmts,
            blocks: b.blocks,
            consts: b.consts,
        };
        ir.cont_assigns.push(sim_ir::ContAssign {
            lhs: Lvalue {
                chunks: vec![LvalChunk {
                    net: y,
                    word: None,
                    offset: None,
                    width: None,
                    kind: SelKind::Bit,
                }],
            },
            rhs: cy,
            delay: None,
        });
        let opts = SimOpts {
            func_table: b.func_table,
            ..SimOpts::default()
        };
        let sink = DiagSink::default();
        simulate(&ir, &sink, opts).finish_reason
    });
    assert_eq!(
        reason,
        FinishReason::Error,
        "cont-assign-originated runaway must terminate in Error (no hang)"
    );
}

#[test]
fn frame_local_nets_have_no_vcd_surface() {
    // CRITICAL-FIX-2: frame-local Reg/Integer nets are REAL ir.nets entries but
    // must NEVER be declared/dumped to the VCD. Build a factorial design with a
    // module reg `r` (the ONLY VCD-visible net), dump it, and assert the VCD has
    // exactly ONE $var (for `r`) and none for the 2 frame nets.
    let dir = std::env::temp_dir();
    let path = dir.join(format!("vita_frame_vcd_{}.vcd", std::process::id()));
    let mut b = B::default();
    let fact = b.lower_recursive(true, false, 32, true);
    let r = b.net(int_net(32, true)); // module reg — the only VCD-visible net
    let c5 = b.k(5);
    let a5 = b.call(fact, vec![c5]);
    let s_dumpfile = {
        // $dumpfile is implicit via vcd_path_override; just $dumpvars.
        b.stmts.push(Stmt::SysTask {
            which: SysTaskId::DumpVars,
            fmt: None,
            args: vec![],
        });
        b.stmts.len() as u32 - 1
    };
    let s_r = b.assign(r, a5);
    let s_fin = b.finish();
    let (ir, mut opts) = b.build(vec![s_dumpfile, s_r, s_fin]);
    opts.vcd_path_override = Some(path.to_string_lossy().to_string());
    let (res, _out) = simulate_capture(&ir, opts);
    assert_eq!(res.finish_reason, FinishReason::Finish);
    let vcd = std::fs::read_to_string(&path).expect("VCD written");
    let _ = std::fs::remove_file(&path);
    let nvar = vcd.matches("$var").count();
    assert_eq!(
        nvar, 1,
        "exactly one $var (module reg `r`); frame nets must NOT appear:\n{vcd}"
    );
}

// ── B2: recursive/automatic TASKS (hand-built engine teeth) ──────────────────

fn whole_lval(net: u32) -> Lvalue {
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

#[test]
fn recursive_automatic_task_with_output_formal() {
    // task automatic factt(input integer n, output integer r);
    //   integer t;
    //   if (n <= 1) r = 1;
    //   else begin factt(n-1, t); r = n * t; end
    // endtask
    // initial begin factt(5, res); $display(res); end   → res = 120
    use sim_engine::{TaskCallFunc, TaskCallInfo, TaskCallProc};
    let mut b = B::default();
    let n = b.net(int_net(32, true)); // 0: input formal
    let r = b.net(int_net(32, true)); // 1: output formal
    let t = b.net(int_net(32, true)); // 2: local
    let res = b.net(int_net(32, true)); // 3: module net (caller output target)

    // body exprs/stmts
    let e_n = b.sig(n);
    let one = b.k(1);
    let cond = b.bin(BinOp::Le, e_n, one); // n <= 1
    let one_r = b.k(1);
    let s_then = b.assign(r, one_r); // r = 1
    let e_n2 = b.sig(n);
    let one2 = b.k(1);
    let nm1 = b.bin(BinOp::Sub, e_n2, one2); // n - 1  (nested-call arg)
    let e_n3 = b.sig(n);
    let e_t = b.sig(t);
    let mul = b.bin(BinOp::Mul, e_n3, e_t); // n * t
    let s_mul = b.assign(r, mul); // r = n * t

    // func arena blocks (this is the first func → indices start at 0):
    //   0 entry: Branch(cond → 1 / 2)   1 then: [r=1] Return
    //   2 else:  Call(target=0, ret=3)  3 after: [r=n*t] Return
    let _entry = b.block(
        vec![],
        Terminator::Branch {
            cond,
            then_bb: 1,
            else_bb: 2,
        },
    );
    let _then = b.block(vec![s_then], Terminator::Return);
    let _else = b.block(
        vec![],
        Terminator::Call {
            target: 0,
            ret_bb: 3,
        },
    );
    let _after = b.block(vec![s_mul], Terminator::Return);

    b.funcs.push(FuncDef {
        entry: 0,
        n_params: 2,
        locals_len: 3,
        is_task: true,
    });
    b.func_table.push(FuncMeta {
        base_net: 0,
        n_params: 2,
        return_slot: 0, // unused for tasks (no func-named return var)
        locals_len: 3,
        is_automatic: true,
        ret_width: 32,
        ret_signed: true,
        auto_override: 0,
    });

    // process: P0 Call(factt, ret=P1); P1 [$display(res)] Return
    let c5 = b.k(5);
    let disp_arg = b.sig(res);
    let s_disp = b.display(disp_arg);

    // nested-call site (func block 2): factt(n-1, t) → out into frame-local t
    let mut tcf = TaskCallFunc::new();
    tcf.insert(
        2,
        TaskCallInfo {
            callee: 0,
            in_binds: vec![(0, nm1)],
            out_binds: vec![(1, whole_lval(t))],
        },
    );
    // top-level call site (proc 0, process block 0): factt(5, res) → out into res
    let mut tcp = TaskCallProc::new();
    tcp.insert(
        (0, 0),
        TaskCallInfo {
            callee: 0,
            in_binds: vec![(0, c5)],
            out_binds: vec![(1, whole_lval(res))],
        },
    );

    let ir = SimIr {
        instances: vec![Instance {
            parent: None,
            module: 0,
            first_net: 0,
            net_count: b.nets.len() as u32,
        }],
        nets: b.nets,
        processes: vec![Process {
            sensitivity: Sensitivity {
                kind: SensKind::Initial,
                edges: Vec::new(),
            },
            body: vec![
                BasicBlock {
                    stmts: vec![],
                    term: Terminator::Call {
                        target: 0,
                        ret_bb: 1,
                    },
                },
                BasicBlock {
                    stmts: vec![s_disp],
                    term: Terminator::Return,
                },
            ],
            entry: 0,
            suspend: suspend0(),
        }],
        cont_assigns: Vec::new(),
        funcs: b.funcs,
        exprs: b.exprs,
        stmts: b.stmts,
        blocks: b.blocks,
        consts: b.consts,
    };
    let opts = SimOpts {
        func_table: b.func_table,
        task_calls_proc: tcp,
        task_calls_func: tcf,
        ..SimOpts::default()
    };
    let (res_r, out) = simulate_capture(&ir, opts);
    assert_eq!(res_r.finish_reason, FinishReason::Quiescent);
    assert_eq!(lines_trimmed(&out), vec!["120"], "factt(5) writes res=120");
}

// ── Increment 5: REAL-pipeline differential (vita design.sv vs iverilog) ─────
// These drive the FULL elaborate front-end (Increment 4), which still loud-
// rejects automatic/recursive functions. They go green once that lands.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn vita_out(src: &str) -> String {
    let (toks, le) = hdl_lexer::lex(src);
    assert!(le.is_empty(), "lex errors: {le:?}");
    let (su, pe) = hdl_parser::parse(&toks, src);
    assert!(pe.is_empty(), "parse errors: {pe:?}");
    let sink = DiagSink::default();
    let (ir, sc) = elaborate::elaborate_with_timescale(
        &su.expect("source unit"),
        &sink,
        &std::collections::BTreeMap::new(),
        -9,
    );
    let hard: Vec<String> = sink
        .0
        .borrow()
        .iter()
        .filter(|d| d.contains("Error") || d.contains("Fatal"))
        .cloned()
        .collect();
    assert!(hard.is_empty(), "elaborate errors: {hard:?}");
    let opts = SimOpts {
        fork_modes: sc.fork_modes,
        net_names: sc.net_names,
        proc_multipliers: sc.proc_multipliers,
        severities: sc.severities,
        assign_ranks: sc.assign_ranks,
        radixes: sc.radixes,
        func_table: sc.func_table,
        task_calls_proc: sc.task_calls_proc,
        task_calls_func: sc.task_calls_func,
        ..SimOpts::default()
    };
    let (_res, out) = simulate_capture(&ir.expect("ir"), opts);
    out
}

fn on_path(tool: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {tool}"))
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn iverilog_out(src: &str) -> Option<String> {
    if !on_path("iverilog") || !on_path("vvp") {
        return None;
    }
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir();
    let sv = dir.join(format!("vita_frame_{}_{n}.sv", std::process::id()));
    let vvp = dir.join(format!("vita_frame_{}_{n}.vvp", std::process::id()));
    std::fs::write(&sv, src).expect("write sv");
    let compile = Command::new("iverilog")
        .args(["-g2012", "-o"])
        .arg(&vvp)
        .arg(&sv)
        .output()
        .expect("run iverilog");
    assert!(
        compile.status.success(),
        "iverilog compile failed: {}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let run = Command::new("vvp").arg(&vvp).output().expect("run vvp");
    let _ = std::fs::remove_file(&sv);
    let _ = std::fs::remove_file(&vvp);
    let s = String::from_utf8_lossy(&run.stdout);
    Some(
        s.lines()
            .filter(|l| !l.contains("$finish called"))
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

fn check(src: &str, expect: &str) {
    let v = vita_out(src);
    assert_eq!(v.trim_end(), expect.trim_end(), "vita output mismatch");
    if let Some(iv) = iverilog_out(src) {
        assert_eq!(
            v.trim_end(),
            iv.trim_end(),
            "vita vs iverilog differ\nvita:\n{v}\niverilog:\n{iv}"
        );
    }
}

/// Elaborate `src` and report whether it was LOUD-REJECTED (no IR produced). Used
/// for the deliberate B1 frame-body cuts (iverilog ACCEPTS these, so they are
/// vita-side rejects, NOT differentials).
fn elaborate_rejects(src: &str) -> bool {
    let (toks, _) = hdl_lexer::lex(src);
    let (su, _) = hdl_parser::parse(&toks, src);
    let sink = DiagSink::default();
    let (ir, _sc) = elaborate::elaborate_with_timescale(
        &su.expect("source unit"),
        &sink,
        &std::collections::BTreeMap::new(),
        -9,
    );
    ir.is_none()
}

#[test]
fn frame_body_loud_rejects_unsupported_constructs() {
    // A $systask in a frame body: the engine's eval_call runs only BlockingAssign,
    // so a $display would be SILENTLY DROPPED — loud-reject instead. (iverilog
    // accepts it: a deliberate B1 cut.)
    assert!(
        elaborate_rejects(
            r#"
module tb;
  function automatic integer noisy(input integer n);
    begin $display("x"); noisy = n; end
  endfunction
  initial $display("%0d", noisy(3));
endmodule
"#
        ),
        "a $systask in a frame function body must be loud-rejected"
    );
    // Writing a MODULE net from a frame function: the &self eval path cannot write
    // the flat store — loud-reject, never a silent mis-route.
    assert!(
        elaborate_rejects(
            r#"
module tb;
  integer g;
  function automatic integer bad(input integer n);
    begin g = n; bad = n; end
  endfunction
  initial $display("%0d", bad(3));
endmodule
"#
        ),
        "a module-net write from a frame function body must be loud-rejected"
    );
    // A non-blocking assign inside a frame body — also outside the subset.
    assert!(
        elaborate_rejects(
            r#"
module tb;
  function automatic integer nb(input integer n);
    nb <= n;
  endfunction
  initial $display("%0d", nb(3));
endmodule
"#
        ),
        "a nonblocking assign in a frame function body must be loud-rejected"
    );
    // B3: `disable <funcname>` (self-disabling a FUNCTION) is illegal — iverilog
    // rejects it ("cannot disable functions"); a TASK self-disable is the legal
    // form. Only named-BLOCK disables (break/continue) are allowed in a function.
    assert!(
        elaborate_rejects(
            r#"
module tb;
  function automatic integer f(input integer n);
    begin f = 0; if (n < 0) disable f; f = n; end
  endfunction
  initial $display("%0d", f(3));
endmodule
"#
        ),
        "self-disabling a frame FUNCTION must be loud-rejected (iverilog: cannot disable functions)"
    );
}

#[test]
fn e2e_recursive_automatic_function_factorial() {
    let src = r#"
module tb;
  function automatic integer fact(input integer n);
    if (n <= 1) fact = 1;
    else fact = n * fact(n - 1);
  endfunction
  initial begin
    $display("fact(5)=%0d", fact(5));
    $display("fact(0)=%0d", fact(0));
    $display("fact(1)=%0d", fact(1));
    $display("fact(10)=%0d", fact(10));
  end
endmodule
"#;
    check(src, "fact(5)=120\nfact(0)=1\nfact(1)=1\nfact(10)=3628800");
}

#[test]
fn e2e_recursive_automatic_task() {
    // B2: a recursive automatic TASK with an output formal, through the REAL
    // pipeline + iverilog. factt(n,r): r=n! computed into the output. Each
    // automatic frame keeps its own local `t`.
    let src = r#"
module tb;
  task automatic factt(input integer n, output integer r);
    integer t;
    if (n <= 1) r = 1;
    else begin factt(n - 1, t); r = n * t; end
  endtask
  integer res;
  initial begin
    factt(5, res); $display("%0d", res);
    factt(0, res); $display("%0d", res);
    factt(7, res); $display("%0d", res);
  end
endmodule
"#;
    check(src, "120\n1\n5040");
}

#[test]
fn e2e_b4_automatic_lifetime_override() {
    // B4 (hand-IEEE — iverilog rejects "overriding the default variable
    // lifetime"): an `automatic` local in a DEFAULT-STATIC recursive function gets
    // fresh-per-call storage. The probe `f = f(n-1) + acc` with `acc = n*10`:
    //   `automatic integer acc` → each frame keeps its own acc → probe(3) = 60
    //   plain (default-static) `integer acc` → shared/clobbered → probe(3) = 30
    // The first is a MIXED-lifetime frame (automatic acc, static n/return).
    let src = r#"
module tb;
  function integer probe_a(input integer n);
    automatic integer acc;
    begin
      acc = n * 10;
      if (n > 1) probe_a = probe_a(n - 1) + acc;
      else probe_a = acc;
    end
  endfunction
  function integer probe_s(input integer n);
    integer acc;
    begin
      acc = n * 10;
      if (n > 1) probe_s = probe_s(n - 1) + acc;
      else probe_s = acc;
    end
  endfunction
  initial begin
    $display("%0d", probe_a(3));
    $display("%0d", probe_s(3));
  end
endmodule
"#;
    // value-pin only (no iverilog oracle — it rejects the override).
    let out = vita_out(src);
    assert_eq!(
        lines_trimmed(&out),
        vec!["60", "30"],
        "automatic-local acc is per-frame (60); default-static acc is shared (30)"
    );
}

#[test]
fn e2e_frame_function_disable_break() {
    // B3: the `disable <named block>` break/continue idiom inside a frame
    // function. `disable scan` ends the current loop-body block (= continue), so
    // the LAST set bit wins (matches iverilog's block-disable semantics).
    let src = r#"
module tb;
  function automatic integer lastset(input integer mask);
    integer i;
    begin
      lastset = -1;
      for (i = 0; i < 8; i = i + 1) begin: scan
        if (mask[i]) begin lastset = i; disable scan; end
      end
    end
  endfunction
  initial begin
    $display("%0d", lastset(8'b00101000));
    $display("%0d", lastset(8'b00000000));
    $display("%0d", lastset(8'b10000001));
  end
endmodule
"#;
    check(src, "5\n-1\n7");
}

#[test]
fn e2e_task_self_disable_early_return() {
    // B3: `disable <taskname>` inside a frame task is a self-disable = early
    // return (the single-frame unwind). clampt(-3) returns with r still 0.
    let src = r#"
module tb;
  task automatic clampt(input integer n, output integer r);
    begin
      r = 0;
      if (n < 0) disable clampt;
      r = n * 2;
    end
  endtask
  integer r;
  initial begin
    clampt(5, r);  $display("%0d", r);
    clampt(-3, r); $display("%0d", r);
    clampt(0, r);  $display("%0d", r);
  end
endmodule
"#;
    check(src, "10\n0\n0");
}

#[test]
fn e2e_static_vs_automatic_corruption() {
    // The lifetime discriminator through the REAL pipeline + iverilog: automatic
    // keeps a per-frame `acc` (probe(3)=60); static shares one slot, clobbered to
    // the deepest frame's 10 (probe(3)=30).
    let src = r#"
module tb;
  function automatic integer probe_auto(input integer n);
    integer acc;
    begin
      acc = n * 10;
      if (n > 1) probe_auto = probe_auto(n - 1) + acc;
      else probe_auto = acc;
    end
  endfunction
  function integer probe_static(input integer n);
    integer acc;
    begin
      acc = n * 10;
      if (n > 1) probe_static = probe_static(n - 1) + acc;
      else probe_static = acc;
    end
  endfunction
  initial begin
    $display("auto=%0d", probe_auto(3));
    $display("static=%0d", probe_static(3));
  end
endmodule
"#;
    check(src, "auto=60\nstatic=30");
}

#[test]
fn e2e_mutual_recursion() {
    // Mutual recursion: both is_even/is_odd are reserved BEFORE either body
    // lowers, so the cross-call resolves. is_even(4)=1, is_odd(4)=0.
    let src = r#"
module tb;
  function automatic integer is_even(input integer n);
    if (n == 0) is_even = 1;
    else is_even = is_odd(n - 1);
  endfunction
  function automatic integer is_odd(input integer n);
    if (n == 0) is_odd = 0;
    else is_odd = is_even(n - 1);
  endfunction
  initial begin
    $display("even4=%0d", is_even(4));
    $display("odd4=%0d", is_odd(4));
    $display("even7=%0d", is_even(7));
  end
endmodule
"#;
    check(src, "even4=1\nodd4=0\neven7=0");
}

#[test]
fn e2e_control_flow_static_function() {
    // A non-recursive, non-automatic function with control flow — framed via the
    // `body_needs_frame` rule (the inline path can't fold an if/else). Static
    // storage is harmless without recursion.
    let src = r#"
module tb;
  function integer clamp(input integer x);
    if (x > 100) clamp = 100;
    else if (x < 0) clamp = 0;
    else clamp = x;
  endfunction
  initial begin
    $display("%0d", clamp(150));
    $display("%0d", clamp(-5));
    $display("%0d", clamp(42));
  end
endmodule
"#;
    check(src, "100\n0\n42");
}

#[test]
fn e2e_non_default_return_width() {
    // `function [15:0]` — an UNSIGNED 16-bit return truncates: fact16(8)=40320
    // fits, fact16(9)=9*40320 wraps to 16 bits. The exact wrap is iverilog's
    // (the differential pins it).
    let src = r#"
module tb;
  function automatic [15:0] fact16(input integer n);
    if (n <= 1) fact16 = 1;
    else fact16 = n * fact16(n - 1);
  endfunction
  initial begin
    $display("%0d", fact16(8));
    $display("%0d", fact16(9));
  end
endmodule
"#;
    check(src, "40320\n35200");
}
