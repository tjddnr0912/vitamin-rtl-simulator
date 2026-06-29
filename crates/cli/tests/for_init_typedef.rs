//! A typed for-loop init using a USER-DEFINED type name — `for (my_t i = 0; …)`
//! where `my_t` is a typedef/enum/struct — was a parse error (E2002, "expected
//! '=' in for-clause assignment"): the for-init parser only recognized built-in
//! type keywords (`net_var_kind()`). Fixed by adding a typedef arm gated on the
//! same `<T> <name>` disambiguation used by block-local typedef decls and
//! function-return typedefs (§4.5.40/§4.5.41): the resolved `TypeInfo` supplies
//! kind/sign/range/packed, mirroring the built-in path, then a shared tail
//! synthesizes the renamed single-declarator loop variable. Pinned to iverilog
//! 13.0. Built-in for-init (`for (int i=0; …)`) stays byte-identical.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_fit_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    // The value is the first stdout line (vita appends a "simulation ended …"
    // trailer); also report process success for the loud-boundary tests.
    let first = String::from_utf8_lossy(&out.stdout)
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .to_owned();
    (first, out.status.success())
}

#[test]
fn typedef_int_loop_var() {
    let (o, ok) = run("module top; typedef int my_t;\n\
         initial begin int s=0; for(my_t i=0;i<4;i=i+1) s=s+i; $display(\"%0d\",s); #1 $finish; end endmodule");
    assert!(ok && o == "6", "got:\n{o}");
}

#[test]
fn typedef_vector_loop_var() {
    // typedef logic [3:0] nibble — the range comes from the TypeInfo, not an
    // inline range after the type name.
    let (o, ok) = run("module top; typedef logic [3:0] nibble;\n\
         initial begin int s=0; for(nibble i=0;i<4;i=i+1) s=s+i; $display(\"%0d\",s); #1 $finish; end endmodule");
    assert!(ok && o == "6", "got:\n{o}");
}

#[test]
fn typedef_atom_and_signed_loop_var() {
    let (ob, okb) = run("module top; typedef byte b_t;\n\
         initial begin int s=0; for(b_t i=0;i<5;i=i+1) s=s+i; $display(\"%0d\",s); #1 $finish; end endmodule");
    assert!(okb && ob == "10", "byte got:\n{ob}");
    // signed typedef: the loop runs from -2..=2, summing to 0.
    let (os, oks) = run("module top; typedef logic signed [7:0] s8;\n\
         initial begin int s=0; for(s8 i=-2;i<=2;i=i+1) s=s+i; $display(\"%0d\",s); #1 $finish; end endmodule");
    assert!(oks && os == "0", "signed got:\n{os}");
}

#[test]
fn typedef_loop_var_with_increment() {
    // The `i++` for-step shorthand works with a typedef loop var.
    let (o, ok) = run("module top; typedef int my_t;\n\
         initial begin int s=0; for(my_t i=0;i<4;i++) s=s+i; $display(\"%0d\",s); #1 $finish; end endmodule");
    assert!(ok && o == "6", "got:\n{o}");
}

#[test]
fn nested_typedef_loop_vars_same_name() {
    // A nested for reusing the same typedef loop-var name: each is renamed to a
    // distinct synthetic name, so the inner does not clobber the outer.
    let (o, ok) = run("module top; typedef int my_t;\n\
         initial begin int s=0; for(my_t i=0;i<3;i=i+1) for(my_t i=0;i<2;i=i+1) s=s+1; \
         $display(\"%0d\",s); #1 $finish; end endmodule");
    assert!(ok && o == "6", "got:\n{o}");
}

#[test]
fn typedef_loop_var_shadows_outer() {
    // The loop var is implicitly local: it must not clobber an outer same-named
    // variable (synthetic rename isolates it).
    let (o, ok) = run("module top; typedef int my_t; int i=99;\n\
         initial begin int s=0; for(my_t i=0;i<4;i=i+1) s=s+i; $display(\"%0d %0d\",s,i); #1 $finish; end endmodule");
    assert!(ok && o == "6 99", "got:\n{o}");
}

#[test]
fn typedef_for_init_in_function_and_task() {
    // Inside a function body (the for-init typedef arm is reached via tf_body too).
    let (of, okf) = run("module top; typedef int my_t;\n\
         function int f; int s=0; for(my_t i=0;i<4;i=i+1) s=s+i; f=s; endfunction\n\
         initial begin $display(\"%0d\",f()); #1 $finish; end endmodule");
    assert!(okf && of == "6", "func got:\n{of}");
    // And inside a task body.
    let (ot, okt) = run("module top; typedef int my_t; int s=0;\n\
         task t; for(my_t i=0;i<4;i=i+1) s=s+i; endtask\n\
         initial begin t(); $display(\"%0d\",s); #1 $finish; end endmodule");
    assert!(okt && ot == "6", "task got:\n{ot}");
}

#[test]
fn builtin_for_init_unchanged() {
    // Byte-identity: the built-in-keyword for-init path is unaffected.
    let (oi, oki) = run("module top;\n\
         initial begin int s=0; for(int i=0;i<4;i=i+1) s=s+i; $display(\"%0d\",s); #1 $finish; end endmodule");
    assert!(oki && oi == "6", "int got:\n{oi}");
    let (ol, okl) = run("module top;\n\
         initial begin int s=0; for(logic[3:0] i=0;i<4;i=i+1) s=s+i; $display(\"%0d\",s); #1 $finish; end endmodule");
    assert!(okl && ol == "6", "logic got:\n{ol}");
}

#[test]
fn enum_and_struct_typedef_loop_var() {
    // An enum typedef as the loop var (iverilog rejects the `i = i + 1` step on an
    // enum without a cast, so this is a vita-internal correctness check): A..C is 3
    // iterations before reaching D.
    let (oe, oke) = run("module top; typedef enum logic [2:0] {A,B,C,D} e;\n\
         initial begin int s=0; for(e i=A;i<D;i=i+1) s=s+1; $display(\"%0d\",s); #1 $finish; end endmodule");
    assert!(oke && oe == "3", "enum got:\n{oe}");
    // A packed-struct typedef as the loop var: the loop counter is the struct's
    // flat bitvector (8 bits here), 0..3 = 4 iterations (matches iverilog).
    let (os, oks) = run(
        "module top; typedef struct packed {logic[3:0] a; logic[3:0] b;} s_t;\n\
         initial begin int s=0; for(s_t i=0;i<4;i=i+1) s=s+1; $display(\"%0d\",s); #1 $finish; end endmodule",
    );
    assert!(oks && os == "4", "struct got:\n{os}");
}

#[test]
fn class_handle_for_loop_var_is_loud() {
    // A class name (handle type) cannot be a for-loop counter (§8.4) — loud, not a
    // silently-wrong int loop. The fix emits a clear parse-time diagnostic.
    let (_o, ok) = run(
        "module top; class C; int x; endclass\n\
         initial begin for(C i=null; i!=null; i=null) ; $display(\"x\"); #1 $finish; end endmodule",
    );
    assert!(!ok, "a class-handle for-loop variable must be loud");
}

#[test]
fn pre_declared_typedef_var_not_grabbed_as_decl() {
    // `for (i = 0; …)` where `i` is a pre-declared typedef-typed var must use the
    // existing var (a plain assign), NOT be misparsed as a new loop-var decl —
    // peek_block_typedef_decl returns None because the second token is `=`, not
    // an identifier.
    let (o, ok) = run("module top; typedef int my_t; my_t i;\n\
         initial begin int s=0; for(i=0;i<4;i=i+1) s=s+i; $display(\"%0d\",s); #1 $finish; end endmodule");
    assert!(ok && o == "6", "got:\n{o}");
}
