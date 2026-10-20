//! §5.2 row 28 (§4.5.620): an assignment pattern into a constant SLICE of an unpacked array
//! (`v[1:0] <= '{a, b};`, the VeeR EH1 testbench `tb_top.sv:356`) is expanded element-wise, the
//! leftmost pattern element on the slice's left index (IEEE 1800-2017 §10.10, §7.6), pinned on
//! native, vm and interp. It was E3009 (`supported only as the whole right-hand side of an unpacked
//! array assignment`).
//!
//! Oracles: Icarus Verilog 13.0 `-g2012` (iv), sv2v 0.0.13 -> iverilog (s2v), Verilator 5.050
//! `--binary` (vl). iv refuses every slice of an unpacked array (`cannot perform a part select on
//! array`), so s2v and vl decide; each cell names who agrees. The cells are the row-28 census's,
//! byte for byte.
//!
//! Loud on purpose: a slice against the declared direction (s2v and vl assign in declaration order,
//! or disagree), a variable bound (vl: unsupported), an element-count mismatch, a blocking pattern
//! that reads the array it writes (s2v `5 3`, vl `5 5`), a slice copy, and — the write-side twin of
//! the read guard — a PACKED value assigned to a slice, which PRE wrote into element 0's bits at
//! exit 0 (`v[1:0] = 2'b11` stored `v[0] = 3`; `assign v[2:1] = x;` drove `v[0]`).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_on(src: &str, backend: &str) -> (Vec<String>, String, i32) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_pus_{backend}_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("t.sv"), src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .args(["--backend", backend, "t.sv"])
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    let lines = stdout
        .lines()
        .filter(|l| !l.starts_with("simulation ended"))
        .map(str::to_string)
        .collect();
    (lines, stderr, out.status.code().unwrap_or(-1))
}

/// Every backend runs `src` to a clean exit and prints exactly `want`.
fn pin(src: &str, want: &[&str]) {
    for be in ["native", "vm", "interp"] {
        let (lines, err, code) = run_on(src, be);
        assert_eq!(code, 0, "{be}: exit {code}\n{err}");
        assert_eq!(lines, want, "backend {be}");
    }
}

/// Every backend refuses `src` with E3009 naming `why`.
fn loud(src: &str, why: &str) {
    for be in ["native", "vm", "interp"] {
        let (_, err, code) = run_on(src, be);
        assert_ne!(code, 0, "{be}: expected a refusal\n{err}");
        assert!(
            err.contains("VITA-E3009") && err.contains(why),
            "{be}: expected E3009 naming {why:?}\n{err}"
        );
    }
}

/// the EH1 tb shape (`tb_top.sv:356`): sv2v = verilator; iverilog refuses every slice (`cannot perform a part select on array`). Cell `c28_01_witness`.
#[test]
fn z01_witness_nonblocking_whole_range() {
    pin(
        r#"module top;
  logic       v [1:0];
  logic [4:0] d [1:0];
  logic clk = 0;
  always @(posedge clk) begin
    v[1:0] <= '{1'b1, 1'b0};                // assignment pattern to a slice (here the whole range) of an unpacked array
    d[1:0] <= '{5'd3, 5'd7};
  end
  initial begin #1 clk = 1; #1 $display("v1=%b v0=%b d1=%0d d0=%0d", v[1], v[0], d[1], d[0]); $finish; end
endmodule
"#,
        &["v1=1 v0=0 d1=3 d0=7"],
    );
}

/// sv2v = verilator. Cell `c28_02_desc_full`.
#[test]
fn z02_descending_full_range() {
    pin(
        r#"module top;
  logic [3:0] v [1:0];
  initial begin
    v[1:0] = '{4'h3, 4'h5};
    $display("v1=%h v0=%h", v[1], v[0]);
  end
endmodule
"#,
        &["v1=3 v0=5"],
    );
}

/// sv2v = verilator. Cell `c28_03_asc_full`.
#[test]
fn z03_ascending_full_range() {
    pin(
        r#"module top;
  logic [3:0] v [0:1];
  initial begin
    v[0:1] = '{4'h3, 4'h5};
    $display("v0=%h v1=%h", v[0], v[1]);
  end
endmodule
"#,
        &["v0=3 v1=5"],
    );
}

/// sv2v = verilator; the elements outside the window keep their values. Cell `c28_04_desc_sub`.
#[test]
fn z04_descending_sub_range() {
    pin(
        r#"module top;
  logic [3:0] v [3:0];
  initial begin
    v[3]=4'h1; v[2]=4'h2; v[1]=4'h4; v[0]=4'h8;
    v[2:1] = '{4'ha, 4'h6};
    $display("v3=%h v2=%h v1=%h v0=%h", v[3], v[2], v[1], v[0]);
  end
endmodule
"#,
        &["v3=1 v2=a v1=6 v0=8"],
    );
}

/// sv2v = verilator. Cell `c28_05_asc_sub`.
#[test]
fn z05_ascending_sub_range() {
    pin(
        r#"module top;
  logic [3:0] v [0:3];
  initial begin
    v[0]=4'h1; v[1]=4'h2; v[2]=4'h4; v[3]=4'h8;
    v[1:2] = '{4'ha, 4'h6};
    $display("v0=%h v1=%h v2=%h v3=%h", v[0], v[1], v[2], v[3]);
  end
endmodule
"#,
        &["v0=1 v1=a v2=6 v3=8"],
    );
}

/// verilator (sv2v loses the sign); the whole-array twin prints the same on iverilog and verilator. Cell `c28_08_signed`.
#[test]
fn z06_signed_elements() {
    pin(
        r#"module top;
  logic signed [7:0] v [2:0];
  logic signed [3:0] a = -4'sd3;
  initial begin
    v[2] = 8'sd9;
    v[1:0] = '{a, 4'sd5};
    $display("v2=%0d v1=%0d v0=%0d", v[2], v[1], v[0]);
  end
endmodule
"#,
        &["v2=9 v1=-3 v0=5"],
    );
}

/// verilator; sv2v prints the same bits unsigned. Cell `c28_09_int`.
#[test]
fn z07_int_elements() {
    pin(
        r#"module top;
  int v [2:0];
  initial begin
    v[2] = 1; v[1] = 2; v[0] = 3;
    v[1:0] = '{-7, 9};
    $display("v2=%0d v1=%0d v0=%0d", v[2], v[1], v[0]);
  end
endmodule
"#,
        &["v2=1 v1=-7 v0=9"],
    );
}

/// verilator; the whole-array twin prints the same on iverilog and verilator. Cell `c28_10_real`.
#[test]
fn z08_real_elements() {
    pin(
        r#"module top;
  real v [2:0];
  initial begin
    v[0] = 0.5;
    v[2:1] = '{1.5, -2.25};
    $display("v2=%g v1=%g v0=%g", v[2], v[1], v[0]);
  end
endmodule
"#,
        &["v2=1.5 v1=-2.25 v0=0.5"],
    );
}

/// sv2v = verilator. Cell `c28_12_struct`.
#[test]
fn z09_packed_struct_elements() {
    pin(
        r#"typedef struct packed {logic [1:0] a; logic [2:0] b;} s_t;
module top;
  s_t v [2:0];
  s_t x, y;
  initial begin
    x = 5'b01110; y = 5'b10011;
    v[0] = 5'h1f;
    v[2:1] = '{x, y};
    $display("v2=%h v1=%h v0=%h", v[2], v[1], v[0]);
  end
endmodule
"#,
        &["v2=0e v1=13 v0=1f"],
    );
}

/// `'{default: v}` fills the window only: sv2v = verilator. Cell `c28_13_default`.
#[test]
fn z10_default_pattern() {
    pin(
        r#"module top;
  logic [3:0] v [3:0];
  initial begin
    v[3]=4'h1; v[2]=4'h2; v[1]=4'h4; v[0]=4'h8;
    v[2:1] = '{default: 4'h9};
    $display("v3=%h v2=%h v1=%h v0=%h", v[3], v[2], v[1], v[0]);
  end
endmodule
"#,
        &["v3=1 v2=9 v1=9 v0=8"],
    );
}

/// sv2v = verilator. Cell `c28_18_nba_ff`.
#[test]
fn z11_always_ff() {
    pin(
        r#"module top;
  logic [3:0] v [3:0];
  logic clk = 0;
  always_ff @(posedge clk) v[2:1] <= '{4'ha, 4'h6};
  initial begin #1 clk = 1; #1 $display("v2=%h v1=%h", v[2], v[1]); $finish; end
endmodule
"#,
        &["v2=a v1=6"],
    );
}

/// sv2v = verilator (the old values print before the update). Cell `c28_19_nba_initial`.
#[test]
fn z12_nonblocking_in_initial() {
    pin(
        r#"module top;
  logic [3:0] v [3:0];
  initial begin
    v[3]=4'h1; v[2]=4'h2; v[1]=4'h4; v[0]=4'h8;
    v[2:1] <= '{4'ha, 4'h6};
    $display("pre v2=%h v1=%h", v[2], v[1]);
    #1 $display("v3=%h v2=%h v1=%h v0=%h", v[3], v[2], v[1], v[0]);
  end
endmodule
"#,
        &["pre v2=2 v1=4", "v3=1 v2=a v1=6 v0=8"],
    );
}

/// sv2v = verilator: every element lands at t3. Cell `c28_20_nba_delay`.
#[test]
fn z13_nonblocking_with_delay() {
    pin(
        r#"module top;
  logic [3:0] v [3:0];
  initial begin
    v[3]=4'h1; v[2]=4'h2; v[1]=4'h4; v[0]=4'h8;
    v[2:1] <= #2 '{4'ha, 4'h6};
    #1 $display("t1 v2=%h v1=%h", v[2], v[1]);
    #2 $display("t3 v2=%h v1=%h", v[2], v[1]);
  end
endmodule
"#,
        &["t1 v2=2 v1=4", "t3 v2=a v1=6"],
    );
}

/// sv2v = verilator. Cell `c28_22_task`.
#[test]
fn z14_static_task() {
    pin(
        r#"module top;
  logic [3:0] v [3:0];
  task t(input logic [3:0] a, input logic [3:0] b);
    v[2:1] = '{a, b};
  endtask
  initial begin
    v[3]=4'h1; v[2]=4'h2; v[1]=4'h4; v[0]=4'h8;
    t(4'ha, 4'h6);
    $display("v3=%h v2=%h v1=%h v0=%h", v[3], v[2], v[1], v[0]);
  end
endmodule
"#,
        &["v3=1 v2=a v1=6 v0=8"],
    );
}

/// sv2v = verilator. Cell `c28_23_always_comb`.
#[test]
fn z15_always_comb() {
    pin(
        r#"module top;
  logic [3:0] a = 4'ha, b = 4'h6;
  logic [3:0] v [3:0];
  always_comb v[2:1] = '{a, b};
  initial begin #1 $display("v2=%h v1=%h", v[2], v[1]); a = 4'h3; #1 $display("v2=%h v1=%h", v[2], v[1]); end
endmodule
"#,
        &["v2=a v1=6", "v2=3 v1=6"],
    );
}

/// `m[1][1:0]`: sv2v = verilator. Cell `c28_27_multidim`.
#[test]
fn z16_slice_after_an_index() {
    pin(
        r#"module top;
  logic [3:0] m [1:0][2:0];
  initial begin
    m[1][2]=4'h1; m[1][1]=4'h2; m[1][0]=4'h4; m[0][2]=4'h5; m[0][1]=4'h6; m[0][0]=4'h7;
    m[1][1:0] = '{4'ha, 4'hc};
    $display("%h %h %h %h %h %h", m[1][2], m[1][1], m[1][0], m[0][2], m[0][1], m[0][0]);
  end
endmodule
"#,
        &["1 a c 5 6 7"],
    );
}

/// sv2v = verilator. Cell `c28_28_swap_nba`.
#[test]
fn z17_nonblocking_swap_reads_old_values() {
    pin(
        r#"module top;
  logic [3:0] v [1:0];
  initial begin
    v[1]=4'h3; v[0]=4'h5;
    v[1:0] <= '{v[0], v[1]};
    #1 $display("v1=%h v0=%h", v[1], v[0]);
  end
endmodule
"#,
        &["v1=5 v0=3"],
    );
}

/// `v[1+:2]`: sv2v = verilator. Cell `c28_47_const_up_desc`.
#[test]
fn z18_constant_indexed_slice() {
    pin(
        r#"module top;
  logic [3:0] v [3:0];
  initial begin
    v[3]=4'h1; v[2]=4'h2; v[1]=4'h4; v[0]=4'h8;
    v[1+:2] = '{4'ha, 4'h6};
    $display("v3=%h v2=%h v1=%h v0=%h", v[3], v[2], v[1], v[0]);
  end
endmodule
"#,
        &["v3=1 v2=a v1=6 v0=8"],
    );
}

/// a nested pattern per element: sv2v = verilator. Cell `c28_48_outer_slice_2d`.
#[test]
fn z19_outer_slice_of_a_2d_array() {
    pin(
        r#"module top;
  logic [3:0] m [2:0][1:0];
  initial begin
    m[2][1]=4'h1; m[2][0]=4'h2; m[1][1]=4'h3; m[1][0]=4'h4; m[0][1]=4'h5; m[0][0]=4'h6;
    m[1:0] = '{'{4'ha, 4'hb}, '{4'hc, 4'hd}};
    $display("%h %h %h %h %h %h", m[2][1], m[2][0], m[1][1], m[1][0], m[0][1], m[0][0]);
  end
endmodule
"#,
        &["1 2 a b c d"],
    );
}

/// `[1:0]` of `v[0:1]`: sv2v and verilator assign in declaration order (`v0=3 v1=5`), against the slice's left index — not settled. Cell `c28_06_ascdecl_descslice`.
#[test]
fn l01_direction_opposite_to_ascending_declaration() {
    loud(
        r#"module top;
  logic [3:0] v [0:1];
  initial begin
    v[1:0] = '{4'h3, 4'h5};
    $display("v0=%h v1=%h", v[0], v[1]);
  end
endmodule
"#,
        "an unpacked-array slice whose direction is opposite",
    );
}

/// verilator refuses, sv2v ignores the direction. Cell `c28_07_descdecl_ascslice`.
#[test]
fn l02_direction_opposite_to_descending_declaration() {
    loud(
        r#"module top;
  logic [3:0] v [1:0];
  initial begin
    v[0:1] = '{4'h3, 4'h5};
    $display("v1=%h v0=%h", v[1], v[0]);
  end
endmodule
"#,
        "an unpacked-array slice whose direction is opposite",
    );
}

/// verilator refuses; sv2v writes garbage. Cell `c28_16_count_short`.
#[test]
fn l03_element_count_mismatch() {
    loud(
        r#"module top;
  logic [3:0] v [3:0];
  initial begin
    v[3]=4'h1; v[2]=4'h2; v[1]=4'h4; v[0]=4'h8;
    v[2:0] = '{4'ha, 4'h6};
    $display("v3=%h v2=%h v1=%h v0=%h", v[3], v[2], v[1], v[0]);
  end
endmodule
"#,
        "assignment pattern has 2 element(s)",
    );
}

/// verilator: unsupported and an internal error; one oracle. Cell `c28_24_var_up_desc`.
#[test]
fn l04_variable_base() {
    loud(
        r#"module top;
  logic [3:0] v [3:0];
  int i;
  initial begin
    v[3]=4'h1; v[2]=4'h2; v[1]=4'h4; v[0]=4'h8; i = 1;
    v[i+:2] = '{4'ha, 4'h6};
    $display("v3=%h v2=%h v1=%h v0=%h", v[3], v[2], v[1], v[0]);
  end
endmodule
"#,
        "base or width is not a single constant",
    );
}

/// a blocking self-read: sv2v `5 3`, verilator `5 5`. Cell `c28_29_swap_ba`.
#[test]
fn l05_blocking_pattern_reads_the_target() {
    loud(
        r#"module top;
  logic [3:0] v [1:0];
  initial begin
    v[1]=4'h3; v[0]=4'h5;
    v[1:0] = '{v[0], v[1]};
    $display("v1=%h v0=%h", v[1], v[0]);
  end
endmodule
"#,
        "reads the array it writes",
    );
}

/// an array-slice copy: a separate gap (sv2v = verilator `1 2 a b`). Cell `c28_31_slicecopy`.
#[test]
fn l06_slice_copy() {
    loud(
        r#"module top;
  logic [3:0] v [3:0];
  logic [3:0] w [3:0];
  initial begin
    v[3]=4'h1; v[2]=4'h2; v[1]=4'h4; v[0]=4'h8;
    w[3]=4'ha; w[2]=4'hb; w[1]=4'hc; w[0]=4'hd;
    v[1:0] = w[3:2];
    $display("v3=%h v2=%h v1=%h v0=%h", v[3], v[2], v[1], v[0]);
  end
endmodule
"#,
        "a slice copy or a packed value is not",
    );
}

/// PRE wrote element 0 (`v1=0 v0=3`) at exit 0; verilator and iverilog refuse. Cell `c28_35_plain_rhs_slice`.
#[test]
fn l07_packed_value_into_a_slice() {
    loud(
        r#"module top;
  logic [3:0] v [1:0];
  initial begin
    v[1] = 4'h0; v[0] = 4'h0;
    v[1:0] = 2'b11;
    $display("v1=%h v0=%h", v[1], v[0]);
  end
endmodule
"#,
        "a slice copy or a packed value is not",
    );
}

/// PRE wrote element 0 (`1 2 4 c`) at exit 0; verilator and iverilog refuse. Cell `c28_36_plain_rhs_slice_nba`.
#[test]
fn l08_packed_value_into_a_slice_nonblocking() {
    loud(
        r#"module top;
  logic [3:0] v [3:0];
  logic [1:0] x = 2'b10;
  initial begin
    v[3]=4'h1; v[2]=4'h2; v[1]=4'h4; v[0]=4'h8;
    v[2:1] <= x;
    #1 $display("v3=%h v2=%h v1=%h v0=%h", v[3], v[2], v[1], v[0]);
  end
endmodule
"#,
        "a slice copy or a packed value is not",
    );
}

/// PRE wrote element 0 (`1 2 4 e`) at exit 0. Cell `c28_38_plain_rhs_idxpart`.
#[test]
fn l09_packed_value_into_a_variable_slice() {
    loud(
        r#"module top;
  logic [3:0] v [3:0];
  int i;
  initial begin
    v[3]=4'h1; v[2]=4'h2; v[1]=4'h4; v[0]=4'h8; i = 1;
    v[i+:2] = 2'b11;
    $display("v3=%h v2=%h v1=%h v0=%h", v[3], v[2], v[1], v[0]);
  end
endmodule
"#,
        "base or width is not a single constant",
    );
}

/// PRE drove element 0 (`v0=Z`) at exit 0; verilator and iverilog refuse. Cell `c28_39_plain_rhs_cont`.
#[test]
fn l10_continuous_packed_value_into_a_slice() {
    loud(
        r#"module top;
  logic [1:0] x = 2'b10;
  wire [3:0] v [3:0];
  assign v[2:1] = x;
  initial begin #1 $display("v2=%h v1=%h v0=%h", v[2], v[1], v[0]); end
endmodule
"#,
        "selects array elements (a slice)",
    );
}

/// a continuous slice would need the sole-writer whole-array machinery. Cell `c28_33_cont`.
#[test]
fn l11_continuous_pattern_into_a_slice() {
    loud(
        r#"module top;
  logic [3:0] a = 4'ha, b = 4'h6;
  wire [3:0] v [3:0];
  assign v[2:1] = '{a, b};
  initial begin #1 $display("v2=%h v1=%h", v[2], v[1]); end
endmodule
"#,
        "selects array elements (a slice)",
    );
}

/// a non-ANSI `output logic [7:0] o [3:0]` recorded no declared direction, so the slice took ascending order (`o2=b2 o1=a1`) at exit 0 (review round 1); sv2v = verilator; iverilog refuses the slice. Cell `L28S_nansi4`.
#[test]
fn r01_non_ansi_port_slice_keeps_its_direction() {
    pin(
        r#"module top(o);
  output logic [7:0] o [3:0];
  initial begin
    o[3] = 8'h0; o[2] = 8'h0; o[1] = 8'h0; o[0] = 8'h0;
    o[1+:2] = '{8'hA1, 8'hB2};
    $display("o3=%h o2=%h o1=%h o0=%h", o[3], o[2], o[1], o[0]);
  end
endmodule
"#,
        &["o3=00 o2=a1 o1=b2 o0=00"],
    );
}

/// the same missing direction reversed the WHOLE-array pattern before this slice (`o3=04`); iverilog = sv2v = verilator. Cell `L28S_nansi3`.
#[test]
fn r02_non_ansi_port_whole_array_order() {
    pin(
        r#"module top(o);
  output logic [7:0] o [3:0];
  initial begin
    o = '{8'h1, 8'h2, 8'h3, 8'h4};
    $display("whole o3=%h o2=%h o1=%h o0=%h", o[3], o[2], o[1], o[0]);
  end
endmodule
"#,
        &["whole o3=01 o2=02 o1=03 o0=04"],
    );
}

/// `x[2:1] = '{8'h05, 8'(x[2])}`: the guard missed the read behind the cast and took verilator's `05 05` where sv2v reads the old `03` (review round 1). Cell `L28_gb_cast`.
#[test]
fn r03_self_read_behind_a_cast_stays_loud() {
    loud(
        r#"module top;
  typedef struct packed { logic [3:0] a; logic [3:0] b; } s_t;
  logic [7:0] x[2:0];
  s_t s[2:0];
  initial begin
    x[2] = 8'h03; x[1] = 8'h04; x[0] = 8'h06;
    s[2] = 8'h03; s[1] = 8'h04; s[0] = 8'h06;
    x[2:1] = '{8'h05, 8'(x[2])};
    $display("cast %h %h %h | %h %h %h", x[2], x[1], x[0], s[2], s[1], s[0]);
  end
endmodule
"#,
        "reads the array it writes",
    );
}

/// `top.x[2]` names the target too. Cell `L28_hierread`.
#[test]
fn r04_self_read_through_a_hierarchical_name_stays_loud() {
    loud(
        r#"module top;
  logic [7:0] x[2:0];
  initial begin
    x[2] = 8'h03; x[1] = 8'h04; x[0] = 8'h06;
    x[2:1] = '{8'h05, top.x[2]};
    $display("hier %h %h %h", x[2], x[1], x[0]);
  end
endmodule
"#,
        "reads the array it writes",
    );
}

/// `{v[1:0], x} = …` and `assign w[1:0] = y` on `logic [1:0][3:0] v [3:0]` wrote element 0's packed dims at exit 0 (PRE too); all three oracles refuse. Cell `L28S_pk`.
#[test]
fn r05_slice_of_an_array_of_multi_dim_packed_elements() {
    loud(
        r#"module top;
  logic [1:0][3:0] v [3:0];
  logic [1:0][3:0] u [3:0];
  logic [3:0] x;
  initial begin
    for (int k = 0; k < 4; k++) begin v[k] = 8'h00; u[k] = 8'h00; end
    {v[1:0], x} = 12'hABC;
    {u[0+:2], x} = 12'hDEF;
    $display("v %h %h %h %h  u %h %h %h %h  x %h", v[3], v[2], v[1], v[0], u[3], u[2], u[1], u[0], x);
  end
endmodule
module top2;
  logic [1:0][3:0] w [3:0];
  logic [7:0] y = 8'h5A;
  assign w[1:0] = y;
  initial #1 $display("w %h %h %h %h", w[3], w[2], w[1], w[0]);
endmodule
"#,
        "selects array elements (a slice)",
    );
}

/// A zero-formal `let` lowers to its body, so `let L = x[2]` read the target past the self-read guard: POST took verilator's `05 05` where the inlined spelling is loud (review round 2). Cell `D28_let`.
#[test]
fn r06_self_read_through_a_let_stays_loud() {
    loud(
        r#"module top;
  logic [7:0] x[2:0];
  let L = x[2];
  initial begin
    x[2] = 8'h03; x[1] = 8'h04; x[0] = 8'h06;
    x[2:1] = '{8'h05, L};
    $display("let %h %h %h", x[2], x[1], x[0]);
  end
endmodule
"#,
        "reads the array it writes",
    );
}

/// The same `let` door in the leading index: `m[LI][1:0]` re-read `LI` after the first element write and landed the second in row 0 (review round 2). Cell `D28_let_lead2`.
#[test]
fn r07_leading_index_through_a_let_stays_loud() {
    loud(
        r#"module top;
  logic [7:0] m[1:0][2:0];
  let LI = m[1][1][0];
  initial begin
    m[0][0] = 8'h00; m[0][1] = 8'h11; m[0][2] = 8'h22;
    m[1][0] = 8'h33; m[1][1] = 8'h01; m[1][2] = 8'h55;
    m[LI][1:0] = '{8'hA0, 8'hA1};
    $display("lead %h %h %h | %h %h %h", m[1][2], m[1][1], m[1][0], m[0][2], m[0][1], m[0][0]);
  end
endmodule
"#,
        "reads the array it writes",
    );
}

/// A task `output` formal named like a module array of multi-dimensional packed elements: PRE refused its writes; declining the packed lane wrote the module's `v[0]` at exit 0 (`r xx v0 ab`, all three oracles `r ab v0 00`). PRE's refusal stands (review round 2). Cell `D28_of5`.
#[test]
fn r08_output_formal_named_like_a_module_array_stays_loud() {
    loud(
        r#"module top;
  logic [1:0][3:0] v [3:0];
  logic [7:0] r;  logic [3:0] x;
  task t(output logic [1:0][3:0] v);
    v[1] = 4'hA; v[0] = 4'hB;
  endtask
  initial begin
    for (int k = 0; k < 4; k++) v[k] = 8'h00;
    x = 4'h0; t(r);
    $display("r %h v0 %h v1 %h x %h", r, v[0], v[1], x);
  end
endmodule
"#,
        "part-select range exceeds the declared bounds",
    );
}

/// An `input` formal named like a module array: the slice lane resolved the array and wrote the module's `v[1:0]` (`r f0 v1 01`, sv2v = verilator `r f2 v1 00`); declined, PRE's refusal stands (review round 2).
#[test]
fn r09_input_formal_named_like_a_module_array_stays_loud() {
    loud(
        r#"module top;
  logic [7:0] v [3:0];
  logic [7:0] r;
  task t(input logic [7:0] v);
    v[1:0] = '{1'b1, 1'b0};
    r = v;
  endtask
  initial begin
    for (int k = 0; k < 4; k++) v[k] = 8'h00;
    t(8'hF0);
    $display("r %h v0 %h v1 %h", r, v[0], v[1]);
  end
endmodule
"#,
        "supported only as the whole right-hand side",
    );
}

/// An inline task's input formal is a snapshot taken at the call, so it reads nothing the slice writes: sv2v = verilator `in 05 03 06` (iverilog refuses the slice). Cell `D28_inl_in`.
#[test]
fn r10_inline_task_input_snapshot_is_not_a_self_read() {
    pin(
        r#"module top;
  logic [7:0] x[2:0];
  task t(input logic [7:0] v);
    x[2:1] = '{8'h05, v};
  endtask
  initial begin
    x[2] = 8'h03; x[1] = 8'h04; x[0] = 8'h06;
    t(x[2]);
    $display("in %h %h %h", x[2], x[1], x[0]);
  end
endmodule
"#,
        &["in 05 03 06"],
    );
}

/// An arithmetic bound is not folded (v1): sv2v = verilator `00 00 00 00 a1 a0 00 00`. Cell `D28_cbx2`.
#[test]
fn r11_arithmetic_bound_is_loud() {
    loud(
        r#"module top;
  logic [7:0] v [7:0];
  initial begin
    for (int k = 0; k < 8; k++) v[k] = 8'h00;
    v[1+2:2] = '{8'hA1, 8'hA0};
    #1 $display("%h %h %h %h %h %h %h %h", v[7], v[6], v[5], v[4], v[3], v[2], v[1], v[0]);
  end
endmodule
"#,
        "is not a single constant",
    );
}
