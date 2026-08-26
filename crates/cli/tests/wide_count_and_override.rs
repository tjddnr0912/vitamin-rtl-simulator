//! Two axes an external round-34 report opened, which turned out to be one channel
//! each, and a third the report did not know about.
//!
//! **R1 — the count/index channel truncated to 32 bits, silently.** `const_eval_u32`
//! took `BitPacked.val[0] as u32` — the low 32 bits of the value plane — and every
//! consumer of that helper is a COUNT or an INDEX, so the bits it discarded were the
//! ones that decide the answer. `64'hDEAD_BEEF_1234_5678 >> 64'h1_0000_0000` folded
//! to the operand UNSHIFTED where both oracles give 0, `A[2**32]` folded to `A[0]`,
//! and `$bits({(2**32+2){8'hA5}})` built a two-element replication. All at
//! `errors=0 warnings=0`.
//!
//! ⚠️ The report put the boundary at 2**64 because it wrote 128-bit literals. It is
//! **2**32**: `>> 64'h1_0000_0000` is a 64-bit literal the i64 domain carries fine.
//!
//! The two halves land on different rungs, deliberately:
//!  * a SHIFT gets the CORRECT answer — §11.4.10 vacates with zeros, so every amount
//!    at or above the operand's width gives the same result and `fold_shift_count`
//!    saturates. Both oracles agree and vita's own runtime lane already did.
//!  * a COUNT or a SELECT INDEX goes LOUD. There is no consensus answer to adopt:
//!    iverilog truncates a replication count with a warning, verilator refuses it
//!    outright, and for an out-of-range select iverilog says `x` (IEEE §11.5.1) while
//!    verilator — a 2-state tool — says 0. Loud is the honest rung.
//!
//! **R2 — the diagnostics named the wrong thing.** `A[128]` over a 128-bit `A` said
//! *"`A` is wider than 64 bits"*, which is equally true of `A[127]`, and that folds.
//! `B[9]` over an 8-bit `B` said *"the select `B[…]` has no constant-fold arm"*, and
//! the arm exists — `B[3]` folds. Both now name the index.
//!
//! **R5 — a narrow override of a wide-declared parameter was silently discarded.**
//! Neither report named this one; the census found it. On
//! `parameter logic [127:0] K = <128-bit default>`, `#(.K(5))` printed the DEFAULT at
//! exit 0. The wide arm asked `wide_disagreeing_value(&p.value, …)` — `p.value` is the
//! DECLARED DEFAULT's expression, so with an override in flight the two domains
//! disagree by construction and the arm installed the default and returned.
//!
//! Every value here is pinned to LIVE iverilog 13.0, and to verilator 5.050 wherever
//! verilator accepts the construct.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_args(src: &str, args: &[&str]) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_widecnt_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let mut c = Command::new(env!("CARGO_BIN_EXE_vita"));
    for a in args {
        c.arg(a);
    }
    let out = c
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr),
        out.status.code(),
    )
}

fn run(src: &str) -> (String, Option<i32>) {
    run_args(src, &[])
}

// ───────────────────────────── R1: the shift lane ─────────────────────────────

#[test]
fn a_shift_by_more_than_the_operand_width_is_zero_not_unshifted() {
    // PRE: every cell here printed the operand back, at `errors=0 warnings=0`, while
    // vita's OWN runtime lane printed 0 in the same run. Values are iverilog's and
    // verilator's, which agree on all of them.
    let (out, c) = run("module tb;\n\
           localparam logic [127:0] A = 128'he1000000000000000000000000000001;\n\
           localparam logic [63:0]  B = 64'hDEADBEEF12345678;\n\
           localparam logic [127:0] C2 = A >> 128'h10000000000000000;\n\
           localparam logic [127:0] C3 = A >> 128'h10000000000000001;\n\
           localparam logic [127:0] C4 = A << 128'h10000000000000000;\n\
           localparam logic [63:0]  S2 = B >> 64'h100000000;\n\
           localparam logic [63:0]  S3 = B << 64'h100000004;\n\
           logic [127:0] r2; assign r2 = A >> 128'h10000000000000000;\n\
           initial begin #1\n\
             $display(\"C2=%032h C3=%032h C4=%032h\", C2, C3, C4);\n\
             $display(\"S2=%016h S3=%016h r2=%032h\", S2, S3, r2);\n\
           $finish; end\n\
         endmodule\n");
    assert_eq!(c, Some(0), "{out}");
    assert!(
        out.contains(
            "C2=00000000000000000000000000000000 \
             C3=00000000000000000000000000000000 \
             C4=00000000000000000000000000000000"
        ),
        "a shift past the width vacates to zero; got:\n{out}"
    );
    assert!(
        out.contains(
            "S2=0000000000000000 S3=0000000000000000 \
             r2=00000000000000000000000000000000"
        ),
        "the 64-bit operand and the runtime lane agree; got:\n{out}"
    );
}

#[test]
fn an_arithmetic_right_shift_past_the_width_fills_with_the_sign() {
    // The `>>>` twin of the cell above. iverilog and verilator both print -1.
    let (out, c) = run("module tb;\n\
           localparam logic signed [63:0] SB = -64'sd8;\n\
           localparam logic signed [63:0] AS = SB >>> 64'h100000000;\n\
           localparam logic signed [63:0] AP = 64'sd8 >>> 64'h100000000;\n\
           initial begin #1 $display(\"AS=%0d AP=%0d\", AS, AP); $finish; end\n\
         endmodule\n");
    assert_eq!(c, Some(0), "{out}");
    assert!(
        out.contains("AS=-1 AP=0"),
        "sign fill, not no-shift:\n{out}"
    );
}

#[test]
fn a_named_wide_shift_amount_folds_like_its_literal_twin() {
    // PRE this was E3009 blaming `A`. The amount now goes through the same
    // saturating count the literal takes, so the two spellings agree — and agree
    // with iverilog, which prints both lines identically.
    let (out, c) = run("module tb;\n\
           localparam logic [127:0] A  = 128'he1000000000000000000000000000001;\n\
           localparam logic [127:0] SH = 128'h10000000000000004;\n\
           localparam logic [127:0] R  = A >> SH;\n\
           localparam logic [127:0] OK = A >> 128'd4;\n\
           initial begin #1 $display(\"R=%032h OK=%032h\", R, OK); $finish; end\n\
         endmodule\n");
    assert_eq!(c, Some(0), "{out}");
    assert!(
        out.contains(
            "R=00000000000000000000000000000000 \
             OK=0e100000000000000000000000000000"
        ),
        "{out}"
    );
}

// ─────────────────── R1: the count / index lane, which stays loud ───────────────────

#[test]
fn a_replication_count_past_32_bits_is_loud_and_names_the_count() {
    // PRE: `$bits` was 16 — a TWO-element replication built from a count of 2**32+2,
    // at exit 0. iverilog also truncates (it warns `verinum::as_long() truncated`);
    // verilator refuses with *"Value too wide for 32-bits expected in this context"*.
    // There is no consensus value, so vita refuses too — and says which number.
    let (out, c) = run("module tb;\n\
           localparam int RPB = $bits({64'h100000002{8'h5A}});\n\
           initial begin $display(\"RPB=%0d\", RPB); $finish; end\n\
         endmodule\n");
    assert_ne!(c, Some(0), "must not build a 2-element replication:\n{out}");
    assert!(
        out.contains("the replication count 4294967298 does not fit 32 bits"),
        "name the count, not the arm:\n{out}"
    );
}

#[test]
fn a_bit_select_index_past_32_bits_is_loud() {
    // PRE: folded to `A[4]` and answered a bit of the operand. Both oracles disagree
    // with that AND with each other (iverilog `x`, verilator refuses the literal), so
    // loud is the only rung available.
    let (out, c) = run("module tb;\n\
           localparam logic [127:0] A = 128'he1000000000000000000000000000001;\n\
           localparam logic CB = A[64'h100000004];\n\
           initial begin $display(\"CB=%b\", CB); $finish; end\n\
         endmodule\n");
    assert_ne!(c, Some(0), "{out}");
}

#[test]
fn an_indexed_part_select_base_past_32_bits_is_loud_and_names_the_base() {
    let (out, c) = run("module tb;\n\
           localparam logic [63:0] B = 64'hDEADBEEF12345678;\n\
           localparam logic [7:0] PS = B[128'h10000000000000004 +: 8];\n\
           initial begin $display(\"PS=%02h\", PS); $finish; end\n\
         endmodule\n");
    assert_ne!(c, Some(0), "{out}");
    assert!(
        out.contains("the indexed part-select base 18446744073709551620 does not fit"),
        "{out}"
    );
}

#[test]
fn a_count_that_fits_32_bits_still_takes_the_route_it_always_took() {
    // The other side of the gate: nothing below 2**32 changes. `{4{2'b01}}` and a
    // named count both fold exactly as before, and a shift by a value that fits is
    // still the plain shift.
    let (out, c) = run("module tb;\n\
           localparam int N = 3;\n\
           localparam logic [5:0] PV = {N{2'b01}};\n\
           localparam logic [7:0] Q = 8'hA5 >> 4;\n\
           localparam logic [7:0] R = 8'hA5 << 2;\n\
           initial begin #1 $display(\"PV=%b Q=%02h R=%02h\", PV, Q, R); $finish; end\n\
         endmodule\n");
    assert_eq!(c, Some(0), "{out}");
    assert!(out.contains("PV=010101 Q=0a R=94"), "{out}");
}

// ──────────────────────────── R2: the diagnostics ────────────────────────────

#[test]
fn an_out_of_range_constant_select_blames_the_index_not_the_operand() {
    // PRE, in one file: `B[9]` said *"the select `B[…]` has no constant-fold arm"*
    // (false — `B[3]` folds) and `A[128]` said *"`A` is wider than 64 bits"* (true of
    // `A[127]` too, and that folds). Both now name the index and its range, and say
    // what §11.5.1 makes the value.
    let (out, c) = run("module tb;\n\
           localparam logic [7:0]   B = 8'hA5;\n\
           localparam logic [127:0] A = 128'he1000000000000000000000000000001;\n\
           localparam logic C1 = B[9];\n\
           localparam logic C2 = A[128];\n\
           initial begin $display(\"%b %b\", C1, C2); $finish; end\n\
         endmodule\n");
    assert_ne!(c, Some(0), "{out}");
    assert!(
        out.contains("the select index 9 is outside `B`'s range [7:0]"),
        "{out}"
    );
    assert!(
        out.contains("the select index 128 is outside `A`'s range [127:0]"),
        "{out}"
    );
    assert!(
        out.contains("§11.5.1"),
        "cite the rule that makes it x:\n{out}"
    );
    assert!(
        !out.contains("has no constant-fold arm"),
        "the arm exists; that sentence must be gone:\n{out}"
    );
}

#[test]
fn an_in_range_select_of_the_same_operands_still_folds() {
    // The control for the cell above — the claim *"the arm exists"* has to be true.
    let (out, c) = run("module tb;\n\
           localparam logic [7:0]   B = 8'hA5;\n\
           localparam logic [127:0] A = 128'he1000000000000000000000000000001;\n\
           localparam logic C3 = B[3];\n\
           localparam logic C4 = A[127];\n\
           initial begin #1 $display(\"C3=%b C4=%b\", C3, C4); $finish; end\n\
         endmodule\n");
    assert_eq!(c, Some(0), "{out}");
    assert!(
        out.contains("C3=0 C4=1"),
        "iverilog prints the same:\n{out}"
    );
}

// ───────────────── R5: an override of a >64-bit declared parameter ─────────────────

const LEAF: &str =
    "module leaf #(parameter logic [127:0] K = 128'hAAAABBBBCCCCDDDDEEEEFFFF00001111)\n\
                      (output logic [127:0] o); assign o = K; endmodule\n";

#[test]
fn a_narrow_override_of_a_wide_parameter_is_applied_not_dropped() {
    // ⚠️⚠️ PRE printed the DECLARED DEFAULT for all three, at exit 0. Values are
    // iverilog's; verilator agrees.
    let (out, c) = run(&format!(
        "{LEAF}\
         module tb; logic [127:0] a,b,d;\n\
           leaf #(.K(5))            u1(.o(a));\n\
           leaf #(.K(32'hDEADBEEF)) u2(.o(b));\n\
           leaf #(.K(128'h5))       u4(.o(d));\n\
           initial begin #1 $display(\"a=%032h b=%032h d=%032h\", a,b,d); $finish; end\n\
         endmodule\n"
    ));
    assert_eq!(c, Some(0), "{out}");
    assert!(
        out.contains(
            "a=00000000000000000000000000000005 \
             b=000000000000000000000000deadbeef \
             d=00000000000000000000000000000005"
        ),
        "{out}"
    );
}

#[test]
fn a_signed_override_sign_extends_past_the_i64_lane_and_an_unsigned_one_does_not() {
    // ⚠️⚠️ This is the pair that needs the override EXPRESSION's signedness. All of
    // `-1`, `8'shFF` and `64'hFFFF_FFFF_FFFF_FFFF` reach `bind_one_param` as the same
    // i64; iverilog and verilator extend the first two with ones and the third with
    // zeros. PRE answered `0000000000000000ffffffffffffffff` for all three — the i64
    // lane zero-extends on a read past bit 63, so a sign that has to reach bit 127
    // was lost.
    let (out, c) = run(&format!(
        "{LEAF}\
         module tb; logic [127:0] c6,c8,c10,c11,c13,c14;\n\
           leaf #(.K(64'hFFFFFFFFFFFFFFFF)) x6(.o(c6));\n\
           leaf #(.K(-1))                   x8(.o(c8));\n\
           leaf #(.K(8'shFF))               x10(.o(c10));\n\
           leaf #(.K(32'shFFFFFFFF))        x11(.o(c11));\n\
           leaf #(.K('1))                   x13(.o(c13));\n\
           leaf #(.K('0))                   x14(.o(c14));\n\
           initial begin #1 $display(\"6=%032h 8=%032h 10=%032h\", c6,c8,c10);\n\
             $display(\"11=%032h 13=%032h 14=%032h\", c11,c13,c14); $finish; end\n\
         endmodule\n"
    ));
    assert_eq!(c, Some(0), "{out}");
    assert!(
        out.contains("6=0000000000000000ffffffffffffffff"),
        "an UNSIGNED 64-bit literal zero-extends:\n{out}"
    );
    for (n, v) in [
        ("8", "ffffffffffffffffffffffffffffffff"),
        ("10", "ffffffffffffffffffffffffffffffff"),
        ("11", "ffffffffffffffffffffffffffffffff"),
        ("13", "ffffffffffffffffffffffffffffffff"),
    ] {
        assert!(out.contains(&format!("{n}={v}")), "cell {n}:\n{out}");
    }
    assert!(out.contains("14=00000000000000000000000000000000"), "{out}");
}

#[test]
fn a_wide_literal_override_applies_on_every_channel() {
    // PRE: W3056 *"not a constant; default kept"* followed by E3009 — two diagnostics
    // contradicting each other about one override that IS a constant. `bits` is now
    // one of the channels both conjunctions ask about.
    let (out, c) = run(&format!(
        "{LEAF}\
         module mid #(parameter logic [127:0] K = 128'hBBBB) (output logic [127:0] o);\n\
           leaf #(.K(K)) i(.o(o)); endmodule\n\
         module tb; logic [127:0] c20,c21,c24,c25,c26,c28,c30;\n\
           localparam logic [127:0] W = 128'hdeadbeef_00000000_00000000_00000001;\n\
           leaf #(.K(128'hdeadbeef_00000000_00000000_00000001)) x20(.o(c20));\n\
           leaf #(128'hdeadbeef_00000000_00000000_00000002)     x21(.o(c21));\n\
           leaf #(.K(65'h1_0000_0000_0000_0000))                x24(.o(c24));\n\
           leaf #(.K({{2{{64'hdeadbeefcafebabe}}}}))            x25(.o(c25));\n\
           leaf #(.K({{64'h0,64'h5}}))                          x26(.o(c26));\n\
           leaf #(.K(W))                                        x28(.o(c28));\n\
           mid  #(.K(128'hdeadbeef_0_0_9))                      x30(.o(c30));\n\
           initial begin #1\n\
             $display(\"20=%032h 21=%032h 24=%032h\", c20,c21,c24);\n\
             $display(\"25=%032h 26=%032h\", c25,c26);\n\
             $display(\"28=%032h 30=%032h\", c28,c30); $finish; end\n\
         endmodule\n"
    ));
    assert_eq!(c, Some(0), "{out}");
    for (n, v) in [
        ("20", "deadbeef000000000000000000000001"),
        ("21", "deadbeef000000000000000000000002"),
        ("24", "00000000000000010000000000000000"),
        ("25", "deadbeefcafebabedeadbeefcafebabe"),
        // ⚠️ A concat override folds to a 128-bit constant whose VALUE is 5. The i64
        // walk declines the concatenation, so before the wide fold was read back this
        // cell fell through to the declared default — loud traded for silent.
        ("26", "00000000000000000000000000000005"),
        ("28", "deadbeef000000000000000000000001"),
        ("30", "000000000000000000000deadbeef009"),
    ] {
        assert!(out.contains(&format!("{n}={v}")), "cell {n}:\n{out}");
    }
    assert!(
        !out.contains("default kept"),
        "the override IS applied; that warning must be gone:\n{out}"
    );
}

#[test]
fn the_g_channel_carries_the_same_values_as_the_instance_channel() {
    let src = "module tb #(parameter logic [127:0] K = 128'hAAAA);\n\
                 initial begin #1 $display(\"K=%032h\", K); $finish; end\n\
               endmodule\n";
    for (arg, want) in [
        ("K=7", "00000000000000000000000000000007"),
        ("K=32'h7", "00000000000000000000000000000007"),
        ("K=64'hFFFFFFFFFFFFFFFF", "0000000000000000ffffffffffffffff"),
        ("K='1", "ffffffffffffffffffffffffffffffff"),
        ("K=128'hdeadbeef_0_0_1", "000000000000000000000deadbeef001"),
        ("K=-1", "ffffffffffffffffffffffffffffffff"),
    ] {
        let (out, c) = run_args(src, &["-G", arg]);
        assert_eq!(c, Some(0), "-G {arg}:\n{out}");
        assert!(out.contains(&format!("K={want}")), "-G {arg}:\n{out}");
    }
}

#[test]
fn an_override_that_fits_the_integer_lane_keeps_the_parameter_usable_as_a_width() {
    // ⭐ A pre-existing correct→loud regression this slice repairs on the way past.
    // `parameter logic [127:0] K = 128'd12` is usable as `logic [K-1:0]` with no
    // override and was E3009 *"`K` is wider than 64 bits"* the moment ANY wide-literal
    // override appeared — because the override arm installed into `wide_param_bits`
    // unconditionally. The install is now keyed on there being bits the i64 lane
    // cannot carry. iverilog prints `a=12 b=20`.
    let (out, c) = run(
        "module leaf #(parameter logic [127:0] K = 128'd12) (output logic [7:0] w);\n\
           logic [K-1:0] v; assign w = $bits(v); endmodule\n\
         module tb; logic [7:0] a,b;\n\
           leaf u1(.w(a));\n\
           leaf #(.K(128'd20)) u2(.w(b));\n\
           initial begin #1 $display(\"a=%0d b=%0d\",a,b); $finish; end\n\
         endmodule\n",
    );
    assert_eq!(c, Some(0), "{out}");
    assert!(out.contains("a=12 b=20"), "{out}");
}

#[test]
fn an_override_with_no_recorded_signedness_stays_on_its_old_route() {
    // The fail-closed half. A `defparam`'s expression is folded to an i64 by its
    // collector before the record exists, so `ResolvedOverride::signed` is `None` and
    // the extension declines rather than guessing. A POSITIVE defparam is unaffected
    // and must keep working — iverilog prints 7.
    let (out, c) = run(&format!(
        "{LEAF}\
         module tb; logic [127:0] o;\n\
           leaf u(.o(o));\n\
           defparam u.K = 32'h7;\n\
           initial begin #1 $display(\"o=%032h\", o); $finish; end\n\
         endmodule\n"
    ));
    assert_eq!(c, Some(0), "{out}");
    assert!(out.contains("o=00000000000000000000000000000007"), "{out}");
}

#[test]
fn a_context_determined_override_expression_is_still_honestly_loud() {
    // `128'h1 << 100` has a context-determined top, so `override_bits` declines it —
    // the rule that keeps a shift from being folded at the wrong width. iverilog
    // prints `00000010000000000000000000000000`; vita refuses and says so. This is a
    // recorded residue, pinned so that closing it is a deliberate act.
    let (out, c) = run(&format!(
        "{LEAF}\
         module tb; logic [127:0] o;\n\
           leaf #(.K(128'h1 << 100)) u(.o(o));\n\
           initial begin #1 $display(\"o=%032h\", o); $finish; end\n\
         endmodule\n"
    ));
    assert_ne!(c, Some(0), "must stay loud:\n{out}");
}
