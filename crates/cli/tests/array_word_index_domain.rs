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
/// index. So the value is multiplied by one BEFORE the bits are cut: arithmetic
/// returns all-X for an X operand, so an unknown anywhere spreads across the
/// whole width first, and a known value is unchanged (measured identical in
/// iverilog 13 and vita at 33, 64 and 65 bits). An earlier shape added the
/// dropped half back as `high * 0`, which has the same semantics but names the
/// index twice — and that gate then refused a pure function call.
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

/// Two things this funnel must NOT do, each with a mutation that survived
/// everything else until the row existed.
///
/// 1. It must not name the index twice. Sign-extending a narrower-than-32 index
///    needs `Concat[Replicate(e[msb]), e]`, so it is gated on `e` being safe to
///    evaluate twice — and "safe" is both channels: a random draw is the value
///    channel, and an out-of-range array read inside the index is the DIAGNOSTIC
///    channel (`warn_run_range` is an error, rate-limited at eight per run, so a
///    doubled report can starve an unrelated site).
/// 2. It must not apply the 32-bit reading to a PACKED element offset. The same
///    funnel serves both geometries; iverilog drops a packed offset that does not
///    fit, and truncating it wrote a neighbouring element at exit 0.
#[test]
fn the_funnel_does_not_duplicate_the_index_or_touch_the_packed_domain() {
    // A draw inside a SIGNED NARROW index — the shape that reaches the
    // extension. Evaluated twice it mixes two draws AND shifts the stream.
    let draw = "module top;\n\
       reg [7:0] m [0:3];\n\
       integer i;\n\
       initial begin\n\
         for (i=0;i<4;i=i+1) m[i] = i+10;\n\
         $display(\"A %0d\", m[byte'($urandom & 8'h03)]);\n\
         $display(\"N %0d\", $urandom);\n\
         $finish;\n\
       end\n\
     endmodule\n";
    let (out, _c) = run(draw);
    assert!(out.contains("A 12"), "index built from ONE draw\n{out}");
    assert!(
        out.contains("N 1055226000"),
        "the stream must not advance twice\n{out}"
    );

    // An out-of-range array read inside the index: one logical site, one report.
    let diag = "module top;\n\
       reg [7:0] m [0:3];\n\
       reg signed [7:0] ix [0:1];\n\
       integer k;\n\
       initial begin\n\
         ix[0]=8'sd1; ix[1]=8'sd1; k=9;\n\
         $display(\"A %0d\", m[ix[k]]);\n\
         $finish;\n\
       end\n\
     endmodule\n";
    let (_o2, code2) = run(diag);
    assert_eq!(code2, Some(1), "the out-of-range read is still loud");

    // The packed element offset keeps its true value and drops, as iverilog does.
    let packed = "module top;\n\
       reg [3:0][7:0] p; reg [63:0] w;\n\
       initial begin\n\
         p = 32'h11223344; w = 64'h1_0000_0002;\n\
         p[w] = 8'hA5;\n\
         $display(\"P %h\", p);\n\
         $finish;\n\
       end\n\
     endmodule\n";
    let (out3, _c3) = run(packed);
    assert!(
        out3.contains("P 11223344"),
        "a packed offset that does not fit must not wrap\n{out3}"
    );
}

/// A subroutine-LOCAL or FORMAL unpacked array is still an unpacked array.
///
/// Its slot is md-packed, so it is registered in `packed_dims` and its word
/// index lowers through the packed read/write path — and taking the domain from
/// where the array LIVES rather than from what it IS skipped the whole 32-bit
/// reading for it. `lm[bg]` inside a function read a different element from the
/// module-level twin `gm[bg]` in the same design, at exit 0 where the previous
/// build had been loud.
///
/// Both oracles agree on all three rows. Without this test the entire fix is
/// invisible: forcing the packed label back at those two call sites passed all
/// 5183 tests (measured), because no other design in the suite indexes a
/// subroutine-local or formal array with a value that needs the reading.
#[test]
fn a_subroutine_local_or_formal_array_is_indexed_like_a_module_one() {
    let src = "module top;\n\
       reg [7:0] gm [0:5]; reg [63:0] bg; integer q;\n\
       function automatic integer f(input reg [63:0] ix);\n\
         reg [7:0] lm [0:5]; integer k;\n\
         begin for (k=0;k<=5;k=k+1) lm[k]=k+50; f = lm[ix]; end\n\
       endfunction\n\
       task automatic t(input reg [63:0] ix, output integer o);\n\
         reg [7:0] tm [0:5]; integer k;\n\
         begin for (k=0;k<=5;k=k+1) tm[k]=k+60; o = tm[ix]; end\n\
       endtask\n\
       integer r;\n\
       initial begin\n\
         for(q=0;q<=5;q=q+1) gm[q]=q+50; bg=64'h1_0000_0002;\n\
         $display(\"G %0d\", gm[bg]);\n\
         $display(\"F %0d\", f(bg));\n\
         t(bg, r); $display(\"T %0d\", r);\n\
         $finish;\n\
       end\n\
     endmodule\n";
    let (out, code) = run(src);
    assert_eq!(
        code,
        Some(0),
        "every row is in range after truncation\n{out}"
    );
    for want in ["G 52", "F 52", "T 62"] {
        assert!(
            out.lines().any(|l| l == want),
            "line `{want}` missing — the module twin and the frame one must agree\n{out}"
        );
    }
}

/// The provisional width path — the one taken once the arena holds a
/// deferred-hierarchy placeholder — must answer what the cached path answers.
///
/// It fills a REUSED scratch buffer at only the queried subtree's ids and leaves
/// every other slot stale, so an incomplete walk silently reads a leftover
/// `{1, unsigned}`. It used to truncate at a 4096-node budget and do exactly
/// that: a wide condition over a signed index took the zero-extend arm, `-3`
/// became 253, and a WRITE landed on a real element at exit 0 — but only when an
/// unrelated hierarchical read appeared earlier in the file.
///
/// The two designs differ in ONE line, which is the whole assertion. Reducing
/// the walk to "the node itself" passed all 5183 tests before this existed.
#[test]
fn the_provisional_width_path_answers_what_the_cached_one_does() {
    // A shared subexpression at every level: 2^depth paths, one node each — the
    // shape the old node budget existed for, and the reason the walk stamps
    // visits instead of counting.
    let mut big = String::from("i8");
    for _ in 0..12 {
        big = format!("({big}+{big})");
    }
    let mk = |hier: bool| {
        let line = if hier {
            "$display(\"HK %0d\", u.kk);"
        } else {
            "$display(\"HK 7\");"
        };
        format!(
            "module sub; reg [7:0] kk = 8'd7; endmodule\n\
             module top;\n\
               reg [7:0] ma [0:255];\n\
               sub u();\n\
               reg [7:0] i8; reg signed [7:0] s8; reg signed [7:0] z8; integer j;\n\
               initial begin\n\
                 for (j=0;j<256;j=j+1) ma[j] = j[7:0];\n\
                 i8 = 8'd3; s8 = -8'sd3; z8 = 8'sd0;\n\
                 {line}\n\
                 $display(\"SIL %0d\", ma[ (({big}) != 32'd0) ? (s8+z8) : (s8+z8) ]);\n\
                 ma[ (({big}) != 32'd0) ? (s8+z8) : (s8+z8) ] = 8'd99;\n\
                 $display(\"W %0d %0d\", ma[253], ma[0]);\n\
                 $finish;\n\
               end\n\
             endmodule\n"
        )
    };
    let (a, ca) = run(&mk(true));
    let (b, cb) = run(&mk(false));
    // `-3` is out of `[0:255]` in both oracles, so the read is x and the write
    // is dropped — and one unrelated `$display` must not change that.
    for (tag, out, code) in [("hier", &a, ca), ("plain", &b, cb)] {
        assert!(out.contains("SIL x"), "{tag}: expected `SIL x`\n{out}");
        assert!(
            out.contains("W 253 0"),
            "{tag}: the write must not land\n{out}"
        );
        assert_eq!(code, Some(1), "{tag}: an out-of-range access stays loud");
    }
}

/// The element's BIT axis is not a word axis, even inside a subroutine.
///
/// A subroutine-local or formal array's extent list is one entry per unpacked
/// dim PLUS a trailing entry for the element's own bit axis, so choosing the
/// domain per NET labelled a bit-select as an array word and truncated it:
/// `lm[0][b]` with a 64-bit `b` wrote bit 2 while the module-level twin
/// `gm[0][b]` in the same design dropped, at exit 0 — matching neither oracle
/// (iverilog drops both, verilator writes both). The axis belongs to the
/// position, not to the net.
#[test]
fn a_frame_array_elements_bit_axis_is_not_a_word_axis() {
    let src = "module top;\n\
       reg [63:0] bg;  int gm [0:3];\n\
       function automatic int fw(input [63:0] b);\n\
         int lm [0:3];\n\
         lm[0]=32'h0; lm[0][b]=1'b1; fw=lm[0];\n\
       endfunction\n\
       initial begin\n\
         bg = 64'h1_0000_0002;\n\
         gm[0]=32'h0; gm[0][bg]=1'b1;\n\
         $display(\"G %0d L %0d\", gm[0], fw(bg));\n\
         $finish;\n\
       end\n\
     endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    // iverilog drops both; the two spellings must at least agree with each other.
    assert!(
        out.contains("G 0 L 0"),
        "the frame spelling must answer like the module one\n{out}"
    );
}

/// The four spellings of one bit-select must answer alike.
///
/// `~v` on a `byte` is 5 at the index's own eight bits. A plain vector and a
/// module array got that; a multi-dim-packed net and a subroutine-local array
/// did not, because the packed branch of the funnel sealed only UNSIGNED
/// indices — so the self-determination half of the seal, which is not an
/// array-word rule at all, was withheld from exactly the two spellings that
/// reach it. Both oracles write bit 5 in all four.
#[test]
fn every_spelling_of_one_bit_select_answers_alike() {
    let src = "module top;\n\
       reg signed [7:0] s8;\n\
       reg [31:0] gv; reg [31:0] gaa [0:1]; reg [1:0][31:0] gp;\n\
       function automatic int Fb(input signed [7:0] v);\n\
         reg [31:0] L [0:1];\n\
         begin L[0]=32'h0; L[0][~v]=1'b1; Fb = L[0]; end\n\
       endfunction\n\
       initial begin\n\
         s8 = -8'sd6;\n\
         gv=32'h0; gv[~s8]=1'b1;\n\
         gaa[0]=32'h0; gaa[0][~s8]=1'b1;\n\
         gp=64'h0; gp[0][~s8]=1'b1;\n\
         $display(\"VEC %0d ARR %0d PKD %0d FRM %0d\", gv, gaa[0], gp[0], Fb(s8));\n\
         $finish;\n\
       end\n\
     endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("VEC 32 ARR 32 PKD 32 FRM 32"),
        "all four spellings write bit 5\n{out}"
    );
}

/// A depth cap on the walks is a cliff, and this is the design that falls off it.
///
/// `index_has_placeholder` runs on every seal in every design — no hierarchical
/// reference needed — and failing closed at depth 64 made `index_self_width`
/// return `None`, so BOTH seal funnels silently reverted to pre-§4.5.308
/// behaviour. Measured exactly at the boundary: 63 pads correct, 64 wrong. The
/// parser caps parentheses at 128, but a left-associative chain is unbounded and
/// the elaborator's own seal nodes count toward the depth, so 31 nested array
/// reads in one index reached it too.
#[test]
fn a_deeply_nested_index_still_gets_its_seal() {
    let mk = |pads: usize| {
        let idx = format!("(~r5){}", " - 5'd0".repeat(pads));
        format!(
            "module top;\n\
               reg [0:31] a0; reg [4:0] r5;\n\
               initial begin\n\
                 a0 = 32'b0; r5 = 5'd28;\n\
                 a0[{idx}] = 1'b1;  $display(\"W %b\", a0);\n\
                 a0 = 32'h1000_0000;\n\
                 $display(\"R %b\", a0[{idx}]);\n\
                 $finish;\n\
               end\n\
             endmodule\n"
        )
    };
    // `~r5` is 3 at the index's own five bits; `[0:31]` is ascending, so bit 3
    // from the left. Both oracles agree, and the answer must not depend on how
    // many `- 0` the user wrote.
    for pads in [0usize, 63, 64, 200] {
        let (out, code) = run(&mk(pads));
        assert_eq!(code, Some(0), "pads={pads}\n{out}");
        assert!(
            out.contains("W 00010000000000000000000000000000"),
            "pads={pads}: the seal was dropped\n{out}"
        );
        assert!(out.contains("R 1"), "pads={pads}\n{out}");
    }
    // …and with a HIERARCHICAL reference first, which routes the query through
    // the provisional path and its own walk. Without this row the cap on that
    // walk is invisible: it was the one that survived when its sibling's was
    // removed, on the strength of a comment citing the removed sibling.
    let hier = |pads: usize| {
        let idx = format!("(~r5){}", " - 5'd0".repeat(pads));
        format!(
            "module sub; reg [7:0] kk = 8'd7; endmodule\n\
             module top;\n\
               reg [0:31] a0; reg [4:0] r5;\n\
               sub u();\n\
               initial begin\n\
                 a0 = 32'b0; r5 = 5'd28;\n\
                 $display(\"HK %0d\", u.kk);\n\
                 a0[{idx}] = 1'b1;  $display(\"W %b\", a0);\n\
                 $finish;\n\
               end\n\
             endmodule\n"
        )
    };
    for pads in [63usize, 64, 200] {
        let (out, code) = run(&hier(pads));
        assert_eq!(code, Some(0), "hier pads={pads}\n{out}");
        assert!(
            out.contains("W 00010000000000000000000000000000"),
            "hier pads={pads}: the seal was dropped on the provisional path\n{out}"
        );
    }
    // A SIGNED index reaches a different walk than the unsigned one above.
    let signed_deep = |pads: usize| {
        format!(
            "module top;\n\
               reg [7:0] ma [0:255]; reg signed [7:0] s8; integer q;\n\
               initial begin\n\
                 for(q=0;q<=255;q=q+1) ma[q]=q[7:0];\n\
                 s8 = 8'sd100;\n\
                 $display(\"R %0d\", ma[(s8 + 8'sd100){}]);\n\
                 $finish;\n\
               end\n\
             endmodule\n",
            " - 8'sd0".repeat(pads)
        )
    };
    for pads in [63usize, 64] {
        let (out, code) = run(&signed_deep(pads));
        assert!(
            out.contains("R x"),
            "signed pads={pads}: 200 is out of range\n{out}"
        );
        assert_eq!(
            code,
            Some(1),
            "signed pads={pads}: and it stays loud\n{out}"
        );
    }
    // A CONSTANT index must stay diagnosed at any depth — `false` from the
    // constant walk means "runtime", which silently wraps instead.
    let const_deep = |pads: usize| {
        format!(
            "module top;\n\
               reg [7:0] mz [0:5]; integer q;\n\
               initial begin\n\
                 for(q=0;q<=5;q=q+1) mz[q]=q+50;\n\
                 mz[64'h1_0000_0002{}] = 8'd99;\n\
                 $write(\"K \"); for(q=0;q<=5;q=q+1) $write(\"%0d \", mz[q]); $display(\"\");\n\
                 $finish;\n\
               end\n\
             endmodule\n",
            " - 64'd0".repeat(pads)
        )
    };
    for pads in [64usize, 65, 200] {
        let (out, code) = run(&const_deep(pads));
        assert!(
            out.contains("K 50 51 52 53 54 55"),
            "const pads={pads}: an out-of-range constant must not land\n{out}"
        );
        assert_eq!(code, Some(1), "const pads={pads}: and it stays loud\n{out}");
    }
}
