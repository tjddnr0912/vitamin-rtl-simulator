//! `lower_expr_ctx` must stop at every NON-INTEGRAL DOMAIN route `lower_expr` has
//! (§4.5.354).
//!
//! `lower_expr_ctx` is a second, width-aware dispatch over the six
//! context-propagating node kinds, entered whenever the node carries a `'0`/`'1`
//! fill literal. It reimplemented those kinds and dropped every domain route its
//! `lower_expr` twin has, so **a fill literal anywhere in the node was a bypass of
//! all five** — the same construct lowered one way with a fill and another way
//! without:
//!
//! | shape                    | with a fill (PRE)               | the no-fill twin        |
//! |--------------------------|---------------------------------|-------------------------|
//! | `wire[11:0]={s,'1}`      | **never terminates**            | `{s,1'b1}` → correct    |
//! | `wire[23:0]={2{s,'1}}`   | `…0101` (string → bits)         | `{2{s,1'b1}}` → correct |
//! | `s < {"a",'1}`           | `0` (packed, not lexicographic) | `s < {"a",1'b1}` → `1`  |
//! | `h == '1`                | `0` at exit 0                   | `h == 1'b1` → E3009     |
//! | `{ {0{x}}, '1 }`         | **false-loud** E3009            | `{ {0{x}}, 1'b1 }` → ok |
//! | `N=-2; {N{'1}}`          | `111111111111` at exit 0        | `{N{1'b1}}` → E3009     |
//! | `r & '1` · `r << '1`     | `0` at exit 0 (§6.2 skipped)    | `r & 1'b1` → E3009      |
//! | `r ** '1`                | `0` (§11.4.9 `$pow` ROUTE lost) | `r ** 1'b1` → `3`       |
//! | `r + '1`                 | `0` (`ir_bits_of` said 64)      | `r + 1'b1` → `4`        |
//! | `c ? r : '1` (c=0)       | `0`                             | `c ? r : 1'b1` → `1`    |
//! | `case (r) '1:`           | falls to `default`              | `case (r) 1'b1:` matches|
//!
//! ⭐ THE FIX IS MOSTLY A DELETION. `Concat` and `Replicate` operands are
//! SELF-determined (§11.4.12), so those two ctx arms passed `ctx = 0` to every
//! operand — which is by definition what `lower_expr` already does. They were pure
//! duplication that had fallen behind the original (no `repl_zero_ok`, none of the
//! §11.4.12.2 count rules, no string route), so they are gone and the node falls to
//! `lower_expr` whole. Only the `Binary` comparison genuinely needed a guard, because
//! there the ctx arm does real work for the integral case: `binary_stops_ctx` is a
//! verbatim copy of that site's own handle and StrCmp conditions.
//!
//! One root under all ten: a string concat is a dynamic-width `$sformatf`, a handle
//! is an object id, and a real has no bit width at all — so a bit-width context has
//! nothing to say about any of them (IEEE §6.16 / §8.4 / §6.12). ⚠️ The last three
//! rows are the SAME TRAP §4.5.353 met one level up: a "how wide is it?" helper
//! answers a real's STORAGE size (64), and taking that as a context is nonsense.
//! `binary_real_operand_route` and `sibling_ctx` are the two halves of that fix, and
//! `sibling_ctx` has THREE callers because the same "how wide is my sibling?" question
//! is asked at a binary operand, a ternary branch, and a `case` selector. The `**` row
//! is a separate reminder: a block of "diagnostics" can hide a ROUTE.
//!
//! ⚠️ THE HANG WAS A MUTUAL TAIL CALL, WHICH IS WHY IT LOOKED LIKE A LIVELOCK RATHER
//! THAN A STACK BUG. `lower_expr_ctx`'s `Concat` arm handed the node back to
//! `lower_expr`, whose front gate sent it straight back. Both calls are in tail
//! position, so the release build spins at 100% CPU with a FLAT RSS (nothing grows to
//! tell you) and only the debug build overflows the stack. `run_bounded` below exists
//! so a regression fails with that sentence instead of hanging CI forever.
//!
//! ORACLES. The string-in-a-bit-concat family has NO external oracle — iverilog 13.0
//! dies with `internal error: vvp_fun_concat::recv_string not implemented` and
//! verilator 5.050 refuses to build (`V3Number.cpp: Number operation called with
//! non-logic argument`) — and that is true of `{s, 8'h0F}` too, which has no fill and
//! which vita has always run. So these values are pinned COMPOSITIONALLY, against two
//! spellings vita itself already answered before this slice: the statement-level
//! `string_concat_special` desugar (which never entered the ctx path) and the explicit
//! `1'b1`/`1'b0` twin. That composition is what makes them checkable at all: the rule
//! "a concat operand is self-determined, so a fill in one is exactly 1 bit" is pinned
//! by BOTH oracles on the string-free family below.
//!
//! The other two axes DO have oracles: `s < {"a",'1}` is `1` in iverilog and
//! verilator, and iverilog rejects `h == '1` ("Both arguments (class, logic) must be
//! class/null").
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn write_src(src: &str) -> std::path::PathBuf {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let p = std::env::temp_dir().join(format!("vita_cldr_{}_{n}.sv", std::process::id()));
    std::fs::write(&p, src).unwrap();
    p
}

/// Run vita on `src`, killing it after 60 s.
///
/// ⚠️ The timeout is INSURANCE, not the usual mechanism. These tests run a DEBUG
/// binary, where the cycle overflows the stack and aborts — so a regression normally
/// fails on `assert!(success)` within a second (measured: a mutant that restores the
/// cycle is killed that way). The timeout exists because the same cycle in a RELEASE
/// build is a true tail call that never dies: without it, a release-shaped regression
/// would hang the test binary forever with no output. 60 s is ~3 orders of magnitude
/// above what these one-line designs need.
fn run_bounded(src: &str) -> (bool, String) {
    let p = write_src(src);
    let mut child = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&p)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn vita");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    loop {
        match child.try_wait().expect("try_wait") {
            Some(_) => break,
            None if std::time::Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = std::fs::remove_file(&p);
                panic!(
                    "vita did not terminate within 60 s on:\n{src}\n\
                     The `lower_expr` ⇄ `lower_expr_ctx` mutual tail call is back — see \
                     `lower_expr_ungated`."
                );
            }
            None => std::thread::sleep(std::time::Duration::from_millis(20)),
        }
    }
    let out = child.wait_with_output().expect("wait");
    let _ = std::fs::remove_file(&p);
    let mut kept = String::new();
    for l in String::from_utf8_lossy(&out.stdout).lines() {
        if !l.starts_with("simulation ended") {
            kept.push_str(l);
            kept.push('\n');
        }
    }
    kept.push_str(&String::from_utf8_lossy(&out.stderr));
    (out.status.success(), kept)
}

fn ok(src: &str) -> String {
    let (success, all) = run_bounded(src);
    assert!(success, "expected success:\n{all}");
    all.lines()
        .filter(|l| !l.starts_with("errors=") && !l.contains("W-PP-TIMESCALE"))
        .fold(String::new(), |mut a, l| {
            a.push_str(l);
            a.push('\n');
            a
        })
}

fn loud(src: &str) -> String {
    let (success, all) = run_bounded(src);
    assert!(!success, "expected a loud reject, got success:\n{all}");
    all
}

// ─────────────────── (a) string concat: never terminated ───────────────────

#[test]
fn a_net_decl_init_string_concat_with_a_fill_terminates_and_matches_its_twin() {
    // PRE: never returned. The value is pinned against `{s, 1'b1}` in the SAME
    // design, so the assertion cannot drift with vita's string rendering.
    let o = ok("module t; string s = \"ab\";\n\
         wire [11:0] a = {s, '1};\n\
         wire [11:0] b = {s, 1'b1};\n\
         initial begin #1 $display(\"%b %b\", a, b); $finish; end endmodule\n");
    assert_eq!(o, "001000000001 001000000001\n");
}

#[test]
fn all_four_spellings_of_the_same_string_concat_agree() {
    // The decl-init, the continuous assign and the display arg all took the ctx path
    // and hung; the blocking assign went through `string_concat_special` and was
    // already right. That disagreement is what located the bug, so it is pinned.
    let o = ok("module t; string s = \"ab\";\n\
         wire  [11:0] a = {s, '1};\n\
         logic [11:0] b; assign b = {s, '1};\n\
         logic [11:0] c;\n\
         initial begin c = {s, '1}; #1 $display(\"%b %b %b %b\", a, b, c, {s, '1}); $finish; end\n\
         endmodule\n");
    assert_eq!(
        o,
        "001000000001 001000000001 001000000001 011000010110001000000001\n"
    );
}

#[test]
fn a_fill_on_either_side_of_the_string_terminates() {
    let o = ok("module t; string s = \"ab\";\n\
         wire [11:0] a = {'1, s};\n\
         wire [11:0] b = {s, '0};\n\
         wire [11:0] c = {s, 1'b0};\n\
         initial begin #1 $display(\"%b %b %b\", a, b, c); $finish; end endmodule\n");
    // `'0` and `1'b0` render identically — the fill is 1 bit, exactly like its twin.
    assert_eq!(o, "000101100010 001000100000 001000100000\n");
}

#[test]
fn a_fill_nested_under_paren_replicate_or_binary_inside_a_string_concat() {
    // Each of these reaches the `Concat` node through a DIFFERENT ctx arm, and every
    // one of them used to bounce.
    let o = ok("module t; string s = \"ab\";\n\
         wire [11:0] a = {s, ('1)};\n\
         wire [11:0] b = {s, {2{'1}}};\n\
         wire [11:0] c = {s, (1'b1 + '1)};\n\
         wire [15:0] d = {{s, '1}, 4'h0};\n\
         initial begin #1 $display(\"%b %b %b %b\", a, b, c, d); $finish; end endmodule\n");
    assert_eq!(
        o,
        "001000000001 001000000011 001000100000 0010000000010000\n"
    );
}

// ─────────────── (b) string replicate: the arm with no check at all ───────────────

#[test]
fn a_string_replicate_with_a_fill_keeps_the_string() {
    // PRE: `000000000000000000000101` — the string collapsed to a single bit and the
    // whole thing became `{2{1'b0, 1'b1}}` = 5. The `Concat` arm at least ATTEMPTED
    // the string route (and hung); `Replicate` had no check whatsoever, so this one
    // was silent-wrong at exit 0.
    let o = ok("module t; string s = \"ab\";\n\
         wire [23:0] a = {2{s, '1}};\n\
         wire [23:0] b = {2{s, 1'b1}};\n\
         initial begin #1 $display(\"%b %b\", a, b); $finish; end endmodule\n");
    assert_eq!(o, "011000010110001000000001 011000010110001000000001\n");
}

// ──────────────── (c) string compare: the StrCmp route, 2-oracle ────────────────

#[test]
fn a_string_compare_against_a_concat_with_a_fill_stays_lexicographic() {
    // ORACLE: iverilog 13.0 and verilator 5.050 both print `1`. vita printed `0`,
    // because the ctx path built a packed compare — which zero-extends on the MSB
    // side and is NOT lexicographic for unequal lengths. The no-fill twin was already
    // `1`, and that asymmetry is the whole finding.
    let o = ok("module t; string s = \"ab\";\n\
         initial begin #1 $display(\"%b %b\", s < {\"a\", '1}, s < {\"a\", 1'b1}); $finish; end\n\
         endmodule\n");
    assert_eq!(o, "1 1\n");
}

#[test]
fn every_string_comparison_matches_its_own_no_fill_twin() {
    // ⭐ WHAT IS PINNED HERE IS THE PAIRWISE AGREEMENT, NOT THE VALUE. PRE, each of
    // these six disagreed with the `1'b1` spelling written beside it, because only the
    // fill spelling was diverted off the StrCmp route.
    //
    // ⚠️ THE VALUE ITSELF IS NOT ALWAYS ORACLE-DECIDABLE HERE. `s <= 1'b1` is `1` in
    // iverilog and `0` in verilator — a genuine oracle SPLIT, on a shape this slice
    // does not touch (the twin reads the same PRE and POST). So this test asserts the
    // property the slice is responsible for, and the split stays a separate open
    // question rather than being silently decided by a value assertion here.
    //
    // Where the oracles DO agree they are pinned in the test above: `s < {"a",'1}` and
    // `s < {"a",'0}` are `1` in both, and PRE vita said `0`.
    //
    // The last pair puts the string on the RIGHT. `lower_expr`'s route fires on EITHER
    // operand, so a copy of it that only looked left would pass every other pair here.
    let o = ok("module t; string s = \"ab\";\n\
         initial begin #1 $display(\"%b%b %b%b %b%b %b%b %b%b %b%b\",\n\
           s <  '1, s <  1'b1,  s <= '1, s <= 1'b1,  s >  '1, s >  1'b1,\n\
           s >= '1, s >= 1'b1,  s == '1, s == 1'b1,  '1 < s,  1'b1 < s); $finish; end\n\
         endmodule\n");
    for (i, pair) in o.trim_end().split(' ').enumerate() {
        let b = pair.as_bytes();
        assert_eq!(
            b[0], b[1],
            "comparison {i}: the fill spelling read {} and its 1'b1 twin read {} — the \
             fill is off the StrCmp route again (whole line: {o})",
            b[0] as char, b[1] as char
        );
    }
}

// ──────────────── (d) handle compare: a fill was bypassing a LOUD gate ────────────

#[test]
fn a_class_handle_compared_to_a_fill_is_still_loud() {
    // ⚠️ This one is loud→silent, the worst direction on the ladder. `h == 1'b1` is
    // E3009 (iverilog: "Both arguments (class, logic) must be class/null for '=='"),
    // but `h == '1` skipped the gate entirely and printed a made-up `0` at exit 0.
    for src in [
        "class C; int x; endclass\n\
         module t; C h; initial begin h = new(); #1 $display(\"%b\", h == '1); $finish; end endmodule\n",
        "class C; int x; endclass\n\
         module t; C h; initial begin h = new(); #1 $display(\"%b\", h < '1); $finish; end endmodule\n",
    ] {
        let e = loud(src);
        assert!(
            e.contains("VITA-E3009") && e.contains("class handle"),
            "expected the handle gate, got:\n{e}"
        );
    }
}

// ──────── (e) the guard must NOT fire — everything it could over-reach into ────────

#[test]
fn a_fill_in_a_string_free_concat_is_untouched() {
    // ⭐ THIS is what pins "a concat operand is self-determined, so a fill is exactly
    // 1 bit" — and BOTH oracles agree on every value here. The compositional argument
    // for the string family above rests on this test.
    let o = ok("module t;\n\
         wire [11:0] a = {4'hA, '1};\n\
         wire [11:0] b = {'1, 4'hA};\n\
         wire [11:0] c = {'1, '1};\n\
         wire [11:0] d = {'1};\n\
         wire [11:0] e = {2{'1}};\n\
         initial begin #1 $display(\"%b %b %b %b %b\", a, b, c, d, e); $finish; end endmodule\n");
    assert_eq!(
        o,
        "000000010101 000000011010 000000000011 000000000001 000000000011\n"
    );
}

#[test]
fn a_string_concat_without_a_fill_is_untouched() {
    // The two packed ones never entered `lower_expr_ctx` at all (all five of its entry
    // points are fill-gated), so they are the control that says the fix changed only
    // fills. The third is the STRING-TARGET concat, and it is the strongest oracle in
    // this file: iverilog, verilator and vita all emit `r=ab\x01` BYTE FOR BYTE, which
    // pins the one step the rest of the string family has to take on faith — that a
    // 1-bit fill renders through `%s` as the single byte 0x01.
    //
    // ⚠️ Read that trailing byte. It is 0x01, not "nothing": comparing these four
    // outputs in a terminal shows `ab` for all of them whatever the last byte is, and
    // that is how this assertion was wrong on the first try.
    let o = ok("module t; string s = \"ab\"; string r;\n\
         wire [11:0] a = {s, 8'h0F};\n\
         wire [23:0] b = {2{s}};\n\
         initial begin r = {s, '1}; #1 $display(\"%b %b %s\", a, b, r); $finish; end endmodule\n");
    assert_eq!(o, "001000001111 011000100110000101100010 ab\u{1}\n");
}

#[test]
fn wildcard_equality_with_a_fill_is_not_a_domain_route() {
    // `==?`/`!=?` are intercepted BEFORE the handle and StrCmp routes in both
    // lowerings, so `ctx_stops_at_domain_route` deliberately answers false for them.
    // ORACLE: iverilog prints 0.
    let o = ok("module t; logic [7:0] x = 8'hF5;\n\
         initial begin #1 $display(\"%b\", x ==? '1); $finish; end endmodule\n");
    assert_eq!(o, "0\n");
}

#[test]
fn a_legal_handle_compare_still_compares() {
    // The guard answers TRUE for a legal handle compare too, not just the loud one —
    // so this pins that routing it to the ungated lowering did not break it.
    let o = ok("class C; int x; endclass\n\
         module t; C h; initial begin h = new();\n\
         #1 $display(\"%b %b\", h == null, h != null); $finish; end endmodule\n");
    assert_eq!(o, "0 1\n");
}

// ──── (f) the two the arms were ALSO getting wrong, fixed by deleting them ────

#[test]
fn a_zero_replication_beside_a_fill_is_still_a_legal_concat_operand() {
    // §11.4.12.1: a zero replication is legal AS A DIRECT CONCATENATION OPERAND, which
    // `lower_expr`'s `Concat` arm signals by setting `repl_zero_ok` before lowering it.
    // The ctx twin never set it, so adding a fill anywhere in the concat turned a legal
    // design into E3009. ORACLE: iverilog prints `000000000001` for both of these.
    let o = ok("module t; logic [7:0] x = 8'hA5;\n\
         wire [11:0] a = { {0{x}}, '1 };\n\
         wire [11:0] b = { {0{x}}, 1'b1 };\n\
         initial begin #1 $display(\"%b %b\", a, b); $finish; end endmodule\n");
    assert_eq!(o, "000000000001 000000000001\n");
}

#[test]
fn a_negative_replication_count_is_loud_even_when_the_value_is_a_fill() {
    // §11.4.12.2. The ctx twin lowered the count through a bare `lower_index_expr`
    // with none of the rules, so the fill spelling rendered `111111111111` at exit 0
    // while the `1'b1` spelling was correctly E3009.
    // ORACLE: iverilog — "Concatenation repeat may not be negative (-2)".
    for v in ["'1", "1'b1"] {
        let e = loud(&format!(
            "module t; parameter int N = -2;\n\
             wire [11:0] a = {{N{{{v}}}}};\n\
             initial begin #1 $display(\"%b\", a); $finish; end endmodule\n"
        ));
        assert!(
            e.contains("VITA-E3009") && e.contains("negative"),
            "expected the negative-count reject for {v}, got:\n{e}"
        );
    }
}

#[test]
fn null_compared_to_a_fill_is_loud_like_a_handle() {
    // `null` is the other half of the N7 gate's `any_handle`, and it was silent (`0`
    // at exit 0) for exactly the same reason. ORACLE: iverilog — "Both arguments
    // (class, logic) must be class/null for '==' operator."
    let e =
        loud("module t; initial begin #1 $display(\"%b\", null == '1); $finish; end endmodule\n");
    assert!(
        e.contains("VITA-E3009") && e.contains("class handle"),
        "expected the handle gate, got:\n{e}"
    );
}

// ─────── (g) the REAL routes: §6.2 illegalities, the §11.4.9 `**` desugar, ───────
// ───────     and the width a real operand does NOT lend                    ───────

#[test]
fn a_real_operand_is_still_illegal_under_bitwise_and_shift_with_a_fill() {
    // §6.2. PRE, each of these printed a silent `0` at exit 0 while the `1'b1`
    // spelling beside it was E3009 — the ctx twin never ran the check at all.
    // ORACLE: iverilog rejects ("& operator may not have REAL operands").
    for op in ["&", "|", "^", "<<", ">>", ">>>"] {
        let e = loud(&format!(
            "module t; real r = 2.5; logic [15:0] a;\n\
             initial begin a = r {op} '1; #1 $display(\"%b\", a); $finish; end endmodule\n"
        ));
        assert!(
            e.contains("VITA-E3009") && e.contains("real operand"),
            "expected the §6.2 reject for `r {op} '1`, got:\n{e}"
        );
    }
}

#[test]
fn a_real_power_with_a_fill_still_desugars_to_pow() {
    // ⚠️ This one is a ROUTE, not a diagnostic: §11.4.9 turns `**` with a real operand
    // into `$pow`. The ctx twin built a plain integer `Binary` instead, so `r ** '1`
    // read `0` where BOTH oracles — and its own `r ** 1'b1` twin — read 3.
    let o = ok("module t; real r = 2.5; logic [15:0] a, b, c;\n\
         initial begin a = r ** '1; b = r ** 1'b1; c = r ** '0;\n\
         #1 $display(\"%0d %0d %0d\", a, b, c); $finish; end endmodule\n");
    assert_eq!(o, "3 3 1\n");
    // ⚠️ NOT PINNED HERE: `'1 ** r` — the fill as the BASE rather than the exponent.
    // Table 11-21 makes a power's base context-determined, and whether that context
    // survives §11.8.1 turning the whole expression real is a genuine oracle SPLIT
    // (iverilog 480, verilator 1). PRE agreed with NEITHER (0); POST is iverilog's
    // answer, which is an improvement but not a decision, so it is recorded in
    // ROADMAP §2-1d instead of being frozen by an assertion.
}

#[test]
fn a_real_sibling_lends_no_width_to_a_fill() {
    // ⭐ 2-ORACLE. `ir_bits_of` answers 64 for a real — its STORAGE size, not a width
    // the language exposes (§6.12) — so `r + '1` computed `2.5 + (2^64-1)` and printed
    // 0. §11.8.1 converts the integral operand of a mixed expression to real, so the
    // fill keeps its own self-determined width of one bit: `2.5 + 1 = 3.5` → 4, which
    // is what iverilog AND verilator print, and what `r + 1'b1` already printed.
    //
    // Same trap as `ir_lvalue_width` in §4.5.353, one level down: at the OPERATOR
    // instead of the assignment.
    let o = ok("module t; real r = 2.5;\n\
         logic [15:0] a, b, c, d, e, f;\n\
         initial begin a = r + '1; b = r + 1'b1; c = r - '1; d = r * '1;\n\
         e = r > '1; f = '1 + r;\n\
         #1 $display(\"%0d %0d %0d %0d %0d %0d\", a, b, c, d, e, f); $finish; end endmodule\n");
    assert_eq!(o, "4 4 2 3 1 4\n");
}

#[test]
fn a_real_ternary_branch_lends_no_width_to_a_fill() {
    // ⭐ 2-ORACLE. `c ? r : '1` with c=0 must select the fill, and the fill is one bit
    // (§11.8.1 — its real sibling lends nothing), so it is 1. vita read 0. The
    // `c ? r : 1'b1` twin beside it already read 1.
    //
    // ⚠️ `c ? '1 : r` does NOT show the bug even though it is mis-sized the same way:
    // c=0 selects the OTHER branch, so the fill's width never reaches the output. The
    // probe has to take the fill branch — which is why both orders are here.
    let o = ok("module t; real r = 2.5; logic c = 1'b0, d = 1'b1;\n\
         logic [15:0] a, b, e, f;\n\
         initial begin a = c ? r : '1; b = c ? r : 1'b1;\n\
         e = d ? r : '1; f = c ? '1 : r;\n\
         #1 $display(\"%0d %0d %0d %0d\", a, b, e, f); $finish; end endmodule\n");
    assert_eq!(o, "1 1 3 3\n");
}

#[test]
fn a_real_case_selector_lends_no_width_to_a_fill_label() {
    // ⭐ 2-ORACLE (iverilog + verilator both match the label). §12.5 sizes a case label
    // to the selector, and `lower_case_label` asked `ir_bits_of` for that size — which
    // answers 64 for a real, so `'1` became 64 bits, never equalled 1.0, and the design
    // silently fell through to `default`. The `1'b1` label already matched.
    let o = ok("module t; real r = 1.0; logic [7:0] a, b;\n\
         initial begin\n\
           case (r) '1:    a = 1; default: a = 2; endcase\n\
           case (r) 1'b1:  b = 1; default: b = 2; endcase\n\
           #1 $display(\"%0d %0d\", a, b); $finish; end endmodule\n");
    assert_eq!(o, "1 1\n");
}
