//! T1-6: a hierarchical read of a dynamic-container ELEMENT — `u.s[0]` on a string
//! array, and the same shape on `int d[]` / `int q[$]`.
//!
//! This was never string-specific. The deferred hierarchical-select resolution rejected
//! every dynamic-storage handle on the reasoning that "a dyn element read routes through
//! `dyn_select_read` at lowering, on a 1-seg base — never a hierarchical ref". That is
//! true of the LOWERING path, but the resolved element read is just a word-indexed
//! `Signal`, and the engine does not care how the name was reached.
//!
//! Restricted to the shape whose position IS its index: exactly one index, on a
//! `DynArray` or `Queue`. An associative array is keyed rather than positioned, and a
//! multi-index access on a routed multi-dim array needs the row-major flatten against
//! dimensions this pass does not carry — reading the first index as a flat one would
//! select a silently wrong element. Both stay loud, as does the hierarchical WRITE
//! (a separate deferred machine).

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn compile(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_hdcr_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    (
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter(|l| !l.starts_with("simulation ended"))
            .fold(String::new(), |mut acc, l| {
                acc.push_str(l);
                acc.push('\n');
                acc
            }),
        out.status.success(),
    )
}

fn run(src: &str) -> String {
    let (out, ok) = compile(src);
    assert!(ok, "expected success for:\n{src}");
    out
}

fn loud(src: &str) -> bool {
    !compile(src).1
}

#[test]
fn hierarchical_string_array_element() {
    // iverilog: aa bb. The routed array is registered under a MANGLED net name (so the
    // declared name stays free for block-locals), which the symbol table alone cannot
    // resolve — the resolution consults the same side map the local resolver does.
    assert_eq!(
        run(
            "module sub; string s[2]; initial begin s[0]=\"aa\"; s[1]=\"bb\"; end endmodule\n\
             module m; sub u();\n\
             initial begin #1; $display(\"%s %s\", u.s[0], u.s[1]); $finish; end\n\
             endmodule\n"
        ),
        "aa bb\n"
    );
}

#[test]
fn hierarchical_string_array_runtime_index() {
    // iverilog: bb
    assert_eq!(
        run(
            "module sub; string s[2]; initial begin s[0]=\"aa\"; s[1]=\"bb\"; end endmodule\n\
             module m; sub u(); int k;\n\
             initial begin #1; k=1; $display(\"%s\", u.s[k]); $finish; end\n\
             endmodule\n"
        ),
        "bb\n"
    );
}

#[test]
fn hierarchical_dynamic_array_and_queue() {
    // Not string-specific — `int d[]` and `int q[$]` were loud for the same reason.
    // iverilog: 7 9 and 5 6.
    assert_eq!(
        run(
            "module sub; int d[]; initial begin d=new[2]; d[0]=7; d[1]=9; end endmodule\n\
             module m; sub u();\n\
             initial begin #1; $display(\"%0d %0d\", u.d[0], u.d[1]); $finish; end\n\
             endmodule\n"
        ),
        "7 9\n"
    );
    assert_eq!(
        run(
            "module sub; int q[$]; initial begin q.push_back(5); q.push_back(6); end endmodule\n\
             module m; sub u();\n\
             initial begin #1; $display(\"%0d %0d\", u.q[0], u.q[1]); $finish; end\n\
             endmodule\n"
        ),
        "5 6\n"
    );
}

#[test]
fn a_parent_array_of_the_same_name_is_not_confused_with_the_child() {
    // The side-map fallback is consulted with the same commit-to-scope walk as the
    // symbol table and only AFTER it, so a same-named array in the parent keeps its own
    // storage. iverilog: parent child.
    assert_eq!(
        run(
            "module sub; string s[2]; initial begin s[0]=\"child\"; end endmodule\n\
             module m; sub u(); string s[2];\n\
             initial begin s[0]=\"parent\"; #1; $display(\"%s %s\", s[0], u.s[0]); $finish; end\n\
             endmodule\n"
        ),
        "parent child\n"
    );
}

#[test]
fn each_instance_resolves_to_its_own_array() {
    // iverilog: one two
    assert_eq!(
        run("module sub#(parameter int ID=0); string s[2];\n\
               initial begin s[0]=(ID==1)?\"one\":\"two\"; end endmodule\n\
             module m; sub #(1) u1(); sub #(2) u2();\n\
             initial begin #1; $display(\"%s %s\", u1.s[0], u2.s[0]); $finish; end\n\
             endmodule\n"),
        "one two\n"
    );
}

#[test]
fn hierarchical_multi_dim_element() {
    // T1-7. The declared→flat map is applied here through `flatten_word_eids`, the
    // pre-lowered-eid twin of the `flatten_word` the LOCAL funnel uses — same arithmetic,
    // so `u.s[i][j]` and a local `s[i][j]` cannot select different elements. iverilog: aa dd.
    assert_eq!(
        run(
            "module sub; string s[2][2]; initial begin s[0][0]=\"aa\"; s[1][1]=\"dd\"; end endmodule\n\
             module m; sub u();\n\
             initial begin #1; $display(\"%s %s\", u.s[0][0], u.s[1][1]); $finish; end\n\
             endmodule\n"
        ),
        "aa dd\n"
    );
}

#[test]
fn hierarchical_element_write() {
    // T1-8. The write is a separate deferred pass over a separate sentinel space, and the
    // hazard of admitting it is that it addresses a DIFFERENT element than the read of the
    // same source text — so both call ONE `hier_dyn_container_word`. iverilog: zz yy.
    assert_eq!(
        run("module sub; string s[2]; endmodule\n\
             module m; sub u();\n\
             initial begin #1; u.s[0]=\"zz\"; u.s[1]=\"yy\"; $display(\"%s %s\", u.s[0], u.s[1]); $finish; end\n\
             endmodule\n"),
        "zz yy\n"
    );
    // Not string-specific — `int d[]` / `int q[$]` were loud for the same reason.
    // iverilog: 7 9 and 5 6.
    assert_eq!(
        run("module sub; int d[]; initial d=new[2]; endmodule\n\
             module m; sub u();\n\
             initial begin #1; u.d[0]=7; u.d[1]=9; $display(\"%0d %0d\", u.d[0], u.d[1]); $finish; end\n\
             endmodule\n"),
        "7 9\n"
    );
    assert_eq!(
        run(
            "module sub; int q[$]; initial begin q.push_back(0); q.push_back(0); end endmodule\n\
             module m; sub u();\n\
             initial begin #1; u.q[0]=5; u.q[1]=6; $display(\"%0d %0d\", u.q[0], u.q[1]); $finish; end\n\
             endmodule\n"
        ),
        "5 6\n"
    );
}

#[test]
fn hierarchical_multi_dim_and_non_zero_base_write() {
    // T1-7/8 together: the write goes through the same declared→flat map as the read, so
    // a non-zero base and a nested index behave hierarchically exactly as they do locally.
    // iverilog: aa dd, then lo mid.
    assert_eq!(
        run("module sub; string s[2][2]; endmodule\n\
             module m; sub u();\n\
             initial begin #1; u.s[0][0]=\"aa\"; u.s[1][1]=\"dd\";\n\
               $display(\"%s %s\", u.s[0][0], u.s[1][1]); $finish; end\n\
             endmodule\n"),
        "aa dd\n"
    );
    assert_eq!(
        run("module sub; string s[1:3]; endmodule\n\
             module m; sub u(); int k;\n\
             initial begin #1; k=2; u.s[k]=\"mid\"; u.s[1]=\"lo\";\n\
               $display(\"%s %s\", u.s[1], u.s[2]); $finish; end\n\
             endmodule\n"),
        "lo mid\n"
    );
}

#[test]
fn hierarchical_associative_array_read_and_write() {
    // T1-10. An assoc array is KEYED rather than positioned, but that distinction is
    // settled DOWNSTREAM by the net — `resolve_lvalue_offsets` re-reads the same word EID
    // as an `AssocKey` when `is_assoc(net)`, and the element read does the same — so it
    // takes the identical one-index spelling.
    //
    // No oracle: iverilog 13.0 cannot parse `int a[int]` at all. The pin is the
    // vita-internal equivalence — the hierarchical view must equal the child's own view
    // of the same array, before AND after a hierarchical write.
    assert_eq!(
        run("module sub; int a[int]; int sm[string];\n\
               initial begin a[5]=7; a[-3]=9; sm[\"k\"]=42; end\n\
               task show; $display(\"%0d %0d %0d\", a[5], a[-3], sm[\"k\"]); endtask\n\
             endmodule\n\
             module m; sub u();\n\
             initial begin #1; $display(\"%0d %0d %0d\", u.a[5], u.a[-3], u.sm[\"k\"]); u.show();\n\
               u.a[5]=77; u.sm[\"k\"]=43; u.show(); $finish; end\n\
             endmodule\n"),
        "7 9 42\n7 9 42\n77 9 43\n"
    );
}

#[test]
fn a_missing_associative_key_is_not_invented_by_the_hierarchical_path() {
    // The keyed lookup must MISS the same way locally and hierarchically — a warn plus X,
    // not a silently defaulted 0 that a positional read of an empty slot could produce.
    let (out, ok) = compile(
        "module sub; int a[int]; initial a[5]=7; endmodule\n\
         module m; sub u();\n\
         initial begin #1; $display(\"%0d\", u.a[99]); $finish; end\n\
         endmodule\n",
    );
    assert!(ok, "expected a warn, not a reject");
    assert_eq!(out, "x\n");
}

// ── boundaries ───────────────────────────────────────────────────────────────

#[test]
fn a_partial_hierarchical_index_of_a_multi_dim_array_stays_loud() {
    // A single index on a 2-D routed array selects a whole row, which has no value
    // surface (iverilog rejects the source). The addressing rule DECLINES on an index
    // count that does not match the dimension count, so the flat container never takes
    // the row number as an element number — loud on both the read and the write.
    assert!(loud(
        "module sub; string s[2][2]; initial begin s[0][0]=\"aa\"; end endmodule\n\
         module m; sub u();\n\
         initial begin #1; $display(\"%s\", u.s[0]); $finish; end\n\
         endmodule\n"
    ));
    assert!(loud(
        "module sub; string s[2][2]; endmodule\n\
         module m; sub u();\n\
         initial begin #1; u.s[0]=\"aa\"; $display(\"done\"); $finish; end\n\
         endmodule\n"
    ));
}

#[test]
fn an_out_of_range_hierarchical_index_warns_and_reads_empty() {
    // Non-silent (a W4020 warn on stderr) and byte-identical to iverilog, which also
    // renders the out-of-range element as a single space.
    let (out, ok) = compile(
        "module sub; string s[2]; initial begin s[0]=\"aa\"; end endmodule\n\
         module m; sub u();\n\
         initial begin #1; $display(\"[%s]\", u.s[9]); $finish; end\n\
         endmodule\n",
    );
    assert!(ok, "expected a warn, not a reject");
    assert_eq!(out, "[ ]\n");
}
