//! A user-defined type name as a MODULE port type — `module m (input mode_e mode);`
//! (IEEE 1800-2017 §23.2.2.1; report §6 A3, the typedef-recognition family's
//! module-port member). vita's port parser (ANSI `parse_ansi_port` and non-ANSI
//! `parse_port_decl`) recognized only built-in kind keywords, so a typedef name
//! in port position was read as the port NAME and the next token errored
//! ("expected ')'"). iverilog 13.0 accepts a typedef port type.
//!
//! Fixed by a shared `try_port_typedef` (mirroring `try_tf_port_typedef` for
//! tf-ports) used by both port parsers: a simple vector/enum typedef resolves to
//! its (kind, signed, range). It requires the next token to be the port name so a
//! bare continuation (`input byte_t a, b`) is not misresolved. struct/union/
//! class/multi-dim-packed typedef ports are honest-loud (v1, as for tf-ports).
//! Pure parser, no AST field added, `.vu`/format unchanged, IR-0. The typedef is
//! brought into scope via a package + header import (A1, §4.5.55). Pinned to
//! iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Run `src`, return (first `R:` display line without the prefix, success).
fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_ptd_{}_{n}", std::process::id()));
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

#[test]
fn enum_typedef_port() {
    let (o, ok) = run(
        "package p; typedef enum logic [1:0] {A,B,C} mode_e; endpackage\n\
         module m import p::*; (input mode_e mode, output logic hit); assign hit=(mode==B); endmodule\n\
         module tb; logic hit; m d(.mode(2'd1),.hit(hit));\n\
         initial begin #1 $display(\"R:%0d\",hit); $finish; end endmodule",
    );
    assert!(ok && o == "1", "got:\n{o}");
}

#[test]
fn vector_typedef_port() {
    let (o, ok) = run("package p; typedef logic [7:0] byte_t; endpackage\n\
         module m import p::*; (input byte_t a, output byte_t q); assign q=a; endmodule\n\
         module tb; logic[7:0] a,q; m d(.a(a),.q(q));\n\
         initial begin a=42; #1 $display(\"R:%0d\",q); $finish; end endmodule");
    assert!(ok && o == "42", "got:\n{o}");
}

#[test]
fn signed_typedef_port_keeps_sign() {
    let (o, ok) = run("package p; typedef logic signed [7:0] sb_t; endpackage\n\
         module m import p::*; (input sb_t a, output logic signed [7:0] q); assign q=a; endmodule\n\
         module tb; logic signed [7:0] a,q; m d(.a(a),.q(q));\n\
         initial begin a=-5; #1 $display(\"R:%0d\",q); $finish; end endmodule");
    assert!(ok && o == "-5", "got:\n{o}");
}

#[test]
fn typedef_port_comma_continuation() {
    // `input byte_t a, b` — `b` inherits the typedef'd port's type (sticky comma),
    // NOT misresolved as a type (the `peek_at(1)=Ident` guard).
    let (o, ok) = run("package p; typedef logic [7:0] byte_t; endpackage\n\
         module m import p::*; (input byte_t a, b, output byte_t q); assign q=a+b; endmodule\n\
         module tb; logic[7:0] q; m d(.a(8'd5),.b(8'd7),.q(q));\n\
         initial begin #1 $display(\"R:%0d\",q); $finish; end endmodule");
    assert!(ok && o == "12", "got:\n{o}");
}

#[test]
fn non_ansi_typedef_port() {
    // The non-ANSI body port declaration (`input byte_t a;`) takes the same path.
    let (o, ok) = run("package p; typedef logic [7:0] byte_t; endpackage\n\
         module m(a,q); import p::*; input byte_t a; output byte_t q; assign q=a; endmodule\n\
         module tb; logic[7:0] a,q; m d(.a(a),.q(q));\n\
         initial begin a=33; #1 $display(\"R:%0d\",q); $finish; end endmodule");
    assert!(ok && o == "33", "got:\n{o}");
}

#[test]
fn enum_label_usable_with_typedef_port() {
    // The enum-ness survives: an enum label compares against the typed port.
    let (o, ok) = run(
        "package p; typedef enum logic [1:0] {A,B,C} mode_e; endpackage\n\
         module m import p::*; (input mode_e mode, output logic [1:0] o);\n\
         assign o = (mode==B) ? mode : 2'd0; endmodule\n\
         module tb; logic[1:0] o; m d(.mode(2'd1),.o(o));\n\
         initial begin #1 $display(\"R:%0d\",o); $finish; end endmodule",
    );
    assert!(ok && o == "1", "got:\n{o}");
}

#[test]
fn builtin_port_unchanged() {
    // A built-in kind keyword port is byte-identical (regression).
    let (o, ok) = run(
        "module m (input logic [7:0] a, output logic [7:0] q); assign q=a; endmodule\n\
         module tb; logic[7:0] a,q; m d(.a(a),.q(q));\n\
         initial begin a=99; #1 $display(\"R:%0d\",q); $finish; end endmodule",
    );
    assert!(ok && o == "99", "got:\n{o}");
}

#[test]
fn struct_typedef_port_is_loud() {
    // A struct/union/class/multi-dim-packed typedef port type is honest-loud (v1,
    // as for tf-ports) — iverilog supports it, vita rejects rather than guessing.
    let (_o, ok) = run(
        "package p; typedef struct packed {logic[3:0] x; logic[3:0] y;} pair_t; endpackage\n\
         module m import p::*; (input pair_t pp, output logic [3:0] s); assign s=pp.x+pp.y; endmodule",
    );
    assert!(!ok, "a struct typedef port type must be loud (v1)");
}

#[test]
fn typedef_port_with_extra_dims_is_loud() {
    // `input byte_t [3:0] a` (an array of the typedef) is honest-loud — the guard
    // declines (`[` after the type, not the name), so it does not silently parse.
    let (_o, ok) = run(
        "package p; typedef logic [7:0] byte_t; endpackage\n\
         module m import p::*; (input byte_t [3:0] a, output logic q); assign q=a[0][0]; endmodule",
    );
    assert!(
        !ok,
        "a typedef port with extra packed dims must be loud (v1)"
    );
}

#[test]
fn two_different_typedef_ports_in_same_list() {
    // `input byte_t a, word_t c` — the second port has its own typedef type.
    // Before the fix the interface-port heuristic (Ident-Ident) fired for `word_t c`
    // and vita errored (E3002); after the fix vita correctly resolves both.
    let (o, ok) = run(
        "package p; typedef logic [7:0] byte_t; typedef logic [15:0] word_t; endpackage\n\
         module m import p::*; (input byte_t a, word_t c, output logic[7:0] oa, output logic[15:0] oc);\n\
         assign oa=a; assign oc=c; endmodule\n\
         module tb; logic[7:0] a,oa; logic[15:0] c,oc; m d(.a(a),.c(c),.oa(oa),.oc(oc));\n\
         initial begin a=8'h42; c=16'hBEEF; #1 $display(\"R:%h %h\",oa,oc); $finish; end endmodule",
    );
    assert!(ok && o == "42 beef", "got:\n{o}");
}

#[test]
fn builtin_first_typedef_second_in_port_list() {
    // `input logic [7:0] a, nib_t c` — built-in type first, typedef second.
    // The second port must get nib_t's width (4-bit), not 1-bit or 8-bit.
    let (o, ok) = run(
        "package p; typedef logic [3:0] nib_t; endpackage\n\
         module m import p::*; (input logic [7:0] a, nib_t c, output logic[7:0] oa, output logic[3:0] oc);\n\
         assign oa=a; assign oc=c; endmodule\n\
         module tb; logic[7:0] a,oa; logic[3:0] c,oc; m d(.a(a),.c(c),.oa(oa),.oc(oc));\n\
         initial begin a=8'hDE; c=4'hA; #1 $display(\"R:%h %h\",oa,oc); $finish; end endmodule",
    );
    assert!(ok && o == "de a", "got:\n{o}");
}

#[test]
fn mixed_typedef_continuation_groups() {
    // `input byte_t a, b, word_t c, d` — a/b inherit byte_t, c/d inherit word_t.
    let (o, ok) = run(
        "package p; typedef logic [7:0] byte_t; typedef logic [15:0] word_t; endpackage\n\
         module m import p::*; (input byte_t a, b, word_t c, d,\n\
           output logic[7:0] oa, ob, output logic[15:0] oc, od);\n\
         assign oa=a; assign ob=b; assign oc=c; assign od=d; endmodule\n\
         module tb; logic[7:0] a,b,oa,ob; logic[15:0] c,d,oc,od;\n\
         m u(.a(a),.b(b),.c(c),.d(d),.oa(oa),.ob(ob),.oc(oc),.od(od));\n\
         initial begin a=8'hAA; b=8'hBB; c=16'hCCCC; d=16'hDDDD;\n\
           #1 $display(\"R:%h %h %h %h\",oa,ob,oc,od); $finish; end endmodule",
    );
    assert!(ok && o == "aa bb cccc dddd", "got:\n{o}");
}
