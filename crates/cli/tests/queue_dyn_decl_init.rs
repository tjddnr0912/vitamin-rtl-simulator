//! A queue / dynamic-array declaration initializer with an assignment pattern —
//! `int q[$] = '{1,2,3};` or `int d[] = '{5,6,7};` — was loud-rejected ("a
//! dynamic-storage handle takes no initializer"): a dynamic-storage handle has no
//! whole-value init surface, and the `'{…}` pattern was only handled for fixed
//! unpacked arrays.
//!
//! Fixed by EXPANDING the `'{…}` decl-init to runtime ops at the var-init flush,
//! reusing the existing mechanisms — no new engine path: a QUEUE pushes each
//! element (`q.push_back(e)`); a DYNAMIC ARRAY allocates `d = new[N]` then writes
//! each element (`d[i] = e`). The expansion happens in the single, declaration-
//! ordered `pending_var_inits` list, so a later scalar init reads an earlier
//! queue's `.size()` correctly. Works in the module body and block-local scopes.
//! Pinned to iverilog 13.0. Runtime queue/dyn-array ops stay byte-identical.
//!
//! Honest-loud (separate): whole-value runtime assignment (`q = '{…}`), queue copy
//! (`r = q`), an assoc-array positional pattern, and a queue/dyn `'{…}` init in a
//! GENERATE or INTERFACE body (whose init pass does not run the flush, so the init
//! is loud-rejected rather than silently dropped).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_qdi_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let first = String::from_utf8_lossy(&out.stdout)
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .to_owned();
    (first, out.status.success())
}

#[test]
fn queue_decl_init() {
    let (o, ok) = run("module top; int q[$]='{1,2,3};\n\
         initial begin $display(\"%0d %0d %0d %0d\", q[0],q[1],q[2],q.size()); #1 $finish; end endmodule");
    assert!(ok && o == "1 2 3 3", "got:\n{o}");
}

#[test]
fn dynarray_decl_init() {
    let (o, ok) = run("module top; int d[]='{5,6,7};\n\
         initial begin $display(\"%0d %0d %0d\", d[0],d[1],d.size()); #1 $finish; end endmodule");
    assert!(ok && o == "5 6 3", "got:\n{o}");
}

#[test]
fn empty_queue_init() {
    let (o, ok) = run("module top; int q[$]='{};\n\
         initial begin $display(\"%0d\", q.size()); #1 $finish; end endmodule");
    assert!(ok && o == "0", "got:\n{o}");
}

#[test]
fn decl_init_then_runtime_push() {
    // The synthesized t0 init coexists with subsequent runtime methods.
    let (o, ok) = run("module top; int q[$]='{10,20};\n\
         initial begin q.push_back(30); $display(\"%0d %0d\", q[2], q.size()); #1 $finish; end endmodule");
    assert!(ok && o == "30 3", "got:\n{o}");
}

#[test]
fn dynarray_init_foreach() {
    let (o, ok) = run("module top; int d[]='{4,5,6}; int s=0;\n\
         initial begin foreach(d[i]) s=s+d[i]; $display(\"%0d\",s); #1 $finish; end endmodule");
    assert!(ok && o == "15", "got:\n{o}");
}

#[test]
fn typed_and_expr_elements() {
    // logic[7:0] queue holds the declared width; a byte queue carries signedness.
    let (ol, okl) = run("module top; logic[7:0] q[$]='{8'hAB, 8'hCD};\n\
         initial begin $display(\"%h %h\", q[0], q[1]); #1 $finish; end endmodule");
    assert!(okl && ol == "ab cd", "logic got:\n{ol}");
    let (ob, okb) = run("module top; byte q[$]='{8'hFF, 8'h01};\n\
         initial begin $display(\"%0d %0d\", q[0], q[1]); #1 $finish; end endmodule");
    assert!(okb && ob == "-1 1", "byte got:\n{ob}");
    // Element expressions are evaluated at t0.
    let (oe, oke) = run("module top; int x=5; int q[$]='{x, x+1};\n\
         initial begin $display(\"%0d %0d\", q[0], q[1]); #1 $finish; end endmodule");
    assert!(oke && oe == "5 6", "expr got:\n{oe}");
}

#[test]
fn runtime_dynamic_storage_unchanged() {
    // Byte-identity: a dynamic-storage decl WITHOUT an initializer + runtime ops is
    // unaffected.
    let (oq, okq) = run("module top; int q[$];\n\
         initial begin q.push_back(7); q.push_back(8); $display(\"%0d %0d\", q[0], q.size()); #1 $finish; end endmodule");
    assert!(okq && oq == "7 2", "queue got:\n{oq}");
    let (od, okd) = run("module top; int d[];\n\
         initial begin d=new[2]; d[0]=1; $display(\"%0d %0d\", d[0], d.size()); #1 $finish; end endmodule");
    assert!(okd && od == "1 2", "dyn got:\n{od}");
}

#[test]
fn declaration_order_is_preserved() {
    // A scalar init reading an EARLIER queue/dyn-array's `.size()` must see the
    // populated collection (the `'{…}` expansion stays in declaration order, not
    // moved after all scalars).
    let (oq, okq) = run("module top; int q[$]='{1,2,3}; int sz=q.size();\n\
         initial begin $display(\"%0d %0d\", sz, q[0]); #1 $finish; end endmodule");
    assert!(okq && oq == "3 1", "queue got:\n{oq}");
    let (od, okd) = run("module top; int d[]='{10,20,30}; int sz=d.size();\n\
         initial begin $display(\"%0d %0d\", sz, d[0]); #1 $finish; end endmodule");
    assert!(okd && od == "3 10", "dyn got:\n{od}");
}

#[test]
fn block_local_queue_decl_init() {
    // A block-local queue decl-init works (the block-local nets ride the same
    // declaration-ordered var-init flush).
    let (o, ok) = run("module top;\n\
         initial begin begin int q[$]='{7,8}; $display(\"%0d %0d\", q[0], q.size()); end #1 $finish; end endmodule");
    assert!(ok && o == "7 2", "got:\n{o}");
}

#[test]
fn generate_scope_decl_init_is_loud() {
    // A generate-body queue decl-init is NOT desugared (the generate init pass does
    // not run the var-init flush), so it is loud-rejected rather than silently
    // dropped (which would leave an empty queue — the converged review finding).
    let (_o, ok) = run("module top;\n\
         if (1) begin : g int q[$]='{10,20}; initial begin $display(\"%0d\", q.size()); #1 $finish; end end endmodule");
    assert!(
        !ok,
        "a generate-scope queue decl-init must be loud, not silently dropped"
    );
}

#[test]
fn whole_value_assignment_and_copy_stay_loud() {
    // Runtime whole-value assignment and an assoc positional pattern are NOT
    // part of this slice — they stay loud. (Queue COPY `r = q` graduated to a
    // supported §7.10 deep copy — see handle_copy.rs — so its old loud assert
    // flipped to a positive check.)
    let (_a, oka) = run("module top; int q[$];\n\
         initial begin q='{1,2}; #1 $finish; end endmodule");
    assert!(!oka, "runtime q='{{…}} must be loud");
    let (out_b, okb) = run(
        "module top; int q[$]; int r[$];\n\
         initial begin q.push_back(1); r=q; $display(\"c=%0d\", r.size()); #1 $finish; end endmodule",
    );
    assert!(okb, "queue copy r=q is now supported:\n{out_b}");
    assert!(out_b.contains("c=1"), "copy carries the element:\n{out_b}");
    let (_c, okc) = run("module top; int aa[string]='{1,2};\n\
         initial begin #1 $finish; end endmodule");
    assert!(!okc, "assoc positional pattern init must be loud");
}
