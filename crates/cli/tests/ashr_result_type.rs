//! `>>>` filled with the sign bit whenever its LEFT OPERAND was signed. IEEE 1800
//! §11.4.10 says the fill follows the **result type**, and §11.8.1 makes that type
//! unsigned as soon as ANY operand of the surrounding context-determined region is.
//!
//! ## What was wrong
//!
//! `eval_ctx`'s `AShr` arm read `self.wt.signed(lhs)` — the left operand's own recorded
//! signedness — under a comment asserting that an unsigned enclosing context
//! *"MUST NOT demote a genuinely-signed `s >>> n` to a logical shift"*. Demoting it is
//! exactly what §11.4.10 requires. A 303-cell census against BOTH oracles put **70 cells**
//! on the wrong side of it, with **zero oracle split** — the two tools never disagreed, so
//! there was nothing to arbitrate.
//!
//! ```text
//!   reg signed [7:0] b = 8'shB3;    (b >>> 4) > 8'd100
//!     vita 1          iverilog 0          verilator 0          exit 0, no diagnostic
//! ```
//!
//! ⭐ The unsignedness arrives from an operand the shift does not contain — the
//! comparison's other side, or a sibling of a `+` — which is why every part of the shift
//! looks signed when you inspect it on its own. That is also why it survived: the
//! neighbouring spellings (`>>> ` alone, against a signed comparand, under `$signed`) are
//! all correct, so only a probe that puts an unsigned operand *nearby* can see it.
//!
//! ## Where it lived
//!
//! ⚠️⚠️ TWO copies. Fixing `eval_ctx` left `native_eval`'s compiler (tier 2) answering
//! 61440 where the generic evaluator now answered 4096 — its own differential battery
//! caught that immediately, which is what that battery is for.
//!
//! ⚠️⚠️ AND A THIRD THING THE FIX UNCOVERED, which the adversarial review graded BLOCKING.
//! Both frame call funnels evaluated an actual with the FORMAL's signedness as the context
//! sign. §11.8.3 gives an assignment-like context its WIDTH but not its SIGN, so that was
//! wrong from the start — but the old `AShr` arm ignored `ctx_signed` entirely, so `>>>`
//! was accidentally immune. Honouring the result type removed the cancellation and turned
//! a wrong context sign into a wrong VALUE: `au(b >>> 2)` went 4294967276 → 44 at exit 0,
//! a correct→silent-wrong of my own making. Fixed at both funnels
//! (`eval_core`'s `Expr::Call` arm and `exec/frame_call.rs`'s `split_in_binds`), which
//! also closed the three pre-existing cells above it and the documented gap pinned in
//! `inline_formal_bind.rs`.
//!
//! ⚠️⚠️ A THIRD site, `wprog`, needed no change — but NOT for the reason I first wrote
//! here. I claimed it was "already producing the oracles' answer", which would have meant
//! one binary held two answers for one expression. **The review measured that and it is
//! false**: before the fix, `native` gave the interpreters' wrong answer too. `wprog` is
//! unaffected because its uniform-sign gate declines the whole family one step earlier —
//! a shift's recorded self-sign IS its left operand's, so a signed operand in an unsigned
//! context fails `sw.signed != signed` before the `AShr` arm is ever reached, and the RHS
//! routes to tier 2, which carried the same buggy copy. `wprog` compiles only the
//! unsigned-operand case, which nothing got wrong.
//!
//! ⭐ Recorded because the mistake is instructive: "the compiled backend declines this"
//! and "the compiled backend answers this correctly" are different claims, and I asserted
//! the second from evidence for the first.
//!
//! ⚠️ TWO COPIES REMAIN, both pre-existing and both filed rather than fixed here: the
//! elaborate-time constant folder (`const_fn.rs`'s plain-i64 `AShr`, and its wide twin)
//! still uses the old rule, so a `localparam` and its runtime twin can now disagree; and
//! a `case` scrutinee's unsignedness is applied as an outer `$unsigned` rather than
//! propagated into the scrutinee, so unsigned case ITEMS do not demote the shift.
//!
//! ## The controls
//!
//! Every cell below carries its own control, because "the shift is wrong" and "the
//! propagation is wrong" look identical from a single failing number: against a SIGNED
//! comparand the shift must stay arithmetic, and with no comparand at all it must be
//! arithmetic too.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// All three backends must agree — `native`, the bytecode VM and the interpreter reach
/// three different `>>>` implementations, and this defect lived in two of them.
fn agrees_across_backends(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_ashr_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).expect("scratch dir");
    std::fs::write(d.join("t.sv"), src).expect("write design");
    let mut first: Option<String> = None;
    for be in ["native", "vm", "interp"] {
        let out = Command::new(env!("CARGO_BIN_EXE_vita"))
            .args(["t.sv", "--backend", be])
            .current_dir(&d)
            .output()
            .expect("run vita");
        let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
        s.push_str(&String::from_utf8_lossy(&out.stderr));
        match &first {
            None => first = Some(s),
            Some(f) => assert_eq!(f, &s, "backend {be} diverged"),
        }
    }
    let _ = std::fs::remove_dir_all(&d);
    first.unwrap()
}

/// ⭐ THE CELL. `8'shB3 >>> 2` is `8'hEC` (236) filled with the sign bit and `8'h2C` (44)
/// filled with zero. Each position below puts an UNSIGNED operand next to the shift, so
/// the result type is unsigned and the fill must be zero.
///
/// ⚠️ The comparands are chosen to sit BETWEEN 44 and 236. My first census used `8'd10`,
/// where both fills answer the same thing — 20 comparison cells that could not fail.
///
/// Both oracles: `A=0 B=1 C=44 D=44 E=44 F=44`.
#[test]
fn an_unsigned_neighbour_makes_the_shift_logical() {
    let out = agrees_across_backends(
        "`timescale 1ns/1ns\nmodule t;\n  reg signed [7:0] b;\n  \
         initial begin b = 8'shB3;\n    \
         $display(\"A=%0d B=%0d C=%0d D=%0d E=%0d F=%0d\",\n      \
         (b >>> 2) > 8'd100,\n      8'd100 > (b >>> 2),\n      \
         (b >>> 2) + 8'd0,\n      (b >>> 2) | 8'd0,\n      \
         (1 ? (b >>> 2) : 8'd0),\n      (b >>> 2) & 8'hFF);\n    \
         $finish;\n  end\nendmodule\n",
    );
    assert!(
        out.contains("A=0 B=1 C=44 D=44 E=44 F=44"),
        "an unsigned operand anywhere in the region zero-fills the shift:\n{out}"
    );
}

/// THE CONTROL for the cell above. Every neighbour is signed, so the result type stays
/// signed and the fill is the sign bit — the behaviour the old rule got right and which a
/// fix that simply made `>>>` always logical would destroy.
///
/// Both oracles: `A=1 B=0 C=-20 D=-20 E=-20`.
#[test]
fn a_signed_neighbour_keeps_the_shift_arithmetic() {
    let out = agrees_across_backends(
        "`timescale 1ns/1ns\nmodule t;\n  reg signed [7:0] b; reg signed [7:0] r;\n  \
         initial begin b = 8'shB3;\n    r = b >>> 2;\n    \
         $display(\"A=%0d B=%0d C=%0d D=%0d E=%0d\",\n      \
         (b >>> 2) > -8'sd100,\n      (b >>> 2) > 8'sd0,\n      \
         (b >>> 2) + 8'sd0,\n      $signed(b >>> 2),\n      r);\n    \
         $finish;\n  end\nendmodule\n",
    );
    assert!(
        out.contains("A=1 B=0 C=-20 D=-20 E=-20"),
        "with only signed neighbours the fill is still the sign bit:\n{out}"
    );
}

/// ⚠️ The rule is about the REGION, not the immediate parent: an unsigned operand two
/// levels out still demotes the shift, because the whole context-determined region takes
/// one type. `G` nests the shift inside a SIGNED `+` inside an unsigned one and still
/// gets 44.
///
/// `H` and `I` are the boundary in the other direction — a SELF-determined position does
/// not inherit the region's type at all, so the shift keeps its own signedness there.
/// `I = {1'b0, (b >>> 2)}` is **236**, not 44: a concatenation part is self-determined
/// even though the concat sits in an unsigned context. ⚠️ I guessed 44 and the oracle
/// said 236; that cell is the one that distinguishes "propagate into the region" from
/// "make `>>>` unsigned whenever anything nearby is".
///
/// Both oracles: `G=44 H=1 I=236`.
#[test]
fn the_region_decides_and_a_self_determined_position_is_outside_it() {
    let out = agrees_across_backends(
        "`timescale 1ns/1ns\nmodule t;\n  reg signed [7:0] b;\n  \
         initial begin b = 8'shB3;\n    \
         $display(\"G=%0d H=%0d I=%0d\",\n      \
         ((b >>> 2) + 8'sd0) + 8'd0,\n      \
         ((b >>> 2) != 0) && 1'b1,\n      \
         {1'b0, (b >>> 2)});\n    \
         $finish;\n  end\nendmodule\n",
    );
    assert!(out.contains("G=44 H=1 I=236"), "{out}");
}

/// The x/z half: an unknown SIGN BIT fills with x when the result type is signed and with
/// ZERO when it is unsigned, so the two rules stay distinguishable on the unknown plane —
/// which is where a fix that handled only the value plane would show up.
///
/// ⚠️ ORACLE NOTE: iverilog only. verilator answers `00001100` for all three because it
/// models the 4-state value as 2-state, so it cannot see this cell at all. vita matches
/// iverilog exactly.
///
/// ⚠️ The observation is `|` and a bare read, NOT `+`: any x in an addend makes the whole
/// sum x, so an arithmetic probe here returns `xxxxxxxx` whichever fill ran. I wrote the
/// `+` version first and it could not have failed.
#[test]
fn an_unknown_sign_bit_follows_the_same_rule() {
    let out = agrees_across_backends(
        "`timescale 1ns/1ns\nmodule t;\n  \
         reg signed [7:0] b; reg [7:0] u8; reg signed [7:0] s8;\n  \
         initial begin b = 8'sbx0110011;\n    \
         u8 = (b >>> 2) | 8'd0;\n    s8 = (b >>> 2) | 8'sd0;\n    \
         $display(\"J=%b K=%b L=%b\", u8, s8, (b >>> 2));\n    \
         $finish;\n  end\nendmodule\n",
    );
    assert!(
        out.contains("J=00x01100") && out.contains("K=xxx01100") && out.contains("L=xxx01100"),
        "an unsigned result type zero-fills even an unknown sign bit; a signed one fills \
         with x, and a self-determined read keeps its own sign:\n{out}"
    );
}
