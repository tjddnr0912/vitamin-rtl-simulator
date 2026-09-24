//! ROADMAP §2 F9: a body-local of an inline (static, straight-line) function
//! declared with a 2-STATE kind (`bit`, `byte`, `shortint`, `int`, `longint`).
//!
//! §6.11.1: such a variable cannot hold x or z, so a store of an unknown bit
//! stores 0. The frame path's local is a net and the engine's store does that;
//! the inline fold has no net, so `bit [7:0] b; b = x; f = b;` with
//! `x = 8'bx0000111` returned `X7` where both oracles give `07`. The fold now
//! wraps the stored value of a 2-state local in `SysFuncId::TwoState`
//! (format_version 34), which keeps the value's width and sign and names it once.
//!
//! ORACLES: iverilog 13.0 (`iverilog -g2012`) and verilator 5.052
//! (`--binary --timing`). verilator is a 2-state simulator, so on a 4-state
//! (`logic`) control it prints 0 where iverilog prints x; iverilog is the oracle
//! for those cells, and each says so. PRE = the binary built at the parent commit.

use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_ok(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_ilts_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let so = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.contains("simulation ended"))
        .collect::<Vec<_>>()
        .join("\n");
    let se = String::from_utf8_lossy(&out.stderr).into_owned();
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(out.status.code(), Some(0), "stdout:\n{so}\nstderr:\n{se}");
    assert!(!se.contains("error["), "no diagnostic expected:\n{se}");
    so
}

/// f9a — every 2-state kind squashes; the 4-state `logic` control keeps its x
/// (iverilog `X7`; verilator, 2-state, prints `07`). PRE: `X7 X7 X7 X7`.
#[test]
fn every_two_state_local_kind_drops_x() {
    let o = run_ok(
        r#"module top;
  function [7:0] f(input [7:0] x); bit [7:0] b; b = x; f = b; endfunction
  function [7:0] g(input [7:0] x); logic [7:0] b; b = x; g = b; endfunction
  function [7:0] fi(input [7:0] x); int b; b = x; fi = b; endfunction
  function [7:0] fb(input [7:0] x); byte b; b = x; fb = b; endfunction
  initial begin $display("F9A bit=%h logic=%h int=%h byte=%h", f(8'bx0000111), g(8'bx0000111), fi(8'bx0000111), fb(8'bx0000111)); #1 $finish; end
endmodule
"#,
    );
    assert_eq!(o, "F9A bit=07 logic=X7 int=07 byte=07");
}

/// f9b — a SIGNED 2-state local read into a wider return keeps its sign stamp
/// under the squash; a `z` bit is dropped too; the `automatic` (frame) twin was
/// already right. iverilog 13 is the oracle (verilator refuses a `z` argument:
/// "Unsupported tristate construct"). PRE: `sgn=xxX7 uns=00Z7`.
#[test]
fn signed_and_z_bearing_values_squash_and_keep_the_sign() {
    let o = run_ok(
        r#"module top;
  function [15:0] f(input [7:0] x); bit signed [7:0] b; b = x; f = b; endfunction
  function [15:0] g(input [7:0] x); bit [7:0] b; b = x; g = b; endfunction
  function automatic [7:0] ff(input [7:0] x); bit [7:0] b; b = x; ff = b; endfunction
  initial begin $display("F9B sgn=%h uns=%h frame=%h", f(8'bx0000111), g(8'bz1000111), ff(8'bx0000111)); #1 $finish; end
endmodule
"#,
    );
    assert_eq!(o, "F9B sgn=0007 uns=0047 frame=07");
}

/// The negative twin of the sign row: a NEGATIVE value in a `bit signed [7:0]`
/// local sign-extends into the `[15:0]` return, with and without an x bit
/// (`8'bx1111101` → `7d`, positive once bit 7 drops). The 2-state RETURN
/// (`function int fi`) was already right in PRE (such a function takes the frame
/// path). Both oracles. PRE: `negx=xxXd`.
#[test]
fn a_negative_signed_local_and_a_two_state_return() {
    let o = run_ok(
        r#"module top;
  function [15:0] f(input [7:0] x); bit signed [7:0] b; b = x; f = b; endfunction
  function int fi(input logic [7:0] x); fi = x; endfunction
  function [15:0] fs(input [7:0] x); bit signed [7:0] b; b = x; fs = b; endfunction
  initial begin $display("T2 neg=%h ret2=%h negx=%h", f(8'hFD), fi(8'bx0000111), fs(8'bx1111101)); #1 $finish; end
endmodule
"#,
    );
    assert_eq!(o, "T2 neg=fffd ret2=00000007 negx=007d");
}

/// f9c — a rhs that may NOT be repeated (`x ^ $random`). Its value has no oracle
/// (iverilog and verilator draw different streams), so only the property is
/// pinned: the stored value has no unknown bit. PRE printed `X3`.
#[test]
fn a_non_repeatable_rhs_is_squashed_without_x() {
    let o = run_ok(
        r#"module top;
  function [7:0] f(input [7:0] x); bit [7:0] b; b = x ^ $random; f = b; endfunction
  logic [7:0] v;
  initial begin v = f(8'bx0000111); $display("F9C unk=%0d", $isunknown(v)); #1 $finish; end
endmodule
"#,
    );
    assert_eq!(o, "F9C unk=0");
}

/// f9d — a chain through the local (`b = b + 1` after the squash), a narrower
/// and a wider 2-state local. PRE: `chain=xx wid=X7`.
#[test]
fn chain_narrow_and_wide_locals() {
    let o = run_ok(
        r#"module top;
  function [7:0] f(input [7:0] x); bit [7:0] b; b = x; b = b + 1; f = b; endfunction
  function [7:0] g(input [7:0] x); bit [3:0] b; b = x; g = b; endfunction
  function [7:0] h(input [7:0] x); bit [15:0] b; b = x; h = b; endfunction
  initial begin $display("F9D chain=%h nar=%h wid=%h", f(8'bx0000111), g(8'bx0000111), h(8'bx0000111)); #1 $finish; end
endmodule
"#,
    );
    assert_eq!(o, "F9D chain=08 nar=07 wid=07");
}
