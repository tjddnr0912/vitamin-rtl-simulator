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

const G_C01_LIT_SRC: &str = r#"module top;
  task automatic hx(input string tag, input string a); $write("@%s", tag); for (int i = 0; i < a.len(); i++) $write(" %02x", a[i]); $display(" len=%0d", a.len()); endtask
  string s, t; byte b; logic [15:0] p;
  initial begin
    s = "\310\377A"; hx("L1", s);
    t = "\xC8\xFFA"; hx("L2", t);
    b = "\310"; $display("@L3 %h", b);
    p = "\310\x81"; $display("@L4 %h", p);
    $display("@L5 \310\xC8|");
    $display("@L6 %s|", "\310\377A");
    $display("@L7 %s|", s);
    #1 $finish;
  end
endmodule
"#;

#[test]
fn g_c01_lit_moves() {
    check(
        &Cell {
            src: G_C01_LIT_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::At,
            staged: true,
        },
        &[
            // L5: MOVE = iv+s2v+vl ; PRE @L5 \xef\xbf\xbd\xef\xbf\xbd|
            //   iv: @L5 \xc8\xc8|
            //   s2v: @L5 \xc8\xc8|
            //   vl: @L5 \xc8\xc8|
            ("L5", b"@L5 \xc8\xc8|"),
            // L6: MOVE = iv+s2v+vl ; PRE @L6 \xef\xbf\xbd\xef\xbf\xbdA|
            //   iv: @L6 \xc8\xffA|
            //   s2v: @L6 \xc8\xffA|
            //   vl: @L6 \xc8\xffA|
            ("L6", b"@L6 \xc8\xffA|"),
            // L7: MOVE = iv+s2v+vl ; PRE @L7 \xef\xbf\xbd\xef\xbf\xbdA|
            //   iv: @L7 \xc8\xffA|
            //   s2v: @L7 \xc8\xffA|
            //   vl: @L7 \xc8\xffA|
            ("L7", b"@L7 \xc8\xffA|"),
        ],
        &[],
    );
}

#[test]
fn g_c01_lit_stays() {
    check(
        &Cell {
            src: G_C01_LIT_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::At,
            staged: true,
        },
        &[
            // L1: STAY = iv+s2v+vl ; PRE @L1 c8 ff 41 len=3
            //   iv: @L1 c8 ff 41 len=3
            //   s2v: @L1 c8 ff 41 len=3
            //   vl: @L1 c8 ff 41 len=3
            ("L1", b"@L1 c8 ff 41 len=3"),
            // L2: STAY = iv+s2v+vl ; PRE @L2 c8 ff 41 len=3
            //   iv: @L2 c8 ff 41 len=3
            //   s2v: @L2 c8 ff 41 len=3
            //   vl: @L2 c8 ff 41 len=3
            ("L2", b"@L2 c8 ff 41 len=3"),
            // L3: STAY = iv+s2v+vl ; PRE @L3 c8
            //   iv: @L3 c8
            //   s2v: @L3 c8
            //   vl: @L3 c8
            ("L3", b"@L3 c8"),
            // L4: STAY = PRE (SPLIT: vita c881 = vl = IEEE octal \310 ; iv/s2v ff81 (PLAN 5 STAY, MN7)) ; PRE @L4 c881
            //   iv: @L4 ff81
            //   s2v: @L4 ff81
            //   vl: @L4 c881
            ("L4", b"@L4 c881"),
        ],
        &[],
    );
}

const G_C02_CAST_SRC: &str = r#"module top;
  task automatic hx(input string tag, input string a); $write("@%s", tag); for (int i = 0; i < a.len(); i++) $write(" %02x", a[i]); $display(" len=%0d", a.len()); endtask
  string s, t, u; logic [31:0] v = 32'hC0937F81;
  initial begin
    s = string'(v); hx("S0", s);
    t = string'(8'hC8); hx("S1", t);
    u = string'({8'hC8, 8'h41, 8'h80}); hx("S2", u);
    $display("@S3 %s|%s|%s", s, t, u);
    #1 $finish;
  end
endmodule
"#;

#[test]
fn g_c02_cast_moves() {
    check(
        &Cell {
            src: G_C02_CAST_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::At,
            staged: true,
        },
        &[
            // S3: MOVE = iv+s2v+vl ; PRE @S3 \xef\xbf\xbd\xef\xbf\xbd\x7f\xef\xbf\xbd|\xef\xbf\xbd|\xef\xbf\xbdA\xef\xbf\xbd
            //   iv: @S3 \xc0\x93\x7f\x81|\xc8|\xc8A\x80
            //   s2v: @S3 \xc0\x93\x7f\x81|\xc8|\xc8A\x80
            //   vl: @S3 \xc0\x93\x7f\x81|\xc8|\xc8A\x80
            ("S3", b"@S3 \xc0\x93\x7f\x81|\xc8|\xc8A\x80"),
        ],
        &[],
    );
}

#[test]
fn g_c02_cast_stays() {
    check(
        &Cell {
            src: G_C02_CAST_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::At,
            staged: true,
        },
        &[
            // S0: STAY = iv+s2v+vl ; PRE @S0 c0 93 7f 81 len=4
            //   iv: @S0 c0 93 7f 81 len=4
            //   s2v: @S0 c0 93 7f 81 len=4
            //   vl: @S0 c0 93 7f 81 len=4
            ("S0", b"@S0 c0 93 7f 81 len=4"),
            // S1: STAY = iv+s2v+vl ; PRE @S1 c8 len=1
            //   iv: @S1 c8 len=1
            //   s2v: @S1 c8 len=1
            //   vl: @S1 c8 len=1
            ("S1", b"@S1 c8 len=1"),
            // S2: STAY = iv+s2v+vl ; PRE @S2 c8 41 80 len=3
            //   iv: @S2 c8 41 80 len=3
            //   s2v: @S2 c8 41 80 len=3
            //   vl: @S2 c8 41 80 len=3
            ("S2", b"@S2 c8 41 80 len=3"),
        ],
        &[],
    );
}

const G_C03_CONCAT_SRC: &str = r#"module top;
  task automatic hx(input string tag, input string a); $write("@%s", tag); for (int i = 0; i < a.len(); i++) $write(" %02x", a[i]); $display(" len=%0d", a.len()); endtask
  string s, t, r;
  initial begin
    s = string'(16'hC841); t = string'(8'hFF);
    r = {s, "!"}; hx("C1", r);
    r = {s, t}; hx("C2", r);
    r = {2{s}}; hx("C3", r);
    r = {"A", t, "B"}; hx("C4", r);
    t = {t, ""}; hx("C5", t);
    $display("@C6 %s", {s, t});
    #1 $finish;
  end
endmodule
"#;

#[test]
fn g_c03_concat_moves() {
    check(
        &Cell {
            src: G_C03_CONCAT_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::At,
            staged: true,
        },
        &[
            // C1: MOVE = iv+s2v+vl ; PRE @C1 ef bf bd 41 21 len=5
            //   iv: @C1 c8 41 21 len=3
            //   s2v: @C1 c8 41 21 len=3
            //   vl: @C1 c8 41 21 len=3
            ("C1", b"@C1 c8 41 21 len=3"),
            // C2: MOVE = iv+s2v+vl ; PRE @C2 ef bf bd 41 ef bf bd len=7
            //   iv: @C2 c8 41 ff len=3
            //   s2v: @C2 c8 41 ff len=3
            //   vl: @C2 c8 41 ff len=3
            ("C2", b"@C2 c8 41 ff len=3"),
            // C3: MOVE = iv+s2v+vl ; PRE @C3 ef bf bd 41 ef bf bd 41 len=8
            //   iv: @C3 c8 41 c8 41 len=4
            //   s2v: @C3 c8 41 c8 41 len=4
            //   vl: @C3 c8 41 c8 41 len=4
            ("C3", b"@C3 c8 41 c8 41 len=4"),
            // C4: MOVE = iv+s2v+vl ; PRE @C4 41 ef bf bd 42 len=5
            //   iv: @C4 41 ff 42 len=3
            //   s2v: @C4 41 ff 42 len=3
            //   vl: @C4 41 ff 42 len=3
            ("C4", b"@C4 41 ff 42 len=3"),
            // C5: MOVE = iv+s2v+vl ; PRE @C5 ef bf bd len=3
            //   iv: @C5 ff len=1
            //   s2v: @C5 ff len=1
            //   vl: @C5 ff len=1
            ("C5", b"@C5 ff len=1"),
            // C6: MOVE = iv+s2v+vl ; PRE @C6 \xef\xbf\xbdA\xef\xbf\xbd
            //   iv: @C6 \xc8A\xff
            //   s2v: @C6 \xc8A\xff
            //   vl: @C6 \xc8A\xff
            ("C6", b"@C6 \xc8A\xff"),
        ],
        &[],
    );
}

const G_C04_PUTC_SRC: &str = r#"module top;
  task automatic hx(input string tag, input string a); $write("@%s", tag); for (int i = 0; i < a.len(); i++) $write(" %02x", a[i]); $display(" len=%0d", a.len()); endtask
  string s, t;
  initial begin
    s = "ABC"; s.putc(1, 8'hC8); hx("P1", s);
    s[2] = 8'hFF; hx("P2", s);
    $display("@P3 %s|", s);
    t = s.toupper(); hx("P4", t);
    t = s.tolower(); hx("P5", t);
    t.itoa(-5); hx("P6", t);
    t.hextoa(32'hC8); hx("P7", t);
    t = s.substr(1, 2); hx("P8", t);
    #1 $finish;
  end
endmodule
"#;

#[test]
fn g_c04_putc_moves() {
    check(
        &Cell {
            src: G_C04_PUTC_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::At,
            staged: true,
        },
        &[
            // P3: MOVE = vl ; PRE @P3 A\xef\xbf\xbd\xef\xbf\xbd|
            //   vl: @P3 A\xc8\xff|
            ("P3", b"@P3 A\xc8\xff|"),
        ],
        &[],
    );
}

#[test]
fn g_c04_putc_stays() {
    check(
        &Cell {
            src: G_C04_PUTC_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::At,
            staged: true,
        },
        &[
            // P1: STAY = vl ; PRE @P1 41 c8 43 len=3
            //   vl: @P1 41 c8 43 len=3
            ("P1", b"@P1 41 c8 43 len=3"),
            // P2: STAY = vl ; PRE @P2 41 c8 ff len=3
            //   vl: @P2 41 c8 ff len=3
            ("P2", b"@P2 41 c8 ff len=3"),
            // P4: STAY = vl ; PRE @P4 41 c8 ff len=3
            //   vl: @P4 41 c8 ff len=3
            ("P4", b"@P4 41 c8 ff len=3"),
            // P5: STAY = vl ; PRE @P5 61 c8 ff len=3
            //   vl: @P5 61 c8 ff len=3
            ("P5", b"@P5 61 c8 ff len=3"),
            // P6: STAY = vl ; PRE @P6 2d 35 len=2
            //   vl: @P6 2d 35 len=2
            ("P6", b"@P6 2d 35 len=2"),
            // P7: STAY = vl ; PRE @P7 63 38 len=2
            //   vl: @P7 63 38 len=2
            ("P7", b"@P7 63 38 len=2"),
            // P8: STAY = vl ; PRE @P8 c8 ff len=2
            //   vl: @P8 c8 ff len=2
            ("P8", b"@P8 c8 ff len=2"),
        ],
        &[],
    );
}

const G_C05_SFORMATF_SRC: &str = r#"module top;
  task automatic hx(input string tag, input string a); $write("@%s", tag); for (int i = 0; i < a.len(); i++) $write(" %02x", a[i]); $display(" len=%0d", a.len()); endtask
  string s, r; logic [31:0] v = 32'hC0937F81;
  initial begin
    s = string'(v);
    r = $sformatf("%s", s); hx("K1", r);
    r = $sformatf("%s", v); hx("K2", r);
    r = $sformatf("%c", 8'hC0); hx("K3", r);
    r = $sformatf("x%sy", s); hx("K4", r);
    $swrite(r, "%s", s); hx("K5", r);
    $sformat(r, "%s", s); hx("K6", r);
    r = $sformatf("\310|"); hx("K7", r);
    r = $sformatf("%s", "\310\377"); hx("K8", r);
    #1 $finish;
  end
endmodule
"#;

#[test]
fn g_c05_sformatf_moves() {
    check(
        &Cell {
            src: G_C05_SFORMATF_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::At,
            staged: true,
        },
        &[
            // K1: MOVE = iv+s2v+vl ; PRE @K1 ef bf bd ef bf bd 7f ef bf bd len=10
            //   iv: @K1 c0 93 7f 81 len=4
            //   s2v: @K1 c0 93 7f 81 len=4
            //   vl: @K1 c0 93 7f 81 len=4
            ("K1", b"@K1 c0 93 7f 81 len=4"),
            // K2: MOVE = iv+s2v+vl ; PRE @K2 c3 80 c2 93 7f c2 81 len=7
            //   iv: @K2 c0 93 7f 81 len=4
            //   s2v: @K2 c0 93 7f 81 len=4
            //   vl: @K2 c0 93 7f 81 len=4
            ("K2", b"@K2 c0 93 7f 81 len=4"),
            // K3: MOVE = iv+s2v+vl ; PRE @K3 c3 80 len=2
            //   iv: @K3 c0 len=1
            //   s2v: @K3 c0 len=1
            //   vl: @K3 c0 len=1
            ("K3", b"@K3 c0 len=1"),
            // K4: MOVE = iv+s2v+vl ; PRE @K4 78 ef bf bd ef bf bd 7f ef bf bd 79 len=12
            //   iv: @K4 78 c0 93 7f 81 79 len=6
            //   s2v: @K4 78 c0 93 7f 81 79 len=6
            //   vl: @K4 78 c0 93 7f 81 79 len=6
            ("K4", b"@K4 78 c0 93 7f 81 79 len=6"),
            // K5: MOVE = iv+s2v+vl ; PRE @K5 ef bf bd ef bf bd 7f ef bf bd len=10
            //   iv: @K5 c0 93 7f 81 len=4
            //   s2v: @K5 c0 93 7f 81 len=4
            //   vl: @K5 c0 93 7f 81 len=4
            ("K5", b"@K5 c0 93 7f 81 len=4"),
            // K6: MOVE = iv+s2v+vl ; PRE @K6 ef bf bd ef bf bd 7f ef bf bd len=10
            //   iv: @K6 c0 93 7f 81 len=4
            //   s2v: @K6 c0 93 7f 81 len=4
            //   vl: @K6 c0 93 7f 81 len=4
            ("K6", b"@K6 c0 93 7f 81 len=4"),
            // K7: MOVE = iv+s2v+vl ; PRE @K7 ef bf bd 7c len=4
            //   iv: @K7 c8 7c len=2
            //   s2v: @K7 c8 7c len=2
            //   vl: @K7 c8 7c len=2
            ("K7", b"@K7 c8 7c len=2"),
            // K8: MOVE = iv+s2v+vl ; PRE @K8 ef bf bd ef bf bd len=6
            //   iv: @K8 c8 ff len=2
            //   s2v: @K8 c8 ff len=2
            //   vl: @K8 c8 ff len=2
            ("K8", b"@K8 c8 ff len=2"),
        ],
        &[],
    );
}

const G_C06_PCTC_SRC: &str = r#"module top;
  task automatic hx(input string tag, input string a); $write("@%s", tag); for (int i = 0; i < a.len(); i++) $write(" %02x", a[i]); $display(" len=%0d", a.len()); endtask
  logic [7:0] c = 8'hC8; int i = 32'h1FF;
  initial begin
    $display("@D1 %c|", c);
    $write("@D2 %c%c|\n", 8'hC8, 8'h80);
    $display("@D3 %c|", i);
    $display("@D4 %c|", 8'h41);
    #1 $finish;
  end
endmodule
"#;

#[test]
fn g_c06_pctc_moves() {
    check(
        &Cell {
            src: G_C06_PCTC_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::At,
            staged: true,
        },
        &[
            // D1: MOVE = iv+s2v+vl ; PRE @D1 \xc3\x88|
            //   iv: @D1 \xc8|
            //   s2v: @D1 \xc8|
            //   vl: @D1 \xc8|
            ("D1", b"@D1 \xc8|"),
            // D2: MOVE = iv+s2v+vl ; PRE @D2 \xc3\x88\xc2\x80|
            //   iv: @D2 \xc8\x80|
            //   s2v: @D2 \xc8\x80|
            //   vl: @D2 \xc8\x80|
            ("D2", b"@D2 \xc8\x80|"),
            // D3: MOVE = iv+s2v+vl ; PRE @D3 \xc3\xbf|
            //   iv: @D3 \xff|
            //   s2v: @D3 \xff|
            //   vl: @D3 \xff|
            ("D3", b"@D3 \xff|"),
        ],
        &[],
    );
}

#[test]
fn g_c06_pctc_stays() {
    check(
        &Cell {
            src: G_C06_PCTC_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::At,
            staged: true,
        },
        &[
            // D4: STAY = iv+s2v+vl ; PRE @D4 A|
            //   iv: @D4 A|
            //   s2v: @D4 A|
            //   vl: @D4 A|
            ("D4", b"@D4 A|"),
        ],
        &[],
    );
}

const G_C07_VEC_SRC: &str = r#"module top;
  task automatic hx(input string tag, input string a); $write("@%s", tag); for (int i = 0; i < a.len(); i++) $write(" %02x", a[i]); $display(" len=%0d", a.len()); endtask
  reg [23:0] v = 24'hC841FF; string r;
  initial begin
    $display("@V1 %s|", v);
    $display("@V2 %s|", 8'hC8);
    $display("@V3 %0s|", 32'h00C841);
    r = $sformatf("%s", v); hx("V4", r);
    $display("@V5 %s|", v[15:8]);
    #1 $finish;
  end
endmodule
"#;

#[test]
fn g_c07_vec_moves() {
    check(
        &Cell {
            src: G_C07_VEC_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::At,
            staged: true,
        },
        &[
            // V1: MOVE = iv+s2v+vl ; PRE @V1 \xc3\x88A\xc3\xbf|
            //   iv: @V1 \xc8A\xff|
            //   s2v: @V1 \xc8A\xff|
            //   vl: @V1 \xc8A\xff|
            ("V1", b"@V1 \xc8A\xff|"),
            // V2: MOVE = iv+s2v+vl ; PRE @V2 \xc3\x88|
            //   iv: @V2 \xc8|
            //   s2v: @V2 \xc8|
            //   vl: @V2 \xc8|
            ("V2", b"@V2 \xc8|"),
            // V3: MOVE = iv+s2v+vl ; PRE @V3 \xc3\x88A|
            //   iv: @V3 \xc8A|
            //   s2v: @V3 \xc8A|
            //   vl: @V3 \xc8A|
            ("V3", b"@V3 \xc8A|"),
            // V4: MOVE = iv+s2v+vl ; PRE @V4 c3 88 41 c3 bf len=5
            //   iv: @V4 c8 41 ff len=3
            //   s2v: @V4 c8 41 ff len=3
            //   vl: @V4 c8 41 ff len=3
            ("V4", b"@V4 c8 41 ff len=3"),
        ],
        &[],
    );
}

#[test]
fn g_c07_vec_stays() {
    check(
        &Cell {
            src: G_C07_VEC_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::At,
            staged: true,
        },
        &[
            // V5: STAY = iv+s2v+vl ; PRE @V5 A|
            //   iv: @V5 A|
            //   s2v: @V5 A|
            //   vl: @V5 A|
            ("V5", b"@V5 A|"),
        ],
        &[],
    );
}

const G_C08_FWRITE_SRC: &str = r#"module top;
  task automatic hx(input string tag, input string a); $write("@%s", tag); for (int i = 0; i < a.len(); i++) $write(" %02x", a[i]); $display(" len=%0d", a.len()); endtask
  string s; integer fd, c; string r;
  initial begin
    s = string'(16'hC841);
    fd = $fopen("fo.bin", "w");
    $fwrite(fd, "%s|", s);
    $fdisplay(fd, "%c|\310|", 8'hFF);
    $fwrite(fd, s);
    $fclose(fd);
    fd = $fopen("fo.bin", "r");
    $write("@F1");
    c = $fgetc(fd);
    while (c != -1) begin $write(" %02x", c[7:0]); c = $fgetc(fd); end
    $display("");
    $fclose(fd);
    #1 $finish;
  end
endmodule
"#;

#[test]
fn g_c08_fwrite_moves() {
    check(
        &Cell {
            src: G_C08_FWRITE_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::At,
            staged: true,
        },
        &[
            // F1: MOVE = iv+s2v+vl ; PRE @F1 ef bf bd 41 7c c3 bf 7c ef bf bd 7c 0a ef bf bd 41
            //   iv: @F1 c8 41 7c ff 7c c8 7c 0a c8 41
            //   s2v: @F1 c8 41 7c ff 7c c8 7c 0a c8 41
            //   vl: @F1 c8 41 7c ff 7c c8 7c 0a c8 41
            ("F1", b"@F1 c8 41 7c ff 7c c8 7c 0a c8 41"),
        ],
        &[
            // FILE:fo.bin: MOVE = iv+s2v+vl ; PRE \xef\xbf\xbdA|\xc3\xbf|\xef\xbf\xbd|\x0a\xef\xbf\xbdA
            ("fo.bin", b"\xc8A|\xff|\xc8|\x0a\xc8A"),
        ],
    );
}

const G_C09_FREAD_SRC: &str = r#"module top;
  task automatic hx(input string tag, input string a); $write("@%s", tag); for (int i = 0; i < a.len(); i++) $write(" %02x", a[i]); $display(" len=%0d", a.len()); endtask
  integer fd, c, n; string l, w, x; reg [7:0] m [0:3]; byte bb;
  initial begin
    fd = $fopen("in.bin", "r");
    n = $fgets(l, fd); hx("R1", l);
    n = $fscanf(fd, "%s", w); hx("R2", w);
    c = $fgetc(fd); c = $fgetc(fd); $display("@R3 %02x", c[7:0]);
    n = $fread(m, fd); $display("@R4 %0d %02x %02x %02x %02x", n, m[0], m[1], m[2], m[3]);
    $fclose(fd);
    l = string'(24'h41C842);
    n = $sscanf(l, "%s", x); hx("R5", x);
    n = $sscanf(l, "%c", bb); $display("@R6 %02x", bb);
    #1 $finish;
  end
endmodule
"#;

#[test]
fn g_c09_fread_stays() {
    check(
        &Cell {
            src: G_C09_FREAD_SRC,
            inputs: &[("in.bin", b"A\xc8\xff\x80B\x0aC\xe9D E\x80\x81\x82\x83\x0a")],
            plus: None,
            mode: Mode::At,
            staged: true,
        },
        &[
            // R1: STAY = iv+vl ; PRE @R1 41 c8 ff 80 42 0a len=6
            //   iv: @R1 41 c8 ff 80 42 0a len=6
            //   vl: @R1 41 c8 ff 80 42 0a len=6
            ("R1", b"@R1 41 c8 ff 80 42 0a len=6"),
            // R2: STAY = iv+vl ; PRE @R2 43 e9 44 len=3
            //   iv: @R2 43 e9 44 len=3
            //   vl: @R2 43 e9 44 len=3
            ("R2", b"@R2 43 e9 44 len=3"),
            // R3: STAY = PRE (SPLIT $fscanf %s consumption iv/vl ; vita = iv (GROUND, PLAN 5 STAY)) ; PRE @R3 45
            //   iv: @R3 45
            //   vl: @R3 20
            ("R3", b"@R3 45"),
            // R4: STAY = PRE (SPLIT as R3 (PLAN 5 STAY)) ; PRE @R4 4 80 81 82 83
            //   iv: @R4 4 80 81 82 83
            //   vl: @R4 4 45 80 81 82
            ("R4", b"@R4 4 80 81 82 83"),
            // R5: STAY = iv+vl ; PRE @R5 41 c8 42 len=3
            //   iv: @R5 41 c8 42 len=3
            //   vl: @R5 41 c8 42 len=3
            ("R5", b"@R5 41 c8 42 len=3"),
            // R6: STAY = iv+vl ; PRE @R6 41
            //   iv: @R6 41
            //   vl: @R6 41
            ("R6", b"@R6 41"),
        ],
        &[],
    );
}

const G_C10_READMEM_SRC: &str = r#"module top;
  task automatic hx(input string tag, input string a); $write("@%s", tag); for (int i = 0; i < a.len(); i++) $write(" %02x", a[i]); $display(" len=%0d", a.len()); endtask
  reg [7:0] m [0:2]; string s;
  initial begin
    $readmemh("mem.hex", m);
    $display("@M1 %02x %02x %02x", m[0], m[1], m[2]);
    $display("@M2 %s|", {m[0], m[1], m[2]});
    s = string'({m[0], m[1], m[2]}); hx("M3", s);
    #1 $finish;
  end
endmodule
"#;

#[test]
fn g_c10_readmem_moves() {
    check(
        &Cell {
            src: G_C10_READMEM_SRC,
            inputs: &[("mem.hex", b"C8\x0a41\x0aFF\x0a")],
            plus: None,
            mode: Mode::At,
            staged: true,
        },
        &[
            // M2: MOVE = iv+s2v+vl ; PRE @M2 \xc3\x88A\xc3\xbf|
            //   iv: @M2 \xc8A\xff|
            //   s2v: @M2 \xc8A\xff|
            //   vl: @M2 \xc8A\xff|
            ("M2", b"@M2 \xc8A\xff|"),
        ],
        &[],
    );
}

#[test]
fn g_c10_readmem_stays() {
    check(
        &Cell {
            src: G_C10_READMEM_SRC,
            inputs: &[("mem.hex", b"C8\x0a41\x0aFF\x0a")],
            plus: None,
            mode: Mode::At,
            staged: true,
        },
        &[
            // M1: STAY = iv+s2v+vl ; PRE @M1 c8 41 ff
            //   iv: @M1 c8 41 ff
            //   s2v: @M1 c8 41 ff
            //   vl: @M1 c8 41 ff
            ("M1", b"@M1 c8 41 ff"),
            // M3: STAY = iv+s2v+vl ; PRE @M3 c8 41 ff len=3
            //   iv: @M3 c8 41 ff len=3
            //   s2v: @M3 c8 41 ff len=3
            //   vl: @M3 c8 41 ff len=3
            ("M3", b"@M3 c8 41 ff len=3"),
        ],
        &[],
    );
}

const G_C11_STROPS_SRC: &str = r#"module top;
  task automatic hx(input string tag, input string a); $write("@%s", tag); for (int i = 0; i < a.len(); i++) $write(" %02x", a[i]); $display(" len=%0d", a.len()); endtask
  string s, t, u; byte b; int i;
  initial begin
    s = string'(16'hC841); t = "A"; u = string'(16'h7F41);
    $display("@O1 %0d %0d %0d", s > t, s < t, s > u);
    $display("@O2 %0d %0d", s == string'(16'hC841), s.compare(u) > 0);
    $display("@O3 %0d", s.len());
    b = s.getc(0); $display("@O4 %0d %h", b, b);
    i = s[0]; $display("@O5 %0d", i);
    #1 $finish;
  end
endmodule
"#;

#[test]
fn g_c11_strops_stays() {
    check(
        &Cell {
            src: G_C11_STROPS_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::At,
            staged: true,
        },
        &[
            // O1: STAY = vl ; PRE @O1 1 0 1
            //   vl: @O1 1 0 1
            ("O1", b"@O1 1 0 1"),
            // O2: STAY = vl ; PRE @O2 1 1
            //   vl: @O2 1 1
            ("O2", b"@O2 1 1"),
            // O3: STAY = vl ; PRE @O3 2
            //   vl: @O3 2
            ("O3", b"@O3 2"),
            // O4: STAY = vl ; PRE @O4 -56 c8
            //   vl: @O4 -56 c8
            ("O4", b"@O4 -56 c8"),
            // O5: STAY = vl ; PRE @O5 200
            //   vl: @O5 200
            ("O5", b"@O5 200"),
        ],
        &[],
    );
}

const G_C12_VCD_SRC: &str = r#"module top;
  task automatic hx(input string tag, input string a); $write("@%s", tag); for (int i = 0; i < a.len(); i++) $write(" %02x", a[i]); $display(" len=%0d", a.len()); endtask
  string s; reg [15:0] v;
  initial begin
    $dumpfile("w.vcd"); $dumpvars(0, top);
    s = "A"; v = 16'h4142;
    #1 s = string'(16'hC841); v = 16'hC841;
    #1 $display("@W1 done"); $finish;
  end
endmodule
"#;

#[test]
fn g_c12_vcd_stays() {
    check(
        &Cell {
            src: G_C12_VCD_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::At,
            staged: true,
        },
        &[
            // W1: STAY = iv+s2v+vl ; PRE @W1 done
            //   iv: @W1 done
            //   s2v: @W1 done
            //   vl: @W1 done
            ("W1", b"@W1 done"),
        ],
        &[],
    );
}

const G_C13_FST_SRC: &str = r#"module top;
  task automatic hx(input string tag, input string a); $write("@%s", tag); for (int i = 0; i < a.len(); i++) $write(" %02x", a[i]); $display(" len=%0d", a.len()); endtask
  string s; reg [15:0] v;
  initial begin
    $dumpfile("w.fst"); $dumpvars(0, top);
    s = "A"; v = 16'h4142;
    #1 s = string'(16'hC841); v = 16'hC841;
    #1 $display("@W1 done"); $finish;
  end
endmodule
"#;

#[test]
fn g_c13_fst_stays() {
    check(
        &Cell {
            src: G_C13_FST_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::At,
            staged: true,
        },
        &[
            // W1: STAY = iv+s2v+vl ; PRE @W1 done
            //   iv: @W1 done
            //   s2v: @W1 done
            //   vl: @W1 done
            ("W1", b"@W1 done"),
        ],
        &[],
    );
}

const G_C14_DISP_FORMS_SRC: &str = r#"module top;
  task automatic hx(input string tag, input string a); $write("@%s", tag); for (int i = 0; i < a.len(); i++) $write(" %02x", a[i]); $display(" len=%0d", a.len()); endtask
  string s;
  initial begin
    s = string'(16'hC841);
    $display("@D1 %s|", s);
    $write("@D2 %s|\n", s);
    $display("@D3 ", s, "|");
    $display("@D4 %s%s|", s, s);
    $fdisplay(32'h8000_0001, "@D5 %s|", s);
    $fwrite(1, "@D6 %s|\n", s);
    #1 $finish;
  end
endmodule
"#;

#[test]
fn g_c14_disp_forms_moves() {
    check(
        &Cell {
            src: G_C14_DISP_FORMS_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::At,
            staged: true,
        },
        &[
            // D1: MOVE = iv+s2v+vl ; PRE @D1 \xef\xbf\xbdA|
            //   iv: @D1 \xc8A|
            //   s2v: @D1 \xc8A|
            //   vl: @D1 \xc8A|
            ("D1", b"@D1 \xc8A|"),
            // D2: MOVE = iv+s2v+vl ; PRE @D2 \xef\xbf\xbdA|
            //   iv: @D2 \xc8A|
            //   s2v: @D2 \xc8A|
            //   vl: @D2 \xc8A|
            ("D2", b"@D2 \xc8A|"),
            // D3: MOVE = iv+s2v+vl ; PRE @D3 \xef\xbf\xbdA|
            //   iv: @D3 \xc8A|
            //   s2v: @D3 \xc8A|
            //   vl: @D3 \xc8A|
            ("D3", b"@D3 \xc8A|"),
            // D4: MOVE = iv+s2v+vl ; PRE @D4 \xef\xbf\xbdA\xef\xbf\xbdA|
            //   iv: @D4 \xc8A\xc8A|
            //   s2v: @D4 \xc8A\xc8A|
            //   vl: @D4 \xc8A\xc8A|
            ("D4", b"@D4 \xc8A\xc8A|"),
            // D5: MOVE = iv+s2v+vl ; PRE @D5 \xef\xbf\xbdA|
            //   iv: @D5 \xc8A|
            //   s2v: @D5 \xc8A|
            //   vl: @D5 \xc8A|
            ("D5", b"@D5 \xc8A|"),
            // D6: MOVE = iv+s2v+vl ; PRE @D6 \xef\xbf\xbdA|
            //   iv: @D6 \xc8A|
            //   s2v: @D6 \xc8A|
            //   vl: @D6 \xc8A|
            ("D6", b"@D6 \xc8A|"),
        ],
        &[],
    );
}
