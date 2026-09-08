//! `parameter type T = a_t` where `a_t` is an UNPACKED-ARRAY typedef — ROADMAP
//! §3 ⑤ⓕ, the declaration subset.
//!
//! `parse_type_param_value` refused any typedef carrying unpacked dims, because
//! the `T$w` / `T$s` value-parameter desugar has no slot for one. The dims do not
//! need a slot there: they ride the TYPEDEF the group already registers for `T`,
//! which is the map every declaration binder reads (`decls.rs` stamps them onto
//! each declarator, `functask.rs` onto a tf-port formal). So `T v;` becomes the
//! `logic [T$w-1:0] v [0:2]` it would have been written as, and `$bits(v)` is 24
//! in vita, iverilog 13.0 and verilator 5.052 alike.
//!
//! The carry is OPT-IN at the one call site that has the carrier: an instance
//! OVERRIDE cannot take it, because `T$w` would replace the width while the
//! default's dims stayed — `$bits` 48 where both oracles answer the override's
//! own 16. Two guards keep that from happening silently, and both are pinned
//! below: the override position refuses a dim-carrying type outright, and
//! `T$s` bit 2 records "the default has dims" so a dim-free override trips the
//! shape guard the group already synthesizes.
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
    let all =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    (all, out.status.code())
}

const TD: &str = "`timescale 1ns/1ns\ntypedef logic [7:0] a_t [0:2];\n";

#[test]
fn a_type_parameter_defaulted_to_an_unpacked_typedef_declares_the_array() {
    // both oracles `bits=24 size=3` / `e0=11 e1=22 e2=33`; PRE was a five-error
    // parse cascade.
    let (out, rc) = run(&format!(
        "{TD}module m #(parameter type T = a_t) ();\n  T v;\n  \
         initial begin v[0] = 8'h11; v[1] = 8'h22; v[2] = 8'h33;\n    \
         $display(\"bits=%0d size=%0d\", $bits(v), $size(v));\n    \
         $display(\"e0=%h e1=%h e2=%h\", v[0], v[1], v[2]); end\nendmodule\n\
         module top; m u(); initial #10 $finish; endmodule\n"
    ));
    assert_eq!(rc, Some(0), "{out}");
    assert!(out.contains("bits=24 size=3"), "{out}");
    assert!(out.contains("e0=11 e1=22 e2=33"), "{out}");
    // the `localparam` spelling of the same declaration (both oracles the same)
    let (out, rc) = run(&format!(
        "{TD}module m; localparam type T = a_t; T v;\n  \
         initial begin v[1] = 8'h55; $display(\"bits=%0d v1=%h\", $bits(v), v[1]); end\n\
         endmodule\nmodule top; m u(); initial #10 $finish; endmodule\n"
    ));
    assert_eq!(rc, Some(0), "{out}");
    assert!(out.contains("bits=24 v1=55"), "{out}");
}

#[test]
fn the_formal_typed_by_such_a_parameter_carries_the_dims_too() {
    // The tf-port formal reads the same typedef map (§4.5.456 built that carry for
    // a plain unpacked typedef), so it follows for free. verilator `sum=102`;
    // iverilog refuses ANY unpacked tf-port formal ("sorry: Subroutine ports with
    // unpacked dimensions are not yet supported"), so it is not an oracle here and
    // the second anchor is vita's own already-green `input a_t x` spelling.
    let (out, rc) = run(&format!(
        "{TD}module m #(parameter type T = a_t) ();\n  T v;\n  \
         function automatic int sum(input T x); sum = x[0] + x[1] + x[2]; endfunction\n  \
         initial begin v[0]=8'h11; v[1]=8'h22; v[2]=8'h33; \
         $display(\"sum=%0d\", sum(v)); end\nendmodule\n\
         module top; m u(); initial #10 $finish; endmodule\n"
    ));
    assert_eq!(rc, Some(0), "{out}");
    assert!(out.contains("sum=102"), "{out}");
}

#[test]
fn a_packed_type_parameter_is_unmoved() {
    // The control twin for the whole slice: a type parameter whose default has no
    // dims keeps `T$w` / `T$s` exactly as before (`shape_flags` bit 2 is 0 for
    // every type value that predates this), all three tools `bits=8 v=ab`.
    let (out, rc) = run("`timescale 1ns/1ns\ntypedef logic [7:0] p_t;\n\
         module m #(parameter type T = p_t) ();\n  T v;\n  \
         initial begin v = 8'hAB; $display(\"bits=%0d v=%h\", $bits(v), v); end\nendmodule\n\
         module top; m u(); initial #10 $finish; endmodule\n");
    assert_eq!(rc, Some(0), "{out}");
    assert!(out.contains("bits=8 v=ab"), "{out}");
}

#[test]
fn an_override_cannot_replace_a_dim_carrying_default() {
    // ⚠️ The silent-wrong this slice had to NOT ship. Both are loud, and each is a
    // different guard.
    //
    // (a) an override that IS an unpacked typedef: refused where it is parsed —
    // `T$w`/`T$s` cannot carry `b_t`'s dims. Oracles run it (`bits=64 size=4`), so
    // this is honest-loud, not correct — the row keeps that residue.
    let (out, rc) = run("`timescale 1ns/1ns\ntypedef logic [7:0] a_t [0:2];\n\
         typedef logic [15:0] b_t [0:3];\n\
         module m #(parameter type T = a_t) ();\n  T v;\n  \
         initial $display(\"bits=%0d\", $bits(v));\nendmodule\n\
         module top; m #(.T(b_t)) u1(); initial #10 $finish; endmodule\n");
    assert_ne!(rc, Some(0), "{out}");
    assert!(
        out.contains("as the type parameter override"),
        "wording pin — the value is the refusal itself\n{out}"
    );
    // (b) an override that is a plain packed type: it parses (it always could),
    // and `T$s` bit 2 is what catches it — without that bit the module would have
    // declared `logic [15:0] v [0:2]` and answered `$bits` 48 where both oracles
    // answer 16.
    let (out, rc) = run(&format!(
        "{TD}module m #(parameter type T = a_t) ();\n  T v;\n  \
         initial $display(\"bits=%0d\", $bits(v));\nendmodule\n\
         module top; m #(.T(logic [15:0])) u1(); initial #10 $finish; endmodule\n"
    ));
    assert_ne!(rc, Some(0), "{out}");
    assert!(out.contains("VITA-F4004"), "{out}");
    assert!(
        !out.contains("bits=48"),
        "the dim-losing width must never be printed\n{out}"
    );
}

#[test]
fn a_cast_to_such_a_type_stays_loud() {
    // `T'(…)` on a dim-carrying type parameter is NO-ORACLE — iverilog aborts on
    // an internal assertion and verilator refuses the cast — and `T$w` is the
    // ELEMENT width, so answering would cast to a third of the type. Loud.
    let (out, rc) = run(&format!(
        "{TD}module m #(parameter type T = a_t) ();\n  T v;\n  \
         initial begin v = T'('{{8'h11, 8'h22, 8'h33}}); $display(\"e0=%h\", v[0]); end\n\
         endmodule\nmodule top; m u(); initial #10 $finish; endmodule\n"
    ));
    assert_ne!(rc, Some(0), "{out}");
    assert!(!out.contains("e0="), "no value may be printed\n{out}");
}

#[test]
fn bits_of_the_bare_type_parameter_name_is_the_element_times_every_dim() {
    // `$bits(T)` on the NAME of a dim-carrying type parameter. `T$w` is the
    // ELEMENT width, so the answer is the symbolic product `T$w × 3` — 24 in
    // iverilog 13.0 and verilator 5.052 alike, where vita raised the
    // `E3010 undeclared net/variable` + `E3009 $bits argument shape unsupported`
    // pair (the argument fell through to the ordinary expression path).
    //
    // Not a constant: the product is an EXPRESSION, so it follows an override of
    // the element width exactly as the packed spelling already did.
    let (out, rc) = run(&format!(
        "{TD}module m #(parameter type T = a_t) ();\n  \
         initial $display(\"bits=%0d\", $bits(T));\nendmodule\n\
         module top; m u(); initial #10 $finish; endmodule\n"
    ));
    assert_eq!(rc, Some(0), "{out}");
    assert!(out.contains("bits=24"), "{out}");
    // A 16-bit element and a 2-D typedef: the answer is a PRODUCT, not the
    // constant 24 (both oracles 32 and 48).
    let (out, rc) = run(
        "`timescale 1ns/1ns\ntypedef logic [15:0] c_t [0:1];\n\
         typedef logic [7:0] d_t [0:2][0:1];\n\
         module m2 #(parameter type T = c_t) (); initial $display(\"b2=%0d\", $bits(T)); endmodule\n\
         module m3 #(parameter type T = d_t) (); initial $display(\"b3=%0d\", $bits(T)); endmodule\n\
         module top; m2 u2(); m3 u3(); initial #10 $finish; endmodule\n",
    );
    assert_eq!(rc, Some(0), "{out}");
    assert!(out.contains("b2=32"), "{out}");
    assert!(out.contains("b3=48"), "{out}");
}

#[test]
fn every_binder_of_a_dim_carrying_type_parameter_answers_bits() {
    // The four spellings that reach the same parse site, all `bits=24` in both
    // oracles. The `localparam` one is the reason the product is built from the
    // type parameter's own record rather than routed through `sym_typedef_bits`:
    // that builder needs the element range to name an OVERRIDABLE parameter, and
    // a `localparam type` does not register one.
    let (out, rc) = run(
        "`timescale 1ns/1ns\ntypedef logic [7:0] a_t [0:2];\n\
         package pk; typedef logic [7:0] a_t [0:2]; endpackage\n\
         module mh #(parameter type T = a_t) (); initial $display(\"h=%0d\", $bits(T)); endmodule\n\
         module mp #(parameter type T = pk::a_t) (); initial $display(\"p=%0d\", $bits(T)); endmodule\n\
         module mb (); parameter type T = a_t; initial $display(\"b=%0d\", $bits(T)); endmodule\n\
         module ml (); localparam type T = a_t; initial $display(\"l=%0d\", $bits(T)); endmodule\n\
         module top; mh uh(); mp up(); mb ub(); ml ul(); initial #10 $finish; endmodule\n",
    );
    assert_eq!(rc, Some(0), "{out}");
    for pin in ["h=24", "p=24", "b=24", "l=24"] {
        assert!(out.contains(pin), "{pin}\n{out}");
    }
    // and as a CONSTANT: `localparam int W = $bits(T)` folded to nothing before
    // (`parameter W value is not a constant: undefined name T`).
    let (out, rc) = run(&format!(
        "{TD}module m #(parameter type T = a_t) ();\n  localparam int W = $bits(T);\n  T v;\n  \
         initial $display(\"W=%0d bitsv=%0d\", W, $bits(v));\nendmodule\n\
         module top; m u(); initial #10 $finish; endmodule\n"
    ));
    assert_eq!(rc, Some(0), "{out}");
    assert!(out.contains("W=24 bitsv=24"), "{out}");
}

#[test]
fn a_dim_free_type_parameter_and_a_shadowed_typedef_keep_their_answers() {
    // The no-move controls for the same parse site. A type parameter WITHOUT
    // dims composes no factor, so `$bits(T)` stays the bare `T$w` it always was
    // (both oracles 8).
    let (out, rc) = run(
        "`timescale 1ns/1ns\n\
         module m #(parameter type T = logic [7:0]) (); initial $display(\"pk=%0d\", $bits(T)); endmodule\n\
         module top; m u(); initial #10 $finish; endmodule\n",
    );
    assert_eq!(rc, Some(0), "{out}");
    assert!(out.contains("pk=8"), "{out}");
    // The TYPEDEF route keeps its `local_decl_names` stand-down: a same-named
    // variable shadows the type and the fold must not claim the name. Only the
    // ROUTE changed, not that guard — so this still answers the variable's 12
    // (verilator's answer; iverilog rejects the source).
    let (out, rc) = run("`timescale 1ns/1ns\nmodule m #(parameter N = 3) ();\n  \
         typedef logic [7:0] b_t [0:N-1];\n  logic [11:0] b_t;\n  \
         initial $display(\"bt=%0d\", $bits(b_t));\nendmodule\n\
         module top; m u(); initial #10 $finish; endmodule\n");
    assert_eq!(rc, Some(0), "{out}");
    assert!(out.contains("bt=12"), "{out}");
}
