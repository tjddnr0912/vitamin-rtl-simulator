//! §4.5.599: an integral value converted to real reads each x/z bit as 0, adds its set
//! bits least significant first, and keeps its width and sign — in every lane.
//!
//! IEEE 1800-2017 §6.12.2: "Individual bits that are x or z in the net or the variable
//! shall be treated as zero upon conversion." The engine's conversion (`Value::to_f64`)
//! answered nothing for any x/z bit and for a value past 128 bits, and every caller read
//! that as 0.0, so `r = a;` with `a = 4'bx011` stored 0.0 where iverilog 13.0 and sv2v
//! 0.0.13 → iverilog store 3.0. The same reading sat under the real operators' integral
//! operand, a ternary's integral arm, `$itor` / `real'()`, the real math functions,
//! `$rtoi`, `%f`, a port or a subroutine formal of type `real`; `$realtobits` alone
//! converted per bit (`Value::integral_to_f64`). Every lane now reads the one conversion,
//! `sim_ir::mw::int_to_real`, which the elaborate-time constant domain reads too.
//!
//! The conversion is iverilog's algorithm, not correct rounding: the set bits are added
//! LSB first in double precision, for a constant and at run time alike. A variable
//! holding `64'hC000_0000_0000_0401` converts to 13835058055282163712.0 in iverilog 13.0
//! and sv2v → iverilog; verilator 5.052 rounds it correctly at run time
//! (13835058055282165760.0, which every vita lane printed) and agrees with iverilog only
//! on a constant, which it folds at compile time — a split, and vita follows iverilog.
//! The elaborate-time constant domain
//! read its i64 fold as signed, so an unsigned 64-bit constant with the top bit set was
//! negative (`localparam real R = 64'hC000_0000_0000_0401;` -4611686018427386880.0, and
//! `generate if (64'hC000_0000_0000_0401 > 1.0e19)` took the `else`), and a constant
//! past 64 bits converted the i64 wrap (`65'd5 - 65'd7` -2.0).
//!
//! Oracles: iverilog 13.0 (`-g2012`) is the x/z oracle and every expected line is its
//! output. verilator 5.052 (`--binary --timing`) zeroes an x assignment whole, so it is
//! no x oracle; on the known-valued cells it prints the same lines, but on the wide ones
//! only because it folds this straight-line `initial` code at compile time — with `#1`
//! before the conversions it prints `st 13835058055282165760.0 …`, `rb
//! 43e8000000000001 …` and `cmp 0 1 1` (correct rounding, vita's old answer). sv2v 0.0.13
//! → iverilog prints iverilog's lines. Splits, each said where it is pinned: a signed
//! negative value with an unknown bit below its sign bit (iverilog propagates the
//! unknown through the negation, `4'sb1z11` is -1.0; verilator and §6.12.2's text read the
//! bit as 0, -5.0 — vita); `$itor` of a wide VARIABLE (iverilog truncates a non-constant
//! argument to 32 bits, `$itor(v)` with `v = 64'hC000_0000_0000_0401` is 1025.0; verilator
//! rounds it correctly, 13835058055282165760.0; both print 13835058055282163712.0 for the
//! literal, as vita does in both forms); the LSB-first rounding of a wide value at run
//! time (iverilog and sv2v → iverilog against verilator's run-time correct rounding,
//! above — vita follows iverilog); and `2^100 + 2^47 + 1` in every lane (iverilog
//! `$realtobits` `4630000000000001`, verilator `4630000000000000`, vita iverilog's).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Run one design through one-shot `vita` with `-Wno-W1017`; exit code and combined
/// output.
fn run(src: &str) -> (Option<i32>, String) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_itr_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("scratch dir");
    std::fs::write(d.join("t.sv"), src).expect("write design");
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("-Wno-W1017")
        .arg("t.sv")
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (out.status.code(), s)
}

/// A clean run printing exactly `want` (diagnostics and status lines dropped).
fn prints(src: &str, want: &[&str], what: &str) {
    let (rc, out) = run(src);
    assert_eq!(rc, Some(0), "{what}: expected exit 0:\n{out}");
    assert!(
        !out.contains("[VITA-"),
        "{what}: no diagnostic expected:\n{out}"
    );
    let got: Vec<&str> = out
        .lines()
        .filter(|l| !l.starts_with("simulation ended") && !l.starts_with("errors="))
        .collect();
    assert_eq!(got, want, "{what}:\n{out}");
}

// ── the run-time lanes ─────────────────────────────────────────────────────────────

/// The assignment conversion (`coerce_assign`): an x/z bit is 0, a signed value whose
/// sign bit is unknown reads unsigned, an all-unknown value is 0.0.
#[test]
fn a_store_reads_each_unknown_bit_as_zero() {
    prints(
        "module t;
  logic [3:0] a; logic signed [3:0] sa; integer n; real r, rs, rn;
  initial begin
    a = 4'bx011; sa = 4'sbx111; n = {28'b0, 4'bx101};
    r = a; rs = sa; rn = n;
    $display(\"s1 r=%f rs=%f rn=%f\", r, rs, rn);
    a = 4'b1z11; n = 32'bz;
    r = a; rn = n;
    $display(\"s2 r=%f rn=%f\", r, rn);
    a = 4'bxxxx; sa = 4'sbzzzz; n = -32'sd5;
    r = a; rs = sa; rn = n;
    $display(\"s3 r=%f rs=%f rn=%f\", r, rs, rn);
  end
endmodule
",
        &[
            "s1 r=3.000000 rs=7.000000 rn=5.000000",
            "s2 r=11.000000 rn=0.000000",
            "s3 r=0.000000 rs=0.000000 rn=-5.000000",
        ],
        "store",
    );
}

/// The split: a signed negative value with an unknown bit below the sign bit. §6.12.2's
/// text and verilator read the bit as 0 (`1011` = -5.0); iverilog carries the unknown
/// through the negation and prints -1.0.
#[test]
fn a_negative_value_with_an_unknown_bit_follows_the_text() {
    prints(
        "module t;
  logic signed [3:0] sa; real rs;
  initial begin sa = 4'sb1z11; rs = sa; $display(\"rs=%f\", rs); end
endmodule
",
        &["rs=-5.000000"],
        "signed split",
    );
}

/// A real operator's integral operand, a comparison with a real, a ternary's integral
/// arm, and a negation of a converted value — each reads the x bit as 0. (iverilog
/// refuses `===` with a real operand, so it is not here.)
#[test]
fn an_operator_converts_its_integral_operand_per_bit() {
    prints(
        "module t;
  logic [3:0] a; logic signed [3:0] sa; logic s; real r, x1, x2, x3, x4, x5; bit b1, b2, b3, b4, b6, b7;
  initial begin
    r = 2.5; s = 0;
    a = 4'bx011; sa = 4'sbx101;
    x1 = a + 1.5; x2 = r * a; x3 = a - r; x4 = 10.0 / a; x5 = s ? 1.5 : a;
    b1 = (a < r); b2 = (a > r); b3 = (a == 3.0); b4 = (a != 3.0); b6 = (a <= 3.0); b7 = (a >= 3.0);
    $display(\"m1 %f %f %f %f %f | %b %b %b %b %b %b\", x1, x2, x3, x4, x5, b1, b2, b3, b4, b6, b7);
    $display(\"m2 %f %f %f %f\", sa + 0.0, -(a + 0.0), a ** 2.0, 2.0 ** a);
    a = 4'b1z1z;
    $display(\"m3 %f %b %b\", a + 0.5, a == 10.0, a > 9.5);
  end
endmodule
",
        &[
            "m1 4.500000 7.500000 0.500000 3.333333 3.000000 | 0 1 1 0 1 1",
            "m2 5.000000 -3.000000 9.000000 8.000000",
            "m3 10.500000 1 1",
        ],
        "operators",
    );
}

/// `$itor`, `real'()`, `$sqrt`, `$pow`, `$ln`, `$rtoi` of an integral argument, and
/// `%f` / `%e` / `%g` of an integral value. All three oracles print these lines.
#[test]
fn system_functions_and_formats_convert_per_bit() {
    prints(
        "module t;
  logic [3:0] a; real r2, r3, r4, r5, r6; int i1;
  initial begin
    a = 4'bx011;
    r2 = $itor(a); r3 = real'(a); r4 = $sqrt(a); r5 = $pow(a, 2); r6 = $ln(a); i1 = $rtoi(a);
    $display(\"y1 %f %f %f %f %f %0d\", r2, r3, r4, r5, r6, i1);
    $display(\"y2 %f %e %g\", a, a, a);
    $display(\"y3 %f %f\", $itor(4'sbx111), $sqrt(4'sbx111));
  end
endmodule
",
        &[
            "y1 3.000000 3.000000 1.732051 9.000000 1.098612 3",
            "y2 3.000000 3.000000e+00 3",
            "y3 7.000000 2.645751",
        ],
        "system functions",
    );
}

/// A `real` input port bound to an integral net, and `real` formals of a static
/// function, an automatic function and a task. The port's `always @(x)` now wakes at
/// time 0 and at 1 as in all three oracles: with the whole value read as 0.0 it never
/// changed, and printed neither line.
#[test]
fn ports_and_formals_convert_per_bit() {
    prints(
        "module m(input real x);
  always @(x) $display(\"port %0t x=%f\", $time, x);
endmodule
module t;
  logic [3:0] a; real z, w, v;
  m u(.x(a));
  function real f(input real p); f = p + 0.25; endfunction
  function automatic real g(input real p); return p * 2.0; endfunction
  task automatic tk(input real p, output real q); q = p; endtask
  initial begin
    a = 4'bx011; #1 z = f(a); tk(a, w); v = g(a);
    $display(\"p1 z=%f w=%f v=%f\", z, w, v);
    a = 4'bz1z1; #1 z = f(a); tk(a, w); v = g(a);
    $display(\"p2 z=%f w=%f v=%f\", z, w, v);
  end
endmodule
",
        &[
            "port 0 x=3.000000",
            "p1 z=3.250000 w=3.000000 v=6.000000",
            "port 1 x=5.000000",
            "p2 z=5.250000 w=5.000000 v=10.000000",
        ],
        "ports and formals",
    );
}

/// A real dynamic-array element and a fixed real-array element take the store's
/// conversion.
#[test]
fn array_elements_convert_per_bit() {
    prints(
        "module t;
  logic [3:0] a; real d[]; real ra[2];
  initial begin
    a = 4'd3; d = new[2]; d[0] = a; ra[0] = a;
    $display(\"k1 %f %f\", d[0], ra[0]);
    a = 4'bx011; d[1] = a; ra[1] = a;
    $display(\"k2 %f %f\", d[1], ra[1]);
  end
endmodule
",
        &["k1 3.000000 3.000000", "k2 3.000000 3.000000"],
        "arrays",
    );
}

/// Wide values, every lane, against iverilog byte for byte: an unsigned 64-bit value whose
/// LSB-first sum differs from its correctly rounded value, the same bits signed, 65 and
/// 100 bits, a signed 100-bit negative, a 100-bit value with an x bit, and 200 bits (past
/// the old 128-bit limit, where every lane printed 0.0). `$realtobits` was already right.
/// The `it` line is vita's own (a split): iverilog truncates a non-constant `$itor`
/// argument to 32 bits (`it 1025.0 1025.0 1025.0 1.0 -1900168395.0 1.0 1.0`), and
/// verilator prints this line only because it folds the straight-line `initial` at compile
/// time — with `#1` before the conversions it rounds the 64-bit cell correctly in every
/// lane (`st` / `it` / `ad` / `fm` 13835058055282165760.0, `rb 43e8000000000001`, `cmp 0
/// 1 1`), so on that cell this pin is iverilog's and sv2v → iverilog's, not verilator's.
#[test]
fn wide_values_match_iverilog_in_every_lane() {
    let st = "13835058055282163712.0 -4611686018427386880.0 32281802128991715328.0 \
              633825300114114841485839958016.0 -12345678901234567741440.0 \
              633825300114114841485839958016.0 \
              803469022129495137770981046170581301261101496891396417650688.0";
    let lines: Vec<String> = ["st", "it", "ad", "fm"]
        .iter()
        .map(|t| format!("{t} {st}"))
        .chain([
            "rb 43e8000000000000 4620000000000001 4c60000000000000".to_string(),
            "sq 3.719551e+09 7.961315e+14".to_string(),
            "cmp 1 0 1".to_string(),
        ])
        .collect();
    let want: Vec<&str> = lines.iter().map(String::as_str).collect();
    prints(
        "module t;
  logic [63:0] u64; logic signed [63:0] s64; logic [64:0] u65; logic [99:0] u100; logic signed [99:0] s100;
  logic [99:0] x100; logic [199:0] u200;
  real r1, r2, r3, r4, r5, r6, r7;
  initial begin
    u64 = 64'hC000_0000_0000_0401; s64 = 64'shC000_0000_0000_0401;
    u65 = 65'h1_C000_0000_0000_0401; u100 = 100'h8_0000_0000_0000_8000_0000_0001;
    s100 = -100'sd12345678901234567890123; x100 = 100'h8_0000_0000_00x0_8000_0000_0001;
    u200 = {1'b1, 52'b0, 1'b1, 73'b0, 1'b1, 72'b0} | 200'd1;
    r1 = u64; r2 = s64; r3 = u65; r4 = u100; r5 = s100; r6 = x100; r7 = u200;
    $display(\"st %.1f %.1f %.1f %.1f %.1f %.1f %.1f\", r1, r2, r3, r4, r5, r6, r7);
    $display(\"it %.1f %.1f %.1f %.1f %.1f %.1f %.1f\", $itor(u64), $itor(s64), $itor(u65), $itor(u100), $itor(s100), $itor(x100), $itor(u200));
    $display(\"ad %.1f %.1f %.1f %.1f %.1f %.1f %.1f\", u64 + 0.0, s64 + 0.0, u65 + 0.0, u100 + 0.0, s100 + 0.0, x100 + 0.0, u200 + 0.0);
    $display(\"fm %.1f %.1f %.1f %.1f %.1f %.1f %.1f\", u64, s64, u65, u100, s100, x100, u200);
    $display(\"rb %h %h %h\", $realtobits(u64), $realtobits(u100), $realtobits(u200));
    $display(\"sq %.6e %.6e\", $sqrt(u64), $sqrt(u100));
    $display(\"cmp %b %b %b\", u64 == 13835058055282163712.0, u64 > 13835058055282163712.0, u100 == 633825300114114841485839958016.0);
  end
endmodule
",
        &want,
        "wide",
    );
}

// ── the elaborate-time constant lanes ──────────────────────────────────────────────

/// A declared-real parameter's integral initializer, a real-free operand of a real
/// operator, a string, an unsigned constant-function return, an unsigned primitive cast,
/// overrides (named, `defparam`), a generate condition and an `int'()` over a real
/// expression. iverilog and sv2v → iverilog print these lines; verilator prints them
/// too, but refuses the `time'()` cast in a constant ("can't convert a CAST to
/// constant").
#[test]
fn constants_convert_at_their_own_width_and_sign() {
    prints(
        "package pk; localparam [63:0] PQ = 64'hC000_0000_0000_0401; localparam real PR = PQ; endpackage
module c #(parameter real R = 1.0) (); initial #1 $display(\"ovr %m R=%.1f\", R); endmodule
module t;
  localparam [63:0] Q = 64'hC000_0000_0000_0401;
  localparam real P1 = 64'hC000_0000_0000_0401;
  localparam real P2 = 64'shC000_0000_0000_0401;
  localparam real P3 = 64'hC000_0000_0000_0401 + 0.0;
  localparam real P4 = \"\\xC0\\x00\\x00\\x00\\x00\\x00\\x04\\x01\";
  localparam real P5 = Q;
  localparam real P6 = 1.0 * Q;
  localparam real P7 = pk::PQ;
  localparam real P8 = pk::PR;
  localparam real P9 = 65'd5 - 65'd7;
  localparam real P10 = 128'h8000_0000_0000_0000_0000_0000_0000_0401;
  localparam real P11 = {64'hC000_0000_0000_0000, 8'h01};
  function automatic logic [63:0] f(input int a); return 64'hC000_0000_0000_0400 + a; endfunction
  localparam real P12 = f(1);
  localparam int I1 = int'(64'hC000_0000_0000_0401 * 1.0 / 1.0e18);
  localparam NM = -5;
  localparam real P13 = time'(NM) + 0.0;
  if (64'hC000_0000_0000_0401 > 1.0e19) begin : g1 initial $display(\"gen then\"); end
  else begin : g2 initial $display(\"gen else\"); end
  c #(.R(64'hC000_0000_0000_0401)) u1();
  c u2();
  defparam u2.R = Q;
  initial $display(\"c %.1f %.1f %.1f %.1f %.1f %.1f\", P1, P2, P3, P4, P5, P6);
  initial $display(\"d %.1f %.1f %.1f %.1f %.1f %.1f %0d %.1f\", P7, P8, P9, P10, P11, P12, I1, P13);
endmodule
",
        &[
            "gen then",
            "c 13835058055282163712.0 -4611686018427386880.0 13835058055282163712.0 \
             13835058055282163712.0 13835058055282163712.0 13835058055282163712.0",
            "d 13835058055282163712.0 13835058055282163712.0 36893488147419103232.0 \
             170141183460469231731687303715884105728.0 3541774862152233910272.0 \
             13835058055282163712.0 14 18446744073709551616.0",
            "ovr t.u1 R=13835058055282163712.0",
            "ovr t.u2 R=13835058055282163712.0",
        ],
        "constants",
    );
}

/// A real parameter bound from an unsigned 64-bit constant with the top bit set decides a
/// generate condition (the soundness lens's n4, verbatim). The parameter read
/// -4611686018427386880.0 and the design elaborated the `else` block, at exit 0; iverilog
/// 13.0, verilator 5.052 and sv2v → iverilog all print `gen=pos`.
#[test]
fn a_real_parameter_of_an_unsigned_constant_decides_a_generate() {
    prints(
        "module top; localparam real P = 64'hC000_0000_0000_0000; if (P > 0.0) begin : g initial $display(\"gen=pos\"); end else begin : h initial $display(\"gen=neg\"); end endmodule\n",
        &["gen=pos"],
        "n4",
    );
}

/// A constant with an x/z bit stays loud in the constant domain, as before: §6.12.2 reads
/// the bit as 0 and the oracles agree (`localparam real R = 4'bx011` is 3.0), but this
/// domain's other readers decline an unknown and the run-time lane is the one that
/// answers it.
#[test]
fn an_unknown_constant_stays_loud() {
    let (rc, out) = run("module t;
  localparam real R = 4'bx011;
  initial $display(\"%f\", R);
endmodule
");
    assert_eq!(rc, Some(1), "{out}");
    assert!(
        out.contains("error[VITA-E3009]") && out.contains("4'bx011 has no constant-fold arm"),
        "{out}"
    );
}
