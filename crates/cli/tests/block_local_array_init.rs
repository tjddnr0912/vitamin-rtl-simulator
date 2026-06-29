//! A block-local unpacked-array declaration initializer `int a[4] = '{1,2,3,4}`
//! (inside an `initial`/`always` block) must apply the pattern, like a module-scope
//! one does (IEEE §6.8). vita's `hoist_block_local_nets` only pushed SCALAR locals
//! to the t0 pre-sweep (`name.unpacked.is_empty()` guard), so a block-local array
//! init was silently dropped — the array stayed at its default (0/x). Fixed by
//! mirroring `collect_var_init_drivers` (a non-constant init, including an array
//! pattern, rides the pre-sweep and is routed through `array_assign_special`).
//! Pinned to iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_blarr_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

const W: &str = "module top; initial begin";
const E: &str = " #1 $finish; end endmodule";

#[test]
fn block_local_int_array_declinit() {
    let out = run(&format!(
        "{W} int a[4]='{{1,2,3,4}}; $display(\"%0d %0d %0d %0d\",a[0],a[1],a[2],a[3]);{E}"
    ));
    assert!(out.contains("1 2 3 4"), "got:\n{out}");
}

#[test]
fn block_local_logic_array_declinit() {
    let out = run(&format!(
        "{W} logic [7:0] a[4]='{{8'h1,8'h2,8'h3,8'h4}}; \
         $display(\"%h %h %h %h\",a[0],a[1],a[2],a[3]);{E}"
    ));
    assert!(out.contains("01 02 03 04"), "got:\n{out}");
}

#[test]
fn block_local_array_declinit_then_foreach() {
    // The reported repro: decl-init then foreach-sum.
    let out = run(&format!(
        "{W} int a[4]='{{1,2,3,4}}; int s=0; foreach(a[i]) s=s+a[i]; $display(\"%0d\",s);{E}"
    ));
    assert!(out.contains("10"), "got:\n{out}");
}

#[test]
fn block_local_byte_array_signed() {
    let out = run(&format!(
        "{W} byte a[3]='{{-1,-2,-3}}; $display(\"%0d %0d %0d\",a[0],a[1],a[2]);{E}"
    ));
    assert!(out.contains("-1 -2 -3"), "got:\n{out}");
}

#[test]
fn block_local_array_expression_elements() {
    // Elements that read an earlier block-local scalar (declaration-order).
    let out = run(&format!(
        "{W} int k=10; int a[3]='{{k,k+1,k+2}}; $display(\"%0d %0d %0d\",a[0],a[1],a[2]);{E}"
    ));
    assert!(out.contains("10 11 12"), "got:\n{out}");
}

#[test]
fn scalar_block_local_init_unchanged() {
    // Byte-identity: a scalar block-local int/non-const init still works.
    let c = run(&format!("{W} int x=5; $display(\"%0d\",x);{E}"));
    assert!(c.contains('5'), "const scalar got:\n{c}");
    let nc = run(&format!(
        "{W} logic [3:0] g=4'h7; logic [3:0] x=g+1; $display(\"%h\",x);{E}"
    ));
    assert!(nc.contains('8'), "non-const scalar got:\n{nc}");
}

#[test]
fn module_scope_array_declinit_unchanged() {
    // Byte-identity: module-scope array decl-init (already worked) is unaffected.
    let out = run("module top; int a[3]='{7,8,9}; initial begin \
         $display(\"%0d %0d %0d\",a[0],a[1],a[2]); #1 $finish; end endmodule");
    assert!(out.contains("7 8 9"), "got:\n{out}");
}
