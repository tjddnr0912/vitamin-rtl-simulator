//! A generate-array hierarchical reference indexed by a constant — `g[P].x` /
//! `g[P].x = …` — where the index is a literal-valued `localparam` (not just a
//! bare decimal literal). The parser folds the index into the scope-segment name
//! `g[<value>]`. Only `localparam`s whose value is a pure literal constant fold
//! (they are non-overridable, so the value is fixed at parse time); a `parameter`
//! (overridable) or a param-derived value stays a loud parse error — folding it
//! at parse time could disagree with an instance override (a silent-wrong).
//! Oracle: iverilog -g2012.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn vita(src: &str) -> std::process::Output {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_ghl_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    out
}

fn run(src: &str) -> String {
    let out = vita(src);
    assert!(
        out.status.success(),
        "vita failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let so = String::from_utf8_lossy(&out.stdout).into_owned();
    let mut s = String::new();
    for l in so.lines().filter(|l| {
        !l.starts_with("simulation ended") && !l.contains("VITA-W1017") && !l.trim().is_empty()
    }) {
        s.push_str(l.trim());
        s.push('\n');
    }
    s
}

fn run_loud(src: &str) {
    let out = vita(src);
    assert!(
        !out.status.success(),
        "expected a loud refusal, but vita succeeded:\n{}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn localparam_indexed_generate_read() {
    // g[2].x = 2 + 10 = 12 (iverilog parity).
    let out = run("module top;\n\
        genvar i;\n\
        generate for (i=0;i<4;i=i+1) begin : g\n\
          wire [7:0] x = i + 10;\n\
        end endgenerate\n\
        localparam P = 2;\n\
        initial begin #1; $display(\"%0d\", g[P].x); end\n\
      endmodule\n");
    assert_eq!(out, "12\n");
}

#[test]
fn localparam_expr_indexed_generate_read() {
    // A literal-arithmetic localparam (`P = 1+2`) and a chained one fold too.
    let out = run("module top;\n\
        genvar i;\n\
        generate for (i=0;i<4;i=i+1) begin : g\n\
          wire [7:0] x = i + 10;\n\
        end endgenerate\n\
        localparam P = 1 + 2;\n\
        localparam Q = P - 1;\n\
        initial begin #1; $display(\"%0d %0d\", g[P].x, g[Q].x); end\n\
      endmodule\n");
    assert_eq!(out, "13 12\n");
}

#[test]
fn localparam_indexed_generate_matches_literal() {
    // g[P] with localparam P=3 must equal the bare-literal g[3].
    let out = run("module top;\n\
        genvar i;\n\
        generate for (i=0;i<4;i=i+1) begin : g\n\
          wire [7:0] x = i * 5;\n\
        end endgenerate\n\
        localparam P = 3;\n\
        initial begin #1; $display(\"%0d %0d\", g[P].x, g[3].x); end\n\
      endmodule\n");
    assert_eq!(out, "15 15\n");
}

#[test]
fn overridable_parameter_index_is_loud() {
    // A `parameter` (overridable via instance override) must NOT fold at parse
    // time — its declared value can disagree with an override. Stay loud.
    run_loud(
        "module top;\n\
        genvar i;\n\
        generate for (i=0;i<4;i=i+1) begin : g\n\
          wire [7:0] x = i + 10;\n\
        end endgenerate\n\
        parameter P = 2;\n\
        initial begin #1; $display(\"%0d\", g[P].x); end\n\
      endmodule\n",
    );
}

#[test]
fn param_derived_localparam_index_is_loud() {
    // A localparam derived from an overridable parameter is NOT a pure literal, so
    // it does not fold — its value depends on the (possibly overridden) parameter.
    run_loud(
        "module top #(parameter W = 2);\n\
        genvar i;\n\
        generate for (i=0;i<4;i=i+1) begin : g\n\
          wire [7:0] x = i + 10;\n\
        end endgenerate\n\
        localparam P = W;\n\
        initial begin #1; $display(\"%0d\", g[P].x); end\n\
      endmodule\n",
    );
}
