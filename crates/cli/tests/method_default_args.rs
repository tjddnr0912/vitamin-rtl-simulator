//! Class METHOD and CONSTRUCTOR calls dropped omitted trailing default arguments
//! (IEEE §13.5.3): an omitted formal with a `= default` bound 0/X instead of the
//! default expression — `f(int a, int b=7)` called `c.f(10)` gave 10, not 17. The
//! frame/inline/task call paths already fill defaults via `fill_default_args`;
//! `build_class_call` and `lower_ctor_call` did not. Fix: both now call
//! `fill_default_args` before lowering the actuals. Pinned to iverilog 13.0 (except
//! virtual dispatch through a base handle, where iverilog's vtable is buggy — vita
//! is IEEE-correct).
//!
//! SCOPE (correct-or-loud): a filled default is lowered in the CALLER scope, but
//! IEEE resolves it in the method's CLASS scope. A LITERAL default is
//! scope-independent and filled correctly; a NAME/CALL default (field, method,
//! class parameter, `.with()` reduction, localparam, …) is scope-ambiguous — with no
//! authoritative per-class scope-symbol table, an allow-list LOUD-rejects every
//! non-literal filled default rather than risk a silent caller-scope mis-bind.
//! Class-scope resolution of name-defaults is a follow-on.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_mdefarg_{}_{n}", std::process::id()));
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
fn method_int_default_omitted() {
    let src = "module m;\n\
         class C; function int f(input int a, input int b=7); return a+b; endfunction endclass\n\
         C c; initial begin c=new; $display(\"r=%0d\", c.f(10)); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=17"), "int default omitted, got:\n{out}");
}

#[test]
fn method_int_default_provided_unchanged() {
    let src = "module m;\n\
         class C; function int f(input int a, input int b=7); return a+b; endfunction endclass\n\
         C c; initial begin c=new; $display(\"r=%0d\", c.f(10,20)); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=30"), "both provided, got:\n{out}");
}

#[test]
fn method_string_default_omitted() {
    // A string default was ALSO dropped (type-agnostic).
    let src = "module m;\n\
         class C; function int f(input int a, input string s=\"yes\"); return a+(s==\"yes\"); endfunction endclass\n\
         C c; initial begin c=new; $display(\"r=%0d\", c.f(9)); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=10"), "string default omitted, got:\n{out}");
}

#[test]
fn method_multiple_defaults() {
    // Omit both, then omit one — each omitted formal gets its default.
    let base = "module m;\n\
         class C; function int f(input int a, input int b=2, input int c=3); return a+b+c; endfunction endclass\n\
         C c; initial begin c=new; $display(\"r=%0d\", c.f";
    let (out, code) = run(&format!("{base}(10)); #1 $finish; end endmodule\n"));
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=15"), "omit both, got:\n{out}");
    let (out, code) = run(&format!("{base}(10,20)); #1 $finish; end endmodule\n"));
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=33"), "omit one, got:\n{out}");
}

#[test]
fn constructor_default_arg() {
    let omit = "module m;\n\
         class C; int v; function new(input int x=5); v=x; endfunction endclass\n\
         C c; initial begin c=new; $display(\"r=%0d\", c.v); #1 $finish; end endmodule\n";
    let (out, code) = run(omit);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=5"), "ctor default omitted, got:\n{out}");
    let given = "module m;\n\
         class C; int v; function new(input int x=5); v=x; endfunction endclass\n\
         C c; initial begin c=new(42); $display(\"r=%0d\", c.v); #1 $finish; end endmodule\n";
    let (out, code) = run(given);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=42"), "ctor arg provided, got:\n{out}");
}

#[test]
fn super_new_default_arg() {
    // A derived ctor calling `super.new()` with the base default omitted must fill
    // the base default (build_class_call path).
    let src = "module m;\n\
         class B; int v; function new(input int x=9); v=x; endfunction endclass\n\
         class D extends B; function new; super.new(); endfunction endclass\n\
         D d; initial begin d=new; $display(\"r=%0d\", d.v); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=9"), "super.new default, got:\n{out}");
}

#[test]
fn inherited_method_default() {
    let src = "module m;\n\
         class B; function int f(input int a, input int b=7); return a+b; endfunction endclass\n\
         class D extends B; endclass\n\
         D d; initial begin d=new; $display(\"r=%0d\", d.f(10)); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=17"), "inherited default, got:\n{out}");
}

#[test]
fn virtual_method_default_dispatches_to_derived() {
    // h (base handle) → D object; h.f(3) virtual-dispatches to D::f with D's default
    // b=7 → 3*7 = 21. (iverilog mis-dispatches to B::f here even for non-string;
    // vita is IEEE-correct.)
    let src = "module m;\n\
         class B; virtual function int f(input int a, input int b=7); return a+b; endfunction endclass\n\
         class D extends B; virtual function int f(input int a, input int b=7); return a*b; endfunction endclass\n\
         B h; D d; initial begin d=new; h=d; $display(\"r=%0d\", h.f(3)); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("r=21"),
        "virtual default dispatch, got:\n{out}"
    );
}

#[test]
fn pkg_scoped_default_works() {
    // A `pkg::K` default resolves in the PACKAGE scope regardless of caller/class
    // scope — scope-independent, so it is allowed (works).
    let src = "package p; localparam int K=99; endpackage\n\
         module m;\n\
         class C; function int f(input int a, input int b=p::K); return a+b; endfunction endclass\n\
         C c; initial begin c=new; $display(\"r=%0d\", c.f(1)); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=100"), "pkg-scoped default, got:\n{out}");
}

#[test]
fn non_default_method_unaffected() {
    let src = "module m;\n\
         class C; function int f(input int a, input int b); return a*b; endfunction endclass\n\
         C c; initial begin c=new; $display(\"r=%0d\", c.f(12,11)); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=132"), "no-default method, got:\n{out}");
}

#[test]
fn too_many_args_loud_rejects() {
    // Passing MORE args than formals must stay loud (not silently ignore extras).
    let src = "module m;\n\
         class C; function int f(input int a); return a; endfunction endclass\n\
         C c; initial begin c=new; $display(\"r=%0d\", c.f(1,2,3)); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(
        code,
        Some(1),
        "expected loud reject, got code={code:?}\n{out}"
    );
    assert!(
        !out.contains("r="),
        "must not silently compute, got:\n{out}"
    );
}

#[test]
fn missing_required_arg_loud_rejects() {
    // Omitting a formal with NO default must stay loud (not bind 0).
    let src = "module m;\n\
         class C; function int f(input int a, input int b); return a+b; endfunction endclass\n\
         C c; initial begin c=new; $display(\"r=%0d\", c.f(1)); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(
        code,
        Some(1),
        "expected loud reject, got code={code:?}\n{out}"
    );
    assert!(
        !out.contains("r="),
        "must not silently compute, got:\n{out}"
    );
}

#[test]
fn name_or_call_defaults_loud_reject() {
    // A filled default is lowered in the CALLER scope but IEEE resolves it in the
    // method's CLASS scope, so any NAME/CALL default is scope-ambiguous: it may name
    // a class field, method, parameter, or a `.with()` reduction over a member, and
    // would silently bind a same-named caller name. vita has no per-class scope table
    // (params are monomorphised), so an ALLOW-LIST loud-rejects every non-literal
    // filled default (adversarial-review findings across 3 rounds). Each case here
    // has a colliding caller-scope name to prove the leak is closed.
    let cases = [
        // field ref + colliding caller variable.
        "module m;\n\
         class C; int k;\n\
          function int f(input int a, input int b=k); return a+b; endfunction\n\
          function void setk(input int x); k=x; endfunction endclass\n\
         int k; C c;\n\
         initial begin c=new; c.setk(100); k=5; $display(\"r=%0d\", c.f(1)); #1 $finish; end endmodule\n",
        // constructor field ref.
        "module m;\n\
         class C; int base; int v; function new(input int x=base); v=x; endfunction endclass\n\
         int base; C c; initial begin base=9; c=new; $display(\"r=%0d\", c.v); #1 $finish; end endmodule\n",
        // handle-field member PATH `inst.Y` + colliding module instance.
        "module sub; int Y=5; endmodule\n\
         module m;\n\
         class Inner; int Y; endclass\n\
         class C; Inner inst; function int f(input int a, input int b=inst.Y); return a+b; endfunction endclass\n\
         sub inst(); C c;\n\
         initial begin c=new; $display(\"r=%0d\", c.f(1)); #1 $finish; end endmodule\n",
        // method-call default `helper()` + colliding free function.
        "module m;\n\
         function int helper(); return 999; endfunction\n\
         class C; function int helper(); return 7; endfunction\n\
          function int f(input int a, input int b=helper()); return a+b; endfunction endclass\n\
         C c; initial begin c=new; $display(\"r=%0d\", c.f(1)); #1 $finish; end endmodule\n",
        // class PARAMETER ref `W` + colliding module localparam (monomorphised class).
        "module m;\n\
         localparam int W=3;\n\
         class C #(int W=50); function int f(input int a, input int b=W); return a+b; endfunction endclass\n\
         C #(200) c; initial begin c=new; $display(\"r=%0d\", c.f(1)); #1 $finish; end endmodule\n",
        // `.with()` reduction over a member `Y` + colliding module var.
        "module top; int Y=99; int q[$];\n\
         class C; int Y=7; function int f(input int a=q.sum() with (item+Y)); return a; endfunction endclass\n\
         C c; initial begin q.push_back(10); c=new; $display(\"r=%0d\", c.f()); #1 $finish; end endmodule\n",
        // a plain localparam ref (no class-scope collision) — still loud (allow-list).
        "module m;\n\
         localparam int K=99;\n\
         class C; function int f(input int a, input int b=K); return a+b; endfunction endclass\n\
         C c; initial begin c=new; $display(\"r=%0d\", c.f(1)); #1 $finish; end endmodule\n",
    ];
    for src in cases {
        let (out, code) = run(src);
        assert_eq!(code, Some(1), "expected loud reject\n{out}");
        assert!(!out.contains("r="), "must not silently bind, got:\n{out}");
    }
}
