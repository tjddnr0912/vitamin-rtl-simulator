//! A 2-state (`bit`/`byte`/`int`/…) class FIELD did not coerce a written X/Z to 0
//! (IEEE §6.11.3): `c.f = 8'hxA` on a `bit [7:0] f` read back `xA`, not `0A`. The engine
//! `class_field_write` resized the value to the field width but never applied the 2-state
//! X/Z→0 coercion that the frame-slot (`coerce_two_state_frame`) and module-net write
//! paths do. The per-field `four_state` flag is already in the class layout, so the fix
//! coerces `!four_state` fields in `class_field_write`. A 4-state `logic`/`integer` field
//! keeps X. This is the object-heap sibling of the class-method-local coercion (§4.5.85).
//! Pinned to iverilog 13.0 with `8'hxA` (high nibble X): 2-state → `0a`, 4-state → `xa`.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_cf2s_{}_{n}", std::process::id()));
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

/// Class `C { <field> }`; write `8'hxA` to `c.f` and print it `%h`.
fn fld(field: &str) -> String {
    format!(
        "module m;\n\
         class C; {field} endclass\n\
         C c; initial begin c = new; c.f = 8'hxA; $display(\"%h\", c.f); #1 $finish; end endmodule\n"
    )
}

#[test]
fn two_state_field_coerces_x_to_zero() {
    for field in ["bit [7:0] f;", "byte f;"] {
        let (out, code) = run(&fld(field));
        assert_eq!(code, Some(0), "{out}");
        assert!(out.contains("0a"), "{field} coerce, got:\n{out}");
    }
    // A wider `int` field, X in the high bits.
    let (out, code) = run("module m;\n\
         class C; int f; endclass\n\
         C c; initial begin c = new; c.f = 32'hxxxx_xxAB; $display(\"%h\", c.f); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("000000ab"), "int field coerce, got:\n{out}");
}

#[test]
fn four_state_field_keeps_x() {
    let (out, code) = run(&fld("logic [7:0] f;"));
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("xa"), "logic field keeps X, got:\n{out}");
    let (out, code) = run("module m;\n\
         class C; integer f; endclass\n\
         C c; initial begin c = new; c.f = 32'hxxxx_xxAB; $display(\"%h\", c.f); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("xxxxxxab"),
        "integer field keeps X, got:\n{out}"
    );
}

#[test]
fn z_write_coerces_and_normal_unchanged() {
    let (out, code) = run("module m;\n\
         class C; bit [7:0] f; endclass\n\
         C c; initial begin c = new; c.f = 8'hzz; $display(\"%h\", c.f); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("00"), "Z-write coerce, got:\n{out}");
    let (out, code) = run("module m;\n\
         class C; bit [7:0] f; endclass\n\
         C c; initial begin c = new; c.f = 8'hAB; $display(\"%h\", c.f); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("ab"), "normal value, got:\n{out}");
}

#[test]
fn field_write_via_method_coerces() {
    // `this.f = u` inside a method reaches the same `class_field_write` path.
    let (out, code) = run("module m;\n\
         class C; bit [7:0] f; function void s(input [7:0] u); this.f = u; endfunction endclass\n\
         C c; initial begin c = new; c.s(8'hxA); $display(\"%h\", c.f); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("0a"), "this.f= coerce, got:\n{out}");
}

#[test]
fn inherited_and_derived_field_coerce() {
    // A base-class 2-state field accessed via a derived handle.
    let (out, code) = run("module m;\n\
         class B; bit [7:0] f; endclass\n\
         class D extends B; endclass\n\
         D d; initial begin d = new; d.f = 8'hxA; $display(\"%h\", d.f); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("0a"), "inherited field, got:\n{out}");
    // A field added by the derived class.
    let (out, code) = run("module m;\n\
         class B; int a; endclass\n\
         class D extends B; bit [7:0] f; endclass\n\
         D d; initial begin d = new; d.f = 8'hxA; $display(\"%h\", d.f); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("0a"), "derived field, got:\n{out}");
}

#[test]
fn mixed_fields_coerce_per_field() {
    // A `logic` field keeps X, a `bit` field in the SAME class coerces (per-field flag).
    let (out, code) = run("module m;\n\
         class C; logic [7:0] g; bit [7:0] f; endclass\n\
         C c; initial begin c = new; c.g = 8'hxA; c.f = 8'hxA; $display(\"%h %h\", c.g, c.f); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("xa 0a"), "mixed fields, got:\n{out}");
}
