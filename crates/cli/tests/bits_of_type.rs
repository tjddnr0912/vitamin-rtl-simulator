//! §4.5.185 (loud -> supported): `$bits(TYPE)` where the argument is a data TYPE —
//! a struct/union/simple/enum typedef name (`$bits(s_t)`) or a data-type keyword with
//! optional signedness and packed dims (`$bits(logic[15:0])`, `$bits(int)`). Such an
//! argument is NOT a valid expression, so vita previously rejected it with a parse
//! error (typedef name → E3010; type keyword → E2002); iverilog folds it to the type
//! size. The parser now folds `$bits(TYPE)` to an integer literal where the type
//! widths (struct_layouts / typedefs / atom kinds) are known, so it works everywhere
//! an expression does — including a decl range `logic [$bits(T)-1:0]` and a parameter
//! `localparam W = $bits(T)`. Parser-only; no AST/IR/format change.
//!
//! correct-or-loud: only INTEGRAL types fold. A `real`/`realtime`/`string`/`event`/
//! class type, or a scoped `pkg::T` type name, stays loud (never a silently-wrong
//! 1-bit fold). `$bits(expr)` (a variable, `arr[i]`, a `x.field`) is unchanged — it
//! still rides the elaborate path.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Returns (first `K=` line, process_success).
fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_bot_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let key = text
        .lines()
        .find(|l| l.starts_with("K="))
        .unwrap_or_default()
        .trim()
        .to_owned();
    (key, out.status.success())
}

fn loud(src: &str) -> bool {
    !run(src).1
}

// ── supported: $bits(TYPE) ──────────────────────────────────────────────────

#[test]
fn bits_struct_and_union_typedef() {
    // struct = SUM of field widths (4+3=7); union = MAX (both members 8 bits).
    let (k, ok) = run("module top;\n\
         typedef struct packed { logic [3:0] a; logic [2:0] b; } s_t;\n\
         typedef union packed { logic [7:0] x; logic [1:0][3:0] y; } u_t;\n\
         initial begin $display(\"K=%0d %0d\", $bits(s_t), $bits(u_t)); $finish; end endmodule");
    assert!(ok && k == "K=7 8", "got ({k}, {ok})");
}

#[test]
fn bits_atom_keywords() {
    let (k, ok) = run("module top;\n\
         initial begin\n\
         $display(\"K=%0d %0d %0d %0d %0d\", $bits(int), $bits(byte), $bits(shortint), $bits(longint), $bits(time));\n\
         $finish; end endmodule");
    assert!(ok && k == "K=32 8 16 64 64", "got ({k}, {ok})");
}

#[test]
fn bits_vector_keyword_with_dims() {
    // Single-dim, multi-dim, and bare (1-bit) vector keywords.
    let (k, ok) = run("module top;\n\
         initial begin\n\
         $display(\"K=%0d %0d %0d %0d\", $bits(logic[15:0]), $bits(logic[1:0][3:0]), $bits(bit[7:0]), $bits(logic));\n\
         $finish; end endmodule");
    assert!(ok && k == "K=16 8 8 1", "got ({k}, {ok})");
}

#[test]
fn bits_simple_and_enum_typedef() {
    let (k, ok) = run("module top;\n\
         typedef logic [11:0] word_t;\n\
         typedef bit signed [7:0] sb_t;\n\
         typedef enum logic [2:0] { A, B, C } e_t;\n\
         initial begin $display(\"K=%0d %0d %0d\", $bits(word_t), $bits(sb_t), $bits(e_t)); $finish; end endmodule");
    assert!(ok && k == "K=12 8 3", "got ({k}, {ok})");
}

#[test]
fn bits_type_in_decl_range() {
    // The common idiom: size a vector by a type's width.
    let (k, ok) = run("module top;\n\
         typedef struct packed { logic [3:0] a; logic [2:0] b; } s_t;\n\
         logic [$bits(s_t)-1:0] packed_vec;\n\
         initial begin packed_vec = 7'h5A; $display(\"K=%0d %h\", $bits(packed_vec), packed_vec); $finish; end endmodule");
    assert!(ok && k == "K=7 5a", "got ({k}, {ok})");
}

#[test]
fn bits_type_in_parameter() {
    let (k, ok) = run("module top;\n\
         typedef logic [7:0] byte_t;\n\
         parameter W = $bits(byte_t);\n\
         logic [W-1:0] r;\n\
         initial begin r = 8'hAB; $display(\"K=%0d %h\", $bits(r), r); $finish; end endmodule");
    assert!(ok && k == "K=8 ab", "got ({k}, {ok})");
}

// ── unaffected: $bits(expr) still rides the elaborate path ──────────────────

#[test]
fn bits_expression_unaffected() {
    let (k, ok) = run("module top;\n\
         logic [9:0] v; logic [7:0] mem [0:3];\n\
         typedef struct packed { logic [3:0] a; logic [2:0] b; } s_t;\n\
         s_t s; real rv;\n\
         initial begin s = 7'h5A;\n\
         $display(\"K=%0d %0d %0d %0d %0d\", $bits(v), $bits(s), $bits(mem[0]), $bits(s.a), $bits(rv));\n\
         $finish; end endmodule");
    assert!(
        ok && k == "K=10 7 8 4 64",
        "$bits(expr) must be unaffected; got ({k}, {ok})"
    );
}

// ── correct-or-loud boundaries: must STAY loud (never a wrong fold) ─────────

#[test]
fn bits_real_type_stays_loud() {
    // `$bits(real)`/`$bits(realtime)` — member_width_kind would give a wrong 1; fold
    // is refused so the site stays loud, not silently 1.
    assert!(loud(
        "module top; initial begin $display(\"K=%0d\", $bits(real)); $finish; end endmodule"
    ));
}

#[test]
fn bits_string_type_stays_loud() {
    assert!(loud(
        "module top; initial begin $display(\"K=%0d\", $bits(string)); $finish; end endmodule"
    ));
}

#[test]
fn bits_scoped_type_stays_loud() {
    // A scoped `pkg::T` type name is a follow-on — loud, not silent-wrong.
    assert!(loud(
        "package p; typedef logic [9:0] w_t; endpackage\n\
         module top; initial begin $display(\"K=%0d\", $bits(p::w_t)); $finish; end endmodule"
    ));
}

#[test]
fn bits_unknown_name_stays_loud() {
    assert!(loud(
        "module top; initial begin $display(\"K=%0d\", $bits(nonexistent)); $finish; end endmodule"
    ));
}
