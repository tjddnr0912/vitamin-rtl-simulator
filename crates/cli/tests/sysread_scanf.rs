//! v9 `$sscanf` / `$fscanf` (Medium-bundle rank 5, SYS-READ PR-B; pure IR-0).
//! The first MULTI-ref-write intercept (the $value$plusargs family): the scanf
//! parser writes every matched ref arg and returns the conversion count.
//!
//! Every expected value is pinned to LIVE iverilog 13.0:
//!   - return = number of successful (non-suppressed) conversions; an empty
//!     source returns -1 (EOF), a present-but-unconvertible source returns 0;
//!   - %d/%h(=%x)/%o/%b numeric, %c (ONE char, NO leading-ws skip), %s
//!     (ws-delimited), %Nd width, %*d suppression; conversion stops at the
//!     first matching failure; the value truncates to the dest width;
//!   - $fscanf %d spans newlines (it is NOT line-buffered); a literal char in
//!     the format must match the input (a mismatch stops the scan).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str, files: &[(&str, &[u8])]) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_scanf_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    for (name, bytes) in files {
        std::fs::write(d.join(name), bytes).unwrap();
    }
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        out.status.code(),
    )
}

#[test]
fn sscanf_numeric_radixes() {
    let (out, _c) = run(
        "module t;\n\
         integer a, n;\n\
         initial begin\n\
           n=$sscanf(\"255\",\"%d\",a); $display(\"D %0d %0d\", n, a);\n\
           n=$sscanf(\"ff\",\"%h\",a); $display(\"H %0d %0d\", n, a);\n\
           n=$sscanf(\"17\",\"%o\",a); $display(\"O %0d %0d\", n, a);\n\
           n=$sscanf(\"1010\",\"%b\",a); $display(\"B %0d %0d\", n, a);\n\
         end\n\
         endmodule\n",
        &[],
    );
    assert!(out.contains("D 1 255"), "{out}");
    assert!(out.contains("H 1 255"), "{out}");
    assert!(out.contains("O 1 15"), "{out}");
    assert!(out.contains("B 1 10"), "{out}");
}

#[test]
fn sscanf_multi_and_partial_and_nomatch_and_eof() {
    let (out, _c) = run(
        "module t;\n\
         integer a, b, n;\n\
         initial begin\n\
           n=$sscanf(\"12 34\",\"%d %d\",a,b); $display(\"DD %0d %0d %0d\", n, a, b);\n\
           a=99; b=88; n=$sscanf(\"7\",\"%d %d\",a,b); $display(\"PART %0d %0d %0d\", n, a, b);\n\
           a=99; n=$sscanf(\"hello\",\"%d\",a); $display(\"NOM %0d %0d\", n, a);\n\
           a=99; n=$sscanf(\"\",\"%d\",a); $display(\"EMP %0d %0d\", n, a);\n\
           a=99; n=$sscanf(\"   \",\"%d\",a); $display(\"WS %0d %0d\", n, a);\n\
         end\n\
         endmodule\n",
        &[],
    );
    assert!(out.contains("DD 2 12 34"), "{out}");
    assert!(
        out.contains("PART 1 7 88"),
        "stop at first failed conversion:\n{out}"
    );
    assert!(out.contains("NOM 0 99"), "{out}");
    assert!(out.contains("EMP -1 99"), "empty source => -1:\n{out}");
    assert!(out.contains("WS 0 99"), "whitespace-only => 0:\n{out}");
}

#[test]
fn sscanf_char_width_suppress_string_truncate() {
    let (out, _c) = run(
        "module t;\n\
         integer a, n; reg [7:0] ch; reg [64:1] str;\n\
         initial begin\n\
           n=$sscanf(\" X\",\"%c\",ch); $display(\"C %0d %0d\", n, ch);\n\
           n=$sscanf(\"123\",\"%2d\",a); $display(\"W %0d %0d\", n, a);\n\
           a=99; n=$sscanf(\"5 6\",\"%*d %d\",a); $display(\"SUP %0d %0d\", n, a);\n\
           n=$sscanf(\"hi there\",\"%s\",str); $display(\"S %0d %h\", n, str);\n\
           n=$sscanf(\"300\",\"%d\",ch); $display(\"TR %0d %0d\", n, ch);\n\
         end\n\
         endmodule\n",
        &[],
    );
    assert!(out.contains("C 1 32"), "%c reads space, no ws skip:\n{out}");
    assert!(out.contains("W 1 12"), "%2d width:\n{out}");
    assert!(out.contains("SUP 1 6"), "%*d suppression:\n{out}");
    assert!(
        out.contains("S 1 0000000000006869"),
        "%s ws-delimited 'hi':\n{out}"
    );
    assert!(out.contains("TR 1 44"), "300 truncated to 8 bits:\n{out}");
}

#[test]
fn fscanf_spans_newlines_and_eof() {
    // $fscanf is NOT line-buffered: %d spans newlines. A 3rd read past the data
    // hits trailing-ws then EOF (=> 0); a truly-empty read => -1.
    let (out, _c) = run(
        "module t;\n\
         integer a, b, n, fd;\n\
         initial begin\n\
           fd=$fopen(\"nums.txt\",\"r\");\n\
           n=$fscanf(fd,\"%d %d\",a,b); $display(\"F1 %0d %0d %0d\", n, a, b);\n\
           n=$fscanf(fd,\"%d %d\",a,b); $display(\"F2 %0d %0d %0d\", n, a, b);\n\
           a=7; b=8; n=$fscanf(fd,\"%d %d\",a,b); $display(\"F3 %0d %0d %0d\", n, a, b);\n\
         end\n\
         endmodule\n",
        &[("nums.txt", b"10 20\n30 40\n")],
    );
    assert!(out.contains("F1 2 10 20"), "{out}");
    assert!(out.contains("F2 2 30 40"), "%d spans the newline:\n{out}");
    assert!(
        out.contains("F3 0 7 8"),
        "trailing ws then EOF => 0:\n{out}"
    );
}

#[test]
fn fscanf_literal_match_and_mismatch() {
    let (out, _c) = run(
        "module t;\n\
         integer a, b, n, fd;\n\
         initial begin\n\
           fd=$fopen(\"csv.txt\",\"r\");\n\
           n=$fscanf(fd,\"%d,%d\",a,b); $display(\"CSV %0d %0d %0d\", n, a, b);\n\
           $fclose(fd);\n\
           a=0; b=0; fd=$fopen(\"csv.txt\",\"r\");\n\
           n=$fscanf(fd,\"%d;%d\",a,b); $display(\"MIS %0d %0d %0d\", n, a, b);\n\
         end\n\
         endmodule\n",
        &[("csv.txt", b"12,34")],
    );
    assert!(out.contains("CSV 2 12 34"), "literal ',' matches:\n{out}");
    assert!(
        out.contains("MIS 1 12 0"),
        "literal ';' mismatch stops:\n{out}"
    );
}

#[test]
fn sscanf_wide_value_and_overflow_wrap() {
    // values wider than 64 bits parse into the full dest (no silent zero), and
    // an over-64-bit value wraps mod 2^width — all iverilog-pinned.
    let (out, _c) = run(
        "module t;\n\
         integer n; reg [63:0] w; reg [95:0] wide;\n\
         initial begin\n\
           w=0; n=$sscanf(\"18446744073709551617\",\"%d\",w); $display(\"OVF %0d %h\", n, w);\n\
           w=0; n=$sscanf(\"99999999999999999999999\",\"%d\",w); $display(\"HUGE %0d %h\", n, w);\n\
           wide=0; n=$sscanf(\"123456789abcdef0123\",\"%h\",wide); $display(\"WIDE %0d %h\", n, wide);\n\
         end\n\
         endmodule\n",
        &[],
    );
    assert!(
        out.contains("OVF 1 0000000000000001"),
        "2^64+1 wraps to 1:\n{out}"
    );
    assert!(out.contains("HUGE 1 02c7e14af67fffff"), "mod 2^64:\n{out}");
    assert!(
        out.contains("WIDE 1 00000123456789abcdef0123"),
        "wide hex into 96-bit dest:\n{out}"
    );
}

#[test]
fn sscanf_hex_4state_digits() {
    // %h/%o/%b accept 4-state digits: 'x' => an X group, with Verilog
    // x-extension when the value's MSB is x (iverilog: "x" => all-X, "1x" =>
    // 001x).
    let (out, _c) = run(
        "module t;\n\
         integer n; reg [15:0] h;\n\
         initial begin\n\
           h=16'h9999; n=$sscanf(\"x\",\"%h\",h); $display(\"HX %0d %h\", n, h);\n\
           h=16'h9999; n=$sscanf(\"1x\",\"%h\",h); $display(\"H1X %0d %h\", n, h);\n\
         end\n\
         endmodule\n",
        &[],
    );
    assert!(out.contains("HX 1 xxxx"), "x-MSB extends to all-X:\n{out}");
    assert!(out.contains("H1X 1 001x"), "known MSB zero-extends:\n{out}");
}

#[test]
fn sscanf_sign_only_for_decimal() {
    // a leading +/- is honored ONLY for %d; for %h/%o/%b it is a non-match, so
    // the conversion fails (dest unchanged). iverilog-pinned.
    let (out, _c) = run(
        "module t;\n\
         integer n, a; reg [63:0] w;\n\
         initial begin\n\
           a=99; n=$sscanf(\"-ff\",\"%h\",a); $display(\"NH %0d %0d\", n, a);\n\
           a=99; n=$sscanf(\"+ff\",\"%h\",a); $display(\"PH %0d %0d\", n, a);\n\
           w=0; n=$sscanf(\"-5\",\"%d\",w); $display(\"ND %0d %h\", n, w);\n\
         end\n\
         endmodule\n",
        &[],
    );
    assert!(out.contains("NH 0 99"), "sign rejected for %h:\n{out}");
    assert!(out.contains("PH 0 99"), "{out}");
    assert!(
        out.contains("ND 1 fffffffffffffffb"),
        "%d sign-extends:\n{out}"
    );
}

#[test]
fn scanf_c_ignores_width_and_zero_width_fails() {
    // %c reads exactly ONE char regardless of an explicit width (iverilog
    // ignores it); %3c%c therefore reads 'a' then 'b'. An explicit %0d/%0c/%0s
    // reads nothing => the conversion fails.
    let (out, _c) = run(
        "module t;\n\
         integer n, a; reg [7:0] c1, c2; reg [64:1] s;\n\
         initial begin\n\
           c1=0; c2=0; n=$sscanf(\"abcd\",\"%3c%c\",c1,c2); $display(\"CHK %0d %0d %0d\", n, c1, c2);\n\
           a=99; n=$sscanf(\"5\",\"%0d\",a); $display(\"D0 %0d %0d\", n, a);\n\
           a=99; n=$sscanf(\"hi\",\"%0s\",s); $display(\"S0 %0d\", n);\n\
         end\n\
         endmodule\n",
        &[],
    );
    assert!(
        out.contains("CHK 2 97 98"),
        "%3c reads 'a', %c reads 'b':\n{out}"
    );
    assert!(out.contains("D0 0 99"), "%0d matches nothing:\n{out}");
    assert!(out.contains("S0 0"), "%0s matches nothing:\n{out}");
}

#[test]
fn scanf_huge_width_does_not_panic() {
    // a pathological field width must not overflow/panic (saturating).
    let (out, code) = run(
        "module t;\n\
         integer n, a;\n\
         initial begin\n\
           a=0; n=$sscanf(\"42\",\"%99999999999d\",a); $display(\"HW %0d %0d\", n, a);\n\
         end\n\
         endmodule\n",
        &[],
    );
    assert!(
        out.contains("HW 1 42"),
        "huge width reads the number:\n{out}"
    );
    assert_eq!(code, Some(0), "no panic");
}

#[test]
fn scanf_nested_placement_is_loud() {
    let (out, code) = run(
        "module t;\n\
         integer a, x;\n\
         initial begin\n\
           x = $sscanf(\"5\",\"%d\",a) + 1;\n\
         end\n\
         endmodule\n",
        &[],
    );
    assert!(
        out.contains("VITA-E3009") || code == Some(1),
        "nested $sscanf must be loud: {out} code={code:?}"
    );
}
