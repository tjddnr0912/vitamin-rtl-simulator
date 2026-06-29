//! Explicit field width `%Ns` for the `%s` conversion (IEEE 1364 — `%s`
//! right-justifies in the field width, a MINIMUM that a longer string overflows
//! rather than truncating). vita's `%s` arm previously ignored `field_width`
//! entirely, so `%5s`/`%05s`/`%10s` were not padded — a silent-wrong. The content
//! is the leading-NUL-stripped form for an explicit-width `%Ns`/`%0s` on a packed
//! reg, the full reg-width form (NUL→space) for a bare `%s`, and the exact text for
//! a string literal / dynamic string. Pinned to iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_sfw2_{}_{n}", std::process::id()));
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
fn literal_right_justified_in_field() {
    // `%5s` of "AB" → "   AB" (right-justified, width 5).
    let out = run("module top; initial begin $display(\"[%5s]\",\"AB\"); $finish; end endmodule\n");
    assert!(out.contains("[   AB]"), "%5s literal; got:\n{out}");
}

#[test]
fn field_width_is_minimum_not_truncating() {
    // `%2s` of "ABCDE" overflows the width (5 chars), never truncated.
    let out =
        run("module top; initial begin $display(\"[%2s]\",\"ABCDE\"); $finish; end endmodule\n");
    assert!(out.contains("[ABCDE]"), "%2s overflow; got:\n{out}");
}

#[test]
fn dynamic_string_right_justified() {
    // `%10s` of the dynamic string "hi" → 8 spaces + "hi".
    let out = run("module top; string s; initial begin s=\"hi\"; \
         $display(\"[%10s]\",s); $finish; end endmodule\n");
    assert!(out.contains("[        hi]"), "%10s dyn string; got:\n{out}");
}

#[test]
fn packed_reg_ns_uses_stripped_content() {
    // A packed reg's `%Ns` content is its leading-NUL-stripped string, padded to N:
    // "AB" in a 4-byte reg → `%6s` "    AB", `%2s` "AB" (exact fit).
    let out = run("module top; reg [8*4:1] s; initial begin s=\"AB\"; \
         $display(\"a[%6s]b[%2s]\",s,s); $finish; end endmodule\n");
    assert!(
        out.contains("a[    AB]b[AB]"),
        "%Ns packed stripped; got:\n{out}"
    );
}

#[test]
fn zero_prefixed_width_strips_and_pads() {
    // `%05s` of "hello" in an 8-byte reg → stripped "hello", padded to 5 = "hello".
    let out = run("module top; reg [8*8:1] s; initial begin s=\"hello\"; \
         $display(\"[%05s]\",s); $finish; end endmodule\n");
    assert!(out.contains("[hello]"), "%05s strip+pad; got:\n{out}");
}

#[test]
fn bare_s_and_zero_s_unchanged() {
    // Byte-identity guard: bare `%s` still pads a packed reg to its full width
    // (NUL→space), and `%0s` still strips with no field-width padding.
    let out = run("module top; reg [8*8:1] s; initial begin s=\"hi\"; \
         $display(\"a[%s]b[%0s]\",s,s); $finish; end endmodule\n");
    assert!(
        out.contains("a[      hi]b[hi]"),
        "%s/%0s unchanged; got:\n{out}"
    );
}
