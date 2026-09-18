//! An INTERFACE member inside a §11.6.1 width region is sized and signed through
//! the interface declaration, exactly as a module instance's net is.
//!
//! `ifc w();` is a body instance like `m u();`, and `w.hi` is a hierarchical leaf
//! of the same shape as `u.hs`; but the per-module fact table the region walks
//! read (`expr_size_hier`) was built from `TopItem::Module` only, so
//! `hier_leaf_net` declined every interface member, `has_opaque_leaf` reported
//! it as unreachable, and the region kept its pre-slice evaluation width — the
//! width of the OTHER operand. `16'(w.uh * sq8)` over `logic [7:0] uh` evaluated
//! at 8 bits and printed `00000080` where both oracles print `0000bb80`; the
//! signed twin printed `ffffff90` for `00000090`; `case (w.s8 * sq8) 16'sd144:`
//! missed its arm. The table now covers interfaces (an interface's parameters are
//! bound by the same `bind_params`, so the child environment fold is the same
//! fold), and the member carries its declared sign and width into the region.
//!
//! What still declines, keeping its pre-slice value: an interface reached
//! through a module PORT (`module m(ifc p)` — one oracle: iverilog rejects the
//! port), a modport path (`w.mp.hi`, loud in vita), an interface array (loud), a
//! width whose override is an ORACLE-SPLIT (`#(.W(P+P))` over `parameter P =
//! 3'd6`).
//!
//! ORACLES: iverilog 13.0 (`-g2012`) and verilator 5.052 (`--binary --timing`);
//! every value below was measured in both, and they agree on every cell unless a
//! cell says otherwise. PRE values are from a release binary built at the parent
//! commit.

use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_ifacereg_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("t.sv"), src).unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("t.sv")
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.contains("simulation ended"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// ① THE HEADLINE: a size cast over an interface member. PRE:
/// `fffffee0 00000080 ffffff90 ffffff90 fffffee0 00000051` — the 16-bit member
/// was right by accident (its own width filled the region), the two 8-bit ones
/// evaluated at 8.
#[test]
fn an_interface_member_in_a_size_cast_carries_its_declared_sign_and_width() {
    let out = run(r#"
interface ifc;
  logic signed [15:0] hi;
  logic [7:0] uh;
  logic signed [7:0] s8;
endinterface
module t;
  ifc w();
  logic signed [7:0] sq8 = -8'sd16;
  logic signed [31:0] r1, r2, r3, r4, r5, r6;
  initial begin
    w.hi = 16'sd18; w.uh = 8'd200; w.s8 = -8'sd9;
    #1;
    r1 = 16'(w.hi * sq8);
    r2 = 16'(w.uh * sq8);
    r3 = 16'(w.s8 * sq8);
    r4 = 32'(w.s8 * sq8);
    r5 = 16'(sq8 * w.hi);
    r6 = 8'(w.s8 * w.s8);
    $display("%08x %08x %08x %08x %08x %08x", r1, r2, r3, r4, r5, r6);
    #1 $finish;
  end
endmodule
"#);
    assert_eq!(out, "fffffee0 0000bb80 00000090 00000090 fffffee0 00000051");
}

/// ② The other two regions and the controls: a cast inside an INLINE body, a
/// CASE selector, and the plain-assignment / inline-assignment controls that
/// were already right. PRE: `0090 bb80 00000090 00000080 0 0 00001940 00001940
/// 00000009 16` (the inline cast at 8 bits; both case arms missed).
#[test]
fn an_inline_body_cast_and_a_case_selector_size_the_member_too() {
    let out = run(r#"
module t;
  ifc w();
  pifc #(.W(16)) pw();
  logic signed [7:0] sq8 = -8'sd16;
  logic signed [31:0] r1, r2, r3, r4, r5, r6, r7, r8;
  logic signed [15:0] y1, y2;
  function automatic logic signed [15:0] fb(input logic signed [7:0] a);
    logic signed [15:0] t;
    t = w.s8 * a;
    return t;
  endfunction
  function automatic logic signed [15:0] fc(input logic signed [7:0] a);
    return 16'(w.uh * a);
  endfunction
  initial begin
    w.hi = 16'sd18; w.uh = 8'd200; w.s8 = -8'sd9; w.u16 = 16'd300; pw.d = 16'd300;
    #1;
    y1 = w.s8 * sq8;
    y2 = w.uh * sq8;
    r1 = fb(sq8);
    r2 = fc(sq8);
    case (w.s8 * sq8)
      16'sd144: r3 = 1;
      default:  r3 = 0;
    endcase
    case (w.uh * sq8)
      16'hbb80: r4 = 1;
      default:  r4 = 0;
    endcase
    r5 = 16'(pw.d * sq8);
    r6 = 16'(w.u16 * sq8);
    r7 = 8'(w.s8 + w.hi);
    r8 = 32'($bits(pw.d));
    $display("%04x %04x %08x %08x %0d %0d %08x %08x %08x %0d", y1, y2, r1, r2, r3, r4, r5, r6, r7, r8);
    #1 $finish;
  end
endmodule
interface ifc;
  logic signed [15:0] hi;
  logic [7:0] uh;
  logic signed [7:0] s8;
  logic [15:0] u16;
endinterface
interface pifc #(parameter W = 8);
  logic [W-1:0] d;
endinterface
"#);
    assert_eq!(
        out,
        "0090 bb80 00000090 ffffbb80 1 1 00001940 00001940 00000009 16"
    );
}

/// ③ A PARAMETER width on the interface folds in the instance's environment
/// (default, a named and a positional override, an `int` parameter, signed and
/// unsigned). One cell keeps its PRE value by design: `pwpp` (`#(.W(P+P))` over
/// `parameter P = 3'd6` is an ORACLE-SPLIT — iverilog folds 12 bits, verilator
/// and vita's binder wrap to 4 — so the fold declines). PRE: `00000080 00000080
/// 0000bb80 00000080 00000080 ffffff90 ffffff90 00000090 bits=4`. Oracles:
/// `0000bb80 0000bb80 0000bb80 <split> 0000bb80 00000090 00000090 00000090
/// bits=<split>`.
#[test]
fn an_interface_parameter_width_folds_in_the_instance_environment() {
    let out = run(r#"
interface pifc #(parameter W = 8);
  logic [W-1:0] d;
  logic signed [W-1:0] sd;
endinterface
interface tifc #(parameter int W = 8);
  logic [W-1:0] d;
endinterface
module t;
  parameter P = 3'd6;
  pifc pwd();
  pifc #(.W(8)) pw8();
  pifc #(16) pw16();
  pifc #(.W(P+P)) pwpp();
  tifc tw();
  logic signed [7:0] sq8 = -8'sd16;
  logic signed [31:0] r1, r2, r3, r4, r5, r6, r7, r8;
  initial begin
    pwd.d = 8'd200; pw8.d = 8'd200; pw16.d = 16'd200; pwpp.d = 200; tw.d = 8'd200;
    pwd.sd = -8'sd9; pw8.sd = -8'sd9; pw16.sd = -16'sd9;
    #1;
    r1 = 16'(pwd.d * sq8);
    r2 = 16'(pw8.d * sq8);
    r3 = 16'(pw16.d * sq8);
    r4 = 16'(pwpp.d * sq8);
    r5 = 16'(tw.d * sq8);
    r6 = 16'(pwd.sd * sq8);
    r7 = 16'(pw8.sd * sq8);
    r8 = 16'(pw16.sd * sq8);
    $display("%08x %08x %08x %08x %08x %08x %08x %08x bits=%0d", r1, r2, r3, r4, r5, r6, r7, r8, $bits(pwpp.d));
    #1 $finish;
  end
endmodule
"#);
    assert_eq!(
        out,
        "0000bb80 0000bb80 0000bb80 00000080 0000bb80 00000090 00000090 00000090 bits=4"
    );
}

/// ④ Review control: a `defparam` inside an UNINSTANTIATED interface does not
/// drop the module environments (the `defparam` pre-scan reads modules only —
/// an instantiated interface with a `defparam` is E3009). Both oracles reject
/// the interface itself; the value is the one they and PRE give for the same
/// design without that line. The round-1 binary printed `0000xx80`.
#[test]
fn a_defparam_in_an_uninstantiated_interface_keeps_the_module_environment() {
    let out = run(r#"
module ch #(parameter W = 8);
  logic [W-1:0] d = 200;
endmodule
interface unused_ifc;
  parameter Z = 1;
  logic [7:0] uh;
  defparam Z = 2;
endinterface
module t;
  ch #(.W(8)) u8();
  logic signed [7:0] sq8 = -8'sd16;
  logic signed [31:0] r1;
  initial begin
    #1;
    r1 = 16'(u8.d * sq8);
    $display("R %08x bits=%0d", r1, $bits(u8.d));
    #1 $finish;
  end
endmodule
"#);
    assert_eq!(out, "R 0000bb80 bits=8");
}
