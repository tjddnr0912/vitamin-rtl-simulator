//! round-20 external report (2026-07-27) — the ten families it isolated with
//! self-contained repros, pinned here at the exact shapes it published.
//!
//! Two of its items were already closed by slices that landed after its base commit
//! (`1fe06e7`) and are pinned in their own files: the CRITICAL part-select bound
//! folding (`d[2*W-1:W]` collapsing to one bit) by §4.5.229 — see
//! `const_fold_bounds.rs` — and `.name()` on an enum task input formal by §4.5.234's
//! sized-literal enum labels — see `enum_sized_label.rs`. The report measured them
//! against a binary that predates both.
//!
//! Oracle note: iverilog 13.0 rejects an explicit `automatic` lifetime on a block
//! local outright ("sorry: Overriding the default variable lifetime"), so the
//! block-local families below have no live oracle and are pinned to hand-IEEE §6.21
//! plus the report's own Xcelium sign-off. Every family that CAN be run under
//! iverilog was diffed against it before being pinned; those values are noted per test.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_args(src: &str, extra: &[&str]) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_r20_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let mut c = Command::new(env!("CARGO_BIN_EXE_vita"));
    c.arg(f.to_str().unwrap());
    c.args(extra);
    let out = c.current_dir(&d).output().expect("run vita");
    let s = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let ok = out.status.code() == Some(0);
    (s, ok)
}

fn run(src: &str) -> (String, bool) {
    run_args(src, &[])
}

fn loud(src: &str) -> bool {
    let (o, ok) = run(src);
    !ok && o.contains("error[VITA-")
}

// ── §4.1 — a fork-arm block-local that is WRITTEN after its initializer ──────

/// The watchdog every testbench has: a timeout constant that `$value$plusargs` may
/// override. BL1 (§4.5.228) supported the arm only while nothing wrote the local, so
/// the plusargs override — the whole point of the idiom — kept it loud.
#[test]
fn a_watchdog_fork_arm_arms_and_fires_at_the_overridden_timeout() {
    let src = "`timescale 1ns/1ns\n\
         module t;\n\
           initial begin\n\
             fork begin : wd\n\
               automatic int unsigned tmo = 5_000_000;\n\
               void'($value$plusargs(\"SIM_TIMEOUT_NS=%d\", tmo));\n\
               $display(\"armed=%0d\", tmo);\n\
               #(tmo * 1ns);\n\
               $display(\"TIMEOUT at %0t\", $time);\n\
               $finish;\n\
             end join_none\n\
             #100 $display(\"DONE at %0t\", $time);\n\
             $finish;\n\
           end\n\
         endmodule\n";
    // Default: the watchdog is armed long, so the body finishes first.
    let (o, ok) = run(src);
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(o.contains("armed=5000000"), "the initializer ran:\n{o}");
    assert!(o.contains("DONE at 100"), "the body must win:\n{o}");
    assert!(!o.contains("TIMEOUT"), "the watchdog must not fire:\n{o}");
    // Overridden: the plusargs WRITE must be what the delay reads.
    let (o, ok) = run_args(src, &["+SIM_TIMEOUT_NS=7"]);
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(o.contains("armed=7"), "the override must land:\n{o}");
    assert!(o.contains("TIMEOUT at 7"), "and drive the delay:\n{o}");
}

/// Each arm of one fork gets its own local and its own initializer.
#[test]
fn sibling_fork_arms_each_run_their_own_initializer() {
    let (o, ok) = run("module t;\n\
           initial fork\n\
             begin automatic int a = 11; #1 $display(\"a=%0d\", a); end\n\
             begin automatic int b = 22; #1 $display(\"b=%0d\", b); end\n\
           join\n\
           initial #5 $finish;\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(o.contains("a=11") && o.contains("b=22"), "both arms:\n{o}");
}

// ── §4.2 — an `automatic string` block-local with a declaration initializer ──

/// `string` was the one scalar kind left out of the per-entry family, though its
/// re-init is the same plain `s = init` the others emit.
#[test]
fn automatic_string_block_locals_keep_their_own_initializers() {
    let (o, ok) = run("module t;\n\
           initial begin\n\
             begin automatic string s = \"a\";   $display(\"1=%s\", s); end\n\
             begin automatic string s = \"bb\";  $display(\"2=%s\", s); end\n\
             begin automatic string s = \"ccc\"; $display(\"3=%s\", s); end\n\
             $finish;\n\
           end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    for want in ["1=a", "2=bb", "3=ccc"] {
        assert!(o.contains(want), "expected `{want}`; got:\n{o}");
    }
}

/// Re-entry re-initializes (§6.21) rather than carrying the previous value.
#[test]
fn an_automatic_string_reinitializes_on_each_block_entry() {
    let (o, ok) = run("module t;\n\
           initial begin\n\
             for (int i = 0; i < 2; i++) begin\n\
               automatic string s = \"base\";\n\
               $display(\"e%0d=%s\", i, s);\n\
               s = \"dirty\";\n\
             end\n\
             $finish;\n\
           end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(
        o.contains("e0=base") && o.contains("e1=base"),
        "the second entry must NOT observe `dirty`:\n{o}"
    );
}

// ── §4.3 — queue idioms: a `'{…}` argument and a discarded pop ───────────────

/// `q.push_back('{1, 2})` — enqueueing a record by pattern. The element's packed
/// value is built from the RECEIVER's declared type, so each field lands at its own
/// width (a bare concat would size every part self-determinedly).
#[test]
fn a_struct_pattern_argument_enqueues_at_the_declared_field_widths() {
    let (o, ok) = run("module t;\n\
           typedef struct packed { logic [7:0] a; logic [3:0] b; } p_t;\n\
           p_t q [$];\n\
           initial begin\n\
             q.push_back('{8'hAB, 4'h5});\n\
             q.push_back('{8'h12, 4'hC});\n\
             q.push_front('{8'hFF, 4'h1});\n\
             $display(\"n=%0d\", q.size());\n\
             $display(\"A=%h %h\", q[0].a, q[0].b);\n\
             $display(\"B=%h %h\", q[1].a, q[1].b);\n\
             $display(\"C=%h %h\", q[2].a, q[2].b);\n\
             $finish;\n\
           end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    for want in ["n=3", "A=ff 1", "B=ab 5", "C=12 c"] {
        assert!(o.contains(want), "expected `{want}`; got:\n{o}");
    }
}

/// The report's own unpacked-struct spelling, round-tripped through a pop.
#[test]
fn an_unpacked_struct_pattern_survives_the_queue_round_trip() {
    let (o, ok) = run("module t;\n\
           typedef struct { int a; int b; } pkt_t;\n\
           pkt_t q [$]; pkt_t x;\n\
           initial begin\n\
             q.push_back('{1, 2});\n\
             q.push_back('{30, 40});\n\
             x = q.pop_front(); $display(\"X=%0d %0d\", x.a, x.b);\n\
             x = q.pop_front(); $display(\"Y=%0d %0d\", x.a, x.b);\n\
             $finish;\n\
           end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(o.contains("X=1 2") && o.contains("Y=30 40"), "values:\n{o}");
}

/// `void'(q.pop_front());` — pop for the side effect. Pinned to LIVE iverilog 13.0,
/// which prints the same `2 22 33` / `1 22`.
#[test]
fn a_discarded_pop_still_removes_the_element() {
    let (o, ok) = run("module t;\n\
           int q [$];\n\
           initial begin\n\
             q.push_back(11); q.push_back(22); q.push_back(33);\n\
             void'(q.pop_front());\n\
             $display(\"A=%0d %0d %0d\", q.size(), q[0], q[1]);\n\
             void'(q.pop_back());\n\
             $display(\"B=%0d %0d\", q.size(), q[0]);\n\
             $finish;\n\
           end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(
        o.contains("A=2 22 33") && o.contains("B=1 22"),
        "pops:\n{o}"
    );
}

/// A pop with arguments is still a real mistake — and its message now says so,
/// instead of the old "must be assigned" that no longer describes anything.
#[test]
fn a_pop_with_arguments_stays_loud() {
    assert!(loud(
        "module t; int q[$];\n\
           initial begin q.push_back(1); void'(q.pop_front(3)); $finish; end\n\
         endmodule\n"
    ));
}

// ── §4.4 — a container method must not abort the definite-assignment walk ────

/// The report's headline MISDIAGNOSIS: `q.delete()` in an `if` arm stopped the walk,
/// and the error named `d` — assigned two tokens earlier — instead of the queue.
#[test]
fn a_container_method_beside_an_assignment_is_not_a_read_of_that_variable() {
    let (o, ok) = run("module t;\n\
           int q [$]; logic sel = 1;\n\
           initial begin\n\
             automatic int d;\n\
             if (sel) begin d = 0; q.delete(); end\n\
             else       d = 1;\n\
             $display(\"d=%0d\", d);\n\
             $finish;\n\
           end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(o.contains("d=0"), "the then-arm assignment:\n{o}");
}

/// The walk must still reject a genuine read-before-write — the call being present
/// changes nothing about that.
#[test]
fn a_real_read_before_write_beside_a_container_method_stays_loud() {
    assert!(loud(
        "module t;\n\
           int q [$]; logic sel = 1;\n\
           initial begin\n\
             automatic int d;\n\
             if (sel) begin q.delete(); $display(\"%0d\", d); end\n\
             else       d = 1;\n\
             $finish;\n\
           end\n\
         endmodule\n"
    ));
}

/// A task enable that WRITES the local through an output actual is still seen as a
/// write (BL4) — vetting `UserTaskCall` in the ref-free walker must not shadow that.
#[test]
fn an_output_actual_write_is_still_a_definite_assignment() {
    let (o, ok) = run("module t;\n\
           int q [$];\n\
           task automatic setit (output int o); o = 42; endtask\n\
           initial begin\n\
             automatic int d;\n\
             q.delete();\n\
             setit(d);\n\
             $display(\"d=%0d\", d);\n\
             $finish;\n\
           end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(o.contains("d=42"), "output-actual write:\n{o}");
}

// ── §4.5 — a `'{…}` actual bound to a dynamic-array formal ──────────────────

/// `run_scenario('{}, msg, exp, …)` — 68 sites in the report's testbench. Pinned to
/// LIVE iverilog 13.0 for the empty case (`R=0/2`); iverilog aborts on the non-empty
/// one (its own assertion failure), so those are hand-IEEE §10.9.2.
#[test]
fn a_pattern_actual_materializes_for_a_dynamic_array_formal() {
    let (o, ok) = run("module t;\n\
           task automatic show (input byte k [], input byte m []);\n\
             $display(\"R=%0d/%0d\", k.size(), m.size());\n\
           endtask\n\
           function automatic int ksz (input byte k []); return k.size(); endfunction\n\
           byte msg [] = '{8'h00, 8'h01};\n\
           int r;\n\
           initial begin\n\
             show('{}, msg);\n\
             show('{8'hAA, 8'hBB, 8'hCC}, msg);\n\
             show(msg, '{});\n\
             show('{}, '{8'h9});\n\
             r = ksz('{});           $display(\"F0=%0d\", r);\n\
             r = ksz('{8'h1, 8'h2}); $display(\"F2=%0d\", r);\n\
             $finish;\n\
           end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    for want in ["R=0/2", "R=3/2", "R=2/0", "R=0/1", "F0=0", "F2=2"] {
        assert!(o.contains(want), "expected `{want}`; got:\n{o}");
    }
}

/// The materialized elements must be the ones written, not just the right count.
#[test]
fn a_materialized_pattern_actual_carries_its_element_values() {
    let (o, ok) = run("module t;\n\
           task automatic dump (input byte k []);\n\
             foreach (k[i]) $display(\"k%0d=%h\", i, k[i]);\n\
           endtask\n\
           initial begin dump('{8'hDE, 8'hAD, 8'hBE}); $finish; end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    for want in ["k0=de", "k1=ad", "k2=be"] {
        assert!(o.contains(want), "expected `{want}`; got:\n{o}");
    }
}

// ── §4.6 — a statement-level whole-value `'{…}` assignment ───────────────────

/// The asymmetry the report named: the same pattern worked as a DECLARATION
/// initializer and was doubly loud as a statement. Every line below was diffed
/// against LIVE iverilog 13.0 and matches.
#[test]
fn a_whole_value_pattern_assignment_matches_the_declaration_initializer() {
    let (o, ok) = run("module t;\n\
           byte e []; int q [$];\n\
           initial begin\n\
             e = '{8'h0a, 8'h0b}; $display(\"A=%0d %h %h\", e.size(), e[0], e[1]);\n\
             e = '{8'h01};        $display(\"B=%0d %h\", e.size(), e[0]);\n\
             e = '{};             $display(\"C=%0d\", e.size());\n\
             q = '{1, 2, 3};      $display(\"D=%0d %0d %0d\", q.size(), q[0], q[2]);\n\
             q = '{9};            $display(\"E=%0d %0d\", q.size(), q[0]);\n\
             q = '{};             $display(\"F=%0d\", q.size());\n\
             q = {4, 5};          $display(\"G=%0d %0d %0d\", q.size(), q[0], q[1]);\n\
             $finish;\n\
           end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    for want in [
        "A=2 0a 0b",
        "B=1 01",
        "C=0",
        "D=3 1 3",
        "E=1 9",
        "F=0",
        "G=2 4 5",
    ] {
        assert!(o.contains(want), "expected `{want}`; got:\n{o}");
    }
}

/// An assignment REPLACES — a queue must be cleared first, not appended to. The
/// expansion push_backs, so getting this wrong accumulates silently.
#[test]
fn a_pattern_assignment_replaces_a_queue_rather_than_appending() {
    let (o, ok) = run("module t;\n\
           int q [$] = '{7, 8, 9};\n\
           initial begin\n\
             q = '{1, 2};\n\
             $display(\"R=%0d %0d %0d\", q.size(), q[0], q[1]);\n\
             $finish;\n\
           end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(o.contains("R=2 1 2"), "replace, not append:\n{o}");
}

// ── §4.8 — named arguments, and the misdiagnosis they cascaded into ──────────

/// A named argument in a TASK enable. The old message said it "is only valid in a
/// user function / task call" while standing inside one. Pinned to LIVE iverilog.
#[test]
fn a_task_enable_accepts_named_arguments() {
    let (o, ok) = run("module t;\n\
           task automatic show (input string s, input int n = 0); $display(\"R=%s %0d\", s, n); endtask\n\
           initial begin\n\
             show(\"x\", .n(32));\n\
             show(.s(\"y\"), .n(7));\n\
             show(.n(5), .s(\"z\"));\n\
             show(\"w\");\n\
             $finish;\n\
           end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    for want in ["R=x 32", "R=y 7", "R=z 5", "R=w 0"] {
        assert!(o.contains(want), "expected `{want}`; got:\n{o}");
    }
}

/// The CASCADE: `r = add(1, .b(2));` was reported as "block-local `r` … read before
/// its first write". Named arguments already worked in function calls — what did not
/// work was the conservative reference walker, which had no `NamedArg` arm and so
/// answered "this rhs may reference `r`" about a clean whole-variable write.
#[test]
fn a_named_argument_does_not_make_the_assignment_look_like_a_read() {
    let (o, ok) = run("module t;\n\
           function automatic int add (input int a, input int b = 0); return a + b; endfunction\n\
           initial begin\n\
             automatic int r;\n\
             r = add(1, .b(2));\n\
             $display(\"r=%0d\", r);\n\
             $finish;\n\
           end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(o.contains("r=3"), "named-arg call result:\n{o}");
}

/// The named-arg errors that ARE errors keep firing.
#[test]
fn malformed_named_arguments_stay_loud() {
    for src in [
        // no such formal
        "module t; task automatic s(input int a); $display(\"%0d\",a); endtask\n\
           initial begin s(.zz(1)); $finish; end endmodule\n",
        // bound twice
        "module t; task automatic s(input int a); $display(\"%0d\",a); endtask\n\
           initial begin s(.a(1), .a(2)); $finish; end endmodule\n",
        // positional after named
        "module t; task automatic s(input int a, input int b); $display(\"%0d\",a+b); endtask\n\
           initial begin s(.a(1), 2); $finish; end endmodule\n",
    ] {
        assert!(loud(src), "should be loud:\n{src}");
    }
}

// ── §4.9 — `new[N]` as a declaration initializer ────────────────────────────

/// One declaration produced EIGHT diagnostics, two of them "undeclared `t.aa_key`"
/// for a name declared on the line above — the reject `continue`d past the handle
/// registration. `new[n]` needs no expansion at all; the flush emits the very
/// statement the split-decl workaround wrote by hand.
#[test]
fn new_bracket_n_works_as_a_declaration_initializer() {
    let (o, ok) = run("module t;\n\
           initial begin\n\
             begin\n\
               automatic byte aa_key [] = new[20];\n\
               foreach (aa_key[i]) aa_key[i] = 8'hAA;\n\
               $display(\"R=%0d %h\", aa_key.size(), aa_key[3]);\n\
             end\n\
             $finish;\n\
           end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(o.contains("R=20 aa"), "size and element:\n{o}");
}

/// Module scope takes the same initializer, and re-entry re-allocates (so a stale
/// size can never leak between entries).
#[test]
fn a_new_bracket_n_initializer_reallocates_on_each_entry() {
    let (o, ok) = run("module t;\n\
           byte g [] = new[4];\n\
           initial begin\n\
             $display(\"G=%0d\", g.size());\n\
             for (int i = 0; i < 2; i++) begin\n\
               automatic byte d [] = new[3];\n\
               $display(\"e%0d=%0d\", i, d.size());\n\
               d = new[9];\n\
             end\n\
             $finish;\n\
           end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(o.contains("G=4"), "module-scope new[]:\n{o}");
    assert!(
        o.contains("e0=3") && o.contains("e1=3"),
        "the second entry must NOT see the 9:\n{o}"
    );
}

/// `new[n]` still belongs only to a dynamic ARRAY — a queue keeps its loud.
#[test]
fn new_bracket_n_on_a_queue_declaration_stays_loud() {
    assert!(loud(
        "module t; int q [$] = new[4];\n\
           initial begin $display(\"%0d\", q.size()); $finish; end endmodule\n"
    ));
}

// ── §4.10 — `$sformatf` outside a direct blocking-assign rhs ─────────────────

/// The three placements the report named, plus the accumulator loop that is the real
/// idiom. Every one below except the NBA (iverilog rejects a `string` NBA) was diffed
/// against LIVE iverilog 13.0 and matches.
#[test]
fn sformatf_renders_in_every_unconditionally_evaluated_position() {
    let (o, ok) = run("module t;\n\
           task automatic sh (input string s); $display(\"T=%s\", s); endtask\n\
           function automatic int ln (input string s); return s.len(); endfunction\n\
           string u, q; byte b [3];\n\
           initial begin\n\
             int n = 3;\n\
             sh($sformatf(\"n=%0d\", n));\n\
             q <= $sformatf(\"nba%0d\", 7);\n\
             u = {\"x\", $sformatf(\"%0d\", n)};                      $display(\"C=%s\", u);\n\
             u = {$sformatf(\"a%0d\",1), \"-\", $sformatf(\"b%0d\",2)}; $display(\"D=%s\", u);\n\
             $display(\"E=%0d\", ln($sformatf(\"abcd\")));\n\
             b = '{8'h1a, 8'h2b, 8'h3c};\n\
             u = \"\";\n\
             for (int i = 0; i < 3; i++) u = {u, $sformatf(\"%02x\", b[i])};\n\
             $display(\"F=%s\", u);\n\
             $display(\"H=%s|%0d\", $sformatf(\"z%0d\",9), ln($sformatf(\"qq\")));\n\
             #1 $display(\"G=%s\", q);\n\
             $finish;\n\
           end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    for want in [
        "T=n=3", "C=x3", "D=a1-b2", "E=4", "F=1a2b3c", "H=z9|2", "G=nba7",
    ] {
        assert!(o.contains(want), "expected `{want}`; got:\n{o}");
    }
}

/// The hoist must not change HOW MANY times the render happens. A ternary ARM is
/// conditionally evaluated, so it is deliberately NOT descended into — it keeps its
/// existing loud rather than becoming a render on both branches.
#[test]
fn a_sformatf_in_a_ternary_arm_stays_loud() {
    assert!(loud(
        "module t; string u; logic c = 1;\n\
           initial begin u = c ? $sformatf(\"a\") : $sformatf(\"b\"); $display(\"%s\", u); $finish; end\n\
         endmodule\n"
    ));
}

/// A hoisted render is evaluated where the statement is, so a loop body re-renders
/// per iteration rather than freezing the first value.
#[test]
fn a_hoisted_sformatf_reevaluates_each_iteration() {
    let (o, ok) = run("module t;\n\
           task automatic sh (input string s); $display(\"R=%s\", s); endtask\n\
           initial begin\n\
             for (int i = 0; i < 3; i++) sh($sformatf(\"i%0d\", i));\n\
             $finish;\n\
           end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    for want in ["R=i0", "R=i1", "R=i2"] {
        assert!(o.contains(want), "expected `{want}`; got:\n{o}");
    }
}
