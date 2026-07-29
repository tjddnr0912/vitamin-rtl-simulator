//! Round-17 report: the definite-assignment walk's three false-loud families, the
//! give-up note that made the third one findable, and the two silent-wrongs the
//! oracling turned up.
//!
//! Oracle note: iverilog rejects an explicit `automatic` lifetime override ("sorry:
//! Overriding the default variable lifetime"), so every differential quoted here was
//! measured on the twin program whose locals sit un-keyworded inside a `task
//! automatic`, where IEEE 1800 §6.21 makes them automatic anyway. Where a chained
//! method call is involved iverilog cannot parse the construct at all (§8.13 chaining
//! is legal SystemVerilog), so those are pinned hand-IEEE with a decomposed
//! two-statement oracle for the value.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Returns (combined stdout+stderr, process_success).
fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_r17chain_{}_{n}", std::process::id()));
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
        out.status.success(),
    )
}

fn ok(src: &str) -> String {
    let (o, good) = run(src);
    assert!(good, "expected clean run:\n{o}");
    o
}

fn loud(src: &str) -> String {
    let (o, good) = run(src);
    assert!(!good && o.contains("E3009"), "expected E3009:\n{o}");
    o
}

// ── §3.1 CHAIN: a chained method call no longer ends the walk ────────────────

#[test]
fn chained_method_call_does_not_abort_the_walk() {
    // The report's §3.1 verbatim. `v` is assigned on the very line the chain appears
    // on and `fd` on the next; both were reported as read-before-write because
    // `expr_no_ref` had no `MethodCall` arm and answered "may reference `<anything>`".
    let o = ok("module t;\n\
         initial begin\n\
           begin\n\
             automatic string line = \"[L = 32]\";\n\
             automatic int    v;\n\
             automatic int    fd;\n\
             v  = line.substr(4, 5).atoi();\n\
             fd = 7;\n\
             if (fd == 7) $display(\"PASS v=%0d\", v);\n\
           end\n\
           $finish;\n\
         end\n\
         endmodule");
    // `"[L = 32]".substr(4,5)` is `" 3"`, and IEEE §6.16.9 `atoi` takes only LEADING
    // digits — a space is not one — so the value is 0. (iverilog cannot parse the
    // chain; the decomposed twin `s2 = line.substr(4,5); v = s2.atoi();` prints 0.)
    assert!(o.contains("PASS v=0"), "{o}");
}

#[test]
fn chain_tail_variants_and_condition_position() {
    // The report's measured boundary list: the tail method does not matter, and a
    // chain in an `if` CONDITION behaves like one in an assignment.
    let o = ok("module t;\n\
         initial begin\n\
           begin\n\
             automatic string line = \"abc123\";\n\
             automatic int    n;\n\
             automatic int    m;\n\
             n = line.substr(3, 5).len();\n\
             if (line.substr(0, 2).len() == 3) m = 1; else m = 2;\n\
             $display(\"PASS n=%0d m=%0d\", n, m);\n\
           end\n\
           $finish;\n\
         end\n\
         endmodule");
    assert!(o.contains("PASS n=3 m=1"), "{o}");
}

#[test]
fn chain_inside_a_callee_body_does_not_blame_the_caller() {
    // §3.1b: the chain is in `hdr`'s body; the diagnostic named `fd`, a local in the
    // CALLER — in the real report, in a different file. The callee-inertness walk
    // shares the same expression walker, so one arm fixed both.
    let o = ok("module t;\n\
         task automatic hdr (input string p, output int a);\n\
           automatic string line = \"[L = 32]\";\n\
           a = 0;\n\
           a = line.substr(4, 5).atoi();\n\
         endtask\n\
         initial begin\n\
           begin\n\
             automatic int lo;\n\
             automatic int fd;\n\
             hdr(\"p\", lo);\n\
             fd = 7;\n\
             if (fd == 7) $display(\"PASS\");\n\
           end\n\
           $finish;\n\
         end\n\
         endmodule");
    assert!(o.contains("PASS"), "{o}");
}

// ── §3.2 UNWRITTEN: a local with no writer is byte-identical ─────────────────

#[test]
fn deliberately_unwritten_dyn_array_as_an_input_actual() {
    // The report's §3.2 verbatim. An unassigned dynamic array is size 0 (IEEE §7.5)
    // and passing it by value is legal; there is no "first write" for the read to be
    // before. iverilog on the `task automatic` twin prints the same line.
    let o = ok("module t;\n\
         task automatic go (input string nm, input byte m [], input byte e []);\n\
           $display(\"%s m=%0d e=%0d\", nm, m.size(), e.size());\n\
         endtask\n\
         initial begin\n\
           begin\n\
             automatic byte msg[] = '{8'h61};\n\
             automatic byte exp[];\n\
             go(\"D11\", msg, exp);\n\
           end\n\
           $display(\"PASS\");\n\
           $finish;\n\
         end\n\
         endmodule");
    assert!(o.contains("D11 m=1 e=0"), "{o}");
}

#[test]
fn an_output_actual_is_a_writer_and_keeps_the_local_loud() {
    // The boundary that makes the rule safe rather than a blanket accept: the same
    // program with `exp` at an OUTPUT formal has a writer, so the flattened net can
    // carry a value into the next entry and the read is a genuine leftover read.
    loud(
        "module t;\n\
         task automatic fill (output byte e []);\n\
           e = new[2];\n\
         endtask\n\
         task automatic go (input byte e []);\n\
           $display(\"e=%0d\", e.size());\n\
         endtask\n\
         initial begin\n\
           begin\n\
             automatic byte exp[];\n\
             go(exp);\n\
             fill(exp);\n\
           end\n\
           $finish;\n\
         end\n\
         endmodule",
    );
}

#[test]
fn a_callee_that_pokes_the_flattened_net_is_a_writer() {
    // The reason the never-written proof consults the callee resolver instead of the
    // signature-blind walker: v1 publishes block-locals as MODULE nets, so a task
    // that names none of them in its actuals can still write one through a
    // hierarchical self-path. If that were counted as "no write", the accept would be
    // a silent-wrong rather than a widening.
    loud(
        "module t;\n\
         task automatic poke; t.a = 99; endtask\n\
         initial begin\n\
           begin\n\
             automatic int a;\n\
             poke();\n\
             $display(\"a=%0d\", a);\n\
           end\n\
           $finish;\n\
         end\n\
         endmodule",
    );
}

// ── §3.3 root: the walk no longer forgets that the local is already written ──

#[test]
fn an_unmodelled_statement_after_the_write_is_harmless() {
    // The reporter's in-situ facts 2 and 3: a dummy write BEFORE the loop cleared the
    // diagnostic, the same write INSIDE the loop did not. The cause was the walk's
    // catch-all rejecting any unmodelled statement outright, ignoring that the local
    // was already definitely assigned on that path — a state the top-level scan
    // checked between statements but no nested construct did.
    let o = ok("module t;\n\
         int c = 1;\n\
         initial begin\n\
           begin\n\
             automatic int x;\n\
             while (c) begin\n\
               x = 5;\n\
               #1 x = x + 1;\n\
               $display(\"R %0d\", x);\n\
               c = 0;\n\
             end\n\
           end\n\
           $finish;\n\
         end\n\
         endmodule");
    assert!(o.contains("R 6"), "{o}");
}

#[test]
fn timing_controlled_first_writes() {
    // A delay-, clock-, intra-assignment- or wait-aligned FIRST write is still a
    // blocking write: the process does not go on until it has happened. All four fell
    // to the catch-all. Values verified byte-identical against iverilog on the `task
    // automatic` twin.
    let o = ok("module t;\n\
         logic clk = 0; always #5 clk = ~clk;\n\
         int gate = 0;\n\
         initial begin\n\
           begin automatic int a; #1 a = 7;               $display(\"A %0d\", a); end\n\
           begin automatic int b; @(posedge clk) b = 9;   $display(\"B %0d\", b); end\n\
           begin automatic int c; c = #1 11;              $display(\"C %0d\", c); end\n\
           begin automatic int d; #1 begin d = 13; $display(\"D %0d\", d); end end\n\
           begin automatic int e; wait (gate == 0) e = 15; $display(\"E %0d\", e); end\n\
           $finish;\n\
         end\n\
         endmodule");
    for want in ["A 7", "B 9", "C 11", "D 13", "E 15"] {
        assert!(o.contains(want), "missing {want}:\n{o}");
    }
}

#[test]
fn a_delay_prefix_that_reads_the_local_stays_loud() {
    // The boundary for the arm above: the timing PREFIX is evaluated before the
    // write, so a prefix that reads the still-unwritten local is a real
    // read-before-write and must stay loud.
    loud(
        "module t;\n\
         initial begin\n\
           begin\n\
             automatic int n;\n\
             #(n) n = 3;\n\
             $display(\"n=%0d\", n);\n\
           end\n\
           $finish;\n\
         end\n\
         endmodule",
    );
}

// ── §3.3 / §4.2: the give-up note ───────────────────────────────────────────

#[test]
fn the_note_points_at_the_construct_that_ended_the_scan() {
    // The report's explicit request. E3009's message names two possible causes and
    // the third — "the analyzer stopped here" — was the real one for most sites, with
    // only the declaration's location printed. The note carries the construct's own
    // span and says which of the modelled reasons applied.
    let o = loud(
        "module t;\n\
         initial begin\n\
           begin\n\
             automatic int x;\n\
             $display(\"%0d\", x);\n\
             x = 1;\n\
           end\n\
           $finish;\n\
         end\n\
         endmodule",
    );
    // error at the declaration (line 4), note at the read (line 5).
    assert!(o.contains("t.sv:4:"), "{o}");
    assert!(o.contains("note[VITA-E3009]"), "{o}");
    assert!(o.contains("t.sv:5:"), "{o}");
    assert!(
        o.contains("definite-assignment for `x` stopped here"),
        "{o}"
    );
    assert!(o.contains("notes=1"), "{o}");
}

#[test]
fn the_note_distinguishes_an_unresolvable_call_from_a_read() {
    // Two different give-up reasons, so the reader can tell "I passed it somewhere"
    // from "I never taught the walk this shape".
    let o = loud(
        "module t;\n\
         initial begin\n\
           begin\n\
             automatic int x;\n\
             u.remote(x);\n\
             x = 1;\n\
           end\n\
           $finish;\n\
         end\n\
         endmodule",
    );
    assert!(o.contains("could not be proven to leave it alone"), "{o}");
}

// ── R17-X1: the reference walker was blind to a method receiver ──────────────

#[test]
fn a_method_call_outside_the_block_is_a_scope_leak() {
    // MEASURED silent-wrong. `expr_reads_ident` checked a call's ARGS but not its
    // receiver path head, so `s.atoi()` was not seen as a reference to `s`. The
    // block-local scope-leak detector is built on that walker, so the leak went
    // undetected, `s` coalesced onto the task-body `s`, and the read after the block
    // returned the block's value: vita printed `B 1234` where iverilog prints
    // `B 9999`. Now loud, with the reference's own location in the note.
    let o = loud(
        "module t;\n\
         task automatic show();\n\
           string s = \"9999\";\n\
           begin\n\
             string s;\n\
             s = \"1234\";\n\
             $display(\"A %s\", s);\n\
           end\n\
           $display(\"B %0d\", s.atoi());\n\
         endtask\n\
         initial show();\n\
         endmodule",
    );
    assert!(
        o.contains("referenced outside its `begin…end` block"),
        "{o}"
    );
    assert!(
        o.contains("t.sv:9:"),
        "note must point at the reference:\n{o}"
    );
}

#[test]
fn a_chained_method_outside_the_block_is_also_a_scope_leak() {
    // The same hole one level deeper: a chain's receiver is an expression, so even
    // the corrected path-head rule cannot see it without a `MethodCall` arm.
    loud(
        "module t;\n\
         task automatic show();\n\
           string s = \"9999\";\n\
           begin\n\
             string s;\n\
             s = \"1234\";\n\
           end\n\
           $display(\"B %0d\", s.substr(0, 1).atoi());\n\
         endtask\n\
         initial show();\n\
         endmodule",
    );
}
