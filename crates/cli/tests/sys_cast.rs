//! v9 Medium-bundle rank 6: `$cast(dst, src)` (IEEE 1800-2017 §6.24.2), function
//! AND task form. iverilog 13.0 does NOT support `$cast` ("not defined by any
//! module") → NO oracle: hand-IEEE. In this class-free integral subset a cast is
//! a checked assignment that ALWAYS succeeds (returns 1; failure=0 needs class /
//! strict-enum range checks vita does not model). The destination is resized to
//! its declared width exactly like any blocking assignment.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_cast_{}_{n}", std::process::id()));
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
        out.status.code(),
    )
}

#[test]
fn func_form_assigns_and_returns_one() {
    // ok = $cast(a, b): a := b (resized), ok := 1.
    let (out, _c) = run("module t;\n\
         integer a, ok; reg [7:0] b;\n\
         initial begin b = 8'hAB; ok = $cast(a, b); $display(\"R %0d %0d\", ok, a); end\n\
         endmodule\n");
    assert!(out.contains("R 1 171"), "func cast ok + value:\n{out}");
}

#[test]
fn task_form_assigns() {
    // $cast(a, src); — assigns, no status.
    let (out, _c) = run("module t;\n\
         integer a;\n\
         initial begin a = 0; $cast(a, 8'h7F); $display(\"T %0d\", a); end\n\
         endmodule\n");
    assert!(out.contains("T 127"), "task cast value:\n{out}");
}

#[test]
fn resize_truncates_and_extends() {
    // wide src → narrow dst truncates to the dst width; narrow src → wide dst
    // zero/sign-extends like a normal assignment.
    let (out, _c) = run("module t;\n\
         reg [3:0] n; integer w, ok1, ok2;\n\
         initial begin\n\
           ok1 = $cast(n, 32'h12345); $display(\"N %0d %0d\", ok1, n);\n\
           ok2 = $cast(w, 4'hF);      $display(\"W %0d %0d\", ok2, w);\n\
         end\n\
         endmodule\n");
    assert!(
        out.contains("N 1 5"),
        "truncate 0x12345 → low nibble 5:\n{out}"
    );
    assert!(out.contains("W 1 15"), "extend 4'hF → 15:\n{out}");
}

#[test]
fn nondirect_func_placement_is_loud() {
    // the func form writes the dst ref: direct-rhs only, else loud E3009.
    let (out, code) = run("module t;\n\
         integer a, r; reg [7:0] b;\n\
         initial begin b = 1; r = 1 + $cast(a, b); end\n\
         endmodule\n");
    assert!(
        out.contains("VITA-E3009") || code == Some(1),
        "nested $cast must be loud: {out} code={code:?}"
    );
}

#[test]
fn non_plain_destination_is_loud() {
    // the destination must be a plain whole-net variable (a memory element or a
    // select is loud in this subset, never a silent mis-write).
    let (_o, code) = run("module t;\n\
         reg [7:0] m [0:3]; integer ok;\n\
         initial begin ok = $cast(m[0], 8'h5); end\n\
         endmodule\n");
    assert_eq!(code, Some(1), "memory-element dst must be loud");
}

#[test]
fn task_form_has_same_loud_guards_as_func() {
    // review H1: the TASK form must NOT silently drop on a non-plain dest or wrong
    // arity (the func form is loud) — both are E3009, never a silent no-op.
    let (_o, c1) = run("module t;\n\
         reg [7:0] m [0:3];\n\
         initial begin $cast(m[0], 8'h5); end\n\
         endmodule\n");
    assert_eq!(c1, Some(1), "task-form memory-element dst must be loud");
    let (_o, c2) = run("module t;\n\
         integer a;\n\
         initial begin $cast(a); end\n\
         endmodule\n");
    assert_eq!(c2, Some(1), "task-form wrong arity must be loud");
}

#[test]
fn arity_error_is_loud() {
    let (_o, code) = run("module t;\n\
         integer a, ok;\n\
         initial begin ok = $cast(a); end\n\
         endmodule\n");
    assert_eq!(code, Some(1), "wrong arity must be loud");
}
