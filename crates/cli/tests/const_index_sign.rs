//! A CONSTANT array index carries its SIGN. §4.5.310 deliberately left a
//! compile-time-constant index on the pre-seal path so that a statically
//! out-of-range index stays out of range and is DIAGNOSED rather than wrapping
//! into a neighbour — but "its true value" was read as bits, and the engine takes
//! an index through `to_u64`, so a negative constant narrower than the 32-bit
//! index domain arrived positive.
//!
//! `logic [7:0] arr[0:255]; arr[-8'sd1]` read element 255 at exit 0 where
//! iverilog returns `xx`. So did `arr[8'sd255]` (the same bits, spelled
//! positive — `8'sd255` IS −1) and `sm[-3'sd1]` on an 8-element array.
//!
//! ⚠️ The bug hid behind a coincidence: it only shows when the UNSIGNED reading
//! also lands in range. `sm[-4'sd1]` on `[0:7]` is 15 — already out — so it was
//! correct for the wrong reason, and a NET index has been sign-extended by the
//! seal since §4.5.309, so only the constant carve-out was ever wrong.
//!
//! Sign-extending the constant to the index domain keeps the carve-out's promise:
//! a negative constant becomes an index no array can hold, so it stays loud.
//!
//! A 2,790-cell sweep (6 array extents incl. a negative base x 6 index widths x
//! signed/unsigned x 11 values x 5 spellings) measured FIXED 159, REGRESSED 0,
//! still-wrong 0 against iverilog 13.
//!
//! ORACLE: iverilog 13.0 — every value below was run through it.

use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_args(src: &str, args: &[&str]) -> (String, String) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_cis_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let mut c = std::process::Command::new(env!("CARGO_BIN_EXE_vita"));
    for a in args {
        c.arg(a);
    }
    let out = c
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let so = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.contains("simulation ended"))
        .collect::<Vec<_>>()
        .join("\n");
    let se = String::from_utf8_lossy(&out.stderr).into_owned();
    let _ = std::fs::remove_dir_all(&d);
    (so, se)
}

fn run(src: &str) -> String {
    run_args(src, &[]).0
}

const ARR: &str = r#"module t;
  logic [7:0] arr [0:255];
  logic [7:0] sm  [0:7];
  integer i;
  logic [7:0] r;
"#;

/// The headline, and the row that shows the coincidence. Rows 1-3 were silent
/// reads of a neighbouring element; row 4 was correct only because 15 is already
/// outside `[0:7]`.
#[test]
fn a_negative_constant_index_is_out_of_range() {
    let o = run(&format!(
        r#"{ARR}  initial begin
    for (i = 0; i < 256; i = i + 1) arr[i] = i[7:0] ^ 8'h5a;
    for (i = 0; i < 8;   i = i + 1) sm[i]  = i[7:0] | 8'hA0;
    #1;
    r = arr[-8'sd1];  $display("%h", r);
    r = arr[8'sd255]; $display("%h", r);
    r = sm[-3'sd1];   $display("%h", r);
    r = sm[-4'sd1];   $display("%h", r);
    #1 $finish;
  end
endmodule
"#
    ));
    // iverilog. PRE: a5 / a5 / a7 / xx — the fourth was already right.
    assert_eq!(o, "xx\nxx\nxx\nxx");
}

/// It must stay LOUD, not merely return x. The carve-out exists so a statically
/// out-of-range index is diagnosed; before the fix three of these four reads were
/// silent, and `errors` counted 1 where it should count 4.
#[test]
fn a_negative_constant_index_is_diagnosed() {
    let (_, err) = run_args(
        &format!(
            r#"{ARR}  initial begin
    #1;
    r = arr[-8'sd1];  r = arr[8'sd255]; r = sm[-3'sd1]; r = sm[-4'sd1];
    #1 $finish;
  end
endmodule
"#
        ),
        &[],
    );
    assert_eq!(err.matches("VITA-E4002").count(), 4, "stderr was: {err}");
}

/// ANTI-VACUITY. A POSITIVE signed constant, an UNSIGNED constant whose top bit
/// is set, and a plain unsized literal must all be untouched — sign-extending
/// must not turn an in-range index out of range.
#[test]
fn positive_and_unsigned_constant_indices_are_unchanged() {
    let o = run(&format!(
        r#"{ARR}  initial begin
    for (i = 0; i < 256; i = i + 1) arr[i] = i[7:0] ^ 8'h5a;
    for (i = 0; i < 8;   i = i + 1) sm[i]  = i[7:0] | 8'hA0;
    #1;
    r = arr[8'sd127]; $display("%h", r);
    r = arr[8'd255];  $display("%h", r);
    r = arr[255];     $display("%h", r);
    r = sm[3'sd3];    $display("%h", r);
    r = sm[3'd7];     $display("%h", r);
    #1 $finish;
  end
endmodule
"#
    ));
    // iverilog; PRE identical on all five.
    assert_eq!(o, "25\na5\na5\na3\na7");
}

/// The recognizer is VALUE-based, not spelling-based (§4.5.310) — so every
/// spelling of the same negative constant must answer alike. A spelling that
/// escaped the recognizer would fall to the runtime path and wrap silently, which
/// is the exact trap §4.5.310 was written to close.
#[test]
fn every_spelling_of_the_same_negative_constant_agrees() {
    let o = run(&format!(
        r#"{ARR}  initial begin
    for (i = 0; i < 256; i = i + 1) arr[i] = i[7:0] ^ 8'h5a;
    #1;
    r = arr[-8'sd1];              $display("%h", r);
    r = arr[(8'sd0 - 8'sd1)];     $display("%h", r);
    r = arr[$signed(-8'sd1)];     $display("%h", r);
    r = arr[8'(-8'sd1)];          $display("%h", r);
    r = arr[~8'sd0];              $display("%h", r);
    r = arr[1 ? -8'sd1 : 8'sd0];  $display("%h", r);
    #1 $finish;
  end
endmodule
"#
    ));
    assert_eq!(o, "xx\nxx\nxx\nxx\nxx\nxx"); // iverilog
}

/// A NET index of the same value was already right (sealed sign-extended since
/// §4.5.309) — pinned so the constant fix cannot be mistaken for the whole story,
/// and so a regression in the net path shows up here too.
#[test]
fn a_net_index_of_the_same_value_agrees_with_the_constant_one() {
    let o = run(&format!(
        r#"{ARR}  logic signed [7:0] s8; logic signed [2:0] s3;
  initial begin
    for (i = 0; i < 256; i = i + 1) arr[i] = i[7:0] ^ 8'h5a;
    for (i = 0; i < 8;   i = i + 1) sm[i]  = i[7:0] | 8'hA0;
    s8 = -8'sd1; s3 = -3'sd1;
    #1;
    r = arr[s8];     $display("%h", r);
    r = sm[s3];      $display("%h", r);
    r = arr[-8'sd1]; $display("%h", r);
    r = sm[-3'sd1];  $display("%h", r);
    #1 $finish;
  end
endmodule
"#
    ));
    assert_eq!(o, "xx\nxx\nxx\nxx"); // iverilog; PRE: xx / xx / a5 / a7
}

/// A negative declared base is the shape where a negative index is LEGAL, so the
/// fix must not make it loud: `arr[-3]` on `[-3:2]` is element 0.
#[test]
fn a_negative_index_into_a_negative_based_array_still_lands() {
    let o = run(r#"module t;
  logic [7:0] g [-3:2];
  integer i;
  logic [7:0] r;
  initial begin
    for (i = -3; i <= 2; i = i + 1) g[i] = (i + 8) & 8'hFF;
    #1;
    r = g[-3'sd3];  $display("%h", r);
    r = g[-32'sd1]; $display("%h", r);
    r = g[2];       $display("%h", r);
    #1 $finish;
  end
endmodule
"#);
    assert_eq!(o, "05\n07\n0a"); // iverilog; PRE identical
}

/// The three backends must agree — the seal is an extra IR node on the index.
#[test]
fn the_three_backends_agree_on_a_signed_constant_index() {
    let src = format!(
        r#"{ARR}  initial begin
    for (i = 0; i < 256; i = i + 1) arr[i] = i[7:0] ^ 8'h5a;
    #1;
    r = arr[-8'sd1];  $display("%h", r);
    r = arr[8'sd127]; $display("%h", r);
    #1 $finish;
  end
endmodule
"#
    );
    let want = "xx\n25";
    assert_eq!(run_args(&src, &["--backend", "interp"]).0, want);
    assert_eq!(run_args(&src, &["--backend", "bytecode"]).0, want);
    assert_eq!(run_args(&src, &["--backend", "native"]).0, want);
}

/// The width guard `sw.width < 32` is load-bearing TWICE. Semantically a >= 32-bit
/// constant already carries its sign in the index domain, and structurally
/// `extend_to` computes `n - w`, so calling it at width 64 underflows. This row is
/// the only thing in the repo that walks a 64-bit signed constant index through
/// the branch.
///
/// ⚠️ Width 31 cannot be pinned and no row here pretends to: a narrow signed
/// constant's UNSIGNED reading is `2^w - |v|`, so discriminating width 31 would
/// need an array of two billion elements. Mutants that move the bound to 31 are
/// equivalent in practice, not untested.
#[test]
fn a_wide_signed_constant_index_takes_no_extension() {
    let o = run(&format!(
        r#"{ARR}  initial begin
    for (i = 0; i < 256; i = i + 1) arr[i] = i[7:0] ^ 8'h5a;
    #1;
    r = arr[-64'sd1];  $display("%h", r);
    r = arr[-32'sd1];  $display("%h", r);
    r = arr[-31'sd1];  $display("%h", r);
    r = arr[32'sd127]; $display("%h", r);
    #1 $finish;
  end
endmodule
"#
    ));
    assert_eq!(o, "xx\nxx\nxx\n25"); // iverilog; PRE identical
}

/// A ONE-BIT signed constant is `-1`, and it was reading element 1. The narrowest
/// index is also the one a width-lower-bound mutant would exclude.
#[test]
fn a_one_bit_signed_constant_index_is_minus_one() {
    let o = run(&format!(
        r#"{ARR}  initial begin
    for (i = 0; i < 256; i = i + 1) arr[i] = i[7:0] ^ 8'h5a;
    #1;
    r = arr[1'sb1]; $display("%h", r);
    r = arr[1'b1];  $display("%h", r);
    #1 $finish;
  end
endmodule
"#
    ));
    // iverilog. PRE: 5b / 5b — the UNSIGNED spelling is element 1 and stays there.
    assert_eq!(o, "xx\n5b");
}

/// Multi-dimensional, with a negative declared base in both dims: a legal negative
/// index must still land, and an out-of-range one in EITHER dim must not.
#[test]
fn a_multi_dim_negative_base_array_keeps_its_legal_indices() {
    let o = run(r#"module t;
  logic [7:0] g2 [-2:1][-1:2];
  integer i, j;
  logic [7:0] r;
  initial begin
    for (i = -2; i <= 1; i = i + 1)
      for (j = -1; j <= 2; j = j + 1) g2[i][j] = ((i+2)*4 + (j+1)) | 8'h40;
    #1;
    r = g2[-2'sd2][-1'sb1]; $display("%h", r);
    r = g2[-2'sd1][2'sd2];  $display("%h", r);
    r = g2[1][2];           $display("%h", r);
    #1 $finish;
  end
endmodule
"#);
    // iverilog. Row 1 is legal (both bases negative) and PRE-identical; row 2's
    // `2'sd2` is -2, outside `[-1:2]` — PRE read `47`.
    assert_eq!(o, "40\nxx\n4f");
}

/// ⚠️ A FUNCTION-CALL index of the same value is still silent. The runtime seal
/// declines a `Call` (it would be evaluated twice by the sign fill), so this shape
/// never reaches either seal — it is not the constant carve-out and this slice does
/// not close it. iverilog says `xx`. ROADMAP §2.
#[test]
fn a_function_call_index_is_a_documented_gap() {
    let o = run(&format!(
        r#"{ARR}  function automatic signed [7:0] fneg(input d); fneg = -8'sd1; endfunction
  initial begin
    for (i = 0; i < 256; i = i + 1) arr[i] = i[7:0] ^ 8'h5a;
    #1;
    r = arr[fneg(0)]; $display("%h", r);
    #1 $finish;
  end
endmodule
"#
    ));
    assert_eq!(o, "a5"); // PRE identical; iverilog xx
}
