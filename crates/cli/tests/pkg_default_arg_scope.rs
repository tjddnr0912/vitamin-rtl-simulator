//! IEEE 1800-2017 §13.5.4: a formal's DEFAULT value is evaluated in the scope where
//! the subroutine is DECLARED — for a package routine, the package.
//!
//! Every lane fills an omitted actual with the declaration's default expression and
//! lowers it beside the user's actuals in the CALLER's scope, so a package routine's
//! default naming a package variable, constant or sibling routine bound to the
//! calling module's same-named object: `function automatic [31:0] gd(input [15:0]
//! a = x)` with a package `x = 16'h0123` and a module `x = 8'hEE` printed `D=ef`
//! where both oracles print `124`; `a = C` read the module's `C`, `a = h() + 1` the
//! module's `h`. `with_default_arg_scope` (pkg_body_scope.rs) runs the actual
//! lowering with the package's scope pushed when the actual IS the declared default
//! (span identity — `resolve_named_args` clones the default, span included) and the
//! routine is a package routine; `inject_pkg_callees` now also collects the callees
//! a default names, so `h()` in a default reaches the package's `h` under its
//! scoped key. A user-written actual, a module routine's default and the existing
//! "binds differently at this call site" refusal are untouched.
//!
//! ORACLES: iverilog 13.0 (-g2012) and verilator 5.052 (--binary --timing); every
//! value asserted below was measured in BOTH. PRE values are from a release binary
//! built at the parent commit.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_pkgdef_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("t.sv"), src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("t.sv")
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    let so = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.contains("simulation ended"))
        .collect::<Vec<_>>()
        .join("\n");
    (
        so,
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    )
}

fn clean(src: &str) -> String {
    let (o, e, code) = run(src);
    assert_eq!(code, Some(0), "expected a clean run:\n{o}\n{e}");
    o
}

const PK: &str = r#"package pk;
  logic [15:0] x = 16'h0123;
  localparam int C = 40;
  function automatic [31:0] h; h = 7; endfunction
  function automatic [31:0] gd(input [15:0] a = x); gd = a + 1; endfunction
  function automatic [31:0] gc(input [15:0] a = C); gc = a + 1; endfunction
  function           [31:0] gs(input [15:0] a = x); gs = a + 1; endfunction
  function automatic [31:0] gh(input [31:0] a = h()); gh = a + 1; endfunction
  function automatic [31:0] gh2(input [31:0] a = h() + 1); gh2 = a; endfunction
  function automatic [31:0] gk(input [31:0] a = $clog2(C) + x); gk = a; endfunction
  function automatic [31:0] gn(input [15:0] a = x, input [15:0] b = 16'h1); gn = a + b; endfunction
  task automatic td(input [15:0] a = x, output [31:0] o); o = a + 1; endtask
  task           ts(input [15:0] a = x, output [31:0] o); o = a + 1; endtask
  task automatic tt(output [31:0] o, input [15:0] a = x + C); o = a; endtask
endpackage
"#;

fn design(body: &str) -> String {
    format!("{PK}module t;\n  import pk::*;\n  logic [7:0] x = 8'hEE;\n  localparam int C = 4;\n  function [31:0] h; h = 99; endfunction\n{body}\nendmodule\n")
}

/// ① THE HEADLINE across the four lanes: a default naming a package VARIABLE in an
/// automatic function (frame), a static function (inline fold), an automatic task
/// (frame task) and a static task (inline task); a default naming a package
/// CONSTANT; and the user-written actual, which stays caller-scoped.
#[test]
fn a_default_actual_of_a_package_routine_resolves_in_the_package() {
    let o = clean(&design(
        "  logic [31:0] o1, o2;\n  initial begin\n    td(.o(o1)); ts(.o(o2));\n    $display(\"D=%h C=%h S=%h TD=%h TS=%h U=%h\", gd(), gc(), gs(), o1, o2, gd(16'h7));\n    #1 $finish;\n  end",
    ));
    // PRE: `D=000000ef C=00000005 S=000000ef TD=000000ef TS=000000ef U=00000008`.
    assert_eq!(
        o,
        "D=00000124 C=00000029 S=00000124 TD=00000124 TS=00000124 U=00000008"
    );
}

/// ② A default that CALLS a sibling package routine, bare and inside an expression,
/// beside a module routine of the same name — the callee is collected from the
/// default (`collect_callee_ports`) and injected under its scoped key.
#[test]
fn a_default_calling_a_sibling_routine_binds_the_package_sibling() {
    let o = clean(&design(
        "  initial begin $display(\"H=%h H2=%h\", gh(), gh2()); #1 $finish; end",
    ));
    // PRE: `H=00000064 H2=00000064` — the module's `h` (99 + 1).
    assert_eq!(o, "H=00000008 H2=00000008");
}

/// ③ A default that is an EXPRESSION over package names (`$clog2(C) + x`,
/// `x + C` in a task whose default follows an output formal), and named-argument
/// spellings that leave one formal to its default (`.b(2)` → `a = x`; `.a(5)`
/// keeps the user's value); the scoped `pk::gn()` lane.
#[test]
fn default_expressions_and_named_argument_defaults() {
    let o = clean(&design(
        "  logic [31:0] o;\n  initial begin\n    tt(o);\n    $display(\"TT=%h GK=%h N1=%h N2=%h N3=%h SC=%h\", o, gk(), gn(.b(16'h2)), gn(.a(16'h5)), gn(), pk::gn());\n    #1 $finish;\n  end",
    ));
    // PRE: `TT=000000f2 GK=000000f0 N1=000000f0 N2=00000006 N3=000000ef SC=000000ef`.
    assert_eq!(
        o,
        "TT=0000014b GK=00000129 N1=00000125 N2=00000006 N3=00000124 SC=00000124"
    );
}

/// ④ CONTROLS, byte-identical to PRE: a MODULE routine's default still resolves in
/// the module (`mf() = ee`), and a package default naming a name the package does
/// NOT declare still falls back to the caller (both oracles refuse that program:
/// `Unable to bind wire/reg/memory zz in pk.gf`; ROADMAP §2, the import-lane
/// lenience).
#[test]
fn a_module_default_and_a_free_name_default_are_unchanged() {
    let o = clean(&design(
        "  function [31:0] mf(input [15:0] a = x); mf = a; endfunction\n  initial begin $display(\"MF=%h\", mf()); #1 $finish; end",
    ));
    assert_eq!(o, "MF=000000ee");
    let o = clean(
        "package pk; logic [15:0] x = 16'h0123; function automatic [31:0] gf(input [15:0] a = zz); gf = a; endfunction endpackage\n\
         module t; import pk::*; logic [7:0] zz = 8'h11; initial begin $display(\"GF=%h\", gf()); #1 $finish; end endmodule\n",
    );
    assert_eq!(o, "GF=00000011");
}
