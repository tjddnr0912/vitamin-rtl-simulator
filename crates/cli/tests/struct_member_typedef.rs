//! A packed struct/union member whose type is a USER-DEFINED type name —
//! `typedef logic[7:0] byte_t; typedef struct packed {byte_t a, b;} word_t;` — was
//! a parse error ("a net/var type in struct member"): the member-type parser only
//! recognized built-in type keywords (`net_var_kind()`). iverilog supports it.
//!
//! Fixed by a shared `parse_struct_member_type` helper (used by both the struct
//! and union member loops) that, when no built-in keyword is present, resolves a
//! SIMPLE user-defined type name (a vector / enum / atom typedef) to its
//! kind/sign/range. A nested struct/union, a class handle, or a multi-dim packed
//! typedef member needs nested-layout machinery not in v1 — honest-loud. Pure
//! parser, AST/`.vu`/format_version unchanged, IR-0. Pinned to iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_smt_{}_{n}", std::process::id()));
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
fn vector_typedef_member() {
    let (o, ok) = run(
        "module top; typedef logic [7:0] byte_t; typedef struct packed {byte_t a, b;} word_t;\n\
         word_t w; initial begin w.a=8'hAB; w.b=8'hCD; $display(\"%h %h %h\", w.a, w.b, w); #1 $finish; end endmodule",
    );
    assert!(ok && o == "ab cd abcd", "got:\n{o}");
}

#[test]
fn enum_typedef_member() {
    let (o, ok) = run(
        "module top; typedef enum logic [1:0] {X,Y,Z} e; typedef struct packed {e a; logic [5:0] b;} w_t;\n\
         w_t w; initial begin w.a=Y; w.b=6'h3F; $display(\"%0d %h\", w.a, w.b); #1 $finish; end endmodule",
    );
    assert!(ok && o == "1 3f", "got:\n{o}");
}

#[test]
fn signed_typedef_member() {
    // The member keeps the typedef's signedness — a whole-field read of a signed
    // member reads back negative.
    let (o, ok) = run(
        "module top; typedef logic signed [7:0] s8; typedef struct packed {s8 a; logic [7:0] b;} w_t;\n\
         w_t w; initial begin w.a=-3; w.b=8'hFF; $display(\"%0d %h\", w.a, w.b); #1 $finish; end endmodule",
    );
    assert!(ok && o == "-3 ff", "got:\n{o}");
}

#[test]
fn whole_struct_assign_and_field_reads() {
    let (o, ok) = run(
        "module top; typedef logic [7:0] byte_t; typedef struct packed {byte_t a, b;} w_t;\n\
         w_t w; initial begin w=16'h1234; $display(\"%h %h %h\", w, w.a, w.b); #1 $finish; end endmodule",
    );
    assert!(ok && o == "1234 12 34", "got:\n{o}");
}

#[test]
fn union_typedef_member() {
    // The union member type-parser shares the same helper.
    let (o, ok) = run(
        "module top; typedef logic [7:0] byte_t; typedef union packed {byte_t a; logic [7:0] b;} u_t;\n\
         u_t u; initial begin u.a=8'hAB; $display(\"%h %h\", u.a, u.b); #1 $finish; end endmodule",
    );
    assert!(ok && o == "ab ab", "got:\n{o}");
}

#[test]
fn builtin_member_struct_and_union_unchanged() {
    // Byte-identity: built-in-type members are unaffected.
    let (os, oks) = run(
        "module top; typedef struct packed {logic [3:0] a, b;} w_t;\n\
         w_t w; initial begin w.a=4'hC; w.b=4'hD; $display(\"%h\", w); #1 $finish; end endmodule",
    );
    assert!(oks && os == "cd", "struct got:\n{os}");
    let (ou, oku) = run(
        "module top; typedef union packed {logic [7:0] a; logic [7:0] b;} u_t;\n\
         u_t u; initial begin u.a=8'hCD; $display(\"%h\", u.b); #1 $finish; end endmodule",
    );
    assert!(oku && ou == "cd", "union got:\n{ou}");
}

#[test]
fn nested_struct_member_is_loud() {
    // A member whose type is itself a struct typedef needs nested-field layout —
    // honest-loud (deferred), not silently mis-laid-out.
    let (_o, ok) = run(
        "module top; typedef struct packed {logic [3:0] lo, hi;} bt; typedef struct packed {bt a, b;} w_t;\n\
         w_t w; initial begin #1 $finish; end endmodule",
    );
    assert!(!ok, "a nested struct member must be loud");
}
