//! S1: gate/assign rise·fall·turnoff delays (IEEE 1364 §7.14 / §28). A delayed
//! continuous assign or gate primitive with a 2- or 3-value delay spec applies a
//! DESTINATION-based delay per transition: a bit going →1 uses `rise`, →0 uses
//! `fall`, →z uses `turnoff`, →x uses `min(rise,fall,turnoff)`. The whole net
//! updates ATOMICALLY at `now + max(per-changed-bit delay)` (confirmed against
//! iverilog 13.0 as atomic-at-max, not per-bit separate arrival). With a 2-value
//! spec, `turnoff` defaults to `min(rise,fall)`. Before S1, vita kept only the
//! rise delay and applied it to every transition (a silent-wrong vs iverilog).
//!
//! Each case settles the net to a known value BEFORE the monitored transition, so
//! it tests the steady-state transition delay (not the orthogonal undriven-initial
//! corner). All expected timings are hand-computed and were verified byte-identical
//! against iverilog (`iverilog -g2012 … && vvp …`, timescale 1ns/1ns).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_rfd_{}_{n}", std::process::id()));
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
fn scalar_rise_vs_fall() {
    // assign #(3,7): a rising edge lands at +3, a falling edge at +7.
    let (out, _c) = run("`timescale 1ns/1ns\n\
        module t;\n\
          reg a; wire y;\n\
          assign #(3,7) y = a;\n\
          initial begin\n\
            a = 0; #20;\n\
            $monitor(\"t=%0t a=%b y=%b\", $time, a, y);\n\
            a = 1; #20;\n\
            a = 0; #20;\n\
            $finish;\n\
          end\n\
        endmodule\n");
    // rising y->1 at t=20 lands at t=23 (rise=3); falling y->0 at t=40 lands at
    // t=47 (fall=7).
    assert!(out.contains("t=23 a=1 y=1"), "rise lands at +3:\n{out}");
    assert!(out.contains("t=47 a=0 y=0"), "fall lands at +7:\n{out}");
}

#[test]
fn not_gate_fall_and_rise() {
    // not #(2,5): output y inverts a. a:0->1 makes y:1->0 (fall=5); a:1->0 makes
    // y:0->1 (rise=2).
    let (out, _c) = run("`timescale 1ns/1ns\n\
        module t;\n\
          reg a; wire y;\n\
          not #(2,5) (y, a);\n\
          initial begin\n\
            a = 0; #20;\n\
            $monitor(\"t=%0t a=%b y=%b\", $time, a, y);\n\
            a = 1; #20;\n\
            a = 0; #20;\n\
            $finish;\n\
          end\n\
        endmodule\n");
    // a:0->1 at t=20 => y:1->0 lands at t=25 (fall=5).
    assert!(
        out.contains("t=25 a=1 y=0"),
        "not falling lands at +5:\n{out}"
    );
    // a:1->0 at t=40 => y:0->1 lands at t=42 (rise=2).
    assert!(
        out.contains("t=42 a=0 y=1"),
        "not rising lands at +2:\n{out}"
    );
}

#[test]
fn bufif1_turnoff_to_z() {
    // bufif1 #(2,4,6): explicit turnoff. ctrl 1->0 drives y to z at +6 (turnoff);
    // ctrl 0->1 drives y to 1 at +2 (rise).
    let (out, _c) = run("`timescale 1ns/1ns\n\
        module t;\n\
          reg d, c; wire y;\n\
          bufif1 #(2,4,6) (y, d, c);\n\
          initial begin\n\
            d = 1; c = 1; #20;\n\
            $monitor(\"t=%0t d=%b c=%b y=%b\", $time, d, c, y);\n\
            c = 0; #20;\n\
            c = 1; #20;\n\
            $finish;\n\
          end\n\
        endmodule\n");
    // c:1->0 at t=20 => y:1->z lands at t=26 (turnoff=6).
    assert!(
        out.contains("t=26 d=1 c=0 y=z"),
        "turnoff to z at +6:\n{out}"
    );
    // c:0->1 at t=40 => y:z->1 lands at t=42 (rise=2).
    assert!(
        out.contains("t=42 d=1 c=1 y=1"),
        "rise from z at +2:\n{out}"
    );
}

#[test]
fn vector_mixed_bit_max_atomic_2_8() {
    // assign #(2,8): the whole vector updates atomically at the MAX of the
    // per-changed-bit delays. 1010 -> 0101 falls bits 1,3 (fall=8) and rises
    // bits 0,2 (rise=2): max=8, so all four bits land together at +8.
    let (out, _c) = run("`timescale 1ns/1ns\n\
        module t;\n\
          reg [3:0] a; wire [3:0] y;\n\
          assign #(2,8) y = a;\n\
          initial begin\n\
            a = 4'b0000; #20;\n\
            $monitor(\"t=%0t a=%b y=%b\", $time, a, y);\n\
            a = 4'b1111; #20;\n\
            a = 4'b0000; #20;\n\
            a = 4'b1010; #20;\n\
            a = 4'b0101; #20;\n\
            $finish;\n\
          end\n\
        endmodule\n");
    // all rise (0000->1111) at t=20 lands at t=22 (rise=2).
    assert!(out.contains("t=22 a=1111 y=1111"), "all-rise at +2:\n{out}");
    // all fall (1111->0000) at t=40 lands at t=48 (fall=8).
    assert!(out.contains("t=48 a=0000 y=0000"), "all-fall at +8:\n{out}");
    // mixed rise-only (0000->1010) at t=60 lands at t=62 (rise=2).
    assert!(
        out.contains("t=62 a=1010 y=1010"),
        "mixed-rise at +2:\n{out}"
    );
    // mixed fall+rise (1010->0101) at t=80 lands atomically at t=88 (max=8).
    assert!(
        out.contains("t=88 a=0101 y=0101"),
        "mixed fall+rise atomic at max=8:\n{out}"
    );
}

#[test]
fn vector_mixed_bit_max_atomic_8_2() {
    // assign #(8,2): symmetric to above with rise/fall swapped. 1010 -> 0101
    // falls bits 1,3 (fall=2) and rises bits 0,2 (rise=8): max=8.
    let (out, _c) = run("`timescale 1ns/1ns\n\
        module t;\n\
          reg [3:0] a; wire [3:0] y;\n\
          assign #(8,2) y = a;\n\
          initial begin\n\
            a = 4'b0000; #20;\n\
            $monitor(\"t=%0t a=%b y=%b\", $time, a, y);\n\
            a = 4'b1111; #20;\n\
            a = 4'b0000; #20;\n\
            a = 4'b1010; #20;\n\
            a = 4'b0101; #20;\n\
            $finish;\n\
          end\n\
        endmodule\n");
    // all rise lands at +8.
    assert!(out.contains("t=28 a=1111 y=1111"), "all-rise at +8:\n{out}");
    // all fall lands at +2.
    assert!(out.contains("t=42 a=0000 y=0000"), "all-fall at +2:\n{out}");
    // mixed fall+rise lands atomically at max=8.
    assert!(
        out.contains("t=88 a=0101 y=0101"),
        "mixed fall+rise atomic at max=8:\n{out}"
    );
}

#[test]
fn x_transition_uses_min() {
    // assign #(10,20,4): a bit going to x uses min(rise,fall,turnoff)=4. Settle
    // y=11, then drive bit1 to x (bit0 unchanged): lands at +4.
    let (out, _c) = run("`timescale 1ns/1ns\n\
        module t;\n\
          reg [1:0] a; wire [1:0] y;\n\
          assign #(10,20,4) y = a;\n\
          initial begin\n\
            a = 2'b11; #40;\n\
            $monitor(\"t=%0t a=%b y=%b\", $time, a, y);\n\
            a = 2'bx1; #30;\n\
            $finish;\n\
          end\n\
        endmodule\n");
    // a:11->x1 at t=40 => y bit1 1->x uses min=4, lands at t=44.
    assert!(
        out.contains("t=44 a=x1 y=x1"),
        "x-transition uses min=4:\n{out}"
    );
}

#[test]
fn two_value_turnoff_defaults_to_min() {
    // With only 2 delay values, turnoff defaults to min(rise,fall). bufif1 #(2,8):
    // turnoff = min(2,8) = 2. ctrl 1->0 drives y to z at +2.
    let (out, _c) = run("`timescale 1ns/1ns\n\
        module t;\n\
          reg d, c; wire y;\n\
          bufif1 #(2,8) (y, d, c);\n\
          initial begin\n\
            d = 1; c = 1; #20;\n\
            $monitor(\"t=%0t d=%b c=%b y=%b\", $time, d, c, y);\n\
            c = 0; #20;\n\
            $finish;\n\
          end\n\
        endmodule\n");
    // c:1->0 at t=20 => y:1->z lands at t=22 (turnoff default = min(2,8) = 2).
    assert!(
        out.contains("t=22 d=1 c=0 y=z"),
        "two-value turnoff defaults to min(rise,fall)=2:\n{out}"
    );
}

#[test]
fn uniform_single_value_unchanged() {
    // assign #5: a single-value delay stays uniform (no S1 sidecar) — every
    // transition uses 5. Byte-identical to pre-S1 behaviour.
    let (out, _c) = run("`timescale 1ns/1ns\n\
        module t;\n\
          reg a; wire y;\n\
          assign #5 y = a;\n\
          initial begin\n\
            a = 0; #20;\n\
            $monitor(\"t=%0t a=%b y=%b\", $time, a, y);\n\
            a = 1; #20;\n\
            a = 0; #20;\n\
            $finish;\n\
          end\n\
        endmodule\n");
    assert!(out.contains("t=25 a=1 y=1"), "uniform rise at +5:\n{out}");
    assert!(out.contains("t=45 a=0 y=0"), "uniform fall at +5:\n{out}");
}

#[test]
fn uniform_equal_values_unchanged() {
    // assign #(3,3): equal rise/fall folds to no sidecar (uniform). Byte-identical
    // to pre-S1.
    let (out, _c) = run("`timescale 1ns/1ns\n\
        module t;\n\
          reg a; wire y;\n\
          assign #(3,3) y = a;\n\
          initial begin\n\
            a = 0; #20;\n\
            $monitor(\"t=%0t a=%b y=%b\", $time, a, y);\n\
            a = 1; #20;\n\
            a = 0; #20;\n\
            $finish;\n\
          end\n\
        endmodule\n");
    assert!(out.contains("t=23 a=1 y=1"), "equal rise at +3:\n{out}");
    assert!(out.contains("t=43 a=0 y=0"), "equal fall at +3:\n{out}");
}

#[test]
fn inertial_supersede_measures_from_net_not_prev_rhs() {
    // CRITICAL regression (adversarial review): the per-bit delay is measured from
    // the value the net CURRENTLY holds (the gate's own last-landed output), NOT
    // the previous RHS. assign #(9,2): a=00→11 at t=10 schedules y=11 at t=19
    // (rise=9). a=11→10 at t=13 SUPERSEDES that pending write before it lands — so
    // the net still holds 00, and 00→10 needs rise=9 for bit1 → lands at t=22. A
    // baseline of the previous RHS (11) would wrongly use fall=2 → t=15 (silent).
    let (out, _c) = run("`timescale 1ns/1ns\n\
        module t;\n\
          reg [1:0] a; wire [1:0] y;\n\
          assign #(9,2) y = a;\n\
          initial begin\n\
            a = 2'b00; #10 a = 2'b11; #3 a = 2'b10; #40 $finish;\n\
          end\n\
          initial $monitor(\"t=%0t a=%b y=%b\", $time, a, y);\n\
        endmodule\n");
    assert!(
        out.contains("t=22 a=10 y=10"),
        "supersede lands at +9 from net:\n{out}"
    );
    assert!(
        !out.contains("t=15 "),
        "must NOT land early at +2 from prev RHS:\n{out}"
    );
}
