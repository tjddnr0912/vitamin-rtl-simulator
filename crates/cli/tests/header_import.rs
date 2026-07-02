//! ANSI module-header package import — `module m import p::*; (ports);`
//! (IEEE 1800-2017 §A.1.2 module_ansi_header / §26.4). The import sits between
//! the module name and the parameter/port list so the imported symbols are
//! visible to the port list (`input logic [W-1:0] a`). vita rejected it at the
//! `import` token ("expected ';' after module header"); only the BODY position
//! and scoped `p::W` references worked. iverilog 13.0 accepts the header form.
//!
//! Fixed by parsing zero+ `import pkg::item;` after the module name and leading
//! the body with them as `ModuleItem::Import` — elaborate's import pass already
//! scans body imports and applies them before resolving port widths, so a
//! header import and a body import register identically. Pure parser, no AST
//! field added (reuses `ModuleItem::Import`), `.vu`/format unchanged, IR-0.
//! Pinned to iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Run `src`, return (first `R:` display line without the prefix, success).
fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_himp_{}_{n}", std::process::id()));
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

const PKG: &str = "package p; localparam int W=8; endpackage\n";

#[test]
fn wildcard_import_param_in_port_width() {
    // The imported `W` resolves in the port width `[W-1:0]`.
    let (o, ok) = run(&format!(
        "{PKG}module m import p::*; (input logic [W-1:0] a, output logic [W-1:0] q);\n\
         assign q=a; endmodule\n\
         module tb; logic[7:0] a,q; m d(.a(a),.q(q));\n\
         initial begin a=5; #1 $display(\"R:%0d\",q); $finish; end endmodule"
    ));
    assert!(ok && o == "5", "got:\n{o}");
}

#[test]
fn explicit_symbol_import() {
    let (o, ok) = run(&format!(
        "{PKG}module m import p::W; (input logic [W-1:0] a, output logic [W-1:0] q);\n\
         assign q=a; endmodule\n\
         module tb; logic[7:0] a,q; m d(.a(a),.q(q));\n\
         initial begin a=6; #1 $display(\"R:%0d\",q); $finish; end endmodule"
    ));
    assert!(ok && o == "6", "got:\n{o}");
}

#[test]
fn multiple_separate_import_statements() {
    // `import p::*; import q2::*;` — two header import declarations.
    let (o, ok) = run("package p; localparam int W=8; endpackage\n\
         package q2; localparam int X=3; endpackage\n\
         module m import p::*; import q2::*; (input logic [W-1:0] a, output logic [W-1:0] r);\n\
         assign r=a+X; endmodule\n\
         module tb; logic[7:0] a,r; m d(.a(a),.r(r));\n\
         initial begin a=5; #1 $display(\"R:%0d\",r); $finish; end endmodule");
    assert!(ok && o == "8", "got:\n{o}");
}

#[test]
fn import_then_param_list() {
    // The header import precedes the `#(…)` parameter list.
    let (o, ok) = run(&format!(
        "{PKG}module m import p::*; #(parameter int N=2)\n\
         (input logic [W-1:0] a, output logic [W-1:0] q); assign q=a+N; endmodule\n\
         module tb; logic[7:0] a,q; m d(.a(a),.q(q));\n\
         initial begin a=5; #1 $display(\"R:%0d\",q); $finish; end endmodule"
    ));
    assert!(ok && o == "7", "got:\n{o}");
}

#[test]
fn header_import_visible_in_body() {
    // A header import is also visible to the module body (it leads the body).
    let (o, ok) = run(&format!(
        "{PKG}module m import p::*; (input logic [W-1:0] a, output logic [W-1:0] q);\n\
         logic [W-1:0] tmp; assign tmp=a; assign q=tmp; endmodule\n\
         module tb; logic[7:0] a,q; m d(.a(a),.q(q));\n\
         initial begin a=12; #1 $display(\"R:%0d\",q); $finish; end endmodule"
    ));
    assert!(ok && o == "12", "got:\n{o}");
}

#[test]
fn no_import_header_unchanged() {
    // A header with no import is byte-identical to before (regression).
    let (o, ok) = run(
        "module m (input logic [7:0] a, output logic [7:0] q); assign q=a; endmodule\n\
         module tb; logic[7:0] a,q; m d(.a(a),.q(q));\n\
         initial begin a=42; #1 $display(\"R:%0d\",q); $finish; end endmodule",
    );
    assert!(ok && o == "42", "got:\n{o}");
}

#[test]
fn comma_import_form_is_loud() {
    // `import p::*, q::*;` (multiple items in ONE declaration) is the existing
    // v7 single-term-per-statement limit — loud in the header exactly as in the
    // body (consistent, not a silent acceptance). Use separate statements.
    let (_o, ok) = run("package p; localparam int W=8; endpackage\n\
         package q2; localparam int X=3; endpackage\n\
         module m import p::*, q2::*; (input logic [W-1:0] a, output logic [W-1:0] r);\n\
         assign r=a+X; endmodule");
    assert!(
        !ok,
        "comma-form header import must be loud (v7 single-term limit)"
    );
}

#[test]
fn unresolvable_import_symbol_is_loud() {
    // An explicit import of a non-existent package symbol is loud (iverilog too).
    let (_o, ok) = run(&format!(
        "{PKG}module m import p::NONEXIST; (input logic [7:0] a, output logic [7:0] q);\n\
         assign q=a; endmodule"
    ));
    assert!(!ok, "import of a non-existent symbol must be loud");
}

#[test]
fn ambiguous_wildcard_symbol_in_port_range_is_loud() {
    // Two header wildcard imports binding the SAME symbol make it ambiguous
    // (IEEE §26.8); using it in a port width must be loud, NOT a silent 1-bit
    // port. (Header import exposed this — a pre-existing range/width gap where an
    // unresolved name silently clamped to width 1; now loud, like the expr path.)
    let (_o, ok) = run("package pa; localparam int W=5; endpackage\n\
         package pb; localparam int W=8; endpackage\n\
         module dut import pa::*; import pb::*; (input logic [W-1:0] a, output logic [W-1:0] b);\n\
         assign b=a; endmodule\n\
         module tb; logic[7:0] ai,bo; dut u0(.a(ai),.b(bo));\n\
         initial begin ai=8'hFF; #1 $display(\"R:%08b\",bo); #2 $finish; end endmodule");
    assert!(
        !ok,
        "an ambiguous wildcard symbol in a port range must be loud"
    );
}

#[test]
fn undefined_name_in_range_is_loud() {
    // Pre-existing root cause (exposed by the header-import slice, fixed here):
    // an undefined name in a constant range bound silently clamped the net to
    // width 1 (`[UNDEF-1:0]` → `[-1:0]` → 1 bit). It is now loud, matching the
    // expression path (E3010) and iverilog. A valid param/genvar still folds.
    let (_o, ok) = run("module m; logic [UNDEF-1:0] x;\n\
         initial begin x=1; $display(\"R:%0d\", x); $finish; end endmodule");
    assert!(!ok, "an undefined name in a range bound must be loud");
}

#[test]
fn valid_param_range_still_folds() {
    // The undefined-name fix must NOT false-loud a legitimate param range.
    let (o, ok) = run("module m; localparam int W=4; logic [W-1:0] x;\n\
         initial begin x=15; $display(\"R:%0d\", x); $finish; end endmodule");
    assert!(ok && o == "15", "got:\n{o}");
}
