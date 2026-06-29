//! G3 (honest-loud batch): a TYPED loop-variable declaration in the `for`-init —
//! `for (int i = 0; …)` / `for (integer i = 0; …)` / `for (byte i = 0; …)` /
//! `for (logic [3:0] i = 0; …)`. Previously loud-rejected with an E2002 cascade.
//!
//! Parser-only (IR-0): `parse_for` parses the decl, gives the loop variable a
//! synthetic UNIQUE name (so it never aliases a same-named outer/module var under
//! v1's flat block-local namespace), rewrites cond/step/body references via
//! `rename_ident_in_stmt`, and wraps the `For` in an unlabeled block carrying the
//! decl (hoisted to a module net by `hoist_block_local_nets`). Same idiom as
//! `parse_foreach`. Oracle = iverilog 13.0; every output below is iverilog-pinned.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_tfi_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    )
}

#[test]
fn typed_for_int() {
    // 0+1+2+3 = 6.
    let (out, err, code) = run("module top;\n\
         initial begin\n\
           int sum = 0;\n\
           for (int i = 0; i < 4; i = i + 1) sum = sum + i;\n\
           $display(\"%0d\", sum);\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("6"), "got:\n{out}");
}

#[test]
fn typed_for_integer() {
    let (out, err, code) = run("module top;\n\
         initial begin\n\
           int sum = 0;\n\
           for (integer i = 0; i < 4; i = i + 1) sum = sum + i;\n\
           $display(\"%0d\", sum);\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("6"), "got:\n{out}");
}

#[test]
fn typed_for_byte() {
    let (out, err, code) = run("module top;\n\
         initial begin\n\
           int sum = 0;\n\
           for (byte i = 0; i < 4; i = i + 1) sum = sum + i;\n\
           $display(\"%0d\", sum);\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("6"), "got:\n{out}");
}

#[test]
fn typed_for_logic_vector() {
    // logic [3:0] loop var still iterates 0..3 → sum 6.
    let (out, err, code) = run("module top;\n\
         initial begin\n\
           int sum = 0;\n\
           for (logic [3:0] i = 0; i < 4; i = i + 1) sum = sum + i;\n\
           $display(\"%0d\", sum);\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("6"), "got:\n{out}");
}

#[test]
fn typed_for_two_loops_same_name() {
    // Two sequential loops in the SAME block both declare `i`. Each loop var is a
    // distinct synthetic net, so they don't interfere: a = 0+1+2+3 = 6, b = 0+1+2 = 3.
    let (out, err, code) = run("module top;\n\
         initial begin\n\
           int a = 0;\n\
           int b = 0;\n\
           for (int i = 0; i < 4; i = i + 1) a = a + i;\n\
           for (int i = 0; i < 3; i = i + 1) b = b + i;\n\
           $display(\"%0d %0d\", a, b);\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("6 3"), "got:\n{out}");
}

#[test]
fn typed_for_loopvar_does_not_alias_module_net() {
    // A module net `i = 99` and a typed for-init `int i`. The loop var must NOT
    // alias / leak into the module net — after the loop the module `i` is still 99.
    let (out, err, code) = run("module top;\n\
         integer i = 99;\n\
         int sum = 0;\n\
         initial begin\n\
           for (int i = 0; i < 4; i = i + 1) sum = sum + i;\n\
           $display(\"sum=%0d outer_i=%0d\", sum, i);\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("sum=6 outer_i=99"), "got:\n{out}");
}

#[test]
fn typed_for_block_local_does_not_leak() {
    // After the loop, `i` refers to the MODULE net (the block-local for-init `i`
    // is gone). 0+1+2 = 3 in the loop, then `a + i` adds the module net 7 → 10.
    let (out, err, code) = run("module top;\n\
         integer i = 7;\n\
         int a = 0;\n\
         initial begin\n\
           for (int i = 0; i < 3; i = i + 1) a = a + i;\n\
           a = a + i;\n\
           $display(\"%0d\", a);\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("10"), "got:\n{out}");
}

#[test]
fn typed_for_nested_distinct_names() {
    // Nested typed loops, distinct names. sum over i*3+j for i,j in 0..2 = 36.
    let (out, err, code) = run("module top;\n\
         initial begin\n\
           int s = 0;\n\
           for (int i = 0; i < 3; i = i + 1)\n\
             for (int j = 0; j < 3; j = j + 1)\n\
               s = s + (i * 3 + j);\n\
           $display(\"%0d\", s);\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("36"), "got:\n{out}");
}

#[test]
fn typed_for_nested_same_name_shadows() {
    // Nested typed loops reusing the SAME name `i`. The inner `i` shadows the
    // outer: inner runs 0..1 three times, summing the INNER i → 3*(0+1) = 3.
    let (out, err, code) = run("module top;\n\
         initial begin\n\
           int s = 0;\n\
           for (int i = 0; i < 3; i = i + 1)\n\
             for (int i = 0; i < 2; i = i + 1)\n\
               s = s + i;\n\
           $display(\"%0d\", s);\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("3"), "got:\n{out}");
}

#[test]
fn typed_for_signed_byte_negative() {
    // A signed `byte` iterating -3..-1 → -3 + -2 + -1 = -6.
    let (out, err, code) = run("module top;\n\
         initial begin\n\
           int s = 0;\n\
           for (byte i = -3; i < 0; i = i + 1) s = s + i;\n\
           $display(\"%0d\", s);\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("-6"), "got:\n{out}");
}

#[test]
fn typed_for_no_initializer_is_loud() {
    // `for (int i; …)` (no `=` seed) is rejected by both vita and iverilog —
    // correct-or-loud: a non-zero exit with no fabricated output.
    let (out, _err, code) = run("module top;\n\
         initial begin\n\
           int s = 0;\n\
           for (int i; i < 4; i = i + 1) s = s + i;\n\
           $display(\"%0d\", s);\n\
         end\n\
         endmodule\n");
    assert_ne!(
        code,
        Some(0),
        "should reject a typed for-init with no initializer"
    );
    assert!(!out.contains("6"), "must not produce a result:\n{out}");
}
