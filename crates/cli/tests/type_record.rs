//! §5.2 row 66: the seven name-keyed type maps became one record per declaration
//! (`hdl-parser/src/type_rec.rs`: `type_infos` keyed by declaration, `type_names` holding
//! each name's slots). Each test below is the stay pin of one mutant of that record that
//! the existing killers (`decl_keys`, `typed_assign_pattern`, … — the row's battery) let
//! through; its comment names the mutant (row 66 battery ids M01–M31).
//!
//! Each cell records PRE (vita 01cac4b3) and the oracles' raw `A` lines: Icarus Verilog
//! 13.0 (`iverilog -g2012`), verilator 5.052 (`--binary`). "refuses" = the tool rejects or
//! crashes on the design. A cell pinned at a value or refusal no oracle shares says why
//! (hand-IEEE, or KNOWN-WRONG held).

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_typerec_{}_{n}", std::process::id()));
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

/// Kills M04 (S1 packed layout read from the declaration, not the re-spelled twin).
#[test]
fn m04_an_alias_over_an_import_copy_differing_only_in_a_nested_name_is_fresh() {
    // row66.m04: PRE error[VITA-E3010] undeclared hierarchical write target `v.z`; iverilog A m04 v=89; verilator A m04 v=89.
    // KNOWN-WRONG, held (the alias twin is compared by layout PartialEq, which reads a nested member by its re-spelled name; PRE refuses the member write)
    loud(
        "row66.m04",
        r##"// row 66 M04 discriminator: q's alias `typedef r::out_t out_t` over the unit's `import p::*` copy of p's out_t. The two layouts
// differ only in their nested member's re-spelled name (`p::in_t` vs `r::in_t`), so the alias is fresh and q registers `q::out_t`
package p;
  typedef struct packed {logic [1:0] a; logic [1:0] b;} in_t;
  typedef struct packed {in_t i; logic [3:0] z;} out_t;
endpackage
package r;
  typedef struct packed {logic [1:0] a; logic [1:0] b;} in_t;
  typedef struct packed {in_t i; logic [3:0] z;} out_t;
endpackage
import p::*;
package q;
  typedef r::out_t out_t;
endpackage
module m; q::out_t v; initial begin v = 0; v.z = 4'h9; v.i.a = 2'b10; #1 $display("A m04 v=%h", v); end endmodule
module top; m u(); initial #5 $finish; endmodule
"##,
        "error[VITA-E3010]",
    );
}

/// Kills M09 (record typedef writes a typedef entry).
#[test]
fn m09_a_record_typedef_writes_no_typedef_entry() {
    // grid.g2.sh.vv_rc.mod_fn: PRE A fn 8|A out v=69 b=8; iverilog refuses ("sorry: Unpacked structs not supported"); verilator A fn 8|A out v=69 b=8.
    value(
        "grid.g2.sh.vv_rc.mod_fn",
        r##"// row66 grid: shadow outer vv inner rc at mod_fn
module m #(parameter int W = 4);
  typedef logic [7:0] st;
  function automatic int g();
    typedef struct {logic [3:0] a; logic [3:0] b;} st;
    st v; v = 8'h96;
    return $bits(v);
  endfunction
  initial #1 $display("A fn %0d", g());
  st vo; initial begin vo = 8'h69; #2 $display("A out v=%h b=%0d", vo, $bits(vo)); end
endmodule
module top;
  m u();
  initial #9 $finish;
endmodule
"##,
        "A fn 8|A out v=69 b=8",
    );
}

/// Kills M10 (S1 alias mirror skips a cleared packed slot).
#[test]
fn m10_a_vector_alias_clears_the_packed_slot_it_shadows() {
    // grid2.dr14: PRE error[VITA-E3009] assignment pattern; iverilog refuses ("Packed array assignment pattern expects 8 element(s)"); verilator refuses ("Assignment pattern missed initializing elements").
    loud(
        "grid2.dr14",
        r##"// DR S1 CLEAR (union + packed): the alias NAME st2 already holds an imported union; 'typedef st st2' with st a vector
package p; typedef union packed {logic [7:0] a; logic [7:0] b;} st2; endpackage
module m; import p::*; typedef logic [7:0] st; typedef st st2;
  st2 v; initial begin v = '{4'h1, 4'h2}; #1 $display("A dr14 v=%h b=%0d", v, $bits(v)); end endmodule
module top; m u(); initial #5 $finish; endmodule
"##,
        "error[VITA-E3009]",
    );
}

/// Kills M11 (S1 alias mirror skips a cleared enum slot).
#[test]
fn m11_a_vector_alias_clears_the_enum_slot_it_shadows() {
    // grid2.dr14c: PRE error[VITA-E3009] enum method `v.name` is unavailable; iverilog refuses ("no method name"); verilator refuses (Unsupported: Member call).
    loud(
        "grid2.dr14c",
        r##"// DR S1 CLEAR (enm): the alias name e2 holds an imported folding enum; 'typedef st e2' with st a vector; enum method
package p; typedef enum logic [7:0] {A0, A1, A2} e2; endpackage
module m; import p::*; typedef logic [7:0] st; typedef st e2;
  e2 v; initial begin v = 8'h1; #1 $display("A dr14c v=%0d n=%s", v, v.name()); end endmodule
module top; m u(); initial #5 $finish; endmodule
"##,
        "error[VITA-E3009]",
    );
}

/// Kills M12 (S1 alias mirror skips a cleared union flag).
#[test]
fn m12_a_struct_alias_clears_the_union_flag_it_shadows() {
    // row66.m12: PRE A m12 v=12; iverilog A m12 v=12; verilator A m12 v=12.
    value(
        "row66.m12",
        r##"// row 66 M12 discriminator: the alias NAME st2 holds an imported union; `typedef st st2` with st a packed struct clears the union flag
package p; typedef union packed {logic [7:0] a; logic [7:0] b;} st2; endpackage
module m; import p::*; typedef struct packed {logic [3:0] x; logic [3:0] y;} st; typedef st st2;
  st2 v; initial begin v = '{4'h1, 4'h2}; #1 $display("A m12 v=%h", v); end endmodule
module top; m u(); initial #5 $finish; endmodule
"##,
        "A m12 v=12",
    );
}

/// Kills M13 (S1 alias copies the base symbolic-layout slot).
#[test]
fn m13_an_alias_copies_no_symbolic_slot() {
    // grid.g2.sy.chain.bits: PRE A bits 8; iverilog A bits 8; verilator A bits 8.
    value(
        "grid.g2.sy.chain.bits",
        r##"// row66 grid: body sy, spelling chain, use bits
module m #(parameter int W = 4);
  typedef struct packed {logic [W-1:0] a; logic [3:0] b;} T0;
  typedef T0 T;
  initial #1 $display("A bits %0d", $bits(T));
endmodule
module top;
  m u();
  initial #9 $finish;
endmodule
"##,
        "A bits 8",
    );
}

/// Kills M14 (S1 alias copies the base record slot).
#[test]
fn m14_an_alias_copies_no_record_slot() {
    // grid2.g3.sh.rc_vv.alias.val: PRE A in v=1a5b b=13; iverilog refuses ("sorry: Unpacked structs not supported"); verilator A in v=1a5b b=13.
    value(
        "grid2.g3.sh.rc_vv.alias.val",
        r##"// row66 grid2 (C5): shadow outer rc inner vv at alias, use val
package p;
  typedef struct {logic [4:0] a; logic [4:0] b;} st;
endpackage
module m #(parameter int W = 4);
  import p::*;
  typedef logic [12:0] st;
  typedef st st2;
  st2 vi; initial begin vi = 13'h1A5B; #1 $display("A in v=%h b=%0d", vi, $bits(vi)); end
endmodule
module top;
  m u();
  initial #9 $finish;
endmodule
"##,
        "A in v=1a5b b=13",
    );
}

/// Kills M18 (S3 union flag copied only with an equal packed key).
#[test]
fn m18_an_ifabsent_import_keeps_the_bound_packed_layout() {
    // row66.m18: PRE error[VITA-E3009] assignment pattern; iverilog refuses ("Ambiguous use of b_t"); verilator A m18 v=12.
    // IEEE 1800-2017 26.3: a name imported by two wildcard imports and referenced is an error; PRE is loud (with another message)
    loud(
        "row66.m18",
        r##"// row 66 M18 discriminator: b_t is bound by `import q::*` (a packed struct); `import p::*` offers a union b_t, IfAbsent
package q; typedef struct packed {logic [3:0] x; logic [3:0] y;} b_t; endpackage
package p; typedef union packed {logic [7:0] a; logic [7:0] c;} b_t; endpackage
module m; import q::*; import p::*; b_t v; initial begin v = '{4'h1, 4'h2}; #1 $display("A m18 v=%h", v); end endmodule
module top; m u(); initial #5 $finish; endmodule
"##,
        "error[VITA-E3009]",
    );
}

/// Kills M20 (S2 `freshest` forced on).
#[test]
fn m20_a_package_alias_of_a_differently_keyed_layout_is_fresh() {
    // grid2.dr03b: PRE error[VITA-E3010] undeclared hierarchical write target `v.a`; iverilog refuses ("This assignment requires an explicit cast", then an assertion); verilator refuses (ENUMVALUE).
    loud(
        "grid2.dr03b",
        r##"// DR B-family: as dr03, a member select through p::st (q's layout twinned under p::st?)
package q; typedef struct packed {logic [1:0] a; logic [5:0] b;} st; endpackage
package p; import q::*; typedef enum logic [2:0] {A0, A1, A2} st; endpackage
module m; p::st v; initial begin v = 0; v.a = 2'b11; #1 $display("A dr03b v=%h", v); end endmodule
module top; m u(); initial #5 $finish; endmodule
"##,
        "error[VITA-E3010]",
    );
}

/// Kills M21 (S2 enum twin written for every alias).
#[test]
fn m21_a_package_vector_alias_registers_no_enum_twin() {
    // row66.m21: PRE error[VITA-E3009] enum method `v.name` is unavailable; iverilog refuses ("no method name"); verilator refuses (Unsupported: Member call).
    loud(
        "row66.m21",
        r##"// row 66 M21 discriminator: the unit's `import q::*` binds a folding enum e; package p's vector alias e leaves it, so p::e has no enum twin
package q; typedef enum logic [1:0] {A0, A1, A2} e; endpackage
import q::*;
package p; typedef logic [1:0] e; endpackage
module m; p::e v; initial begin v = 2'd1; #1 $display("A m21 v=%0d n=%s", v, v.name()); end endmodule
module top; m u(); initial #5 $finish; endmodule
"##,
        "error[VITA-E3009]",
    );
}

/// Kills M22 (S2 freshness by packed KEY instead of layout).
#[test]
fn m22_a_package_alias_compares_the_layout_not_its_key() {
    // grid2.dr04: PRE error[VITA-E3010] undeclared hierarchical write target `v.a`; iverilog A dr04 v=c0; verilator A dr04 v=c0.
    // KNOWN-WRONG, held (p::st equals the unit-imported q::st by layout, so p registers no twin of its own; PRE refuses the member write)
    loud(
        "grid2.dr04",
        r##"// DR M22 trap: CU import q::* (bare st = q layout), package r identical layout st2, package p 'typedef r::st2 st;' (equal by PartialEq, other key)
package q; typedef struct packed {logic [1:0] a; logic [5:0] b;} st; endpackage
import q::*;
package r; typedef struct packed {logic [1:0] a; logic [5:0] b;} st2; endpackage
package p; typedef r::st2 st; endpackage
module m; p::st v; initial begin v = 0; v.a = 2'b11; #1 $display("A dr04 v=%h", v); end endmodule
module top; m u(); initial #5 $finish; endmodule
"##,
        "error[VITA-E3010]",
    );
}

/// Kills M23 (S4 RecordBind twin dropped).
#[test]
fn m23_a_package_record_twin_keeps_its_registration_number() {
    // row66.m23: PRE A m23 v=12; iverilog refuses ("Type identifier T has already been declared"); verilator refuses ("Duplicate declaration of TYPEDEF").
    // KNOWN-WRONG, held (vita accepts the redeclared T; the newest registration, the packed struct, decides)
    value(
        "row66.m23",
        r##"// row 66 M23 discriminator: a package record T registered BEFORE a packed struct T; an explicit import keeps that order
package p;
  typedef struct {logic [3:0] a; logic [3:0] b;} T;
  typedef struct packed {logic [3:0] a; logic [3:0] b;} T;
endpackage
module m; import p::T; T v; initial begin v = T'{4'h1, 4'h2}; #1 $display("A m23 v=%h", v); end endmodule
module top; m u(); initial #5 $finish; endmodule
"##,
        "A m23 v=12",
    );
}

/// Kills M24 (S5 packed twin written over the body's own).
#[test]
fn m24_a_package_binding_twin_is_added_only_where_the_twin_has_none() {
    // row66.m24: PRE error[VITA-E3010] undeclared hierarchical write target `v.z`; iverilog A m04 v=89; verilator A m04 v=89.
    // KNOWN-WRONG, held (as row66.m04, with a package variable of out_t)
    loud(
        "row66.m24",
        r##"// row 66 M24 discriminator (with a package variable of out_t): q's alias `typedef r::out_t out_t` over the unit's `import p::*` copy of p's out_t. The two layouts
// differ only in their nested member's re-spelled name (`p::in_t` vs `r::in_t`), so the alias is fresh and q registers `q::out_t`
package p;
  typedef struct packed {logic [1:0] a; logic [1:0] b;} in_t;
  typedef struct packed {in_t i; logic [3:0] z;} out_t;
  out_t pv = 8'h5A;
endpackage
package r;
  typedef struct packed {logic [1:0] a; logic [1:0] b;} in_t;
  typedef struct packed {in_t i; logic [3:0] z;} out_t;
endpackage
import p::*;
package q;
  typedef r::out_t out_t;
endpackage
module m; q::out_t v; initial begin v = 0; v.z = 4'h9; v.i.a = 2'b10; #1 $display("A m04 v=%h", v); end endmodule
module top; m u(); initial #5 $finish; endmodule
"##,
        "error[VITA-E3010]",
    );
}

/// Kills M28 (S4 whole-entry write of the typedef twin).
#[test]
fn m28_a_package_typedef_twin_keeps_the_enum_slot() {
    // grid2.dr06b: PRE A dr06b 0 A0; iverilog refuses ("Type identifier t has already been declared"); verilator refuses ("Duplicate declaration of TYPEDEF").
    // KNOWN-WRONG, held (vita accepts the redeclared t; the enum slot survives the struct item)
    value(
        "grid2.dr06b",
        r##"// DR D: as dr06, enum method on p::t (enm twin written by the Enum item, kept by the Struct item?)
package p;
  typedef enum logic [1:0] {A0, A1, A2} t;
  typedef struct packed {logic [1:0] a; logic [5:0] b;} t;
endpackage
module m; p::t v; initial begin v = v.first(); #1 $display("A dr06b %0d %s", v, v.name()); end endmodule
module top; m u(); initial #5 $finish; endmodule
"##,
        "A dr06b 0 A0",
    );
}

/// Kills M29 (S5 whole-entry write of the binding twin).
#[test]
fn m29_a_package_binding_twin_keeps_the_typedef_slot() {
    // row66.m29: PRE A m29 x=1 n=A1 pv=2; iverilog A m29 x=1 n=A1 pv=2; verilator A m29 x=1 n=A1 pv=2.
    value(
        "row66.m29",
        r##"// row 66 M29 discriminator: as grid2/dr03c, and the importer declares its own variable of p's enum st (the p::st typedef slot)
package q; typedef struct packed {logic [1:0] a; logic [5:0] b;} st; endpackage
package p; import q::*; typedef enum logic [2:0] {A0, A1, A2} st; st pv = A2; endpackage
module m; import p::*; st x; initial begin x = A1; #1 $display("A m29 x=%0d n=%s pv=%0d", x, x.name(), pv); end endmodule
module top; m u(); initial #5 $finish; endmodule
"##,
        "A m29 x=1 n=A1 pv=2",
    );
}

/// Kills M25 (the unit end keeps a `::` name's slots per NAME, not per slot).
#[test]
fn m25_an_escaped_unit_typedef_name_keeps_its_own_slots_at_the_unit_end() {
    // lens-sound.e05: PRE A e05 v=a5 bt=6 bv=8; iverilog A e05 v=a5 bt=8 bv=8; verilator A e05 v=a5 bt=8 bv=8.
    // KNOWN-WRONG, held (an escaped `\q::t` declared in m2 escapes the unit end's `::` keep, so the unit's `\q::t` reads m2's 6-bit struct in `$bits`)
    value(
        "lens-sound.e05",
        r##"// lens-sound e05 (M25, CU-scope typedef = legal everywhere): a CU `\q::t ` 8-bit vector; m2 declares its own 6-bit
// packed struct of the same escaped name (module-local) ; top reads the CU type's $bits and declares a variable of it.
typedef logic [7:0] \q::t ;
module m2; typedef struct packed { logic [2:0] a; logic [2:0] b; } \q::t ; endmodule
module top; \q::t  v; initial begin v = 8'hA5; #1 $display("A e05 v=%h bt=%0d bv=%0d", v, $bits(\q::t ), $bits(v)); $finish; end endmodule
"##,
        "A e05 v=a5 bt=6 bv=8",
    );
}

/// Kills M26 (the unit end keeps a new `::` symbolic slot).
#[test]
fn m26_an_escaped_unit_typedef_name_drops_a_module_local_struct_slot() {
    // lens-sound.e09: PRE A e09 v=a5 b=8; iverilog A e09 v=a5 b=8; verilator A e09 v=a5 b=8.
    value(
        "lens-sound.e09",
        r##"// lens-sound e09 (M26, legal): CU `\q::s ` 8-bit vector ; m1's module-local per-instance struct of that name ;
// top's type parameter defaults to the CU type.
typedef logic [7:0] \q::s ;
module m1 #(parameter P = 3); typedef struct packed { logic [P-1:0] a; logic [P-1:0] b; } \q::s ; endmodule
module top #(parameter type T = \q::s ); T v; initial begin v = 8'hA5; #1 $display("A e09 v=%h b=%0d", v, $bits(v)); $finish; end endmodule
"##,
        "A e09 v=a5 b=8",
    );
}
