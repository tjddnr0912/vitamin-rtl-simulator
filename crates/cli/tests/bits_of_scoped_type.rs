//! §3 ⑤ⓕ residue: `$bits(pkg::T)` — the PACKAGE-SCOPED spelling of a bare type
//! name.
//!
//! `parse_bits_type_arg` admitted a bare identifier only, so `$bits(pk::a_t)`
//! fell through to the expression path and elaborate reported `pk::a_t` "does not
//! name a package constant or variable" (E3009). The scalar twin `$bits(pk::e_t)`
//! was loud for the same reason, so one gate buys both.
//!
//! No new width rule: the parser already registers a `"pkg::T"` twin of every
//! package typedef in `typedefs` / `struct_layouts` / `union_type_names` (the twin
//! `pkg::T'(e)` casts read), so `bits_of_type_name` answers a scoped key exactly
//! as it answers a bare one — INCLUDING its declines, which is what keeps the
//! loud shapes loud.
//!
//! Every value is pinned to LIVE iverilog 13.0 AND verilator 5.052; both agree on
//! all nine accepted cells.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, i32) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_bst_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
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
        out.status.code().unwrap_or(-1),
    )
}

const PKG: &str = "package pk;\n\
   typedef logic [7:0] a_t [0:3];\n\
   typedef logic [7:0] e_t;\n\
   typedef logic [3:0][7:0] pk_t;\n\
   typedef struct packed { logic [7:0] a; logic [3:0] b; } s_t;\n\
   typedef union  packed { logic [7:0] a; logic [7:0] b; } u_t;\n\
   typedef enum logic [2:0] { A, B } en_t;\n\
   typedef int i_t;\n\
   localparam int W = 5;\n\
   typedef logic [7:0] a2_t [0:1][0:2];\n\
 endpackage\n";

/// Every shape `bits_of_type_name` can answer, through the scoped spelling. The
/// numbers are the oracles', not re-derived: unpacked array 8x4, scalar 8, packed
/// 2-D 4x8, struct 8+4, union max(8,8), enum base 3, `int` 32, and the 2-D
/// unpacked 8x2x3.
#[test]
fn every_scoped_type_shape_folds_to_the_oracles_width() {
    let (out, code) = run(&format!(
        "{PKG}module top; initial begin\n\
           $display(\"a=%0d\", $bits(pk::a_t));\n\
           $display(\"e=%0d\", $bits(pk::e_t));\n\
           $display(\"p=%0d\", $bits(pk::pk_t));\n\
           $display(\"s=%0d\", $bits(pk::s_t));\n\
           $display(\"u=%0d\", $bits(pk::u_t));\n\
           $display(\"n=%0d\", $bits(pk::en_t));\n\
           $display(\"i=%0d\", $bits(pk::i_t));\n\
           $display(\"d=%0d\", $bits(pk::a2_t));\n\
         $finish; end endmodule\n"
    ));
    assert_eq!(code, 0, "expected exit 0:\n{out}");
    for want in ["a=32", "e=8", "p=32", "s=12", "u=8", "n=3", "i=32", "d=48"] {
        assert!(out.contains(want), "missing `{want}` in:\n{out}");
    }
}

/// The bare-name twin of the same package types, to say the scoped path added no
/// width rule of its own — the two spellings must agree cell for cell.
#[test]
fn the_bare_name_twin_answers_identically() {
    let (out, code) = run("module top;\n\
           typedef logic [7:0] a_t [0:3];\n\
           typedef logic [7:0] e_t;\n\
           initial begin $display(\"a=%0d\", $bits(a_t)); $display(\"e=%0d\", $bits(e_t));\n\
             $finish; end\n\
         endmodule\n");
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("a=32") && out.contains("e=8"), "{out}");
}

/// The gate must fire ONLY on a type key. A scoped VARIABLE, a scoped enum LABEL
/// and a scoped typed LOCALPARAM all keep the expression path — `bits_of_type_name`
/// declines them, and the decline happens BEFORE any token is consumed so the
/// cursor is intact for the expression parse. All three are three-way pinned.
#[test]
fn a_scoped_non_type_still_takes_the_expression_path() {
    let (out, code) = run("package pk;\n\
           logic [11:0] v = 12'h5a;\n\
           typedef enum logic [2:0] { LBL, LB2 } en_t;\n\
           localparam logic [5:0] W = 6'd3;\n\
         endpackage\n\
         module top; initial begin\n\
           $display(\"var=%0d\", $bits(pk::v));\n\
           $display(\"lbl=%0d\", $bits(pk::LBL));\n\
           $display(\"w=%0d\", $bits(pk::W));\n\
         $finish; end endmodule\n");
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("var=12") && out.contains("lbl=3") && out.contains("w=6"),
        "{out}"
    );
}

/// The DECLINE set stays loud. None of these four has a consensus oracle — a
/// `real` typedef is iverilog-reject against verilator 64, a `string` one is
/// iverilog-reject against a verilator INTERNAL FAULT, and `[]` / `[$]` are
/// rejected or unsupported by both — so folding any of them would be a
/// silent-wrong with nothing to pin it to.
#[test]
fn the_shapes_without_an_oracle_stay_loud_through_the_scoped_spelling() {
    for (ty, what) in [
        ("typedef real r_t;", "pk::r_t"),
        ("typedef string s_t;", "pk::s_t"),
        ("typedef logic [7:0] d_t [];", "pk::d_t"),
        ("typedef logic [7:0] q_t [$];", "pk::q_t"),
    ] {
        let (out, code) = run(&format!(
            "package pk; {ty} endpackage\n\
             module top; initial begin $display(\"v=%0d\", $bits({what})); $finish; end endmodule\n"
        ));
        assert_eq!(code, 1, "expected loud for {what}:\n{out}");
        assert!(!out.contains("v="), "{what} folded a value:\n{out}");
    }
}

/// An unknown scoped name is loud, not a fold — iverilog answers 0 and verilator
/// errors, so there is no value to take.
#[test]
fn an_unknown_scoped_name_is_loud() {
    let (out, code) = run(
        "package pk; typedef logic [7:0] e_t; endpackage\n\
         module top; initial begin $display(\"v=%0d\", $bits(pk::nope_t)); $finish; end endmodule\n",
    );
    assert_eq!(code, 1, "{out}");
    assert!(!out.contains("v="), "{out}");
}

/// Control: a VARIABLE declared with the scoped type keeps answering through the
/// net path (this one always worked, and must not move).
#[test]
fn a_variable_of_the_scoped_type_is_unchanged() {
    let (out, code) = run("package pk; typedef logic [7:0] e_t; endpackage\n\
         module top; pk::e_t v;\n\
           initial begin v = 8'h5a; $display(\"v=%0d\", $bits(v)); $finish; end\n\
         endmodule\n");
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("v=8"), "{out}");
}
