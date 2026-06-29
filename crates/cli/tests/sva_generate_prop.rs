//! Generate-scope SVA declarations + property-references-property (Phase-3 slice A4).
//! (1) A `sequence`/`property` declared inside a `generate` block is collected into
//! the module-global table (the prescan now recurses into generate), so a reference
//! resolves. (2) A property whose CONSEQUENT is a bare named-property instance
//! (`property p; @(clk) a |-> q; endproperty`, q an OVERLAP property) flattens to
//! `a |-> (!q.ante || q.cons)` — `q`'s single-tick `b |-> c` becomes the boolean
//! `!b || c`. Both are pure IR-0 (sim-ir frozen, format_version 8; no AST change).
//!
//! iverilog 13.0 rejects all of this (NULL oracle) → hand-IEEE. LOUD (deferred):
//! a self/mutually recursive property; an inner property with a DIFFERENT clock
//! (multi-clock), its own `disable iff`, or formal arguments. (An inner `|=>` is no
//! longer loud — slice SVA-R2 synthesizes the canonical `a |-> (b |=> c)` ≡
//! `(a && b) |=> c`; see cli/tests/sva_propref_nonoverlap.rs.)
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_svagp_{}_{n}", std::process::id()));
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

// ── (1) generate-scope declarations ────────────────────────────────────────

#[test]
fn generate_if_scope_property_collected_and_holds() {
    // `property p_ab` declared inside `generate if(1) begin:g … end` must be
    // collected; the in-scope `assert property(p_ab)` resolves it. a=1,b=1 → holds.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0;\n\
         always #5 clk=~clk;\n\
         generate if (1) begin : g\n\
           property p_ab; @(posedge clk) a |-> b; endproperty\n\
           initial assert property(p_ab);\n\
         end endgenerate\n\
         initial begin #10 a=1; b=1; #20 $finish; end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "generate-scope property must be collected + hold → clean. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        !err.contains("unknown property"),
        "the decl must resolve, not be unknown:\n{err}"
    );
}

#[test]
fn generate_if_scope_property_violation_fires() {
    // Same, but a=1,b=0 at a posedge → the generate-scope property fires.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0;\n\
         always #5 clk=~clk;\n\
         generate if (1) begin : g\n\
           property p_ab; @(posedge clk) a |-> b; endproperty\n\
           initial assert property(p_ab);\n\
         end endgenerate\n\
         initial begin #10 a=1; b=0; #20 $finish; end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "generate-scope property violation must fire. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        format!("{err}{out}").to_lowercase().contains("assertion"),
        "expected an assertion violation:\n{err}\n{out}"
    );
}

#[test]
fn generate_scope_decl_referenced_at_module_scope() {
    // A decl inside a generate block is module-global, so a MODULE-scope assert
    // resolves it too.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0;\n\
         always #5 clk=~clk;\n\
         generate if (1) begin : g\n\
           sequence s_ab; a ##1 b; endsequence\n\
         end endgenerate\n\
         initial assert property(@(posedge clk) s_ab |-> 1'b1);\n\
         initial begin #10 a=1; b=1; #20 $finish; end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "a generate-scope sequence must resolve at module scope. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        !err.contains("unknown") && !err.contains("undeclared net 's_ab'"),
        "the generate-scope sequence must resolve:\n{err}"
    );
}

// ── (2) property-references-property (overlap inner) ───────────────────────

#[test]
fn property_consequent_references_property_holds() {
    // property q; @(clk) b |-> c;  property p; @(clk) a |-> q;
    // p ≡ a |-> (b |-> c) ≡ a |-> (!b || c). a=1,b=1,c=1 → !b||c = 1 → holds.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0;\n\
         always #5 clk=~clk;\n\
         property q_bc; @(posedge clk) b |-> c; endproperty\n\
         property p_aq; @(posedge clk) a |-> q_bc; endproperty\n\
         initial assert property(p_aq);\n\
         initial begin #10 a=1; b=1; c=1; #20 $finish; end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "prop-ref-prop (a&&b ⇒ c) holds with c=1 → clean. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        !err.contains("undeclared") && !err.contains("unknown"),
        "q_bc must resolve as a property consequent:\n{err}"
    );
}

#[test]
fn property_consequent_references_property_violation_fires() {
    // a=1,b=1,c=0 → !b||c = 0 → a |-> 0 → fires (proves the inner consequent c is
    // actually checked, not silently passed).
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0;\n\
         always #5 clk=~clk;\n\
         property q_bc; @(posedge clk) b |-> c; endproperty\n\
         property p_aq; @(posedge clk) a |-> q_bc; endproperty\n\
         initial assert property(p_aq);\n\
         initial begin #10 a=1; b=1; c=0; #20 $finish; end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "prop-ref-prop violation (a&&b, c=0) must fire. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        format!("{err}{out}").to_lowercase().contains("assertion"),
        "expected an assertion violation:\n{err}\n{out}"
    );
}

#[test]
fn property_consequent_inner_vacuous_holds() {
    // a=1, b=0 → inner antecedent false → !b||c = 1 → holds regardless of c.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0;\n\
         always #5 clk=~clk;\n\
         property q_bc; @(posedge clk) b |-> c; endproperty\n\
         property p_aq; @(posedge clk) a |-> q_bc; endproperty\n\
         initial assert property(p_aq);\n\
         initial begin #10 a=1; b=0; c=0; #20 $finish; end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "inner antecedent false → vacuous hold → clean. stderr:\n{err}\nout:\n{out}"
    );
}

// ── LOUD (deferred) ────────────────────────────────────────────────────────

#[test]
fn self_recursive_property_is_loud_and_terminates() {
    let (_o, err, code) = run("module top;\n\
         reg clk=0, a=0; always #5 clk=~clk;\n\
         property p; @(posedge clk) a |-> p; endproperty\n\
         initial assert property(p);\n\
         initial #20 $finish;\n\
         endmodule\n");
    assert_eq!(code, Some(1), "self-recursive property must be loud. {err}");
    assert!(err.contains("VITA-E"), "{err}");
}

#[test]
fn inner_property_different_clock_is_loud() {
    let (_o, err, code) = run("module top;\n\
         reg clk=0, clk2=0, a=0, b=0, c=0;\n\
         always #5 clk=~clk; always #7 clk2=~clk2;\n\
         property q; @(posedge clk2) b |-> c; endproperty\n\
         property p; @(posedge clk) a |-> q; endproperty\n\
         initial assert property(p);\n\
         initial #20 $finish;\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "inner-property different clock must be loud. {err}"
    );
    assert!(err.contains("VITA-E"), "{err}");
}

#[test]
fn inner_property_nonoverlap_now_synthesized() {
    // SVA-R2 (2026-06-16): an inner NON-OVERLAP property `q: b |=> c` referenced by an
    // OVERLAP outer `p: a |-> q` is now synthesized as `(a && b) |=> c` (was loud
    // pre-SVA-R2). a=b=0 → (a&&b)=0 → vacuous → clean. Full behavior coverage lives in
    // cli/tests/sva_propref_nonoverlap.rs.
    let (_o, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0; always #5 clk=~clk;\n\
         property q; @(posedge clk) b |=> c; endproperty\n\
         property p; @(posedge clk) a |-> q; endproperty\n\
         initial assert property(p);\n\
         initial #20 $finish;\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "inner |=> property is now synthesized (vacuous here) → clean. {err}"
    );
    assert!(!err.contains("VITA-E"), "must not be loud anymore:\n{err}");
}
