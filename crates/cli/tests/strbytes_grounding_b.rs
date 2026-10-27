//! Q-STRBYTES pins, GENERATED from the Q-STRBYTES grounding cells (W/qstrbytes/cells, cells2) by W/qstrbytes/r1/bin/gen_pins.py.
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

const G_C15_STROBE_MON_SRC: &str = r#"module top;
  task automatic hx(input string tag, input string a); $write("@%s", tag); for (int i = 0; i < a.len(); i++) $write(" %02x", a[i]); $display(" len=%0d", a.len()); endtask
  string s;
  initial begin
    s = string'(16'hC841);
    $strobe("@B1 %s|", s);
    $monitor("@B2 %s|", s);
    #1 s = string'(16'hFF42);
    #1 $finish;
  end
endmodule
"#;

#[test]
fn g_c15_strobe_mon_moves() {
    check(
        &Cell {
            src: G_C15_STROBE_MON_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::At,
            staged: true,
        },
        &[
            // B2: MOVE = vl ; PRE @B2 \xef\xbf\xbdA|
            //   vl: @B2 \xc8A|
            ("B2", b"@B2 \xc8A|"),
            // B1: MOVE = vl ; PRE @B1 \xef\xbf\xbdA|
            //   vl: @B1 \xc8A|
            ("B1", b"@B1 \xc8A|"),
            // B2#2: MOVE = vl ; PRE @B2 \xef\xbf\xbdB|
            //   vl: @B2 \xffB|
            ("B2#2", b"@B2 \xffB|"),
        ],
        &[],
    );
}

const G_C16_SEVERITY_SRC: &str = r#"module top;
  task automatic hx(input string tag, input string a); $write("@%s", tag); for (int i = 0; i < a.len(); i++) $write(" %02x", a[i]); $display(" len=%0d", a.len()); endtask
  string s;
  initial begin
    s = string'(16'hC841);
    $info("@E1 %s|", s);
    $warning("@E2 %s|", s);
    $display("@E3 end");
    #1 $finish;
  end
endmodule
"#;

#[test]
fn g_c16_severity_moves() {
    check(
        &Cell {
            src: G_C16_SEVERITY_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::At,
            staged: true,
        },
        &[
            // E1: MOVE = iv+s2v+vl ; PRE @E1 \xef\xbf\xbdA|
            //   iv: @E1 \xc8A|
            //   s2v: @E1 \xc8A|
            //   vl: @E1 \xc8A|
            ("E1", b"@E1 \xc8A|"),
            // E2: MOVE = iv+s2v+vl ; PRE @E2 \xef\xbf\xbdA|
            //   iv: @E2 \xc8A|
            //   s2v: @E2 \xc8A|
            //   vl: @E2 \xc8A|
            ("E2", b"@E2 \xc8A|"),
        ],
        &[],
    );
}

#[test]
fn g_c16_severity_stays() {
    check(
        &Cell {
            src: G_C16_SEVERITY_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::At,
            staged: true,
        },
        &[
            // E3: STAY = iv+s2v+vl ; PRE @E3 end
            //   iv: @E3 end
            //   s2v: @E3 end
            //   vl: @E3 end
            ("E3", b"@E3 end"),
        ],
        &[],
    );
}

const G_C17_PCTP_SRC: &str = r#"module top;
  task automatic hx(input string tag, input string a); $write("@%s", tag); for (int i = 0; i < a.len(); i++) $write(" %02x", a[i]); $display(" len=%0d", a.len()); endtask
  string s; string q[$];
  initial begin
    s = string'(16'hC841); q.push_back(s);
    $display("@Q1 %p|", s);
    $display("@Q2 %p|", q);
    #1 $finish;
  end
endmodule
"#;

#[test]
fn g_c17_pctp_moves() {
    check(
        &Cell {
            src: G_C17_PCTP_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::At,
            staged: true,
        },
        &[
            // Q1: MOVE = vl ; PRE @Q1 "\xef\xbf\xbdA"|
            //   vl: @Q1 "\xc8A"|
            ("Q1", b"@Q1 \"\xc8A\"|"),
            // Q2: MOVE = vl ; PRE @Q2 '{"\xef\xbf\xbdA"}|
            //   vl: @Q2 '{"\xc8A"}|
            ("Q2", b"@Q2 '{\"\xc8A\"}|"),
        ],
        &[],
    );
}

const G_C18_PARAM_SRC: &str = r#"module top;
  task automatic hx(input string tag, input string a); $write("@%s", tag); for (int i = 0; i < a.len(); i++) $write(" %02x", a[i]); $display(" len=%0d", a.len()); endtask
  parameter string P = "\310A";
  localparam string LP = {"B", P};
  parameter [15:0] Q = "\310A";
  initial begin
    hx("A1", P); hx("A2", LP);
    $display("@A3 %h", Q);
    $display("@A4 %s|%s|%s|", P, LP, Q);
    #1 $finish;
  end
endmodule
"#;

#[test]
fn g_c18_param_moves() {
    check(
        &Cell {
            src: G_C18_PARAM_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::At,
            staged: true,
        },
        &[
            // A4: MOVE = s2v+vl (SPLIT: iv drops a part (PLAN 5 MOVE -> s2v+vl)) ; PRE @A4 \xef\xbf\xbdA|B\xef\xbf\xbdA|\xc3\x88A|
            //   iv: @A4 \xc8A| |\xc8A|
            //   s2v: @A4 \xc8A|B\xc8A|\xc8A|
            //   vl: @A4 \xc8A|B\xc8A|\xc8A|
            ("A4", b"@A4 \xc8A|B\xc8A|\xc8A|"),
        ],
        &[],
    );
}

#[test]
fn g_c18_param_stays() {
    check(
        &Cell {
            src: G_C18_PARAM_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::At,
            staged: true,
        },
        &[
            // A1: STAY = iv+s2v+vl ; PRE @A1 c8 41 len=2
            //   iv: @A1 c8 41 len=2
            //   s2v: @A1 c8 41 len=2
            //   vl: @A1 c8 41 len=2
            ("A1", b"@A1 c8 41 len=2"),
            // A2: STAY = s2v+vl (iv drops the parameter (len=0) ; s2v = vl = PRE) ; PRE @A2 42 c8 41 len=3
            //   iv: @A2 len=0
            //   s2v: @A2 42 c8 41 len=3
            //   vl: @A2 42 c8 41 len=3
            ("A2", b"@A2 42 c8 41 len=3"),
            // A3: STAY = iv+s2v+vl ; PRE @A3 c841
            //   iv: @A3 c841
            //   s2v: @A3 c841
            //   vl: @A3 c841
            ("A3", b"@A3 c841"),
        ],
        &[],
    );
}

const G_C20_NESTED_SRC: &str = r#"module top;
  task automatic hx(input string tag, input string a); $write("@%s", tag); for (int i = 0; i < a.len(); i++) $write(" %02x", a[i]); $display(" len=%0d", a.len()); endtask
  string s, r;
  initial begin
    s = string'(16'hC841);
    $display("@N1 %s|", $sformatf("%s", s));
    r = {s, $sformatf("%s", s)}; hx("N2", r);
    $display("@N3 [%s]", $sformatf("<%s>", s));
    #1 $finish;
  end
endmodule
"#;

#[test]
fn g_c20_nested_moves() {
    check(
        &Cell {
            src: G_C20_NESTED_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::At,
            staged: true,
        },
        &[
            // N1: MOVE = iv+s2v+vl ; PRE @N1 \xef\xbf\xbdA|
            //   iv: @N1 \xc8A|
            //   s2v: @N1 \xc8A|
            //   vl: @N1 \xc8A|
            ("N1", b"@N1 \xc8A|"),
            // N2: MOVE = iv+s2v+vl ; PRE @N2 ef bf bd 41 ef bf bd 41 len=8
            //   iv: @N2 c8 41 c8 41 len=4
            //   s2v: @N2 c8 41 c8 41 len=4
            //   vl: @N2 c8 41 c8 41 len=4
            ("N2", b"@N2 c8 41 c8 41 len=4"),
            // N3: MOVE = iv+s2v+vl ; PRE @N3 [<\xef\xbf\xbdA>]
            //   iv: @N3 [<\xc8A>]
            //   s2v: @N3 [<\xc8A>]
            //   vl: @N3 [<\xc8A>]
            ("N3", b"@N3 [<\xc8A>]"),
        ],
        &[],
    );
}

const G_C21_PCTU_SRC: &str = r#"module top;
  task automatic hx(input string tag, input string a); $write("@%s", tag); for (int i = 0; i < a.len(); i++) $write(" %02x", a[i]); $display(" len=%0d", a.len()); endtask
  integer fd, c;
  initial begin
    fd = $fopen("fo.bin", "w");
    $fwrite(fd, "%u", 32'hC0937F81);
    $fwrite(fd, "|%z|", 8'hC8);
    $fclose(fd);
    fd = $fopen("fo.bin", "r");
    $write("@U1");
    c = $fgetc(fd);
    while (c != -1) begin $write(" %02x", c[7:0]); c = $fgetc(fd); end
    $display("");
    $fclose(fd);
    #1 $finish;
  end
endmodule
"#;

#[test]
fn g_c21_pctu_stays() {
    check(
        &Cell {
            src: G_C21_PCTU_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::At,
            staged: true,
        },
        &[
            // U1: STAY = PRE (NOT THIS ROW: P-PCTU (PLAN 5)) ; PRE @U1 7c 7c
            //   iv: @U1 81 7f 93 c0 7c c8 00 00 00 00 00 00 00 7c
            //   s2v: @U1 81 7f 93 c0 7c c8 00 00 00 00 00 00 00 7c
            //   vl: @U1 81 7f 93 c0 7c c8 00 00 00 00 00 00 00 7c
            ("U1", b"@U1 7c 7c"),
        ],
        &[
            // FILE:fo.bin: STAY = PRE (NOT THIS ROW: P-PCTU) ; PRE ||
            ("fo.bin", b"||"),
        ],
    );
}

const G_C22_WIDTH_SRC: &str = r#"module top;
  task automatic hx(input string tag, input string a); $write("@%s", tag); for (int i = 0; i < a.len(); i++) $write(" %02x", a[i]); $display(" len=%0d", a.len()); endtask
  string s;
  initial begin
    s = string'(16'hC841);
    $display("@J1 [%5s]", s);
    $display("@J2 [%-5s]", s);
    $display("@J3 [%3c]", 8'hC8);
    #1 $finish;
  end
endmodule
"#;

#[test]
fn g_c22_width_moves() {
    check(
        &Cell {
            src: G_C22_WIDTH_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::At,
            staged: true,
        },
        &[
            // J1: MOVE = iv+s2v+vl ; PRE @J1 [   \xef\xbf\xbdA]
            //   iv: @J1 [   \xc8A]
            //   s2v: @J1 [   \xc8A]
            //   vl: @J1 [   \xc8A]
            ("J1", b"@J1 [   \xc8A]"),
            // J2: MOVE = iv+s2v+vl ; PRE @J2 [\xef\xbf\xbdA   ]
            //   iv: @J2 [\xc8A   ]
            //   s2v: @J2 [\xc8A   ]
            //   vl: @J2 [\xc8A   ]
            ("J2", b"@J2 [\xc8A   ]"),
            // J3: MOVE = iv+s2v (SPLIT %2c: iv pads, vl not ; vita's existing %Nc rule (PLAN 5)) ; PRE @J3 [  \xc3\x88]
            //   iv: @J3 [  \xc8]
            //   s2v: @J3 [  \xc8]
            //   vl: @J3 [\xc8]
            ("J3", b"@J3 [  \xc8]"),
        ],
        &[],
    );
}

const G_C23_UTF8SRC_SRC: &str = r#"module top;
  task automatic hx(input string tag, input string a); $write("@%s", tag); for (int i = 0; i < a.len(); i++) $write(" %02x", a[i]); $display(" len=%0d", a.len()); endtask
  string s;
  initial begin
    s = "é→"; hx("Z1", s);
    $display("@Z2 é→|");
    $display("@Z3 %s|", s);
    #1 $finish;
  end
endmodule
"#;

#[test]
fn g_c23_utf8src_stays() {
    check(
        &Cell {
            src: G_C23_UTF8SRC_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::At,
            staged: true,
        },
        &[
            // Z1: STAY = iv+s2v+vl ; PRE @Z1 c3 a9 e2 86 92 len=5
            //   iv: @Z1 c3 a9 e2 86 92 len=5
            //   s2v: @Z1 c3 a9 e2 86 92 len=5
            //   vl: @Z1 c3 a9 e2 86 92 len=5
            ("Z1", b"@Z1 c3 a9 e2 86 92 len=5"),
            // Z2: STAY = iv+s2v+vl ; PRE @Z2 \xc3\xa9\xe2\x86\x92|
            //   iv: @Z2 \xc3\xa9\xe2\x86\x92|
            //   s2v: @Z2 \xc3\xa9\xe2\x86\x92|
            //   vl: @Z2 \xc3\xa9\xe2\x86\x92|
            ("Z2", b"@Z2 \xc3\xa9\xe2\x86\x92|"),
            // Z3: STAY = iv+s2v+vl ; PRE @Z3 \xc3\xa9\xe2\x86\x92|
            //   iv: @Z3 \xc3\xa9\xe2\x86\x92|
            //   s2v: @Z3 \xc3\xa9\xe2\x86\x92|
            //   vl: @Z3 \xc3\xa9\xe2\x86\x92|
            ("Z3", b"@Z3 \xc3\xa9\xe2\x86\x92|"),
        ],
        &[],
    );
}

const G_C24_FMTVAR_SRC: &str = r#"module top;
  task automatic hx(input string tag, input string a); $write("@%s", tag); for (int i = 0; i < a.len(); i++) $write(" %02x", a[i]); $display(" len=%0d", a.len()); endtask
  string f, s;
  initial begin
    f = {"@G1 ", string'(8'hC8), " %0d|"};
    $display(f, 5);
    #1 $finish;
  end
endmodule
"#;

#[test]
fn g_c24_fmtvar_moves() {
    check(
        &Cell {
            src: G_C24_FMTVAR_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::At,
            staged: true,
        },
        &[
            // G1: MOVE = iv+s2v (SPLIT: vl prints the format text of a string-variable format (PLAN 5)) ; PRE @G1 \xef\xbf\xbd 5|
            //   iv: @G1 \xc8 5|
            //   s2v: @G1 \xc8 5|
            //   vl: @G1 \xc8 %0d|          5
            ("G1", b"@G1 \xc8 5|"),
        ],
        &[],
    );
}

const G_C25_ELABINFO_SRC: &str = r#"module top;
  task automatic hx(input string tag, input string a); $write("@%s", tag); for (int i = 0; i < a.len(); i++) $write(" %02x", a[i]); $display(" len=%0d", a.len()); endtask
  parameter string P = "\310A";
  localparam string LS = $sformatf("%s!", P);
  $info("@I1 %s|", P);
  initial begin
    hx("I2", LS);
    #1 $finish;
  end
endmodule
"#;

#[test]
fn g_c25_elabinfo_moves() {
    check(
        &Cell {
            src: G_C25_ELABINFO_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::At,
            staged: true,
        },
        &[
            // I1: MOVE = vl ; PRE @I1 \xef\xbf\xbdA|
            //   vl: @I1 \xc8A|
            ("I1", b"@I1 \xc8A|"),
        ],
        &[],
    );
}

const G_C26_TIMEFMT_SRC: &str = r#"module top;
  task automatic hx(input string tag, input string a); $write("@%s", tag); for (int i = 0; i < a.len(); i++) $write(" %02x", a[i]); $display(" len=%0d", a.len()); endtask
  initial begin
    $timeformat(0, 0, "\310s", 6);
    #3 $display("@T1 %t|", $time);
    #1 $finish;
  end
endmodule
"#;

#[test]
fn g_c26_timefmt_moves() {
    check(
        &Cell {
            src: G_C26_TIMEFMT_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::At,
            staged: true,
        },
        &[
            // T1: MOVE = vl (SPLIT on the digit: suffix byte -> c8 (all 3), digit 0 = vl = BASE (PLAN 5)) ; PRE @T1    0\xef\xbf\xbds|
            //   iv: @T1    3\xc8s|
            //   s2v: @T1    3\xc8s|
            //   vl: @T1    0\xc8s|
            ("T1", b"@T1    0\xc8s|"),
        ],
        &[],
    );
}

const G_C27_FRAMES_SRC: &str = r#"module top;
  task automatic hx(input string tag, input string a); $write("@%s", tag); for (int i = 0; i < a.len(); i++) $write(" %02x", a[i]); $display(" len=%0d", a.len()); endtask
  string g;
  function automatic string mk(input string a); return $sformatf("<%s>", a); endfunction
  function automatic void pf(input string a); $display("@H1 %s|", a); endfunction
  task automatic pt(input string a); $display("@H2 %s|", a); endtask
  initial begin
    g = string'(16'hC841);
    hx("H3", mk(g));
    pf(g);
    pt(g);
    $fwrite(32'h8000_0002, "@H4 %s|\n", g);
    #1 $finish;
  end
endmodule
"#;

#[test]
fn g_c27_frames_moves() {
    check(
        &Cell {
            src: G_C27_FRAMES_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::At,
            staged: true,
        },
        &[
            // H3: MOVE = iv+s2v+vl ; PRE @H3 3c ef bf bd 41 3e len=6
            //   iv: @H3 3c c8 41 3e len=4
            //   s2v: @H3 3c c8 41 3e len=4
            //   vl: @H3 3c c8 41 3e len=4
            ("H3", b"@H3 3c c8 41 3e len=4"),
            // H1: MOVE = iv+s2v+vl ; PRE @H1 \xef\xbf\xbdA|
            //   iv: @H1 \xc8A|
            //   s2v: @H1 \xc8A|
            //   vl: @H1 \xc8A|
            ("H1", b"@H1 \xc8A|"),
            // H2: MOVE = iv+s2v+vl ; PRE @H2 \xef\xbf\xbdA|
            //   iv: @H2 \xc8A|
            //   s2v: @H2 \xc8A|
            //   vl: @H2 \xc8A|
            ("H2", b"@H2 \xc8A|"),
            // H4: MOVE = iv+s2v+vl ; PRE @H4 \xef\xbf\xbdA|
            //   iv: @H4 \xc8A|
            //   s2v: @H4 \xc8A|
            //   vl: @H4 \xc8A|
            ("H4", b"@H4 \xc8A|"),
        ],
        &[],
    );
}

const G_C28_SCANFMT_SRC: &str = r#"module top;
  task automatic hx(input string tag, input string a); $write("@%s", tag); for (int i = 0; i < a.len(); i++) $write(" %02x", a[i]); $display(" len=%0d", a.len()); endtask
  string l, x; integer n;
  initial begin
    l = string'(24'hC84142); x = "";
    n = $sscanf(l, "\310%s", x); $display("@X1 %0d", n); hx("X2", x);
    #1 $finish;
  end
endmodule
"#;

#[test]
fn g_c28_scanfmt_moves() {
    check(
        &Cell {
            src: G_C28_SCANFMT_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::At,
            staged: true,
        },
        &[
            // X1: MOVE = iv+s2v+vl ; PRE @X1 0
            //   iv: @X1 1
            //   s2v: @X1 1
            //   vl: @X1 1
            ("X1", b"@X1 1"),
            // X2: MOVE = iv+s2v+vl ; PRE @X2 len=0
            //   iv: @X2 41 42 len=2
            //   s2v: @X2 41 42 len=2
            //   vl: @X2 41 42 len=2
            ("X2", b"@X2 41 42 len=2"),
        ],
        &[],
    );
}

const G_C30_ASSOC_P_SRC: &str = r#"module top;
  int aa [string];
  initial begin
    aa[string'(16'hC841)] = 5;
    $display("@V3 %p|", aa);
    #1 $finish;
  end
endmodule
"#;

#[test]
fn g_c30_assoc_p_moves() {
    check(
        &Cell {
            src: G_C30_ASSOC_P_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::At,
            staged: true,
        },
        &[
            // V3: MOVE = vl ; PRE @V3 '{"\xef\xbf\xbdA":'h5}|
            //   vl: @V3 '{"\xc8A":'h5}|
            ("V3", b"@V3 '{\"\xc8A\":'h5}|"),
        ],
        &[],
    );
}
