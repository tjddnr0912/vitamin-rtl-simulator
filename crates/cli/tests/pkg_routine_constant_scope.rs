//! The constant interpreter reads a package function's own constant at its DECLARED width
//! and sign, whatever the calling module binds under the same name (IEEE 1800-2017 §26.3).
//!
//! Before: the interpreter seeded the package's constants as bare values and sized them
//! from the CALLER's same-named parameter — `return C;` over a package `[15:0] C =
//! 16'h0123` beside a module `[7:0] C` folded `23`. A select, a concatenation and the
//! run-time lanes of the same constant are ROADMAP §2 residues (the declaring-scope
//! fold). Values: iverilog 13.0 and verilator 5.052 agree on every line. `PRE` is vita
//! before §4.5.561.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn lines(src: &str) -> Vec<String> {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_prcs_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    let s =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "expected exit 0, got:\n{s}");
    let mut v: Vec<String> = s
        .lines()
        .filter(|l| l.starts_with("T "))
        .map(|l| l.trim_end().to_string())
        .collect();
    v.sort();
    v
}

/// The interpreter reads a package constant at its declared width and sign — a signed
/// 8-bit, an unsigned 16-bit, an untyped expression — beside module parameters of the
/// same names and other shapes. PRE: `0000000d 00000001 00000000` in the constant lane.
#[test]
fn a_package_constant_keeps_its_type_in_the_interpreter() {
    let src = "package pk;\n  localparam signed [7:0] S = -8'sd3;\n  localparam [15:0] U = 16'hff00;\n  \
               localparam E = 5 + 3;\n  function automatic [31:0] f(); return S; endfunction\n  \
               function automatic [31:0] g(); return U + 1; endfunction\n  function automatic \
               [31:0] h(); return E << 30; endfunction\nendpackage\nmodule t;\n  localparam [3:0] S \
               = 4'd7;\n  localparam signed [3:0] U = -4'sd1;\n  localparam [2:0] E = 3'd1;\n  \
               localparam [31:0] A = pk::f(), B = pk::g(), C = pk::h();\n  initial begin #1 \
               $display(\"T %h %h %h | %h %h %h\", A, B, C, pk::f(), pk::g(), pk::h()); $finish; \
               end\nendmodule\n";
    assert_eq!(
        lines(src),
        vec!["T fffffffd 0000ff01 00000000 | fffffffd 0000ff01 00000000"],
        "{src}"
    );
}

/// The same through an explicit import of the function into the calling module, whose
/// `[7:0] C` the pre-slice interpreter sized the package's `[15:0] C` from. PRE:
/// `K=00000023`.
#[test]
fn an_imported_package_function_keeps_its_constants_type() {
    let src = "package pk;\n  localparam [15:0] C = 16'h0123;\n  function automatic [31:0] g(); \
               return C; endfunction\nendpackage\nmodule t;\n  import pk::g;\n  localparam [7:0] \
               C = 8'hEE;\n  localparam [31:0] K = g();\n  initial begin #1 $display(\"T K=%h\", \
               K); $finish; end\nendmodule\n";
    assert_eq!(lines(src), vec!["T K=00000123"], "{src}");
}
