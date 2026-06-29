//! Default `%d` field width for a SIGNED value (IEEE 1364 — `%d` right-justifies
//! in the operand's default decimal field width). vita's `dec_field_width`
//! computed only the UNSIGNED width (digits of 2^n-1), so a signed operand was one
//! column too narrow — a signed `%d` field is a sign char plus the digits of the
//! most-negative magnitude 2^(n-1) (8-bit → "-128" = 4, 32-bit → "-2147483648" =
//! 11). This is NOT simply unsigned_width+1: for some widths the two coincide
//! (10-bit signed "-512" and unsigned 1023 are both 4), so it must be computed
//! directly. Affects both the unformatted-arg path (push_default_radix) and the
//! explicit `%d` path. Pinned to iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_sfw_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn signed_8bit_field_is_four() {
    // -5 in a signed 8-bit field: "  -5" (width 4 = sign + 3 magnitude digits of 128).
    let out = run("module top; reg signed [7:0] s; initial begin s=-5; \
         $write(\"[%d]\",s); $finish; end endmodule\n");
    assert!(out.contains("[  -5]"), "signed 8-bit width 4; got:\n{out}");
}

#[test]
fn signed_32bit_integer_field_is_eleven() {
    // An `integer` is signed 32-bit: "-2147483648" = 11 wide.
    let out = run("module top; integer i; initial begin i=-5; \
         $write(\"[%d]\",i); $finish; end endmodule\n");
    assert!(
        out.contains("[         -5]"),
        "signed 32-bit width 11; got:\n{out}"
    );
}

#[test]
fn unsigned_widths_unchanged() {
    // Unsigned fields are unaffected: 8-bit → 3, 16-bit → 5, 32-bit → 10.
    let out = run("module top; reg [7:0] u8; reg [15:0] u16; reg [31:0] u32; \
         initial begin u8=5; u16=5; u32=5; \
         $write(\"[%d][%d][%d]\",u8,u16,u32); $finish; end endmodule\n");
    assert!(
        out.contains("[  5][    5][         5]"),
        "unsigned widths 3/5/10; got:\n{out}"
    );
}

#[test]
fn signed_10bit_coincides_with_unsigned() {
    // 10-bit: signed "-512" and unsigned 1023 are both 4 wide — the magnitude must be
    // computed directly (not unsigned_width+1, which would give 5).
    let out = run(
        "module top; reg signed [9:0] s; reg [9:0] u; initial begin s=-5; u=5; \
         $write(\"[%d][%d]\",s,u); $finish; end endmodule\n",
    );
    assert!(
        out.contains("[  -5][   5]"),
        "10-bit both width 4; got:\n{out}"
    );
}

#[test]
fn signed_1bit_field_is_one() {
    // 1-bit signed is the exception: iverilog gives it field width 1 (NOT 2). A
    // value 0 exposes this — "[0]", not "[ 0]". (The lone -1 prints "-1" and
    // overflows the 1-col field, matching iverilog; only 0/positive reveals the
    // width.) An adversarial differential found vita over-padding this case.
    let zero = run("module top; reg signed [0:0] s; initial begin s=0; \
         $write(\"[%d]\",s); $finish; end endmodule\n");
    assert!(
        zero.contains("[0]"),
        "1-bit signed value 0 width 1; got:\n{zero}"
    );
    let neg = run("module top; reg signed [0:0] s; initial begin s=-1; \
         $write(\"<%d>\",s); $finish; end endmodule\n");
    assert!(
        neg.contains("<-1>"),
        "1-bit signed -1 overflows; got:\n{neg}"
    );
}

#[test]
fn signed_unformatted_default_radix() {
    // The unformatted-arg path ($write with no format) uses the same field width:
    // a signed 8-bit -5 padded to 4.
    let out = run("module top; reg signed [7:0] s; initial begin s=-5; \
         $write(s); $write(\"|\\n\"); $finish; end endmodule\n");
    assert!(
        out.contains("  -5|"),
        "unformatted signed width 4; got:\n{out}"
    );
}

#[test]
fn signed_min_zero_no_pad() {
    // `%0d` is minimal width — no padding regardless of signedness.
    let out = run("module top; integer i; initial begin i=-5; \
         $write(\"[%0d]\",i); $finish; end endmodule\n");
    assert!(out.contains("[-5]"), "%0d minimal; got:\n{out}");
}
