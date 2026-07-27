//! Pins for a fresh-area sweep (§4.5.235) that came back CLEAN. Nothing here was
//! broken; the point is that several of these areas had no regression test, and
//! two of them are places where vita is AHEAD of iverilog — which means the oracle
//! cannot notice if they ever regress. A no-oracle capability with no test is one
//! refactor away from silently disappearing.
//!
//! Covered and verified against iverilog 13.0 where it accepts the construct:
//!   * array query functions on unpacked, descending-non-zero-LSB and ascending
//!     vectors (`$left/$right/$low/$high/$size/$increment/$dimensions/$bits`);
//!   * bit queries (`$countones/$onehot/$onehot0/$isunknown/$countbits`),
//!     `$clog2(0)`/`$clog2(1)`, `do…while`, a `case` over a sized pattern;
//!   * `$sformatf` across `%0d/%s/%h/%b`, `$signed`/`$unsigned` round trips.
//!
//! NO ORACLE (iverilog 13.0 rejects the syntax outright) — hand-IEEE, pinned here:
//!   * a modport-typed port (`sub(ib.mp p)`);
//!   * a part-select of a FUNCTION RESULT (`f(0)[7:0]`).
//!
//! Streaming operators (`{<<8{x}}`) are rejected by BOTH, so vita's loud is honest
//! and there is nothing to pin.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_fsp_{}_{n}", std::process::id()));
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

/// `$increment` is −1 for an ascending range and +1 for a descending one, and the
/// query functions must follow the DECLARED direction, not a normalized one.
#[test]
fn array_query_functions_follow_the_declared_direction() {
    let (out, c) = run(
        "module m;\n  logic [7:0] a [0:3];\n  logic [15:4] b;\n  logic [0:7] c;\n\
           initial begin\n\
             $display(\"A=%0d %0d %0d %0d %0d %0d\", $left(a), $right(a), $low(a), \
                      $high(a), $size(a), $increment(a));\n\
             $display(\"B=%0d %0d %0d %0d %0d %0d\", $left(b), $right(b), $low(b), \
                      $high(b), $size(b), $increment(b));\n\
             $display(\"C=%0d %0d %0d %0d %0d %0d\", $left(c), $right(c), $low(c), \
                      $high(c), $size(c), $increment(c));\n\
             $display(\"D=%0d %0d\", $dimensions(a), $bits(a));\n\
             #1 $finish; end\nendmodule\n",
    );
    assert_eq!(c, Some(0), "no diagnostics expected; got:\n{out}");
    assert!(
        out.contains("A=0 3 0 3 4 -1"),
        "unpacked ascending; got:\n{out}"
    );
    assert!(
        out.contains("B=15 4 4 15 12 1"),
        "non-zero-LSB desc; got:\n{out}"
    );
    assert!(
        out.contains("C=0 7 0 7 8 -1"),
        "ascending vector; got:\n{out}"
    );
    assert!(out.contains("D=2 32"), "dimensions / bits; got:\n{out}");
}

/// Bit queries, `$clog2` at its edges, `do…while`, and a sized `case` pattern.
#[test]
fn bit_queries_and_control_flow() {
    let (out, c) = run(
        "module m;\n  logic [7:0] v = 8'b1101_0010;\n  logic [31:0] w = 32'h0000_00F0;\n\
           int i;\n  initial begin\n\
             $display(\"A=%0d %0d %0d %0d\", $countones(v), $onehot(w), $onehot0(w), \
                      $isunknown(v));\n\
             $display(\"B=%0d %0d\", $clog2(0), $clog2(1));\n\
             i = 0; do i = i + 1; while (i < 3); $display(\"C=%0d\", i);\n\
             case (v) 8'b1101_0010: $display(\"D=hit\"); default: $display(\"D=miss\"); endcase\n\
             $display(\"E=%0d %0d\", $countbits(v, 1'b1), $countbits(v, 1'b0));\n\
             #1 $finish; end\nendmodule\n",
    );
    assert_eq!(c, Some(0), "no diagnostics expected; got:\n{out}");
    for want in ["A=4 0 0 0", "B=0 0", "C=3", "D=hit", "E=4 4"] {
        assert!(out.contains(want), "expected `{want}`; got:\n{out}");
    }
}

/// NO ORACLE (iverilog rejects both spellings) — hand-IEEE. A modport-typed port
/// and a part-select of a function RESULT both work today and nothing pinned them.
#[test]
fn no_oracle_capabilities_that_iverilog_rejects() {
    // §25.5: a port declared with a modport takes that modport's directions.
    let (out, c) = run(
        "interface ib; logic [7:0] d; logic v; modport mp (input d, output v); endinterface\n\
         module sub(ib.mp p); assign p.v = (p.d > 8'd10); endmodule\n\
         module m;\n  ib bus();\n  sub u(bus);\n  initial begin\n\
             bus.d = 8'd20; #1 $display(\"M=%0d %b\", bus.d, bus.v);\n\
             bus.d = 8'd5;  #1 $display(\"M=%0d %b\", bus.d, bus.v);\n\
             #1 $finish; end\nendmodule\n",
    );
    assert_eq!(c, Some(0), "modport port; got:\n{out}");
    assert!(
        out.contains("M=20 1") && out.contains("M=5 0"),
        "modport; got:\n{out}"
    );

    // §11.5.1 on a function result: `f(0)` is 16'hABCD, so [7:0] is CD and the
    // next call's [15:8] is AB.
    let (out, c) = run("module m;\n\
           function automatic logic [15:0] f(input int k); return 16'hABCD + k; endfunction\n\
           logic [7:0] x;\n  initial begin\n\
             x = f(0)[7:0];  $display(\"F=%h\", x);\n\
             x = f(1)[15:8]; $display(\"G=%h\", x);\n\
             #1 $finish; end\nendmodule\n");
    assert_eq!(c, Some(0), "function-result part-select; got:\n{out}");
    assert!(
        out.contains("F=cd") && out.contains("G=ab"),
        "part-select; got:\n{out}"
    );
}

/// Formatting and sign-conversion round trips.
#[test]
fn sformatf_and_sign_conversions() {
    let (out, c) = run("module m;\n  string s;\n  initial begin\n\
             s = $sformatf(\"%0d|%s|%h|%b\", 42, \"hi\", 8'hAF, 3'b101);\n\
             $display(\"S=%s\", s);\n\
             $display(\"T=%0d %0d\", $unsigned(-8'sd1), $signed(8'hFF));\n\
             #1 $finish; end\nendmodule\n");
    assert_eq!(c, Some(0), "no diagnostics expected; got:\n{out}");
    assert!(out.contains("S=42|hi|af|101"), "sformatf; got:\n{out}");
    assert!(out.contains("T=255 -1"), "sign conversions; got:\n{out}");
}
