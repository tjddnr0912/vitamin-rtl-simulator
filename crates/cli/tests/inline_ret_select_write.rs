//! §5.2 row 25 (VeeR O9) — a function whose straight-line body writes a bit- or part-select
//! of its return variable, called from a continuous assignment (or any other expression the
//! inline lane lowers: `always_comb`, `always @*`, `initial`, another function's body), merges
//! each select write into the return variable's running value
//! (`elaborate/src/inline_ret_select.rs`). PRE was E3009 `function countones body is not
//! reducible to an expression (control flow)` on every cell below; VeeR EL2 `countones` and
//! EH1 `decode2_4`, `decode3_8`, `encode8_3` are the corpus witnesses (cells c01–c03 are
//! first-party spellings of their idioms).
//!
//! Oracles beside each cell: iverilog 13.0 (`-g2012`), sv2v 0.0.13 -> iverilog 13.0, verilator
//! 5.052 (`--binary`). The expected text is the two 4-state oracles' (they agree on every
//! value cell); verilator is 2-state, so it prints 0/1 where they print x, and it writes an
//! out-of-range select differently (c10, c46). PRE is vita at 9ba715e2 (md5 73ff88e8).
//!
//! Refused by name (still E3009, now with the reason): an index that is not an
//! elaboration-time constant (a runtime index needs §11.5.1's "an x index writes nothing",
//! which a merge cannot express); a body in which any name the body or its declarations
//! resolve is not declared by the body and bound before it is read (the inline lane lowers
//! the body where the CALL is; row-25 lens r1 B1/B2 — that root is pre-existing for a whole
//! write and stays a queue line); a value with an effect (it would run a different number of
//! times); a reversed part-select and a nested select. The `l_*` cells are row-25 lens r1's.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// stdout without the `simulation ended` line, success, stderr
fn run_raw(src: &str) -> (String, bool, String) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_irs_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    let so = String::from_utf8_lossy(&out.stdout);
    let s: Vec<&str> = so
        .lines()
        .filter(|l| !l.starts_with("simulation ended"))
        .collect();
    (
        s.join("\n"),
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// Runs, and prints exactly this.
fn prints(cells: &[(&str, &str, &str)]) {
    let mut bad = Vec::new();
    for (name, src, want) in cells {
        let (out, ok, err) = run_raw(src);
        if !(ok && out.trim_end() == *want) {
            bad.push(format!("{name}: stdout {out:?}\nstderr {err}"));
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n---\n"));
}

/// A refusal: at least one error, every one with this code and this text, the first at
/// this `line:col` (a cell calling the function twice gets one per call).
fn refused(cells: &[(&str, &str, &str, &str, &str)]) {
    let mut bad = Vec::new();
    for (name, src, at, code, text) in cells {
        let (out, ok, err) = run_raw(src);
        let head = format!(".sv:{at}: error[VITA-{code}]");
        let errs: Vec<&str> = err.lines().filter(|l| l.contains("error[VITA-")).collect();
        let pass = !ok
            && !errs.is_empty()
            && errs[0].contains(&head)
            && errs
                .iter()
                .all(|l| l.contains(&format!("error[VITA-{code}]")) && l.contains(text));
        if !pass {
            bad.push(format!("{name}: stdout {out:?}\nstderr {err}"));
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n---\n"));
}

/// The merge: bit, part and indexed part-selects with constant bounds, in any order with
/// whole writes, overlapping, partly or wholly out of range (only the in-range bits are
/// written, §11.5.1), on descending, offset and ascending return ranges, signed, `integer`,
/// typedef and multi-dimensional packed returns, a real value (converted at the select's
/// width), a value sized by the select (§11.6.1), a local assigned between select writes, and
/// called from a continuous assign, `always_comb`, `always @*`, `initial`, a framed function
/// and another inlined function; fill and string literal values (c57, c56). A bit the body
/// never writes is x (c06, c32).
#[test]
fn select_writes_of_the_return_variable_merge() {
    prints(&[
        (
            // iverilog: n=2
            // sv2v -> iverilog: n=2
            // verilator --binary: n=2
            "c01_countones",
            r#"module top;
  function [1:0] countones;
    input [1:0] valid;
    begin
      countones[1:0] = {1'b0, valid[1]} + {1'b0, valid[0]};
    end
  endfunction
  logic [1:0] v = 2'b11;
  wire  [1:0] n = countones(v);
  initial #1 begin $display("n=%0d", n); $finish; end
endmodule
"#,
            "n=2",
        ),
        (
            // iverilog: d0=0001 d1=0010 d2=0100 d3=1000 dx=x0x0
            // sv2v -> iverilog: d0=0001 d1=0010 d2=0100 d3=1000 dx=x0x0
            // verilator --binary: d0=0001 d1=0010 d2=0100 d3=1000 dx=0010
            "c02_decode2_4",
            r#"module top;
  function [3:0] decode2_4;
    input [1:0] in;
    begin
      decode2_4[3] =  in[1] &  in[0];
      decode2_4[2] =  in[1] & ~in[0];
      decode2_4[1] = ~in[1] &  in[0];
      decode2_4[0] = ~in[1] & ~in[0];
    end
  endfunction
  logic [1:0] s;
  wire [3:0] d = decode2_4(s);
  initial begin
    s = 0; #1 $display("d0=%b", d);
    s = 1; #1 $display("d1=%b", d);
    s = 2; #1 $display("d2=%b", d);
    s = 3; #1 $display("d3=%b", d);
    s = 2'bx1; #1 $display("dx=%b", d);
    $finish;
  end
endmodule
"#,
            "d0=0001\nd1=0010\nd2=0100\nd3=1000\ndx=x0x0",
        ),
        (
            // iverilog: s0 d=00000001 e=0 s5 d=00100000 e=5 s7 d=10000000 e=7 sx d=0x0x0000 e=xx0
            // sv2v -> iverilog: s0 d=00000001 e=0 s5 d=00100000 e=5 s7 d=10000000 e=7 sx d=0x0x0000 e=xx0
            // verilator --binary: s0 d=00000001 e=0 s5 d=00100000 e=5 s7 d=10000000 e=7 sx d=00010000 e=100
            "c03_encode8_3",
            r#"module top;
  function [2:0] encode8_3;
    input [7:0] in;
    encode8_3[2] = |in[7:4];
    encode8_3[1] = in[7] | in[6] | in[3] | in[2];
    encode8_3[0] = in[7] | in[5] | in[3] | in[1];
  endfunction
  function [7:0] decode3_8;
    input [2:0] in;
    decode3_8[7] =  in[2] &  in[1] &  in[0];
    decode3_8[6] =  in[2] &  in[1] & ~in[0];
    decode3_8[5] =  in[2] & ~in[1] &  in[0];
    decode3_8[4] =  in[2] & ~in[1] & ~in[0];
    decode3_8[3] = ~in[2] &  in[1] &  in[0];
    decode3_8[2] = ~in[2] &  in[1] & ~in[0];
    decode3_8[1] = ~in[2] & ~in[1] &  in[0];
    decode3_8[0] = ~in[2] & ~in[1] & ~in[0];
  endfunction
  logic [2:0] s;
  wire [7:0] d = decode3_8(s);
  wire [2:0] e = encode8_3(d);
  initial begin
    s = 0; #1 $display("s0 d=%b e=%0d", d, e);
    s = 5; #1 $display("s5 d=%b e=%0d", d, e);
    s = 7; #1 $display("s7 d=%b e=%0d", d, e);
    s = 3'b1x0; #1 $display("sx d=%b e=%b", d, e);
    $finish;
  end
endmodule
"#,
            "s0 d=00000001 e=0\ns5 d=00100000 e=5\ns7 d=10000000 e=7\nsx d=0x0x0000 e=xx0",
        ),
        (
            // iverilog: y=90
            // sv2v -> iverilog: y=90
            // verilator --binary: y=90
            "c04_partsel_const",
            r#"module top;
  function [7:0] f;
    input [3:0] a, b;
    begin
      f[7:4] = a;
      f[3:0] = b + 4'd1;
    end
  endfunction
  logic [3:0] a = 4'h9, b = 4'hf;
  wire [7:0] y = f(a, b);
  initial #1 begin $display("y=%h", y); $finish; end
endmodule
"#,
            "y=90",
        ),
        (
            // iverilog: y=x5 z=x6
            // sv2v -> iverilog: y=x5 z=x6
            // verilator --binary: y=05 z=06
            "c06_partial_cover_twice",
            r#"module top;
  function [7:0] f;
    input [3:0] a;
    f[3:0] = a;
  endfunction
  logic [3:0] a = 4'h5, b = 4'h6;
  wire [7:0] y = f(a);
  wire [7:0] z = f(b);
  initial #1 begin $display("y=%h z=%h", y, z); $finish; end
endmodule
"#,
            "y=x5 z=x6",
        ),
        (
            // iverilog: y=7b
            // sv2v -> iverilog: y=7b
            // verilator --binary: y=7b
            "c07_whole_then_sel",
            r#"module top;
  function [7:0] f;
    input [7:0] a;
    input b;
    begin
      f = a;
      f[2] = b;
      f[7:6] = 2'b01;
    end
  endfunction
  logic [7:0] a = 8'hff; logic b = 0;
  wire [7:0] y = f(a, b);
  initial #1 begin $display("y=%h", y); $finish; end
endmodule
"#,
            "y=7b",
        ),
        (
            // iverilog: y=3c
            // sv2v -> iverilog: y=3c
            // verilator --binary: y=3c
            "c08_sel_then_whole",
            r#"module top;
  function [7:0] f;
    input [7:0] a;
    begin
      f[3:0] = 4'h5;
      f = a;
    end
  endfunction
  logic [7:0] a = 8'h3c;
  wire [7:0] y = f(a);
  initial #1 begin $display("y=%h", y); $finish; end
endmodule
"#,
            "y=3c",
        ),
        (
            // iverilog: y=11001011
            // sv2v -> iverilog: y=11001011
            // verilator --binary: y=11001011
            "c09_overlap",
            r#"module top;
  function [7:0] f;
    input [7:0] a;
    input [1:0] b;
    begin
      f[7:0] = a;
      f[5:2] = 4'h0;
      f[3:2] = b;
      f[3] = 1'b1;
    end
  endfunction
  logic [7:0] a = 8'hff; logic [1:0] b = 2'b10;
  wire [7:0] y = f(a, b);
  initial #1 begin $display("y=%b", y); $finish; end
endmodule
"#,
            "y=11001011",
        ),
        (
            // iverilog: y=0111
            // sv2v -> iverilog: y=0111
            // verilator --binary: y=0101
            "c10_oor_const",
            r#"module top;
  function [3:0] f;
    input [3:0] a;
    begin
      f = a;
      f[5] = 1'b0;
      f[6:3] = 4'h0;
      f[-1] = 1'b0;
    end
  endfunction
  logic [3:0] a = 4'hf;
  wire [3:0] y = f(a);
  initial #1 begin $display("y=%b", y); $finish; end
endmodule
"#,
            "y=0111",
        ),
        (
            // iverilog: y=1100
            // sv2v -> iverilog: y=1100
            // verilator --binary: y=0111
            "c46_neg_index",
            r#"module top;
  function [3:0] f(input [3:0] a);
    f = a;
    f[-1] = 1'b0;
    f[1 -: 3] = 3'b000;
  endfunction
  logic [3:0] a = 4'hf;
  wire [3:0] y = f(a);
  initial #1 begin $display("y=%b", y); $finish; end
endmodule
"#,
            "y=1100",
        ),
        (
            // iverilog: y=1110
            // sv2v -> iverilog: y=1110
            // verilator --binary: y=1110
            "c59_low_clip_value",
            r#"module top;
  function [3:0] f(input [3:0] a);
    f = a;
    f[1 -: 3] = 3'b101;
    f[4 +: 2] = 2'b10;
  endfunction
  logic [3:0] a = 4'hf;
  wire [3:0] y = f(a);
  initial #1 begin $display("y=%b", y); $finish; end
endmodule
"#,
            "y=1110",
        ),
        (
            // iverilog: y=00000011
            // sv2v -> iverilog: y=00000011
            // verilator --binary: y=00000011
            "c47_wrap_index",
            r#"module top;
  function [7:0] f(input a);
    f = 8'h00;
    f[4'd15 + 4'd2] = a;
    f[3'd7 + 3'd1 +: 2] = {a, a};
  endfunction
  logic a = 1'b1;
  wire [7:0] y = f(a);
  initial #1 begin $display("y=%b", y); $finish; end
endmodule
"#,
            "y=00000011",
        ),
        (
            // iverilog: y=01101001 z=01100001
            // sv2v -> iverilog: y=01101001 z=01100001
            // verilator --binary: y=01101001 z=01100001
            "c19_offset_range",
            r#"module top;
  function [8:1] f;
    input [3:0] a;
    begin
      f[8:5] = a;
      f[4:1] = ~a;
      f[1] = 1'b1;
    end
  endfunction
  function [0:7] g;
    input [3:0] a;
    begin
      g[0:3] = a;
      g[4:7] = 4'h0;
      g[7] = 1'b1;
    end
  endfunction
  logic [3:0] a = 4'h6;
  wire [7:0] y = f(a);
  wire [7:0] z = g(a);
  initial #1 begin $display("y=%b z=%b", y, z); $finish; end
endmodule
"#,
            "y=01101001 z=01100001",
        ),
        (
            // iverilog: y=-14 z=-14
            // sv2v -> iverilog: y=-14 z=-14
            // verilator --binary: y=-14 z=-14
            "c20_signed",
            r#"module top;
  function signed [7:0] f;
    input [3:0] a;
    begin
      f[7:4] = 4'hf;
      f[3:0] = a;
    end
  endfunction
  logic [3:0] a = 4'h2;
  wire signed [15:0] y = f(a);
  wire signed [15:0] z = f(a) + 16'sd0;
  initial #1 begin $display("y=%0d z=%0d", y, z); $finish; end
endmodule
"#,
            "y=-14 z=-14",
        ),
        (
            // iverilog: d=0010 e=1101 d=0100 e=1011
            // sv2v -> iverilog: d=0010 e=1101 d=0100 e=1011
            // verilator --binary: d=0010 e=1101 d=0100 e=1011
            "c21_always_comb",
            r#"module top;
  function [3:0] decode2_4;
    input [1:0] in;
    begin
      decode2_4[3] =  in[1] &  in[0];
      decode2_4[2] =  in[1] & ~in[0];
      decode2_4[1] = ~in[1] &  in[0];
      decode2_4[0] = ~in[1] & ~in[0];
    end
  endfunction
  logic [1:0] s; logic [3:0] d, e;
  always_comb d = decode2_4(s);
  always @* e = decode2_4(s) ^ 4'b1111;
  initial begin
    s = 1; #1 $display("d=%b e=%b", d, e);
    s = 2; #1 $display("d=%b e=%b", d, e);
    $finish;
  end
endmodule
"#,
            "d=0010 e=1101\nd=0100 e=1011",
        ),
        (
            // iverilog: y=51
            // sv2v -> iverilog: y=51
            // verilator --binary: y=51
            "c27_typedef_ret",
            r#"module top;
  typedef logic [7:0] byte_t;
  function byte_t f;
    input [3:0] a;
    begin
      f[7:4] = a;
      f[3:0] = 4'h1;
    end
  endfunction
  logic [3:0] a = 4'h5;
  wire [7:0] y = f(a);
  initial #1 begin $display("y=%h", y); $finish; end
endmodule
"#,
            "y=51",
        ),
        (
            // iverilog: y=3c
            // sv2v -> iverilog: y=3c
            // verilator --binary: y=3c
            "c28_mdpacked_ret",
            r#"module top;
  function [1:0][3:0] f;
    input [3:0] a;
    begin
      f[1] = a;
      f[0] = ~a;
    end
  endfunction
  logic [3:0] a = 4'h3;
  wire [7:0] y = f(a);
  initial #1 begin $display("y=%h", y); $finish; end
endmodule
"#,
            "y=3c",
        ),
        (
            // iverilog: y=xx01
            // sv2v -> iverilog: y=xx01
            // verilator --binary: y=0001
            "c32_x_value_partial",
            r#"module top;
  function [3:0] f;
    input [1:0] a;
    f[1:0] = a;
  endfunction
  logic [1:0] a = 2'b01;
  wire [3:0] y = f(a);
  initial #1 begin $display("y=%b", y); $finish; end
endmodule
"#,
            "y=xx01",
        ),
        (
            // iverilog: y=0000 y=xxxx
            // sv2v -> iverilog: y=0000 y=xxxx
            // verilator --binary: y=0000 y=1110
            "c33_mixed_calls_x",
            r#"module top;
  function [3:0] f;
    input [1:0] a;
    input b;
    begin
      f[3:2] = a;
      f[1] = b;
      f[0] = ^a;
    end
  endfunction
  logic [1:0] a; logic b;
  wire [3:0] y = f(a, b) + f(~a, ~b);
  initial begin
    a = 2'b10; b = 1; #1 $display("y=%b", y);
    a = 2'bz0; b = 0; #1 $display("y=%b", y);
    $finish;
  end
endmodule
"#,
            "y=0000\ny=xxxx",
        ),
        (
            // iverilog: y=0100 z=01001011 q=0010
            // sv2v -> iverilog: y=0100 z=01001011 q=0010
            // verilator --binary: y=0100 z=01001011 q=0010
            "c39_called_from_frame",
            r#"module top;
  function [3:0] decode2_4;
    input [1:0] in;
    begin
      decode2_4[3] =  in[1] &  in[0];
      decode2_4[2] =  in[1] & ~in[0];
      decode2_4[1] = ~in[1] &  in[0];
      decode2_4[0] = ~in[1] & ~in[0];
    end
  endfunction
  function automatic [3:0] wrap(input [1:0] s);
    if (s == 2'd3) wrap = 4'hf;
    else wrap = decode2_4(s);
  endfunction
  function [7:0] both(input [1:0] s);
    both = {decode2_4(s), ~decode2_4(s)};
  endfunction
  logic [1:0] s = 2'd2;
  wire [3:0] y = wrap(s);
  wire [7:0] z = both(s);
  logic [3:0] q;
  initial begin q = decode2_4(2'd1); #1 $display("y=%b z=%b q=%b", y, z, q); $finish; end
endmodule
"#,
            "y=0100 z=01001011 q=0010",
        ),
        (
            // iverilog: y=-14 z=2
            // sv2v -> iverilog: y=-14 z=2
            // verilator --binary: y=-14 z=2
            "c42_integer_ret",
            r#"module top;
  function integer f(input [3:0] a);
    f = -1;
    f[3:0] = a;
  endfunction
  logic [3:0] a = 4'h2;
  wire [31:0] y = f(a);
  wire [3:0] z = f(a);
  initial #1 begin $display("y=%0d z=%0d", $signed(y), z); $finish; end
endmodule
"#,
            "y=-14 z=2",
        ),
        (
            // iverilog: y=31
            // sv2v -> iverilog: y=31
            // verilator --binary: y=31
            "c43_real_rhs",
            r#"module top;
  function [7:0] f(input [3:0] a);
    f = 8'h00;
    f[7:4] = 2.6;
    f[3:0] = a;
  endfunction
  logic [3:0] a = 4'h1;
  wire [7:0] y = f(a);
  initial #1 begin $display("y=%h", y); $finish; end
endmodule
"#,
            "y=31",
        ),
        (
            // iverilog: y=01001010
            // sv2v -> iverilog: y=01001010
            // verilator --binary: y=01001010
            "c44_ctx_width",
            r#"module top;
  function [7:0] f(input [3:0] a, input [3:0] b);
    f = 8'h00;
    f[7:3] = (a + b) >> 1;
    f[1:0] = a + b;
  endfunction
  logic [3:0] a = 4'hf, b = 4'h3;
  wire [7:0] y = f(a, b);
  initial #1 begin $display("y=%b", y); $finish; end
endmodule
"#,
            "y=01001010",
        ),
        (
            // iverilog: y=01100110
            // sv2v -> iverilog: y=01100110
            // verilator --binary: y=01100110
            "c52_sel_then_local",
            r#"module top;
  function [7:0] f(input [3:0] a, input [3:0] b);
    reg [3:0] t;
    begin
      f[7:4] = a;
      t = a ^ b;
      f[3:0] = t;
      t = ~t;
      f[1:0] = t[1:0];
    end
  endfunction
  logic [3:0] a = 4'h6, b = 4'h3;
  wire [7:0] y = f(a, b);
  initial #1 begin $display("y=%b", y); $finish; end
endmodule
"#,
            "y=01100110",
        ),
        (
            // iverilog: y=6162 ab z=6162
            // sv2v -> iverilog: y=6162 ab z=6162
            // verilator --binary: y=6162 ab z=6162
            "c56_string_lit",
            r#"module top;
  function [15:0] f(input a);
    f[15:8] = "a";
    f[7:0] = "b";
  endfunction
  function [15:0] h(input a);
    h = "ab";
  endfunction
  wire [15:0] y = f(1'b0);
  wire [15:0] z = h(1'b0);
  initial #1 begin $display("y=%h %s z=%h", y, y, z); $finish; end
endmodule
"#,
            "y=6162 ab z=6162",
        ),
        (
            // iverilog: y=1111zzzz0001
            // sv2v -> iverilog: y=1111zzzz0001
            // verilator --binary: y=111100000001
            "c57_fill_lits",
            r#"module top;
  function [11:0] f(input [3:0] a);
    f[11:8] = '1;
    f[7:4] = 'z;
    f[3:0] = a + '1;
  endfunction
  logic [3:0] a = 4'h2;
  wire [11:0] y = f(a);
  initial #1 begin $display("y=%b", y); $finish; end
endmodule
"#,
            "y=1111zzzz0001",
        ),
        (
            // iverilog: y=11011100
            // sv2v -> iverilog: y=11011100
            // verilator --binary: y=11011100
            "c58_unary_value",
            r#"module top;
  function [7:0] f(input [3:0] a, input signed [3:0] b);
    f[7:4] = -a;
    f[3:0] = b >>> 1;
    f[0] = &a;
  endfunction
  logic [3:0] a = 4'h3; logic signed [3:0] b = -4'sd6;
  wire [7:0] y = f(a, b);
  initial #1 begin $display("y=%b", y); $finish; end
endmodule
"#,
            "y=11011100",
        ),
        (
            // iverilog: y=00001000
            // sv2v -> iverilog: y=00001000
            // verilator --binary: y=00001000
            "c55_pkg_ret_range",
            r#"package p;
  localparam W = 4;
  function [W-1:0] dec(input [1:0] in);
    dec[0] = ~in[1] & ~in[0];
    dec[1] = ~in[1] &  in[0];
    dec[2] =  in[1] & ~in[0];
    dec[3] =  in[1] &  in[0];
  endfunction
endpackage
module top;
  localparam W = 2;
  import p::dec;
  logic [1:0] s = 2'd3;
  wire [7:0] y = dec(s);
  initial #1 begin $display("y=%b", y); $finish; end
endmodule
"#,
            "y=00001000",
        ),
        (
            // iverilog: y=0f
            // sv2v -> iverilog: y=0f
            // verilator --binary: y=0f
            "l_q3x_local_range_param_pkg",
            r#"package p;
  localparam W = 4;
  function [7:0] f(input [7:0] a);
    reg [W-1:0] t;
    begin
      t = a;
      f[7:0] = t;
    end
  endfunction
endpackage
module top;
  import p::f;
  localparam W = 8;
  wire [7:0] y = f(8'hFF);
  initial #1 begin $display("y=%h", y); $finish; end
endmodule
"#,
            "y=0f",
        ),
    ]);
}

/// The index, bounds and width must fold to constants without naming a formal: a runtime
/// index (c12, c13), an x index (c11) and a formal shadowing a parameter (c31) are refused.
#[test]
fn a_select_index_that_is_not_a_constant_is_refused() {
    refused(&[
        (
            // iverilog: y=1111
            // sv2v -> iverilog: y=1111
            // verilator --binary: %Error: Internal Error: t.sv:6:9: ../V3Number.cpp:1063: toUInt with 4-state 1'bx
            "c11_x_index_const",
            r#"module top;
  function [3:0] f;
    input [3:0] a;
    begin
      f = a;
      f[1'bx] = 1'b0;
    end
  endfunction
  logic [3:0] a = 4'hf;
  wire [3:0] y = f(a);
  initial #1 begin $display("y=%b", y); $finish; end
endmodule
"#,
            "1:8",
            "E3009",
            "whose index is not an elaboration-time constant",
        ),
        (
            // iverilog: i2 y=0100 ix y=0000
            // sv2v -> iverilog: i2 y=0100 ix y=0000
            // verilator --binary: i2 y=0100 ix y=0001
            "c12_var_index",
            r#"module top;
  function [3:0] f;
    input [1:0] i;
    begin
      f = 4'b0000;
      f[i] = 1'b1;
    end
  endfunction
  logic [1:0] i;
  wire [3:0] y = f(i);
  initial begin
    i = 2; #1 $display("i2 y=%b", y);
    i = 2'bx0; #1 $display("ix y=%b", y);
    $finish;
  end
endmodule
"#,
            "1:8",
            "E3009",
            "whose index is not an elaboration-time constant",
        ),
        (
            // iverilog: i2 y=00110000 ix y=00000000
            // sv2v -> iverilog: i2 y=00110000 ix y=00000000
            // verilator --binary: i2 y=00110000 ix y=00001100
            "c13_var_idxpart",
            r#"module top;
  function [7:0] f;
    input [1:0] i;
    input [1:0] v;
    begin
      f = 8'h00;
      f[i*2 +: 2] = v;
    end
  endfunction
  logic [1:0] i, v;
  wire [7:0] y = f(i, v);
  initial begin
    i = 2; v = 3; #1 $display("i2 y=%b", y);
    i = 2'bx1; #1 $display("ix y=%b", y);
    $finish;
  end
endmodule
"#,
            "1:8",
            "E3009",
            "whose index is not an elaboration-time constant",
        ),
        (
            // iverilog: y=0010
            // sv2v -> iverilog: y=0010
            // verilator --binary: y=0010
            "c31_formal_shadow_param",
            r#"module top;
  localparam K = 3;
  function [3:0] f;
    input [1:0] K;
    begin
      f = 4'b0000;
      f[K] = 1'b1;
    end
  endfunction
  logic [1:0] s = 2'd1;
  wire [3:0] y = f(s);
  initial #1 begin $display("y=%b", y); $finish; end
endmodule
"#,
            "1:8",
            "E3009",
            "whose index is not an elaboration-time constant",
        ),
    ]);
}

/// Every name the body or its declarations resolve must be declared by the body and bound
/// before it is read: the inline lane lowers the body where the CALL is, so a name that
/// reaches it unbound resolves to whatever the caller has under it. Refused: a parameter
/// (c05, c30), a module signal (c24), a package constant of an imported routine (c38, c40),
/// a generate block's constant (c49); a local read before its first assignment, binding a
/// module constant (q3a, q3d), a genvar (q3b, q3o), a module net (q3c, q3e), a generate
/// constant (q3p), the same under `always_comb` (q3ab) or a caller's named-block variable
/// (c61); a block-local read after its block (q3t); a read of the return variable (c18).
/// Row-25 lens r1 measured every q3* cell a silent wrong value on the round-1 merge.
#[test]
fn a_select_writing_body_that_reads_an_unbound_name_is_refused() {
    refused(&[
        (
            // iverilog: y=a63
            // sv2v -> iverilog: y=a63
            // verilator --binary: y=a63
            "c05_idxpart_const",
            r#"module top;
  localparam W = 4;
  function [11:0] f;
    input [3:0] a, b, c;
    begin
      f[0 +: W] = a;
      f[11 -: W] = c;
      f[W +: W] = b ^ a;
    end
  endfunction
  logic [3:0] a = 4'h3, b = 4'h5, c = 4'ha;
  wire [11:0] y = f(a, b, c);
  initial #1 begin $display("y=%h", y); $finish; end
endmodule
"#,
            "1:8",
            "E3009",
            "reads a name it does not declare and bind itself before the read",
        ),
        (
            // iverilog: y=1010 y=1010
            // sv2v -> iverilog: y=1010 y=1010
            // verilator --binary: y=1010 y=1001
            "c24_reads_global",
            r#"module top;
  logic g = 1'b1;
  function [3:0] f;
    input [1:0] in;
    begin
      f[3:2] = in;
      f[1] = g;
      f[0] = ~g;
    end
  endfunction
  logic [1:0] s = 2'b10;
  wire [3:0] y = f(s);
  initial begin #1 $display("y=%b", y); g = 0; #1 $display("y=%b", y); $finish; end
endmodule
"#,
            "1:8",
            "E3009",
            "reads a name it does not declare and bind itself before the read",
        ),
        (
            // iverilog: y=0111
            // sv2v -> iverilog: y=0111
            // verilator --binary: y=0111
            "c30_param_index",
            r#"module top #(parameter K = 2);
  function [3:0] f;
    input a;
    begin
      f = 4'b0000;
      f[K] = a;
      f[K-1 -: 2] = {a, a};
    end
  endfunction
  logic a = 1'b1;
  wire [3:0] y = f(a);
  initial #1 begin $display("y=%b", y); $finish; end
endmodule
"#,
            "1:8",
            "E3009",
            "reads a name it does not declare and bind itself before the read",
        ),
        (
            // iverilog: b=0010
            // sv2v -> iverilog: b=0010
            // verilator --binary: b=0010
            "c38_import_pkg",
            r#"package p;
  localparam K = 1;
  function [3:0] decs(input [1:0] in);
    decs[3] =  in[1] &  in[0];
    decs[K+1] =  in[1] & ~in[0];
    decs[K] = ~in[1] &  in[0];
    decs[0] = ~in[1] & ~in[0];
  endfunction
endpackage
module top;
  import p::*;
  logic [1:0] s = 2'd1;
  wire [3:0] b = decs(s);
  initial #1 begin $display("b=%b", b); $finish; end
endmodule
"#,
            "10:8",
            "E3009",
            "reads a name it does not declare and bind itself before the read",
        ),
        (
            // iverilog: y=0010 z=0010
            // sv2v -> iverilog: y=0010 z=0010
            // verilator --binary: y=0010 z=0010
            "c40_pkg_param_shadow",
            r#"package p;
  localparam K = 1;
  function [3:0] g(input a);
    g = 4'b0000;
    g[K] = a;
  endfunction
endpackage
module top;
  localparam K = 3;
  import p::g;
  logic a = 1'b1;
  wire [3:0] y = g(a);
  wire [3:0] z = p::g(a);
  initial #1 begin $display("y=%b z=%b", y, z); $finish; end
endmodule
"#,
            "8:8",
            "E3009",
            "reads a name it does not declare and bind itself before the read",
        ),
        (
            // iverilog: y=0010 z=0001
            // sv2v -> iverilog: y=0010 z=0001
            // verilator --binary: y=0010 z=0001
            "c49_gen_shadow",
            r#"module top;
  localparam K = 1;
  function [3:0] f(input a);
    f = 4'b0000;
    f[K] = a;
  endfunction
  function [3:0] h(input a);
    h = K;
  endfunction
  logic a = 1'b1;
  generate if (1) begin : g
    localparam K = 3;
    wire [3:0] y = f(a);
    wire [3:0] z = h(a);
  end endgenerate
  initial #1 begin $display("y=%b z=%b", g.y, g.z); $finish; end
endmodule
"#,
            "1:8",
            "E3009",
            "reads a name it does not declare and bind itself before the read",
        ),
        (
            // iverilog: y=0000
            // sv2v -> iverilog: y=0000
            // verilator --binary: y=0001
            "l_q3a_unassigned_local_idx_param",
            r#"module top;
  localparam K = 1;
  function [3:0] f(input a);
    reg [1:0] K;
    begin
      f = 4'b0000;
      f[K] = a;
    end
  endfunction
  logic a = 1'b1;
  wire [3:0] y = f(a);
  initial #1 begin $display("y=%b", y); $finish; end
endmodule
"#,
            "1:8",
            "E3009",
            "reads a name it does not declare and bind itself before the read",
        ),
        (
            // iverilog: y=0110xxxx
            // sv2v -> iverilog: y=0110xxxx
            // verilator --binary: y=01100000
            "l_q3ab_always_comb_local",
            r#"module top;
  localparam [3:0] T = 4'b1010;
  function [7:0] f(input [3:0] a);
    reg [3:0] T;
    begin
      f[7:4] = a;
      f[3:0] = T;
    end
  endfunction
  logic [3:0] a = 4'h6; logic [7:0] y;
  always_comb y = f(a);
  initial begin #1 $display("y=%b", y); $finish; end
endmodule
"#,
            "11:15",
            "E3009",
            "reads a name it does not declare and bind itself before the read",
        ),
        (
            // iverilog: y0=0000 y1=0000
            // sv2v -> iverilog: y0=0000 y1=0000
            // verilator --binary: y0=0001 y1=0001
            "l_q3b_unassigned_local_idx_genvar",
            r#"module top;
  function [3:0] f(input a);
    integer i;
    begin
      f = 4'b0000;
      f[i] = a;
    end
  endfunction
  wire [3:0] y0, y1;
  for (genvar i = 0; i < 2; i++) begin : g
    wire [3:0] w = f(1'b1);
  end
  assign y0 = g[0].w; assign y1 = g[1].w;
  initial #1 begin $display("y0=%b y1=%b", y0, y1); $finish; end
endmodule
"#,
            "1:8",
            "E3009",
            "reads a name it does not declare and bind itself before the read",
        ),
        (
            // iverilog: y=0110xxxx
            // sv2v -> iverilog: y=0110xxxx
            // verilator --binary: y=01100000
            "l_q3c_unassigned_local_val_net",
            r#"module top;
  wire [3:0] t = 4'b1010;
  function [7:0] f(input [3:0] a);
    reg [3:0] t;
    begin
      f[7:4] = a;
      f[3:0] = t;
    end
  endfunction
  logic [3:0] a = 4'h6;
  wire [7:0] y = f(a);
  initial #1 begin $display("y=%b", y); $finish; end
endmodule
"#,
            "1:8",
            "E3009",
            "reads a name it does not declare and bind itself before the read",
        ),
        (
            // iverilog: y=0110xxxx
            // sv2v -> iverilog: y=0110xxxx
            // verilator --binary: y=01100000
            "l_q3d_unassigned_local_val_param",
            r#"module top;
  localparam [3:0] T = 4'b1010;
  function [7:0] f(input [3:0] a);
    reg [3:0] T;
    begin
      f[7:4] = a;
      f[3:0] = T;
    end
  endfunction
  logic [3:0] a = 4'h6;
  wire [7:0] y = f(a);
  initial #1 begin $display("y=%b", y); $finish; end
endmodule
"#,
            "1:8",
            "E3009",
            "reads a name it does not declare and bind itself before the read",
        ),
        (
            // iverilog: y=0110xxxx y=00110110
            // sv2v -> iverilog: y=0110xxxx y=00110110
            // verilator --binary: y=01100110 y=00110011
            "l_q3e_read_before_write_local_net",
            r#"module top;
  wire [3:0] t = 4'b1010;
  function [7:0] f(input [3:0] a);
    reg [3:0] t;
    begin
      f[3:0] = t;
      t = a;
      f[7:4] = a;
    end
  endfunction
  logic [3:0] a = 4'h6;
  wire [7:0] y = f(a);
  initial begin #1 $display("y=%b", y); a = 4'h3; #1 $display("y=%b", y); $finish; end
endmodule
"#,
            "1:8",
            "E3009",
            "reads a name it does not declare and bind itself before the read",
        ),
        (
            // iverilog: y0=0000 y1=0000
            // sv2v -> iverilog: y0=0000 y1=0000
            // verilator --binary: y0=0001 y1=0001
            "l_q3o_blocklocal_unassigned_genvar",
            r#"module top;
  function [3:0] f(input a);
    begin
      f = 4'b0000;
      begin : b
        reg [1:0] i;
        f[i +: 1] = a;
      end
    end
  endfunction
  for (genvar i = 0; i < 2; i++) begin : g
    wire [3:0] w = f(1'b1);
  end
  initial #1 begin $display("y0=%b y1=%b", g[0].w, g[1].w); $finish; end
endmodule
"#,
            "1:8",
            "E3009",
            "reads a name it does not declare and bind itself before the read",
        ),
        (
            // iverilog: w=0110xxxx
            // sv2v -> iverilog: w=0110xxxx
            // verilator --binary: w=01100000
            "l_q3p_local_shadow_gen_param_val",
            r#"module top;
  function [7:0] f(input [3:0] a);
    reg [3:0] Q;
    begin
      f[7:4] = a;
      f[3:0] = Q;
    end
  endfunction
  for (genvar i = 0; i < 1; i++) begin : g
    localparam [3:0] Q = 4'b1001;
    wire [7:0] w = f(4'h6);
  end
  initial #1 begin $display("w=%b", g[0].w); $finish; end
endmodule
"#,
            "1:8",
            "E3009",
            "reads a name it does not declare and bind itself before the read",
        ),
        (
            // iverilog: y=01101001
            // sv2v -> iverilog: y=01101001
            // verilator --binary: y=01101001
            "l_q3t_block_local_after_exit",
            r#"module top;
  wire [3:0] t = 4'b1001;
  function [7:0] f(input [3:0] a);
    begin
      begin : b reg [3:0] t; t = a; f[7:4] = t; end
      f[3:0] = t;
    end
  endfunction
  logic [3:0] a = 4'h6;
  wire [7:0] y = f(a);
  initial #1 begin $display("y=%b", y); $finish; end
endmodule
"#,
            "1:8",
            "E3009",
            "reads a name it does not declare and bind itself before the read",
        ),
        (
            // iverilog: y=Xf
            // sv2v -> iverilog: y=Xf
            // verilator --binary: y=0f
            "c61_caller_block_var",
            r#"module top;
  localparam W = 4;
  function [7:0] f(input [7:0] a);
    reg [3:0] t;
    reg [7:0] u;
    begin
      t = a;
      f[7:0] = t;
      f[7] = u[0];
    end
  endfunction
  logic [7:0] y;
  initial begin : b
    reg [7:0] u;
    u = 8'h01;
    y = f(8'hFF);
    $display("y=%h", y);
    $finish;
  end
endmodule
"#,
            "16:5",
            "E3009",
            "reads a name it does not declare and bind itself before the read",
        ),
        (
            // iverilog: y=1101
            // sv2v -> iverilog: y=1101
            // verilator --binary: y=1101
            "c18_read_ret",
            r#"module top;
  function [3:0] f;
    input [3:0] a;
    begin
      f[1:0] = a[1:0];
      f[3:2] = f[1:0] ^ a[3:2];
    end
  endfunction
  logic [3:0] a = 4'b1001;
  wire [3:0] y = f(a);
  initial #1 begin $display("y=%b", y); $finish; end
endmodule
"#,
            "1:8",
            "E3009",
            "reads a name it does not declare and bind itself before the read",
        ),
    ]);
}

/// The fold keeps one flat name space and folds every declaration range where the call is:
/// a name declared twice (sibling block-locals q3q, a block-local over a formal q3r), a
/// formal's or local's range naming a module constant (q3v, q3w, q3z) or typed by a module
/// typedef (q4b), and a local with an initializer (q3f, a static initializer runs once) are
/// refused.
#[test]
fn a_select_writing_body_whose_declarations_bind_at_the_call_is_refused() {
    refused(&[
        (
            // iverilog: y=xxxx0110
            // sv2v -> iverilog: y=xxxx0110
            // verilator --binary: y=00000110
            "l_q3q_sibling_block_local",
            r#"module top;
  function [7:0] f(input [3:0] a);
    begin
      begin : b1 reg [3:0] t; t = a; f[3:0] = t; end
      begin : b2 reg [3:0] t; f[7:4] = t; end
    end
  endfunction
  logic [3:0] a = 4'h6;
  wire [7:0] y = f(a);
  initial #1 begin $display("y=%b", y); $finish; end
endmodule
"#,
            "1:8",
            "E3009",
            "declares one name twice",
        ),
        (
            // iverilog: y=0110xxxx
            // sv2v -> iverilog: y=0110xxxx
            // verilator --binary: y=01100000
            "l_q3r_block_local_shadows_formal",
            r#"module top;
  function [7:0] f(input [3:0] a);
    begin
      f[7:4] = a;
      begin : b reg [3:0] a; f[3:0] = a; end
    end
  endfunction
  logic [3:0] a = 4'h6;
  wire [7:0] y = f(a);
  initial #1 begin $display("y=%b", y); $finish; end
endmodule
"#,
            "1:8",
            "E3009",
            "declares one name twice",
        ),
        (
            // iverilog: y=0f w=0f
            // sv2v -> iverilog: y=0f w=0f
            // verilator --binary: y=0f w=0f
            "l_q3v_local_range_param_gen",
            r#"module top;
  localparam W = 4;
  function [7:0] f(input [7:0] a);
    reg [W-1:0] t;
    begin
      t = a;
      f[7:0] = t;
    end
  endfunction
  for (genvar i = 0; i < 1; i++) begin : g
    localparam W = 8;
    wire [7:0] w = f(8'hFF);
  end
  wire [7:0] y = f(8'hFF);
  initial #1 begin $display("y=%h w=%h", y, g[0].w); $finish; end
endmodule
"#,
            "1:8",
            "E3009",
            "declares a formal or local whose range or type names a constant",
        ),
        (
            // iverilog: y=0f w=0f
            // sv2v -> iverilog: y=0f w=0f
            // verilator --binary: y=0f w=0f
            "l_q3w_formal_range_param_gen",
            r#"module top;
  localparam W = 4;
  function [7:0] f(input [W-1:0] a);
    begin
      f = 8'h00;
      f[7:0] = a;
    end
  endfunction
  for (genvar i = 0; i < 1; i++) begin : g
    localparam W = 8;
    wire [7:0] w = f(8'hFF);
  end
  wire [7:0] y = f(8'hFF);
  initial #1 begin $display("y=%h w=%h", y, g[0].w); $finish; end
endmodule
"#,
            "1:8",
            "E3009",
            "declares a formal or local whose range or type names a constant",
        ),
        (
            // iverilog: y=Xf
            // sv2v -> iverilog: y=Xf
            // verilator --binary: y=0f
            "l_q3z_proc_block_caller",
            r#"module top;
  localparam W = 4;
  function [7:0] f(input [7:0] a);
    reg [W-1:0] t;
    reg [7:0] u;
    begin
      t = a;
      f[7:0] = t;
      f[7] = u[0];
    end
  endfunction
  logic [7:0] y;
  initial begin : b
    reg [7:0] u;
    u = 8'h01;
    y = f(8'hFF);
    $display("y=%h", y);
    $finish;
  end
endmodule
"#,
            "16:5",
            "E3009",
            "declares a formal or local whose range or type names a constant",
        ),
        (
            // iverilog: y=0f w=0f
            // sv2v -> iverilog: y=0f w=0f
            // verilator --binary: y=0f w=0f
            "l_q4b_local_typedef_param_gen",
            r#"module top;
  localparam W = 4;
  typedef logic [W-1:0] w_t;
  function [7:0] f(input [7:0] a);
    w_t t;
    begin
      t = a;
      f[7:0] = t;
    end
  endfunction
  for (genvar i = 0; i < 1; i++) begin : g
    localparam W = 8;
    typedef logic [W-1:0] w_t;
    wire [7:0] w = f(8'hFF);
  end
  wire [7:0] y = f(8'hFF);
  initial #1 begin $display("y=%h w=%h", y, g[0].w); $finish; end
endmodule
"#,
            "1:8",
            "E3009",
            "declares a formal or local whose range or type names a constant",
        ),
        (
            // iverilog: y=01100101 y=00110110
            // sv2v -> iverilog: y=01100101 y=00110101
            // verilator --binary: ?
            "l_q3f_local_init_static",
            r#"module top;
  function [7:0] f(input [3:0] a);
    reg [3:0] t = 4'h5;
    begin
      f[3:0] = t;
      t = a;
      f[7:4] = a;
    end
  endfunction
  logic [3:0] a = 4'h6;
  wire [7:0] y = f(a);
  initial begin #1 $display("y=%b", y); a = 4'h3; #1 $display("y=%b", y); $finish; end
endmodule
"#,
            "1:8",
            "E3009",
            "declares a local with an initializer",
        ),
    ]);
}

/// Every value a select-writing body assigns must be repeatable: the merge names the running
/// value once per kept slice and the substitution fold names a local once per reference.
#[test]
fn a_select_writing_body_with_an_effect_is_refused() {
    refused(&[
        (
            // iverilog: eq=1
            // sv2v -> iverilog: eq=1
            // verilator --binary: eq=1
            "c41_random_dup",
            r#"module top;
  function [7:0] f(input [3:0] a);
    reg [3:0] t;
    begin
      t = $random;
      f[3:0] = t;
      f[7:4] = t;
    end
  endfunction
  logic [3:0] a = 0;
  wire [7:0] y = f(a);
  initial #1 begin $display("eq=%b", y[7:4] == y[3:0]); $finish; end
endmodule
"#,
            "1:8",
            "E3009",
            "assigns a value that has an effect or reads an array element",
        ),
        (
            // iverilog: y0=0
            // sv2v -> iverilog: y0=0
            // verilator --binary: y0=0
            "c51_random_whole",
            r#"module top;
  function [7:0] f(input [3:0] a);
    f = $random;
    f[0] = 1'b0;
  endfunction
  logic [3:0] a = 4'h1;
  wire [7:0] y = f(a);
  initial #1 begin $display("y0=%b", y[0]); $finish; end
endmodule
"#,
            "1:8",
            "E3009",
            "assigns a value that has an effect or reads an array element",
        ),
    ]);
}

/// A reversed part-select (both 4-state oracles reject it), a nested select, and a return
/// range that names a constant: it folds where the call is, so `function [K:0] f` called
/// under a generate block declaring its own `K` would be sized by the block's (c53); an
/// interface function's named range is refused alike (c54). An owned package routine's
/// ranges fold in its package's window and may name its own constants (c55, q3x run).
#[test]
fn a_reversed_or_nested_select_or_a_named_return_range_is_refused() {
    refused(&[
        (
            // iverilog: t.sv:4: error: part select f[0:3] is reversed. 1 error(s) during elaboration.
            // sv2v -> iverilog: sv.v:6: error: part select f[0:3] is reversed. 1 error(s) during elaboration.
            // verilator --binary: y=08
            "c45_reversed_part",
            r#"module top;
  function [7:0] f(input [3:0] a);
    f = 8'h00;
    f[0:3] = a;
  endfunction
  logic [3:0] a = 4'h1;
  wire [7:0] y = f(a);
  initial #1 begin $display("y=%h", y); $finish; end
endmodule
"#,
            "1:8",
            "E3009",
            "with a part-select reversed against its declared range",
        ),
        (
            // iverilog: t.sv:4: error: the number of indices (2) is greater than the number of dimensions (1). 1 error(s) during elaboration.
            // sv2v -> iverilog: sv.v:6: error: the number of indices (2) is greater than the number of dimensions (1). 1 error(s) during elaboration.
            // verilator --binary: y=20
            "c50_nested_sel",
            r#"module top;
  function [7:0] f(input [3:0] a);
    f = 8'h00;
    f[7:4][1] = a[0];
  endfunction
  logic [3:0] a = 4'h1;
  wire [7:0] y = f(a);
  initial #1 begin $display("y=%h", y); $finish; end
endmodule
"#,
            "1:8",
            "E3009",
            "through a nested select",
        ),
        (
            // iverilog: y=00001011 z=00001011
            // sv2v -> iverilog: y=00001011 z=00001011
            // verilator --binary: y=00001011 z=00001011
            "c53_gen_ret_range",
            r#"module top;
  localparam K = 3;
  function [K:0] f(input a);
    f[0] = a;
    f[3:1] = 3'b101;
  endfunction
  function [K:0] h(input a);
    h = {3'b101, a};
  endfunction
  generate if (1) begin : g
    localparam K = 1;
    wire [7:0] y = f(1'b1);
    wire [7:0] z = h(1'b1);
  end endgenerate
  initial #1 begin $display("y=%b z=%b", g.y, g.z); $finish; end
endmodule
"#,
            "1:8",
            "E3009",
            "whose declared type has no literal, non-negative bit range",
        ),
        (
            // iverilog: d=0100
            // sv2v -> iverilog: d=0100
            // verilator --binary: d=0100
            "c54_iface_fn",
            r#"interface bus #(parameter W = 4);
  logic [1:0] s;
  function [W-1:0] dec(input [1:0] in);
    dec[0] = ~in[1] & ~in[0];
    dec[1] = ~in[1] &  in[0];
    dec[2] =  in[1] & ~in[0];
    dec[3] =  in[1] &  in[0];
  endfunction
  wire [W-1:0] d = dec(s);
endinterface
module top;
  localparam W = 2;
  bus #(.W(4)) b();
  initial begin b.s = 2'd2; #1 $display("d=%b", b.d); $finish; end
endmodule
"#,
            "11:8",
            "E3009",
            "whose declared type has no literal, non-negative bit range",
        ),
    ]);
}

/// Unchanged: a select write under control flow, or into a 2-state return, still takes the
/// frame lane (c14, c17, c29, c37); a local's select write and a struct-member write of the
/// return variable stay refused; and the constant-function
/// interpreter still has no arm for a select write (a parameter value, c22).
#[test]
fn neighbouring_lanes_do_not_move() {
    prints(&[
        (
            // iverilog: y=10100011
            // sv2v -> iverilog: y=10100011
            // verilator --binary: y=10100011
            "c14_for_loop",
            r#"module top;
  function [7:0] rev;
    input [7:0] a;
    integer k;
    begin
      for (k = 0; k < 8; k = k + 1) rev[k] = a[7-k];
    end
  endfunction
  logic [7:0] a = 8'b1100_0101;
  wire [7:0] y = rev(a);
  initial #1 begin $display("y=%b", y); $finish; end
endmodule
"#,
            "y=10100011",
        ),
        (
            // iverilog: s0 y=0001 s2 y=1000 s3 y=1000
            // sv2v -> iverilog: s0 y=0001 s2 y=1000 s3 y=1000
            // verilator --binary: s0 y=0001 s2 y=1000 s3 y=1000
            "c17_case_sel",
            r#"module top;
  function [3:0] f;
    input [1:0] s;
    begin
      f = 4'b0000;
      case (s)
        2'd0: f[0] = 1'b1;
        2'd1: f[1] = 1'b1;
        2'd2: f[3:2] = 2'b10;
        default: f[3] = 1'b1;
      endcase
    end
  endfunction
  logic [1:0] s;
  wire [3:0] y = f(s);
  initial begin
    s = 0; #1 $display("s0 y=%b", y);
    s = 2; #1 $display("s2 y=%b", y);
    s = 3; #1 $display("s3 y=%b", y);
    $finish;
  end
endmodule
"#,
            "s0 y=0001\ns2 y=1000\ns3 y=1000",
        ),
        (
            // iverilog: y=1001
            // sv2v -> iverilog: y=1xx1
            // verilator --binary: y=1001
            "c29_ret_twostate",
            r#"module top;
  function bit [3:0] f;
    input [1:0] in;
    begin
      f[3:2] = in;
      f[1:0] = 2'bx1;
    end
  endfunction
  logic [1:0] s = 2'b1x;
  wire [3:0] y = f(s);
  initial #1 begin $display("y=%b", y); $finish; end
endmodule
"#,
            "y=1001",
        ),
        (
            // iverilog: y=x5 z=x5
            // sv2v -> iverilog: y=x5 z=x5
            // verilator --binary: y=05 z=05
            "c37_frame_partial",
            r#"module top;
  function [7:0] f;
    input [3:0] a;
    input c;
    if (c) f[3:0] = a;
  endfunction
  function automatic [7:0] h(input [3:0] a);
    h[3:0] = a;
  endfunction
  logic [3:0] a = 4'h5;
  wire [7:0] y = f(a, 1'b1);
  wire [7:0] z = h(a);
  initial #1 begin $display("y=%h z=%h", y, z); $finish; end
endmodule
"#,
            "y=x5 z=x5",
        ),
    ]);
    refused(&[
        (
            // iverilog: y=0100
            // sv2v -> iverilog: y=0100
            // verilator --binary: y=0100
            "c25_local_sel",
            r#"module top;
  function [3:0] f;
    input [1:0] in;
    reg [3:0] t;
    begin
      t = 4'b0000;
      t[in] = 1'b1;
      f = t;
    end
  endfunction
  logic [1:0] s = 2'd2;
  wire [3:0] y = f(s);
  initial #1 begin $display("y=%b", y); $finish; end
endmodule
"#,
            "1:8",
            "E3009",
            "body is not reducible to an expression (control flow)",
        ),
        (
            // iverilog: P=0100 W=5
            // sv2v -> iverilog: P=0100 W=5
            // verilator --binary: P=0100 W=5
            "c22_param_const",
            r#"module top;
  function [3:0] decode2_4;
    input [1:0] in;
    begin
      decode2_4[3] =  in[1] &  in[0];
      decode2_4[2] =  in[1] & ~in[0];
      decode2_4[1] = ~in[1] &  in[0];
      decode2_4[0] = ~in[1] & ~in[0];
    end
  endfunction
  localparam [3:0] P = decode2_4(2'd2);
  wire [P:0] w;
  initial #1 begin $display("P=%b W=%0d", P, $bits(w)); $finish; end
endmodule
"#,
            "11:24",
            "E3009",
            "has no constant-fold arm",
        ),
        (
            // iverilog: y=3c
            // sv2v -> iverilog: y=3c
            // verilator --binary: y=3c
            "c26_struct_ret",
            r#"module top;
  typedef struct packed { logic [3:0] hi; logic [3:0] lo; } pr_t;
  function pr_t f;
    input [3:0] a;
    begin
      f.hi = a;
      f.lo = ~a;
    end
  endfunction
  logic [3:0] a = 4'h3;
  pr_t y;
  assign y = f(a);
  initial #1 begin $display("y=%h", y); $finish; end
endmodule
"#,
            "1:8",
            "E3009",
            "body is not reducible to an expression (control flow)",
        ),
    ]);
}
