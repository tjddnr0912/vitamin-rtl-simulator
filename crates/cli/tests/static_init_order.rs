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
