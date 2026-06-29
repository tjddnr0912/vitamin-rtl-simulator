//! HIER-REST: a hierarchical PARAMETER read (`dut.WIDTH`) in a RUNTIME/expression
//! context folds to the referenced instance's parameter value. Parameters are
//! restored out of `self.params` after each instance, so a persistent `hier_params`
//! table (populated as each instance binds its header params / body localparams)
//! backs the deferred read; the placeholder Signal is patched to a Const.
//! Pure IR-0 (elaborate-only).
//!
//! Every expected value is pinned to LIVE iverilog 13.0 (which supports hierarchical
//! param reads in runtime expressions). A hierarchical param in a CONSTANT context
//! (`reg [dut.W-1:0]`) is NOT covered here — iverilog rejects it ("not allowed in a
//! constant expression"), and vita folds it via the pre-existing non-const-width
//! path (a separate, general issue), so it is out of this slice's scope.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_hp_{}_{n}", std::process::id()));
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
fn read_overridden_header_param() {
    // dut overrides WIDTH=12; dut.WIDTH reads 12 (not the default 8).
    let (out, _c) = run(
        "module sub #(parameter WIDTH=8) (); reg [7:0] r; endmodule\n\
         module top; sub #(.WIDTH(12)) dut();\n\
           initial $display(\"R %0d\", dut.WIDTH);\n\
         endmodule\n",
    );
    assert!(out.contains("R 12"), "overridden header param:\n{out}");
}

#[test]
fn read_default_header_param() {
    let (out, _c) = run("module sub #(parameter DEPTH=16) (); reg r; endmodule\n\
         module top; sub dut();\n\
           initial $display(\"R %0d\", dut.DEPTH);\n\
         endmodule\n");
    assert!(out.contains("R 16"), "default header param:\n{out}");
}

#[test]
fn read_param_only_child() {
    // a child with NO nets still registers as a scope via hier_params.
    let (out, _c) = run("module sub #(parameter WIDTH=8) (); endmodule\n\
         module top; sub #(.WIDTH(20)) dut();\n\
           initial $display(\"R %0d\", dut.WIDTH);\n\
         endmodule\n");
    assert!(out.contains("R 20"), "param-only child scope:\n{out}");
}

#[test]
fn read_body_localparam() {
    // a body `localparam` (not a header param) is also hierarchically readable.
    let (out, _c) = run("module sub; localparam LP = 42; reg r; endmodule\n\
         module top; sub dut();\n\
           initial $display(\"R %0d\", dut.LP);\n\
         endmodule\n");
    assert!(out.contains("R 42"), "body localparam:\n{out}");
}

#[test]
fn localparam_derived_from_header_param() {
    // LP2 = W*2; dut overrides W=10 ⇒ dut.W=10, dut.LP2=20.
    let (out, _c) = run(
        "module sub #(parameter W=8) (); localparam LP2 = W*2; reg r; endmodule\n\
         module top; sub #(.W(10)) dut();\n\
           initial $display(\"R %0d %0d\", dut.W, dut.LP2);\n\
         endmodule\n",
    );
    assert!(
        out.contains("R 10 20"),
        "localparam from header param:\n{out}"
    );
}

#[test]
fn two_instances_distinct_param_values() {
    let (out, _c) = run("module sub #(parameter K=1) (); reg r; endmodule\n\
         module top; sub #(.K(7)) a(); sub #(.K(9)) b();\n\
           initial $display(\"R %0d %0d\", a.K, b.K);\n\
         endmodule\n");
    assert!(
        out.contains("R 7 9"),
        "two instances distinct params:\n{out}"
    );
}

#[test]
fn three_level_param_read() {
    let (out, _c) = run("module leaf #(parameter K=3) (); reg r; endmodule\n\
         module mid; leaf #(.K(55)) l(); endmodule\n\
         module top; mid m();\n\
           initial $display(\"R %0d\", m.l.K);\n\
         endmodule\n");
    assert!(out.contains("R 55"), "3-level param read:\n{out}");
}

#[test]
fn param_in_runtime_arithmetic() {
    // dut.N used in a runtime expression: N=10 ⇒ x = 10 + 5 = 15.
    let (out, _c) = run("module sub #(parameter N=4) (); reg r; endmodule\n\
         module top; sub #(.N(10)) dut(); reg [7:0] x;\n\
           initial begin x = dut.N + 5; $display(\"R %0d\", x); end\n\
         endmodule\n");
    assert!(out.contains("R 15"), "param in runtime arithmetic:\n{out}");
}

#[test]
fn negative_param_value() {
    // a negative parameter folds to a signed const: dut.SN = -5.
    let (out, _c) = run("module sub #(parameter SN=-5) (); reg r; endmodule\n\
         module top; sub dut(); integer z;\n\
           initial begin z = dut.SN; $display(\"R %0d\", z); end\n\
         endmodule\n");
    assert!(out.contains("R -5"), "negative param value:\n{out}");
}

#[test]
fn unresolved_hier_param_is_loud() {
    // a name that is neither a net nor a param is loud (not a silent 0).
    let (out, code) = run("module sub #(parameter W=8) (); reg r; endmodule\n\
         module top; sub dut();\n\
           initial $display(\"%0d\", dut.NOPE);\n\
         endmodule\n");
    assert!(
        out.contains("VITA-E") || code == Some(1),
        "unresolved hierarchical name must be loud: {out} {code:?}"
    );
}
