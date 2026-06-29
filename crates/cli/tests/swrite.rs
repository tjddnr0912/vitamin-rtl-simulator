//! `$swrite` / `$swriteb` / `$swriteo` / `$swriteh` — "`$write` to a string"
//! (IEEE 1364-2005 §21.3.3). vita previously left `$swrite` unmapped, so it was
//! silently skipped (an unknown `$task` is a W3056 no-op) and the target string
//! stayed empty — a silent-wrong. The fix maps the family to `SysTaskId::Sformat`
//! (the same engine as `$sformat`: dest = args[0], a leading string-literal is the
//! format, every other arg renders `$write`-style via `format_args_str`; the b/o/h
//! variants set the default radix of unformatted args). Supported cases are pinned
//! to iverilog 13.0.
//!
//! Two pre-existing vita-vs-iverilog rendering quirks are INHERITED here (shared by
//! `$write`/`$display`/`$sformat`, NOT introduced by `$swrite`): the 32-bit decimal
//! default field width is 10 not 11 (`$write(42)` differs identically), and `%0s` of
//! a packed reg with leading null bytes pads instead of skipping. Those are recorded
//! as separate ROADMAP candidates; the tests below avoid the bare-int / packed-`%0s`
//! cases by either oracle-pinning the clean forms or asserting `$swrite` ≡ `$sformat`.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_swr_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
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
fn swrite_format_and_arg() {
    // The headline regression: `$swrite(s,"v=%0d",42)` must format into `s` (was
    // silently empty). iverilog: [v=42].
    let (out, code) = run(
        "module top; string s; initial begin $swrite(s,\"v=%0d\",42); \
         $display(\"[%s]\",s); $finish; end endmodule\n",
    );
    assert_eq!(code, Some(0));
    assert!(out.contains("[v=42]"), "swrite format+arg; got:\n{out}");
}

#[test]
fn swrite_hex_variant() {
    // `$swriteh(s,255)` renders the unformatted arg in hex (32-bit padded), matching
    // iverilog [000000ff].
    let (out, code) = run(
        "module top; string s; initial begin $swriteh(s,255); $display(\"[%s]\",s); \
         $finish; end endmodule\n",
    );
    assert_eq!(code, Some(0));
    assert!(out.contains("[000000ff]"), "swriteh; got:\n{out}");
}

#[test]
fn swrite_bin_variant() {
    let (out, code) = run(
        "module top; string s; initial begin $swriteb(s,5); $display(\"[%s]\",s); \
         $finish; end endmodule\n",
    );
    assert_eq!(code, Some(0));
    assert!(
        out.contains("[00000000000000000000000000000101]"),
        "swriteb; got:\n{out}"
    );
}

#[test]
fn swrite_oct_variant() {
    let (out, code) = run(
        "module top; string s; initial begin $swriteo(s,8); $display(\"[%s]\",s); \
         $finish; end endmodule\n",
    );
    assert_eq!(code, Some(0));
    assert!(out.contains("[00000000010]"), "swriteo; got:\n{out}");
}

#[test]
fn swrite_multi_string_concat() {
    // A `$write`-style list of string literals concatenates (no dedicated format):
    // iverilog [abc].
    let (out, code) = run(
        "module top; string s; initial begin $swrite(s,\"a\",\"b\",\"c\"); \
         $display(\"[%s]\",s); $finish; end endmodule\n",
    );
    assert_eq!(code, Some(0));
    assert!(out.contains("[abc]"), "swrite multi-string; got:\n{out}");
}

#[test]
fn swrite_format_with_signal() {
    let (out, code) = run(
        "module top; string s; logic [7:0] x; initial begin x=8'd9; \
         $swrite(s,\"x=%0d done\",x); $display(\"[%s]\",s); $finish; end endmodule\n",
    );
    assert_eq!(code, Some(0));
    assert!(out.contains("[x=9 done]"), "swrite fmt+signal; got:\n{out}");
}

#[test]
fn swrite_equals_sformat() {
    // For the common `(dest, format, args)` form, `$swrite` and `$sformat` share the
    // engine and must produce byte-identical output (proves correct routing,
    // independent of the oracle).
    let prog = |task: &str| {
        format!(
            "module top; string s; logic [15:0] a,b; initial begin a=16'd12; b=16'd34; \
             {task}(s,\"a=%0d b=%0d\",a,b); $display(\"[%s]\",s); $finish; end endmodule\n"
        )
    };
    let sw = run(&prog("$swrite"));
    let sf = run(&prog("$sformat"));
    assert_eq!(sw.1, Some(0));
    assert_eq!(sw, sf, "$swrite must equal $sformat for (dest,fmt,args)");
    assert!(sw.0.contains("[a=12 b=34]"), "got:\n{}", sw.0);
}

#[test]
fn swrite_nonwhole_dest_is_loud() {
    // A non-whole-net destination (part-select / array element) cannot be written by
    // the Sformat engine — it used to silently no-op (a silent-wrong). iverilog
    // loud-rejects it ("first argument must be a register or SV string"); vita now
    // does too (E3009), for both $swrite and $sformat.
    let (_o, code) = run(
        "module top; reg [15:0] s; initial begin s=0; $swrite(s[7:0],\"A\"); \
         $display(\"%h\",s); $finish; end endmodule\n",
    );
    assert_ne!(code, Some(0), "$swrite to a part-select must be loud");
    let (_o2, code2) = run(
        "module top; reg [7:0] arr[0:3]; initial begin $sformat(arr[1],\"%0d\",65); \
         $finish; end endmodule\n",
    );
    assert_ne!(code2, Some(0), "$sformat to an array element must be loud");
}

#[test]
fn swrite_no_longer_silently_empty() {
    // Direct regression guard: the formatted target must be non-empty (the bug
    // produced an empty string with no error).
    let (out, code) = run("module top; string s; initial begin $swrite(s,\"hello\"); \
         $display(\"<%s>\",s); $finish; end endmodule\n");
    assert_eq!(code, Some(0));
    assert!(
        out.contains("<hello>"),
        "swrite must not be empty; got:\n{out}"
    );
}
