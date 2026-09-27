//! The package lane answers what the module lane answers (ROADMAP §2 row 14, which absorbed
//! row 26). Row 26 recorded a routing of the PACKAGE binder alone through the width-aware fold:
//! it made `pk::X`'s stored value canonical while every consumer still folded through the
//! width-unlimited walk, and measured a net loss (1,233 correct→silent against 714 fixed).
//!
//! The four lanes agree cell for cell — a `localparam` in the module, in a package read as
//! `pk::X`, a module `localparam` over an IMPORTED `PA`, and one over `pk::PA`. Six of the twelve
//! cells were wrong in all four until row 14's routing (a ≤64-bit declared target folded through
//! the unlimited walk: `localparam logic [63:0] X = PA ^ 64'h0;` over a signed 8-bit
//! `PA = 8'hFE` was `fffffffffffffffe`, both oracles `00000000000000fe`), and they moved together
//! because every binder takes the route (§4.5.556). This file keeps pinning the agreement, so a
//! change that moves one lane and not the others fails here before it can reach a design. Every
//! value is iverilog 13.0's and verilator 5.052's.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_pklane_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    assert!(out.status.success(), "expected exit 0, got:\n{s}");
    s.lines()
        .find(|l| l.starts_with("T "))
        .unwrap_or_else(|| panic!("no T line in:\n{s}"))
        .to_string()
}

/// `x1` — both oracles `bits=64 hex=00000000000000fe`; all four lanes moved together when row 14's routing landed (was `bits=64 hex=fffffffffffffffe`).
#[test]
fn lane_x1_module_package_import_and_scoped_agree() {
    assert_eq!(
        run(r#"module t;
  localparam logic signed [7:0] PA = 8'hFE;
  localparam logic [63:0] X = PA ^ 64'h0;
  initial begin #1; $display("T bits=%0d hex=%h", $bits(X), X); $finish; end
endmodule
"#),
        "T bits=64 hex=00000000000000fe"
    );
    assert_eq!(
        run(r#"package pk;
  localparam logic signed [7:0] PA = 8'hFE;
  localparam logic [63:0] X = PA ^ 64'h0;
endpackage
module t;
  initial begin #1; $display("T bits=%0d hex=%h", $bits(pk::X), pk::X); $finish; end
endmodule
"#),
        "T bits=64 hex=00000000000000fe"
    );
    assert_eq!(
        run(r#"package pk;
  localparam logic signed [7:0] PA = 8'hFE;
endpackage
module t;
  import pk::*;
  localparam logic [63:0] X = PA ^ 64'h0;
  initial begin #1; $display("T bits=%0d hex=%h", $bits(X), X); $finish; end
endmodule
"#),
        "T bits=64 hex=00000000000000fe"
    );
    assert_eq!(
        run(r#"package pk;
  localparam logic signed [7:0] PA = 8'hFE;
endpackage
module t;
  localparam logic [63:0] X = pk::PA ^ 64'h0;
  initial begin #1; $display("T bits=%0d hex=%h", $bits(X), X); $finish; end
endmodule
"#),
        "T bits=64 hex=00000000000000fe"
    );
}

/// `x2` — both oracles `bits=64 hex=00000000000000fe`; all four lanes moved together when row 14's routing landed (was `bits=64 hex=fffffffffffffffe`).
#[test]
fn lane_x2_module_package_import_and_scoped_agree() {
    assert_eq!(
        run(r#"module t;
  localparam logic signed [7:0] PA = 8'hFE;
  localparam logic [63:0] X = PA + 64'd0;
  initial begin #1; $display("T bits=%0d hex=%h", $bits(X), X); $finish; end
endmodule
"#),
        "T bits=64 hex=00000000000000fe"
    );
    assert_eq!(
        run(r#"package pk;
  localparam logic signed [7:0] PA = 8'hFE;
  localparam logic [63:0] X = PA + 64'd0;
endpackage
module t;
  initial begin #1; $display("T bits=%0d hex=%h", $bits(pk::X), pk::X); $finish; end
endmodule
"#),
        "T bits=64 hex=00000000000000fe"
    );
    assert_eq!(
        run(r#"package pk;
  localparam logic signed [7:0] PA = 8'hFE;
endpackage
module t;
  import pk::*;
  localparam logic [63:0] X = PA + 64'd0;
  initial begin #1; $display("T bits=%0d hex=%h", $bits(X), X); $finish; end
endmodule
"#),
        "T bits=64 hex=00000000000000fe"
    );
    assert_eq!(
        run(r#"package pk;
  localparam logic signed [7:0] PA = 8'hFE;
endpackage
module t;
  localparam logic [63:0] X = pk::PA + 64'd0;
  initial begin #1; $display("T bits=%0d hex=%h", $bits(X), X); $finish; end
endmodule
"#),
        "T bits=64 hex=00000000000000fe"
    );
}

/// `x3` — both oracles `bits=16 hex=00fe`; all four lanes moved together when row 14's routing landed (was `bits=16 hex=fffe`).
#[test]
fn lane_x3_module_package_import_and_scoped_agree() {
    assert_eq!(
        run(r#"module t;
  localparam logic signed [7:0] PA = 8'hFE;
  localparam logic [15:0] X = PA + 16'd0;
  initial begin #1; $display("T bits=%0d hex=%h", $bits(X), X); $finish; end
endmodule
"#),
        "T bits=16 hex=00fe"
    );
    assert_eq!(
        run(r#"package pk;
  localparam logic signed [7:0] PA = 8'hFE;
  localparam logic [15:0] X = PA + 16'd0;
endpackage
module t;
  initial begin #1; $display("T bits=%0d hex=%h", $bits(pk::X), pk::X); $finish; end
endmodule
"#),
        "T bits=16 hex=00fe"
    );
    assert_eq!(
        run(r#"package pk;
  localparam logic signed [7:0] PA = 8'hFE;
endpackage
module t;
  import pk::*;
  localparam logic [15:0] X = PA + 16'd0;
  initial begin #1; $display("T bits=%0d hex=%h", $bits(X), X); $finish; end
endmodule
"#),
        "T bits=16 hex=00fe"
    );
    assert_eq!(
        run(r#"package pk;
  localparam logic signed [7:0] PA = 8'hFE;
endpackage
module t;
  localparam logic [15:0] X = pk::PA + 16'd0;
  initial begin #1; $display("T bits=%0d hex=%h", $bits(X), X); $finish; end
endmodule
"#),
        "T bits=16 hex=00fe"
    );
}

/// `x4` — both oracles `bits=16 hex=00fe`; all four lanes moved together when row 14's routing landed (was `bits=16 hex=fffe`).
#[test]
fn lane_x4_module_package_import_and_scoped_agree() {
    assert_eq!(
        run(r#"module t;
  localparam logic signed [7:0] PA = 8'hFE;
  localparam logic [15:0] X = PA | 16'h0;
  initial begin #1; $display("T bits=%0d hex=%h", $bits(X), X); $finish; end
endmodule
"#),
        "T bits=16 hex=00fe"
    );
    assert_eq!(
        run(r#"package pk;
  localparam logic signed [7:0] PA = 8'hFE;
  localparam logic [15:0] X = PA | 16'h0;
endpackage
module t;
  initial begin #1; $display("T bits=%0d hex=%h", $bits(pk::X), pk::X); $finish; end
endmodule
"#),
        "T bits=16 hex=00fe"
    );
    assert_eq!(
        run(r#"package pk;
  localparam logic signed [7:0] PA = 8'hFE;
endpackage
module t;
  import pk::*;
  localparam logic [15:0] X = PA | 16'h0;
  initial begin #1; $display("T bits=%0d hex=%h", $bits(X), X); $finish; end
endmodule
"#),
        "T bits=16 hex=00fe"
    );
    assert_eq!(
        run(r#"package pk;
  localparam logic signed [7:0] PA = 8'hFE;
endpackage
module t;
  localparam logic [15:0] X = pk::PA | 16'h0;
  initial begin #1; $display("T bits=%0d hex=%h", $bits(X), X); $finish; end
endmodule
"#),
        "T bits=16 hex=00fe"
    );
}

/// `x5` — both oracles agree.
#[test]
fn lane_x5_module_package_import_and_scoped_agree() {
    assert_eq!(
        run(r#"module t;
  localparam logic signed [7:0] PA = 8'hFE;
  localparam logic [127:0] X = PA ^ 128'h0;
  initial begin #1; $display("T bits=%0d hex=%h", $bits(X), X); $finish; end
endmodule
"#),
        "T bits=128 hex=000000000000000000000000000000fe"
    );
    assert_eq!(
        run(r#"package pk;
  localparam logic signed [7:0] PA = 8'hFE;
  localparam logic [127:0] X = PA ^ 128'h0;
endpackage
module t;
  initial begin #1; $display("T bits=%0d hex=%h", $bits(pk::X), pk::X); $finish; end
endmodule
"#),
        "T bits=128 hex=000000000000000000000000000000fe"
    );
    assert_eq!(
        run(r#"package pk;
  localparam logic signed [7:0] PA = 8'hFE;
endpackage
module t;
  import pk::*;
  localparam logic [127:0] X = PA ^ 128'h0;
  initial begin #1; $display("T bits=%0d hex=%h", $bits(X), X); $finish; end
endmodule
"#),
        "T bits=128 hex=000000000000000000000000000000fe"
    );
    assert_eq!(
        run(r#"package pk;
  localparam logic signed [7:0] PA = 8'hFE;
endpackage
module t;
  localparam logic [127:0] X = pk::PA ^ 128'h0;
  initial begin #1; $display("T bits=%0d hex=%h", $bits(X), X); $finish; end
endmodule
"#),
        "T bits=128 hex=000000000000000000000000000000fe"
    );
}

/// `x6` — both oracles `bits=32 hex=000000fe`; all four lanes moved together when row 14's routing landed (was `bits=32 hex=fffffffe`).
#[test]
fn lane_x6_module_package_import_and_scoped_agree() {
    assert_eq!(
        run(r#"module t;
  localparam logic signed [7:0] PA = 8'hFE;
  localparam logic [31:0] X = PA * 32'd1;
  initial begin #1; $display("T bits=%0d hex=%h", $bits(X), X); $finish; end
endmodule
"#),
        "T bits=32 hex=000000fe"
    );
    assert_eq!(
        run(r#"package pk;
  localparam logic signed [7:0] PA = 8'hFE;
  localparam logic [31:0] X = PA * 32'd1;
endpackage
module t;
  initial begin #1; $display("T bits=%0d hex=%h", $bits(pk::X), pk::X); $finish; end
endmodule
"#),
        "T bits=32 hex=000000fe"
    );
    assert_eq!(
        run(r#"package pk;
  localparam logic signed [7:0] PA = 8'hFE;
endpackage
module t;
  import pk::*;
  localparam logic [31:0] X = PA * 32'd1;
  initial begin #1; $display("T bits=%0d hex=%h", $bits(X), X); $finish; end
endmodule
"#),
        "T bits=32 hex=000000fe"
    );
    assert_eq!(
        run(r#"package pk;
  localparam logic signed [7:0] PA = 8'hFE;
endpackage
module t;
  localparam logic [31:0] X = pk::PA * 32'd1;
  initial begin #1; $display("T bits=%0d hex=%h", $bits(X), X); $finish; end
endmodule
"#),
        "T bits=32 hex=000000fe"
    );
}

/// `x7` — both oracles agree.
#[test]
fn lane_x7_module_package_import_and_scoped_agree() {
    assert_eq!(
        run(r#"module t;
  localparam logic signed [7:0] PA = 8'hFE;
  localparam logic [15:0] X = PA >>> 1;
  initial begin #1; $display("T bits=%0d hex=%h", $bits(X), X); $finish; end
endmodule
"#),
        "T bits=16 hex=ffff"
    );
    assert_eq!(
        run(r#"package pk;
  localparam logic signed [7:0] PA = 8'hFE;
  localparam logic [15:0] X = PA >>> 1;
endpackage
module t;
  initial begin #1; $display("T bits=%0d hex=%h", $bits(pk::X), pk::X); $finish; end
endmodule
"#),
        "T bits=16 hex=ffff"
    );
    assert_eq!(
        run(r#"package pk;
  localparam logic signed [7:0] PA = 8'hFE;
endpackage
module t;
  import pk::*;
  localparam logic [15:0] X = PA >>> 1;
  initial begin #1; $display("T bits=%0d hex=%h", $bits(X), X); $finish; end
endmodule
"#),
        "T bits=16 hex=ffff"
    );
    assert_eq!(
        run(r#"package pk;
  localparam logic signed [7:0] PA = 8'hFE;
endpackage
module t;
  localparam logic [15:0] X = pk::PA >>> 1;
  initial begin #1; $display("T bits=%0d hex=%h", $bits(X), X); $finish; end
endmodule
"#),
        "T bits=16 hex=ffff"
    );
}

/// `x8` — both oracles agree.
#[test]
fn lane_x8_module_package_import_and_scoped_agree() {
    assert_eq!(
        run(r#"module t;
  localparam logic signed [7:0] PA = 8'hFE;
  localparam logic [15:0] X = ~PA;
  initial begin #1; $display("T bits=%0d hex=%h", $bits(X), X); $finish; end
endmodule
"#),
        "T bits=16 hex=0001"
    );
    assert_eq!(
        run(r#"package pk;
  localparam logic signed [7:0] PA = 8'hFE;
  localparam logic [15:0] X = ~PA;
endpackage
module t;
  initial begin #1; $display("T bits=%0d hex=%h", $bits(pk::X), pk::X); $finish; end
endmodule
"#),
        "T bits=16 hex=0001"
    );
    assert_eq!(
        run(r#"package pk;
  localparam logic signed [7:0] PA = 8'hFE;
endpackage
module t;
  import pk::*;
  localparam logic [15:0] X = ~PA;
  initial begin #1; $display("T bits=%0d hex=%h", $bits(X), X); $finish; end
endmodule
"#),
        "T bits=16 hex=0001"
    );
    assert_eq!(
        run(r#"package pk;
  localparam logic signed [7:0] PA = 8'hFE;
endpackage
module t;
  localparam logic [15:0] X = ~pk::PA;
  initial begin #1; $display("T bits=%0d hex=%h", $bits(X), X); $finish; end
endmodule
"#),
        "T bits=16 hex=0001"
    );
}

/// `x9` — both oracles agree.
#[test]
fn lane_x9_module_package_import_and_scoped_agree() {
    assert_eq!(
        run(r#"module t;
  localparam logic signed [7:0] PA = 8'hFE;
  localparam logic signed [15:0] X = PA + 16'sd0;
  initial begin #1; $display("T bits=%0d hex=%h", $bits(X), X); $finish; end
endmodule
"#),
        "T bits=16 hex=fffe"
    );
    assert_eq!(
        run(r#"package pk;
  localparam logic signed [7:0] PA = 8'hFE;
  localparam logic signed [15:0] X = PA + 16'sd0;
endpackage
module t;
  initial begin #1; $display("T bits=%0d hex=%h", $bits(pk::X), pk::X); $finish; end
endmodule
"#),
        "T bits=16 hex=fffe"
    );
    assert_eq!(
        run(r#"package pk;
  localparam logic signed [7:0] PA = 8'hFE;
endpackage
module t;
  import pk::*;
  localparam logic signed [15:0] X = PA + 16'sd0;
  initial begin #1; $display("T bits=%0d hex=%h", $bits(X), X); $finish; end
endmodule
"#),
        "T bits=16 hex=fffe"
    );
    assert_eq!(
        run(r#"package pk;
  localparam logic signed [7:0] PA = 8'hFE;
endpackage
module t;
  localparam logic signed [15:0] X = pk::PA + 16'sd0;
  initial begin #1; $display("T bits=%0d hex=%h", $bits(X), X); $finish; end
endmodule
"#),
        "T bits=16 hex=fffe"
    );
}

/// `x10` — both oracles agree.
#[test]
fn lane_x10_module_package_import_and_scoped_agree() {
    assert_eq!(
        run(r#"module t;
  localparam logic signed [7:0] PA = 8'hFE;
  localparam logic [15:0] X = PA;
  initial begin #1; $display("T bits=%0d hex=%h", $bits(X), X); $finish; end
endmodule
"#),
        "T bits=16 hex=fffe"
    );
    assert_eq!(
        run(r#"package pk;
  localparam logic signed [7:0] PA = 8'hFE;
  localparam logic [15:0] X = PA;
endpackage
module t;
  initial begin #1; $display("T bits=%0d hex=%h", $bits(pk::X), pk::X); $finish; end
endmodule
"#),
        "T bits=16 hex=fffe"
    );
    assert_eq!(
        run(r#"package pk;
  localparam logic signed [7:0] PA = 8'hFE;
endpackage
module t;
  import pk::*;
  localparam logic [15:0] X = PA;
  initial begin #1; $display("T bits=%0d hex=%h", $bits(X), X); $finish; end
endmodule
"#),
        "T bits=16 hex=fffe"
    );
    assert_eq!(
        run(r#"package pk;
  localparam logic signed [7:0] PA = 8'hFE;
endpackage
module t;
  localparam logic [15:0] X = pk::PA;
  initial begin #1; $display("T bits=%0d hex=%h", $bits(X), X); $finish; end
endmodule
"#),
        "T bits=16 hex=fffe"
    );
}

/// `x11` — both oracles `bits=64 hex=000000000000007f`; all four lanes moved together when row 14's routing landed (was `bits=64 hex=ffffffffffffffff`).
#[test]
fn lane_x11_module_package_import_and_scoped_agree() {
    assert_eq!(
        run(r#"module t;
  localparam logic signed [7:0] PA = 8'hFE;
  localparam logic [63:0] X = PA / 64'd2;
  initial begin #1; $display("T bits=%0d hex=%h", $bits(X), X); $finish; end
endmodule
"#),
        "T bits=64 hex=000000000000007f"
    );
    assert_eq!(
        run(r#"package pk;
  localparam logic signed [7:0] PA = 8'hFE;
  localparam logic [63:0] X = PA / 64'd2;
endpackage
module t;
  initial begin #1; $display("T bits=%0d hex=%h", $bits(pk::X), pk::X); $finish; end
endmodule
"#),
        "T bits=64 hex=000000000000007f"
    );
    assert_eq!(
        run(r#"package pk;
  localparam logic signed [7:0] PA = 8'hFE;
endpackage
module t;
  import pk::*;
  localparam logic [63:0] X = PA / 64'd2;
  initial begin #1; $display("T bits=%0d hex=%h", $bits(X), X); $finish; end
endmodule
"#),
        "T bits=64 hex=000000000000007f"
    );
    assert_eq!(
        run(r#"package pk;
  localparam logic signed [7:0] PA = 8'hFE;
endpackage
module t;
  localparam logic [63:0] X = pk::PA / 64'd2;
  initial begin #1; $display("T bits=%0d hex=%h", $bits(X), X); $finish; end
endmodule
"#),
        "T bits=64 hex=000000000000007f"
    );
}

/// `x12` — both oracles agree.
#[test]
fn lane_x12_module_package_import_and_scoped_agree() {
    assert_eq!(
        run(r#"module t;
  localparam logic signed [7:0] PA = 8'hFE;
  localparam logic [7:0] X = PA >> 1;
  initial begin #1; $display("T bits=%0d hex=%h", $bits(X), X); $finish; end
endmodule
"#),
        "T bits=8 hex=7f"
    );
    assert_eq!(
        run(r#"package pk;
  localparam logic signed [7:0] PA = 8'hFE;
  localparam logic [7:0] X = PA >> 1;
endpackage
module t;
  initial begin #1; $display("T bits=%0d hex=%h", $bits(pk::X), pk::X); $finish; end
endmodule
"#),
        "T bits=8 hex=7f"
    );
    assert_eq!(
        run(r#"package pk;
  localparam logic signed [7:0] PA = 8'hFE;
endpackage
module t;
  import pk::*;
  localparam logic [7:0] X = PA >> 1;
  initial begin #1; $display("T bits=%0d hex=%h", $bits(X), X); $finish; end
endmodule
"#),
        "T bits=8 hex=7f"
    );
    assert_eq!(
        run(r#"package pk;
  localparam logic signed [7:0] PA = 8'hFE;
endpackage
module t;
  localparam logic [7:0] X = pk::PA >> 1;
  initial begin #1; $display("T bits=%0d hex=%h", $bits(X), X); $finish; end
endmodule
"#),
        "T bits=8 hex=7f"
    );
}
