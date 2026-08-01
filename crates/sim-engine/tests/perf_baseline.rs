//! [C3] Perf BASELINE harness — DATA, not a gate (plan/review: "Add a perf harness
//! (DATA, not a gate yet)"). `#[ignore]`d so it NEVER runs in the normal suite and can
//! never fail CI on timing variance; run it explicitly:
//!
//! ```text
//! cargo test -p sim-engine --test perf_baseline -- --ignored --nocapture
//! ```
//!
//! It times a codegen-able-heavy design on BOTH backends. At C2 the bytecode VM
//! DELEGATES expression eval to the SAME kernel the interpreter uses, so it is expected
//! AT-OR-BELOW the interpreter (a compile pass + op-dispatch loop on top of identical
//! eval cost) — this run records that honest structural-milestone baseline and pins the
//! interpreter time that C3 (native value registers, removing the `Value` heap-alloc and
//! eval tree-walk) must beat. It is intentionally NOT an assertion on the ratio.

mod common;

use std::time::Instant;

use common::build;
use diag::{LogEvent, LogSink};
use sim_engine::{simulate, Backend, FinishReason, SimOpts};

/// Discards all output so wall-time reflects the engine, not the sink.
struct NullSink;
impl LogSink for NullSink {
    fn emit(&self, _e: LogEvent) {}
}

/// Best-of-`reps` wall-time (ns) of a full `simulate` on `backend` (min = least noise).
fn time_backend(ir: &sim_ir::SimIr, backend: Backend, reps: u32) -> u128 {
    let sink = NullSink;
    let mut best = u128::MAX;
    for _ in 0..reps {
        let opts = SimOpts {
            backend,
            ..SimOpts::default()
        };
        let t = Instant::now();
        let res = simulate(ir, &sink, opts);
        best = best.min(t.elapsed().as_nanos());
        assert_eq!(
            res.finish_reason,
            FinishReason::Finish,
            "perf design must $finish"
        );
    }
    best
}

/// A datapath dominated by ONE codegen-able `always @(posedge clk)` body running many
/// thousands of cycles, each doing five 64-bit nonblocking assigns with arithmetic /
/// shifts / xor — heavy on both measured hot spots (eval dispatch + `Value` heap-alloc).
/// The clock-driving `initial` is interpreted in BOTH backends (common overhead), so the
/// interp-vs-VM delta isolates the always-body path.
const CODEGEN_HEAVY: &str = "module top;\n\
  reg clk;\n\
  reg [63:0] a, b, c, d, e;\n\
  integer k;\n\
  always @(posedge clk) begin\n\
    a <= a + 64'd3;\n\
    b <= b ^ a;\n\
    c <= c + b;\n\
    d <= (d << 1) | (d >> 63);\n\
    e <= e + d + a;\n\
  end\n\
  initial begin\n\
    clk = 0; a = 1; b = 2; c = 3; d = 4; e = 5;\n\
    for (k = 0; k < 20000; k = k + 1) begin #1 clk = 1; #1 clk = 0; end\n\
    $finish;\n\
  end\n\
endmodule";

/// EVAL-dominated: one codegen-able `always @(posedge clk)` body with a heavy inner
/// `for` loop (a Branch back-edge — all inside ONE activation, no suspension), driven by
/// only a few hundred clock edges. Each clock runs thousands of 64-bit arithmetic /
/// shift / xor evals, so wall-time is dominated by the eval + `Value`-alloc path (NOT the
/// scheduler/clock churn that swamps `CODEGEN_HEAVY`) — this is the case the `Value`
/// inline-storage change (and later native eval) is meant to move.
const EVAL_HEAVY: &str = "module top;\n\
  reg clk;\n\
  reg [63:0] acc;\n\
  integer i;\n\
  integer j;\n\
  always @(posedge clk) begin\n\
    for (i = 0; i < 3000; i = i + 1) begin\n\
      acc = acc + (acc << 1) + 64'd7;\n\
      acc = acc ^ (acc >> 3);\n\
    end\n\
  end\n\
  initial begin\n\
    clk = 0; acc = 1;\n\
    for (j = 0; j < 200; j = j + 1) begin #1 clk = 1; #1 clk = 0; end\n\
    $finish;\n\
  end\n\
endmodule";

/// EXPRESSION-bound: a deep operator chain (16 `acc` reads + adds) per statement, so
/// the per-statement EVAL cost dwarfs the fixed net-write/loop/scheduling cost. This is
/// the case `EVAL_HEAVY` (only ~3 ops/stmt) under-represents — and the one native-eval
/// actually moves. Measured scaling law (release, 1M statements, K = ops/stmt):
/// `t ≈ 0.39 s (fixed) + 0.058 s × K`, with the per-operand 58 ns being ~98% Value-
/// construct + `eval_ctx` dispatch overhead (net-read ≈ literal; irreducible u64 ALU
/// ≈ 1 ns). ⇒ eval is 55 % of runtime at K=8, 70 % at K=16, 82 % at K=32. Realistic
/// expression-bound RTL (wide ALUs, CRC/crypto datapaths, deep combinational cones)
/// lives in this regime; clock/scheduler-bound designs (see `CODEGEN_HEAVY`) do not.
///
/// [C4-lite] With the native-eval VM fast path (`native_eval`) live, the bytecode VM
/// now runs this body's `+` chain on native u64 registers instead of delegating each
/// operator to `eval_ctx`: measured **VM ≈ 0.42x interpreter** here (was 0.92x at C2 —
/// statement compilation alone was nearly useless for an expression-bound body), i.e.
/// ~2.3x on the VM path, realizing the "expression-bound ~2-3x" prediction. `EVAL_HEAVY`
/// (mixed) improves to ~0.77x; `CODEGEN_HEAVY` (scheduler-bound) stays ~0.94x (eval is
/// not its bottleneck — native-eval correctly does not help there).
const EXPR_HEAVY: &str = "module top;\n\
  reg clk;\n\
  reg [63:0] acc;\n\
  integer i;\n\
  integer j;\n\
  always @(posedge clk) begin\n\
    for (i = 0; i < 10000; i = i + 1) begin\n\
      acc = acc + acc + acc + acc + acc + acc + acc + acc\n\
          + acc + acc + acc + acc + acc + acc + acc + acc + 64'd1;\n\
    end\n\
  end\n\
  initial begin\n\
    clk = 0; acc = 1;\n\
    for (j = 0; j < 100; j = j + 1) begin #1 clk = 1; #1 clk = 0; end\n\
    $finish;\n\
  end\n\
endmodule";

/// STRUCTURAL-bound: selects, concats and a replicate per statement inside a hot
/// loop — the shape the native structural increment (Select/ConcatPair/Repl ops)
/// targets. Before that increment any select/concat node bailed the WHOLE
/// expression to `eval_ctx`, so this regime sat at VM ≈ interp.
const STRUCT_HEAVY: &str = "module top;\n\
  reg clk;\n\
  reg [31:0] s;\n\
  reg [15:0] acc;\n\
  reg [3:0] idx;\n\
  integer i;\n\
  integer j;\n\
  always @(posedge clk) begin\n\
    for (i = 0; i < 3000; i = i + 1) begin\n\
      acc = acc + {s[11:4], s[3:0], s[19 -: 4]} + {2{s[7:0]}};\n\
      acc = acc ^ {12'd0, s[idx +: 4]};\n\
      s = {s[30:0], s[31]};\n\
    end\n\
  end\n\
  initial begin\n\
    s = 32'hA5C31234; acc = 0; idx = 4'd6; clk = 0;\n\
    for (j = 0; j < 100; j = j + 1) begin #1 clk = 1; #1 clk = 0; end\n\
    $finish;\n\
  end\n\
endmodule";

/// [C6] WIDE-bound: the EXPR_HEAVY shape at 100 bits — every operator runs on
/// TWO-word values, the regime the u128 wide lane (WArith/WBitwise/WShl/…)
/// moves. Before C6 any >64-bit node bailed the whole expression to `eval_ctx`.
const WIDE_HEAVY: &str = "module top;\n\
  reg clk;\n\
  reg [99:0] acc;\n\
  integer i;\n\
  integer j;\n\
  always @(posedge clk) begin\n\
    for (i = 0; i < 5000; i = i + 1) begin\n\
      acc = acc + acc + acc + acc + acc + acc + acc + acc + 100'd1;\n\
      acc = acc ^ (acc >> 13);\n\
    end\n\
  end\n\
  initial begin\n\
    clk = 0; acc = 1;\n\
    for (j = 0; j < 100; j = j + 1) begin #1 clk = 1; #1 clk = 0; end\n\
    $finish;\n\
  end\n\
endmodule";

/// [v6 ④] WIDE-STRUCTURAL: >64-bit selects/concats/replicates per statement —
/// the wide-struct trio (WSelect/WConcatPair/WRepl). Before it, any wide
/// structural node bailed the WHOLE expression to `eval_ctx` (VM ≈ interp).
const WIDE_STRUCT_HEAVY: &str = "module top;\n\
  reg clk;\n\
  reg [99:0] s;\n\
  reg [99:0] acc;\n\
  integer i;\n\
  integer j;\n\
  always @(posedge clk) begin\n\
    for (i = 0; i < 3000; i = i + 1) begin\n\
      acc = acc + {s[91:28], s[27:0], s[95 -: 8]} + {2{s[49:0]}};\n\
      acc = acc ^ {s[63:0], s[99:64]};\n\
      s = {s[98:0], s[99]};\n\
    end\n\
  end\n\
  initial begin\n\
    s = 100'hA5C31234DEADBEEF55AA33; acc = 0; clk = 0;\n\
    for (j = 0; j < 100; j = j + 1) begin #1 clk = 1; #1 clk = 0; end\n\
    $finish;\n\
  end\n\
endmodule";

/// [v6 ④] REAL-bound: f64 arithmetic per statement. The native lane has NO
/// real support (every real node bails the whole expression to `eval_ctx`),
/// so VM ≈ interp here — this probe MEASURES whether a dedicated f64 register
/// lane would pay (the measure-retire gate for the documented low-ROI item).
const REAL_HEAVY: &str = "module top;\n\
  reg clk;\n\
  real a, b, acc;\n\
  integer i;\n\
  integer j;\n\
  always @(posedge clk) begin\n\
    for (i = 0; i < 5000; i = i + 1) begin\n\
      acc = acc + a * b - acc / 1.0001;\n\
      a = a * 1.0000001;\n\
      b = b + 0.0000003;\n\
    end\n\
  end\n\
  initial begin\n\
    clk = 0; a = 1.5; b = 2.25; acc = 0.0;\n\
    for (j = 0; j < 100; j = j + 1) begin #1 clk = 1; #1 clk = 0; end\n\
    $finish;\n\
  end\n\
endmodule";

/// [C6] MEMORY-bound expressions: dynamic `mem[i]` reads inside every statement
/// (the LoadIndexed lane). Before C6 an array-indexed Signal bailed the whole
/// expression to `eval_ctx`.
const MEM_HEAVY: &str = "module top;\n\
  reg clk;\n\
  reg [31:0] mem [0:15];\n\
  reg [31:0] acc;\n\
  reg [3:0] p, q;\n\
  integer i;\n\
  integer j;\n\
  always @(posedge clk) begin\n\
    for (i = 0; i < 5000; i = i + 1) begin\n\
      acc = acc + mem[p] + (mem[q] ^ acc);\n\
      p = p + 4'd3;\n\
      q = q + 4'd5;\n\
    end\n\
  end\n\
  initial begin\n\
    clk = 0; acc = 1; p = 0; q = 7;\n\
    for (i = 0; i < 16; i = i + 1) mem[i] = i * 32'h01010101;\n\
    for (j = 0; j < 100; j = j + 1) begin #1 clk = 1; #1 clk = 0; end\n\
    $finish;\n\
  end\n\
endmodule";

fn report(name: &str, src: &str, reps: u32) {
    let ir = build(src);
    let interp = time_backend(&ir, Backend::Interpreter, reps);
    let vm = time_backend(&ir, Backend::Bytecode, reps);
    println!("\n[C3 perf] {name} (best-of-{reps}):");
    println!("  interpreter : {:>8.3} ms", interp as f64 / 1e6);
    println!(
        "  bytecode VM : {:>8.3} ms   ({:.2}x interpreter)",
        vm as f64 / 1e6,
        vm as f64 / interp as f64
    );
}

#[test]
#[ignore = "perf baseline (DATA, not a gate); run with --ignored --nocapture"]
fn perf_baseline_codegen_heavy() {
    report("codegen-heavy (scheduler-dominated)", CODEGEN_HEAVY, 5);
    report("eval-heavy (eval/Value-dominated)", EVAL_HEAVY, 5);
    report(
        "expr-heavy (deep operator chain; native-eval target)",
        EXPR_HEAVY,
        5,
    );
    report(
        "struct-heavy (select/concat/replicate; structural-native target)",
        STRUCT_HEAVY,
        5,
    );
    report(
        "wide-heavy (100-bit two-word; C6 wide-lane target)",
        WIDE_HEAVY,
        5,
    );
    report(
        "mem-heavy (dynamic mem[i] reads; C6 LoadIndexed target)",
        MEM_HEAVY,
        5,
    );
    report(
        "wide-struct-heavy (>64-bit select/concat/replicate; v6 trio target)",
        WIDE_STRUCT_HEAVY,
        5,
    );
    report(
        "real-heavy (f64 arithmetic; native-lane measure-retire probe)",
        REAL_HEAVY,
        5,
    );
}

/// [P4-T0b] DUMP-heavy: many VCD value-change records (8 nets toggling every tick
/// for 20k ticks ≈ 320k records) with trivially cheap eval, so wall-time isolates
/// the VCD encode+write path. The no-dump twin is byte-for-byte the same design
/// minus `$dumpfile/$dumpvars` — the delta is the VCD share that a writer THREAD
/// (T1, `--threads ≥2`) can hide. Measures, does not gate.
const DUMP_HEAVY: &str = "module top;\n\
  reg clk;\n\
  reg [63:0] a, b, c, d, e, f, g;\n\
  integer k;\n\
  always @(posedge clk) begin\n\
    a <= a + 64'd1; b <= b + 64'd2; c <= c + 64'd3; d <= d + 64'd5;\n\
    e <= e + 64'd7; f <= f + 64'd11; g <= g + 64'd13;\n\
  end\n\
  initial begin\n\
    DUMP\n\
    clk = 0; a = 0; b = 0; c = 0; d = 0; e = 0; f = 0; g = 0;\n\
    for (k = 0; k < 20000; k = k + 1) begin #1 clk = 1; #1 clk = 0; end\n\
    $finish;\n\
  end\n\
endmodule";

/// Best-of-`reps` wall-time (ns) of `simulate` with an optional real-file VCD dump.
fn time_dump(ir: &sim_ir::SimIr, vcd_path: Option<&std::path::Path>, reps: u32) -> u128 {
    let sink = NullSink;
    let mut best = u128::MAX;
    for _ in 0..reps {
        let opts = SimOpts {
            vcd_path_override: vcd_path.map(|p| p.to_string_lossy().into_owned()),
            ..SimOpts::default()
        };
        let t = Instant::now();
        let res = simulate(ir, &sink, opts);
        best = best.min(t.elapsed().as_nanos());
        assert_eq!(res.finish_reason, FinishReason::Finish);
    }
    best
}

#[test]
#[ignore = "perf data (VCD share measurement for P4-T1); run with --ignored --nocapture"]
fn perf_dump_share() {
    let with_dump_src = DUMP_HEAVY.replace("DUMP", "$dumpfile(\"x.vcd\"); $dumpvars;");
    let no_dump_src = DUMP_HEAVY.replace("DUMP", "");
    let ir_dump = build(&with_dump_src);
    let ir_plain = build(&no_dump_src);
    let path = std::env::temp_dir().join(format!("vita_perf_dump_{}.vcd", std::process::id()));
    let t_dump = time_dump(&ir_dump, Some(&path), 5);
    let t_plain = time_dump(&ir_plain, None, 5);
    let bytes = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    let _ = std::fs::remove_file(&path);
    let share = 1.0 - (t_plain as f64 / t_dump as f64);
    println!("\n[T0b] dump-heavy VCD share (best-of-5, {bytes} VCD bytes):");
    println!("  with dump   : {:>8.3} ms", t_dump as f64 / 1e6);
    println!("  without dump: {:>8.3} ms", t_plain as f64 / 1e6);
    println!(
        "  VCD share   : {:>7.1}%   (T1 writer-thread ceiling ≤ {:.2}x)",
        share * 100.0,
        1.0 / (1.0 - share)
    );
}

/// NETS-heavy: many mostly-IDLE nets. The per-delta change sweep used to be a
/// full O(nets) `cur != prev` scan, so idle nets taxed every delta of every
/// timestep; the dirty-list sweep (scheduler R2) makes the sweep proportional
/// to nets actually WRITTEN. 512 idle regs + a 2-net clk/counter churn.
fn nets_heavy_src() -> String {
    nets_heavy_src_n(512)
}

/// Same shape with a parameterized idle-net count (scaling probe for the
/// net_to_edge/waiter layer: post-R2 wall-clock should be FLAT in N).
fn nets_heavy_src_n(n: usize) -> String {
    let mut decls = String::new();
    for i in 0..n {
        decls.push_str(&format!("  reg [63:0] idle{i};\n"));
    }
    format!(
        "module top;\n\
         {decls}\
         reg clk; reg [63:0] acc; integer k;\n\
         always @(posedge clk) acc <= acc + 64'd1;\n\
         initial begin\n\
           clk = 0; acc = 0;\n\
           for (k = 0; k < 20000; k = k + 1) begin #1 clk = 1; #1 clk = 0; end\n\
           $finish;\n\
         end\n\
         endmodule"
    )
}

#[test]
#[ignore = "perf baseline (DATA, not a gate); run with --ignored --nocapture"]
fn perf_nets_heavy() {
    let src = nets_heavy_src();
    report("nets-heavy (512 idle nets; dirty-list target)", &src, 5);
}

#[test]
#[ignore = "perf data (idle-net scaling probe); run with --ignored --nocapture"]
fn perf_nets_scaling() {
    for n in [512usize, 2048, 8192] {
        let src = nets_heavy_src_n(n);
        report(&format!("nets-scaling ({n} idle nets)"), &src, 3);
    }
}

// ─── [P4-T4] PDES feasibility probes (research track) ───────────────────────
//
// Three instruments that bound the engine-internal-parallelism design space
// WITHOUT touching the engine. The BSP-per-delta sketch (doc-18 §PDES) only
// pays when, per delta, `W × g` (batch width × per-activation work) clears the
// scatter-gather round-trip `τ` with margin; these measure all three on the
// host so the verdict is numbers, not vibes.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

/// Per-round-trip cost (ns) of a naive `thread::scope` spawn+join per delta —
/// the zero-infrastructure dispatch a first cut would reach for.
fn tau_scope_spawn(threads: usize, rounds: u32) -> f64 {
    let t = Instant::now();
    for _ in 0..rounds {
        std::thread::scope(|s| {
            for _ in 0..threads {
                s.spawn(|| std::hint::black_box(0u64));
            }
        });
    }
    t.elapsed().as_nanos() as f64 / rounds as f64
}

/// Per-round-trip cost (ns) of a persistent worker pool with a spin barrier —
/// the realistic floor for per-delta dispatch (generation counter scatter,
/// countdown gather, no parking).
fn tau_spin_pool(threads: usize, rounds: u64) -> f64 {
    let gen = AtomicU64::new(0);
    let done = AtomicUsize::new(0);
    let stop = AtomicU64::new(0);
    let mut elapsed = 0f64;
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| {
                let mut seen = 0u64;
                loop {
                    while gen.load(Ordering::Acquire) == seen {
                        if stop.load(Ordering::Acquire) == 1 {
                            return;
                        }
                        std::hint::spin_loop();
                    }
                    seen += 1;
                    done.fetch_add(1, Ordering::AcqRel);
                }
            });
        }
        let t = Instant::now();
        for _ in 0..rounds {
            done.store(0, Ordering::Release);
            gen.fetch_add(1, Ordering::AcqRel);
            while done.load(Ordering::Acquire) < threads {
                std::hint::spin_loop();
            }
        }
        elapsed = t.elapsed().as_nanos() as f64 / rounds as f64;
        stop.store(1, Ordering::Release);
    });
    elapsed
}

#[test]
#[ignore = "perf data (PDES sync-cost probe); run with --ignored --nocapture"]
fn perf_pdes_sync_cost() {
    let avail = std::thread::available_parallelism().map_or(1, |n| n.get());
    println!("\n[P4-T4] per-delta dispatch round-trip τ (host parallelism {avail}):");
    for t in [2usize, 4, 8] {
        if t > avail {
            continue;
        }
        let scope = tau_scope_spawn(t, 2000);
        let spin = tau_spin_pool(t, 200_000);
        println!(
            "  {t} threads : scope-spawn {:>9.0} ns/delta   spin-pool {:>7.0} ns/delta",
            scope, spin
        );
    }
}

/// WIDE design: `n` independent `always @(posedge clk)` blocks (the PDES unit
/// of work), each four 64-bit NBAs on private regs — every posedge delta has an
/// active batch of width `n`, the best case the BSP sketch could parallelize.
fn pdes_wide_src(n: usize, cycles: usize) -> String {
    let mut body = String::new();
    for i in 0..n {
        body.push_str(&format!(
            "  reg [63:0] a{i}, b{i}, c{i}, d{i};\n\
             \x20 always @(posedge clk) begin\n\
             \x20   a{i} <= a{i} + 64'd3;\n\
             \x20   b{i} <= b{i} ^ a{i};\n\
             \x20   c{i} <= c{i} + b{i};\n\
             \x20   d{i} <= (d{i} << 1) | (d{i} >> 63);\n\
             \x20 end\n"
        ));
    }
    format!(
        "module top;\n\
         \x20 reg clk; integer k;\n\
         {body}\
         \x20 initial begin\n\
         \x20   clk = 0;\n\
         \x20   for (k = 0; k < {cycles}; k = k + 1) begin #1 clk = 1; #1 clk = 0; end\n\
         \x20   $finish;\n\
         \x20 end\n\
         endmodule"
    )
}

#[test]
#[ignore = "perf data (PDES per-activation grain probe); run with --ignored --nocapture"]
fn perf_pdes_engine_grain() {
    println!("\n[P4-T4] engine per-activation grain g (4-NBA flop body, interp):");
    let mut prev: Option<(usize, u128)> = None;
    for n in [1usize, 16, 256, 1024] {
        let cycles = 2000usize;
        let ir = build(&pdes_wide_src(n, cycles));
        let t = time_backend(&ir, Backend::Interpreter, 3);
        let per_act = t as f64 / (cycles as f64 * n as f64);
        // Marginal cost vs the previous width isolates the body+NBA share from
        // the fixed clock-driver/timestep overhead.
        let marginal =
            prev.map(|(pn, pt)| (t.saturating_sub(pt)) as f64 / (cycles as f64 * (n - pn) as f64));
        match marginal {
            Some(m) => println!(
                "  W={n:>5} : {:>8.3} ms   {per_act:>7.1} ns/activation (marginal {m:>6.1} ns)",
                t as f64 / 1e6
            ),
            None => println!(
                "  W={n:>5} : {:>8.3} ms   {per_act:>7.1} ns/activation",
                t as f64 / 1e6
            ),
        }
        prev = Some((n, t));
    }
}

/// The synthetic per-task work kernel (wrapping integer mix on private state,
/// like a flop-body eval). File-scope so calibration times exactly this loop.
fn pdes_kernel(seed: u64, iters: u64) -> u64 {
    let mut x = seed | 1;
    for _ in 0..iters {
        x = x
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        x ^= x >> 29;
    }
    x
}

/// End-to-end BSP mock: per "delta", `w` tasks of synthetic eval work
/// (private-state integer arithmetic, like a flop body) plus a serial commit
/// pass (~the dirty-list/NBA merge a real BSP delta keeps sequential), run (a)
/// single-thread and (b) on a persistent spin-barrier pool with static chunk
/// partitioning — exactly the deterministic-by-construction dispatch the
/// design sketch proposes. Returns (sequential ns/delta, parallel ns/delta).
fn bsp_mock(w: usize, iters_per_task: u64, threads: usize, deltas: u32) -> (f64, f64) {
    let kernel = pdes_kernel;
    let mut out = vec![0u64; w];
    // Sequential reference.
    let t = Instant::now();
    for _ in 0..deltas {
        for (i, slot) in out.iter_mut().enumerate() {
            *slot = kernel(i as u64, iters_per_task);
        }
        std::hint::black_box(&mut out);
    }
    let seq = t.elapsed().as_nanos() as f64 / deltas as f64;

    // Parallel: persistent pool, generation-counter scatter, countdown gather,
    // static contiguous chunks (deterministic ownership), serial commit after.
    let gen = AtomicU64::new(0);
    let done = AtomicUsize::new(0);
    let stop = AtomicU64::new(0);
    let chunk = w.div_ceil(threads);
    let slots: Vec<AtomicU64> = (0..w).map(|_| AtomicU64::new(0)).collect();
    let mut par = 0f64;
    std::thread::scope(|s| {
        for tid in 0..threads {
            let slots = &slots;
            let gen = &gen;
            let done = &done;
            let stop = &stop;
            s.spawn(move || {
                let lo = tid * chunk;
                let hi = ((tid + 1) * chunk).min(w);
                let mut seen = 0u64;
                loop {
                    while gen.load(Ordering::Acquire) == seen {
                        if stop.load(Ordering::Acquire) == 1 {
                            return;
                        }
                        std::hint::spin_loop();
                    }
                    seen += 1;
                    for (i, slot) in slots.iter().enumerate().take(hi).skip(lo) {
                        slot.store(kernel(i as u64, iters_per_task), Ordering::Relaxed);
                    }
                    done.fetch_add(1, Ordering::AcqRel);
                }
            });
        }
        let t = Instant::now();
        for _ in 0..deltas {
            done.store(0, Ordering::Release);
            gen.fetch_add(1, Ordering::AcqRel);
            while done.load(Ordering::Acquire) < threads {
                std::hint::spin_loop();
            }
            // Serial commit pass: the merge work a real BSP delta keeps
            // single-threaded (dirty-list push + NBA log splice per task).
            for (slot, o) in slots.iter().zip(out.iter_mut()) {
                *o = slot.load(Ordering::Relaxed);
            }
            std::hint::black_box(&mut out);
        }
        par = t.elapsed().as_nanos() as f64 / deltas as f64;
        stop.store(1, Ordering::Release);
    });
    (seq, par)
}

#[test]
#[ignore = "perf data (PDES BSP-mock speedup matrix); run with --ignored --nocapture"]
fn perf_pdes_bsp_mock() {
    let avail = std::thread::available_parallelism().map_or(1, |n| n.get());
    // Calibrate the bare kernel loop so iteration counts map to target grains.
    let cal_iters = 50_000_000u64;
    let t = Instant::now();
    std::hint::black_box(pdes_kernel(1, cal_iters));
    let ns_per_iter = t.elapsed().as_nanos() as f64 / cal_iters as f64;
    println!(
        "\n[P4-T4] BSP scatter-gather mock (host parallelism {avail}, kernel {ns_per_iter:.2} ns/iter):"
    );
    for &g_target in &[60f64, 250.0, 1000.0] {
        let iters = (g_target / ns_per_iter).max(1.0) as u64;
        for &w in &[8usize, 64, 512, 4096] {
            // Keep each config ~tens of ms total.
            let deltas = ((40e6 / (w as f64 * g_target)) as u32).clamp(20, 20_000);
            for &t in &[4usize, 8] {
                if t > avail {
                    continue;
                }
                let (seq, par) = bsp_mock(w, iters, t, deltas);
                println!(
                    "  g≈{g_target:>4.0}ns W={w:>4} T={t} : seq {seq:>10.0} ns/delta (meas {:>5.0} ns/task)   par {par:>10.0} ns/delta   speedup {:>5.2}x",
                    seq / w as f64,
                    seq / par
                );
            }
        }
    }
}

// ── P9 coverage probe (2026-07-31) ───────────────────────────────────────────
//
// Speedup numbers above are per-BODY. They only matter to a user if that user's
// bodies actually clear the P9 allow-list, and a body outside it runs on the
// interpreter under EITHER backend. So the question that decides whether
// `Backend::Bytecode` is worth exposing — and whether a faster (native/JIT)
// backend could ever pay, since it would inherit the SAME allow-list — is
// coverage, not ratio.
//
// The two designs below are the same SHA-256 compression round written the two
// ways real RTL is written: transforms inline, and transforms behind `function`.

/// SHA-256 round, transforms written INLINE. No calls, no delays in the body.
const SHA256_INLINE: &str = "module top;\n\
  reg clk = 0;\n\
  reg [31:0] a,b,c,d,e,f,g,h,w,k;\n\
  reg [31:0] s0,s1,ch,maj,t1,t2;\n\
  integer i;\n\
  always @(posedge clk) begin\n\
    s1  = {e[5:0],e[31:6]} ^ {e[10:0],e[31:11]} ^ {e[24:0],e[31:25]};\n\
    ch  = (e & f) ^ (~e & g);\n\
    t1  = h + s1 + ch + k + w;\n\
    s0  = {a[1:0],a[31:2]} ^ {a[12:0],a[31:13]} ^ {a[21:0],a[31:22]};\n\
    maj = (a & b) ^ (a & c) ^ (b & c);\n\
    t2  = s0 + maj;\n\
    h <= g; g <= f; f <= e; e <= d + t1;\n\
    d <= c; c <= b; b <= a; a <= t1 + t2;\n\
    w <= {w[6:0],w[31:7]} ^ w ^ k;\n\
    k <= k + 32'h9e3779b9;\n\
  end\n\
  initial begin\n\
    a=32'h6a09e667; b=32'hbb67ae85; c=32'h3c6ef372; d=32'ha54ff53a;\n\
    e=32'h510e527f; f=32'h9b05688c; g=32'h1f83d9ab; h=32'h5be0cd19;\n\
    w=32'h428a2f98; k=32'h71374491;\n\
    for (i=0;i<20000;i=i+1) begin clk=~clk; #1; end\n\
    $display(\"%h\", a^b^c^d^e^f^g^h);\n\
    $finish;\n\
  end\n\
endmodule\n";

/// The SAME round with the transforms behind `function` — the idiomatic form.
const SHA256_FUNCS: &str = "module top;\n\
  reg clk = 0;\n\
  reg [31:0] a,b,c,d,e,f,g,h,w,k;\n\
  reg [31:0] t1,t2;\n\
  integer i;\n\
  function [31:0] bsig1(input [31:0] x);\n\
    bsig1 = {x[5:0],x[31:6]} ^ {x[10:0],x[31:11]} ^ {x[24:0],x[31:25]};\n\
  endfunction\n\
  function [31:0] bsig0(input [31:0] x);\n\
    bsig0 = {x[1:0],x[31:2]} ^ {x[12:0],x[31:13]} ^ {x[21:0],x[31:22]};\n\
  endfunction\n\
  function [31:0] choose(input [31:0] x, y, z);\n\
    choose = (x & y) ^ (~x & z);\n\
  endfunction\n\
  function [31:0] major(input [31:0] x, y, z);\n\
    major = (x & y) ^ (x & z) ^ (y & z);\n\
  endfunction\n\
  always @(posedge clk) begin\n\
    t1 = h + bsig1(e) + choose(e,f,g) + k + w;\n\
    t2 = bsig0(a) + major(a,b,c);\n\
    h <= g; g <= f; f <= e; e <= d + t1;\n\
    d <= c; c <= b; b <= a; a <= t1 + t2;\n\
    w <= {w[6:0],w[31:7]} ^ w ^ k;\n\
    k <= k + 32'h9e3779b9;\n\
  end\n\
  initial begin\n\
    a=32'h6a09e667; b=32'hbb67ae85; c=32'h3c6ef372; d=32'ha54ff53a;\n\
    e=32'h510e527f; f=32'h9b05688c; g=32'h1f83d9ab; h=32'h5be0cd19;\n\
    w=32'h428a2f98; k=32'h71374491;\n\
    for (i=0;i<20000;i=i+1) begin clk=~clk; #1; end\n\
    $display(\"%h\", a^b^c^d^e^f^g^h);\n\
    $finish;\n\
  end\n\
endmodule\n";

/// STIMULUS-LIKE: an eligible body doing what a *stimulus* body does — a couple of
/// trivial assignments per activation and nothing else. This is the shape C
/// (expanding the P9 allow-list over suspend-bearing bodies) would absorb, so the
/// VM ratio HERE is the upper bound on what C could realize: C's bodies are this
/// light AND carry suspend points, which add resume-state work this one does not.
const STIM_LIKE: &str = "module top;\n\
  reg tick; reg clk; reg [7:0] ctr;\n\
  integer k;\n\
  always @(posedge tick) begin\n\
    clk = ~clk;\n\
    ctr = ctr + 8'd1;\n\
  end\n\
  initial begin\n\
    tick = 0; clk = 0; ctr = 0;\n\
    for (k = 0; k < 200000; k = k + 1) begin #1 tick = 1; #1 tick = 0; end\n\
    $display(\"%0d\", ctr);\n\
    $finish;\n\
  end\n\
endmodule\n";

/// Print P9 coverage AND the backend ratio for one design.
fn report_coverage(name: &str, src: &str, reps: u32) {
    let ir = build(src);
    let cov = sim_engine::codegen_coverage(&ir);
    let interp = time_backend(&ir, Backend::Interpreter, reps);
    let vm = time_backend(&ir, Backend::Bytecode, reps);
    println!(
        "  {name:<22} P9 {:>2}/{:<2} ({:>5.1}%)   interp {:>8.3} ms   vm {:>8.3} ms   {:.2}x",
        cov.codegen_able,
        cov.total,
        cov.ratio() * 100.0,
        interp as f64 / 1e6,
        vm as f64 / 1e6,
        vm as f64 / interp as f64
    );
}

#[test]
#[ignore = "perf/coverage probe (DATA, not a gate); run with --ignored --nocapture"]
fn perf_p9_coverage() {
    println!("\n[P9 coverage] process templates the bytecode VM can claim:\n");
    report_coverage("sha256-round inline", SHA256_INLINE, 3);
    report_coverage("sha256-round funcs", SHA256_FUNCS, 3);
    report_coverage("expr-heavy", EXPR_HEAVY, 3);
    report_coverage("struct-heavy", STRUCT_HEAVY, 3);
    report_coverage("clock-bound", CODEGEN_HEAVY, 3);
    report_coverage("mem-heavy", MEM_HEAVY, 3);
    report_coverage("stimulus-like", STIM_LIKE, 3);

    // The repository's own example designs — what a first-time user actually runs.
    println!("\n[P9 coverage] examples/ (skipped if the directory is absent):\n");
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let Ok(rd) = std::fs::read_dir(&dir) else {
        println!("  (examples/ not found — skipped)");
        return;
    };
    let mut paths: Vec<_> = rd
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "sv" || x == "v"))
        .collect();
    paths.sort();
    for p in paths {
        let raw = std::fs::read_to_string(&p).expect("example is readable");
        // Examples carry `` `timescale ``, so they must go through the preprocessor —
        // `build` starts at the lexer and would see a bare Directive token.
        let pp = hdl_preprocess::preprocess_sources(
            &dir,
            &[(p.to_string_lossy().into_owned(), raw)],
            &hdl_preprocess::PreOpts::default(),
        );
        let ir = build(&pp.text);
        let cov = sim_engine::codegen_coverage(&ir);
        println!(
            "  {:<22} P9 {:>2}/{:<2} ({:>5.1}%)",
            p.file_name().unwrap_or_default().to_string_lossy(),
            cov.codegen_able,
            cov.total,
            cov.ratio() * 100.0
        );
    }
}

/// An eligible body carrying exactly `stmts` trivial statements, driven by a fixed
/// number of edges. Sweeping `stmts` finds the CROSSOVER: how much work a body must
/// carry before the VM's per-activation fixed cost (register-file lease, prologue,
/// dispatch loop) is amortized. Everything below the crossover is a body the VM
/// makes SLOWER.
fn work_per_body_src(stmts: usize, cycles: usize) -> String {
    let mut body = String::new();
    for _ in 0..stmts {
        body.push_str("    ctr = ctr + 8'd1;\n");
    }
    format!(
        "module top;\n\
           reg tick; reg [7:0] ctr;\n\
           integer k;\n\
           always @(posedge tick) begin\n{body}  end\n\
           initial begin\n\
             tick = 0; ctr = 0;\n\
             for (k = 0; k < {cycles}; k = k + 1) begin #1 tick = 1; #1 tick = 0; end\n\
             $display(\"%0d\", ctr);\n\
             $finish;\n\
           end\n\
         endmodule\n"
    )
}

/// [C-GAIN] Does expanding the P9 allow-list (step C) pay?
///
/// C would absorb the suspend-bearing STIMULUS bodies — the `#delay`-driven half of
/// every design. Those bodies are eval-light, so the question is not "can the VM run
/// them" but "does the VM make a light body faster at all". This sweep answers it
/// without building C: it measures the VM ratio as a function of work per activation,
/// on bodies the VM ALREADY takes.
#[test]
#[ignore = "perf probe (DATA, not a gate); run with --ignored --nocapture"]
fn perf_work_per_body_crossover() {
    println!("\n[C-GAIN] VM ratio vs work per activation (100k edges, fixed):\n");
    println!(
        "  {:>6}  {:>10}  {:>10}  {:>8}",
        "stmts", "interp ms", "vm ms", "vm/interp"
    );
    for stmts in [1usize, 2, 4, 8, 16, 32, 64] {
        let ir = build(&work_per_body_src(stmts, 100_000));
        let i = time_backend(&ir, Backend::Interpreter, 3) as f64;
        let v = time_backend(&ir, Backend::Bytecode, 3) as f64;
        println!(
            "  {stmts:>6}  {:>10.1}  {:>10.1}  {:>7.2}x{}",
            i / 1e6,
            v / 1e6,
            v / i,
            if v < i { "   <- VM wins" } else { "" }
        );
    }
    println!(
        "\n  A stimulus body carries 1-3 statements. If the crossover sits above that,\n  \
         step C would move those bodies onto a path that is SLOWER for them."
    );
}

/// A chain of `d` combinational stages between two clocked endpoints — the shape whose
/// per-cycle activation profile is the `D²/2` triangle levelization targets.
fn comb_chain_src(d: usize, cycles: usize) -> String {
    let mut decls = String::new();
    for i in 0..d {
        decls.push_str(&format!("  reg [31:0] s{i};\n"));
    }
    let mut stages = String::new();
    for i in 0..d {
        let src = if i == 0 {
            "seed".to_string()
        } else {
            format!("s{}", i - 1)
        };
        stages.push_str(&format!(
            "  always_comb s{i} = ({src} ^ (({src} << 1) | ({src} >> 31))) + 32'd{};\n",
            i + 1
        ));
    }
    format!(
        "module top;\n\
           reg clk; reg [31:0] seed; reg [31:0] acc;\n{decls}{stages}\
           integer k;\n\
           always @(posedge clk) begin\n\
             seed <= seed + 32'd1;\n\
             acc  <= acc ^ s{last};\n\
           end\n\
           initial begin\n\
             clk = 0; seed = 32'd1; acc = 0;\n\
             for (k = 0; k < {cycles}; k = k + 1) begin #1 clk = 1; #1 clk = 0; end\n\
             $display(\"acc=%h\", acc);\n\
             $finish;\n\
           end\n\
         endmodule\n",
        last = d - 1
    )
}

/// The §4.5.278 triangle shape: stages chained through module INSTANCES with port
/// continuous assigns. The port settle drives every stage's input in one go, so all
/// `d` stages wake in the SAME delta and each runs on a partially stale input — the
/// `7 6 5 4 3 2 1 …` profile. A chain of plain `always_comb` in one module does NOT
/// reproduce it (the wake chain is naturally one-at-a-time), which is why the first
/// version of this probe measured a flat 1.00x.
fn inst_chain_src(d: usize, cycles: usize) -> String {
    let mut insts = String::new();
    let mut decls = String::new();
    for i in 0..d {
        decls.push_str(&format!("  wire [31:0] w{i};\n"));
        let src = if i == 0 {
            "seed".to_string()
        } else {
            format!("w{}", i - 1)
        };
        insts.push_str(&format!("  stage u{i} (.a({src}), .y(w{i}));\n"));
    }
    format!(
        "module stage(input [31:0] a, output [31:0] y);\n\
           reg [31:0] r;\n\
           always_comb r = (a ^ ((a << 1) | (a >> 31))) + 32'd1;\n\
           assign y = r;\n\
         endmodule\n\
         module top;\n\
           reg clk; reg [31:0] seed; reg [31:0] acc;\n{decls}{insts}\
           integer k;\n\
           always @(posedge clk) begin\n\
             seed <= seed + 32'd1;\n\
             acc  <= acc ^ w{last};\n\
           end\n\
           initial begin\n\
             clk = 0; seed = 32'd1; acc = 0;\n\
             for (k = 0; k < {cycles}; k = k + 1) begin #1 clk = 1; #1 clk = 0; end\n\
             $display(\"acc=%h\", acc);\n\
             $finish;\n\
           end\n\
         endmodule\n",
        last = d - 1
    )
}

/// [D] Where the depth cost actually lives.
///
/// Total cycles are held fixed and only DEPTH varies, so any growth is pure
/// depth cost. `maxrank` is the design's combinational depth as
/// `levelize::comb_ranks` computes it — reported so the number the cost is
/// plotted against is the measured one, not the one the generator intended.
///
/// The measured conclusion (2026-08-01): the growth is NOT in process
/// activations, so rank-ordering the Active drain does not touch it. A
/// rank-ordered drain with inter-rank settle was built and measured at 1.00x on
/// exactly this shape, then reverted. The cost is in `settle_cont_assigns`,
/// which makes a FULL pass over every continuous assign, to fixpoint, on EVERY
/// delta — and a depth-D chain takes D deltas to propagate, so the settle work
/// is paid D times over D assigns.
#[test]
#[ignore = "perf probe (DATA); run with --ignored --nocapture"]
fn perf_depth_cost_shape() {
    println!("\n[D] depth cost, total cycles held fixed:\n");
    println!(
        "  {:>6} {:>8} {:>10} {:>10}",
        "depth", "maxrank", "chain ms", "instances ms"
    );
    for d in [1usize, 2, 3, 6, 12, 24] {
        let one = |src: String| {
            let ir = build(&src);
            let mr = sim_engine::comb_ranks(&ir)
                .iter()
                .copied()
                .max()
                .unwrap_or(0);
            (time_backend(&ir, Backend::Interpreter, 3) as f64 / 1e6, mr)
        };
        let (chain, mr) = one(comb_chain_src(d, 2000));
        let (inst, _) = one(inst_chain_src(d, 2000));
        println!("  {d:>6} {mr:>8} {chain:>10.1} {inst:>12.1}");
    }
}

/// The SAME combinational logic as `comb_chain_src`/`inst_chain_src`, but FUSED into a
/// single `always_comb` body. This is what process fusion (step E) would produce, so
/// comparing the three forms measures E's payoff without building E.
fn fused_chain_src(d: usize, cycles: usize) -> String {
    let mut body = String::new();
    for i in 0..d {
        let src = if i == 0 {
            "seed".to_string()
        } else {
            format!("s{}", i - 1)
        };
        body.push_str(&format!(
            "    s{i} = ({src} ^ (({src} << 1) | ({src} >> 31))) + 32'd{};\n",
            i + 1
        ));
    }
    let mut decls = String::new();
    for i in 0..d {
        decls.push_str(&format!("  reg [31:0] s{i};\n"));
    }
    format!(
        "module top;\n\
           reg clk; reg [31:0] seed; reg [31:0] acc;\n{decls}\
           integer k;\n\
           always_comb begin\n{body}  end\n\
           always @(posedge clk) begin\n\
             seed <= seed + 32'd1;\n\
             acc  <= acc ^ s{last};\n\
           end\n\
           initial begin\n\
             clk = 0; seed = 32'd1; acc = 0;\n\
             for (k = 0; k < {cycles}; k = k + 1) begin #1 clk = 1; #1 clk = 0; end\n\
             $display(\"acc=%h\", acc);\n\
             $finish;\n\
           end\n\
         endmodule\n",
        last = d - 1
    )
}

/// [E-SPIKE] Does fusing chained combinational processes raise what the VM can pay?
///
/// The C-GAIN crossover says the VM returns ~nothing at 1-2 statements per activation
/// and 1.33x at 64. Fusion is the transform that moves a design rightward on that
/// curve. Three forms of IDENTICAL logic, same depth, same cycle count:
///
/// - `instances` — d modules chained through port cont-assigns (1 stmt/body)
/// - `separate`  — d `always_comb` in one module      (1 stmt/body)
/// - `fused`     — ONE `always_comb` with d statements (d stmts/body)
///
/// NOTE on the `value` column: `instances` legitimately computes a DIFFERENT function
/// (the shared `stage` module adds a uniform `+1`, while the other two add `+i+1` at
/// stage `i`), so its value differs by construction. The equivalence that matters is
/// `fused == separate`, which holds at every depth — fusion must not change the value.
///
/// If `fused` shows a materially better VM ratio than the other two, E is the enabler
/// and step ② is worth building. If it does not, the compiled path is closed for good
/// and this probe is the record of why.
#[test]
#[ignore = "E-spike (DATA); run with --ignored --nocapture"]
fn perf_fusion_spike() {
    println!("\n[E-SPIKE] identical logic, three process shapes (2000 cycles):\n");
    println!(
        "  {:>5}  {:<10} {:>9} {:>9} {:>9}  value",
        "depth", "form", "interp", "vm", "vm/interp"
    );
    for d in [6usize, 12, 24, 48] {
        let forms: [(&str, String); 3] = [
            ("instances", inst_chain_src(d, 2000)),
            ("separate", comb_chain_src(d, 2000)),
            ("fused", fused_chain_src(d, 2000)),
        ];
        for (name, src) in forms {
            let ir = build(&src);
            let i = time_backend(&ir, Backend::Interpreter, 3) as f64 / 1e6;
            let v = time_backend(&ir, Backend::Bytecode, 3) as f64 / 1e6;
            let (_, out) = sim_engine::simulate_capture(&ir, SimOpts::default());
            println!(
                "  {d:>5}  {name:<10} {i:>9.1} {v:>9.1} {:>8.2}x  {}",
                v / i,
                out.trim()
            );
        }
        println!();
    }
}

/// [E-OPP] How much fusion opportunity do real designs actually have?
///
/// Building fusion is only worth it if real designs contain fusable chains. This
/// reports, per design, how many combinational processes could be fused away under the
/// order-preserving condition — measured BEFORE any fusion machinery is built, the same
/// discipline that stopped steps C and F from being built on an assumed payoff.
#[test]
#[ignore = "E opportunity probe (DATA); run with --ignored --nocapture"]
fn perf_fusion_opportunity() {
    println!("\n[E-OPP] fusable combinational pairs per design:\n");
    println!(
        "  {:<24} {:>10} {:>10}  {:>5}   {:>8}",
        "design", "processes", "fusable", "chain", "x-copy"
    );
    let report = |name: &str, src: &str| {
        let ir = build(src);
        let pairs = sim_engine::fusion_candidates(&ir);
        // Longest fusable chain: how many bodies would collapse into one.
        let mut succ = std::collections::BTreeMap::new();
        for p in &pairs {
            succ.insert(p.producer, p.consumer);
        }
        let mut longest = 0usize;
        for &start in succ.keys() {
            let (mut cur, mut len) = (start, 1usize);
            while let Some(&nx) = succ.get(&cur) {
                len += 1;
                cur = nx;
                if len > ir.processes.len() {
                    break;
                }
            }
            longest = longest.max(len);
        }
        println!(
            "  {name:<24} {:>10} {:>10}  {longest:>5}   {:>8}",
            ir.processes.len(),
            pairs.len(),
            sim_engine::fusion_candidates_across_copies(&ir)
        );
    };
    report("inst-chain d=24", &inst_chain_src(24, 10));
    report("separate d=24", &comb_chain_src(24, 10));
    report("fused d=24", &fused_chain_src(24, 10));
    report("sha256-round", SHA256_INLINE);
    report("expr-heavy", EXPR_HEAVY);

    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let Ok(rd) = std::fs::read_dir(&dir) else {
        return;
    };
    let mut paths: Vec<_> = rd
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "sv"))
        .collect();
    paths.sort();
    for p in paths {
        let raw = std::fs::read_to_string(&p).expect("example is readable");
        let pp = hdl_preprocess::preprocess_sources(
            &dir,
            &[(p.to_string_lossy().into_owned(), raw)],
            &hdl_preprocess::PreOpts::default(),
        );
        report(
            &p.file_name().unwrap_or_default().to_string_lossy(),
            &pp.text,
        );
    }
}

/// [M1-M3] Phase-1 prerequisites, taken on a REAL design rather than a generated chain.
///
/// Reads the design from `bench/` (gitignored, third-party, local measurement only).
/// Skips gracefully when absent so this is never a CI dependency.
///
/// M1 = combinational depth (`comb_ranks().max()`) — is there a deep cone at all?
/// M2 = process/net shape and P9 coverage — how much can a body-side backend reach?
/// M3 = fusion opportunity — would a cycle mode have anything to fuse?
#[test]
#[ignore = "Phase-1 measurement on bench/ designs; run with --ignored --nocapture"]
fn perf_real_design_m1_m3() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../bench");
    let Ok(rd) = std::fs::read_dir(&root) else {
        println!("\n[M1-M3] bench/ absent — skipped");
        return;
    };
    println!("\n[M1-M3] real designs:\n");
    println!(
        "  {:<22} {:>6} {:>6} {:>7} {:>8} {:>8} {:>7}",
        "design", "procs", "nets", "cont-a", "M1 depth", "M2 P9", "M3 fuse"
    );
    let mut dirs: Vec<_> = rd.filter_map(|e| e.ok().map(|e| e.path())).collect();
    dirs.sort();
    for d in dirs {
        let Ok(files) = std::fs::read_dir(&d) else {
            continue;
        };
        let mut srcs: Vec<_> = files
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "v" || x == "sv"))
            .collect();
        srcs.sort();
        for p in srcs {
            let raw = std::fs::read_to_string(&p).expect("bench source is readable");
            let pp = hdl_preprocess::preprocess_sources(
                &d,
                &[(p.to_string_lossy().into_owned(), raw)],
                &hdl_preprocess::PreOpts::default(),
            );
            let (toks, le) = hdl_lexer::lex(&pp.text);
            if !le.is_empty() {
                println!("  {:<22} LEX FAILED ({} errors)", name_of(&p), le.len());
                continue;
            }
            let (su, pe) = hdl_parser::parse(&toks, &pp.text);
            if !pe.is_empty() {
                println!("  {:<22} PARSE FAILED ({} errors)", name_of(&p), pe.len());
                continue;
            }
            let sink = QuietSink;
            let Some(ir) = elaborate::elaborate(&su.expect("source unit"), &sink) else {
                println!("  {:<22} ELABORATE FAILED", name_of(&p));
                continue;
            };
            let ranks = sim_engine::comb_ranks(&ir);
            let cov = sim_engine::codegen_coverage(&ir);
            println!(
                "  {:<22} {:>6} {:>6} {:>7} {:>8} {:>7.0}% {:>7}",
                name_of(&p),
                ir.processes.len(),
                ir.nets.len(),
                ir.cont_assigns.len(),
                match sim_engine::comb_depth(&ir) {
                    Some(d) => format!("{d}"),
                    None => format!("cyc>{}", ranks.iter().copied().max().unwrap_or(0)),
                },
                cov.ratio() * 100.0,
                sim_engine::fusion_candidates(&ir).len()
                    + sim_engine::fusion_candidates_across_copies(&ir),
            );
            let srw = sim_engine::self_read_write_processes(&ir);
            println!(
                "      self read+write processes: {} of {} (nets involved: {})",
                srw.len(),
                ir.processes.len(),
                srw.iter().map(|(_, n)| n.len()).sum::<usize>()
            );
        }
    }
}

fn name_of(p: &std::path::Path) -> String {
    p.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}

/// Swallows elaborate diagnostics — a third-party design emits plenty that are not
/// this measurement's concern.
struct QuietSink;
impl LogSink for QuietSink {
    fn emit(&self, _e: LogEvent) {}
}

/// [B-OPS] Which operators does a REAL design actually use, and which of them can
/// native-eval compile?
///
/// The benchmark suite in this file was written from the ops native-eval already
/// supported — `EXPR_HEAVY` is a chain of `+`, `STRUCT_HEAVY` is select/concat/replicate.
/// So it measured 1.9-2.8x while never exercising anything that bails. This counts the
/// operators a real design contains instead.
#[test]
#[ignore = "operator census on bench/ designs; run with --ignored --nocapture"]
fn perf_real_design_operator_census() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../bench/picorv32");
    let (Ok(tb), Ok(core)) = (
        std::fs::read_to_string(dir.join("tb.v")),
        std::fs::read_to_string(dir.join("picorv32.v")),
    ) else {
        println!("\n[B-OPS] bench/picorv32 absent — skipped");
        return;
    };
    let pp = hdl_preprocess::preprocess_sources(
        &dir,
        &[("tb.v".into(), tb), ("picorv32.v".into(), core)],
        &hdl_preprocess::PreOpts::default(),
    );
    let (toks, _) = hdl_lexer::lex(&pp.text);
    let (su, _) = hdl_parser::parse(&toks, &pp.text);
    let sink = QuietSink;
    let (ir, _sc) = elaborate::elaborate_with_timescale_roots(
        &su.expect("source unit"),
        &sink,
        &std::collections::BTreeMap::new(),
        -9,
        Some(&["tb".to_string()]),
    );
    let ir = ir.expect("picorv32 tb elaborates");

    // The seven binary ops native-eval can compile today (native_eval/compile.rs).
    let supported = |op: sim_ir::BinOp| {
        use sim_ir::BinOp::*;
        matches!(op, Add | Sub | Mul | BitAnd | BitOr | BitXor | BitXnor)
    };
    let mut counts: std::collections::BTreeMap<String, (usize, bool)> =
        std::collections::BTreeMap::new();
    let (mut sup, mut unsup) = (0usize, 0usize);
    for e in &ir.exprs {
        if let sim_ir::Expr::Binary { op, .. } = e {
            let ok = supported(*op);
            let k = format!("{op:?}");
            let ent = counts.entry(k).or_insert((0, ok));
            ent.0 += 1;
            if ok {
                sup += 1
            } else {
                unsup += 1
            }
        }
    }
    println!("\n[B-OPS] picorv32 + tb: {} binary-op nodes\n", sup + unsup);
    let mut rows: Vec<_> = counts.into_iter().collect();
    rows.sort_by_key(|(_, (n, _))| std::cmp::Reverse(*n));
    for (op, (n, ok)) in &rows {
        println!(
            "  {:<10} {:>6}  {}",
            op,
            n,
            if *ok { "native" } else { "BAILS to eval_ctx" }
        );
    }
    println!(
        "\n  native-eval can compile {}/{} = {:.0}% of binary-op nodes",
        sup,
        sup + unsup,
        100.0 * sup as f64 / (sup + unsup) as f64
    );
}
