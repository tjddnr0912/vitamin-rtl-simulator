//! ROADMAP §2 🆕 N, two cells that a hierarchical path and an instance array got
//! wrong in opposite directions.
//!
//! A. A generate scope's hierarchical SPELLING (IEEE 1800 §27.4 vs §27.5). vita
//!    stores a conditional / `case` / bare labelled block as `label[0]` so the scope
//!    walk treats it like a loop iteration, and it stores a generate-for iteration as
//!    `label[idx]` for real. The storage shape is the same, the legal source spelling
//!    is not: a singleton block's name is the BARE label (§27.5 gives it no index) and
//!    a loop block's name is an ARRAY that must be indexed at any trip count (§27.4).
//!    Both storage spellings were reachable from source text, so `gi[0].x` read `3`
//!    and `gl.x` read iteration 0's `5`, each at exit 0, where iverilog 13.0 refuses
//!    to bind (`Unable to bind wire/reg/memory `gi['sd0].x'`, ``gl.x'`) and verilator
//!    5.052 cannot find the scope. Measured on net read and write, localparam,
//!    `$bits`, event control, element and part select, cross-instance, nested, `case`
//!    generate and an unnamed `genblk1`. Now E3010 in every one of them, naming the
//!    block and the spelling that works.
//!
//! B. An instance ARRAY of a PORTLESS child. `module ch;` parses as `PortList::None`
//!    and `module ch();` as an empty ANSI list — the same module — and the array
//!    lane's ANSI-only cut read "no ANSI list" as "non-ANSI", so the first was E3009
//!    ("child `ch` has non-ANSI ports") plus an E3010 per element while the second
//!    ran. Both oracles run both. An empty port list is trivially ANSI, so the
//!    portless child is admitted; a NON-EMPTY non-ANSI header stays refused (its port
//!    widths live in body `PortDecl`s this lane does not read — recorded, both oracles
//!    run it).
//!
//! Every expected value pinned to LIVE iverilog 13.0 and verilator 5.052.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_args(src: &str, args: &[&str]) -> (String, String, Option<i32>, std::path::PathBuf) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_gsi_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .args(args)
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
        d,
    )
}

fn run(src: &str) -> (String, String, Option<i32>) {
    let (o, e, c, _) = run_args(src, &[]);
    (o, e, c)
}

/// Loud = a VITA error code anywhere on either stream, or exit 1. Never a value.
fn is_loud(out: &str, err: &str, code: Option<i32>) -> bool {
    out.contains("VITA-E") || err.contains("VITA-E") || code == Some(1)
}

// ════════════════════════════════════════════════════════════════════
//  A. §27.5 — a SINGLETON generate block has no index
// ════════════════════════════════════════════════════════════════════

const SINGLETON: &str = "module top; generate if (1) begin : gi int x = 3; end endgenerate\n";

#[test]
fn singleton_indexed_net_read_is_loud() {
    // iverilog: Unable to bind `gi['sd0].x'. verilator: Can't find 'gi[0]'. vita: 3.
    let (out, err, c) = run(&format!(
        "{SINGLETON}  initial begin $display(\"X=%0d\", gi[0].x); #1 $finish; end\nendmodule\n"
    ));
    assert!(is_loud(&out, &err, c), "must refuse:\n{out}{err}");
    assert!(!out.contains("X=3"), "must not answer a value:\n{out}");
    // The refusal names the block, not "no such net" — the net exists as `gi.x`.
    assert!(err.contains("`gi`"), "names the block:\n{err}");
}

#[test]
fn singleton_bare_net_read_still_works() {
    // Both oracles: X=3. The legal spelling must be untouched.
    let (out, err, _c) = run(&format!(
        "{SINGLETON}  initial begin $display(\"X=%0d\", gi.x); #1 $finish; end\nendmodule\n"
    ));
    assert!(out.contains("X=3"), "out:\n{out}err:\n{err}");
    assert!(!out.contains("VITA-E"), "{out}");
}

#[test]
fn singleton_indexed_write_is_loud() {
    // iverilog: Could not find variable ``gi['sd0].x''. vita wrote it and read 9 back.
    let (out, err, c) = run(&format!(
        "{SINGLETON}  initial begin gi[0].x = 9; $display(\"X=%0d\", gi.x); #1 $finish; end\n\
         endmodule\n"
    ));
    assert!(is_loud(&out, &err, c), "must refuse the write:\n{out}{err}");
    assert!(!out.contains("X=9"), "{out}");
}

#[test]
fn singleton_indexed_localparam_is_loud() {
    // iverilog: Unable to bind `gi['sd0].P'. verilator answers 11 here while it
    // refuses `gi[0].x` one line up — it contradicts itself, so it is not the oracle
    // for this cell; the §27.5 rule and iverilog agree and vita follows them.
    let (out, err, c) = run(
        "module top; generate if (1) begin : gi localparam int P = 11; end endgenerate\n\
           initial begin $display(\"P=%0d\", gi[0].P); #1 $finish; end\n\
         endmodule\n",
    );
    assert!(is_loud(&out, &err, c), "{out}{err}");
    assert!(!out.contains("P=11"), "{out}");
}

#[test]
fn singleton_indexed_bits_is_loud() {
    // iverilog degrades `$bits` of the unbound name to 0; verilator answers 8 (the
    // same self-contradiction). Loud is the only answer that is not a guess.
    let (out, err, c) = run(
        "module top; generate if (1) begin : gi logic [7:0] x = 3; end endgenerate\n\
           initial begin $display(\"B=%0d\", $bits(gi[0].x)); #1 $finish; end\n\
         endmodule\n",
    );
    assert!(is_loud(&out, &err, c), "{out}{err}");
}

#[test]
fn singleton_bare_bits_still_works() {
    // iverilog and verilator: B=8.
    let (out, err, _c) = run(
        "module top; generate if (1) begin : gi logic [7:0] x = 8'ha5; end endgenerate\n\
           initial begin $display(\"B=%0d\", $bits(gi.x)); #1 $finish; end\n\
         endmodule\n",
    );
    assert!(out.contains("B=8"), "out:\n{out}err:\n{err}");
}

#[test]
fn singleton_indexed_event_control_is_loud() {
    let (out, err, c) = run(
        "module top; generate if (1) begin : gi logic x = 0; end endgenerate\n\
           int n = 0;\n\
           always @(gi[0].x) n = n + 1;\n\
           initial begin #1 gi.x = 1; #1 $display(\"N=%0d\", n); #1 $finish; end\n\
         endmodule\n",
    );
    assert!(is_loud(&out, &err, c), "{out}{err}");
}

#[test]
fn singleton_indexed_array_element_is_loud() {
    // iverilog: Unable to bind `gi['sd0].mem['sd1]'. vita read 17.
    let (out, err, c) = run(
        "module top; generate if (1) begin : gi int mem [0:3]; end endgenerate\n\
           initial begin gi.mem[1] = 17; $display(\"M=%0d\", gi[0].mem[1]); #1 $finish; end\n\
         endmodule\n",
    );
    assert!(is_loud(&out, &err, c), "{out}{err}");
    assert!(!out.contains("M=17"), "{out}");
}

#[test]
fn singleton_indexed_part_select_is_loud() {
    // iverilog: Unable to bind `gi['sd0].x['sd3:'sd0]'. vita read 5.
    let (out, err, c) = run(
        "module top; generate if (1) begin : gi logic [7:0] x = 8'ha5; end endgenerate\n\
           initial begin $display(\"P=%0h\", gi[0].x[3:0]); #1 $finish; end\n\
         endmodule\n",
    );
    assert!(is_loud(&out, &err, c), "{out}{err}");
    assert!(!out.contains("P=5"), "{out}");
}

#[test]
fn cross_instance_singleton_indexed_is_loud() {
    // iverilog: Unable to bind `u.gi['sd0].x'. vita read 4.
    let (out, err, c) = run(
        "module sub; generate if (1) begin : gi int x = 4; end endgenerate endmodule\n\
         module top; sub u();\n\
           initial begin $display(\"X=%0d\", u.gi[0].x); #1 $finish; end\n\
         endmodule\n",
    );
    assert!(is_loud(&out, &err, c), "{out}{err}");
    assert!(!out.contains("X=4"), "{out}");
}

#[test]
fn cross_instance_singleton_bare_still_works() {
    // Both oracles: X=4.
    let (out, err, _c) = run(
        "module sub; generate if (1) begin : gi int x = 4; end endgenerate endmodule\n\
         module top; sub u();\n\
           initial begin $display(\"X=%0d\", u.gi.x); #1 $finish; end\n\
         endmodule\n",
    );
    assert!(out.contains("X=4"), "out:\n{out}err:\n{err}");
}

#[test]
fn nested_singleton_indexed_is_loud() {
    // iverilog: Unable to bind `gi['sd0].gj['sd0].x'. vita read 6.
    let (out, err, c) = run(
        "module top; generate if (1) begin : gi if (1) begin : gj int x = 6; end end endgenerate\n\
           initial begin $display(\"X=%0d\", gi[0].gj[0].x); #1 $finish; end\n\
         endmodule\n",
    );
    assert!(is_loud(&out, &err, c), "{out}{err}");
    assert!(!out.contains("X=6"), "{out}");
}

#[test]
fn nested_singleton_bare_still_works() {
    // Both oracles: X=6.
    let (out, err, _c) = run(
        "module top; generate if (1) begin : gi if (1) begin : gj int x = 6; end end endgenerate\n\
           initial begin $display(\"X=%0d\", gi.gj.x); #1 $finish; end\n\
         endmodule\n",
    );
    assert!(out.contains("X=6"), "out:\n{out}err:\n{err}");
}

#[test]
fn case_generate_indexed_is_loud() {
    // A `case` generate block is a singleton too. iverilog: Unable to bind `gc['sd0].x'.
    let (out, err, c) = run("module top; localparam int S = 1;\n\
           generate case (S) 1: begin : gc int x = 14; end endcase endgenerate\n\
           initial begin $display(\"X=%0d\", gc[0].x); #1 $finish; end\n\
         endmodule\n");
    assert!(is_loud(&out, &err, c), "{out}{err}");
    assert!(!out.contains("X=14"), "{out}");
}

#[test]
fn case_generate_bare_still_works() {
    // Both oracles: X=14.
    let (out, err, _c) = run("module top; localparam int S = 1;\n\
           generate case (S) 1: begin : gc int x = 14; end endcase endgenerate\n\
           initial begin $display(\"X=%0d\", gc.x); #1 $finish; end\n\
         endmodule\n");
    assert!(out.contains("X=14"), "out:\n{out}err:\n{err}");
}

#[test]
fn unnamed_genblk_indexed_is_loud() {
    // §27.6 names an unnamed block `genblk1`; it is still a singleton.
    // iverilog: Unable to bind `genblk1['sd0].x'.
    let (out, err, c) = run(
        "module top; generate if (1) begin int x = 19; end endgenerate\n\
           initial begin $display(\"X=%0d\", genblk1[0].x); #1 $finish; end\n\
         endmodule\n",
    );
    assert!(is_loud(&out, &err, c), "{out}{err}");
    assert!(!out.contains("X=19"), "{out}");
}

#[test]
fn unnamed_genblk_bare_still_works() {
    // Both oracles: X=19.
    let (out, err, _c) = run(
        "module top; generate if (1) begin int x = 19; end endgenerate\n\
           initial begin $display(\"X=%0d\", genblk1.x); #1 $finish; end\n\
         endmodule\n",
    );
    assert!(out.contains("X=19"), "out:\n{out}err:\n{err}");
}

#[test]
fn out_of_range_singleton_index_is_loud() {
    // `gi[1]` was already refused; it must stay refused and now says WHY.
    let (out, err, c) = run(&format!(
        "{SINGLETON}  initial begin $display(\"X=%0d\", gi[1].x); #1 $finish; end\nendmodule\n"
    ));
    assert!(is_loud(&out, &err, c), "{out}{err}");
}

// ════════════════════════════════════════════════════════════════════
//  A'. §27.4 — a generate-FOR block IS an array and must be indexed
// ════════════════════════════════════════════════════════════════════

const LOOP2: &str =
    "module top; genvar i; generate for (i=0;i<2;i=i+1) begin : gl int x = i+5; end endgenerate\n";

#[test]
fn loop_indexed_read_still_works() {
    // Both oracles: X=5 6.
    let (out, err, _c) = run(&format!(
        "{LOOP2}  initial begin $display(\"X=%0d %0d\", gl[0].x, gl[1].x); #1 $finish; end\n\
         endmodule\n"
    ));
    assert!(out.contains("X=5 6"), "out:\n{out}err:\n{err}");
    assert!(!out.contains("VITA-E"), "{out}");
}

#[test]
fn loop_indexed_write_still_works() {
    // Both oracles: X=5 40.
    let (out, err, _c) = run(&format!(
        "{LOOP2}  initial begin gl[1].x = 40; $display(\"X=%0d %0d\", gl[0].x, gl[1].x);\n\
                    #1 $finish; end\n\
         endmodule\n"
    ));
    assert!(out.contains("X=5 40"), "out:\n{out}err:\n{err}");
}

#[test]
fn loop_bare_read_is_loud() {
    // iverilog: Unable to bind `gl.x'. verilator: Can't find 'x' in dotted 'gl.x'.
    // vita answered iteration 0's 5 at exit 0 — the leading-segment arm of
    // `hier_resolve` was the one place that did not ask `gen_loop_labels`.
    let (out, err, c) = run(&format!(
        "{LOOP2}  initial begin $display(\"X=%0d\", gl.x); #1 $finish; end\nendmodule\n"
    ));
    assert!(is_loud(&out, &err, c), "must refuse:\n{out}{err}");
    assert!(!out.contains("X=5"), "must not pick iteration 0:\n{out}");
    assert!(err.contains("`gl`"), "names the block:\n{err}");
}

#[test]
fn one_trip_loop_indexed_read_still_works() {
    // A one-trip loop leaves exactly the keys a singleton leaves; the label sets,
    // not the keys, decide. Both oracles: X=7.
    let (out, err, _c) = run(
        "module top; genvar i; generate for (i=0;i<1;i=i+1) begin : gl int x = 7; end endgenerate\n\
           initial begin $display(\"X=%0d\", gl[0].x); #1 $finish; end\n\
         endmodule\n",
    );
    assert!(out.contains("X=7"), "out:\n{out}err:\n{err}");
}

#[test]
fn one_trip_loop_bare_read_is_loud() {
    // §27.4 makes it an array at ANY trip count — iverilog refuses `gl.x` here too.
    let (out, err, c) = run(
        "module top; genvar i; generate for (i=0;i<1;i=i+1) begin : gl int x = 7; end endgenerate\n\
           initial begin $display(\"X=%0d\", gl.x); #1 $finish; end\n\
         endmodule\n",
    );
    assert!(is_loud(&out, &err, c), "{out}{err}");
    assert!(!out.contains("X=7"), "{out}");
}

#[test]
fn cross_instance_loop_indexed_still_works() {
    // Both oracles: X=30 31.
    let (out, err, _c) = run("module sub; genvar i;\n\
           generate for (i=0;i<2;i=i+1) begin : gl int x = i+30; end endgenerate\n\
         endmodule\n\
         module top; sub u();\n\
           initial begin $display(\"X=%0d %0d\", u.gl[0].x, u.gl[1].x); #1 $finish; end\n\
         endmodule\n");
    assert!(out.contains("X=30 31"), "out:\n{out}err:\n{err}");
}

#[test]
fn cross_instance_loop_bare_is_loud() {
    // Already refused before this slice (the `hier_key_within` half of the rule);
    // pinned so the shared helper cannot lose it.
    let (out, err, c) = run("module sub; genvar i;\n\
           generate for (i=0;i<2;i=i+1) begin : gl int x = i+30; end endgenerate\n\
         endmodule\n\
         module top; sub u();\n\
           initial begin $display(\"X=%0d\", u.gl.x); #1 $finish; end\n\
         endmodule\n");
    assert!(is_loud(&out, &err, c), "{out}{err}");
}

// ════════════════════════════════════════════════════════════════════
//  B. an instance ARRAY of a PORTLESS child
// ════════════════════════════════════════════════════════════════════

const PORTLESS: &str = "module ch; int q = 4; endmodule\n";

#[test]
fn portless_child_instance_array_runs() {
    // Both oracles: Q=4 4. vita was E3009 "child `ch` has non-ANSI ports" + 2× E3010.
    let (out, err, _c) = run(&format!(
        "{PORTLESS}module top; ch w[1:0]();\n\
           initial begin $display(\"Q=%0d %0d\", w[0].q, w[1].q); #1 $finish; end\n\
         endmodule\n"
    ));
    assert!(out.contains("Q=4 4"), "out:\n{out}err:\n{err}");
    assert!(!out.contains("VITA-E"), "{out}");
}

#[test]
fn empty_ansi_child_instance_array_still_runs() {
    // `module ch();` is the SAME module, and it already took this lane — the control
    // that shows the refusal was about the spelling of the header, not the design.
    let (out, err, _c) = run("module ch(); int q = 4; endmodule\n\
         module top; ch w[1:0]();\n\
           initial begin $display(\"Q=%0d %0d\", w[0].q, w[1].q); #1 $finish; end\n\
         endmodule\n");
    assert!(out.contains("Q=4 4"), "out:\n{out}err:\n{err}");
}

#[test]
fn portless_array_elements_are_distinct_nets() {
    // A hierarchical WRITE into one element must not move the other. Both oracles: Q=9 4.
    let (out, err, _c) = run(&format!(
        "{PORTLESS}module top; ch w[1:0]();\n\
           initial begin w[0].q = 9; #1 $display(\"Q=%0d %0d\", w[0].q, w[1].q); #1 $finish; end\n\
         endmodule\n"
    ));
    assert!(out.contains("Q=9 4"), "out:\n{out}err:\n{err}");
}

#[test]
fn portless_array_takes_param_overrides() {
    // Both oracles: Q=6 6.
    let (out, err, _c) = run("module ch; parameter int A = 0; int q = A + 1; endmodule\n\
         module top; ch #(.A(5)) w[1:0]();\n\
           initial begin $display(\"Q=%0d %0d\", w[0].q, w[1].q); #1 $finish; end\n\
         endmodule\n");
    assert!(out.contains("Q=6 6"), "out:\n{out}err:\n{err}");
}

#[test]
fn portless_array_percent_m_names_each_element() {
    // Both oracles print `top.w[0]` and `top.w[1]` (vita walks the DECLARED order
    // `[1:0]`, i.e. w[1] first — the pre-existing instance-array elaboration order,
    // identical on the ANSI-ported twin, so it is not this lane's).
    let (out, err, _c) = run("module ch; initial $display(\"M=%m\"); endmodule\n\
         module top; ch w[1:0](); initial begin #1 $finish; end\n\
         endmodule\n");
    assert!(out.contains("M=top.w[0]"), "out:\n{out}err:\n{err}");
    assert!(out.contains("M=top.w[1]"), "out:\n{out}");
}

#[test]
fn portless_array_inside_a_generate_scope() {
    // Both oracles: Q=4 4.
    let (out, err, _c) = run(&format!(
        "{PORTLESS}module top; generate if (1) begin : gi ch w[1:0](); end endgenerate\n\
           initial begin $display(\"Q=%0d %0d\", gi.w[0].q, gi.w[1].q); #1 $finish; end\n\
         endmodule\n"
    ));
    assert!(out.contains("Q=4 4"), "out:\n{out}err:\n{err}");
}

#[test]
fn portless_array_beside_a_scalar_instance() {
    // The array elements and a plain instance of the same portless module coexist.
    // Both oracles: Q=4 4 4.
    let (out, err, _c) = run(&format!(
        "{PORTLESS}module top; ch w[1:0](); ch v();\n\
           initial begin $display(\"Q=%0d %0d %0d\", w[0].q, w[1].q, v.q); #1 $finish; end\n\
         endmodule\n"
    ));
    assert!(out.contains("Q=4 4 4"), "out:\n{out}err:\n{err}");
}

#[test]
fn portless_array_wildcard_connects_nothing() {
    // `.*` on a portless child matches zero ports, so there is no per-element
    // resolution to do and the array-lane `.*` cut does not apply. Both oracles: Q=4 4.
    let (out, err, _c) = run(&format!(
        "{PORTLESS}module top; ch w[1:0](.*);\n\
           initial begin $display(\"Q=%0d %0d\", w[0].q, w[1].q); #1 $finish; end\n\
         endmodule\n"
    ));
    assert!(out.contains("Q=4 4"), "out:\n{out}err:\n{err}");
}

// ════════════════════════════════════════════════════════════════════
//  C. a BARE instance-array label is never an element's name
//
//  `singleton_scope_key` decided "is this a singleton generate scope" NEGATIVELY
//  (`label` not in `gen_loop_labels`, `label[0]` a scope, `label[1]` not), and an
//  INSTANCE-ARRAY label is in neither label set — so a ONE-ELEMENT array passed
//  both halves and the bare `u.q` reached element 0 at exit 0. It is now keyed
//  POSITIVELY on `gen_singleton_labels`, the set `display_prefix` already used for
//  the `%m` twin. Three spellings of the same array, all three refused by both
//  oracles; an element must be written `u[0].q`.
// ════════════════════════════════════════════════════════════════════

#[test]
fn bare_label_on_a_one_element_portless_array_is_loud() {
    // iverilog:  "g408.sv:4: error: Unable to bind wire/reg/memory `u.q' in `g408'"
    // verilator: "%Error: g408.sv:4:37: Can't find definition of 'u'"
    let (out, err, c) = run("module ch; logic [7:0] q = 8'h7; endmodule\n\
         module top; ch u [0:0] ();\n\
           initial begin $display(\"A=%0d\", u.q); #1 $finish; end\n\
         endmodule\n");
    assert!(is_loud(&out, &err, c), "{out}{err}");
    assert!(!out.contains("A=7"), "{out}");
}

#[test]
fn bare_label_on_a_one_element_ansi_ported_array_is_loud() {
    // The PRE-EXISTING twin of the cell above (loud on neither PRE nor POST).
    // iverilog:  "g409.sv:5: error: Unable to bind wire/reg/memory `u.q' in `g409'"
    // verilator: "%Error: g409.sv:5:41: Can't find definition of 'u'"
    let (out, err, c) = run(
        "module ch(input logic [7:0] p); logic [7:0] q; assign q = p; endmodule\n\
         module top; logic [7:0] w = 8'h7; ch u [0:0] (.p(w));\n\
           initial begin #1; $display(\"A=%0d\", u.q); #1 $finish; end\n\
         endmodule\n",
    );
    assert!(is_loud(&out, &err, c), "{out}{err}");
    assert!(!out.contains("A=7"), "{out}");
}

#[test]
fn bare_label_on_a_one_element_empty_ansi_array_is_loud() {
    // The third spelling of the same module header (`module ch();`), also
    // PRE-EXISTING. iverilog: "g411.sv:4: error: Unable to bind wire/reg/memory
    // `u.q' in `g411'"; verilator: "%Error: g411.sv:4:37: Can't find definition
    // of 'u'".
    let (out, err, c) = run("module ch(); logic [7:0] q = 8'h7; endmodule\n\
         module top; ch u [0:0] ();\n\
           initial begin $display(\"A=%0d\", u.q); #1 $finish; end\n\
         endmodule\n");
    assert!(is_loud(&out, &err, c), "{out}{err}");
    assert!(!out.contains("A=7"), "{out}");
}

#[test]
fn indexed_element_of_a_one_element_array_still_works() {
    // The control the refusal above must not take with it: the LEGAL spelling of
    // the same net. Both oracles: A=7.
    let (out, err, _c) = run("module ch; logic [7:0] q = 8'h7; endmodule\n\
         module top; ch u [0:0] ();\n\
           initial begin $display(\"A=%0d\", u[0].q); #1 $finish; end\n\
         endmodule\n");
    assert!(out.contains("A=7"), "out:\n{out}err:\n{err}");
}

#[test]
fn connection_to_a_portless_child_array_is_loud() {
    // iverilog: "Wrong number of ports. Expecting at most 0, got 1."
    // verilator: PINNOTFOUND. Admitting the portless child must not admit this.
    let (out, err, c) = run(&format!(
        "{PORTLESS}module top; ch w[1:0](1'b0);\n\
           initial begin $display(\"Q=%0d\", w[0].q); #1 $finish; end\n\
         endmodule\n"
    ));
    assert!(is_loud(&out, &err, c), "{out}{err}");
}

#[test]
fn nonempty_nonansi_child_array_stays_loud() {
    // OUT OF SCOPE and recorded: both oracles run this (Q=4 4). The refusal is a
    // FALSE loud, but its fix is the body-`PortDecl` width read this lane does not
    // do — a separate slice. Pinned so the portless admission cannot drift into it.
    let (out, err, c) = run("module ch(a); input a; int q = 4; endmodule\n\
         module top; wire s = 1'b0; ch w[1:0](s);\n\
           initial begin $display(\"Q=%0d %0d\", w[0].q, w[1].q); #1 $finish; end\n\
         endmodule\n");
    assert!(is_loud(&out, &err, c), "{out}{err}");
}

#[test]
fn ansi_ported_child_array_unchanged() {
    // The lane this slice widened INTO must be byte-for-byte what it was.
    // Both oracles: Q=4 4.
    let (out, err, _c) = run("module ch(input logic a); int q = 4; endmodule\n\
         module top; wire s = 1'b0; ch w[1:0](s);\n\
           initial begin $display(\"Q=%0d %0d\", w[0].q, w[1].q); #1 $finish; end\n\
         endmodule\n");
    assert!(out.contains("Q=4 4"), "out:\n{out}err:\n{err}");
}

#[test]
fn portless_array_obs_listing_matches_the_ported_twin() {
    // G2 OBS: `--hier-tree` and `--inst-paths` must list a portless array's elements
    // exactly as they list the ANSI-ported twin's.
    const OBS: &[&str] = &["--hier-tree", "ht.txt", "--inst-paths", "ip.txt"];
    let (_o, _e, _c, d1) = run_args(
        "module ch; int q = 4; endmodule\n\
         module top; ch w[1:0]();\n  initial begin #1 $finish; end\nendmodule\n",
        OBS,
    );
    let (_o, _e, _c, d2) = run_args(
        "module ch(input logic a); int q = 4; endmodule\n\
         module top; wire s = 1'b0; ch w[1:0](s);\n  initial begin #1 $finish; end\n\
         endmodule\n",
        OBS,
    );
    for f in ["ht.txt", "ip.txt"] {
        let a = std::fs::read_to_string(d1.join(f)).unwrap();
        let b = std::fs::read_to_string(d2.join(f)).unwrap();
        assert_eq!(a, b, "{f}: portless listing must equal the ported twin's");
        assert!(a.contains("w[0]") && a.contains("w[1]"), "{f}:\n{a}");
    }
}
