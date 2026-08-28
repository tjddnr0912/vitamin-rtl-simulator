//! Every frame entry rebuilt its local window from the IR, and every static-slot access
//! walked a `BTreeMap`.
//!
//! Neither is a semantic change and neither can be pinned by a wall clock without
//! flaking, so what this file pins is the one property a reuse bug WOULD break that a
//! timing measurement cannot see: a recycled window handing the next activation the
//! PREVIOUS one's values.
//!
//! ## What was wrong
//!
//! All three frame-entry sites — `run_frame_call_with`, `run_task`, `enter_task_frame` —
//! opened with the same six lines:
//!
//! ```text
//!     let fresh: Vec<Value> = (0..nloc)
//!         .map(|s| { let nv = &self.ir.nets[(base + s) as usize];
//!                    if nv.kind == NetKind::String { Value::from_str_bytes(&[]) }
//!                    else { Value::from_packed(&nv.init, nv.width.max(1), nv.signed) } })
//!         .collect();
//! ```
//!
//! a `malloc` for the `Vec`, `locals_len` `Value` constructions, and a `free` at the pop —
//! **per call** — for a list that is a pure function of the IMMUTABLE IR. `keccak`'s
//! `rotl64` is called millions of times; `aes` has 18 frame bodies. It is now a per-function
//! template built once at init, plus a capacity-capped free-list of retired windows.
//!
//! ⭐ And the `(false, _)` arm — a function with only STATIC locals, which is what a plain
//! non-`automatic` Verilog function is — built `fresh` on every call and handed it to
//! `static_store.entry(func).or_insert(fresh)`, which DROPS it on every call but the first.
//! The whole per-call window cost, paid for a value nobody reads.
//!
//! Separately, the static slab was a `BTreeMap<u32, Vec<Value>>` whose stated reason was
//! determinism — but the key is a dense `FuncId`, so a `Vec` indexed by it is deterministic
//! by construction and costs an index instead of a tree descent. Nothing iterates it and it
//! is never serialized.
//!
//! ## What it bought
//!
//! `keccak` 4.583 s -> 4.257 s (-7.1%), every pinned corpus digest unchanged.
//!
//! ## The traps
//!
//! * ⚠️ A window must be recycled ONLY from a pop that provably DROPS it.
//!   `stash_windows_in` also pops — but it MOVES the window into a `FrameRec` and pushes it
//!   back when the activity resumes. Recycling one of those hands a LIVE window to the next
//!   call. Only the two synchronous `&self` executors' terminal pops retire.
//! * ⚠️ The Case-B fork-in-frame window is moved into the `frame_windows` arena and
//!   outlives the call, so it is never pooled either.
//! * ⚠️ Reuse is `clear` + `extend_from_slice`, not "hand back the same contents". The
//!   `clear` is what makes an unwritten local read as its DEFAULT rather than as the
//!   previous activation's value — the property every cell below is about.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_fwr_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).expect("scratch dir");
    std::fs::write(d.join("t.sv"), src).expect("write design");
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("t.sv")
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    s
}

/// Run under every backend that ships and require identical output. The VM and interpreter
/// are the project's bisection oracles; a window that leaked state would move the default
/// backend away from them.
fn agrees_across_backends(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_fwr_b_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).expect("scratch dir");
    std::fs::write(d.join("t.sv"), src).expect("write design");
    let mut first: Option<String> = None;
    for be in ["native", "vm", "interp"] {
        let out = Command::new(env!("CARGO_BIN_EXE_vita"))
            .args(["t.sv", "--backend", be])
            .current_dir(&d)
            .output()
            .expect("run vita");
        let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
        s.push_str(&String::from_utf8_lossy(&out.stderr));
        match &first {
            None => first = Some(s),
            Some(f) => assert_eq!(f, &s, "backend {be} diverged"),
        }
    }
    let _ = std::fs::remove_dir_all(&d);
    first.unwrap()
}

// ── the leftover guard ────────────────────────────────────────────────────

/// ⭐ THE CELL THIS FILE EXISTS FOR. Call an automatic function once on a path that WRITES
/// its local, then again on a path that does not. A recycled window that skipped the reset
/// returns the first call's value.
///
/// The two answers for an unwritten local are both pinned and they are different, which is
/// the point: an `integer` (4-state) reads X per IEEE §6.4, an `int` (2-state) reads 0.
/// A leftover would show as `10` / `15` in either slot, so both halves refute it.
///
/// ⚠️ ORACLE NOTE. iverilog gives `B=x`; verilator gives `B=0`, because it models the
/// 4-state local as 2-state. iverilog is the oracle here (IEEE §6.4 is explicit), and the
/// disagreement is beside the point of this cell — the two tools AGREE that it is not 10.
/// `C`/`D` (the 2-state pair) match in both.
#[test]
fn an_unwritten_local_does_not_inherit_the_previous_activation() {
    let out = agrees_across_backends(
        "`timescale 1ns/1ns\nmodule t;\n  \
         function automatic integer f4 (input integer a);\n    integer t;\n    \
         if (a > 0) t = a * 2;\n    f4 = t;\n  endfunction\n  \
         function automatic int f2 (input int a);\n    int t;\n    \
         if (a > 0) t = a * 3;\n    f2 = t;\n  endfunction\n  \
         initial begin\n    $display(\"A=%0d B=%0d C=%0d D=%0d E=%0d\", \
         f4(5), f4(-1), f2(5), f2(-1), f4(7));\n    $finish;\n  end\nendmodule\n",
    );
    assert!(
        out.contains("A=10 B=x C=15 D=0 E=14"),
        "an unwritten 4-state local is x and an unwritten 2-state local is 0, \
         on a call that follows one which wrote them:\n{out}"
    );
}

/// The same guard for a `string` slot, whose default is the EMPTY string rather than a
/// width-1 value — a distinct arm of the template, and the one whose earlier bug made
/// `s == ""` false on an unwritten path. `slen(0)` appends nothing, so it can only be 0
/// if the recycled window reset the slot.
///
/// Both oracles: `L=3 0 2`.
#[test]
fn a_recycled_string_slot_starts_empty() {
    let out = agrees_across_backends(
        "`timescale 1ns/1ns\nmodule t;\n  \
         function automatic int slen (input int n);\n    string s;\n    int i;\n    \
         for (i = 0; i < n; i = i + 1) s = {s, \"x\"};\n    slen = s.len();\n  endfunction\n  \
         initial begin\n    $display(\"L=%0d %0d %0d\", slen(3), slen(0), slen(2));\n    \
         $finish;\n  end\nendmodule\n",
    );
    assert!(out.contains("L=3 0 2"), "{out}");
}

// ── the pool must hand out DISTINCT windows ───────────────────────────────

/// Recursion is the shape a pool gets wrong by handing the same buffer to two live
/// activations: the inner call would overwrite the outer's accumulator and the factorial
/// would collapse. Interleaving depths (5, then 1, then 6) also exercises the free-list
/// across a full unwind and a re-descent.
///
/// Both oracles: `F=120 1 720`.
#[test]
fn a_recursive_automatic_function_gets_one_window_per_activation() {
    let out = agrees_across_backends(
        "`timescale 1ns/1ns\nmodule t;\n  \
         function automatic integer fact (input integer k);\n    integer acc;\n    \
         if (k <= 1) acc = 1; else acc = k * fact(k - 1);\n    fact = acc;\n  endfunction\n  \
         initial begin\n    $display(\"F=%0d %0d %0d\", fact(5), fact(1), fact(6));\n    \
         $finish;\n  end\nendmodule\n",
    );
    assert!(out.contains("F=120 1 720"), "{out}");
}

/// ⚠️ A recursion DEEPER than the pool's cap, so the run both overflows the free-list on
/// the way down (the extra windows are simply dropped, the pre-change behaviour) and
/// re-fills it on the way back up. The answer must not depend on which side of the cap a
/// given activation fell.
#[test]
fn a_recursion_deeper_than_the_pool_still_unwinds_correctly() {
    let out = run("`timescale 1ns/1ns\nmodule t;\n  \
         function automatic integer depth (input integer k);\n    integer acc;\n    \
         if (k <= 0) acc = 0; else acc = k + depth(k - 1);\n    depth = acc;\n  endfunction\n  \
         initial begin\n    $display(\"D=%0d %0d\", depth(200), depth(3));\n    \
         $finish;\n  end\nendmodule\n");
    // 200*201/2 = 20100
    assert!(out.contains("D=20100 6"), "{out}");
}

// ── the STATIC slab: persistence, and the arm that no longer builds a window ──

/// A plain (non-`automatic`) function's locals are STATIC: they persist across calls, and
/// the slab must be seeded exactly ONCE. This is the arm that used to build and discard a
/// whole fresh window on every call, and the arm 5b re-indexed off a `BTreeMap`.
///
/// The accumulator is seeded by the first call rather than left at its X-init, because an
/// unseeded `integer` static makes the oracles disagree for a reason unrelated to this
/// change (iverilog `x x x`, verilator `1 3 6` — verilator models the 4-state local as
/// 2-state). Seeded, both give `S=0 1 3 6`.
#[test]
fn a_static_local_persists_across_calls() {
    let out = agrees_across_backends(
        "`timescale 1ns/1ns\nmodule t;\n  \
         function integer counter (input integer inc);\n    integer n;\n    \
         if (inc < 0) n = 0; else n = n + inc;\n    counter = n;\n  endfunction\n  \
         initial begin\n    $display(\"S=%0d %0d %0d %0d\", \
         counter(-1), counter(1), counter(2), counter(3));\n    $finish;\n  end\nendmodule\n",
    );
    assert!(out.contains("S=0 1 3 6"), "{out}");
}

/// ⚠️ BOTH lifetimes in one function — the `(true, true)` arm, which pushes a pooled window
/// AND seeds a static slab. The automatic local must reset per call while the static one
/// must not, so a fix that reset the wrong one shows up here and nowhere else.
///
/// ⚠️ The reachable spelling is the OPPOSITE of the obvious one. `static` inside an
/// `automatic` function does not parse (vita and iverilog both refuse it); what elaborate's
/// `auto_override` actually records is an `automatic` declaration inside a PLAIN function,
/// so that is the shape that reaches the arm.
///
/// ⚠️ NO TOOL ORACLE. iverilog refuses the lifetime override outright ("sorry: Overriding
/// the default variable lifetime is not yet supported"), and verilator answers
/// `R=6 0 2 K=6 0 2` because it models both 4-state locals as 2-state. The pin is
/// hand-IEEE, and vita already gave this answer before the change:
///
/// * `R1 = 6` — `fresh_each` written on the `a != 0` path.
/// * `R2 = x` — the SAME automatic slot on the next call, which does not write it. §6.4:
///   an unwritten 4-state automatic local is X. ⭐ THE GUARD: a recycled window that
///   skipped the reset would answer `6` here, and verilator's `0` refutes that too.
/// * `K1 = x` — `keep` is STATIC and unwritten on its first read, so `keep + 6` is X.
/// * `K2 = 0`, `R3 = 2`, `K3 = 2` — and then it persists across calls (0, then 0 + 2).
#[test]
fn a_function_with_both_lifetimes_resets_only_the_automatic_half() {
    let out = agrees_across_backends(
        "`timescale 1ns/1ns\nmodule t;\n  \
         function integer mixed (input integer a, output integer seen_keep);\n    \
         automatic integer fresh_each;\n    integer keep;\n    \
         if (a == 0) keep = 0;\n    \
         else begin fresh_each = a * 2; keep = keep + fresh_each; end\n    \
         seen_keep = keep;\n    mixed = fresh_each;\n  endfunction\n  \
         integer k1, k2, k3, r1, r2, r3;\n  \
         initial begin\n    r1 = mixed(3, k1);\n    r2 = mixed(0, k2);\n    \
         r3 = mixed(1, k3);\n    \
         $display(\"R=%0d %0d %0d K=%0d %0d %0d\", r1, r2, r3, k1, k2, k3);\n    \
         $finish;\n  end\nendmodule\n",
    );
    assert!(out.contains("R=6 x 2 K=x 0 2"), "{out}");
}

// ── the windows that must NOT be pooled ───────────────────────────────────

/// ⚠️ A task with a `#delay` SUSPENDS, so its window is stashed into a `FrameRec` by
/// `stash_windows_in` and pushed back on resume — it never reaches a terminal pop. Two
/// concurrent activations of the same task interleave across the delay, so if either
/// window had been recycled the second activation would read the first's argument.
///
/// Both oracles: each activation reports its OWN tag.
#[test]
fn a_suspended_task_window_is_not_recycled_under_it() {
    let out = agrees_across_backends(
        "`timescale 1ns/1ns\nmodule t;\n  \
         task automatic slow (input integer tag);\n    integer mine;\n    \
         mine = tag * 10;\n    #5;\n    $display(\"T=%0d\", mine);\n  endtask\n  \
         initial slow(1);\n  initial begin #1 slow(2); end\n  \
         initial begin #20 $finish; end\nendmodule\n",
    );
    assert!(
        out.contains("T=10"),
        "the first activation's own local:\n{out}"
    );
    assert!(
        out.contains("T=20"),
        "the second activation's own local:\n{out}"
    );
}

/// ⚠️ A `fork … join` inside a task whose arms touch the parent's automatic locals is the
/// Case-B path: the window is MOVED into the `frame_windows` arena and referenced by handle
/// by the parked parent and every running arm, so it outlives the call and is never pooled.
#[test]
fn a_fork_in_frame_shared_window_is_not_recycled() {
    let out = run("`timescale 1ns/1ns\nmodule t;\n  \
         task automatic par (input integer a);\n    integer x, y;\n    \
         fork\n      begin #2 x = a + 1; end\n      begin #1 y = a + 2; end\n    join\n    \
         $display(\"P=%0d %0d\", x, y);\n  endtask\n  \
         initial begin par(10); par(20); $finish; end\nendmodule\n");
    assert!(out.contains("P=11 12"), "{out}");
    assert!(out.contains("P=21 22"), "{out}");
}
