//! The `+` (force-sign) format flag in `$display`-family format strings
//! (`%+d`, `%+8.2f`, `%+05d`). vita's format parser recognized `-`/`0`/width but
//! not `+`, so on `%+d` it echoed the spec literally and dumped the argument at
//! the end — a silent-wrong for signed-decimal and real formatting. The `+`
//! forces a leading `+` on a NON-negative `%d`/`%f`/`%e`/`%g` value (a negative
//! already carries `-`; `nan` gets none); iverilog IGNORES it for the unsigned
//! `%h`/`%o`/`%b`, `%t`, `%c`. The same slice also fixes a pre-existing real
//! zero-pad gap (`%08.2f` space-padded instead of zero-padding). Pinned to LIVE
//! iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_pf_{}_{n}", std::process::id()));
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
fn plus_forces_sign_on_decimal() {
    // `%+d`: "+42" for a non-negative, "-42" for a negative, "+0" for zero — each
    // right-justified in the operand's default decimal width (11 for a 32-bit int).
    let out = run("module top; initial begin \
         $display(\"[%+d][%+d][%+d]\", 42, -42, 0); #1 $finish; end endmodule\n");
    assert!(
        out.contains("[        +42][        -42][         +0]"),
        "plus on %d; got:\n{out}"
    );
}

#[test]
fn plus_with_width_and_zeropad() {
    // `%+5d` right-justifies with the sign; `%+05d` zero-pads AFTER the `+`
    // ("+0042", not "0+042"); `%+0d` is minimal.
    let out = run("module top; initial begin \
         $display(\"[%+5d][%+05d][%+0d][%+05d]\", 42, 42, 42, -42); #1 $finish; end endmodule\n");
    assert!(
        out.contains("[  +42][+0042][+42][-0042]"),
        "plus width/zeropad; got:\n{out}"
    );
}

#[test]
fn plus_and_dash_order_independent() {
    // `-` and `+` combine in either order → left-justify with a forced sign.
    let out = run("module top; initial begin \
         $display(\"[%-+5d][%+-5d]\", 42, 42); #1 $finish; end endmodule\n");
    assert!(
        out.contains("[+42  ][+42  ]"),
        "plus+dash order-independent; got:\n{out}"
    );
}

#[test]
fn plus_on_reals() {
    // `+` forces the sign on `%f`/`%e`/`%g`, counting toward the field width.
    let out = run("module top; real x; initial begin x = 3.14; \
         $display(\"[%+f][%+8.2f][%+e][%+g]\", x, x, x, x); #1 $finish; end endmodule\n");
    assert!(out.contains("[+3.140000]"), "%+f; got:\n{out}");
    assert!(out.contains("[   +3.14]"), "%+8.2f; got:\n{out}");
    assert!(out.contains("[+3.140000e+00]"), "%+e; got:\n{out}");
    assert!(out.contains("[+3.14]"), "%+g; got:\n{out}");
}

#[test]
fn plus_on_infinity_but_not_nan() {
    // `+inf` gets a sign; `nan` does NOT (iverilog-pinned).
    let out = run(
        "module top; real p, n; initial begin p = 1.0/0.0; n = 0.0/0.0; \
         $display(\"[%+f][%+f]\", p, n); #1 $finish; end endmodule\n",
    );
    assert!(out.contains("[+inf][nan]"), "plus on inf/nan; got:\n{out}");
}

#[test]
fn plus_ignored_for_unsigned_radix() {
    // `%h`/`%o`/`%b` are unsigned — the `+` flag is a no-op (no leading sign).
    let out = run("module top; initial begin \
         $display(\"[%+h][%+o][%+b]\", 8'hA, 8'o17, 4'b101); #1 $finish; end endmodule\n");
    assert!(
        out.contains("[0a][017][0101]"),
        "plus ignored for radix; got:\n{out}"
    );
}

#[test]
fn plus_on_unknown_and_unsigned_decimal() {
    // `%+d` of an x/z value collapses to "+X"; of an unsigned reg it still forces
    // the "+".
    let out = run("module top; logic [3:0] v; logic [7:0] u; initial begin \
         v = 4'bxx01; u = 8'd200; $display(\"[%+d][%+d]\", v, u); #1 $finish; end endmodule\n");
    assert!(
        out.contains("[+X][+200]"),
        "plus on x / unsigned; got:\n{out}"
    );
}

#[test]
fn real_zeropad_bonus() {
    // Pre-existing gap fixed by this slice: `%08.2f` zero-pads (was space-padded).
    // Sign-aware: "-0003.14"; `%+08.2f` → "+0003.14"; `%08g` → "000012.5".
    let out = run(
        "module top; real a, b, c; initial begin a = 3.14; b = -3.14; c = 12.5; \
         $display(\"[%08.2f][%08.2f][%+08.2f][%08g]\", a, b, a, c); #1 $finish; end endmodule\n",
    );
    assert!(
        out.contains("[00003.14][-0003.14][+0003.14][000012.5]"),
        "real zero-pad; got:\n{out}"
    );
}

#[test]
fn nonfinite_ignores_zeropad() {
    // C/iverilog IGNORE the `0` flag for a non-finite real — inf/nan space-pad
    // regardless (adversarial-review regression: was zero-padded to "00000inf").
    let out = run(
        "module top; real p, n; initial begin p = 1.0/0.0; n = 0.0/0.0; \
         $display(\"[%08.2f][%08.2f][%+08.2f]\", p, n, p); #1 $finish; end endmodule\n",
    );
    assert!(
        out.contains("[     inf][     nan][    +inf]"),
        "non-finite ignores zero-pad; got:\n{out}"
    );
}

#[test]
fn plus_across_sinks() {
    // The shared `format_args_str` engine feeds $write / $swrite / $sformatf.
    let out = run("module top; string s; initial begin \
         $write(\"[%+d]\\n\", 5); \
         $swrite(s, \"[%+d]\", 5); $display(\"%s\", s); \
         s = $sformatf(\"[%+d]\", 5); $display(\"%s\", s); #1 $finish; end endmodule\n");
    // Three identical "[         +5]" (right-justified in width 11).
    assert_eq!(
        out.matches("[         +5]").count(),
        3,
        "plus sink parity; got:\n{out}"
    );
}

#[test]
fn non_plus_specs_unchanged() {
    // Regression guard: without `+`, every spec renders exactly as before.
    let out = run("module top; real x; initial begin x = 3.14; \
         $display(\"[%d][%5d][%h][%8.2f]\", 42, 42, 8'hA, x); #1 $finish; end endmodule\n");
    assert!(
        out.contains("[         42][   42][0a][    3.14]"),
        "non-plus unchanged; got:\n{out}"
    );
}
