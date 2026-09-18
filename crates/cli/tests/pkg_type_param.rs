//! §3.b: a package `parameter type PT = <type>` used scope-qualified as `p::PT`
//! (ROADMAP §5.2 row 1), and the literal-range registration every NON-overridable
//! type parameter shares with it.
//!
//! Before this slice every `p::PT` use was E2002 ("expected identifier, found
//! '::'") at the parse: the `endpackage` twin pass registered a `pkg::t` typedef
//! for `ModuleItem::Typedef` items only, and the type-parameter desugar emits
//! `PT$w` / `PT$s` parameters plus a BARE typedef — which `restore_scope_unit`
//! drops with every other bare name a unit adds. Both oracles run all of it.
//!
//! The second half is the DIMS the typedef carries. `[PT$w-1:0]` is a width, not a
//! range: it names a constant an importing module never declared, and it loses the
//! declared LSB even inside its own module — `localparam type L = logic [8:1]; L u;`
//! read `u[1]` as bit 0 and `$low(u)` as 0 where both oracles read 1 and 1 (X4
//! below, silent-wrong before this slice). A non-overridable parameter of a
//! concrete type has no override to follow, so it now registers the DECLARED range.
//! The OVERRIDABLE `parameter type T = logic [8:1]` half keeps `[T$w-1:0]` and is
//! out of scope here (it needs a one-dimensional `T$p0a/b` carrier; ROADMAP §2).
//!
//! Every pinned value was measured on iverilog 13.0 (`-g2012`) AND verilator 5.052
//! (`--binary --timing`); the `c*` cells are the same designs with `typedef … PT;`
//! in place of `parameter type PT = …`, which ran correctly before this slice and
//! must stay byte-identical.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_pkgtp_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    let all =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    (all, out.status.code())
}

/// The design's own `$display` lines, in the order the run printed them.
fn said(out: &str, tag: &str) -> Vec<String> {
    out.lines()
        .filter(|l| l.starts_with(tag))
        .map(|l| l.to_string())
        .collect()
}

fn prints(src: &str, tag: &str, want: &[&str]) {
    let (out, rc) = run(src);
    assert_eq!(rc, Some(0), "{out}");
    assert_eq!(said(&out, tag), want, "{out}");
}

fn is_loud(src: &str, needle: &str) {
    let (out, rc) = run(src);
    assert_ne!(rc, Some(0), "expected a refusal\n{out}");
    assert!(out.contains(needle), "expected `{needle}` in\n{out}");
}

const P1: &str = "package p; parameter type PT = logic [7:0]; endpackage\nmodule top; p::PT w; initial begin w = 8'hab; #1 $display(\"P1 w=%h bits=%0d s=%0d\", w, $bits(p::PT), $bits(w)); $finish; end endmodule\n";
const P2: &str = "package p; localparam type PT = logic [3:0]; endpackage\nmodule top; p::PT w; initial begin w = 4'h9 + 4'h9; #1 $display(\"P2 w=%h bits=%0d\", w, $bits(p::PT)); $finish; end endmodule\n";
const P3: &str = "package p; parameter type PT = logic signed [7:0]; endpackage\nmodule top; p::PT w; int i; initial begin w = -3; i = w; #1 $display(\"P3 w=%h i=%0d lt=%0d\", w, i, w < 0); $finish; end endmodule\n";
const P4: &str = "package p; parameter type PT = logic [1:0][3:0]; endpackage\nmodule top; p::PT v; initial begin v = 8'h5a; #1 $display(\"P4 v1=%h v0=%h bits=%0d\", v[1], v[0], $bits(p::PT)); $finish; end endmodule\n";
const P5: &str = "package p; parameter type PT = logic [7:0] [0:2]; endpackage\nmodule top; p::PT a; initial begin a[0]=1; a[1]=2; a[2]=3; #1 $display(\"P5 a2=%0d size=%0d bits=%0d\", a[2], $size(a), $bits(p::PT)); $finish; end endmodule\n";
const P6: &str = "package p; parameter type PT = logic [7:0]; endpackage\nmodule top; import p::*; PT w; initial begin w = 8'hcd; #1 $display(\"P6 w=%h bits=%0d\", w, $bits(PT)); $finish; end endmodule\n";
const P7: &str = "package p; parameter type PT = logic [7:0]; parameter type PT2 = PT; endpackage\nmodule top; p::PT2 w; initial begin w = 8'h77; #1 $display(\"P7 w=%h bits=%0d\", w, $bits(p::PT2)); $finish; end endmodule\n";
const P8: &str = "package p; localparam W = 12; parameter type PT = logic [W-1:0]; endpackage\nmodule top; p::PT w; initial begin w = 12'hfff; #1 $display(\"P8 w=%h bits=%0d\", w, $bits(p::PT)); $finish; end endmodule\n";
const P9: &str = "package p; parameter type PT = bit [3:0]; endpackage\nmodule top; p::PT w; initial begin w = 4'bx01x; #1 $display(\"P9 w=%b bits=%0d\", w, $bits(p::PT)); $finish; end endmodule\n";
const P10: &str = "package p; parameter type PT = logic [7:0]; endpackage\npackage q; parameter type PT = logic [15:0]; endpackage\nmodule top; p::PT a; q::PT b; initial begin a = 16'h1234; b = 16'h1234; #1 $display(\"P10 a=%h b=%h ba=%0d bb=%0d\", a, b, $bits(p::PT), $bits(q::PT)); $finish; end endmodule\n";
const P11: &str = "package p; parameter type PT = int; endpackage\nmodule top; p::PT w; initial begin w = -5; #1 $display(\"P11 w=%0d bits=%0d\", w, $bits(p::PT)); $finish; end endmodule\n";
const P12: &str = "package p; parameter type PT = logic [7:0]; endpackage\nmodule m #(parameter type T = p::PT)(output T o); assign o = 8'h3c; endmodule\nmodule top; logic [7:0] x; m u(.o(x)); initial begin #1 $display(\"P12 x=%h\", x); $finish; end endmodule\n";
const P13: &str = "package p; parameter type PT = logic [7:0]; endpackage\nmodule top;\nfunction p::PT f(input p::PT a); return a + 8'd1; endfunction\ninitial begin #1 $display(\"P13 f=%h\", f(8'hfe)); $finish; end endmodule\n";
const P14: &str = "package p; parameter type PT = logic [7:0]; endpackage\nmodule top; typedef p::PT my_t; my_t w; initial begin w = 8'h42; #1 $display(\"P14 w=%h bits=%0d\", w, $bits(my_t)); $finish; end endmodule\n";
const P15: &str = "package p; parameter type PT = logic [7:0]; endpackage\nmodule top; logic [15:0] x = 16'habcd; initial begin #1 $display(\"P15 c=%h\", p::PT'(x)); $finish; end endmodule\n";
const P16: &str = "package p; parameter type PT = logic [7:0]; endpackage\nmodule top; typedef struct packed { p::PT a; logic [3:0] b; } s_t; s_t s; initial begin s = 12'hab5; #1 $display(\"P16 a=%h b=%h bits=%0d\", s.a, s.b, $bits(s_t)); $finish; end endmodule\n";
const P17: &str = "package p; parameter type PT = logic [7:0]; endpackage\nmodule m(input p::PT i, output p::PT o); assign o = i + 8'd1; endmodule\nmodule top; logic [7:0] x; m u(.i(8'h10), .o(x)); initial begin #1 $display(\"P17 x=%h\", x); $finish; end endmodule\n";
const P18: &str = "package p; parameter type PT = logic [7:0]; endpackage\npackage q; parameter type QT = p::PT; endpackage\nmodule top; q::QT w; initial begin w = 8'h99; #1 $display(\"P18 w=%h bits=%0d\", w, $bits(q::QT)); $finish; end endmodule\n";
const P19: &str = "package p; localparam W = 4; typedef logic [W-1:0] n_t; parameter type PT = n_t; parameter type P2 = logic [W-1:0][1:0]; endpackage\nmodule top; p::PT w; p::P2 v; initial begin w = 4'hf; v = 8'h5a; #1 $display(\"P19 w=%h bits=%0d v0=%h bits2=%0d\", w, $bits(p::PT), v[0], $bits(p::P2)); $finish; end endmodule\n";
const P20: &str = "package p; parameter type PT = logic [7:0]; localparam PT K = 8'h7e; function PT g(input PT a); return a ^ K; endfunction endpackage\nmodule top; p::PT w; initial begin w = p::g(8'h01); #1 $display(\"P20 w=%h K=%h\", w, p::K); $finish; end endmodule\n";
const P21: &str = "package p; parameter type PT = logic [7:0]; endpackage\nmodule top; logic [p::PT$w-1:0] w; initial begin w = 8'h11; #1 $display(\"P21 w=%h\", w); $finish; end\ndefparam p::PT$w = 16;\nendmodule\n";
const P22: &str = "package p; parameter type PT = logic [7:0]; endpackage\nmodule top; p::PT q[$]; p::PT arr [0:1]; initial begin q.push_back(8'h12); q.push_back(8'h34); arr[1] = 8'h56; #1 $display(\"P22 q1=%h n=%0d a1=%h\", q[1], q.size(), arr[1]); $finish; end endmodule\n";
const P23: &str = "package p; parameter type PT = logic [8:1]; endpackage\nmodule top; p::PT v; initial begin v = 8'h81; #1 $display(\"P23 v1=%b v8=%b lo=%0d hi=%0d\", v[1], v[8], $low(v), $high(v)); $finish; end endmodule\n";
const C1: &str = "package p; typedef logic [7:0] PT; endpackage\nmodule top; p::PT w; initial begin w = 8'hab; #1 $display(\"C1 w=%h bits=%0d s=%0d\", w, $bits(p::PT), $bits(w)); $finish; end endmodule\n";
const C2: &str = "package p; typedef logic [1:0][3:0] PT; endpackage\nmodule top; p::PT v; initial begin v = 8'h5a; #1 $display(\"C2 v1=%h v0=%h bits=%0d\", v[1], v[0], $bits(p::PT)); $finish; end endmodule\n";
const C3: &str = "package p; typedef logic signed [7:0] PT; endpackage\nmodule top; p::PT w; int i; initial begin w = -3; i = w; #1 $display(\"C3 w=%h i=%0d lt=%0d\", w, i, w < 0); $finish; end endmodule\n";
const C5: &str = "package p; typedef logic [7:0] [0:2] PT; endpackage\nmodule top; p::PT a; initial begin a[0]=1; a[1]=2; a[2]=3; #1 $display(\"C5 a2=%0d size=%0d bits=%0d\", a[2], $size(a), $bits(p::PT)); $finish; end endmodule\n";
const C6: &str = "package p; typedef logic [7:0] PT; endpackage\nmodule top; import p::*; PT w; initial begin w = 8'hcd; #1 $display(\"C6 w=%h bits=%0d\", w, $bits(PT)); $finish; end endmodule\n";
const C8: &str = "package p; localparam W = 12; typedef logic [W-1:0] PT; endpackage\nmodule top; p::PT w; initial begin w = 12'hfff; #1 $display(\"C8 w=%h bits=%0d\", w, $bits(p::PT)); $finish; end endmodule\n";
const C10: &str = "package p; typedef logic [7:0] PT; endpackage\npackage q; typedef logic [15:0] PT; endpackage\nmodule top; p::PT a; q::PT b; initial begin a = 16'h1234; b = 16'h1234; #1 $display(\"C10 a=%h b=%h ba=%0d bb=%0d\", a, b, $bits(p::PT), $bits(q::PT)); $finish; end endmodule\n";
const C12: &str = "package p; typedef logic [7:0] PT; endpackage\nmodule m #(parameter type T = p::PT)(output T o); assign o = 8'h3c; endmodule\nmodule top; logic [7:0] x; m u(.o(x)); initial begin #1 $display(\"C12 x=%h\", x); $finish; end endmodule\n";
const C13: &str = "package p; typedef logic [7:0] PT; endpackage\nmodule top;\nfunction p::PT f(input p::PT a); return a + 8'd1; endfunction\ninitial begin #1 $display(\"C13 f=%h\", f(8'hfe)); $finish; end endmodule\n";
const C14: &str = "package p; typedef logic [7:0] PT; endpackage\nmodule top; typedef p::PT my_t; my_t w; initial begin w = 8'h42; #1 $display(\"C14 w=%h bits=%0d\", w, $bits(my_t)); $finish; end endmodule\n";
const C15: &str = "package p; typedef logic [7:0] PT; endpackage\nmodule top; logic [15:0] x = 16'habcd; initial begin #1 $display(\"C15 c=%h\", p::PT'(x)); $finish; end endmodule\n";
const C16: &str = "package p; typedef logic [7:0] PT; endpackage\nmodule top; typedef struct packed { p::PT a; logic [3:0] b; } s_t; s_t s; initial begin s = 12'hab5; #1 $display(\"C16 a=%h b=%h bits=%0d\", s.a, s.b, $bits(s_t)); $finish; end endmodule\n";
const C17: &str = "package p; typedef logic [7:0] PT; endpackage\nmodule m(input p::PT i, output p::PT o); assign o = i + 8'd1; endmodule\nmodule top; logic [7:0] x; m u(.i(8'h10), .o(x)); initial begin #1 $display(\"C17 x=%h\", x); $finish; end endmodule\n";
const C18: &str = "package p; typedef logic [7:0] PT; endpackage\npackage q; typedef p::PT QT; endpackage\nmodule top; q::QT w; initial begin w = 8'h99; #1 $display(\"C18 w=%h bits=%0d\", w, $bits(q::QT)); $finish; end endmodule\n";
const M1: &str = "module top; localparam type LT = logic [7:0]; localparam W = 12; localparam type LW = logic [W-1:0]; LT w; logic [15:0] x = 16'habcd;\ntypedef struct packed { LT a; logic [3:0] b; } s_t; s_t s; typedef LT my_t; my_t y;\nfunction LT f(input LT a); return a + 8'd1; endfunction\ninitial begin w = 8'hab; s = 12'hab5; y = 8'h42; #1 $display(\"M1 w=%h bits=%0d bw=%0d c=%h a=%h b=%h sb=%0d y=%h f=%h\", w, $bits(LT), $bits(LW), LT'(x), s.a, s.b, $bits(s_t), y, f(8'hfe)); $finish; end endmodule\n";
const M2: &str = "module m(input logic [7:0] i, output logic [7:0] o); localparam type LT = logic [7:0]; LT t; assign t = i + 8'd1; assign o = t; endmodule\nmodule top; logic [7:0] x; m u(.i(8'h10), .o(x)); initial begin #1 $display(\"M2 x=%h\", x); $finish; end endmodule\n";
const X4: &str = "module top; localparam type L = logic [8:1]; L u; initial begin u = 8'h81; #1 $display(\"X4 u1=%b u8=%b ulo=%0d uhi=%0d bits=%0d\", u[1], u[8], $low(u), $high(u), $bits(L)); $finish; end endmodule\n";
const L1: &str = "package p; parameter type PT = logic [7:0]; endpackage\nmodule top; PT w; initial begin w = 8'hab; #1 $display(\"L1 w=%h\", w); $finish; end endmodule\n";
const L2: &str = "package p; parameter type PT = logic [7:0]; endpackage\nmodule top; logic [PT$w-1:0] w; initial begin w = 8'hab; #1 $display(\"L2 w=%h\", w); $finish; end endmodule\n";
const L3: &str = "package p; parameter type PT = logic [7:0]; endpackage\nmodule top; typedef logic [3:0] PT; PT w; initial begin w = 8'hab; #1 $display(\"L3 w=%h bits=%0d\", w, $bits(PT)); $finish; end endmodule\n";

const N13: &str = "module top;\n  localparam W = 8;\n  localparam type L = logic [W-1:0];\n  generate if (1) begin : g\n    localparam W = 4;\n    L v;\n    initial begin v = '1; #1 $display(\"N13 lb=%0d v=%h\", $bits(v), v); $finish; end\n  end endgenerate\n  initial #2000 $finish;\nendmodule\n";
const N17: &str = "module m #(parameter int W = 8) ();\n  localparam type L = logic [W-1:0];\n  generate if (1) begin : g\n    localparam W = 3;\n    L v;\n    initial begin v = '1; #1 $display(\"N17 lb=%0d v=%h\", $bits(v), v); end\n  end endgenerate\nendmodule\nmodule top; m #(.W(12)) u(); initial #50 $finish; endmodule\n";
const S1: &str = "parameter type PT = logic [8:1];\nmodule top;\n  PT v;\n  initial begin\n    v = 8'hAB;\n    $display(\"S1 v1=%0d lo=%0d hi=%0d bits=%0d\", v[1], $low(v), $high(v), $bits(v));\n    #1 $finish;\n  end\nendmodule\n";
const S23: &str = "module dummy; endmodule\nparameter type PT = logic [8:1];\nmodule top;\n  PT v;\n  initial begin v = 8'hAB; $display(\"S23 v1=%0d lo=%0d hi=%0d\", v[1], $low(v), $high(v)); #1 $finish; end\nendmodule\n";
const S11: &str = "parameter type PT = logic [15:0];\npackage p; parameter int PT$w = 3; endpackage\nmodule top;\n  p::PT v;\n  initial begin v = 16'h1234; $display(\"S11 v=%0h bits=%0d\", v, $bits(v)); #1 $finish; end\nendmodule\n";
// ───────────────── the scoped type, in every declaration shape ─────────────────

#[test]
fn a_package_type_parameter_names_a_type_scope_qualified() {
    // `p::PT w;` — the declaration, `$bits(p::PT)` and the net's own width.
    prints(P1, "P1", &["P1 w=ab bits=8 s=8"]);
    // `localparam type` in a package is the same non-overridable type (§6.20.1);
    // the 4-bit wrap (`4'h9 + 4'h9` = `2`) is the declared width doing its work.
    prints(P2, "P2", &["P2 w=2 bits=4"]);
    // the SHAPE rides with it: signed, and 2-state `bit`.
    prints(P3, "P3", &["P3 w=fd i=-3 lt=1"]);
    prints(P9, "P9", &["P9 w=0010 bits=4"]);
    // an ATOM default (`int`) — 32 bits, signed, no declared range.
    prints(P11, "P11", &["P11 w=-5 bits=32"]);
    // the DECLARED range, not `[PT$w-1:0]`: `logic [8:1]` keeps its LSB.
    prints(P23, "P23", &["P23 v1=1 v8=1 lo=1 hi=8"]);
}

#[test]
fn a_package_type_parameter_reaches_every_container_a_typedef_does() {
    // a tf return type and formal, a size cast, a packed-struct member, a port,
    // a `typedef` alias of the scoped name, a queue and an unpacked array.
    prints(P13, "P13", &["P13 f=ff"]);
    prints(P15, "P15", &["P15 c=cd"]);
    prints(P16, "P16", &["P16 a=ab b=5 bits=12"]);
    prints(P17, "P17", &["P17 x=11"]);
    prints(P14, "P14", &["P14 w=42 bits=8"]);
    prints(P22, "P22", &["P22 q1=34 n=2 a1=56"]);
    // …and a module type parameter's DEFAULT (`parameter type T = p::PT`).
    prints(P12, "P12", &["P12 x=3c"]);
}

#[test]
fn a_package_type_parameter_carries_its_dimensions_and_aliases() {
    // two packed dimensions, and an unpacked one.
    prints(P4, "P4", &["P4 v1=5 v0=a bits=8"]);
    prints(P5, "P5", &["P5 a2=3 size=8 bits=24"]);
    // dims naming the package's OWN constant: the twin re-spells `W` as `p::W`,
    // so the extent folds at a use site that never imported the package.
    prints(P8, "P8", &["P8 w=fff bits=12"]);
    // an alias of another type parameter IN the package, and one whose default is
    // a package typedef over a package constant, plus a 2-D sibling.
    prints(P7, "P7", &["P7 w=77 bits=8"]);
    prints(P19, "P19", &["P19 w=f bits=4 v0=2 bits2=8"]);
    // a CHAIN: `package q; parameter type QT = p::PT;`.
    prints(P18, "P18", &["P18 w=99 bits=8"]);
    // two packages with the SAME type-parameter name — the scoped twin keeps them
    // apart (a bare-keyed registry would answer the last one registered).
    prints(P10, "P10", &["P10 a=34 b=1234 ba=8 bb=16"]);
    // `import p::*` copies the twin back to the bare name.
    prints(P6, "P6", &["P6 w=cd bits=8"]);
    // the package's OWN body using its type parameter (a constant of that type and
    // a function over it), read back through `p::g` / `p::K`.
    prints(P20, "P20", &["P20 w=7f K=7e"]);
}

// ───────────────── the non-overridable literal-range registration ─────────────────

#[test]
fn a_non_overridable_type_parameter_registers_the_declared_dims() {
    // X4 was SILENT-WRONG before this slice: `u1=0 u8=x ulo=0 uhi=7` against
    // `u1=1 u8=1 ulo=1 uhi=8` in both oracles, because `[L$w-1:0]` = `[7:0]`
    // discarded the declared `[8:1]`.
    prints(X4, "X4", &["X4 u1=1 u8=1 ulo=1 uhi=8 bits=8"]);
    // M1 was LOUD before it (a packed-struct member cannot fold `LT$w`): a
    // `localparam type` now folds at parse everywhere a typedef does — `$bits`,
    // a symbolic-width sibling, a cast, a struct member, a `typedef` alias, a tf.
    prints(
        M1,
        "M1",
        &["M1 w=ab bits=8 bw=12 c=cd a=ab b=5 sb=12 y=42 f=ff"],
    );
    // a `localparam type` in a module that HAS ports (the pre-slice path).
    prints(M2, "M2", &["M2 x=11"]);
}

// ───────────────────────────── loud, in all three tools ─────────────────────────────

#[test]
fn the_carriers_are_not_a_scoped_defparam_target() {
    // `defparam p::PT$w = 16;` — iverilog and verilator both refuse the syntax.
    is_loud(P21, "VITA-E2002");
}

#[test]
fn a_package_type_parameter_does_not_leak_its_bare_name() {
    // The bare `PT` / `PT$w` a package body registers are unit-scoped: a later
    // module that never imported the package must not see either. Both oracles
    // refuse both (verilator: "Can't find typedef/interface: 'PT'" and "Can't find
    // definition of variable: 'PT$w'"), and vita refused both before this slice
    // too — the twin registration must not turn either into a value.
    is_loud(L1, "VITA-E2002");
    is_loud(L2, "VITA-E3009");
    // …and a module-local typedef of the same name is the module's own type.
    prints(L3, "L3", &["L3 w=b bits=4"]);
}

// ───────── controls: the package TYPEDEF twin of every design above ─────────

#[test]
fn a_control_package_typedef_is_unchanged() {
    prints(C1, "C1", &["C1 w=ab bits=8 s=8"]);
    prints(C2, "C2", &["C2 v1=5 v0=a bits=8"]);
    prints(C3, "C3", &["C3 w=fd i=-3 lt=1"]);
    prints(C5, "C5", &["C5 a2=3 size=8 bits=24"]);
    prints(C6, "C6", &["C6 w=cd bits=8"]);
    prints(C8, "C8", &["C8 w=fff bits=12"]);
    prints(C10, "C10", &["C10 a=34 b=1234 ba=8 bb=16"]);
    prints(C12, "C12", &["C12 x=3c"]);
    prints(C13, "C13", &["C13 f=ff"]);
    prints(C14, "C14", &["C14 w=42 bits=8"]);
    prints(C15, "C15", &["C15 c=cd"]);
    prints(C16, "C16", &["C16 a=ab b=5 bits=12"]);
    prints(C17, "C17", &["C17 x=11"]);
    prints(C18, "C18", &["C18 w=99 bits=8"]);
}

// ───────── round 2: the folded bound, the positive stem set, the unit scope ─────────

#[test]
fn a_declared_bound_is_folded_so_an_inner_scope_cannot_capture_it() {
    // vita's type dimensions are NAME-keyed and re-resolved at each USE, so storing
    // the declared range as the parsed expression let a generate-local `W` capture
    // it: `$bits(L)` read 4 (N13) and 3 (N17) where both oracles read 8 and 12.
    // `[T$w-1:0]` was immune because a `$` carrier cannot be shadowed; a FOLDED
    // literal bound is immune because it names nothing.
    prints(N13, "N13", &["N13 lb=8 v=ff"]);
    // …including a bound over a module HEADER parameter under an override, whose
    // value is a constant at the declaration point but not a literal in the source.
    prints(N17, "N17", &["N17 lb=12 v=fff"]);
}

#[test]
fn a_unit_scope_type_parameter_is_a_localparam() {
    // IEEE §6.20.1: a `parameter` outside a module header cannot be overridden. The
    // unit-scope arm rewrote `kind` only AFTER parsing the item, so the group saw an
    // overridable `parameter` and froze `[PT$w-1:0]` — `$low` 0 / `$high` 7 where
    // both oracles read 1 / 8. Position must not matter: the same declaration after
    // a module (S23) was wrong while the one after a package was right only because
    // `in_package` leaked past `endpackage`.
    prints(S1, "S1", &["S1 v1=1 lo=1 hi=8 bits=8"]);
    prints(S23, "S23", &["S23 v1=1 lo=1 hi=8"]);
}

#[test]
fn a_package_twin_needs_this_bodys_own_type_parameter() {
    // `package p; parameter int PT$w = 3;` declares no type at all. Reading the
    // stems off `<stem>$w` + `type_params` accepted it, because every container
    // seeds `type_params` from the unit scope and a unit-scope `parameter type PT`
    // is in it — so `p::PT` silently became a 3-bit type. Both oracles reject the
    // program (verilator: "Can't find typedef/interface: 'PT'").
    is_loud(S11, "VITA-E2002");
}
