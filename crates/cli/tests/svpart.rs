//! SVPART: partial-SystemVerilog caveat items.
//!  - 2-state integer types (`bit`/`byte`/`shortint`/`int`/`longint`): X-free
//!    (default 0, and X/Z coerces to 0 on every write — IEEE §6.11.3), fixed
//!    widths, signed atoms (`bit` unsigned). AST `.vu` re-pin + map to
//!    `NetKind::Reg`; init-to-0 rides the golden SimIr, the write-time X→0
//!    coercion rides a one-shot `SimOpts.two_state_nets` sidecar (staged vrun does
//!    init-to-0 but not on-write coercion — frame-call sidecar precedent).
//!  - Wildcard-import ambiguity (IEEE §26.8): a name from two different wildcard
//!    imports is unbound ⇒ loud at the use site, silent if unused; an explicit
//!    import always wins.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_svp_{}_{n}", std::process::id()));
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

// ── 2-state integer types ─────────────────────────────────────────────────────

#[test]
fn two_state_types_init_to_zero() {
    // bit/byte/shortint/int/longint default-initialise to 0, never X (X-free init).
    let (out, _c) = run("module t;\n\
        bit b; byte c; shortint s; int i; longint l; bit [7:0] bv;\n\
        initial begin\n\
          $display(\"INIT b=%0d c=%0d s=%0d i=%0d l=%0d bv=%0d\", b, c, s, i, l, bv);\n\
        end endmodule\n");
    assert!(
        out.contains("INIT b=0 c=0 s=0 i=0 l=0 bv=0"),
        "2-state init-to-0:\n{out}"
    );
}

#[test]
fn two_state_widths_and_signedness() {
    let (out, _c) = run("module t;\n\
        bit b; byte c; shortint s; int i; longint l; bit [7:0] bv;\n\
        initial begin\n\
          $display(\"W %0d %0d %0d %0d %0d %0d\", $bits(b), $bits(c), $bits(s), $bits(i), $bits(l), $bits(bv));\n\
          c = -8'sd2; i = -5; bv = 8'hAB;\n\
          $display(\"V c=%0d i=%0d bv=%0d\", c, i, bv);\n\
        end endmodule\n");
    assert!(out.contains("W 1 8 16 32 64 8"), "2-state widths:\n{out}");
    // byte/int are signed (print negative); bit is unsigned (171, not -85).
    assert!(out.contains("V c=-2 i=-5 bv=171"), "2-state sign:\n{out}");
}

#[test]
fn two_state_types_are_procedurally_assignable() {
    // A 2-state var is a variable (procedural assign legal, no E3018 net error).
    let (out, _c) = run("module t; int i;\n\
        initial begin i = 42; $display(\"i=%0d\", i); end endmodule\n");
    assert!(out.contains("i=42"), "2-state procedural assign:\n{out}");
}

#[test]
fn two_state_typename() {
    let (out, _c) = run("module t;\n\
        bit b; byte c; int i; longint l; bit [3:0] bv;\n\
        initial begin\n\
          $display(\"<%s><%s><%s><%s><%s>\", $typename(b), $typename(c), $typename(i), $typename(l), $typename(bv));\n\
        end endmodule\n");
    assert!(
        out.contains("<bit><byte><int><longint><bit[3:0]>"),
        "2-state typename:\n{out}"
    );
}

// ── wildcard-import ambiguity (IEEE §26.8) ────────────────────────────────────

#[test]
fn ambiguous_wildcard_import_used_is_loud() {
    // K from two wildcard imports ⇒ ambiguous; USING it is loud (not a silent K=20).
    let (out, code) = run("package a; localparam K = 10; endpackage\n\
        package b; localparam K = 20; endpackage\n\
        module t; import a::*; import b::*;\n\
          initial begin $display(\"K=%0d\", K); $finish; end\n\
        endmodule\n");
    assert!(
        out.contains("VITA-E") || code == Some(1),
        "ambiguous import used must be loud: {out} code={code:?}"
    );
    assert!(
        !out.contains("K=20"),
        "must not silently pick last import:\n{out}"
    );
}

#[test]
fn ambiguous_wildcard_import_unused_is_ok() {
    // The same ambiguity, never referenced ⇒ no error (IEEE: ambiguous only at use).
    let (out, _c) = run("package a; localparam K = 10; endpackage\n\
        package b; localparam K = 20; endpackage\n\
        module t; import a::*; import b::*;\n\
          initial begin $display(\"ok\"); $finish; end\n\
        endmodule\n");
    assert!(
        out.contains("ok"),
        "unused ambiguity must not error:\n{out}"
    );
}

#[test]
fn explicit_import_wins_over_wildcard() {
    // An explicit `import a::K` resolves the ambiguity (a's value wins).
    let (out, _c) = run("package a; localparam K = 10; endpackage\n\
        package b; localparam K = 20; endpackage\n\
        module t; import a::K; import b::*;\n\
          initial begin $display(\"K=%0d\", K); $finish; end\n\
        endmodule\n");
    assert!(out.contains("K=10"), "explicit import must win:\n{out}");
}

// Adversarial-found fixes (workflow wmt54fc0r):

#[test]
fn two_state_coerces_x_on_assign() {
    // HIGH: a 2-state var can NEVER hold X (IEEE §6.11.3). Assigning X/Z (directly
    // or from a 4-state source) coerces to 0; $isunknown is always 0. (logic keeps X.)
    let (out, _c) = run("module t; bit b; int i; logic l; logic [7:0] xs;\n\
        initial begin\n\
          b = 1'bx; i = 32'hxxxx_xxxx; l = 1'bx; i = xs;\n\
          $display(\"b=%b i=%0d l=%b unk=%0d eq0=%b\", b, i, l, $isunknown(i), (i==0));\n\
        end endmodule\n");
    assert!(
        out.contains("b=0 i=0 l=x unk=0 eq0=1"),
        "2-state X-coercion:\n{out}"
    );
}

#[test]
fn ambiguous_wildcard_import_of_function_is_loud() {
    // HIGH: the ambiguity rule must cover FUNCTIONS/TASKS, not just consts —
    // two wildcard imports of `f` ⇒ calling it is loud, not a silent first-wins.
    let (out, code) = run(
        "package a; function integer f(); f = 8; endfunction endpackage\n\
        package b; function integer f(); f = 16; endfunction endpackage\n\
        module t; import a::*; import b::*; integer r;\n\
          initial begin r = f(); $display(\"f=%0d\", r); $finish; end\n\
        endmodule\n",
    );
    assert!(
        out.contains("VITA-E") || code == Some(1),
        "ambiguous func import must be loud: {out} code={code:?}"
    );
    assert!(
        !out.contains("f=8") && !out.contains("f=16"),
        "no silent pick:\n{out}"
    );
}

#[test]
fn explicit_function_import_wins_over_wildcard() {
    // Explicit `import b::f` after the wildcards resolves the routine ambiguity.
    let (out, _c) = run(
        "package a; function integer f(); f = 8; endfunction endpackage\n\
        package b; function integer f(); f = 16; endfunction endpackage\n\
        module t; import a::*; import b::*; import b::f; integer r;\n\
          initial begin r = f(); $display(\"f=%0d\", r); $finish; end\n\
        endmodule\n",
    );
    assert!(
        out.contains("f=16"),
        "explicit func import must win:\n{out}"
    );
}
