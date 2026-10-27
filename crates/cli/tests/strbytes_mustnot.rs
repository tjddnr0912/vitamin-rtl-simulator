//! Q-STRBYTES must-not-move pins (PLAN § 6, MN1-MN4, MN9, MN10): what the row keeps
//! byte-identical to BASE. File names stay an OS boundary with BASE's spelling, a
//! `$vita_stage` label stays valid JSON, a code-authored diagnostic keeps its own UTF-8,
//! ASCII text is untouched on every format arm, a UTF-8 plusarg reads as before and the
//! stdout / stderr / `--log` order is unchanged. Every expected byte string below is
//! the PRE binary's output (W/accept/vita-9d60771d), compared raw.

mod strbytes_util;
use std::process::Command;
use strbytes_util::{check, esc, lanes, one_shot, Cell, Mode};

/// MN1: `"\310A.bin"` opened three ways. A literal and a string variable name the file
/// `ef bf bd 41 2e 62 69 6e` (U+FFFD), a packed reg `c3 88 43 2e 62 69 6e` (Latin-1),
/// as before: APFS refuses a lone `c8`, so no oracle can agree across file systems.
#[test]
fn mn1_file_names_keep_their_spelling() {
    let c = Cell {
        src: r#"module top;
  // MN1 (Q-STRBYTES L1, file names stay BASE's spelling): "\310A.bin" opened as a literal, from a string variable and from a
  // 56-bit reg.  PRE / POST directory bytes: ef bf bd 41 2e 62 69 6e (literal, string) and c3 88 41 2e 62 69 6e (packed).
  integer fd; string s; reg [55:0] r;
  initial begin
    fd = $fopen("\310A.bin", "w"); $display("N1 %0d", fd != 0); if (fd != 0) begin $fwrite(fd, "1"); $fclose(fd); end
    s = "\310B.bin"; fd = $fopen(s, "w"); $display("N2 %0d", fd != 0); if (fd != 0) begin $fwrite(fd, "2"); $fclose(fd); end
    r = "\310C.bin"; fd = $fopen(r, "w"); $display("N3 %0d", fd != 0); if (fd != 0) begin $fwrite(fd, "3"); $fclose(fd); end
    #1 $finish;
  end
endmodule
"#,
        inputs: &[],
        plus: None,
        mode: Mode::Word,
        staged: true,
    };
    for r in lanes(&c) {
        let mut names: Vec<&[u8]> = r.files.keys().map(|k| k.as_bytes()).collect();
        names.sort();
        let want: [&[u8]; 3] = [b"\xc3\x88C.bin", b"\xef\xbf\xbdA.bin", b"\xef\xbf\xbdB.bin"];
        assert_eq!(names, want, "[{}] {}", r.lane, esc(&r.out));
        assert_eq!(
            r.files[std::str::from_utf8(want[0]).unwrap()],
            b"3",
            "[{}]",
            r.lane
        );
    }
}

/// MN2: a `$vita_stage` label with a byte >= 0x80 stays valid JSON (the lossy view).
#[test]
fn mn2_stage_label_stays_json() {
    let d = std::env::temp_dir().join(format!("vita_strbytes_mn2_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("t.sv"), r#"module top;
  // MN2 (Q-STRBYTES L3): a $vita_stage label with a byte >= 0x80 -> stage.jsonl JSON bytes identical (the lossy view).
  integer x;
  initial begin
    x = 7;
    $vita_stage("\310lab", x);
    $vita_stage("plain", x + 1);
    $display("D1 done");
    #1 $finish;
  end
endmodule
"#).unwrap();
    let o = Command::new(env!("CARGO_BIN_EXE_vita"))
        .args(["-Wno-W1017", "--obs-dir", "obs", "t.sv", "+STAGE_TRACE"])
        .current_dir(&d)
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(0));
    let got = std::fs::read(d.join("obs/stage.jsonl")).unwrap();
    let _ = std::fs::remove_dir_all(&d);
    let want: &[u8] = b"{\"v\":1,\"t\":0,\"kind\":\"stage\",\"label\":\"\xef\xbf\xbdlab\",\"idx\":0,\"vals\":[\"7\"]}\x0a{\"v\":1,\"t\":0,\"kind\":\"stage\",\"label\":\"plain\",\"idx\":1,\"vals\":[\"8\"]}\x0a";
    assert_eq!(esc(&got), esc(want));
}

/// MN3: a code-authored diagnostic keeps its own UTF-8 ("§" = c2 a7) on stderr and in
/// the `--log` file.
#[test]
fn mn3_code_authored_diagnostic_keeps_its_utf8() {
    let c = Cell {
        src: r#"module sub(input a, output b); assign b = a; endmodule
module top;
  // MN3 (Q-STRBYTES (c)): a code-authored diagnostic carrying UTF-8 (W2003's "§", c2 a7) keeps its exact bytes on stderr and --log.
  sub u(.a(1'b1), .b(w));
  initial #1 $display("T0 %b", w);
endmodule
"#,
        inputs: &[],
        plus: None,
        mode: Mode::Word,
        staged: false,
    };
    let want: &[u8] = b"t.sv:2:8: warning[VITA-W2003] W-PARSE-IMPLICIT-NET: implicit net `top.w` inferred as a 1-bit wire (IEEE 1364-2005 \xc2\xa73.5); declare it explicitly, or use ``default_nettype none`` to make this an error [in top]";
    let mut runs = lanes(&c);
    let l = one_shot(&c, "native", &[], true);
    runs.push(strbytes_util::Run {
        lane: "--log file".into(),
        rc: l.rc,
        out: l.files["log.txt"].clone(),
        files: Default::default(),
    });
    for r in runs {
        let hit = r.out.split(|&b| b == b'\n').any(|x| x == want);
        assert!(hit, "[{}] want {}\n{}", r.lane, esc(want), esc(&r.out));
    }
}

/// MN4: ASCII text through every format arm, with widths, flags and `-`: byte-identical.
#[test]
fn mn4_ascii_every_arm_is_unchanged() {
    let c = Cell {
        src: r#"`timescale 1ns/1ps
module top;
  // MN4 (Q-STRBYTES (a)): every format arm over ASCII text, with widths, flags and `-`: byte-identical on every lane.
  string s; reg [31:0] v; reg [63:0] p; real r; integer i; reg [3:0] xz; int q[$]; int m[string];
  initial begin
    s = "abc"; v = 32'h4142_4344; p = "hello"; r = 3.25; i = -42; xz = 4'b10xz;
    q.push_back(1); q.push_back(2); m["k"] = 9;
    $display("A1 [%d|%5d|%-5d|%0d|%+d|%05d]", i, i, i, i, 42, i);
    $display("A2 [%h|%8h|%-8h|%0h|%08h]", v, v, v, v, v);
    $display("A3 [%b|%0b|%12b|%-12b]", xz, xz, xz, xz);
    $display("A4 [%o|%0o|%14o]", v, v, v);
    $display("A5 [%e|%f|%g|%10.3f|%-10.2e|%E|%G]", r, r, r, r, r, r, r);
    $display("A6 [%t|%0t|%12t|%-12t]", $time, $time, $time, $time);
    $display("A7 [%m|%12m|%-12m|]");
    $display("A8 [%l]");
    $display("A9 [%p|%0p|%8p|%-8p|%p|%p]", i, i, s, s, q, m);
    $display("B1 [%v|%8v]", xz[0], xz[1]);
    $display("B2 [%c|%3c|%-3c|]", 8'h41, 8'h42, 8'h43);
    $display("B3 [%s|%8s|%-8s|%0s|%s|%2s]", s, s, s, p, p, p);
    $display("B4 [%u|%z|]", v, v);
    $display("B5 [%%|%5%|%q]", v);
    $timeformat(-9, 2, " ns", 12);
    #3 $display("B6 [%t|%0t|%-14t|]", $realtime, $time, $time);
    $display("B7 ", s, " ", v, " ", r);
    $write("B8 [%s]", "x"); $write("\n");
    #1 $finish;
  end
endmodule
"#,
        inputs: &[],
        plus: None,
        mode: Mode::Word,
        staged: true,
    };
    let want: &[u8] = b"A1 [        -42|  -42|-42  |-42|        +42|-0042]\x0aA2 [41424344|41424344|41424344|41424344|41424344]\x0aA3 [10xz|10xz|        10xz|10xz        ]\x0aA4 [10120441504|10120441504|   10120441504]\x0aA5 [3.250000e+00|3.250000|3.25|     3.250|3.25e+00  |3.250000E+00|3.25]\x0aA6 [                   0|0|           0|0           ]\x0aA7 [top|         top|top         |]\x0aA8 [%l]\x0aA9 [-42|'hffffffd6|   \"abc\"|\"abc\"   |'{'h1, 'h2}|'{\"k\":'h9}]\x0aB1 [HiZ|     StX]\x0aB2 [A|  B|C  |]\x0aB3 [abc|     abc|abc     |hello|   hello|hello]\x0aB4 [||]\x0aB5 [%|%|%q]1094861636\x0aB6 [     3.00 ns|3.00 ns|3.00 ns       |]\x0aB7 abc 1094861636 3.25\x0aB8 [x]\x0asimulation ended (Finish) at time 4000\x0aerrors=0 warnings=0 notes=0\x0a";
    for r in lanes(&c) {
        let keep = |b: &[u8]| -> Vec<u8> {
            b.split(|&x| x == b'\n')
                .filter(|l| {
                    !l.is_empty()
                        && !l.starts_with(b"errors=")
                        && !l.windows(9).any(|w| w == b"VITA-W403")
                })
                .flat_map(|l| [l, b"\n"].concat())
                .collect()
        };
        assert_eq!(esc(&keep(&r.out)), esc(&keep(want)), "[{}]", r.lane);
    }
}

/// MN9: a UTF-8 plusarg (`+S=\303\251`) through `$test$plusargs` / `$value$plusargs`.
#[test]
fn mn9_utf8_plusarg_is_unchanged() {
    let c = Cell {
        src: r#"module top;
  // MN9: a UTF-8 plusarg (+S=\303\251) through $test$plusargs / $value$plusargs + %s: unchanged.
  string x;
  initial begin
    $display("U1 %0d %0d", $test$plusargs("S=\303\251"), $test$plusargs("T"));
    x = "-"; if ($value$plusargs("S=%s", x)) $display("U2 1 [%s] %0d", x, x.len()); else $display("U2 0 [%s]", x);
    #1 $finish;
  end
endmodule
"#,
        inputs: &[],
        plus: Some(b"+S=\xc3\xa9"),
        mode: Mode::Word,
        staged: true,
    };
    check(&c, &[("U1", b"U1 1 0"), ("U2", b"U2 1 [\xc3\xa9] 2")], &[]);
}

/// MN10: stdout, the STDERR descriptor and severity messages keep their order and their
/// stream (stdout: O1 O4 O5 ; stderr: O2 O3 O6 ; `--log`: the diagnostics and stdout).
#[test]
fn mn10_stream_order_is_unchanged() {
    let d = std::env::temp_dir().join(format!("vita_strbytes_mn10_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("t.sv"), r#"module top;
  // MN10: interleaving of stdout, STDERR-fd and a severity message: stdout, stderr and --log order identical (ASCII).
  initial begin
    $display("O1 first");
    $fwrite(32'h8000_0002, "O2 to stderr\n");
    $error("O3 sev");
    $display("O4 after");
    $fdisplay(32'h8000_0001, "O5 stdout fd");
    $warning("O6 warn");
    #1 $finish;
  end
endmodule
"#).unwrap();
    for be in ["native", "interp", "vm"] {
        let o = Command::new(env!("CARGO_BIN_EXE_vita"))
            .args(["--backend", be, "-Wno-W1017", "--log", "log.txt", "t.sv"])
            .current_dir(&d)
            .output()
            .unwrap();
        let log = std::fs::read(d.join("log.txt")).unwrap();
        assert_eq!(
            esc(&o.stdout),
            esc(b"O1 first\x0aO4 after\x0aO5 stdout fd\x0asimulation ended (Finish) at time 1\x0a"),
            "[{be}] stdout"
        );
        let e = String::from_utf8_lossy(&o.stderr);
        let (a, b, c) = (
            e.find("O2 to stderr").unwrap(),
            e.find("O3 sev").unwrap(),
            e.find("O6 warn").unwrap(),
        );
        assert!(a < b && b < c, "[{be}] stderr order\n{e}");
        assert_eq!(esc(&log), esc(b"O1 first\x0at.sv:6:5: error[VITA-E4003] E-RUN-USER-ERROR: O3 sev [in top] [at time 0]\x0aO4 after\x0aO5 stdout fd\x0at.sv:9:5: warning[VITA-W4007] W-RUN-USER-WARNING: O6 warn [in top] [at time 0]\x0asimulation ended (Finish) at time 1\x0aerrors=1 warnings=1 notes=0\x0a"), "[{be}] --log");
    }
    let _ = std::fs::remove_dir_all(&d);
}
