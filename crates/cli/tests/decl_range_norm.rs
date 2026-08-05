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
/// | row | index                         | iverilog | verilator | vita |
/// |-----|-------------------------------|----------|-----------|------|
/// | A   | `mg[u32]`         array word  | writes   | writes    | writes |
/// | B   | `mg[$unsigned(s32)]`          | writes   | writes    | writes |
/// | C   | `dn[u32]`         bit offset  | drops    | writes    | drops  |
/// | D   | `dn[$unsigned(s32)]`          | drops    | writes    | drops  |
/// | E   | `dn[s32]`         signed      | writes   | writes    | writes |
/// | F   | `mg[$unsigned(-32'sd3)]` const| drops    | writes    | drops  |
///
/// iverilog is not self-contradictory here; it has one rule per path — a
/// RUNTIME array-word index is its low 32 bits read as `i32`, a CONSTANT one
/// keeps its true value, a packed bit offset keeps its true value — and vita
/// now matches it on all three.
///
/// §4.5.308 left rows A and B dropping, on the argument that IEEE 1364 §5.2.1
/// has no reinterpretation step so vita should answer by VALUE everywhere and
/// be "ahead of the oracle" in that one cell. §4.5.310 measured the cell against
/// a SECOND oracle and that argument did not survive: verilator 5.050 lands on
/// the same element iverilog does, so vita was not ahead, it was alone. (On C/D
/// the two oracles genuinely split, and there vita stays with iverilog; on F
/// they split too, and there the ladder decides — a statically known index
/// outside a statically known range is exactly what a tool should say out loud.)
/// The rows are unchanged; what changed is which column vita is in and why.
#[test]
fn one_bit_pattern_three_index_paths_each_pinned_to_iverilog() {
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
    // A/B: `0xFFFF_FFFD` read as `i32` is -3, so both writes land on `mg[-3]`
    // — 8'hA5 then 8'hB6. They read 11 (untouched) before §4.5.310.
    assert!(out.contains("A 165"), "{out}");
    assert!(out.contains("B 182"), "{out}");
    assert!(out.contains("C 00000000"), "{out}");
    assert!(out.contains("D 00000000"), "{out}");
    assert!(out.contains("E 40000000"), "{out}");
    // F: the constant path drops, so `mg[-3]` still holds what B wrote.
    assert!(out.contains("F 182"), "{out}");
}

/// The cells where the SIGNED seal actually changes the answer.
///
/// Every other signed-index test in this file uses `reg [-2:-33] dn`, i.e. a
/// NEGATIVE declared LSB. There the old emission was `raw + |k|` and the new one
/// is `$signed(idx) − k`, and those two agree modulo 2³² for every result that
/// lands in range — so reverting the signed half of the seal passed all nine of
/// them (measured; the differential review's M4). The seal is only observable
/// where the normalization is a SUBTRACTION the old form got wrong: an ASCENDING
/// range (`k − idx`) and a POSITIVE non-zero LSB.
///
/// Oracle: iverilog 13 on all six rows.
#[test]
fn the_signed_seal_is_pinned_where_it_is_observable() {
    let src = "module top;\n\
       reg [0:7] asc; reg [3:10] ascn; reg [10:3] dsc;\n\
       reg signed [7:0] s8; byte bb;\n\
       function automatic signed [7:0] fs(input [7:0] x); fs = -8'sd6; endfunction\n\
       initial begin\n\
         s8 = -8'sd6; bb = -8'sd6;\n\
         asc = 8'b0; ascn = 8'b0; dsc = 8'b0;\n\
         asc[~s8]  = 1'b1; $display(\"ASC %b\", asc);\n\
         ascn[~bb] = 1'b1; $display(\"ASN %b\", ascn);\n\
         dsc[~s8]  = 1'b1; $display(\"DSC %b\", dsc);\n\
         asc = 8'b0; ascn = 8'b0; dsc = 8'b0;\n\
         asc[fs(8'd0)+8]  = 1'b1; $display(\"ASF %b\", asc);\n\
         ascn[fs(8'd0)+8] = 1'b1; $display(\"ANF %b\", ascn);\n\
         dsc[fs(8'd0)+8]  = 1'b1; $display(\"DSF %b\", dsc);\n\
         $finish;\n\
       end\n\
     endmodule\n";
    let out = run(src);
    // `~s8` is 5 at the index's own eight bits. Unsealed it evaluates at
    // thirty-two (`~(-6)` widened) and misses every one of these.
    // Values are the ORACLE's, measured, not predicted — my hand answers for the
    // `fs()+8` rows were wrong twice before iverilog settled them.
    for (tag, want) in [
        ("ASC", "00000100"), // [0:7]  ascending, zero base — PRE wrote nothing
        ("ASN", "00100000"), // [3:10] ascending, base 3     — PRE wrote nothing
        ("DSC", "00000100"), // [10:3] descending, base 3    — PRE wrote nothing
        ("ASF", "00100000"), // signed function return, ascending — PRE wrote nothing
        // The last two are CONTROLS: `fs(0)+8` is 2, which is below base 3, so
        // both readings agree it is out of range and both write nothing. They
        // are here so a mutation that simply stops writing cannot pass.
        ("ANF", "00000000"),
        ("DSF", "00000000"),
    ] {
        assert!(
            out.lines().any(|l| l == format!("{tag} {want}")),
            "line `{tag} {want}` missing\n{out}"
        );
    }
}

/// The UNSIGNED half of the seal is FROZEN at its pre-§4.5.309 decision, and
/// these are the cells that made that necessary.
///
/// Replacing the old hand predicate with the canonical rule here proved
/// `$urandom`/`$urandom_range`/`$stime` unsigned for the first time, which put
/// them through the seal — and under a NEGATIVE declared base the unsealed
/// emission is `raw + |k|` at thirty-two bits, which WRAPS, and the wrap is the
/// answer iverilog gives. Sealed at thirty-three bits it becomes 4294967296:
/// out of range, so twelve unpacked cells went correct → `x` + E4002 + exit 1
/// and one packed WRITE went correct → silently dropped.
///
/// Which unsigned shapes to seal is therefore not a signedness question at all
/// — it is the array-word i32-reinterpretation question in ROADMAP §2, and no
/// width/base predicate separates the two groups (`reg [31:0] ix` and `$stime`
/// are both 32-bit unsigned under the same negative base, and the old decision
/// is right about the first only when sealed and about the second only when
/// not). Until that is decided, these rows must not move.
#[test]
fn the_unsigned_seal_keeps_its_frozen_decision_under_a_negative_base() {
    let src = "module top;\n\
       reg [7:0] ma [-3:2];\n\
       reg [4:-3] pk;\n\
       integer ii, q;\n\
       initial begin\n\
         for (q=-3; q<=2; q=q+1) ma[q] = (q+40);\n\
         ii = -3; pk = 8'b0;\n\
         $display(\"U1 %0d\", ma[($urandom%1)+ii]);\n\
         $display(\"U2 %0d\", ma[$urandom_range(0,0)+ii]);\n\
         $display(\"U3 %0d\", ma[$stime+ii]);\n\
         ma[$stime+ii] = 8'd77; $display(\"U4 %0d\", ma[-3]);\n\
         pk[$stime+ii] = 1'b1;  $display(\"U5 %b\", pk);\n\
         $finish;\n\
       end\n\
     endmodule\n";
    let out = run(src);
    // `run` already asserts exit 0 — which is half the point: U1..U3 went to
    // `x` plus an E4002 and exit 1, so a regression here is loud, and U5 is the
    // one that went SILENT.
    for (tag, want) in [
        ("U1", "37"),
        ("U2", "37"),
        ("U3", "37"),
        ("U4", "77"),
        ("U5", "00000001"),
    ] {
        assert!(
            out.lines().any(|l| l == format!("{tag} {want}")),
            "line `{tag} {want}` missing — the unsigned seal moved\n{out}"
        );
    }
}

/// A CLASS FIELD as the index, paired with its plain twin.
///
/// The field's sign is a vita sidecar, not a language rule — the `Signal` the
/// index reads sits on the 32-bit HANDLE net — so `elaborate`'s driver over the
/// shared width rule applies that map inline as it fills. Dropping the arm makes
/// `c.sb` (a `byte`) read as unsigned 32 and the seal declines: measured, all
/// three rows below go to "wrote nothing" while the twin keeps working.
///
/// Oracle: iverilog 13 compiles both forms and agrees with the twin on all four.
#[test]
fn a_class_field_index_is_sealed_like_its_plain_twin() {
    let mk = |cls: bool| {
        let (decl, init, sb, u5) = if cls {
            (
                "class C; byte sb; bit [4:0] u5; endclass\nmodule top;\n  C c;",
                "c = new(); c.sb = -8'sd6; c.u5 = 5'd6;",
                "c.sb",
                "c.u5",
            )
        } else {
            (
                "module top;\n  byte sb; reg [4:0] u5;",
                "sb = -8'sd6; u5 = 5'd6;",
                "sb",
                "u5",
            )
        };
        format!(
            "{decl}\n\
               reg [33:2] d2; reg [-2:-33] dn; reg [0:7] asc;\n\
               initial begin\n\
                 {init}\n\
                 d2 = 32'h0; dn = 32'h0; asc = 8'b0;\n\
                 d2[~{sb}] = 1'b1;  $display(\"E1 %h\", d2);\n\
                 dn[~{sb}] = 1'b1;  $display(\"E2 %h\", dn);\n\
                 asc[~{sb}] = 1'b1; $display(\"E3 %b\", asc);\n\
                 d2 = 32'h0; d2[~{u5}] = 1'b1; $display(\"E4 %h\", d2);\n\
                 $finish;\n\
               end\n\
             endmodule\n"
        )
    };
    let cls = run(&mk(true));
    let plain = run(&mk(false));
    assert_eq!(
        cls, plain,
        "a class-field index must lower like its plain twin"
    );
    // …and an anti-vacuity floor: the twin must actually be doing the thing.
    // `E2` is a control — `~c.sb` is 5, which is above `[-2:-33]`'s high end, so
    // both readings write nothing and it cannot carry the assertion alone.
    for want in ["E1 00000008", "E3 00000100", "E4 00800000", "E2 00000000"] {
        assert!(
            plain.lines().any(|l| l == want),
            "line `{want}` missing\n{plain}"
        );
    }
}
