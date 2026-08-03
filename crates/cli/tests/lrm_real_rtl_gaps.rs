//! Three IEEE-legal constructs vita rejected, found by elaborating a real open-source
//! core (PicoRV32) instead of another synthetic benchmark.
//!
//! Every measurement in the compiled-backend investigation had been taken on generated
//! chains or on the 2-5 process designs in `examples/`. The first time a real third-party
//! design was fed in, it failed to parse on line 84 — and behind that were two more gaps,
//! each masked by the one in front of it. All three are plain Verilog-2005/2001, none
//! needed new machinery, and none would ever have surfaced from a generator that only
//! emits what vita already accepts.
//!
//! Oracle: iverilog 13 accepts all three; the expected values below are hand-IEEE and
//! agree with it.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("vita_lrm_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("t.sv");
    std::fs::write(&p, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&p)
        .arg("--timeout")
        .arg("100000")
        .current_dir(&dir)
        .output()
        .expect("run vita");
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    let _ = std::fs::remove_dir_all(&dir);
    (s, out.status.success())
}

#[track_caller]
fn pins(label: &str, src: &str, want: &str) {
    let (o, ok) = run(src);
    assert!(
        ok && !o.contains("error[VITA") && !o.contains("fatal[VITA") && o.contains(want),
        "{label}: expected {want:?}\n{o}"
    );
}

/// IEEE 1800-2017 §5.7.1: white space is permitted between the SIZE and the base
/// specifier, and between the base SPECIFIER and the value. PicoRV32's parameter block
/// is written `32'h 0000_0000` throughout, and vita's lexer required the digits to
/// abut the base letter — the apostrophe fell out as a bare token and the parser
/// emitted a cascade of errors pointing at the wrong construct.
#[test]
fn a_based_literal_may_have_space_around_the_base_specifier() {
    pins(
        "space after base",
        "module t; initial begin $display(\"%0d\", 32'h 0000_00ff); $finish; end endmodule",
        "255",
    );
    pins(
        "space before tick",
        "module t; initial begin $display(\"%0d\", 8 'hFF); $finish; end endmodule",
        "255",
    );
    pins(
        "space on both sides, signed marker",
        "module t; initial begin $display(\"%0d\", 8 'sh 7f); $finish; end endmodule",
        "127",
    );
    // The no-space spelling must be untouched.
    pins(
        "no space still works",
        "module t; initial begin $display(\"%0d\", 32'hff); $finish; end endmodule",
        "255",
    );
}

/// IEEE 1800-2017 §5.12 attribute instances `(* ... *)` are tool hints with no
/// simulation semantics, so they are skipped like a comment. PicoRV32 uses
/// `(* parallel_case *)` and `(* full_case *)` on its decoder `case` statements.
///
/// The trap: `always @(*)` must keep lexing as `(`/`*`/`)`.
///
/// ⚠️ This test was written to hold that line and DID NOT — see round-27. The reasoning
/// quoted here was wrong ("the skip regex cannot match `(*)` — after `(*` its body is
/// `[^*]` or `*[^)]` and its terminator is `*)`, so a lone `)` can never reach one"):
/// true of those three characters alone, and irrelevant, because the body consumed the
/// `)` and ran on to the next `*)` anywhere later in the unit. Every design below has
/// exactly ONE `@(*)` and no later `*)`, so the regex silently failed to match and the
/// fallback rescued it. Two `@(*)` blocks — the shape real RTL actually has — destroyed
/// the code between them, and a `*)` in a trailing comment became live code with
/// `errors=0`. The real pins are in `round27_report.rs`; this one keeps its original
/// coverage (attributes are skipped, and PicoRV32's `(* parallel_case *)` works).
#[test]
fn attribute_instances_are_skipped_without_eating_implicit_sensitivity() {
    pins(
        "attribute on a case",
        "module t;\n\
           reg [1:0] s; reg [7:0] y;\n\
           always @* begin\n\
             (* parallel_case *)\n\
             case (s)\n\
               2'd0: y = 8'd10;\n\
               2'd1: y = 8'd20;\n\
               default: y = 8'd30;\n\
             endcase\n\
           end\n\
           initial begin s = 2'd1; #1 $display(\"y=%0d\", y); $finish; end\n\
         endmodule",
        "y=20",
    );
    pins(
        "attribute with a value, and @(*) in the same design",
        "module t;\n\
           reg [7:0] a, b;\n\
           (* keep = \"true\" *) reg [7:0] c;\n\
           always @(*) c = a + b;\n\
           initial begin a = 8'd3; b = 8'd4; #1 $display(\"c=%0d\", c); $finish; end\n\
         endmodule",
        "c=7",
    );
    // The implicit sensitivity list is NOT an attribute. NOTE: one `@(*)` and no later
    // `*)` is precisely the shape that passed while the defect was live — this assertion
    // has no teeth on its own. `round27_report.rs` carries the ones that do.
    pins(
        "@(*) alone",
        "module t;\n\
           reg [7:0] a, y;\n\
           always @(*) y = a + 8'd1;\n\
           initial begin a = 8'd41; #1 $display(\"y=%0d\", y); $finish; end\n\
         endmodule",
        "y=42",
    );
}

/// IEEE 1800-2017 §11.4.12: a concatenation of constants is a constant expression. Its
/// value depends on every operand's WIDTH, not just its value, which is why the plain
/// i64 constant folder could not do it and reported "not a foldable constant
/// expression". PicoRV32 builds its trace masks as
/// `localparam [35:0] TRACE_BRANCH = {4'b 0001, 32'b 0};` — which also needs the
/// whitespace fix above, so this gap was invisible until that one was closed.
#[test]
fn a_constant_concatenation_folds_in_a_localparam() {
    pins(
        "widths decide the value",
        "module t;\n\
           localparam [11:0] P = {4'b0001, 8'b0};\n\
           initial begin $display(\"P=%0d\", P); $finish; end\n\
         endmodule",
        "P=256", // 1 << 8, NOT 1 — the 8-bit zero contributes width, not just value
    );
    pins(
        "picorv32's own spelling",
        "module t;\n\
           localparam [35:0] TRACE_BRANCH = {4'b 0001, 32'b 0};\n\
           initial begin $display(\"T=%0d\", TRACE_BRANCH[35:32]); $finish; end\n\
         endmodule",
        "T=1",
    );
    pins(
        "three parts",
        "module t;\n\
           localparam [7:0] P = {2'b11, 3'b010, 3'b001};\n\
           initial begin $display(\"P=%0d\", P); $finish; end\n\
         endmodule",
        "P=209", // 11_010_001
    );
}

/// A concatenation whose operand width is NOT statically determinable must stay LOUD,
/// not guess. Guessing a width silently changes the value — the folder returns None and
/// the caller reports its usual diagnostic.
#[test]
fn an_unfoldable_concatenation_stays_loud() {
    let (o, ok) = run("module t;\n\
           reg [7:0] r;\n\
           localparam [11:0] P = {4'b0001, r};\n\
           initial begin $display(\"P=%0d\", P); $finish; end\n\
         endmodule");
    assert!(
        !ok && o.contains("error[VITA"),
        "a concat over a non-constant operand must be a diagnostic, not a guessed \
         width — got:\n{o}"
    );
}

/// The whitespace allowance has EXACT edges, and they were taken from the oracle rather
/// than from a reading of the LRM. iverilog 13 accepts `32'h 0000_00ff`, `8 'hFF` and
/// `8 'sh 7f`, and REJECTS `8' h 7f` and `8 's h 7f` — space is legal before the
/// apostrophe and after the base character, but not INSIDE the base specifier.
///
/// Pinned as a negative test because a regex written slightly too wide would accept
/// these silently, and the failure mode of a too-wide literal lexer is an apostrophe
/// swallowing text that belongs to the next construct.
#[test]
fn space_inside_the_base_specifier_is_still_rejected() {
    for bad in ["8' h 7f", "8 's h 7f"] {
        let src =
            format!("module t; initial begin $display(\"%0d\", {bad}); $finish; end endmodule");
        let (o, ok) = run(&src);
        assert!(
            !ok && o.contains("error[VITA"),
            "`{bad}` is a syntax error for iverilog and must be one here too — got:\n{o}"
        );
    }
}
