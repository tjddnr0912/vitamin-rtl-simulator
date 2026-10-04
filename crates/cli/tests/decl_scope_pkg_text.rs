//! IEEE 1800-2017 §13.4 / §26.3: a package routine's own TEXT — its return and
//! formal ranges, its formals' defaults, its body-local declaration ranges and the
//! constant calls in its body — names what its DECLARING package names (§4.5.589,
//! ROADMAP §2 🆕 AD, package half).
//!
//! vita folded that text where the call was: the constant interpreter folded a
//! callee's header before switching to the callee's package, and the frame reserve,
//! the inline lane and the call-typing sites folded it at the caller's prefix with
//! the caller's constant-function table live. A package routine
//! `function automatic logic [f(2):0] h(...)` whose package `f` returns 3 was 8
//! bits wide, not 4, from a module declaring its own `f` returning 7 (`v=232`, both
//! oracles `v=8`). `elaborate/src/decl_scope.rs` folds each unit of such text twice — the
//! pre-slice fold first, then with the declaring package's bindings probed first —
//! and takes the second answer only where the first one answered ("corrections
//! only"): a unit the pre-slice fold could not fold stays loud, because opening it
//! hands a value to consumers that inherit known silent defects (ROADMAP §3.b
//! `pkg-text-open`).
//!
//! ORACLES: iverilog 13.0 (`-g2012` + vvp), verilator 5.052 (`--binary --timing
//! --assert`) and sv2v 0.0.13 → iverilog. Every value asserted below was measured
//! in all three unless the docstring says otherwise. PRE values are from the release
//! binary built at 303703f9's code (2f2d3f2d).

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn tdir() -> std::path::PathBuf {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_declscope_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn vita(dir: &std::path::Path, args: &[&str]) -> (String, String, Option<i32>) {
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("run vita");
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

/// (stdout without the trailer, stderr, exit code) of a one-shot run.
fn run(src: &str) -> (String, String, Option<i32>) {
    let d = tdir();
    std::fs::write(d.join("t.sv"), src).unwrap();
    let r = vita(&d, &["t.sv"]);
    let _ = std::fs::remove_dir_all(&d);
    r
}

fn clean(src: &str) -> String {
    let (o, e, code) = run(src);
    assert_eq!(code, Some(0), "expected a clean run:\n{o}\n{e}");
    o
}

fn loud(src: &str, needle: &str) {
    let (o, e, code) = run(src);
    assert_eq!(code, Some(1), "expected a loud run (exit 1):\n{o}\n{e}");
    assert!(e.contains(needle), "expected {needle:?} in:\n{e}");
}

/// The package `q` most cells share: `f(2)` is 3, so `h` is 4 bits wide.
const Q_F3: &str = "package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic logic [f(2):0] h(input int x); return x; endfunction
endpackage
";

/// The caller's own `f`, which answers 7 for the same call — what PRE bound.
const TOP_F7: &str = "  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
";

// ---------------------------------------------------------------------------
// Corrections: PRE answered with the caller's binding; POST = all three oracles.
// ---------------------------------------------------------------------------

/// Interpreter header (L1): a localparam's call folds the callee's return range in
/// the package. PRE `P=232`.
#[test]
fn interp_return_range_binds_package_function() {
    let src = format!(
        "{Q_F3}module top;\n{TOP_F7}  localparam int P = q::h(1000);
  initial begin #1 $display(\"P=%0d\", P); $finish; end
endmodule\n"
    );
    assert_eq!(clean(&src), "P=8");
}

/// Interpreter header (L1): a formal's DEFAULT folds in the package. PRE `P=1007`.
#[test]
fn interp_default_binds_package_function() {
    let src = format!(
        "package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic int h(input int x, input int k = f(2)); return x + k; endfunction
endpackage
module top;\n{TOP_F7}  localparam int P = q::h(1000);
  initial begin #1 $display(\"P=%0d\", P); $finish; end
endmodule\n"
    );
    assert_eq!(clean(&src), "P=1003");
}

/// Frame reserve (L4): the run-time return net is sized in the package. PRE `v=232`.
#[test]
fn reserve_return_range_binds_package_function() {
    let src = format!(
        "{Q_F3}module top;\n{TOP_F7}  int v;
  initial begin v = q::h(1000); $display(\"v=%0d\", v); #1 $finish; end
endmodule\n"
    );
    assert_eq!(clean(&src), "v=8");
}

/// Frame reserve (L4), the constant twin: a package `localparam` in the return range
/// beats the caller's same-named one. PRE `v=232`.
#[test]
fn reserve_return_range_binds_package_constant() {
    let src = "package q;
  localparam int W = 3;
  function automatic logic [W:0] h(input int x); return x; endfunction
endpackage
module top;
  localparam int W = 7;
  int v;
  initial begin v = q::h(1000); $display(\"v=%0d\", v); #1 $finish; end
endmodule\n";
    assert_eq!(clean(src), "v=8");
}

/// Body lane (L6): a size cast's width is a constant call in the body. PRE `v=-24`.
#[test]
fn body_cast_width_binds_package_function() {
    let src = format!(
        "package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic int h(input int x); return f(2)'(x); endfunction
endpackage
module top;\n{TOP_F7}  int v;
  initial begin v = q::h(1000); $display(\"v=%0d\", v); #1 $finish; end
endmodule\n"
    );
    assert_eq!(clean(&src), "v=0");
}

/// Body lane (L6): a replication count. PRE `v=0000007f`.
#[test]
fn body_replication_binds_package_function() {
    let src = format!(
        "package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic logic [31:0] h(input int x); return {{f(2){{1'b1}}}}; endfunction
endpackage
module top;\n{TOP_F7}  logic [31:0] v;
  initial begin v = q::h(0); $display(\"v=%h\", v); #1 $finish; end
endmodule\n"
    );
    assert_eq!(clean(&src), "v=00000007");
}

/// Body lane (L6): a part-select bound. PRE `v=000000cd`.
#[test]
fn body_part_select_binds_package_function() {
    let src = format!(
        "package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic logic [31:0] h(input logic [15:0] x); return x[f(2):0]; endfunction
endpackage
module top;\n{TOP_F7}  logic [31:0] v;
  initial begin v = q::h(16'hABCD); $display(\"v=%h\", v); #1 $finish; end
endmodule\n"
    );
    assert_eq!(clean(&src), "v=0000000d");
}

/// `$bits` of a call reads the reserved return width (L8). PRE `B=8`.
#[test]
fn bits_of_call_reads_package_width() {
    let src = format!(
        "{Q_F3}module top;\n{TOP_F7}  initial begin #1 $display(\"B=%0d\", $bits(q::h(0))); $finish; end
endmodule\n"
    );
    assert_eq!(clean(&src), "B=4");
}

/// Inline lane (L5): a static body with no `return` that reads a package variable
/// is inlined, and its return width is the package's. PRE `v=232`.
#[test]
fn inline_return_range_binds_package_function() {
    let src = "package q;
  function automatic int f(input int a); return 3; endfunction
  int pv = 0;
  function logic [f(2):0] h(input int x); h = x + pv; endfunction
endpackage
module top;
  import q::h;
  function automatic int f(input int a); f = 7; if (a == 1) f = 10; endfunction
  int v; int a = 1000;
  always_comb v = h(a);
  initial begin #1 $display(\"v=%0d\", v); $finish; end
endmodule\n";
    assert_eq!(clean(src), "v=8");
}

/// A package TASK's local range (task reserve, L4). PRE `v=232`.
#[test]
fn task_local_range_binds_package_function() {
    let src = format!(
        "package q;
  function automatic int f(input int a); return 3; endfunction
  task automatic tk(input int x, output int o);
    logic [f(2):0] t;
    t = x;
    o = t;
  endtask
endpackage
module top;
  import q::tk;\n{TOP_F7}  int v;
  initial begin tk(1000, v); $display(\"v=%0d\", v); #1 $finish; end
endmodule\n"
    );
    assert_eq!(clean(&src), "v=8");
}

/// A STATIC package task's FORMAL range (inline task lane, L5, armed as the task's
/// text): `input logic [W:0] a` takes the package's `W = 3`, not the caller's `W = 7`,
/// so `-1` copies in as 15. `v=15` = iverilog, verilator, sv2v; PRE `v=255`.
#[test]
fn static_task_formal_range_binds_package_constant() {
    let src = "package p;
  localparam int W = 3;
  function automatic int g(); return 3; endfunction
  task t(input logic [W:0] a, output int o); o = a; endtask
endpackage
module top;
  import p::t;
  localparam int W = 7;
  function automatic int g(); return 7; endfunction
  int v;
  initial begin t(-1, v); $display(\"v=%0d\", v); end
endmodule\n";
    assert_eq!(clean(src), "v=15");
}

/// A STATIC package task is expanded inline, and its local's range is folded while
/// the task's body scope is pushed: the body lane reaches that constant call. PRE
/// `v=232`. (The same local sized by a package CONSTANT, `logic [W:0] t`, keeps
/// PRE's caller binding `v=232` — no arming site folds the inline task's locals
/// yet; iverilog, verilator, sv2v `v=8`.)
#[test]
fn inline_task_local_range_binds_package_function() {
    let src = "package q;
  function automatic int f(input int a); return 3; endfunction
  int pv = 0;
  task tk(input int x, output int o);
    logic [f(2):0] t;
    t = x + pv;
    o = t;
  endtask
endpackage
module top;
  import q::tk;
  function automatic int f(input int a); return 7; endfunction
  int v;
  initial begin tk(1000, v); $display(\"v=%0d\", v); #1 $finish; end
endmodule\n";
    assert_eq!(clean(src), "v=8");
}

/// Call typing (L9): an untyped localparam takes the call's declared return width.
/// PRE `P=232 b=8`.
#[test]
fn untyped_param_takes_package_return_width() {
    let src = format!(
        "{Q_F3}module top;
  function automatic int f(input int a); f = 7; if (a == 1) f = 10; endfunction
  localparam P = q::h(1000);
  initial begin #1 $display(\"P=%0d b=%0d\", P, $bits(P)); $finish; end
endmodule\n"
    );
    assert_eq!(clean(&src), "P=8 b=4");
}

/// Call typing (L9): a signed return range sized in the package sign-extends from
/// its true top bit. PRE `P=15 P2=15 b=8`.
#[test]
fn signed_return_width_binds_package_function() {
    let src = "package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic logic signed [f(2):0] h(input int x); return x; endfunction
endpackage
module top;
  function automatic int f(input int a); f = 7; if (a == 1) f = 10; endfunction
  localparam int P = q::h(15) + 0;
  localparam P2 = q::h(15);
  initial begin #1 $display(\"P=%0d P2=%0d b=%0d\", P, P2, $bits(P2)); $finish; end
endmodule\n";
    assert_eq!(clean(src), "P=-1 P2=-1 b=4");
}

/// The caller's `f` arrives through a wildcard import of ANOTHER package. PRE
/// `P=232 v=232`.
#[test]
fn imported_caller_function_does_not_capture_package_text() {
    let src = "package a;
  function automatic int f(input int n);
    f = 7;
    if (n == 1) f = 10;
  endfunction
endpackage
package q;
  function automatic int f(input int n); return 3; endfunction
  function automatic logic [f(2):0] h(input int x); return x; endfunction
endpackage
module top;
  import a::*;
  localparam int P = q::h(1000);
  int v;
  initial begin v = q::h(1000); #1 $display(\"P=%0d v=%0d\", P, v); $finish; end
endmodule\n";
    assert_eq!(clean(src), "P=8 v=8");
}

/// Nested package text: `q::h`'s range calls `r::g`, whose own range calls `r`'s
/// `f2`, while `q` and the caller each declare an `f2` of their own. PRE
/// `P=1000 v=1000`.
#[test]
fn nested_package_header_binds_each_declaring_package() {
    let src = "package r;
  function automatic int f2(input int a); return 3; endfunction
  function automatic logic [f2(0):0] g(input int a); return a; endfunction
endpackage
package q;
  function automatic int f2(input int a); return 7; endfunction
  function automatic logic [r::g(21)+2:0] h(input int x); return x; endfunction
endpackage
module top;
  function automatic int f2(input int a); return 11; endfunction
  localparam int P = q::h(1000);
  int v;
  initial begin v = q::h(1000); #1 $display(\"P=%0d v=%0d\", P, v); $finish; end
endmodule\n";
    assert_eq!(clean(src), "P=232 v=232");
}

/// The caller's `f` made the header 100 bits wide, so PRE kept every bit of the
/// value; the package's makes it 4. PRE `P=1000`.
#[test]
fn caller_width_does_not_survive_the_correction() {
    let src = format!(
        "{Q_F3}module top;
  function automatic int f(input int a); return 99; endfunction
  localparam int P = q::h(1000);
  initial begin #1 $display(\"P=%0d\", P); $finish; end
endmodule\n"
    );
    assert_eq!(clean(&src), "P=8");
}

/// The package's `f` calls `r::k`, whose body calls a bare `g` that `q` and `r` both
/// declare: the callee BODIES resolve in their own packages, never in the window of
/// the text that called them. PRE `P=235 v=235` (caller `f`).
#[test]
fn header_window_does_not_reach_callee_bodies() {
    let src = "package r;
  function automatic int g(input int a); return 2; endfunction
  function automatic int k(input int a); return g(a); endfunction
endpackage
package q;
  function automatic int g(input int a); return 5; endfunction
  function automatic int f(input int a); return r::k(a); endfunction
  function automatic logic [f(2):0] h(input int x); return x; endfunction
endpackage
module top;
  function automatic int f(input int a); return 7; endfunction
  localparam int P = q::h(1003);
  int v;
  initial begin v = q::h(1003); #1 $display(\"P=%0d v=%0d\", P, v); $finish; end
endmodule\n";
    assert_eq!(clean(src), "P=3 v=3");
}

/// The body-lane twin of the cell above. PRE `v=0000007f`.
#[test]
fn body_window_does_not_reach_callee_bodies() {
    let src = "package r;
  function automatic int g(input int a); return 2; endfunction
  function automatic int k(input int a); return g(a); endfunction
endpackage
package q;
  function automatic int g(input int a); return 5; endfunction
  function automatic int f(input int a); return r::k(a); endfunction
  function automatic logic [31:0] h(input int x); return {f(2){1'b1}}; endfunction
endpackage
module top;
  import q::h;
  function automatic int f(input int a); return 7; endfunction
  logic [31:0] v;
  initial begin v = h(0); $display(\"v=%h\", v); #1 $finish; end
endmodule\n";
    assert_eq!(clean(src), "v=00000003");
}

/// Two compilation units: the package and its caller compiled into one library
/// separately from a child module. The window keys on the package NAME, so the
/// staged flow corrects exactly as the one-shot flow does. PRE `top.v=232`;
/// iverilog `top.v=8 c.w=40 c.g.P=3` — `c.g.P` is a generate-scoped constant
/// function (verilator refuses it, IEEE 1800-2023 §13.4.3), not this slice's
/// half, and stays at PRE's 5.
#[test]
fn staged_two_units_correct_like_one_shot() {
    let d = tdir();
    std::fs::write(
        d.join("t.sv"),
        format!(
            "{Q_F3}module top;\n{TOP_F7}  c u ();
  int v;
  initial begin v = q::h(1000); $display(\"top.v=%0d\", v); #2 $finish; end
endmodule\n"
        ),
    )
    .unwrap();
    std::fs::write(
        d.join("c.sv"),
        "module c;
  function automatic int f(input int a); return 5; endfunction
  if (1) begin : g
    function automatic int f(input int a); return 3; endfunction
    localparam int P = f(2);
    initial #1 $display(\"c.g.P=%0d\", P);
  end
  function automatic logic [f(2):0] h(input int x); return x; endfunction
  int w;
  initial begin w = h(1000); $display(\"c.w=%0d\", w); end
endmodule\n",
    )
    .unwrap();
    for f in ["c.sv", "t.sv"] {
        let (o, e, code) = vita(&d, &["vcmp", "--work", "work=wl", f]);
        assert_eq!(code, Some(0), "vcmp {f}:\n{o}\n{e}");
    }
    let (o, e, code) = vita(
        &d,
        &["velab", "-L", "work=wl", "--top", "top", "-o", "t.velab"],
    );
    assert_eq!(code, Some(0), "velab:\n{o}\n{e}");
    let (o, e, code) = vita(&d, &["vrun", "t.velab"]);
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(code, Some(0), "vrun:\n{o}\n{e}");
    assert_eq!(o, "top.v=8\nc.w=40\nc.g.P=5");
}

// ---------------------------------------------------------------------------
// PRE kept: the window answers nothing better, so the pre-slice answer stands.
// ---------------------------------------------------------------------------

/// The package's `f` has a `case` body, outside the constant interpreter's subset;
/// the caller's same-valued `f` folds. The window's half declines, so PRE's answer
/// stays (`win.or(pre)`) instead of turning loud. = PRE = all three oracles.
#[test]
fn window_decline_keeps_pre_answer() {
    let pkg = "package q;
  function automatic int f(input int a);
    case (a)
      2: f = 3;
      default: f = 9;
    endcase
  endfunction
  function automatic logic [f(2):0] h(input int x); return x; endfunction
endpackage
module top;
  function automatic int f(input int a); return 3; endfunction
";
    let ce = format!(
        "{pkg}  localparam int P = q::h(1000);
  initial begin #1 $display(\"P=%0d\", P); $finish; end
endmodule\n"
    );
    assert_eq!(clean(&ce), "P=8");
    let rt = format!(
        "{pkg}  int v;
  initial begin v = q::h(1000); $display(\"v=%0d\", v); #1 $finish; end
endmodule\n"
    );
    assert_eq!(clean(&rt), "v=8");
    let rep = format!(
        "{pkg}  wire [31:0] w;
  assign w = {{q::h(1000){{1'b1}}}};
  initial begin #1 $display(\"w=%h\", w); $finish; end
endmodule\n"
    );
    assert_eq!(clean(&rep), "w=000000ff");
}

/// A `$unit` routine whose header calls a `$unit` function, called from `q::h`'s
/// header: the window never switches the interpreter's package, so the unit callee
/// still resolves. = PRE = all three oracles.
#[test]
fn unit_routine_in_package_header_keeps_resolving() {
    let src = "function automatic int uf(input int a); return 3; endfunction
function automatic logic [uf(2):0] u(input int a); return a; endfunction
package q;
  function automatic logic [u(21)+2:0] h(input int x); return x; endfunction
endpackage
module top;
  localparam int P = q::h(1000);
  int v;
  initial begin v = q::h(1000); #1 $display(\"P=%0d v=%0d\", P, v); $finish; end
endmodule\n";
    assert_eq!(clean(src), "P=232 v=232");
}

/// `q` imports `r::g`; `g`'s range names `K`, which `q` declares differently. The
/// window belongs to the package that DECLARES a routine, so `g`'s text is not
/// `q`'s: PRE's fold stands (the caller's `K` equals `r`'s, so it is right here).
/// = PRE = all three oracles.
#[test]
fn routine_imported_into_package_is_not_its_text() {
    let src = "package r;
  localparam int K = 3;
  function automatic logic [K:0] g(input int x); return x; endfunction
endpackage
package q;
  import r::g;
  localparam int K = 5;
  function automatic int h(input int x); return g(x); endfunction
endpackage
module top;
  localparam int K = 3;
  localparam int P = q::h(1000);
  int v;
  initial begin v = q::h(1000); #1 $display(\"P=%0d v=%0d\", P, v); $finish; end
endmodule\n";
    assert_eq!(clean(src), "P=8 v=8");
}

/// A header that calls itself through the package's `f` (the caller declares an `f`
/// too): the window re-enters `f`'s header, which is refused, and the split keeps
/// PRE's answer — no stack overflow. = PRE `P=8`. Oracles: iverilog asserts
/// (`elab_expr.cc:2927: failed assertion def`), verilator crashes (build rc 139),
/// sv2v → iverilog `P=0`; no oracle decides this illegal design.
#[test]
fn header_reentry_keeps_pre_answer() {
    let src = "package q;
  function automatic logic [f(2):0] f(input int a); return a; endfunction
  function automatic logic [f(2):0] h(input int x); return x; endfunction
endpackage
module top;
  function automatic int f(input int a); return 3; endfunction
  localparam int P = q::h(1000);
  initial begin #1 $display(\"P=%0d\", P); $finish; end
endmodule\n";
    assert_eq!(clean(src), "P=8");
}

/// RESIDUE, kept at PRE. `q`'s `f` is imported from `r`, and the window answers only
/// for a function its package DECLARES: `q`'s copy of an imported function would
/// run its body under `q`'s constants (§4.5.589 review round 1, the pins below), and
/// binding it to `r` needs the import's origin recorded (ROADMAP §2 "Scoping", a
/// routine imported into another package). So the header still binds the caller's
/// `f`: vita `P=232 v=232` = PRE; iverilog, verilator, sv2v `P=8 v=8`.
#[test]
fn imported_callee_in_package_text_keeps_pre_residue() {
    let src = format!(
        "package r;
  function automatic int f(input int a); return 3; endfunction
endpackage
package q;
  import r::f;
  function automatic logic [f(2):0] h(input int x); return x; endfunction
endpackage
module top;\n{TOP_F7}  localparam int P = q::h(1000);
  int v;
  initial begin v = q::h(1000); #1 $display(\"P=%0d v=%0d\", P, v); $finish; end
endmodule\n"
    );
    assert_eq!(clean(&src), "P=232 v=232");
}

/// `p` imports `r::h`, whose body reads `K`; `p` declares its own `K`. The window
/// does not answer an imported callee, so `h` runs in `r` (PRE's resolution through
/// the module's own import) and reads `r`'s 8, not `p`'s 3. Before the round-1 fix
/// the window filed `h` under `p`: `P=7`. = PRE = iverilog, verilator, sv2v.
const R_H_P_K3: &str = "package r;
  localparam int K = 8;
  function automatic int h(); return K; endfunction
endpackage
package p;
  import r::h;
  localparam int K = 3;
  function automatic logic [h()-1:0] f(); return '1; endfunction
endpackage
";

/// Interpreter header (L1) over an imported callee. `P=255` (see `R_H_P_K3`).
#[test]
fn imported_callee_header_runs_in_its_own_package() {
    let src = format!(
        "{R_H_P_K3}module top;
  import r::*;
  localparam longint P = p::f();
  initial begin #1 $display(\"P=%0d\", P); $finish; end
endmodule\n"
    );
    assert_eq!(clean(&src), "P=255");
}

/// `$bits` of the call (L8, the reserved return width). `b=8` (see `R_H_P_K3`).
#[test]
fn imported_callee_bits_runs_in_its_own_package() {
    let src = format!(
        "{R_H_P_K3}module top;
  import r::*;
  int b;
  initial begin #1 b = $bits(p::f()); $display(\"b=%0d\", b); $finish; end
endmodule\n"
    );
    assert_eq!(clean(&src), "b=8");
}

/// Inline lane (L5) twin. `v=255` = PRE = iverilog, verilator, sv2v (round 1: `v=7`).
#[test]
fn imported_callee_inline_runs_in_its_own_package() {
    let src = "package r;
  localparam int K = 8;
  function automatic int h(); return K; endfunction
endpackage
package p;
  import r::h;
  localparam int K = 3;
  logic [31:0] pv = 32'hFFFF_FFFF;
  function logic [h()-1:0] f(); f = pv; endfunction
endpackage
module top;
  import r::*;
  logic [31:0] v;
  initial begin #1 v = p::f(); $display(\"v=%0d\", v); $finish; end
endmodule\n";
    assert_eq!(clean(src), "v=255");
}

/// The wildcard spelling: `p` imports `q::*`, `q::g` reads `q`'s `K = 3`, `p`
/// declares `K = 9`. Frame reserve and `$bits`: `v=15 b=4` = PRE = iverilog,
/// verilator, sv2v (round 1: `v=1023 b=10`).
#[test]
fn wildcard_imported_callee_header_runs_in_its_own_package() {
    let src = "package q;
  localparam int K = 3;
  function automatic int g(); return K; endfunction
endpackage
package p;
  import q::*;
  localparam int K = 9;
  function automatic logic [g():0] h(); h = '1; endfunction
endpackage
module top;
  import q::*;
  int v, b;
  initial begin v = p::h(); b = $bits(p::h()); $display(\"v=%0d b=%0d\", v, b); end
endmodule\n";
    assert_eq!(clean(src), "v=15 b=4");
}

/// Body lane over a wildcard-imported callee: `{g(){1'b1}}` in `p::f2`. `v=7` = PRE
/// = iverilog, verilator, sv2v (round 1: `v=511`).
#[test]
fn imported_callee_body_lane_runs_in_its_own_package() {
    let src = "package q;
  localparam int K = 3;
  function automatic int g(); return K; endfunction
endpackage
package p;
  import q::*;
  localparam int K = 9;
  function automatic logic [31:0] f2(); return {g(){1'b1}}; endfunction
endpackage
module top;
  import q::*;
  int v;
  initial begin v = p::f2(); $display(\"v=%0d\", v); end
endmodule\n";
    assert_eq!(clean(src), "v=7");
}

/// Interpreter default `a = g()` over a wildcard-imported callee: `L=3` = PRE =
/// iverilog (verilator: `Duplicate declaration of function: 'h4__Vtcwrap_1'`; sv2v:
/// `has been called with missing/empty parameters`); round 1 gave `L=9`. The
/// run-time `v=9` is a pre-existing wrong on PRE too (iverilog `v=3`): the run-time
/// default lane binds `p`'s copy (ROADMAP §2 "Scoping", the import-origin line).
#[test]
fn imported_callee_default_runs_in_its_own_package() {
    let src = "package q;
  localparam int K = 3;
  function automatic int g(); return K; endfunction
endpackage
package p;
  import q::*;
  localparam int K = 9;
  function automatic int h4(input int a = g()); return a; endfunction
endpackage
module top;
  import q::*;
  localparam int L = p::h4();
  int v;
  initial begin v = p::h4(); $display(\"L=%0d v=%0d\", L, v); end
endmodule\n";
    assert_eq!(clean(src), "L=3 v=9");
}

/// KNOWN ILLEGAL, kept at PRE. `p` declares a TASK `h` and wildcard-imports `r`'s
/// FUNCTION `h`, which `elaborate_package` files in `p`'s function table. A declared
/// task does not make that function `p`'s own (`pkg_owns(.., RtnKind::Func)`), so the
/// window does not answer it and the header keeps PRE's resolution: `v=255`. Round 2
/// answered it tagged `p`: `v=7`. All three oracles reject the design: iverilog
/// `error: No function named `h' found in this context (p).`, verilator `Cannot call a
/// task/void-function as a function: 'h'`, sv2v → iverilog `error: No function named
/// `p_h' found in this context (top).`
#[test]
fn declared_task_does_not_own_an_imported_function() {
    let src = "package r;
  localparam int K = 8;
  function automatic int h(); return K; endfunction
endpackage
package p;
  import r::*;
  localparam int K = 3;
  task h(); endtask
  function automatic logic [h()-1:0] f(); return '1; endfunction
endpackage
module top;
  import r::*;
  logic [31:0] v;
  initial begin #1 v = p::f(); $display(\"v=%0d\", v); $finish; end
endmodule\n";
    assert_eq!(clean(src), "v=255");
}

/// The same with a task that has an output formal, through the frame reserve and
/// `$bits`: `v=15 b=4` = PRE (round 2: `v=1023 b=10`). All three oracles reject the
/// design: iverilog `error: No function named `g' found in this context (p).`,
/// verilator `Missing argument on non-defaulted argument 'o' in function call to TASK
/// 'g'`, sv2v → iverilog `error: No function named `p_g' found in this context (top).`
#[test]
fn declared_task_does_not_own_an_imported_function_in_the_reserve() {
    let src = "package q;
  localparam int K = 3;
  function automatic int g(); return K; endfunction
endpackage
package p;
  import q::*;
  localparam int K = 9;
  task automatic g(output int o); o = 1; endtask
  function automatic logic [g():0] h(); h = '1; endfunction
endpackage
module top;
  import q::*;
  int v, b;
  initial begin v = p::h(); b = $bits(p::h()); $display(\"v=%0d b=%0d\", v, b); end
endmodule\n";
    assert_eq!(clean(src), "v=15 b=4");
}

/// A header re-entry earlier in the design (`P`, kept at PRE) must not stop a later,
/// unrelated correction (`Q`): the re-entry flag is reset after its own split.
/// `P=8` has no oracle (iverilog asserts `elab_expr.cc:2927`, verilator crashes,
/// sv2v `P=0`); `Q=15` = iverilog, verilator, sv2v on the `Q`-only twin. PRE
/// `P=8 Q=255`.
#[test]
fn reentry_does_not_block_a_later_correction() {
    let src = "package q;
  localparam int W = 3;
  function automatic logic [f(2):0] f(input int a); return a; endfunction
  function automatic logic [f(2):0] h(input int x); return x; endfunction
  function automatic logic [W:0] g(); g = '1; endfunction
endpackage
module top;
  function automatic int f(input int a); return 3; endfunction
  localparam int W = 7;
  localparam int P = q::h(1000);
  localparam int Q = q::g();
  initial begin #1 $display(\"P=%0d Q=%0d\", P, Q); $finish; end
endmodule\n";
    assert_eq!(clean(src), "P=8 Q=15");
}

/// The body lane arms only an OWNED routine's body: `p::h`'s body calls `g`, which
/// `p` imports from `q`; `g`'s body `{kk(){1'b1}}` must keep resolving `kk` in `q`.
/// `v=7` = PRE = iverilog, verilator, sv2v.
#[test]
fn body_lane_skips_an_imported_body() {
    let src = "package q;
  localparam int K = 3;
  function automatic int kk(); return K; endfunction
  function automatic logic [31:0] g(); return {kk(){1'b1}}; endfunction
endpackage
package p;
  import q::*;
  localparam int K = 9;
  function automatic logic [31:0] h(); return g(); endfunction
endpackage
module top;
  import q::*;
  int v;
  initial begin v = p::h(); $display(\"v=%0d\", v); end
endmodule\n";
    assert_eq!(clean(src), "v=7");
}

/// The same rule where it is observable: `g` is `q`'s, imported into `p`, and `p`
/// DECLARES its own `f`. An owned-body window over `g`'s body would bind `p`'s `f`
/// (`v=511`); `g`'s body keeps PRE's resolution, the module's import of `q`.
/// `v=7` = PRE = iverilog, verilator, sv2v.
#[test]
fn body_lane_skips_an_imported_body_over_an_owned_name() {
    let src = "package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic logic [31:0] g(); return {f(2){1'b1}}; endfunction
endpackage
package p;
  import q::*;
  function automatic int f(input int a); return 9; endfunction
  function automatic logic [31:0] h(); return g(); endfunction
endpackage
module top;
  import q::*;
  int v;
  initial begin v = p::h(); $display(\"v=%0d\", v); end
endmodule\n";
    assert_eq!(clean(src), "v=7");
}

/// The kept package bindings carry the declared width (`param_meta`) as well as the
/// value: `$bits(W)` in the return range reads the package's 5-bit `W`, not the
/// caller's 8-bit one. `v=31 b=5` = iverilog, verilator, sv2v; PRE `v=255 b=8`.
#[test]
fn kept_binding_carries_declared_width() {
    let src = "package p;
  localparam logic [4:0] W = 5'd2;
  function automatic logic [$bits(W)-1:0] h(); h = '1; endfunction
endpackage
module top;
  localparam logic [7:0] W = 8'd2;
  int v, b;
  initial begin v = p::h(); b = $bits(p::h()); $display(\"v=%0d b=%0d\", v, b); end
endmodule\n";
    assert_eq!(clean(src), "v=31 b=5");
}

/// The self-referential and mutually recursive headers with no caller `f`: PRE
/// declines first, so the window is never tried. = PRE (E3009).
#[test]
fn recursive_headers_stay_loud() {
    loud(
        "package q;
  function automatic logic [f(2):0] f(input int a); return a; endfunction
endpackage
module top;
  localparam int P = q::f(3);
  initial begin #1 $display(\"P=%0d\", P); $finish; end
endmodule\n",
        "VITA-E3009",
    );
    loud(
        "package q;
  function automatic logic [g(2):0] f(input int a); return a; endfunction
  function automatic logic [f(2):0] g(input int a); return a; endfunction
endpackage
module top;
  int v;
  initial begin v = q::f(3); #1 $display(\"v=%0d\", v); $finish; end
endmodule\n",
        "VITA-E3009",
    );
}

/// A body-local enum label shadows the package's same-named constant inside the
/// routine; the probe never answers a name the routine declares, so PRE's fold
/// stands. KNOWN WRONG, kept at PRE: vita `P=232` (the caller's `W`); the oracles
/// split — iverilog `P=8` (the package's `W`), verilator refuses the localparam
/// (`Expecting expression to be constant, but can't determine constant for FUNCREF
/// 'h'`) and prints `v=0` (the label) for the run-time twin; sv2v does not parse a
/// function-local typedef.
#[test]
fn routine_declared_name_is_not_probed() {
    let src = "package q;
  localparam int W = 3;
  function automatic int h(input int x);
    typedef enum {A, B, W} e_t;
    logic [W:0] t;
    t = x;
    return t;
  endfunction
endpackage
module top;
  localparam int W = 7;
  localparam int P = q::h(1000);
  initial begin #1 $display(\"P=%0d\", P); $finish; end
endmodule\n";
    assert_eq!(clean(src), "P=232");
}

// ---------------------------------------------------------------------------
// REFUSED: PRE could not fold the unit, so it stays loud (ROADMAP §3.b
// `pkg-text-open`). Each pin keeps the oracles' text beside it.
// ---------------------------------------------------------------------------

/// REFUSED. No caller `f`: opening the header would hand the interpreter's value to
/// every constant consumer. Oracles: iverilog, verilator, sv2v `P=8`.
#[test]
fn refused_header_with_no_caller_binding() {
    loud(
        &format!(
            "{Q_F3}module top;
  localparam int P = q::h(1000);
  initial begin #1 $display(\"P=%0d\", P); $finish; end
endmodule\n"
        ),
        "parameter `P` value is not a constant",
    );
}

/// REFUSED. A package constant the caller does not declare, in a run-time return
/// range. Oracles: iverilog, verilator, sv2v `v=8`.
#[test]
fn refused_reserve_with_no_caller_binding() {
    loud(
        "package q;
  localparam int W = 3;
  function automatic logic [W:0] h(input int x); return x; endfunction
endpackage
module top;
  int v;
  initial begin v = q::h(1000); $display(\"v=%0d\", v); #1 $finish; end
endmodule\n",
        "undefined name `W` is not allowed in a constant range bound",
    );
}

/// REFUSED. A generate-if condition. Oracles: iverilog, verilator, sv2v `br=then`.
#[test]
fn refused_generate_condition() {
    loud(
        &format!(
            "{Q_F3}module top;
  if (q::h(18) == 4'd2) begin : g
    initial #1 $display(\"br=then\");
  end else begin : e
    initial #1 $display(\"br=else\");
  end
  initial #2 $finish;
endmodule\n"
        ),
        "VITA-E3010",
    );
}

/// REFUSED. The body returns a never-assigned 4-state local: opened, the
/// interpreter would answer 0 (🆕 AE). Oracles: iverilog, verilator, sv2v `P=xxxx`.
#[test]
fn refused_never_assigned_local() {
    loud(
        "package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic logic [f(2):0] h(input int x);
    logic [3:0] t;
    h = t;
  endfunction
endpackage
module top;
  localparam logic [3:0] P = q::h(2);
  initial begin #1 $display(\"P=%b\", P); $finish; end
endmodule\n",
        "parameter `P` value is not a constant",
    );
}

/// REFUSED. A replication count over a call whose body has a `case`: opened, the
/// frame would reserve and 🆕 AC's fallback would answer `w=00000000`. Oracles:
/// iverilog, verilator, sv2v `w=00000007`.
#[test]
fn refused_replication_over_case_body() {
    loud(
        "package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic logic [f(2):0] h(input int x);
    case (x)
      2: h = 4'd3;
      default: h = 4'd1;
    endcase
  endfunction
endpackage
module top;
  wire [31:0] w;
  assign w = {4'd0, {q::h(2){1'b1}}};
  initial begin #1 $display(\"w=%h\", w); $finish; end
endmodule\n",
        "VITA-E3009",
    );
}

/// REFUSED. A select write into a local whose initializer did not fold (the
/// "Constant domain" OPEN line): opened, `P=0`. Oracles: iverilog, verilator,
/// sv2v `P=2`.
#[test]
fn refused_select_write_into_unfolded_local() {
    loud(
        "package q;
  localparam int W = 31;
  function automatic logic [W:0] h(input int a);
    int t = int'(2.5);
    t[0] = 1'b0;
    h = t;
  endfunction
endpackage
module top;
  localparam int P = q::h(0);
  initial begin #1 $display(\"P=%0d\", P); $finish; end
endmodule\n",
        "parameter `P` value is not a constant",
    );
}
