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
//! An instance OVERRIDE carries the same shape: §4.5.459 gave an OVERRIDABLE type
//! parameter two more synthesized value parameters per dim (`T$d<i>a` / `T$d<i>b`,
//! the dim's declared endpoints) and registered `[T$d0a:T$d0b]` as the typedef's
//! dim, so `#(.T(b_t))` replaces the element width AND the extents together. What
//! it cannot replace is the dim COUNT — the declarators were stamped with the
//! default's dim LIST once, at parse — and `shape_flags` records that count so a
//! mismatch is loud in both directions: a dim-losing override trips the group's
//! `$fatal`, a dim-ADDING one trips E3002 on the missing `T$d…` half. Both are
//! pinned below.
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
fn an_override_replaces_the_element_and_the_extents_together() {
    // ⚠️ This assertion was the refusal until §4.5.459, for the reason the
    // refusal's own message gave: `T$w`/`T$s` could not carry `b_t`'s dims, so an
    // override that replaced only the WIDTH would have kept the default's `[0:2]`
    // and answered `$bits` 48 where both oracles answer 64. The dims are carried
    // now — `T$d0a`/`T$d0b` — so both halves move together and the cell is the
    // value both oracles measure, not a refusal. The extents differ from the
    // default's on purpose (3 elements → 4): that is the half `T$w` alone could
    // never have carried.
    let (out, rc) = run("`timescale 1ns/1ns\ntypedef logic [7:0] a_t [0:2];\n\
         typedef logic [15:0] b_t [0:3];\n\
         module m #(parameter type T = a_t) ();\n  T v;\n  \
         initial $display(\"bits=%0d size=%0d\", $bits(v), $size(v,1));\nendmodule\n\
         module top; m #(.T(b_t)) u1(); initial #10 $finish; endmodule\n");
    assert_eq!(rc, Some(0), "{out}");
    assert!(out.contains("bits=64 size=4"), "{out}");
}

#[test]
fn an_override_cannot_change_the_dimension_count() {
    // ⚠️ The silent-wrong this slice had to NOT ship, in both directions. The
    // module's declarators are stamped with the DEFAULT's dim list at parse time,
    // so an override may move the extents but never the arity.
    //
    // (a) a dim-LOSING override: it parses (it always could), and the dim count in
    // `T$s` is what catches it — without it the module would have declared
    // `logic [15:0] v [0:2]` and answered `$bits` 48 where both oracles answer 16.
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
    // (b) the mirror — a dim-ADDING override on a scalar default. The `T$d0…`
    // halves the override pushes name parameters the module never declared, so it
    // is E3002, reported ONCE and against `T` rather than the synthesized carrier.
    let (out, rc) = run("`timescale 1ns/1ns\ntypedef logic [15:0] b_t [0:3];\n\
         module m #(parameter type T = logic) ();\n  T v;\n  \
         initial $display(\"bits=%0d\", $bits(v));\nendmodule\n\
         module top; m #(.T(b_t)) u1(); initial #10 $finish; endmodule\n");
    assert_ne!(rc, Some(0), "{out}");
    assert!(out.contains("VITA-E3002"), "{out}");
    assert!(out.contains("dimension COUNT"), "{out}");
    assert_eq!(
        out.matches("VITA-E3002").count(),
        1,
        "one report, not two\n{out}"
    );
    assert!(
        !out.contains("$d0"),
        "the carrier name must not leak\n{out}"
    );
    // (c) ⚠️ The near-miss the soundness lens caught: a type parameter that is not
    // OVERRIDABLE here (a body `parameter type` under a module that has a header,
    // §12.2) has no `T$w` either, so the arity message would have been a false
    // diagnosis of a real refusal. It is suppressed — the two carrier reports are
    // the whole story, and iverilog refuses this for the same reason it gives
    // ("Parameter cannot be overridden in the scope it has been declared in").
    let (out, rc) = run("`timescale 1ns/1ns\ntypedef logic [7:0] a_t [0:2];\n\
         typedef logic [15:0] b_t [0:3];\n\
         module m #(parameter int W = 1) ();\n  parameter type T = a_t;\n  T v;\n  \
         initial $display(\"bits=%0d\", $bits(v));\nendmodule\n\
         module top; m #(.W(2), .T(b_t)) u(); initial #10 $finish; endmodule\n");
    assert_ne!(rc, Some(0), "{out}");
    assert!(
        !out.contains("dimension COUNT"),
        "not an arity failure\n{out}"
    );
    assert_eq!(
        out.matches("VITA-E3002").count(),
        2,
        "just the two carriers\n{out}"
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

#[test]
fn the_overrides_extents_reach_every_declaration_readout() {
    // The extents are the half `T$w` could never carry, so every readout that
    // depends on WHERE the elements are — not just how many bits they total — is
    // asserted: `$size`, `$low`/`$high`, and an element write/read at the
    // override's own bounds. `[1:4]` is the cell that separates "carries the
    // COUNT" from "carries the DECLARED endpoints"; a size-only carrier would put
    // `lo` at 0 and address a word the design never wrote.
    //
    // All five instances measured 3-way identical (iverilog 13.0, verilator
    // 5.052); `u0` is the no-override control and `u1` the identity override,
    // which must stay on the default's answer.
    let (out, rc) = run("`timescale 1ns/1ns\ntypedef logic [7:0]  a_t [0:3];\n\
         typedef logic [15:0] b_t [0:3];\ntypedef logic [15:0] w_t [0:7];\n\
         typedef logic [15:0] r_t [1:4];\n\
         module m #(parameter type T = a_t) ();\n  T v;\n  \
         initial begin v[$low(v)] = 16'h11; v[$high(v)] = 16'h44;\n    \
         $display(\"bits=%0d sz=%0d lo=%0d hi=%0d vl=%h vh=%h\", $bits(T), $size(v,1), \
         $low(v), $high(v), v[$low(v)], v[$high(v)]); end\nendmodule\n\
         module top;\n  m u0(); m #(.T(a_t)) u1(); m #(.T(b_t)) u2();\n  \
         m #(.T(w_t)) u3(); m #(.T(r_t)) u4();\n  initial #10 $finish;\nendmodule\n");
    assert_eq!(rc, Some(0), "{out}");
    for want in [
        "bits=32 sz=4 lo=0 hi=3 vl=11 vh=44", // u0 and u1: default, and the identity override
        "bits=64 sz=4 lo=0 hi=3 vl=0011 vh=0044", // u2: element 8 -> 16
        "bits=128 sz=8 lo=0 hi=7 vl=0011 vh=0044", // u3: extent 4 -> 8
        "bits=64 sz=4 lo=1 hi=4 vl=0011 vh=0044", // u4: bounds [0:3] -> [1:4]
    ] {
        assert!(out.contains(want), "missing `{want}`\n{out}");
    }
    assert_eq!(
        out.matches("bits=32 sz=4 lo=0 hi=3 vl=11 vh=44").count(),
        2,
        "the control and the identity override must both stay on the default\n{out}"
    );
}

#[test]
fn every_override_spelling_carries_the_same_dims() {
    // The POSITIONAL spelling is the one that could have misaligned: the group now
    // declares 2 + 2*dims parameters, so an override has to push its dim values in
    // the same order — measured `bits=64 sz=4`, the named spelling's answer and
    // both oracles'.
    let (out, rc) = run("`timescale 1ns/1ns\ntypedef logic [7:0] a_t [0:3];\n\
         typedef logic [15:0] b_t [0:3];\n\
         module m #(parameter type T = a_t) ();\n  T v;\n  \
         initial $display(\"bits=%0d sz=%0d\", $bits(T), $size(v,1));\nendmodule\n\
         module top; m #(b_t) u(); initial #10 $finish; endmodule\n");
    assert_eq!(rc, Some(0), "{out}");
    assert!(out.contains("bits=64 sz=4"), "{out}");
    // A package-SCOPED override reaches the same channel through the same parse.
    let (out, rc) = run(
        "`timescale 1ns/1ns\npackage pk; typedef logic [15:0] pb_t [0:3]; endpackage\n\
         typedef logic [7:0] a_t [0:2];\n\
         module m #(parameter type T = a_t) ();\n  T v;\n  \
         initial $display(\"bits=%0d sz=%0d lo=%0d\", $bits(T), $size(v,1), $low(v));\nendmodule\n\
         module top; m #(.T(pk::pb_t)) u(); initial #10 $finish; endmodule\n",
    );
    assert_eq!(rc, Some(0), "{out}");
    assert!(out.contains("bits=64 sz=4 lo=0"), "{out}");
    // ⚠️⚠️ The alignment cell. The group declares 2 + 2*dims parameters now, so a
    // POSITIONAL override followed by a VALUE parameter is where a miscount would
    // show: `W` would silently keep its default while a dim endpoint ate the 5.
    // All four instances match iverilog, including the no-override control and an
    // identity type override with a different `W`.
    let (out, rc) = run("`timescale 1ns/1ns\ntypedef logic [7:0] a_t [0:2];\n\
         typedef logic [15:0] b_t [0:3];\n\
         module m #(parameter type T = a_t, parameter int W = 1) ();\n  T v;\n  \
         initial $display(\"R bits=%0d sz=%0d W=%0d\", $bits(T), $size(v,1), W);\nendmodule\n\
         module top;\n  m u0(); m #(b_t, 5) u1(); m #(.T(b_t), .W(5)) u2(); m #(a_t, 7) u3();\n  \
         initial #10 $finish;\nendmodule\n");
    assert_eq!(rc, Some(0), "{out}");
    for want in [
        "R bits=24 sz=3 W=1",
        "R bits=64 sz=4 W=5",
        "R bits=24 sz=3 W=7",
    ] {
        assert!(out.contains(want), "missing `{want}`\n{out}");
    }
    assert_eq!(
        out.matches("R bits=64 sz=4 W=5").count(),
        2,
        "positional == named\n{out}"
    );
    // A TWO-dimensional override, the arity the census showed working end to end:
    // both oracles `bits=128 s1=4 s2=2` for the identity and `bits=48 s1=2 s2=3`
    // for a different element and different extents on both dims.
    let (out, rc) = run(
        "`timescale 1ns/1ns\ntypedef logic [15:0] b2_t [0:3][0:1];\n\
         typedef logic [7:0] c2_t [0:1][0:2];\n\
         module m #(parameter type T = b2_t) ();\n  T v;\n  \
         initial $display(\"R bits=%0d s1=%0d s2=%0d\", $bits(T), $size(v,1), $size(v,2));\n\
         endmodule\n\
         module top; m #(.T(b2_t)) u1(); m #(.T(c2_t)) u2(); initial #10 $finish; endmodule\n",
    );
    assert_eq!(rc, Some(0), "{out}");
    assert!(out.contains("R bits=128 s1=4 s2=2"), "{out}");
    assert!(out.contains("R bits=48 s1=2 s2=3"), "{out}");
    // ⚠️ The `[N]` spelling normalizes to `[0:N-1]` on BOTH sides of the channel,
    // which is what makes it exactly two values per dim whatever either side
    // wrote. All three tools read `[3]` as lo 0 / hi 2, so the normalization is
    // observationally free — this is that control.
    let (out, rc) = run(
        "`timescale 1ns/1ns\ntypedef logic [7:0] a_t [0:2];\ntypedef logic [7:0] sz_t [3];\n\
         module m #(parameter type T = a_t) ();\n  T v;\n  \
         initial $display(\"bits=%0d sz=%0d lo=%0d hi=%0d\", $bits(T), $size(v,1), \
         $low(v), $high(v));\nendmodule\n\
         module top; m #(.T(sz_t)) u(); initial #10 $finish; endmodule\n",
    );
    assert_eq!(rc, Some(0), "{out}");
    assert!(out.contains("bits=24 sz=3 lo=0 hi=2"), "{out}");
}

#[test]
fn a_localparam_type_keeps_its_literal_dims() {
    // ⚠️ The carrier is OPT-IN to an OVERRIDABLE type parameter, and this is why.
    // `$bits(T)` of a symbolic-extent type is built by `sym_range_width`, which
    // answers only when a bound NAMES an overridable parameter — so giving a
    // `localparam type` / package one synthesized extents would turn a literal
    // fold into a decline. Both spellings stay on the default's literal dims.
    let (out, rc) = run("`timescale 1ns/1ns\ntypedef logic [7:0] a_t [0:2];\n\
         module m ();\n  localparam type T = a_t;\n  T v;\n  \
         initial $display(\"bits=%0d sz=%0d\", $bits(T), $size(v,1));\nendmodule\n\
         module top; m u(); initial #10 $finish; endmodule\n");
    assert_eq!(rc, Some(0), "{out}");
    assert!(out.contains("bits=24 sz=3"), "{out}");
    // The second non-overridable spelling: a BODY `parameter type` under a module
    // that already has a parameter header (§12.2 — the header is the overridable
    // list, so this one is not overridable either).
    let (out, rc) = run("`timescale 1ns/1ns\ntypedef logic [7:0] a_t [0:2];\n\
         module m #(parameter int W = 1) ();\n  parameter type T = a_t;\n  T v;\n  \
         initial $display(\"bits=%0d sz=%0d w=%0d\", $bits(T), $size(v,1), W);\nendmodule\n\
         module top; m #(.W(2)) u(); initial #10 $finish; endmodule\n");
    assert_eq!(rc, Some(0), "{out}");
    assert!(out.contains("bits=24 sz=3 w=2"), "{out}");
}
