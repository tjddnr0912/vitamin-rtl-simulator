//! §4.5.250 — the adversarial review of §4.5.248/249, and what it found.
//!
//! Two lenses ran over those commits: a DIFFERENTIAL one (PRE binary vs POST vs live
//! iverilog 13.0) and a SOUNDNESS one (code-path reading against IEEE 1800). Between
//! them they found FIVE ladder descents — cases the slices had turned from an honest
//! loud into a silent wrong — plus one plain regression. Every one is pinned here at
//! the shape that exposed it, because each is a rule the next slice must not break
//! again.
//!
//! The recurring shape is worth naming: every descent came from moving evaluation.
//! The `$sformatf` hoist moves a render EARLIER, and the `'{…}` expansion moves a
//! clear BEFORE its own elements. "It is pure, so moving it is free" was the wrong
//! frame — what matters is how many times it runs, when relative to its siblings, and
//! what it reads while it runs.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_r20rv_{}_{n}", std::process::id()));
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
    !ok && o.contains("error[VITA-")
}

// ── the hoist must not change HOW MANY times a render happens ────────────────

/// `$monitor` re-displays whenever an argument changes (§21.2.3) and `$strobe` renders
/// at the END of the time step (§21.2.2). A hoisted render happens ONCE, at the
/// statement — so the monitor watched a frozen temp and printed only at t=0, and the
/// strobe reported the value from before the rest of the time step. Both were loud
/// before the hoist existed, and both are loud again.
#[test]
fn the_deferred_print_tasks_are_not_hoisted_into() {
    assert!(loud(
        "module t;\n\
           int a = 0;\n\
           initial begin\n\
             $monitor(\"t=%0t s=%s\", $time, {\"[\", $sformatf(\"%0d\", a), \"]\"});\n\
             #1 a = 1; #1 a = 2; #1 $finish;\n\
           end\n\
         endmodule\n"
    ));
    assert!(loud(
        "module t;\n\
           int a = 0;\n\
           initial begin\n\
             a = 1;\n\
             $strobe(\"s=%s\", {\"[\", $sformatf(\"%0d\", a), \"]\"});\n\
             a = 2;\n\
             #1 $finish;\n\
           end\n\
         endmodule\n"
    ));
    // The IMMEDIATE twin still renders — the exclusion is exactly the deferred family.
    let (o, ok) = run("module t;\n\
           int a = 3;\n\
           initial begin $display(\"D=%s\", {\"[\", $sformatf(\"%0d\", a), \"]\"}); $finish; end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(o.contains("D=[3]"), "immediate task still hoists:\n{o}");
}

/// `&&` / `||` may skip their right operand (§11.4.7). `$sformatf` is pure, but its
/// ARGUMENTS are not: hoisting out of a short-circuited operand made
/// `c && (s == $sformatf("%0d", $random))` advance the random seed with `c` false —
/// caught against live iverilog, which leaves the seed untouched.
#[test]
fn a_short_circuited_operand_is_not_hoisted_out_of() {
    assert!(loud(
        "module t;\n\
           int c, r; string s;\n\
           initial begin\n\
             c = 0; s = \"zz\";\n\
             r = c && (s == $sformatf(\"%0d\", $random));\n\
             $display(\"%0d\", r);\n\
             $finish;\n\
           end\n\
         endmodule\n"
    ));
    assert!(loud(
        "module t;\n\
           int c, r; string s;\n\
           initial begin\n\
             c = 1; s = \"zz\";\n\
             r = c || (s == $sformatf(\"%0d\", $random));\n\
             $display(\"%0d\", r);\n\
             $finish;\n\
           end\n\
         endmodule\n"
    ));
    // A NON-short-circuit operator evaluates both sides, so it still hoists.
    let (o, ok) = run("module t;\n\
           int r; string s;\n\
           initial begin s = \"7\"; r = (s == $sformatf(\"%0d\", 7)); $display(\"R=%0d\", r); $finish; end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(o.contains("R=1"), "`==` still hoists:\n{o}");
}

/// A replication evaluates its value `count` times — ZERO for `{0{…}}`. Not "exactly
/// as many times as before" for any count but one, so it is not descended into.
#[test]
fn a_replication_value_is_not_hoisted_out_of() {
    assert!(loud(
        "module t;\n\
           string u;\n\
           initial begin u = {2{$sformatf(\"a\")}}; $display(\"%s\", u); $finish; end\n\
         endmodule\n"
    ));
}

// ── the hoist must not change the ORDER of sibling evaluations ───────────────

/// The hoist moves a render ahead of everything to its left, so everything to its left
/// must be inert. `show($urandom, {"<", $sformatf("%0d", $urandom), ">"})` gave
/// argument 1 the SECOND draw and the format the FIRST — the two arguments swapped
/// values, against live iverilog's left-to-right order.
#[test]
fn a_render_is_not_hoisted_past_a_side_effecting_sibling() {
    assert!(loud(
        "module t;\n\
           task automatic show (int a, string b); $display(\"%0d %s\", a, b); endtask\n\
           initial begin show($urandom, {\"<\", $sformatf(\"%0d\", $urandom), \">\"}); $finish; end\n\
         endmodule\n"
    ));
    // An INERT left neighbour keeps the hoist — this is the common case and must not
    // be collateral damage.
    let (o, ok) = run("module t;\n\
           task automatic show (int a, string b); $display(\"R=%0d %s\", a, b); endtask\n\
           int k = 4;\n\
           initial begin show(k + 1, {\"<\", $sformatf(\"%0d\", k), \">\"}); $finish; end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(o.contains("R=5 <4>"), "inert neighbour still hoists:\n{o}");
}

// ── a multi-name declaration is scoped as a WHOLE ────────────────────────────

/// Review F1 was a SILENT ZERO: `$blk$` scoping is decided per DECLARATION, so asking
/// the "no initializer" exclusion per NAME let a qualifying `m` drag its init-bearing
/// sibling onto the scoped arm, which returned before the decl-init collector.
///
/// §4.5.251 removed the exclusion entirely by making the scoped path collect its own
/// initializers, so this shape is now CORRECT rather than merely loud — the stronger
/// guarantee, and the one pinned here against live iverilog 13.0.
#[test]
fn a_multi_name_declaration_keeps_every_initializer() {
    let (o, ok) = run("module t;\n\
           byte g = 8'd9;\n\
           initial begin\n\
             begin byte m[], n = g; m = new[2]; $display(\"A=%0d %0d\", m.size(), n); end\n\
             begin byte m[], k = g; m = new[3]; $display(\"B=%0d %0d\", m.size(), k); end\n\
             $finish;\n\
           end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(
        o.contains("A=2 9") && o.contains("B=3 9"),
        "iverilog A=2 9 B=3 9:\n{o}"
    );

    let (o, ok) = run("module t;\n\
           initial begin\n\
             begin int m[], n[] = '{1,2,3}; m = new[1]; $display(\"A=%0d %0d\", m.size(), n.size()); end\n\
             begin int m[]; m = new[2]; $display(\"B=%0d\", m.size()); end\n\
             $finish;\n\
           end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(
        o.contains("A=1 3") && o.contains("B=2"),
        "iverilog A=1 3 B=2:\n{o}"
    );
}

// ── an empty `'{}` actual is an INPUT-formal shorthand ───────────────────────

/// §13.5.2 requires an `output`/`inout` actual to be an lvalue, which `'{}` is not.
/// Accepting it silently discarded the copy-out — and because one temp net is minted
/// per CALL SITE, a call in a loop then observed the PREVIOUS activation's write
/// (`size=0`, `1`, `2` where every call should see the same thing).
#[test]
fn an_empty_pattern_actual_is_rejected_on_an_output_formal() {
    assert!(loud(
        "module t;\n\
           int i;\n\
           task automatic acc (inout byte o []);\n\
             int n; n = o.size(); $display(\"S=%0d\", n); o = new[n + 1];\n\
           endtask\n\
           initial begin for (i = 0; i < 3; i++) acc('{}); $finish; end\n\
         endmodule\n"
    ));
    assert!(loud(
        "module t;\n\
           task automatic fill (output byte o []); o = new[2]; endtask\n\
           initial begin fill('{}); $finish; end\n\
         endmodule\n"
    ));
    // The INPUT form is the one this shorthand is for, and it stays supported.
    let (o, ok) = run("module t;\n\
           task automatic show (input byte k []); $display(\"R=%0d\", k.size()); endtask\n\
           initial begin for (int i = 0; i < 3; i++) show('{}); $finish; end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert_eq!(
        o.matches("R=0").count(),
        3,
        "every activation sees an empty array:\n{o}"
    );
}

// ── a whole-value pattern reads its target BEFORE the clear ──────────────────

/// A queue assignment clears first (assignment replaces; `push_back` appends), so an
/// element that reads the TARGET read the emptied queue and came back X / 0. The
/// elements are snapshotted into temps before the clear, in source order.
///
/// DELIBERATE DIVERGENCE on the swap: iverilog writes the elements in place with no
/// snapshot, so it prints `6 6` for `q = '{q[1], q[0]}`. §10.7 evaluates the whole
/// right-hand side before assigning, which makes a swap a swap — every other line here
/// matches iverilog, and this one is where iverilog contradicts itself.
#[test]
fn a_self_referential_pattern_assignment_reads_the_old_queue() {
    let (o, ok) = run("module t;\n\
           int q [$];\n\
           initial begin\n\
             q.push_back(3); q.push_back(4);\n\
             q = '{q[0], 9};\n\
             $display(\"A=%0d %0d %0d\", q.size(), q[0], q[1]);\n\
             $finish;\n\
           end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(
        o.contains("A=2 3 9"),
        "element read (iverilog: 2 3 9):\n{o}"
    );

    let (o, ok) = run("module t;\n\
           int q [$];\n\
           initial begin\n\
             q.push_back(3);\n\
             q = '{q.size(), 9};\n\
             $display(\"B=%0d %0d %0d\", q.size(), q[0], q[1]);\n\
             $finish;\n\
           end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(o.contains("B=2 1 9"), "size() read (iverilog: 2 1 9):\n{o}");

    let (o, ok) = run("module t;\n\
           int q [$];\n\
           initial begin\n\
             q.push_back(5); q.push_back(6);\n\
             q = '{q[1], q[0]};\n\
             $display(\"C=%0d %0d %0d\", q.size(), q[0], q[1]);\n\
             $finish;\n\
           end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(o.contains("C=2 6 5"), "a swap is a swap (§10.7):\n{o}");

    // A pattern that does NOT name the target keeps the plain two-statement expansion.
    let (o, ok) = run("module t;\n\
           int q [$] = '{7, 8};\n\
           initial begin q = '{1, 2}; $display(\"D=%0d %0d %0d\", q.size(), q[0], q[1]); $finish; end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(o.contains("D=2 1 2"), "plain replace:\n{o}");
}

// ── a REJECT gate needs a POSITIVE walker ────────────────────────────────────

/// The static-initializer gate decided to REJECT using `expr_no_ref`, whose "unknown ⇒
/// may reference" answer is right for an ACCEPT gate and inverted here: every
/// initializer it had not vetted — `pkg::PARAM`, a time literal, `new()` — was
/// rejected, with a message naming a variable the initializer never mentions.
#[test]
fn the_static_init_gate_does_not_reject_what_it_cannot_read() {
    for init in ["p::K", "10ns", "$clog2(16)"] {
        let (o, ok) = run(&format!(
            "package p; parameter int K = 3; endpackage\n\
             module t;\n\
               initial begin\n\
                 begin\n\
                   automatic int c = 5;\n\
                   int z = {init};\n\
                   $display(\"R=%0d\", c);\n\
                 end\n\
                 $finish;\n\
               end\n\
             endmodule\n"
        ));
        assert!(ok, "`{init}` must not be rejected; got:\n{o}");
        assert!(o.contains("R=5"), "`{init}`:\n{o}");
    }
    // The real hazard still fires — the fix is the polarity, not the rule.
    assert!(loud(
        "module t;\n\
           function automatic int f (inout int io); io = io + 1; return io; endfunction\n\
           initial begin\n\
             begin automatic int c = 5; int z = f(c); $display(\"%0d %0d\", z, c); end\n\
             $finish;\n\
           end\n\
         endmodule\n"
    ));
}

/// …and when it does fire it points at the declaration, like every other elaborate
/// diagnostic. It runs at block level, before the per-declaration anchor, so it has to
/// carry the span itself.
#[test]
fn the_static_init_gate_points_at_the_declaration() {
    let (o, _) = run("module t;\n\
         function automatic int f (inout int io); io = io + 1; return io; endfunction\n\
         initial begin\n\
           begin\n\
             automatic int c = 5;\n\
             int z = f(c);\n\
             $display(\"%0d\", z);\n\
           end\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert!(o.contains("t.sv:6:"), "the declaration is line 6:\n{o}");
}

// ── the DA walk's new arm is a CONTAINER-METHOD arm ──────────────────────────

/// v1 publishes block-locals as MODULE nets, so a callee body can name the flattened
/// bare name and read it with neither the callee head nor an argument mentioning it.
/// "A task writes a caller variable only through an output actual" is an IEEE scoping
/// argument, and the flatten is exactly where IEEE scoping does not hold — so a plain
/// single-segment enable stays unvetted. Only the 2-segment container-method form,
/// which §4.4 needed, is.
#[test]
fn a_plain_task_enable_does_not_vouch_for_a_block_local() {
    assert!(loud(
        "module t;\n\
           int i;\n\
           task show; $display(\"V=%0d\", v); endtask\n\
           initial begin\n\
             for (i = 0; i < 2; i++) begin automatic int v; show(); v = i + 10; end\n\
             $finish;\n\
           end\n\
         endmodule\n"
    ));
    // The container-method form §4.4 is about still works.
    let (o, ok) = run("module t;\n\
           int q [$]; logic sel = 1;\n\
           initial begin\n\
             automatic int d;\n\
             if (sel) begin d = 0; q.delete(); end else d = 1;\n\
             $display(\"R=%0d\", d);\n\
             $finish;\n\
           end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(o.contains("R=0"), "container method:\n{o}");
}

// ── elaborator temps stay out of the user namespace ──────────────────────────

/// Every other elaborate temp is `$`-fenced precisely so it cannot collide with a user
/// identifier or appear in a dump. The pop-discard sink was not, so it was DUMPED into
/// the VCD and false-louded against a legal user declaration of the same name.
#[test]
fn the_pop_discard_sink_is_not_in_the_user_namespace() {
    let (o, ok) = run("module t;\n\
           reg [31:0] __popsink_2;\n\
           int q [$];\n\
           initial begin\n\
             __popsink_2 = 1;\n\
             q.push_back(7); q.push_back(8);\n\
             void'(q.pop_front());\n\
             $display(\"R=%0d %0d\", q.size(), __popsink_2);\n\
             $finish;\n\
           end\n\
         endmodule\n");
    assert!(ok, "a user name must not collide with a temp:\n{o}");
    assert!(o.contains("R=1 1"), "both survive:\n{o}");
}
