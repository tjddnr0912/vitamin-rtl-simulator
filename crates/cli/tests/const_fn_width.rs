//! The §4.5.186 constant-function interpreter evaluated every body expression in
//! the width-UNLIMITED i64 domain, so a narrow assignment TARGET never truncated.
//! `function int f(); bit [3:0] t; t = 4'd15 + 4'd15; return t; endfunction` folded
//! to 30 where SystemVerilog gives 14 — which meant `localparam W = f()` was a
//! silently wrong PARAMETER value at exit 0 (the P0-5 class), and every width /
//! range / case context derived from it inherited the error.
//!
//! The decisive signal was internal: vita's own RUNTIME already executed those
//! functions correctly, so the interpreter disagreed with the engine about the
//! same source. iverilog agrees with the runtime. A 3-way sweep over 11 target
//! types × 15 right-hand sides had 84 of 165 param values wrong before; all 165
//! match iverilog now.
//!
//! The rule (IEEE 1800 §11.6 / Table 11-21): an assignment evaluates its RHS at
//! `max(self-determined width of the RHS, the target's width)`; the operands of a
//! context-determined operator all widen to that width; the result is then
//! assigned to the target. Self-determined positions — a shift's COUNT, a `**`
//! exponent, a ternary's condition, a comparison's operands, a system-function
//! argument — are sized by themselves and never by the surrounding context.
//! Every value below is pinned to LIVE iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_cfw_{}_{n}", std::process::id()));
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

/// Build a design where each `(target type, rhs)` pair is folded at ELABORATE
/// time into a localparam AND executed at RUNTIME by the same function, then
/// printed side by side. The equivalence "interpreter == engine" needs no oracle
/// to be meaningful, and it is exactly the invariant that was broken.
fn interp_vs_runtime(cases: &[(&str, &str)]) -> String {
    let mut decls = String::new();
    let mut prints = String::new();
    for (i, (ty, rhs)) in cases.iter().enumerate() {
        decls.push_str(&format!(
            "  function automatic int f{i}(); {ty} t; t = {rhs}; return t; endfunction\n  \
             localparam L{i} = f{i}();\n"
        ));
        prints.push_str(&format!("    $display(\"c{i}=%0d/%0d\", L{i}, f{i}());\n"));
    }
    format!("module m;\n{decls}  initial begin\n{prints}    #1 $finish;\n  end\nendmodule\n")
}

/// Every line prints `elaborate/runtime`; the two halves must agree, and each
/// must equal the value iverilog prints.
fn check(out: &str, want: &[&str], ctx: &str) {
    let got: Vec<&str> = out
        .lines()
        .filter(|l| l.starts_with('c') && l.contains('='))
        .collect();
    assert!(
        !got.is_empty(),
        "{ctx}: NO output at all (silent drop); got:\n{out}"
    );
    assert_eq!(got.len(), want.len(), "{ctx}: line count; got:\n{out}");
    for (line, w) in got.iter().zip(want) {
        let v = line.split('=').nth(1).unwrap_or("");
        let (a, b) = v.split_once('/').unwrap_or((v, ""));
        assert_eq!(a, b, "{ctx}: interpreter != runtime on `{line}`\n{out}");
        assert_eq!(&a, w, "{ctx}: expected iverilog's `{w}` on `{line}`\n{out}");
    }
}

/// The core rule: the RHS is evaluated at max(self-width, target width).
#[test]
fn assignment_target_width_governs_the_rhs() {
    let cases = [
        ("bit [3:0]", "4'd15 + 4'd15"),     // 4-bit context  -> 30 wraps to 14
        ("int", "4'd15 + 4'd15"),           // 32-bit context -> 30
        ("byte", "(8'd200 + 8'd100) >> 2"), // 8-bit  -> 300 wraps to 44, >>2 = 11
        ("int", "(8'd200 + 8'd100) >> 2"),  // 32-bit -> 300 >> 2 = 75
        ("bit [3:0]", "4'd8 * 4'd3"),       // 24 wraps to 8
        ("bit [7:0]", "4'd8 * 4'd3"),       // 24 fits
        ("bit [3:0]", "20"),                // a wide literal narrows at the assignment
        ("bit [3:0]", "4'd15 - 4'd1"),      // 14, no wrap
    ];
    let (out, c) = run(&interp_vs_runtime(&cases));
    assert_eq!(c, Some(0), "no diagnostics expected; got:\n{out}");
    check(
        &out,
        &["14", "30", "11", "75", "8", "24", "4", "14"],
        "assignment context width",
    );
}

/// Signedness decides whether the narrowing sign-extends (§11.8.1).
#[test]
fn narrowing_sign_extends_only_for_a_signed_target() {
    let cases = [
        ("byte", "8'sd100 + 8'sd100"),           // signed 8-bit: 200 -> -56
        ("bit [7:0]", "8'sd100 + 8'sd100"),      // unsigned 8-bit: 200
        ("bit signed [3:0]", "4'd7 + 4'd1"),     // signed 4-bit: 8 -> -8
        ("bit [3:0]", "4'd7 + 4'd1"),            // unsigned 4-bit: 8
        ("shortint", "16'sd30000 + 16'sd30000"), // signed 16-bit wrap
    ];
    let (out, c) = run(&interp_vs_runtime(&cases));
    assert_eq!(c, Some(0), "no diagnostics expected; got:\n{out}");
    check(
        &out,
        &["-56", "200", "-8", "8", "-5536"],
        "signed vs unsigned narrowing",
    );
}

/// A self-determined operand is sized by itself, whatever the target is: the
/// shift COUNT, the `**` exponent, the ternary CONDITION, and a system-function
/// ARGUMENT. `$clog2(4'd15 + 4'd15)` is `$clog2(14)` = 4, never `$clog2(30)` = 5
/// — including when the target is 64 bits wide, where nothing is masked at all
/// but the argument still sizes itself.
#[test]
fn self_determined_operands_ignore_the_target_width() {
    let cases = [
        ("int", "$clog2(4'd15 + 4'd15)"),
        ("longint", "$clog2(4'd15 + 4'd15)"),
        ("bit [3:0]", "$clog2(4'd15 + 4'd15)"),
        ("int", "8'd1 << (4'd2 + 4'd1)"),
        ("int", "(4'd3 ? 4'd15 + 4'd15 : 4'd1)"),
        ("bit [3:0]", "(4'd15 + 4'd15) > 20"),
    ];
    let (out, c) = run(&interp_vs_runtime(&cases));
    assert_eq!(c, Some(0), "no diagnostics expected; got:\n{out}");
    // The comparison sizes its operands against EACH OTHER, not against the
    // target: the 32-bit `20` widens the add to 32 bits, so it is 30 > 20 = 1.
    check(
        &out,
        &["4", "4", "4", "8", "30", "1"],
        "self-determined operands",
    );
}

/// The width env must reach every binding the interpreter makes — a formal, a
/// nested-block declaration, a `for` init, and a chained assignment — or that one
/// name assigns unmasked.
#[test]
fn every_binding_carries_its_declared_width() {
    let (out, c) = run("module m;\n\
           function automatic int viaformal(input bit [3:0] a); bit [3:0] t; t = a + a; \
             return t; endfunction\n\
           function automatic int vianested(); begin : inner bit [3:0] t; t = 4'd15 + 4'd15; \
             vianested = t; end endfunction\n\
           function automatic int viachain(); bit [3:0] t; int u; t = 4'd15 + 4'd15; u = t + 1; \
             return u; endfunction\n\
           function automatic int vialoop(); int s; bit [3:0] i; s = 0;\n\
             for (i = 4'd14; i < 4'd15; i = i + 4'd1) s = s + 1; return s; endfunction\n\
           localparam A = viaformal(4'd15), B = vianested(), C = viachain(), D = vialoop();\n\
           initial begin\n\
             $display(\"c0=%0d/%0d\", A, viaformal(4'd15));\n\
             $display(\"c1=%0d/%0d\", B, vianested());\n\
             $display(\"c2=%0d/%0d\", C, viachain());\n\
             $display(\"c3=%0d/%0d\", D, vialoop());\n\
             #1 $finish; end endmodule\n");
    assert_eq!(c, Some(0), "no diagnostics expected; got:\n{out}");
    check(&out, &["14", "14", "15", "1"], "every binding");
}

/// An untyped parameter initialized by a call takes the function's declared
/// RETURN type — width and sign. Widening only the VALUE predicate left the other
/// two behind, and a folded −56 materialized as 4294967240 at 32 unsigned bits.
#[test]
fn param_from_a_call_takes_the_return_type() {
    let (out, c) = run("module m;\n\
           function automatic int fi(); fi = -56; endfunction\n\
           function automatic byte fb(); fb = -56; endfunction\n\
           function automatic bit [3:0] fu(); fu = 4'd9; endfunction\n\
           localparam LI = fi(), LB = fb(), LU = fu();\n\
           localparam byte TY = fi();\n\
           initial begin\n\
             $display(\"V=%0d %0d %0d\", LI, LB, LU);\n\
             $display(\"B=%0d %0d %0d\", $bits(LI), $bits(LB), $bits(LU));\n\
             $display(\"S=%b %b %b\", LI < 0, LB < 0, LU < 0);\n\
             $display(\"D=%0d %0d\", $bits(TY), TY);\n\
             #1 $finish; end endmodule\n");
    assert_eq!(c, Some(0));
    assert!(out.contains("V=-56 -56 9"), "values; got:\n{out}");
    assert!(
        out.contains("B=32 8 4"),
        "declared return widths; got:\n{out}"
    );
    assert!(
        out.contains("S=1 1 0"),
        "declared return signs; got:\n{out}"
    );
    // An explicit DECLARED type on the parameter still wins over the return type.
    assert!(out.contains("D=8 -56"), "declared type wins; got:\n{out}");
}

/// §11.8.1: the signedness of a context-determined expression is decided ONCE for
/// the whole context — if ANY operand is unsigned, every operand is reinterpreted
/// as unsigned. Deciding it per node sign-extended a signed sub-expression under
/// an unsigned parent and turned a CORRECT `(b+b)/u` into 228.
#[test]
fn signedness_is_resolved_once_for_the_whole_context() {
    let (out, c) = run("module m;\n\
           function automatic int mix(); byte b; bit [7:0] u; bit [7:0] r;\n\
             b = 100; u = 2; r = (b + b) / u; return r; endfunction\n\
           function automatic int lit(); byte t; t = (8'sd100 + 8'sd100) / 8'd2; \
             return t; endfunction\n\
           function automatic int mod(); byte t; t = (8'sd100 + 8'sd100) % 8'd7; \
             return t; endfunction\n\
           function automatic int shr(); bit [7:0] t; t = (8'sd100 + 8'sd100) >> 1; \
             return t; endfunction\n\
           localparam A = mix(), B = lit(), C = mod(), D = shr();\n\
           initial begin\n\
             $display(\"c0=%0d/%0d\", A, mix());\n\
             $display(\"c1=%0d/%0d\", B, lit());\n\
             $display(\"c2=%0d/%0d\", C, mod());\n\
             $display(\"c3=%0d/%0d\", D, shr());\n\
             #1 $finish; end endmodule\n");
    assert_eq!(c, Some(0), "must not become loud either; got:\n{out}");
    check(
        &out,
        &["100", "100", "4", "100"],
        "one signedness per context",
    );
}

/// Findings from the adversarial review, each pinned so the fix cannot regress:
/// a 63-bit target (the shared coercion overflowed `1i64 << 63` and PANICKED), an
/// `int unsigned` local (the atom kinds were hard-coded signed), a multi-packed
/// local (only the first packed dim is known, so the width must DECLINE rather
/// than mask to it), and a parameter-sized range (the commonest RTL spelling,
/// which the literal-only range reader could not see).
#[test]
fn review_findings_stay_fixed() {
    let (out, c) = run("module m; parameter PW = 4;\n\
           function automatic integer w63(); bit [62:0] t; t = 1; return t; endfunction\n\
           function automatic longint uns(); int unsigned u; u = 32'hFFFF_FFFF; \
             return u; endfunction\n\
           function automatic int mdp(); logic [3:0][7:0] mm; mm = 32'h1234_5678; \
             return mm; endfunction\n\
           function automatic int pw(); bit [PW-1:0] t; t = 4'd15 + 4'd15; return t; endfunction\n\
           localparam A = w63(), B = uns(), C = mdp(), D = pw();\n\
           initial begin\n\
             $display(\"c0=%0d/%0d\", A, w63());\n\
             $display(\"c1=%0d/%0d\", B, uns());\n\
             $display(\"c2=%0d/%0d\", C, mdp());\n\
             $display(\"c3=%0d/%0d\", D, pw());\n\
             #1 $finish; end endmodule\n");
    assert_eq!(c, Some(0), "width 63 must not panic; got:\n{out}");
    check(
        &out,
        &["1", "4294967295", "305419896", "14"],
        "review findings",
    );
}

/// `return e` and `f = e` are the same assignment and must agree — they disagreed
/// on the identical expression until `return` was given the declared return type
/// as its context. And a declared range that CALLS the function being interpreted
/// must not fold at all: `const_eval_in_scope` restarts the call depth, so folding
/// it recursed until the stack overflowed.
#[test]
fn return_matches_assignment_and_a_self_sized_range_declines() {
    let (out, c) = run("module m;\n\
           function automatic bit [3:0] viaret(); return (4'd15 + 4'd15) >> 1; endfunction\n\
           function automatic bit [3:0] viaasg(); bit [3:0] r; r = (4'd15 + 4'd15) >> 1; \
             return r; endfunction\n\
           function automatic bit [3:0] vianame(); vianame = (4'd15 + 4'd15) >> 1; endfunction\n\
           localparam A = viaret(), B = viaasg(), C = vianame();\n\
           initial begin\n\
             $display(\"c0=%0d/%0d\", A, viaret());\n\
             $display(\"c1=%0d/%0d\", B, viaasg());\n\
             $display(\"c2=%0d/%0d\", C, vianame());\n\
             #1 $finish; end endmodule\n");
    assert_eq!(c, Some(0));
    check(&out, &["7", "7", "7"], "return == assignment");

    // A range bound calling its own function: must stay the old non-folding
    // behavior (value 1), NEVER a stack overflow.
    let (out, c) = run(
        "module m; function automatic int f(); bit [f()-1:0] tt; return 1; endfunction\n\
         localparam int P = f(); initial begin $display(\"S=%0d\", P); #1 $finish; end endmodule\n",
    );
    assert_eq!(c, Some(0), "self-sized range must not crash; got:\n{out}");
    assert!(
        out.contains("S=1"),
        "declines to fold, no recursion; got:\n{out}"
    );
}

/// §11.6.1: an operand enters a context at ITS OWN declared width, sign-extended
/// only when the operand is signed AND the whole expression is signed — otherwise
/// ZERO-extended. The env stores a narrow signed local already sign-extended into
/// i64, so a `byte a = -100` reached a 32-bit UNSIGNED context as 0xFFFF_FF9C
/// instead of 0x0000_009C and flipped a comparison.
#[test]
fn a_narrow_signed_leaf_zero_extends_into_an_unsigned_context() {
    let (out, c) = run("module m;\n\
           localparam bit [31:0] LIMIT = 32'd200;\n\
           function automatic int over(input byte a); \
             if ((a * 8'sd1) > LIMIT) return 1; else return 0; endfunction\n\
           function automatic int shf(input byte a); int t; t = (a + 32'd0) >> 1; \
             return t; endfunction\n\
           function automatic int mo(); byte b; bit [7:0] u; b = -100; u = 7; \
             return b % u; endfunction\n\
           localparam A = over(-8'sd100), B = shf(-8'sd100), C = mo();\n\
           initial begin\n\
             $display(\"c0=%0d/%0d\", A, over(-8'sd100));\n\
             $display(\"c1=%0d/%0d\", B, shf(-8'sd100));\n\
             $display(\"c2=%0d/%0d\", C, mo());\n\
             #1 $finish; end endmodule\n");
    assert_eq!(c, Some(0), "the shift must fold, not go loud; got:\n{out}");
    check(
        &out,
        &["0", "78", "2"],
        "narrow signed leaf in unsigned context",
    );
}

/// A declaration whose shape the interpreter CANNOT determine must mean UNKNOWN —
/// propagating "no masking" — not a guessed 32-bit unsigned. Guessing truncated a
/// 64-bit multi-packed local and mis-signed a call-ranged one. (Truth here is
/// vita's own runtime: iverilog's *elaborate-time* evaluator is itself wrong on
/// the signed case, where it disagrees with its own runtime.)
#[test]
fn an_undeterminable_declaration_means_unknown_not_32_bit_unsigned() {
    let (out, c) = run("module m;\n\
           function automatic int w8(); return 8; endfunction\n\
           function automatic int wide(); logic [1:0][31:0] v; v = 64'h1_0000_0000;\n\
             if ((v + 0) > 32'hFFFF_FFFF) return 1; else return 0; endfunction\n\
           function automatic int callrange(); logic signed [w8()-1:0] v; int t;\n\
             v = -100; t = (v + 0) / 2; return t; endfunction\n\
           localparam A = wide(), B = callrange();\n\
           initial begin\n\
             $display(\"c0=%0d/%0d\", A, wide());\n\
             $display(\"c1=%0d/%0d\", B, callrange());\n\
             #1 $finish; end endmodule\n");
    assert_eq!(c, Some(0));
    check(&out, &["1", "-50"], "unknown declaration shape");
}

/// An all-`int` const function is the overwhelmingly common shape and must be
/// completely untouched by the width machinery — including recursion and loops.
#[test]
fn wide_const_functions_are_unaffected() {
    let (out, c) = run("module m;\n\
           function automatic int fact(input int n); \
             if (n <= 1) return 1; else return n * fact(n - 1); endfunction\n\
           function automatic int summ(input int n); int s; int i; s = 0;\n\
             for (i = 0; i <= n; i = i + 1) s = s + i; return s; endfunction\n\
           function automatic int cl(input int n); return $clog2(n) + 1; endfunction\n\
           localparam F = fact(6), S = summ(10), C = cl(1000);\n\
           logic [cl(1000)-1:0] wide;\n\
           initial begin wide = '1;\n\
             $display(\"c0=%0d/%0d\", F, fact(6));\n\
             $display(\"c1=%0d/%0d\", S, summ(10));\n\
             $display(\"c2=%0d/%0d\", C, cl(1000));\n\
             $display(\"WB=%0d\", $bits(wide));\n\
             #1 $finish; end endmodule\n");
    assert_eq!(c, Some(0), "no diagnostics expected; got:\n{out}");
    check(&out, &["720", "55", "11"], "wide const functions");
    assert!(
        out.contains("WB=11"),
        "call-derived range bound; got:\n{out}"
    );
}
