//! The `-` (left-justify) format flag in `$display`-family format strings
//! (`%-5d`, `%-8h`, `%-8.2f`, `%-5s`, …). vita's format parser only recognized
//! the `0` flag + width digits, so it broke on the `-`: it echoed the spec
//! literally (`%-5d`) and dumped the unconsumed argument at the end in the
//! default radix — a broad silent-wrong across ALL conversions (d/s/h/o/b/f/e/g)
//! and ALL sinks ($display/$write/$monitor/$strobe/$fdisplay/$swrite/$sformatf,
//! which share one `format_args_str`). `-` renders the content at its natural
//! width then RIGHT-pads spaces to the field width, overriding the `0` flag.
//! Every value is pinned to LIVE iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_lj_{}_{n}", std::process::id()));
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
fn left_justify_decimal() {
    // `%-5d` of 42 → "42   "; a negative counts its sign in the field.
    let out = run("module top; initial begin \
         $display(\"[%-5d][%-5d]\", 42, -42); #1 $finish; end endmodule\n");
    assert!(
        out.contains("[42   ][-42  ]"),
        "left-justify %d; got:\n{out}"
    );
}

#[test]
fn left_justify_string() {
    // `%-5s` right-pads spaces; a string longer than the width is never truncated.
    let out = run("module top; initial begin \
         $display(\"[%-5s][%-2s]\", \"hi\", \"hello\"); #1 $finish; end endmodule\n");
    assert!(
        out.contains("[hi   ][hello]"),
        "left-justify %s; got:\n{out}"
    );
}

#[test]
fn left_justify_radix_hob() {
    // The content is the base-natural zero-padded digit string (8'hA → "0a",
    // 8'o7 → "007", 4'b11 → "0011"), then space right-pad to the field width.
    let out = run("module top; initial begin \
         $display(\"[%-6h][%-6o][%-6b]\", 8'hA, 8'o7, 4'b11); #1 $finish; end endmodule\n");
    assert!(
        out.contains("[0a    ][007   ][0011  ]"),
        "left-justify radix; got:\n{out}"
    );
}

#[test]
fn left_justify_real() {
    // `%-8.2f` → "3.14    "; `%-10.3e` → "3.142e+00 ".
    let out = run("module top; real x; initial begin x = 3.14159; \
         $display(\"[%-8.2f][%-10.3e]\", x, x); #1 $finish; end endmodule\n");
    assert!(
        out.contains("[3.14    ][3.142e+00 ]"),
        "left-justify real; got:\n{out}"
    );
}

#[test]
fn dash_overrides_zero_flag() {
    // `%-05d`: the `-` OVERRIDES the `0` — spaces, not zeros (iverilog-pinned).
    let out = run("module top; initial begin \
         $display(\"[%-05d][%05d]\", 42, 42); #1 $finish; end endmodule\n");
    assert!(out.contains("[42   ][00042]"), "- overrides 0; got:\n{out}");
}

#[test]
fn dash_no_width_uses_default_field() {
    // `%-d` (no explicit width) left-justifies in the operand's DEFAULT decimal
    // field width (11 for a 32-bit int) → "42" + 9 spaces. `%-0d` is minimal.
    let out = run("module top; initial begin \
         $display(\"[%-d][%-0d]\", 42, 42); #1 $finish; end endmodule\n");
    assert!(
        out.contains("[42         ][42]"),
        "%-d default width / %-0d minimal; got:\n{out}"
    );
}

#[test]
fn content_wider_than_field_is_not_truncated() {
    // The field width is a minimum; a longer value overflows it unchanged.
    let out = run("module top; initial begin \
         $display(\"[%-2d][%-2s]\", 12345, \"abcdef\"); #1 $finish; end endmodule\n");
    assert!(
        out.contains("[12345][abcdef]"),
        "min-width no truncation; got:\n{out}"
    );
}

#[test]
fn shared_across_all_sinks() {
    // The same `format_args_str` engine feeds $write / $swrite / $sformatf, so the
    // flag works identically in each (parity check).
    let out = run("module top; string s; initial begin \
         $write(\"[%-5d]\\n\", 42); \
         $swrite(s, \"[%-5d]\", 42); $display(\"%s\", s); \
         s = $sformatf(\"[%-5d]\", 42); $display(\"%s\", s); #1 $finish; end endmodule\n");
    // Three identical "[42   ]" lines.
    assert_eq!(
        out.matches("[42   ]").count(),
        3,
        "sink parity; got:\n{out}"
    );
}

#[test]
fn unknown_value_left_justifies() {
    // A partially-unknown %d value collapses to "X", left-justified in the field.
    let out = run("module top; logic [3:0] v; initial begin v = 4'bxx01; \
         $display(\"[%-6d]\", v); #1 $finish; end endmodule\n");
    assert!(
        out.contains("[X     ]"),
        "left-justify unknown; got:\n{out}"
    );
}

#[test]
fn non_dash_specs_unchanged() {
    // Regression guard: without the `-` flag, every spec right-justifies exactly as
    // before (byte-identity of the common forms).
    let out = run("module top; initial begin \
         $display(\"[%5d][%05d][%6h][%8.2f]\", 42, 42, 8'hA, 3.14); #1 $finish; end endmodule\n");
    assert!(
        out.contains("[   42][00042][    0a][    3.14]"),
        "non-dash right-justify unchanged; got:\n{out}"
    );
}

#[test]
fn bare_dash_s_on_packed_reg_left_justifies() {
    // A bare `%-s` (NO explicit width) on a packed reg strips the leading-NUL
    // padding and left-justifies in the reg byte width: 64-bit "cpu0" → "cpu0    "
    // (not the right-justified NUL→space "    cpu0" of a plain `%s`).
    let out = run("module top; reg [8*8:1] nm; initial begin nm = \"cpu0\"; \
         $display(\"|%-s||%s|\", nm, nm); #1 $finish; end endmodule\n");
    assert!(
        out.contains("|cpu0    ||    cpu0|"),
        "bare %-s left-justify on packed reg; got:\n{out}"
    );
}

#[test]
fn width_on_char_strength_scope() {
    // iverilog justifies %c / %v / %m in an explicit field width (both the default
    // right and the `-` left form). Was a pre-existing width-ignore silent-wrong
    // for the non-dash forms; the `-` plumbing corrects both directions.
    let out = run("module top; reg x; initial begin x = 1; \
         $display(\"a[%8c]b[%-8c]\", 65, 65); \
         $display(\"c[%8v]d[%-8v]\", x, x); \
         $display(\"e[%12m]f[%-12m]\"); #1 $finish; end endmodule\n");
    assert!(
        out.contains("a[       A]b[A       ]"),
        "%c width; got:\n{out}"
    );
    assert!(
        out.contains("c[     St1]d[St1     ]"),
        "%v width; got:\n{out}"
    );
    assert!(
        out.contains("e[         top]f[top         ]"),
        "%m width; got:\n{out}"
    );
}
