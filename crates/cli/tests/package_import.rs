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

#[test]
fn explicit_single_symbol_type_import_binds() {
    // §4.5.148 — an explicit `import p::<type>` (scalar type + packed struct).
    // Before: E3009 "package `p` has no symbol `byte_t`" — apply_import_consts
    // only knew consts/vars/funcs/tasks, so it rejected a legal type import even
    // though `import p::*` and a bare `p::byte_t` both already resolved (the
    // parser copies the scoped type twin at parse; elaborate binds nothing for
    // a type import — it must merely stop erroring). iverilog-13.0 pins x=ab,
    // pv=35 (0x23 packed = a:2 b:3 → 35).
    let (out, err, code) = run("package p;\n\
           typedef logic [7:0] byte_t;\n\
           typedef struct packed { logic [3:0] a; logic [3:0] b; } pair_t;\n\
         endpackage\n\
         module top;\n\
           import p::byte_t;\n\
           import p::pair_t;\n\
           byte_t x;\n\
           pair_t pv;\n\
           initial begin\n\
             x = 8'hab;\n\
             pv = 8'h23;\n\
             $display(\"x=%h pv=%0d\", x, pv);\n\
             $finish;\n\
           end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("x=ab pv=35"), "got:\n{out}");
}

#[test]
fn explicit_enum_type_import_binds() {
    // The type-name collection runs for EVERY typedef kind, including the
    // special-cased enum branch. Importing the enum TYPE name binds the type;
    // the label is reached via `p::Y` (importing a type does not import its
    // literals). iverilog-13.0 pins ev=1 (X=0, Y=1, Z=2).
    let (out, err, code) = run("package p;\n\
           typedef enum logic [1:0] { X, Y, Z } e_t;\n\
         endpackage\n\
         module top;\n\
           import p::e_t;\n\
           e_t ev;\n\
           initial begin\n\
             ev = p::Y;\n\
             $display(\"ev=%0d\", ev);\n\
             $finish;\n\
           end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("ev=1"), "got:\n{out}");
}

#[test]
fn explicit_type_import_keeps_unknown_symbol_loud() {
    // The type-name recognition must NOT become a blanket accept: a genuinely
    // absent symbol is still E3009 (over-accept guard — pins the exact boundary
    // the fix draws).
    let (_o, err, code) = run("package p;\n\
           typedef logic [7:0] byte_t;\n\
         endpackage\n\
         module top;\n\
           import p::nonexistent_xyz;\n\
           initial $finish;\n\
         endmodule\n");
    assert_ne!(code, Some(0));
    assert!(err.contains("E3009"), "stderr:\n{err}");
}

#[test]
fn package_enum_label_at_i64_max_wraps_like_module_scope() {
    // The package enum auto-increment used an unchecked `v + 1` (its module-scope
    // and body-local twins already `wrapping_add`) — an explicit label at
    // i64::MAX was a debug-build overflow PANIC. iverilog: B wraps to
    // 0x8000_0000_0000_0000.
    let (out, err, code) = run("package p;\n\
           typedef enum logic [63:0] { A = 64'sh7FFF_FFFF_FFFF_FFFF, B } e_t;\n\
         endpackage\n\
         module top;\n\
           import p::*;\n\
           e_t e;\n\
           initial begin e = B; $display(\"%h\", e); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("8000000000000000"), "got:\n{out}");
}
