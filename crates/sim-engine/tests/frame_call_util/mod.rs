#![allow(dead_code)]
#![allow(unused_imports)]
// shared helpers for the split frame_call integration tests (mechanical move)

use diag::{LogEvent, LogSink};
use sim_engine::{simulate, simulate_capture, ExitClass, FinishReason, FuncMeta, SimOpts};
use sim_ir::{
    BasicBlock, BinOp, BitPacked, ConstRepr, ConstVal, Expr, FuncDef, Instance, JoinState,
    LvalChunk, Lvalue, NetKind, NetVar, PortDir, ProcFlags, Process, RegionTag, SelKind, SensKind,
    Sensitivity, SimIr, Stmt, SuspendState, SysTaskId, Terminator, WakeCond, WakeKey,
};

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
// ── construction helpers ─────────────────────────────────────────────────────

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

/// An `integer`/`reg [w-1:0]` net (`Internal`, array_len 1, X-init).
pub fn int_net(width: u32, signed: bool) -> NetVar {
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
pub struct B {
    pub nets: Vec<NetVar>,
    pub consts: Vec<ConstVal>,
    pub exprs: Vec<Expr>,
    pub stmts: Vec<Stmt>,
    pub blocks: Vec<BasicBlock>,
    pub funcs: Vec<FuncDef>,
    pub func_table: Vec<FuncMeta>,
}

impl B {
    pub fn net(&mut self, nv: NetVar) -> u32 {
        self.nets.push(nv);
        self.nets.len() as u32 - 1
    }
    /// A `Const` expr of value `v` (32-bit signed numeric). Returns its ExprId.
    pub fn k(&mut self, v: i64) -> u32 {
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
    pub fn sig(&mut self, net: u32) -> u32 {
        self.expr(Expr::Signal { net, word: None })
    }
    pub fn bin(&mut self, op: BinOp, lhs: u32, rhs: u32) -> u32 {
        self.expr(Expr::Binary { op, lhs, rhs })
    }
    pub fn call(&mut self, func: u32, args: Vec<u32>) -> u32 {
        self.expr(Expr::Call { func, args })
    }
    pub fn expr(&mut self, e: Expr) -> u32 {
        self.exprs.push(e);
        self.exprs.len() as u32 - 1
    }
    /// `net = rhs` whole-net blocking assign. Returns its StmtId.
    pub fn assign(&mut self, net: u32, rhs: u32) -> u32 {
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
    pub fn block(&mut self, stmts: Vec<u32>, term: Terminator) -> u32 {
        self.blocks.push(BasicBlock { stmts, term });
        self.blocks.len() as u32 - 1
    }
    pub fn display(&mut self, arg: u32) -> u32 {
        self.stmts.push(Stmt::SysTask {
            which: SysTaskId::Display,
            fmt: None,
            args: vec![arg],
        });
        self.stmts.len() as u32 - 1
    }
    pub fn finish(&mut self) -> u32 {
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
    pub fn lower_recursive(
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
            str_params: 0,
            has_hier_call: false,
        });
        func
    }

    /// Lower `cnt(n) = (n<=0)?0 : n + cnt(n-1)` (a deep-recursion counter:
    /// `cnt(k)=k*(k+1)/2`). Single func, 2 frame nets, 3 BBs. Returns FuncId.
    pub fn lower_counter(&mut self, automatic: bool) -> u32 {
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
            str_params: 0,
            has_hier_call: false,
        });
        func
    }

    /// Finish the build: wrap `proc_stmts` in one `initial` process and assemble
    /// the `SimIr` + matching `SimOpts.func_table`.
    pub fn build(self, proc_stmts: Vec<u32>) -> (SimIr, SimOpts) {
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
pub fn on_big_stack<R: Send + 'static>(f: impl FnOnce() -> R + Send + 'static) -> R {
    std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(f)
        .expect("spawn big-stack worker")
        .join()
        .expect("big-stack worker panicked")
}

pub fn lines_trimmed(out: &str) -> Vec<String> {
    out.lines().map(|l| l.trim().to_string()).collect()
}

#[derive(Default)]
pub struct DiagSink(pub std::cell::RefCell<Vec<String>>);
impl LogSink for DiagSink {
    fn emit(&self, e: LogEvent) {
        if let LogEvent::Diagnostic(d) = e {
            self.0
                .borrow_mut()
                .push(format!("{}: {}", d.code.code_num(), d.message));
        }
    }
}

// ── B2: recursive/automatic TASKS (hand-built engine teeth) ──────────────────

pub fn whole_lval(net: u32) -> Lvalue {
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

pub static NEXT: AtomicU64 = AtomicU64::new(0);

pub fn vita_out(src: &str) -> String {
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

pub fn on_path(tool: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {tool}"))
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn iverilog_out(src: &str) -> Option<String> {
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

pub fn check(src: &str, expect: &str) {
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
pub fn elaborate_rejects(src: &str) -> bool {
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
