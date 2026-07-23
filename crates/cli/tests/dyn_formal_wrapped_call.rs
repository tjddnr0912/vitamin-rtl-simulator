//! r18 (F3): a framed dynamic-array-formal FUNCTION call in a WRAPPED position — a
//! non-blocking-assign rhs (`reg_out <= packk(b)`) or a ternary arm
//! (`reg_out <= en ? packk(b) : 64'd0`) — is now hoisted to a blessed temp, extending the
//! §4.5.179 direct-blocking-rhs hoist. Was E3009 "supported only as the DIRECT rhs of a
//! blocking assignment".
//!
//! A `?:` arm is conditionally evaluated, so hoisting the call to an unconditional temp is
//! only correct when the function is SIDE-EFFECT-FREE (else its `$display`/severity would
//! fire even when the arm is not taken). An impure dyn-formal function in a `?:` arm stays
//! LOUD (correct-or-loud).
//!
//! ORACLE: iverilog 13.0 runs these, so they are iverilog-verified.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_dfwc_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

fn is_loud(o: &str) -> bool {
    o.contains("E3009")
}

const PACKK: &str = "logic clk = 0; always #5 clk = ~clk;\n\
    logic [63:0] reg_out;\n\
    function automatic logic [63:0] packk (input byte b[]);\n\
      packk = 0; foreach (b[i]) packk += b[i];\n\
    endfunction\n";

// ── the report's F3: NBA rhs + ternary ──
#[test]
fn nba_ternary() {
    let o = run(&format!(
        "module t;\n{PACKK}\
        task automatic run (input byte b[], input bit en);\n\
          @(posedge clk); reg_out <= en ? packk(b) : 64'd0;\n\
          @(posedge clk); if (reg_out == 6) $display(\"PASS\");\n\
        endtask\n\
        initial begin byte v[]; v = new[3]; v[0]=1; v[1]=2; v[2]=3; run(v, 1); $finish; end\n\
        endmodule\n"
    ));
    assert!(!is_loud(&o) && o.contains("PASS"), "F3 repro:\n{o}");
}

// ── the ternary ELSE arm is taken (en=0) — the call value is discarded ──
#[test]
fn nba_ternary_else_taken() {
    let o = run(&format!(
        "module t;\n{PACKK}\
        task automatic run (input byte b[], input bit en);\n\
          @(posedge clk); reg_out <= en ? packk(b) : 64'd99;\n\
          @(posedge clk); if (reg_out == 99) $display(\"PASS\");\n\
        endtask\n\
        initial begin byte v[]; v = new[3]; v[0]=1; v[1]=2; v[2]=3; run(v, 0); $finish; end\n\
        endmodule\n"
    ));
    assert!(!is_loud(&o) && o.contains("PASS"), "F3 else arm:\n{o}");
}

// ── NBA with a DIRECT dyn-formal call rhs (no ternary) ──
#[test]
fn nba_direct_call() {
    let o = run(&format!(
        "module t;\n{PACKK}\
        task automatic run (input byte b[]);\n\
          @(posedge clk); reg_out <= packk(b);\n\
          @(posedge clk); if (reg_out == 6) $display(\"PASS\");\n\
        endtask\n\
        initial begin byte v[]; v = new[3]; v[0]=1; v[1]=2; v[2]=3; run(v); $finish; end\n\
        endmodule\n"
    ));
    assert!(!is_loud(&o) && o.contains("PASS"), "NBA direct:\n{o}");
}

// ── a ternary dyn-formal call on a BLOCKING assign rhs ──
#[test]
fn blocking_ternary() {
    let o = run(&format!(
        "module t;\n{PACKK}\
        initial begin\n\
          byte v[]; logic [63:0] s; bit en = 1;\n\
          v = new[3]; v[0]=1; v[1]=2; v[2]=3;\n\
          s = en ? packk(v) : 64'd0;\n\
          if (s == 6) $display(\"PASS\");\n\
          $finish;\n\
        end\n\
        endmodule\n"
    ));
    assert!(!is_loud(&o) && o.contains("PASS"), "blocking ternary:\n{o}");
}

// ── regression: the direct blocking rhs (§4.5.179/195) still works ──
#[test]
fn direct_blocking_rhs_unchanged() {
    let o = run(&format!(
        "module t;\n{PACKK}\
        task automatic run (input byte b[]);\n\
          @(posedge clk); reg_out = packk(b);\n\
          if (reg_out == 6) $display(\"PASS\");\n\
        endtask\n\
        initial begin byte v[]; v = new[3]; v[0]=1; v[1]=2; v[2]=3; run(v); $finish; end\n\
        endmodule\n"
    ));
    assert!(!is_loud(&o) && o.contains("PASS"), "direct blocking:\n{o}");
}

// ── correct-or-loud: an IMPURE dyn-formal fn ($display) in a ?: arm stays LOUD ──
#[test]
fn impure_fn_in_ternary_stays_loud() {
    let o = run("module t;\n\
        logic clk = 0; always #5 clk = ~clk;\n\
        logic [63:0] r;\n\
        function automatic logic [63:0] noisy (input byte b[]);\n\
          noisy = 0; foreach (b[i]) noisy += b[i]; $display(\"called\");\n\
        endfunction\n\
        task automatic run (input byte b[], input bit en);\n\
          @(posedge clk); r <= en ? noisy(b) : 64'd0;\n\
          @(posedge clk);\n\
        endtask\n\
        initial begin byte v[]; v = new[2]; v[0]=1; v[1]=2; run(v, 0); $finish; end\n\
        endmodule\n");
    assert!(is_loud(&o), "impure fn in ternary must stay loud:\n{o}");
}
