//! Unpacked-array assignment (whole-array and partial slices) — Phase-1.x
//! precision item ② unlocked from the v1 loud-reject.
//!
//! ⚠️ ORACLE NOTE: iverilog 13.0 rejects fixed-size unpacked array assignment
//! outright ("the type of the variable doesn't match the context type" /
//! "Assignment to an array slice is not yet supported"), so this lane is
//! hand-pinned to IEEE 1800-2017 §7.6: source and target need the same number
//! of unpacked dimensions, the same SIZE per dimension, and assignment-
//! compatible elements; elements correspond POSITIONALLY left-to-right in
//! declared index order (the LRM's own example pairs `int A[10:1]` with
//! `int B[0:9]` as A[10]=B[0] … A[1]=B[9]).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_aa_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d) // a `$dumpvars` test writes its VCD beside the design
        .output()
        .expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    )
}

#[test]
fn whole_array_blocking_copy() {
    let (out, err, code) = run("module top;\n\
         reg [7:0] a [0:3]; reg [7:0] b [0:3];\n\
         integer i;\n\
         initial begin\n\
           for (i=0;i<4;i=i+1) b[i] = 8'h10 + i;\n\
           a = b;\n\
           $display(\"a=%h %h %h %h\", a[0], a[1], a[2], a[3]);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("a=10 11 12 13"), "got:\n{out}");
}

#[test]
fn opposite_directions_pair_left_to_left() {
    // IEEE §7.6 positional correspondence: target `[3:0]` left index is 3,
    // source `[0:3]` left index is 0 ⇒ a[3]=b[0] … a[0]=b[3].
    let (out, err, code) = run("module top;\n\
         reg [7:0] a [3:0]; reg [7:0] b [0:3];\n\
         integer i;\n\
         initial begin\n\
           for (i=0;i<4;i=i+1) b[i] = 8'h10 + i;\n\
           a = b;\n\
           $display(\"a3=%h a2=%h a1=%h a0=%h\", a[3], a[2], a[1], a[0]);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("a3=10 a2=11 a1=12 a0=13"), "got:\n{out}");
}

#[test]
fn row_slice_write_with_dynamic_index() {
    let (out, err, code) = run("module top;\n\
         reg [7:0] g [0:2][0:3]; reg [7:0] row [0:3];\n\
         integer i, k;\n\
         initial begin\n\
           for (k=0;k<4;k=k+1) row[k] = 8'ha0 + k;\n\
           i = 1;\n\
           g[i] = row;\n\
           $display(\"g1=%h %h %h %h\", g[1][0], g[1][1], g[1][2], g[1][3]);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("g1=a0 a1 a2 a3"), "got:\n{out}");
}

#[test]
fn row_slice_read_into_array() {
    let (out, err, code) = run("module top;\n\
         reg [7:0] g [0:2][0:3]; reg [7:0] row [0:3];\n\
         initial begin\n\
           g[0][0]=8'h55; g[0][1]=8'h56; g[0][2]=8'h57; g[0][3]=8'h58;\n\
           row = g[0];\n\
           $display(\"row=%h %h %h %h\", row[0], row[1], row[2], row[3]);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("row=55 56 57 58"), "got:\n{out}");
}

#[test]
fn slice_to_slice_same_array_disjoint_rows() {
    let (out, err, code) = run("module top;\n\
         reg [7:0] g [0:2][0:1];\n\
         initial begin\n\
           g[2][0]=8'hde; g[2][1]=8'had;\n\
           g[0][0]=8'h00; g[0][1]=8'h00;\n\
           g[0] = g[2];\n\
           $display(\"g0=%h %h\", g[0][0], g[0][1]);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("g0=de ad"), "got:\n{out}");
}

#[test]
fn three_d_slice_copies_two_d_residual() {
    let (out, err, code) = run("module top;\n\
         reg [3:0] c [0:1][0:1][0:2]; reg [3:0] m [0:1][0:2];\n\
         integer j, k;\n\
         initial begin\n\
           for (j=0;j<2;j=j+1) for (k=0;k<3;k=k+1) m[j][k] = j*4 + k + 1;\n\
           c[1] = m;\n\
           $display(\"c1=%h%h%h %h%h%h\", c[1][0][0], c[1][0][1], c[1][0][2],\n\
                    c[1][1][0], c[1][1][1], c[1][1][2]);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("c1=123 567"), "got:\n{out}");
}

#[test]
fn descending_residual_dim_pairs_left_to_left() {
    // Residual dim `[3:0]` (left=3) feeding `[0:3]` (left=0): r[0]=g[1][3].
    let (out, err, code) = run("module top;\n\
         reg [3:0] g [0:1][3:0]; reg [3:0] r [0:3];\n\
         initial begin\n\
           g[1][3]=4'ha; g[1][2]=4'hb; g[1][1]=4'hc; g[1][0]=4'hd;\n\
           r = g[1];\n\
           $display(\"r=%h%h%h%h\", r[0], r[1], r[2], r[3]);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("r=abcd"), "got:\n{out}");
}

#[test]
fn nba_whole_array_lands_in_nba_region() {
    let (out, err, code) = run("module top;\n\
         reg [7:0] a [0:1]; reg [7:0] b [0:1];\n\
         initial begin\n\
           b[0]=8'h21; b[1]=8'h22; a[0]=8'h00; a[1]=8'h00;\n\
           a <= b;\n\
           $display(\"pre=%h %h\", a[0], a[1]);\n\
           #1 $display(\"post=%h %h\", a[0], a[1]);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("pre=00 00"), "NBA must not land early:\n{out}");
    assert!(out.contains("post=21 22"), "got:\n{out}");
}

#[test]
fn nba_array_with_transport_delay() {
    let (out, err, code) = run("module top;\n\
         reg [7:0] a [0:1]; reg [7:0] b [0:1];\n\
         initial begin\n\
           b[0]=8'h31; b[1]=8'h32; a[0]=8'h00; a[1]=8'h00;\n\
           a <= #2 b;\n\
           b[0]=8'hff;\n\
           #1 $display(\"t1=%h %h\", a[0], a[1]);\n\
           #2 $display(\"t3=%h %h\", a[0], a[1]);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    // RHS captured at schedule time (31), not at delivery (ff).
    assert!(out.contains("t1=00 00"), "got:\n{out}");
    assert!(out.contains("t3=31 32"), "got:\n{out}");
}

#[test]
fn nba_array_swap() {
    // The classic NBA witness: both RHS reads capture pre-update values.
    let (out, err, code) = run("module top;\n\
         reg [7:0] a [0:1]; reg [7:0] b [0:1];\n\
         initial begin\n\
           a[0]=8'h0a; a[1]=8'h0b; b[0]=8'h1a; b[1]=8'h1b;\n\
           a <= b; b <= a;\n\
           #1 $display(\"a=%h %h b=%h %h\", a[0], a[1], b[0], b[1]);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("a=1a 1b b=0a 0b"), "got:\n{out}");
}

#[test]
fn dual_descending_two_d_whole_copy() {
    // Both dims descending on the source: position (0,0) is s[1][3] (left
    // index of each dim), landing in t[0][0].
    let (out, err, code) = run("module top;\n\
         reg [3:0] s [1:0][3:0]; reg [3:0] t [0:1][0:3];\n\
         integer i, j;\n\
         initial begin\n\
           for (i=0;i<2;i=i+1) for (j=0;j<4;j=j+1) s[i][j] = i*8 + j;\n\
           t = s;\n\
           $display(\"t00=%0d t03=%0d t10=%0d t13=%0d\", t[0][0], t[0][3], t[1][0], t[1][3]);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("t00=11 t03=8 t10=3 t13=0"), "got:\n{out}");
}

#[test]
fn size_mismatch_is_loud() {
    let (_, err, code) = run("module top;\n\
         reg [7:0] a [0:3]; reg [7:0] b [0:4];\n\
         initial begin a = b; $finish; end\n\
         endmodule\n");
    assert_ne!(code, Some(0), "must reject; stderr:\n{err}");
    assert!(err.contains("VITA-E3009"), "E3009 expected:\n{err}");
}

#[test]
fn element_width_mismatch_is_loud() {
    let (_, err, code) = run("module top;\n\
         reg [7:0] a [0:3]; reg [3:0] b [0:3];\n\
         initial begin a = b; $finish; end\n\
         endmodule\n");
    assert_ne!(code, Some(0), "must reject; stderr:\n{err}");
    assert!(err.contains("VITA-E3009"), "E3009 expected:\n{err}");
}

#[test]
fn scalar_rhs_to_whole_array_is_loud() {
    // Was a silent word-0 write before this item — must be loud now.
    let (_, err, code) = run("module top;\n\
         reg [7:0] a [0:3];\n\
         initial begin a = 8'h5; $finish; end\n\
         endmodule\n");
    assert_ne!(code, Some(0), "must reject; stderr:\n{err}");
    assert!(err.contains("VITA-E3009"), "E3009 expected:\n{err}");
}

#[test]
fn whole_array_read_outside_assignment_is_loud() {
    // Was a silent word-0 read before this item — must be loud now.
    let (_, err, code) = run("module top;\n\
         reg [7:0] a [0:3];\n\
         initial begin $display(\"%h\", a); $finish; end\n\
         endmodule\n");
    assert_ne!(code, Some(0), "must reject; stderr:\n{err}");
    assert!(err.contains("VITA-E3009"), "E3009 expected:\n{err}");
}

#[test]
fn blocking_intra_delay_on_array_is_loud() {
    let (_, err, code) = run("module top;\n\
         reg [7:0] a [0:1]; reg [7:0] b [0:1];\n\
         initial begin a = #2 b; $finish; end\n\
         endmodule\n");
    assert_ne!(code, Some(0), "must reject; stderr:\n{err}");
    assert!(err.contains("VITA-E3009"), "E3009 expected:\n{err}");
}

#[test]
fn slice_index_reading_target_array_is_loud() {
    // Pathological aliasing (`g[g[0][0]] = row`): the element-wise expansion
    // would re-evaluate the moved index mid-copy, so it is rejected loudly.
    let (_, err, code) = run("module top;\n\
         reg [7:0] g [0:2][0:3]; reg [7:0] row [0:3];\n\
         initial begin g[g[0][0]] = row; $finish; end\n\
         endmodule\n");
    assert_ne!(code, Some(0), "must reject; stderr:\n{err}");
    assert!(err.contains("VITA-E3009"), "E3009 expected:\n{err}");
}

#[test]
fn dumpvars_with_array_arg_keeps_running() {
    // Item-⑤ status quo: `$dumpvars(1, mem)` must NOT trip the new whole-array
    // loud check (v1 dumps all signals anyway; doc-01 known simplification).
    let (out, err, code) = run("module top;\n\
         reg [7:0] mem [0:3];\n\
         initial begin\n\
           $dumpvars(1, mem);\n\
           mem[0] = 8'h7;\n\
           $display(\"m0=%h\", mem[0]);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("m0=07"), "got:\n{out}");
}

#[test]
fn modport_input_array_assignment_is_loud() {
    // Adversarial find #1: the array-assign expansion bypassed the modport
    // write check (`p.arr[0] = x` was loud, `p.arr = l` was a silent write).
    let (_, err, code) = run("interface bus_if;\n\
         logic [7:0] arr [0:2];\n\
         modport mp (input arr);\n\
         endinterface\n\
         module consumer(bus_if.mp p);\n\
         reg [7:0] l [0:2];\n\
         initial begin l[0]=1; l[1]=2; l[2]=3; p.arr = l; end\n\
         endmodule\n\
         module top;\n\
         bus_if b();\n\
         consumer c(.p(b));\n\
         endmodule\n");
    assert_ne!(code, Some(0), "must reject; stderr:\n{err}");
    assert!(
        err.contains("modport"),
        "modport write rejection expected:\n{err}"
    );
}

#[test]
fn task_out_formal_bound_to_array_read_is_loud() {
    // Adversarial find #2: reading a task output formal whose actual is a
    // whole array used to slip past the loud check (silent word-0 read,
    // `x=xx` exit 0 — the WRITE side was already loud, so the body must
    // only READ to isolate the defect).
    let (_, err, code) = run("module top;\n\
         reg [7:0] mem [0:3]; reg [7:0] x;\n\
         task t(output [7:0] o);\n\
           begin x = o; end\n\
         endtask\n\
         initial begin t(mem); $finish; end\n\
         endmodule\n");
    assert_ne!(code, Some(0), "must reject; stderr:\n{err}");
    assert!(err.contains("VITA-E3009"), "E3009 expected:\n{err}");
}

#[test]
fn target_named_new_in_slice_index_is_loud() {
    // Adversarial find #3: the V2005 net-named-`new` fallback turns this
    // index into a read of the target. The aliasing guard now has a `New`
    // arm; before that, the outcome was loud only by luck (the fallback's
    // own partial-slice rejection) — this pins that it STAYS loud.
    let (_, err, code) = run("module top;\n\
         reg [7:0] new [0:2][0:3]; reg [7:0] row [0:3];\n\
         initial begin new[new[0][0]] = row; $finish; end\n\
         endmodule\n");
    assert_ne!(code, Some(0), "must reject; stderr:\n{err}");
    assert!(err.contains("VITA-E3009"), "E3009 expected:\n{err}");
}

#[test]
fn signedness_mismatch_is_loud() {
    // §6.22.2: equivalent element types must agree on signedness.
    let (_, err, code) = run("module top;\n\
         reg signed [7:0] a [0:3]; reg [7:0] b [0:3];\n\
         initial begin a = b; $finish; end\n\
         endmodule\n");
    assert_ne!(code, Some(0), "must reject; stderr:\n{err}");
    assert!(err.contains("VITA-E3009"), "E3009 expected:\n{err}");
}

#[test]
fn one_element_array_is_still_an_array() {
    // Adversarial find #5: `[0:0]` arrays were exempt from every rule
    // (array_len > 1 test) — scalar RHS must be loud, copy must work.
    let (_, err, code) = run("module top;\n\
         reg [7:0] a [0:0];\n\
         initial begin a = 8'h5; $finish; end\n\
         endmodule\n");
    assert_ne!(code, Some(0), "must reject; stderr:\n{err}");
    assert!(err.contains("VITA-E3009"), "E3009 expected:\n{err}");
    let (out, err2, code2) = run("module top;\n\
         reg [7:0] a [0:0]; reg [7:0] b [0:0];\n\
         initial begin b[0] = 8'h42; a = b;\n\
           $display(\"a0=%h\", a[0]); $finish; end\n\
         endmodule\n");
    assert_eq!(code2, Some(0), "stderr:\n{err2}");
    assert!(out.contains("a0=42"), "got:\n{out}");
}

#[test]
fn partial_slice_in_expression_stays_loud() {
    // `g[0]` outside an array-assignment context keeps the v1 loud reject.
    let (_, err, code) = run("module top;\n\
         reg [7:0] g [0:2][0:3];\n\
         initial begin $display(\"%h\", g[0]); $finish; end\n\
         endmodule\n");
    assert_ne!(code, Some(0), "must reject; stderr:\n{err}");
    assert!(err.contains("VITA-E3009"), "E3009 expected:\n{err}");
}
