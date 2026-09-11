//! §2 "Index sealing" residues ⓐⓑⓒ — a DECLARED-width leaf that the width-certification
//! gates declined: a **genvar**, an **integer-returning system function**, and a
//! **`pkg::` constant**.
//!
//! One gate, `ctx_width_names_are_evident`, answered `false` for two of the three and a
//! missing `param_range` row answered for the third, so every consumer that requires
//! DECLARED provenance fell back to the LEAF's own default width. Measured over a
//! 134-cell census against BOTH oracles: 34 cells were silent-wrong before, 3 after.
//!
//! **What each cell is.** `C1_*` = direct override `#(.P(~LEAF))`; `C2_*` = derived
//! `localparam R = …; #(.P(R))`; `C4_*` = the select bound `localparam W = ~LEAF; logic
//! [(W[15:8])+8-1:0] v;`. `a*`/`b1`/`c*` are the ROADMAP rows' own designs.
//!
//! ⚠️ **POLICY — a genvar's SELF width is an oracle split, and it is decided here, not
//! measured.** iverilog binds `#(.P(i))` at 32 bits; verilator binds it at 1. The split
//! is DISQUALIFIED on verilator's side by its own self-contradiction: asked for the same
//! genvar's width by every other spelling it answers 32 —
//!
//! ```text
//! bits_i=32  bits_i_plus0=32  bits_concat_i=32  bits_i_shl0=32  bits_J=32
//! ```
//!
//! (`$bits(i)`, `$bits(i+0)`, `$bits({i})`, `$bits(i<<0)`, `localparam integer J = i;
//! $bits(J)` — verilator 5.052, one design, one genvar) — while binding a ONE-BIT
//! override from that same 32-bit value. iverilog is self-contradictory in the other
//! direction (its `$bits(i)` is 2 where its override binding is 32). IEEE 1800 §27.4
//! makes a genvar a signed 32-bit integer, so vita follows the LRM and iverilog: **32**.
//! Group E pins the five cells that ride on that decision.
//!
//! ⚠️ **`pkg::` leaves are keyed QUALIFIED (`pkg::name`), never bare.** `S_collide` is
//! why: `parameter [7:0] PW = 8'haa;` in the module beside `pk::PW` at 36 bits, in the
//! one expression `pk::PW + PW`, is a design both oracles run. A bare `PW` env key would
//! hand the module's 8-bit parameter the package constant's 36 bits.
//!
//! ⚠️ **The system-function admission is a NAMED list** (`$clog2`, `$bits`, `$rtoi` —
//! exactly the three `const_fn.rs` folds into this domain), never a blanket `SysCall`.
//! The width rule it unlocks says "32 bits, signed", which is false for a real-returning
//! call. `real_returning_syscall_stays_loud` is the control: `$itor` is unchanged, and
//! the two oracles SPLIT on it (iverilog rejects, verilator folds), so it is not
//! arbitrable and vita keeps its refusal.
//!
//! **Not closed by this slice**, still silent-wrong and listed so a partial fix cannot
//! hide: `C1_bitsx_bare` (a BARE `$bits(x)` as the whole override — `override_self_meta`
//! requires a top-level operator, a different lane), and `C1_pkq80_not` / `C1_plp80_not`
//! (the pre-existing >64-bit direct-override decline in `const_ctx_within_i64`; the
//! second is a PLAIN `parameter [79:0]` twin, which is what proves it is not a `pkg::`
//! defect). `S_clog_neg` (an unsigned fold of a signed `$clog2` result — `4294967285`
//! where both oracles print `-11`) was closed afterwards by the `SysCall` arm in
//! `const_expr_signed`; its pins live in `sysfn_integer_const_sign.rs`.
//!
//! Values pinned to iverilog 13.0 and verilator 5.052; every design carries an
//! `initial #1 $finish;` watchdog.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// The preamble 123 of the 134 census designs share.
const PK: &str = "package pk;
  parameter logic [35:0] PW = 36'h0a;
  parameter logic [79:0] PW80 = 80'h0a;
endpackage
";
const LEAF: &str = "module leaf #(parameter P = 1'b0) ();
  initial $display(\"RES bits=%0d val=%0h\", $bits(P), P);
endmodule
";

fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_dlc_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (s, out.status.success())
}

/// `want` must appear, and the run must SUCCEED — a diagnostic here would be a
/// loud regression, which is what the `ok` half catches.
#[track_caller]
fn chk_raw(src: &str, want: &str) {
    let (s, ok) = run(src);
    assert!(ok, "vita rejected the design:\n{s}");
    assert!(s.contains(want), "want {want:?}\ngot:\n{s}");
}
#[track_caller]
fn chk(body: &str, want: &str) {
    chk_raw(&format!("{PK}{LEAF}{body}\n"), want);
}
#[track_caller]
fn chk_pk(body: &str, want: &str) {
    chk_raw(&format!("{PK}{body}\n"), want);
}

// ===== A. genvar (fix 4a): silent-wrong -> the value BOTH oracles print =====

/// `C1_gv_not` — PRE `RES bits=1 val=1` · both oracles `RES bits=32 val=ffffffff`.
#[test]
fn cell_c1_gv_not() {
    chk(
        r#"module top;
  genvar i;
  generate for (i=0;i<1;i=i+1) begin : G
    leaf #(.P(~(i))) u1();
  end endgenerate
  initial #1 $finish;
endmodule"#,
        "RES bits=32 val=ffffffff",
    );
}

/// `C2_gv_not` — PRE `RES bits=1 val=1` · both oracles `RES bits=32 val=ffffffff`.
#[test]
fn cell_c2_gv_not() {
    chk(
        r#"module top;
  genvar i;
  generate for (i=0;i<1;i=i+1) begin : G
    localparam R = ~(i);
    leaf #(.P(R)) u1();
  end endgenerate
  initial #1 $finish;
endmodule"#,
        "RES bits=32 val=ffffffff",
    );
}

/// `C4_gv` — PRE `RES bits=1 val=0` · both oracles `RES bits=263 val=0`.
#[test]
fn cell_c4_gv() {
    chk_pk(
        r#"module top;
  genvar i;
  generate for (i=0;i<1;i=i+1) begin : G
  localparam W = ~(i);
    logic [(W[15:8])+8-1:0] v;
    initial $display("RES bits=%0d val=0", $bits(v));
  end endgenerate
  initial #1 $finish;
endmodule"#,
        "RES bits=263 val=0",
    );
}

/// `S_gv_neg` — PRE `RES bits=1 val=1` · both oracles `RES bits=32 val=ffffffff`.
#[test]
fn cell_s_gv_neg() {
    chk(
        r#"module top;
  parameter [7:0] Q8 = 8'h0a;
  genvar i;
  generate for (i=0;i<1;i=i+1) begin : G
    leaf #(.P(i - 1)) u1();
  end endgenerate
  initial #1 $finish;
endmodule"#,
        "RES bits=32 val=ffffffff",
    );
}

/// `S_gv_nest` — PRE `RES bits=1 val=0` · both oracles `RES bits=8 val=a`.
#[test]
fn cell_s_gv_nest() {
    chk(
        r#"module top;
  parameter [7:0] Q8 = 8'h0a;
  genvar i,j;
  generate for (i=0;i<1;i=i+1) begin : G
    for (j=0;j<1;j=j+1) begin : H
      leaf #(.P(Q8 << (i+j))) u1();
    end
  end endgenerate
  initial #1 $finish;
endmodule"#,
        "RES bits=8 val=a",
    );
}

/// `S_gv_if` — PRE `RES bits=1 val=0` · both oracles `RES bits=8 val=a`.
#[test]
fn cell_s_gv_if() {
    chk(
        r#"module top;
  parameter [7:0] Q8 = 8'h0a;
  genvar i;
  generate for (i=0;i<1;i=i+1) begin : G
    if (i==0) begin : K
      leaf #(.P(Q8 << i)) u1();
    end
  end endgenerate
  initial #1 $finish;
endmodule"#,
        "RES bits=8 val=a",
    );
}

/// `S_gv_sh40` — PRE `RES bits=1 val=0` · both oracles `RES bits=64 val=10000000000`.
#[test]
fn cell_s_gv_sh40() {
    chk(
        r#"module top;
  genvar i;
  generate for (i=1;i<2;i=i+1) begin : G
    leaf #(.P(64'h1 << (i+39))) u1();
  end endgenerate
  initial #1 $finish;
endmodule"#,
        "RES bits=64 val=10000000000",
    );
}

/// `a1` — PRE `leaf bits=32 val=a` · both oracles `leaf bits=8 val=a`.
#[test]
fn cell_a1() {
    chk_raw(
        r#"module leaf #(parameter P = 1) ();
  initial $display("leaf bits=%0d val=%0h", $bits(P), P);
endmodule
module top;
  parameter Q = 8'h0a;
  genvar i;
  generate for (i=0;i<1;i=i+1) begin : G
    leaf #(.P(Q << i)) u1();
  end endgenerate
  initial #1 $finish;
endmodule"#,
        "leaf bits=8 val=a",
    );
}

/// `a1c` — PRE `leaf bits=1 val=0` · both oracles `leaf bits=8 val=a`.
#[test]
fn cell_a1c() {
    chk_raw(
        r#"module leaf #(parameter P = 1'b0) ();
  initial $display("leaf bits=%0d val=%0h", $bits(P), P);
endmodule
module top;
  parameter Q = 8'h0a;
  genvar i;
  generate for (i=0;i<1;i=i+1) begin : G
    leaf #(.P(Q << i)) u1();
  end endgenerate
  initial #1 $finish;
endmodule"#,
        "leaf bits=8 val=a",
    );
}

/// `a2` — PRE `leaf bits=32 val=a` · both oracles `leaf bits=8 val=a`.
#[test]
fn cell_a2() {
    chk_raw(
        r#"module leaf #(parameter P = 1) ();
  initial $display("leaf bits=%0d val=%0h", $bits(P), P);
endmodule
module top;
  parameter Q = 8'h0a;
  genvar i;
  generate for (i=0;i<1;i=i+1) begin : G
    localparam R = Q << i;
    leaf #(.P(R)) u1();
  end endgenerate
  initial #1 $finish;
endmodule"#,
        "leaf bits=8 val=a",
    );
}

// ===== B. integer-returning system function (fix 4b): silent-wrong -> the value BOTH oracles print =====

/// `C1_cl300_not` — PRE `RES bits=1 val=0` · both oracles `RES bits=32 val=fffffff6`.
#[test]
fn cell_c1_cl300_not() {
    chk(
        r#"module top;
  leaf #(.P(~($clog2(300)))) u1();
  initial #1 $finish;
endmodule"#,
        "RES bits=32 val=fffffff6",
    );
}

/// `C1_clQ_not` — PRE `RES bits=1 val=1` · both oracles `RES bits=32 val=fffffffb`.
#[test]
fn cell_c1_clq_not() {
    chk(
        r#"module top;
  parameter [7:0] Q8 = 8'h0a;
  leaf #(.P(~($clog2(Q8)))) u1();
  initial #1 $finish;
endmodule"#,
        "RES bits=32 val=fffffffb",
    );
}

/// `C1_bitsx_not` — PRE `RES bits=1 val=1` · both oracles `RES bits=32 val=fffffff7`.
#[test]
fn cell_c1_bitsx_not() {
    chk(
        r#"module top;
  logic [7:0] xx;
  leaf #(.P(~($bits(xx)))) u1();
  initial #1 $finish;
endmodule"#,
        "RES bits=32 val=fffffff7",
    );
}

/// `C2_cl300_bare` — PRE `RES bits=1 val=1` · both oracles `RES bits=32 val=9`.
#[test]
fn cell_c2_cl300_bare() {
    chk(
        r#"module top;
  localparam R = $clog2(300);
  leaf #(.P(R)) u1();
  initial #1 $finish;
endmodule"#,
        "RES bits=32 val=9",
    );
}

/// `C2_cl300_not` — PRE `RES bits=1 val=0` · both oracles `RES bits=32 val=fffffff6`.
#[test]
fn cell_c2_cl300_not() {
    chk(
        r#"module top;
  localparam R = ~($clog2(300));
  leaf #(.P(R)) u1();
  initial #1 $finish;
endmodule"#,
        "RES bits=32 val=fffffff6",
    );
}

/// `C2_clQ_bare` — PRE `RES bits=1 val=0` · both oracles `RES bits=32 val=4`.
#[test]
fn cell_c2_clq_bare() {
    chk(
        r#"module top;
  parameter [7:0] Q8 = 8'h0a;
  localparam R = $clog2(Q8);
  leaf #(.P(R)) u1();
  initial #1 $finish;
endmodule"#,
        "RES bits=32 val=4",
    );
}

/// `C2_clQ_not` — PRE `RES bits=1 val=1` · both oracles `RES bits=32 val=fffffffb`.
#[test]
fn cell_c2_clq_not() {
    chk(
        r#"module top;
  parameter [7:0] Q8 = 8'h0a;
  localparam R = ~($clog2(Q8));
  leaf #(.P(R)) u1();
  initial #1 $finish;
endmodule"#,
        "RES bits=32 val=fffffffb",
    );
}

/// `C2_bitsx_bare` — PRE `RES bits=1 val=0` · both oracles `RES bits=32 val=8`.
#[test]
fn cell_c2_bitsx_bare() {
    chk(
        r#"module top;
  logic [7:0] xx;
  localparam R = $bits(xx);
  leaf #(.P(R)) u1();
  initial #1 $finish;
endmodule"#,
        "RES bits=32 val=8",
    );
}

/// `C2_bitsx_not` — PRE `RES bits=1 val=1` · both oracles `RES bits=32 val=fffffff7`.
#[test]
fn cell_c2_bitsx_not() {
    chk(
        r#"module top;
  logic [7:0] xx;
  localparam R = ~($bits(xx));
  leaf #(.P(R)) u1();
  initial #1 $finish;
endmodule"#,
        "RES bits=32 val=fffffff7",
    );
}

/// `C4_cl300` — PRE `RES bits=1 val=0` · both oracles `RES bits=263 val=0`.
#[test]
fn cell_c4_cl300() {
    chk_pk(
        r#"module top;
  localparam W = ~($clog2(300));
  logic [(W[15:8])+8-1:0] v;
  initial $display("RES bits=%0d val=0", $bits(v));
  initial #1 $finish;
endmodule"#,
        "RES bits=263 val=0",
    );
}

/// `C4_clQ` — PRE `RES bits=1 val=0` · both oracles `RES bits=263 val=0`.
#[test]
fn cell_c4_clq() {
    chk_pk(
        r#"module top;
  parameter [7:0] Q8 = 8'h0a;
  localparam W = ~($clog2(Q8));
  logic [(W[15:8])+8-1:0] v;
  initial $display("RES bits=%0d val=0", $bits(v));
  initial #1 $finish;
endmodule"#,
        "RES bits=263 val=0",
    );
}

/// `C4_bitsx` — PRE `RES bits=1 val=0` · both oracles `RES bits=263 val=0`.
#[test]
fn cell_c4_bitsx() {
    chk_pk(
        r#"module top;
  logic [7:0] xx;
  localparam W = ~($bits(xx));
  logic [(W[15:8])+8-1:0] v;
  initial $display("RES bits=%0d val=0", $bits(v));
  initial #1 $finish;
endmodule"#,
        "RES bits=263 val=0",
    );
}

/// `S_clog0` — PRE `RES bits=1 val=1` · both oracles `RES bits=32 val=ffffffff`.
#[test]
fn cell_s_clog0() {
    chk(
        r#"module top;
  leaf #(.P(~$clog2(0))) u1();
  initial #1 $finish;
endmodule"#,
        "RES bits=32 val=ffffffff",
    );
}

/// `S_clog1` — PRE `RES bits=1 val=1` · both oracles `RES bits=32 val=ffffffff`.
#[test]
fn cell_s_clog1() {
    chk(
        r#"module top;
  leaf #(.P(~$clog2(1))) u1();
  initial #1 $finish;
endmodule"#,
        "RES bits=32 val=ffffffff",
    );
}

/// `b1` — PRE `vbits=1` · both oracles `vbits=263`.
#[test]
fn cell_b1() {
    chk_raw(
        r#"module top;
  localparam A = $clog2(300);
  localparam W = ~A;
  logic [(W[15:8])+8-1:0] v;
  initial begin $display("vbits=%0d", $bits(v)); #1 $finish; end
endmodule"#,
        "vbits=263",
    );
}

// ===== C. `pkg::` leaf (fix 4c): silent-wrong -> the value BOTH oracles print =====

/// `C1_pkq_not` — PRE `RES bits=1 val=1` · both oracles `RES bits=36 val=ffffffff5`.
#[test]
fn cell_c1_pkq_not() {
    chk(
        r#"module top;
  leaf #(.P(~(pk::PW))) u1();
  initial #1 $finish;
endmodule"#,
        "RES bits=36 val=ffffffff5",
    );
}

/// `C2_pkq_bare` — PRE `RES bits=1 val=0` · both oracles `RES bits=36 val=a`.
#[test]
fn cell_c2_pkq_bare() {
    chk(
        r#"module top;
  localparam R = pk::PW;
  leaf #(.P(R)) u1();
  initial #1 $finish;
endmodule"#,
        "RES bits=36 val=a",
    );
}

/// `C2_pkq_not` — PRE `RES bits=1 val=1` · both oracles `RES bits=36 val=ffffffff5`.
#[test]
fn cell_c2_pkq_not() {
    chk(
        r#"module top;
  localparam R = ~(pk::PW);
  leaf #(.P(R)) u1();
  initial #1 $finish;
endmodule"#,
        "RES bits=36 val=ffffffff5",
    );
}

/// `C2_pkq80_bare` — PRE `RES bits=1 val=0` · both oracles `RES bits=80 val=a`.
#[test]
fn cell_c2_pkq80_bare() {
    chk(
        r#"module top;
  localparam R = pk::PW80;
  leaf #(.P(R)) u1();
  initial #1 $finish;
endmodule"#,
        "RES bits=80 val=a",
    );
}

/// `C4_pkq` — PRE `RES bits=1 val=0` · both oracles `RES bits=263 val=0`.
#[test]
fn cell_c4_pkq() {
    chk_pk(
        r#"module top;
  localparam W = ~(pk::PW);
  logic [(W[15:8])+8-1:0] v;
  initial $display("RES bits=%0d val=0", $bits(v));
  initial #1 $finish;
endmodule"#,
        "RES bits=263 val=0",
    );
}

/// `S_gvpk` — PRE `RES bits=1 val=0` · both oracles `RES bits=36 val=a`.
#[test]
fn cell_s_gvpk() {
    chk_raw(
        r#"package pk; parameter logic [35:0] PW = 36'h0a; endpackage
module leaf #(parameter P = 1'b0) ();
  initial $display("RES bits=%0d val=%0h", $bits(P), P);
endmodule
module top;
  genvar i;
  generate for (i=0;i<1;i=i+1) begin : G
    leaf #(.P(pk::PW << i)) u1();
  end endgenerate
  initial #1 $finish;
endmodule"#,
        "RES bits=36 val=a",
    );
}

/// `c1` — PRE `leaf bits=32 val=fffffff5` · both oracles `leaf bits=36 val=ffffffff5`.
#[test]
fn cell_c1() {
    chk_raw(
        r#"package pk; parameter logic [35:0] PW = 36'h0a; endpackage
module leaf #(parameter P = 1) ();
  initial $display("leaf bits=%0d val=%0h", $bits(P), P);
endmodule
module top;
  leaf #(.P(~pk::PW)) u1();
  initial #1 $finish;
endmodule"#,
        "leaf bits=36 val=ffffffff5",
    );
}

// ===== D. controls: correct BEFORE this slice and unchanged by it =====

/// `a3` — PRE `leaf bits=8 val=14` · both oracles `leaf bits=8 val=14`.
#[test]
fn cell_a3() {
    chk_raw(
        r#"module leaf #(parameter P = 1) ();
  initial $display("leaf bits=%0d val=%0h", $bits(P), P);
endmodule
module top;
  parameter Q = 8'h0a;
  genvar i;
  generate for (i=0;i<1;i=i+1) begin : G
    localparam R = Q << 1;
    leaf #(.P(R)) u1();
  end endgenerate
  initial #1 $finish;
endmodule"#,
        "leaf bits=8 val=14",
    );
}

/// `a1b` — PRE `leaf bits=1 val=0` · both oracles `leaf bits=1 val=0`.
#[test]
fn cell_a1b() {
    chk_raw(
        r#"module leaf #(parameter logic P = 1'b0) ();
  initial $display("leaf bits=%0d val=%0h", $bits(P), P);
endmodule
module top;
  parameter Q = 8'h0a;
  genvar i;
  generate for (i=0;i<1;i=i+1) begin : G
    leaf #(.P(Q << i)) u1();
  end endgenerate
  initial #1 $finish;
endmodule"#,
        "leaf bits=1 val=0",
    );
}

/// `c3` — PRE `leaf bits=36 val=a` · both oracles `leaf bits=36 val=a`.
#[test]
fn cell_c3() {
    chk_raw(
        r#"package pk; parameter logic [35:0] PW = 36'h0a; endpackage
module leaf #(parameter P = 1) ();
  initial $display("leaf bits=%0d val=%0h", $bits(P), P);
endmodule
module top;
  leaf #(.P(pk::PW)) u1();
  initial #1 $finish;
endmodule"#,
        "leaf bits=36 val=a",
    );
}

/// `S_pk_imp` — PRE `RES bits=36 val=ffffffff5` · both oracles `RES bits=36 val=ffffffff5`.
#[test]
fn cell_s_pk_imp() {
    chk(
        r#"module top;
  import pk::*;
  leaf #(.P(~PW)) u1();
  initial #1 $finish;
endmodule"#,
        "RES bits=36 val=ffffffff5",
    );
}

/// `C1_cl300_bare` — PRE `RES bits=32 val=9` · both oracles `RES bits=32 val=9`.
#[test]
fn cell_c1_cl300_bare() {
    chk(
        r#"module top;
  leaf #(.P($clog2(300))) u1();
  initial #1 $finish;
endmodule"#,
        "RES bits=32 val=9",
    );
}

/// `C1_pkq_bare` — PRE `RES bits=36 val=a` · both oracles `RES bits=36 val=a`.
#[test]
fn cell_c1_pkq_bare() {
    chk(
        r#"module top;
  leaf #(.P(pk::PW)) u1();
  initial #1 $finish;
endmodule"#,
        "RES bits=36 val=a",
    );
}

/// `C1_pkq80_bare` — PRE `RES bits=80 val=a` · both oracles `RES bits=80 val=a`.
#[test]
fn cell_c1_pkq80_bare() {
    chk(
        r#"module top;
  leaf #(.P(pk::PW80)) u1();
  initial #1 $finish;
endmodule"#,
        "RES bits=80 val=a",
    );
}

/// `C3_pkq` — PRE `RES bits=12 val=0` · both oracles `RES bits=12 val=0`.
#[test]
fn cell_c3_pkq() {
    chk_pk(
        r#"module top;
  logic [((((pk::PW) | 3) << 30) >> 30) : 0] v;
  initial $display("RES bits=%0d val=0", $bits(v));
  initial #1 $finish;
endmodule"#,
        "RES bits=12 val=0",
    );
}

/// `C3b_pkq80` — PRE `RES bits=12 val=0` · both oracles `RES bits=12 val=0`.
#[test]
fn cell_c3b_pkq80() {
    chk_pk(
        r#"module top;
  logic [((((pk::PW80) | 3) << 34) >> 34) : 0] v;
  initial $display("RES bits=%0d val=0", $bits(v));
  initial #1 $finish;
endmodule"#,
        "RES bits=12 val=0",
    );
}

/// `C1_ilp_not` — PRE `RES bits=32 val=fffffff5` · both oracles `RES bits=32 val=fffffff5`.
#[test]
fn cell_c1_ilp_not() {
    chk(
        r#"module top;
  localparam integer IL = 10;
  leaf #(.P(~(IL))) u1();
  initial #1 $finish;
endmodule"#,
        "RES bits=32 val=fffffff5",
    );
}

/// `C2_ulp_not` — PRE `RES bits=8 val=f5` · both oracles `RES bits=8 val=f5`.
#[test]
fn cell_c2_ulp_not() {
    chk(
        r#"module top;
  localparam UL = 8'h0a;
  localparam R = ~(UL);
  leaf #(.P(R)) u1();
  initial #1 $finish;
endmodule"#,
        "RES bits=8 val=f5",
    );
}

/// `C1_plp36_not` — PRE `RES bits=36 val=ffffffff5` · both oracles `RES bits=36 val=ffffffff5`.
#[test]
fn cell_c1_plp36_not() {
    chk(
        r#"module top;
  parameter [35:0] PQ36 = 36'h0a;
  leaf #(.P(~(PQ36))) u1();
  initial #1 $finish;
endmodule"#,
        "RES bits=36 val=ffffffff5",
    );
}

// ===== E. oracle-split cells, pinned under the POLICY in the header =====

/// `C1_gv_bare` — PRE `RES bits=1 val=0` · iverilog `RES bits=32 val=0` · verilator `RES bits=1 val=0`. POLICY: 32, IEEE §27.4 + iverilog; verilator's 1 is disqualified (header).
#[test]
fn cell_c1_gv_bare() {
    chk(
        r#"module top;
  genvar i;
  generate for (i=0;i<1;i=i+1) begin : G
    leaf #(.P(i)) u1();
  end endgenerate
  initial #1 $finish;
endmodule"#,
        "RES bits=32 val=0",
    );
}

/// `S_gv_two` — PRE `RES bits=1 val=0` · iverilog `RES bits=32 val=0` · verilator `RES bits=1 val=0`. POLICY: 32, IEEE §27.4 + iverilog; verilator's 1 is disqualified (header).
#[test]
fn cell_s_gv_two() {
    chk(
        r#"module top;
  genvar i,j;
  generate for (i=0;i<1;i=i+1) begin : G
    for (j=0;j<1;j=j+1) begin : H
      leaf #(.P(i | j)) u1();
    end
  end endgenerate
  initial #1 $finish;
endmodule"#,
        "RES bits=32 val=0",
    );
}

/// `c2` — PRE `leaf bits=32 val=b` · iverilog `leaf bits=37 val=b` · verilator `leaf bits=36 val=b`. Both oracles are >32 and vita matched NEITHER before; it now matches verilator. iverilog's 37 is its known `+` max+1.
#[test]
fn cell_c2() {
    chk_raw(
        r#"package pk; parameter logic [35:0] PW = 36'h0a; endpackage
module leaf #(parameter P = 1) ();
  initial $display("leaf bits=%0d val=%0h", $bits(P), P);
endmodule
module top;
  leaf #(.P(pk::PW + 1)) u1();
  initial #1 $finish;
endmodule"#,
        "leaf bits=36 val=b",
    );
}

/// `S_pk_plus` — PRE `RES bits=1 val=1` · iverilog `RES bits=37 val=b` · verilator `RES bits=36 val=b`. Same split as `c2`; vita matched neither before.
#[test]
fn cell_s_pk_plus() {
    chk(
        r#"module top;
  leaf #(.P(pk::PW + 1)) u1();
  initial #1 $finish;
endmodule"#,
        "RES bits=36 val=b",
    );
}

/// `S_collide` — PRE `RES bits=1 val=0` · iverilog `RES bits=37 val=b4` · verilator `RES bits=36 val=b4`. The KEY-COLLISION pin: module `PW` (8 bits) beside `pk::PW` (36). A bare-name env key would hand the module parameter the package constant's width. vita matched neither oracle before.
#[test]
fn cell_s_collide() {
    chk_raw(
        r#"package pk; parameter logic [35:0] PW = 36'h0a; endpackage
module leaf #(parameter P = 1'b0) ();
  initial $display("RES bits=%0d val=%0h", $bits(P), P);
endmodule
module top;
  parameter [7:0] PW = 8'haa;
  leaf #(.P(pk::PW + PW)) u1();
  initial #1 $finish;
endmodule"#,
        "RES bits=36 val=b4",
    );
}

// ===== F. `wide_name_bits` census — what the genvar's new `param_range` row makes the
// >64-bit NAME resolver answer that it DECLINED before. Three new answers, all
// loud -> value, all matching both oracles. =====

/// `wn_shr80` — PRE `error[VITA-E3009] … the `>>` operation has no constant-fold arm`,
/// then three cascading E3010s. Both oracles `X=7fffffffffffffffffff bits=80` / `Xi=1`.
#[test]
fn wide_domain_genvar_shift_count() {
    chk_raw(
        r#"module top;
  genvar i;
  generate for (i = 1; i < 2; i = i + 1) begin : G
    localparam [79:0] X = {80{1'b1}} >> i;
    initial begin
      $display("X=%0h bits=%0d", X, $bits(X));
      $display("Xi=%0b", X[i]);
    end
  end endgenerate
  initial #1 $finish;
endmodule
"#,
        "X=7fffffffffffffffffff bits=80",
    );
}

/// `wn2` — a SELECT OF THE GENVAR in the wide domain. PRE `error[VITA-E3009] … the
/// part-select `i[…]` has no constant-fold arm`. Both oracles `Z=fffffffffffffffffffe
/// bits=80`.
#[test]
fn wide_domain_genvar_select() {
    chk_raw(
        r#"module top;
  genvar i;
  generate for (i = 5; i < 6; i = i + 1) begin : G
    localparam [79:0] Z = {80{1'b1}} << i[1:0];
    initial $display("Z=%0h bits=%0d", Z, $bits(Z));
  end endgenerate
  initial #1 $finish;
endmodule
"#,
        "Z=fffffffffffffffffffe bits=80",
    );
}

/// `wn3` — a NON-COMMUTING operator (`/`) over a genvar at 96 bits, so the readout is
/// the evaluation width itself. PRE `error[VITA-E3009] … the `/` operation has no
/// constant-fold arm`. Both oracles `V=555555555555555555555555 bits=96`.
#[test]
fn wide_domain_genvar_divide() {
    chk_raw(
        r#"module top;
  genvar i;
  generate for (i = 2; i < 3; i = i + 1) begin : G
    localparam [95:0] V = {96{1'b1}} / (i + 1);
    initial $display("V=%0h bits=%0d", V, $bits(V));
  end endgenerate
  initial #1 $finish;
endmodule
"#,
        "V=555555555555555555555555 bits=96",
    );
}

/// `wn1` — the control for the three above: an operator the NARROW lane already folded.
/// Unchanged, `Y=102 bits=80` in PRE, POST and both oracles.
#[test]
fn wide_domain_genvar_add_unchanged() {
    chk_raw(
        r#"module top;
  genvar i;
  generate for (i = 3; i < 4; i = i + 1) begin : G
    localparam [79:0] Y = 80'hff + i;
    initial $display("Y=%0h bits=%0d", Y, $bits(Y));
  end endgenerate
  initial #1 $finish;
endmodule
"#,
        "Y=102 bits=80",
    );
}

// ===== G. axes the census left at one instance =====

/// THREE iterations with a per-iteration read — the per-iteration `bind_param_range`
/// at the rebind, which the one-instance census could not exercise. PRE printed
/// `RES bits=1 val=0` three times; both oracles print 8/`a`, 8/`14`, 8/`28`.
#[test]
fn genvar_three_iterations_each_certified() {
    let (s, ok) = run(&format!(
        "{LEAF}module top;
  parameter [7:0] Q8 = 8'h0a;
  genvar i;
  generate for (i = 0; i < 3; i = i + 1) begin : G
    leaf #(.P(Q8 << i)) u();
  end endgenerate
  initial #1 $finish;
endmodule
"
    ));
    assert!(ok, "{s}");
    for want in ["RES bits=8 val=a", "RES bits=8 val=14", "RES bits=8 val=28"] {
        assert!(s.contains(want), "want {want:?}\ngot:\n{s}");
    }
}

/// `$rtoi` is on the admitted list because `const_fn.rs` folds it. PRE
/// `RES bits=1 val=0`; both oracles `RES bits=32 val=fffffffc`.
#[test]
fn rtoi_is_a_certified_integer_leaf() {
    chk_raw(
        &format!(
            "{LEAF}module top;\n  leaf #(.P(~$rtoi(3.7))) u();\n  initial #1 $finish;\nendmodule\n"
        ),
        "RES bits=32 val=fffffffc",
    );
}

/// …and in the select-bound position. PRE `vbits=1`; both oracles `vbits=263`.
#[test]
fn rtoi_certifies_a_derived_select_bound() {
    chk_raw(
        r#"module top;
  localparam A = $rtoi(3.7);
  localparam W = ~A;
  logic [(W[15:8])+8-1:0] v;
  initial $display("vbits=%0d", $bits(v));
  initial #1 $finish;
endmodule
"#,
        "vbits=263",
    );
}

/// THE CONTROL for the named list: a REAL-returning system call is not on it and is
/// unchanged — still refused, by the same code, PRE and POST. The oracles SPLIT here
/// (iverilog `error: Unable to evaluate real parameter W value: ~(<A=3.00000>)`;
/// verilator folds it to `vbits=263`), so there is nothing to move toward, and the
/// refusal is the honest answer. Pinned by CODE, not by wording.
#[test]
fn real_returning_syscall_stays_loud() {
    let (s, ok) = run(r#"module top;
  localparam A = $itor(3);
  localparam W = ~A;
  logic [(W[15:8])+8-1:0] v;
  initial $display("vbits=%0d", $bits(v));
  initial #1 $finish;
endmodule
"#);
    assert!(!ok, "$itor must stay loud:\n{s}");
    assert!(s.contains("VITA-E3009"), "{s}");
    assert!(!s.contains("vbits="), "no value may be manufactured:\n{s}");
}
