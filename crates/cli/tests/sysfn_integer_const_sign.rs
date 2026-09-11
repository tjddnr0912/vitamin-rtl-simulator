//! §2 "Index sealing" — an INTEGER-returning system function folds SIGNED in
//! `const_expr_signed`.
//!
//! `const_expr_signed` (`elaborate/src/const_eval.rs`) had no `SysCall` arm, so
//! `$clog2(300)` fell to its `_ => false` tail and the untyped-parameter tail at
//! `params.rs:777` recorded the folded −11 as UNSIGNED: `localparam W = $clog2(300) - 20;
//! $display("%0d", W)` printed `4294967285` where BOTH oracles print `-11`. The defect
//! was band-independent (≤32 / 33..64 / >64 all wrong), reached every arithmetic
//! operator, both the value lane and the ELABORATION-STRUCTURE lane (a `generate if`
//! took the wrong branch), and the package-constant and default-binds lanes.
//!
//! Decisive pair: `localparam A = $clog2(300); localparam B = A - 20;` already printed
//! `-11` (the STORED meta for `A` is `(32, signed)` via `params.rs:709`) beside
//! `localparam C = $clog2(300) - 20;` printing `4294967285` — identical arithmetic,
//! identical stored leaf sign; only the expression walk was blind. `B`/`A` are pinned
//! here as `sign_stored_meta_leaf_was_already_signed`.
//!
//! ⚠️ **The admission is TWO NAMED lists, never a blanket `SysCall { .. } => true`**
//! (the spelling at `const_fn_width.rs:257`/`:354`). `sys_fn_is_integer` is
//! `$clog2 | $bits | $rtoi`; `is_dim_query_name` is the `$size`/`$high`/… family, which
//! `const_eval_in_scope` ALSO folds (`const_fn.rs:393`) — without it `$size(W8) - 20`
//! kept `4294967284` against both oracles' `-12`. Everything OUTSIDE both lists must
//! stay as it is: `$unsigned` is unsigned by definition, and `$itor` / `$realtobits` /
//! `$sformatf` are not in this domain at all. The `must_stay_loud_*` tests are the
//! controls: they assert a non-zero exit and the `VITA-E3009` CODE, never the message
//! text.
//!
//! ⚠️ **The WIDTH half is an oracle split and is NOT touched here.** iverilog answers
//! 33 / 65 / 71 bits where verilator answers 32 / 64 / 70 (and `%h` of the same value is
//! `1fffffff5` vs `fffffff5`); vita matches verilator. Only `bits=` values the two
//! oracles AGREE on are pinned (`$bits($clog2(300)) == 32`).
//!
//! ⚠️ **The OVERRIDE lane was already correct** and must stay that way: it forks to
//! `const_signed_env` at `params.rs:124`, which has answered signed all along.
//! `override_lane_was_already_signed` pins it.
//!
//! ⚠️ **`$high` / `$low` VALUE is a genuine 1-oracle cell** — iverilog returns
//! range-relative bounds (`$high([15:8]) = 7`), verilator the declared ones (`15`).
//! They are NOT pinned; only `$size`, on which both oracles agree, is.
//!
//! Values pinned to iverilog 13.0 and verilator 5.052; every design carries an
//! `initial #1 $finish;` watchdog.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_sfics_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (s, out.status.success())
}

/// `want` must appear and the run must SUCCEED — a diagnostic here would be a loud
/// regression, which the `ok` half catches.
#[track_caller]
fn chk(src: &str, want: &str) {
    let (s, ok) = run(src);
    assert!(ok, "vita rejected the design:\n{s}");
    assert!(s.contains(want), "want {want:?}\ngot:\n{s}");
}

/// The design must be REFUSED: non-zero exit and the `VITA-E3009` code. The message
/// TEXT is deliberately not asserted (a wording pin outlives its limitation).
#[track_caller]
fn chk_loud(src: &str) {
    let (s, ok) = run(src);
    assert!(!ok, "vita accepted a design that must stay loud:\n{s}");
    assert!(s.contains("VITA-E3009"), "want VITA-E3009\ngot:\n{s}");
}

/// Wrap a module body that prints and finishes.
fn top(body: &str) -> String {
    format!("module top;\n{body}\n  initial #1 $finish;\nendmodule\n")
}

// ===== A. the three `sys_fn_is_integer` leaves — PRE unsigned, both oracles signed ====

/// g1.A_CLOG — PRE `A_CLOG=4294967285` · iverilog/verilator `A_CLOG=-11`.
#[test]
fn clog2_minus_literal_is_signed() {
    chk(
        &top(
            "  localparam A_CLOG = $clog2(300) - 20;\n  initial $display(\"A_CLOG=%0d\", A_CLOG);",
        ),
        "A_CLOG=-11",
    );
}

/// g1.A_BITS — PRE `A_BITS=4294967288` · both oracles `A_BITS=-8`.
#[test]
fn bits_minus_literal_is_signed() {
    chk(
        &top("  logic [11:0] x;\n  localparam A_BITS = $bits(x) - 20;\n  initial $display(\"A_BITS=%0d\", A_BITS);"),
        "A_BITS=-8",
    );
}

/// g1.A_RTOI — PRE `A_RTOI=4294967279` · both oracles `A_RTOI=-17`.
#[test]
fn rtoi_minus_literal_is_signed() {
    chk(
        &top("  localparam A_RTOI = $rtoi(3.7) - 20;\n  initial $display(\"A_RTOI=%0d\", A_RTOI);"),
        "A_RTOI=-17",
    );
}

// ===== B. the dim-query family — the half `sys_fn_is_integer` alone does NOT cover ====

/// g2.B_SIZE — PRE `B_SIZE=4294967284` · both oracles `B_SIZE=-12`. This is the cell
/// that makes `|| is_dim_query_name(..)` mandatory: `sys_fn_is_integer` excludes
/// `$size`, but `const_fn.rs:393` folds it, so it reaches the untyped-param tail with a
/// value and (without the second list) the wrong sign.
#[test]
fn dim_query_size_minus_literal_is_signed() {
    chk(
        &top("  localparam logic [15:8] W8 = 8'hA5;\n  localparam B_SIZE = $size(W8) - 20;\n  initial $display(\"B_SIZE=%0d\", B_SIZE);"),
        "B_SIZE=-12",
    );
}

// ===== C. position / binder lanes ====

/// g5.u_def — the DEFAULT-BINDS lane of a header `parameter` (`params.rs:126`, the
/// non-`declared_only` fork). PRE `sub P=4294967285` · both oracles `sub P=-11`.
#[test]
fn default_binds_lane_is_signed() {
    chk(
        "module sub #(parameter P = $clog2(300) - 20) ();\n  initial $display(\"sub P=%0d\", P);\nendmodule\nmodule top;\n  sub u_def ();\n  initial #1 $finish;\nendmodule\n",
        "sub P=-11",
    );
}

/// g5.pkgPP — a PACKAGE constant, read back through `pkg_const_meta`. PRE
/// `pkgPP=4294967285` · both oracles `pkgPP=-11`.
#[test]
fn package_constant_is_signed() {
    chk(
        "package pk;\n  localparam PP = $clog2(300) - 20;\nendpackage\nmodule top;\n  initial begin\n    $display(\"pkgPP=%0d\", pk::PP);\n    #1 $finish;\n  end\nendmodule\n",
        "pkgPP=-11",
    );
}

/// g5.gen — the defect changed ELABORATION STRUCTURE, not only a printed value: the
/// `generate if` took the wrong branch. PRE `gen=pos` · both oracles `gen=neg`.
#[test]
fn generate_branch_selection_is_signed() {
    chk(
        &top("  localparam W = $clog2(300) - 20;\n  if (W < 0) begin : g_neg\n    initial $display(\"gen=neg\");\n  end else begin : g_pos\n    initial $display(\"gen=pos\");\n  end"),
        "gen=neg",
    );
}

/// g5.tern — a constant ternary CONDITION. PRE `tern=2` · both oracles `tern=1`.
#[test]
fn constant_ternary_condition_is_signed() {
    chk(
        &top("  localparam W = $clog2(300) - 20;\n  initial $display(\"tern=%0d\", (W < 0) ? 1 : 2);"),
        "tern=1",
    );
}

/// g5.W_cmp — the same comparison evaluated at RUNTIME. PRE `W_cmp=2` · both oracles
/// `W_cmp=1`. Its twin `rt_cmp` (the value first assigned to a `logic signed [31:0]`)
/// was already `1` in PRE, which is what proves the defect is the PARAM's recorded sign
/// and not the comparison operator.
#[test]
fn runtime_comparison_against_param_is_signed() {
    chk(
        &top("  localparam W = $clog2(300) - 20;\n  logic signed [31:0] r;\n  initial begin\n    r = W;\n    $display(\"rt_cmp=%0d\", (r < 0) ? 1 : 2);\n    $display(\"W_cmp=%0d\", (W < 0) ? 1 : 2);\n  end"),
        "W_cmp=1",
    );
}

// ===== D. operator census — five operators, each with its literal twin ====

/// g6 — `-` `*` `/` `%` and unary `-`. PRE `O_SUB=4294967285 O_MUL=4294967287
/// O_DIV=4294967291 O_MOD=4294967294 O_NEG=4294967287`; both oracles print the `L_*`
/// literal twins' values, which PRE already printed correctly.
#[test]
fn operator_census_all_five_are_signed() {
    let src = top(
        "  localparam O_SUB = $clog2(300) - 20;\n  localparam O_MUL = $clog2(300) * -1;\n  localparam O_DIV = ($clog2(300) - 20) / 2;\n  localparam O_MOD = ($clog2(300) - 20) % 3;\n  localparam O_NEG = -$clog2(300);\n  localparam L_SUB = 9 - 20;\n  localparam L_MUL = 9 * -1;\n  localparam L_DIV = (9 - 20) / 2;\n  localparam L_MOD = (9 - 20) % 3;\n  localparam L_NEG = -9;\n  initial begin\n    $display(\"O_SUB=%0d L_SUB=%0d\", O_SUB, L_SUB);\n    $display(\"O_MUL=%0d L_MUL=%0d\", O_MUL, L_MUL);\n    $display(\"O_DIV=%0d L_DIV=%0d\", O_DIV, L_DIV);\n    $display(\"O_MOD=%0d L_MOD=%0d\", O_MOD, L_MOD);\n    $display(\"O_NEG=%0d L_NEG=%0d\", O_NEG, L_NEG);\n  end",
    );
    for want in [
        "O_SUB=-11 L_SUB=-11",
        "O_MUL=-9 L_MUL=-9",
        "O_DIV=-5 L_DIV=-5",
        "O_MOD=-2 L_MOD=-2",
        "O_NEG=-9 L_NEG=-9",
    ] {
        chk(&src, want);
    }
}

// ===== E. wrapper forms ====

/// g7.P_PAREN — the `Paren` arm recurses straight into the arm that was missing. PRE
/// `P_PAREN=4294967285` · both oracles `P_PAREN=-11`.
#[test]
fn parenthesised_syscall_is_signed() {
    chk(
        &top("  localparam P_PAREN = ($clog2(300)) - 20;\n  initial $display(\"P_PAREN=%0d\", P_PAREN);"),
        "P_PAREN=-11",
    );
}

// ===== F. width bands — the defect was band-independent, and so is the fix ====

/// g10 — a SIGNED operand at ≤32 / 33..64 / >64 bits. PRE `G16=4294967285`
/// `G64=18446744073709551605` `G70=1180591620717411303413`; both oracles print `-11` in
/// every band (only the `bits=` column splits, and it is not pinned here).
#[test]
fn signed_wide_operand_all_three_bands() {
    let src = top(
        "  localparam G16 = $clog2(300) - 16'sd20;\n  localparam G64 = $clog2(300) - 64'sd20;\n  localparam G70 = $clog2(300) - 70'sd20;\n  localparam L16 = 9 - 16'sd20;\n  localparam L64 = 9 - 64'sd20;\n  localparam L70 = 9 - 70'sd20;\n  initial begin\n    $display(\"G16=%0d L16=%0d\", G16, L16);\n    $display(\"G64=%0d L64=%0d\", G64, L64);\n    $display(\"G70=%0d L70=%0d\", G70, L70);\n  end",
    );
    for want in ["G16=-11 L16=-11", "G64=-11 L64=-11", "G70=-11 L70=-11"] {
        chk(&src, want);
    }
}

// ===== G. controls that must keep TODAY'S value ====

/// g6.O_SHR — `>>` is LOGICAL even for a signed operand in all three tools, so making
/// the operand signed must NOT change it. Its literal twin `L_SHR` already took the
/// signed path to the same value in PRE. PRE = POST = both oracles = `2147483642`.
#[test]
fn logical_shift_right_value_is_unchanged() {
    chk(
        &top("  localparam O_SHR = ($clog2(300) - 20) >> 1;\n  localparam L_SHR = (9 - 20) >> 1;\n  initial $display(\"O_SHR=%0d L_SHR=%0d\", O_SHR, L_SHR);"),
        "O_SHR=2147483642 L_SHR=2147483642",
    );
}

/// g8.bits_v — a RANGE BOUND that was correct BY CANCELLATION before the fix
/// (`0xFFFFFFF5 + 12` wraps to 1 in the 32-bit lane; `−11 + 12 = 1` in the oracles).
/// Both roads must still reach `2`. PRE = POST = both oracles.
#[test]
fn range_bound_from_syscall_param_is_unchanged() {
    chk(
        &top("  localparam W = $clog2(300) - 20;\n  localparam L = 9 - 20;\n  logic [W+12:0] v;\n  logic [L+12:0] u;\n  initial begin\n    $display(\"bits_v=%0d\", $bits(v));\n    $display(\"bits_u=%0d\", $bits(u));\n  end"),
        "bits_v=2",
    );
}

/// g9.A / g9.B — the decisive pair. `A` is the BARE call (`params.rs:709` already
/// stored `(32, signed)` for it) and `B` derives from `A` through the same arithmetic
/// that was wrong when written inline. Both were already correct in PRE and must stay.
/// The `bits=32` pin is the one width both oracles agree on.
#[test]
fn sign_stored_meta_leaf_was_already_signed() {
    let src = top(
        "  localparam A = $clog2(300);\n  localparam B = A - 20;\n  initial begin\n    $display(\"A=%0d bits=%0d\", A, $bits(A));\n    $display(\"B=%0d\", B);\n  end",
    );
    chk(&src, "A=9 bits=32");
    chk(&src, "B=-11");
}

/// g1.A_TYPED — a DECLARED type wins over the inferred sign; correct in PRE.
#[test]
fn typed_declaration_is_unchanged() {
    chk(
        &top("  localparam integer A_TYPED = $clog2(300) - 20;\n  initial $display(\"A_TYPED=%0d\", A_TYPED);"),
        "A_TYPED=-11",
    );
}

/// g7.P_CAST / g7.P_FN — the `Cast`→`Prim` arm and the `Call` arm (declared return
/// type) already answered signed; correct in PRE, must stay.
#[test]
fn cast_and_const_function_lanes_are_unchanged() {
    chk(
        &top("  localparam P_CAST = int'($clog2(300)) - 20;\n  initial $display(\"P_CAST=%0d\", P_CAST);"),
        "P_CAST=-11",
    );
    chk(
        "function automatic integer f(input integer n);\n  f = $clog2(n) - 20;\nendfunction\nmodule top;\n  localparam P_FN = f(300);\n  initial begin\n    $display(\"P_FN=%0d\", P_FN);\n    #1 $finish;\n  end\nendmodule\n",
        "P_FN=-11",
    );
}

/// g5.u_ovr — the OVERRIDE lane forks to `const_signed_env` (`params.rs:124`) and was
/// already signed; the fix must not disturb it.
#[test]
fn override_lane_was_already_signed() {
    chk(
        "module sub #(parameter P = 0) ();\n  initial $display(\"sub P=%0d\", P);\nendmodule\nmodule top;\n  sub #(.P($clog2(300) - 20)) u_ovr ();\n  initial #1 $finish;\nendmodule\n",
        "sub P=-11",
    );
}

// ===== H. must stay LOUD — the names OUTSIDE both lists ====

/// g3.C_UNS — `$unsigned` is unsigned BY DEFINITION and has no fold arm; a blanket
/// `SysCall { .. } => true` would claim it signed the day it folds. It is loud today in
/// vita (both oracles print `4294967271`), and it must stay loud, not silently signed.
#[test]
fn must_stay_loud_unsigned_syscall() {
    chk_loud(&top(
        "  localparam C_UNS = $unsigned(-5) - 20;\n  initial $display(\"C_UNS=%0d\", C_UNS);",
    ));
}

/// g7.P_SCAST — `signed'(…)` is a separate, pre-existing decline
/// (`CastTarget::Signing` is not folded by `const_eval_cast`). Both oracles print
/// `-11`; vita refuses. Pinned so this slice cannot be credited with closing it and so
/// a later fix has to move this pin deliberately.
#[test]
fn must_stay_loud_signing_cast() {
    chk_loud(&top(
        "  localparam P_SCAST = signed'($clog2(300)) - 20;\n  initial $display(\"P_SCAST=%0d\", P_SCAST);",
    ));
}
