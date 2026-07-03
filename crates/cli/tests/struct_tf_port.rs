//! EXT2-C: a packed struct/union typedef as a function/task port (tf-port). vita
//! previously v1-loud-rejected these (`E2002` "a simple (non-struct) typedef type
//! for a tf-port …") while already supporting a struct-typed LOCAL var in a tf body
//! and a simple-vector/enum tf-port. The fix accepts a struct/union tf-port
//! (`try_tf_port_typedef` returns its type name), sizes the frame var to the
//! struct's flat vector (its `TypeInfo.range` = total width), and binds the port
//! NAME to the layout (`var_struct`) so `c.field` desugars to a part-select in the
//! body — exactly as a struct decl does. The binding is scoped to the enclosing
//! function/task by a snapshot/restore around the port list + body, so a struct
//! port name never leaks into the module or another tf (a leak would silently
//! mis-desugar `name.field` elsewhere).
//!
//! Pinned to iverilog 13.0. Out of scope (separately honest-loud): a struct RETURN
//! type and a struct-output-port FIELD write (frame-call-subset limit, E3009); a
//! multi-dim-packed tf-port (E2002 — blocked by a pre-existing md-packed frame-local
//! element-select gap).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_tfstruct_{}_{n}", std::process::id()));
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
fn struct_input_port_field_read() {
    // `function f(input cfg_t c)` — read `c.a`/`c.b` in the body. iverilog: sum=7.
    let (out, code) = run("module top;\n\
         typedef struct packed { logic [7:0] a; logic [7:0] b; } cfg_t;\n\
         function automatic logic [7:0] sum(input cfg_t c); sum = c.a + c.b; endfunction\n\
         cfg_t x;\n\
         initial begin x.a = 8'd3; x.b = 8'd4; #1 $display(\"sum=%0d\", sum(x)); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "struct input tf-port must parse+run:\n{out}");
    assert!(out.contains("sum=7"), "{out}");
}

#[test]
fn struct_output_port_whole_write() {
    // `task mk(output cfg_t c)` — a WHOLE-struct write `c = 16'h0506` (a field write
    // would hit the frame-call-subset limit). iverilog: a=5 b=6.
    let (out, code) = run("module top;\n\
         typedef struct packed { logic [7:0] a; logic [7:0] b; } cfg_t;\n\
         task automatic mk(output cfg_t c); c = 16'h0506; endtask\n\
         cfg_t y;\n\
         initial begin mk(y); #1 $display(\"a=%0d b=%0d\", y.a, y.b); $finish; end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "struct output tf-port whole-write must parse+run:\n{out}"
    );
    assert!(out.contains("a=5 b=6"), "{out}");
}

#[test]
fn union_tf_port() {
    // Equal-width packed union tf-port (iverilog requires equal-width members).
    let (out, code) = run("module top;\n\
         typedef union packed { logic [15:0] w; logic [15:0] m; } u_t;\n\
         function automatic logic [7:0] hi(input u_t u); hi = u.w[15:8]; endfunction\n\
         u_t x;\n\
         initial begin x.w = 16'hABCD; #1 $display(\"hi=%h\", hi(x)); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "union tf-port must parse+run:\n{out}");
    assert!(out.contains("hi=ab"), "{out}");
}

#[test]
fn comma_shared_struct_ports() {
    // `input cfg_t x, y` — the bare `y` inherits the struct type (and struct-ness)
    // via the sticky type inheritance, so `y.field` also desugars. iverilog: s=5.
    let (out, code) = run("module top;\n\
         typedef struct packed { logic [7:0] a; logic [7:0] b; } cfg_t;\n\
         function automatic logic [7:0] s2(input cfg_t x, y); s2 = x.a + y.b; endfunction\n\
         cfg_t p, q;\n\
         initial begin p.a=8'd1; p.b=8'd2; q.a=8'd3; q.b=8'd4; #1 $display(\"s=%0d\", s2(p, q)); $finish; end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "comma-shared struct ports must parse+run:\n{out}"
    );
    assert!(out.contains("s=5"), "{out}");
}

#[test]
fn non_ansi_struct_port() {
    // Non-ANSI formal `input cfg_t c;` declared in the body prefix (no header parens).
    let (out, code) = run("module top;\n\
         typedef struct packed { logic [7:0] a; logic [7:0] b; } cfg_t;\n\
         function automatic logic [7:0] sum;\n\
         input cfg_t c;\n\
         sum = c.a + c.b;\n\
         endfunction\n\
         cfg_t x;\n\
         initial begin x.a=8'd10; x.b=8'd20; #1 $display(\"sum=%0d\", sum(x)); $finish; end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "non-ANSI struct tf-port must parse+run:\n{out}"
    );
    assert!(out.contains("sum=30"), "{out}");
}

#[test]
fn scoped_struct_tf_port() {
    // EXT2-E + EXT2-C compose: a package scope-qualified struct typedef `pk::cfg_t`
    // as a tf-port. iverilog: sum=7.
    let (out, code) = run(
        "package pk; typedef struct packed { logic [7:0] a; logic [7:0] b; } cfg_t; endpackage\n\
         module top;\n\
         function automatic logic [7:0] sum(input pk::cfg_t c); sum = c.a + c.b; endfunction\n\
         pk::cfg_t x;\n\
         initial begin x.a=8'd3; x.b=8'd4; #1 $display(\"sum=%0d\", sum(x)); $finish; end\n\
         endmodule\n",
    );
    assert_eq!(
        code,
        Some(0),
        "scoped struct tf-port must parse+run:\n{out}"
    );
    assert!(out.contains("sum=7"), "{out}");
}

#[test]
fn struct_port_name_does_not_leak_across_functions() {
    // The struct binding from `f1(input cfg_t c)` must NOT leak into `f2(input
    // logic [7:0] c)` — `c` there is a plain vector. If the binding leaked, f2's `c`
    // would be mis-bound. iverilog: f1=3 f2=9.
    let (out, code) = run("module top;\n\
         typedef struct packed { logic [7:0] a; logic [7:0] b; } cfg_t;\n\
         function automatic logic [7:0] f1(input cfg_t c); f1 = c.a; endfunction\n\
         function automatic logic [7:0] f2(input logic [7:0] c); f2 = c; endfunction\n\
         initial begin #1 $display(\"f1=%0d f2=%0d\", f1(16'h0304), f2(8'd9)); $finish; end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "cross-function no-leak must parse+run:\n{out}"
    );
    assert!(
        out.contains("f1=3 f2=9"),
        "struct port binding leaked across functions:\n{out}"
    );
}

#[test]
fn struct_port_name_does_not_leak_to_module() {
    // A module var `c` (plain 16-bit) sharing the struct port's name must stay a
    // plain vector — passing it to `f` reads the high byte (c.a) INSIDE f only.
    // iverilog: v=18 (0x12).
    let (out, code) = run("module top;\n\
         typedef struct packed { logic [7:0] a; logic [7:0] b; } cfg_t;\n\
         function automatic logic [7:0] f(input cfg_t c); f = c.a; endfunction\n\
         logic [15:0] c;\n\
         initial begin c = 16'h1234; #1 $display(\"v=%0d\", f(c)); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "module-var no-leak must parse+run:\n{out}");
    assert!(out.contains("v=18"), "{out}");
}

// ---- deferred facets stay honest-loud (separate follow-ons) ----

#[test]
fn md_packed_tf_port_is_loud() {
    // A multi-dim-packed vector typedef as a tf-port stays loud (blocked by a
    // pre-existing md-packed frame-local element-select gap).
    let (out, code) = run("module top;\n\
         typedef logic [1:0][7:0] pair_t;\n\
         function automatic logic [7:0] lo(input pair_t p); lo = p[0]; endfunction\n\
         pair_t q;\n\
         initial begin q[0]=8'hCD; #1 $display(\"%h\", lo(q)); $finish; end\n\
         endmodule\n");
    assert_ne!(code, Some(0), "md-packed tf-port must stay loud:\n{out}");
}
