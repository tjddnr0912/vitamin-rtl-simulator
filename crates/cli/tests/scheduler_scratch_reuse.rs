//! The default backend allocated two fresh `Vec`s per DELTA, and threw away a third
//! buffer's capacity on every continuous-assign fixpoint pass.
//!
//! Neither is a semantic change and neither can be pinned by a wall clock without
//! flaking, so what this file pins is the OUTPUT — byte-for-byte, on the shapes that
//! exercise the two paths hardest — plus the one property a reuse bug would break that
//! a value comparison cannot see: a stale buffer's leftover contents leaking into the
//! next delta.
//!
//! ## What was wrong
//!
//! `native::run::propagate` runs once per delta — measured at 5.5 M deltas on picorv32,
//! 7.0 M on serv, ~15 M on sha256 — and opened with
//!
//! ```text
//!     let mut changed = Vec::new();   k.arena.take_changed(&mut changed);
//!     let mut woken   = Vec::new();
//!     let mut clocked = Vec::new();   k.wake.wake(&changed, &mut woken, &mut clocked);
//! ```
//!
//! The vectors are TINY — median 2 to 8 entries — which is not a reason to leave them
//! alone but the reason they cost so much: each is a `malloc(48)`, one to two `realloc`s
//! as it grows through capacity 4/8/16, and a `free`. Caller attribution of
//! `alloc::raw_vec::finish_grow` put `simulate←propagate` at **74.9% of all `Vec` growth
//! in the process** on sha256, 65.5% on picorv32, 44.2% on serv.
//!
//! ⭐ The interpreter has done the take/restore since its own measurement
//! (`sched/propagate.rs`, with the comment "a fresh Vec pair per call was measurable
//! allocator traffic", backed by named scratch fields on the scheduler). The DEFAULT
//! backend never got it — the reference implementation was in the sibling all along.
//!
//! Separately, `settle_cont_assigns` built its visit list with
//! `let pass = { let mut v = mem::take(&mut ca_dirty); … v };` and then dropped `pass`.
//! That left `ca_dirty` at capacity ZERO, so every later `note_change` push regrew it
//! from scratch — several times per delta. `note_change`'s push line alone measured
//! 6.2% of serv.
//!
//! ## What it bought
//!
//! ```text
//!   serv       9.234 s -> 7.890 s   -14.6%      darkriscv  7.573 -> 7.199  -4.9%
//!   sha256     1.460 s -> 1.317 s    -9.8%      biriscv    4.187 -> 4.088  -2.4%
//!   picorv32   4.847 s -> 4.603 s    -5.0%      aes / keccak / keccak-arr   flat
//! ```
//!
//! every pinned corpus digest unchanged. The two designs it moves most are exactly the
//! two vita was losing to iverilog.
//!
//! ## The traps, because each one silently undoes the fix
//!
//! * `for p in woken { … }` iterates BY VALUE and consumes the vector, so the buffer
//!   never comes back. `drain(..)` yields the same items in the same order and keeps the
//!   capacity.
//! * `propagate`'s early return on an empty changed set is the arm it takes MOST often —
//!   an idle delta — so failing to restore there re-allocates on the next delta and
//!   undoes the whole thing through its own fast path.
//! * `settle_cont_assigns` has two `return`s inside the fixpoint loop; the buffer goes
//!   back before all of them.
//!
//! None of those has a wrong-answer symptom. They are only slower, which is why the
//! guard below is about leftover CONTENTS rather than about timing.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_ssr_{}_{n}", std::process::id()));
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
    (s, out.status.code())
}

/// Run the same source under every backend that ships and require identical output.
/// The VM and interpreter are the project's bisection oracles; a scratch buffer that
/// leaked state would move the default backend away from them.
fn agrees_across_backends(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_ssr_b_{}_{n}", std::process::id()));
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

// ── the delta loop: many deltas, varying changed-set sizes ────────────────

/// ⭐ THE LEFTOVER GUARD. A wide combinational cone driven by a counter makes the
/// changed set grow and shrink from delta to delta, so a buffer that came back holding
/// the previous delta's entries would wake processes that did not change — the output
/// would gain transitions rather than lose them.
///
/// The observable is a per-net RUNNING XOR sampled at every wake, so a spurious wake
/// changes the answer even when it re-delivers a value the reader has already seen.
/// A plain final-value assertion cannot see an extra wake at all.
///
/// ⚠️ NOT a wake COUNT. vita and iverilog disagree on how many times `always @(w)`
/// fires while a combinational cone settles (65 vs 194 on this design) — a
/// pre-existing glitch-visibility difference, identical across all three vita
/// backends, and nothing this change touches. Pinning that number here would make the
/// cell fail for a reason it is not about.
#[test]
fn a_changing_changed_set_size_does_not_leak_between_deltas() {
    let out = agrees_across_backends(
        "`timescale 1ns/1ns\nmodule t;\n  \
         reg [31:0] c; wire [31:0] w; reg [31:0] acc;\n  \
         assign w = c ^ (c << 1) ^ (c >> 1);\n  \
         initial begin c = 0; acc = 0; end\n  \
         always @(w) acc = acc ^ w;\n  \
         initial begin\n    \
         repeat (64) begin #1 c = c + 32'h0101_0101; end\n    \
         #1 $display(\"ACC=%0h W=%0h\", acc, w);\n    $finish;\n  end\nendmodule\n",
    );
    assert!(out.contains("W=e0e0e0e0"), "the cone's final value:\n{out}");
    assert!(out.contains("ACC="), "{out}");
}

/// Deltas where NOTHING changes are the arm `propagate`'s early return takes, and the
/// arm where forgetting to hand the buffer back costs the most. A design that settles
/// and then idles has to produce the same answer as one that does not idle.
#[test]
fn idle_deltas_do_not_disturb_the_result() {
    let a = agrees_across_backends(
        "`timescale 1ns/1ns\nmodule t;\n  reg [7:0] c; wire [7:0] w;\n  \
         assign w = c + 8'd1;\n  \
         initial begin c = 0; #1 c = 5; #20 $display(\"W=%0d\", w); $finish; end\nendmodule\n",
    );
    let b = agrees_across_backends(
        "`timescale 1ns/1ns\nmodule t;\n  reg [7:0] c; wire [7:0] w;\n  \
         assign w = c + 8'd1;\n  \
         initial begin c = 0; #1 c = 5; #1 $display(\"W=%0d\", w); $finish; end\nendmodule\n",
    );
    assert!(a.contains("W=6") && b.contains("W=6"), "{a}\n---\n{b}");
}

// ── the continuous-assign fixpoint: `ca_dirty`'s capacity ─────────────────

/// A chain of continuous assigns takes several fixpoint passes per delta, which is the
/// loop that used to strand `ca_dirty`'s allocation. The visit set is order-sensitive
/// — ascending index is declaration order and several goldens depend on it — so the
/// rebuild has to preserve both the SET and the ORDER, not just the set.
#[test]
fn a_deep_continuous_assign_chain_settles_to_the_same_value() {
    let out = agrees_across_backends(
        "`timescale 1ns/1ns\nmodule t;\n  reg [7:0] a;\n  \
         wire [7:0] b, c, d, e, f, g, h;\n  \
         assign h = g + 8'd1;\n  assign g = f + 8'd1;\n  assign f = e + 8'd1;\n  \
         assign e = d + 8'd1;\n  assign d = c + 8'd1;\n  assign c = b + 8'd1;\n  \
         assign b = a + 8'd1;\n  \
         initial begin a = 0; #1 $display(\"H=%0d\", h); \
         a = 100; #1 $display(\"H=%0d\", h); $finish; end\nendmodule\n",
    );
    assert!(out.contains("H=7"), "seven links from 0:\n{out}");
    assert!(out.contains("H=107"), "and from 100:\n{out}");
}

/// ⚠️ The fixpoint's DELTA-LIMIT exit is one of the two `return`s that must still hand
/// the scratch buffer back, and it is unreachable from any value assertion. This cell
/// EXERCISES it: a genuine combinational oscillation drives `settle_cont_assigns` a
/// million passes until `F4016` fires, so a `return` that forgot to restore is at least
/// executed by the suite rather than never run at all.
///
/// ⚠️ `assign a = ~a;` on a bare wire does NOT oscillate — it settles at x, since `~x`
/// is x — so the design has to close the loop through a definite value.
#[test]
fn the_delta_limit_exit_is_reached_cleanly() {
    let (out, code) = run(
        "`timescale 1ns/1ns\nmodule t;\n  reg [7:0] x; wire [7:0] y;\n  \
         assign y = x + 8'd1;\n  always @(*) x = y;\n  \
         initial begin x = 0; #10 $display(\"UNREACHED\"); $finish; end\nendmodule\n",
    );
    assert!(
        out.contains("F4016") && out.contains("delta limit"),
        "the limit must be reported, not hung:\n{out}"
    );
    assert!(!out.contains("UNREACHED"), "{out}");
    assert_ne!(code, Some(0), "{out}");
}

// ── clocking blocks: the third buffer ─────────────────────────────────────

/// `clocked` is provably free on a design with no clocking blocks (nothing is ever
/// pushed), so this is the cell that actually exercises it. A clocking block samples
/// and commits inside `propagate`, through the buffer the change now reuses.
#[test]
fn a_clocking_block_still_samples_and_commits() {
    let out = run(
        "`timescale 1ns/1ns\nmodule t;\n  logic clk = 0; logic [7:0] d = 8'd1;\n  \
         always #5 clk = ~clk;\n  \
         clocking cb @(posedge clk); input d; endclocking\n  \
         initial begin\n    #7 d = 8'd42;\n    #10 $display(\"CB=%0d\", cb.d);\n    \
         #10 $display(\"CB=%0d\", cb.d);\n    $finish;\n  end\nendmodule\n",
    )
    .0;
    assert!(
        out.contains("CB=42"),
        "the sample must reach the reader:\n{out}"
    );
}
