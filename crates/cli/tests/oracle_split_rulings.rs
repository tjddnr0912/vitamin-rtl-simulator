//! Where the two oracles DISAGREE, this file pins what vita does and why (§4.5.361).
//!
//! Every other differential test in this suite exists because two tools agreed and vita
//! did not. These four cells are the opposite: iverilog and verilator disagree with each
//! other, so the differential rule cannot break the tie and a hand-IEEE reading has to.
//! The danger they share is specific — a future sweep sees vita differing from ONE tool,
//! "fixes" it, and lands a silent-wrong. These tests are what makes that fail loudly.
//!
//! ⭐ THREE OF THE FOUR ARE DELIBERATE NO-OPS, and the fourth is a REFUTED FIX.
//!
//! 1. `string` relational against an integral (`s <= 1'b1`). vita reads 0, iverilog 1.
//!    ⚠️ IVERILOG 13.0 IS NOT AN ORACLE HERE AT ALL: `s < "ab"`, `s < "aa"` and
//!    `s < "zz"` all read 1 — a pure string-vs-string comparison with no width, sign or
//!    conversion in it, whose answer cannot be 1 for all three. Its `1` carries no
//!    information. verilator and vita agree on every cell.
//!
//! 2. `'1 ** r` with real `r`. ⚠️⚠️ THIS ROW WAS RULED WRONG AND IS NOW FIXED (§4.5.362).
//!    §4.5.361 read the 480 as correct because iverilog produced it, and scored a
//!    candidate fix by iverilog agreement (267 -> 247) — but iverilog is the leaking
//!    party here, and the score was measuring the leak. The disqualifying evidence was
//!    one question away and I did not ask it: send the SAME expression somewhere with no
//!    assignment width. `real x = ('1+4'h0) ** r` is 871.4213 in all three simulators,
//!    and `$pow(('1+4'h0), r)` — the spelling §11.4.9 defines the operator to mean — is
//!    871.4213 in all three too. A base cannot change value according to the width of
//!    the variable the RESULT is later stored in. The fix and its 192-pair gate live in
//!    `power_real_exponent_self_determines_base.rs`; what stays here is the corrected
//!    ruling and the neighbours that prove it.
//!
//! 3. `$itor(-<unsigned>)`. vita reads 4294967288, iverilog -8. ⚠️ The split is NOT about
//!    signedness: iverilog reads `$itor(64'h1_0000_0008)` as 8 for an unsigned AND for a
//!    signed `longint` of the same value, i.e. it truncates to a 32-bit container.
//!    Adopting that would turn vita's mathematically correct answer into a wrong one for
//!    every argument above 2**31 — correct-support to silent-wrong, which the ladder
//!    forbids.
//!
//! 4. `%` on a real operand. vita rejects (E3009), both tools run it — and produce
//!    DIFFERENT answers, which is the signature of undefined territory rather than of a
//!    defined meaning. For illegal code a loud reject is the top rung, so this is not a
//!    §3 gap and must not be "supported".
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (bool, String) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let p = std::env::temp_dir().join(format!("vita_osr_{}_{n}.sv", std::process::id()));
    std::fs::write(&p, src).unwrap();
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
    (out.status.success(), all)
}

fn line(src: &str) -> String {
    let (ok, all) = run(src);
    assert!(ok, "expected success:\n{src}\n{all}");
    all.lines()
        .find_map(|l| l.strip_prefix("r=").map(str::to_string))
        .unwrap_or_else(|| panic!("no r= line:\n{src}\n{all}"))
}

// ── 1. string relational: iverilog is not an oracle ─────────────────────────────

#[test]
fn a_string_relational_is_lexicographic_and_iverilog_is_not_the_reference() {
    // The three cells that disqualify iverilog: it answers 1 to all of them.
    assert_eq!(
        line(
            "module t; string s = \"ab\";\n\
              initial begin #1 $display(\"r=%b %b %b\", s < \"ab\", s < \"aa\", s < \"zz\");\n\
              $finish; end endmodule\n"
        ),
        "0 0 1"
    );
    // The headline cell, and its `>=` mirror. verilator agrees with both.
    assert_eq!(
        line("module t; string s = \"ab\";\n\
              initial begin #1 $display(\"r=%b %b\", s <= 1'b1, s >= 1'b1); $finish; end endmodule\n"),
        "0 1"
    );
}

// ── 2. the ruling that was wrong, and the neighbours that show it ───────────────

#[test]
fn a_power_with_a_real_exponent_self_determines_its_base() {
    // ⭐ THE DISQUALIFYING QUESTION: the same expression with no assignment width.
    // All three simulators say the base is 15 here. iverilog's 480 for the 16-bit
    // destination is that destination leaking into the base — it cannot be the
    // reference for a value it reports as 15 when asked without an integral target.
    assert_eq!(
        line("module t; real r = 2.5, x;\n              initial begin x = ('1+4'h0) ** r; #1 $display(\"r=%0.4f\", x); $finish; end endmodule\n"),
        "871.4213"
    );
    // ⭐ And the spelling §11.4.9 defines the operator to MEAN, which even verilator
    // agrees with (it is wrong about fill sizing, not about self-determination).
    assert_eq!(
        line("module t; real r = 2.5, x;\n              initial begin x = $pow(('1+4'h0), r); #1 $display(\"r=%0.4f\", x); $finish; end endmodule\n"),
        "871.4213"
    );
    // The integral destinations, which used to read 480 for every base because the
    // assignment width overrode the sibling.
    assert_eq!(
        line("module t; real r = 2.5; logic [15:0] a, b, c;\n              initial begin a = '1 ** r; b = ('1+4'h0) ** r; c = ('1|16'd0) ** r;\n              #1 $display(\"r=%0d %0d %0d\", a, b, c); $finish; end endmodule\n"),
        "1 871 480"
    );
    // ⚠️ THE HALF THAT MUST NOT MOVE: with an INTEGRAL exponent the base really is
    // context-determined (Table 11-21), so the fill takes the assignment width and
    // ignores its 4-bit sibling. Both readings are live in this one file on purpose.
    assert_eq!(
        line("module t; logic [15:0] a, b;\n              initial begin a = (4'd15+4'd1) ** 2; b = ('1+4'h0) ** 3;\n              #1 $display(\"r=%0d %0d\", a, b); $finish; end endmodule\n"),
        "256 65535"
    );
    // ⚠️ AND THE NON-FILL BASE, which was already self-determined and stays that way —
    // iverilog reads 1024 here, again by leaking the destination width into the base.
    assert_eq!(
        line("module t; real r = 2.5; logic [15:0] a;\n              initial begin a = (4'd15+4'd1) ** r; #1 $display(\"r=%0d\", a); $finish; end endmodule\n"),
        "0"
    );
}

// ── 3. `$itor`: iverilog truncates to a 32-bit container ────────────────────────

#[test]
fn itor_keeps_the_value_instead_of_truncating_to_thirty_two_bits() {
    // The reported split.
    assert_eq!(
        line(
            "module t; int unsigned iu = 8; real a;\n\
              initial begin a = $itor(-iu); #1 $display(\"r=%g\", a); $finish; end endmodule\n"
        ),
        "4.29497e+09"
    );
    // ⭐ THE ANTI-TRUNCATION PIN, and the reason the split is not about signedness:
    // iverilog reads BOTH of these as 8. Neither has a unary minus in it, and the second
    // is signed. A future "align to iverilog" edit fails here first.
    assert_eq!(
        line(
            "module t; longint unsigned l1 = 64'h1_0000_0008; longint ls = 4294967304;\n\
              real a, b;\n\
              initial begin a = $itor(l1); b = $itor(ls);\n\
              #1 $display(\"r=%g %g\", a, b); $finish; end endmodule\n"
        ),
        "4.29497e+09 4.29497e+09"
    );
    // The rows where all three tools DO agree — the ones an alignment edit would break.
    assert_eq!(
        line(
            "module t; int unsigned iu = 8; real a, b;\n\
              initial begin a = $itor($signed(-iu)); b = real'(-iu);\n\
              #1 $display(\"r=%g %g\", a, b); $finish; end endmodule\n"
        ),
        "-8 4.29497e+09"
    );
}

// ── 4. `%` on a real operand stays loud ─────────────────────────────────────────

#[test]
fn modulo_on_a_real_operand_stays_loud() {
    // Both tools run this and disagree with EACH OTHER on the answer (iverilog computes
    // fmod and yields 1.5; verilator rounds the operand and yields 0), which is what an
    // undefined corner looks like. For illegal code the reject is the top rung.
    for expr in ["r % 1'b1", "r % s", "1'b1 % r"] {
        let (ok, all) = run(&format!(
            "module t; real r = 5.5, s = 2.0; logic [15:0] a;\n\
             initial begin a = {expr}; #1 $display(\"r=%0d\", a); $finish; end endmodule\n"
        ));
        assert!(!ok, "`{expr}` must stay loud:\n{all}");
        assert!(
            all.contains("modulo (%) not defined on real operand"),
            "`{expr}` must name the rule:\n{all}"
        );
    }
    // ⭐ iverilog enforces the SAME table one operator over — it rejects a shift on the
    // real that its own `%` produced. That self-contradiction is why its acceptance of
    // `%` reads as an unfilled hole rather than as a defined meaning.
    let (ok, all) = run("module t; real r = 5.5; logic [15:0] a;\n\
         initial begin a = r << 1; #1 $display(\"r=%0d\", a); $finish; end endmodule\n");
    assert!(!ok, "shift on a real must stay loud too:\n{all}");
}
