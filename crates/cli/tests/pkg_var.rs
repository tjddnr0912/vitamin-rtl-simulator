//! A2b-prereq — package-level VARIABLE storage (IEEE 1800 §26).
//!
//! Live oracle: iverilog 13.0 supports package variables — every PASS
//! expectation below is pinned to a live iverilog run (2026-07-02):
//!   - one storage instance per elaboration, shared by every module,
//!   - const-foldable decl-init visible at t0 (before module procs),
//!   - `import p::*` bare-name access (read AND write),
//!   - explicit `import p::name`,
//!   - `p::name` scoped read without an import,
//!   - a LOCAL declaration shadows a WILDCARD import; `p::name` still sees
//!     the package storage,
//!   - explicit import + local declaration of the same name = loud error,
//!   - two wildcard imports of the same name, referenced = loud error,
//!   - `p::name = …` as an LVALUE is a syntax error (iverilog rejects it too),
//!   - unpacked-array vars: element read/write; 2-state `int` defaults to 0,
//!   - bare `$dumpvars` does NOT dump package vars (iverilog parity — it
//!     declares none either); explicitly selecting one warns (W4026) instead
//!     of a silent no-op (iverilog asserts/crashes on that input).
//!
//! v1 loud line (honest-loud, never silent): non-const initializers, array
//! `'{…}` initializers (A2b), wire kinds, event/string/real/class/dyn storage.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, i32) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_pkgvar_{}_{n}", std::process::id()));
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
        out.status.code().unwrap_or(-1),
    )
}

/// Like `run`, but also returns the produced `dump.vcd` (empty if none).
fn run_vcd(src: &str) -> (String, String, i32, String) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_pkgvar_vcd_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let vcd = std::fs::read_to_string(d.join("dump.vcd")).unwrap_or_default();
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code().unwrap_or(-1),
        vcd,
    )
}

// ── PASS cases (iverilog-pinned stdout) ─────────────────────────────

#[test]
fn init_read_write_and_scoped_read() {
    // iverilog: "cnt=5 base=a0" / "cnt2=6" / "qual=6" — the scoped read sees
    // the SAME storage the imported bare name mutated (alias, not a copy).
    let (o, e, c) = run(
        "package p;\n  int cnt = 5;\n  logic [7:0] base = 8'hA0;\nendpackage\n\
         module top; import p::*;\n  initial begin\n    $display(\"cnt=%0d base=%h\", cnt, base);\n    cnt = cnt + 1;\n    $display(\"cnt2=%0d\", cnt);\n    $display(\"qual=%0d\", p::cnt);\n    $finish;\n  end\nendmodule\n",
    );
    assert_eq!(c, 0, "must elaborate clean:\n{e}");
    assert_eq!(
        o,
        "cnt=5 base=a0\ncnt2=6\nqual=6\nsimulation ended (Finish) at time 0\n"
    );
}

#[test]
fn single_storage_shared_across_modules() {
    // iverilog: t1 sees 0 (writer runs at #2), t3 sees 77 — ONE instance per
    // elaboration (IEEE §26), not per importing module.
    let (o, e, c) = run(
        "package p; int cnt = 0; endpackage\n\
         module writer; import p::*;\n  initial #2 cnt = 77;\nendmodule\n\
         module reader; import p::*;\n  initial begin #1 $display(\"t1 cnt=%0d\", cnt); #2 $display(\"t3 cnt=%0d\", cnt); end\nendmodule\n\
         module top;\n  writer w(); reader r();\n  initial #5 $finish;\nendmodule\n",
    );
    assert_eq!(c, 0, "must elaborate clean:\n{e}");
    assert_eq!(
        o,
        "t1 cnt=0\nt3 cnt=77\nsimulation ended (Finish) at time 5\n"
    );
}

#[test]
fn local_decl_shadows_wildcard_import() {
    // iverilog: "local=99 pkg=5" — the local variable wins the bare name; the
    // scoped form still reads the package storage.
    let (o, e, c) = run(
        "package p; int cnt = 5; endpackage\n\
         module top; import p::*;\n  int cnt = 99;\n  initial begin $display(\"local=%0d pkg=%0d\", cnt, p::cnt); $finish; end\nendmodule\n",
    );
    assert_eq!(c, 0, "must elaborate clean:\n{e}");
    assert_eq!(o, "local=99 pkg=5\nsimulation ended (Finish) at time 0\n");
}

#[test]
fn local_param_shadows_wildcard_import_read() {
    // iverilog: "read=3 pkg=5" — a local CONSTANT also wins the bare-name read.
    let (o, e, c) = run(
        "package p; int cnt = 5; endpackage\n\
         module top; import p::*;\n  localparam int cnt = 3;\n  initial begin $display(\"read=%0d pkg=%0d\", cnt, p::cnt); $finish; end\nendmodule\n",
    );
    assert_eq!(c, 0, "must elaborate clean:\n{e}");
    assert_eq!(o, "read=3 pkg=5\nsimulation ended (Finish) at time 0\n");
}

#[test]
fn explicit_import_and_scoped_read_without_import() {
    // iverilog: "cnt=15 qual=15" and "z=6".
    let (o, e, c) = run(
        "package p; int cnt = 5; endpackage\n\
         module top; import p::cnt;\n  initial begin cnt = cnt + 10; $display(\"cnt=%0d qual=%0d\", cnt, p::cnt); $finish; end\nendmodule\n",
    );
    assert_eq!(c, 0, "must elaborate clean:\n{e}");
    assert_eq!(o, "cnt=15 qual=15\nsimulation ended (Finish) at time 0\n");
    let (o, e, c) = run(
        "package p; int cnt = 5; endpackage\n\
         module top;\n  int z;\n  initial begin z = p::cnt + 1; $display(\"z=%0d\", z); $finish; end\nendmodule\n",
    );
    assert_eq!(c, 0, "must elaborate clean:\n{e}");
    assert_eq!(o, "z=6\nsimulation ended (Finish) at time 0\n");
}

#[test]
fn array_var_element_rw_and_two_state_default() {
    // iverilog: "arr1=11 arr2=22 arr0=0" — 2-state `int` elements default to
    // 0; element writes land in the shared package storage.
    let (o, e, c) = run(
        "package p; int arr[0:3]; endpackage\n\
         module top; import p::*;\n  initial begin\n    arr[1] = 11; arr[2] = 22;\n    $display(\"arr1=%0d arr2=%0d arr0=%0d\", arr[1], arr[2], arr[0]);\n    $finish;\n  end\nendmodule\n",
    );
    assert_eq!(c, 0, "must elaborate clean:\n{e}");
    assert_eq!(
        o,
        "arr1=11 arr2=22 arr0=0\nsimulation ended (Finish) at time 0\n"
    );
}

#[test]
fn four_state_kinds_and_t0_visibility() {
    // iverilog: "v=ab u=x b=1" — init visible at t0, no-init logic is X,
    // `bit` is 2-state.
    let (o, e, c) = run(
        "package p; logic [7:0] v = 8'hAB; logic [3:0] u; bit b = 1'b1; endpackage\n\
         module top; import p::*;\n  initial begin $display(\"v=%h u=%h b=%b\", v, u, b); $finish; end\nendmodule\n",
    );
    assert_eq!(c, 0, "must elaborate clean:\n{e}");
    assert_eq!(o, "v=ab u=x b=1\nsimulation ended (Finish) at time 0\n");
}

// ── loud cases ──────────────────────────────────────────────────────

#[test]
fn explicit_import_plus_local_decl_is_loud() {
    // iverilog: "error: 'cnt' has already been imported into this scope".
    let (_, e, c) = run("package p; int cnt = 5; endpackage\n\
         module top; import p::cnt;\n  int cnt = 99;\n  initial $finish;\nendmodule\n");
    assert_ne!(c, 0);
    assert!(
        e.contains("already been imported"),
        "explicit-import conflict must be loud:\n{e}"
    );
}

#[test]
fn wildcard_ambiguity_referenced_is_loud() {
    // iverilog: "Ambiguous use of 'v'". vita unbinds the name (the consts
    // machinery's rule) so the reference is a loud undeclared.
    let (_, e, c) = run(
        "package p1; int v = 1; endpackage\npackage p2; int v = 2; endpackage\n\
         module top; import p1::*; import p2::*;\n  initial begin $display(\"%0d\", v); $finish; end\nendmodule\n",
    );
    assert_ne!(c, 0);
    assert!(
        e.contains("VITA-E3010") || e.contains("undeclared"),
        "ambiguous wildcard reference must be loud:\n{e}"
    );
}

#[test]
fn scoped_lvalue_is_loud() {
    // iverilog: syntax error on `p::cnt = 42;` — vita's parser rejects the
    // same form (never a silent drop).
    let (_, e, c) = run("package p; int cnt = 5; endpackage\n\
         module top;\n  initial begin p::cnt = 42; $finish; end\nendmodule\n");
    assert_ne!(c, 0);
    assert!(
        e.contains("VITA-E2002") || e.contains("lvalue"),
        "scoped lvalue must be a loud reject:\n{e}"
    );
}

#[test]
fn nonconst_init_supported_a2b() {
    // A2b: a non-constant package init rides the package's own §6.8
    // pre-sweep initial (ProcId before every module process) — iverilog: y=10.
    // (Was a v1 honest-loud before the package pre-sweep existed.)
    let (o, e, c) = run(
        "package p; int x = 5; int y = x * 2; endpackage\n\
         module top; import p::*;\n  initial begin $display(\"y=%0d\", y); $finish; end\nendmodule\n",
    );
    assert_eq!(c, 0, "must elaborate clean:\n{e}");
    assert_eq!(o, "y=10\nsimulation ended (Finish) at time 0\n");
}

#[test]
fn array_pattern_init_supported_a2b() {
    // A2b: a package array `'{…}` decl-init rides the package pre-sweep —
    // iverilog-pinned element values.
    let (o, e, c) = run(
        "package p; int t[0:2] = '{1, 2, 3}; endpackage\n\
         module top; import p::*;\n  initial begin $display(\"%0d %0d %0d\", t[0], t[1], t[2]); $finish; end\nendmodule\n",
    );
    assert_eq!(c, 0, "must elaborate clean:\n{e}");
    assert_eq!(o, "1 2 3\nsimulation ended (Finish) at time 0\n");
}

#[test]
fn unsupported_kinds_are_loud() {
    for (body, needle) in [
        ("wire w;", "not a package item"),
        ("string s;", "outside the v1 subset"),
        ("real r;", "outside the v1 subset"),
        ("event ev;", "outside the v1 subset"),
        ("int q[$];", "dynamic storage"),
        ("int d[];", "dynamic storage"),
        ("int a[string];", "dynamic storage"),
    ] {
        let (_, e, c) = run(&format!(
            "package p; {body} endpackage\nmodule top; initial $finish;\nendmodule\n"
        ));
        assert_ne!(c, 0, "`{body}` must be loud");
        assert!(e.contains(needle), "`{body}` wrong diagnostic:\n{e}");
    }
}

#[test]
fn param_shadow_write_is_loud() {
    // A local constant shadows the imported variable for READS; a write to
    // the bare name must therefore be loud, never a silent write into the
    // package storage (iverilog also rejects: "Could not find variable").
    let (_, e, c) = run(
        "package p; int cnt = 5; endpackage\n\
         module top; import p::*;\n  localparam int cnt = 3;\n  initial begin cnt = 9; $finish; end\nendmodule\n",
    );
    assert_ne!(c, 0);
    assert!(
        e.contains("not") && e.contains("assignable"),
        "write to a param-shadowed import must be loud:\n{e}"
    );
}

#[test]
fn whole_array_scoped_read_is_loud() {
    let (_, e, c) = run("package p; int arr[0:3]; endpackage\n\
         module top;\n  int z;\n  initial begin z = p::arr; $finish; end\nendmodule\n");
    assert_ne!(c, 0);
    assert!(
        e.contains("whole unpacked array"),
        "whole-array scoped read must be loud:\n{e}"
    );
}

#[test]
fn unknown_var_explicit_import_is_loud() {
    let (_, e, c) = run("package p; int cnt = 5; endpackage\n\
         module top; import p::nope;\n  initial $finish;\nendmodule\n");
    assert_ne!(c, 0);
    assert!(e.contains("has no symbol"), "unknown symbol import:\n{e}");
}

#[test]
fn pkg_var_in_const_context_is_loud() {
    // A variable is not a constant — `localparam L = p::cnt` must not fold.
    let (_, e, c) = run(
        "package p; int cnt = 5; endpackage\n\
         module top;\n  localparam L = p::cnt;\n  initial begin $display(\"L=%0d\", L); $finish; end\nendmodule\n",
    );
    assert_ne!(c, 0);
    assert!(
        e.contains("not a foldable constant"),
        "package var in const context must be loud:\n{e}"
    );
}

// ── VCD surface (v1: excluded — iverilog parity) ────────────────────

#[test]
fn bare_dumpvars_excludes_package_vars() {
    // iverilog's VCD for this design declares ONLY top.local_x — no package
    // scope, no package var. vita mirrors that; the module var still dumps.
    let (_, e, c, vcd) = run_vcd(
        "package p; int cnt = 5; endpackage\n\
         module top; import p::*;\n  int local_x = 1;\n  initial begin $dumpfile(\"dump.vcd\"); $dumpvars; #2 cnt = 9; #1 $finish; end\nendmodule\n",
    );
    assert_eq!(c, 0, "must run clean:\n{e}");
    assert!(!vcd.contains("$pkg$"), "no $pkg$ scope in the VCD:\n{vcd}");
    assert!(
        !vcd.contains("cnt"),
        "package var must not be dumped:\n{vcd}"
    );
    assert!(
        vcd.contains("local_x"),
        "module var must still dump:\n{vcd}"
    );
}

#[test]
fn explicit_dumpvars_of_package_var_warns() {
    // Explicitly selecting a package var is never a silent no-op: W4026 once
    // (iverilog ASSERT-CRASHES on this input — any loud beats that).
    let (_, e, c, vcd) = run_vcd(
        "package p; int cnt = 5; endpackage\n\
         module top; import p::*;\n  initial begin $dumpfile(\"dump.vcd\"); $dumpvars(0, cnt); #2 cnt = 9; #1 $finish; end\nendmodule\n",
    );
    assert_eq!(c, 0, "warn, not error:\n{e}");
    assert!(
        e.contains("VITA-W4026"),
        "explicit selection must warn W4026:\n{e}"
    );
    assert!(!vcd.contains("cnt"), "package var still not dumped:\n{vcd}");
}

// ── adversarial-review regressions (round 1 findings, all fixed) ────

#[test]
fn pkg_var_in_range_bound_is_loud_f1() {
    // Diff F1: `p::x` (a VARIABLE) in a constant range/dimension used to fall
    // into the const-but-unfoldable catch-all and clamp to a SILENT width-1
    // (iverilog: "not allowed in a constant expression"). Now loud.
    let (_, e, c) = run(
        "package p; int x = 5; endpackage\n\
         module top;\n  logic [p::x-1:0] w;\n  initial begin $display(\"%0d\", $bits(w)); $finish; end\nendmodule\n",
    );
    assert_ne!(c, 0);
    assert!(
        e.contains("package variable") && e.contains("constant"),
        "p::var in a range bound must be loud:\n{e}"
    );
    let (_, e, c) = run(
        "package p; int x = 5; endpackage\n\
         module top;\n  int a [p::x];\n  initial begin $display(\"%0d\", $size(a)); $finish; end\nendmodule\n",
    );
    assert_ne!(c, 0, "p::var as an array dimension must be loud:\n{e}");
}

#[test]
fn hier_access_to_imported_var_is_loud_f2() {
    // Diff F2 (IEEE §26.3): an import binds the BARE name lexically — a
    // hierarchical path must never resolve through it (iverilog: "Unable to
    // bind"). Read, write, and element-select lanes.
    let read = "package p; int cnt = 5; endpackage\n\
         module inner; import p::*;\n  initial #1 cnt = 9;\nendmodule\n\
         module top;\n  inner i1();\n  int z;\n  initial begin #2 z = top.i1.cnt; $display(\"%0d\", z); $finish; end\nendmodule\n";
    let (_, e, c) = run(read);
    assert_ne!(c, 0);
    assert!(
        e.contains("undeclared hierarchical name"),
        "hier READ through an import alias must be loud:\n{e}"
    );
    let write = "package p; int cnt = 5; endpackage\n\
         module inner; import p::*;\n  initial #2 $display(\"%0d\", cnt);\nendmodule\n\
         module top;\n  inner i1();\n  initial begin #1 top.i1.cnt = 42; #3 $finish; end\nendmodule\n";
    let (_, e, c) = run(write);
    assert_ne!(c, 0);
    assert!(
        e.contains("undeclared hierarchical write target"),
        "hier WRITE through an import alias must be loud:\n{e}"
    );
    let elem = "package p; int arr[4]; endpackage\n\
         module inner; import p::*;\n  initial arr[1] = 7;\nendmodule\n\
         module top;\n  inner i1();\n  int z;\n  initial begin #1 z = top.i1.arr[1]; $display(\"%0d\", z); $finish; end\nendmodule\n";
    let (_, e, c) = run(elem);
    assert_ne!(c, 0);
    assert!(
        e.contains("undeclared hierarchical name"),
        "hier ELEMENT read through an import alias must be loud:\n{e}"
    );
}

#[test]
fn const_shadowed_array_funnels_are_loud_s1() {
    // Sound S1: the whole-array / pattern / element lvalue funnels bypass
    // `resolve_net` — each must still refuse to write a package array whose
    // name a local constant shadows (iverilog: "Could not find variable").
    for stmt in ["mem = loc;", "mem = '{1,2,3,4};", "mem[0] = 5;"] {
        let (_, e, c) = run(&format!(
            "package p; int mem[4]; endpackage\n\
             module top; import p::*;\n  localparam int mem = 7;\n  int loc[4];\n  initial begin {stmt} $display(\"done\"); $finish; end\nendmodule\n"
        ));
        assert_ne!(c, 0, "`{stmt}` must be loud");
        assert!(
            e.contains("not") && e.contains("assignable"),
            "`{stmt}` wrong diagnostic:\n{e}"
        );
    }
}

#[test]
fn genvar_shadow_is_loud_s2() {
    // Sound S2: a genvar binds into `params` only during unroll — the
    // persistent genvar record must still shadow the import for procedural
    // code (iverilog: "Unable to bind"). Write and element-read lanes.
    let (_, e, c) = run(
        "package p; int x = 0; endpackage\n\
         module top; import p::*;\n  genvar x;\n  initial begin x = 9; $display(\"%0d\", p::x); $finish; end\nendmodule\n",
    );
    assert_ne!(c, 0);
    assert!(
        e.contains("not") && e.contains("assignable"),
        "genvar-shadowed write must be loud:\n{e}"
    );
    let (_, e, c) = run(
        "package p; int x[4]; endpackage\n\
         module top; import p::*;\n  genvar x;\n  int z;\n  initial begin z = x[0]; $display(\"%0d\", z); $finish; end\nendmodule\n",
    );
    assert_ne!(c, 0, "genvar-shadowed element read must be loud:\n{e}");
}

#[test]
fn explicit_var_import_wins_over_wildcard_const_s4() {
    // Sound S4 (IEEE §26.8): an explicit import beats a wildcard binding —
    // iverilog-pinned "x=8 / x2=9" (the pb::x VARIABLE wins over pa's
    // wildcard CONST, and stays writable).
    let (o, e, c) = run(
        "package pa; localparam int x = 3; endpackage\n\
         package pb; int x = 8; endpackage\n\
         module top; import pa::*; import pb::x;\n  initial begin $display(\"x=%0d\", x); x = x + 1; $display(\"x2=%0d\", x); $finish; end\nendmodule\n",
    );
    assert_eq!(c, 0, "must elaborate clean:\n{e}");
    assert_eq!(o, "x=8\nx2=9\nsimulation ended (Finish) at time 0\n");
}
