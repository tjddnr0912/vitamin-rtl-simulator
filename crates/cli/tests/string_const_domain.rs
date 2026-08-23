//! The string constant domain was literal-only, so a `?:` over two strings was loud.
//!
//! `elaborate::strings::param_str_literal` matched a `StrLit` and a parenthesised one
//! and nothing else, and every parameter-binding consumer called it. Meanwhile a
//! FULLER resolver — `const_fn::const_str_in_scope`, which also handles a bare name
//! bound in `str_param_raw` and a `pkg::NAME` — was already in the tree with exactly
//! one caller: the string-equality fold. Nobody ever asked it for a VALUE.
//!
//! The workload corpus (§4.5.369) found this as the single axis blocking three of
//! eight third-party designs. `lfsr.v` — instantiated by every one of Alex
//! Forencich's cores — writes
//!
//! ```verilog
//! parameter STYLE_INT = (STYLE == "AUTO") ? "REDUCTION" : STYLE;
//! ```
//!
//! and `servant.v` forwards a string parameter down an instantiation. Neither folded.
//!
//! Every expected value below was measured live on iverilog 13.0 AND verilator 5.050,
//! which agree on all of them.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_strconst_{}_{n}", std::process::id()));
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

/// A module whose body binds `SI` and prints it, instantiated with `inst`.
fn shape(body: &str, inst: &str) -> String {
    format!(
        "module m;\n  {body}\n  initial $display(\"VAL=%0s\", SI);\nendmodule\n\
         module tb; {inst} endmodule\n"
    )
}

fn expect_val(body: &str, inst: &str, want: &str) {
    let (out, code) = run(&shape(body, inst));
    assert_eq!(code, Some(0), "expected exit 0, got {code:?}\n{out}");
    assert!(
        out.contains(&format!("VAL={want}")),
        "expected VAL={want}\n{out}"
    );
}

#[test]
fn ternary_over_two_string_literals_folds() {
    expect_val(
        r#"parameter S="AUTO"; parameter SI = (S=="AUTO") ? "RED" : "BLU";"#,
        r#"m #(.S("AUTO")) u();"#,
        "RED",
    );
}

/// The `lfsr.v` spelling exactly: the false arm is the parameter itself, so the
/// domain has to resolve an Ident as well as a literal.
#[test]
fn ternary_whose_false_arm_is_a_string_parameter_folds() {
    expect_val(
        r#"parameter S="AUTO"; parameter SI = (S=="AUTO") ? "RED" : S;"#,
        r#"m #(.S("AUTO")) u();"#,
        "RED",
    );
}

/// ...and the same source line with the OTHER arm taken, which is what an IP user
/// writes when they pin the style explicitly.
#[test]
fn ternary_taking_the_false_arm_yields_the_parameter() {
    expect_val(
        r#"parameter S="LOOP"; parameter SI = (S=="AUTO") ? "RED" : S;"#,
        r#"m #(.S("LOOP")) u();"#,
        "LOOP",
    );
}

/// A literal condition was refused too, which is what showed the gap was the ternary
/// itself and not the string comparison: `S=="AUTO"` as an integer localparam, a bare
/// `parameter SI="RED"`, and a generate-if string compare were all ALREADY correct.
#[test]
fn ternary_with_a_constant_condition_folds() {
    expect_val(r#"parameter SI = 1 ? "RED" : "BLU";"#, "m u();", "RED");
}

#[test]
fn ternary_with_an_integer_condition_folds() {
    expect_val(
        r#"parameter N=3; parameter SI = (N>2) ? "RED" : "BLU";"#,
        "m #(.N(3)) u();",
        "RED",
    );
}

#[test]
fn nested_ternary_folds() {
    expect_val(
        r#"parameter S="X"; parameter SI = (S=="AUTO") ? "RED" : (S=="X") ? "MID" : "BLU";"#,
        r#"m #(.S("X")) u();"#,
        "MID",
    );
}

#[test]
fn localparam_twin_folds() {
    expect_val(
        r#"parameter S="AUTO"; localparam SI = (S=="AUTO") ? "RED" : "BLU";"#,
        r#"m #(.S("AUTO")) u();"#,
        "RED",
    );
}

/// Plain forwarding, with no ternary anywhere. This is `servant.v`'s shape at the
/// declaration end.
#[test]
fn a_string_parameter_bound_to_another_string_parameter_folds() {
    expect_val(
        r#"parameter S="AUTO"; parameter SI = S;"#,
        r#"m #(.S("AUTO")) u();"#,
        "AUTO",
    );
}

/// `servant.v:93` verbatim in shape: `.RESET_STRATEGY (reset_strategy)`. The override
/// expression resolves in the PARENT scope, which is a different consumer from the
/// declaration binding and was separately literal-only.
#[test]
fn an_instantiation_override_that_forwards_a_string_parameter_folds() {
    let (out, code) = run(r#"module leaf #(parameter RS = "NONE") ();
  initial $display("VAL=%0s", RS);
endmodule
module mid #(parameter reset_strategy = "MINI") ();
  leaf #(.RS(reset_strategy)) l ();
endmodule
module tb; mid #(.reset_strategy("MINI")) u(); endmodule
"#);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("VAL=MINI"), "{out}");
}

#[test]
fn an_instantiation_override_that_is_a_ternary_folds() {
    let (out, code) = run(r#"module leaf #(parameter RS = "NONE") ();
  initial $display("VAL=%0s", RS);
endmodule
module mid #(parameter S = "AUTO") ();
  leaf #(.RS((S=="AUTO") ? "RED" : S)) l ();
endmodule
module tb; mid #(.S("AUTO")) u(); endmodule
"#);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("VAL=RED"), "{out}");
}

/// The whole `lfsr.v` decision, both ways, in one design: a generate-if consuming the
/// folded string picks a different branch per instance.
#[test]
fn a_generate_if_selects_on_the_folded_string() {
    let (out, code) = run(r#"module lf #(parameter STYLE="AUTO") ();
  parameter STYLE_INT = (STYLE=="AUTO") ? "REDUCTION" : STYLE;
  generate if (STYLE_INT=="REDUCTION") begin:g initial $display("VAL=red");
  end else if (STYLE_INT=="LOOP") begin:g2 initial $display("VAL=loop");
  end else begin:g3 initial $display("VAL=other"); end endgenerate
endmodule
module tb; lf #(.STYLE("AUTO")) a(); lf #(.STYLE("LOOP")) b(); endmodule
"#);
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("VAL=red"),
        "the AUTO instance takes REDUCTION\n{out}"
    );
    assert!(
        out.contains("VAL=loop"),
        "the LOOP instance takes LOOP\n{out}"
    );
}

/// ⚠️ The first implementation of the concatenation arm joined the RAW literals, which
/// carry their own quotes — `{"RE","D"}` became the four-character `RE""D` and read as
/// 1092756034 in an integer context. That is loud turning into SILENT-WRONG, the one
/// move the accuracy ladder forbids, and it is why this test asserts the exact text
/// rather than merely that something was printed.
#[test]
fn concatenating_string_literals_does_not_smuggle_their_quotes_through() {
    let (out, code) = run("module m;\n  parameter SI = {\"RE\",\"D\"};\n  \
         initial $display(\"VAL=%0s|\", SI);\nendmodule\nmodule tb; m u(); endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("VAL=RED|"), "expected exactly RED\n{out}");
    assert!(
        !out.contains("\"\""),
        "the raw quotes leaked into the value\n{out}"
    );
}

/// The same defect's integer face: iverilog and verilator both read `{"A","B"}` as
/// 16706 (0x4142). The quote-smuggling version read 1092756034. Untyped, because a
/// declaration that states a width takes the gate below instead.
#[test]
fn a_string_concatenation_read_as_an_integer_is_two_bytes_not_four() {
    let (out, code) = run("module tb; localparam Q = {\"A\",\"B\"};\n  \
         initial $display(\"VAL=%0d/%0d\", Q, $bits(Q));\nendmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("VAL=16706/16"),
        "both oracles agree on 16706 at 16 bits\n{out}"
    );
}

/// ⚠️ The string side map carries NO WIDTH, and the string route runs before the
/// width-carrying numeric and wide paths at every consumer. So a declaration that
/// STATES a width silently loses it: `localparam [95:0] X = {"A","B"}` came out 16
/// bits where both oracles say 96, and `localparam [95:0] Z = 1 ? "AB" : "CD"` came
/// out 16706 where iverilog says 0. Those shapes were LOUD before the widening, so
/// folding them would be loud → silent-wrong. Declining keeps them as loud as they
/// were — and the untyped twin above still folds, because it has no width to lose.
#[test]
fn a_declaration_that_states_a_width_declines_rather_than_lose_it() {
    for decl in [
        "localparam [95:0] X = {\"A\",\"B\"};",
        "localparam [95:0] X = 1 ? \"AB\" : \"CD\";",
        "localparam integer X = {\"A\",\"B\"};",
        "localparam integer X = 1 ? \"A\" : \"B\";",
        // ⚠️ These three cost a second review round. `logic`/`reg`/`bit` with no
        // explicit range are ONE bit, but the parser recorded that only in a
        // `var_kind` it then dropped — `ParamDecl` has no such field, so they were
        // indistinguishable from a genuinely untyped `parameter P` and sailed through
        // a gate spelled `p.range.is_none() && p.ty == Implicit`. Both oracles say 1
        // bit; this folded 16. The parser now supplies the range.
        "localparam bit X = {\"A\",\"B\"};",
        "localparam logic X = 1 ? \"AB\" : \"CD\";",
        "localparam reg X = {\"A\",\"B\"};",
        "localparam byte X = {\"A\",\"B\"};",
        "localparam shortint X = {\"A\",\"B\"};",
        "localparam longint X = {\"A\",\"B\"};",
    ] {
        let (out, code) = run(&format!(
            "module tb;\n  {decl}\n  initial $display(\"VAL=%0d\", X);\nendmodule\n"
        ));
        assert_ne!(
            code,
            Some(0),
            "must stay loud rather than drop the width: {decl}\n{out}"
        );
    }
}

/// ⚠️ `{"ab", ""}` is a BIT concatenation in which `""` is one NUL byte (§5.9 /
/// §11.4.12) — both oracles read it as 0x616200 — but a TEXT join represents that
/// byte as no characters at all and yields 0x6162. The string domain cannot carry an
/// interior NUL, so it declines instead of shortening the value.
#[test]
fn an_empty_operand_in_a_concatenation_stays_loud_rather_than_drop_its_nul_byte() {
    for decl in [
        r#"localparam U = {"ab", ""};"#,
        r#"localparam U = {"", "ab"};"#,
        r#"parameter string P = "ab"; localparam U = {"", P, ""};"#,
    ] {
        let (out, code) = run(&format!(
            "module tb;\n  {decl}\n  initial $display(\"VAL=%0d\", U);\nendmodule\n"
        ));
        assert_ne!(
            code,
            Some(0),
            "an empty operand carries a byte: {decl}\n{out}"
        );
    }
}

/// ⚠️ An untyped `parameter` whose default happens to be a string EXPRESSION is an
/// ordinary numeric parameter, and a numeric override of it is legal — both oracles
/// print 9. Two ways to get this wrong were measured, one after the other: asking the
/// widened resolver in the "is this a string?" escalation made it a false-loud
/// (E3002), and then letting the folded default bind anyway made the override VANISH
/// and the design run at 16706. Both are worse than what was there before.
#[test]
fn a_numeric_override_of_a_string_expression_default_still_applies() {
    for (decl, inst) in [
        (r#"parameter W = {"A","B"}"#, "child #(.W(9)) u();"),
        (r#"parameter W = (1) ? "AB" : "CD""#, "child #(.W(9)) u();"),
        (r#"parameter W = (1) ? "AB" : "CD""#, "child #(9) u();"),
    ] {
        let (out, code) = run(&format!(
            "module child #({decl}) ();\n  initial $display(\"VAL=%0d\", W);\nendmodule\n\
             module tb; {inst} endmodule\n"
        ));
        assert_eq!(code, Some(0), "{decl} / {inst}\n{out}");
        assert!(
            out.contains("VAL=9"),
            "the override must win: {decl} / {inst}\n{out}"
        );
    }
}

/// The override channel has the same width hazard as the declaration, and the same
/// answer: a FOLDED override applies only when the child has no declared width. (A
/// LITERAL override on a widthed child already lost the width before this slice, so
/// that pre-existing behaviour is left exactly as it was.)
#[test]
fn a_folded_override_declines_on_a_child_that_declares_a_width() {
    let (out, code) = run(r#"module leaf #(parameter [95:0] RS = "NONE") ();
  initial $display("VAL=%0d", RS);
endmodule
module mid #(parameter S = "AB") ();
  leaf #(.RS(1 ? "AB" : "CD")) a ();
endmodule
module tb; mid u(); endmodule
"#);
    assert_ne!(
        code,
        Some(0),
        "must stay loud rather than drop the 96-bit width\n{out}"
    );
}

/// Fail-closed: BOTH arms must resolve as strings, not just the taken one. Requiring
/// only the selected arm would make the same source line a string on one override and
/// an integer on another — an expression changing DOMAIN under a parameter value.
#[test]
fn a_ternary_mixing_a_string_and_a_number_stays_loud() {
    let (out, code) = run("module m;\n  parameter SI = 1 ? \"RED\" : 5;\n  \
         initial $display(\"VAL=%0s\", SI);\nendmodule\nmodule tb; m u(); endmodule\n");
    assert_ne!(
        code,
        Some(0),
        "the mixed form must not fold silently\n{out}"
    );
}

/// Pinned residue, NOT a target of this slice: the parameter-declaration fold exists
/// in FOUR copies (`params.rs` is canonical; `instance.rs`, `generate.rs` and
/// `package.rs` are the others), and `package.rs` does not route string or real
/// parameters at all. Three of the four now share the widened resolver; the package
/// copy is a class problem recorded in ROADMAP §3, and patching it where it was found
/// is exactly what §4.5.311 warns against. This test exists so that the day someone
/// absorbs the fourth copy, it turns red and asks to be promoted.
#[test]
fn package_scope_is_still_loud_and_that_is_recorded() {
    let (out, code) = run(
        r#"package P; parameter S = "AUTO"; parameter SI = (S=="AUTO") ? "RED" : S; endpackage
module tb; import P::*; initial $display("VAL=%0s", SI); endmodule
"#,
    );
    assert_ne!(
        code,
        Some(0),
        "package-scope string folding is unimplemented — if this now passes, \
         the fourth copy of the parameter fold was absorbed; move this row to \
         ROADMAP §3 as RESOLVED\n{out}"
    );
}

/// Side-harvest, and the proof that the parser fix above is not merely a gate patch:
/// the dropped `var_kind` was losing the declared width for NUMERIC values too, with
/// no string in sight. `localparam bit N = 8'hFF` read 255 at 8 bits where both
/// oracles say 1 at 1 bit — a pre-existing silent-wrong, closed on the way past.
#[test]
fn an_unranged_bit_logic_or_reg_parameter_is_one_bit() {
    let (out, code) = run(
        "module tb;\n  localparam bit N = 8'hFF;\n  localparam logic M = 8'hFF;\n  \
         localparam reg R = 2'b10;\n  localparam logic [7:0] W = 8'hFF;\n  \
         initial $display(\"VAL=%0d/%0d %0d/%0d %0d/%0d %0d/%0d\",\n    \
         N,$bits(N), M,$bits(M), R,$bits(R), W,$bits(W));\nendmodule\n",
    );
    assert_eq!(code, Some(0), "{out}");
    // iverilog and verilator: 1/1 1/1 0/1 255/8. An EXPLICIT range still wins.
    assert!(out.contains("VAL=1/1 1/1 0/1 255/8"), "{out}");
}
