//! `$typename` (IEEE 1800 §20.6.1) had ZERO test coverage, and iverilog 13.0 does
//! not implement it at all ("System task/function $typename() is not defined"), so
//! differential testing can never notice it regressing. Found by the zero-coverage
//! scan strategy (§4.5.236/237); unlike `%p`, which that scan found broken, this
//! one works — the exposure was that nothing pinned it.
//!
//! Pinned: the atom types, packed vectors, and the unpacked-array form, which uses
//! IEEE's `$[lo:hi]` notation for the unpacked dimensions.
//!
//! RESIDUAL (ROADMAP §3, no oracle): an ENUM and a PACKED STRUCT render as their
//! underlying base type (`logic[1:0]`, `logic[3:0]`) rather than IEEE's
//! `enum{...}` / `struct packed{...}` forms. That is a type-NAME rendering
//! simplification — the values and every other use of those types are unaffected —
//! so it is recorded rather than guessed at. Exact spacing is vita's own choice
//! absent an oracle; what these tests protect is that the strings stay stable and
//! stay distinct per type.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_tn_{}_{n}", std::process::id()));
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

const SRC: &str = "module m;\n\
       typedef enum bit [1:0] { A, B } e_t;\n\
       typedef struct packed { logic [3:0] a; } sp_t;\n\
       logic [7:0] v; logic w; int i; integer g; byte b; shortint sh; longint lo;\n\
       real r; time t; string s; bit [3:0] bv; logic [7:0] arr [3]; e_t e; sp_t sp;\n\
       initial begin\n\
         $display(\"1=%s 2=%s 3=%s\", $typename(v), $typename(w), $typename(i));\n\
         $display(\"4=%s 5=%s 6=%s\", $typename(g), $typename(b), $typename(sh));\n\
         $display(\"7=%s 8=%s 9=%s\", $typename(lo), $typename(r), $typename(t));\n\
         $display(\"10=%s 11=%s\", $typename(s), $typename(bv));\n\
         $display(\"12=%s 13=%s 14=%s\", $typename(arr), $typename(e), $typename(sp));\n\
         #1 $finish; end\nendmodule\n";

/// The atom types each report their own name — no two collapse together.
#[test]
fn typename_reports_each_atom_type() {
    let (out, c) = run(SRC);
    assert_eq!(c, Some(0), "no diagnostics expected; got:\n{out}");
    assert!(out.contains("1=logic[7:0] 2=logic 3=int"), "got:\n{out}");
    assert!(out.contains("4=integer 5=byte 6=shortint"), "got:\n{out}");
    assert!(out.contains("7=longint 8=real 9=time"), "got:\n{out}");
    assert!(out.contains("10=string 11=bit[3:0]"), "got:\n{out}");
}

/// An unpacked array appends its dimensions in IEEE's `$[lo:hi]` notation, which
/// is what distinguishes it from the packed vector of the same element type.
#[test]
fn typename_marks_unpacked_dimensions() {
    let (out, c) = run(SRC);
    assert_eq!(c, Some(0), "no diagnostics expected; got:\n{out}");
    assert!(
        out.contains("12=logic[7:0]$[0:2]"),
        "unpacked dims; got:\n{out}"
    );
    // The unpacked form must not be confusable with its element type.
    assert!(
        !out.contains("12=logic[7:0] "),
        "unpacked array must not render as its element; got:\n{out}"
    );
}

/// RESIDUAL pinned so it is visible: an enum and a packed struct currently render
/// as their base type, not as IEEE's `enum{...}` / `struct packed{...}`.
#[test]
fn typename_of_enum_and_packed_struct_is_the_base_type_today() {
    let (out, c) = run(SRC);
    assert_eq!(c, Some(0), "no diagnostics expected; got:\n{out}");
    assert!(
        out.contains("13=logic[1:0] 14=logic[3:0]"),
        "recorded simplification (ROADMAP §3); got:\n{out}"
    );
}
