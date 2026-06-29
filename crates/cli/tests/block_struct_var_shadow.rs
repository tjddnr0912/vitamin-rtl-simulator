//! A block-local struct variable whose name SHADOWS a module-level struct
//! variable used to clobber the parser's flat `var_struct` (var→type) map: after
//! the block, the OUTER `x.field` desugared against the INNER struct's layout —
//! e.g. printing `b` (4-bit field) instead of `bb` (8-bit). A pre-existing
//! silent-wrong, surfaced while adding body-local typedefs (§4.5.51).
//!
//! Fixed by extending the block scope snapshot/restore (§4.5.51) to the
//! VAR-name-keyed maps (var_struct / var_enum / struct_scalar_vars /
//! struct_1d_array_vars): block_body now snapshots them at the block's first
//! struct/enum-typed var decl (or body typedef) and restores them when the block
//! ends, so a block-local struct var's layout binding does not leak out of /
//! clobber the outer one. Pure parser, AST/`.vu`/format unchanged, IR-0. Pinned
//! to iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_bsvs_{}_{n}", std::process::id()));
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
fn block_local_struct_var_shadow_is_scoped() {
    // THE fix: a block-local `inner_s x` (4-bit field) shadows a module-level
    // `outer_s x` (8-bit field). After the block, the module-level `x.a=8'hBB`
    // must print `bb`, not the truncated `b` the flat-map clobber produced.
    let (o, ok) = run("module top;\n\
         typedef struct packed {logic [7:0] a;} outer_s;\n\
         typedef struct packed {logic [3:0] a;} inner_s;\n\
         outer_s x;\n\
         initial begin inner_s x; x.a=4'hA; end\n\
         initial begin #1 x.a=8'hBB; #2 $display(\"%h\", x.a); #1 $finish; end endmodule");
    assert!(ok && o == "bb", "got:\n{o}");
}

#[test]
fn block_local_struct_var_used_inside_block() {
    // The block-local struct var resolves correctly to its own (inner) layout
    // WHILE inside the block.
    let (o, ok) = run("module top;\n\
         typedef struct packed {logic [7:0] a;} outer_s;\n\
         typedef struct packed {logic [3:0] a;} inner_s;\n\
         outer_s x;\n\
         initial begin inner_s x; x.a=4'hA; $display(\"%h\", x.a); end\n\
         initial begin #1 $finish; end endmodule");
    assert!(ok && o == "a", "got:\n{o}");
}

#[test]
fn unique_named_block_struct_var_unaffected() {
    // A block-local struct var with a UNIQUE name (no shadow) is unchanged — the
    // outer var and the unique local both read back correctly.
    let (o, ok) = run(
        "module top;\n\
         typedef struct packed {logic [7:0] a;} outer_s;\n\
         outer_s x;\n\
         initial begin typedef struct packed {logic [3:0] a;} t2; t2 q; q.a=4'h7; x.a=8'hBB; $display(\"%h %h\", x.a, q.a); #1 $finish; end endmodule",
    );
    assert!(ok && o == "bb 7", "got:\n{o}");
}

#[test]
fn body_typedef_scope_still_works() {
    // Regression: the §4.5.51 body-typedef scoping (now sharing the same snapshot)
    // is unaffected — module `t` (8-bit) and function-local `t` (4-bit) stay
    // separate ("ff 15").
    let (o, ok) = run("module top; typedef logic [7:0] t;\n\
         function automatic int f; typedef logic [3:0] t; t x; x=15; return x; endfunction\n\
         initial begin t z; z=8'hFF; $display(\"%h %0d\", z, f()); #1 $finish; end endmodule");
    assert!(ok && o == "ff 15", "got:\n{o}");
}

#[test]
fn plain_block_is_byte_identical() {
    // A block with only plain (non-struct/non-typedef) locals never snapshots —
    // byte-identical to before.
    let (o, ok) = run(
        "module top; logic [7:0] r;\n\
         initial begin logic [7:0] a; a=8'hAB; r=a; $display(\"%h\", r); #1 $finish; end endmodule",
    );
    assert!(ok && o == "ab", "got:\n{o}");
}

#[test]
fn enum_var_value_unaffected_by_shadow() {
    // An enum var's VALUE width comes from its decl, not var_enum (which only
    // affects methods), so an enum-var shadow was already benign — confirm it
    // stays correct after the scope change.
    let (o, ok) = run("module top;\n\
         typedef enum logic [7:0] {A=8'hAA} e8;\n\
         typedef enum logic [3:0] {B=4'h5} e4;\n\
         e8 x;\n\
         initial begin e4 x; x=B; end\n\
         initial begin #1 x=A; #2 $display(\"%h\", x); #1 $finish; end endmodule");
    assert!(ok && o == "aa", "got:\n{o}");
}

#[test]
fn function_local_struct_var_shadow_is_scoped() {
    // The SAME shadow scoping must hold in a function/task body (tf_body), not
    // just begin/end blocks: a function-local `inner_s x` (4-bit) need NOT do a
    // struct field assign to clobber var_struct (so the frame-call-subset E3009
    // does not cover it). After the function, the module-level `x.a=8'hbb` must
    // read the OUTER (8-bit) layout → `bb`, not the truncated `b`. Both values are
    // on one $display line so the first-line check covers the critical `x.a`.
    let (o, ok) = run("module m;\n\
         typedef struct packed {logic [7:0] a;} outer_s;\n\
         typedef struct packed {logic [3:0] a;} inner_s;\n\
         outer_s x;\n\
         function automatic logic [7:0] f; inner_s x; return 8'hff; endfunction\n\
         initial begin logic [7:0] r; r=f(); x.a=8'hbb; $display(\"%h %h\", r, x.a); #1 $finish; end endmodule");
    assert!(ok && o == "ff bb", "got:\n{o}");
}
