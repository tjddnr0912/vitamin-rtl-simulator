//! §4.5.440 (ROADMAP §2 🆕 L ⓦ / §3): a PACKAGE function in a constant context —
//! a header parameter default, a body `localparam`, a declared range bound —
//! folds through the constant-function interpreter, in the package's own scope.
//!
//! Every value below is both oracles' (iverilog 13 / verilator 5.050). Before this
//! slice: `dbl(…) has no constant-fold arm` (E3009) for the parameter spellings,
//! and a SILENT 1-bit net for the range-bound spelling (both oracles fold 8).
//! A module function in a HEADER default was loud too (the table was collected
//! after the header fold); an UNKNOWN function in a range bound was a silent
//! 1-bit net where both oracles reject the declaration — loud now.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_pkgcf_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (s, out.status.code())
}

const PKG: &str = "package p;
  localparam int K = 2;
  function automatic int dbl(int a); return a*K; endfunction
  function automatic int trp(int a); return a*3; endfunction
endpackage
";

fn prints(body: &str, want: &str) {
    let src = format!(
        "`timescale 1ns/1ns\n{PKG}{body}\nmodule top; m u(); initial #1 $finish; endmodule\n"
    );
    let (out, code) = run(&src);
    assert_eq!(code, Some(0), "{src}\n{out}");
    assert!(
        out.lines().any(|l| l == want),
        "expected `{want}`\n{src}\n{out}"
    );
}

fn loud(body: &str, want: &str) {
    let src = format!(
        "`timescale 1ns/1ns\n{PKG}{body}\nmodule top; m u(); initial #1 $finish; endmodule\n"
    );
    let (out, code) = run(&src);
    assert_eq!(code, Some(1), "{src}\n{out}");
    assert!(out.contains(want), "expected `{want}`\n{src}\n{out}");
}

/// A header import: the header default and a range bound (both oracles `X=8 bits=8`).
#[test]
fn header_import_default_and_range_bound() {
    prints(
        "module m import p::*; #(parameter int W = 4, parameter int X = dbl(W)) ();
  logic [dbl(W)-1:0] r; initial $display(\"X=%0d bits=%0d\", X, $bits(r)); endmodule",
        "X=8 bits=8",
    );
}

/// The range-bound spelling alone was the SILENT cell (1 bit, oracles 8).
#[test]
fn range_bound_alone_folds() {
    prints(
        "module m import p::*; #(parameter int W = 4) ();
  logic [dbl(W)-1:0] r; initial $display(\"bits=%0d\", $bits(r)); endmodule",
        "bits=8",
    );
}

/// `p::dbl(4)` scoped, an explicit `import p::dbl`, a body wildcard import.
#[test]
fn scoped_explicit_and_body_import() {
    prints(
        "module m (); localparam int X = p::dbl(4); logic [p::dbl(2)-1:0] r;
  initial $display(\"scoped X=%0d bits=%0d\", X, $bits(r)); endmodule",
        "scoped X=8 bits=4",
    );
    prints(
        "module m (); import p::dbl; localparam int X = dbl(4); initial $display(\"explicit X=%0d\", X); endmodule",
        "explicit X=8",
    );
    prints(
        "module m (); import p::*; localparam int X = dbl(4); logic [trp(2)-1:0] r;
  initial $display(\"body X=%0d bits=%0d\", X, $bits(r)); endmodule",
        "body X=8 bits=6",
    );
}

/// §26.3: a module-local function of the same name wins a wildcard import (40, not 8).
#[test]
fn local_definition_shadows_the_wildcard() {
    prints(
        "module m (); import p::*; function automatic int dbl(int a); return a*10; endfunction
  localparam int X = dbl(4); initial $display(\"shadow X=%0d\", X); endmodule",
        "shadow X=40",
    );
}

/// The package body folds in the PACKAGE's scope: `K` inside `dbl` is the
/// package's 2, not the importing module's same-named 7 (both oracles 8).
#[test]
fn package_body_reads_the_packages_constant() {
    prints(
        "module m (); import p::*; localparam int K = 7; localparam int X = dbl(4);
  initial $display(\"pkgK X=%0d\", X); endmodule",
        "pkgK X=8",
    );
}

/// A MODULE function in a header default (both oracles `X=8 bits=4`; was E3009).
#[test]
fn module_function_in_a_header_default() {
    prints(
        "module m #(parameter int X = mdbl(4)) ();
  function automatic int mdbl(int a); return a*2; endfunction
  logic [mdbl(2)-1:0] r; initial $display(\"mod X=%0d bits=%0d\", X, $bits(r)); endmodule",
        "mod X=8 bits=4",
    );
}

/// An unknown function in a range bound is loud (both oracles reject; vita was a
/// silent 1-bit net).
#[test]
fn unknown_function_in_a_range_bound_is_loud() {
    loud(
        "module m (); logic [nosuch(2)-1:0] r; initial $display(\"bits=%0d\", $bits(r)); endmodule",
        "a function call that does not fold to a constant is not allowed in a constant range bound",
    );
}

/// Review (§4.5.440 S1-D1): an explicit import of a name the module also declares is
/// an error (IEEE §26.3; iverilog refuses it) — the constant table used to answer the
/// package's function where the runtime table answered the local's.
#[test]
fn explicit_import_colliding_with_a_local_function_is_loud() {
    loud(
        "module m (); import p::dbl; function automatic int dbl(int a); return a*10; endfunction
  localparam int X = dbl(4); initial $display(\"X=%0d\", X); endmodule",
        "explicit import of `dbl` from package `p` conflicts with a local declaration of the same name",
    );
}

/// Review (§4.5.440 G1): a package function calling a function it IMPORTS from another
/// package folds (both oracles `A=10 B=8 w=4`); the importing package's function set
/// includes the imported ones, a local definition winning.
#[test]
fn a_package_function_calling_an_imported_package_function_folds() {
    let src = "`timescale 1ns/1ns
package q; function automatic int inc(int a); return a + 1; endfunction endpackage
package p; import q::*; function automatic int stat(int a); return inc(a) * 2; endfunction endpackage
module m (); import p::*; localparam int A = stat(4); localparam int B = p::stat(3); logic [stat(1)-1:0] w;
  initial $display(\"A=%0d B=%0d w=%0d\", A, B, $bits(w)); endmodule
module top; m u(); initial #1 $finish; endmodule
";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.lines().any(|l| l == "A=10 B=8 w=4"), "{out}");
}
