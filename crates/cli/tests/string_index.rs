//! String element indexing `s[i]` — IEEE 1800 §6.16.2 (read) / §6.16.3 (write).
//! A `string` variable is a dynamic byte SEQUENCE, so `s[i]` is the front-indexed
//! CHARACTER (an 8-bit byte), 0 if out of range — NOT a bit-select of the
//! materialized packed vector. vita used to lower `s[i]` to a plain bit-select
//! (`"ABCD"[0]` read bit 0 of the packed value = a silent-wrong value, and a
//! write mutated one packed bit instead of the character). Fixed by routing the
//! read to the existing `.getc(i)` byte primitive (`StrGetC`) and the blocking
//! write to `.putc(i, c)` (`StrPutC`); both already implement §6.16 semantics
//! (front-indexed, OOB / NUL = no-op). Pure elaborate lowering, no IR change.
//! A non-blocking `s[i] <= c` (which iverilog 13.0 aborts on) is honest-loud.
//! Pinned to iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Run `src`, return (joined `R:`-prefixed display lines without the prefix, success).
fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_sidx_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let joined = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.trim().strip_prefix("R:"))
        .collect::<Vec<_>>()
        .join("|");
    (joined, out.status.success())
}

#[test]
fn read_front_indexed_bytes() {
    // s[0] is the FIRST character (front-indexed), an 8-bit byte value.
    let (o, ok) = run("module top; string s; initial begin s=\"ABCD\";\n\
         $display(\"R:%0d %0d %0d %0d\", s[0],s[1],s[2],s[3]); #1 $finish; end endmodule");
    assert!(ok && o == "65 66 67 68", "got:\n{o}");
}

#[test]
fn read_as_characters() {
    let (o, ok) = run("module top; string s; initial begin s=\"hello\";\n\
         $display(\"R:%c%c%c%c%c\", s[0],s[1],s[2],s[3],s[4]); #1 $finish; end endmodule");
    assert!(ok && o == "hello", "got:\n{o}");
}

#[test]
fn read_out_of_range_is_zero() {
    // Past the end, far past, and a negative index all read 0 (§6.16.2).
    let (o, ok) = run(
        "module top; string s; int i; initial begin s=\"ABCD\"; i=-1;\n\
         $display(\"R:%0d %0d %0d\", s[4], s[99], s[i]); #1 $finish; end endmodule",
    );
    assert!(ok && o == "0 0 0", "got:\n{o}");
}

#[test]
fn read_bit_width_is_eight() {
    // `s[i]` is a byte — `$bits` is 8, not 1 (the old bit-select width).
    let (o, ok) = run("module top; string s; initial begin s=\"A\";\n\
         $display(\"R:%0d\", $bits(s[0])); #1 $finish; end endmodule");
    assert!(ok && o == "8", "got:\n{o}");
}

#[test]
fn read_variable_index() {
    let (o, ok) = run(
        "module top; string s; int i; initial begin s=\"hello\"; i=2;\n\
         $display(\"R:%0d %c\", s[i], s[i]); #1 $finish; end endmodule",
    );
    assert!(ok && o == "108 l", "got:\n{o}");
}

#[test]
fn read_in_comparison() {
    // The character compares equal to its literal byte.
    let (o, ok) = run(
        "module top; string s; initial begin s=\"hello\";\n\
         if(s[0]==\"h\") $display(\"R:match\"); else $display(\"R:no\"); #1 $finish; end endmodule",
    );
    assert!(ok && o == "match", "got:\n{o}");
}

#[test]
fn read_counts_chars_in_loop() {
    // The canonical scan: count a character across the string.
    let (o, ok) = run(
        "module top; string s; int n; initial begin s=\"hello\"; n=0;\n\
         for(int i=0;i<s.len();i++) if(s[i]==\"l\") n++;\n\
         $display(\"R:%0d\", n); #1 $finish; end endmodule",
    );
    assert!(ok && o == "2", "got:\n{o}");
}

#[test]
fn write_sets_character() {
    let (o, ok) = run(
        "module top; string s; initial begin s=\"hello\"; s[0]=\"H\";\n\
         $display(\"R:%s\", s); #1 $finish; end endmodule",
    );
    assert!(ok && o == "Hello", "got:\n{o}");
}

#[test]
fn write_oob_and_nul_are_noops() {
    // s[4]="O" applies; s[99] (OOB) and s[1]=0 (NUL) are no-ops (§6.16.3).
    let (o, ok) = run(
        "module top; string s; initial begin s=\"hello\"; s[4]=\"O\"; s[99]=\"X\"; s[1]=0;\n\
         $display(\"R:%s\", s); #1 $finish; end endmodule",
    );
    assert!(ok && o == "hellO", "got:\n{o}");
}

#[test]
fn read_and_write_combined_reverse() {
    // In-place reverse exercises read and write on the same string.
    let (o, ok) = run("module top; string s; byte t; initial begin s=\"abcd\";\n\
         for(int i=0;i<2;i++) begin t=s[i]; s[i]=s[3-i]; s[3-i]=t; end\n\
         $display(\"R:%s\", s); #1 $finish; end endmodule");
    assert!(ok && o == "dcba", "got:\n{o}");
}

#[test]
fn nonblocking_string_element_is_loud() {
    // `s[i] <= c` has no defined behavior (iverilog 13.0 aborts) — honest-loud,
    // not a silent packed bit-write.
    let (_o, ok) = run(
        "module top; string s; initial begin s=\"hello\"; s[0] <= \"H\";\n\
         #1 $display(\"R:%s\", s); $finish; end endmodule",
    );
    assert!(!ok, "non-blocking string element write must be loud");
}

#[test]
fn nonblocking_intra_event_string_element_is_loud() {
    // The intra-assignment-event variant `s[i] <= @(ev) c` must also be loud —
    // the guard is ahead of the event early-return, not after it.
    let (_o, ok) = run("module top; string s; reg clk=0; always #5 clk=~clk;\n\
         initial begin s=\"hello\"; s[0] <= @(posedge clk) 8'h48;\n\
         #20 $display(\"R:%s\", s); $finish; end endmodule");
    assert!(
        !ok,
        "non-blocking intra-event string element write must be loud"
    );
}

#[test]
fn blocking_intra_event_string_element_is_loud() {
    // `s[i] = @(ev) c` (and `s[i] = #d c`): the byte write would have to land
    // after the wait — honest-loud rather than a silent same-tick bit-write.
    let (_o, ok) = run("module top; string s; reg clk=0; always #5 clk=~clk;\n\
         initial begin s=\"hello\"; #3 s[0] = @(posedge clk) 8'h48;\n\
         #20 $display(\"R:%s\", s); $finish; end endmodule");
    assert!(
        !ok,
        "blocking intra-event string element write must be loud"
    );
}

#[test]
fn concat_string_element_lvalue_is_loud() {
    // A string element inside a (single-element) concat lvalue `{s[0]} = c` slips
    // past the multi-chunk concat guard; the element chunk carries a non-None
    // offset, so it is rejected (a plain `s[0] = c` is the supported form).
    let (_o, ok) = run(
        "module top; string s; initial begin s=\"hello\"; {s[0]}=8'h48;\n\
         $display(\"R:%s\", s); #1 $finish; end endmodule",
    );
    assert!(!ok, "string element in a concat lvalue must be loud");
}

#[test]
fn nonstring_concat_lvalue_unchanged() {
    // The new concat guard must NOT over-catch a valid non-string concat write.
    let (o, ok) = run(
        "module top; logic[7:0] x; logic[7:0] y; initial begin x=0; y=0;\n\
         {x[0], y[7]}=2'b11; $display(\"R:%b %b\", x[0], y[7]); #1 $finish; end endmodule",
    );
    assert!(ok && o == "1 1", "got:\n{o}");
}

#[test]
fn packed_vector_bitselect_unchanged() {
    // A real packed vector keeps bit-select semantics — the fix is string-only.
    let (o, ok) = run("module top; logic[7:0] x; initial begin x=8'b10100101;\n\
         $display(\"R:%b %b %b\", x[0], x[7], x[4]); #1 $finish; end endmodule");
    assert!(ok && o == "1 1 0", "got:\n{o}");
}

#[test]
fn whole_string_ops_unchanged() {
    // Concatenation, len, and substr are untouched by the element-index routing.
    let (o, ok) = run(
        "module top; string s,t; initial begin s=\"hello\"; t={s,\" world\"};\n\
         $display(\"R:%s|%0d|%s\", t, s.len(), s.substr(1,3)); #1 $finish; end endmodule",
    );
    assert!(ok && o == "hello world|5|ell", "got:\n{o}");
}
