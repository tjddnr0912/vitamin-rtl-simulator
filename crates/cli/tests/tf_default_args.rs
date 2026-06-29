//! Function / task default argument values (IEEE §13.5.3) — `function f(int a,
//! int b = 10)` — were a parse error ("expected ')' closing tf-port list, found
//! Eq"): an ANSI tf-port did not accept a `= default` value, and a call omitting
//! trailing actuals errored on the arity mismatch. iverilog supports them
//! (`f(5)` → b defaults to 10).
//!
//! Fixed by adding `default: Option<Expr>` to `TfPort` (parsed for ANSI tf-ports)
//! and a shared `fill_default_args` at the call site (inline + frame function,
//! inline + frame task) that fills omitted trailing actuals with their formals'
//! default expressions, evaluated in the caller scope. A missing actual for a
//! formal with no default, or too many actuals, stays loud. A default that
//! references another FORMAL (e.g. `int b = a + 1`) is loud-rejected — it would
//! otherwise bind to a same-named caller variable (a silent-wrong vs iverilog,
//! which resolves it in the subroutine scope). Pinned to iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_tfda_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let s = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.contains("simulation ended"))
        .collect::<Vec<_>>()
        .join("|");
    (s.trim().to_owned(), out.status.success())
}

#[test]
fn function_single_default() {
    let (o, ok) = run(
        "module top; function int f(int a, int b=10); f=a+b; endfunction\n\
         initial begin $display(\"%0d %0d\", f(5), f(5,20)); #1 $finish; end endmodule",
    );
    assert!(ok && o == "15 25", "got:\n{o}");
}

#[test]
fn function_multiple_defaults_various_omissions() {
    let (o, ok) = run(
        "module top; function int f(int a=1, int b=2, int c=3); f=a*100+b*10+c; endfunction\n\
         initial begin $display(\"%0d %0d %0d\", f(), f(9), f(9,8)); #1 $finish; end endmodule",
    );
    assert!(ok && o == "123 923 983", "got:\n{o}");
}

#[test]
fn function_automatic_default() {
    // The frame-call (automatic) path fills defaults too.
    let (o, ok) = run(
        "module top; function automatic int f(int a, int b=10); f=a+b; endfunction\n\
         initial begin $display(\"%0d\", f(5)); #1 $finish; end endmodule",
    );
    assert!(ok && o == "15", "got:\n{o}");
}

#[test]
fn task_default() {
    let (o, ok) = run(
        "module top; task t(int a, int b=5); $display(\"%0d\", a+b); endtask\n\
         initial begin t(3); t(3,7); #1 $finish; end endmodule",
    );
    assert!(ok && o == "8|10", "got:\n{o}");
}

#[test]
fn module_param_default() {
    // A default may be a module-scope parameter / constant expression.
    let (o, ok) = run(
        "module top; parameter P=100; function int f(int a, int b=P); f=a+b; endfunction\n\
         initial begin $display(\"%0d\", f(5)); #1 $finish; end endmodule",
    );
    assert!(ok && o == "105", "got:\n{o}");
}

#[test]
fn no_default_functions_unchanged() {
    // Byte-identity: functions/tasks with no default args are unaffected.
    let (of, okf) = run(
        "module top; function int f(int a, int b); f=a+b; endfunction\n\
         initial begin $display(\"%0d\", f(3,4)); #1 $finish; end endmodule",
    );
    assert!(okf && of == "7", "func got:\n{of}");
    let (ot, okt) = run(
        "module top; task t(int a, int b); $display(\"%0d\", a+b); endtask\n\
         initial begin t(3,4); #1 $finish; end endmodule",
    );
    assert!(okt && ot == "7", "task got:\n{ot}");
}

#[test]
fn arity_and_formal_ref_default_are_loud() {
    // Too few actuals for a formal with no default → loud.
    let (_a, oka) = run(
        "module top; function int f(int a, int b); f=a+b; endfunction\n\
         initial begin $display(\"%0d\", f(5)); #1 $finish; end endmodule",
    );
    assert!(!oka, "too few args must be loud");
    // Too many actuals → loud.
    let (_b, okb) = run(
        "module top; function int f(int a, int b=1); f=a+b; endfunction\n\
         initial begin $display(\"%0d\", f(1,2,3)); #1 $finish; end endmodule",
    );
    assert!(!okb, "too many args must be loud");
    // A default that references another formal → loud (would bind to a same-named
    // caller variable otherwise) — both the bare form and a cast-wrapped form.
    let (_c, okc) = run(
        "module top; function int f(int a, int b=a+1); f=a+b; endfunction int a=100;\n\
         initial begin $display(\"%0d\", f(5)); #1 $finish; end endmodule",
    );
    assert!(!okc, "a default referencing a formal must be loud");
    let (_d, okd) = run(
        "module top; int a=99; function int f(int a, int b=int'(a)); f=b; endfunction\n\
         initial begin $display(\"%0d\", f(5)); #1 $finish; end endmodule",
    );
    assert!(
        !okd,
        "a default casting a formal must be loud (the converged review finding)"
    );
}
