//! N3.4: a `[msb:lsb]` part-select on a bare multi-dimensional PACKED net
//! (e.g. `reg [3:0][7:0] x; x[3:2]`) addresses the OUTER packed dimension —
//! it selects whole sub-elements, NOT flat bits. Before this fix the generic
//! flat-bit path silently read/wrote `(msb-lsb+1)` raw bits of element 0
//! (`x[3:2]` => `3`, a write was a no-op). Pure IR-0 (elaborate lowering only).
//!
//! Every expected value is pinned to LIVE iverilog 13.0. The ascending `[lo:hi]`
//! form on an ascending net (`reg [0:3][7:0]; x[0:1]`) selects whole outer
//! elements too (`x[0:1]` = `aabb`, elem 0 is the MSB). A direction-MISMATCHED
//! ("out of order") select is a loud E3009, matching iverilog — NOT a silent value.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_n34_{}_{n}", std::process::id()));
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
        out.status.code(),
    )
}

#[test]
fn read_selects_whole_outer_elements() {
    // reg [3:0][7:0] x = {aa,bb,cc,dd}: x[3:2] = {aa,bb} = aabb (16 bits),
    // x[1:0] = ccdd, x[2:1] = bbcc; a single x[2] still = bb (N3.2).
    let (out, _c) = run("module t;\n\
         reg [3:0][7:0] x;\n\
         initial begin\n\
           x[3]=8'haa; x[2]=8'hbb; x[1]=8'hcc; x[0]=8'hdd;\n\
           $display(\"E2 %h\", x[2]);\n\
           $display(\"PS32 %h\", x[3:2]);\n\
           $display(\"PS10 %h\", x[1:0]);\n\
           $display(\"PS21 %h\", x[2:1]);\n\
         end\n\
         endmodule\n");
    assert!(out.contains("E2 bb"), "single element:\n{out}");
    assert!(out.contains("PS32 aabb"), "x[3:2] = elements 3,2:\n{out}");
    assert!(out.contains("PS10 ccdd"), "{out}");
    assert!(out.contains("PS21 bbcc"), "{out}");
}

#[test]
fn write_targets_whole_outer_elements() {
    // x[3:2] = 16'h1234 writes elements 3,2 (=> 12340000), not 2 flat bits.
    let (out, _c) = run("module t;\n\
         reg [3:0][7:0] x;\n\
         initial begin\n\
           x=32'h0; x[3:2]=16'h1234; $display(\"W1 %h\", x);\n\
           x=32'h0; x[1:0]=16'h5678; $display(\"W2 %h\", x);\n\
           x=32'h0; x[2:1]=16'h9abc; $display(\"W3 %h\", x);\n\
         end\n\
         endmodule\n");
    assert!(
        out.contains("W1 12340000"),
        "x[3:2]=… writes elements 3,2:\n{out}"
    );
    assert!(out.contains("W2 00005678"), "{out}");
    assert!(out.contains("W3 009abc00"), "{out}");
}

#[test]
fn three_dim_packed_part_select() {
    // reg [1:0][1:0][7:0] g: each outer element is 16 bits. g[1:0] = whole,
    // g[1] = the first 16-bit element.
    let (out, _c) = run("module t;\n\
         reg [1:0][1:0][7:0] g;\n\
         initial begin\n\
           g[1][1]=8'h11; g[1][0]=8'h22; g[0][1]=8'h33; g[0][0]=8'h44;\n\
           $display(\"G3PS %h\", g[1:0]);\n\
           $display(\"G3E %h\", g[1]);\n\
         end\n\
         endmodule\n");
    assert!(out.contains("G3PS 11223344"), "whole 3-dim packed:\n{out}");
    assert!(
        out.contains("G3E 1122"),
        "outer element 1 (16 bits):\n{out}"
    );
}

#[test]
fn plain_vector_part_select_is_flat_bits() {
    // a part-select on a PLAIN (single-dim) vector is unchanged: v[3:0] reads
    // the low nibble (flat bits), NOT an "element".
    let (out, _c) = run("module t;\n\
         reg [7:0] v;\n\
         initial begin\n\
           v=8'h5a; $display(\"VPS %h\", v[3:0]);\n\
         end\n\
         endmodule\n");
    assert!(
        out.contains("VPS a"),
        "plain vector part-select stays flat:\n{out}"
    );
}

#[test]
fn out_of_range_part_select_is_loud() {
    // x[4:2] on reg [3:0][7:0] (msb=4 > outer hi=3): iverilog rejects at compile
    // ("exceeds the declared bounds"); vita must be loud (E3009), NOT silently
    // read/write past the net. Both read and write.
    let (rd, rc) = run("module t;\n\
         reg [3:0][7:0] x;\n\
         initial begin x[3]=8'haa; $display(\"ROOB %h\", x[4:2]); end\n\
         endmodule\n");
    assert!(
        rd.contains("VITA-E3009") || rc == Some(1),
        "upper-OOB read must be loud: {rd} code={rc:?}"
    );
    let (_w, wc) = run("module t;\n\
         reg [3:0][7:0] x;\n\
         initial begin x=0; x[4:2]=24'h112233; end\n\
         endmodule\n");
    assert_eq!(wc, Some(1), "upper-OOB write must be loud (E3009)");
}

#[test]
fn non_zero_base_outer_dim() {
    // reg [5:2][7:0] x: outer indices 5..2. x[5:4] = elements 5,4; x[3:2] = 3,2;
    // x[6:2] is out of range (loud).
    let (out, _c) = run("module t;\n\
         reg [5:2][7:0] x;\n\
         initial begin\n\
           x[5]=8'h11; x[4]=8'h22; x[3]=8'h33; x[2]=8'h44;\n\
           $display(\"NZ54 %h\", x[5:4]);\n\
           $display(\"NZ32 %h\", x[3:2]);\n\
         end\n\
         endmodule\n");
    assert!(
        out.contains("NZ54 1122"),
        "non-zero-base elements 5,4:\n{out}"
    );
    assert!(out.contains("NZ32 3344"), "{out}");
    let (_o, c) = run("module t;\n\
         reg [5:2][7:0] x;\n\
         initial begin $display(\"%h\", x[6:2]); end\n\
         endmodule\n");
    assert_eq!(c, Some(1), "x[6:2] beyond [5:2] must be loud");
}

#[test]
fn ascending_read_selects_whole_outer_elements() {
    // reg [0:3][7:0] y = {aa,bb,cc,dd}: the ascending outer dim makes the LEFT
    // index the MSB element, so y[0:1] = {y[0],y[1]} = aabb, y[1:2] = bbcc,
    // y[0:3] = aabbccdd, y[0:0] = aa. (LIVE iverilog 13.0.)
    let (out, _c) = run("module t;\n\
         reg [0:3][7:0] y;\n\
         initial begin\n\
           y[0]=8'haa; y[1]=8'hbb; y[2]=8'hcc; y[3]=8'hdd;\n\
           $display(\"A01 %h\", y[0:1]);\n\
           $display(\"A12 %h\", y[1:2]);\n\
           $display(\"A03 %h\", y[0:3]);\n\
           $display(\"A00 %h\", y[0:0]);\n\
         end\n\
         endmodule\n");
    assert!(out.contains("A01 aabb"), "y[0:1] = elems 0,1:\n{out}");
    assert!(out.contains("A12 bbcc"), "y[1:2] = elems 1,2:\n{out}");
    assert!(out.contains("A03 aabbccdd"), "y[0:3] = whole:\n{out}");
    assert!(out.contains("A00 aa"), "y[0:0] = elem 0:\n{out}");
}

#[test]
fn ascending_write_targets_whole_outer_elements() {
    // y[0:1] = 16'h1234 writes elems 0,1 (=> y[0]=12, y[1]=34), not 2 flat bits.
    let (out, _c) = run("module t;\n\
         reg [0:3][7:0] y;\n\
         initial begin\n\
           y[0]=0;y[1]=0;y[2]=0;y[3]=0;\n\
           y[0:1]=16'h1234;\n\
           $display(\"AW %h %h %h %h\", y[0], y[1], y[2], y[3]);\n\
         end\n\
         endmodule\n");
    assert!(
        out.contains("AW 12 34 00 00"),
        "ascending write elems 0,1:\n{out}"
    );
}

#[test]
fn ascending_indexed_part_select() {
    // indexed part-selects are direction-agnostic: y[0+:2] = elems {0,1} = aabb,
    // y[3-:2] = elems {2,3} = ccdd (LIVE iverilog 13.0).
    let (out, _c) = run("module t;\n\
         reg [0:3][7:0] y;\n\
         initial begin\n\
           y[0]=8'haa; y[1]=8'hbb; y[2]=8'hcc; y[3]=8'hdd;\n\
           $display(\"IP %h\", y[0+:2]);\n\
           $display(\"IM %h\", y[3-:2]);\n\
         end\n\
         endmodule\n");
    assert!(out.contains("IP aabb"), "y[0+:2] = elems 0,1:\n{out}");
    assert!(out.contains("IM ccdd"), "y[3-:2] = elems 2,3:\n{out}");
}

#[test]
fn ascending_out_of_range_part_select_is_loud() {
    // y[2:4] on reg [0:3][7:0] (hi=4 > outer hi=3): iverilog rejects at compile;
    // vita must be loud (E3009), NOT a silent read past the net.
    let (out, code) = run("module t;\n\
         reg [0:3][7:0] y;\n\
         initial begin\n\
           y[0]=8'h11; $display(\"%h\", y[2:4]);\n\
         end\n\
         endmodule\n");
    assert!(
        out.contains("VITA-E3009") || code == Some(1),
        "ascending over-range y[2:4] must be loud: {out} code={code:?}"
    );
}

#[test]
fn reversed_part_select_is_out_of_order_loud() {
    // A descending select on an ascending net (y[1:0] on reg [0:3]) is "out of
    // order" — iverilog: "part select ... is out of order." vita must be loud,
    // NOT a silent value. Likewise an ascending select on a descending net.
    let (a, ac) = run("module t;\n\
         reg [0:3][7:0] y;\n\
         initial begin y[0]=8'h11; $display(\"%h\", y[1:0]); end\n\
         endmodule\n");
    assert!(
        a.contains("VITA-E3009") || ac == Some(1),
        "descending select on ascending net must be loud: {a} code={ac:?}"
    );
    let (b, bc) = run("module t;\n\
         reg [3:0][7:0] x;\n\
         initial begin x[0]=8'h11; $display(\"%h\", x[0:1]); end\n\
         endmodule\n");
    assert!(
        b.contains("VITA-E3009") || bc == Some(1),
        "ascending select on descending net must be loud: {b} code={bc:?}"
    );
}
