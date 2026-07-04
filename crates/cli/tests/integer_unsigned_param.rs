//! A `parameter`/`localparam` (or variable) declared `integer unsigned` was a loud
//! parse error (E2002) — the `integer` atom's parser consumed only a LEADING
//! `signed`/`unsigned`, not the trailing one the `int`/`byte`/… atoms accept. So
//! `integer unsigned P` left `unsigned` unparsed. This is the `integer` companion to
//! §4.5.91 (which honored `int/byte/shortint/longint unsigned`): the parser now also
//! parses a trailing `opt_signed()` after `integer`, and an explicit signing (either
//! position) wins over the V2005 `integer` = 32-bit-signed default. Pinned to
//! iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_intu_{}_{n}", std::process::id()));
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

#[test]
fn integer_unsigned_param_is_unsigned() {
    // `integer unsigned P = -1` → P is unsigned, so (P > 100) is TRUE (1). Was E2002.
    let src = "module m; parameter integer unsigned P = -1;\n\
         initial begin $display(\"r=%0d\", (P > 100)); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=1"), "integer unsigned > 100, got:\n{out}");
}

#[test]
fn integer_signed_param_is_signed() {
    // Explicit `integer signed P = -1` → signed, (P > 100) is FALSE (0).
    let src = "module m; parameter integer signed P = -1;\n\
         initial begin $display(\"r=%0d\", (P > 100)); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=0"), "integer signed > 100, got:\n{out}");
}

#[test]
fn plain_integer_param_default_signed() {
    // GUARD: a plain `integer P = -1` keeps its V2005 signed default (byte-identical
    // to before — `opt_signed()` returns None → same `unwrap_or(true)`). (P>100)=0.
    let src = "module m; parameter integer P = -1;\n\
         initial begin $display(\"r=%0d\", (P > 100)); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=0"), "plain integer > 100, got:\n{out}");
}

#[test]
fn int_unsigned_param_unchanged() {
    // GUARD: the §4.5.91 `int unsigned` path is untouched by this change. -1 → 1.
    let src = "module m; parameter int unsigned R = -1;\n\
         initial begin $display(\"r=%0d\", (R > 100)); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=1"), "int unsigned > 100, got:\n{out}");
}

#[test]
fn integer_unsigned_arithmetic() {
    // Unsigned semantics propagate to arithmetic: 0xFFFFFFFF / 2 = 2147483647
    // (unsigned), not a signed -1/2 = 0.
    let src = "module m; localparam integer unsigned P = 32'hFFFFFFFF;\n\
         initial begin $display(\"r=%0d\", P / 2); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("r=2147483647"),
        "integer unsigned division, got:\n{out}"
    );
}

#[test]
fn integer_unsigned_var() {
    // The variable form `integer unsigned v` also parses and is unsigned. -1 → 1.
    let src = "module m; integer unsigned v;\n\
         initial begin v = -1; $display(\"r=%0d\", (v > 100)); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("r=1"),
        "integer unsigned var > 100, got:\n{out}"
    );
}
