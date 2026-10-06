//! §4.5.600: a continuous `assign` to a variable that is not `logic` — `reg`, `integer`,
//! `time`, `real`, `realtime`, `bit`, `byte`, `shortint`, `int`, `longint`, a 2-state packed
//! struct — is legal as the variable's sole writer.
//!
//! IEEE 1800-2017 §6.5: "variables can be written by one continuous assignment or one
//! port"; §10.3.2: "Variables can only be driven by one continuous assignment or by one
//! primitive output or module output. It shall be an error for a variable driven by a
//! continuous assignment or output to have an initializer in the declaration or any
//! procedural assignment."; §6.11.2: `logic` and `reg` name one type. vita refused every
//! such `assign` with E3018 ("continuous assign drives variable … (declare it
//! wire/logic)"), IEEE 1364's rule, since the first commit; `int`, `bit` and `time` reached
//! it because their storage is a `Reg` slot. iverilog 13.0 `-g2012` and verilator 5.052
//! run every design below that this file accepts; iverilog `-g2005` refuses the `reg`
//! one ("Variable 'y' cannot be driven by a continuous assignment/module." / "This is
//! allowed when SystemVerilog is enabled.").
//!
//! `elaborate/src/cont_var.rs` keeps the `assign` once every writer exists, where the
//! writer census the whole-array `assign` uses (`cont_array.rs`) finds no other writer, the
//! right-hand side calls no function, and the `assign`s drive all of the variable: the whole
//! variable, or every element of an unpacked array once at a constant index. Otherwise E3018
//! stays, worded for the reason, at the `assign`; a variable `multidriver.rs` already
//! reported as E3001 gets no E3018 on top (one diagnostic per variable).
//!
//! Every accepted design prints iverilog 13.0 `-g2012`'s output byte for byte (the one
//! `$typename` line, which iverilog cannot compile, is verilator's). verilator
//! 5.052 (`--binary --timing`) agrees except on a 4-state `x` (2-state: `x_integer_xpass`
//! `v=…0100 r=01000100 t=68`, `x_reg_delay` `t1 r=0`, the `x` element of
//! `elements_of_an_array_driven_once_each` `00 01`). sv2v 0.0.13 → iverilog agrees except
//! where sv2v drops the 2-state type (`x_bit_xcoerce` `t1 v=x vi=X vb=x1z0x1z0`,
//! `x_struct2s` `t2 d=x1x1`, the 2-state ports `uy=x uz=X`, the `int` elements `x X`), at
//! t0 (`x_reg_t0` `t0 r=x t1 r=x`: sv2v rewrites the `assign` as an `always @(*)` that
//! misses the t0 value), and on the concatenation target, which sv2v cannot translate
//! (`'lo' is not a valid l-value for a procedural assignment.`).
//!
//! The owner's ruling of 2026-10-06: the same `assign` to a `reg`, `integer`, `time`,
//! `real` or `realtime` in a file named `*.v` is accepted with the warning
//! `VITA-W3062` — legal in IEEE 1800, an error in IEEE 1364. It is vita's only
//! file-extension-aware behaviour.
//!
//! Every test counts the EXACT set of codes the run prints, under `-Wno-W1017`.

use std::collections::BTreeSet;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Run one design, written to `file`, through one-shot `vita` with `-Wno-W1017` and
/// return its exit code and combined output.
fn run_as(src: &str, file: &str) -> (Option<i32>, String) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_cavk_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("scratch dir");
    std::fs::write(d.join(file), src).expect("write design");
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("-Wno-W1017")
        .arg(file)
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (out.status.code(), s)
}

fn run(src: &str) -> (Option<i32>, String) {
    run_as(src, "t.sv")
}

/// Every diagnostic code the run printed (`[VITA-…]`), as a set.
fn codes(out: &str) -> BTreeSet<String> {
    let mut set = BTreeSet::new();
    for (i, _) in out.match_indices("[VITA-") {
        let rest = &out[i + 1..];
        if let Some(end) = rest.find(']') {
            set.insert(rest[..end].to_string());
        }
    }
    set
}

fn expect_codes(out: &str, want: &[&str], what: &str) {
    let want: BTreeSet<String> = want.iter().map(|c| c.to_string()).collect();
    assert_eq!(codes(out), want, "{what}: exact code set:\n{out}");
}

/// The design's own output lines: diagnostics and status lines dropped.
fn shown(out: &str) -> Vec<&str> {
    out.lines()
        .filter(|l| {
            !l.contains("[VITA-") && !l.starts_with("simulation ended") && !l.starts_with("errors=")
        })
        .collect()
}

/// A clean run printing exactly `want` — iverilog 13.0 `-g2012`'s lines.
fn accepts(src: &str, want: &[&str], what: &str) {
    let (rc, out) = run(src);
    assert_eq!(rc, Some(0), "{what}: expected exit 0:\n{out}");
    expect_codes(&out, &[], what);
    assert_eq!(shown(&out), want, "{what}:\n{out}");
}

/// Refused with exactly the codes `want`, and one line of `code` that starts with `at`
/// (`t.sv:LINE:COL: error[…]`) and contains every `needle`.
fn rejects(src: &str, want: &[&str], code: &str, at: &str, needles: &[&str], what: &str) {
    let (rc, out) = run(src);
    assert_eq!(rc, Some(1), "{what}: expected exit 1:\n{out}");
    expect_codes(&out, want, what);
    let lines: Vec<&str> = out.lines().filter(|l| l.contains(code)).collect();
    assert_eq!(lines.len(), 1, "{what}: one {code} per variable:\n{out}");
    assert!(lines[0].starts_with(at), "{what}: caret at {at}:\n{out}");
    for n in needles {
        assert!(lines[0].contains(n), "{what}: `{n}` in:\n{out}");
    }
}

const OTHER: &str = "is driven by a continuous `assign` and also written by";
const TWO: &str = "is driven by more than one continuous `assign`";
const PART: &str = "drives only part of variable";

// ── accepted: the sole writer, every value iverilog's ────────────────────────────

/// The blog's testbench shape (`i4_*`): a body variable of each kind, one `assign`, read
/// through a `logic` output. All three tools print these four lines for each.
#[test]
fn each_variable_kind_takes_one_assign() {
    let tb = "module tb;
  logic a, b; wire y;
  dut u(.a(a), .b(b), .y(y));
  initial begin
    a=0; b=1; #1 $display(\"t1 a=0 b=1 y=%b\", y);
    a=1; b=1; #1 $display(\"t2 a=1 b=1 y=%b\", y);
    a=1; b=0; #1 $display(\"t3 a=1 b=0 y=%b\", y);
    a=0; b=0; #1 $display(\"t4 a=0 b=0 y=%b\", y);
    #10 $finish;
  end
endmodule
";
    let and = [
        "t1 a=0 b=1 y=0",
        "t2 a=1 b=1 y=1",
        "t3 a=1 b=0 y=0",
        "t4 a=0 b=0 y=0",
    ];
    for (kind, rhs, out) in [
        ("reg", "a & b", "v"),
        ("bit", "a & b", "v"),
        ("byte", "a + b", "v[1]"),
        ("int", "a + b", "v[1]"),
        ("integer", "a + b", "v[1]"),
        ("time", "a + b", "v[1]"),
    ] {
        let dut = format!(
            "module dut(input logic a, b, output logic y);
  {kind} v;
  assign v = {rhs};
  assign y = {out};
endmodule
"
        );
        accepts(&format!("{dut}{tb}"), &and, kind);
    }
    let real = "module dut(input logic a, b, output logic y);
  real v;
  assign v = a + b + 0.5;
  assign y = (v > 1.0);
endmodule
";
    accepts(
        &format!("{real}{tb}"),
        &[
            "t1 a=0 b=1 y=1",
            "t2 a=1 b=1 y=0",
            "t3 a=1 b=0 y=1",
            "t4 a=0 b=0 y=0",
        ],
        "real",
    );
}

/// An output port that is a variable: ANSI `output reg`, non-ANSI `output reg`, and an
/// `output int` read through an `int` in the parent (`%0d`).
#[test]
fn an_output_port_variable_takes_one_assign() {
    let tb = |fmt: &str, decl: &str| {
        format!(
            "module tb;
  logic a, b; {decl} y;
  dut u(.a(a), .b(b), .y(y));
  initial begin
    a=0; b=1; #1 $display(\"t1 a=0 b=1 y=%{fmt}\", y);
    a=1; b=1; #1 $display(\"t2 a=1 b=1 y=%{fmt}\", y);
    a=1; b=0; #1 $display(\"t3 a=1 b=0 y=%{fmt}\", y);
    a=0; b=0; #1 $display(\"t4 a=0 b=0 y=%{fmt}\", y);
    #10 $finish;
  end
endmodule
"
        )
    };
    let and = [
        "t1 a=0 b=1 y=0",
        "t2 a=1 b=1 y=1",
        "t3 a=1 b=0 y=0",
        "t4 a=0 b=0 y=0",
    ];
    let ansi = "module dut(input logic a, b, output reg y);
  assign y = a & b;
endmodule
";
    accepts(
        &format!("{ansi}{}", tb("b", "wire")),
        &and,
        "ANSI output reg",
    );
    let non_ansi = "module dut(a, b, y);
  input a, b;
  output reg y;
  assign y = a & b;
endmodule
";
    accepts(
        &format!("{non_ansi}{}", tb("b", "wire")),
        &and,
        "non-ANSI output reg",
    );
    let int_port = "module dut(input logic a, b, output int y);
  assign y = a + b;
endmodule
";
    accepts(
        &format!("{int_port}{}", tb("0d", "int")),
        &[
            "t1 a=0 b=1 y=1",
            "t2 a=1 b=1 y=2",
            "t3 a=1 b=0 y=1",
            "t4 a=0 b=0 y=0",
        ],
        "output int",
    );
    // `x_int_port_chain`: the port's value carried on through the parent's `int`.
    accepts(
        "module dut(input logic [3:0] a, output int y);
  assign y = a * 3;
endmodule
module tb;
  logic [3:0] a; int y1;
  dut u(.a(a), .y(y1));
  initial begin
    a = 4'd5; #1 $display(\"t1 y1=%0d\", y1);
    a = 4'd9; #1 $display(\"t2 y1=%0d\", y1);
    $finish;
  end
endmodule
",
        &["t1 y1=15", "t2 y1=27"],
        "port chain",
    );
}

/// `x_bit_xcoerce`: a write to a 2-state variable through an `assign` coerces x and z to
/// 0 (IEEE 1800 §6.11.2), as a procedural write does. sv2v → iverilog loses the 2-state
/// type (`t1 v=x vi=X vb=x1z0x1z0`).
#[test]
fn a_two_state_variable_reads_x_as_0() {
    accepts(
        "module tb;
  logic a; bit v; logic [3:0] a4; int vi; byte vb;
  assign v = a;
  assign vi = a4;
  assign vb = {a4, a4};
  initial begin
    a = 1'bx; a4 = 4'bx1z0; #1 $display(\"t1 v=%b vi=%0d vb=%b\", v, vi, vb);
    a = 1'b1; a4 = 4'b0101; #1 $display(\"t2 v=%b vi=%0d vb=%b\", v, vi, vb);
    a = 1'bz; a4 = 4'b1x11; #1 $display(\"t3 v=%b vi=%0d vb=%b\", v, vi, vb);
    $finish;
  end
endmodule
",
        &[
            "t1 v=0 vi=4 vb=01000100",
            "t2 v=1 vi=5 vb=01010101",
            "t3 v=0 vi=11 vb=10111011",
        ],
        "2-state coercion",
    );
}

/// `x_integer_xpass`: a 4-state `integer`, `reg` and `time` keep x and z (verilator, 2-state,
/// prints `t1 v=…0100 r=01000100 t=68`).
#[test]
fn a_four_state_variable_keeps_x() {
    accepts(
        "module tb;
  logic [3:0] a4; integer v; reg [7:0] r; time t;
  assign v = a4;
  assign r = {a4, 4'h3} + 8'd1;
  assign t = {a4, a4};
  initial begin
    a4 = 4'bx1z0; #1 $display(\"t1 v=%b r=%b t=%0d\", v, r, t);
    a4 = 4'b0101; #1 $display(\"t2 v=%0d r=%h t=%0d\", v, r, t);
    $finish;
  end
endmodule
",
        &[
            "t1 v=0000000000000000000000000000x1z0 r=xxxxxxxx t=X",
            "t2 v=5 r=54 t=85",
        ],
        "integer x pass",
    );
}

/// A `real` driven continuously (`x_real`).
#[test]
fn a_real_variable_takes_one_assign() {
    accepts(
        "module tb;
  logic [3:0] a4; real r;
  assign r = a4 * 1.5;
  initial begin
    a4 = 4'd3; #1 $display(\"t1 r=%f\", r);
    a4 = 4'd7; #1 $display(\"t2 r=%f\", r);
    $finish;
  end
endmodule
",
        &["t1 r=4.500000", "t2 r=10.500000"],
        "real",
    );
}

/// When the value lands: an `assign #2` (`x_reg_delay`; verilator reads the initial `x` as
/// 0), at t0 (`x_reg_t0`), and the wake of a process waiting on the variable
/// (`x_reg_wake`).
#[test]
fn delay_t0_and_wake_follow_the_assign() {
    accepts(
        "module tb;
  logic a; reg r;
  assign #2 r = a;
  initial begin
    a = 0; #1 $display(\"t1 r=%b\", r); #2 $display(\"t3 r=%b\", r);
    a = 1; #1 $display(\"t4 r=%b\", r); #2 $display(\"t6 r=%b\", r);
    $finish;
  end
endmodule
",
        &["t1 r=x", "t3 r=0", "t4 r=0", "t6 r=1"],
        "delay",
    );
    accepts(
        "module tb;
  logic a = 1; reg r;
  assign r = a;
  initial $display(\"t0 r=%b\", r);
  initial #1 $display(\"t1 r=%b\", r);
  initial #2 $finish;
endmodule
",
        &["t0 r=1", "t1 r=1"],
        "t0",
    );
    accepts(
        "module tb;
  logic a; reg r; int n;
  assign r = a;
  always @(r) n = n + 1;
  initial begin
    n = 0; a = 0; #1 a = 1; #1 a = 1; #1 a = 0; #1 $display(\"n=%0d r=%b\", n, r);
    $finish;
  end
endmodule
",
        &["n=3 r=0"],
        "wake",
    );
}

/// Aggregates driven whole: a 2-state packed struct (`x_struct2s`; sv2v → iverilog reads
/// `t2 d=x1x1`) and a concatenation target over two variables (sv2v cannot translate it).
#[test]
fn a_struct_and_a_concatenation_take_one_assign() {
    accepts(
        "module tb;
  typedef struct packed { bit [2:0] a; bit b; } s_t;
  logic [3:0] v; s_t d;
  assign d = v;
  initial begin
    v = 4'h9; #1 $display(\"t1 d=%h a=%0d b=%b\", d, d.a, d.b);
    v = 4'bx1x1; #1 $display(\"t2 d=%b\", d);
    $finish;
  end
endmodule
",
        &["t1 d=9 a=4 b=1", "t2 d=0101"],
        "2-state struct",
    );
    accepts(
        "module t;
  logic [7:0] a; reg [3:0] hi; int lo;
  assign {hi, lo} = {a, 32'd5} + 36'd1;
  initial begin a = 8'h3c; #1 $display(\"hi=%h lo=%0d\", hi, lo); $finish; end
endmodule
",
        &["hi=c lo=6"],
        "concatenation",
    );
}

/// Elements of an unpacked array, each driven by its own `assign`, every element once:
/// `x_reg_array_elem`, a descending range and a 2-state array (verilator reads the `x`
/// elements as `00 01`; sv2v → iverilog loses the 2-state type, `x X`).
#[test]
fn elements_of_an_array_driven_once_each() {
    accepts(
        "module tb;
  logic [7:0] b; reg [7:0] rg [2];
  assign rg[0] = b;
  assign rg[1] = b + 8'd1;
  initial begin
    b = 8'h10; #1 $display(\"t1 %h %h\", rg[0], rg[1]);
    b = 8'h20; #1 $display(\"t2 %h %h\", rg[0], rg[1]);
    $finish;
  end
endmodule
",
        &["t1 10 11", "t2 20 21"],
        "reg array",
    );
    accepts(
        "module t;
  logic [7:0] b; reg [7:0] rg [3:2]; int ia [2];
  assign rg[2] = b;
  assign rg[3] = b + 8'd1;
  assign ia[1] = b;
  assign ia[0] = -b;
  initial begin
    b = 8'h10; #1 $display(\"t1 %h %h %0d %0d\", rg[2], rg[3], ia[0], ia[1]);
    b = 8'bx; #1 $display(\"t2 %h %h %0d %0d\", rg[2], rg[3], ia[0], ia[1]);
    $finish;
  end
endmodule
",
        &["t1 10 11 -16 16", "t2 xx xx 0 0"],
        "descending range, int array",
    );
}

/// Variables of an interface instance, driven by `assign`s outside it.
#[test]
fn interface_variables_take_one_assign() {
    accepts(
        "interface ifc; int w; reg [3:0] r; endinterface
module t;
  logic [3:0] a;
  ifc i();
  assign i.w = a * 2;
  assign i.r = a;
  initial begin a = 4'd3; #1 $display(\"t1 w=%0d r=%0d\", i.w, i.r); a = 4'd5; #1 $display(\"t2 w=%0d r=%0d\", i.w, i.r); $finish; end
endmodule
",
        &["t1 w=6 r=3", "t2 w=10 r=5"],
        "interface",
    );
}

/// A non-ANSI 2-state output port (`output bit y;`, `output int z;`) records its declared
/// kind, so a write of x reads 0. Without it the lift would have read `uy=x uz=X` where
/// iverilog reads 0 — and the PROCEDURAL twin already did, at exit 0, before §4.5.600
/// (verilator agrees; sv2v → iverilog loses the 2-state type, `uy=x uz=X`).
#[test]
fn a_non_ansi_two_state_port_coerces() {
    let body = |drive: &str| {
        format!(
            "module dut(a, y, z);
  input logic a;
  output bit y;
  output int z;
{drive}
endmodule
module t;
  logic a; bit y; int z;
  dut u(.a(a), .y(y), .z(z));
  initial begin a = 1'b1; #1 a = 1'bx; #1 $display(\"t1 uy=%b uz=%0d\", u.y, u.z); a = 1; #1 $display(\"t2 uy=%b uz=%0d\", u.y, u.z); $finish; end
endmodule
"
        )
    };
    let want = ["t1 uy=0 uz=0", "t2 uy=1 uz=1"];
    accepts(
        &body("  assign y = a;\n  assign z = {31'b0, a};"),
        &want,
        "continuous",
    );
    accepts(
        &body("  always @* y = a;\n  always @* z = {31'b0, a};"),
        &want,
        "procedural",
    );
    // The soundness review's v13, verbatim: an x written by the child's own `initial`,
    // read through the parent's `wire` (iverilog `y=0`; vita printed `y=x`).
    accepts(
        "module c(y); output bit y; initial y = 1'bx; endmodule
module top; wire y; c u(y); initial begin #1 $display(\"y=%b\", y); $finish; end endmodule
",
        &["y=0"],
        "v13",
    );
}

/// The same record names the port's type. Before it, `$typename` of a non-ANSI `output int
/// y;` and `output bit [3:0] z;` read `logic signed[31:0]` / `logic[3:0]` and the values
/// `v X xxxx xxxx`. verilator 5.052 prints the same `tn` line (iverilog has no `$typename`);
/// on the `v` line iverilog 13.0 (the design without `$typename`) prints `v 0 0000 xxxx`,
/// and verilator, 2-state, `v 0 0000 0000`.
#[test]
fn a_non_ansi_two_state_port_names_its_type() {
    accepts(
        "module d(a, y, z, w);
  input logic a;
  output int y;
  output bit [3:0] z;
  output reg [3:0] w;
  initial #1 $display(\"tn %s %s %s bits %0d %0d %0d\", $typename(y), $typename(z), $typename(w), $bits(y), $bits(z), $bits(w));
  always @* y = a;
  always @* z = {4{a}};
  always @* w = {4{a}};
endmodule
module t;
  logic a; int y; bit [3:0] z; logic [3:0] w;
  d u(.a(a), .y(y), .z(z), .w(w));
  initial begin a = 1; #1 a = 1'bx; #1 $display(\"v %0d %b %b\", u.y, u.z, u.w); $finish; end
endmodule
",
        &["tn int bit[3:0] logic[3:0] bits 32 4 4", "v 0 0000 xxxx"],
        "non-ANSI 2-state port type",
    );
    // The soundness review's v14, verbatim (verilator `tn=int`; vita printed
    // `tn=logic signed[31:0]`).
    accepts(
        "module c(y); output int y; initial $display(\"tn=%s\", $typename(y)); endmodule
module top; wire [31:0] y; c u(y); endmodule
",
        &["tn=int"],
        "v14",
    );
}

// ── the `.v` warning (owner ruling, 2026-10-06) ──────────────────────────────────

const N_REG_ASSIGN: &str = "module n_reg_assign (input a, b, output reg y);
    assign y = a & b;
endmodule
";

/// The blog's `n_reg_assign`: accepted in a `.sv` file with no diagnostic, and in a `.v`
/// file with `VITA-W3062` at the `assign` (iverilog `-g2005`: "Variable 'y' cannot be
/// driven by a continuous assignment/module." / "This is allowed when SystemVerilog is
/// enabled."; iverilog `-g2012` and verilator run both).
#[test]
fn n_reg_assign_warns_only_in_a_v_file() {
    let (rc, out) = run_as(N_REG_ASSIGN, "n_reg_assign.sv");
    assert_eq!(rc, Some(0), "{out}");
    expect_codes(&out, &[], ".sv");
    let (rc, out) = run_as(N_REG_ASSIGN, "n_reg_assign.v");
    assert_eq!(rc, Some(0), "{out}");
    expect_codes(&out, &["VITA-W3062"], ".v");
    let w: Vec<&str> = out.lines().filter(|l| l.contains("VITA-W3062")).collect();
    assert_eq!(w.len(), 1, "{out}");
    assert!(
        w[0].starts_with(
            "n_reg_assign.v:2:5: warning[VITA-W3062] W-ELAB-CONT-ASSIGN-VAR-1364: \
             continuous assignment to a variable is legal in IEEE 1800 but not in IEEE 1364 \
             (Verilog): `n_reg_assign.y`"
        ),
        "{out}"
    );
    // The extension in either case (soundness review F4: `.V` did not warn).
    let (rc, out) = run_as(N_REG_ASSIGN, "N_REG_ASSIGN.V");
    assert_eq!(rc, Some(0), "{out}");
    expect_codes(&out, &["VITA-W3062"], ".V");
}

/// The warning names the kinds IEEE 1364 calls variables — `reg`, `integer`, `time`,
/// `real`, `realtime` — and not `logic` or the SystemVerilog 2-state types; one per
/// `assign`. A refused `assign` gets no warning beside its error.
#[test]
fn the_v_warning_follows_the_1364_variable_kinds() {
    let src = "module t;
  logic a;
  reg r; integer i; time tm; real rl; realtime rt;
  logic l; int n; bit b;
  assign r = a;
  assign i = a;
  assign tm = a;
  assign rl = a;
  assign rt = a;
  assign l = a;
  assign n = a;
  assign b = a;
  initial begin a = 1; #1 $display(\"%b %0d %0d %0.1f %0.1f %b %0d %b\", r, i, tm, rl, rt, l, n, b); end
endmodule
";
    let (rc, out) = run_as(src, "t.v");
    assert_eq!(rc, Some(0), "{out}");
    expect_codes(&out, &["VITA-W3062"], "kinds");
    let at: Vec<&str> = out
        .lines()
        .filter(|l| l.contains("VITA-W3062"))
        .map(|l| l.split(": warning").next().unwrap_or(""))
        .collect();
    assert_eq!(
        at,
        ["t.v:5:3", "t.v:6:3", "t.v:7:3", "t.v:8:3", "t.v:9:3"],
        "{out}"
    );
    assert_eq!(shown(&out), ["1 1 1 1.0 1.0 1 1 1"], "{out}");
    // Two assigns on a `reg` in a `.v` file: Rule D's E3001 alone.
    let (rc, out) = run_as(
        "module t;\n  logic a, b; reg y;\n  assign y = a;\n  assign y = b;\nendmodule\n",
        "t.v",
    );
    assert_eq!(rc, Some(1), "{out}");
    expect_codes(&out, &["VITA-E3001"], "refused in a .v file");
}

// ── one diagnostic per variable: an E3001 is not doubled by an E3018 ──────────────

/// Two `assign`s (`i4_reg_cc2`), an `assign` beside an `always_comb`, and an initializer
/// beside an `assign` are `multidriver.rs`'s E3001 alone (iverilog: "Variable 'v' cannot
/// have multiple drivers." / "Cannot perform procedural assignment to variable 'y'
/// because it is also continuously assigned."). They printed E3018 beside it before.
#[test]
fn an_e3001_variable_gets_no_e3018() {
    rejects(
        "module dut(input logic a, b, output logic y);
  reg v;
  assign v = a & b;
  assign v = a | b;
  assign y = v;
endmodule
module tb; logic a, b; wire y; dut u(.a(a), .b(b), .y(y)); endmodule
",
        &["VITA-E3001"],
        "[VITA-E3001]",
        "t.sv:4:3",
        &["variable `v`", "more than one continuous `assign`"],
        "i4_reg_cc2",
    );
    rejects(
        "module t;\n  logic a, b; int y;\n  assign y = a;\n  always_comb y = b;\nendmodule\n",
        &["VITA-E3001"],
        "[VITA-E3001]",
        "t.sv:4:3",
        &["variable `y`", "`always_comb`"],
        "always_comb",
    );
    rejects(
        "module t;\n  logic a; reg [3:0] y = 4'd1;\n  assign y = a;\nendmodule\n",
        &["VITA-E3001"],
        "[VITA-E3001]",
        "t.sv:2:12",
        &["variable `y`", "declaration initializer"],
        "initializer",
    );
    // In a generate block Rule D does not look; a whole `assign` beside a delayed one is
    // the overlap check's E3001 (iverilog "Variable 'r' cannot have multiple drivers.").
    let (rc, out) = run(
        "module t;\n  logic a, b;\n  if (1) begin : g\n    reg r;\n    assign r = a;\n    assign #1 r = b;\n  end\nendmodule\n",
    );
    assert_eq!(rc, Some(1), "{out}");
    expect_codes(&out, &["VITA-E3001"], "generate overlap");
}

// ── refused: another writer the census names ─────────────────────────────────────

/// Each writer `multidriver.rs` Rule D does not reach, on a `reg` / `int`: a lifted
/// `assign` would have run beside it (an instance output plus an `assign` read `x1x0`
/// under a blanket lift). iverilog 13.0 refuses each one it parses.
#[test]
fn another_writer_keeps_e3018() {
    for (src, at, by, iverilog) in [
        (
            "module ch(input logic a, output logic o); assign o = a; endmodule
module t;
  logic a, b; reg y;
  ch u(.a(a), .o(y));
  assign y = b;
endmodule
",
            "t.sv:5:3",
            "a port connection",
            "Variable 'y' cannot have multiple drivers.",
        ),
        (
            "module c(input int a); assign a = 5; endmodule
module t; int x = 3; c u(.a(x)); endmodule
",
            "t.sv:1:24",
            "a port connection",
            "uwire \"a\" must have a single driver, found (2).",
        ),
        (
            "module t;
  logic [3:0] a; reg [3:0] y;
  task wt; y = 4'd0; endtask
  assign y = a;
  initial wt();
endmodule
",
            "t.sv:4:3",
            "a procedural assignment",
            "Variable 'y' cannot be driven by a continuous assignment/module or continuous assignment with non-default strength.",
        ),
        (
            "module c(input logic [3:0] a); int y; assign y = a; endmodule
module t;
  logic [3:0] a;
  c u(.a(a));
  initial u.y = 0;
endmodule
",
            "t.sv:1:39",
            "a procedural assignment",
            "Cannot perform procedural assignment to variable 'u.y' because it is also continuously assigned.",
        ),
        (
            "module t;
  reg [3:0] y;
  assign y = 4'h3;
  initial #1 void'($sscanf(\"5\", \"%h\", y));
endmodule
",
            "t.sv:3:3",
            "a system function's output argument",
            "(run time) $sscanf argument 3 (a vpiNet) is not assignable.",
        ),
        (
            "module t;
  logic [7:0] b; reg [7:0] rg [2];
  assign rg[0] = b; assign rg[1] = b;
  initial $readmemh(\"m.hex\", rg);
endmodule
",
            "t.sv:3:3",
            "a system task's output argument",
            "(run time) $readmemh's second argument must be a memory.",
        ),
        (
            "module t;
  logic [3:0] a; int y;
  task automatic st(output int o); o = 9; endtask
  assign y = a;
  initial st(y);
endmodule
",
            "t.sv:4:3",
            "an `output` or `inout` argument of a task or function call",
            "Cannot perform procedural assignment to variable 'y' because it is also continuously assigned.",
        ),
        (
            "module t;
  logic clk = 0; logic [7:0] a; reg [7:0] y;
  assign y = a;
  clocking cb @(posedge clk); output y; endclocking
  initial #12 cb.y <= 8'hAB;
endmodule
",
            "t.sv:3:3",
            "a clocking block output",
            "(cannot parse a clocking block)",
        ),
        (
            "module t;
  logic a, b; reg [1:0] y;
  assign y = {a, a};
  always @* y[0] = b;
endmodule
",
            "t.sv:3:3",
            "a procedural assignment",
            "Cannot perform procedural assignment to bit select 'y['sd0]' because it is also continuously assigned.",
        ),
        (
            "module t;
  logic a;
  if (1) begin : g
    reg r = 1'b1;
    assign r = a;
  end
endmodule
",
            "t.sv:5:5",
            "a procedural assignment",
            "Cannot perform procedural assignment to variable 'r' because it is also continuously assigned.",
        ),
        (
            "module t;
  reg [3:0] y;
  generate assign y = 4'h5; endgenerate
  initial y = 4'h0;
endmodule
",
            "t.sv:3:12",
            "a procedural assignment",
            "Cannot perform procedural assignment to variable 'y' because it is also continuously assigned.",
        ),
    ] {
        rejects(
            src,
            &["VITA-E3018"],
            "[VITA-E3018]",
            &format!("{at}: error[VITA-E3018] E-ELAB-LVALUE-KIND: variable `"),
            &[OTHER, by, "(IEEE 1800 §6.5, §10.3.2)"],
            iverilog,
        );
    }
}

/// Two `assign`s on one variable where `multidriver.rs` Rule D does not look: a generate
/// block, a `generate for`, an interface body, and one array element twice (iverilog
/// "Variable 'y' cannot have multiple drivers." for each).
#[test]
fn two_assigns_rule_d_does_not_see_keep_e3018() {
    for (src, at, what) in [
        (
            "module t;\n  logic a, b; reg y;\n  assign y = a;\n  if (1) begin : g assign y = b; end\nendmodule\n",
            "t.sv:4:20",
            "generate block",
        ),
        (
            "module t;\n  logic a; reg y;\n  for (genvar i = 0; i < 2; i++) begin : g assign y = a; end\nendmodule\n",
            "t.sv:3:44",
            "generate for",
        ),
        (
            "interface ifc(input logic a, b); int w; assign w = a; assign w = b; endinterface
module t; logic a, b; ifc i(.a(a), .b(b)); endmodule
",
            "t.sv:1:55",
            "interface body",
        ),
        (
            "module t;\n  logic [7:0] b; reg [7:0] rg [2];\n  assign rg[0] = b;\n  assign rg[0] = ~b;\n  assign rg[1] = b;\nendmodule\n",
            "t.sv:4:3",
            "one element twice",
        ),
    ] {
        rejects(
            src,
            &["VITA-E3018"],
            "[VITA-E3018]",
            &format!("{at}: error[VITA-E3018]"),
            &[TWO],
            what,
        );
    }
}

// ── refused: shapes v1 does not keep (out of scope, ROADMAP rows) ──────────────────

/// An `assign` that leaves part of a variable undriven stays E3018: a part select, a
/// struct member, an array not every element of which is driven — and a multi-dimensional
/// array's elements assigned one by one, whose flattened index carries a bounds guard the
/// check does not fold (iverilog, verilator and sv2v → iverilog print `m=1 2 3 4`;
/// ROADMAP §3.b `cont-var-partial`). The undriven part
/// reads differently across tools — iverilog 13.0 prints `r=zz10 b=0100 i=10zz`, `10 zz`
/// and `16 z` (`z` in an `int` element), vita's `logic` twin of the first reads `xx10`,
/// verilator `0010`.
#[test]
fn a_partly_driven_variable_keeps_e3018() {
    rejects(
        "module t;\n  logic [1:0] a; reg [3:0] r;\n  assign r[1:0] = a;\nendmodule\n",
        &["VITA-E3018"],
        "[VITA-E3018]",
        "t.sv:3:3",
        &[PART, "`t.r`"],
        "part select",
    );
    rejects(
        "module t;\n  typedef struct packed { bit [2:0] a; bit b; } s_t;\n  logic [2:0] v; s_t d;\n  assign d.a = v;\nendmodule\n",
        &["VITA-E3018"],
        "[VITA-E3018]",
        "t.sv:4:3",
        &[PART, "`t.d`"],
        "struct member",
    );
    rejects(
        "module t;\n  logic [7:0] b; reg [7:0] rg [2];\n  assign rg[0] = b;\nendmodule\n",
        &["VITA-E3018"],
        "[VITA-E3018]",
        "t.sv:3:3",
        &[PART, "`t.rg`"],
        "one element of two",
    );
    rejects(
        "module t;\n  logic [7:0] b; int ia [2];\n  assign ia[0] = b;\nendmodule\n",
        &["VITA-E3018"],
        "[VITA-E3018]",
        "t.sv:3:3",
        &[PART, "`t.ia`"],
        "one int element of two",
    );
    rejects(
        "module t;
  logic [7:0] b; reg [3:0] m [2][2];
  assign m[0][0] = b[3:0]; assign m[0][1] = b[7:4]; assign m[1][0] = 4'h1; assign m[1][1] = 4'h2;
endmodule
",
        &["VITA-E3018"],
        "[VITA-E3018]",
        "t.sv:3:3",
        &["drives an element of the multi-dimensional array `t.m`"],
        "multi-dimensional",
    );
}

/// Two writers IEEE 1800 allows, which v1 still refuses on a variable that is not `logic`:
/// a `force` over the `assign` (§10.6.2; iverilog `y=1 y=0 y=1 y=0`, and the `logic` twin
/// runs it), and one element assigned continuously beside another written procedurally
/// (§6.5; iverilog `10 55`).
#[test]
fn writers_ieee_allows_stay_refused() {
    rejects(
        "module t;
  logic a; reg y;
  assign y = a;
  initial begin a = 1; #1 force y = 0; #1 release y; end
endmodule
",
        &["VITA-E3018"],
        "[VITA-E3018]",
        "t.sv:3:3",
        &["also by a `force` or `release`", "declare it `logic`"],
        "force",
    );
    rejects(
        "module t;
  logic [7:0] b; reg [7:0] rg [2];
  assign rg[0] = b;
  initial rg[1] = 8'h55;
endmodule
",
        &["VITA-E3018"],
        "[VITA-E3018]",
        "t.sv:3:3",
        &[OTHER, "a procedural assignment"],
        "element + procedural element",
    );
}

/// A built-in gate's output on a variable that is not `logic` keeps E3018 (ROADMAP §3.b
/// `cont-var-gate`): IEEE 1800 §10.3.2 allows a variable one primitive output, but iverilog
/// 13.0 and sv2v → iverilog refuse it ("Variable 'y' cannot be driven by a primitive or
/// continuous assignment with non-default strength."), so only verilator, 2-state, runs it
/// (`y=1 l=1` / `y=0 l=1`, as vita's `logic` twin `l` does). A gate beside an `assign` is
/// Rule D's E3001 alone.
#[test]
fn a_gate_output_keeps_e3018() {
    rejects(
        "module t;
  logic a, b; reg y; logic l;
  and g(y, a, b);
  or g2(l, a, b);
  initial begin a = 1; b = 1; #1 $display(\"y=%b l=%b\", y, l); b = 0; #1 $display(\"y=%b l=%b\", y, l); $finish; end
endmodule
",
        &["VITA-E3018"],
        "[VITA-E3018]",
        "t.sv:3:3: error[VITA-E3018]",
        &["variable `t.y` is driven by a gate output", "declare it a `wire`"],
        "gate",
    );
    rejects(
        "module t;\n  logic a, b, c; reg y;\n  and g(y, a, b);\n  assign y = c;\nendmodule\n",
        &["VITA-E3001"],
        "[VITA-E3001]",
        "t.sv:4:3",
        &["more than one continuous driver (an `assign` or a gate output)"],
        "gate + assign",
    );
}

/// The kinds that keep the old E3018 (ROADMAP §3.b `cont-var-string`): a `string`
/// (iverilog 13.0 prints it as one bit — byte 0x01, then a space — where verilator prints `c`
/// / `ab`) and a class handle (iverilog aborts: `vvp_fun_bufz: recv_object(...) not
/// implemented`).
#[test]
fn a_string_or_a_class_handle_keeps_e3018() {
    for (src, name) in [
        (
            "module t;\n  logic a; string v;\n  assign v = a ? \"ab\" : \"c\";\nendmodule\n",
            "t.v",
        ),
        (
            "class C; int v; endclass\nmodule t;\n  C a, b;\n  assign b = a;\nendmodule\n",
            "t.b",
        ),
    ] {
        let (rc, out) = run(src);
        assert_eq!(rc, Some(1), "{out}");
        expect_codes(&out, &["VITA-E3018"], name);
        assert!(
            out.contains(&format!(
                "continuous assign drives variable `{name}` (declare it wire/logic)"
            )),
            "{out}"
        );
    }
}

/// A whole-array `assign` of a `reg` array (`cont_array.rs`'s element-by-element lowering)
/// meets the same rules as its element-wise spelling (soundness review F1): it runs (`m=1
/// 2`, iverilog), warns once in a `.v` file as the two element `assign`s warn twice, and
/// keeps E3018 when its right-hand side calls a function that reads a variable that is not
/// an argument — `logic` runs that design to `t2 m0=10` with verilator, iverilog prints
/// `t2 m0=6`. Before, the whole-array path bypassed both rules.
#[test]
fn a_whole_array_assign_meets_the_same_rules() {
    let whole = "module top; logic [7:0] b [2]; reg [7:0] m [2]; assign m = b; initial begin b[0] = 1; b[1] = 2; #1 $display(\"m=%0d %0d\", m[0], m[1]); $finish; end endmodule\n";
    accepts(whole, &["m=1 2"], "whole-array .sv");
    let (rc, out) = run_as(whole, "x1_whole.v");
    assert_eq!(rc, Some(0), "{out}");
    expect_codes(&out, &["VITA-W3062"], "whole-array .v");
    assert_eq!(out.matches("[VITA-W3062]").count(), 1, "{out}");
    let elems = "module top; logic [7:0] b [2]; reg [7:0] m [2]; assign m[0] = b[0]; assign m[1] = b[1]; initial begin b[0] = 1; b[1] = 2; #1 $display(\"m=%0d %0d\", m[0], m[1]); $finish; end endmodule\n";
    let (rc, out) = run_as(elems, "x2_elems.v");
    assert_eq!(rc, Some(0), "{out}");
    assert_eq!(out.matches("[VITA-W3062]").count(), 2, "{out}");
    rejects(
        "module top; integer k = 3; reg [7:0] m [0:1]; function [7:0] f(input [7:0] x); f = x * k; endfunction
assign m = '{f(2), 8'h1}; initial begin #1 $display(\"t1 m0=%0d\", m[0]); k = 5; #1 $display(\"t2 m0=%0d\", m[0]); $finish; end endmodule
",
        &["VITA-E3018"],
        "[VITA-E3018]",
        "t.sv:2:1: error[VITA-E3018]",
        &["`top.m` is driven by a continuous `assign` that calls a function"],
        "whole-array call",
    );
}

/// An `assign` whose right-hand side calls a function or method keeps E3018 (ROADMAP §3.b
/// `real-cont-assign`). A `logic` target re-runs the `assign` when the function body reads a
/// variable that is not an argument, as verilator does, and iverilog 13.0 does not (IEEE 1800
/// §10.3.2 re-evaluates on an operand): with `k` 1.25 then 3.0, `fr() = k * 2.0` prints `t2
/// w=6.000000` in verilator and `t2 w=2.500000` in iverilog and sv2v → iverilog; the `logic
/// [7:0]` twin `f() = k * 2` prints `t2 w=10` in vita and verilator, `t2 w=6` in iverilog.
/// So no call is lifted, though a callee reading only its formals agrees in all three
/// (`fr(a)`: `t1 w=0.750000 t2 w=2.250000`; `f(2)`: `w=7`). A class method with a side effect
/// (`n = n + 1`) is refused here; its `logic [31:0]` twin stops at F4016, a zero-delay loop
/// at time 0 (iverilog aborts, `vvp_wide_fun_t: recv_object(...) not implemented`; verilator
/// `t=1 y=1 n=1`).
#[test]
fn a_call_in_the_assign_keeps_e3018() {
    for (src, at) in [
        (
            "module t;
  real w; real k;
  function real fr(); fr = k * 2.0; endfunction
  assign w = fr();
  initial begin k = 1.25; #1 $display(\"t1 w=%f\", w); k = 3.0; #1 $display(\"t2 w=%f\", w); $finish; end
endmodule
",
            "t.sv:4:3",
        ),
        (
            "module t;
  logic [3:0] a; real w;
  function real fr(input logic [3:0] x); fr = x * 0.25; endfunction
  assign w = fr(a);
  initial begin a = 4'd3; #1 $display(\"t1 w=%f\", w); a = 4'd9; #1 $display(\"t2 w=%f\", w); $finish; end
endmodule
",
            "t.sv:4:3",
        ),
        (
            "module t;
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  int w;
  assign w = f(2);
  initial begin #1 $display(\"w=%0d\", w); $finish; end
endmodule
",
            "t.sv:7:3",
        ),
    ] {
        rejects(
            src,
            &["VITA-E3018"],
            "[VITA-E3018]",
            &format!("{at}: error[VITA-E3018]"),
            &["is driven by a continuous `assign` that calls a function"],
            at,
        );
    }
    let method = |decl: &str| {
        format!(
            "class C;
  int n;
  function int f(input logic x);
    n = n + 1;
    return n;
  endfunction
endclass
module top;
  logic a; {decl} y;
  C obj; initial obj = new;
  assign y = obj.f(a);
  initial begin
    a = 0;
    #1 $display(\"t=%0t y=%0d n=%0d\", $time, y, obj.n);
    #1 $finish;
  end
endmodule
"
        )
    };
    let (rc, out) = run(&method("int"));
    assert_eq!(rc, Some(1), "{out}");
    expect_codes(&out, &["VITA-E3018"], "method, int");
    let (rc, out) = run(&method("logic [31:0]"));
    assert_eq!(rc, Some(1), "{out}");
    expect_codes(&out, &["VITA-F4016"], "method, logic twin");
}
