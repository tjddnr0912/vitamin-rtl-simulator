//! §4.5.418 review B F1 + the census control twin: a packed-struct member width is
//! folded at the width IEEE §11.6 gives the expression (`try_const_index_w`), so a
//! constant's arithmetic WRAPS as it does in both oracles — `int unsigned W =
//! 32'hffff_ffff` then `[$clog2(W+2):0]` (32-bit `W+2` is 1), a 33-bit range, and two
//! 4-bit constants whose sum overflows (`[C+D:0]` is one bit). The based and decimal
//! spellings are pinned side by side (the decimal one was a pre-existing silent 35 /
//! 17). Oracle lines from iverilog 13.0 and verilator 5.050.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_smww_{}_{n}", std::process::id()));
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

fn digest(name: &str, src: &str, expect: &str) {
    let (out, rc) = run(src);
    assert_eq!(rc, Some(0), "{name}: expected exit 0, got {rc:?}:\n{out}");
    let v: Vec<&str> = out
        .lines()
        .filter_map(|l| l.strip_prefix("DIGEST="))
        .collect();
    assert_eq!(v.join("|"), expect, "{name}:\n{out}");
}

fn wrap_src(decl: &str, bound: &str) -> String {
    format!(
        "module tb;\n  {decl}\n  typedef struct packed {{ logic [{bound}:0] a; logic c; }} s_t;\n  s_t s;\n  initial begin s = '0; s.a = '1; s.c = 1; $display(\"DIGEST=%0d %0d %h %b\", $bits(s), $bits(s.a), s, s.c); end\n  initial #5 $finish;\nendmodule\n"
    )
}

#[test]
fn member_width_wraps_at_32_bits() {
    // both oracles: `2 1 3 1`
    for (name, decl) in [
        ("q1_wrap_based", "localparam int unsigned W = 32'hffffffff;"),
        ("q1_wrap_dec", "localparam int unsigned W = 4294967295;"),
    ] {
        digest(name, &wrap_src(decl, "$clog2(W+2)"), "2 1 3 1");
    }
}

#[test]
fn member_width_wraps_at_33_bits() {
    // both oracles: `2 1 3 1`
    for (name, decl) in [
        (
            "q2_w33_based",
            "localparam logic [32:0] W = 33'h1_ffff_ffff;",
        ),
        ("q2_w33_dec", "localparam logic [32:0] W = 8589934591;"),
    ] {
        digest(name, &wrap_src(decl, "$clog2(W+2)"), "2 1 3 1");
    }
}

#[test]
fn member_width_narrow_sum_wraps() {
    // both oracles: `2 0 2` (a 4-bit `15 + 1` is 0 — the member is `[0:0]`)
    let src = "module tb;\n  localparam logic [3:0] C = 15;\n  localparam logic [3:0] D = 1;\n  typedef struct packed { logic [C+D:0] a; logic c; } s_t;\n  s_t s;\n  initial begin s = '0; s.a = '1; $display(\"DIGEST=%0d %0d %h\", $bits(s), $bits(s.a) - 1, s); end\n  initial #5 $finish;\nendmodule\n";
    digest("narrow_sum_dec", src, "2 0 2");
    // the typedef spelling of the declared type (review A p01c)
    let src2 = src.replace(
        "localparam logic [3:0] C = 15;\n  localparam logic [3:0] D = 1;",
        "typedef logic [3:0] n_t;\n  localparam n_t C = 15, D = 1;",
    );
    digest("narrow_sum_typedef", &src2, "2 0 2");
    // mixed widths: a 32-bit decimal operand lifts the sum to 32 bits (no wrap);
    // both oracles: `18 16 3fffe`
    let src3 = src.replace("[C+D:0]", "[C+1:0]");
    digest("narrow_plus_decimal", &src3, "18 16 3fffe");
}
