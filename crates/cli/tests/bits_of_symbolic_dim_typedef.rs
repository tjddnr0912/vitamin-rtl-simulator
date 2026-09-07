//! §3 ⑤ⓕ residue: `$bits(a_t)` where the typedef's UNPACKED dimension names a
//! `parameter`.
//!
//! `parse_bits_sym_type_arg` — the desugar that answers `$bits(T)` with a width
//! EXPRESSION so elaborate folds it per instance, after the override — declined
//! `!info.unpacked.is_empty()` ("no dim slot in the desugar"), and the numeric
//! twin `bits_of_type_name` cannot fold an overridable name at parse time. So the
//! packed spelling `logic [N-1:0]` already answered 8 under `#(.N(8))` while the
//! unpacked one beside it was E3010 + E3009.
//!
//! The fix is a product: element × every packed dim × every unpacked dim, each
//! factor literal where the parse-time table folds it and symbolic where it does
//! not. That is what lets a LITERAL element width sit beside a symbolic dimension
//! (`logic [7:0] a_t [0:N-1]`), which is the shape the row names — the old code
//! took `sym_range_width` of the element range alone and had nowhere to put the
//! dimension.
//!
//! ⚠️ The answer must stay an EXPRESSION. A parse-time number would bake the
//! PRE-override width, which is exactly the guard-rail the ROADMAP row credited
//! the decline with; the product keeps it because every factor is lowered, not
//! folded.
//!
//! Every value is pinned to LIVE iverilog 13.0 and verilator 5.052; both agree on
//! every accepted cell here.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// `(stdout, exit code)`. The code matters: the defect was a LOUD refusal, so a
/// stdout-only assertion cannot tell a fix from a fold that never ran.
fn run(src: &str) -> (String, i32) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_bsd_{}_{n}", std::process::id()));
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

/// The row's own design, at the module's own default: `8 × 4`.
#[test]
fn an_unpacked_dim_naming_a_parameter_folds() {
    let (out, code) = run("module top;\n\
           parameter N = 4;\n\
           typedef logic [7:0] a_t [0:N-1];\n\
           initial begin $display(\"R=%0d\", $bits(a_t)); $finish; end\n\
         endmodule\n");
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("R=32"), "{out}");
}

/// …and the half that says the answer is an EXPRESSION and not a parse-time
/// number: under `#(.N(8))` both oracles read 64, not the declaration's own 32.
#[test]
fn the_width_follows_the_instance_override() {
    let (out, code) = run("module m #(parameter N = 4);\n\
           typedef logic [7:0] a_t [0:N-1];\n\
           initial begin $display(\"R=%0d\", $bits(a_t)); $finish; end\n\
         endmodule\n\
         module top; m #(.N(8)) u1(); endmodule\n");
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("R=64"),
        "expected the POST-override width:\n{out}"
    );
}

/// Two instances of the SAME module in one design, so the cell cannot pass by
/// baking any single width: 8×2 and 8×8 from one typedef.
#[test]
fn two_instances_of_one_module_get_their_own_widths() {
    let (out, code) = run("module m #(parameter N = 4);\n\
           typedef logic [7:0] a_t [0:N-1];\n\
           initial $display(\"R%0d=%0d\", N, $bits(a_t));\n\
         endmodule\n\
         module top; m #(.N(2)) u1(); m #(.N(8)) u2();\n\
           initial begin #1; $finish; end\n\
         endmodule\n");
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("R2=16") && out.contains("R8=64"), "{out}");
}

/// Every factor the product has to carry, each under an override so a literal
/// fold cannot produce the number: a `[N]` size dim, two unpacked dims, a
/// symbolic ELEMENT beside a literal dim, a symbolic PACKED dim, and a range-free
/// (1-bit) element.
#[test]
fn every_factor_shape_composes() {
    for (decl, want) in [
        ("typedef logic [7:0] a_t [N];", "R=64"),
        ("typedef logic [7:0] a_t [0:N-1][0:2];", "R=192"),
        ("typedef logic [N-1:0] a_t [0:1];", "R=16"),
        ("typedef logic [7:0][N-1:0] a_t;", "R=64"),
        ("typedef logic a_t [0:N-1];", "R=8"),
    ] {
        let (out, code) = run(&format!(
            "module m #(parameter N = 4);\n\
               {decl}\n\
               initial begin $display(\"R=%0d\", $bits(a_t)); $finish; end\n\
             endmodule\n\
             module top; m #(.N(8)) u1(); endmodule\n"
        ));
        assert_eq!(code, 0, "{decl}:\n{out}");
        assert!(out.contains(want), "{decl} wanted {want}:\n{out}");
    }
}

/// The DECLINE set is unchanged, and it has no oracle to move toward: iverilog
/// rejects `$bits` of a `[]` / `[$]` typedef ("Invalid data type for $bits()")
/// and verilator reports an internal fault on both.
#[test]
fn a_dynamic_or_queue_dim_stays_loud() {
    for dim in ["[]", "[$]"] {
        let (out, code) = run(&format!(
            "module m #(parameter N = 4);\n\
               typedef logic [7:0] a_t {dim};\n\
               initial begin $display(\"R=%0d\", $bits(a_t)); $finish; end\n\
             endmodule\n\
             module top; m #(.N(8)) u1(); endmodule\n"
        ));
        assert_eq!(code, 1, "expected loud for {dim}:\n{out}");
        assert!(!out.contains("R="), "{dim} folded:\n{out}");
    }
}

/// Controls that must NOT move — each one already answered before this slice and
/// takes a different path: an all-literal typedef (the numeric fold), a
/// `localparam` dim (also the numeric fold — it is not overridable, so the
/// parse-time table holds it), and the packed twin that was already symbolic.
#[test]
fn the_paths_that_already_answered_are_unchanged() {
    let (out, code) = run("module top;\n\
           localparam N = 4;\n\
           typedef logic [7:0] lit_t [0:3];\n\
           typedef logic [7:0] loc_t [0:N-1];\n\
           initial begin $display(\"R=%0d %0d\", $bits(lit_t), $bits(loc_t)); $finish; end\n\
         endmodule\n");
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("R=32 32"), "{out}");

    let (out, code) = run("module m #(parameter N = 4);\n\
           typedef logic [N-1:0] p_t;\n\
           initial begin $display(\"R=%0d\", $bits(p_t)); $finish; end\n\
         endmodule\n\
         module top; m #(.N(8)) u1(); endmodule\n");
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("R=8"), "{out}");
}

/// The PACKAGE-scoped spelling of the same shape is a SEPARATE root and its own
/// slice (`pkg_scoped_unpacked_dim_twin.rs`): the `pkg::T` twin left the unpacked
/// dimension's names bare, which is a respell question and not this desugar. Both
/// spellings are pinned here so the two cannot drift apart — 32 in all three tools.
#[test]
fn the_package_scoped_spelling_answers_the_same_width() {
    let (out, code) = run(
        "package pk; localparam N = 4; typedef logic [7:0] a_t [0:N-1]; endpackage\n\
         module top; initial begin $display(\"R=%0d\", $bits(pk::a_t)); $finish; end endmodule\n",
    );
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("R=32"), "{out}");

    let (out, code) = run(
        "package pk; localparam N = 4; typedef logic [N-1:0] p_t; endpackage\n\
         module top; initial begin $display(\"R=%0d\", $bits(pk::p_t)); $finish; end endmodule\n",
    );
    assert_eq!(code, 0, "the packed twin folds:\n{out}");
    assert!(out.contains("R=4"), "{out}");
}

/// A VARIABLE of the type's own name shadows it, and the parser folds `$bits(<type>)`
/// with no scope of its own — so the fold must stand down whenever the enclosing body
/// DECLARES that name. Four binders: a block-local, an ANSI formal, a non-ANSI formal,
/// and (the control) a module-scope net.
///
/// verilator is the oracle here — it reads 12, the declared width of the local, in
/// every one of these; iverilog rejects a name used as both a type and a variable, so
/// it cannot judge. vita's own pre-fold answer was 12 too, which is why letting the
/// type's width through would have been a correct → silent-wrong trade.
#[test]
fn a_same_named_declaration_stands_the_fold_down() {
    let pre = "module top; parameter N = 4; typedef logic [7:0] a_t [0:N-1];\n";
    for (body, what) in [
        (
            "initial begin : b logic [11:0] a_t; a_t = 12'h5;\n\
               $display(\"R=%0d\", $bits(a_t)); $finish; end\n",
            "block-local",
        ),
        (
            "function automatic int f(input logic [11:0] a_t); return $bits(a_t); endfunction\n\
             initial begin $display(\"R=%0d\", f(12'h5)); $finish; end\n",
            "ANSI formal",
        ),
        (
            "function automatic int f;\n input logic [11:0] a_t;\n f = $bits(a_t);\n endfunction\n\
             initial begin $display(\"R=%0d\", f(12'h5)); $finish; end\n",
            "non-ANSI formal",
        ),
    ] {
        let (out, code) = run(&format!("{pre}  {body}endmodule\n"));
        assert_eq!(code, 0, "{what}:\n{out}");
        assert!(out.contains("R=12"), "{what} took the type's width:\n{out}");
    }
}

/// …and the stand-down is scoped: a formal's name is dropped at the end of its
/// subroutine, so the SAME `$bits(a_t)` outside it still folds the type. Without the
/// scope the fix would have traded one silent width for a permanently loud one.
#[test]
fn the_stand_down_does_not_leak_past_the_subroutine() {
    let (out, code) = run(
        "module top; parameter N = 4; typedef logic [7:0] a_t [0:N-1];\n\
           function automatic int f(input logic [11:0] a_t); return $bits(a_t); endfunction\n\
           initial begin $display(\"R=%0d %0d\", f(12'h5), $bits(a_t)); $finish; end\n\
         endmodule\n",
    );
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("R=12 32"), "{out}");
}
