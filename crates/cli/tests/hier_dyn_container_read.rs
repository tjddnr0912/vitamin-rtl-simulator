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

// ── what stays loud ──────────────────────────────────────────────────────────

#[test]
fn a_hierarchical_associative_array_read_stays_loud() {
    // An assoc array is KEYED, not positioned, so a bare index is a different operation
    // — admitting it under the positional arm would read the wrong thing.
    assert!(loud(
        "module sub; int a[int]; initial begin a[5]=7; end endmodule\n\
         module m; sub u();\n\
         initial begin #1; $display(\"%0d\", u.a[5]); $finish; end\n\
         endmodule\n"
    ));
}

#[test]
fn a_hierarchical_multi_dim_element_stays_loud() {
    // The row-major flatten is applied at lowering against dimensions this resolution
    // pass does not carry; reading the first index as a flat one would be a silently
    // wrong element. iverilog runs it — a recorded gap, not a wrong answer.
    assert!(loud(
        "module sub; string s[2][2]; initial begin s[0][0]=\"aa\"; end endmodule\n\
         module m; sub u();\n\
         initial begin #1; $display(\"%s\", u.s[0][0]); $finish; end\n\
         endmodule\n"
    ));
}

#[test]
fn a_hierarchical_element_write_stays_loud() {
    // The write is a separate deferred machine. Loud before this slice and loud after —
    // an asymmetry with the read, but a gap rather than a regression.
    assert!(loud(
        "module sub; string s[2]; endmodule\n\
         module m; sub u();\n\
         initial begin #1; u.s[0]=\"zz\"; $display(\"%s\", u.s[0]); $finish; end\n\
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
