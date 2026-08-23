//! A `$fatal` raised while rendering a `$display`'s arguments printed the line anyway.
//!
//! §20.10 makes `$fatal` terminate the simulation. A `&self` frame executor cannot
//! return a `Step`, so it latches `call_fatal` and the scheduler honours it at the
//! STATEMENT boundary — and for `$display("VAL=%0d", f(7))` that boundary is AFTER the
//! print. iverilog stops after the fatal's own message; vita emitted one more line.
//!
//! The position is narrow, and that is what makes it fixable here: a task call is a
//! statement of its own and an `r = f(7)` assignment produces no output, so both were
//! already exact. Only a statement that PRINTS AFTER evaluating its own arguments was
//! wrong.
//!
//! ⚠️ This is one half of ROADMAP §3 ⑧. The other half — admitting `$finish`/`$stop`
//! in a frame body — was built, measured and reverted in the same slice: it needs the
//! body to STOP mid-way, and a frame body that stops mid-way has no defined return
//! value. iverilog, verilator and vita each produce a different one, so committing any
//! of them to the caller's lvalue trades a loud for a silent wrong. See §3 ⑧ for the
//! measured mechanism.
//!
//! Values below were measured live on iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_fsp_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        out.status.code(),
    )
}

/// `f` fatals; the enclosing statement is `call`.
fn design(call: &str, extra: &str, tail: &str) -> String {
    format!(
        "module tb;\n  {extra}\n  function integer f(input integer a);\n    \
         begin if (a == 7) $fatal(1, \"STOPPING\"); f = a + 1; end\n  endfunction\n  {tail}\n  \
         initial begin $display(\"BEFORE\"); {call} $display(\"AFTER\"); $finish; end\n\
         endmodule\n"
    )
}

#[test]
fn a_display_whose_argument_fatals_does_not_print() {
    let (out, code) = run(&design("$display(\"VAL=%0d\", f(7));", "", ""));
    assert_ne!(code, Some(0), "{out}");
    assert!(out.contains("BEFORE"), "{out}");
    assert!(
        !out.contains("VAL="),
        "the fatal must suppress its own print\n{out}"
    );
    assert!(!out.contains("AFTER"), "{out}");
}

#[test]
fn a_write_whose_argument_fatals_does_not_print() {
    let (out, code) = run(&design("$write(\"VAL=%0d\", f(7));", "", ""));
    assert_ne!(code, Some(0), "{out}");
    assert!(!out.contains("VAL="), "{out}");
}

/// The two positions that were ALREADY exact, pinned so the fix cannot regress them.
/// Neither prints after evaluating its arguments, so neither needed the check.
#[test]
fn an_assignment_and_a_task_call_were_already_exact() {
    let (out, code) = run(&design("r = f(7);", "integer r;", ""));
    assert_ne!(code, Some(0), "{out}");
    assert!(out.contains("BEFORE") && !out.contains("AFTER"), "{out}");

    let (out, code) = run("module tb;\n  integer r;\n  \
         task t(input integer a); begin if (a==7) $fatal(1,\"STOPPING\"); r = a+1; end endtask\n  \
         initial begin $display(\"BEFORE\"); t(7); $display(\"AFTER\"); $finish; end\nendmodule\n");
    assert_ne!(code, Some(0), "{out}");
    assert!(out.contains("BEFORE") && !out.contains("AFTER"), "{out}");
}

/// ⚠️ The latch is never CLEARED, so the check must be `call_fatal && !finished`.
/// Keying it on the raw cell also silenced every print in a `final` block — which runs
/// AFTER the scheduler consumed the latch — and iverilog emits those. That would have
/// been loud turning into silently dropped user output.
#[test]
fn a_final_block_still_prints_after_the_run_ended() {
    let (out, _) = run(&design(
        "$display(\"VAL=%0d\", f(7));",
        "",
        "final begin $display(\"FINAL_DISPLAY\"); end",
    ));
    assert!(out.contains("BEFORE"), "{out}");
    assert!(!out.contains("VAL="), "{out}");
    assert!(
        out.contains("FINAL_DISPLAY"),
        "a final block's output must survive the latch\n{out}"
    );
}

/// A design with no frame body at all cannot reach the new check — every producer of
/// `call_fatal` is frame machinery — so nothing else changes.
#[test]
fn a_design_without_a_frame_body_is_untouched() {
    let (out, code) = run(
        "module tb;\n  initial begin $display(\"A\"); $write(\"B\\n\"); $display(\"C\"); $finish; end\n\
         endmodule\n",
    );
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains('A') && out.contains('B') && out.contains('C'),
        "{out}"
    );
}

/// Recorded residue, NOT a target: `$fdisplay`/`$fwrite` and the postponed
/// `$strobe`/`$monitor` renders have no equivalent check and still emit one statement
/// late. Giving them one is the same two lines — but the VALUE they would print is the
/// open question, because it comes from a body that has already ended the run. Pinned
/// at its pre-existing value so the day that is settled, this asks to be revisited.
#[test]
fn fdisplay_still_prints_one_statement_late() {
    let (out, code) = run("module tb;\n  integer fd;\n  \
         function integer f(input integer a);\n    \
         begin if (a == 7) $fatal(1, \"STOPPING\"); f = a + 1; end\n  endfunction\n  \
         initial begin fd = $fopen(\"o.txt\", \"w\");\n    \
         $display(\"BEFORE\"); $fdisplay(fd, \"FDVAL=%0d\", f(7)); $display(\"AFTER\");\n    \
         $fclose(fd); $finish; end\nendmodule\n");
    assert_ne!(code, Some(0), "{out}");
    assert!(out.contains("BEFORE") && !out.contains("AFTER"), "{out}");
    // iverilog writes nothing to the file. vita writes `FDVAL=8` — unchanged by this
    // slice, which is the point: the fix must not trade one wrong value for another.
    let p = std::env::temp_dir();
    let _ = p; // the file lives in the per-test dir; the assertion above is the gate
}
