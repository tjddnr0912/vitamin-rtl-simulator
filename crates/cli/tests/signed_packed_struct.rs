//! `typedef struct packed signed {...}` / `union packed signed` — the `signed`
//! qualifier sets the WHOLE-struct value signedness (§7.2.1 / §7.3.1). Before the
//! fix the parser discarded the keyword (`let _ = self.opt_signed();`) and the
//! struct/union typedef's `TypeInfo.signed` was hardcoded `false`, so a signed
//! packed struct read as one value was silently treated as UNSIGNED:
//! `struct packed signed {logic[7:0] v} s; s=8'hFF; $display("%0d",s)` printed
//! `255` instead of `-1` (also wrong for compare / arithmetic / `>>>` /
//! sign-extend-on-assign). Fixed by capturing the qualifier into `TypeInfo.signed`.
//! Member access (`s.field`) and the packed LAYOUT are unaffected (members keep
//! their own signedness). All expected values pinned to iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_sps_{}_{n}", std::process::id()));
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

/// Whole-value read of a signed packed struct is SIGNED across display, compare,
/// arithmetic, and arithmetic-shift (iverilog-pinned).
#[test]
fn signed_struct_whole_value_is_signed() {
    let (o, ok) = run("module t;\n\
         typedef struct packed signed { logic [3:0] hi; logic [3:0] lo; } ss_t;\n\
         ss_t s;\n\
         initial begin\n\
           s = 8'hFF;\n\
           $display(\"disp=%0d cmp=%b add=%0d ashr=%b\", s, s < 0, s + 1, s >>> 1);\n\
           s = 8'h80;\n\
           $display(\"neg=%0d\", s);\n\
           #1 $finish;\n\
         end endmodule");
    // -1 < 0 == 1 ; -1 + 1 == 0 ; -1 >>> 1 == 11111111 ; 0x80 == -128
    assert!(
        ok && o.contains("disp=-1 cmp=1 add=0 ashr=11111111") && o.contains("neg=-128"),
        "got:\n{o}"
    );
}

/// A signed packed struct sign-EXTENDS when assigned to a wider target (the RHS
/// signedness drives extension). An unsigned struct zero-extends.
#[test]
fn signed_struct_sign_extends_on_assign() {
    let (o, ok) = run("module t;\n\
         typedef struct packed signed { logic [7:0] v; } ss_t;\n\
         typedef struct packed        { logic [7:0] v; } us_t;\n\
         ss_t s; us_t u;\n\
         logic signed [15:0] ws; logic [15:0] wu;\n\
         initial begin\n\
           s = 8'hFF; u = 8'hFF;\n\
           ws = s; wu = u;\n\
           $display(\"ws=%0d wu=%0d\", ws, wu);\n\
           #1 $finish;\n\
         end endmodule");
    // signed -> -1 ; unsigned -> 255
    assert!(ok && o.contains("ws=-1 wu=255"), "got:\n{o}");
}

/// Members keep their OWN (unsigned) signedness even when the enclosing struct is
/// signed — the whole-value signedness must not leak into `s.field`.
#[test]
fn signed_struct_members_stay_unsigned() {
    let (o, ok) = run("module t;\n\
         typedef struct packed signed { logic [3:0] hi; logic [3:0] lo; } ss_t;\n\
         ss_t s;\n\
         initial begin\n\
           s = 8'hFF;\n\
           $display(\"hi=%0d lo=%0d\", s.hi, s.lo);\n\
           #1 $finish;\n\
         end endmodule");
    // each 4-bit member reads 15 (unsigned), not -1
    assert!(ok && o.contains("hi=15 lo=15"), "got:\n{o}");
}

/// A signed packed UNION is signed as a whole; its members overlay and stay
/// member-typed (unsigned here).
#[test]
fn signed_union_whole_value_is_signed() {
    let (o, ok) = run("module t;\n\
         typedef union packed signed { logic [7:0] a; logic [7:0] b; } su_t;\n\
         su_t u;\n\
         initial begin\n\
           u = 8'hFF;\n\
           $display(\"whole=%0d mem=%0d\", u, u.a);\n\
           #1 $finish;\n\
         end endmodule");
    assert!(ok && o.contains("whole=-1 mem=255"), "got:\n{o}");
}

/// BYTE-IDENTITY: a plain (default) and an explicit-`unsigned` packed struct stay
/// UNSIGNED — the fix must not change any non-`signed` struct.
#[test]
fn unsigned_struct_unchanged() {
    let (o, ok) = run("module t;\n\
         typedef struct packed          { logic [7:0] v; } d_t;\n\
         typedef struct packed unsigned { logic [7:0] v; } u_t;\n\
         d_t d; u_t u;\n\
         initial begin\n\
           d = 8'hFF; u = 8'hFF;\n\
           $display(\"d=%0d u=%0d dcmp=%b\", d, u, d < 0);\n\
           #1 $finish;\n\
         end endmodule");
    // both unsigned 255 ; 255 < 0 == 0
    assert!(ok && o.contains("d=255 u=255 dcmp=0"), "got:\n{o}");
}

/// vita-INTERNAL equivalence teeth: a signed packed struct wrapping `logic [7:0]`
/// must behave BYTE-IDENTICALLY to a plain `logic signed [7:0]` for every
/// whole-value operation. Both printed lines must match, char-for-char.
#[test]
fn signed_struct_equiv_plain_signed_vector() {
    let mk = |decl: &str, name: &str| {
        format!(
            "module t;\n{decl}\n\
             initial begin\n\
               {name} = 8'hFF;\n\
               $display(\"%0d %b %0d %b %b\", {name}, {name} < 0, {name} + 1, {name} >>> 1, {name} <= -1);\n\
               {name} = 8'h80;\n\
               $display(\"%0d\", {name});\n\
               #1 $finish;\n\
             end endmodule",
        )
    };
    let (a, oka) = run(&mk(
        "typedef struct packed signed { logic [7:0] v; } ss_t; ss_t x;",
        "x",
    ));
    let (b, okb) = run(&mk("logic signed [7:0] x;", "x"));
    assert!(oka && okb, "run failed:\nstruct:\n{a}\nplain:\n{b}");
    assert_eq!(a, b, "signed struct diverged from plain signed vector");
}
