//! Preprocessor rung behind ibex's whole design (§4.5.417): `define default argument
//! values (IEEE 1800-2017 §22.5.1 — omitted and empty actuals take the default, a
//! default is bound at USE time like an actual), a macro body that keeps one directive
//! per continued line (`\`ifdef … `\`else … `\`endif` inside a body, evaluated at
//! expansion time), a `// comment \` line that continues the body (the comment is
//! dropped), and `\`__FILE__` / `\`__LINE__` (§22.13 — the USE's file and the line where
//! its argument list closes, inside a body and through an `include).
//!
//! Every expected value is the grounding-probe oracle line (iverilog 13.0 `-g2012`
//! and verilator 5.050 agree unless the comment says which one ran). File names are
//! printed as given on the command line, so `\`__FILE__` reads `t.sv`.

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
    std::fs::write(
        d.join("sub4.svh"),
        "// included from a macro body\n\n`ifdef NOPE\nlogic never;\n",
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
fn probe_p01() {
    // p01_default_basic: both oracles
    digest(
        "p01_default_basic",
        r#"`define M(a, b = 5) ((a)+(b))
module tb;
initial $display("DIGEST=%0d %0d %0d", `M(1), `M(1,), `M(1,2));
initial begin #1 $finish; end
endmodule
"#,
        "6 6 3",
    );
}

#[test]
fn probe_p02() {
    // p02_default_macro: both oracles
    digest(
        "p02_default_macro",
        r#"`define DEF 7
`define M(a, b = `DEF) ((a)+(b))
module tb;
initial $display("DIGEST=%0d %0d", `M(1), `M(1, 3));
initial begin #1 $finish; end
endmodule
"#,
        "8 4",
    );
}

#[test]
fn probe_p03() {
    // p03_default_two: both oracles
    digest(
        "p03_default_two",
        r#"`define M(a = 1, b = 2) ((a)*10+(b))
module tb;
initial $display("DIGEST=%0d %0d %0d %0d", `M(), `M(3), `M(,4), `M(3,4));
initial begin #1 $finish; end
endmodule
"#,
        "12 32 14 34",
    );
}

#[test]
fn probe_p04() {
    // p04_empty_no_default: verilator — an EMPTY actual for a formal without a default substitutes empty text (`((1)+())` — a parse error, both oracles reject)
    loud(
        "p04_empty_no_default",
        r#"`define M(a, b) ((a)+(b))
module tb;
initial $display("DIGEST=%0d", `M(1,));
initial begin #1 $finish; end
endmodule
"#,
        "expected expression, found ')'",
    );
}

#[test]
fn probe_p05() {
    // p05_ifdef_in_body: both oracles
    digest(
        "p05_ifdef_in_body",
        r#"`define M(a) \
`ifdef FLAG \
  (a)+100 \
`else \
  (a)+1 \
`endif
module tb;
initial $display("DIGEST=%0d", `M(2));
initial begin #1 $finish; end
endmodule
"#,
        "3",
    );
}

#[test]
fn probe_p05b() {
    // p05b_ifdef_in_body_defined: both oracles
    digest(
        "p05b_ifdef_in_body_defined",
        r#"`define FLAG
`define M(a) \
`ifdef FLAG \
  (a)+100 \
`else \
  (a)+1 \
`endif
module tb;
initial $display("DIGEST=%0d", `M(2));
initial begin #1 $finish; end
endmodule
"#,
        "102",
    );
}

#[test]
fn probe_p06() {
    // p06_ifdef_body_flag_after: both oracles
    digest(
        "p06_ifdef_body_flag_after",
        r#"`define M(a) \
`ifdef FLAG \
  (a)+100 \
`else \
  (a)+1 \
`endif
`define FLAG
module tb;
initial $display("DIGEST=%0d", `M(2));
initial begin #1 $finish; end
endmodule
"#,
        "102",
    );
}

#[test]
fn probe_p07() {
    // p07_fewer_no_default: verilator — an OMITTED actual whose formal has no default is an arity error (both oracles)
    loud(
        "p07_fewer_no_default",
        r#"`define M(a, b) ((a)+(b))
module tb;
initial $display("DIGEST=%0d", `M(1));
initial begin #1 $finish; end
endmodule
"#,
        "macro `M expects 2 argument(s), got 1 — formal `b` has no default",
    );
}

#[test]
fn probe_p08() {
    // p08_default_names_param: verilator — a default naming another formal is substituted as literal text (`b = a` leaves `a` unbound — both oracles reject)
    loud(
        "p08_default_names_param",
        r#"`define M(a, b = a) ((a)*10+(b))
module tb;
initial $display("DIGEST=%0d %0d", `M(3), `M(3,4));
initial begin #1 $finish; end
endmodule
"#,
        "undeclared net/variable `tb.a`",
    );
}

#[test]
fn probe_p09() {
    // p09_default_parens_string: verilator
    digest(
        "p09_default_parens_string",
        r#"`define M(a, b = (1+2), c = "xy") $sformatf("%0d-%0d-%s", a, b, c)
module tb;
initial $display("DIGEST=%s %s", `M(1), `M(1, 4, "q"));
initial begin #1 $finish; end
endmodule
"#,
        "1-3-xy 1-4-q",
    );
}

#[test]
fn probe_p10() {
    // p10_ws_comment_in_list: both oracles
    digest(
        "p10_ws_comment_in_list",
        r#"`define M(a /* first */ , b = 3 ) ((a)+(b))
module tb;
initial $display("DIGEST=%0d %0d", `M(1), `M(1,2));
initial begin #1 $finish; end
endmodule
"#,
        "4 3",
    );
}

#[test]
fn probe_p11() {
    // p11_default_macro_late: both oracles
    digest(
        "p11_default_macro_late",
        r#"`define M(a, b = `LATE) ((a)+(b))
`define LATE 9
module tb;
initial $display("DIGEST=%0d", `M(1));
initial begin #1 $finish; end
endmodule
"#,
        "10",
    );
}

#[test]
fn probe_p12() {
    // p12_nested_default_call: verilator — a default that calls another macro with a formal as its argument (both oracles reject)
    loud(
        "p12_nested_default_call",
        r#"`define IN(x, y = 2) ((x)*(y))
`define M(a, b = `IN(a)) ((a)+(b))
module tb;
initial $display("DIGEST=%0d %0d", `M(3), `M(3, 1));
initial begin #1 $finish; end
endmodule
"#,
        "undeclared net/variable `tb.a`",
    );
}

#[test]
fn probe_p13() {
    // p13_ifdef_body_include_shape: both oracles
    digest(
        "p13_ifdef_body_include_shape",
        r#"`define ASSERT_ERROR(__name) \
`ifdef UVM \
  $display("uvm %s", `"__name`"); \
`else \
  $error("ASSERT FAILED: %s", `"__name`"); \
`endif
`define ASSERT_I(__name, __prop) \
  always_comb begin if (!(__prop)) begin `ASSERT_ERROR(__name) end end
module tb;
logic ok = 1;
`ASSERT_I(a1, ok)
initial $display("DIGEST=ok");
initial begin #1 $finish; end
endmodule
"#,
        "ok",
    );
}

#[test]
fn probe_p14() {
    // p14_default_used_twice: both oracles
    digest(
        "p14_default_used_twice",
        r#"`define M(a, b = 5) ((a)+(b)+(b))
module tb;
initial $display("DIGEST=%0d %0d", `M(1), `M(1,2));
initial begin #1 $finish; end
endmodule
"#,
        "11 5",
    );
}

#[test]
fn probe_q01() {
    // q01_file_line: both oracles
    digest(
        "q01_file_line",
        r#"`define WHERE(x) $sformatf("%0d:%s", `__LINE__, x)
module tb;
  initial begin
    $display("DIGEST=%0d", `__LINE__);
    $display("DIGEST=%s", `WHERE("a"));
    $display("DIGEST=%s",
             `WHERE("b"));
    #1 $finish;
  end
endmodule
"#,
        "4|5:a|7:b",
    );
}

#[test]
fn probe_q02() {
    // q02_file_name: both oracles
    digest(
        "q02_file_name",
        r#"module tb;
  initial begin
    $display("DIGEST=%s", `__FILE__);
    #1 $finish;
  end
endmodule
"#,
        "t.sv",
    );
}

#[test]
fn probe_q03() {
    // q03_file_in_include: both oracles
    digest(
        "q03_file_in_include",
        r#"`include "hdr.svh"
module tb;
  initial begin
    $display("DIGEST=%0d %s", `INCL, `INCF);
    #1 $finish;
  end
endmodule
"#,
        "4 t.sv",
    );
}

#[test]
fn probe_q04() {
    // q04_comment_continuation: both oracles
    digest(
        "q04_comment_continuation",
        r#"`define M(a) \
  ((a) + \
  // a comment that continues the body \
  1)
`define N(a) \
`ifdef FLAG \
  // inside the taken arm \
  (a) + 10 \
`else \
  (a) + 20 \
`endif
module tb;
  initial begin
    $display("DIGEST=%0d %0d", `M(2), `N(2));
    #1 $finish;
  end
endmodule
"#,
        "3 22",
    );
}

#[test]
fn probe_q05() {
    // q05_comment_ends_body: both oracles
    digest(
        "q05_comment_ends_body",
        r#"`define M(a) ((a) + 1) // trailing comment
`define K 5 // trailing
module tb;
  initial begin
    $display("DIGEST=%0d %0d", `M(2), `K);
    #1 $finish;
  end
endmodule
"#,
        "3 5",
    );
}

#[test]
fn probe_q06() {
    // q06_line_in_multiline_use: both oracles
    digest(
        "q06_line_in_multiline_use",
        r#"`define TWO(a, b) $sformatf("%0d:%0d:%0d", `__LINE__, a, b)
module tb;
  initial begin
    $display("DIGEST=%s", `TWO(1,
                               2));
    #1 $finish;
  end
endmodule
"#,
        "5:1:2",
    );
}

/// Adversarial-review pins (§4.5.417). B1: `` `__LINE__ `` inside the ACTUALS of a
/// multi-line use is the line where the use closes (the anchor is set before the
/// actuals are pre-expanded). B2: an `include performed from a macro body is scanned
/// verbatim — an `ifdef diagnostic inside it names the included file, not the use.
/// A F1: a blank actual whose only content is a COMMENT takes the default (ibex's
/// `` `ASSERT(x, y, /*clk*/, /*rst*/) ``). A F2: a default holding `` `"…`" `` is
/// resolved as macro text. Oracles: iverilog 13.0 and verilator 5.050 agree.
#[test]
fn review_pins() {
    digest(
        "b1_line_in_multiline_actual",
        "`define ID(x) (x)\nmodule tb;\ninitial begin $display(\"DIGEST=%0d\", `ID(\n   `__LINE__\n   )); #1 $finish; end\nendmodule\n",
        "5",
    );
    digest(
        "b1_two_lines_in_actuals",
        "`define TWO(a, b) ((a)*100 + (b))\nmodule tb;\ninitial begin $display(\"DIGEST=%0d\", `TWO(`__LINE__,\n `__LINE__)); #1 $finish; end\nendmodule\n",
        "404",
    );
    {
        let (out, rc) = run("`define INC `include \"sub4.svh\"\nmodule tb;\ninitial begin $display(\"DIGEST=x\"); #1 $finish; end\n`INC\nendmodule\n");
        assert_ne!(
            rc,
            Some(0),
            "b2: an unterminated `ifdef inside the include is loud:\n{out}"
        );
        assert!(
            out.contains("sub4.svh:3:12: error[VITA-E1013]"),
            "b2: the diagnostic names the INCLUDED file's own line, not the macro use:\n{out}"
        );
    }
    digest(
        "a_f1_comment_only_actual_takes_default",
        "`define M(a, b = 5) ((a)*10+(b))\nmodule tb;\ninitial begin $display(\"DIGEST=%0d %0d %0d\", `M(1, /*c*/ ), `M(1,/*c*/), `M(1, // c\n )); #1 $finish; end\nendmodule\n",
        "15 15 15",
    );
    digest(
        "a_f1_first_formal_comment_only",
        "`define M(a = 4, b) ((a)*10+(b))\nmodule tb;\ninitial begin $display(\"DIGEST=%0d\", `M(/*z*/, 2)); #1 $finish; end\nendmodule\n",
        "42",
    );
    digest(
        "a_f2_default_with_stringify",
        "`define M(a, b = `\"zz`\") $display(\"DIGEST=%0d %s\", (a), b);\nmodule tb;\ninitial begin `M(3) #1 $finish; end\nendmodule\n",
        "3 zz",
    );
}
