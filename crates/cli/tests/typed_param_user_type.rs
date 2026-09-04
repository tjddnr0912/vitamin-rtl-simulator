//! A parameter/localparam whose type prefix is a USER typedef — `localparam
//! exc_cause_t E = '{…};`, `parameter lfsr_seed_t S = …;`, `localparam p::u Q = …;`
//! — ROADMAP §3 ⑤ (the ibex entry blocker).
//!
//! `parse_param_prefix` knew only keyword prefixes, so every one of ibex's 50 typed
//! parameters was an E2002 at the parameter NAME. The prefix now resolves a typedef
//! name to the same `(kind, sign, range)` the equivalent variable declaration gets
//! (`parse_typed_decl`), binds a struct/enum-typed parameter name exactly like that
//! variable (member part-select desugar, `'{…}` positional/named pattern, enum
//! methods), and carries those bindings across `import pkg::*` / `import pkg::X` so
//! `ExcCauseIrqNm.lower_cause` reads in the importing module.
//!
//! Two pre-existing silent-wrongs the census exposed on the keyword spelling are pinned
//! at the end: a package's derived constant read a sibling's UNTRUNCATED initializer
//! (`localparam logic [3:0] P = 5'h1F; localparam W = P + 1;` folded 32), and a
//! wildcard import bound a package constant OVER a module-local variable of the same
//! name (IEEE §26.3 says the local declaration shadows).
//!
//! Every value here was measured on verilator 5.050; iverilog 13.0 agrees wherever it
//! parses the construct (it cannot parse a struct/enum-typed parameter at all, and
//! aborts on a struct parameter's member access). 227 cells in the slice's census.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_tput_{}_{n}", std::process::id()));
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
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (s, out.status.code())
}

fn digest(src: &str) -> String {
    let (out, rc) = run(src);
    assert_eq!(rc, Some(0), "expected exit 0, got {rc:?}:\n{out}");
    out.lines()
        .find_map(|l| l.strip_prefix("DIGEST="))
        .unwrap_or_else(|| panic!("no DIGEST line:\n{out}"))
        .to_string()
}

fn loud(src: &str, needle: &str) {
    let (out, rc) = run(src);
    assert_ne!(rc, Some(0), "expected a loud reject:\n{out}");
    assert!(out.contains(needle), "expected `{needle}` in:\n{out}");
}

const ST: &str = "typedef struct packed { logic a; logic [4:0] b; } st_t;";

#[test]
fn a_struct_typed_localparam_takes_a_named_and_a_positional_pattern() {
    // verilator: 3 7 100011 0 6 1 4
    let src = format!(
        "package p; {ST}
  localparam st_t P1 = '{{a: 1'b1, b: 5'd3}};
  localparam st_t P2 = '{{1'b0, 5'd7}};
  localparam int W = $bits(P1);
endpackage
module tb; import p::*;
  localparam st_t P3 = '{{a: 1'b1, b: 5'd0}};
  localparam logic [4:0] L = P1.b + 1;
  initial begin $display(\"DIGEST=%0d %0d %b %b %0d %0d %0d\", P1.b, P2.b, P1, P2.a, W, P3.a, L); #1 $finish; end
endmodule"
    );
    assert_eq!(digest(&src), "3 7 100011 0 6 1 4");
}

#[test]
fn a_struct_typed_localparam_takes_a_plain_vector_value() {
    // The row's own probe: verilator + iverilog 12.
    let src = format!(
        "package p; {ST} typedef logic [5:0] u;
  localparam u   Q = 6'd5;
  localparam st_t R = 6'd6;
  localparam bit V = 1'b1;
endpackage
module tb; import p::*; initial begin $display(\"DIGEST=%0d\", Q+R+V); #1 $finish; end endmodule"
    );
    assert_eq!(digest(&src), "12");
}

#[test]
fn the_ibex_shape_reads_a_package_parameter_member_in_an_importing_module() {
    // ibex_if_stage.sv:218 `irq_vec = ExcCauseIrqNm.lower_cause;` — verilator 31 0100000 ... .
    let src = "package ibex_pkg; typedef struct packed { logic irq_int; logic irq_ext; logic [4:0] lower_cause; } exc_cause_t;
  localparam exc_cause_t ExcCauseIrqNm = '{irq_ext: 1'b1, irq_int: 1'b0, lower_cause: 5'd31};
  localparam exc_cause_t ExcCauseEcallMMode = '{irq_ext: 1'b0, irq_int: 1'b0, lower_cause: 5'd11};
endpackage
module tb; import ibex_pkg::*; logic [4:0] irq_vec; exc_cause_t c;
  initial begin irq_vec = ExcCauseIrqNm.lower_cause; c = ExcCauseEcallMMode;
    $display(\"DIGEST=%0d %b %0d %0d\", irq_vec, c, c.lower_cause, c == ExcCauseIrqNm); #1 $finish; end
endmodule";
    assert_eq!(digest(src), "31 0001011 11 0");
}

#[test]
fn an_explicit_import_carries_only_the_named_binding() {
    let src = format!(
        "package p; {ST} localparam st_t P = '{{a: 1'b1, b: 5'd3}}; localparam st_t Q = '{{a: 1'b0, b: 5'd9}}; endpackage
module tb; import p::st_t; import p::P;
  initial begin $display(\"DIGEST=%0d %0d\", P.b, P.a); #1 $finish; end
endmodule"
    );
    assert_eq!(digest(&src), "3 1");
}

#[test]
fn an_explicit_import_wins_over_an_earlier_wildcard_for_the_layout_too() {
    // IEEE §26.8: `import q::*; import r::P;` — P is r's. Elaborate already gave
    // the explicit import the VALUE; the parser's layout binding must follow, or
    // r's value is decoded through q's struct (`P=122 lo=2 hi=12`, no simulator's
    // answer). verilator is order-sensitive here and takes q (its own defect:
    // on the plain-constant twin iverilog + vita say 9, verilator 5), so the
    // struct/enum cells are hand-IEEE + that two-oracle control.
    let src = "package q; typedef struct packed { logic [7:0] hi; logic [3:0] lo; } qt; localparam qt P = '{hi:8'hAA, lo:4'h5}; endpackage
package r; typedef struct packed { logic [3:0] lo; logic [7:0] hi; } rt; localparam rt P = '{lo:4'h1, hi:8'h22}; endpackage
module tb; import q::*; import r::P;
  initial begin #1 $display(\"DIGEST=%h %h %h\", P, P.lo, P.hi); #1 $finish; end
endmodule";
    assert_eq!(digest(src), "122 1 22");
    // The other order is the same answer (the wildcard never displaces).
    let src = src.replace("import q::*; import r::P;", "import r::P; import q::*;");
    assert_eq!(digest(&src), "122 1 22");
    // r's P is a plain vector: q's stale struct layout must not desugar `P[3:0]`.
    let src = "package q; typedef struct packed { logic [7:0] hi; logic [3:0] lo; } qt; localparam qt P = '{hi:8'hAA, lo:4'h5}; endpackage
package r; localparam logic [11:0] P = 12'h122; endpackage
module tb; import q::*; import r::P;
  initial begin #1 $display(\"DIGEST=%h %h\", P, P[3:0]); #1 $finish; end
endmodule";
    assert_eq!(digest(src), "122 2");
    // Enum: r's labels, r's `.name()`.
    let src = "package q; typedef enum logic [1:0] {A=1,B=2} qe; localparam qe P = B; endpackage
package r; typedef enum logic [2:0] {X=5,Y=6} re; localparam re P = Y; endpackage
module tb; import q::*; import r::P;
  initial begin #1 $display(\"DIGEST=%0d %s\", P, P.name()); #1 $finish; end
endmodule";
    assert_eq!(digest(src), "6 Y");
    // The two-oracle control (plain constants): iverilog + vita 9.
    let src = "package q; localparam int P = 5; endpackage
package r; localparam int P = 9; endpackage
module tb; import q::*; import r::P;
  initial begin #1 $display(\"DIGEST=%0d\", P); #1 $finish; end
endmodule";
    assert_eq!(digest(src), "9");
}

#[test]
fn a_local_declaration_after_the_import_unbinds_the_package_struct_name() {
    // Review F2: `import p::*` replayed p's struct VARIABLE `V` into the pattern
    // desugar set, and the module's later `logic [7:0] V [0:1]` never removed it —
    // `V = '{8'hAA, 8'hBB}` became a struct concat and was refused (PRE ran;
    // verilator aa bb). Every declaration site now unbinds its names first.
    let src = "package p; typedef struct packed { logic [3:0] a; logic [3:0] b; } st_t; st_t V = '{a:4'h1, b:4'h2}; endpackage
module tb; import p::*; logic [7:0] V [0:1];
  initial begin V = '{8'hAA, 8'hBB}; #1 $display(\"DIGEST=%h %h\", V[0], V[1]); #1 $finish; end
endmodule";
    assert_eq!(digest(src), "aa bb");
    // The same shape with the local array declared BEFORE the import (review R1:
    // a plain `logic` declaration writes no struct map, so the replay must consult
    // the module's declared names; both oracles 12 34).
    let src = "package p; typedef struct packed { logic [3:0] a; logic [3:0] b; } s_t; s_t V = '{a:4'h1, b:4'h2}; endpackage
module tb; logic [7:0] V [0:1]; import p::*;
  initial begin V = '{8'h12, 8'h34}; #1 $display(\"DIGEST=%h %h\", V[0], V[1]); #1 $finish; end
endmodule";
    assert_eq!(digest(src), "12 34");
    // A local struct VARIABLE named like a package TYPE being imported explicitly
    // keeps its own binding (review: the unconditional drop made this E3010).
    let src = "package r; typedef struct packed { logic [11:0] a; logic [3:0] b; } rs_t; endpackage
module tb; typedef struct packed { logic [3:0] a; logic [3:0] b; } ls_t; ls_t rs_t; import r::rs_t;
  initial begin rs_t = 8'h34; $display(\"DIGEST=%h %h\", rs_t.a, rs_t.b); #1 $finish; end
endmodule";
    assert_eq!(digest(src), "3 4");
}

#[test]
fn a_package_binding_of_an_imported_type_keeps_the_origin_layout() {
    // Review (soundness F1): p2 imports p1's `s_t`; its binding for `V` was captured
    // under the BARE name and re-resolved in tb against tb's own `s_t` (a different
    // layout) — `V.a` cut as `V[15:4]` out of an 8-bit net (`xx1`; both oracles 1).
    let src = "package p1; typedef struct packed { logic [3:0] a; logic [3:0] b; } s_t; endpackage
package p2; import p1::*; s_t V; localparam s_t Q = '{4'h1, 4'h2}; endpackage
module tb; typedef struct packed { logic [11:0] a; logic [3:0] b; } s_t; import p2::*;
  initial begin V = 8'h12; $display(\"DIGEST=%h %h %h %h\", V.a, V.b, Q.a, Q.b); #1 $finish; end
endmodule";
    assert_eq!(digest(src), "1 2 1 2");
    // The enum twin (`var_enum`): both oracles A1.
    let src = "package p1; typedef enum logic [1:0] {A1=1, B1=2} e_t; endpackage
package p2; import p1::*; e_t E = A1; endpackage
module tb; typedef enum logic [1:0] {A2=1, B2=2} e_t; import p2::*;
  initial begin $display(\"DIGEST=%s\", E.name()); #1 $finish; end
endmodule";
    assert_eq!(digest(src), "A1");
}

#[test]
fn a_generate_block_scopes_its_bindings_and_its_import_is_loud() {
    // Review F4 / soundness F3: a generate-block struct localparam (or an explicit
    // import inside the block) must not rebind the module-scope name: `S.a` at
    // module scope read through the block's layout (`xx3 4`; both oracles 3 4).
    let src = "module tb;
  typedef struct packed { logic [3:0] a; logic [3:0] b; } sa_t;
  typedef struct packed { logic [11:0] a; logic [3:0] b; } sb_t;
  sa_t S;
  if (1) begin : g
    localparam sb_t S = '{12'h111, 4'h2};
    initial $display(\"G=%h %h\", S.a, S.b);
  end
  initial begin S = 8'h34; #1 $display(\"DIGEST=%h %h\", S.a, S.b); #1 $finish; end
endmodule";
    let (out, rc) = run(src);
    assert_eq!(rc, Some(0), "{out}");
    assert!(out.contains("G=111 2"), "{out}");
    assert!(out.contains("DIGEST=3 4"), "{out}");
    // An `import` inside a generate block was silently ignored (PRE read q's 5
    // where both oracles read r's 9) — loud now, never a mixed answer.
    let src = "package q; localparam int P = 5; endpackage
package r; localparam int P = 9; endpackage
module tb; import q::*;
  if (1) begin : g
    import r::P;
    initial $display(\"DIGEST=%0d\", P);
  end
  initial begin #1 $finish; end
endmodule";
    loud(
        src,
        "an import inside a generate block is not applied in v1",
    );
    // A REDUNDANT block import (the module already imports the package) keeps
    // running (review F5: PRE ran it), and an import written directly in a bare
    // `generate … endgenerate` REGION is a module-scope import (§27.3; both
    // oracles 9; with a prior `import q::*` the explicit region import wins,
    // iverilog 9 — verilator's 7 is its order-sensitivity, see above).
    let src = "package p; localparam int K = 7; endpackage
module tb; import p::*;
  generate if (1) begin : g import p::K; initial $display(\"DIGEST=%0d\", K); end endgenerate
  initial begin #1 $finish; end
endmodule";
    assert_eq!(digest(src), "7");
    let src = "package q; localparam int P = 7; endpackage
package r; localparam int P = 9; endpackage
module tb; import q::*;
  generate import r::P; endgenerate
  initial begin $display(\"DIGEST=%0d\", P); #1 $finish; end
endmodule";
    assert_eq!(digest(src), "9");
}

#[test]
fn a_struct_member_of_a_parameter_folds_as_a_width() {
    let src = format!(
        "package p; {ST} localparam st_t P = '{{a: 1'b1, b: 5'd3}}; endpackage
module tb; import p::*;
  localparam int W = P.b + 1; logic [W-1:0] v = '1;
  initial begin $display(\"DIGEST=%0d %0d %0d\", W, v, $bits(v)); #1 $finish; end
endmodule"
    );
    assert_eq!(digest(&src), "4 15 4");
}

#[test]
fn a_struct_parameter_is_a_case_item_and_an_override_value() {
    let src = format!(
        "package p; {ST} localparam st_t P1 = '{{a: 1'b1, b: 5'd3}}; localparam st_t P2 = '{{a: 1'b0, b: 5'd3}}; endpackage
module m #(parameter logic [5:0] K = 0) (output logic [5:0] o); assign o = K; endmodule
module tb; import p::*; st_t v; int r; logic [5:0] o; m #(.K(P1)) u(.o(o));
  initial begin v = '{{a: 1'b0, b: 5'd3}}; case (v) P1: r = 1; P2: r = 2; default: r = 0; endcase
    #1 $display(\"DIGEST=%0d %0d %0d %0d\", r, v == P2, o, o == P1); $finish; end
endmodule"
    );
    assert_eq!(digest(&src), "2 1 35 1");
}

#[test]
fn a_header_parameter_of_a_scoped_struct_type_binds_and_overrides() {
    // verilator: 6 21 (X.b * K: 3*2, 7*3). The override of a struct-typed header
    // parameter by a plain vector; a `'{…}` override at the instance stays loud (below).
    let src = format!(
        "package p; {ST} endpackage
module m #(parameter p::st_t X = '{{1'b1, 5'd3}}, parameter int K = 2) (output logic [7:0] o);
  assign o = X.b * K;
endmodule
module tb; import p::*;
  logic [7:0] o1, o2;
  m u1(.o(o1));
  m #(.X(6'd7), .K(3)) u2(.o(o2));
  initial begin #1 $display(\"DIGEST=%0d %0d\", o1, o2); $finish; end
endmodule"
    );
    assert_eq!(digest(&src), "6 21");
}

#[test]
fn an_assignment_pattern_as_an_instance_override_stays_loud() {
    // The instance override has no struct target type at parse time — never a
    // silent default: E3009 names the parameter.
    let src = format!(
        "package p; {ST} endpackage
module m #(parameter p::st_t X = '{{1'b1, 5'd3}}) (output logic [7:0] o); assign o = X.b; endmodule
module tb; logic [7:0] o; m #(.X('{{1'b0, 5'd7}})) u(.o(o));
  initial begin #1 $display(\"DIGEST=%0d\", o); $finish; end
endmodule"
    );
    loud(&src, "override of parameter `X` is not a constant");
}

#[test]
fn an_enum_typed_localparam_carries_the_label_and_its_methods() {
    // verilator: 2 B 2 2
    let src = "module tb;
  typedef enum logic [1:0] {A=1, B=2, C=3} e_t;
  localparam e_t E = B;
  localparam int N = E;
  initial begin $display(\"DIGEST=%0d %s %0d %0d\", E, E.name(), N, $bits(E)); #1 $finish; end
endmodule";
    assert_eq!(digest(src), "2 B 2 2");
    // Imported from a package: the enum method's synthetic function is generated
    // in the importing module (verilator: C B 1 — `C.next()` wraps to the first label, §6.19.5.2).
    let src = "package p; typedef enum logic [1:0] {A=1, B=2, C=3} e_t; localparam e_t P = C; e_t v = B; endpackage
module tb; import p::*;
  initial begin $display(\"DIGEST=%s %s %0d\", P.name(), v.name(), P.next()); #1 $finish; end
endmodule";
    assert_eq!(digest(src), "C B 1");
}

#[test]
fn vector_signed_and_atom_typedefs_keep_their_declared_width_and_sign() {
    // verilator + iverilog: -1 -5 255 0 32 ; and 1 2 63 0 for the comma list.
    let src = "module tb;
  typedef logic signed [7:0] s8; typedef int myint; typedef logic [7:0] u8;
  localparam s8 S = -1; localparam myint K = -5; localparam u8 U = -1; localparam S2 = S + 1;
  initial begin $display(\"DIGEST=%0d %0d %0d %0d %0d\", S, K, U, S2, $bits(K)); #1 $finish; end
endmodule";
    assert_eq!(digest(src), "-1 -5 255 0 32");
    let src = "module tb; typedef logic [5:0] u;
  localparam u A = 6'd1, B = 6'd2, C = 6'd63; localparam u D = C + 1;
  initial begin $display(\"DIGEST=%0d %0d %0d %0d\", A, B, C, D); #1 $finish; end
endmodule";
    assert_eq!(digest(src), "1 2 63 0");
    // byte / longint typedefs are the atom's fixed width, not a "packed dimension".
    let src = "module tb; typedef byte b_t; typedef longint l_t;
  localparam b_t P = 9'h100; localparam l_t L = 64'hFFFF_FFFF_FFFF_FFFF; localparam W = P + 1;
  initial begin $display(\"DIGEST=%0d %0d %0d %0d\", P, $bits(P), L, W); #1 $finish; end
endmodule";
    assert_eq!(digest(src), "0 8 -1 1");
}

#[test]
fn a_scoped_and_a_chained_typedef_resolve_in_the_prefix() {
    let src = format!(
        "package p; typedef logic [5:0] u; {ST} endpackage
module tb;
  localparam p::u Q = 6'd9;
  localparam p::st_t R = '{{a: 1'b0, b: 5'd2}};
  typedef p::u u2;
  localparam u2 S = 9'h1FF;
  initial begin $display(\"DIGEST=%0d %0d %0d %0d\", Q, R.b, S, $bits(S)); #1 $finish; end
endmodule"
    );
    assert_eq!(digest(&src), "9 2 63 6");
}

#[test]
fn a_header_continuation_named_like_a_typedef_stays_a_value() {
    // `, U = 6'd4` continues the group even though `U` is a typedef name in tb.
    let src = "module m #(parameter logic [5:0] T = 6'd3, U = 6'd4) (output logic [7:0] o);
  assign o = T + U;
endmodule
module tb; typedef logic [5:0] U;
  logic [7:0] o; m u(.o(o));
  initial begin #1 $display(\"DIGEST=%0d\", o); $finish; end
endmodule";
    assert_eq!(digest(src), "7");
    let src = "package p; typedef logic [5:0] u; endpackage
module m import p::*; #(parameter u A = 6'd1, B = 9'h1FF) (output logic [63:0] o);
  assign o = A + B;
endmodule
module tb; logic [63:0] o; m u(.o(o));
  initial begin #1 $display(\"DIGEST=%0d\", o); $finish; end
endmodule";
    assert_eq!(digest(src), "64");
}

#[test]
fn a_generate_scope_localparam_takes_a_typedef_prefix() {
    let src = format!(
        "module tb; {ST}
  if (1) begin : g
    localparam st_t P = '{{a: 1'b1, b: 5'd3}}; localparam W = P + 1;
    initial begin $display(\"DIGEST=%b %0d %0d %0d\", P, $bits(P), P.b, W); #1 $finish; end
  end
endmodule"
    );
    assert_eq!(digest(&src), "100011 6 3 36");
}

#[test]
fn an_array_parameter_of_a_vector_typedef_keeps_the_element_width() {
    let src = "module tb; typedef logic [5:0] T;
  localparam T A[2] = '{6'd1, 9'h1FF};
  initial begin $display(\"DIGEST=%0d %0d %0d\", A[0], A[1], $bits(A[1])); #1 $finish; end
endmodule";
    assert_eq!(digest(src), "1 63 6");
}

#[test]
fn the_v1_limits_are_loud_not_silent() {
    // A multi-dimensional packed typedef (ibex_pkg.sv:742 `lfsr_perm_t`) was the
    // next rung — §4.5.412 (`packed_md_param.rs`) declares the parameter flat and
    // rewrites `P[1]` to the element's part-select; verilator 5.050: `12345 1a`.
    assert_eq!(
        digest(
            "module tb; typedef logic [3:0][4:0] pt; localparam pt P = 20'h12345;
  initial begin $display(\"DIGEST=%h %h\", P, P[1]); #1 $finish; end endmodule"
        ),
        "12345 1a"
    );
    // An array parameter of a struct typedef (ibex_pkg.sv:769 `PmpCfgRst`) was the
    // next rung — §4.5.411 (`struct_array_param.rs`) desugars the nested `'{'{…}}`
    // per element; verilator 5.050: 3.
    assert_eq!(
        digest(&format!(
            "module tb; {ST}
  localparam st_t A[2] = '{{'{{a: 1'b1, b: 5'd3}}, '{{1'b0, 5'd7}}}};
  initial begin $display(\"DIGEST=%0d\", A[0].b); #1 $finish; end endmodule"
        )),
        "3"
    );
    // An unpacked struct typedef is not a scalar parameter type.
    loud(
        "module tb; typedef struct { logic a; logic [4:0] b; } us_t; localparam us_t U = '{a: 1'b1, b: 5'd3};
  initial begin $display(\"DIGEST=%0d\", U.b); #1 $finish; end endmodule",
        "E2002",
    );
}

#[test]
fn a_package_derived_constant_reads_its_sibling_at_the_declared_width() {
    // Pre-existing, keyword spelling: verilator + iverilog 15 16 1 2 (was 15 32 1
    // 4294967298 — the package fold bound the i64 walk's unlimited value).
    let src = "package p; localparam logic [3:0] P = 5'h1F; localparam W = P + 1;
  localparam int I = 40'h1_0000_0001; localparam WI = I + 1; endpackage
module tb; import p::*; initial begin $display(\"DIGEST=%0d %0d %0d %0d\", P, W, I, WI); #1 $finish; end endmodule";
    assert_eq!(digest(src), "15 16 1 2");
}

#[test]
fn a_local_declaration_shadows_a_wildcard_import() {
    // Pre-existing (IEEE §26.3): verilator + iverilog 42 7 35 (a local VARIABLE `P`
    // read the package's 35). A header parameter, a package variable and a >64-bit
    // package constant are shadowed the same way: 3 1 9 5.
    let src =
        "package p; localparam logic [5:0] P = 6'd35; localparam logic [5:0] Q = 6'd9; endpackage
module tb; import p::*; logic [5:0] P = 6'd42; localparam logic [5:0] Q = 6'd7;
  initial begin $display(\"DIGEST=%0d %0d %0d\", P, Q, p::P); #1 $finish; end endmodule";
    assert_eq!(digest(src), "42 7 35");
    let src = "package p; localparam logic [5:0] P = 6'd35; logic [5:0] V = 6'd9; localparam logic [127:0] WD = 128'h1; endpackage
module m #(parameter P = 3) (output logic [7:0] o); import p::*; assign o = P; endmodule
module tb; import p::*; logic [5:0] V = 6'd1; logic [7:0] o; m u(.o(o)); logic [7:0] WD = 8'd5;
  initial begin #1 $display(\"DIGEST=%0d %0d %0d %0d\", o, V, p::V, WD); $finish; end endmodule";
    assert_eq!(digest(src), "3 1 9 5");
    // A local struct variable shadowing an imported struct PARAMETER of the same
    // name desugars against the LOCAL layout (verilator 5 2).
    let src = format!(
        "package p; {ST} localparam st_t P = '{{a: 1'b1, b: 5'd3}}; endpackage
module tb; import p::*; typedef struct packed {{ logic [2:0] x; logic [2:0] y; }} L; L P = '{{x: 3'd5, y: 3'd2}};
  initial begin $display(\"DIGEST=%0d %0d\", P.x, P.y); #1 $finish; end
endmodule"
    );
    assert_eq!(digest(&src), "5 2");
}
