//! §5.2 row 67a: a class body is a scope (IEEE 1800-2017 §8.3). A class property
//! binds through the declaration paths every scope shares, and before this row its
//! binding outlived `endclass`: the enclosing scope's own variable of that name then
//! read through the property's type (a module `st1 s` after `class K; st2 s; endclass`
//! read `s.a` as `st2`'s member), or stayed hidden behind it (a plain property hid a
//! module struct variable, or a later `import p::*` of that name — E3010). The parser
//! now snapshots the scope after the class header, restores it at `endclass`, and
//! keeps each class's record of what its body bound, which `extends` replays into a
//! derived class's body so an inherited property stays in scope there.
//!
//! Oracles: verilator 5.052 `--binary` on every cell; iverilog 13.0 where it runs the
//! cell (it refuses the rest: "sorry" on a member path of a class property in a method,
//! a syntax error on a parameterized class, an assertion on a class in a generate block
//! or on a packed-array property); sv2v 0.0.13 has no classes. Every
//! moved cell also carries its hand-spelled twin's line — the same design with the
//! class property renamed so nothing shadows anything, run on vita before this row
//! (35993da4) — the value this row must reproduce without adding a new arm.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_clsscope_{}_{n}", std::process::id()));
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

/// The `A …` lines, in order, joined by `|`.
fn lines(out: &str) -> String {
    out.lines()
        .filter(|l| l.starts_with("A "))
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

/// Two struct types whose `a` members sit at different bits: `8'h56` reads `a=5`
/// through `st1` and `a=16` through `st2`; `8'hD6` reads `0d` and `16`.
const ST: &str = "  typedef struct packed { logic [3:0] a; logic [3:0] b; } st1;
  typedef struct packed { logic [1:0] b; logic [5:0] a; } st2;
";

/// `module m` holding `ST` and `body`, instantiated by a `top` that finishes at 20.
fn cell(body: &str) -> String {
    format!("module m;\n{ST}{body}endmodule\nmodule top; m u(); initial #20 $finish; endmodule\n")
}

#[test]
fn a_property_binding_ends_at_endclass() {
    // Each cell: iverilog and verilator (or verilator alone, `(vl)`) print the
    // expected line; the twin (`st2 sk`) printed it on 35993da4, where the cell
    // printed the value in brackets.
    let leak = "  st1 s;\n  class K; st2 s; endclass\n";
    // read [a=16]
    value(
        "leak_rd",
        &cell(&format!(
            "{leak}  initial begin s = 8'h56; #1 $display(\"A a=%h\", s.a); end\n"
        )),
        "A a=5",
    );
    // member write [s=96]
    value(
        "leak_wr",
        &cell(&format!(
            "{leak}  initial begin s = 8'h56; s.b = 4'hA; #1 $display(\"A s=%h\", s); end\n"
        )),
        "A s=5a",
    );
    // assignment pattern [s=c4]
    value(
        "leak_pat",
        &cell(&format!(
            "{leak}  initial begin s = '{{4'h3, 4'h4}}; #1 $display(\"A s=%h\", s); end\n"
        )),
        "A s=34",
    );
    // a literal part-select of the member [a=16 p=3]
    value(
        "leak_partsel",
        &cell(&format!(
            "{leak}  initial begin s = 8'h56; #1 $display(\"A a=%h p=%h\", s.a, s.a[2:1]); end\n"
        )),
        "A a=5 p=2",
    );
    // `rand` property [a=16]
    value(
        "leak_rand",
        &cell(
            "  st1 s;\n  class K; rand st2 s; endclass
  initial begin s = 8'h56; #1 $display(\"A a=%h\", s.a); end\n",
        ),
        "A a=5",
    );
    // parameterized class (vl) [a=16]
    value(
        "leak_param_class",
        &cell(
            "  st1 s;\n  class K #(int N = 1); st2 s; endclass
  initial begin s = 8'h56; #1 $display(\"A a=%h\", s.a); end\n",
        ),
        "A a=5",
    );
    // packed array of struct (vl) [a=16]
    value(
        "leak_packed_array",
        &cell(
            "  st1 [1:0] s;\n  class K; st2 [1:0] s; endclass
  initial begin s = 16'h5600; s[1] = 8'h56; #1 $display(\"A a=%h\", s[1].a); end\n",
        ),
        "A a=5",
    );
    // a class in a generate block, read in the same block after `endclass` (vl) [w=16]
    value(
        "leak_generate_block",
        &cell(
            "  st1 s; logic [7:0] w;
  if (1) begin : g
    class K; st2 s; endclass
    assign w = {4'h0, s.a};
  end
  initial begin s = 8'h56; #1 $display(\"A w=%h\", w); end\n",
        ),
        "A w=05",
    );
    // the class's own method still reads its property (vl) [ga=16 a=16]
    value(
        "leak_method_self",
        &cell(
            "  st1 s;
  class K; st2 s;
    function logic [5:0] ga(); return s.a; endfunction
  endclass
  K k;
  initial begin k = new; k.s = 8'hD6; s = 8'h56; #1 $display(\"A ga=%h a=%h\", k.ga(), s.a); end\n",
        ),
        "A ga=16 a=5",
    );
}

#[test]
fn a_property_binding_of_an_enum_or_md_parameter_ends_at_endclass() {
    // enum `.name()` / `.next()` on the module's variable [n=B1 nx=0]; iverilog and
    // verilator `n=A1 nx=2`.
    value(
        "leak_enum",
        "module m;
  typedef enum logic [1:0] {A0, A1, A2} e1_t;
  typedef enum logic [1:0] {B0 = 2'd2, B1 = 2'd1, B2 = 2'd0} e2_t;
  e1_t e;
  class K; e2_t e; endclass
  initial begin e = A1; #1 $display(\"A n=%s nx=%0d\", e.name(), e.next()); end
endmodule
module top; m u(); initial #20 $finish; endmodule
",
        "A n=A1 nx=2",
    );
    // a plain property named like a module multi-dimensional packed localparam: the
    // parameter's element select (vl) [p1=0]
    value(
        "leak_md_param",
        "module m;
  localparam logic [1:0][3:0] P = 8'hA5;
  class K; logic [7:0] P; endclass
  initial begin #1 $display(\"A p1=%h\", P[1]); end
endmodule
module top; m u(); initial #20 $finish; endmodule
",
        "A p1=a",
    );
    // a plain property named like a module localparam: the module's range folds the
    // localparam (iverilog and verilator `bx=4`; unchanged).
    value(
        "localparam_after_class",
        "module m;
  localparam int W = 4;
  class K; logic [7:0] W; endclass
  logic [W-1:0] x;
  initial begin #1 $display(\"A bx=%0d\", $bits(x)); end
endmodule
module top; m u(); initial #20 $finish; endmodule
",
        "A bx=4",
    );
}

#[test]
fn a_plain_property_no_longer_hides_the_enclosing_name() {
    // A plain property hid the module's struct variable [E3010 `s.a`]; iverilog and
    // verilator `a=5`, the twin `a=5`.
    value(
        "leak_plain",
        &cell(
            "  st1 s;\n  class K; logic [7:0] s; endclass
  initial begin s = 8'h56; #1 $display(\"A a=%h\", s.a); end\n",
        ),
        "A a=5",
    );
    // A plain property's name was a module declaration, so a later `import p::*` did
    // not bind p's struct variable [E3010 `pv.a`]; iverilog and verilator `a=5`, the
    // twin `a=5`.
    value(
        "leak_local_name",
        "package p;
  typedef struct packed { logic [3:0] a; logic [3:0] b; } st1;
  st1 pv;
endpackage
module m;
  class K; logic [7:0] pv; endclass
  import p::*;
  initial begin pv = 8'h56; #1 $display(\"A a=%h\", pv.a); end
endmodule
module top; m u(); initial #20 $finish; endmodule
",
        "A a=5",
    );
}

#[test]
fn an_inherited_property_is_in_scope_in_the_derived_class() {
    // These were right before this row only because the base's binding leaked into
    // the module; `extends` now replays the base's record. verilator `ga=16`
    // (iverilog: "sorry" on the member path in the method), unchanged.
    let base = "  st1 s;\n  class B; st2 s; endclass\n";
    let ga = "    function logic [5:0] ga(); return s.a; endfunction\n";
    let run_d =
        "  D d;\n  initial begin d = new; d.s = 8'hD6; #1 $display(\"A ga=%h\", d.ga()); end\n";
    value(
        "inherit_one_level",
        &cell(&format!(
            "{base}  class D extends B;\n{ga}  endclass\n{run_d}"
        )),
        "A ga=16",
    );
    // A's property through B into C: B's record carries what it replayed.
    value(
        "inherit_two_levels",
        &cell(&format!(
            "  st1 s;\n  class A; st2 s; endclass\n  class B extends A; endclass
  class C extends B;\n{ga}  endclass
  C c;\n  initial begin c = new; c.s = 8'hD6; #1 $display(\"A ga=%h\", c.ga()); end\n"
        )),
        "A ga=16",
    );
    value(
        "inherit_parameterized_base",
        &cell(&format!(
            "  st1 s;\n  class B #(int N = 1); st2 s; endclass\n  class D extends B;\n{ga}  endclass\n{run_d}"
        )),
        "A ga=16",
    );
    // An inherited property's `'{…}` (iverilog and verilator `ds=42`).
    value(
        "inherit_pattern",
        &cell(&format!(
            "{base}  class D extends B;
    function void sp(); s = '{{2'h1, 6'h2}}; endfunction
  endclass
  D d;\n  initial begin d = new; d.sp(); #1 $display(\"A ds=%h\", d.s); end\n"
        )),
        "A ds=42",
    );
    // An inherited enum property's `.next()` (verilator `nx=0`).
    value(
        "inherit_enum",
        "module m;
  typedef enum logic [1:0] {A0, A1, A2} e1_t;
  typedef enum logic [1:0] {B0 = 2'd2, B1 = 2'd1, B2 = 2'd0} e2_t;
  e1_t e;
  class B; e2_t e; endclass
  class D extends B;
    function logic [1:0] nx(); return e.next(); endfunction
  endclass
  D d;
  initial begin d = new; d.e = B1; #1 $display(\"A nx=%0d\", d.nx()); end
endmodule
module top; m u(); initial #20 $finish; endmodule
",
        "A nx=0",
    );
    // The derived class's own `st1 s` shadows the inherited `st2 s` (verilator `ga=5`).
    value(
        "derived_shadows_inherited",
        &cell(
            "  class B; st2 s; endclass
  class D extends B;
    st1 s;
    function logic [3:0] ga(); return s.a; endfunction
  endclass
  D d;
  initial begin d = new; d.s = 8'h56; #1 $display(\"A ga=%h\", d.ga()); end\n",
        ),
        "A ga=5",
    );
    // The module reads its own `st1 s` after a base and a derived class (vl) [a=16].
    value(
        "module_after_base_and_derived",
        &cell(&format!(
            "{base}  class D extends B;\n{ga}  endclass
  initial begin s = 8'h56; #1 $display(\"A a=%h\", s.a); end\n"
        )),
        "A a=5",
    );
    // A unit-scope base is a base for a module's class: class records are not
    // cleared at a container's start (vl) [ga=0d a=5].
    value(
        "unit_scope_base",
        &format!(
            "{ST}class B; st2 s; endclass
module m;
  st1 s;
  class D extends B;\n{ga}  endclass
  D d;
  initial begin d = new; d.s = 8'hD6; s = 8'h56; #1 $display(\"A ga=%h a=%h\", d.ga(), s.a); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"
        ),
        "A ga=16 a=5",
    );
}

#[test]
fn an_inherited_plain_property_hides_the_enclosing_struct_variable() {
    // The base's PLAIN `logic [7:0] s` is replayed as an unbind, so the derived
    // method's `s.a` does not read the module's `st1 s` (all three refuse it: `s` is a
    // vector). The module's own `s.a` after the classes is no longer refused.
    let src = cell(
        "  st1 s;
  class B; logic [7:0] s; endclass
  class D extends B;
    function logic [3:0] ga(); return s.a; endfunction
  endclass
  D d;
  initial begin d = new; d.s = 8'hD6; s = 8'h56; #1 $display(\"A ga=%h a=%h\", d.ga(), s.a); end\n",
    );
    loud(
        "inherited_plain",
        &src,
        "t.sv:7:32: error[VITA-E3010] E-ELAB-UNRESOLVED-NAME: undeclared hierarchical name `s.a`",
    );
    let (out, _) = run(&src);
    assert!(!out.contains("t.sv:10:"), "module `s.a` refused:\n{out}");
}

#[test]
fn a_class_in_a_generate_block_is_scoped_to_the_block() {
    // The generate block's class `B` is the block's: the module's `D extends B` takes
    // the module's `B` (`st1 s`), whose record the block's same-named class must not
    // replace (verilator `ga=0d`; iverilog asserts in pform). Unchanged.
    value(
        "generate_block_class_record",
        &cell(
            "  class B; st1 s; endclass
  if (1) begin : g
    class B; st2 s; endclass
  end
  class D extends B;
    function logic [5:0] ga(); return s.a; endfunction
  endclass
  D d;
  initial begin d = new; d.s = 8'hD6; #1 $display(\"A ga=%h\", d.ga()); end\n",
        ),
        "A ga=0d",
    );
    // The module reads its own `st1 s` after the generate block (verilator `a=5`).
    value(
        "after_generate_block",
        &cell(
            "  st1 s;
  if (1) begin : g
    class K; st2 s; endclass
  end
  initial begin s = 8'h56; #1 $display(\"A a=%h\", s.a); end\n",
        ),
        "A a=5",
    );
}

/// The `a=…` field of the single `A` line.
fn field_a(out: &str) -> String {
    lines(out)
        .split_whitespace()
        .find(|f| f.starts_with("a="))
        .unwrap_or("")
        .to_string()
}

#[test]
fn class_scope_forward_use_keeps_the_module_variable() {
    // ROADMAP §2 "class-scope forward use": a method reading a property declared
    // after it, and a derived class declared before its base (IEEE-illegal without
    // `typedef class`; vita accepts it). The module's own `s.a` is right (verilator
    // `a=5`; 35993da4 `a=16`). `ga` is the residual line's known-wrong value — vita
    // `0d`, verilator `16` — recorded here, not asserted: the method's `s.a` binds when
    // `s` is not yet the class's.
    let (out, rc) = run(&cell(
        "  st1 s;
  class K;
    function logic [5:0] ga(); return s.a; endfunction
    st2 s;
  endclass
  K k;
  initial begin k = new; k.s = 8'hD6; s = 8'h56; #1 $display(\"A ga=%h a=%h\", k.ga(), s.a); end\n",
    ));
    assert_eq!(rc, Some(0), "{out}");
    assert_eq!(field_a(&out), "a=5", "property_after_method:\n{out}");
    let (out, rc) = run(&cell(
        "  st1 s;
  class D extends B;
    function logic [5:0] ga(); return s.a; endfunction
  endclass
  class B; st2 s; endclass
  D d;
  initial begin d = new; d.s = 8'hD6; s = 8'h56; #1 $display(\"A ga=%h a=%h\", d.ga(), s.a); end\n",
    ));
    assert_eq!(rc, Some(0), "{out}");
    assert_eq!(field_a(&out), "a=5", "derived_before_base:\n{out}");
}

/// A `$unit` base class `B` (`st2 s`, `st2` = `{b[1:0], a[5:0]}`) and a module's
/// `D extends B` reading `s.a`, with `body` between the module header and `D`.
fn unit_base(body: &str, use_s: &str, show: &str) -> String {
    format!(
        "typedef struct packed {{ logic [1:0] b; logic [5:0] a; }} st2;
package q; typedef struct packed {{ logic [3:0] a; logic [3:0] b; }} st2; endpackage
class B; st2 s; endclass
module m;
{body}  class D extends B;
{use_s}  endclass
  D d;
  initial begin d = new; {show} end
endmodule
module top; m u(); initial #20 $finish; endmodule
"
    )
}

#[test]
fn an_inherited_property_keeps_the_type_its_class_declared_it_with() {
    // §5.2 row 67b: a class record stores each property's declaration KEY, so a `$unit`
    // base replayed into a module that declares or imports another type of that name
    // reads the type the base declared the property with (IEEE 1800-2017 §8.13). Before
    // row 67b the record named the type by NAME and such a property was replayed
    // unbound (E3010 / E3009; the unchecked replay of row 67a's review read the
    // module's type, `ga=0d` / `ds=12`). Oracles: verilator `ga=16` on all three;
    // verilator and iverilog `ds=42` on the pattern; sv2v has no classes.
    let ga = "    function logic [5:0] ga(); return s.a; endfunction\n";
    let show = "d.s = 8'hD6; #1 $display(\"A ga=%h\", d.ga());";
    for (name, body) in [
        (
            "module_typedef",
            "  typedef struct packed { logic [3:0] a; logic [3:0] b; } st2;\n",
        ),
        ("wildcard_import", "  import q::*;\n"),
        ("explicit_import", "  import q::st2;\n"),
    ] {
        value(name, &unit_base(body, ga, show), "A ga=16");
    }
    value(
        "module_typedef_pattern",
        &unit_base(
            "  typedef struct packed { logic [3:0] a; logic [3:0] b; } st2;\n",
            "    function void sp(); s = '{2'h1, 6'h2}; endfunction\n",
            "d.sp(); #1 $display(\"A ds=%h\", d.s);",
        ),
        "A ds=42",
    );
    // An enum property of a `$unit` base: its binding is withheld in a module's derived
    // class (the enum-method-width lane, `replay_class_record`), so `e.next()` stays
    // refused there (verilator `nx=2`, the base's labels).
    loud(
        "module_enum",
        "typedef enum logic [1:0] {A0, A1, A2, A3} e_t;
class B; e_t e; function void set1(); e = e_t'(2'd1); endfunction endclass
module m;
  typedef enum logic [1:0] {Z0 = 2'd3, Z1 = 2'd1, Z2 = 2'd0, Z3 = 2'd2} e_t;
  class D extends B;
    function logic [1:0] nx(); return e.next(); endfunction
  endclass
  D d;
  initial begin d = new; d.set1(); #1 $display(\"A nx=%0d\", d.nx()); end
endmodule
module top; m u(); initial #20 $finish; endmodule
",
        "error[VITA-E3009]",
    );
    // A nested member type re-declared in the module: `st2` is unchanged and its member
    // `i` keeps the `in_t` it was declared with (verilator `gx=3` = the renamed twin
    // below; a by-name replay read the module's `in_t`, `gx=6`; row 67a refused it).
    let nested = |in_name: &str| {
        format!(
            "typedef struct packed {{ logic [1:0] x; logic [1:0] y; }} in_t;
typedef struct packed {{ in_t i; logic [3:0] b; }} st2;
class B; st2 s; endclass
module m;
  typedef struct packed {{ logic [2:0] x; logic y; }} {in_name};
  class D extends B;
    function logic [3:0] gx(); return s.i.x; endfunction
  endclass
  D d;
  initial begin d = new; d.s = 8'hD6; #1 $display(\"A gx=%h\", d.gx()); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"
        )
    };
    value("nested_member_type", &nested("in_t"), "A gx=3");
    value("nested_member_type_other_name", &nested("in_m"), "A gx=3");
    // Controls (verilator `ga=16`): the module re-declares `st2` with the same layout,
    // and a module type of another name.
    value(
        "same_layout",
        &unit_base(
            "  typedef struct packed { logic [1:0] b; logic [5:0] a; } st2;\n",
            ga,
            show,
        ),
        "A ga=16",
    );
    value(
        "other_name",
        &unit_base(
            "  typedef struct packed { logic [3:0] a; logic [3:0] b; } st_m;\n",
            ga,
            show,
        ),
        "A ga=16",
    );
}

#[test]
fn a_randomize_with_block_resolves_in_the_object_class() {
    // IEEE 1800-2017 §18.7: a name in `h.randomize() with { … }` resolves in `h`'s
    // class first. No tool oracle on this host: iverilog has no inline `with` syntax
    // and verilator has no constraint solver here (`randomize` returns 0), so the
    // expected values are the standard's and the run before this row's (35993da4,
    // right only because the class's binding leaked into the module). The class
    // record of `h`'s class is put back around the block; before this fix the block
    // read the module's tables (E3009 where no module variable binds the name, the
    // module enum's labels where one does).
    let e = "  typedef enum logic [1:0] {A0, A1, A2} e1_t;
  typedef enum logic [1:0] {B0 = 2'd2, B1 = 2'd1, B2 = 2'd0} e2_t;
";
    let modvar = |extra: &str, decl: &str, call: &str, show: &str| {
        format!(
            "module m;\n{e}{extra}{decl}  initial begin {call} #1 $display({show}); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"
        )
    };
    // `B0` = 2: `first()` of the class's `e2_t`.
    value(
        "no_shadow",
        &modvar(
            "",
            "  class K; rand e2_t ek; endclass\n  K k; int ok;\n",
            "k = new; ok = k.randomize() with { ek == ek.first(); };",
            "\"A ke=%0d ok=%0d\", k.ek, ok",
        ),
        "A ke=2 ok=1",
    );
    // A module `e1_t e` beside the class's `rand e2_t e`: the class's (not `A0` = 0).
    value(
        "module_variable_of_the_name",
        &modvar(
            "  e1_t e;\n",
            "  class K; rand e2_t e; endclass\n  K k; int ok;\n",
            "e = A1; k = new; ok = k.randomize() with { e == e.first(); };",
            "\"A ke=%0d ok=%0d\", k.e, ok",
        ),
        "A ke=2 ok=1",
    );
    // `num()`: the class's two-label type, not the module's five-label one.
    value(
        "num",
        "module m;
  typedef enum logic [2:0] {A0, A1, A2, A3, A4} e1_t;
  typedef enum logic [1:0] {B0 = 2'd2, B1 = 2'd1} e2_t;
  e1_t e;
  class K; rand bit [7:0] x; rand e2_t e; endclass
  K k; int ok;
  initial begin e = A1; k = new; ok = k.randomize() with { x == e.num(); }; #1 $display(\"A x=%0d ok=%0d\", k.x, ok); end
endmodule
module top; m u(); initial #20 $finish; endmodule
",
        "A x=2 ok=1",
    );
    // An inherited property, through the derived class's record.
    value(
        "inherited",
        &modvar(
            "",
            "  class B; rand e2_t e; endclass\n  class D extends B; endclass\n  D d; int ok;\n",
            "d = new; ok = d.randomize() with { e == e.first(); };",
            "\"A de=%0d ok=%0d\", d.e, ok",
        ),
        "A de=2 ok=1",
    );
    // The statement form, and a call in a module task (`B2` = 0: `last()`).
    value(
        "statement_form",
        &modvar(
            "",
            "  class K; rand e2_t ek; endclass\n  K k;\n",
            "k = new; k.randomize() with { ek == ek.last(); };",
            "\"A ke=%0d\", k.ek",
        ),
        "A ke=0",
    );
    value(
        "module_task",
        &modvar(
            "",
            "  class Item; rand e2_t kind; endclass\n  Item it;
  task automatic run(); it.randomize() with { kind == kind.last(); }; endtask\n",
            "it = new; run();",
            "\"A kind=%0d\", it.kind",
        ),
        "A kind=0",
    );
    // A `$unit` class's handle in a module: the record crosses a container, a lane
    // this row opens, where an enum method's result width is wrong (§2
    // enum-method-width), so its enum binding is withheld and the call stays refused
    // as before this row (IEEE value `kind=0`).
    loud(
        "unit_class",
        "typedef enum logic [1:0] {B0 = 2'd2, B1 = 2'd1, B2 = 2'd0} e2_t;
class Item; rand e2_t kind; endclass
module m;
  Item it; int ok;
  initial begin it = new; ok = it.randomize() with { kind == kind.last(); }; #1 $display(\"A kind=%0d ok=%0d\", it.kind, ok); end
endmodule
module top; m u(); initial #20 $finish; endmodule
",
        "error[VITA-E3009]",
    );
}

#[test]
fn a_rand_property_and_a_property_named_like_a_type_or_constant() {
    // A `rand` property joins the class record (verilator `ga=16 a=5`, `ga=16`).
    value(
        "inherited_rand",
        &cell(
            "  st1 s;
  class B; rand st2 s; endclass
  class D extends B;
    function logic [5:0] ga(); return s.a; endfunction
  endclass
  D d;
  initial begin d = new; d.s = 8'hD6; s = 8'h56; #1 $display(\"A ga=%h a=%h\", d.ga(), s.a); end\n",
        ),
        "A ga=16 a=5",
    );
    value(
        "inherited_rand_no_module_variable",
        &cell(
            "  class B; rand st2 s; endclass
  class D extends B;
    function logic [5:0] ga(); return s.a; endfunction
  endclass
  D d;
  initial begin d = new; d.s = 8'hD6; #1 $display(\"A ga=%h\", d.ga()); end\n",
        ),
        "A ga=16",
    );
    // An inherited `int a_t` property beside a module type `a_t`: the replay declares
    // the name in the derived class, so `$bits(a_t)` there is the property's 32 and the
    // module's `$bits(a_t)` the type's 24 (verilator `gb=32 tb=24`; E3010 before this row).
    value(
        "inherited_property_named_like_a_type",
        "module m #(parameter int N = 3);
  typedef logic [7:0] a_t [0:N-1];
  class B; int a_t; endclass
  class D extends B;
    function int gb(); return $bits(a_t); endfunction
  endclass
  D d;
  initial begin d = new; #1 $display(\"A gb=%0d tb=%0d\", d.gb(), $bits(a_t)); end
endmodule
module top; m u(); initial #20 $finish; endmodule
",
        "A gb=32 tb=24",
    );
    // A property named like a module localparam no longer hides it from the
    // parse-time fold of a later struct member width and `$bits(logic [W-1:0])`
    // (iverilog and verilator `a=35 b=2 bits=8`, `B=10 x=3ff`; E2002 before this row).
    value(
        "localparam_in_a_member_width",
        "module m;
  localparam int W = 6;
  class K; int W; endclass
  typedef struct packed { logic [W-1:0] a; logic [1:0] b; } st;
  st s;
  initial begin s = 8'hD6; #1 $display(\"A a=%h b=%h bits=%0d\", s.a, s.b, $bits(st)); end
endmodule
module top; m u(); initial #20 $finish; endmodule
",
        "A a=35 b=2 bits=8",
    );
    value(
        "localparam_in_a_type_bits",
        "module m;
  localparam int W = 5;
  class K; int W; endclass
  localparam int B = $bits(logic [W-1:0]) + W;
  logic [B-1:0] x;
  initial begin x = '1; #1 $display(\"A B=%0d x=%h\", B, x); end
endmodule
module top; m u(); initial #20 $finish; endmodule
",
        "A B=10 x=3ff",
    );
}

#[test]
fn a_handle_shadowed_in_a_block_and_an_unknown_receiver() {
    // No tool oracle (iverilog has no inline `with`, verilator no constraint solver
    // here); expected values are IEEE 1800-2017's. §18.7: a with-block name resolves
    // in the randomized object's class first, then where the call stands. §6.19.5.2:
    // `last()` is the last label in declaration order, so for
    // `e2_t {B0 = 2, B1 = 1, B2 = 0}` `kind.last()` and `EP.last()` are `B2` = 0.
    let e2 = "  typedef enum logic [1:0] {B0 = 2'd2, B1 = 2'd1, B2 = 2'd0} e2_t;\n";
    let module = |decls: &str, block: &str, with: &str, show: &str| {
        format!(
            "module m;\n{e2}{decls}  Item it; int ok;\n{block}  initial begin #1 it = new; ok = it.randomize() with {{ {with} }}; #1 $display({show}); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"
        )
    };
    // A plain block-local `int it` hides the module's `Item it` in its block only: the
    // later call's receiver is `Item` again (`kind.last()` = 0; refused in round 2).
    value(
        "block_local_plain_redeclaration",
        &module(
            "  class Item; rand e2_t kind; endclass\n",
            "  initial begin : b int it; it = 3; end\n",
            "kind == kind.last();",
            "\"A kind=%0d ok=%0d\", it.kind, ok",
        ),
        "A kind=0 ok=1",
    );
    // `Item` declares no `EP`/`P`/`CFG`, so the caller's constant answers: `EP.last()`
    // = 0; `P[1]` of `localparam logic [1:0][3:0] P = 8'hA5` is element 1 = `4'hA`;
    // `CFG.a` of `localparam st1 CFG = 8'h5C` is `4'h5`. Another class `Other`
    // declaring the name must not hide the caller's constant (round 2 refused the
    // first two, read `P[1]` as the flat bit 0 and refused `CFG.a`).
    let other_ep = "  localparam e2_t EP = B1;\n  class Item; rand bit [7:0] x; endclass\n  class Other; e2_t EP; endclass\n";
    let show_x = "\"A x=%0d ok=%0d\", it.x, ok";
    value(
        "named_block_redeclaration_caller_enum_constant",
        &module(
            other_ep,
            "  initial begin : b int it; it = 3; end\n",
            "x == EP.last();",
            show_x,
        ),
        "A x=0 ok=1",
    );
    value(
        "unnamed_block_redeclaration_caller_enum_constant",
        &module(
            other_ep,
            "  always @(ok) begin logic [3:0] it; it = 4'd1; end\n",
            "x == EP.last();",
            show_x,
        ),
        "A x=0 ok=1",
    );
    value(
        "caller_md_parameter",
        &module(
            "  localparam logic [1:0][3:0] P = 8'hA5;\n  class Item; rand bit [7:0] x; endclass\n  class Other; bit [7:0] P; endclass\n",
            "  initial begin : b int it; it = 3; end\n",
            "x == P[1];",
            "\"A x=%h ok=%0d\", it.x, ok",
        ),
        "A x=0a ok=1",
    );
    value(
        "caller_struct_parameter",
        &module(
            "  typedef struct packed { logic [3:0] a; logic [3:0] b; } st1;\n  localparam st1 CFG = 8'h5C;\n  class Item; rand bit [7:0] x; endclass\n  class Other; int CFG; endclass\n",
            "  initial begin : b int it; it = 3; end\n",
            "x == CFG.a;",
            show_x,
        ),
        "A x=5 ok=1",
    );
    // Controls: no block re-declaration; no class `Other`.
    value(
        "no_block_redeclaration",
        &module(other_ep, "", "x == EP.last();", show_x),
        "A x=0 ok=1",
    );
    value(
        "no_other_class",
        &module(
            "  localparam e2_t EP = B1;\n  class Item; rand bit [7:0] x; endclass\n",
            "  always @(ok) begin logic [3:0] it; it = 4'd1; end\n",
            "x == EP.last();",
            show_x,
        ),
        "A x=0 ok=1",
    );
}

#[test]
fn an_enum_method_on_a_unit_base_property_stays_refused() {
    // ROADMAP §2 enum-method-width: an enum method's result is 32 bits wide in vita
    // instead of the enum's, which a self-determined use exposes. A `$unit` base's
    // enum property in a module's derived class was refused before this row (the
    // container's start cleared the unit's binding); the record replayed across the
    // container would open the wrong width (`cn=02 bn=32 cf=00`; verilator 5.052
    // `A cn=0e bn=2 cf=0c`), so the replay withholds the enum binding there.
    loud(
        "unit_base_enum_method",
        "typedef enum logic [1:0] {A0, A1, A2} e_t;
class B; e_t e; function void set1(); e = e_t'(2'd1); endfunction endclass
module m;
  class D extends B;
    function logic [7:0] cn(); return {2'b11, e.next()}; endfunction
    function int bn(); return $bits(e.next()); endfunction
    function logic [7:0] cf(); return {2'b11, e.first()}; endfunction
  endclass
  D d;
  initial begin d = new; d.set1(); #1 $display(\"A cn=%h bn=%0d cf=%h\", d.cn(), d.bn(), d.cf()); end
endmodule
module top; m u(); initial #20 $finish; endmodule
",
        "error[VITA-E3009]",
    );
    // KNOWN-WRONG, recorded so §2 enum-method-width flips these pins when it lands:
    // a module's own enum variable (iverilog and verilator `cn=e bn=2 cf=c`) and an
    // in-module base class (unchanged by this row; verilator `cn=0e bn=2`).
    value(
        "module_variable_known_wrong",
        "module m;
  typedef enum logic [1:0] {A0, A1, A2} e_t;
  e_t e;
  initial begin e = A1; #1 $display(\"A cn=%h bn=%0d cf=%h\", {2'b11, e.next()}, $bits(e.next()), {2'b11, e.first()}); end
endmodule
module top; m u(); initial #20 $finish; endmodule
",
        "A cn=300000002 bn=32 cf=300000000",
    );
    value(
        "in_module_base_known_wrong",
        "module m;
  typedef enum logic [1:0] {A0, A1, A2} e_t;
  class B; e_t e; function void set1(); e = e_t'(2'd1); endfunction endclass
  class D extends B;
    function logic [7:0] cn(); return {2'b11, e.next()}; endfunction
    function int bn(); return $bits(e.next()); endfunction
  endclass
  D d;
  initial begin d = new; d.set1(); #1 $display(\"A cn=%h bn=%0d\", d.cn(), d.bn()); end
endmodule
module top; m u(); initial #20 $finish; endmodule
",
        "A cn=02 bn=32",
    );
}

#[test]
fn a_with_block_on_a_receiver_of_unknown_class_naming_a_class_property_is_refused() {
    // A cross-instance receiver `u.it`: no class is known for it where the call
    // stands, and the block names `kind`, a property of a visible class, which could
    // be the object's or the caller's. Refused at parse (vita refuses this receiver at
    // elaboration anyway, E3009).
    loud(
        "cross_instance_receiver",
        "typedef enum logic [1:0] {B0 = 2'd2, B1 = 2'd1, B2 = 2'd0} e2_t;
class Item; rand e2_t kind; endclass
module m; Item it; initial it = new; endmodule
module top;
  m u(); int ok;
  initial begin #1 ok = u.it.randomize() with { kind == kind.last(); }; #1 $display(\"A kind=%0d ok=%0d\", u.it.kind, ok); #20 $finish; end
endmodule
",
        "a receiver whose class is known where the call stands",
    );
}
