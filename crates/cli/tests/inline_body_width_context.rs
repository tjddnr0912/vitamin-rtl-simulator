//! An INLINE (static, non-`automatic`) function body's return / local assignment
//! IS a §11.6.1 width context, exactly as the frame twin's is.
//!
//! The frame route gets it from the engine: the body writes a real NET, and an
//! assignment's rhs is evaluated at `max(lvalue_w, self_w)`. The inline fold has
//! no net — the body becomes one substituted ExprId — so every context-determined
//! region folded at `max(operand self-widths)` and `resize_inline_assign` then
//! resized a value whose carry/product bits were already gone:
//! `function [31:0] fh(input [7:0] x); fh = fld * x;` with `8'hFF` printed
//! `00000001` where the `automatic` twin beside it printed `0000fe01`.
//!
//! The defect was NOT about the formal (the ROADMAP entry blamed one): a body
//! with no formal at all, a body that never reads its formal, and a body local
//! all reproduce it — see `the_context_is_the_target_width_not_the_formal`.
//!
//! ORACLES: iverilog 13.0 (-g2012) and verilator 5.052 (--binary --timing). Every
//! value asserted below was measured in BOTH unless the assert says otherwise
//! (verilator is 2-state, so it is not an oracle for the x/z row). The PRE value
//! in each comment was measured on a release binary built at the parent commit.
//!
//! The §11.8.1 half is measured too: ONE unsigned leaf makes the whole region
//! unsigned and then EVERY leaf zero-extends, signed ones included — the first
//! draft sign-extended each leaf by its own sign and printed `fffff709` for both
//! oracles' `0000f609` (`a_mixed_sign_region_zero_extends_its_signed_leaf`).

use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_args(src: &str, args: &[&str]) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_ibwc_{}_{n}", std::process::id()));
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
    let _ = std::fs::remove_dir_all(&d);
    so
}

fn run(src: &str) -> String {
    run_args(src, &[])
}

/// The same source on all three backends; diverging is the failure.
fn agrees_across_backends(src: &str) -> String {
    let mut first: Option<String> = None;
    for be in ["native", "vm", "interp"] {
        let s = run_args(src, &["--backend", be]);
        match &first {
            None => first = Some(s),
            Some(f) => assert_eq!(f, &s, "backend {be} diverged"),
        }
    }
    first.unwrap()
}

/// ① THE HEADLINE, with its frame twin beside it. The static and `automatic`
/// spellings of one body must print the same thing; before this slice they did
/// not, and the `automatic` one was the oracles' answer.
#[test]
fn an_inline_body_folds_at_the_declared_return_width() {
    let o = agrees_across_backends(
        r#"module t;
  logic [7:0] fld;
  function           [31:0] fs(input [7:0] x); fs = fld * x; endfunction
  function automatic [31:0] fa(input [7:0] x); fa = fld * x; endfunction
  initial begin
    fld = 8'hFF;
    $display("%h %h", fs(8'hFF), fa(8'hFF));
    #1 $finish;
  end
endmodule
"#,
    );
    // PRE printed `00000001 0000fe01` — the static half folded the product at 8.
    assert_eq!(o, "0000fe01 0000fe01");
}

/// ② EVERY context-determined operator of Table 11-21, at one return width.
/// `-`/`&` are the controls whose result already fits 8 bits: a fix that moved
/// them would be widening something else than the context.
#[test]
fn every_context_determined_operator_takes_the_return_width() {
    let o = run(r#"module t;
  logic [7:0] a8, b8;
  function [31:0] f_mul;  f_mul  = a8 * b8;        endfunction
  function [31:0] f_add;  f_add  = a8 + b8;        endfunction
  function [31:0] f_sub;  f_sub  = a8 - b8;        endfunction
  function [31:0] f_shl;  f_shl  = a8 << 4;        endfunction
  function [31:0] f_pow;  f_pow  = a8 ** 2;        endfunction
  function [31:0] f_neg;  f_neg  = -a8;            endfunction
  function [31:0] f_not;  f_not  = ~a8;            endfunction
  function [31:0] f_and;  f_and  = a8 & b8;        endfunction
  function [31:0] f_tern; f_tern = 1 ? a8 + b8 : 0; endfunction
  function [31:0] f_par;  f_par  = (a8 + b8);      endfunction
  function [31:0] f_nest; f_nest = (a8 + b8) * 1;  endfunction
  initial begin
    a8 = 8'hFF; b8 = 8'hFF;
    $display("%h %h %h %h %h", f_mul(), f_add(), f_sub(), f_shl(), f_pow());
    $display("%h %h %h %h %h %h", f_neg(), f_not(), f_and(), f_tern(), f_par(), f_nest());
    #1 $finish;
  end
endmodule
"#);
    // PRE: `00000001 000000fe 00000000 000000f0 00000001` /
    //      `00000001 00000000 000000ff 000001fe 000000fe 000001fe`.
    // The two ternary/`*1` rows were already right — a 32-bit SIBLING had widened
    // the region — and they must not double-widen.
    assert_eq!(
        o,
        "0000fe01 000001fe 00000000 00000ff0 0000fe01\n\
         ffffff01 ffffff00 000000ff 000001fe 000001fe 000001fe"
    );
}

/// ③ The context is the TARGET's declared width, whatever it is — including one
/// NARROWER than 32 and one that is not a byte multiple.
#[test]
fn the_context_is_the_declared_width_of_the_target() {
    let o = run(r#"module t;
  logic [7:0] a8, b8;
  function [15:0] r16; r16 = a8 + b8;  endfunction
  function [63:0] r64; r64 = a8 * b8;  endfunction
  function [8:0]  r9;  r9  = a8 + b8;  endfunction
  initial begin
    a8 = 8'hFF; b8 = 8'hFF;
    $display("%h %h %h", r16(), r64(), r9());
    #1 $finish;
  end
endmodule
"#);
    // PRE printed `00fe 0000000000000001 0fe`.
    assert_eq!(o, "01fe 000000000000fe01 1fe");
}

/// ④ The ROADMAP entry blamed the FORMAL bind ("does not push the declared width
/// into the body"). Measured, the formal is not involved: a body with no formal,
/// a body that ignores its formal, and a BODY LOCAL (whose own declared width is
/// the context for its own assignment) all reproduced it.
#[test]
fn the_context_is_the_target_width_not_the_formal() {
    let o = run(r#"module t;
  logic [7:0] fld, g8;
  function [31:0] nofml;                 nofml = fld * g8;    endfunction
  function [31:0] one(input [7:0] x);    one   = fld * 8'hFF; endfunction
  function [31:0] mul(input [7:0] x);    mul   = fld * x;     endfunction
  function [31:0] loc(input [7:0] x);
    begin logic [7:0] t; t = x; loc = fld * t; end
  endfunction
  initial begin
    fld = 8'hFF; g8 = 8'hFF;
    $display("%h %h %h %h", nofml(), one(8'hFF), mul(8'hFF), loc(8'hFF));
    #1 $finish;
  end
endmodule
"#);
    // PRE printed `00000001` four times.
    assert_eq!(o, "0000fe01 0000fe01 0000fe01 0000fe01");
}

/// ⑤ §11.8.1: one UNSIGNED leaf makes the whole region unsigned, and then a
/// SIGNED leaf ZERO-extends. `f_mix` is the cell that refuted per-leaf sign
/// extension; `f_sgn`/`f_div`/`f_mod`/`f_asr` are the all-signed region beside it,
/// where the extension is by the sign and the operator stays signed.
#[test]
fn a_mixed_sign_region_zero_extends_its_signed_leaf() {
    let o = run(r#"module t;
  logic [7:0] b8;
  logic signed [7:0] s8, t8;
  function        [31:0] f_mix; f_mix = s8 * b8;  endfunction
  function signed [31:0] f_div; f_div = s8 / t8;  endfunction
  function signed [31:0] f_mod; f_mod = s8 % t8;  endfunction
  function signed [31:0] f_asr; f_asr = s8 >>> 1; endfunction
  function        [31:0] f_sgn; f_sgn = s8 * s8;  endfunction
  initial begin
    b8 = 8'hFF; s8 = -8'sd9; t8 = 8'sd2;
    $display("%h %h %h %h %h", f_mix(), f_div(), f_mod(), f_asr(), f_sgn());
    #1 $finish;
  end
endmodule
"#);
    // PRE printed `00000009 fffffffc ffffffff fffffffb 00000051`: the first was
    // an 8-bit product, and the signed rows were right by cancellation.
    // The first draft of this slice sign-extended each leaf by its OWN sign and
    // printed `fffff709` for `f_mix` — a different silent-wrong, not a fix.
    assert_eq!(o, "0000f609 fffffffc ffffffff fffffffb 00000051");
}

/// ⑥ Table 11-21's SELF-determined positions must not take the context: a shift
/// amount, a `**` exponent, a reduction's operand, a concatenation's operands,
/// and a size cast (which is its own context, §4.5.403).
#[test]
fn a_self_determined_position_does_not_take_the_body_context() {
    let o = run(r#"module t;
  logic [7:0] a8, b8;
  logic signed [7:0] s8, t8;
  function [31:0] f_shamt; f_shamt = 8'h01 << (a8 & 8'h04); endfunction
  function [31:0] f_exp;   f_exp   = 8'h02 ** (a8 & 8'h03); endfunction
  function [31:0] f_cast;  f_cast  = 8'(a8 * b8);           endfunction
  function [31:0] f_red;   f_red   = ^a8;                   endfunction
  function [31:0] f_cat;   f_cat   = {a8, b8} + 1;          endfunction
  function [31:0] f_rel;   f_rel   = (s8 < t8) + 8'h10;     endfunction
  initial begin
    a8 = 8'hFF; b8 = 8'hFF; s8 = -8'sd9; t8 = 8'sd2;
    $display("%h %h %h %h %h %h", f_shamt(), f_exp(), f_cast(), f_red(), f_cat(), f_rel());
    #1 $finish;
  end
endmodule
"#);
    // Every one of these already matched both oracles before the slice; they are
    // here because widening the wrong operand moves exactly them.
    assert_eq!(o, "00000010 00000008 00000001 00000000 00010000 00000011");
}

/// ⑦ The widening is 4-STATE preserving: it is a `Concat` of a fill with the
/// operand, not an arithmetic zero-extension. iverilog prints `xxxxxxxx` (the
/// x reaches the whole product); verilator is 2-state and is not an oracle here.
#[test]
fn the_widening_preserves_x() {
    let o = run(r#"module t;
  logic [7:0] xz, b8;
  function [31:0] f_xz; f_xz = xz * b8; endfunction
  initial begin
    xz = 8'hxF; b8 = 8'hFF;
    $display("%h", f_xz());
    #1 $finish;
  end
endmodule
"#);
    // PRE printed `000000xx` — the product was folded at 8 and then zero-padded.
    assert_eq!(o, "xxxxxxxx");
}

/// ⑧ An UNSIZED FILL keeps the sizing it already had: the context walk this
/// slice opts into is the same one a fill was already taking, so a fill-bearing
/// rhs must be byte-identical.
#[test]
fn an_unsized_fill_keeps_its_pre_slice_sizing() {
    let o = run(r#"module t;
  logic [7:0] a8;
  function [31:0] f_fill;    f_fill    = '1;      endfunction
  function [31:0] f_fillmul; f_fillmul = '1 * a8; endfunction
  initial begin
    a8 = 8'hFF;
    $display("%h %h", f_fill(), f_fillmul());
    #1 $finish;
  end
endmodule
"#);
    assert_eq!(o, "ffffffff ffffff01");
}

/// ⑨ A `real` or `string` rhs has no bit width to widen, and must not go loud.
#[test]
fn a_real_or_string_body_is_untouched() {
    let o = run(r#"module t;
  real r; string st;
  function real   f_real; f_real = r + 0.5;      endfunction
  function [31:0] f_rmix; f_rmix = r * 2.0;      endfunction
  function string f_str;  f_str  = {st, "x"};    endfunction
  initial begin
    r = 2.25; st = "ab";
    $display("%0.2f %0.2f %s", f_real(), f_rmix(), f_str());
    #1 $finish;
  end
endmodule
"#);
    // ⚠️ `f_rmix` = 4.50 is NOT the oracles' answer (both print 5.00: they apply
    // the declared `[31:0]`, so 4.5 rounds to 5). A real rhs skips the §10.7 seal
    // — a PRE-EXISTING gap (ROADMAP §2, "A `real` rhs skips §10.7"), byte-identical
    // before and after this slice, pinned here so the slice owns no part of it.
    assert_eq!(o, "2.75 4.50 abx");
}

/// ⑩ DUPLICATE DRAW. A signed widening's fill bit is a SECOND mention of the
/// operand, and the engine walks the DAG as a tree — so an operand that cannot
/// be drawn twice is left alone. `$random` is the detector: a duplicated draw
/// shifts every later value in the stream.
#[test]
fn a_widened_region_never_draws_its_operand_twice() {
    let o = run(r#"module t;
  function [63:0] f_rnd; f_rnd = $random * 1; endfunction
  initial begin
    $display("%h %h", f_rnd(), $random);
    #1 $finish;
  end
endmodule
"#);
    // Byte-identical to PRE, and to iverilog's stream (`12153524`, `c0895e81`).
    assert_eq!(o, "0000000012153524 c0895e81");
}

/// ⑪ A WIDER OPERAND or an explicit cast was already widening the region before
/// this slice (that is why the ROADMAP entry's own `w16` twin looked correct);
/// those cells must not move.
#[test]
fn a_region_already_widened_by_an_operand_is_unchanged() {
    let o = run(r#"module t;
  logic [7:0] fld;
  function [31:0] w16(input [15:0] x);   w16    = fld * x;      endfunction
  function [31:0] cast32(input [7:0] x); cast32 = fld * 32'(x); endfunction
  initial begin
    fld = 8'hFF;
    $display("%h %h", w16(8'hFF), cast32(8'hFF));
    #1 $finish;
  end
endmodule
"#);
    assert_eq!(o, "0000fe01 0000fe01");
}

/// ⑫ A `real` / `realtime` TARGET IS NOT A WIDTH CONTEXT (§11.8.1): the integral
/// rhs stays self-determined and its RESULT converts. Round-1 review found the
/// opt-in firing on one — `rmul` printed `65025.000000` for both oracles'
/// `1.000000` (the 8-bit product, then converted). The `dims` map cannot answer
/// this (a real local carries a width there), so the fold classifies the target
/// with `ast_kind_is_bit_vector` / the return type.
#[test]
fn a_real_target_is_not_a_width_context() {
    let o = run(r#"module t;
  logic [7:0] a8 = 8'hFF, b8 = 8'hFF;
  logic signed [7:0] s8 = -100, q8 = -100;
  function real     rmul;  rmul  = a8 * b8;   endfunction
  function real     radd;  radd  = a8 + b8;   endfunction
  function real     rshl;  rshl  = a8 << 4;   endfunction
  function real     rsgn;  rsgn  = s8 * q8;   endfunction
  function real     rdiv;  rdiv  = a8 / 2;    endfunction
  function real     rpure; rpure = 2.5 * 2.0; endfunction
  function real     rmixr; rmixr = a8 * 1.0;  endfunction
  function realtime rtmul; rtmul = a8 * b8;   endfunction
  initial begin
    $display("%f %f %f %f", rmul(), radd(), rshl(), rsgn());
    $display("%f %f %f %f", rdiv(), rpure(), rmixr(), rtmul());
    #1 $finish;
  end
endmodule
"#);
    // Both oracles. `rdiv`/`rpure`/`rmixr` are the controls that held even while
    // the first draft was wrong — they never reach an integral widening.
    assert_eq!(
        o,
        "1.000000 254.000000 240.000000 16.000000\n\
         127.000000 5.000000 255.000000 1.000000"
    );
}

/// ⑬ The same for a `real` / `realtime` BODY LOCAL, in a function whose RETURN is
/// integral (so the return context is open and only the local's must close).
#[test]
fn a_real_body_local_is_not_a_width_context() {
    let o = run(r#"module t;
  logic [7:0] a8 = 8'hFF, b8 = 8'hFF;
  function [31:0] f_rloc;  begin real     x; x = a8 * b8; f_rloc  = x; end endfunction
  function real   f_rloc2; begin real     x; x = a8 * b8; f_rloc2 = x; end endfunction
  function [31:0] f_rt;    begin realtime x; x = a8 * b8; f_rt    = x; end endfunction
  initial begin
    $display("%0d %f %0d", f_rloc(), f_rloc2(), f_rt());
    #1 $finish;
  end
endmodule
"#);
    // Both oracles. POST of the first draft printed `65025 65025.000000 65025`.
    assert_eq!(o, "1 1.000000 1");
}

/// ⑭ §6.2 stays LOUD through the context walk. `lower_expr_ctx`'s `Unary` arm had
/// no real-operand check — its `lower_expr_ungated` twin does — and the opt-in
/// made that arm reachable for a fill-free rhs, turning a correct-loud E3009 into
/// a silent `0`. iverilog also refuses (`^ operator may not have a REAL operand`);
/// verilator accepts it, so this is oracle parity with iverilog, not a new
/// restriction.
#[test]
fn a_reduction_over_a_real_stays_loud_inside_the_context_walk() {
    let (o, e) = {
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let d = std::env::temp_dir().join(format!("vita_ibwc_e_{}_{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        let f = d.join("t.sv");
        std::fs::write(
            &f,
            r#"module t;
  real r;
  function [31:0] f(input [7:0] x); f = ^r; endfunction
  function [31:0] g(input [7:0] x); g = ~r; endfunction
  initial begin
    r = 2.5;
    $display("F=%0h G=%0h", f(8'h01), g(8'h01));
    #1 $finish;
  end
endmodule
"#,
        )
        .unwrap();
        let out = std::process::Command::new(env!("CARGO_BIN_EXE_vita"))
            .arg(f.to_str().unwrap())
            .current_dir(&d)
            .output()
            .expect("run vita");
        let s = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        let _ = std::fs::remove_dir_all(&d);
        (s, out.status.code())
    };
    assert_eq!(e, Some(1), "must exit 1 (loud), got:\n{o}");
    assert_eq!(
        o.matches("bitwise/shift/reduction not defined on real operand")
            .count(),
        2,
        "one diagnostic per site, got:\n{o}"
    );
    assert!(!o.contains("F="), "no value may be printed:\n{o}");
}

/// ⑮ A REAL-DOMAIN OPERAND anywhere in the context-determined region closes the
/// context (§11.8.1): the integral sub-expression is self-determined, computes at
/// its own width, and the RESULT converts. Round-2 review measured 15 cells where
/// the opt-in widened it first — `a8 * b8 + r` printed `65028` for both oracles'
/// `4` (`(255*255) mod 256 = 1`, then `1 + 2.5`, rounded).
///
/// Every row here is a different producer of the real domain (literal, body
/// local, formal, `$sqrt`, `$itor`, a ternary arm, a real one level deeper on
/// either side) crossed with a different consumer (`+`, `-`, `*`, `/`, `**`, a
/// 16-bit target, a bit-vector LOCAL target).
#[test]
fn a_real_operand_in_the_region_closes_the_context() {
    let o = run(r#"module t;
  logic [7:0] a8 = 8'hFF, b8 = 8'hFF;
  function [31:0] h01; h01 = a8 * b8 + 2.5;                       endfunction
  function [31:0] h03(input real rr); h03 = a8 * b8 + rr;         endfunction
  function [31:0] h04; real r; r = 1.0; h04 = a8 * b8 * r;        endfunction
  function [31:0] h06; real r; r = 2.5; h06 = (1'b1) ? a8 * b8 : r;    endfunction
  function [15:0] h07; real r; r = 2.5; h07 = a8 * b8 + r;        endfunction
  function [31:0] h08; real r; r = 2.0; h08 = a8 * b8 / r;        endfunction
  function [31:0] h10; logic [31:0] t; real r; r = 2.5; begin t = a8 * b8 + r; h10 = t; end endfunction
  function [31:0] g11; real r; r = 2.5; g11 = a8 * b8 + r;        endfunction
  function [31:0] i01; real r; r = 0.0; i01 = a8 + b8 + r;        endfunction
  function [31:0] i02; real r; r = 0.0; i02 = a8 * b8 - r;        endfunction
  function [31:0] i03; real r; r = 0.0; i03 = a8 * b8 + (r * 1.0); endfunction
  function [31:0] i04; real r; r = 0.0; i04 = (a8 * b8 + r) + 0;  endfunction
  function [31:0] j06; j06 = a8 * b8 + $sqrt(4.0);                endfunction
  function [31:0] j07; j07 = a8 * b8 + $itor(0);                  endfunction
  function [31:0] k02; real r; r = 0.0; k02 = a8 ** 2 + r;        endfunction
  initial begin
    $display("%0d %0d %0d %0d", h01(), h03(2.5), h04(), h06());
    $display("%0d %0d %0d %0d", h07(), h08(), h10(), g11());
    $display("%0d %0d %0d %0d", i01(), i02(), i03(), i04());
    $display("%0d %0d %0d", j06(), j07(), k02());
    #1 $finish;
  end
endmodule
"#);
    // Both oracles. The widened (wrong) readings were 65028 / 65028 / 65025 /
    // 65025 / 65028 / 32513 / 65028 / 65028 / 510 / 65025 / 65025 / 65025 /
    // 65027 / 65025 / 65025.
    // `h06`'s real arm and `k02`'s `** 2` are spelled as the review's designs
    // spell them: `(a8*b8) ** r` and `a8*b8 + (1 ? r : 0.0)` are ORACLE SPLITS
    // (iverilog 65025, verilator 1), and a split is not chased.
    assert_eq!(
        o,
        "4 4 1 1\n\
         4 1 4 4\n\
         254 1 1 1\n\
         3 1 1"
    );
}

/// ⑯ The BOUNDARY of ⑮, all still taking the context (or still not needing it).
/// `cmp` is the one that makes this a domain walk and not a "contains a real"
/// walk: a comparison's RESULT is a 1-bit integral, so the region keeps its width
/// context even though `r` is named inside it — both oracles print `65025`.
#[test]
fn an_integral_result_over_a_real_keeps_the_context() {
    let o = run(r#"module t;
  logic [7:0] a8 = 8'hFF, b8 = 8'hFF;
  real gr = 3.9;
  function [31:0] cmp;  real r; r = 0.0; cmp  = a8 * b8 + (r > 1.0); endfunction
  function [31:0] rtoi; rtoi = a8 * $rtoi(gr);                       endfunction
  function [31:0] cat;  real r; r = 0.0; cat  = {a8, b8} + r;        endfunction
  function [7:0]  narrow; real r; r = 0.0; narrow = a8 * b8 + r;     endfunction
  function [31:0] first;  real r; r = 2.5; first  = r + a8 * b8;     endfunction
  initial begin
    $display("%0d %0d %0d %0d %0d", cmp(), rtoi(), cat(), narrow(), first());
    #1 $finish;
  end
endmodule
"#);
    // Both oracles. `cmp` is this slice's gain (PRE printed 1); the other four
    // print the same value before and after the slice.
    assert_eq!(o, "65025 765 65535 1 4");
}
