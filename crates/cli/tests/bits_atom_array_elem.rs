//! `$bits` of an element of an unpacked array whose element is a fixed-width
//! integer ATOM (`byte`/`shortint`/`int`/`longint`) reported the rangeless default
//! of 1 instead of the kind's width: `byte a[2]; $bits(a[0])` gave 1, not 8. The
//! net STORAGE, `%b`, and arithmetic already sized the element correctly (via
//! `range_to_dims`); only the static `$bits` prescan (`prescan_net_bits`) was stale
//! — it special-cased `integer`/`real`/`time` but sent the fixed-width atoms to the
//! `range: None => 1` fallback. Fixed (§4.5.155) by giving the prescan the same
//! atom widths `range_to_dims` uses. `integer` (32), `bit`/`logic` ranges, and the
//! whole-array `$bits` are unaffected. All widths pinned to iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_baae_{}_{n}", std::process::id()));
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

/// An element of a `byte`/`shortint`/`int`/`longint` unpacked array reports its
/// kind width (8/16/32/64), not 1 (iverilog-pinned).
#[test]
fn atom_array_element_bits() {
    let (o, ok) = run("module t;\n\
         byte ba[2]; shortint sa[2]; int ia[2]; longint la[2];\n\
         initial begin\n\
           $display(\"b=%0d s=%0d i=%0d l=%0d\", $bits(ba[0]), $bits(sa[0]), $bits(ia[0]), $bits(la[0]));\n\
           #1 $finish;\n\
         end endmodule");
    assert!(ok && o.contains("b=8 s=16 i=32 l=64"), "got:\n{o}");
}

/// The whole-array `$bits` (element × count) and a multi-dim ELEMENT are also
/// correct: `byte[2]` = 16, `int[3]` = 96, `byte[2][3]` element = 8.
#[test]
fn atom_array_whole_and_multidim_element() {
    let (o, ok) = run("module t;\n\
         byte ba[2]; int ia[3]; byte m[2][3];\n\
         initial begin\n\
           $display(\"whole=%0d %0d mdelem=%0d\", $bits(ba), $bits(ia), $bits(m[0][0]));\n\
           #1 $finish;\n\
         end endmodule");
    assert!(ok && o.contains("whole=16 96 mdelem=8"), "got:\n{o}");
}

/// A forward reference — `localparam W = $bits(byte_arr[i])` used before the array
/// is otherwise consumed — folds to 8 (this is the prescan's raison d'être).
#[test]
fn atom_array_element_bits_forward_ref() {
    let (o, ok) = run("module t;\n\
         byte ba[4];\n\
         localparam W = $bits(ba[0]);\n\
         initial begin\n\
           $display(\"W=%0d\", W);\n\
           #1 $finish;\n\
         end endmodule");
    assert!(ok && o.contains("W=8"), "got:\n{o}");
}

/// BYTE-IDENTITY: ranged (`logic [7:0]`) elements, `integer` (32), and `bit` (1)
/// arrays are UNCHANGED — the fix only added the fixed-width atoms.
#[test]
fn ranged_integer_bit_array_element_unchanged() {
    let (o, ok) = run("module t;\n\
         logic [7:0] va[2]; integer ga[2]; bit bta[2];\n\
         initial begin\n\
           $display(\"v=%0d g=%0d b=%0d\", $bits(va[0]), $bits(ga[0]), $bits(bta[0]));\n\
           #1 $finish;\n\
         end endmodule");
    assert!(ok && o.contains("v=8 g=32 b=1"), "got:\n{o}");
}

/// The same prescan powers CONST-context `$bits` of a scalar atom, so the fix also
/// corrects `localparam W = $bits(byte_var)` and `logic [$bits(b)-1:0] vec` (both
/// were 1, now 8) — the const path consults the prescan first. iverilog-pinned.
#[test]
fn const_context_scalar_atom_bits() {
    let (o, ok) = run("module t;\n\
         byte b; int i;\n\
         localparam BW = $bits(b);\n\
         logic [$bits(b)-1:0] vec;\n\
         initial begin\n\
           $display(\"BW=%0d IW=%0d vecw=%0d\", BW, $bits(i), $bits(vec));\n\
           #1 $finish;\n\
         end endmodule");
    assert!(ok && o.contains("BW=8 IW=32 vecw=8"), "got:\n{o}");
}

/// vita-INTERNAL equivalence teeth: `$bits` of an atom-array element equals `$bits`
/// of a scalar of the same atom kind, and both equal the element's real `%b` width.
#[test]
fn atom_array_element_bits_equiv_scalar() {
    let (o, ok) = run("module t;\n\
         byte scal; byte arr[2];\n\
         initial begin\n\
           arr[0] = 8'hA5;\n\
           $display(\"scal=%0d elem=%0d valbits=%b\", $bits(scal), $bits(arr[0]), arr[0]);\n\
           #1 $finish;\n\
         end endmodule");
    // scalar bits == element bits == 8, and %b renders 8 bits
    assert!(
        ok && o.contains("scal=8 elem=8 valbits=10100101"),
        "got:\n{o}"
    );
}
