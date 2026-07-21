//! §4.5.184 (loud -> supported): a MULTI-DIMENSIONAL packed array type as a
//! struct/union member (`logic [1:0][3:0] m`). Previously the struct/union member
//! parser accepted only a single `[range]` and rejected the second dim with a parse
//! error, even though vita supports multi-dim packed nets everywhere else and
//! iverilog (the oracle) accepts the member form. The member now folds to its flat
//! bit width (∏ of all dim widths); the whole member `s.m` reads/writes that flat
//! vector, and a first-level element select `s.m[i]` picks the i-th ∏(inner)-bit
//! element via the existing indexed-part-select machinery (parser-only desugar — no
//! AST/IR/format change).
//!
//! Correct-or-loud boundaries kept loud (a follow-on, never silent-wrong): an
//! element WRITE `s.m[i] = x`, an element RANGE `s.m[i:j]`, an ascending or
//! non-zero-base outer dim, a nested `s.m[i][j]`, and a multi-dim member inside a
//! record array. A runtime element index `s.m[i]` IS supported (iverilog 13.0 itself
//! requires a constant there, so vita exceeds it — the runtime results are
//! hand-verified against the flat layout).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Returns (first `K=` line, process_success).
fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_mdpsm_{}_{n}", std::process::id()));
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

// ── supported: multi-dim packed member ──────────────────────────────────────

#[test]
fn union_multidim_member_read() {
    // The original find: `logic [1:0][3:0] nib` overlaid on a byte.
    let (k, ok) = run("module top;\n\
         typedef union packed { logic [7:0] byte_v; logic [1:0][3:0] nib; } u_t;\n\
         u_t u;\n\
         initial begin u.byte_v = 8'hAB;\n\
         $display(\"K=%h %h %h\", u.byte_v, u.nib[0], u.nib[1]); $finish; end endmodule");
    assert!(ok && k == "K=ab b a", "got ({k}, {ok})");
}

#[test]
fn struct_multidim_member_with_neighbor() {
    // 2D member `m` (8 bits) beside a 4-bit tag inside a 12-bit struct.
    let (k, ok) = run("module top;\n\
         typedef struct packed { logic [1:0][3:0] m; logic [3:0] tag; } s_t;\n\
         s_t s;\n\
         initial begin s = 12'hABC;\n\
         $display(\"K=%h %h %h %h\", s, s.m[0], s.m[1], s.tag); $finish; end endmodule");
    assert!(ok && k == "K=abc b a c", "got ({k}, {ok})");
}

#[test]
fn whole_member_read_write() {
    let (k, ok) = run("module top;\n\
         typedef struct packed { logic [1:0][3:0] m; } s_t;\n\
         s_t s;\n\
         initial begin s.m = 8'h5A;\n\
         $display(\"K=%h %h %h\", s.m, s.m[0], s.m[1]); $finish; end endmodule");
    assert!(ok && k == "K=5a a 5", "got ({k}, {ok})");
}

#[test]
fn three_dim_first_level_element() {
    // `[1:0][1:0][3:0]` — a first-level element is 2*4 = 8 bits.
    let (k, ok) = run("module top;\n\
         typedef struct packed { logic [1:0][1:0][3:0] m; } s_t;\n\
         s_t s;\n\
         initial begin s = 16'h1234;\n\
         $display(\"K=%h %h %h\", s, s.m[0], s.m[1]); $finish; end endmodule");
    assert!(ok && k == "K=1234 34 12", "got ({k}, {ok})");
}

#[test]
fn runtime_element_index() {
    // A runtime index over `[3:0][3:0]` — s=0x89AB → m[0]=b(11), m[1]=a(10), m[2]=9,
    // m[3]=8; the running sum is 11+10+9+8 = 38 (each element read at a runtime index).
    let (k, ok) = run("module top;\n\
         typedef struct packed { logic [3:0][3:0] m; } s_t;\n\
         s_t s; int i, acc;\n\
         initial begin s = 16'h89AB; acc = 0;\n\
         for(i=0;i<4;i++) acc = acc + s.m[i];\n\
         $display(\"K=%0d\", acc); $finish; end endmodule");
    assert!(ok && k == "K=38", "got ({k}, {ok})");
}

#[test]
fn signed_multidim_member() {
    let (k, ok) = run("module top;\n\
         typedef struct packed { logic signed [1:0][3:0] m; } s_t;\n\
         s_t s;\n\
         initial begin s = 8'hF0;\n\
         $display(\"K=%h %h\", s.m[0], s.m[1]); $finish; end endmodule");
    assert!(ok && k == "K=0 f", "got ({k}, {ok})");
}

#[test]
fn whole_struct_copy_compare() {
    let (k, ok) = run("module top;\n\
         typedef struct packed { logic [1:0][3:0] m; } s_t;\n\
         s_t a, b;\n\
         initial begin a = 8'h5A; b = a;\n\
         $display(\"K=%b %h %h\", a==b, b.m[0], b.m[1]); $finish; end endmodule");
    assert!(ok && k == "K=1 a 5", "got ({k}, {ok})");
}

// ── unaffected: single-dim members stay byte-identical ──────────────────────

#[test]
fn single_dim_member_unaffected() {
    let (k, ok) = run("module top;\n\
         typedef struct packed { logic [3:0] hi; logic [3:0] lo; } b_t;\n\
         b_t s;\n\
         initial begin s = 8'hA5;\n\
         $display(\"K=%h %h %h %b\", s, s.hi, s.lo, s.lo[1]); $finish; end endmodule");
    assert!(
        ok && k == "K=a5 a 5 0",
        "single-dim member must be unaffected; got ({k}, {ok})"
    );
}

#[test]
fn non_zero_lsb_member_subselect_unaffected() {
    let (k, ok) = run("module top;\n\
         typedef struct packed { logic [15:8] a; logic [7:0] b; } t_t;\n\
         t_t s;\n\
         initial begin s = 16'hABCD;\n\
         $display(\"K=%h %h %h\", s.a, s.b, s.a[11:8]); $finish; end endmodule");
    assert!(
        ok && k == "K=ab cd b",
        "non-zero-LSB member must be unaffected; got ({k}, {ok})"
    );
}

// ── correct-or-loud boundaries: must STAY loud (never silent-wrong) ──────────

#[test]
fn element_write_stays_loud() {
    assert!(loud(
        "module top;\n\
         typedef struct packed { logic [1:0][3:0] m; } s_t;\n\
         s_t s;\n\
         initial begin s.m[0] = 4'hF; $display(\"K=%h\", s.m); $finish; end endmodule"
    ));
}

#[test]
fn element_range_stays_loud() {
    assert!(loud(
        "module top;\n\
         typedef struct packed { logic [3:0][3:0] m; } s_t;\n\
         s_t s;\n\
         initial begin s = 16'h1234; $display(\"K=%h\", s.m[2:1]); $finish; end endmodule"
    ));
}

#[test]
fn ascending_outer_dim_stays_loud() {
    assert!(loud(
        "module top;\n\
         typedef struct packed { logic [0:1][3:0] m; } s_t;\n\
         s_t s;\n\
         initial begin s = 8'hAB; $display(\"K=%h\", s.m[0]); $finish; end endmodule"
    ));
}

#[test]
fn nested_element_bit_select_stays_loud() {
    // `s.m[i][j]` (bit within an element) is a follow-on — loud, not silent-wrong.
    assert!(loud(
        "module top;\n\
         typedef struct packed { logic [1:0][3:0] m; } s_t;\n\
         s_t s;\n\
         initial begin s = 8'hA5; $display(\"K=%b\", s.m[0][0]); $finish; end endmodule"
    ));
}
