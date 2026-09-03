//! An ARRAY parameter of a struct/enum typedef — `localparam pmp_cfg_t P[N] =
//! '{ '{lock: 1'b0, mode: OFF, …}, … };` — ROADMAP §3 ⑤ ⓑ (ibex_pkg.sv:769), plus
//! the package-scope `parameter` that IEEE §6.20.1 makes a `localparam`
//! (ibex_pkg.sv:791, the package half of §3 ⑤ ⓒ).
//!
//! The A2a array-parameter path (`parse_array_param`) desugars a body array
//! parameter to the const variable-array decl the equivalent variable parses to;
//! it rejected a struct/enum typedef because nothing desugared the per-element
//! `'{…}`. Now a 1-D struct-typed array parameter binds its name exactly as the
//! variable does (`var_struct` + `struct_1d_array_vars`, so `P[i].member` and
//! `P[i] = '{…}` work) and `desugar_struct_array_init` turns every outer element
//! that is a `'{…}` into the field-width concat `P[i] = '{…}` already produces. The
//! SAME helper is applied to the variable twin's decl-init (`st_t V[2] = '{'{…},…}`)
//! and to a whole-array procedural `V = '{'{…},…}` — both were loud before — and
//! the 1-D struct-array binding is carried across `import pkg::*` / `import pkg::X`
//! like the scalar one.
//!
//! Every value here was measured on verilator 5.050 (iverilog 13.0 cannot parse an
//! unpacked array parameter at all: "sorry: unpacked array parameters are not
//! supported yet"). 230 cells in the slice's census, with a keyword-spelled control
//! twin (`logic [5:0] P[2] = '{6'b100011, …}`) per position.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_sap_{}_{n}", std::process::id()));
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

const ST: &str = "typedef struct packed { logic a; logic [4:0] b; } T;";

/// The census body: element values, `$size`/`$bits`, a runtime index, equality, a
/// member read (constant and runtime index), a struct-typed function actual and a
/// foreach sum. Same text for a parameter and for its variable twin.
fn body() -> &'static str {
    "  function automatic logic [5:0] gm(T s); return s.b; endfunction
  int i = 0; int acc = 0;
  initial begin
    i = 0; foreach (P[k]) acc = acc + P[k].b;
    $display(\"DIGEST=%b %b %0d %0d %0d %b %0d %0d %0d %0d %0d %0d %0d\",
      P[1], P[0], $size(P), $bits(P[0]), $bits(P), P[i], P[1]==P[0],
      P[1].b, P[0].b, P[1].a, P[i].b, gm(P[1]), acc);
    #1 $finish;
  end
"
}

const KEYED: &str = "'{ '{a: 1'b1, b: 5'd3}, '{a: 1'b0, b: 5'd7} }";
const KEYED_DIGEST: &str = "000111 100011 2 6 12 100011 0 7 3 0 3 7 10";

#[test]
fn a_struct_typed_array_localparam_with_keyed_element_patterns() {
    let src = format!(
        "module tb; {ST}\n  localparam T P[2] = {KEYED};\n{}endmodule\n",
        body()
    );
    assert_eq!(digest(&src), KEYED_DIGEST);
}

#[test]
fn positional_mixed_default_fill_and_unsized_element_patterns() {
    // positional `'{1'b1, 5'd3}` and a packed literal next to a pattern
    for v in [
        "'{ '{1'b1, 5'd3}, '{1'b0, 5'd7} }",
        "'{ 6'd35, '{a: 1'b0, b: 5'd7} }",
        "'{ '{a: 1, b: 3}, '{a: 0, b: 7} }",
    ] {
        let src = format!(
            "module tb; {ST}\n  localparam T P[2] = {v};\n{}endmodule\n",
            body()
        );
        assert_eq!(digest(&src), KEYED_DIGEST, "{v}");
    }
    // `'{default: 1'b1}` fills every member of the element (§10.9.1): a=1, b=00001
    let src = format!("module tb; {ST}\n  localparam T P[2] = '{{ '{{default: 1'b1}}, '{{a: 1'b0, b: 5'd7}} }};\n{}endmodule\n", body());
    assert_eq!(digest(&src), "000111 100001 2 6 12 100001 0 7 1 0 1 7 8");
    // a fill grows to the FIELD width: `'{a: '1, b: '0}` = 1_00000
    let src = format!("module tb; {ST}\n  localparam T P[2] = '{{ '{{a: '1, b: '0}}, '{{a: 1'b0, b: 5'd7}} }};\n{}endmodule\n", body());
    assert_eq!(digest(&src), "000111 100000 2 6 12 100000 0 7 0 0 0 7 7");
}

#[test]
fn the_ibex_pmp_cfg_shape_with_an_enum_member() {
    // verilator: 1 3 1 0 / 3 6 111101 001011
    let src = "module tb;
  typedef enum logic [1:0] {OFF=0, TOR=1, NA4=2, NAPOT=3} mode_e;
  typedef struct packed { logic lock; mode_e mode; logic exec; logic write; logic read; } cfg_t;
  localparam cfg_t P[3] = '{
    '{lock: 1'b0, mode: OFF, exec: 1'b0, write: 1'b0, read: 1'b0},
    '{lock: 1'b1, mode: NAPOT, exec: 1'b1, write: 1'b0, read: 1'b1},
    '{lock: 1'b0, mode: TOR, exec: 1'b0, write: 1'b1, read: 1'b1}
  };
  initial begin
    $display(\"DIGEST=%0d %0d %0d %0d %0d %0d %b %b\", P[1].lock, P[1].mode, P[2].write, P[0].read,
      $size(P), $bits(P[0]), P[1], P[2]);
    #1 $finish;
  end
endmodule
";
    assert_eq!(digest(src), "1 3 1 0 3 6 111101 001011");
}

#[test]
fn a_package_parameter_array_is_a_localparam_and_crosses_a_wildcard_import() {
    // IEEE §6.20.1: a `parameter` in a package cannot be overridden. ibex_pkg.sv:769
    // and :791 are both spelled `parameter`. verilator: 1 3 111101 000000 / 2 101000
    let src = "package pk;
  typedef enum logic [1:0] {OFF=0, TOR=1, NA4=2, NAPOT=3} mode_e;
  typedef struct packed { logic lock; mode_e mode; logic exec; logic write; logic read; } cfg_t;
  parameter cfg_t P[2] = '{
    '{lock: 1'b0, mode: OFF, exec: 1'b0, write: 1'b0, read: 1'b0},
    '{lock: 1'b1, mode: NAPOT, exec: 1'b1, write: 1'b0, read: 1'b1}
  };
  cfg_t V[2] = '{ '{lock: 1'b1, mode: TOR, exec: 1'b0, write: 1'b0, read: 1'b0},
                  '{lock: 1'b0, mode: NA4, exec: 1'b1, write: 1'b0, read: 1'b1} };
  parameter int X[2] = '{5, 6};
endpackage
module tb;
  import pk::*;
  initial begin
    $display(\"DIGEST=%0d %0d %b %b %0d %b %0d %0d\", P[1].lock, P[1].mode, P[1], pk::P[0],
      V[1].mode, V[0], X[1], pk::X[0]);
    #1 $finish;
  end
endmodule
";
    assert_eq!(digest(src), "1 3 111101 000000 2 101000 6 5");
}

#[test]
fn package_localparam_arrays_cross_explicit_and_wildcard_imports() {
    for imp in ["import p::*;", "import p::P; import p::T;"] {
        let src = format!(
            "package p; {ST}\n  localparam T P[2] = {KEYED};\nendpackage\nmodule tb; {imp}\n{}endmodule\n",
            body()
        );
        assert_eq!(digest(&src), KEYED_DIGEST, "{imp}");
    }
    // a derived package constant from an element, read through the import
    let src = format!(
        "package p; {ST}\n  localparam T P[2] = {KEYED};\n  localparam int X = P[1];\nendpackage\nmodule tb; import p::*;\n  initial begin $display(\"DIGEST=%0d %b\", X, P[1]); #1 $finish; end\nendmodule\n"
    );
    assert_eq!(digest(&src), "7 000111");
}

#[test]
fn an_enum_typed_array_parameter_and_a_struct_with_an_enum_member_in_case_items() {
    // verilator: 2 1 3 2 6 / hit1 / phit0
    let src = "module tb;
  typedef enum logic [1:0] {A, B, C, D} e_t;
  localparam e_t E[3] = '{A, C, B};
  typedef struct packed { logic lock; e_t m; } st_t;
  localparam st_t P[2] = '{ '{lock: 1'b1, m: C}, '{lock: 1'b0, m: D} };
  e_t x = C;
  initial begin
    $display(\"DIGEST=%0d %0d %0d %0d %0d\", E[1], E[2], P[1].m, P[0].m, $bits(P));
    case (x) E[1]: $display(\"hit1\"); E[2]: $display(\"hit2\"); default: $display(\"miss\"); endcase
    case (x) P[0].m: $display(\"phit0\"); default: $display(\"pmiss\"); endcase
    #1 $finish;
  end
endmodule
";
    let (out, rc) = run(src);
    assert_eq!(rc, Some(0), "{out}");
    assert!(out.contains("DIGEST=2 1 3 2 6\nhit1\nphit0\n"), "{out}");
    // the census body on an enum array: element, $size/$bits, runtime index, ==
    let src = "module tb; typedef enum logic [1:0] {A, B, C, D} T;
  localparam T P[2] = '{ C, B };
  int i = 0; int acc = 0;
  initial begin i = 0; foreach (P[k]) acc = acc + P[k];
    $display(\"DIGEST=%b %b %0d %0d %0d %b %0d %0d\", P[1], P[0], $size(P), $bits(P[0]), $bits(P), P[i], P[1]==P[0], acc);
    #1 $finish; end
endmodule
";
    assert_eq!(digest(src), "01 10 2 2 4 10 0 3");
}

#[test]
fn declared_dimension_direction_orders_the_elements() {
    // `[1:0]`: the first element is P[1]; `[0:1]` and `[2]`: the first is P[0]
    let src = format!(
        "module tb; {ST}\n  localparam T P[1:0] = {KEYED};\n{}endmodule\n",
        body()
    );
    assert_eq!(digest(&src), "100011 000111 2 6 12 000111 0 3 7 1 7 3 10");
    let src = format!(
        "module tb; {ST}\n  localparam T P[0:1] = {KEYED};\n{}endmodule\n",
        body()
    );
    assert_eq!(digest(&src), KEYED_DIGEST);
}

#[test]
fn an_outer_default_pattern_and_a_comma_list_and_a_generate_block() {
    // `'{default: '{…}}` — the element pattern is desugared, the outer `default:`
    // is elaborate's array arm (§10.9.1)
    let src = format!(
        "module tb; {ST}\n  localparam T P[2] = '{{default: '{{a: 1'b1, b: 5'd3}}}};\n{}endmodule\n",
        body()
    );
    assert_eq!(digest(&src), "100011 100011 2 6 12 100011 1 3 3 1 3 3 6");
    let src = format!(
        "module tb; {ST}\n  localparam T P[2] = {KEYED}, Q[1] = '{{ '{{a: 1'b1, b: 5'd9}} }};\n  initial begin $display(\"DIGEST=%b %b %b %0d\", P[1], P[0], Q[0], Q[0].b); #1 $finish; end\nendmodule\n"
    );
    assert_eq!(digest(&src), "000111 100011 101001 9");
    let src = format!(
        "module tb; {ST} function automatic logic [5:0] gm(T s); return s.b; endfunction
  if (1) begin : g
    localparam T P[2] = {KEYED};
  int i = 0; int acc = 0;
  initial begin
    i = 0; foreach (P[k]) acc = acc + P[k].b;
    $display(\"DIGEST=%b %b %0d %0d %0d %b %0d %0d %0d %0d %0d %0d %0d\",
      P[1], P[0], $size(P), $bits(P[0]), $bits(P), P[i], P[1]==P[0],
      P[1].b, P[0].b, P[1].a, P[i].b, gm(P[1]), acc);
    #1 $finish;
  end
  end
endmodule
"
    );
    assert_eq!(digest(&src), KEYED_DIGEST);
}

#[test]
fn a_signed_member_and_a_signed_struct() {
    // verilator: -3 -3 11101 10010 2
    let src = "module tb;
  typedef struct packed { logic lock; logic signed [3:0] v; } st_t;
  localparam st_t P[2] = '{ '{lock: 1'b1, v: -3}, '{lock: 1'b0, v: 4'sd5} };
  localparam st_t D[2] = '{ default: '{lock: 1'b1, v: 4'sd2} };
  initial begin
    $display(\"DIGEST=%0d %0d %b %b %0d\", P[0].v, $signed(P[0].v), P[0], D[1], D[0].v);
    #1 $finish;
  end
endmodule
";
    assert_eq!(digest(src), "-3 -3 11101 10010 2");
}

#[test]
fn runtime_index_function_actual_foreach_and_element_copy() {
    // verilator: 110 011 011 110 10 11 / 110 3 2 1 / 2 3
    let src = "module tb;
  typedef struct packed { logic lock; logic [1:0] mode; } st_t;
  localparam st_t P[1:0] = '{ '{lock: 1'b1, mode: 2'd2}, '{lock: 1'b0, mode: 2'd3} };
  localparam st_t Q[0:1] = '{ '{lock: 1'b1, mode: 2'd2}, '{lock: 1'b0, mode: 2'd3} };
  localparam int A[1:0] = '{10, 11};
  st_t v;
  int i;
  function automatic logic [1:0] gm(st_t s); return s.mode; endfunction
  initial begin
    $display(\"DIGEST=%b %b %b %b %0d %0d\", P[1], P[0], Q[1], Q[0], A[1], A[0]);
    i = 1; v = P[i];
    $display(\"%b %0d %0d %0d\", v, gm(P[0]), P[i].mode, P[1] == v);
    foreach (P[k]) $write(\"%0d \", P[k].mode); $display(\"\");
    #1 $finish;
  end
endmodule
";
    let (out, rc) = run(src);
    assert_eq!(rc, Some(0), "{out}");
    assert!(
        out.contains("DIGEST=110 011 011 110 10 11\n110 3 2 1\n2 3 \n"),
        "{out}"
    );
    // an element as an instance override value
    let src = format!(
        "module m #(parameter logic [5:0] Q = 0) (output logic [5:0] o); assign o = Q; endmodule
module tb; {ST}
  localparam T P[2] = {KEYED};
  logic [5:0] o1; m #(.Q(P[1])) u1(.o(o1));
  initial begin #1 $display(\"DIGEST=%b\", o1); #1 $finish; end
endmodule
"
    );
    assert_eq!(digest(&src), "000111");
}

#[test]
fn the_variable_twin_decl_init_whole_array_assign_and_element_assign() {
    // Same helper, same digest: a struct-array VARIABLE's `= '{ '{…}, … }` decl-init
    // and a whole-array procedural `V = '{…}` were loud before this slice.
    let src = format!(
        "module tb; {ST}\n  T P[2] = {KEYED};\n{}endmodule\n",
        body()
    );
    assert_eq!(digest(&src), KEYED_DIGEST);
    let src = format!(
        "module tb; {ST}\n  T P[2];\n  initial P = {KEYED};\n{}endmodule\n",
        body()
    );
    assert_eq!(digest(&src), KEYED_DIGEST);
    // verilator: 1 3 111101 000000 / 101010 010111
    let src = "module tb;
  typedef enum logic [1:0] {OFF=0, TOR=1, NA4=2, NAPOT=3} mode_e;
  typedef struct packed { logic lock; mode_e mode; logic exec; logic write; logic read; } cfg_t;
  cfg_t V[2] = '{ '{1'b0, OFF, 1'b0, 1'b0, 1'b0}, '{1'b1, NAPOT, 1'b1, 1'b0, 1'b1} };
  cfg_t W[2];
  initial begin
    $display(\"DIGEST=%0d %0d %b %b\", V[1].lock, V[1].mode, V[1], V[0]);
    W = '{ '{1'b1, TOR, 1'b0, 1'b1, 1'b0}, '{lock: 1'b0, mode: NA4, exec: 1'b1, write: 1'b1, read: 1'b1} };
    $display(\"%b %b\", W[0], W[1]);
    #1 $finish;
  end
endmodule
";
    let (out, rc) = run(src);
    assert_eq!(rc, Some(0), "{out}");
    assert!(
        out.contains("DIGEST=1 3 111101 000000\n101010 010111\n"),
        "{out}"
    );
    // element assign + element copy (pre-existing path, unchanged)
    let src = format!(
        "module tb; {ST}\n  T P[2] = {KEYED};\n  initial begin P[0] = '{{a: 1'b1, b: 5'd9}}; P[1] = P[0]; $display(\"DIGEST=%b %b\", P[1], P[0]); #1 $finish; end\nendmodule\n"
    );
    assert_eq!(digest(&src), "101001 101001");
    // a package struct-array VARIABLE through a wildcard import (binding carry)
    let src = format!(
        "package p; {ST}\n  T P[2] = {KEYED};\nendpackage\nmodule tb; import p::*;\n{}endmodule\n",
        body()
    );
    assert_eq!(digest(&src), KEYED_DIGEST);
}

#[test]
fn a_local_array_declaration_shadows_a_wildcard_imported_struct_array() {
    // IEEE §26.3: the local declaration wins in either order; verilator 000010 000001 2
    for (before, after) in [
        (
            "import p::*;",
            "localparam logic [5:0] P[2] = '{6'd1, 6'd2};",
        ),
        (
            "localparam logic [5:0] P[2] = '{6'd1, 6'd2};",
            "import p::*;",
        ),
    ] {
        let src = format!(
            "package p; {ST}\n  localparam T P[2] = {KEYED};\nendpackage\nmodule tb;\n  {before}\n  {after}\n  initial begin $display(\"DIGEST=%b %b %0d\", P[1], P[0], $size(P)); #1 $finish; end\nendmodule\n"
        );
        assert_eq!(digest(&src), "000010 000001 2", "{before} {after}");
    }
    let src = format!(
        "package p; {ST}\n  T P[2] = {KEYED};\nendpackage\nmodule tb; import p::*;\n  logic [5:0] P[2] = '{{6'd1, 6'd2}};\n  initial begin $display(\"DIGEST=%b %b\", P[1], P[0]); #1 $finish; end\nendmodule\n"
    );
    assert_eq!(digest(&src), "000010 000001");
}

#[test]
fn a_whole_element_folds_in_a_constant_context() {
    let src = format!(
        "module tb; {ST}\n  localparam T P[2] = {KEYED};\n  localparam int X = P[1]; localparam int Y = P[0];\n  initial begin $display(\"DIGEST=%0d %0d\", X, Y); #1 $finish; end\nendmodule\n"
    );
    assert_eq!(digest(&src), "7 35");
}

#[test]
fn v1_loud_limits_stay_loud() {
    // a module-body `parameter` array is overridable — §3 ⑤ ⓒ (no override channel
    // for aggregates); the package spelling is the localparam case above
    loud(
        &format!("module tb; {ST}\n  parameter T P[2] = {KEYED};\n  initial begin $display(\"DIGEST=%b\", P[1]); #1 $finish; end\nendmodule\n"),
        "an overridable array `parameter` is unsupported",
    );
    // a union typedef array parameter: packed elements work (as for the variable),
    // a member access on an element stays loud (the union overlay is not desugared)
    let src = "module tb; typedef union packed { logic [5:0] w; logic [5:0] v; } T;
  localparam T P[2] = '{ 6'd35, 6'd7 };
  initial begin $display(\"DIGEST=%b %b %0d\", P[1], P[0], $size(P)); #1 $finish; end
endmodule
";
    assert_eq!(digest(src), "000111 100011 2");
    loud(
        "module tb; typedef union packed { logic [5:0] w; logic [5:0] v; } T;
  localparam T P[2] = '{ 6'd35, 6'd7 };
  initial begin $display(\"DIGEST=%b\", P[1].w); #1 $finish; end
endmodule
",
        "error",
    );
    // a multi-dimensional struct array parameter
    loud(
        &format!("module tb; {ST}\n  localparam T P[2][2] = '{{ {KEYED}, {KEYED} }};\n  initial begin $display(\"DIGEST=%b\", P[1][0]); #1 $finish; end\nendmodule\n"),
        "multi-dimensional struct array parameter is unsupported",
    );
    // a member left unfilled / an element count mismatch (verilator rejects both)
    loud(
        &format!("module tb; {ST}\n  localparam T P[2] = '{{ '{{a: 1'b1}}, '{{a: 1'b0, b: 5'd7}} }};\n  initial begin $display(\"DIGEST=%b\", P[0]); #1 $finish; end\nendmodule\n"),
        "error",
    );
    loud(
        &format!("module tb; {ST}\n  localparam T P[2] = '{{ '{{a: 1'b1, b: 5'd3}} }};\n  initial begin $display(\"DIGEST=%b\", P[0]); #1 $finish; end\nendmodule\n"),
        "error",
    );
    // Residue (pre-existing on the keyword spelling too, see ROADMAP §3): a MEMBER
    // of an element in a constant context — `localparam int X = P[1].b;` — and
    // `$size(P)` there. Wording pin on the shared decline, not on a value.
    loud(
        &format!("module tb; {ST}\n  localparam T P[2] = {KEYED};\n  localparam int X = P[1].b;\n  initial begin $display(\"DIGEST=%0d\", X); #1 $finish; end\nendmodule\n"),
        "is not a constant",
    );
    loud(
        &format!("module tb; {ST}\n  localparam logic [5:0] P[2] = '{{6'b100011, 6'b000111}};\n  localparam int X = P[1][4:0];\n  initial begin $display(\"DIGEST=%0d\", X); #1 $finish; end\nendmodule\n"),
        "is not a constant",
    );
}

/// Pre-existing silent-wrong closed by the same change (review finding B1): an array
/// parameter of a NON-struct typedef took the KEYWORD's default signedness and
/// ignored the typedef's explicit `signed`/`unsigned` — `typedef bit signed [3:0]
/// tb2; localparam tb2 B[1] = '{4'hF};` read 15 (verilator -1), `typedef shortint
/// unsigned tsu; … '{16'hFFFF}` read -1 (verilator 65535). The typedef prefix's
/// folded sign is now passed as the explicit one. The variable twin was already
/// right; a keyword-spelled control twin is unchanged.
#[test]
fn a_typedef_array_parameter_keeps_the_typedefs_signedness() {
    let src = "module tb;
  typedef integer ti;            localparam ti  A[1] = '{-5};
  typedef bit signed [3:0] tb2;  localparam tb2 B[1] = '{4'hF};
  typedef shortint unsigned tsu; localparam tsu C[1] = '{16'hFFFF};
  typedef logic [3:0] tl;        localparam tl  D[1] = '{4'hF};
  typedef reg signed [7:0] tr;   localparam tr  E[1] = '{8'hF0};
  typedef longint unsigned tlu;  localparam tlu F[1] = '{64'hFFFF_FFFF_FFFF_FFFF};
  typedef byte unsigned tbu;     localparam tbu G[1] = '{8'hFF};
  tb2 VB[1] = '{4'hF};           localparam bit signed [3:0] KB[1] = '{4'hF};
  initial begin
    $display(\"DIGEST=%0d %0d %0d %0d %0d %0d %0d %0d %0d\", A[0], B[0], C[0], D[0], E[0], F[0], G[0], VB[0], KB[0]);
    #1 $finish;
  end
endmodule
";
    assert_eq!(
        digest(src),
        "-5 -1 65535 15 -16 18446744073709551615 255 -1 -1"
    );
}
