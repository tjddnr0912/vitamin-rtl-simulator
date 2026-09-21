//! §3.b `iface-subr` — a `function` or `task` DECLARED inside an `interface` body.
//!
//! ## The defect
//!
//! `elaborate_iface_instances` (`elaborate/src/iface_inst.rs`) ended its Logic loop
//! with a catch-all that refused `ast::ModuleItem::Func` / `Task` as
//! `VITA-E3009 functions/tasks inside an interface are outside the MVP`. Because the
//! declaration never reached `func_table` / `task_table` either, every call to it was
//! a second diagnostic, `VITA-E3010 call to undeclared function/task`, and a
//! `localparam` folded through a declared constant function was
//! `VITA-E3009 … has no constant-fold arm`. 21 cells that both oracles run were loud:
//! a plain function, a task with an output formal, a function with a block-local, a
//! recursive one, a loop body, a task with a delay, a void function, a function in a
//! continuous assign and in `always_comb`, a packed return, a header-parameter default
//! and a body `localparam` folded through a declared function, `%m`, and every
//! per-instance shape (a function writing the interface's own net, a static local, a
//! parameter read).
//!
//! ## The fix
//!
//! The interface window binds the declarations exactly where the module lane does:
//! the body's functions join `const_func_table` before `bind_params` (the module
//! lane's step 3a.5), the `function`/`task` declarations join the routine tables
//! before `apply_import_routines` (step 3.5) through the ONE registration both lanes
//! share (`elaborate/src/rtn_decl.rs`), the containment gate runs over the routine
//! bodies in the same loop that gates the `Proc` bodies — after the five block-local
//! classifier maps are installed, which is later in this lane than in the module one —
//! and the Logic loop treats Func/Task as definitions. `lower_frame_funcs` (step 6.5)
//! was already in the window and classifies them from there.
//!
//! ## Oracles
//!
//! Every value pinned here was measured three-way against iverilog 13.0 (`-g2012` +
//! `vvp -n`) and verilator 5.052 (`--binary --timing`), which agree on every one of
//! them, except where a test's doc says otherwise. Where the oracles SPLIT — `p.t()`
//! through an interface port, a duplicate declaration — no VALUE is pinned: the cell
//! pins the refusal, or the parity between the interface and module lanes, and its
//! doc names both oracles' positions.
//!
//! ## Sibling file
//!
//! `iface_subr_scope.rs` holds the row's NAME-RESOLUTION half — which binding a name
//! in an interface body resolves to (§26.3 import-vs-declaration over both halves of
//! the §3.13 routine name space, §26.4 `$unit` shadowing, the modport name space) and
//! when a name exists (§6.10 use before declaration). Split at the 1000-line policy.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_ifacesubr_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).expect("scratch dir");
    std::fs::write(d.join("t.sv"), src).expect("write design");
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("t.sv")
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (s, out.status.code())
}

/// A clean run (exit 0) whose output contains every `want` line and none of `absent`.
/// `VITA-E3010` is in every cell's `absent` list: the second diagnostic the refusal
/// produced was the call, so a fix that registered the declaration but not its body
/// would still show up here.
fn lines(src: &str, want: &[&str], absent: &[&str]) {
    let (o, code) = run(src);
    assert_eq!(code, Some(0), "expected a clean run:\n{o}");
    for w in want {
        assert!(o.contains(w), "expected {w:?} in:\n{o}");
    }
    for a in absent {
        assert!(!o.contains(a), "did NOT expect {a:?} in:\n{o}");
    }
}

/// The two diagnostics this row removed. Passed as `absent` by every value cell.
const GONE: &[&str] = &["VITA-E3009", "VITA-E3010"];

// --------------------------------------------------------------- the row's own cells

/// d01: the headline cell — a function, a task with an `output` formal, and a function
/// holding a named block-local, all declared in one interface body and called from the
/// interface's own `initial`. Both oracles `F=44 T=103 B=41`.
#[test]
fn a_function_a_task_and_a_block_local_function() {
    lines(
        "interface ifc; int r;\n\
         \x20 function automatic int f(input int a); return a + 4; endfunction\n\
         \x20 task automatic t(input int a, output int o); o = a + 100; endtask\n\
         \x20 function int bl(input int a); begin : b int x; x = a * 2; return x + 1; end endfunction\n\
         \x20 initial begin int q; r = f(40); t(3, q); $display(\"F=%0d T=%0d B=%0d\", r, q, bl(20)); end\n\
         endinterface\n\
         module top; ifc i(); initial #2 $finish; endmodule\n",
        &["F=44 T=103 B=41"],
        GONE,
    );
}

/// m01: the MODULE twin of the cell above, byte-identical text with `interface` →
/// `module`. Correct before this row and after it — the control that attributes the
/// change to the interface lane rather than to the routine machinery.
#[test]
fn the_module_twin_of_the_headline_cell() {
    lines(
        "module ifc; int r;\n\
         \x20 function automatic int f(input int a); return a + 4; endfunction\n\
         \x20 task automatic t(input int a, output int o); o = a + 100; endtask\n\
         \x20 function int bl(input int a); begin : b int x; x = a * 2; return x + 1; end endfunction\n\
         \x20 initial begin int q; r = f(40); t(3, q); $display(\"F=%0d T=%0d B=%0d\", r, q, bl(20)); end\n\
         endmodule\n\
         module top; ifc i(); initial #2 $finish; endmodule\n",
        &["F=44 T=103 B=41"],
        GONE,
    );
}

/// d02: §26.3 — a DECLARED `g` beside `import pk::*` wins the wildcard. This is why
/// the registration has to run BEFORE `apply_import_routines`, which is
/// skip-if-present: with the two in the other order the package's `+4` would have
/// answered. Both oracles `R=1040`.
#[test]
fn a_declared_routine_wins_a_wildcard_import() {
    lines(
        "package pk; function automatic int g(input int a); return a + 4; endfunction endpackage\n\
         interface ifc; import pk::*; int r;\n\
         \x20 function automatic int g(input int a); return a + 1000; endfunction\n\
         \x20 initial begin r = g(40); $display(\"R=%0d\", r); end\n\
         endinterface\n\
         module top; ifc i(); initial #2 $finish; endmodule\n",
        &["R=1040"],
        GONE,
    );
}

/// m02: the MODULE twin of the wildcard-shadow cell. Both oracles `R=1040`.
#[test]
fn the_module_twin_of_the_wildcard_shadow() {
    lines(
        "package pk; function automatic int g(input int a); return a + 4; endfunction endpackage\n\
         module ifc; import pk::*; int r;\n\
         \x20 function automatic int g(input int a); return a + 1000; endfunction\n\
         \x20 initial begin r = g(40); $display(\"R=%0d\", r); end\n\
         endmodule\n\
         module top; ifc i(); initial #2 $finish; endmodule\n",
        &["R=1040"],
        GONE,
    );
}

/// d23: the CONSTANT half of the same §26.3 rule — a declared constant function beside
/// `import pk::*`, folded in a `localparam`. `local_const_funcs` was an empty set in
/// this window before the row, so the wildcard had nothing to lose to. Both oracles
/// `W=44` (`4*11`, the interface's own `f`, not `pk::f`'s `4+4`).
#[test]
fn a_declared_const_function_wins_a_wildcard_import() {
    lines(
        "package pk; function automatic int f(input int a); return a + 4; endfunction endpackage\n\
         interface ifc; import pk::*;\n\
         \x20 function automatic int f(input int a); return a * 11; endfunction\n\
         \x20 localparam int W = f(4);\n\
         \x20 initial $display(\"W=%0d\", W);\n\
         endinterface\n\
         module top; ifc i(); initial #2 $finish; endmodule\n",
        &["W=44"],
        GONE,
    );
}

/// d12: a declared function whose body calls an IMPORTED package function — the two
/// lanes fill one table, so the declaration must not displace the import. Both oracles
/// `R=48` (`(20+4)*2`).
#[test]
fn a_declared_function_calls_an_imported_one() {
    lines(
        "package pk; function automatic int h(input int a); return a + 4; endfunction endpackage\n\
         interface ifc; import pk::h; int r;\n\
         \x20 function automatic int f(input int a); return h(a) * 2; endfunction\n\
         \x20 initial begin r = f(20); $display(\"R=%0d\", r); end\n\
         endinterface\n\
         module top; ifc i(); initial #2 $finish; endmodule\n",
        &["R=48"],
        GONE,
    );
}

// ------------------------------------------------------- one routine per INSTANCE

/// d04b: a declared function that WRITES the interface's own net, in a parameterised
/// interface instantiated twice. Each instance's `cnt` is its own and each sees its own
/// `A`. Both oracles `O1=6 O2=10 N1=6 N2=10`.
#[test]
fn a_declared_function_writes_its_own_instance_net() {
    lines(
        "interface ifc #(parameter int A = 1); int cnt = 0; int o;\n\
         \x20 function int bump(); cnt = cnt + A; return cnt; endfunction\n\
         \x20 initial begin o = bump(); o = bump(); end\n\
         endinterface\n\
         module top; ifc #(3) u1(); ifc #(5) u2();\n\
         \x20 initial begin #1 $display(\"O1=%0d O2=%0d N1=%0d N2=%0d\", u1.o, u2.o, u1.cnt, u2.cnt); #1 $finish; end\n\
         endmodule\n",
        &["O1=6 O2=10 N1=6 N2=10"],
        GONE,
    );
}

/// d05: the STATIC local of a routine DECLARED in an interface is ONE VARIABLE PER
/// INSTANCE — unlike a PACKAGE routine's static local, which is one design-wide
/// (ROADMAP §2) and which `static_scoped_keys` carries between sibling interface
/// instances. A declared routine's key is a bare name with no `::`, so that carry
/// never sees it. Both oracles `L1=10 L2=10`; the module twin below is identical.
#[test]
fn a_declared_static_local_is_per_instance() {
    lines(
        "interface ifc; int o;\n\
         \x20 function int st(input int a); int x; x = x + a; return x; endfunction\n\
         \x20 initial begin o = st(10); end\n\
         endinterface\n\
         module top; ifc u1(); ifc u2();\n\
         \x20 initial begin #1 $display(\"L1=%0d L2=%0d\", u1.o, u2.o); #1 $finish; end\n\
         endmodule\n",
        &["L1=10 L2=10"],
        GONE,
    );
}

/// m05: the MODULE twin of the static-local cell. Both oracles `L1=10 L2=10`.
#[test]
fn the_module_twin_of_the_static_local() {
    lines(
        "module ifc; int o;\n\
         \x20 function int st(input int a); int x; x = x + a; return x; endfunction\n\
         \x20 initial begin o = st(10); end\n\
         endmodule\n\
         module top; ifc u1(); ifc u2();\n\
         \x20 initial begin #1 $display(\"L1=%0d L2=%0d\", u1.o, u2.o); #1 $finish; end\n\
         endmodule\n",
        &["L1=10 L2=10"],
        GONE,
    );
}

/// d15: a declared task reads its instance's PARAMETER, two instances overridden
/// differently. Both oracles `T1=101 T2=102`.
#[test]
fn a_declared_task_reads_its_instance_parameter() {
    lines(
        "interface ifc #(parameter int P = 1); int o;\n\
         \x20 task automatic t(input int a, output int r); r = a + P; endtask\n\
         \x20 initial t(100, o);\n\
         endinterface\n\
         module top; ifc #(1) u1(); ifc #(2) u2();\n\
         \x20 initial begin #1 $display(\"T1=%0d T2=%0d\", u1.o, u2.o); #1 $finish; end\n\
         endmodule\n",
        &["T1=101 T2=102"],
        GONE,
    );
}

/// d16: the FUNCTION half of the same question, one instance on the declared default
/// and one overridden. Both oracles `FP1=10 FP2=14`.
#[test]
fn a_declared_function_reads_its_instance_parameter() {
    lines(
        "interface ifc #(parameter int P = 5); int o;\n\
         \x20 function int fp(); return P * 2; endfunction\n\
         \x20 initial o = fp();\n\
         endinterface\n\
         module top; ifc u1(); ifc #(7) u2();\n\
         \x20 initial begin #1 $display(\"FP1=%0d FP2=%0d\", u1.o, u2.o); #1 $finish; end\n\
         endmodule\n",
        &["FP1=10 FP2=14"],
        GONE,
    );
}

/// d09: `%m` inside a DECLARED task names the interface INSTANCE, not the interface
/// type and not the parent. This is an ORACLE pin, not a parity pin: iverilog and
/// verilator both print `top.u.sc` and `top.w.sc` (unlike `%m` inside a PACKAGE
/// routine, which is a recorded oracle split).
#[test]
fn percent_m_in_a_declared_task_names_the_instance() {
    lines(
        "interface ifc;\n\
         \x20 task automatic sc(); $display(\"SCOPE=%m\"); endtask\n\
         \x20 initial sc();\n\
         endinterface\n\
         module top; ifc u(); ifc w(); initial #2 $finish; endmodule\n",
        &["SCOPE=top.u.sc", "SCOPE=top.w.sc"],
        GONE,
    );
}

/// d13: the interface's own `g` and the PARENT's `g` of the same name, in one design.
/// The window takes the enclosing module's routine scope, so neither can see the
/// other: both oracles print `I=1040` (the interface's `+1000`) and `P=44` (the
/// parent's `+4`). The parent's half is the half a shared table would have broken.
#[test]
fn the_parents_own_routine_of_the_same_name_is_untouched() {
    lines(
        "interface ifc; int r;\n\
         \x20 function automatic int g(input int a); return a + 1000; endfunction\n\
         \x20 initial begin r = g(40); $display(\"I=%0d\", r); end\n\
         endinterface\n\
         module top; ifc i(); int p;\n\
         \x20 function automatic int g(input int a); return a + 4; endfunction\n\
         \x20 initial begin p = g(40); $display(\"P=%0d\", p); #2 $finish; end\n\
         endmodule\n",
        &["I=1040", "P=44"],
        GONE,
    );
}

// ----------------------------------------------------------- what needs a FRAME

/// d08: a recursive `automatic` function — step 6.5 has to reserve its frame before
/// its own body is lowered. Both oracles `FACT=120`.
#[test]
fn a_declared_recursive_function() {
    lines(
        "interface ifc; int o;\n\
         \x20 function automatic int fact(input int n); if (n <= 1) return 1; return n * fact(n - 1); endfunction\n\
         \x20 initial begin o = fact(5); $display(\"FACT=%0d\", o); end\n\
         endinterface\n\
         module top; ifc i(); initial #2 $finish; endmodule\n",
        &["FACT=120"],
        GONE,
    );
}

/// d07: a loop body inside a declared function. Both oracles `LP=10` (0+1+2+3+4).
#[test]
fn a_declared_function_with_a_loop() {
    lines(
        "interface ifc; int o;\n\
         \x20 function automatic int lp(input int n); int s; s = 0; for (int k = 0; k < n; k++) s = s + k; return s; endfunction\n\
         \x20 initial begin o = lp(5); $display(\"LP=%0d\", o); end\n\
         endinterface\n\
         module top; ifc i(); initial #2 $finish; endmodule\n",
        &["LP=10"],
        GONE,
    );
}

/// d06: a declared task carrying a DELAY plus an output formal — the suspendable
/// shape, which only a framed task can take. The TIME is pinned beside the value: both
/// oracles print `D=21 @1`, so the `#1` has to be inside the call, not swallowed.
#[test]
fn a_declared_task_with_a_delay_and_an_output_formal() {
    lines(
        "interface ifc; int o;\n\
         \x20 task automatic dly(input int a, output int r); #1 r = a * 3; endtask\n\
         \x20 initial begin dly(7, o); $display(\"D=%0d @%0t\", o, $time); end\n\
         endinterface\n\
         module top; ifc i(); initial #3 $finish; endmodule\n",
        &["D=21 @1"],
        GONE,
    );
}

/// d20: a `void` function used as a STATEMENT, writing the interface's own net twice.
/// Both oracles `V=7`.
#[test]
fn a_declared_void_function_as_a_statement() {
    lines(
        "interface ifc; int cnt = 0;\n\
         \x20 function void v(input int a); cnt = cnt + a; endfunction\n\
         \x20 initial begin v(3); v(4); $display(\"V=%0d\", cnt); end\n\
         endinterface\n\
         module top; ifc i(); initial #2 $finish; endmodule\n",
        &["V=7"],
        GONE,
    );
}

/// d25: a PACKED return value, so the width and the bit order travel out of the call.
/// Both oracles `R=5a` (`8'hA5` nibble-swapped).
#[test]
fn a_declared_function_with_a_packed_return() {
    lines(
        "interface ifc; logic [7:0] r;\n\
         \x20 function automatic logic [7:0] f(input logic [7:0] a); return {a[3:0], a[7:4]}; endfunction\n\
         \x20 initial begin r = f(8'hA5); $display(\"R=%h\", r); end\n\
         endinterface\n\
         module top; ifc i(); initial #2 $finish; endmodule\n",
        &["R=5a"],
        GONE,
    );
}

// ------------------------------------------------------ every caller POSITION

/// d21: a declared function called from a CONTINUOUS ASSIGN in the interface body.
/// Both oracles `W=44`. (Before the row this cell also produced a spurious
/// `VITA-E3018 continuous assign drives variable top.i.w` — a downstream artifact of
/// the unresolved call, not a real lvalue-kind verdict: the module twin and the
/// function-free interface twin were both clean.)
#[test]
fn a_declared_function_in_a_continuous_assign() {
    lines(
        "interface ifc; int r = 0; logic [31:0] w;\n\
         \x20 function automatic int f(input int a); return a + 4; endfunction\n\
         \x20 assign w = f(r);\n\
         \x20 initial begin r = 40; #1 $display(\"W=%0d\", w); end\n\
         endinterface\n\
         module top; ifc i(); initial #2 $finish; endmodule\n",
        &["W=44"],
        &["VITA-E3009", "VITA-E3010", "VITA-E3018"],
    );
}

/// d22: the same call from an `always_comb`. Both oracles `W=44`.
#[test]
fn a_declared_function_in_an_always_comb() {
    lines(
        "interface ifc; int r = 0; logic [31:0] w;\n\
         \x20 function automatic int f(input int a); return a + 4; endfunction\n\
         \x20 always_comb w = f(r);\n\
         \x20 initial begin r = 40; #1 $display(\"W=%0d\", w); end\n\
         endinterface\n\
         module top; ifc i(); initial #2 $finish; endmodule\n",
        &["W=44"],
        GONE,
    );
}

/// d10: a body `localparam` folded through a declared constant function — the
/// ELABORATE-TIME caller, which reads `const_func_table` rather than `func_table`.
/// Both oracles `W=44`.
#[test]
fn a_body_localparam_folds_through_a_declared_function() {
    lines(
        "interface ifc;\n\
         \x20 function automatic int cf(input int a); return a * 11; endfunction\n\
         \x20 localparam int W = cf(4);\n\
         \x20 initial $display(\"W=%0d\", W);\n\
         endinterface\n\
         module top; ifc i(); initial #2 $finish; endmodule\n",
        &["W=44"],
        GONE,
    );
}

/// d10b: the HEADER parameter default folded through a function declared in the BODY.
/// This is the cell that decides WHERE the collection runs: `bind_params` folds the
/// header, so `const_func_table` has to be filled ahead of it. Both oracles `X=44`.
#[test]
fn a_header_parameter_default_folds_through_a_body_function() {
    lines(
        "interface ifc #(parameter int X = cf(4));\n\
         \x20 function automatic int cf(input int a); return a * 11; endfunction\n\
         \x20 initial $display(\"X=%0d\", X);\n\
         endinterface\n\
         module top; ifc i(); initial #2 $finish; endmodule\n",
        &["X=44"],
        GONE,
    );
}

/// d14: a GENERATE-nested interface instance, which reaches this window through pass 8
/// rather than pass 4c. Both oracles `G=44`.
#[test]
fn a_generate_nested_interface_instance() {
    lines(
        "interface ifc; int r;\n\
         \x20 function automatic int f(input int a); return a + 4; endfunction\n\
         \x20 initial begin r = f(40); $display(\"G=%0d\", r); end\n\
         endinterface\n\
         module top; generate if (1) begin : gb ifc u(); end endgenerate initial #2 $finish; endmodule\n",
        &["G=44"],
        GONE,
    );
}

/// d04: a HIERARCHICAL call from the parent (`u1.bump(3)`), which goes through the
/// `hier_defer` function-call route rather than a bare-name lookup. That route needed
/// the callee in `func_table` too: before the row it was
/// `E3009 unsupported hierarchical function call` on top of the refusal. Both oracles
/// `C1=3 C2=5` and `N1=3 N2=5` — one `cnt` per interface instance.
#[test]
fn a_hierarchical_call_to_a_declared_interface_function() {
    lines(
        "interface ifc; int cnt = 0;\n\
         \x20 function int bump(input int a); cnt = cnt + a; return cnt; endfunction\n\
         endinterface\n\
         module top; ifc u1(); ifc u2();\n\
         \x20 initial begin #1 $display(\"C1=%0d C2=%0d\", u1.bump(3), u2.bump(5)); $display(\"N1=%0d N2=%0d\", u1.cnt, u2.cnt); #1 $finish; end\n\
         endmodule\n",
        &["C1=3 C2=5", "N1=3 N2=5"],
        GONE,
    );
}

// --------------------------------------------------------- the louds that STAY loud

/// d11: the CONTAINMENT gate, on a body that this row newly admits. An outer
/// block-local read after an inner same-named declaration must stay refused with the
/// MODULE twin's message — v1 keeps a body's block-locals in a flat table, so
/// admitting it would read the INNER `x` silently. Both oracles print
/// `LK=3 o1=2 o2=1`, so this is honest-loud, not correct: it is a recorded §2/§3 row
/// of the module lane, and the pin here is PARITY with that lane (m11 below).
///
/// It is also the cell that decides WHERE the gate runs in this window: the five
/// block-local classifier maps are installed after the import passes here, so gating
/// at registration time would have asked the PARENT module's maps.
#[test]
fn a_block_local_leak_in_a_declared_function_stays_loud() {
    let (out, code) = run("interface ifc; int o1, o2;\n\
         \x20 function int lk(input int a);\n\
         \x20   begin : outer int x; x = a;\n\
         \x20     begin : inner int x; x = 2; o1 = x; end\n\
         \x20     o2 = x;\n\
         \x20   end\n\
         \x20   return o1 + o2;\n\
         \x20 endfunction\n\
         \x20 initial begin #1 $display(\"LK=%0d o1=%0d o2=%0d\", lk(1), o1, o2); end\n\
         endinterface\n\
         module top; ifc i(); initial #2 $finish; endmodule\n");
    assert_ne!(code, Some(0), "expected the containment refusal:\n{out}");
    assert!(
        out.contains("block-local `x` is referenced outside its `begin…end` block"),
        "expected the module twin's message:\n{out}"
    );
}

/// m11: the MODULE twin of the containment cell — the same refusal, which is what
/// makes the interface one parity rather than a new rule.
#[test]
fn the_module_twin_of_the_block_local_leak() {
    let (out, code) = run("module ifc; int o1, o2;\n\
         \x20 function int lk(input int a);\n\
         \x20   begin : outer int x; x = a;\n\
         \x20     begin : inner int x; x = 2; o1 = x; end\n\
         \x20     o2 = x;\n\
         \x20   end\n\
         \x20   return o1 + o2;\n\
         \x20 endfunction\n\
         \x20 initial begin #1 $display(\"LK=%0d o1=%0d o2=%0d\", lk(1), o1, o2); end\n\
         endmodule\n\
         module top; ifc i(); initial #2 $finish; endmodule\n");
    assert_ne!(code, Some(0), "expected the containment refusal:\n{out}");
    assert!(
        out.contains("block-local `x` is referenced outside its `begin…end` block"),
        "{out}"
    );
}

/// d18: `modport mp(import f, …)` is a PARSE error and stays one — the modport
/// spelling is a separate row (iverilog: "sorry: modport task/function ports are not
/// yet supported"; verilator runs it, so it is a 1-oracle cell and out of this row).
#[test]
fn a_modport_import_of_a_routine_stays_a_parse_error() {
    let (out, code) = run(
        "interface ifc; int r; function automatic int f(input int a); return a + 4; endfunction\n\
         \x20 modport mp(import f, input r); initial r = f(40);\n\
         endinterface\n\
         module sub(ifc p); initial begin #1 $display(\"P=%0d Q=%0d\", p.r, p.f(1)); end endmodule\n\
         module top; ifc i(); sub s(i); initial begin #1 $display(\"H=%0d\", i.f(2)); #1 $finish; end endmodule\n",
    );
    assert_ne!(code, Some(0), "expected the parse refusal:\n{out}");
    assert!(out.contains("VITA-E2002"), "{out}");
}

/// d24: a task called through an interface PORT (`p.t()` inside a child module). Out of
/// this row and an ORACLE SPLIT besides — iverilog rejects the design
/// ("syntax error" on the port declaration), verilator runs it (`RT=6 HT=15`) — so
/// nothing is value-pinned. What IS pinned: vita's answer is a diagnostic and not a
/// crash, and the interface's own direct call is not what refuses it.
#[test]
fn a_task_through_an_interface_port_stays_loud() {
    let (out, code) = run("interface ifc; int r;\n\
         \x20 task t(input int a); r = a + 5; endtask\n\
         \x20 initial begin t(1); $display(\"RT=%0d\", r); end\n\
         endinterface\n\
         module sub(ifc p); initial begin #1 p.t(10); $display(\"HT=%0d\", p.r); end endmodule\n\
         module top; ifc i(); sub s(i); initial #3 $finish; endmodule\n");
    assert_ne!(code, Some(0), "expected a refusal:\n{out}");
    assert!(
        out.contains("unsupported hierarchical task call `p.t`"),
        "{out}"
    );
    assert!(!out.contains("panicked"), "{out}");
}

// --------------------------------------- a decl-initializer calling a declared routine

/// s34b (round-2 soundness S-5): a net DECL-INITIALIZER calling a FRAMED declared
/// routine. Both oracles `R=10`. Step (6.5) used to run BELOW the whole rank-scope
/// block in this window, i.e. after `collect_var_init_drivers`, so the initializer was
/// collected with no frame reserved, routed to the CONSTANT interpreter and refused as
/// `function \`lp\` body is not reducible to an expression (control flow)` — while the
/// module lane, whose order is nets → hoist → 6.5 → var-init collection, printed 10.
/// The fix is that order.
#[test]
fn a_decl_initializer_calling_a_framed_declared_function() {
    lines(
        "interface ifc;\n\
         \x20 function automatic int lp(input int n); int s; s = 0; for (int k = 0; k < n; k++) s = s + k; return s; endfunction\n\
         \x20 int r = lp(5);\n\
         \x20 initial $display(\"R=%0d\", r);\n\
         endinterface\n\
         module top; ifc i(); initial #2 $finish; endmodule\n",
        &["R=10"],
        GONE,
    );
}

/// s34bm: the MODULE twin of the cell above — correct before and after, which is what
/// made the interface answer a lane-parity gap rather than a missing capability.
#[test]
fn the_module_twin_of_the_framed_decl_initializer() {
    lines(
        "module ifc;\n\
         \x20 function automatic int lp(input int n); int s; s = 0; for (int k = 0; k < n; k++) s = s + k; return s; endfunction\n\
         \x20 int r = lp(5);\n\
         \x20 initial $display(\"R=%0d\", r);\n\
         endmodule\n\
         module top; ifc i(); initial #2 $finish; endmodule\n",
        &["R=10"],
        GONE,
    );
}

/// s34: the NON-framed control for the two cells above — a straight-line declared
/// function in the same position, which the constant interpreter can fold and which
/// was already correct. It is pinned so that moving step (6.5) earlier cannot be read
/// as having moved this one too. Both oracles `R=44`.
#[test]
fn a_decl_initializer_calling_a_straight_line_declared_function() {
    lines(
        "interface ifc;\n\
         \x20 function automatic int f(input int a); return a + 4; endfunction\n\
         \x20 int r = f(40);\n\
         \x20 initial $display(\"R=%0d\", r);\n\
         endinterface\n\
         module top; ifc i(); initial #2 $finish; endmodule\n",
        &["R=44"],
        GONE,
    );
}

// ----------------------------------------------- cells the oracles do not decide

/// d17 / m17: a DUPLICATE declaration. BOTH oracles REJECT it (iverilog "'f' has
/// already been declared in this scope."; verilator "Duplicate declaration of
/// function: 'f'").
///
/// CONVERTED, same designs, flipped expectation. The pin used to be PARITY over a
/// shared DEFECT: both lanes warned `W3056 function 'f' redeclared; first declaration
/// used` and then used the SECOND declaration (`BTreeMap::insert` keeps the last), and
/// the two `RD=` lines were compared to each other rather than to a value because
/// neither was right. `decl_collide.rs` refuses the pair per definition now, so the
/// parity claim is the same and the shared answer is a refusal: same exit code, same
/// E3009, no `RD=` line in either lane, and the only difference is the unit word.
#[test]
fn a_duplicate_declaration_answers_like_the_module_twin() {
    let body = "\x20 function automatic int f(input int a); return a + 4; endfunction\n\
         \x20 function automatic int f(input int a); return a + 9; endfunction\n\
         \x20 initial begin r = f(40); $display(\"RD=%0d\", r); end\n";
    let (iout, icode) = run(&format!(
        "interface ifc; int r;\n{body}endinterface\n\
         module top; ifc i(); initial #2 $finish; endmodule\n"
    ));
    let (mout, mcode) = run(&format!(
        "module ifc; int r;\n{body}endmodule\n\
         module top; ifc i(); initial #2 $finish; endmodule\n"
    ));
    assert_eq!(icode, mcode, "interface:\n{iout}\nmodule:\n{mout}");
    assert_eq!(icode, Some(1), "both lanes refuse:\n{iout}");
    for (tag, unit, o) in [
        ("interface", "interface", &iout),
        ("module", "module", &mout),
    ] {
        // Same-kind pairs take the "both times as …" sentence.
        let want = format!("`f` is declared twice in this {unit}, both times as a function");
        assert!(
            o.contains("VITA-E3009") && o.contains(&want),
            "{tag} lane missing the §3.13 refusal:\n{o}"
        );
        assert!(
            !o.contains("redeclared; first declaration used"),
            "{tag} lane must not also warn — one defect, one report:\n{o}"
        );
        assert!(!o.contains("RD="), "{tag} lane must not run the call:\n{o}");
    }
}
