//! Cross-map scope shadowing: an inner-scope local must shadow an OUTER entry in a
//! side-map, not just an outer entry in the same map.
//!
//! `walk_scopes_key` is the one outward scope walk, and it is innermost-wins only
//! WITHIN the single map its `hit` closure probes. But a function/task/block/generate
//! local is a NET (in `symbols`), while several lookups walk a *different* keyspace
//! (`string_array_elems`, `array_const_vals`, `pkg_var_aliases`, `iface_insts`,
//! `genvar_decls`). Walking past an inner net binding therefore resolved an inner
//! local to an OUTER side-map entry — silently, and for non-string locals too.
//!
//! Live iverilog differential (all of these are iverilog-supported constructs):
//!
//! ```text
//!   function-local `string sa` vs module `string sa[2]`   vita 0    iverilog 1
//!   task-local `logic [15:0] sa`                          vita 1515673431  iverilog 1
//!   task-local `logic [7:0] sa[2]` WRITE                   vita "a,YY"      iverilog "ZZ,YY"
//!   generate-local `logic [15:0] sa` read                  vita E3009 range  iverilog 1
//! ```
//!
//! The last one is the tell that this was never string-specific: a *generate-local
//! packed vector*'s bit select was being range-checked against an unrelated module
//! string array's declared `[0:1]`.
//!
//! A block-local is a separate mechanism — v1 flattens it into the module namespace
//! by BARE name, so it collides with the array's name rather than nesting under it.
//! A colliding block-local `string` is loud (correct-or-loud; v1 has no per-block
//! scope for a static local). A NON-string one is left alone: it occupies `t.sa`
//! while the array's storage stays `t.sa$sae$i`, so it mostly coalesces harmlessly,
//! and rejecting on the bare name alone turned a dozen byte-correct designs loud.
//!
//! The shadow-aware walk is OPT-IN (`walk_scopes_key_shadowed`), used only by the
//! `string_array_elems` lookups. It keys on `symbols`, which is populated during
//! elaboration, so it is unsound wherever a lookup can run before the shadowing net
//! exists — see `nested_generate_body_survives_a_sibling_net_named_like_the_bound`.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_raw(src: &str) -> (String, bool, String) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_shadow_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    let so = String::from_utf8_lossy(&out.stdout).into_owned();
    let mut s = String::new();
    for l in so.lines().filter(|l| !l.starts_with("simulation ended")) {
        s.push_str(l);
        s.push('\n');
    }
    (
        s,
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn run(src: &str) -> String {
    let (out, ok, err) = run_raw(src);
    assert!(ok, "expected success, stderr:\n{err}");
    out
}

// ── an inner local SHADOWS the outer string array (was silent-wrong) ──────────

#[test]
fn function_local_string_shadows_module_string_array() {
    // The local is a scalar `string`, so `sa[i]` is the §6.16.2 BYTE select and the
    // compare is 'A'(65) < 'Z'(90) = 1. Resolving to the module ARRAY instead gave
    // "zz" < "aa" = 0.
    let out = run("module t;\n\
           string sa[2];\n\
           function automatic int f();\n\
             string sa;\n\
             sa = \"AZ\";\n\
             return (sa[0] < sa[1]);\n\
           endfunction\n\
           initial begin sa[0]=\"zz\"; sa[1]=\"aa\"; $display(\"R=%0d\", f()); end\n\
         endmodule\n");
    assert_eq!(out, "R=1\n");
}

#[test]
fn static_function_local_string_shadows_module_string_array() {
    // Same, on the INLINE (non-`automatic`) function path.
    let out = run("module t;\n\
           string sa[2];\n\
           function int f();\n\
             string sa;\n\
             sa = \"AZ\";\n\
             f = (sa[0] < sa[1]);\n\
           endfunction\n\
           initial begin sa[0]=\"zz\"; sa[1]=\"aa\"; $display(\"R=%0d\", f()); end\n\
         endmodule\n");
    assert_eq!(out, "R=1\n");
}

#[test]
fn task_local_vector_shadows_module_string_array() {
    // A NON-string local: `sa[0]`/`sa[1]` are bit selects of 16'h6162 → {0,1} = 1.
    let out = run("module t;\n\
           string sa[2];\n\
           int q;\n\
           task automatic tk();\n\
             logic [15:0] sa;\n\
             sa = 16'h6162;\n\
             q = {sa[0], sa[1]};\n\
           endtask\n\
           initial begin sa[0]=\"ZZZ\"; sa[1]=\"WWW\"; tk(); $display(\"R=%0d\", q); end\n\
         endmodule\n");
    assert_eq!(out, "R=1\n");
}

#[test]
fn task_local_array_write_does_not_clobber_module_string_array() {
    // The WRITE path twin: the task-local write must land on the LOCAL array, not on
    // the module string array's element net.
    let out = run("module t;\n\
           string sa[2];\n\
           task automatic tk();\n\
             logic [7:0] sa[2];\n\
             sa[0] = 8'h61;\n\
           endtask\n\
           initial begin sa[0]=\"ZZ\"; sa[1]=\"YY\"; tk(); $display(\"R=%s,%s\", sa[0], sa[1]); end\n\
         endmodule\n");
    assert_eq!(out, "R=ZZ,YY\n");
}

#[test]
fn generate_local_vector_shadows_module_string_array() {
    // Not string-specific at all: this used to be a LOUD misdiagnosis — the
    // generate-local vector's `sa[8]` was range-checked against the module string
    // array's declared `[0:1]` ("string-array index 8 is out of the declared range").
    let out = run("module t;\n\
           string sa[2];\n\
           genvar i;\n\
           generate for (i=0;i<1;i=i+1) begin : g\n\
             logic [15:0] sa;\n\
             initial begin sa = 16'hFF01; $display(\"R=%0d\", sa[8]); end\n\
           end endgenerate\n\
           initial begin sa[0]=\"A\"; sa[1]=\"B\"; end\n\
         endmodule\n");
    assert_eq!(out, "R=1\n");
}

// ── the OUTER entry is still visible when nothing shadows it ─────────────────

#[test]
fn module_string_array_still_visible_from_a_function() {
    let out = run("module t;\n\
           string sa[2];\n\
           function automatic int f();\n\
             return (sa[0] < sa[1]);\n\
           endfunction\n\
           initial begin sa[0]=\"aa\"; sa[1]=\"zz\"; $display(\"R=%0d\", f()); end\n\
         endmodule\n");
    assert_eq!(out, "R=1\n");
}

#[test]
fn module_net_still_visible_from_a_generate_block() {
    let out = run("module t;\n\
           logic [7:0] mv;\n\
           genvar i;\n\
           generate for (i=0;i<1;i=i+1) begin : g\n\
             initial begin #1 $display(\"R=%0h\", mv); end\n\
           end endgenerate\n\
           initial mv = 8'hA5;\n\
         endmodule\n");
    assert_eq!(out, "R=a5\n");
}

#[test]
fn module_const_array_still_folds_inside_a_generate_block() {
    // `array_const_vals` is another side-map reached through the same walk — the
    // shadow check must not break the ordinary outward fold.
    let out = run("module t;\n\
           localparam int ROT [3] = '{5, 6, 7};\n\
           genvar i;\n\
           generate for (i=0;i<1;i=i+1) begin : g\n\
             localparam int X = ROT[1];\n\
             initial $display(\"R=%0d\", X);\n\
           end endgenerate\n\
         endmodule\n");
    assert_eq!(out, "R=6\n");
}

#[test]
fn module_param_still_visible_from_a_function() {
    let out = run("module t;\n\
           localparam int P = 42;\n\
           function automatic int f(); return P; endfunction\n\
           initial $display(\"R=%0d\", f());\n\
         endmodule\n");
    assert_eq!(out, "R=42\n");
}

// ── the shadow check must NOT reach the param lookup (order-dependence) ──────

#[test]
fn nested_generate_body_survives_a_sibling_net_named_like_the_bound() {
    // REGRESSION GUARD for the reason `walk_scopes_key_shadowed` is opt-in.
    //
    // `symbols` is populated DURING elaboration, so a shadow test keyed on it is
    // order-dependent. `elaborate_gen_item` re-folds every generate control
    // expression once per phase but reports a fold failure only in the Nets phase.
    // Making the PARAM lookup shadow-aware made `j < N` fold in Nets (the sibling
    // net `N` did not exist yet) and fail in Logic (it did) — so the body was
    // unrolled, its nets created, and then never lowered: the whole generate body
    // silently vanished with exit 0 and no diagnostic. Strictly worse than the bug
    // being fixed. Keep the param consumer on the plain walk.
    let out = run("module t; localparam N = 2; genvar i, j;\n\
           generate for (i=0;i<1;i=i+1) begin : g\n\
             for (j=0;j<N;j=j+1) begin : h initial $display(\"h %0d %0d\", i, j); end\n\
             logic [7:0] N;\n\
           end endgenerate\n\
         endmodule\n");
    assert_eq!(out, "h 0 0\nh 0 1\n");
}

#[test]
fn generate_if_body_survives_a_sibling_net_named_like_the_condition() {
    let out = run("module t; localparam EN = 1; genvar i;\n\
           generate for (i=0;i<1;i=i+1) begin : g\n\
             if (EN) begin : h initial $display(\"taken\"); end\n\
             logic [7:0] EN;\n\
           end endgenerate\n\
         endmodule\n");
    assert_eq!(out, "taken\n");
}

// ── block-local collision: loud, not silently aliased ────────────────────────

#[test]
fn block_local_string_colliding_with_string_array_is_loud() {
    // v1 flattens a block-local into the module namespace by BARE name, so this one
    // collides with the array's name instead of nesting under it. It used to alias:
    // the module's own `sa[0]="zz"` became a putc byte-write into the block-local
    // scalar and read back "".
    // ⚠️ Not loud any more, and not aliased either: the block-local shadows a
    // module-scope name, so it gets its own `$blk$` net and the array keeps its
    // element storage. The property under test — the module's `sa[0]="zz"` and its
    // read-back must reach the SAME storage — is now asserted as the value iverilog
    // prints (`R=zz,yy`) rather than as a refusal.
    let (out, ok, err) = run_raw(
        "module t;\n\
           string sa[2];\n\
           initial begin : blk\n\
             string sa;\n\
             sa = \"AZ\";\n\
           end\n\
           initial begin sa[0]=\"zz\"; sa[1]=\"yy\"; #1 $display(\"R=%s,%s\", sa[0], sa[1]); end\n\
         endmodule\n",
    );
    assert!(ok, "expected a clean run:\n{err}");
    assert!(
        out.contains("R=zz,yy"),
        "write and read must reach one storage:\n{out}"
    );
}

#[test]
fn block_local_non_string_colliding_with_string_array_is_not_rejected() {
    // A NON-string block-local of the array's name mostly coalesces harmlessly: it
    // occupies `t.sa` while the array's storage stays `t.sa$sae$i`. iverilog runs
    // this and vita already got it right, so the collision loud must NOT fire here
    // (rejecting on the bare name alone turned a dozen correct designs loud).
    let out = run("module top;\n\
           string sa[2];\n\
           initial begin\n\
             logic [7:0] sa[2];\n\
             sa[0]=8'hAA; sa[1]=8'hBB;\n\
             $display(\"blk=%0h %0h\", sa[0], sa[1]);\n\
           end\n\
           initial begin sa[0]=\"ZZ\"; sa[1]=\"YY\";\n\
             #1 $display(\"mod=%s %s len=%0d\", sa[0], sa[1], sa[1].len()); end\n\
         endmodule\n");
    assert_eq!(out, "blk=aa bb\nmod=ZZ YY len=2\n");
}

#[test]
fn block_local_scalar_vector_colliding_with_string_array_is_not_rejected() {
    let out = run("module top;\n\
           string sa[2];\n\
           initial begin logic [7:0] sa; sa = 8'hFF; $display(\"blk=%0h\", sa); end\n\
           initial begin sa[0]=\"ZZ\"; sa[1]=\"YY\"; #1 $display(\"mod=%s %s\", sa[0], sa[1]); end\n\
         endmodule\n");
    assert_eq!(out, "blk=ff\nmod=ZZ YY\n");
}

#[test]
fn block_local_not_colliding_is_unaffected() {
    // The loud must be keyed on the collision, not on "a module string array exists".
    let out = run("module t;\n\
           string sa[2];\n\
           initial begin : blk\n\
             string other;\n\
             other = \"AZ\";\n\
             $display(\"R=%0d\", other[0] < other[1]);\n\
           end\n\
         endmodule\n");
    assert_eq!(out, "R=1\n");
}
