//! Block-locals inside an `interface` body — FULL branch parity with the module path.
//!
//! v1 flattens a block-local (`begin … int x; … end`) to a scope-level net, created in
//! the Nets phase so references inside the block resolve. `instance.rs` has done that
//! for a MODULE body since v1; `iface_inst.rs` had no such loop, so every block-local
//! in an interface resolved to nothing. That is much wider than the spelling suggests,
//! because the PARSER synthesizes a block-local pair (`__foreach_<i>_<n>`,
//! `__foreach_st_<n>`) for every `foreach` — so `foreach` over any unpacked array in an
//! interface was loud while the identical body in a module was correct.
//!
//! Parity is in two parts and they landed in two slices. The hoist alone is NOT the
//! module twin: the module path first builds five classifier maps from an
//! `&ast::ModuleDecl`, and those are what keep a block-local that COLLIDES with a
//! scope-level name from coalescing onto it. The first slice shipped only the
//! parser-SYNTHESIZED names, whose uniqueness is structural, and refused the rest.
//!
//! ⚠️ That refusal turned out not to be the thing keeping the collision safe. The gate
//! blocked NET CREATION only, and a colliding name needs no net created — so
//! `interface ifc; integer b; … begin integer b; b = 7; end` was ALREADY silent-wrong
//! at exit 0 (`OUTER b=7`, both oracles 99) whenever the body had no `foreach` to make
//! it loud. The second slice computes all five maps from the INTERFACE decl and holds
//! them across both the Nets pass and the Logic loop, which closes the silent-wrong and
//! the refusal together. No signature work was needed: `hdl_ast::Item::Interface` holds
//! an `ast::ModuleDecl`, so every one of those passes already accepted an interface body.
//!
//! Measured on iverilog 13.0 and verilator 5.052.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_ibl_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (s, out.status.code())
}

/// The headline cell and its CONTROL TWIN in one design: the same `foreach` body was
/// correct in a `module` and emitted nine errors in an `interface` — seven
/// `undeclared net/variable top.u.__foreach_i_<n>` / `__foreach_st_<n>` and two
/// `enum method 'v.first' is unavailable` on a design that contains no enum.
/// Both oracles print all six lines.
#[test]
fn a_foreach_in_an_interface_matches_its_module_twin() {
    let (out, code) = run("interface ifc;\n  int v [0:2];\n  initial begin\n\
        \x20   v[0]=1; v[1]=2; v[2]=3;\n\
        \x20   foreach (v[i]) $display(\"IF v[%0d]=%0d\", i, v[i]);\n\
        \x20 end\nendinterface\n\
         module mod;\n  int w [0:2];\n  initial begin\n\
        \x20   w[0]=1; w[1]=2; w[2]=3;\n\
        \x20   foreach (w[i]) $display(\"MOD w[%0d]=%0d\", i, w[i]);\n\
        \x20 end\nendmodule\n\
         module top;\n  ifc u();\n  mod m();\n\
        \x20 initial begin #10; $finish; end\nendmodule\n");
    assert_eq!(code, Some(0), "the interface twin must elaborate:\n{out}");
    for i in 0..3 {
        for want in [
            format!("IF v[{i}]={}", i + 1),
            format!("MOD w[{i}]={}", i + 1),
        ] {
            assert!(
                out.lines().any(|l| l == want),
                "missing `{want}` (both oracles print it):\n{out}"
            );
        }
    }
}

/// Every `foreach` SHAPE in an interface body: single index, multi-dimension
/// (`q[i,j]`) and a nested pair whose synthesized names must not collide with each
/// other. All three tools print these fourteen lines identically.
#[test]
fn every_foreach_shape_in_an_interface_body_resolves() {
    let (out, code) = run(
        "interface ifc;\n  int v [0:2];\n  int q [0:1][0:1];\n  initial begin\n\
        \x20   v[0]=1; v[1]=2; v[2]=3;\n\
        \x20   foreach (v[i]) $display(\"C v[%0d]=%0d\", i, v[i]);\n\
        \x20   foreach (q[i,j]) $display(\"D q[%0d][%0d]\", i, j);\n\
        \x20   foreach (v[i]) foreach (q[a,b]) $display(\"N %0d %0d %0d\", i, a, b);\n\
        \x20 end\nendinterface\n\
         module top;\n  ifc u();\n  initial begin #10; $finish; end\nendmodule\n",
    );
    assert_eq!(code, Some(0), "{out}");
    for want in [
        "C v[0]=1",
        "C v[1]=2",
        "C v[2]=3",
        "D q[0][0]",
        "D q[0][1]",
        "D q[1][0]",
        "D q[1][1]",
        "N 0 0 0",
        "N 1 1 1",
        "N 2 1 0",
    ] {
        assert!(
            out.lines().any(|l| l == want),
            "missing `{want}` (both oracles print it):\n{out}"
        );
    }
}

/// THE LINE, now crossed — and the reason the old refusal existed is the reason this
/// test asserts VALUES rather than an exit code.
///
/// v1 flattens a block-local to a scope-level net by BARE NAME, and the classifier that
/// keeps a COLLIDING one out of that flattening (`compute_scoped_block_locals` and its
/// four siblings) is built from an `&ast::ModuleDecl`. On the interface path those maps
/// used to describe the PARENT module, so the interface's own members were invisible to
/// them and this cell printed `OUTER b=7` / `u.b = 7` where both oracles print 99 —
/// while the MODULE twin of the identical text was correct
/// (`the_module_twin_of_the_collision_is_correct`, which is what attributes the defect
/// to the interface path rather than to the flatten model).
///
/// ⚠️⚠️ The refusal that stood here was NOT what made this cell safe, and that is the
/// trap this test used to sit in. The gate only blocked NET CREATION; when the name
/// already exists at interface scope no net needs creating, so the write landed on the
/// member anyway. This design exits non-zero in PRE only because of the CO-LOCATED
/// `foreach` — delete that one line from the PRE source and the identical collision
/// exits 0 printing `OUTER b=7`. The shape was already SILENT-WRONG; an exit-code
/// assertion could not see it, which is why the replacement pins the numbers.
///
/// `interface ifc; integer b; initial b = 99; initial begin #1; begin integer b; b = 7;
/// end end` — the version with no `foreach` at all — is the minimal proof, measured
/// PRE `OUTER b=7 / HIER b=7` vs POST and both oracles `99 / 99`.
///
/// The fix is those five passes computed from the INTERFACE decl and held across BOTH
/// the Nets pass and the Logic loop (`iface_inst.rs`). No signature work was needed:
/// `hdl_ast::Item::Interface` holds an `ast::ModuleDecl`.
#[test]
fn a_user_written_block_local_in_an_interface_binds_to_its_own_net() {
    // The collision cell itself. Both oracles print exactly these six lines.
    let (out, code) = run(
        "interface ifc;\n  int v [0:2];\n  integer b;\n  initial begin\n\
        \x20   v[0]=1; v[1]=2; v[2]=3;\n  b = 99;\n\
        \x20   begin integer b; b = 7; $display(\"INNER b=%0d\", b); end\n\
        \x20   $display(\"OUTER b=%0d\", b);\n\
        \x20   foreach (v[i]) $display(\"V[%0d]=%0d\", i, v[i]);\n\
        \x20 end\nendinterface\n\
         module top;\n  ifc u();\n\
        \x20 initial begin #10; $display(\"HIER b=%0d\", u.b); $finish; end\nendmodule\n",
    );
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("INNER b=7"), "{out}");
    // The two that were silently 7 before: the member survives the block-local.
    assert!(out.contains("OUTER b=99"), "silent-wrong: {out}");
    assert!(out.contains("HIER b=99"), "silent-wrong: {out}");
    assert!(
        out.contains("V[0]=1") && out.contains("V[1]=2") && out.contains("V[2]=3"),
        "the co-located foreach regressed: {out}"
    );
    // And the NON-colliding user block-local, which the all-or-nothing rule refused
    // together with its `foreach`. iverilog prints the same four lines (`v` is never
    // written in this one, so the elements are 0).
    let (out, code) = run("interface ifc;\n  int v [0:2];\n  initial begin\n\
        \x20   begin integer c; c = 1; $display(\"C c=%0d\", c); end\n\
        \x20   foreach (v[i]) $display(\"V[%0d]=%0d\", i, v[i]);\n\
        \x20 end\nendinterface\n\
         module top;\n  ifc u();\n  initial begin #10; $finish; end\nendmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("C c=1"), "{out}");
    assert!(
        out.contains("V[0]=0") && out.contains("V[1]=0") && out.contains("V[2]=0"),
        "{out}"
    );
}

/// The minimal discriminator, with NO `foreach` anywhere — the design that separates
/// "the gate refused this" from "the gate never applied here".
///
/// A probe that writes the member AFTER the block-local cannot tell the two rules apart
/// (both end at 99); this one writes the member FIRST, at time 0, and reads it after the
/// block-local has run. PRE: `OUTER b=7 / HIER b=7` at exit 0. Both oracles and POST: 99.
#[test]
fn a_colliding_interface_block_local_does_not_write_the_member() {
    let (out, code) = run("interface ifc;\n  integer b;\n  initial b = 99;\n\
        \x20 initial begin\n    #1;\n\
        \x20   begin integer b; b = 7; $display(\"INNER b=%0d\", b); end\n\
        \x20 end\n  initial begin #2 $display(\"OUTER b=%0d\", b); end\nendinterface\n\
         module top;\n  ifc u();\n\
        \x20 initial #3 $display(\"HIER b=%0d\", u.b);\n\
        \x20 initial #10 $finish;\nendmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("INNER b=7"), "{out}");
    assert!(out.contains("OUTER b=99"), "silent-wrong: {out}");
    assert!(out.contains("HIER b=99"), "silent-wrong: {out}");
}

/// The shapes the all-or-nothing refusal took down with it, in ONE design and across TWO
/// instances — a named block, an `always`, a `for` body, two sibling blocks reusing one
/// name, and a `foreach` beside all of them. PRE emitted 14 `E3010`s and refused;
/// POST is byte-identical to both oracles.
///
/// Two instances matter: a block-local flattens per SCOPE, so one shared net across
/// `u1`/`u2` would show here and nowhere else.
#[test]
fn every_user_block_local_shape_in_an_interface_body_resolves_per_instance() {
    let (out, code) = run("interface ifc;\n  int arr[0:2];\n  integer acc;\n\
        \x20 initial begin int x; x = 3; $display(\"P1 x=%0d\", x); end\n\
        \x20 initial begin : nm integer t; t = 11; $display(\"Dn t=%0d\", t); end\n\
        \x20 always @(*) begin integer q; q = 5; end\n\
        \x20 initial begin\n    integer s;\n\
        \x20   for (int k = 0; k < 3; k = k + 1) begin integer inner; inner = k * 2; s = inner; end\n\
        \x20   $display(\"H s=%0d\", s);\n  end\n\
        \x20 initial begin acc = 0; foreach (arr[i]) acc = acc + i; $display(\"FOREACH acc=%0d\", acc); end\n\
        \x20 initial begin integer c; c = 1; $display(\"B1 c=%0d\", c); end\n\
        \x20 initial begin integer c; c = 2; $display(\"B2 c=%0d\", c); end\n\
         endinterface\n\
         module top;\n  ifc u1(); ifc u2();\n  initial #10 $finish;\nendmodule\n");
    assert_eq!(code, Some(0), "{out}");
    for (line, want) in [
        ("P1 x=3", 2),
        ("Dn t=11", 2),
        ("H s=4", 2),
        ("FOREACH acc=3", 2),
        ("B1 c=1", 2),
        ("B2 c=2", 2),
    ] {
        assert_eq!(
            out.lines().filter(|l| l.trim() == line).count(),
            want,
            "`{line}` should appear once per instance:\n{out}"
        );
    }
}

/// The MODULE twin of the collision cell, which is what ATTRIBUTES the defect above to
/// the interface path rather than to the flatten model: correct in PRE and in POST.
#[test]
fn the_module_twin_of_the_collision_is_correct() {
    let (out, code) = run(
        "module mod;\n  int v [0:2];\n  integer b;\n  initial begin\n\
        \x20   v[0]=1; v[1]=2; v[2]=3;\n  b = 99;\n\
        \x20   begin integer b; b = 7; $display(\"INNER b=%0d\", b); end\n\
        \x20   $display(\"OUTER b=%0d\", b);\n\
        \x20 end\nendmodule\n\
         module top;\n  mod u();\n\
        \x20 initial begin #10; $display(\"HIER b=%0d\", u.b); $finish; end\nendmodule\n",
    );
    assert_eq!(code, Some(0), "{out}");
    for want in ["INNER b=7", "OUTER b=99", "HIER b=99"] {
        assert!(
            out.lines().any(|l| l == want),
            "missing `{want}` (both oracles):\n{out}"
        );
    }
}

/// ⚠️ The neighbouring loud that this slice does NOT lift, pinned so the next reader
/// does not read the fixed hoist as covering it: a task or function declared inside an
/// interface is still honest-loud by design. Both oracles run it; ROADMAP §3 owns it.
#[test]
fn a_task_in_an_interface_is_still_loud() {
    let (out, code) = run("interface ifc;\n\
        \x20 task automatic tk(); begin int t = 9; $display(\"F t=%0d\", t); end endtask\n\
        \x20 initial #1 tk();\nendinterface\n\
         module top;\n  ifc u();\n  initial begin #10; $finish; end\nendmodule\n");
    assert_ne!(code, Some(0), "still refused:\n{out}");
    assert!(
        out.contains("functions/tasks inside an interface are outside the MVP"),
        "{out}"
    );
}

/// Review round 1, differential lens, BLOCKING: admitting the interface body to the hoist
/// without also running the module path's CONTAINMENT gate was loud→silent-wrong.
///
/// `instance.rs` runs `check_block_local_scope_leaks` over every process body before it
/// hoists; the first cut of the interface parity installed the five maps and not that call.
/// A NESTED same-name shadow then silently read the INNER `x`:
///   PRE  `error[VITA-E3010]` ×8, refused
///   POST without the gate: `o1=02 o2=02` at exit 0, both oracles `o1=02 o2=01`
///   MODULE twin in POST: still `E3009 … referenced outside its begin…end block`
/// The twin is what proved the guard existed and simply was not reached, rather than the
/// flatten model being different for an interface.
///
/// The pin is the DIAGNOSTIC, not a value: v1 keeps a body's block-locals in a flat table,
/// so the outer read is refused on both paths until per-block scope lowering lands. What
/// this test forbids is the two paths ANSWERING DIFFERENTLY.
#[test]
fn a_nested_block_local_shadow_is_refused_in_an_interface_as_in_a_module() {
    let body = "  logic [7:0] o1, o2;\n  initial begin : outer\n    int x;\n    x = 1;\n\
        \x20   begin : inner int x; x = 2; o1 = x[7:0]; end\n    #1 o2 = x[7:0];\n  end\n";
    let (iface_out, iface_code) = run(&format!(
        "interface ifb;\n{body}endinterface\n\
         module top;\n  ifb a1();\n  initial #10 $finish;\nendmodule\n"
    ));
    let (mod_out, mod_code) = run(&format!(
        "module mtb;\n{body}endmodule\n\
         module top;\n  mtb b1();\n  initial #10 $finish;\nendmodule\n"
    ));
    // Both refuse, with the SAME diagnostic — that is the parity under test.
    assert_ne!(
        iface_code,
        Some(0),
        "interface silently bound it:\n{iface_out}"
    );
    assert_ne!(mod_code, Some(0), "{mod_out}");
    for (what, out) in [("interface", &iface_out), ("module", &mod_out)] {
        assert!(
            out.contains("block-local `x` is referenced outside its `begin…end` block"),
            "{what} lost the containment diagnostic:\n{out}"
        );
    }
    // And the silent value the gate exists to prevent must be absent.
    assert!(!iface_out.contains("o2=02"), "silent-wrong:\n{iface_out}");
}
