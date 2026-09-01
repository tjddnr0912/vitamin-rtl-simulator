//! A clocking INPUT was writable through `$readmem*`.
//!
//! IEEE 1800 §14.3 makes a clocking input read-only, and vita enforced it — from the
//! LVALUE path: `cb.sig = 8'hAA;` is a correct `E3009`. But a `$readmem*` memory
//! argument never becomes an lvalue. It is resolved through the ordinary READ path,
//! so it reached the clocking holding net and wrote it at exit 0 while the real
//! variable kept its old contents and nothing said so. One rule, two funnels, two
//! answers. ROADMAP §2 row 8.
//!
//! ⭐⭐ ROW 8b (2026-09-01) put the rule on ONE FUNNEL. `deny_readonly_write` (once
//! `deny_const_param_write`) is the function every write POSITION already calls —
//! lvalues, task output/inout binds, hierarchical writes, `foreach` keys, `$readmem*`,
//! and the destinations of `$fgets`/`$fread`/`$fscanf`/`$sscanf`/`$value$plusargs`. A
//! clocking input is read-only for the same reason a parameter is, so the two
//! questions are now asked in one place; asking them in two is what let six more tasks
//! write the holding net at exit 0 while the lvalue spelling of the same rule was loud.
//! `$sformat`/`$swrite` did not go through that funnel at all and now do.
//!
//! verilator is an oracle here — the question is accept/reject, not x/z — and it
//! rejects all nine spellings with *"Cannot write to input clockvar"*.
//!
//! ⚠️ The guard asks from the PATH, like the lvalue twin, and NOT from the resolved
//! net id. A clocking input of an unpacked ARRAY gets a scalar holding net, so the
//! array-view arm never produces the id to test — a net-keyed first version fired for
//! the scalar spelling and missed the array one that motivated the row. Both
//! spellings are asserted below.
//!
//! ⚠️ No oracle: iverilog 13 cannot parse a clocking block and verilator rejects the
//! shape, so the rule is hand-IEEE (§14.3) and the CONTROLS carry the weight — an
//! ordinary array must still load, and `$writemem*`, which only READS, must still be
//! allowed to take a clocking input.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_ckrm_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("f.hex"), "00\n11\n22\n33\n").unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.success(),
    )
}

/// The array spelling — the one the row is about, and the one a net-keyed guard
/// misses.
#[test]
fn readmem_into_a_clocking_input_array_is_loud() {
    let (o, e, ok) = run("module t;\n  \
           logic clk = 0;\n  logic [7:0] mem [0:3];\n  \
           clocking cb @(posedge clk); input mem; endclocking\n  \
           initial begin $readmemh(\"f.hex\", cb.mem); #1 $finish; end\n\
         endmodule\n");
    assert!(!ok, "must not run:\n{o}{e}");
    let all = format!("{o}{e}");
    assert!(all.contains("$readmem into a clocking INPUT"), "{all}");
    assert!(
        all.contains("`cb.mem`"),
        "the message must name the signal the SOURCE wrote:\n{all}"
    );
    assert!(all.contains("§14.3"), "{all}");
}

/// …and the scalar spelling, whose lvalue twin was already loud. Both now give one
/// answer.
#[test]
fn the_lvalue_spelling_is_still_loud() {
    let (o, e, ok) = run("module t;\n  \
           logic clk = 0; logic [7:0] s;\n  \
           clocking cb @(posedge clk); input s; endclocking\n  \
           initial begin cb.s = 8'hAA; #1 $finish; end\n\
         endmodule\n");
    assert!(!ok, "must not run:\n{o}{e}");
    assert!(
        format!("{o}{e}").contains("cannot drive a clocking INPUT"),
        "{o}{e}"
    );
}

/// CONTROLS. An ordinary array still loads, `$writemem*` — which only READS its
/// memory argument — may still take a clocking input, and a clocking input used as
/// the FILE NAME is a pure read and must be accepted.
///
/// ⚠️ That last one is not hypothetical: the first version of the guard ran for
/// EVERY argument index, so `$readmemh(cb.fn, mem)` — a legal design vita ran
/// correctly — was refused. §21.4's shape is `(file, mem, start, end)` and only
/// argument 1 is written; two review lenses found the missing position test.
#[test]
fn an_ordinary_array_still_loads_and_writemem_may_read_a_clocking_input() {
    let (o, e, ok) = run("module t;\n  \
           logic clk = 0;\n  logic [7:0] mem [0:3];\n  logic [7:0] cmem [0:3];\n  \
           clocking cb @(posedge clk); input cmem; endclocking\n  \
           initial begin\n    \
             $readmemh(\"f.hex\", mem);\n    \
             $writememh(\"out.hex\", cb.cmem);\n    \
             #1 $display(\"m0=%h m3=%h\", mem[0], mem[3]);\n    \
             $finish;\n  \
           end\n\
         endmodule\n");
    assert!(ok, "vita failed:\n{o}{e}");
    assert!(o.contains("m0=00 m3=33"), "got:\n{o}");
}

/// Every OTHER task that writes an argument, on one line. Each of these ran silently
/// before row 8b, leaving the real variable at `55` while the clocking holding net
/// moved; verilator rejects all of them.
#[test]
fn every_argument_writing_task_refuses_a_clocking_input() {
    let body = |call: &str| {
        format!(
            "module t;\n  \
               logic clk = 0; logic [7:0] s;\n  \
               clocking cb @(posedge clk); input s; endclocking\n  \
               initial begin s = 8'h55; {call} #1 $finish; end\n\
             endmodule\n"
        )
    };
    for (call, who) in [
        (
            "begin integer fd; fd=$fopen(\"d.txt\",\"r\"); void'($fgets(cb.s, fd)); end",
            "$fgets",
        ),
        (
            "begin integer fd; fd=$fopen(\"d.txt\",\"r\"); void'($fread(cb.s, fd)); end",
            "$fread",
        ),
        (
            "begin integer fd; fd=$fopen(\"d.txt\",\"r\"); void'($fscanf(fd, \"%d\", cb.s)); end",
            "$fscanf",
        ),
        ("begin void'($sscanf(\"9\", \"%d\", cb.s)); end", "$fscanf"),
        (
            "begin void'($value$plusargs(\"V=%d\", cb.s)); end",
            "$value$plusargs",
        ),
        ("begin $swrite(cb.s, \"%d\", 9); end", "$swrite"),
        ("begin $sformat(cb.s, \"%d\", 9); end", "$sformat"),
    ] {
        let (o, e, ok) = run(&body(call));
        let all = format!("{o}{e}");
        assert!(!ok, "{who} must not run:\n{all}");
        assert!(
            all.contains("a clocking INPUT") && all.contains("`cb.s`"),
            "{who} must name the rule and the signal:\n{all}"
        );
    }
}

/// The HIERARCHICAL spellings, both directions. The lvalue one was already loud but
/// hardcoded a signal name that need not exist in the design; the `$readmem*` one was
/// silent.
#[test]
fn the_hierarchical_spellings_are_loud_and_name_the_signal() {
    let (o, e, ok) = run("module sub;\n  logic [7:0] mem [0:3]; logic clk = 0;\n  \
           clocking cb @(posedge clk); input mem; endclocking\n\
         endmodule\n\
         module t;\n  sub u();\n  \
           initial begin $readmemh(\"f.hex\", u.cb.mem); #1 $finish; end\n\
         endmodule\n");
    assert!(!ok, "must not run:\n{o}{e}");
    assert!(format!("{o}{e}").contains("`cb.mem`"), "{o}{e}");

    let (o, e, ok) = run("module sub;\n  logic [7:0] sig8; logic clk = 0;\n  \
           clocking cb @(posedge clk); input sig8; endclocking\n\
         endmodule\n\
         module t;\n  sub u();\n  \
           initial begin u.cb.sig8 = 8'h55; #1 $finish; end\n\
         endmodule\n");
    assert!(!ok, "must not run:\n{o}{e}");
    assert!(
        format!("{o}{e}").contains("`cb.sig8`"),
        "the message must name the signal the SOURCE wrote:\n{o}{e}"
    );
}

/// ⭐ THREE MORE, from review: `$cast`'s destination and the SEED of `$random` and
/// `$dist_*`, which the engine advances. `$cast`'s guard is the one the `$sformat`
/// block cites as its own model, and it was the only half of that pair that never
/// asked the read-only question.
#[test]
fn cast_and_the_seeded_rng_destinations_refuse_a_clocking_input() {
    for (call, who) in [
        ("rc = $cast(cb.s, 8'hAA);", "$cast into"),
        ("$cast(cb.s, 8'hAA);", "$cast into"),
        ("iv = $random(cb.s);", "advance $random's seed in"),
        (
            "iv = $dist_uniform(cb.s, 0, 10);",
            "advance a $dist_* seed in",
        ),
        (
            "iv = $dist_exponential(cb.s, 3);",
            "advance a $dist_* seed in",
        ),
    ] {
        let (o, e, ok) = run(&format!(
            "module t;\n  \
               logic clk = 0; logic [31:0] s = 32'd55; int iv; int rc;\n  \
               always #1 clk = ~clk;\n  \
               clocking cb @(posedge clk); input s; endclocking\n  \
               initial begin #3; {call} $finish; end\n  \
               initial #100 $finish;\n\
             endmodule\n"
        ));
        let all = format!("{o}{e}");
        assert!(!ok, "{call} must not run:\n{all}");
        assert!(all.contains(who), "{call} wants `{who}`:\n{all}");
    }
}

/// CONTROLS for the three above — an ordinary destination still works, and the seed
/// still advances.
#[test]
fn cast_and_the_seeded_rng_still_write_an_ordinary_variable() {
    let (o, e, ok) = run("module t;\n  \
           logic [31:0] s = 32'd55; int iv; int rc; int sd = 32'd7;\n  \
           initial begin\n    \
             rc = $cast(s, 32'hAA);\n    \
             iv = $random(sd);\n    \
             $display(\"s=%0h moved=%0d\", s, sd != 32'd7);\n    \
             $finish;\n  \
           end\n\
         endmodule\n");
    assert!(ok, "vita failed:\n{o}{e}");
    assert!(o.contains("s=aa moved=1"), "got:\n{o}");
}

/// ⚠️⚠️ ONE KEYWORD used to flip the whole fix off. Every spelling above is loud at
/// module scope, and adding `automatic` to the enclosing task — or writing the
/// destination hierarchically — made the SAME statement silent again, because the
/// destination is lowered before the name it points at exists, so the funnel is handed
/// a poison net and answers nothing. `$readmem*` was loud there only because it
/// happens to carry a side map for an unrelated reason. Review measured all seven.
#[test]
fn the_automatic_and_hierarchical_spellings_are_loud_too() {
    let auto = |call: &str| {
        format!(
            "module t;\n  \
               logic clk = 0; logic [31:0] s = 32'd55; int iv; int fd;\n  \
               always #1 clk = ~clk;\n  \
               clocking cb @(posedge clk); input s; endclocking\n  \
               task automatic doit(); {call} endtask\n  \
               initial begin #3; doit(); $finish; end\n  \
               initial #100 $finish;\n\
             endmodule\n"
        )
    };
    for call in [
        "$swrite(cb.s, \"%0d\", 9);",
        "$sformat(cb.s, \"%0d\", 9);",
        "void'($value$plusargs(\"V=%d\", cb.s));",
        "void'($sscanf(\"9\", \"%d\", cb.s));",
        "begin fd = $fopen(\"d.txt\", \"r\"); void'($fgets(cb.s, fd)); end",
        "begin fd = $fopen(\"d.txt\", \"r\"); void'($fread(cb.s, fd)); end",
        "begin fd = $fopen(\"d.txt\", \"r\"); void'($fscanf(fd, \"%d\", cb.s)); end",
    ] {
        let (o, e, ok) = run(&auto(call));
        let all = format!("{o}{e}");
        assert!(!ok, "`automatic` must not turn the rule off: {call}\n{all}");
        assert!(all.contains("a clocking INPUT"), "{call}:\n{all}");
    }

    // …and the hierarchical spelling of the same question.
    let (o, e, ok) = run("module sub;\n  \
           logic clk = 0; logic [31:0] s = 32'd55;\n  \
           clocking cb @(posedge clk); input s; endclocking\n\
         endmodule\n\
         module t;\n  sub u();\n  \
           initial begin $swrite(u.cb.s, \"%0d\", 9); #1 $finish; end\n\
         endmodule\n");
    assert!(!ok, "must not run:\n{o}{e}");
    assert!(format!("{o}{e}").contains("a clocking INPUT"), "{o}{e}");
}

/// CONTROL for the one above: the same shapes inside an `automatic` task, writing an
/// ordinary variable, must still run. A guard that fires on every deferred
/// destination would pass the test above and fail this one.
#[test]
fn an_automatic_task_still_writes_an_ordinary_destination() {
    let (o, e, ok) = run("module t;\n  \
           logic [31:0] s = 32'd55; string txt;\n  \
           task automatic doit();\n    \
             $swrite(s, \"%0d\", 9);\n    \
             $sformat(txt, \"t%0d\", 7);\n  \
           endtask\n  \
           initial begin doit(); $display(\"s=%0d txt=%s\", s, txt); $finish; end\n\
         endmodule\n");
    assert!(ok, "vita failed:\n{o}{e}");
    // 57 is ASCII `9`: `$swrite` renders TEXT into the 32-bit reg. iverilog agrees.
    assert!(o.contains("s=57 txt=t7"), "got:\n{o}");
}

/// …and the FILE-NAME position, which the first version of the guard broke.
#[test]
fn a_clocking_input_as_the_file_name_is_a_read_and_is_accepted() {
    let (o, e, ok) = run("module t;\n  \
           logic clk = 0; logic [7:0] mem [0:3];\n  \
           logic [8*8-1:0] fn = \"f.hex\";\n  \
           clocking cb @(posedge clk); input fn; endclocking\n  \
           initial begin $readmemh(cb.fn, mem); #1 $finish; end\n\
         endmodule\n");
    assert!(ok, "the file name is a READ, not a write:\n{o}{e}");
    assert!(
        !format!("{o}{e}").contains("clocking INPUT"),
        "must not fire on argument 0:\n{o}{e}"
    );
}
