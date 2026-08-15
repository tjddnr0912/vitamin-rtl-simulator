//! S3a gate — **subroutine calls, both backends, byte for byte.**
//!
//! The P6 corpus contains exactly ZERO subroutine designs (measured: 72 designs,
//! `func_table` empty on every one), so unlike every earlier tier-3 slice this
//! one gets no coverage from the shared corpus at all. Everything here is a
//! dedicated design, and the set is chosen by the two questions the slice
//! actually turns on:
//!
//! 1. **Does the delegation reproduce the frame executor's behaviour?** The
//!    calls go to `SimState::run_frame_call`, the same function the engine runs
//!    — so what has to be shown is that the CALLER side (argument evaluation,
//!    the returned value, the diagnostic and fatal channels, the exit class)
//!    crosses the store boundary intact.
//! 2. **Does every admitted POSITION really evaluate through the composite
//!    reader?** A call in an lvalue index, a branch condition, a delay amount, a
//!    `wait` predicate, a continuous assign, an NBA rhs — each one is a
//!    different funnel, and the ones that are NOT admitted have to refuse rather
//!    than answer X.
//!
//! `agree` (run_tests) is the comparison: interleaved stdout+diagnostics, finish
//! reason, end time, exit class, and VCD bytes — with the anti-vacuity check
//! that the native backend actually ran, since a refused design would otherwise
//! compare the VM against itself.

use crate::native::arena::NetArena;
use crate::SimOpts;

use super::run_tests::agree;
use super::tests::build_with_opts;

/// The designs the delegation has to reproduce. Named, because a failure names
/// the shape rather than an index.
fn frame_designs() -> Vec<(&'static str, String)> {
    let d = |n: &'static str, s: &str| (n, s.to_string());
    vec![
        d(
            "plain call",
            r#"
module top;
  function automatic integer inc(input integer x);
    integer loc;
    begin loc = x + 1; inc = loc; end
  endfunction
  integer r;
  initial begin r = inc(3); $display("r=%0d", r); $finish; end
endmodule
"#,
        ),
        d(
            "recursion",
            r#"
module top;
  function automatic integer fact(input integer n);
    begin if (n <= 1) fact = 1; else fact = n * fact(n-1); end
  endfunction
  integer r;
  initial begin r = fact(10); $display("fact=%0d", r); $finish; end
endmodule
"#,
        ),
        d(
            "nested distinct callees",
            r#"
module top;
  function automatic integer a(input integer x); begin a = x + 1; end endfunction
  function automatic integer b(input integer x); begin b = a(x) * 2; end endfunction
  function automatic integer c(input integer x); begin c = b(x) + a(x); end endfunction
  integer r;
  initial begin r = c(5); $display("r=%0d", r); $finish; end
endmodule
"#,
        ),
        d(
            // `formal_width` is one of the three `NetReader` methods the tier-3
            // kernel delegates to the engine, and it is the one with a VALUE
            // consequence: IEEE 1800 §13.4.3 sizes each actual to the FORMAL's
            // type before the call, so a 16-bit actual passed to an 8-bit formal
            // must truncate. Without the delegation the arena's default returns
            // `None` and the arm falls back to the actual's self-width — a
            // silently WIDER argument.
            //
            // The discriminator is an ACTUAL that is an EXPRESSION and a formal
            // WIDER than it: `a + b` must be evaluated at the formal's 16 bits
            // (`r=0101`), not at its own 8 (`r=0001`). A plain wider-actual
            // shape does NOT discriminate — the bind loop's
            // `resize_keep_sign` truncates it anyway, measured.
            "formal-width binding of an expression actual",
            r#"
module top;
  function automatic [15:0] widen(input [15:0] v); begin widen = v; end endfunction
  function automatic integer sgn(input signed [15:0] v); begin sgn = v; end endfunction
  reg [7:0] a = 8'hFF, b = 8'h02;
  reg [15:0] r; integer s;
  initial begin r = widen(a + b); s = sgn(a); $display("r=%h s=%0d", r, s); $finish; end
endmodule
"#,
        ),
        d(
            // NOT `automatic`: the frame slots live in the persistent static SLAB
            // (`func_has_static`), a different storage branch in `run_frame_call`.
            //
            // ⚠️ An earlier version of this file claimed a non-`automatic` module
            // function is INLINED and that the slab path is therefore unreachable.
            // That was measured FALSE by the adversarial review: elaborate's
            // `body_needs_frame` frames a subroutine whose body has ANY control
            // flow, so the `if` below is what makes this a frame at all. My probe
            // missed it because its body was straight-line — which sent the local
            // down the flattened block-local path instead, where a read before a
            // definite assignment is a loud E3010 and looked like the whole family
            // was unsupported.
            "static lifetime persists across calls",
            r#"
module top;
  function integer accum(input integer x);
    integer acc;
    begin
      if (x == 0) acc = 0;
      acc = acc + x;
      accum = acc;
    end
  endfunction
  integer r, i;
  initial begin
    for (i = 0; i < 5; i = i + 1) begin r = accum(i); $display("i=%0d r=%0d", i, r); end
    $finish;
  end
endmodule
"#,
        ),
        d(
            // …and recursion on the SLAB, where each level shares one slot rather
            // than getting its own window (the iverilog-faithful static-lifetime
            // model — `FuncMeta::is_automatic`'s doc).
            "non-automatic recursion (shared slab)",
            r#"
module top;
  function integer fact(input integer n);
    begin if (n <= 1) fact = 1; else fact = n * fact(n-1); end
  endfunction
  integer r;
  initial begin r = fact(5); $display("fact=%0d", r); $finish; end
endmodule
"#,
        ),
        d(
            "2-state local defaults to 0, 4-state to x",
            r#"
module top;
  function automatic integer two(input integer x);
    int    i2;
    begin two = x + i2; end
  endfunction
  function automatic integer four(input integer x);
    integer i4;
    begin four = x + i4; end
  endfunction
  integer a, b;
  initial begin a = two(1); b = four(1); $display("two=%0d four=%0d", a, b); $finish; end
endmodule
"#,
        ),
        d(
            "wide return (>64 bits)",
            r#"
module top;
  function automatic [127:0] rot(input [127:0] v, input integer n);
    begin rot = (v << n) | (v >> (128 - n)); end
  endfunction
  reg [127:0] r;
  initial begin r = rot(128'hdeadbeef_cafebabe_01234567_89abcdef, 33);
    $display("r=%h", r); $finish; end
endmodule
"#,
        ),
        d(
            "signed formal and signed return",
            r#"
module top;
  function automatic signed [7:0] neg(input signed [7:0] v);
    begin neg = -v; end
  endfunction
  reg signed [7:0] r;
  integer i;
  initial begin
    for (i = -3; i < 4; i = i + 1) begin r = neg(i[7:0]); $display("i=%0d r=%0d", i, r); end
    $finish;
  end
endmodule
"#,
        ),
        d(
            "x/z actual survives the frame",
            r#"
module top;
  function automatic [3:0] thru(input [3:0] v); begin thru = v ^ 4'b0000; end endfunction
  reg [3:0] r;
  initial begin r = thru(4'b01xz); $display("a=%b", r);
    r = thru(4'bzzzz); $display("b=%b", r); $finish; end
endmodule
"#,
        ),
        d(
            "frame-local unpacked array (keccak_f_arr's shape)",
            r#"
module top;
  function automatic integer lut(input integer i);
    integer tb [0:7];
    begin
      tb[0]=5; tb[1]=9; tb[2]=13; tb[3]=17; tb[4]=21; tb[5]=25; tb[6]=29; tb[7]=33;
      lut = tb[i];
    end
  endfunction
  integer i, s;
  initial begin s = 0; for (i=0;i<8;i=i+1) s = s + lut(i); $display("s=%0d", s); $finish; end
endmodule
"#,
        ),
        d(
            "call in an lvalue index, a branch cond and a delay amount",
            r#"
module top;
  function automatic integer f(input integer x); begin f = x + 1; end endfunction
  reg [7:0] mem [0:7];
  integer i;
  initial begin
    for (i = 0; i < 8; i = i + 1) mem[i] = 8'd0;
    mem[f(2)] = 8'hAA;
    if (f(0) == 1) $display("branch taken");
    #(f(4)) $display("t=%0t mem3=%h", $time, mem[3]);
    $finish;
  end
endmodule
"#,
        ),
        d(
            "call in a zero-delay continuous assign",
            r#"
module top;
  function automatic [7:0] inv(input [7:0] v); begin inv = ~v; end endfunction
  reg [7:0] a = 8'h0F;
  wire [7:0] y;
  assign y = inv(a);
  initial begin #1 $display("y=%h", y); a = 8'hF0; #1 $display("y=%h", y); $finish; end
endmodule
"#,
        ),
        d(
            "call in an NBA rhs across clock edges",
            r#"
module top;
  function automatic integer f(input integer x); begin f = x * 3; end endfunction
  reg clk = 1'b0; integer q = 0;
  always #1 clk = ~clk;
  always @(posedge clk) q <= f(q) + 1;
  initial begin #10 $display("q=%0d", q); $finish; end
endmodule
"#,
        ),
        d(
            "call inside a wait predicate",
            r#"
module top;
  function automatic integer f(input integer x); begin f = x; end endfunction
  reg [7:0] n = 8'd0;
  initial #1 n = 8'd5;
  initial begin wait (f(n) == 5); $display("woke n=%0d t=%0t", n, $time); $finish; end
endmodule
"#,
        ),
        d(
            "call in a ternary and in a concat",
            r#"
module top;
  function automatic [3:0] f(input [3:0] x); begin f = x + 4'd1; end endfunction
  reg [7:0] r; reg [3:0] a = 4'd5;
  initial begin
    r = {f(a), (a[0] ? f(a) : f(4'd0))};
    $display("r=%h", r); $finish;
  end
endmodule
"#,
        ),
        d(
            "$display inside the function body",
            r#"
module top;
  function automatic integer dbl(input integer x);
    begin $display("in dbl x=%0d", x); dbl = x * 2; end
  endfunction
  integer r;
  initial begin r = dbl(21); $display("r=%0d", r); $finish; end
endmodule
"#,
        ),
        d(
            "$error inside the function body latches the exit class",
            r#"
module top;
  function automatic integer chk(input integer x);
    begin if (x < 0) $error("negative %0d", x); chk = x; end
  endfunction
  integer r;
  initial begin r = chk(-5); $display("r=%0d", r); $finish; end
endmodule
"#,
        ),
        d(
            "$fatal inside the function body stops the body",
            r#"
module top;
  function automatic integer boom(input integer x);
    begin if (x == 3) $fatal(1, "boom at %0d", x); boom = x; end
  endfunction
  integer r;
  initial begin
    r = boom(1); $display("a=%0d", r);
    r = boom(3); $display("b=%0d", r);
    $finish;
  end
endmodule
"#,
        ),
        d(
            "runaway recursion hits the depth limit",
            r#"
module top;
  function automatic integer inf(input integer n); begin inf = inf(n + 1); end endfunction
  integer r;
  initial begin r = inf(0); $display("r=%0d", r); $finish; end
endmodule
"#,
        ),
        d(
            "the same function from two processes in one delta",
            r#"
module top;
  function automatic integer f(input integer x); begin f = x * x; end endfunction
  integer a = 0, b = 0;
  reg go = 1'b0;
  initial begin #1 go = 1'b1; #2 $display("a=%0d b=%0d", a, b); $finish; end
  always @(posedge go) a = f(4);
  always @(posedge go) b = f(5);
endmodule
"#,
        ),
        d(
            "a call whose argument reads an out-of-range memory word",
            r#"
module top;
  function automatic integer f(input integer x); begin f = x + 1; end endfunction
  reg [7:0] mem [0:3];
  integer r, i;
  initial begin
    for (i = 0; i < 4; i = i + 1) mem[i] = i[7:0];
    i = 9;
    r = f(mem[i]);
    $display("r=%0d", r);
    $finish;
  end
endmodule
"#,
        ),
        d(
            // ⚠️ **The ORDER, not the value.** An out-of-range read in a call
            // ARGUMENT is counted by the arena and reported by whoever holds the
            // sink; the engine reports at the access. Without the drain in
            // `eval_call` the callee's `$display` comes out BEFORE the E4002 its
            // own argument earned, while the VM emits them the other way round —
            // same values, same exit class, a different stream. `agree` compares
            // the interleaved stream, which is the only reason this is visible.
            // (Found by the S3a soundness review; the design above with a SILENT
            // callee cannot distinguish it.)
            "out-of-range call argument with a printing callee",
            r#"
module top;
  function automatic integer f(input integer x);
    begin $display("in f x=%0d", x); f = x + 1; end
  endfunction
  reg [7:0] mem [0:3];
  integer r, i;
  initial begin
    for (i = 0; i < 4; i = i + 1) mem[i] = i[7:0];
    i = 9;
    r = f(mem[i]);
    $display("r=%0d", r);
    $finish;
  end
endmodule
"#,
        ),
        d(
            // …and the same order with a SEVERITY in the callee, which reaches
            // the sink through `frame_emit_severity` rather than the formatter.
            "out-of-range call argument with a severity in the callee",
            r#"
module top;
  function automatic integer f(input integer x);
    begin if (x !== 0) $error("saw %0d", x); f = x + 1; end
  endfunction
  reg [7:0] mem [0:3];
  integer r, i;
  initial begin
    for (i = 0; i < 4; i = i + 1) mem[i] = i[7:0];
    i = 9;
    r = f(mem[i]);
    $display("r=%0d", r);
    $finish;
  end
endmodule
"#,
        ),
        d(
            "two instances of one module, each with the same function",
            r#"
module leaf(input integer a, output integer y);
  function automatic integer bump(input integer x);
    integer t;
    begin t = x + 1; bump = t; end
  endfunction
  always @(*) y = bump(a);
endmodule
module top;
  integer a1 = 1, a2 = 10;
  integer y1, y2;
  leaf u1(.a(a1), .y(y1));
  leaf u2(.a(a2), .y(y2));
  initial begin
    #1 $display("y1=%0d y2=%0d", y1, y2);
    a1 = 5;
    #1 $display("y1=%0d y2=%0d", y1, y2);
    $finish;
  end
endmodule
"#,
        ),
        d(
            "call under $dumpvars (the waveform half of the gate)",
            r#"
module top;
  function automatic [7:0] f(input [7:0] x); begin f = x + 8'd1; end endfunction
  reg [7:0] q = 8'd0;
  reg clk = 1'b0;
  always #1 clk = ~clk;
  always @(posedge clk) q <= f(q);
  initial begin $dumpfile("s3a.vcd"); $dumpvars(0, top); #9 $display("q=%0d", q); $finish; end
endmodule
"#,
        ),
        d(
            // `formal_is_string` — the third delegated `NetReader` method, and
            // the one whose default is silently WRONG rather than absent: a
            // `string` formal lowers to a ONE-BIT `Wire` net, so without the
            // mask the literal actual binds at the formal's 1-bit width and the
            // text is truncated to nothing. Reachable here precisely because the
            // actual is a LITERAL (a packed const): a string VARIABLE would be a
            // `NetKind::String` net, which the S0 `string` row refuses.
            "string formal bound from a literal actual",
            r#"
module top;
  function automatic integer slen(input string s); begin slen = s.len(); end endfunction
  integer r;
  initial begin r = slen("hello"); $display("r=%0d", r); $finish; end
endmodule
"#,
        ),
        d(
            // `k_sformatf` renders through the format engine with the KERNEL as
            // reader, not the bare arena — the one seam whose fix was possible
            // (both are shared reborrows) where `k_dispatch_systask`'s was not.
            // The destination is PACKED, not a `string` net, or the S0 `string`
            // row would refuse the design and this would never run.
            "$sformatf with a call in its arguments",
            r#"
module top;
  function automatic integer f(input integer x); begin f = x + 1; end endfunction
  reg [63:0] p;
  initial begin p = $sformatf("%0d", f(4)); $display("p=%0s", p); $finish; end
endmodule
"#,
        ),
        d(
            // …and the same seam with an OUT-OF-RANGE read inside it. The format
            // engine drains the reader's deferred range reports so the E4002
            // lands BEFORE the line it was read for; the kernel therefore has to
            // forward `take_deferred_range_reports` to the arena. Without it the
            // report survives to the statement boundary and comes out after.
            // (iverilog refuses a packed `$sformatf` destination, so this pair is
            // an internal differential — the ORDER is what it pins.)
            "$sformatf over an out-of-range read",
            r#"
module top;
  function automatic integer f(input integer x); begin f = x + 1; end endfunction
  reg [7:0] mem [0:3];
  reg [63:0] p;
  integer i;
  initial begin
    for (i = 0; i < 4; i = i + 1) mem[i] = i[7:0];
    i = 9;
    p = $sformatf("v=%0d", f(mem[i]));
    $display("p=%0s", p);
    $finish;
  end
endmodule
"#,
        ),
        d(
            "two functions with adjacent frame windows, interleaved",
            r#"
module top;
  function automatic integer lo(input integer x); integer t; begin t = x - 1; lo = t; end endfunction
  function automatic integer hi(input integer x); integer t; begin t = x + 1; hi = t; end endfunction
  integer i, s;
  initial begin
    s = 0;
    for (i = 0; i < 6; i = i + 1) s = s + (i[0] ? hi(i) : lo(i));
    $display("s=%0d", s); $finish;
  end
endmodule
"#,
        ),
        d(
            "a call in a multi-driven net's driver",
            r#"
module top;
  function automatic [3:0] f(input [3:0] x); begin f = x | 4'b0001; end endfunction
  reg [3:0] a = 4'b0100, b = 4'b0010;
  wire [3:0] y;
  assign y = f(a);
  assign y = f(b);
  initial begin #1 $display("y=%b", y); $finish; end
endmodule
"#,
        ),
        d(
            "call inside a task-free always_comb",
            r#"
module top;
  function automatic [7:0] f(input [7:0] x); begin f = x ^ 8'hA5; end endfunction
  reg  [7:0] a = 8'h00;
  reg  [7:0] y;
  always @(*) y = f(a);
  initial begin #1 $display("y=%h", y); a = 8'hFF; #1 $display("y=%h", y); $finish; end
endmodule
"#,
        ),
    ]
}

/// The differential: every design above must run NATIVELY and agree with the VM
/// on every observable.
///
/// ⚠️ **On a 256 MiB worker stack**, for the reason `MAX_CALL_DEPTH`'s doc gives:
/// `run_frame_call` recurses natively on a nested `Expr::Call`, so the runaway
/// design's guard is the depth CAP only if the cap is reached before the host
/// stack is. The CLI driver spawns exactly such a thread (`cli::pipeline`), and
/// `frame_call_util::on_big_stack` is the engine-side twin — measured: without
/// it this test aborts with SIGABRT on the default test stack, which would have
/// read as a tier-3 defect rather than as a harness one.
#[test]
fn s3a_frame_calls_match_the_vm() {
    std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(|| {
            let designs = frame_designs();
            assert_eq!(designs.len(), 32, "the S3a design set shrank");
            for (name, src) in &designs {
                agree(src, name)
                    .unwrap_or_else(|r| panic!("{name}: must be runnable, refused: {r}"));
            }
        })
        .expect("spawn big-stack worker")
        .join()
        .expect("big-stack worker panicked");
}

/// **Anti-vacuity for the set itself**: every design must actually carry a frame
/// table, or `agree` would be comparing two backends on a design with no calls
/// in it — which is what the 72-design corpus already does.
#[test]
fn s3a_every_design_actually_frames_a_subroutine() {
    for (name, src) in frame_designs() {
        let (_, opts) = build_with_opts(&src);
        assert!(
            !opts.func_table.is_empty(),
            "{name}: no frame table — this design proves nothing about S3a"
        );
    }
}

/// V1 slice 3a: a CALL in a system-task argument is admitted, and the three
/// shapes that used to prove its refusal row now prove the opposite.
///
/// Written as its own case rather than as a deletion from the table below, for
/// the reason this repository keeps restating: a shrinking vector shows that a
/// key left, not that the shape RUNS. The differential rows in `run_tests.rs`
/// prove the values; this proves the gate, which is the half that decides
/// whether those rows are vacuous.
#[test]
fn a_call_in_a_system_task_argument_is_admitted() {
    for (name, src) in [
        (
            "bare",
            "module top;\n  function automatic integer f(input integer x); begin f = x + 1; end endfunction\n  initial begin $display(\"f=%0d\", f(4)); $finish; end\nendmodule\n",
        ),
        (
            "under an operator",
            "module top;\n  function automatic integer f(input integer x); begin f = x + 1; end endfunction\n  initial begin $display(\"v=%0d\", f(1) + 1); $finish; end\nendmodule\n",
        ),
        (
            "under a ternary inside a concat",
            "module top;\n  function automatic [3:0] f(input [3:0] x); begin f = x + 4'd1; end endfunction\n  reg [3:0] a = 4'd5;\n  initial begin $display(\"v=%h\", {a, (a[0] ? f(a) : 4'd0)}); $finish; end\nendmodule\n",
        ),
    ] {
        let (ir, opts) = build_with_opts(src);
        assert!(
            !opts.func_table.is_empty(),
            "{name}: no frame table — this design proves nothing"
        );
        assert_eq!(
            crate::native::frames::frames_admitted(&ir, &opts).err(),
            None,
            "{name}: the frames gate must admit a call in a system-task argument"
        );
    }
}

/// Every REFUSAL ROW of `frames_admitted` must be reached by a design, and the
/// design must be one the delegation would MIS-handle if the row were removed.
///
/// The remaining `S3b` position row is the sharpest: without it the evaluation
/// reaches `NetArena::eval_call`, which panics — so deleting it turns this gate
/// red loudly rather than quietly.
#[test]
fn s3a_each_frame_refusal_row_has_a_design() {
    let cases: Vec<(&str, &str, &str)> = vec![
        // ⚠️ This row has moved TWICE, and both times because the walk grew an arm
        // rather than because anything about the row was wrong.
        //   * before A3-i it refused EVERY task;
        //   * A3-i left it refusing every SUSPENDABLE task — and the designs here
        //     were `$display`-in-a-task and write-a-module-net-from-a-task;
        //   * A3-ii-a runs both of those, because "suspendable" names the engine's
        //     EXECUTOR CHOICE and neither of them PARKS.
        // What is left is the actual suspension, and it needs its own designs.
        // ⚠️⚠️ A3-ii-b REMOVED three cases from here — "a task frame that
        // delays", "a task frame whose CALLEE delays" and "a task frame that
        // waits on an edge" — because all three RUN now. The row they pinned
        // narrowed from "delay, wait or fork" to fork alone.
        //
        // And the fork half cannot replace them: a design with a `fork` in a
        // task trips the DESIGN gate's `fork` row, which this table's second
        // assertion explicitly forbids ("the DESIGN gate must accept it, or an
        // earlier layer is doing the work"). So the storage row's remaining
        // population is not reachable in isolation — measured, not assumed, and
        // the same shape §5.1-v found for `has_hier_call`. The row stays
        // fail-closed; what those three designs now assert lives in
        // `cli::frame_park` as positive, iverilog-pinned coverage.
        // ⚠️ A3-iii turned this from a READ into a WRITE, because a read is no
        // longer refused: the delegated executor takes the caller's store now.
        // A class-field write is the shape that still reaches this row — a plain
        // `g = …` on a module net is refused a phase EARLIER (elaborate E3009),
        // so building it the obvious way would give a test that passes because
        // elaborate refused it.
        (
            "subroutine writes a module-scope class field",
            "a subroutine that WRITES a net outside its own frame: S3b",
            r#"
module top;
  class C; int v; endclass
  C c;
  function automatic integer addg(input integer x); begin c.v = x; addg = x; end endfunction
  integer r;
  initial begin c = new(); r = addg(3); $display("r=%0d", r); $finish; end
endmodule
"#,
        ),
        (
            "call in a delayed continuous assign",
            "a call in a delayed continuous assign: S3b",
            r#"
module top;
  function automatic [7:0] f(input [7:0] x); begin f = ~x; end endfunction
  reg [7:0] a = 8'h0F;
  wire [7:0] y;
  assign #1 y = f(a);
  initial begin #3 $display("y=%h", y); $finish; end
endmodule
"#,
        ),
        // ── the six below discriminate the WALK, not the row. Each was a
        // surviving mutation until it existed: with only the designs above, the
        // walk could stop descending into an `else` arm, into a loop body, into
        // any interior expression node, or into all but the first frame window,
        // and every test still passed. (Shapes from the S3a differential review.)
        (
            "module-scope write only in the ELSE arm",
            "a subroutine that WRITES a net outside its own frame: S3b",
            r#"
module top;
  class C; int v; endclass
  C c;
  function automatic integer f(input integer x);
    begin if (x > 100) f = x; else begin c.v = x; f = x; end end
  endfunction
  integer r;
  initial begin c = new(); r = f(3); $display("r=%0d", r); $finish; end
endmodule
"#,
        ),
        (
            "module-scope write only inside a loop body",
            "a subroutine that WRITES a net outside its own frame: S3b",
            r#"
module top;
  class C; int v; endclass
  C c;
  function automatic integer f(input integer n);
    integer i, s;
    begin s = 0; for (i = 0; i < n; i = i + 1) c.v = i; f = s; end
  endfunction
  integer r;
  initial begin c = new(); r = f(3); $display("r=%0d", r); $finish; end
endmodule
"#,
        ),
        (
            // Kills dropping the `n < base_net` half of the containment test.
            //
            // ⚠️ This design used to declare `gg` AFTER the function, and said so:
            // "its NetId is ABOVE the frame window". That was not legal
            // SystemVerilog — a variable may not be used above its declaration
            // (IEEE 1800 §6.10), and iverilog rejects it with "Check for
            // declaration after use". vita accepted it silently until the aes_top
            // report, so this design was riding a gap. Declaration order does not
            // in fact move the NetId either: module nets are all created in pass 4,
            // frame nets in pass 6.5, so a module net is ALWAYS below the window
            // and the `n < base_net` half is killed by any out-of-frame read.
            "module-scope write by a function declared before it",
            "a subroutine that WRITES a net outside its own frame: S3b",
            r#"
module top;
  class C; int v; endclass
  C cc;
  function automatic integer f(input integer x); begin cc.v = x; f = x; end endfunction
  integer r;
  initial begin cc = new(); r = f(3); $display("r=%0d", r); $finish; end
endmodule
"#,
        ),
        (
            // The call is in the delayed assign's LVALUE INDEX, not its rhs.
            "call in a delayed continuous assign's lvalue index",
            "a call in a delayed continuous assign: S3b",
            r#"
module top;
  function automatic integer f(input integer x); begin f = x; end endfunction
  reg [7:0] a = 8'h0F;
  wire [7:0] y [0:3];
  assign #1 y[f(2)] = a;
  initial begin #3 $display("y2=%h", y[2]); $finish; end
endmodule
"#,
        ),
    ];
    for (what, row, src) in &cases {
        let (ir, opts) = build_with_opts(src);
        assert!(
            !opts.func_table.is_empty(),
            "{what}: no frame table — the row under test is not the one refusing"
        );
        assert_eq!(
            NetArena::buildable(&ir, &opts).err(),
            Some(*row),
            "{what}: wrong refusal row"
        );
        // …and the DESIGN gate must accept it, or an earlier layer is doing the
        // work and this row is untested.
        assert!(
            crate::native::design_eligibility(&ir, &opts)
                .reject_reasons
                .is_empty(),
            "{what}: refused by the S0 design gate, so this row is untested"
        );
    }
    // 10 -> 7: V1 slice 3a admitted the three system-task-argument shapes, and
    // their positive twin is `a_call_in_a_system_task_argument_is_admitted`.
    // 7 -> 8: A3-i split the task row in two — the subset half runs natively now
    // (`a_subset_task_call_has_its_iverilog_values`).
    // 8 -> 9: A3-ii-a moved the remaining half from "suspendable" to "SUSPENDS",
    // and it needs THREE designs: a `Delay`, a `Wait`, and a callee that parks
    // behind a caller that does not (the transitive arm of the park walk).
    // 9 -> 6: A3-ii-b RUNS all three of those, and the row they pinned narrowed
    // to `fork`, whose designs the DESIGN gate refuses first — so the count went
    // down by exactly the three that moved, and no case replaced them. The count
    // is asserted exactly, rather than as a floor, so that a row losing its last
    // design has to be justified by a human instead of quietly passing.
    assert_eq!(cases.len(), 6, "frame refusal-row coverage moved");
    // ⚠️ The WRITE twin of the "reads a module net" row has no design, and not
    // by omission: elaborate refuses a frame body that assigns to a net outside
    // the function (E3009), so the row can only ever be reached by a READ.
}

/// A subroutine CALL STATEMENT is decided by the THIRD gate layer rather than by
/// `frames_admitted`, and the two layers must not both claim it.
///
/// Kept apart from the row table above because the point is the SPLIT: a function
/// with an output formal is not a task, its body stays in its own frame, so the
/// storage gate has nothing to say — and it is the PROCESS that carries the
/// `Terminator::Call`, which `frames_admitted` never scans.
///
/// ⚠️ **This test used to assert the executor REFUSED it.** A3-i gave the walk an
/// arm, so the same two layers now answer accept/accept; the split is still the
/// subject, only the verdict moved. What keeps the assertion from being empty is
/// the pair: the subset shape is admitted here and the suspendable one is refused
/// three lines down, and both go through the same two calls.
#[test]
fn s3a_a_call_statement_is_decided_by_the_executor_layer() {
    let subset = r#"
module top;
  function automatic integer f(input integer x, output integer o);
    begin o = x * 2; f = x + 1; end
  endfunction
  integer r, o;
  initial begin r = f(4, o); $display("r=%0d o=%0d", r, o); $finish; end
endmodule
"#;
    let (ir, opts) = build_with_opts(subset);
    assert!(!opts.func_table.is_empty());
    assert_eq!(
        NetArena::buildable(&ir, &opts).err(),
        None,
        "the storage gate has no row for a call STATEMENT — the walk does"
    );
    assert_eq!(
        crate::native::run::runnable(&ir, &opts),
        Ok(()),
        "a subset call statement is walkable since A3-i"
    );
    // The SAME shape as a TASK that prints — "suspendable" by the engine's
    // executor test, but it does not park, so A3-ii-a DRIVES it. Both layers say
    // yes, and the storage layer is the one that changed its mind.
    let driven = r#"
module top;
  integer g;
  task automatic t(input integer x, output integer o);
    begin o = x * 2; $display("in t o=%0d", o); g = x; end
  endtask
  integer r;
  initial begin t(4, r); $display("r=%0d g=%0d", r, g); $finish; end
endmodule
"#;
    let (ir2, opts2) = build_with_opts(driven);
    assert_eq!(
        NetArena::buildable(&ir2, &opts2).err(),
        None,
        "a suspendable task that never parks is admitted since A3-ii-a"
    );
    assert_eq!(crate::native::run::runnable(&ir2, &opts2), Ok(()));
}

/// ⚠️⚠️ **A3-ii-b INVERTED this test.** It used to be
/// `a_parking_callee_is_refused_by_the_storage_layer`, and it pinned "where the
/// refusal of a PARKING callee lives — the storage layer, and only there".
/// Nothing lives there any more: the walk parks and resumes such a frame, so
/// BOTH layers must now admit it, and asserting the old closure would assert it
/// about a capability.
///
/// It is inverted rather than deleted for the reason it was written: the claim
/// worth keeping is that the two layers AGREE about this shape. They disagreed
/// once (A3-i put the refusal on the executor row while the storage row also
/// carried it), and a widening that moves one and not the other is exactly what
/// this catches — in whichever direction the agreement points.
#[test]
fn a_parking_callee_is_admitted_by_both_layers() {
    let src = r#"
module top;
  task automatic wait_then(input integer a, output integer b);
    begin #1 b = a + 1; end
  endtask
  integer r;
  initial begin wait_then(4, r); $display("r=%0d", r); $finish; end
endmodule
"#;
    let (ir, opts) = build_with_opts(src);
    assert_eq!(NetArena::buildable(&ir, &opts).err(), None);
    // …and the DESIGN gate accepts it, as it always did.
    assert!(crate::native::design_eligibility(&ir, &opts)
        .reject_reasons
        .is_empty());
    // The executor layer, asked on its own — which is how the census asks it.
    // Both answers are kept because a layer that changed its mind on the grounds
    // that another layer gets there first is a widening waiting to happen.
    assert!(crate::native::body::body_is_walkable(
        &ir,
        0,
        ir.processes[0].entry,
        &|bb| crate::native::frames::call_site_runnable(
            &ir,
            &opts,
            &crate::native::frames::suspendable_set(&ir, &opts),
            0,
            bb
        )
    ));
    // …and the FORK half, which is what the row narrowed to, still refuses at the
    // storage layer. Asked through the predicate rather than through a design,
    // because a design carrying a `fork` trips the DESIGN gate first — that is
    // why the row-coverage table can no longer carry a case for it either.
    let susp = crate::native::frames::suspendable_set(&ir, &opts);
    let entry = ir.funcs[0].entry;
    assert!(
        !crate::exec::frame_call::frame_forks(&ir, &susp, entry),
        "this callee parks on a delay, it does not fork"
    );
}

/// The row that protects the frame-BLIND consumers — `wprog`'s compile-time slot
/// resolution, `fast_offsets`, `write_lvalue` and the arena's own `read_net` —
/// from ever seeing a frame slot.
///
/// ⚠️ **No SOURCE reaches it.** Measured across every design in this file, the
/// four `examples/`, `bench/keccak` and `bench/picorv32`: elaborate never emits
/// a module-body reference to a frame slot (a call's copy-out destinations live
/// in the `task_calls_func` side table, not in a `Stmt` lvalue). So the row is
/// exercised the only honest way left — by MOVING a frame window over a module
/// net in the sidecar, which is exactly the shape the row exists to catch.
#[test]
fn s3a_a_module_body_naming_a_frame_slot_is_refused() {
    // TWO designs, because the module scan has two INDEPENDENT sources and a
    // single design cannot show either is load-bearing: (a) a process body, and
    // (b) a continuous assign whose nets NO process body names — declared first
    // so their ids fall inside the widened window, and read by nothing, so
    // dropping the cont-assign walk makes the design pass.
    //
    // (There is no third design for the `Delay { amount }` arm. An earlier version
    // of this note justified that with "a net named in a delay amount must also be
    // DRIVEN, so the scan reaches it through its driver" — which the round-2 review
    // falsified: an UNDRIVEN `wire [7:0] w;` read only in `#(w)` is named nowhere
    // else, and the design elaborates and runs. The arm is still equivalent-under-
    // mutation for a different reason: the row it feeds asks whether a MODULE body
    // names a FRAME-LOCAL net, and a frame slot cannot be a delay amount in any
    // design elaborate produces — only a corrupt sidecar could put one there, and
    // that is what the moved-window tests above simulate. It stays because it costs
    // one line.)
    let by_process = r#"
module top;
  function automatic integer inc(input integer x); begin inc = x + 1; end endfunction
  function automatic integer dec(input integer x); begin dec = x - 1; end endfunction
  integer r, s;
  initial begin r = inc(3); s = dec(9); $display("r=%0d s=%0d", r, s); $finish; end
endmodule
"#;
    // `r` is declared FIRST so its id is BELOW the two wires: the widening for
    // this design stops at the first cont-assign's lhs, so the only module body
    // naming anything inside the moved window is the cont-assign itself.
    let by_cont_assign = r#"
module top;
  integer r;
  wire [7:0] w;
  wire [7:0] v;
  assign w = 8'd3;
  assign v = w + 8'd1;
  function automatic integer inc(input integer x); begin inc = x + 1; end endfunction
  initial begin r = inc(3); $display("r=%0d", r); $finish; end
endmodule
"#;
    for (label, src, floor_is_ca) in [
        ("process body", by_process, false),
        ("cont-assign", by_cont_assign, true),
    ] {
        let (ir, opts) = build_with_opts(src);
        assert!(!opts.func_table.is_empty());
        assert_eq!(
            NetArena::buildable(&ir, &opts).err(),
            None,
            "{label}: the unmodified design must be admitted, or the mutations prove nothing"
        );
        // WIDEN a window DOWN so it swallows module nets, WITHOUT moving the real
        // slots out of it (the body-containment row must still pass, or it would
        // be the one refusing). Done for EACH function in turn — with only the
        // first, a disjointness loop that stops after `func_table[0]` survives.
        let floor = if floor_is_ca {
            let n = ir.cont_assigns[0].lhs.chunks[0].net;
            assert!(n > 0, "the process net must sit below the cont-assign's");
            n
        } else {
            0
        };
        for fi in 0..opts.func_table.len() {
            let mut moved = opts.clone();
            let m = &mut moved.func_table[fi];
            let end = m.base_net + m.locals_len;
            assert!(end as usize <= ir.nets.len() && m.base_net > floor);
            m.locals_len = end - floor;
            m.base_net = floor;
            assert_eq!(
                NetArena::buildable(&ir, &moved).err(),
                Some("a module body that names a frame-local net"),
                "{label}: widening function {fi}'s window must be caught"
            );
        }
    }
}

/// A malformed frame sidecar refuses instead of indexing out of range.
///
/// The engine latches a run-fatal on the same input (`build_func_routing`); the
/// tier-3 gate must not race it by laying out an arena first.
#[test]
fn s3a_a_malformed_frame_sidecar_refuses() {
    let src = r#"
module top;
  function automatic integer inc(input integer x); begin inc = x + 1; end endfunction
  integer r;
  initial begin r = inc(3); $display("r=%0d", r); $finish; end
endmodule
"#;
    let (ir, opts) = build_with_opts(src);
    let mut short = opts.clone();
    short.func_table.push(short.func_table[0]);
    assert_eq!(
        NetArena::buildable(&ir, &short).err(),
        Some("malformed frame sidecar (func_table length)"),
    );
    let mut oob = opts.clone();
    oob.func_table[0].base_net = ir.nets.len() as u32;
    oob.func_table[0].locals_len = 4;
    assert_eq!(
        NetArena::buildable(&ir, &oob).err(),
        Some("malformed frame sidecar (frame window out of range)"),
    );
    // The THIRD condition `build_func_routing` fatals on. Checking only the other
    // two and calling it "refuses instead of racing" was the gap.
    let mut ret = opts.clone();
    ret.func_table[0].return_slot = ret.func_table[0].locals_len;
    assert_eq!(
        NetArena::buildable(&ir, &ret).err(),
        Some("malformed frame sidecar (return slot out of range)"),
    );
}

/// A design with NO subroutines must not pay for this gate, and must not be
/// re-classified by it: `frames_admitted` returns on its first line.
#[test]
fn s3a_a_frameless_design_is_unaffected() {
    let src = r#"
module top;
  reg [7:0] q = 8'd0;
  initial begin #1 q = 8'd7; $display("q=%0d", q); $finish; end
endmodule
"#;
    let (ir, opts) = build_with_opts(src);
    assert!(opts.func_table.is_empty());
    assert_eq!(NetArena::buildable(&ir, &opts).err(), None);
    assert_eq!(
        crate::native::frames::frames_admitted(&ir, &SimOpts::default()).err(),
        None
    );
}

/// `bench/keccak`'s two SUBROUTINE variants — the designs this slice exists for,
/// compared on the real files rather than on a paraphrase of them.
///
/// They are the reason the frame row was the last thing keeping a real workload
/// off tier-3: `keccak_f` looks its `rho` offsets up through a `case`, and
/// `keccak_f_arr` builds a 25-entry frame-local array on every call. Skipped
/// rather than failed if the directory is absent, following `perf_baseline`.
#[test]
fn s3a_bench_keccak_subroutine_variants_match_the_vm() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../bench/keccak");
    let Ok(tb) = std::fs::read_to_string(dir.join("tb.sv")) else {
        return; // bench/keccak absent
    };
    let mut ran = 0;
    for name in ["keccak_f", "keccak_f_arr"] {
        let core = std::fs::read_to_string(dir.join(format!("{name}.sv"))).expect("variant");
        // `tb.sv` carries a `` `timescale ``, so the sources go through the
        // preprocessor — `build_with_opts` starts at the lexer.
        let pp = hdl_preprocess::preprocess_sources(
            &dir,
            &[
                (dir.join("tb.sv").to_string_lossy().into_owned(), tb.clone()),
                (
                    dir.join(format!("{name}.sv"))
                        .to_string_lossy()
                        .into_owned(),
                    core,
                ),
            ],
            &hdl_preprocess::PreOpts::default(),
        );
        // `+N` is unset, so the TB's `$value$plusargs` misses and it runs its
        // built-in 100 permutations — enough to exercise every round constant.
        agree(&pp.text, name).unwrap_or_else(|r| panic!("{name}: refused: {r}"));
        ran += 1;
    }
    assert_eq!(ran, 2, "both keccak subroutine variants must have been run");
}

/// The composite reader's SPLIT, exercised directly.
///
/// ⚠️ It has to be exercised directly, because on the admitted class it is
/// **unreached**: the `a module body that names a frame-local net` row is
/// precisely the statement that no module position ever hands the kernel a frame
/// slot, so every production `read_net` here takes the arena arm. The routing is
/// kept anyway — a correct read beats a junk one if that row is ever weakened,
/// and S3b widens the class it guards — but "kept as defence" is a claim that
/// has to be backed by something, and a differential over admitted designs
/// cannot back it.
///
/// The discriminator is a value the two stores DISAGREE on: a planted arena word
/// versus a different word in an open activation window. A reader that ignored
/// `frame_local` would return the planted value; one that routed everything to
/// the frame would lose the module net. (The window is opened deliberately —
/// with none open `frame_slot_read` PANICS rather than answering X, so "the
/// frame answers X" would have been the wrong discriminator as well as a wrong
/// sentence.)
#[test]
fn s3a_the_composite_reader_routes_frame_slots_to_the_frame() {
    use crate::eval::NetReader;
    use crate::native::kernel::NativeKernel;
    use crate::sched::Scheduler;
    use crate::state::SimState;

    let src = r#"
module top;
  function automatic integer inc(input integer x); integer loc; begin loc = x + 1; inc = loc; end endfunction
  integer r;
  initial begin r = inc(3); $display("r=%0d", r); $finish; end
endmodule
"#;
    let (ir, opts) = build_with_opts(src);
    let m = opts.func_table[0];
    let frame_net = m.base_net; // the input formal's slot
    let module_net = (0..ir.nets.len() as u32)
        .find(|n| *n < m.base_net || *n >= m.base_net + m.locals_len)
        .expect("the design has a module net");

    let mut arena = NetArena::build(&ir, &opts).expect("admitted");
    // Plant the SAME distinguishable word in both slots, so the two answers can
    // only differ by which store was read.
    arena.set_elem(frame_net, 0, &[0xA5A5_A5A5], &[0]);
    arena.set_elem(module_net, 0, &[0xA5A5_A5A5], &[0]);

    let sink = super::tests::NullSink;
    let mut st = SimState::new(
        &ir,
        Box::new(std::io::sink()),
        &sink,
        "1ns".to_string(),
        "test".to_string(),
        None,
    );
    st.func_table = opts.func_table.clone();
    st.build_func_routing();
    let mut sched = Scheduler::new(&mut st, 33_000, 10_000, None, Default::default());
    let empty: std::collections::BTreeMap<u32, u32> = std::collections::BTreeMap::new();
    let nk = NativeKernel::new(&ir, arena, &mut sched, &empty, 10_000);

    assert!(nk.has_frames, "the design must have a frame table");
    assert!(nk.is_frame_local(frame_net));
    assert!(!nk.is_frame_local(module_net));

    let from_module = nk.read_net(module_net, None);
    assert_eq!(
        from_module.val[0], 0xA5A5_A5A5,
        "a module net must read the arena word that was planted"
    );
    // Open an activation with a DIFFERENT word in the same slot, so the answer
    // discriminates the two stores by value rather than by absence.
    nk.sched
        .st
        .frame_stack
        .borrow_mut()
        .push(crate::state::WindowSlot::Owned(
            (0..m.locals_len)
                .map(|_| crate::value::Value::from_i128(0x1234_5678, 32, true))
                .collect(),
        ));
    let from_frame = nk.read_net(frame_net, None);
    assert_eq!(
        from_frame.val[0], 0x1234_5678,
        "a frame slot must read the ACTIVATION window, not the arena word planted \
         in its (unused) slot"
    );
}

/// The two rows that guard `run_frame_call`'s CATCH-ALLS — a statement it drops
/// silently (`_ => {}`) and a terminator it `break`s out of defensively.
///
/// ⚠️ **Neither is reachable from source.** Elaborate's B1 cut refuses a
/// subroutine body containing an NBA, a delay or a wait (E3009), and SystemVerilog
/// itself forbids a function calling a task — so no design can be written that
/// lands on either. They exist because the executor's catch-alls would SKIP the
/// statement or END the call, both silently, if elaborate's cut ever moved. So
/// the rows are exercised the only honest way left: by putting the shape into
/// the IR directly, which is exactly the situation the rows exist for.
#[test]
fn s3a_the_frame_body_catch_all_rows_are_exercised() {
    let src = r#"
module top;
  function automatic integer inc(input integer x); integer loc; begin loc = x + 1; inc = loc; end endfunction
  integer r;
  initial begin r = inc(3); $display("r=%0d", r); $finish; end
endmodule
"#;
    let (base, opts) = build_with_opts(src);
    assert_eq!(
        NetArena::buildable(&base, &opts).err(),
        None,
        "the unmodified design must be admitted, or the mutations prove nothing"
    );
    let entry = base.funcs[0].entry as usize;

    // (a) a TERMINATOR the walk of `run_frame_call` would `break` out of.
    let mut ir = base.clone();
    ir.blocks[entry].term = sim_ir::Terminator::Delay {
        amount: 0,
        region: sim_ir::DelayRegion::Active,
        resume: entry as u32,
    };
    assert_eq!(
        NetArena::buildable(&ir, &opts).err(),
        Some("a subroutine body that suspends, forks or calls a task"),
    );

    // (a2) slice #7: a `Terminator::Call` in a FUNCTION body must STILL refuse.
    // Synthesized rather than written in SystemVerilog on purpose — elaborate
    // refuses the only source shape that produces one (a nested call to a
    // subroutine with an output formal, E3009, pinned by
    // `a_nested_call_in_a_function_body_is_refused_by_elaborate`), so this arm
    // is fail-closed over an empty set and an IR mutation is the only way to
    // ask it anything. `run_frame_call` has no `Call` arm and would `break`.
    let mut ir = base.clone();
    ir.blocks[entry].term = sim_ir::Terminator::Call {
        target: 0,
        ret_bb: entry as u32,
    };
    assert_eq!(
        NetArena::buildable(&ir, &opts).err(),
        Some("a subroutine body that suspends, forks or calls a task"),
        "a nested call in a FUNCTION body (not a task) must still refuse"
    );

    // (b) a STATEMENT the executor's `_ => {}` would drop. The lvalue is the
    // function's own return slot, so the body stays inside its frame and the
    // containment row cannot be the one refusing.
    let mut ir = base.clone();
    let lhs = match &ir.stmts[ir.blocks[entry].stmts[0] as usize] {
        sim_ir::Stmt::BlockingAssign { lhs, .. } => lhs.clone(),
        other => panic!("expected a blocking assign to rewrite, found {other:?}"),
    };
    let rhs = match &ir.stmts[ir.blocks[entry].stmts[0] as usize] {
        sim_ir::Stmt::BlockingAssign { rhs, .. } => *rhs,
        _ => unreachable!(),
    };
    ir.stmts.push(sim_ir::Stmt::NonblockingAssign {
        lhs,
        rhs,
        delay: None,
    });
    let nba = ir.stmts.len() as u32 - 1;
    ir.blocks[entry].stmts[0] = nba;
    assert_eq!(
        NetArena::buildable(&ir, &opts).err(),
        Some("a subroutine statement the frame executor drops"),
    );
}

/// A3-iii RETIRED-AND-REPLACED: the `LvalChunk::width` edge is no longer
/// reachable for the containment row, and that is recorded rather than worked
/// around.
///
/// The old test built `f = loc[i:0]` — a non-constant part-select whose msb
/// READS a module net — to prove `Walk::lvalue` descends into the `width` edge.
/// A3-iii narrowed the row from `names` to `WRITES`, so a read through that edge
/// refuses nothing, and the write side cannot be spelled: the only out-of-window
/// destination elaborate permits from a subroutine body is a CLASS FIELD (a
/// plain module net is E3009 a phase earlier), and `c.v[i:0] = …` is itself a
/// pre-existing elaborate gap (E3010, a bit-select on a class field).
///
/// ⚠️ So the edge keeps its `self.expr(e)` call and LOSES its test. Saying so is
/// the point: the walk still feeds `w.nets` for the module-body row, but no
/// design reaches that row through an lvalue index either. Recorded as
/// unreachable rather than quietly deleted — if either gap closes, this comment
/// is what says a test is owed.
///
/// What IS pinned here is the row's live discriminator, in the two directions
/// the old test had: the out-of-window WRITE refuses, and the same body with the
/// write removed builds.
#[test]
fn s3a_an_out_of_window_write_refuses_and_its_read_only_twin_builds() {
    let refused = r#"
module top;
  class C; int v; endclass
  C c;
  function automatic integer f(input integer x);
    reg [31:0] loc;
    begin loc = x; c.v = loc[3:0]; f = loc[3:0]; end
  endfunction
  integer r;
  initial begin c = new(); r = f(255); $display("r=%0d", r); $finish; end
endmodule
"#;
    // CONTROL: the same body READING the field instead of writing it must be
    // admitted, or the refusal above says nothing about the WRITE in particular.
    let admitted = r#"
module top;
  class C; int v; endclass
  C c;
  function automatic integer f(input integer x);
    reg [31:0] loc;
    begin loc = x + c.v; f = loc[3:0]; end
  endfunction
  integer r;
  initial begin c = new(); r = f(255); $display("r=%0d", r); $finish; end
endmodule
"#;
    let (ir, opts) = build_with_opts(refused);
    assert_eq!(
        NetArena::buildable(&ir, &opts).err(),
        Some("a subroutine that WRITES a net outside its own frame: S3b"),
    );
    let (ir, opts) = build_with_opts(admitted);
    assert_eq!(NetArena::buildable(&ir, &opts).err(), None);
}

/// Slice #3 DIFFERENTIAL — frame-local heap statements, native against the VM.
#[test]
fn frame_local_heap_statements_match_the_vm() {
    let designs: Vec<(&str, &str)> = vec![
        // `new[]` with a COPY source (`new[n](old)`), whose element copy is the
        // heap half the size read sits next to.
        (
            "frame-local new with a copy source",
            r#"
module top;
  int n = 4;
  function automatic int f();
    int a[];
    int b[];
    int i;
    a = new[n];
    for (i = 0; i < n; i = i + 1) a[i] = i + 1;
    b = new[n + 2](a);
    f = b.size() * 100 + b[n - 1];
  endfunction
  initial begin $display("N %0d", f()); n = 6; $display("N %0d", f()); $finish; end
endmodule
"#,
        ),
        // `delete()` on a frame-local dyn array — the sibling arm, admitted with
        // `new[]` because it is the same family. (Zero corpus designs spell it,
        // which is why it needs a row of its own here.)
        (
            "frame-local dyn delete",
            r#"
module top;
  int n = 3;
  function automatic int f();
    int a[];
    a = new[n];
    a[0] = 9;
    a.delete();
    f = a.size();
  endfunction
  initial begin $display("D %0d", f()); $finish; end
endmodule
"#,
        ),
        // An X/Z size, which takes the warn-once path rather than the alloc.
        (
            "frame-local new with an x size",
            r#"
module top;
  reg [7:0] n;
  function automatic int f();
    int a[];
    a = new[n];
    f = a.size();
  endfunction
  initial begin $display("X %0d", f()); $finish; end
endmodule
"#,
        ),
    ];
    let mut ran = 0usize;
    let mut refused: std::collections::BTreeMap<&'static str, usize> =
        std::collections::BTreeMap::new();
    for (name, src) in &designs {
        match agree(src, name) {
            Ok(()) => ran += 1,
            Err(r) => *refused.entry(r).or_default() += 1,
        }
    }
    assert_eq!(ran, 3, "slice #3 differential: runnable count moved");
    assert_eq!(
        refused,
        Default::default(),
        "slice #3 differential: refusal breakdown moved"
    );
}

/// Slice #7 DIFFERENTIAL — nested task calls, native against the VM.
#[test]
fn nested_task_calls_match_the_vm() {
    let designs: Vec<(&str, &str)> = vec![
        // Three levels deep, so the recursion is not a single step, with a
        // module net read at the BOTTOM.
        (
            "three-level nested task call",
            r#"
module top;
  reg [7:0] g = 8'd2;
  task automatic lvl3(input [7:0] x, output [7:0] y); y = x * g; endtask
  task automatic lvl2(input [7:0] x, output [7:0] y);
    reg [7:0] t; lvl3(x, t); y = t + 8'd1;
  endtask
  task automatic lvl1(input [7:0] x, output [7:0] y);
    reg [7:0] t; lvl2(x, t); y = t + 8'd10;
  endtask
  reg [7:0] r;
  initial begin lvl1(8'd3, r); $display("D r=%0d", r); g = 8'd5; lvl1(8'd3, r); $display("D r=%0d", r); $finish; end
endmodule
"#,
        ),
        // A nested call whose callee has TWO output formals and whose actuals
        // are the caller's frame locals — the copy-out order matters.
        (
            "nested call with two output formals",
            r#"
module top;
  task automatic two(input [7:0] x, output [7:0] a, output [7:0] b);
    a = x + 8'd1;
    b = x + 8'd2;
  endtask
  task automatic caller(input [7:0] x, output [7:0] r);
    reg [7:0] p; reg [7:0] q;
    two(x, p, q);
    r = p * 8'd10 + q;
  endtask
  reg [7:0] r;
  initial begin caller(8'd3, r); $display("T r=%0d", r); $finish; end
endmodule
"#,
        ),
        // A nested call inside a LOOP in the caller's body, so the recursion
        // runs several times against one activation window.
        (
            "nested call inside a loop",
            r#"
module top;
  reg [7:0] mem [0:3];
  task automatic get(input [7:0] i, output [7:0] v); v = mem[i]; endtask
  task automatic sum(output [7:0] s);
    integer i; reg [7:0] v;
    s = 8'd0;
    for (i = 0; i < 4; i = i + 1) begin get(i[7:0], v); s = s + v; end
  endtask
  reg [7:0] s;
  initial begin
    mem[0] = 8'd1; mem[1] = 8'd2; mem[2] = 8'd3; mem[3] = 8'd4;
    sum(s); $display("S s=%0d", s);
    mem[2] = 8'd30; sum(s); $display("S s=%0d", s);
    $finish;
  end
endmodule
"#,
        ),
    ];
    let mut ran = 0usize;
    let mut refused: std::collections::BTreeMap<&'static str, usize> =
        std::collections::BTreeMap::new();
    for (name, src) in &designs {
        match agree(src, name) {
            Ok(()) => ran += 1,
            Err(r) => *refused.entry(r).or_default() += 1,
        }
    }
    assert_eq!(ran, 3, "slice #7 differential: runnable count moved");
    assert_eq!(
        refused,
        Default::default(),
        "slice #7 differential: refusal breakdown moved"
    );
}
