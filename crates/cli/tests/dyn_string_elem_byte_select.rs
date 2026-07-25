//! Byte select on a DYNAMIC string-array element — `d[0][0]` (element, then the
//! IEEE §6.16.2 byte select).
//!
//! iverilog rejects `d[0][0]` outright ("number of indices is greater than the number
//! of dimensions"), so there is no direct oracle. The teeth here are the
//! vita-INTERNAL equivalence: a dynamic element holding "world" must byte-select
//! byte-identically to a FIXED element holding "world", and the fixed form IS
//! oracle-verified (iverilog gives 119/111 for `string s[2]; s[0]="world"; s[0][0]`).
//! A scalar `string p = "world"` is the third reference point, also oracle-verified.
//!
//! Before this slice the dynamic form read a silent 0: `string_index_read` routes to
//! the `.getc` byte primitive only when the lowered base is a handle the engine can
//! read bytes from, and that gate accepted only a word-LESS `Signal` or a `Const`.
//! A dyn element lowers to a WORD-INDEXED `Signal`, so it fell through to a packed
//! bit-select of a width-0 handle. The engine's `handle_str_bytes` could always read
//! it — that is the same eval fallback `%s` and a compare on `d[i]` already use — so
//! the gate was simply under-approximating its own predicate.
//!
//! This is the recorded prerequisite for routing the fixed string-array
//! representation at the dynamic one: until the dynamic path is at least as capable
//! (dyn ⊇ fixed), such a routing would regress byte-select into a silent 0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_dsebs_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    assert!(
        out.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let so = String::from_utf8_lossy(&out.stdout).into_owned();
    let mut s = String::new();
    for l in so.lines().filter(|l| !l.starts_with("simulation ended")) {
        s.push_str(l);
        s.push('\n');
    }
    s
}

/// The same body over the three string-holding forms; all three must agree, and the
/// FIXED and SCALAR forms are the iverilog-verified references.
fn all_three_agree(body_fmt: &str, expect: &str) {
    let fixed = run(&format!(
        "module t;\n  string s[2];\n  initial begin\n    s[0] = \"world\";\n{}\n  end\nendmodule\n",
        body_fmt.replace("@", "s")
    ));
    let dynamic = run(&format!(
        "module t;\n  string s[] = '{{\"world\",\"x\"}};\n  initial begin\n{}\n  end\nendmodule\n",
        body_fmt.replace("@", "s")
    ));
    let dyn_new = run(&format!(
        "module t;\n  string s[];\n  initial begin\n    s = new[2];\n    s[0] = \"world\";\n{}\n  end\nendmodule\n",
        body_fmt.replace("@", "s")
    ));
    assert_eq!(fixed, expect, "fixed form (iverilog-verified reference)");
    assert_eq!(dynamic, expect, "dynamic form");
    assert_eq!(dyn_new, expect, "dynamic form built with new[]");
}

#[test]
fn byte_select_agrees_across_fixed_and_dynamic() {
    // 'w'=119, 'o'=111 — iverilog gives exactly this for the fixed form.
    all_three_agree("    $display(\"%0d %0d\", @[0][0], @[0][1]);", "119 111\n");
}

#[test]
fn byte_select_matches_the_scalar_string_reference() {
    // The third reference point, also oracle-verified.
    let scalar = run("module t;\n\
           string p = \"world\";\n\
           initial $display(\"%0d %0d\", p[0], p[1]);\n\
         endmodule\n");
    assert_eq!(scalar, "119 111\n");
}

#[test]
fn out_of_range_byte_index_is_zero_on_both_paths() {
    // IEEE §6.16.2: an out-of-range byte select reads 0.
    all_three_agree("    $display(\"%0d\", @[0][9]);", "0\n");
}

#[test]
fn runtime_element_index_then_byte_select() {
    let out = run("module t;\n\
           string d[] = '{\"world\",\"x\"};\n\
           int i;\n\
           initial begin i = 0; $display(\"%0d\", d[i][0]); end\n\
         endmodule\n");
    assert_eq!(out, "119\n");
}

#[test]
fn byte_select_inside_an_expression() {
    // The byte is an ordinary integral once selected.
    all_three_agree(
        "    $display(\"%0d %0d\", @[0][0] + 1, @[0][0] == 119);",
        "120 1\n",
    );
}

// ── boundaries: a NON-string element must keep the packed bit-select ─────────

#[test]
fn int_dynamic_array_element_keeps_bit_select() {
    // `d[0]` is an int here, so `d[0][0]` is bit 0 of 0x5 = 1, bit 1 = 0. Treating it
    // as a string handle would be wrong; the `string_elem_dyn_nets` test is what keeps
    // the widened gate from capturing it.
    let out = run("module t;\n\
           int d[];\n\
           initial begin d = new[2]; d[0] = 32'h5; $display(\"%0d %0d\", d[0][0], d[0][1]); end\n\
         endmodule\n");
    assert_eq!(out, "1 0\n");
}

#[test]
fn logic_vector_dynamic_array_element_keeps_bit_select() {
    let out = run("module t;\n\
           logic [7:0] d[];\n\
           initial begin\n\
             d = new[2]; d[0] = 8'b0000_0110;\n\
             $display(\"%0d %0d %0d\", d[0][0], d[0][1], d[0][2]);\n\
           end\n\
         endmodule\n");
    assert_eq!(out, "0 1 1\n");
}

#[test]
fn element_methods_are_undisturbed() {
    // NOTE: `d[0].len()` is an `ExprKind::MethodCall`, dispatched at a DIFFERENT site
    // (gated by `ir_expr_is_string`), not through the gate this slice widened. It
    // already worked and must stay byte-identical — see the inline-function tests
    // below for the consumer that this change actually newly enables.
    all_three_agree(
        "    $display(\"%0d %s\", @[0].len(), @[0].substr(0,2));",
        "5 wor\n",
    );
}

#[test]
fn element_still_renders_and_compares_as_a_string() {
    all_three_agree(
        "    $display(\"[%s] %0d\", @[0], @[0] == \"world\");",
        "[world] 1\n",
    );
}

// ── the second consumer: inline-function string-method dispatch ──────────────
//
// A 4-state-return function stays INLINE (`build_frame_set` frames anything
// `automatic` / 2-state-return / with control flow), and an inline `string` formal is
// bound verbatim — so the receiver lowers to the word-indexed `Signal` of the caller's
// element. That reaches the gate through `inline_fn.rs`, the consumer the tests above
// do NOT cover. Every one of these was a loud "unsupported hierarchical function call"
// before this slice.

fn inline_fn(ret: &str, body: &str, args: &str) -> String {
    run(&format!(
        "module t;\n  string d[] = '{{\"world\",\"x\"}};\n  \
         function {ret} f(input string s); f = {body}; endfunction\n  \
         initial $display({args});\nendmodule\n"
    ))
}

#[test]
fn inline_function_string_formal_byte_select() {
    assert_eq!(inline_fn("[7:0]", "s[0]", "\"%0d\", f(d[0])"), "119\n");
}

#[test]
fn inline_function_string_formal_len_and_getc() {
    assert_eq!(inline_fn("[31:0]", "s.len()", "\"%0d\", f(d[0])"), "5\n");
    assert_eq!(inline_fn("[7:0]", "s.getc(1)", "\"%0d\", f(d[0])"), "111\n");
}

#[test]
fn inline_function_string_formal_compare_and_atoi() {
    assert_eq!(
        inline_fn("[31:0]", "s.compare(\"world\")", "\"%0d\", f(d[0])"),
        "0\n"
    );
    let out = run("module t;\n\
           string d[] = '{\"42\",\"x\"};\n\
           function [31:0] f(input string s); f = s.atoi(); endfunction\n\
           initial $display(\"%0d\", f(d[0]));\n\
         endmodule\n");
    assert_eq!(out, "42\n");
}

// ── declaration scopes (the decl-collector asymmetry trap) ──────────────────

#[test]
fn block_local_dynamic_string_array_byte_select() {
    let out = run("module t;\n\
           initial begin : blk\n\
             string d[] = '{\"world\",\"x\"};\n\
             $display(\"%0d %0d\", d[0][0], d[0][1]);\n\
           end\n\
         endmodule\n");
    assert_eq!(out, "119 111\n");
}

#[test]
fn generate_scope_dynamic_string_array_byte_select() {
    let out = run("module t;\n\
           genvar i;\n\
           generate for (i=0;i<1;i=i+1) begin : g\n\
             string d[];\n\
             initial begin d = new[2]; d[0] = \"world\"; $display(\"%0d\", d[0][0]); end\n\
           end endgenerate\n\
         endmodule\n");
    assert_eq!(out, "119\n");
}

#[test]
fn dynamic_record_array_string_member_byte_select() {
    // The SoA member reaches the same marker through the parser's `$unp$r$nm[]`
    // desugar, so it is fixed by the same change. The FIXED record array already
    // worked and is the reference.
    let dynamic = run("module t;\n\
           typedef struct { int id; string nm; } rec_t;\n\
           rec_t r[];\n\
           initial begin r = new[2]; r[0].nm = \"world\"; $display(\"%0d %0d\", r[0].nm[0], r[0].nm[1]); end\n\
         endmodule\n");
    let fixed = run("module t;\n\
           typedef struct { int id; string nm; } rec_t;\n\
           rec_t r[2];\n\
           initial begin r[0].nm = \"world\"; $display(\"%0d %0d\", r[0].nm[0], r[0].nm[1]); end\n\
         endmodule\n");
    assert_eq!(dynamic, "119 111\n");
    assert_eq!(fixed, "119 111\n", "fixed reference");
}

// ── write twin: loud in every SELECT form, never a partial byte write ────────

fn loud(src: &str, needle: &str) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_dsebs_l_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    assert!(!out.status.success(), "expected a loud reject");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains(needle), "unexpected diagnostic:\n{err}");
}

#[test]
fn bit_select_write_into_an_element_is_loud() {
    loud(
        "module t;\n\
           string d[] = '{\"world\",\"x\"};\n\
           initial begin d[0][0] = \"W\"; $display(\"[%s]\", d[0]); end\n\
         endmodule\n",
        "nested lvalue select",
    );
}

#[test]
fn part_select_write_into_an_element_is_loud() {
    // Was SILENTLY WRONG: the engine re-derives the element from its byte string and
    // discards the offset/width, so `d[0][3:0] = 4'hF` on "world" wrote '5' giving
    // "worle", and `d[0][15:8] = 8'h41` did nothing at all. The fixed and scalar twins
    // both reject this shape.
    for lhs in [
        "d[0][3:0] = 4'hF",
        "d[0][15:8] = 8'h41",
        "d[0][7 +: 8] = 8'h41",
    ] {
        loud(
            &format!(
                "module t;\n\
                   string d[] = '{{\"world\",\"x\"}};\n\
                   initial begin {lhs}; $display(\"[%s]\", d[0]); end\n\
                 endmodule\n"
            ),
            "part-select write into a string / real array element",
        );
    }
}

#[test]
fn whole_element_write_still_works() {
    let out = run("module t;\n\
           string d[] = '{\"world\",\"x\"};\n\
           initial begin d[0] = \"hello\"; $display(\"[%s] %0d\", d[0], d[0][0]); end\n\
         endmodule\n");
    assert_eq!(out, "[hello] 104\n");
}

#[test]
fn non_string_dynamic_element_part_select_write_still_works() {
    // The guard must be keyed on the element having no bit-addressable storage
    // (string or real), not on "dyn element" — a packed element must still work.
    let out = run("module t;\n\
           logic [15:0] d[];\n\
           initial begin d = new[2]; d[0] = 16'h0000; d[0][7:0] = 8'hAB; $display(\"%h\", d[0]); end\n\
         endmodule\n");
    assert_eq!(out, "00ab\n");
}

#[test]
fn part_select_write_variants_are_all_loud() {
    // `-:` and a RUNTIME offset. The runtime-offset form was a distinct pre-existing
    // silent path: `d[0][j +: 4] = 4'hF` with j=0 silently produced "worle".
    loud(
        "module t;\n\
           string d[] = '{\"world\",\"x\"};\n\
           initial begin d[0][7 -: 4] = 4'hF; $display(\"[%s]\", d[0]); end\n\
         endmodule\n",
        "part-select write into a string / real array element",
    );
    loud(
        "module t;\n\
           string d[] = '{\"world\",\"x\"};\n\
           int j;\n\
           initial begin j = 0; d[0][j +: 4] = 4'hF; $display(\"[%s]\", d[0]); end\n\
         endmodule\n",
        "part-select write into a string / real array element",
    );
}

#[test]
fn whole_element_inside_a_concat_lvalue_is_loud() {
    // Was SILENTLY emptying the element (`[] len=0`) while the fixed and scalar twins
    // were loud. The `lower_lvalue` funnel guards keyed on `is_string_net` could not
    // see a dyn element's `DynArray` net; all three guards now share
    // `is_non_bit_addressable_target`.
    loud(
        "module t;\n\
           string d[] = '{\"world\",\"x\"};\n\
           logic [3:0] x;\n\
           initial begin {d[0], x} = 8'hAB; $display(\"[%s]\", d[0]); end\n\
         endmodule\n",
        "inside a concatenation lvalue",
    );
}

#[test]
fn real_dynamic_array_element_part_select_write_is_loud() {
    // The `real_elem_dyn_nets` twin of the string case — a real element has no
    // bit-addressable storage either, and this was a silent no-op (3.5 unchanged).
    loud(
        "module t;\n\
           real r[];\n\
           initial begin r = new[2]; r[0] = 3.5; r[0][3:0] = 4'hF; $display(\"%0f\", r[0]); end\n\
         endmodule\n",
        "part-select write into a string / real array element",
    );
}

#[test]
fn non_string_concat_lvalue_still_works() {
    // The concat guard must stay keyed on string-valued storage, not on "dyn element".
    let out = run("module t;\n\
           int d[];\n\
           logic [3:0] x;\n\
           initial begin d = new[2]; {d[0], x} = 8'hAB; $display(\"%0d %h\", d[0], x); end\n\
         endmodule\n");
    assert_eq!(out, "10 b\n");
}

#[test]
fn real_element_inside_a_concat_lvalue_is_loud() {
    // The real twin on the CONCAT axis. Keying the funnel guards on a string-only
    // predicate left this silently overwriting the element with a bit pattern
    // (3.5 -> 10.0); all three guards now share one predicate.
    loud(
        "module t;\n\
           real r[];\n\
           logic [3:0] x;\n\
           initial begin r = new[2]; r[0] = 3.5; {r[0], x} = 8'hAB; $display(\"%0f\", r[0]); end\n\
         endmodule\n",
        "inside a concatenation lvalue",
    );
}

#[test]
fn single_element_concat_of_a_dyn_string_element_still_works() {
    // The nearest neighbour to the newly-loud multi-chunk case: a ONE-chunk concat is
    // the supported whole-element write and must not be caught by the funnel guard.
    let out = run("module t;\n\
           string d[] = '{\"world\",\"x\"};\n\
           initial begin {d[0]} = \"hi\"; $display(\"[%s] %0d\", d[0], d[0].len()); end\n\
         endmodule\n");
    assert_eq!(out, "[hi] 2\n");
}
