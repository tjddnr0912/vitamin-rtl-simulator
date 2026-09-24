//! A HIERARCHICAL leaf inside a static (inlined) function body is sized, signed
//! and typed by the instance module's declaration.
//!
//! A cross-instance read `u.lv` lowers to a placeholder `Signal{POISON_NET}` and
//! a cross-instance call `u.hf(x)` to `Call{POISON_FID}`; both are patched only
//! after every instance exists. The inline function lane (`fold_straight_line`,
//! `bind_formal_actual`) sizes every store and formal bind by the rhs's trusted
//! width, and a placeholder had none, so each rule kept its verbatim tail:
//! `function [3:0] f4; f4 = u.lv;` returned `f0` (no truncation), a body local
//! `bit [7:0] b; b = u.lv; f2 = b;` returned `f0` (no widening to 16), a tree
//! `u.lv + 1` kept 32 bits, and a `real u.r` was stored as its raw IEEE-754
//! bits. The declaration walk the size-cast lane already used
//! (`hier_leaf_net` / `hier_leaf_func`, plus `hier_leaf_real` for a real scalar)
//! now records the placeholder's shape when it is created, and the resolution
//! passes refuse a net or callee whose shape differs.
//!
//! What moved (every value measured in both oracles):
//! - store/return/formal-bind rules in the inline lane: truncation, widening of
//!   a body local, a tree folded to the return width, a narrowing formal;
//! - a real hierarchical leaf stored into an integral target rounds and narrows;
//! - `mg[u.k]` with a signed `u.k` = -1 selects element -1 (it was E4002 + `xx`);
//! - `$display("%h", u.r)` is E3009, as the local twin `%h` of a real is;
//! - `$bits` of a hierarchical leaf or of a function whose body reads one folds.
//!
//! What declines and keeps its previous behaviour: a path the declaration walk
//! does not follow (a generate scope, an upward or absolute path, an instance
//! array, `bind`, `defparam`, a typed parameter width), a declared width whose
//! parameter fold is not exact by construction (a unary operator, a narrow or
//! signed sized literal, `/ %`, a typed `int`/`integer` slot), a `real`-returning
//! hierarchical call, and a widening size cast of a bare signed hierarchical call
//! (`16'(u.hs(3))`, ROADMAP §2 — its sign fill would name the call twice).
//!
//! ORACLES: iverilog 13.0 (`-g2012`) and verilator 5.052 (`--binary --timing`);
//! they agree on every value pinned here (the `%h` refusal is vita's local-twin
//! rule; both oracles print a value there).

use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_raw(src: &str) -> (Option<i32>, String, String) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_hierinl_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("t.sv"), src).unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("t.sv")
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn run(src: &str) -> String {
    let (code, stdout, stderr) = run_raw(src);
    assert_eq!(code, Some(0), "{stderr}");
    stdout
        .lines()
        .filter(|l| !l.contains("simulation ended"))
        .collect::<Vec<_>>()
        .join("\n")
}

const CH: &str = "module ch; logic [7:0] lv = 8'hf0; logic signed [7:0] sv = -8'sd3; \
logic [15:0] wv = 16'hbeef; logic signed [2:0] k = -3'sd1; real r = 45.6; real nr = -45.6; \
function [7:0] hf(input [7:0] x); return x + 8'd1; endfunction \
function signed [7:0] hs(input [7:0] x); return -x; endfunction endmodule\n";

/// A body local stored from a hierarchical net, then returned: the local is 8
/// bits and the return widens it to 16 (s15 m_u8 f2 / m_u16 f2 / m_s1 f2). PRE
/// kept the leaf's own width: `f0`, `beef`, `01`.
#[test]
fn a_local_then_return_widens_to_the_return_width() {
    let o = run(&format!(
        "{CH}module top; ch u();
  function [15:0] f2; bit [7:0] b; b = u.lv; f2 = b; endfunction
  function [15:0] g2; bit [7:0] b; b = u.wv; g2 = b; endfunction
  function [15:0] s2; bit [7:0] b; b = u.sv; s2 = b; endfunction
  initial begin $display(\"%h %h %h %h\", f2(), g2(), s2(), {{f2(), 4'h1}}); #1 $finish; end
endmodule"
    ));
    assert_eq!(o, "00f0 00ef 00fd 00f01");
}

/// A tree over a hierarchical leaf folds to the return width (m_u8 f3 / f9):
/// `u.lv + 1` is `00f1`, `(u.lv) * 2` is `01e0`. PRE printed 32 bits.
#[test]
fn a_tree_over_a_hierarchical_leaf_folds_to_the_return_width() {
    let o = run(&format!(
        "{CH}module top; ch u();
  function [15:0] f3; f3 = u.lv + 1; endfunction
  function [15:0] f9; f9 = (u.lv) * 2; endfunction
  function [15:0] s3; s3 = u.sv + 1; endfunction
  initial begin $display(\"%h %h %h\", f3(), f9(), s3()); #1 $finish; end
endmodule"
    ));
    assert_eq!(o, "00f1 01e0 fffe");
}

/// A narrowing return and a narrowing formal bind truncate (m_u8 f4/f7, m_u16
/// f4/f7): `0`, `0`, `f`, `f`. PRE: `f0`, `f0`, `beef`, `beef`.
#[test]
fn a_narrowing_return_and_formal_truncate() {
    let o = run(&format!(
        "{CH}module top; ch u();
  function [3:0] f4; f4 = u.lv; endfunction
  function [3:0] w4; w4 = u.wv; endfunction
  function [3:0] f7(input [3:0] x); f7 = x; endfunction
  initial begin $display(\"%h %h %h %h\", f4(), f7(u.lv), w4(), f7(u.wv)); #1 $finish; end
endmodule"
    ));
    assert_eq!(o, "0 0 f f");
}

/// The widening controls keep their values (m_u8/m_s8 f1 f5 f6 f8 and the
/// automatic twins g1..g3).
#[test]
fn widening_controls_are_unchanged() {
    let o = run(&format!(
        "{CH}module top; ch u();
  function [15:0] f1; f1 = u.sv; endfunction
  function signed [15:0] f5; f5 = u.lv; endfunction
  function [15:0] f6(input [15:0] x); f6 = x; endfunction
  function signed [15:0] f8(input signed [15:0] x); f8 = x; endfunction
  function automatic [3:0] g2; g2 = u.lv; endfunction
  initial begin $display(\"%h %h %h %h %h\", f1(), f5(), f6(u.sv), f8(u.sv), g2()); #1 $finish; end
endmodule"
    ));
    assert_eq!(o, "fffd 00f0 fffd fffd 0");
}

/// A hierarchical CALL is sized by the callee's declared return (m_hf f2/f3/f4/f7,
/// m_hs f2/f4): PRE `f1 000000f2 f1 f1 fd fd`.
#[test]
fn a_hierarchical_call_is_sized_by_its_declared_return() {
    let o = run(&format!(
        "{CH}module top; ch u();
  function [15:0] f2; bit [7:0] b; b = u.hf(8'hf0); f2 = b; endfunction
  function [15:0] f3; f3 = u.hf(8'hf0) + 1; endfunction
  function [3:0] f4; f4 = u.hf(8'hf0); endfunction
  function [3:0] f7(input [3:0] x); f7 = x; endfunction
  function [15:0] s2; bit [7:0] b; b = u.hs(3); s2 = b; endfunction
  function [3:0] s4; s4 = u.hs(3); endfunction
  initial begin $display(\"%h %h %h %h %h %h\", f2(), f3(), f4(), f7(u.hf(8'hf0)), s2(), s4()); #1 $finish; end
endmodule"
    ));
    assert_eq!(o, "00f1 00f2 1 1 00fd d");
}

/// A `real` hierarchical leaf stored into an integral inline target rounds and
/// narrows (m_real2 / m_sreal2): 45.6 → `002e`, `e`; -45.6 → `ffd2`, `2`, and a
/// body local `bit [7:0]` gets `d2`. PRE stored the raw bits `4046cccccccccccd`
/// / `c046cccccccccccd`.
#[test]
fn a_real_hierarchical_leaf_rounds_into_an_integral_store() {
    let o = run(&format!(
        "{CH}module top; ch u();
  function [15:0] f1; f1 = u.r; endfunction
  function [3:0] f4; f4 = u.r; endfunction
  function [15:0] n1; n1 = u.nr; endfunction
  function [3:0] n4; n4 = u.nr; endfunction
  function [15:0] n2; bit [7:0] b; b = u.nr; n2 = b; endfunction
  function [15:0] f3; f3 = u.r + 1; endfunction
  function [3:0] f7(input [3:0] x); f7 = x; endfunction
  initial begin $display(\"%h %h %h %h %h %h %h\", f1(), f4(), n1(), n4(), n2(), f3(), f7(u.nr)); #1 $finish; end
endmodule"
    ));
    assert_eq!(o, "002e e ffd2 2 00d2 002f 2");
}

/// A signed hierarchical index seals by its sign (m_idx2): `mg[u.k]` with
/// `u.k = -1` over `reg [7:0] mg[-3:2]` is element -1, `9f`, directly and inside
/// an inline function. PRE: E4002 plus `xx`.
#[test]
fn a_signed_hierarchical_index_selects_its_negative_element() {
    let o = run(&format!(
        "{CH}module top; ch u();
  logic [7:0] mg [-3:2];
  function [7:0] fi; fi = mg[u.k]; endfunction
  initial begin
    for (int i = -3; i <= 2; i++) mg[i] = 8'ha0 + i;
    $display(\"%h %h\", mg[u.k], fi()); #1 $finish;
  end
endmodule"
    ));
    assert_eq!(o, "9f 9f");
}

/// `%h` of a real hierarchical leaf is refused like the local twin (m_loud2 /
/// m_loud3). iverilog prints `2e` and verilator `000000000000002e`; PRE
/// printed the raw bits `4046cccccccccccd` at exit 0.
#[test]
fn hex_format_of_a_real_hierarchical_leaf_is_refused() {
    let (code, _out, err) = run_raw(&format!(
        "{CH}module top; ch u();
  initial begin $display(\"h=%h\", u.r); #1 $finish; end
endmodule"
    ));
    assert_eq!(code, Some(1), "{err}");
    assert!(
        err.contains("binary/hex/octal format not defined on a real argument"),
        "{err}"
    );
}

/// `$bits` of a hierarchical leaf and of an inline function returning one folds
/// (s15 c_bits, h1_inline): every cell was E3009 "argument shape unsupported".
#[test]
fn bits_of_a_hierarchical_leaf_folds() {
    let o = run(&format!(
        "{CH}module top; ch u();
  function [11:0] fw; fw = u.lv; endfunction
  initial begin
    $display(\"%0d %0d %0d %0d %0d\", $bits(u.lv + 1'b1), $bits({{u.lv, u.sv}}), $bits(u.hs(1)), $bits(u.hs(1) + 1'b1), $bits(fw()));
    #1 $finish;
  end
endmodule"
    ));
    assert_eq!(o, "8 16 8 8 12");
}

/// Module-level stores and casts of a hierarchical leaf (s15 c_ctl2): `16'(u.sv)`
/// sign-extends (`fffd`, PRE `xxfd`), `int'(u.r)` rounds (`46`, PRE
/// `-858993459`), `8'(u.hs(3))` into 32 bits sign-extends (`fffffffd`, PRE
/// `000000fd`), `16'(u.hf(...))` has no `x` bits.
#[test]
fn casts_of_a_hierarchical_leaf_use_its_declared_shape() {
    let o = run(&format!(
        "{CH}module top; ch u();
  logic [15:0] a2; int ia; logic [31:0] d, e;
  initial begin
    a2 = 16'(u.sv); ia = int'(u.r); d = 8'(u.hs(3)); e = 16'(u.hf(8'h82));
    $display(\"%h %0d %h %h\", a2, ia, d, e); #1 $finish;
  end
endmodule"
    ));
    assert_eq!(o, "fffd 46 fffffffd 00000083");
}

/// A widening size cast of a bare SIGNED hierarchical call keeps its previous
/// route (s15 c_call5): `32'(u.hs(3))` into 32 bits is `fffffffd` as on both
/// oracles; the recorded width alone would zero-fill it (`000000fd`), the
/// ROADMAP §2 impure-operand sign residue that `32'(f())` has.
#[test]
fn a_widening_cast_of_a_signed_hierarchical_call_keeps_its_route() {
    let o = run(&format!(
        "{CH}module top; ch u();
  logic [31:0] a;
  initial begin a = 32'(u.hs(3)); $display(\"%h\", a); #1 $finish; end
endmodule"
    ));
    assert_eq!(o, "fffffffd");
}

/// A whole-rhs streaming concatenation of a hierarchical leaf, stored inside an
/// INLINE body into a target of a DIFFERENT width, is refused. Its width is now
/// known, but the inline lane right-justifies a stream stored into a wider
/// return (`fl = {<<4{loc}}` with a LOCAL `loc` prints `0000000001101100` where
/// verilator gives `0110110000000000`, s15 review p27), so admitting the
/// hierarchical twin would turn a refusal into that wrong value. The forward
/// spelling `{>>{u.lv}}` is refused there too (PRE printed `11000110`,
/// verilator `1100011000000000`, s15 review r02_fwd). An EQUAL-width inline
/// store needs no padding: `{>>{u.lv}}` into `[7:0]` is `36` and `{>>{u.w}}`
/// into `[15:0]` is `1234` (PRE and verilator, s15 review r6h), and the reverse
/// spellings print verilator's `6c 3412 163` (PRE refused them; s15
/// impl/r4/r4_eqrev2). A module or automatic-function position pads by
/// §11.4.14.3 and prints verilator's values (iverilog does not support
/// streaming).
#[test]
fn a_stream_of_a_hierarchical_leaf_stays_refused() {
    let (code, _out, err) = run_raw(&format!(
        "{CH}module top; ch u();
  function [15:0] fh; fh = {{<<4{{u.lv}}}}; endfunction
  initial begin $display(\"%b\", fh()); #1 $finish; end
endmodule"
    ));
    assert_eq!(code, Some(1), "{err}");
    assert!(
        err.contains("stored into a target of a different width inside a non-`automatic` function"),
        "{err}"
    );
    let (code, _out, err) = run_raw(&format!(
        "{CH}module top; ch u();
  function [15:0] fh; fh = {{>>{{u.lv}}}}; endfunction
  initial begin $display(\"%b\", fh()); #1 $finish; end
endmodule"
    ));
    assert_eq!(code, Some(1), "{err}");
    assert!(
        err.contains("stored into a target of a different width inside a non-`automatic` function"),
        "{err}"
    );
    let o = run(
        "module ch; logic [7:0] lv = 8'h36; logic [15:0] w = 16'h1234; endmodule
module top;
  ch u();
  function [7:0] se; se = {>>{u.lv}}; endfunction
  function [15:0] sw; sw = {>>{u.w}}; endfunction
  function [7:0] re; re = {<<{u.lv}}; endfunction
  function [15:0] rw; rw = {<<8{u.w}}; endfunction
  function [11:0] rc; rc = {<<4{u.lv, 4'h1}}; endfunction
  initial begin
    $display(\"%h %h %h %h %h\", se(), sw(), re(), rw(), rc());
    #1 $finish;
  end
endmodule",
    );
    assert_eq!(o, "36 1234 6c 3412 163");
    // Outside an inline body the recorded width pads by §11.4.14.3 (s15 review
    // p27 `gh`/`mh`, r02_fwd `mh`/`mc`; verilator's values).
    let o = run("module ch; logic [7:0] lv = 8'b1100_0110; endmodule
module top;
  ch u();
  function automatic [15:0] gh; gh = {<<4{u.lv}}; endfunction
  logic [15:0] mh, mf, mc;
  initial begin
    #1;
    mh = {<<4{u.lv}}; mf = {>>{u.lv}}; mc = {>>{u.lv, 4'h1}};
    $display(\"%b %b %b %b\", gh(), mh, mf, mc);
    #1 $finish;
  end
endmodule");
    assert_eq!(
        o,
        "0110110000000000 0110110000000000 1100011000000000 1100011000010000"
    );
}

/// A hierarchical SELECT in the inline lane is sized by what it selects (s15
/// g/c_sel2, s15 review q2b/q4g; both oracles): a part-select `u.a[11:4]` is 8
/// bits, an indexed part `u.a[k*4 +: 8]` 8 bits, a bit `u.sv[7]` 1 unsigned bit,
/// and an element of a one-dimensional array `u.sa[0]` / `u.ua[k]` has the
/// element's width and sign. PRE (c_sel2): `s1 s2 s3 s6 s7 sb` =
/// `ab bc 000000ac f1 f 0e`, and `$bits(u.sa[0])` was E3009.
#[test]
fn a_hierarchical_select_is_sized_by_what_it_selects() {
    let o = run("module ch; logic [11:0] a = 12'habc; logic signed [7:0] sv = -8'sd3;
  logic signed [7:0] sa [0:1]; logic [7:0] ua [0:1];
  initial begin sa[0] = -8'sd2; sa[1] = 8'sd5; ua[0] = 8'hf1; ua[1] = 8'h0e; end endmodule
module top;
  ch u();
  int k = 1;
  function [3:0] s1; s1 = u.a[11:4]; endfunction
  function [15:0] s2; bit [3:0] b; b = u.a[7:0]; s2 = b; endfunction
  function [15:0] s3; s3 = u.a[k*4 +: 8] + 1; endfunction
  function [15:0] s4; s4 = u.sa[0]; endfunction
  function [3:0] s6; s6 = u.ua[0]; endfunction
  function [15:0] s7; s7 = u.ua[0][7:4]; endfunction
  function [15:0] s8; s8 = u.sv[7]; endfunction
  function [3:0] sb; sb = u.ua[k]; endfunction
  initial begin
    #1;
    $display(\"%h %h %h %h %h %h %h %h %0d\", s1(), s2(), s3(), s4(), s6(), s7(), s8(), sb(), $bits(u.sa[0]));
    #1 $finish;
  end
endmodule");
    assert_eq!(o, "b 000c 00ac fffe 1 000f 0001 e 8");
}
