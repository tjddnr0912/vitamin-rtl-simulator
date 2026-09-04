//! §4.5.417 census: default-argument position × use shape (blank / omitted / given /
//! whitespace), default kinds (literal / macro / expression / parenthesised / string /
//! empty / replication / a macro defined later), continuation and comment shapes in a
//! body, conditional directives inside a body (taken / untaken / defined later /
//! `\`undef` between / nested `\`ifndef`+`\`elsif`), `\`__FILE__` / `\`__LINE__` in every
//! position, and the arity errors. Oracle lines from iverilog 13.0 and verilator 5.050
//! (verilator refuses a replication in a default and a body comment continuation —
//! those cells carry iverilog's line, see the comments).

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_macro_de_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    std::fs::write(
        d.join("h.svh"),
        "`define INCL `__LINE__\n`define INCF `__FILE__\n`define INCADD(a, b = 3) ((a)+(b))\n",
    )
    .unwrap();
    std::fs::write(
        d.join("hdr.svh"),
        "`define INCL `__LINE__\n\n\n`define INCF `__FILE__\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("-I")
        .arg(d.to_str().unwrap())
        .arg("t.sv")
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (s, out.status.code())
}

/// Every `DIGEST=` line, sorted, joined by `|` (the census harness format; sorted
/// because two instances print in scheduling order).
fn digest(name: &str, src: &str, expect: &str) {
    let (out, rc) = run(src);
    assert_eq!(rc, Some(0), "{name}: expected exit 0, got {rc:?}:\n{out}");
    let mut v: Vec<&str> = out
        .lines()
        .filter_map(|l| l.strip_prefix("DIGEST="))
        .collect();
    v.sort_unstable();
    assert_eq!(v.join("|"), expect, "{name}:\n{out}");
}

fn loud(name: &str, src: &str, needle: &str) {
    let (out, rc) = run(src);
    assert_ne!(rc, Some(0), "{name}: expected a loud reject:\n{out}");
    assert!(
        out.contains(needle),
        "{name}: expected `{needle}` in:\n{out}"
    );
}

#[test]
fn cell_cont() {
    // cont_block_comment: both oracles
    digest(
        "cont_block_comment",
        r#"`define M(a) ((a) /* c \
  more */ + 1)
module tb;
initial begin $display("DIGEST=%0d", `M(2)); #1 $finish; end
endmodule
"#,
        "3",
    );
    // cont_comment_cont: both oracles
    digest(
        "cont_comment_cont",
        r#"`define M(a) \
  ((a) + \
  // c \
  1)
module tb;
initial begin $display("DIGEST=%0d", `M(2)); #1 $finish; end
endmodule
"#,
        "3",
    );
    // cont_comment_end: both oracles
    digest(
        "cont_comment_end",
        r#"`define M(a) ((a) + 1) // c
module tb;
initial begin $display("DIGEST=%0d", `M(2)); #1 $finish; end
endmodule
"#,
        "3",
    );
    // cont_ifdef_late_define: both oracles
    digest(
        "cont_ifdef_late_define",
        r#"`define M(a) \
`ifdef F \
  (a)+10 \
`else \
  (a)+20 \
`endif
`define F
module tb;
initial begin $display("DIGEST=%0d", `M(1)); #1 $finish; end
endmodule
"#,
        "11",
    );
    // cont_ifdef_taken: both oracles
    digest(
        "cont_ifdef_taken",
        r#"`define F
`define M(a) \
`ifdef F \
  (a)+10 \
`else \
  (a)+20 \
`endif
module tb;
initial begin $display("DIGEST=%0d", `M(1)); #1 $finish; end
endmodule
"#,
        "11",
    );
    // cont_ifdef_untaken: both oracles
    digest(
        "cont_ifdef_untaken",
        r#"`define M(a) \
`ifdef F \
  (a)+10 \
`else \
  (a)+20 \
`endif
module tb;
initial begin $display("DIGEST=%0d", `M(1)); #1 $finish; end
endmodule
"#,
        "21",
    );
    // cont_ifndef_elsif: both oracles
    digest(
        "cont_ifndef_elsif",
        r#"`define G
`define M(a) \
`ifndef F \
`ifdef G \
  (a)+30 \
`else \
  (a)+40 \
`endif \
`else \
  (a)+50 \
`endif
module tb;
initial begin $display("DIGEST=%0d", `M(1)); #1 $finish; end
endmodule
"#,
        "31",
    );
    // cont_string_backslash: both oracles
    digest(
        "cont_string_backslash",
        r#"`define M(a) $sformatf("%0d\\n", a)
module tb;
initial begin $display("DIGEST=%s", `M(2)); #1 $finish; end
endmodule
"#,
        "2\\n",
    );
    // cont_two_uses_lines: both oracles
    digest(
        "cont_two_uses_lines",
        r#"`define M(a) \
  ((a) + \
  1)
module tb;
initial begin $display("DIGEST=%0d", `M(1) + `M(2)); #1 $finish; end
endmodule
"#,
        "5",
    );
    // cont_undef_between: both oracles
    digest(
        "cont_undef_between",
        r#"`define F
`define M(a) \
`ifdef F \
  (a)+10 \
`else \
  (a)+20 \
`endif
`undef F
module tb;
initial begin $display("DIGEST=%0d", `M(1)); #1 $finish; end
endmodule
"#,
        "21",
    );
}

#[test]
fn cell_def_both() {
    // def_both_blank_a: both oracles
    digest(
        "def_both_blank_a",
        r#"`define M(a = 1, b = 2) ((a)*10+(b))
module tb;
initial begin $display("DIGEST=%0d", `M(,7)); #1 $finish; end
endmodule
"#,
        "17",
    );
    // def_both_blank_b: both oracles
    digest(
        "def_both_blank_b",
        r#"`define M(a = 1, b = 2) ((a)*10+(b))
module tb;
initial begin $display("DIGEST=%0d", `M(7,)); #1 $finish; end
endmodule
"#,
        "72",
    );
    // def_both_given: both oracles
    digest(
        "def_both_given",
        r#"`define M(a = 1, b = 2) ((a)*10+(b))
module tb;
initial begin $display("DIGEST=%0d", `M(7,8)); #1 $finish; end
endmodule
"#,
        "78",
    );
    // def_both_omit_all: both oracles
    digest(
        "def_both_omit_all",
        r#"`define M(a = 1, b = 2) ((a)*10+(b))
module tb;
initial begin $display("DIGEST=%0d", `M()); #1 $finish; end
endmodule
"#,
        "12",
    );
    // def_both_omit_b: both oracles
    digest(
        "def_both_omit_b",
        r#"`define M(a = 1, b = 2) ((a)*10+(b))
module tb;
initial begin $display("DIGEST=%0d", `M(7)); #1 $finish; end
endmodule
"#,
        "72",
    );
    // def_both_ws: both oracles
    digest(
        "def_both_ws",
        r#"`define M(a = 1, b = 2) ((a)*10+(b))
module tb;
initial begin $display("DIGEST=%0d", `M( 7 , )); #1 $finish; end
endmodule
"#,
        "72",
    );
}

#[test]
fn cell_def_first() {
    // def_first_blank_a: both oracles
    digest(
        "def_first_blank_a",
        r#"`define M(a = 5, b) ((a)*10+(b))
module tb;
initial begin $display("DIGEST=%0d", `M(,7)); #1 $finish; end
endmodule
"#,
        "57",
    );
    // def_first_given: both oracles
    digest(
        "def_first_given",
        r#"`define M(a = 5, b) ((a)*10+(b))
module tb;
initial begin $display("DIGEST=%0d", `M(7,8)); #1 $finish; end
endmodule
"#,
        "78",
    );
}

#[test]
fn cell_def_last() {
    // def_last_blank_b: both oracles
    digest(
        "def_last_blank_b",
        r#"`define M(a, b = 5) ((a)*10+(b))
module tb;
initial begin $display("DIGEST=%0d", `M(7,)); #1 $finish; end
endmodule
"#,
        "75",
    );
    // def_last_given: both oracles
    digest(
        "def_last_given",
        r#"`define M(a, b = 5) ((a)*10+(b))
module tb;
initial begin $display("DIGEST=%0d", `M(7,8)); #1 $finish; end
endmodule
"#,
        "78",
    );
    // def_last_omit_b: both oracles
    digest(
        "def_last_omit_b",
        r#"`define M(a, b = 5) ((a)*10+(b))
module tb;
initial begin $display("DIGEST=%0d", `M(7)); #1 $finish; end
endmodule
"#,
        "75",
    );
    // def_last_ws: both oracles
    digest(
        "def_last_ws",
        r#"`define M(a, b = 5) ((a)*10+(b))
module tb;
initial begin $display("DIGEST=%0d", `M( 7 , )); #1 $finish; end
endmodule
"#,
        "75",
    );
}

#[test]
fn cell_err() {
    // err_bad_default_syntax: verilator — a parameter without a name (both oracles reject)
    loud(
        "err_bad_default_syntax",
        r#"`define M(a, = 3) ((a))
module tb;
initial begin $display("DIGEST=%0d", `M(1)); #1 $finish; end
endmodule
"#,
        "`define parameter name is invalid",
    );
    // err_dup_param: split (vita = verilator) — duplicate formal (both oracles reject)
    loud(
        "err_dup_param",
        r#"`define M(a, a = 3) ((a))
module tb;
initial begin $display("DIGEST=%0d", `M(1)); #1 $finish; end
endmodule
"#,
        "`define has a duplicate parameter name",
    );
    // err_omit_no_default: verilator — arity error (both oracles)
    loud(
        "err_omit_no_default",
        r#"`define M(a, b) ((a)+(b))
module tb;
initial begin $display("DIGEST=%0d", `M(1)); #1 $finish; end
endmodule
"#,
        "macro `M expects 2 argument(s), got 1 — formal `b` has no default",
    );
    // err_too_many: verilator — arity error (both oracles)
    loud(
        "err_too_many",
        r#"`define M(a, b = 1) ((a)+(b))
module tb;
initial begin $display("DIGEST=%0d", `M(1,2,3)); #1 $finish; end
endmodule
"#,
        "macro `M expects 2 argument(s), got 3",
    );
}

#[test]
fn cell_fl() {
    // fl_after_multiline_define: both oracles
    digest(
        "fl_after_multiline_define",
        r#"`define BIG \
 a \
 b
module tb;
initial begin $display("DIGEST=%0d", `__LINE__); #1 $finish; end
endmodule
"#,
        "5",
    );
    // fl_in_body: both oracles
    digest(
        "fl_in_body",
        r#"`define L `__LINE__
module tb;
initial begin $display("DIGEST=%0d", `L); #1 $finish; end
endmodule
"#,
        "3",
    );
    // fl_in_body_fn: both oracles
    digest(
        "fl_in_body_fn",
        r#"`define L(x) (`__LINE__ + (x))
module tb;
initial begin $display("DIGEST=%0d", `L(100)); #1 $finish; end
endmodule
"#,
        "103",
    );
    // fl_in_string_literal: both oracles
    digest(
        "fl_in_string_literal",
        r#"module tb;
initial begin $display("DIGEST=%s", "`__LINE__"); #1 $finish; end
endmodule
"#,
        "`__LINE__",
    );
    // fl_include_body: both oracles
    digest(
        "fl_include_body",
        r#"`include "h.svh"
module tb;
initial begin $display("DIGEST=%0d", `INCL); #1 $finish; end
endmodule
"#,
        "3",
    );
    // fl_include_default: both oracles
    digest(
        "fl_include_default",
        r#"`include "h.svh"
module tb;
initial begin $display("DIGEST=%0d", `INCADD(1)); #1 $finish; end
endmodule
"#,
        "4",
    );
    // fl_include_file: both oracles
    digest(
        "fl_include_file",
        r#"`include "h.svh"
module tb;
initial begin $display("DIGEST=%s", `INCF); #1 $finish; end
endmodule
"#,
        "t.sv",
    );
    // fl_line_after_continuation_use: both oracles
    digest(
        "fl_line_after_continuation_use",
        r#"`define M(a) ((a)+1)
module tb;
initial begin $display("DIGEST=%0d", `M(1) + 
 `__LINE__); #1 $finish; end
endmodule
"#,
        "6",
    );
    // fl_multiline_use: both oracles
    digest(
        "fl_multiline_use",
        r#"`define L(x, y) (`__LINE__ + (x) + (y))
module tb;
initial begin $display("DIGEST=%0d", `L(100,
 200)); #1 $finish; end
endmodule
"#,
        "304",
    );
    // fl_nested_body: both oracles
    digest(
        "fl_nested_body",
        r#"`define IN `__LINE__
`define OUT (`IN * 1000 + `__LINE__)
module tb;
initial begin $display("DIGEST=%0d", `OUT); #1 $finish; end
endmodule
"#,
        "4004",
    );
    // fl_top_file: both oracles
    digest(
        "fl_top_file",
        r#"module tb;
initial begin $display("DIGEST=%s", `__FILE__); #1 $finish; end
endmodule
"#,
        "t.sv",
    );
    // fl_top_line: both oracles
    digest(
        "fl_top_line",
        r#"module tb;
initial begin $display("DIGEST=%0d", `__LINE__); #1 $finish; end
endmodule
"#,
        "2",
    );
}

#[test]
fn cell_kind() {
    // kind_brack: both oracles
    digest(
        "kind_brack",
        r#"`define M(a, b = {4{1'b1}}) ((a)+(b))
module tb;
initial begin $display("DIGEST=%0d", `M(1)); #1 $finish; end
endmodule
"#,
        "16",
    );
    // kind_brack_given: both oracles
    digest(
        "kind_brack_given",
        r#"`define M(a, b = {4{1'b1}}) ((a)+(b))
module tb;
initial begin $display("DIGEST=%0d", `M(1, 9)); #1 $finish; end
endmodule
"#,
        "10",
    );
    // kind_empty_given: both oracles
    digest(
        "kind_empty_given",
        r#"`define M(a, b = ) ((a)+(b))
module tb;
initial begin $display("DIGEST=%0d", `M(1, 9)); #1 $finish; end
endmodule
"#,
        "10",
    );
    // kind_expr: both oracles
    digest(
        "kind_expr",
        r#"`define M(a, b = 2*3) ((a)+(b))
module tb;
initial begin $display("DIGEST=%0d", `M(1)); #1 $finish; end
endmodule
"#,
        "7",
    );
    // kind_expr_given: both oracles
    digest(
        "kind_expr_given",
        r#"`define M(a, b = 2*3) ((a)+(b))
module tb;
initial begin $display("DIGEST=%0d", `M(1, 9)); #1 $finish; end
endmodule
"#,
        "10",
    );
    // kind_late: both oracles
    digest(
        "kind_late",
        r#"`define M(a, b = `LATE) ((a)+(b))
`define LATE 6
module tb;
initial begin $display("DIGEST=%0d", `M(1)); #1 $finish; end
endmodule
"#,
        "7",
    );
    // kind_late_given: both oracles
    digest(
        "kind_late_given",
        r#"`define M(a, b = `LATE) ((a)+(b))
`define LATE 6
module tb;
initial begin $display("DIGEST=%0d", `M(1, 9)); #1 $finish; end
endmodule
"#,
        "10",
    );
    // kind_lit: both oracles
    digest(
        "kind_lit",
        r#"`define M(a, b = 5) ((a)+(b))
module tb;
initial begin $display("DIGEST=%0d", `M(1)); #1 $finish; end
endmodule
"#,
        "6",
    );
    // kind_lit_given: both oracles
    digest(
        "kind_lit_given",
        r#"`define M(a, b = 5) ((a)+(b))
module tb;
initial begin $display("DIGEST=%0d", `M(1, 9)); #1 $finish; end
endmodule
"#,
        "10",
    );
    // kind_macro: both oracles
    digest(
        "kind_macro",
        r#"`define D 4
`define M(a, b = `D) ((a)+(b))
module tb;
initial begin $display("DIGEST=%0d", `M(1)); #1 $finish; end
endmodule
"#,
        "5",
    );
    // kind_macro_given: both oracles
    digest(
        "kind_macro_given",
        r#"`define D 4
`define M(a, b = `D) ((a)+(b))
module tb;
initial begin $display("DIGEST=%0d", `M(1, 9)); #1 $finish; end
endmodule
"#,
        "10",
    );
    // kind_paren: verilator
    digest(
        "kind_paren",
        r#"`define M(a, b = (1+2)) ((a)+(b))
module tb;
initial begin $display("DIGEST=%0d", `M(1)); #1 $finish; end
endmodule
"#,
        "4",
    );
    // kind_paren_given: verilator
    digest(
        "kind_paren_given",
        r#"`define M(a, b = (1+2)) ((a)+(b))
module tb;
initial begin $display("DIGEST=%0d", `M(1, 9)); #1 $finish; end
endmodule
"#,
        "10",
    );
    // kind_str: both oracles
    digest(
        "kind_str",
        r#"`define M(a, b = "s") $sformatf("%0d%s", a, b)
module tb;
initial begin $display("DIGEST=%s", `M(1)); #1 $finish; end
endmodule
"#,
        "1s",
    );
    // kind_str_given: both oracles
    digest(
        "kind_str_given",
        r#"`define M(a, b = "s") $sformatf("%0d%s", a, b)
module tb;
initial begin $display("DIGEST=%s", `M(1, "q")); #1 $finish; end
endmodule
"#,
        "1q",
    );
}
