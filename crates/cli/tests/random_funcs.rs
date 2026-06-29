//! v7 $random/$urandom(_range)/$stime. $random = IEEE 1364 Annex N — the
//! whole sequence (default-seed AND seeded, incl. the written-back seed) is
//! pinned LIVE against iverilog 13.0 (2026-06-12, probe t6). $urandom is
//! implementation-defined by IEEE → vitamin-pinned splitmix64 (contract
//! values asserted; 3-OS deterministic by construction — pure integer ops).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_rnd_{}_{n}", std::process::id()));
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
fn stime_truncates_in_module_units() {
    // 10ns/1ns at #5 → $stime=5, $time=5 (iverilog-pinned).
    let (out, err, code) = run("`timescale 10ns/1ns\n\
         module top;\n\
         initial begin\n\
           #5;\n\
           $display(\"stime=%0d time=%0d\", $stime, $time);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("stime=5 time=5"), "got:\n{out}");
}

#[test]
fn random_default_seed_matches_iverilog() {
    let (out, err, code) = run("module top;\n\
         integer x, y, z;\n\
         initial begin\n\
           x = $random; y = $random; z = $random;\n\
           $display(\"r1=%0d r2=%0d r3=%0d\", x, y, z);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(
        out.contains("r1=303379748 r2=-1064739199 r3=-2071669239"),
        "got:\n{out}"
    );
}

#[test]
fn random_seeded_updates_seed_and_reproduces() {
    // iverilog-pinned: s=5 → draw -2147138048, seed becomes 345346; next
    // draw 230383387, seed -1917100901. Re-seeding 5 reproduces draw 1.
    let (out, err, code) = run("module top;\n\
         integer s, x;\n\
         initial begin\n\
           s = 5;\n\
           x = $random(s);\n\
           $display(\"rs1=%0d seed=%0d\", x, s);\n\
           x = $random(s);\n\
           $display(\"rs2=%0d seed=%0d\", x, s);\n\
           s = 5;\n\
           x = $random(s);\n\
           $display(\"again=%0d\", x);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("rs1=-2147138048 seed=345346"), "got:\n{out}");
    assert!(
        out.contains("rs2=230383387 seed=-1917100901"),
        "got:\n{out}"
    );
    assert!(out.contains("again=-2147138048"), "got:\n{out}");
}

#[test]
fn seeded_random_outside_direct_rhs_is_loud() {
    // The seeded form writes its ref argument — any placement the
    // statement intercept can't serve is an elaborate-time E3009.
    let (out, err, code) = run("module top;\n\
         integer s, x;\n\
         initial begin\n\
           s = 1;\n\
           x = $random(s) + 1;\n\
           $display(\"x=%0d\", x);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_ne!(code, Some(0), "must fail loud, got stdout:\n{out}");
    assert!(err.contains("E3009"), "stderr:\n{err}");
}

#[test]
fn urandom_is_deterministic_contract() {
    // vitamin contract pin (splitmix64 from state 0): same design, same
    // values, every OS, every run. Values asserted EXACTLY so any silent
    // generator change breaks here.
    let (out, err, code) = run("module top;\n\
         reg [31:0] a, b, c;\n\
         initial begin\n\
           a = $urandom; b = $urandom;\n\
           c = $urandom(32'd7);\n\
           $display(\"u1=%0d u2=%0d useed=%0d\", a, b, c);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    let (out2, _, _) = run("module top;\n\
         reg [31:0] a, b, c;\n\
         initial begin\n\
           a = $urandom; b = $urandom;\n\
           c = $urandom(32'd7);\n\
           $display(\"u1=%0d u2=%0d useed=%0d\", a, b, c);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    let line = out.lines().find(|l| l.starts_with("u1=")).unwrap_or("");
    let line2 = out2.lines().find(|l| l.starts_with("u1=")).unwrap_or("");
    assert_eq!(line, line2, "two runs must agree byte-for-byte");
    // splitmix64(0+γ)>>32 / next / seeded-from-7 — the pinned contract.
    assert!(
        out.contains("u1=3793791033 u2=1853398634 useed=1674306020"),
        "got:\n{out}"
    );
}

#[test]
fn urandom_range_bounds_and_swap() {
    // Inclusive bounds; swapped args auto-correct (IEEE §18.13.3);
    // single-arg form is [0, max]; max==min collapses.
    let (out, err, code) = run("module top;\n\
         reg [31:0] v;\n\
         integer i, bad;\n\
         initial begin\n\
           bad = 0;\n\
           for (i = 0; i < 50; i = i + 1) begin\n\
             v = $urandom_range(10, 5);\n\
             if (v < 5 || v > 10) bad = bad + 1;\n\
             v = $urandom_range(5, 10);\n\
             if (v < 5 || v > 10) bad = bad + 1;\n\
             v = $urandom_range(3);\n\
             if (v > 3) bad = bad + 1;\n\
           end\n\
           v = $urandom_range(9, 9);\n\
           $display(\"bad=%0d fix=%0d\", bad, v);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("bad=0 fix=9"), "got:\n{out}");
}
