//! `%t` / `$timeformat` full IEEE 1800 §21.3.2 semantics — iverilog-13.0-pinned.
//!
//! The old `%t` was a plain `%0d` (a documented caveat but a use-site
//! silent-wrong): no rescale of the module-unit time value to the `$timeformat`
//! units (default = the GLOBAL PRECISION, so a 1ns/1ps module must show ps) and
//! no default min field width 20. `$timeformat` itself was a W3056 warn-skip.
//! Every expectation string here is a live iverilog byte pin, except the one
//! negative scale-down case where iverilog's own output is corrupt (`0.0-u`) —
//! there vita pins its spec-sensible rendering instead.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, i32) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_tfmt_{}_{n}", std::process::id()));
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
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code().unwrap_or(-1),
    )
}

#[test]
fn default_t_scales_to_precision_and_pads_20() {
    // 1ns/1ps ⇒ multiplier 1000; default units = the global precision (ps):
    // $time=5 renders 5000. Bare %t right-justifies in the DEFAULT width 20; an
    // explicit %0t/%5t/%30t OVERRIDES it. A plain literal is a time value too
    // (42 → 42000). An unknown-bearing value keeps its collapsed char and the
    // scale zeros append TEXTUALLY (xx10 → "X000"). Zero stays "0".
    let (out, _, _) = run("`timescale 1ns/1ps\n\
         module t; initial begin\n\
           $display(\"[%t]\", 64'd0);\n\
           #5 $display(\"[%t]\", $time);\n\
           $display(\"[%0t]\", $time);\n\
           $display(\"[%5t]\", $time);\n\
           $display(\"[%30t]\", $time);\n\
           $display(\"[%t]\", 42);\n\
           $display(\"[%t]\", 4'bxx10);\n\
           $display(\"[%t]\", 4'bzz10);\n\
           $display(\"[%t]\", 64'hFFFF_FFFF_FFFF_FFFF);\n\
           $finish; end endmodule\n");
    assert!(
        out.contains("[                   0]"),
        "zero stays 0:\n{out}"
    );
    assert!(
        out.contains("[                5000]"),
        "bare width 20:\n{out}"
    );
    assert!(out.contains("[5000]"), "%0t no pad:\n{out}");
    assert!(out.contains("[ 5000]"), "%5t:\n{out}");
    assert!(
        out.contains("[                          5000]"),
        "%30t:\n{out}"
    );
    assert!(
        out.contains("[               42000]"),
        "literal scales:\n{out}"
    );
    assert!(
        out.contains("[                X000]"),
        "x collapse + zeros:\n{out}"
    );
    assert!(
        out.contains("[                Z000]"),
        "z collapse + zeros:\n{out}"
    );
    assert!(
        out.contains("[18446744073709551615000]"),
        "u64::MAX × 1000 stays exact (textual zeros):\n{out}"
    );
}

#[test]
fn timeformat_basic_and_width_override() {
    // $timeformat(-9, 2, " ns", 10): "   5.00 ns" (min width 10 including the
    // suffix). %0t/%15t override the min width. $realtime matches $time.
    let (out, _, _) = run("`timescale 1ns/1ps\n\
         module t; initial begin\n\
           $timeformat(-9, 2, \" ns\", 10);\n\
           #5 $display(\"[%t]\", $time);\n\
           $display(\"[%t]\", $realtime);\n\
           $display(\"[%0t]\", $time);\n\
           $display(\"[%15t]\", $time);\n\
           $finish; end endmodule\n");
    assert!(out.contains("[   5.00 ns]"), "min width 10:\n{out}");
    assert_eq!(
        out.matches("[   5.00 ns]").count(),
        2,
        "$realtime == $time:\n{out}"
    );
    assert!(out.contains("[5.00 ns]"), "%0t overrides:\n{out}");
    assert!(out.contains("[        5.00 ns]"), "%15t overrides:\n{out}");
}

#[test]
fn timeformat_is_runtime_last_wins() {
    // A %t BEFORE the $timeformat call uses the defaults; each later call
    // re-formats every later %t (a static/SimOpts lowering would silently
    // reformat the first display — the reason this is a runtime statement).
    let (out, _, _) = run("`timescale 1ns/1ps\n\
         module t; initial begin\n\
           $display(\"[%t]\", $time);\n\
           $timeformat(-9, 2, \" ns\", 10);\n\
           $display(\"[%t]\", $time);\n\
           $timeformat(-12, 0, \"ps\", 8);\n\
           #3 $display(\"[%t]\", $time);\n\
           $finish; end endmodule\n");
    assert!(
        out.contains("[                   0]"),
        "pre-call default:\n{out}"
    );
    assert!(out.contains("[   0.00 ns]"), "first format:\n{out}");
    assert!(out.contains("[  3000ps]"), "second format wins:\n{out}");
}

#[test]
fn timeformat_units_and_precision_edges() {
    // prec beyond the tick resolution zero-fills; units FINER than the global
    // precision multiply (−15 in a ps design → ×1000); coarse units divide down
    // to 0.x. All unclamped arithmetic (iverilog-pinned).
    let (out, _, _) = run("`timescale 1ns/1ps\n\
         module t; initial begin\n\
           #5 $timeformat(-9, 5, \"n\", 3);\n\
           $display(\"[%t]\", $time);\n\
           $timeformat(-15, 0, \"fs\", 1);\n\
           $display(\"[%t]\", $time);\n\
           $timeformat(0, 1, \"s\", 1);\n\
           $display(\"[%t]\", $time);\n\
           $finish; end endmodule\n");
    assert!(out.contains("[5.00000n]"), "prec zero-fill:\n{out}");
    assert!(out.contains("[5000000fs]"), "finer units multiply:\n{out}");
    assert!(out.contains("[0.0s]"), "coarse units:\n{out}");
}

#[test]
fn t_integer_truncates_never_rounds() {
    // The INTEGER path is exact decimal-string math with TRUNCATION at the
    // precision digit (iverilog-pinned: 5.5→5, 6.5→6, 9.995@1→9.9, 9.999@2→9.99).
    let (out, _, _) = run("`timescale 1ns/1ps\n\
         module t; initial begin\n\
           $timeformat(-6, 0, \"\", 1);\n\
           $display(\"[%0t][%0t][%0t]\", 64'd5500, 64'd5499, 64'd6500);\n\
           $timeformat(-6, 1, \"\", 1);\n\
           $display(\"[%0t][%0t][%0t]\", 64'd5550, 64'd5549, 64'd9995);\n\
           $timeformat(-6, 2, \"\", 1);\n\
           $display(\"[%0t]\", 64'd9999);\n\
           $finish; end endmodule\n");
    assert!(out.contains("[5][5][6]"), "prec 0 truncates:\n{out}");
    assert!(out.contains("[5.5][5.5][9.9]"), "prec 1 truncates:\n{out}");
    assert!(out.contains("[9.99]"), "prec 2 truncates:\n{out}");
}

#[test]
fn t_real_path_rounds_like_percent_f() {
    // REAL args ($realtime, real literals) go through f64 and round like %f:
    // t=5.556ns @ prec 2 → "5.56" (the integer path would truncate to 5.55).
    let (out, _, _) = run("`timescale 1ns/1ps\n\
         module t; initial begin\n\
           #5 #0 $timeformat(-9, 2, \"\", 0);\n\
           $display(\"[%t]\", $realtime);\n\
           #0.5555 $display(\"[%t]\", $realtime);\n\
           $timeformat(-12, 0, \"p\", 1);\n\
           $display(\"[%0t]\", 2.75);\n\
           $finish; end endmodule\n");
    assert!(out.contains("[5.00]"), "real base:\n{out}");
    assert!(out.contains("[5.56]"), "real rounds:\n{out}");
    assert!(
        out.contains("[2750p]"),
        "real literal scales exactly:\n{out}"
    );
}

#[test]
fn t_xz_with_timeformat_keeps_collapse_char() {
    // With a zero net shift the collapsed x/z char stays and the fraction
    // zero-fills ("X.00 ns" / "z.00 ns"); a huge known value stays exact.
    let (out, _, _) = run("`timescale 1ns/1ps\n\
         module t; initial begin\n\
           $timeformat(-9, 2, \" ns\", 10);\n\
           $display(\"[%t]\", 4'bxx10);\n\
           $display(\"[%t]\", 4'bzzzz);\n\
           $display(\"[%t]\", 64'hFFFF_FFFF_FFFF_FFFF);\n\
           $finish; end endmodule\n");
    assert!(out.contains("[   X.00 ns]"), "mixed x collapse:\n{out}");
    assert!(out.contains("[   z.00 ns]"), "uniform z lowercase:\n{out}");
    assert!(
        out.contains("[18446744073709551615.00 ns]"),
        "u64::MAX exact at ratio 1:\n{out}"
    );
}

#[test]
fn t_xz_scale_down_goes_numeric() {
    // Once the shift cuts INTO the digits the unknown bits clear to 0 and the
    // value goes numeric (iverilog live: xx10 @ −6 in a ns module → "0.00us").
    let (out, _, _) = run("`timescale 1ns/1ps\n\
         module t; initial begin\n\
           $timeformat(-6, 2, \"us\", 10);\n\
           $display(\"[%t]\", 4'bxx10);\n\
           $display(\"[%t]\", 16'd5000);\n\
           $finish; end endmodule\n");
    assert!(
        out.contains("[    0.00us]"),
        "x clears to 0 numeric:\n{out}"
    );
    assert!(out.contains("[    5.00us]"), "known scale-down:\n{out}");
}

#[test]
fn timeformat_arity_is_loud() {
    // iverilog: "$timeformat requires zero or four arguments." 1–3 args must be
    // a loud compile error, never accept-and-guess.
    for src in [
        "module t; initial begin $timeformat(-9); end endmodule\n",
        "module t; initial begin $timeformat(-9, 2); end endmodule\n",
        "module t; initial begin $timeformat(-9, 2, \"ns\"); end endmodule\n",
    ] {
        let (_, err, code) = run(src);
        assert_ne!(code, 0, "1-3 args must fail loud");
        assert!(
            err.contains("zero or four arguments"),
            "arity diagnostic; got:\n{err}"
        );
    }
}

#[test]
fn timeformat_zero_args_resets_defaults() {
    // `$timeformat;` resets to the defaults (units = precision, width 20).
    let (out, _, _) = run("`timescale 1ns/1ps\n\
         module t; initial begin\n\
           $timeformat(-9, 2, \"n\", 8);\n\
           #5 $display(\"[%0t]\", $time);\n\
           $timeformat;\n\
           $display(\"[%t]\", $time);\n\
           $finish; end endmodule\n");
    assert!(out.contains("[5.00n]"), "set:\n{out}");
    assert!(
        out.contains("[                5000]"),
        "reset to defaults:\n{out}"
    );
}

#[test]
fn timeformat_runtime_variable_args() {
    // Args are RUNTIME expressions — a variable units / string-net suffix is
    // legal and honored (iverilog-pinned: "    5.00ns").
    let (out, _, _) = run("`timescale 1ns/1ps\n\
         module t; int u; string sfx; initial begin\n\
           u = -9; sfx = \"ns\";\n\
           $timeformat(u, 2, sfx, 10);\n\
           #5 $display(\"[%t]\", $time);\n\
           $finish; end endmodule\n");
    assert!(out.contains("[    5.00ns]"), "runtime args honored:\n{out}");
}

#[test]
fn timeformat_unclamped_units_neg_prec_minw() {
    // Units beyond the IEEE table work arithmetically (−16 → ×10, 2 → ÷1e11);
    // a negative precision clamps to 0; a NEGATIVE min width LEFT-justifies in
    // |width| (iverilog-pinned: "[5n ]").
    let (out, _, _) = run("`timescale 1ns/1ps\n\
         module t; initial begin\n\
           $timeformat(-16, 2, \"?\", 5);\n\
           #5 $display(\"[%t]\", $time);\n\
           $timeformat(2, 0, \"h\", 5);\n\
           $display(\"[%t]\", $time);\n\
           $timeformat(-9, -1, \"n\", -3);\n\
           $display(\"[%t]\", $time);\n\
           $finish; end endmodule\n");
    assert!(out.contains("[50000000.00?]"), "units −16:\n{out}");
    assert!(out.contains("[   0h]"), "units 2:\n{out}");
    assert!(out.contains("[5n ]"), "neg minw left-justifies:\n{out}");
}

#[test]
fn t_scales_per_displaying_module() {
    // Mixed `timescale`s: each %t scales by ITS module's unit (the 1us module's
    // $time=2 → 2000000 ticks; the 1ns module's 5000 → 5000000).
    let (out, _, _) = run("`timescale 1us/1ns\n\
         module m2(); initial #2 $display(\"m2 [%t]\", $time); endmodule\n\
         `timescale 1ns/1ps\n\
         module t; m2 u2(); initial begin #5000 $display(\"t [%t]\", $time); end endmodule\n");
    assert!(
        out.contains("m2 [             2000000]"),
        "1us module:\n{out}"
    );
    assert!(
        out.contains("t [             5000000]"),
        "1ns module:\n{out}"
    );
}

#[test]
fn t_packed_vector_suffix() {
    // The suffix arg %s-coerces: a packed 8'h6E renders "n" (iverilog-pinned).
    let (out, _, _) = run("`timescale 1ns/1ps\n\
         module t; initial begin\n\
           $timeformat(-9, 2, 8'h6E, 5);\n\
           #5 $display(\"[%t]\", $time);\n\
           $finish; end endmodule\n");
    assert!(out.contains("[5.00n]"), "packed suffix:\n{out}");
}

#[test]
fn strobe_renders_with_live_format_at_flush() {
    // A $strobe registered BEFORE the $timeformat call flushes at Postponed —
    // AFTER the call — and must render with the NEW state (iverilog-pinned:
    // both lines "[ 5.0ns]"). The display prints first (Active region).
    let (out, _, _) = run("`timescale 1ns/1ps\n\
         module t; int a=1; initial begin\n\
           #5 $strobe(\"s [%t] a=%0d\", $time, a);\n\
           $timeformat(-9, 1, \"ns\", 6);\n\
           $display(\"d [%t]\", $time);\n\
           $finish; end endmodule\n");
    assert!(out.contains("d [ 5.0ns]"), "display after call:\n{out}");
    assert!(
        out.contains("s [ 5.0ns]"),
        "strobe uses flush-time state:\n{out}"
    );
    let d = out.find("d [").unwrap();
    let s = out.find("s [").unwrap();
    assert!(d < s, "Active display before Postponed strobe:\n{out}");
}

#[test]
fn t_negative_value_scale_up() {
    // A negative time value keeps its sign through the textual scale-up
    // ("-5" × 1000 → "-5000", iverilog-pinned). The scale-DOWN case is pinned
    // to vita's sign-aware "-0.00u" — iverilog 13.0 prints a corrupt "0.0-u"
    // there (self-inconsistent output, so vita targets the sensible form).
    let (out, _, _) = run("`timescale 1ns/1ps\n\
         module t; integer neg; initial begin\n\
           neg = -5;\n\
           $display(\"[%0t]\", neg);\n\
           $timeformat(-6, 2, \"u\", 1);\n\
           $display(\"[%0t]\", neg);\n\
           $finish; end endmodule\n");
    assert!(out.contains("[-5000]"), "sign survives scale-up:\n{out}");
    assert!(out.contains("[-0.00u]"), "sign-aware scale-down:\n{out}");
}

#[test]
fn t_zero_flag_zero_pads() {
    // `%0Nt` zero-fills the field; the zeros go before even a minus sign
    // (iverilog-pinned: unlike the sign-aware `%0Nd`). R1 differential caught
    // the flag being dropped (space-pad) — this pins the fix.
    let (out, _, _) = run("`timescale 1ns/1ps\n\
         module t; integer neg; initial begin\n\
           $display(\"|%08t|\", 64'd5000);\n\
           $display(\"|%8t|\", 64'd5000);\n\
           neg = -1;\n\
           $display(\"|%08t|\", neg);\n\
           $display(\"|%012t|\", 64'd0);\n\
           $finish; end endmodule\n");
    assert!(out.contains("|05000000|"), "%08t zero-fills:\n{out}");
    assert!(out.contains("| 5000000|"), "%8t space-fills:\n{out}");
    assert!(out.contains("|000-1000|"), "zeros before the sign:\n{out}");
    assert!(
        out.contains("|000000000000|"),
        "zero value zero-fills:\n{out}"
    );
}

#[test]
fn t_wide_timescale_ratio_multiplier_is_u64() {
    // `timescale 1s/1ps` ⇒ M = 10^12 — the old u32 proc_multipliers SATURATED
    // at u32::MAX (a non-power-of-10) making `$time` and `%t` off by decades
    // (R1 soundness CRITICAL; also a pre-existing `$time` bug). iverilog-pinned.
    let (out, _, _) = run("`timescale 1s/1ps\n\
         module t; initial begin\n\
           #2 $display(\"[%0t] t=%0d\", $time, $time);\n\
           $timeformat(-3, 1, \"ms\", 1);\n\
           $display(\"[%0t]\", $time);\n\
           $finish; end endmodule\n");
    assert!(out.contains("[2000000000000] t=2"), "M=10^12 exact:\n{out}");
    assert!(
        out.contains("[2000.0ms]"),
        "rescale from 10^12 ticks:\n{out}"
    );
}

#[test]
fn timeformat_as_deferred_assert_action_is_loud() {
    // The §16.4 push_stmt hook captures EVERY SysTask under `cur_defer` into
    // `defer_acts`, and the engine's try_defer intercept runs BEFORE the
    // timeformat intercept — the call would silently print its args at
    // maturation instead of updating the %t state. Loud-reject (R1 soundness).
    let (_, err, code) = run("module t; logic ok=0; initial begin\n\
           assert #0 (ok) else $timeformat(-9, 2, \" ns\", 10);\n\
         end endmodule\n");
    assert_ne!(code, 0, "deferred-action $timeformat must fail loud");
    assert!(
        err.contains("deferred-assertion action is unsupported"),
        "diagnostic text; got:\n{err}"
    );
}

#[test]
fn t_inside_sformatf_shares_the_renderer() {
    // %t inside $sformatf goes through the same render funnel (state + scaling).
    let (out, _, code) = run("`timescale 1ns/1ps\n\
         module t; string s; initial begin\n\
           $timeformat(-9, 1, \"n\", 1);\n\
           #5 s = $sformatf(\"%0t\", $time);\n\
           $display(\"got=%s\", s);\n\
           if (s != \"5.0n\") $fatal(1, \"sformatf %%t mismatch\");\n\
           $finish; end endmodule\n");
    assert_eq!(code, 0, "clean exit:\n{out}");
    assert!(out.contains("got=5.0n"), "sformatf shares %t:\n{out}");
}

#[test]
fn staged_timeformat_parity() {
    // vcmp→velab→vrun must carry the `timeformat_stmts` sidecar (16th-field
    // append in the extra-sidecars trailer) and the precision exponent. If the
    // sidecar dropped, `$timeformat` would PRINT its args as a bare Display and
    // the $sformatf comparison would see the default-format "5000" → $fatal.
    let src = "`timescale 1ns/1ps\n\
         module t; string s; initial begin\n\
           $timeformat(-9, 1, \"n\", 1);\n\
           #5 s = $sformatf(\"%0t\", $time);\n\
           if (s != \"5.0n\") $fatal(1, \"staged timeformat sidecar dropped\");\n\
           $finish;\n\
         end endmodule\n";
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("vita_tfst_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let sv = dir.join("t.sv");
    std::fs::write(&sv, src).unwrap();
    let s = |p: &std::path::Path| p.to_str().unwrap().to_string();
    let vu = dir.join("t.vu");
    let velab = dir.join("t.velab");
    let o = cli::VitaOpts::default();
    assert_eq!(
        cli::run_vcmp(&[s(&sv)], Some(&s(&vu)), &o),
        0,
        "vcmp failed"
    );
    assert_eq!(cli::run_velab(&s(&vu), &s(&velab), &o), 0, "velab failed");
    assert_eq!(
        cli::run_vrun(&s(&velab), &o),
        0,
        "staged $timeformat dropped (timeformat_stmts sidecar or precision exp)"
    );
}
