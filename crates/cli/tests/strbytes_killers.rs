//! Q-STRBYTES pins, GENERATED from the round-3 battery killer cells (W/qstrbytes/r3/cellsrc k_s23_frame_expr: S23 frame Display ; k_s28_sfmt_generic: S28 generic $sformatf arm) by W/qstrbytes/r3/bin/gen_pins.py.
//!
//! RTL text is bytes (IEEE 1800-2017 §21.2.1.8: a string is 8 bits per character, a field
//! width counts them). Each `_moves` test holds the lines this row moves (wrong -> right),
//! each `_stays` test the lines it must not move; every tagged line carries its source:
//! the oracle set whose output it equals (iv = iverilog 13, s2v = sv2v -> iverilog, vl =
//! verilator 5.052; their text quoted raw-escaped beside it), HAND (an IEEE value where no
//! oracle prints the line), JUDGED (a ruling against an oracle self-contradiction) or PRE
//! (a named stay). The comparison is raw bytes on every lane (`strbytes_util`).

mod strbytes_util;
use strbytes_util::{check, Cell, Mode};

const K_S23_FRAME_EXPR_SRC: &str = r#"module top;
  // k_s23: a $display / $write inside a function called in an EXPRESSION runs on the frame executor (frame_eval.rs run_frame_call
  // Display arm, S23) ; a void function or a task called as a statement runs on the statement executor instead (c27 H1 H2)
  string g; int r;
  function automatic int fe(input string a); $display("@F1 %s|", a); return 1; endfunction
  function automatic int fw(input string a); $write("@F2 %s|\n", a); return 2; endfunction
  initial begin
    g = string'(16'hC841);
    r = fe(g);
    r = r + fw(g);
    $display("@F3 %0d", r);
    #1 $finish;
  end
endmodule
"#;

#[test]
fn k_s23_frame_expr_moves() {
    check(
        &Cell {
            src: K_S23_FRAME_EXPR_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::At,
            staged: true,
        },
        &[
            // F1: MOVE = iv+s2v+vl ; PRE @F1 \xef\xbf\xbdA|
            //   iv: @F1 \xc8A|
            //   s2v: @F1 \xc8A|
            //   vl: @F1 \xc8A|
            ("F1", b"@F1 \xc8A|"),
            // F2: MOVE = iv+s2v+vl ; PRE @F2 \xef\xbf\xbdA|
            //   iv: @F2 \xc8A|
            //   s2v: @F2 \xc8A|
            //   vl: @F2 \xc8A|
            ("F2", b"@F2 \xc8A|"),
        ],
        &[],
    );
}

#[test]
fn k_s23_frame_expr_stays() {
    check(
        &Cell {
            src: K_S23_FRAME_EXPR_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::At,
            staged: true,
        },
        &[
            // F3: STAY = iv+s2v+vl ; PRE @F3 3
            //   iv: @F3 3
            //   s2v: @F3 3
            //   vl: @F3 3
            ("F3", b"@F3 3"),
        ],
        &[],
    );
}

const K_S28_SFMT_GENERIC_SRC: &str = r#"module top;
  // k_s28: a $sformatf passed to a function inside a string concat in a FRAME function body / return lowers as a plain node
  // (sformatf_expr_ok) and is rendered by the generic eval arm (eval/sysfunc.rs SysFuncId::Sformatf, S28): a packed argument (Q3 Q5)
  // and a string-literal argument (Q4 Q6).  Q1 Q2 = the same rhs in a module process (does not reach S28)
  function automatic string id(input string a); return a; endfunction
  task automatic hx(input string tag, input string a); $write("@%s", tag); for (int i = 0; i < a.len(); i++) $write(" %02x", a[i]); $display(" len=%0d", a.len()); endtask
  function automatic string fp(input string a, input [7:0] c); string t; t = {a, id($sformatf("%s%s", a, c))}; return t; endfunction
  function automatic string fl(input string a); string t; t = {a, id($sformatf("%s%s", a, "\310"))}; return t; endfunction
  function automatic string rp(input string a, input [7:0] c); return {a, id($sformatf("%s%s", a, c))}; endfunction
  function automatic string rl(input string a); return {a, id($sformatf("%s%s", a, "\310"))}; endfunction
  string s, r; reg [7:0] b;
  initial begin
    s = "A"; b = 8'hC8;
    r = {s, id($sformatf("%s%s", s, b))}; hx("Q1", r);
    r = {s, id($sformatf("%s%s", s, "\310"))}; hx("Q2", r);
    r = fp(s, b); hx("Q3", r);
    r = fl(s); hx("Q4", r);
    r = rp(s, b); hx("Q5", r);
    r = rl(s); hx("Q6", r);
    #1 $finish;
  end
endmodule
"#;

#[test]
fn k_s28_sfmt_generic_moves() {
    check(
        &Cell {
            src: K_S28_SFMT_GENERIC_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::At,
            staged: true,
        },
        &[
            // Q1: MOVE = iv+s2v+vl ; PRE @Q1 41 41 c3 88 len=4
            //   iv: @Q1 41 41 c8 len=3
            //   s2v: @Q1 41 41 c8 len=3
            //   vl: @Q1 41 41 c8 len=3
            ("Q1", b"@Q1 41 41 c8 len=3"),
            // Q2: MOVE = iv+s2v+vl ; PRE @Q2 41 41 ef bf bd len=5
            //   iv: @Q2 41 41 c8 len=3
            //   s2v: @Q2 41 41 c8 len=3
            //   vl: @Q2 41 41 c8 len=3
            ("Q2", b"@Q2 41 41 c8 len=3"),
            // Q3: MOVE = iv+s2v+vl ; PRE @Q3 41 41 c3 88 len=4
            //   iv: @Q3 41 41 c8 len=3
            //   s2v: @Q3 41 41 c8 len=3
            //   vl: @Q3 41 41 c8 len=3
            ("Q3", b"@Q3 41 41 c8 len=3"),
            // Q4: MOVE = iv+s2v+vl ; PRE @Q4 41 41 ef bf bd len=5
            //   iv: @Q4 41 41 c8 len=3
            //   s2v: @Q4 41 41 c8 len=3
            //   vl: @Q4 41 41 c8 len=3
            ("Q4", b"@Q4 41 41 c8 len=3"),
            // Q5: MOVE = iv+s2v+vl ; PRE @Q5 41 41 c3 88 len=4
            //   iv: @Q5 41 41 c8 len=3
            //   s2v: @Q5 41 41 c8 len=3
            //   vl: @Q5 41 41 c8 len=3
            ("Q5", b"@Q5 41 41 c8 len=3"),
            // Q6: MOVE = iv+s2v+vl ; PRE @Q6 41 41 ef bf bd len=5
            //   iv: @Q6 41 41 c8 len=3
            //   s2v: @Q6 41 41 c8 len=3
            //   vl: @Q6 41 41 c8 len=3
            ("Q6", b"@Q6 41 41 c8 len=3"),
        ],
        &[],
    );
}
