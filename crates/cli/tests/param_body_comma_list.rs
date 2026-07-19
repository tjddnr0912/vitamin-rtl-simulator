//! A body `parameter`/`localparam` COMMA-LIST (`localparam A = 1, B = 2;`) was
//! loud-rejected (E2002 at the comma) — the module-item path parsed exactly ONE
//! param and then demanded `;`. iverilog accepts it (a very common form). Now the
//! whole comma-list parses: the type prefix is read ONCE and shared across every
//! name (IEEE §6.20.1), so an unadorned continuation inherits the leading type/
//! width/signedness. The first name emits inline; the rest queue in the parser
//! and drain (in order, same scope) at the enclosing collection loop — covering
//! module / package / generate-block bodies, and the last-item-before-`end` edge
//! (the first name having already advanced the cursor onto the end keyword).
//! iverilog 13.0-pinned.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_pbcl_{}_{n}", std::process::id()));
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

/// A module body with `decl` then a `$display(fmt, args)`.
fn m(decl: &str, fmt: &str, args: &str) -> String {
    format!(
        "module t;\n  {decl}\n  initial begin $display(\"{fmt}\", {args}); $finish; end\nendmodule\n"
    )
}

#[test]
fn untyped_comma_list() {
    let (out, code) = run(&m(
        "localparam A = 1, B = 2, C = 3;",
        "%0d %0d %0d",
        "A, B, C",
    ));
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("1 2 3"), "untyped body comma-list:\n{out}");
}

#[test]
fn typed_int_comma_list() {
    let (out, code) = run(&m("localparam int W = 8, X = 16;", "%0d %0d", "W, X"));
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("8 16"), "typed int body comma-list:\n{out}");
}

#[test]
fn continuation_inherits_narrow_width() {
    // `M` inherits `[3:0]` ⇒ 20 truncates to 4 (was value-sized implicit 32-bit).
    let (out, code) = run(&m("localparam [3:0] N = 20, M = 20;", "%0d %0d", "N, M"));
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("4 4"), "narrow-width inherit:\n{out}");
}

#[test]
fn continuation_inherits_signedness() {
    // `Y` inherits `signed [7:0]` ⇒ 200 wraps to -56.
    let (out, code) = run(&m(
        "localparam signed [7:0] X = -1, Y = 200;",
        "%0d %0d",
        "X, Y",
    ));
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("-1 -56"), "signed inherit:\n{out}");
}

#[test]
fn parameter_keyword_comma_list() {
    let (out, code) = run(&m("parameter P = 5, Q = 6;", "%0d %0d", "P, Q"));
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("5 6"), "parameter comma-list:\n{out}");
}

#[test]
fn inter_param_reference_in_list() {
    // A later name may reference an earlier one in the SAME list.
    let (out, code) = run(&m("localparam int A = 5, B = A + 1;", "%0d %0d", "A, B"));
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("5 6"), "inter-param reference:\n{out}");
}

#[test]
fn comma_list_is_last_module_item() {
    // The comma-list is the LAST item before `endmodule` — the first name advances
    // the cursor onto `endmodule`, so the drain must survive the loop-exit check.
    let (out, code) = run(
        "module t;\n  initial begin #1 $display(\"%0d %0d\", A, B); $finish; end\n  \
         localparam A = 11, B = 22;\nendmodule\n",
    );
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("11 22"), "last-item comma-list drain:\n{out}");
}

#[test]
fn package_body_comma_list_scoped() {
    // Package body comma-list — every name is a scoped package constant (`pk::B`),
    // not only the first. This is the last-item edge inside a package.
    let (out, code) = run("package pk; localparam A = 10, B = 20; endpackage\n\
         module t; initial begin $display(\"%0d %0d\", pk::A, pk::B); $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("10 20"), "package scoped comma-list:\n{out}");
}

#[test]
fn generate_block_body_comma_list() {
    let (out, code) = run(
        "module t;\n  genvar g;\n  generate for (g=0;g<1;g++) begin:blk\n\
         localparam L1 = 100, L2 = 200;\n    initial $display(\"%0d %0d\", L1, L2);\n\
         end endgenerate\n  initial #1 $finish;\nendmodule\n",
    );
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("100 200"), "generate-block comma-list:\n{out}");
}

#[test]
fn single_param_unchanged() {
    // A single (non-comma) param stays byte-identical.
    let (out, code) = run(&m("localparam int A = 7;", "%0d", "A"));
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains('7'), "single param regression:\n{out}");
}
