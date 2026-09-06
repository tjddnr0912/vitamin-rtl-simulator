//! §3 ⑤ ⓕ: an ANSI header `import` (`module m import p::*; #(parameter T X = Dflt)`,
//! IEEE §A.1.2 / §26.4) and a compilation-unit import are visible to the header's
//! own parameter defaults and ranges — every ibex module header is this shape.
//! Elaborate applies those imports BEFORE `bind_params`, a body import after
//! (a header default naming a body import's constant is an oracle split and stays
//! loud). Also: the interface twin, an explicit import colliding with a local
//! declaration (loud, §26.3), a DECLARED parameter range that does not fold
//! (loud through `check_const_range_bound` — was silently 32 bits), and a scoped
//! typedef whose dims name the package's own constants (§2 🆕 L ⓟ).
//!
//! Every expected value is the grounding-probe oracle line (iverilog 13.0 `-g2012`
//! and verilator 5.050 agree unless the comment says which one ran).

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_/Users_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (s, out.status.code())
}

/// Every `DIGEST=` line, sorted, joined by `|` (the census harness format; sorted
/// because two instances print in scheduling order).
fn digest(name: &str, src: &str, expect: &str) {
    let (out, rc) = run(src);
    assert_eq!(rc, Some(0), "{name}: expected exit 0, got {rc:?}:\n{out}");
    let mut v: Vec<&str> = out
        .lines()
        .filter_map(|l| l.strip_prefix("DIGEST="))
        .collect();
    v.sort_unstable();
    assert_eq!(v.join("|"), expect, "{name}:\n{out}");
}

fn loud(name: &str, src: &str, needle: &str) {
    let (out, rc) = run(src);
    assert_ne!(rc, Some(0), "{name}: expected a loud reject:\n{out}");
    assert!(
        out.contains(needle),
        "{name}: expected `{needle}` in:\n{out}"
    );
}

#[test]
fn edge_f01() {
    // f01_scalar: both oracles
    digest(
        "f01_scalar",
        r#"package p; parameter logic [7:0] Dflt = 8'hA5; endpackage
module m import p::*; #(parameter logic [7:0] X = Dflt) (); initial $display("DIGEST=%h", X); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
"#,
        "a5",
    );
}

#[test]
fn edge_f02() {
    // f02_typed: both oracles
    digest(
        "f02_typed",
        r#"package p; typedef logic [7:0] perm_t; parameter perm_t Dflt = 8'h3C; endpackage
module m import p::*; #(parameter perm_t X = Dflt) (); initial $display("DIGEST=%h", X); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
"#,
        "3c",
    );
}

#[test]
fn edge_f03() {
    // f03_untyped: both oracles
    digest(
        "f03_untyped",
        r#"package p; parameter int Dflt = 37; endpackage
module m import p::*; #(parameter X = Dflt) (); initial $display("DIGEST=%0d", X); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
"#,
        "37",
    );
}

#[test]
fn edge_f04() {
    // f04_explicit: both oracles
    digest(
        "f04_explicit",
        r#"package p; parameter int Dflt = 37; parameter int Other = 9; endpackage
module m import p::Dflt; #(parameter X = Dflt) (); initial $display("DIGEST=%0d", X); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
"#,
        "37",
    );
}

#[test]
fn edge_f05() {
    // f05_override: both oracles
    digest(
        "f05_override",
        r#"package p; parameter int Dflt = 37; endpackage
module m import p::*; #(parameter X = Dflt) (); initial $display("DIGEST=%0d", X); endmodule
module tb; m u(); m #(.X(5)) v(); initial begin #1 $finish; end endmodule
"#,
        "37|5",
    );
}

#[test]
fn edge_f06() {
    // f06_expr: both oracles
    digest(
        "f06_expr",
        r#"package p; parameter int W = 4; endpackage
module m import p::*; #(parameter int X = W*2+1, parameter logic [W-1:0] Y = '1) (); initial $display("DIGEST=%0d %h %0d", X, Y, $bits(Y)); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
"#,
        "9 f 4",
    );
}

#[test]
fn edge_f07() {
    // f07_port: both oracles
    digest(
        "f07_port",
        r#"package p; parameter int W = 4; parameter logic [3:0] D = 4'hC; endpackage
module m import p::*; #(parameter logic [W-1:0] X = D) (input logic [W-1:0] a); initial $display("DIGEST=%h %h", X, a); endmodule
module tb; logic [3:0] a = 4'h7; m u(a); initial begin #1 $finish; end endmodule
"#,
        "c 7",
    );
}

#[test]
fn edge_f08() {
    // f08_body_ctrl: both oracles
    digest(
        "f08_body_ctrl",
        r#"package p; parameter int Dflt = 37; endpackage
module m #(parameter X = 1) (); import p::*; localparam Y = Dflt; initial $display("DIGEST=%0d %0d", X, Y); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
"#,
        "1 37",
    );
}

#[test]
fn edge_f09() {
    // f09_two_pkgs: both oracles
    digest(
        "f09_two_pkgs",
        r#"package p; parameter int A = 3; endpackage
package q; parameter int B = 4; endpackage
module m import p::*; import q::*; #(parameter X = A + B) (); initial $display("DIGEST=%0d", X); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
"#,
        "7",
    );
}

#[test]
fn edge_f10() {
    // f10_comma: both oracles
    digest(
        "f10_comma",
        r#"package p; parameter int A = 3; endpackage
package q; parameter int B = 4; endpackage
module m import p::*, q::B; #(parameter X = A + B) (); initial $display("DIGEST=%0d", X); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
"#,
        "7",
    );
}

#[test]
fn edge_f11() {
    // f11_typedef_param: both oracles — §4.5.437: `parameter type` is parsed; the typed parameter folds through it (oracle output copied)
    digest(
        "f11_typedef_param",
        r#"package p; typedef logic [7:0] perm_t; parameter perm_t Dflt = 8'h3C; endpackage
module m import p::*; #(parameter type T = perm_t, parameter T X = Dflt) (); initial $display("DIGEST=%h %0d", X, $bits(T)); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
"#,
        "3c 8",
    );
}

#[test]
fn edge_f12() {
    // f12_local_shadow: both oracles
    digest(
        "f12_local_shadow",
        r#"package p; parameter int Dflt = 37; endpackage
module m import p::*; #(parameter Dflt = 5, parameter X = Dflt) (); initial $display("DIGEST=%0d %0d", Dflt, X); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
"#,
        "5 5",
    );
}

#[test]
fn edge_f13() {
    // f13_struct_dflt: verilator — verilator only (iverilog rejects a keyed pattern in a package parameter)
    digest(
        "f13_struct_dflt",
        r#"package p; typedef struct packed { logic [3:0] a; logic [3:0] b; } s_t; parameter s_t Dflt = '{a:4'h1, b:4'h2}; endpackage
module m import p::*; #(parameter s_t X = Dflt) (); initial $display("DIGEST=%h %h", X, X.b); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
"#,
        "12 2",
    );
}

#[test]
fn edge_f14() {
    // f14_enum_dflt: verilator — verilator only (iverilog rejects the enum-typed package parameter default)
    digest(
        "f14_enum_dflt",
        r#"package p; typedef enum logic [1:0] {E0, E1, E2} e_t; parameter e_t Dflt = E2; endpackage
module m import p::*; #(parameter e_t X = Dflt) (); initial $display("DIGEST=%0d %s", X, X.name()); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
"#,
        "2 E2",
    );
}

#[test]
fn edge_f15() {
    // f15_nested_inst: both oracles
    digest(
        "f15_nested_inst",
        r#"package p; parameter int Dflt = 37; endpackage
module c import p::*; #(parameter X = Dflt) (); initial $display("DIGEST=%0d", X); endmodule
module m import p::*; #(parameter Y = Dflt + 1) (); c u1(); c #(.X(Y)) u2(); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
"#,
        "37|38",
    );
}

#[test]
fn edge_f16() {
    // f16_scoped_ctrl: both oracles
    digest(
        "f16_scoped_ctrl",
        r#"package p; parameter int Dflt = 37; endpackage
module m #(parameter X = p::Dflt) (); initial $display("DIGEST=%0d", X); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
"#,
        "37",
    );
}

#[test]
fn edge_f17() {
    // f17_array_param: verilator — verilator only (iverilog: unpacked array parameters are not supported)
    digest(
        "f17_array_param",
        r#"package p; parameter int Dflt = 37; parameter int R[2] = '{1,2}; endpackage
module m import p::*; #(parameter int A[2] = R, parameter X = Dflt) (); initial $display("DIGEST=%0d %0d %0d", A[0], A[1], X); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
"#,
        "1 2 37",
    );
}

#[test]
fn edge_f18() {
    // f18_interface: both oracles
    digest(
        "f18_interface",
        r#"package p; parameter int W = 4; endpackage
interface i import p::*; #(parameter int N = W) (); logic [N-1:0] d; endinterface
module tb; i u(); initial begin u.d = 4'hb; #1 $display("DIGEST=%h %0d", u.d, u.N); $finish; end endmodule
"#,
        "b 4",
    );
}

#[test]
fn edge_f19() {
    // f19_localparam_hdr: both oracles
    digest(
        "f19_localparam_hdr",
        r#"package p; parameter int Dflt = 37; endpackage
module m import p::*; #(parameter X = 2, localparam Y = Dflt * X) (); initial $display("DIGEST=%0d", Y); endmodule
module tb; m u(); m #(.X(3)) v(); initial begin #1 $finish; end endmodule
"#,
        "111|74",
    );
}

#[test]
fn edge_f20() {
    // f20_ansi_no_hash: both oracles
    digest(
        "f20_ansi_no_hash",
        r#"package p; parameter int W = 4; endpackage
module m import p::*; (input logic [W-1:0] a); initial $display("DIGEST=%h", a); endmodule
module tb; logic [3:0] a = 4'h7; m u(a); initial begin #1 $finish; end endmodule
"#,
        "7",
    );
}

#[test]
fn edge_f21() {
    // f21_body_import: verilator — a BODY import is not visible to the header default (iverilog rejects; verilator folds) — loud on the LRM side (§26.3: an import is visible from its own position on)
    loud(
        "f21_body_import",
        r#"package p; parameter int Dflt = 37; endpackage
module m #(parameter X = Dflt) (); import p::*; initial $display("DIGEST=%0d", X); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
"#,
        "parameter `X` value is not a constant: undefined name `Dflt` is not a ",
    );
}

#[test]
fn edge_f22() {
    // f22_cu_import: both oracles
    digest(
        "f22_cu_import",
        r#"package p; parameter int Dflt = 37; endpackage
import p::*;
module m #(parameter X = Dflt) (); initial $display("DIGEST=%0d", X); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
"#,
        "37",
    );
}

#[test]
fn edge_f23() {
    // f23_explicit_collide: verilator — IEEE §26.3: an explicit import of a name the scope declares is an error (iverilog rejects; verilator answers the local) — loud since §4.5.415 (was the silent §2 🆕 L ⓘ)
    loud(
        "f23_explicit_collide",
        r#"package p; parameter int Dflt = 37; endpackage
module m import p::Dflt; #(parameter Dflt = 5, parameter X = Dflt) (); initial $display("DIGEST=%0d", X); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
"#,
        "`Dflt` has already been imported into this scope (explicit import from",
    );
}

#[test]
fn edge_f24() {
    // f24_ambiguous: verilator — IEEE §26.8: a name two wildcards export is ambiguous when referenced (iverilog rejects; verilator takes the first) — loud
    loud(
        "f24_ambiguous",
        r#"package p; parameter int D = 3; endpackage
package q; parameter int D = 4; endpackage
module m import p::*; import q::*; #(parameter X = D) (); initial $display("DIGEST=%0d", X); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
"#,
        "parameter `X` value is not a constant: undefined name `D` is not a con",
    );
}

#[test]
fn edge_f25() {
    // f25_enum_label: both oracles
    digest(
        "f25_enum_label",
        r#"package p; typedef enum logic [1:0] {E0, E1, E2} e_t; endpackage
module m import p::*; #(parameter e_t X = E1, parameter int Y = E2) (); initial $display("DIGEST=%0d %0d", X, Y); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
"#,
        "1 2",
    );
}

#[test]
fn edge_f26() {
    // f26_pkg_func: both oracles 10 — §4.5.440 (§2 🆕 L ⓦ): a package FUNCTION in a
    // header default folds through the constant-function interpreter (was a loud
    // wording pin, "no constant-fold arm").
    digest(
        "f26_pkg_func",
        r#"package p; function automatic int dbl(int a); return a*2; endfunction parameter int W = 5; endpackage
module m import p::*; #(parameter int X = dbl(W)) (); initial $display("DIGEST=%0d", X); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
"#,
        "10",
    );
}

#[test]
fn edge_f27() {
    // f27_sysfn: both oracles
    digest(
        "f27_sysfn",
        r#"package p; typedef logic [11:0] perm_t; parameter int N = 20; endpackage
module m import p::*; #(parameter int A = $bits(perm_t), parameter int B = $clog2(N)) (); initial $display("DIGEST=%0d %0d", A, B); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
"#,
        "12 5",
    );
}

#[test]
fn edge_f28() {
    // f28_generate: both oracles
    digest(
        "f28_generate",
        r#"package p; parameter int N = 3; endpackage
module m import p::*; #(parameter int K = N) (); logic [K-1:0] v; genvar i; generate for (i = 0; i < K; i++) begin : g assign v[i] = i[0]; end endgenerate initial begin #1 $display("DIGEST=%b", v); end endmodule
module tb; m u(); initial begin #2 $finish; end endmodule
"#,
        "010",
    );
}

#[test]
fn edge_f29() {
    // f29_string: verilator — PRE-EXISTING: a string package parameter is not on the wildcard channel (§2 🆕 L ⓕ); iverilog rejects it too
    loud(
        "f29_string",
        r#"package p; parameter string S = "hi"; endpackage
module m import p::*; #(parameter string X = S) (); initial $display("DIGEST=%s", X); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
"#,
        "parameter `X` value is not a constant: undefined name `S` is not a con",
    );
}

#[test]
fn edge_f30() {
    // f30_real: both oracles — PRE-EXISTING: a real package parameter is not on the import channel (loud; §2 🆕 L ⓕ family)
    loud(
        "f30_real",
        r#"package p; parameter real R = 2.5; endpackage
module m import p::*; #(parameter real X = R) (); initial $display("DIGEST=%f", X); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
"#,
        "parameter `X` value is not a constant: undefined name `R` is not a con",
    );
}

#[test]
fn edge_f31() {
    // f31_wide: both oracles
    digest(
        "f31_wide",
        r#"package p; parameter logic [79:0] Wd = 80'hABCD_0000_0000_0000_1234; endpackage
module m import p::*; #(parameter logic [79:0] X = Wd) (); initial $display("DIGEST=%h", X); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
"#,
        "abcd0000000000001234",
    );
}

#[test]
fn edge_f32() {
    // f32_type_cast: both oracles — §4.5.437: `parameter type` is parsed; the typed parameter folds through it (oracle output copied)
    digest(
        "f32_type_cast",
        r#"package p; typedef logic [7:0] perm_t; parameter int Dflt = 300; endpackage
module m import p::*; #(parameter type T = perm_t, parameter T X = T'(Dflt)) (); initial $display("DIGEST=%h", X); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
"#,
        "2c",
    );
}

#[test]
fn edge_f33() {
    // f33_nonansi: both oracles
    digest(
        "f33_nonansi",
        r#"package p; parameter int Dflt = 37; endpackage
module m import p::*; #(parameter X = Dflt) (a); input a; initial $display("DIGEST=%0d %b", X, a); endmodule
module tb; m u(1'b1); initial begin #1 $finish; end endmodule
"#,
        "37 1",
    );
}

#[test]
fn edge_f34() {
    // f34_pkg_var: both oracles
    digest(
        "f34_pkg_var",
        r#"package p; parameter int Dflt = 37; logic [7:0] V = 8'h5A; endpackage
module m import p::*; #(parameter X = Dflt) (); initial $display("DIGEST=%0d %h", X, V); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
"#,
        "37 5a",
    );
}

#[test]
fn edge_f35() {
    // f35_override_uses_import: both oracles
    digest(
        "f35_override_uses_import",
        r#"package p; parameter int Dflt = 37; parameter int Alt = 9; endpackage
module c #(parameter X = 1) (); initial $display("DIGEST=%0d", X); endmodule
module m import p::*; #(parameter Y = Dflt) (); c #(.X(Alt)) u1(); c #(.X(Y)) u2(); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
"#,
        "37|9",
    );
}

#[test]
fn edge_f36() {
    // f36_dflt_pkg_derived: both oracles
    digest(
        "f36_dflt_pkg_derived",
        r#"package p; parameter int W = 4; parameter logic [W-1:0] Mask = '1; localparam int H = W/2; endpackage
module m import p::*; #(parameter logic [W-1:0] X = Mask, parameter int Y = H) (); initial $display("DIGEST=%h %0d", X, Y); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
"#,
        "f 2",
    );
}

#[test]
fn edge_f37() {
    // f37_iface_body_import: both oracles
    digest(
        "f37_iface_body_import",
        r#"package p; parameter int W = 4; endpackage
interface i #(parameter int N = 4) (); import p::*; localparam int M = W + N; logic [M-1:0] d; endinterface
module tb; i u(); initial begin u.d = 8'hb3; #1 $display("DIGEST=%h %0d", u.d, u.M); $finish; end endmodule
"#,
        "b3 8",
    );
}

#[test]
fn edge_f38() {
    // f38_iface_body_to_hdr: verilator — interface twin of f21 — loud on the LRM side
    loud(
        "f38_iface_body_to_hdr",
        r#"package p; parameter int W = 4; endpackage
interface i #(parameter int N = W) (); import p::*; logic [N-1:0] d; endinterface
module tb; i u(); initial begin u.d = 4'hb; #1 $display("DIGEST=%h", u.d); $finish; end endmodule
"#,
        "parameter `N` value is not a constant: undefined name `W` is not a con",
    );
}

#[test]
fn edge_f39() {
    // f39_iface_func_import: both oracles — an interface body has no functions, so a routine an import brings in has no caller — a call stays loud (correct-or-loud), the constants bind
    loud(
        "f39_iface_func_import",
        r#"package p; function automatic int dbl(int a); return a*2; endfunction parameter int W = 4; endpackage
interface i import p::*; #(parameter int N = W) (); logic [N-1:0] d; initial begin d = dbl(3); end endinterface
module tb; i u(); initial begin #1 $display("DIGEST=%h", u.d); $finish; end endmodule
"#,
        "call to undeclared function `dbl`",
    );
}

#[test]
fn edge_f40() {
    // f40_type_param_ctrl: both oracles — §4.5.437: `parameter type` is parsed; the typed parameter folds through it (oracle output copied)
    digest(
        "f40_type_param_ctrl",
        r#"module m #(parameter type T = logic [7:0], parameter T X = 8'h2c) (); initial $display("DIGEST=%h", X); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
"#,
        "2c",
    );
}

#[test]
fn edge_f42() {
    // f42_undef_range: verilator — PRE-EXISTING silent-wrong fixed: a header parameter whose DECLARED range does not fold went value-inferred (`$bits` 32 at exit 0 where both oracles refuse) — loud through `check_const_range_bound`
    loud(
        "f42_undef_range",
        r#"module m #(parameter logic [Nope-1:0] X = 4'h9) (); initial $display("DIGEST=%h %0d", X, $bits(X)); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
"#,
        "undefined name `Nope` is not allowed in a constant range bound",
    );
}

#[test]
fn edge_f43() {
    // f43_undef_range_pos: verilator — the same, every position: port and net were loud already, the body localparam was not
    loud(
        "f43_undef_range_pos",
        r#"module m #(parameter int N = 4) (input logic [Nope1-1:0] a); localparam logic [Nope2-1:0] Y = 4'h9; logic [Nope3-1:0] v; initial begin v = '1; $display("DIGEST=%0d %0d %0d %0d", $bits(a), $bits(Y), $bits(v), $bits(m.a)); end endmodule
module tb; logic [3:0] a = 4'h7; m u(a); initial begin #1 $finish; end endmodule
"#,
        "undefined name `Nope1` is not allowed in a constant range bound",
    );
}

#[test]
fn edge_f44() {
    // f44_undef_range_pkg_gen: verilator — the same, the package-scope and generate-scope binders
    loud(
        "f44_undef_range_pkg_gen",
        r#"package p; parameter logic [Nope1-1:0] P = 4'h9; endpackage
module m (); generate if (1) begin : g localparam logic [Nope2-1:0] L = 4'h9; initial $display("DIGEST=%0d", $bits(L)); end endgenerate initial $display("DIGEST=%0d", $bits(p::P)); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
"#,
        "undefined name `Nope1` is not allowed in a constant range bound",
    );
}

#[test]
fn edge_f45() {
    // f45_pmd: verilator — the `packed_md_param` shape that a loud declared range must NOT refuse — `p::perm_t` dims fold through the respelled twin
    digest(
        "f45_pmd",
        r#"package p;
  parameter int W = 4;
  typedef logic [W-1:0][$clog2(W)-1:0] perm_t;
  parameter perm_t Dflt = {8'h1b};
endpackage
module lfsr #(parameter int Dw = 4, parameter int Iw = 2, parameter logic [Dw-1:0][Iw-1:0] SP = '0) (input logic [Dw-1:0] i, output logic [Dw-1:0] o);
  for (genvar k = 0; k < Dw; k++) begin : g
    assign o[k] = i[SP[k]];
  end
endmodule
module top #(parameter p::perm_t Perm = p::Dflt) (input logic [3:0] i, output logic [3:0] o);
  lfsr #(.Dw(p::W), .Iw($clog2(p::W)), .SP(Perm)) u (.i(i), .o(o));
endmodule
module tb;
  logic [3:0] i = 4'b0110, o, o2;
  top u (.i(i), .o(o));
  top #(.Perm({2'd3,2'd3,2'd0,2'd0})) u2 (.i(i), .o(o2));
  initial begin #1 $display("DIGEST=%b %b", o, o2); #1 $finish; end
endmodule
"#,
        "0110 0000",
    );
}

#[test]
fn edge_f46() {
    // f46_scoped_typedef_dims: both oracles — §2 🆕 L ⓟ fixed: the scoped typedef twin re-spells the package's own constants as `p::W` at endpackage
    digest(
        "f46_scoped_typedef_dims",
        r#"package p; parameter int W = 4; typedef logic [W-1:0] t; typedef logic [W-1:0][1:0] t2; endpackage
module m; p::t v; p::t2 w; initial begin v = '1; w = '1; $display("DIGEST=%h %0d %0d", v, $bits(v), $bits(w)); end endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
"#,
        "f 4 8",
    );
}

#[test]
fn edge_f47() {
    // f47_type_import_local_var: verilator — IEEE §26.3, the TYPE-import spelling of f23 (iverilog rejects; verilator answers the local) — loud
    loud(
        "f47_type_import_local_var",
        r#"package r; typedef struct packed { logic [11:0] a; logic [3:0] b; } rs_t; endpackage
module tb; typedef struct packed { logic [3:0] a; logic [3:0] b; } ls_t; ls_t rs_t; import r::rs_t;
  initial begin rs_t = 8'h34; $display("DIGEST=%h %h", rs_t.a, rs_t.b); #1 $finish; end
endmodule
"#,
        "`rs_t` has already been imported into this scope (explicit import from",
    );
}

/// Review pins (§4.5.415, lenses A and B): every cell is the oracle line, or a loud
/// pin where the LRM side of an oracle split is loud.
#[test]
fn review_pins() {
    // B F1: the non-overridden module-BODY parameter binder is a reduced copy that did
    // not call the declared-range gate (both oracles refuse the design).
    loud(
        "b_f1_body_param_range",
        "module tb;\n  parameter logic [Nope-1:0] X = 4'h9;\n  initial begin $display(\"DIGEST=%0d %0d\", $bits(X), X); #1 $finish; end\nendmodule\n",
        "undefined name `Nope` is not allowed in a constant range bound",
    );
    // B F2: inside a package a wildcard import never binds a name the package declares
    // itself (both oracles 9; vita read q's 5).
    digest(
        "b_f2_pkg_own_decl_shadows_wildcard",
        "package q; parameter int K = 5; endpackage\npackage p; parameter int K = 9; import q::*; parameter int USE = K; endpackage\nmodule tb; initial begin $display(\"DIGEST=%0d\", p::USE); #1 $finish; end endmodule\n",
        "9",
    );
    // B F2 explicit twin: loud (iverilog "'K' has already been imported"; verilator 9).
    loud(
        "b_f2_pkg_explicit_collision",
        "package q; parameter int K = 5; endpackage\npackage p; import q::K; parameter int K = 9; parameter int USE = K; endpackage\nmodule tb; initial begin $display(\"DIGEST=%0d\", p::USE); #1 $finish; end endmodule\n",
        "already been imported",
    );
    // B F3 / A F2: a package typedef dim naming a constant the package IMPORTED carries
    // its value in the `p::t` twin (both oracles 6; vita read the importer's local 12).
    digest(
        "b_f3_pkg_typedef_dim_imported_const",
        "package q; parameter int QW = 6; endpackage\npackage p; import q::*; typedef logic [QW-1:0] t; endpackage\nmodule tb; localparam int QW = 12; p::t v;\n  initial begin v='1; $display(\"DIGEST=%0d\", $bits(v)); #1 $finish; end\nendmodule\n",
        "6",
    );
    digest(
        "a_f2_pkg_typedef_dim_imported_header",
        "package q; parameter int QW = 5; endpackage\npackage p; import q::*; typedef logic [QW-1:0] t; endpackage\nmodule m #(parameter p::t X = 5'h1b) (); initial $display(\"DIGEST=%h %0d\", X, $bits(X)); endmodule\nmodule tb; m u(); initial begin #1 $finish; end endmodule\n",
        "1b 5",
    );
    // A F1: a compilation-unit EXPLICIT import is an outer scope — a local declaration
    // shadows it in silence (both oracles 5), unlike the same-scope collision.
    digest(
        "a_f1_cu_explicit_shadowed_by_local",
        "package p; parameter int Dflt = 37; endpackage\nimport p::Dflt;\nmodule tb; localparam int Dflt = 5; initial begin $display(\"DIGEST=%0d\", Dflt); #1 $finish; end endmodule\n",
        "5",
    );
    digest(
        "a_f1_cu_explicit_shadowed_by_header_param",
        "package p; parameter int Dflt = 37; endpackage\nimport p::Dflt;\nmodule m #(parameter int Dflt = 5, parameter int X = Dflt) (); initial $display(\"DIGEST=%0d\", X); endmodule\nmodule tb; m u(); initial begin #1 $finish; end endmodule\n",
        "5",
    );
    // A F3: a real parameter in a body parameter's declared range — loud (verilator
    // refuses; iverilog 9 4; vita was silently 32 bits).
    loud(
        "a_f3_real_in_body_param_range",
        "module m (); parameter real R = 4.0; parameter logic [R-1:0] X = 4'h9; initial $display(\"DIGEST=%h %0d\", X, $bits(X)); endmodule\nmodule tb; m u(); initial begin #1 $finish; end endmodule\n",
        "a real parameter is not an integral constant",
    );
    // A a33 / B F2: a package explicitly importing a name it declares — loud (iverilog;
    // verilator 5).
    loud(
        "a33_pkg_self_explicit_import",
        "package q; parameter int W = 3; endpackage\npackage p; import q::W; parameter int W = 5; endpackage\nmodule tb; initial begin $display(\"DIGEST=%0d\", p::W); #1 $finish; end endmodule\n",
        "already been imported",
    );
}
