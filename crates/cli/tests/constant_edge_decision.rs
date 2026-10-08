//! §4.5.601 round 3 — every part-select width, indexed width and replication count is
//! decided: its IEEE value (the constant domain's, else the lowered tree's own region),
//! or E3009. One pin per review cell (the two round-2 lenses, the design review), each
//! with the oracles' text beside it: verilator 5.052 and sv2v 0.0.13 -> iverilog 13.0;
//! iverilog alone refuses a hierarchical reference in a constant expression. PRE is
//! vita at 1d23be89 (md5 f4d778f2).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// stdout without the `simulation ended` line, success, stderr
fn run_raw(src: &str) -> (String, bool, String) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_ced_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    let so = String::from_utf8_lossy(&out.stdout);
    let s: Vec<&str> = so
        .lines()
        .filter(|l| !l.starts_with("simulation ended"))
        .collect();
    (
        s.join("\n"),
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

enum Want {
    Prints(&'static str),
    Refused(&'static str),
}

fn check(cells: &[(&str, &str, Want)]) {
    let mut bad = Vec::new();
    for (name, src, want) in cells {
        let (out, ok, err) = run_raw(src);
        let pass = match want {
            Want::Prints(w) => ok && out.trim_end() == *w,
            Want::Refused(n) => !ok && err.contains("error[VITA-E") && err.contains(n),
        };
        if !pass {
            bad.push(format!("{name}: stdout {out:?}\nstderr {err}"));
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n---\n"));
}

/// A width or count whose leaves are at or past 2^24, where the engine's fold clamps
/// each leaf (`32'h1000_0003 - 32'h1000_0000` read as 0, so a select of one bit): the
/// constant domain's value is handed over as a `Const`. PRE reads one bit or zero copies.
#[test]
fn g1_values_decided_while_lowering() {
    check(&[
        // ra05_literal_large_diff_rep: oracles V:r=0000000f; PRE V:r=00000000
        (
            "ra05_literal_large_diff_rep",
            r#"module top;
  logic [31:0] f, r, a;
  logic [0:31] g, g2;
  integer n;
  initial begin
    f = 32'h89ab_cdef; g = 32'h89ab_cdef; g2 = 0; r = 0; a = 0; n = 0;
    r = {(32'h1000_0004 - 32'h1000_0000){1'b1}};
    #2 $display("r=%h", r);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=0000000f"),
        ),
        // ra06_localparam_large_diff_rep: oracles V:r=0000000f; PRE V:r=00000000
        (
            "ra06_localparam_large_diff_rep",
            r#"module top;
  localparam int LO = 32'h1000_0000; localparam int HI = 32'h1000_0004;
  logic [31:0] f, r, a;
  logic [0:31] g, g2;
  integer n;
  initial begin
    f = 32'h89ab_cdef; g = 32'h89ab_cdef; g2 = 0; r = 0; a = 0; n = 0;
    r = {(HI - LO){1'b1}};
    #2 $display("r=%h", r);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=0000000f"),
        ),
        // ra14_literal_large_diff_psel: oracles V:a=00000000 r=0000000f; PRE V:r=00000001 a=00000000
        (
            "ra14_literal_large_diff_psel",
            r#"module top;
  localparam int LO = 32'h1000_0000; localparam int HI = 32'h1000_0004;
  logic [31:0] f, r, a;
  logic [HI-LO-1:0] dv;
  initial begin
    f = 32'h89ab_cdef; r = 0; a = 0;
    r = f[32'h1000_0003 - 32'h1000_0000 : 0];
    #1 $display("r=%h a=%h", r, a);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=0000000f a=00000000"),
        ),
        // a01_addr_part: oracles V:r=0000000f; PRE V:r=00000001
        (
            "a01_addr_part",
            r#"module top;
  localparam int unsigned MEM_BASE = 32'h8000_0000;
  localparam int unsigned MEM_END = 32'h8000_0003;
  localparam int A = 20000000;
  localparam int B24 = 16777216;
  localparam int B24m = 16777215;
  logic [31:0] f, r, a;
  logic [0:31] g;
  integer i, n;
  initial begin
    f = 32'hffff_ffff; g = 32'hffff_ffff; r = 0; a = 0; n = 0;
    r = f[MEM_END - MEM_BASE : 0];
    #1 $display("r=%h", r);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=0000000f"),
        ),
        // a02_addr_rep: oracles V:r=0000000f; PRE V:r=00000001
        (
            "a02_addr_rep",
            r#"module top;
  localparam int unsigned MEM_BASE = 32'h8000_0000;
  localparam int unsigned MEM_END = 32'h8000_0003;
  localparam int A = 20000000;
  localparam int B24 = 16777216;
  localparam int B24m = 16777215;
  logic [31:0] f, r, a;
  logic [0:31] g;
  integer i, n;
  initial begin
    f = 32'hffff_ffff; g = 32'hffff_ffff; r = 0; a = 0; n = 0;
    r = {(MEM_END - MEM_BASE + 1){1'b1}};
    #1 $display("r=%h", r);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=0000000f"),
        ),
        // a03_addr_idx: oracles V:r=0000000f; PRE V:r=00000001
        (
            "a03_addr_idx",
            r#"module top;
  localparam int unsigned MEM_BASE = 32'h8000_0000;
  localparam int unsigned MEM_END = 32'h8000_0003;
  localparam int A = 20000000;
  localparam int B24 = 16777216;
  localparam int B24m = 16777215;
  logic [31:0] f, r, a;
  logic [0:31] g;
  integer i, n;
  initial begin
    f = 32'hffff_ffff; g = 32'hffff_ffff; r = 0; a = 0; n = 0;
    r = f[0 +: MEM_END - MEM_BASE + 1];
    #1 $display("r=%h", r);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=0000000f"),
        ),
        // a04_addr_write_fill: oracles V:r=0000000f; PRE V:r=00000001
        (
            "a04_addr_write_fill",
            r#"module top;
  localparam int unsigned MEM_BASE = 32'h8000_0000;
  localparam int unsigned MEM_END = 32'h8000_0003;
  localparam int A = 20000000;
  localparam int B24 = 16777216;
  localparam int B24m = 16777215;
  logic [31:0] f, r, a;
  logic [0:31] g;
  integer i, n;
  initial begin
    f = 32'hffff_ffff; g = 32'hffff_ffff; r = 0; a = 0; n = 0;
    a[MEM_END - MEM_BASE : 0] = '1;
    #1 $display("r=%h", a);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=0000000f"),
        ),
        // a08_dec_rep_concat: oracles V:r=00000297; PRE V:r=000000a5
        (
            "a08_dec_rep_concat",
            r#"module top;
  localparam int unsigned MEM_BASE = 32'h8000_0000;
  localparam int unsigned MEM_END = 32'h8000_0003;
  localparam int A = 20000000;
  localparam int B24 = 16777216;
  localparam int B24m = 16777215;
  logic [31:0] f, r, a;
  logic [0:31] g;
  integer i, n;
  initial begin
    f = 32'hffff_ffff; g = 32'hffff_ffff; r = 0; a = 0; n = 0;
    r = {8'hA5, {(A + 2 - A){1'b1}}};
    #1 $display("r=%h", r);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=00000297"),
        ),
        // a06_addr_ca: oracles V:r=0000000f; PRE V:r=0000000Z
        (
            "a06_addr_ca",
            r#"module top;
  localparam int unsigned MEM_BASE = 32'h8000_0000;
  localparam int unsigned MEM_END = 32'h8000_0003;
  localparam int A = 20000000;
  localparam int B24 = 16777216;
  localparam int B24m = 16777215;
  logic [31:0] f, r, a;
  logic [0:31] g;
  integer i, n;
  wire [31:0] w;
  assign w[MEM_END - MEM_BASE : 0] = 4'hf;
  assign w[31:4] = '0;
  initial begin
    f = 32'hffff_ffff; g = 32'hffff_ffff; r = 0; a = 0; n = 0;

    #1 $display("r=%h", w);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=0000000f"),
        ),
        // a14_addr_ia: oracles V:r=0000000f; PRE V:r=00000001
        (
            "a14_addr_ia",
            r#"module top;
  localparam int unsigned MEM_BASE = 32'h8000_0000;
  localparam int unsigned MEM_END = 32'h8000_0003;
  localparam int A = 20000000;
  localparam int B24 = 16777216;
  localparam int B24m = 16777215;
  logic [31:0] f, r, a;
  logic [0:31] g;
  integer i, n;
  initial begin
    f = 32'hffff_ffff; g = 32'hffff_ffff; r = 0; a = 0; n = 0;
    a[MEM_END - MEM_BASE : 0] = #1 f;
    #1;
    #1 $display("r=%h", a);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=0000000f"),
        ),
        // s03_intermediate_clamp: oracles V:r=89abcdef; PRE V:r=0000000f
        (
            "s03_intermediate_clamp",
            r#"module top;
  logic [31:0] f, r;
  localparam int OFF = -2;
  localparam int ADJ = -1;
  localparam int H = 9000000;
  if (1) begin : gb localparam int L = 5; end
  initial begin
    f = 32'h89ab_cdef; r = 0;
    r = f[H + H - 17000000 + 3 : 0];
    #1 $display("r=%h", r);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=89abcdef"),
        ),
    ]);
}

/// The same values over a generate-block or instance name, decided after the deferred
/// passes; also a task output copied to such a target.
#[test]
fn late_g1_values() {
    check(&[
        // ra01_genblk_large_diff_rep: oracles V:r=0000000f; PRE V:r=00000000
        (
            "ra01_genblk_large_diff_rep",
            r#"module top;
  if (1) begin : gb localparam int LO = 32'h1000_0000; localparam int HI = 32'h1000_0004; end
  logic [31:0] f, r, a;
  logic [0:31] g, g2;
  integer n;
  initial begin
    f = 32'h89ab_cdef; g = 32'h89ab_cdef; g2 = 0; r = 0; a = 0; n = 0;
    r = {(gb.HI - gb.LO){1'b1}};
    #2 $display("r=%h", r);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=0000000f"),
        ),
        // a20_hier_addr_part: oracles V:r=0000000f; PRE V:r=00000001
        (
            "a20_hier_addr_part",
            r#"module sub; parameter int unsigned E = 32'h8000_0003; parameter int unsigned B = 32'h8000_0000; endmodule
module top;
  logic [31:0] f, r;
  sub u();
  initial begin
    f = 32'hffff_ffff; r = 0;
    r = f[u.E - u.B : 0];
    #1 $display("r=%h", r);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=0000000f"),
        ),
        // ra07_genblk_large_diff_write: oracles V:a=0000000f; PRE V:a=00000001
        (
            "ra07_genblk_large_diff_write",
            r#"module top;
  if (1) begin : gb localparam int LO = 32'h1000_0000; localparam int HI = 32'h1000_0004; end
  logic [31:0] f, r, a;
  logic [0:31] g, g2;
  integer n;
  initial begin
    f = 32'h89ab_cdef; g = 32'h89ab_cdef; g2 = 0; r = 0; a = 0; n = 0;
    a[gb.HI - gb.LO - 1 : 0] = 4'hf;
    #2 $display("a=%h", a);
    $finish;
  end
endmodule
"#,
            Want::Prints("a=0000000f"),
        ),
        // t1_task_out_late_g1: oracles s2v a=ffff0015; vl a=ffff0015
        (
            "t1_task_out_late_g1",
            r#"module top;
  if (1) begin : gb localparam logic [31:0] HI = 32'h1000_0004; localparam logic [31:0] LO = 32'h1000_0000; end
  logic [31:0] a;
  task automatic t(output logic [4:0] y); y = 5'h15; endtask
  initial begin
    a = 32'hffff_0000;
    t(a[gb.HI - gb.LO : 0]);
    #1 $display("a=%h", a);
    $finish;
  end
endmodule
"#,
            Want::Prints("a=ffff0015"),
        ),
    ]);
}

/// A late bound is one self-determined region: it wraps at its width and is read at its
/// sign (`gb.U + 6` with `U = 32'hFFFF_FFFE` is 4; `gb.N + 6` with `N = -2` is 4); the
/// engine's fold clamps and saturates instead.
#[test]
fn late_wrapping_and_signed_values() {
    check(&[
        // rd03_genblk_unsigned_wrap: oracles V:r=0000000f; PRE V:r=89abcdef
        (
            "rd03_genblk_unsigned_wrap",
            r#"module top;
  if (1) begin : gb localparam logic [31:0] U = 32'hFFFF_FFFE; end
  logic [31:0] f, r, a;
  logic [0:31] g, g2;
  integer n;
  initial begin
    f = 32'h89ab_cdef; g = 32'h89ab_cdef; g2 = 0; r = 0; a = 0; n = 0;
    r = f[gb.U + 6 : 0];
    #2 $display("r=%h", r);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=0000000f"),
        ),
        // rd07_genblk_wrap_to_zero_count: oracles V:r=000000a5; PRE V:r=ffffffff
        (
            "rd07_genblk_wrap_to_zero_count",
            r#"module top;
  if (1) begin : gb localparam logic [31:0] BIG = 32'hFFFF_FFFC; end
  logic [31:0] f, r, a;
  logic [0:31] g, g2;
  integer n;
  initial begin
    f = 32'h89ab_cdef; g = 32'h89ab_cdef; g2 = 0; r = 0; a = 0; n = 0;
    r = {8'hA5, {(gb.BIG + 4){1'b1}}};
    #2 $display("r=%h", r);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=000000a5"),
        ),
        // rd08_genblk_wrap_idx_width: oracles V:r=0000000f; PRE V:r=89abcdef
        (
            "rd08_genblk_wrap_idx_width",
            r#"module top;
  if (1) begin : gb localparam logic [31:0] U = 32'hFFFF_FFFE; end
  logic [31:0] f, r, a;
  logic [0:31] g, g2;
  integer n;
  initial begin
    f = 32'h89ab_cdef; g = 32'h89ab_cdef; g2 = 0; r = 0; a = 0; n = 0;
    r = f[0 +: gb.U + 6];
    #2 $display("r=%h", r);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=0000000f"),
        ),
        // rd01_genblk_neg_leaf_pos_sum: oracles V:r=0000000f; PRE V:r=89abcdef
        (
            "rd01_genblk_neg_leaf_pos_sum",
            r#"module top;
  if (1) begin : gb localparam int N = -2; end
  logic [31:0] f, r, a;
  logic [0:31] g, g2;
  integer n;
  initial begin
    f = 32'h89ab_cdef; g = 32'h89ab_cdef; g2 = 0; r = 0; a = 0; n = 0;
    r = f[gb.N+6:0];
    #2 $display("r=%h", r);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=0000000f"),
        ),
        // rd02_genblk_neg_leaf_rep: oracles V:r=0000000f; PRE V:r=ffffffff
        (
            "rd02_genblk_neg_leaf_rep",
            r#"module top;
  if (1) begin : gb localparam int N = -2; end
  logic [31:0] f, r, a;
  logic [0:31] g, g2;
  integer n;
  initial begin
    f = 32'h89ab_cdef; g = 32'h89ab_cdef; g2 = 0; r = 0; a = 0; n = 0;
    r = {(gb.N+6){1'b1}};
    #2 $display("r=%h", r);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=0000000f"),
        ),
        // rd06_genblk_intermediate_neg: oracles V:r=0000000f; PRE V:r=0000002f
        (
            "rd06_genblk_intermediate_neg",
            r#"module top;
  if (1) begin : gb localparam int A = 1; localparam int B = 3; end
  logic [31:0] f, r, a;
  logic [0:31] g, g2;
  integer n;
  initial begin
    f = 32'h89ab_cdef; g = 32'h89ab_cdef; g2 = 0; r = 0; a = 0; n = 0;
    r = f[gb.A - gb.B + 5 : 0];
    #2 $display("r=%h", r);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=0000000f"),
        ),
        // rd09_genblk_signed_neg_lsb: oracles V:r=0000000f; PRE V:r=00000001
        (
            "rd09_genblk_signed_neg_lsb",
            r#"module top;
  if (1) begin : gb localparam int N = -2; end
  logic [31:0] f, r, a;
  logic [0:31] g, g2;
  integer n;
  initial begin
    f = 32'h89ab_cdef; g = 32'h89ab_cdef; g2 = 0; r = 0; a = 0; n = 0;
    r = f[3 : gb.N+2];
    #2 $display("r=%h", r);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=0000000f"),
        ),
        // rd12_genblk_signed4_signed_ctx: oracles V:a=00000000 r=0000000f; PRE V:r=000bcdef a=00000000
        (
            "rd12_genblk_signed4_signed_ctx",
            r#"module top;
  if (1) begin : gb localparam logic signed [3:0] S = 4'sb1111; end
  logic [31:0] f, r, a;
  integer n;
  initial begin
    f = 32'h89ab_cdef; r = 0; a = 0; n = 0;
    r = f[gb.S + 5 : 0];
    #2 $display("r=%h a=%h", r, a);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=0000000f a=00000000"),
        ),
        // s02_signed_onesided: oracles V:r=0000000f; PRE V:r=89abcdef
        (
            "s02_signed_onesided",
            r#"module top;
  logic [31:0] f, r;
  localparam int OFF = -2;
  localparam int ADJ = -1;
  localparam int H = 9000000;
  if (1) begin : gb localparam int L = 5; end
  initial begin
    f = 32'h89ab_cdef; r = 0;
    r = f[gb.L + ADJ : 0];
    #1 $display("r=%h", r);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=0000000f"),
        ),
        // d2_gb_untyped_sized: oracles iv IV_BUILD_FAILED; sv2v r=00003fff; ver r=00003fff; PRE warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in
        (
            "d2_gb_untyped_sized",
            r#"typedef logic [3:0] nib_t;
module top;
  logic [31:0] f = 32'h89ab_cdef, r, v;
  if (1) begin : gb
    localparam L = 4'd15;
  end
  initial begin
    #1 r = {(gb.L + gb.L){1'b1}};
    v = {gb.L + gb.L};
    $display("r=%h", r);
    $display("v=%h b=%0d", v, $bits(gb.L));
  end
endmodule
"#,
            Want::Prints("r=00003fff\nv=0000000e b=4"),
        ),
        // d5_gb_typedef4: oracles iv IV_BUILD_FAILED; sv2v r=00003fff; PRE warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in
        (
            "d5_gb_typedef4",
            r#"typedef logic [3:0] nib_t;
module top;
  logic [31:0] f = 32'h89ab_cdef, r, v;
  if (1) begin : gb
    localparam nib_t L = 4'd15;
  end
  initial begin
    #1 r = {(gb.L + gb.L){1'b1}};
    v = {gb.L + gb.L};
    $display("r=%h", r);
    $display("v=%h b=%0d", v, $bits(gb.L));
  end
endmodule
"#,
            Want::Prints("r=00003fff\nv=0000000e b=4"),
        ),
        // d6_gb_wrap8_psel: oracles iv IV_BUILD_FAILED; sv2v r=0000000f; PRE warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in
        (
            "d6_gb_wrap8_psel",
            r#"module top;
  logic [31:0] f = 32'h89ab_cdef, r, v;
  if (1) begin : gb
    localparam logic [7:0] L = 8'd200;
  end
  initial begin
    #1 r = f[gb.L + gb.L - 8'd140 : 0];
    v = {gb.L + gb.L - 8'd140};
    $display("r=%h", r);
    $display("v=%h", v);
  end
endmodule
"#,
            Want::Prints("r=0000000f\nv=00000004"),
        ),
        // d7_gb_signed8_psel: oracles iv IV_BUILD_FAILED; sv2v r=0000000f; ver r=0000000f; PRE warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in
        (
            "d7_gb_signed8_psel",
            r#"module top;
  logic [31:0] f = 32'h89ab_cdef, r, v;
  if (1) begin : gb
    localparam logic signed [7:0] L = -8'sd3;
  end
  initial begin
    #1 r = f[gb.L + 8'sd7 : 0];
    v = {gb.L + 8'sd7};
    $display("r=%h", r);
    $display("v=%h", v);
  end
endmodule
"#,
            Want::Prints("r=0000000f\nv=00000004"),
        ),
    ]);
}

/// A signed leaf in a region with an unsigned operand is unsigned (IEEE §11.8.1):
/// `4'sb1111 + 5'd5` is 20. Round 2 refused these as negative.
#[test]
fn signed_leaves_in_unsigned_regions() {
    check(&[
        // ri01_lit_signed_neg_unsigned_ctx: oracles V:r=000bcdef; PRE V:r=000bcdef
        (
            "ri01_lit_signed_neg_unsigned_ctx",
            r#"module top;

  logic [31:0] f, r;
  logic [0:31] g;
  initial begin
    f = 32'h89ab_cdef; g = 32'h89ab_cdef; r = 0;
    r = f[4'sb1111 + 5'd5 : 0];
    #1 $display("r=%h", r);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=000bcdef"),
        ),
        // rg09_lp_s4_plus_unsigned_param: oracles V:r=000bcdef; PRE V:r=000bcdef
        (
            "rg09_lp_s4_plus_unsigned_param",
            r#"module top;
  localparam logic signed [3:0] S = 4'sb1111; localparam logic [4:0] U5 = 5'd5;
  logic [31:0] f, r;
  initial begin
    f = 32'h89ab_cdef; r = 0;
    r = f[S + U5 : 0];
    #1 $display("r=%h", r);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=000bcdef"),
        ),
        // rd16_lp_signed4_unsigned_ctx: oracles V:a=00000000 r=000bcdef; PRE V:r=000bcdef a=00000000
        (
            "rd16_lp_signed4_unsigned_ctx",
            r#"module top;
  localparam logic signed [3:0] S = 4'sb1111;
  logic [31:0] f, r, a;
  integer n;
  initial begin
    f = 32'h89ab_cdef; r = 0; a = 0; n = 0;
    r = f[S + 5'd5 : 0];
    #2 $display("r=%h a=%h", r, a);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=000bcdef a=00000000"),
        ),
        // ri02_lp_signed_neg_lsb_unsigned: oracles V:r=0000001a; PRE V:r=0000001a
        (
            "ri02_lp_signed_neg_lsb_unsigned",
            r#"module top;
  localparam logic signed [3:0] S = 4'sb1111; localparam logic [4:0] U5 = 5'd5;
  logic [31:0] f, r;
  logic [0:31] g;
  initial begin
    f = 32'h89ab_cdef; g = 32'h89ab_cdef; r = 0;
    r = f[S + U5 + 4 : S + U5];
    #1 $display("r=%h", r);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=0000001a"),
        ),
        // ri07_lp_signed_neg_write: oracles V:r=001fffff; PRE V:r=001fffff
        (
            "ri07_lp_signed_neg_write",
            r#"module top;
  localparam logic signed [3:0] S = 4'sb1111; localparam logic [4:0] U5 = 5'd5;
  logic [31:0] f, r;
  logic [0:31] g;
  initial begin
    f = 32'h89ab_cdef; g = 32'h89ab_cdef; r = 0;
    r = 0; r[S + U5 : 0] = '1;
    #1 $display("r=%h", r);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=001fffff"),
        ),
        // ri08_lp_signed_byte_unsigned: oracles V:r=000001ef; PRE V:r=000001ef
        (
            "ri08_lp_signed_byte_unsigned",
            r#"module top;
  localparam logic signed [7:0] SB = 8'sb1000_0000; localparam logic [8:0] U9 = 9'd0;
  logic [31:0] f, r;
  logic [0:31] g;
  initial begin
    f = 32'h89ab_cdef; g = 32'h89ab_cdef; r = 0;
    r = f[SB + U9 - 120 : 0];
    #1 $display("r=%h", r);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=000001ef"),
        ),
        // rd11_genblk_signed4_unsigned_ctx: oracles V:a=00000000 r=000bcdef; PRE V:r=000bcdef a=00000000
        (
            "rd11_genblk_signed4_unsigned_ctx",
            r#"module top;
  if (1) begin : gb localparam logic signed [3:0] S = 4'sb1111; end
  logic [31:0] f, r, a;
  integer n;
  initial begin
    f = 32'h89ab_cdef; r = 0; a = 0; n = 0;
    r = f[gb.S + 5'd5 : 0];
    #2 $display("r=%h a=%h", r, a);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=000bcdef a=00000000"),
        ),
        // rd15_genblk_signed4_rep_unsigned: oracles V:a=00000000 r=0000ffff; PRE V:r=0000ffff a=00000000
        (
            "rd15_genblk_signed4_rep_unsigned",
            r#"module top;
  if (1) begin : gb localparam logic signed [3:0] S = 4'sb1111; end
  logic [31:0] f, r, a;
  integer n;
  initial begin
    f = 32'h89ab_cdef; r = 0; a = 0; n = 0;
    r = {(gb.S + 5'd1){1'b1}};
    #2 $display("r=%h a=%h", r, a);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=0000ffff a=00000000"),
        ),
        // s01_signed_cancel: oracles V:r=00000007; PRE V:r=00000007
        (
            "s01_signed_cancel",
            r#"module top;
  logic [31:0] f, r;
  localparam int OFF = -2;
  localparam int ADJ = -1;
  localparam int H = 9000000;
  if (1) begin : gb localparam int L = 5; end
  initial begin
    f = 32'h89ab_cdef; r = 0;
    r = f[gb.L + OFF : gb.L + OFF - 2];
    #1 $display("r=%h", r);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=00000007"),
        ),
    ]);
}

/// A fill in a region of its own (a comparison, a logical or a reduction operand) never
/// took the target's width, so a late target does not matter to it. Round 2 refused these.
#[test]
fn fills_in_a_new_region_beside_a_late_target() {
    check(&[
        // rc08_cmp_fill: oracles V:a=00000001; PRE V:a=00000001
        (
            "rc08_cmp_fill",
            r#"module top;
  if (1) begin : gb localparam int L = 5; end
  logic [31:0] f, r, a;
  logic [0:31] g, g2;
  integer n;
  initial begin
    f = 32'h89ab_cdef; g = 32'h89ab_cdef; g2 = 0; r = 0; a = 0; n = 0;
    a[gb.L-1:0] = (f[3:0] == '1);
    #2 $display("a=%h", a);
    $finish;
  end
endmodule
"#,
            Want::Prints("a=00000001"),
        ),
        // rc09_cmp_fill_in_cond8: oracles V:a=00000003; PRE V:a=00000003
        (
            "rc09_cmp_fill_in_cond8",
            r#"module top;
  if (1) begin : gb localparam int L = 8; end
  logic [31:0] f, r, a;
  logic [0:31] g, g2;
  integer n;
  initial begin
    f = 32'h89ab_cdef; g = 32'h89ab_cdef; g2 = 0; r = 0; a = 0; n = 0;
    a[gb.L-1:0] = (f[3:0] == '1) ? 5'd3 : 5'd4;
    #2 $display("a=%h", a);
    $finish;
  end
endmodule
"#,
            Want::Prints("a=00000003"),
        ),
        // rc10_logical_fill: oracles V:a=00000001; PRE V:a=00000001
        (
            "rc10_logical_fill",
            r#"module top;
  if (1) begin : gb localparam int L = 5; end
  logic [31:0] f, r, a;
  logic [0:31] g, g2;
  integer n;
  initial begin
    f = 32'h89ab_cdef; g = 32'h89ab_cdef; g2 = 0; r = 0; a = 0; n = 0;
    a[gb.L-1:0] = '1 && f[0];
    #2 $display("a=%h", a);
    $finish;
  end
endmodule
"#,
            Want::Prints("a=00000001"),
        ),
        // rc17_ne_zero_fill: oracles V:a=00000001; PRE V:a=00000001
        (
            "rc17_ne_zero_fill",
            r#"module top;
  if (1) begin : gb localparam int L = 5; end
  logic [31:0] f, r, a;
  logic [0:31] g, g2;
  integer n;
  initial begin
    f = 32'h89ab_cdef; g = 32'h89ab_cdef; g2 = 0; r = 0; a = 0; n = 0;
    a[gb.L-1:0] = (f != '0);
    #2 $display("a=%h", a);
    $finish;
  end
endmodule
"#,
            Want::Prints("a=00000001"),
        ),
        // rc23_ternary_zero_fill8: oracles V:a=ffffff00; PRE V:a=ffffff00
        (
            "rc23_ternary_zero_fill8",
            r#"module top;
  if (1) begin : gb localparam int L = 8; end
  logic [31:0] f, r, a;
  logic [0:31] g, g2;
  integer n;
  initial begin
    f = 32'h89ab_cdef; g = 32'h89ab_cdef; g2 = 0; r = 0; a = 0; n = 0;
    a = 32'hffff_ffff; a[gb.L-1:0] = n ? 5'd3 : '0;
    #2 $display("a=%h", a);
    $finish;
  end
endmodule
"#,
            Want::Prints("a=ffffff00"),
        ),
        // rf02_and_fill: oracles V:a=0000000f r=00000000; PRE V:r=00000000 a=0000000f
        (
            "rf02_and_fill",
            r#"module top;
  if (1) begin : gb localparam int L = 5; end
  logic [31:0] f, r, a;
  integer n;
  initial begin
    f = 32'h89ab_cdef; r = 0; a = 0; n = 0;
    a[gb.L-1:0] = f[3:0] & '1;
    #2 $display("r=%h a=%h", r, a);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=00000000 a=0000000f"),
        ),
        // rf04_lt_fill: oracles V:a=00000000 r=00000000; PRE V:r=00000000 a=00000000
        (
            "rf04_lt_fill",
            r#"module top;
  if (1) begin : gb localparam int L = 5; end
  logic [31:0] f, r, a;
  integer n;
  initial begin
    f = 32'h89ab_cdef; r = 0; a = 0; n = 0;
    a[gb.L-1:0] = (f[3:0] < '1);
    #2 $display("r=%h a=%h", r, a);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=00000000 a=00000000"),
        ),
        // fr01_cmp_fill: oracles V:a=00000001; PRE V:a=00000001
        (
            "fr01_cmp_fill",
            r#"module top;
  logic [31:0] f, a;
  if (1) begin : gb localparam int L = 5; end
  initial begin
    f = 32'h89ab_cdef; a = 0;
    a[gb.L-1:0] = (f[3:0] == '1);
    #1 $display("a=%h", a);
    $finish;
  end
endmodule
"#,
            Want::Prints("a=00000001"),
        ),
        // fr02_redand_fill: oracles V:a=00000001; PRE V:a=00000001
        (
            "fr02_redand_fill",
            r#"module top;
  logic [31:0] f, a;
  if (1) begin : gb localparam int L = 5; end
  initial begin
    f = 32'h89ab_cdef; a = 0;
    a[gb.L-1:0] = &'1;
    #1 $display("a=%h", a);
    $finish;
  end
endmodule
"#,
            Want::Prints("a=00000001"),
        ),
        // fr03_neg_fill0: oracles V:a=ffffffe0; PRE V:a=ffffffe0
        (
            "fr03_neg_fill0",
            r#"module top;
  logic [31:0] f, a;
  if (1) begin : gb localparam int L = 5; end
  initial begin
    f = 32'h89ab_cdef; a = 0;
    a = '1; a[gb.L-1:0] = -'0;
    #1 $display("a=%h", a);
    $finish;
  end
endmodule
"#,
            Want::Prints("a=ffffffe0"),
        ),
        // fr04_not_fill0: oracles V:a=0000001f; PRE V:a=0000001f
        (
            "fr04_not_fill0",
            r#"module top;
  logic [31:0] f, a;
  if (1) begin : gb localparam int L = 5; end
  initial begin
    f = 32'h89ab_cdef; a = 0;
    a[gb.L-1:0] = ~'0;
    #1 $display("a=%h", a);
    $finish;
  end
endmodule
"#,
            Want::Prints("a=0000001f"),
        ),
        // fr07_logical_fill: oracles V:a=00000001; PRE V:a=00000001
        (
            "fr07_logical_fill",
            r#"module top;
  logic [31:0] f, a;
  if (1) begin : gb localparam int L = 5; end
  initial begin
    f = 32'h89ab_cdef; a = 0;
    a[gb.L-1:0] = !'0;
    #1 $display("a=%h", a);
    $finish;
  end
endmodule
"#,
            Want::Prints("a=00000001"),
        ),
        // b16_neg_fill0: oracles V:r=ffffffe0; PRE V:r=ffffffe0
        (
            "b16_neg_fill0",
            r#"module sub; parameter W = 5; endmodule
module top;
  logic [31:0] f, r, a, b32;
  logic [3:0] b4;
  integer i, n;
  sub u();
  if (1) begin : gb localparam int L = 5; end
  initial begin
    f = 32'hffff_ffff; r = 0; a = 0; n = 0; b4 = 0; b32 = 0;
    a = '1; a[u.W-1:0] = -'0;
    #1 $display("r=%h", a);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=ffffffe0"),
        ),
        // b17_not_fill0: oracles V:r=0000001f; PRE V:r=0000001f
        (
            "b17_not_fill0",
            r#"module sub; parameter W = 5; endmodule
module top;
  logic [31:0] f, r, a, b32;
  logic [3:0] b4;
  integer i, n;
  sub u();
  if (1) begin : gb localparam int L = 5; end
  initial begin
    f = 32'hffff_ffff; r = 0; a = 0; n = 0; b4 = 0; b32 = 0;
    a[u.W-1:0] = ~'0;
    #1 $display("r=%h", a);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=0000001f"),
        ),
        // t7_xfill_cmp_late_target: oracles s2v a=00000001; vl a=00000001
        (
            "t7_xfill_cmp_late_target",
            r#"module top;
  if (1) begin : gb localparam int L = 5; end
  logic [31:0] a;
  logic [3:0] f;
  initial begin
    a = 0; f = 4'b1010;
    a[gb.L-1:0] = (f !== 'x);
    #1 $display("a=%h", a);
    $finish;
  end
endmodule
"#,
            Want::Prints("a=00000001"),
        ),
    ]);
}

/// An all-ones or all-zeros fill whose context is a late target is spelled `~1'b0` / a
/// one-bit zero, which the run sizes at the final context (F2) — through every
/// context-passing operator, in every assignment kind; a call argument starts a new region
/// (`g('1)` keeps its 4-bit formal).
#[test]
fn fills_into_a_late_target() {
    check(&[
        // e01_genblk_param_lvalue_fill: oracles V:a=0000001f; PRE V:a=00000001
        (
            "e01_genblk_param_lvalue_fill",
            r#"module top;
  logic [31:0] f, r, a;
  if (1) begin : gb localparam int L = 5; end
  initial begin
    f = 32'h89ab_cdef; r = 0; a = 0;
    a[gb.L-1:0] = '1;
    #1 $display("a=%h", a);
    $finish;
  end
endmodule
"#,
            Want::Prints("a=0000001f"),
        ),
        // b04_fill_plus_3bit: oracles V:r=0000001f; PRE V:r=00000007
        (
            "b04_fill_plus_3bit",
            r#"module sub; parameter W = 5; endmodule
module top;
  logic [31:0] f, r, a, b32;
  logic [3:0] b4;
  integer i, n;
  sub u();
  if (1) begin : gb localparam int L = 5; end
  initial begin
    f = 32'hffff_ffff; r = 0; a = 0; n = 0; b4 = 0; b32 = 0;
    a[u.W-1:0] = '1 + 3'd0;
    #1 $display("r=%h", a);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=0000001f"),
        ),
        // b09_concat_fill: oracles V:b4=f r=0000001f; PRE V:r=00000001 b4=f
        (
            "b09_concat_fill",
            r#"module sub; parameter W = 5; endmodule
module top;
  logic [31:0] f, r, a, b32;
  logic [3:0] b4;
  integer i, n;
  sub u();
  if (1) begin : gb localparam int L = 5; end
  initial begin
    f = 32'hffff_ffff; r = 0; a = 0; n = 0; b4 = 0; b32 = 0;
    {a[u.W-1:0], b4} = '1;
    #1 $display("r=%h b4=%h", a, b4);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=0000001f b4=f"),
        ),
        // b14_ternary_fills: oracles V:r=0000001f; PRE V:r=00000001
        (
            "b14_ternary_fills",
            r#"module sub; parameter W = 5; endmodule
module top;
  logic [31:0] f, r, a, b32;
  logic [3:0] b4;
  integer i, n;
  sub u();
  if (1) begin : gb localparam int L = 5; end
  initial begin
    f = 32'hffff_ffff; r = 0; a = 0; n = 0; b4 = 0; b32 = 0;
    a[u.W-1:0] = n ? '0 : '1;
    #1 $display("r=%h", a);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=0000001f"),
        ),
        // b26_fill_shift: oracles V:r=0000001e; PRE V:r=00000002
        (
            "b26_fill_shift",
            r#"module sub; parameter W = 5; endmodule
module top;
  logic [31:0] f, r, a, b32;
  logic [3:0] b4;
  integer i, n;
  sub u();
  if (1) begin : gb localparam int L = 5; end
  initial begin
    f = 32'hffff_ffff; r = 0; a = 0; n = 0; b4 = 0; b32 = 0;
    a[u.W-1:0] = '1 << 1;
    #1 $display("r=%h", a);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=0000001e"),
        ),
        // rf03_xor_fill: oracles V:a=00000010 r=00000000; PRE V:r=00000000 a=00000000
        (
            "rf03_xor_fill",
            r#"module top;
  if (1) begin : gb localparam int L = 5; end
  logic [31:0] f, r, a;
  integer n;
  initial begin
    f = 32'h89ab_cdef; r = 0; a = 0; n = 0;
    a[gb.L-1:0] = '1 ^ f[3:0];
    #2 $display("r=%h a=%h", r, a);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=00000000 a=00000010"),
        ),
        // rc02_ternary_fill_wider8: oracles V:a=000000ff; PRE V:a=0000001f
        (
            "rc02_ternary_fill_wider8",
            r#"module top;
  if (1) begin : gb localparam int L = 8; end
  logic [31:0] f, r, a;
  logic [0:31] g, g2;
  integer n;
  initial begin
    f = 32'h89ab_cdef; g = 32'h89ab_cdef; g2 = 0; r = 0; a = 0; n = 0;
    a[gb.L-1:0] = n ? 5'd0 : '1;
    #2 $display("a=%h", a);
    $finish;
  end
endmodule
"#,
            Want::Prints("a=000000ff"),
        ),
        // rc22_fill_plus_sized: oracles V:a=00000000; PRE V:a=00000020
        (
            "rc22_fill_plus_sized",
            r#"module top;
  if (1) begin : gb localparam int L = 8; end
  logic [31:0] f, r, a;
  logic [0:31] g, g2;
  integer n;
  initial begin
    f = 32'h89ab_cdef; g = 32'h89ab_cdef; g2 = 0; r = 0; a = 0; n = 0;
    a[gb.L-1:0] = '1 + 5'd1;
    #2 $display("a=%h", a);
    $finish;
  end
endmodule
"#,
            Want::Prints("a=00000000"),
        ),
        // x01_fill_shift: oracles V:a=0000003f; PRE V:a=0000007f
        (
            "x01_fill_shift",
            r#"module subx; logic [6:0] X; endmodule
module top;
  logic [31:0] f, r, a;
  integer n;
  subx u();
  logic [6:0] XL;
  initial begin
    f = 32'hffff_ffff; r = 0; a = 0; n = 0;
    a[$bits(u.X)-1:0] = '1 >> 1;
    #1 $display("a=%h", a);
    $finish;
  end
endmodule
"#,
            Want::Prints("a=0000003f"),
        ),
        // x11_fill_wider: oracles V:a=0000ffffffffffff; PRE V:a=00000000ffffffff
        (
            "x11_fill_wider",
            r#"module suby; logic [47:0] Y; endmodule
module top;
  logic [63:0] f64, a64;
  logic [47:0] YL;
  suby u();
  initial begin
    f64 = 64'hffff_ffff_ffff_ffff; a64 = 0;
    a64[$bits(u.Y)-1:0] = '1;
    #1 $display("a=%h", a64);
    $finish;
  end
endmodule
"#,
            Want::Prints("a=0000ffffffffffff"),
        ),
        // d106_hier_param_ca_lhs: oracles V:w=0000001f; PRE V:w=00000001
        (
            "d106_hier_param_ca_lhs",
            r#"module sub; parameter W = 5; logic [31:0] f = 32'h89ab_cdef; logic [31:0] g = 0; endmodule
module top;
  logic [31:0] f, r, a;
  sub u();
  wire [31:0] w;
  assign w[u.W-1:0] = '1;
  assign w[31:u.W] = '0;
  initial begin
    f = 32'h89ab_cdef; r = 0; a = 0;

    #1 $display("w=%h", w);
    $finish;
  end
endmodule
"#,
            Want::Prints("w=0000001f"),
        ),
        // b20_ff_fill: oracles V:r=0000001f; PRE V:r=00000001
        (
            "b20_ff_fill",
            r#"module sub; parameter W = 5; endmodule
module top;
  logic [31:0] f, r, a, b32;
  logic [3:0] b4;
  integer i, n;
  sub u();
  if (1) begin : gb localparam int L = 5; end
  logic clk = 0;
  logic [31:0] q = 0;
  always_ff @(posedge clk) q[u.W-1:0] <= '1;
  initial begin
    f = 32'hffff_ffff; r = 0; a = 0; n = 0; b4 = 0; b32 = 0;
    #1 clk = 1;
    #1 $display("r=%h", q);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=0000001f"),
        ),
        // b21_comb_fill: oracles V:r=0000001f; PRE V:r=00000001
        (
            "b21_comb_fill",
            r#"module sub; parameter W = 5; endmodule
module top;
  logic [31:0] f, r, a, b32;
  logic [3:0] b4;
  integer i, n;
  sub u();
  if (1) begin : gb localparam int L = 5; end
  logic [31:0] c;
  always_comb begin c = 0; c[u.W-1:0] = '1; end
  initial begin
    f = 32'hffff_ffff; r = 0; a = 0; n = 0; b4 = 0; b32 = 0;

    #1 $display("r=%h", c);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=0000001f"),
        ),
        // b23_fn_fill: oracles V:r=0000001f; PRE V:r=00000001
        (
            "b23_fn_fill",
            r#"module sub; parameter W = 5; endmodule
module top;
  logic [31:0] f, r, a, b32;
  logic [3:0] b4;
  integer i, n;
  sub u();
  if (1) begin : gb localparam int L = 5; end
  function automatic logic [31:0] gf(); logic [31:0] t; t = 0; t[u.W-1:0] = '1; return t; endfunction
  initial begin
    f = 32'hffff_ffff; r = 0; a = 0; n = 0; b4 = 0; b32 = 0;
    r = gf();
    #1 $display("r=%h", r);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=0000001f"),
        ),
        // b24_idx_fill: oracles V:r=0000001f; PRE V:r=00000001
        (
            "b24_idx_fill",
            r#"module sub; parameter W = 5; endmodule
module top;
  logic [31:0] f, r, a, b32;
  logic [3:0] b4;
  integer i, n;
  sub u();
  if (1) begin : gb localparam int L = 5; end
  initial begin
    f = 32'hffff_ffff; r = 0; a = 0; n = 0; b4 = 0; b32 = 0;
    a[0 +: u.W] = '1;
    #1 $display("r=%h", a);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=0000001f"),
        ),
        // e07_hier_param_nba_fill: oracles V:a=0000001f; PRE V:a=00000001
        (
            "e07_hier_param_nba_fill",
            r#"module sub; parameter W = 5; logic [31:0] f = 32'h89ab_cdef; logic [31:0] g = 0; endmodule
module top;
  logic [31:0] f, r, a;
  sub u();
  initial begin
    f = 32'h89ab_cdef; r = 0; a = 0;
    #1 a[u.W-1:0] <= '1;
    #1 $display("a=%h", a);
    $finish;
  end
endmodule
"#,
            Want::Prints("a=0000001f"),
        ),
        // t2_inline_formal_leak: oracles s2v a=0000001e; vl a=0000001e
        (
            "t2_inline_formal_leak",
            r#"module top;
  if (1) begin : gb localparam int L = 5; end
  logic [31:0] a;
  function automatic logic [4:0] g(input logic [3:0] x); return {x, 1'b0}; endfunction
  initial begin
    a = 0;
    a[gb.L-1:0] = g('1);
    #1 $display("a=%h", a);
    $finish;
  end
endmodule
"#,
            Want::Prints("a=0000001e"),
        ),
        // q3d_fill_ctx_late: oracles sv2v c01 000000000000001f 000000000000001f; ver c01 000000000000001f 000000000000001f; PRE warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in
        (
            "q3d_fill_ctx_late",
            r#"module top;
  if (1) begin : gb localparam int W5 = 5; localparam int W8 = 8; localparam int W32 = 32; localparam int W40 = 40; end
  logic [31:0] f = 32'h89ab_cdef;
  logic signed [4:0] s = 5'sb10000;
  logic signed [7:0] s8 = -8'sd3;
  logic signed [3:0] s4 = -4'sd2;
  logic n = 1'b0;
  logic [63:0] c01a = 64'd0, c01b = 64'd0;
  logic [63:0] c02a = 64'd0, c02b = 64'd0;
  logic [63:0] c03a = 64'd0, c03b = 64'd0;
  logic [63:0] c04a = 64'd0, c04b = 64'd0;
  logic [63:0] c05a = 64'd0, c05b = 64'd0;
  logic [63:0] c06a = 64'd0, c06b = 64'd0;
  logic [63:0] c07a = 64'd0, c07b = 64'd0;
  logic [63:0] c08a = 64'd0, c08b = 64'd0;
  logic [63:0] c09a = 64'd0, c09b = 64'd0;
  logic [63:0] c10a = 64'd0, c10b = 64'd0;
  logic [63:0] c11a = 64'd0, c11b = 64'd0;
  logic [63:0] c12a = 64'd0, c12b = 64'd0;
  logic [63:0] c13a = 64'd0, c13b = 64'd0;
  logic [63:0] c14a = 64'd0, c14b = 64'd0;
  logic [63:0] c15a = 64'd0, c15b = 64'd0;
  logic [63:0] c16a = 64'd0, c16b = 64'd0;
  logic [63:0] c17a = 64'd0, c17b = 64'd0;
  logic [63:0] c18a = 64'd0, c18b = 64'd0;
  logic [63:0] c19a = 64'd0, c19b = 64'd0;
  logic [63:0] c20a = 64'd0, c20b = 64'd0;
  logic [63:0] c21a = 64'd0, c21b = 64'd0;
  logic [63:0] c22a = 64'd0, c22b = 64'd0;
  logic [63:0] c23a = 64'd0, c23b = 64'd0;
  logic [63:0] c24a = 64'd0, c24b = 64'd0;
  logic [63:0] c25a = 64'd0, c25b = 64'd0;
  logic [63:0] c26a = 64'd0, c26b = 64'd0;
  logic [63:0] c27a = 64'd0, c27b = 64'd0;
  logic [63:0] c28a = 64'd0, c28b = 64'd0;
  logic [63:0] c29a = 64'd0, c29b = 64'd0;
  logic [63:0] c30a = 64'd0, c30b = 64'd0;
  logic [63:0] c31a = 64'd0, c31b = 64'd0;
  logic [63:0] c32a = 64'd0, c32b = 64'd0;
  initial begin
    #1;
    c01a[gb.W5-1:0] = '1; c01b[gb.W5-1:0] = (~1'b0);
    c02a[gb.W5-1:0] = '1 >> 1; c02b[gb.W5-1:0] = (~1'b0) >> 1;
    c03a[gb.W5-1:0] = '1 >>> 1; c03b[gb.W5-1:0] = (~1'b0) >>> 1;
    c04a[gb.W5-1:0] = '1 + s; c04b[gb.W5-1:0] = (~1'b0) + s;
    c05a[gb.W5-1:0] = -('1); c05b[gb.W5-1:0] = -((~1'b0));
    c06a[gb.W5-1:0] = ~('1); c06b[gb.W5-1:0] = ~((~1'b0));
    c07a[gb.W5-1:0] = '1 ** 2; c07b[gb.W5-1:0] = (~1'b0) ** 2;
    c08a[gb.W5-1:0] = '1 * 3; c08b[gb.W5-1:0] = (~1'b0) * 3;
    c09a[gb.W5-1:0] = '1 / 2; c09b[gb.W5-1:0] = (~1'b0) / 2;
    c10a[gb.W5-1:0] = '1 % 7; c10b[gb.W5-1:0] = (~1'b0) % 7;
    c11a[gb.W5-1:0] = '1 - 1; c11b[gb.W5-1:0] = (~1'b0) - 1;
    c12a[gb.W5-1:0] = '1 ^ f[3:0]; c12b[gb.W5-1:0] = (~1'b0) ^ f[3:0];
    c13a[gb.W5-1:0] = n ? '1 : 5'd3; c13b[gb.W5-1:0] = n ? (~1'b0) : 5'd3;
    c14a[gb.W8-1:0] = n ? 4'sd3 : '1; c14b[gb.W8-1:0] = n ? 4'sd3 : (~1'b0);
    c15a[gb.W5-1:0] = ('1); c15b[gb.W5-1:0] = ((~1'b0));
    c16a[gb.W5-1:0] = '1 & 8'hF0; c16b[gb.W5-1:0] = (~1'b0) & 8'hF0;
    c17a[gb.W5-1:0] = '1 + 0.5; c17b[gb.W5-1:0] = (~1'b0) + 0.5;
    c18a[gb.W40-1:0] = $signed('1); c18b[gb.W40-1:0] = $signed((~1'b0));
    c19a[gb.W40-1:0] = $unsigned('1); c19b[gb.W40-1:0] = $unsigned((~1'b0));
    c20a[gb.W5-1:0] = '1 << 1; c20b[gb.W5-1:0] = (~1'b0) << 1;
    c21a[gb.W5-1:0] = ('1 == 5'h1f); c21b[gb.W5-1:0] = ((~1'b0) == 5'h1f);
    c22a[gb.W40-1:0] = '1; c22b[gb.W40-1:0] = (~1'b0);
    c23a[gb.W40-1:0] = '1 + 33'd0; c23b[gb.W40-1:0] = (~1'b0) + 33'd0;
    c24a[gb.W40-1:0] = '1 <<< 2; c24b[gb.W40-1:0] = (~1'b0) <<< 2;
    c25a[gb.W5-1:0] = !('1); c25b[gb.W5-1:0] = !((~1'b0));
    c26a[gb.W5-1:0] = &('1); c26b[gb.W5-1:0] = &((~1'b0));
    c27a[gb.W32-1:0] = '1 + s8; c27b[gb.W32-1:0] = (~1'b0) + s8;
    c28a[gb.W8-1:0] = s4 + '1; c28b[gb.W8-1:0] = s4 + (~1'b0);
    c29a[gb.W5-1:0] = '1 >> f[1:0]; c29b[gb.W5-1:0] = (~1'b0) >> f[1:0];
    c30a[gb.W5-1:0] = 2'd1 << '1; c30b[gb.W5-1:0] = 2'd1 << (~1'b0);
    c31a[gb.W5-1:0] = '1 ? 5'd7 : 5'd9; c31b[gb.W5-1:0] = (~1'b0) ? 5'd7 : 5'd9;
    c32a[gb.W40-1:0] = {'1}; c32b[gb.W40-1:0] = {(~1'b0)};
    $display("c01 %h %h", c01a, c01b);
    $display("c02 %h %h", c02a, c02b);
    $display("c03 %h %h", c03a, c03b);
    $display("c04 %h %h", c04a, c04b);
    $display("c05 %h %h", c05a, c05b);
    $display("c06 %h %h", c06a, c06b);
    $display("c07 %h %h", c07a, c07b);
    $display("c08 %h %h", c08a, c08b);
    $display("c09 %h %h", c09a, c09b);
    $display("c10 %h %h", c10a, c10b);
    $display("c11 %h %h", c11a, c11b);
    $display("c12 %h %h", c12a, c12b);
    $display("c13 %h %h", c13a, c13b);
    $display("c14 %h %h", c14a, c14b);
    $display("c15 %h %h", c15a, c15b);
    $display("c16 %h %h", c16a, c16b);
    $display("c17 %h %h", c17a, c17b);
    $display("c18 %h %h", c18a, c18b);
    $display("c19 %h %h", c19a, c19b);
    $display("c20 %h %h", c20a, c20b);
    $display("c21 %h %h", c21a, c21b);
    $display("c22 %h %h", c22a, c22b);
    $display("c23 %h %h", c23a, c23b);
    $display("c24 %h %h", c24a, c24b);
    $display("c25 %h %h", c25a, c25b);
    $display("c26 %h %h", c26a, c26b);
    $display("c27 %h %h", c27a, c27b);
    $display("c28 %h %h", c28a, c28b);
    $display("c29 %h %h", c29a, c29b);
    $display("c30 %h %h", c30a, c30b);
    $display("c31 %h %h", c31a, c31b);
    $display("c32 %h %h", c32a, c32b);
  end
endmodule
"#,
            Want::Prints("c01 000000000000001f 000000000000001f\nc02 000000000000000f 000000000000000f\nc03 000000000000000f 000000000000000f\nc04 000000000000000f 000000000000000f\nc05 0000000000000001 0000000000000001\nc06 0000000000000000 0000000000000000\nc07 0000000000000001 0000000000000001\nc08 000000000000001d 000000000000001d\nc09 000000000000001f 000000000000001f\nc10 0000000000000003 0000000000000003\nc11 000000000000001e 000000000000001e\nc12 0000000000000010 0000000000000010\nc13 0000000000000003 0000000000000003\nc14 00000000000000ff 00000000000000ff\nc15 000000000000001f 000000000000001f\nc16 0000000000000010 0000000000000010\nc17 0000000000000002 0000000000000002\nc18 000000ffffffffff 000000ffffffffff\nc19 0000000000000001 0000000000000001\nc20 000000000000001e 000000000000001e\nc21 0000000000000001 0000000000000001\nc22 000000ffffffffff 000000ffffffffff\nc23 000000ffffffffff 000000ffffffffff\nc24 000000fffffffffc 000000fffffffffc\nc25 0000000000000000 0000000000000000\nc26 0000000000000001 0000000000000001\nc27 00000000000000fc 00000000000000fc\nc28 000000000000000d 000000000000000d\nc29 0000000000000003 0000000000000003\nc30 0000000000000002 0000000000000002\nc31 0000000000000007 0000000000000007\nc32 0000000000000001 0000000000000001"),
        ),
        // f01_mintypmax: oracles sv2v a=0000001f; ver a=0000001f; PRE a=00000001
        (
            "f01_mintypmax",
            r#"module top;
  logic [31:0] a, f;
  logic [3:0] b4;
  logic signed [3:0] s4;
  real rr;
  integer n;
  if (1) begin : gb
    localparam int L = 5;
  end
  function automatic logic [7:0] id8(input logic [7:0] x); return x; endfunction
  initial begin
    f = 32'h89ab_cdef; b4 = 4'h3; s4 = -4'sd2; rr = 2.0; n = 0;
    a = 32'h0;
    a[gb.L-1:0] = (1:'1:0);
    #1 $display("a=%h", a);
    $finish;
  end
endmodule
"#,
            Want::Prints("a=0000001f"),
        ),
    ]);
}

/// A fill beside an operand whose width is decided only after the deferred passes is
/// spelled for the run to size, not sized at 32 bits (RC2) — a `$bits(u.X)` placeholder
/// width too.
#[test]
fn fills_beside_a_late_operand() {
    check(&[
        // g4x1_cmp_fill_late_sib_late_tgt: oracles iv IV_BUILD_FAILED; sv2v a=00000001; ver a=00000001; PRE warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in
        (
            "g4x1_cmp_fill_late_sib_late_tgt",
            r#"module top;
  if (1) begin : gb localparam int L = 5; end
  logic [31:0] f = 32'h89ab_cdff, r = 32'd0, a = 32'd0;
  logic [4:0] b = 5'd0;
  initial begin #1 a[gb.L-1:0] = (f[gb.L-1:0] == '1);
    $display("a=%h", a);
  end
endmodule
"#,
            Want::Prints("a=00000001"),
        ),
        // g4x1b_cmp_fill_late_sib_fixed_tgt: oracles iv IV_BUILD_FAILED; sv2v r=00000001; ver r=00000001; PRE warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in
        (
            "g4x1b_cmp_fill_late_sib_fixed_tgt",
            r#"module top;
  if (1) begin : gb localparam int L = 5; end
  logic [31:0] f = 32'h89ab_cdff, r = 32'd0, a = 32'd0;
  logic [4:0] b = 5'd0;
  initial begin #1 r = (f[gb.L-1:0] == '1);
    $display("r=%h", r);
  end
endmodule
"#,
            Want::Prints("r=00000001"),
        ),
        // g4x2_ne_fill_late_sib_late_tgt: oracles iv IV_BUILD_FAILED; sv2v a=00000009; ver a=00000009; PRE warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in
        (
            "g4x2_ne_fill_late_sib_late_tgt",
            r#"module top;
  if (1) begin : gb localparam int L = 5; end
  logic [31:0] f = 32'h89ab_cdff, r = 32'd0, a = 32'd0;
  logic [4:0] b = 5'd0;
  initial begin #1 a[gb.L-1:0] = (f[gb.L-1:0] != '1) ? 5'd3 : 5'd9;
    $display("a=%h", a);
  end
endmodule
"#,
            Want::Prints("a=00000009"),
        ),
        // o03_eq_fill: oracles V:r=00000001; PRE V:r=00000000
        (
            "o03_eq_fill",
            r#"module sub; parameter W = 5; endmodule
module top;
  logic [31:0] f, r, a, b32;
  logic [3:0] b4;
  integer i, n;
  sub u();
  if (1) begin : gb localparam int L = 5; end
  initial begin
    f = 32'hffff_ffff; r = 0; a = 0; n = 0; b4 = 0; b32 = 0;
    r = (f[u.W-1:0] == '1);
    #1 $display("r=%h", r);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=00000001"),
        ),
        // g07_ternary_fill_ctx: oracles V:r=0000001f; PRE V:r=ffffffff
        (
            "g07_ternary_fill_ctx",
            r#"module top;
  logic [31:0] f, r;
  integer n;
  if (1) begin : gb localparam int L = 5; end
  initial begin
    f = 32'hffff_ffff; r = 0; n = 0;
    r = {n ? f[gb.L-1:0] : '1};
    #1 $display("r=%h", r);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=0000001f"),
        ),
        // o01_case_fill: oracles V:r=00000001; PRE V:r=00000002
        (
            "o01_case_fill",
            r#"module sub; parameter W = 5; endmodule
module top;
  logic [31:0] f, r, a, b32;
  logic [3:0] b4;
  integer i, n;
  sub u();
  if (1) begin : gb localparam int L = 5; end
  initial begin
    f = 32'hffff_ffff; r = 0; a = 0; n = 0; b4 = 0; b32 = 0;
    case (f[u.W-1:0]) '1: r = 1; default: r = 2; endcase
    #1 $display("r=%h", r);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=00000001"),
        ),
        // x04_operand_fill: oracles V:r=00000000; PRE V:r=ffffff80
        (
            "x04_operand_fill",
            r#"module subx; logic [6:0] X; endmodule
module top;
  logic [31:0] f, r, a;
  integer n;
  subx u();
  logic [6:0] XL;
  initial begin
    f = 32'hffff_ffff; r = 0; a = 0; n = 0;
    r = {f[$bits(u.X)-1:0] ^ '1};
    #1 $display("r=%h", r);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=00000000"),
        ),
        // x06_rep_operand_fill: oracles V:r=00000000; PRE V:r=ffffff80
        (
            "x06_rep_operand_fill",
            r#"module subx; logic [6:0] X; endmodule
module top;
  logic [31:0] f, r, a;
  integer n;
  subx u();
  logic [6:0] XL;
  initial begin
    f = 32'hffff_ffff; r = 0; a = 0; n = 0;
    r = {{($bits(u.X)){1'b1}} ^ '1};
    #1 $display("r=%h", r);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=00000000"),
        ),
        // x07_eq_fill: oracles V:r=00000001; PRE V:r=00000000
        (
            "x07_eq_fill",
            r#"module subx; logic [6:0] X; endmodule
module top;
  logic [31:0] f, r, a;
  integer n;
  subx u();
  logic [6:0] XL;
  initial begin
    f = 32'hffff_ffff; r = 0; a = 0; n = 0;
    r = (f[$bits(u.X)-1:0] == '1);
    #1 $display("r=%h", r);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=00000001"),
        ),
    ]);
}

/// An intra-assignment capture into a late target holds the right-hand side at its own
/// width; that is exact under the write's zero-extension unless its top region is signed,
/// holds a fill other than `'0`, or has an operator whose value depends on its width (RC3).
#[test]
fn captures_into_a_late_target() {
    check(&[
        // e26_genblk_intra_delay_capture: oracles V:a=0000000f; PRE V:a=00000001
        (
            "e26_genblk_intra_delay_capture",
            r#"module top;
  logic [31:0] f, r, a;
  logic clk = 0;
  if (1) begin : gb localparam int L = 5; end
  initial begin
    f = 32'h89ab_cdef; r = 0; a = 0;
    a[gb.L-1:0] = #1 f;
    #5 $display("a=%h", a);
    $finish;
  end
endmodule
"#,
            Want::Prints("a=0000000f"),
        ),
        // e30_genblk_intra_event: oracles V:a=0000000f; PRE V:a=00000001
        (
            "e30_genblk_intra_event",
            r#"module top;
  logic [31:0] f, r, a;
  logic clk = 0;
  if (1) begin : gb localparam int L = 5; end
  initial begin
    f = 32'h89ab_cdef; r = 0; a = 0;
    fork #1 clk = 1; join_none
    a[gb.L-1:0] = @(posedge clk) f;
    #5 $display("a=%h", a);
    $finish;
  end
endmodule
"#,
            Want::Prints("a=0000000f"),
        ),
        // rc25_sized_capture_delay: oracles V:a=0000001f; PRE V:a=00000001
        (
            "rc25_sized_capture_delay",
            r#"module top;
  if (1) begin : gb localparam int L = 5; end
  logic [31:0] f, r, a;
  logic [0:31] g, g2;
  integer n;
  initial begin
    f = 32'h89ab_cdef; g = 32'h89ab_cdef; g2 = 0; r = 0; a = 0; n = 0;
    a[gb.L-1:0] = #1 32'hffff_ffff;
    #2 $display("a=%h", a);
    $finish;
  end
endmodule
"#,
            Want::Prints("a=0000001f"),
        ),
        // b06_ia_val: oracles V:r=0000001f; PRE V:r=00000001
        (
            "b06_ia_val",
            r#"module sub; parameter W = 5; endmodule
module top;
  logic [31:0] f, r, a, b32;
  logic [3:0] b4;
  integer i, n;
  sub u();
  if (1) begin : gb localparam int L = 5; end
  initial begin
    f = 32'hffff_ffff; r = 0; a = 0; n = 0; b4 = 0; b32 = 0;
    a[u.W-1:0] = #1 f;
    #1;
    #1 $display("r=%h", a);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=0000001f"),
        ),
        // x12_ia_wider: oracles V:a=0000ffffffffffff; PRE V:a=00000000ffffffff
        (
            "x12_ia_wider",
            r#"module suby; logic [47:0] Y; endmodule
module top;
  logic [63:0] f64, a64;
  logic [47:0] YL;
  suby u();
  initial begin
    f64 = 64'hffff_ffff_ffff_ffff; a64 = 0;
    a64[$bits(u.Y)-1:0] = #1 f64;
    #1;
    #1 $display("a=%h", a64);
    $finish;
  end
endmodule
"#,
            Want::Prints("a=0000ffffffffffff"),
        ),
        // h01_hier_whole_ia: oracles V:g=89abcdef; PRE V:g=00000001
        (
            "h01_hier_whole_ia",
            r#"module sub; logic [31:0] g = 0; endmodule
module top;
  logic [31:0] f;
  logic clk = 0;
  sub u();
  initial #1 clk = 1;
  initial begin
    f = 32'h89ab_cdef;
    u.g = #1 f;
    #3 $display("g=%h", u.g);
    $finish;
  end
endmodule
"#,
            Want::Prints("g=89abcdef"),
        ),
        // h02_hier_part_ia: oracles V:g=0000cdef; PRE V:g=00000001
        (
            "h02_hier_part_ia",
            r#"module sub; logic [31:0] g = 0; endmodule
module top;
  logic [31:0] f;
  logic clk = 0;
  sub u();
  initial #1 clk = 1;
  initial begin
    f = 32'h89ab_cdef;
    u.g[15:0] = #1 f;
    #3 $display("g=%h", u.g);
    $finish;
  end
endmodule
"#,
            Want::Prints("g=0000cdef"),
        ),
        // h03_hier_whole_ev_ia: oracles V:g=89abcdef; PRE V:g=00000001
        (
            "h03_hier_whole_ev_ia",
            r#"module sub; logic [31:0] g = 0; endmodule
module top;
  logic [31:0] f;
  logic clk = 0;
  sub u();
  initial #1 clk = 1;
  initial begin
    f = 32'h89ab_cdef;
    u.g = @(posedge clk) f;
    #3 $display("g=%h", u.g);
    $finish;
  end
endmodule
"#,
            Want::Prints("g=89abcdef"),
        ),
        // l04_hier_bits_ia: oracles V:r=0000007f; PRE V:r=0000007f
        (
            "l04_hier_bits_ia",
            r#"module subx; logic [6:0] X; endmodule
module top;
  logic [31:0] f, r, a;
  integer i, n;
  subx u();
  initial begin
    f = 32'hffff_ffff; r = 0; a = 0; n = 0;
    a[$bits(u.X)-1:0] = #1 f;
    #1;
    #1 $display("r=%h", a);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=0000007f"),
        ),
        // i15_bits_x_intra: oracles V:r=0000007f; PRE V:r=0000007f
        (
            "i15_bits_x_intra",
            r#"module sub; parameter W = 5; logic [6:0] X; endmodule
module top;
  logic [31:0] f, r, a;
  integer i, n;
  sub u();
  initial begin
    f = 32'hffff_ffff; r = 0; a = 0; n = 0;
    a[$bits(u.X)-1:0] = #1 f;
    #1;
    #1 $display("r=%h", a);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=0000007f"),
        ),
        // x08_ia_shift_ctx: oracles V:a=0000007f; PRE V:a=0000007f
        (
            "x08_ia_shift_ctx",
            r#"module subx; logic [6:0] X; endmodule
module top;
  logic [31:0] f, r, a;
  integer n;
  subx u();
  logic [6:0] XL;
  initial begin
    f = 32'hffff_ffff; r = 0; a = 0; n = 0;
    a[$bits(u.X)-1:0] = #1 (f >> 1);
    #1;
    #1 $display("a=%h", a);
    $finish;
  end
endmodule
"#,
            Want::Prints("a=0000007f"),
        ),
        // ia1_cmp_rhs_late: oracles iv IV_BUILD_FAILED; sv2v a=ffffffe1; ver a=ffffffe1; PRE warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in
        (
            "ia1_cmp_rhs_late",
            r#"module sub; parameter W = 5; endmodule
module top;
  if (1) begin : gb localparam int L = 5; end
  sub u();
  logic [31:0] f = 32'h89ab_cdef, a = 32'hffff_ffff;
  logic [3:0] b4 = 4'hf;
  logic signed [3:0] s4 = -4'sd1;
  logic clk = 0;
  initial #1 clk = 1;
  initial begin a[gb.L-1:0] = #1 (f[3:0] == 4'hf);
    #3 $display("a=%h", a);
  end
endmodule
"#,
            Want::Prints("a=ffffffe1"),
        ),
        // ia2_redand_rhs_late: oracles iv IV_BUILD_FAILED; sv2v a=ffffffe1; ver a=ffffffe1; PRE warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in
        (
            "ia2_redand_rhs_late",
            r#"module sub; parameter W = 5; endmodule
module top;
  if (1) begin : gb localparam int L = 5; end
  sub u();
  logic [31:0] f = 32'h89ab_cdef, a = 32'hffff_ffff;
  logic [3:0] b4 = 4'hf;
  logic signed [3:0] s4 = -4'sd1;
  logic clk = 0;
  initial #1 clk = 1;
  initial begin a[gb.L-1:0] = #1 &f[3:0];
    #3 $display("a=%h", a);
  end
endmodule
"#,
            Want::Prints("a=ffffffe1"),
        ),
        // ia3_1bit_lit_late: oracles iv IV_BUILD_FAILED; sv2v a=ffffffe1; ver a=ffffffe1; PRE warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in
        (
            "ia3_1bit_lit_late",
            r#"module sub; parameter W = 5; endmodule
module top;
  if (1) begin : gb localparam int L = 5; end
  sub u();
  logic [31:0] f = 32'h89ab_cdef, a = 32'hffff_ffff;
  logic [3:0] b4 = 4'hf;
  logic signed [3:0] s4 = -4'sd1;
  logic clk = 0;
  initial #1 clk = 1;
  initial begin a[gb.L-1:0] = #1 1'b1;
    #3 $display("a=%h", a);
  end
endmodule
"#,
            Want::Prints("a=ffffffe1"),
        ),
        // ia4_bitsel_event_hier: oracles iv IV_BUILD_FAILED; sv2v IV_BUILD_FAILED; ver a=ffffffe1; PRE warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in
        (
            "ia4_bitsel_event_hier",
            r#"module sub; parameter W = 5; endmodule
module top;
  if (1) begin : gb localparam int L = 5; end
  sub u();
  logic [31:0] f = 32'h89ab_cdef, a = 32'hffff_ffff;
  logic [3:0] b4 = 4'hf;
  logic signed [3:0] s4 = -4'sd1;
  logic clk = 0;
  initial #1 clk = 1;
  initial begin a[u.W-1:0] = @(posedge clk) f[0];
    #3 $display("a=%h", a);
  end
endmodule
"#,
            Want::Prints("a=ffffffe1"),
        ),
        // ia5_4bit_var_late: oracles iv IV_BUILD_FAILED; sv2v a=ffffffef; ver a=ffffffef; PRE warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in
        (
            "ia5_4bit_var_late",
            r#"module sub; parameter W = 5; endmodule
module top;
  if (1) begin : gb localparam int L = 5; end
  sub u();
  logic [31:0] f = 32'h89ab_cdef, a = 32'hffff_ffff;
  logic [3:0] b4 = 4'hf;
  logic signed [3:0] s4 = -4'sd1;
  logic clk = 0;
  initial #1 clk = 1;
  initial begin a[gb.L-1:0] = #1 b4;
    #3 $display("a=%h", a);
  end
endmodule
"#,
            Want::Prints("a=ffffffef"),
        ),
        // ia7_concat_rhs_late: oracles iv IV_BUILD_FAILED; sv2v a=ffffffe5; ver a=ffffffe5; PRE warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in
        (
            "ia7_concat_rhs_late",
            r#"module sub; parameter W = 5; endmodule
module top;
  if (1) begin : gb localparam int L = 5; end
  sub u();
  logic [31:0] f = 32'h89ab_cdef, a = 32'hffff_ffff;
  logic [3:0] b4 = 4'hf;
  logic signed [3:0] s4 = -4'sd1;
  logic clk = 0;
  initial #1 clk = 1;
  initial begin a[gb.L-1:0] = #1 {1'b1, 2'b01};
    #3 $display("a=%h", a);
  end
endmodule
"#,
            Want::Prints("a=ffffffe5"),
        ),
        // w7_concat_tgt_late_chunk_ia: oracles iv IV_BUILD_FAILED; sv2v a=0000001f b=01; ver a=0000001f b=01; PRE warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in
        (
            "w7_concat_tgt_late_chunk_ia",
            r#"module top;
  if (1) begin : gb localparam int L = 5; end
  logic [31:0] f = 32'h89ab_cdff, r = 32'd0, a = 32'd0;
  logic [4:0] b = 5'd0;
  initial begin #1 {a[gb.L-1:0], b} = #1 10'b11111_00001;
    $display("a=%h b=%h", a, b);
  end
endmodule
"#,
            Want::Prints("a=0000001f b=01"),
        ),
        // t6_ia_zero_fill_late: oracles s2v a=ffffffe0; vl a=ffffffe0
        (
            "t6_ia_zero_fill_late",
            r#"module top;
  if (1) begin : gb localparam int L = 5; end
  logic [31:0] a;
  initial begin
    a = 32'hffff_ffff;
    a[gb.L-1:0] = #1 '0;
    #2 $display("a=%h", a);
    $finish;
  end
endmodule
"#,
            Want::Prints("a=ffffffe0"),
        ),
    ]);
}

/// REFUSED. A capture whose value depends on a width it could not know, an `'x` / `'z`
/// fill sized before its target, a stream padded before its target. The oracles' values are
/// in the comments; the decided target is wider than what was built.
#[test]
fn captures_and_fills_a_late_target_cannot_hold() {
    check(&[
        // e27_genblk_intra_delay_ctx: oracles V:a=00000010; PRE V:a=00000000
        (
            "e27_genblk_intra_delay_ctx",
            r#"module top;
  logic [31:0] f, r, a;
  logic clk = 0;
  if (1) begin : gb localparam int L = 5; end
  initial begin
    f = 32'h89ab_cdef; r = 0; a = 0;
    a[gb.L-1:0] = #1 4'hf + 4'h1;
    #5 $display("a=%h", a);
    $finish;
  end
endmodule
"#,
            Want::Refused("the width of this assignment target (5 bits) is known only after elabo"),
        ),
        // b05_ia_fill: oracles V:r=0000001f; PRE V:r=00000001
        (
            "b05_ia_fill",
            r#"module sub; parameter W = 5; endmodule
module top;
  logic [31:0] f, r, a, b32;
  logic [3:0] b4;
  integer i, n;
  sub u();
  if (1) begin : gb localparam int L = 5; end
  initial begin
    f = 32'hffff_ffff; r = 0; a = 0; n = 0; b4 = 0; b32 = 0;
    a[u.W-1:0] = #1 '1;
    #1;
    #1 $display("r=%h", a);
    $finish;
  end
endmodule
"#,
            Want::Refused("the width of this assignment target (5 bits) is known only after elabo"),
        ),
        // ia6_signed4_late: oracles iv IV_BUILD_FAILED; sv2v a=ffffffff; ver a=ffffffff; PRE warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in
        (
            "ia6_signed4_late",
            r#"module sub; parameter W = 5; endmodule
module top;
  if (1) begin : gb localparam int L = 5; end
  sub u();
  logic [31:0] f = 32'h89ab_cdef, a = 32'hffff_ffff;
  logic [3:0] b4 = 4'hf;
  logic signed [3:0] s4 = -4'sd1;
  logic clk = 0;
  initial #1 clk = 1;
  initial begin a[gb.L-1:0] = #1 s4;
    #3 $display("a=%h", a);
  end
endmodule
"#,
            Want::Refused("the width of this assignment target (5 bits) is known only after elabo"),
        ),
        // i18_lval_fill_x: oracles V:r=00000000; PRE V:r=0000000X
        (
            "i18_lval_fill_x",
            r#"module sub; parameter W = 5; logic [6:0] X; endmodule
module top;
  logic [31:0] f, r, a;
  integer i, n;
  sub u();
  initial begin
    f = 32'hffff_ffff; r = 0; a = 0; n = 0;
    a[u.W-1:0] = 'x;
    #1 $display("r=%h", a);
    $finish;
  end
endmodule
"#,
            Want::Refused("the width of this assignment target (5 bits) is known only after elabo"),
        ),
        // j06_hier_fill_x: oracles V:r=00000000000000000000000000000000; PRE V:r=0000000000000000000000000000000x
        (
            "j06_hier_fill_x",
            r#"module sub; parameter W = 5; endmodule
module top;
  logic [31:0] f, r, a;
  integer i, n;
  sub u();
  initial begin
    f = 32'hffff_ffff; r = 0; a = 0; n = 0;
    a[u.W-1:0] = 'x;
    #1 $display("r=%b", a);
    $finish;
  end
endmodule
"#,
            Want::Refused("the width of this assignment target (5 bits) is known only after elabo"),
        ),
        // b15_fill_x: oracles V:r=00000000000000000000000000000000; PRE V:r=0000000000000000000000000000000x
        (
            "b15_fill_x",
            r#"module sub; parameter W = 5; endmodule
module top;
  logic [31:0] f, r, a, b32;
  logic [3:0] b4;
  integer i, n;
  sub u();
  if (1) begin : gb localparam int L = 5; end
  initial begin
    f = 32'hffff_ffff; r = 0; a = 0; n = 0; b4 = 0; b32 = 0;
    a[u.W-1:0] = 'x;
    #1 $display("r=%b", a);
    $finish;
  end
endmodule
"#,
            Want::Refused("the width of this assignment target (5 bits) is known only after elabo"),
        ),
        // rc21_zfill: oracles SPLIT; PRE V:a=0000000000000000000000000000000z
        (
            "rc21_zfill",
            r#"module top;
  if (1) begin : gb localparam int L = 5; end
  logic [31:0] f, r, a;
  logic [0:31] g, g2;
  integer n;
  initial begin
    f = 32'h89ab_cdef; g = 32'h89ab_cdef; g2 = 0; r = 0; a = 0; n = 0;
    a[gb.L-1:0] = 'z;
    #2 $display("a=%b", a);
    $finish;
  end
endmodule
"#,
            Want::Refused("the width of this assignment target (5 bits) is known only after elabo"),
        ),
        // rc06_stream_wider12: oracles V:a=00000f70; PRE V:a=000000f7
        (
            "rc06_stream_wider12",
            r#"module top;
  if (1) begin : gb localparam int L = 12; end
  logic [31:0] f, r, a;
  logic [0:31] g, g2;
  integer n;
  initial begin
    f = 32'h89ab_cdef; g = 32'h89ab_cdef; g2 = 0; r = 0; a = 0; n = 0;
    a[gb.L-1:0] = {<<{f[7:0]}};
    #2 $display("a=%h", a);
    $finish;
  end
endmodule
"#,
            Want::Refused("the width of this assignment target (12 bits) is known only after elab"),
        ),
        // x02_stream_pad: oracles V:a=00000050; PRE V:a=00000000
        (
            "x02_stream_pad",
            r#"module subx; logic [6:0] X; endmodule
module top;
  logic [31:0] f, r, a;
  integer n;
  subx u();
  logic [6:0] XL;
  initial begin
    f = 32'hffff_ffff; r = 0; a = 0; n = 0;
    a[$bits(u.X)-1:0] = {>>{4'hA}};
    #1 $display("a=%h", a);
    $finish;
  end
endmodule
"#,
            Want::Refused("the width of this assignment target (7 bits) is known only after elabo"),
        ),
        // t3_sentinel_ia_sum: oracles iv g=10; s2v g=10; vl g=10
        (
            "t3_sentinel_ia_sum",
            r#"module sub; logic [7:0] g = 8'h00; endmodule
module top;
  sub u();
  initial begin
    u.g = #1 4'hf + 4'h1;
    #1 $display("g=%h", u.g);
    $finish;
  end
endmodule
"#,
            Want::Refused("the width of this hierarchical assignment target (8 bits) is known onl"),
        ),
    ]);
}

/// REFUSED, or the value where it is legal: a negative count or width (a wrapped negative
/// literal too, RC1), bounds out of order, a zero indexed width (IEEE §11.5.1), a zero count
/// outside a concatenation, a width over vita's limit from bounds that are not literals.
#[test]
fn negative_zero_and_over_limit_edges() {
    check(&[
        // rd05_genblk_neg_rep: oracles REFUSE; PRE V:r=ffffffff
        (
            "rd05_genblk_neg_rep",
            r#"module top;
  if (1) begin : gb localparam int N = -1; end
  logic [31:0] f, r, a;
  logic [0:31] g, g2;
  integer n;
  initial begin
    f = 32'h89ab_cdef; g = 32'h89ab_cdef; g2 = 0; r = 0; a = 0; n = 0;
    r = {8'hA5, {(gb.N){1'b1}}};
    #2 $display("r=%h", r);
    $finish;
  end
endmodule
"#,
            Want::Refused("this replication count is negative (IEEE §11.4.12.2)"),
        ),
        // j01_desc_reversed_size: oracles V:r=00000017; PRE V:r=00000001
        (
            "j01_desc_reversed_size",
            r#"module top;
  logic [7:0] arr [6];
  logic [31:0] f, r;
  initial begin
    f = 32'h89ab_cdef;
    r = f[2:$size(arr)];
    #1 $display("r=%h", r);
    $finish;
  end
endmodule
"#,
            Want::Refused("the width of this part-select is negative: its bounds are out of order"),
        ),
        // rb11_asc_reversed_late: oracles V:r=00000004; PRE V:r=000000XX
        (
            "rb11_asc_reversed_late",
            r#"module top;
  if (1) begin : gb localparam int A = 2; localparam int B = 6; localparam int W = 5; end
  logic [31:0] f, r, a;
  logic [0:31] g, g2;
  integer n;
  initial begin
    f = 32'h89ab_cdef; g = 32'h89ab_cdef; g2 = 0; r = 0; a = 0; n = 0;
    r = g[gb.B:gb.A];
    #2 $display("r=%h", r);
    $finish;
  end
endmodule
"#,
            Want::Refused("the width of this part-select is negative: its bounds are out of order"),
        ),
        // n1_idx_neg_lit: oracles iv r=00000001; sv2v r=00000001; ver BUILD_FAILED; PRE warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in
        (
            "n1_idx_neg_lit",
            r#"module top;

  logic [31:0] f = 32'h89ab_cdef, r;
  initial begin #1 r = f[0 +: -1]; $display("r=%h", r); end
endmodule
"#,
            Want::Refused("the width of this indexed part-select is negative"),
        ),
        // n5_idx_neg_sized: oracles iv r=00000def; sv2v r=00000def; ver r=00000def; PRE warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in
        (
            "n5_idx_neg_sized",
            r#"module top;

  logic [31:0] f = 32'h89ab_cdef, r;
  initial begin #1 r = f[0 +: -4'sd2]; $display("r=%h", r); end
endmodule
"#,
            Want::Refused("the width of this indexed part-select is negative"),
        ),
        // n8_idxdown_neg_lit: oracles iv r=00000001; sv2v r=00000001; ver BUILD_FAILED; PRE warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in
        (
            "n8_idxdown_neg_lit",
            r#"module top;
  logic [31:0] f = 32'h89ab_cdef, r;
  initial begin #1 r = f[7 -: -1]; $display("r=%h", r); end
endmodule
"#,
            Want::Refused("the width of this indexed part-select is negative"),
        ),
        // z1_idx_zero_lit: oracles iv IV_BUILD_FAILED; sv2v IV_BUILD_FAILED; PRE warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in
        (
            "z1_idx_zero_lit",
            r#"module top;
  logic [31:0] f = 32'h89ab_cdef, r = 32'h1234_5678;
  initial begin #1 r = f[0 +: 0]; $display("r=%h", r); end
endmodule
"#,
            Want::Refused("the width of this indexed part-select is zero; an indexed part-select"),
        ),
        // z2_idx_zero_late: oracles iv IV_BUILD_FAILED; sv2v IV_BUILD_FAILED; PRE warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in
        (
            "z2_idx_zero_late",
            r#"module top;
  if (1) begin : gb localparam int Z = 0; end
  logic [31:0] f = 32'h89ab_cdef, r = 32'h1234_5678;
  initial begin #1 r = f[0 +: gb.Z]; $display("r=%h", r); end
endmodule
"#,
            Want::Refused("the width of this indexed part-select is zero; an indexed part-select"),
        ),
        // z3_idx_zero_late_arith: oracles iv IV_BUILD_FAILED; sv2v IV_BUILD_FAILED; PRE warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in
        (
            "z3_idx_zero_late_arith",
            r#"module top;
  if (1) begin : gb localparam int A = 3; end
  logic [31:0] f = 32'h89ab_cdef, r = 32'h1234_5678;
  initial begin #1 r = f[0 +: gb.A - 3]; $display("r=%h", r); end
endmodule
"#,
            Want::Refused("the width of this indexed part-select is zero; an indexed part-select"),
        ),
        // t4_late_zero_count: oracles
        (
            "t4_late_zero_count",
            r#"module top;
  if (1) begin : gb localparam int Z = 0; end
  logic [31:0] r;
  initial begin
    r = 32'hffff_ffff;
    r = {(gb.Z){1'b1}};
    #1 $display("r=%h", r);
    $finish;
  end
endmodule
"#,
            Want::Refused("a replication count of zero is only legal as a direct operand of a con"),
        ),
        // t5_late_zero_count_concat: oracles s2v r=000000a5; vl r=000000a5
        (
            "t5_late_zero_count_concat",
            r#"module top;
  if (1) begin : gb localparam int Z = 0; end
  logic [31:0] r;
  initial begin
    r = {8'hA5, {(gb.Z){1'b1}}};
    #1 $display("r=%h", r);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=000000a5"),
        ),
        // k2_big_over_max: oracles V:r=ffffffff; PRE V:r=00000001
        (
            "k2_big_over_max",
            r#"typedef struct packed { logic [63:0] PAD; logic [31:0] BIGM; logic [7:0] HI; logic [7:0] LO; } prm2_t;
module top #(parameter prm2_t p2 = {64'h0, 32'd1048576, 8'd5, 8'd2}, parameter logic [71:0] WP2 = 72'h00_0000_0000_0010_0000,
             parameter logic [71:0] WP3 = 72'h00_0000_0000_000F_FFFF);
  logic [31:0] f, r, a;
  integer i, n;
  localparam int BIG = 1048576;
  initial begin
    f = 32'hffff_ffff; r = 0; a = 0; n = 0;
    r = f[BIG*1:0];
    #1 $display("r=%h", r);
    $finish;
  end
endmodule
"#,
            Want::Refused("the width of this part-select (1048577 bits) exceeds vita's limit of 1"),
        ),
        // d67_huge_bare_bound: oracles V:r=89abcdef; PRE V:r=00000001
        (
            "d67_huge_bare_bound",
            r#"module top #(parameter logic [71:0] WP = {32'h0010_0001, 40'h0});
  logic [31:0] f, r, a;
  initial begin
    f = 32'h89ab_cdef; r = 0; a = 0;
    r = f[WP[71:40]:0];
    #1 $display("r=%h", r);
    $finish;
  end
endmodule
"#,
            Want::Refused("the width of this part-select (1048578 bits) exceeds vita's limit of 1"),
        ),
        // d67b_huge_literal_twin: oracles V:r=89abcdef; PRE V:r=89abcdef
        (
            "d67b_huge_literal_twin",
            r#"module top;
  logic [31:0] f, r;
  initial begin
    f = 32'h89ab_cdef;
    r = f[1048577:0];
    #1 $display("r=%h", r);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=89abcdef"),
        ),
        // x2_bits_hier_count_neg_at_lowering: oracles iv r=000000ef; sv2v IV_BUILD_FAILED; ver r=000000ef; PRE warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in
        (
            "x2_bits_hier_count_neg_at_lowering",
            r#"module subx; logic [47:0] X; endmodule
module top;
  subx u();
  logic [31:0] f = 32'h89ab_cdef, r;
  initial begin #1 r = f[$bits(u.X) - 41 : 0]; $display("r=%h", r); end
endmodule
"#,
            Want::Prints("r=000000ef"),
        ),
    ]);
}

/// REFUSED: an operator outside `+`/`-` over a late name, arithmetic over a >64-bit
/// parameter's bits (VeeR EL2's shape), an x-valued bound.
#[test]
fn edges_with_no_decided_value() {
    check(&[
        // k08_genblk_mul: oracles V:r=000001ef 000003ff 000001ef; PRE V:r=00000001 00000000 00000001
        (
            "k08_genblk_mul",
            r#"module top;
  if (1) begin : gb localparam int L = 5; end
  logic [31:0] f, r1, r2, r3;
  initial begin
    f = 32'h89ab_cdef;
    r1 = f[gb.L*2-1:0];
    r2 = {gb.L*2{1'b1}};
    r3 = f[0 +: gb.L*2];
    #1 $display("r=%h %h %h", r1, r2, r3);
    $finish;
  end
endmodule
"#,
            Want::Refused("the width of this part-select does not fold to a constant: it applies"),
        ),
        // s0_msb_minus: oracles V:r=0000001f; PRE V:r=00000001
        (
            "s0_msb_minus",
            r#"typedef struct packed { logic [63:0] PAD; logic [7:0] HI; logic [7:0] LO; } prm_t;
module top #(parameter prm_t pt = 80'h0000000000000000_05_02, parameter logic [71:0] WP = 72'h00_0000_0000_0000_0005);
  logic [31:0] f, r, a;
  integer i, n;
  initial begin
    f = 32'hffff_ffff; r = 0; a = 0; n = 0;
    r = f[pt.HI-1:0];
    #1 $display("r=%h", r);
    $finish;
  end
endmodule
"#,
            Want::Refused("the width of this part-select does not fold to a constant: it reads `p"),
        ),
        // d42_x_in_bound: oracles V:r=xxxxxxxx; PRE V:r=00000001
        (
            "d42_x_in_bound",
            r#"module top #(parameter logic [71:0] WP = {64'h0, 8'bxxxx_0101});
  logic [31:0] f, r, a;
  initial begin
    f = 32'h89ab_cdef; r = 0; a = 0;
    r = f[WP[7:0]+1:0];
    #1 $display("r=%h", r);
    $finish;
  end
endmodule
"#,
            Want::Refused("the width of this part-select does not fold to a constant: its value h"),
        ),
    ]);
}

/// Late widths are decided before the multidriver scan reads a continuous-assign chunk
/// (RC7): disjoint late chunks no longer overlap, a true overlap still does.
#[test]
fn continuous_assign_widths_before_the_multidriver_scan() {
    check(&[
        // m1_ca_late_wrap_overlap: oracles iv IV_BUILD_FAILED; sv2v w=00000015; ver w=00000015; PRE warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in
        (
            "m1_ca_late_wrap_overlap",
            r#"module top;
  if (1) begin : gb localparam logic [31:0] U = 32'hFFFF_FFFE; end
  wire [31:0] w;
  assign w[gb.U + 6 : 0] = 5'h15;
  assign w[31:5] = '0;
  initial begin #1 $display("w=%h", w); end
endmodule
"#,
            Want::Prints("w=00000015"),
        ),
        // m2_ca_late_signed_overlap: oracles iv IV_BUILD_FAILED; sv2v w=00000015; ver w=00000015; PRE warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in
        (
            "m2_ca_late_signed_overlap",
            r#"module top;
  if (1) begin : gb localparam int N = -2; end
  wire [31:0] w;
  assign w[gb.N + 6 : 0] = 5'h15;
  assign w[31:5] = '0;
  initial begin #1 $display("w=%h", w); end
endmodule
"#,
            Want::Prints("w=00000015"),
        ),
        // m3_ca_late_g1_overlap: oracles iv IV_BUILD_FAILED; sv2v w=00000015; ver w=00000015; PRE warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in
        (
            "m3_ca_late_g1_overlap",
            r#"module top;
  if (1) begin : gb localparam int LO = 32'h1000_0000; localparam int HI = 32'h1000_0004; end
  wire [31:0] w;
  assign w[gb.HI - gb.LO : 0] = 5'h15;
  assign w[31:5] = '0;
  initial begin #1 $display("w=%h", w); end
endmodule
"#,
            Want::Prints("w=00000015"),
        ),
        // m4_ca_late_g1_true_overlap: oracles iv IV_BUILD_FAILED; sv2v w=00000XX5; PRE warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in
        (
            "m4_ca_late_g1_true_overlap",
            r#"module top;
  if (1) begin : gb localparam int LO = 32'h1000_0000; localparam int HI = 32'h1000_0008; end
  logic [31:0] w;
  assign w[gb.HI - gb.LO : 0] = 9'h155;
  assign w[31:5] = '0;
  initial begin #1 $display("w=%h", w); end
endmodule
"#,
            Want::Refused("net `top.w` driven by multiple overlapping continuous assignments"),
        ),
    ]);
}

/// A negative literal bound keeps its PRE tree for read and write: an oracle split
/// (iverilog and sv2v refuse it as out of order, verilator reads a two-bit select).
#[test]
fn negative_literal_split_keeps_its_old_read_and_write() {
    check(&[
        // rh01_neg_lit_write: oracles V:a=00000003 w=00000000; PRE V:a=00000003 w=zzzzzzzz
        (
            "rh01_neg_lit_write",
            r#"module top;
  logic [31:0] f, r, a;
  wire [31:0] w;

  initial begin
    f = 32'h89ab_cdef; r = 0; a = 32'h0;
    a[-1:0] = 2'b11;
    #1 $display("a=%h w=%h", a, w);
    $finish;
  end
endmodule
"#,
            Want::Prints("a=00000003 w=zzzzzzzz"),
        ),
        // rh02_neg_lit_write_wide: oracles V:a=00000003 w=00000000; PRE V:a=ffffffff w=zzzzzzzz
        (
            "rh02_neg_lit_write_wide",
            r#"module top;
  logic [31:0] f, r, a;
  wire [31:0] w;

  initial begin
    f = 32'h89ab_cdef; r = 0; a = 32'h0;
    a[-1:0] = 32'hffff_ffff;
    #1 $display("a=%h w=%h", a, w);
    $finish;
  end
endmodule
"#,
            Want::Prints("a=ffffffff w=zzzzzzzz"),
        ),
        // rh03_neg_lit_ca: oracles V:a=00000000 w=00000003; PRE V:a=00000000 w=00000003
        (
            "rh03_neg_lit_ca",
            r#"module top;
  logic [31:0] f, r, a;
  wire [31:0] w;
  assign w[-1:0] = 2'b11;
  initial begin
    f = 32'h89ab_cdef; r = 0; a = 32'h0;

    #1 $display("a=%h w=%h", a, w);
    $finish;
  end
endmodule
"#,
            Want::Prints("a=00000000 w=00000003"),
        ),
        // rh04_neg_lit_write_nba: oracles V:a=00000003 w=00000000; PRE V:a=ffffffff w=zzzzzzzz
        (
            "rh04_neg_lit_write_nba",
            r#"module top;
  logic [31:0] f, r, a;
  wire [31:0] w;

  initial begin
    f = 32'h89ab_cdef; r = 0; a = 32'h0;
    a[-1:0] <= 32'hffff_ffff;
    #1 $display("a=%h w=%h", a, w);
    $finish;
  end
endmodule
"#,
            Want::Prints("a=ffffffff w=zzzzzzzz"),
        ),
    ]);
}

/// A decided tree is kept only when elaborate's own fold reads its value too: a
/// `$clog2` width the engine reads but elaborate's mirror does not becomes a `Const`, so a
/// fill, a cast or a context sized from it while lowering sees 3, not a default.
#[test]
fn elaborate_reads_the_decided_width_too() {
    check(&[
        // c01_idx_clog2_fill: oracles iv a=00000007; ver a=00000007
        (
            "c01_idx_clog2_fill",
            r#"module top;
  logic [31:0] f, a, r;
  localparam int P = 8;
  initial begin
    f = 32'h89ab_cdef; a = 0; r = 0;
    a[0 +: $clog2(P)] = '1;
    #1 $display("a=%h", a);
    $finish;
  end
endmodule
"#,
            Want::Prints("a=00000007"),
        ),
        // c03_rep_clog2_concat_ctx: oracles iv r=00000000; ver r=00000000
        (
            "c03_rep_clog2_concat_ctx",
            r#"module top;
  logic [31:0] f, a, r;
  localparam int P = 8;
  initial begin
    f = 32'h89ab_cdef; a = 0; r = 0;
    r = {{$clog2(P){1'b1}} ^ '1};
    #1 $display("r=%h", r);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=00000000"),
        ),
        // c05_rep_clog2_in_cast: oracles iv r=00000007; ver r=00000007
        (
            "c05_rep_clog2_in_cast",
            r#"module top;
  logic [31:0] f, a, r;
  localparam int P = 8;
  initial begin
    f = 32'h89ab_cdef; a = 0; r = 0;
    r = 16'({$clog2(P){1'b1}});
    #1 $display("r=%h", r);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=00000007"),
        ),
    ]);
}

/// A late replication count takes its own zero-count legality, not that of an edge nested
/// inside it (`{8'hA5, {(gb.Z + $bits({2{1'b1}}) - 2){1'b1}}}` with `Z = 0` is a zero count
/// inside a concatenation).
#[test]
fn a_late_count_keeps_its_own_zero_count_rule() {
    check(&[
        // z1_nested_rep_in_count: oracles sv2v r=000000a5; ver r=000000a5; PRE r=000000a5
        (
            "z1_nested_rep_in_count",
            r#"module top;
  logic [31:0] f, r;
  if (1) begin : gb
    localparam int Z = 0;
  end
  initial begin
    f = 32'hffff_ffff; r = 0;
    r = {8'hA5, {(gb.Z + $bits({2{1'b1}}) - 2){1'b1}}};
    #1 $display("r=%h", r);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=000000a5"),
        ),
        // z2_plain_twin: oracles sv2v r=000000a5; ver r=000000a5; PRE r=000000a5
        (
            "z2_plain_twin",
            r#"module top;
  logic [31:0] f, r;
  if (1) begin : gb
    localparam int Z = 0;
  end
  initial begin
    f = 32'hffff_ffff; r = 0;
    r = {8'hA5, {(gb.Z + 2 - 2){1'b1}}};
    #1 $display("r=%h", r);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=000000a5"),
        ),
        // z3_nested_rep_count_local: oracles sv2v r=000000a5; ver r=000000a5; PRE r=000000a5
        (
            "z3_nested_rep_count_local",
            r#"module top;
  logic [31:0] f, r;
  if (1) begin : gb
    localparam int Z = 0;
  end
  initial begin
    f = 32'hffff_ffff; r = 0;
    r = {8'hA5, {(0 + $bits({2{1'b1}}) - 2){1'b1}}};
    #1 $display("r=%h", r);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=000000a5"),
        ),
        // z4_nested_rep_in_count_nonzero: oracles sv2v r=0000014b; ver r=0000014b; PRE r=0000014b
        (
            "z4_nested_rep_in_count_nonzero",
            r#"module top;
  logic [31:0] f, r;
  if (1) begin : gb
    localparam int Z = 0;
  end
  initial begin
    f = 32'hffff_ffff; r = 0;
    r = {8'hA5, {(gb.Z + $bits({2{1'b1}}) - 1){1'b1}}};
    #1 $display("r=%h", r);
    $finish;
  end
endmodule
"#,
            Want::Prints("r=0000014b"),
        ),
    ]);
}
