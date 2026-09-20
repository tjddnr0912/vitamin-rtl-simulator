//! A procedural read of a WIDTH-CHANGING copy — `logic signed [7:0] v;
//! logic [15:0] c; assign c = v;` — after the reader's own blocking write of `v`,
//! in the process body and inside every frame body the process runs.
//! ROADMAP §2 🆕 I ⓖ (residue of §4.5.438 / §4.5.442).
//!
//! `copied_source` requires the driver to be a bit MOVE, so a sign-extending
//! driver was not a copy net and no read of it was ever marked: the read kept the
//! settle's `xxxx` where iverilog 13.0 prints `ffa5` — in the process body, inside
//! a called task or function, through a nested callee, in a call's in-bind actual,
//! and when the write of `v` itself happens inside a callee. The runtime rename set
//! (`copy_nets`) is unchanged — an extension is not a move — and only the READ
//! alias (`alias::sign_extending_copies`) admits it.
//!
//! ORACLES. Every value below was measured on iverilog 13.0 and verilator 5.052
//! and the lines are their output, copied. Where the two split, the ruling is
//! iverilog's, because verilator is not an oracle for this cell: it answers the
//! SAME read `ffa5` when the design reads `c` once and `0000` when it reads it
//! twice (`y = c; $display(c);`) — a later read changing what an earlier one
//! returns — while iverilog answers `ffa5` for every spelling (a direct
//! `$display(c)`, an assignment to a local, two reads, a re-write and a second
//! call). iverilog builds this driver as an `.extend/s` functor, which propagates
//! on the store; it builds the ZERO-extending one as a `.concat` with a constant
//! and the TRUNCATING one as a select, neither of which propagates, and it reads
//! the stale value for both — which is why those two shapes are pinned UNCHANGED
//! here rather than at an oracle's value.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_backend(src: &str, backend: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_crtcw_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("--backend")
        .arg(backend)
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (s, out.status.code())
}

/// Every `D=` line, on all three backends (the interpreter re-stamps the copy's
/// width and sign; both compiled lanes decline such a read and fall back to it,
/// so a divergence here is the "one read, two answers" hazard).
fn prints_all(src: &str, want: &[&str]) {
    for b in ["native", "interp", "vm"] {
        let (out, code) = run_backend(src, b);
        assert_eq!(code, Some(0), "[{b}] exit\n{out}");
        let got: Vec<&str> = out.lines().filter(|l| l.starts_with("D=")).collect();
        assert_eq!(got, want, "[{b}]\n{out}");
    }
}

/// A signed 8-bit `v` with its sign-extending 16-bit copy `c`, plus `decls`, and
/// one `initial` running `body`.
fn top(decls: &str, body: &str) -> String {
    format!(
        "`timescale 1ns/1ns\nmodule top;\n  logic signed [7:0] v;\n  logic [15:0] c;\n  \
         assign c = v;\n  {decls}\n  logic [15:0] r2;\n  \
         initial begin {body} #5 $finish; end\nendmodule\n"
    )
}

const RD: &str = "task automatic rd(output logic [15:0] o); o = c; endtask";

#[test]
fn a_sign_extending_copy_reads_through_inside_a_called_frame() {
    // the callee body's own read (both oracles `ffa5`)
    prints_all(
        &top(RD, "v = 8'hA5; rd(r2); $display(\"D=%h\", r2);"),
        &["D=ffa5"],
    );
    // a FUNCTION callee, reached from an expression rather than a terminator
    prints_all(
        &top(
            "function automatic logic [15:0] g(); return c; endfunction",
            "v = 8'hA5; r2 = g(); $display(\"D=%h\", r2);",
        ),
        &["D=ffa5"],
    );
    // the call's in-bind ACTUAL, whose expression lives in the task-call sidecar
    prints_all(
        &top(
            "task automatic tk(input logic [15:0] x, output logic [15:0] y); y = x; endtask",
            "v = 8'hA5; tk(c, r2); $display(\"D=%h\", r2);",
        ),
        &["D=ffa5"],
    );
    // a NESTED callee (task calls task): the transitive walk
    prints_all(
        &top(
            "task automatic inner(output logic [15:0] o); o = c; endtask\n  \
             task automatic outer(output logic [15:0] o); inner(o); endtask",
            "v = 8'hA5; outer(r2); $display(\"D=%h\", r2);",
        ),
        &["D=ffa5"],
    );
    // a STATIC (non-automatic) task
    prints_all(
        &top(
            "logic [15:0] sr;\n  task rds(); sr = c; endtask",
            "v = 8'hA5; rds(); $display(\"D=%h\", sr);",
        ),
        &["D=ffa5"],
    );
}

#[test]
fn the_process_body_twin_and_a_write_inside_a_callee() {
    // the same read in the process's OWN body — one defect, not a callee-only one
    prints_all(
        &top("", "v = 8'hA5; r2 = c; $display(\"D=%h\", r2);"),
        &["D=ffa5"],
    );
    // the writer predicate's side: `v` is written INSIDE a callee, read outside
    prints_all(
        &top(
            "task automatic wr(input logic signed [7:0] x); v = x; endtask",
            "wr(8'hA5); r2 = c; $display(\"D=%h\", r2);",
        ),
        &["D=ffa5"],
    );
    // a direct `$display(c)` — the read with no destination at all
    prints_all(&top("", "v = 8'hA5; $display(\"D=%h\", c);"), &["D=ffa5"]);
    // two reads of `c`, and a re-write between them (iverilog `ffa5` then `003c`;
    // verilator answers `0000` for BOTH, which is the self-contradiction above)
    prints_all(
        &top(
            RD,
            "v = 8'hA5; rd(r2); $display(\"D=%h\", r2); v = 8'h3C; rd(r2); $display(\"D=%h\", r2);",
        ),
        &["D=ffa5", "D=003c"],
    );
    // the read inside an operator's context
    prints_all(
        &top(
            "task automatic rdp(output logic [15:0] o); o = c + 16'd1; endtask",
            "v = 8'hA5; rdp(r2); $display(\"D=%h\", r2);",
        ),
        &["D=ffa6"],
    );
}

#[test]
fn the_extension_takes_the_sources_sign_and_the_read_takes_the_copys() {
    // The copy's width is reached by the ASSIGNMENT (the rhs is self-determined in
    // sign, so a signed source sign-extends into an unsigned copy); the read then
    // carries the COPY's sign into its own context. Both oracles on both rows —
    // and the two rows differ only in the copy's declared sign, so a single-stage
    // rule cannot answer both (an `ffa5` copy zero-extended vs sign-extended).
    prints_all(
        &format!(
            "`timescale 1ns/1ns\nmodule top;\n  logic signed [7:0] v;\n  logic [15:0] c;\n  \
             assign c = v;\n  {RD}\n  int r;\n  \
             initial begin logic [15:0] y; v = 8'hA5; rd(y); r = y; $display(\"D=%h\", r);\n    \
             r = c; $display(\"D=%h\", r); #5 $finish; end\nendmodule\n"
        ),
        &["D=0000ffa5", "D=0000ffa5"],
    );
    prints_all(
        "`timescale 1ns/1ns\nmodule top;\n  logic signed [7:0] v;\n  logic signed [15:0] c;\n  \
         assign c = v;\n  int r;\n  \
         initial begin v = 8'hA5; r = c; $display(\"D=%h\", r); #5 $finish; end\nendmodule\n",
        &["D=ffffffa5"],
    );
    // an `integer` source (signed 32) into a 64-bit copy
    prints_all(
        "`timescale 1ns/1ns\nmodule top;\n  integer v;\n  logic [63:0] c;\n  assign c = v;\n  \
         task automatic rd(output logic [63:0] o); o = c; endtask\n  \
         initial begin logic [63:0] y; v = -2; rd(y); $display(\"D=%h\", y); #5 $finish; end\n\
         endmodule\n",
        &["D=fffffffffffffffe"],
    );
    // a CHAIN: the same-width tail collapses onto the extension's root, which is
    // exact only because the tail does not change the width again
    prints_all(
        "`timescale 1ns/1ns\nmodule top;\n  logic signed [7:0] v;\n  logic [15:0] c;\n  \
         assign c = v;\n  logic [15:0] d;\n  assign d = c;\n  \
         task automatic rd(output logic [15:0] o); o = d; endtask\n  \
         initial begin logic [15:0] y; v = 8'hA5; rd(y); $display(\"D=%h\", y); #5 $finish; end\n\
         endmodule\n",
        &["D=ffa5"],
    );
    // ⭐ THE CELL THAT MAKES THE COMPILED LANES' DECLINE LOAD-BEARING. Signs
    // MATCH here (both signed), so the §4.5.442 sign half of
    // `alias::alias_read_needs_restamp` admits it and only the WIDTH half
    // declines; the read then lands in an UNSIGNED context (`+ 32'd0`), where a
    // compiled `LoadScalar` would take the source slot's 8 bits and the
    // context's unsignedness and zero-extend the source — `000000a5`. Measured
    // by mutation: with the width half removed, `native` and `vm` answer
    // `000000a5` while `interp` and both oracles answer `0000ffa5`.
    prints_all(
        "`timescale 1ns/1ns\nmodule top;\n  logic signed [7:0] v;\n  logic signed [15:0] c;\n  \
         assign c = v;\n  logic [31:0] r;\n  logic clk = 0;\n  always #1 clk = ~clk;\n  \
         always @(posedge clk) begin v = 8'hA5; r = c + 32'd0; end\n  \
         initial begin #5 $display(\"D=%h\", r); $finish; end\nendmodule\n",
        &["D=0000ffa5"],
    );
    // a `wire` destination is the same driver under another declaration
    prints_all(
        "`timescale 1ns/1ns\nmodule top;\n  logic signed [7:0] v;\n  wire [15:0] c;\n  \
         assign c = v;\n  task automatic rd(output logic [15:0] o); o = c; endtask\n  \
         initial begin logic [15:0] y; v = 8'hA5; rd(y); $display(\"D=%h\", y); #5 $finish; end\n\
         endmodule\n",
        &["D=ffa5"],
    );
}

#[test]
fn a_zero_extending_or_truncating_copy_keeps_the_settles_value() {
    // NOT an oracle pin — a REGRESSION pin. iverilog reads neither of these
    // through (it builds a `.concat` and a select, which do not propagate on the
    // store) and verilator reads both through; vita keeps the value it had, which
    // is what the oracles' disagreement leaves it entitled to. If a future slice
    // widens the admission to these shapes it must move these lines deliberately.
    let zero_ext = "`timescale 1ns/1ns\nmodule top;\n  logic [7:0] v;\n  logic [15:0] c;\n  \
                    assign c = v;\n  task automatic rd(output logic [15:0] o); o = c; endtask\n  \
                    initial begin logic [15:0] y; v = 8'hA5; rd(y); $display(\"D=%h\", y);\n    \
                    #1 rd(y); $display(\"D=%h\", y); #5 $finish; end\nendmodule\n";
    prints_all(zero_ext, &["D=00xx", "D=00a5"]);
    let trunc = "`timescale 1ns/1ns\nmodule top;\n  logic signed [15:0] v;\n  logic [7:0] c;\n  \
                 assign c = v;\n  task automatic rd(output logic [7:0] o); o = c; endtask\n  \
                 initial begin logic [7:0] y; v = 16'hA55A; rd(y); $display(\"D=%h\", y);\n    \
                 #1 rd(y); $display(\"D=%h\", y); #5 $finish; end\nendmodule\n";
    prints_all(trunc, &["D=xx", "D=5a"]);
    // a full-range part-select rhs into a WIDER destination: a part-select is
    // unsigned, so this driver zero-extends and stays outside the set
    let sel = "`timescale 1ns/1ns\nmodule top;\n  logic signed [7:0] v;\n  logic [15:0] c;\n  \
               assign c = v[7:0];\n  task automatic rd(output logic [15:0] o); o = c; endtask\n  \
               initial begin logic [15:0] y; v = 8'hA5; rd(y); $display(\"D=%h\", y); #5 $finish; \
               end\nendmodule\n";
    prints_all(sel, &["D=00xx"]);
}

#[test]
fn the_controls_are_unchanged() {
    // same-width copy inside a callee: §4.5.438's cell, still the oracles' `a5`
    prints_all(
        "`timescale 1ns/1ns\nmodule top;\n  logic [7:0] v, c;\n  assign c = v;\n  \
         task automatic rd(output logic [7:0] o); o = c; endtask\n  \
         initial begin logic [7:0] y; v = 8'hA5; rd(y); $display(\"D=%h\", y); #5 $finish; end\n\
         endmodule\n",
        &["D=a5"],
    );
    // the SETTLED read (`#1` later) is the copy's own value, not a re-derivation:
    // it was `ffa5` before this slice and must stay `ffa5`
    prints_all(
        &top(RD, "v = 8'hA5; #1 rd(r2); $display(\"D=%h\", r2);"),
        &["D=ffa5"],
    );
    // no write of the source at all: the settle's value, both oracles
    prints_all(
        &format!(
            "`timescale 1ns/1ns\nmodule top;\n  logic signed [7:0] v = 8'hA5;\n  \
             logic [15:0] c;\n  assign c = v;\n  logic [7:0] q;\n  {RD}\n  \
             initial begin logic [15:0] y; q = 8'h11; rd(y); $display(\"D=%h\", y); #5 $finish; \
             end\nendmodule\n"
        ),
        &["D=ffa5"],
    );
    // MIXED CALLER POPULATION (§4.5.438's pin): a callee body is ONE set of
    // expressions, so its read is marked only for the roots EVERY calling process
    // writes — one writer and one non-writer leave both on the settle's value.
    // iverilog reads both through; verilator splits (`0000` / `ffa5`), so the pin
    // stays vita's own, as it was before this slice.
    prints_all(
        &format!(
            "`timescale 1ns/1ns\nmodule top;\n  logic signed [7:0] v;\n  logic [15:0] c;\n  \
             assign c = v;\n  logic [15:0] y1, y2;\n  {RD}\n  \
             initial begin v = 8'hA5; rd(y1); $display(\"D=%h\", y1); end\n  \
             initial begin rd(y2); $display(\"D=%h\", y2); end\n  \
             initial begin #5 $finish; end\nendmodule\n"
        ),
        &["D=xxxx", "D=xxxx"],
    );
    // ANOTHER process's read in the same delta is a §5.4.1 race (ROADMAP §2 🆕 I ⓐ)
    // and keeps the settle's value, exactly as the same-width twin does.
    prints_all(
        &format!(
            "`timescale 1ns/1ns\nmodule top;\n  logic signed [7:0] v;\n  logic [15:0] c;\n  \
             assign c = v;\n  logic [15:0] y2;\n  {RD}\n  \
             initial begin v = 8'hA5; end\n  \
             initial begin rd(y2); $display(\"D=%h\", y2); end\n  \
             initial begin #5 $finish; end\nendmodule\n"
        ),
        &["D=xxxx"],
    );
}
