//! A procedural body-local typedef DEFINITION — `function f; typedef logic[3:0]
//! t; t x; …` (IEEE §6.18) — was a parse error: the function/task body parser
//! (tf_body) and the begin/end block parser (block_body) accepted a typedef NAME
//! used in a declaration, but not a typedef DEFINITION statement. iverilog 13.0
//! supports it.
//!
//! Fixed by a `parse_body_typedef_def` arm in both decl regions that calls the
//! existing `parse_typedef` (registering the name) and discards the AST node (a
//! body typedef emits no runtime decl). The name is LEXICALLY SCOPED: the type
//! registries are snapshotted at the body's first body-local typedef and restored
//! when the body ends, so a local name does not leak out of / clobber a same-named
//! outer typedef (which would be a silent-wrong — iverilog scopes it). An ENUM
//! body typedef needs elaborate-side label registration the body path can't do —
//! honest-loud. Pure parser, AST/`.vu`/format unchanged, IR-0. Pinned to iverilog
//! 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_btd_{}_{n}", std::process::id()));
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
fn function_body_typedef_alias() {
    let (o, ok) = run(
        "module top;\n\
         function automatic int f(input int a); typedef logic [3:0] t; t tmp; tmp=a[3:0]; return tmp; endfunction\n\
         initial begin $display(\"%0d\", f(255)); #1 $finish; end endmodule",
    );
    assert!(ok && o == "15", "got:\n{o}");
}

#[test]
fn task_body_typedef() {
    let (o, ok) = run(
        "module top;\n\
         task automatic tk(input int a, output int o); typedef logic [3:0] t; t tmp; tmp=a[3:0]; o=tmp; endtask\n\
         int r; initial begin tk(255, r); $display(\"%0d\", r); #1 $finish; end endmodule",
    );
    assert!(ok && o == "15", "got:\n{o}");
}

#[test]
fn initial_block_typedef() {
    let (o, ok) = run(
        "module top;\n\
         initial begin typedef logic [7:0] t; t x; x=8'hAB; $display(\"%h\", x); #1 $finish; end endmodule",
    );
    assert!(ok && o == "ab", "got:\n{o}");
}

#[test]
fn always_block_typedef() {
    let (o, ok) = run("module top; logic clk=0; logic [7:0] r;\n\
         always @(posedge clk) begin typedef logic [7:0] t; t x; x=8'hCD; r=x; end\n\
         initial begin #1 clk=1; #1 $display(\"%h\", r); #1 $finish; end endmodule");
    assert!(ok && o == "cd", "got:\n{o}");
}

#[test]
fn block_struct_typedef_field_access() {
    // A struct typedef defined in a block; field access works (block context allows
    // the part-select assign — the frame-call subset does not, but that is a
    // separate pre-existing limitation independent of this change).
    let (o, ok) = run(
        "module top; logic [7:0] r;\n\
         initial begin typedef struct packed {logic [3:0] a, b;} s_t; s_t s; s.a=4'hC; s.b=4'hD; r=s; $display(\"%h\", r); #1 $finish; end endmodule",
    );
    assert!(ok && o == "cd", "got:\n{o}");
}

#[test]
fn body_chained_typedef() {
    let (o, ok) = run(
        "module top;\n\
         function automatic int f; typedef logic [7:0] b; typedef b a; a x; x=8'hAB; return x; endfunction\n\
         initial begin $display(\"%h\", f()); #1 $finish; end endmodule",
    );
    assert!(ok && o == "000000ab", "got:\n{o}");
}

#[test]
fn local_typedef_is_scoped_not_clobbering_outer() {
    // CRITICAL: a function-local typedef `t` (4-bit) must NOT clobber the
    // module-level `t` (8-bit). iverilog prints "ff 15"; a flat-map clobber would
    // truncate z to 4 bits ("0f 15") — a silent-wrong the scoping prevents.
    let (o, ok) = run("module top; typedef logic [7:0] t;\n\
         function automatic int f; typedef logic [3:0] t; t x; x=15; return x; endfunction\n\
         initial begin t z; z=8'hFF; $display(\"%h %0d\", z, f()); #1 $finish; end endmodule");
    assert!(ok && o == "ff 15", "got:\n{o}");
}

#[test]
fn local_typedef_does_not_leak_out() {
    // A function-local typedef is invisible at module scope after the function —
    // iverilog rejects `lt y;`; vita must too (the restore drops the local name).
    let (_o, ok) = run("module top;\n\
         function automatic int f; typedef logic [3:0] lt; lt x; x=5; return x; endfunction\n\
         lt y;\n\
         initial begin $display(\"%0d\", f()); #1 $finish; end endmodule");
    assert!(
        !ok,
        "a function-local typedef must not leak to module scope"
    );
}

#[test]
fn nested_block_typedef_is_scoped() {
    // An inner-block typedef is invisible in the outer block after the inner closes.
    let (_o, ok) = run("module top;\n\
         initial begin\n\
           begin typedef logic [3:0] it; it a; a=4'hF; $display(\"%h\", a); end\n\
           it b;\n\
           #1 $finish;\n\
         end endmodule");
    assert!(
        !ok,
        "an inner-block typedef must not be visible in the outer block"
    );
}

#[test]
fn body_enum_typedef_is_loud() {
    // An enum typedef in a procedural body needs elaborate-side label registration
    // — honest-loud (define at module scope instead).
    let (_o, ok) = run(
        "module top;\n\
         function automatic int f; typedef enum logic [1:0] {X,Y,Z} e; e v; v=Y; return v; endfunction\n\
         initial begin $display(\"%0d\", f()); #1 $finish; end endmodule",
    );
    assert!(!ok, "a body-local enum typedef must be loud");
}

#[test]
fn no_body_typedef_unchanged() {
    // Byte-identity: a body with no typedef is unaffected.
    let (o, ok) = run(
        "module top;\n\
         function automatic int f(input int a); logic [3:0] tmp; tmp=a[3:0]; return tmp; endfunction\n\
         initial begin $display(\"%0d\", f(255)); #1 $finish; end endmodule",
    );
    assert!(ok && o == "15", "got:\n{o}");
}
