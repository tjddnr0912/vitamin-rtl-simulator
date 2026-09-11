//! A name read inside a generate block keeps its SIGN in an untyped param
//! initializer (ROADMAP §5.2 row 1, §2 "Index sealing").
//!
//! ## What was wrong
//!
//! `Elaborator::const_expr_signed`'s single-segment `Ident` arm resolved with
//! `self.param_meta.get(&self.fq(name))`, which keys only the CURRENT prefix. A
//! genvar is seeded signed `(32, true)` under the prefix that ENCLOSES the generate
//! block (`top.g`), and so is any outer-scope param, so from inside `top.blk[0]` the
//! lookup missed and the arm answered UNSIGNED. The untyped-param tail at
//! `params.rs` (`ParamType::Implicit`, value-inferred sign) then bound the fold as
//! unsigned and materialized a 32-bit wrap:
//!
//! ```text
//! genvar g;
//! generate for (g = 0; g < 1; g = g + 1) begin : blk
//!   localparam A = g - 20;      // vita 4294967276, both oracles -20
//! end endgenerate
//! ```
//!
//! It is not genvar-specific — a module-scope `localparam integer GK = 0;` read from
//! inside a generate block (`localparam P = GK - 20;`) was wrong the same way, and so
//! was the §2 "Index sealing" cell `parameter signed [7:0] S8 = -1;` with
//! `localparam K = S8 >>> 1;` inside `generate if (1)`, which folded 255 where module
//! scope and both oracles give -1.
//!
//! ## The fix
//!
//! That arm resolves with `self.walk_scopes(name, &self.param_meta)` — the outward
//! scope walk, and the same resolver the env twin `const_signed_env`
//! (`const_fn_width.rs`) has always used. `const_signed_env` was already correct on
//! every override / range / select / generate-if cell measured, so it is the
//! specification here rather than a new rule. `walk_scopes` is the plain,
//! deliberately NON-shadow-aware walk (see `scope.rs`); a same-named param declared
//! inside the block is bound under the inner prefix and therefore still wins,
//! innermost-first, which `a_inner_param_shadows_outer_same_name` pins.
//!
//! ## Oracles
//!
//! Every expectation below is a line printed by BOTH iverilog 13 (`-g2012` + `vvp
//! -n`) and verilator 5.052 (`--binary --timing`), copied verbatim. The oracles did
//! not split on any cell here. The `M_d20` and `L_mul` controls pin values that must
//! NOT move: `g - 20'd20` has an unsigned sized operand, so the result is unsigned
//! and 4294967276 is correct, and `g * -1` is 0, where the sign is invisible.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_cess_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).expect("scratch dir");
    std::fs::write(d.join("t.sv"), src).expect("write design");
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

/// A clean run (exit 0) whose output contains every `want` line, verbatim as observed.
fn lines(src: &str, want: &[&str]) {
    let (o, code) = run(src);
    assert_eq!(code, Some(0), "expected a clean run:\n{o}");
    for w in want {
        assert!(o.contains(w), "expected {w:?} in:\n{o}");
    }
}

/// A LOUD run (exit 1) whose output carries the given diagnostic CODE.
///
/// Pinned on the code, never on the message TEXT.
fn loud_code(src: &str, code_str: &str) {
    let (o, code) = run(src);
    assert_eq!(code, Some(1), "expected a loud refusal:\n{o}");
    assert!(o.contains(code_str), "expected {code_str:?} in:\n{o}");
}

/// The eleven real-gap cells of the row's grounding census, one design.
///
/// `B_int` / `C_s32` are the TYPED controls (they were already correct — the sign
/// comes from the declaration, not from this predicate). `L_mul` / `M_d20` are the
/// must-not-move controls described in the module docstring.
#[test]
fn a_genvar_expression_in_an_untyped_generate_param_is_signed() {
    lines(
        r#"
module top;
  genvar g;
  generate for (g = 0; g < 1; g = g + 1) begin : blk
    localparam A_untyped     = g - 20;
    localparam integer B_int = g - 20;
    localparam signed [31:0] C_s32 = g - 20;
    localparam L_rev = -20 + g;
    localparam L_mul = g * -1;
    localparam L_neg = -g - 1;
    localparam L_div = g / 3 - 7;
    localparam L_shr = (g - 20) >>> 1;
    localparam M_d20 = g - 20'd20;
    localparam M_s32 = g - 32'sd20;
    localparam M_s8  = g - 8'sd20;
    initial begin
      $display("A_untyped=%0d", A_untyped);
      $display("B_int=%0d", B_int);
      $display("C_s32=%0d", C_s32);
      $display("L_rev=%0d", L_rev);
      $display("L_mul=%0d", L_mul);
      $display("L_neg=%0d", L_neg);
      $display("L_div=%0d", L_div);
      $display("L_shr=%0d", L_shr);
      $display("M_d20=%0d", M_d20);
      $display("M_s32=%0d", M_s32);
      $display("M_s8=%0d", M_s8);
      $display("O_rt_g=%0d", g);
      $display("O_rt_expr=%0d", g - 20);
    end
  end endgenerate
  initial #1 $finish;
endmodule
"#,
        &[
            "A_untyped=-20",
            "B_int=-20",
            "C_s32=-20",
            "L_rev=-20",
            "L_mul=0",
            "L_neg=-1",
            "L_div=-7",
            "L_shr=-10",
            "M_d20=4294967276",
            "M_s32=-20",
            "M_s8=-20",
            "O_rt_g=0",
            "O_rt_expr=-20",
        ],
    );
}

/// The defect is NOT genvar-specific: a module-scope `localparam integer` read from
/// inside a generate block (`P_ctl_gen`) was wrong where the same read at module
/// scope (`P_ctl_mod`) was right. The two override cells (`leafu`/`leafi`) took the
/// env twin's already-correct lane and are controls.
#[test]
fn b_module_scope_param_read_inside_a_generate_block_is_signed() {
    lines(
        r#"
module leafu #(parameter P = 0) (); initial $display("E_ovr_untyped=%0d", P); endmodule
module leafi #(parameter integer P = 0) (); initial $display("F_ovr_int=%0d", P); endmodule
module top;
  localparam integer GK = 0;
  localparam P_ctl_mod = GK - 20;
  genvar g;
  generate for (g = 0; g < 1; g = g + 1) begin : blk
    localparam P_ctl_gen = GK - 20;
    leafu #(.P(g - 20)) u1 ();
    leafi #(.P(g - 20)) u2 ();
    leafu #(.P(GK - 20)) u3 ();
    initial $display("P_ctl_gen=%0d", P_ctl_gen);
  end endgenerate
  initial $display("P_ctl_mod=%0d", P_ctl_mod);
  initial #1 $finish;
endmodule
"#,
        &[
            "P_ctl_gen=-20",
            "P_ctl_mod=-20",
            "E_ovr_untyped=-20",
            "F_ovr_int=-20",
        ],
    );
}

/// A genvar read TWO scope levels up (`J_nested`) and one level up (`J_inner`).
/// `G_range_bits` / `H_partsel` / `I_genif` are the non-param consumers of the same
/// expression; they already went through the env twin and must not move.
#[test]
fn c_nested_generate_scopes_walk_outward() {
    lines(
        r#"
module top;
  logic [31:0] vec = 32'hDEADBEEF;
  genvar g, h;
  generate for (g = 0; g < 1; g = g + 1) begin : blk
    logic [g-20+21:0] w;
    initial $display("G_range_bits=%0d", $bits(w));
    initial $display("H_partsel=%0d", vec[g-20+24 +: 4]);
    if (g - 20 < 0) begin : gi
      initial $display("I_genif=taken");
    end else begin : ge
      initial $display("I_genif=nottaken");
    end
    for (h = 0; h < 1; h = h + 1) begin : inner
      localparam J_nested = g - 20 + h;
      localparam J_inner  = h - 20;
      initial $display("J_nested=%0d J_inner=%0d", J_nested, J_inner);
    end
  end endgenerate
  initial #1 $finish;
endmodule
"#,
        &[
            "G_range_bits=2",
            "H_partsel=14",
            "I_genif=taken",
            "J_nested=-20 J_inner=-20",
        ],
    );
}

/// A NEGATIVE genvar. `N_untyped` / `N_int` were already correct (a bare `Ident`
/// initializer does not reach the value tail); only the expression cell moved.
#[test]
fn d_negative_genvar_expression_is_signed() {
    lines(
        r#"
module top;
  genvar g;
  generate for (g = -3; g < -2; g = g + 1) begin : blk
    localparam N_untyped = g;
    localparam N_expr    = g - 20;
    localparam integer N_int = g;
    initial begin
      $display("N_rt_g=%0d", g);
      $display("N_untyped=%0d", N_untyped);
      $display("N_expr=%0d", N_expr);
      $display("N_int=%0d", N_int);
    end
  end endgenerate
  initial #1 $finish;
endmodule
"#,
        &["N_rt_g=-3", "N_untyped=-3", "N_expr=-23", "N_int=-3"],
    );
}

/// The §2 "Index sealing" cell: an explicitly `signed [7:0]` module param read inside
/// `generate if (1)`. The arithmetic right shift folded 255 there while module scope
/// gave -1; both oracles print -1 at both scopes.
#[test]
fn e_signed_module_param_shifted_inside_a_generate_if() {
    lines(
        r#"
module top;
  parameter signed [7:0] S8 = -1;
  localparam S8_mod = S8 >>> 1;
  generate if (1) begin : gb
    localparam K = S8 >>> 1;
    initial $display("S8_gen_K=%0d", K);
  end endgenerate
  initial $display("S8_mod=%0d", S8_mod);
  initial #1 $finish;
endmodule
"#,
        &["S8_gen_K=-1", "S8_mod=-1"],
    );
}

/// The SHADOW axis. `walk_scopes` is innermost-first, so a generate block that
/// declares its OWN `g` reads the inner one (5 - 20 = -15) while module scope reads
/// the outer (7 - 20 = -13). Both oracles print exactly this, PRE and POST, so the
/// resolver change must not disturb it.
#[test]
fn f_inner_param_shadows_outer_same_name() {
    lines(
        r#"
module top;
  localparam integer g = 7;
  localparam SH_mod = g - 20;
  genvar i;
  generate for (i = 0; i < 1; i = i + 1) begin : blk
    localparam g = 5;
    localparam SH_gen = g - 20;
    initial $display("SH_gen=%0d", SH_gen);
  end endgenerate
  initial $display("SH_mod=%0d", SH_mod);
  initial #1 $finish;
endmodule
"#,
        &["SH_gen=-15", "SH_mod=-13"],
    );
}

/// `$signed` / `$unsigned` in a param initializer stays LOUD at EVERY scope — a
/// separate ROADMAP row, not this one. Pinned here so the loud→value column is not
/// silently opened by a later change to this resolver.
#[test]
fn g_signed_system_function_in_a_param_initializer_stays_loud() {
    loud_code(
        r#"
module top;
  localparam integer GK = 0;
  localparam S_mod = $signed(GK - 20);
  initial $display("S_mod=%0d", S_mod);
  initial #1 $finish;
endmodule
"#,
        "VITA-E3009",
    );
    loud_code(
        r#"
module top;
  genvar g;
  generate for (g = 0; g < 1; g = g + 1) begin : blk
    localparam K_sgn = $signed(g - 20);
    initial $display("K_sgn=%0d", K_sgn);
  end endgenerate
  initial #1 $finish;
endmodule
"#,
        "VITA-E3009",
    );
}
