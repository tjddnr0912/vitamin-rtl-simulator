//! An unpacked ARRAY-WORD index is a self-determined expression whose value is
//! then read as a thirty-two-bit integer — and vita was reading it three
//! different wrong ways.
//!
//! Measured against BOTH iverilog 13 and verilator 5.050 (36-cell matrix over
//! four declared ranges × nine index shapes). Ten cells had the two oracles
//! agreeing with each other and disagreeing with vita; on the twenty-two cells
//! where the oracles disagree, vita already matched iverilog and still does —
//! verilator there is masking an out-of-range index into a power-of-two-padded
//! array and returning whatever it finds, which is not an answer.
//!
//! The three causes, each its own row group below:
//!
//!   1. NOT SELF-DETERMINED. `m[~s8]` widened `s8` before applying `~`, so the
//!      index was `0xFFFF_FF05` instead of 5.
//!   2. WIDENED PAST THIRTY-TWO. The seal pinned an unsigned index to its own
//!      width PLUS ONE, making the normalization thirty-three bits and removing
//!      a wrap: under a negative base `0xFFFF_FFFD + 3` is 0 (element `[-3]`) at
//!      thirty-two bits and 4294967296 — dropped — at thirty-three.
//!   3. NEVER TRUNCATED. A wider-than-thirty-two index kept all its bits, so
//!      `64'h1_0000_0002` landed nowhere; both oracles read element 2.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_awi_{}_{n}", std::process::id()));
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

/// Reads. Each row is a cell both oracles agree on; the `X`-suffixed rows are
/// the controls — genuinely out of range in every reading, so a change that
/// simply stops bounds-checking cannot pass this test.
#[test]
fn an_array_word_index_is_self_determined_then_read_as_i32() {
    let src = "module top;\n\
       reg [7:0] ma [-3:2];\n\
       reg [7:0] mz [0:5];\n\
       reg [7:0] mp [2:5];\n\
       reg signed [7:0] s8; reg [31:0] u32; reg [63:0] bg; integer i32, q;\n\
       initial begin\n\
         for (q=-3; q<=2; q=q+1) ma[q] = (q+40);\n\
         for (q=0;  q<=5; q=q+1) mz[q] = (q+50);\n\
         for (q=2;  q<=5; q=q+1) mp[q] = (q+70);\n\
         s8 = -8'sd6; u32 = 32'hFFFF_FFFD; bg = 64'h1_0000_0002; i32 = -1;\n\
         $display(\"S %0d\", mp[~s8]);\n\
         $display(\"W %0d\", ma[u32]);\n\
         $display(\"B %0d\", mz[bg]);\n\
         $display(\"M %0d\", ma[(u32 + 32'd0)]);\n\
         $display(\"I %0d\", ma[i32]);\n\
         $display(\"XA %0d\", mz[u32]);\n\
         $display(\"XB %0d\", mz[i32]);\n\
         $finish;\n\
       end\n\
     endmodule\n";
    let (out, code) = run(src);
    for (tag, want) in [
        // `~s8` is 5 at the index's own eight bits — mp[5] = 75. The array
        // must have a NON-ZERO base: at base 0 `dim_coord` emits no arithmetic
        // at all, so there is no widening context and the row passes even
        // unfixed (measured — the first version of this test used `mz [0:5]`
        // and was vacuous).
        ("S", "75"),
        // 0xFFFF_FFFD read as i32 is -3 — ma[-3] = 37. Was `x` plus an E4002.
        ("W", "37"),
        // Low thirty-two bits of 64'h1_0000_0002 is 2 — mz[2] = 52. Was `x`.
        ("B", "52"),
        // …and the same through an arithmetic expression, not just a load.
        ("M", "37"),
        // -1 was already right and stays right (the signed control).
        ("I", "39"),
        // Controls: -3 and -1 are out of `[0:5]` in every reading.
        ("XA", "x"),
        ("XB", "x"),
    ] {
        assert!(
            out.lines().any(|l| l == format!("{tag} {want}")),
            "line `{tag} {want}` missing\n{out}"
        );
    }
    // The two controls are genuine out-of-range reads, so the run stays loud.
    assert_eq!(code, Some(1), "the control rows must still be loud\n{out}");
}

/// Writes, on the funnel that drops silently rather than reading `x`.
#[test]
fn an_array_word_index_write_lands_where_both_oracles_put_it() {
    let src = "module top;\n\
       reg [7:0] ma [-3:2];\n\
       reg [7:0] mz [0:5];\n\
       reg signed [7:0] s8; reg [31:0] u32; reg [63:0] bg; integer q;\n\
       initial begin\n\
         for (q=-3; q<=2; q=q+1) ma[q] = 8'd0;\n\
         for (q=0;  q<=5; q=q+1) mz[q] = 8'd0;\n\
         s8 = -8'sd6; u32 = 32'hFFFF_FFFD; bg = 64'h1_0000_0002;\n\
         mz[~s8] = 8'd11;\n\
         ma[u32] = 8'd22;\n\
         mz[bg]  = 8'd33;\n\
         $write(\"A \"); for (q=-3; q<=2; q=q+1) $write(\"%0d \", ma[q]); $display(\"\");\n\
         $write(\"Z \"); for (q=0;  q<=5; q=q+1) $write(\"%0d \", mz[q]); $display(\"\");\n\
         $finish;\n\
       end\n\
     endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "no row here is out of range\n{out}");
    assert!(out.contains("A 22 0 0 0 0 0"), "ma[-3] must hold 22\n{out}");
    assert!(out.contains("Z 0 0 33 0 0 11"), "mz[2]/mz[5]\n{out}");
}

/// A SIGNED index must be sign-extended to thirty-two bits, not zero-extended.
///
/// This is the row my first attempt broke: pinning the width with `$signed(x)`
/// alone left the enclosing unsigned context to zero-extend, so `m[s8 >>> 1]`
/// (-3) became 253 and went out of range where it had been correct. The fix is
/// `extend_to(.., signed)`, which fills with the operand's own sign bit.
#[test]
fn a_signed_narrow_index_is_sign_extended_not_zero_extended() {
    let src = "module top;\n\
       reg [7:0] ua [-2:-9];\n\
       reg signed [7:0] s8; integer q;\n\
       initial begin\n\
         for (q=0;q<8;q=q+1) ua[q-9] = 8'd0;\n\
         s8 = -8'sd6;\n\
         ua[s8 >>> 1] = 8'd77;\n\
         $write(\"R \"); for (q=0;q<8;q=q+1) $write(\"%0d \", ua[q-9]); $display(\"\");\n\
         $finish;\n\
       end\n\
     endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "`s8 >>> 1` is -3, which is in range\n{out}");
    assert!(
        out.contains("R 0 0 0 0 0 0 77 0"),
        "expected ua[-3] = 77\n{out}"
    );
    // …and a row the PREVIOUS build got wrong, so this test fails on a revert
    // rather than only on the wrong-extension mutation (measured: the design
    // above alone passes unchanged on the pre-§4.5.310 binary).
    let up = "module top;\n\
       reg [7:0] mp [2:5];\n\
       reg signed [7:0] s8; integer q;\n\
       initial begin\n\
         for (q=2;q<=5;q=q+1) mp[q] = (q+70);\n\
         s8 = -8'sd6;\n\
         $display(\"U %0d\", mp[~s8]);\n\
         $finish;\n\
       end\n\
     endmodule\n";
    let (out2, code2) = run(up);
    assert!(out2.contains("U 75"), "both oracles read mp[5]\n{out2}");
    assert_eq!(code2, Some(0), "in range, so not loud\n{out2}");
}

/// A CONSTANT index keeps its true value and stays LOUD when that value is out
/// of the declared range — it does not join the runtime wrap.
///
/// The two oracles split here: iverilog 13 drops the access with a warning,
/// verilator 5.050 writes the wrapped element. The ladder decides, not the
/// majority: an index a compiler can evaluate, outside a range a compiler can
/// evaluate, is exactly what "correct-or-loud" exists for, and vita was already
/// loud about it. So the wrap is for runtime indices only.
///
/// The last three rows are the ones a SPELLING-based recognizer leaked on. The
/// first version of `index_is_compile_time_constant` named shapes (`Const`, the
/// sign casts, a unary sign over a literal) and so `m[64'h1_0000_0002]` dropped
/// while `m[~64'hFFFF_FFFE_FFFF_FFFD]` — the same value — silently wrote, inside
/// one design. The recognizer walks the expression now, and these rows are why.
#[test]
fn a_constant_index_out_of_range_stays_loud() {
    let src = "module top;\n\
       reg [7:0] mg [-3:2];\n\
       localparam [31:0] R = 32'd1;\n\
       integer q;\n\
       initial begin\n\
         for (q=-3; q<=2; q=q+1) mg[q] = 8'd11;\n\
         mg[32'hFFFF_FFFD]        = 8'hAA;\n\
         mg[$unsigned(-32'sd3)]   = 8'hBB;\n\
         mg[$unsigned(32'd4294967293)] = 8'hCC;\n\
         mg[99]                   = 8'hDD;\n\
         mg[-9]                   = 8'hEE;\n\
         mg[~32'd2]               = 8'h11;\n\
         mg[32'd0-32'd3]          = 8'h22;\n\
         mg[R-32'd4]              = 8'h33;\n\
         $write(\"K \"); for (q=-3; q<=2; q=q+1) $write(\"%0d \", mg[q]); $display(\"\");\n\
         $finish;\n\
       end\n\
     endmodule\n";
    let (out, code) = run(src);
    assert!(
        out.contains("K 11 11 11 11 11 11"),
        "no constant index may land\n{out}"
    );
    assert_eq!(code, Some(1), "every row must be diagnosed\n{out}");
    // …and the same value as a RUNTIME index DOES land, which is what makes the
    // rows above a decision rather than an accident.
    let rt = "module top;\n\
       reg [7:0] mg [-3:2];\n\
       reg [31:0] u32; integer q;\n\
       initial begin\n\
         for (q=-3; q<=2; q=q+1) mg[q] = 8'd11;\n\
         u32 = 32'hFFFF_FFFD;\n\
         mg[u32] = 8'hAA;\n\
         $write(\"K \"); for (q=-3; q<=2; q=q+1) $write(\"%0d \", mg[q]); $display(\"\");\n\
         $finish;\n\
       end\n\
     endmodule\n";
    let (out2, code2) = run(rt);
    assert!(
        out2.contains("K 170 11 11 11 11 11"),
        "runtime lands\n{out2}"
    );
    assert_eq!(code2, Some(0), "and is not diagnosed\n{out2}");
}

/// Truncating a wider-than-32 index must not turn an UNKNOWN index into a known
/// one.
///
/// `select_low` is a bit operation, so on its own it throws away any x/z above
/// bit 31: `reg [63:0] b; b[31:0] = 3;` (high half never driven, so X) read AND
/// WROTE element 3 where iverilog gives `x` and drops — loud → silent-wrong, and
/// none of the value matrices covered it because none of them puts x in an
/// index. The dropped half is added back as `high * 0`, which is 0 when high is
/// known and all-X when it is not (arithmetic returns X for an X operand —
/// measured identical in iverilog 13 and vita).
///
/// Every row's expectation is iverilog's, measured.
#[test]
fn a_wide_index_keeps_the_unknown_bits_it_truncates() {
    let src = "module top;\n\
       reg [7:0] m [0:5];\n\
       reg [63:0] b64; reg [32:0] b33; reg [64:0] b65;\n\
       integer i;\n\
       initial begin\n\
         for (i=0;i<6;i=i+1) m[i] = i+10;\n\
         b64 = 64'd0; b64[31:0] = 32'd3; b64[40] = 1'bz;  $display(\"Z %0d\", m[b64]);\n\
         b64 = 64'd0; b64[3] = 1'bx;                      $display(\"L %0d\", m[b64]);\n\
         b33 = 33'd0; b33[32] = 1'b1; b33[2:0] = 3'd2;    $display(\"H %0d\", m[b33]);\n\
         b33 = 33'd0; b33[32] = 1'bx; b33[2:0] = 3'd2;    $display(\"HX %0d\", m[b33]);\n\
         b65 = 65'd0; b65[64] = 1'b1; b65[2:0] = 3'd4;    $display(\"W %0d\", m[b65]);\n\
         b64 = 64'd0; b64[2:0] = 3'd5;                    $display(\"K %0d\", m[b64]);\n\
         $finish;\n\
       end\n\
     endmodule\n";
    let (out, _code) = run(src);
    for (tag, want) in [
        ("Z", "x"),  // z above bit 31 poisons the index
        ("L", "x"),  // …and so does an x inside the kept half
        ("HX", "x"), // …and a 1-bit high part, which is the narrowest such half
        ("H", "12"), // known high bits are simply dropped: 2^32 + 2 reads m[2]
        ("W", "14"), // 65 bits, same
        ("K", "15"), // control: nothing to truncate
    ] {
        assert!(
            out.lines().any(|l| l == format!("{tag} {want}")),
            "line `{tag} {want}` missing\n{out}"
        );
    }
    // And the WRITE side of the poisoned case, which drops silently rather than
    // reading x — this is the row that made the defect a silent-wrong.
    let w = "module top;\n\
       reg [7:0] m [0:5];\n\
       reg [63:0] b;\n\
       integer i;\n\
       initial begin\n\
         for (i=0;i<6;i=i+1) m[i] = i+10;\n\
         b[31:0] = 32'd3;\n\
         m[b] = 8'd99;\n\
         $write(\"P \"); for (i=0;i<6;i=i+1) $write(\"%0d \", m[i]); $display(\"\");\n\
         $finish;\n\
       end\n\
     endmodule\n";
    let (out2, _c2) = run(w);
    assert!(
        out2.contains("P 10 11 12 13 14 15"),
        "an unknown index must not land\n{out2}"
    );
}
