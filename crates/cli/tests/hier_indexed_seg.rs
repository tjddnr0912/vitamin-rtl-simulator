//! HIER-REST②: an INDEXED-SEGMENT hierarchical reference — a generate / instance
//! array element in a hierarchical path (`g[0].x`, `bank[3].c.r`, `g[0].u.r`).
//! Previously a parse error (`expected ')', found Dot`). The parser now folds the
//! CONSTANT generate index into the scope-segment name (`g` + `[0]` => `g[0]`),
//! which is exactly how generate-for scopes are keyed, so the existing
//! hierarchical resolver handles read AND write with NO new AST/IR (the segment is
//! a plain string). Pure IR-0 (parser-only; the AST shape is unchanged).
//!
//! Same-module generate ARRAY / multi-dim-PACKED element selects (`g[0].mem[i]`,
//! `g[0].pm[k]`) resolve correctly — the array/packed chain detectors accept a
//! multi-segment resolvable generate-scope net (this also closed a silent-wrong
//! where `g[0].pm[k]` flat-bit-selected). The one remaining sub-limitation is a
//! NON-literal index (`g[P].x`), which stays LOUD (never silent).
//!
//! Values pinned to LIVE iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_his_{}_{n}", std::process::id()));
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

fn is_loud(out: &str, code: Option<i32>) -> bool {
    out.contains("VITA-E") || code == Some(1)
}

#[test]
fn read_generate_scalar() {
    let (out, _c) = run("module t; genvar i;\n\
         generate for(i=0;i<2;i=i+1) begin:g reg [7:0] x; initial x=8'h10+i; end endgenerate\n\
         initial #1 $display(\"R %h\", g[0].x);\n\
         endmodule\n");
    assert!(out.contains("R 10"), "g[0].x read:\n{out}");
}

#[test]
fn read_generate_scalar_distinct_index() {
    let (out, _c) = run("module t; genvar i;\n\
         generate for(i=0;i<2;i=i+1) begin:g reg [7:0] x; initial x=8'h10+i; end endgenerate\n\
         initial #1 $display(\"R %h\", g[1].x);\n\
         endmodule\n");
    assert!(out.contains("R 11"), "g[1].x distinct index read:\n{out}");
}

#[test]
fn read_generate_instance_net() {
    // bank[1].c.r — generate-block array, then an instance, then its net (3 levels).
    let (out, _c) = run("module unit; reg [7:0] r; initial r=8'hde; endmodule\n\
         module t; genvar gi;\n\
         generate for(gi=0;gi<2;gi=gi+1) begin:bank unit c(); end endgenerate\n\
         initial #1 $display(\"R %h\", bank[1].c.r);\n\
         endmodule\n");
    assert!(out.contains("R de"), "bank[1].c.r read:\n{out}");
}

#[test]
fn read_nested_block_instance_net() {
    let (out, _c) = run("module unit; reg [7:0] r; initial r=8'hcc; endmodule\n\
         module t; genvar i;\n\
         generate for(i=0;i<2;i=i+1) begin:g unit u(); end endgenerate\n\
         initial #1 $display(\"R %h\", g[0].u.r);\n\
         endmodule\n");
    assert!(out.contains("R cc"), "g[0].u.r nested read:\n{out}");
}

#[test]
fn write_generate_scalar() {
    let (out, _c) = run("module t; genvar i;\n\
         generate for(i=0;i<2;i=i+1) begin:g reg [7:0] x; end endgenerate\n\
         initial begin g[0].x=8'haa; #1 $display(\"R %h\", g[0].x); end\n\
         endmodule\n");
    assert!(out.contains("R aa"), "g[0].x write:\n{out}");
}

#[test]
fn write_generate_distinct() {
    let (out, _c) = run("module t; genvar i;\n\
         generate for(i=0;i<2;i=i+1) begin:g reg [7:0] x; end endgenerate\n\
         initial begin g[0].x=8'h11; g[1].x=8'h22; #1 $display(\"R %h %h\", g[0].x, g[1].x); end\n\
         endmodule\n");
    assert!(out.contains("R 11 22"), "distinct generate writes:\n{out}");
}

#[test]
fn write_generate_part_select() {
    // composes with HIER-REST-PS: `g[0].x[3:0] = …`.
    let (out, _c) = run("module t; genvar i;\n\
         generate for(i=0;i<2;i=i+1) begin:g reg [7:0] x; end endgenerate\n\
         initial begin g[0].x=8'h00; g[0].x[3:0]=4'hc; #1 $display(\"R %h\", g[0].x); end\n\
         endmodule\n");
    assert!(
        out.contains("R 0c"),
        "g[0].x[3:0] part-select write:\n{out}"
    );
}

#[test]
fn read_generate_scalar_in_continuous_assign() {
    let (out, _c) = run("module t; genvar i;\n\
         generate for(i=0;i<2;i=i+1) begin:g reg [7:0] x; initial x=8'h3a; end endgenerate\n\
         wire [7:0] w = g[1].x;\n\
         initial #1 $display(\"R %h\", w);\n\
         endmodule\n");
    assert!(out.contains("R 3a"), "g[1].x in continuous assign:\n{out}");
}

#[test]
fn read_generate_scalar_bit_select() {
    // a bit-select on the resolved hierarchical scalar: g[0].x[4].
    let (out, _c) = run("module t; genvar i;\n\
         generate for(i=0;i<2;i=i+1) begin:g reg [7:0] x; initial x=8'h10; end endgenerate\n\
         initial #1 $display(\"R %h\", g[0].x[4]);\n\
         endmodule\n");
    assert!(out.contains("R 1"), "g[0].x[4] bit-select:\n{out}");
}

#[test]
fn non_literal_generate_index_is_loud() {
    // `g[P].x` (a param index) is a documented sub-limitation — loud, NOT silent.
    let (out, code) = run("module t; parameter P=0; genvar i;\n\
         generate for(i=0;i<2;i=i+1) begin:g reg [7:0] x; initial x=8'h11; end endgenerate\n\
         initial #1 $display(\"R %h\", g[P].x);\n\
         endmodule\n");
    assert!(
        is_loud(&out, code),
        "non-literal generate index must be loud: {out} {code:?}"
    );
}

#[test]
fn same_module_generate_array_element_read() {
    // `g[0].mem[i]` (a same-module generate-block ARRAY element) reads the element —
    // the array/packed chain detectors resolve the multi-segment generate-scope net.
    let (out, _c) = run("module t; genvar i;\n\
         generate for(i=0;i<2;i=i+1) begin:g reg [7:0] mem [0:3]; initial mem[1]=8'h7e; end endgenerate\n\
         initial #1 $display(\"R %h\", g[0].mem[1]);\n\
         endmodule\n");
    assert!(out.contains("R 7e"), "g[0].mem[1] element read:\n{out}");
}

#[test]
fn same_module_generate_packed_element_read() {
    // `g[0].pm[k]` (a same-module generate-block multi-dim PACKED element) reads the
    // packed sub-vector, NOT a flat bit (the silent-wrong the chain generalization
    // closed). pm=cafe ⇒ [1]=ca, [0]=fe.
    let (out, _c) = run("module t; genvar gi;\n\
         generate for(gi=0;gi<1;gi=gi+1) begin:g reg [1:0][7:0] pm; end endgenerate\n\
         initial begin g[0].pm=16'hcafe; #1 $display(\"R %h %h\", g[0].pm[1], g[0].pm[0]); end\n\
         endmodule\n");
    assert!(
        out.contains("R ca fe"),
        "g[0].pm[k] packed element read:\n{out}"
    );
}

#[test]
fn same_module_generate_packed_element_write() {
    let (out, _c) = run("module t; genvar gi;\n\
         generate for(gi=0;gi<1;gi=gi+1) begin:g reg [1:0][7:0] pm; end endgenerate\n\
         initial begin g[0].pm=16'h0; g[0].pm[1]=8'haa; g[0].pm[0]=8'hbb; #1 $display(\"R %h\", g[0].pm); end\n\
         endmodule\n");
    assert!(
        out.contains("R aabb"),
        "g[0].pm[k] packed element write:\n{out}"
    );
}

#[test]
fn leading_zero_generate_index() {
    // `g[00].x` normalizes to scope `g[0]` (the value, not the spelling).
    let (out, _c) = run("module t; genvar gi;\n\
         generate for(gi=0;gi<2;gi=gi+1) begin:g reg [7:0] x; initial x=gi+8'hf0; end endgenerate\n\
         initial #1 $display(\"R %h\", g[00].x);\n\
         endmodule\n");
    assert!(out.contains("R f0"), "leading-zero generate index:\n{out}");
}
