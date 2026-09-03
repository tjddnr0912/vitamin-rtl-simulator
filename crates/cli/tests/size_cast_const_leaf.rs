//! ROADMAP §2 row 29: a size cast applies ITS OWN width down into a
//! context-determined operand (§4.5.212) only when `ast_ctx_signed` can resolve
//! the sign of every leaf. It resolved a NET leaf and nothing else, so every
//! operand with a parameter / localparam / genvar / enum-label / package-constant
//! leaf fell to the fill-only path, which computes at the operand's SELF width
//! and only then resizes:
//!
//!   `parameter W32 = 32'hDEADBEEF;  64'(~(W32 + 32'd1))`
//!   vita `000000002152410f` — both oracles `ffffffff2152410f`, exit 0.
//!
//! Measured at HEAD before the fix: 114 of 720 cells wrong over 16 leaf kinds
//! (15 operators × 3 cast widths), 0 of them with a net leaf, 0 oracle splits.
//! The one-token discriminator was `64'({W32} + 32'd1)`: a concat is a leaf the
//! classifier knows, so the braces alone made the cast correct.
//!
//! The classifier now answers a bare constant the way `lower_expr` MATERIALIZES
//! it — the same side maps in the same order (string → wide → real → numeric)
//! and the same sign rule (`param_const_signed` over `lookup_scoped` +
//! `param_meta`) — so it cannot extend a leaf one way while the lowering builds
//! it the other. A `pkg::NAME` leaf mirrors the `PkgScoped` arm the same way.
//!
//! Every expected value below is the agreement of iverilog 13.0 and verilator
//! 5.050 (`vvp` / `--binary --timing`), run on this exact source.
//!
//! OUT OF SCOPE, unchanged: a function-call leaf (`64'(f(1) - 40)`) still
//! answers `None` and stays on the fill-only path — 16 of the 720 cells, all of
//! them. `expr_self_signed`'s `_ => false` has 21 callers and is its own slice.
//! A real parameter leaf keeps answering `None` so the real refusal keeps
//! firing (pinned below); an untyped parameter whose OVERRIDE changes its sign
//! (`parameter P = 5` with `#(.P(8'hF0))`) reads with the default's sign on both
//! the fill-only and the context path — the declared-vs-override provenance
//! gap, PRE == POST, ROADMAP §2.

use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_raw(src: &str) -> (String, String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_sccl_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let so = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.contains("simulation ended"))
        .collect::<Vec<_>>()
        .join("\n");
    let se = String::from_utf8_lossy(&out.stderr).to_string();
    let _ = std::fs::remove_dir_all(&d);
    (so, se, out.status.success())
}

fn run(src: &str) -> String {
    let (so, se, ok) = run_raw(src);
    assert!(ok, "vita failed:\n{se}");
    so
}

/// Lines sorted, for designs whose instances print in an order the oracles do
/// not pin.
fn run_sorted(src: &str) -> String {
    let o = run(src);
    let mut v: Vec<&str> = o.lines().collect();
    v.sort_unstable();
    v.join("\n")
}

/// The headline shape and every leaf kind the fix reaches, on the operators
/// whose answer changes with the context width (`~` `-` `*` `**` `<<` `-` and
/// the sign-carrying `/` `%` `>>>`). Rows 6–10 (`P5 / -2`, `PN % 7`, `PI >>> 1`,
/// `PS / 2`, `LPS >>> 2`) were ALREADY right on the fill-only path and are here
/// so a WRONG `Some` — `always false` or `always true` in the classifier —
/// cannot pass: a signed leaf zero-extended, or an unsigned one sign-extended,
/// changes each of them.
#[test]
fn a_constant_leaf_takes_the_cast_width_as_its_context() {
    let o = run(
        r#"package pk; parameter int PK = -9; parameter logic [7:0] PKU = 8'hF0; parameter logic [95:0] PW = 96'h8000_0000_0000_0000_0000_0001; endpackage
module t;
  parameter W32 = 32'hDEADBEEF;
  parameter P5 = 5;
  parameter PN = -3;
  parameter int PI = -7;
  parameter logic signed [7:0] PS = -8'sd16;
  localparam [15:0] LP = 16'hBEEF;
  localparam signed [15:0] LPS = -16'sd1234;
  parameter Dw = 64;
  parameter logic [95:0] PW96 = 96'h8000_0000_0000_0000_0000_0001;
  typedef enum logic [3:0] {EA = 4'd9} e_t;
  typedef enum logic signed [3:0] {SA = -4'sd3} se_t;
  logic [31:0] n32 = 32'h0000_0001;
  logic signed [7:0] s8 = -8'sd16;
  initial begin
    $display("%h", 64'(~(W32 + 32'd1)));
    $display("%h", 128'(~(W32 + 32'd1)));
    $display("%h", 64'(-W32));
    $display("%h", 64'(W32 * 3));
    $display("%h", 64'(W32 ** 2));
    $display("%h", 64'(W32 >>> 1));
    $display("%h", 64'(~(P5 + 32'd1)));
    $display("%h", 64'(P5 / -2));
    $display("%h", 64'(PN % 7));
    $display("%h", 64'(PI >>> 1));
    $display("%h", 64'(-PS));
    $display("%h", 64'(PS / 2));
    $display("%h", 64'(~LP));
    $display("%h", 64'(LPS >>> 2));
    $display("%h", 8'(-EA));
    $display("%h", 8'(EA ** 2));
    $display("%h", 8'(EA << 1));
    $display("%h", 64'(EA - 40));
    $display("%h", 8'(SA >>> 1));
    $display("%h", 64'(SA / 2));
    $display("%h", 64'(pk::PK / -2));
    $display("%h", 64'(-pk::PKU));
    $display("%h", 128'(~pk::PW));
    $display("%h", 128'(~PW96));
    $display("%h", 128'(PW96 + 1));
    $display("%h", 64'(PW96 >> 40));
    $display("%h", Dw'(~(W32 + 32'd1)));
    $display("%h", Dw'(-W32));
    $display("%h", 64'(~(n32 + W32)));
    $display("%h", 64'(s8 * PS));
    $display("%h", 64'(s8 * W32));
    $display("%h", 64'((n32 + P5) >>> 1));
  end
endmodule
"#,
    );
    // PRE (fill-only path), where it differed: rows 1–2 `000000002152410f` /
    // `…2152410f` zero-filled, row 3 `0000000021524111`, row 4 `000000009c093ccd`
    // (the carry lost), row 5 `00000000216da321`, row 7 `00000000fffffff9`,
    // rows 15–17 `07` / `01` / `02` (computed at the label's 4 bits), row 22
    // `0000000000000010`, rows 23–24 the top word zero, rows 27–28 the `Dw'(`
    // spelling exactly as rows 1 and 3.
    assert_eq!(
        o,
        [
            "ffffffff2152410f",
            "ffffffffffffffffffffffff2152410f",
            "ffffffff21524111",
            "000000029c093ccd",
            "c1b1cd12216da321",
            "000000006f56df77",
            "fffffffffffffff9",
            "fffffffffffffffe",
            "fffffffffffffffd",
            "fffffffffffffffc",
            "0000000000000010",
            "fffffffffffffff8",
            "ffffffffffff4110",
            "fffffffffffffecb",
            "f7",
            "51",
            "12",
            "ffffffffffffffe1",
            "fe",
            "ffffffffffffffff",
            "0000000000000004",
            "ffffffffffffff10",
            "ffffffff7ffffffffffffffffffffffe",
            "ffffffff7ffffffffffffffffffffffe",
            "00000000800000000000000000000002",
            "0080000000000000",
            "ffffffff2152410f",
            "ffffffff21524111",
            "ffffffff2152410f",
            "0000000000000100",
            "000000d0c2e30010",
            "0000000000000003",
        ]
        .join("\n")
    );
}

/// The same leaf through the two binding channels that are not a module-scope
/// declaration: a parameter OVERRIDE (`#(.P(-3))` beside the default, on a TYPED
/// parameter) and a generate-scope genvar / localparam. `param_meta` for a genvar
/// is pinned `(32, signed)` by the generate lowering; the classifier reads that
/// entry, not a guess. PRE: `00000000fffffffe` / `07` / `01` for the genvar rows.
///
/// The parameter is `int` on purpose. An UNTYPED `parameter P = 5` overridden
/// with `-3` binds with the DEFAULT literal's type (ROADMAP §2 row 25), which is
/// a guess the classifier declines (`param_type_guessed`) — such a cast stays on
/// the pre-slice path, right or wrong as it was, and is not pinned here.
#[test]
fn an_override_and_a_generate_scope_constant_resolve_the_same_way() {
    let o = run_sorted(
        r#"module child #(parameter int P = 5) ();
  initial begin
    $display("%0d %h %h %h", P, 64'(~(P + 32'd1)), 64'(-P), 8'(P ** 2));
  end
endmodule
module t;
  child #(.P(-3)) c1();
  child c3();
  genvar g;
  generate for (g = 0; g < 2; g++) begin : gb
    localparam [3:0] GL = 4'd9 + g;
    localparam signed [3:0] GS = -4'sd3 - g;
    initial $display("g%0d %h %h %h %h %h", g, 64'(~(g + 32'd1)), 8'(-GL), 8'(GL ** 2), 8'(GS >>> 1), 64'(GS / 2));
  end endgenerate
endmodule
"#,
    );
    assert_eq!(
        o,
        [
            "-3 ffffffff00000001 0000000000000003 09",
            "5 fffffffffffffff9 fffffffffffffffb 19",
            "g0 fffffffffffffffe f7 51 fe ffffffffffffffff",
            "g1 fffffffffffffffd f6 64 fe fffffffffffffffe",
        ]
        .join("\n")
    );
}

/// A real parameter leaf must keep answering `None`: on the context path the
/// real refusal fires per operand, on the fill-only path `cast_operand_is_real`
/// fires — either way the cast is LOUD, and iverilog refuses the same source
/// (`Cast base expression must be a vector type`). Wording pin on the code only.
#[test]
fn a_real_parameter_leaf_stays_loud() {
    let (so, se, ok) = run_raw(
        "module t; parameter real RI = 4.0; initial $display(\"%h\", 64'(~(RI + 32'd1))); endmodule\n",
    );
    assert!(!ok, "expected a refusal, got stdout:\n{so}");
    assert!(se.contains("VITA-E3009"), "stderr:\n{se}");
}

/// The discriminator from the row: braces around the leaf made the cast correct
/// before, because a concat is a leaf the classifier could always resolve. Both
/// spellings must now agree with each other and with the oracles.
#[test]
fn the_brace_discriminator_no_longer_discriminates() {
    let o = run("module t; parameter W32 = 32'hDEADBEEF; initial begin\n\
         $display(\"%h\", 64'(~({W32} + 32'd1)));\n\
         $display(\"%h\", 64'(~(W32 + 32'd1)));\n\
         end endmodule\n");
    assert_eq!(o, "ffffffff2152410f\nffffffff2152410f");
}

/// The half of the fix that §4.5.318 built and reverted on: routing a constant
/// leaf into the context path REGRESSED `8'(13 + (PS8 >> 2))` (`49`, fill-only
/// PRE `09` = both oracles) because `lower_size_ctx` recursed at the cast width
/// N, while §11.8.1 evaluates the operand at `max(N, its self width)` — here 32,
/// the literal's. The NET spelling `8'(13 + (s8 >> 2))` had been wrong that way
/// all along (PRE `49`), so the width model, not the classifier, was the defect.
/// `lower_size_ctx_entry` now evaluates at that maximum. Rows 1–2 are the net
/// leaf (PRE `49` / `3ffb`), rows 4–7 the constant leaves that must not regress,
/// the rest the shapes §4.5.316/318 pinned around the narrowing branch, which a
/// wider evaluation width must not disturb (`4'(u4 + (s8 >>> 9))` = `d`,
/// `2'(PS8 % 4)` = `0`, `8'(s8 >> 2)` = `3c` — self width 8 = N, so no change).
#[test]
fn the_operand_is_evaluated_at_max_of_cast_width_and_its_own_width() {
    let o = run(r#"module t;
  parameter logic signed [7:0] PS8 = -8'sd16;
  localparam signed [15:0] LPS = -16'sd1234;
  parameter W32 = 32'hDEADBEEF;
  logic signed [7:0] s8 = -8'sd16;
  logic [3:0] u4 = 4'hD;
  initial begin
    $display("%h", 8'(13 + (s8 >> 2)));
    $display("%h", 16'((s8 >> 2) - 1));
    $display("%h", 4'(13 + (s8 >> 2)));
    $display("%h", 8'(13 + (PS8 >> 2)));
    $display("%h", 16'((PS8 >> 2) - 1));
    $display("%h", 16'(13 + (LPS >> 2)));
    $display("%h", 16'((LPS >> 2) - 1));
    $display("%h", 8'(1 + (PS8 / 3)));
    $display("%h", 8'(W32 >> 4));
    $display("%h", 8'((W32 >> 4) + 8'd1));
    $display("%h", 4'(u4 + (s8 >>> 9)));
    $display("%h", 2'(PS8 % 4));
    $display("%h", 8'(s8 >> 2));
    $display("%h", 8'(PS8 >> 2));
    $display("%h", 8'({PS8} >> 2));
  end
endmodule
"#);
    assert_eq!(
        o,
        [
            "09", "fffb", "9", "09", "fffb", "fed8", "feca", "fc", "ee", "ef", "d", "0", "3c",
            "3c", "3c",
        ]
        .join("\n")
    );
}

/// The evaluation width has to be EXACT — an over-estimate changes what a
/// logical shift brings in as surely as an under-estimate does — so the AST
/// self-width walk is checked against the lowering itself. With
/// `VITA_SCW_CHECK=<file>` set, every size cast that enters the context path
/// also lowers its operand plain, reads `ir_bits_of`, and appends `SCW-OK`,
/// `SCW-NONE` (the walk declined; the probe answered) or `SCW-MISMATCH`.
/// Over the three matrices this slice measured (720 + 53 + 720 cells) and the
/// whole workspace suite the count of mismatches was 0; this keeps it there,
/// and the `SCW-OK` assertion keeps the hook itself alive.
#[test]
fn the_ast_self_width_never_disagrees_with_the_lowering() {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_sccl_scw_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(
        &f,
        r#"package pk; parameter int PK = -9; parameter logic [7:0] PKU = 8'hF0; parameter logic [95:0] PW = 96'h8000_0000_0000_0000_0000_0001; endpackage
module t;
  parameter W32 = 32'hDEADBEEF;
  parameter P5 = 5;
  parameter PN = -3;
  parameter int PI = -7;
  parameter logic signed [7:0] PS = -8'sd16;
  parameter logic [95:0] PW96 = 96'h8000_0000_0000_0000_0000_0001;
  typedef enum logic [3:0] {EA = 4'd9} e_t;
  logic [31:0] n32 = 32'h0000_0001;
  logic signed [7:0] s8 = -8'sd16;
  logic [7:0] am [0:3];
  logic [15:0] v16 = 16'hBEEF;
  logic [3:0] u4x = 4'hD;
  bit c = 1;
  genvar g;
  generate for (g = 0; g < 2; g++) begin : gb
    localparam [3:0] GL = 4'd9 + g;
    initial $display("%h %h", 8'(-GL), 64'(~(g + 32'd1)));
  end endgenerate
  initial begin
    am[0] = 8'h7F;
    $display("%h", 64'(~(W32 + 32'd1)));
    $display("%h", 8'(13 + (PS >> 2)));
    $display("%h", 64'(P5 / -2));
    $display("%h", 64'(PN % 7));
    $display("%h", 64'(PI >>> 1));
    $display("%h", 128'(~PW96));
    $display("%h", 128'(~pk::PW));
    $display("%h", 64'(pk::PK * pk::PKU));
    $display("%h", 8'(-EA));
    $display("%h", 64'(~(n32 + W32)));
    $display("%h", 4'(u4x + (s8 >>> 9)));
    $display("%h", 16'(am[0] * 3));
    $display("%h", 8'(v16[11:4] + 1));
    $display("%h", 8'(v16[4 +: 8] - 1));
    $display("%h", 8'({v16[3:0], v16[7:4]} + 1));
    $display("%h", 8'({2{v16[3:0]}} - 1));
    $display("%h", 8'(c ? s8 : 32'd7));
    $display("%h", 8'(n32[3] + s8));
    $display("%h", 16'('1 - s8));
  end
endmodule
"#,
    )
    .unwrap();
    let log = d.join("scw.log");
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .env("VITA_SCW_CHECK", &log)
        .current_dir(&d)
        .output()
        .expect("run vita");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = std::fs::read_to_string(&log).expect("the check hook wrote its log");
    let _ = std::fs::remove_dir_all(&d);
    let mismatches: Vec<&str> = text
        .lines()
        .filter(|l| l.starts_with("SCW-MISMATCH"))
        .collect();
    assert!(mismatches.is_empty(), "{mismatches:?}");
    assert!(
        text.lines().filter(|l| l == &"SCW-OK").count() >= 20,
        "the hook must have checked every routed cast:\n{text}"
    );
}

/// Round-2 review: an inline formal whose actual `bind_formal_actual` handed over
/// VERBATIM (a frame-function call — not repeatable, not width-trusted) has a
/// sign/width MIRROR the engine does not honour (`expr_self_signed` says a call
/// is unsigned; the callee's return sign lives in a sidecar), so the classifier
/// must answer `None` for it, as it did before this slice. Reading the mirror
/// zero-extended `16'(x + 0)` to `00f0` where PRE and both oracles say `fff0`.
/// `fs_shr` is the pre-existing `000f` (both PRE and now) and is not pinned.
#[test]
fn a_verbatim_inline_actual_is_not_classified_from_its_mirror() {
    let o = run(r#"module t;
  function automatic logic signed [7:0] g(input logic signed [7:0] a); return a; endfunction
  function [15:0] fs_add(input logic signed [7:0] x); fs_add = 16'(x + 0); endfunction
  function [15:0] fs_mul(input logic signed [7:0] x); fs_mul = 16'(x * 2); endfunction
  function [15:0] fs_div(input logic signed [7:0] x); fs_div = 16'(x / 2); endfunction
  function [63:0] fs_neg(input logic signed [7:0] x); fs_neg = 64'(-x); endfunction
  initial begin
    #1;
    $display("%h %h %h %h", fs_add(g(-8'sd16)), fs_mul(g(-8'sd16)), fs_div(g(-8'sd16)), fs_neg(g(-8'sd16)));
  end
endmodule
"#);
    assert_eq!(o, "fff0 ffe0 fff8 0000000000000010");
}

/// A chain of `[i]` selects: a BIT (more selects than unpacked dimensions —
/// `a1[0][3]`, `g2[0][1][7]`, `u1[0][4]`) is 1-bit unsigned whatever its base,
/// and the fill-only path must NOT get it (it evaluates the 1-bit leaf at 1 bit:
/// `8'(~a1[0][3])` `ff` → `01` when round 4 first declined it); an ELEMENT (as
/// many selects as dimensions) carries the element's sign — of a 1-D or 2-D
/// static array, and of a frame-local array (an md-packed slot whose element
/// sign lives in `frame_arr_formal_meta`: `64'(PS16 / la[0])` had been `10b`
/// for the oracles' `4d`); an element of a dynamic / queue handle declines.
/// Every value = iverilog 13.0 = verilator 5.050.
#[test]
fn a_select_chain_is_a_bit_or_an_element_by_its_depth() {
    let o = run(r#"module t;
  parameter PS16 = -16'sd1234;
  logic signed [7:0] a1 [0:3];
  logic signed [7:0] g2 [0:1][0:1];
  logic [7:0] u1 [0:1];
  int dq[$];
  function automatic logic [63:0] fdiv(input logic signed [7:0] x);
    logic signed [7:0] la [0:1];
    la[0] = x; la[1] = 8'sd1;
    return 64'(PS16 / la[0]);
  endfunction
  function automatic logic [63:0] fmod(input logic signed [7:0] x);
    logic signed [7:0] la [0:1];
    la[0] = 8'sd1; la[1] = x;
    return 64'(PS16 % la[1]);
  endfunction
  initial begin
    a1[0] = -8'sd16; g2[0][1] = -8'sd16; u1[0] = 8'hF0; dq.push_back(-5);
    $display("%h %h %h", 8'(-a1[0][3]), 8'(~a1[0][3]), 16'((a1[0][3] - 2) >> 1));
    $display("%h %h", 64'(PS16 / g2[0][1]), 64'(PS16 / a1[0]));
    $display("%h %h", 8'(-g2[0][1][7]), 16'(u1[0][4] - 2));
    $display("%h %h", fdiv(-8'sd16), fmod(-8'sd16));
    $display("%h", 64'(-dq[0]));
  end
endmodule
"#);
    assert_eq!(
        o,
        [
            "00 ff ffff",
            "000000000000004d 000000000000004d",
            "ff ffff",
            "000000000000004d fffffffffffffffe",
            "0000000000000005",
        ]
        .join("\n")
    );
}

/// A header-list SIBLING derived from an overridden untyped parameter inherits
/// the guess (`parameter Q = P + 1` — its value was folded from the wrongly
/// typed `P`, §2 row 25) and declines to the pre-slice path, where this cell is
/// right by accident; a TYPED sibling (`parameter int QI = P + 1`) carries a
/// declared type, is not guessed, and is fixed. The un-overridden instance is
/// fixed on all three. Lines sorted.
#[test]
fn a_header_sibling_derived_from_a_guessed_parameter_declines() {
    let o = run_sorted(
        r#"module c #(parameter P = -5, parameter Q = P + 1, parameter int QI = P + 1) ();
  initial $display("%h %h %h", 64'(Q >> 1), 64'(QI >> 1), 64'(P >> 1));
endmodule
module t;
  c #(.P(32'hF0F0F0F0)) a();
  c b();
  initial #1 $finish;
endmodule
"#,
    );
    // PRE: `0000000078787878 0000000078787878 0000000078787878` /
    //      `000000007ffffffe 000000007ffffffe 000000007ffffffd`
    assert_eq!(
        o,
        "0000000078787878 7ffffffff8787878 0000000078787878\n7ffffffffffffffe 7ffffffffffffffe 7ffffffffffffffd"
    );
}

/// Round-5 review: a frame (automatic function) array is an md-packed slot whose
/// UNPACKED dimension count is `ArrayFormal::dims.len()`, not 1 — a 2-D frame
/// local's element (`m[0][0]`) was read as a BIT of a 1-D element and
/// `64'(PS16 / m[0][0])` became `10b` (both leaves zero-extended) for the
/// oracles' `4d`; `16'(PS16 % m[0][0])` on `int m[2][2]` became `fb2e` for
/// `fffe`. The chain rule now counts the frame array's real dimensions, the same
/// predicate `lower_packed_read` gates its `$signed` re-stamp on. Third value:
/// a bit OF a 2-D frame element stays a 1-bit unsigned leaf.
#[test]
fn a_multi_dimensional_frame_array_element_carries_its_sign() {
    let o = run(r#"module t;
  parameter PS16 = -16'sd1234;
  function automatic logic [63:0] f2d(input logic signed [7:0] x);
    logic signed [7:0] m [0:1][0:1];
    m[0][0] = x; m[1][1] = 8'sd3;
    return 64'(PS16 / m[0][0]);
  endfunction
  function automatic logic [15:0] f2m(input int x);
    int m [2][2];
    m[0][0] = x;
    return 16'(PS16 % m[0][0]);
  endfunction
  function automatic logic [7:0] f2b(input logic signed [7:0] x);
    logic signed [7:0] m [0:1][0:1];
    m[0][0] = x;
    return 8'(-m[0][0][7]);
  endfunction
  initial $display("%h %h %h", f2d(-8'sd16), f2m(-16), f2b(-8'sd16));
endmodule
"#);
    assert_eq!(o, "000000000000004d fffe ff");
}

/// Round-6 review: the select chain must resolve its BASE with the resolver the
/// rest of the file (and the lowering) uses. It read `lookup_net_scoped` and the
/// i64 constant domain directly, so a name carried by neither — a parameter
/// wider than i64 (`wide_param_bits`), its package twin (`pkg_wide_bits`), an
/// inline-substituted formal — made a BIT of it decline, and the fill-only path
/// then evaluated that 1-bit leaf at 1 bit (108 cells where PRE routed and was
/// right). Values here are iverilog 13.0 = verilator 5.050 = PRE.
#[test]
fn a_bit_of_a_wide_or_substituted_name_is_still_a_bit() {
    let o = run(
        r#"package pk; parameter logic [95:0] WP = 96'hDEAD_BEEF_1234_5678_9ABC_DEF0; endpackage
module t;
  parameter logic [95:0] W96 = 96'h8000_0000_0000_0000_0000_0001;
  logic signed [7:0] s8 = -8'sd16;
  function automatic logic [15:0] fb(input logic [7:0] x); return 16'(x[7] + x[0]); endfunction
  initial begin
    $display("%h %h %h", 2'(W96[95] + W96[0]), 4'(W96[95] + W96[0]), 8'(W96[95] - W96[94]));
    $display("%h %h", 2'(pk::WP[95] + pk::WP[94]), 8'(pk::WP[95] - pk::WP[94]));
    $display("%h", fb(8'hF0));
    $display("%h", 8'(s8[7] - s8[0]));
  end
endmodule
"#,
    );
    assert_eq!(
        o,
        "2 2 01
2 00
0001
01"
    );
}

/// Round-6 review: the guard for that placeholder has to see it wherever it
/// hides. The `Concat`/`Replicate` sign arms answer `Some(false)` WITHOUT
/// descending, so an opaque leaf in braces walked past a syntactic guard three
/// ways: the callee's own NAME is a path (`u.hf(x)` — the args were walked, the
/// name was not), an inline formal is a bare name bound to a hierarchical actual
/// (no hierarchical spelling in the operand at all — `verbatim_actuals` knows
/// it), and the WIDTH walk sized a select from its own `[msb+:w]` without
/// looking at the base (`4'(s8 - u.v[0+:4])` measured 8 and evaluated wider than
/// the pre-slice `n` over a placeholder). All three printed `x` where PRE and
/// both oracles agree. One `has_opaque_leaf` gate now decides BOTH walks.
#[test]
fn an_opaque_leaf_is_seen_wherever_it_hides() {
    let o = run(r#"module u_m;
  logic signed [7:0] v = -8'sd16;
  function automatic logic signed [7:0] hf(input logic signed [7:0] x); hf = x; endfunction
endmodule
module t;
  parameter logic signed [15:0] PS16 = -16'sd1234;
  logic signed [7:0] s8 = -8'sd100;
  u_m u();
  function logic [15:0] c1(input logic signed [7:0] x); c1 = 16'(PS16 * {x, 1'b0}); endfunction
  initial begin
    #1;
    $display("%h %h", 16'(PS16 * {u.hf(-8'sd16), 1'b0}), 16'(PS16 / {u.hf(-8'sd16), 1'b0}));
    $display("%h", c1(u.v));
    $display("%h %h", 4'(s8 - u.v[0+:4]), 2'(u.v[0+:4] - PS16));
  end
endmodule
"#);
    assert_eq!(
        o,
        "f640 0085
f640
c 2"
    );
}

/// A hierarchical (or class-member) read inside a cast operand is a PLACEHOLDER
/// at classification time — resolved only after the instance tree exists, with
/// no width to resize by — so a cast over one is right on neither path in
/// general. Round-5 review measured both moves: resolving the constant sibling
/// routed `16'(PS16 * u.a1[2])` over the widthless leaf (`xxxx` for the oracles'
/// `4d20`), and declining the hierarchical leaf sent `8'(-u.v[3])` to the
/// fill-only path (`01` for `ff`). Such an operand therefore keeps the
/// PRE-SLICE decision exactly (nets answer, constants and formals decline, `&&`
/// short-circuits); these cells are PRE = both oracles and must stay so.
#[test]
fn a_hierarchical_read_in_the_operand_keeps_the_pre_slice_route() {
    let o = run(r#"module u_m;
  logic signed [7:0] a1 [0:3];
  logic signed [7:0] s8 = -8'sd16;
  initial begin a1[0] = -8'sd1; a1[1] = 8'sd18; a1[2] = -8'sd16; a1[3] = 8'sd3; end
endmodule
module t;
  parameter logic signed [15:0] PS16 = -16'sd1234;
  parameter int PI = 5;
  u_m u();
  initial begin
    #1;
    $display("A %h %h %h", 16'(PS16 * u.a1[2]), 4'(PS16 * u.s8[3]), 4'(PS16 * u.a1[0][3]));
    $display("B %h %h %h", 16'(PI * u.a1[2]), 32'(PI * u.a1[2]), 8'(PI / u.a1[2]));
    $display("C %h %h", 8'((PS16 + u.a1[2]) >> 4), 16'((PS16 + u.a1[2]) >> 4));
  end
endmodule
"#);
    assert_eq!(o, "A 4d20 0 e\nB ffb0 ffffffb0 00\nC b1 0fb1");
}

/// A `time` parameter has no declared-width arm in the meta producer, so a
/// `time` constant derived from a guessed parameter is a guess too and declines
/// (round-5 review: `localparam time LT = P` on `#(.P(32'hF0F0F0F0))` routed
/// with a 32-bit signed meta and printed `7ffffffff8787878` for the oracles'
/// `0000000078787878`, which the pre-slice path gives). Only this cell is
/// pinned: the VALUE of `LT` itself is bound wrongly in every build (§2 row 25),
/// so its other casts are pre-existing wrong and not the oracles' numbers.
#[test]
fn a_time_constant_derived_from_a_guessed_parameter_declines() {
    let o = run(r#"module c #(parameter P = 5) ();
  localparam time LT = P;
  initial $display("%h", 64'(LT >> 1));
endmodule
module t;
  c #(.P(32'hF0F0F0F0)) a();
  initial #1 $finish;
endmodule
"#);
    assert_eq!(o, "0000000078787878");
}
