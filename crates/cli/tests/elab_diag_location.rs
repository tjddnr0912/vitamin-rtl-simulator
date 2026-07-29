//! §4.5.249 — elaborate-time diagnostics carry `file:line:col`.
//!
//! The round-20 report's §6 was a request, not a defect list: "`E3009`/`E3010` have no
//! file:line. TB=top prints the same message 81 times and we cannot tell which
//! declaration it is about — this one thing is why we could not narrow §4.11." Lex and
//! parse diagnostics were located all along; only elaborate's were not, because the
//! elaborator had no view of the preprocessor's `SourceMap`.
//!
//! It now takes one (`diag::SpanResolver`, supplied by the front end), and the
//! statement / declaration walkers anchor `cur_span` so a diagnostic raised deep in a
//! helper still points at the construct the user wrote.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Run vita over N named sources in a fresh directory; returns combined output.
fn run_files(files: &[(&str, &str)]) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_loc_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let mut c = Command::new(env!("CARGO_BIN_EXE_vita"));
    for (name, src) in files {
        std::fs::write(d.join(name), src).unwrap();
        c.arg(name);
    }
    let out = c.current_dir(&d).output().expect("run vita");
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

fn run(src: &str) -> String {
    run_files(&[("t.sv", src)])
}

/// The line must be the DECLARATION's, not the module's or the file's first line —
/// otherwise 81 of these are still indistinguishable.
#[test]
fn a_block_local_diagnostic_points_at_its_declaration() {
    let out = run("module t;\n\
         \n\
         initial begin\n\
           begin\n\
             automatic int x;\n\
             if (x == 0) $display(\"q\");\n\
           end\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert!(out.contains("error[VITA-E3009]"), "expected loud:\n{out}");
    assert!(
        out.contains("t.sv:5:"),
        "must point at the declaration on line 5:\n{out}"
    );
}

/// Across MULTIPLE command-line sources each diagnostic must name its OWN file with a
/// file-local line — the whole point of resolving through the SourceMap rather than
/// counting lines in the concatenated buffer.
#[test]
fn each_source_file_gets_its_own_name_and_local_line() {
    let out = run_files(&[
        (
            "a.sv",
            "module a;\n\
             initial begin\n\
               begin automatic int x; if (x == 0) $display(\"q\"); end\n\
               $finish;\n\
             end\n\
             endmodule\n",
        ),
        (
            "b.sv",
            "module b;\n\
             \n\
             \n\
             initial begin\n\
               begin automatic int y; if (y == 1) $display(\"w\"); end\n\
               $finish;\n\
             end\n\
             endmodule\n",
        ),
    ]);
    assert!(out.contains("a.sv:3:"), "a.sv line 3:\n{out}");
    assert!(out.contains("b.sv:5:"), "b.sv line 5:\n{out}");
}

/// A diagnostic raised while lowering a STATEMENT (not a declaration) anchors at that
/// statement, so two of the same kind in one process are still distinguishable.
#[test]
fn statement_diagnostics_anchor_at_their_own_statement() {
    let out = run("module t;\n\
         string u;\n\
         logic c = 1;\n\
         initial begin\n\
           u = c ? $sformatf(\"a\") : $sformatf(\"b\");\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert!(out.contains("error[VITA-E3009]"), "expected loud:\n{out}");
    assert!(out.contains("t.sv:5:"), "the assignment is line 5:\n{out}");
}

/// A warning gets the same treatment — the channel is the diagnostic, not the severity.
#[test]
fn elaborate_warnings_are_located_too() {
    let out = run("module t;\n\
         logic [3:0] a;\n\
         initial begin\n\
           a = 8'hFF;\n\
           $display(\"%h\", a);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    // Whether this particular assignment warns is not the point; if any elaborate
    // warning is emitted at all it must carry a position.
    for line in out.lines().filter(|l| l.contains("warning[VITA-W3")) {
        assert!(
            line.starts_with("t.sv:"),
            "an elaborate warning without a position:\n{line}"
        );
    }
}

/// The same-name dynamic-storage message used to carry NO identifier either, so a run
/// full of them named nothing at all. It now names the local AND states the rule that
/// decides whether two same-named locals can have distinct storage.
#[test]
fn the_same_name_dynamic_local_message_names_the_local_and_the_rule() {
    // A block-local dynamic array shadowing a MODULE net of the same name — the shape
    // §4.5.251's widening still cannot scope (one declaring block, and the name is a
    // module name), so this is where the message is still read.
    let out = run("module t;\n\
         byte m [];\n\
         initial begin\n\
           m = new[5];\n\
           begin byte m[]; m = new[1]; $display(\"A=%0d\", m.size()); end\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert!(out.contains("error[VITA-E3009]"), "expected loud:\n{out}");
    assert!(out.contains("`m`"), "must name the local:\n{out}");
    assert!(
        out.contains("A pair earns distinct storage when both are `automatic`"),
        "must state the rule that would make it work:\n{out}"
    );
    // R16 §4-2: and it must not INFER the other declaration's lifetime. The old text
    // ended "this one is `automatic`, so the OTHER is not", which was simply false
    // whenever the pair failed for one of the other reasons — in the round-16 report
    // both declarations were spelled `automatic` and the reader was sent looking for a
    // static twin that did not exist.
    assert!(
        !out.contains("so the OTHER is not"),
        "must not infer the other declaration's lifetime:\n{out}"
    );
}

/// §4.5 wording: a task with no timing control is not a "frame task" to anyone reading
/// their own source. The message says `task`.
#[test]
fn a_non_suspending_task_is_not_called_a_frame_task() {
    let out = run("module t;\n\
         task automatic show (input byte k [], input byte m []);\n\
           $display(\"%0d %0d\", k.size(), m.size());\n\
         endtask\n\
         byte msg [] = '{8'h00};\n\
         initial begin show(msg[0], msg); $finish; end\n\
         endmodule\n");
    assert!(out.contains("error[VITA-"), "expected loud:\n{out}");
    assert!(
        !out.contains("frame task"),
        "`frame task` is internal vocabulary:\n{out}"
    );
}
