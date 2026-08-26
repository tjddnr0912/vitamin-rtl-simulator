//! `%p` (IEEE 1800 §21.2.1.7 — the assignment-pattern form) had no test at all,
//! and iverilog 13.0 does not implement the spec, so differential testing could
//! never have caught a regression here. A fresh-area sweep found it rendering
//! EVERY argument through the integer path: `$display("%p", 2.5)` printed `3`,
//! silently discarding the value. A real now renders as a real (`%g` — the
//! shortest spelling that reads back as the same number), which is what an
//! assignment pattern of a real looks like.
//!
//! The RESIDUAL this file used to record — "a STRING renders as its packed byte
//! value and an UNPACKED STRUCT as its fields concatenated" — is half closed and
//! half re-diagnosed, both by V34-5 (`fmt_p_aggregate.rs`, which owns the
//! aggregate half of `%p` and the oracle measurement behind it):
//!
//! * a `string` VARIABLE now renders as `"hi"`, because the renderer can see
//!   `Value::is_str` after all — what it could not see was the AGGREGATE, and
//!   fixing that put a real domain test in front of the packed fallback. A string
//!   LITERAL is still a packed value under `%p` (`"abc"` -> 6382179), which is
//!   what verilator prints too, measured;
//! * the UNPACKED STRUCT was never a `%p` gap. vita cannot DECLARE one at all
//!   (`u_t u;` is E3010, "undeclared net/variable"), so there is no net for `%p`
//!   to render from. A declaration gap, filed as such.
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

/// A packed struct is a bit vector, so its pattern form is its value.
///
/// The second half of this test used to assert that `%p` of an unpacked array
/// stays LOUD "rather than inventing" a format. That was the right call while the
/// only oracle had not been measured; V34-5 measured it (iverilog does not
/// implement `%p` at all, verilator does), so the array case moved to
/// `fmt_p_aggregate.rs` with verilator's format and lives there as a VALUE
/// assertion. What is pinned here is the half that did not move.
#[test]
fn p_spec_on_packed_shapes() {
    let (out, c) = run(
        "module m;\n  typedef struct packed { logic [3:0] a; logic [3:0] b; } sp_t;\n\
           sp_t sp;\n  initial begin sp.a = 4'h3; sp.b = 4'hC;\n\
             $display(\"S=%p 0S=%0p\", sp, sp); #1 $finish; end\nendmodule\n",
    );
    assert_eq!(c, Some(0), "packed struct; got:\n{out}");
    assert!(
        out.contains("S=60 0S='h3c"),
        "packed struct is its value under `%p`, its hex under `%0p` \
         (verilator-matched); got:\n{out}"
    );
}
