//! §4.5.211 (ROADMAP §0 follow-on #3): a NON-ZERO-BASE DESCENDING unpacked-array formal
//! (`int m[4:1]`) loud→supported. §4.5.206 added the per-dim `(lo, size, ascending)` machinery and
//! supported zero-based-any-direction + non-zero-base ASCENDING, but gated non-zero-base
//! DESCENDING out of caution ("base+direction not cleanly verifiable"). §4.5.211 shows the
//! machinery §4.5.206 built already handles it: `array_formal_ext_dims` sets the packed dim's `lo`,
//! so `flatten_word` normalizes `idx-lo` CONSISTENTLY on both the actual pack
//! (`lower_array_actual_packed`, which reads flat words in position order) and the formal read
//! (`m[k]` → `k-lo`). Hence `m[k] == a[k]` for a same-range formal (§13.5.1 positional copy),
//! forward, regardless of direction.
//!
//! VERIFICATION (the §4.5.206 "not verifiable" claim was wrong): iverilog rejects unpacked
//! subroutine ports AND whole-unpacked-array copy, so there is no direct oracle for the formal —
//! BUT IEEE §13.5.1 mandates `m[k] == a[k]` for a same-declared-range formal, which is fully
//! observable. Distinct-digit differentials (`4321`, non-square `654321`) make ANY mis-map
//! (reversal, off-by-lo, dim-swap) visible. The element-index semantics themselves are
//! iverilog-checked (a module `int m[4:1]; m[k]=..` reads back identically in both).
//!
//! Correct-or-loud: a per-dim direction / base / size / dim-count MISMATCH between actual and
//! formal stays loud (the real §7.6 positional-copy guard in `lower_array_actual_packed`).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_nzd_{}_{n}", std::process::id()));
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

// ── supported (hand-IEEE §13.5.1; distinct digits prove the forward mapping) ──

#[test]
fn input_1d_descending() {
    // `m[k]` must equal `a[k]` (4321, not the reversed 1234).
    let o = run("module t;\n\
         function automatic int t4(input int m[4:1]); return m[4]*1000+m[3]*100+m[2]*10+m[1]; endfunction\n\
         initial begin int a[4:1]; int r; a[4]=4;a[3]=3;a[2]=2;a[1]=1; r=t4(a);\n\
           $display(\"r=%0d\", r); $finish; end endmodule\n");
    assert!(o.contains("r=4321"), "input 1-D descending:\n{o}");
}

#[test]
fn input_offset_base() {
    // A base above 1 (`[5:2]`, lo=2).
    let o = run("module t;\n\
         function automatic int t4(input int m[5:2]); return m[5]*1000+m[4]*100+m[3]*10+m[2]; endfunction\n\
         initial begin int a[5:2]; int r; a[5]=4;a[4]=3;a[3]=2;a[2]=1; r=t4(a);\n\
           $display(\"r=%0d\", r); $finish; end endmodule\n");
    assert!(o.contains("r=4321"), "offset-base descending:\n{o}");
}

#[test]
fn input_nonsquare_2x3_descending() {
    // Non-square is the reversal clincher — any dim-swap or reversal shows as a wrong 6-digit value.
    let o = run("module t;\n\
         function automatic int t6(input int m[2:1][3:1]);\n\
           return m[1][1]+m[1][2]*10+m[1][3]*100+m[2][1]*1000+m[2][2]*10000+m[2][3]*100000;\n\
         endfunction\n\
         initial begin int a[2:1][3:1]; int r;\n\
           a[1][1]=1;a[1][2]=2;a[1][3]=3;a[2][1]=4;a[2][2]=5;a[2][3]=6; r=t6(a);\n\
           $display(\"r=%0d\", r); $finish; end endmodule\n");
    assert!(o.contains("r=654321"), "non-square 2x3 descending:\n{o}");
}

#[test]
fn input_mixed_direction_nonzero() {
    // Outer ascending non-zero `[1:2]`, inner descending non-zero `[3:1]`.
    let o = run("module t;\n\
         function automatic int t6(input int m[1:2][3:1]);\n\
           return m[1][1]+m[1][2]*10+m[1][3]*100+m[2][1]*1000+m[2][2]*10000+m[2][3]*100000;\n\
         endfunction\n\
         initial begin int a[1:2][3:1]; int r;\n\
           a[1][1]=1;a[1][2]=2;a[1][3]=3;a[2][1]=4;a[2][2]=5;a[2][3]=6; r=t6(a);\n\
           $display(\"r=%0d\", r); $finish; end endmodule\n");
    assert!(
        o.contains("r=654321"),
        "mixed-direction non-zero base:\n{o}"
    );
}

#[test]
fn signed_byte_descending() {
    let o = run("module t;\n\
         function automatic int s3(input byte m[3:1]); return m[3]+m[2]+m[1]; endfunction\n\
         initial begin byte a[3:1]; int r; a[3]=-8'sd100;a[2]=8'sd50;a[1]=8'sd10; r=s3(a);\n\
           $display(\"r=%0d\", r); $finish; end endmodule\n");
    assert!(o.contains("r=-40"), "signed byte descending:\n{o}");
}

#[test]
fn task_copy_in_descending() {
    // Body copies `m[k]` into a module array indexed 1..4 — cross-checks the declared index.
    let o = run("module t; int mem[4:1];\n\
         task automatic ld(input int m[4:1]); for(int k=1;k<=4;k++) mem[k]=m[k]; endtask\n\
         initial begin int a[4:1]; a[4]=40;a[3]=30;a[2]=20;a[1]=10; ld(a);\n\
           $display(\"%0d %0d %0d %0d\", mem[4],mem[3],mem[2],mem[1]); $finish; end endmodule\n");
    assert!(o.contains("40 30 20 10"), "task copy-in descending:\n{o}");
}

#[test]
fn output_descending() {
    // The copy-out `caller[lo+pos]` writes the caller's declared index.
    let o = run("module t;\n\
         task automatic gen(output int m[4:1]); m[4]=44;m[3]=33;m[2]=22;m[1]=11; endtask\n\
         initial begin int a[4:1]; gen(a);\n\
           $display(\"%0d %0d %0d %0d\", a[4],a[3],a[2],a[1]); $finish; end endmodule\n");
    assert!(o.contains("44 33 22 11"), "output descending:\n{o}");
}

#[test]
fn inout_descending_rmw() {
    let o = run("module t;\n\
         task automatic bump(inout int m[4:1]); m[4]=m[4]+1;m[3]=m[3]+1;m[2]=m[2]+1;m[1]=m[1]+1; endtask\n\
         initial begin int a[4:1]; a[4]=40;a[3]=30;a[2]=20;a[1]=10; bump(a);\n\
           $display(\"%0d %0d %0d %0d\", a[4],a[3],a[2],a[1]); $finish; end endmodule\n");
    assert!(o.contains("41 31 21 11"), "inout descending RMW:\n{o}");
}

#[test]
fn hier_input_descending() {
    // The §4.5.207 hier copy-in path now accepts the descending shape.
    let o = run("module sub; int acc;\n\
         task automatic tk(input int d[4:1]); acc=d[4]*1000+d[3]*100+d[2]*10+d[1]; endtask\n\
         endmodule\n\
         module t; sub u();\n\
         initial begin int a[4:1]; a[4]=4;a[3]=3;a[2]=2;a[1]=1; u.tk(a);\n\
           $display(\"acc=%0d\", u.acc); $finish; end endmodule\n");
    assert!(o.contains("acc=4321"), "hier input descending:\n{o}");
}

#[test]
fn hier_output_descending() {
    // The §4.5.209 hier copy-out path now accepts the descending shape.
    let o = run("module sub;\n\
         task automatic gen(output int d[4:1]); d[4]=44;d[3]=33;d[2]=22;d[1]=11; endtask\n\
         endmodule\n\
         module t; sub u();\n\
         initial begin int a[4:1]; u.gen(a);\n\
           $display(\"%0d %0d %0d %0d\", a[4],a[3],a[2],a[1]); $finish; end endmodule\n");
    assert!(o.contains("44 33 22 11"), "hier output descending:\n{o}");
}

#[test]
fn frame_local_descending() {
    let o = run("module t;\n\
         task automatic go(); int m[4:1]; m[4]=40;m[3]=30;m[2]=20;m[1]=10;\n\
           $display(\"%0d %0d %0d %0d\", m[4],m[3],m[2],m[1]); endtask\n\
         initial begin go(); $finish; end endmodule\n");
    assert!(o.contains("40 30 20 10"), "frame-local descending:\n{o}");
}

#[test]
fn forward_descending_frame_formal() {
    // The §4.5.210 forward path now accepts the descending shape.
    let o = run("module sub; int acc;\n\
         task automatic tk(input int d[4:1]); acc=d[4]*1000+d[3]*100+d[2]*10+d[1]; endtask\n\
         endmodule\n\
         module t; sub u();\n\
         task automatic driver(input int a[4:1]); u.tk(a); endtask\n\
         initial begin int arr[4:1]; arr[4]=4;arr[3]=3;arr[2]=2;arr[1]=1; driver(arr);\n\
           $display(\"acc=%0d\", u.acc); $finish; end endmodule\n");
    assert!(
        o.contains("acc=4321"),
        "forward descending frame formal:\n{o}"
    );
}

// ── correct-or-loud boundaries ───────────────────────────────────────────────

#[test]
fn direction_mismatch_stays_loud() {
    // An ASCENDING actual `[1:4]` into a DESCENDING formal `[4:1]` would silently reverse under
    // §7.6 positional copy — loud (the real guard).
    let o = run("module t;\n\
         function automatic int t4(input int m[4:1]); return m[4]; endfunction\n\
         initial begin int a[1:4]; int r; a[4]=9; r=t4(a); $display(\"%0d\",r); $finish; end endmodule\n");
    assert!(o.contains("E3009"), "direction mismatch must be loud:\n{o}");
}

#[test]
fn base_mismatch_stays_loud() {
    // A zero-based actual `[3:0]` into a `[4:1]` formal — each side normalizes by its own lo, so
    // positions would mis-map. Loud.
    let o = run("module t;\n\
         function automatic int t4(input int m[4:1]); return m[4]; endfunction\n\
         initial begin int a[3:0]; int r; a[3]=9; r=t4(a); $display(\"%0d\",r); $finish; end endmodule\n");
    assert!(o.contains("E3009"), "base mismatch must be loud:\n{o}");
}
