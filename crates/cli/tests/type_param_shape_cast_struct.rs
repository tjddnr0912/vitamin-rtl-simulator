//! §3 ⑤ⓕ SIGN axis of the two UNCARRIED positions that can carry it — a `T'(e)`
//! cast and a whole-member read of a packed-struct member declared `T`, where `T` is
//! an OVERRIDABLE `parameter type` (ROADMAP §5.2 row 3).
//!
//! §4.5.479 gave the DECLARATION containers (`NetVarDecl`/`AnsiPort`/`PortDecl`/
//! `TfPort`) a `shape_param` slot, so an override's signedness and 2-state kind reach
//! every declaration of `T`. Five positions had no such slot and kept `T`'s STRICT
//! guard — the design stayed LOUD (F4004) rather than silently binding the default's
//! shape. Two of the five do not need a declaration slot at all: the SIGN they apply
//! is a NODE in the emitted expression (`CastTarget::Signing { signed: <parse-time
//! bool> }`, from `hdl-parser/src/casts.rs` and `struct_sel.rs`), and a node can name
//! `T$s` instead of baking a bool. `CastTarget::SigningParam { shape_param }` is that
//! node; elaborate folds bit 0 of `T$s` in the instance's own parameter scope
//! (`Elaborator::cast_shape_signed`), exactly as `T$w` already carried the width.
//!
//! The 2-STATE axis of these two positions is NOT carried and deliberately stays
//! strict: a cast node has no kind field, and a packed struct's 4-state-ness is the
//! whole variable's, decided at parse from the member kinds. So the guard became
//! PER-AXIS — a `T` whose only uncarried uses are these two compares `T$s >> 1`
//! (the 2-state bit and the arity bits), and a `T` used as an enum base / class
//! property / function RETURN type keeps the full strict compare. Both wordings and
//! the boundary between them are pinned below.
//!
//! Oracles: iverilog 13.0 `-g2012` and verilator 5.052 `--binary --timing` agree on
//! every value pinned here (verilator fails to LINK the 40-bit struct design on this
//! machine — an environment failure, not a verdict — so that one cell is iverilog's
//! text alone and is marked).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_tpcaststruct_{}_{n}", std::process::id()));
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

/// Container (d): `d = T'(8'hF0)` into a signed 32-bit, `r` into an unsigned one.
/// `$bits` is the cast width, `neg` the sign observable.
fn cast_src(default_ty: &str, inst: &str) -> String {
    format!(
        "`timescale 1ns/1ns\nmodule m #(parameter type T = {default_ty}) ();\n  \
         logic signed [31:0] d;\n  logic [31:0] r;\n  initial begin\n    \
         d = T'(8'hF0);\n    r = T'(8'hF0);\n    \
         $display(\"bits=%0d raw=%0d d=%0d neg=%0d\", $bits(T'(8'hF0)), r, d, \
         (T'(8'hF0) - 1) < 0);\n  end\n  initial #10 $finish;\nendmodule\n\
         module top; m {inst} u(); endmodule\n"
    )
}

/// Container (a): `typedef struct packed { T f; } s_t;` — the whole-member READ.
fn struct_src(default_ty: &str, inst: &str) -> String {
    format!(
        "`timescale 1ns/1ns\nmodule m #(parameter type T = {default_ty}) ();\n  \
         typedef struct packed {{ T f; }} s_t;\n  s_t s;\n  logic signed [31:0] d;\n  \
         initial begin\n    s.f = 8'hF0;\n    d = s.f;\n    \
         $display(\"bits=%0d raw=%0d d=%0d neg=%0d\", $bits(s.f), s.f, d, \
         (s.f - 1) < 0);\n  end\n  initial #10 $finish;\nendmodule\n\
         module top; m {inst} u(); endmodule\n"
    )
}

/// The wide bands: `'1` into the position, read as hex + sign.
fn band_cast_src(w: u32, inst: &str) -> String {
    format!(
        "`timescale 1ns/1ns\nmodule m #(parameter type T = logic [{h}:0]) ();\n  \
         initial $display(\"bits=%0d neg=%0d hx=%0h\", $bits(T'(8'hF0)), \
         (T'('1) - 1) < 0, T'(8'hF0));\n  initial #10 $finish;\nendmodule\n\
         module top; m {inst} u(); endmodule\n",
        h = w - 1
    )
}

fn band_struct_src(w: u32, inst: &str) -> String {
    format!(
        "`timescale 1ns/1ns\nmodule m #(parameter type T = logic [{h}:0]) ();\n  \
         typedef struct packed {{ T f; }} s_t;\n  s_t s;\n  initial begin\n    \
         s.f = '1;\n    $display(\"bits=%0d neg=%0d hx=%0h\", $bits(s.f), \
         (s.f - 1) < 0, s.f);\n  end\n  initial #10 $finish;\nendmodule\n\
         module top; m {inst} u(); endmodule\n",
        h = w - 1
    )
}

fn value(src: &str, want: &str) {
    let (out, rc) = run(src);
    assert_eq!(rc, Some(0), "expected {want}\n{out}");
    assert!(out.contains(want), "expected {want}\n{out}");
}

/// A cell that must stay LOUD. The CODE is pinned, never the wording.
fn loud(src: &str) {
    let (out, rc) = run(src);
    assert_eq!(rc, Some(1), "{out}");
    assert!(out.contains("[VITA-F4004]"), "{out}");
}

// ── loud → value: the SIGN axis now follows the override ───────────────────────
// Every expected string below is BOTH oracles' output text, byte-identical.

#[test]
fn cast_sign_override_follows() {
    // d-sign-P. iverilog/verilator: `bits=8 raw=4294967280 d=-16 neg=1`.
    // PRE (155a738): F4004 strict.
    value(
        &cast_src("logic [7:0]", "#(.T(logic signed [7:0]))"),
        "bits=8 raw=4294967280 d=-16 neg=1",
    );
}

#[test]
fn struct_member_sign_override_follows() {
    // a-sign-P. Both oracles `bits=8 raw=-16 d=-16 neg=1` (the member read is
    // 8 bits SIGNED, so `%0d` of `s.f` itself prints −16, unlike the 32-bit `r`
    // the cast cell assigns into). PRE: F4004 strict.
    value(
        &struct_src("logic [7:0]", "#(.T(logic signed [7:0]))"),
        "bits=8 raw=-16 d=-16 neg=1",
    );
}

#[test]
fn cast_sign_override_follows_band_33_64() {
    // e-d-w40-P. Both oracles `bits=40 neg=1 hx=f0`. PRE: F4004.
    value(
        &band_cast_src(8, "#(.T(logic signed [39:0]))"),
        "bits=40 neg=1 hx=f0",
    );
}

#[test]
fn cast_sign_override_follows_band_over_64() {
    // e-d-w80-P. Both oracles `bits=80 neg=1 hx=f0`. PRE: F4004.
    value(
        &band_cast_src(8, "#(.T(logic signed [79:0]))"),
        "bits=80 neg=1 hx=f0",
    );
}

#[test]
fn struct_member_sign_override_follows_band_33_64() {
    // e-a-w40-P. iverilog `bits=40 neg=1 hx=ffffffffff`; verilator FAILED TO LINK
    // this design on the measuring machine (`no such file or directory:
    // 'V…__ALL.a'`) — an environment failure, so iverilog is the sole oracle here.
    value(
        &band_struct_src(8, "#(.T(logic signed [39:0]))"),
        "bits=40 neg=1 hx=ffffffffff",
    );
}

#[test]
fn struct_member_sign_override_follows_band_over_64() {
    // e-a-w80-P. Both oracles `bits=80 neg=1 hx=ffffffffffffffffffff`. PRE: F4004.
    value(
        &band_struct_src(8, "#(.T(logic signed [79:0]))"),
        "bits=80 neg=1 hx=ffffffffffffffffffff",
    );
}

#[test]
fn cast_unsigned_override_of_a_signed_default_follows() {
    // The OTHER direction — the default is `logic signed [7:0]` and the override
    // drops the sign. Both oracles `bits=8 raw=240 d=240 neg=0`; PRE: F4004. This is
    // the cell that proves the node reads `T$s` rather than defaulting to unsigned:
    // a `shape_signed(false, …)` fallback would also print this, but the
    // `signed_default_*_no_override` pair below would then print it too, and they
    // do not.
    value(
        &cast_src("logic signed [7:0]", "#(.T(logic [7:0]))"),
        "bits=8 raw=240 d=240 neg=0",
    );
}

#[test]
fn cast_sign_override_follows_in_a_constant_context() {
    // The cast in a `localparam` initializer, i.e. the const-fold domain
    // (`const_eval_cast` / `const_expr_signed` / `param_decl_wsign`), not the
    // runtime lowering. Both oracles `P=-16 Q=-17`; PRE: F4004.
    value(
        "`timescale 1ns/1ns\nmodule m #(parameter type T = logic [7:0]) ();\n  \
         localparam int P = T'(8'hF0);\n  localparam int Q = T'(8'hF0) - 1;\n  \
         initial $display(\"P=%0d Q=%0d\", P, Q);\n  initial #10 $finish;\nendmodule\n\
         module top; m #(.T(logic signed [7:0])) u(); endmodule\n",
        "P=-16 Q=-17",
    );
}

// ── must stay LOUD: the 2-STATE axis has no carrier in either position ─────────
// Measured, and a REFUTATION of the grounding report's §4 count of ten loud→value
// cells: the `both` and `signwidth` cells of the census spell their override
// `bit signed [7:0]` / `bit signed [15:0]`, so each of them changes the 2-STATE
// axis too. The sign half of those overrides IS followed now; the design stays
// loud because the 2-state half is not, and the message says so.

#[test]
fn cast_two_state_override_stays_loud() {
    // d-2st-P: `bit [7:0]` over a `logic [7:0]` default.
    loud(&cast_src("logic [7:0]", "#(.T(bit [7:0]))"));
}

#[test]
fn struct_member_two_state_override_stays_loud() {
    // a-2st-P.
    loud(&struct_src("logic [7:0]", "#(.T(bit [7:0]))"));
}

#[test]
fn cast_sign_and_two_state_override_stays_loud() {
    // d-both-P (`bit signed [7:0]`) and d-signwidth-P (`bit signed [15:0]`).
    loud(&cast_src("logic [7:0]", "#(.T(bit signed [7:0]))"));
    loud(&cast_src("logic [7:0]", "#(.T(bit signed [15:0]))"));
}

#[test]
fn struct_member_sign_and_two_state_override_stays_loud() {
    // a-both-P and a-signwidth-P.
    loud(&struct_src("logic [7:0]", "#(.T(bit signed [7:0]))"));
    loud(&struct_src("logic [7:0]", "#(.T(bit signed [15:0]))"));
}

#[test]
fn two_state_uninitialised_read_stays_loud() {
    // f-d-2st-P / f-a-2st-P — the readout that PROVES the 2-state axis is not
    // cosmetic: iverilog prints `u=00000000` for the explicit 2-state twin and
    // `u=xxxxxxxx` for the 4-state one. Nothing in a cast node or a part-select can
    // carry that, so both stay loud.
    loud(
        "`timescale 1ns/1ns\nmodule m #(parameter type T = logic [7:0]) ();\n  \
         logic [7:0] q;\n  initial begin q = T'(q); $display(\"u=%b\", q); end\n  \
         initial #10 $finish;\nendmodule\n\
         module top; m #(.T(bit [7:0])) u(); endmodule\n",
    );
    loud(
        "`timescale 1ns/1ns\nmodule m #(parameter type T = logic [7:0]) ();\n  \
         typedef struct packed { T f; } s_t;\n  s_t s;\n  \
         initial $display(\"u=%b\", s.f);\n  initial #10 $finish;\nendmodule\n\
         module top; m #(.T(bit [7:0])) u(); endmodule\n",
    );
}

#[test]
fn enum_base_sign_override_stays_fully_strict() {
    // b-sign-P — an enum base has NO node to hang a sign on (the labels are bound
    // at elaborate from `TypedefKind::Enum.signed`, a frozen parse-time scalar), so
    // it keeps the ORIGINAL strict compare, including the sign bit. Out of scope for
    // this slice by the brief; pinned so the per-axis guard cannot quietly relax it.
    loud(
        "`timescale 1ns/1ns\nmodule m #(parameter type T = logic [7:0]) ();\n  \
         typedef enum T { EA = 8'hF0, EB = 8'h01 } e_t;\n  e_t e;\n  \
         initial begin e = EA; $display(\"raw=%0d\", e); end\n  \
         initial #10 $finish;\nendmodule\n\
         module top; m #(.T(logic signed [7:0])) u(); endmodule\n",
    );
}

// ── controls: byte-identical PRE and POST ─────────────────────────────────────

#[test]
fn no_override_controls_unchanged() {
    // d-none-P / a-none-P: `bits=8 raw=240 d=240 neg=0` in both oracles.
    value(&cast_src("logic [7:0]", ""), "bits=8 raw=240 d=240 neg=0");
    value(&struct_src("logic [7:0]", ""), "bits=8 raw=240 d=240 neg=0");
}

#[test]
fn width_only_override_controls_unchanged() {
    // d-width-P / a-width-P — the width always followed; `bits=16 raw=240 d=240
    // neg=0` in both oracles, before and after.
    value(
        &cast_src("logic [7:0]", "#(.T(logic [15:0]))"),
        "bits=16 raw=240 d=240 neg=0",
    );
    value(
        &struct_src("logic [7:0]", "#(.T(logic [15:0]))"),
        "bits=16 raw=240 d=240 neg=0",
    );
}

#[test]
fn signed_default_with_no_override_keeps_the_defaults_sign() {
    // The fallback hazard, measured: the node carries only a NAME, so a reader that
    // failed to resolve `T$s` would answer UNSIGNED and silently destroy a signed
    // DEFAULT. Both oracles `…raw=4294967280 d=-16 neg=1` (cast) and `raw=-16`
    // (member); PRE prints the same, so these are correct→correct.
    value(
        &cast_src("logic signed [7:0]", ""),
        "bits=8 raw=4294967280 d=-16 neg=1",
    );
    value(
        &struct_src("logic signed [7:0]", ""),
        "bits=8 raw=-16 d=-16 neg=1",
    );
}

#[test]
fn plain_user_signing_cast_unchanged() {
    // `signed'(e)` / `unsigned'(e)` keep emitting `CastTarget::Signing`; the new
    // variant is additive and no existing producer moved. Both oracles
    // `d=-16 r=240 neg=1`.
    value(
        "`timescale 1ns/1ns\nmodule top;\n  logic [7:0] v;\n  logic signed [31:0] d;\n  \
         logic [31:0] r;\n  initial begin\n    v = 8'hF0;\n    d = signed'(v);\n    \
         r = unsigned'(v);\n    $display(\"d=%0d r=%0d neg=%0d\", d, r, \
         (signed'(v) - 1) < 0);\n  end\n  initial #10 $finish;\nendmodule\n",
        "d=-16 r=240 neg=1",
    );
}

#[test]
fn numeric_signed_struct_member_unchanged() {
    // The NUMERIC (fully folded) struct path — `struct_sel.rs`'s second
    // `CastTarget::Signing` producer, which this slice does not touch. Both oracles
    // `bits=8 raw=-16 d=-16 neg=1`.
    value(
        "`timescale 1ns/1ns\nmodule top;\n  typedef struct packed \
         { logic signed [7:0] f; logic [7:0] g; } s_t;\n  s_t s;\n  \
         logic signed [31:0] d;\n  initial begin\n    s.f = 8'hF0;\n    d = s.f;\n    \
         $display(\"bits=%0d raw=%0d d=%0d neg=%0d\", $bits(s.f), s.f, d, \
         (s.f - 1) < 0);\n  end\n  initial #10 $finish;\nendmodule\n",
        "bits=8 raw=-16 d=-16 neg=1",
    );
}

#[test]
fn packed_union_member_of_a_type_parameter_stays_loud() {
    // The UNION spelling the grounding left unmeasured. A union's layout is the
    // NUMERIC overlay table (`typedefs.rs`), which needs every member's width to
    // fold at parse — `[T$w-1:0]` never does — so the design is refused at PARSE
    // with E2002, before the shape guard is reached. PRE-existing and unchanged by
    // this slice (`parse_struct_member_type(false)` for the union caller keeps both
    // axes blocked there). Both oracles RUN it (`bits=8 raw=-16 d=-16 neg=1`), so
    // this is an honest-loud gap, filed, not closed here.
    let (out, rc) = run(
        "`timescale 1ns/1ns\nmodule m #(parameter type T = logic [7:0]) ();\n  \
         typedef union packed { T f; logic [7:0] g; } u_t;\n  u_t s;\n  \
         initial $display(\"raw=%0d\", s.f);\n  initial #10 $finish;\nendmodule\n\
         module top; m #(.T(logic signed [7:0])) u(); endmodule\n",
    );
    assert_eq!(rc, Some(1), "{out}");
    assert!(out.contains("[VITA-E2002]"), "{out}");
    assert!(
        out.contains("union member width must be a named integer type"),
        "{out}"
    );
}

#[test]
fn explicit_twin_of_a_packed_union_member_unchanged() {
    // The union control: written explicitly it runs, and agrees with both oracles.
    value(
        "`timescale 1ns/1ns\nmodule top;\n  typedef union packed \
         { logic signed [7:0] f; logic [7:0] g; } u_t;\n  u_t s;\n  \
         logic signed [31:0] d;\n  initial begin\n    s.f = 8'hF0;\n    d = s.f;\n    \
         $display(\"bits=%0d raw=%0d d=%0d neg=%0d\", $bits(s.f), s.f, d, \
         (s.f - 1) < 0);\n  end\n  initial #10 $finish;\nendmodule\n",
        "bits=8 raw=-16 d=-16 neg=1",
    );
}
