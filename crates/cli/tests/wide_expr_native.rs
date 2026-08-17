//! D1.5 ABSOLUTE ANCHOR — >64-bit expressions on TIER-3.
//!
//! Tier-3's speed comes from `wprog`, a width-specialised evaluator that admits
//! **uniform width ≤ 64 bits only**. Everything wider used to fall all the way to
//! the generic `eval_ctx` tree walk, because `CompileCtx` refused tier-3 the
//! tier-2 expression VM outright — with a reason that was correct about the
//! expressions both evaluators accept and silent about the ones `wprog` refuses.
//!
//! Measured cost of that silence (ROADMAP §5.1-av, same-session A/B): tier-3 was
//! **1.70×** slower than the VM on 100-bit arithmetic and **2.62×** on wide
//! select/concat — i.e. the product backend lost to the one it replaced, on an
//! entire family, for as long as tier-3 has existed.
//!
//! The fix is a partition rather than a switch: `native_eval` gets exactly the
//! RHSs `wprog` declines. Turning it on unconditionally was measured too and is
//! much worse (expr-heavy 157 → 478 ms), which is what the old comment was right
//! about.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_on(backend: &str, src: &str) -> (String, String) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_wide_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("--backend")
        .arg(backend)
        .arg("--obs-dir")
        .arg("obs")
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let rj = std::fs::read_to_string(d.join("obs").join("run.json")).unwrap_or_default();
    let txt = String::from_utf8_lossy(&out.stdout).into_owned();
    let body: String = txt
        .lines()
        .filter(|l| !l.starts_with("simulation ended") && !l.starts_with("warning"))
        .fold(String::new(), |mut a, l| {
            a.push_str(l);
            a.push('\n');
            a
        });
    (body, rj)
}

/// The whole >64-bit family in one design, pinned to iverilog 13.
///
/// Six lines, chosen so that a wrong wide lane cannot pass by accident:
///
///   * `add` and `sub` cross the 64-bit word boundary, so a carry/borrow that is
///     not propagated between words shows up immediately.
///   * `xsh` is `a ^ (a >> 13)` — a shift whose amount is not a multiple of 64,
///     which is where a word-at-a-time shift gets the seam wrong.
///   * `cat` is a three-part concat including a descending part-select
///     (`a[95 -: 8]`), the spelling whose `- width + 1` rule this repository has
///     had to fix before.
///   * `rep` is `{2{a[49:0]}}` — a replicate whose result (100 bits) is wider than
///     the 64-bit lane and whose source is not word-aligned.
///   * `mul` is the one operation where a wrong high word is invisible in the low
///     64 bits, so the top hex digits are the assertion.
///
/// ⚠️ ALL THREE BACKENDS, and that is the point of the third one. This exercises
/// a path that only tier-3 takes (`Op::EvalNative` emitted where `wprog`
/// declines), so a differential against the VM alone would be comparing the VM to
/// itself in the shape that matters least.
#[test]
fn wide_expressions_agree_with_iverilog_on_every_backend() {
    const SRC: &str = "module top;\n\
           reg [99:0] a, b, s;\n\
           initial begin\n\
             a = 100'hA5C31234DEADBEEF55AA33;\n\
             b = 100'h0123456789ABCDEF01234;\n\
             s = a + b;                            $display(\"add=%h\", s);\n\
             s = a ^ (a >> 13);                    $display(\"xsh=%h\", s);\n\
             s = {a[91:28], a[27:0], a[95 -: 8]};  $display(\"cat=%h\", s);\n\
             s = {2{a[49:0]}};                     $display(\"rep=%h\", s);\n\
             s = a * 100'd3;                       $display(\"mul=%h\", s);\n\
             s = a - b;                            $display(\"sub=%h\", s);\n\
             $finish;\n\
           end\n\
         endmodule\n";
    // iverilog 13.0, verbatim.
    const WANT: &str = "add=000a5d5468b57487bce45bc67\n\
                        xsh=000a5c63c2c4f0b4b82a2d09e\n\
                        cat=0a5c31234deadbeef55aa3300\n\
                        rep=ab6fbbd56a8ceadbeef55aa33\n\
                        mul=001f149369e9c093cce00fe99\n\
                        sub=000a5b0ddde661302106597ff\n";

    let (native, rj) = run_on("native", SRC);
    assert!(rj.contains("\"backend\": \"native\""), "not native:\n{rj}");
    assert_eq!(native, WANT, "the wide lane disagrees with iverilog");
    for b in ["vm", "interp"] {
        let (other, _) = run_on(b, SRC);
        assert_eq!(other, WANT, "backend {b} disagrees");
    }
}

/// The PARTITION itself: a design holding a wide expression AND a narrow one.
///
/// ⚠️ This is the shape a "switch" implementation gets wrong. Routing every RHS
/// to `native_eval` is measurably worse on the narrow half, and routing none of
/// them is worse on the wide half — so a single design containing both is the one
/// that cannot be satisfied by either extreme. Values are iverilog-pinned, and
/// the narrow line is written so its result depends on the wide one (`n` reads
/// `w`'s low bits), which stops a compiler from proving the halves independent
/// and reordering them apart.
#[test]
fn a_design_mixing_wide_and_narrow_expressions_is_correct() {
    const SRC: &str = "module top;\n\
           reg [99:0] w;\n\
           reg [31:0] n;\n\
           integer i;\n\
           initial begin\n\
             w = 100'h1234_5678_9ABC_DEF0_1234_5;\n\
             n = 32'd0;\n\
             for (i = 0; i < 4; i = i + 1) begin\n\
               w = (w << 3) ^ (w >> 61);\n\
               n = n + w[31:0] + i[31:0];\n\
             end\n\
             $display(\"w=%h n=%h\", w, n);\n\
             $finish;\n\
           end\n\
         endmodule\n";

    let (native, rj) = run_on("native", SRC);
    assert!(rj.contains("\"backend\": \"native\""), "not native:\n{rj}");
    // iverilog 13.0, verbatim — the value is pinned as well as compared, so all
    // three backends moving together is still caught.
    const WANT: &str = "w=0123456789abcdef012345040 n=4a7d2251\n";
    assert_eq!(
        native, WANT,
        "the mixed-width design disagrees with iverilog"
    );
    for b in ["vm", "interp"] {
        let (other, _) = run_on(b, SRC);
        assert_eq!(other, WANT, "backend {b} disagrees");
    }
}

/// D1.6: an RHS `wprog` declines for a reason that is NOT its width.
///
/// ⚠️⚠️ This is the shape D1.5's boundary got wrong. That version routed to
/// `native_eval` when the context width exceeded 64 — the first line of
/// `wprog::compile` — which is necessary and not sufficient: `compile` also
/// declines on node kinds. `s[idx +: 4]` is the common one (a part-select whose
/// offset is a RUNTIME value; `wprog` admits constant offsets only, §4.5.327), and
/// at 16 bits the width test waves it through. It then reached NEITHER evaluator
/// and fell to the generic tree walk, which is why `struct-heavy` stayed 1.30×
/// slower than the VM until the boundary asked `compile` itself (127 → 86 ms,
/// ROADMAP §5.1-ax).
///
/// The design keeps the narrow constant-offset work beside it (`s[11:4]`,
/// `s[19 -: 4]`, `{2{s[7:0]}}`) so both sides of the partition run in one body —
/// and `idx` MOVES each iteration, so a routing decision that silently froze the
/// offset would land on a different bit field. iverilog-pinned.
#[test]
fn a_runtime_offset_part_select_is_correct_on_every_backend() {
    const SRC: &str = "module top;\n\
           reg [31:0] s;\n\
           reg [15:0] acc;\n\
           reg [3:0] idx;\n\
           integer i;\n\
           initial begin\n\
             s = 32'hA5C31234; acc = 16'd0; idx = 4'd6;\n\
             for (i = 0; i < 6; i = i + 1) begin\n\
               acc = acc + {s[11:4], s[3:0], s[19 -: 4]} + {2{s[7:0]}};\n\
               acc = acc ^ {12'd0, s[idx +: 4]};\n\
               idx = idx + 4'd3;\n\
               s   = {s[30:0], s[31]};\n\
             end\n\
             $display(\"acc=%h s=%h idx=%h\", acc, s, idx);\n\
             $finish;\n\
           end\n\
         endmodule\n";
    // iverilog 13.0, verbatim.
    const WANT: &str = "acc=a439 s=70c48d29 idx=8\n";

    let (native, rj) = run_on("native", SRC);
    assert!(rj.contains("\"backend\": \"native\""), "not native:\n{rj}");
    assert_eq!(native, WANT, "a wprog-declined narrow RHS disagrees");
    for b in ["vm", "interp"] {
        let (other, _) = run_on(b, SRC);
        assert_eq!(other, WANT, "backend {b} disagrees");
    }
}
