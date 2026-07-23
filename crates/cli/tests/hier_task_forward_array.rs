//! §4.5.210 (ROADMAP §0 follow-on #2): forwarding a frame task/function's OWN unpacked-array
//! FORMAL into a nested hierarchical TASK enable (`task driver(input int a[]); u.tk(a);`)
//! loud→supported. §4.5.207/209 handled a STATIC array actual (`byte a[4]`, packed / unpacked
//! element-by-element); a forwarded frame formal is an md-packed FRAME net (not a static array),
//! so it was loud — the defer gate only recorded static-array actuals, and lowering the whole
//! array formal as a value is itself loud.
//!
//! Mechanism: the defer gate (`inline_task`) now also records a bare Ident that resolves to a
//! frame array formal (`frame_arr_formal_meta`); at resolution `pack_hier_array_actual` forwards
//! the WHOLE md-packed net value (both slots share the identical `array_formal_ext_dims` layout,
//! so no repack is needed — the same UARR2 forwarding `lower_array_actual_packed` does locally),
//! gated by the shared `hier_array_shape_ok` (per-dim `dims` + `elem_w` must match).
//!
//! NO ORACLE (iverilog rejects unpacked subroutine ports) → hand-IEEE §13.5.1 pass-by-value.
//!
//! Correct-or-loud: INPUT forwarding only — an OUTPUT/INOUT forwarded frame formal (a copy-out
//! writeback into an md-packed frame net) stays loud; a shape mismatch stays loud; a non-hier
//! whole-array-formal use (`$display(a)`) stays loud.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_htfa_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

// ── supported (hand-IEEE; iverilog rejects the construct) ────────────────────

#[test]
fn forward_input_1d() {
    // The core case: a frame task forwards its own input array formal to a nested hier enable.
    let o = run("module sub; int mem[4];\n\
         task automatic tk(input int d[4]); for(int i=0;i<4;i++) mem[i]=d[i]; endtask\n\
         endmodule\n\
         module t; sub u();\n\
         task automatic driver(input int a[4]); u.tk(a); endtask\n\
         initial begin int arr[4]; arr[0]=1;arr[1]=2;arr[2]=3;arr[3]=4; driver(arr);\n\
           $display(\"%0d %0d\", u.mem[0], u.mem[3]); $finish; end endmodule\n");
    assert!(o.contains("1 4"), "forward input 1-D:\n{o}");
}

#[test]
fn forward_multidim() {
    let o = run("module sub; int acc;\n\
         task automatic tk(input int m[2][2]); acc=m[0][0]+m[0][1]+m[1][0]+m[1][1]; endtask\n\
         endmodule\n\
         module t; sub u();\n\
         task automatic driver(input int a[2][2]); u.tk(a); endtask\n\
         initial begin int arr[2][2]; arr[0][0]=1;arr[0][1]=2;arr[1][0]=3;arr[1][1]=4; driver(arr);\n\
           $display(\"%0d\", u.acc); $finish; end endmodule\n");
    assert!(o.contains("10"), "forward multi-dim:\n{o}");
}

#[test]
fn forward_signed_byte() {
    // Signed element: the forwarded whole value preserves the element bits; the callee re-stamps.
    let o = run("module sub; int acc;\n\
         task automatic tk(input byte d[3]); acc=d[0]+d[1]+d[2]; endtask\n\
         endmodule\n\
         module t; sub u();\n\
         task automatic driver(input byte a[3]); u.tk(a); endtask\n\
         initial begin byte arr[3]; arr[0]=-8'sd100;arr[1]=8'sd50;arr[2]=8'sd10; driver(arr);\n\
           $display(\"%0d\", u.acc); $finish; end endmodule\n");
    assert!(o.contains("-40"), "forward signed byte:\n{o}");
}

#[test]
fn forward_frame_local_array() {
    // A frame-LOCAL array (declared in the body, not a formal) is also md-packed — forwarding it
    // works the same way (bonus coverage; `frame_arr_formal_meta` covers frame locals too).
    let o = run("module sub; int acc;\n\
         task automatic tk(input int d[3]); acc=d[0]+d[1]+d[2]; endtask\n\
         endmodule\n\
         module t; sub u();\n\
         task automatic driver(); int loc[3]; loc[0]=5;loc[1]=6;loc[2]=7; u.tk(loc); endtask\n\
         initial begin driver(); $display(\"%0d\", u.acc); $finish; end endmodule\n");
    assert!(o.contains("18"), "forward frame-local array:\n{o}");
}

#[test]
fn forward_reads_current_value() {
    // The driver MUTATES its formal (pass-by-value local copy) BEFORE forwarding — the callee
    // must see the mutated value, and the caller's array must be untouched (§13.5.1).
    let o = run("module sub; int acc;\n\
         task automatic tk(input int d[2]); acc=d[0]+d[1]; endtask\n\
         endmodule\n\
         module t; sub u();\n\
         task automatic driver(input int a[2]); a[0]=100; u.tk(a); endtask\n\
         initial begin int arr[2]; arr[0]=1;arr[1]=2; driver(arr);\n\
           $display(\"acc=%0d arr0=%0d\", u.acc, arr[0]); $finish; end endmodule\n");
    assert!(
        o.contains("acc=102 arr0=1"),
        "forward reads mutated value, caller untouched:\n{o}"
    );
}

#[test]
fn forward_chained_through_two_levels() {
    // t.driver forwards to m.relay, which forwards to lf.tk — the forward composes across levels.
    let o = run("module leaf; int acc;\n\
         task automatic tk(input int d[3]); acc=d[0]+d[1]+d[2]; endtask endmodule\n\
         module mid; leaf lf();\n\
         task automatic relay(input int b[3]); lf.tk(b); endtask endmodule\n\
         module t; mid m();\n\
         task automatic driver(input int a[3]); m.relay(a); endtask\n\
         initial begin int arr[3]; arr[0]=7;arr[1]=8;arr[2]=9; driver(arr);\n\
           $display(\"%0d\", m.lf.acc); $finish; end endmodule\n");
    assert!(o.contains("24"), "chained forward:\n{o}");
}

#[test]
fn forward_same_formal_to_two_enables() {
    let o = run("module sub; int acc;\n\
         task automatic add(input int d[2]); acc=acc+d[0]+d[1]; endtask\n\
         endmodule\n\
         module t; sub u1(); sub u2();\n\
         task automatic driver(input int a[2]); u1.add(a); u2.add(a); endtask\n\
         initial begin int arr[2]; arr[0]=3;arr[1]=4; driver(arr);\n\
           $display(\"%0d %0d\", u1.acc, u2.acc); $finish; end endmodule\n");
    assert!(o.contains("7 7"), "forward same formal twice:\n{o}");
}

// ── correct-or-loud boundaries ───────────────────────────────────────────────

#[test]
fn forward_shape_mismatch_stays_loud() {
    // A forwarded formal `[3]` into a callee formal `[4]` is a shape mismatch. Loud.
    let o = run(
        "module sub; int acc; task automatic tk(input int d[4]); acc=d[0]; endtask endmodule\n\
         module t; sub u();\n\
         task automatic driver(input int a[3]); u.tk(a); endtask\n\
         initial begin int arr[3]; arr[0]=9; driver(arr); $finish; end endmodule\n",
    );
    assert!(
        o.contains("E3009"),
        "forward shape mismatch must be loud:\n{o}"
    );
}

#[test]
fn forward_output_array_stays_loud() {
    // Forwarding into an OUTPUT array formal needs a copy-out writeback into the md-packed frame
    // net — loud (input forwarding only).
    let o = run(
        "module sub; task automatic tk(output int d[3]); d[0]=1; endtask endmodule\n\
         module t; sub u();\n\
         task automatic driver(output int a[3]); u.tk(a); endtask\n\
         initial begin int arr[3]; driver(arr); $finish; end endmodule\n",
    );
    assert!(
        o.contains("E3009"),
        "forward to output array must be loud:\n{o}"
    );
}

#[test]
fn forward_inout_array_stays_loud() {
    let o = run(
        "module sub; task automatic tk(inout int d[3]); d[0]=d[0]+1; endtask endmodule\n\
         module t; sub u();\n\
         task automatic driver(inout int a[3]); u.tk(a); endtask\n\
         initial begin int arr[3]; arr[0]=1; driver(arr); $finish; end endmodule\n",
    );
    assert!(
        o.contains("E3009"),
        "forward to inout array must be loud:\n{o}"
    );
}

#[test]
fn whole_array_formal_in_display_stays_loud() {
    // Guard: my defer-gate relaxation must NOT relax a non-hier whole-array-formal use.
    let o = run("module t;\n\
         function automatic int show(input int a[3]); $display(\"%0d\", a); return 0; endfunction\n\
         initial begin int arr[3]; int r; r=show(arr); $finish; end endmodule\n");
    assert!(
        o.contains("E3009"),
        "whole array formal in $display must be loud:\n{o}"
    );
}
