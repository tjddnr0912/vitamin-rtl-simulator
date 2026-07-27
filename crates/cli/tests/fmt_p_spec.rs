//! `%p` (IEEE 1800 §21.2.1.7 — the assignment-pattern form) had no test at all,
//! and iverilog 13.0 does not implement the spec, so differential testing could
//! never have caught a regression here. A fresh-area sweep found it rendering
//! EVERY argument through the integer path: `$display("%p", 2.5)` printed `3`,
//! silently discarding the value. A real now renders as a real (`%g` — the
//! shortest spelling that reads back as the same number), which is what an
//! assignment pattern of a real looks like.
//!
//! RESIDUAL, recorded in ROADMAP §3 and pinned below so the current behavior is
//! visible rather than assumed: a STRING renders as its packed byte value and an
//! UNPACKED STRUCT as its fields concatenated, where IEEE wants `"hi"` and
//! `'{x:7, y:-2}`. Neither is distinguishable from an ordinary packed value where
//! the renderer runs — `Value` carries `is_real` but no string/struct marker — so
//! those need type information this layer never receives.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_fmtp_{}_{n}", std::process::id()));
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

/// A real keeps its value under `%p`, and matches what `%g` prints.
#[test]
fn p_spec_renders_a_real_as_a_real() {
    let (out, c) = run("module m;\n  real a, b, c;\n  initial begin\n\
           a = 2.5; b = -0.125; c = 1000000.0;\n\
           $display(\"P=%p %p %p\", a, b, c);\n\
           $display(\"G=%g %g %g\", a, b, c);\n\
           #1 $finish; end\nendmodule\n");
    assert_eq!(c, Some(0), "no diagnostics expected; got:\n{out}");
    assert!(
        out.contains("P=2.5 -0.125 1e+06"),
        "%p of a real must not round to an integer; got:\n{out}"
    );
    // The two spellings agree — that equivalence is the property, not the digits.
    let p = out.lines().find(|l| l.starts_with("P=")).unwrap();
    let g = out.lines().find(|l| l.starts_with("G=")).unwrap();
    assert_eq!(
        p.trim_start_matches("P="),
        g.trim_start_matches("G="),
        "%p and %g must agree on a real; got:\n{out}"
    );
}

/// Integrals were already right and must stay byte-identical.
#[test]
fn p_spec_leaves_integrals_alone() {
    let (out, c) = run(
        "module m;\n  int i; logic [7:0] v; logic [3:0] n;\n  initial begin\n\
           i = -5; v = 8'hA5; n = 4'b1010;\n\
           $display(\"I=%p %p %p\", i, v, n);\n\
           $display(\"D=%0d %0d %0d\", i, v, n);\n\
           #1 $finish; end\nendmodule\n",
    );
    assert_eq!(c, Some(0), "no diagnostics expected; got:\n{out}");
    assert!(out.contains("I=-5 165 10"), "integral %p; got:\n{out}");
}

/// A packed struct is a bit vector, so its pattern form is its value — and the
/// aggregate shapes that have no whole-value surface stay LOUD rather than
/// inventing one.
#[test]
fn p_spec_on_packed_and_aggregate_shapes() {
    let (out, c) = run(
        "module m;\n  typedef struct packed { logic [3:0] a; logic [3:0] b; } sp_t;\n\
           sp_t sp;\n  initial begin sp.a = 4'h3; sp.b = 4'hC;\n\
             $display(\"S=%p\", sp); #1 $finish; end\nendmodule\n",
    );
    assert_eq!(c, Some(0), "packed struct; got:\n{out}");
    assert!(
        out.contains("S=60"),
        "packed struct is its value; got:\n{out}"
    );

    // An unpacked array has no whole value here; `%p` must not fabricate one.
    let (out, c) = run("module m;\n  int arr [3];\n  initial begin arr[0]=1;\n\
           $display(\"%p\", arr); #1 $finish; end\nendmodule\n");
    assert_ne!(
        c,
        Some(0),
        "%p of an unpacked array must stay loud; got:\n{out}"
    );
}
