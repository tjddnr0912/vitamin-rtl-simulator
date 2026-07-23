//! Round-17 D-family (the reviewer's §5 "minor 잔여") — oracle-backed loud→supported.
//! Each has an iverilog oracle unless noted (some D-items iverilog itself rejects —
//! those are hand-IEEE and tracked in ROADMAP, not here).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_d17_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

// ── D: import inside a package ───────────────────────────────────────────────
// A package may `import` another package; its TYPES (parser unit-global typedef
// map) and CONSTANTS (apply_import_consts into the fold scope) become usable.

#[test]
fn import_type_inside_package() {
    let o = run("package base; typedef logic [7:0] byte8_t; endpackage\n\
         package derived; import base::*; function automatic byte8_t dbl(byte8_t x); return x<<1; endfunction endpackage\n\
         module t; import derived::*; initial begin $display(\"r=%0d\", dbl(8'd5)); $finish; end endmodule\n");
    assert!(o.contains("r=10"), "import type inside package:\n{o}");
}

#[test]
fn import_const_inside_package() {
    let o = run("package base; localparam int W = 8; endpackage\n\
         package derived; import base::*; localparam int DW = W * 2; endpackage\n\
         module t; import derived::*; initial begin $display(\"DW=%0d\", DW); $finish; end endmodule\n");
    assert!(
        o.contains("DW=16"),
        "import const inside package (fold W*2):\n{o}"
    );
}

// ── D: $display / $write inside a subset FUNCTION body ───────────────────────
// iverilog runs a $display inside a function; the &self frame executors now render
// it (a severity $error stays loud — correct-or-loud).

#[test]
fn display_inside_function_body() {
    let o = run("module t;\n\
         function automatic int f(input int x); $display(\"dbg x=%0d\", x); return x*2; endfunction\n\
         initial begin $display(\"r=%0d\", f(5)); $finish; end endmodule\n");
    assert!(
        o.contains("dbg x=5") && o.contains("r=10"),
        "$display in function:\n{o}"
    );
}

#[test]
fn write_and_radix_inside_function() {
    let o = run("module t;\n\
         function automatic int f(input int x); $write(\"[x=\"); $writeh(x); $display(\"]\"); return x; endfunction\n\
         initial begin $display(\"r=%0d\", f(255)); $finish; end endmodule\n");
    assert!(
        o.contains("[x=000000ff]") && o.contains("r=255"),
        "$write/$writeh in function:\n{o}"
    );
}

#[test]
fn display_inside_subset_task_body() {
    let o = run("module t;\n\
         task automatic tk(input int x); $display(\"t x=%0d\", x); endtask\n\
         initial begin tk(9); $finish; end endmodule\n");
    assert!(o.contains("t x=9"), "$display in subset task:\n{o}");
}

// ── D: task INOUT unpacked-fixed array formal ───────────────────────────────
// §4.5.193 opened OUTPUT arrays; r17 adds INOUT (copy-in at entry + copy-out at
// exit, IEEE §13.5.2). iverilog rejects unpacked subroutine array ports, so this is
// hand-IEEE / self-consistent (the round-trip proves the copy-in reads old values).

#[test]
fn task_inout_array_round_trip() {
    let o = run("module t;\n\
         task automatic addv(inout int a[3], input int v); for(int i=0;i<3;i++) a[i]=a[i]+v; endtask\n\
         initial begin int m[3]; m[0]=10;m[1]=20;m[2]=30; addv(m, 5); $display(\"%0d %0d %0d\", m[0],m[1],m[2]); $finish; end endmodule\n");
    // copy-in reads 10/20/30 (not 0), body adds 5, copy-out writes back → 15/25/35.
    assert!(o.contains("15 25 35"), "inout array round-trip:\n{o}");
}

#[test]
fn task_output_array_regression() {
    // §4.5.193 output array must still work after the inout extension.
    let o = run("module t;\n\
         task automatic fill(output byte a[4]); for(int i=0;i<4;i++) a[i]=i*10; endtask\n\
         initial begin byte m[4]; fill(m); $display(\"%0d %0d %0d %0d\", m[0],m[1],m[2],m[3]); $finish; end endmodule\n");
    assert!(o.contains("0 10 20 30"), "output array regression:\n{o}");
}

// ── D: hierarchical function call `u1.f(x)` ─────────────────────────────────
// Call a function defined in a child instance, resolved to that instance's
// per-instance FuncId (so its own nets/params are baked in correctly).

#[test]
fn hierarchical_function_call_basic() {
    let o = run(
        "module sub; function automatic int dbl(input int x); return x*2; endfunction endmodule\n\
         module t; sub u1();\n\
         initial begin $display(\"r=%0d\", u1.dbl(5)); $finish; end endmodule\n",
    );
    assert!(o.contains("r=10"), "hierarchical function call:\n{o}");
}

#[test]
fn hierarchical_call_per_instance_params() {
    // Two instances with different param overrides — each `u.addk` reads its OWN K.
    let o = run("module sub #(parameter int K=0); function automatic int addk(input int x); return x+K; endfunction endmodule\n\
         module t; sub #(.K(1000)) u1(); sub #(.K(2000)) u2();\n\
         initial begin $display(\"a=%0d b=%0d\", u1.addk(5), u2.addk(5)); $finish; end endmodule\n");
    assert!(
        o.contains("a=1005 b=2005"),
        "per-instance param in hier call:\n{o}"
    );
}

#[test]
fn hierarchical_call_reads_callee_net() {
    let o = run("module sub; int base=100; function automatic int addbase(input int x); return x+base; endfunction endmodule\n\
         module t; sub u1();\n\
         initial begin $display(\"r=%0d\", u1.addbase(5)); $finish; end endmodule\n");
    assert!(o.contains("r=105"), "hier call reading callee net:\n{o}");
}

#[test]
fn hierarchical_call_output_formal_stays_loud() {
    // correct-or-loud: an output/inout formal (pass-by-ref across a module boundary) is
    // not in the hier-callable subset → loud.
    let o = run("module sub; function automatic int f(input int x, output int y); y=x; return x; endfunction endmodule\n\
         module t; sub u1(); initial begin int a,b; a=u1.f(5,b); $display(\"%0d\",a); $finish; end endmodule\n");
    assert!(
        o.contains("E3009") || !o.contains('5'),
        "hier call with output formal must be loud:\n{o}"
    );
}

#[test]
fn hierarchical_call_unknown_function_stays_loud() {
    let o = run("module sub; function automatic int f(input int x); return x; endfunction endmodule\n\
         module t; sub u1(); initial begin int a; a=u1.nope(5); $display(\"ran=%0d\",a); $finish; end endmodule\n");
    assert!(
        !o.contains("ran="),
        "hier call to unknown function must be loud:\n{o}"
    );
}

#[test]
fn hierarchical_call_wrong_arity_stays_loud() {
    let o = run("module sub; function automatic int add(input int a, input int b); return a+b; endfunction endmodule\n\
         module t; sub u1(); initial begin int r; r=u1.add(5); $display(\"ran=%0d\",r); $finish; end endmodule\n");
    assert!(
        !o.contains("ran="),
        "hier call with wrong arity must be loud:\n{o}"
    );
}

#[test]
fn severity_inside_function_now_supported() {
    // r18 (F2): $error/$warning/$info/$fatal in a frame function/subset-task body is now
    // correct-support — the &self frame executor renders it to the diag stream and applies
    // its effect ($error → had_error via a Cell, $fatal → call_fatal). Was a §4.5.196
    // follow-on loud. Here `f(5)` fires the `$error`, so "bad 5" is raised as a diagnostic
    // (not silently dropped, not loud-rejected at elaborate).
    let o = run("module t; function automatic int f(input int x); $error(\"bad %0d\", x); return x; endfunction\n\
         initial begin int r=f(5); $finish; end endmodule\n");
    assert!(
        !o.contains("E3009") && o.contains("bad 5"),
        "$error in function must render its diagnostic:\n{o}"
    );
}
