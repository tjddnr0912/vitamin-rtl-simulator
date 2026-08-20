//! Three consumers that ate a NEGATIVE constant instead of obeying the language rule.
//!
//! Each was reachable from an ordinary spelling (`localparam W = -56`) and each ran to
//! exit 0 with a wrong answer:
//!
//! - a **replication count** `{W{1'b1}}` reached the engine as its two's-complement bit
//!   pattern — 4294967240 copies, truncated to the target, printing 255. IEEE §11.4.12.2
//!   requires a non-negative constant and BOTH oracles reject ("Concatenation repeat may
//!   not be negative" / "Replication value of < 0 or X/Z not legal").
//! - an **ascending packed range** with a negative left bound (`logic [-3:0]`, and the
//!   `[W-1:0]`-with-`W==0` spelling of it) was clamped to width 1 under a warning whose
//!   text named a parameter underflow that does not exist. IEEE §7.4.2 sizes a vector
//!   `|msb-lsb|+1` whatever the signs; both oracles do.
//! - an **unpacked array dimension size** `logic q[-3]` was absorbed by a `.max(1)` floor
//!   and declared one word. IEEE §7.4.2 wants a positive size; both oracles reject.
//!
//! The replication rule needs the count's SIGN, and the sign only exists at the count's
//! own self-determined width: `{(4'd0-4'd1){1'b1}}` is 15 copies in both oracles while
//! `{(4'sd0-4'sd1){1'b1}}` is rejected, and the width-unlimited const fold calls both of
//! them -1. Both readings below are pinned so a future simplification to the unlimited
//! domain fails here instead of false-rejecting a correct design.
//!
//! ORACLES: iverilog 13.0 + verilator 5.050 (they agree on every cell in this file).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let p = std::env::temp_dir().join(format!("vita_ncc_{}_{n}.sv", std::process::id()));
    std::fs::write(&p, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&p)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&p);
    (
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
        out.status.success(),
    )
}

fn ok(src: &str) -> String {
    let (o, s) = run(src);
    assert!(s, "expected success:\n{o}");
    o
}

fn loud(src: &str, needle: &str) {
    let (o, s) = run(src);
    assert!(!s, "must be loud:\n{o}");
    assert!(o.contains(needle), "expected `{needle}` in:\n{o}");
}

// ────────────────────────── (a) replication count ──────────────────────────

#[test]
fn a_negative_replication_count_from_a_param_is_loud() {
    // Was: 255 at exit 0 (a 4294967240-copy replication truncated to the target).
    loud(
        "module t; localparam int W = -56; wire [7:0] r = {W{1'b1}};\n\
         initial $display(\"R=%0d\", r); endmodule\n",
        "replication count may not be negative",
    );
}

#[test]
fn a_negative_replication_count_from_an_untyped_param_is_loud() {
    // The untyped spelling takes a different route to the lowered count than the
    // `int` one; both must land on the same rule.
    loud(
        "module t; localparam W = -1; wire [7:0] r = {W{1'b1}};\n\
         initial $display(\"R=%0d\", r); endmodule\n",
        "replication count may not be negative",
    );
}

#[test]
fn a_negative_replication_count_that_did_not_fold_to_a_const_is_loud() {
    // `{(2-3){…}}` and `{(W-4){…}}` reach the lowered tree as an `Add`/`Sub`, whose
    // u32 fold SATURATES to 0 — so the zero-count check used to report "a replication
    // count of zero" about a count of -1. Loud for the real reason now.
    loud(
        "module t; wire [7:0] r = {(2-3){1'b1}};\n\
         initial $display(\"R=%0d\", r); endmodule\n",
        "replication count may not be negative",
    );
    loud(
        "module t; localparam int W = 3; wire [15:0] r = {(W-4){1'b1}};\n\
         initial $display(\"R=%0d\", r); endmodule\n",
        "replication count may not be negative",
    );
}

#[test]
fn a_negative_string_replication_count_is_loud() {
    // Three spellings, three different code paths, one rule. A string LITERAL operand
    // is not `expr_is_string_ast`, so it rides the generic replicate lowering and the
    // count wrapped — `{P{"ab"}}` built thousands of copies. A string VARIABLE operand
    // returns from its own arm BEFORE that guard, and there are two such arms: the
    // assignment special-case and the generic expression one. Each was an independent
    // `n.max(0)` that turned a negative count into an empty string at exit 0.
    for src in [
        // literal operand → generic path
        "module t; localparam int P = -3; string s;\n\
         initial begin s = {P{\"ab\"}}; $display(\"S=[%s]\", s); end endmodule\n",
        // variable operand, ASSIGNMENT rhs → string_concat_special
        "module t; localparam int P = -3; string s = \"ab\"; string r;\n\
         initial begin r = {P{s}}; $display(\"S=[%s]\", r); end endmodule\n",
        // variable operand, EXPRESSION position → the Replicate arm's own string branch
        "module t; localparam int P = -3; string s = \"ab\";\n\
         initial $display(\"S=[%s]\", {P{s}}); endmodule\n",
    ] {
        loud(src, "replication count may not be negative");
    }
}

#[test]
fn an_unsigned_wrap_count_still_replicates() {
    // THE BOUNDARY, and the reason the sign must be read self-determined:
    // `4'd0-4'd1` is 15 at four unsigned bits — both oracles print 255. Reading the
    // count in the width-unlimited domain would call it -1 and reject a correct
    // design. `32'd0-32'd1` is the 32-bit twin (iverilog 255; verilator hits an
    // internal error on it, so iverilog alone pins that one).
    assert!(ok("module t; wire [7:0] r = {(4'd0-4'd1){1'b1}};\n\
            initial $display(\"R=%0d\", r); endmodule\n")
    .contains("R=255"));
    assert!(ok("module t; wire [7:0] r = {(32'd0-32'd1){1'b1}};\n\
            initial $display(\"R=%0d\", r); endmodule\n")
    .contains("R=255"));
}

#[test]
fn a_signed_wrap_count_is_loud() {
    // The other side of the same boundary: `4'sd0-4'sd1` is -1 at four SIGNED bits
    // and both oracles reject. Identical bit pattern to the cell above.
    loud(
        "module t; wire [7:0] r = {(4'sd0-4'sd1){1'b1}};\n\
         initial $display(\"R=%0d\", r); endmodule\n",
        "replication count may not be negative",
    );
}

#[test]
fn a_negative_count_that_mentions_a_substituted_name_is_loud() {
    // This is the cell the LOWERED reading exists for. Inside an inlined function
    // body the count reads the formal `N`, and `const_bound_signed` declines on a
    // substituted name by contract (the const domain would resolve it through the
    // parameter scope, not the substitution stack). Only the lowered `Const` answers.
    // Both oracles reject; a mutation dropping that reading survives every other
    // cell in this file and dies here.
    loud(
        "module t;\n\
           localparam int N = -3;\n\
           function automatic int g(input int N);\n\
             logic [7:0] t;\n\
             t = {N{1'b1}};\n\
             g = t;\n\
           endfunction\n\
           initial $display(\"V %0d\", g(-3));\n\
         endmodule\n",
        "replication count may not be negative",
    );
}

#[test]
fn a_zero_count_inside_a_concatenation_still_contributes_nothing() {
    // §11.4.12.1 is untouched: a zero count is legal as a direct concat operand.
    assert!(ok(
        "module t; localparam int W = 0; wire [7:0] r = {1'b0, {W{1'b1}}};\n\
            initial $display(\"R=%0d\", r); endmodule\n"
    )
    .contains("R=0"));
}

// ─────────────────── (b) ascending range with a negative left bound ───────────────────

#[test]
fn an_ascending_negative_range_is_sized_and_selectable() {
    // iverilog and verilator: 4 bits, `1010`, `$left` -3, `$right` 0, `q[-3]`=1, `q[0]`=0.
    let o = ok("module t; logic [-3:0] q;\n\
        initial begin q = 4'b1010;\n\
          $display(\"B=%0d V=%b L=%0d R=%0d m=%b l=%b\",\n\
            $bits(q), q, $left(q), $right(q), q[-3], q[0]);\n\
        end endmodule\n");
    assert!(o.contains("B=4 V=1010 L=-3 R=0 m=1 l=0"), "{o}");
}

#[test]
fn an_ascending_range_negative_at_both_ends() {
    // `[-8:-1]` used to take the DESCENDING negative-low-bound branch, fail its
    // `m >= l` test and fall through to the clamp. iverilog/verilator: 8 bits.
    let o = ok("module t; logic [-8:-1] q;\n\
        initial begin q = 8'b10100101;\n\
          $display(\"B=%0d V=%b L=%0d R=%0d m=%b l=%b\",\n\
            $bits(q), q, $left(q), $right(q), q[-8], q[-1]);\n\
        end endmodule\n");
    assert!(o.contains("B=8 V=10100101 L=-8 R=-1 m=1 l=1"), "{o}");
}

#[test]
fn an_ascending_negative_range_with_a_nonzero_right_bound() {
    let o = ok("module t; localparam int P = -3; logic [P:1] q;\n\
        initial begin q = 5'b11010;\n\
          $display(\"B=%0d V=%b L=%0d R=%0d m=%b l=%b\",\n\
            $bits(q), q, $left(q), $right(q), q[-3], q[1]);\n\
        end endmodule\n");
    assert!(o.contains("B=5 V=11010 L=-3 R=1 m=1 l=0"), "{o}");
}

#[test]
fn a_wide_ascending_negative_range() {
    // The recorded cell: `localparam int P = -56; logic [P:0] q` — 57 bits in both
    // oracles, one bit in vita.
    let o = ok("module t; localparam int P = -56; logic [P:0] q;\n\
        initial begin q = '1; $display(\"B=%0d V=%0d\", $bits(q), q); end endmodule\n");
    assert!(o.contains("B=57 V=144115188075855871"), "{o}");
}

#[test]
fn a_runtime_index_walks_the_declared_numbering() {
    // Exercises the offset path a CONSTANT index does not: the index is a variable,
    // so the normalization is an arena `Sub` on a sealed signed index rather than a
    // folded constant. iverilog/verilator: `1011` (MSB `q[-3]` first).
    let o = ok("module t; logic [-3:0] q; int i;\n\
        initial begin q = 4'b1011;\n\
          for (i = -3; i <= 0; i = i + 1) $write(\"%b\", q[i]);\n\
          $display(\"\");\n\
        end endmodule\n");
    assert!(o.contains("1011"), "{o}");
}

#[test]
fn both_directions_coexist_in_concat_and_arithmetic() {
    let o = ok("module t; logic [-3:0] q; logic [-8:-1] r;\n\
        initial begin q = 4'b1011; r = 8'b11001010;\n\
          $display(\"cat=%b sum=%0d\", {q, r}, q + r);\n\
        end endmodule\n");
    assert!(o.contains("cat=101111001010 sum=213"), "{o}");
}

#[test]
fn the_vcd_var_line_uses_the_declared_ascending_range() {
    // The stored range is normalized `[0:3]`, so without the declared-pair sidecar a
    // viewer would number the bits `[0:3]` where iverilog writes `[-3:0]`.
    let d = std::env::temp_dir().join(format!("vita_ncc_vcd_{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(
        &f,
        "module t; logic [-3:0] q;\n\
         initial begin $dumpfile(\"o.vcd\"); $dumpvars(0, t); q = 4'b1011; #1 $finish; end\n\
         endmodule\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    assert!(out.status.success(), "{out:?}");
    let vcd = std::fs::read_to_string(d.join("o.vcd")).unwrap();
    assert!(vcd.contains("q [-3:0]"), "{vcd}");
}

#[test]
fn an_indexed_part_select_of_an_ascending_negative_net_is_exact() {
    // `+:`/`-:` do NOT go through the const `[msb:lsb]` bound fold, so they are exact
    // where the `[msb:lsb]` form below stays loud — and the STORED direction is what
    // makes them exact, which is the only thing that pins it (a mutation that stored
    // this net descending survived every other test in this file).
    // iverilog and verilator: `11 10 00`.
    let o = ok("module t; logic [-3:0] q;\n\
        initial begin q = 4'b1100;\n\
          $display(\"IP %b %b %b\", q[-3 +: 2], q[-1 -: 2], q[0 -: 2]);\n\
        end endmodule\n");
    assert!(o.contains("IP 11 10 00"), "{o}");
}

#[test]
fn a_negative_bound_element_of_dynamic_storage_keeps_the_honest_clamp() {
    // The element net of a queue / dynamic / associative array is built on an
    // early-return path that never reaches the declared-bound record, so sizing it
    // wide would leave its selects unnormalized — `q[0][-3]` read internal bit 6 and
    // printed a silent `x`. BOTH directions opt out and keep the warning: iverilog
    // rejects these outright and verilator sizes them, so the honest degrade is the
    // clamp until the element path records the bound too (ROADMAP §3).
    for src in [
        "module t; logic [-3:0] q[$];\n\
         initial begin q.push_back(4'b1100); $display(\"Q %0d\", $bits(q[0])); end endmodule\n",
        "module t; logic [3:-2] q[$];\n\
         initial begin q.push_back(6'b110011); $display(\"Q %0d\", $bits(q[0])); end endmodule\n",
        "module t; logic [-3:0] d[];\n\
         initial begin d = new[1]; $display(\"Q %0d\", $bits(d[0])); end endmodule\n",
    ] {
        let (o, s) = run(src);
        assert!(s, "must stay graceful:\n{o}");
        assert!(o.contains("W3056"), "must announce the clamp:\n{o}");
        assert!(o.contains("Q 1"), "clamped to width 1:\n{o}");
    }
}

#[test]
fn a_bit_select_of_a_packed_dim_with_a_negative_low_bound_is_loud() {
    // `logic [-3:0][1:0] x; x[-3]` needs the SIGNED coordinate `(lo+size-1) - idx`,
    // which the packed-dim path does not build. It used to be a `debug_assert` on an
    // invariant that was already false, so a RELEASE build read `lo.max(0)`'s
    // coordinate — a silently wrong bit. Loud in both build profiles now; `$bits`
    // and the whole value stay exact.
    loud(
        "module t; logic [-3:0][1:0] x;\n\
         initial begin x = 8'b11001010; $display(\"%b\", x[-3]); end endmodule\n",
        "packed dimension declared with a negative",
    );
    assert!(ok(
        "module t; logic [-3:0][1:0] x;\n\
            initial begin x = 8'b11001010; $display(\"B=%0d V=%b\", $bits(x), x); end endmodule\n"
    )
    .contains("B=8 V=11001010"));
}

#[test]
fn a_part_select_of_an_ascending_negative_net_is_loud_for_the_right_reason() {
    // Mirrors the descending twin's documented gap: the whole value and BIT selects
    // are exact, a PART select folds its own bounds through the unsigned const path.
    loud(
        "module t; logic [-3:0] q;\n\
         initial begin q = 4'b1011; $display(\"%b\", q[-2:0]); end endmodule\n",
        "PART select of a net declared with a negative low bound",
    );
}

#[test]
fn an_ascending_negative_range_over_the_width_cap_is_loud_not_clamped() {
    // `[-2000000:0]` is 2000001 bits — legal in both oracles, over vita's v1 net-width
    // cap. It must hit the SAME cap error every other over-wide declaration does, not
    // the silent `.min(MAX_NET_WIDTH)` the descending branch used to apply.
    loud(
        "module t; logic [-2000000:0] q;\n\
         initial $display(\"B=%0d\", $bits(q)); endmodule\n",
        "exceeds the v1 cap",
    );
}

#[test]
fn an_ordinary_ascending_range_is_untouched() {
    // The non-negative ascending path must not move: `[0:7]` and `[1:3]`.
    let o = ok("module t; logic [0:7] a; logic [1:3] b;\n\
        initial begin a = 8'b10110001; b = 3'b101;\n\
          $display(\"a=%b a0=%b a7=%b b=%b b1=%b\", a, a[0], a[7], b, b[1]);\n\
        end endmodule\n");
    assert!(o.contains("a=10110001 a0=1 a7=1 b=101 b1=1"), "{o}");
}

// ─────────────────── (c) unpacked array dimension size ───────────────────

#[test]
fn a_negative_unpacked_dimension_size_is_loud() {
    // Was: `$size` 1 at exit 0. Both oracles: "Dimension size must be greater than
    // zero" / "Size of range is '[-3]', must be positive integer".
    loud(
        "module t; localparam int P = -3; logic q [P];\n\
         initial $display(\"S=%0d\", $size(q)); endmodule\n",
        "dimension size must be a POSITIVE constant",
    );
    loud(
        "module t; logic q [-3];\n\
         initial $display(\"S=%0d\", $size(q)); endmodule\n",
        "dimension size must be a POSITIVE constant",
    );
}

#[test]
fn a_zero_unpacked_dimension_size_is_loud() {
    // The other side of the boundary, and the cell the `.max(1)` floor hid best:
    // `[0]` declares no words and both oracles reject it too.
    loud(
        "module t; logic q [0];\n\
         initial $display(\"S=%0d\", $size(q)); endmodule\n",
        "dimension size must be a POSITIVE constant",
    );
}

#[test]
fn a_negative_size_on_an_inner_dimension_is_loud() {
    // The outer dim folds fine, so the array looked healthy: `$size` reported the
    // OUTER 2 while the inner dim had silently become one word.
    loud(
        "module t; localparam int P = -3; logic [7:0] q [2][P];\n\
         initial $display(\"S=%0d\", $size(q)); endmodule\n",
        "dimension size must be a POSITIVE constant",
    );
}

#[test]
fn a_size_of_one_still_declares_one_word() {
    assert!(ok("module t; logic q [1];\n\
            initial $display(\"S=%0d\", $size(q)); endmodule\n")
    .contains("S=1"));
}
