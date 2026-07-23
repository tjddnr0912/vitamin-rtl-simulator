//! §4.5.207: a hierarchical TASK enable whose callee has an INPUT unpacked-array formal
//! (`u.load(data)` where `task load(input int d[4])`) loud→supported. §4.5.197/201 handled
//! scalar hier-task formals; an array formal was loud because a whole-array actual cannot be
//! lowered to a value at the call site and the callee's array shape is unknown until the
//! child instance is elaborated.
//!
//! Mechanism: the hier_tasks gate now admits an INPUT fixed-array formal; the defer records
//! the actual's static-array NET (resolved in the caller scope) in `arg_arrays`; the resolver
//! looks up the callee's per-instance array shape (`frame_arr_formal_meta[base_net+slot]`) and
//! packs the net into the md-packed slot (`pack_hier_array_actual`, the resolve-time twin of
//! `lower_array_actual_packed`). Reuses the §4.5.202–206 array machinery.
//!
//! NO ORACLE: iverilog rejects unpacked subroutine ports outright, so every supported case is
//! hand-IEEE (§13.5.1 pass-by-value copy-in).
//!
//! Correct-or-loud: an OUTPUT/INOUT array formal (deferred copy-out is a follow-on), a shape
//! mismatch, an array formal fed a scalar actual (or a scalar formal fed an array), and a
//! string/dynamic array formal all stay loud.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_hta_{}_{n}", std::process::id()));
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
fn hier_task_1d_input_array() {
    // The DUT-drive idiom: a hier task loads an array into the child instance's memory.
    let o = run("module sub; int mem[4];\n\
         task automatic load(input int d[4]); for(int i=0;i<4;i++) mem[i]=d[i]; endtask\n\
         endmodule\n\
         module t; sub u();\n\
         initial begin int a[4]; a[0]=1;a[1]=2;a[2]=3;a[3]=4; u.load(a);\n\
           $display(\"%0d %0d\", u.mem[0], u.mem[3]); $finish; end endmodule\n");
    assert!(o.contains("1 4"), "hier task 1-D input array:\n{o}");
}

#[test]
fn hier_task_mixed_scalar_and_array() {
    // A scalar `addr` + an array `d` — the classic register-file writer over a hier enable.
    let o = run("module sub; logic [7:0] mem[8];\n\
         task automatic wr(input int addr, input logic [7:0] d[4]);\n\
           for(int i=0;i<4;i++) mem[addr+i]=d[i];\n\
         endtask\n\
         endmodule\n\
         module t; sub u();\n\
         initial begin logic [7:0] v[4]; v[0]=8'hA0;v[1]=8'hA1;v[2]=8'hA2;v[3]=8'hA3;\n\
           u.wr(2,v); $display(\"%h %h %h %h\", u.mem[2],u.mem[3],u.mem[4],u.mem[5]); $finish; end endmodule\n");
    assert!(o.contains("a0 a1 a2 a3"), "mixed scalar + array:\n{o}");
}

#[test]
fn hier_task_2d_array() {
    // A multi-dim array formal over a hier enable (reuses §4.5.202).
    let o = run("module sub; int acc;\n\
         task automatic p(input int m[2][2]); acc=m[0][0]+m[1][1]; endtask\n\
         endmodule\n\
         module t; sub u();\n\
         initial begin int a[2][2]; a[0][0]=40;a[1][1]=2; u.p(a); $display(\"%0d\",u.acc); $finish; end endmodule\n");
    assert!(o.contains("42"), "hier task 2-D array:\n{o}");
}

#[test]
fn hier_task_array_per_instance_isolation() {
    // Two instances with different param overrides — each `u.add` touches its own acc/K.
    let o = run("module sub #(parameter int K=0); int acc;\n\
         task automatic add(input int d[2]); acc=d[0]+d[1]+K; endtask\n\
         endmodule\n\
         module t; sub #(.K(100)) u1(); sub #(.K(200)) u2();\n\
         initial begin int a[2]; a[0]=1;a[1]=2; u1.add(a); u2.add(a);\n\
           $display(\"%0d %0d\", u1.acc, u2.acc); $finish; end endmodule\n");
    assert!(o.contains("103 203"), "per-instance isolation:\n{o}");
}

#[test]
fn hier_task_signed_byte_array_deep_path() {
    // A signed `byte` element (re-stamp) through a 3-segment instance path `m.lf.p`.
    let o = run("module leaf; int acc;\n\
         task automatic p(input byte d[3]); acc=d[0]+d[1]+d[2]; endtask endmodule\n\
         module mid; leaf lf(); endmodule\n\
         module t; mid m();\n\
         initial begin byte a[3]; a[0]=-8'sd100;a[1]=8'sd50;a[2]=8'sd10; m.lf.p(a);\n\
           $display(\"%0d\", m.lf.acc); $finish; end endmodule\n");
    assert!(o.contains("-40"), "signed byte array deep path:\n{o}");
}

// ── correct-or-loud boundaries ───────────────────────────────────────────────

#[test]
fn hier_task_output_array_stays_loud() {
    // An OUTPUT array formal over a hier enable — the deferred copy-out is a follow-on. Loud.
    let o = run("module sub; task automatic fill(output int d[4]); d[0]=1; endtask endmodule\n\
         module t; sub u(); initial begin int a[4]; u.fill(a); $display(\"%0d\",a[0]); $finish; end endmodule\n");
    assert!(
        o.contains("E3009"),
        "output array over hier must be loud:\n{o}"
    );
}

#[test]
fn hier_task_array_shape_mismatch_stays_loud() {
    // A shape mismatch (actual `[3]`, formal `[4]`) is loud.
    let o = run("module sub; int acc; task automatic p(input int d[4]); acc=d[0]; endtask endmodule\n\
         module t; sub u(); initial begin int a[3]; a[0]=9; u.p(a); $display(\"%0d\",u.acc); $finish; end endmodule\n");
    assert!(
        o.contains("E3009"),
        "array shape mismatch must be loud:\n{o}"
    );
}

#[test]
fn hier_task_array_formal_scalar_actual_stays_loud() {
    // An array formal fed a scalar actual is a type mismatch. Loud.
    let o = run("module sub; int acc; task automatic p(input int d[4]); acc=d[0]; endtask endmodule\n\
         module t; sub u(); initial begin int x; x=5; u.p(x); $display(\"%0d\",u.acc); $finish; end endmodule\n");
    assert!(
        o.contains("E3009"),
        "array formal + scalar actual must be loud:\n{o}"
    );
}

#[test]
fn hier_task_scalar_formal_array_actual_stays_loud() {
    // A scalar formal fed a whole-array actual is a type mismatch. Loud.
    let o = run("module sub; int acc; task automatic p(input int x); acc=x; endtask endmodule\n\
         module t; sub u(); initial begin int a[4]; a[0]=5; u.p(a); $display(\"%0d\",u.acc); $finish; end endmodule\n");
    assert!(
        o.contains("E3009"),
        "scalar formal + array actual must be loud:\n{o}"
    );
}
