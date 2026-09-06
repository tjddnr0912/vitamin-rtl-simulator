//! Compilation-unit-scope declarations (IEEE §3.12.1): a `typedef`, a
//! `localparam` / `parameter`, a `function` or a `task` written OUTSIDE any module
//! is visible to every module, interface, package and class that follows it in
//! the unit — ROADMAP §3 (§4.5.434).
//!
//! vita refused the file at its first token (`expected 'module', found keyword
//! 'typedef'`) where iverilog 13.0 and verilator 5.050 both run it. The parser now
//! registers the type / constant under the unit scope as it parses (a later module
//! resolves the bare name) and replicates the item into the body of every later
//! module (constants only into an interface, whose elaboration refuses the other
//! kinds), minus the names the module declares itself — so elaborate binds it
//! exactly as a module-local declaration would be.
//!
//! Every value here was measured on iverilog 13.0 AND verilator 5.050 (42 cells);
//! the lines are the oracles' output, copied. A struct assignment pattern is
//! verilator-only (iverilog refuses `'{…}` on a typedef'd struct).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_cuscope_{}_{n}", std::process::id()));
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

fn prints(src: &str, want: &[&str]) {
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "exit\n{out}");
    let got: Vec<&str> = out.lines().filter(|l| l.starts_with("D=")).collect();
    assert_eq!(got, want, "\n{out}");
}

fn loud(src: &str, code_text: &str) {
    let (out, code) = run(src);
    assert_ne!(code, Some(0), "expected a refusal\n{out}");
    assert!(out.contains(code_text), "expected {code_text}\n{out}");
    assert!(!out.lines().any(|l| l.starts_with("D=")), "\n{out}");
}

const TS: &str = "`timescale 1ns/1ns\n";

#[test]
fn a_unit_scope_typedef_is_visible_to_the_modules_after_it() {
    // s01 · a vector alias; s14 · an atom; s17 · signed; s06 · a chained alias
    prints(
        &format!("{TS}typedef logic [3:0] t4;\nmodule top; t4 x = 4'd13; initial begin #1 $display(\"D=%0d %0d\", x, $bits(x)); #5 $finish; end endmodule\n"),
        &["D=13 4"],
    );
    prints(
        &format!("{TS}typedef int myint;\nmodule top; myint x = -5; initial begin #1 $display(\"D=%0d %0d\", x, $bits(x)); #5 $finish; end endmodule\n"),
        &["D=-5 32"],
    );
    prints(
        &format!("{TS}typedef logic signed [3:0] s4;\nmodule top; s4 x = -3; initial begin #1 $display(\"D=%0d\", x); #5 $finish; end endmodule\n"),
        &["D=-3"],
    );
    prints(
        &format!("{TS}typedef logic [3:0] t4;\ntypedef t4 t4b;\nmodule top; t4b x = 4'd7; initial begin #1 $display(\"D=%0d %0d\", x, $bits(t4b)); #5 $finish; end endmodule\n"),
        &["D=7 4"],
    );
    // s04 / s16 / s33 · two modules, a port, a second typedef between two modules
    prints(
        &format!("{TS}typedef logic [7:0] byte_t;\nmodule ch(output byte_t o); assign o = 8'hA5; endmodule\nmodule top; byte_t w; ch u(w); initial begin #1 $display(\"D=%h %0d\", w, $bits(byte_t)); #5 $finish; end endmodule\n"),
        &["D=a5 8"],
    );
    prints(
        &format!("{TS}typedef logic [3:0] t4;\nmodule ch; t4 y = 4'd3; initial #1 $display(\"D=%0d\", y); endmodule\nmodule top; t4 x = 4'd13; ch u(); initial begin #1 $display(\"D=%0d\", x); #5 $finish; end endmodule\n"),
        &["D=13", "D=3"],
    );
    prints(
        &format!("{TS}typedef logic [3:0] t4;\nmodule a; t4 x = 4'd1; initial #1 $display(\"D=%0d\", x); endmodule\ntypedef logic [7:0] t8;\nmodule top; t8 y = 8'd200; a u(); initial begin #1 $display(\"D=%0d\", y); #5 $finish; end endmodule\n"),
        &["D=200", "D=1"],
    );
    // s39 / s40 / s23 · an unpacked array of the type, a function typed by it, a
    // header parameter typed by it
    prints(
        &format!("{TS}typedef logic [3:0] t4;\nmodule top; t4 arr [0:1]; initial begin #1 arr[1] = 4'd9; $display(\"D=%0d %0d\", arr[1], $bits(arr[0])); #5 $finish; end endmodule\n"),
        &["D=9 4"],
    );
    prints(
        &format!("{TS}typedef logic [3:0] t4;\nmodule top; function t4 f(t4 a); return a + 4'd1; endfunction initial begin #1 $display(\"D=%0d\", f(4'd14)); #5 $finish; end endmodule\n"),
        &["D=15"],
    );
    prints(
        &format!("{TS}typedef logic [7:0] t8;\nmodule top #(parameter t8 P = 8'd200) (); initial begin #1 $display(\"D=%0d\", P); #5 $finish; end endmodule\n"),
        &["D=200"],
    );
}

#[test]
fn a_unit_scope_struct_union_and_enum_typedef() {
    // s02 / s20 · a packed struct, nested (verilator `9 2 26` / `9 2 1`)
    prints(
        &format!("{TS}typedef struct packed {{ logic [3:0] a; logic [1:0] b; }} st_t;\nmodule top; st_t s; initial begin #1 s = '{{a: 4'd9, b: 2'd2}}; $display(\"D=%0d %0d %h\", s.a, s.b, s); #5 $finish; end endmodule\n"),
        &["D=9 2 26"],
    );
    prints(
        &format!("{TS}typedef struct packed {{ logic [3:0] a; logic [1:0] b; }} st_t;\ntypedef struct packed {{ st_t s; logic c; }} outer_t;\nmodule top; outer_t o; initial begin #1 o = '{{s: '{{a: 4'd9, b: 2'd2}}, c: 1'b1}}; $display(\"D=%0d %0d %b\", o.s.a, o.s.b, o.c); #5 $finish; end endmodule\n"),
        &["D=9 2 1"],
    );
    // s12 · a struct port (verilator `9`)
    prints(
        &format!("{TS}typedef struct packed {{ logic [3:0] a; logic [1:0] b; }} st_t;\nmodule ch(input st_t i, output logic [3:0] o); assign o = i.a; endmodule\nmodule top; st_t s; logic [3:0] o; ch u(s, o); initial begin #1 s = '{{a: 4'd9, b: 2'd2}}; #1 $display(\"D=%0d\", o); #5 $finish; end endmodule\n"),
        &["D=9"],
    );
    // s07 · a packed union (both oracles `a 5`)
    prints(
        &format!("{TS}typedef union packed {{ logic [7:0] b; logic [1:0][3:0] n; }} u_t;\nmodule top; u_t u; initial begin #1 u.b = 8'hA5; $display(\"D=%h %h\", u.n[1], u.n[0]); #5 $finish; end endmodule\n"),
        &["D=a 5"],
    );
    // s03 / s24 / s37 / s18 · an enum: labels, `$bits`, `.name()`/`.next()`, a
    // cast, beside a module-local enum
    prints(
        &format!("{TS}typedef enum logic [1:0] {{ RED, GREEN, BLUE }} col_t;\nmodule top; col_t c = BLUE; initial begin #1 $display(\"D=%0d %0d %0d\", c, GREEN, $bits(c)); #5 $finish; end endmodule\n"),
        &["D=2 1 2"],
    );
    prints(
        &format!("{TS}typedef enum logic [1:0] {{ RED, GREEN, BLUE }} col_t;\nmodule top; col_t c = BLUE; initial begin #1 $display(\"D=%s %0d\", c.name(), c.next()); #5 $finish; end endmodule\n"),
        &["D=BLUE 0"],
    );
    prints(
        &format!("{TS}typedef enum logic [1:0] {{ RED, GREEN, BLUE }} col_t;\nmodule top; col_t c; initial begin #1 c = col_t'(2); $display(\"D=%s %0d\", c.name(), c); #5 $finish; end endmodule\n"),
        &["D=BLUE 2"],
    );
    prints(
        &format!("{TS}typedef enum logic [1:0] {{ RED, GREEN, BLUE }} col_t;\nmodule top; typedef enum {{ RED2, GREEN2 }} c2_t; col_t c = GREEN; c2_t d = GREEN2; initial begin #1 $display(\"D=%0d %0d\", c, d); #5 $finish; end endmodule\n"),
        &["D=1 1"],
    );
}

#[test]
fn a_unit_scope_constant_function_and_task() {
    // s08 / s31 / s32 · `localparam` (a list too); a unit-scope `parameter` is a
    // localparam (§6.20.1)
    prints(
        &format!("{TS}localparam int CW = 6;\nmodule top; logic [CW-1:0] x = 6'd63; initial begin #1 $display(\"D=%0d %0d\", x, CW); #5 $finish; end endmodule\n"),
        &["D=63 6"],
    );
    prints(
        &format!("{TS}localparam int A = 3, B = 5;\nmodule top; initial begin #1 $display(\"D=%0d %0d\", A, B); #5 $finish; end endmodule\n"),
        &["D=3 5"],
    );
    prints(
        &format!("{TS}parameter int Q = 9;\nmodule top; initial begin #1 $display(\"D=%0d\", Q); #5 $finish; end endmodule\n"),
        &["D=9"],
    );
    // s09 / s36 / s35 · a function (also in a constant), a task
    prints(
        &format!("{TS}function automatic int dbl(int n); return n * 2; endfunction\nmodule top; initial begin #1 $display(\"D=%0d\", dbl(21)); #5 $finish; end endmodule\n"),
        &["D=42"],
    );
    prints(
        &format!("{TS}function automatic int dbl(int n); return n * 2; endfunction\nmodule top; localparam int K = dbl(4); logic [K-1:0] x = '1; initial begin #1 $display(\"D=%0d %0d\", K, x); #5 $finish; end endmodule\n"),
        &["D=8 255"],
    );
    prints(
        &format!("{TS}task automatic say(input int n); $display(\"D=%0d\", n); endtask\nmodule top; initial begin #1 say(7); #5 $finish; end endmodule\n"),
        &["D=7"],
    );
}

#[test]
fn a_module_local_declaration_shadows_the_unit_scope_one() {
    // s05 · a module typedef of the same name (both oracles `255 8`); s29 · a
    // module localparam of the same name (`15 4`)
    prints(
        &format!("{TS}typedef logic [3:0] t;\nmodule top; typedef logic [7:0] t; t x = 8'hFF; initial begin #1 $display(\"D=%0d %0d\", x, $bits(t)); #5 $finish; end endmodule\n"),
        &["D=255 8"],
    );
    prints(
        &format!("{TS}localparam int CW = 6;\nmodule top; localparam int CW = 4; logic [CW-1:0] x = 4'd15; initial begin #1 $display(\"D=%0d %0d\", x, CW); #5 $finish; end endmodule\n"),
        &["D=15 4"],
    );
}

#[test]
fn a_unit_scope_type_reaches_a_package_an_interface_and_a_class() {
    // s13 · a package constant typed by it; s27 / s28 · an interface net typed by
    // it, sized by a unit constant, beside a unit function; s22 · a class field
    prints(
        &format!("{TS}typedef logic [3:0] t4;\npackage p; localparam t4 K = 4'd11; endpackage\nmodule top; import p::*; initial begin #1 $display(\"D=%0d\", K); #5 $finish; end endmodule\n"),
        &["D=11"],
    );
    prints(
        &format!("{TS}typedef logic [3:0] t4;\ninterface bus; t4 d; endinterface\nmodule top; bus b(); initial begin #1 b.d = 4'd5; $display(\"D=%0d\", b.d); #5 $finish; end endmodule\n"),
        &["D=5"],
    );
    prints(
        &format!("{TS}localparam int CW = 6;\nfunction automatic int dbl(int n); return n * 2; endfunction\ninterface bus; logic [CW-1:0] d; endinterface\nmodule top; bus b(); initial begin #1 b.d = 6'd33; $display(\"D=%0d %0d\", b.d, dbl(CW)); #5 $finish; end endmodule\n"),
        &["D=33 12"],
    );
    prints(
        &format!("{TS}typedef logic [3:0] t4;\nclass C; t4 v; function new(); v = 4'd6; endfunction endclass\nmodule top; C c; initial begin c = new; #1 $display(\"D=%0d\", c.v); #5 $finish; end endmodule\n"),
        &["D=6"],
    );
}

#[test]
fn a_port_and_an_import_shadow_the_unit_scope_constant() {
    // review B A-1 · an ANSI port / a non-ANSI port named like a unit constant reads
    // the port (both oracles `9`); A-2 · a wildcard and an explicit import of a
    // package constant of the same name read the package's (`7`, IEEE §26.3)
    prints(
        &format!("{TS}parameter a = 5;\nmodule sub(input [3:0] a); initial #1 $display(\"D=%0d\", a); endmodule\nmodule top; wire [3:0] x = 4'd9; sub s(x); initial #3 $finish; endmodule\n"),
        &["D=9"],
    );
    prints(
        &format!("{TS}parameter a = 5;\nmodule sub(a); input [3:0] a; initial #1 $display(\"D=%0d\", a); endmodule\nmodule top; wire [3:0] x = 4'd9; sub s(x); initial #3 $finish; endmodule\n"),
        &["D=9"],
    );
    prints(
        &format!("{TS}localparam K = 5;\npackage p; localparam K = 7; endpackage\nmodule top; import p::*; initial #1 $display(\"D=%0d\", K); initial #3 $finish; endmodule\n"),
        &["D=7"],
    );
    prints(
        &format!("{TS}localparam K = 5;\npackage p; localparam K = 7; endpackage\nmodule top; import p::K; initial #1 $display(\"D=%0d\", K); initial #3 $finish; endmodule\n"),
        &["D=7"],
    );
}

#[test]
fn a_unit_enum_label_and_an_instance_name_yield_to_the_module() {
    // review A A-1 · a module net / variable / localparam / port named like a unit
    // enum LABEL reads the module's own (both oracles `7` / `3` / `9` / `9`)
    let en = "typedef enum logic [1:0] { RED, GRN, BLU } col_e;\n";
    prints(
        &format!("{TS}{en}module top; wire [3:0] BLU; assign BLU = 4'd7;\n  initial begin #1 $display(\"D=%0d\", BLU); #5 $finish; end endmodule\n"),
        &["D=7"],
    );
    prints(
        &format!("{TS}{en}module top; reg [1:0] GRN;\n  initial begin #1 GRN = 2'd3; $display(\"D=%0d\", GRN); #5 $finish; end endmodule\n"),
        &["D=3"],
    );
    prints(
        &format!("{TS}{en}module top; localparam int GRN = 9;\n  initial begin #1 $display(\"D=%0d\", GRN); #5 $finish; end endmodule\n"),
        &["D=9"],
    );
    prints(
        &format!("{TS}{en}module sub(input [3:0] GRN); initial #1 $display(\"D=%0d\", GRN); endmodule\nmodule top; sub u(4'd9); initial #5 $finish; endmodule\n"),
        &["D=9"],
    );
    // review A A-3 · a unit constant named like an INSTANCE is not read in its place
    // (both oracles refuse the read)
    loud(
        &format!("{TS}localparam int u = 7;\nmodule sub; initial #1 $display(\"SUB\"); endmodule\nmodule top; sub u(); initial begin #1 $display(\"D=%0d\", u); #5 $finish; end endmodule\n"),
        "VITA-E3010",
    );
}

#[test]
fn a_wildcard_import_is_nearer_than_the_unit_scope() {
    // review A A-2 · a wildcard-imported typedef / constant / enum label beats the
    // unit's (IEEE §26.3; both oracles `200 8` / `9` / `7`)
    prints(
        &format!("{TS}typedef logic [3:0] t;\npackage p; typedef logic [7:0] t; endpackage\nmodule top; import p::*; t x = 8'd200;\n  initial begin #1 $display(\"D=%0d %0d\", x, $bits(x)); #5 $finish; end endmodule\n"),
        &["D=200 8"],
    );
    prints(
        &format!("{TS}localparam int K = 4;\npackage p; localparam int K = 9; endpackage\nmodule top; import p::*;\n  initial begin #1 $display(\"D=%0d\", K); #5 $finish; end endmodule\n"),
        &["D=9"],
    );
    prints(
        &format!("{TS}typedef enum logic [1:0] {{ RED, GRN, BLU }} cu_e;\npackage p; typedef enum logic [3:0] {{ RED=5, GRN=6, BLU=7 }} pk_e; endpackage\nmodule top; import p::*;\n  initial begin #1 $display(\"D=%0d\", BLU); #5 $finish; end endmodule\n"),
        &["D=7"],
    );
    // a19 control · an EXPLICIT import of the same name (`200 8`)
    prints(
        &format!("{TS}typedef logic [3:0] t;\npackage p; typedef logic [7:0] t; endpackage\nmodule top; import p::t; t x = 8'd200;\n  initial begin #1 $display(\"D=%0d %0d\", x, $bits(x)); #5 $finish; end endmodule\n"),
        &["D=200 8"],
    );
}

#[test]
fn a_typedef_after_its_use_is_refused() {
    // s11 · both oracles refuse (verilator: "Reference to 't4' before declaration")
    loud(
        &format!("{TS}module top; t4 x = 4'd13; initial begin #1 $display(\"D=%0d\", x); #5 $finish; end endmodule\ntypedef logic [3:0] t4;\n"),
        "VITA-E2002",
    );
}
