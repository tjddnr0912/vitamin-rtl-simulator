//! `parameter type T = <type>` — a TYPE parameter (IEEE 1800 §6.20.3) on a module /
//! interface header, in a body, at the compilation-unit scope, and its instance
//! overrides (named, positional, a typedef, `pkg::t`, a pass-through `.T(T)`).
//! ROADMAP §3 ⑤ / §2 🆕 L ⓥ (§4.5.437). Was E2002 "expected '=' in parameter,
//! found identifier 'T'" for every cell.
//!
//! The parser desugars a type parameter to two value parameters — `T$w` (the
//! width) and `T$s` (the shape: signed / 2-state) — and a symbolic-width typedef
//! `T` = `logic [T$w-1:0]` of the default's kind and signedness; `$bits(T)` is
//! `T$w`, `T'(e)` is the size cast at `T$w` with `T`'s signedness (which also
//! closes the §4.5.434 residue `$bits(t)` / `t'(e)` on a module typedef whose range
//! names a header parameter). An override may change the WIDTH; one that changes
//! the shape (signedness / 2-state kind, which the module's declarations cannot
//! follow) is refused loudly by a synthesized `initial if (T$s != …) $fatal`.
//!
//! Every value here was measured on iverilog 13.0 AND verilator 5.050 (39 cells);
//! the lines are the oracles' output, copied (iverilog refuses a `.T(T)`
//! pass-through and a bare `type T` header — those cells are verilator's).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_backend(src: &str, backend: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_tprm_{}_{n}", std::process::id()));
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

fn prints_all(src: &str, want: &[&str]) {
    for b in ["native", "interp", "vm"] {
        let (out, code) = run_backend(src, b);
        assert_eq!(code, Some(0), "[{b}] exit\n{out}");
        let got: Vec<&str> = out.lines().filter(|l| l.starts_with("D=")).collect();
        assert_eq!(got, want, "[{b}]\n{out}");
    }
}

fn is_loud(src: &str, needle: &str) {
    let (out, code) = run_backend(src, "native");
    assert_ne!(code, Some(0), "expected a refusal\n{out}");
    assert!(out.contains(needle), "expected `{needle}` in\n{out}");
}

/// A module `m` with header `hdr`, body `body`, instantiated as `insts` in `top`.
fn design(hdr: &str, body: &str, insts: &str) -> String {
    format!(
        "`timescale 1ns/1ns\nmodule m #({hdr});\n{body}\nendmodule\nmodule top;\n  {insts}\n  initial #5 $finish;\nendmodule\n"
    )
}

#[test]
fn a_type_parameter_declares_ports_and_variables_at_its_width() {
    // ports of type `T`, the default; `$bits` of the port
    prints_all(
        "`timescale 1ns/1ns\nmodule m #(parameter type T = logic [7:0]) (input T d, output T q);\n  assign q = d + 1;\nendmodule\n\
         module top;\n  logic [7:0] d, q;\n  m u(.d(d), .q(q));\n  initial begin d = 8'hfe; #1 $display(\"D=%h %0d\", q, $bits(q)); #5 $finish; end\nendmodule\n",
        &["D=ff 8"],
    );
    // a named override widens the ports; `$bits(T)` inside the module
    prints_all(
        "`timescale 1ns/1ns\nmodule m #(parameter type T = logic [7:0]) (input T d, output T q);\n  assign q = d + 1;\n  initial #2 $display(\"D=%0d\", $bits(T));\nendmodule\n\
         module top;\n  logic [15:0] d, q;\n  m #(.T(logic [15:0])) u(.d(d), .q(q));\n  initial begin d = 16'hfffe; #1 $display(\"D=%h\", q); #5 $finish; end\nendmodule\n",
        &["D=ffff", "D=16"],
    );
    // `T` on a variable, a positional type override with a value after it, an
    // unpacked array of `T`
    prints_all(
        &design(
            "parameter type T = logic [7:0], parameter int N = 2",
            "  T v [N];\n  initial begin v[0] = 8'h11; v[N-1] = 8'h22; #1 $display(\"D=%h %h %0d\", v[0], v[N-1], $bits(T)); end",
            "m #(logic [15:0], 3) u();",
        ),
        &["D=0011 0022 16"],
    );
    // the bare `type T` spelling (verilator; iverilog accepts it too)
    prints_all(
        &design(
            "type T = logic [7:0]",
            "  T v;\n  initial begin v = 8'hab; #1 $display(\"D=%h\", v); end",
            "m u(); m #(.T(logic [11:0])) u2();",
        ),
        &["D=ab", "D=0ab"],
    );
    // two type parameters, and a continuation that inherits `type`
    prints_all(
        &design(
            "parameter type A = int, parameter type B = logic [3:0]",
            "  A a; B b;\n  initial begin a = -1; b = 4'hf; #1 $display(\"D=%0d %h %0d %0d\", a, b, $bits(A), $bits(B)); end",
            "m u1(); m #(.B(logic [7:0])) u2();",
        ),
        &["D=-1 f 32 4", "D=-1 0f 32 8"],
    );
    prints_all(
        &design(
            "parameter type A = byte, B = shortint",
            "  A a; B b;\n  initial begin a = -1; b = -2; #1 $display(\"D=%0d %0d %0d %0d\", a, b, $bits(A), $bits(B)); end",
            "m u1();",
        ),
        &["D=-1 -2 8 16"],
    );
    // an interface header
    prints_all(
        "`timescale 1ns/1ns\ninterface ifc #(parameter type T = logic [7:0]);\n  T d;\nendinterface\n\
         module top;\n  ifc #(.T(logic [3:0])) i();\n  initial begin i.d = '1; #1 $display(\"D=%h\", i.d); #5 $finish; end\nendmodule\n",
        &["D=f"],
    );
}

#[test]
fn the_type_parameter_is_a_type_everywhere_a_typedef_is() {
    // `T'(e)` (truncating), a parameter typed by `T`
    prints_all(
        &design(
            "parameter type T = logic [7:0], parameter T X = T'(300)",
            "  T v;\n  initial begin v = X; #1 $display(\"D=%h %h\", v, X); end",
            "m u1(); m #(.T(logic [3:0])) u2();",
        ),
        &["D=2c 2c", "D=c c"],
    );
    // `$bits(T)` in a localparam and in a range; in an unpacked dimension
    prints_all(
        &design(
            "parameter type T = logic [7:0]",
            "  localparam int W = $bits(T);\n  logic [W-1:0] x;\n  initial begin x = '1; #1 $display(\"D=%h %0d\", x, W); end",
            "m u1(); m #(.T(logic [2:0])) u2();",
        ),
        &["D=ff 8", "D=7 3"],
    );
    prints_all(
        &design(
            "parameter type T = logic [7:0]",
            "  logic [$bits(T)*2-1:0] w;\n  initial begin w = '1; #1 $display(\"D=%h\", w); end",
            "m u1(); m #(.T(logic [1:0])) u2();",
        ),
        &["D=ffff", "D=f"],
    );
    prints_all(
        &design(
            "parameter type T = logic [7:0]",
            "  logic [3:0] mem [0:$bits(T)-1];\n  initial begin mem[$bits(T)-1] = 4'ha; #1 $display(\"D=%h %0d\", mem[$bits(T)-1], $size(mem)); end",
            "m u1(); m #(.T(logic [1:0])) u2();",
        ),
        &["D=a 8", "D=a 2"],
    );
    // a signed default: arithmetic and a compare are signed at every width
    prints_all(
        &design(
            "parameter type T = logic signed [7:0]",
            "  T v;\n  initial begin v = -8'sd100; #1 $display(\"D=%0d %0d\", v, v >>> 2); end",
            "m u1(); m #(.T(logic signed [15:0])) u2();",
        ),
        &["D=-100 -25", "D=-100 -25"],
    );
    prints_all(
        &design(
            "parameter type T = logic signed [7:0]",
            "  T a, b;\n  initial begin a = -1; b = 1; #1 $display(\"D=%0d\", a < b); end",
            "m u1(); m #(.T(logic signed [15:0])) u2();",
        ),
        &["D=1", "D=1"],
    );
    // a struct member of type `T`, a function with `T` formals and return, a
    // chained alias `typedef T u_t;`
    prints_all(
        &design(
            "parameter type T = logic [7:0]",
            "  typedef struct packed { T a; logic b; } s_t;\n  s_t s;\n  initial begin s = '1; #1 $display(\"D=%h %0d\", s.a, $bits(s)); end",
            "m u1(); m #(.T(logic [3:0])) u2();",
        ),
        &["D=ff 9", "D=f 5"],
    );
    prints_all(
        &design(
            "parameter type T = logic [7:0]",
            "  function T inc(input T x); return x + 1; endfunction\n  T v;\n  initial begin v = inc(T'(254)); #1 $display(\"D=%h\", v); end",
            "m u1(); m #(.T(logic [3:0])) u2();",
        ),
        &["D=ff", "D=f"],
    );
    prints_all(
        &design(
            "parameter type T = logic [7:0]",
            "  typedef T u_t;\n  u_t v;\n  initial begin v = '1; #1 $display(\"D=%h\", v); end",
            "m u1(); m #(.T(logic [2:0])) u2();",
        ),
        &["D=ff", "D=7"],
    );
    // a 2-state default: the variable is 2-state (prints 0, not x)
    prints_all(
        &design(
            "parameter type T = bit [3:0]",
            "  T v;\n  initial begin #1 $display(\"D=%h %0d\", v, $bits(T)); end",
            "m u1(); m #(.T(bit [5:0])) u2();",
        ),
        &["D=0 4", "D=00 6"],
    );
    // the 2-state atoms: `int` overridden by `longint` / `byte` (the same shape)
    prints_all(
        &design(
            "parameter type T = int",
            "  T v;\n  initial begin v = -5; #1 $display(\"D=%0d %0d\", v, $bits(T)); end",
            "m u1(); m #(.T(longint)) u2(); m #(.T(byte)) u3();",
        ),
        &["D=-5 32", "D=-5 64", "D=-5 8"],
    );
}

#[test]
fn overrides_by_a_typedef_a_package_type_a_pass_through_and_a_symbolic_default() {
    // a package vector typedef as the override
    prints_all(
        "`timescale 1ns/1ns\npackage p; typedef logic [11:0] w12_t; endpackage\n\
         module m #(parameter type T = logic [7:0]);\n  T v;\n  initial begin v = '1; #1 $display(\"D=%h %0d\", v, $bits(T)); end\nendmodule\n\
         module top;\n  m #(.T(p::w12_t)) u();\n  initial #5 $finish;\nendmodule\n",
        &["D=fff 12"],
    );
    // a module typedef as the override (declared in the instantiating module)
    prints_all(
        "`timescale 1ns/1ns\nmodule top;\n  typedef logic [5:0] s6;\n  m #(.T(s6)) u();\n  initial #5 $finish;\nendmodule\n\
         module m #(parameter type T = logic [7:0]);\n  T v;\n  initial begin v = '1; #1 $display(\"D=%h %0d\", v, $bits(T)); end\nendmodule\n",
        &["D=3f 6"],
    );
    // a pass-through `inner #(.T(T))` (verilator; iverilog refuses the spelling)
    prints_all(
        "`timescale 1ns/1ns\nmodule inner #(parameter type T = logic [7:0]);\n  T v;\n  initial begin v = '1; #1 $display(\"D=%h %0d\", v, $bits(T)); end\nendmodule\n\
         module m #(parameter type T = logic [7:0]);\n  inner #(.T(T)) i();\nendmodule\n\
         module top;\n  m #(.T(logic [5:0])) u();\n  initial #5 $finish;\nendmodule\n",
        &["D=3f 6"],
    );
    // a default that names an earlier header parameter, overridden through either
    prints_all(
        &design(
            "parameter W = 8, parameter type T = logic [W-1:0]",
            "  T v;\n  initial begin v = '1; #1 $display(\"D=%h %0d\", v, $bits(T)); end",
            "m u1(); m #(.W(3)) u2(); m #(.W(3), .T(logic [5:0])) u3();",
        ),
        &["D=ff 8", "D=7 3", "D=3f 6"],
    );
    // a body `localparam type`, and a compilation-unit-scope one
    prints_all(
        "`timescale 1ns/1ns\nmodule top;\n  localparam type T = logic [5:0];\n  T v;\n  initial begin v = '1; #1 $display(\"D=%h %0d\", v, $bits(T)); #5 $finish; end\nendmodule\n",
        &["D=3f 6"],
    );
    prints_all(
        "`timescale 1ns/1ns\nparameter type T = logic [9:0];\nmodule top;\n  T v;\n  initial begin v = '1; #1 $display(\"D=%h %0d\", v, $bits(T)); #5 $finish; end\nendmodule\n",
        &["D=3ff 10"],
    );
}

#[test]
fn the_434_residue_bits_and_cast_of_a_symbolic_width_module_typedef() {
    // `typedef logic [W-1:0] t;` with `W` a header parameter: `$bits(t)` and
    // `t'(e)` fold to the instance's width (was E3009 / E3010)
    prints_all(
        &design(
            "parameter W = 8",
            "  typedef logic [W-1:0] t;\n  t v;\n  initial begin v = '1; #1 $display(\"D=%h %0d\", v, $bits(t)); end",
            "m u1(); m #(.W(3)) u2();",
        ),
        &["D=ff 8", "D=7 3"],
    );
    prints_all(
        &design(
            "parameter W = 8",
            "  typedef logic [W-1:0] t;\n  localparam int B = $bits(t);\n  logic [B*2-1:0] w;\n  initial begin w = t'(-1); #1 $display(\"D=%h B=%0d\", w, B); end",
            "m u1(); m #(.W(3)) u2();",
        ),
        &["D=00ff B=8", "D=07 B=3"],
    );
    prints_all(
        "`timescale 1ns/1ns\nmodule m #(parameter W = 8) (input logic [W-1:0] d, output logic [W-1:0] q);\n  typedef logic [W-1:0] t;\n  t v;\n  assign q = d + 1;\n  initial begin v = t'(300); #1 $display(\"D=%h %h\", v, q); end\nendmodule\n\
         module top;\n  logic [3:0] d, q;\n  m #(.W(4)) u(.d(d), .q(q));\n  initial begin d = 4'he; #5 $finish; end\nendmodule\n",
        &["D=c f"],
    );
}

#[test]
fn a_shape_changing_override_and_a_non_integral_type_are_loud() {
    // signed default, unsigned override (both oracles run it: `156 39`) — refused
    is_loud(
        &design(
            "parameter type T = logic signed [7:0]",
            "  T v;\n  initial begin v = -8'sd100; #1 $display(\"D=%0d %0d\", v, v >>> 2); end",
            "m #(.T(logic [7:0])) u2();",
        ),
        "the override changes the type's signedness or 2-state kind",
    );
    // 4-state default, 2-state override
    is_loud(
        &design(
            "parameter type T = logic [7:0]",
            "  T v;\n  initial begin #1 $display(\"D=%h\", v); end",
            "m #(.T(bit [7:0])) u2();",
        ),
        "the override changes the type's signedness or 2-state kind",
    );
    // a struct / enum / real default, a struct override, a multi-dimensional
    // packed default: outside the integral vector subset (parse errors)
    is_loud(
        "`timescale 1ns/1ns\ntypedef struct packed { logic [3:0] a; logic [3:0] b; } st_t;\nmodule m #(parameter type T = st_t);\n  T v;\nendmodule\nmodule top; m u(); endmodule\n",
        "an integral type as the type parameter's default",
    );
    is_loud(
        "`timescale 1ns/1ns\ntypedef enum logic [1:0] {A=0, B=1, C=2} e_t;\nmodule m #(parameter type T = e_t);\n  T v;\nendmodule\nmodule top; m u(); endmodule\n",
        "an integral type as the type parameter's default",
    );
    is_loud(
        "`timescale 1ns/1ns\nmodule m #(parameter type T = real);\n  T v;\nendmodule\nmodule top; m u(); endmodule\n",
        "an integral type as the type parameter's default",
    );
    is_loud(
        "`timescale 1ns/1ns\ntypedef struct packed { logic [3:0] a; logic [3:0] b; } st_t;\nmodule m #(parameter type T = logic [7:0]);\n  T v;\nendmodule\nmodule top; m #(.T(st_t)) u(); endmodule\n",
        "an integral type as the type parameter override",
    );
    is_loud(
        "`timescale 1ns/1ns\nmodule m #(parameter type T = logic [3:0][7:0]);\n  T v;\nendmodule\nmodule top; m u(); endmodule\n",
        "an integral type as the type parameter's default",
    );
}
