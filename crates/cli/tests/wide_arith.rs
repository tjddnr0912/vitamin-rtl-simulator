//! Multi-word arithmetic (Phase-1.x precision item ⑥): signed-beyond-64-bit
//! and unsigned-beyond-128-bit add/sub/mul/div/mod/pow were X-poisoned (an
//! honest "unsupported"); they now compute exactly. Every expected value
//! below is pinned LIVE against iverilog 13.0 (2026-06-12) — wide arithmetic
//! is fully differential-able (unlike the assoc/array-assign lanes).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_wa_{}_{n}", std::process::id()));
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

#[test]
fn unsigned_256_add_carries_across_words() {
    let (out, err, code) = run("module top;\n\
         reg [255:0] a, b, s;\n\
         initial begin\n\
           a = 256'h1 << 200; b = (256'h1 << 200) + 256'd7;\n\
           s = a + b;\n\
           $display(\"add=%h\", s);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(
        out.contains("add=0000000000000200000000000000000000000000000000000000000000000007"),
        "got:\n{out}"
    );
}

#[test]
fn unsigned_256_sub_borrows() {
    // (1<<200) - 7 : borrow ripples down through three zero words.
    let (out, err, code) = run("module top;\n\
         reg [255:0] s;\n\
         initial begin\n\
           s = (256'h1 << 200) - 256'd7;\n\
           $display(\"sub=%h\", s);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(
        out.contains("sub=00000000000000fffffffffffffffffffffffffffffffffffffffffffffffff9"),
        "got:\n{out}"
    );
}

#[test]
fn unsigned_256_mul_truncates_mod_2_256() {
    // (2^130)·(2^130+3) = 2^260 + 3·2^130 ≡ 3·2^130 (mod 2^256) — iverilog: c<<128.
    let (out, err, code) = run("module top;\n\
         reg [255:0] m;\n\
         initial begin\n\
           m = (256'h1 << 130) * ((256'h1 << 130) + 256'd3);\n\
           $display(\"mul=%h\", m);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(
        out.contains("mul=0000000000000000000000000000000c00000000000000000000000000000000"),
        "got:\n{out}"
    );
}

#[test]
fn unsigned_256_div_mod_wide_divisor() {
    let (out, err, code) = run("module top;\n\
         reg [255:0] q, r;\n\
         initial begin\n\
           q = ((256'h1 << 150) + 256'd12345) / ((256'h1 << 70) + 256'd1);\n\
           r = ((256'h1 << 150) + 256'd12345) % ((256'h1 << 70) + 256'd1);\n\
           $display(\"div=%h mod=%h\", q, r);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(
        out.contains("div=00000000000000000000000000000000000000000000fffffffffffffffffc00"),
        "got:\n{out}"
    );
    assert!(
        out.contains("mod=0000000000000000000000000000000000000000000000000000000000003439"),
        "got:\n{out}"
    );
}

#[test]
fn unsigned_192_div_short_divisor() {
    // Divisor fits one word → the O(n) short-division path.
    let (out, err, code) = run("module top;\n\
         reg [191:0] q, r;\n\
         initial begin\n\
           q = ((192'h1 << 130) + 192'd999) / 192'd1000003;\n\
           r = ((192'h1 << 130) + 192'd999) % 192'd1000003;\n\
           $display(\"q=%h r=%0d\", q, r);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    // iverilog-pinned live: q=…431bd151272d671a37965032ece8, r=13103.
    assert!(
        out.contains("q=00000000000000000000431bd151272d671a37965032ece8"),
        "got:\n{out}"
    );
    assert!(out.contains("r=13103"), "got:\n{out}");
}

#[test]
fn signed_128_div_mod_signs() {
    // IEEE: quotient truncates toward zero; remainder takes the DIVIDEND sign.
    let (out, err, code) = run("module top;\n\
         reg signed [127:0] sq, sr;\n\
         initial begin\n\
           sq = -128'sd5 / 128'sd3; sr = -128'sd5 % 128'sd3;\n\
           $display(\"sdiv=%0d smod=%0d\", sq, sr);\n\
           sq = 128'sd5 / -128'sd3; sr = 128'sd5 % -128'sd3;\n\
           $display(\"sdiv2=%0d smod2=%0d\", sq, sr);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("sdiv=-1 smod=-2"), "got:\n{out}");
    assert!(out.contains("sdiv2=-1 smod2=2"), "got:\n{out}");
}

#[test]
fn signed_128_add_sign_extends() {
    let (out, err, code) = run("module top;\n\
         reg signed [127:0] ss;\n\
         initial begin\n\
           ss = (-128'sd1 << 100) + 128'sd9;\n\
           $display(\"sadd=%h\", ss);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(
        out.contains("sadd=fffffff0000000000000000000000009"),
        "got:\n{out}"
    );
}

#[test]
fn signed_96_mul_wraps_two_complement() {
    // (-3) * 5 at 96 bits = 0xfff...f1 (-15) — school mul mod 2^96 is
    // sign-agnostic; iverilog-pinned.
    let (out, err, code) = run("module top;\n\
         reg signed [95:0] sm;\n\
         initial begin\n\
           sm = -96'sd3 * 96'sd5;\n\
           $display(\"smul=%h %0d\", sm, sm);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(
        out.contains("smul=fffffffffffffffffffffff1 -15"),
        "got:\n{out}"
    );
}

#[test]
fn unsigned_256_pow_square_multiplies() {
    // 3^170 mod 2^256 — iverilog-pinned.
    let (out, err, code) = run("module top;\n\
         reg [255:0] p;\n\
         initial begin\n\
           p = 256'd3 ** 256'd170;\n\
           $display(\"pow=%h\", p);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(
        out.contains("pow=433c91af7921692ef71435894b14a4434286f9e09132c1ce47f6356ef132b329"),
        "got:\n{out}"
    );
}

#[test]
fn wide_div_by_zero_is_x() {
    let (out, err, code) = run("module top;\n\
         reg [255:0] q;\n\
         reg signed [127:0] sq;\n\
         initial begin\n\
           q = (256'h1 << 200) / 256'd0;\n\
           sq = 128'sd5 / 128'sd0;\n\
           $display(\"q=%h sq=%h\", q, sq);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(
        out.contains("q=xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"),
        "got:\n{out}"
    );
    assert!(
        out.contains("sq=xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"),
        "got:\n{out}"
    );
}

#[test]
fn wide_x_operand_still_poisons() {
    let (out, err, code) = run("module top;\n\
         reg [255:0] a, s;\n\
         initial begin\n\
           s = a + 256'd1;\n\
           $display(\"s=%h\", s);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(
        out.contains("s=xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"),
        "got:\n{out}"
    );
}
