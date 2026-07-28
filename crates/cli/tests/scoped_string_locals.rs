//! §4.5.255 — a `string` ARRAY declared under the same name in two blocks.
//!
//! Two same-named block-locals are two distinct variables (IEEE 1800 §6.21), and vita
//! gives such a pair distinct storage by putting each declaration in its own `$blk$<lo>`
//! scope. String arrays had been excluded from that: their per-element storage is
//! registered under the DECLARING prefix, while the collector that expands the
//! initializer and the pre-size that sets the length both ran in the MODULE prefix — so a
//! scoped one came up length 0. Review S1 answered that by excluding the shape (back to
//! loud); this slice answers it by removing the asymmetry (the collector runs inside the
//! scope, and the pre-size is recorded there), which makes the shape correct.
//!
//! Every expectation below is live iverilog 13.0's, with one exception called out at
//! `an_unassigned_element_is_the_empty_string`.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_sstr_{}_{n}", std::process::id()));
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

fn loud(src: &str) -> bool {
    let (o, ok) = run(src);
    !ok && o.contains("error[VITA")
}

/// Declaration initializers on both sides, each block keeping its own elements.
#[test]
fn each_blocks_string_array_initializer_fills_its_own_storage() {
    let (o, ok) = run("module t;\n\
           initial begin\n\
             begin string s[2] = '{\"x\",\"y\"}; $display(\"A=|%s|%s|\", s[0], s[1]); end\n\
             begin string s[2] = '{\"p\",\"q\"}; $display(\"B=|%s|%s|\", s[0], s[1]); end\n\
             $finish;\n\
           end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(
        o.contains("A=|x|y|") && o.contains("B=|p|q|"),
        "iverilog A=|x|y| B=|p|q|:\n{o}"
    );
}

/// Multi-dimensional, which flattens row-major onto one container — the scoped copy must
/// use the same geometry as the module-scope one.
#[test]
fn a_multi_dim_string_array_keeps_its_row_major_geometry() {
    let (o, ok) = run("module t;\n\
           initial begin\n\
             begin string s[2][2] = '{'{\"a\",\"b\"},'{\"c\",\"d\"}};\n\
               $display(\"A=|%s|%s|\", s[0][1], s[1][0]); end\n\
             begin string s[2][2]; s[1][1]=\"z\"; $display(\"B=|%s|\", s[1][1]); end\n\
             $finish;\n\
           end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(
        o.contains("A=|b|c|") && o.contains("B=|z|"),
        "iverilog A=|b|c| B=|z|:\n{o}"
    );
}

/// A DESCENDING declared range: pattern element k fills from the left bound (§10.9.1),
/// and an index read must agree with it.
#[test]
fn a_descending_string_array_fills_from_the_left_bound() {
    let (o, ok) = run("module t;\n\
           initial begin\n\
             begin string s[3:1] = '{\"a1\",\"b2\",\"c3\"};\n\
               $display(\"A=|%s|%s|%s|\", s[3], s[2], s[1]); end\n\
             begin string s[3:1]; s[2]=\"zz\"; $display(\"B=|%s|\", s[2]); end\n\
             $finish;\n\
           end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(
        o.contains("A=|a1|b2|c3|") && o.contains("B=|zz|"),
        "iverilog A=|a1|b2|c3| B=|zz|:\n{o}"
    );
}

/// The same name declared with DIFFERENT string shapes in two blocks — a fixed array and
/// a dynamic one. Each gets the storage its own declaration asks for.
#[test]
fn a_fixed_and_a_dynamic_string_array_can_share_a_name() {
    let (o, ok) = run("module t;\n\
           initial begin\n\
             begin string s[2]; s[0]=\"aa\"; $display(\"A=|%s|\", s[0]); end\n\
             begin string s[]; s = new[1]; s[0]=\"bb\";\n\
               $display(\"B=|%s| n=%0d\", s[0], s.size()); end\n\
             $finish;\n\
           end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(
        o.contains("A=|aa|") && o.contains("B=|bb| n=1"),
        "iverilog A=|aa| B=|bb| n=1:\n{o}"
    );
}

/// A scoped block re-entered by a loop. The storage is STATIC, so it persists across
/// iterations and is simply rewritten — not re-created.
#[test]
fn a_scoped_string_array_in_a_loop_body_is_static_storage() {
    let (o, ok) = run("module t;\n\
           initial begin\n\
             for (int i = 0; i < 2; i++) begin\n\
               begin string s[2]; s[0] = $sformatf(\"i%0d\", i); $display(\"L=|%s|\", s[0]); end\n\
             end\n\
             begin string s[2]; s[1]=\"q\"; $display(\"B=|%s|\", s[1]); end\n\
             $finish;\n\
           end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(
        o.contains("L=|i0|") && o.contains("L=|i1|") && o.contains("B=|q|"),
        "iverilog L=|i0| L=|i1| B=|q|:\n{o}"
    );
}

/// Two `fork` ARMS declaring the same name. Each arm is its own block with its own span,
/// so each gets its own net — and a process reaches an arm once per fork, which is the
/// single-live-activation condition the flatten needs. This was loud before.
#[test]
fn two_fork_arms_can_declare_the_same_string_array() {
    let (o, ok) = run("module t;\n\
           initial begin\n\
             fork\n\
               begin string s[2]; s[0]=\"f1\"; $display(\"F1=|%s|\", s[0]); end\n\
               begin string s[2]; s[0]=\"f2\"; $display(\"F2=|%s|\", s[0]); end\n\
             join\n\
             $finish;\n\
           end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(
        o.contains("F1=|f1|") && o.contains("F2=|f2|"),
        "each arm keeps its own array (arm ORDER is unspecified):\n{o}"
    );
}

/// The exclusions this slice does NOT lift, kept loud rather than guessed at: a name that
/// also names a module net, and two blocks where one encloses the other. iverilog runs
/// both; vita says so instead of aliasing them.
#[test]
fn the_two_unsupported_same_name_shapes_stay_loud() {
    assert!(loud(
        "module t;\n\
           string s[2];\n\
           initial begin\n\
             begin string s[2]; s[0]=\"aa\"; $display(\"A=|%s|\", s[0]); end\n\
             begin string s[2]; s[0]=\"bb\"; $display(\"B=|%s|\", s[0]); end\n\
             $finish;\n\
           end\n\
         endmodule\n"
    ));
    assert!(loud(
        "module t;\n\
           initial begin\n\
             begin\n\
               string s[2]; s[0]=\"out\";\n\
               begin string s[2]; s[0]=\"in\"; $display(\"I=|%s|\", s[0]); end\n\
               $display(\"O=|%s|\", s[0]);\n\
             end\n\
             $finish;\n\
           end\n\
         endmodule\n"
    ));
}

/// The one place vita and iverilog differ here, pinned deliberately. An element that was
/// never assigned is the EMPTY string (IEEE 1800 §6.16 — an unset `string` is `""`).
/// iverilog prints one space for it and reports `.len()` as 2 even in a single block that
/// never wrote it, which is uninitialized memory rather than a rule, so this follows the
/// LRM. The distinctness of the two blocks' storage does NOT rest on this: iverilog's own
/// second block does not see `aa`/`bb` either.
#[test]
fn an_unassigned_element_is_the_empty_string() {
    let (o, ok) = run("module t;\n\
           string m[2];\n\
           initial begin\n\
             $display(\"M=|%s| len=%0d\", m[1], m[1].len());\n\
             begin string s[2]; s[0]=\"aa\"; s[1]=\"bb\"; $display(\"A=|%s|%s|\", s[0], s[1]); end\n\
             begin string s[2]; $display(\"B=|%s|%s|\", s[0], s[1]); end\n\
             $finish;\n\
           end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(o.contains("M=|| len=0"), "module scope, §6.16:\n{o}");
    assert!(
        o.contains("A=|aa|bb|") && o.contains("B=|||"),
        "block scope, same rule — and no leak from A:\n{o}"
    );
}

// ── §4.5.258: the same rules inside a generate scope ─────────────────────────

/// The two classifiers that decide which block-locals earn a `$blk$` scope walked
/// `module.body` for `ModuleItem::Proc` only, so a process inside a `generate` was
/// invisible to them. The whole same-name family stayed loud there while the identical
/// code at module scope worked — this was the last shape §4.5.255 did not reach.
#[test]
fn a_generate_scope_gets_the_same_name_rules_as_a_module() {
    let (o, ok) = run("module t;\n\
           generate if (1) begin : g\n\
             initial begin\n\
               begin string s[2] = '{\"a\",\"b\"}; $display(\"A=|%s|%s|\", s[0], s[1]); end\n\
               begin string s[2] = '{\"c\",\"d\"}; $display(\"B=|%s|%s|\", s[0], s[1]); end\n\
               $finish;\n\
             end\n\
           end endgenerate\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(
        o.contains("A=|a|b|") && o.contains("B=|c|d|"),
        "own storage:\n{o}"
    );

    // …and for the rest of the family, which was loud there for the same reason.
    let (o, ok) = run("module t;\n\
           generate if (1) begin : g\n\
             initial begin\n\
               begin int q[$] = '{1,2}; $display(\"C=%0d\", q.size()); end\n\
               begin int q[$] = '{7};   $display(\"D=%0d %0d\", q.size(), q[0]); end\n\
               $finish;\n\
             end\n\
           end endgenerate\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(
        o.contains("C=2") && o.contains("D=1 7"),
        "iverilog values:\n{o}"
    );
}

/// A generate-for body is ONE subtree however many times it unrolls, and each unroll
/// elaborates under its own prefix — so a name declared once inside the loop is declared
/// in one block, and only two DISTINCT blocks can collide. Each unroll keeps its own
/// storage.
#[test]
fn an_unrolled_generate_keeps_each_iterations_storage() {
    let (o, ok) = run("module t;\n\
           genvar i;\n\
           generate for (i = 0; i < 2; i = i + 1) begin : g\n\
             initial begin\n\
               begin string s[2] = '{\"a\",\"b\"}; $display(\"L%0d A=|%s|\", i, s[0]); end\n\
               begin string s[2] = '{\"c\",\"d\"}; $display(\"L%0d B=|%s|\", i, s[0]); end\n\
             end\n\
           end endgenerate\n\
           initial #1 $finish;\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    for k in 0..2 {
        assert!(
            o.contains(&format!("L{k} A=|a|")) && o.contains(&format!("L{k} B=|c|")),
            "iteration {k}:\n{o}"
        );
    }
}

/// A nested generate scope, and a module that has BOTH its own same-name pair and one
/// inside a generate — the two families must not see each other's blocks.
#[test]
fn generate_and_module_same_name_pairs_are_independent() {
    let (o, ok) = run("module t;\n\
           initial begin\n\
             begin int q[$] = '{1,2}; $display(\"M=%0d\", q.size()); end\n\
             begin int q[$] = '{7};   $display(\"N=%0d\", q.size()); end\n\
           end\n\
           generate if (1) begin : g1\n\
             if (1) begin : g2\n\
               initial begin\n\
                 begin int q[$] = '{1,2,3}; $display(\"A=%0d\", q.size()); end\n\
                 begin int q[$] = '{9};     $display(\"B=%0d\", q.size()); end\n\
               end\n\
             end\n\
           end endgenerate\n\
           initial #1 $finish;\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(
        o.contains("M=2") && o.contains("N=1") && o.contains("A=3") && o.contains("B=1"),
        "iverilog values:\n{o}"
    );
}

/// F5 (§4.5.259). A span in a nesting relation with another declaring span of the same
/// name is dropped from CANDIDACY; it is not a reason to disqualify the NAME. Globally
/// disqualifying it meant a dead `generate if (0)` arm carrying its own nested `k`
/// withdrew the scoping of a live, disjoint pair elsewhere in the design.
#[test]
fn a_nested_declaration_elsewhere_does_not_disqualify_a_disjoint_pair() {
    let live = "module t;\n\
           generate if (1) begin : live\n\
             initial begin\n\
               begin automatic int k; k = 1; $display(\"A=%0d\", k); end\n\
               begin automatic int k; k = 2; $display(\"B=%0d\", k); end\n\
             end\n\
           end endgenerate\n";
    let dead = "generate if (0) begin : dead\n\
             initial begin\n\
               begin automatic int k; k = 3;\n\
                 begin automatic int k; k = 4; $display(\"X=%0d\", k); end\n\
               end\n\
             end\n\
           end endgenerate\n";
    for src in [
        format!("{live}  initial #1 $finish;\nendmodule\n"),
        format!("{live}  {dead}  initial #1 $finish;\nendmodule\n"),
    ] {
        let (o, ok) = run(&src);
        assert!(ok, "expected clean sim, got:\n{o}");
        assert!(
            o.contains("A=1") && o.contains("B=2"),
            "live pair scoped:\n{o}"
        );
    }

    // A genuinely nested LIVE pair still has nothing to scope, so it stays loud.
    assert!(loud(
        "module t;\n\
           initial begin\n\
             begin\n\
               string s[2]; s[0]=\"out\";\n\
               begin string s[2]; s[0]=\"in\"; $display(\"I=|%s|\", s[0]); end\n\
             end\n\
             $finish;\n\
           end\n\
         endmodule\n"
    ));
}
