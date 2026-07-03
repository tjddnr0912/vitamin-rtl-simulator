//! An `int`/`byte`/`shortint`/`longint` PARAMETER (or localparam) declared
//! `unsigned` dropped the `unsigned` qualifier and was treated as SIGNED — so
//! `parameter int unsigned P = -1; (P > 100)` gave 0 (signed -1) instead of 1
//! (unsigned 0xFFFFFFFF). Two bugs: the parser's `signed = signed || expl` could
//! not flip a 2-state atom's signed DEFAULT back to unsigned (byte/shortint/
//! longint), and `param_decl_width`/`coerce_param_value` hard-coded `int`/`integer`
//! as signed (ignoring `p.signed`). Fix: the parser lets an explicit `signed`/
//! `unsigned` win over the atom default (and `int`/`integer` default to signed via
//! `p.signed`), and the elaborate param typing reads `p.signed`. Pinned to iverilog
//! 13.0. Discriminator: `P = -1` (all-ones) compared `> 100` — unsigned is huge
//! (true), signed is -1 (false).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_iup_{}_{n}", std::process::id()));
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
        out.status.code(),
    )
}

/// `<decl> P = -1;` then print `(P > 100)` as a number (unsigned ⇒ 1, signed ⇒ 0).
fn cmp(decl: &str) -> String {
    format!(
        "module m;\n\
         {decl} P = -1;\n\
         initial begin $display(\"r=%0d\", (P > 100)); #1 $finish; end endmodule\n"
    )
}

#[test]
fn unsigned_atom_params_are_unsigned() {
    // Each 2-state atom (and `int`) with `unsigned` must compare as unsigned (big).
    for decl in [
        "parameter int unsigned",
        "localparam int unsigned",
        "parameter byte unsigned",
        "parameter shortint unsigned",
        "parameter longint unsigned",
    ] {
        let (out, code) = run(&cmp(decl));
        assert_eq!(code, Some(0), "{out}");
        assert!(
            out.contains("r=1"),
            "`{decl}` should be unsigned, got:\n{out}"
        );
    }
}

#[test]
fn signed_atom_params_unchanged() {
    // Bare atoms (and `integer`, `signed [7:0]`) stay SIGNED (small) — the fix must
    // not perturb the default-signed behavior.
    for decl in [
        "parameter int",
        "parameter byte",
        "parameter shortint",
        "parameter longint",
        "parameter integer",
        "parameter signed [7:0]",
    ] {
        let (out, code) = run(&cmp(decl));
        assert_eq!(code, Some(0), "{out}");
        assert!(
            out.contains("r=0"),
            "`{decl}` should be signed, got:\n{out}"
        );
    }
}

#[test]
fn default_unsigned_vector_params_unchanged() {
    // A `logic`/`bit` vector (or untyped vector) param is unsigned by default —
    // unchanged (big).
    for decl in [
        "parameter logic [31:0]",
        "parameter bit [31:0]",
        "parameter [31:0]",
    ] {
        let (out, code) = run(&cmp(decl));
        assert_eq!(code, Some(0), "{out}");
        assert!(
            out.contains("r=1"),
            "`{decl}` should be unsigned, got:\n{out}"
        );
    }
}

#[test]
fn int_unsigned_value_coercion() {
    // The folded VALUE of an `int unsigned` param is the unsigned bit pattern, not a
    // sign-extended negative: 0xFFFFFFFF prints 4294967295, not -1.
    let (out, code) = run("module m;\n\
         parameter int unsigned P = 32'hFFFFFFFF;\n\
         initial begin $display(\"r=%0d\", P); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("r=4294967295"),
        "int unsigned value, got:\n{out}"
    );
}

#[test]
fn signed_int_value_unchanged() {
    // Bare `int` param keeps sign-extension: 0xFFFFFFFF prints -1.
    let (out, code) = run("module m;\n\
         parameter int P = 32'hFFFFFFFF;\n\
         initial begin $display(\"r=%0d\", P); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=-1"), "signed int value, got:\n{out}");
}

#[test]
fn int_unsigned_used_as_width() {
    // A positive `int unsigned` param used as a width is unaffected by the sign fix.
    let (out, code) = run("module m;\n\
         parameter int unsigned W = 8;\n\
         logic [W-1:0] x;\n\
         initial begin x = 8'hFF; $display(\"r=%0d\", x); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=255"), "int unsigned width, got:\n{out}");
}
