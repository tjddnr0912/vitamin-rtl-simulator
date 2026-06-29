//! Per-dimension bounds checks (Phase-1.x precision item ③).
//!
//! v1 simplification (doc-01): a multi-dim index only had a FLAT-space check,
//! so an inner-dim over-index ALIASED into the neighbouring row (`g[0][5]` on
//! `[0:1][0:2]` read `g[1][2]`) — a silent-wrong. IEEE 1800-2017 §7.4.6: an
//! out-of-bounds index makes the whole access invalid (read = X, write = no-op),
//! PER DIMENSION.
//!
//! ⚠️ ORACLE NOTE: iverilog 13.0 itself aliases these accesses (both the
//! unpacked word space and the packed bit space — verified live 2026-06-12),
//! i.e. it shares the pre-fix behavior, so this lane is hand-pinned to the
//! LRM (same precedent as expression force / assoc arrays). 1-D behavior is
//! ALREADY conformant via lo-normalization wrap + the flat check and stays
//! byte-identical (no guard emitted for d == 1).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_ab_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    )
}

#[test]
fn inner_dim_over_index_reads_x_not_alias() {
    // THE silent-wrong fix: flat offset 5 used to alias g[1][2] (=22).
    let (out, err, code) = run("module top;\n\
         reg [7:0] g [0:1][0:2];\n\
         integer i;\n\
         initial begin\n\
           g[0][0]=8'h10; g[1][2]=8'h22;\n\
           i = 5;\n\
           $display(\"r=%h\", g[0][i]);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert!(
        err.contains("VITA-E4002"),
        "OOB must emit the runtime range diagnostic:\n{err}"
    );
    let _ = code; // exit 1 by the severity contract (E4002 is an ERROR)
    assert!(
        out.contains("r=xx"),
        "OOB read must be X, not an alias:\n{out}"
    );
}

#[test]
fn inner_dim_over_index_write_is_noop() {
    let (out, err, code) = run("module top;\n\
         reg [7:0] g [0:1][0:2];\n\
         integer i;\n\
         initial begin\n\
           g[1][2]=8'h22;\n\
           i = 5;\n\
           g[0][i] = 8'hee;\n\
           $display(\"g12=%h\", g[1][2]);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert!(
        err.contains("VITA-E4002"),
        "OOB must emit the runtime range diagnostic:\n{err}"
    );
    let _ = code; // exit 1 by the severity contract (E4002 is an ERROR)
    assert!(out.contains("g12=22"), "OOB write must not alias:\n{out}");
}

#[test]
fn outer_dim_over_index_reads_x() {
    // Was already X via the flat check — pinned so the guard keeps it.
    let (out, err, code) = run("module top;\n\
         reg [7:0] g [0:1][0:2];\n\
         integer i;\n\
         initial begin\n\
           g[0][0]=8'h10;\n\
           i = 9;\n\
           $display(\"r=%h\", g[i][0]);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert!(
        err.contains("VITA-E4002"),
        "OOB must emit the runtime range diagnostic:\n{err}"
    );
    let _ = code; // exit 1 by the severity contract (E4002 is an ERROR)
    assert!(out.contains("r=xx"), "got:\n{out}");
}

#[test]
fn under_index_below_nonzero_lo_reads_x() {
    let (out, err, code) = run("module top;\n\
         reg [7:0] m [1:2][4:7];\n\
         integer i;\n\
         initial begin\n\
           m[1][4]=8'haa; m[2][7]=8'hbb;\n\
           i = 3;\n\
           $display(\"a=%h\", m[1][i]);\n\
           i = 0;\n\
           $display(\"b=%h\", m[i][4]);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert!(
        err.contains("VITA-E4002"),
        "OOB must emit the runtime range diagnostic:\n{err}"
    );
    let _ = code; // exit 1 by the severity contract (E4002 is an ERROR)
    assert!(out.contains("a=xx"), "got:\n{out}");
    assert!(out.contains("b=xx"), "got:\n{out}");
}

#[test]
fn nonzero_lo_in_range_access_still_works() {
    // Regression: guards must not break lo-normalized valid accesses.
    let (out, err, code) = run("module top;\n\
         reg [7:0] m [1:2][4:7];\n\
         integer i, j;\n\
         initial begin\n\
           for (i=1;i<=2;i=i+1) for (j=4;j<=7;j=j+1) m[i][j] = i*16 + j;\n\
           $display(\"m14=%h m27=%h m25=%h\", m[1][4], m[2][7], m[2][5]);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("m14=14 m27=27 m25=25"), "got:\n{out}");
}

#[test]
fn negative_index_reads_x() {
    let (out, err, code) = run("module top;\n\
         reg [7:0] g [0:1][0:2];\n\
         integer i;\n\
         initial begin\n\
           g[0][0]=8'h10;\n\
           i = -1;\n\
           $display(\"r=%h\", g[0][i]);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert!(
        err.contains("VITA-E4002"),
        "OOB must emit the runtime range diagnostic:\n{err}"
    );
    let _ = code; // exit 1 by the severity contract (E4002 is an ERROR)
    assert!(out.contains("r=xx"), "got:\n{out}");
}

#[test]
fn x_index_reads_x_and_write_noop() {
    // Pre-existing precise behavior — the guard's ternary X-merge keeps it.
    let (out, err, code) = run("module top;\n\
         reg [7:0] g [0:1][0:2];\n\
         reg [3:0] i;\n\
         initial begin\n\
           g[0][1]=8'h11;\n\
           $display(\"r=%h\", g[0][i]);\n\
           g[0][i] = 8'hee;\n\
           $display(\"g01=%h\", g[0][1]);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    let _ = (code, &err); // X-index: value contract only (diag policy pinned elsewhere)
    assert!(out.contains("r=xx"), "got:\n{out}");
    assert!(
        out.contains("g01=11"),
        "X-index write must stay a no-op:\n{out}"
    );
}

#[test]
fn three_d_middle_dim_over_index_reads_x() {
    let (out, err, code) = run("module top;\n\
         reg [3:0] c [0:1][0:1][0:2];\n\
         integer i;\n\
         initial begin\n\
           c[0][0][0]=4'h1; c[1][0][0]=4'h2;\n\
           i = 2;\n\
           $display(\"r=%h\", c[0][i][0]);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert!(
        err.contains("VITA-E4002"),
        "OOB must emit the runtime range diagnostic:\n{err}"
    );
    let _ = code; // exit 1 by the severity contract (E4002 is an ERROR)
    assert!(out.contains("r=x"), "got:\n{out}");
}

#[test]
fn one_d_over_index_unchanged() {
    // d == 1 keeps the guard-free lowering (flat check + wrap already exact).
    let (out, err, code) = run("module top;\n\
         reg [7:0] mem [0:3];\n\
         integer i;\n\
         initial begin\n\
           mem[0]=8'h10;\n\
           i = 7;\n\
           $display(\"r=%h\", mem[i]);\n\
           mem[i] = 8'hee;\n\
           $display(\"m0=%h\", mem[0]);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert!(
        err.contains("VITA-E4002"),
        "OOB must emit the runtime range diagnostic:\n{err}"
    );
    let _ = code; // exit 1 by the severity contract (E4002 is an ERROR)
    assert!(out.contains("r=xx"), "got:\n{out}");
    assert!(out.contains("m0=10"), "got:\n{out}");
}

#[test]
fn element_part_select_with_oob_word_is_x() {
    // `g[0][5][3:0]` — the part-select rides the guarded element word.
    let (out, err, code) = run("module top;\n\
         reg [7:0] g [0:1][0:2];\n\
         integer i;\n\
         initial begin\n\
           g[1][2]=8'h22;\n\
           i = 5;\n\
           $display(\"r=%h\", g[0][i][3:0]);\n\
           g[0][i][3:0] = 4'hf;\n\
           $display(\"g12=%h\", g[1][2]);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert!(
        err.contains("VITA-E4002"),
        "OOB must emit the runtime range diagnostic:\n{err}"
    );
    let _ = code; // exit 1 by the severity contract (E4002 is an ERROR)
    assert!(out.contains("r=x"), "got:\n{out}");
    assert!(
        out.contains("g12=22"),
        "OOB part-select write must not alias:\n{out}"
    );
}

#[test]
fn packed_inner_dim_over_index_is_x_and_write_noop() {
    // Packed multi-dim shares flatten_word, so its BIT space gets the same
    // per-dim guard (`m[0][9]` used to alias bit 1 of m[1]).
    let (out, err, code) = run("module top;\n\
         logic [1:0][7:0] m;\n\
         integer i;\n\
         initial begin\n\
           m[0]=8'haa; m[1]=8'h00;\n\
           i = 9;\n\
           $display(\"r=%b\", m[0][i]);\n\
           m[0][i] = 1'b1;\n\
           $display(\"m1=%h\", m[1]);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    // BIT-space OOB is silently X by the long-standing part-select contract
    // (a plain `v[9]` on `[7:0]` is silent too) — no E4002 here, values only.
    let (_, _) = (code, err);
    assert!(out.contains("r=x"), "packed OOB bit read must be X:\n{out}");
    assert!(
        out.contains("m1=00"),
        "packed OOB bit write must not alias:\n{out}"
    );
}

#[test]
fn array_assign_slice_with_oob_row_index_is_noop() {
    // Item-② composition: `g[i] = row` with i OOB — every element write
    // rides the guarded base word, so the whole copy is a no-op.
    let (out, err, code) = run("module top;\n\
         reg [7:0] g [0:1][0:1]; reg [7:0] row [0:1];\n\
         integer i;\n\
         initial begin\n\
           g[1][0]=8'h77; row[0]=8'hee; row[1]=8'hef;\n\
           i = 5;\n\
           g[i] = row;\n\
           $display(\"g10=%h\", g[1][0]);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert!(
        err.contains("VITA-E4002"),
        "OOB must emit the runtime range diagnostic:\n{err}"
    );
    let _ = code; // exit 1 by the severity contract (E4002 is an ERROR)
    assert!(
        out.contains("g10=77"),
        "OOB slice copy must not land:\n{out}"
    );
}
