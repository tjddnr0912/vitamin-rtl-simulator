//! A `case` selector that is a fill literal is sized to the COMMON MAXIMUM of the
//! selector and every item, not left at one bit (§4.5.356).
//!
//! IEEE 1800 §12.5 sizes the case expression and all item expressions together. vita
//! implemented one half of that — `lower_case_label` gives every label the selector's
//! width — which is the whole rule while the selector HAS a width. A bare fill does
//! not, so `case ('1)` compared one bit against everything:
//!
//! | shape                                     | PRE  | both oracles |
//! |-------------------------------------------|------|--------------|
//! | `case ('1) 8'hFF: a=1; 1'b1: a=3;`        | `3`  | `1`          |
//! | `casez`/`casex` with the same selector    | `3`  | `1`          |
//! | `case (('1)) …`                           | `3`  | `1`          |
//! | `case ('1) 8'hFF: a=1; '1: a=3;`          | `3`  | `1`          |
//! | `case ('1) 8'h01: a=1; '1: a=3;`          | `1`  | `3`          |
//!
//! ⚠️ THE LAST ROW IS WHY BOTH ENDS HAVE TO MOVE. Widening only the selector sends it
//! to `default` — a different wrong answer, which is the trade this project forbids. A
//! label that is itself a fill was sized against the old one-bit selector and has to
//! meet the new width too, so the fix re-runs the SAME `lower_case_label` on the
//! fill-bearing labels once the selector is real.
//!
//! ⚠️ The label half was already right and is pinned below: `case (x8) '1: …` has
//! worked since §4.5.353, and a fill selector whose siblings are self-determined
//! (`case ('1 + 1)`, `case ({2{'1}})`) was already right for the same reason it is in
//! an assignment — the width comes from the sibling or from the operand's own
//! position, not from the context.
//!
//! ORACLES: iverilog 13.0 and verilator 5.050 agree on every value here.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// `case (<sel>)` with `arms` = (label, value) pairs, `default: a = 2`.
fn case_of(kind: &str, sel: &str, arms: &[(&str, &str)]) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let mut body = String::new();
    for (l, v) in arms {
        body.push_str(&format!("      {l}: a = {v};\n"));
    }
    let src = format!(
        "module t;\n  logic [7:0] x8 = 8'hFF; logic x1 = 1'b1; logic [7:0] a;\n\
         initial begin\n    {kind} ({sel})\n{body}      default: a = 2;\n    endcase\n\
         #1 $display(\"r=%0d\", a); $finish; end\nendmodule\n"
    );
    let p = std::env::temp_dir().join(format!("vita_csfw_{}_{n}.sv", std::process::id()));
    std::fs::write(&p, &src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&p)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&p);
    let all = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.status.success(), "expected success for `{sel}`:\n{all}");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .find_map(|l| l.strip_prefix("r=").map(str::to_string))
        .unwrap_or_else(|| panic!("no r= line for `{sel}`:\n{all}"))
}

#[test]
fn a_fill_selector_is_sized_to_the_widest_item() {
    // The label order is deliberately both ways round: the bug was not "the first arm
    // wins", it was that the 1-bit arm was the only one the selector could ever equal.
    for arms in [
        [("8'hFF", "1"), ("1'b1", "3")],
        [("1'b1", "3"), ("8'hFF", "1")],
    ] {
        assert_eq!(case_of("case", "'1", &arms), "1", "arms {arms:?}");
    }
    for w in ["4'hF", "16'hFFFF"] {
        assert_eq!(case_of("case", "'1", &[(w, "1"), ("1'b1", "3")]), "1");
    }
}

#[test]
fn casez_and_casex_and_a_parenthesised_selector_behave_the_same() {
    for kind in ["case", "casez", "casex"] {
        assert_eq!(
            case_of(kind, "'1", &[("8'hFF", "1"), ("1'b1", "3")]),
            "1",
            "for {kind}"
        );
    }
    assert_eq!(
        case_of("case", "('1)", &[("8'hFF", "1"), ("1'b1", "3")]),
        "1"
    );
}

#[test]
fn a_fill_label_widens_with_the_selector() {
    // ⚠️ THE PIN THAT FORCES BOTH ENDS TO MOVE. `'1` must equal the now-8-bit selector,
    // so the answer is the `'1` arm — NOT `default`, which is where widening only the
    // selector would land.
    assert_eq!(case_of("case", "'1", &[("8'h01", "1"), ("'1", "3")]), "3");
    // And with a matching sized label first, that one still wins.
    assert_eq!(case_of("case", "'1", &[("8'hFF", "1"), ("'1", "3")]), "1");
    // Selector and label both fills and nothing else: common max is one bit, so this
    // is a plain match and the widening must not disturb it.
    assert_eq!(case_of("case", "'1", &[("'1", "1")]), "1");
}

#[test]
fn a_zero_fill_selector_is_unchanged() {
    // `'0` is invisible to this bug (zero is zero at every width), so it is the control
    // that says the fix did not simply start taking a different arm.
    assert_eq!(case_of("case", "'0", &[("8'h00", "1"), ("1'b0", "3")]), "1");
    assert_eq!(case_of("case", "'0", &[("8'hFF", "1"), ("1'b0", "3")]), "3");
}

#[test]
fn the_label_half_of_the_rule_is_untouched() {
    // §4.5.353 made a fill LABEL take the selector's width; that direction was already
    // right and must stay right — this test fails if the new pass re-lowers labels when
    // it should not.
    assert_eq!(case_of("case", "x8", &[("'1", "1"), ("8'hFF", "3")]), "1");
    assert_eq!(case_of("case", "x8", &[("'1", "1")]), "1");
    assert_eq!(case_of("case", "x1", &[("'1", "1")]), "1");
    assert_eq!(case_of("case", "x8", &[("8'hFF", "1"), ("1'b1", "3")]), "1");
}

#[test]
fn the_common_maximum_is_collective_even_when_the_selector_has_no_fill() {
    // ⭐ THE OTHER HALF, found by the sweep after the first fix landed. §12.5 sizes the
    // selector and EVERY item together, so a sized selector plus a fill label plus a
    // wider sized label still has a common maximum of 8 — the fill label becomes
    // `8'hFF`, matches neither the 1-bit selector nor `8'hFF`'s value, and the answer
    // is `default`. vita gave the fill label the SELECTOR's width (1), so `'1` was
    // `1'b1` and matched. Both oracles say `2`.
    for sel in ["1'b1", "x1", "$signed('1)", "{2{'1}}"] {
        assert_eq!(
            case_of("case", sel, &[("'1", "1"), ("8'hFF", "3")]),
            "2",
            "for selector {sel}"
        );
    }
    // The same shape with nothing wider than the selector keeps matching: the maximum
    // is the selector's own width and nothing moves.
    assert_eq!(case_of("case", "x1", &[("'1", "1")]), "1");
    assert_eq!(case_of("case", "1'b1", &[("'1", "1")]), "1");
}

#[test]
fn a_real_selector_is_excluded_from_the_collective_width() {
    // ⚠️ §6.12 / the §4.5.354 pin, re-checked with a WIDER sibling label present — the
    // shape that would drag `'1` up to eight bits if `real` were not excluded. All
    // three oracles keep the `'1` arm.
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let src = "module t;\n  real r = 1.0; logic [7:0] a;\n\
        initial begin\n    case (r)\n      '1: a = 1;\n      8'hFF: a = 3;\n\
        default: a = 2;\n    endcase\n    #1 $display(\"r=%0d\", a); $finish; end\nendmodule\n";
    let p = std::env::temp_dir().join(format!("vita_csfw_r_{}_{n}.sv", std::process::id()));
    std::fs::write(&p, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&p)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&p);
    let so = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{so}");
    assert!(so.contains("r=1"), "expected the '1 arm, got:\n{so}");
}

#[test]
fn a_fill_with_a_sibling_in_the_selector_is_untouched() {
    // Same reason as in an assignment: the width comes from the sibling or from the
    // operand's own self-determined position, so the context was never the only source.
    assert_eq!(
        case_of("case", "'1 + 1", &[("8'h00", "1"), ("1'b0", "3")]),
        "1"
    );
    assert_eq!(
        case_of("case", "{2{'1}}", &[("8'h03", "1"), ("2'b11", "3")]),
        "1"
    );
}
