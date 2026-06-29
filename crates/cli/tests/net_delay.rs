//! Net-declaration delays — `wire #3 w = a;` / `wire #(2,4) w = a;` (IEEE §6.1.3).
//!
//! vita previously PARSE-rejected a `#delay` between a net type and its name
//! ("expected identifier, found Hash"), while iverilog accepts it. A net-decl
//! delay is semantically identical to the same delay on the equivalent
//! `assign #d net = expr;` — so the parser now stores it on `NetVarDecl.delay`
//! and elaborate desugars each net-decl-assignment through the SAME delayed
//! continuous-assign path (uniform `ContAssign.delay` + distinct rise/fall/turnoff
//! `ca_delays` sidecar). Purely additive (the syntax was rejected before) ⇒
//! byte-identical. Pinned to iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_netdelay_{}_{n}", std::process::id()));
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

#[test]
fn wire_delay_single() {
    // `wire #3 w = a;` ≡ `assign #3 w = a;`: a rises at t1 → w rises at t4.
    // Sample at t6 (settled 1), then a falls at t6 → w falls at t9, sample t11 (0).
    let (out, code) = run("module top; reg a; wire #3 w = a;\n\
         initial begin\n\
           a=0; #1 a=1; #5 $display(\"A w=%b\", w);\n\
           a=0; #5 $display(\"B w=%b\", w); $finish;\n\
         end endmodule\n");
    assert_eq!(code, Some(0));
    assert!(out.contains("A w=1"), "rise settled; got:\n{out}");
    assert!(out.contains("B w=0"), "fall settled; got:\n{out}");
}

#[test]
fn wire_delay_rise_fall_distinct() {
    // `wire #(2,4) w = a;`: rise delay 2, fall delay 4 — distinct, so the
    // `ca_delays` sidecar must carry the (2,4,*) triple. a^ at t1 → w^ at t3;
    // a_ at t5 → w_ at t9. Sample B at t8 (fall NOT yet landed → still 1) proves
    // fall>3; if fall were wrongly collapsed to rise(2), w would be 0 by t7.
    let (out, code) = run("module top; reg a; wire #(2,4) w = a;\n\
         initial begin\n\
           a=0; #1 a=1; #4 $display(\"A w=%b\", w);\n\
           a=0; #3 $display(\"B w=%b\", w);\n\
           #3 $display(\"C w=%b\", w); $finish;\n\
         end endmodule\n");
    assert_eq!(code, Some(0));
    assert!(out.contains("A w=1"), "rise(2) landed by t5; got:\n{out}");
    assert!(
        out.contains("B w=1"),
        "fall(4) NOT landed at t8; got:\n{out}"
    );
    assert!(out.contains("C w=0"), "fall(4) landed by t11; got:\n{out}");
}

#[test]
fn wire_delay_with_width() {
    // Delay position is AFTER the range: `wire [7:0] #5 bus = x;`. x set to AB at
    // t1 → bus = AB at t6.
    let (out, code) = run("module top; reg [7:0] x; wire [7:0] #5 bus = x;\n\
         initial begin x=0; #1 x=8'hAB; #6 $display(\"bus=%h\", bus); $finish; end endmodule\n");
    assert_eq!(code, Some(0));
    assert!(out.contains("bus=ab"), "width+delay; got:\n{out}");
}

#[test]
fn wire_delay_multi_decl() {
    // One delay applies to EVERY assignment in the decl: `wire #3 a=p, b=q;`.
    // p^ at t1 → a^ at t4; q^ at t2 → b^ at t5. Sample at t7: both 1.
    let (out, code) = run("module top; reg p,q; wire #3 a=p, b=q;\n\
         initial begin p=0;q=0; #1 p=1; #1 q=1; #5 $display(\"a=%b b=%b\", a, b); $finish; end endmodule\n");
    assert_eq!(code, Some(0));
    assert!(
        out.contains("a=1 b=1"),
        "multi-decl shared delay; got:\n{out}"
    );
}

#[test]
fn wire_delay_desugars_like_assign_delay() {
    // The strongest guard: `wire #3 w = a;` must produce BYTE-IDENTICAL output to
    // the explicit `wire w; assign #3 w = a;` it desugars to — regardless of the
    // exact delay semantics. (vita-internal differential; no oracle dependence.)
    let body = "\n\
         initial begin a=0; #1 a=1; #5 $display(\"w=%b @%0t\", w, $time);\n\
           a=0; #5 $display(\"w=%b @%0t\", w, $time); $finish; end endmodule\n";
    let (decl_form, c1) = run(&format!("module top; reg a; wire #3 w = a;{body}"));
    let (assign_form, c2) = run(&format!(
        "module top; reg a; wire w; assign #3 w = a;{body}"
    ));
    assert_eq!(c1, Some(0));
    assert_eq!(c2, Some(0));
    assert_eq!(
        decl_form, assign_form,
        "wire #3 w=a must equal wire w; assign #3 w=a"
    );
}

#[test]
fn reg_delay_stays_loud() {
    // A `#` after a VARIABLE range is not a net delay — it must stay a loud parse
    // error (correct-or-loud; iverilog also rejects `reg #3 r`).
    let (_out, code) = run("module top; reg #3 r = 5;\n\
         initial begin #1 $display(\"r=%b\", r); $finish; end endmodule\n");
    assert_ne!(
        code,
        Some(0),
        "reg #3 must be rejected, not silently accepted"
    );
}

#[test]
fn wire_delay_in_generate() {
    // A net-decl delay inside a generate block must desugar its driver too
    // (adversarial review found the generate Logic phase silently dropped the
    // net-init driver → net stuck at z). a^ at t2 → w^ at t7; sample t12 = 1.
    let (out, code) = run("module top; reg a;\n\
         generate if (1) begin : g wire #5 w = a; end endgenerate\n\
         initial begin a=0; #2 a=1; #10 $display(\"g w=%b\", g.w); $finish; end endmodule\n");
    assert_eq!(code, Some(0));
    assert!(
        out.contains("g w=1"),
        "generate net-decl delay driver; got:\n{out}"
    );
}

#[test]
fn wire_delay_in_block_stays_loud() {
    // A net-decl delay is illegal inside a PROCEDURAL block — it must stay a loud
    // parse error (correct-or-loud), NOT silently swallow the delay. The parser
    // only accepts a net delay at module/generate scope. iverilog also rejects it.
    let (_out, code) = run("module top; reg a;\n\
         initial begin: bb wire #3 w = a; #5 $display(\"x\"); $finish; end endmodule\n");
    assert_ne!(
        code,
        Some(0),
        "wire #3 in a procedural block must be loud, not silently accepted"
    );
}

#[test]
fn staged_wire_delay_carries_rise_fall_sidecar() {
    // The staged vcmp→velab→vrun path must carry the distinct rise/fall `ca_delays`
    // sidecar for a net-decl delay. If it were dropped, the fall delay would
    // collapse to the uniform `ContAssign.delay` (= rise = 2) and w would fall at
    // t9 instead of t11 → the t10 $fatal guard fires → non-zero exit. A clean exit
    // 0 proves the sidecar survived the `.velab` trailer.
    let src = "module top; reg a; wire #(2,4) w = a;\n\
         initial begin\n\
           a=0; #1 a=1; #6;\n\
           a=0;            // a_ at t7 -> w_ at t11 (fall=4)\n\
           #3;             // t10: fall not landed; if collapsed to rise(2) it would be 0\n\
           if (w !== 1'b1) $fatal(1, \"ca_delays fall sidecar dropped on staged path\");\n\
           #3;             // t13: fall landed at t11\n\
           if (w !== 1'b0) $fatal(1, \"fall delay wrong on staged path\");\n\
           $finish;\n\
         end endmodule\n";
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("vita_ndst_{}_{n}", std::process::id()));
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
        "staged net-delay dropped the ca_delays sidecar"
    );
}
