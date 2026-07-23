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

use sim_engine::{simulate, simulate_capture, ExitClass, FinishReason, FuncMeta, SimOpts};
use sim_ir::{
    BasicBlock, BinOp, FuncDef, Instance, LvalChunk, Lvalue, Process, SelKind, SensKind,
    Sensitivity, SimIr, Stmt, SysTaskId, Terminator,
};

#[path = "frame_call_util/mod.rs"]
mod util;
#[allow(unused_imports)]
use util::*;

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
            str_params: 0,
            has_hier_call: false,
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
            str_params: 0,
            has_hier_call: false,
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
        str_params: 0,
        has_hier_call: false,
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
