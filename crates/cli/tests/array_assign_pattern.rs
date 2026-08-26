//! SV §10.9 positional assignment pattern `'{e0,…,eN}` bound to a 1-D unpacked
//! array — `int a[N] = '{…};` (declaration init) and `a = '{…};` (runtime). Each
//! element is assigned to the corresponding array slot in DECLARATION order
//! (correct for ascending, descending, and offset bounds). A packed-array or
//! struct target, a multi-dimensional array, an element-count mismatch, and any
//! non-array context stay loud (correct-or-loud; a replicated `'{N{e}}` and an
//! INTEGER-key pattern are still loud). Pinned to iverilog 13.0 — except the
//! `'{default: v}` cells added by V34-3, which iverilog cannot judge (it rejects
//! every keyed pattern) and are pinned to verilator 5.050 + IEEE 1800 §10.9.1;
//! the keyed shapes live in `assign_pattern_keyed.rs`.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_apat_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        out.status.code(),
    )
}

#[test]
fn decl_init_ascending() {
    let (out, code) = run("module top; int a[3]='{10,20,30}; \
         initial begin $display(\"%0d %0d %0d\",a[0],a[1],a[2]); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0));
    assert!(out.contains("10 20 30"), "decl init asc; got:\n{out}");
}

#[test]
fn decl_init_descending_bounds() {
    // `[2:0]`: the pattern fills from the LEFT bound (2) down — a[2]=10, a[0]=30.
    let (out, _c) = run("module top; int a[2:0]='{10,20,30}; \
         initial begin $display(\"%0d %0d %0d\",a[0],a[1],a[2]); #1 $finish; end endmodule\n");
    assert!(out.contains("30 20 10"), "decl init desc; got:\n{out}");
}

#[test]
fn decl_init_offset_bounds() {
    let (out, _c) = run("module top; int a[1:3]='{10,20,30}; \
         initial begin $display(\"%0d %0d %0d\",a[1],a[2],a[3]); #1 $finish; end endmodule\n");
    assert!(out.contains("10 20 30"), "decl init offset; got:\n{out}");
}

#[test]
fn runtime_assign() {
    let (out, _c) = run(
        "module top; int a[3]; \
         initial begin a='{7,8,9}; $display(\"%0d %0d %0d\",a[0],a[1],a[2]); #1 $finish; end endmodule\n",
    );
    assert!(out.contains("7 8 9"), "runtime assign; got:\n{out}");
}

#[test]
fn expression_elements() {
    // Each element is an arbitrary self-determined expression.
    let (out, _c) = run(
        "module top; int x=5; int a[3]; \
         initial begin a='{x,x+1,x*2}; $display(\"%0d %0d %0d\",a[0],a[1],a[2]); #1 $finish; end endmodule\n",
    );
    assert!(out.contains("5 6 10"), "expr elements; got:\n{out}");
}

#[test]
fn logic_vector_array() {
    let (out, _c) = run("module top; logic [7:0] a[4]='{8'hAA,8'hBB,8'hCC,8'hDD}; \
         initial begin $display(\"%h %h %h %h\",a[0],a[1],a[2],a[3]); #1 $finish; end endmodule\n");
    assert!(out.contains("aa bb cc dd"), "logic array; got:\n{out}");
}

#[test]
fn fill_literal_elements_grow_to_element_width() {
    // §11.6: a bare fill literal element (`'1`/`'x`/`'z`) takes the array element's
    // width context. An adversarial review caught the first cut sizing `'1` to one
    // bit (then the engine zero-extended it to 8'h01 instead of 8'hFF).
    let (out, _c) = run(
        "module top; logic [7:0] a[3]; \
         initial begin a='{'1,8'hAA,'0}; $display(\"%h %h %h\",a[0],a[1],a[2]); #1 $finish; end endmodule\n",
    );
    assert!(
        out.contains("ff aa 00"),
        "fill literal element width; got:\n{out}"
    );
}

#[test]
fn size_mismatch_is_loud() {
    let (_o, code) = run("module top; int a[3]='{1,2}; initial $finish; endmodule\n");
    assert_ne!(code, Some(0), "element-count mismatch must be loud");
}

#[test]
fn packed_array_pattern_is_loud() {
    // A packed-array '{} (which iverilog reverses) is not yet supported — loud.
    let (_o, code) =
        run("module top; logic [2:0][7:0] p='{8'd1,8'd2,8'd3}; initial $finish; endmodule\n");
    assert_ne!(code, Some(0), "packed-array pattern must be loud");
}

#[test]
fn multidim_pattern_now_supported() {
    // Previously deferred (§4.5.33 "v1: a 1-D unpacked array"); a nested `'{…}` on
    // a multi-dimensional unpacked array is now supported (§4.5.46), row-major.
    let (o, code) = run(
        "module top; int a[2][2]='{'{1,2},'{3,4}};\n\
         initial begin $display(\"%0d %0d %0d %0d\", a[0][0],a[0][1],a[1][0],a[1][1]); $finish; end endmodule\n",
    );
    assert_eq!(code, Some(0), "got:\n{o}");
    assert!(o.contains("1 2 3 4"), "got:\n{o}");
}

#[test]
fn pattern_in_non_array_context_is_loud() {
    let (_o, code) = run("module top; int x; initial begin x=1+'{1,2}; $finish; end endmodule\n");
    assert_ne!(code, Some(0), "pattern in a sub-expression must be loud");
}

#[test]
fn default_pattern_fills_every_element() {
    // WAS `named_pattern_is_loud` (V34-3). The old test pinned the LIMITATION —
    // `'{default:7}` was a parse error — so it stayed green on a refusal without
    // ever saying what the right answer is. Re-measured 2026-08-26: verilator 5.050
    // prints `7 7 7`; iverilog 13 rejects the form outright (it rejects EVERY keyed
    // pattern, so it is not an oracle here). Now a VALUE assertion.
    let (o, code) = run("module top; int a[3]='{default:7};\n\
         initial begin $display(\"%0d %0d %0d\", a[0],a[1],a[2]); $finish; end endmodule\n");
    assert_eq!(code, Some(0), "got:\n{o}");
    assert!(o.contains("7 7 7"), "got:\n{o}");
}

#[test]
fn index_key_pattern_is_loud() {
    // The half of the old `named_pattern_is_loud` that is STILL loud: an integer
    // key. iverilog rejects it and verilator alone cannot settle index-vs-`default`
    // priority or a non-zero-based target, so vita refuses rather than guesses.
    let (_o, code) = run("module top; int a[3]='{0:1,1:2,2:3}; initial $finish; endmodule\n");
    assert_ne!(code, Some(0), "an integer-key pattern must be loud");
}
