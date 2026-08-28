//! A block-local declared with the same name as a module-scope net, port or param
//! is a SHADOW. vita aliased it onto the shadowed net and answered with that net's
//! width, signedness, type and lifetime — at exit 0, in 22 measured shapes.
//!
//! ## The root was one early `return`
//!
//! `block_local/hoist.rs` guarded the flatten with a width/signedness check, a
//! definite-assignment check and `elaborate_netvar_decl` itself. All three sat
//! behind this:
//!
//! ```text
//!     if self.local_decl_names.contains(nm) {
//!         return;      // returns from the WHOLE function
//!     }
//! ```
//!
//! whose comment claimed the case was "handled by the struct/enum/typedef
//! shadow-scoping". It was not. So the moment a block-local's name matched a
//! module-scope name, every guard was disabled at once, no declaration was created,
//! and every reference resolved to the module net.
//!
//! ⭐ One token separates the two behaviours: rename the MODULE net and the same
//! design is loud. `int zzz; begin int x; ... end` hits the guards; `int x; begin
//! int x; ... end` is silently aliased. That is the shape of a guard reached
//! through the wrong door, not of a missing rule.
//!
//! ## What it cost, measured against BOTH oracles
//!
//! ```text
//!   logic[15:0] over logic[7:0]   val=ef   bits=8    both oracles: beef / 16
//!   real over int                 3.000000           both: 2.500000
//!   int signed over int unsigned  4294967293         both: -3
//!   enum over logic[1:0]          x=1 name=RED       both: x=5 name=GRN
//!   int x[0:3] over logic[7:0]    arr 1 1            both: arr 5 9
//!   read before write             55 (net leftover)  both: 0
//!   write in the block            module x=99        both: module x=0 / 5
//! ```
//!
//! The last row is the one that escapes the module: the shadowed net is observable
//! by a hierarchical reference from another module and by `$dumpvars`, so a block
//! that only ever names its own local was rewriting the parent's state.
//!
//! ## The fix reuses machinery that was already in the tree
//!
//! The function/task path (`$func$`) and the generate path (`t.g.`) both give a
//! local a distinct key and were already correct. A shadow now earns a `$blk$<lo>`
//! scope the same way `automatic` and dynamic-storage locals have since §4.5.249 —
//! `gather_auto_block_locals` marks it, `compute_scoped_block_locals` admits it on
//! ONE declaring block (a shadow has nothing to coalesce with), and the hoist
//! follows the marks. `walk_scopes_key` treats `$blk$` as transparent, so every
//! OTHER name in the block still falls through to the enclosing module net; only
//! the shadowed one is captured.
//!
//! ⚠️ This is loud → support as well as silent → correct: a shadowing `string`, a
//! shadowed `wire` (which reported E3018, blaming the user for a net assignment
//! they had not written) and a shadowing unpacked array of a different size (which
//! reported E4002 twice at runtime) all run now.
//!
//! A reference from OUTSIDE the declaring block is closed by the same move, in
//! `check_block_local_scope_leaks`: that gate exists because the flat table would
//! answer such a reference with the block-local, and a scoped name is not in the flat
//! table. It is keyed on `scoped_block_locals`, which covers module process bodies
//! only, so a FRAME body keeps both its flat table and its diagnostic.
//!
//! ⚠️ NOT closed here, and still loud: a sibling coalesce read after a suspension
//! point (a non-shadowing shape, so nothing here reaches it), a block-local shadowing
//! another BLOCK-local rather than a module name, and `static int x;` as a
//! block-local, which the parser does not accept at all.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_blms_{}_{n}", std::process::id()));
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

/// Every expectation below is iverilog's, measured; verilator agrees on all of them.
fn expect(src: &str, want: &[&str]) {
    let (o, code) = run(src);
    assert_eq!(code, Some(0), "expected a clean run:\n{o}");
    for w in want {
        assert!(o.contains(w), "expected {w:?} in:\n{o}");
    }
}

// ── the local keeps its OWN type, not the shadowed net's ───────────────────

/// A wider local over a narrower net. The value was truncated to the net's width
/// and `$bits` reported the net's width too.
#[test]
fn a_wider_local_over_a_narrower_net_keeps_its_own_width() {
    expect(
        "`timescale 1ns/1ns\nmodule t;\n  logic [7:0] x;\n  \
         initial begin x = 8'h11; #5; end\n  \
         initial begin #1; begin : blk\n    logic [15:0] x;\n    x = 16'hBEEF;\n    \
         $display(\"R val=%0h bits=%0d\", x, $bits(x));\n  end end\nendmodule\n",
        &["R val=beef bits=16"],
    );
}

/// A `real` local over an `int` net. The net has no fractional part, so the value
/// was rounded — a silent 2.5 → 3.0.
#[test]
fn a_real_local_over_an_int_net_keeps_its_fraction() {
    expect(
        "`timescale 1ns/1ns\nmodule t;\n  int x;\n  \
         initial begin #1; begin : b\n    real x; x = 2.5;\n    \
         $display(\"R x=%0f\", x);\n  end end\nendmodule\n",
        &["R x=2.500000"],
    );
}

/// ⚠️ Signedness, not just width. `int signed` over `int unsigned` read back as
/// 4294967293 and compared `x < 0` FALSE — a wrong branch, not just a wrong print.
#[test]
fn a_signed_local_over_an_unsigned_net_keeps_its_sign() {
    expect(
        "`timescale 1ns/1ns\nmodule t;\n  int unsigned x;\n  \
         initial begin #1; begin : blk\n    int signed x; x = -3;\n    \
         $display(\"R x=%0d cmp=%0d\", x, (x < 0));\n  end end\n  \
         initial begin #2; $display(\"MOD x=%0d\", x); end\nendmodule\n",
        &["R x=-3 cmp=1", "MOD x=0"],
    );
}

/// An `enum` local over a plain vector net. The label did not fit the net's width,
/// so `.name()` returned a DIFFERENT label — the method lied rather than failing.
#[test]
fn an_enum_local_over_a_vector_net_keeps_its_labels() {
    expect(
        "`timescale 1ns/1ns\nmodule t;\n  \
         typedef enum logic [2:0] { RED=1, GRN=5 } e_t;\n  logic [1:0] x;\n  \
         initial begin #1; begin : b\n    e_t x; x = GRN;\n    \
         $display(\"R x=%0d name=%s bits=%0d\", x, x.name(), $bits(x));\n  \
         end end\nendmodule\n",
        &["R x=5 name=GRN bits=3"],
    );
}

/// An unpacked ARRAY local over a scalar net collapsed onto one word, so every
/// element read the same storage.
#[test]
fn an_unpacked_array_local_over_a_scalar_net_keeps_its_elements() {
    expect(
        "`timescale 1ns/1ns\nmodule t;\n  logic [7:0] x;\n  \
         initial begin #1; begin : b\n    int x [0:3];\n    x[0]=5; x[3]=9;\n    \
         $display(\"R %0d %0d\", x[0], x[3]);\n  end end\nendmodule\n",
        &["R 5 9"],
    );
}

// ── the local has its own LIFETIME, so nothing leaks in either direction ───

/// Reading before writing must see a fresh variable's default, not whatever the
/// shadowed net happens to hold.
#[test]
fn a_local_read_before_its_first_write_does_not_see_the_nets_value() {
    expect(
        "`timescale 1ns/1ns\nmodule t;\n  int x;\n  \
         initial begin x = 55; #3; end\n  \
         initial begin #1; begin : b\n    int x;\n    \
         $display(\"R x=%0d\", x);\n  end end\nendmodule\n",
        &["R x=0"],
    );
}

/// ⚠️ The escaping direction: a write inside the block used to land on the module
/// net, where the rest of the design could read it.
#[test]
fn a_write_to_the_local_does_not_reach_the_module_net() {
    expect(
        "`timescale 1ns/1ns\nmodule t;\n  int x = 5;\n  \
         initial begin begin : blk\n    int x; x = 99;\n    \
         $display(\"R inner=%0d\", x);\n  end end\n  \
         initial begin #1; $display(\"MOD x=%0d\", x); end\nendmodule\n",
        &["R inner=99", "MOD x=5"],
    );
}

/// Two sibling blocks each declaring the name: neither may see the other's value,
/// and neither may reach the module net.
#[test]
fn sibling_blocks_do_not_share_the_shadowed_net() {
    expect(
        "`timescale 1ns/1ns\nmodule t;\n  int x;\n  \
         initial begin\n    #1;\n    \
         begin : b1 int x; x = 10; $display(\"R b1=%0d\", x); end\n    \
         begin : b2 int x; $display(\"R b2=%0d\", x); x = 20; end\n  end\n  \
         initial begin #5; $display(\"MOD x=%0d\", x); end\nendmodule\n",
        &["R b1=10", "R b2=0", "MOD x=0"],
    );
}

/// ⭐ Two PROCESSES interleaving in time over the same name. The second block's
/// write used to be visible to the first when it resumed, which no amount of
/// single-process reasoning would have caught.
#[test]
fn two_processes_interleaving_do_not_share_the_shadowed_net() {
    expect(
        "`timescale 1ns/1ns\nmodule t;\n  int x;\n  \
         initial begin #1; begin : b1\n    int x; x = 111;\n    #4;\n    \
         $display(\"R b1again=%0d\", x);\n  end end\n  \
         initial begin #2; begin : b2\n    int x; x = 222;\n  end end\nendmodule\n",
        &["R b1again=111"],
    );
}

// ── every declaring context, because each reaches the hoist differently ────

/// A module INPUT PORT is in the same name set as a net, and was shadowed the
/// same way — with the port's width.
#[test]
fn a_local_shadowing_a_module_port_keeps_its_own_type() {
    expect(
        "`timescale 1ns/1ns\nmodule sub(input logic [3:0] x);\n  \
         initial begin #1; begin : b\n    int x; x = 4321;\n    \
         $display(\"R x=%0d bits=%0d\", x, $bits(x));\n  end end\nendmodule\n\
         module t; logic [3:0] p = 4'h6; sub u(.x(p)); endmodule\n",
        &["R x=4321 bits=32"],
    );
}

/// `always_comb` has its own lowering path, and its block-local was clobbering the
/// module net on every settle.
#[test]
fn an_always_comb_block_local_does_not_clobber_the_module_net() {
    expect(
        "`timescale 1ns/1ns\nmodule t;\n  logic [3:0] x;\n  \
         initial begin x = 4'h1; #3; end\n  \
         always_comb begin : b\n    int x;\n    x = 999;\n  end\n  \
         initial begin #2; $display(\"MOD x=%0h\", x); end\nendmodule\n",
        &["MOD x=1"],
    );
}

/// A `fork` arm's block. `gather_auto_block_locals` recurses through `Fork`, so an
/// arm's own `begin…end` is reached — but that is exactly the kind of path a fix
/// landing in one walk misses, so it gets its own cell.
#[test]
fn a_fork_arm_block_local_keeps_its_own_type() {
    expect(
        "`timescale 1ns/1ns\nmodule t;\n  logic [3:0] x;\n  \
         initial begin\n    #1;\n    fork\n      \
         begin : a1 int x; x = 1234; $display(\"R x=%0d bits=%0d\", x, $bits(x)); end\n    \
         join\n  end\nendmodule\n",
        &["R x=1234 bits=32"],
    );
}

/// ⭐ The leak was observable from ANOTHER MODULE by hierarchical reference, which
/// is what makes this a whole-design defect rather than a local one.
#[test]
fn the_leak_is_not_visible_through_a_hierarchical_reference() {
    expect(
        "`timescale 1ns/1ns\nmodule sub;\n  int x;\n  \
         initial begin #1; begin : b\n    int x; x = 4242;\n    \
         $display(\"R inner=%0d\", x);\n  end end\nendmodule\n\
         module t;\n  sub u();\n  \
         initial begin #5; $display(\"R hier=%0d\", u.x); end\nendmodule\n",
        &["R inner=4242", "R hier=0"],
    );
}

// ── shapes that used to be LOUD and now run ────────────────────────────────

/// A `string` local over an `int` net was refused ("the dynamic-storage local …").
#[test]
fn a_string_local_over_an_int_net_now_runs() {
    expect(
        "`timescale 1ns/1ns\nmodule t;\n  int x;\n  \
         initial begin #1; begin : b\n    string x; x = \"hello\";\n    \
         $display(\"R x=%s len=%0d\", x, x.len());\n  end end\nendmodule\n",
        &["R x=hello len=5"],
    );
}

/// ⚠️ A shadowed `wire` produced E3018 — "procedural assignment to net `t.x`
/// (declare it reg/logic)" — blaming the user for an assignment to a net they had
/// not written. The message was accurate about the net and wrong about the design.
#[test]
fn a_local_shadowing_a_wire_now_runs_instead_of_blaming_the_net() {
    let (o, code) = run("`timescale 1ns/1ns\nmodule t;\n  wire x;\n  \
         initial begin #1; begin : b\n    int x; x = 888;\n    \
         $display(\"R x=%0d bits=%0d\", x, $bits(x));\n  end end\nendmodule\n");
    assert_eq!(code, Some(0), "{o}");
    assert!(o.contains("R x=888 bits=32"), "{o}");
    assert!(
        !o.contains("E3018"),
        "no procedural-assignment-to-net claim:\n{o}"
    );
}

/// A local unpacked array LARGER than the shadowed module array reported E4002
/// twice at runtime and exited 1, because the element writes went to the module
/// array's storage.
#[test]
fn a_larger_local_array_over_a_smaller_module_array_now_runs() {
    let (o, code) = run("`timescale 1ns/1ns\nmodule t;\n  int x [0:1];\n  \
         initial begin #1; begin : b\n    int x [0:3];\n    x[0]=5; x[2]=7;\n    \
         $display(\"R %0d %0d %0d\", x[0], x[2], $size(x));\n  end end\nendmodule\n");
    assert_eq!(code, Some(0), "{o}");
    assert!(o.contains("R 5 7 4"), "{o}");
    assert!(!o.contains("E4002"), "no out-of-range report:\n{o}");
}

// ── the waveform ──────────────────────────────────────────────────────────

/// ⚠️ THE THIRD OBSERVABLE. The module net used to CHANGE VALUE in the dump when
/// only the local was written, and the local had no var of its own. iverilog emits
/// both, in a nested scope; so does vita now.
///
/// The scope NAME still differs — vita spells it `$blk$<span.lo>` where iverilog
/// uses the block's label `b`. That is the pre-existing `$blk$` convention (it has
/// named every `automatic` local's scope since §4.5.249) and not something this
/// change introduced, but the change makes it visible on far more designs. Recorded
/// rather than fixed; only the STRUCTURE is asserted here.
#[test]
fn the_shadowed_net_does_not_move_in_the_waveform() {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_blms_vcd_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).expect("scratch dir");
    std::fs::write(
        d.join("t.sv"),
        "`timescale 1ns/1ns\nmodule t;\n  reg [7:0] x;\n  \
         initial begin x = 8'h11; #3; end\n  \
         initial begin\n    $dumpfile(\"w.vcd\"); $dumpvars(0, t);\n    #1;\n    \
         begin : b\n      reg [15:0] x; x = 16'hDEAD;\n      \
         $display(\"R inner=%0h\", x);\n    end\n  end\nendmodule\n",
    )
    .expect("write design");
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("t.sv")
        .current_dir(&d)
        .output()
        .expect("run vita");
    let vcd = std::fs::read_to_string(d.join("w.vcd")).expect("vcd written");
    let _ = std::fs::remove_dir_all(&d);
    assert!(out.status.success(), "{vcd}");

    // Two vars, not one: the 8-bit module net and a 16-bit local in a nested scope.
    assert!(vcd.contains("$var reg 8 ! x [7:0] $end"), "{vcd}");
    assert!(
        vcd.contains("x [15:0] $end"),
        "the local needs a var of its own:\n{vcd}"
    );
    assert_eq!(
        vcd.matches("$scope").count(),
        2,
        "nested scope missing:\n{vcd}"
    );

    // The module net is dumped once at 8'h11 and never changes again. Its id is `!`,
    // so a later `b… !` line would be the leak this test exists to catch.
    let after_dumpvars = vcd.split("$enddefinitions").nth(1).unwrap_or("");
    let net_changes = after_dumpvars
        .lines()
        .filter(|l| l.ends_with(" !") && l.starts_with('b'))
        .count();
    assert_eq!(
        net_changes, 1,
        "the module net must be written once:\n{vcd}"
    );
}

// ── the negative side: what must NOT change ───────────────────────────────

/// A block-local whose name does NOT shadow anything still takes the flatten path,
/// and two sequential blocks reusing a temp name still coalesce onto one net. The
/// fix must be reachable only from a shadow.
#[test]
fn a_non_shadowing_block_local_still_coalesces() {
    expect(
        "`timescale 1ns/1ns\nmodule t;\n  int zzz;\n  \
         initial begin\n    begin : b1 int tmp; tmp = 1; $display(\"R b1=%0d\", tmp); end\n    \
         begin : b2 int tmp; tmp = 2; $display(\"R b2=%0d\", tmp); end\n  end\nendmodule\n",
        &["R b1=1", "R b2=2"],
    );
}

/// ⭐ A reference OUTSIDE the declaring block, which is where the fix pays a second
/// time. `check_block_local_scope_leaks` rejected this because vita "keeps a body's
/// block-locals in a FLAT per-body table, so a reference outside the declaring block
/// would silently resolve to the block-local". Once the local owns a `$blk$` net that
/// sentence is false for it — the outer reference walks past a scope that does not
/// hold the name and lands on the module net — so the gate now skips names it can see
/// in `scoped_block_locals` and this runs.
///
/// ⚠️ I wrote this cell asserting the refusal, one edit before removing it. The
/// docstring it carried ("the scoping does not teach the OUTER reference to resolve")
/// was a guess about machinery I had not measured; `walk_scopes_key` already treats
/// `$blk$` as transparent, so it never needed teaching.
#[test]
fn a_reference_outside_the_declaring_block_resolves_to_the_module_net() {
    expect(
        "`timescale 1ns/1ns\nmodule t;\n  int n = 7;\n  \
         initial begin\n    begin int n; n = 3; end\n    \
         $display(\"R n=%0d\", n);\n  end\nendmodule\n",
        &["R n=7"],
    );
}

/// ⚠️ The gate is keyed on `scoped_block_locals`, which is populated for MODULE
/// PROCESS bodies only. A function/task body still keeps its flat table and therefore
/// its diagnostic — the boundary is asserted here so a later widening of the map does
/// not silently change what a frame body does.
#[test]
fn a_frame_body_keeps_the_outside_reference_diagnostic() {
    let (o, code) = run("`timescale 1ns/1ns\nmodule t;\n  \
         function automatic int g();\n    int n;\n    n = 7;\n    \
         begin int n; n = 3; end\n    return n;\n  endfunction\n  \
         initial $display(\"R=%0d\", g());\nendmodule\n");
    assert_ne!(
        code,
        Some(0),
        "a frame body has no `$blk$` scope to fall past:\n{o}"
    );
    assert!(o.contains("outside its"), "{o}");
}
