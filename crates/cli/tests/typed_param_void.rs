//! A1 (Tier-ⓐ close): typed `parameter int W` / `localparam logic [3:0] X`
//! and `function void`. Pure lexer/parser extension (IR-0). Oracle = iverilog
//! 13.0 for typed params; void-function-as-statement is SV semantics (iverilog
//! parity where applicable).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_tpv_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    )
}

#[test]
fn typed_parameter_int_binds_and_folds_as_width() {
    // `parameter int W = 5` must fold to 5 and drive a packed width.
    let (out, err, code) = run("module top #(parameter int W = 5) ();\n\
         reg [W-1:0] r;\n\
         initial begin r = {W{1'b1}}; $display(\"W=%0d r=%0d\", W, r); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("W=5 r=31"), "got:\n{out}");
}

#[test]
fn typed_localparam_logic_vec_binds_value() {
    // `localparam logic [3:0] X = 4'hA` must bind X = 10.
    let (out, err, code) = run("module top;\n\
         localparam logic [3:0] X = 4'hA;\n\
         initial begin $display(\"X=%0d\", X); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("X=10"), "got:\n{out}");
}

#[test]
fn typed_parameter_byte_coerces_out_of_range_value() {
    // IEEE §6.20: a typed/ranged param coerces its value to the declared width+sign.
    // `byte B = 200` (signed 8-bit) -> -56; `byte = 8'hA5` -> -91. iverilog parity.
    let (out, err, code) = run("module top;\n\
         parameter byte B = 200;\n\
         parameter byte H = 8'hA5;\n\
         parameter shortint S = 65543;\n\
         initial begin $display(\"B=%0d H=%0d S=%0d\", B, H, S); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("B=-56 H=-91 S=7"), "got:\n{out}");
}

#[test]
fn ranged_parameter_signedness_coercion() {
    // The same coercion fixes the pre-existing untyped path: `signed [7:0] = 200`
    // -> -56, while `unsigned [7:0] = 200` -> 200, and `int W = -5` stays -5.
    let (out, err, code) = run("module top;\n\
         parameter signed [7:0] C = 200;\n\
         parameter [7:0] U = 200;\n\
         parameter int W = -5;\n\
         initial begin $display(\"C=%0d U=%0d W=%0d\", C, U, W); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("C=-56 U=200 W=-5"), "got:\n{out}");
}

#[test]
fn typed_parameter_byte_is_signed_8bit() {
    // `parameter byte B = -3` binds as 8-bit signed -3.
    let (out, err, code) = run("module top #(parameter byte B = -3) ();\n\
         initial begin $display(\"B=%0d\", B); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("B=-3"), "got:\n{out}");
}

#[test]
fn function_void_called_as_statement_with_output() {
    // `function void f(input, output)` invoked as a statement, side effect via
    // the output formal. The void return slot is discarded.
    let (out, err, code) = run("module top;\n\
         integer y;\n\
         function void addone(input integer a, output integer r);\n\
           r = a + 1;\n\
         endfunction\n\
         initial begin addone(5, y); $display(\"y=%0d\", y); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("y=6"), "got:\n{out}");
}

#[test]
fn class_void_method_with_keyword_parses_and_discards() {
    // `function void` inside a class: a frame-function whose result is discarded
    // at the statement call site. The void keyword now parses in class scope.
    let (out, err, code) = run("class C;\n\
           int v;\n\
           function void set(int x);\n\
             v = x;\n\
           endfunction\n\
         endclass\n\
         module top; C c;\n\
         initial begin\n\
           c = new;\n\
           c.set(7);\n\
           $display(\"v=%0d\", c.v);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("v=7"), "got:\n{out}");
}

#[test]
fn function_void_parses_with_empty_ports() {
    // `function void f();` no-arg, writes an outer var, called twice.
    let (out, err, code) = run("module top;\n\
         integer c;\n\
         function void bump();\n\
           c = c + 10;\n\
         endfunction\n\
         initial begin c = 0; bump(); bump(); $display(\"c=%0d\", c); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("c=20"), "got:\n{out}");
}
