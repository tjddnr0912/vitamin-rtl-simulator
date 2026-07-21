//! §4.5.183 (SILENT-WRONG fix): a PROCESS-BLOCK-LOCAL `string s[] = '{...}`
//! dynamic-array initializer was silently dropped, leaving the array EMPTY, while
//! the byte-identical MODULE-SCOPE declaration worked. Root cause: the block-local
//! decl-init collector classified any `string` as `scalar_string` (it checks only
//! the base range/packed dims, not the per-name unpacked dims) and then gated the
//! push on `name.unpacked.is_empty()`, so a dimensioned string (`s[]`) fell through
//! and its `'{...}` was never collected into the t0 pre-sweep. The module-scope
//! collector (`collect_var_init_drivers`) already had an `is_dyn_str_init` special
//! case, so only the block-local path was affected.
//!
//! Fix: the block-local path now also pushes a string DYNAMIC array (`name.unpacked
//! == [Dim::Dyn]`) with a `'{...}` init, so it rides the same flush expansion
//! (`new[N]` + element writes) as the module-scope path. Other string dims (fixed
//! `s[2]`, multi-dim, non-pattern init) were loud-rejected at the decl and are
//! unaffected; a `{...}` unpacked-concat on a string array also stays loud
//! (§4.5.182 keeps string-element concat loud, not silent-empty).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Returns (first `K=` line, process_success).
fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_blsdi_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let key = text
        .lines()
        .find(|l| l.starts_with("K="))
        .unwrap_or_default()
        .trim()
        .to_owned();
    (key, out.status.success())
}

fn loud(src: &str) -> bool {
    !run(src).1
}

// ── the fix: block-local string dyn-array `'{…}` now populates the array ─────

#[test]
fn block_local_string_dyn_basic() {
    // Previously SZ=0 with empty elements; now the three elements are present.
    let (k, ok) = run("module top; initial begin\n\
         string s[] = '{\"foo\",\"bar\",\"baz\"};\n\
         $display(\"K=%s %s %s %0d\", s[0],s[1],s[2], s.size()); $finish; end endmodule");
    assert!(ok && k == "K=foo bar baz 3", "got ({k}, {ok})");
}

#[test]
fn block_local_string_dyn_named_block() {
    let (k, ok) = run("module top; initial begin : blk\n\
         string s[] = '{\"x\",\"y\"};\n\
         $display(\"K=%s %s\", s[0],s[1]); $finish; end endmodule");
    assert!(ok && k == "K=x y", "got ({k}, {ok})");
}

#[test]
fn block_local_string_dyn_single() {
    let (k, ok) = run("module top; initial begin\n\
         string s[] = '{\"only\"};\n\
         $display(\"K=%s %0d\", s[0], s.size()); $finish; end endmodule");
    assert!(ok && k == "K=only 1", "got ({k}, {ok})");
}

#[test]
fn block_local_string_dyn_then_mutate() {
    // A block-local string dyn array is a normal handle: new[](copy)/element write work.
    let (k, ok) = run("module top; initial begin\n\
         string s[] = '{\"a\",\"b\"};\n\
         s = new[3](s); s[2] = \"c\";\n\
         $display(\"K=%s %s %s\", s[0],s[1],s[2]); $finish; end endmodule");
    assert!(ok && k == "K=a b c", "got ({k}, {ok})");
}

#[test]
fn block_local_string_dyn_in_always() {
    let (k, ok) = run("module top; logic clk=0;\n\
         initial begin #1 clk=1; #1 $finish; end\n\
         always @(posedge clk) begin\n\
         string s[] = '{\"p\",\"q\"};\n\
         $display(\"K=%s %s\", s[0],s[1]); end endmodule");
    assert!(ok && k == "K=p q", "got ({k}, {ok})");
}

// ── unaffected: module-scope + sibling shapes still work ─────────────────────

#[test]
fn module_scope_string_dyn_unaffected() {
    let (k, ok) = run("module top;\n\
         string s[] = '{\"a\",\"b\",\"c\"};\n\
         initial begin $display(\"K=%s %s %s\", s[0],s[1],s[2]); $finish; end endmodule");
    assert!(
        ok && k == "K=a b c",
        "module-scope must stay working; got ({k}, {ok})"
    );
}

#[test]
fn block_local_scalar_string_unaffected() {
    let (k, ok) = run("module top; initial begin\n\
         string s = \"hello\";\n\
         $display(\"K=%s\", s); $finish; end endmodule");
    assert!(
        ok && k == "K=hello",
        "scalar string must stay working; got ({k}, {ok})"
    );
}

#[test]
fn block_local_int_dyn_unaffected() {
    let (k, ok) = run("module top; initial begin\n\
         int d[] = '{4,5,6};\n\
         $display(\"K=%0d %0d %0d\", d[0],d[1],d[2]); $finish; end endmodule");
    assert!(
        ok && k == "K=4 5 6",
        "int dyn must stay working; got ({k}, {ok})"
    );
}

// ── correct-or-loud boundary: string `{…}` concat stays loud (§4.5.182) ──────

#[test]
fn block_local_string_concat_stays_loud() {
    // A `{…}` unpacked concat on a STRING array stays loud (not silent-empty):
    // §4.5.182 excludes string-element concat at the handle gate.
    assert!(loud(
        "module top; initial begin\n\
         string s[] = {\"a\",\"b\"};\n\
         $display(\"K=%s\", s[0]); $finish; end endmodule"
    ));
}
