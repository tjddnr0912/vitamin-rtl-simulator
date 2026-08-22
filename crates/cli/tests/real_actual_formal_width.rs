//! A REAL actual bound to an INTEGRAL subroutine formal (IEEE 1800 §13.5.3).
//!
//! §13.5.3 makes a call an ASSIGNMENT to a variable of the formal's DECLARED
//! type, so a real actual ROUNDS (§6.24.1, half away from zero) and then NARROWS
//! to the formal's width. vita rounded but did not narrow: the inline function
//! path substitutes the actual's ExprId for the formal's NAME (no net exists to
//! coerce at) and the frame paths handed the raw value to the slot in-bind, so
//! the body read the real at ITS OWN width. `function integer f(input byte k);
//! f = k;` called `f(300.0)` gave 300 where both oracles give 44.
//!
//! ⚠️ The INTEGER-actual twins were already correct everywhere, and so was the
//! inline TASK path — it copies each input actual into a formal-WIDTH local net,
//! so the store coerces. That path is the reference this slice made the other
//! three match.
//!
//! Every expected value was measured live on iverilog 13.0 and verilator 5.050,
//! which agree on all of them except where noted.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_realformal_{}_{n}", std::process::id()));
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

/// `function <lifetime> integer f(input <formal> k); f = k;` called with `actual`.
fn call_fn(lifetime: &str, formal: &str, actual: &str) -> (String, Option<i32>) {
    run(&format!(
        "module top;\n  function {lifetime} integer f(input {formal} k); f = k; endfunction\n\
         \x20 initial begin $display(\"R=%0d\", f({actual})); $finish; end\nendmodule\n"
    ))
}

fn assert_call(lifetime: &str, formal: &str, actual: &str, want: i64) {
    let (out, code) = call_fn(lifetime, formal, actual);
    assert_eq!(
        code,
        Some(0),
        "`{formal}` ← `{actual}`: nonzero exit;\n{out}"
    );
    assert!(
        out.contains(&format!("R={want}")),
        "`{formal}` ← `{actual}`: want R={want};\n{out}"
    );
}

#[test]
fn a_real_actual_narrows_to_the_formal_width_inline() {
    // The reported shape and its neighbours, on the INLINE (static) function path.
    assert_call("", "byte", "300.0", 44);
    assert_call("", "byte", "-300.0", -44);
    assert_call("", "byte unsigned", "300.0", 44);
    assert_call("", "byte unsigned", "-300.0", 212);
    assert_call("", "bit [3:0]", "300.0", 12);
    assert_call("", "shortint", "1e10", -7168);
    assert_call("", "logic [7:0]", "-1.5", 254);
    assert_call("", "logic signed [7:0]", "300.0", 44);
    assert_call("", "reg [15:0]", "-300.0", 65236);
    assert_call("", "int", "1e10", 1410065408);
    assert_call("", "longint", "1e10", 1410065408);
}

#[test]
fn a_real_actual_narrows_on_the_frame_function_path_too() {
    // ⚠️ `automatic` routes to a DIFFERENT emitter, and the first probe of this
    // slice used `input int` with `3.0` — a value that FITS — so the frame path
    // looked correct. Every formal narrower than the value is where it shows.
    for (formal, want) in [
        ("byte", 44),
        ("byte unsigned", 44),
        ("bit [3:0]", 12),
        ("logic [7:0]", 44),
        ("logic signed [7:0]", 44),
        ("reg [15:0]", 300),
    ] {
        assert_call("automatic", formal, "300.0", want);
    }
    assert_call("automatic", "byte", "-300.0", -44);
    assert_call("automatic", "shortint", "1e10", -7168);
}

#[test]
fn a_real_actual_narrows_on_the_frame_task_path_too() {
    // The THIRD binding site. A STATIC task was already correct (it copies the
    // actual into a formal-width local net); an `automatic` one goes through the
    // frame slot in-bind and was not.
    for (formal, actual, want) in [
        ("byte", "300.0", 44),
        ("byte", "-300.0", -44),
        ("bit [3:0]", "300.0", 12),
    ] {
        let (out, code) = run(&format!(
            "module top;\n  task automatic tt(input {formal} k, output integer o); o = k; endtask\n\
             \x20 integer oo;\n\
             \x20 initial begin tt({actual}, oo); $display(\"R=%0d\", oo); $finish; end\nendmodule\n"
        ));
        assert_eq!(code, Some(0), "task `{formal}` ← `{actual}`;\n{out}");
        assert!(out.contains(&format!("R={want}")), "want R={want};\n{out}");
    }
}

#[test]
fn the_static_task_path_was_already_right_and_stays_right() {
    // The reference implementation — pinned so a future change to the three
    // binds cannot quietly drag it along.
    for (formal, actual, want) in [
        ("byte", "300.0", 44),
        ("byte unsigned", "-300.0", 212),
        ("bit [3:0]", "300.0", 12),
        ("shortint", "1e10", -7168),
    ] {
        let (out, code) = run(&format!(
            "module top;\n  task tt(input {formal} k, output integer o); o = k; endtask\n\
             \x20 integer oo;\n\
             \x20 initial begin tt({actual}, oo); $display(\"R=%0d\", oo); $finish; end\nendmodule\n"
        ));
        assert_eq!(code, Some(0), "static task `{formal}` ← `{actual}`;\n{out}");
        assert!(out.contains(&format!("R={want}")), "want R={want};\n{out}");
    }
}

#[test]
fn every_real_actual_shape_reaches_the_coercion() {
    // Not just a literal: a real variable, a real parameter, a real-returning
    // call, a negation and a ternary all bind through the same rule.
    let (out, code) = run("module top;\n  real rv;\n  parameter real RP = 300.0;\n\
         \x20 function real g(); g = 300.0; endfunction\n\
         \x20 function integer f(input byte k); f = k; endfunction\n\
         \x20 initial begin rv = 300.0;\n\
         \x20   $display(\"A=%0d B=%0d C=%0d D=%0d E=%0d\",\n\
         \x20     f(rv), f(RP), f(g()), f(-rv), f(1 ? 300.0 : 1.0));\n\
         \x20   $finish; end\nendmodule\n");
    assert_eq!(code, Some(0), "got:\n{out}");
    assert!(
        out.contains("A=44 B=44 C=44 D=-44 E=44"),
        "variable / param / call / negation / ternary all narrow;\n{out}"
    );
}

#[test]
fn an_integer_actual_and_a_real_formal_are_untouched() {
    // ⚠️ THE OTHER HALF. The coercion is keyed on the ACTUAL being real and the
    // formal being an integral bit vector; an integer actual keeps the existing
    // §11.6.2 path (byte-identical) and a `real` formal keeps its payload.
    assert_call("", "byte", "8'sd44", 44);
    let (out, code) = run(
        "module top;\n  function integer f(input byte k); f = k; endfunction\n\
         \x20 integer m;\n  initial begin m = 300; $display(\"R=%0d\", f(m)); $finish; end\nendmodule\n",
    );
    assert_eq!(code, Some(0), "got:\n{out}");
    assert!(
        out.contains("R=44"),
        "integer actual already narrowed;\n{out}"
    );

    let (out, code) = run(
        "module top;\n  function real f(input real k); f = k * 2.0; endfunction\n\
         \x20 initial begin $display(\"R=%0d\", $rtoi(f(1.25) * 100.0)); $finish; end\nendmodule\n",
    );
    assert_eq!(code, Some(0), "got:\n{out}");
    assert!(
        out.contains("R=250"),
        "real formal keeps its payload;\n{out}"
    );
}

#[test]
fn a_formal_wider_than_the_cast_scope_does_not_become_loud() {
    // ⚠️ `lower_real_to_int_cast` REPORTS above 64 bits, so the coercion declines
    // there rather than turning a wrong value into a new diagnostic. This design
    // is refused for a pre-existing reason (unchanged by this slice); what is
    // pinned is that the exit code did not move.
    let (out, code) = run(
        "module top;\n  function integer f(input logic [127:0] k); f = k[7:0]; endfunction\n\
         \x20 initial begin $display(\"R=%0d\", f(300.0)); $finish; end\nendmodule\n",
    );
    assert_eq!(code, Some(1), "pre-existing refusal, not a new one;\n{out}");
}

#[test]
fn a_non_repeatable_real_actual_keeps_the_pre_slice_answer() {
    // ⚠️ THE DELIBERATE DECLINE. The real→int cast names its operand about six
    // times, so applying it to a `$random`-bearing actual would DRAW MORE THAN
    // ONCE — a different wrong answer, not a right one. Such an actual keeps the
    // pre-slice (un-narrowed) value; the residue is ROADMAP §2's.
    let (out, code) = run(
        "module top;\n  function integer f(input byte k); f = k; endfunction\n\
         \x20 integer i;\n\
         \x20 initial begin for (i=0;i<2;i=i+1) $display(\"R=%0d\", f($random * 1.0));\n\
         \x20 $finish; end\nendmodule\n",
    );
    assert_eq!(code, Some(0), "must stay quiet;\n{out}");
    assert!(
        out.contains("R=303379748") && out.contains("R=-1064739199"),
        "one draw per call, un-narrowed (pre-slice);\n{out}"
    );
}

#[test]
fn the_round_is_exact_at_a_half_ulp_tie() {
    // ⚠️ ADVERSARIAL FIND, and the reason this slice touched the cast itself.
    // `lower_real_to_int_cast` rounded a ≤32-bit target as `$rtoi(e + 0.5)`, but
    // for an ODD integer with |e| in [2^52, 2^53) the f64 ulp is 1.0, so that add
    // is a TIE and IEEE-754 breaks it to EVEN — the answer came back `e + 1`. The
    // > 32-bit branch already computed the exact `trunc + (frac ≥ ½ ? ±1 : 0)`
    // form and said so in its own comment; the narrow branch did not. Routing
    // every ≤32-bit FORMAL through it made that reachable from a call, so both
    // branches now share the exact construction — which also repairs the
    // pre-existing `int'()` / `byte'()` spelling.
    let (out, code) = run("module top;\n\
         \x20 function automatic longint ff(input byte k); ff = k; endfunction\n\
         \x20 task tt(input byte k, output longint o); o = k; endtask\n\
         \x20 longint oo; real rv;\n\
         \x20 initial begin\n\
         \x20   tt(4503599627370497.0, oo); rv = 4503599627370497.0;\n\
         \x20   $display(\"FN=%0d TASK=%0d CAST=%0d\", ff(4503599627370497.0), oo, byte'(rv));\n\
         \x20   $finish; end\nendmodule\n");
    assert_eq!(code, Some(0), "got:\n{out}");
    assert!(
        out.contains("FN=1 TASK=1 CAST=1"),
        "2^52+1 rounds to itself, so a byte formal holds 1 — and the three \
         spellings agree (both oracles);\n{out}"
    );

    // ⚠️ THE OTHER HALF: ordinary halves still round AWAY from zero, not to even.
    let (out, code) = run("module top;\n  real rv;\n  initial begin\n\
         \x20   rv = 3.5;  $display(\"D=%0d\", int'(rv));\n\
         \x20   rv = -3.5; $display(\"E=%0d\", int'(rv));\n\
         \x20   rv = 2.4;  $display(\"F=%0d\", int'(rv));\n\
         \x20   rv = -4503599627370497.0; $display(\"C=%0d\", int'(rv));\n\
         \x20 $finish; end\nendmodule\n");
    assert_eq!(code, Some(0), "got:\n{out}");
    assert!(out.contains("D=4"), "3.5 → 4 (away from zero);\n{out}");
    assert!(out.contains("E=-4"), "-3.5 → -4 (away from zero);\n{out}");
    assert!(out.contains("F=2"), "2.4 → 2;\n{out}");
    assert!(out.contains("C=-1"), "-(2^52+1) → -1 in an int;\n{out}");
}

#[test]
fn a_time_formal_with_an_explicit_signed_qualifier_is_declined() {
    // ⚠️ A DELIBERATE DECLINE, pinned so it is not read as support. `time` is
    // 64-bit UNSIGNED (§6.11.2) and vita's shared `kind_signedness` therefore
    // drops an explicit `signed` qualifier when it shapes the formal NET — so on
    // the paths where the formal IS a net the body reads it unsigned whatever the
    // bind computed (the static-task spelling below shows that, and it is
    // pre-existing). Narrowing the other three would have turned their correct
    // pre-slice answer into the same wrong one, so `time signed` keeps the
    // pre-slice value everywhere. The root — a dropped qualifier — is ROADMAP §2's.
    let (out, code) = run(
        "module top;\n  integer d;\n\
         \x20 function integer f(input time signed k); $display(\"FN=%0d\", k); f = 0; endfunction\n\
         \x20 function automatic integer af(input time signed k); $display(\"AF=%0d\", k); af = 0; endfunction\n\
         \x20 task automatic at(input time signed k); $display(\"AT=%0d\", k); endtask\n\
         \x20 initial begin d = f(-3.7); d = af(-3.7); at(-3.7); $finish; end\nendmodule\n",
    );
    assert_eq!(code, Some(0), "got:\n{out}");
    assert!(
        out.contains("FN=-4") && out.contains("AF=-4") && out.contains("AT=-4"),
        "both oracles read a signed `time` formal as -4; the decline keeps it;\n{out}"
    );

    // A plain `time` formal DOES narrow — loud (E3009) before this slice, 44 now.
    let (out, code) = run(
        "module top;\n  function integer f(input time k); f = k[7:0]; endfunction\n\
         \x20 initial begin $display(\"U=%0d\", f(300.0)); $finish; end\nendmodule\n",
    );
    assert_eq!(code, Some(0), "was E3009 before this slice;\n{out}");
    assert!(out.contains("U=44"), "plain `time` narrows;\n{out}");
}

#[test]
fn a_string_formal_is_never_bit_resized() {
    // ⚠️ The frame-TASK bind gates on the AST kind, spelled exactly as the other
    // two binds spell it. Gating on the formal NET's kind instead is strictly
    // weaker — `String` maps to a `Wire` net — and a `string` INPUT formal was
    // then destroyed by a 1-bit real→int cast (`range_to_dims(String, None)` = 1).
    // iverilog aborts on this shape; verilator is the oracle and agrees with the
    // pre-slice length.
    let (out, code) = run(
        "module top;\n\
         \x20 task automatic ats(input string s); $display(\"AUTO=%0d\", s.len()); endtask\n\
         \x20 task ss(input string s); $display(\"STATIC=%0d\", s.len()); endtask\n\
         \x20 function automatic int fs(input string s); return s.len(); endfunction\n\
         \x20 initial begin ats(3.7); ss(3.7); $display(\"FN=%0d\", fs(3.7)); $finish; end\nendmodule\n",
    );
    assert_eq!(code, Some(0), "got:\n{out}");
    assert!(
        out.contains("AUTO=8") && out.contains("STATIC=8") && out.contains("FN=8"),
        "all three keep the heap payload;\n{out}"
    );
}
