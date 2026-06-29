//! `wand` / `wor` net types — wired-AND / wired-OR multi-driver resolution.
//!
//! vita previously E3009-rejected the `wand`/`wor` net kinds. Now they elaborate
//! as plain Wire nets plus a sidecar marking their resolution kind; a MULTI-driven
//! wand/wor net is resolved by IEEE 1364 4-state wired-AND / wired-OR instead of
//! the default wire resolution (z is the identity for all three; wand: a 0 forces
//! 0, two 1s give 1; wor: a 1 forces 1, two 0s give 0; else x). A single-driver
//! wand/wor is just its driver's value (same as a wire). Purely additive (these
//! kinds were rejected before) ⇒ byte-identical. Pinned to iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_wired_{}_{n}", std::process::id()));
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
fn wand_resolves() {
    // wired-AND: 1&1=1, 1&0=0, 0&0=0, both-z=z.
    let (out, code) = run("module top;\n\
         reg a, b; wand w;\n\
         assign w = a; assign w = b;\n\
         initial begin\n\
           a=1; b=1; #1; $display(\"p w=%b\", w);\n\
           a=1; b=0; #1; $display(\"q w=%b\", w);\n\
           a=1'bz; b=1'bz; #1; $display(\"r w=%b\", w); $finish;\n\
         end endmodule\n");
    assert_eq!(code, Some(0));
    assert!(out.contains("p w=1"), "1&1=1; got:\n{out}");
    assert!(out.contains("q w=0"), "1&0=0; got:\n{out}");
    assert!(out.contains("r w=z"), "z&z=z; got:\n{out}");
}

#[test]
fn wor_resolves() {
    // wired-OR: 0|0=0, 1|0=1, 1|1=1, both-z=z.
    let (out, code) = run("module top;\n\
         reg a, b; wor w;\n\
         assign w = a; assign w = b;\n\
         initial begin\n\
           a=0; b=0; #1; $display(\"p w=%b\", w);\n\
           a=1; b=0; #1; $display(\"q w=%b\", w);\n\
           a=1'bz; b=1'bz; #1; $display(\"r w=%b\", w); $finish;\n\
         end endmodule\n");
    assert_eq!(code, Some(0));
    assert!(out.contains("p w=0"), "0|0=0; got:\n{out}");
    assert!(out.contains("q w=1"), "1|0=1; got:\n{out}");
    assert!(out.contains("r w=z"), "z|z=z; got:\n{out}");
}

#[test]
fn wand_x_on_disagree() {
    // wand with a 1 vs x ⇒ x (not 0, since neither is 0).
    let (out, code) = run("module top;\n\
         wand w; assign w = 1'b1; assign w = 1'bx;\n\
         initial begin #1; $display(\"w=%b\", w); $finish; end endmodule\n");
    assert_eq!(code, Some(0));
    assert!(out.contains("w=x"), "1 wand x = x; got:\n{out}");
}

#[test]
fn multibit_wand_and_wor() {
    // 4-bit wand (AND of F,5 = 5) and wor (OR of 0,A = A), per bit.
    let (out, code) = run("module top;\n\
         wand [3:0] w; assign w = 4'hF; assign w = 4'h5;\n\
         wor [3:0] r; assign r = 4'h0; assign r = 4'hA;\n\
         initial begin #1; $display(\"w=%h r=%h\", w, r); $finish; end endmodule\n");
    assert_eq!(code, Some(0));
    assert!(out.contains("w=5"), "F wand 5 = 5; got:\n{out}");
    assert!(out.contains("r=a"), "0 wor A = A; got:\n{out}");
}

#[test]
fn three_driver_wand() {
    // wired-AND of three drivers: 1&1&1=1, 1&1&0=0.
    let (out, code) = run("module top;\n\
         reg a, b, c; wand w;\n\
         assign w = a; assign w = b; assign w = c;\n\
         initial begin a=1; b=1; c=1; #1; $display(\"p w=%b\", w); a=1; b=1; c=0; #1; $display(\"q w=%b\", w); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0));
    assert!(
        out.contains("p w=1") && out.contains("q w=0"),
        "3-driver wand; got:\n{out}"
    );
}

#[test]
fn output_wand_wor_port_resolves() {
    // A net declared `output wand`/`wor` and multi-driven INSIDE the submodule
    // must resolve wired-AND/OR, not wire — the port-net creation path needs the
    // sidecar too (regression: it was missing, giving wire resolution = x).
    let (out, code) = run("module drv(output wand y);\n\
         assign y = 1'b1; assign y = 1'b0;\n\
         endmodule\n\
         module top; wire w; drv d(.y(w));\n\
         initial begin #1; $display(\"w=%b\", w); $finish; end endmodule\n");
    assert_eq!(code, Some(0));
    assert!(
        out.contains("w=0"),
        "output wand port 1&0 = 0 (not x); got:\n{out}"
    );
}

#[test]
fn single_driver_wand_is_value() {
    // A single-driver wand/wor is just its driver's value (like a wire).
    let (out, code) = run("module top;\n\
         wand w; assign w = 1'b0; wor r; assign r = 1'b1;\n\
         initial begin #1; $display(\"w=%b r=%b\", w, r); $finish; end endmodule\n");
    assert_eq!(code, Some(0));
    assert!(
        out.contains("w=0 r=1"),
        "single-driver wand/wor = value; got:\n{out}"
    );
}

#[test]
fn staged_wand_wor_carries_sidecar() {
    // The staged vcmp→velab→vrun path must carry the wand/wor wired-logic
    // sidecars (the STAGED-DROP class of bug). If dropped, a multi-driven wand/wor
    // net silently falls back to plain WIRE resolution: wand(1,0)=x (not 0),
    // wor(0,1)=x (not 1) — both would trip the $fatal guards → non-zero exit.
    // A clean exit 0 proves both sidecars survived the `.velab` trailer.
    let src = "module top;\n\
         wand wa; assign wa = 1'b1; assign wa = 1'b0;\n\
         wor  wo; assign wo = 1'b0; assign wo = 1'b1;\n\
         initial begin\n\
           #1;\n\
           if (wa !== 1'b0) $fatal(1, \"wand sidecar dropped (got wire x)\");\n\
           if (wo !== 1'b1) $fatal(1, \"wor sidecar dropped (got wire x)\");\n\
           $finish;\n\
         end\n\
         endmodule\n";
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("vita_wirst_{}_{n}", std::process::id()));
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
    // Clean exit 0 ⇒ no $fatal ⇒ wand/wor resolved correctly ⇒ sidecars carried.
    assert_eq!(
        cli::run_vrun(&s(&velab), &o),
        0,
        "staged wand/wor dropped the wired-logic sidecar"
    );
}
