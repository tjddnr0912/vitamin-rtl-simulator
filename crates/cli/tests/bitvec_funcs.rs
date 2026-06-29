//! v7 bit-vector system functions: $countones/$onehot/$onehot0/$isunknown
//! (eval arms — x/z bits never count as 1, the result is always KNOWN 0/1)
//! and $bits (pure elaborate const-fold, IR-0). Every expected value pinned
//! LIVE against iverilog 13.0 (2026-06-12).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_bvf_{}_{n}", std::process::id()));
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
fn countones_onehot_isunknown_with_xz() {
    // 1x1z: two 1s (x/z don't count) -> co=2, not onehot, unknown=1.
    let (out, err, code) = run("module top;\n\
         reg [3:0] a;\n\
         initial begin\n\
           a = 4'b1x1z;\n\
           $display(\"co=%0d oh=%b oh0=%b unk=%b\", $countones(a), $onehot(a), $onehot0(a), $isunknown(a));\n\
           a = 4'b0100;\n\
           $display(\"co2=%0d oh2=%b oh02=%b unk2=%b\", $countones(a), $onehot(a), $onehot0(a), $isunknown(a));\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("co=2 oh=0 oh0=0 unk=1"), "got:\n{out}");
    assert!(out.contains("co2=1 oh2=1 oh02=1 unk2=0"), "got:\n{out}");
}

#[test]
fn onehot_zero_and_two_bit_cases() {
    let (out, err, code) = run("module top;\n\
         reg [3:0] a;\n\
         initial begin\n\
           a = 4'b0000;\n\
           $display(\"oh3=%b oh03=%b\", $onehot(a), $onehot0(a));\n\
           a = 4'b0110;\n\
           $display(\"oh4=%b oh04=%b\", $onehot(a), $onehot0(a));\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("oh3=0 oh03=1"), "got:\n{out}");
    assert!(out.contains("oh4=0 oh04=0"), "got:\n{out}");
}

#[test]
fn countones_multiword_65bit() {
    // Two 1s straddling the word boundary — the count walks every word.
    let (out, err, code) = run("module top;\n\
         reg [64:0] w;\n\
         initial begin\n\
           w = 65'h1_0000_0000_0000_0001;\n\
           $display(\"cow=%0d\", $countones(w));\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("cow=2"), "got:\n{out}");
}

#[test]
fn bits_const_folds_every_surface() {
    // $bits = elaborate const-fold: whole unpacked array = TOTAL bits (80),
    // element = 8, unsized param = 32, expression = self-width (4), and the
    // const contexts (localparam init, range spec) fold too. iverilog-pinned:
    // bmem=80 bel=8 bp=32 bexpr=4 blb=8 bc=4.
    let (out, err, code) = run("module top;\n\
         reg [7:0] mem [0:9];\n\
         reg [3:0] a, b;\n\
         parameter P = 100;\n\
         localparam LB = $bits(mem[0]);\n\
         reg [$bits(a)-1:0] c;\n\
         initial begin\n\
           $display(\"bmem=%0d bel=%0d bp=%0d bexpr=%0d blb=%0d bc=%0d\",\n\
             $bits(mem), $bits(mem[3]), $bits(P), $bits(a+b), LB, $bits(c));\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(
        out.contains("bmem=80 bel=8 bp=32 bexpr=4 blb=8 bc=4"),
        "got:\n{out}"
    );
}

#[test]
fn bits_of_concat_and_select() {
    // iverilog-pinned: bitsel=4 bitexpr=8.
    let (out, err, code) = run("module top;\n\
         reg [64:0] w;\n\
         reg [3:0] a;\n\
         integer n;\n\
         initial begin\n\
           n = $bits(w[3:0]);\n\
           $display(\"bitsel=%0d bitexpr=%0d\", n, $bits({a, a}));\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("bitsel=4 bitexpr=8"), "got:\n{out}");
}
