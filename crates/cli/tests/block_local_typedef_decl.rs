//! A procedural block-local declaration using a user-defined type name —
//! `initial begin my_enum_t s = IDLE; … end`, and likewise in a task/function
//! body — was a parse error (E2002): vita's block / tf-body decl loops only
//! recognized built-in type keywords (`net_var_kind`), not typedef names. The
//! module-item parser already recognized them (via `self.typedefs`). Fixed by
//! adding a `<typedef_name> <ident>` decl arm to `block_body` (begin/end + fork)
//! and `tf_body` (direct task/function body), dispatching to the same
//! `parse_typed_decl`. Pinned to iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_bltd_{}_{n}", std::process::id()));
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
        out.status.success(),
    )
}

#[test]
fn block_local_enum_decl() {
    let (o, ok) = run("module top; typedef enum {A,B,C} e;\n\
         initial begin e x=B; $display(\"%0d\",x); #1 $finish; end endmodule");
    assert!(ok && o.contains('1'), "got:\n{o}");
}

#[test]
fn block_local_typedef_and_struct_decl() {
    let (ot, okt) = run("module top; typedef logic [7:0] byte_t;\n\
         initial begin byte_t x=8'hAB; $display(\"%h\",x); #1 $finish; end endmodule");
    assert!(okt && ot.contains("ab"), "typedef got:\n{ot}");
    let (os, oks) = run(
        "module top; typedef struct packed {logic[3:0] a; logic[3:0] b;} s_t;\n\
         initial begin s_t x=8'h35; $display(\"%h %h\",x.a,x.b); #1 $finish; end endmodule",
    );
    assert!(oks && os.contains("3 5"), "struct got:\n{os}");
}

#[test]
fn block_local_struct_pattern_init() {
    // The struct '{} pattern decl-init (§4.5.35) also works for a block-local.
    let (o, ok) = run(
        "module top; typedef struct packed {logic[3:0] a; logic[3:0] b;} s_t;\n\
         initial begin s_t x='{4'h7,4'h8}; $display(\"%h\",x); #1 $finish; end endmodule",
    );
    assert!(ok && o.contains("78"), "got:\n{o}");
}

#[test]
fn task_body_typedef_decl() {
    // The DIRECT task body (no begin/end) uses tf_body.
    let (o, ok) = run(
        "module top; typedef enum {A,B,C} e; typedef logic[7:0] b_t;\n\
         task t; e x; b_t y; x=C; y=8'hAB; $display(\"%0d %h\",x,y); endtask\n\
         initial begin t(); #1 $finish; end endmodule",
    );
    assert!(ok && o.contains("2 ab"), "got:\n{o}");
}

#[test]
fn function_body_typedef_decl() {
    let (o, ok) = run("module top; typedef logic [7:0] b_t;\n\
         function logic [7:0] f; b_t tmp; tmp=8'hCD; f=tmp; endfunction\n\
         initial begin $display(\"%h\",f()); #1 $finish; end endmodule");
    assert!(ok && o.contains("cd"), "got:\n{o}");
}

#[test]
fn fork_body_typedef_decl() {
    // fork...join uses block_body (BlockEnd::Join).
    let (o, ok) = run(
        "module top; typedef enum {A,B,C} e;\n\
         initial begin fork begin e x; x=B; $display(\"%0d\",x); end join #1 $finish; end endmodule",
    );
    assert!(ok && o.contains('1'), "got:\n{o}");
}

#[test]
fn always_block_typedef_decl() {
    let (o, ok) = run(
        "module top; typedef enum logic[1:0]{S0,S1,S2} st; logic clk=0;\n\
         always #5 clk=~clk;\n\
         always @(posedge clk) begin st s; s=S1; $display(\"%0d\",s); end\n\
         initial #12 $finish; endmodule",
    );
    assert!(ok && o.contains('1'), "got:\n{o}");
}

#[test]
fn plain_body_decls_unchanged() {
    // Byte-identity: a body with only built-in-type decls is unaffected.
    let (ot, okt) = run(
        "module top; task t; int x; x=42; $display(\"%0d\",x); endtask\n\
         initial begin t(); #1 $finish; end endmodule",
    );
    assert!(okt && ot.contains("42"), "task got:\n{ot}");
    let (ob, okb) = run(
        "module top; initial begin int a[2]; a[0]=1; a[1]=2; $display(\"%0d %0d\",a[0],a[1]); \
         #1 $finish; end endmodule",
    );
    assert!(okb && ob.contains("1 2"), "block got:\n{ob}");
}

#[test]
fn statement_using_typedef_name_not_misparsed() {
    // A statement that references a typedef-name (`e::A` scope, or assigning a
    // module-scope enum var) must NOT be grabbed as a decl. Here `g=B` assigns a
    // module-scope enum var — the block has no decls and the statement runs.
    let (o, ok) = run("module top; typedef enum {A,B,C} e; e g;\n\
         initial begin g=B; $display(\"%0d\",g); #1 $finish; end endmodule");
    assert!(ok && o.contains('1'), "got:\n{o}");
}
