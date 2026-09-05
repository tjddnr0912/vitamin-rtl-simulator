//! §4.5.422 (ROADMAP §2 🆕 L (aa)): the parse-time constant table's readers and the
//! declared-range bounds fold at the expression's WIDTH, not at i64. `localparam logic
//! [3:0] C = 15, D = 1;` — `C+D` is a 4-bit 0 in a self-determined position (a cast
//! bound, a generate index, a range bound), 16 under an `int` assignment context.
//! Expected lines are the output of iverilog 13.0 and verilator 5.050 (they agree on
//! every pinned line; the untyped `localparam G = C + D` is an oracle split — 16 / 0 —
//! and stays unpinned).

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_ctwa_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("t.sv")
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (s, out.status.code())
}

fn lines(src: &str, prefix: &str) -> Vec<String> {
    let (out, rc) = run(src);
    assert_eq!(rc, Some(0), "expected exit 0, got {rc:?}:\n{out}");
    out.lines()
        .filter(|l| l.starts_with(prefix))
        .map(|l| l.to_string())
        .collect()
}

const DECL: &str = "localparam logic [3:0] C = 15, D = 1;";

#[test]
fn cast_bound_generate_index_and_typed_values() {
    let src = format!("module top;\n  {DECL}\n  localparam int E = C + D;\n  localparam logic [3:0] F = C + D;\n  localparam logic [7:0] H = C + D;\n  localparam int S3 = D - C;\n  typedef logic [C+D:0] t;\n  t tv;\n  logic [11:0] src = 12'hfff;\n  genvar i;\n  generate for (i = 0; i < 20; i++) begin : g  wire [7:0] w = i + 8'd7; end endgenerate\n  initial begin\n    $display(\"D=E=%0d F=%0d H=%0d S3=%0d bits_t=%0d\", E, F, H, S3, $bits(t));\n    $display(\"D=cast=%h\", t'(src));\n    $display(\"D=g=%0d\", g[C+D].w);\n    #1 $finish;\n  end\nendmodule\n");
    assert_eq!(
        lines(&src, "D="),
        vec!["D=E=16 F=0 H=16 S3=-14 bits_t=1", "D=cast=1", "D=g=7"]
    );
}

#[test]
fn declared_range_bounds_are_self_determined() {
    // `R` is `[0:0]`, `v` one bit, `a` one element, `f`'s local one bit; the
    // packed-struct member (§4.5.418) is the control.
    let src = format!("module top;\n  {DECL}\n  localparam logic [C+D:0] R = 3;\n  logic [C+D:0] v;\n  logic a [C+D:0];\n  typedef struct packed {{ logic [C+D:0] f; logic g; }} st;\n  function automatic int f(); logic [C+D:0] t; return $bits(t); endfunction\n  initial begin\n    $display(\"D=R=%0d bitsR=%0d v=%0d a=%0d st=%0d f=%0d\", R, $bits(R), $bits(v), $size(a), $bits(st), f());\n    #1 $finish;\n  end\nendmodule\n");
    assert_eq!(lines(&src, "D="), vec!["D=R=1 bitsR=1 v=1 a=1 st=2 f=1"]);
}

#[test]
fn unsigned_underflow_bound_keeps_the_graceful_reading() {
    // `logic [W-1:0]` with an UNSIGNED 8-bit `W = 0` is an oracle split (verilator 2,
    // iverilog 0); vita keeps its pre-existing `[-1:0]` reading (ROADMAP §2 row 11)
    // rather than a width-cap error.
    let src = "module top;\n  localparam logic [7:0] W = 0; localparam int I = 0;\n  logic [W-1:0] v; logic [I-1:0] u;\n  initial begin $display(\"D=%0d %0d\", $bits(v), $bits(u)); #1 $finish; end\nendmodule\n";
    assert_eq!(lines(src, "D="), vec!["D=2 2"]);
}

#[test]
fn based_literal_narrow_parameter_is_recorded() {
    // §4.5.418 declined a based-literal parameter narrower than 32 bits from the table;
    // with width-aware readers it folds like its decimal twin (both oracles: 1 / 7).
    let src = "module top;\n  localparam logic [3:0] B = 4'hf, D = 4'h1;\n  typedef logic [B+D:0] t;\n  genvar i;\n  generate for (i = 0; i < 20; i++) begin : g  wire [7:0] w = i + 8'd7; end endgenerate\n  initial begin $display(\"D=%0d %0d\", $bits(t), g[B+D].w); #1 $finish; end\nendmodule\n";
    assert_eq!(lines(src, "D="), vec!["D=1 7"]);
}
