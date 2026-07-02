//! [P5] The compiled-vs-interpreter differential gate.
//!
//! For every design in the deterministic P6 corpus, run it on BOTH the interpreter
//! and the bytecode backend from the SAME elaborated `SimIr`, and assert the two
//! runs are byte-identical: stdout, VCD bytes, and the `SimResult` summary
//! (sim_time / finish_reason / exit_class).
//!
//! This is vita-self-contained — it does NOT shell out to iverilog (that oracle
//! lives separately in `differential.rs` and is graceful-skippable). Being a plain
//! `#[test]` in the default suite, it runs under `cargo test --workspace --locked`
//! on every CI leg with no skip → a HARD equivalence gate.
//!
//! STAGE-B STATE: the bytecode backend currently falls back to the interpreter for
//! every body, so this passes by construction today. That is exactly the point —
//! the gate is wired and green BEFORE the kernel refactor (P7a/P7b) and BEFORE any
//! VM lowering (Stage C), so the moment a real VM body diverges in stdout or a
//! single VCD byte, this test goes red and names the offending design.

mod common;

use common::{build, corpus, run_capture};
use sim_engine::{simulate_capture, Backend, ExitClass, SimOpts, SimResult};

/// A wide, fixed-seed sweep: every corpus design must produce byte-identical
/// stdout + VCD + summary across the two backends.
#[test]
fn compiled_equals_interpreter_over_corpus() {
    // 72 designs over the 9 templates (8 repeats each, varied params). Fixed seed →
    // reproducible on every OS.
    for d in corpus(0x5EED_F00D, 72) {
        let ir = build(&d.src);
        // P4-T0a: the two backend runs are independent (separate sinks, separate
        // VCD temp paths) — run them CONCURRENTLY via thread::scope. `SimIr` is
        // plain shared data (Sync); each thread builds its own capture sink, so
        // nothing crosses threads but the `&ir` borrow. ~2x suite wall-clock.
        let (ir_ref, name) = (&ir, d.name.as_str());
        let ((ri, oi, vi), (rb, ob, vb)) = std::thread::scope(|s| {
            let hi = s.spawn(move || run_capture(ir_ref, Backend::Interpreter, name));
            let hb = s.spawn(move || run_capture(ir_ref, Backend::Bytecode, name));
            (
                hi.join().expect("interpreter run panicked"),
                hb.join().expect("bytecode run panicked"),
            )
        });

        assert_eq!(oi, ob, "stdout differs across backends for `{}`", d.name);
        assert_eq!(
            vi,
            vb,
            "VCD bytes differ across backends for `{}` ({} vs {} bytes)",
            d.name,
            vi.as_ref().map_or(0, |v| v.len()),
            vb.as_ref().map_or(0, |v| v.len()),
        );
        assert_eq!(
            ri.sim_time, rb.sim_time,
            "sim_time differs for `{}`",
            d.name
        );
        assert_eq!(
            ri.finish_reason, rb.finish_reason,
            "finish_reason differs for `{}`",
            d.name
        );
        assert_eq!(
            ri.exit_class, rb.exit_class,
            "exit_class differs for `{}`",
            d.name
        );
    }
}

/// Sanity that the gate has TEETH: a design that actually dumps VCD yields non-empty
/// VCD bytes on both backends (so an all-`None` VCD comparison can't vacuously pass).
#[test]
fn gate_actually_compares_vcd_bytes() {
    // The `counter_*` template always `$dumpvars` — find one and assert real bytes.
    let d = corpus(0x5EED_F00D, 8)
        .into_iter()
        .find(|d| d.name.starts_with("counter_"))
        .expect("corpus must contain a counter design");
    let ir = build(&d.src);
    let (_ri, _oi, vi) = run_capture(&ir, Backend::Interpreter, &d.name);
    let (_rb, _ob, vb) = run_capture(&ir, Backend::Bytecode, &d.name);
    let bytes = vi.expect("counter design must emit a VCD");
    assert!(bytes.len() > 32, "VCD should be non-trivial");
    assert!(
        bytes.starts_with(b"$date") || bytes.starts_with(b"$version") || bytes.starts_with(b"$"),
        "VCD should start with a $-keyword preamble"
    );
    assert_eq!(Some(bytes), vb, "counter VCD must match across backends");
}

/// [P9b] A single run MIXES backends. In the Bytecode backend the codegen-able
/// `always @(posedge clk)` body takes the VM path (P9), while the interpreted
/// `initial #1 …` and BOTH continuous assigns fall back to the interpreter — all
/// writing SHARED nets (`a`/`sum`/`q`/`r`). Prove the mixed run is byte-identical
/// (stdout AND VCD) to an all-interpreter run.
///
/// nba_seq ordering is verified IMPLICITLY and with teeth: the always body issues two
/// nonblocking writes (`q <= sum; r <= q;`), so `r` must see the OLD `q` (a one-cycle
/// lag). If a compiled body ever called `schedule_nba` in a different order, `apply_nba`
/// would sort differently, `r` would capture the NEW `q`, and the shared-net values —
/// hence stdout + VCD bytes — would diverge from the interpreter. (Stage B: the VM
/// delegates, so this is byte-identical now; Stage C makes it the live gate.)
#[test]
fn mixed_backend_run_equals_all_interpreter() {
    let src = "module top;\n\
      reg clk;\n\
      reg [7:0] a, b;\n\
      wire [7:0] sum;\n\
      reg [7:0] q, r;\n\
      integer k;\n\
      assign sum = a + b;                                 // cont-assign: interpreted\n\
      always @(posedge clk) begin q <= sum; r <= q; end   // codegen-able: VM path\n\
      initial begin                                       // initial #1: interpreted\n\
        $dumpfile(\"x.vcd\"); $dumpvars(0, top);\n\
        clk = 0; a = 8'd10; b = 8'd20;\n\
        for (k = 0; k < 4; k = k + 1) begin #1 clk = 1; #1 clk = 0; #1 a = a + 1; end\n\
        $display(\"%0d %0d %0d\", sum, q, r); $finish;\n\
      end\n\
    endmodule";
    let ir = build(src);
    let (ri, oi, vi) = run_capture(&ir, Backend::Interpreter, "p9b_mixed");
    let (rb, ob, vb) = run_capture(&ir, Backend::Bytecode, "p9b_mixed");

    assert_eq!(oi, ob, "mixed-backend stdout must equal all-interpreter");
    assert_eq!(vi, vb, "mixed-backend VCD must equal all-interpreter");
    assert!(
        vi.as_ref().is_some_and(|v| v.len() > 32),
        "the mixed design must emit a non-trivial VCD (teeth — not a vacuous None==None)"
    );
    assert_eq!(ri.sim_time, rb.sim_time, "sim_time must match");
    assert_eq!(
        ri.finish_reason, rb.finish_reason,
        "finish_reason must match"
    );
    assert_eq!(ri.exit_class, rb.exit_class, "exit_class must match");
}

// ── C2 targeted teeth: seams the round-robin corpus does NOT exercise ──────────
// The P6 corpus runs every codegen-able body through BOTH backends (byte-identity),
// but with proc-multiplier 1 everywhere and no infinite loops — so the two pieces of
// run_process-only state the VM must reproduce itself (the cur_time_mult PROLOGUE and
// the per-activation termination GUARD, review must-fix #1/#2) are UNTESTED by it.
// These designs add that coverage with teeth.

/// Run `ir` on `backend` with explicit `proc_multipliers` + `max_deltas`, capturing
/// stdout + summary. No VCD — these teeth live in stdout / finish_reason / exit_class.
fn run_opts(
    ir: &sim_ir::SimIr,
    backend: Backend,
    mults: Vec<u64>,
    max_deltas: u64,
) -> (SimResult, String) {
    let opts = SimOpts {
        backend,
        proc_multipliers: mults,
        max_deltas,
        ..SimOpts::default()
    };
    simulate_capture(ir, opts)
}

/// [C2 PROLOGUE teeth] A codegen-able `always @(posedge clk)` body reads `$time`, run
/// under per-process-DISTINCT non-unit multipliers. `$time = now / M` where M is THIS
/// process's multiplier — set by `run_process` for the interpreter and by the VM's
/// `vm_run_body` prologue for the bytecode backend. If the VM dropped the prologue, the
/// always body would render `$time` with whatever multiplier the previously-run
/// (interpreted `initial`) process left in `cur_time_mult` — a DIFFERENT value — and the
/// backends would diverge. Distinct multipliers make the divergence guaranteed,
/// independent of which ProcId the always body received.
#[test]
fn timescale_prologue_equals_across_backends() {
    let src = "module top;\n\
      reg clk;\n\
      reg [31:0] t0;\n\
      always @(posedge clk) t0 = $time;\n\
      initial begin\n\
        clk = 0;\n\
        #5000 clk = 1; #5000 clk = 0;\n\
        #5000 $display(\"%0d\", t0); $finish;\n\
      end\n\
    endmodule";
    let ir = build(src);
    // Distinct, non-unit multiplier per process so a stale cur_time_mult on the VM path
    // can never coincidentally match the correct one.
    let mults: Vec<u64> = (0..ir.processes.len() as u64)
        .map(|i| (i + 1) * 10)
        .collect();
    let (ri, oi) = run_opts(&ir, Backend::Interpreter, mults.clone(), 1_000_000);
    let (rb, ob) = run_opts(&ir, Backend::Bytecode, mults, 1_000_000);
    assert_eq!(
        oi, ob,
        "$time (scaled by per-process multiplier) must match across backends"
    );
    assert_eq!(ri.sim_time, rb.sim_time);
    assert_eq!(ri.finish_reason, rb.finish_reason);
    // Teeth: a scaled, non-zero $time was actually printed (5000/M ∈ {500,250}).
    let v: u64 = oi
        .trim()
        .parse()
        .unwrap_or_else(|_| panic!("expected numeric $time, got {oi:?}"));
    assert!(v > 0, "scaled $time must be non-zero (teeth), got {v}");
}

/// [C2 GUARD teeth] A codegen-able body with a delay-free `forever` loops forever in ONE
/// activation. Both backends must trip the per-activation termination guard at the SAME
/// point and report the same fatal summary — `run_process` does it (exec.rs:176-180) and
/// `vm_exec` mirrors it. If the VM dropped the guard this test would HANG instead of
/// failing, so its mere existence is the teeth; a small `max_deltas` keeps it fast.
#[test]
fn runaway_codegenable_loop_equal_and_fatal() {
    let src = "module top;\n\
      reg [7:0] y;\n\
      initial forever y = y + 1;\n\
    endmodule";
    let ir = build(src);
    let unit = vec![1u64; ir.processes.len()];
    let (ri, oi) = run_opts(&ir, Backend::Interpreter, unit.clone(), 256);
    let (rb, ob) = run_opts(&ir, Backend::Bytecode, unit, 256);
    assert_eq!(oi, ob, "runaway stdout must match");
    assert_eq!(
        ri.finish_reason, rb.finish_reason,
        "finish_reason must match"
    );
    assert_eq!(ri.exit_class, rb.exit_class, "exit_class must match");
    // Teeth: the guard actually fired (fatal class), not a clean exit.
    assert_eq!(
        rb.exit_class,
        ExitClass::Fatal,
        "the per-activation guard must fire on the VM path"
    );
}

/// [C2 / P8 #3 teeth] A codegen-able body runs `a[i] = K; i = i + 1;` — the blocking LHS
/// index must be SAMPLED (`ResolveOff`) before `i` is bumped, so the write lands at the
/// OLD `i`. The compile pass emits ResolveOff-immediately-before-WriteLval and lowers
/// statements in textual order; a reorder would write `a[1]` instead of `a[0]`. Both
/// backends must agree, and the witnessed value pins the sample moment.
#[test]
fn blocking_index_sample_equals_across_backends() {
    let src = "module top;\n\
      reg clk;\n\
      reg [7:0] a [0:3];\n\
      integer i;\n\
      always @(posedge clk) begin a[i] = 8'hAB; i = i + 1; end\n\
      initial begin\n\
        i = 0; a[0] = 0; a[1] = 0; a[2] = 0; a[3] = 0;\n\
        #1 clk = 1; #1 clk = 0;\n\
        #1 $display(\"%0d %0d %0d\", a[0], a[1], i); $finish;\n\
      end\n\
    endmodule";
    let ir = build(src);
    let unit = vec![1u64; ir.processes.len()];
    let (ri, oi) = run_opts(&ir, Backend::Interpreter, unit.clone(), 1_000_000);
    let (rb, ob) = run_opts(&ir, Backend::Bytecode, unit, 1_000_000);
    assert_eq!(oi, ob, "blocking-index stdout must match across backends");
    assert_eq!(ri.finish_reason, rb.finish_reason);
    // Teeth: a[0] got 0xAB (171) via the OLD i=0; a[1] stayed 0; i bumped to 1.
    assert_eq!(oi.trim(), "171 0 1", "a[i]=K must sample i BEFORE i=i+1");
}

// ── [C4-lite] native-eval teeth: designs whose codegen-able bodies exercise the
// VM-only native expression fast path (Const/scalar Signal, +/-/* , &/|/^/~^, ~,
// unary +/-) at ≤64 bits. The native path must be byte-identical to the kernel
// tree-walk `eval_ctx` the interpreter uses — these give the P5 gate teeth ON the
// native path (the round-robin corpus does not target it specifically). Each asserts
// cross-backend identity AND a hand-computed witness so a silently-wrong native op
// (which would match neither) is caught.

/// Run `src` on both backends, assert byte-identical stdout/VCD/summary, and return
/// the (shared) stdout for an additional witness assertion.
fn assert_backends_equal(src: &str, name: &str) -> String {
    let ir = build(src);
    let (ri, oi, vi) = run_capture(&ir, Backend::Interpreter, name);
    let (rb, ob, vb) = run_capture(&ir, Backend::Bytecode, name);
    assert_eq!(oi, ob, "stdout differs across backends for `{name}`");
    assert_eq!(vi, vb, "VCD differs across backends for `{name}`");
    assert_eq!(ri.sim_time, rb.sim_time, "sim_time differs for `{name}`");
    assert_eq!(
        ri.finish_reason, rb.finish_reason,
        "finish_reason differs for `{name}`"
    );
    assert_eq!(
        ri.exit_class, rb.exit_class,
        "exit_class differs for `{name}`"
    );
    oi
}

/// A deep arithmetic chain (the EXPR_HEAVY shape) in a codegen-able always body:
/// `acc <= acc + acc + ... + 1` over a few clocks. Pure native `Add`s on 32 bits.
#[test]
fn native_arith_chain_equals_across_backends() {
    let src = "module top;\n\
      reg clk;\n\
      reg [31:0] acc;\n\
      integer k;\n\
      always @(posedge clk) acc <= acc + acc + acc + acc + 32'd1;\n\
      initial begin\n\
        acc = 32'd1; clk = 0;\n\
        for (k = 0; k < 5; k = k + 1) begin #1 clk = 1; #1 clk = 0; end\n\
        #1 $display(\"%0d\", acc); $finish;\n\
      end\n\
    endmodule";
    let out = assert_backends_equal(src, "native_arith_chain");
    // acc_{n+1} = 4*acc_n + 1, acc_0=1: 1→5→21→85→341→1365.
    assert_eq!(
        out.trim(),
        "1365",
        "native add chain must compute 4*acc+1 per clock"
    );
}

/// X/Z propagation: an uninitialised reg is all-X; any arith touching it must poison
/// the whole result to X (native `(0,unk)` poison mirroring `Value::xs`). `%0d` of an
/// all-X 8-bit value prints `x`; both backends must agree.
#[test]
fn native_arith_xz_poison_equals_across_backends() {
    let src = "module top;\n\
      reg clk;\n\
      reg [7:0] a, b, s;\n\
      always @(posedge clk) s <= a + b;\n\
      initial begin\n\
        a = 8'hxx; b = 8'd5; clk = 0;        // a is X ⇒ s must be all-X\n\
        #1 clk = 1; #1 clk = 0;\n\
        #1 $display(\"%0d %h\", s, s); $finish;\n\
      end\n\
    endmodule";
    let out = assert_backends_equal(src, "native_xz_poison");
    assert_eq!(
        out.trim(),
        "x xx",
        "X operand must poison the whole native add to X"
    );
}

/// Signed arithmetic + two's-complement negate at ≤64 bits. The native low-`w` math
/// is sign-independent at the bit level, so the printed signed value must match the
/// interpreter's signed-lane result.
#[test]
fn native_signed_arith_equals_across_backends() {
    let src = "module top;\n\
      reg clk;\n\
      reg signed [7:0] a, b, d;\n\
      always @(posedge clk) d <= a - b;\n\
      initial begin\n\
        a = -8'sd10; b = 8'sd20; clk = 0;    // -10 - 20 = -30\n\
        #1 clk = 1; #1 clk = 0;\n\
        #1 $display(\"%0d\", d); $finish;\n\
      end\n\
    endmodule";
    let out = assert_backends_equal(src, "native_signed");
    assert_eq!(
        out.trim(),
        "-30",
        "signed native sub must wrap two's-complement"
    );
}

/// 4-state bitwise + complement: mixes definite bits with X so the native `and_w/
/// or_w/xor_w/xnor_w/not_w` truth tables (not just 2-state) are exercised.
#[test]
fn native_bitwise_4state_equals_across_backends() {
    let src = "module top;\n\
      reg clk;\n\
      reg [7:0] a, b, x1, x2, x3, n;\n\
      always @(posedge clk) begin\n\
        x1 <= a & b; x2 <= a | b; x3 <= a ^ b; n <= ~a;\n\
      end\n\
      initial begin\n\
        a = 8'b1010_01xz; b = 8'b1100_0011; clk = 0;\n\
        #1 clk = 1; #1 clk = 0;\n\
        #1 $display(\"%h %h %h %h\", x1, x2, x3, n); $finish;\n\
      end\n\
    endmodule";
    // Just identity across backends — the 4-state result is whatever the oracle says;
    // the point is the native path reproduces it bit-for-bit.
    assert_backends_equal(src, "native_bitwise");
}

/// Mixed operand widths into a wider context (8-bit + 16-bit → 16-bit): exercises the
/// per-node context width/sign propagation in `try_compile` (each leaf resizes to the
/// node width via the SAME `resize_keep_sign` the oracle uses).
#[test]
fn native_mixed_width_equals_across_backends() {
    let src = "module top;\n\
      reg clk;\n\
      reg [7:0]  a;\n\
      reg [15:0] b, s;\n\
      always @(posedge clk) s <= a + b;     // 8-bit a widens into the 16-bit add\n\
      initial begin\n\
        a = 8'd200; b = 16'd1000; clk = 0;  // 200 + 1000 = 1200, no truncation\n\
        #1 clk = 1; #1 clk = 0;\n\
        #1 $display(\"%0d\", s); $finish;\n\
      end\n\
    endmodule";
    let out = assert_backends_equal(src, "native_mixed_width");
    assert_eq!(
        out.trim(),
        "1200",
        "8-bit operand must widen into the 16-bit native add"
    );
}

/// [C3 word-write] Exercise the WORD-PARALLEL net write/read fast path on a 64-bit-
/// element array (`net_w % 64 == 0` ⇒ each element is a whole store word ⇒ the aligned
/// fast path is taken) and PROVE element INDEPENDENCE: a word-granular write to one
/// element must not disturb its neighbours (the masking-clobber hazard the fast path's
/// `array_len <= 1 || net_w % 64 == 0` guard exists to avoid). Both backends must agree.
#[test]
fn word_aligned_array_write_read_equals_and_independent() {
    let src = "module top;\n\
      reg [63:0] mem [0:3];\n\
      integer i;\n\
      initial begin\n\
        for (i = 0; i < 4; i = i + 1) mem[i] = 0;\n\
        mem[0] = 64'h0000000100000002;\n\
        mem[2] = 64'h0000000300000004;\n\
        #1 mem[1] = mem[0] + mem[2];\n\
        #1 $display(\"%h %h %h %h\", mem[0], mem[1], mem[2], mem[3]);\n\
        $finish;\n\
      end\n\
    endmodule";
    let ir = build(src);
    let unit = vec![1u64; ir.processes.len()];
    let (ri, oi) = run_opts(&ir, Backend::Interpreter, unit.clone(), 1_000_000);
    let (rb, ob) = run_opts(&ir, Backend::Bytecode, unit, 1_000_000);
    assert_eq!(oi, ob, "64-bit array write/read must match across backends");
    assert_eq!(ri.finish_reason, rb.finish_reason);
    // mem[1] = mem[0]+mem[2] = 0x..0400000006; mem[0]/mem[2] retain their writes;
    // mem[3] stays 0 (the word write to its neighbours never touched it).
    assert_eq!(
        oi.trim(),
        "0000000100000002 0000000400000006 0000000300000004 0000000000000000",
        "64-bit array elements must be written/read correctly AND independently"
    );
}

/// Structural increment: bit/part selects (incl. dynamic offsets), concat and
/// replicate compiled native inside a codegen-able body — both backends must
/// produce identical bytes AND the expected oracle values.
#[test]
fn native_select_concat_repl_equals_across_backends() {
    let src = "module top;\n\
      reg clk;\n\
      reg [15:0] s;\n\
      reg [3:0] idx;\n\
      reg [7:0] p, q, r, m;\n\
      reg b0;\n\
      reg [15:0] cat, rep;\n\
      always @(posedge clk) begin\n\
        p <= s[11:4];\n\
        q <= s[idx +: 8];\n\
        r <= s[11 -: 8];\n\
        b0 <= s[5];\n\
        cat <= {p, q};\n\
        rep <= {2{p}};\n\
        m <= s[idx2 +: 8];\n\
      end\n\
      reg [3:0] idx2;\n\
      initial begin\n\
        s = 16'hA5C3; idx = 4'd4; idx2 = 4'bxxxx; clk = 0;\n\
        #1 clk = 1; #1 clk = 0;\n\
        #1 clk = 1; #1 clk = 0;\n\
        #1 $display(\"%h %h %h %b %h %h %h\", p, q, r, b0, cat, rep, m);\n\
        $finish;\n\
      end\n\
    endmodule";
    let out = assert_backends_equal(src, "native_select_concat_repl");
    // s=A5C3=1010_0101_1100_0011: [11:4]=5C, [4+:8]=5C, [11-:8]=5C, s[5]=0,
    // {p,q}=5C5C, {2{p}}=5C5C; X index ⇒ m all-X.
    assert_eq!(
        out.trim(),
        "5c 5c 5c 0 5c5c 5c5c xx",
        "native structural ops must match the oracle's select/concat/replicate"
    );
}

/// [C6] The 65..=128-bit two-word wide lane end-to-end: 100-bit unsigned
/// arith/bitwise/shift/div/compare/reduction compiled native inside a
/// codegen-able body. Witnesses hand-computed at mod 2^100.
#[test]
fn native_wide_lane_equals_across_backends() {
    let src = "module top;\n\
      reg clk;\n\
      reg [99:0] a, b, sum, dif, prd, bnd, sl, sr, dv, md;\n\
      reg lt, rx;\n\
      always @(posedge clk) begin\n\
        sum <= a + b;\n\
        dif <= a - b;\n\
        prd <= a * b;\n\
        bnd <= (a & b) ^ ~a;\n\
        sl  <= a << 37;\n\
        sr  <= a >> 65;\n\
        dv  <= a / 7;\n\
        md  <= a % 7;\n\
        lt  <= a < b;\n\
        rx  <= ^a;\n\
      end\n\
      initial begin\n\
        a = 100'habcdef0123456789abcdef012;\n\
        b = 100'h00003fffffffffffffff00001;\n\
        clk = 0;\n\
        #1 clk = 1; #1 clk = 0;\n\
        #1 $display(\"%h %h %h %h %h %h %h %h %b %b\",\n\
                    sum, dif, prd, bnd, sl, sr, dv, md, lt, rx);\n\
        $finish;\n\
      end\n\
    endmodule";
    let out = assert_backends_equal(src, "native_wide_lane");
    assert_eq!(
        out.trim(),
        "abce2f0123456789abccef013 abcdaf0123456789abceef011 \
         77c03aaaaaaaaaaabbbbef012 54323fffffffffffffff10fed \
         68acf13579bde024000000000 000000000000000055e6f7809 \
         188b2224bbe557ef188b2224b 0000000000000000000000005 0 1",
        "wide native lane must compute exact 100-bit results"
    );
}

/// [C6] Wide X-poison: any X bit (here only in the HIGH word) must poison the
/// whole wide arith result; bitwise keeps per-bit 4-state. Both backends agree.
#[test]
fn native_wide_xz_poison_equals_across_backends() {
    let src = "module top;\n\
      reg clk;\n\
      reg [99:0] a, b, s, o;\n\
      always @(posedge clk) begin\n\
        s <= a + b;\n\
        o <= a | b;\n\
      end\n\
      initial begin\n\
        a = 100'h0; a[99] = 1'bx;            // X only above bit 63\n\
        b = 100'hfffffffffffffffffffffffff;\n\
        clk = 0;\n\
        #1 clk = 1; #1 clk = 0;\n\
        #1 $display(\"%h %h\", s, o); $finish;\n\
      end\n\
    endmodule";
    let out = assert_backends_equal(src, "native_wide_xz");
    // add: all-X (25 x's); or: definite-1 everywhere (1|x = 1).
    assert_eq!(
        out.trim(),
        "xxxxxxxxxxxxxxxxxxxxxxxxx fffffffffffffffffffffffff",
        "high-word X must poison wide add; wide OR keeps definite bits"
    );
}

/// [C6] Array-indexed reads INSIDE expressions (the LoadIndexed lane): valid
/// index, X index (→ all-X read), and out-of-range index (→ all-X read), each
/// composing with native arith. Both backends agree + witness.
#[test]
fn native_indexed_read_equals_across_backends() {
    let src = "module top;\n\
      reg clk;\n\
      reg [15:0] mem [0:3];\n\
      reg [1:0] i, j;\n\
      reg [3:0] xi;\n\
      reg [15:0] q, qx, qo;\n\
      always @(posedge clk) begin\n\
        q  <= mem[i] + mem[j] * 16'd2;\n\
        qx <= mem[xi[1:0]] + 16'd1;\n\
        qo <= mem[xi] + 16'd1;\n\
      end\n\
      initial begin\n\
        mem[0] = 16'h0010; mem[1] = 16'h0200; mem[2] = 16'h3000; mem[3] = 16'h0004;\n\
        i = 2'd1; j = 2'd2; xi = 4'bxxxx; clk = 0;\n\
        #1 clk = 1; #1 clk = 0;\n\
        #1 $display(\"%h %h %h\", q, qx, qo);\n\
        xi = 4'd9;\n\
        #1 clk = 1; #1 clk = 0;\n\
        #1 $display(\"%h\", qo);\n\
        $finish;\n\
      end\n\
    endmodule";
    let out = assert_backends_equal(src, "native_indexed_read");
    // q = 0x200 + 0x3000*2 = 0x6200; X index ⇒ all-X read ⇒ X+1 = all-X;
    // then xi=9 (out of range on mem[0:3]) ⇒ all-X read again.
    assert_eq!(
        out.trim(),
        "6200 xxxx xxxx\nxxxx",
        "indexed native reads must select/poison exactly like the oracle"
    );
}

/// Phase-1.x ②: array ASSIGNMENT desugars to element-wise statements at
/// elaborate, so both backends see the same SimIr — this pins that the VM
/// executes the expanded shapes (Signal-word RHS reads, word-expr LHS chunks,
/// per-element NBAs) byte-identically in a clocked codegen-able body.
#[test]
fn array_assignment_equals_across_backends() {
    let out = assert_backends_equal(
        "module t; \
           reg clk; reg [7:0] src [0:3]; reg [7:0] dst [0:3]; \
           reg [7:0] g [0:1][0:3]; \
           integer i; \
           always @(posedge clk) begin g[1] <= src; end \
           initial begin \
             clk = 0; \
             for (i=0;i<4;i=i+1) begin src[i] = 8'h30 + i; dst[i] = 0; end \
             dst = src; \
             #1 clk = 1; #1 clk = 0; \
             $display(\"%h %h %h %h | %h %h\", dst[0], dst[1], dst[2], dst[3], \
                      g[1][0], g[1][3]); \
             $finish; \
           end \
         endmodule",
        "array_assign_parity",
    );
    assert_eq!(out.trim(), "30 31 32 33 | 30 33");
}

/// Phase-1.x ③: per-dim bounds guards lower to Ge/Le/LogAnd/Ternary around
/// the flat word — pin that OOB X-reads and no-op writes (and the E4002
/// stderr surface, which rides stdout capture here as a SimResult) are
/// byte-identical across backends.
#[test]
fn per_dim_bounds_guard_equals_across_backends() {
    let out = assert_backends_equal(
        "module t; \
           reg [7:0] g [0:1][0:2]; \
           integer i; \
           initial begin \
             g[0][0]=8'h10; g[1][2]=8'h22; \
             i = 5; \
             $display(\"r=%h\", g[0][i]); \
             g[0][i] = 8'hee; \
             i = 1; \
             $display(\"v=%h g12=%h\", g[0][i], g[1][2]); \
             $finish; \
           end \
         endmodule",
        "bounds_guard_parity",
    );
    assert_eq!(out.trim(), "r=xx\nv=xx g12=22");
}

/// v7 no-arg `$random` rides the SHARED eval kernel (the RNG cell advances
/// identically in both backends; the SEEDED form is excluded from codegen
/// like the queue pops) — pin byte-parity on a clocked draw sequence.
#[test]
fn random_draws_equal_across_backends() {
    let out = assert_backends_equal(
        "module t; \
           reg clk; integer r; \
           always @(posedge clk) r <= $random; \
           initial begin \
             clk = 0; \
             #1 clk = 1; #1 clk = 0; \
             $display(\"a=%0d\", r); \
             #1 clk = 1; #1 clk = 0; \
             $display(\"b=%0d\", r); \
             $finish; \
           end \
         endmodule",
        "random_parity",
    );
    // The Annex N default-seed sequence (iverilog-pinned in random_funcs.rs).
    assert_eq!(out.trim(), "a=303379748\nb=-1064739199");
}

/// v7 casez/casex match ops live in the SHARED comparison kernel (native-eval
/// compile-bails on CasezEq/CasexEq) — pin byte-parity on a clocked decoder
/// exercising z-wildcard hit, strict-x miss, and a casex x-wash.
#[test]
fn casez_casex_equal_across_backends() {
    let out = assert_backends_equal(
        "module t; \
           reg clk; reg [3:0] s; reg [7:0] r; \
           always @(posedge clk) begin \
             casez (s) \
               4'b1z10: r <= 8'd1; \
               4'b0x01: r <= 8'd2; \
               default: r <= 8'd9; \
             endcase \
           end \
           initial begin \
             clk = 0; s = 4'b1010; \
             #1 clk = 1; #1 clk = 0; \
             $display(\"a=%0d\", r); \
             s = 4'b0001; \
             #1 clk = 1; #1 clk = 0; \
             $display(\"b=%0d\", r); \
             s = 4'b1x10; \
             casex (s) 4'b1010: r = 8'd5; default: r = 8'd6; endcase \
             $display(\"c=%0d\", r); \
             $finish; \
           end \
         endmodule",
        "casez_parity",
    );
    // a: 1010 matches 1z10 via the label z. b: 0001 vs 0x01 is a strict miss
    // (x is not a casez wildcard) -> default 9. c: casex washes the x -> 5.
    assert_eq!(out.trim(), "a=1\nb=9\nc=5");
}

/// Phase-1.x ⑥: multi-word arithmetic lives in the SHARED eval kernel
/// (native-eval compile-bails past its lanes → EvalForLval → same kernel) —
/// pin byte-parity on a clocked body mixing 256-bit unsigned and 128-bit
/// signed arithmetic.
#[test]
fn wide_arith_equals_across_backends() {
    let out = assert_backends_equal(
        "module t; \
           reg clk; reg [255:0] acc; reg signed [127:0] s; \
           always @(posedge clk) begin \
             acc <= acc * 256'd1000003 + 256'd7; \
             s <= s - 128'sd5; \
           end \
           initial begin \
             clk = 0; acc = 256'd1; s = -128'sd2; \
             #1 clk = 1; #1 clk = 0; #1 clk = 1; #1 clk = 0; \
             $display(\"%h %0d\", acc, s); \
             $finish; \
           end \
         endmodule",
        "wide_arith_parity",
    );
    assert_eq!(
        out.trim(),
        "000000000000000000000000000000000000000000000000000000e8d56b6d65 -12"
    );
}
