//! A parameter's DECLARED type or range is the context its value converts into; only an
//! UNTYPED declaration takes the type of its value (IEEE 1800-2017 §6.20.2).
//!
//! Three binder routes answered from the value alone and lost the declaration:
//!
//! - A string-literal default or override went to the width-free string map whatever the
//!   target declared: `localparam logic [15:0] A = "a";` was 8 bits `61` and `logic [7:0]
//!   D = "ab"` 16 bits `6162`, where §5.9 gives the string's bytes at the declared width
//!   (`0061`, `62`). A string operand inside a declared-width initializer (`"a" + 1`) was
//!   E3009, and a forwarded string parameter onto a typed target was refused (named) or
//!   dropped for the declared default (positional).
//! - A real-literal default made the parameter `real` even when it declared an integral
//!   type: `localparam int X = 2.6;` read 2.6 with `$bits` 64 in every real context, and
//!   `logic [7:0] X = 1e3` was 1000 where both oracles convert to 232 (§6.24.1).
//! - A fill override of a `real` parameter read the parent-side 32-bit fold, 4294967295.0,
//!   where the fill is one unsigned bit (§5.7.1) and both oracles read 1.0.
//!
//! Every value below is what iverilog 13.0 and verilator 5.052 both print, unless a
//! cell says otherwise; `PRE` is what vita printed before.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_args(src: &str, args: &[&str]) -> Vec<String> {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_pdtdt_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .args(args)
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    let s =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "expected exit 0, got:\n{s}");
    s.lines()
        .filter(|l| l.starts_with("T "))
        .map(|l| l.trim_end().to_string())
        .collect()
}

fn check(src: &str, want: &[&str]) {
    assert_eq!(run_args(src, &[]), want, "{src}");
}

/// Run in a fresh directory holding `files`, for the cells that open a file by name.
fn check_in_dir(src: &str, files: &[(&str, &str)], want: &[&str]) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_pdtdt_d_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    for (name, text) in files {
        std::fs::write(d.join(name), text).unwrap();
    }
    std::fs::write(d.join("t.sv"), src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("t.sv")
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    let s =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "expected exit 0, got:\n{s}");
    let got: Vec<String> = s
        .lines()
        .filter(|l| l.starts_with("T "))
        .map(|l| l.trim_end().to_string())
        .collect();
    assert_eq!(got, want, "{src}\n{s}");
}

fn loud(src: &str, needle: &str) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_pdtdt_l_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    let s =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    assert!(
        !out.status.success() && s.contains(needle),
        "expected a refusal naming {needle:?}, got:\n{s}"
    );
}

/// The same declaration in the six places a parameter is bound: an ANSI header, a module
/// body, a top-level `localparam`, a generate scope, a package and an interface body.
fn in_every_scope(decl: &str, fmt: &str, want: &str) {
    let d = |x: &str| format!("$display(\"T {fmt}\", {x}, $bits({x}));");
    let srcs = [
        format!(
            "module sub #(parameter {decl}) ();\n  initial #1 {}\nendmodule\nmodule t; sub u(); initial #2 $finish; endmodule\n",
            d("X")
        ),
        format!(
            "module sub;\n  parameter {decl};\n  initial #1 {}\nendmodule\nmodule t; sub u(); initial #2 $finish; endmodule\n",
            d("X")
        ),
        format!(
            "module t;\n  localparam {decl};\n  initial begin #1 {} $finish; end\nendmodule\n",
            d("X")
        ),
        format!(
            "module t;\n  if (1) begin : g\n    localparam {decl};\n    initial #1 {}\n  end\n  initial #2 $finish;\nendmodule\n",
            d("X")
        ),
        format!(
            "package pk;\n  localparam {decl};\nendpackage\nmodule t;\n  initial begin #1 {} $finish; end\nendmodule\n",
            d("pk::X")
        ),
        format!(
            "interface ifc;\n  parameter {decl};\n  initial #1 {}\nendinterface\nmodule t; ifc i(); initial #2 $finish; endmodule\n",
            d("X")
        ),
    ];
    for src in &srcs {
        check(src, &[want]);
    }
}

/// A typed or ranged string-literal default is the string's bytes at the declared width.
#[test]
fn a_typed_string_default_takes_the_declared_width() {
    in_every_scope("logic [15:0] X = \"a\"", "%h %0d", "T 0061 16"); // PRE 61 8
    in_every_scope("[23:0] X = \"ab\"", "%h %0d", "T 006162 24"); // PRE 6162 16
    in_every_scope("bit [3:0] X = \"a\"", "%h %0d", "T 1 4"); // PRE 61 8
    in_every_scope("int X = \"a\"", "%h %0d", "T 00000061 32"); // PRE 61 8
    in_every_scope("logic [7:0] X = \"ab\"", "%h %0d", "T 62 8"); // PRE 6162 16
}

/// Past 64 bits the bytes zero-extend (or truncate) the same way.
#[test]
fn a_wide_typed_string_default_takes_the_declared_width() {
    check(
        "module t;\n  localparam logic [127:0] X = \"abcdefghijklmnop\";\n  \
         localparam logic [95:0] Y = \"ab\";\n  localparam logic [71:0] Z = \"abcdefghijk\";\n  \
         initial begin #1 $display(\"T %h %0d %h %0d %h %0d\", X, $bits(X), Y, $bits(Y), Z, $bits(Z)); $finish; end\n\
         endmodule\n",
        // PRE Y `6162 16`, Z `6162636465666768696a6b 88`
        &["T 6162636465666768696a6b6c6d6e6f70 128 000000000000000000006162 96 636465666768696a6b 72"],
    );
}

/// A string operand inside a declared-width initializer folds (PRE: E3009 on each).
#[test]
fn a_string_operand_folds_in_a_declared_width_initializer() {
    check(
        "module t;\n  localparam logic [15:0] A = \"a\" + 1;\n  localparam logic [15:0] B = {\"a\", 8'h0};\n  \
         localparam logic [15:0] C = \"a\" | 16'h100;\n  localparam logic [7:0] D = \"ab\" >> 4;\n  \
         localparam logic [31:0] E = \"ab\" * 2;\n  localparam logic F = \"a\" == 8'h61;\n  \
         localparam int G = \"a\" - \"b\";\n  localparam logic [15:0] H = {\"a\", \"\"};\n  \
         initial begin #1 $display(\"T %h %h %h %h %h %h %0d %h\", A, B, C, D, E, F, G, H); $finish; end\n\
         endmodule\n",
        &["T 0062 6100 0161 16 0000c2c4 1 -1 6100"],
    );
    check(
        "module t;\n  localparam logic [15:0] A = \"a\";\n  localparam logic [15:0] B = A + \"b\";\n  \
         localparam B2 = A;\n  localparam logic [7:0] C = A;\n  logic [A[3:0]-1:0] v;\n  \
         initial begin #1 $display(\"T %h %h %0d %h %0d\", B, B2, $bits(B2), C, $bits(v)); $finish; end\n\
         endmodule\n",
        &["T 00c3 0061 16 61 1"],
    );
}

/// The converted parameter is an ordinary integral one everywhere it is read: `%s` prints
/// its leading NUL byte as a space, a hierarchical read and `$bits` see 16 bits.
#[test]
fn a_typed_string_default_reads_as_an_integral_value() {
    check(
        "module t;\n  localparam logic [15:0] A = \"a\";\n  logic [15:0] v;\n  \
         initial begin v = A; #1 $display(\"T [%s] [%0s] %0d %h %0d\", A, A, A, v, A == \"a\"); $finish; end\n\
         endmodule\n",
        &["T [ a] [a] 97 0061 1"], // PRE `[a]` for the first
    );
    check(
        "module sub #(parameter logic [15:0] A = \"a\") (); endmodule\n\
         module t; sub u(); initial begin #1 $display(\"T %h %0d\", u.A, $bits(u.A)); $finish; end endmodule\n",
        &["T 0061 16"], // PRE E3009
    );
}

/// An untyped (or `string`) declaration still takes the type of its value.
#[test]
fn an_untyped_string_default_is_unchanged() {
    check(
        "module t;\n  localparam Q = \"a\";\n  localparam R = {\"a\",\"b\"};\n  \
         localparam string S = \"ab\";\n  localparam signed U = \"ab\";\n  string s;\n  \
         initial begin s = S; #1 $display(\"T %h %0d %h %0d %s %0d %h %0d\", Q, $bits(Q), R, $bits(R), s, s.len(), U, $bits(U)); $finish; end\n\
         endmodule\n",
        &["T 61 8 6162 16 ab 2 6162 16"],
    );
}

/// A string override of a TYPED target takes the bytes at the declared width, on every
/// channel, whether it is a literal or a forwarded string parameter; a numeric override of
/// a typed target whose default is a string applies.
#[test]
fn a_string_override_of_a_typed_target_takes_the_declared_width() {
    let sub = |decl: &str| {
        format!("module sub #(parameter {decl}) (); initial #1 $display(\"T %h %0d\", P, $bits(P)); endmodule\n")
    };
    let cells: &[(&str, &str, &str)] = &[
        ("logic [15:0] P = 0", "#(.P(\"a\"))", "T 0061 16"), // PRE 61 8
        ("logic [15:0] P = 0", "#(.P(\"abc\"))", "T 6263 16"), // PRE 616263 24
        ("int P = 0", "#(.P(\"a\"))", "T 00000061 32"),      // PRE 61 8
        ("bit [3:0] P = 0", "#(.P(\"a\"))", "T 1 4"),        // PRE 61 8
        ("logic [7:0] P = 0", "#(.P(\"ab\"))", "T 62 8"),    // PRE 6162 16
        ("logic [15:0] P = \"xy\"", "#(.P(\"a\"))", "T 0061 16"), // PRE 61 8
        ("logic [15:0] P = \"xy\"", "#(.P(5))", "T 0005 16"), // PRE E3002
        (
            "logic [127:0] P = 0",
            "#(.P(\"abcdefghijklmnop\"))",
            "T 6162636465666768696a6b6c6d6e6f70 128",
        ),
    ];
    for (decl, ov, want) in cells {
        check(
            &format!(
                "{}module t; sub {ov} u(); initial #2 $finish; endmodule\n",
                sub(decl)
            ),
            &[want],
        );
    }
    // defparam
    check(
        "module sub; parameter logic [15:0] P = 0; initial #1 $display(\"T %h %0d\", P, $bits(P)); endmodule\n\
         module t; sub u(); defparam u.P = \"a\"; initial #2 $finish; endmodule\n",
        &["T 0061 16"], // PRE 61 8
    );
    // a forwarded string parameter, positional (PRE: the declared default 0001) and named
    // (PRE: E3009)
    check(
        &format!(
            "{}module t; localparam S = \"ab\"; sub #(S) u(); initial #2 $finish; endmodule\n",
            sub("logic [15:0] P = 1")
        ),
        &["T 6162 16"],
    );
    check(
        &format!(
            "{}module t; localparam S = \"ab\"; sub #(.P(S)) u(); initial #2 $finish; endmodule\n",
            sub("logic [15:0] P = 1")
        ),
        &["T 6162 16"],
    );
    // an untyped target keeps the string's own width
    check(
        &format!(
            "{}module t; sub #(.P(\"a\")) u(); initial #2 $finish; endmodule\n",
            sub("P = 0")
        ),
        &["T 61 8"],
    );
}

/// `-G` on a top-level parameter: the same conversions (verilator's `-G` binds the same
/// values as its `#()` twins above).
#[test]
fn the_command_line_channel_converts_the_same_way() {
    let src = "module t #(parameter real R = 2.5, parameter logic [15:0] P = 1);\n  \
               initial begin #1 $display(\"T %f %h %0d\", R, P, $bits(P)); $finish; end\n\
               endmodule\n";
    assert_eq!(
        run_args(src, &["-G", "R='1", "-G", "P=\"a\""]),
        ["T 1.000000 0061 16"] // PRE 4294967295.000000 61 8
    );
    assert_eq!(
        run_args(src, &["-G", "R='0", "-G", "P=\"abc\""]),
        ["T 0.000000 6263 16"]
    );
}

/// A declared-integral type or range converts a real-literal default (§6.24.1): the value
/// rounds, and every read — a real context, `$bits`, an integer division, a width — sees
/// the integral parameter. PRE bound all of these `real`.
#[test]
fn a_real_literal_default_converts_to_the_declared_type() {
    let cells: &[(&str, &str, &str)] = &[
        ("int X = 2.6", "T 3 32 3.000000 1", "PRE 3 64 2.600000 1"),
        (
            "int X = -2.5",
            "T -3 32 -3.000000 -1",
            "PRE -2 64 -2.500000 -1",
        ),
        (
            "integer X = 1e3",
            "T 1000 32 1000.000000 500",
            "PRE 1000 64",
        ),
        (
            "logic [7:0] X = 1e3",
            "T 232 8 232.000000 116",
            "PRE 1000 64 1000.000000 500",
        ),
        (
            "logic [7:0] X = 2.6",
            "T 3 8 3.000000 1",
            "PRE 3 64 2.600000 1",
        ),
        ("byte X = -2.5", "T -3 8 -3.000000 -1", "PRE -2 64"),
        ("bit [3:0] X = 1e3", "T 8 4 8.000000 4", "PRE 1000 64"),
        ("time X = 2.6", "T 3 64 3.000000 1", "PRE 3 64 2.600000 1"),
        (
            "int unsigned X = (1.5)",
            "T 2 32 2.000000 1",
            "PRE 2 64 1.500000 1",
        ),
        (
            "logic signed [7:0] X = -(0.4)",
            "T 0 8 0.000000 0",
            "PRE 0 64 -0.400000 0",
        ),
    ];
    for (decl, want, _pre) in cells {
        let body = |x: &str| {
            format!("r = {x}; #1 $display(\"T %0d %0d %f %0d\", {x}, $bits({x}), r, {x} / 2);")
        };
        check(
            &format!(
                "module t;\n  localparam {decl};\n  real r;\n  initial begin {} $finish; end\nendmodule\n",
                body("X")
            ),
            &[want],
        );
        check(
            &format!(
                "module sub #(parameter {decl}) ();\n  real r;\n  initial begin {} end\nendmodule\n\
                 module t; sub u(); initial #2 $finish; endmodule\n",
                body("X")
            ),
            &[want],
        );
        check(
            &format!(
                "module t;\n  if (1) begin : g\n    localparam {decl};\n    real r;\n    initial begin {} end\n  end\n  initial #2 $finish;\nendmodule\n",
                body("X")
            ),
            &[want],
        );
        check(
            &format!(
                "package pk;\n  localparam {decl};\nendpackage\nmodule t;\n  real r;\n  initial begin {} $finish; end\nendmodule\n",
                body("pk::X")
            ),
            &[want],
        );
    }
    check(
        "module t;\n  localparam int X = 2.6;\n  localparam int Y = X * 2;\n  localparam real Z = X / 2;\n  \
         logic [X-1:0] v;\n  initial begin #1 $display(\"T %0d %0d %f %0d\", X, Y, Z, $bits(v)); $finish; end\n\
         endmodule\n",
        &["T 3 6 1.000000 3"], // PRE `3 5 1.300000`, and `logic [X-1:0]` E3009
    );
    check(
        "module sub #(parameter int X = 2.6) (); endmodule\n\
         module t; sub u(); initial begin #1 $display(\"T %0d %0d\", u.X, $bits(u.X)); $finish; end endmodule\n",
        &["T 3 32"], // PRE E3009
    );
}

/// An untyped real literal still makes a `real` parameter. `parameter signed X = 2.6` is an
/// oracle split (iverilog: 32-bit 3; verilator: real 2.6) and keeps vita's answer, which is
/// verilator's.
#[test]
fn an_untyped_real_literal_is_still_real() {
    check(
        "module t;\n  parameter X = 2.6;\n  localparam int Y = X * 2;\n  localparam signed Z = 2.6;\n  real r;\n  \
         initial begin r = Z; #1 $display(\"T %f %0d %0d %f %0d\", X, Y, $bits(Y), r, $bits(Z)); $finish; end\n\
         endmodule\n",
        &["T 2.600000 5 32 2.600000 64"],
    );
}

/// A declared-integral initializer that MENTIONS a real converts in a header and an
/// interface body too — the module-body, generate and package binders already did (PRE:
/// E3009 in these two).
#[test]
fn a_real_initializer_converts_in_a_header_and_an_interface() {
    check(
        "module sub #(parameter logic [7:0] X = 2.5 + 1) (); initial #1 $display(\"T %h %0d\", X, $bits(X)); endmodule\n\
         module t; sub u(); initial #2 $finish; endmodule\n",
        &["T 04 8"],
    );
    check(
        "interface ifc; parameter logic [7:0] X = 2.5 + 1; initial #1 $display(\"T %h %0d\", X, $bits(X)); endinterface\n\
         module t; ifc i(); initial #2 $finish; endmodule\n",
        &["T 04 8"],
    );
}

/// A fill override of a `real` parameter is one unsigned bit (§5.7.1): 1.0 or 0.0, on the
/// named, positional and `defparam` channels, and onto `realtime`. PRE 4294967295.0.
#[test]
fn a_fill_override_of_a_real_parameter_is_one_bit() {
    let sub = |ty: &str| {
        format!("module sub #(parameter {ty} R = 2.5) (); initial #1 $display(\"T %f\", R); endmodule\n")
    };
    for (ty, ov, want) in [
        ("real", "#(.R('1))", "T 1.000000"),
        ("real", "#(.R('0))", "T 0.000000"),
        ("real", "#('1)", "T 1.000000"),
        ("realtime", "#(.R('1))", "T 1.000000"),
    ] {
        check(
            &format!(
                "{}module t; sub {ov} u(); initial #2 $finish; endmodule\n",
                sub(ty)
            ),
            &[want],
        );
    }
    check(
        "module sub; parameter real R = 2.5; initial #1 $display(\"T %f\", R); endmodule\n\
         module t; sub u(); defparam u.R = '1; initial #2 $finish; endmodule\n",
        &["T 1.000000"],
    );
}

/// NOT this change, pinned as measured (ROADMAP §2): the EMPTY string literal is one NUL
/// byte (§5.9) — both oracles print `$bits("")` 8 and bind an untyped `localparam P = "";`
/// and `#(.P(""))` as 8 bits `00` — while vita's string map and runtime literal carry it
/// as 1 bit. A typed declaration takes the byte (`0000` at 16 bits, above).
#[test]
fn the_empty_string_residue_is_recorded() {
    check(
        "module t;\n  localparam P = \"\";\n  localparam logic [15:0] A = \"\";\n  string s;\n  \
         initial begin s = P; #1 $display(\"T %h %0d %0d %h %0d len=%0d\", P, $bits(P), $bits(\"\"), A, $bits(A), s.len()); $finish; end\n\
         endmodule\n",
        // both oracles: `T 00 8 8 0000 16 len=0`
        &["T 0 1 1 0000 16 len=0"],
    );
}

/// A DECLARED `real` / `realtime` parameter reads a string default or override as the
/// string's integral value converted to real (§5.9, §6.12.1). PRE bound these through
/// the width-free string map, which printed the right `%f` and divided as an integer
/// (`R/2` 48.0); the first cut of this change refused them.
#[test]
fn a_real_parameter_reads_a_string_as_its_integral_value() {
    check(
        "module sub #(parameter real R = \"a\") ();\n  initial #1 $display(\"T %f %f\", R, R/2);\nendmodule\n\
         module t; sub u(); initial #2 $finish; endmodule\n",
        &["T 97.000000 48.500000"],
    );
    check(
        "module t;\n  localparam real R = \"a\";\n  localparam realtime T = \"\\002\";\n  \
         initial begin #1 $display(\"T %f %f\", R, T); $finish; end\nendmodule\n",
        &["T 97.000000 2.000000"],
    );
    check(
        "interface ifc; parameter real R = \"a\"; initial #1 $display(\"T %f\", R); endinterface\n\
         module t; ifc i(); initial #2 $finish; endmodule\n",
        &["T 97.000000"],
    );
    let sub = "module sub #(parameter real R = 2.5) (); initial #1 $display(\"T %f %f\", R, R/2); endmodule\n";
    for (ov, want) in [
        ("#(.R(\"a\"))", "T 97.000000 48.500000"),
        ("#(\"ab\")", "T 24930.000000 12465.000000"),
    ] {
        check(
            &format!("{sub}module t; sub {ov} u(); initial #2 $finish; endmodule\n"),
            &[want],
        );
    }
    check(
        "module sub; parameter real P = 2.5; parameter realtime Q = 1.5; initial #1 $display(\"T %f %f\", P, Q); endmodule\n\
         module t; sub u(); defparam u.P = \"a\"; defparam u.Q = \"\\002\"; initial #2 $finish; endmodule\n",
        &["T 97.000000 2.000000"],
    );
    check(
        "interface ifc #(parameter real R = 2.5) (); initial #1 $display(\"T %f\", R); endinterface\n\
         module t; ifc #(.R(\"a\")) i(); initial #2 $finish; endmodule\n",
        &["T 97.000000"],
    );
}

/// A typed parameter holding a string is used as a FILE NAME by value: the leading NUL
/// bytes of its declared width are dropped (§6.16), as both oracles drop them — and as
/// vita already did for a non-constant argument.
#[test]
fn a_typed_string_parameter_names_a_file() {
    let hex = [("k.hex", "11\n22\n33\n44\n")];
    check_in_dir(
        "module t;\n  parameter [8*16-1:0] MEMFILE = \"k.hex\";\n  reg [7:0] mem [0:3];\n  \
         initial begin $readmemh(MEMFILE, mem); #1 $display(\"T %h %h %h %h\", mem[0], mem[1], mem[2], mem[3]); $finish; end\n\
         endmodule\n",
        &hex,
        &["T 11 22 33 44"],
    );
    check_in_dir(
        "module sub #(parameter logic [8*16-1:0] F = \"none.hex\") ();\n  reg [7:0] mem [0:3];\n  \
         initial begin $readmemh(F, mem); #1 $display(\"T %h %h %h %h\", mem[0], mem[1], mem[2], mem[3]); end\n\
         endmodule\nmodule t; sub #(.F(\"k.hex\")) u(); initial #2 $finish; endmodule\n",
        &hex,
        &["T 11 22 33 44"],
    );
    check_in_dir(
        "module t;\n  localparam logic [8*8-1:0] FN = \"k.hex\";\n  integer fd, c;\n  logic [7:0] mem [0:3];\n  \
         initial begin fd = $fopen(FN, \"r\"); c = $fgetc(fd); $readmemh(FN, mem);\n    \
         #1 $display(\"T %0d %0d %h\", fd != 0, c, mem[3]); $finish; end\nendmodule\n",
        &hex,
        &["T 1 49 44"],
    );
}

/// A generate-case label that is a string reads the string's integral value (§5.9):
/// `"ab"` matches `16'h6162`. The label mismatch predates this change for a numeric
/// scrutinee (vita took the default arm in both); a typed string parameter reaches it now.
#[test]
fn a_generate_case_matches_a_string_label() {
    check(
        "module sub #(parameter logic [15:0] P = \"ab\") ();\n  case (P)\n    \
         \"ab\": begin : ga initial #1 $display(\"T ab %h\", P); end\n    \
         \"c\": begin : gc initial #2 $display(\"T c %h\", P); end\n    \
         default: begin : gd initial #3 $display(\"T def %h\", P); end\n  endcase\nendmodule\n\
         module t; sub u1(); sub #(.P(\"c\")) u2(); sub #(.P(16'h0063)) u3(); initial #5 $finish; endmodule\n",
        &["T ab 6162", "T c 0063", "T c 0063"],
    );
}

/// An UNTYPED parameter takes the type of its final value (§6.20.2): an integral override
/// of `parameter R = 2.5` makes it integral — `#(.R(3))` is 32 bits with `R/2` 1, a fill
/// is one bit. PRE kept it real (`R/2` 1.5; the fill read 4294967295.0).
#[test]
fn an_integral_override_of_an_untyped_real_default_is_integral() {
    let sub = "module sub #(parameter R = 2.5) (); initial #1 $display(\"T %f %0d %f\", R, $bits(R), R/2); endmodule\n";
    for (ov, want) in [
        ("#(.R(3))", "T 3.000000 32 1.000000"),
        ("#(.R('1))", "T 1.000000 1 0.000000"),
    ] {
        check(
            &format!("{sub}module t; sub {ov} u(); initial #2 $finish; endmodule\n"),
            &[want],
        );
    }
    check(
        "module sub; parameter R = 2.5; initial #1 $display(\"T %f %0d %f\", R, $bits(R), R/2); endmodule\n\
         module t; sub #('1) u(); initial #2 $finish; endmodule\n",
        &["T 1.000000 1 0.000000"],
    );
}

/// An interface's real parameter read hierarchically shows the value its override bound
/// (PRE: the declared default, 4).
#[test]
fn an_interface_real_parameter_reads_its_override_hierarchically() {
    for (ov, want) in [("#(.R(7))", "T 7 7.000000"), ("#(.R('1))", "T 1 1.000000")] {
        check(
            &format!(
                "interface ifc; parameter real R = 4; endinterface\n\
                 module t; ifc {ov} i(); initial begin #1 $display(\"T %0d %f\", i.R, i.R); $finish; end endmodule\n"
            ),
            &[want],
        );
    }
}

/// NOT this change: an UNTYPED parameter whose initializer has a REAL result is a `real`
/// parameter (§6.20.2; both oracles `X/2` 2.5) and vita cannot bind it real yet. It is
/// refused in every binder — the module-body, generate and package binders used to round
/// it to an integer (`X/2` 2.0), which is the conversion only a declared integral type
/// asks for.
#[test]
fn an_untyped_real_expression_default_is_refused_rather_than_rounded() {
    loud(
        "module sub #(parameter real R0 = 2.5, parameter X = R0 * 2) (); initial #1 $display(\"T %f %f\", X, X/2); endmodule\n\
         module t; sub u(); initial #2 $finish; endmodule\n",
        "`R0` is a real",
    );
    loud(
        "module t;\n  localparam real R0 = 2.5;\n  localparam X = R0 * 2;\n  \
         initial begin #1 $display(\"T %f %f\", X, X/2); $finish; end\nendmodule\n",
        "`R0` is a real",
    );
}

/// An UNTYPED parameter whose initializer mentions a real but has an INTEGRAL result — a
/// comparison, a logical operator, a ternary choosing integers — is an integral parameter
/// and folds exactly (both oracles; PRE too). Only a real RESULT is refused (above).
#[test]
fn an_untyped_integral_result_over_reals_still_folds() {
    check(
        "module t;\n  localparam real R = 2.5;\n  localparam X1 = $rtoi(2.5);\n  localparam X2 = int'(R);\n  \
         localparam X3 = R > 1.0;\n  localparam X4 = 2.5 > 1;\n  localparam X5 = (R > 1.0) ? 4 : 5;\n  \
         localparam X6 = R == 2.5;\n  localparam X7 = 2.5 < 3.0 && 1;\n  \
         initial begin #1 $display(\"T %0d %0d %0d %0d %0d %0d %0d %0d %0d\", X1, X2, X3, X4, X5, X6, X7, $bits(X5), $bits(X3)); $finish; end\n\
         endmodule\n",
        &["T 2 3 1 1 4 1 1 32 1"],
    );
}

/// An untyped `parameter R = 2.5` overridden by a REAL parameter stays real, although the
/// real's exact value folds to an i64 too (both oracles `R/2` 2.5; iverilog `$bits` 1,
/// verilator 64 — a split, vita's 64 is verilator's).
#[test]
fn a_real_override_of_an_untyped_real_default_stays_real() {
    check(
        "module sub #(parameter R = 2.5) (); initial #1 $display(\"T %f %f\", R, R/2); endmodule\n\
         module t; localparam real X = 5; sub #(.R(X)) u(); initial #2 $finish; endmodule\n",
        &["T 5.000000 2.500000"],
    );
}

/// A string label compares like a numeric label: its bytes (§5.9) against the folded
/// scrutinee. A 64-bit string whose top bit is set does not read back as a negative
/// integer (both oracles compare unsigned at 64 bits: `int P = -1` misses it).
///
/// The compare is §12.5 case equality in the bit domain (ROADMAP §2 🆕 T,
/// `elaborate/src/gen_case.rs`), so a signed `-1` hits `16'hffff` and `"\\377\\377"`,
/// as both oracles do; before 🆕 T it compared two i64 values and missed both (these two
/// rows were pinned `T def` as KNOWN-WRONG). A pairwise self-width compare alone was built
/// and reverted earlier: it wrapped a context-determined scrutinee (`case (P + 4'd1)` over
/// a 4-bit `P = 15`) at 4 bits, where the case sizes every expression to the widest; the
/// last cell below is that shape, and 🆕 T decides a label only where the two sizings agree.
#[test]
fn a_generate_case_string_label_compares_as_an_i64() {
    for (decl, lab, want) in [
        ("logic [15:0] P = \"ab\"", "\"ab\"", "T hit"),
        (
            "int P = -1",
            "\"\\377\\377\\377\\377\\377\\377\\377\\377\"",
            "T def",
        ),
        ("logic signed [15:0] P = -1", "-1", "T hit"),
        ("logic [3:0] P = 15", "16", "T def"),
        // Both oracles `T hit` (PRE `T def`).
        ("logic signed [15:0] P = -1", "16'hffff", "T hit"),
        ("logic signed [15:0] P = -1", "\"\\377\\377\"", "T hit"),
    ] {
        check(
            &format!(
                "module t;\n  localparam {decl};\n  case (P)\n    {lab}: begin : ga initial #1 $display(\"T hit\"); end\n    \
                 default: begin : gd initial #1 $display(\"T def\"); end\n  endcase\n  initial #2 $finish;\nendmodule\n"
            ),
            &[want],
        );
    }
    // A context-determined scrutinee is sized to the widest expression of the case (both
    // oracles' procedural case; verilator's generate case too — iverilog's generate case
    // contradicts its own procedural twin).
    check(
        "module t;\n  localparam logic [3:0] P = 15;\n  case (P + 4'd1)\n    \
         16: begin : a initial #1 $display(\"T arm16\"); end\n    0: begin : b initial #1 $display(\"T arm0\"); end\n    \
         default: begin : d initial #1 $display(\"T def\"); end\n  endcase\n  initial #2 $finish;\nendmodule\n",
        &["T arm16"],
    );
}

/// A string LITERAL keeps its NUL bytes as text: `$sformatf("[%s]", "\\000a")` is four
/// characters in both oracles (and was in PRE). Only a NUMERIC constant used as a string
/// drops its leading NUL padding (the file-name cells above).
#[test]
fn a_string_literal_keeps_its_nul_bytes() {
    check(
        "module t;\n  string s;\n  initial begin s = $sformatf(\"[%s]\", \"\\000a\"); #1 $display(\"T %0d\", s.len()); $finish; end\nendmodule\n",
        &["T 4"],
    );
}
