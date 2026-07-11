//! A replication count MUST be a constant expression (IEEE §11.4.12.2). vita
//! used to lower a RUNTIME-variable count (`{n{4'hA}}` with a `logic`/`int` `n`)
//! to a net the engine folded to 0 → silent `000`, where iverilog loud-rejects
//! ("a reference to a variable is not allowed in a constant expression"). The
//! `count_reads_runtime_net` walker now makes a runtime-variable count LOUD
//! (correct-or-loud). A CONSTANT count (param / localparam / genvar / literal /
//! const-function) reads no net and keeps its existing lowering, byte-identical.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_ncr_{}_{n}", std::process::id()));
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
fn runtime_variable_count_is_loud() {
    // `{n{4'hA}}` with a runtime `int n` — iverilog rejects it as non-constant;
    // vita must be LOUD, not silently print `000`.
    let (out, code) = run("module m; int n = 3; logic [11:0] r;\n\
         initial begin r = {n{4'hA}}; $display(\"R=%h\", r); #1 $finish; end\n\
       endmodule\n");
    assert_ne!(code, Some(0), "runtime count must be loud; got:\n{out}");
    assert!(
        !out.contains("R=000"),
        "must not silently print 000:\n{out}"
    );
}

#[test]
fn runtime_arithmetic_count_is_loud() {
    // A runtime net inside an arithmetic count (`{n+1{…}}`) is still non-constant.
    let (out, code) = run("module m; int n = 2; logic [11:0] r;\n\
         initial begin r = {n + 1{4'hA}}; $display(\"R=%h\", r); #1 $finish; end\n\
       endmodule\n");
    assert_ne!(
        code,
        Some(0),
        "runtime arith count must be loud; got:\n{out}"
    );
}

#[test]
fn param_count_still_works() {
    // A `parameter` / `localparam` / arithmetic-of-param count is constant → the
    // existing lowering is kept (byte-identical to before).
    let (out, code) = run("module m;\n\
         parameter P = 3; localparam L = 2; logic [11:0] a, b;\n\
         initial begin a = {P{4'hA}}; b = {L+1{4'hA}};\n\
           $display(\"R=%h %h\", a, b); #1 $finish; end\n\
       endmodule\n");
    assert_eq!(code, Some(0), "param count must work; got:\n{out}");
    assert!(out.contains("R=aaa aaa"), "param count value; got:\n{out}");
}

#[test]
fn literal_and_genvar_count_still_work() {
    // A literal count and a genvar count (in a generate block) are constants.
    let (out, code) = run("module m; logic [11:0] r;\n\
         genvar g; generate for (g = 1; g < 2; g = g + 1) begin : blk\n\
           logic [7:0] q; initial begin q = {(g+1){2'b11}}; $display(\"G=%b\", q); end\n\
         end endgenerate\n\
         initial begin r = {3{4'hA}}; $display(\"R=%h\", r); #1 $finish; end\n\
       endmodule\n");
    assert_eq!(code, Some(0), "literal/genvar count must work; got:\n{out}");
    assert!(out.contains("R=aaa"), "literal count; got:\n{out}");
    assert!(out.contains("G=00001111"), "genvar count; got:\n{out}");
}

#[test]
fn type_query_sysfunc_count_not_over_rejected() {
    // The type/shape-query system functions ($bits, $size, …) are elaboration
    // CONSTANTS regardless of a net operand — `{$bits(net){…}}` is a constant
    // count and must NOT be flagged (adversarial-review over-reject). Includes
    // the idiomatic width-portable sign-extend. iverilog accepts all.
    let (out, code) = run("module m;\n\
         logic [7:0] data = 8'hff; logic [7:0] arr[0:3]; logic [7:0] a = 8'h80;\n\
         logic [15:0] r, s, w;\n\
         initial begin\n\
           r = {$bits(data){1'b1}};\n\
           s = {$size(arr){1'b1}};\n\
           w = { {$bits(w)-$bits(a){a[$bits(a)-1]}}, a };\n\
           $display(\"R=%h %h %h\", r, s, w); #1 $finish; end\n\
       endmodule\n");
    assert_eq!(code, Some(0), "type-query count must work; got:\n{out}");
    // $bits(data)=8 → {8{1}}=ff; $size(arr[0:3])=4 → {4{1}}=f; sign-extend → ff80.
    assert!(
        out.contains("R=00ff 000f ff80"),
        "type-query counts; got:\n{out}"
    );
}

#[test]
fn shadowing_inner_const_not_over_rejected() {
    // A generate-scope localparam `N` that SHADOWS an outer-scope net `N` is a
    // constant count — lower_expr resolves it to the localparam. The runtime-net
    // walker must mirror that param-first precedence, else it over-rejects
    // (adversarial-review find). iverilog: y = {3{1'b1}} = 8'h07.
    let (out, code) = run("module m;\n\
         logic [3:0] N; logic [7:0] y;\n\
         genvar g; generate for (g = 0; g < 1; g = g + 1) begin : blk\n\
           localparam int N = 3; assign y = {N{1'b1}};\n\
         end endgenerate\n\
         initial begin #1 $display(\"R=%h\", y); $finish; end\n\
       endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "shadowing const count must work; got:\n{out}"
    );
    assert!(
        out.contains("R=07"),
        "shadowing localparam count; got:\n{out}"
    );
}

#[test]
fn const_function_count_not_over_rejected() {
    // A const-function call with CONSTANT args (`{f(3){…}}`) is a constant count
    // the engine folds — it reads no net, so the runtime-net guard must NOT flag
    // it (the key no-over-reject case). iverilog accepts it.
    let (out, code) = run("module m;\n\
         function integer f(input integer x); f = x; endfunction\n\
         logic [11:0] r;\n\
         initial begin r = {f(3){4'hA}}; $display(\"R=%h\", r); #1 $finish; end\n\
       endmodule\n");
    assert_eq!(code, Some(0), "const-func count must work; got:\n{out}");
    assert!(out.contains("R=aaa"), "const-func count; got:\n{out}");
}
