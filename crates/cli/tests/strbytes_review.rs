//! Q-STRBYTES pins, GENERATED from the design-review cells (W/qstrbytes/design/cells, REPORT C1-C4) by W/qstrbytes/r1/bin/gen_pins.py.
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

const Z_Z01_STDOUT_SRC: &str = r#"module top;
  string h, e, n;
  reg [31:0] p1, p2; reg [15:0] pe; reg [23:0] p3;
  initial begin
    h = "\310A\200"; e = "\303\251"; n = $sformatf("x%cy", 8'h00);
    p1 = 32'h6100C862; p2 = 32'h00C80041; pe = 16'hC3A9; p3 = 24'h00C800;
    $display("Z01 [a\000b]");
    $display("Z02 [%c]", 8'h00);
    $display("Z03 [%c]", 8'hxx);
    $display("Z04 [%s]", p1);
    $display("Z05 [%0s]", p2);
    $display("Z06 [%5s]", pe);
    $display("Z07 [%s] %0d", n, n.len());
    $display("Z08 [%-4c]", 8'hC8);
    $display("Z09 [%s]", "\310A\200");
    $display("Z10 [%s|%5s|%-5s]", h, h, h);
    $display("Z11 [\310A\200]");
    $write("Z12 [%s]\n", h);
    $display("Z17 [%s]", p3);
    $display("Z19 [%5s]", n);
    $display("Z23 [%5s|%-5s]", e, e);
    $timeformat(0, 0, "\303\251", 6);
    #3 $display("Z14 [%t]", $time);
    #1 $finish;
  end
endmodule
"#;

#[test]
fn z_z01_stdout_moves() {
    check(
        &Cell {
            src: Z_Z01_STDOUT_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::Word,
            staged: true,
        },
        &[
            // Z04: MOVE = iv+s2v ; PRE Z04 [a \xc3\x88b]
            //   iv: Z04 [a \xc8b]
            //   s2v: Z04 [a \xc8b]
            ("Z04", b"Z04 [a \xc8b]"),
            // Z05: MOVE = iv+s2v ; PRE Z05 [\xc3\x88 A]
            //   iv: Z05 [\xc8 A]
            //   s2v: Z05 [\xc8 A]
            ("Z05", b"Z05 [\xc8 A]"),
            // Z06: MOVE = iv+s2v ; PRE Z06 [   \xc3\x83\xc2\xa9]
            //   iv: Z06 [   \xc3\xa9]
            //   s2v: Z06 [   \xc3\xa9]
            ("Z06", b"Z06 [   \xc3\xa9]"),
            // Z08: MOVE = iv+s2v ; PRE Z08 [\xc3\x88   ]
            //   iv: Z08 [\xc8   ]
            //   s2v: Z08 [\xc8   ]
            ("Z08", b"Z08 [\xc8   ]"),
            // Z09: MOVE = iv+s2v ; PRE Z09 [\xef\xbf\xbdA\xef\xbf\xbd]
            //   iv: Z09 [\xc8A\x80]
            //   s2v: Z09 [\xc8A\x80]
            ("Z09", b"Z09 [\xc8A\x80]"),
            // Z10: MOVE = iv+s2v ; PRE Z10 [\xef\xbf\xbdA\xef\xbf\xbd|  \xef\xbf\xbdA\xef\xbf\xbd|\xef\xbf\xbdA\xef\xbf\xbd  ]
            //   iv: Z10 [\xc8A\x80|  \xc8A\x80|\xc8A\x80  ]
            //   s2v: Z10 [\xc8A\x80|  \xc8A\x80|\xc8A\x80  ]
            ("Z10", b"Z10 [\xc8A\x80|  \xc8A\x80|\xc8A\x80  ]"),
            // Z11: MOVE = iv+s2v ; PRE Z11 [\xef\xbf\xbdA\xef\xbf\xbd]
            //   iv: Z11 [\xc8A\x80]
            //   s2v: Z11 [\xc8A\x80]
            ("Z11", b"Z11 [\xc8A\x80]"),
            // Z12: MOVE = iv+s2v ; PRE Z12 [\xef\xbf\xbdA\xef\xbf\xbd]
            //   iv: Z12 [\xc8A\x80]
            //   s2v: Z12 [\xc8A\x80]
            ("Z12", b"Z12 [\xc8A\x80]"),
            // Z17: MOVE = iv+s2v ; PRE Z17 [ \xc3\x88 ]
            //   iv: Z17 [ \xc8 ]
            //   s2v: Z17 [ \xc8 ]
            ("Z17", b"Z17 [ \xc8 ]"),
            // Z23: MOVE = iv+s2v ; PRE Z23 [    \xc3\xa9|\xc3\xa9    ]
            //   iv: Z23 [   \xc3\xa9|\xc3\xa9   ]
            //   s2v: Z23 [   \xc3\xa9|\xc3\xa9   ]
            ("Z23", b"Z23 [   \xc3\xa9|\xc3\xa9   ]"),
            // Z14: MOVE = HAND (z01 has a NUL step (vl unreadable): = z01b Z14 (vl, C2): %t width counts bytes (§21.2.1.8 l.28962-28968 + field width l.28763-28788), digit 0 = BASE) ; PRE Z14 [    0\xc3\xa9]
            //   iv: Z14 [   3\xc3\xa9]
            //   s2v: Z14 [   3\xc3\xa9]
            ("Z14", b"Z14 [   0\xc3\xa9]"),
        ],
        &[],
    );
}

#[test]
fn z_z01_stdout_stays() {
    check(
        &Cell {
            src: Z_Z01_STDOUT_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::Word,
            staged: true,
        },
        &[
            // Z01: STAY = PRE (NUL in a packed %s: PRE = s2v-side split, ROADMAP:399 OPEN class (REPORT N4)) ; PRE Z01 [a\x00b]
            //   iv: Z01 [a b]
            //   s2v: Z01 [a b]
            //   vl: Z01 [ab]
            ("Z01", b"Z01 [a\x00b]"),
            // Z02: STAY = PRE (NUL stays (REPORT C4 STAY)) ; PRE Z02 [\x00]
            //   iv: Z02 [\x00]
            //   s2v: Z02 [\x00]
            //   vl: Z02 [Z14 [   0\xc3\xa9]
            ("Z02", b"Z02 [\x00]"),
            // Z03: STAY = iv+s2v ; PRE Z03 [\x00]
            //   iv: Z03 [\x00]
            //   s2v: Z03 [\x00]
            ("Z03", b"Z03 [\x00]"),
            // Z07: STAY = PRE (NUL stays (P1 ; REPORT C4 STAY ; ROADMAP:399)) ; PRE Z07 [x\x00y] 3
            //   iv: Z07 [x] 1
            //   s2v: Z07 [x] 1
            ("Z07", b"Z07 [x\x00y] 3"),
            // Z19: STAY = PRE (NUL stays) ; PRE Z19 [  x\x00y]
            //   iv: Z19 [    x]
            //   s2v: Z19 [    x]
            ("Z19", b"Z19 [  x\x00y]"),
        ],
        &[],
    );
}

const Z_Z01B_NONUL_SRC: &str = r#"module top;
  string h, e;
  reg [31:0] p1, p2; reg [15:0] pe; reg [23:0] p3;
  initial begin
    h = "\310A\200"; e = "\303\251";
    p1 = 32'h6100C862; p2 = 32'h00C80041; pe = 16'hC3A9; p3 = 24'h00C800;
    $display("Z04 [%s]", p1);
    $display("Z05 [%0s]", p2);
    $display("Z06 [%5s]", pe);
    $display("Z08 [%-4c]", 8'hC8);
    $display("Z09 [%s]", "\310A\200");
    $display("Z10 [%s|%5s|%-5s]", h, h, h);
    $display("Z11 [\310A\200]");
    $write("Z12 [%s]\n", h);
    $display("Z17 [%s]", p3);
    $display("Z23 [%5s|%-5s]", e, e);
    $timeformat(0, 0, "\303\251", 6);
    #3 $display("Z14 [%t]", $time);
    #1 $finish;
  end
endmodule
"#;

#[test]
fn z_z01b_nonul_moves() {
    check(
        &Cell {
            src: Z_Z01B_NONUL_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::Word,
            staged: true,
        },
        &[
            // Z04: MOVE = iv+s2v+vl ; PRE Z04 [a \xc3\x88b]
            //   iv: Z04 [a \xc8b]
            //   s2v: Z04 [a \xc8b]
            //   vl: Z04 [a \xc8b]
            ("Z04", b"Z04 [a \xc8b]"),
            // Z05: MOVE = iv+s2v+vl ; PRE Z05 [\xc3\x88 A]
            //   iv: Z05 [\xc8 A]
            //   s2v: Z05 [\xc8 A]
            //   vl: Z05 [\xc8 A]
            ("Z05", b"Z05 [\xc8 A]"),
            // Z06: MOVE = iv+s2v+vl ; PRE Z06 [   \xc3\x83\xc2\xa9]
            //   iv: Z06 [   \xc3\xa9]
            //   s2v: Z06 [   \xc3\xa9]
            //   vl: Z06 [   \xc3\xa9]
            ("Z06", b"Z06 [   \xc3\xa9]"),
            // Z08: MOVE = iv+s2v (SPLIT %-4c pad: vita's existing %Nc rule = iv+s2v (REPORT C4)) ; PRE Z08 [\xc3\x88   ]
            //   iv: Z08 [\xc8   ]
            //   s2v: Z08 [\xc8   ]
            //   vl: Z08 [\xc8]
            ("Z08", b"Z08 [\xc8   ]"),
            // Z09: MOVE = iv+s2v+vl ; PRE Z09 [\xef\xbf\xbdA\xef\xbf\xbd]
            //   iv: Z09 [\xc8A\x80]
            //   s2v: Z09 [\xc8A\x80]
            //   vl: Z09 [\xc8A\x80]
            ("Z09", b"Z09 [\xc8A\x80]"),
            // Z10: MOVE = iv+s2v+vl ; PRE Z10 [\xef\xbf\xbdA\xef\xbf\xbd|  \xef\xbf\xbdA\xef\xbf\xbd|\xef\xbf\xbdA\xef\xbf\xbd  ]
            //   iv: Z10 [\xc8A\x80|  \xc8A\x80|\xc8A\x80  ]
            //   s2v: Z10 [\xc8A\x80|  \xc8A\x80|\xc8A\x80  ]
            //   vl: Z10 [\xc8A\x80|  \xc8A\x80|\xc8A\x80  ]
            ("Z10", b"Z10 [\xc8A\x80|  \xc8A\x80|\xc8A\x80  ]"),
            // Z11: MOVE = iv+s2v+vl ; PRE Z11 [\xef\xbf\xbdA\xef\xbf\xbd]
            //   iv: Z11 [\xc8A\x80]
            //   s2v: Z11 [\xc8A\x80]
            //   vl: Z11 [\xc8A\x80]
            ("Z11", b"Z11 [\xc8A\x80]"),
            // Z12: MOVE = iv+s2v+vl ; PRE Z12 [\xef\xbf\xbdA\xef\xbf\xbd]
            //   iv: Z12 [\xc8A\x80]
            //   s2v: Z12 [\xc8A\x80]
            //   vl: Z12 [\xc8A\x80]
            ("Z12", b"Z12 [\xc8A\x80]"),
            // Z17: MOVE = iv+s2v+vl ; PRE Z17 [ \xc3\x88 ]
            //   iv: Z17 [ \xc8 ]
            //   s2v: Z17 [ \xc8 ]
            //   vl: Z17 [ \xc8 ]
            ("Z17", b"Z17 [ \xc8 ]"),
            // Z23: MOVE = iv+s2v+vl ; PRE Z23 [    \xc3\xa9|\xc3\xa9    ]
            //   iv: Z23 [   \xc3\xa9|\xc3\xa9   ]
            //   s2v: Z23 [   \xc3\xa9|\xc3\xa9   ]
            //   vl: Z23 [   \xc3\xa9|\xc3\xa9   ]
            ("Z23", b"Z23 [   \xc3\xa9|\xc3\xa9   ]"),
            // Z14: MOVE = vl (C2: %t width counts bytes (all 3) ; digit 0 = vl = BASE (REPORT C2)) ; PRE Z14 [    0\xc3\xa9]
            //   iv: Z14 [   3\xc3\xa9]
            //   s2v: Z14 [   3\xc3\xa9]
            //   vl: Z14 [   0\xc3\xa9]
            ("Z14", b"Z14 [   0\xc3\xa9]"),
        ],
        &[],
    );
}

const Z_Z01C_NUL_SRC: &str = r#"module top;
  string n;
  initial begin
    n = $sformatf("x%cy", 8'h00);
    #1 $display("Z01 [a\000b]");
    #1 $display("Z02 [%c]", 8'h00);
    #1 $display("Z03 [%c]", 8'hxx);
    #1 $display("Z07 [%s] %0d", n, n.len());
    #1 $display("Z19 [%5s]", n);
    #1 $display("Z20 [%s]", "a\000b");
    #1 $display("Z21 [%s]", 24'h610062);
    #1 $finish;
  end
endmodule
"#;

#[test]
fn z_z01c_nul_stays() {
    check(
        &Cell {
            src: Z_Z01C_NUL_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::Word,
            staged: true,
        },
        &[
            // Z01: STAY = PRE (as z01 Z01) ; PRE Z01 [a\x00b]
            //   iv: Z01 [a b]
            //   s2v: Z01 [a b]
            //   vl: Z01 [ab]
            ("Z01", b"Z01 [a\x00b]"),
            // Z02: STAY = PRE (as z01 Z02) ; PRE Z02 [\x00]
            //   iv: Z02 [\x00]
            //   s2v: Z02 [\x00]
            //   vl: Z02 [Z03 [Z07 [xZ19 [  xZ20 [a b]
            ("Z02", b"Z02 [\x00]"),
            // Z03: STAY = iv+s2v ; PRE Z03 [\x00]
            //   iv: Z03 [\x00]
            //   s2v: Z03 [\x00]
            ("Z03", b"Z03 [\x00]"),
            // Z07: STAY = PRE (NUL stays) ; PRE Z07 [x\x00y] 3
            //   iv: Z07 [x] 1
            //   s2v: Z07 [x] 1
            ("Z07", b"Z07 [x\x00y] 3"),
            // Z19: STAY = PRE (NUL stays) ; PRE Z19 [  x\x00y]
            //   iv: Z19 [    x]
            //   s2v: Z19 [    x]
            ("Z19", b"Z19 [  x\x00y]"),
            // Z20: STAY = PRE (NUL stays (N4)) ; PRE Z20 [a\x00b]
            //   iv: Z20 [a b]
            //   s2v: Z20 [a b]
            ("Z20", b"Z20 [a\x00b]"),
            // Z21: STAY = iv+s2v ; PRE Z21 [a b]
            //   iv: Z21 [a b]
            //   s2v: Z21 [a b]
            //   vl: Z21 [a b]
            ("Z21", b"Z21 [a b]"),
        ],
        &[],
    );
}

const Z_Z02_FILES_SRC: &str = r#"module top;
  string h, e; integer fd, mcd; reg [31:0] p1;
  initial begin
    h = "\310A\200"; e = "\303\251"; p1 = 32'h6100C862;
    fd = $fopen("fo.bin", "w");
    $fwrite(fd, "F1 [\310A\200|%s|%c|%5s]\n", h, 8'hC8, e);
    $fwrite(fd, "F2 [a%cb]\n", 8'h00);
    $fdisplay(fd, "F3 [%s]", p1);
    $fclose(fd);
    mcd = $fopen("mc.bin");
    $fdisplay(mcd | 1, "F4 [%s|%c|%5s]", h, 8'h00, e);
    $fclose(mcd);
    $fwrite(32'h8000_0002, "F5 [%s|%c|%5s]\n", h, 8'h00, e);
    $fwrite(32'h8000_0001, "F6 [%s|%c|%5s]\n", h, 8'h00, e);
    #1 $finish;
  end
endmodule
"#;

#[test]
fn z_z02_files_moves() {
    check(
        &Cell {
            src: Z_Z02_FILES_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::Word,
            staged: true,
        },
        &[
            // F4: MOVE = iv+s2v+vl ; PRE F4 [\xef\xbf\xbdA\xef\xbf\xbd|\x00|    \xc3\xa9]
            //   iv: F4 [\xc8A\x80|\x00|   \xc3\xa9]
            //   s2v: F4 [\xc8A\x80|\x00|   \xc3\xa9]
            //   vl: F4 [\xc8A\x80|\x00|   \xc3\xa9]
            ("F4", b"F4 [\xc8A\x80|\x00|   \xc3\xa9]"),
            // F5: MOVE = iv+s2v+vl ; PRE F5 [\xef\xbf\xbdA\xef\xbf\xbd|\x00|    \xc3\xa9]
            //   iv: F5 [\xc8A\x80|\x00|   \xc3\xa9]
            //   s2v: F5 [\xc8A\x80|\x00|   \xc3\xa9]
            //   vl: F5 [\xc8A\x80|\x00|   \xc3\xa9]
            ("F5", b"F5 [\xc8A\x80|\x00|   \xc3\xa9]"),
            // F6: MOVE = iv+s2v+vl ; PRE F6 [\xef\xbf\xbdA\xef\xbf\xbd|\x00|    \xc3\xa9]
            //   iv: F6 [\xc8A\x80|\x00|   \xc3\xa9]
            //   s2v: F6 [\xc8A\x80|\x00|   \xc3\xa9]
            //   vl: F6 [\xc8A\x80|\x00|   \xc3\xa9]
            ("F6", b"F6 [\xc8A\x80|\x00|   \xc3\xa9]"),
        ],
        &[
            // FILE:fo.bin: MOVE = iv+s2v+vl ; PRE F1 [\xef\xbf\xbdA\xef\xbf\xbd|\xef\xbf\xbdA\xef\xbf\xbd|\xc3\x88|    \xc3\xa9]\x0aF2 [a\x00b]\x0aF3 [a \xc3\x88b]\x0a
            (
                "fo.bin",
                b"F1 [\xc8A\x80|\xc8A\x80|\xc8|   \xc3\xa9]\x0aF2 [a\x00b]\x0aF3 [a \xc8b]\x0a",
            ),
            // FILE:mc.bin: MOVE = iv+s2v+vl ; PRE F4 [\xef\xbf\xbdA\xef\xbf\xbd|\x00|    \xc3\xa9]\x0a
            ("mc.bin", b"F4 [\xc8A\x80|\x00|   \xc3\xa9]\x0a"),
        ],
    );
}

const Z_Z03_SEV_SRC: &str = r#"module top;
  string h, e; reg [31:0] p1;
  initial begin
    h = "\310A\200"; e = "\303\251"; p1 = 32'h6100C862;
    $info("V1 [%s|%c|%5s]", h, 8'hC8, e);
    $warning("V2 [a%cb]", 8'h00);
    $error("V3 [%s]", p1);
    $info("V4 [\310A\200]");
    #1 $finish;
  end
endmodule
"#;

#[test]
fn z_z03_sev_moves() {
    check(
        &Cell {
            src: Z_Z03_SEV_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::Word,
            staged: true,
        },
        &[
            // V1: MOVE = iv+s2v ; PRE V1 [\xef\xbf\xbdA\xef\xbf\xbd|\xc3\x88|    \xc3\xa9]
            //   iv: V1 [\xc8A\x80|\xc8|   \xc3\xa9]
            //   s2v: V1 [\xc8A\x80|\xc8|   \xc3\xa9]
            //   vl: V1 [\xc8A\x80|\xc8|   \xc3\xa9]
            ("V1", b"V1 [\xc8A\x80|\xc8|   \xc3\xa9]"),
            // V3: MOVE = iv+s2v ; PRE V3 [a \xc3\x88b]
            //   iv: V3 [a \xc8b]
            //   s2v: V3 [a \xc8b]
            ("V3", b"V3 [a \xc8b]"),
            // V4: MOVE = iv+s2v ; PRE V4 [\xef\xbf\xbdA\xef\xbf\xbd]
            //   iv: V4 [\xc8A\x80]
            //   s2v: V4 [\xc8A\x80]
            ("V4", b"V4 [\xc8A\x80]"),
        ],
        &[],
    );
}

#[test]
fn z_z03_sev_stays() {
    check(
        &Cell {
            src: Z_Z03_SEV_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::Word,
            staged: true,
        },
        &[
            // V2: STAY = PRE (NUL in a severity message: PRE `a\x00b` = s2v (REPORT C4 STAY)) ; PRE V2 [a\x00b]
            //   iv: V2 [ab]
            //   s2v: V2 [a\x00b]
            //   vl: V2 [a[0] %Error: t.sv:7: Assertion failed in top: V3 [a \xc8b]
            ("V2", b"V2 [a\x00b]"),
        ],
        &[],
    );
}

const Z_Z04_VALUES_SRC: &str = r#"module top;
  task automatic hx(input string tag, input string a); $write("%s", tag); for (int i = 0; i < a.len(); i++) $write(" %02x", a[i]); $display(" len=%0d", a.len()); endtask
  string h, e, u; reg [31:0] p1; reg [7:0] c9;
  initial begin
    h = "\310A\200"; e = "\303\251"; p1 = 32'h6100C862; c9 = 8'hC9;
    u = $sformatf("%s|%c|%5s", h, 8'hC8, e); hx("X1", u);
    u = $sformatf("a%cb", 8'h00); hx("X2", u);
    u = $sformatf("%s", p1); hx("X3", u);
    $sformat(u, "%s|%c|%5s", h, 8'h00, e); hx("X4", u);
    $swrite(u, "%s|%c", h, 8'hC8); hx("X5", u);
    u = {h, "!"}; hx("X6", u);
    u = {"\310", c9, "A"}; hx("X7", u);
    u = $sformatf("\310%0d\200", 5); hx("X8", u);
    #1 $finish;
  end
endmodule
"#;

#[test]
fn z_z04_values_moves() {
    check(
        &Cell {
            src: Z_Z04_VALUES_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::Word,
            staged: true,
        },
        &[
            // X1: MOVE = iv+s2v+vl ; PRE X1 ef bf bd 41 ef bf bd 7c c3 88 7c 20 20 20 20 c3 a9 len=17
            //   iv: X1 c8 41 80 7c c8 7c 20 20 20 c3 a9 len=11
            //   s2v: X1 c8 41 80 7c c8 7c 20 20 20 c3 a9 len=11
            //   vl: X1 c8 41 80 7c c8 7c 20 20 20 c3 a9 len=11
            ("X1", b"X1 c8 41 80 7c c8 7c 20 20 20 c3 a9 len=11"),
            // X3: MOVE = iv+s2v+vl ; PRE X3 61 20 c3 88 62 len=5
            //   iv: X3 61 20 c8 62 len=4
            //   s2v: X3 61 20 c8 62 len=4
            //   vl: X3 61 20 c8 62 len=4
            ("X3", b"X3 61 20 c8 62 len=4"),
            // X4: MOVE = vl ($sformat with %c 0: iv truncates at NUL ; -> vl (REPORT C4)) ; PRE X4 ef bf bd 41 ef bf bd 7c 00 7c 20 20 20 20 c3 a9 len=16
            //   iv: X4 c8 41 80 7c len=4
            //   s2v: X4 c8 41 80 7c len=4
            //   vl: X4 c8 41 80 7c 00 7c 20 20 20 c3 a9 len=11
            ("X4", b"X4 c8 41 80 7c 00 7c 20 20 20 c3 a9 len=11"),
            // X5: MOVE = iv+s2v+vl ; PRE X5 ef bf bd 41 ef bf bd 7c c3 88 len=10
            //   iv: X5 c8 41 80 7c c8 len=5
            //   s2v: X5 c8 41 80 7c c8 len=5
            //   vl: X5 c8 41 80 7c c8 len=5
            ("X5", b"X5 c8 41 80 7c c8 len=5"),
            // X6: MOVE = iv+s2v+vl ; PRE X6 ef bf bd 41 ef bf bd 21 len=8
            //   iv: X6 c8 41 80 21 len=4
            //   s2v: X6 c8 41 80 21 len=4
            //   vl: X6 c8 41 80 21 len=4
            ("X6", b"X6 c8 41 80 21 len=4"),
            // X8: MOVE = iv+s2v+vl ; PRE X8 ef bf bd 35 ef bf bd len=7
            //   iv: X8 c8 35 80 len=3
            //   s2v: X8 c8 35 80 len=3
            //   vl: X8 c8 35 80 len=3
            ("X8", b"X8 c8 35 80 len=3"),
        ],
        &[],
    );
}

#[test]
fn z_z04_values_stays() {
    check(
        &Cell {
            src: Z_Z04_VALUES_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::Word,
            staged: true,
        },
        &[
            // X2: STAY = vl (NUL in $sformatf: iv truncates at NUL (REPORT N3) ; PRE = vl STAY) ; PRE X2 61 00 62 len=3
            //   iv: X2 61 len=1
            //   s2v: X2 61 len=1
            //   vl: X2 61 00 62 len=3
            ("X2", b"X2 61 00 62 len=3"),
            // X7: STAY = iv+s2v+vl ; PRE X7 c8 c9 41 len=3
            //   iv: X7 c8 c9 41 len=3
            //   s2v: X7 c8 c9 41 len=3
            //   vl: X7 c8 c9 41 len=3
            ("X7", b"X7 c8 c9 41 len=3"),
        ],
        &[],
    );
}

const Z_Z05_SCAN_SRC: &str = r#"module top;
  string src, x; int n;
  initial begin
    x = "-"; src = "\310A\200zz"; n = $sscanf(src, "\310A\200%s", x); $display("Y1 %0d [%s]", n, x);
    x = "-"; src = "ab";  n = $sscanf(src, "a\000%s", x); $display("Y2 %0d [%s]", n, x);
    x = "-"; src = "\303\251q"; n = $sscanf(src, "\303\251%s", x); $display("Y3 %0d [%s]", n, x);
    x = "-"; src = "Aq"; n = $sscanf(src, "\310%s", x); $display("Y4 %0d [%s]", n, x);
    #1 $finish;
  end
endmodule
"#;

#[test]
fn z_z05_scan_moves() {
    check(
        &Cell {
            src: Z_Z05_SCAN_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::Word,
            staged: true,
        },
        &[
            // Y1: MOVE = iv+s2v+vl ; PRE Y1 0 [-]
            //   iv: Y1 1 [zz]
            //   s2v: Y1 1 [zz]
            //   vl: Y1 1 [zz]
            ("Y1", b"Y1 1 [zz]"),
        ],
        &[],
    );
}

#[test]
fn z_z05_scan_stays() {
    check(
        &Cell {
            src: Z_Z05_SCAN_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::Word,
            staged: true,
        },
        &[
            // Y2: STAY = vl (NUL in a scan format: PRE = vl `0` (iv = s2v `1 [b]`) STAY (REPORT C4)) ; PRE Y2 0 [-]
            //   iv: Y2 1 [b]
            //   s2v: Y2 1 [b]
            //   vl: Y2 0 [-]
            ("Y2", b"Y2 0 [-]"),
            // Y3: STAY = iv+s2v+vl ; PRE Y3 1 [q]
            //   iv: Y3 1 [q]
            //   s2v: Y3 1 [q]
            //   vl: Y3 1 [q]
            ("Y3", b"Y3 1 [q]"),
            // Y4: STAY = iv+s2v+vl ; PRE Y4 0 [-]
            //   iv: Y4 0 [-]
            //   s2v: Y4 0 [-]
            //   vl: Y4 0 [-]
            ("Y4", b"Y4 0 [-]"),
        ],
        &[],
    );
}

const Z_Z06_ELAB_SRC: &str = r#"module top;
  parameter string P = "\310A\200";
  $info("I5 [%5s]", "\303\251");
  $info("I6 [a%cb]", 8'h00);
  $info("I7 [%s|%c]", P, 8'hC8);
  $info("I8 [%-4c]", 8'hC8);
  initial #1 $finish;
endmodule
"#;

#[test]
fn z_z06_elab_moves() {
    check(
        &Cell {
            src: Z_Z06_ELAB_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::Word,
            staged: true,
        },
        &[
            // I5: MOVE = s2v+vl ; PRE I5 [    \xc3\xa9]
            //   s2v: I5 [   \xc3\xa9]
            //   vl: I5 [   \xc3\xa9]
            ("I5", b"I5 [   \xc3\xa9]"),
            // I7: MOVE = s2v+vl ; PRE I7 [\xef\xbf\xbdA\xef\xbf\xbd|\xc3\x88]
            //   s2v: I7 [\xc8A\x80|\xc8]
            //   vl: I7 [\xc8A\x80|\xc8]
            ("I7", b"I7 [\xc8A\x80|\xc8]"),
            // I8: MOVE = s2v (REPORT C4 named I8 STAY for its %-4c pad split ; the c8 byte (PRE c3 88) moves to s2v = vita's existing %Nc pad rule (as c22 J3, z01b Z08)) ; PRE I8 [\xc3\x88   ]
            //   s2v: I8 [\xc8   ]
            //   vl: I8 [\xc8]
            ("I8", b"I8 [\xc8   ]"),
        ],
        &[],
    );
}

#[test]
fn z_z06_elab_stays() {
    check(
        &Cell {
            src: Z_Z06_ELAB_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::Word,
            staged: true,
        },
        &[
            // I6: STAY = s2v+vl ; PRE I6 [a\x00b]
            //   s2v: I6 [a\x00b]
            //   vl: I6 [a\x00b]
            ("I6", b"I6 [a\x00b]"),
        ],
        &[],
    );
}

const Z_Z07_FRAMES_SRC: &str = r#"module top;
  string h, e;
  function automatic void fdisp(); $display("W1 [%5s|%c|%s]", e, 8'h00, h); endfunction
  task automatic tdisp(); $display("W2 [%5s|%c|%s]", e, 8'hC8, h); endtask
  function automatic int ferr(); $error("W3 [%5s|%s]", e, h); return 0; endfunction
  function automatic string fsf(); return $sformatf("%5s|%c|%s", e, 8'hC8, h); endfunction
  task automatic hx(input string tag, input string a); $write("%s", tag); for (int i = 0; i < a.len(); i++) $write(" %02x", a[i]); $display(" len=%0d", a.len()); endtask
  int r; string u;
  initial begin
    h = "\310A\200"; e = "\303\251";
    fdisp(); tdisp(); r = ferr();
    u = fsf(); hx("W7", u);
    $strobe("W4 [%s|%c|%5s]", h, 8'h00, e);
    #1 $monitor("W6 [%s|%5s]", h, e);
    #1 $finish;
  end
endmodule
"#;

#[test]
fn z_z07_frames_moves() {
    check(
        &Cell {
            src: Z_Z07_FRAMES_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::Word,
            staged: true,
        },
        &[
            // W1: MOVE = HAND (NOVL (NUL); iv aborts: %5s pads by bytes (§21.2.1.8), %c 0 = raw 00 (BASE), %s raw bytes = z07a W1 shape (vl)) ; PRE W1 [    \xc3\xa9|\x00|\xef\xbf\xbdA\xef\xbf\xbd]
            //   vl: W1 [   \xc3\xa9|W2 [   \xc3\xa9|\xc8|\xc8A\x80]
            ("W1", b"W1 [   \xc3\xa9|\x00|\xc8A\x80]"),
            // W2: MOVE = HAND (NOVL; = z07a W2 (vl) value) ; PRE W2 [    \xc3\xa9|\xc3\x88|\xef\xbf\xbdA\xef\xbf\xbd]
            ("W2", b"W2 [   \xc3\xa9|\xc8|\xc8A\x80]"),
            // W3: MOVE = HAND (NOVL; = z07b W3 (iv = s2v = vl) value) ; PRE W3 [    \xc3\xa9|\xef\xbf\xbdA\xef\xbf\xbd]
            //   vl: W3 [   \xc3\xa9|\xc8A\x80]
            ("W3", b"W3 [   \xc3\xa9|\xc8A\x80]"),
            // W7: MOVE = HAND (NOVL; = z07a W7 (vl) value) ; PRE W7 20 20 20 20 c3 a9 7c c3 88 7c ef bf bd 41 ef bf bd len=17
            ("W7", b"W7 20 20 20 c3 a9 7c c8 7c c8 41 80 len=11"),
            // W4: MOVE = HAND (NOVL; %s raw, %c 0 raw 00, %5s pads by bytes (§21.2.1.8) = z07a W4 (vl) shape) ; PRE W4 [\xef\xbf\xbdA\xef\xbf\xbd|\x00|    \xc3\xa9]
            ("W4", b"W4 [\xc8A\x80|\x00|   \xc3\xa9]"),
            // W6: MOVE = HAND (no oracle line ($monitor): %s raw + %5s pads by bytes (§21.2.1.8) = W4 value form) ; PRE W6 [\xef\xbf\xbdA\xef\xbf\xbd|    \xc3\xa9]
            ("W6", b"W6 [\xc8A\x80|   \xc3\xa9]"),
        ],
        &[],
    );
}

const Z_Z07A_FRAMES_SRC: &str = r#"module top;
  string h, e;
  function automatic void fdisp(); $display("W1 [%5s|%c|%s]", e, 8'hC8, h); endfunction
  task automatic tdisp(); $display("W2 [%5s|%c|%s]", e, 8'hC8, h); endtask
  function automatic string fsf(); return $sformatf("%5s|%c|%s", e, 8'hC8, h); endfunction
  task automatic hx(input string tag, input string a); $write("%s", tag); for (int i = 0; i < a.len(); i++) $write(" %02x", a[i]); $display(" len=%0d", a.len()); endtask
  string u;
  initial begin
    h = "\310A\200"; e = "\303\251";
    fdisp(); tdisp();
    u = fsf(); hx("W7", u);
    $strobe("W4 [%s|%c|%5s]", h, 8'hC8, e);
    #1 $monitor("W6 [%s|%5s]", h, e);
    #1 $finish;
  end
endmodule
"#;

#[test]
fn z_z07a_frames_moves() {
    check(
        &Cell {
            src: Z_Z07A_FRAMES_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::Word,
            staged: true,
        },
        &[
            // W1: MOVE = vl ; PRE W1 [    \xc3\xa9|\xc3\x88|\xef\xbf\xbdA\xef\xbf\xbd]
            //   vl: W1 [   \xc3\xa9|\xc8|\xc8A\x80]
            ("W1", b"W1 [   \xc3\xa9|\xc8|\xc8A\x80]"),
            // W2: MOVE = vl ; PRE W2 [    \xc3\xa9|\xc3\x88|\xef\xbf\xbdA\xef\xbf\xbd]
            //   vl: W2 [   \xc3\xa9|\xc8|\xc8A\x80]
            ("W2", b"W2 [   \xc3\xa9|\xc8|\xc8A\x80]"),
            // W7: MOVE = vl ; PRE W7 20 20 20 20 c3 a9 7c c3 88 7c ef bf bd 41 ef bf bd len=17
            //   vl: W7 20 20 20 c3 a9 7c c8 7c c8 41 80 len=11
            ("W7", b"W7 20 20 20 c3 a9 7c c8 7c c8 41 80 len=11"),
            // W4: MOVE = vl ; PRE W4 [\xef\xbf\xbdA\xef\xbf\xbd|\xc3\x88|    \xc3\xa9]
            //   vl: W4 [\xc8A\x80|\xc8|   \xc3\xa9]
            ("W4", b"W4 [\xc8A\x80|\xc8|   \xc3\xa9]"),
            // W6: MOVE = HAND (no oracle line ($monitor, vl prints none): as z07 W6) ; PRE W6 [\xef\xbf\xbdA\xef\xbf\xbd|    \xc3\xa9]
            ("W6", b"W6 [\xc8A\x80|   \xc3\xa9]"),
        ],
        &[],
    );
}

const Z_Z07B_FERR_SRC: &str = r#"module top;
  string h, e; int r;
  function automatic int ferr(); $error("W3 [%5s|%s|%c]", e, h, 8'hC8); return 0; endfunction
  initial begin
    h = "\310A\200"; e = "\303\251";
    r = ferr();
    #1 $finish;
  end
endmodule
"#;

#[test]
fn z_z07b_ferr_moves() {
    check(
        &Cell {
            src: Z_Z07B_FERR_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::Word,
            staged: true,
        },
        &[
            // W3: MOVE = iv+s2v+vl ; PRE W3 [    \xc3\xa9|\xef\xbf\xbdA\xef\xbf\xbd|\xc3\x88]
            //   iv: W3 [   \xc3\xa9|\xc8A\x80|\xc8]
            //   s2v: W3 [   \xc3\xa9|\xc8A\x80|\xc8]
            //   vl: W3 [   \xc3\xa9|\xc8A\x80|\xc8]
            ("W3", b"W3 [   \xc3\xa9|\xc8A\x80|\xc8]"),
        ],
        &[],
    );
}

const Z_Z08_DEFER_SRC: &str = r#"module top;
  string h, e; reg a;
  initial begin
    h = "\310A\200"; e = "\303\251"; a = 0;
    #1;
    assert #0 (a) else $display("D1 [%s|%c|%5s]", h, 8'h00, e);
    assert #0 (a) else $error("D2 [%s|%5s]", h, e);
    #1 $finish;
  end
endmodule
"#;

#[test]
fn z_z08_defer_moves() {
    check(
        &Cell {
            src: Z_Z08_DEFER_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::Word,
            staged: true,
        },
        &[
            // D1: MOVE = HAND (NOVL (NUL); = z08b D1 (vl) shape with %c 0 raw 00) ; PRE D1 [\xef\xbf\xbdA\xef\xbf\xbd|\x00|    \xc3\xa9]
            //   vl: D1 [\xc8A\x80|[1] %Error: t.sv:7: Assertion failed in top: D2 [\xc8A\x80|   \xc3\xa9]
            ("D1", b"D1 [\xc8A\x80|\x00|   \xc3\xa9]"),
            // D2: MOVE = HAND (NOVL; = z08b D2 (vl) value) ; PRE D2 [\xef\xbf\xbdA\xef\xbf\xbd|    \xc3\xa9]
            ("D2", b"D2 [\xc8A\x80|   \xc3\xa9]"),
        ],
        &[],
    );
}

const Z_Z08B_DEFER_SRC: &str = r#"module top;
  string h, e; reg a;
  initial begin
    h = "\310A\200"; e = "\303\251"; a = 0;
    #1;
    assert #0 (a) else $display("D1 [%s|%c|%5s]", h, 8'hC8, e);
    #1;
    assert #0 (a) else $error("D2 [%s|%5s|%c]", h, e, 8'hC8);
    #1 $finish;
  end
endmodule
"#;

#[test]
fn z_z08b_defer_moves() {
    check(
        &Cell {
            src: Z_Z08B_DEFER_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::Word,
            staged: true,
        },
        &[
            // D1: MOVE = vl ; PRE D1 [\xef\xbf\xbdA\xef\xbf\xbd|\xc3\x88|    \xc3\xa9]
            //   vl: D1 [\xc8A\x80|\xc8|   \xc3\xa9]
            ("D1", b"D1 [\xc8A\x80|\xc8|   \xc3\xa9]"),
            // D2: MOVE = vl ; PRE D2 [\xef\xbf\xbdA\xef\xbf\xbd|    \xc3\xa9|\xc3\x88]
            //   vl: D2 [\xc8A\x80|   \xc3\xa9|\xc8]
            ("D2", b"D2 [\xc8A\x80|   \xc3\xa9|\xc8]"),
        ],
        &[],
    );
}

const Z_Z09_PLUS_SRC: &str = r#"module top;
  string x;
  initial begin
    $display("U1 %0d", $test$plusargs("\310"));
    x = "-"; if ($value$plusargs("\310=%s", x)) $display("U2 1 [%s]", x); else $display("U2 0 [%s]", x);
    $display("U3 %0d", $test$plusargs("\357\277\275"));
    #1 $finish;
  end
endmodule
"#;

#[test]
fn z_z09_plus_moves() {
    check(
        &Cell {
            src: Z_Z09_PLUS_SRC,
            inputs: &[],
            plus: Some(b"+\xef\xbf\xbd=ab"),
            mode: Mode::Word,
            staged: true,
        },
        &[
            // U1: MOVE = iv+s2v+vl ; PRE U1 1
            //   iv: U1 0
            //   s2v: U1 0
            //   vl: U1 0
            ("U1", b"U1 0"),
            // U2: MOVE = iv+s2v+vl ; PRE U2 1 [ab]
            //   iv: U2 0 [-]
            //   s2v: U2 0 [-]
            //   vl: U2 0 [-]
            ("U2", b"U2 0 [-]"),
        ],
        &[],
    );
}

#[test]
fn z_z09_plus_stays() {
    check(
        &Cell {
            src: Z_Z09_PLUS_SRC,
            inputs: &[],
            plus: Some(b"+\xef\xbf\xbd=ab"),
            mode: Mode::Word,
            staged: true,
        },
        &[
            // U3: STAY = iv+s2v+vl ; PRE U3 1
            //   iv: U3 1
            //   s2v: U3 1
            //   vl: U3 1
            ("U3", b"U3 1"),
        ],
        &[],
    );
}

const Z_Z10_PCTP_SRC: &str = r#"module top;
  string e, h; string q[$]; int m[string];
  initial begin
    e = "\303\251"; h = "\310A";
    q.push_back(h); q.push_back(e);
    m[h] = 5; m[e] = 6;
    $display("P1 [%8p]", e);
    $display("P2 [%p]", q);
    $display("P3 [%30p]", m);
    $display("P4 [%-8p]", e);
    #1 $finish;
  end
endmodule
"#;

#[test]
fn z_z10_pctp_moves() {
    check(
        &Cell {
            src: Z_Z10_PCTP_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::Word,
            staged: true,
        },
        &[
            // P1: MOVE = HAND (C4 no-oracle: %8p of "\303\251" pads by bytes (§21.2.1.8 l.28962-28968: 8 bits per character) ; vl ignores %p width (N3)) ; PRE P1 [     "\xc3\xa9"]
            //   vl: P1 ["\xc3\xa9"]
            ("P1", b"P1 [    \"\xc3\xa9\"]"),
            // P2: MOVE = vl ; PRE P2 ['{"\xef\xbf\xbdA", "\xc3\xa9"}]
            //   vl: P2 ['{"\xc8A", "\xc3\xa9"}]
            ("P2", b"P2 ['{\"\xc8A\", \"\xc3\xa9\"}]"),
            // P3: MOVE = HAND (vl content `'{"\xc3\xa9":'h6, "\xc8A":'h5}`, %30p pad counted in bytes (§21.2.1.8) ; vl ignores %p width (N3)) ; PRE P3 [          '{"\xc3\xa9":'h6, "\xef\xbf\xbdA":'h5}]
            //   vl: P3 ['{"\xc3\xa9":'h6, "\xc8A":'h5}]
            ("P3", b"P3 [         '{\"\xc3\xa9\":'h6, \"\xc8A\":'h5}]"),
            // P4: MOVE = HAND (C4 no-oracle: %-8p as P1, left-justified) ; PRE P4 ["\xc3\xa9"     ]
            //   vl: P4 ["\xc3\xa9"]
            ("P4", b"P4 [\"\xc3\xa9\"    ]"),
        ],
        &[],
    );
}

const Z_Z11_ESCID_SRC: &str = r#"module top;
  initial begin : \bé 
    $display("M1 [%12m]");
    $display("M2 [%-12m|]");
    #1 $finish;
  end
endmodule
"#;

#[test]
fn z_z11_escid_moves() {
    check(
        &Cell {
            src: Z_Z11_ESCID_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::Word,
            staged: true,
        },
        &[
            // M1: MOVE = HAND (C4 no-oracle: %12m pads by bytes (= iv byte count, §21.2.1.8) ; `\` content = Q-ESCSCOPE (N1, not this row)) ; PRE M1 [     top.\x5cb\xc3\xa9]
            //   iv: M1 [     top.b\xc3\xa9]
            //   s2v: M1 [     top.b\xc3\xa9]
            ("M1", b"M1 [    top.\\b\xc3\xa9]"),
            // M2: MOVE = HAND (C4 no-oracle: %-12m as M1) ; PRE M2 [top.\x5cb\xc3\xa9     |]
            //   iv: M2 [top.b\xc3\xa9     |]
            //   s2v: M2 [top.b\xc3\xa9     |]
            ("M2", b"M2 [top.\\b\xc3\xa9    |]"),
        ],
        &[],
    );
}

const Z_Z12_ATOI_SRC: &str = r#"module top;
  localparam string S1 = " 3";
  localparam string S2 = "\302\2407";
  localparam int A1 = S1.atoi();
  localparam int A2 = S2.atoi();
  string s1, s2;
  initial begin
    s1 = " 3"; s2 = "\302\2407";
    $display("K1 %0d %0d", A1, s1.atoi());
    $display("K2 %0d %0d", A2, s2.atoi());
    #1 $finish;
  end
endmodule
"#;

#[test]
fn z_z12_atoi_stays() {
    check(
        &Cell {
            src: Z_Z12_ATOI_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::Word,
            staged: true,
        },
        &[
            // K1: STAY = iv (runtime atoi of " 3": vita = iv 0, vl 3 ; not this row (Q-ATOREAL-WS)) ; PRE K1 3 0
            //   iv: K1 3 0
            //   vl: K1 3 3
            ("K1", b"K1 3 0"),
            // K2: STAY = PRE (C3: elaborate const_str.rs:341 = B18 (Q-ATOREAL-WS), byte-identical here ; PRE 7, iv 0, vl 0) ; PRE K2 7 0
            //   iv: K2 0 0
            //   vl: K2 0 0
            ("K2", b"K2 7 0"),
        ],
        &[],
    );
}

const Z_Z13_UNKSPEC_SRC: &str = r#"module top;
  initial begin
    $display("Q1 [%\303\251]");
    $display("Q2 [%\310]");
    $display("Q3 [%5\310]");
    #1 $finish;
  end
endmodule
"#;

#[test]
fn z_z13_unkspec_moves() {
    check(
        &Cell {
            src: Z_Z13_UNKSPEC_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::Word,
            staged: true,
        },
        &[
            // Q2: MOVE = HAND (C4 no-oracle: unknown spec byte \310 echoed raw (one byte = one character, §21.2.1.8) ; vita's unknown-spec rule = BASE ; iv `<%..>` (N3), vl compile error) ; PRE Q2 [%\xef\xbf\xbd]
            //   iv: Q2 [<%\xc8>]
            //   s2v: Q2 [<%\xc8>]
            ("Q2", b"Q2 [%\xc8]"),
            // Q3: MOVE = HAND (C4 no-oracle: as Q2 (BASE drops the width on an unknown spec)) ; PRE Q3 [%\xef\xbf\xbd]
            //   iv: Q3 [<%5\xc8>]
            //   s2v: Q3 [<%5\xc8>]
            ("Q3", b"Q3 [%\xc8]"),
        ],
        &[],
    );
}

#[test]
fn z_z13_unkspec_stays() {
    check(
        &Cell {
            src: Z_Z13_UNKSPEC_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::Word,
            staged: true,
        },
        &[
            // Q1: STAY = PRE (REPORT C4 STAY: valid UTF-8 spec byte pair, PRE = POST) ; PRE Q1 [%\xc3\xa9]
            //   iv: Q1 [<%\xc3>\xa9]
            //   s2v: Q1 [<%\xc3>\xa9]
            ("Q1", b"Q1 [%\xc3\xa9]"),
        ],
        &[],
    );
}

const Z_Z14_ESCASCII_SRC: &str = r#"module top;
  initial begin : \blk+1 
    $display("M3 [%m]");
    #1 $finish;
  end
endmodule
"#;

#[test]
fn z_z14_escascii_stays() {
    check(
        &Cell {
            src: Z_Z14_ESCASCII_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::Word,
            staged: true,
        },
        &[
            // M3: STAY = PRE (NOT THIS ROW: Q-ESCSCOPE (N1)) ; PRE M3 [top.\x5cblk+1]
            //   iv: M3 [top.blk+1]
            //   s2v: M3 [top.blk+1]
            //   vl: M3 [top.blk+1]
            ("M3", b"M3 [top.\\blk+1]"),
        ],
        &[],
    );
}
