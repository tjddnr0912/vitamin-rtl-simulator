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
//! The gate was wired and green BEFORE the kernel refactor (P7a/P7b) and BEFORE any
//! VM lowering, back when every body still fell back to the interpreter and it passed
//! by construction. It no longer does: Stage C landed the compiler + register VM, so
//! a corpus design whose body clears the P9 allow-list really executes on the VM here.
//! The moment such a body diverges in stdout or a single VCD byte, this test goes red
//! and names the offending design.

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
      reg [3:0] idx2;\n\
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

/// WHY COMBINATIONAL PROCESS FUSION WAS BUILT AND REVERTED (2026-08-01).
///
/// Fusing a chain of combinational processes into one activation was implemented,
/// measured at 1.7-2.5x, gated against the whole corpus — and was still WRONG. This
/// test pins the design that proves it, so the transform cannot be rebuilt without
/// meeting the counter-example.
///
/// Unfused, a depth-D combinational chain takes D deltas to propagate. A process that
/// wakes in the SAME batch and reads the chain's OUTPUT therefore samples a PARTIALLY
/// propagated value. Fused, the chain completes inside one activation, so that reader
/// sees a fully propagated one — a different value, at exit 0, with no diagnostic.
///
/// Here the flop samples `s2` at a posedge produced in the very activation that
/// initialises `seed`, so unfused it reads the stale X and `acc` is X forever. iverilog
/// 13 agrees: `acc=xxxxxxxx`. The fused build produced `acc=0000017c`.
///
/// The safety condition that was implemented guarded the chain's INTERMEDIATE nets
/// (nothing else may read them). It did not guard WHEN THE CHAIN'S OUTPUT BECOMES
/// FRESH relative to other processes in the same batch — and the reader of that output
/// is the flop, which is the entire point of a combinational cone. Requiring "no
/// concurrent reader of the chain output" empties the safe set, so fusion is not
/// semantics-preserving for a simulator whose intra-delta ordering is pinned to
/// another simulator's.
///
/// Note the stimulus: `#1 clk = 1; #1 clk = 0` does NOT expose this, because the
/// initialisation and the first edge land in different timesteps. Every corpus design
/// used that shape, which is exactly why the corpus gate passed while the transform was
/// wrong. A gate is only as good as the shapes it contains.
#[test]
fn a_comb_chain_output_is_sampled_mid_propagation() {
    let src = "module t;\n\
      reg clk = 0;\n\
      reg [31:0] seed, acc, s0, s1, s2;\n\
      integer i;\n\
      always_comb s0 = (seed ^ (seed << 1)) + 32'd1;\n\
      always_comb s1 = (s0 ^ (s0 << 1)) + 32'd2;\n\
      always_comb s2 = (s1 ^ (s1 << 1)) + 32'd3;\n\
      always @(posedge clk) begin seed <= seed + 1; acc <= acc ^ s2; end\n\
      initial begin\n\
        seed = 1; acc = 0;\n\
        for (i = 0; i < 40; i = i + 1) begin clk = ~clk; #1; end\n\
        $display(\"acc=%h\", acc);\n\
        $finish;\n\
      end\n\
    endmodule\n";
    let ir = build(src);
    for backend in [Backend::Interpreter, Backend::Bytecode] {
        let (_r, out) = simulate_capture(
            &ir,
            SimOpts {
                backend,
                ..SimOpts::default()
            },
        );
        assert!(
            out.contains("acc=xxxxxxxx"),
            "{backend:?}: the flop must sample the chain output MID-propagation and read \
             X, as iverilog 13 does. Getting a settled value here means something \
             completed the combinational chain inside one activation — which is the \
             fusion transform this test exists to keep out. Got:\n{out}"
        );
    }
}

/// `comb_depth` measures INTER-process combinational depth, and a process reading a net
/// it itself writes must not count as a dependency on itself.
///
/// `always_comb begin y = a; z = y + 1; end` reads `y` and writes `y`. Treating that as
/// a dependency makes the rank relaxation climb without bound — `rank[p] >= net_rank[y]`
/// and `net_rank[y] >= rank[p]+1` cannot both hold — so the analysis reported CYCLIC for
/// designs that have no combinational loop. Measured on PicoRV32: 4 of 43 processes do
/// this, and that alone was enough to make a RISC-V CPU look cyclic.
#[test]
fn a_process_reading_its_own_output_is_not_a_cycle() {
    // Self read+write, no inter-process chain at all: depth 0, and NOT cyclic.
    let ir = build(
        "module t;\n\
           reg [7:0] a, y, z;\n\
           always @* begin y = a; z = y + 8'd1; end\n\
           initial begin a = 8'd1; #1 $display(\"z=%0d\", z); $finish; end\n\
         endmodule",
    );
    assert!(
        sim_engine::comb_depth(&ir).is_some(),
        "a self read+write process is not a combinational cycle"
    );
    assert!(
        !sim_engine::self_read_write_processes(&ir).is_empty(),
        "teeth: this design must actually contain the shape being tested"
    );

    // A genuine two-stage chain still measures 2 — the fix must not flatten real depth.
    let chain = build(
        "module t;\n\
           reg [7:0] a, s0, s1;\n\
           always_comb s0 = a + 8'd1;\n\
           always_comb s1 = s0 + 8'd1;\n\
           initial begin a = 8'd1; #1 $display(\"s1=%0d\", s1); $finish; end\n\
         endmodule",
    );
    assert_eq!(
        sim_engine::comb_depth(&chain),
        Some(2),
        "a real two-stage combinational chain must still measure depth 2"
    );
}

/// Branch conditions are now compiled by native-eval on the VM path. Truthiness is a
/// TRI-VALUED control-flow rule — `x`/`z` takes the ELSE branch, it is not "non-zero" —
/// so the native path must route its computed value through the same `truthiness` the
/// interpreter uses rather than reimplementing the test.
///
/// Reimplementing it as `value != 0` is the obvious mistake and would be silent: every
/// X-free design keeps passing. These cases are the ones that would not.
#[test]
fn a_natively_compiled_branch_condition_keeps_the_tri_valued_rule() {
    let cases: [(&str, &str, &str); 4] = [
        // A pure-X condition takes else, even though its bit pattern is not "zero".
        ("if (x)", "xz", "else"),
        // X in ONE bit with no definite 1 anywhere: still else.
        ("if (part_x)", "part", "else"),
        // A definite 1 anywhere makes it true even with X elsewhere.
        ("if (one_and_x)", "onex", "then"),
        // Plain zero: else.
        ("if (0)", "zero", "else"),
    ];
    for (label, which, want) in cases {
        let src = format!(
            "module t;\n\
               reg [3:0] v;\n\
               reg [7:0] r;\n\
               integer k;\n\
               always @* begin\n\
                 if (v) r = 8'd1; else r = 8'd2;\n\
               end\n\
               initial begin\n\
                 case (\"{which}\")\n\
                   \"xz\":   v = 4'bxxxx;\n\
                   \"part\": v = 4'b00x0;\n\
                   \"onex\": v = 4'b01x0;\n\
                   default: v = 4'b0000;\n\
                 endcase\n\
                 #1 $display(\"r=%0d\", r);\n\
                 $finish;\n\
               end\n\
             endmodule"
        );
        let ir = build(&src);
        let want_val = if want == "then" { "r=1" } else { "r=2" };
        for backend in [Backend::Interpreter, Backend::Bytecode] {
            let (_r, out) = simulate_capture(
                &ir,
                SimOpts {
                    backend,
                    ..SimOpts::default()
                },
            );
            assert!(
                out.contains(want_val),
                "{label} on {backend:?}: expected {want_val} (x/z takes else; a definite 1 \
                 anywhere is true) — got:\n{out}"
            );
        }
    }
}

/// The native-eval LEAF fast path must equal the read-then-resize it replaces.
///
/// `read_scalar_words` reproduces `read_net(net, None).resize_keep_sign(w, signed)`
/// without building either `Value`. Getting it subtly wrong is invisible on ordinary
/// values and shows only on sign extension or on an x in the sign bit, so this sweeps a
/// design over signed/unsigned nets of several widths read into several context widths,
/// with X present, on BOTH backends — the interpreter is the oracle.
#[test]
fn leaf_fast_path_matches_read_net() {
    for (decl, init) in [
        ("reg signed [7:0]  a", "8'sh80"), // sign bit set
        ("reg signed [7:0]  a", "8'sh7f"),
        ("reg signed [3:0]  a", "4'sb1x0x"), // x beside the sign bit
        ("reg        [7:0]  a", "8'hff"),
        ("reg        [31:0] a", "32'hdead_beef"),
        ("reg signed [31:0] a", "-32'sd12345"),
        ("reg        [1:0]  a", "2'bx1"),
        ("reg signed [63:0] a", "-64'sd1"),
    ] {
        // Read `a` into several context widths, signed and unsigned, inside a
        // codegen-able body so the native path is the one exercised.
        let src = format!(
            "module t;\n\
               {decl};\n\
               reg [63:0] w64; reg [15:0] w16; reg [3:0] w4;\n\
               reg signed [63:0] s64; reg signed [15:0] s16;\n\
               reg clk = 0;\n\
               integer i;\n\
               always @(posedge clk) begin\n\
                 w64 = a; w16 = a; w4 = a; s64 = a; s16 = a;\n\
               end\n\
               initial begin\n\
                 a = {init};\n\
                 for (i = 0; i < 3; i = i + 1) begin #1 clk = ~clk; end\n\
                 $display(\"%h %h %h %h %h\", w64, w16, w4, s64, s16);\n\
                 $finish;\n\
               end\n\
             endmodule"
        );
        let ir = build(&src);
        let (_ri, oi) = simulate_capture(
            &ir,
            SimOpts {
                backend: Backend::Interpreter,
                ..SimOpts::default()
            },
        );
        let (_rb, ob) = simulate_capture(
            &ir,
            SimOpts {
                backend: Backend::Bytecode,
                ..SimOpts::default()
            },
        );
        assert_eq!(
            oi, ob,
            "leaf fast path diverged for `{decl} = {init}` — the VM must read exactly \
             what read_net + resize_keep_sign produce"
        );
        assert!(
            oi.split_whitespace().count() >= 5,
            "teeth: the design must actually have printed five values, got:\n{oi}"
        );
    }
}

/// Shapes the GENERATED corpus cannot produce — pinned by hand, both backends.
///
/// The corpus above is nine parameterised templates of arithmetic, clocking and
/// structure. That is a fine sweep of the expression lowering and a poor sweep of
/// everything else, and "72 designs agree" reads like far broader coverage than it is.
/// Running the whole workspace suite with the backend default flipped to `Bytecode`
/// turned up 39 failures across 18 targets that this gate was green through — every one
/// of them a shape no template emits.
///
/// Two roots, both landed with this test:
///
/// 1. NON-INTEGRAL VALUES. A native program's register is a `(val, unk)` word pair, so
///    it cannot carry `is_real`/`is_str`/a heap handle, and `try_compile` had no type
///    test — only a width test. A real read delivered its raw IEEE-754 bits as an
///    integer; a `string` destination has net width 0, so `lvalue_width().max(1)` handed
///    the compiler a ONE-BIT context and `"a"` (0x61) became the single bit 1.
/// 2. THE PER-BODY PROLOGUE. `vm_run_body` carried a hand-copied excerpt of
///    `run_process`'s prologue that set `cur_time_mult` and neither `cur_prec_mult` nor
///    the `%m` scope — so a submodule `$display("%m")` rendered whatever scope ran last.
///
/// Each entry is a shape, not a regression note: keep them here so the gate stays
/// non-vacuous for the class rather than for the four bugs that revealed it.
///
/// `cur_prec_mult`, the prologue's third field, is pinned in `cli/tests/backend_flag.rs`
/// instead: it needs per-module `timescale`, and this harness calls `elaborate` directly
/// without the preprocessor, so the directive would not even parse here.
const HAND_SHAPES: &[(&str, &str)] = &[
    (
        "real_and_string_values",
        "module t;\n\
           real r = 2.75, q;\n\
           string s = \"hi\", s2;\n\
           int i;\n\
           initial begin\n\
             q = r;   $display(\"q=%0f\", q);\n\
             i = r;   $display(\"i=%0d\", i);\n\
             s2 = s;  $display(\"s2=[%s]\", s2);\n\
           end\n\
         endmodule\n",
    ),
    (
        "real_continuous_assign",
        "module t;\n\
           real r = 2.75;\n\
           wire [63:0] w;\n\
           assign w = r;\n\
           initial #1 $display(\"w=%0d\", w);\n\
         endmodule\n",
    ),
    (
        "submodule_percent_m",
        "module sub;\n\
           initial $display(\"in %m\");\n\
         endmodule\n\
         module t;\n\
           sub u1();\n\
           initial begin $display(\"at %m\"); #1 $finish; end\n\
         endmodule\n",
    ),
];

#[test]
fn hand_written_shapes_agree_across_backends() {
    for (name, src) in HAND_SHAPES {
        let ir = build(src);
        let (ri, oi, _) = run_capture(&ir, Backend::Interpreter, name);
        let (rb, ob, _) = run_capture(&ir, Backend::Bytecode, name);
        assert_eq!(oi, ob, "stdout differs across backends for `{name}`");
        assert_eq!(ri.sim_time, rb.sim_time, "sim_time differs for `{name}`");
        assert_eq!(
            ri.finish_reason, rb.finish_reason,
            "finish_reason differs for `{name}`"
        );
        // Not vacuous: each shape must actually have printed something.
        assert!(!oi.trim().is_empty(), "`{name}` produced no transcript");
    }
}

/// ⭐⭐ **B1: the suite is byte-identical under EITHER default, and exactly three
/// tests notice which one is set.**
///
/// This is the claim the flip is actually defensible on, and it is a property of
/// the TEST SUITE rather than of the product — so it is written down here rather
/// than only measured once and forgotten.
///
/// Measured 2026-08-16, both directions, whole workspace:
///
/// | default | result |
/// |---|---|
/// | `Native` (shipped) | 5,469 pass |
/// | `Bytecode` (reverse flip) | 5,466 pass, 3 fail |
///
/// and the three are the same three either way — this test,
/// `cli::obs::run_json_codegen_pins_the_vm_claim_and_reasons` and
/// `cli::obs::run_json_codegen_is_backend_invariant_and_backend_is_recorded`.
///
/// ⚠️ **The reverse direction is the one that matters now.** Before B1 the flip
/// run asked "does native agree with the default?"; after it, the default IS
/// native, so the same question has to be asked the other way or the suite
/// quietly becomes native-only and the oracle stops being exercised. That
/// obligation is recorded on `Backend::Bytecode` and is why the VM's own
/// differentials must keep naming their backend explicitly.
///
/// This function cannot run the flip itself (the default is a compile-time
/// constant), so what it pins is the INVARIANT the flip rests on: the two
/// spellings agree, and every other test in the workspace names its backend.
///
/// ---
///
/// The default backend IS tier-3 native (Phase B1, 2026-08-16).
///
/// Pinned as a VALUE, not inferred from behaviour: every differential in this file and
/// in `cli/tests/backend_flag.rs` names both backends explicitly precisely so it cannot
/// notice the default moving. Something has to, and this is it.
///
/// ⚠️ **BOTH spellings, and that is the whole point of the second assertion.**
/// §4.5.336 measured that `SimOpts::default()` hardcoded a backend instead of
/// deriving it, so flipping the enum's `#[default]` alone moved only the CLI half
/// of the suite and the census came out wrong. They can silently disagree again.
///
/// The flip is defensible on two measurements. Coverage: Phase A closed every
/// reachable gate row, so the census is 6,470/6,470 with zero refusals — this
/// default routes nobody silently elsewhere. Equivalence: the whole workspace
/// suite passes with `Native` in this slot, which is the gate that has
/// historically found what the corpus differential could not (the corpus was
/// green through 39 failures across 18 targets in §4.5.279; the suite was not).
#[test]
fn the_default_backend_is_native() {
    assert_eq!(SimOpts::default().backend, Backend::Native);
    assert_eq!(Backend::default(), Backend::Native);
}
