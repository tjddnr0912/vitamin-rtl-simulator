//! `foreach` inside an `interface` body — §3, PARTIAL branch parity.
//!
//! v1 flattens a block-local (`begin … int x; … end`) to a scope-level net, created in
//! the Nets phase so references inside the block resolve. `instance.rs` has done that
//! for a MODULE body since v1; `iface_inst.rs` had no such loop, so every block-local
//! in an interface resolved to nothing. That is much wider than the spelling suggests,
//! because the PARSER synthesizes a block-local pair (`__foreach_<i>_<n>`,
//! `__foreach_st_<n>`) for every `foreach` — so `foreach` over any unpacked array in an
//! interface was loud while the identical body in a module was correct.
//!
//! ⚠️ Only the SYNTHESIZED half is fixed. The hoist alone is not the module twin: the
//! module path first builds five classifier maps from a `&ast::ModuleDecl`, and those
//! are what keep a block-local that COLLIDES with a scope-level name from coalescing
//! onto it. On the interface path they describe the parent module, so a user-written
//! block-local was measured going loud→silent-wrong and is refused. See
//! `a_user_written_block_local_in_an_interface_is_still_loud`.
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

/// ⚠️⚠️ THE LINE THIS SLICE DELIBERATELY DOES NOT CROSS, and the measurement that put
/// it there. A USER-written block-local in an interface stays refused, because v1
/// flattens a block-local to a scope-level net by BARE NAME and the classifier that
/// keeps a COLLIDING one out of that flattening (`compute_scoped_block_locals` and its
/// four siblings) is built from a `&ast::ModuleDecl` — on this path, the PARENT
/// module's. With the hoist ungated, `interface ifc; integer b; … begin integer b;
/// b = 7; end` printed `OUTER b=7` and `u.b = 7` where BOTH oracles print 99, while the
/// MODULE twin of the identical text is correct in PRE and POST. Refusing is the
/// accuracy ladder; the prerequisite is those five passes taught to run over an
/// interface body and held across both the Nets and the Logic pass. ROADMAP §3 owns it.
///
/// The refusal is ALL-OR-NOTHING per body on purpose: one user block-local anywhere
/// re-refuses the `foreach`es too. A per-block filter would have to redo the
/// containment/disjointness analysis those passes exist for.
#[test]
fn a_user_written_block_local_in_an_interface_is_still_loud() {
    // The collision cell itself.
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
    assert_ne!(
        code,
        Some(0),
        "must not bind a colliding block-local:\n{out}"
    );
    assert!(
        !out.contains("OUTER b=7"),
        "silent-wrong reappeared:\n{out}"
    );
    // And a NON-colliding user block-local, which is refused by the same all-or-nothing
    // rule — recorded so the next reader knows the refusal is broader than the hazard.
    let (out, code) = run("interface ifc;\n  int v [0:2];\n  initial begin\n\
        \x20   begin integer c; c = 1; $display(\"C c=%0d\", c); end\n\
        \x20   foreach (v[i]) $display(\"V[%0d]=%0d\", i, v[i]);\n\
        \x20 end\nendinterface\n\
         module top;\n  ifc u();\n  initial begin #10; $finish; end\nendmodule\n");
    assert_ne!(code, Some(0), "{out}");
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
