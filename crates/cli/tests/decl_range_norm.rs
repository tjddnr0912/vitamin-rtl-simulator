//! §4.5.308 — declared-range index normalization, pinned to iverilog 13.
//!
//! The normalization (`idx − lsb` descending, `hi − idx` ascending) used to be
//! lowered as an UNSIGNED 32-bit subtraction, which was wrong two ways at once:
//! it WIDENED the user's index so a context-determined operator inside it
//! (`~`, a carry, a borrow) evaluated at 32 bits instead of its own, and it
//! WRAPPED when the index sat below a non-zero declared LSB, turning a
//! legitimate partial write into a dropped one.
//!
//! Every expectation below is iverilog 13's, measured. The zero-LSB descending
//! rows are the REGRESSION axis: they emit no normalization at all and must not
//! move.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_drn_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// `run` for a design whose out-of-range ARRAY-WORD accesses are DIAGNOSED —
/// those drops are loud (`E4002`, exit 1), which is itself worth pinning: the
/// pre-fix binary exits 0 with no diagnostic at all on the anchor design.
///
/// ⚠️ Scope: only the array-word rows are loud. A packed BIT-offset drop is
/// silent by design (an out-of-range bit select is not a range error), so rows
/// C and D contribute nothing here — the values are what pin those.
fn run_loud(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_drnl_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let err = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "expected a loud drop:\n{err}");
    assert!(
        err.contains("VITA-E4002"),
        "the drop must be diagnosed:\n{err}"
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn context_determined_index_is_sealed_at_its_own_width() {
    // `~r5` is five bits wide. Widening it into a 32-bit normalization made it
    // 0xFFFF_FFEC instead of 28, so every one of these wrote nothing and read x.
    let out = run("module top;\n\
           reg [33:2] d2; reg [-2:-33] dn; reg [0:31] a0; reg [2:33] a2;\n\
           reg [4:0] r5;\n\
           initial begin\n\
             r5 = 5'd3;\n\
             d2 = 0; d2[~r5] = 1'b1; $display(\"d2 %h\", d2);\n\
             dn = 0; dn[~r5] = 1'b1; $display(\"dn %h\", dn);\n\
             a0 = 0; a0[~r5] = 1'b1; $display(\"a0 %h\", a0);\n\
             a2 = 0; a2[~r5] = 1'b1; $display(\"a2 %h\", a2);\n\
             d2 = 32'h0F0F0F0F; $display(\"rd %b\", d2[~r5]);\n\
             a0 = 32'h0F0F0F0F; $display(\"ra %b\", a0[~r5]);\n\
             $finish;\n\
           end\n\
         endmodule\n");
    assert!(out.contains("d2 04000000"), "{out}");
    assert!(out.contains("dn 00000000"), "{out}");
    assert!(out.contains("a0 00000008"), "{out}");
    assert!(out.contains("a2 00000020"), "{out}");
    assert!(out.contains("rd 1"), "{out}");
    assert!(out.contains("ra 1"), "{out}");
}

#[test]
fn index_below_a_non_zero_declared_lsb_partial_writes() {
    // `d2[1 +: 2]` normalizes to −1: bit 0 of the value lands out of range and
    // bit 1 lands on internal bit 0. Computed unsigned this wrapped and the
    // whole write vanished.
    let out = run("module top;\n\
           reg [33:2] d2; reg [31:0] z0;\n\
           initial begin\n\
             d2 = 0; d2[1 +: 2] = 2'b11; $display(\"a %h\", d2);\n\
             d2 = 0; d2[0 +: 4] = 4'hF;  $display(\"b %h\", d2);\n\
             d2 = 0; d2[2 +: 2] = 2'b11; $display(\"c %h\", d2);\n\
             z0 = 0; z0[-1 +: 4] = 4'hF; $display(\"d %h\", z0);\n\
             $finish;\n\
           end\n\
         endmodule\n");
    assert!(out.contains("a 00000001"), "{out}");
    assert!(out.contains("b 00000003"), "{out}");
    assert!(out.contains("c 00000003"), "{out}");
    // REGRESSION axis: the zero-LSB underflow was always right and stays right.
    assert!(out.contains("d 00000007"), "{out}");
}

#[test]
fn zero_lsb_descending_is_untouched() {
    // No normalization is emitted for `[N:0]`, so these rows are the proof that
    // the fix is confined to the arms that were wrong.
    let out = run("module top;\n\
           reg [31:0] v; reg [4:0] r5; integer k;\n\
           initial begin\n\
             r5 = 5'd3; k = 3;\n\
             v = 0; v[~r5] = 1'b1;      $display(\"a %h\", v);\n\
             v = 0; v[k] = 1'b1;        $display(\"b %h\", v);\n\
             v = 0; v[3 +: 2] = 2'b11;  $display(\"c %h\", v);\n\
             v = 0; v[k - 1] = 1'b1;    $display(\"d %h\", v);\n\
             $finish;\n\
           end\n\
         endmodule\n");
    assert!(out.contains("a 10000000"), "{out}");
    assert!(out.contains("b 00000008"), "{out}");
    assert!(out.contains("c 00000018"), "{out}");
    assert!(out.contains("d 00000004"), "{out}");
}

/// The UNPACKED (array-word) twin — same defect, same seal, and the indices
/// are chosen IN RANGE on purpose.
///
/// The differential review's own first unpacked sweep was vacuous because it
/// seeded `r5 = 3`, making `~r5 = 28` out of range for every small array, so
/// the headline axis measured nothing. `r5 = 28` puts `~r5` at 3, which every
/// array below holds. `mz` (1-D, 0-based) is the regression axis: it was
/// always right, because its per-dim guard does not exist for `d == 1`.
/// `g2` (2-D, 0-based) was NOT right — the `d >= 2` guard is where the
/// widened index failed — which makes it a genuine fix, not a regression axis.
#[test]
fn unpacked_dimension_index_is_sealed_too() {
    let out = run("module top;\n\
           reg [7:0] ma [2:5]; reg [7:0] md [5:2];\n\
           reg [7:0] g2 [0:3][0:3]; reg [7:0] mz [0:7];\n\
           reg [4:0] r5;\n\
           initial begin\n\
             ma[2]=8'd20; ma[3]=8'd30; ma[4]=8'd40; ma[5]=8'd50;\n\
             md[2]=8'd20; md[3]=8'd30; md[4]=8'd40; md[5]=8'd50;\n\
             mz[3]=8'd77; g2[3][3]=8'd99;\n\
             r5 = 5'd28;\n\
             $display(\"A %0d %0d %0d\", ma[~r5], md[~r5], mz[~r5]);\n\
             $display(\"B %0d\", g2[~r5][~r5]);\n\
             ma[~r5] = 8'd1; md[~r5] = 8'd2; g2[~r5][~r5] = 8'd3; mz[~r5] = 8'd4;\n\
             $display(\"C %0d %0d %0d %0d\", ma[3], md[3], g2[3][3], mz[3]);\n\
             $finish;\n\
           end\n\
         endmodule\n");
    assert!(out.contains("A 30 30 77"), "{out}");
    assert!(out.contains("B 99"), "{out}");
    assert!(out.contains("C 1 2 3 4"), "{out}");
}

/// A WHOLE clocked design that actually reaches the changed lowering.
///
/// This exists because the obvious whole-design checks cannot see this fix:
/// the review byte-compared `examples/` and `bench/keccak` and found their
/// `.velab` artifacts IDENTICAL before and after — every net in them is
/// zero-LSB descending, so they never enter a changed arm. They are a fine
/// fast-path check and structurally worthless as validation. This design puts
/// a non-zero-LSB, an ascending, and a negative-low-bound net on a clock with
/// NBAs and a moving index; PRE prints `acc=00000000 asc=00000000`, and the
/// values below are iverilog 13's.
#[test]
fn a_clocked_design_over_changed_ranges_matches_the_oracle() {
    let out = run("module top;\n\
           reg clk = 1'b0;\n\
           reg [33:2] acc; reg [0:31] asc; reg [3:-2] neg;\n\
           reg [4:0] ptr; integer n;\n\
           always #5 clk = ~clk;\n\
           initial begin\n\
             acc = 0; asc = 0; neg = 0; ptr = 0;\n\
             for (n = 0; n < 12; n = n + 1) begin\n\
               @(posedge clk);\n\
               ptr <= ptr + 5'd1;\n\
               acc[~ptr] <= 1'b1;\n\
               asc[~ptr] <= 1'b1;\n\
               neg[ptr[2:0]] <= 1'b1;\n\
             end\n\
             @(posedge clk);\n\
             $display(\"acc=%h asc=%h neg=%h\", acc, asc, neg);\n\
             $finish;\n\
           end\n\
         endmodule\n");
    assert!(out.contains("acc=3ffc0000 asc=00000fff neg=3c"), "{out}");
}

/// The three properties the first version of this file could not see, each
/// found by a surviving mutation in the soundness review.
///
/// - **A/B/C — a NEGATIVE folded constant.** `const_index_value` reads the
///   constant with its own signedness; ignoring that survived the entire
///   workspace suite, because no test used a negative constant index.
/// - **D — the seal's SIGN, on a non-constant index.** Swapping `$signed` for
///   `$unsigned` passed all three original tests: they use constant indices,
///   which take the const-fold path and never reach the seal at all. Only a
///   computed index below the declared LSB exercises it.
/// - **E — a >32-bit signed index on a negative-LSB net.** The non-sealed arm
///   emits `Add |k|` rather than `Sub k`; at 32 bits the two are identical, so
///   nothing narrower can tell them apart.
///
/// Values are iverilog 13's.
#[test]
fn negative_constants_the_seals_sign_and_wide_indices() {
    let out = run("module top;\n\
           localparam integer LN1 = -1;\n\
           localparam integer LN34 = -34;\n\
           reg [33:2] d2; reg [-2:-33] dn; reg [0:31] a0;\n\
           reg [4:0] r5; reg signed [63:0] ll;\n\
           initial begin\n\
             d2 = 0; d2[LN1 +: 4] = 4'hF;    $display(\"A %h\", d2);\n\
             dn = 0; dn[LN34 +: 3] = 3'b111; $display(\"B %h\", dn);\n\
             a0 = 0; a0[LN1 +: 3] = 3'b111;  $display(\"C %h\", a0);\n\
             r5 = 5'd3;\n\
             d2 = 0; d2[r5 - 5'd2 +: 2] = 2'b11; $display(\"D %h\", d2);\n\
             ll = -64'sd5; dn = 0; dn[ll] = 1'b1; $display(\"E %h\", dn);\n\
             $finish;\n\
           end\n\
         endmodule\n");
    assert!(out.contains("A 00000001"), "{out}");
    assert!(out.contains("B 00000003"), "{out}");
    assert!(out.contains("C c0000000"), "{out}");
    assert!(out.contains("D 00000001"), "{out}");
    assert!(out.contains("E 10000000"), "{out}");
}

/// The two shapes whose SIGNEDNESS the free predicate cannot judge alone, each
/// a measured regression the review caught before this shipped.
///
/// `**` takes its BASE's sign, not "either operand unsigned"; and a class-field
/// read is a `Signal` whose net is a 32-bit handle, so its signedness lives in
/// `class_field_widths`, not in the net. Proving either unsigned made the seal
/// reinterpret a negative index as a huge positive and drop the write.
#[test]
fn pow_and_class_field_indices_keep_their_sign() {
    let out = run("module top;\n\
           reg signed [31:0] s32; reg [31:0] u32; reg [-2:-33] dn;\n\
           initial begin\n\
             s32 = -32'sd2; u32 = 32'd3; dn = 0;\n\
             dn[s32 ** u32] = 1'b1; $display(\"A %h\", dn);\n\
             $finish;\n\
           end\n\
         endmodule\n");
    assert!(out.contains("A 02000000"), "{out}");

    // The bare `dn[c.si]` form hits the predicate's `Signal` arm directly.
    // `~c.si` and `c.si + 0` reach it through the Unary and Binary arms, which
    // an earlier design — a blanket refusal in a wrapper — left untested: both
    // mutations survived the WHOLE workspace suite. An UNSIGNED field is here
    // too (`D`): the predicate reads the field's OWN `signed` flag, so an
    // unsigned field still earns the seal instead of losing it to a blanket
    // refusal, and `~c.bu` then correctly lands out of range and writes nothing.
    let out = run("class C; int si; bit [7:0] bu; endclass\n\
         module top;\n\
           C c; reg [-2:-33] dn;\n\
           initial begin\n\
             c = new(); c.si = -5; c.bu = 8'd5; dn = 0;\n\
             dn[c.si] = 1'b1;          $display(\"A %h\", dn);\n\
             c.si = 5; dn = 0;\n\
             dn[~c.si] = 1'b1;         $display(\"B %h\", dn);\n\
             c.si = -5; dn = 0;\n\
             dn[c.si + 32'sd0] = 1'b1; $display(\"C %h\", dn);\n\
             dn = 0; dn[~c.bu] = 1'b1; $display(\"D %h\", dn);\n\
             $finish;\n\
           end\n\
         endmodule\n");
    assert!(out.contains("A 10000000"), "{out}");
    assert!(out.contains("B 08000000"), "{out}");
    assert!(out.contains("C 10000000"), "{out}");
    assert!(out.contains("D 00000000"), "{out}");
}

/// The HIERARCHICAL dimension funnel, which a patch that reached only its twin
/// left behind — local `mem[~i]` correct while `u.mem[~i]` stayed wrong. Two
/// spellings of one geometry disagreeing is worse than both being wrong.
#[test]
fn the_hierarchical_dimension_funnel_is_sealed_too() {
    let out = run("module sub; reg [7:0] ma [2:5]; endmodule\n\
         module top;\n\
           sub u(); reg [4:0] r5;\n\
           initial begin\n\
             u.ma[2]=8'd20; u.ma[3]=8'd30; r5 = 5'd28;\n\
             $display(\"A %0d\", u.ma[~r5]);\n\
             u.ma[~r5] = 8'd99;\n\
             $display(\"B %0d\", u.ma[3]);\n\
             $finish;\n\
           end\n\
         endmodule\n");
    assert!(out.contains("A 30"), "{out}");
    assert!(out.contains("B 99"), "{out}");
}

/// A vita-INTERNAL anchor: one bit pattern, three code paths in the oracle,
/// three different answers.
///
/// `0xFFFF_FFFD` reached through an UNSIGNED-typed index is 4294967293 by
/// value — out of range for every net here — so vita drops it everywhere, and
/// writes only when the index is genuinely signed (`E`). Measured on iverilog
/// 13, which instead reinterprets the low 32 bits as `i32` for a RUNTIME array
/// word but not for a packed bit offset and not for a CONSTANT array word:
///
/// | row | index                        | iverilog | pre-fix | vita now |
/// |-----|------------------------------|----------|---------|----------|
/// | A   | `mg[u32]`        array word  | writes   | writes  | **drops**|
/// | B   | `mg[$unsigned(s32)]`         | writes   | writes  | **drops**|
/// | C   | `dn[u32]`        bit offset  | drops    | writes  | drops    |
/// | D   | `dn[$unsigned(s32)]`         | drops    | writes  | drops    |
/// | E   | `dn[s32]`        signed      | writes   | writes  | writes   |
/// | F   | `mg[$unsigned(-32'sd3)]` const| drops   | writes  | drops    |
///
/// iverilog is not self-contradictory here; it has one rule per path — a
/// RUNTIME array-word index is its low 32 bits read as `i32`, a CONSTANT one
/// keeps its true value, a packed bit offset keeps its true value. vita
/// answers by VALUE on all three, which MATCHES the oracle on the constant and
/// packed paths (C, D, F) and diverges only on the runtime array-word path
/// (A, B). The pre-fix column was uniform 32-bit wrap and so agreed only where
/// the oracle wraps. IEEE 1364 §5.2.1 has no reinterpretation step, so this
/// test pins vita's answer, NOT the oracle's, and the table above is why.
#[test]
fn one_bit_pattern_three_oracle_answers_vita_answers_by_value() {
    let out = run_loud(
        "module top;\n\
           reg [7:0] mg [-3:2]; reg [-2:-33] dn;\n\
           reg [31:0] u32; reg signed [31:0] s32;\n\
           initial begin\n\
             mg[-3]=8'd11; dn=0; u32 = 32'hFFFF_FFFD; s32 = -32'sd3;\n\
             mg[u32] = 8'hA5;                   $display(\"A %0d\", mg[-3]);\n\
             mg[$unsigned(s32)] = 8'hB6;        $display(\"B %0d\", mg[-3]);\n\
             dn[u32] = 1'b1;                    $display(\"C %h\", dn);\n\
             dn = 0; dn[$unsigned(s32)] = 1'b1; $display(\"D %h\", dn);\n\
             dn = 0; dn[s32] = 1'b1;            $display(\"E %h\", dn);\n\
             mg[$unsigned(-32'sd3)] = 8'hC7;    $display(\"F %0d\", mg[-3]);\n\
             $finish;\n\
           end\n\
         endmodule\n",
    );
    assert!(out.contains("A 11"), "{out}");
    assert!(out.contains("B 11"), "{out}");
    assert!(out.contains("C 00000000"), "{out}");
    assert!(out.contains("D 00000000"), "{out}");
    assert!(out.contains("E 40000000"), "{out}");
    assert!(out.contains("F 11"), "{out}");
}
