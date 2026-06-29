//! A function may READ a module-level variable (SV §13.4 — functions are not
//! pure). vita previously rejected this with E3010 (`undeclared net/variable
//! top.$func$<fn>.<modvar>`) for any function lowered on the FRAME path
//! (`automatic`, a 2-state `int` return, recursive, or a control-flow body): the
//! `$func$<name>` scope was a hard boundary, so a module-var reference resolved
//! under the function scope and was never looked up in the module scope. The fix
//! makes `$func$<name>` transparent for the outward name walk (like `$itask$`
//! already is), so a NON-local name falls through to the module scope while a
//! formal/local — registered UNDER the function scope — is still found first and
//! shadows. WRITING a module var from a frame function stays loud (the frame-call
//! subset writes only its own locals). Pinned to iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_fmv_{}_{n}", std::process::id()));
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
fn frame_function_reads_module_var() {
    // `int` return ⇒ frame path. The body reads module var `g`.
    let (out, code) = run(
        "module top; integer g; function int rd; rd=g+1; endfunction\n\
         integer x; initial begin g=10; x=rd(); $display(\"R x=%0d\",x); $finish; end endmodule\n",
    );
    assert_eq!(code, Some(0));
    assert!(
        out.contains("R x=11"),
        "frame fn reads module var; got:\n{out}"
    );
}

#[test]
fn automatic_function_reads_module_var() {
    let (out, code) = run(
        "module top; integer g; function automatic int rd; rd=g*2; endfunction\n\
         integer x; initial begin g=7; x=rd(); $display(\"R x=%0d\",x); $finish; end endmodule\n",
    );
    assert_eq!(code, Some(0));
    assert!(
        out.contains("R x=14"),
        "automatic fn reads module var; got:\n{out}"
    );
}

#[test]
fn recursive_function_reads_module_var() {
    // Recursive factorial whose base case reads module var `base`. fac(4)*base = 24.
    let (out, code) = run("module top; integer base;\n\
         function automatic int fac; input int n; if(n<=1) fac=base; else fac=n*fac(n-1); endfunction\n\
         integer x; initial begin base=1; x=fac(4); $display(\"R x=%0d\",x); $finish; end endmodule\n");
    assert_eq!(code, Some(0));
    assert!(
        out.contains("R x=24"),
        "recursive fn reads module var; got:\n{out}"
    );
}

#[test]
fn local_shadows_module_var() {
    // A function local named like a module var must resolve to the LOCAL (the
    // local is registered under the function scope, found first), and the module
    // var must be untouched.
    let (out, code) = run("module top; integer v;\n\
         function int f; int v; begin v=99; f=v; end endfunction\n\
         integer x; initial begin v=5; x=f(); $display(\"R x=%0d topv=%0d\",x,v); $finish; end endmodule\n");
    assert_eq!(code, Some(0));
    assert!(
        out.contains("R x=99 topv=5"),
        "local shadows module var; got:\n{out}"
    );
}

#[test]
fn formal_and_module_var_mixed() {
    // The body reads both a formal (`a`) and a module var (`base`).
    let (out, code) = run("module top; integer base;\n\
         function int addb; input int a; addb=a+base; endfunction\n\
         integer x; initial begin base=100; x=addb(5); $display(\"R x=%0d\",x); $finish; end endmodule\n");
    assert_eq!(code, Some(0));
    assert!(out.contains("R x=105"), "formal+module var; got:\n{out}");
}

#[test]
fn frame_function_writing_module_var_is_loud() {
    // Writing a module var from a frame function is OUTSIDE the frame-call subset
    // (writes only its own locals) — must stay a loud error, never silent.
    let (_o, code) = run(
        "module top; integer cnt; function int inc; inc=0; cnt=cnt+1; endfunction\n\
         integer x; initial begin cnt=0; x=inc(); $display(\"%0d\",cnt); $finish; end endmodule\n",
    );
    assert_ne!(
        code,
        Some(0),
        "writing a module var from a frame fn must be loud"
    );
}

#[test]
fn param_width_frame_function_resolves_param() {
    // Secondary fix: a frame function with a PARAMETER-width formal/return
    // (`function [W-1:0] f(input [W-1:0] a)`) must resolve the module parameter
    // `W` for its width. Before the `$func$` transparency fix, `W` failed to
    // resolve under the function scope and the width silently fell back to 1.
    let (out, code) = run("module top; parameter W=8;\n\
         function automatic [W-1:0] f; input [W-1:0] a; f=a+1; endfunction\n\
         logic [7:0] x; initial begin x=f(8'hFE); $display(\"R %h\",x); $finish; end endmodule\n");
    assert_eq!(code, Some(0));
    assert!(out.contains("R ff"), "param-width frame fn; got:\n{out}"); // FE+1 = FF
}

#[test]
fn undeclared_name_in_function_still_loud() {
    // A name that is neither a local nor a module var must still be a loud error
    // (the transparent walk must not invent a resolution).
    let (_o, code) = run("module top;\n\
         function int f; f=nonexist+1; endfunction\n\
         integer x; initial begin x=f(); $display(\"%0d\",x); $finish; end endmodule\n");
    assert_ne!(
        code,
        Some(0),
        "undeclared name in a function must stay loud"
    );
}
