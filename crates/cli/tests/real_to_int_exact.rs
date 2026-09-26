//! A real converts to an integral target EXACTLY at every width: round half away from
//! zero, then the low bits of the rounded integer's two's-complement image (IEEE 1800
//! §6.12.2 / §6.24.1). ROADMAP §2 "Size cast / signedness" (the out-of-range
//! PREREQUISITE row) and "Real" (the |x| ≥ 2^127 row).
//!
//! The engine's `real_to_int_round` was `r as i128`, which SATURATES at |x| ≥ 2^127, and
//! the cast lane's IR-0 composition rounded through `$rtoi` (`x.trunc() as i128`, the
//! same saturation). So `real rv = 1.0e40; byte b = rv;` stored `ff` (both oracles `00`
//! — the low 8 bits of 10^40 are zero), `int'(rv)` was `ffffffff` against `00000000`,
//! `longint'(rv)` was `ffffffff00000000` against 0, `pl(rv)` into a `longint` formal
//! `ffffffff00000000`, and a `[191:0]` target held `…7fff…` where both oracles hold the
//! exact `…1d6329f1c35ca5000…`. Now every lane — the module store, the nonblocking
//! store, the continuous assign, `always_comb`, the frame bind, the inline bind and every
//! primitive cast — is the one conversion, and the cast names its operand ONCE at every
//! width (`longint'($random * 1.0)` draws once: `0000000012153524 next=c0895e81`, PRE
//! `ffffffff06d7cd0d next=47ecdb8f`).
//!
//! A second, separate defect the grounding found (`sim_engine::alias::copy_alias`): a
//! same-width copy of a REAL net (`wire [63:0] cw = rv;`) whose source a process writes
//! joined the READ alias, so a read of `cw` handed back `rv`'s IEEE-754 word
//! (`4072c00000000000` for 300.0, both oracles `12c`). A real root is excluded now; the
//! copy still rides the settle's repair.
//!
//! Every expected text is printed by iverilog 13.0 (`-g2012`) and verilator 5.052
//! (`--binary --timing`) alike, except where marked:
//! - `±inf` and NaN have no integer: verilator stores 0, iverilog all-x (oracle split,
//!   ROADMAP §2). vita stores 0 (the `RealToInt` node is never unknown).
//! - `-0.4` into a 128-bit target: iverilog stores `ffffffffffffffff0000000000000000`
//!   (its own 64-bit stores of the same value are 0 — a self-contradiction), verilator 0.
//! - `$rtoi` of an out-of-range real and `%d` of an out-of-range real are oracle splits
//!   (iverilog wraps / prints the exact decimal, verilator saturates / prints the low 64
//!   bits) and are not in this file.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn vita_on(src: &str, backend: Option<&str>) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_r2ie_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_vita"));
    cmd.arg(&path);
    if let Some(b) = backend {
        cmd.arg("--backend").arg(b);
    }
    let out = cmd.output().expect("run vita");
    let _ = std::fs::remove_file(&path);
    let mut all = String::from_utf8_lossy(&out.stdout).into_owned();
    all.push_str(&String::from_utf8_lossy(&out.stderr));
    let mut s = String::new();
    for l in all.lines().filter(|l| {
        !l.starts_with("simulation ended")
            && !l.starts_with("errors=")
            && !l.contains("W-PP-TIMESCALE-DEFAULT")
            && !l.contains("W-RUN-BACKEND-FALLBACK")
    }) {
        s.push_str(l);
        s.push('\n');
    }
    (s, out.status.success())
}

fn run(src: &str) -> String {
    let (s, ok) = vita_on(src, None);
    assert!(ok, "expected exit 0, got:\n{s}");
    s
}

/// The seven-width store cell: `b s i l w128 w130 w192 = rv` for each literal.
fn stores(rows: &[(&str, &str)]) -> String {
    let mut body = String::new();
    for (lit, tag) in rows {
        body.push_str(&format!(
            " rv = {lit}; b = rv; s = rv; i = rv; l = rv; w128 = rv; w130 = rv; w192 = rv; show(\"{tag}\");\n"
        ));
    }
    format!(
        "module t; real rv; byte b; shortint s; int i; longint l; reg [127:0] w128; reg [129:0] w130; reg [191:0] w192;\n\
task show(input string n); $display(\"%s b=%h s=%h i=%h l=%h w128=%h w130=%h w192=%h\", n, b, s, i, l, w128, w130, w192); endtask\n\
initial begin\n{body} #1 $finish; end endmodule\n"
    )
}

const OUT_OF_RANGE: &[(&str, &str)] = &[
    ("1.0e300", "e300"),
    ("1.0e40", "e40"),
    ("-1.0e40", "-e40"),
    ("3.0e38", "3e38"),
    ("-3.0e38", "-3e38"),
    ("1.0/0.0", "inf"),
    ("-1.0/0.0", "-inf"),
    ("0.0/0.0", "nan"),
];

const OUT_OF_RANGE_WANT: &str = "\
e300 b=00 s=0000 i=00000000 l=0000000000000000 w128=00000000000000000000000000000000 w130=000000000000000000000000000000000 w192=000000000000000000000000000000000000000000000000
e40 b=00 s=0000 i=00000000 l=0000000000000000 w128=6329f1c35ca500000000000000000000 w130=16329f1c35ca500000000000000000000 w192=000000000000001d6329f1c35ca500000000000000000000
-e40 b=00 s=0000 i=00000000 l=0000000000000000 w128=9cd60e3ca35b00000000000000000000 w130=29cd60e3ca35b00000000000000000000 w192=ffffffffffffffe29cd60e3ca35b00000000000000000000
3e38 b=00 s=0000 i=00000000 l=0000000000000000 w128=e1b1e5f90f9450000000000000000000 w130=0e1b1e5f90f9450000000000000000000 w192=0000000000000000e1b1e5f90f9450000000000000000000
-3e38 b=00 s=0000 i=00000000 l=0000000000000000 w128=1e4e1a06f06bb0000000000000000000 w130=31e4e1a06f06bb0000000000000000000 w192=ffffffffffffffff1e4e1a06f06bb0000000000000000000
inf b=00 s=0000 i=00000000 l=0000000000000000 w128=00000000000000000000000000000000 w130=000000000000000000000000000000000 w192=000000000000000000000000000000000000000000000000
-inf b=00 s=0000 i=00000000 l=0000000000000000 w128=00000000000000000000000000000000 w130=000000000000000000000000000000000 w192=000000000000000000000000000000000000000000000000
nan b=00 s=0000 i=00000000 l=0000000000000000 w128=00000000000000000000000000000000 w130=000000000000000000000000000000000 w192=000000000000000000000000000000000000000000000000
";

#[test]
fn an_out_of_range_real_stores_the_low_bits_of_its_exact_integer() {
    // PRE: `ff ffff ffffffff ffffffffffffffff 7fff… 07fff… …7fff…` on every positive row,
    // `00 … 8000… 38000… …8000…` on every negative one, `inf` as the positive rows.
    // The `inf`/`-inf`/`nan` rows are vita's 0 (verilator; iverilog all-x).
    assert_eq!(run(&stores(OUT_OF_RANGE)), OUT_OF_RANGE_WANT);
}

#[test]
fn the_i128_boundary_and_the_in_range_values_are_unchanged() {
    // 2^127 and 2^128 sat on the old saturation (PRE `ff … 7fff…`); the rest is PRE = POST.
    // `-0.4` is vita's 0 (verilator; iverilog's upper word is `ffff…`, its own 64-bit
    // stores 0).
    let src = stores(&[
        ("1.0e20", "e20"),
        ("-1.0e20", "-e20"),
        ("3.0e9", "3e9"),
        ("9223372036854775808.0", "2^63"),
        ("18446744073709551616.0", "2^64"),
        ("170141183460469231731687303715884105728.0", "2^127"),
        ("-170141183460469231731687303715884105728.0", "-2^127"),
        ("340282366920938463463374607431768211456.0", "2^128"),
        ("300.0", "300"),
        ("-300.0", "-300"),
        ("2.5", "2.5"),
        ("-2.5", "-2.5"),
        ("0.5", "0.5"),
        ("-0.4", "-0.4"),
    ]);
    assert_eq!(
        run(&src),
        "\
e20 b=00 s=0000 i=63100000 l=6bc75e2d63100000 w128=00000000000000056bc75e2d63100000 w130=000000000000000056bc75e2d63100000 w192=000000000000000000000000000000056bc75e2d63100000
-e20 b=00 s=0000 i=9cf00000 l=9438a1d29cf00000 w128=fffffffffffffffa9438a1d29cf00000 w130=3fffffffffffffffa9438a1d29cf00000 w192=fffffffffffffffffffffffffffffffa9438a1d29cf00000
3e9 b=00 s=5e00 i=b2d05e00 l=00000000b2d05e00 w128=000000000000000000000000b2d05e00 w130=0000000000000000000000000b2d05e00 w192=0000000000000000000000000000000000000000b2d05e00
2^63 b=00 s=0000 i=00000000 l=8000000000000000 w128=00000000000000008000000000000000 w130=000000000000000008000000000000000 w192=000000000000000000000000000000008000000000000000
2^64 b=00 s=0000 i=00000000 l=0000000000000000 w128=00000000000000010000000000000000 w130=000000000000000010000000000000000 w192=000000000000000000000000000000010000000000000000
2^127 b=00 s=0000 i=00000000 l=0000000000000000 w128=80000000000000000000000000000000 w130=080000000000000000000000000000000 w192=000000000000000080000000000000000000000000000000
-2^127 b=00 s=0000 i=00000000 l=0000000000000000 w128=80000000000000000000000000000000 w130=380000000000000000000000000000000 w192=ffffffffffffffff80000000000000000000000000000000
2^128 b=00 s=0000 i=00000000 l=0000000000000000 w128=00000000000000000000000000000000 w130=100000000000000000000000000000000 w192=000000000000000100000000000000000000000000000000
300 b=2c s=012c i=0000012c l=000000000000012c w128=0000000000000000000000000000012c w130=00000000000000000000000000000012c w192=00000000000000000000000000000000000000000000012c
-300 b=d4 s=fed4 i=fffffed4 l=fffffffffffffed4 w128=fffffffffffffffffffffffffffffed4 w130=3fffffffffffffffffffffffffffffed4 w192=fffffffffffffffffffffffffffffffffffffffffffffed4
2.5 b=03 s=0003 i=00000003 l=0000000000000003 w128=00000000000000000000000000000003 w130=000000000000000000000000000000003 w192=000000000000000000000000000000000000000000000003
-2.5 b=fd s=fffd i=fffffffd l=fffffffffffffffd w128=fffffffffffffffffffffffffffffffd w130=3fffffffffffffffffffffffffffffffd w192=fffffffffffffffffffffffffffffffffffffffffffffffd
0.5 b=01 s=0001 i=00000001 l=0000000000000001 w128=00000000000000000000000000000001 w130=000000000000000000000000000000001 w192=000000000000000000000000000000000000000000000001
-0.4 b=00 s=0000 i=00000000 l=0000000000000000 w128=00000000000000000000000000000000 w130=000000000000000000000000000000000 w192=000000000000000000000000000000000000000000000000
"
    );
}

#[test]
fn every_cast_and_bind_lane_takes_the_exact_low_bits() {
    // PRE on the first two rows: `int'=ffffffff longint'=ffffffff00000000 byte'=ff
    // shortint'=ffff pl=ffffffff00000000 pi=ffffffff pb=ff` (e300) and
    // `int'=ffffffff longint'=0000000000000000 byte'=ff shortint'=ffff pl=0 pi=ffffffff
    // pb=ff` (e40 — the two-word split's low word happened to be right).
    let src = "module t; real rv;
function automatic longint pl(input longint x); return x; endfunction
function automatic int pi(input int x); return x; endfunction
function automatic byte pb(input byte x); return x; endfunction
task show(input string n);
 $display(\"%s int'=%h longint'=%h byte'=%h shortint'=%h pl=%h pi=%h pb=%h\", n, int'(rv), longint'(rv), byte'(rv), shortint'(rv), pl(rv), pi(rv), pb(rv));
endtask
initial begin
 rv = 1.0e300; show(\"e300\");
 rv = 1.0e40; show(\"e40\");
 rv = -1.0e40; show(\"-e40\");
 rv = 1.0e20; show(\"e20\");
 rv = -1.0e20; show(\"-e20\");
 rv = 3.0e9; show(\"3e9\");
 rv = 9223372036854775808.0; show(\"2^63\");
 rv = 300.0; show(\"300\");
 rv = -2.5; show(\"-2.5\");
 #1 $finish; end endmodule
";
    assert_eq!(
        run(src),
        "\
e300 int'=00000000 longint'=0000000000000000 byte'=00 shortint'=0000 pl=0000000000000000 pi=00000000 pb=00
e40 int'=00000000 longint'=0000000000000000 byte'=00 shortint'=0000 pl=0000000000000000 pi=00000000 pb=00
-e40 int'=00000000 longint'=0000000000000000 byte'=00 shortint'=0000 pl=0000000000000000 pi=00000000 pb=00
e20 int'=63100000 longint'=6bc75e2d63100000 byte'=00 shortint'=0000 pl=6bc75e2d63100000 pi=63100000 pb=00
-e20 int'=9cf00000 longint'=9438a1d29cf00000 byte'=00 shortint'=0000 pl=9438a1d29cf00000 pi=9cf00000 pb=00
3e9 int'=b2d05e00 longint'=00000000b2d05e00 byte'=00 shortint'=5e00 pl=00000000b2d05e00 pi=b2d05e00 pb=00
2^63 int'=00000000 longint'=8000000000000000 byte'=00 shortint'=0000 pl=8000000000000000 pi=00000000 pb=00
300 int'=0000012c longint'=000000000000012c byte'=2c shortint'=012c pl=000000000000012c pi=0000012c pb=2c
-2.5 int'=fffffffd longint'=fffffffffffffffd byte'=fd shortint'=fffd pl=fffffffffffffffd pi=fffffffd pb=fd
"
    );
}

#[test]
fn a_target_wider_than_128_bits_holds_the_exact_integer() {
    // PRE: `…7fff…` (positive) / `…8000…` (negative) in every row.
    let src = "module t; real rv; reg [255:0] a; reg [511:0] c; reg [1023:0] d; reg [1030:0] e;
initial begin
 rv = 1.0e40; a = rv; c = rv; d = rv; e = rv; $display(\"e40 a=%h\", a); $display(\"e40 d=%h\", d);
 rv = -1.0e40; a = rv; $display(\"-e40 a=%h\", a);
 rv = 1.0e300; d = rv; e = rv; $display(\"e300 d=%h\", d); $display(\"e300 e=%h\", e);
 rv = 1.7976931348623157e308; d = rv; e = rv; $display(\"max d=%h\", d); $display(\"max e=%h\", e);
 rv = -1.7976931348623157e308; e = rv; $display(\"-max e=%h\", e);
 rv = 1.0e77; a = rv; $display(\"e77 a=%h\", a);
 rv = 1.0e78; a = rv; c = rv; $display(\"e78 a=%h c=%h\", a, c);
 #1 $finish; end endmodule
";
    assert_eq!(
        run(src),
        "\
e40 a=0000000000000000000000000000001d6329f1c35ca500000000000000000000
e40 d=0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001d6329f1c35ca500000000000000000000
-e40 a=ffffffffffffffffffffffffffffffe29cd60e3ca35b00000000000000000000
e300 d=00000017e43c8800759c00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
e300 e=0000000017e43c8800759c00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
max d=fffffffffffff800000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
max e=00fffffffffffff800000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
-max e=7f0000000000000800000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
e77 a=dd15fe86affad800000000000000000000000000000000000000000000000000
e78 a=a2dbf142dfcc8000000000000000000000000000000000000000000000000000 c=0000000000000000000000000000000000000000000000000000000000000008a2dbf142dfcc8000000000000000000000000000000000000000000000000000
"
    );
}

#[test]
fn the_continuous_always_comb_and_nonblocking_lanes_convert_the_same_way() {
    // PRE: `i=ffffffff l=ffffffffffffffff ui=ffffffff ul=ffffffffffffffff u64=ffffffffffffffff
    // u8=ff cw=483d6329f1c35ca5 cw130=07fff…` (the `cw` word was the read alias's IEEE
    // word, the second defect). `inf` is vita's 0 (verilator; iverilog x).
    let src = "module t; real rv = 1.0e40; real ra [0:1]; int i; longint l; int unsigned ui; longint unsigned ul; reg [63:0] u64; reg [7:0] u8;
wire [63:0] cw = rv; wire [129:0] cw130 = rv;
always_comb ui = rv;
initial begin
 ra[0] = 1.0e40; ra[1] = -1.0e40;
 i <= rv; l <= rv; ul = rv; u64 = rv; u8 = ra[0]; #1;
 $display(\"i=%h l=%h ui=%h ul=%h u64=%h u8=%h cw=%h cw130=%h\", i, l, ui, ul, u64, u8, cw, cw130);
 u64 = ra[1]; l = ra[1]; $display(\"neg u64=%h l=%h\", u64, l);
 rv = 1.0/0.0; #1 $display(\"inf cw=%h ui=%h\", cw, ui);
 #1 $finish; end endmodule
";
    assert_eq!(
        run(src),
        "i=00000000 l=0000000000000000 ui=00000000 ul=0000000000000000 u64=0000000000000000 u8=00 cw=0000000000000000 cw130=16329f1c35ca500000000000000000000\n\
neg u64=0000000000000000 l=0000000000000000\n\
inf cw=0000000000000000 ui=00000000\n"
    );
}

#[test]
fn a_same_width_copy_of_a_written_real_reads_the_converted_value() {
    // PRE: `L cw=4072c00000000000 … cs=4072c00000000000 cc=4072c00000000000`, `W 1
    // cw=c004000000000000`, `L3 cw=483d6329f1c35ca5`. Without the `rv = …` statements
    // (no process writes `rv`) the settle's repair answered and PRE was right — the
    // alias is the difference. `L4` is vita's 0 (iverilog x).
    let src = "module t; real rv = 300.0; wire [63:0] cw = rv; wire [65:0] c66 = rv; wire [129:0] c130 = rv; wire signed [63:0] cs = rv; wire [63:0] cc = cw;
always @(cw) $display(\"W %0t cw=%h\", $time, cw);
initial begin #1 $display(\"L cw=%h c66=%h c130=%h cs=%h cc=%h\", cw, c66, c130, cs, cc);
 rv = -2.5; #1 $display(\"L2 cw=%h c66=%h c130=%h cs=%h cc=%h\", cw, c66, c130, cs, cc);
 rv = 1.0e40; #1 $display(\"L3 cw=%h c66=%h c130=%h cs=%h cc=%h\", cw, c66, c130, cs, cc);
 rv = 0.0/0.0; #1 $display(\"L4 cw=%h c130=%h\", cw, c130);
 #1 $finish; end endmodule
";
    assert_eq!(
        run(src),
        "\
L cw=000000000000012c c66=0000000000000012c c130=00000000000000000000000000000012c cs=000000000000012c cc=000000000000012c
W 1 cw=fffffffffffffffd
L2 cw=fffffffffffffffd c66=3fffffffffffffffd c130=3fffffffffffffffffffffffffffffffd cs=fffffffffffffffd cc=fffffffffffffffd
W 2 cw=0000000000000000
L3 cw=0000000000000000 c66=00000000000000000 c130=16329f1c35ca500000000000000000000 cs=0000000000000000 cc=0000000000000000
L4 cw=0000000000000000 c130=000000000000000000000000000000000
"
    );
    // The port twin, and a copy of a copy, read the same way.
    let src = "module sub(input [63:0] p64, input [31:0] p32); endmodule
module t; real rv = 300.0; wire [63:0] cw = rv; wire [63:0] cc = cw;
sub u(.p64(rv), .p32(rv));
initial begin #1 $display(\"A cw=%h cc=%h p64=%h p32=%h\", cw, cc, u.p64, u.p32);
 rv = 1.0e20; #1 $display(\"B cw=%h cc=%h p64=%h p32=%h\", cw, cc, u.p64, u.p32);
 #1 $finish; end endmodule
";
    assert_eq!(
        run(src),
        "A cw=000000000000012c cc=000000000000012c p64=000000000000012c p32=0000012c\n\
B cw=6bc75e2d63100000 cc=6bc75e2d63100000 p64=6bc75e2d63100000 p32=63100000\n"
    );
}

#[test]
fn a_frame_call_or_a_random_draw_in_the_operand_is_named_once_at_every_width() {
    // PRE: `M4 i=ffffffff` (the frame call's result took the `$rtoi` composition).
    let src = "module t; int n = 0;
function automatic real rf(input real x); n++; return x; endfunction
function automatic longint pl(input longint x); return x; endfunction
initial begin
 $display(\"M l=%h n=%0d\", longint'(rf(1.0e40)), n);
 $display(\"M2 l=%h n=%0d\", longint'(rf(-1.0e40)), n);
 $display(\"M3 l=%h n=%0d\", longint'(rf(1.0e20)), n);
 $display(\"M4 i=%h n=%0d\", int'(rf(1.0e40)), n);
 $display(\"M5 s=%h b=%h n=%0d\", shortint'(rf(-3.0e38)), byte'(rf(300.7)), n);
 $display(\"M6 pl=%h n=%0d\", pl(rf(1.0e40)), n);
 $display(\"M7 l=%h n=%0d\", longint'(rf(2.5) * 2.0), n);
 #1 $finish; end endmodule
";
    assert_eq!(
        run(src),
        "M l=0000000000000000 n=1\nM2 l=0000000000000000 n=2\nM3 l=6bc75e2d63100000 n=3\nM4 i=00000000 n=4\nM5 s=0000 b=2d n=6\nM6 pl=0000000000000000 n=7\nM7 l=0000000000000005 n=8\n"
    );
    // `$random` inside a >32-bit cast drew 2–5 times (PRE `N2 l=ffffffff06d7cd0d`,
    // `N3 next=47ecdb8f`); iverilog's sequence, one draw per cast.
    let src = "module t; real r;
initial begin
 $display(\"N2 l=%h\", longint'($random * 1.0)); $display(\"N3 next=%h\", $random);
 $display(\"N4 i=%h\", int'($random * 1.0)); $display(\"N5 next=%h\", $random);
 r = $random * 1.0; $display(\"N6 r=%f i=%h l=%h\", r, int'(r), longint'(r));
 #1 $finish; end endmodule
";
    assert_eq!(
        run(src),
        "N2 l=0000000012153524\nN3 next=c0895e81\nN4 i=8484d609\nN5 next=b1f05663\nN6 r=112818957.000000 i=06b97b0d l=0000000006b97b0d\n"
    );
}

#[test]
fn the_cast_lanes_rounding_and_in_range_boundaries_are_unchanged() {
    // PRE = POST on every line (the composition was exact in range; this pins that the
    // single-mention node is too): ties away from zero, the odd integer in [2^52, 2^53),
    // the just-below-half, ±2^31 ± 0.5, ±1e19 into 64 bits, an x operand reading 0.0.
    let src = "module t; real rv; int i; time tm; logic [47:0] l48; logic unsigned [63:0] u64; integer it;
initial begin
 rv = 2.5;  $display(\"O1 %h %h %h %h\", int'(rv), time'(rv), 48'(int'(rv)), integer'(rv));
 rv = -2.5; i = rv; tm = rv; l48 = rv; u64 = rv; it = rv; $display(\"O2 i=%h tm=%h l48=%h u64=%h it=%h int'=%h time'=%h\", i, tm, l48, u64, it, int'(rv), time'(rv));
 rv = 4503599627370497.0; $display(\"O3 int'=%h longint'=%h\", int'(rv), longint'(rv));
 rv = 9007199254740993.0; $display(\"O4 longint'=%h\", longint'(rv));
 rv = 0.49999999999999994; $display(\"O5 int'=%h\", int'(rv));
 rv = -0.5; $display(\"O6 int'=%h byte'=%h\", int'(rv), byte'(rv));
 rv = 2147483647.5; $display(\"O7 int'=%h longint'=%h\", int'(rv), longint'(rv));
 rv = -2147483648.5; $display(\"O8 int'=%h longint'=%h\", int'(rv), longint'(rv));
 rv = 1.0e19; $display(\"O9 longint'=%h time'=%h\", longint'(rv), time'(rv));
 rv = -1.0e19; $display(\"O10 longint'=%h\", longint'(rv));
 i = 'x; rv = i; $display(\"O11 rv=%f int'=%h\", rv, int'(rv));
 #1 $finish; end endmodule
";
    assert_eq!(
        run(src),
        "\
O1 00000003 0000000000000003 000000000003 00000003
O2 i=fffffffd tm=fffffffffffffffd l48=fffffffffffd u64=fffffffffffffffd it=fffffffd int'=fffffffd time'=fffffffffffffffd
O3 int'=00000001 longint'=0010000000000001
O4 longint'=0020000000000000
O5 int'=00000000
O6 int'=ffffffff byte'=ff
O7 int'=80000000 longint'=0000000080000000
O8 int'=7fffffff longint'=ffffffff7fffffff
O9 longint'=8ac7230489e80000 time'=8ac7230489e80000
O10 longint'=7538dcfb76180000
O11 rv=0.000000 int'=00000000
"
    );
}

#[test]
fn every_backend_converts_the_same_way() {
    let src = stores(OUT_OF_RANGE);
    for be in ["interp", "vm", "native"] {
        let (s, ok) = vita_on(&src, Some(be));
        assert!(ok, "{be}: expected exit 0, got:\n{s}");
        assert_eq!(s, OUT_OF_RANGE_WANT, "backend {be}");
    }
}
