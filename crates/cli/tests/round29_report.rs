//! External report round-29 (2026-08-18) — diagnostic quality, no value defects.
//!
//! The reporter ran the whole regression corpus on the new default backend and found
//! **zero wrong values** (4 testbenches, 105 guards, 793 PASS on an unlimited sweep, and
//! `native`/`vm`/`interp` byte-identical across 219 scenarios). What they found instead
//! is that `correct-or-loud` says nothing about whether the LOUD half is usable, and on
//! three counts it was not:
//!
//! - **R29-1** every syntax error rendered the `found` token with Rust's `Debug`, so a
//!   user-facing message carried vita's own lexer enum (`found Word(Keyword(End))` for
//!   `end`). The reader had to translate it back to source; a log consumer had to know
//!   vita's token type. One site, and the spelling was recoverable from the span all
//!   along.
//! - **R29-2** the clocking `default input/output` skew item — the first line of most
//!   real clocking blocks — was rejected by a message that (a) put a whole sentence in
//!   the `expected {X}` slot so it did not read as English, (b) leaked the token as
//!   above, and (c) named `default skew only` as the road to take while rejecting every
//!   `default …` spelling. The item is now PARSED and STAMPED onto the skew-less items,
//!   so the one predicate that decides which skews this subset honours is elaborate's,
//!   and it names the skew and the signal that were actually written.
//! - **R29-4** runtime diagnostics printed no position of any kind, in a run where parse
//!   diagnostics printed `file:line:col`. The `sim_time` was on the Diagnostic already —
//!   every runtime emitter stamps it — and the renderer dropped it. The reporter's own
//!   design raises 8 `W4029` + 1 `W4007` across 11 files and 22 `unique case` sites; the
//!   time alone separates "X before reset" from "X in steady state".
//!
//! - **R29-3** a `unique`/`priority` violation and an RTL `$warning` were one diagnostic
//!   code, because the parser desugars the violation arm into a literal `$warning`
//!   statement. So `-Wno-W-RUN-USER-WARNING` could not silence one known-benign
//!   unique-case without silencing every `$warning` in the tree, and the CI gate this
//!   repo's own doc-15 prescribes (`-Werror=W-RUN-USER-WARNING`) failed builds that
//!   contain no `$warning` at all. IEEE puts the two in different clauses — §12.5.3 is a
//!   report the SIMULATOR produces, §20.10 is a task the DESIGN called.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_args(src: &str, args: &[&str]) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_r29_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("t.sv"), src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .args(args)
        .arg("t.sv")
        .current_dir(&d)
        .output()
        .expect("run vita");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = std::fs::remove_dir_all(&d);
    (text, out.status.code())
}

fn run(src: &str) -> (String, Option<i32>) {
    run_args(src, &[])
}

// ─────────────────────────── R29-1 · token spelling ───────────────────────────

/// The three shapes from the report, plus the invariant that covers the rest: no
/// diagnostic may contain a Rust type name. `Word(`/`Keyword(` are the two the
/// report saw, and they are the ones a `{:?}` on `TokenKind` produces.
#[test]
fn a_syntax_error_never_prints_a_rust_token_debug() {
    for (what, src) in [
        ("Semi", "module m; logic a; initial a = ; endmodule\n"),
        (
            "Eq",
            "module m; logic [3:0] a; initial a[b = 1; endmodule\n",
        ),
        ("Keyword", "module m; initial begin if end end endmodule\n"),
    ] {
        let (o, _) = run(src);
        assert!(
            o.contains("error[VITA-E2002]"),
            "{what}: expected a parse error:\n{o}"
        );
        assert!(
            !o.contains("Word(") && !o.contains("Keyword(") && !o.contains("Ident)"),
            "{what}: a user-facing message leaked vita's token enum:\n{o}"
        );
    }
}

/// …and it says the right thing, not merely a non-Rust thing. Each row is a
/// different arm of the renderer: a punctuation token quotes bare, a keyword and
/// an identifier are NAMED because for those the bare spelling does not say why
/// it is wrong (`end` alone reads like a name).
#[test]
fn the_found_token_is_quoted_as_the_user_spelled_it() {
    for (src, want) in [
        ("module m; logic a; initial a = ; endmodule\n", "found ';'"),
        (
            "module m; logic [3:0] a; initial a[b = 1; endmodule\n",
            "found '='",
        ),
        (
            "module m; initial begin if end end endmodule\n",
            "found keyword 'end'",
        ),
        (
            "module m; logic a; initial a = 1; endmodule extra\n",
            "found identifier 'extra'",
        ),
    ] {
        let (o, _) = run(src);
        assert!(o.contains(want), "expected `{want}` in:\n{o}");
    }
}

/// End of file keeps its own wording — that arm was already right and the token
/// path must not swallow it (there is no span to slice).
#[test]
fn an_unterminated_unit_still_says_end_of_file() {
    let (o, _) = run("module m;\n  initial begin\n");
    assert!(o.contains("found end of file"), "{o}");
}

/// The anchor for the split between `span` (where to report) and `found_span`
/// (what to quote): `error_at` reports at an EARLIER node than the cursor, so a
/// renderer that sliced the report anchor would quote a token that is not the
/// one in `found`. Here the anchor is the `w` inside `g[w]` and the cursor is
/// the `.` after `]`.
#[test]
fn a_report_anchored_earlier_still_quotes_the_token_it_found() {
    let (o, _) = run("module sub; logic q; endmodule\n\
         module m;\n\
           logic w;\n\
           genvar i;\n\
           generate for (i=0;i<2;i=i+1) begin : g sub u(); end endgenerate\n\
           initial w = g[w].u.q;\n\
         endmodule\n");
    assert!(
        o.contains("found '.'"),
        "`found` must name the cursor token, not the report anchor:\n{o}"
    );
    assert!(!o.contains("Dot"), "and not the enum:\n{o}");
}

// ──────────────────────── R29-2 · clocking default skew ───────────────────────

/// The block-wide default is not a second skew mechanism: it is stamped onto the
/// items that declared no skew, so `default input #1step;` runs exactly as the
/// per-signal spelling does. PRE: a hard parse error for every `default …` line.
#[test]
fn a_default_input_skew_item_is_accepted_and_runs() {
    let (o, ok) = run(
        "module m;\n\
           logic clk, d;\n\
           clocking cb @(posedge clk);\n\
             default input #1step;\n\
             input d;\n\
           endclocking\n\
           initial begin clk=0; d=0; #1 d=1; #1 clk=1; #1 $display(\"cb.d=%0d\", cb.d); $finish; end\n\
         endmodule\n",
    );
    assert!(o.contains("cb.d=1"), "the sampled value must arrive:\n{o}");
    assert_eq!(ok, Some(0), "and cleanly:\n{o}");
}

/// The output half, and the reason the stamp runs over the WHOLE item list
/// rather than the items that follow the `default` textually: a `default` is a
/// property of the block. Declaring it after the signal must behave the same.
#[test]
fn a_default_output_skew_applies_to_an_item_declared_before_it() {
    let (o, ok) = run("module m;\n\
           logic clk; logic q;\n\
           clocking cb @(posedge clk);\n\
             output q;\n\
             default output #1step;\n\
           endclocking\n\
           initial begin clk=0; #1 cb.q <= 1; #1 clk=1; #1 $display(\"q=%0d\", q); $finish; end\n\
         endmodule\n");
    assert!(o.contains("q=1"), "the driven value must arrive:\n{o}");
    assert_eq!(ok, Some(0), "and cleanly:\n{o}");
}

/// An unsupported skew stays loud — but the message now names the SKEW as the
/// user wrote it (`#0`, not `0`) and the SIGNAL it landed on. Both matter: a
/// block-wide default puts one written skew on several items, so "which signal"
/// is not recoverable by reading the source.
#[test]
fn an_unsupported_skew_names_the_skew_and_the_signal() {
    let (o, ok) = run("module m;\n\
           logic clk, d;\n\
           clocking cb @(posedge clk);\n\
             input #0 d;\n\
           endclocking\n\
           initial #1 $finish;\n\
         endmodule\n");
    assert!(o.contains("error[VITA-E3009]"), "{o}");
    assert!(o.contains("clocking skew `#0` on `d`"), "{o}");
    assert_ne!(ok, Some(0), "and it must not be clean:\n{o}");
}

/// …and it no longer points at a road that does not exist. PRE said "`#1step` is
/// accepted as the explicit default", which — read next to a rejected `default
/// …;` item — is an instruction to write the thing that was just refused.
#[test]
fn the_skew_message_names_the_per_signal_spelling_not_a_rejected_one() {
    let (o, _) = run("module m;\n\
           logic clk, d;\n\
           clocking cb @(posedge clk);\n\
             input #2 d;\n\
           endclocking\n\
           initial #1 $finish;\n\
         endmodule\n");
    assert!(
        o.contains("written per signal"),
        "the message must say WHERE the accepted spelling goes:\n{o}"
    );
    assert!(
        !o.contains("accepted as the explicit default"),
        "and must not name the rejected road:\n{o}"
    );
}

/// A bare `default #1step;` is a syntax error in the LANGUAGE — IEEE 1800-2017
/// §14.3's `default_skew` has no direction-less production — and PRE rejected it
/// with a message that claimed the opposite ("default skew only"). The reporter
/// tried it BECAUSE of that message.
#[test]
fn a_direction_less_default_names_the_legal_forms() {
    let (o, ok) = run("module m;\n\
           logic clk, d;\n\
           clocking cb @(posedge clk);\n\
             default #1step;\n\
             input d;\n\
           endclocking\n\
           initial #1 $finish;\n\
         endmodule\n");
    assert!(
        o.contains("`input` or `output` after `default`"),
        "the message must say what is missing:\n{o}"
    );
    assert!(
        o.contains("default input SKEW"),
        "and show the legal shape:\n{o}"
    );
    assert!(
        !o.contains("default skew only"),
        "the PRE wording pointed at a road that does not exist:\n{o}"
    );
    assert_ne!(ok, Some(0), "{o}");
}

/// `default input` with no skew is the other half of that production.
#[test]
fn a_default_with_no_skew_is_loud() {
    let (o, ok) = run("module m;\n\
           logic clk, d;\n\
           clocking cb @(posedge clk);\n\
             default input;\n\
             input d;\n\
           endclocking\n\
           initial #1 $finish;\n\
         endmodule\n");
    assert!(o.contains("a skew (`#…`) after `default input`"), "{o}");
    assert_ne!(ok, Some(0), "{o}");
}

/// Two `default` items have no defined reading, so the second is loud — and it
/// is reported BEFORE the keyword is consumed, so the offending token in the
/// message is the second `default` itself rather than whatever follows it.
#[test]
fn a_second_default_item_is_loud_at_its_own_keyword() {
    let (o, ok) = run("module m;\n\
           logic clk, d;\n\
           clocking cb @(posedge clk);\n\
             default input #1step;\n\
             default input #1step;\n\
             input d;\n\
           endclocking\n\
           initial #1 $finish;\n\
         endmodule\n");
    assert!(o.contains("at most one `default` item"), "{o}");
    assert!(
        o.contains("found keyword 'default'"),
        "the message must name the second `default`, not the token after it:\n{o}"
    );
    assert_ne!(ok, Some(0), "{o}");
}

/// A per-signal skew WINS over the block default — that is what "stamped onto the
/// items that declared no skew of their own" means, and without this row the
/// stamp could overwrite and no test would see it (both spellings are `#1step`
/// in the passing cases above).
#[test]
fn a_per_signal_skew_overrides_the_block_default() {
    let (o, ok) = run(
        "module m;\n\
           logic clk, d;\n\
           clocking cb @(posedge clk);\n\
             default input #0;\n\
             input #1step d;\n\
           endclocking\n\
           initial begin clk=0; d=0; #1 d=1; #1 clk=1; #1 $display(\"cb.d=%0d\", cb.d); $finish; end\n\
         endmodule\n",
    );
    assert!(
        o.contains("cb.d=1"),
        "the item's own `#1step` must win over the block's `#0`:\n{o}"
    );
    assert_eq!(ok, Some(0), "and the rejected default must not fire:\n{o}");
}

/// …and the converse: an item with NO skew of its own takes the block default,
/// including when that default is one the subset refuses. Without this row the
/// stamp could be a no-op and every test above would still pass.
#[test]
fn an_unskewed_item_takes_a_rejected_block_default() {
    let (o, ok) = run("module m;\n\
           logic clk, d;\n\
           clocking cb @(posedge clk);\n\
             default input #0;\n\
             input d;\n\
           endclocking\n\
           initial #1 $finish;\n\
         endmodule\n");
    assert!(
        o.contains("clocking skew `#0` on `d`"),
        "the block default must reach the item:\n{o}"
    );
    assert_ne!(ok, Some(0), "{o}");
}

// ────────────────────── R29-4 · runtime diagnostics say when ──────────────────

/// The report's shape: two runtime warnings with no position of any kind, in a
/// run whose parse diagnostics carry `file:line:col`. The engine works on IR and
/// has no span, but it does know the time — and every runtime emitter already
/// stamped it.
#[test]
fn a_runtime_diagnostic_says_when_it_fired() {
    let (o, ok) = run("module m;\n\
           logic [1:0] s; logic [3:0] mem [0:3]; logic [1:0] ix; logic [3:0] r;\n\
           initial begin\n\
             #7;\n\
             s = 2'b11;\n\
             unique case (s) 2'b00: ; 2'b01: ; endcase\n\
             ix = 2'bx1;\n\
             r = mem[ix];\n\
             #1 $finish;\n\
           end\n\
         endmodule\n");
    // Two DIFFERENT runtime emitters — the severity path (W4031, via the
    // violation desugar) and the range path (W4029) — so this also pins that the
    // stamp is not a property of one call site.
    assert!(
        o.contains("warning[VITA-W4031]") && o.contains("warning[VITA-W4029]"),
        "both runtime warnings must fire:\n{o}"
    );
    // t=7, not 0 — a timestamp that is always 0 would pass a weaker assertion
    // while carrying no information.
    assert!(
        o.lines()
            .filter(|l| l.contains("VITA-W4031") || l.contains("VITA-W4029"))
            .all(|l| l.contains("[at time 7]")),
        "every runtime diagnostic must carry the time it fired:\n{o}"
    );
    assert_eq!(ok, Some(0), "{o}");
}

/// The same warning at two different times is two distinguishable lines — which
/// is the whole point. PRE printed the identical text both times, so a design
/// with many indexed arrays reported N identical lines and the reader could not
/// tell reset from steady state.
#[test]
fn the_same_warning_at_two_times_is_two_distinguishable_lines() {
    let (o, _) = run("module m;\n\
           logic [3:0] mem [0:3]; logic [1:0] ix; logic [3:0] r;\n\
           initial begin\n\
             ix = 2'bx1;\n\
             #3 r = mem[ix];\n\
             #9 r = mem[ix];\n\
             #1 $finish;\n\
           end\n\
         endmodule\n");
    assert!(o.contains("[at time 3]"), "{o}");
    assert!(o.contains("[at time 12]"), "{o}");
}

/// A COMPILE-time diagnostic must not grow a timestamp: it has no sim time, and
/// stamping one (0, say) would be a fact about nothing. This is the control for
/// the row above — without it, "always append a time" would pass.
#[test]
fn an_elaborate_diagnostic_carries_no_time() {
    let (o, ok) = run("module m; initial x = 1; endmodule\n");
    assert!(o.contains("error[VITA-E3010]"), "{o}");
    assert!(
        !o.contains("[at time"),
        "a compile-time diagnostic has no simulation time to report:\n{o}"
    );
    assert_ne!(ok, Some(0), "{o}");
}

// ───────────── R29-3 · the violation report is not an RTL `$warning` ──────────

/// A design with BOTH: the reporter's exact shape. Two diagnostics, two codes.
#[test]
fn a_unique_violation_and_an_rtl_warning_have_different_codes() {
    const SRC: &str = "module m;\n\
           logic [1:0] s; int depth = 14;\n\
           initial begin\n\
             s = 2'b11;\n\
             unique case (s) 2'b00: ; 2'b01: ; endcase\n\
             $warning(\"rtl warning: depth=%0d\", depth);\n\
             #1 $finish;\n\
           end\n\
         endmodule\n";
    let (o, ok) = run(SRC);
    assert!(
        o.contains("warning[VITA-W4031]"),
        "the violation report:\n{o}"
    );
    assert!(o.contains("warning[VITA-W4007]"), "the RTL $warning:\n{o}");
    assert_eq!(ok, Some(0), "{o}");

    // …and each `-Wno-` reaches exactly one of them. PRE: either flag took both.
    let (a, _) = run_args(SRC, &["-Wno-W-RUN-USER-WARNING"]);
    assert!(
        a.contains("VITA-W4031") && !a.contains("VITA-W4007"),
        "-Wno- on the user warning must leave the violation report:\n{a}"
    );
    let (b, _) = run_args(SRC, &["-Wno-W-RUN-UNIQUE-VIOLATION"]);
    assert!(
        b.contains("VITA-W4007") && !b.contains("VITA-W4031"),
        "-Wno- on the violation must leave the RTL $warning:\n{b}"
    );
}

/// The other direction, and the one that broke a real CI: `-Werror` on the RTL
/// `$warning` code failed a design containing NO `$warning`. The report measured
/// `errors=2` on exactly this shape.
#[test]
fn werror_on_user_warning_does_not_fail_a_design_with_no_user_warning() {
    const SRC: &str = "module m;\n\
           logic [1:0] s;\n\
           initial begin\n\
             s = 2'b11;\n\
             unique case (s) 2'b00: ; 2'b01: ; endcase\n\
             #1 $finish;\n\
           end\n\
         endmodule\n";
    let (o, ok) = run_args(SRC, &["-Werror=W-RUN-USER-WARNING"]);
    assert_eq!(ok, Some(0), "no `$warning` in this design:\n{o}");
    assert!(o.contains("warning[VITA-W4031]"), "{o}");

    // …while the violation's own promotion still works — otherwise the fix would
    // just be "the gate never fires", which is a different bug.
    let (p, pok) = run_args(SRC, &["-Werror=W-RUN-UNIQUE-VIOLATION"]);
    assert_ne!(pok, Some(0), "promotion must still bite:\n{p}");
    assert!(p.contains("error[VITA-W4031]"), "{p}");
}

/// `unique0`/`priority0` suppress the no-match check by design (IEEE §12.4.2), so
/// the new code must not fire for them. Control against "always report".
#[test]
fn the_zero_variants_report_no_violation() {
    let (o, ok) = run("module m;\n\
           logic [1:0] s;\n\
           initial begin\n\
             s = 2'b11;\n\
             unique0 case (s) 2'b00: ; 2'b01: ; endcase\n\
             priority0 case (s) 2'b00: ; endcase\n\
             #1 $finish;\n\
           end\n\
         endmodule\n");
    assert!(!o.contains("VITA-W4031"), "{o}");
    assert_eq!(ok, Some(0), "{o}");
}

/// The FRAME emitter is a second site that maps a severity class to a code, and
/// a `unique case` inside a function body is the only shape that reaches it.
/// Without this row, one of the two spellings could keep answering W4007.
#[test]
fn a_violation_inside_a_function_body_uses_the_same_code() {
    let (o, ok) = run("module m;\n\
           function automatic int f(input logic [1:0] s);\n\
             f = 0;\n\
             unique case (s) 2'b00: f = 1; 2'b01: f = 2; endcase\n\
           endfunction\n\
           initial begin $display(\"f=%0d\", f(2'b11)); #1 $finish; end\n\
         endmodule\n");
    assert!(o.contains("warning[VITA-W4031]"), "{o}");
    assert!(o.contains("f=0"), "{o}");
    assert_eq!(ok, Some(0), "{o}");
}

/// The desugar's channel to elaborate is the task NAME, so source that can write
/// that name can file a violation the design never violated. The whole
/// `$__vita_` namespace is reserved — a rule, not a list, so the next desugar is
/// covered without anyone remembering. Both entries (statement and expression)
/// must refuse it, or the channel is only half private.
#[test]
fn the_internal_desugar_namespace_cannot_be_written_in_source() {
    let (a, aok) = run("module m;\n\
           initial begin $__vita_unique_violation(\"hi\"); #1 $finish; end\n\
         endmodule\n");
    assert!(a.contains("error[VITA-E2002]"), "{a}");
    assert!(a.contains("reserved `$__vita_` namespace"), "{a}");
    assert!(
        !a.contains("VITA-W4031"),
        "source must not be able to file a violation report:\n{a}"
    );
    assert_ne!(aok, Some(0), "{a}");

    let (b, bok) = run("module m;\n\
           int x;\n\
           initial begin x = $__vita_secret(1); #1 $finish; end\n\
         endmodule\n");
    assert!(b.contains("reserved `$__vita_` namespace"), "{b}");
    assert_ne!(bok, Some(0), "{b}");

    // An ordinary `$`-name is untouched — the rule is a prefix, not "unknown".
    let (c, cok) = run("module m; initial begin $display(\"ok\"); #1 $finish; end endmodule\n");
    assert!(c.contains("ok") && !c.contains("reserved"), "{c}");
    assert_eq!(cok, Some(0), "{c}");
}
