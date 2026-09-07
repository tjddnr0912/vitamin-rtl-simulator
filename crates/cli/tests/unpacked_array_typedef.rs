//! `typedef logic [7:0] a_t [0:3];` — an UNPACKED-array typedef (ROADMAP §3 ⑤).
//!
//! vita refused it at the TYPEDEF (`E-PARSE-UNEXPECTED-TOKEN: expected ';', found
//! '['`), never at the use, while both oracles accept it and vita's own machinery
//! already runs the shape: the identical `logic [7:0] x [0:3];` declaration is
//! correct today in every position tested here. The gap was one missing carrier —
//! the parser's `TypeInfo` had no `unpacked` field, so the typedef's dims could not
//! reach the declaration.
//!
//! Every value is pinned to iverilog 13.0 (`-g2012`) and verilator 5.052
//! (`--binary --timing`), which agree on all of them. What stays LOUD stays loud
//! for a stated reason, and each of those has its own test below.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn vita(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_uat_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    let mut all = String::from_utf8_lossy(&out.stdout).into_owned();
    all.push_str(&String::from_utf8_lossy(&out.stderr));
    let mut s = String::new();
    for l in all.lines().filter(|l| {
        !l.starts_with("simulation ended")
            && !l.starts_with("errors=")
            && !l.contains("W-PP-TIMESCALE-DEFAULT")
    }) {
        s.push_str(l);
        s.push('\n');
    }
    (s, out.status.success())
}

fn run(src: &str) -> String {
    let (s, ok) = vita(src);
    assert!(ok, "expected exit 0, got:\n{s}");
    s
}

fn loud(src: &str, needle: &str) {
    let (s, ok) = vita(src);
    assert!(!ok, "expected a loud reject, got exit 0:\n{s}");
    assert!(s.contains(needle), "expected `{needle}` in:\n{s}");
}

/// The headline, beside its control twin — the same shape written explicitly,
/// which was correct before this slice and must stay byte-identical.
#[test]
fn a_module_variable_and_its_explicit_twin() {
    let out = run("typedef logic [7:0] a_t [0:3];\n\
         module top;\n\
           a_t x;\n\
           logic [7:0] y [0:3];\n\
           initial begin\n\
             x[0]=8'h11; x[3]=8'h44; y[0]=8'h11; y[3]=8'h44;\n\
             $display(\"x=%h %h y=%h %h d=%0d s=%0d\", x[0], x[3], y[0], y[3],\n\
                      $dimensions(x), $size(x));\n\
             $finish;\n\
           end\n\
         endmodule\n");
    // Both oracles: x=11 44 y=11 44 d=2 s=4.
    assert_eq!(out, "x=11 44 y=11 44 d=2 s=4\n");
}

/// The four other declaration binders the carry reaches: a package (through a
/// wildcard import), compilation-unit scope, a block-local declaration and a
/// declaration initializer.
#[test]
fn every_declaration_binder() {
    let pkg = run("package p;\n\
           typedef logic [7:0] a_t [0:3];\n\
         endpackage\n\
         module top;\n\
           import p::*;\n\
           a_t x;\n\
           initial begin x[0]=8'hAA; x[3]=8'hDD; $display(\"%h %h\", x[0], x[3]); $finish; end\n\
         endmodule\n");
    assert_eq!(pkg, "aa dd\n");

    let unit = run("typedef logic [7:0] a_t [0:3];\n\
         module top;\n\
           a_t x;\n\
           initial begin x[0]=8'hAA; x[3]=8'hDD; $display(\"%h %h\", x[0], x[3]); $finish; end\n\
         endmodule\n");
    assert_eq!(unit, "aa dd\n");

    let block_local = run("typedef logic [7:0] a_t [0:3];\n\
         module top;\n\
           initial begin : blk\n\
             a_t x;\n\
             x[0]=8'h11; x[3]=8'h44;\n\
             $display(\"%h %h\", x[0], x[3]);\n\
             $finish;\n\
           end\n\
         endmodule\n");
    assert_eq!(block_local, "11 44\n");

    let decl_init = run("typedef logic [7:0] a_t [0:3];\n\
         module top;\n\
           a_t x = '{8'h11, 8'h22, 8'h33, 8'h44};\n\
           initial begin $display(\"%h %h\", x[0], x[3]); $finish; end\n\
         endmodule\n");
    assert_eq!(decl_init, "11 44\n");
}

/// An ANSI module PORT — the second carrier, `try_port_typedef`'s sixth slot.
#[test]
fn an_ansi_module_port() {
    let out = run("typedef logic [7:0] a_t [0:3];\n\
         module sub(input a_t i, output logic [7:0] o);\n\
           assign o = i[2];\n\
         endmodule\n\
         module top;\n\
           a_t x; logic [7:0] o;\n\
           sub u(.i(x), .o(o));\n\
           initial begin x[2]=8'h5A; #1; $display(\"%h\", o); $finish; end\n\
           initial #100 $finish;\n\
         endmodule\n");
    // Both oracles: 5a.
    assert_eq!(out, "5a\n");
}

/// Both spellings of the dimension list, and a CHAINED alias of one.
#[test]
fn multi_dim_size_form_and_a_chained_alias() {
    let multi = run("typedef logic [7:0] a_t [0:3][0:1];\n\
         module top;\n\
           a_t x;\n\
           initial begin x[0][0]=8'h11; x[3][1]=8'h44; $display(\"%h %h\", x[0][0], x[3][1]); $finish; end\n\
         endmodule\n");
    assert_eq!(multi, "11 44\n");

    let size_form = run("typedef logic [7:0] a_t [4];\n\
         module top;\n\
           a_t x;\n\
           initial begin x[0]=8'h11; x[3]=8'h44; $display(\"%h %h\", x[0], x[3]); $finish; end\n\
         endmodule\n");
    assert_eq!(size_form, "11 44\n");

    // `typedef a_t b_t;` inherits the whole registration, dims included.
    let chained = run("typedef logic [7:0] a_t [0:3];\n\
         typedef a_t b_t;\n\
         module top;\n\
           b_t x;\n\
           initial begin x[1]=8'h22; $display(\"%h %0d\", x[1], $dimensions(x)); $finish; end\n\
         endmodule\n");
    // Both oracles: 22 2.
    assert_eq!(chained, "22 2\n");
}

/// A module-local typedef SHADOWS the wildcard-imported one, and the two keep
/// separate shapes — the cell that catches a shared or last-write-wins entry.
#[test]
fn a_module_local_typedef_shadows_the_imported_one() {
    let out = run("package p;\n\
           typedef logic [7:0] a_t [0:3];\n\
         endpackage\n\
         module top;\n\
           import p::*;\n\
           typedef logic [15:0] a_t [0:1];\n\
           a_t x;\n\
           p::a_t y;\n\
           initial begin\n\
             x[0]=16'hBEEF; y[3]=8'h44;\n\
             $display(\"%h %h %0d %0d\", x[0], y[3], $bits(x[0]), $size(x));\n\
             $finish;\n\
           end\n\
         endmodule\n");
    // Both oracles: beef 44 16 2 — the local `[15:0] [0:1]`, the package `[7:0] [0:3]`.
    assert_eq!(out, "beef 44 16 2\n");
}

/// Dims on BOTH the typedef and the declarator stay loud: the two oracles
/// disagree about the resulting dimension order (iverilog `$size(y,1)=4
/// $size(y,2)=2`, verilator `2` and `4`), and iverilog contradicts its own answer
/// for the identical explicit type — so there is nothing to build on.
#[test]
fn declarator_dims_on_top_of_typedef_dims_stay_loud() {
    loud(
        "typedef logic [7:0] a_t [0:3];\n\
         module top;\n\
           a_t y [0:1];\n\
           initial begin y[0][0]=8'h11; $display(\"%h\", y[0][0]); $finish; end\n\
         endmodule\n",
        "an unpacked-array typedef combined with declarator dimensions",
    );
    // The port spelling of the same refusal.
    loud(
        "typedef logic [7:0] a_t [0:3];\n\
         module sub(input a_t i [0:1]);\n\
         endmodule\n\
         module top; sub u(); initial #1 $finish; endmodule\n",
        "an unpacked-array typedef combined with port dimensions",
    );
}

/// The shape-blind consumers decline instead of silently binding ONE element.
/// The first three are refused by both oracles too; the last two are vita-only
/// declines with the reason named.
#[test]
fn the_consumers_that_cannot_carry_the_dims_are_loud() {
    // Both oracles: "Enum data type must be an integer atom or vector type".
    loud(
        "typedef logic [7:0] a_t [0:3];\n\
         module top;\n\
           typedef enum a_t { A, B } e_t;\n\
           initial begin $display(\"e\"); $finish; end\n\
         endmodule\n",
        "as an enum base",
    );
    // Both oracles: "Member f of packed struct/union must be packed".
    loud(
        "typedef logic [7:0] a_t [0:3];\n\
         module top;\n\
           typedef struct packed { a_t f; logic [3:0] g; } s_t;\n\
           s_t s;\n\
           initial begin $display(\"s\"); $finish; end\n\
         endmodule\n",
        "unpacked-array member is unsupported",
    );
    // Both oracles: "cast operation is not yet supported" / "Unsupported: static cast".
    loud(
        "typedef logic [7:0] a_t [0:3];\n\
         module top;\n\
           initial begin $display(\"%h\", a_t'(8'h5A)); $finish; end\n\
         endmodule\n",
        "outside the v1 cast scope",
    );
    // vita-only declines: a subroutine formal and a parameter, both of which
    // would otherwise bind the ELEMENT type.
    loud(
        "typedef logic [7:0] a_t [0:3];\n\
         module top;\n\
           function automatic logic [7:0] pick(input a_t v); pick = v[1]; endfunction\n\
           initial begin $display(\"f\"); $finish; end\n\
         endmodule\n",
        "unpacked-array typedef formal is unsupported",
    );
    loud(
        "typedef logic [7:0] a_t [0:3];\n\
         module top;\n\
           parameter a_t P = '{1,2,3,4};\n\
           initial begin $display(\"%0d\", P[2]); $finish; end\n\
         endmodule\n",
        "unpacked-array typedef parameter is unsupported",
    );
}

/// §3 ⑤ⓕ (§4.5.447): a NON-ANSI port declaration — the second port binder — and
/// `$bits` of the bare TYPE NAME.
#[test]
fn a_non_ansi_port_and_bits_of_the_type_name() {
    let port = run("typedef logic [7:0] a_t [0:3];\n\
         module sub(i, o);\n\
           input a_t i;\n\
           output logic [7:0] o;\n\
           assign o = i[1];\n\
         endmodule\n\
         module top;\n\
           a_t arr; logic [7:0] o;\n\
           sub u(.i(arr), .o(o));\n\
           initial begin arr[1]=8'h33; #1; $display(\"%h\", o); $finish; end\n\
           initial #100 $finish;\n\
         endmodule\n");
    // Both oracles: 33.
    assert_eq!(port, "33\n");

    // `$bits` of the type NAME. The element width times every unpacked dim, for both
    // dimension spellings and through a chained alias.
    let bits = run("typedef logic [7:0] a_t [0:3];\n\
         typedef logic [7:0] b_t [0:3][0:1];\n\
         typedef logic [7:0] c_t [4];\n\
         typedef a_t d_t;\n\
         module top;\n\
           initial begin\n\
             $display(\"a=%0d b=%0d c=%0d d=%0d\", $bits(a_t), $bits(b_t), $bits(c_t), $bits(d_t));\n\
             $finish;\n\
           end\n\
         endmodule\n");
    // Both oracles: a=32 b=64 c=32 d=32.
    assert_eq!(bits, "a=32 b=64 c=32 d=32\n");
}

/// The non-ANSI port keeps the both-dims refusal, and a comma list gives EVERY name
/// the dims (a moved rather than cloned vector would leave the second one scalar).
#[test]
fn the_non_ansi_port_carries_per_name_and_keeps_the_split_refusal() {
    let comma = run("typedef logic [7:0] a_t [0:3];\n\
         module sub(i, j, o);\n\
           input a_t i, j;\n\
           output logic [7:0] o;\n\
           assign o = i[1] ^ j[2];\n\
         endmodule\n\
         module top;\n\
           a_t x, y; logic [7:0] o;\n\
           sub u(.i(x), .j(y), .o(o));\n\
           initial begin x[1]=8'hF0; y[2]=8'h0F; #1; $display(\"%h\", o); $finish; end\n\
           initial #100 $finish;\n\
         endmodule\n");
    // Both oracles: ff.
    assert_eq!(comma, "ff\n");

    loud(
        "typedef logic [7:0] a_t [0:3];\n\
         module sub(i);\n\
           input a_t i [0:1];\n\
         endmodule\n\
         module top; a_t q[0:1]; sub u(.i(q)); initial #1 $finish; endmodule\n",
        "an unpacked-array typedef combined with port dimensions",
    );
}

/// `$bits` of a DYNAMIC / QUEUE / ASSOCIATIVE typedef stays loud: there is no oracle
/// to move toward — iverilog rejects it ("Invalid data type for $bits()") and
/// verilator crashes ("Verilator internal fault") on the first two.
#[test]
fn bits_of_an_unsized_dimension_stays_loud() {
    for dim in ["[]", "[$]", "[string]"] {
        loud(
            &format!(
                "typedef logic [7:0] u_t {dim};\n\
                 module top;\n\
                   initial begin $display(\"%0d\", $bits(u_t)); $finish; end\n\
                 endmodule\n"
            ),
            "E-ELAB",
        );
    }
}
