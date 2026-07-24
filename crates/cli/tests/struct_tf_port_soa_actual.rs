//! Round-19 Task 5 ("F-struct"): a record SoA array ELEMENT (`kats[0]`) passed as a
//! task/function actual for a NON-packable-struct (string/mixed-state member) tf-port
//! formal. `kat_t k` (a `string`/`m_e` member struct) expands the FORMAL into N
//! per-member nets `$unp$k$mode`, `$unp$k$name` (`unpacked_struct_member_ports`), but
//! the call-site expander (`expand_struct_call_args`) only matched a bare `Ident`
//! naming a SCALAR unpacked-struct var — `kats[0]` is a `BitSelect` on a record SoA
//! ARRAY (`record_soa_vars`), so it fell through unexpanded (1 actual for N formals) →
//! `E3009 missing actual for formal '$unp$k$name'`.
//!
//! Fix: `expand_struct_call_args` now ALSO matches `arr[i]` (and `arr[i +: w]`) whose
//! base names a record SoA array, expanding it to its N per-member element actuals
//! `$unp$arr$field[i]` — one per member, in the SAME declaration order the formal
//! side used (both walk `unpacked_struct_layouts[ty]` with a plain `.iter()`), so the
//! N actuals line up 1:1 with the N formals. Parser-only (`$unp$`/SoA desugar) — no
//! `format_version` bump.
//!
//! Also resolves the round-19 report's tb_sha3 "m.name" item: that failure was this
//! same F-struct gap (`run_test(kats[0])` never reached `m.name()` because the call
//! itself failed to bind) — `mname_resolved` below is the tb_sha3-shape replica.
//!
//! No external oracle (iverilog rejects unpacked structs entirely) — hand-IEEE
//! verification, cross-checked against the existing per-member string-formal copy-in
//! (§13.5.1 pass-by-value) that a scalar struct actual already exercises.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_stfsoa_{}_{n}", std::process::id()));
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

// The F-struct repro verbatim (round-19 report shape): a dynamic record SoA array
// element passed to a task's non-packable-struct input formal, body reads `k.mode`.
#[test]
fn struct_input_formal_string_member() {
    let (out, code) = run("package kp; typedef enum logic[1:0]{M0,M1} m_e; \
         typedef struct { m_e mode; string name; } kat_t; endpackage\n\
         module t; import kp::*;\n\
         logic clk=0; always #5 clk=~clk; m_e mode;\n\
         task automatic run_kat (input kat_t k);\n\
         m_e m; @(posedge clk); m = k.mode; mode <= m; $display(\"PASS\");\n\
         endtask\n\
         kat_t kats [];\n\
         initial begin kats=new[1]; kats[0].mode=M1; kats[0].name=\"a\"; \
         run_kat(kats[0]); #20 $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "F-struct repro must parse+run:\n{out}");
    assert!(out.contains("PASS"), "{out}");
}

// tb_sha3-shape replica: `run_test(kats[0])`, body reads `k.mode` into a local enum
// var `m`, then `$display` uses BOTH `k.name` (string member) and `m.name()` (enum
// method on the LOCAL var) inside an if/else — confirms the round-19 report's
// tb_sha3 "m.name" failure is subsumed by the F-struct fix (the call itself now
// binds, so `.name()` is reached).
#[test]
fn mname_resolved() {
    let (out, code) = run("package kp; typedef enum logic[1:0]{M0,M1} m_e; \
         typedef struct { m_e mode; string name; } kat_t; endpackage\n\
         module t; import kp::*;\n\
         task automatic run_test (input kat_t k);\n\
         m_e m; m = k.mode;\n\
         if (m == M1) $display(\"[%-10s] mode=%s\", k.name, m.name());\n\
         else $display(\"[%-10s] mode=%s (other)\", k.name, m.name());\n\
         endtask\n\
         kat_t kats [];\n\
         initial begin\n\
         kats=new[2];\n\
         kats[0].mode=M1; kats[0].name=\"first\";\n\
         kats[1].mode=M0; kats[1].name=\"second\";\n\
         run_test(kats[0]); run_test(kats[1]);\n\
         $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "mname replica must parse+run:\n{out}");
    assert!(out.contains("[first     ] mode=M1"), "{out}");
    assert!(out.contains("[second    ] mode=M0 (other)"), "{out}");
}

// Body reads BOTH members of the passed element.
#[test]
fn struct_formal_both_members_used() {
    let (out, code) = run("package kp; typedef enum logic[1:0]{M0,M1} m_e; \
         typedef struct { m_e mode; string name; } kat_t; endpackage\n\
         module t; import kp::*;\n\
         task automatic run_kat (input kat_t k);\n\
         m_e m; m = k.mode;\n\
         $display(\"mode=%s name=%s\", m.name(), k.name);\n\
         endtask\n\
         kat_t kats [];\n\
         initial begin kats=new[1]; kats[0].mode=M1; kats[0].name=\"both\"; \
         run_kat(kats[0]); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "both-members body must parse+run:\n{out}");
    assert!(out.contains("mode=M1 name=both"), "{out}");
}

// A FIXED (not dynamic) SoA array element (`kat_t kats[2]; run_kat(kats[1])`).
#[test]
fn struct_formal_fixed_array() {
    let (out, code) = run("package kp; typedef enum logic[1:0]{M0,M1} m_e; \
         typedef struct { m_e mode; string name; } kat_t; endpackage\n\
         module t; import kp::*;\n\
         task automatic run_kat (input kat_t k);\n\
         m_e m; m = k.mode;\n\
         $display(\"mode=%s name=%s\", m.name(), k.name);\n\
         endtask\n\
         kat_t kats [2];\n\
         initial begin\n\
         kats[0].mode=M0; kats[0].name=\"zero\";\n\
         kats[1].mode=M1; kats[1].name=\"one\";\n\
         run_kat(kats[1]);\n\
         $finish; end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "fixed SoA array element must parse+run:\n{out}"
    );
    assert!(out.contains("mode=M1 name=one"), "{out}");
}

// Adversarial (IEEE §13.5.1 pass-by-value): the caller mutates `kats[0].name` AFTER
// the call — while the callee is suspended at `@(posedge clk)`, WAITING to read
// `k.name` — so the callee's read must see the value AT CALL TIME ("original"), never
// the caller's later mutation ("mutated"). A silent alias (rather than a per-member
// deep copy) would leak the mutation through and print "callee_saw=mutated".
#[test]
fn struct_input_by_value() {
    let (out, code) = run("package kp; typedef enum logic[1:0]{M0,M1} m_e; \
         typedef struct { m_e mode; string name; } kat_t; endpackage\n\
         module t; import kp::*;\n\
         logic clk=0; always #5 clk=~clk;\n\
         task automatic run_kat (input kat_t k);\n\
         @(posedge clk);\n\
         $display(\"callee_saw=%s\", k.name);\n\
         endtask\n\
         kat_t kats [];\n\
         initial begin\n\
         kats=new[1]; kats[0].mode=M0; kats[0].name=\"original\";\n\
         run_kat(kats[0]);\n\
         kats[0].name=\"mutated\";\n\
         #20 $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "by-value adversarial must parse+run:\n{out}");
    assert!(
        out.contains("callee_saw=original"),
        "pass-by-value violated (formal aliased the caller's later mutation):\n{out}"
    );
    assert!(!out.contains("callee_saw=mutated"), "{out}");
}

// ---- regression: paths this change must NOT touch ----

// A PACKABLE struct (`{int;int}`, no string/real/mixed member) array element actual
// already worked via a DIFFERENT path (`record_array_vars` — a whole packed-vector
// array, single-net element read, no per-member expansion needed). Must stay
// unaffected by the new `record_soa_vars` fallback (disjoint registries).
#[test]
fn packable_struct_array_elem_actual_regression() {
    let (out, code) = run("module t;\n\
         typedef struct { int a; int b; } kd_t;\n\
         task automatic run_kd (input kd_t k);\n\
         $display(\"a=%0d b=%0d\", k.a, k.b);\n\
         endtask\n\
         kd_t kd [];\n\
         initial begin kd=new[1]; kd[0].a=3; kd[0].b=4; run_kd(kd[0]); $finish; end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "packable struct array-elem actual regression:\n{out}"
    );
    assert!(out.contains("a=3 b=4"), "{out}");
}

// A bare-Ident SCALAR non-packable struct var actual (the pre-existing R5 path) must
// keep working unchanged.
#[test]
fn bare_ident_struct_actual_regression() {
    let (out, code) = run("package kp; typedef enum logic[1:0]{M0,M1} m_e; \
         typedef struct { m_e mode; string name; } kat_t; endpackage\n\
         module t; import kp::*;\n\
         task automatic run_kat (input kat_t k);\n\
         m_e m; m = k.mode;\n\
         $display(\"mode=%s name=%s\", m.name(), k.name);\n\
         endtask\n\
         kat_t structvar;\n\
         initial begin structvar.mode=M1; structvar.name=\"bare\"; \
         run_kat(structvar); $finish; end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "bare-ident scalar struct actual regression:\n{out}"
    );
    assert!(out.contains("mode=M1 name=bare"), "{out}");
}

// correct-or-loud: the expansion also matches an `arr[i +: w]` INDEXED-PART actual
// (mirroring the `arr[i]` BitSelect case structurally), but an indexed-part SLICE of
// a dyn/queue member net has no whole-value surface at elaborate (it denotes a
// SUB-ARRAY, not a scalar element) — so each expanded per-member actual is rejected
// there, loud. Confirms the extra match arm never silently miscompiles; at worst it
// produces a well-formed AST elaborate correctly refuses.
#[test]
fn soa_array_elem_indexed_part_slice_stays_loud() {
    let (out, code) = run("package kp; typedef enum logic[1:0]{M0,M1} m_e; \
         typedef struct { m_e mode; string name; } kat_t; endpackage\n\
         module t; import kp::*;\n\
         task automatic run_kat (input kat_t k);\n\
         $display(\"name=%s\", k.name);\n\
         endtask\n\
         kat_t kats [];\n\
         initial begin kats=new[2]; kats[0].mode=M1; kats[0].name=\"idx\"; \
         run_kat(kats[0+:1]); $finish; end\n\
         endmodule\n");
    assert_ne!(
        code,
        Some(0),
        "an indexed-part slice actual must stay loud, not crash or silently mis-bind:\n{out}"
    );
}

// correct-or-loud: an arity mismatch this expansion can't resolve (a 2-member record
// SoA array element passed where a DIFFERENT-shape record type is expected) must stay
// loud, never silently mis-line-up members. Cross-type element actual → non-matching
// formal member set (`$unp$k$other` has no bound actual) is caught by the existing
// `fill_default_args` arity gate.
#[test]
fn cross_type_soa_array_elem_actual_stays_loud() {
    let (out, code) = run("module t;\n\
         typedef struct { string a; int b; } rec1_t;\n\
         typedef struct { string x; int y; int z; } rec2_t;\n\
         task automatic run_r2 (input rec2_t k);\n\
         $display(\"x=%s y=%0d z=%0d\", k.x, k.y, k.z);\n\
         endtask\n\
         rec1_t kats [];\n\
         initial begin kats=new[1]; kats[0].a=\"x\"; kats[0].b=1; \
         run_r2(kats[0]); $finish; end\n\
         endmodule\n");
    assert_ne!(
        code,
        Some(0),
        "cross-type SoA element actual must stay loud, not silently mis-map:\n{out}"
    );
}

// correct-or-loud (adversarial-review Finding 1): the fan-out CLONES the array index
// into each of the N per-member reads. A SIDE-EFFECTING / non-deterministic index (a
// CALL) would be evaluated N times → duplicated side effects + a TORN record read
// (members read from different elements), silently. IEEE §13.5.1 evaluates the actual
// (and its index) ONCE. The parser can't hoist the index to a temp, so a call-bearing
// index is rejected (unexpanded → loud arity), never silently multi-evaluated.
#[test]
fn call_index_stays_loud() {
    let (out, code) = run("package kp; typedef enum logic[1:0]{M0,M1} m_e;\n\
         typedef struct { m_e mode; string name; } kat_t; endpackage\n\
         module t; import kp::*;\n\
         logic clk=0; always #5 clk=~clk; m_e mode;\n\
         function automatic int nxt(); nxt = 0; endfunction\n\
         task automatic run_kat (input kat_t k); m_e m; @(posedge clk); m = k.mode;\n\
         mode <= m; $display(\"PASS\"); endtask\n\
         kat_t kats [];\n\
         initial begin kats=new[1]; kats[0].mode=M1; kats[0].name=\"a\";\n\
         run_kat(kats[nxt()]); #20 $finish; end\n\
         endmodule\n");
    assert_ne!(
        code,
        Some(0),
        "a call-bearing SoA element index must stay loud (would multi-evaluate → torn read):\n{out}"
    );
}

// A PURE arithmetic index (`i + 1`) is idempotent — every clone reads the same element,
// so it stays supported (the fix rejects only call-bearing indices).
#[test]
fn pure_arith_index_supported() {
    let (out, code) = run("package kp; typedef enum logic[1:0]{M0,M1} m_e;\n\
         typedef struct { m_e mode; string name; } kat_t; endpackage\n\
         module t; import kp::*;\n\
         logic clk=0; always #5 clk=~clk; m_e mode;\n\
         task automatic run_kat (input kat_t k); m_e m; @(posedge clk); m = k.mode;\n\
         mode <= m; $display(\"PASS\"); endtask\n\
         kat_t kats [];\n\
         initial begin int i=1; kats=new[3]; kats[2].mode=M1; kats[2].name=\"a\";\n\
         run_kat(kats[i+1]); #20 $finish; end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "pure arithmetic index must stay supported:\n{out}"
    );
    assert!(
        out.contains("PASS"),
        "pure-index actual did not bind:\n{out}"
    );
}
