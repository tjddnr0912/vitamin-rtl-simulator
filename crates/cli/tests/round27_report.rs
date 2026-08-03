//! External report round-27 (2026-08-03) — `@(*)` was lexed as an attribute instance.
//!
//! The highest-severity report this repository has received: a **silent wrong result**,
//! which is the one thing `correct-or-loud` promises cannot happen.
//!
//! The lexer skipped IEEE 1800-2017 §5.12 attribute instances with a regex over RAW
//! TEXT, `\(\*([^*]|\*[^)])*\*\)`. The old comment above it argued that the regex could
//! not match `(*)` — "after `(*` the body is `[^*]` or `*[^)]` and the terminator is
//! `*)`, so a lone `)` can never reach one". That is true of those three characters in
//! isolation and it is not what the regex does: the body happily consumes the `)` and
//! everything after it, terminating at the next `*)` ANYWHERE LATER in the compilation
//! unit. So:
//!
//! - a `*)` inside a trailing `//` comment closed the sensitivity list's `(*`, and the
//!   REST OF THE COMMENT became live code — executed, with `errors=0`;
//! - a second `@(*)` supplied exactly such a `*)`, so two of them in one unit deleted
//!   the code between (and the diagnostic landed on the second, not the cause);
//! - the pairing crossed file boundaries, because a compilation unit is one stream;
//! - and when no `*)` existed the regex silently failed and the `(*` fell through, so
//!   whether any of this happened depended on the `(*`/`*)` census of the whole unit.
//!
//! The fix recognises attributes over the TOKEN STREAM instead. Comments are already
//! gone and string literals are already single tokens by then, so text that is not code
//! cannot contribute a delimiter; a `(*` whose previous token is `@` is an event control
//! (IEEE 1364-2005 A.6.5) and never an attribute; and an opener with no closer is loud.
//!
//! Every case below was measured three ways — PRE (`bb8003b`), POST, and iverilog 13.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Run vita over one or more (filename, source) pairs in a fresh directory.
fn run_files(files: &[(&str, &str)]) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_r27_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let mut c = Command::new(env!("CARGO_BIN_EXE_vita"));
    for (name, src) in files {
        std::fs::write(d.join(name), src).unwrap();
        c.arg(name);
    }
    let out = c.current_dir(&d).output().expect("run vita");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = std::fs::remove_dir_all(&d);
    (text, out.status.code())
}

fn run(src: &str) -> (String, Option<i32>) {
    run_files(&[("t.v", src)])
}

/// ★ R1 — the silent wrong result, verbatim from the report.
///
/// `always @(*) a = b;  // *)(b) begin a = 1'b1; $display("…"); end`
///
/// PRE printed `!! COMMENTED-OUT CODE EXECUTED !!` and `a=1` with `errors=0`: the `(*`
/// of the sensitivity list closed on the `*)` inside the comment, and what was left —
/// `(b) begin a = 1'b1; … end` — parsed as the event control and body of the `always`.
/// The user's `a = b;` was gone. This is the pin that matters most: no diagnostic could
/// have saved it, because there was none.
#[test]
fn a_comment_can_never_become_live_code() {
    let (o, ok) = run(
        "module t;\n\
           reg a, b;\n\
           always @(*) a = b;  // *)(b) begin a = 1'b1; $display(\"!! COMMENTED-OUT CODE EXECUTED !!\"); end\n\
           initial begin\n\
             b = 1'b0;\n\
             #1 $display(\"a=%b\", a);\n\
             $finish;\n\
           end\n\
         endmodule\n",
    );
    assert!(
        !o.contains("COMMENTED-OUT CODE EXECUTED"),
        "commented-out text was promoted to live code:\n{o}"
    );
    assert!(
        o.contains("a=0"),
        "`a` must follow `b`, which is 0 (iverilog: a=0):\n{o}"
    );
    assert_eq!(ok, Some(0), "and it must be clean:\n{o}");
}

/// R2 — two `@(*)` blocks in one file. PRE: `expected statement, found Eq` on the
/// SECOND block, while the cause was the first.
#[test]
fn two_implicit_sensitivity_blocks_do_not_pair_with_each_other() {
    let (o, ok) = run(
        "module t;\n\
           reg a, b, c, d;\n\
           always @(*) a = b;\n\
           always @(*) c = d;\n\
           initial begin b = 1'b0; d = 1'b1; #1\n\
             if (a === 1'b0 && c === 1'b1) $display(\"PASS\"); else $display(\"FAIL a=%b c=%b\", a, c);\n\
             $finish; end\n\
         endmodule\n",
    );
    assert!(
        o.contains("PASS"),
        "both blocks must be live comb logic:\n{o}"
    );
    assert_eq!(ok, Some(0), "must be clean:\n{o}");
}

/// R2's `case`-bodied variant, lifted from a commercial RTL tree (`PERI.v`). It failed
/// with a DIFFERENT message — `expected '(' or '*' after '@', found Keyword(Case)` —
/// because the swallow ended in a different place. Same root, so same pin.
#[test]
fn the_case_bodied_variant_from_real_rtl() {
    let (o, ok) = run("module v3;\n\
           reg [3:0] pclk_cnt; reg [2:0] STM_CLK_DIV;\n\
           reg SCLK_OUT, STM_CLK_OUT;\n\
           always @ (*)\n\
               SCLK_OUT = pclk_cnt[0];\n\
           always @ (*)\n\
               case (STM_CLK_DIV[2:1])\n\
                   0: STM_CLK_OUT = pclk_cnt[0];\n\
               endcase\n\
           initial begin pclk_cnt = 4'b1; STM_CLK_DIV = 3'b0; #1\n\
             $display(\"S=%b T=%b\", SCLK_OUT, STM_CLK_OUT); $finish; end\n\
         endmodule\n");
    assert!(o.contains("S=1 T=1"), "iverilog prints S=1 T=1:\n{o}");
    assert_eq!(ok, Some(0), "must be clean:\n{o}");
}

/// R3 — the pairing crossed FILE boundaries, because a compilation unit is one token
/// stream. Each file alone passed; together they failed, and the diagnostic named the
/// second file while the cause was in the first. That is what made the defect
/// untraceable on a real design.
#[test]
fn the_pairing_does_not_cross_file_boundaries() {
    let (o, ok) = run_files(&[
        (
            "i1.v",
            "module i1; reg a, b;\n  always @(*) a = b;\n  initial begin b = 1'b0; #1 if (a === 1'b0) $display(\"P1\"); $finish; end\nendmodule\n",
        ),
        (
            "i2.v",
            "module i2; reg c, d;\n  always @(*) c = d;\nendmodule\n",
        ),
    ]);
    assert!(o.contains("P1"), "must elaborate and run:\n{o}");
    assert!(
        !o.contains("error["),
        "each file is valid alone, so together they are too:\n{o}"
    );
    assert_eq!(ok, Some(0), "must be clean:\n{o}");
}

/// R4 — a string literal is not code. PRE reported `unterminated string literal`
/// because the `(*)` inside the message was consumed as a delimiter pair.
#[test]
fn a_string_literal_can_never_supply_a_delimiter() {
    let (o, ok) = run("module t; reg a, b;\n\
           always @(*) a = b;\n\
           initial begin b = 1'b0; #1 $display(\"sensitivity (*) is legacy\");\n\
             if (a === 1'b0) $display(\"PASS\"); else $display(\"FAIL\"); $finish; end\n\
         endmodule\n");
    assert!(
        o.contains("sensitivity (*) is legacy"),
        "string intact:\n{o}"
    );
    assert!(o.contains("PASS"), "and `a` still follows `b`:\n{o}");
    assert_eq!(ok, Some(0), "must be clean:\n{o}");
}

/// The same for a BLOCK comment — the raw-text scan went through `/* … */` too.
#[test]
fn a_block_comment_can_never_supply_a_delimiter() {
    let (o, ok) = run(
        "module t; reg a, b;\n\
           always @(*) a = b;  /* legacy *) form */\n\
           initial begin b = 1'b0; #1 if (a === 1'b0) $display(\"PASS\"); else $display(\"FAIL\"); $finish; end\n\
         endmodule\n",
    );
    assert!(
        o.contains("PASS"),
        "block comment leaked into the code:\n{o}"
    );
    assert_eq!(ok, Some(0), "must be clean:\n{o}");
}

/// Every spelling the report tabulated, plus `@*`. All seven are the same construct and
/// must behave identically; PRE split them into three groups depending on which
/// delimiters happened to be adjacent.
#[test]
fn every_spelling_of_the_implicit_sensitivity_list_agrees() {
    for form in ["@(*)", "@ (*)", "@(* )", "@( *)", "@ ( * )", "@*", "@ *"] {
        let src = format!(
            "module t; reg a, b, c, d;\n\
               always {form} a = b;\n\
               always {form} c = d;\n\
               initial begin b = 1'b0; d = 1'b1; #1\n\
                 if (a === 1'b0 && c === 1'b1) $display(\"PASS\"); else $display(\"FAIL a=%b c=%b\", a, c);\n\
                 $finish; end\n\
             endmodule\n"
        );
        let (o, ok) = run(&src);
        assert!(o.contains("PASS"), "form `{form}` is broken:\n{o}");
        assert_eq!(ok, Some(0), "form `{form}` is not clean:\n{o}");
    }
}

/// Attribute instances must KEEP working — they are the reason the skip existed, and
/// PicoRV32 uses `(* parallel_case *)` / `(* full_case *)`. Includes an attribute both
/// before and after an `@(*)`, and one attached to a statement inside a body.
#[test]
fn attribute_instances_are_still_skipped() {
    let (o, ok) = run("module t; reg [1:0] s; reg [7:0] y; reg a, b;\n\
           (* keep = 1 *) wire w = 1'b1;\n\
           always @(*) a = b;\n\
           (* parallel_case *)\n\
           always @(*) begin\n\
             (* full_case *)\n\
             case (s) 2'd0: y = 8'd10; 2'd1: y = 8'd20; default: y = 8'd30; endcase\n\
           end\n\
           initial begin s = 2'd1; b = 1'b0; #1\n\
             if (y === 8'd20 && a === 1'b0 && w === 1'b1) $display(\"PASS\");\n\
             else $display(\"FAIL y=%0d a=%b w=%b\", y, a, w);\n\
             $finish; end\n\
         endmodule\n");
    assert!(o.contains("PASS"), "attribute support regressed:\n{o}");
    assert_eq!(ok, Some(0), "must be clean:\n{o}");
}

/// An attribute whose own STRING contains the closing delimiter. The reporter did not
/// find this one; the raw-text regex terminated inside the string, so PRE failed here
/// too. Token-level pairing gets it right for free — the string is one token.
#[test]
fn an_attribute_string_may_contain_the_delimiter() {
    let (o, ok) = run(
        "module t; reg a, b;\n\
           (* keep = \"*)\" *) wire w = 1'b1;\n\
           always @(*) a = b;\n\
           initial begin b = 1'b0; #1\n\
             if (a === 1'b0 && w === 1'b1) $display(\"PASS\"); else $display(\"FAIL a=%b w=%b\", a, w);\n\
             $finish; end\n\
         endmodule\n",
    );
    assert!(o.contains("PASS"), "delimiter inside a string leaked:\n{o}");
    assert_eq!(ok, Some(0), "must be clean:\n{o}");
}

/// An opener with no closer is LOUD. PRE fell through silently, and that silence is
/// what made the whole defect non-local: whether a `(*` acted as an attribute depended
/// on the `(*`/`*)` census of the entire compilation unit, so one `@(*)` in a file
/// passed and two broke.
#[test]
fn an_unterminated_attribute_is_loud() {
    let (o, ok) = run("module u; (* keep = 1 wire x = 1'b0;\n\
           initial begin $display(\"SHOULD-NOT-RUN\"); $finish; end\n\
         endmodule\n");
    assert_ne!(ok, Some(0), "must not exit clean:\n{o}");
    assert!(
        o.contains("attribute instance"),
        "the message must name the construct:\n{o}"
    );
    assert!(
        !o.contains("SHOULD-NOT-RUN"),
        "must not run a design it could not lex:\n{o}"
    );
}

/// …and it stays loud when an `@(*)` follows. An `@(*)` contains an adjacent `*` `)`,
/// so without excluding event controls from the CLOSER scan too, an unterminated
/// attribute would silently swallow forward to the next sensitivity list — the same
/// non-local behaviour, just one step removed.
#[test]
fn an_unterminated_attribute_does_not_close_on_a_sensitivity_list() {
    let (o, ok) = run("module u; reg a, b;\n\
           (* keep = 1\n\
           always @(*) a = b;\n\
           initial begin b = 1'b1; #1 $display(\"SHOULD-NOT-RUN a=%b\", a); $finish; end\n\
         endmodule\n");
    assert_ne!(ok, Some(0), "must not exit clean:\n{o}");
    assert!(
        o.contains("attribute instance"),
        "must report the unterminated opener, not swallow to the `@(*)`:\n{o}"
    );
    assert!(!o.contains("SHOULD-NOT-RUN"), "must not run:\n{o}");
}

/// `@(*)` in the other places a design puts it: inside a subroutine, inside a generate
/// body, and beside an `always_comb`. PRE failed all three.
#[test]
fn the_implicit_sensitivity_list_works_everywhere_it_is_written() {
    let cases: [(&str, &str); 3] = [
        (
            "inside a task",
            "module t; reg a, b, c, d;\n\
               task automatic tk; begin @(*) ; end endtask\n\
               always @(*) a = b;\n\
               always @(*) c = d;\n\
               initial begin b = 1'b0; d = 1'b1; #1 if (a===1'b0 && c===1'b1) $display(\"PASS\"); else $display(\"FAIL\"); $finish; end\n\
             endmodule\n",
        ),
        (
            "inside a generate body",
            "module t; reg a, b, c, d;\n\
               genvar g;\n\
               generate for (g = 0; g < 1; g = g + 1) begin : gb\n\
                 always @(*) a = b;\n\
                 always @(*) c = d;\n\
               end endgenerate\n\
               initial begin b = 1'b0; d = 1'b1; #1 if (a===1'b0 && c===1'b1) $display(\"PASS\"); else $display(\"FAIL\"); $finish; end\n\
             endmodule\n",
        ),
        (
            "beside an always_comb",
            "module t; reg a, b, c, d;\n\
               always_comb a = b;\n\
               always @(*) c = d;\n\
               always @(*) ;\n\
               initial begin b = 1'b0; d = 1'b1; #1 if (a===1'b0 && c===1'b1) $display(\"PASS\"); else $display(\"FAIL\"); $finish; end\n\
             endmodule\n",
        ),
    ];
    for (what, src) in cases {
        let (o, ok) = run(src);
        assert!(o.contains("PASS"), "{what}:\n{o}");
        assert_eq!(ok, Some(0), "{what} is not clean:\n{o}");
    }
}
