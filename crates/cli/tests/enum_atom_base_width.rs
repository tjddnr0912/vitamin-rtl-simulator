//! `typedef enum <built-in-base> {...}` — the enum's WIDTH, sign, and 2-state-ness
//! all follow its base kind. Before the fix the parser collapsed every rangeless
//! built-in base onto the 4-state 32-bit `Integer` `None` arm, so `$bits`/`%b`/
//! concat width were wrong (`enum byte {A=5}` = 32 not 8, `enum logic {A,B}` = 32
//! not 1, `enum time` = 32 not 64) for BOTH the variable and its labels, AND `int`
//! / the base-less `enum {…}` were 4-state (uninit X) instead of 2-state (0). Fixed
//! (§4.5.154) by PRESERVING the real base kind in the enum's `TypeInfo` (so it is
//! sized + state-typed exactly like a plain `byte`/`int`/`logic`/… decl) and
//! synthesizing the kind's width as a range only for the AST label-width path.
//! `integer` stays 4-state (Verilog legacy). All values pinned to iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_eabw_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        out.status.success(),
    )
}

/// Atom bases size to their fixed width: byte=8, shortint=16, longint=64. `%b`
/// renders at the real width (iverilog-pinned).
#[test]
fn atom_base_widths() {
    let (o, ok) = run("module t;\n\
         typedef enum byte     { A = 5 } b_t;\n\
         typedef enum shortint { B = 5 } s_t;\n\
         typedef enum longint  { C = 5 } l_t;\n\
         b_t b=A; s_t s=B; l_t l=C;\n\
         initial begin\n\
           $display(\"bits=%0d %0d %0d byteb=%b\", $bits(b), $bits(s), $bits(l), b);\n\
           #1 $finish;\n\
         end endmodule");
    assert!(ok && o.contains("bits=8 16 64 byteb=00000101"), "got:\n{o}");
}

/// A bare vector kind with NO range (`enum logic {...}`/`enum bit {...}`) is 1-bit
/// (not the old 32). Two labels fit; iverilog agrees.
#[test]
fn bare_vector_base_width_is_1() {
    let (o, ok) = run("module t;\n\
         typedef enum logic { A, B } v_t;\n\
         typedef enum bit   { C, D } t_t;\n\
         v_t v=B; t_t t=D;\n\
         initial begin\n\
           $display(\"lbits=%0d tbits=%0d v=%b t=%b\", $bits(v), $bits(t), v, t);\n\
           #1 $finish;\n\
         end endmodule");
    assert!(ok && o.contains("lbits=1 tbits=1 v=1 t=1"), "got:\n{o}");
}

/// BYTE-IDENTITY: `int`/`integer` bases and the base-less `enum {...}` stay 32-bit
/// (int) — the fix must not disturb the already-correct kinds.
#[test]
fn int_integer_baseless_stay_32() {
    let (o, ok) = run("module t;\n\
         typedef enum int     { A = 5 } i_t;\n\
         typedef enum integer { B = 5 } g_t;\n\
         typedef enum         { C = 5 } d_t;\n\
         i_t i=A; g_t g=B; d_t d=C;\n\
         initial begin\n\
           $display(\"bits=%0d %0d %0d\", $bits(i), $bits(g), $bits(d));\n\
           #1 $finish;\n\
         end endmodule");
    assert!(ok && o.contains("bits=32 32 32"), "got:\n{o}");
}

/// The atom width propagates into a concatenation: `{byte_enum, 4'hF}` is 8+4=12
/// bits, not 32+4.
#[test]
fn atom_width_in_concat() {
    let (o, ok) = run("module t;\n\
         typedef enum byte { A = 5 } b_t;\n\
         b_t b = A;\n\
         initial begin\n\
           $display(\"concat=%b lbl=%b\", {b, 4'hF}, {A, 4'hF});\n\
           #1 $finish;\n\
         end endmodule");
    // both 12 bits: 00000101 1111
    assert!(
        ok && o.contains("concat=000001011111 lbl=000001011111"),
        "got:\n{o}"
    );
}

/// Width and sign are BOTH correct: a signed `byte` base reads -1 at 8 bits; an
/// explicit `unsigned` byte base reads 200 at 8 bits (width fix composes with the
/// §4.5.153 signedness fix).
#[test]
fn atom_base_width_and_sign() {
    let (o, ok) = run("module t;\n\
         typedef enum byte          { A = -1 }  sb_t;\n\
         typedef enum byte unsigned { B = 200 } ub_t;\n\
         sb_t sb=A; ub_t ub=B;\n\
         initial begin\n\
           $display(\"s=%0d %b %0d u=%0d %b %0d\", sb, sb<0, $bits(sb), ub, ub<0, $bits(ub));\n\
           #1 $finish;\n\
         end endmodule");
    // signed byte: -1, cmp 1, 8 bits ; unsigned byte: 200, cmp 0, 8 bits
    assert!(ok && o.contains("s=-1 1 8 u=200 0 8"), "got:\n{o}");
}

/// `enum int` and the base-less `enum {…}` (whose implicit base IS `int`) are
/// 2-STATE — an uninitialized value reads 0, not X. Before the fix they used the
/// 4-state `Integer` kind, so a state-machine `enum {IDLE,RUN}` read X before its
/// first assignment (diverging from iverilog's 0). iverilog-pinned.
#[test]
fn int_and_baseless_enum_are_2state() {
    let (o, ok) = run("module t;\n\
         typedef enum int { A = 5 } i_t;\n\
         typedef enum     { IDLE, RUN } s_t;\n\
         i_t i; s_t s;\n\
         initial begin\n\
           $display(\"i_unk=%b s_unk=%b s_val=%0d ibits=%0d\", $isunknown(i), $isunknown(s), s, $bits(i));\n\
           #1 $finish;\n\
         end endmodule");
    // both 2-state: uninitialized reads 0 / not-unknown ; int is 32-bit
    assert!(
        ok && o.contains("i_unk=0 s_unk=0 s_val=0 ibits=32"),
        "got:\n{o}"
    );
}

/// BYTE-IDENTITY guard: `enum integer` stays 4-STATE (X-init) — `integer` is the
/// Verilog legacy 4-state type, distinct from 2-state `int`. The int-family 2-state
/// change must not flip `integer`.
#[test]
fn enum_integer_stays_4state() {
    let (o, ok) = run("module t;\n\
         typedef enum integer { A = 5 } g_t;\n\
         g_t g;\n\
         initial begin\n\
           $display(\"unk=%b bits=%0d\", $isunknown(g), $bits(g));\n\
           #1 $finish;\n\
         end endmodule");
    assert!(ok && o.contains("unk=1 bits=32"), "got:\n{o}");
}

/// An `enum time` base is 64-bit (was silently 32). iverilog-pinned.
#[test]
fn enum_time_base_width_is_64() {
    let (o, ok) = run("module t;\n\
         typedef enum time { A = 5 } tm_t;\n\
         tm_t tm = A;\n\
         initial begin\n\
           $display(\"bits=%0d\", $bits(tm));\n\
           #1 $finish;\n\
         end endmodule");
    assert!(ok && o.contains("bits=64"), "got:\n{o}");
}

/// vita-INTERNAL equivalence teeth: an `enum byte` variable must behave
/// BYTE-IDENTICALLY to a plain `byte` variable for width and every whole-value op.
#[test]
fn enum_byte_equiv_plain_byte() {
    let mk = |decl: &str, set: &str| {
        format!(
            "module t;\n{decl}\n\
             initial begin\n\
               {set}\n\
               $display(\"%0d %b %0d %b %0d\", x, x < 0, x + 1, x, $bits(x));\n\
               #1 $finish;\n\
             end endmodule",
        )
    };
    let (a, oka) = run(&mk(
        "typedef enum byte { NEG1 = -1 } et; et x;",
        "x = NEG1;",
    ));
    let (b, okb) = run(&mk("byte x;", "x = -1;"));
    assert!(oka && okb, "run failed:\nenum:\n{a}\nplain:\n{b}");
    assert_eq!(a, b, "enum byte diverged from plain byte");
}
