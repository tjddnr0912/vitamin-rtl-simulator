//! REAL-valued parameters and localparams — `localparam real R = 1.5`.
//!
//! Every form was loud ("parameter `R` value is not a foldable constant expression")
//! even for a plain literal, while a real VARIABLE (`real r = 1.5;`) already worked.
//! `parameter real` is a common RTL idiom (clock periods, scaling constants), and
//! iverilog supports all of these forms, so this is a straight oracle-backed gap.
//!
//! Mechanism mirrors the existing STRING-parameter precedent exactly: a real has no
//! i64 value, so it is kept out of `params` and held in a `real_param_raw` side map
//! as its RAW literal text, then folded on read to the same const the literal itself
//! would produce. The read rides the same innermost-wins re-derivation over the
//! COMBINED binding set that the string path uses — an independent walk over the side
//! map alone would match an OUTER real param even when an inner net or numeric param
//! shadows it, resolving one name two different ways.
//!
//! Deliberately still loud (correct-or-loud — a wrong parameter value poisons every
//! downstream width with no trace): real ARITHMETIC in the initializer
//! (`2.0 + 3.0`, no real const-fold path exists), a real PACKAGE parameter (its own
//! import-const machinery), and OVERRIDING a real parameter (the override machinery
//! is i64-only, so the declared default would be used silently).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_raw(src: &str) -> (String, bool, String) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_rparam_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    let so = String::from_utf8_lossy(&out.stdout).into_owned();
    let mut s = String::new();
    for l in so.lines().filter(|l| !l.starts_with("simulation ended")) {
        s.push_str(l);
        s.push('\n');
    }
    (
        s,
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn run(src: &str) -> String {
    let (out, ok, err) = run_raw(src);
    assert!(ok, "expected success, stderr:\n{err}");
    out
}

fn loud(src: &str, needle: &str) {
    let (_, ok, err) = run_raw(src);
    assert!(!ok, "expected a loud reject");
    assert!(err.contains(needle), "unexpected diagnostic:\n{err}");
}

#[test]
fn localparam_real_literal() {
    let out = run("module t;\n\
           localparam real R = 1.5;\n\
           initial $display(\"%0.1f\", R);\n\
         endmodule\n");
    assert_eq!(out, "1.5\n");
}

#[test]
fn body_parameter_real() {
    let out = run("module t;\n\
           parameter real R = 2.25;\n\
           initial $display(\"%0.2f\", R);\n\
         endmodule\n");
    assert_eq!(out, "2.25\n");
}

#[test]
fn header_parameter_real() {
    let out = run("module t #(parameter real R = 1.5) ();\n\
           initial $display(\"%0.1f\", R);\n\
         endmodule\n");
    assert_eq!(out, "1.5\n");
}

#[test]
fn untyped_localparam_with_a_real_value() {
    // IEEE §6.20.2: the type follows the value, so this is a real parameter.
    let out = run("module t;\n\
           localparam R = 1.5;\n\
           initial $display(\"%0.1f\", R);\n\
         endmodule\n");
    assert_eq!(out, "1.5\n");
}

#[test]
fn negative_and_scientific_literals() {
    let out = run("module t;\n\
           localparam real N = -1.5;\n\
           localparam real S = 1.5e3;\n\
           initial $display(\"%0.1f %0.1f\", N, S);\n\
         endmodule\n");
    assert_eq!(out, "-1.5 1500.0\n");
}

#[test]
fn real_param_in_a_runtime_expression() {
    let out = run("module t;\n\
           localparam real R = 1.5;\n\
           real x;\n\
           initial begin x = R * 2.0; $display(\"%0.1f\", x); end\n\
         endmodule\n");
    assert_eq!(out, "3.0\n");
}

#[test]
fn real_param_matches_the_same_literal_written_inline() {
    // The vita-internal equivalence: reading the param must fold to exactly what the
    // literal itself folds to.
    let out = run("module t;\n\
           localparam real R = 2.5;\n\
           real a, b;\n\
           initial begin\n\
             a = R; b = 2.5;\n\
             $display(\"%0.1f %0.1f %0d\", a, b, a == b);\n\
           end\n\
         endmodule\n");
    assert_eq!(out, "2.5 2.5 1\n");
}

#[test]
fn inner_binding_shadows_an_outer_real_param() {
    // The read must re-derive the innermost binding over the COMBINED set. Resolving
    // the side map independently would return the outer 1.5 inside the function.
    let out = run("module t;\n\
           localparam real R = 1.5;\n\
           function automatic real f();\n\
             real R;\n\
             R = 9.5;\n\
             return R;\n\
           endfunction\n\
           initial $display(\"%0.1f %0.1f\", R, f());\n\
         endmodule\n");
    assert_eq!(out, "1.5 9.5\n");
}

#[test]
fn per_instance_real_parameter_defaults() {
    let out = run("module m #(parameter real R = 1.5) ();\n\
           initial $display(\"%0.1f\", R);\n\
         endmodule\n\
         module t;\n\
           m a();\n\
           m b();\n\
           initial #1 $finish;\n\
         endmodule\n");
    assert_eq!(out, "1.5\n1.5\n");
}

// ── integer parameters must be completely unaffected ─────────────────────────

#[test]
fn integer_parameters_and_overrides_unaffected() {
    let out = run("module m #(parameter int W = 4) ();\n\
           initial $display(\"%0d\", W);\n\
         endmodule\n\
         module t;\n\
           m #(.W(9)) u();\n\
           localparam int L = 3;\n\
           initial begin #1 $display(\"%0d\", L); $finish; end\n\
         endmodule\n");
    assert_eq!(out, "9\n3\n");
}

// ── shapes that must stay loud ───────────────────────────────────────────────

#[test]
fn real_arithmetic_initializer_is_loud() {
    // No real const-fold path exists; a wrong parameter value would poison every
    // downstream use with no trace, so this stays loud rather than guessing.
    loud(
        "module t;\n\
           localparam real R = 2.0 + 3.0;\n\
           initial $display(\"%0.1f\", R);\n\
         endmodule\n",
        "not a foldable constant expression",
    );
}

#[test]
fn overriding_a_real_parameter_is_loud() {
    // The override machinery is i64-only. Before this slice the real param decl was
    // itself loud so the design never ran; without an explicit reject it would now
    // run with the DECLARED DEFAULT (1.5) where iverilog gives 2.5 — wrong output at
    // exit 0. Reject instead.
    loud(
        "module m #(parameter real R = 1.5) ();\n\
           initial $display(\"%0.1f\", R);\n\
         endmodule\n\
         module t;\n\
           m #(.R(2.5)) u();\n\
           initial #1 $finish;\n\
         endmodule\n",
        "with a real value is unsupported",
    );
}

#[test]
fn package_real_parameter_is_loud() {
    // Package constants travel through their own import machinery
    // (`pkg_consts`/`apply_import_consts`), which is i64-only — a separate follow-on.
    loud(
        "package pk;\n\
           localparam real R = 2.5;\n\
         endpackage\n\
         module t;\n\
           import pk::*;\n\
           initial $display(\"%0.1f\", R);\n\
         endmodule\n",
        "not a foldable constant",
    );
}

#[test]
fn real_param_in_a_replication_count_is_loud() {
    // A real param is deliberately kept out of `params`, so a constant-required
    // context without its own loud gate folded it to a silent 0: `{int'(R){1'b1}}`
    // with R=2.0 printed `0` instead of iverilog's `11`. Before this slice the decl
    // itself was loud, so this was loud -> silent-wrong.
    loud(
        "module t;\n\
           localparam real R = 2.0;\n\
           initial $display(\"%b\", {int'(R){1'b1}});\n\
         endmodule\n",
        "replication count that reads a real parameter",
    );
}

#[test]
fn integer_replication_counts_unaffected() {
    let out = run("module t;\n\
           localparam int N = 2;\n\
           initial $display(\"%b\", {N{1'b1}});\n\
         endmodule\n");
    assert_eq!(out, "11\n");
}

#[test]
fn real_param_in_an_integer_constant_context_is_loud() {
    // A real param has no integral value. Two wrong answers were tried before this
    // one: folding to None left `$clog2(R)` with NO diagnostic and a silent 1-bit
    // width, and converting at the const-eval LEAF destroyed the real value before
    // the enclosing expression chose its context — `if (R > 2)` with R=2.4 took the
    // wrong generate branch, `R == 2` folded TRUE, and `localparam real B = A;`
    // silently rounded. Loud here, converted nowhere. iverilog does convert, so this
    // is a recorded capability gap, not a correctness one.
    for body in [
        "logic [R-1:0] x; initial begin x=0; $display(\"%0d\", $bits(x)); end",
        "logic [$clog2(R)-1:0] y; initial begin y=0; $display(\"%0d\", $bits(y)); end",
    ] {
        loud(
            &format!("module t;\n  localparam real R = 4.0;\n  {body}\nendmodule\n"),
            "real parameter is not an integral constant",
        );
    }
}

#[test]
fn a_real_param_never_folds_into_the_integer_domain() {
    // The generate-condition / comparison / real-alias shapes that the leaf
    // conversion got wrong. Each must be loud, never a silently rounded answer.
    for src in [
        "module t;\n  parameter real R = 2.4;\n  localparam int A = (R == 2);\n           initial $display(\"%0d\", A);\nendmodule\n",
        "module t;\n  localparam real A = 1.5;\n  localparam real B = A;\n           initial $display(\"%0.2f\", B);\nendmodule\n",
        "module t;\n  localparam real CLK = 5.0;\n  localparam real HALF = CLK/2;\n           initial $display(\"%0.2f\", HALF);\nendmodule\n",
    ] {
        let (_, ok, _) = run_raw(src);
        assert!(!ok, "expected a loud reject for:\n{src}");
    }
}

#[test]
fn same_named_real_params_in_different_modules_do_not_bleed() {
    // The side map is FQ-keyed and the read re-derives the FQ key from the current
    // prefix, so distinct instances get distinct entries without the save/restore
    // that `params` needs.
    let out = run("module a;\n\
           localparam real R = 1.5;\n\
           initial $display(\"a=%0.1f\", R);\n\
         endmodule\n\
         module b;\n\
           localparam int R = 7;\n\
           initial $display(\"b=%0d\", R);\n\
         endmodule\n\
         module t;\n\
           a ia();\n\
           b ib();\n\
           initial #1 $finish;\n\
         endmodule\n");
    assert_eq!(out, "a=1.5\nb=7\n");
}

#[test]
fn real_param_survives_real_math_and_formats() {
    let out = run("module t;\n\
           localparam real R = 2.25;\n\
           initial $display(\"%0.2f %e %g %0d\", R, R, R,\n\
                            $realtobits(R) == $realtobits(2.25));\n\
         endmodule\n");
    assert_eq!(out, "2.25 2.250000e+00 2.25 1\n");
}

#[test]
fn nested_sign_on_a_real_literal() {
    // Negating the raw TEXT built "--1.25", which parses to a silent 0.0. The value is
    // negated instead, so nesting works.
    let out = run("module t;\n\
           localparam real A = -(-1.25);\n\
           localparam real B = -(-(-4.5));\n\
           initial $display(\"%0.2f %0.1f\", A, B);\n\
         endmodule\n");
    assert_eq!(out, "1.25 -4.5\n");
}

#[test]
fn integer_literal_initializing_a_real_param() {
    // Keyed on the DECLARED type, not the initializer's literal form: `parameter real
    // R = 3` is a real parameter, so `R/2` is 1.5 and `$realtobits` is the f64 pattern.
    // Keying on the literal made it bind as an i64 param that integer-divided to 1.0.
    let out = run("module t;\n\
           parameter real R = 3;\n\
           initial $display(\"%0.1f %h\", R/2, $realtobits(R));\n\
         endmodule\n");
    assert_eq!(out, "1.5 4008000000000000\n");
}

#[test]
fn real_select_index_is_loud() {
    // IEEE §11.5.1 wants an integral index. A real one folded to 0, so `v[R]` read the
    // wrong bit, a real part-select bound produced a multi-megabit X, and a real lvalue
    // index silently DROPPED the write. One wrapper guards every index site.
    for body in [
        "$display(\"%b\", v[R]);",
        "$display(\"%b\", v[T:R]);",
        "$display(\"%b\", v[R+:2]);",
        "v[R] = 1'b1;",
    ] {
        loud(
            &format!(
                "module t;\n\
                   parameter real R = 1.0, T = 2.0;\n\
                   logic [7:0] v = 8'b0000_0110;\n\
                   initial begin {body} end\n\
                 endmodule\n"
            ),
            "select index / bound must be integral",
        );
    }
}

#[test]
fn integer_selects_are_unaffected() {
    let out = run("module t;\n\
           localparam int I = 1;\n\
           logic [7:0] v = 8'b0000_0110;\n\
           int q[];\n\
           initial begin\n\
             q = new[3]; q[1] = 7;\n\
             $display(\"%b %b %b %0d\", v[I], v[2:1], v[I+:2], q[I]);\n\
             v[I] = 1'b0; $display(\"%b\", v);\n\
           end\n\
         endmodule\n");
    assert_eq!(out, "1 11 11 7\n00000100\n");
}

#[test]
fn unfoldable_override_of_a_real_target_is_loud() {
    // Not just a real LITERAL override: any override that fails to fold against a
    // real-typed target must reject, or the design runs with the declared default —
    // the wrong value at exit 0, where before this slice the decl itself was loud.
    for ovr in ["1.5 + 0.5", "sig"] {
        loud(
            &format!(
                "module m #(parameter real R = 1.5) ();\n\
                   initial $display(\"R=%0.2f\", R);\n\
                 endmodule\n\
                 module t;\n\
                   logic [3:0] sig;\n\
                   m #(.R({ovr})) u();\n\
                   initial begin sig = 1; #1 $finish; end\n\
                 endmodule\n"
            ),
            "the override of real parameter",
        );
    }
}

#[test]
fn positional_real_override_is_loud() {
    loud(
        "module m #(parameter real R = 1.5) ();\n\
           initial $display(\"%0.1f\", R);\n\
         endmodule\n\
         module t;\n\
           m #(2.5) u();\n\
           initial #1 $finish;\n\
         endmodule\n",
        "overriding a parameter with a real value",
    );
}

#[test]
fn empty_named_override_still_keeps_the_default() {
    // `.W()` legally means "keep the default" and must NOT be mistaken for an override
    // that failed to fold.
    let out = run("module m #(parameter int W = 5) ();\n\
           initial $display(\"W=%0d\", W);\n\
         endmodule\n\
         module t;\n\
           m #(.W()) u();\n\
           initial #1 $finish;\n\
         endmodule\n");
    assert_eq!(out, "W=5\n");
}

#[test]
fn non_real_unfoldable_override_keeps_warn_and_default() {
    // The escalation must not swallow the pre-existing warn-and-default behaviour for
    // ordinary non-constant overrides of an integer param.
    let out = run("module m #(parameter int W = 5) ();\n\
           initial $display(\"W=%0d\", W);\n\
         endmodule\n\
         module t;\n\
           logic [3:0] sig;\n\
           m #(.W(sig)) u();\n\
           initial begin sig = 1; #1 $finish; end\n\
         endmodule\n");
    assert_eq!(out, "W=5\n");
}

#[test]
fn an_exact_integer_real_param_keeps_every_integral_capability() {
    // `parameter real R = 4;` const-folds to an exact i64, so it is registered in BOTH
    // `real_param_val` and `params`: the two representations agree. Binding it real-ONLY
    // took eight byte-correct designs loud — a descent down the accuracy ladder, since
    // every one of these worked before real params were supported at all.
    for (body, want) in [
        (
            "logic [R-1:0] v; initial begin v=5; $display(\"%0d %0d\", $bits(v), v); end",
            "4 5",
        ),
        (
            "logic [7:0] arr [R]; initial $display(\"%0d\", $size(arr));",
            "4",
        ),
        ("localparam int W = R; initial $display(\"%0d\", W);", "4"),
        (
            "generate if (R > 2) begin initial $display(\"gt\"); end \
             else begin initial $display(\"le\"); end endgenerate",
            "gt",
        ),
    ] {
        let src = format!("module t;\n  parameter real R = 4;\n  {body}\nendmodule\n");
        let (out, ok, _) = run_raw(&src);
        assert!(ok, "expected support, got a reject for:\n{src}");
        assert!(out.contains(want), "want {want:?} in {out:?}\n{src}");
    }
    // …and it still divides in the REAL domain, which is why it is bound real at all.
    let (out, ok, _) = run_raw(
        "module t;\n  parameter real R = 3;\n  initial $display(\"%0.4f\", R/2);\nendmodule\n",
    );
    assert!(ok);
    assert!(out.contains("1.5000"), "R/2 must be 1.5, got {out:?}");
}

#[test]
fn an_override_that_folds_applies_to_a_real_formal() {
    // `#(.R(i+2))` on a real formal is legal and the oracle answers it; rejecting it
    // was a false-loud. Only an override that does NOT fold stays unsupported.
    let (out, ok, _) = run_raw(
        "module sub #(parameter real R = 1.5) ();\n  initial $display(\"R=%0.2f\", R);\n\
         endmodule\nmodule t;\n  genvar i;\n  generate for (i=0;i<2;i=i+1) begin : g\n    \
         sub #(.R(i+2)) u();\n  end endgenerate\nendmodule\n",
    );
    assert!(ok, "folded override must apply, got:\n{out}");
    assert!(
        out.contains("R=2.00") && out.contains("R=3.00"),
        "got {out:?}"
    );
}

#[test]
fn a_real_param_reached_through_a_const_function_call_is_loud() {
    // `count_reads_real_param` had no `Call` arm and `nonconst_bound_reason` does not
    // descend into call args either, so a real param smuggled through a const function
    // hit NEITHER net: `logic [f(R)-1:0]` folded to None and silently became 1 bit.
    let pre = "module t;\n  parameter real R = 8.0;\n  \
               function automatic int f(input int x); return x; endfunction\n  ";
    for body in [
        "logic [f(R)-1:0] v; initial begin v=5; $display(\"%0d\", $bits(v)); end",
        "logic [7:0] arr [f(R)]; initial $display(\"%0d\", $size(arr));",
        "initial $display(\"%b\", {f(R){1'b1}});",
    ] {
        let src = format!("{pre}{body}\nendmodule\n");
        let (_, ok, _) = run_raw(&src);
        assert!(!ok, "expected a loud reject for:\n{src}");
    }
}

#[test]
fn a_real_valued_override_of_an_integral_formal_is_loud() {
    // The sibling guard tested a real LITERAL; this slice newly made real-VALUED
    // override expressions reachable, and those fell into warn-and-keep-default —
    // the child silently ran with the wrong parameter (and the wrong port width).
    for ovr in ["#(R)", "#(.W(R))", "#(.W(R+1))"] {
        let src = format!(
            "module sub #(parameter W = 8) (output logic [31:0] o);\n  assign o = W;\n\
             endmodule\nmodule t;\n  parameter real R = 4.5;\n  logic [31:0] o;\n  \
             sub {ovr} u(o);\n  initial #1 $display(\"W=%0d\", o);\nendmodule\n"
        );
        let (_, ok, _) = run_raw(&src);
        assert!(!ok, "expected a loud reject for:\n{src}");
    }
    // …but an EXACT integer twin still folds and applies, matching the oracle.
    let (out, ok, _) = run_raw(
        "module sub #(parameter W = 8) (output logic [31:0] o);\n  assign o = W;\nendmodule\n\
         module t;\n  parameter real R = 4;\n  logic [31:0] o;\n  sub #(.W(R)) u(o);\n  \
         initial #1 $display(\"W=%0d\", o);\nendmodule\n",
    );
    assert!(ok, "exact integer twin must still apply");
    assert!(out.contains("W=4"), "got {out:?}");
}

#[test]
fn a_lowered_real_count_or_width_cannot_reach_the_ir() {
    // `lower_expr` PREFERS `real_param_val` over `params`, so a real param with an
    // exact i64 twin still lowers to a real Const. A gate keyed on the const-FOLD
    // resolver said "integral" and waved it through: `{R{1'b1}}` emitted 2^24 bits
    // and `v[0 +: R]` 2^24 x-bits, both at exit 0. Same predicate, two resolvers —
    // each consumer must ask the one matching how IT resolves the name.
    for body in [
        "initial $display(\"%b\", {R{1'b1}});",
        "logic [15:0] x; initial begin x = {R{1'b1}}; $display(\"%h\", x); end",
        "logic [31:0] v; initial begin v='1; $display(\"%b\", v[0 +: R]); end",
        "logic [31:0] v; initial begin v='1; $display(\"%b\", v[15 -: R]); end",
        "logic [31:0] v; initial begin v=0; v[0 +: R] = '1; $display(\"%h\", v); end",
        "string s = \"abc\"; initial $display(\"%c\", s[R]);",
    ] {
        let src = format!("module t;\n  parameter real R = 4;\n  {body}\nendmodule\n");
        let (_, ok, _) = run_raw(&src);
        assert!(!ok, "expected a loud reject for:\n{src}");
    }
}
