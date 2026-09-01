//! A continuous assign whose RHS reaches a user FUNCTION was re-evaluated on every
//! settle pass, forever. ROADMAP §3 ⑧'s other half.
//!
//! `levelize::ca_deps` certifies which assigns the dirty settle may SKIP when none of
//! their dependency nets moved, and `expr_is_pure_of_nets` answered `false` for every
//! `Expr::Call` — "a user function can read state no net records". True, and it made
//! the answer "re-evaluate always" instead of "collect what it reads". `verilog-ethernet`
//! pays for that eighty times over: `lfsr.v` generates one
//! `wire [39:0] mask = lfsr_mask(n);` per LFSR bit, `n` is a GENVAR, and each of those
//! eighty constant-argument calls ran on every pass of every delta of every cycle.
//! Measured with `--obs-procs-time` over a 20-cycle run: **99.99% of the run** in those
//! two source lines, 240 evaluations each. The pinned `+N=1000` run took ~38 HOURS
//! against iverilog's 7.62 s; it now takes **2.2 s**, with the same digest.
//!
//! ⭐ **What has to be true is not what it looks like.** The certification is not
//! "the RHS is a mathematical function" — it is that the DEPENDENCY SET IS COMPLETE,
//! because both reference simulators re-evaluate a continuous assign exactly when a net
//! in its sensitivity list moves, and reproducing that rule reproduces their answer.
//! That distinction is what the cells below pin from both sides:
//!
//! * a function that reads a module net through NO argument keeps tracking it, because
//!   its reads are collected — and that is not a free choice, it is an ORACLE SPLIT
//!   (iverilog freezes such a `wire` at its t0 value, verilator tracks it) on which vita
//!   was already on verilator's side and must stay;
//! * a function that carries a STATIC local between calls keeps working, because
//!   iverilog carries the same state and still only calls it when a dependency moves —
//!   evaluating on dependency changes is what reproduces it, and `ca_always`'s
//!   every-pass evaluation was the anomaly.
//!
//! ⭐ The side effect that proves the rule: a `$display` inside such a function — with
//! nothing for it to depend on — printed **30 times** over a five-cycle run in the PRE
//! binary and prints **once** now, which is what iverilog AND verilator print. Its count
//! was never "once per pass"; it was always "once per evaluation", and the evaluation
//! count is the thing this fixes.
//!
//! ⚠️ Give the same function a DEPENDENCY and the number changes with it (review
//! measured 26 → 3) — that is the same rule, not a shortfall, and it is a number neither
//! oracle can arbitrate: iverilog's sensitivity list for such an assign is empty, and
//! verilator aborts on the first `$error`.
//!
//! ⚠️ A `SysFunc` still declines, so `assign m = f()` where `f` reads `$random` or
//! `$time` keeps vita's every-pass answer while both oracles freeze it. That is
//! pre-existing and untouched here: `$random` advances a seed other readers draw from
//! and `$fgetc` advances a file position, and neither is a net, so a certified set
//! could not name what changed. ROADMAP §2.
//!
//! Values pinned to iverilog 13.0 and verilator 5.050.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_cacd_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.v");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (s, out.status.success())
}

/// The driver the cells share: five posedges, with `a`, `z` and `mem[1]` all moving so a
/// dependency that was dropped shows up as a frozen column.
fn design(decls: &str, print: &str) -> String {
    format!(
        "module top;\n  reg clk; reg [7:0] c; reg [7:0] a; reg [7:0] z; reg [7:0] mem [0:3];\n  \
           integer i;\n{decls}  \
           initial begin clk=0; c=0; a=0; z=0; for(i=0;i<4;i=i+1) mem[i]=8'd0; end\n  \
           always #1 clk = ~clk;\n  \
           always @(posedge clk) begin\n    \
             c <= c + 8'd1; a <= a + 8'd3; z <= z + 8'd10; mem[1] <= mem[1] + 8'd5;\n    \
             $display({print});\n    \
             if (c==4) $finish;\n  end\n\
         endmodule\n"
    )
}

/// The headline shape, minimised out of `lfsr.v`: a generate loop whose every arm declares
/// `wire = f(<genvar>)`. Constant argument, no dependency, one evaluation — and the
/// values are what both oracles print.
#[test]
fn a_constant_argument_call_in_a_generate_loop_is_the_verilog_ethernet_shape() {
    let (o, ok) = run(&design(
        "  function [7:0] f(input [7:0] x); begin f = x ^ 8'hA5; end endfunction\n  \
           genvar n; wire [7:0] mm [0:3];\n  \
           generate for (n=0;n<4;n=n+1) begin : gl\n    \
             wire [7:0] mv = f(n[7:0]); assign mm[n] = mv;\n  end endgenerate\n",
        r#""OUT c=%0d m=%0d %0d %0d %0d", c, mm[0], mm[1], mm[2], mm[3]"#,
    ));
    assert!(ok, "vita failed:\n{o}");
    assert!(
        o.contains("OUT c=4 m=165 164 167 166"),
        "iverilog and verilator both print 165 164 167 166:\n{o}"
    );
}

/// ⚠️ The cell that says the certification is about the DEPENDENCY SET, not purity: `f`
/// reads `z` through no argument, so `z` has to be collected or `m` freezes at 1.
///
/// This one is an ORACLE SPLIT and vita is on verilator's side: iverilog gives the
/// continuous assign an EMPTY sensitivity list and prints `m=1` five times. Certifying
/// with an incomplete set would have silently switched vita to iverilog's answer, which
/// is why the census measured this cell before the change and after.
#[test]
fn a_function_reading_a_module_net_through_no_argument_still_tracks_it() {
    let (o, ok) = run(&design(
        "  function [7:0] f(input [7:0] x); begin f = x + z; end endfunction\n  \
           wire [7:0] m = f(8'd1);\n",
        r#""OUT c=%0d m=%0d", c, m"#,
    ));
    assert!(ok, "vita failed:\n{o}");
    for (c, m) in [(0, 1), (1, 11), (2, 21), (3, 31), (4, 41)] {
        assert!(
            o.contains(&format!("OUT c={c} m={m}")),
            "verilator agrees; iverilog freezes at 1 (empty sensitivity list):\n{o}"
        );
    }
}

/// …and the same through TWO levels of call, which is the transitive-closure half of
/// `func_read_deps` rather than its per-body walk.
#[test]
fn a_net_read_two_calls_deep_is_still_a_dependency() {
    let (o, ok) = run(&design(
        "  function [7:0] g(input [7:0] x); begin g = x + z; end endfunction\n  \
           function [7:0] f(input [7:0] x); begin f = g(x); end endfunction\n  \
           wire [7:0] m = f(8'd5);\n",
        r#""OUT c=%0d m=%0d", c, m"#,
    ));
    assert!(ok, "vita failed:\n{o}");
    assert!(o.contains("OUT c=4 m=45"), "{o}");
}

/// An ARRAY net read from inside the function, at a constant index. `note_change` marks
/// the whole net on a word write, so the whole net is the right granularity for the
/// dependency — but only if the read is collected at all.
#[test]
fn an_array_net_read_inside_the_function_is_a_dependency() {
    let (o, ok) = run(&design(
        "  function [7:0] f(input [7:0] x); begin f = x + mem[1]; end endfunction\n  \
           wire [7:0] m = f(8'd1);\n",
        r#""OUT c=%0d m=%0d", c, m"#,
    ));
    assert!(ok, "vita failed:\n{o}");
    assert!(o.contains("OUT c=4 m=21"), "{o}");
}

/// ⚠️ The cell that looked like the worst hazard and is not one. `f` keeps `t` between
/// calls (a plain Verilog function's locals are static), so its value depends on how
/// many times it ran — and evaluating it on dependency changes is exactly what iverilog
/// does with exactly the same carried state. All three tools agree on every column.
#[test]
fn a_static_local_carried_between_calls_agrees_with_both_oracles() {
    let (o, ok) = run(&design(
        "  function [7:0] f(input [7:0] x);\n    reg [7:0] t;\n    \
             begin if (x[0]) t = x; f = t; end\n  endfunction\n  \
           wire [7:0] m = f(a);\n",
        r#""OUT c=%0d m=%0d", c, m"#,
    ));
    assert!(ok, "vita failed:\n{o}");
    // iverilog prints exactly these; verilator differs only in reading the
    // uninitialised local as 0 rather than x.
    for s in [
        "OUT c=0 m=x",
        "OUT c=1 m=3",
        "OUT c=2 m=3",
        "OUT c=3 m=9",
        "OUT c=4 m=9",
    ] {
        assert!(o.contains(s), "expected `{s}`:\n{o}");
    }
}

/// ⭐ A pre-existing silent-wrong this closed on the way past: the print count follows
/// the EVALUATION count, and the evaluation count was "every settle pass". PRE printed
/// `tick` 30 times; both oracles print it once.
#[test]
fn a_display_inside_the_function_prints_once_like_both_oracles() {
    let (o, ok) = run(&design(
        "  function [7:0] f(input [7:0] x); begin $display(\"OUT tick\"); f = x; end \
           endfunction\n  wire [7:0] m = f(8'd5);\n",
        r#""OUT c=%0d m=%0d", c, m"#,
    ));
    assert!(ok, "vita failed:\n{o}");
    assert_eq!(
        o.matches("OUT tick").count(),
        1,
        "PRE printed this 30 times; iverilog and verilator print it once:\n{o}"
    );
}

/// ⚠️ NOT fixed, and pinned so it cannot drift silently: a `SysFunc` in the body
/// declines, so this keeps vita's every-pass answer while both oracles freeze `m`.
/// Closing it means naming what `$random` advances, and a seed is not a net.
#[test]
fn a_random_in_the_body_still_declines_and_still_varies() {
    let (o, ok) = run(&design(
        "  function [7:0] f(input [7:0] x); begin f = x + $random; end endfunction\n  \
           wire [7:0] m = f(8'd5);\n",
        r#""OUT c=%0d m=%0d", c, m"#,
    ));
    assert!(ok, "vita failed:\n{o}");
    let first = o
        .lines()
        .find(|l| l.starts_with("OUT c=0 "))
        .expect("a c=0 line");
    let last = o
        .lines()
        .find(|l| l.starts_with("OUT c=4 "))
        .expect("a c=4 line");
    assert_ne!(
        first.trim_start_matches("OUT c=0 "),
        last.trim_start_matches("OUT c=4 "),
        "KNOWN-WRONG: both oracles freeze this value; vita re-draws every pass:\n{o}"
    );
}

/// A dependency reached only through the LVALUE's index expression, which `ca_deps`
/// collects separately from the RHS — the arm that had to learn about calls twice.
#[test]
fn a_call_in_an_lvalue_index_keeps_its_dependency() {
    let (o, ok) = run(&design(
        "  function [1:0] f(input [7:0] x); begin f = x[1:0]; end endfunction\n  \
           wire [7:0] w; assign w[f(a)] = 1'b1;\n",
        r#""OUT c=%0d w=%b", c, w"#,
    ));
    assert!(ok, "vita failed:\n{o}");
    // verilator agrees on the driven bits (it reads the undriven ones as 0, vita as z).
    assert!(o.contains("OUT c=3 w=zzzz1111"), "{o}");
}

/// ⚠️ The structural check: `func_read_deps` is keyed by `FuncId`, and v1 FLATTENS a
/// design per instance. Two instances of the same module, each with its own `z` and its
/// own value of the parameter the call passes, must not share one dependency set — and
/// they do not, because elaborate gives each instance its own nets. verilator agrees
/// column for column; iverilog freezes both at their time-zero value for the reason the
/// module docstring gives.
#[test]
fn two_instances_of_the_same_function_keep_separate_dependencies() {
    let (o, ok) = run(
        "module sub #(parameter K = 8'd0) (input wire clk, output wire [7:0] m);\n  \
           reg [7:0] z;\n  \
           function [7:0] f(input [7:0] x); begin f = x + z; end endfunction\n  \
           assign m = f(K);\n  initial z = 8'd0;\n  \
           always @(posedge clk) z <= z + K + 8'd1;\n\
         endmodule\n\
         module top;\n  reg clk; reg [7:0] c; wire [7:0] m0, m1;\n  \
           sub #(8'd10) u0(clk, m0);\n  sub #(8'd20) u1(clk, m1);\n  \
           initial begin clk=0; c=0; end\n  always #1 clk = ~clk;\n  \
           always @(posedge clk) begin c<=c+8'd1;\n    \
             $display(\"OUT c=%0d m0=%0d m1=%0d\", c, m0, m1);\n    \
             if (c==4) $finish; end\n\
         endmodule\n",
    );
    assert!(ok, "vita failed:\n{o}");
    assert!(o.contains("OUT c=4 m0=54 m1=104"), "verilator agrees:\n{o}");
}

/// ⚠️ A dependency net written by a system TASK rather than by an assignment —
/// `$readmemh` fills the memory the callee reads. Its write goes through the same
/// `note_change` funnel the dirty settle listens on, so the certified assign still sees
/// it; PRE and POST are identical here, and the one-delta offset against the oracles is
/// pre-existing (iverilog is a delta later, verilator a delta earlier).
#[test]
fn a_dependency_written_by_readmemh_is_still_noticed() {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_cacd_rm_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("m1.hex"), "0a\n0b\n0c\n0d\n").unwrap();
    std::fs::write(d.join("m2.hex"), "aa\nbb\ncc\ndd\n").unwrap();
    let f = d.join("t.v");
    std::fs::write(
        &f,
        "module top;\n  reg clk; reg [7:0] c; reg [7:0] mem [0:3];\n  \
           function [7:0] f(input [7:0] x); begin f = mem[1] + x; end endfunction\n  \
           wire [7:0] m = f(8'd1);\n  \
           initial begin clk=0; c=0; $readmemh(\"m1.hex\", mem); end\n  \
           always #1 clk = ~clk;\n  \
           always @(posedge clk) begin c <= c + 8'd1;\n    \
             if (c==2) $readmemh(\"m2.hex\", mem);\n    \
             $display(\"OUT c=%0d m=%0h\", c, m);\n    \
             if (c==4) $finish; end\n\
         endmodule\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let o = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(out.status.success(), "vita failed:\n{o}");
    assert!(o.contains("OUT c=0 m=c"), "before the reload:\n{o}");
    assert!(o.contains("OUT c=4 m=bc"), "after it:\n{o}");
}

/// ⚠️⚠️ **The cell adversarial review built, and the reason the certification has a
/// second condition.** `bump` keeps a counter in a static local, so its value is a
/// function of HOW MANY TIMES it has run — and how many times it runs is exactly what
/// certifying the call changes. With a live dependency the first version of this slice
/// answered `2 3 3` where PRE answered `3 3 3` (= verilator exactly) and iverilog says
/// `1 2 3`: correct → agreeing with nobody.
///
/// The assign is now refused certification (`own_reads_are_definitely_assigned` sees `s`
/// read before it is written) and keeps PRE's answer. The residual disagreement with
/// iverilog is vita's own extra t0 settle pass and is pre-existing.
#[test]
fn a_counter_carried_in_a_static_local_is_not_certified_while_it_has_a_dependency() {
    let (o, ok) = run("module tb;\n  reg [7:0] z=1; wire [7:0] m,d;\n  \
           function [7:0] bump(input [7:0] a);\n    reg [7:0] s; integer i;\n    \
             begin for(i=0;i<1;i=i+1) begin\n      \
                 if (s === 8'hxx) s = 8'd0;\n      \
                 if (s <  8'd3)   s = s + 8'd1;\n    end\n    bump = s; end\n  \
           endfunction\n  assign m = bump(z);\n  assign d = z;\n  \
           initial begin\n    #1 $display(\"OUT A m=%0d d=%0d\",m,d);\n    \
             z=8'd7; #1 $display(\"OUT B m=%0d d=%0d\",m,d);\n    \
             z=8'd9; #1 $display(\"OUT C m=%0d d=%0d\",m,d);\n    $finish; end\n\
         endmodule\n");
    assert!(ok, "vita failed:\n{o}");
    assert!(
        o.contains("OUT A m=3") && o.contains("OUT B m=3") && o.contains("OUT C m=3"),
        "PRE's answer, which is verilator's exactly; certifying this gave 2 3 3, which \
         is nobody's:\n{o}"
    );
}

/// …and the same function with its DEPENDENCY REMOVED, which is the other arm of the
/// disjunct and a loud → correct move. With no dependency the assign is evaluated once,
/// at the settle seed, so "how many times" is one and cannot vary — which is what both
/// oracles do with an empty sensitivity list.
///
/// ⭐ PRE could not run this at all: re-evaluating the counter on every settle pass never
/// reaches a fixpoint, so PRE reports `F4016 did not converge` and exits 1 (verilator
/// independently fails to converge on it too). POST prints iverilog's answer.
#[test]
fn the_same_counter_with_no_dependency_is_evaluated_once_and_matches_iverilog() {
    let (o, ok) = run("module tb;\n  wire [7:0] m;\n  \
           function [7:0] bump(input [7:0] a);\n    reg [7:0] s; integer i;\n    \
             begin for(i=0;i<1;i=i+1) begin\n      \
                 if (s === 8'hxx) s = 8'd0;\n      \
                 s = s + 8'd1;\n    end\n    bump = s; end\n  \
           endfunction\n  assign m = bump(8'd0);\n  \
           initial begin #1 $display(\"OUT A m=%0d\",m); #1 $display(\"OUT B m=%0d\",m); \
             $finish; end\n\
         endmodule\n");
    assert!(
        ok,
        "PRE refused this with F4016 (did not converge); it must run now:\n{o}"
    );
    assert!(
        o.contains("OUT A m=1") && o.contains("OUT B m=1"),
        "iverilog prints 1 and 1:\n{o}"
    );
}

/// ⚠️⚠️ **Round-2 review, BLOCKING 1 of 3: the empty-dependency arm's premise was false.**
/// "No dependencies ⇒ evaluated once at the settle seed" is not true on every path —
/// `k_release` calls `redirty_drivers_of` on its target UNCONDITIONALLY (deliberately, so
/// a released wire snaps back in the same settle), so a certified assign was evaluated
/// `1 + releases` times. Both oracles evaluate it once whatever the release count, and a
/// `release` with no matching `force` is a no-op in both.
///
/// ⭐ The census that made the fix precise rather than defensive: `ca_dirty_flag[..] = true`
/// has exactly THREE producers — the seed, `note_change` (a dependency really moved), and
/// `redirty_drivers_of`, whose only callers are the two `k_release` twins. So naming the
/// force/release targets names every evaluation not caused by a dependency change.
#[test]
fn a_release_on_the_driven_net_blocks_the_empty_dependency_arm() {
    let src = |sat: &str| {
        format!(
            "module t;\n  \
               function [15:0] cnt(input [7:0] x);\n    reg [15:0] c; reg init;\n    \
                 begin if (init !== 1'b1) begin c = 0; init = 1'b1; end\n      \
                   {sat}\n      cnt = c; end\n  endfunction\n  \
               wire [15:0] m; assign m = cnt(8'd1);\n  \
               initial begin\n    #5 $display(\"OUT t5 m=%0d\", m);\n    \
                 #5 release m;\n    #5 $display(\"OUT t15 m=%0d\", m);\n    \
                 #5 release m;\n    #5 $display(\"OUT t25 m=%0d\", m);\n    $finish; end\n  \
               initial #200 $finish;\n\
             endmodule\n"
        )
    };
    // Saturating: the previous release answered `3 3 3`, which is verilator's answer
    // exactly. Certifying this gave `1 2 3`, which is nobody's.
    let (o, ok) = run(&src("if (c < 3) c = c + 1;"));
    assert!(ok, "vita failed:\n{o}");
    for t in ["OUT t5 m=3", "OUT t15 m=3", "OUT t25 m=3"] {
        assert!(o.contains(t), "expected `{t}` (verilator's answer):\n{o}");
    }
    // Non-saturating: re-evaluating forever never settles, so this is a LOUD refusal and
    // must stay one. Certifying it turned the fatal into a silent `1 2 3`.
    let (o2, ok2) = run(&src("c = c + 1;"));
    assert!(
        !ok2 && o2.contains("F-RUN-NO-CONVERGE"),
        "a counter re-evaluated by every release cannot settle; that must stay loud:\n{o2}"
    );
}

/// ⚠️⚠️ **Round-2 BLOCKING 2: a zero-parameter function's RETURN slot was read as a
/// formal.** The entry set counted with `insert` before the bound check, so a function
/// with `n_params == 0` got one net anyway — the func-named return variable — and a
/// counter kept in it read as definitely assigned. That resurrected round 1's BLOCKING
/// through a one-token change to its own repro.
#[test]
fn a_counter_in_the_return_variable_of_a_zero_parameter_function_is_not_certified() {
    let (o, ok) = run("module tb;\n  reg [7:0] z; wire [7:0] m, d;\n  \
           function [7:0] bump();\n    \
             begin if (bump === 8'hxx) bump = 8'd0;\n      \
               if (bump < 8'd3) bump = bump + 8'd1; end\n  endfunction\n  \
           assign m = bump() + (z & 8'd0);\n  assign d = z;\n  \
           initial begin z = 8'd1;\n    #1 $display(\"OUT A m=%0d\", m);\n    \
             z = 8'd7; #1 $display(\"OUT B m=%0d\", m);\n    \
             z = 8'd9; #1 $display(\"OUT C m=%0d\", m);\n    $finish; end\n  \
           initial #100 $finish;\n\
         endmodule\n");
    assert!(ok, "vita failed:\n{o}");
    assert!(
        o.contains("OUT A m=3") && o.contains("OUT B m=3") && o.contains("OUT C m=3"),
        "the previous release's answer; certifying this gave 2 3 3, which is nobody's:\n{o}"
    );
}

/// ⚠️⚠️ **Round-2 BLOCKING 3: a PARTIAL write established definite assignment for the
/// whole net.** The docstring called this "array-word imprecision … a shape no corpus
/// design or probe has produced". Neither half was true: this launders a per-call counter
/// through a packed part-select on a plain `reg [15:0]`, with no array anywhere, and the
/// previous release refused the design loudly.
#[test]
fn a_partial_write_does_not_establish_definite_assignment() {
    let (o, ok) = run("module tb;\n  reg [7:0] z; wire [7:0] m, d;\n  \
           function [7:0] bump(input [7:0] a);\n    reg [15:0] t;\n    \
             begin t[15:8] = a;\n      if (t[7:0] === 8'hxx) t[7:0] = 8'd0;\n      \
               t[7:0] = t[7:0] + 8'd1;\n      bump = t[7:0]; end\n  endfunction\n  \
           assign m = bump(z);\n  assign d = z;\n  \
           initial begin z = 8'd1;\n    #1 $display(\"OUT A m=%0d\", m);\n    \
             z = 8'd7; #1 $display(\"OUT B m=%0d\", m);\n    $finish; end\n  \
           initial #100 $finish;\n\
         endmodule\n");
    assert!(
        !ok && o.contains("F-RUN-NO-CONVERGE"),
        "a counter that advances on every evaluation cannot settle; certifying it \
         answered a silent `2 3 4`:\n{o}"
    );
}
