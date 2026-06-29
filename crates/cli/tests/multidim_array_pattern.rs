//! A nested assignment pattern `'{…}` on a MULTI-DIMENSIONAL unpacked array —
//! `int a[2][3] = '{'{1,2,3},'{4,5,6}};` — was loud-rejected ("an assignment
//! pattern `'{…}` on a multi-dimensional array is not yet supported (v1: a 1-D
//! unpacked array)") since §4.5.33 only handled 1-D arrays. iverilog supports it
//! (row-major nested, `a[0]={1,2,3}`, `a[1]={4,5,6}`).
//!
//! Fixed by `flatten_assign_pattern`: a (possibly nested) pattern is flattened to
//! its leaf elements in row-major / declaration order, validating each level's
//! count against the residual unpacked dimensions, then each leaf is assigned to
//! the matching flat offset (`residual_word_offsets`, which already produces
//! row-major offsets respecting per-dimension direction). A 1-D residual is the
//! unchanged base case (byte-identical). Pinned to iverilog 13.0. The PACKED
//! multi-dim `'{…}` (reverse order) goes through a different path and stays loud.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_mdap_{}_{n}", std::process::id()));
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
fn two_dim_decl_init() {
    let (o, ok) = run("module top; int a[2][3]='{'{1,2,3},'{4,5,6}};\n\
         initial begin $display(\"%0d %0d %0d %0d\", a[0][0],a[0][2],a[1][0],a[1][2]); #1 $finish; end endmodule");
    assert!(ok && o == "1 3 4 6", "got:\n{o}");
}

#[test]
fn three_dim_decl_init() {
    let (o, ok) = run(
        "module top; int a[2][2][2]='{'{'{1,2},'{3,4}},'{'{5,6},'{7,8}}};\n\
         initial begin $display(\"%0d %0d\", a[0][0][0], a[1][1][1]); #1 $finish; end endmodule",
    );
    assert!(ok && o == "1 8", "got:\n{o}");
}

#[test]
fn typed_elements_and_fill() {
    // logic[7:0] elements keep their declared width; fill elements (`'1`/`'0`) in a
    // nested pattern grow to the element width.
    let (ol, okl) = run("module top; logic[7:0] a[2][2]='{'{8'hAA,8'hBB},'{8'hCC,8'hDD}};\n\
         initial begin $display(\"%h %h %h %h\", a[0][0],a[0][1],a[1][0],a[1][1]); #1 $finish; end endmodule");
    assert!(okl && ol == "aa bb cc dd", "logic got:\n{ol}");
    let (of, okf) = run("module top; logic[7:0] a[2][2]='{'{'1,'0},'{8'hAB,'1}};\n\
         initial begin $display(\"%h %h %h %h\", a[0][0],a[0][1],a[1][0],a[1][1]); #1 $finish; end endmodule");
    assert!(okf && of == "ff 00 ab ff", "fill got:\n{of}");
}

#[test]
fn descending_dimensions() {
    // residual_word_offsets honors per-dimension direction: a[2:0][1:0] with
    // row-major pattern places the first row at the high index.
    let (o, ok) = run("module top; int a[2:0][1:0]='{'{1,2},'{3,4},'{5,6}};\n\
         initial begin $display(\"%0d %0d %0d\", a[2][0],a[1][1],a[0][0]); #1 $finish; end endmodule");
    assert!(ok && o == "2 3 6", "got:\n{o}");
}

#[test]
fn runtime_statement_and_expr_elements() {
    // The same path handles a runtime `a = '{…}'` statement and element expressions.
    let (or, okr) = run("module top; int a[2][3];\n\
         initial begin a='{'{1,2,3},'{4,5,6}}; $display(\"%0d %0d\", a[0][0], a[1][2]); #1 $finish; end endmodule");
    assert!(okr && or == "1 6", "runtime got:\n{or}");
    let (oe, oke) = run("module top; int x=5; int a[2][2]='{'{x,x+1},'{x+2,x+3}};\n\
         initial begin $display(\"%0d %0d\", a[0][0], a[1][1]); #1 $finish; end endmodule");
    assert!(oke && oe == "5 8", "expr got:\n{oe}");
}

#[test]
fn foreach_sum_over_wide_2d() {
    let (o, ok) = run("module top; int a[3][4]; int s=0;\n\
         initial begin a='{'{1,2,3,4},'{5,6,7,8},'{9,10,11,12}}; foreach(a[i,j]) s=s+a[i][j]; \
         $display(\"%0d\",s); #1 $finish; end endmodule");
    assert!(ok && o == "78", "got:\n{o}");
}

#[test]
fn one_dim_pattern_unchanged() {
    // Byte-identity: the 1-D base case is unaffected.
    let (o, ok) = run("module top; int a[3]='{1,2,3};\n\
         initial begin $display(\"%0d %0d %0d\", a[0],a[1],a[2]); #1 $finish; end endmodule");
    assert!(ok && o == "1 2 3", "got:\n{o}");
}

#[test]
fn oversized_pattern_is_capped_loud() {
    // The pattern unrolls one assignment per leaf; a pattern expanding past the
    // v1 cap (4096) is loud, matching the array-copy path's bound (robustness).
    let rows = 70;
    let cols = 70; // 4900 leaves > 4096
    let inner = vec!["1"; cols].join(",");
    let pat = (0..rows)
        .map(|_| format!("'{{{inner}}}"))
        .collect::<Vec<_>>()
        .join(",");
    let src = format!(
        "module top; int a[{rows}][{cols}]='{{{pat}}};\n\
         initial begin $display(\"%0d\", a[0][0]); #1 $finish; end endmodule"
    );
    let (_o, ok) = run(&src);
    assert!(!ok, "an oversized pattern must be loud (cap 4096)");
}

#[test]
fn shape_mismatch_is_loud() {
    // A wrong element count at any level, or a flat (non-nested) pattern on a
    // multi-dim array, is loud.
    let (_a, oka) = run("module top; int a[2][3]='{'{1,2},'{4,5,6}};\n\
         initial begin #1 $finish; end endmodule");
    assert!(!oka, "a wrong nested count must be loud");
    let (_b, okb) = run("module top; int a[2][3]='{1,2,3,4,5,6};\n\
         initial begin #1 $finish; end endmodule");
    assert!(!okb, "a flat pattern on a multi-dim array must be loud");
}
