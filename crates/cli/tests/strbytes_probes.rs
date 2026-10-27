//! Q-STRBYTES pins, GENERATED from the Q-STRBYTES plan probes (W/qstrbytes/planprobes), the 26a cells x26a / q6 / pS, the OP-c $fscanf boundary pair and C1's split plusarg by W/qstrbytes/r1/bin/gen_pins.py.
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

const P_P01_WIDTH_UTF8_SRC: &str = r#"module top;
  task automatic hx(input string tag, input string a); $write("@%s", tag); for (int i = 0; i < a.len(); i++) $write(" %02x", a[i]); $display(" len=%0d", a.len()); endtask
  string e, u, m;
  initial begin
    e = "é"; m = string'(24'hC3A9C8);
    $display("@W1 [%5s]", "é");
    $display("@W2 [%-5s]", "é");
    $display("@W3 [%5s]", e);
    $display("@W4 [%-5s]", e);
    u = $sformatf("%5s", e); hx("W5", u);
    $display("@W6 [%5s]", m);
    $display("@W7 [%2s]", e);
    #1 $finish;
  end
endmodule
"#;

#[test]
fn p_p01_width_utf8_moves() {
    check(
        &Cell {
            src: P_P01_WIDTH_UTF8_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::At,
            staged: true,
        },
        &[
            // W1: MOVE = iv+s2v+vl ; PRE @W1 [    \xc3\xa9]
            //   iv: @W1 [   \xc3\xa9]
            //   s2v: @W1 [   \xc3\xa9]
            //   vl: @W1 [   \xc3\xa9]
            ("W1", b"@W1 [   \xc3\xa9]"),
            // W2: MOVE = iv+s2v+vl ; PRE @W2 [\xc3\xa9    ]
            //   iv: @W2 [\xc3\xa9   ]
            //   s2v: @W2 [\xc3\xa9   ]
            //   vl: @W2 [\xc3\xa9   ]
            ("W2", b"@W2 [\xc3\xa9   ]"),
            // W3: MOVE = iv+s2v+vl ; PRE @W3 [    \xc3\xa9]
            //   iv: @W3 [   \xc3\xa9]
            //   s2v: @W3 [   \xc3\xa9]
            //   vl: @W3 [   \xc3\xa9]
            ("W3", b"@W3 [   \xc3\xa9]"),
            // W4: MOVE = iv+s2v+vl ; PRE @W4 [\xc3\xa9    ]
            //   iv: @W4 [\xc3\xa9   ]
            //   s2v: @W4 [\xc3\xa9   ]
            //   vl: @W4 [\xc3\xa9   ]
            ("W4", b"@W4 [\xc3\xa9   ]"),
            // W5: MOVE = iv+s2v+vl ; PRE @W5 20 20 20 20 c3 a9 len=6
            //   iv: @W5 20 20 20 c3 a9 len=5
            //   s2v: @W5 20 20 20 c3 a9 len=5
            //   vl: @W5 20 20 20 c3 a9 len=5
            ("W5", b"@W5 20 20 20 c3 a9 len=5"),
            // W6: MOVE = iv+s2v+vl ; PRE @W6 [   \xc3\xa9\xef\xbf\xbd]
            //   iv: @W6 [  \xc3\xa9\xc8]
            //   s2v: @W6 [  \xc3\xa9\xc8]
            //   vl: @W6 [  \xc3\xa9\xc8]
            ("W6", b"@W6 [  \xc3\xa9\xc8]"),
            // W7: MOVE = iv+s2v+vl ; PRE @W7 [ \xc3\xa9]
            //   iv: @W7 [\xc3\xa9]
            //   s2v: @W7 [\xc3\xa9]
            //   vl: @W7 [\xc3\xa9]
            ("W7", b"@W7 [\xc3\xa9]"),
        ],
        &[],
    );
}

const P_P02_SPLIT_UTF8_SRC: &str = r#"module top;
  task automatic hx(input string tag, input string a); $write("@%s", tag); for (int i = 0; i < a.len(); i++) $write(" %02x", a[i]); $display(" len=%0d", a.len()); endtask
  string a, b, r;
  initial begin
    a = string'(8'hC3); b = string'(8'hA9);
    $write("@S1 "); $write("%c", 8'hC3); $write("%c", 8'hA9); $display("|");
    $display("@S2 %s%s|", a, b);
    r = {a, b}; hx("S3", r);
    $write("@S4 "); $write("%s", a); $write("%s", b); $display("|");
    $display("@S5 %c%c|", 8'hC3, 8'hA9);
    #1 $finish;
  end
endmodule
"#;

#[test]
fn p_p02_split_utf8_moves() {
    check(
        &Cell {
            src: P_P02_SPLIT_UTF8_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::At,
            staged: true,
        },
        &[
            // S1: MOVE = iv+s2v+vl ; PRE @S1 \xc3\x83\xc2\xa9|
            //   iv: @S1 \xc3\xa9|
            //   s2v: @S1 \xc3\xa9|
            //   vl: @S1 \xc3\xa9|
            ("S1", b"@S1 \xc3\xa9|"),
            // S2: MOVE = iv+s2v+vl ; PRE @S2 \xef\xbf\xbd\xef\xbf\xbd|
            //   iv: @S2 \xc3\xa9|
            //   s2v: @S2 \xc3\xa9|
            //   vl: @S2 \xc3\xa9|
            ("S2", b"@S2 \xc3\xa9|"),
            // S3: MOVE = iv+s2v+vl ; PRE @S3 ef bf bd ef bf bd len=6
            //   iv: @S3 c3 a9 len=2
            //   s2v: @S3 c3 a9 len=2
            //   vl: @S3 c3 a9 len=2
            ("S3", b"@S3 c3 a9 len=2"),
            // S4: MOVE = iv+s2v+vl ; PRE @S4 \xef\xbf\xbd\xef\xbf\xbd|
            //   iv: @S4 \xc3\xa9|
            //   s2v: @S4 \xc3\xa9|
            //   vl: @S4 \xc3\xa9|
            ("S4", b"@S4 \xc3\xa9|"),
            // S5: MOVE = iv+s2v+vl ; PRE @S5 \xc3\x83\xc2\xa9|
            //   iv: @S5 \xc3\xa9|
            //   s2v: @S5 \xc3\xa9|
            //   vl: @S5 \xc3\xa9|
            ("S5", b"@S5 \xc3\xa9|"),
        ],
        &[],
    );
}

const P_P03_SEV_PATHS_SRC: &str = r#"module top;
  task automatic hx(input string tag, input string a); $write("@%s", tag); for (int i = 0; i < a.len(); i++) $write(" %02x", a[i]); $display(" len=%0d", a.len()); endtask
  string s;
  function automatic int f(input int x); $error("@F1 %s|", s); return x + 1; endfunction
  task automatic tk(); $warning("@F2 %s|", s); endtask
  int k;
  initial begin
    s = string'(16'hC841);
    $info("@E3 %c%s|", 8'hC8, 16'hC841);
    k = f(1);
    tk();
    $display("@E9 end %0d", k);
    #1 $finish;
  end
endmodule
"#;

#[test]
fn p_p03_sev_paths_moves() {
    check(
        &Cell {
            src: P_P03_SEV_PATHS_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::At,
            staged: true,
        },
        &[
            // E3: MOVE = iv+s2v+vl ; PRE @E3 \xc3\x88\xc3\x88A|
            //   iv: @E3 \xc8\xc8A|
            //   s2v: @E3 \xc8\xc8A|
            //   vl: @E3 \xc8\xc8A|
            ("E3", b"@E3 \xc8\xc8A|"),
            // F1: MOVE = iv+s2v+vl ; PRE @F1 \xef\xbf\xbdA|
            //   iv: @F1 \xc8A|
            //   s2v: @F1 \xc8A|
            //   vl: @F1 \xc8A|
            ("F1", b"@F1 \xc8A|"),
            // F2: MOVE = iv+s2v ; PRE @F2 \xef\xbf\xbdA|
            //   iv: @F2 \xc8A|
            //   s2v: @F2 \xc8A|
            ("F2", b"@F2 \xc8A|"),
        ],
        &[],
    );
}

#[test]
fn p_p03_sev_paths_stays() {
    check(
        &Cell {
            src: P_P03_SEV_PATHS_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::At,
            staged: true,
        },
        &[
            // E9: STAY = iv+s2v ; PRE @E9 end 2
            //   iv: @E9 end 2
            //   s2v: @E9 end 2
            ("E9", b"@E9 end 2"),
        ],
        &[],
    );
}

const P_P04_FILES_MCD_SRC: &str = r#"module top;
  integer mcd; string s;
  initial begin
    s = string'(16'hC841);
    mcd = $fopen("fo.bin");
    $fdisplay(mcd | 1, "@M1 %s|", s);
    $fstrobe(32'h8000_0001, "@M2 %s|", s);
    $fmonitor(32'h8000_0001, "@M3 %s|", s);
    $fdisplay(32'h8000_0001, "@M4 %c|", 8'hFF);
    #1 $fclose(mcd);
    $finish;
  end
endmodule
"#;

#[test]
fn p_p04_files_mcd_moves() {
    check(
        &Cell {
            src: P_P04_FILES_MCD_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::At,
            staged: true,
        },
        &[
            // M1: MOVE = vl ; PRE @M1 \xef\xbf\xbdA|
            //   vl: @M1 \xc8A|
            ("M1", b"@M1 \xc8A|"),
            // M4: MOVE = vl ; PRE @M4 \xc3\xbf|
            //   vl: @M4 \xff|
            ("M4", b"@M4 \xff|"),
            // M3: MOVE = vl ; PRE @M3 \xef\xbf\xbdA|
            //   vl: @M3 \xc8A|
            ("M3", b"@M3 \xc8A|"),
            // M2: MOVE = vl ; PRE @M2 \xef\xbf\xbdA|
            //   vl: @M2 \xc8A|
            ("M2", b"@M2 \xc8A|"),
        ],
        &[
            // FILE:fo.bin: MOVE = vl ; PRE @M1 \xef\xbf\xbdA|\x0a
            ("fo.bin", b"@M1 \xc8A|\x0a"),
        ],
    );
}

const P_P05_SFORMAT_DEST_SRC: &str = r#"module top;
  task automatic hx(input string tag, input string a); $write("@%s", tag); for (int i = 0; i < a.len(); i++) $write(" %02x", a[i]); $display(" len=%0d", a.len()); endtask
  reg [15:0] r; reg [23:0] q; string s, t;
  initial begin
    s = string'(16'hC841);
    $sformat(r, "%s", s); $display("@Y1 %h", r);
    $swrite(t, "%s", s); hx("Y2", t);
    $sformat(t, "<%s>", s); hx("Y3", t);
    $sformat(q, "%c%s", 8'hFF, s); $display("@Y4 %h", q);
    #1 $finish;
  end
endmodule
"#;

#[test]
fn p_p05_sformat_dest_moves() {
    check(
        &Cell {
            src: P_P05_SFORMAT_DEST_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::At,
            staged: true,
        },
        &[
            // Y1: MOVE = iv+s2v+vl ; PRE @Y1 bd41
            //   iv: @Y1 c841
            //   s2v: @Y1 c841
            //   vl: @Y1 c841
            ("Y1", b"@Y1 c841"),
            // Y2: MOVE = iv+s2v+vl ; PRE @Y2 ef bf bd 41 len=4
            //   iv: @Y2 c8 41 len=2
            //   s2v: @Y2 c8 41 len=2
            //   vl: @Y2 c8 41 len=2
            ("Y2", b"@Y2 c8 41 len=2"),
            // Y3: MOVE = iv+s2v+vl ; PRE @Y3 3c ef bf bd 41 3e len=6
            //   iv: @Y3 3c c8 41 3e len=4
            //   s2v: @Y3 3c c8 41 3e len=4
            //   vl: @Y3 3c c8 41 3e len=4
            ("Y3", b"@Y3 3c c8 41 3e len=4"),
            // Y4: MOVE = iv+s2v+vl ; PRE @Y4 bfbd41
            //   iv: @Y4 ffc841
            //   s2v: @Y4 ffc841
            //   vl: @Y4 ffc841
            ("Y4", b"@Y4 ffc841"),
        ],
        &[],
    );
}

const P_P06_ATOREAL_NBSP_SRC: &str = r#"module top;
  string s, t; real r;
  initial begin
    s = string'(40'hC2A0312E35); t = string'(32'h20312E35);
    r = s.atoreal(); $display("@R1 %f", r);
    r = t.atoreal(); $display("@R2 %f", r);
    #1 $finish;
  end
endmodule
"#;

#[test]
fn p_p06_atoreal_nbsp_stays() {
    check(
        &Cell {
            src: P_P06_ATOREAL_NBSP_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::At,
            staged: true,
        },
        &[
            // R1: STAY = PRE (NOT THIS ROW: Q-ATOREAL-WS) ; PRE @R1 1.500000
            //   iv: @R1 0.000000
            //   s2v: @R1 0.000000
            //   vl: @R1 0.000000
            ("R1", b"@R1 1.500000"),
            // R2: STAY = iv+s2v+vl ; PRE @R2 1.500000
            //   iv: @R2 1.500000
            //   s2v: @R2 1.500000
            //   vl: @R2 1.500000
            ("R2", b"@R2 1.500000"),
        ],
        &[],
    );
}

const P_P07_ELAB_INFO_SRC: &str = r#"module top;
  parameter string P = "\310A";
  parameter [15:0] Q = 16'hC841;
  $info("@I1 %s|", P);
  $info("@I3 %s|", Q);
  $info("@I4 %c|", 8'hC8);
  initial #1 $finish;
endmodule
"#;

#[test]
fn p_p07_elab_info_moves() {
    check(
        &Cell {
            src: P_P07_ELAB_INFO_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::At,
            staged: true,
        },
        &[
            // I1: MOVE = s2v+vl ; PRE @I1 \xef\xbf\xbdA|
            //   s2v: @I1 \xc8A|
            //   vl: @I1 \xc8A|
            ("I1", b"@I1 \xc8A|"),
            // I3: MOVE = s2v+vl ; PRE @I3 \xc3\x88A|
            //   s2v: @I3 \xc8A|
            //   vl: @I3 \xc8A|
            ("I3", b"@I3 \xc8A|"),
            // I4: MOVE = s2v+vl ; PRE @I4 \xc3\x88|
            //   s2v: @I4 \xc8|
            //   vl: @I4 \xc8|
            ("I4", b"@I4 \xc8|"),
        ],
        &[],
    );
}

const P_P08_CONCAT_PACKED_SRC: &str = r#"module top;
  task automatic hx(input string tag, input string a); $write("@%s", tag); for (int i = 0; i < a.len(); i++) $write(" %02x", a[i]); $display(" len=%0d", a.len()); endtask
  string s, r; reg [7:0] b;
  initial begin
    s = "A"; b = 8'hC8;
    r = {s, b}; hx("K1", r);
    $display("@K2 %s|", {s, b});
    r = {s, "\310"}; hx("K3", r);
    #1 $finish;
  end
endmodule
"#;

#[test]
fn p_p08_concat_packed_moves() {
    check(
        &Cell {
            src: P_P08_CONCAT_PACKED_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::At,
            staged: true,
        },
        &[
            // K1: MOVE = vl ; PRE @K1 41 ef bf bd len=4
            //   vl: @K1 41 c8 len=2
            ("K1", b"@K1 41 c8 len=2"),
            // K2: MOVE = vl ; PRE @K2 A\xef\xbf\xbd|
            //   vl: @K2 A\xc8|
            ("K2", b"@K2 A\xc8|"),
            // K3: MOVE = vl ; PRE @K3 41 ef bf bd len=4
            //   vl: @K3 41 c8 len=2
            ("K3", b"@K3 41 c8 len=2"),
        ],
        &[],
    );
}

const P_P09_DEFERRED_PRINT_SRC: &str = r#"module top;
  string s; logic c;
  initial begin
    s = string'(16'hC841); c = 0;
    assert #0 (c) else $display("@DP1 %s|", s);
    assert #0 (c) else $error("@DP2 %s|", s);
    #1 $finish;
  end
endmodule
"#;

#[test]
fn p_p09_deferred_print_moves() {
    check(
        &Cell {
            src: P_P09_DEFERRED_PRINT_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::At,
            staged: true,
        },
        &[
            // DP1: MOVE = vl ; PRE @DP1 \xef\xbf\xbdA|
            //   vl: @DP1 \xc8A|
            ("DP1", b"@DP1 \xc8A|"),
            // DP2: MOVE = vl ; PRE @DP2 \xef\xbf\xbdA|
            //   vl: @DP2 \xc8A|
            ("DP2", b"@DP2 \xc8A|"),
        ],
        &[],
    );
}

const P_P10_FSCANF_FMT_SRC: &str = r#"module top;
  task automatic hx(input string tag, input string a); $write("@%s", tag); for (int i = 0; i < a.len(); i++) $write(" %02x", a[i]); $display(" len=%0d", a.len()); endtask
  integer fd, n; string x;
  initial begin
    fd = $fopen("fo.bin", "w"); $fwrite(fd, "%s", string'(24'hC84142)); $fclose(fd);
    fd = $fopen("fo.bin", "r"); x = "";
    n = $fscanf(fd, "\310%s", x); $display("@X3 %0d", n); hx("X4", x);
    $fclose(fd);
    #1 $finish;
  end
endmodule
"#;

#[test]
fn p_p10_fscanf_fmt_moves() {
    check(
        &Cell {
            src: P_P10_FSCANF_FMT_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::At,
            staged: true,
        },
        &[],
        &[
            // FILE:fo.bin: MOVE = iv+s2v+vl ; PRE \xef\xbf\xbdAB
            ("fo.bin", b"\xc8AB"),
        ],
    );
}

#[test]
fn p_p10_fscanf_fmt_stays() {
    check(
        &Cell {
            src: P_P10_FSCANF_FMT_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::At,
            staged: true,
        },
        &[
            // X3: STAY = PRE (judged STAY (PLAN 5 ; D7)) ; PRE @X3 1
            //   iv: @X3 0
            //   s2v: @X3 0
            //   vl: @X3 0
            ("X3", b"@X3 1"),
            // X4: STAY = PRE (judged STAY) ; PRE @X4 41 42 len=2
            //   iv: @X4 len=0
            //   s2v: @X4 len=0
            //   vl: @X4 len=0
            ("X4", b"@X4 41 42 len=2"),
        ],
        &[],
    );
}

const P_P11_SCANF_PAIR_SRC: &str = r#"module top;
  integer fd, n; string x, l;
  initial begin
    l = string'(24'hC84142);
    x = ""; n = $sscanf(l, "\310%s", x); $display("@Q1 %0d %0d", n, x.len());
    fd = $fopen("in.bin", "r"); x = ""; n = $fscanf(fd, "\310%s", x); $display("@Q2 %0d %0d", n, x.len()); $fclose(fd);
    fd = $fopen("mem.hex", "r"); x = ""; n = $fscanf(fd, "X%s", x); $display("@Q3 %0d %0d", n, x.len()); $fclose(fd);
    fd = $fopen("in.bin", "r"); x = ""; n = $fscanf(fd, "%s", x); $display("@Q4 %0d %0d %h", n, x.len(), x[0]); $fclose(fd);
    x = ""; n = $sscanf(l, "%s", x); $display("@Q5 %0d %0d %h", n, x.len(), x[0]);
    #1 $finish;
  end
endmodule
"#;

#[test]
fn p_p11_scanf_pair_moves() {
    check(
        &Cell {
            src: P_P11_SCANF_PAIR_SRC,
            inputs: &[("in.bin", b"\xc8AB\x0a"), ("mem.hex", b"XAB\x0a")],
            plus: None,
            mode: Mode::At,
            staged: true,
        },
        &[
            // Q1: MOVE = iv+s2v+vl ; PRE @Q1 0 0
            //   iv: @Q1 1 2
            //   s2v: @Q1 1 2
            //   vl: @Q1 1 2
            ("Q1", b"@Q1 1 2"),
            // Q2: MOVE = JUDGED (D7 adopted (PLAN 454): $fscanf "\310%s" on c8 41 42 matches (§21.3.4.3 b) = the oracles' own $sscanf twin Q1) ; PRE @Q2 0 0
            //   iv: @Q2 0 0
            //   s2v: @Q2 0 0
            //   vl: @Q2 0 0
            ("Q2", b"@Q2 1 2"),
        ],
        &[],
    );
}

#[test]
fn p_p11_scanf_pair_stays() {
    check(
        &Cell {
            src: P_P11_SCANF_PAIR_SRC,
            inputs: &[("in.bin", b"\xc8AB\x0a"), ("mem.hex", b"XAB\x0a")],
            plus: None,
            mode: Mode::At,
            staged: true,
        },
        &[
            // Q3: STAY = iv+s2v+vl ; PRE @Q3 1 2
            //   iv: @Q3 1 2
            //   s2v: @Q3 1 2
            //   vl: @Q3 1 2
            ("Q3", b"@Q3 1 2"),
            // Q4: STAY = iv+s2v+vl ; PRE @Q4 1 3 c8
            //   iv: @Q4 1 3 c8
            //   s2v: @Q4 1 3 c8
            //   vl: @Q4 1 3 c8
            ("Q4", b"@Q4 1 3 c8"),
            // Q5: STAY = iv+s2v+vl ; PRE @Q5 1 3 c8
            //   iv: @Q5 1 3 c8
            //   s2v: @Q5 1 3 c8
            //   vl: @Q5 1 3 c8
            ("Q5", b"@Q5 1 3 c8"),
        ],
        &[],
    );
}

const X_X26A_HIBYTE_FMT_SRC: &str = r#"module top;
  // OP1 / N1 (d26): a licensed NON-E format over string bytes >= 0x80 moves to its direct-form twin's value (Q-STRBYTES).
  string s = "S", t = "\310\377a", u;
  function automatic string fs(input logic c); return c ? $sformatf("<%s>", t) : s; endfunction
  initial begin
    u = $sformatf("<%s>", t); $display("W1 %0d %h %h", u.len(), u.getc(0), u.getc(1));
    u = fs(1); $display("W1d %0d %h %h", u.len(), u.getc(0), u.getc(1));
    #1 $finish;
  end
endmodule
"#;

#[test]
fn x_x26a_hibyte_fmt_moves() {
    check(
        &Cell {
            src: X_X26A_HIBYTE_FMT_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::Word,
            staged: true,
        },
        &[
            // W1: MOVE = vl ; PRE W1 9 3c ef
            //   vl: W1 5 3c c8
            ("W1", b"W1 5 3c c8"),
        ],
        &[],
    );
}

#[test]
fn x_x26a_hibyte_fmt_stays() {
    check(
        &Cell {
            src: X_X26A_HIBYTE_FMT_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::Word,
            staged: true,
        },
        &[
            // W1d: STAY = PRE (PLAN2: W1d dropped (26a twin-equal move, not on main) -> stays) ; PRE W1d 3 c8 ff
            //   vl: W1d 5 3c c8
            ("W1d", b"W1d 3 c8 ff"),
        ],
        &[],
    );
}

const X_Q6_UTF8_SRC: &str = r#"module top;
  string s, r, t; logic [31:0] v = 32'hC0937F81;
  task automatic hx(input string tag, input string a); $write("%s", tag); for (int i = 0; i < a.len(); i++) $write(" %02x", a[i]); $display(" len=%0d", a.len()); endtask
  initial begin
    s = string'(v); hx("S0", s);
    r = $sformatf("%s", s); hx("K1", r);
    r = $sformatf("%s", v); hx("K2", r);
    r = $sformatf("%c", 8'hC0); hx("K3", r);
    r = {s, "!"}; hx("C1", r);
    t = s; t = {t, ""}; hx("C2", t);
    $display("D1 %s", s);
    #1 $finish;
  end
endmodule
"#;

#[test]
fn x_q6_utf8_moves() {
    check(
        &Cell {
            src: X_Q6_UTF8_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::Word,
            staged: true,
        },
        &[
            // K1: MOVE = iv+s2v+vl ; PRE K1 ef bf bd ef bf bd 7f ef bf bd len=10
            //   iv: K1 c0 93 7f 81 len=4
            //   s2v: K1 c0 93 7f 81 len=4
            //   vl: K1 c0 93 7f 81 len=4
            ("K1", b"K1 c0 93 7f 81 len=4"),
            // K2: MOVE = iv+s2v+vl ; PRE K2 c3 80 c2 93 7f c2 81 len=7
            //   iv: K2 c0 93 7f 81 len=4
            //   s2v: K2 c0 93 7f 81 len=4
            //   vl: K2 c0 93 7f 81 len=4
            ("K2", b"K2 c0 93 7f 81 len=4"),
            // K3: MOVE = iv+s2v+vl ; PRE K3 c3 80 len=2
            //   iv: K3 c0 len=1
            //   s2v: K3 c0 len=1
            //   vl: K3 c0 len=1
            ("K3", b"K3 c0 len=1"),
            // C1: MOVE = iv+s2v+vl ; PRE C1 ef bf bd ef bf bd 7f ef bf bd 21 len=11
            //   iv: C1 c0 93 7f 81 21 len=5
            //   s2v: C1 c0 93 7f 81 21 len=5
            //   vl: C1 c0 93 7f 81 21 len=5
            ("C1", b"C1 c0 93 7f 81 21 len=5"),
            // C2: MOVE = iv+s2v+vl ; PRE C2 ef bf bd ef bf bd 7f ef bf bd len=10
            //   iv: C2 c0 93 7f 81 len=4
            //   s2v: C2 c0 93 7f 81 len=4
            //   vl: C2 c0 93 7f 81 len=4
            ("C2", b"C2 c0 93 7f 81 len=4"),
            // D1: MOVE = iv+s2v+vl ; PRE D1 \xef\xbf\xbd\xef\xbf\xbd\x7f\xef\xbf\xbd
            //   iv: D1 \xc0\x93\x7f\x81
            //   s2v: D1 \xc0\x93\x7f\x81
            //   vl: D1 \xc0\x93\x7f\x81
            ("D1", b"D1 \xc0\x93\x7f\x81"),
        ],
        &[],
    );
}

#[test]
fn x_q6_utf8_stays() {
    check(
        &Cell {
            src: X_Q6_UTF8_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::Word,
            staged: true,
        },
        &[
            // S0: STAY = iv+s2v+vl ; PRE S0 c0 93 7f 81 len=4
            //   iv: S0 c0 93 7f 81 len=4
            //   s2v: S0 c0 93 7f 81 len=4
            //   vl: S0 c0 93 7f 81 len=4
            ("S0", b"S0 c0 93 7f 81 len=4"),
        ],
        &[],
    );
}

const X_PS_CONCAT_ASSIGN_ORDER_SRC: &str = r#"module top;
  string r;
  initial #1000 $finish;
  initial begin
    r = {string'($random), "|", $sformatf("%0d", $random)};
    for (int i = 0; i < r.len(); i++) $write("%02x", r[i]);
    $display("");
    #1 $finish;
  end
endmodule
"#;

#[test]
fn x_ps_concat_assign_order_moves() {
    check(
        &Cell {
            src: X_PS_CONCAT_ASSIGN_ORDER_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::Word,
            staged: true,
        },
        &[
            // #1: MOVE = HAND (PLAN 5 no-oracle: the raw 4 bytes of the same $random value replace efbfbd (vl values differ, iv aborts)) ; PRE efbfbdefbfbd5eefbfbd7c333033333739373438
            ("#1", b"c0895e817c333033333739373438"),
        ],
        &[],
    );
}

const Q2_BOUNDARY_SRC: &str = r#"module top;
  integer fd, n; string x, l8, l7;
  initial begin
    l8 = string'(24'h804142); l7 = string'(24'h7f4142);
    fd = $fopen("c8.bin", "r"); x = ""; n = $fscanf(fd, "\310%s", x); $display("@F_c8 %0d %0d", n, x.len()); $fclose(fd);
    fd = $fopen("7f.bin", "r"); x = ""; n = $fscanf(fd, "\177%s", x); $display("@F_7f %0d %0d", n, x.len()); $fclose(fd);
    fd = $fopen("80.bin", "r"); x = ""; n = $fscanf(fd, "\200%s", x); $display("@F_80 %0d %0d", n, x.len()); $fclose(fd);
    x = ""; n = $sscanf(l8, "\200%s", x); $display("@S_80 %0d %0d", n, x.len());
    x = ""; n = $sscanf(l7, "\177%s", x); $display("@S_7f %0d %0d", n, x.len());
    #1 $finish;
  end
endmodule
"#;

#[test]
fn q2_boundary_moves() {
    check(
        &Cell {
            src: Q2_BOUNDARY_SRC,
            inputs: &[
                ("7f.bin", b"\x7fAB\x0a"),
                ("80.bin", b"\x80AB\x0a"),
                ("c8.bin", b"\xc8AB\x0a"),
            ],
            plus: None,
            mode: Mode::At,
            staged: true,
        },
        &[
            // F_c8: MOVE = JUDGED (D7 qstrbytes-Q2: as p11 Q2 ; the oracles' $sscanf twin S_80 = 1 2) ; PRE @F_c8 0 0
            //   iv: @F_c8 0 0
            //   vl: @F_c8 0 0
            ("F_c8", b"@F_c8 1 2"),
            // F_80: MOVE = JUDGED (D7 qstrbytes-Q2: lone 0x80 literal byte matches as S_80 (iv = vl 1 2)) ; PRE @F_80 0 0
            //   iv: @F_80 0 0
            //   vl: @F_80 0 0
            ("F_80", b"@F_80 1 2"),
            // S_80: MOVE = iv+vl ; PRE @S_80 0 0
            //   iv: @S_80 1 2
            //   vl: @S_80 1 2
            ("S_80", b"@S_80 1 2"),
        ],
        &[],
    );
}

#[test]
fn q2_boundary_stays() {
    check(
        &Cell {
            src: Q2_BOUNDARY_SRC,
            inputs: &[
                ("7f.bin", b"\x7fAB\x0a"),
                ("80.bin", b"\x80AB\x0a"),
                ("c8.bin", b"\xc8AB\x0a"),
            ],
            plus: None,
            mode: Mode::At,
            staged: true,
        },
        &[
            // F_7f: STAY = iv+vl ; PRE @F_7f 1 2
            //   iv: @F_7f 1 2
            //   vl: @F_7f 1 2
            ("F_7f", b"@F_7f 1 2"),
            // S_7f: STAY = iv+vl ; PRE @S_7f 1 2
            //   iv: @S_7f 1 2
            //   vl: @S_7f 1 2
            ("S_7f", b"@S_7f 1 2"),
        ],
        &[],
    );
}

const C1_PLUS_SPLIT_SRC: &str = r#"module top;
  // C1 twin of MN9: a plusarg query that ends inside a UTF-8 character (byte prefix c3 of "é" = c3 a9). A byte compare
  // matches; the lossy query (U+FFFD) did not.
  string x;
  initial begin
    $display("U4 %0d", $test$plusargs("S=\303"));
    x = "-"; if ($value$plusargs("S=\303%s", x)) $display("U5 1 [%s] %0d", x, x.len()); else $display("U5 0 [%s]", x);
    #1 $finish;
  end
endmodule
"#;

#[test]
fn c1_plus_split_moves() {
    check(
        &Cell {
            src: C1_PLUS_SPLIT_SRC,
            inputs: &[],
            plus: Some(b"+S=\xc3\xa9"),
            mode: Mode::Word,
            staged: true,
        },
        &[
            // U4: MOVE = iv+vl ; PRE U4 0
            //   iv: U4 1
            //   vl: U4 1
            ("U4", b"U4 1"),
            // U5: MOVE = iv+vl ; PRE U5 0 [-]
            //   iv: U5 1 [\xa9] 1
            //   vl: U5 1 [\xa9] 1
            ("U5", b"U5 1 [\xa9] 1"),
        ],
        &[],
    );
}
