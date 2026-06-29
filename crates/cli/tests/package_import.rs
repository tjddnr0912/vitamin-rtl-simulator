//! v7 P2-D packages — IR-0 elaborate flattening (the interface precedent):
//! imported params/enum-labels bind as scoped constants, package
//! functions/tasks clone into the module's inline tables, explicit `pkg::sym`
//! folds through the package const map. iverilog 13.0 live pins (probe t15):
//! a LOCAL declaration wins over an import; `p::W` sees the package value
//! even when shadowed locally.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_pkg_{}_{n}", std::process::id()));
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
fn import_star_params_labels_funcs_and_shadow() {
    // iverilog-pinned (t15): W=99 L2=16 g=5 b=6 dbl=42 pW=8.
    let (out, err, code) = run("package p;\n\
           parameter W = 8;\n\
           localparam L2 = W * 2;\n\
           typedef enum { RED, GREEN = 5, BLUE } color_t;\n\
           function integer dbl(input integer x);\n\
             dbl = x * 2;\n\
           endfunction\n\
         endpackage\n\
         import p::*;\n\
         module top;\n\
           parameter W = 99;\n\
           integer x;\n\
           initial begin\n\
             x = dbl(21);\n\
             $display(\"W=%0d L2=%0d g=%0d b=%0d dbl=%0d pW=%0d\", W, L2, GREEN, BLUE, x, p::W);\n\
             $finish;\n\
           end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(
        out.contains("W=99 L2=16 g=5 b=6 dbl=42 pW=8"),
        "got:\n{out}"
    );
}

#[test]
fn module_scope_single_symbol_import() {
    let (out, err, code) = run("package p;\n\
           parameter A = 3;\n\
           parameter B = 4;\n\
         endpackage\n\
         module top;\n\
           import p::A;\n\
           initial begin\n\
             $display(\"a=%0d\", A);\n\
             $finish;\n\
           end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("a=3"), "got:\n{out}");
}

#[test]
fn imported_param_feeds_widths_and_const_contexts() {
    // imported W drives a range spec AND a localparam fold.
    let (out, err, code) = run("package p;\n\
           parameter W = 8;\n\
         endpackage\n\
         module top;\n\
           import p::*;\n\
           reg [W-1:0] v;\n\
           localparam HALF = p::W / 2;\n\
           initial begin\n\
             v = {W{1'b1}};\n\
             $display(\"v=%h half=%0d bits=%0d\", v, HALF, $bits(v));\n\
             $finish;\n\
           end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("v=ff half=4 bits=8"), "got:\n{out}");
}

#[test]
fn unknown_package_and_symbol_are_loud() {
    let (_o, err, code) = run("module top;\n\
           import nopkg::*;\n\
           initial $finish;\n\
         endmodule\n");
    assert_ne!(code, Some(0));
    assert!(err.contains("E3009"), "stderr:\n{err}");
    let (_o2, err2, code2) = run("package p;\n\
           parameter A = 1;\n\
         endpackage\n\
         module top;\n\
           import p::NOPE;\n\
           initial $finish;\n\
         endmodule\n");
    assert_ne!(code2, Some(0));
    assert!(err2.contains("E3009"), "stderr:\n{err2}");
}

#[test]
fn package_typedef_alias_usable_in_module() {
    // type-name visibility rides the parser's unit-global typedef map.
    let (out, err, code) = run("package p;\n\
           typedef logic [7:0] byte_t;\n\
         endpackage\n\
         import p::*;\n\
         module top;\n\
           byte_t b;\n\
           initial begin\n\
             b = 8'ha5;\n\
             $display(\"b=%h w=%0d\", b, $bits(b));\n\
             $finish;\n\
           end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("b=a5 w=8"), "got:\n{out}");
}
