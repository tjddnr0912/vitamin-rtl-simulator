//! §3 ⑤ⓕ NON-ARITY axis — an instance override of `parameter type T` may change the
//! type's SIGNEDNESS and its 2-STATE kind, not only its width (ROADMAP §5.2 row 1).
//!
//! `parameter type T` desugars to two value parameters — `T$w` (the width) and `T$s`
//! (the shape: bit 0 signed, bit 1 2-state, bits 2.. the unpacked dim count) — plus a
//! parser typedef `T` = `logic [T$w-1:0]` that every declaration resolves through. The
//! WIDTH rode an expression naming `T$w`, so elaborate folded it per INSTANCE; the
//! signedness and the 2-state kind were plain scalars stamped once per MODULE from the
//! DEFAULT type, so `T$s` had exactly ONE reader — a synthesized
//! `initial if (T$s != <default>) $fatal` that refused the override outright.
//!
//! The carrier this slice adds is `NetVarDecl`/`AnsiPort`/`PortDecl`/`TfPort`.
//! `shape_param`: the name of `T$s`. Elaborate folds it in the instance's own
//! parameter scope and lets bit 0 decide `signed` and bit 1 decide `bit` vs `logic`,
//! by the same route `T$w` already reached the range. The group's guard then compares
//! the ARITY bits only (`T$s >> 2`), because the declarators are still stamped with the
//! default's unpacked dim LIST at parse and that list cannot follow an override.
//!
//! A use of `T` that lands where nothing can re-fold the shape — a packed struct/union
//! member, an enum base, a class property, a function RETURN type, a `T'(e)` cast —
//! keeps the STRICT compare for that `T`, so those designs stay LOUD rather than
//! silently binding the default's shape. Both wordings are pinned below.
//!
//! Oracles: iverilog 13.0 `-g2012` and verilator 5.052 `--binary --timing` agree on
//! every `%0d` / `>>>` / `<0` column pinned here. Verilator is DISQUALIFIED on the
//! 4-state `%b` column (it prints `0` for every 4-state type, including the explicit
//! `logic [7:0] v` twin), so the `x=` assertions are iverilog's text alone and are
//! marked where that matters.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_tpshape_{}_{n}", std::process::id()));
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

/// The shared body of the A/B/C/E/G/H cells: `m1`/`shr`/`neg` are the SIGN
/// observables (2-oracle), `x=` the 2-STATE one (iverilog-only), `bits` the width.
fn body(default_ty: &str, override_ty: &str, prelude: &str) -> String {
    format!(
        "`timescale 1ns/1ns\n{prelude}\
         module m #(parameter type T = {default_ty}) ();\n  T v;\n  initial begin\n    \
         v = -1;   $display(\"bits=%0d m1=%0d\", $bits(T), v);\n    \
         v = -8;   $display(\"shr=%0d neg=%0d\", v >>> 1, (v < 0));\n    \
         v = 'x;   $display(\"x=%b\", v);\n  end\nendmodule\n\
         module top; m #(.T({override_ty})) u (); initial #10 $finish; endmodule\n"
    )
}

fn cell(default_ty: &str, override_ty: &str, bits: &str, shr: &str, xcol: &str) {
    let (out, rc) = run(&body(default_ty, override_ty, ""));
    assert_eq!(rc, Some(0), "{default_ty} -> {override_ty}\n{out}");
    assert!(out.contains(bits), "{default_ty} -> {override_ty}\n{out}");
    assert!(out.contains(shr), "{default_ty} -> {override_ty}\n{out}");
    assert!(
        out.contains(&format!("x={xcol}")),
        "{default_ty} -> {override_ty}\n{out}"
    );
}

// ───────────────────────── 1a. the SIGN axis ─────────────────────────

#[test]
fn an_override_that_only_adds_signedness_is_followed_by_every_declaration() {
    // A1 `logic [7:0]` -> `logic signed [7:0]`. Both oracles: `bits=8 m1=-1` /
    // `shr=-4 neg=1`; iverilog `x=xxxxxxxx` (verilator disqualified on that column).
    // PRE: F4004.
    cell(
        "logic [7:0]",
        "logic signed [7:0]",
        "bits=8 m1=-1",
        "shr=-4 neg=1",
        "xxxxxxxx",
    );
    // A7 — the MIRROR direction, signed default -> unsigned override.
    cell(
        "logic signed [7:0]",
        "logic [7:0]",
        "bits=8 m1=255",
        "shr=124 neg=0",
        "xxxxxxxx",
    );
}

#[test]
fn an_atom_overrides_its_own_unsigned_twin() {
    // A2 `int` -> `int unsigned`, A4 `byte`, A5 `shortint`, A6 `longint` — the
    // keyword-atom channel; both oracles agree on every column.
    cell(
        "int",
        "int unsigned",
        "bits=32 m1=4294967295",
        "shr=2147483644 neg=0",
        &"0".repeat(32),
    );
    cell(
        "byte",
        "byte unsigned",
        "bits=8 m1=255",
        "shr=124 neg=0",
        "00000000",
    );
    cell(
        "shortint",
        "shortint unsigned",
        "bits=16 m1=65535",
        "shr=32764 neg=0",
        &"0".repeat(16),
    );
    cell(
        "longint",
        "longint unsigned",
        "bits=64 m1=18446744073709551615",
        "shr=9223372036854775804 neg=0",
        &"0".repeat(64),
    );
}

#[test]
fn an_atom_default_takes_a_vector_override_of_the_same_width() {
    // A3 `int` -> `logic [31:0]`: the sign AND the 2-state kind both move.
    cell(
        "int",
        "logic [31:0]",
        "bits=32 m1=4294967295",
        "shr=2147483644 neg=0",
        &"x".repeat(32),
    );
    // H1 `logic [7:0]` -> `int`: the mirror — width, sign and 2-state together.
    cell(
        "logic [7:0]",
        "int",
        "bits=32 m1=-1",
        "shr=-4 neg=1",
        &"0".repeat(32),
    );
}

#[test]
fn the_sign_axis_holds_in_every_width_band() {
    // A9 33..64 and A8 >64 — the bands the width carrier already crossed (C4).
    cell(
        "logic [40:0]",
        "logic signed [40:0]",
        "bits=41 m1=-1",
        "shr=-4 neg=1",
        &"x".repeat(41),
    );
    cell(
        "logic [70:0]",
        "logic signed [70:0]",
        "bits=71 m1=-1",
        "shr=-4 neg=1",
        &"x".repeat(71),
    );
}

#[test]
fn the_sign_and_the_width_compose_in_one_override() {
    // E1 `logic [7:0]` -> `logic signed [15:0]`: BOTH follow (the width already did).
    cell(
        "logic [7:0]",
        "logic signed [15:0]",
        "bits=16 m1=-1",
        "shr=-4 neg=1",
        &"x".repeat(16),
    );
}

// ───────────────────────── 1b. the 2-STATE axis ─────────────────────────

#[test]
fn a_four_state_override_of_a_two_state_default_stores_four_state() {
    // B1 `bit [7:0]` -> `logic [7:0]` and B3 `int` -> `integer`. The ONLY moving
    // column is `x=`, and it is 1-ORACLE: iverilog prints `x…x`, verilator `0…0`
    // (it prints `0…0` for the explicit `logic [7:0] v` twin too, so it is
    // disqualified here, not disagreeing). The pin is iverilog's.
    cell(
        "bit [7:0]",
        "logic [7:0]",
        "bits=8 m1=255",
        "shr=124 neg=0",
        "xxxxxxxx",
    );
    cell(
        "int",
        "integer",
        "bits=32 m1=-1",
        "shr=-4 neg=1",
        &"x".repeat(32),
    );
}

#[test]
fn a_two_state_override_of_a_four_state_default_stores_two_state() {
    // B2 `integer` -> `int` and B4 `logic [7:0]` -> `bit [7:0]`. 2-ORACLE on the
    // moving column: both oracles print `x=0…0` (a 2-state variable coerces X to 0).
    cell(
        "integer",
        "int",
        "bits=32 m1=-1",
        "shr=-4 neg=1",
        &"0".repeat(32),
    );
    cell(
        "logic [7:0]",
        "bit [7:0]",
        "bits=8 m1=255",
        "shr=124 neg=0",
        "00000000",
    );
}

// ───────────────────────── controls (unchanged) ─────────────────────────

#[test]
fn a_width_only_or_identical_override_is_unchanged() {
    // C1 / C2 / C3 / C4 — the axis that already worked, re-pinned as the control.
    cell(
        "logic [7:0]",
        "logic [15:0]",
        "bits=16 m1=65535",
        "shr=32764 neg=0",
        &"x".repeat(16),
    );
    cell(
        "logic [7:0]",
        "logic [7:0]",
        "bits=8 m1=255",
        "shr=124 neg=0",
        "xxxxxxxx",
    );
    cell(
        "int",
        "int",
        "bits=32 m1=-1",
        "shr=-4 neg=1",
        &"0".repeat(32),
    );
    cell(
        "logic [7:0]",
        "logic [70:0]",
        "bits=71 m1=2361183241434822606847",
        "shr=1180591620717411303420 neg=0",
        &"x".repeat(71),
    );
}

#[test]
fn a_typedef_named_override_carries_its_own_shape() {
    // G3 — an UNSIGNED typedef override (the control: no shape change).
    let (out, rc) = run(&body("logic [7:0]", "u8_t", "typedef logic [7:0] u8_t;\n"));
    assert_eq!(rc, Some(0), "{out}");
    assert!(out.contains("bits=8 m1=255"), "{out}");
    // G1 — a SIGNED typedef override. Both oracles `m1=-1 shr=-4 neg=1`; PRE F4004.
    let (out, rc) = run(&body(
        "logic [7:0]",
        "s8_t",
        "typedef logic signed [7:0] s8_t;\n",
    ));
    assert_eq!(rc, Some(0), "{out}");
    assert!(out.contains("bits=8 m1=-1"), "{out}");
    assert!(out.contains("shr=-4 neg=1"), "{out}");
    // G2 — the CHAINED alias spelling of the same type reaches the same answer.
    let (out, rc) = run(&body(
        "logic [7:0]",
        "s8b_t",
        "typedef logic signed [7:0] s8_t;\ntypedef s8_t s8b_t;\n",
    ));
    assert_eq!(rc, Some(0), "{out}");
    assert!(out.contains("bits=8 m1=-1"), "{out}");
    assert!(out.contains("shr=-4 neg=1"), "{out}");
}

// ───────────────────── the ARITY axis stays loud, both ways ─────────────────────

const AT: &str = "`timescale 1ns/1ns\ntypedef logic [7:0] a_t [0:2];\n";

#[test]
fn a_dim_losing_override_is_still_loud_with_an_arity_only_message() {
    // D1. The compare is now `T$s >> 2` (the dim COUNT), so the message names only
    // that — the signedness and the 2-state kind are no longer part of it.
    let (out, rc) = run(&format!(
        "{AT}module m #(parameter type T = a_t) ();\n  T v;\n  \
         initial $display(\"bits=%0d dims=%0d\", $bits(v), $dimensions(v));\nendmodule\n\
         module top; m #(.T(logic [15:0])) u (); initial #10 $finish; endmodule\n"
    ));
    assert_eq!(rc, Some(1), "{out}");
    assert!(out.contains("fatal[VITA-F4004]"), "{out}");
    assert!(
        out.contains("the override changes the type's unpacked dimension COUNT"),
        "{out}"
    );
    assert!(
        out.contains(
            "an override may change the width, the unpacked extents, the signedness and the \
             2-state kind; only the dimension count is fixed"
        ),
        "{out}"
    );
}

#[test]
fn a_dim_adding_override_is_still_the_elaborate_reject() {
    // D2 — a different gate (E3002 on the missing `T$d…` half), untouched here.
    let (out, rc) = run(&format!(
        "{AT}module m #(parameter type T = logic [7:0]) ();\n  T v;\n  \
         initial $display(\"bits=%0d\", $bits(v));\nendmodule\n\
         module top; m #(.T(a_t)) u (); initial #10 $finish; endmodule\n"
    ));
    assert_eq!(rc, Some(1), "{out}");
    assert!(out.contains("error[VITA-E3002]"), "{out}");
    assert!(
        out.contains("the override has more unpacked dimensions than the default"),
        "{out}"
    );
}

#[test]
fn a_same_arity_override_still_replaces_the_extents_and_the_element_width() {
    // D3 / D4 — the controls the extents carrier (§4.5.459) earned; unchanged.
    let (out, rc) = run(&format!(
        "{AT}module m #(parameter type T = a_t) ();\n  T v;\n  \
         initial $display(\"bits=%0d dims=%0d\", $bits(v), $dimensions(v));\nendmodule\n\
         module top; m #(.T(a_t)) u (); initial #10 $finish; endmodule\n"
    ));
    assert_eq!(rc, Some(0), "{out}");
    assert!(out.contains("bits=24 dims=2"), "{out}");
    let (out, rc) = run(&format!(
        "{AT}typedef logic [15:0] b_t [0:1];\n\
         module m #(parameter type T = a_t) ();\n  T v;\n  \
         initial $display(\"bits=%0d dims=%0d\", $bits(v), $dimensions(v));\nendmodule\n\
         module top; m #(.T(b_t)) u (); initial #10 $finish; endmodule\n"
    ));
    assert_eq!(rc, Some(0), "{out}");
    assert!(out.contains("bits=32 dims=2"), "{out}");
}

// ───────────────── the other containers that carry the shape ─────────────────

#[test]
fn an_ansi_port_of_type_t_follows_the_override() {
    // Both oracles `port m1=-1 neg=1` / `top r=-1`; PRE F4004.
    let (out, rc) = run("`timescale 1ns/1ns\n\
         module m #(parameter type T = logic [7:0]) (input T a, output T o);\n  \
         assign o = a;\n  initial begin #1; $display(\"port m1=%0d neg=%0d\", o, (o < 0)); end\n\
         endmodule\n\
         module top;\n  logic signed [7:0] w = -1; logic signed [7:0] r;\n  \
         m #(.T(logic signed [7:0])) u (.a(w), .o(r));\n  \
         initial begin #2; $display(\"top r=%0d\", r); #10 $finish; end\nendmodule\n");
    assert_eq!(rc, Some(0), "{out}");
    assert!(out.contains("port m1=-1 neg=1"), "{out}");
    assert!(out.contains("top r=-1"), "{out}");
}

#[test]
fn a_tf_port_formal_of_type_t_follows_the_override() {
    // Both oracles `tf=-4` (the formal is signed, so `>>>` is arithmetic); PRE F4004.
    let (out, rc) = run("`timescale 1ns/1ns\n\
         module m #(parameter type T = logic [7:0]) ();\n  \
         function automatic int f(T x); f = (x >>> 1); endfunction\n  T v;\n  \
         initial begin v = -8; $display(\"tf=%0d\", f(v)); end\nendmodule\n\
         module top; m #(.T(logic signed [7:0])) u (); initial #10 $finish; endmodule\n");
    assert_eq!(rc, Some(0), "{out}");
    assert!(out.contains("tf=-4"), "{out}");
}

#[test]
fn an_interface_header_type_parameter_follows_the_override() {
    // Both oracles `iface m1=-1 neg=1`; PRE F4004 (`[in top.i]`).
    let (out, rc) = run("`timescale 1ns/1ns\n\
         interface ifc #(parameter type T = logic [7:0]) ();\n  T v;\nendinterface\n\
         module top;\n  ifc #(.T(logic signed [7:0])) i ();\n  \
         initial begin i.v = -1; $display(\"iface m1=%0d neg=%0d\", i.v, (i.v < 0)); \
         #10 $finish; end\nendmodule\n");
    assert_eq!(rc, Some(0), "{out}");
    assert!(out.contains("iface m1=-1 neg=1"), "{out}");
}

#[test]
fn a_chained_typedef_of_t_and_a_type_parameter_of_t_both_follow() {
    // `typedef T t2; t2 v;` — the alias copies the carrier. Both oracles `-1` / `1`.
    let (out, rc) = run("`timescale 1ns/1ns\n\
         module m #(parameter type T = logic [7:0]) ();\n  typedef T t2;\n  t2 v;\n  \
         initial begin v = -1; $display(\"chain m1=%0d neg=%0d\", v, (v < 0)); end\nendmodule\n\
         module top; m #(.T(logic signed [7:0])) u (); initial #10 $finish; endmodule\n");
    assert_eq!(rc, Some(0), "{out}");
    assert!(out.contains("chain m1=-1 neg=1"), "{out}");
    // `parameter type U = T;` — `U$s`'s DEFAULT is the EXPRESSION `T$s`, not `T`'s
    // default flags, so overriding `T` alone reaches `U v;`. A literal there would
    // have frozen the default's shape into `U` — a silent-wrong of this very class.
    let (out, rc) = run("`timescale 1ns/1ns\n\
         module m #(parameter type T = logic [7:0], parameter type U = T) ();\n  U v;\n  \
         initial begin v = -1; $display(\"chainp m1=%0d neg=%0d\", v, (v < 0)); end\nendmodule\n\
         module top; m #(.T(logic signed [7:0])) u (); initial #10 $finish; endmodule\n");
    assert_eq!(rc, Some(0), "{out}");
    assert!(out.contains("chainp m1=-1 neg=1"), "{out}");
}

// ───────────── uses that carry NO shape keep the strict guard ─────────────

/// The wording a module with an uncarried use of `T` keeps.
fn assert_strict_loud(out: &str, rc: Option<i32>) {
    assert_eq!(rc, Some(1), "{out}");
    assert!(out.contains("fatal[VITA-F4004]"), "{out}");
    assert!(
        out.contains(
            "the override changes the type's signedness, 2-state kind or unpacked \
                      dimensions"
        ),
        "{out}"
    );
    assert!(
        out.contains("is used here in a position that carries no shape"),
        "{out}"
    );
}

#[test]
fn a_packed_struct_member_of_type_t_keeps_the_strict_guard() {
    // `StructMember` has no shape slot and the flat layout is built at parse, so this
    // stays LOUD. Both oracles run it (`mem=-1`) — an honest-loud gap, not a value.
    let (out, rc) = run("`timescale 1ns/1ns\n\
         module m #(parameter type T = logic [7:0]) ();\n  \
         typedef struct packed { T a; } s_t;\n  s_t s;\n  \
         initial begin s.a = -1; $display(\"mem=%0d\", s.a); end\nendmodule\n\
         module top; m #(.T(logic signed [7:0])) u (); initial #10 $finish; endmodule\n");
    assert_strict_loud(&out, rc);
}

#[test]
fn an_enum_base_of_type_t_keeps_the_strict_guard() {
    let (out, rc) = run("`timescale 1ns/1ns\n\
         module m #(parameter type T = logic [7:0]) ();\n  \
         typedef enum T { A = 1, B = 2 } e_t;\n  e_t e;\n  \
         initial begin e = B; $display(\"enum=%0d\", e); end\nendmodule\n\
         module top; m #(.T(logic signed [7:0])) u (); initial #10 $finish; endmodule\n");
    assert_strict_loud(&out, rc);
}

#[test]
fn a_function_return_type_of_t_keeps_the_strict_guard() {
    // `FunctionDef` has no shape slot either — the return width/sign are stamped once.
    let (out, rc) = run("`timescale 1ns/1ns\n\
         module m #(parameter type T = logic [7:0]) ();\n  \
         function automatic T f(T x); f = x >>> 1; endfunction\n  \
         initial begin T v; v = -8; $display(\"tf=%0d\", f(v)); end\nendmodule\n\
         module top; m #(.T(logic signed [7:0])) u (); initial #10 $finish; endmodule\n");
    assert_strict_loud(&out, rc);
}

// ───────────── the carrier names are not user parameters ─────────────

#[test]
fn an_override_naming_a_synthesized_carrier_is_refused() {
    // Before this slice the strict shape guard caught `#(.T$s(1))` as a SIDE EFFECT
    // (F4004, the wrong reason). The arity-only guard no longer does, so the spelling
    // is refused where it is written. Both oracles reject it too:
    //   iverilog `error: parameter `T$s' not found in `top.u'.`
    //   verilator `%Error-PINNOTFOUND: Parameter not found: 'T$s'`
    for carrier in ["T$s", "T$w", "T$d0a", "T$d0b"] {
        let (out, rc) = run(&format!(
            "`timescale 1ns/1ns\n\
             module m #(parameter type T = logic [7:0]) ();\n  T v;\n  \
             initial $display(\"bits=%0d\", $bits(T));\nendmodule\n\
             module top; m #(.{carrier}(16)) u (); initial #10 $finish; endmodule\n"
        ));
        assert_eq!(rc, Some(1), "{carrier}\n{out}");
        assert!(out.contains("error[VITA-E2002]"), "{carrier}\n{out}");
        assert!(
            out.contains("is an internal type-parameter carrier, not a user parameter"),
            "{carrier}\n{out}"
        );
    }
    // …and an ordinary parameter with a `$` that is NOT a carrier spelling is still
    // overridable (the predicate cuts from the carrier grammar, not from `$`).
    let (out, rc) = run("`timescale 1ns/1ns\n\
         module m #(parameter W$x = 8) ();\n  initial $display(\"w=%0d\", W$x);\nendmodule\n\
         module top; m #(.W$x(16)) u (); initial #10 $finish; endmodule\n");
    assert_eq!(rc, Some(0), "{out}");
    assert!(out.contains("w=16"), "{out}");
}

// ──────── review round 1: the alias / pass-through / frame-local roots ────────

#[test]
fn a_pass_through_override_hands_on_the_outer_instances_shape() {
    // `n #(.T(T))` — the instance override whose VALUE is another type parameter must
    // pass `T$s` as an EXPRESSION, exactly as the `parameter type U = T` DEFAULT does.
    // A literal froze the OUTER parameter's default shape into the inner module: the
    // inner `n` printed `m1=255 neg=0` where verilator prints `-1 1` (iverilog rejects
    // the `#(.T(T))` spelling outright — "Syntax error in parameter value assignment
    // list" — so this is a 1-oracle cell; the mechanism is the same one every other
    // cell here is 2-oracle on). Both instances are checked, so the DEFAULT instance
    // pins that nothing moved for it.
    let (out, rc) = run("`timescale 1ns/1ns\n\
         module n #(parameter type T = logic [7:0]) ();\n  \
         T v; initial begin v = -1; \
         $display(\"n m1=%0d neg=%0d bits=%0d\", v, (v<0), $bits(T)); end\nendmodule\n\
         module m #(parameter type T = logic [7:0]) ();\n  n #(.T(T)) u ();\n  \
         T w; initial begin w = -1; $display(\"m m1=%0d\", w); end\nendmodule\n\
         module top; m #(.T(logic signed [7:0])) a (); m b (); initial #10 $finish; \
         endmodule\n");
    assert_eq!(rc, Some(0), "{out}");
    assert!(out.contains("n m1=-1 neg=1 bits=8"), "{out}");
    assert!(out.contains("m m1=-1"), "{out}");
    assert!(out.contains("n m1=255 neg=0 bits=8"), "{out}");
    assert!(out.contains("m m1=255"), "{out}");
}

#[test]
fn a_pass_through_is_a_carried_use_so_the_guard_still_narrows() {
    // `T` used ONLY in the pass-through (no declaration in `m` at all): the guard of
    // `m` must still narrow — handing `T$s` on IS carrying it. verilator `n m1=-1
    // neg=1`; iverilog rejects the spelling.
    let (out, rc) = run("`timescale 1ns/1ns\n\
         module n #(parameter type T = logic [7:0]) ();\n  \
         T v; initial begin v = -1; $display(\"n m1=%0d neg=%0d\", v, (v<0)); end\nendmodule\n\
         module m #(parameter type T = logic [7:0]) ();\n  n #(.T(T)) u ();\nendmodule\n\
         module top; m #(.T(logic signed [7:0])) a (); initial #10 $finish; endmodule\n");
    assert_eq!(rc, Some(0), "{out}");
    assert!(out.contains("n m1=-1 neg=1"), "{out}");
    // …and with an UNPACKED-array default on both sides, so the pass-through carries
    // the dim extents AND the shape while the arity compare still holds. verilator
    // `n e0=-1 neg=1 bits=24`.
    let (out, rc) = run("`timescale 1ns/1ns\ntypedef logic [7:0] a_t [0:2];\n\
         module n #(parameter type T = a_t) ();\n  T v; initial begin v[0] = -1; \
         $display(\"n e0=%0d neg=%0d bits=%0d\", v[0], (v[0]<0), $bits(v)); end\nendmodule\n\
         module m #(parameter type T = a_t) ();\n  n #(.T(T)) u ();\nendmodule\n\
         module top;\n  typedef logic signed [7:0] s_t [0:2];\n  m #(.T(s_t)) a ();\n  \
         initial #10 $finish;\nendmodule\n");
    assert_eq!(rc, Some(0), "{out}");
    assert!(out.contains("n e0=-1 neg=1 bits=24"), "{out}");
}

#[test]
fn a_non_overridable_alias_type_parameter_still_carries_the_shape() {
    // `localparam type U = T; U u;` — `U` is NOT overridable, so it synthesizes no
    // guard, but its `U$s` VALUE is the expression `T$s`: an ordinary parameter
    // elaborate folds per instance. Registering the carrier only for an OVERRIDABLE
    // parameter made `U u;` bind the DEFAULT's shape — `u=255 neg=0` where both
    // oracles print `-1 1`.
    let (out, rc) = run("`timescale 1ns/1ns\n\
         module m #(parameter type T = logic [7:0]) ();\n  localparam type U = T;\n  \
         U u; initial begin u = -1; $display(\"u=%0d neg=%0d\", u, (u<0)); end\nendmodule\n\
         module top; m #(.T(logic signed [7:0])) u (); initial #10 $finish; endmodule\n");
    assert_eq!(rc, Some(0), "{out}");
    assert!(out.contains("u=-1 neg=1"), "{out}");
    // the BODY `parameter type U = T` spelling in a module that HAS a header — also
    // non-overridable (IEEE §6.20.1), same carrier.
    let (out, rc) = run("`timescale 1ns/1ns\n\
         module m #(parameter type T = logic [7:0]) ();\n  parameter type U = T;\n  \
         U u; initial begin u = -1; $display(\"u=%0d neg=%0d\", u, (u<0)); end\nendmodule\n\
         module top; m #(.T(logic signed [7:0])) u (); initial #10 $finish; endmodule\n");
    assert_eq!(rc, Some(0), "{out}");
    assert!(out.contains("u=-1 neg=1"), "{out}");
}

#[test]
fn an_uncarried_use_of_an_alias_marks_the_root_parameters_guard() {
    // `parameter type U = T; typedef enum U {…}` — the enum base is an UNCARRIED use
    // of `U`, and `U` has no guard of its own, so without the transitive alias the
    // mark landed on `U$s` and `T`'s guard was narrowed anyway: the enum printed
    // `e=255 u=255` where both oracles print `-1 -1`. The alias map makes it `T`'s
    // guard, so the design is LOUD.
    let (out, rc) = run("`timescale 1ns/1ns\n\
         module m #(parameter type T = logic [7:0]) ();\n  parameter type U = T;\n  \
         typedef enum U { A = -1, B = 2 } e_t;\n  e_t e; U u;\n  \
         initial begin e = A; u = -1; $display(\"e=%0d u=%0d\", e, u); end\nendmodule\n\
         module top; m #(.T(logic signed [7:0])) u (); initial #10 $finish; endmodule\n");
    assert_strict_loud(&out, rc);
}

#[test]
fn a_two_state_override_reaches_every_subprogram_local() {
    // `two_state_nets` is DERIVED from `intro_kind`, and the frame-local writers
    // inserted the RAW declared kind: a module-level `T mv` coerced X→0 correctly
    // while a static task local, an automatic task local and a function local all
    // printed `xxxxxxxx`. Both oracles print `00000000` in all four positions.
    let (out, rc) = run("`timescale 1ns/1ns\n\
         module m #(parameter type T = logic [7:0]) ();\n  \
         task t; T tl; begin tl = 'x; $display(\"t x=%b\", tl); end endtask\n  \
         task automatic ta; T tl; begin tl = 'x; $display(\"ta x=%b\", tl); end endtask\n  \
         function automatic int f(input int k); T loc; loc = 'x; \
         $display(\"f x=%b\", loc); return 0; endfunction\n  \
         T mv; initial begin mv = 'x; $display(\"m x=%b\", mv); t(); ta(); void'(f(0)); end\n\
         endmodule\n\
         module top; m #(.T(bit [7:0])) u (); initial #10 $finish; endmodule\n");
    assert_eq!(rc, Some(0), "{out}");
    for l in [
        "m x=00000000",
        "t x=00000000",
        "ta x=00000000",
        "f x=00000000",
    ] {
        assert!(out.contains(l), "missing `{l}`\n{out}");
    }
    // The MIRROR: a 4-state override of a 2-state default keeps X in all three
    // positions. iverilog `xxxxxxxx` everywhere; verilator is DISQUALIFIED on this
    // column (it prints `00000000` for the explicit 4-state twin too).
    let (out, rc) = run("`timescale 1ns/1ns\n\
         module m #(parameter type T = bit [7:0]) ();\n  \
         task t; T tl; begin tl = 'x; $display(\"t x=%b\", tl); end endtask\n  \
         function automatic int f(input int k); T loc; loc = 'x; \
         $display(\"f x=%b\", loc); return 0; endfunction\n  \
         T mv; initial begin mv = 'x; $display(\"m x=%b\", mv); t(); void'(f(0)); end\n\
         endmodule\n\
         module top; m #(.T(logic [7:0])) u (); initial #10 $finish; endmodule\n");
    assert_eq!(rc, Some(0), "{out}");
    for l in ["m x=xxxxxxxx", "t x=xxxxxxxx", "f x=xxxxxxxx"] {
        assert!(out.contains(l), "missing `{l}`\n{out}");
    }
}

#[test]
fn typename_of_an_overridden_t_reports_the_override() {
    // NO ORACLE: iverilog has no `$typename` ("not defined by any module") and
    // verilator answers `PARAMTYPEDTYPE 'T'` for both the module var and the task
    // local. Pinned for INTERNAL CONSISTENCY only — the same `intro_kind` entry that
    // drives the 2-state coercion drives `$typename`, so this is the witness that the
    // two cannot drift apart again.
    let (out, rc) = run("`timescale 1ns/1ns\n\
         module m #(parameter type T = logic [7:0]) ();\n  T mv;\n  \
         task t; T tl; begin $display(\"tn=%s\", $typename(tl)); end endtask\n  \
         initial begin $display(\"mn=%s\", $typename(mv)); t(); end\nendmodule\n\
         module top; m #(.T(bit [7:0])) u (); initial #10 $finish; endmodule\n");
    assert_eq!(rc, Some(0), "{out}");
    assert!(out.contains("mn=bit[7:0]"), "{out}");
    assert!(out.contains("tn=bit[7:0]"), "{out}");
}
