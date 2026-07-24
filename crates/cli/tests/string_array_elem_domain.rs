//! A string-ARRAY ELEMENT (`sa[i]`) is a string-domain value (IEEE §7.4: the
//! declared element type is `string`) — unlike `str[i]` on a SCALAR string, which
//! is the §6.16.2 8-bit BYTE select.
//!
//! Before this slice the AST-level string-domain classifier (`expr_is_string_ast`)
//! had no arm for an indexed expression, so every context gated on it —
//! concatenation, replication and the relational compares — took the PACKED path on
//! an element and silently dropped its bytes. Live iverilog differential (a fixed
//! `string s[N]` is an iverilog-supported construct, so this whole family has a real
//! oracle):
//!
//! ```text
//!   {s[0],s[1]}          vita ""      iverilog "abcb"
//!   {2{s[0]}}            vita ""      iverilog "abcabc"
//!   {s[0],"-",s[1]}      vita "b"     iverilog "abc-b"
//!   s[0] < s[1]          vita 0       iverilog 1
//! ```
//!
//! Each `*_matches_scalar` test also pins the vita-INTERNAL equivalence that is the
//! real teeth: an element holding value V must render byte-identically to a scalar
//! `string` holding V, in every context.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_saed_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    let so = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        out.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let mut s = String::new();
    for l in so.lines().filter(|l| !l.starts_with("simulation ended")) {
        s.push_str(l);
        s.push('\n');
    }
    s
}

/// A body run twice: once over a FIXED string array, once over the equivalent
/// scalar `string` variables. Both must print the same thing (the vita-internal
/// equivalence check), and that thing must be `expect` (the iverilog oracle).
fn both(decl_body: &str, scalar_body: &str, expect: &str) {
    let fixed = run(&format!(
        "module t;\n  string s[2];\n  string r;\n  initial begin\n    s[0]=\"abc\"; s[1]=\"b\";\n{decl_body}  end\nendmodule\n"
    ));
    let scalar = run(&format!(
        "module t;\n  string s0, s1;\n  string r;\n  initial begin\n    s0=\"abc\"; s1=\"b\";\n{scalar_body}  end\nendmodule\n"
    ));
    assert_eq!(fixed, expect, "fixed-array form");
    assert_eq!(scalar, expect, "scalar form (internal equivalence)");
}

#[test]
fn concat_two_elements_matches_scalar() {
    both(
        "    $display(\"[%s]\", {s[0], s[1]});\n",
        "    $display(\"[%s]\", {s0, s1});\n",
        "[abcb]\n",
    );
}

#[test]
fn concat_element_and_literal_matches_scalar() {
    both(
        "    $display(\"[%s]\", {s[0], \"!\"});\n",
        "    $display(\"[%s]\", {s0, \"!\"});\n",
        "[abc!]\n",
    );
}

#[test]
fn concat_assign_three_parts_matches_scalar() {
    both(
        "    r = {s[0], \"-\", s[1]}; $display(\"[%s] %0d\", r, r.len());\n",
        "    r = {s0, \"-\", s1}; $display(\"[%s] %0d\", r, r.len());\n",
        "[abc-b] 5\n",
    );
}

#[test]
fn replicate_element_matches_scalar() {
    both(
        "    $display(\"[%s]\", {2{s[0]}});\n",
        "    $display(\"[%s]\", {2{s0}});\n",
        "[abcabc]\n",
    );
}

#[test]
fn relational_lt_gt_le_matches_scalar() {
    // "abc" < "b" lexicographically (a<b) — a PACKED compare MSB-extends the
    // shorter operand and gets the opposite answer.
    both(
        "    $display(\"%0d %0d %0d\", s[0] < s[1], s[0] > s[1], s[0] <= s[1]);\n",
        "    $display(\"%0d %0d %0d\", s0 < s1, s0 > s1, s0 <= s1);\n",
        "1 0 1\n",
    );
}

#[test]
fn equality_unequal_lengths_matches_scalar() {
    both(
        "    $display(\"%0d %0d %0d\", s[0]==\"abc\", s[0]==\"ab\", s[0]!=s[1]);\n",
        "    $display(\"%0d %0d %0d\", s0==\"abc\", s0==\"ab\", s0!=s1);\n",
        "1 0 1\n",
    );
}

#[test]
fn nested_concat_matches_scalar() {
    both(
        "    r = {{s[0], \"-\"}, s[1]}; $display(\"[%s]\", r);\n",
        "    r = {{s0, \"-\"}, s1}; $display(\"[%s]\", r);\n",
        "[abc-b]\n",
    );
}

#[test]
fn concat_in_task_arg_and_sformatf() {
    both(
        "    $display(\"[%s]\", $sformatf(\"<%s>\", {s[0], s[1]}));\n",
        "    $display(\"[%s]\", $sformatf(\"<%s>\", {s0, s1}));\n",
        "[<abcb>]\n",
    );
}

// ── dynamic string array (`string s[]`) — same element-domain rule ─────────────

#[test]
fn dyn_array_concat_and_relational() {
    let out = run("module t;\n\
           string d[] = '{\"abc\", \"b\"};\n\
           int i;\n\
           initial begin\n\
             i = 0;\n\
             $display(\"[%s] [%s] %0d\", {d[0], d[1]}, {d[i], \"!\"}, d[0] < d[1]);\n\
           end\n\
         endmodule\n");
    assert_eq!(out, "[abcb] [abc!] 1\n");
}

#[test]
fn dyn_array_replicate_runtime_index() {
    let out = run("module t;\n\
           string d[] = '{\"ab\", \"cd\"};\n\
           int i;\n\
           initial begin i = 1; $display(\"[%s]\", {2{d[i]}}); end\n\
         endmodule\n");
    assert_eq!(out, "[cdcd]\n");
}

// ── boundaries that must NOT change ───────────────────────────────────────────

#[test]
fn scalar_string_byte_select_stays_packed() {
    // `a[i]` on a SCALAR string is the §6.16.2 BYTE select (8-bit integral), not a
    // string element — the concat of two of them is a 2-byte PACKED value. iverilog
    // agrees (`byte=97 cat=[ab]`); the new element arm must not capture this form.
    let out = run("module t;\n\
           string a;\n\
           initial begin a = \"abc\"; $display(\"%0d [%s]\", a[0], {a[0], a[1]}); end\n\
         endmodule\n");
    assert_eq!(out, "97 [ab]\n");
}

#[test]
fn non_string_array_concat_stays_packed() {
    // An ordinary unpacked array of vectors keeps the bit-concat meaning.
    let out = run("module t;\n\
           logic [3:0] m [2];\n\
           initial begin m[0] = 4'ha; m[1] = 4'h5; $display(\"%h\", {m[0], m[1]}); end\n\
         endmodule\n");
    assert_eq!(out, "a5\n");
}

#[test]
fn non_string_dyn_array_concat_stays_packed() {
    // The DYNAMIC clause of the classifier keyed off `string_elem_dyn_nets`: an
    // `int d[]` is a DynArray net too, but its elements are NOT strings, so the
    // concat must stay a bit-concat and the compare numeric.
    let out = run("module t;\n\
           int d[];\n\
           initial begin d = new[2]; d[0] = 8'h41; d[1] = 8'h42;\n\
             $display(\"%h %0d\", {d[0], d[1]}, d[0] > d[1]); end\n\
         endmodule\n");
    assert_eq!(out, "0000004100000042 0\n");
}

#[test]
fn inlined_dyn_formal_shadowing_a_string_array_stays_numeric() {
    // A dyn-array formal SHADOWS an outer same-named net inside an R2-inlined body
    // (`dyn_handle_read` consults the `dyn_subst` alias first). The classifier must
    // use that SAME resolver: resolving with a plain `lookup_net_scoped` made the
    // module-level `string b[]` win, so this inlined `int b[]` formal compared as
    // TEXT — 256 < 255 answered 1 where iverilog (and the numeric path) say 0.
    let out = run("module t;\n\
           string b[];\n\
           int    a[];\n\
           task show(input int b[]);\n\
             $display(\"lt=%0d\", b[0] < b[1]);\n\
           endtask\n\
           initial begin b = new[1]; b[0] = \"z\";\n\
             a = new[2]; a[0] = 256; a[1] = 255; show(a); end\n\
         endmodule\n");
    assert_eq!(out, "lt=0\n");
}

#[test]
fn block_local_string_array_element_concat() {
    // A string array declared inside a named procedural block resolves through the
    // same outward scope walk; the element keeps its string domain there too.
    let out = run("module t;\n\
           initial begin : blk\n\
             string s[2];\n\
             s[0] = \"abc\"; s[1] = \"b\";\n\
             $display(\"[%s] %0d\", {s[0], \"!\"}, s[0] < s[1]);\n\
           end\n\
         endmodule\n");
    assert_eq!(out, "[abc!] 1\n");
}

#[test]
fn element_as_concat_target_round_trips() {
    // The element on the WRITE side of a string concat (not just as a part).
    let out = run("module t;\n\
           string s[2];\n\
           initial begin s[0] = \"abc\"; s[1] = \"b\";\n\
             s[0] = {s[0], \"-\", s[1]};\n\
             $display(\"[%s] %0d\", s[0], s[0].len()); end\n\
         endmodule\n");
    assert_eq!(out, "[abc-b] 5\n");
}

#[test]
fn unassigned_element_concats_as_empty() {
    // An unwritten element is the empty string, so it contributes nothing — and
    // must not contribute a stray NUL byte either.
    let out = run("module t;\n\
           string s[2];\n\
           string r;\n\
           initial begin s[0] = \"ab\";\n\
             r = {s[0], s[1], \"!\"};\n\
             $display(\"[%s] %0d\", r, r.len()); end\n\
         endmodule\n");
    assert_eq!(out, "[ab!] 3\n");
}

#[test]
fn runtime_index_on_fixed_string_array_stays_loud() {
    // A FIXED string array still requires a constant index (deeper follow-on); the
    // new string-domain arm must not silently route a runtime index anywhere.
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_saed_loud_{}_{n}.sv", std::process::id()));
    std::fs::write(
        &path,
        "module t;\n\
           string s[2];\n\
           int i;\n\
           initial begin s[0]=\"a\"; i=0; $display(\"[%s]\", {s[i], \"!\"}); end\n\
         endmodule\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    assert!(!out.status.success(), "expected a loud reject");
    let se = String::from_utf8_lossy(&out.stderr);
    assert!(
        se.contains("requires a constant index"),
        "unexpected diagnostic:\n{se}"
    );
}
