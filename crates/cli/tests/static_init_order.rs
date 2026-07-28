//! §4.5.254 — the order static (time-0) declaration initializers run in.
//!
//! The rule these pin is not a guess; it was MEASURED against live iverilog 13.0 with
//! `$random` as the order witness (each draw is distinct, so the printed values say
//! exactly which initializer ran first):
//!
//!   1. every MODULE-scope initializer, in declaration order,
//!   2. then every BLOCK-LOCAL one, in declaration order among themselves —
//!      regardless of where the block sits relative to the module declaration,
//!      of whether the declaration is a string, and of whether its block earned a
//!      `$blk$` scope.
//!
//! vita had the first half backwards (a block-local was pushed during the Nets-phase
//! hoist, which runs BEFORE the module-scope sweep) and patched the symptom for STRINGS
//! only by holding those back to the end — which then reordered a string against its own
//! block's non-strings. Both halves are one list now, keyed by declaration offset.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_initord_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    (
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
        out.status.code() == Some(0),
    )
}

/// The three `$random` draws every test below reads, in the order iverilog produces
/// them from a fresh seed.
/// [`run`] with extra CLI arguments.
fn run_args(src: &str, args: &[&str]) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_initord_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .args(args)
        .current_dir(&d)
        .output()
        .expect("run vita");
    (
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
        out.status.code() == Some(0),
    )
}

const D1: &str = "303379748";
const D2: &str = "-1064739199";
const D3: &str = "-2071669239";

/// A module-scope initializer runs before a block-local one — even though the hoist
/// that collects the block-local runs first. iverilog: `m` takes the first draw.
#[test]
fn module_scope_initializers_run_before_block_local_ones() {
    let (o, ok) = run("module t;\n\
           int m = $random;\n\
           initial begin\n\
             begin int a = $random; $display(\"P m=%0d a=%0d\", m, a); end\n\
             $finish;\n\
           end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(
        o.contains(&format!("P m={D1} a={D2}")),
        "iverilog order:\n{o}"
    );
}

/// …and it still does when the module declaration is written AFTER the block. Source
/// position does not decide this; scope does. iverilog: `m` first, `a` second.
#[test]
fn module_scope_wins_even_when_declared_after_the_block() {
    let (o, ok) = run("module t;\n\
           initial begin begin int a = $random; #1 $display(\"A a=%0d\", a); end end\n\
           int m1 = $random;\n\
           int m2 = $random;\n\
           initial begin #2 $display(\"B m1=%0d m2=%0d\", m1, m2); $finish; end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(
        o.contains(&format!("A a={D3}")) && o.contains(&format!("B m1={D1} m2={D2}")),
        "module-scope statics first, then the block-local:\n{o}"
    );
}

/// S4: within one block, a STRING initializer keeps its declared position against the
/// non-strings. Holding strings back to the end (r19's fix for the module-scope
/// ordering) silently swapped these two. iverilog: `s` takes the first draw.
#[test]
fn a_block_local_string_keeps_its_declared_position() {
    let (o, ok) = run("module t;\n\
           initial begin\n\
             begin\n\
               string s = $sformatf(\"%0d\", $random);\n\
               int a = $random;\n\
               $display(\"P s=%s a=%0d\", s, a);\n\
             end\n\
             $finish;\n\
           end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(
        o.contains(&format!("P s={D1} a={D2}")),
        "declared order:\n{o}"
    );

    // The mirror image, which was right by accident before (last is last either way).
    let (o, ok) = run("module t;\n\
           initial begin\n\
             begin\n\
               int a = $random;\n\
               string s = $sformatf(\"%0d\", $random);\n\
               $display(\"P a=%0d s=%s\", a, s);\n\
             end\n\
             $finish;\n\
           end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(
        o.contains(&format!("P a={D1} s={D2}")),
        "declared order:\n{o}"
    );
}

/// The reason strings were held back in the first place, which must keep working: a
/// block-local string initializer may READ a module-scope string. It does, because
/// module scope goes first as a whole — not because strings go last.
#[test]
fn a_block_local_string_can_still_read_a_module_scope_string() {
    let (o, ok) = run("module t;\n\
           string g = \"G\";\n\
           initial begin\n\
             begin string s = {g, \"!\"}; $display(\"P s=%s\", s); end\n\
             $finish;\n\
           end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(o.contains("P s=G!"), "module string init first:\n{o}");
}

/// Block-locals that earned a `$blk$` scope (a same-named dynamic local in two blocks)
/// interleave with flattened ones by declaration order. Each run becomes its own t0
/// `initial`, and those execute in ProcId order — so the runs must be EMITTED in order.
#[test]
fn scoped_and_flattened_block_locals_interleave_in_declaration_order() {
    let (o, ok) = run("module t;\n\
           initial begin\n\
             begin int q[$] = '{$random}; $display(\"A=%0d\", q[0]); end\n\
             begin int z    = $random;    $display(\"B=%0d\", z); end\n\
             begin int q[$] = '{$random}; $display(\"C=%0d\", q[0]); end\n\
             $finish;\n\
           end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(
        o.contains(&format!("A={D1}"))
            && o.contains(&format!("B={D2}"))
            && o.contains(&format!("C={D3}")),
        "scoped/flattened interleave:\n{o}"
    );
}

/// Two separate processes: each block's locals initialize in the order the blocks are
/// declared, not the order the processes run.
#[test]
fn block_locals_of_different_processes_keep_source_order() {
    let (o, ok) = run("module t;\n\
           initial begin begin int a = $random; #1 $display(\"A=%0d\", a); end end\n\
           initial begin begin int b = $random; #2 $display(\"B=%0d\", b); end end\n\
           initial #3 $finish;\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(
        o.contains(&format!("A={D1}")) && o.contains(&format!("B={D2}")),
        "source order across processes:\n{o}"
    );
}

/// A generate scope has its own flush point; its block-locals must land there (and after
/// that scope's own module-level declarations), not in the enclosing module's sweep.
#[test]
fn a_generate_scopes_block_locals_flush_in_that_scope() {
    let (o, ok) = run("module t;\n\
           genvar i;\n\
           generate for (i = 0; i < 2; i = i + 1) begin : g\n\
             int m = i + 10;\n\
             initial begin\n\
               begin string s = $sformatf(\"%0d\", m); $display(\"G%0d=%s\", i, s); end\n\
             end\n\
           end endgenerate\n\
           initial #1 $finish;\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(
        o.contains("G0=10") && o.contains("G1=11"),
        "generate-scope block-local reads its own scope's init:\n{o}"
    );
}

// ── §4.5.255: what the adversarial review of the ordering slice found ────────

/// A generate body is a SCOPE to iverilog even where vita mints no prefix segment for it
/// — a `case` arm, an unlabeled `if`/`begin`. Its initializers run BEFORE the enclosing
/// module's, so keying them at the module prefix and filing them under "block-local, so
/// last" made three shapes silently wrong that had been right. iverilog: `a` first.
#[test]
fn an_unscoped_generate_bodys_initializers_precede_the_modules() {
    for gen in [
        "generate if (1) begin\n\
           initial begin begin int a = $random; #1 $display(\"P a=%0d mm=%0d\", a, mm); end end\n\
         end endgenerate\n",
        "generate case (1) 1: begin\n\
           initial begin begin int a = $random; #1 $display(\"P a=%0d mm=%0d\", a, mm); end end\n\
         end endcase endgenerate\n",
    ] {
        let (o, ok) = run(&format!(
            "module t;\n  int mm = $random;\n  {gen}  initial #2 $finish;\nendmodule\n"
        ));
        assert!(ok, "expected clean sim, got:\n{o}");
        assert!(
            o.contains(&format!("P a={D1} mm={D2}")),
            "generate content initializes first:\n{o}"
        );
    }
}

/// The same rule for a LABELED generate scope, which also pins the order INSIDE it: the
/// scope's own declarations, then its block-locals, then the enclosing module's.
/// iverilog: `gm` `a` `mm`.
#[test]
fn a_generate_scope_initializes_before_the_module_and_in_order_within() {
    let (o, ok) = run("module t;\n\
           genvar i;\n\
           int mm = $random;\n\
           generate for (i = 0; i < 1; i = i + 1) begin : g\n\
             int gm = $random;\n\
             initial begin\n\
               begin int a = $random; #1 $display(\"P gm=%0d a=%0d mm=%0d\", gm, a, mm); end\n\
             end\n\
           end endgenerate\n\
           initial #2 $finish;\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(
        o.contains(&format!("P gm={D1} a={D2} mm={D3}")),
        "generate scope first, module last:\n{o}"
    );
}

/// A routed string array's `new[n]` pre-size must be drained by the scope that OWNS the
/// declaration. Draining it when a walk OPENS handed an inner generate body's pre-size to
/// the outer process, so `new[n]` ran after the element writes the inner had already
/// emitted and wiped them — an empty array at exit 0. Flush order is innermost-first,
/// which is ownership order.
#[test]
fn a_pre_size_is_drained_by_the_scope_that_owns_the_declaration() {
    let (o, ok) = run("module t;\n\
           generate if (1) begin\n\
             initial begin\n\
               begin string s[2] = '{\"a\",\"b\"}; $display(\"P=|%s|%s|\", s[0], s[1]); end\n\
             end\n\
           end endgenerate\n\
           initial #1 $finish;\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(
        o.contains("P=|a|b|"),
        "the writes survive the pre-size:\n{o}"
    );
}

/// The cross-product the first round of these tests missed entirely, and where both
/// review lenses found the same regression: a MODULE-BODY block-local alongside a
/// `generate` region. An unlabeled generate body shares the module's prefix, so a
/// prefix-only claim let the generate walk's flush emit the MODULE's block-locals ahead
/// of the module sweep. Ownership is the flag, not the prefix.
#[test]
fn a_generate_region_does_not_capture_the_modules_own_block_locals() {
    // Even an EMPTY generate arms it — the flush is unconditional.
    for gen in [
        "generate\n  endgenerate\n",
        "generate if (1) begin : g\n    wire dummy;\n  end endgenerate\n",
        "generate if (0) begin : g\n    wire dummy;\n  end endgenerate\n",
        "genvar i;\n  generate for (i = 0; i < 2; i = i + 1) begin : g\n    wire dummy;\n  end endgenerate\n",
    ] {
        let (o, ok) = run(&format!(
            "module t;\n  {gen}  int mm = $random;\n\
             initial begin begin int a = $random; $display(\"P mm=%0d a=%0d\", mm, a); end $finish; end\n\
             endmodule\n"
        ));
        assert!(ok, "expected clean sim, got:\n{o}");
        assert!(
            o.contains(&format!("P mm={D1} a={D2}")),
            "module sweep still runs first with a generate present:\n{o}"
        );
    }
}

/// The same shape for the two things a captured initializer breaks: a block-local that
/// READS a module variable, and a routed string array split from its `new[n]` (the
/// pre-size drain partitions by owner, so a captured write left its pre-size behind and
/// the array came out empty at exit 0).
#[test]
fn a_generate_region_breaks_neither_a_read_nor_a_pre_size() {
    let (o, ok) = run("module t;\n\
           int mm = $random;\n\
           initial begin\n\
             string s = (mm != 0) ? \"SET\" : \"ZERO\";\n\
             string arr[2] = '{\"a\",\"b\"};\n\
             $display(\"P s=%s arr=|%s|%s|\", s, arr[0], arr[1]);\n\
             $finish;\n\
           end\n\
           generate if (1) begin : g\n    wire dummy;\n  end endgenerate\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(
        o.contains("P s=SET arr=|a|b|"),
        "the read sees mm, and the pre-size still precedes the writes:\n{o}"
    );
}

/// A module body is never "inside a generate", however it was reached. A child
/// instantiated inside a generate used to elaborate its whole body with the flag stuck
/// on, so its own module-scope block-locals were tagged as generate-owned.
#[test]
fn a_child_instantiated_inside_a_generate_owns_its_own_body() {
    let (o, ok) = run("module sub;\n\
           int mm = $random;\n\
           string m = \"M\";\n\
           initial begin\n\
             begin\n\
               int a = $random;\n\
               string bl = {m, \"-x\"};\n\
               string arr[2] = '{\"p\",\"q\"};\n\
               #1 $display(\"P mm=%0d a=%0d bl=%s arr=|%s|%s|\", mm, a, bl, arr[0], arr[1]);\n\
             end\n\
           end\n\
         endmodule\n\
         module t;\n\
           genvar i;\n\
           generate for (i = 0; i < 1; i = i + 1) begin : g\n    sub u();\n  end endgenerate\n\
           initial #2 $finish;\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(
        o.contains(&format!("P mm={D1} a={D2} bl=M-x arr=|p|q|")),
        "the child's own body owns its initializers:\n{o}"
    );
}

// ── §4.5.256: the scope order, measured as a matrix ──────────────────────────
//
// Static initialization runs before any user process (IEEE 1800 §6.21), and the order
// AMONG the initializers is, against live iverilog 13.0:
//
//   MODULE   scope: ① its generate scopes ② its child instances ③ its own variables
//                   ④ its own block-locals
//   GENERATE scope: ① its child instances ② its own variables ③ its own block-locals
//                   ④ its nested generate scopes
//
// Neither is the order vita creates the processes in, and no pass reordering can produce
// both — a child instance's initializers must precede its parent's, while the parent's
// own processes are created before the child exists. So the order is carried as data (a
// rank path per initializer process, exported as a t0 ordering key) and the elaboration
// pass order is left alone. Each test below states the iverilog draws it pins.

/// The two directions that make the module rule: its generate scope goes first even when
/// written last, and its own variables go last even when written first.
#[test]
fn a_modules_own_variables_initialize_after_its_generate_scopes() {
    let (o, ok) = run("module t;\n\
           int mm = $random;\n\
           generate if (1) begin : g\n\
             int gm = $random;\n\
             initial #1 $display(\"P gm=%0d\", gm);\n\
           end endgenerate\n\
           initial begin #1 $display(\"P mm=%0d\", mm); #1 $finish; end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(
        o.contains(&format!("P gm={D1}")) && o.contains(&format!("P mm={D2}")),
        "generate scope first:\n{o}"
    );
}

/// A child instance initializes before its parent — the case vita could not express at
/// all, because the parent's processes are created in an earlier pass than the child.
#[test]
fn a_child_instance_initializes_before_its_parent() {
    let (o, ok) = run(
        "module sub; int sm = $random; initial #1 $display(\"P sm=%0d\", sm); endmodule\n\
         module t;\n\
           int mm = $random;\n\
           sub u();\n\
           initial begin #1 $display(\"P mm=%0d\", mm); #1 $finish; end\n\
         endmodule\n",
    );
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(
        o.contains(&format!("P sm={D1}")) && o.contains(&format!("P mm={D2}")),
        "child first:\n{o}"
    );
}

/// …which is observable without `$random` too: a parent `initial` reading a child's
/// STRING saw the empty default, because "before any process" had been approximated by
/// "gets a lower ProcId" and that approximation does not survive an instance boundary.
#[test]
fn a_parent_process_sees_a_childs_initialized_string() {
    let (o, ok) = run("module sub; int sm = 5; string ss = \"CHILD\"; endmodule\n\
         module t;\n\
           sub u();\n\
           initial $display(\"P sm=%0d ss=|%s|\", u.sm, u.ss);\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(o.contains("P sm=5 ss=|CHILD|"), "iverilog value:\n{o}");
}

/// Generate scopes go before child instances, whichever is written first.
#[test]
fn generate_scopes_initialize_before_child_instances() {
    let (o, ok) = run(
        "module sub; int sm = $random; initial #1 $display(\"P sm=%0d\", sm); endmodule\n\
         module t;\n\
           sub u();\n\
           generate if (1) begin : g\n\
             int a = $random;\n\
             initial #1 $display(\"P a=%0d\", a);\n\
           end endgenerate\n\
           initial #2 $finish;\n\
         endmodule\n",
    );
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(
        o.contains(&format!("P a={D1}")) && o.contains(&format!("P sm={D2}")),
        "generate before instance:\n{o}"
    );
}

/// Inside a GENERATE scope the order is different, and both halves are pinned here: a
/// nested generate goes AFTER the scope's own variables (even when written before them),
/// while a child instance goes BEFORE them.
#[test]
fn a_generate_scope_orders_its_instance_before_and_its_nested_generate_after() {
    let (o, ok) = run(
        "module sub; int sm = $random; initial #1 $display(\"P sm=%0d\", sm); endmodule\n\
         module t;\n\
           generate if (1) begin : g1\n\
             sub u();\n\
             int a = $random;\n\
             if (1) begin : g2\n\
               int b = $random;\n\
               initial #1 $display(\"P b=%0d\", b);\n\
             end\n\
             initial #1 $display(\"P a=%0d\", a);\n\
           end endgenerate\n\
           int mm = $random;\n\
           initial begin #1 $display(\"P mm=%0d\", mm); #1 $finish; end\n\
         endmodule\n",
    );
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(
        o.contains(&format!("P sm={D1}"))
            && o.contains(&format!("P a={D2}"))
            && o.contains(&format!("P b={D3}"))
            && o.contains("P mm=-1309649309"),
        "instance, own, nested generate, then the module:\n{o}"
    );
}

/// A non-root module follows the same rule, and its whole subtree precedes its parent's
/// own variables.
#[test]
fn the_rule_is_the_same_at_every_level_of_the_hierarchy() {
    let (o, ok) = run(
        "module leaf; int lv = $random; initial #1 $display(\"P lv=%0d\", lv); endmodule\n\
         module mid;\n\
           int mv = $random;\n\
           generate if (1) begin : mg\n\
             int gv = $random;\n\
             initial #1 $display(\"P gv=%0d\", gv);\n\
           end endgenerate\n\
           leaf lf();\n\
           initial #1 $display(\"P mv=%0d\", mv);\n\
         endmodule\n\
         module t;\n\
           int tv = $random;\n\
           mid u();\n\
           initial begin #1 $display(\"P tv=%0d\", tv); #1 $finish; end\n\
         endmodule\n",
    );
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(
        o.contains(&format!("P gv={D1}"))
            && o.contains(&format!("P lv={D2}"))
            && o.contains(&format!("P mv={D3}"))
            && o.contains("P tv=-1309649309"),
        "mid's generate, mid's child, mid's own, then the root's own:\n{o}"
    );
}

/// Sibling scopes keep source order, and two instances of one module keep declaration
/// order — the rank is a path, so siblings differ only in their own slot.
#[test]
fn sibling_scopes_keep_source_order() {
    let (o, ok) = run("module sub(input int id);\n\
           int sm = $random;\n\
           initial #1 $display(\"P id=%0d sm=%0d\", id, sm);\n\
         endmodule\n\
         module t;\n\
           sub u1(.id(1));\n\
           generate if (1) begin : ga\n\
             int a = $random;\n\
             initial #1 $display(\"P a=%0d\", a);\n\
           end endgenerate\n\
           sub u2(.id(2));\n\
           generate if (1) begin : gb\n\
             int b = $random;\n\
             initial #1 $display(\"P b=%0d\", b);\n\
           end endgenerate\n\
           initial #2 $finish;\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(
        o.contains(&format!("P a={D1}"))
            && o.contains(&format!("P b={D2}"))
            && o.contains(&format!("P id=1 sm={D3}"))
            && o.contains("P id=2 sm=-1309649309"),
        "generates in source order, then instances in source order:\n{o}"
    );
}

// ── §4.5.257: initialization is a PHASE, not a process ───────────────────────

/// Measured: a declaration initializer produces no event. `reg clk = 0;` gives
/// `always @clk` no X→0 edge, and neither does a NON-constant `int nc = src + 1;` give
/// one to `always @nc`. Running the initializers as ordinary t0 processes produced both,
/// and ordering them correctly does not help — by then the arming has happened. IEEE 1800
/// §6.21's "before any initial or always block starts" is literal: a pre-arm phase.
#[test]
fn a_declaration_initializer_produces_no_event() {
    let (o, ok) = run("module t;\n\
           reg clk = 0;\n\
           int src = 7;\n\
           int nc = src + 1;\n\
           int ec = 0, en = 0;\n\
           always @clk ec = ec + 1;\n\
           always @nc  en = en + 1;\n\
           initial begin #1 clk = 1; #1 clk = 0; #1\n\
             $display(\"P nc=%0d clk_edges=%0d nc_edges=%0d\", nc, ec, en); $finish; end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(
        o.contains("P nc=8 clk_edges=2 nc_edges=0"),
        "iverilog: the two user edges only:\n{o}"
    );
}

/// …and it still preserves Z, which the pre-applied `net.init` value used to be the only
/// carrier of.
#[test]
fn a_four_state_initializer_keeps_z() {
    let (o, ok) = run("module t;\n\
           initial begin\n\
             logic [3:0] zi = 4'bz0z1;\n\
             $display(\"P declinit %b\", 8'(zi));\n\
           end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(o.contains("P declinit 0000z0z1"), "iverilog value:\n{o}");
}

/// A CONSTANT initializer is an ordered assignment like any other. Pre-applying it at net
/// creation took it out of the initialization order, so vita disagreed with ITSELF: a
/// generate initializer reading a module `int mm = 77;` saw 77, while the same read of a
/// non-constant `int mm = f();` correctly saw 0. iverilog gives 0 for both.
#[test]
fn a_constant_initializer_participates_in_the_order() {
    let src = |init: &str| {
        format!(
            "module t;\n\
               function int f(); return 77; endfunction\n\
               int mm = {init};\n\
               generate if (1) begin : g\n\
                 int gm = t.mm;\n\
                 initial #1 $display(\"P gm=%0d\", gm);\n\
               end endgenerate\n\
               initial begin #1 $display(\"P mm=%0d\", mm); #1 $finish; end\n\
             endmodule\n"
        )
    };
    for init in ["77", "f()"] {
        let (o, ok) = run(&src(init));
        assert!(ok, "expected clean sim for `{init}`, got:\n{o}");
        assert!(
            o.contains("P gm=0") && o.contains("P mm=77"),
            "`{init}`: the generate scope initializes first, so it reads mm's default:\n{o}"
        );
    }
}

/// The same rule inside one scope, where it is plain declaration order: a later
/// declaration's initializer sees an earlier one's value, an earlier one does not see a
/// later one's — whether or not the later initializer is constant.
#[test]
fn declaration_order_holds_for_constants_too() {
    let (o, ok) = run("module t;\n\
           int a = 5;\n\
           int b = a + 1;\n\
           initial begin $display(\"P a=%0d b=%0d\", a, b); $finish; end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(o.contains("P a=5 b=6"), "iverilog value:\n{o}");
}

// ── §4.5.259: what the adversarial review of the phase found ─────────────────

/// F1. Clearing the whole dirty list to suppress the initialization's own events also
/// discarded the t0 continuous-assign settle's, which run BEFORE arming on the same list.
/// Those are not recoverable — the settle inside the run loop writes the same value, and
/// only an ACTUAL change is recorded — so `always @(w)` on `assign w = 1'b1;` never fired.
/// Design-wide: one unrelated `reg r = 1'b0;` anywhere was enough to arm it.
#[test]
fn the_t0_continuous_assign_settle_still_produces_its_events() {
    let (o, ok) = run("module t;\n\
           reg unrelated = 1'b0;\n\
           wire w1, w0;\n\
           assign w1 = 1'b1;\n\
           assign w0 = 1'b0;\n\
           int c1 = 0, c0 = 0;\n\
           always @(w1) c1 = c1 + 1;\n\
           always @(w0) c0 = c0 + 1;\n\
           initial begin #1 $display(\"P c1=%0d c0=%0d\", c1, c0); $finish; end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(o.contains("P c1=1 c0=1"), "iverilog fires each once:\n{o}");
}

/// F2. The rank counter was shared across slots, and only ONE of the four generate walks
/// visits `Instance` items — so a generate written after an instance drew a different
/// number in the VarInit and Instances phases, and its child instance was filed under a
/// path that no longer matched its own flush. Moving the instance changed the answer.
#[test]
fn an_instance_written_before_a_generate_does_not_shift_its_rank() {
    let (o, ok) = run(
        "module m_a; int v = $random; initial #1 $display(\"P a=%0d\", v); endmodule\n\
         module m_b; int v = $random; initial #1 $display(\"P b=%0d\", v); endmodule\n\
         module t;\n\
           m_a u_a();\n\
           generate if (1) begin : g\n\
             m_b u_b();\n\
             int gv = $random;\n\
             initial #1 $display(\"P gv=%0d\", gv);\n\
           end endgenerate\n\
           initial #2 $finish;\n\
         endmodule\n",
    );
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(
        o.contains(&format!("P b={D1}"))
            && o.contains(&format!("P gv={D2}"))
            && o.contains(&format!("P a={D3}")),
        "the generate's child instance still initializes before its own variable:\n{o}"
    );
}

/// F3. The package flush was the one unranked flush in the crate, so package initializers
/// were not in the phase at all: they ran after every module's, and their writes produced
/// events. Both halves are pinned — the value a module reads, and the absence of an edge.
#[test]
fn package_initializers_are_part_of_the_phase() {
    let (o, ok) = run("package pk;\n\
           int pv = 11;\n\
           logic [7:0] pb = 8'hA5;\n\
           int pa[2] = '{7,8};\n\
           logic pclk = 1'b1;\n\
         endpackage\n\
         module t;\n\
           import pk::*;\n\
           int mv = pv + 1;\n\
           logic [7:0] mb = pb ^ 8'h0F;\n\
           int ma = pa[0] + 1;\n\
           int pos = 0, any = 0;\n\
           always @(posedge pclk) pos = pos + 1;\n\
           always @(pv) any = any + 1;\n\
           initial begin #1\n\
             $display(\"P mv=%0d mb=%h ma=%0d pos=%0d any=%0d\", mv, mb, ma, pos, any);\n\
             $finish; end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(
        o.contains("P mv=12 mb=aa ma=8 pos=0 any=0"),
        "iverilog values, and no spurious edge on a package constant:\n{o}"
    );
}

/// F4. An interface instance is a scope; without a rank scope of its own its flush
/// borrowed the ENCLOSING scope's own-variables slot, and since its two call sites run in
/// different passes than the module's own flush, the rank vectors collided — a module's
/// own initializer ran BETWEEN two interfaces.
#[test]
fn an_interface_instance_is_ranked_as_a_scope() {
    let (o, ok) = run("interface i_a; int a = $random; endinterface\n\
         interface i_b; int b = $random; endinterface\n\
         module t;\n\
           i_a u1();\n\
           i_b u2();\n\
           int mm = $random;\n\
           initial begin #1 $display(\"P a=%0d b=%0d mm=%0d\", u1.a, u2.b, mm); $finish; end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(
        o.contains(&format!("P a={D1} b={D2} mm={D3}")),
        "both interfaces before the module's own:\n{o}"
    );

    // …and a generate-nested one initializes before the generate's own variable.
    let (o, ok) = run("interface i_v; int iv = $random; endinterface\n\
         module t;\n\
           generate if (1) begin : g\n\
             i_v u();\n\
             int gv = $random;\n\
             initial #1 $display(\"P iv=%0d gv=%0d\", u.iv, gv);\n\
           end endgenerate\n\
           initial #2 $finish;\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(
        o.contains(&format!("P iv={D1} gv={D2}")),
        "instance slot:\n{o}"
    );
}

/// §4.5.260. An interface instance and a module instance share the instance slot but are
/// elaborated in DIFFERENT passes — interfaces in Nets, module children in Instances — so
/// the per-scope counter could not order them: every interface drew a lower number than
/// every module child, whatever the source said. Giving the earlier fix a counter made the
/// "interface written first" half right and the "module instance written first" half
/// wrong. Both draw the declaring name's source offset now, which is the same in every
/// pass. Pinned in BOTH orders, and with a hierarchical read as the value witness.
#[test]
fn an_interface_and_a_module_instance_interleave_by_source_order() {
    let (o, ok) = run("module ch; int c = 5; endmodule\n\
         interface ifc; int iv = tb.u0.c + 1; endinterface\n\
         module tb;\n\
           ch u0();\n\
           ifc f1();\n\
           initial $display(\"P iv=%0d\", f1.iv);\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(
        o.contains("P iv=6"),
        "the instance declared first initializes first:\n{o}"
    );

    for (order, want) in [
        ("  ch  u0();\n  ifc i0();\n", format!("P c={D1} iv={D2}")),
        ("  ifc i0();\n  ch  u0();\n", format!("P c={D2} iv={D1}")),
    ] {
        let (o, ok) = run(&format!(
            "interface ifc; int iv = $random; endinterface\n\
             module ch; int c = $random; endmodule\n\
             module tb;\n{order}\
               initial #1 $display(\"P c=%0d iv=%0d\", u0.c, i0.iv);\n\
               initial #2 $finish;\n\
             endmodule\n"
        ));
        assert!(ok, "expected clean sim, got:\n{o}");
        assert!(o.contains(&want), "source order decides:\n{o}");
    }
}

/// §4.5.261. One source offset cannot carry three different questions, and the review
/// found all three ways it fails. A ROOT's key is its position in the root list, not its
/// offset — `--top zz --top aa` elaborates in the order given, and `-L` library mode
/// compiles each unit separately so offsets from different units are not comparable.
#[test]
fn the_top_option_decides_root_initialization_order() {
    let src = "module aa; int v = 1 + zz.v; initial $display(\"P aa=%0d\", v); endmodule\n\
               module zz; int v = 1 + aa.v; initial $display(\"P zz=%0d\", v); endmodule\n";
    let (o, ok) = run_args(src, &["--top", "zz", "--top", "aa"]);
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(
        o.contains("P zz=1") && o.contains("P aa=2"),
        "iverilog -s zz -s aa:\n{o}"
    );

    let (o, ok) = run_args(src, &["--top", "aa", "--top", "zz"]);
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(
        o.contains("P aa=1") && o.contains("P zz=2"),
        "the other order:\n{o}"
    );
}

/// An instance ARRAY's elements each keep their own subtree. Sharing one key does NOT
/// fall back to a ProcId tie-break: an element's child scopes and its own variables
/// produce DIFFERENT rank vectors, so equal keys made the sort group by slot ACROSS
/// elements and interleave them. iverilog: 5.
#[test]
fn instance_array_elements_keep_their_subtrees_together() {
    let (o, ok) = run("module inner(); int gz = 1; endmodule\n\
         module ch();\n\
           inner n();\n\
           int own = 1 + tb.u[0].n.gz + tb.u[1].n.gz;\n\
         endmodule\n\
         module tb;\n\
           ch u[1:0]();\n\
           initial $display(\"P sum=%0d\", tb.u[0].own + tb.u[1].own);\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(
        o.contains("P sum=5"),
        "each element's child before its own:\n{o}"
    );
}

/// A `bind` directive lives in the compilation unit, so its source offset is not a
/// position inside the target module's body. Using it as one made the answer depend on
/// where the `bind` line was written — and, across files, on the order they were listed.
/// Bound checkers go in their own band, after everything the target declares itself.
#[test]
fn a_bind_directives_position_does_not_change_initialization_order() {
    let chk = "module chk;   int c = 1 + tb.u.w.z; initial $display(\"P chk=%0d\", c); endmodule\n\
               module grand; int z = 1 + tb.u.bk.c; initial $display(\"P gr=%0d\", z); endmodule\n";
    for src in [
        format!("{chk}bind sub chk bk();\nmodule sub; grand w(); endmodule\nmodule tb; sub u(); endmodule\n"),
        format!("{chk}module sub; grand w(); endmodule\nbind sub chk bk();\nmodule tb; sub u(); endmodule\n"),
    ] {
        let (o, ok) = run(&src);
        assert!(ok, "expected clean sim, got:\n{o}");
        assert!(
            o.contains("P gr=1") && o.contains("P chk=2"),
            "the bind line's position must not matter:\n{o}"
        );
    }
}

/// §4.5.262. The band says how THIS instance was reached; the children a module declares
/// itself are ordinary body items however their parent got here. Leaving `rank_band` set
/// leaked it into the whole bound subtree, so a module reached through a bind keyed its
/// own body children band 1 too — and when that module was itself a bind target, its body
/// children and its bound children collided on band and fell back to comparing a body
/// offset against a compilation-unit one. iverilog cannot parse `bind`, so this pins the
/// invariant rather than an oracle value: the answer must not depend on source layout.
#[test]
fn a_nested_bind_does_not_leak_its_band_into_the_bound_subtree() {
    let head = "package pk;\n\
           function automatic int tick(input string s); $display(\"P %s\", s); return 0; endfunction\n\
         endpackage\n\
         import pk::*;\n\
         module chk2; int d = tick(\"chk2.d\"); endmodule\n\
         module leaf; int lv = tick(\"leaf.lv\"); endmodule\n";
    let tail = "module sub; int sv = tick(\"sub.sv\"); endmodule\n\
         bind sub chk bk();\n\
         module tb; sub u(); initial $display(\"done\"); endmodule\n";
    let mut seen: Vec<String> = Vec::new();
    for body in [
        "bind chk chk2 bk2();\nmodule chk; leaf L(); int c = tick(\"chk.c\"); endmodule\n",
        "module chk; leaf L(); int c = tick(\"chk.c\"); endmodule\nbind chk chk2 bk2();\n",
    ] {
        let (o, ok) = run(&format!("{head}{body}{tail}"));
        assert!(ok, "expected clean sim, got:\n{o}");
        seen.push(
            o.lines()
                .filter(|l| l.starts_with("P "))
                .collect::<Vec<_>>()
                .join(" "),
        );
    }
    assert_eq!(
        seen[0], seen[1],
        "where the `bind` line sits must not change the initialization order"
    );
    assert!(
        seen[0].contains("P leaf.lv") && seen[0].contains("P chk.c"),
        "and the bound checker's own body children still come first: {}",
        seen[0]
    );
}

/// §4.5.263. A `generate … endgenerate` REGION is purely syntactic (IEEE 1800 §27.3):
/// items written directly in it, outside any `if`/`for`/`case`, are ordinary module items.
/// One function serves both the region and a generate BLOCK body, so giving it a rank
/// scope made a region behave like a block — a region-level `int mv = g.gv;` read 0
/// instead of the block's 7, and a region-level instance initialized before the block
/// beside it. Regions are transparent again, and the module sweep walks them in
/// declaration order so a region-level variable interleaves with the module's own.
#[test]
fn a_generate_region_is_transparent() {
    // Value: the region-level variable reads the BLOCK's variable, so the block must
    // initialize first. iverilog: 7.
    let (o, ok) = run("module t;\n\
           generate\n\
             if (1) begin : g\n\
               int gv = 7;\n\
             end\n\
             int mv = g.gv;\n\
             initial $display(\"P mv=%0d\", mv);\n\
           endgenerate\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(
        o.contains("P mv=7"),
        "the block initializes before the region item:\n{o}"
    );

    // Order: a bare instance in the region is a MODULE child, so the block beside it
    // still goes first. iverilog: gv, lv.
    let (o, ok) = run(
        "module leaf; int lv = $random; initial #1 $display(\"P lv=%0d\", lv); endmodule\n\
         module t;\n\
           generate\n\
             leaf u1();\n\
             if (1) begin : g\n\
               int gv = $random;\n\
               initial #1 $display(\"P gv=%0d\", gv);\n\
             end\n\
           endgenerate\n\
           initial #2 $finish;\n\
         endmodule\n",
    );
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(
        o.contains(&format!("P gv={D1}")) && o.contains(&format!("P lv={D2}")),
        "the region does not make its bare instance a generate child:\n{o}"
    );

    // Interleaving: a region-level variable takes its DECLARATION position among the
    // module's own. iverilog: a, b, c — which vita never got right, before or after.
    let (o, ok) = run("module t;\n\
           int a = $random;\n\
           generate\n\
             int b = $random;\n\
           endgenerate\n\
           int c = $random;\n\
           initial #1 $display(\"P a=%0d b=%0d c=%0d\", a, b, c);\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(
        o.contains(&format!("P a={D1} b={D2} c={D3}")),
        "declaration order straight through the region:\n{o}"
    );
}

/// §4.5.264. One level below the region: a free-standing `begin…end` in a gen-item list
/// is the ANACHRONISTIC SURROUND (iverilog warns and treats it as syntax), because the
/// parser unwraps an `if`/`for`/`case` body's `begin…end` and hoists its label — so a
/// `GenItem::Block` only ever arrives as a free item. Unlabeled, it is transparent;
/// labeled, it still mints a scope; and an unlabeled `if` BODY is a scope either way.
/// All three iverilog-measured.
#[test]
fn a_bare_begin_in_a_generate_list_is_syntax_but_a_labeled_one_is_a_scope() {
    let (o, ok) = run("module t;\n\
           int m0 = $random;\n\
           generate begin int u = $random; end endgenerate\n\
           int m1 = $random;\n\
           initial $display(\"P m0=%0d u=%0d m1=%0d\", m0, u, m1);\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(
        o.contains(&format!("P m0={D1} u={D2} m1={D3}")),
        "unlabeled: declaration order straight through:\n{o}"
    );

    let (o, ok) = run("module t;\n\
           int m0 = $random;\n\
           generate begin : lb int u = $random; end endgenerate\n\
           int m1 = $random;\n\
           initial $display(\"P m0=%0d u=%0d m1=%0d\", m0, lb.u, m1);\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(
        o.contains(&format!("P m0={D2} u={D1} m1={D3}")),
        "labeled: a scope, so it initializes first:\n{o}"
    );

    let (o, ok) = run("module t;\n\
           int m0 = $random;\n\
           generate if (1) begin\n\
             int u = $random;\n\
             initial $display(\"P u=%0d\", u);\n\
           end endgenerate\n\
           int m1 = $random;\n\
           initial $display(\"P m0=%0d m1=%0d\", m0, m1);\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(
        o.contains(&format!("P u={D1}")) && o.contains(&format!("P m0={D2} m1={D3}")),
        "an unlabeled `if` body is a scope either way:\n{o}"
    );
}

/// §4.5.265. Ownership by a BOOL could not separate two nested generate scopes: a `case`
/// arm and an unlabeled `if` body mint no prefix segment, so a nested one shares its
/// parent's key AND is "in a generate" too — and its flush claimed the PARENT's
/// block-local initializers, emitting them under its own (later) rank. iverilog runs the
/// enclosing scope's own block-locals first. The owner's RANK PATH says whose, and it is
/// stable across the elaboration phases by construction.
#[test]
fn a_prefix_less_nested_generate_does_not_claim_its_parents_block_locals() {
    let tick = "package pk;\n\
           function automatic int tick(input string s); $display(\"P %s\", s); return 0; endfunction\n\
         endpackage\n\
         import pk::*;\n";
    // Both nestings that mint no segment, and both source orders.
    for inner in [
        "case (1) 1: begin : h int v = tick(\"IN\"); end endcase\n",
        "if (1) begin int v = tick(\"IN\"); end\n",
    ] {
        for order in 0..2 {
            let bl = "initial begin int l = tick(\"OUT\"); end\n";
            let body = if order == 0 {
                format!("{inner}{bl}")
            } else {
                format!("{bl}{inner}")
            };
            let (o, ok) = run(&format!(
                "{tick}module t;\n  generate case (1) 1: begin : g\n{body}  end endcase endgenerate\n\
                 endmodule\n"
            ));
            assert!(ok, "expected clean sim, got:\n{o}");
            let seen: Vec<&str> = o.lines().filter(|l| l.starts_with("P ")).collect();
            assert_eq!(
                seen,
                vec!["P OUT", "P IN"],
                "the enclosing scope's own block-local runs first:\n{o}"
            );
        }
    }
}
