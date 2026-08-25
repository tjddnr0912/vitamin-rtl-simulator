//! §2 「다음 착수 순서」 #1 — a constant-function body's DECLARATION binding.
//!
//! Three findings of one root: the interpreter turned things it did not know
//! into values.
//!
//!   1. A body/block local's initializer shared one `.unwrap_or(0)` with "there
//!      is no initializer", so an init the interpreter could not fold became a
//!      silent 0: `int x = 8'(5); g = x;` answered 0 where iverilog answers 5 —
//!      a wrong `localparam` at exit 0, and the opposite of what the same text
//!      written `int x; x = 8'(5);` already did (LOUD). The local is left
//!      UNBOUND instead, so an unfoldable initializer is loud only when the
//!      local is actually READ — one whose init is dead still folds as it did.
//!      And what the interpreter could not fold shrank: a concatenation, a
//!      replication and a SIZE CAST are self-determined bit placements, so they
//!      route through the carry-free wide folder (`const_placement_env`) — the
//!      same helper module scope now uses, which is what stops the two scopes
//!      from answering differently. `8'(5)`, `{4'hA,4'hB}` and `{2{4'hA}}` are
//!      VALUES here now, not diagnostics.
//!   2. Reading an unbound local must not fall through to the module scope: a
//!      same-named parameter would answer for a reference that names the local.
//!   3. A body declaration folded at the CALLER's depth, so `int t = g();`
//!      inside `g` re-entered the same ast node forever — a stack overflow, not
//!      a diagnostic. Declarations are part of the body and now run at the
//!      body's depth, which the existing cap bounds.
//!
//! And the width record those three exposed: a MULTI-packed local declined its
//! width entirely, and an unknown target contributes nothing to §11.6's
//! `max(self, target)` — so `bit [1:0][3:0] tt; tt = 4'd13 ** 4'd2;` evaluated
//! at the RHS's own 4 bits and stored 9 for iverilog's 169. The width is the
//! PRODUCT of the dimensions and is now computed.
//!
//! Oracle: iverilog 13.0 for every value here.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_raw(src: &str) -> (String, Option<i32>, String) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_cfdb_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        out.status.code(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// Wrap a constant-function body so its value lands in a `localparam` — the
/// position where a wrong fold is a silently wrong parameter.
fn func(body: &str, extra: &str) -> String {
    format!(
        "module top;\n{extra}\n  function integer g();\n{body}\n  endfunction\n\
         localparam integer L = g();\n\
         initial begin $display(\"R=%0d\", L); #1 $finish; end\nendmodule\n"
    )
}

fn folds(body: &str, extra: &str, want: i64) {
    let (out, code, err) = run_raw(&func(body, extra));
    assert_eq!(code, Some(0), "expected a fold, stderr:\n{err}");
    assert!(
        out.contains(&format!("R={want}")),
        "want R={want}; got:\n{out}"
    );
}

fn loud(body: &str, extra: &str) {
    let (_, code, err) = run_raw(&func(body, extra));
    assert_eq!(
        code,
        Some(1),
        "expected a loud reject, not {code:?}:\n{err}"
    );
    assert!(
        err.contains("is not a foldable constant expression")
            || err.contains("value is not a constant:"),
        "unexpected diagnostic:\n{err}"
    );
}

/// #1 headline. Every one of these answered a silent 0 at exit 0; they are
/// iverilog's values now. The placement arms are what closed them, and the
/// ASSIGNMENT spelling of the identical text — which used to be LOUD, the
/// self-inconsistency that found this — now agrees cell for cell.
#[test]
fn placement_and_cast_initializers_fold() {
    for (body, want) in [
        ("    int x = 8'(5);\n    g = x;", 5),
        ("    int x = {4'hA,4'hB};\n    g = x;", 171),
        ("    int x = {2{4'hA}};\n    g = x;", 170),
        ("    begin int y = 8'(5); g = y; end", 5),
        // §11.8.1 `max(self, N)`: the 4-bit sum wraps to 0 at its own width only
        // if the cast does NOT lift it — iverilog says 16, so the cast does.
        ("    int x = 8'(4'd15 + 4'd1);\n    g = x;", 16),
        ("    int x = 8'({2{2'b11}});\n    g = x;", 15),
        // A concat operand may be a bound LOCAL (the resolver reads the
        // interpreter's env first, so a local shadows a same-named param)…
        (
            "    bit [3:0] b = 3;\n    int x = {4'd2, b};\n    g = x;",
            35,
        ),
        // …and the assignment spelling of a replication, LOUD before this slice.
        ("    int x;\n    x = {2{4'hA}};\n    g = x;", 170),
    ] {
        folds(body, "", want);
    }
    // …or a module PARAM, at its declared width.
    folds(
        "    int x = {4'd2, P};\n    g = x;",
        "  parameter bit [3:0] P = 4'd3;",
        35,
    );
}

/// The i64 const domain carries 63 UNSIGNED magnitude bits, so a placement that
/// does not FIT it declines — the boundary the deleted hand-rolled concat loop
/// spelled as `total > 63`. Relaxing it to "64 bits" let a top-bit-set 64-bit
/// concatenation reach a consumer as a NEGATIVE i64, where `> 0` answered false
/// against BOTH oracles and a generate-if silently elaborated the other branch.
/// The gain the old cap gave away is kept: 63 bits, and 64 bits with the top bit
/// CLEAR, still fold.
///
/// ⚠️ The declined cells are honest-loud, not correct — iverilog has a value for
/// each. The same shape is a PRE-EXISTING silent-wrong for a 64-bit LITERAL and a
/// 64-bit PARAM (`(64'hFFFFFFFF00000000 > 0)` answers 222 where iverilog answers
/// 111, before and after this slice), tracked in ROADMAP §2 — which is why
/// folding these would have EXTENDED that class to new syntax, not matched it.
#[test]
fn a_placement_that_does_not_fit_the_i64_domain_declines() {
    // 64 bits, top bit SET — every consumer shape the review measured.
    loud("    int x = {32'hFFFFFFFF, 32'h0};\n    g = x;", "");
    loud("    int x = {2{32'hFFFFFFFF}};\n    g = x;", "");
    // ⭐ The two MODULE-SCOPE cells below used to assert a loud reject, and the
    // docstring above called them "honest-loud, not correct — iverilog has a value for
    // each". They are correct now, and the mechanism is the same one that closed the
    // 64-bit comparison generally: the WIDE bit domain compares inside the width the
    // operands declare, so a 64-bit value with the top bit set is a large positive
    // number rather than a negative i64. Both oracles: 111 for each.
    let (out, code, err) = run_raw(
        "module top;\n\
         localparam integer L = ({32'hFFFFFFFF, 32'h0} > 0) ? 111 : 222;\n\
         initial begin $display(\"R=%0d\", L); #1 $finish; end\n\
         endmodule\n",
    );
    assert_eq!(code, Some(0), "module-scope 64-bit placement:\n{err}");
    assert!(out.contains("R=111"), "both oracles say 111:\n{out}");
    let (out, code, err) = run_raw(
        "module top;\n\
         generate if ({1'b1, 63'd0} > 0) begin : y\n\
         initial begin $display(\"R=%0d\", 111); #1 $finish; end\n\
         end else begin : n\n\
         initial begin $display(\"R=%0d\", 222); #1 $finish; end\n\
         end endgenerate\n\
         endmodule\n",
    );
    assert_eq!(code, Some(0), "generate-if over a 64-bit value:\n{err}");
    assert!(out.contains("R=111"), "both oracles take THEN:\n{out}");
    // …and what still folds: 63 bits, and 64 bits with the top bit clear.
    folds("    int x = {63{1'b1}} > 0;\n    g = x;", "", 1);
    folds("    longint x = {32'h1, 32'h0};\n    g = x >> 32;", "", 1);
}

/// A SIZE cast propagates the OPERAND's signedness (§6.24.1), which is what makes
/// `8'(255)` iverilog's −1 and not 255: the decimal literal is signed, so the
/// cast sign-extends the truncated pattern. An unsigned operand keeps its value.
#[test]
fn size_cast_inherits_the_operand_signedness() {
    folds("    int x = 8'(255);\n    g = x;", "", -1);
    folds("    int x = 8'(-1);\n    g = x;", "", -1);
    folds("    int x = 4'(4'sb1111);\n    g = x;", "", -1);
    folds("    int x = 8'(8'sd200);\n    g = x;", "", -56);
    // Unsigned operands are NOT sign-extended — the control that keeps the rule
    // from becoming "always signed".
    folds(
        "    int x = 8'(8'hFF);\n    int y = 8'(4'hF);\n    g = x * 1000 + y;",
        "",
        255015,
    );
    // ⚠️ A ≥64-bit TARGET is the only place the cast's own sign is observable:
    // below 64 the enclosing context re-derives it (`leaf_into_ctx`), so every
    // narrower cell above stays correct even if the cast hands over the wrong
    // signedness. Proven on a mutant build — with the sign forced unsigned this
    // one cell answers 255 and all the others still answer iverilog's values.
    folds("    longint x = 8'(255);\n    g = x;", "", -1);
}

/// Module scope and a constant-function body share ONE placement helper, so the
/// same text folds to the same value in both. A replication and a
/// param-bearing concatenation were honest-loud at module scope before this.
#[test]
fn module_scope_and_body_agree_on_placement() {
    for (expr, want) in [
        ("{4'hA,4'hB}", 171),
        ("{2{4'hA}}", 170),
        ("{4'd2,P}", 35),
        ("8'(4'd15 + 4'd1)", 16),
    ] {
        let src = format!(
            "module top;\n  parameter bit [3:0] P = 4'd3;\n             localparam integer L = {expr};\n             initial begin $display(\"R=%0d\", L); #1 $finish; end\nendmodule\n"
        );
        let (out, code, err) = run_raw(&src);
        assert_eq!(code, Some(0), "module scope `{expr}`, stderr:\n{err}");
        assert!(out.contains(&format!("R={want}")), "`{expr}`; got:\n{out}");
        folds(
            &format!("    int x = {expr};\n    g = x;"),
            "  parameter bit [3:0] P = 4'd3;",
            want,
        );
    }
}

/// What the placement folder still declines stays LOUD — it is CARRY-FREE, so an
/// arithmetic operand inside a concatenation is not admitted, and it refuses x/z
/// rather than growing a second 4-state table. A prim cast keeps its own rule
/// (its operand is self-determined, not `max(self, N)`) and is not routed there.
/// Every one of these was a silent 0 before the slice.
#[test]
fn unfoldable_decl_init_is_loud_not_a_silent_zero() {
    // `{4'd2, (4'd1+4'd1)}` was the first cell here and it FOLDS now — 34, which is
    // what both oracles say. What remains unfoldable is what the domain cannot
    // represent (an x/z bit) or cannot model (a cast in a function-local initializer).
    folds("    int x = {4'd2, (4'd1+4'd1)};\n    g = x;", "", 34);
    loud("    int x = {4'd2, 4'bxx01};\n    g = x;", "");
    loud("    int x = int'(7);\n    g = x;", "");
    // A concat operand that is an UNBOUND local declines rather than letting the
    // module scope answer for it.
    loud(
        "    int u = int'(7);\n    int x = {4'd2, u};\n    g = x;",
        "",
    );
    // A replication COUNT is not a runtime value (iverilog rejects this too).
    loud("    int n = 2;\n    int x = {n{4'hA}};\n    g = x;", "");
}

/// The loud is scoped to a READ. A local whose unfoldable initializer is DEAD —
/// overwritten before use, or never used — still folds, so nothing that worked
/// before descends to loud. (Every value here is iverilog's.)
#[test]
fn dead_unfoldable_decl_init_still_folds() {
    // A shape the CARRY-FREE placement folder declines, so it stands in for
    // "the interpreter cannot fold this initializer" now that a bare cast,
    // concatenation and replication all do fold.
    // ⚠️ The stand-in used to be `{4'd2, (4'd1+4'd1)}`, chosen because the CARRY-FREE
    // placement folder declined an addition. It folds now (34, both oracles), so the
    // proxy had to become one that cannot stop being unfoldable: a parameter value has
    // no x/z plane, so an initializer carrying UNKNOWN bits can never have one.
    const DEAD: &str = "{4'd2, 4'bxx01}";
    folds(
        &format!("    int x = {DEAD};\n    x = 42;\n    g = x;"),
        "",
        42,
    );
    folds(&format!("    int x = {DEAD};\n    g = 11;"), "", 11);
    folds(
        &format!("    begin int y = {DEAD}; y = 9; g = y; end"),
        "",
        9,
    );
    // An inner declaration shadows an outer BOUND one: unbound while unread…
    loud(
        &format!("    begin int y = 5; begin int y = {DEAD}; g = y; end end"),
        "",
    );
    // …and bound again by the assignment.
    folds(
        &format!("    begin int y = 5; begin int y = {DEAD}; y = 8; g = y; end end"),
        "",
        8,
    );
    // No initializer at all still binds 0 — that half of the old `unwrap_or`
    // was the correct half.
    folds("    int x;\n    g = x + 1;", "", 1);
}

/// #2: an unbound local is the interpreter's own name, so reading it is loud —
/// never the module-scope value of a same-named parameter, which would answer a
/// different object than the reference names. The three neighbouring
/// resolutions are unchanged.
#[test]
fn unbound_local_never_resolves_a_same_named_param() {
    let p = "  parameter integer X = 99;";
    loud("    int X = {4'd2, 4'bxx01};\n    g = X;", p);
    folds("    int X = 7;\n    g = X;", p, 7);
    folds("    g = X;", p, 99);
    folds("    int X = {4'd2, 4'bxx01};\n    X = 3;\n    g = X;", p, 3);
    // An initializer that mentions the name being DECLARED sees the local, not
    // yet bound — never the param. iverilog answers 0 here; answering 99 is what
    // happens if the width twin is recorded AFTER the fold instead of before it.
    loud("    int x = x;\n    g = x;", "  parameter integer x = 99;");
    // A local the interpreter cannot model at all is loud even where a param of
    // the same name could have answered — skipping the declaration instead of
    // declining the fold would let it.
    loud(
        "    real r = 1.5;\n    g = r;",
        "  parameter integer r = 99;",
    );
    // A local whose declared WIDTH is unknown (its range calls the function being
    // folded, which the recursion guard declines) is still the interpreter's own
    // name: the loud must not be conditional on the width being known.
    //
    // ⚠️ That is the property this cell was written for, and it only STARTED being the
    // thing under test here. The pair used to split — `= {4'd2, (4'd1+4'd1)}` was loud
    // and `= 7` folded to 7 — but the split was an accident of which INITIALIZERS the
    // placement folder could handle, not of the width; once it could handle both, one
    // unknown-width local bound 34 and its neighbour bound 7, and nothing in either
    // path would ever truncate them.
    //
    // ⚠️ NEITHER ORACLE HAS A VALUE HERE: iverilog aborts on this shape (`assert:
    // elab_expr.cc:2927: failed assertion def`) and verilator fails to build it. So
    // the choice is vita's, and it is the safe side of the ladder — a value bound at a
    // width nothing can state is not a value this domain will claim. Both cells loud.
    for init in ["{4'd2, (4'd1+4'd1)}", "7"] {
        loud(
            &format!("    bit [g()-1:0] X = {init};\n    g = X;"),
            "  parameter integer X = 99;",
        );
    }
}

/// #3: a self-referential declaration initializer used to recurse at a constant
/// depth until the process died with a stack overflow (SIGABRT, no diagnostic).
/// It is bounded by the existing depth cap now. iverilog rejects both designs.
#[test]
fn self_referential_decl_init_is_loud_not_a_stack_overflow() {
    loud("    int t = g();\n    g = t;", "");
    let (_, code, err) = run_raw(
        "module top;\n\
         function automatic integer f(input integer n);\n\
         int t = f(n-1);\n\
         f = t;\n\
         endfunction\n\
         localparam integer L = f(4);\n\
         initial begin $display(\"R=%0d\", L); #1 $finish; end\n\
         endmodule\n",
    );
    assert_eq!(code, Some(1), "expected loud, not a crash:\n{err}");
    assert!(
        err.contains("is not a foldable constant expression")
            || err.contains("value is not a constant:")
    );
}

/// Charging the declaration a depth level must not cost a legitimate design:
/// a finite call in an initializer, a recursion with a base case, a 30-deep
/// chain and a mutual recursion all still fold to iverilog's answers.
#[test]
fn finite_calls_in_a_decl_init_still_fold() {
    let base = "module top;\n\
                function automatic integer f(input integer n);\n\
                int t = (n <= 0) ? {B} : f(n-1) + 1;\n\
                f = t;\n\
                endfunction\n\
                localparam integer L = f({N});\n\
                initial begin $display(\"R=%0d\", L); #1 $finish; end\n\
                endmodule\n";
    for (b, n, want) in [("0", "4", 4), ("1", "30", 31)] {
        let src = base.replace("{B}", b).replace("{N}", n);
        let (out, code, err) = run_raw(&src);
        assert_eq!(code, Some(0), "stderr:\n{err}");
        assert!(out.contains(&format!("R={want}")), "got:\n{out}");
    }
    let (out, code, err) = run_raw(
        "module top;\n\
         function integer f(); f = 7; endfunction\n\
         function integer g(); int t = f(); g = t; endfunction\n\
         localparam integer L = g();\n\
         initial begin $display(\"R=%0d\", L); #1 $finish; end\n\
         endmodule\n",
    );
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("R=7"), "got:\n{out}");
    let (out, code, err) = run_raw(
        "module top;\n\
         function automatic integer a(input integer n); a = (n<=0) ? 0 : b(n-1); endfunction\n\
         function automatic integer b(input integer n); int t = a(n-1) + 2; b = t; endfunction\n\
         localparam integer L = b(6);\n\
         initial begin $display(\"R=%0d\", L); #1 $finish; end\n\
         endmodule\n",
    );
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("R=8"), "got:\n{out}");
    // A recursion whose call sits in a BLOCK-local initializer must cost ONE
    // depth level per frame, not two: the block arm's `depth` is already the
    // body's depth, so charging it again would halve the reachable recursion.
    let (out, code, err) = run_raw(
        "module top;\n\
         function automatic integer f(input integer n);\n\
         begin\n\
         int t;\n\
         if (n <= 0) t = 0;\n\
         else begin int u = f(n-1); t = u + 1; end\n\
         f = t;\n\
         end\n\
         endfunction\n\
         localparam integer L = f(40);\n\
         initial begin $display(\"R=%0d\", L); #1 $finish; end\n\
         endmodule\n",
    );
    assert_eq!(code, Some(0), "block-local decl recursion, stderr:\n{err}");
    assert!(out.contains("R=40"), "got:\n{out}");
}

/// The width record the three above exposed: a multi-packed local's target width
/// is the PRODUCT of its dimensions, so §11.6's `max(self, target)` sees it. Both
/// spellings (assignment and declaration initializer) move together, at 8, 12, 32
/// and 64 bits, and with a parameterized dimension.
#[test]
fn multi_packed_target_masks_at_its_product_width() {
    folds(
        "    bit [1:0][3:0] tt;\n    tt = 4'd13 ** 4'd2;\n    g = tt;",
        "",
        169,
    );
    folds(
        "    bit [1:0][3:0] tt;\n    tt = 8'd100 * 8'd100;\n    g = tt;",
        "",
        16,
    );
    folds(
        "    bit [1:0][3:0] tt;\n    tt = 4'd15 ** 4'd3;\n    g = tt;",
        "",
        47,
    );
    folds(
        "    bit [2:0][3:0] tt;\n    tt = 16'd5000 + 16'd1;\n    g = tt;",
        "",
        905,
    );
    folds(
        "    bit [1:0][3:0] tt = 4'd13 ** 4'd2;\n    g = tt;",
        "",
        169,
    );
    folds(
        "    bit [1:0][3:0] tt = 8'd100 * 8'd100;\n    g = tt;",
        "",
        16,
    );
    // 64 bits: the mask is the identity there, which is the unlimited behavior —
    // but the CONTEXT is 64, so the operands stop masking at their own 8.
    folds(
        "    bit [7:0][7:0] tt;\n    tt = 8'd200 + 8'd100;\n    g = tt;",
        "",
        300,
    );
    folds(
        "    bit [3:0][7:0] tt;\n    tt = 100000 * 100000;\n    g = tt;",
        "",
        1410065408,
    );
    folds(
        "    bit [PW-1:0][3:0] tt;\n    tt = 8'd100 * 8'd100;\n    g = tt;",
        "  parameter integer PW = 2;",
        16,
    );
    // THREE dimensions. With only two, "multiply just the FIRST extra dim" and
    // "decline more than one extra dim" are both byte-identical, so neither half
    // of the product rule is pinned without this. The DECLARATION-INITIALIZER
    // spelling is the discriminator for the second one: a declined shape reaches
    // the assignment funnel as `None`, whose final coercion is the identity.
    folds(
        "    bit [1:0][1:0][3:0] tt;\n    tt = 4'd13 ** 4'd2;\n    g = tt;",
        "",
        169,
    );
    folds(
        "    bit [1:0][1:0][3:0] tt;\n    tt = 8'd100 * 8'd100;\n    g = tt;",
        "",
        10000,
    );
    folds(
        "    bit [1:0][1:0][3:0] tt = 16'd50000 + 16'd50000;\n    g = tt;",
        "",
        34464,
    );
    // The extra dimensions repeat the FIRST one's recursion guard: a bound that
    // calls the function being folded must decline, not re-enter it through
    // `const_eval_in_scope` (which restarts the call depth) and overflow the
    // stack. Exit 0 with the pre-existing value is the whole assertion.
    folds(
        "    bit [1:0][g()-1:0] tt;\n    tt = 3;\n    g = tt;",
        "",
        3,
    );
}

/// Byte-identity neighbours: a single-packed target, the plain integer widths,
/// the signed/shift context rules, a loop, the `return` spelling, and the formal
/// paths (an explicit argument, a narrow formal, a default) are all untouched.
#[test]
fn single_packed_and_formal_paths_unchanged() {
    folds(
        "    bit [7:0] tt;\n    tt = 8'd100 * 8'd100;\n    g = tt;",
        "",
        16,
    );
    folds(
        "    bit [3:0] t;\n    t = 4'd15 + 4'd15;\n    g = t;",
        "",
        14,
    );
    folds(
        "    int r;\n    r = 100000 * 100000;\n    g = r;",
        "",
        1410065408,
    );
    folds(
        "    byte b;\n    b = 100;\n    g = (b * 8'sd1) > 50;",
        "",
        1,
    );
    folds(
        "    bit [7:0] t;\n    t = (8'sd100 + 8'sd100) >> 1;\n    g = t;",
        "",
        100,
    );
    folds(
        "    int i; int s;\n    s = 0;\n    for (i = 0; i < 5; i = i + 1) s = s + i;\n    g = s;",
        "",
        10,
    );
    folds("    return (4'd15 + 4'd15) >> 1;", "", 15);
    let (out, code, err) = run_raw(
        "module top;\n\
         function integer f(input int k = 8'(3)); f = k + 1; endfunction\n\
         function integer h(input bit [3:0] a); h = a; endfunction\n\
         localparam integer L = f();\n\
         localparam integer M = h(4'd15 + 1);\n\
         initial begin $display(\"R=%0d %0d\", L, M); #1 $finish; end\n\
         endmodule\n",
    );
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("R=4 0"), "formal paths unchanged; got:\n{out}");
}
