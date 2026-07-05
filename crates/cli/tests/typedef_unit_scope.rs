//! Compilation-unit type-name scoping (IEEE §3.12.1). vita's parser keeps type
//! names in flat unit-global maps; a package or module-local `typedef` used to
//! LEAK its BARE name to every later-parsed unit, so:
//!   - `package p; typedef t; module m; t x;` (NO import) silently resolved `t`
//!     to `p::t` — iverilog rejects "undeclared t" (a package type is invisible
//!     without `import`/`p::t`);
//!   - a module-local `typedef t;` leaked into a later module's body.
//!
//! Both are silent-accepts of code iverilog rejects (correct-or-loud violation).
//!
//! Fixed by snapshotting the type registries around each top-level unit and
//! restoring them after (`restore_scope_unit`) — dropping the unit's BARE names
//! while KEEPING the scoped `pkg::t` twins — plus a parse-time `import` type-copy
//! (`import p::t;` / `import p::*;` copies the scoped twin back to its bare name,
//! since the leak it used to rely on is gone). Pure parser, AST/`.vu`/format
//! unchanged, IR-0. Pinned to iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_tus_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let first = String::from_utf8_lossy(&out.stdout)
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .to_owned();
    (first, out.status.success())
}

// ── supported: imports / scoped access still resolve ─────────────────

#[test]
fn wildcard_import_brings_types() {
    // `import p::*` brings a vector / packed-struct / enum type into bare scope;
    // a scoped `p::s_t` also resolves. iverilog: "dead aabb 1".
    let (o, ok) = run(
        "package p; typedef logic [15:0] w_t; typedef struct packed {logic[7:0] a,b;} s_t; typedef enum logic[1:0]{X,Y,Z} e_t; endpackage\n\
         module top;\n  import p::*;\n  w_t w; p::s_t s; e_t e;\n  initial begin w=16'hDEAD; s.a=8'hAA; s.b=8'hBB; e=Y; $display(\"%h %h %0d\", w, s, e); #1 $finish; end\nendmodule\n",
    );
    assert!(ok && o == "dead aabb 1", "got:\n{o}");
}

#[test]
fn scoped_type_without_import_works() {
    // `p::w_t` resolves without any import. iverilog: "beef".
    let (o, ok) = run(
        "package p; typedef logic [15:0] w_t; endpackage\n\
         module top;\n  p::w_t w;\n  initial begin w=16'hBEEF; $display(\"%h\", w); #1 $finish; end\nendmodule\n",
    );
    assert!(ok && o == "beef", "got:\n{o}");
}

#[test]
fn package_var_and_type_import_coexist() {
    // A wildcard import brings BOTH a package variable and a type; a scoped
    // `p::cnt` also resolves. iverilog: "cnt=5 x=aa q=5".
    let (o, ok) = run(
        "package p; int cnt = 5; typedef logic[7:0] b_t; endpackage\n\
         module top;\n  import p::*;\n  b_t x;\n  initial begin x=8'hAA; $display(\"cnt=%0d x=%h q=%0d\", cnt, x, p::cnt); #1 $finish; end\nendmodule\n",
    );
    assert!(ok && o == "cnt=5 x=aa q=5", "got:\n{o}");
}

#[test]
fn two_package_type_imports() {
    // Two packages, both wildcard-imported, distinct types. iverilog: "11 2222".
    let (o, ok) = run(
        "package a; typedef logic[7:0] ta; endpackage\n\
         package b; typedef logic[15:0] tb; endpackage\n\
         module top;\n  import a::*; import b::*;\n  ta x; tb y;\n  initial begin x=8'h11; y=16'h2222; $display(\"%h %h\", x, y); #1 $finish; end\nendmodule\n",
    );
    assert!(ok && o == "11 2222", "got:\n{o}");
}

// ── the fix: leaked bare names are now loud (iverilog rejects them too) ──

#[test]
fn package_bare_type_without_import_is_loud() {
    // Using a package type by its BARE name WITHOUT `import`/`p::` is loud — the
    // package type is not visible in the compilation unit (iverilog: syntax
    // error). Was silently resolved via the leaked bare name.
    let (_, ok) = run(
        "package p; typedef logic [15:0] t; endpackage\n\
         module top;\n  t a;\n  initial begin a=16'hFFFF; $display(\"%h\", a); #1 $finish; end\nendmodule\n",
    );
    assert!(!ok, "bare package type without import must be loud");
}

#[test]
fn module_local_type_does_not_leak_forward() {
    // A module-local typedef is unit-scoped; a LATER module that never declares
    // the name must not silently see it (iverilog: syntax error). Was leaked.
    let (_, ok) = run(
        "module m2; typedef logic [15:0] byte_t; byte_t v; initial v=16'h0; endmodule\n\
         module top;\n  byte_t w;\n  initial begin w=16'hFFFF; $display(\"%h\", w); #1 $finish; end\nendmodule\n",
    );
    assert!(
        !ok,
        "module-local typedef must not leak into a later module"
    );
}

#[test]
fn local_type_shadows_within_module_only() {
    // A module MAY reuse a name locally; that local typedef is confined to the
    // module and does not leak. Here both modules declare their own `t` — each
    // sees its own. iverilog: "m1=f m2=ff" ($bits differ: 4 vs 8).
    let (o, ok) = run(
        "module m1; typedef logic [3:0] t; t a; initial begin a=4'hF; $display(\"m1=%h w=%0d\", a, $bits(a)); end endmodule\n\
         module m2; typedef logic [7:0] t; t b; initial begin b=8'hFF; $display(\"m2=%h w=%0d\", b, $bits(b)); #1 $finish; end endmodule\n",
    );
    // First stdout line is m1's (module order); both must elaborate clean.
    assert!(ok && o == "m1=f w=4", "got:\n{o}");
}
