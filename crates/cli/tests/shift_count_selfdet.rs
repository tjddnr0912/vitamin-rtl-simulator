//! A shift's RIGHT operand is self-determined AND unsigned (IEEE 1800 §11.4.10).
//!
//! The constant domain gave the count its OWN signedness — right for a ternary
//! condition and a select index, wrong here — so a narrow SIGNED count arrived negative
//! and every shift by it collapsed to 0. ROADMAP §2 row 27.
//!
//! ⭐⭐ It needed no function and no name to reach. The row was filed from a constant
//! function because that is where it was found; re-grounding it at HEAD showed the
//! mechanism reaches a NET's declared width and a `generate` condition, where the
//! damage is a silently wrong bus and a silently deleted body — both at exit 0. A
//! 158-cell census found 66 silent-wrong, and 32 more that were LATENT: the defect only
//! shows when the count's unsigned value is below the target width, so half the "correct"
//! cells were coincidences.
//!
//! ⭐ Every cell was a vita SELF-CONTRADICTION before any oracle was asked — the runtime
//! lane and the >64-bit constant lane both answered correctly while the i64 constant lane
//! did not. The >64-bit lane is also where the reference implementation lives:
//! `const_wide::fold_shift_count` reads the count's bits at its own width and treats them
//! as unsigned, citing this clause.
//!
//! ⚠️ The rule is a POST-STEP on the value and it belongs at the SHIFT ARM, not inside
//! the self-determined evaluator: that evaluator has 17 callers and exactly one is a
//! shift count. All THREE constant folds call one helper, so the rule is written once —
//! this repo has been bitten repeatedly by a rule mirrored into a second walk and then
//! drifting, and review found the third fold was the one that would have been left out.
//!
//! ⚠️⚠️ RESIDUE, measured and deliberate: the mask is applied only where the count's
//! width is a FACT — a literal, an operator tree over literals, or a name whose width
//! `envw` records (a subprogram local's declared range). A MODULE-SCOPE name keeps the
//! pre-slice answer, because `const_self_width` would take its width from `param_meta`,
//! and for an untyped `parameter C = 3'sd1;` that is the DEFAULT literal's 3 bits, which
//! §6.20.2 replaces the moment `#(.C(-3))` arrives. Reading the count SIGNED was
//! accidentally immune (−3 is out of range at any width, so the shift collapsed to 0 —
//! the right answer for a 32-bit count); reading it unsigned at a stale 3 bits makes it
//! **5** and the shift really happens. Review measured 21 cells going correct →
//! silent-wrong that way, across all four override channels and into a declared net
//! width. ⭐ `param_range` was tried as a second admission and MEASURED to have an entry
//! for that same overridden parameter — the maps that could answer this are the ones §2
//! row 14 is stuck on. Cost: 16 of 168 census cells. Every position the row is about uses
//! a literal count and is fixed.
//!
//! Values pinned to iverilog 13.0 and verilator 5.050.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_shc_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (s, out.status.success())
}

/// ⭐ The reachability the row did not record: no function, no name, and the two worst
/// consequences in one design — a declared width and a `generate` branch.
#[test]
fn a_signed_literal_count_reaches_a_net_width_and_a_generate_branch() {
    let (o, ok) = run("module top;\n  \
           logic [(16'h0100 >> 3'sb101)-1:0] bus;\n  \
           generate if (16'hFF01 << 3'sb101) begin : y\n    \
             initial begin $display(\"OUT=%0d taken\", $bits(bus)); $finish; end\n  \
           end else begin : n\n    \
             initial begin $display(\"OUT=%0d else\", $bits(bus)); $finish; end\n  \
           end endgenerate\n\
         endmodule\n");
    assert!(ok, "vita failed:\n{o}");
    // `3'sb101` is the count 5, so the width is 8 and the branch is taken. Before the
    // fix: `OUT=2 else` — a two-bit bus and the wrong body, at exit 0.
    assert!(o.contains("OUT=8 taken"), "{o}");
}

/// Every position a constant shift count can sit in, on one line.
#[test]
fn the_count_folds_the_same_in_every_constant_position() {
    for (what, src) in [
        (
            "unpacked dimension",
            "module top;\n  logic [7:0] m [(16'h0100 >> 3'sb101)];\n  \
               initial begin $display(\"OUT=%0d\", $size(m)); $finish; end\nendmodule\n",
        ),
        (
            "repeat count",
            "module top;\n  integer k = 0;\n  \
               initial begin repeat (16'h0100 >> 3'sb101) k = k + 1;\n    \
                 $display(\"OUT=%0d\", k); $finish; end\nendmodule\n",
        ),
        (
            "indexed part-select width",
            "module top;\n  logic [95:0] b = 96'h1234;\n  \
               initial begin $display(\"OUT=%0d\", $bits(b[7 -: (16'h0100 >> 3'sb101)]));\n    \
                 $finish; end\nendmodule\n",
        ),
        (
            "a localparam consumed as a bound",
            "module top;\n  localparam int W = 16'h0100 >> 3'sb101;\n  logic [W-1:0] v;\n  \
               initial begin $display(\"OUT=%0d\", $bits(v)); $finish; end\nendmodule\n",
        ),
        (
            "a constant function body",
            "module top;\n  \
               function automatic logic [15:0] f();\n    \
                 logic signed [2:0] C; logic [15:0] B;\n    \
                 C = -3'sd3; B = 16'hFF01;\n    f = B << C;\n  \
               endfunction\n  \
               localparam logic [15:0] FN = f();\n  \
               initial begin $display(\"OUT=%0d\", FN == 16'he020); $finish; end\nendmodule\n",
        ),
    ] {
        let (o, ok) = run(src);
        assert!(ok, "{what}:\n{o}");
        assert!(
            o.contains("OUT=8") || o.contains("OUT=1"),
            "{what}: want the oracle's answer\n{o}"
        );
    }
}

/// All four shift operators, and the spellings that decide whether the rule was put in
/// the right place. ⚠️ `C + 0` is 32 bits wide, so its UNSIGNED reading is 4294967293 and
/// the shift really does yield 0 — both oracles agree, and a "fix" that made the count
/// positive everywhere would get this cell wrong.
#[test]
fn every_operator_and_the_spellings_that_bound_the_rule() {
    // ⚠️ The count is a subprogram LOCAL, not a module-scope name. The mask is applied
    // only where the count's width is a FACT — `envw` holds a local's declared range,
    // and a module-scope name's width comes from `param_meta`, which is a stale DEFAULT
    // for an overridden untyped parameter. See `eval_const_shift_count`; the named
    // module-scope spelling is recorded residue, not a fix this test may assume.
    let body = |count: &str, op: &str| {
        format!(
            "module top;\n  \
               function automatic logic [15:0] f();\n    \
                 logic signed [2:0] C; logic [15:0] B;\n    \
                 C = -3'sd3; B = 16'hFF01;\n    f = B {op} ({count});\n  \
               endfunction\n  \
               localparam logic [15:0] X = f();\n  \
               initial begin $display(\"OUT=%h\", X); $finish; end\nendmodule\n"
        )
    };
    // The count is 5 in every spelling below, so `>>` gives 07f8 and `<<` gives e020.
    for count in ["C", "(C)", "3'sb101", "-3'sd3", "3'(C)", "1?C:C", "3'd5"] {
        for (op, want) in [
            ("<<", "e020"),
            (">>", "07f8"),
            ("<<<", "e020"),
            (">>>", "07f8"),
        ] {
            let (o, ok) = run(&body(count, op));
            assert!(ok, "{count} {op}:\n{o}");
            assert!(o.contains(&format!("OUT={want}")), "{count} {op}:\n{o}");
        }
    }
    // ⚠️ `C + 0` widens the count to 32 bits FIRST, so the unsigned reading is huge and
    // the answer is 0 — in vita and in both oracles.
    let (o, ok) = run(&body("C+0", ">>"));
    assert!(ok, "{o}");
    assert!(
        o.contains("OUT=0000"),
        "a widened count is genuinely huge:\n{o}"
    );
}

/// ⚠️ NO OTHER self-determined position takes this rule. §11.4.10 is specific to the
/// shift count; an index, a replication count and a `$clog2` argument each have their own
/// sizing sentence, and all three already folded correctly. Pinned so a future widening
/// of the rule has to justify itself here.
#[test]
fn the_neighbouring_self_determined_positions_are_untouched() {
    let (o, ok) = run("module top;\n  \
           localparam logic signed [2:0] C = -3'sd3;\n  \
           localparam logic [31:0] REP = {2{3'b101}};\n  \
           localparam int CL = $clog2(4'sd7 + 4'sd1);\n  \
           localparam logic T = C ? 1'b1 : 1'b0;\n  \
           initial begin $display(\"OUT=%0d %0d %0d\", REP, CL, T); $finish; end\n\
         endmodule\n");
    assert!(ok, "vita failed:\n{o}");
    assert!(o.contains("OUT=45 3 1"), "{o}");
}
