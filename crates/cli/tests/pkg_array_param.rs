//! A2b — package-level array parameter (IEEE 1800 §6.20.2 + §26) and the
//! package §6.8 pre-sweep that powers every non-net.init package initializer.
//!
//! Oracles:
//! - The array-parameter FORM has no live oracle (iverilog 13.0: "sorry:
//!   unpacked array parameters are not supported yet") → the teeth is the
//!   vita-INTERNAL twin differential: `localparam <ty> T[dims] = '{…}` in a
//!   package must be byte-identical on stdout to the equivalent package
//!   VARIABLE `<ty> T[dims] = '{…}` — and that var twin IS iverilog-checked
//!   live (package array vars with '{…}' inits print identical values).
//! - The pre-sweep mechanism (array '{…}' inits, non-const scalar inits,
//!   chained/mixed inits, t0 visibility) is iverilog-pinned (2026-07-03).
//!
//! Ordering pin: the package pre-sweep initial is emitted while packages
//! elaborate (before any instance), so its ProcId precedes every module
//! process — a module `initial` with NO delay observes initialized values.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, i32) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_pkgaparam_{}_{n}", std::process::id()));
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

/// Twin differential: the package array PARAM form must be byte-identical
/// (stdout) to the package array VARIABLE form.
fn assert_pkg_twin(reads: &str, param_decl: &str, var_decl: &str) {
    let p = format!(
        "package p; {param_decl} endpackage\nmodule top; import p::*;\n  initial begin {reads} $finish; end\nendmodule\n"
    );
    let v = format!(
        "package p; {var_decl} endpackage\nmodule top; import p::*;\n  initial begin {reads} $finish; end\nendmodule\n"
    );
    let (po, pe, pc) = run(&p);
    let (vo, _, vc) = run(&v);
    assert_eq!(pc, 0, "param form must elaborate clean:\n{pe}");
    assert_eq!(vc, 0, "var twin must elaborate clean");
    assert_eq!(
        po, vo,
        "package param form ≠ var twin (byte-identity broken)"
    );
}

// ── acceptance: the sha3_pkg RC_TABLE shape (ROADMAP §6 report CLOSE) ──

#[test]
fn rc_table_24_entry_acceptance() {
    let (o, e, c) = run(
        "package sha3_pkg;\n  localparam logic [63:0] RC_TABLE [0:23] = '{\n    64'h0000000000000001, 64'h0000000000008082, 64'h800000000000808A, 64'h8000000080008000,\n    64'h000000000000808B, 64'h0000000080000001, 64'h8000000080008081, 64'h8000000000008009,\n    64'h000000000000008A, 64'h0000000000000088, 64'h0000000080008009, 64'h000000008000000A,\n    64'h000000008000808B, 64'h800000000000008B, 64'h8000000000008089, 64'h8000000000008003,\n    64'h8000000000008002, 64'h8000000000000080, 64'h000000000000800A, 64'h800000008000000A,\n    64'h8000000080008081, 64'h8000000000008080, 64'h0000000080000001, 64'h8000000080008008\n  };\nendpackage\nmodule top; import sha3_pkg::*;\n  initial begin\n    $display(\"rc0=%h rc12=%h rc23=%h n=%0d\", RC_TABLE[0], RC_TABLE[12], RC_TABLE[23], $size(RC_TABLE));\n    $finish;\n  end\nendmodule\n",
    );
    assert_eq!(c, 0, "RC_TABLE must elaborate clean:\n{e}");
    assert_eq!(
        o,
        "rc0=0000000000000001 rc12=000000008000808b rc23=8000000080008008 n=24\nsimulation ended (Finish) at time 0\n"
    );
}

// ── twin differentials (param ≡ var; var is iverilog-checked live) ──

#[test]
fn wide_logic_table_matches_var_twin() {
    assert_pkg_twin(
        "$display(\"%h %h %h %0d\", T[0], T[2], T[3], $size(T));",
        "localparam logic [63:0] T [0:3] = '{64'h1, 64'h8082, 64'h800000000000808A, 64'h8000000080008000};",
        "logic [63:0] T [0:3] = '{64'h1, 64'h8082, 64'h800000000000808A, 64'h8000000080008000};",
    );
}

#[test]
fn int_table_matches_var_twin() {
    assert_pkg_twin(
        "for (int i = 0; i < 5; i++) $display(\"%0d\", T[i]);",
        "localparam int T [0:4] = '{9, -8, 7, 0, 5};",
        "int T [0:4] = '{9, -8, 7, 0, 5};",
    );
}

#[test]
fn unsupported_reads_match_var_twin_loud() {
    // Both forms must be EQUALLY loud on the scoped whole-array read (proves
    // the param form adds zero new semantics over the var storage).
    let reads = "int z; z = p::T; $display(\"%0d\", z);";
    for decl in [
        "localparam int T [0:2] = '{1,2,3};",
        "int T [0:2] = '{1,2,3};",
    ] {
        let (_, e, c) = run(&format!(
            "package p; {decl} endpackage\nmodule top;\n  initial begin {reads} $finish; end\nendmodule\n"
        ));
        assert_ne!(c, 0, "`{decl}` scoped whole-array read must be loud");
        assert!(
            e.contains("whole unpacked array"),
            "`{decl}` wrong diagnostic:\n{e}"
        );
    }
}

// ── write denial (A2a const-param hooks, net-id keyed → work via import) ──

#[test]
fn writes_to_package_array_param_are_loud() {
    for stmt in ["T[0] = 1;", "T = '{4,5,6};", "$readmemh(\"nope.hex\", T);"] {
        let (_, e, c) = run(&format!(
            "package p; localparam int T[0:2] = '{{7,8,9}}; endpackage\n\
             module top; import p::*;\n  initial begin {stmt} $finish; end\nendmodule\n"
        ));
        assert_ne!(c, 0, "`{stmt}` must be loud");
        assert!(
            e.contains("parameter") && (e.contains("cannot") || e.contains("constant")),
            "`{stmt}` wrong diagnostic:\n{e}"
        );
    }
}

// ── the package pre-sweep mechanism (iverilog-pinned) ──

#[test]
fn array_init_visible_at_t0_and_chains() {
    // iverilog: "t0: a1=22 y=23" — the pre-sweep ProcId precedes the module
    // initial; a later scalar init reads an earlier array element.
    let (o, e, c) = run(
        "package p; int arr[0:1] = '{11,22}; int y = arr[1] + 1; endpackage\n\
         module top; import p::*;\n  initial begin $display(\"t0: a1=%0d y=%0d\", arr[1], y); $finish; end\nendmodule\n",
    );
    assert_eq!(c, 0, "must elaborate clean:\n{e}");
    assert_eq!(o, "t0: a1=22 y=23\nsimulation ended (Finish) at time 0\n");
}

#[test]
fn nonconst_array_elements() {
    // iverilog: "a0=3 a1=6" — element exprs read the earlier package var.
    let (o, e, c) = run(
        "package p; int x = 3; int arr[0:1] = '{x, x*2}; endpackage\n\
         module top; import p::*;\n  initial begin $display(\"a0=%0d a1=%0d\", arr[0], arr[1]); $finish; end\nendmodule\n",
    );
    assert_eq!(c, 0, "must elaborate clean:\n{e}");
    assert_eq!(o, "a0=3 a1=6\nsimulation ended (Finish) at time 0\n");
}

#[test]
fn two_packages_each_flush_their_own_presweep() {
    // Scoped ELEMENT reads are a documented loud (㉻) — use imported names
    // from two different modules instead (iverilog-shape-pinned values).
    let (o, e, c) = run("package pa; int arr[0:1] = '{1,2}; endpackage\n\
         package pb; int brr[0:1] = '{3,4}; endpackage\n\
         module ma; import pa::*;\n  initial $display(\"a=%0d\", arr[0]);\nendmodule\n\
         module mb; import pb::*;\n  initial $display(\"b=%0d\", brr[1]);\nendmodule\n\
         module top;\n  ma u1(); mb u2();\n  initial #1 $finish;\nendmodule\n");
    assert_eq!(c, 0, "must elaborate clean:\n{e}");
    assert!(
        o.contains("a=1\n") && o.contains("b=4\n"),
        "both packages' inits must run before module initials:\n{o}"
    );
}

#[test]
fn forward_ref_init_is_documented_leniency() {
    // iverilog REJECTS declaration-after-use inside a package, so there is no oracle;
    // vita lowers by pass, so the pre-sweep runs in DECLARATION order. It used to print
    // a=6, because b's constant was pre-applied at net creation and so sat outside that
    // order — the same expression with a NON-constant `b` printed a=1. §4.5.257 made a
    // constant initializer an ordered assignment like any other, so both spellings now
    // give the declaration-order answer. Documented leniency (F4 family), pinned at the
    // value vita is self-consistent about.
    let (o, e, c) = run(
        "package p; int a = b + 1; int b = 5; endpackage\n\
         module top; import p::*;\n  initial begin $display(\"a=%0d b=%0d\", a, b); $finish; end\nendmodule\n",
    );
    assert_eq!(c, 0, "documented-leniency value must stay clean:\n{e}");
    assert_eq!(o, "a=1 b=5\nsimulation ended (Finish) at time 0\n");
}

#[test]
fn const_context_use_of_package_array_param_folds() {
    // `$size(T)` in a localparam init folds over the imported constant array's
    // captured geometry (§3 ⑤ ⓔ; was a loud "not a constant" wording pin —
    // iverilog rejects unpacked array parameters, verilator answers 5).
    let (o, e, c) = run(
        "package p; localparam int T[0:4] = '{1,2,3,4,5}; endpackage\n\
         module top; import p::*;\n  localparam int N = $size(T);\n  initial begin $display(\"%0d\", N); $finish; end\nendmodule\n",
    );
    assert_eq!(c, 0, "must elaborate clean:\n{e}");
    assert!(o.starts_with("5\n"), "verilator answers 5:\n{o}");
}

#[test]
fn hier_ref_in_package_init_is_loud() {
    // Adversarial sound #2: LRM-illegal (§26.3 — a package sees only its own
    // scope). Was prereq-loud, briefly silent with an oracle-DIVERGENT value
    // (vita read the resolved net, iverilog its own leniency) — now loud
    // again via the hier_resolve `$pkg$` prefix guard. Read + element forms.
    for init in ["top.m + 1", "top.arr[0] + 1"] {
        let (_, e, c) = run(&format!(
            "package p; int y = {init}; endpackage\n\
             module top; import p::*;\n  int m = 3; int arr[0:1];\n  initial begin $display(\"%0d\", y); $finish; end\nendmodule\n"
        ));
        assert_ne!(c, 0, "`{init}` must be loud");
        assert!(
            e.contains("undeclared hierarchical name"),
            "`{init}` wrong diagnostic:\n{e}"
        );
    }
}

#[test]
fn package_function_call_in_init_is_loud() {
    // Adversarial sound #1: a package's own function in a package init is
    // iverilog-supported (follow-on candidate ㉽) — vita is LOUD (the package
    // funcs are not bound into the callable table during the package flush).
    let (_, e, c) = run(
        "package p;\n  function int f(int a); return a * 2; endfunction\n  int x = 6; int y = f(x);\nendpackage\n\
         module top; import p::*;\n  initial begin $display(\"%0d\", y); $finish; end\nendmodule\n",
    );
    assert_ne!(c, 0, "package-func init must stay loud (follow-on):\n{e}");
}
