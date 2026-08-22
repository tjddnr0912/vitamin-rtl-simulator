//! Structural-delay VALUE folding beyond literals — `assign #(D) y = a;`,
//! `wire #(D) w = a;`, `buf #(D) g(y,a);` (IEEE §6.1.3 / §7.14 / §28.16).
//!
//! The delay itself (and its inertial cancellation) was already correct; the
//! VALUE was not. `fold_ca_delay` folded through a literal-only helper
//! (`IntLit` / `(…)` / unary ±, then `_ => None`) and the caller consumes `None`
//! as a SILENT default — no delay. So `parameter D = 7; assign #(D) y = a;`
//! propagated at t=1 instead of t=8 with `errors=0`, and so did `#(2+3)`,
//! `#($clog2(32))`, `#(5ns)` and a `parameter real`.
//!
//! Every expected value below was measured live on BOTH oracles (iverilog 13.0
//! and verilator 5.050), and they agree on all of them EXCEPT the two named at
//! their own test: the `bufif1` row (verilator refuses to compile a tristate
//! primitive here — iverilog-only), and the `#0`-vs-no-delay ORDERING, where the
//! oracles split by one inactive hop. The zero-rise test is therefore pinned in
//! the postponed region (`$strobe`), where they do agree.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_delayfold_{}_{n}", std::process::id()));
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

/// `a` rises at t=1; sample `y` every unit and report the first unit at which it
/// reads 1 (`RISE=-1` ⇒ it never did inside the window). One line per design, so
/// the assertion is on the DELAY, not on formatting.
fn rise(ts: &str, decls: &str, body: &str, window: u32) -> (String, Option<i32>) {
    run(&format!(
        "`timescale {ts}\nmodule top;\n  reg a; wire y;\n{decls}{body}\
         \n  integer i; integer tf;\n\
         \x20 initial begin\n\
         \x20   tf = -1; a = 1'b0; #1 a = 1'b1;\n\
         \x20   for (i = 0; i < {window}; i = i + 1) begin\n\
         \x20     #1; if (tf < 0 && y === 1'b1) tf = $time;\n\
         \x20   end\n\
         \x20   $display(\"RISE=%0d\", tf); $finish;\n\
         \x20 end\nendmodule\n"
    ))
}

fn assert_rise(ts: &str, decls: &str, body: &str, want: i64, why: &str) {
    let (out, code) = rise(ts, decls, body, 300);
    assert_eq!(code, Some(0), "{why}: nonzero exit; got:\n{out}");
    assert!(
        out.contains(&format!("RISE={want}")),
        "{why}: want RISE={want}; got:\n{out}"
    );
}

#[test]
fn parameter_delay_is_not_dropped() {
    // The reported shape. Both oracles: y rises at t=8 (1 + 7).
    assert_rise(
        "1ns/1ns",
        "  parameter D = 7;\n",
        "  assign #(D) y = a;\n",
        8,
        "`assign #(D)` with D=7",
    );
}

#[test]
fn constant_expression_delay_is_not_dropped() {
    // Not "parameters are unsupported" — a literal-only fold, so `2+3` failed too.
    assert_rise(
        "1ns/1ns",
        "",
        "  assign #(2+3) y = a;\n",
        6,
        "`assign #(2+3)`",
    );
}

#[test]
fn delay_reaches_every_scope_and_const_form() {
    // One funnel, many spellings of "5": localparam, param arithmetic, an
    // instance override, a package parameter, `$clog2`, a size cast, a constant
    // function, a specparam, a generate-scope localparam, a ternary and a shift.
    for (decls, body, why) in [
        ("  localparam D = 5;\n", "  assign #(D) y = a;\n", "localparam"),
        ("  parameter D = 3;\n", "  assign #(2*D-1) y = a;\n", "param arithmetic"),
        ("", "  assign #($clog2(32)) y = a;\n", "$clog2"),
        ("  parameter D = 5;\n", "  assign #(8'(D)) y = a;\n", "size cast"),
        (
            "  function integer f(input integer x); f = x + 1; endfunction\n",
            "  assign #(f(4)) y = a;\n",
            "constant function",
        ),
        ("  specparam SD = 5;\n", "  assign #(SD) y = a;\n", "specparam"),
        ("  parameter D = 5;\n", "  assign #(D > 3 ? D : 1) y = a;\n", "ternary"),
        ("  parameter D = 1;\n", "  assign #(D << 2) y = a;\n", "shift (=4)"),
        (
            "",
            "  generate if (1) begin : g\n    localparam D = 5;\n    assign #(D) y = a;\n  end endgenerate\n",
            "generate-scope localparam",
        ),
    ] {
        let want = if why == "shift (=4)" { 5 } else { 6 };
        assert_rise("1ns/1ns", decls, body, want, why);
    }
}

#[test]
fn package_parameter_and_instance_override_delays() {
    let (out, code) = run(
        "`timescale 1ns/1ns\npackage pk; parameter PD = 5; endpackage\n\
         module top; reg a; wire y;\n  assign #(pk::PD) y = a;\n\
         initial begin a=0; #1 a=1; #3 $display(\"A y=%b\", y);\n\
         #3 $display(\"B y=%b\", y); $finish; end endmodule\n",
    );
    assert_eq!(code, Some(0));
    assert!(
        out.contains("A y=x"),
        "pkg param delay not yet landed; got:\n{out}"
    );
    assert!(out.contains("B y=1"), "pkg param delay landed; got:\n{out}");

    let (out, code) = run(
        "`timescale 1ns/1ns\nmodule sub #(parameter D = 1) (input a, output y);\n\
         assign #(D) y = a; endmodule\n\
         module top; reg a; wire y; sub #(.D(5)) u(a, y);\n\
         initial begin a=0; #1 a=1; #3 $display(\"A y=%b\", y);\n\
         #3 $display(\"B y=%b\", y); $finish; end endmodule\n",
    );
    assert_eq!(code, Some(0));
    assert!(
        out.contains("A y=x"),
        "override delay not yet landed; got:\n{out}"
    );
    assert!(out.contains("B y=1"), "override delay landed; got:\n{out}");
}

#[test]
fn delay_is_self_determined_and_read_unsigned() {
    // ⭐ The rule the oracles pin, and the reason this shares `$clog2`'s helper
    // rather than the width-unlimited `const_eval_in_scope`:
    //   * `4'd15 + 4'd1` wraps to a 4-bit 0 ⇒ NO delay (unlimited would say 16),
    //   * an 8-bit signed −1 reads as the unsigned pattern 255 (unlimited would
    //     say 4294967295 ⇒ never), and
    //   * widening one operand to 32 bits brings 16 back.
    assert_rise(
        "1ns/1ns",
        "",
        "  assign #(4'd15 + 4'd1) y = a;\n",
        2,
        "4-bit wrap ⇒ 0",
    );
    assert_rise(
        "1ns/1ns",
        "  parameter [3:0] W = 15;\n",
        "  assign #(W + 4'd1) y = a;\n",
        2,
        "4-bit wrap through a param ⇒ 0",
    );
    assert_rise(
        "1ns/1ns",
        "  parameter [3:0] W = 15;\n",
        "  assign #(W + 1) y = a;\n",
        17,
        "unsized 32-bit sibling ⇒ 16",
    );
    let (out, code) = rise(
        "1ns/1ns",
        "  parameter signed [7:0] D = -8'sd1;\n",
        "  assign #(D) y = a;\n",
        280,
    );
    assert_eq!(code, Some(0));
    assert!(out.contains("RISE=256"), "8-bit −1 reads 255; got:\n{out}");
}

#[test]
fn unsized_negative_and_oversized_delays_match_their_literal_twin() {
    // An UNSIZED negative delay is the 32-bit all-ones pattern — both oracles
    // never fire it, and so does the literal spelling `#(-1)` that already
    // folded. Same for a value past the tick field: SATURATE (a wrap here is a
    // silent EARLY fire).
    //
    // ⚠️ UNSIZED is the whole claim. A SIZED negative literal (`#(-4'd1)`) never
    // reaches the new rule — `const_delay_ticks` answers first and its
    // `const_eval_u32` does a 32-bit `wrapping_neg` — so it still never fires
    // where both oracles delay 15, while its param twin
    // (`parameter [3:0] Q = -4'd1`) now correctly delays 15. That split is
    // PRE-identical (this slice did not create it) and is ROADMAP §2's.
    for (decls, body, why) in [
        (
            "  parameter D = 1;\n",
            "  assign #(-D) y = a;\n",
            "negative param",
        ),
        ("", "  assign #(-1) y = a;\n", "negative literal twin"),
        (
            "  parameter D = 5000000000;\n",
            "  assign #(D) y = a;\n",
            "> u32 param",
        ),
        ("", "  assign #5000000000 y = a;\n", "> u32 literal twin"),
    ] {
        assert_rise("1ns/1ns", decls, body, -1, why);
    }
}

#[test]
fn real_parameter_and_real_arithmetic_delays() {
    // The real branch was literal-only too. Under `1ns/100ps` a 2.5-unit delay is
    // resolvable; both oracles rise at 3.5 units (a rises at 1.0).
    let (out, code) = run(
        "`timescale 1ns/100ps\nmodule top;\n  parameter real RD = 2.5;\n\
         reg a; wire y1, y2, y3;\n\
         assign #(RD) y1 = a;\n  assign #(RD+1.0) y2 = a;\n  assign #(2.5+1.0) y3 = a;\n\
         initial begin a=0; #1 a=1;\n\
         #2.4 $display(\"A %b%b%b\", y1, y2, y3);\n\
         #0.2 $display(\"B %b%b%b\", y1, y2, y3);\n\
         #1.0 $display(\"C %b%b%b\", y1, y2, y3);\n\
         $finish; end endmodule\n",
    );
    assert_eq!(code, Some(0), "got:\n{out}");
    assert!(out.contains("A xxx"), "nothing landed at 3.4; got:\n{out}");
    assert!(out.contains("B 1xx"), "RD=2.5 landed at 3.5; got:\n{out}");
    assert!(
        out.contains("C 111"),
        "the 3.5-unit pair landed at 4.5; got:\n{out}"
    );
}

#[test]
fn an_integral_delay_expression_stays_in_the_integer_domain() {
    // The real domain is asked FIRST when the expression mentions a real, so a
    // real-free `#(D/2)` with D=11 never reaches it and stays integer division (5),
    // not 5.5. Its real twin `#(RD/2.0)` IS 5.5.
    assert_rise(
        "1ns/1ns",
        "  parameter D = 11;\n",
        "  assign #(D/2) y = a;\n",
        6,
        "integer division",
    );
    // ⚠️⚠️ AND THE DISCRIMINATOR the first draft of this slice failed: an
    // exactly-integral `parameter real` keeps an i64 TWIN, so asking the integer
    // domain first finds it and folds INTEGER division — 5 where both oracles
    // (and vita's own procedural `#(RD/2)`) say 5.5 ⇒ 6. `RD = 11.0` with
    // `#(RD/2.0)` cannot catch that: a `.0` initializer registers no twin.
    assert_rise(
        "1ns/1ns",
        "  parameter real RD = 11;\n",
        "  assign #(RD/2) y = a;\n",
        7,
        "integral real param keeps the REAL domain",
    );
    let (out, code) = run(
        "`timescale 1ns/100ps\nmodule top;\n  parameter real RD = 11.0;\n\
         reg a; wire y;\n  assign #(RD/2.0) y = a;\n\
         initial begin a=0; #1 a=1; #5.4 $display(\"A y=%b\", y);\n\
         #0.2 $display(\"B y=%b\", y); $finish; end endmodule\n",
    );
    assert_eq!(code, Some(0), "got:\n{out}");
    assert!(out.contains("A y=x"), "5.5 not landed at 6.4; got:\n{out}");
    assert!(out.contains("B y=1"), "5.5 landed at 6.6; got:\n{out}");
}

#[test]
fn time_literal_delays_scale_once() {
    // `const_eval_in_scope`'s `TimeLit` arm already answers in MODULE units, and
    // the delay path multiplies by the module's time multiplier — so this must
    // scale ONCE. Both spellings and both timescales give a 5-unit / 50 ns delay.
    assert_rise(
        "1ns/100ps",
        "",
        "  assign #(5ns) y = a;\n",
        6,
        "#(5ns) @1ns",
    );
    assert_rise(
        "10ns/1ns",
        "",
        "  assign #(50ns) y = a;\n",
        6,
        "#(50ns) @10ns",
    );
    assert_rise(
        "1ns/100ps",
        "  parameter T = 5ns;\n",
        "  assign #(T) y = a;\n",
        6,
        "param 5ns",
    );
    assert_rise(
        "10ns/1ns",
        "  parameter D = 5;\n",
        "  assign #(D) y = a;\n",
        6,
        "param @10ns",
    );
}

#[test]
fn every_structural_delay_construct_shares_the_funnel() {
    // The parser desugars gate primitives and net-decl delays into the same
    // `ContinuousAssign`, so one fold serves all of them.
    for (decls, body, why) in [
        (
            "  parameter D = 5;\n",
            "  wire #(D) z = a;\n  assign y = z;\n",
            "net-decl",
        ),
        ("  parameter D = 5;\n", "  buf #(D) g(y, a);\n", "buf"),
        ("  parameter D = 5;\n", "  and #(D) g(y, a, 1'b1);\n", "and"),
        ("  parameter D = 5;\n", "  or #(D) g(y, a, 1'b0);\n", "or"),
        ("  parameter D = 5;\n", "  xor #(D) g(y, a, 1'b0);\n", "xor"),
        (
            "  parameter D = 5;\n",
            "  bufif1 #(D) g(y, a, 1'b1);\n",
            "bufif1",
        ),
    ] {
        assert_rise("1ns/1ns", decls, body, 6, why);
    }
}

#[test]
fn distinct_rise_fall_from_parameters_reaches_the_sidecar() {
    // `#(D,F)` with D=3, F=9 — a fall that differs from the rise rides the
    // `ca_delays` sidecar, which only exists when values[0] folds.
    let (out, code) = run(
        "`timescale 1ns/1ns\nmodule top;\n  parameter D = 3; parameter F = 9;\n\
         reg a; wire y;\n  assign #(D,F) y = a;\n\
         initial begin a=0; #20 a=1; #4 $display(\"A y=%b\", y);\n\
         a=0; #5 $display(\"B y=%b\", y);\n\
         #6 $display(\"C y=%b\", y); $finish; end endmodule\n",
    );
    assert_eq!(code, Some(0), "got:\n{out}");
    assert!(out.contains("A y=1"), "rise(3) landed; got:\n{out}");
    assert!(out.contains("B y=1"), "fall(9) not yet at +5; got:\n{out}");
    assert!(out.contains("C y=0"), "fall(9) landed; got:\n{out}");
}

#[test]
fn a_zero_rise_keeps_the_pre_slice_shape() {
    // ⚠️ ANTI-TRUNCATION PIN, not a claim that this is IEEE-correct. A
    // scope-folded rise of 0 stays `None` (no delay) rather than `Some(0)`,
    // because `Some(0)` routes the assign onto the delayed path where a
    // zero-tick write lands only AFTER the Postponed region of its own time
    // step. Pinned with `$strobe` (which IS the postponed region) because that
    // is where both oracles agree: they read all three of these as 1, and a
    // `Some(0)` reads 0. `y0` — the LITERAL `#0` — is that pre-existing lag,
    // pinned here so it cannot spread; ROADMAP §2 owns it and it is the root
    // that would unblock `#(ZERO_PARAM, F)` below.
    let (out, code) = run("`timescale 1ns/1ns\nmodule top;\n  parameter Z = 0;\n\
         reg a; wire y0, yz, yn;\n\
         assign #0   y0 = a;\n  assign #(Z) yz = a;\n  assign      yn = a;\n\
         initial begin a=0; #1 a=1;\n\
         $strobe(\"S y0=%b yz=%b yn=%b\", y0, yz, yn);\n\
         #2 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "got:\n{out}");
    assert!(
        out.contains("S y0=0 yz=1 yn=1"),
        "`#(Z)` must stay on the no-delay shape (both oracles read 1 here) while \
         the literal `#0` keeps its pre-existing postponed-region lag; got:\n{out}"
    );
}

#[test]
fn a_zero_rise_with_a_distinct_fall_is_a_recorded_residue() {
    // ⚠️ THE LOSING HALF OF THAT TRADE, pinned so it is not mistaken for support.
    // The zero-rise rule suppresses the uniform, and the engine only consults the
    // rise/fall sidecar on `delay.is_some()` — so `#(Z,F)` keeps the pre-slice NO
    // delay and its fall is wrong, while the literal twin `#(0,F)` is right. Both
    // oracles fall at +9 on BOTH spellings. Emitting `Some(0)` + sidecar would fix
    // this fall and break the rise on the lag above, and both halves are
    // 2-oracle-agreed — a silent-wrong traded for a silent-wrong, which the
    // accuracy ladder forbids. ROADMAP §2 owns it; the fix is the lag, not here.
    let (out, code) = run(
        "`timescale 1ns/1ns\nmodule top;\n  parameter Z = 0; parameter F = 9;\n\
         reg a; wire yl, yp;\n  assign #(0,9) yl = a;\n  assign #(Z,F) yp = a;\n\
         initial begin a=0; #10 a=1; #2 $display(\"A yl=%b yp=%b\", yl, yp);\n\
         a=0; #4 $display(\"B yl=%b yp=%b\", yl, yp);\n\
         #8 $display(\"C yl=%b yp=%b\", yl, yp); $finish; end endmodule\n",
    );
    assert_eq!(code, Some(0), "got:\n{out}");
    assert!(out.contains("A yl=1 yp=1"), "both risen; got:\n{out}");
    assert!(
        out.contains("B yl=1 yp=0"),
        "literal `#(0,9)` holds for fall(9) — both oracles; the param spelling \
         falls immediately = the recorded residue; got:\n{out}"
    );
    assert!(out.contains("C yl=0 yp=0"), "both fallen; got:\n{out}");
}

#[test]
fn a_zero_fall_still_reaches_the_sidecar() {
    // The zero rule is scoped to values[0]: a FALL of 0 is a real distinct edge
    // delay and rides the sidecar, so `#(D,Z)` matches its literal twin `#(5,0)`.
    let (out, code) = run(
        "`timescale 1ns/1ns\nmodule top;\n  parameter D = 5; parameter Z = 0;\n\
         reg a; wire yl, yp;\n  assign #(5,0) yl = a;\n  assign #(D,Z) yp = a;\n\
         initial begin a=0; #10 a=1; #6 $display(\"A yl=%b yp=%b\", yl, yp);\n\
         a=0; #1 $display(\"B yl=%b yp=%b\", yl, yp); $finish; end endmodule\n",
    );
    assert_eq!(code, Some(0), "got:\n{out}");
    assert!(
        out.contains("A yl=1 yp=1"),
        "rise(5) landed on both; got:\n{out}"
    );
    assert!(
        out.contains("B yl=0 yp=0"),
        "fall(0) landed on both; got:\n{out}"
    );
}

#[test]
fn a_runtime_variable_delay_is_still_zero_delay_and_still_quiet() {
    // ⚠️ THE OTHER HALF, pinned so closing the const gap does not silently start
    // loud-rejecting it: `assign #(dv) y = a;` over a VARIABLE is not a constant
    // and keeps the pre-slice zero-delay behavior at exit 0. Both oracles delay
    // by dv — recorded in ROADMAP §2 as a separate (runtime) axis.
    let (out, code) = rise(
        "1ns/1ns",
        "  reg [7:0] dv;\n",
        "  assign #(dv) y = a;\n  initial dv = 8'd5;\n",
        60,
    );
    assert_eq!(code, Some(0), "must not become loud; got:\n{out}");
    assert!(out.contains("RISE=2"), "still zero-delay; got:\n{out}");
}
