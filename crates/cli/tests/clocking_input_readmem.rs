//! A clocking INPUT was writable through `$readmem*`.
//!
//! IEEE 1800 §14.3 makes a clocking input read-only, and vita enforced it — from the
//! LVALUE path: `cb.sig = 8'hAA;` is a correct `E3009`. But a `$readmem*` memory
//! argument never becomes an lvalue. It is resolved through the ordinary READ path,
//! so it reached the clocking holding net and wrote it at exit 0 while the real
//! variable kept its old contents and nothing said so. One rule, two funnels, two
//! answers. ROADMAP §2 row 8.
//!
//! ⚠️⚠️ It closes the SPELLING the row was filed against, not the class: a review
//! census measured six more argument-write tasks (`$fgets`, `$fread`, `$fscanf`,
//! `$sscanf`, `$value$plusargs`, `$swrite`/`$sformat`) and the HIERARCHICAL spelling
//! of `$readmem*` itself still writing the holding net at exit 0. ROADMAP §2 row 8b.
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
