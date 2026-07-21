//! §4.5.192 UARR: a packable UNPACKED-struct SCALAR body-local in a TASK / FUNCTION
//! (`rec_t p;`) loud→supported — round-16 report V8 (blocks TB=partial: a task-local
//! unpacked struct). The type lives only in `unpacked_struct_layouts` (not `typedefs`),
//! so the tf-body decl loop missed it and `rec_t p;` parsed as a statement (E2002).
//!
//! Fix: recognized in the tf-body decl loop and lowered to a packed-vector local
//! (`logic/bit [W-1:0] p;`) registered as a scalar struct var, so `p.field` desugars
//! to a part-select (`struct_field_select` → `packable_record_layout`) exactly like a
//! packed struct — which already worked as a body-local. A non-packable record
//! (string/real/nested member) stays loud (correct-or-loud); module-scope records keep
//! their member-net representation.
//!
//! iverilog rejects unpacked structs outright, so verified by vita self-consistency
//! (field write→read, whole-value packing, per-call frame re-init) + regression that
//! module-scope member-net records and packed-struct body-locals are unchanged.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_flus_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

#[test]
fn task_local_unpacked_struct() {
    let o = run("module top;\n\
         typedef struct { int a; int b; } pair_t;\n\
         task automatic t();\n\
           pair_t p;\n\
           p.a=3; p.b=4;\n\
           $display(\"s=%0d a=%0d b=%0d\", p.a+p.b, p.a, p.b);\n\
         endtask\n\
         initial t();\nendmodule\n");
    assert!(o.contains("s=7 a=3 b=4"), "got:\n{o}");
}

#[test]
fn function_local_unpacked_struct() {
    let o = run("module top;\n\
         typedef struct { int a; int b; } pair_t;\n\
         function automatic int f();\n\
           pair_t p;\n\
           p.a=5; p.b=6;\n\
           return p.a + p.b;\n\
         endfunction\n\
         initial $display(\"f=%0d\", f());\nendmodule\n");
    assert!(o.contains("f=11"), "got:\n{o}");
}

#[test]
fn frame_local_reinit_per_call() {
    // The packed-vector local is a frame-local, so each call gets fresh storage.
    let o = run("module top;\n\
         typedef struct { logic [7:0] x; logic [15:0] y; } rec_t;\n\
         task automatic t(int seed);\n\
           rec_t r;\n\
           r.x=seed; r.y=seed*256;\n\
           $display(\"x=%0d y=%0d\", r.x, r.y);\n\
         endtask\n\
         initial begin t(3); t(7); end\nendmodule\n");
    assert!(o.contains("x=3 y=768"), "got:\n{o}");
    assert!(o.contains("x=7 y=1792"), "got:\n{o}");
}

#[test]
fn runtime_assign_pattern_in_task() {
    let o = run("module top;\n\
         typedef struct { logic [7:0] a; logic [7:0] b; } pr;\n\
         task automatic t();\n\
           pr p;\n\
           p = '{8'h11, 8'h22};\n\
           $display(\"%h %h\", p.a, p.b);\n\
         endtask\n\
         initial t();\nendmodule\n");
    assert!(o.contains("11 22"), "got:\n{o}");
}

#[test]
fn read_before_write_is_x_for_four_state() {
    let o = run("module top;\n\
         typedef struct { logic [7:0] a; logic [7:0] b; } pr;\n\
         task automatic t();\n\
           pr p;\n\
           p.a=8'h11;\n\
           $display(\"a=%h b=%h\", p.a, p.b);\n\
         endtask\n\
         initial t();\nendmodule\n");
    assert!(o.contains("a=11 b=xx"), "got:\n{o}");
}

#[test]
fn nonpackable_record_body_local_stays_loud() {
    // A string-member record is not packable → loud (correct-or-loud).
    let o = run("module top;\n\
         typedef struct { string name; int id; } rec_t;\n\
         task automatic t(); rec_t r; r.id=5; $display(\"%0d\", r.id); endtask\n\
         initial t();\nendmodule\n");
    assert!(
        o.contains("E2002") || o.contains("E3009"),
        "should be loud:\n{o}"
    );
}

#[test]
fn module_scope_record_unchanged() {
    // Regression: a module-scope unpacked-struct var keeps its member-net path.
    let o = run("module top;\n\
         typedef struct { int a; int b; } pr;\n\
         pr p;\n\
         initial begin p.a=3; p.b=4; $display(\"m=%0d\", p.a+p.b); end\nendmodule\n");
    assert!(o.contains("m=7"), "got:\n{o}");
}

#[test]
fn packed_struct_body_local_unchanged() {
    // Regression: a packed-struct body-local (the pre-existing supported path).
    let o = run("module top;\n\
         typedef struct packed { int a; int b; } pr;\n\
         task automatic t(); pr p; p.a=3; p.b=4; $display(\"p=%0d\", p.a+p.b); endtask\n\
         initial t();\nendmodule\n");
    assert!(o.contains("p=7"), "got:\n{o}");
}
