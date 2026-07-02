//! Numeric cast to a user-defined type name — `mode_e'(raw)` (report §6 B2; IEEE
//! 1800-2017 §6.24.1). vita supported the built-in casts (`int'`, `byte'`, `N'`,
//! `signed'`/`unsigned'`) but a typedef-name cast was loud (E3009). iverilog 13.0
//! supports it.
//!
//! Fixed by a pure-parser desugar: `T'(e)` for a simple 4-state vector or
//! enum-with-logic-base typedef becomes the composition of the existing casts
//! `(signed'|unsigned')(W'(e))` — the size cast sets T's width (extending with the
//! OPERAND's sign), then the signing cast stamps T's signedness. A
//! struct/union/class/multi-dim/atom(base-less enum, `int`)/2-state(`bit`) typedef
//! has no simple (width, signed) form and stays honest-loud. No AST shape change
//! (reuses Cast/Size/Signing), `.vu`/format unchanged, IR-0. Pinned to iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_tdc_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let r = String::from_utf8_lossy(&out.stdout)
        .lines()
        .find_map(|l| l.trim().strip_prefix("R:").map(str::to_owned))
        .unwrap_or_default();
    (r, out.status.success())
}

const PKG: &str = "package p; typedef enum logic [1:0] {A,B,C} mode_e;\n\
                   typedef logic [7:0] byte_t; typedef logic signed [7:0] sb_t; endpackage\n";

#[test]
fn enum_typedef_cast() {
    let (o, ok) = run(&format!(
        "{PKG}module tb; import p::*; mode_e m; logic [1:0] raw;\n\
         initial begin raw=2'd1; m=mode_e'(raw); $display(\"R:%0d\", m); $finish; end endmodule"
    ));
    assert!(ok && o == "1", "got:\n{o}");
}

#[test]
fn vector_typedef_cast_truncates() {
    // 16'h1FF cast to the 8-bit byte_t keeps the low byte.
    let (o, ok) = run(&format!(
        "{PKG}module tb; import p::*; logic [7:0] q;\n\
         initial begin q=byte_t'(16'h1FF); $display(\"R:%h\", q); $finish; end endmodule"
    ));
    assert!(ok && o == "ff", "got:\n{o}");
}

#[test]
fn signed_typedef_cast_reinterprets() {
    // 8'hFF cast to the 8-bit signed sb_t reads as -1.
    let (o, ok) = run(&format!(
        "{PKG}module tb; import p::*; int r;\n\
         initial begin r=sb_t'(8'hFF); $display(\"R:%0d\", r); $finish; end endmodule"
    ));
    assert!(ok && o == "-1", "got:\n{o}");
}

#[test]
fn signed_typedef_cast_widens_with_operand_sign() {
    // 4'shF is -1; widened to 8-bit signed via the operand's sign, still -1.
    let (o, ok) = run(&format!(
        "{PKG}module tb; import p::*; int r;\n\
         initial begin r=sb_t'(4'shF); $display(\"R:%0d\", r); $finish; end endmodule"
    ));
    assert!(ok && o == "-1", "got:\n{o}");
}

#[test]
fn unsigned_operand_into_signed_typedef_zero_extends() {
    // 4'hF (UNSIGNED 4-bit 15) into the 8-bit signed sb_t: the extension uses the
    // OPERAND's sign (zero-extend → 0x0F), then signed is stamped → +15, NOT -1.
    let (o, ok) = run(&format!(
        "{PKG}module tb; import p::*; int r;\n\
         initial begin r=sb_t'(4'hF); $display(\"R:%0d\", r); $finish; end endmodule"
    ));
    assert!(ok && o == "15", "got:\n{o}");
}

#[test]
fn signed_operand_into_unsigned_typedef_sign_extends() {
    // 4'shF (SIGNED -1) into the 8-bit unsigned byte_t: operand-sign extension
    // sign-extends → 0xFF, then unsigned is stamped → 255.
    let (o, ok) = run(&format!(
        "{PKG}module tb; import p::*; int r;\n\
         initial begin r=byte_t'(4'shF); $display(\"R:%0d\", r); $finish; end endmodule"
    ));
    assert!(ok && o == "255", "got:\n{o}");
}

#[test]
fn typedef_cast_preserves_xz() {
    // A 4-state logic typedef cast keeps x/z (the size cast is 4-state).
    let (o, ok) = run(&format!(
        "{PKG}module tb; import p::*; logic [7:0] q;\n\
         initial begin q=byte_t'(8'hxz); $display(\"R:%h\", q); $finish; end endmodule"
    ));
    assert!(ok && o == "xz", "got:\n{o}");
}

#[test]
fn typedef_cast_in_expression() {
    let (o, ok) = run(
        "package p; typedef enum logic [2:0] {X0,X1,X2,X3,X4,X5} e_t; endpackage\n\
         module tb; import p::*; logic [2:0] o;\n\
         initial begin o=e_t'(3'd5)+1; $display(\"R:%0d\", o); $finish; end endmodule",
    );
    assert!(ok && o == "6", "got:\n{o}");
}

#[test]
fn module_local_typedef_cast() {
    // The typedef need not come from a package.
    let (o, ok) = run("module tb; typedef logic [3:0] nib_t; logic [3:0] q;\n\
         initial begin q=nib_t'(8'hAB); $display(\"R:%h\", q); $finish; end endmodule");
    assert!(ok && o == "b", "got:\n{o}");
}

#[test]
fn struct_typedef_cast_is_loud() {
    // A struct typedef cast needs per-field semantics — honest-loud (v1).
    let (_o, ok) = run(
        "package p; typedef struct packed {logic[3:0] a,b;} s_t; endpackage\n\
         module tb; import p::*; logic [7:0] q;\n\
         initial begin q=s_t'(8'hAB); $display(\"R:%h\", q); $finish; end endmodule",
    );
    assert!(!ok, "a struct typedef cast must be loud (v1)");
}

#[test]
fn baseless_enum_cast_is_loud() {
    // A base-less enum (int storage, 2-state) has no simple range — honest-loud.
    let (_o, ok) = run(
        "package p; typedef enum {A,B,C} e_t; endpackage\n\
         module tb; import p::*; int r; initial begin r=e_t'(1); $display(\"R:%0d\", r); $finish; end endmodule",
    );
    assert!(!ok, "a base-less enum cast must be loud (v1)");
}

#[test]
fn builtin_casts_unchanged() {
    // The built-in int'/N' casts are byte-identical (regression).
    let (o, ok) = run(
        "module tb; int r; logic[3:0] q;\n\
         initial begin r=int'(8'shFF); q=4'(8'hAB); $display(\"R:%0d %h\", r, q); $finish; end endmodule",
    );
    assert!(ok && o == "-1 b", "got:\n{o}");
}
