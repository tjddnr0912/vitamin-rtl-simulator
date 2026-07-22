//! §4.5.199 (frame↔inline parity, step 2): a MULTI-DIM frame-LOCAL unpacked array
//! (`int m[2][2];` inside a task/function body) loud→supported. It was the last
//! `frame ⊂ inline` capability gap (a differential sweep of `task automatic` vs `task`
//! found module writes / control flow / 1-D locals / timing all at parity, and ONLY the
//! multi-dim frame-local array loud). Module-scope `m[i][j]` already worked, so it was a
//! frame-lowering-specific gap: the array was reserved as a 1-elem `frame_array_local`
//! net, so `m[0][0]` hit "nested lvalue select (v1: single-level)".
//!
//! Fix: `classify_unpacked_array` accepts every zero-based-const dim (not just one) and
//! `reserve_frame_local_decl` reserves an md-packed slot with ONE `packed_dims` entry per
//! dim + the elem_w entry. `m[i][j]` then routes through the SAME N-D packed chain
//! (`expr_packed_chain`/`lval_packed_chain` → `flatten_word`) that module arrays use:
//! offset `i*∏inner*elem_w + j*elem_w`, width `elem_w`. No new offset math, format 22
//! unchanged. A FORMAL stays 1-D (`classify_array_formal` rejects multi-dim — the formal
//! binding packs a single dimension).
//!
//! Correct-or-loud: a PARTIAL index (`m[i]` on a 2-D array — fewer indices than dims) has
//! no whole-sub-array value and is loud (not a silent multi-element slice); a whole 2-D
//! array as a subroutine arg is loud. iverilog is the oracle for every supported case.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_fmd_{}_{n}", std::process::id()));
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

#[test]
fn two_dim_local_basic() {
    // iverilog: g=5678.
    let o = run("module t; int g;\n\
         task automatic tk(); int m[2][2];\n\
           m[0][0]=5; m[0][1]=6; m[1][0]=7; m[1][1]=8;\n\
           g = m[0][0]*1000 + m[0][1]*100 + m[1][0]*10 + m[1][1];\n\
         endtask\n\
         initial begin tk(); $display(\"g=%0d\", g); $finish; end endmodule\n");
    assert!(o.contains("g=5678"), "2-D local basic:\n{o}");
}

#[test]
fn three_dim_non_square_runtime_index() {
    // iverilog: g=726 (sum of i*100+j*10+k over 2x3x2).
    let o = run("module t; int g;\n\
         task automatic tk(); int m[2][3][2]; int s=0;\n\
           for(int i=0;i<2;i++) for(int j=0;j<3;j++) for(int k=0;k<2;k++) m[i][j][k]=(i*100+j*10+k);\n\
           for(int i=0;i<2;i++) for(int j=0;j<3;j++) for(int k=0;k<2;k++) s+=m[i][j][k];\n\
           g=s;\n\
         endtask\n\
         initial begin tk(); $display(\"g=%0d\", g); $finish; end endmodule\n");
    assert!(o.contains("g=726"), "3-D non-square runtime index:\n{o}");
}

#[test]
fn descending_dims() {
    // iverilog: g=189 (0xAB=171 + 0x12=18).
    let o = run("module t; int g;\n\
         task automatic tk(); logic [7:0] m[1:0][3:0]; m[1][3]=8'hAB; m[0][0]=8'h12; g=m[1][3]+m[0][0]; endtask\n\
         initial begin tk(); $display(\"g=%0d\", g); $finish; end endmodule\n");
    assert!(o.contains("g=189"), "descending dims:\n{o}");
}

#[test]
fn signed_byte_elements() {
    // iverilog: g=-105 — the md-packed slot is whole-unsigned, so the element read must
    // re-stamp $signed (a full-element read of a 2-D md-packed net).
    let o = run("module t; int g;\n\
         task automatic tk(); byte m[2][2]; m[0][0]=-5; m[1][1]=-100; g=m[0][0]+m[1][1]; endtask\n\
         initial begin tk(); $display(\"g=%0d\", g); $finish; end endmodule\n");
    assert!(o.contains("g=-105"), "signed byte elements:\n{o}");
}

#[test]
fn runtime_index_both_dims() {
    let o = run("module t; int g;\n\
         task automatic tk(input int a, input int b); int m[3][3]; m[a][b]=77; g=m[a][b]; endtask\n\
         initial begin tk(1,2); $display(\"g=%0d\", g); $finish; end endmodule\n");
    assert!(o.contains("g=77"), "runtime index both dims:\n{o}");
}

#[test]
fn bit_select_of_element() {
    // iverilog: g=11 — a bit-select INTO a 2-D element (`m[i][j][bit]`) is one index past
    // the element and must still work (not be caught by the partial-index guard).
    let o = run("module t; int g;\n\
         task automatic tk(); logic [7:0] m[2][2]; m[1][0]=8'b1010_1010; g=m[1][0][3]+m[1][0][1]*10; endtask\n\
         initial begin tk(); $display(\"g=%0d\", g); $finish; end endmodule\n");
    assert!(o.contains("g=11"), "bit-select of element:\n{o}");
}

#[test]
fn multidim_in_function_body() {
    // iverilog: r=30.
    let o = run("module t;\n\
         function automatic int f(input int x); int m[2][2]; m[0][0]=x; m[1][1]=x*2; return m[0][0]+m[1][1]; endfunction\n\
         initial begin $display(\"r=%0d\", f(10)); $finish; end endmodule\n");
    assert!(o.contains("r=30"), "multi-dim in function:\n{o}");
}

#[test]
fn survives_across_suspend() {
    // iverilog: at 5 g=14 — the whole md-packed array is one frame slot, so it is isolated
    // across a `#delay` suspend like any frame-local.
    let o = run("module t; int g;\n\
         task automatic tk(); int m[2][2]; m[0][0]=5; #5; m[1][1]=9; g=m[0][0]+m[1][1]; endtask\n\
         initial begin tk(); $display(\"at %0t g=%0d\", $time, g); $finish; end endmodule\n");
    assert!(o.contains("at 5 g=14"), "survives across suspend:\n{o}");
}

// ── correct-or-loud boundaries ───────────────────────────────────────────────

#[test]
fn partial_index_read_stays_loud() {
    // `m[i]` on a 2-D array copied whole (`r = m[0]`) has no sub-array value → loud, not a
    // silent 2-element slice.
    let o = run("module t; int g;\n\
         task automatic tk(); int m[2][2]; int r[2]; m[0][0]=5; r=m[0]; g=r[0]; endtask\n\
         initial begin tk(); $display(\"g=%0d\", g); $finish; end endmodule\n");
    assert!(o.contains("E3009"), "partial-index read must be loud:\n{o}");
}

#[test]
fn whole_multidim_array_arg_stays_loud() {
    // Passing a whole 2-D array as a subroutine arg is loud (the formal binding is 1-D).
    let o = run("module t;\n\
         function automatic int s2(input int a[2][2]); return a[0][0]; endfunction\n\
         task automatic tk(); int m[2][2]; m[0][0]=7; $display(\"s=%0d\", s2(m)); endtask\n\
         initial begin tk(); $finish; end endmodule\n");
    assert!(
        o.contains("E3009"),
        "whole multi-dim array arg must be loud:\n{o}"
    );
}
