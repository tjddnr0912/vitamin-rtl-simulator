//! §4.5.203: a STATIC (non-`automatic`) task with a FIXED unpacked-array formal
//! (`task load(input int m[4]); ...`) loud→supported. The array formal is an md-packed
//! value slot that only exists on the FRAME path (§4.5.188 input / §4.5.193 output-inout /
//! §4.5.202 multi-dim); the inline (static-task) binding path had no slot, so vita rejected
//! it ("task `X` has an unpacked-array formal — unsupported"). §4.5.198/199/200 established
//! that frame ⊇ inline, so FORCE-FRAMING such a task (via `build_task_frame_set`, exactly
//! like the §4.5.200 hier force-frame) loses no capability its LOCAL callers rely on, and
//! routes the array formal through the proven md-packed machinery.
//!
//! NO ORACLE: iverilog 13.0 rejects unpacked subroutine ports outright, so every supported
//! case is hand-IEEE (§13.5.1/2 pass-by-value / value-result; static-local storage persists
//! across calls, unchanged by the framing).
//!
//! Supported shapes: any base + any direction (§4.5.204 OUTPUT/INOUT multi-dim; §4.5.205
//! descending / mixed; §4.5.206 non-zero-base ascending; §4.5.211 non-zero-base descending).
//! A per-dim direction / base / size MISMATCH between actual and formal stays loud.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_staf_{}_{n}", std::process::id()));
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
fn static_task_1d_input() {
    // The register-file / memory-load idiom: a plain (static) task taking an array. acc=10.
    let o = run("module tb; int acc;\n\
         task load(input int m[4]); acc = m[0]+m[1]+m[2]+m[3]; endtask\n\
         initial begin int a[4]; a[0]=1;a[1]=2;a[2]=3;a[3]=4;\n\
           load(a); $display(\"acc=%0d\", acc); $finish; end endmodule\n");
    assert!(o.contains("acc=10"), "static 1-D input:\n{o}");
}

#[test]
fn static_task_2d_input() {
    // A static task with a multi-dim (§4.5.202) formal — force-framed + N-D md-packed. acc=42.
    let o = run("module tb; int acc;\n\
         task p(input int m[2][2]); acc = m[0][0]+m[1][1]; endtask\n\
         initial begin int a[2][2]; a[0][0]=40;a[1][1]=2;\n\
           p(a); $display(\"acc=%0d\", acc); $finish; end endmodule\n");
    assert!(o.contains("acc=42"), "static 2-D input:\n{o}");
}

#[test]
fn static_local_accumulation_across_calls() {
    // Static-local semantics survive the framing: a module-net accumulator hit by two calls.
    let o = run("module tb; int cnt;\n\
         task tick(input int m[2]); cnt = cnt + m[0] + m[1]; endtask\n\
         initial begin int a[2]; a[0]=1;a[1]=2;\n\
           tick(a); tick(a); $display(\"cnt=%0d\", cnt); $finish; end endmodule\n");
    assert!(
        o.contains("cnt=6"),
        "static accumulation across calls:\n{o}"
    );
}

#[test]
fn static_task_output_1d() {
    // A static OUTPUT array formal (§4.5.193 copy-out, now reachable for a static task).
    let o = run("module tb;\n\
         task fill(output int m[3]); m[0]=10; m[1]=20; m[2]=30; endtask\n\
         initial begin int a[3]; fill(a);\n\
           $display(\"%0d %0d %0d\", a[0], a[1], a[2]); $finish; end endmodule\n");
    assert!(o.contains("10 20 30"), "static output 1-D:\n{o}");
}

#[test]
fn static_task_inout_1d() {
    // INOUT = copy-in + copy-out. 5+1=6, 7+10=17.
    let o = run("module tb;\n\
         task bump(inout int m[2]); m[0]=m[0]+1; m[1]=m[1]+10; endtask\n\
         initial begin int a[2]; a[0]=5;a[1]=7; bump(a);\n\
           $display(\"%0d %0d\", a[0], a[1]); $finish; end endmodule\n");
    assert!(o.contains("6 17"), "static inout 1-D:\n{o}");
}

#[test]
fn static_register_file_writer() {
    // A static task combining an array formal, a module-array element write, and a loop
    // (the classic register-file / memory writer). mem[2..5] = a0..a3.
    let o = run("module tb; logic [7:0] mem[8];\n\
         task wr(input int addr, input logic [7:0] vals[4]);\n\
           for(int i=0;i<4;i++) mem[addr+i]=vals[i];\n\
         endtask\n\
         initial begin logic [7:0] v[4]; v[0]=8'hA0;v[1]=8'hA1;v[2]=8'hA2;v[3]=8'hA3;\n\
           wr(2,v); $display(\"%h %h %h %h\", mem[2],mem[3],mem[4],mem[5]); $finish; end endmodule\n");
    assert!(
        o.contains("a0 a1 a2 a3"),
        "static register-file writer:\n{o}"
    );
}

#[test]
fn static_task_array_formal_suspends() {
    // A static task with an array formal AND a `#5` — the framing routes it to the
    // suspendable path (§4.5.168), and the value slot survives the suspend.
    let o = run("module tb; int acc;\n\
         task waitsum(input int m[3]); #5 acc=m[0]+m[1]+m[2]; endtask\n\
         initial begin int a[3]; a[0]=1;a[1]=2;a[2]=4; waitsum(a);\n\
           $display(\"at %0t acc=%0d\", $time, acc); $finish; end endmodule\n");
    assert!(
        o.contains("at 5 acc=7"),
        "static array formal + timing:\n{o}"
    );
}

#[test]
fn inline_caller_to_framed_array_formal_callee() {
    // A still-inline static task calls a (force-framed) static array-formal task — the
    // inline→frame call boundary works (frame ⊇ inline). g=34.
    let o = run("module tb; int g;\n\
         task inner(input int m[2]); g=m[0]*10+m[1]; endtask\n\
         task outer(); int a[2]; a[0]=3;a[1]=4; inner(a); endtask\n\
         initial begin outer(); $display(\"g=%0d\", g); $finish; end endmodule\n");
    assert!(o.contains("g=34"), "inline caller → framed callee:\n{o}");
}

#[test]
fn static_input_array_is_pass_by_value() {
    // The body may write its own input-array copy (`m[0]=999`) — local read sees it
    // (g=1004) but the caller's array is unchanged (a0 stays 1).
    let o = run("module tb;\n\
         task p(input int m[2], output int got); m[0]=999; got=m[0]+m[1]; endtask\n\
         initial begin\n\
           int a[2]; int g;\n\
           a[0]=1;a[1]=5; p(a,g);\n\
           $display(\"g=%0d a0=%0d\", g, a[0]); $finish; end endmodule\n");
    assert!(
        o.contains("g=1004 a0=1"),
        "static input array pass-by-value:\n{o}"
    );
}

#[test]
fn static_signed_byte_array() {
    // A signed `byte` element re-stamps $signed on the md-packed element read. -100+55=-45.
    let o = run("module tb; int acc;\n\
         task p(input byte m[2]); acc = m[0] + m[1]; endtask\n\
         initial begin byte a[2]; a[0]=-8'sd100; a[1]=8'sd55; p(a);\n\
           $display(\"acc=%0d\", acc); $finish; end endmodule\n");
    assert!(o.contains("acc=-45"), "static signed byte array:\n{o}");
}

#[test]
fn static_array_formal_refreshed_each_call() {
    // Each call re-packs the CURRENT actual — not a stale snapshot. a→3, then mutate a→30.
    let o = run("module tb; int r1, r2;\n\
         task snap(input int m[2], output int s); s = m[0]+m[1]; endtask\n\
         initial begin\n\
           int a[2];\n\
           a[0]=1;a[1]=2; snap(a, r1);\n\
           a[0]=10;a[1]=20; snap(a, r2);\n\
           $display(\"r1=%0d r2=%0d\", r1, r2); $finish; end endmodule\n");
    assert!(o.contains("r1=3 r2=30"), "actual refreshed each call:\n{o}");
}

#[test]
fn static_output_multidim_supported() {
    // §4.5.204: a STATIC task with an OUTPUT multi-dim array formal — force-framed AND the
    // multi-dim copy-out unpack. 1 2 3 4.
    let o = run(
        "module tb;\n\
         task p(output int m[2][2]); m[0][0]=1;m[0][1]=2;m[1][0]=3;m[1][1]=4; endtask\n\
         initial begin int a[2][2]; p(a);\n\
           $display(\"%0d %0d %0d %0d\",a[0][0],a[0][1],a[1][0],a[1][1]); $finish; end endmodule\n",
    );
    assert!(o.contains("1 2 3 4"), "static output multi-dim:\n{o}");
}

// ── correct-or-loud boundaries ───────────────────────────────────────────────

#[test]
fn static_descending_array_supported() {
    // §4.5.205: a descending multi-dim array formal on a static (force-framed) task copies
    // forward, matching-direction. m[i][j]=a[i][j].
    let o = run("module tb; int acc;\n\
         task p(input int m[1:0][1:0]); acc=m[0][0]*1000+m[0][1]*100+m[1][0]*10+m[1][1]; endtask\n\
         initial begin int a[1:0][1:0]; a[0][0]=1;a[0][1]=2;a[1][0]=3;a[1][1]=4; p(a); $display(\"%0d\",acc); $finish; end endmodule\n");
    assert!(o.contains("1234"), "static descending array forward:\n{o}");
}

#[test]
fn static_non_zero_based_ascending_supported() {
    // §4.5.206: a non-zero-based ASCENDING array formal (`m[1:4]`) on a static (force-framed)
    // task is supported — the base `lo=1` normalizes the index. m[1]=a[1].
    let o = run("module tb; int acc;\n\
         task p(input int m[1:4]); acc=m[1]*1000+m[2]*100+m[3]*10+m[4]; endtask\n\
         initial begin int a[1:4]; a[1]=1;a[2]=2;a[3]=3;a[4]=4; p(a); $display(\"%0d\",acc); $finish; end endmodule\n");
    assert!(o.contains("1234"), "static non-zero-based ascending:\n{o}");
}

#[test]
fn static_non_zero_based_descending_now_supported() {
    // §4.5.211: a non-zero-base DESCENDING formal (`m[4:1]`) is supported (force-framed static
    // task + `idx-lo` normalization). Distinct digits (4321) prove the forward mapping.
    let o = run("module tb; int acc;\n\
         task p(input int m[4:1]); acc=m[4]*1000+m[3]*100+m[2]*10+m[1]; endtask\n\
         initial begin int a[4:1]; a[4]=4;a[3]=3;a[2]=2;a[1]=1; p(a); $display(\"%0d\",acc); $finish; end endmodule\n");
    assert!(
        o.contains("4321"),
        "static non-zero-base descending now supported:\n{o}"
    );
}
