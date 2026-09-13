//! A user CALL as a leaf of a §11.6.1 region carries the callee's declared return
//! sign, so the region is widened like any other — on the inline body and on the
//! size cast.
//!
//! The size-cast sign walk (`ctx_signed_impl`, reused by the inline-body context of
//! §4.5.491) answered `None` for a `Call` from its `_` tail, and `None` stands the
//! WHOLE region down: `function [31:0] f; f = id8(a8) * b8;` printed `00000001`
//! for both oracles' `0000fe01`, `16'(ids8(s8) * q8)` printed `0020` for `0120`.
//! The `Call` arm now answers a single-segment call resolved by `lookup_func` with
//! the callee's declared return sign (`kind_signedness`); a real / realtime /
//! string return and a hierarchical or method call still decline. A frame call
//! cannot be named twice, so a signed region widens it through a context instead
//! of a sign fill (`e | 32'sd0`, one mention, per-bit so x survives) — before that
//! a region whose every leaf is a frame call folded at 8 bits.
//!
//! ORACLES: iverilog 13.0 (-g2012) and verilator 5.052 (--binary --timing); every
//! value below was measured in both except the x-carry cell (verilator is 2-state).
//! PRE values are from a release binary built at the parent commit.

use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_iclr_{}_{n}", std::process::id()));
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
    let _ = std::fs::remove_dir_all(&d);
    so
}

/// ① THE HEADLINE: inline and frame callees, unsigned and signed returns, a wider
/// return, a signed callee in an unsigned region, two calls in a sum, unary minus,
/// and the three size-cast twins.
#[test]
fn a_call_leaf_carries_its_declared_return_sign() {
    let o = run(r#"module t;
  logic [7:0] a8 = 8'hFF, b8 = 8'hFF, u8 = 8'hF7;
  logic signed [7:0] s8 = -9, q8 = -32;
  function [7:0] id8(input [7:0] v); id8 = v; endfunction
  function signed [7:0] ids8(input signed [7:0] v); ids8 = v; endfunction
  function automatic [7:0] fa8(input [7:0] v); fa8 = v; endfunction
  function automatic signed [7:0] fas8(input signed [7:0] v); fas8 = v; endfunction
  function [15:0] id16(input [7:0] v); id16 = v; endfunction
  function [31:0] f1; f1 = id8(a8) * b8; endfunction
  function [31:0] f2; f2 = ids8(s8) * q8; endfunction
  function [31:0] f3; f3 = fa8(a8) * b8; endfunction
  function [31:0] f4; f4 = fas8(s8) * q8; endfunction
  function [31:0] f5; f5 = id16(a8) * b8; endfunction
  function [31:0] f6; f6 = ids8(s8) * b8; endfunction
  function [31:0] f7; f7 = id8(a8) + id8(b8); endfunction
  function [31:0] f8; f8 = -ids8(s8); endfunction
  logic [15:0] c1, c2, c3;
  initial begin
    c1 = 16'(id8(a8) * b8); c2 = 16'(ids8(s8) * q8); c3 = 16'(fas8(s8) * q8);
    $display("F=%h %h %h %h %h %h %h %h C=%h %h %h", f1(), f2(), f3(), f4(), f5(), f6(), f7(), f8(), c1, c2, c3);
    #1 $finish;
  end
endmodule
"#);
    // PRE: `F=00000001 00000020 00000001 00000020 0000fe01 00000009 000000fe
    // 00000009 C=0001 0020 0020` — f5 (a wider return) and f8 (fits) were right.
    assert_eq!(
        o,
        "F=0000fe01 00000120 0000fe01 00000120 0000fe01 0000f609 000001fe 00000009 C=fe01 0120 0120"
    );
}

/// ② Regions whose EVERY leaf is a frame call, an `integer` and a package callee,
/// a call under a shift, and the module-scope `assign` / `always_comb` twins.
#[test]
fn frame_call_only_regions_and_the_module_twins() {
    let o = run(
        r#"package pk; function [7:0] pf(input [7:0] v); pf = v; endfunction endpackage
module t;
  import pk::pf;
  logic [7:0] a8 = 8'hFF, b8 = 8'hFF;
  logic signed [7:0] s8 = -9, q8 = -32;
  function automatic [7:0] fa8(input [7:0] v); fa8 = v; endfunction
  function automatic signed [7:0] fas8(input signed [7:0] v); fas8 = v; endfunction
  function integer fint(input [7:0] v); fint = v; endfunction
  function signed [7:0] ids8(input signed [7:0] v); ids8 = v; endfunction
  function [7:0] id8(input [7:0] v); id8 = v; endfunction
  function [31:0] g1; g1 = fa8(a8) + fa8(b8); endfunction
  function [31:0] g2; g2 = fint(a8) * b8; endfunction
  function [31:0] g3; g3 = pf(a8) * b8; endfunction
  function [31:0] g4; g4 = fas8(s8) * fas8(q8); endfunction
  function [31:0] g5; g5 = ids8(s8) * ids8(q8); endfunction
  function [31:0] g6; g6 = id8(a8) * 2; endfunction
  function [31:0] g7; g7 = (id8(a8) * b8) >> 4; endfunction
  function [31:0] g8; g8 = fas8(s8) + fas8(q8) + fas8(s8); endfunction
  logic [31:0] z1, z2;
  assign z1 = id8(a8) * b8;
  always_comb z2 = ids8(s8) * q8;
  initial begin
    #1 $display("G=%h %h %h %h %h %h %h %h Z=%h %h", g1(), g2(), g3(), g4(), g5(), g6(), g7(), g8(), z1, z2);
    #1 $finish;
  end
endmodule
"#,
    );
    // PRE refused the design (the `always_comb` twin was the §4.5.498 false-loud);
    // with that removed, `g4` printed `00000020` and `g8` `000000ce` — the
    // frame-call-only regions.
    assert_eq!(
        o,
        "G=000001fe 0000fe01 0000fe01 00000120 00000120 000001fe 00000fe0 ffffffce Z=0000fe01 00000120"
    );
}

/// ③ An x-bearing frame-call leaf keeps its x through the context widening
/// (iverilog; verilator is 2-state), and the size-cast twin of a frame-call
/// region.
#[test]
fn the_context_widening_preserves_x() {
    let o = run(r#"module t;
  logic [7:0] ux = 8'hFx;
  logic signed [7:0] s8 = -9, q8 = -32;
  function automatic signed [7:0] fas8(input signed [7:0] v); fas8 = v; endfunction
  function [31:0] f1; f1 = fas8(s8) * fas8(q8); endfunction
  function [31:0] f2; f2 = fas8(ux) * fas8(q8); endfunction
  logic [15:0] c1;
  initial begin
    c1 = 16'(fas8(s8) * fas8(q8));
    $display("F=%h %h C=%h", f1(), f2(), c1);
    #1 $finish;
  end
endmodule
"#);
    // PRE: `F=00000020 000000xx C=0020`.
    assert_eq!(o, "F=00000120 xxxxxxxx C=0120");
}

/// ④ ROUND-1 SOUNDNESS RESIDUE (closed): the scoped spelling `pk::f(…)` is a
/// two-segment path whose head is a PACKAGE, resolved in the package's table by
/// the lowering (`inline_pkg_function`); the sign walk, the opaque-leaf test and
/// the real-domain guard now resolve it the same way instead of treating it as a
/// hierarchical placeholder. PRE printed `F=00000020 00000020 00000001 C=0020`.
#[test]
fn a_scoped_package_call_leaf_carries_its_declared_return_sign() {
    let o = run(r#"package pk;
  function signed [7:0] pfs(input signed [7:0] v); pfs = v; endfunction
  function [7:0] pf(input [7:0] v); pf = v; endfunction
endpackage
module t;
  import pk::pfs;
  logic [7:0] a8 = 8'hFF, b8 = 8'hFF;
  logic signed [7:0] s8 = -9, q8 = -32;
  function [31:0] f1; f1 = pk::pfs(s8) * q8; endfunction
  function [31:0] f2; f2 = pfs(s8) * q8; endfunction
  function [31:0] f3; f3 = pk::pf(a8) * b8; endfunction
  logic [15:0] c1;
  initial begin
    c1 = 16'(pk::pfs(s8) * q8);
    $display("F=%h %h %h C=%h", f1(), f2(), f3(), c1);
    #1 $finish;
  end
endmodule
"#);
    assert_eq!(o, "F=00000120 00000120 0000fe01 C=0120");
}
