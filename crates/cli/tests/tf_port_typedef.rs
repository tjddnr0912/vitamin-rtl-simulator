//! A function/task PORT whose type is a user-defined type name —
//! `typedef logic[7:0] byte_t; function byte_t f(byte_t a); … ` — was a parse
//! error: both the ANSI port parser (`parse_tf_port`) and the non-ANSI one
//! (`parse_tf_port_decl_into`) accepted only a built-in type keyword for a port's
//! type, so the typedef name was mis-consumed as the port name. iverilog 13.0
//! supports it.
//!
//! Fixed by a shared `try_tf_port_typedef` helper (used by BOTH port parsers) that
//! resolves a SIMPLE typedef (vector / enum / atom) to its kind/sign/range. A
//! struct / union / class / multi-dim-packed typedef port needs per-port layout or
//! method binding not in v1 — honest-loud. Pure parser, AST/`.vu`/format unchanged
//! (the same TfPort fields are filled), IR-0. Pinned to iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_tfpt_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let first = String::from_utf8_lossy(&out.stdout)
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .to_owned();
    (first, out.status.success())
}

#[test]
fn ansi_function_port() {
    let (o, ok) = run("module top; typedef logic [7:0] byte_t;\n\
         function automatic byte_t f(byte_t a); return a+1; endfunction\n\
         initial begin $display(\"%h\", f(8'h41)); #1 $finish; end endmodule");
    assert!(ok && o == "42", "got:\n{o}");
}

#[test]
fn ansi_task_output_port() {
    let (o, ok) = run("module top; typedef logic [7:0] byte_t;\n\
         task automatic t(byte_t a, output byte_t o); o=a+1; endtask\n\
         byte_t r; initial begin t(8'h41, r); $display(\"%h\", r); #1 $finish; end endmodule");
    assert!(ok && o == "42", "got:\n{o}");
}

#[test]
fn ansi_enum_port() {
    let (o, ok) = run("module top; typedef enum logic [1:0] {R,G,B} e_t;\n\
         function automatic e_t f(e_t a); return a; endfunction\n\
         initial begin $display(\"%0d\", f(G)); #1 $finish; end endmodule");
    assert!(ok && o == "1", "got:\n{o}");
}

#[test]
fn ansi_signed_port_reads_negative() {
    // The port carries the typedef's signedness.
    let (o, ok) = run("module top; typedef logic signed [7:0] s8;\n\
         function automatic s8 f(s8 a); return a; endfunction\n\
         initial begin $display(\"%0d\", f(-3)); #1 $finish; end endmodule");
    assert!(ok && o == "-3", "got:\n{o}");
}

#[test]
fn ansi_comma_shared_typedef_port() {
    // `f(byte_t a, b)` — the bare `, b` inherits the typedef type from `a`.
    let (o, ok) = run("module top; typedef logic [7:0] byte_t;\n\
         function automatic byte_t f(byte_t a, b); return a+b; endfunction\n\
         initial begin $display(\"%h\", f(8'h10, 8'h22)); #1 $finish; end endmodule");
    assert!(ok && o == "32", "got:\n{o}");
}

#[test]
fn nonansi_function_port() {
    let (o, ok) = run("module top; typedef logic [7:0] byte_t;\n\
         function automatic byte_t f; input byte_t a; f=a+1; endfunction\n\
         initial begin $display(\"%h\", f(8'h41)); #1 $finish; end endmodule");
    assert!(ok && o == "42", "got:\n{o}");
}

#[test]
fn nonansi_task_ports() {
    let (o, ok) = run("module top; typedef logic [7:0] byte_t;\n\
         task automatic t; input byte_t a; output byte_t o; o=a+1; endtask\n\
         byte_t r; initial begin t(8'h41, r); $display(\"%h\", r); #1 $finish; end endmodule");
    assert!(ok && o == "42", "got:\n{o}");
}

#[test]
fn nonansi_comma_and_signed() {
    let (oc, okc) = run("module top; typedef logic [7:0] byte_t;\n\
         function automatic byte_t f; input byte_t a, b; f=a+b; endfunction\n\
         initial begin $display(\"%h\", f(8'h10, 8'h22)); #1 $finish; end endmodule");
    assert!(okc && oc == "32", "comma got:\n{oc}");
    let (os, oks) = run("module top; typedef logic signed [7:0] s8;\n\
         function automatic s8 f; input s8 a; f=a; endfunction\n\
         initial begin $display(\"%0d\", f(-5)); #1 $finish; end endmodule");
    assert!(oks && os == "-5", "signed got:\n{os}");
}

#[test]
fn builtin_ports_unchanged() {
    // Byte-identity: built-in port types are unaffected, ANSI and non-ANSI.
    let (oa, oka) = run(
        "module top; function automatic logic [7:0] f(logic [7:0] a); return a+1; endfunction\n\
         initial begin $display(\"%h\", f(8'h41)); #1 $finish; end endmodule",
    );
    assert!(oka && oa == "42", "ansi got:\n{oa}");
    let (on, okn) = run(
        "module top; function automatic logic [7:0] f; input logic [7:0] a; f=a+1; endfunction\n\
         initial begin $display(\"%h\", f(8'h41)); #1 $finish; end endmodule",
    );
    assert!(okn && on == "42", "nonansi got:\n{on}");
}

#[test]
fn struct_port_is_loud_both_forms() {
    // A struct typedef port needs per-port layout binding — honest-loud, ANSI and non-ANSI.
    let (_a, oka) = run(
        "module top; typedef struct packed {logic [3:0] a, b;} s_t;\n\
         function automatic logic [7:0] f(s_t a); return a; endfunction\n\
         initial begin $display(\"%h\", f(8'hCD)); #1 $finish; end endmodule",
    );
    assert!(!oka, "ANSI struct port must be loud");
    let (_n, okn) = run(
        "module top; typedef struct packed {logic [3:0] a, b;} s_t;\n\
         function automatic logic [7:0] f; input s_t a; f=a; endfunction\n\
         initial begin $display(\"%h\", f(8'hCD)); #1 $finish; end endmodule",
    );
    assert!(!okn, "non-ANSI struct port must be loud");
}
