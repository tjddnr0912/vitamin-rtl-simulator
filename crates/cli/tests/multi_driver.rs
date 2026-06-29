//! Multi-driver continuous-assign WIRE resolution (tristate buses).
//!
//! A net driven by ≥2 cont-assigns that are ALL whole-net and non-delayed is
//! resolved by IEEE 1364 4-state wire resolution (z = high-Z yields; equal non-z
//! keeps; conflicting non-z → x) instead of vita's old E3001 hard-reject. This is
//! the bidirectional/tristate-bus pattern. The fix is purely additive — such nets
//! were rejected before, so no pre-existing design carries one (byte-identical).
//!
//! Out of scope (still E3001 / E3009): a PARTIAL/bit-select overlap, a dynamic or
//! array-element driver, a delayed `assign #d`, and `wand`/`wor` net types — those
//! are separate follow-up slices. Every expectation is pinned to iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_md_{}_{n}", std::process::id()));
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
fn tristate_bus_resolves() {
    // Two gated drivers: only-1, only-2, both (conflict→x), neither (high-Z).
    let (out, code) = run("module top;\n\
         reg en1, en2, d1, d2; wire w;\n\
         assign w = en1 ? d1 : 1'bz;\n\
         assign w = en2 ? d2 : 1'bz;\n\
         initial begin\n\
           en1=1; en2=0; d1=1; d2=0; #1; $display(\"a w=%b\", w);\n\
           en1=0; en2=1; #1; $display(\"b w=%b\", w);\n\
           en1=1; en2=1; d1=1; d2=0; #1; $display(\"c w=%b\", w);\n\
           en1=0; en2=0; #1; $display(\"d w=%b\", w); $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0));
    assert!(out.contains("a w=1"), "driver-1 only ⇒ 1; got:\n{out}");
    assert!(out.contains("b w=0"), "driver-2 only ⇒ 0; got:\n{out}");
    assert!(out.contains("c w=x"), "1 vs 0 conflict ⇒ x; got:\n{out}");
    assert!(out.contains("d w=z"), "both high-Z ⇒ z; got:\n{out}");
}

#[test]
fn agreeing_drivers_keep_value() {
    // Two enabled drivers asserting the SAME value ⇒ that value (no conflict).
    let (out, code) = run("module top;\n\
         reg d1, d2; wire w;\n\
         assign w = d1;\n\
         assign w = d2;\n\
         initial begin d1=1; d2=1; #1; $display(\"e w=%b\", w); d1=0; d2=0; #1; $display(\"f w=%b\", w); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0));
    assert!(
        out.contains("e w=1") && out.contains("f w=0"),
        "agreeing drivers keep value; got:\n{out}"
    );
}

#[test]
fn eight_bit_bus_three_drivers() {
    // 8-bit bus, three gated drivers; conflict resolves per-bit to x.
    let (out, code) = run("module top;\n\
         reg [7:0] d1, d2, d3; reg e1, e2, e3; wire [7:0] bus;\n\
         assign bus = e1 ? d1 : 8'bz;\n\
         assign bus = e2 ? d2 : 8'bz;\n\
         assign bus = e3 ? d3 : 8'bz;\n\
         initial begin\n\
           d1=8'hAA; d2=8'h55; d3=8'hF0; e1=1; e2=0; e3=0; #1; $display(\"a bus=%h\", bus);\n\
           e1=0; e2=1; #1; $display(\"b bus=%h\", bus);\n\
           e1=1; e2=1; #1; $display(\"c bus=%h\", bus);\n\
           e1=0; e2=0; e3=0; #1; $display(\"d bus=%h\", bus); $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0));
    assert!(out.contains("a bus=aa"), "d1 only; got:\n{out}");
    assert!(out.contains("b bus=55"), "d2 only; got:\n{out}");
    assert!(
        out.contains("c bus=xx"),
        "aa vs 55 all-bit conflict ⇒ xx; got:\n{out}"
    );
    assert!(out.contains("d bus=zz"), "all high-Z ⇒ zz; got:\n{out}");
}

#[test]
fn resolved_net_drives_an_edge() {
    // A multi-driven net used as a clock: the resolved value's posedges must fire
    // `@(posedge clk)` ⇒ cnt=2 (one posedge from driver-1's pulse, one when
    // driver-2 takes over at 1).
    let (out, code) = run("module top;\n\
         reg en1, en2, c1, c2; wire clk; integer cnt;\n\
         assign clk = en1 ? c1 : 1'bz;\n\
         assign clk = en2 ? c2 : 1'bz;\n\
         always @(posedge clk) cnt = cnt + 1;\n\
         initial begin cnt=0; en1=1; en2=0; c1=0; c2=0; #1; c1=1; #1; c1=0; #1; en1=0; en2=1; c2=1; #1; $display(\"cnt=%0d\", cnt); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0));
    assert!(
        out.contains("cnt=2"),
        "resolved clock must fire posedges; got:\n{out}"
    );
}

#[test]
fn partial_overlap_still_loud() {
    // OUT OF SCOPE: a whole-net driver overlapping a part-select driver stays an
    // E3001 loud reject (honest-loud — not silent-wrong). iverilog resolves it;
    // vita defers that to a follow-up slice.
    let (out, code) = run("module top;\n\
         reg a, b; wire [3:0] w;\n\
         assign w = a ? 4'hf : 4'bz;\n\
         assign w[1:0] = b ? 2'h3 : 2'bz;\n\
         initial begin a=1; b=0; #1; $display(\"w=%h\", w); $finish; end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "partial-overlap multi-driver must still loud-reject; got:\n{out}"
    );
}
