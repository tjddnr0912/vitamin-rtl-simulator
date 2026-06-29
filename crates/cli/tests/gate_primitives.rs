//! GATE: gate-level primitive instantiation (and/or/nand/nor/xor/xnor/buf/not/
//! bufif0/bufif1/notif0/notif1) desugared to continuous assigns (IEEE 1364 §7).
//! Multi-input gates reduce inputs (first terminal = output); buf/not pass/invert
//! the last terminal to every preceding output; bufif/notif drive (inverted) data
//! when the control matches, else `1'bz`. iverilog is the oracle (gates standard).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_gate_{}_{n}", std::process::id()));
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
fn basic_two_input_gates() {
    // and/or/nand/nor/xor/xnor with a=1,b=0.
    let (out, _c) = run("module t;\n\
        reg a, b; wire wa, wo, wna, wno, wx, wxn;\n\
        and  (wa, a, b);\n\
        or   (wo, a, b);\n\
        nand (wna, a, b);\n\
        nor  (wno, a, b);\n\
        xor  (wx, a, b);\n\
        xnor (wxn, a, b);\n\
        initial begin a=1; b=0; #1;\n\
          $display(\"a%b o%b na%b no%b x%b xn%b\", wa, wo, wna, wno, wx, wxn);\n\
        end endmodule\n");
    // and=0 or=1 nand=1 nor=0 xor=1 xnor=0
    assert!(
        out.contains("a0 o1 na1 no0 x1 xn0"),
        "two-input gates:\n{out}"
    );
}

#[test]
fn multi_input_and() {
    // and with 3 inputs: 1&1&1=1, then flip one input -> 0.
    let (out, _c) = run("module t;\n\
        reg a, b, c; wire o;\n\
        and (o, a, b, c);\n\
        initial begin a=1;b=1;c=1; #1; $display(\"x%b\", o); c=0; #1; $display(\"y%b\", o); end\n\
        endmodule\n");
    assert!(
        out.contains("x1") && out.contains("y0"),
        "3-input and:\n{out}"
    );
}

#[test]
fn buf_and_not_single_and_multi_output() {
    // buf passes, not inverts; buf with two outputs drives both.
    let (out, _c) = run("module t;\n\
        reg in; wire b, n, o1, o2;\n\
        buf (b, in);\n\
        not (n, in);\n\
        buf (o1, o2, in);\n\
        initial begin in=1; #1; $display(\"b%b n%b o1%b o2%b\", b, n, o1, o2); end\n\
        endmodule\n");
    // buf=1 not=0 o1=1 o2=1
    assert!(
        out.contains("b1 n0 o1 1 o2 1") || out.contains("b1 n0 o11 o21"),
        "buf/not:\n{out}"
    );
}

#[test]
fn tristate_bufif() {
    // bufif1: drive on ctrl=1 else z. bufif0: drive on ctrl=0 else z.
    let (out, _c) = run("module t;\n\
        reg d, c; wire b1, b0;\n\
        bufif1 (b1, d, c);\n\
        bufif0 (b0, d, c);\n\
        initial begin\n\
          d=1; c=1; #1; $display(\"on b1%b b0%b\", b1, b0);\n\
          c=0; #1; $display(\"off b1%b b0%b\", b1, b0);\n\
        end endmodule\n");
    // ctrl=1: bufif1 drives d=1, bufif0 is z. ctrl=0: bufif1 z, bufif0 drives d=1.
    assert!(out.contains("on b11 b0z"), "bufif ctrl=1:\n{out}");
    assert!(out.contains("off b1z b01"), "bufif ctrl=0:\n{out}");
}

#[test]
fn tristate_notif() {
    // notif1: drive ~d on ctrl=1 else z.
    let (out, _c) = run("module t;\n\
        reg d, c; wire n1;\n\
        notif1 (n1, d, c);\n\
        initial begin d=1; c=1; #1; $display(\"a%b\", n1); c=0; #1; $display(\"b%b\", n1); end\n\
        endmodule\n");
    // ctrl=1: ~d = ~1 = 0; ctrl=0: z.
    assert!(out.contains("a0") && out.contains("bz"), "notif1:\n{out}");
}

#[test]
fn vector_gate() {
    // a 4-bit and gate (bitwise across the vector).
    let (out, _c) = run("module t;\n\
        reg [3:0] a, b; wire [3:0] o;\n\
        and (o, a, b);\n\
        initial begin a=4'b1100; b=4'b1010; #1; $display(\"o=%b\", o); end\n\
        endmodule\n");
    assert!(out.contains("o=1000"), "vector and:\n{out}");
}

#[test]
fn multiple_instances_one_statement() {
    // two named instances in one statement share the gate type.
    let (out, _c) = run("module t;\n\
        reg a, b, c, d; wire o1, o2;\n\
        and g1(o1, a, b), g2(o2, c, d);\n\
        initial begin a=1;b=1;c=1;d=0; #1; $display(\"o1%b o2%b\", o1, o2); end\n\
        endmodule\n");
    assert!(
        out.contains("o1 1 o2 0") || out.contains("o11 o20"),
        "multi-instance:\n{out}"
    );
}

#[test]
fn gate_input_z_becomes_x() {
    // Adversarial-found (workflow w3hperb4n): a `z` on an active gate input must
    // become `x`, not pass through (IEEE 1364 §7.3/§7.4). buf/bufif1 used to leak z.
    let (out, _c) = run("module t;\n\
        reg d, c; wire b, b1, b0, n1;\n\
        buf    gb(b, d);\n\
        bufif1 g1(b1, d, c);\n\
        bufif0 g0(b0, d, c);\n\
        notif1 gn(n1, d, c);\n\
        initial begin d=1'bz; c=1'b1; #1; $display(\"buf%b bufif1%b notif1%b\", b, b1, n1);\n\
          c=1'b0; #1; $display(\"bufif0%b\", b0); end\n\
        endmodule\n");
    // active gate with z data ⇒ x (not z). notif1: ~z = x.
    assert!(
        out.contains("bufx bufif1x notif1x"),
        "gate z-input ⇒ x:\n{out}"
    );
    assert!(out.contains("bufif0x"), "bufif0 z-data ⇒ x:\n{out}");
}
