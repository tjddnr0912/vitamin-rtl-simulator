//! V33-1 — a `generate` rejection anchored its caret at the MODULE HEADER.
//!
//! Every one of the ten `self.error(` calls in `elaborate/src/generate.rs` fell
//! through to `cur_span`, which during module elaboration is the module header.
//! So a module holding a `generate if`, a `generate case` and a `generate for`
//! that all rejected printed the same `file:line:col` three times, at a line with
//! no generate on it — the reader could not tell which construct each sentence
//! was about, and the only way to find out was to delete them one at a time.
//! §4.5.382 fixed exactly this shape for parameter declarations
//! (`params.rs::param_value_unfoldable`) and did not reach this file.
//!
//! And the five const-fold rejections stopped at "is not a constant", naming the
//! construct and nothing else. Both oracles name the offending sub-expression:
//!
//!   iverilog 13.0  `A reference to a net or variable (`q') is not allowed in a
//!                   constant expression.`
//!   verilator 5.050 `Expecting expression to be constant, but variable isn't
//!                    const: 'q'`
//!
//! so appending `unfoldable_reason` is reaching parity, not inventing a format.
//!
//! ORACLE PINS. Every LINE asserted here is the line iverilog 13.0 reports for
//! the same source, measured live; the three columns marked below are also
//! verilator 5.050's exact column. The vita-only limits (func/task/defparam and
//! a port declaration inside a generate, the unroll cap, the nesting cap) have no
//! oracle — iverilog accepts the first two and the caps are vita's — so those
//! cases assert only the property this slice is about: distinct anchors, none of
//! them the module header.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Run vita on `src` and hand back its stderr (where diagnostics go) with the
/// temp path stripped, so an assertion can match `t.sv:LINE:COL`.
fn diags(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_gra_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let err = String::from_utf8_lossy(&out.stderr).into_owned();
    let _ = std::fs::remove_dir_all(&d);
    err.replace(f.to_str().unwrap(), "t.sv")
}

/// The `LINE:COL` of every error line, in order.
fn error_anchors(err: &str) -> Vec<String> {
    err.lines()
        .filter(|l| l.contains("error[VITA-"))
        .filter_map(|l| l.strip_prefix("t.sv:"))
        .map(|rest| {
            let mut it = rest.split(':');
            let line = it.next().unwrap_or("");
            let col = it.next().unwrap_or("");
            format!("{line}:{col}")
        })
        .collect()
}

/// The repro. Three rejections, three constructs, one module.
///
/// iverilog 13.0 on this exact source:
///   t.sv:4: Cannot evaluate genvar conditional expression: q
///   t.sv:5: Cannot evaluate genvar case expression: q
///   t.sv:6: Cannot evaluate generate "loop" conditional expression: (i)<(q)
/// verilator 5.050 adds the columns: 4:16 and 5:18 (it aborts with an internal
/// error before reaching the third).
#[test]
fn three_generate_rejections_print_three_distinct_locations() {
    let err = diags(
        "module t;\n\
         \x20 logic q;\n\
         \x20 genvar i;\n\
         \x20 generate if (q) begin : g1 wire w1; end endgenerate\n\
         \x20 generate case (q) 0: begin : g2 wire w2; end endcase endgenerate\n\
         \x20 generate for (i = 0; i < q; i = i + 1) begin : g3 wire w3; end endgenerate\n\
         \x20 initial $display(\"x\");\n\
         endmodule\n",
    );
    let anchors = error_anchors(&err);
    // The whole point: three rejections, three anchors, all different.
    assert_eq!(anchors.len(), 3, "expected three rejections; got:\n{err}");
    assert_eq!(
        anchors,
        // 4:16 and 5:18 are verilator's exact columns; every line is iverilog's.
        vec!["4:16".to_string(), "5:18".to_string(), "6:24".to_string()],
        "each generate must anchor on its own condition; got:\n{err}"
    );
    // The old behaviour, stated as the thing that must not come back: all three
    // on line 1, the `module t;` header.
    assert!(
        !anchors.iter().any(|a| a.starts_with("1:")),
        "no rejection may anchor on the module header; got:\n{err}"
    );
}

/// The second half: the message names the sub-expression the constant domain
/// has no answer for. Both oracles name `q`; before this slice neither the name
/// nor any other cause appeared in any of the three sentences.
#[test]
fn a_generate_rejection_names_the_offending_subexpression() {
    let err = diags(
        "module t;\n\
         \x20 logic q;\n\
         \x20 genvar i;\n\
         \x20 generate if (q) begin : g1 wire w1; end endgenerate\n\
         \x20 generate case (q) 0: begin : g2 wire w2; end endcase endgenerate\n\
         \x20 generate for (i = 0; i < q; i = i + 1) begin : g3 wire w3; end endgenerate\n\
         \x20 initial $display(\"x\");\n\
         endmodule\n",
    );
    for what in [
        "generate-if condition is not a constant:",
        "generate-case scrutinee is not a constant:",
        "generate-for condition is not a constant:",
    ] {
        let line = err
            .lines()
            .find(|l| l.contains(what))
            .unwrap_or_else(|| panic!("no line saying `{what}`; got:\n{err}"));
        assert!(
            line.contains('`') && line.contains("`q`"),
            "`{what}` must name `q` like both oracles do; got:\n{line}"
        );
    }
}

/// The for-loop's init and step anchor on their own expression, not on the
/// `for` and not on the module header.
///
/// iverilog 13.0: line 4 ("loop initialization expression: q") and line 5
/// ("loop increment expression: (j)+(q)"). verilator 5.050 gives 4:21 for the
/// initializer — vita's column exactly.
#[test]
fn for_init_and_step_anchor_on_their_own_expression() {
    let err = diags(
        "module t;\n\
         \x20 logic q;\n\
         \x20 genvar i, j;\n\
         \x20 generate for (i = q; i < 2; i = i + 1) begin : a wire w; end endgenerate\n\
         \x20 generate for (j = 0; j < 2; j = j + q) begin : b wire w; end endgenerate\n\
         \x20 initial $display(\"x\");\n\
         endmodule\n",
    );
    assert_eq!(
        error_anchors(&err),
        vec!["4:21".to_string(), "5:35".to_string()],
        "init and step anchor on their own value; got:\n{err}"
    );
    assert!(
        err.contains("generate-for init is not a constant: ")
            && err.contains("generate-for step is not a constant: "),
        "both name a cause; got:\n{err}"
    );
    for l in err.lines().filter(|l| l.contains("error[VITA-")) {
        assert!(l.contains("`q`"), "must name `q`; got:\n{l}");
    }
}

/// A genvar whose step leaves it unchanged. iverilog 13.0 reports this on line 4
/// too ("The generate \"loop\" is not incrementing"), so the LINE is pinned;
/// the anchor is the `for` itself because the defect is the loop, not one
/// expression in it.
#[test]
fn a_non_advancing_genvar_anchors_on_its_loop() {
    let err = diags(
        "module t;\n\
         \x20 genvar i;\n\
         \x20 initial $display(\"x\");\n\
         \x20 generate for (i = 0; i < 4; i = i) begin : a wire w; end endgenerate\n\
         endmodule\n",
    );
    assert_eq!(
        error_anchors(&err),
        vec!["4:12".to_string()],
        "the loop is the anchor; got:\n{err}"
    );
    assert!(
        err.contains("genvar does not advance"),
        "message unchanged; got:\n{err}"
    );
}

/// The three vita-only rejections that are not const folds — a function, a task
/// and a body port declaration inside a generate — used to share the module
/// header as well. No oracle: iverilog accepts a function inside a generate
/// block, so only the anchoring is asserted here.
#[test]
fn deferred_constructs_inside_generate_anchor_on_themselves() {
    let err = diags(
        "module t;\n\
         \x20 initial $display(\"x\");\n\
         \x20 generate\n\
         \x20   if (1) begin : b\n\
         \x20     function int f(); return 1; endfunction\n\
         \x20     task tk(); endtask\n\
         \x20     input z;\n\
         \x20   end\n\
         \x20 endgenerate\n\
         endmodule\n",
    );
    let anchors = error_anchors(&err);
    assert_eq!(anchors.len(), 3, "three rejections expected; got:\n{err}");
    assert_eq!(
        anchors,
        vec!["5:7".to_string(), "6:7".to_string(), "7:7".to_string()],
        "each deferred construct anchors on its own declaration; got:\n{err}"
    );
}
