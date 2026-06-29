//! An unsized fill literal (`'1`/`'0`/`'x`/`'z`) used as a PARAMETER or
//! LOCALPARAM value is context-determined to the declared parameter width
//! (IEEE 1800 §5.7.1 / §11.6) — it must fill ALL declared bits, not a fixed
//! 32-bit self-determined width. Pre-fix `parameter logic [63:0] P = '1`
//! silently truncated to `ffffffff` (top 32 bits dropped). Oracle: iverilog.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_pfw_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    let so = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        out.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let mut s = String::new();
    for l in so.lines().filter(|l| !l.starts_with("simulation ended")) {
        s.push_str(l);
        s.push('\n');
    }
    s
}

#[test]
fn module_param_64_fill() {
    let out = run("module m #(parameter logic [63:0] P = '1);\n\
           initial $display(\"%h\", P);\n\
         endmodule\n\
         module top; m inst(); endmodule\n");
    assert_eq!(out, "ffffffffffffffff\n");
}

#[test]
fn localparam_64_fill() {
    let out = run("module t;\n\
           localparam logic [63:0] Q = '1;\n\
           initial $display(\"%h\", Q);\n\
         endmodule\n");
    assert_eq!(out, "ffffffffffffffff\n");
}

#[test]
fn param_48_fill() {
    let out = run("module m #(parameter logic [47:0] R = '1);\n\
           initial $display(\"%h\", R);\n\
         endmodule\n\
         module top; m inst(); endmodule\n");
    assert_eq!(out, "ffffffffffff\n");
}

#[test]
fn param_fill_zero_and_value() {
    let out = run("module t;\n\
           localparam logic [63:0] Z = '0;\n\
           localparam logic [63:0] V = 64'd5;\n\
           initial begin $display(\"%h\", Z); $display(\"%h\", V); end\n\
         endmodule\n");
    // '0 fills all 64 bits with 0; a normal value is unaffected.
    assert_eq!(out, "0000000000000000\n0000000000000005\n");
}

#[test]
fn param_override_fill() {
    let out = run("module m #(parameter logic [63:0] P = 64'd0);\n\
           initial $display(\"%h\", P);\n\
         endmodule\n\
         module top; m #(.P('1)) inst(); endmodule\n");
    assert_eq!(out, "ffffffffffffffff\n");
}

#[test]
fn untyped_param_sized_literal_width() {
    // An UNTYPED param initialized with a SIZED literal takes the literal's width
    // (`localparam P = 8'hAB` ⇒ 8 bits), so $bits and concat match iverilog. A
    // plain-decimal untyped param stays 32-bit (unchanged).
    let out = run("module m;\n\
           localparam P = 8'hAB;\n\
           localparam D = 5;\n\
           initial begin\n\
             $display(\"%0d %h\", $bits(P), {P, P});\n\
             $display(\"%0d %h\", $bits(D), {D, D});\n\
           end\n\
         endmodule\n");
    assert_eq!(out, "8 abab\n32 0000000500000005\n");
}

#[test]
fn bits_of_typed_param() {
    // $bits of a TYPED param returns its DECLARED width, not 32. The wrong width
    // also propagated into net sizing / replication counts, so probe both.
    let out = run("module m;\n\
           localparam logic [11:0] P = 12'h5;\n\
           localparam logic [3:0] Q = 4'h0;\n\
           logic [$bits(Q)-1:0] w;\n\
           initial begin\n\
             $display(\"%0d\", $bits(P));\n\
             w = '1; $display(\"%h bw=%0d\", w, $bits(w));\n\
           end\n\
         endmodule\n");
    assert_eq!(out, "12\nf bw=4\n");
}

#[test]
fn hierarchical_typed_param_read() {
    // A cross-instance read of a TYPED param (`dut.W`) materializes at the
    // declared width too, not the value-inferred 32 bits.
    let out = run("module sub;\n\
           localparam logic [63:0] W = '1;\n\
           localparam logic [3:0] N = 4'd5;\n\
         endmodule\n\
         module top;\n\
           sub d();\n\
           initial begin $display(\"%h\", d.W); $display(\"%h\", d.N); end\n\
         endmodule\n");
    assert_eq!(out, "ffffffffffffffff\n5\n");
}

#[test]
fn param_fill_in_expression() {
    // The all-ones param participating in 64-bit arithmetic must wrap correctly
    // (proves the value is genuinely 64-bit-wide, not a 32-bit artifact).
    let out = run("module t;\n\
           localparam logic [63:0] P = '1;\n\
           logic [63:0] s;\n\
           initial begin s = P + 64'd1; $display(\"%0d\", s); end\n\
         endmodule\n");
    assert_eq!(out, "0\n");
}
