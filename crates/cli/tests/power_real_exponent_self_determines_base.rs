//! §11.4.9 + §11.8.1: a real exponent revokes the base's context (§4.5.362).
//!
//! `x ** y` with a real `y` IS `$pow(x, y)`, and once the expression is real there is no
//! bit width left to hand down — the base is self-determined, exactly as it already was
//! for `+`, `-`, `*` and `/`. The ctx twin was handing it the ASSIGNMENT width instead,
//! so one base expression had two values depending on which operator consumed it:
//! `('1+4'h0)` reads 15 under `+ r` and read 65535 under `** r`.
//!
//! ⭐ THE ORACLE IS THE SAME TOOL ONE SPELLING OVER. iverilog produces the old value, so
//! for a while it looked like the reference; the question that settles it is to send the
//! same expression somewhere with no assignment width. To a `real`, and through the
//! explicit `$pow` the LRM defines the operator to mean, all three simulators report the
//! base as 15. A base cannot change value according to the width of the variable the
//! result is later stored in, so 480 is the destination leaking backwards.
//!
//! What must NOT move is the other half of Table 11-21: with an INTEGRAL exponent the
//! base genuinely is context-determined, and there the fill takes the assignment width
//! and ignores its sibling. Both halves are asserted here so a future edit cannot get
//! one by breaking the other; 384 pure-integral designs were byte-identical across this
//! change, and the 192-pair operator-vs-`$pow` gate went 115 -> 192.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn out(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let p = std::env::temp_dir().join(format!("vita_pwr_{}_{n}.sv", std::process::id()));
    std::fs::write(&p, src).unwrap();
    let o = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&p)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&p);
    let all = format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
    assert!(o.status.success(), "expected success:\n{src}\n{all}");
    all.lines()
        .find_map(|l| l.strip_prefix("r=").map(str::to_string))
        .unwrap_or_else(|| panic!("no r= line:\n{src}\n{all}"))
}

/// `a = <base> ** <exp>` into `logic [w-1:0]`.
fn pow_into(w: u32, base: &str, exp: &str) -> String {
    out(&format!(
        "module t; real r = 2.5, rn = 1.5; logic c = 1; logic [{}:0] a;\n\
         initial begin a = {base} ** {exp}; #1 $display(\"r=%0d\", a); $finish; end endmodule\n",
        w - 1
    ))
}

/// The same operands through the spelling §11.4.9 defines the operator to mean.
fn pow_ref(w: u32, base: &str, exp: &str) -> String {
    out(&format!(
        "module t; real r = 2.5, rn = 1.5, x; logic c = 1; logic [{}:0] a;\n\
         initial begin x = $pow({base}, {exp}); a = x; #1 $display(\"r=%0d\", a); $finish; end endmodule\n",
        w - 1
    ))
}

// ── the rule ───────────────────────────────────────────────────────────────────

#[test]
fn the_operator_form_agrees_with_the_pow_form_it_is_defined_as() {
    // ⭐ THE WHOLE SLICE IN ONE ASSERTION. Every pair below disagreed before the fix
    // for at least one width; `$pow` is the side all three simulators already agreed on.
    for base in [
        "'1",
        "('1+4'h0)",
        "('1|16'd0)",
        "(c?'1:4'h2)",
        "('1+8'h0)",
        "{'1}",
        "(4'h3+'1)",
        "'1+'1",
        "(4'd15+4'd1)",
        "4'd15",
    ] {
        for exp in ["r", "rn", "2.0", "0.5"] {
            for w in [8u32, 16, 32, 64] {
                assert_eq!(
                    pow_into(w, base, exp),
                    pow_ref(w, base, exp),
                    "`{base} ** {exp}` into {w} bits must equal $pow of the same operands"
                );
            }
        }
    }
}

#[test]
fn a_fill_next_to_a_real_takes_its_sibling_not_the_destination() {
    // The four bases that used to collapse to one value (480) at 16 bits. Each one's
    // sibling is different, so each answer must be different.
    assert_eq!(
        pow_into(16, "'1", "r"),
        "1",
        "no sibling with a width ⇒ the fill is one bit"
    );
    assert_eq!(pow_into(16, "('1+4'h0)", "r"), "871", "4-bit sibling ⇒ 15");
    assert_eq!(
        pow_into(16, "('1|16'd0)", "r"),
        "480",
        "16-bit sibling ⇒ 65535"
    );
    assert_eq!(
        pow_into(16, "(c?'1:4'h2)", "r"),
        "871",
        "ternary sibling is 4 bits ⇒ 15"
    );
    // ⭐ AND THE PROOF THAT THIS IS THE SAME RULE THE OTHER OPERATORS ALREADY USE:
    // the identical four bases under `+ r` have always produced these four values,
    // and vita agreed with iverilog on all of them before this slice.
    assert_eq!(
        out("module t; real r = 2.5; logic c = 1; logic [15:0] a, b, cc, d;\n\
             initial begin a = '1 + r; b = ('1+4'h0) + r; cc = ('1|16'd0) + r; d = (c?'1:4'h2) + r;\n\
             #1 $display(\"r=%0d %0d %0d %0d\", a, b, cc, d); $finish; end endmodule\n"),
        "4 18 2 18"
    );
}

// ── the half that must not move ────────────────────────────────────────────────

#[test]
fn an_integral_exponent_still_leaves_the_base_context_determined() {
    // Table 11-21 makes a power's base context-determined and its exponent
    // self-determined. That is unchanged: with an integral exponent the fill takes the
    // ASSIGNMENT width and its 4-bit sibling does not get a say.
    assert_eq!(pow_into(16, "('1+4'h0)", "3"), "65535");
    assert_eq!(pow_into(16, "(4'd15+4'd1)", "2"), "256");
    assert_eq!(pow_into(16, "'1", "3"), "65535");
    assert_eq!(pow_into(8, "('1+4'h0)", "3"), "255");
    // A fill in the EXPONENT is still self-determined at one bit (§4.5.354's pin —
    // the guard that fix installed is the reason this arm is reached at all).
    assert_eq!(pow_into(8, "4'd2", "'1"), "2");
}

#[test]
fn the_exponents_domain_is_what_switches_the_rule_not_its_spelling() {
    // ⚠️ `2.0` and `2` differ only in domain, and that is exactly the switch: the real
    // literal self-determines the base (fill ⇒ 4-bit sibling ⇒ 15² = 225), the integral
    // one leaves it context-determined (fill ⇒ 16 bits ⇒ 65535² mod 2**16 = 1).
    assert_eq!(pow_into(16, "('1+4'h0)", "2.0"), "225");
    assert_eq!(pow_into(16, "('1+4'h0)", "2"), "1");
    // And a real-valued VARIABLE exponent switches it the same way a literal does.
    assert_eq!(
        out("module t; real two = 2.0; logic [15:0] a;\n\
             initial begin a = ('1+4'h0) ** two; #1 $display(\"r=%0d\", a); $finish; end endmodule\n"),
        "225"
    );
}

#[test]
fn a_real_destination_never_had_the_leak_and_still_does_not() {
    // No assignment width exists here, so this side was always correct — it is the
    // measurement that disqualified the old reading, kept as a pin.
    assert_eq!(
        out("module t; real r = 2.5, x; logic c = 1;\n\
             initial begin x = (c?'1:4'h2) ** r; #1 $display(\"r=%0.4f\", x); $finish; end endmodule\n"),
        "871.4213"
    );
    assert_eq!(
        out("module t; real r = 2.5, x;\n\
             initial begin x = '1 ** r; #1 $display(\"r=%0.4f\", x); $finish; end endmodule\n"),
        "1.0000"
    );
}
