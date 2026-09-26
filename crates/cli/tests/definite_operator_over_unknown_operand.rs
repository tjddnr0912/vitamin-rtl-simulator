//! The wide constant fold's ACCEPT SET over an x/z-bearing operand (ROADMAP §2 start-order
//! 🆕 H ⓐ): an operator that is DEFINITE whatever the unknown bits hold now folds.
//!
//! `const_wide::fold_self_bits` declined every reduction, logical operator, equality and
//! ternary condition the moment ONE operand bit was x or z. IEEE decides most of them
//! regardless: a known 0 decides `&` / `~&`, a known 1 decides `|` / `~|` and a truth
//! (`!`, `&&`, `||`, `?:`), `===` / `!==` compare the four states literally, and `==` /
//! `!=` are x only "if, due to unknown or high-impedance bits in the operands, the
//! relation is ambiguous" (§11.4.5) — a known bit that differs decides them. Both oracles
//! (iverilog 13.0 `-g2012`, verilator 5.052 `--binary --timing`) fold every definite cell
//! pinned here; the cells whose answer IS x stay loud (a `localparam` and an override are
//! E3009 as before) or, in a range bound, one bit as iverilog sizes them (verilator
//! refuses a non-two-state bound, §6.9.1 — an oracle split, pinned on iverilog's side).
//! A range bound `[(&4'b110x)+2:0]` was ONE bit at exit 0 against both oracles' three:
//! the bound consumers read a declined fold as width 1 with no diagnostic.
//!
//! An ambiguous one-bit result (`|4'b000x`) is carried as an x BIT rather than a decline,
//! so an operator above it that is definite regardless still folds
//! (`(4'bxxxx || 1'b0) || 1'b1` is 1); every value-reading consumer declines on the
//! unknown bit exactly as it declined on the `None`, measured at every binder below.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_any(src: &str) -> (bool, String) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_xdef_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (out.status.success(), s)
}

fn run(src: &str) -> String {
    let (ok, s) = run_any(src);
    assert!(ok, "expected exit 0, got:\n{s}");
    s
}

fn loud(src: &str, code: &str) -> String {
    let (ok, s) = run_any(src);
    assert!(
        !ok && s.contains(code),
        "expected a {code} rejection, got:\n{s}"
    );
    s
}

/// PRE: every one of these declared ONE bit at exit 0 (both oracles 3 or 4).
#[test]
fn a_range_bound_over_a_definite_operator_with_an_x_operand_is_sized_by_both_oracles() {
    assert!(run("module t;\n  wire [(&4'b110x)+2:0] w;\n  initial begin #1; $display(\"T ra0 bound=%0d\", $bits(w)); $finish; end\nendmodule\n").contains("T ra0 bound=3"));
    assert!(run("module t;\n  wire [(|4'b101x)+2:0] w;\n  initial begin #1; $display(\"T ro1 bound=%0d\", $bits(w)); $finish; end\nendmodule\n").contains("T ro1 bound=4"));
    assert!(run("module t;\n  wire [(~&4'b110x)+2:0] w;\n  initial begin #1; $display(\"T rna bound=%0d\", $bits(w)); $finish; end\nendmodule\n").contains("T rna bound=4"));
    assert!(run("module t;\n  wire [(~|4'b101x)+2:0] w;\n  initial begin #1; $display(\"T rno bound=%0d\", $bits(w)); $finish; end\nendmodule\n").contains("T rno bound=3"));
    assert!(run("module t;\n  wire [(!4'b101x)+2:0] w;\n  initial begin #1; $display(\"T ln0 bound=%0d\", $bits(w)); $finish; end\nendmodule\n").contains("T ln0 bound=3"));
    assert!(run("module t;\n  wire [(4'b1x1x && 1'b0)+2:0] w;\n  initial begin #1; $display(\"T la0 bound=%0d\", $bits(w)); $finish; end\nendmodule\n").contains("T la0 bound=3"));
    assert!(run("module t;\n  wire [(4'b101x && 1'b1)+2:0] w;\n  initial begin #1; $display(\"T la1 bound=%0d\", $bits(w)); $finish; end\nendmodule\n").contains("T la1 bound=4"));
    assert!(run("module t;\n  wire [(4'b000x || 1'b1)+2:0] w;\n  initial begin #1; $display(\"T lo1 bound=%0d\", $bits(w)); $finish; end\nendmodule\n").contains("T lo1 bound=4"));
    assert!(run("module t;\n  wire [(4'b1x10 === 4'b1x10)+2:0] w;\n  initial begin #1; $display(\"T ce1 bound=%0d\", $bits(w)); $finish; end\nendmodule\n").contains("T ce1 bound=4"));
    assert!(run("module t;\n  wire [(4'b1x10 === 4'b1x11)+2:0] w;\n  initial begin #1; $display(\"T ce0 bound=%0d\", $bits(w)); $finish; end\nendmodule\n").contains("T ce0 bound=3"));
    assert!(run("module t;\n  wire [(4'b1x10 !== 4'b1x11)+2:0] w;\n  initial begin #1; $display(\"T cn1 bound=%0d\", $bits(w)); $finish; end\nendmodule\n").contains("T cn1 bound=4"));
    assert!(run("module t;\n  wire [(4'b110x == 4'b0000)+2:0] w;\n  initial begin #1; $display(\"T eq0 bound=%0d\", $bits(w)); $finish; end\nendmodule\n").contains("T eq0 bound=3"));
    assert!(run("module t;\n  wire [(4'b110x != 4'b0000)+2:0] w;\n  initial begin #1; $display(\"T ne1 bound=%0d\", $bits(w)); $finish; end\nendmodule\n").contains("T ne1 bound=4"));
    assert!(run("module t;\n  wire [(&{61'd0, 4'b110x})+2:0] w;\n  initial begin #1; $display(\"T w65 bound=%0d\", $bits(w)); $finish; end\nendmodule\n").contains("T w65 bound=3"));
    assert!(run("module t;\n  wire [(|{61'd0, 4'b101x})+2:0] w;\n  initial begin #1; $display(\"T w65o bound=%0d\", $bits(w)); $finish; end\nendmodule\n").contains("T w65o bound=4"));
    assert!(run("module t;\n  wire [(&4'b110z)+2:0] w;\n  initial begin #1; $display(\"T rz0 bound=%0d\", $bits(w)); $finish; end\nendmodule\n").contains("T rz0 bound=3"));
    assert!(run("module t;\n  wire [(4'b110x ? 1'b1 : 1'b0)+2:0] w;\n  initial begin #1; $display(\"T tern bound=%0d\", $bits(w)); $finish; end\nendmodule\n").contains("T tern bound=4"));
}

/// PRE: E3009 `4'b110x has no constant-fold arm` on every one (both oracles the value).
#[test]
fn an_untyped_localparam_over_a_definite_operator_with_an_x_operand_folds() {
    assert!(run("module t;\n  localparam L = (&4'b110x) + 2;\n  initial begin #1; $display(\"T ra0 L=%0d Lbits=%0d\", L, $bits(L)); $finish; end\nendmodule\n").contains("T ra0 L=2 Lbits=32"));
    assert!(run("module t;\n  localparam L = (|4'b101x) + 2;\n  initial begin #1; $display(\"T ro1 L=%0d Lbits=%0d\", L, $bits(L)); $finish; end\nendmodule\n").contains("T ro1 L=3 Lbits=32"));
    assert!(run("module t;\n  localparam L = (~&4'b110x) + 2;\n  initial begin #1; $display(\"T rna L=%0d Lbits=%0d\", L, $bits(L)); $finish; end\nendmodule\n").contains("T rna L=3 Lbits=32"));
    assert!(run("module t;\n  localparam L = (~|4'b101x) + 2;\n  initial begin #1; $display(\"T rno L=%0d Lbits=%0d\", L, $bits(L)); $finish; end\nendmodule\n").contains("T rno L=2 Lbits=32"));
    assert!(run("module t;\n  localparam L = (!4'b101x) + 2;\n  initial begin #1; $display(\"T ln0 L=%0d Lbits=%0d\", L, $bits(L)); $finish; end\nendmodule\n").contains("T ln0 L=2 Lbits=32"));
    assert!(run("module t;\n  localparam L = (4'b1x1x && 1'b0) + 2;\n  initial begin #1; $display(\"T la0 L=%0d Lbits=%0d\", L, $bits(L)); $finish; end\nendmodule\n").contains("T la0 L=2 Lbits=32"));
    assert!(run("module t;\n  localparam L = (4'b101x && 1'b1) + 2;\n  initial begin #1; $display(\"T la1 L=%0d Lbits=%0d\", L, $bits(L)); $finish; end\nendmodule\n").contains("T la1 L=3 Lbits=32"));
    assert!(run("module t;\n  localparam L = (4'b000x || 1'b1) + 2;\n  initial begin #1; $display(\"T lo1 L=%0d Lbits=%0d\", L, $bits(L)); $finish; end\nendmodule\n").contains("T lo1 L=3 Lbits=32"));
    assert!(run("module t;\n  localparam L = (4'b1x10 === 4'b1x10) + 2;\n  initial begin #1; $display(\"T ce1 L=%0d Lbits=%0d\", L, $bits(L)); $finish; end\nendmodule\n").contains("T ce1 L=3 Lbits=32"));
    assert!(run("module t;\n  localparam L = (4'b1x10 === 4'b1x11) + 2;\n  initial begin #1; $display(\"T ce0 L=%0d Lbits=%0d\", L, $bits(L)); $finish; end\nendmodule\n").contains("T ce0 L=2 Lbits=32"));
    assert!(run("module t;\n  localparam L = (4'b1x10 !== 4'b1x11) + 2;\n  initial begin #1; $display(\"T cn1 L=%0d Lbits=%0d\", L, $bits(L)); $finish; end\nendmodule\n").contains("T cn1 L=3 Lbits=32"));
    assert!(run("module t;\n  localparam L = (4'b110x == 4'b0000) + 2;\n  initial begin #1; $display(\"T eq0 L=%0d Lbits=%0d\", L, $bits(L)); $finish; end\nendmodule\n").contains("T eq0 L=2 Lbits=32"));
    assert!(run("module t;\n  localparam L = (4'b110x != 4'b0000) + 2;\n  initial begin #1; $display(\"T ne1 L=%0d Lbits=%0d\", L, $bits(L)); $finish; end\nendmodule\n").contains("T ne1 L=3 Lbits=32"));
    assert!(run("module t;\n  localparam L = (&{61'd0, 4'b110x}) + 2;\n  initial begin #1; $display(\"T w65 L=%0d Lbits=%0d\", L, $bits(L)); $finish; end\nendmodule\n").contains("T w65 L=2 Lbits=32"));
    assert!(run("module t;\n  localparam L = (|{61'd0, 4'b101x}) + 2;\n  initial begin #1; $display(\"T w65o L=%0d Lbits=%0d\", L, $bits(L)); $finish; end\nendmodule\n").contains("T w65o L=3 Lbits=32"));
    assert!(run("module t;\n  localparam L = (&4'b110z) + 2;\n  initial begin #1; $display(\"T rz0 L=%0d Lbits=%0d\", L, $bits(L)); $finish; end\nendmodule\n").contains("T rz0 L=2 Lbits=32"));
    assert!(run("module t;\n  localparam L = (4'b110x ? 1'b1 : 1'b0) + 2;\n  initial begin #1; $display(\"T tern L=%0d Lbits=%0d\", L, $bits(L)); $finish; end\nendmodule\n").contains("T tern L=3 Lbits=32"));
}

/// The declared-width lane, the override lane and the generate-if condition; PRE was loud on all three.
#[test]
fn a_typed_localparam_an_override_and_a_generate_condition_fold_the_same_cells() {
    assert!(run("module t;\n  localparam logic [3:0] L = (&4'b110x) + 2;\n  initial begin #1; $display(\"T ra0 L=%0d Lbits=%0d\", L, $bits(L)); $finish; end\nendmodule\n").contains("T ra0 L=2 Lbits=4"));
    assert!(run("module t;\n  localparam logic [3:0] L = (|4'b101x) + 2;\n  initial begin #1; $display(\"T ro1 L=%0d Lbits=%0d\", L, $bits(L)); $finish; end\nendmodule\n").contains("T ro1 L=3 Lbits=4"));
    assert!(run("module t;\n  localparam logic [3:0] L = (~&4'b110x) + 2;\n  initial begin #1; $display(\"T rna L=%0d Lbits=%0d\", L, $bits(L)); $finish; end\nendmodule\n").contains("T rna L=3 Lbits=4"));
    assert!(run("module t;\n  localparam logic [3:0] L = (~|4'b101x) + 2;\n  initial begin #1; $display(\"T rno L=%0d Lbits=%0d\", L, $bits(L)); $finish; end\nendmodule\n").contains("T rno L=2 Lbits=4"));
    assert!(run("module t;\n  localparam logic [3:0] L = (!4'b101x) + 2;\n  initial begin #1; $display(\"T ln0 L=%0d Lbits=%0d\", L, $bits(L)); $finish; end\nendmodule\n").contains("T ln0 L=2 Lbits=4"));
    assert!(run("module t;\n  localparam logic [3:0] L = (4'b1x1x && 1'b0) + 2;\n  initial begin #1; $display(\"T la0 L=%0d Lbits=%0d\", L, $bits(L)); $finish; end\nendmodule\n").contains("T la0 L=2 Lbits=4"));
    assert!(run("module t;\n  localparam logic [3:0] L = (4'b101x && 1'b1) + 2;\n  initial begin #1; $display(\"T la1 L=%0d Lbits=%0d\", L, $bits(L)); $finish; end\nendmodule\n").contains("T la1 L=3 Lbits=4"));
    assert!(run("module t;\n  localparam logic [3:0] L = (4'b000x || 1'b1) + 2;\n  initial begin #1; $display(\"T lo1 L=%0d Lbits=%0d\", L, $bits(L)); $finish; end\nendmodule\n").contains("T lo1 L=3 Lbits=4"));
    assert!(run("module t;\n  localparam logic [3:0] L = (4'b1x10 === 4'b1x10) + 2;\n  initial begin #1; $display(\"T ce1 L=%0d Lbits=%0d\", L, $bits(L)); $finish; end\nendmodule\n").contains("T ce1 L=3 Lbits=4"));
    assert!(run("module t;\n  localparam logic [3:0] L = (4'b1x10 === 4'b1x11) + 2;\n  initial begin #1; $display(\"T ce0 L=%0d Lbits=%0d\", L, $bits(L)); $finish; end\nendmodule\n").contains("T ce0 L=2 Lbits=4"));
    assert!(run("module t;\n  localparam logic [3:0] L = (4'b1x10 !== 4'b1x11) + 2;\n  initial begin #1; $display(\"T cn1 L=%0d Lbits=%0d\", L, $bits(L)); $finish; end\nendmodule\n").contains("T cn1 L=3 Lbits=4"));
    assert!(run("module t;\n  localparam logic [3:0] L = (4'b110x == 4'b0000) + 2;\n  initial begin #1; $display(\"T eq0 L=%0d Lbits=%0d\", L, $bits(L)); $finish; end\nendmodule\n").contains("T eq0 L=2 Lbits=4"));
    assert!(run("module t;\n  localparam logic [3:0] L = (4'b110x != 4'b0000) + 2;\n  initial begin #1; $display(\"T ne1 L=%0d Lbits=%0d\", L, $bits(L)); $finish; end\nendmodule\n").contains("T ne1 L=3 Lbits=4"));
    assert!(run("module t;\n  localparam logic [3:0] L = (&{61'd0, 4'b110x}) + 2;\n  initial begin #1; $display(\"T w65 L=%0d Lbits=%0d\", L, $bits(L)); $finish; end\nendmodule\n").contains("T w65 L=2 Lbits=4"));
    assert!(run("module t;\n  localparam logic [3:0] L = (|{61'd0, 4'b101x}) + 2;\n  initial begin #1; $display(\"T w65o L=%0d Lbits=%0d\", L, $bits(L)); $finish; end\nendmodule\n").contains("T w65o L=3 Lbits=4"));
    assert!(run("module t;\n  localparam logic [3:0] L = (&4'b110z) + 2;\n  initial begin #1; $display(\"T rz0 L=%0d Lbits=%0d\", L, $bits(L)); $finish; end\nendmodule\n").contains("T rz0 L=2 Lbits=4"));
    assert!(run("module t;\n  localparam logic [3:0] L = (4'b110x ? 1'b1 : 1'b0) + 2;\n  initial begin #1; $display(\"T tern L=%0d Lbits=%0d\", L, $bits(L)); $finish; end\nendmodule\n").contains("T tern L=3 Lbits=4"));
    assert!(run("module sub #(parameter P = 7) ();\n  initial begin #1; $display(\"T ra0 P=%0d Pbits=%0d\", P, $bits(P)); end\nendmodule\nmodule t;\n  sub #(.P((&4'b110x))) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T ra0 P=0 Pbits=1"));
    assert!(run("module sub #(parameter P = 7) ();\n  initial begin #1; $display(\"T ro1 P=%0d Pbits=%0d\", P, $bits(P)); end\nendmodule\nmodule t;\n  sub #(.P((|4'b101x))) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T ro1 P=1 Pbits=1"));
    assert!(run("module sub #(parameter P = 7) ();\n  initial begin #1; $display(\"T rna P=%0d Pbits=%0d\", P, $bits(P)); end\nendmodule\nmodule t;\n  sub #(.P((~&4'b110x))) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T rna P=1 Pbits=1"));
    assert!(run("module sub #(parameter P = 7) ();\n  initial begin #1; $display(\"T rno P=%0d Pbits=%0d\", P, $bits(P)); end\nendmodule\nmodule t;\n  sub #(.P((~|4'b101x))) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T rno P=0 Pbits=1"));
    assert!(run("module sub #(parameter P = 7) ();\n  initial begin #1; $display(\"T ln0 P=%0d Pbits=%0d\", P, $bits(P)); end\nendmodule\nmodule t;\n  sub #(.P((!4'b101x))) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T ln0 P=0 Pbits=1"));
    assert!(run("module sub #(parameter P = 7) ();\n  initial begin #1; $display(\"T la0 P=%0d Pbits=%0d\", P, $bits(P)); end\nendmodule\nmodule t;\n  sub #(.P((4'b1x1x && 1'b0))) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T la0 P=0 Pbits=1"));
    assert!(run("module sub #(parameter P = 7) ();\n  initial begin #1; $display(\"T la1 P=%0d Pbits=%0d\", P, $bits(P)); end\nendmodule\nmodule t;\n  sub #(.P((4'b101x && 1'b1))) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T la1 P=1 Pbits=1"));
    assert!(run("module sub #(parameter P = 7) ();\n  initial begin #1; $display(\"T lo1 P=%0d Pbits=%0d\", P, $bits(P)); end\nendmodule\nmodule t;\n  sub #(.P((4'b000x || 1'b1))) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T lo1 P=1 Pbits=1"));
    assert!(run("module sub #(parameter P = 7) ();\n  initial begin #1; $display(\"T ce1 P=%0d Pbits=%0d\", P, $bits(P)); end\nendmodule\nmodule t;\n  sub #(.P((4'b1x10 === 4'b1x10))) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T ce1 P=1 Pbits=1"));
    assert!(run("module sub #(parameter P = 7) ();\n  initial begin #1; $display(\"T ce0 P=%0d Pbits=%0d\", P, $bits(P)); end\nendmodule\nmodule t;\n  sub #(.P((4'b1x10 === 4'b1x11))) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T ce0 P=0 Pbits=1"));
    assert!(run("module sub #(parameter P = 7) ();\n  initial begin #1; $display(\"T cn1 P=%0d Pbits=%0d\", P, $bits(P)); end\nendmodule\nmodule t;\n  sub #(.P((4'b1x10 !== 4'b1x11))) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T cn1 P=1 Pbits=1"));
    assert!(run("module sub #(parameter P = 7) ();\n  initial begin #1; $display(\"T eq0 P=%0d Pbits=%0d\", P, $bits(P)); end\nendmodule\nmodule t;\n  sub #(.P((4'b110x == 4'b0000))) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T eq0 P=0 Pbits=1"));
    assert!(run("module sub #(parameter P = 7) ();\n  initial begin #1; $display(\"T ne1 P=%0d Pbits=%0d\", P, $bits(P)); end\nendmodule\nmodule t;\n  sub #(.P((4'b110x != 4'b0000))) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T ne1 P=1 Pbits=1"));
    assert!(run("module sub #(parameter P = 7) ();\n  initial begin #1; $display(\"T w65 P=%0d Pbits=%0d\", P, $bits(P)); end\nendmodule\nmodule t;\n  sub #(.P((&{61'd0, 4'b110x}))) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T w65 P=0 Pbits=1"));
    assert!(run("module sub #(parameter P = 7) ();\n  initial begin #1; $display(\"T w65o P=%0d Pbits=%0d\", P, $bits(P)); end\nendmodule\nmodule t;\n  sub #(.P((|{61'd0, 4'b101x}))) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T w65o P=1 Pbits=1"));
    assert!(run("module sub #(parameter P = 7) ();\n  initial begin #1; $display(\"T rz0 P=%0d Pbits=%0d\", P, $bits(P)); end\nendmodule\nmodule t;\n  sub #(.P((&4'b110z))) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T rz0 P=0 Pbits=1"));
    assert!(run("module sub #(parameter P = 7) ();\n  initial begin #1; $display(\"T tern P=%0d Pbits=%0d\", P, $bits(P)); end\nendmodule\nmodule t;\n  sub #(.P((4'b110x ? 1'b1 : 1'b0))) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T tern P=1 Pbits=1"));
    assert!(run("module t;\n  generate if ((&4'b110x) == 0) begin : g0\n    initial begin #1; $display(\"T ra0 branch=0\"); end\n  end else begin : g1\n    initial begin #1; $display(\"T ra0 branch=1\"); end\n  end endgenerate\n  initial begin #2 $finish; end\nendmodule\n").contains("T ra0 branch=0"));
    assert!(run("module t;\n  generate if ((|4'b101x) == 0) begin : g0\n    initial begin #1; $display(\"T ro1 branch=0\"); end\n  end else begin : g1\n    initial begin #1; $display(\"T ro1 branch=1\"); end\n  end endgenerate\n  initial begin #2 $finish; end\nendmodule\n").contains("T ro1 branch=1"));
    assert!(run("module t;\n  generate if ((~&4'b110x) == 0) begin : g0\n    initial begin #1; $display(\"T rna branch=0\"); end\n  end else begin : g1\n    initial begin #1; $display(\"T rna branch=1\"); end\n  end endgenerate\n  initial begin #2 $finish; end\nendmodule\n").contains("T rna branch=1"));
    assert!(run("module t;\n  generate if ((~|4'b101x) == 0) begin : g0\n    initial begin #1; $display(\"T rno branch=0\"); end\n  end else begin : g1\n    initial begin #1; $display(\"T rno branch=1\"); end\n  end endgenerate\n  initial begin #2 $finish; end\nendmodule\n").contains("T rno branch=0"));
    assert!(run("module t;\n  generate if ((!4'b101x) == 0) begin : g0\n    initial begin #1; $display(\"T ln0 branch=0\"); end\n  end else begin : g1\n    initial begin #1; $display(\"T ln0 branch=1\"); end\n  end endgenerate\n  initial begin #2 $finish; end\nendmodule\n").contains("T ln0 branch=0"));
    assert!(run("module t;\n  generate if ((4'b1x1x && 1'b0) == 0) begin : g0\n    initial begin #1; $display(\"T la0 branch=0\"); end\n  end else begin : g1\n    initial begin #1; $display(\"T la0 branch=1\"); end\n  end endgenerate\n  initial begin #2 $finish; end\nendmodule\n").contains("T la0 branch=0"));
    assert!(run("module t;\n  generate if ((4'b101x && 1'b1) == 0) begin : g0\n    initial begin #1; $display(\"T la1 branch=0\"); end\n  end else begin : g1\n    initial begin #1; $display(\"T la1 branch=1\"); end\n  end endgenerate\n  initial begin #2 $finish; end\nendmodule\n").contains("T la1 branch=1"));
    assert!(run("module t;\n  generate if ((4'b000x || 1'b1) == 0) begin : g0\n    initial begin #1; $display(\"T lo1 branch=0\"); end\n  end else begin : g1\n    initial begin #1; $display(\"T lo1 branch=1\"); end\n  end endgenerate\n  initial begin #2 $finish; end\nendmodule\n").contains("T lo1 branch=1"));
    assert!(run("module t;\n  generate if ((4'b1x10 === 4'b1x10) == 0) begin : g0\n    initial begin #1; $display(\"T ce1 branch=0\"); end\n  end else begin : g1\n    initial begin #1; $display(\"T ce1 branch=1\"); end\n  end endgenerate\n  initial begin #2 $finish; end\nendmodule\n").contains("T ce1 branch=1"));
    assert!(run("module t;\n  generate if ((4'b1x10 === 4'b1x11) == 0) begin : g0\n    initial begin #1; $display(\"T ce0 branch=0\"); end\n  end else begin : g1\n    initial begin #1; $display(\"T ce0 branch=1\"); end\n  end endgenerate\n  initial begin #2 $finish; end\nendmodule\n").contains("T ce0 branch=0"));
    assert!(run("module t;\n  generate if ((4'b1x10 !== 4'b1x11) == 0) begin : g0\n    initial begin #1; $display(\"T cn1 branch=0\"); end\n  end else begin : g1\n    initial begin #1; $display(\"T cn1 branch=1\"); end\n  end endgenerate\n  initial begin #2 $finish; end\nendmodule\n").contains("T cn1 branch=1"));
    assert!(run("module t;\n  generate if ((4'b110x == 4'b0000) == 0) begin : g0\n    initial begin #1; $display(\"T eq0 branch=0\"); end\n  end else begin : g1\n    initial begin #1; $display(\"T eq0 branch=1\"); end\n  end endgenerate\n  initial begin #2 $finish; end\nendmodule\n").contains("T eq0 branch=0"));
    assert!(run("module t;\n  generate if ((4'b110x != 4'b0000) == 0) begin : g0\n    initial begin #1; $display(\"T ne1 branch=0\"); end\n  end else begin : g1\n    initial begin #1; $display(\"T ne1 branch=1\"); end\n  end endgenerate\n  initial begin #2 $finish; end\nendmodule\n").contains("T ne1 branch=1"));
    assert!(run("module t;\n  generate if ((&{61'd0, 4'b110x}) == 0) begin : g0\n    initial begin #1; $display(\"T w65 branch=0\"); end\n  end else begin : g1\n    initial begin #1; $display(\"T w65 branch=1\"); end\n  end endgenerate\n  initial begin #2 $finish; end\nendmodule\n").contains("T w65 branch=0"));
    assert!(run("module t;\n  generate if ((|{61'd0, 4'b101x}) == 0) begin : g0\n    initial begin #1; $display(\"T w65o branch=0\"); end\n  end else begin : g1\n    initial begin #1; $display(\"T w65o branch=1\"); end\n  end endgenerate\n  initial begin #2 $finish; end\nendmodule\n").contains("T w65o branch=1"));
    assert!(run("module t;\n  generate if ((&4'b110z) == 0) begin : g0\n    initial begin #1; $display(\"T rz0 branch=0\"); end\n  end else begin : g1\n    initial begin #1; $display(\"T rz0 branch=1\"); end\n  end endgenerate\n  initial begin #2 $finish; end\nendmodule\n").contains("T rz0 branch=0"));
    assert!(run("module t;\n  generate if ((4'b110x ? 1'b1 : 1'b0) == 0) begin : g0\n    initial begin #1; $display(\"T tern branch=0\"); end\n  end else begin : g1\n    initial begin #1; $display(\"T tern branch=1\"); end\n  end endgenerate\n  initial begin #2 $finish; end\nendmodule\n").contains("T tern branch=1"));
}

/// An answer that IS x keeps its route: a `localparam` (untyped and typed) and an override
/// are E3009 as on PRE — both oracles print `x`, and vita's binders have no unknown plane
/// for a scalar — and a generate-if on one is E3010 (the oracles split: iverilog takes
/// the else branch, verilator the then branch). A range bound over one is ONE bit, which
/// is iverilog's sizing; verilator refuses the bound (§6.9.1).
#[test]
fn an_answer_that_is_x_stays_loud_or_takes_iverilogs_one_bit_bound() {
    loud("module t;\n  localparam L = (^4'b110x) + 2;\n  initial begin #1; $display(\"T rx L=%0d Lbits=%0d\", L, $bits(L)); $finish; end\nendmodule\n", "E3009");
    loud("module t;\n  localparam logic [3:0] L = (^4'b110x) + 2;\n  initial begin #1; $display(\"T rx L=%0d Lbits=%0d\", L, $bits(L)); $finish; end\nendmodule\n", "E3009");
    loud("module sub #(parameter P = 7) ();\n  initial begin #1; $display(\"T rx P=%0d Pbits=%0d\", P, $bits(P)); end\nendmodule\nmodule t;\n  sub #(.P((^4'b110x))) u();\n  initial begin #2 $finish; end\nendmodule\n", "E3009");
    loud("module t;\n  generate if ((^4'b110x) == 0) begin : g0\n    initial begin #1; $display(\"T rx branch=0\"); end\n  end else begin : g1\n    initial begin #1; $display(\"T rx branch=1\"); end\n  end endgenerate\n  initial begin #2 $finish; end\nendmodule\n", "E3010");
    assert!(run("module t;\n  wire [(^4'b110x)+2:0] w;\n  initial begin #1; $display(\"T rx bound=%0d\", $bits(w)); $finish; end\nendmodule\n").contains("T rx bound=1"));
    loud("module t;\n  localparam L = (&4'b111x) + 2;\n  initial begin #1; $display(\"T rax L=%0d Lbits=%0d\", L, $bits(L)); $finish; end\nendmodule\n", "E3009");
    loud("module t;\n  localparam logic [3:0] L = (&4'b111x) + 2;\n  initial begin #1; $display(\"T rax L=%0d Lbits=%0d\", L, $bits(L)); $finish; end\nendmodule\n", "E3009");
    loud("module sub #(parameter P = 7) ();\n  initial begin #1; $display(\"T rax P=%0d Pbits=%0d\", P, $bits(P)); end\nendmodule\nmodule t;\n  sub #(.P((&4'b111x))) u();\n  initial begin #2 $finish; end\nendmodule\n", "E3009");
    loud("module t;\n  generate if ((&4'b111x) == 0) begin : g0\n    initial begin #1; $display(\"T rax branch=0\"); end\n  end else begin : g1\n    initial begin #1; $display(\"T rax branch=1\"); end\n  end endgenerate\n  initial begin #2 $finish; end\nendmodule\n", "E3010");
    assert!(run("module t;\n  wire [(&4'b111x)+2:0] w;\n  initial begin #1; $display(\"T rax bound=%0d\", $bits(w)); $finish; end\nendmodule\n").contains("T rax bound=1"));
    loud("module t;\n  localparam L = (|4'b000x) + 2;\n  initial begin #1; $display(\"T rox L=%0d Lbits=%0d\", L, $bits(L)); $finish; end\nendmodule\n", "E3009");
    loud("module t;\n  localparam logic [3:0] L = (|4'b000x) + 2;\n  initial begin #1; $display(\"T rox L=%0d Lbits=%0d\", L, $bits(L)); $finish; end\nendmodule\n", "E3009");
    loud("module sub #(parameter P = 7) ();\n  initial begin #1; $display(\"T rox P=%0d Pbits=%0d\", P, $bits(P)); end\nendmodule\nmodule t;\n  sub #(.P((|4'b000x))) u();\n  initial begin #2 $finish; end\nendmodule\n", "E3009");
    loud("module t;\n  generate if ((|4'b000x) == 0) begin : g0\n    initial begin #1; $display(\"T rox branch=0\"); end\n  end else begin : g1\n    initial begin #1; $display(\"T rox branch=1\"); end\n  end endgenerate\n  initial begin #2 $finish; end\nendmodule\n", "E3010");
    assert!(run("module t;\n  wire [(|4'b000x)+2:0] w;\n  initial begin #1; $display(\"T rox bound=%0d\", $bits(w)); $finish; end\nendmodule\n").contains("T rox bound=1"));
    loud("module t;\n  localparam L = (!4'b000x) + 2;\n  initial begin #1; $display(\"T lnx L=%0d Lbits=%0d\", L, $bits(L)); $finish; end\nendmodule\n", "E3009");
    loud("module t;\n  localparam logic [3:0] L = (!4'b000x) + 2;\n  initial begin #1; $display(\"T lnx L=%0d Lbits=%0d\", L, $bits(L)); $finish; end\nendmodule\n", "E3009");
    loud("module sub #(parameter P = 7) ();\n  initial begin #1; $display(\"T lnx P=%0d Pbits=%0d\", P, $bits(P)); end\nendmodule\nmodule t;\n  sub #(.P((!4'b000x))) u();\n  initial begin #2 $finish; end\nendmodule\n", "E3009");
    loud("module t;\n  generate if ((!4'b000x) == 0) begin : g0\n    initial begin #1; $display(\"T lnx branch=0\"); end\n  end else begin : g1\n    initial begin #1; $display(\"T lnx branch=1\"); end\n  end endgenerate\n  initial begin #2 $finish; end\nendmodule\n", "E3010");
    assert!(run("module t;\n  wire [(!4'b000x)+2:0] w;\n  initial begin #1; $display(\"T lnx bound=%0d\", $bits(w)); $finish; end\nendmodule\n").contains("T lnx bound=1"));
    loud("module t;\n  localparam L = (4'b000x && 1'b1) + 2;\n  initial begin #1; $display(\"T lax L=%0d Lbits=%0d\", L, $bits(L)); $finish; end\nendmodule\n", "E3009");
    loud("module t;\n  localparam logic [3:0] L = (4'b000x && 1'b1) + 2;\n  initial begin #1; $display(\"T lax L=%0d Lbits=%0d\", L, $bits(L)); $finish; end\nendmodule\n", "E3009");
    loud("module sub #(parameter P = 7) ();\n  initial begin #1; $display(\"T lax P=%0d Pbits=%0d\", P, $bits(P)); end\nendmodule\nmodule t;\n  sub #(.P((4'b000x && 1'b1))) u();\n  initial begin #2 $finish; end\nendmodule\n", "E3009");
    loud("module t;\n  generate if ((4'b000x && 1'b1) == 0) begin : g0\n    initial begin #1; $display(\"T lax branch=0\"); end\n  end else begin : g1\n    initial begin #1; $display(\"T lax branch=1\"); end\n  end endgenerate\n  initial begin #2 $finish; end\nendmodule\n", "E3010");
    assert!(run("module t;\n  wire [(4'b000x && 1'b1)+2:0] w;\n  initial begin #1; $display(\"T lax bound=%0d\", $bits(w)); $finish; end\nendmodule\n").contains("T lax bound=1"));
    loud("module t;\n  localparam L = (4'b110x == 4'b1100) + 2;\n  initial begin #1; $display(\"T eqx L=%0d Lbits=%0d\", L, $bits(L)); $finish; end\nendmodule\n", "E3009");
    loud("module t;\n  localparam logic [3:0] L = (4'b110x == 4'b1100) + 2;\n  initial begin #1; $display(\"T eqx L=%0d Lbits=%0d\", L, $bits(L)); $finish; end\nendmodule\n", "E3009");
    loud("module sub #(parameter P = 7) ();\n  initial begin #1; $display(\"T eqx P=%0d Pbits=%0d\", P, $bits(P)); end\nendmodule\nmodule t;\n  sub #(.P((4'b110x == 4'b1100))) u();\n  initial begin #2 $finish; end\nendmodule\n", "E3009");
    loud("module t;\n  generate if ((4'b110x == 4'b1100) == 0) begin : g0\n    initial begin #1; $display(\"T eqx branch=0\"); end\n  end else begin : g1\n    initial begin #1; $display(\"T eqx branch=1\"); end\n  end endgenerate\n  initial begin #2 $finish; end\nendmodule\n", "E3010");
    assert!(run("module t;\n  wire [(4'b110x == 4'b1100)+2:0] w;\n  initial begin #1; $display(\"T eqx bound=%0d\", $bits(w)); $finish; end\nendmodule\n").contains("T eqx bound=1"));
}

/// Wide operands (a 68-bit concat), a comparison whose x-bearing side is NARROWER than
/// the other (`4'b110x == 8'h00`: the unsigned side is zero-extended, §11.6.1, and a known
/// bit differs), nested truths (`!(!4'b101x)`, a ternary on a reduction, a `&&` / `||`
/// chain through an x link) and a `z` operand. PRE: bound 1 / E3009 on every one.
#[test]
fn wide_operands_mixed_widths_nested_truths_and_z_fold_where_definite() {
    assert!(run("module t;\n  \n  wire [(&{64'hFFFF_FFFF_FFFF_FFFF, 4'b110x})+2:0] w;\n  localparam L = (&{64'hFFFF_FFFF_FFFF_FFFF, 4'b110x}) + 2;\n  initial begin #1; $display(\"T w_ra0 bound=%0d L=%0d\", $bits(w), L); $finish; end\nendmodule\n").contains("T w_ra0 bound=3 L=2"));
    assert!(run("module t;\n  \n  wire [({64'hx, 4'b1x10} === {64'hx, 4'b1x10})+2:0] w;\n  localparam L = ({64'hx, 4'b1x10} === {64'hx, 4'b1x10}) + 2;\n  initial begin #1; $display(\"T w_ce1 bound=%0d L=%0d\", $bits(w), L); $finish; end\nendmodule\n").contains("T w_ce1 bound=4 L=3"));
    assert!(run("module t;\n  \n  wire [(4'b110x == 8'h00)+2:0] w;\n  localparam L = (4'b110x == 8'h00) + 2;\n  initial begin #1; $display(\"T m_eq0 bound=%0d L=%0d\", $bits(w), L); $finish; end\nendmodule\n").contains("T m_eq0 bound=3 L=2"));
    assert!(run("module t;\n  \n  wire [(4'b1x10 === 8'b0000_1x10)+2:0] w;\n  localparam L = (4'b1x10 === 8'b0000_1x10) + 2;\n  initial begin #1; $display(\"T m_ce0 bound=%0d L=%0d\", $bits(w), L); $finish; end\nendmodule\n").contains("T m_ce0 bound=4 L=3"));
    assert!(run("module t;\n  \n  wire [((&4'b110x) ? 4'd9 : 4'd5)+2:0] w;\n  localparam L = ((&4'b110x) ? 4'd9 : 4'd5) + 2;\n  initial begin #1; $display(\"T x_tern bound=%0d L=%0d\", $bits(w), L); $finish; end\nendmodule\n").contains("T x_tern bound=8 L=7"));
    assert!(run("module t;\n  \n  wire [((!4'b101x) ? 4'd9 : 4'd5)+2:0] w;\n  localparam L = ((!4'b101x) ? 4'd9 : 4'd5) + 2;\n  initial begin #1; $display(\"T x_tnot bound=%0d L=%0d\", $bits(w), L); $finish; end\nendmodule\n").contains("T x_tnot bound=8 L=7"));
    assert!(run("module t;\n  \n  wire [(4'b1x1x && 4'b0000 && 4'bxxxx)+2:0] w;\n  localparam L = (4'b1x1x && 4'b0000 && 4'bxxxx) + 2;\n  initial begin #1; $display(\"T x_and3 bound=%0d L=%0d\", $bits(w), L); $finish; end\nendmodule\n").contains("T x_and3 bound=3 L=2"));
    assert!(run("module t;\n  \n  wire [(4'bxxxx || 4'b0000 || 4'b0001)+2:0] w;\n  localparam L = (4'bxxxx || 4'b0000 || 4'b0001) + 2;\n  initial begin #1; $display(\"T x_or3 bound=%0d L=%0d\", $bits(w), L); $finish; end\nendmodule\n").contains("T x_or3 bound=4 L=3"));
    assert!(run("module t;\n  \n  wire [(!(!4'b101x))+2:0] w;\n  localparam L = (!(!4'b101x)) + 2;\n  initial begin #1; $display(\"T x_lnn bound=%0d L=%0d\", $bits(w), L); $finish; end\nendmodule\n").contains("T x_lnn bound=4 L=3"));
    assert!(run("module t;\n  \n  wire [((&4'b110x) == 1'b0)+2:0] w;\n  localparam L = ((&4'b110x) == 1'b0) + 2;\n  initial begin #1; $display(\"T x_cs0 bound=%0d L=%0d\", $bits(w), L); $finish; end\nendmodule\n").contains("T x_cs0 bound=4 L=3"));
    assert!(run("module t;\n  \n  wire [(|4'b1z0z)+2:0] w;\n  localparam L = (|4'b1z0z) + 2;\n  initial begin #1; $display(\"T x_xz bound=%0d L=%0d\", $bits(w), L); $finish; end\nendmodule\n").contains("T x_xz bound=4 L=3"));
    assert!(run("module t;\n  \n  wire [(^4'b1100)+2:0] w;\n  localparam L = (^4'b1100) + 2;\n  initial begin #1; $display(\"T x_rxor bound=%0d L=%0d\", $bits(w), L); $finish; end\nendmodule\n").contains("T x_rxor bound=3 L=2"));
}

/// Where the carried x BIT lands (the loud→value column, measured at every binder): a
/// 128-bit `localparam` holds it as `0…0x` (both oracles; the unsigned extension fills
/// zeros above it — `resize_bits` alone x-extended an unknown top bit), a class field
/// initializer holds `000x` (iverilog; verilator's 2-state class field is `0000`), and a
/// definite twin binds at every one of those sites. A ≤64-bit typed / untyped
/// `localparam`, an override (typed, 2-state or untyped) and a concatenation holding the
/// x bit stay loud: those binders drop the unknown plane (§2 row 15), so the override lane
/// declines an x out of an OPERATOR while a sized x/z LITERAL keeps row 15's route.
#[test]
fn the_x_bit_lands_only_where_the_binder_keeps_an_unknown_plane() {
    assert!(run("module t;\n  localparam logic [127:0] L = (|4'b000x);\n  initial begin #1; $display(\"T k_w L=%h\", L); $finish; end\nendmodule\n").contains("T k_w L=0000000000000000000000000000000X"));
    assert!(run("class C; logic [3:0] f = (|4'b000x); endclass\nmodule t;\n  C c;\n  initial begin #1; c = new; $display(\"T k_cls f=%b\", c.f); $finish; end\nendmodule\n").contains("T k_cls f=000x"));
    assert!(run("module t;\n  localparam logic [3:0] L = (|4'b001x);\n  initial begin #1; $display(\"T k_d4 L=%b\", L); $finish; end\nendmodule\n").contains("T k_d4 L=0001"));
    assert!(run("class C; logic [3:0] f = (|4'b001x); endclass\nmodule t;\n  C c;\n  initial begin #1; c = new; $display(\"T k_dcls f=%b\", c.f); $finish; end\nendmodule\n").contains("T k_dcls f=0001"));
    assert!(run("module t;\n  initial begin\n    logic [3:0] v = (|4'b000x);\n    #1; $display(\"T k_blk v=%b\", v); $finish;\n  end\nendmodule\n").contains("T k_blk v=000x"));
    assert!(run("module t;\n  wire [3:0] n = (|4'b000x);\n  initial begin #1; $display(\"T k_net n=%b\", n); $finish; end\nendmodule\n").contains("T k_net n=000x"));
    loud("module t;\n  localparam logic [3:0] L = (|4'b000x);\n  initial begin #1; $display(\"T k_t4 L=%b\", L); $finish; end\nendmodule\n", "E3009");
    loud("module t;\n  localparam L = (|4'b000x);\n  initial begin #1; $display(\"T k_u L=%b\", L); $finish; end\nendmodule\n", "E3009");
    loud("module sub #(parameter logic [3:0] P = 4'd7) ();\n  initial begin #1; $display(\"T k_o4 P=%b\", P); end\nendmodule\nmodule t;\n  sub #(.P((|4'b000x))) u();\n  initial begin #2 $finish; end\nendmodule\n", "E3009");
    loud("module sub #(parameter P = 7) ();\n  initial begin #1; $display(\"T k_ou P=%b\", P); end\nendmodule\nmodule t;\n  sub #(.P((|4'b000x))) u();\n  initial begin #2 $finish; end\nendmodule\n", "E3009");
    loud("module t;\n  localparam bit [3:0] L = (|4'b000x);\n  initial begin #1; $display(\"T k_t2 L=%b\", L); $finish; end\nendmodule\n", "E3009");
    loud("module sub #(parameter bit [3:0] P = 4'd7) ();\n  initial begin #1; $display(\"T k_o2 P=%b\", P); end\nendmodule\nmodule t;\n  sub #(.P((|4'b000x))) u();\n  initial begin #2 $finish; end\nendmodule\n", "E3009");
    loud("module t;\n  localparam logic [1:0] L = {(|4'b000x), 1'b0};\n  initial begin #1; $display(\"T k_cat L=%b\", L); $finish; end\nendmodule\n", "E3009");
}

/// Not this slice, pinned as measured: a parameter whose own VALUE carries x
/// (`parameter logic [3:0] X = 4'b110x;`) is E3009 at its declaration, so a definite
/// operator over the NAME never folds (both oracles fold `&X` to 0) — §2 row 15's
/// x/z-bearing parameter value; and a bitwise `~` over an x operand declines
/// (`~&(~4'b110x)` is 1 in both oracles) — the bitwise arms have no 4-state tables.
#[test]
fn an_x_bearing_parameter_value_and_a_bitwise_complement_over_x_are_not_this_slice() {
    loud("module t;\n  parameter logic [3:0] X = 4'b110x;\n  wire [(&X)+2:0] w;\n  localparam L = (&X) + 2;\n  initial begin #1; $display(\"T n_ra0 bound=%0d L=%0d\", $bits(w), L); $finish; end\nendmodule\n", "E3009");
    loud("module t;\n  parameter logic [3:0] X = 4'b101x;\n  wire [(|X)+2:0] w;\n  localparam L = (|X) + 2;\n  initial begin #1; $display(\"T n_ro1 bound=%0d L=%0d\", $bits(w), L); $finish; end\nendmodule\n", "E3009");
    loud("module t;\n  parameter X = 4'b110x;\n  wire [(&X)+2:0] w;\n  localparam L = (&X) + 2;\n  initial begin #1; $display(\"T u_ra0 bound=%0d L=%0d\", $bits(w), L); $finish; end\nendmodule\n", "E3009");
    loud("module t;\n  \n  wire [(~&(~4'b110x))+2:0] w;\n  localparam L = (~&(~4'b110x)) + 2;\n  initial begin #1; $display(\"T x_redn bound=%0d L=%0d\", $bits(w), L); $finish; end\nendmodule\n", "E3009");
}

/// Round-2 lens cells. A signed x-bearing side whose MSB is KNOWN sign-extends before the
/// equality (`4'sb1x10 == 8'sh00` is 0: `1111_1x10` differs on a known bit); one whose
/// MSB is itself x declines (`4'sbx110 == 8'sh06` — both oracles x). `!W` and `W ? :`
/// over a 128-bit parameter name fold through the same truth (the i64 walk cannot hold
/// `W`; PRE was loud). A replication and a shift carry the x bit to a definite consumer.
/// The size cast `128'((|4'b000x))` is `0…0x` (PRE loud; the first cut x-extended it —
/// `resize_bits` replicates an unknown top bit even for an UNSIGNED value, which is
/// §11.6.1's zero-extension in both oracles, so `extend_bits` now fills the zeros at the
/// cast, the literal initializer and the declared-width binders: `localparam logic
/// [127:0] Z = 4'bz001;` is `0…0z` in both oracles and was `zz…z` on PRE). The cells
/// whose answer is x stay loud: an ambiguous ternary condition (even with equal arms —
/// §11.4.11's merge is not folded), `!=` with every known bit equal, a replication of
/// `4'b000x`, `$isunknown` over the x bit (the i64 walk has no arm for it) and a
/// constant-function call with an x argument (the interpreter lane).
#[test]
fn round_two_signed_extension_wide_names_casts_and_the_honest_loud_cells() {
    assert!(run("module t;\n  \n  localparam bit [3:0] L = (&4'b110x) + 2;\n  initial begin #1; $display(\"T r_b2 L=%0d\", L); $finish; end\nendmodule\n").contains("T r_b2 L=2"));
    assert!(run("module t;\n  parameter logic [127:0] W = 128'h1 << 100;\n  localparam L = (!W) + 2;\n  initial begin #1; $display(\"T r_nw L=%0d\", L); $finish; end\nendmodule\n").contains("T r_nw L=2"));
    assert!(run("module t;\n  parameter logic [127:0] W = 128'h1 << 100;\n  localparam L = (W ? 4'd9 : 4'd5) + 2;\n  initial begin #1; $display(\"T r_tw L=%0d\", L); $finish; end\nendmodule\n").contains("T r_tw L=11"));
    assert!(run("module t;\n  \n  localparam L = (|{2{4'b100x}}) + 2;\n  initial begin #1; $display(\"T r_rep1 L=%0d\", L); $finish; end\nendmodule\n").contains("T r_rep1 L=3"));
    assert!(run("module t;\n  \n  localparam L = (|((|4'b000x) << 1)) + 2;\n  initial begin #1; $display(\"T r_shd L=%0d\", L); $finish; end\nendmodule\n").contains("T r_shd L=2"));
    assert!(run("module t;\n  \n  localparam L = (4'sb1x10 == 8'sh00) + 2;\n  initial begin #1; $display(\"T r_sx L=%0d\", L); $finish; end\nendmodule\n").contains("T r_sx L=2"));
    assert!(run("module t;\n  \n  localparam logic [127:0] L = 128'((|4'b000x));\n  initial begin #1; $display(\"T r_cast L=%h\", L); $finish; end\nendmodule\n").contains("T r_cast L=0000000000000000000000000000000X"));
    assert!(run("module t;\n  localparam logic [127:0] L = 128'((|4'b000x));\n  initial begin #1; $display(\"T v_cast L=%h\", L); $finish; end\nendmodule\n").contains("T v_cast L=0000000000000000000000000000000X"));
    assert!(run("module t;\n  localparam logic [127:0] L = 4'bz001;\n  initial begin #1; $display(\"T v_litz L=%h\", L); $finish; end\nendmodule\n").contains("T v_litz L=0000000000000000000000000000000Z"));
    assert!(run("module t;\n  localparam logic [127:0] L = 4'b000x;\n  initial begin #1; $display(\"T v_lit L=%h\", L); $finish; end\nendmodule\n").contains("T v_lit L=0000000000000000000000000000000X"));
    assert!(run("module t;\n  localparam logic [127:0] L = 8'(4'b000x);\n  initial begin #1; $display(\"T v_cast8 L=%h\", L); $finish; end\nendmodule\n").contains("T v_cast8 L=0000000000000000000000000000000X"));
    loud("module t;\n  \n  localparam L = (4'b000x ? 4'd5 : 4'd5) + 2;\n  initial begin #1; $display(\"T r_teq L=%0d\", L); $finish; end\nendmodule\n", "E3009");
    loud("module t;\n  \n  localparam L = 8'((|4'b000x)) + 2;\n  initial begin #1; $display(\"T r_cast8 L=%0d\", L); $finish; end\nendmodule\n", "E3009");
    loud("module t;\n  \n  localparam L = (4'b110x != 4'b1100) + 2;\n  initial begin #1; $display(\"T r_nex L=%0d\", L); $finish; end\nendmodule\n", "E3009");
    loud("module t;\n  \n  localparam L = (|{2{4'b000x}}) + 2;\n  initial begin #1; $display(\"T r_rep L=%0d\", L); $finish; end\nendmodule\n", "E3009");
    loud("module t;\n  \n  localparam L = (|({1'b0,(|4'b000x)} << 1)) + 2;\n  initial begin #1; $display(\"T r_shd2 L=%0d\", L); $finish; end\nendmodule\n", "E3009");
    loud("module t;\n  \n  localparam L = (4'sbx110 == 8'sh06) + 2;\n  initial begin #1; $display(\"T r_sx2 L=%0d\", L); $finish; end\nendmodule\n", "E3009");
    loud("module t;\n  \n  localparam L = $isunknown((|4'b000x)) + 2;\n  initial begin #1; $display(\"T r_isu L=%0d\", L); $finish; end\nendmodule\n", "E3009");
    loud("module t;\n  function automatic int f(input logic [3:0] a); return (!a) + 2; endfunction\n  localparam L = f(4'b101x);\n  initial begin #1; $display(\"T r_fn L=%0d\", L); $finish; end\nendmodule\n", "E3009");
    loud("module t;\n  function automatic int g(input logic [3:0] a); return (|a) + 2; endfunction\n  localparam L = g(4'b101x);\n  initial begin #1; $display(\"T r_fn2 L=%0d\", L); $finish; end\nendmodule\n", "E3009");
}
