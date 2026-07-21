//! §4.5.190 UARR: `arr[i].field` on a 1-D unpacked array of a PACKED struct
//! (`struct_1d_array_vars`) — a member access is a part-select on the packed
//! element value `arr[i][off+w-1 : off]`, reusing the scalar `s.field` machinery
//! (whole-field sign wrap, trailing sub-select, RMW field write). Previously E3010
//! (the element-member desugar only covered dynamic RECORD arrays / SoA, and the
//! scalar path only a whole-variable base).
//!
//! iverilog 13.0 has no usable oracle here (it aborts / rejects packed-struct array
//! field access outright), so field OFFSETS are cross-checked against the equivalent
//! plain packed-vector part-selects (which iverilog does pin), plus vita↔vita
//! self-consistency (`arr[i].field` == the manual `arr[i][hi:lo]`).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_sam_{}_{n}", std::process::id()));
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

const PR: &str = "typedef struct packed { logic [7:0] a; logic [7:0] b; } pr;\n";

#[test]
fn read_const_index() {
    let o = run(&format!(
        "module top;\n{PR}pr arr[3];\n\
         initial begin\n\
           arr[0]=16'h1122; arr[1]=16'h3344; arr[2]=16'h5566;\n\
           $display(\"%h %h %h %h\", arr[0].a, arr[0].b, arr[1].a, arr[2].b);\n\
         end\nendmodule\n"
    ));
    assert!(o.contains("11 22 33 66"), "got:\n{o}");
}

#[test]
fn read_runtime_index() {
    let o = run(&format!(
        "module top;\n{PR}pr arr[4]; int k;\n\
         initial begin arr[2]=16'hAABB; k=2; $display(\"%h %h\", arr[k].a, arr[k].b); end\n\
         endmodule\n"
    ));
    assert!(o.contains("aa bb"), "got:\n{o}");
}

#[test]
fn read_matches_manual_part_select() {
    // Field access is self-consistent with the manual part-select on the element.
    let o = run(&format!(
        "module top;\n{PR}pr arr[2];\n\
         initial begin arr[0]=16'h1234;\n\
           $display(\"a=%h ps=%h\", arr[0].a, arr[0][15:8]);\n\
           $display(\"b=%h ps=%h\", arr[0].b, arr[0][7:0]);\n\
         end\nendmodule\n"
    ));
    assert!(o.contains("a=12 ps=12"), "got:\n{o}");
    assert!(o.contains("b=34 ps=34"), "got:\n{o}");
}

#[test]
fn field_offsets_match_iverilog_vector_layout() {
    // Declaration-order MSB-first: field a = element[15:8], b = element[7:0].
    // iverilog pins these part-selects on a plain `logic [15:0] arr[3]` as 11/22/33/44.
    let o = run(&format!(
        "module top;\n{PR}pr arr[3];\n\
         initial begin arr[0]=16'h1122; arr[1]=16'h3344;\n\
           $display(\"%h %h %h %h\", arr[0].a, arr[0].b, arr[1].a, arr[1].b);\n\
         end\nendmodule\n"
    ));
    assert!(o.contains("11 22 33 44"), "got:\n{o}");
}

#[test]
fn write_field_rmw_preserves_other() {
    // A field write is a read-modify-write on the element; the other field is kept.
    let o = run(&format!(
        "module top;\n{PR}pr arr[2]; int k;\n\
         initial begin arr[0]=16'hFFFF; k=0; arr[k].a=8'h11;\n\
           $display(\"whole=%h\", arr[0]);\n\
         end\nendmodule\n"
    ));
    assert!(o.contains("whole=11ff"), "got:\n{o}");
}

#[test]
fn write_matches_manual_part_select_write() {
    let o = run(&format!(
        "module top;\n{PR}pr x[1]; pr y[1];\n\
         initial begin x[0]=16'h0; y[0]=16'h0;\n\
           x[0].a=8'h5A;\n\
           y[0][15:8]=8'h5A;\n\
           $display(\"field=%h manual=%h\", x[0], y[0]);\n\
         end\nendmodule\n"
    ));
    assert!(o.contains("field=5a00 manual=5a00"), "got:\n{o}");
}

#[test]
fn signed_byte_field_sign_extends() {
    let o = run(
        "module top;\n\
         typedef struct packed { byte a; byte b; } pr;\n\
         pr arr[2];\n\
         initial begin arr[0].a=-5; arr[0].b=100; $display(\"a=%0d b=%0d\", arr[0].a, arr[0].b); end\n\
         endmodule\n",
    );
    assert!(o.contains("a=-5 b=100"), "got:\n{o}");
}

#[test]
fn three_field_offsets() {
    let o = run("module top;\n\
         typedef struct packed { logic [3:0] x; logic [7:0] y; logic [3:0] z; } t3;\n\
         t3 arr[2];\n\
         initial begin arr[0].x=4'hA; arr[0].y=8'hBC; arr[0].z=4'hD;\n\
           $display(\"%h %h %h w=%h\", arr[0].x, arr[0].y, arr[0].z, arr[0]);\n\
         end\nendmodule\n");
    assert!(o.contains("a bc d w=abcd"), "got:\n{o}");
}

#[test]
fn trailing_subselect_read() {
    let o = run("module top;\n\
         typedef struct packed { logic [15:0] a; logic [15:0] b; } pr;\n\
         pr arr[2];\n\
         initial begin arr[0].a=16'h1234;\n\
           $display(\"hi=%h lo=%h bit=%b\", arr[0].a[15:8], arr[0].a[7:0], arr[0].a[0]);\n\
         end\nendmodule\n");
    assert!(o.contains("hi=12 lo=34 bit=0"), "got:\n{o}");
}

#[test]
fn assign_pattern_element_still_works() {
    // Regression: `arr[i] = '{…}` (positional pattern on the element) is unaffected.
    let o = run(&format!(
        "module top;\n{PR}pr arr[2];\n\
         initial begin arr[0] = '{{8'h11, 8'h22}}; $display(\"%h %h\", arr[0].a, arr[0].b); end\n\
         endmodule\n"
    ));
    assert!(o.contains("11 22"), "got:\n{o}");
}
