//! `typedef enum <base> signed [N] {...}` — an explicit `signed` on the built-in
//! enum base sets the WHOLE-enum value signedness (§6.19). Before the fix the
//! parser discarded the keyword (`let _ = self.opt_signed();`) and the enum
//! typedef's `TypeInfo.signed` was hardcoded `false`, so a signed-base enum read
//! as one value was silently treated as UNSIGNED:
//! `enum logic signed [3:0] {B=-1} et; et e=B; $display("%0d",e)` printed `15`
//! instead of `-1` (also wrong for compare / arithmetic / sign-extend-on-assign).
//! Fixed by capturing the qualifier into `TypeInfo.signed` — the SAME funnel the
//! struct/union §4.5.152 fix uses. An UNSIGNED base, the default-`int` base (no
//! explicit base), and the raw bit pattern are unaffected. All expected values
//! pinned to iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_esb_{}_{n}", std::process::id()));
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

/// Whole-value read of a signed-base enum is SIGNED across display, compare, and
/// arithmetic (iverilog-pinned). `B = -1` reads `-1`, not `15`.
#[test]
fn signed_enum_base_whole_value_is_signed() {
    let (o, ok) = run("module t;\n\
         typedef enum logic signed [3:0] { A=-2, B=-1, C=1 } et;\n\
         et e;\n\
         initial begin\n\
           e = B;\n\
           $display(\"disp=%0d cmp=%b add=%0d bin=%b\", e, e < 0, e + 1, e);\n\
           #1 $finish;\n\
         end endmodule");
    // -1 ; -1 < 0 == 1 ; -1 + 1 == 0 ; bit pattern 1111
    assert!(
        ok && o.contains("disp=-1 cmp=1 add=0 bin=1111"),
        "got:\n{o}"
    );
}

/// A signed-base enum sign-EXTENDS when assigned to a wider target (the RHS
/// signedness drives extension), to BOTH a signed and an unsigned wide target.
#[test]
fn signed_enum_base_sign_extends_on_assign() {
    let (o, ok) = run("module t;\n\
         typedef enum logic signed [3:0] { B=-1 } et;\n\
         et e;\n\
         logic signed [7:0] ws; logic [7:0] wu;\n\
         initial begin\n\
           e = B;\n\
           ws = e; wu = e;\n\
           $display(\"ws=%0d wsb=%b wu=%0d wub=%b\", ws, ws, wu, wu);\n\
           #1 $finish;\n\
         end endmodule");
    // signed target -> -1 ; unsigned target -> 255 ; both bit-extend to 11111111
    assert!(
        ok && o.contains("ws=-1 wsb=11111111 wu=255 wub=11111111"),
        "got:\n{o}"
    );
}

/// BYTE-IDENTITY: an UNSIGNED-base enum stays UNSIGNED — the fix must not change
/// any non-`signed` enum. `D = 1` reads `1`; a whole-value compare is unsigned.
#[test]
fn unsigned_enum_base_unchanged() {
    let (o, ok) = run("module t;\n\
         typedef enum logic [3:0] { D=1, E=2, F=15 } et;\n\
         et e;\n\
         initial begin\n\
           e = F;\n\
           $display(\"v=%0d cmp=%b\", e, e < 0);\n\
           #1 $finish;\n\
         end endmodule");
    // F == 15 (unsigned) ; 15 < 0 == 0
    assert!(ok && o.contains("v=15 cmp=0"), "got:\n{o}");
}

/// The DEFAULT enum base (no explicit base type) is `int` = 32-bit SIGNED, and was
/// already correct — guard it against regression from the signed-base change.
#[test]
fn default_int_enum_base_stays_signed() {
    let (o, ok) = run("module t;\n\
         typedef enum { G=-2, H=-1, I=1 } et;\n\
         et e;\n\
         initial begin\n\
           e = H;\n\
           $display(\"v=%0d cmp=%b\", e, e < 0);\n\
           #1 $finish;\n\
         end endmodule");
    assert!(ok && o.contains("v=-1 cmp=1"), "got:\n{o}");
}

/// SIBLING (§4.5.153, same funnel): an ATOM base (`int`) with an explicit
/// `unsigned` is UNSIGNED as a whole. `enum int unsigned {A=32'hFFFFFFFF}` reads
/// `4294967295`, not `-1`. Before the fix the `None` (atom/base-less) arm hardcoded
/// `signed:true` and dropped the qualifier. iverilog-pinned (the clean `int`
/// case; iverilog is self-consistent here — unsigned display AND unsigned compare).
#[test]
fn atom_int_unsigned_base_is_unsigned() {
    let (o, ok) = run("module t;\n\
         typedef enum int unsigned { A = 32'hFFFFFFFF } et;\n\
         et e;\n\
         initial begin\n\
           e = A;\n\
           $display(\"v=%0d cmp=%b\", e, e < 0);\n\
           #1 $finish;\n\
         end endmodule");
    assert!(ok && o.contains("v=4294967295 cmp=0"), "got:\n{o}");
}

/// BYTE-IDENTITY guard for the atom/base-less default: an explicit `signed` int
/// base, and a base-less `enum {…}` (implicit int), both stay SIGNED — the atom-arm
/// change must not flip the qualifier-less default away from int-signed.
#[test]
fn atom_int_signed_and_baseless_stay_signed() {
    let (o, ok) = run("module t;\n\
         typedef enum int signed { A = -1 } es_t;\n\
         typedef enum          { B = -1 } ed_t;\n\
         es_t es; ed_t ed;\n\
         initial begin\n\
           es = A; ed = B;\n\
           $display(\"sgn=%0d %b base=%0d %b\", es, es < 0, ed, ed < 0);\n\
           #1 $finish;\n\
         end endmodule");
    // both -1 and signed-compare true
    assert!(ok && o.contains("sgn=-1 1 base=-1 1"), "got:\n{o}");
}

/// A signed-base enum used as a packed-STRUCT MEMBER keeps its member signedness
/// (member sub-select reads `-1`), while the struct's whole bit pattern is intact.
#[test]
fn signed_enum_as_struct_member() {
    let (o, ok) = run("module t;\n\
         typedef enum logic signed [3:0] { N1 = -1 } et;\n\
         typedef struct packed { et f; logic [3:0] g; } sp_t;\n\
         sp_t s;\n\
         initial begin\n\
           s.f = N1; s.g = 4'h2;\n\
           $display(\"f=%0d fcmp=%b whole=%h\", s.f, s.f < 0, s);\n\
           #1 $finish;\n\
         end endmodule");
    assert!(ok && o.contains("f=-1 fcmp=1 whole=f2"), "got:\n{o}");
}

/// vita-INTERNAL equivalence teeth: a signed-base enum wrapping `logic [7:0]` must
/// behave BYTE-IDENTICALLY to a plain `logic signed [7:0]` for every whole-value
/// operation. Both printed blocks must match, char-for-char.
#[test]
fn signed_enum_equiv_plain_signed_vector() {
    let mk = |decl: &str, set_ff: &str, set_80: &str| {
        format!(
            "module t;\n{decl}\n\
             initial begin\n\
               {set_ff}\n\
               $display(\"%0d %b %0d %b %b\", x, x < 0, x + 1, x >>> 1, x <= -1);\n\
               {set_80}\n\
               $display(\"%0d\", x);\n\
               #1 $finish;\n\
             end endmodule",
        )
    };
    // enum: reach 0xFF / 0x80 via signed labels NEG1 / NEG128.
    let (a, oka) = run(&mk(
        "typedef enum logic signed [7:0] { NEG1=-1, NEG128=-128 } et; et x;",
        "x = NEG1;",
        "x = NEG128;",
    ));
    let (b, okb) = run(&mk("logic signed [7:0] x;", "x = 8'hFF;", "x = 8'h80;"));
    assert!(oka && okb, "run failed:\nenum:\n{a}\nplain:\n{b}");
    assert_eq!(a, b, "signed-base enum diverged from plain signed vector");
}
