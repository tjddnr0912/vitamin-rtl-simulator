//! `%v` / `%V` strength format on a multi-bit value renders EVERY bit (MSB-first,
//! joined by `_`), not just bit 0 (IEEE 1364 / iverilog: `4'b10xz` →
//! "St1_St0_StX_HiZ"). vita previously emitted only bit 0's strength. Each bit maps
//! St0/St1 (known 0/1), StX (x), HiZ (z) — vitamin has no strength model, so a
//! driven bit is the conventional STRONG form; this matches iverilog for
//! register/strong-net designs. Pinned to iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_strf_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn multibit_strength_all_bits_msb_first() {
    // `4'b10xz` → bit3=1,bit2=0,bit1=x,bit0=z → St1_St0_StX_HiZ.
    let out = run("module top; reg [3:0] r; initial begin r=4'b10xz; \
         $display(\"[%v]\",r); $finish; end endmodule\n");
    assert!(
        out.contains("[St1_St0_StX_HiZ]"),
        "4-bit %v all bits; got:\n{out}"
    );
}

#[test]
fn eight_bit_strength() {
    // 8'hA5 = 1010_0101.
    let out = run("module top; reg [7:0] r; initial begin r=8'hA5; \
         $display(\"[%v]\",r); $finish; end endmodule\n");
    assert!(
        out.contains("[St1_St0_St1_St0_St0_St1_St0_St1]"),
        "8-bit %v; got:\n{out}"
    );
}

#[test]
fn single_bit_strength_unchanged() {
    // 1-bit values are a single field — byte-identical to the old behavior.
    let out = run(
        "module top; reg a; wire w; assign w=1'b1; initial begin a=1'b0; #1 \
         $display(\"a[%v]w[%v]\",a,w); $finish; end endmodule\n",
    );
    assert!(
        out.contains("a[St0]w[St1]"),
        "1-bit %v unchanged; got:\n{out}"
    );
}

#[test]
fn uppercase_v_same_as_lowercase() {
    let out = run("module top; reg [3:0] r; initial begin r=4'b1100; \
         $display(\"[%V][%v]\",r,r); $finish; end endmodule\n");
    assert!(
        out.contains("[St1_St1_St0_St0][St1_St1_St0_St0]"),
        "%V == %v; got:\n{out}"
    );
}
