//! YELLOW #1 (combinational User-Defined Primitive, IEEE 1364 §29).
//!
//! `primitive … table … endtable endprimitive` with an `output` first port, N
//! `input`s, and a single-colon truth table over symbols `0 1 x ? b` (inputs) and
//! `0 1 x` (output). DESUGARED in the parser into a synthetic ordinary module: an
//! `always @(*)` whose if/else-if cascade matches each input column 4-state-EXACT
//! (`===`, NOT casez — casez would wildcard the scrutinee's x/z and silently
//! mis-match), resolves conflicting rows order-INDEPENDENTLY by priority 0 > 1 > x,
//! and yields x for any unmatched combination. Pure parser desugar (IR-0): no new
//! AST node, no `.vu` schema-hash flip, no `format_version` bump.
//!
//! Every output below is pinned to live iverilog 13.0 (UDPs are fully supported).
//! Sequential UDPs / `reg` output / `z` symbols / `z`,`-` outputs / multi-output /
//! wrong column count are honest-loud rejects (combinational only — sequential
//! UDPs are slice #9).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_udp_{}_{n}", std::process::id()));
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
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    )
}

/// All `$display` lines (in order) must appear in stdout.
fn expect_lines(out: &str, lines: &[&str]) {
    for l in lines {
        assert!(out.contains(l), "missing {l:?} in:\n{out}");
    }
}

#[test]
fn udp_and_xz_inputs() {
    // AND UDP; x/z inputs that don't match a defined-input row → x (NOT casez).
    let (out, err, code) = run("primitive p_and(o,a,b);\n\
           output o; input a,b;\n\
           table 1 1:1; 0 ?:0; ? 0:0; endtable\n\
         endprimitive\n\
         module top; reg a,b; wire o; p_and u(o,a,b);\n\
         initial begin\n\
           a=0;b=0;#1 $display(\"00->%b\",o);\n\
           a=1;b=1;#1 $display(\"11->%b\",o);\n\
           a=1'bx;b=1;#1 $display(\"x1->%b\",o);\n\
           a=1'bx;b=0;#1 $display(\"x0->%b\",o);\n\
           a=1'bz;b=1;#1 $display(\"z1->%b\",o);\n\
         end endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    expect_lines(&out, &["00->0", "11->1", "x1->x", "x0->0", "z1->x"]);
}

#[test]
fn udp_qmark_matches_z() {
    // `?` matches 0/1/x AND z.
    let (out, err, code) = run(
        "primitive p(o,a);\n\
           output o; input a;\n\
           table ?:1; endtable\n\
         endprimitive\n\
         module top; reg a; wire o; p u(o,a);\n\
         initial begin a=0;#1 $display(\"0->%b\",o); a=1'bx;#1 $display(\"x->%b\",o); a=1'bz;#1 $display(\"z->%b\",o); end endmodule\n",
    );
    assert_eq!(code, Some(0), "stderr:\n{err}");
    expect_lines(&out, &["0->1", "x->1", "z->1"]);
}

#[test]
fn udp_b_not_match_xz() {
    // `b` matches 0/1 only — NOT x, NOT z.
    let (out, err, code) = run("primitive p(o,a,b);\n\
           output o; input a,b;\n\
           table b 1:1; b 0:0; endtable\n\
         endprimitive\n\
         module top; reg a,b; wire o; p u(o,a,b);\n\
         initial begin\n\
           a=0;b=1;#1 $display(\"01->%b\",o);\n\
           a=1'bx;b=1;#1 $display(\"x1->%b\",o);\n\
           a=1'bz;b=1;#1 $display(\"z1->%b\",o);\n\
         end endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    expect_lines(&out, &["01->1", "x1->x", "z1->x"]);
}

#[test]
fn udp_conflict_priority_order_independent() {
    // KILLER #1: overlapping rows {output 1, output 0} on input 01 → 0 (0>1>x),
    // regardless of textual order (a first-match cascade would give 1).
    let body = |rows: &str| {
        format!(
            "primitive p(o,a,b);\n\
               output o; input a,b;\n\
               table {rows} endtable\n\
             endprimitive\n\
             module top; reg a,b; wire o; p u(o,a,b);\n\
             initial begin a=0;b=1;#1 $display(\"01->%b\",o); end endmodule\n"
        )
    };
    let (out1, _, c1) = run(&body("? 1:1; 0 ?:0;"));
    let (out2, _, c2) = run(&body("0 ?:0; ? 1:1;"));
    assert_eq!(c1, Some(0));
    assert_eq!(c2, Some(0));
    assert!(out1.contains("01->0"), "order A: {out1}");
    assert!(out2.contains("01->0"), "order B (reversed): {out2}");
}

#[test]
fn udp_z_selector_not_wildcarded() {
    // KILLER #2: z (or x) on the input must NOT be wildcarded against a 0/1 column
    // (the casez trap). Only `?` covers z; here no `?` → x.
    let (out, err, code) = run("primitive p(o,a,b);\n\
           output o; input a,b;\n\
           table 1 1:1; 0 0:0; endtable\n\
         endprimitive\n\
         module top; reg a,b; wire o; p u(o,a,b);\n\
         initial begin\n\
           a=1'bz;b=1;#1 $display(\"z1->%b\",o);\n\
           a=1'bx;b=1;#1 $display(\"x1->%b\",o);\n\
         end endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    expect_lines(&out, &["z1->x", "x1->x"]);
}

#[test]
fn udp_explicit_x_output_and_unmatched() {
    // explicit `x` output row is emitted; an omitted combination also → x.
    let (out, err, code) = run("primitive p(o,a,b);\n\
           output o; input a,b;\n\
           table 1 0:x; 0 0:0; 1 1:1; endtable\n\
         endprimitive\n\
         module top; reg a,b; wire o; p u(o,a,b);\n\
         initial begin\n\
           a=1;b=0;#1 $display(\"10->%b\",o);\n\
           a=0;b=1;#1 $display(\"01->%b\",o);\n\
           a=0;b=0;#1 $display(\"00->%b\",o);\n\
         end endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    expect_lines(&out, &["10->x", "01->x", "00->0"]);
}

#[test]
fn udp_compact_lexing() {
    // compact `111:1` (one int token) + mixed spacing + in-table comments decompose
    // to one symbol per input column.
    // Non-overlapping rows (so the test isolates lexing, not conflict resolution):
    // compact `111`/`101`, mixed-spacing `0 ? ?`, and a `/* */` in-table comment.
    let (out, err, code) = run("primitive p(o,a,b,c);\n\
           output o; input a,b,c;\n\
           table\n\
             111:1;  /* all ones */\n\
             101:1;\n\
             100:0;\n\
             0 ? ? :0;\n\
           endtable\n\
         endprimitive\n\
         module top; reg a,b,c; wire o; p u(o,a,b,c);\n\
         initial begin\n\
           a=1;b=1;c=1;#1 $display(\"111->%b\",o);\n\
           a=1;b=0;c=1;#1 $display(\"101->%b\",o);\n\
           a=1;b=0;c=0;#1 $display(\"100->%b\",o);\n\
           a=0;b=1;c=1;#1 $display(\"011->%b\",o);\n\
         end endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    expect_lines(&out, &["111->1", "101->1", "100->0", "011->0"]);
}

#[test]
fn udp_instance_port_order_output_first() {
    // The first port is the output; swapping the two input args swaps the result.
    let (out, err, code) = run("primitive sub2(o,a,b);\n\
           output o; input a,b;\n\
           table 1 0:1; 0 ?:0; 1 1:0; endtable\n\
         endprimitive\n\
         module top; reg a,b; wire o1,o2;\n\
           sub2 i1(o1,a,b);\n\
           sub2 i2(o2,b,a);\n\
         initial begin a=1;b=0;#1 $display(\"o1=%b o2=%b\",o1,o2); end endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    // a=1,b=0: i1(a,b)=sub2(1,0)=1 ; i2(b,a)=sub2(0,1)=0.
    expect_lines(&out, &["o1=1 o2=0"]);
}

#[test]
fn udp_x_literal_column_matches_only_x() {
    let (out, err, code) = run("primitive p(o,a,b,c);\n\
           output o; input a,b,c;\n\
           table x ? ?:1; endtable\n\
         endprimitive\n\
         module top; reg a,b,c; wire o; p u(o,a,b,c);\n\
         initial begin\n\
           a=1'bx;b=1'bx;c=1'bx;#1 $display(\"xxx->%b\",o);\n\
           a=1'bx;b=0;c=0;#1 $display(\"x00->%b\",o);\n\
           a=0;b=1'bx;c=0;#1 $display(\"0x0->%b\",o);\n\
         end endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    expect_lines(&out, &["xxx->1", "x00->1", "0x0->x"]);
}

#[test]
fn udp_x_symbol_matches_z_input() {
    // IEEE 1364 §29.3.4: a `z` on a UDP input is treated as `x` for table matching,
    // so an `x` table symbol matches BOTH x and z. (Caught by post-impl adversarial
    // review — a bare `=== 1'bx` would miss the z input → silent-wrong.)
    let (out, err, code) = run("primitive p(o,a,b);\n\
           output o; input a,b;\n\
           table 0 0:0; 1 1:1; x x:1; x 0:0; 0 x:1; endtable\n\
         endprimitive\n\
         module top; reg a,b; wire o; p u(o,a,b);\n\
         initial begin\n\
           a=1'bz;b=1'bz;#1 $display(\"zz->%b\",o);\n\
           a=1'bz;b=0;#1 $display(\"z0->%b\",o);\n\
           a=0;b=1'bz;#1 $display(\"0z->%b\",o);\n\
           a=1'bx;b=1'bz;#1 $display(\"xz->%b\",o);\n\
         end endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    // z folds to x: zz matches `x x`→1, z0 matches `x 0`→0, 0z matches `0 x`→1,
    // xz matches `x x`→1.
    expect_lines(&out, &["zz->1", "z0->0", "0z->1", "xz->1"]);
}

#[test]
fn udp_z_flips_conflict_resolution() {
    // z→x conversion must feed conflict resolution: `x 1:0` (z→x) beats `? ?:1`
    // by 0>1 priority, so a=z,b=1 → 0 (not 1).
    let (out, err, code) = run("primitive p(o,a,b);\n\
           output o; input a,b;\n\
           table ? ?:1; x 1:0; endtable\n\
         endprimitive\n\
         module top; reg a,b; wire o; p u(o,a,b);\n\
         initial begin a=1'bz;b=1;#1 $display(\"z1->%b\",o); end endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    expect_lines(&out, &["z1->0"]);
}

// ─────────────────────────── honest-loud rejects ───────────────────────────

fn rejects(src: &str, needle: &str) {
    let (_out, err, code) = run(src);
    assert_ne!(
        code,
        Some(0),
        "expected loud reject, succeeded. stderr:\n{err}"
    );
    assert!(err.contains(needle), "wrong reject reason; stderr:\n{err}");
}

#[test]
fn udp_reg_output_is_now_sequential() {
    // YELLOW #9: `output reg` + a two-colon row is a SEQUENTIAL UDP — no longer a
    // loud reject (it was "not yet supported" before slice #9). It now compiles and
    // runs (full sequential coverage lives in udp_seq.rs).
    let (out, err, code) = run("primitive p(o,a);\n\
           output reg o; input a;\n\
           table 0:?:0; 1:?:1; endtable\n\
         endprimitive\n\
         module top; reg a; wire o; p u(o,a);\n\
         initial begin a=0;#1 a=1;#1 $display(\"seq ok=%b\",o); end endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    expect_lines(&out, &["seq ok=1"]);
}

#[test]
fn udp_multi_output_is_loud() {
    rejects(
        "primitive p(o1,o2,a);\n\
           output o1,o2; input a;\n\
           table 0:0; endtable\n\
         endprimitive\n\
         module top; reg a; wire o1,o2; p u(o1,o2,a); initial $display(\"x\"); endmodule\n",
        "exactly one UDP output",
    );
}

#[test]
fn udp_z_output_is_loud() {
    rejects(
        "primitive p(o,a);\n\
           output o; input a;\n\
           table 0:z; endtable\n\
         endprimitive\n\
         module top; reg a; wire o; p u(o,a); initial $display(\"x\"); endmodule\n",
        "next-state symbol (0 1 x -)",
    );
}

#[test]
fn udp_z_input_is_loud() {
    rejects(
        "primitive p(o,a);\n\
           output o; input a;\n\
           table z:0; endtable\n\
         endprimitive\n\
         module top; reg a; wire o; p u(o,a); initial $display(\"x\"); endmodule\n",
        "input symbol (0 1 x ? b r f p n * or (vw))",
    );
}

#[test]
fn udp_wrong_column_count_is_loud() {
    rejects(
        "primitive p(o,a,b);\n\
           output o; input a,b;\n\
           table 0:0; endtable\n\
         endprimitive\n\
         module top; reg a,b; wire o; p u(o,a,b); initial $display(\"x\"); endmodule\n",
        "one symbol per input port",
    );
}

#[test]
fn udp_empty_table_is_loud() {
    // An empty `table … endtable` (iverilog: "Empty UDP table") must be rejected,
    // not silently synthesized into an always-x primitive. (Adversarial-review gap.)
    rejects(
        "primitive p(o,a);\n\
           output o; input a;\n\
           table endtable\n\
         endprimitive\n\
         module top; reg a; wire o; p u(o,a); initial $display(\"x\"); endmodule\n",
        "non-empty UDP table",
    );
}
