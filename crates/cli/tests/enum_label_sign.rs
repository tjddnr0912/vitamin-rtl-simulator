//! An enum LABEL reference carries the enum's DECLARED signedness in a relational /
//! collective context (§4.5.158). Before the fix a label's sign was inferred PER
//! VALUE (`v < 0`), so a POSITIVE label of a SIGNED enum was treated UNSIGNED — and
//! IEEE §11.8.1 collective signedness then made the whole comparison unsigned:
//! `enum byte {A=-1,B=2} v=A; v > B` printed 1 (255 > 2) instead of 0 (-1 > 2), and
//! `B > C` (C=-3) printed 0 instead of 1. Fixed by threading the enum base's declared
//! sign into the AST `TypedefKind::Enum.signed`, so each label's `param_meta` sign is
//! `enum_signed || v < 0` (a negative label stays signed even on an illegal unsigned
//! base — graceful). Base-less `enum {…}` = int (32-bit signed) — its labels also get
//! the sign. Unsigned enums and label VALUE/display are unaffected. iverilog-pinned.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_els_{}_{n}", std::process::id()));
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

/// A POSITIVE label of a signed enum is SIGNED in a relational comparison, so a
/// mixed-sign comparison stays signed (iverilog-pinned).
#[test]
fn signed_enum_label_relational_is_signed() {
    let (o, ok) = run("module t;\n\
         typedef enum byte { A=-1, B=2, C=-3 } se_t;\n\
         se_t v;\n\
         initial begin\n\
           v = A;\n\
           $display(\"vB=%0d BC=%0d vlt=%0d\", v > B, B > C, v < B);\n\
           #1 $finish;\n\
         end endmodule");
    // -1 > 2 == 0 ; 2 > -3 == 1 ; -1 < 2 == 1
    assert!(ok && o.contains("vB=0 BC=1 vlt=1"), "got:\n{o}");
}

/// A base-less `enum {…}` is `int` (32-bit SIGNED); its labels compare signed too.
#[test]
fn baseless_int_enum_label_is_signed() {
    let (o, ok) = run("module t;\n\
         typedef enum { I0=-2, I1=3 } ie_t;\n\
         initial begin\n\
           $display(\"lt=%0d gt=%0d\", I0 < I1, I1 > I0);\n\
           #1 $finish;\n\
         end endmodule");
    // -2 < 3 == 1 ; 3 > -2 == 1
    assert!(ok && o.contains("lt=1 gt=1"), "got:\n{o}");
}

/// BYTE-IDENTITY: an UNSIGNED-base enum keeps UNSIGNED labels — a comparison of two
/// non-negative labels is unsigned (unchanged by the fix).
#[test]
fn unsigned_enum_label_stays_unsigned() {
    let (o, ok) = run("module t;\n\
         typedef enum logic [3:0] { U0=1, U1=5, U2=15 } ue_t;\n\
         initial begin\n\
           $display(\"gt=%0d sub=%0d\", U2 > U0, U0 - U1);\n\
           #1 $finish;\n\
         end endmodule");
    // 15 > 1 == 1 ; 1 - 5 unsigned 4-bit == 12
    assert!(ok && o.contains("gt=1 sub=12"), "got:\n{o}");
}

/// The fix touches only the relational/collective SIGN — label VALUE, display, and
/// arithmetic are unchanged (a negative label still reads and adds correctly).
#[test]
fn label_value_and_arith_unchanged() {
    let (o, ok) = run("module t;\n\
         typedef enum byte { A=-1, B=2, C=-3 } se_t;\n\
         initial begin\n\
           $display(\"disp=%0d %0d add=%0d neg=%0d\", A, C, A + B, C < 0);\n\
           #1 $finish;\n\
         end endmodule");
    // -1, -3 ; -1+2 == 1 ; -3 < 0 == 1
    assert!(ok && o.contains("disp=-1 -3 add=1 neg=1"), "got:\n{o}");
}

/// A body-local enum (declared inside a function/task body) also carries the enum's
/// sign + width via a SCOPED `param_meta` (§4.5.158, saved/restored so it doesn't
/// leak) — a positive label of a signed enum compares signed inside the function, and
/// `$bits` is the base width, not 32. (iverilog 13.0 cannot bind a body-local enum
/// label, so the value is hand-IEEE-pinned to the module-scope-equivalent result.)
#[test]
fn body_local_enum_label_is_signed() {
    let (o, ok) = run("module t;\n\
         function logic f(logic signed [7:0] v);\n\
           typedef enum byte { B=2 } e_t;\n\
           f = (v < B);\n\
         endfunction\n\
         function int g;\n\
           typedef enum byte { W=2 } w_t;\n\
           g = $bits(W);\n\
         endfunction\n\
         initial begin\n\
           $display(\"r=%b bits=%0d\", f(-1), g());\n\
           #1 $finish;\n\
         end endmodule");
    // -1 < 2 signed == 1 ; $bits(byte label) == 8
    assert!(ok && o.contains("r=1 bits=8"), "got:\n{o}");
}

/// vita-INTERNAL equivalence teeth: a signed-enum label in a comparison behaves
/// BYTE-IDENTICALLY to a plain `logic signed [7:0]` constant of the same value.
#[test]
fn signed_enum_label_equiv_plain_signed_const() {
    let mk = |decl: &str, b: &str| {
        format!(
            "module t;\n{decl}\n\
             logic signed [7:0] v;\n\
             initial begin\n\
               v = -1;\n\
               $display(\"%0d %0d %0d\", v > {b}, {b} < 0, {b} > -4);\n\
               #1 $finish;\n\
             end endmodule",
        )
    };
    let (a, oka) = run(&mk("typedef enum byte { B=2 } e_t;", "B"));
    let (b, okb) = run(&mk("localparam logic signed [7:0] B = 2;", "B"));
    assert!(oka && okb, "run failed:\nenum:\n{a}\nplain:\n{b}");
    assert_eq!(a, b, "signed enum label diverged from plain signed const");
}
