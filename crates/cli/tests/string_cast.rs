//! IEEE 1800-2017 §6.16 integral→string conversion — the `string'(e)` static cast
//! (§6.24.1) AND the implicit `string s = <integral>` assignment that shares its rule.
//!
//! ONE rule, one engine funnel (`Value::to_sv_string_bytes`): the operand's packed
//! bytes MSB-first, every UNKNOWN bit read as 0, and every 0x00 byte dropped. A value
//! that is already a string is not converted.
//!
//! Oracles. Every value below was measured live on `iverilog -g2012` 13.0 and
//! `verilator --binary --timing` 5.052; a test says which tools back it:
//!  - BOTH: the implicit-assignment family, and every `string'(e)` whose result is
//!    read back through a `string` VARIABLE.
//!  - verilator ONLY: the cells iverilog cannot run at all — it aborts the compile on
//!    a string-vs-integral compare and on a string formal bound to an integral actual
//!    (`draw_eval_vec4` assertion / `%cmp/u operand width mismatch`), and it has no
//!    `.getc` string method ("Method getc is not a string method").
//!  - SPLIT (recorded, NOT pinned to one side): `%s` of a `string'(e)` that is never
//!    stored. iverilog keeps the cast's PACKED operand there — `$display("%s",
//!    string'(24'h610062))` prints `a b` and `$display(string'(x))` prints the decimal
//!    `6382179` — while verilator prints the converted string. vita follows verilator
//!    (and §6.16), because in vita the cast IS a string-domain value; the iverilog text
//!    is recorded in the test that covers the cell.
//!
//! What this file must ALSO keep true: `%s` on a packed integral is a DIFFERENT rule
//! that all three tools agree on (the NUL renders as a space, nothing is dropped). The
//! negative-control tests at the bottom pin it, so the §6.16 change cannot leak into
//! the packed-ASCII rendering surface.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_strcast_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    )
}

/// `decls` at MODULE scope, `src_body` inside one `initial`; assert a printed line.
fn line_mod(decls: &str, src_body: &str, want: &str) {
    let src = format!("module t;\n{decls}\n  initial begin\n{src_body}\n  end\nendmodule\n");
    let (out, err, code) = run(&src);
    assert_eq!(code, Some(0), "expected exit 0.\nsrc:\n{src}\n{err}{out}");
    assert!(
        out.lines().any(|l| l == want),
        "expected a line `{want}`.\nsrc:\n{src}\ngot:\n{out}{err}"
    );
}

fn loud(decls: &str, src_body: &str) {
    let src = format!("module t;\n{decls}\n  initial begin\n{src_body}\n  end\nendmodule\n");
    let (out, err, code) = run(&src);
    assert_ne!(
        code,
        Some(0),
        "must be loud, not silent.\nsrc:\n{src}\n{out}{err}"
    );
    assert!(
        format!("{err}{out}").contains("VITA-E"),
        "expected a loud E-diagnostic.\nsrc:\n{src}\n{out}{err}"
    );
}

// ─────────────────── §6.16 implicit integral → string assignment ───────────────────

/// The pre-existing 2-oracle silent-wrong this slice closes: the conversion used to
/// strip only the LEADING width padding, so an interior or trailing 0x00 survived as a
/// real character. All four cells are `iverilog` AND `verilator`.
#[test]
fn implicit_assign_drops_every_nul_byte() {
    // interior NUL: both oracles `[ab] 2`; vita printed `[a<NUL>b] 3`.
    line_mod(
        "  string s;",
        "    s = 24'h610062;\n    $display(\"[%s] %0d\", s, s.len());",
        "[ab] 2",
    );
    // trailing NUL: both oracles `[ab] 2`; vita printed `[ab<NUL>] 3`.
    line_mod(
        "  string s;",
        "    s = 24'h616200;\n    $display(\"[%s] %0d\", s, s.len());",
        "[ab] 2",
    );
    // leading NUL (the only one the old rule handled).
    line_mod(
        "  string s;",
        "    s = 24'h006162;\n    $display(\"[%s] %0d\", s, s.len());",
        "[ab] 2",
    );
    // all-NUL operand → the empty string, not three NUL characters.
    line_mod(
        "  string s;",
        "    s = 24'h000000;\n    $display(\"[%s] %0d\", s, s.len());",
        "[] 0",
    );
}

/// An UNKNOWN bit reads 0 — vita packs `z` as val=1, so the old `val`-plane read
/// answered 0xF0 for a `zzzzxxxx` byte and kept it. Both oracles: `[a] 1`.
///
/// The second cell is the discriminator the first one cannot give: a byte with x bits
/// AND known 1 bits is NOT dropped, it is `0x0f` — so the rule is "unknown bits read 0",
/// not "a byte containing an unknown is dropped". (verilator; iverilog has no `.getc`
/// to probe the byte, and agrees on the length cell above.)
#[test]
fn implicit_assign_reads_unknown_bits_as_zero() {
    line_mod(
        "  string s; logic [15:0] u;",
        "    u = 16'h61zz; u[3:0] = 4'bxxxx;\n    s = u;\n    $display(\"[%s] %0d\", s, s.len());",
        "[a] 1",
    );
    line_mod(
        "  string s; logic [15:0] u;",
        "    u = 16'h610f; u[7:4] = 4'bxxxx;\n    s = u;\n    $display(\"%0d %0h\", s.len(), s.getc(1));",
        "2 f",
    );
}

/// The conversion is not the assignment's alone: a string QUEUE element store takes it
/// too (both oracles `[ab] 2`).
#[test]
fn string_queue_element_store_converts() {
    line_mod(
        "  string q[$];",
        "    q.push_back(24'h610062);\n    $display(\"[%s] %0d\", q[0], q[0].len());",
        "[ab] 2",
    );
}

/// A `string` FORMAL bound to an integral actual, and a string-vs-integral compare.
///
/// verilator ONLY: iverilog 13 aborts the COMPILE on both shapes (an internal
/// `ivl_expr_value` assertion in `draw_eval_vec4` for the formal, a `%cmp/u operand
/// width mismatch` vvp abort for the compare), so it is not an oracle here. verilator
/// prints `2` and `1`; vita printed `3` and `0` while it kept the interior NUL.
#[test]
fn string_formal_and_compare_convert() {
    line_mod(
        "  function automatic int flen(input string p); flen = p.len(); endfunction",
        "    $display(\"%0d\", flen(24'h610062));",
        "2",
    );
    line_mod(
        "  string s;",
        "    s = \"ab\";\n    $display(\"%0d\", s == 24'h610062);",
        "1",
    );
}

// ─────────────────────────── `string'(e)` — the cast ───────────────────────────

/// The ROADMAP §2 row: `string'(expr)` used to be a parse error
/// (`E-PARSE-UNEXPECTED-TOKEN: expected expression, found keyword 'string'`) although
/// BOTH oracles run it. Pinned values are iverilog + verilator.
#[test]
fn string_cast_basic_values() {
    line_mod(
        "  string s;",
        "    s = string'(24'h610062);\n    $display(\"[%s] %0d %0d\", s, s.len(), s == \"ab\");",
        "[ab] 2 1",
    );
    line_mod(
        "  string s; int x = 24'h616263;",
        "    s = string'(x);\n    $display(\"[%s] %0d\", s, s.len());",
        "[abc] 3",
    );
    line_mod(
        "  string s;",
        "    s = string'(16'h4142);\n    $display(\"[%s] %0d\", s, s.len());",
        "[AB] 2",
    );
    // a packed reg operand, and a CONCAT operand carrying the interior NUL.
    line_mod(
        "  string s; reg [23:0] r = 24'h610062;",
        "    s = string'(r);\n    $display(\"[%s] %0d\", s, s.len());",
        "[ab] 2",
    );
    line_mod(
        "  string s;",
        "    s = string'({8'h61, 8'h00, 8'h62});\n    $display(\"[%s] %0d\", s, s.len());",
        "[ab] 2",
    );
}

/// The operand width edges. All eight cells are iverilog AND verilator.
///
/// `string'(0)` is the one that proves the operand is SELF-determined: an unsized `0`
/// is 32 bits of padding, and every byte of it drops.
#[test]
fn string_cast_operand_width_edges() {
    let cells: &[(&str, &str)] = &[
        ("24'h610062", "[ab] 2"), // NUL in the middle
        ("24'h006162", "[ab] 2"), // NUL at the top
        ("24'h616200", "[ab] 2"), // NUL at the bottom
        ("0", "[] 0"),            // zero operand
        ("8'h00", "[] 0"),        // one NUL byte
        ("7'h61", "[a] 1"),       // partial byte
        ("64'h6162636465666768", "[abcdefgh] 8"),
        ("72'h616263646566676869", "[abcdefghi] 9"), // wider than one word
    ];
    for (operand, want) in cells {
        line_mod(
            "  string s;",
            &format!("    s = string'({operand});\n    $display(\"[%s] %0d\", s, s.len());"),
            want,
        );
    }
}

/// A SIGNED negative operand converts its two's-complement bytes: `-8'sd1` is one
/// 0xFF byte. The LENGTH is iverilog + verilator (through the implicit twin); the BYTE
/// is verilator only (iverilog 13 has no `.getc` method at all).
#[test]
fn string_cast_signed_negative_operand() {
    line_mod(
        "  string s;",
        "    s = string'(-8'sd1);\n    $display(\"%0d %0h\", s.len(), s.getc(0));",
        "1 ff",
    );
}

/// A 4-state operand: unknown bits read 0 and the resulting NUL byte drops. Both
/// oracles `[a] 1`.
#[test]
fn string_cast_four_state_operand() {
    line_mod(
        "  string s; logic [15:0] u;",
        "    u = 16'h61zz; u[3:0] = 4'bxxxx;\n    s = string'(u);\n    $display(\"[%s] %0d\", s, s.len());",
        "[a] 1",
    );
}

/// §6.16 converts an INTEGRAL operand, so a STRING operand is the identity — including
/// the nested spelling. verilator `[hi] 2`; iverilog is not an oracle for the probe
/// (its `.getc`-less string surface stops the design elaborating), but it agrees on the
/// nested cell's stored form.
#[test]
fn string_cast_of_a_string_is_the_identity() {
    line_mod(
        "  string s0 = \"hi\"; string s;",
        "    s = string'(s0);\n    $display(\"[%s] %0d\", s, s.len());",
        "[hi] 2",
    );
    line_mod(
        "  string s;",
        "    s = string'(string'(24'h610062));\n    $display(\"[%s] %0d\", s, s.len());",
        "[ab] 2",
    );
    // a string LITERAL operand (a packed Const at the IR level) is the identity too.
    line_mod(
        "  string s;",
        "    s = string'(\"ab\");\n    $display(\"[%s] %0d\", s, s.len());",
        "[ab] 2",
    );
}

/// The cast is a string-domain VALUE, not just an assignment source: it must work as a
/// concat part, a function actual, a compare operand and a `case` scrutinee.
///
/// `{string'(x), "!"}` is the cell that catches a cast the AST string classifier does
/// not claim: before `CastTarget::is_string_cast` reached `expr_is_string_ast`, the
/// concat took the PACKED path and printed `c!` (the cast node's 8-bit static
/// placeholder width) where both oracles print `abc!`.
///
/// concat + function actual = iverilog AND verilator. compare and `case` = verilator
/// only (iverilog's vvp aborts on a string-vs-packed compare, see above).
#[test]
fn string_cast_in_every_value_position() {
    let decls = "  int x = 24'h616263;\n  \
                 function automatic int flen(input string p); flen = p.len(); endfunction";
    line_mod(
        decls,
        "    $display(\"[%s]\", {string'(x), \"!\"});",
        "[abc!]",
    );
    line_mod(decls, "    $display(\"%0d\", flen(string'(x)));", "3");
    line_mod(decls, "    $display(\"%0d\", string'(x) == \"abc\");", "1");
    line_mod(
        decls,
        "    case (string'(x)) \"abc\": $display(\"hit\"); default: $display(\"miss\"); endcase",
        "hit",
    );
}

/// SPLIT — recorded, not pinned to one side. `%s` of a cast that is never stored:
/// verilator converts (`[abc]`, `[ab]`) and a bare `$display(string'(x))` prints
/// `abc`; iverilog keeps the PACKED operand and prints `[ abc]`, `[a b]` and the
/// decimal `    6382179`. vita follows verilator and §6.16 — in vita the cast is a
/// string-domain value, so the same bytes reach `%s` that reach a `string` variable.
#[test]
fn string_cast_rendered_directly_follows_verilator() {
    let decls = "  int x = 24'h616263;";
    line_mod(decls, "    $display(\"[%s]\", string'(x));", "[abc]");
    line_mod(
        decls,
        "    $display(\"[%s]\", string'(24'h610062));",
        "[ab]",
    );
    // a bare string-domain argument is itself a format segment (§17.1).
    line_mod(decls, "    $display(string'(x));", "abc");
}

// ──────────────────────────────── loud shapes ────────────────────────────────

/// A `real` operand. iverilog refuses it ("sorry: This cast operation is not yet
/// supported"); verilator renders the raw f64 bytes (`3.5` → `@\x0c`). No oracle for a
/// value ⇒ correct-or-loud.
#[test]
fn string_cast_of_a_real_is_loud() {
    loud("  string s; real r = 3.5;", "    s = string'(r);");
    loud("  string s;", "    s = string'(3.5);");
}

/// A METHOD on the cast expression. BOTH oracles refuse the SYNTAX — verilator "Not
/// expecting CVTPACKSTRING under a DOT in dotted expression", iverilog "syntax error" —
/// so there is no oracle for the value and the shape stays a parse error. Assign the
/// cast to a `string` first, which every tool runs.
#[test]
fn a_method_on_the_cast_expression_is_loud() {
    loud(
        "  int x = 24'h616263;",
        "    $display(\"%0d\", string'(x).len());",
    );
}

/// The reserved `string` spelling must not leak into the OTHER `Named` cast arms: a
/// typedef/class-name cast keeps its own diagnostic, and a bare `string` keyword with
/// no `'(` after it is still not an expression.
#[test]
fn the_reserved_spelling_does_not_widen_the_named_cast_arm() {
    // an unresolvable NAME cast still reports "typedef/class cast `name'(expr)` is
    // outside the v1 cast scope", not the string-cast diagnostic.
    loud("  logic [7:0] v;", "    v = foo_t'(8'h12);");
    // the parser arm is guarded on `'(`, so a bare `string` keyword is still not an
    // expression (E-PARSE-UNEXPECTED-TOKEN, as before this slice).
    loud("  int x;", "    x = string;");
}

// ───────────────────── negative controls: `%s` is a DIFFERENT rule ─────────────────────

/// `%s` on a packed integral is NOT §6.16 and must not move: the NUL renders as a
/// SPACE and nothing is dropped. iverilog 13, verilator 5.052 and vita all print
/// `[a b]` / `[ab ]`, and `$sformatf("%s", 24'h610062)` is a THREE-character string
/// (the middle character is the rendered space, 0x20 — not a NUL).
///
/// This is the control on the whole slice: the §6.16 rule lives on the integral→string
/// CROSSING (`Value::to_sv_string_bytes`), never on the packed-ASCII rendering surface
/// (`Value::to_str_bytes`).
#[test]
fn percent_s_of_a_packed_integral_is_unchanged() {
    line_mod("", "    $display(\"[%s]\", 24'h610062);", "[a b]");
    line_mod("", "    $display(\"[%s]\", 24'h616200);", "[ab ]");
    line_mod(
        "  string q;",
        "    q = $sformatf(\"%s\", 24'h610062);\n    $display(\"%0d %0h\", q.len(), q.getc(1));",
        "3 20",
    );
}

/// A string→string copy is not a conversion: bytes already in a string stay put. (Both
/// oracles print `ab`; the cell exists so the `is_str` short-circuit in
/// `to_sv_string_bytes` is exercised on the assignment path, not only the cast path.)
#[test]
fn a_string_to_string_copy_is_not_converted() {
    line_mod(
        "  string a; string b;",
        "    a = 24'h610062;\n    b = a;\n    $display(\"[%s] %0d\", b, b.len());",
        "[ab] 2",
    );
}
