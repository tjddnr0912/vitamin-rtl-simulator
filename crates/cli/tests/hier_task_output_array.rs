//! item 1 (ROADMAP §0): a hierarchical TASK enable whose callee has an OUTPUT/INOUT unpacked-
//! array formal (`u.gen(a)` where `task gen(output int d[4])`) loud→supported. §4.5.207 handled
//! the INPUT half (defer the actual net + resolve-time pack into the md-packed slot); the
//! OUTPUT/INOUT half was loud because the copy-OUT has to be synthesized at resolution — the
//! callee's array shape is unknown until the child instance is elaborated.
//!
//! Mechanism (the deferred copy-out): the hier_tasks gate now admits a fixed unpacked-array
//! formal of ANY direction; at resolution `resolve_deferred_hier_task_call` reserves a fresh
//! packed temp net, adds a scalar out-bind (the callee's md-packed slot → the temp at the task's
//! exit), and PREPENDS an unpack (`caller[i] = temp[i*ew +: ew]`) to the enable's ret block. The
//! unpack is direct IR — position `i` ↔ the caller's flat word `i`, the exact reverse of
//! `pack_hier_array_actual` — so the round-trip is correct by construction. INOUT additionally
//! PACKS the caller array into the slot at entry (the §4.5.207 copy-in), i.e. IEEE §13.5.2
//! pass-by-value-result. Reuses the §4.5.193/204 local copy-out shape + §4.5.201 scalar out-bind.
//!
//! NO ORACLE: iverilog rejects unpacked subroutine ports outright, so every supported case is
//! hand-IEEE (§13.5.1/2) verified by write→read-back self-consistency and cross-check with the
//! LOCAL output-array path (`frame_task_output_array.rs`, which IS differential-verified).
//!
//! Correct-or-loud: an unsupported array SHAPE (descending non-zero-base), a shape mismatch, an
//! array formal fed a scalar actual, and a string/dynamic output array all stay loud.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_htoa_{}_{n}", std::process::id()));
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
fn output_1d_array() {
    // The DUT-read idiom: a hier task fills the caller's array.
    let o = run("module sub;\n\
         task automatic gen(output int d[4]); for(int i=0;i<4;i++) d[i]=(i+1)*10; endtask\n\
         endmodule\n\
         module t; sub u();\n\
         initial begin int a[4]; a[0]=99;a[1]=99;a[2]=99;a[3]=99; u.gen(a);\n\
           $display(\"%0d %0d %0d %0d\", a[0],a[1],a[2],a[3]); $finish; end endmodule\n");
    assert!(o.contains("10 20 30 40"), "output 1-D array:\n{o}");
}

#[test]
fn inout_1d_array() {
    // INOUT = copy-in at entry + copy-out at exit (§13.5.2 pass-by-value-result).
    let o = run("module sub;\n\
         task automatic bump(inout int d[4]); for(int i=0;i<4;i++) d[i]=d[i]+5; endtask\n\
         endmodule\n\
         module t; sub u();\n\
         initial begin int a[4]; a[0]=1;a[1]=2;a[2]=3;a[3]=4; u.bump(a);\n\
           $display(\"%0d %0d %0d %0d\", a[0],a[1],a[2],a[3]); $finish; end endmodule\n");
    assert!(o.contains("6 7 8 9"), "inout 1-D array:\n{o}");
}

#[test]
fn inout_read_modify_write() {
    // The body reads the OLD element then writes the NEW — proves copy-in landed before the body.
    let o = run("module sub;\n\
         task automatic dbl(inout int d[3]); for(int i=0;i<3;i++) d[i]=d[i]*2; endtask\n\
         endmodule\n\
         module t; sub u();\n\
         initial begin int a[3]; a[0]=1;a[1]=2;a[2]=3; u.dbl(a);\n\
           $display(\"%0d %0d %0d\", a[0],a[1],a[2]); $finish; end endmodule\n");
    assert!(o.contains("2 4 6"), "inout RMW:\n{o}");
}

#[test]
fn output_multidim_2x2() {
    let o = run("module sub;\n\
         task automatic gen(output int m[2][2]); m[0][0]=10;m[0][1]=20;m[1][0]=30;m[1][1]=40; endtask\n\
         endmodule\n\
         module t; sub u();\n\
         initial begin int a[2][2]; u.gen(a);\n\
           $display(\"%0d %0d %0d %0d\", a[0][0],a[0][1],a[1][0],a[1][1]); $finish; end endmodule\n");
    assert!(o.contains("10 20 30 40"), "output 2x2:\n{o}");
}

#[test]
fn output_nonsquare_2x3_byte() {
    // Non-square is the reversal clincher — a mis-mapped element would be visibly wrong.
    let o = run("module sub;\n\
         task automatic gen(output byte m[2][3]); for(int i=0;i<2;i++) for(int j=0;j<3;j++) m[i][j]=i*10+j; endtask\n\
         endmodule\n\
         module t; sub u();\n\
         initial begin byte a[2][3]; u.gen(a);\n\
           $display(\"%0d %0d %0d %0d %0d %0d\", a[0][0],a[0][1],a[0][2],a[1][0],a[1][1],a[1][2]); $finish; end endmodule\n");
    assert!(o.contains("0 1 2 10 11 12"), "output non-square 2x3:\n{o}");
}

#[test]
fn output_3d_array() {
    let o = run("module sub;\n\
         task automatic gen(output int m[2][2][2]);\n\
           for(int i=0;i<2;i++) for(int j=0;j<2;j++) for(int l=0;l<2;l++) m[i][j][l]=i*4+j*2+l;\n\
         endtask endmodule\n\
         module t; sub u();\n\
         initial begin int a[2][2][2]; int s=0; u.gen(a);\n\
           for(int i=0;i<2;i++) for(int j=0;j<2;j++) for(int l=0;l<2;l++) s+=a[i][j][l];\n\
           $display(\"sum=%0d\", s); $finish; end endmodule\n");
    assert!(o.contains("sum=28"), "output 3-D array:\n{o}");
}

#[test]
fn output_signed_byte() {
    // Element signedness is the caller array's — a negative element must read as negative.
    let o = run("module sub;\n\
         task automatic gen(output byte d[2]); d[0]=-8'sd100; d[1]=8'sd50; endtask\n\
         endmodule\n\
         module t; sub u();\n\
         initial begin byte a[2]; u.gen(a); $display(\"%0d %0d\", a[0], a[1]); $finish; end endmodule\n");
    assert!(o.contains("-100 50"), "output signed byte:\n{o}");
}

#[test]
fn mixed_scalar_out_and_array_out() {
    // A scalar output + an array output in one task — both copy-outs coexist.
    let o = run("module sub;\n\
         task automatic gen(output int cnt, output int d[3]); cnt=3; d[0]=11;d[1]=22;d[2]=33; endtask\n\
         endmodule\n\
         module t; sub u();\n\
         initial begin int c; int a[3]; u.gen(c,a);\n\
           $display(\"%0d %0d %0d %0d\", c, a[0],a[1],a[2]); $finish; end endmodule\n");
    assert!(
        o.contains("3 11 22 33"),
        "mixed scalar-out + array-out:\n{o}"
    );
}

#[test]
fn partial_write_defaults_zero() {
    // IEEE §13.5.2: an unwritten output element copies out the slot's default (0, not the
    // caller's prior value) — the whole array is passed by value-result.
    let o = run("module sub;\n\
         task automatic gen(output int d[4]); d[1]=55; endtask\n\
         endmodule\n\
         module t; sub u();\n\
         initial begin int a[4]; a[0]=9;a[1]=9;a[2]=9;a[3]=9; u.gen(a);\n\
           $display(\"%0d %0d %0d %0d\", a[0],a[1],a[2],a[3]); $finish; end endmodule\n");
    assert!(o.contains("0 55 0 0"), "partial write defaults to 0:\n{o}");
}

#[test]
fn output_array_nested_in_frame_body() {
    // §4.5.208 path: the hier enable is NESTED inside a frame-TASK body (func_block route) — the
    // copy-out unpack must be injected into the func_blocks ret block, not a process block.
    let o = run("module sub;\n\
         task automatic gen(output int d[3]); d[0]=7;d[1]=8;d[2]=9; endtask\n\
         endmodule\n\
         module t; sub u(); int r[3];\n\
         task automatic driver(); u.gen(r); endtask\n\
         initial begin driver(); $display(\"%0d %0d %0d\", r[0],r[1],r[2]); $finish; end endmodule\n");
    assert!(
        o.contains("7 8 9"),
        "output array nested in frame body:\n{o}"
    );
}

#[test]
fn output_array_survives_suspend() {
    // The copy-out runs at the task's EXIT, after a `#delay` in the body — the value-result
    // semantics hold across suspension.
    let o = run("module sub;\n\
         task automatic gen(output int d[2]); d[0]=1; #5; d[1]=2; endtask\n\
         endmodule\n\
         module t; sub u();\n\
         initial begin int a[2]; u.gen(a);\n\
           $display(\"t=%0t a=%0d %0d\", $time, a[0], a[1]); $finish; end endmodule\n");
    assert!(
        o.contains("t=5 a=1 2"),
        "output array survives suspend:\n{o}"
    );
}

#[test]
fn output_array_deep_path() {
    // A 3-segment instance path `m.lf.gen` to the output-array task.
    let o = run("module leaf;\n\
         task automatic gen(output int d[3]); d[0]=100;d[1]=200;d[2]=300; endtask endmodule\n\
         module mid; leaf lf(); endmodule\n\
         module t; mid m();\n\
         initial begin int a[3]; m.lf.gen(a);\n\
           $display(\"%0d %0d %0d\", a[0],a[1],a[2]); $finish; end endmodule\n");
    assert!(o.contains("100 200 300"), "output array deep path:\n{o}");
}

#[test]
fn output_array_per_instance_isolation() {
    // Two instances write DIFFERENT caller arrays — no cross-instance leak.
    let o = run("module sub #(parameter K=0);\n\
         task automatic gen(output int d[2]); d[0]=K; d[1]=K+1; endtask\n\
         endmodule\n\
         module t; sub #(100) u1(); sub #(200) u2();\n\
         initial begin int a[2]; int b[2]; u1.gen(a); u2.gen(b);\n\
           $display(\"%0d %0d %0d %0d\", a[0],a[1],b[0],b[1]); $finish; end endmodule\n");
    assert!(
        o.contains("100 101 200 201"),
        "per-instance isolation:\n{o}"
    );
}

// ── correct-or-loud boundaries ───────────────────────────────────────────────

#[test]
fn output_array_descending_nonzero_base_stays_loud() {
    // A non-zero-base DESCENDING array shape is rejected in `classify_unpacked_array` (§4.5.206),
    // so the formal has no md-packed slot → the resolver treats it as a scalar → the whole-array
    // actual is loud there (correct-or-loud).
    let o = run(
        "module sub; task automatic gen(output int d[4:1]); d[1]=1; endtask endmodule\n\
         module t; sub u(); initial begin int a[4:1]; u.gen(a); $finish; end endmodule\n",
    );
    assert!(
        o.contains("E3009"),
        "descending non-zero-base output must be loud:\n{o}"
    );
}

#[test]
fn output_array_shape_mismatch_stays_loud() {
    // A shape mismatch (actual `[3]`, formal `[4]`) is loud.
    let o = run(
        "module sub; task automatic gen(output int d[4]); d[0]=1; endtask endmodule\n\
         module t; sub u(); initial begin int a[3]; u.gen(a); $finish; end endmodule\n",
    );
    assert!(
        o.contains("E3009"),
        "output array shape mismatch must be loud:\n{o}"
    );
}

#[test]
fn output_array_scalar_actual_stays_loud() {
    // An array formal fed a scalar actual is a type mismatch. Loud.
    let o = run(
        "module sub; task automatic gen(output int d[4]); d[0]=1; endtask endmodule\n\
         module t; sub u(); initial begin int x; u.gen(x); $finish; end endmodule\n",
    );
    assert!(
        o.contains("E3009"),
        "array formal + scalar actual must be loud:\n{o}"
    );
}

#[test]
fn output_string_array_stays_loud() {
    // A `string` array formal is not hier-callable (no md-packed slot). Loud.
    let o = run(
        "module sub; task automatic gen(output string d[2]); d[0]=\"a\"; endtask endmodule\n\
         module t; sub u(); initial begin string a[2]; u.gen(a); $finish; end endmodule\n",
    );
    assert!(
        o.contains("E3009") || o.contains("E3010"),
        "string output array must be loud:\n{o}"
    );
}
