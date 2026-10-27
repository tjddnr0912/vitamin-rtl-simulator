//! Q-STRBYTES pins, GENERATED from the round-1 review cells (W/qstrbytes/r1/lens-sound/cells s01 s04 s07, W/qstrbytes/r1/lens-diff/cells2 n20 ; r2 F1-F3, N4) by W/qstrbytes/r2/bin/gen_pins.py.
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

const L_S01_TFVAR_SRC: &str = r#"module top;
  // s01: $timeformat suffix from a string VARIABLE and from a packed REG (render.rs run_timeformat non-literal branches)
  string sfx; reg [15:0] r16;
  initial begin
    sfx = "\310s";
    $timeformat(0, 0, sfx, 6);
    #3 $display("@T2 [%t]", $time);
    r16 = 16'hC873;
    $timeformat(0, 0, r16, 6);
    #1 $display("@T3 [%t]", $time);
    sfx = "\303\251";
    $timeformat(0, 0, sfx, 6);
    #1 $display("@T4 [%t]", $time);
    $timeformat(0, 0, sfx, -6);
    #1 $display("@T5 [%t]", $time);
    #1 $finish;
  end
endmodule
"#;

#[test]
fn l_s01_tfvar_moves() {
    check(
        &Cell {
            src: L_S01_TFVAR_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::At,
            staged: true,
        },
        &[
            // T2: MOVE = vl (F1 suffix from a string VARIABLE "\310s" (render.rs run_timeformat var branch) ; iv = s2v `[   3\xc8s]`: same bytes, the time digit is the pre-existing c26 split (PRE 0)) ; PRE @T2 [   0\xef\xbf\xbds]
            //   iv: @T2 [   3\xc8s]
            //   s2v: @T2 [   3\xc8s]
            //   vl: @T2 [   0\xc8s]
            ("T2", b"@T2 [   0\xc8s]"),
            // T3: MOVE = vl (F1 suffix from a packed REG 16'hC873 (packed branch, BASE Latin-1 c3 88) ; iv = s2v `[   4\xc8s]` digit split as T2) ; PRE @T3 [   0\xc3\x88s]
            //   iv: @T3 [   4\xc8s]
            //   s2v: @T3 [   4\xc8s]
            //   vl: @T3 [   0\xc8s]
            ("T3", b"@T3 [   0\xc8s]"),
            // T4: MOVE = vl (F1 width +6 counts bytes (c3 a9 = 2) ; iv = s2v `[   5\xc3\xa9]` same pad, digit split as T2) ; PRE @T4 [    0\xc3\xa9]
            //   iv: @T4 [   5\xc3\xa9]
            //   s2v: @T4 [   5\xc3\xa9]
            //   vl: @T4 [   0\xc3\xa9]
            ("T4", b"@T4 [   0\xc3\xa9]"),
            // T5: MOVE = HAND (F1 width -6: left-justified, pad counts bytes (IEEE ieee1800-2017.txt:27223 + :28965) = iv = s2v pad `[6\xc3\xa9   ]` ; digit 0 = vl / PRE (c26 split) ; vl does not pad `[0\xc3\xa9]`) ; PRE @T5 [0\xc3\xa9    ]
            //   iv: @T5 [6\xc3\xa9   ]
            //   s2v: @T5 [6\xc3\xa9   ]
            //   vl: @T5 [0\xc3\xa9]
            ("T5", b"@T5 [0\xc3\xa9   ]"),
        ],
        &[],
    );
}

const L_S04_PLUS_SRC: &str = r#"module top;
  // s04: argv +S=<c3 a9> ; a query that splits the UTF-8 char, numeric conversion (W4028 text) and %s
  task automatic hx(input string tag, input string a); $write("@%s", tag); for (int i = 0; i < a.len(); i++) $write(" %02x", a[i]); $display(" len=%0d", a.len()); endtask
  integer v, r; string s;
  initial begin
    v = 5; r = $value$plusargs("S=\303%d", v); $display("@P1 %0d %0d", r, v);
    s = "-"; r = $value$plusargs("S=\303%s", s); $write("@P2 %0d", r); hx("P2b", s);
    r = $test$plusargs("S=\303\251"); $display("@P3 %0d", r);
    s = "-"; r = $value$plusargs("S=%s", s); hx("P4", s);
    #1 $finish;
  end
endmodule
"#;

#[test]
fn l_s04_plus_moves() {
    check(
        &Cell {
            src: L_S04_PLUS_SRC,
            inputs: &[],
            plus: Some(b"+S=\xc3\xa9"),
            mode: Mode::At,
            staged: true,
        },
        &[
            // P1: MOVE = iv+s2v (F3 argv +S=c3 a9, $value$plusargs("S=\303%d"): byte prefix hits, numeric view of the rest (plusargs.rs:80, B19) invalid -> x = iv = s2v ; vl `1 0` (PROBE_CATALOG) ; W4028 quote = lossy view of argv (P-ARGV)) ; PRE @P1 0 5
            //   iv: @P1 1 x
            //   s2v: @P1 1 x
            //   vl: @P1 1 0
            ("P1", b"@P1 1 x"),
            // P2: MOVE = iv+s2v+vl ; PRE @P2 0@P2b 2d len=1
            //   iv: @P2 1@P2b a9 len=1
            //   s2v: @P2 1@P2b a9 len=1
            //   vl: @P2 1@P2b a9 len=1
            ("P2", b"@P2 1@P2b a9 len=1"),
        ],
        &[],
    );
}

#[test]
fn l_s04_plus_stays() {
    check(
        &Cell {
            src: L_S04_PLUS_SRC,
            inputs: &[],
            plus: Some(b"+S=\xc3\xa9"),
            mode: Mode::At,
            staged: true,
        },
        &[
            // P3: STAY = iv+s2v+vl ; PRE @P3 1
            //   iv: @P3 1
            //   s2v: @P3 1
            //   vl: @P3 1
            ("P3", b"@P3 1"),
            // P4: STAY = iv+s2v+vl ; PRE @P4 c3 a9 len=2
            //   iv: @P4 c3 a9 len=2
            //   s2v: @P4 c3 a9 len=2
            //   vl: @P4 c3 a9 len=2
            ("P4", b"@P4 c3 a9 len=2"),
        ],
        &[],
    );
}

const L_S07_MISC_SRC: &str = r#"module top;
  // s07: non-first string literal used as a format, %p of a string var with width, $swrite, $sformatf in a continuous-ish function arg
  task automatic hx(input string tag, input string a); $write("@%s", tag); for (int i = 0; i < a.len(); i++) $write(" %02x", a[i]); $display(" len=%0d", a.len()); endtask
  function automatic string g(input string a); return {a, "\377"}; endfunction
  string s, u; integer x;
  initial begin
    s = "\310A"; x = 7;
    $display("@M1 ", "\310%0d|", x);
    $display("@M2 %6p|", s);
    $swrite(u, "<%s>%c", s, 8'h80); hx("M3", u);
    hx("M4", g(s));
    u = $sformatf("%-4s|%4s", g(s), s); hx("M5", u);
    #1 $finish;
  end
endmodule
"#;

#[test]
fn l_s07_misc_moves() {
    check(
        &Cell {
            src: L_S07_MISC_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::At,
            staged: true,
        },
        &[
            // M1: MOVE = iv+s2v+vl ; PRE @M1 \xef\xbf\xbd7|
            //   iv: @M1 \xc87|
            //   s2v: @M1 \xc87|
            //   vl: @M1 \xc87|
            ("M1", b"@M1 \xc87|"),
            // M2: MOVE = HAND (%6p of "\310A": 4 bytes `"\xc8A"`, pad 2 (IEEE :28784 + :28965) ; vl ignores the %p width, iv / s2v have no %p (z10 class)) ; PRE @M2   "\xef\xbf\xbdA"|
            //   iv: @M2 <%6p>|\xc8A
            //   s2v: @M2 <%6p>|\xc8A
            //   vl: @M2 "\xc8A"|
            ("M2", b"@M2   \"\xc8A\"|"),
            // M3: MOVE = iv+s2v+vl ; PRE @M3 3c ef bf bd 41 3e c2 80 len=8
            //   iv: @M3 3c c8 41 3e 80 len=5
            //   s2v: @M3 3c c8 41 3e 80 len=5
            //   vl: @M3 3c c8 41 3e 80 len=5
            ("M3", b"@M3 3c c8 41 3e 80 len=5"),
            // M4: MOVE = iv+s2v+vl ; PRE @M4 ef bf bd 41 ef bf bd len=7
            //   iv: @M4 c8 41 ff len=3
            //   s2v: @M4 c8 41 ff len=3
            //   vl: @M4 c8 41 ff len=3
            ("M4", b"@M4 c8 41 ff len=3"),
            // M5: MOVE = iv+s2v+vl ; PRE @M5 ef bf bd 41 ef bf bd 20 7c 20 20 ef bf bd 41 len=15
            //   iv: @M5 c8 41 ff 20 7c 20 20 c8 41 len=9
            //   s2v: @M5 c8 41 ff 20 7c 20 20 c8 41 len=9
            //   vl: @M5 c8 41 ff 20 7c 20 20 c8 41 len=9
            ("M5", b"@M5 c8 41 ff 20 7c 20 20 c8 41 len=9"),
        ],
        &[],
    );
}

const L_N20_PLUSNUM_SRC: &str = r#"module top;
  integer n, h;
  initial begin
    n = -1; h = -1;
    if ($value$plusargs("S=\303%d", n)) $display("@U1 hit %0d|", n); else $display("@U1 miss %0d|", n);
    if ($value$plusargs("S=\303%h", h)) $display("@U2 hit %0d|", h); else $display("@U2 miss %0d|", h);
    if ($value$plusargs("S=\303\251%d", n)) $display("@U3 hit %0d|", n); else $display("@U3 miss %0d|", n);
    if ($value$plusargs("S=%d", n)) $display("@U4 hit %0d|", n); else $display("@U4 miss %0d|", n);
  end
endmodule
"#;

#[test]
fn l_n20_plusnum_moves() {
    check(
        &Cell {
            src: L_N20_PLUSNUM_SRC,
            inputs: &[],
            plus: Some(b"+S=\xc3\xa95"),
            mode: Mode::At,
            staged: true,
        },
        &[
            // U1: MOVE = iv+s2v (N4 argv +S=c3 a9 35, "S=\303%d": prefix hits, rest a9 35 invalid -> x = iv = s2v ; vl `hit 0`) ; PRE @U1 miss -1|
            //   iv: @U1 hit x|
            //   s2v: @U1 hit x|
            //   vl: @U1 hit 0|
            ("U1", b"@U1 hit x|"),
            // U2: MOVE = iv+s2v (N4 "S=\303%h": as U1 = iv = s2v ; vl `hit 5` parses past the invalid byte (PROBE_CATALOG)) ; PRE @U2 miss -1|
            //   iv: @U2 hit x|
            //   s2v: @U2 hit x|
            //   vl: @U2 hit 5|
            ("U2", b"@U2 hit x|"),
        ],
        &[],
    );
}

#[test]
fn l_n20_plusnum_stays() {
    check(
        &Cell {
            src: L_N20_PLUSNUM_SRC,
            inputs: &[],
            plus: Some(b"+S=\xc3\xa95"),
            mode: Mode::At,
            staged: true,
        },
        &[
            // U3: STAY = iv+s2v+vl ; PRE @U3 hit 5|
            //   iv: @U3 hit 5|
            //   s2v: @U3 hit 5|
            //   vl: @U3 hit 5|
            ("U3", b"@U3 hit 5|"),
            // U4: STAY = iv+s2v (STAY: "S=%d" of c3 a9 35 -> x = iv = s2v ; vl 0) ; PRE @U4 hit x|
            //   iv: @U4 hit x|
            //   s2v: @U4 hit x|
            //   vl: @U4 hit 0|
            ("U4", b"@U4 hit x|"),
        ],
        &[],
    );
}
