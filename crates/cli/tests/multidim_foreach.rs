//! Multi-dimension `foreach (a[i,j,…])` (IEEE §12.7.3) was a parse error (E2002,
//! "a single foreach index (multi-dimension foreach is unsupported)"): the foreach
//! parser only accepted a single index. Single-index `foreach (a[i])` already
//! worked (including on a multi-dim array, iterating the first dimension).
//!
//! Fixed by branching to `parse_multidim_foreach` when a comma follows the first
//! index: each named slot iterates its 1-indexed unpacked dimension (an empty slot
//! leaves that dimension un-iterated), built as nested `while` loops driven by a
//! new 2-arg `arr.first/next(idx, DIM)` desugar. Row-major (leftmost = outermost);
//! `break`/`continue` target the innermost dimension's loop (matching iverilog's
//! nested-for desugar). Elaborate's `lower_fixed_foreach_step` gained a `dim`
//! parameter selecting the unpacked dimension's bounds (descending honored).
//! Pinned to iverilog 13.0. Single-index foreach stays byte-identical.
//!
//! Honest-loud (separate): an empty LEADING slot `foreach(a[,j])`; multi-dim
//! foreach on a dynamic/assoc array; an index dimension past the array's actual
//! unpacked dimensions (`a[i,j]` on a 1-D array).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Returns (stdout with the "simulation ended …" trailer stripped, process ok).
fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_mdfe_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    // Truncate at the "simulation ended …" trailer (it may share a line with a
    // trailing $write that emitted no newline).
    let raw = String::from_utf8_lossy(&out.stdout).into_owned();
    let s = match raw.find("simulation ended") {
        Some(idx) => &raw[..idx],
        None => &raw,
    };
    (s.trim().to_owned(), out.status.success())
}

#[test]
fn count_and_order_2d() {
    let (c, ok) = run("module top; int a[2][3]; int c=0;\n\
         initial begin foreach(a[i,j]) c=c+1; $display(\"%0d\",c); #1 $finish; end endmodule");
    assert!(ok && c == "6", "count got:\n{c}");
    // Row-major: i (dim 1) outermost, j (dim 2) innermost.
    let (o, oko) = run("module top; int a[2][3];\n\
         initial begin foreach(a[i,j]) $write(\"(%0d,%0d)\",i,j); #1 $finish; end endmodule");
    assert!(
        oko && o == "(0,0)(0,1)(0,2)(1,0)(1,1)(1,2)",
        "order got:\n{o}"
    );
}

#[test]
fn three_dim_and_index_values() {
    let (c, ok) = run("module top; int a[2][2][2]; int c=0;\n\
         initial begin foreach(a[i,j,k]) c=c+1; $display(\"%0d\",c); #1 $finish; end endmodule");
    assert!(ok && c == "8", "3d count got:\n{c}");
    // i,j carry the real indices: write each element then read back.
    let (v, okv) = run("module top; int a[2][2];\n\
         initial begin foreach(a[i,j]) a[i][j]=i*10+j; \
         $write(\"%0d \",a[0][0]); $write(\"%0d \",a[0][1]); $write(\"%0d \",a[1][0]); $display(\"%0d\",a[1][1]); \
         #1 $finish; end endmodule");
    assert!(okv && v == "0 1 10 11", "idx-vals got:\n{v}");
}

#[test]
fn break_exits_innermost_loop() {
    // iverilog desugars foreach to nested for-loops, so `break` exits only the
    // INNERMOST dimension's loop: a[3][3], break at (1,1) → i=0 (3) + i=1 (1) +
    // i=2 (3) = 7 increments.
    let (c, ok) = run("module top; int a[3][3]; int c=0;\n\
         initial begin foreach(a[i,j]) begin if(i==1 && j==1) break; c=c+1; end \
         $display(\"%0d\",c); #1 $finish; end endmodule");
    assert!(ok && c == "7", "break got:\n{c}");
}

#[test]
fn continue_advances_innermost() {
    // continue skips the rest of the innermost iteration: a[2][3], skip j==1 →
    // 2 per row × 2 rows = 4.
    let (c, ok) = run("module top; int a[2][3]; int c=0;\n\
         initial begin foreach(a[i,j]) begin if(j==1) continue; c=c+1; end \
         $display(\"%0d\",c); #1 $finish; end endmodule");
    assert!(ok && c == "4", "continue got:\n{c}");
}

#[test]
fn empty_slots_skip_dimensions() {
    // Empty trailing slot: foreach(a[i,]) iterates only dim 1 (2 elements).
    let (t, okt) = run("module top; int a[2][3]; int c=0;\n\
         initial begin foreach(a[i,]) c=c+1; $display(\"%0d\",c); #1 $finish; end endmodule");
    assert!(okt && t == "2", "trailing got:\n{t}");
    // Empty middle slot: foreach(a[i,,k]) iterates dims 1 and 3 (2×4 = 8).
    let (m, okm) = run("module top; int a[2][3][4]; int c=0;\n\
         initial begin foreach(a[i,,k]) c=c+1; $display(\"%0d\",c); #1 $finish; end endmodule");
    assert!(okm && m == "8", "middle got:\n{m}");
}

#[test]
fn descending_and_offset_bounds() {
    // Descending unpacked dims walk high→low and carry the declared index values.
    let (d, okd) = run("module top; int a[2:1][1:0];\n\
         initial begin foreach(a[i,j]) $write(\"(%0d,%0d)\",i,j); #1 $finish; end endmodule");
    assert!(okd && d == "(2,1)(2,0)(1,1)(1,0)", "descending got:\n{d}");
    // Non-zero offset bounds: a[1:2][5:7], sum of i*10+j = 126.
    let (o, oko) = run("module top; int a[1:2][5:7]; int s=0;\n\
         initial begin foreach(a[i,j]) s=s+i*10+j; $display(\"%0d\",s); #1 $finish; end endmodule");
    assert!(oko && o == "126", "offset got:\n{o}");
}

#[test]
fn nested_foreach_shadowed_index() {
    // A nested foreach reusing an outer index name: each renames its own body, so
    // the inner index is independent (2×2 outer × 2 inner = 8).
    let (c, ok) = run("module top; int a[2][2]; int b[2]; int c=0;\n\
         initial begin foreach(a[i,j]) foreach(b[i]) c=c+1; $display(\"%0d\",c); #1 $finish; end endmodule");
    assert!(ok && c == "8", "nested got:\n{c}");
}

#[test]
fn single_index_foreach_unchanged() {
    // Byte-identity: single-index foreach is untouched, including on a multi-dim
    // array (iterates the first dimension only).
    let (a, oka) = run("module top; int a[3]; int c=0;\n\
         initial begin foreach(a[i]) c=c+1; $display(\"%0d\",c); #1 $finish; end endmodule");
    assert!(oka && a == "3", "1d got:\n{a}");
    let (b, okb) = run("module top; int a[2][3]; int c=0;\n\
         initial begin foreach(a[i]) c=c+1; $display(\"%0d\",c); #1 $finish; end endmodule");
    assert!(okb && b == "2", "2d-first got:\n{b}");
}

#[test]
fn out_of_range_dimension_is_loud() {
    // `foreach(a[i,j])` on a 1-D array references a non-existent dimension 2 — loud.
    let (_o, ok) = run("module top; int a[3];\n\
         initial begin foreach(a[i,j]) ; $display(\"x\"); #1 $finish; end endmodule");
    assert!(!ok, "an out-of-range foreach dimension must be loud");
}

#[test]
fn duplicate_index_name_is_loud() {
    // Each foreach index declares a distinct implicit loop variable; a repeat
    // (`foreach(a[i,i])`) is illegal (iverilog rejects it) — loud, not a silent
    // mis-iteration from aliasing both levels to one synthetic net.
    let (_o, ok) = run("module top; int a[2][3]; int c=0;\n\
         initial begin foreach(a[i,i]) c=c+1; $display(\"%0d\",c); #1 $finish; end endmodule");
    assert!(!ok, "a duplicate foreach index name must be loud");
}

#[test]
fn multidim_on_dynamic_array_is_loud() {
    // Multi-dim foreach is supported only on fixed-size unpacked arrays — loud on
    // a dynamic array.
    let (_o, ok) = run("module top; int a[][];\n\
         initial begin a=new[2]; foreach(a[i,j]) ; $display(\"x\"); #1 $finish; end endmodule");
    assert!(!ok, "multi-dim foreach on a dynamic array must be loud");
}
