//! A fill literal takes its assignment's width even when the target is a DEFERRED
//! HIERARCHICAL write (§4.5.355).
//!
//! §4.5.353 gave `resize_fill_rhs` the assignment width at every assignment form, and
//! it worked — unless the target was `u.a` rather than `a`. A cross-instance write
//! lowers its lvalue to a SENTINEL net (the child instance's nets do not exist yet),
//! `ir_lvalue_width` reads `nets.get(0xFF00_0000)` → `None` → `.max(1)` → 1, and the
//! fill became one bit. The width was never missing, only LATE.
//!
//! | shape                     | PRE            | both oracles   |
//! |---------------------------|----------------|----------------|
//! | `u.a = '1`                | `000000000001` | `111111111111` |
//! | `u.a <= '1`               | `000000000001` | `111111111111` |
//! | `force u.a = '1`          | `000000000001` | `111111111111` |
//! | `assign u.a = '1` (proc)  | `000000000001` | `111111111111` |
//! | `u.v.q = '1` (two levels) | `000000000001` | `111111111111` |
//! | `u.arr[0] = '1`           | `000000000001` | `111111111111` |
//! | `u.a[7:0] = '1`           | `xxxx00000001` | `xxxx11111111` |
//!
//! ⭐ HOW IT IS RIGHT NOW (§5.2 Q-LOSSYSPELL fix H; the mechanism §4.5.355 / §4.5.357 /
//! §4.5.360 built — a second lowering of a "scope-free" right-hand side after elaboration,
//! with its name re-check and its repoint of every statement — is deleted). A deferred
//! piece is an UNDECIDED piece of the target (`edge_gate.rs` THE FILL RULE): the fill is
//! lowered ONCE, at the assignment, in its own scope, under the width the lowering can see
//! (the deferred piece counted 1). `'0` / `'1` are spelled `1'b0` / `~1'b0`, which the run
//! sizes at the final assignment context; an `'x` / `'z` constant leaves a record naming
//! its own slot, and after both deferral lanes have published the chunk they decided on,
//! the end-phase decision re-sizes that slot in place at the decided width of the WHOLE
//! target, or refuses the design. So any right-hand side is right, not only a scope-free
//! one (a plain name, a dotted read, a call), and nothing is enumerated per lane — in
//! particular the row NOT in the table:
//!
//! ⚠️ `u.a[0] = '1` is a ONE-BIT target and all three oracles read `xxxxxxxxxxx1`.
//! It goes down the same deferral lane as the element and part-select writes, so a fix
//! written per lane would have widened it to twelve ones. Its rebuilt chunk is one bit,
//! so the decided width is 1 and the run sizes `~1'b0` at one bit. The test below fails
//! if that ever stops being true.
//!
//! The rule is `max(ctx, sibling)`: a fill normally takes its sibling's width, which is
//! why `{2{'1}}` and `~'0` were right at a hierarchical target even before §4.5.355 — but
//! when the sibling is NARROWER than the target the context is the only source. Measured
//! at §4.5.357's boundary: `logic [63:0] W; W = '1 + 1;` is 0 (iverilog agrees) while
//! `u.wide = '1 + 1;` was 4294967296, because the sibling `1` is 32 bits.
//!
//! The sibling-supplied cases are pinned below so a later "simplification" that routes
//! them through the deferral cannot change them silently.
//!
//! ORACLES: iverilog 13.0 and verilator 5.050 agree on every value above. On the
//! part-select and bit-select rows verilator prints `0` for the untouched bits because
//! it is 2-state; iverilog and §4.9.1 are the authority there, and the bits under
//! assignment agree in both.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

const SUB: &str = "module sub;\n\
     logic [11:0] a;\n\
     logic [11:0] arr [0:1];\n\
     sub2 v();\n\
     endmodule\n\
     module sub2;\n  logic [11:0] q;\nendmodule\n";

fn run(body: &str, probe: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let src = format!(
        "{SUB}module t;\n  logic [11:0] L;\n  logic c = 1'b1;\n  sub u();\n\
         initial begin {body} #1 $display(\"r=%b\", {probe}); $finish; end\nendmodule\n"
    );
    let p = std::env::temp_dir().join(format!("vita_fht_{}_{n}.sv", std::process::id()));
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
    assert!(
        out.status.success(),
        "expected success for `{body}`:\n{all}"
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .find_map(|l| l.strip_prefix("r=").map(str::to_string))
        .unwrap_or_else(|| panic!("no r= line for `{body}`:\n{all}"))
}

// ───────────────── the four assignment forms, one level down ─────────────────

#[test]
fn every_assignment_form_gives_a_hierarchical_fill_the_target_width() {
    for body in [
        "u.a = '1;",
        "u.a <= '1;",
        "force u.a = '1;",
        "assign u.a = '1;",
    ] {
        assert_eq!(run(body, "u.a"), "111111111111", "for `{body}`");
    }
    // The `1'b1` twins, which were already correct, must stay put: the fix must widen
    // the FILL, not the assignment.
    for body in ["u.a = 1'b1;", "u.a <= 1'b1;", "force u.a = 1'b1;"] {
        assert_eq!(run(body, "u.a"), "000000000001", "for `{body}`");
    }
}

#[test]
fn a_two_level_hierarchical_target_works_the_same() {
    assert_eq!(run("u.v.q = '1;", "u.v.q"), "111111111111");
    assert_eq!(run("u.v.q = '0;", "u.v.q"), "000000000000");
}

// ───────────── the other deferral lane: element, part-select, bit-select ─────────────

#[test]
fn a_hierarchical_array_element_takes_the_element_width() {
    assert_eq!(run("u.arr[0] = '1;", "u.arr[0]"), "111111111111");
    assert_eq!(run("u.arr[0] = 12'hFFF;", "u.arr[0]"), "111111111111");
}

#[test]
fn a_hierarchical_part_select_takes_the_part_width() {
    // ORACLE: iverilog `xxxx11111111` (the untouched top nibble stays x).
    assert_eq!(run("u.a[7:0] = '1;", "u.a"), "xxxx11111111");
    assert_eq!(run("u.a[7:0] = 8'hFF;", "u.a"), "xxxx11111111");
}

#[test]
fn a_hierarchical_bit_select_still_takes_one_bit() {
    // ⚠️ THE ANTI-REGRESSION PIN. Same deferral lane as the two tests above, and all
    // three oracles say one bit. Widening it would be the exact trade this project
    // forbids: fixing one silent-wrong by creating another.
    assert_eq!(run("u.a[0] = '1;", "u.a"), "xxxxxxxxxxx1");
    assert_eq!(run("u.a[0] = '0;", "u.a"), "xxxxxxxxxxx0");
    assert_eq!(run("u.a[0] = 1'b1;", "u.a"), "xxxxxxxxxxx1");
}

// ───────────────── the halves that were already right ─────────────────

#[test]
fn a_local_target_is_unchanged() {
    // The non-hierarchical spelling never went near a sentinel; it is here because it
    // is the reference the hierarchical rows are supposed to match.
    assert_eq!(run("L = '1;", "L"), "111111111111");
    assert_eq!(run("L <= '1;", "L"), "111111111111");
    assert_eq!(run("force L = '1;", "L"), "111111111111");
    assert_eq!(run("L = '0;", "L"), "000000000000");
}

#[test]
fn a_fill_whose_sibling_is_wide_enough_is_untouched_at_a_hierarchical_target() {
    // Each of these agreed with iverilog before this slice and still does: the width
    // came from the sibling, or from the operand's own self-determined position, not
    // from the assignment context.
    assert_eq!(run("u.a = {2{'1}};", "u.a"), "000000000011");
    assert_eq!(run("u.a = c ? '1 : 12'h0;", "u.a"), "111111111111");
    assert_eq!(run("u.a = ~'0;", "u.a"), "111111111111");
    // ⭐ The one that states the rule exactly: `max(ctx, sibling)`. Here the sibling is
    // as wide as the target, so the deferred context contributes nothing and the answer
    // is right — the same expression with a NARROW sibling is the recorded residue (see
    // the header), which is why this test is named for the sibling's width rather than
    // for "a fill with a sibling".
    assert_eq!(run("u.a = '1 & 12'hF0F;", "u.a"), "111100001111",);
}

#[test]
fn a_parenthesised_bare_fill_is_still_a_bare_fill() {
    // `lower_expr` sees through `(…)`, so the fill lowering must too.
    assert_eq!(run("u.a = ('1);", "u.a"), "111111111111");
}

#[test]
fn a_scope_free_fill_expression_reaches_the_target_width() {
    // ⭐ §4.5.357: an operator over the fill, not only a bare literal. Each of these
    // has a sibling NARROWER than the 12-bit target, so the assignment context is the
    // only width source — and at a deferred target it used to be 1. ORACLE: iverilog
    // for every value.
    for (rhs, want) in [
        ("'1 + 1", "000000000000"),
        ("'1 - 1", "111111111110"),
        ("'1 & 1", "000000000001"),
        ("'1 | 1", "111111111111"),
        ("'1 ^ 1", "111111111110"),
        ("1 + '1", "000000000000"),
        ("-'1", "000000000001"),
        ("~'1", "000000000000"),
        ("'1 << 0", "111111111111"),
        ("'1 & 8'hF0", "000011110000"),
    ] {
        assert_eq!(run(&format!("u.a = {rhs};"), "u.a"), want, "for `{rhs}`");
    }
}

#[test]
fn a_right_hand_side_that_reads_a_net_reaches_the_target_width_too() {
    // ⭐ This was ROADMAP §2-1e's residue and read `000000000001` until §4.5.360. `c` is
    // a module net, and the "scope snapshot" its prerequisite asked for is the captured
    // `cur_prefix`. Both oracles read all-ones, and the LOCAL twin below is what says
    // this was a hierarchy problem rather than a ternary one.
    assert_eq!(run("u.a = c ? '1 : 1'b0;", "u.a"), "111111111111");
    assert_eq!(run("L = c ? '1 : 1'b0;", "L"), "111111111111");
    // A name on the other side of the operator, and a name as the whole non-fill
    // sibling — both lowered once, in their own scope (fix H).
    assert_eq!(run("u.a = c ? 1'b0 : '1;", "u.a"), "000000000000");
    assert_eq!(run("u.a = '1 & {11'b0, c};", "u.a"), "000000000001");
}

#[test]
fn a_name_reached_from_a_subprogram_body_still_runs_and_agrees() {
    // ⚠️ THE GUARD, stated as the property it protects rather than as a value: the
    // design must still RUN. The failure this rules out is a spurious unresolved-name
    // error from a lowering outside the name's scope (the deleted second lowering ran
    // after elaboration) — a wrong value turning into a loud reject is still a regression.
    //
    // A task formal is the interesting case: its name exists only inside the subprogram
    // body the fill is lowered in, and both oracles read all-ones.
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let src = format!(
        "{SUB}module t; sub u();\n\
         task automatic g(logic q);\n    u.a = q ? '1 : 1'b0;\n  endtask\n\
         initial begin g(1'b1); #1 $display(\"r=%b\", u.a); $finish; end\nendmodule\n"
    );
    let p = std::env::temp_dir().join(format!("vita_fht_g_{}_{n}.sv", std::process::id()));
    std::fs::write(&p, &src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&p)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&p);
    let so = String::from_utf8_lossy(&out.stdout);
    let se = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "must still run:\n{so}{se}");
    assert!(
        so.contains("r=111111111111"),
        "iverilog reads all-ones:\n{so}{se}"
    );
}
