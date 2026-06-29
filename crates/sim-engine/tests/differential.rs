//! Differential verification against Icarus Verilog (`iverilog` + `vvp`) — the
//! doc-01 §성공기준 golden. Each representative design is run through BOTH vitamin and
//! iverilog and their `$display` output is compared. If iverilog/vvp are not on PATH
//! (e.g. a minimal CI image) the check SKIPS gracefully (the design still simulates
//! through vitamin so a vitamin-side crash is still caught).

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use diag::{LogEvent, LogSink};
use sim_engine::{simulate_capture, SimOpts};

static NEXT: AtomicU64 = AtomicU64::new(0);

#[derive(Default)]
struct DiagSink(std::cell::RefCell<Vec<String>>);
impl LogSink for DiagSink {
    fn emit(&self, e: LogEvent) {
        if let LogEvent::Diagnostic(d) = e {
            self.0
                .borrow_mut()
                .push(format!("{:?}: {}", d.severity, d.message));
        }
    }
}

/// vitamin: lex → parse → elaborate → simulate, returning the captured stdout.
fn vita_out(src: &str) -> String {
    let (toks, le) = hdl_lexer::lex(src);
    assert!(le.is_empty(), "lex errors: {le:?}");
    let (su, pe) = hdl_parser::parse(&toks, src);
    assert!(pe.is_empty(), "parse errors: {pe:?}");
    let sink = DiagSink::default();
    // Full sidecars (severity/radix/fork/timescale tables) — a differential
    // case using $displayh or $fatal must exercise the SAME tables production
    // threads through SimOpts.
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
        .filter(|d| d.starts_with("Error") || d.starts_with("Fatal"))
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

/// iverilog + vvp: compile and run, returning the design's `$display` stdout
/// (the vvp `$finish called …` banner line is stripped). `None` ⇒ tool absent.
fn iverilog_out(src: &str) -> Option<String> {
    if !on_path("iverilog") || !on_path("vvp") {
        return None;
    }
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir();
    let sv = dir.join(format!("vita_diff_{}_{n}.sv", std::process::id()));
    let vvp = dir.join(format!("vita_diff_{}_{n}.vvp", std::process::id()));
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
    let stdout = String::from_utf8_lossy(&run.stdout);
    // strip vvp's runtime banner lines (`$finish called …`, etc.)
    let mut filtered = String::new();
    for l in stdout
        .lines()
        .filter(|l| !l.contains("$finish called") && !l.contains("$stop called"))
    {
        filtered.push_str(l);
        filtered.push('\n');
    }
    Some(filtered)
}

/// Assert vitamin and iverilog produce identical `$display` output (skip if iverilog
/// is unavailable). The design must be IEEE-deterministic (no X-dependent output).
fn assert_matches_iverilog(name: &str, src: &str) {
    let v = vita_out(src);
    match iverilog_out(src) {
        None => eprintln!("[{name}] iverilog/vvp not on PATH — differential check skipped"),
        Some(iv) => assert_eq!(
            v.trim_end(),
            iv.trim_end(),
            "[{name}] vitamin vs iverilog divergence"
        ),
    }
}

#[test]
fn diff_alu() {
    assert_matches_iverilog(
        "alu",
        "module alu(input [7:0] a, b, input [1:0] op, output reg [7:0] y); \
           always @* case (op) 2'd0:y=a+b; 2'd1:y=a-b; 2'd2:y=a&b; 2'd3:y=a|b; endcase endmodule \
         module tb; reg [7:0] a, b; reg [1:0] op; wire [7:0] y; alu u(a,b,op,y); \
           initial begin a=8'd10; b=8'd3; \
             op=2'd0; #1 $display(\"%0d\",y); op=2'd1; #1 $display(\"%0d\",y); \
             op=2'd2; #1 $display(\"%0d\",y); op=2'd3; #1 $display(\"%0d\",y); $finish; end endmodule",
    );
}

#[test]
fn diff_counter_with_reset() {
    assert_matches_iverilog(
        "counter",
        "module counter(input clk, rst, output reg [3:0] cnt); \
           always @(posedge clk) if (rst) cnt<=4'd0; else cnt<=cnt+4'd1; endmodule \
         module tb; reg clk, rst; wire [3:0] cnt; counter c(clk,rst,cnt); integer k; \
           initial begin clk=0; rst=1; #5 clk=1; #5 clk=0; rst=0; \
             for (k=0;k<6;k=k+1) begin #5 clk=1; #5 clk=0; end \
             $display(\"%0d\", cnt); $finish; end endmodule",
    );
}

#[test]
fn diff_memory_accumulate() {
    assert_matches_iverilog(
        "memory",
        "module tb; reg [7:0] mem[0:7]; integer i; reg [15:0] sum; \
           initial begin for (i=0;i<8;i=i+1) mem[i]=i*3; \
             sum=0; for (i=0;i<8;i=i+1) sum=sum+mem[i]; \
             $display(\"%0d\", sum); $finish; end endmodule",
    );
}

#[test]
fn diff_shift_and_arith() {
    assert_matches_iverilog(
        "shift",
        "module tb; reg [15:0] x; integer i; \
           initial begin x=16'd1; for (i=0;i<5;i=i+1) x=x<<1; \
             $display(\"%0d %0d %0d\", x, x>>2, x*3); $finish; end endmodule",
    );
}

#[test]
fn diff_packed_struct() {
    assert_matches_iverilog(
        "packed_struct",
        "module tb; typedef struct packed { logic [7:0] hi; logic [7:0] lo; } word_t; \
           word_t w; \
           initial begin w.hi = 8'hDE; w.lo = 8'hAD; \
             $display(\"%h %h %h\", w.hi, w.lo, w); \
             w = 16'hBEEF; $display(\"%h %h\", w.hi, w.lo); $finish; end endmodule",
    );
}

#[test]
fn diff_packed_struct_single_bit_field() {
    // 3 fields incl. a 1-bit member: tag[4](high), valid[1](mid), data[3:0](low).
    // total=9. Exercises odd-boundary offset math against iverilog.
    assert_matches_iverilog(
        "packed_struct_bits",
        "module tb; typedef struct packed { logic [4:0] tag; logic valid; logic [2:0] data; } e_t; \
           e_t e; \
           initial begin e.tag = 5'h1A; e.valid = 1'b1; e.data = 3'h5; \
             $display(\"%h %b %h %h\", e.tag, e.valid, e.data, e); \
             e = 9'h0AB; $display(\"%h %b %h\", e.tag, e.valid, e.data); $finish; end endmodule",
    );
}

#[test]
fn diff_typedef_alias() {
    assert_matches_iverilog(
        "typedef_alias",
        "module tb; typedef logic [7:0] byte_t; typedef reg [3:0] nib_t; \
           byte_t a, b; nib_t n; \
           initial begin a = 8'hF0; b = 8'h0F; n = a[7:4]; \
             $display(\"%h %0d %0d\", a | b, n, a + b); $finish; end endmodule",
    );
}

#[test]
fn diff_enum_labels() {
    assert_matches_iverilog(
        "enum",
        "module tb; typedef enum {RED, GREEN, BLUE} color_t; color_t c; \
           integer i; reg [31:0] acc; \
           initial begin acc = 0; \
             c = RED;   acc = acc + c; \
             c = GREEN; acc = acc + c; \
             c = BLUE;  acc = acc + c; \
             $display(\"%0d %0d %0d\", RED, GREEN, BLUE); \
             $display(\"%0d\", acc); $finish; end endmodule",
    );
}

#[test]
fn diff_wide_reduction_word_boundary() {
    // 100-bit reductions spanning two words — exercises word-level reduce_word
    // (any-0 / any-1 / parity + last-word mask) against iverilog.
    assert_matches_iverilog(
        "wide_reduction",
        "module tb; reg [99:0] v; \
           initial begin v = 100'h0; v[0] = 1'b1; v[50] = 1'b1; v[99] = 1'b1; \
             $display(\"%b %b %b %b\", &v, |v, ^v, ~|v); $finish; end endmodule",
    );
}

#[test]
fn diff_wide_bitwise_word_boundary() {
    // 96-bit bitwise across the 64-bit word boundary — exercises the word-level
    // and_w/or_w/xor_w/not_w paths + last-word masking against iverilog.
    assert_matches_iverilog(
        "wide_bitwise",
        "module tb; reg [95:0] a, b; \
           initial begin a = 96'hFFFF0000_FFFF0000_FFFF0000; \
                         b = 96'h0F0F0F0F_0F0F0F0F_0F0F0F0F; \
             $display(\"%h %h %h %h\", a & b, a | b, a ^ b, ~a); $finish; end endmodule",
    );
}

#[test]
fn diff_casez_priority() {
    assert_matches_iverilog(
        "casez",
        "module tb; reg [3:0] s; reg [7:0] r; \
           initial begin s=4'b0110; \
             casez (s) 4'b1???:r=8'd1; 4'b01??:r=8'd2; default:r=8'd9; endcase \
             $display(\"%0d\", r); $finish; end endmodule",
    );
}

#[test]
fn diff_display_arg_semantics() {
    // P0-8/9 (2026-06-10): sequential $display argument processing (trailing
    // args in default radix, string args as inline format segments, %v
    // consumption) + $monitor not retriggering on a direct $time argument.
    assert_matches_iverilog(
        "display_args",
        "module tb; reg clk; reg [7:0] v; reg w; \
           initial begin \
             v = 8'd255; \
             $display(\"val=\", v); \
             $display(\"v=\", 8'd5); \
             $display(8'd7, \" is a\"); \
             $display(\"a=%0d\", 4'd1, \" b=\", 8'd5); \
             $display(8'd255, 8'd255); \
             w = 1'b1; \
             $display(\"%v %0d\", w, 8'd5); \
           end \
           initial begin \
             v = 8'd5; clk = 0; \
             $monitor(\"t=%0t mv=%0d\", $time, v); \
             #2 v = 8'd9; \
             #2 $finish; \
           end \
           always #1 clk = ~clk; \
         endmodule",
    );
}

#[test]
fn diff_const_domain_cluster() {
    // P0-5/6/7 (2026-06-10): signed-i64 elaboration const domain — ternary and
    // $clog2 param folding, `1<<32` past the old u32 wrap, signed `>>>`,
    // negative-param display, and a descending generate-for. iverilog parity.
    assert_matches_iverilog(
        "const_domain",
        "module tb; \
           parameter MODE = 1; \
           parameter W = MODE ? 16 : 8; \
           parameter DEPTH = 300; \
           localparam AW = $clog2(DEPTH); \
           parameter [63:0] F = 1 << 32; \
           parameter S = -4; \
           parameter T = S >>> 1; \
           wire [3:0] in_w, out_w; \
           assign in_w = 4'b1010; \
           genvar i; \
           generate for (i = 3; i >= 0; i = i - 1) begin: g \
             assign out_w[i] = in_w[i]; \
           end endgenerate \
           initial begin \
             $display(\"%0d %0d %0d %0d %0d\", W, AW, F, S, T); \
             #1 $display(\"%b\", out_w); \
             $finish; \
           end endmodule",
    );
}

#[test]
fn diff_wide_value_truncation_cluster() {
    // P0-1~4 (2026-06-10): >64-bit relational compare, over-u64 shift amounts,
    // full-width unary minus and wide $clog2 — all formerly truncated to the
    // low word. Locks the fixed semantics against the iverilog oracle.
    assert_matches_iverilog(
        "wide_trunc",
        "module tb; reg [127:0] a, b, n; reg signed [127:0] sa, sb; \
           reg [7:0] x, l, r; reg signed [7:0] sx, sy; reg [127:0] s; \
           initial begin \
             a = 128'h1_0000_0000_0000_0000; b = 128'h1; \
             $display(\"%b %b %b %b\", a > b, a < b, a >= b, a <= b); \
             sa = 128'hffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff; sb = 128'sd1; \
             $display(\"%b %b\", sa < sb, sa > sb); \
             x = 8'hFF; s = 128'h1_0000_0000_0000_0000; \
             l = x << s; r = x >> s; \
             $display(\"%h %h\", l, r); \
             sx = -8'sd2; sy = sx >>> s; \
             $display(\"%b\", sy); \
             n = -128'd1; \
             $display(\"%h\", n); \
             $display(\"%0d %0d\", $clog2(a), $clog2(a + 1)); \
             $finish; end endmodule",
    );
}

#[test]
fn diff_inbody_star_and_finish_flush() {
    // P1-4 (in-body @(*) wakes on the controlled statement's read set) +
    // P1-6 (same-step $strobe/$monitor flush before $finish) — iverilog parity.
    assert_matches_iverilog(
        "inbody_star_finish_flush",
        "module tb; reg a, b, y; reg [3:0] s; \
           initial begin \
             a = 0; b = 0; \
             #1 a = 1; \
             #2 $finish; \
           end \
           initial begin \
             @(*) y = a ^ b; \
             $display(\"y=%b at %0t\", y, $time); \
           end \
           initial begin \
             s = 4'd3; \
             $strobe(\"s=%0d\", s); \
           end \
         endmodule",
    );
}

#[test]
fn diff_radix_print_variants() {
    // P1-5: $displayb/o/h + $writeh + $strobeh + $monitorh — the b/o/h variants
    // change ONLY the default radix of unformatted args (iverilog parity,
    // including the no-separator padded-field join). Same-step strobe-vs-monitor
    // flush ORDER is sim-specific, so the two register in different timesteps.
    assert_matches_iverilog(
        "radix_variants",
        "module tb; reg [7:0] a; reg [15:0] b; reg [3:0] c; reg [7:0] m; \
           initial begin \
             a = 8'd255; b = 16'h00ab; c = 4'd5; \
             $displayh(a, b); \
             $displayb(c); \
             $displayo(c); \
             $displayh(\"d=%0d\", 4'd5, a); \
             $writeh(8'hf0); \
             $display(\"\"); \
             $strobeh(a); \
             #1 m = 8'd16; \
             $monitorh(m); \
             #1 m = 8'd255; \
             #1 $finish; \
           end \
         endmodule",
    );
}

#[test]
fn diff_time_type_semantics() {
    // P2-12: `time` = 64-bit unsigned 4-state variable. Unsigned wrap of -1,
    // full 64-bit hex, $time capture, and unpacked time arrays must all match
    // iverilog byte-for-byte. (No `timescale — this harness skips preprocess;
    // both sides then count raw 1-unit ticks, so $time agrees.)
    assert_matches_iverilog(
        "time_type",
        "module tb; time t; time arr [0:1]; \
           initial begin \
             #5 t = $time; \
             $display(\"a=%0d\", t); \
             t = -1; \
             $display(\"b=%0d\", t); \
             t = 64'hFFFF_FFFF_FFFF_FFFF; \
             $display(\"c=%0h\", t); \
             arr[0] = $time; arr[1] = arr[0] + 1; \
             $display(\"p=%0d q=%0d\", arr[0], arr[1]); \
             $finish; \
           end \
         endmodule",
    );
}

#[test]
fn diff_runtime_delay() {
    // format_version 4: #<expr> evaluates at suspension time. Integer var,
    // arithmetic expr, and an X delay (= 0 ticks) — iverilog byte parity.
    // (No `timescale in this harness ⇒ M=1 on both sides.)
    assert_matches_iverilog(
        "runtime_delay",
        "module tb; integer d; reg [3:0] xd; integer n; reg clk; \
           initial begin \
             d = 3; clk = 0; n = 0; \
             #d $display(\"a=%0d\", $time); \
             #(d*2) $display(\"b=%0d\", $time); \
             xd = 4'hx; \
             #xd $display(\"c=%0d\", $time); \
             $finish; \
           end \
         endmodule",
    );
}

#[test]
fn diff_force_release() {
    // CONST-RHS force on a net + a variable; release restores the net's
    // driver and a variable keeps the forced value — iverilog byte parity.
    // (EXPRESSION-RHS forces re-evaluate continuously per IEEE §9.3.2 in
    // vitamin but NOT in iverilog — iverilog itself warns "sorry: ... will
    // only be evaluated once" — so that lane is pinned by the hand-computed
    // end_to_end::force_expression_reevaluates_continuously instead.)
    assert_matches_iverilog(
        "force_release",
        "module tb; wire w; reg a; assign w = a; reg r; \
           initial begin \
             a = 0; r = 0; \
             #1 $display(\"t1 %b %b\", w, r); \
             force w = 1'b1; force r = 1'b1; \
             #1 $display(\"t2 %b %b\", w, r); \
             a = 1; r = 0; \
             #1 $display(\"t3 %b %b\", w, r); \
             release w; release r; \
             #1 $display(\"t4 %b %b\", w, r); \
             r = 0; \
             #1 $display(\"t5 %b\", r); \
             $finish; \
           end \
         endmodule",
    );
}

#[test]
fn diff_wide_lane_c6() {
    // [C6] 100-bit two-word datapath: arith carries crossing bit 63, wide
    // bitwise/shift/div/mod/compare/reduction — the native wide lane's oracle
    // (the interpreter) must match iverilog bit-for-bit; backend_equiv's
    // native_wide_lane teeth then pin VM == interpreter on the same shape.
    assert_matches_iverilog(
        "wide_lane_c6",
        "module tb; \
           reg clk; \
           reg [99:0] a, b, sum, dif, prd, bnd, sl, sr, dv, md; \
           reg lt, rx; \
           always @(posedge clk) begin \
             sum <= a + b; \
             dif <= a - b; \
             prd <= a * b; \
             bnd <= (a & b) ^ ~a; \
             sl  <= a << 37; \
             sr  <= a >> 65; \
             dv  <= a / 7; \
             md  <= a % 7; \
             lt  <= a < b; \
             rx  <= ^a; \
           end \
           initial begin \
             a = 100'habcdef0123456789abcdef012; \
             b = 100'h00003fffffffffffffff00001; \
             clk = 0; \
             #1 clk = 1; #1 clk = 0; \
             #1 $display(\"%h %h %h %h\", sum, dif, prd, bnd); \
             $display(\"%h %h %h %h %b %b\", sl, sr, dv, md, lt, rx); \
             $finish; \
           end \
         endmodule",
    );
}

#[test]
fn diff_indexed_expr_reads() {
    // [C6] array-word reads inside expressions (LoadIndexed): valid and
    // out-of-range indices (OOR reads all-X; X+1 stays all-X under %h).
    assert_matches_iverilog(
        "indexed_expr_reads",
        "module tb; \
           reg clk; \
           reg [15:0] mem [0:3]; \
           reg [1:0] i, j; \
           reg [3:0] oor; \
           reg [15:0] q, qo; \
           always @(posedge clk) begin \
             q  <= mem[i] + mem[j] * 16'd2; \
             qo <= mem[oor] + 16'd1; \
           end \
           initial begin \
             mem[0] = 16'h0010; mem[1] = 16'h0200; \
             mem[2] = 16'h3000; mem[3] = 16'h0004; \
             i = 2'd1; j = 2'd2; oor = 4'd9; clk = 0; \
             #1 clk = 1; #1 clk = 0; \
             #1 $display(\"%h %h\", q, qo); \
             $finish; \
           end \
         endmodule",
    );
}

#[test]
fn diff_blocking_intra_assignment_delay() {
    // `a = #d rhs` — RHS captured at eval time (a cross-process write during
    // the suspension must not leak in), write + resume at t+d; `#0` form lands
    // same-time (inactive region); runtime delay expr works in the intra form.
    assert_matches_iverilog(
        "blocking_intra_delay",
        "module tb; \
           reg [7:0] a, b, z; integer d; \
           initial begin \
             b = 8'd5; \
             a = #3 b; \
             $display(\"t=%0d a=%0d b=%0d\", $time, a, b); \
             z = #0 8'd9; \
             $display(\"t=%0d z=%0d\", $time, z); \
             d = 4; \
             a = #(d) b + 8'd1; \
             $display(\"t=%0d a=%0d\", $time, a); \
             $finish; \
           end \
           initial #1 b = 8'd77; \
         endmodule",
    );
}

#[test]
fn diff_immediate_assert() {
    // Immediate assert with EXPLICIT actions only — both tools print the same
    // pass/else $display lines. The no-else asserts in this design always PASS
    // so no tool-specific default-failure text reaches stdout. X conditions
    // (uninitialized reg) must take the else branch in both tools.
    assert_matches_iverilog(
        "immediate_assert",
        "module tb; \
           reg a; reg b; reg [1:0] c; \
           initial begin \
             a = 1; \
             assert (a) $display(\"p1\"); else $display(\"f1\"); \
             a = 0; \
             assert (a) $display(\"p2\"); else $display(\"f2\"); \
             assert (b) $display(\"p3\"); else $display(\"f3\"); \
             assert (a == 0); \
             assert (a | b) else $display(\"f4\"); \
             c = 2'b10; \
             assert (c[1] & ~c[0]) $display(\"p5\"); \
             $finish; \
           end \
         endmodule",
    );
}

#[test]
fn diff_disable_break_continue() {
    // break idiom: disable of the enclosing named block abandons loop + tail.
    assert_matches_iverilog(
        "disable_break",
        "module tb; integer i; \
           initial begin : L \
             for (i = 0; i < 10; i = i + 1) begin \
               if (i == 3) disable L; \
               $display(\"i=%0d\", i); \
             end \
             $display(\"tail\"); \
           end \
           initial #1 begin $display(\"done\"); $finish; end \
         endmodule",
    );
    // continue idiom: disable of the per-iteration block skips to the step.
    assert_matches_iverilog(
        "disable_continue",
        "module tb; integer i; \
           initial begin \
             for (i = 0; i < 5; i = i + 1) begin : ITER \
               if (i == 2) disable ITER; \
               $display(\"i=%0d\", i); \
             end \
             $display(\"end\"); \
             $finish; \
           end \
         endmodule",
    );
    // nested: disable OUTER from INNER skips both tails.
    assert_matches_iverilog(
        "disable_nested",
        "module tb; \
           initial begin \
             begin : OUTER \
               begin : INNER disable OUTER; $display(\"x1\"); end \
               $display(\"x2\"); \
             end \
             $display(\"after\"); \
             $finish; \
           end \
         endmodule",
    );
}

#[test]
fn diff_proc_assign_deassign() {
    // CONST-RHS proc assign/deassign lanes only: iverilog implements the
    // pin/hold/rank model for constants but self-admits "evaluated once" for
    // expression RHS (same as force) — the expression lane is pinned by the
    // hand-IEEE end_to_end test instead.
    assert_matches_iverilog(
        "proc_assign_basic",
        "module tb; reg [7:0] q; \
           initial begin \
             q = 8'd1; \
             $display(\"t0 q=%0d\", q); \
             assign q = 8'd42; \
             #1 q = 8'd5; \
             $display(\"t1 q=%0d\", q); \
             deassign q; \
             $display(\"t2 q=%0d\", q); \
             q = 8'd7; \
             $display(\"t3 q=%0d\", q); \
             $finish; \
           end \
         endmodule",
    );
    assert_matches_iverilog(
        "proc_assign_force_rank",
        "module tb; reg [7:0] q; \
           initial begin \
             assign q = 8'd10; \
             $display(\"a q=%0d\", q); \
             force q = 8'd20; \
             $display(\"b q=%0d\", q); \
             release q; \
             $display(\"c q=%0d\", q); \
             deassign q; \
             q = 8'd30; \
             $display(\"d q=%0d\", q); \
             $finish; \
           end \
         endmodule",
    );
}

#[test]
fn diff_nba_transport_delay() {
    // v5 increment (A): value-carrying delayed NBA. Four lanes — NBA-region
    // landing vs active reads, overlapping in-flight activations (the case a
    // static capture net gets silently wrong), #0 statement order, and
    // schedule-time index sampling.
    assert_matches_iverilog(
        "nba_transport_basic",
        "module tb; reg [7:0] q; \
           initial begin \
             q = 8'd1; \
             q <= #3 8'd9; \
             $display(\"t%0d q=%0d\", $time, q); \
             #1 $display(\"t%0d q=%0d\", $time, q); \
             #1 $display(\"t%0d q=%0d\", $time, q); \
             #1 $display(\"t%0d q=%0d\", $time, q); \
             #1 $finish; \
           end \
         endmodule",
    );
    assert_matches_iverilog(
        "nba_transport_overlap",
        "module tb; reg clk; reg [7:0] d, q; \
           initial begin clk = 0; d = 8'd1; end \
           always #1 clk = ~clk; \
           always @(posedge clk) begin \
             q <= #3 d; \
             d <= d + 8'd1; \
           end \
           initial #13 $finish; \
           always @(q) $display(\"t%0d q=%0d\", $time, q); \
         endmodule",
    );
    assert_matches_iverilog(
        "nba_zero_delay_order",
        "module tb; reg [7:0] q; \
           initial begin \
             q = 8'd1; \
             q <= #0 8'd5; \
             q <= 8'd7; \
             #1 $display(\"t1 q=%0d\", q); \
             $finish; \
           end \
         endmodule",
    );
    assert_matches_iverilog(
        "nba_index_freeze",
        "module tb; reg [7:0] mem [0:3]; integer i; \
           initial begin \
             mem[0] = 8'd0; mem[1] = 8'd0; mem[2] = 8'd0; mem[3] = 8'd0; \
             i = 1; \
             mem[i] <= #2 8'd42; \
             i = 3; \
             #3 $display(\"m1=%0d m3=%0d\", mem[1], mem[3]); \
             $finish; \
           end \
         endmodule",
    );
}

#[test]
fn diff_named_events() {
    // v5 batch (B), re-classified OUT of the bump: named events desugar to a
    // 64-bit counter reg (`->e` = e+1, `@(e)` = AnyEdge on the net). Three
    // lanes: plain trigger/wake, mixed sensitivity list, no-latch semantics.
    assert_matches_iverilog(
        "event_basic",
        "module tb; event e; \
           initial begin #1 -> e; #2 -> e; #2 $finish; end \
           always @(e) $display(\"t%0d fired\", $time); \
         endmodule",
    );
    assert_matches_iverilog(
        "event_mixed_list",
        "module tb; event e; reg clk; \
           initial clk = 0; \
           always #1 clk = ~clk; \
           initial begin #4 -> e; #4 $finish; end \
           always @(posedge clk or e) $display(\"t%0d wake\", $time); \
         endmodule",
    );
    assert_matches_iverilog(
        "event_no_latch",
        "module tb; event e; \
           initial begin -> e; #1 -> e; #1 $finish; end \
           initial begin #0 @(e) $display(\"t%0d caught\", $time); end \
         endmodule",
    );
}
