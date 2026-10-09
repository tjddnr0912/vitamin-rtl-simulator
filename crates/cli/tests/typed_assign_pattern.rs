//! §10.9 (§5.2 row 36): a typed assignment pattern `T'{…}` — row-9 census root R24
//! (OpenTitan `keymgr_dpe_pkg::extract_metadata_from_slot` returns
//! `keymgr_dpe_metadata_slot_t'{…}`; aes, hmac, kmac and otbn reach it), which was
//! E2002 `expected ';' after return, found '''`.
//!
//! The pattern lowers through the untyped packed-struct pattern's own rule against
//! `T`, so its value is what `r = '{…};` stores into a variable `r` of type `T`, and
//! it stands wherever an expression does. Oracles: verilator 5.052 `--binary` and
//! sv2v 0.0.13 → iverilog 13.0 (iverilog itself has no typed-pattern syntax). Where
//! sv2v contradicts Icarus Verilog on the UNTYPED twin of the same pattern — an
//! `'x` fill, an unsized element, an x or z into a 2-state member — it is not an
//! oracle on that axis, and the expected value is verilator's and the untyped
//! twin's in iverilog (and in vita before this row).

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_typat_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("-Wno-W1017")
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (s, out.status.code())
}

/// The `A …` / `B …` lines, in order, joined by `|`.
fn lines(out: &str) -> String {
    out.lines()
        .filter(|l| l.starts_with("A ") || l.starts_with("B "))
        .collect::<Vec<_>>()
        .join("|")
}

fn value(name: &str, src: &str, expect: &str) {
    let (out, rc) = run(src);
    assert_eq!(rc, Some(0), "{name}: expected exit 0:\n{out}");
    assert_eq!(lines(&out), expect, "{name}:\n{out}");
}

fn loud(name: &str, src: &str, needle: &str) {
    let (out, rc) = run(src);
    assert_eq!(rc, Some(1), "{name}: expected exit 1:\n{out}");
    assert!(
        out.contains(needle),
        "{name}: expected `{needle}` in:\n{out}"
    );
}

const S_T: &str = "  typedef struct packed { logic [3:0] a; logic [3:0] b; } s_t;\n";

fn module_with(body: &str) -> String {
    format!("module t;\n{S_T}{body}endmodule\n")
}

#[test]
fn return_and_package_function() {
    // The row-9 repro: a function returning `T` returns a typed pattern.
    value(
        "repro",
        &module_with(
            "  function automatic s_t f(logic [3:0] x); return s_t'{x, ~x}; endfunction
  s_t r;
  initial begin r = f(4'h3); #1 $display(\"A r=%h\", r); $finish; end
",
        ),
        "A r=3c",
    );
    // The OpenTitan shape: a package function returns a typed pattern of members of
    // its formal, written one per line.
    value(
        "package_function",
        "package p;
  typedef struct packed { logic [1:0] pol; logic [2:0] stage; logic v; logic [7:0] ver; } meta_t;
  typedef struct packed { logic [15:0] key; logic [1:0] pol; logic [2:0] stage; logic v; logic [7:0] ver; } slot_t;
  function automatic meta_t extract(slot_t slot);
    return meta_t'{
      slot.pol,
      slot.stage,
      slot.v,
      slot.ver
    };
  endfunction
endpackage
module t;
  import p::*;
  slot_t s; meta_t m;
  initial begin s = {16'hdead, 2'b10, 3'b101, 1'b1, 8'h5a}; m = extract(s); #1 $display(\"A m=%h\", m); $finish; end
endmodule
",
        "A m=2b5a",
    );
    // In a constant function: the parameter and a range that reads it fold.
    value(
        "constant_function",
        &module_with(
            "  function automatic s_t mk(int k); return s_t'{k[3:0], ~k[3:0]}; endfunction
  localparam s_t P = mk(3);
  logic [P.a:0] w;
  initial begin #1 $display(\"A P=%h bits=%0d\", P, $bits(w)); $finish; end
",
        ),
        "A P=3c bits=4",
    );
}

#[test]
fn pattern_forms_and_type_spellings() {
    // PROBE_CATALOG §4.5.571's ibex grounding cell: `exc_cause_t` as `ibex_pkg`
    // declares it (`irq_int` first), into the struct and into a 10-bit vector.
    value(
        "ibex_exc_cause",
        "module t;
  typedef struct packed { logic irq_int; logic irq_ext; logic [4:0] lower_cause; } exc_cause_t;
  exc_cause_t e; logic [9:0] w;
  initial begin e = exc_cause_t'{irq_ext: 1'b0, irq_int: 1'b1, lower_cause: 5'd3}; w = exc_cause_t'{irq_ext: 1'b0, irq_int: 1'b1, lower_cause: 5'd3};
    #1 $display(\"A e=%b w=%b\", e, w); $finish; end
endmodule
",
        "A e=1000011 w=0001000011",
    );
    value(
        "keyed_and_wider_target",
        "module t;
  typedef struct packed { logic irq_ext; logic irq_int; logic [4:0] lower_cause; } exc_cause_t;
  exc_cause_t e; logic [9:0] w;
  initial begin e = exc_cause_t'{irq_ext: 1'b0, irq_int: 1'b1, lower_cause: 5'd3}; w = exc_cause_t'{irq_ext: 1'b1, irq_int: 1'b0, lower_cause: 5'd7};
    #1 $display(\"A e=%b w=%b\", e, w); $finish; end
endmodule
",
        "A e=0100011 w=0001000111",
    );
    value(
        "default",
        "module t;
  typedef struct packed { logic [3:0] a; logic [1:0] b; logic c; } s_t;
  s_t r, q;
  initial begin r = s_t'{default: '1}; q = s_t'{b: 2'b01, default: '0}; #1 $display(\"A r=%b q=%b\", r, q); $finish; end
endmodule
",
        "A r=1111111 q=0000010",
    );
    value(
        "nested",
        "module t;
  typedef struct packed { logic [1:0] h; logic l; } in_t;
  typedef struct packed { in_t i; logic [3:0] z; } s_t;
  s_t r, q;
  initial begin r = s_t'{i: '{h: 2'b10, l: 1'b1}, z: 4'h9}; q = s_t'{in_t'{2'b01, 1'b0}, 4'h3}; #1 $display(\"A r=%b q=%b\", r, q); $finish; end
endmodule
",
        "A r=1011001 q=0100011",
    );
    value(
        "scoped",
        "package p; typedef struct packed { logic [3:0] a; logic [3:0] b; } s_t; endpackage
module t;
  p::s_t r;
  initial begin r = p::s_t'{4'h1, 4'h2}; #1 $display(\"A r=%h\", r); $finish; end
endmodule
",
        "A r=12",
    );
    value(
        "explicit_import_keyed_out_of_order",
        "package p; typedef struct packed { logic [3:0] a; logic [3:0] b; } s_t; endpackage
module t;
  import p::s_t;
  s_t r;
  initial begin r = s_t'{b: 4'h2, a: 4'h1}; #1 $display(\"A r=%h\", r); $finish; end
endmodule
",
        "A r=12",
    );
    value(
        "alias",
        &module_with(
            "  typedef s_t a_t;
  a_t r;
  initial begin r = a_t'{4'h9, 4'h4}; #1 $display(\"A r=%h\", r); $finish; end
",
        ),
        "A r=94",
    );
    // A union member takes a plain value; a member wider than 64 bits.
    value(
        "union_member_value",
        "module t;
  typedef union packed { logic [3:0] a; logic [3:0] b; } u_t;
  typedef struct packed { u_t u; logic [3:0] c; } s_t;
  s_t r;
  initial begin r = s_t'{u: 4'h5, c: 4'h6}; #1 $display(\"A r=%h\", r); $finish; end
endmodule
",
        "A r=56",
    );
    value(
        "wide_member",
        "module t;
  typedef struct packed { logic [69:0] a; logic [5:0] b; } s_t;
  s_t r;
  initial begin r = s_t'{70'h3f_0000_0000_0000_0001, 6'h2a}; #1 $display(\"A r=%h\", r); $finish; end
endmodule
",
        "A r=fc0000000000000006a",
    );
}

#[test]
fn expression_positions() {
    value(
        "zero_extend_and_truncate",
        "module t;
  typedef struct packed { logic [2:0] a; logic b; } s_t;
  logic [7:0] w; logic [1:0] n;
  initial begin w = s_t'{3'b101, 1'b1}; n = s_t'{3'b101, 1'b0}; #1 $display(\"A w=%b n=%b\", w, n); $finish; end
endmodule
",
        "A w=00001011 n=10",
    );
    value(
        "replication_and_concat",
        "module t;
  typedef struct packed { logic [1:0] req; } c_t;
  logic [1:0] x; logic [5:0] r; logic [4:0] k;
  initial begin x = 2'b10; r = {3{c_t'{req: x}}}; k = {c_t'{2'b01}, 3'b111}; #1 $display(\"A r=%b k=%b\", r, k); $finish; end
endmodule
",
        "A r=101010 k=01111",
    );
    value(
        "conditional_arms",
        "module t;
  typedef struct packed { logic [2:0] a; logic [1:0] b; } s_t;
  s_t r; logic c;
  initial begin c = 1; r = c ? s_t'{3'd5, 2'd1} : s_t'{3'd2, 2'd2}; #1 $display(\"A r=%b\", r); c = 0; r = c ? s_t'{3'd5, 2'd1} : s_t'{3'd2, 2'd2}; #1 $display(\"B r=%b\", r); $finish; end
endmodule
",
        "A r=10101|B r=01010",
    );
    // An unsigned `T` beside a signed arm: the conditional is unsigned, the signed
    // arm zero-extends (both oracles `00111`).
    value(
        "unsigned_beside_signed_arm",
        "module t;
  typedef struct packed { logic [2:0] a; logic [1:0] b; } s_t;
  typedef struct packed signed { logic [2:0] a; logic [1:0] b; } ss_t;
  ss_t e; logic c;
  initial begin c = 0; e = c ? s_t'{3'd1, 2'd1} : 3'sb111; #1 $display(\"A e=%b\", e); $finish; end
endmodule
",
        "A e=00111",
    );
    value(
        "parameters_and_bits",
        &module_with(
            "  localparam s_t P = s_t'{4'h7, 4'h8};
  parameter s_t Q = s_t'{a: 4'h1, b: 4'h2};
  localparam int W = $bits(s_t'{4'h1, 4'h2});
  initial begin #1 $display(\"A P=%h Q=%h W=%0d\", P, Q, W); $finish; end
",
        ),
        "A P=78 Q=12 W=8",
    );
    value(
        "declaration_and_continuous_assign",
        &module_with(
            "  logic [3:0] x = 4'hc;
  s_t r = s_t'{4'h5, 4'h6};
  s_t w;
  assign w = s_t'{x, ~x};
  initial begin #1 $display(\"A r=%h w=%h\", r, w); x = 4'h1; #1 $display(\"B w=%h\", w); $finish; end
",
        ),
        "A r=56 w=c3|B w=1e",
    );
    value(
        "argument_and_port_actual",
        "module c(input logic [7:0] i, output logic [7:0] o); assign o = i + 8'd1; endmodule
module t;
  typedef struct packed { logic [3:0] a; logic [3:0] b; } s_t;
  function automatic logic [7:0] g(s_t v); return {v.b, v.a}; endfunction
  logic [7:0] o;
  c u(.i(s_t'{4'h1, 4'h2}), .o(o));
  initial begin #1 $display(\"A g=%h o=%h\", g(s_t'{4'h3, 4'h4}), o); $finish; end
endmodule
",
        "A g=43 o=13",
    );
    // Each element is evaluated once.
    value(
        "side_effects_once",
        &module_with(
            "  int n = 0;
  function automatic logic [3:0] f(); n++; return n[3:0]; endfunction
  s_t r;
  initial begin r = s_t'{f(), f()}; #1 $display(\"A r=%h n=%0d\", r, n); $finish; end
",
        ),
        "A r=12 n=2",
    );
    value(
        "compare_case_nba",
        &module_with(
            "  s_t r, q; int k; logic clk = 0; logic [3:0] x = 4'h3;
  always @(posedge clk) q <= s_t'{x, x + 4'd1};
  initial begin r = 8'h12;
    case (r) s_t'{4'h2, 4'h1}: k = 1; s_t'{4'h1, 4'h2}: k = 2; default: k = 3; endcase
    #1 clk = 1; #1 clk = 0;
    #1 $display(\"A eq=%0d ne=%0d k=%0d q=%h\", r == s_t'{4'h1, 4'h2}, r != s_t'{4'h2, 4'h1}, k, q); $finish; end
",
        ),
        "A eq=1 ne=1 k=2 q=34",
    );
    value(
        "generate_continuous_assign",
        "package p;
  typedef struct packed { logic [1:0] h; logic l; } in_t;
  typedef struct packed { in_t i; logic [1:0] z; } s_t;
endpackage
module t;
  logic [1:0] k [2];
  p::s_t w [2];
  for (genvar g = 0; g < 2; g++) begin : gg
    assign w[g] = p::s_t'{i: p::in_t'{h: k[g], l: g[0]}, z: ~k[g]};
  end
  initial begin k[0] = 2'b01; k[1] = 2'b10; #1 $display(\"A w0=%b w1=%b\", w[0], w[1]); $finish; end
endmodule
",
        "A w0=01010 w1=10101",
    );
}

#[test]
fn element_sizing_and_two_state() {
    // sv2v → iverilog prints `000xz01x` and `002 003`, and the same for the untyped
    // twins, where Icarus Verilog prints `xxxxz01x` and `f2c f01`: verilator's line.
    value(
        "fill_x",
        &module_with("  initial begin #1 $display(\"A v=%b\", s_t'{'x, 4'bz01x}); $finish; end\n"),
        "A v=xxxxz01x",
    );
    value(
        "unsized_elements",
        "module t;
  typedef struct packed { logic [3:0] a; logic [7:0] b; } s_t;
  initial begin #1 $display(\"A v=%h u=%h\", s_t'{-1, 300}, s_t'{'1, 1}); $finish; end
endmodule
",
        "A v=f2c u=f01",
    );
    // A 2-state member reads an x or z as 0 (IEEE 1800-2017 §6.11.3); the untyped
    // twin `s = '{x, 1'bz};` prints `s=100 m=10z` in Icarus Verilog (verilator is no
    // x oracle and rejects the typed form here; sv2v keeps the x and z).
    value(
        "two_state_members",
        "module t;
  typedef struct packed { bit [1:0] a; bit b; } s_t;
  typedef struct packed { bit [1:0] a; logic b; } m_t;
  logic [1:0] x;
  initial begin x = 2'b1x; #1 $display(\"A s=%b m=%b\", s_t'{x, 1'bz}, m_t'{x, 1'bz}); $finish; end
endmodule
",
        "A s=100 m=10z",
    );
}

#[test]
fn refused_types_and_patterns_stay_loud() {
    let refused = "a typed assignment pattern `T'{…}` whose type `T` is an unsigned packed struct";
    // A signed struct type: the oracles split wherever its sign is read (verilator
    // `e=00111` and `v=30`, sv2v → iverilog `e=11111` and `v=-2`).
    loud(
        "signed_struct",
        "module t;
  typedef struct packed signed { logic [2:0] a; logic [1:0] b; } s_t;
  s_t e; logic c;
  initial begin c = 0; e = c ? s_t'{3'd1, 2'd1} : 3'sb111; #1 $display(\"A e=%b\", e); $finish; end
endmodule
",
        refused,
    );
    for (name, decl, expr) in [
        (
            "union",
            "typedef union packed { logic [3:0] a; logic [3:0] b; } x_t;",
            "x_t'{a: 4'h3}",
        ),
        (
            "unpacked_array",
            "typedef logic [3:0] x_t [2];",
            "x_t'{4'h1, 4'h2}",
        ),
        (
            "record",
            "typedef struct { logic [3:0] a; int b; } x_t;",
            "x_t'{4'h3, 7}",
        ),
        (
            "packed_vector",
            "typedef logic [1:0][3:0] x_t;",
            "x_t'{4'h1, 4'h2}",
        ),
    ] {
        loud(
            name,
            &format!(
                "module t;\n  {decl}\n  initial begin #1 $display(\"A %p\", {expr}); $finish; end\nendmodule\n"
            ),
            refused,
        );
    }
    loud(
        "per_instance_struct",
        "module t #(parameter int W = 4);
  typedef struct packed { logic [W-1:0] a; logic b; } s_t;
  s_t r;
  initial begin r = s_t'{4'h5, 1'b1}; #1 $display(\"A r=%b\", r); $finish; end
endmodule
",
        refused,
    );
    // The untyped rule's own refusals: a count mismatch (verilator rejects it, sv2v
    // pads), an empty pattern, an unknown key, a replicated pattern.
    loud(
        "count_mismatch",
        &module_with(
            "  s_t r;\n  initial begin r = s_t'{4'h1}; #1 $display(\"A r=%h\", r); $finish; end\n",
        ),
        "exactly one `'{…}` element for each packed-struct field",
    );
    loud(
        "empty",
        &module_with(
            "  s_t r;\n  initial begin r = s_t'{}; #1 $display(\"A r=%h\", r); $finish; end\n",
        ),
        "exactly one `'{…}` element for each packed-struct field",
    );
    loud(
        "unknown_key",
        &module_with("  s_t r;\n  initial begin r = s_t'{a: 4'h1, c: 4'h2}; #1 $display(\"A r=%h\", r); $finish; end\n"),
        "an assignment-pattern key naming a member of",
    );
    loud(
        "replication",
        &module_with("  s_t r;\n  initial begin r = s_t'{2{4'h1}}; #1 $display(\"A r=%h\", r); $finish; end\n"),
        "'}' closing an assignment pattern",
    );
    // A typed pattern as an assignment target stays refused (verilator `x=1 y=2`).
    loud(
        "as_lvalue",
        &module_with("  logic [3:0] x, y;\n  initial begin s_t'{x, y} = 8'h12; #1 $display(\"A x=%h y=%h\", x, y); $finish; end\n"),
        "expected '=' or '<=' after lvalue",
    );
}

#[test]
fn a_value_before_a_pattern_is_not_a_type() {
    // `#D` with `D` a value, then an ordinary statement: unchanged (all three
    // `A x=1 y=2`).
    value(
        "delay_then_statement",
        "module t;
  localparam int D = 1;
  logic [3:0] x, y;
  initial begin #D x = 4'h1; y = 4'h2; #1 $display(\"A x=%h y=%h\", x, y); $finish; end
endmodule
",
        "A x=1 y=2",
    );
}

// ── Review round 1 (§5.2 row 36): a name re-registered as another type, a pattern for a
// union member, a nested member type re-declared between the type and its use ──

/// A name re-declared as a per-instance struct, an unpacked record, a vector, an enum or a
/// type parameter, or imported as a vector type, leaves a same-named outer or imported
/// packed struct's layout under it (a variable of the outer type reads it there), and the
/// typed pattern lowered against it (review F2: `x09` `o=fffffffe b=65`, verilator and
/// sv2v → iverilog `o=0003fffe b=17`). The typed pattern now asks what `T` means where it
/// stands — the typedef entry and the layout must come from one declaration — and is
/// refused by name otherwise.
#[test]
fn a_name_re_declared_as_another_kind_refuses_the_typed_pattern() {
    loud(
        "x09",
        r#"package p;
  typedef struct packed { logic [31:0] addr; logic [31:0] data; logic we; } req_t;
endpackage
module ch09 import p::*; #(parameter int AW = 8, parameter int DW = 8) (output logic [31:0] o, output int b);
  typedef struct packed { logic [AW-1:0] addr; logic [DW-1:0] data; logic we; } req_t;
  req_t r;
  assign o = {req_t'{default: '1}, 1'b0};
  assign b = $bits(req_t'{addr: '0, data: '1, we: 1'b1});
endmodule
module t;
  logic [31:0] o; int b;
  ch09 u (.o(o), .b(b));
  initial begin #1 $display("A o=%h b=%0d", o, b); end
endmodule
"#,
        "whose type `T` is an unsigned packed struct",
    );
    loud(
        "k05",
        r#"package p;
  typedef struct packed { logic [5:0] a; logic [1:0] b; } s_t;
endpackage
module t #(parameter int W = 2);
  import p::*;
  typedef struct packed { logic [W-1:0] a; logic b; } s_t;
  s_t r;
  initial begin r = s_t'{2'b10, 1'b1}; #1 $display("A r=%b bits=%0d", r, $bits(s_t'{2'b10, 1'b1})); end
endmodule
"#,
        "whose type `T` is an unsigned packed struct",
    );
    loud(
        "k06",
        r#"package p;
  typedef struct packed { logic [5:0] a; logic [1:0] b; } s_t;
endpackage
module t;
  import p::*;
  typedef struct { logic [3:0] a; logic [3:0] b; } s_t;
  s_t r;
  initial begin r = s_t'{4'h9, 4'h7}; #1 $display("A a=%h b=%h", r.a, r.b); end
endmodule
"#,
        "whose type `T` is an unsigned packed struct",
    );
    loud(
        "u01",
        r#"typedef struct packed { logic [5:0] a; logic [1:0] b; } s_t;
module t;
  typedef struct { logic [3:0] a; logic [3:0] b; } s_t;
  s_t r;
  initial begin r = s_t'{4'h9, 4'h7}; #1 $display("A a=%h b=%h", r.a, r.b); end
endmodule
"#,
        "whose type `T` is an unsigned packed struct",
    );
    loud(
        "u02",
        r#"typedef struct packed { logic [5:0] a; logic [1:0] b; } s_t;
module ch #(parameter type s_t = logic [1:0][3:0]) (output logic [7:0] o);
  assign o = s_t'{4'h1, 4'h2};
endmodule
module t;
  logic [7:0] o;
  ch u (.o(o));
  initial begin #1 $display("A o=%h", o); end
endmodule
"#,
        "whose type `T` is an unsigned packed struct",
    );
    loud(
        "i1",
        r#"package p; typedef logic [1:0][3:0] s_t; endpackage
typedef struct packed { logic [5:0] a; logic [1:0] b; } s_t;
module t;
  import p::s_t;
  logic [7:0] w;
  initial begin w = s_t'{4'h1, 4'h2}; #1 $display("A w=%h", w); $finish; end
endmodule
"#,
        "whose type `T` is an unsigned packed struct",
    );
    loud(
        "i2",
        r#"package p; typedef logic [1:0][3:0] s_t; endpackage
typedef struct packed { logic [5:0] a; logic [1:0] b; } s_t;
module t;
  import p::*;
  logic [7:0] w;
  initial begin w = s_t'{4'h1, 4'h2}; #1 $display("A w=%h", w); $finish; end
endmodule
"#,
        "whose type `T` is an unsigned packed struct",
    );
    loud(
        "k07",
        r#"package p;
  typedef struct packed { logic [5:0] a; logic [1:0] b; } s_t;
endpackage
module t;
  import p::*;
  typedef enum logic [7:0] { E0 = 8'h00, E1 = 8'h06 } s_t;
  initial begin #1 $display("A v=%h", s_t'{6'h01, 2'b10}); end
endmodule
"#,
        "whose type `T` is an unsigned packed struct",
    );
    // An imported vector type over a unit struct, read whole: unchanged (all three `A r=12`).
    value(
        "i3",
        r#"package p; typedef logic [7:0] s_t; endpackage
typedef struct packed { logic [5:0] a; logic [1:0] b; } s_t;
module t;
  import p::*;
  s_t r;
  initial begin r = 8'h12; #1 $display("A r=%h", r); $finish; end
endmodule
"#,
        "A r=12",
    );
}

/// A nested `'{…}` for a packed-union member concatenates every member of the overlay
/// (review F3: typed `w=567` where verilator and sv2v → iverilog print `w=067`). Refused in
/// a typed pattern, row 32's anonymous union member (`d01`) included. The untyped pattern
/// keeps its lowering: right where the target truncates the surplus away (`b11u` `r=67`,
/// `b11f` `r=57`, `r8a` — verilator and sv2v → iverilog print the same), wrong otherwise
/// (`b11gu` `r=56`, both oracles `r=76`; ROADMAP §2).
#[test]
fn a_typed_pattern_for_a_union_member_is_refused() {
    loud(
        "b11",
        r#"module t;
  typedef union packed { logic [3:0] a; logic [3:0] b; } u_t;
  typedef struct packed { u_t u; logic [3:0] c; } s_t;
  logic [11:0] w;
  initial begin w = s_t'{u: '{4'h5, 4'h6}, c: 4'h7}; #1 $display("A w=%h", w); $finish; end
endmodule
"#,
        "a `'{…}` pattern for a union member is unsupported in v1",
    );
    loud(
        "b11g",
        r#"module t;
  typedef union packed { logic [3:0] a; logic [3:0] b; } u_t;
  typedef struct packed { logic [3:0] c; u_t u; } s_t;
  logic [11:0] w;
  initial begin w = s_t'{c: 4'h7, u: '{4'h5, 4'h6}}; #1 $display("A w=%h", w); $finish; end
endmodule
"#,
        "a `'{…}` pattern for a union member is unsupported in v1",
    );
    loud(
        "b11h",
        r#"module t;
  typedef union packed { logic [3:0] a; logic [3:0] b; } u_t;
  typedef struct packed { u_t u; logic k; } in_t;
  typedef struct packed { logic [2:0] c; in_t i; } s_t;
  logic [11:0] w;
  initial begin w = s_t'{c: 3'h5, i: '{u: '{4'h3, 4'h6}, k: 1'b1}}; #1 $display("A w=%b", w); $finish; end
endmodule
"#,
        "a `'{…}` pattern for a union member is unsupported in v1",
    );
    loud(
        "w03",
        r#"module t;
  typedef union packed { logic [3:0] a; logic [3:0] b; } u_t;
  typedef struct packed { u_t u; logic [3:0] c; } s_t;
  logic [15:0] w; s_t r;
  initial begin w = s_t'{'{4'h5, 4'h7}, 4'h6}; r = '{'{4'h5, 4'h7}, 4'h6}; #1 $display("A w=%h bits=%0d r=%h", w, $bits(s_t'{'{4'h5, 4'h7}, 4'h6}), r); end
endmodule
"#,
        "a `'{…}` pattern for a union member is unsupported in v1",
    );
    loud(
        "n01",
        r#"module t;
  typedef union packed { logic [3:0] a; logic [3:0] b; } u_t;
  typedef struct packed { logic [3:0] c; u_t u; } s_t;
  s_t r;
  initial begin r = s_t'{4'h6, '{4'h5, 4'h7}}; #1 $display("A r=%h", r); end
endmodule
"#,
        "a `'{…}` pattern for a union member is unsupported in v1",
    );
    value(
        "b11u",
        r#"module t;
  typedef union packed { logic [3:0] a; logic [3:0] b; } u_t;
  typedef struct packed { u_t u; logic [3:0] c; } s_t;
  s_t r;
  initial begin r = '{u: '{4'h5, 4'h6}, c: 4'h7}; #1 $display("A r=%h", r); $finish; end
endmodule
"#,
        "A r=67",
    );
    value(
        "b11f",
        r#"module t;
  typedef union packed { logic [3:0] a; logic [3:0] b; } u_t;
  typedef struct packed { u_t u; logic [3:0] c; } s_t;
  s_t r;
  initial begin r = '{u: '{default: 4'h5}, c: 4'h7}; #1 $display("A r=%h", r); $finish; end
endmodule
"#,
        "A r=57",
    );
    value(
        "r8a",
        r#"module t;
  typedef union packed { logic [3:0] a; logic [3:0] b; } u_t;
  typedef struct packed { u_t u; logic [3:0] c; } s_t;
  s_t arr [2] = '{'{u: '{4'h5, 4'h6}, c: 4'h7}, '{u: 4'h1, c: 4'h2}};
  initial begin #1 $display("A a0=%h a1=%h", arr[0], arr[1]); $finish; end
endmodule
"#,
        "A a0=67 a1=12",
    );
    loud(
        "d01",
        r#"module t;
  typedef struct packed { union packed { logic [3:0] a; logic [3:0] b; } u; logic [3:0] c; } s_t;
  logic [11:0] w;
  initial begin w = s_t'{u: '{4'h5, 4'h6}, c: 4'h7}; #1 $display("A w=%h", w); $finish; end
endmodule
"#,
        "error[VITA-E2002]",
    );
}

/// A nested member's layout is looked up by its bare type key where the pattern stands,
/// so a same-named type declared in a generate block, a function, a task or a `begin`
/// block between the type and its use re-laid the member (review F1: `w=0010011001`,
/// verilator and sv2v → iverilog `w=0001011001`). The typed pattern refuses then; a
/// package type's nested keys are package-qualified and keep working (all three
/// `v=1011001 k=0110011`). The check runs at every depth: a type two levels down (`r7k`)
/// and row 32's anonymous member (`d10`). The untyped pattern keeps the use-site lookup
/// (ROADMAP §2, row 67).
#[test]
fn a_nested_type_re_declared_before_the_pattern_is_refused() {
    loud(
        "a01",
        r#"module t;
  typedef struct packed { logic [1:0] h; logic l; } in_t;
  typedef struct packed { in_t i; logic [3:0] z; } s_t;
  logic [9:0] w;
  if (1) begin : g
    typedef struct packed { logic [3:0] h; logic [1:0] l; } in_t;
    assign w = s_t'{i: '{h: 2'b10, l: 1'b1}, z: 4'h9};
  end
  initial begin #1 $display("A w=%b", w); $finish; end
endmodule
"#,
        "whose nested member types mean here what they meant where `T` was declared",
    );
    loud(
        "a01s",
        r#"module t;
  typedef struct packed { logic [1:0] h; logic l; } in_t;
  typedef struct packed { in_t i; logic [3:0] z; } s_t;
  logic [9:0] w;
  if (1) begin : g
    typedef struct packed { logic h; logic [1:0] l; } in_t;
    assign w = s_t'{i: '{h: 2'b10, l: 1'b1}, z: 4'h9};
  end
  initial begin #1 $display("A w=%b", w); $finish; end
endmodule
"#,
        "whose nested member types mean here what they meant where `T` was declared",
    );
    loud(
        "a02",
        r#"module t;
  typedef struct packed { logic [1:0] h; logic l; } in_t;
  typedef struct packed { in_t i; logic [3:0] z; } s_t;
  function automatic logic [9:0] f();
    typedef struct packed { logic [3:0] h; logic [1:0] l; } in_t;
    return s_t'{i: '{h: 2'b10, l: 1'b1}, z: 4'h9};
  endfunction
  initial begin #1 $display("A f=%b", f()); $finish; end
endmodule
"#,
        "whose nested member types mean here what they meant where `T` was declared",
    );
    loud(
        "a03",
        r#"module t;
  typedef struct packed { logic [1:0] h; logic l; } in_t;
  typedef struct packed { in_t i; logic [3:0] z; } s_t;
  logic [9:0] w;
  initial begin : b
    typedef struct packed { logic [3:0] h; logic [1:0] l; } in_t;
    w = s_t'{i: '{h: 2'b10, l: 1'b1}, z: 4'h9};
    #1 $display("A w=%b", w); $finish;
  end
endmodule
"#,
        "whose nested member types mean here what they meant where `T` was declared",
    );
    loud(
        "a29",
        r#"module t;
  typedef struct packed { logic [1:0] h; logic l; } in_t;
  typedef struct packed { in_t i; logic [3:0] z; } s_t;
  logic [9:0] w;
  task automatic tk();
    typedef struct packed { logic [3:0] h; logic [1:0] l; } in_t;
    w = s_t'{i: '{h: 2'b10, l: 1'b1}, z: 4'h9};
  endtask
  initial begin tk(); #1 $display("A w=%b", w); $finish; end
endmodule
"#,
        "whose nested member types mean here what they meant where `T` was declared",
    );
    loud(
        "n04",
        r#"module t;
  typedef struct packed { logic [1:0] h; logic l; } in_t;
  typedef struct packed { in_t i; logic [3:0] z; } s_t;
  function automatic logic [6:0] f();
    typedef struct packed { logic l; logic [1:0] h; } in_t;
    return s_t'{'{2'b10, 1'b1}, 4'h9};
  endfunction
  initial begin #1 $display("A f=%b m=%b", f(), s_t'{'{2'b10, 1'b1}, 4'h9}); end
endmodule
"#,
        "whose nested member types mean here what they meant where `T` was declared",
    );
    loud(
        "r7k",
        r#"module t;
  typedef struct packed { logic [1:0] q; } x_t;
  typedef struct packed { x_t x; logic l; } in_t;
  typedef struct packed { in_t i; logic [3:0] z; } s_t;
  logic [9:0] w;
  if (1) begin : g
    typedef struct packed { logic q; } x_t;
    assign w = s_t'{i: '{x: '{q: 2'b10}, l: 1'b1}, z: 4'h9};
  end
  initial begin #1 $display("A w=%b", w); $finish; end
endmodule
"#,
        "whose nested member types mean here what they meant",
    );
    loud(
        "d10",
        r#"module t;
  typedef struct packed { logic [1:0] h; logic l; } in_t;
  typedef struct packed { struct packed { in_t i; logic k; } a; logic b; } s_t;
  logic [9:0] w;
  if (1) begin : g
    typedef struct packed { logic [3:0] h; logic [1:0] l; } in_t;
    assign w = s_t'{a: '{i: '{h: 2'b10, l: 1'b1}, k: 1'b0}, b: 1'b1};
  end
  initial begin #1 $display("A w=%b", w); $finish; end
endmodule
"#,
        "error[VITA-E2002]",
    );
    value(
        "n03",
        r#"package p;
  typedef struct packed { logic [1:0] h; logic l; } in_t;
  typedef struct packed { in_t i; logic [3:0] z; } s_t;
endpackage
module t;
  typedef struct packed { logic l; logic [1:0] h; } in_t;
  initial begin #1 $display("A v=%b k=%b", p::s_t'{'{2'b10, 1'b1}, 4'h9}, p::s_t'{i: '{h: 2'b01, l: 1'b1}, z: 4'h3}); end
endmodule
"#,
        "A v=1011001 k=0110011",
    );
    value(
        "n03b",
        r#"package p;
  typedef struct packed { logic [1:0] h; logic l; } in_t;
  typedef struct packed { in_t i; logic [3:0] z; } s_t;
endpackage
module t;
  import p::s_t;
  typedef struct packed { logic l; logic [2:0] h; } in_t;
  s_t r;
  initial begin r = s_t'{'{2'b10, 1'b1}, 4'h9}; #1 $display("A r=%b k=%b", r, s_t'{i: '{h: 2'b01, l: 1'b1}, z: 4'h3}); end
endmodule
"#,
        "A r=1011001 k=0110011",
    );
    value(
        "n03c",
        r#"package p;
  typedef struct packed { logic [1:0] h; logic l; } in_t;
  typedef struct packed { in_t i; logic [3:0] z; } s_t;
endpackage
package q;
  typedef struct packed { logic [2:0] h; logic [1:0] l; } in_t;
endpackage
module t;
  import q::*;
  initial begin #1 $display("A v=%b", p::s_t'{'{2'b10, 1'b1}, 4'h9}); end
endmodule
"#,
        "A v=1011001",
    );
}

/// A variable is bound to its struct type by NAME, so inside a function, task, generate
/// block or `begin` block that re-declares the name, an outer variable's member access,
/// untyped pattern, queue push or element access still reads the layout the outer type
/// left under it — the outer variable's own type (review round 2: every cell below printed
/// the inner type's member, or was refused, once a re-declaration dropped that layout;
/// verilator, and Icarus Verilog / sv2v → iverilog where they run, print these values).
/// The typed pattern decides what `T` means where it stands from the typedef entry
/// instead, so it never needs that layout dropped.
#[test]
fn an_outer_variable_keeps_its_type_where_the_name_is_re_declared() {
    value(
        "g05b",
        r#"module t;
  typedef struct packed { logic [3:0] a; logic [3:0] b; } s_t;
  s_t r;
  function automatic logic [7:0] f();
    typedef struct { logic [1:0] a; logic [5:0] b; } s_t;
    return r.b;
  endfunction
  initial begin r = 8'h12; #1 $display("A f=%h", f()); end
endmodule
"#,
        "A f=02",
    );
    value(
        "g06b",
        r#"module t #(parameter int W = 2);
  typedef struct packed { logic [3:0] a; logic [3:0] b; } s_t;
  s_t r;
  function automatic logic [7:0] f();
    typedef struct packed { logic [W-1:0] a; logic [5:0] b; } s_t;
    return r.b;
  endfunction
  initial begin r = 8'h12; #1 $display("A f=%h", f()); end
endmodule
"#,
        "A f=02",
    );
    value(
        "g18b",
        r#"module t;
  typedef struct packed { logic [3:0] a; logic [3:0] b; } s_t;
  s_t r; logic [7:0] w;
  if (1) begin : g
    typedef struct { logic [1:0] a; logic [5:0] b; } s_t;
    assign w = r.b;
  end
  initial begin r = 8'h12; #1 $display("A w=%h", w); end
endmodule
"#,
        "A w=02",
    );
    value(
        "g19c",
        r#"module t #(parameter int W = 2);
  typedef struct packed { logic [3:0] a; logic [3:0] b; } s_t;
  s_t r; logic clk = 0;
  if (1) begin : g
    typedef struct packed { logic [W-1:0] a; logic [5:0] b; } s_t;
    always @(posedge clk) r.b <= 4'h7;
  end
  initial begin r = 8'h12; #1 clk = 1; #1 $display("A r=%h", r); end
endmodule
"#,
        "A r=17",
    );
    value(
        "g23",
        r#"module t;
  typedef struct packed { logic [3:0] a; logic [3:0] b; } s_t;
  s_t r;
  function automatic void f();
    typedef struct { logic [1:0] a; logic [5:0] b; } s_t;
    r = '{4'h3, 4'h4};
  endfunction
  initial begin f(); #1 $display("A r=%h", r); end
endmodule
"#,
        "A r=34",
    );
    value(
        "g26",
        r#"module t;
  typedef struct packed { logic [3:0] a; logic [3:0] b; } s_t;
  s_t q [$];
  function automatic void f();
    typedef struct { logic [1:0] a; logic [5:0] b; } s_t;
    q.push_back('{4'h3, 4'h4});
  endfunction
  initial begin f(); #1 $display("A n=%0d q0=%h", q.size(), q[0]); end
endmodule
"#,
        "A n=1 q0=34",
    );
    value(
        "g30",
        r#"module t;
  typedef struct packed { logic [3:0] a; logic [3:0] b; } s_t;
  s_t r; logic clk = 0; logic [7:0] o;
  if (1) begin : g
    typedef struct { logic [1:0] a; logic [5:0] b; } s_t;
    always @(posedge clk) begin r.a <= 4'h9; o <= r.b; end
  end
  initial begin r = 8'h12; #1 clk = 1; #1 $display("A r=%h o=%h", r, o); end
endmodule
"#,
        "A r=92 o=02",
    );
    value(
        "g31",
        r#"module t;
  typedef struct packed { logic [3:0] a; logic [3:0] b; } s_t;
  s_t r; logic [7:0] o;
  initial begin
    r = 8'h12;
    begin : blk
    typedef struct { logic [1:0] a; logic [5:0] b; } s_t;
      o = r.b;
      r.b = 4'hf;
    end
    #1 $display("A o=%h r=%h", o, r);
  end
endmodule
"#,
        "A o=02 r=1f",
    );
    value(
        "v03",
        r#"module t;
  typedef union packed { logic [7:0] w; logic [7:0] v; } u_t;
  u_t u;
  function automatic logic [7:0] f();
    typedef struct { logic [1:0] w; logic [5:0] v; } u_t;
    return u.v;
  endfunction
  initial begin u = 8'h5a; #1 $display("A f=%h", f()); end
endmodule
"#,
        "A f=5a",
    );
    value(
        "r1y",
        r#"module t;
  typedef struct packed { logic [3:0] a; logic [3:0] b; } s_t;
  s_t r;
  function automatic logic [3:0] f();
    typedef struct { logic [1:0] a; logic [5:0] b; } s_t;
    return r.a;
  endfunction
  initial begin r = 8'h5a; #1 $display("A f=%h", f()); $finish; end
endmodule
"#,
        "A f=5",
    );
    value(
        "r1z2",
        r#"module t;
  typedef struct packed { logic [3:0] a; logic [3:0] b; } s_t;
  s_t r;
  function automatic void f();
    typedef struct { logic [1:0] a; logic [5:0] b; } s_t;
    r = '{4'h3, 4'hc};
  endfunction
  initial begin f(); #1 $display("A r=%h", r); $finish; end
endmodule
"#,
        "A r=3c",
    );
    value(
        "g01b",
        r#"module t;
  typedef struct packed { logic [3:0] a; logic [3:0] b; } s_t;
  s_t r;
  function automatic logic [3:0] f();
    typedef logic [7:0] s_t;
    return r.a;
  endfunction
  initial begin r = 8'h12; #1 $display("A f=%h", f()); end
endmodule
"#,
        "A f=1",
    );
    value(
        "g04",
        r#"module t;
  typedef struct packed { logic [3:0] a; logic [3:0] b; } s_t;
  s_t r; logic [3:0] w;
  if (1) begin : g
    typedef logic [7:0] s_t;
    assign w = r.b;
  end
  initial begin r = 8'h12; #1 $display("A w=%h", w); end
endmodule
"#,
        "A w=2",
    );
    value(
        "g09",
        r#"module t;
  typedef struct packed { logic [3:0] a; logic [3:0] b; } s_t;
  s_t r;
  function automatic void f();
    typedef logic [7:0] s_t;
    r = '{4'h3, 4'h4};
  endfunction
  initial begin f(); #1 $display("A r=%h", r); end
endmodule
"#,
        "A r=34",
    );
    value(
        "g14",
        r#"package p;
  typedef struct packed { logic [3:0] a; logic [3:0] b; } s_t;
endpackage
module t;
  import p::*;
  s_t r;
  function automatic logic [3:0] f();
    typedef logic [7:0] s_t;
    return r.a;
  endfunction
  initial begin r = 8'h12; #1 $display("A f=%h", f()); end
endmodule
"#,
        "A f=1",
    );
    value(
        "r9e",
        r#"module t;
  typedef union packed { logic [7:0] a; logic [7:0] b; } u_t;
  u_t u; logic [7:0] w;
  function automatic logic [7:0] f();
    typedef logic [7:0] u_t;
    return u.a;
  endfunction
  initial begin u = 8'h5a; #1 $display("A f=%h", f()); $finish; end
endmodule
"#,
        "A f=5a",
    );
}

/// The name `T` means the type of its newest registration, whatever the kind. An unpacked
/// record writes no typedef entry, so a record declared after a same-named packed struct (in
/// a function, a generate block, a module under a unit struct, or by an import) left the
/// struct's typedef entry and layout under the name, and an alias of it copied both: `Y'{…}`
/// lowered against the shadowed struct (review round 3, J1 / R3-F1: `h01` `bits=8`, `o46`
/// `a=09 b=3`, verilator and sv2v → iverilog `bits=16`, `a=9 b=7`). Every registration now
/// takes a number from one counter and the typed pattern runs only for the newest; an alias
/// of a name whose newest registration is no packed struct names none. The other way round, a
/// packed struct declared after a same-named record or per-instance struct runs (R3-N1: `o40`
/// … `o43`, `h06`, `h07`, what verilator and sv2v → iverilog print), and the R24 shape (`o01`)
/// is unchanged. The untyped twins of the alias cells keep their value (ROADMAP §2,
/// struct-binding-by-name).
#[test]
fn the_newest_registration_of_the_name_decides() {
    loud(
        "h01",
        r#"module t;
  typedef struct packed { logic [3:0] a; logic [3:0] b; } s_t;
  function automatic int f();
    typedef struct { logic [7:0] a; logic [7:0] b; } s_t;
    typedef s_t t2;
    return $bits(t2'{8'h12, 8'h34});
  endfunction
  initial begin #1 $display("A bits=%0d", f()); $finish; end
endmodule
"#,
        "whose type `T` is an unsigned packed struct",
    );
    loud(
        "h01b",
        r#"module t;
  typedef struct packed { logic [3:0] a; logic [3:0] b; } s_t;
  logic [15:0] w;
  if (1) begin : g
    typedef struct { logic [7:0] a; logic [7:0] b; } s_t;
    typedef s_t t2;
    t2 r;
    initial begin r = t2'{8'h12, 8'h34}; #1 w = {r.a, r.b}; end
  end
  initial begin #2 $display("A w=%h", w); $finish; end
endmodule
"#,
        "whose type `T` is an unsigned packed struct",
    );
    loud(
        "h01c",
        r#"typedef struct packed { logic [3:0] a; logic [3:0] b; } s_t;
module t;
  typedef struct { logic [7:0] a; logic [7:0] b; } s_t;
  typedef s_t t2;
  initial begin #1 $display("A bits=%0d", $bits(t2'{8'h12, 8'h34})); $finish; end
endmodule
"#,
        "whose type `T` is an unsigned packed struct",
    );
    loud(
        "h01d",
        r#"package p; typedef struct { logic [7:0] a; logic [7:0] b; } s_t; endpackage
typedef struct packed { logic [3:0] a; logic [3:0] b; } s_t;
module t;
  import p::s_t;
  typedef s_t t2;
  initial begin #1 $display("A bits=%0d", $bits(t2'{8'h12, 8'h34})); $finish; end
endmodule
"#,
        "whose type `T` is an unsigned packed struct",
    );
    loud(
        "h01g",
        r#"typedef struct packed { logic [3:0] a; logic [3:0] b; } s_t;
package p;
  typedef struct { logic [7:0] a; logic [7:0] b; } s_t;
  typedef s_t t2;
endpackage
module t;
  initial begin #1 $display("A bits=%0d", $bits(p::t2'{8'h12, 8'h34})); $finish; end
endmodule
"#,
        "whose type `T` is an unsigned packed struct",
    );
    loud(
        "h01h",
        r#"typedef struct packed { logic [3:0] a; logic [3:0] b; } s_t;
package p;
  typedef struct { logic [7:0] a; logic [7:0] b; } s_t;
endpackage
module t;
  typedef p::s_t t2;
  initial begin #1 $display("A bits=%0d", $bits(t2'{8'h12, 8'h34})); $finish; end
endmodule
"#,
        "whose type `T` is an unsigned packed struct",
    );
    loud(
        "h01i",
        r#"package p; typedef struct { logic [7:0] a; logic [7:0] b; } s_t; endpackage
typedef struct packed { logic [3:0] a; logic [3:0] b; } s_t;
module t;
  import p::*;
  typedef s_t t2;
  initial begin #1 $display("A bits=%0d", $bits(t2'{8'h12, 8'h34})); $finish; end
endmodule
"#,
        "whose type `T` is an unsigned packed struct",
    );
    loud(
        "h01j",
        r#"module t;
  typedef struct packed { logic [3:0] a; logic [3:0] b; } s_t;
  logic [15:0] w;
  if (1) begin : g
    typedef struct { logic [7:0] a; logic [7:0] b; } s_t;
    typedef s_t t2;
    typedef t2 t3;
    t3 r;
    initial begin r = t3'{8'h12, 8'h34}; #1 w = {r.a, r.b}; end
  end
  initial begin #2 $display("A w=%h", w); $finish; end
endmodule
"#,
        "whose type `T` is an unsigned packed struct",
    );
    loud(
        "o46",
        r#"typedef struct packed { logic [5:0] a; logic [1:0] b; } X;
module t;
  typedef struct { logic [3:0] a; logic [3:0] b; } X;
  typedef X Y;
  Y r;
  initial begin r = Y'{4'h9, 4'h7}; #1 $display("A a=%h b=%h", r.a, r.b); end
endmodule
"#,
        "whose type `T` is an unsigned packed struct",
    );
    loud(
        "o46b",
        r#"typedef struct packed { logic [5:0] a; logic [1:0] b; } X;
module t;
  typedef struct { logic [3:0] a; logic [5:0] b; } X;
  typedef X Y;
  initial begin #1 $display("A bits=%0d", $bits(Y'{4'h9, 6'h27})); end
endmodule
"#,
        "whose type `T` is an unsigned packed struct",
    );
    loud(
        "o46c",
        r#"module t;
  typedef struct packed { logic [5:0] a; logic [1:0] b; } X;
  logic [7:0] w; int b;
  if (1) begin : g
    typedef struct { logic [3:0] a; logic [5:0] b; } X;
    typedef X Y;
    assign b = $bits(Y'{4'h9, 6'h27});
  end
  initial begin #1 $display("A b=%0d", b); end
endmodule
"#,
        "whose type `T` is an unsigned packed struct",
    );
    loud(
        "o46d",
        r#"package p;
  typedef struct packed { logic [5:0] a; logic [1:0] b; } X;
endpackage
module t;
  import p::*;
  typedef struct { logic [3:0] a; logic [5:0] b; } X;
  typedef X Y;
  initial begin #1 $display("A bits=%0d", $bits(Y'{4'h9, 6'h27})); end
endmodule
"#,
        "whose type `T` is an unsigned packed struct",
    );
    loud(
        "o46e",
        r#"module t;
  typedef struct packed { logic [5:0] a; logic [1:0] b; } X;
  function automatic int f();
    typedef struct { logic [3:0] a; logic [5:0] b; } X;
    typedef X Y;
    return $bits(Y'{4'h9, 6'h27});
  endfunction
  initial begin #1 $display("A f=%0d", f()); end
endmodule
"#,
        "whose type `T` is an unsigned packed struct",
    );
    loud(
        "o46f",
        r#"typedef struct packed { logic [5:0] a; logic [1:0] b; } X;
module t;
  typedef struct { logic [3:0] a; logic [5:0] b; } X;
  typedef X Y;
  Y r;
  initial begin r = Y'{4'h9, 6'h27}; #1 $display("A a=%h b=%h", r.a, r.b); end
endmodule
"#,
        "whose type `T` is an unsigned packed struct",
    );
    loud(
        "o46g",
        r#"typedef struct packed { logic [5:0] a; logic [1:0] b; } X;
package pr;
  typedef struct { logic [3:0] a; logic [5:0] b; } X;
endpackage
module t;
  import pr::X;
  typedef X Y;
  Y r;
  initial begin r = Y'{4'h9, 6'h27}; #1 $display("A a=%h b=%h bits=%0d", r.a, r.b, $bits(Y'{4'h9, 6'h27})); end
endmodule
"#,
        "whose type `T` is an unsigned packed struct",
    );
    value(
        "o40",
        r#"module t;
  typedef struct { logic [3:0] a; logic [3:0] b; } X;
  logic [7:0] w;
  if (1) begin : g
  typedef struct packed { logic [5:0] a; logic [1:0] b; } X;
    assign w = X'{6'h15, 2'b10};
  end
  initial begin #1 $display("A w=%h", w); end
endmodule
"#,
        "A w=56",
    );
    value(
        "o40b",
        r#"module t;
  typedef struct { logic [3:0] a; logic [3:0] b; } X;
  function automatic logic [7:0] f();
  typedef struct packed { logic [5:0] a; logic [1:0] b; } X;
    return X'{6'h15, 2'b10};
  endfunction
  initial begin #1 $display("A f=%h", f()); end
endmodule
"#,
        "A f=56",
    );
    value(
        "o41",
        r#"module t #(parameter int W = 2);
  typedef struct packed { logic [W-1:0] a; logic b; } X;
  function automatic logic [7:0] f();
  typedef struct packed { logic [5:0] a; logic [1:0] b; } X;
    return X'{6'h15, 2'b10};
  endfunction
  initial begin #1 $display("A f=%h", f()); end
endmodule
"#,
        "A f=56",
    );
    value(
        "o41b",
        r#"module t #(parameter int W = 2);
  typedef struct packed { logic [W-1:0] a; logic b; } X;
  logic [7:0] w;
  if (1) begin : g
  typedef struct packed { logic [5:0] a; logic [1:0] b; } X;
    assign w = X'{6'h15, 2'b10};
  end
  initial begin #1 $display("A w=%h", w); end
endmodule
"#,
        "A w=56",
    );
    value(
        "o42",
        r#"package p;
  typedef struct { logic [3:0] a; logic [3:0] b; } X;
endpackage
module t;
  import p::*;
  typedef struct packed { logic [5:0] a; logic [1:0] b; } X;
  logic [7:0] w;
  assign w = X'{6'h15, 2'b10};
  initial begin #1 $display("A w=%h", w); end
endmodule
"#,
        "A w=56",
    );
    value(
        "o43",
        r#"typedef struct { logic [3:0] a; logic [3:0] b; } X;
module t;
  typedef struct packed { logic [5:0] a; logic [1:0] b; } X;
  logic [7:0] w;
  assign w = X'{6'h15, 2'b10};
  initial begin #1 $display("A w=%h", w); end
endmodule
"#,
        "A w=56",
    );
    value(
        "h06",
        r#"module t;
  typedef struct { logic [7:0] a; logic [7:0] b; } s_t;
  function automatic logic [7:0] f();
    typedef struct packed { logic [3:0] a; logic [3:0] b; } s_t;
    return s_t'{4'h3, 4'h1};
  endfunction
  initial begin #1 $display("A f=%h", f()); $finish; end
endmodule
"#,
        "A f=31",
    );
    value(
        "h07",
        r#"module t #(parameter int W = 8);
  typedef struct packed { logic [W-1:0] a; logic b; } s_t;
  logic [7:0] w;
  if (1) begin : g
    typedef struct packed { logic [3:0] a; logic [3:0] b; } s_t;
    assign w = s_t'{4'h3, 4'h1};
  end
  initial begin #1 $display("A w=%h", w); $finish; end
endmodule
"#,
        "A w=31",
    );
    value(
        "o01",
        r#"package keymgr_dpe_pkg;
  parameter int KeyWidth = 256;
  parameter int Shares = 2;
  parameter int KeyVersionWidth = 32;
  parameter int DpeBootStagesWidth = 2;
  typedef enum logic [DpeBootStagesWidth-1:0] { BootStageCreator = 0, BootStageOwnerInt = 1, BootStageOwner = 2, BootStageRuntime = 3 } keymgr_dpe_boot_stage_e;
  typedef struct packed { logic retain_parent; logic exportable; logic allow_child; } keymgr_dpe_policy_t;
  localparam keymgr_dpe_policy_t DEFAULT_UDS_POLICY = '{ retain_parent : 1'b0, exportable : 1'b0, allow_child : 1'b1 };
  typedef struct packed {
    logic valid;
    keymgr_dpe_boot_stage_e boot_stage;
    logic [Shares-1:0][KeyWidth-1:0] key;
    logic [KeyVersionWidth-1:0] max_key_version;
    keymgr_dpe_policy_t key_policy;
  } keymgr_dpe_slot_t;
  typedef struct packed {
    keymgr_dpe_policy_t key_policy;
    keymgr_dpe_boot_stage_e boot_stage;
    logic valid;
    logic [KeyVersionWidth-1:0] max_key_version;
  } keymgr_dpe_metadata_slot_t;
  function automatic keymgr_dpe_metadata_slot_t extract_metadata_from_slot (keymgr_dpe_slot_t slot);
    logic [Shares-1:0][KeyWidth-1:0] unused_key = slot.key;
    return keymgr_dpe_metadata_slot_t'{
      slot.key_policy,
      slot.boot_stage,
      slot.valid,
      slot.max_key_version
    };
  endfunction
endpackage : keymgr_dpe_pkg
module t;
  import keymgr_dpe_pkg::*;
  keymgr_dpe_slot_t s; keymgr_dpe_metadata_slot_t m;
  initial begin
    s = '0; s.valid = 1'b1; s.boot_stage = BootStageOwner; s.max_key_version = 32'h1234_5678; s.key_policy = '{1'b1, 1'b0, 1'b1}; s.key = '1;
    m = extract_metadata_from_slot(s);
    #1 $display("A m=%h pol=%b bs=%0d v=%b mkv=%h", m, m.key_policy, m.boot_stage, m.valid, m.max_key_version);
  end
endmodule
"#,
        "A m=2d12345678 pol=101 bs=2 v=1 mkv=12345678",
    );
}

/// A select after a typed pattern (verilator and sv2v → iverilog reject `T'{…}[7:4]`).
#[test]
fn a_select_after_a_typed_pattern_is_refused() {
    loud(
        "p40",
        r#"module t;
  typedef struct packed { logic [3:0] a; logic [3:0] b; } s_t;
  initial begin #1 $display("A s=%h", s_t'{4'h1, 4'h2}[7:4]); end
endmodule
"#,
        "an operator after a typed assignment pattern, not a select",
    );
}
