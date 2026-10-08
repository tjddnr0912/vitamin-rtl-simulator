//! §3 ⑤ⓓ (§5.2 row 32): an ANONYMOUS packed struct or union as a member's type —
//! `typedef struct packed { struct packed { logic q; logic qe; } tx_done; … } t;`,
//! the shape of every OpenTitan `*_reg_pkg` (row-9 census root R02; 18 of 19
//! OpenTitan pages stopped at it with E2002 `expected a net/var type in a
//! struct/union member, found keyword 'struct'`).
//!
//! The parser lays the anonymous body out with the typedef path itself, under a
//! minted type key no source name can reach, and the member then resolves as a
//! member of a NAMED nested typedef does. So every cell here runs twice: the
//! anonymous design and its named twin (each anonymous body hoisted into a
//! `typedef … x_t;` of its own) must print the same line, and that line is the
//! oracles' (iverilog 13.0 `-g2012`, sv2v 0.0.13 → iverilog and verilator 5.052
//! `--binary`, all three unless the comment names fewer: iverilog rejects a `'{…}`
//! pattern into a nested struct and an unpacked array of a struct). A twin the
//! tool refuses keeps its anonymous spelling refused as well.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_anon_{}_{n}", std::process::id()));
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

/// The anonymous design and its named twin both exit 0 and print `expect`.
fn twin(name: &str, anon: &str, named: &str, expect: &str) {
    for (which, src) in [("anonymous", anon), ("named twin", named)] {
        let (out, rc) = run(src);
        assert_eq!(rc, Some(0), "{name} ({which}): expected exit 0:\n{out}");
        assert_eq!(lines(&out), expect, "{name} ({which}):\n{out}");
    }
}

fn loud(name: &str, src: &str, needle: &str) {
    let (out, rc) = run(src);
    assert_eq!(rc, Some(1), "{name}: expected exit 1:\n{out}");
    assert!(
        out.contains(needle),
        "{name}: expected `{needle}` in:\n{out}"
    );
}

const REG_ANON: &str = "  typedef struct packed {
    struct packed { logic [1:0] q; } a;
    struct packed { logic q; logic qe; } b;
  } r_t;
";
const REG_NAMED: &str = "  typedef struct packed { logic [1:0] q; } a_t;
  typedef struct packed { logic q; logic qe; } b_t;
  typedef struct packed { a_t a; b_t b; } r_t;
";

fn module_with(types: &str, body: &str) -> String {
    format!("module t;\n{types}{body}endmodule\n")
}

#[test]
fn read_write_pattern_and_bits() {
    let read = "  r_t r;
  initial begin r = 5'b10110; #1 $display(\"A a.q=%b b.q=%b b.qe=%b r=%b a=%b b=%b bits=%0d %0d\", r.a.q, r.b.q, r.b.qe, r, r.a, r.b, $bits(r), $bits(r.b)); $finish; end
";
    twin(
        "read",
        &module_with(REG_ANON, read),
        &module_with(REG_NAMED, read),
        "A a.q=01 b.q=1 b.qe=0 r=0110 a=01 b=10 bits=4 2",
    );
    let write = "  r_t r;
  initial begin r = '0; r.a.q = 2'b11; r.b.qe = 1'b1; r.a.q[0] = 1'b0; #1 $display(\"A r=%b\", r); r.b = 2'b10; #1 $display(\"B r=%b\", r); $finish; end
";
    twin(
        "write",
        &module_with(REG_ANON, write),
        &module_with(REG_NAMED, write),
        "A r=1001|B r=1010",
    );
    // iverilog rejects a pattern into a nested struct; sv2v → iverilog and verilator.
    let pattern = "  r_t r, s, u;
  initial begin
    r = '{a: '{q: 2'b01}, b: '{q: 1'b1, qe: 1'b0}};
    s = '{'{2'b10}, '{1'b0, 1'b1}};
    u = '{default: '0};
    #1 $display(\"A r=%b s=%b u=%b\", r, s, u); $finish;
  end
";
    twin(
        "pattern",
        &module_with(REG_ANON, pattern),
        &module_with(REG_NAMED, pattern),
        "A r=0110 s=1001 u=0000",
    );
}

#[test]
fn package_imports_and_collisions() {
    let pkg = |types: &str| format!("package p;\n{types}endpackage\n");
    let use_ = |imp: &str, decl: &str| {
        format!(
            "module t;\n  {imp}\n  {decl} r;\n  initial begin r = 5'b10110; #1 $display(\"A a.q=%b b.q=%b b.qe=%b\", r.a.q, r.b.q, r.b.qe); $finish; end\nendmodule\n"
        )
    };
    for (cell, imp, decl) in [
        ("wildcard", "import p::*;", "r_t"),
        ("explicit", "import p::r_t;", "r_t"),
        ("scoped", "", "p::r_t"),
    ] {
        twin(
            cell,
            &format!("{}{}", pkg(REG_ANON), use_(imp, decl)),
            &format!("{}{}", pkg(REG_NAMED), use_(imp, decl)),
            "A a.q=01 b.q=1 b.qe=0",
        );
    }
    // Two packages each declare `r_t` with different anonymous layouts; iverilog
    // rejects the struct-typed scoped declarations, sv2v → iverilog and verilator.
    let collide = |p: &str, q: &str| {
        format!(
            "package p;\n{p}endpackage\npackage q;\n{q}endpackage\nmodule t;\n  p::r_t r; q::r_t s;\n  initial begin r = 5'b10110; s = 5'b10110; #1 $display(\"A r.a.q=%b r.b.qe=%b s.a.q=%b s.a.qe=%b s.b.q=%b\", r.a.q, r.b.qe, s.a.q, s.a.qe, s.b.q); $finish; end\nendmodule\n"
        )
    };
    let q_anon = "  typedef struct packed {
    struct packed { logic q; logic qe; } a;
    struct packed { logic [1:0] q; } b;
  } r_t;
";
    let q_named = "  typedef struct packed { logic q; logic qe; } a_t;
  typedef struct packed { logic [1:0] q; } b_t;
  typedef struct packed { a_t a; b_t b; } r_t;
";
    twin(
        "collide",
        &collide(REG_ANON, q_anon),
        &collide(REG_NAMED, q_named),
        "A r.a.q=01 r.b.qe=0 s.a.q=0 s.a.qe=1 s.b.q=10",
    );
    // An importer's own `r_t` beside the package's: the scoped one keeps its layout.
    let shadow = |types: &str| {
        format!(
            "package p;\n{types}endpackage\nmodule t;\n  import p::*;\n  typedef struct packed {{ logic [3:0] a; logic b; }} r_t;\n  r_t r; p::r_t s;\n  initial begin r = 5'b10110; s = 4'b1011; #1 $display(\"A r.a=%b r.b=%b s.a.q=%b s.b.qe=%b\", r.a, r.b, s.a.q, s.b.qe); $finish; end\nendmodule\n"
        )
    };
    twin(
        "import_shadow",
        &shadow(REG_ANON),
        &shadow(REG_NAMED),
        "A r.a=1011 r.b=0 s.a.q=10 s.b.qe=1",
    );
    // A package that wildcard-imports another and nests its type inside an
    // anonymous member.
    twin(
        "cross_package",
        "package p;
  typedef struct packed { logic [1:0] h; logic l; } e_t;
endpackage
package q;
  import p::*;
  typedef struct packed {
    struct packed { e_t x; logic y; } m;
    logic z;
  } r_t;
endpackage
module t;
  import q::*;
  r_t r;
  initial begin r = 5'b10110; #1 $display(\"A h=%b l=%b y=%b z=%b\", r.m.x.h, r.m.x.l, r.m.y, r.z); $finish; end
endmodule
",
        "package p;
  typedef struct packed { logic [1:0] h; logic l; } e_t;
endpackage
package q;
  import p::*;
  typedef struct packed { e_t x; logic y; } m_t;
  typedef struct packed {
    m_t m;
    logic z;
  } r_t;
endpackage
module t;
  import q::*;
  r_t r;
  initial begin r = 5'b10110; #1 $display(\"A h=%b l=%b y=%b z=%b\", r.m.x.h, r.m.x.l, r.m.y, r.z); $finish; end
endmodule
",
        "A h=10 l=1 y=1 z=0",
    );
}

#[test]
fn nesting_unions_and_signedness() {
    twin(
        "depth3_in_package",
        "package p;
  typedef struct packed {
    struct packed {
      struct packed {
        struct packed { logic [1:0] k; } d;
        logic e;
      } c;
      logic f;
    } b;
    logic g;
  } r_t;
endpackage
module t;
  import p::r_t;
  r_t r;
  initial begin r = 5'b10110; #1 $display(\"A k=%b e=%b f=%b g=%b c=%b\", r.b.c.d.k, r.b.c.e, r.b.f, r.g, r.b.c); r.b.c.d.k = 2'b01; #1 $display(\"B r=%b\", r); $finish; end
endmodule
",
        "package p;
  typedef struct packed { logic [1:0] k; } d_t;
  typedef struct packed { d_t d; logic e; } c_t;
  typedef struct packed { c_t c; logic f; } b_t;
  typedef struct packed {
    b_t b;
    logic g;
  } r_t;
endpackage
module t;
  import p::r_t;
  r_t r;
  initial begin r = 5'b10110; #1 $display(\"A k=%b e=%b f=%b g=%b c=%b\", r.b.c.d.k, r.b.c.e, r.b.f, r.g, r.b.c); r.b.c.d.k = 2'b01; #1 $display(\"B r=%b\", r); $finish; end
endmodule
",
        "A k=10 e=1 f=1 g=0 c=101|B r=01110",
    );
    // An anonymous union member holding an anonymous struct, and an anonymous struct
    // inside a union typedef.
    let union_body = "  r_t r;
  initial begin r = 5'b10111; #1 $display(\"A a=%b h=%b l=%b c=%b u=%b\", r.u.a, r.u.s.h, r.u.s.l, r.c, r.u); $finish; end
";
    twin(
        "union_member",
        &module_with(
            "  typedef struct packed {
    union packed { logic [3:0] a; struct packed { logic [1:0] h; logic [1:0] l; } s; } u;
    logic c;
  } r_t;
",
            union_body,
        ),
        &module_with(
            "  typedef struct packed { logic [1:0] h; logic [1:0] l; } s_t;
  typedef union packed { logic [3:0] a; s_t s; } u_t;
  typedef struct packed { u_t u; logic c; } r_t;
",
            union_body,
        ),
        "A a=1011 h=10 l=11 c=1 u=1011",
    );
    let in_union = "  u_t u;
  initial begin u = 4'b1101; #1 $display(\"A h=%b l=%b v=%b\", u.s.h, u.s.l, u.v); $finish; end
";
    twin(
        "struct_in_union_typedef",
        &module_with(
            "  typedef union packed { struct packed { logic [1:0] h; logic [1:0] l; } s; logic [3:0] v; } u_t;\n",
            in_union,
        ),
        &module_with(
            "  typedef struct packed { logic [1:0] h; logic [1:0] l; } s_t;\n  typedef union packed { s_t s; logic [3:0] v; } u_t;\n",
            in_union,
        ),
        "A h=11 l=01 v=1101",
    );
    // `struct packed signed` as the anonymous member: the whole member reads signed.
    let signed = "  r_t r;
  initial begin r = 8'b1110_1100; #1 $display(\"A m=%0d n=%0d a=%0d b=%0d gt=%0d\", r.m, r.n, r.m.a, r.n.b, r.m < 0); $finish; end
";
    twin(
        "signed_member",
        &module_with(
            "  typedef struct packed {
    struct packed signed { logic [3:0] a; } m;
    struct packed { logic signed [3:0] b; } n;
  } r_t;
",
            signed,
        ),
        &module_with(
            "  typedef struct packed signed { logic [3:0] a; } m_t;
  typedef struct packed { logic signed [3:0] b; } n_t;
  typedef struct packed { m_t m; n_t n; } r_t;
",
            signed,
        ),
        "A m=-2 n=12 a=14 b=-4 gt=1",
    );
    // One anonymous type, two member names; a 2-state anonymous member in a
    // 4-state struct (the whole struct starts x) and an all-2-state one (starts 0).
    let comma = "  r_t r;
  initial begin r = 5'b10110; #1 $display(\"A aq=%b aqe=%b bq=%b bqe=%b c=%b\", r.a.q, r.a.qe, r.b.q, r.b.qe, r.c); $finish; end
";
    twin(
        "comma_list",
        &module_with(
            "  typedef struct packed {
    struct packed { logic q; logic qe; } a, b;
    logic c;
  } r_t;
",
            comma,
        ),
        &module_with(
            "  typedef struct packed { logic q; logic qe; } ab_t;
  typedef struct packed { ab_t a, b; logic c; } r_t;
",
            comma,
        ),
        "A aq=1 aqe=0 bq=1 bqe=1 c=0",
    );
    let states = "  r_t r; s_t s;
  initial begin #1 $display(\"A r=%b s=%b\", r, s); $finish; end
";
    // iverilog; IEEE 1800-2017 §6.8 / §7.2.1: a struct with a 4-state member starts
    // x, an all-2-state one 0 (verilator is no x oracle, `r=000`; sv2v → iverilog `z`).
    twin(
        "two_state",
        &module_with(
            "  typedef struct packed {
    struct packed { bit [1:0] q; } a;
    struct packed { logic q; } b;
  } r_t;
  typedef struct packed {
    struct packed { bit [1:0] q; } a;
  } s_t;
",
            states,
        ),
        &module_with(
            "  typedef struct packed { bit [1:0] q; } a_t;
  typedef struct packed { logic q; } b_t;
  typedef struct packed { a_t a; b_t b; } r_t;
  typedef struct packed { a_t a; } s_t;
",
            states,
        ),
        "A r=xxx s=00",
    );
}

#[test]
fn scopes_ports_and_arrays() {
    // Function-local typedef (sv2v rejects the function-local typedef), generate
    // block, `$unit`.
    let fn_body = |types: &str| {
        format!(
            "module t;\n  function automatic logic [2:0] f(logic [4:0] v);\n{types}    r_t r;\n    r = v;\n    return {{r.a.q[0], r.b.q, r.b.qe}};\n  endfunction\n  initial begin #1 $display(\"A f=%b\", f(5'b10110)); $finish; end\nendmodule\n"
        )
    };
    twin(
        "function_local",
        &fn_body(REG_ANON),
        &fn_body(REG_NAMED),
        "A f=110",
    );
    let gen = |types: &str| {
        format!(
            "module t;\n  if (1) begin : g\n{types}    r_t r;\n    initial begin r = 5'b10110; #1 $display(\"A a.q=%b b.q=%b b.qe=%b\", r.a.q, r.b.q, r.b.qe); $finish; end\n  end\nendmodule\n"
        )
    };
    twin(
        "generate_block",
        &gen(REG_ANON),
        &gen(REG_NAMED),
        "A a.q=01 b.q=1 b.qe=0",
    );
    let unit = |types: &str| {
        format!(
            "{types}module t;\n  r_t r;\n  initial begin r = 5'b10110; #1 $display(\"A a.q=%b b.q=%b b.qe=%b\", r.a.q, r.b.q, r.b.qe); $finish; end\nendmodule\n"
        )
    };
    twin(
        "unit_scope",
        &unit(REG_ANON),
        &unit(REG_NAMED),
        "A a.q=01 b.q=1 b.qe=0",
    );
    let port = |types: &str| {
        format!(
            "package p;\n{types}endpackage\nmodule c(input p::r_t i, output p::r_t o);\n  assign o.a.q = ~i.a.q;\n  assign o.b = {{i.b.qe, i.b.q}};\nendmodule\nmodule t;\n  p::r_t x, y;\n  c u(.i(x), .o(y));\n  initial begin x = 5'b10110; #1 $display(\"A y=%b ya=%b ybqe=%b\", y, y.a.q, y.b.qe); $finish; end\nendmodule\n"
        )
    };
    twin(
        "struct_ports",
        &port(REG_ANON),
        &port(REG_NAMED),
        "A y=1001 ya=10 ybqe=1",
    );
    // iverilog rejects the unpacked array of a struct; sv2v → iverilog and verilator.
    let arrays = "  r_t arr [2];
  r_t [1:0] pa;
  initial begin arr[0] = 5'b10110; arr[1] = 5'b01001; pa = 10'b10110_01101;
    #1 $display(\"A %b %b %b %b %b\", arr[1].a.q, arr[0].b.qe, pa[1].a.q, pa[0].b.q, pa[0].b.qe);
    arr[1].b.q = 1'b1; pa[0].a.q = 2'b00; #1 $display(\"B %b %b\", arr[1], pa); $finish; end
";
    twin(
        "arrays",
        &module_with(REG_ANON, arrays),
        &module_with(REG_NAMED, arrays),
        "A 10 0 11 0 1|B 1011 11000001",
    );
    // A localparam / parameter of the type, and a member width over a localparam.
    let params = "  localparam r_t P = 5'b10110;
  parameter r_t Q = '{a: '{q: 2'b11}, b: '{q: 1'b0, qe: 1'b1}};
  initial begin #1 $display(\"A P.a.q=%b P.b.qe=%b Q=%b Q.a.q=%b\", P.a.q, P.b.qe, Q, Q.a.q); $finish; end
";
    twin(
        "typed_parameters",
        &module_with(REG_ANON, params),
        &module_with(REG_NAMED, params),
        "A P.a.q=01 P.b.qe=0 Q=1101 Q.a.q=11",
    );
    let lp = "  r_t r;
  initial begin r = 6'b101101; #1 $display(\"A q=%b qe=%b b=%b bits=%0d\", r.a.q, r.a.qe, r.b, $bits(r_t)); $finish; end
";
    twin(
        "localparam_width",
        &module_with(
            "  localparam int W = 3;
  typedef struct packed {
    struct packed { logic [W-1:0] q; logic qe; } a;
    logic [1:0] b;
  } r_t;
",
            lp,
        ),
        &module_with(
            "  localparam int W = 3;
  typedef struct packed { logic [W-1:0] q; logic qe; } a_t;
  typedef struct packed { a_t a; logic [1:0] b; } r_t;
",
            lp,
        ),
        "A q=101 qe=1 b=01 bits=6",
    );
}

#[test]
fn refused_shapes_stay_loud() {
    // Packed dimensions after the anonymous body: the named twin `ab_t [1:0] m;` is
    // refused at parse too (all three oracles `A m1a=1 m0b=1 c=0`).
    loud(
        "packed_dims",
        "module t;
  typedef struct packed {
    struct packed { logic a; logic b; } [1:0] m;
    logic c;
  } r_t;
  r_t r;
  initial begin r = 5'b10110; #1 $display(\"A m1a=%b\", r.m[1].a); $finish; end
endmodule
",
        "packed dimensions on an anonymous member type are unsupported in v1",
    );
    // An unpacked anonymous member of a packed struct: iverilog and verilator reject
    // it (IEEE 1800-2017 §7.2.1 allows only packed members), sv2v → iverilog `A c=1`.
    loud(
        "unpacked_member",
        "module t;
  typedef struct packed {
    struct { logic [1:0] q; } a;
    logic c;
  } r_t;
  r_t r;
  initial begin r = 3'b101; #1 $display(\"A c=%b\", r.c); $finish; end
endmodule
",
        "`packed` after an anonymous struct/union member type",
    );
    // A packed anonymous member of an UNPACKED struct: refused like its named twin.
    loud(
        "unpacked_outer",
        "module t;
  typedef struct {
    struct packed { logic [1:0] q; } a;
    int x;
  } r_t;
  r_t r;
  initial begin r.a = 2'b10; r.x = 5; #1 $display(\"A a.q=%b x=%0d\", r.a.q, r.x); $finish; end
endmodule
",
        "a packed-struct member of an unpacked struct is unsupported in v1",
    );
    // Seventy levels: refused at the nesting cap rather than recursing on.
    let mut inner = String::from("logic z;");
    for i in 0..70 {
        inner = format!("struct packed {{ {inner} }} m{i};");
    }
    loud(
        "nesting_cap",
        &format!(
            "module t;\n  typedef struct packed {{ {inner} }} r_t;\n  r_t r;\n  initial begin r = 1'b1; #1 $display(\"A r=%b\", r); $finish; end\nendmodule\n"
        ),
        "anonymous struct/union member nesting too deep (cap 64)",
    );
}

#[test]
fn ten_levels_resolve() {
    // Below the cap: a ten-level chain reads its leaf (all three oracles `A r=1 z=1`).
    let mut inner = String::from("logic z;");
    for i in 0..10 {
        inner = format!("struct packed {{ {inner} }} m{i};");
    }
    let (out, rc) = run(&format!(
        "module t;\n  typedef struct packed {{ {inner} }} r_t;\n  r_t r;\n  initial begin r = 1'b1; #1 $display(\"A r=%b z=%b\", r, r.m9.m8.m7.m6.m5.m4.m3.m2.m1.m0.z); $finish; end\nendmodule\n"
    ));
    assert_eq!(rc, Some(0), "{out}");
    assert_eq!(lines(&out), "A r=1 z=1", "{out}");
}
