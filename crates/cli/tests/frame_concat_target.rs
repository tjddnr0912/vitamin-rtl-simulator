//! §5.2 row 22 (§4.5.621): a concatenation target `{a, b} = e;` in a FRAME body — a function or
//! task that runs in its own activation window — pinned on native, vm and interp. It lowers to a
//! frame-local capture `tmp = e` plus one single-chunk write per target, MSB first
//! (`frames_concat.rs`). It was E3009 (`body uses a concatenation-target assignment, which is
//! outside the frame-call subset`), and a suspendable task that lifted past that refusal wrote it
//! through the flat store at exit 0.
//!
//! Oracles: Icarus Verilog 13.0 `-g2012` (iv), sv2v 0.0.13 -> iverilog (s2v), Verilator 5.050
//! `--binary` (vl). Each cell names who agrees; where the oracles split, the frame lane is pinned
//! to vita's own module lane (an internal-equivalence pin, ER §7.3). The cells are the row-22
//! census's, byte for byte.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_on(src: &str, backend: &str) -> (Vec<String>, String, i32) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_fct_{backend}_{}_{n}", std::process::id()));
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

/// the OpenTitan `tlul_pkg::get_cmd_intg` shape (row-9 R09): an automatic function whose locals take a concatenation. iverilog = sv2v = verilator. Cell `c22_01_wA`.
#[test]
fn z01_witness_opentitan_get_cmd_intg() {
    pin(
        r#"module top;
  function automatic logic [3:0] f(logic [7:0] x);
    logic [3:0] hi; logic [3:0] lo;
    {hi, lo} = x + 8'h11;
    return hi ^ lo;
  endfunction
  logic [3:0] o; logic [7:0] a;
  assign o = f(a);
  initial begin a = 8'h5A; #1 $display("A o=%h", o); $finish; end
endmodule
"#,
        &["A o=d"],
    );
}

/// the VeeR EL2 / EH1 testbench `dasm16_ciw` shape (row-9 O11): part- and bit-selects of an `int` local, a string return. iverilog = verilator. Cell `c22_02_wB`.
#[test]
fn z02_witness_veer_dasm16_ciw() {
    pin(
        r#"module top;
  function automatic string f(input [31:0] op);
    int imm;
    imm = 0;
    {imm[5:4], imm[9:6], imm[2], imm[3]} = op[12:5];
    return $sformatf("imm=%0h", imm);
  endfunction
  string s;
  initial begin s = f(32'h0000_1FE0); $display("%s", s); $finish; end
endmodule
"#,
        &["imm=3fc"],
    );
}

/// a static function framed for its loop. iverilog = sv2v = verilator. Cell `c22_05_static_loop`.
#[test]
fn z03_static_function_with_a_loop() {
    pin(
        r#"module top;
  function [7:0] f(input [7:0] x);
    reg [3:0] hi, lo; integer i;
    begin f = 0; for (i = 0; i < 2; i = i + 1) begin {hi, lo} = x + i; f = f + {lo, hi}; end end
  endfunction
  initial begin $display("L o=%h", f(8'h5A)); $finish; end
endmodule
"#,
        &["L o=5a"],
    );
}

/// a right-hand side narrower than the targets. iverilog = sv2v = verilator. Cell `c22_06_rhs_narrow_unsigned`.
#[test]
fn z04_narrow_unsigned_rhs_zero_extends() {
    pin(
        r#"module top;
  function automatic logic [7:0] f(logic [3:0] x);
    logic [3:0] a, b;
    a = 4'h7; b = 4'h7;
    {a, b} = x;
    return {a, b};
  endfunction
  initial begin $display("NU o=%h", f(4'hC)); $finish; end
endmodule
"#,
        &["NU o=0c"],
    );
}

/// a narrower SIGNED right-hand side extends with its sign. iverilog = sv2v = verilator. Cell `c22_07_rhs_narrow_signed`.
#[test]
fn z05_narrow_signed_rhs_sign_extends() {
    pin(
        r#"module top;
  function automatic logic [7:0] f(logic signed [3:0] x);
    logic [3:0] a, b;
    {a, b} = x;
    return {a, b};
  endfunction
  initial begin $display("NS o=%h", f(-4'sd2)); $finish; end
endmodule
"#,
        &["NS o=fe"],
    );
}

/// a right-hand side wider than the targets keeps its low bits. iverilog = sv2v = verilator. Cell `c22_08_rhs_wide`.
#[test]
fn z06_wide_rhs_truncates() {
    pin(
        r#"module top;
  function automatic logic [7:0] f(logic [11:0] x);
    logic [3:0] a, b;
    {a, b} = x;
    return {a, b};
  endfunction
  initial begin $display("W o=%h", f(12'hABC)); $finish; end
endmodule
"#,
        &["W o=bc"],
    );
}

/// `{c, s} = x + y` adds in the targets' width. iverilog = sv2v = verilator. Cell `c22_09_ctx_carry`.
#[test]
fn z07_carry_out_in_the_targets_context() {
    pin(
        r#"module top;
  function automatic logic [8:0] f(logic [7:0] x, logic [7:0] y);
    logic c; logic [7:0] s;
    {c, s} = x + y;
    return {c, s};
  endfunction
  initial begin $display("K o=%h", f(8'hFF, 8'h03)); $finish; end
endmodule
"#,
        &["K o=102"],
    );
}

/// the function's own name as a target. iverilog = sv2v = verilator. Cell `c22_11_retvar`.
#[test]
fn z08_the_return_variable_as_a_target() {
    pin(
        r#"module top;
  function automatic [3:0] f(input [7:0] x);
    logic [3:0] lo;
    {f, lo} = x;
    f = f ^ lo;
  endfunction
  initial begin $display("RV o=%h", f(8'h5A)); $finish; end
endmodule
"#,
        &["RV o=f"],
    );
}

/// a variable bit-select target. iverilog = sv2v = verilator. Cell `c22_12_bitsel_var`.
#[test]
fn z09_variable_bit_select_target() {
    pin(
        r#"module top;
  function automatic logic [7:0] f(input int i, input [3:0] x);
    logic [7:0] loc; logic [2:0] b;
    loc = 8'h00;
    {loc[i], b} = x;
    return {loc[7:3], b};
  endfunction
  initial begin $display("BV o=%h", f(6, 4'hD)); $finish; end
endmodule
"#,
        &["BV o=45"],
    );
}

/// a variable `+:` target. iverilog = sv2v = verilator. Cell `c22_13_idxpart_var`.
#[test]
fn z10_variable_indexed_part_select_target() {
    pin(
        r#"module top;
  function automatic logic [15:0] f(input int i, input [7:0] x);
    logic [11:0] loc; logic [3:0] b;
    loc = 12'h000;
    {loc[i +: 4], b} = x;
    return {loc, b};
  endfunction
  initial begin $display("IP o=%h", f(5, 8'hC3)); $finish; end
endmodule
"#,
        &["IP o=1803"],
    );
}

/// an element of a local unpacked array. iverilog = sv2v = verilator. Cell `c22_14_array_elem`.
#[test]
fn z11_local_array_element_target() {
    pin(
        r#"module top;
  function automatic logic [7:0] f(input [7:0] x);
    logic [3:0] arr [2]; logic [3:0] b;
    arr[0] = 0; arr[1] = 0;
    {arr[1], b} = x;
    return {arr[1], b};
  endfunction
  initial begin $display("AE o=%h", f(8'h96)); $finish; end
endmodule
"#,
        &["AE o=96"],
    );
}

/// a packed-struct member. iverilog = sv2v = verilator. Cell `c22_15_struct_member`.
#[test]
fn z12_packed_struct_member_target() {
    pin(
        r#"module top;
  typedef struct packed { logic [3:0] a; logic [3:0] b; } s_t;
  function automatic logic [7:0] f(input [7:0] x);
    s_t s; logic [1:0] c;
    s = '0;
    {s.b, c} = x[5:0];
    return s;
  endfunction
  initial begin $display("SM o=%h", f(8'h37)); $finish; end
endmodule
"#,
        &["SM o=0d"],
    );
}

/// an automatic task in the `&self` subset. iverilog = sv2v = verilator. Cell `c22_19_task_auto`.
#[test]
fn z13_automatic_task() {
    pin(
        r#"module top;
  task automatic t(input [7:0] x, output [7:0] y);
    logic [3:0] a, b;
    {a, b} = x + 8'h11;
    y = {b, a};
  endtask
  logic [7:0] r;
  initial begin t(8'h5A, r); $display("TA r=%h", r); $finish; end
endmodule
"#,
        &["TA r=b6"],
    );
}

/// an automatic task with a delay: PRE lifted it past the refusal and wrote `r=21` at exit 0. iverilog = sv2v = verilator. Cell `c22_20_task_suspend`.
#[test]
fn z14_suspending_task() {
    pin(
        r#"module top;
  task automatic t(input [7:0] x, output [7:0] y);
    logic [3:0] a, b;
    a = 4'h1; b = 4'h2;
    #1;
    {a, b} = x + 8'h11;
    y = {b, a};
  endtask
  logic [7:0] r;
  initial begin t(8'h5A, r); $display("TS r=%h t=%0t", r, $time); $finish; end
endmodule
"#,
        &["TS r=b6 t=1"],
    );
}

/// a nonblocking concatenation to module nets from a task. iverilog = sv2v = verilator. Cell `c22_21_task_nba_module`.
#[test]
fn z15_task_nonblocking_to_module_nets() {
    pin(
        r#"module top;
  logic [3:0] m1, m2;
  task automatic t(input [7:0] x);
    {m1, m2} <= x;
    #1;
  endtask
  initial begin m1 = 0; m2 = 0; t(8'h5A); $display("TN m1=%h m2=%h", m1, m2); $finish; end
endmodule
"#,
        &["TN m1=5 m2=a"],
    );
}

/// each activation captures into its own slot. iverilog = sv2v = verilator. Cell `c22_23_recursion`.
#[test]
fn z16_recursion() {
    pin(
        r#"module top;
  function automatic logic [7:0] f(input [7:0] x, input int n);
    logic [3:0] a, b;
    if (n == 0) return x;
    {a, b} = x;
    return f({b, a} + 8'h01, n - 1);
  endfunction
  initial begin $display("RC o=%h", f(8'h5A, 3)); $finish; end
endmodule
"#,
        &["RC o=b7"],
    );
}

/// `{a, b} = {b, a}`: the capture evaluates the right-hand side once. iverilog = sv2v = verilator. Cell `c22_24_swap_selfread`.
#[test]
fn z17_swap_reads_the_old_values() {
    pin(
        r#"module top;
  function automatic logic [7:0] f(input [7:0] x);
    logic [3:0] a, b;
    {a, b} = x;
    {a, b} = {b, a};
    return {a, b};
  endfunction
  initial begin $display("SW o=%h", f(8'h1E)); $finish; end
endmodule
"#,
        &["SW o=e1"],
    );
}

/// 36 + 64 bits. iverilog = sv2v = verilator. Cell `c22_27_wide`.
#[test]
fn z18_a_100_bit_concatenation() {
    pin(
        r#"module top;
  function automatic logic [99:0] f(input logic [99:0] x);
    logic [35:0] a; logic [63:0] b;
    {a, b} = x;
    return {b[3:0], a, b[63:4]};
  endfunction
  initial begin $display("WD o=%h", f(100'hF_EDCB_A987_6543_2101_2345_6789)); $finish; end
endmodule
"#,
        &["WD o=9fedcba987654321012345678"],
    );
}

/// called from `always_comb`. iverilog = sv2v = verilator. Cell `c22_28_always_comb`.
#[test]
fn z19_called_from_always_comb() {
    pin(
        r#"module top;
  function automatic logic [3:0] f(logic [7:0] x);
    logic [3:0] hi, lo;
    {hi, lo} = x;
    return hi - lo;
  endfunction
  logic [7:0] a; logic [3:0] o;
  always_comb o = f(a);
  initial begin a = 8'h93; #1 $display("AC o=%h", o); a = 8'h2C; #1 $display("AC o=%h", o); $finish; end
endmodule
"#,
        &["AC o=6", "AC o=6"],
    );
}

/// an `int` return. iverilog = sv2v = verilator. Cell `c22_29_static_fn_framed_ret2`.
#[test]
fn z20_two_state_return() {
    pin(
        r#"module top;
  function int f(input [7:0] x);
    logic [3:0] hi, lo;
    {hi, lo} = x;
    return hi * 16'd10 + lo;
  endfunction
  initial begin $display("S2 o=%0d", f(8'h5A)); $finish; end
endmodule
"#,
        &["S2 o=60"],
    );
}

/// a static task. iverilog = sv2v = verilator. Cell `c22_30_static_task`.
#[test]
fn z21_static_task() {
    pin(
        r#"module top;
  task t(input [7:0] x, output [7:0] y);
    reg [3:0] a, b;
    begin {a, b} = x; y = {b, a}; end
  endtask
  logic [7:0] r;
  initial begin t(8'h5A, r); $display("ST r=%h", r); $finish; end
endmodule
"#,
        &["ST r=a5"],
    );
}

/// signed targets. iverilog = sv2v = verilator. Cell `c22_31_signed_targets`.
#[test]
fn z22_signed_targets() {
    pin(
        r#"module top;
  function automatic int f(input [7:0] x);
    logic signed [3:0] a; int b;
    {a, b[3:0]} = x;
    return a;
  endfunction
  initial begin $display("SG o=%0d", f(8'hE5)); $finish; end
endmodule
"#,
        &["SG o=-2"],
    );
}

/// an `inout` formal. sv2v = verilator. Cell `c22_32_inout_formal`.
#[test]
fn z23_inout_formal_target() {
    pin(
        r#"module top;
  function automatic void f(input [3:0] x, inout [7:0] y);
    logic [3:0] lo;
    {y[7:4], lo} = {y[3:0], x};
    y[3:0] = lo;
  endfunction
  logic [7:0] r;
  initial begin r = 8'h3C; f(4'h9, r); $display("IO r=%h", r); $finish; end
endmodule
"#,
        &["IO r=c9"],
    );
}

/// `output` formals. verilator. Cell `c22_10_out_formal`.
#[test]
fn z24_output_formals_as_targets() {
    pin(
        r#"module top;
  function automatic int f(input [7:0] x, output [3:0] a, output [3:0] b);
    {a, b} = x ^ 8'h0F;
    return 1;
  endfunction
  logic [3:0] p, q; int r;
  initial begin r = f(8'h3A, p, q); $display("OF p=%h q=%h r=%0d", p, q, r); $finish; end
endmodule
"#,
        &["OF p=3 q=5 r=1"],
    );
}

/// an automatic task that leaves the `&self` subset for a `$display`: PRE `a=1 b=2 / r=21` at exit 0. iverilog = sv2v = verilator. Cell `c22_35_task_display`.
#[test]
fn z25_task_with_a_display() {
    pin(
        r#"module top;
  task automatic t(input [7:0] x, output [7:0] y);
    logic [3:0] a, b;
    a = 4'h1; b = 4'h2;
    {a, b} = x + 8'h11;
    $display("in a=%h b=%h", a, b);
    y = {b, a};
  endtask
  logic [7:0] r;
  initial begin t(8'h5A, r); $display("TD r=%h", r); $finish; end
endmodule
"#,
        &["in a=6 b=b", "TD r=b6"],
    );
}

/// a module net and a local in one concatenation from a suspending task: PRE dropped the local (`r=1`). iverilog = sv2v = verilator. Cell `c22_36_task_suspend_mixed`.
#[test]
fn z26_task_mixing_a_module_net_and_a_local() {
    pin(
        r#"module top;
  logic [3:0] m;
  task automatic t(input [7:0] x, output [3:0] y);
    logic [3:0] a;
    a = 4'h1;
    #1;
    {m, a} = x;
    y = a;
  endtask
  logic [3:0] r;
  initial begin m = 0; t(8'h5A, r); $display("TM m=%h r=%h", m, r); $finish; end
endmodule
"#,
        &["TM m=5 r=a"],
    );
}

/// a static task with a delay. iverilog = sv2v = verilator. Cell `c22_41_static_task_suspend`.
#[test]
fn z27_static_suspending_task() {
    pin(
        r#"module top;
  task t(input [7:0] x, output [7:0] y);
    reg [3:0] a, b;
    begin a = 1; b = 2; #1; {a, b} = x + 8'h11; y = {b, a}; end
  endtask
  logic [7:0] r;
  initial begin t(8'h5A, r); $display("SS r=%h", r); $finish; end
endmodule
"#,
        &["SS r=b6"],
    );
}

/// `{a, {b, c}} = e`. iverilog = sv2v = verilator. Cell `c22_47_nested_concat_target`.
#[test]
fn z28_nested_concatenation_target() {
    pin(
        r#"module top;
  function automatic logic [11:0] f(input [11:0] x);
    logic [3:0] a, b, c;
    {a, {b, c}} = x;
    return {c, a, b};
  endfunction
  initial begin $display("NC o=%h", f(12'h1A7)); $finish; end
endmodule
"#,
        &["NC o=71a"],
    );
}

/// a package function called through `import p::*`. iverilog = sv2v = verilator. Cell `c22_48_pkg_import`.
#[test]
fn z29_imported_package_function() {
    pin(
        r#"package p;
  function automatic logic [7:0] f(input [7:0] x);
    logic [3:0] a, b;
    {a, b} = x + 8'h11;
    return {b, a};
  endfunction
endpackage
module top;
  import p::*;
  initial begin $display("PK o=%h", f(8'h5A)); $finish; end
endmodule
"#,
        &["PK o=b6"],
    );
}

/// overlapping targets `{loc[3:0], loc[1:0]}`: iverilog and sv2v `0b`, verilator and vita's module path `09` — an oracle split; the frame lane must not pick a side the module lane does not. Cells `c22_25_overlap` (frame) and `c22_37_module_overlap` (module twin, unchanged by this slice).
#[test]
fn e01_overlapping_targets_follow_the_module_path() {
    pin(
        r#"module top;
  function automatic logic [7:0] f(input [5:0] x);
    logic [7:0] loc;
    loc = 8'h00;
    {loc[3:0], loc[1:0]} = x;
    return loc;
  endfunction
  initial begin $display("OV o=%h", f(6'b101101)); $finish; end
endmodule
"#,
        &["OV o=09"],
    );
    pin(
        r#"module top;
  logic [7:0] loc;
  initial begin loc = 8'h00; {loc[3:0], loc[1:0]} = 6'b101101; $display("MO o=%h", loc); $finish; end
endmodule
"#,
        &["MO o=09"],
    );
}

/// x/z into a `logic` and a `bit` target: the three oracles give three answers; the frame lane prints what the module lane does. Cells `c22_26_xz` (frame) and `c22_38_module_xz` (module twin, unchanged by this slice).
#[test]
fn e02_x_and_z_follow_the_module_path() {
    pin(
        r#"module top;
  function automatic logic [15:0] f(input logic [7:0] x);
    logic [3:0] a; bit [3:0] b; logic [7:0] r;
    {a, b} = x;
    r = {a, b};
    return {r, 8'h00};
  endfunction
  initial begin $display("XZ o=%b", f(8'b1x0z_x1z0)); $finish; end
endmodule
"#,
        &["XZ o=1x0z010000000000"],
    );
    pin(
        r#"module top;
  logic [3:0] a; bit [3:0] b;
  initial begin {a, b} = 8'b1x0z_x1z0; $display("MX a=%b b=%b", a, b); $finish; end
endmodule
"#,
        &["MX a=1x0z b=0100"],
    );
}

/// `{i, loc[i]} = e`: sequential per-target writes would read the NEW `i`; vita's module path, iverilog and sv2v read every index first, verilator does not. Cell `c22_45_index_dep_frame`.
#[test]
fn l01_an_index_that_reads_another_target() {
    loud(
        r#"module top;
  function automatic logic [7:0] f(input int k);
    int i; logic [7:0] loc;
    i = k; loc = 0;
    {i, loc[i]} = {32'd5, 1'b1};
    return loc;
  endfunction
  initial begin $display("IF loc=%b", f(1)); $finish; end
endmodule
"#,
        "concatenation-target assignment",
    );
}

/// a string has no bit slice to take (iverilog aborts on the cell). Cell `c22_16_string_part`.
#[test]
fn l02_a_string_target() {
    loud(
        r#"module top;
  function automatic string f(input [15:0] x);
    string s; logic [7:0] b;
    {s, b} = x;
    return s;
  endfunction
  initial begin $display("ST o=%s", f(16'h4142)); $finish; end
endmodule
"#,
        "concatenation-target assignment",
    );
}

/// all three oracles reject a `real` in a concatenation target. Cell `c22_17_real_part`.
#[test]
fn l03_a_real_target() {
    loud(
        r#"module top;
  function automatic real f(input [7:0] x);
    real r; logic [3:0] b;
    {r, b} = x;
    return r;
  endfunction
  initial begin $display("RE o=%f", f(8'h5A)); $finish; end
endmodule
"#,
        "concatenation-target assignment",
    );
}

/// The OpenTitan path itself (row-9 R09, `uart_reg_top.sv:58`): `tlul_pkg::get_cmd_intg` —
/// reached through `import tlul_pkg::*`, and from another package function — splits
/// `{cmd_intg, unused_payload} = prim_secded_pkg::…_enc(…)`, the right-hand side a scoped call
/// into a second package. PRE: E3009 naming `get_cmd_intg`. sv2v = verilator (iverilog cannot
/// parse the cell's keyed struct pattern). Cell `c22_ot`.
#[test]
fn z30_opentitan_get_cmd_intg_through_two_packages() {
    pin(
        r#"package secded_pkg;
  function automatic logic [71:0] enc(logic [63:0] d);
    logic [71:0] r;
    r = {^d[31:0] ? 8'h5a : 8'ha5, d};
    return r;
  endfunction
endpackage
package tl_pkg;
  typedef struct packed { logic [3:0] op; logic [31:0] addr; logic [7:0] mask; } h2d_t;
  function automatic logic [6:0] get_cmd_intg(h2d_t tl);
    logic [6:0] cmd_intg;
    logic [64:0] unused_payload;
    {cmd_intg, unused_payload} = secded_pkg::enc(64'(tl));
    return cmd_intg;
  endfunction
  function automatic logic [6:0] get_bad_cmd_intg(h2d_t tl);
    logic [6:0] c;
    c = get_cmd_intg(tl);
    return ~c;
  endfunction
endpackage
module top import tl_pkg::*; ;
  h2d_t t;
  logic [6:0] a, b;
  assign a = get_cmd_intg(t);
  assign b = get_bad_cmd_intg(t);
  initial begin t = '{op: 4'h4, addr: 32'h1000_0004, mask: 8'hf0}; #1 $display("A a=%h b=%h", a, b); t.mask = 8'h0f; #1 $display("A a=%h b=%h", a, b); $finish; end
endmodule
"#,
        &["A a=2d b=52", "A a=2d b=52"],
    );
}

/// `{m, mv[top.m]}`: the split read the NEW `m` (verilator's `00001000`); declined, the module-net write keeps iverilog = sv2v = PRE (review round 1). Cell `L22_27_idx_nodelay`.
#[test]
fn r01_index_through_a_hierarchical_self_name_is_not_split() {
    pin(
        r#"module top;
  logic [3:0] m; logic [7:0] mv;
  task automatic t_nodelay(); {m, mv[top.m]} = 5'b0011_1; endtask
  initial begin m = 1; mv = 0; t_nodelay(); $display("A m=%0d mv=%b", m, mv); end
endmodule
"#,
        &["A m=3 mv=00000010"],
    );
}

/// `{m, mv[getm()]}`: the read hides in the callee; iverilog = sv2v = PRE. Cell `L22_03_call_index`.
#[test]
fn r02_index_through_a_call_is_not_split() {
    pin(
        r#"module top;
  logic [3:0] m;
  logic [7:0] mv;
  function automatic [3:0] getm(); return m; endfunction
  task automatic t();
    #1;
    {m, mv[getm()]} = 5'b0011_1;
  endtask
  initial begin m = 4'd1; mv = 0; t(); $display("CI m=%0d mv=%b", m, mv); end
endmodule
"#,
        &["CI m=3 mv=00000010"],
    );
}

/// iverilog = sv2v = verilator; PRE `r=0`. Cell `L22S_09b_lift_bare`.
#[test]
fn r03_lifted_task_mixing_a_module_net_and_a_local() {
    pin(
        r#"module top;
  logic [3:0] m;
  task automatic t(output [3:0] y);
    logic [3:0] loc;
    loc = 0;
    #1;
    {m, loc} = 8'hA5;
    y = loc;
  endtask
  logic [3:0] r;
  initial begin m = 0; t(r); $display("LHB m=%h r=%h", m, r); $finish; end
endmodule
"#,
        &["LHB m=a r=5"],
    );
}

/// `{a, b} = #1 x` in a suspending task printed `r=xx` at exit 0 (PRE too); the split takes no intra-assignment timing, so the lifting gate now refuses it (iverilog = sv2v `r=6b`). Cell `L22_17_intra_delay`.
#[test]
fn r04_intra_assignment_delay_in_a_lifted_task_is_loud() {
    loud(
        r#"module top;
  logic [7:0] r;
  task automatic t(input [7:0] x);
    logic [3:0] a, b;
    {a, b} = #1 x;
    r = {b, a};
  endtask
  initial begin t(8'hb6); $display("ID r=%h t=%0t", r, $time); end
endmodule
"#,
        "concatenation target over this task",
    );
}

/// `{v[i], i} = e` in a suspending task wrote `v=00` at exit 0 (PRE too; all three oracles `v=01`). Cell `L22S_01_lift_cotarget`.
#[test]
fn r05_cotarget_index_in_a_lifted_task_is_loud() {
    loud(
        r#"module top;
  task automatic t(output [7:0] y, output int yi);
    int i; logic [7:0] v;
    i = 0; v = 8'h00;
    #1;
    {v[i], i} = {1'b1, 32'd0};
    y = v; yi = i;
  endtask
  logic [7:0] r; int ri;
  initial begin t(r, ri); $display("LC v=%h i=%0d", r, ri); $finish; end
endmodule
"#,
        "concatenation target over this task",
    );
}

/// The two declines a lifted task cannot split — intra-assignment timing, and an index reading a module net — never wrote the local in PRE (`K1 m=b y=0`, `K2 n=d y=0` at exit 0; iverilog, sv2v and verilator `y=6`, `y=2`), so both are loud, naming the cause (review round 2). Cell `D22_21_lift_read_local`.
#[test]
fn r06_lifted_declines_name_their_cause() {
    loud(
        r#"module top;
  logic [3:0] m, n; logic [1:0] sel;
  task automatic t1(input [7:0] x, output [3:0] y);
    logic [3:0] junk;
    junk = 0;
    {m, junk} = #1 x;
    y = junk;
  endtask
  task automatic t2(input [7:0] x, output [3:0] y);
    logic [3:0] junk;
    junk = 0;
    #1;
    {n, junk[sel]} = x[4:0];
    y = junk;
  endtask
  logic [3:0] y1, y2;
  initial begin
    sel = 1; m = 0; n = 0;
    t1(8'hb6, y1); $display("K1 m=%h y=%h", m, y1);
    t2(8'h1b, y2); $display("K2 n=%h y=%h", n, y2);
  end
endmodule
"#,
        "an index that reads anything but this task's own locals",
    );
}
