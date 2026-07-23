//! §4.5.202: a MULTI-DIMENSIONAL unpacked-array FORMAL on a framed function / `task
//! automatic` (`input int m[2][2]`) loud→supported. §4.5.199 gave frame-LOCAL multi-dim
//! arrays the md-packed N-D layout (`array_formal_ext_dims` + `flatten_word`), but a
//! subroutine array FORMAL stayed 1-D (`classify_array_formal` rejected `dims.len() > 1`).
//! This slice lets the SAME N-D md-packed slot back a formal: the reservation uses
//! `array_formal_ext_dims`, and the call-site whole-array pack (`lower_array_actual_packed`)
//! copies the actual's flat words into the slot in matching row-major order.
//!
//! NO ORACLE: iverilog 13.0 rejects the whole construct ("sorry: Subroutine ports with
//! unpacked dimensions are not yet supported"), so every SUPPORTED case is hand-IEEE
//! (§13.5.1 pass-by-value: `m[i][j] = a[i][j]`, the body may write its own copy without
//! touching the caller).
//!
//! Correct-or-loud: only ASCENDING zero-based dims (`[N]` / `[0:N-1]`) are supported — then
//! declared index == flat position, unambiguous. A DESCENDING dim, a shape / dim-count
//! mismatch, a partial (whole sub-array) select, or a HIER enable with an array formal all
//! stay loud. (INPUT and — since §4.5.204 — OUTPUT/INOUT are all supported; a STATIC task is
//! force-framed, §4.5.203.)
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_mdaf_{}_{n}", std::process::id()));
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
fn task_automatic_2x2_input() {
    // The base case: a 2-D `int m[2][2]` input formal on a framed task. acc = sum = 10.
    let o = run("module tb; int acc;\n\
         task automatic proc(input int m[2][2]);\n\
           acc = m[0][0] + m[0][1] + m[1][0] + m[1][1];\n\
         endtask\n\
         initial begin int a[2][2];\n\
           a[0][0]=1; a[0][1]=2; a[1][0]=3; a[1][1]=4;\n\
           proc(a); $display(\"acc=%0d\", acc); $finish; end endmodule\n");
    assert!(o.contains("acc=10"), "2x2 task input:\n{o}");
}

#[test]
fn function_2x2_element_access() {
    // A function reading each element (place-weighted so any mis-map shows). r = 5678.
    let o = run("module tb;\n\
         function automatic int fsum(input int m[2][2]);\n\
           fsum = m[0][0]*1000 + m[0][1]*100 + m[1][0]*10 + m[1][1];\n\
         endfunction\n\
         initial begin int a[2][2];\n\
           a[0][0]=5; a[0][1]=6; a[1][0]=7; a[1][1]=8;\n\
           $display(\"r=%0d\", fsum(a)); $finish; end endmodule\n");
    assert!(o.contains("r=5678"), "2x2 function element access:\n{o}");
}

#[test]
fn three_dim_2x2x2() {
    // A 3-D formal, filled 0..7 and summed via nested loops. acc = 28.
    let o = run("module tb; int acc;\n\
         task automatic p(input int m[2][2][2]);\n\
           acc=0;\n\
           for(int i=0;i<2;i++) for(int j=0;j<2;j++) for(int k=0;k<2;k++) acc+=m[i][j][k];\n\
         endtask\n\
         initial begin int a[2][2][2]; int c=0;\n\
           for(int i=0;i<2;i++) for(int j=0;j<2;j++) for(int k=0;k<2;k++) begin a[i][j][k]=c; c++; end\n\
           p(a); $display(\"acc=%0d\", acc); $finish; end endmodule\n");
    assert!(o.contains("acc=28"), "3-D formal:\n{o}");
}

#[test]
fn non_square_2x3() {
    // A non-square 2×3 formal, place-weighted across all 6 elements. r = 726189.
    let o = run("module tb;\n\
         function automatic int f(input int m[2][3]);\n\
           f = m[0][0]*100000+m[0][1]*10000+m[0][2]*1000+m[1][0]*100+m[1][1]*10+m[1][2];\n\
         endfunction\n\
         initial begin int a[2][3];\n\
           a[0][0]=7;a[0][1]=2;a[0][2]=6;a[1][0]=1;a[1][1]=8;a[1][2]=9;\n\
           $display(\"r=%0d\", f(a)); $finish; end endmodule\n");
    assert!(o.contains("r=726189"), "non-square 2x3:\n{o}");
}

#[test]
fn signed_byte_element_restamp() {
    // A signed `byte` element must read back negative ($signed re-stamp on the md-packed
    // element read). -100 + 50 - 5 + 10 = -45.
    let o = run("module tb; int acc;\n\
         task automatic p(input byte m[2][2]);\n\
           acc = m[0][0] + m[0][1] + m[1][0] + m[1][1];\n\
         endtask\n\
         initial begin byte a[2][2];\n\
           a[0][0]=-8'sd100; a[0][1]=8'sd50; a[1][0]=-8'sd5; a[1][1]=8'sd10;\n\
           p(a); $display(\"acc=%0d\", acc); $finish; end endmodule\n");
    assert!(o.contains("acc=-45"), "signed byte element:\n{o}");
}

#[test]
fn runtime_index_element() {
    // A runtime `m[i][j]` (not a constant select) — the md-packed part-select handles it.
    let o = run("module tb;\n\
         function automatic int f(input int m[2][2], input int i, input int j);\n\
           f = m[i][j];\n\
         endfunction\n\
         initial begin int a[2][2];\n\
           a[0][0]=11;a[0][1]=22;a[1][0]=33;a[1][1]=44;\n\
           $display(\"r=%0d %0d %0d %0d\", f(a,0,0),f(a,0,1),f(a,1,0),f(a,1,1)); $finish; end endmodule\n");
    assert!(o.contains("r=11 22 33 44"), "runtime index:\n{o}");
}

#[test]
fn packed_vector_element() {
    // A `logic [7:0]` (packed-vector) element. AA^0F^F0^55 = 00.
    let o = run("module tb;\n\
         function automatic logic [7:0] f(input logic [7:0] m[2][2]);\n\
           f = m[0][0] ^ m[0][1] ^ m[1][0] ^ m[1][1];\n\
         endfunction\n\
         initial begin logic [7:0] a[2][2];\n\
           a[0][0]=8'hAA;a[0][1]=8'h0F;a[1][0]=8'hF0;a[1][1]=8'h55;\n\
           $display(\"r=%h\", f(a)); $finish; end endmodule\n");
    assert!(o.contains("r=00"), "packed vector element:\n{o}");
}

#[test]
fn two_state_bit_element() {
    // A 2-state `bit [7:0]` element array (whole-slot X/Z→0 coercion at bind). 200+55 = 255.
    let o = run("module tb;\n\
         function automatic int f(input bit [7:0] m[2][2]); f=m[0][0]+m[1][1]; endfunction\n\
         initial begin bit [7:0] a[2][2]; a[0][0]=8'd200; a[1][1]=8'd55;\n\
           $display(\"r=%0d\", f(a)); $finish; end endmodule\n");
    assert!(o.contains("r=255"), "2-state bit element:\n{o}");
}

#[test]
fn body_element_write_is_pass_by_value() {
    // IEEE §13.5.1: the body may WRITE its own formal copy (`m[0][0]=999`) — the local read
    // sees the write (r=1004) but the CALLER's array is unchanged (a00 stays 1).
    let o = run("module tb;\n\
         function automatic int f(input int m[2][2]); m[0][0]=999; f=m[0][0]+m[1][1]; endfunction\n\
         initial begin\n\
           int a[2][2]; int r;\n\
           a[0][0]=1; a[1][1]=5;\n\
           r = f(a);\n\
           $display(\"r=%0d a00=%0d\", r, a[0][0]); $finish; end endmodule\n");
    assert!(
        o.contains("r=1004 a00=1"),
        "body write must be pass-by-value (local write, caller immune):\n{o}"
    );
}

#[test]
fn mixed_scalar_output_and_array_input() {
    // An array `input` formal alongside a scalar `output` formal. The body's array write is
    // local (a10 stays 3); the scalar output copies out (g=77).
    let o = run("module tb;\n\
         task automatic p(input int m[2][2], output int got);\n\
           m[1][0] = 77; got = m[1][0];\n\
         endtask\n\
         initial begin\n\
           int a[2][2]; int g;\n\
           a[1][0]=3;\n\
           p(a, g);\n\
           $display(\"g=%0d a10=%0d\", g, a[1][0]); $finish; end endmodule\n");
    assert!(
        o.contains("g=77 a10=3"),
        "mixed scalar-output + array-input:\n{o}"
    );
}

#[test]
fn value_immune_across_suspend() {
    // The whole array is copied into the md-packed slot at ENTRY, so a `#5` suspend then a
    // caller mutation cannot leak in — acc reads the entry snapshot (40+2=42), not 999.
    let o = run("module tb; int acc;\n\
         task automatic p(input int m[2][2]); #5 acc = m[0][0] + m[1][1]; endtask\n\
         initial begin int a[2][2]; a[0][0]=40; a[1][1]=2;\n\
           p(a);\n\
           a[0][0]=999;\n\
           $display(\"at %0t acc=%0d\", $time, acc); $finish; end endmodule\n");
    assert!(
        o.contains("at 5 acc=42"),
        "value immune across suspend:\n{o}"
    );
}

#[test]
fn forward_multidim_formal() {
    // A caller's OWN 2-D formal forwarded whole to a callee's matching 2-D formal — both
    // share the identical md-packed layout, so it passes through as a whole-net value. 123.
    let o = run("module tb;\n\
         function automatic int inner(input int m[2][2]); inner=m[0][0]+m[1][1]; endfunction\n\
         function automatic int outer(input int m[2][2]); outer=inner(m); endfunction\n\
         initial begin int a[2][2]; a[0][0]=100;a[1][1]=23;\n\
           $display(\"r=%0d\", outer(a)); $finish; end endmodule\n");
    assert!(o.contains("r=123"), "multi-dim formal forward:\n{o}");
}

#[test]
fn one_dim_formal_still_works() {
    // Regression: a plain 1-D formal keeps the long-standing behavior (byte-identical
    // reservation + pack). 1+2+3+4 = 10.
    let o = run("module tb;\n\
         function automatic int f(input int m[4]); f=m[0]+m[1]+m[2]+m[3]; endfunction\n\
         initial begin int a[4]; a[0]=1;a[1]=2;a[2]=3;a[3]=4;\n\
           $display(\"r=%0d\", f(a)); $finish; end endmodule\n");
    assert!(o.contains("r=10"), "1-D formal regression:\n{o}");
}

// ── correct-or-loud boundaries ───────────────────────────────────────────────

#[test]
fn descending_dim_stays_loud() {
    // A DESCENDING multi-dim formal (`[1:0][1:0]`) — the md-packed read is index-major but
    // the actual's physical storage is declaration-major, so a passthrough would reverse
    // elements. Loud (only ascending `[N]` / `[0:N-1]` dims are supported).
    let o = run("module tb;\n\
         function automatic int f(input int m[1:0][1:0]); f=m[0][0]; endfunction\n\
         initial begin int a[1:0][1:0]; a[0][0]=9; $display(\"r=%0d\",f(a)); $finish; end endmodule\n");
    assert!(
        o.contains("E3009") && o.contains("descending"),
        "descending multi-dim formal must be loud:\n{o}"
    );
}

#[test]
fn shape_mismatch_actual_stays_loud() {
    // A 2×3 actual for a 2×2 formal (6 vs 4 elements) — loud.
    let o = run("module tb; int acc;\n\
         task automatic p(input int m[2][2]); acc=m[0][0]; endtask\n\
         initial begin int a[2][3]; a[0][0]=9; p(a); $display(\"acc=%0d\",acc); $finish; end endmodule\n");
    assert!(
        o.contains("E3009"),
        "shape-mismatch actual must be loud:\n{o}"
    );
}

#[test]
fn partial_row_select_stays_loud() {
    // A partial select `m[0]` (a whole ROW of a 2-D formal) has no scalar value in the
    // md-packed slot — loud (index every dimension down to an element).
    let o = run("module tb;\n\
         function automatic int f(input int m[2][2]); int row[2]; row=m[0]; f=row[0]; endfunction\n\
         initial begin int a[2][2]; a[0][0]=9; $display(\"r=%0d\",f(a)); $finish; end endmodule\n");
    assert!(
        o.contains("E3009"),
        "partial (whole-row) select must be loud:\n{o}"
    );
}

#[test]
fn hier_call_with_array_formal_stays_loud() {
    // A HIER enable `u.p(a)` whose callee has an array formal — cross-boundary array copy is
    // a separate follow-on; the hier gate accepts only scalar formals. Loud.
    let o = run("module sub; int acc; task automatic p(input int m[2][2]); acc=m[0][0]; endtask endmodule\n\
         module tb; sub u();\n\
         initial begin int a[2][2]; a[0][0]=9; u.p(a); $display(\"acc=%0d\",u.acc); $finish; end endmodule\n");
    assert!(
        o.contains("E3009"),
        "hier call with an array formal must be loud:\n{o}"
    );
}

#[test]
fn output_multidim_array_formal_supported() {
    // §4.5.204: OUTPUT multi-dim is now supported — the §4.5.193 copy-out unpack writes each
    // caller element with a FULLY-indexed `caller[i0][i1]` (row-major decomposition of the
    // flat index), not a whole-array assign. 10 20 30 40.
    let o = run("module tb;\n\
         task automatic p(output int m[2][2]); m[0][0]=10; m[0][1]=20; m[1][0]=30; m[1][1]=40; endtask\n\
         initial begin int a[2][2]; p(a);\n\
           $display(\"%0d %0d %0d %0d\", a[0][0], a[0][1], a[1][0], a[1][1]); $finish; end endmodule\n");
    assert!(
        o.contains("10 20 30 40"),
        "output multi-dim array formal:\n{o}"
    );
}

#[test]
fn inout_multidim_array_formal_supported() {
    // §4.5.204: INOUT = copy-in (§4.5.202) + copy-out (§4.5.204). Read-modify-write doubles
    // each element. 1 2 3 4 → 2 4 6 8.
    let o = run("module tb;\n\
         task automatic dbl(inout int m[2][2]);\n\
           for(int i=0;i<2;i++) for(int j=0;j<2;j++) m[i][j]=m[i][j]*2;\n\
         endtask\n\
         initial begin int a[2][2]; a[0][0]=1;a[0][1]=2;a[1][0]=3;a[1][1]=4; dbl(a);\n\
           $display(\"%0d %0d %0d %0d\", a[0][0],a[0][1],a[1][0],a[1][1]); $finish; end endmodule\n");
    assert!(o.contains("2 4 6 8"), "inout multi-dim array formal:\n{o}");
}

#[test]
fn output_non_square_and_3d() {
    // Non-square 2×3 output — the row-major decomposition must handle unequal dim sizes.
    let o = run("module tb;\n\
         task automatic p(output int m[2][3]);\n\
           for(int i=0;i<2;i++) for(int j=0;j<3;j++) m[i][j]=i*10+j;\n\
         endtask\n\
         initial begin int a[2][3]; p(a);\n\
           $display(\"%0d %0d %0d %0d %0d %0d\", a[0][0],a[0][1],a[0][2],a[1][0],a[1][1],a[1][2]); $finish; end endmodule\n");
    assert!(o.contains("0 1 2 10 11 12"), "non-square output:\n{o}");
}

#[test]
fn output_3d() {
    // 3-D output, summed + spot-checked.
    let o = run("module tb;\n\
         task automatic p(output int m[2][2][2]);\n\
           for(int i=0;i<2;i++) for(int j=0;j<2;j++) for(int k=0;k<2;k++) m[i][j][k]=i*4+j*2+k;\n\
         endtask\n\
         initial begin int a[2][2][2]; int s=0; p(a);\n\
           for(int i=0;i<2;i++) for(int j=0;j<2;j++) for(int k=0;k<2;k++) s+=a[i][j][k];\n\
           $display(\"sum=%0d a010=%0d a111=%0d\", s, a[0][1][0], a[1][1][1]); $finish; end endmodule\n");
    assert!(o.contains("sum=28 a010=2 a111=7"), "3-D output:\n{o}");
}

#[test]
fn output_signed_byte_multidim() {
    // A signed `byte` element reads back negative from the caller after copy-out.
    let o = run("module tb;\n\
         task automatic p(output byte m[2][2]); m[0][0]=-8'sd100; m[1][1]=8'sd50; endtask\n\
         initial begin byte a[2][2]; a[0][1]=0;a[1][0]=0; p(a);\n\
           $display(\"%0d %0d\", a[0][0], a[1][1]); $finish; end endmodule\n");
    assert!(o.contains("-100 50"), "signed output multi-dim:\n{o}");
}

#[test]
fn descending_output_multidim_stays_loud() {
    // A descending multi-dim output formal stays loud (not an ascending-zero-based shape).
    let o = run("module tb;\n\
         task automatic p(output int m[1:0][1:0]); m[0][0]=1; endtask\n\
         initial begin int a[1:0][1:0]; p(a); $display(\"%0d\",a[0][0]); $finish; end endmodule\n");
    assert!(
        o.contains("E3009"),
        "descending output multi-dim must be loud:\n{o}"
    );
}
