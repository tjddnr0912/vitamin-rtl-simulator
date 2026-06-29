//! Phase B1 (N7-REST): constrained-random verification — `rand` class members,
//! `constraint` blocks with range/relational constraints, and `obj.randomize()`.
//! Determinism is part of the contract: same design → byte-identical draw sequence
//! on every OS (seeded `dist_uniform`). iverilog 13 does NOT support randomization,
//! so the oracle is IEEE 1800 §18 semantics + determinism + constraint satisfaction.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_crv_{}_{n}", std::process::id()));
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
        out.status.code(),
    )
}

#[test]
fn randomize_respects_range_constraint() {
    // A rand field with a [1:6] range constraint: every randomize() draw must land
    // in range. Loop several draws; all must satisfy the constraint.
    let (out, err, code) = run("class Die;\n\
           rand int v;\n\
           constraint c { v >= 1; v <= 6; }\n\
         endclass\n\
         module top; Die d; integer i; integer ok;\n\
         initial begin\n\
           d = new;\n\
           ok = 1;\n\
           for (i = 0; i < 20; i = i + 1) begin\n\
             d.randomize();\n\
             if (d.v < 1 || d.v > 6) ok = 0;\n\
           end\n\
           $display(\"ok=%0d\", ok);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("ok=1"), "got:\n{out}");
}

#[test]
fn randomize_is_deterministic() {
    // The same program run twice must produce the identical draw sequence (seeded).
    let src = "class P;\n\
           rand int x;\n\
           constraint c { x >= 0; x <= 99; }\n\
         endclass\n\
         module top; P p; integer i;\n\
         initial begin\n\
           p = new;\n\
           for (i = 0; i < 5; i = i + 1) begin\n\
             p.randomize();\n\
             $display(\"x=%0d\", p.x);\n\
           end\n\
           $finish;\n\
         end\n\
         endmodule\n";
    let (out1, _, c1) = run(src);
    let (out2, _, c2) = run(src);
    assert_eq!(c1, Some(0));
    assert_eq!(c2, Some(0));
    assert_eq!(out1, out2, "randomize must be deterministic across runs");
    // and the draws must vary (not all identical) — a real distribution.
    let xs: Vec<&str> = out1.lines().filter(|l| l.starts_with("x=")).collect();
    assert_eq!(xs.len(), 5, "got:\n{out1}");
    assert!(xs.iter().any(|&l| l != xs[0]), "draws must vary:\n{out1}");
}

#[test]
fn randomize_returns_success() {
    // `r = obj.randomize();` returns 1 on success (feasible constraints).
    let (out, err, code) = run("class P;\n\
           rand int x;\n\
           constraint c { x >= 5; x <= 5; }\n\
         endclass\n\
         module top; P p; integer r;\n\
         initial begin\n\
           p = new;\n\
           r = p.randomize();\n\
           $display(\"r=%0d x=%0d\", r, p.x);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    // single-value constraint x==5 -> always 5, success.
    assert!(out.contains("r=1 x=5"), "got:\n{out}");
}

#[test]
fn randomize_multiple_fields_each_in_range() {
    let (out, err, code) = run("class P;\n\
           rand int a;\n\
           rand int b;\n\
           constraint c { a >= 10; a <= 12; b >= 100; b <= 102; }\n\
         endclass\n\
         module top; P p; integer i; integer ok;\n\
         initial begin\n\
           p = new; ok = 1;\n\
           for (i = 0; i < 20; i = i + 1) begin\n\
             p.randomize();\n\
             if (p.a < 10 || p.a > 12 || p.b < 100 || p.b > 102) ok = 0;\n\
           end\n\
           $display(\"ok=%0d\", ok); $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("ok=1"), "got:\n{out}");
}

#[test]
fn randomize_conjunction_in_single_expr() {
    // A single constraint expr with `&&` narrows the field on both sides.
    let (out, err, code) = run("class P;\n\
           rand int x;\n\
           constraint c { x > 3 && x < 7; }\n\
         endclass\n\
         module top; P p; integer i; integer ok;\n\
         initial begin\n\
           p = new; ok = 1;\n\
           for (i = 0; i < 20; i = i + 1) begin\n\
             p.randomize();\n\
             if (p.x <= 3 || p.x >= 7) ok = 0;\n\
           end\n\
           $display(\"ok=%0d\", ok); $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("ok=1"), "got:\n{out}");
}

#[test]
fn randomize_inherited_rand_and_constraint() {
    // A rand field + constraint declared in the BASE applies to a derived instance.
    let (out, err, code) = run("class Base;\n\
           rand int v;\n\
           constraint cb { v >= 1; v <= 4; }\n\
         endclass\n\
         class Der extends Base;\n\
           int tag;\n\
         endclass\n\
         module top; Der d; integer i; integer ok;\n\
         initial begin\n\
           d = new; ok = 1;\n\
           for (i = 0; i < 20; i = i + 1) begin\n\
             d.randomize();\n\
             if (d.v < 1 || d.v > 4) ok = 0;\n\
           end\n\
           $display(\"ok=%0d\", ok); $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("ok=1"), "got:\n{out}");
}

#[test]
fn randomize_wide_field_honors_constraint() {
    // Regression (adversarial hunt): a rand field WIDER than 32 bits must still honor
    // its range constraint — earlier the >32-bit draw path silently dropped [lo,hi].
    let (out, err, code) = run("class C;\n\
           rand bit [40:0] x;\n\
           constraint c { x >= 10; x <= 20; }\n\
         endclass\n\
         module top; C o; integer i; integer oob;\n\
         initial begin\n\
           o = new; oob = 0;\n\
           for (i = 0; i < 100; i = i + 1) begin\n\
             o.randomize();\n\
             if (o.x < 10 || o.x > 20) oob = oob + 1;\n\
           end\n\
           $display(\"oob=%0d\", oob); $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("oob=0"), "got:\n{out}");
}

#[test]
fn randomize_upper_unsigned_bounds_honored() {
    // Regression: a 32-bit field with bounds in the UPPER unsigned half (> i32::MAX)
    // must honor them — earlier `hi > i32::MAX` forced the unconstrained draw.
    let (out, err, code) = run("class C;\n\
           rand bit [31:0] u;\n\
           constraint c { u >= 3000000000; u <= 4000000000; }\n\
         endclass\n\
         module top; C o; integer i; reg [31:0] v; integer bad;\n\
         initial begin\n\
           o = new; bad = 0;\n\
           for (i = 0; i < 100; i = i + 1) begin\n\
             o.randomize(); v = o.u;\n\
             if (v < 3000000000 || v > 4000000000) bad = bad + 1;\n\
           end\n\
           $display(\"bad=%0d\", bad); $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("bad=0"), "got:\n{out}");
}

#[test]
fn randomize_longint_single_sided_constraint() {
    // Regression: a wide signed field with a single-sided constraint (`x < 5`) must
    // honor it (earlier the full-width draw produced ~half over the bound).
    let (out, err, code) = run("class C;\n\
           rand longint x;\n\
           constraint c { x < 5; }\n\
         endclass\n\
         module top; C o; integer i; integer bad;\n\
         initial begin\n\
           o = new; bad = 0;\n\
           for (i = 0; i < 100; i = i + 1) begin\n\
             o.randomize();\n\
             if (o.x >= 5) bad = bad + 1;\n\
           end\n\
           $display(\"bad=%0d\", bad); $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("bad=0"), "got:\n{out}");
}

#[test]
fn randc_visits_each_value_once_per_cycle() {
    // A `randc bit[2:0]` cycles through all 8 values once before repeating
    // (a random permutation) — 8 draws give 8 distinct values.
    let (out, err, code) = run("class P;\n\
           randc bit [2:0] x;\n\
         endclass\n\
         module top; P p; integer i; integer seen[0:7]; integer dup; integer all;\n\
         initial begin\n\
           p = new; for (i=0;i<8;i=i+1) seen[i]=0; dup=0;\n\
           for (i=0;i<8;i=i+1) begin p.randomize(); if (seen[p.x]) dup=1; seen[p.x]=1; end\n\
           all=1; for (i=0;i<8;i=i+1) if (!seen[i]) all=0;\n\
           $display(\"dup=%0d all=%0d\", dup, all); $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(
        out.contains("dup=0 all=1"),
        "randc must visit each value once:\n{out}"
    );
}

#[test]
fn constrained_randc_is_loud() {
    // A constraint on a `randc` field is unsupported in B2 v1 (loud, not silent).
    let (_, err, code) = run("class P;\n\
           randc bit [3:0] x;\n\
           constraint c { x < 5; }\n\
         endclass\n\
         module top; P p; initial begin p = new; p.randomize(); $finish; end endmodule\n");
    assert_ne!(
        code,
        Some(0),
        "a constrained randc must be loud; stderr:\n{err}"
    );
    assert!(err.contains("randc"), "expected a randc diagnostic:\n{err}");
}

#[test]
fn contradictory_constraint_is_loud_rejected() {
    let (_, err, code) = run("class P;\n\
           rand int x;\n\
           constraint c { x >= 10; x <= 5; }\n\
         endclass\n\
         module top; P p; initial begin p = new; p.randomize(); $finish; end endmodule\n");
    assert_ne!(code, Some(0), "must reject contradiction; stderr:\n{err}");
    assert!(
        err.contains("contradictory") || err.contains("empty solution"),
        "expected a contradiction diagnostic:\n{err}"
    );
}

#[test]
fn staged_randomize_carries_sidecar() {
    // The staged vcmp→velab→vrun path must carry the `class_rand` sidecar (the
    // STAGED-DROP class of bug). If it were dropped, randomize() would be a no-op,
    // leaving `v` at its 2-state default 0 → the `v < 1` $fatal fires → non-zero
    // exit. A clean exit 0 proves the sidecar survived the `.velab` trailer.
    let src = "class Die;\n\
           rand int v;\n\
           constraint c { v >= 1; v <= 6; }\n\
         endclass\n\
         module top; Die d; integer i;\n\
         initial begin\n\
           d = new;\n\
           for (i = 0; i < 8; i = i + 1) begin\n\
             d.randomize();\n\
             if (d.v < 1 || d.v > 6) $fatal(1, \"rand out of range\");\n\
           end\n\
           $finish;\n\
         end\n\
         endmodule\n";
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("vita_crvst_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let sv = dir.join("t.sv");
    std::fs::write(&sv, src).unwrap();
    let s = |p: &std::path::Path| p.to_str().unwrap().to_string();
    let vu = dir.join("t.vu");
    let velab = dir.join("t.velab");
    let o = cli::VitaOpts::default();
    assert_eq!(
        cli::run_vcmp(&[s(&sv)], Some(&s(&vu)), &o),
        0,
        "vcmp failed"
    );
    assert_eq!(cli::run_velab(&s(&vu), &s(&velab), &o), 0, "velab failed");
    // Clean exit 0 ⇒ no $fatal ⇒ every draw was in [1,6] ⇒ sidecar carried.
    assert_eq!(
        cli::run_vrun(&s(&velab), &o),
        0,
        "staged randomize dropped the sidecar"
    );
}

#[test]
fn randomize_unconstrained_field_varies() {
    // A rand field with NO constraint draws across its full range; two draws differ
    // (overwhelmingly likely) — checks the unconstrained path works.
    let (out, err, code) = run("class P;\n\
           rand bit [7:0] b;\n\
         endclass\n\
         module top; P p; integer i; integer seen_hi;\n\
         initial begin\n\
           p = new;\n\
           seen_hi = 0;\n\
           for (i = 0; i < 30; i = i + 1) begin\n\
             p.randomize();\n\
             if (p.b > 200) seen_hi = 1;\n\
           end\n\
           $display(\"seen_hi=%0d\", seen_hi);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("seen_hi=1"), "got:\n{out}");
}

// ───────────────────────── Phase B2: general constraint solver ─────────────────────────

#[test]
fn randomize_inter_variable_lt() {
    // INTER-VARIABLE `x < y` (no single-field [lo,hi] can express it) — enforced by
    // the rejection-sampling solver. Every draw must satisfy x < y.
    let (out, err, code) = run("class P;\n\
           rand int x;\n\
           rand int y;\n\
           constraint c { x >= 0; x <= 100; y >= 0; y <= 100; x < y; }\n\
         endclass\n\
         module top; P p; integer i; integer ok;\n\
         initial begin\n\
           p = new; ok = 1;\n\
           for (i = 0; i < 80; i = i + 1) begin\n\
             p.randomize();\n\
             if (!(p.x < p.y)) ok = 0;\n\
           end\n\
           $display(\"ok=%0d\", ok); $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("ok=1"), "x<y must always hold:\n{out}");
}

#[test]
fn randomize_arithmetic_inter_variable() {
    // Arithmetic inter-variable `a + b == 50` — the solver must find satisfying pairs.
    let (out, err, code) = run("class P;\n\
           rand int a;\n\
           rand int b;\n\
           constraint c { a >= 0; a <= 50; b >= 0; b <= 50; a + b == 50; }\n\
         endclass\n\
         module top; P p; integer i; integer ok;\n\
         initial begin\n\
           p = new; ok = 1;\n\
           for (i = 0; i < 200; i = i + 1) begin\n\
             p.randomize();\n\
             if (p.a + p.b != 50) ok = 0;\n\
           end\n\
           $display(\"ok=%0d\", ok); $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("ok=1"), "a+b==50 must always hold:\n{out}");
}

#[test]
fn randomize_inter_variable_deterministic() {
    // The rejection-sampling solver consumes the seed deterministically: the same
    // program run twice produces the identical accepted draw sequence.
    let src = "class P;\n\
           rand int x;\n\
           rand int y;\n\
           constraint c { x >= 0; x <= 1000; y >= 0; y <= 1000; x < y; }\n\
         endclass\n\
         module top; P p; integer i;\n\
         initial begin\n\
           p = new;\n\
           for (i = 0; i < 5; i = i + 1) begin p.randomize(); $display(\"%0d %0d\", p.x, p.y); end\n\
           $finish;\n\
         end\n\
         endmodule\n";
    let (a, _, _) = run(src);
    let (b, _, _) = run(src);
    assert_eq!(a, b, "the accepted draw sequence must be deterministic");
}

#[test]
fn staged_inter_variable_carries() {
    // The staged vcmp→velab→vrun path must carry the B2 `class_constraints` predicate
    // (else x<y is dropped → the $fatal-on-violation fires → non-zero exit).
    let src = "class P;\n\
           rand int x;\n\
           rand int y;\n\
           constraint c { x >= 0; x <= 100; y >= 0; y <= 100; x < y; }\n\
         endclass\n\
         module top; P p; integer i;\n\
         initial begin\n\
           p = new;\n\
           for (i = 0; i < 8; i = i + 1) begin\n\
             p.randomize();\n\
             if (!(p.x < p.y)) $fatal(1, \"inter-variable constraint dropped\");\n\
           end\n\
           $finish;\n\
         end\n\
         endmodule\n";
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("vita_crvb2_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let sv = dir.join("t.sv");
    std::fs::write(&sv, src).unwrap();
    let s = |p: &std::path::Path| p.to_str().unwrap().to_string();
    let vu = dir.join("t.vu");
    let velab = dir.join("t.velab");
    let o = cli::VitaOpts::default();
    assert_eq!(
        cli::run_vcmp(&[s(&sv)], Some(&s(&vu)), &o),
        0,
        "vcmp failed"
    );
    assert_eq!(cli::run_velab(&s(&vu), &s(&velab), &o), 0, "velab failed");
    assert_eq!(
        cli::run_vrun(&s(&velab), &o),
        0,
        "staged path dropped the B2 constraint predicate"
    );
}

#[test]
fn randomize_inside_set_constraint() {
    // `x inside {1, 3, [10:15], 99}` — the domain narrows to the set's bounding
    // [1,99] and the predicate filters to exact membership.
    let (out, err, code) = run("class P;\n\
           rand int x;\n\
           constraint c { x inside {1, 3, [10:15], 99}; }\n\
         endclass\n\
         module top; P p; integer i; integer ok;\n\
         initial begin\n\
           p = new; ok = 1;\n\
           for (i = 0; i < 200; i = i + 1) begin\n\
             p.randomize();\n\
             if (!(p.x == 1 || p.x == 3 || (p.x >= 10 && p.x <= 15) || p.x == 99)) ok = 0;\n\
           end\n\
           $display(\"ok=%0d\", ok); $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(
        out.contains("ok=1"),
        "x must stay in the inside set:\n{out}"
    );
}

#[test]
fn inside_in_ordinary_expression() {
    // `inside` also works in a plain `if` (it desugars to OR-of-comparisons).
    let (out, _err, code) = run("module top;\n\
         reg [7:0] v;\n\
         initial begin\n\
           v = 12;\n\
           if (v inside {1, [10:15], 99}) $display(\"IN\"); else $display(\"OUT\");\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0));
    assert!(out.contains("IN"), "12 is inside {{1,[10:15],99}}:\n{out}");
}

#[test]
fn randomize_implication_constraint() {
    // `mode == 1 -> x inside {[10:20]}` — when mode is 1, x must be in [10,20];
    // otherwise x is unconstrained (within its range). (`a -> b` ≡ `!a || b`.)
    let (out, err, code) = run("class P;\n\
           rand int mode;\n\
           rand int x;\n\
           constraint c { mode inside {0,1}; x >= 0; x <= 100; mode == 1 -> x inside {[10:20]}; }\n\
         endclass\n\
         module top; P p; integer i; integer ok; integer saw0;\n\
         initial begin\n\
           p = new; ok = 1; saw0 = 0;\n\
           for (i = 0; i < 300; i = i + 1) begin\n\
             p.randomize();\n\
             if (p.mode == 1 && (p.x < 10 || p.x > 20)) ok = 0;\n\
             if (p.mode == 0) saw0 = 1;\n\
           end\n\
           $display(\"ok=%0d saw0=%0d\", ok, saw0); $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("ok=1"), "mode==1 ⇒ x in [10,20]:\n{out}");
    assert!(
        out.contains("saw0=1"),
        "mode==0 must be reachable (implication vacuous):\n{out}"
    );
}

// ───────── B2 adversarial-hunt fixes (randomize return value + wide-field eval) ─────────

#[test]
fn randomize_returns_zero_on_unsatisfiable() {
    // §18.11: an unsatisfiable constraint → randomize() returns 0 (was hardcoded 1),
    // and the fields are left unchanged.
    let (out, err, code) = run("class P;\n\
           rand int x;\n\
           rand int y;\n\
           constraint c { x >= 0; x <= 10; y >= 0; y <= 10; x < y; y < x; }\n\
         endclass\n\
         module top; P p; integer r;\n\
         initial begin p = new; r = p.randomize(); $display(\"r=%0d\", r); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(
        out.contains("r=0"),
        "unsatisfiable randomize() must return 0:\n{out}"
    );
}

#[test]
fn randomize_returns_one_on_success() {
    let (out, err, code) = run("class P;\n\
           rand int x;\n\
           constraint c { x >= 1; x <= 6; }\n\
         endclass\n\
         module top; P p; integer r;\n\
         initial begin p = new; r = p.randomize(); $display(\"r=%0d ok=%0d\", r, (p.x>=1 && p.x<=6)); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(
        out.contains("r=1 ok=1"),
        "satisfiable randomize() returns 1:\n{out}"
    );
}

#[test]
fn randomize_wide_signed_negative_range() {
    // A signed field wider than 64 bits with a negative range: draws must be in
    // range (the copy-out must SIGN-extend the high words, not zero-pad).
    let (out, err, code) = run("class P;\n\
           rand bit signed [127:0] x;\n\
           constraint c { x >= -100; x <= -50; }\n\
         endclass\n\
         module top; P p; integer i; integer ok;\n\
         initial begin\n\
           p = new; ok = 1;\n\
           for (i = 0; i < 30; i = i + 1) begin\n\
             p.randomize();\n\
             if (p.x < -128'sd100 || p.x > -128'sd50) ok = 0;\n\
           end\n\
           $display(\"ok=%0d\", ok); $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(
        out.contains("ok=1"),
        "signed wide negative range must hold:\n{out}"
    );
}

#[test]
fn wide_field_general_constraint_is_loud() {
    // A general (non-range) constraint on a >64-bit field is honestly loud-rejected
    // (the i64 predicate lane cannot faithfully evaluate it) rather than silently
    // accepting out-of-constraint draws.
    let (_o, err, code) = run("class P;\n\
           rand bit signed [127:0] x;\n\
           rand bit signed [127:0] y;\n\
           constraint c { x < y; }\n\
         endclass\n\
         module top; P p; initial begin p = new; p.randomize(); $finish; end\n\
         endmodule\n");
    assert_ne!(
        code,
        Some(0),
        "a >64-bit general constraint must be loud-rejected"
    );
    assert!(err.contains("VITA-E3009"), "expected E3009:\n{err}");
}

#[test]
fn soft_constraint_preferred_when_feasible() {
    // `soft x == 50` within a hard `[0:100]` domain: every draw should be 50.
    let (out, err, code) = run("class P;\n\
           rand int x;\n\
           constraint c { x inside {[0:100]}; soft x == 50; }\n\
         endclass\n\
         module top; P p; integer i; integer all50;\n\
         initial begin\n\
           p = new; all50 = 1;\n\
           for (i = 0; i < 30; i = i + 1) begin p.randomize(); if (p.x != 50) all50 = 0; end\n\
           $display(\"all50=%0d\", all50); $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(
        out.contains("all50=1"),
        "feasible soft constraint must be honored:\n{out}"
    );
}

#[test]
fn soft_constraint_dropped_when_conflicting() {
    // `soft x == 5` conflicts with hard `x != 5`: the soft is dropped, randomize()
    // still succeeds (r==1) and the hard constraint holds.
    let (out, err, code) = run("class P;\n\
           rand int x;\n\
           constraint c { x inside {[0:10]}; x != 5; soft x == 5; }\n\
         endclass\n\
         module top; P p; integer i; integer ok; integer r;\n\
         initial begin\n\
           p = new; ok = 1;\n\
           for (i = 0; i < 30; i = i + 1) begin\n\
             r = p.randomize();\n\
             if (p.x < 0 || p.x > 10 || p.x == 5 || r != 1) ok = 0;\n\
           end\n\
           $display(\"ok=%0d\", ok); $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(
        out.contains("ok=1"),
        "conflicting soft is dropped, hard holds:\n{out}"
    );
}

#[test]
fn dist_weighted_distribution() {
    // `x dist {1 := 10, 2 := 90}`: ~10% land on 1, ~90% on 2 (seeded, so the exact
    // counts are deterministic; assert the gross weighting n2 >> n1 and coverage).
    let (out, err, code) = run("class P;\n\
           rand int x;\n\
           constraint c { x dist {1 := 10, 2 := 90}; }\n\
         endclass\n\
         module top; P p; integer i; integer n1; integer n2;\n\
         initial begin\n\
           p = new; n1 = 0; n2 = 0;\n\
           for (i = 0; i < 400; i = i + 1) begin\n\
             p.randomize();\n\
             if (p.x == 1) n1 = n1 + 1; else if (p.x == 2) n2 = n2 + 1;\n\
           end\n\
           $display(\"cov=%0d wt=%0d\", (n1+n2==400), (n2 > 3*n1)); $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(
        out.contains("cov=1 wt=1"),
        "dist must weight 2 far above 1:\n{out}"
    );
}

#[test]
fn dist_range_spread_vs_per_value() {
    // `[10:19] :/ 50` (spread 50 over the range) vs `100 := 50` (value weight 50):
    // roughly equal mass on the range and on 100.
    let (out, err, code) = run("class P;\n\
           rand int x;\n\
           constraint c { x dist {[10:19] :/ 50, 100 := 50}; }\n\
         endclass\n\
         module top; P p; integer i; integer nr; integer nv;\n\
         initial begin\n\
           p = new; nr = 0; nv = 0;\n\
           for (i = 0; i < 400; i = i + 1) begin\n\
             p.randomize();\n\
             if (p.x >= 10 && p.x <= 19) nr = nr + 1; else if (p.x == 100) nv = nv + 1;\n\
           end\n\
           $display(\"cov=%0d bal=%0d\", (nr+nv==400), (nr > 100 && nv > 100)); $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(
        out.contains("cov=1 bal=1"),
        "spread vs per-value mass should be balanced:\n{out}"
    );
}

// ───────────────────────── inline `randomize() with {…}` (B-CRV final) ─────────

#[test]
fn inline_with_equality() {
    // `randomize() with { v == 5; }` adds a per-call equality constraint (§18.7);
    // every draw must be exactly 5 even though the class has no constraint.
    let (out, err, code) = run("class C;\n\
           rand int v;\n\
         endclass\n\
         module top; C c; integer i; integer ok;\n\
         initial begin\n\
           c = new; ok = 1;\n\
           for (i = 0; i < 10; i = i + 1) begin\n\
             c.randomize() with { v == 5; };\n\
             if (c.v != 5) ok = 0;\n\
           end\n\
           $display(\"ok=%0d\", ok); $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("ok=1"), "inline-with equality:\n{out}");
}

#[test]
fn inline_with_range() {
    // `randomize() with { v >= 10; v < 20; }` — per-call range; all draws in [10,19].
    let (out, err, code) = run("class C;\n\
           rand int v;\n\
         endclass\n\
         module top; C c; integer i; integer ok;\n\
         initial begin\n\
           c = new; ok = 1;\n\
           for (i = 0; i < 30; i = i + 1) begin\n\
             c.randomize() with { v >= 10; v < 20; };\n\
             if (c.v < 10 || c.v >= 20) ok = 0;\n\
           end\n\
           $display(\"ok=%0d\", ok); $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("ok=1"), "inline-with range:\n{out}");
}

#[test]
fn inline_with_adds_to_class_constraint() {
    // §18.7: inline constraints are ADDED to the class constraint, not replacing it.
    // Class says v in [1,100]; inline says v < 10 → draws in [1,9].
    let (out, err, code) = run("class C;\n\
           rand int v;\n\
           constraint base { v >= 1; v <= 100; }\n\
         endclass\n\
         module top; C c; integer i; integer ok;\n\
         initial begin\n\
           c = new; ok = 1;\n\
           for (i = 0; i < 30; i = i + 1) begin\n\
             c.randomize() with { v < 10; };\n\
             if (c.v < 1 || c.v >= 10) ok = 0;\n\
           end\n\
           $display(\"ok=%0d\", ok); $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("ok=1"), "inline-with intersects class:\n{out}");
}

#[test]
fn inline_with_result_and_intervar() {
    // Captured result form `r = obj.randomize() with {…}` + inter-variable predicate.
    let (out, err, code) = run("class C;\n\
           rand int a; rand int b;\n\
           constraint dom { a >= 0; a <= 20; b >= 0; b <= 20; }\n\
         endclass\n\
         module top; C c; integer i; integer ok; integer r;\n\
         initial begin\n\
           c = new; ok = 1;\n\
           for (i = 0; i < 30; i = i + 1) begin\n\
             r = c.randomize() with { a < b; };\n\
             if (r != 1) ok = 0;\n\
             if (!(c.a < c.b)) ok = 0;\n\
           end\n\
           $display(\"ok=%0d\", ok); $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("ok=1"), "inline-with result+intervar:\n{out}");
}

// ───── inline-with adversarial-hunt fixes (silent-wrong → loud / correct) ──────

#[test]
fn inline_with_unknown_field_is_loud() {
    // HUNT finding 1/5: a single-field RANGE constraint on a typo'd / unresolvable
    // field name was SILENTLY dropped (field drawn unconstrained, exit 0), while the
    // `!=` form correctly loud-rejected. Must be loud (matching the `!=` path).
    let (_out, err, code) = run("class C; rand int value;\n\
         endclass\n\
         module top; C c; integer i;\n\
         initial begin\n\
           c = new;\n\
           for (i = 0; i < 5; i = i + 1) c.randomize() with { valu < 5; };\n\
           $display(\"done\"); $finish;\n\
         end\n\
         endmodule\n");
    assert_ne!(
        code,
        Some(0),
        "typo'd inline field must be loud, not silent:\n{err}"
    );
}

#[test]
fn inline_with_on_randc_field_is_loud() {
    // HUNT finding 2: an inline range on a `randc` field was silently dropped (randc
    // drew full range, success=1). Class constraints loud-reject randc constraints;
    // the inline path must too.
    let (_out, err, code) = run("class P; randc bit [3:0] x;\n\
         endclass\n\
         module top; P p; integer i;\n\
         initial begin\n\
           p = new;\n\
           for (i = 0; i < 4; i = i + 1) p.randomize() with { x < 4; };\n\
           $display(\"done\"); $finish;\n\
         end\n\
         endmodule\n");
    assert_ne!(
        code,
        Some(0),
        "inline constraint on randc must be loud:\n{err}"
    );
}

#[test]
fn inline_with_referencing_randc_field_is_loud() {
    // HUNT finding 3: an inline predicate referencing a randc field read it as a
    // stale 0, silently corrupting the inter-variable constraint. Must be loud.
    let (_out, err, code) = run("class P; randc bit [3:0] c; rand bit [3:0] r;\n\
         endclass\n\
         module top; P p; integer i;\n\
         initial begin\n\
           p = new;\n\
           for (i = 0; i < 4; i = i + 1) p.randomize() with { r > c; };\n\
           $display(\"done\"); $finish;\n\
         end\n\
         endmodule\n");
    assert_ne!(
        code,
        Some(0),
        "inline predicate referencing randc must be loud:\n{err}"
    );
}

#[test]
fn inline_with_range_on_dist_field_constrains() {
    // HUNT finding 4: an inline range on a `dist` field was silently dropped (the
    // out-of-range dist value 200 kept appearing). The inline range must exclude it.
    let (out, err, code) = run("class P; rand bit [7:0] x;\n\
           constraint dc { x dist { 10 := 1, 20 := 1, 200 := 1 }; }\n\
         endclass\n\
         module top; P p; integer i; integer ok;\n\
         initial begin\n\
           p = new; ok = 1;\n\
           for (i = 0; i < 24; i = i + 1) begin\n\
             p.randomize() with { x < 100; };\n\
             if (p.x >= 100) ok = 0;\n\
           end\n\
           $display(\"ok=%0d\", ok); $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(
        out.contains("ok=1"),
        "inline range must exclude out-of-range dist value:\n{out}"
    );
}

#[test]
fn class_constraint_unknown_field_range_is_loud() {
    // Pre-existing sibling of finding 1: a CLASS single-field range on a typo'd
    // field was also silently dropped. Fixed by the same predicate-skip gate.
    let (_out, err, code) = run("class C; rand int value;\n\
           constraint c { valu < 5; }\n\
         endclass\n\
         module top; C c; integer i;\n\
         initial begin\n\
           c = new;\n\
           for (i = 0; i < 5; i = i + 1) c.randomize();\n\
           $display(\"done\"); $finish;\n\
         end\n\
         endmodule\n");
    assert_ne!(
        code,
        Some(0),
        "typo'd class-constraint field must be loud:\n{err}"
    );
}
